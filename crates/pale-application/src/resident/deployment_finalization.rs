//! Join one deployment's admitted binding and initial signers to a trust successor.
//!
//! The complete old head sets must reproduce the predecessor's signed roots.
//! Finalization inserts one checkpoint signer and one runtime signer, preserves
//! every other head, and checks the new roots against the signed successor.
//! These are cold administration checks, not work performed on the warm hook.
//!
//! After checkpoint construction, [`DeploymentFinalizationSubmission`] checks
//! that an authenticated command names these exact records and affected heads.
//! Neither join resolves current anchored trust, signs records, commits either
//! store, or completes an anti-rollback anchor.

use pale_core::records::*;

use super::trust_transition::{AdmittedPrincipalTrustCheckpointTransition, TrustHeadChange};

pub(super) mod projection;
#[allow(
    dead_code,
    reason = "historical finalization recovery is consumed by private cold-start composition"
)]
pub(in crate::resident) mod recovery;
mod submission;
pub use submission::DeploymentFinalizationSubmission;

/// Already admitted records and complete old head sets for one finalization.
///
/// The caller resolves references, signatures and current trust before joining
/// these records. The head arrays must be in their canonical lineage order.
/// Supplying this value does not establish that the deployment is finalized.
pub struct DeploymentFinalizationCandidate<'a> {
    /// Immutable binding reference and its admitted proposal/acceptance closure.
    pub binding: (&'a ResidentRecordRefV1, &'a DeploymentBindingV1),
    /// New deployment-scoped checkpoint delegation and its immutable reference.
    pub checkpoint_signer: (&'a ResidentRecordRefV1, &'a CheckpointSigningAuthorityV1),
    /// New deployment-scoped runtime delegation and its immutable reference.
    pub runtime_authority: (&'a ResidentRecordRefV1, &'a DeploymentRuntimeAuthorityV1),
    /// Admitted predecessor and successor with the generic succession law proved.
    pub trust: AdmittedPrincipalTrustCheckpointTransition<'a>,
    /// Complete active checkpoint-signer heads selected by the predecessor.
    pub checkpoint_heads: &'a [CheckpointSigningAuthorityHeadV1],
    /// Complete active runtime-authority heads selected by the predecessor.
    pub runtime_heads: &'a [DeploymentRuntimeAuthorityHeadV1],
}

/// The binding, initial signers and trust successor agree as one semantic change.
///
/// This retained join is input to checkpoint construction, not a store receipt.
/// It cannot establish current authority, administrator permission, completed
/// deployment genesis or runtime readiness.
#[must_use = "admission still requires checkpoint construction and durable custody"]
pub struct AdmittedDeploymentFinalization<'a> {
    candidate: DeploymentFinalizationCandidate<'a>,
    checkpoint_head: CheckpointSigningAuthorityHeadV1,
    runtime_head: DeploymentRuntimeAuthorityHeadV1,
}

/// Why the selected finalization records cannot form the requested trust change.
#[derive(Debug, thiserror::Error)]
pub enum DeploymentFinalizationError {
    /// Existing record or canonical-set validation rejected an input.
    #[error(transparent)]
    Record(#[from] InvalidResidentRecordV1),
    /// The old arrays do not reproduce the predecessor's complete head roots.
    #[error("deployment finalization requires the complete predecessor signer heads")]
    CurrentHeads,
    /// Records disagree on principal, deployment, store, initial state or time.
    #[error("deployment finalization records do not match the selected scope and validity")]
    Scope,
    /// A new lineage or immutable record reference is already selected.
    #[error("deployment finalization reuses a selected lineage or record reference")]
    Conflict,
    /// The successor does not make exactly the requested signer insertions.
    #[error("deployment finalization changes unrelated trust or has different signer roots")]
    Successor,
}

impl<'a> AdmittedDeploymentFinalization<'a> {
    /// Check the exact initial signer insertions without changing any other trust head.
    ///
    /// Core owns record admission and canonical head hashing. This join proves
    /// complete predecessor membership and the permitted change, then retains
    /// the records for later physical checkpoint construction. No I/O occurs.
    ///
    /// # Errors
    /// Rejects malformed or incomplete head arrays, unlike scopes, expired
    /// inputs, reused lineages/references, or unrelated successor changes.
    pub fn admit(
        candidate: DeploymentFinalizationCandidate<'a>,
    ) -> Result<Self, DeploymentFinalizationError> {
        let prior = candidate.trust.predecessor;
        let next = candidate.trust.successor;
        if checkpoint_signing_authority_heads_root_sha256(candidate.checkpoint_heads)?
            != *prior.checkpoint_signing_authority_heads_root_sha256()
            || deployment_runtime_authority_heads_root_sha256(candidate.runtime_heads)?
                != *prior.deployment_runtime_authority_heads_root_sha256()
        {
            return Err(DeploymentFinalizationError::CurrentHeads);
        }
        candidate.verify_scope()?;
        candidate.verify_references()?;
        if !candidate
            .trust
            .preserves_other_heads(TrustHeadChange::DeploymentSigners)
        {
            return Err(DeploymentFinalizationError::Successor);
        }
        let checkpoint_head = candidate
            .checkpoint_signer
            .1
            .active_head(candidate.checkpoint_signer.0.clone())?;
        let runtime_head = candidate
            .runtime_authority
            .1
            .active_head(candidate.runtime_authority.0.clone())?;
        let checkpoint_index = candidate.checkpoint_heads.binary_search_by(|head| {
            head.checkpoint_signing_authority_id
                .cmp(&checkpoint_head.checkpoint_signing_authority_id)
        });
        let runtime_index = candidate.runtime_heads.binary_search_by(|head| {
            head.runtime_authority_id
                .cmp(&runtime_head.runtime_authority_id)
        });
        let (Err(checkpoint_index), Err(runtime_index)) = (checkpoint_index, runtime_index) else {
            return Err(DeploymentFinalizationError::Conflict);
        };
        // Only the selected insertions change these bounded cold-path arrays.
        let mut checkpoint_heads = candidate.checkpoint_heads.to_vec();
        checkpoint_heads.insert(checkpoint_index, checkpoint_head.clone());
        let mut runtime_heads = candidate.runtime_heads.to_vec();
        runtime_heads.insert(runtime_index, runtime_head.clone());
        if checkpoint_signing_authority_heads_root_sha256(&checkpoint_heads)?
            != *next.checkpoint_signing_authority_heads_root_sha256()
            || deployment_runtime_authority_heads_root_sha256(&runtime_heads)?
                != *next.deployment_runtime_authority_heads_root_sha256()
        {
            return Err(DeploymentFinalizationError::Successor);
        }
        Ok(Self {
            candidate,
            checkpoint_head,
            runtime_head,
        })
    }

    /// Borrow the exact admitted records and their checked predecessor head sets.
    #[must_use]
    pub const fn records(&self) -> &DeploymentFinalizationCandidate<'a> {
        &self.candidate
    }

    /// Borrow the single checkpoint-signer leaf to insert during construction.
    #[must_use]
    pub const fn checkpoint_signer_head(&self) -> &CheckpointSigningAuthorityHeadV1 {
        &self.checkpoint_head
    }

    /// Borrow the single runtime-authority leaf to insert during construction.
    #[must_use]
    pub const fn runtime_authority_head(&self) -> &DeploymentRuntimeAuthorityHeadV1 {
        &self.runtime_head
    }
}

impl DeploymentFinalizationCandidate<'_> {
    fn verify_scope(&self) -> Result<(), DeploymentFinalizationError> {
        let (binding_ref, binding) = self.binding;
        let signer = self.checkpoint_signer.1;
        let runtime = self.runtime_authority.1;
        binding.verify()?;
        signer.verify()?;
        runtime.verify()?;
        let issued_at = self.trust.successor.issued_at();
        if binding.deployment_terms().principal_scope() != self.trust.predecessor.principal_id()
            || signer.principal_id() != self.trust.predecessor.principal_id()
            || runtime.principal_id() != self.trust.predecessor.principal_id()
            || signer.scope()
                != &(CheckpointSigningScopeV1::Deployment {
                    deployment_binding_ref: binding_ref.clone(),
                    deployment_binding_sha256: binding.canonical_binding_sha256().clone(),
                })
            || runtime.deployment_binding_ref() != binding_ref
            || runtime.deployment_binding_sha256() != binding.canonical_binding_sha256()
            || signer.control_store_id() != runtime.control_store_id()
            || signer.control_incarnation_id() != runtime.control_incarnation_id()
            || runtime.control_store_id() == self.trust.predecessor.trust_store_id()
            || runtime.control_store_id() == self.trust.predecessor.principal_control_store_id()
            || signer.state() != &PrincipalAuthorityStateV1::Active
            || runtime.state() != &PrincipalAuthorityStateV1::Active
            || signer.signer_epoch().get() != 1
            || runtime.authority_epoch().get() != 1
            || signer.previous_authority_ref().is_some()
            || runtime.previous_authority_ref().is_some()
            || issued_at < binding.issued_at()
            || issued_at >= binding.expires_at()
            || issued_at < signer.issued_at()
            || issued_at >= signer.expires_at()
            || issued_at < runtime.issued_at()
            || issued_at >= runtime.expires_at()
        {
            return Err(DeploymentFinalizationError::Scope);
        }
        Ok(())
    }

    fn verify_references(&self) -> Result<(), DeploymentFinalizationError> {
        if self.checkpoint_heads.iter().any(|head| matches!(
            &head.scope,
            CheckpointSigningScopeV1::Deployment { deployment_binding_ref, deployment_binding_sha256 }
                if deployment_binding_ref == self.binding.0
                    || deployment_binding_sha256 == self.binding.1.canonical_binding_sha256()
        )) || self.runtime_heads.iter().any(|head| &head.deployment_binding_sha256 == self.binding.1.canonical_binding_sha256()) {
            return Err(DeploymentFinalizationError::Conflict);
        }
        let selected = [
            self.binding.0,
            self.checkpoint_signer.0,
            self.runtime_authority.0,
            self.trust.successor_ref,
        ];
        for (index, reference) in selected.iter().enumerate() {
            if selected[..index].contains(reference)
                || *reference == self.trust.predecessor_ref
                || self
                    .checkpoint_heads
                    .iter()
                    .any(|head| &head.authority_ref == *reference)
                || self
                    .runtime_heads
                    .iter()
                    .any(|head| &head.authority_ref == *reference)
            {
                return Err(DeploymentFinalizationError::Conflict);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/deployment_finalization.rs"]
pub(in crate::resident) mod tests;
