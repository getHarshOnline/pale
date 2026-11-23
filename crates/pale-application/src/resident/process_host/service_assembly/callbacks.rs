//! Callbacks supplied by the deployment owner for record admission and signing.

use pale_core::records::*;
use pale_ports::ProviderInvocationRequest;

/// Check the exact predecessor and successor signing records.
pub type ResidentServiceRotationAdmission<'a> = dyn FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;

/// Sign one observation under its independently admitted authority.
pub type ResidentServiceObservationSigner<'a> = dyn FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;

/// Sign one control checkpoint under its independently admitted authority.
pub type ResidentServiceCheckpointSigner<'a> = dyn FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;

/// Sign one capability result under its independently admitted authority.
pub type ResidentServiceResultSigner<'a> = dyn FnMut(
        &[u8],
        &CapabilityResultPreimageV1,
        &CapabilityResultDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;

/// Admit the exact transport request and selected evidence.
pub type ResidentServiceTransportAdmission<'a> = dyn FnOnce(
        &ImmutableRecordReferenceV1,
        &ProviderInvocationRequest<'_>,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;

/// Sign one control checkpoint once under its independently admitted authority.
pub type ResidentServiceOneShotCheckpointSigner<'a> = dyn FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;

/// Admit the exact reconciliation frontier.
pub type ResidentServiceReconciliationAdmission<'a> =
    dyn FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1> + 'a;

/// Admit one historical signing rotation.
pub type ResidentServiceOneShotRotationAdmission<'a> = dyn FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;

/// Sign one observation once under its independently admitted authority.
pub type ResidentServiceOneShotObservationSigner<'a> = dyn FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;
