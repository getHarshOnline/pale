//! Construct principal checkpoint roots from admitted records before custody.
//!
//! The signed trust head commits its referenced authority records. The initial
//! control tree adds that head and its checkpoint-signer head. Its roots and
//! physical nodes remain attached through control admission, receipt admission
//! and the existing atomic bootstrap commit. No signer or external anchor is
//! selected here.

use pale_core::{canon, records::*};
use pale_ports::{
    AccumulatorBatchError, AccumulatorInitialDomain, AccumulatorLeafChange,
    AccumulatorPreparationBasis, AccumulatorPreparationInputError, AccumulatorPreparationLimits,
    AuthorityStoreCoordinate, ControlCheckpointHead, OperationAccumulatorBatch,
    OperationAccumulatorPreparation, OperationAccumulatorPreparationError,
    PreparedOperationAccumulator, PrincipalBootstrapCustodyPort, StateLeafChanges,
    UnresolvedCustodyLeafChanges,
};
use serde::Serialize;

use super::{
    checkpoint_profile::{canonical_hash, KEY_DOMAIN},
    prepare_principal_bootstrap_custody, record_projection, PreparedPrincipalBootstrapCustody,
    PrincipalBootstrapProjectionError,
};

mod finalization;
mod profile;
pub use finalization::{
    ConstructedDeploymentFinalization, PreparedDeploymentFinalizationCheckpoint,
};
pub use profile::SupportedPrincipalTrustCheckpointProfile;
use profile::{
    CONTROL_SCHEMA_VERSION, IMPLEMENTATION, INITIAL_DOMAIN_EPOCH, SCOPE_DOMAIN, VALUE_DOMAIN,
};

/// Why a principal checkpoint could not be constructed or admitted.
#[derive(Debug, thiserror::Error)]
pub enum PrincipalCheckpointError {
    /// The selected bytes do not describe source-supported principal content.
    #[error("principal checkpoint profile is unsupported or exceeds its byte limit")]
    UnsupportedProfile,
    /// The descriptor does not have the selected digest.
    #[error("principal checkpoint profile content does not match its digest")]
    ProfileDigest,
    /// The retained profile, stores, roots or checkpoint association differ.
    #[error("principal checkpoint does not match its retained construction")]
    Association,
    /// The selected trust head or exact stored body is missing or has changed.
    #[error("principal finalization trust predecessor is not current")]
    CurrentTrust,
    /// The independently retained principal minimum is not the selected predecessor.
    #[error("principal finalization trust minimum is missing or differs")]
    TrustMinimum,
    /// The independently retained hard-domain anchor is not the selected predecessor.
    #[error("principal finalization control anchor is missing or differs")]
    ControlAnchor,
    /// A trust-checkpoint body exceeds the explicit read ceiling, or the ceiling is zero.
    #[error("principal finalization trust checkpoint exceeds its read bound")]
    TrustReadBound,
    /// Native trust-head discovery failed without changing the store.
    #[error(transparent)]
    TrustRead(#[from] pale_ports::ControlStoreError),
    /// The selected independent anchor could not be read reliably.
    #[error(transparent)]
    AnchorRead(#[from] pale_ports::AntiRollbackAnchorError),
    /// Canonical encoding or strict content decoding failed.
    #[error(transparent)]
    Canonicalize(#[from] canon::CanonError),
    /// Existing record admission rejected the candidate.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
    /// Logical changes or preparation bounds were invalid.
    #[error(transparent)]
    Preparation(#[from] AccumulatorPreparationInputError),
    /// Native snapshot preparation failed or exceeded its work budget.
    #[error(transparent)]
    Native(#[from] OperationAccumulatorPreparationError),
    /// The physical tree batch does not close over its checkpoint.
    #[error(transparent)]
    Batch(#[from] AccumulatorBatchError),
    /// Existing bootstrap custody projection rejected the admitted atom.
    #[error(transparent)]
    Custody(#[from] PrincipalBootstrapProjectionError),
    /// A physical record coordinate could not be represented.
    #[error(transparent)]
    Record(#[from] pale_ports::ControlRecordError),
}

#[derive(Serialize)]
struct PrincipalScope<'a> {
    domain: &'static str,
    control_store_id: &'a ControlStoreIdV1,
    control_incarnation_id: &'a ControlIncarnationIdV1,
    checkpoint_domain_id: &'a ControlCheckpointDomainIdV1,
    domain_epoch: ControlDomainEpochV1,
}

#[derive(Serialize)]
struct TrustHead<'a> {
    principal_id: &'a ResidentPrincipalIdV1,
    checkpoint_epoch: &'a PrincipalTrustCheckpointEpochV1,
    trust_checkpoint_ref: &'a ResidentRecordRefV1,
    trust_checkpoint_sha256: &'a PrincipalTrustCheckpointDigest,
}

fn leaf(
    scope: &ArtifactDigest,
    class: &str,
    logical_key: &str,
    payload: &impl Serialize,
) -> Result<(ChangedControlLeafKeyV1, ArtifactDigest, ArtifactDigest), PrincipalCheckpointError> {
    let key = ChangedControlLeafKeyV1 {
        record_class: ControlRecordClassV1::parse(class)?,
        scope_key_sha256: scope.clone(),
        logical_key: ControlLogicalKeyV1::parse(logical_key)?,
    };
    let hashed_key = canonical_hash(&(KEY_DOMAIN, &key))?;
    let value = canonical_hash(&(VALUE_DOMAIN, &key, payload))?;
    Ok((key, hashed_key, value))
}

/// Native-prepared initial roots awaiting the exact signed control checkpoint.
/// Dropping this value has no store effect and grants no bootstrap permission.
#[must_use = "prepared principal roots require control admission and atomic custody"]
pub struct PreparedPrincipalBootstrapCheckpoint<'profile> {
    trust: AdmittedInitialPrincipalTrustCheckpointV1<'profile>,
    changed_keys: Vec<ChangedControlLeafKeyV1>,
    accumulator: PreparedOperationAccumulator,
}

impl SupportedPrincipalTrustCheckpointProfile {
    /// Construct initial roots from the admitted trust head and signer authority.
    ///
    /// The native owner must prove absence of this domain and return preparation
    /// for the exact proposed physical transaction. The signed checkpoint uses
    /// the separately retained principal-control identity, not the database ID.
    /// No write, checkpoint signature or independent anchor is produced here.
    ///
    /// # Errors
    /// Rejects unlike profile content, domains or stores; invalid limits; native
    /// state contradictions; or a result for another preparation request.
    pub fn prepare_bootstrap<'profile>(
        &self,
        trust: AdmittedInitialPrincipalTrustCheckpointV1<'profile>,
        limits: AccumulatorPreparationLimits,
        custody: &mut dyn PrincipalBootstrapCustodyPort,
    ) -> Result<PreparedPrincipalBootstrapCheckpoint<'profile>, PrincipalCheckpointError> {
        let profile = trust.profile();
        if profile.checkpoint_profile_ref != &self.reference
            || profile.checkpoint_profile_sha256 != &self.digest
            || profile.required_checkpoint_domains != self.domains
            || profile.control_schema_version != CONTROL_SCHEMA_VERSION
        {
            return Err(PrincipalCheckpointError::Association);
        }
        let store = AuthorityStoreCoordinate::new(
            profile.trust_store_id.clone(),
            profile.trust_store_incarnation_id.clone(),
        );
        let epoch = ControlDomainEpochV1::new(INITIAL_DOMAIN_EPOCH)?;
        let scope = canonical_hash(&PrincipalScope {
            domain: SCOPE_DOMAIN,
            control_store_id: profile.principal_control_store_id,
            control_incarnation_id: profile.principal_control_incarnation_id,
            checkpoint_domain_id: &self.domains[0],
            domain_epoch: epoch,
        })?;
        let (reference, checkpoint) = trust.trust_checkpoint_entry();
        let signer = trust
            .authorities()
            .checkpoint_authority()
            .active_head(trust.authorities().checkpoint_authority_ref().clone())?;
        let mut changes = vec![
            leaf(
                &scope,
                "principal-trust-head",
                checkpoint.principal_id().as_str(),
                &TrustHead {
                    principal_id: checkpoint.principal_id(),
                    checkpoint_epoch: checkpoint.checkpoint_epoch(),
                    trust_checkpoint_ref: reference,
                    trust_checkpoint_sha256: checkpoint.canonical_trust_checkpoint_sha256(),
                },
            )?,
            leaf(
                &scope,
                "checkpoint-signing-authority-head",
                signer.checkpoint_signing_authority_id.as_str(),
                &signer,
            )?,
        ];
        let mut changed_keys: Vec<_> = changes.iter().map(|(key, _, _)| key.clone()).collect();
        changed_keys.sort();
        changes.sort_by(|left, right| left.1.cmp(&right.1));
        let changes = changes
            .into_iter()
            .map(|(_, key, value)| AccumulatorLeafChange::new(key, None, Some(value)))
            .collect::<Result<Vec<_>, _>>()?;
        let transaction = custody
            .prepare_transaction(&store)
            .map_err(OperationAccumulatorPreparationError::from)?;
        let request = OperationAccumulatorPreparation::new(
            AccumulatorPreparationBasis::Initial(AccumulatorInitialDomain::new(
                store,
                self.reference.clone(),
                self.digest.clone(),
                self.domains[0].clone(),
                epoch,
            )),
            StateLeafChanges::new(changes, limits.maximum_leaf_changes())?,
            UnresolvedCustodyLeafChanges::new(Vec::new(), limits.maximum_leaf_changes())?,
            limits,
            IMPLEMENTATION,
            transaction,
        )?;
        let accumulator = custody.prepare_accumulator(&request)?;
        if accumulator.request() != &request {
            return Err(PrincipalCheckpointError::Association);
        }
        Ok(PreparedPrincipalBootstrapCheckpoint {
            trust,
            changed_keys,
            accumulator,
        })
    }
}

impl PreparedPrincipalBootstrapCheckpoint<'_> {
    /// Proposed physical transaction sequence for the control checkpoint.
    #[must_use]
    pub fn transaction_sequence(&self) -> ControlTransactionSequenceV1 {
        self.accumulator.request().transaction().successor()
    }

    /// Borrow the native-computed state root.
    #[must_use]
    pub fn state_root(&self) -> &ArtifactDigest {
        self.accumulator.state_root()
    }

    /// Borrow the native-computed empty unresolved-custody root.
    #[must_use]
    pub fn unresolved_root(&self) -> &ArtifactDigest {
        self.accumulator.unresolved_root()
    }

    /// Borrow the two initial logical keys in canonical tuple order.
    #[must_use]
    pub fn changed_keys(&self) -> &[ChangedControlLeafKeyV1] {
        &self.changed_keys
    }

    /// Admit the exact signed control checkpoint and retain its physical nodes.
    ///
    /// Construction checks precede the supplied signature verifier. Existing
    /// core admission owns authority, schema, validity and sequence checks.
    /// This cannot accept a different root merely because it was signed.
    ///
    /// # Errors
    /// Rejects changed roots, keys, count or transaction; invalid core records;
    /// or a rejected signature. No receipt or database commitment follows.
    pub fn admit_control_checkpoint<CS, DC>(
        self,
        reference: ResidentRecordRefV1,
        wire: ControlStateCheckpointWireV1,
        mut verify_signature: CS,
        authority_covers_domain: DC,
    ) -> Result<PreparedPrincipalBootstrapFrontier, PrincipalCheckpointError>
    where
        CS: FnMut(
            &[u8],
            &Ed25519PublicKeyV1,
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        DC: FnMut(&[CheckpointDomainPrefixV1], &ControlCheckpointDomainIdV1) -> bool,
    {
        let Self {
            trust,
            changed_keys,
            accumulator,
        } = self;
        let frontier = trust.admit_control_checkpoints(
            vec![(reference, wire)],
            |domain, key, preimage, digest, signature| {
                if &preimage.state_root_sha256 != accumulator.state_root()
                    || &preimage.unresolved_custody_root_sha256 != accumulator.unresolved_root()
                    || preimage.changed_leaf_keys != changed_keys
                    || preimage.leaf_count.get() != 2
                    || preimage.committed_transaction_sequence
                        != accumulator.request().transaction().successor()
                {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "principal bootstrap control checkpoint",
                        invariant: "exact prepared roots, leaf set and transaction",
                    });
                }
                verify_signature(domain, key, preimage, digest, signature)
            },
            authority_covers_domain,
        )?;
        let (reference, checkpoint) = frontier
            .initial_control_checkpoints()
            .next()
            .ok_or(PrincipalCheckpointError::Association)?;
        let coordinate = record_projection::coordinate(
            reference,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )
        .map_err(PrincipalBootstrapProjectionError::from)?;
        let batch = accumulator.bind_successor(ControlCheckpointHead::new(
            checkpoint.checkpoint_domain_id().clone(),
            *checkpoint.domain_epoch(),
            *checkpoint.domain_sequence(),
            *checkpoint.committed_transaction_sequence(),
            coordinate,
        ))?;
        Ok(PreparedPrincipalBootstrapFrontier {
            frontier,
            batches: vec![batch],
        })
    }
}

/// Admitted bootstrap frontier retaining its exact initial physical tree.
#[must_use = "the frontier still requires its root-signed receipt and atomic custody"]
pub struct PreparedPrincipalBootstrapFrontier {
    frontier: AdmittedPrincipalTrustFrontierV1,
    batches: Vec<OperationAccumulatorBatch>,
}

impl PreparedPrincipalBootstrapFrontier {
    /// Admit the existing bootstrap receipt and assemble its complete custody atom.
    ///
    /// No database write occurs. The returned existing custody type commits
    /// signed records and attached nodes together; external anchors remain due.
    ///
    /// # Errors
    /// Returns existing receipt-admission or physical closure errors.
    pub fn admit_receipt<RS>(
        self,
        reference: ResidentRecordRefV1,
        wire: PrincipalBootstrapReceiptWireV1,
        schema: &SchemaDigest,
        verify_signature: RS,
    ) -> Result<PreparedPrincipalBootstrapCustody, PrincipalCheckpointError>
    where
        RS: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalBootstrapReceiptPreimageV1,
            &PrincipalBootstrapReceiptDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let admitted = self
            .frontier
            .admit_receipt(reference, wire, schema, verify_signature)?;
        prepare_principal_bootstrap_custody(&admitted, self.batches).map_err(Into::into)
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/principal_checkpoint.rs"]
pub(crate) mod tests;
