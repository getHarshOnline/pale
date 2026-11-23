//! Prepare observation roots before signing and retain their exact input association.
//!
//! Profile selection, checkpoint admission, and signer authority remain separate
//! prerequisites. This phase uses the selected operation-leaf encoding; it does
//! not infer profile support from an arbitrary reference or digest.

use pale_core::records::{
    CanonicalSignatureV1, ChangedControlLeafKeyV1,
    ControlCheckpointSigningAuthorityAdmissionContextV1, ControlLeafCountV1,
    ControlStateCheckpointDigest, ControlStateCheckpointPreimageV1, ControlStateCheckpointV1,
    InvalidResidentRecordV1, ResidentRecordRefV1, SchemaDigest,
};
use pale_ports::{
    AccumulatorLeafChange, AccumulatorPreparationBasis, AccumulatorPreparationLimits,
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecordCoordinate,
    ControlTransactionPreparation, OperationAccumulatorCheckpoint, OperationAccumulatorPreparation,
    OperationCustodyPort, PreparedOperationAccumulator, StateLeafChanges,
    UnresolvedCustodyLeafChanges,
};

use super::{
    checkpoint_profile::{
        operation_observation_change, OperationCheckpointSelection, OperationLeafProjectionError,
    },
    operation_observation_custody::{
        OperationObservationCheckpointConstruction,
        OperationObservationCheckpointConstructionRejection,
        OperationObservationCheckpointConstructionVerifier,
        OperationObservationCheckpointTransition, OperationObservationCustodyProjectionError,
        PreparedOperationObservationAppend,
    },
    record_projection, PreparedOperationObservationCustody,
};

mod recovery;
pub use recovery::CommittedObservationCheckpointRecovery;

/// Observation changes and verified roots awaiting their exact signed checkpoint.
/// No database snapshot, lock, signature authority, or usable operation escapes
/// preparation. Dropping this value has no storage effect.
#[must_use = "prepared roots require an admitted signed checkpoint and durable custody"]
#[derive(Debug)]
pub struct PreparedOperationObservationCheckpoint<'a> {
    selection: OperationCheckpointSelection,
    append: PreparedOperationObservationAppend<'a>,
    predecessor_ref: &'a ResidentRecordRefV1,
    predecessor: &'a ControlStateCheckpointV1,
    observation_key: ChangedControlLeafKeyV1,
    changed_keys: Vec<ChangedControlLeafKeyV1>,
    leaf_count: ControlLeafCountV1,
    accumulator: PreparedOperationAccumulator,
}

impl<'a> PreparedOperationObservationAppend<'a> {
    /// Compute this append's roots using one read-only native preparation.
    ///
    /// The caller must independently admit `predecessor`. The retained selection
    /// binds supported immutable profile content and the original request before
    /// any native read. The native port verifies the physical snapshot against
    /// those roots. This method cannot establish current signer authority or
    /// replace an anchor check.
    /// An observation-only append preserves unresolved custody unchanged.
    ///
    /// # Errors
    /// Rejects store mismatch, invalid input, unavailable/corrupt state, exceeded
    /// work limits, or a response for another predecessor. No write occurs here.
    pub fn prepare_checkpoint(
        self,
        selection: OperationCheckpointSelection,
        predecessor_ref: &'a ResidentRecordRefV1,
        predecessor: &'a ControlStateCheckpointV1,
        limits: AccumulatorPreparationLimits,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        let transaction = custody
            .prepare_transaction(self.store())
            .map_err(pale_ports::OperationAccumulatorPreparationError::from)?;
        self.prepare_checkpoint_with_transaction(
            selection,
            predecessor_ref,
            predecessor,
            limits,
            transaction,
            custody,
        )
    }

    /// Compute roots with one exact transaction selected by a compound owner.
    ///
    /// Consultation uses this seam after deriving its private journal/frontier
    /// pair from the same transaction. The transaction is still only a fence;
    /// native custody must compare it atomically before anything is reported.
    pub(in crate::resident) fn prepare_checkpoint_with_transaction(
        self,
        selection: OperationCheckpointSelection,
        predecessor_ref: &'a ResidentRecordRefV1,
        predecessor: &'a ControlStateCheckpointV1,
        limits: AccumulatorPreparationLimits,
        transaction: ControlTransactionPreparation,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        self.prepare_checkpoint_with(
            selection,
            predecessor_ref,
            predecessor,
            limits,
            transaction,
            |request| custody.prepare_accumulator(request),
        )
    }

    fn prepare_checkpoint_with(
        self,
        selection: OperationCheckpointSelection,
        predecessor_ref: &'a ResidentRecordRefV1,
        predecessor: &'a ControlStateCheckpointV1,
        limits: AccumulatorPreparationLimits,
        transaction: ControlTransactionPreparation,
        prepare: impl FnOnce(
            &OperationAccumulatorPreparation,
        ) -> Result<
            PreparedOperationAccumulator,
            pale_ports::OperationAccumulatorPreparationError,
        >,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        selection.verify(self.store(), self.handle().1, predecessor)?;
        let leaf =
            operation_observation_change(&self, predecessor).map_err(|error| match error {
                OperationLeafProjectionError::Store => {
                    OperationObservationCustodyProjectionError::StoreScope
                }
                OperationLeafProjectionError::Identity(error) => error.into(),
                OperationLeafProjectionError::Canonicalize(error) => error.into(),
            })?;
        let prior = OperationAccumulatorCheckpoint::new(
            self.store().clone(),
            predecessor.checkpoint_profile_ref().clone(),
            predecessor.checkpoint_profile_sha256().clone(),
            checkpoint_head(predecessor_ref, predecessor)?,
            predecessor.state_root_sha256().clone(),
            predecessor.unresolved_custody_root_sha256().clone(),
        );
        let observation_key = leaf.key.clone();
        let mut changes = vec![AccumulatorLeafChange::new(
            leaf.key_sha256,
            Some(leaf.predecessor_sha256),
            Some(leaf.successor_sha256),
        )?];
        let mut changed_keys = vec![leaf.key];
        let inserted = self.a2a_evidence().len() as u64;
        for evidence in self.a2a_evidence() {
            let evidence_leaf =
                super::checkpoint_profile::a2a_evidence_leaf(self.store(), predecessor, evidence)
                    .map_err(|error| match error {
                    OperationLeafProjectionError::Store => {
                        OperationObservationCustodyProjectionError::StoreScope
                    }
                    OperationLeafProjectionError::Identity(error) => error.into(),
                    OperationLeafProjectionError::Canonicalize(error) => error.into(),
                })?;
            changes.push(AccumulatorLeafChange::new(
                evidence_leaf.hashed_key,
                None,
                Some(evidence_leaf.value),
            )?);
            changed_keys.push(evidence_leaf.key);
        }
        changes.sort_by(|left, right| left.key().cmp(right.key()));
        changed_keys.sort();
        let changes = StateLeafChanges::new(changes, limits.maximum_leaf_changes())?;
        let leaf_count = ControlLeafCountV1::new(
            predecessor
                .leaf_count()
                .get()
                .checked_add(inserted)
                .ok_or(OperationObservationCheckpointConstructionRejection)?,
        )?;
        if transaction.store() != self.store() {
            return Err(OperationObservationCheckpointConstructionRejection.into());
        }
        let request = OperationAccumulatorPreparation::new(
            AccumulatorPreparationBasis::Successor(prior),
            changes,
            UnresolvedCustodyLeafChanges::new(Vec::new(), limits.maximum_leaf_changes())?,
            limits,
            selection.implementation(),
            transaction,
        )?;
        let accumulator = prepare(&request)?;
        if accumulator.request() != &request {
            return Err(OperationObservationCheckpointConstructionRejection.into());
        }
        Ok(PreparedOperationObservationCheckpoint {
            selection,
            append: self,
            predecessor_ref,
            predecessor,
            observation_key,
            changed_keys,
            leaf_count,
            accumulator,
        })
    }
}

impl PreparedOperationObservationCheckpoint<'_> {
    /// Proposed store mutation to put in the checkpoint preimage.
    /// This is neither committed nor reserved; a later writer causes conflict.
    #[must_use]
    pub fn transaction_sequence(&self) -> pale_core::records::ControlTransactionSequenceV1 {
        self.accumulator.request().transaction().successor()
    }

    /// Borrow the full-state root to put in the successor checkpoint.
    #[must_use]
    pub fn state_root(&self) -> &pale_core::records::ArtifactDigest {
        self.accumulator.state_root()
    }

    /// Borrow the unchanged unresolved-custody root.
    #[must_use]
    pub fn unresolved_root(&self) -> &pale_core::records::ArtifactDigest {
        self.accumulator.unresolved_root()
    }

    /// Borrow the observation key changed by every append.
    ///
    /// Use [`Self::changed_keys`] for the complete checkpoint preimage. An append
    /// can also insert one or two exact A2A evidence leaves.
    #[must_use]
    pub const fn observation_key(&self) -> &ChangedControlLeafKeyV1 {
        &self.observation_key
    }

    /// Borrow every exact logical key changed by this append.
    #[must_use]
    pub fn changed_keys(&self) -> &[ChangedControlLeafKeyV1] {
        &self.changed_keys
    }

    /// Return the retained logical leaf count after this append.
    #[must_use]
    pub const fn leaf_count(&self) -> ControlLeafCountV1 {
        self.leaf_count
    }

    /// Sign the exact prepared successor and assemble its mandatory custody.
    ///
    /// Native roots, changed keys, retained count, selected profile and handle
    /// issuer are checked before the signer is called. The existing core owner
    /// then checks the exact retained predecessor, schema and supplied signing
    /// witness. This method does not resolve authority lifetime or verify the
    /// returned signature. Reference allocation, key custody and physical time
    /// remain caller obligations. The retained native transaction fence is checked
    /// before signing and must still be compared by actual atomic commitment.
    /// Physical commitment and independent anchoring still follow. No custody
    /// or anchor port is called here; the supplied signer owns its own work.
    /// Returns the prepared custody and the exact admitted checkpoint, so a
    /// later phase need not decode or re-admit the checkpoint just issued.
    ///
    /// # Errors
    /// Rejects unlike retained coordinates before signing, returns core or signer
    /// failures through the existing semantic error, and rejects invalid physical
    /// batch assembly. A failure cannot return usable operation custody.
    pub fn issue_checkpoint<Sign>(
        self,
        successor_ref: &ResidentRecordRefV1,
        preimage: ControlStateCheckpointPreimageV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<
        (
            PreparedOperationObservationCustody,
            ControlStateCheckpointV1,
        ),
        OperationObservationCustodyProjectionError,
    >
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let successor = self.issue_checkpoint_value(preimage, schema, authority, sign)?;
        let custody = self.attach_issued_checkpoint(successor_ref, &successor)?;
        Ok((custody, successor))
    }

    /// Sign the exact prepared successor without attaching it to custody.
    ///
    /// The caller must retain the returned checkpoint if later attachment
    /// fails. Retrying attachment must use that value and must not sign again.
    pub(in crate::resident) fn issue_checkpoint_value<Sign>(
        &self,
        preimage: ControlStateCheckpointPreimageV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<ControlStateCheckpointV1, OperationObservationCustodyProjectionError>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let basis = self.accumulator.basis();
        if !self.selection.matches_authority(authority)
            || authority.principal_id != self.append.handle().1.issuer_principal_id()
            || &preimage.control_store_id != basis.store().store_id()
            || &preimage.control_incarnation_id != basis.store().incarnation_id()
            || &preimage.checkpoint_domain_id != basis.domain()
            || preimage.domain_epoch != basis.epoch()
            || preimage.committed_transaction_sequence != self.transaction_sequence()
            || &preimage.checkpoint_profile_ref != basis.profile_ref()
            || &preimage.checkpoint_profile_sha256 != basis.profile_sha256()
            || preimage.control_schema_version != super::checkpoint_profile::CONTROL_SCHEMA_VERSION
            || &preimage.state_root_sha256 != self.accumulator.state_root()
            || &preimage.unresolved_custody_root_sha256 != self.accumulator.unresolved_root()
            || preimage.changed_leaf_keys != self.changed_keys
            || preimage.leaf_count != self.leaf_count
        {
            return Err(OperationObservationCheckpointConstructionRejection.into());
        }
        ControlStateCheckpointV1::issue_successor_contextual(
            preimage,
            self.predecessor_ref,
            self.predecessor,
            schema,
            authority,
            sign,
        )
        .map_err(Into::into)
    }

    /// Attach one already signed checkpoint without calling a signer.
    ///
    /// This consumes the prepared roots. A caller that needs to continue after
    /// failure must retain the issued checkpoint and rebuild only these roots.
    pub(in crate::resident) fn attach_issued_checkpoint(
        self,
        successor_ref: &ResidentRecordRefV1,
        successor: &ControlStateCheckpointV1,
    ) -> Result<PreparedOperationObservationCustody, OperationObservationCustodyProjectionError>
    {
        let successor_head = checkpoint_head(successor_ref, successor)?;
        let proof = PreparedObservationConstruction {
            store: self.append.store().clone(),
            predecessor: self.append.predecessor_record().coordinate().clone(),
            successor: self.append.successor_record().coordinate().clone(),
            associated: self
                .append
                .associated_records()
                .iter()
                .map(|record| (record.kind(), record.record().coordinate().clone()))
                .collect(),
            associated_indexes: self.append.associated_indexes().to_vec(),
            a2a_evidence: self
                .append
                .a2a_evidence()
                .iter()
                .map(|evidence| evidence.coordinate().clone())
                .collect(),
            checkpoint: successor_head.clone(),
        };
        let batch = self.accumulator.bind_successor(successor_head)?;
        self.append.attach_checkpoint(
            OperationObservationCheckpointTransition::new(
                self.predecessor_ref,
                self.predecessor,
                successor_ref,
                successor,
                &proof,
            ),
            batch,
        )
    }
}

// Constructed only by the consuming prepared phase after root/change checks.
// Retains record coordinates, not duplicate canonical record bodies.
struct PreparedObservationConstruction {
    store: AuthorityStoreCoordinate,
    predecessor: ControlRecordCoordinate,
    successor: ControlRecordCoordinate,
    associated: Vec<(
        pale_ports::OperationCustodyRecordKind,
        ControlRecordCoordinate,
    )>,
    associated_indexes: Vec<pale_ports::OperationCustodyIndexMutation>,
    a2a_evidence: Vec<ControlRecordCoordinate>,
    checkpoint: ControlCheckpointHead,
}

impl OperationObservationCheckpointConstructionVerifier for PreparedObservationConstruction {
    fn verify(
        &self,
        construction: &OperationObservationCheckpointConstruction<'_>,
    ) -> Result<(), OperationObservationCheckpointConstructionRejection> {
        (construction.store() == &self.store
            && construction.predecessor_record().coordinate() == &self.predecessor
            && construction.successor_record().coordinate() == &self.successor
            && construction
                .associated_records()
                .iter()
                .map(|record| (record.kind(), record.record().coordinate()))
                .eq(self
                    .associated
                    .iter()
                    .map(|(kind, coordinate)| (*kind, coordinate)))
            && construction.associated_indexes() == self.associated_indexes
            && construction
                .a2a_evidence()
                .iter()
                .map(|evidence| evidence.coordinate())
                .eq(self.a2a_evidence.iter())
            && construction.checkpoint().successor() == &self.checkpoint)
            .then_some(())
            .ok_or(OperationObservationCheckpointConstructionRejection)
    }
}

fn checkpoint_head(
    reference: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<ControlCheckpointHead, OperationObservationCustodyProjectionError> {
    Ok(ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        *checkpoint.domain_epoch(),
        *checkpoint.domain_sequence(),
        *checkpoint.committed_transaction_sequence(),
        record_projection::coordinate(
            reference,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )?,
    ))
}
