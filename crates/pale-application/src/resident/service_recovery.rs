//! Admitted trust evidence for resident service assembly.
//!
//! This module projects historical trust recovery into one opaque service
//! input. Its physical path reads only the trust-store binding fixed by the
//! admitted bootstrap. It does not discover a store, endpoint, current key,
//! writer, deployment, or process policy.

#[cfg(feature = "interface-test-support")]
use pale_core::records::PrincipalTrustCheckpointV1;
use pale_core::records::{
    AdmittedPrincipalBootstrapV1, CanonicalSignatureV1, CheckpointSigningAuthorityV1,
    DeploymentBindingV1, DeploymentRuntimeAuthorityV1, Ed25519PublicKeyV1, InvalidResidentRecordV1,
    PrincipalKeyEpochV1, PrincipalTrustCheckpointDigest, PrincipalTrustCheckpointPreimageV1,
    ResidentKeyIdV1, ResidentPrincipalIdV1, ResidentRecordRefV1, SchemaDigest,
};
use pale_ports::{
    ControlRecord, DeploymentFinalizationSetReadLimits, DeploymentFinalizationSetReadPort,
    DeploymentFinalizationSetSnapshot, PrincipalProducerIncarnationReadPort,
    PrincipalTrustCheckpointSetReadLimits, PrincipalTrustCheckpointSetReadPort,
    PrincipalTrustCheckpointSetSnapshot, PrincipalTrustStoreBinding,
};

use super::{
    principal_trust_frontier::{
        recover_current_principal_trust_frontier, AdmittedHistoricalDeploymentFinalizationRecords,
        HistoricalDeploymentFinalizationAdmission, RecoveredCurrentPrincipalTrustFrontier,
    },
    principal_trust_recovery::{
        recover_signed_principal_trust_checkpoint_chain,
        HistoricalPrincipalCheckpointSignatureVerifier, HistoricalPrincipalKeyResolution,
        HistoricalPrincipalKeyResolver,
    },
    recover_principal_producer_incarnation_frontier, CurrentPrincipalTrustHead,
    PrincipalProducerIncarnationRecoveryAdmission, RecoveredPrincipalProducerIncarnationFrontier,
};

/// Limits and admitted bootstrap evidence for physical trust recovery.
pub struct ResidentServicePhysicalTrustRecoveryInput {
    /// Bootstrap admission that fixes the principal and trust-store binding.
    pub bootstrap: AdmittedPrincipalBootstrapV1,
    /// Embedded schema identity expected for every trust checkpoint.
    pub checkpoint_schema_sha256: SchemaDigest,
    /// Item and aggregate-byte limits for checkpoint recovery.
    pub checkpoint_limits: PrincipalTrustCheckpointSetReadLimits,
    /// Maximum producer heads copied in one physical page.
    pub maximum_producer_page_items: usize,
    /// Count, byte, and mutation limits for finalization recovery.
    pub finalization_limits: DeploymentFinalizationSetReadLimits,
}

/// Separate physical readers used by one trust-recovery attempt.
pub struct ResidentServiceTrustPhysicalPorts<'a> {
    /// Physical checkpoint-set reader.
    pub checkpoints: &'a mut dyn PrincipalTrustCheckpointSetReadPort,
    /// Physical producer-incarnation reader.
    pub producers: &'a mut dyn PrincipalProducerIncarnationReadPort,
    /// Physical deployment-finalization reader.
    pub finalizations: &'a mut dyn DeploymentFinalizationSetReadPort,
}

/// Bounded physical and already-admitted semantic evidence for trust closure.
pub struct ResidentServiceTrustRecoveryInput {
    /// Bootstrap admission that owns the initial trust checkpoint.
    pub bootstrap: AdmittedPrincipalBootstrapV1,
    /// Complete physical checkpoint evidence through the selected current head.
    pub checkpoints: PrincipalTrustCheckpointSetSnapshot,
    /// Embedded schema identity expected for every trust checkpoint.
    pub checkpoint_schema_sha256: SchemaDigest,
    /// Item and aggregate-byte limits for checkpoint recovery.
    pub checkpoint_limits: PrincipalTrustCheckpointSetReadLimits,
    /// Independently recovered producer-incarnation frontier.
    pub producers: RecoveredPrincipalProducerIncarnationFrontier,
    /// Exact physical deployment-finalization set, when one exists.
    pub finalizations: Option<DeploymentFinalizationSetSnapshot>,
}

/// Result of resolving one record-time principal key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResidentServiceHistoricalKeyResolution {
    /// The exact historical public key was found.
    Resolved(Ed25519PublicKeyV1),
    /// No evidence exists for the exact key coordinate.
    Unavailable,
    /// Available evidence assigns unlike keys to the same coordinate.
    Conflicting,
}

/// Record-time principal-key resolver used during trust recovery.
pub trait ResidentServiceHistoricalKeyPort {
    /// Resolve one exact record-time principal key.
    fn resolve_historical_principal_key(
        &mut self,
        principal_id: &ResidentPrincipalIdV1,
        key_id: &ResidentKeyIdV1,
        key_epoch: PrincipalKeyEpochV1,
    ) -> ResidentServiceHistoricalKeyResolution;
}

/// Cryptographic verifier for historical trust-checkpoint signatures.
pub trait ResidentServiceTrustSignaturePort {
    /// Verify one exact canonical trust-checkpoint signature.
    ///
    /// # Errors
    ///
    /// Returns an error when the signature does not verify under the supplied
    /// historical key and domain.
    fn verify_historical_checkpoint_signature(
        &mut self,
        domain: &[u8],
        public_key: &Ed25519PublicKeyV1,
        preimage: &PrincipalTrustCheckpointPreimageV1,
        digest: &PrincipalTrustCheckpointDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>;
}

/// Record-specific admission for one historical deployment finalization.
pub trait ResidentServiceDeploymentAuthorityAdmissionPort {
    /// Admit the three records retained by one historical deployment finalization.
    ///
    /// # Errors
    ///
    /// Returns an error when a record is invalid or the three records do not
    /// form one deployment-authority association.
    fn admit_historical_deployment_finalization(
        &mut self,
        binding: &ControlRecord,
        checkpoint_signer: &ControlRecord,
        runtime_authority: &ControlRecord,
    ) -> Result<ResidentServiceDeploymentAuthorityRecords, InvalidResidentRecordV1>;
}

/// Exact evidence owners selected for one trust-recovery attempt.
pub struct ResidentServiceTrustEvidencePorts<'a> {
    /// Historical principal-key owner.
    pub historical_keys: &'a mut dyn ResidentServiceHistoricalKeyPort,
    /// Historical trust-checkpoint signature verifier.
    pub checkpoint_signatures: &'a mut dyn ResidentServiceTrustSignaturePort,
    /// Deployment finalization record admission owner.
    pub deployment_authorities: &'a mut dyn ResidentServiceDeploymentAuthorityAdmissionPort,
}

/// Typed records admitted from one historical deployment finalization.
pub struct ResidentServiceDeploymentAuthorityRecords {
    /// Immutable deployment-binding reference and admitted record.
    pub binding: (ResidentRecordRefV1, DeploymentBindingV1),
    /// Immutable checkpoint-signer reference and admitted record.
    pub checkpoint_signer: (ResidentRecordRefV1, CheckpointSigningAuthorityV1),
    /// Immutable runtime-authority reference and admitted record.
    pub runtime_authority: (ResidentRecordRefV1, DeploymentRuntimeAuthorityV1),
}

/// Opaque admitted trust frontier used only by resident service assembly.
pub struct ResidentServiceTrustFrontier(RecoveredCurrentPrincipalTrustFrontier);

impl ResidentServiceTrustFrontier {
    /// Build the one-record trust view used by the in-memory interface proof.
    ///
    /// All supplied records must already be admitted by their owning record
    /// families. This feature-only constructor does not recover a historical
    /// trust chain, compare an independent anchor, or prove production
    /// authentication. It exists only so interface conformance can execute the
    /// real resident service without importing application test modules.
    #[cfg(feature = "interface-test-support")]
    pub(in crate::resident) fn for_interface_conformance(
        checkpoint_ref: ResidentRecordRefV1,
        checkpoint: PrincipalTrustCheckpointV1,
        binding_ref: ResidentRecordRefV1,
        binding: DeploymentBindingV1,
        runtime_authority_ref: ResidentRecordRefV1,
        runtime_authority: DeploymentRuntimeAuthorityV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        super::principal_trust_frontier::RecoveredCurrentPrincipalTrustFrontier::for_interface_conformance(
            checkpoint_ref,
            checkpoint,
            binding_ref,
            binding,
            runtime_authority_ref,
            runtime_authority,
        )
        .map(Self)
    }

    /// Recover and close one exact principal trust frontier.
    ///
    /// # Errors
    ///
    /// Returns an error when checkpoint signatures, chain order, retained
    /// finalization records, producer roots, or active authority roots disagree.
    pub fn recover(
        input: ResidentServiceTrustRecoveryInput,
        evidence: ResidentServiceTrustEvidencePorts<'_>,
    ) -> Result<Self, ResidentServiceTrustRecoveryError> {
        let bootstrap = input.bootstrap.atom_view();
        let (initial_checkpoint_ref, initial_checkpoint) = bootstrap.trust_checkpoint_entry();
        let mut keys = HistoricalKeyAdapter(evidence.historical_keys);
        let mut signatures = TrustSignatureAdapter(evidence.checkpoint_signatures);
        let checkpoints = recover_signed_principal_trust_checkpoint_chain(
            input.checkpoints,
            initial_checkpoint_ref,
            initial_checkpoint,
            &input.checkpoint_schema_sha256,
            input.checkpoint_limits,
            &mut keys,
            &mut signatures,
        )
        .map_err(|_| ResidentServiceTrustRecoveryError::CheckpointChain)?;
        let mut deployment_authorities =
            DeploymentAuthorityAdmissionAdapter(evidence.deployment_authorities);
        recover_current_principal_trust_frontier(
            &input.bootstrap,
            checkpoints,
            input.producers,
            input.finalizations,
            &mut deployment_authorities,
        )
        .map(Self)
        .map_err(|_| ResidentServiceTrustRecoveryError::Frontier)
    }

    /// Read, admit, and close one physical principal trust frontier.
    ///
    /// The admitted bootstrap fixes the only store binding used by every read.
    /// Checkpoints are admitted first. Producer history is then recovered under
    /// the exact admitted current checkpoint. Finalizations are read last before
    /// the existing trust-frontier closure checks all three evidence sets.
    ///
    /// # Errors
    ///
    /// Returns an error when a physical read fails or is absent, a supplied
    /// limit is invalid, or any recovered record, signature, root, or authority
    /// association is inconsistent.
    pub fn recover_from_physical(
        input: ResidentServicePhysicalTrustRecoveryInput,
        physical: ResidentServiceTrustPhysicalPorts<'_>,
        producer_admission: &mut dyn PrincipalProducerIncarnationRecoveryAdmission,
        evidence: ResidentServiceTrustEvidencePorts<'_>,
    ) -> Result<Self, ResidentServiceTrustRecoveryError> {
        let bootstrap_view = input.bootstrap.atom_view();
        let (_, initial_checkpoint) = bootstrap_view.trust_checkpoint_entry();
        let binding = PrincipalTrustStoreBinding::new(
            initial_checkpoint.trust_store_id().clone(),
            initial_checkpoint.trust_store_incarnation_id().clone(),
            initial_checkpoint.principal_id().clone(),
        );
        let checkpoints = physical
            .checkpoints
            .read_checkpoint_set(&binding, input.checkpoint_limits)
            .map_err(|_| ResidentServiceTrustRecoveryError::CheckpointRead)?
            .ok_or(ResidentServiceTrustRecoveryError::CheckpointUnavailable)?;

        let (initial_checkpoint_ref, initial_checkpoint) = bootstrap_view.trust_checkpoint_entry();
        let mut keys = HistoricalKeyAdapter(evidence.historical_keys);
        let mut signatures = TrustSignatureAdapter(evidence.checkpoint_signatures);
        let checkpoints = recover_signed_principal_trust_checkpoint_chain(
            checkpoints,
            initial_checkpoint_ref,
            initial_checkpoint,
            &input.checkpoint_schema_sha256,
            input.checkpoint_limits,
            &mut keys,
            &mut signatures,
        )
        .map_err(|_| ResidentServiceTrustRecoveryError::CheckpointChain)?;

        let current = checkpoints.current();
        let producers = recover_principal_producer_incarnation_frontier(
            physical.producers,
            binding.clone(),
            CurrentPrincipalTrustHead {
                checkpoint_ref: current.coordinate().record_ref(),
                checkpoint: current.checkpoint(),
            },
            input.maximum_producer_page_items,
            producer_admission,
        )
        .map_err(|_| ResidentServiceTrustRecoveryError::ProducerFrontier)?;

        let finalizations = physical
            .finalizations
            .read_finalization_set(&binding, input.finalization_limits)
            .map_err(|_| ResidentServiceTrustRecoveryError::FinalizationRead)?;
        let mut deployment_authorities =
            DeploymentAuthorityAdmissionAdapter(evidence.deployment_authorities);
        recover_current_principal_trust_frontier(
            &input.bootstrap,
            checkpoints,
            producers,
            finalizations,
            &mut deployment_authorities,
        )
        .map(Self)
        .map_err(|_| ResidentServiceTrustRecoveryError::Frontier)
    }

    pub(in crate::resident) const fn inner(&self) -> &RecoveredCurrentPrincipalTrustFrontier {
        &self.0
    }
}

/// Why resident service trust evidence could not be closed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ResidentServiceTrustRecoveryError {
    /// Physical checkpoint evidence could not be read completely.
    #[error("resident service trust checkpoint evidence could not be read")]
    CheckpointRead,
    /// The selected trust store has no checkpoint evidence.
    #[error("resident service trust checkpoint evidence is unavailable")]
    CheckpointUnavailable,
    /// Historical checkpoints did not form one bounded signature-verified chain.
    #[error("resident service trust checkpoint chain is invalid")]
    CheckpointChain,
    /// Physical producer history did not reproduce the admitted current root.
    #[error("resident service producer frontier is invalid")]
    ProducerFrontier,
    /// Physical deployment-finalization evidence could not be read completely.
    #[error("resident service deployment finalizations could not be read")]
    FinalizationRead,
    /// Admitted checkpoint, producer, and finalization evidence did not close.
    #[error("resident service trust frontier is invalid")]
    Frontier,
}

struct HistoricalKeyAdapter<'a>(&'a mut dyn ResidentServiceHistoricalKeyPort);

impl HistoricalPrincipalKeyResolver for HistoricalKeyAdapter<'_> {
    fn resolve_historical_principal_key(
        &mut self,
        principal_id: &ResidentPrincipalIdV1,
        key_id: &ResidentKeyIdV1,
        key_epoch: PrincipalKeyEpochV1,
    ) -> HistoricalPrincipalKeyResolution {
        match self
            .0
            .resolve_historical_principal_key(principal_id, key_id, key_epoch)
        {
            ResidentServiceHistoricalKeyResolution::Resolved(key) => {
                HistoricalPrincipalKeyResolution::Resolved(key)
            }
            ResidentServiceHistoricalKeyResolution::Unavailable => {
                HistoricalPrincipalKeyResolution::Unavailable
            }
            ResidentServiceHistoricalKeyResolution::Conflicting => {
                HistoricalPrincipalKeyResolution::Conflicting
            }
        }
    }
}

struct TrustSignatureAdapter<'a>(&'a mut dyn ResidentServiceTrustSignaturePort);

impl HistoricalPrincipalCheckpointSignatureVerifier for TrustSignatureAdapter<'_> {
    fn verify_historical_checkpoint_signature(
        &mut self,
        domain: &[u8],
        public_key: &Ed25519PublicKeyV1,
        preimage: &PrincipalTrustCheckpointPreimageV1,
        digest: &PrincipalTrustCheckpointDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.0
            .verify_historical_checkpoint_signature(domain, public_key, preimage, digest, signature)
    }
}

struct DeploymentAuthorityAdmissionAdapter<'a>(
    &'a mut dyn ResidentServiceDeploymentAuthorityAdmissionPort,
);

impl HistoricalDeploymentFinalizationAdmission for DeploymentAuthorityAdmissionAdapter<'_> {
    fn admit_historical_deployment_finalization(
        &mut self,
        binding: &ControlRecord,
        checkpoint_signer: &ControlRecord,
        runtime_authority: &ControlRecord,
    ) -> Result<AdmittedHistoricalDeploymentFinalizationRecords, InvalidResidentRecordV1> {
        let admitted = self.0.admit_historical_deployment_finalization(
            binding,
            checkpoint_signer,
            runtime_authority,
        )?;
        Ok(AdmittedHistoricalDeploymentFinalizationRecords::new(
            admitted.binding.0,
            admitted.binding.1,
            admitted.checkpoint_signer.0,
            admitted.checkpoint_signer.1,
            admitted.runtime_authority.0,
            admitted.runtime_authority.1,
        ))
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/service_recovery.rs"]
mod tests;
