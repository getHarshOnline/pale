//! Current-authority proofs required by ambient-session admission.
//!
//! Application policy validates record structure and relationships. This port
//! supplies only facts that require durable owner custody: current-head
//! lineage, checkpoint membership, revocation-frontier completeness, and the
//! owner-defined data-classification order. It neither resolves arbitrary
//! records nor grants session authority.

use pale_core::records::{
    ArtifactDigest, CanonicalSignatureV1, CapabilityRevocationCheckpointV1,
    CapabilitySessionGrantRevocationV1, CapabilitySessionGrantV1, DataClassificationV1,
    DeploymentAcceptanceRevocationEpochV1, DeploymentAcceptanceRevocationV1, DeploymentBindingV1,
    DeploymentRuntimeAuthorityV1, InvalidResidentRecordV1, PrincipalRevocationEpochV1,
    PrincipalTrustCheckpointV1, ResidentPrincipalIdV1, ResidentRecordRefV1,
    SessionAdmissionLeaseDigest, SessionAdmissionLeasePreimageV1,
};

/// A current authority fact required for ambient admission is not proved.
///
/// Each variant identifies a distinct owner frontier so application policy can
/// preserve the correct recovery route instead of flattening every uncertainty
/// into generic unavailability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AmbientSessionAuthorityError {
    /// The current principal checkpoint is not proved to descend from the
    /// issuance checkpoint.
    #[error("principal trust-checkpoint lineage is stale or unproved")]
    PrincipalTrustCheckpoint,
    /// The capability checkpoint or selected grant-revocation head is not proved current.
    #[error("capability-revocation frontier is stale or unproved")]
    CapabilityRevocationFrontier,
    /// The deployment-acceptance epoch or selected revocation head is not proved current.
    #[error("deployment-acceptance revocation frontier is stale or unproved")]
    DeploymentRevocationFrontier,
    /// The runtime authority is not proved to belong to the current trust-checkpoint head set.
    #[error("deployment-runtime-authority head is stale or unproved")]
    RuntimeAuthorityHead,
    /// The session-revocation coordinate is not proved to be the current principal frontier.
    #[error("session-revocation frontier is stale or unproved")]
    SessionRevocationFrontier,
    /// The deployment classification is not proved to be within the grant ceiling.
    #[error("deployment classification is outside or unproved beneath the grant ceiling")]
    DataClassificationCeiling,
}

/// Narrow owner boundary for current ambient-session authority.
///
/// Implementations prove only the named current-frontier relationships over
/// already admitted records. The application remains the sole owner of the
/// admission decision and independently checks relationships expressible by
/// those records.
pub trait AmbientSessionAuthorityPort {
    /// Verify the lease signature with the selected deployment-runtime key.
    ///
    /// # Errors
    /// Returns the record-owned admission error when the signature is not
    /// valid for the exact domain, preimage, digest, and runtime authority.
    fn verify_session_admission_lease_signature(
        &mut self,
        domain: &[u8],
        preimage: &SessionAdmissionLeasePreimageV1,
        digest: &SessionAdmissionLeaseDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>;

    /// Prove that `current` is the active descendant of, or equal to, `issued`.
    ///
    /// # Errors
    /// Returns [`AmbientSessionAuthorityError::PrincipalTrustCheckpoint`] when
    /// lineage is unproved.
    fn prove_current_principal_trust(
        &mut self,
        issued_ref: &ResidentRecordRefV1,
        issued: &PrincipalTrustCheckpointV1,
        current_ref: &ResidentRecordRefV1,
        current: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError>;

    /// Prove the exact checkpoint and optional grant head form the complete current frontier.
    ///
    /// # Errors
    /// Returns [`AmbientSessionAuthorityError::CapabilityRevocationFrontier`]
    /// when freshness or completeness is unproved.
    fn prove_capability_revocation_frontier(
        &mut self,
        checkpoint_ref: &ResidentRecordRefV1,
        checkpoint: &CapabilityRevocationCheckpointV1,
        grant_ref: &ResidentRecordRefV1,
        grant: &CapabilitySessionGrantV1,
        revocation_ref: Option<&ResidentRecordRefV1>,
        revocation: Option<&CapabilitySessionGrantRevocationV1>,
    ) -> Result<(), AmbientSessionAuthorityError>;

    /// Prove the exact deployment epoch and optional binding-specific head are current.
    ///
    /// # Errors
    /// Returns [`AmbientSessionAuthorityError::DeploymentRevocationFrontier`]
    /// when the owner frontier is unproved.
    fn prove_deployment_revocation_frontier(
        &mut self,
        binding: &DeploymentBindingV1,
        epoch: DeploymentAcceptanceRevocationEpochV1,
        revocation_ref: Option<&ResidentRecordRefV1>,
        revocation: Option<&DeploymentAcceptanceRevocationV1>,
        current_trust: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError>;

    /// Prove that `authority` is a member of the current checkpoint's runtime-authority head set.
    ///
    /// # Errors
    /// Returns [`AmbientSessionAuthorityError::RuntimeAuthorityHead`] when membership is unproved.
    fn prove_runtime_authority_head(
        &mut self,
        authority_ref: &ResidentRecordRefV1,
        authority: &DeploymentRuntimeAuthorityV1,
        current_trust: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError>;

    /// Prove the exact principal-scoped session-revocation frontier is current.
    ///
    /// # Errors
    /// Returns [`AmbientSessionAuthorityError::SessionRevocationFrontier`]
    /// when the principal frontier is unproved.
    fn prove_session_revocation_frontier(
        &mut self,
        principal_id: &ResidentPrincipalIdV1,
        epoch: PrincipalRevocationEpochV1,
        revocation_ref: Option<&ResidentRecordRefV1>,
        revocation_sha256: Option<&ArtifactDigest>,
        current_trust: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError>;

    /// Prove the owner-defined classification relation without ordering opaque names.
    ///
    /// # Errors
    /// Returns [`AmbientSessionAuthorityError::DataClassificationCeiling`]
    /// when the relation is unproved.
    fn prove_data_classification_ceiling(
        &mut self,
        deployment: &DataClassificationV1,
        ceiling: &DataClassificationV1,
    ) -> Result<(), AmbientSessionAuthorityError>;
}
