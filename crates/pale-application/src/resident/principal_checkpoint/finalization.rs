//! Construct the principal checkpoint for an admitted deployment finalization.
//! Logical encoding belongs to the existing principal profile. Native preparation
//! computes the tree and releases its snapshot before returning. The retained
//! result admits only a checkpoint over those exact changes; commitment follows.

use super::*;
use crate::resident::AdmittedDeploymentFinalization;

mod predecessor;
use pale_ports::{
    ControlRecordCoordinate, ControlRecordDigest, OperationAccumulatorCheckpoint,
    PrincipalCheckpointPreparationPort,
};

#[derive(Serialize)]
struct FinalizationSelection<'a> {
    binding_ref: &'a ResidentRecordRefV1,
    binding_sha256: &'a DeploymentBindingDigest,
    checkpoint_signer_ref: &'a ResidentRecordRefV1,
    checkpoint_signer_sha256: &'a CheckpointSigningAuthorityDigest,
    runtime_authority_ref: &'a ResidentRecordRefV1,
    runtime_authority_sha256: &'a DeploymentRuntimeAuthorityDigest,
    trust_checkpoint_ref: &'a ResidentRecordRefV1,
    trust_checkpoint_sha256: &'a PrincipalTrustCheckpointDigest,
}

/// Finalization records and native tree changes awaiting the signed checkpoint.
/// Dropping this value writes nothing and leaves no database snapshot open.
#[must_use = "prepared roots still require checkpoint admission and atomic custody"]
pub struct PreparedDeploymentFinalizationCheckpoint<'a> {
    finalization: AdmittedDeploymentFinalization<'a>,
    predecessor_ref: &'a ResidentRecordRefV1,
    predecessor: &'a ControlStateCheckpointV1,
    changed_keys: Vec<ChangedControlLeafKeyV1>,
    leaf_count: ControlLeafCountV1,
    accumulator: PreparedOperationAccumulator,
}

/// Admitted checkpoint retaining the finalization and its physical tree batch.
///
/// This is construction evidence, not a database receipt. It cannot finalize a
/// deployment, complete genesis or establish an external anti-rollback anchor.
#[must_use = "constructed finalization still requires authenticated atomic custody"]
pub struct ConstructedDeploymentFinalization<'a> {
    finalization: AdmittedDeploymentFinalization<'a>,
    checkpoint_ref: ResidentRecordRefV1,
    checkpoint: ControlStateCheckpointV1,
    accumulator: OperationAccumulatorBatch,
}

impl SupportedPrincipalTrustCheckpointProfile {
    /// Prepare the trust replacement, two signer insertions and finalization leaf.
    ///
    /// The caller supplies an admitted current control checkpoint. Its signed
    /// identity is the logical principal control store, while preparation reads
    /// the separately bound physical trust store. The native port must compare
    /// the current checkpoint and transaction, prove old leaf values and enforce
    /// the supplied work limits. Exact trust-head and independent-anchor reads
    /// bracket preparation. Two trust reads each obey `maximum_trust_checkpoint_bytes`;
    /// four anchor reads carry only fixed coordinates. These separate reads do not
    /// form an atomic snapshot or prove that the heads remain current after return.
    /// Commitment must still compare the retained predecessor and transaction.
    /// This call does not sign, advance an anchor, or commit anything.
    ///
    /// # Errors
    /// Rejects unsupported scope/profile, invalid chronology, a nonempty
    /// unresolved tree, exceeded limits, stale native state or a substituted
    /// preparation result. The finalization phase is consumed on failure.
    pub fn prepare_finalization<'a, P>(
        &self,
        finalization: AdmittedDeploymentFinalization<'a>,
        (predecessor_ref, predecessor): (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
        maximum_trust_checkpoint_bytes: usize,
        limits: AccumulatorPreparationLimits,
        custody: &mut P,
        anchors: &mut dyn pale_ports::AntiRollbackAnchorPort,
    ) -> Result<PreparedDeploymentFinalizationCheckpoint<'a>, PrincipalCheckpointError>
    where
        P: PrincipalCheckpointPreparationPort + pale_ports::PrincipalTrustReadPort + ?Sized,
    {
        predecessor.verify()?;
        let records = finalization.records();
        let trust = records.trust.predecessor;
        if predecessor.checkpoint_profile_ref() != &self.reference
            || predecessor.checkpoint_profile_sha256() != &self.digest
            || predecessor.checkpoint_domain_id() != &self.domains[0]
            || predecessor.domain_epoch().get() != INITIAL_DOMAIN_EPOCH
            || *predecessor.control_schema_version() != CONTROL_SCHEMA_VERSION
            || predecessor.control_store_id() != trust.principal_control_store_id()
            || predecessor.control_incarnation_id() != trust.principal_control_incarnation_id()
            || predecessor.issuer_principal_id() != trust.principal_id()
            || predecessor.issued_at() < trust.issued_at()
            || records.trust.successor.issued_at() < predecessor.issued_at()
            || predecessor.leaf_count().get() < 2
            || !predecessor
                .unresolved_custody_root_sha256()
                .as_str()
                .bytes()
                .all(|byte| byte == b'0')
        {
            return Err(PrincipalCheckpointError::Association);
        }
        let expected = predecessor::Expected::new(
            records.trust.predecessor_ref,
            trust,
            predecessor_ref,
            predecessor,
            maximum_trust_checkpoint_bytes,
        )?;
        let leaf_count = ControlLeafCountV1::new(
            predecessor
                .leaf_count()
                .get()
                .checked_add(3)
                .ok_or(PrincipalCheckpointError::Association)?,
        )?;
        let scope = canonical_hash(&PrincipalScope {
            domain: SCOPE_DOMAIN,
            control_store_id: predecessor.control_store_id(),
            control_incarnation_id: predecessor.control_incarnation_id(),
            checkpoint_domain_id: predecessor.checkpoint_domain_id(),
            domain_epoch: *predecessor.domain_epoch(),
        })?;
        let trust_payload = |reference, checkpoint: &'a PrincipalTrustCheckpointV1| TrustHead {
            principal_id: checkpoint.principal_id(),
            checkpoint_epoch: checkpoint.checkpoint_epoch(),
            trust_checkpoint_ref: reference,
            trust_checkpoint_sha256: checkpoint.canonical_trust_checkpoint_sha256(),
        };
        let (_, _, old_trust) = leaf(
            &scope,
            "principal-trust-head",
            trust.principal_id().as_str(),
            &trust_payload(records.trust.predecessor_ref, trust),
        )?;
        let (trust_key, trust_hash, new_trust) = leaf(
            &scope,
            "principal-trust-head",
            trust.principal_id().as_str(),
            &trust_payload(records.trust.successor_ref, records.trust.successor),
        )?;
        let mut changes = vec![(
            trust_key,
            AccumulatorLeafChange::new(trust_hash, Some(old_trust), Some(new_trust))?,
        )];
        let signer = finalization.checkpoint_signer_head();
        let runtime = finalization.runtime_authority_head();
        let selection = FinalizationSelection {
            binding_ref: records.binding.0,
            binding_sha256: records.binding.1.canonical_binding_sha256(),
            checkpoint_signer_ref: records.checkpoint_signer.0,
            checkpoint_signer_sha256: records
                .checkpoint_signer
                .1
                .canonical_checkpoint_signing_authority_sha256(),
            runtime_authority_ref: records.runtime_authority.0,
            runtime_authority_sha256: records
                .runtime_authority
                .1
                .canonical_deployment_runtime_authority_sha256(),
            trust_checkpoint_ref: records.trust.successor_ref,
            trust_checkpoint_sha256: records.trust.successor.canonical_trust_checkpoint_sha256(),
        };
        for (key, hashed_key, value) in [
            leaf(
                &scope,
                "checkpoint-signing-authority-head",
                signer.checkpoint_signing_authority_id.as_str(),
                signer,
            )?,
            leaf(
                &scope,
                "deployment-runtime-authority-head",
                runtime.runtime_authority_id.as_str(),
                runtime,
            )?,
            leaf(
                &scope,
                "deployment-finalization",
                records.binding.1.deployment_binding_id().as_str(),
                &selection,
            )?,
        ] {
            changes.push((
                key,
                AccumulatorLeafChange::new(hashed_key, None, Some(value))?,
            ));
        }
        let mut changed_keys: Vec<_> = changes.iter().map(|(key, _)| key.clone()).collect();
        changed_keys.sort();
        changes.sort_by(|left, right| left.1.key().cmp(right.1.key()));
        let state = StateLeafChanges::new(
            changes.into_iter().map(|(_, change)| change).collect(),
            limits.maximum_leaf_changes(),
        )?;
        let store = AuthorityStoreCoordinate::new(
            trust.trust_store_id().clone(),
            trust.trust_store_incarnation_id().clone(),
        );
        expected.verify(custody, anchors)?;
        let transaction = custody
            .prepare_transaction(&store)
            .map_err(OperationAccumulatorPreparationError::from)?;
        let request = OperationAccumulatorPreparation::new(
            AccumulatorPreparationBasis::Successor(OperationAccumulatorCheckpoint::new(
                store,
                self.reference.clone(),
                self.digest.clone(),
                physical_head(predecessor_ref, predecessor)?,
                predecessor.state_root_sha256().clone(),
                predecessor.unresolved_custody_root_sha256().clone(),
            )),
            state,
            UnresolvedCustodyLeafChanges::new(Vec::new(), limits.maximum_leaf_changes())?,
            limits,
            IMPLEMENTATION,
            transaction,
        )?;
        let accumulator = custody.prepare_accumulator(&request)?;
        if accumulator.request() != &request {
            return Err(PrincipalCheckpointError::Association);
        }
        expected.verify(custody, anchors)?;
        Ok(PreparedDeploymentFinalizationCheckpoint {
            finalization,
            predecessor_ref,
            predecessor,
            changed_keys,
            leaf_count,
            accumulator,
        })
    }
}

impl<'a> PreparedDeploymentFinalizationCheckpoint<'a> {
    /// Proposed physical transaction sequence; it is neither reserved nor committed.
    #[must_use]
    pub fn transaction_sequence(&self) -> ControlTransactionSequenceV1 {
        self.accumulator.request().transaction().successor()
    }
    /// Full state root computed by native preparation.
    #[must_use]
    pub fn state_root(&self) -> &ArtifactDigest {
        self.accumulator.state_root()
    }
    /// Unchanged empty unresolved-custody root.
    #[must_use]
    pub fn unresolved_root(&self) -> &ArtifactDigest {
        self.accumulator.unresolved_root()
    }
    /// Complete logical changes in canonical tuple order.
    #[must_use]
    pub fn changed_keys(&self) -> &[ChangedControlLeafKeyV1] {
        &self.changed_keys
    }
    /// Retained state leaf count after three insertions and one replacement.
    #[must_use]
    pub const fn leaf_count(&self) -> ControlLeafCountV1 {
        self.leaf_count
    }

    /// Admit the exact signed successor while retaining its construction.
    ///
    /// Construction is checked before signature verification. The caller resolves
    /// the current principal checkpoint signer, domain permission and validity;
    /// neither a matching signature nor this result proves native commitment.
    ///
    /// # Errors
    /// Rejects a reused reference, changed construction, wrong signing scope,
    /// invalid successor or signature. No write or external anchor is attempted.
    pub fn admit_checkpoint<F>(
        self,
        reference: ResidentRecordRefV1,
        wire: ControlStateCheckpointWireV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<ConstructedDeploymentFinalization<'a>, PrincipalCheckpointError>
    where
        F: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let records = self.finalization.records();
        if [
            &self.predecessor_ref,
            &records.binding.0,
            &records.checkpoint_signer.0,
            &records.runtime_authority.0,
            &records.trust.predecessor_ref,
            &records.trust.successor_ref,
        ]
        .iter()
        .any(|prior| **prior == &reference)
            || authority.principal_id != records.trust.predecessor.principal_id()
            || authority.control_store_id != self.predecessor.control_store_id()
            || authority.control_incarnation_id != self.predecessor.control_incarnation_id()
        {
            return Err(PrincipalCheckpointError::Association);
        }
        let checkpoint = ControlStateCheckpointV1::admit_successor_wire(
            wire,
            self.predecessor_ref,
            self.predecessor,
            schema,
            authority,
            |domain, preimage, digest, signature| {
                if preimage.state_root_sha256 != *self.state_root()
                    || preimage.unresolved_custody_root_sha256 != *self.unresolved_root()
                    || preimage.changed_leaf_keys != self.changed_keys
                    || preimage.leaf_count != self.leaf_count
                    || preimage.committed_transaction_sequence != self.transaction_sequence()
                    || preimage.issued_at < *records.trust.successor.issued_at()
                {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "deployment finalization checkpoint",
                        invariant: "exact prepared roots, keys, count, transaction and chronology",
                    });
                }
                verify_signature(domain, preimage, digest, signature)
            },
        )?;
        let accumulator = self
            .accumulator
            .bind_successor(physical_head(&reference, &checkpoint)?)?;
        Ok(ConstructedDeploymentFinalization {
            finalization: self.finalization,
            checkpoint_ref: reference,
            checkpoint,
            accumulator,
        })
    }
}

impl ConstructedDeploymentFinalization<'_> {
    /// Borrow the admitted finalization retained by this checkpoint.
    pub fn finalization(&self) -> &AdmittedDeploymentFinalization<'_> {
        &self.finalization
    }
    /// Borrow the admitted signed checkpoint and its allocated reference.
    #[must_use]
    pub fn checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint_ref, &self.checkpoint)
    }
    /// Borrow the physical tree batch that must accompany the same atomic commit.
    #[must_use]
    pub fn accumulator(&self) -> &OperationAccumulatorBatch {
        &self.accumulator
    }
}

fn physical_head(
    reference: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<ControlCheckpointHead, PrincipalCheckpointError> {
    Ok(ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        *checkpoint.domain_epoch(),
        *checkpoint.domain_sequence(),
        *checkpoint.committed_transaction_sequence(),
        ControlRecordCoordinate::new(
            reference.clone(),
            ControlRecordDigest::parse(checkpoint.canonical_control_checkpoint_sha256().as_str())?,
        ),
    ))
}
