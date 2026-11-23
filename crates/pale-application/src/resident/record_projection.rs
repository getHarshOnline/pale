//! Encode admitted resident records for storage.
//!
//! The returned control records retain their schema and record identity.
//! Encoding does not prove that storage succeeded.

use pale_core::canon;
use pale_core::records::{CommonRequestEnvelopeV1, ResidentRecordRefV1, SchemaDigest, SchemaId};
use pale_ports::{
    ControlRecord, ControlRecordBytes, ControlRecordCoordinate, ControlRecordDigest,
    ControlRecordError,
};
use serde::Serialize;

/// Failure while encoding or projecting an admitted resident record.
#[derive(Debug, thiserror::Error)]
pub(super) enum ControlRecordProjectionError {
    /// Canonical signed-record encoding failed.
    #[error(transparent)]
    Canonicalize(#[from] canon::CanonError),
    /// The projected record digest or canonical byte string was malformed.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
}

/// Stable request tuple shared by effect and operation custody indexes.
///
/// Each index adds its own hash domain and the port supplies store incarnation.
/// Content digests do not belong in this tuple: changed signed contents under
/// the same identity must meet the existing entry and fail its exact comparison.
#[derive(Serialize)]
pub(super) struct RequestIdentityKeyPreimage<'a> {
    initial_v1_domain: &'static str,
    caller_identity: &'a str,
    signer_key_epoch: u64,
    request_id: &'a str,
}

impl<'a> From<&'a CommonRequestEnvelopeV1> for RequestIdentityKeyPreimage<'a> {
    fn from(request: &'a CommonRequestEnvelopeV1) -> Self {
        Self {
            initial_v1_domain: "request_identity",
            caller_identity: request.caller().identity.as_str(),
            signer_key_epoch: request.caller().signer_key_epoch.get(),
            request_id: request.request_id().as_str(),
        }
    }
}

/// Encode the signed request once without cloning its body to inspect its schema.
pub(super) fn request_record(
    record_ref: &ResidentRecordRefV1,
    request: &CommonRequestEnvelopeV1,
) -> Result<ControlRecord, ControlRecordProjectionError> {
    record(
        coordinate(record_ref, request.canonical_request_sha256().as_str())?,
        request,
    )
}

/// Retain admitted diagnostic bytes without re-encoding or redacting evidence.
/// The physical digest hashes these bytes; it is not the domain-separated SDK identity.
pub(super) fn diagnostic_record(
    record_ref: &ResidentRecordRefV1,
    diagnostic: &crate::diagnostic::AdmittedDiagnostic,
) -> Result<(pale_core::records::ArtifactDigest, ControlRecord), ControlRecordProjectionError> {
    let digest = pale_core::records::ArtifactDigest::hash_bytes(diagnostic.canonical_bytes());
    let record = ControlRecord::new(
        coordinate(record_ref, digest.as_str())?,
        diagnostic.wire().schema.clone(),
        diagnostic.wire().schema_sha256.clone(),
        ControlRecordBytes::new(diagnostic.canonical_bytes().to_vec())?,
    );
    Ok((digest, record))
}

/// Erase a family-specific digest only at the heterogeneous physical boundary.
pub(super) fn coordinate(
    record_ref: &ResidentRecordRefV1,
    digest: &str,
) -> Result<ControlRecordCoordinate, ControlRecordProjectionError> {
    Ok(ControlRecordCoordinate::new(
        record_ref.clone(),
        ControlRecordDigest::parse(digest)?,
    ))
}

/// Canonically encode one already-admitted signed record exactly once.
pub(super) fn record<R>(
    coordinate: ControlRecordCoordinate,
    value: &R,
) -> Result<ControlRecord, ControlRecordProjectionError>
where
    R: Serialize + ResidentRecordSchema,
{
    Ok(ControlRecord::new(
        coordinate,
        value.schema_id().clone(),
        value.schema_sha256().clone(),
        ControlRecordBytes::new(canon::to_canonical_string(value)?.into_bytes())?,
    ))
}

/// Canonically encode an admitted unsigned body under its enclosing schema coordinate.
///
/// [`pale_core::records::ActionEnvelopeV1`] and
/// [`pale_core::records::ReconciliationRequestV1`] deliberately carry no
/// resident-record schema fields of their own. Their caller-signed
/// [`pale_core::records::CommonRequestEnvelopeV1`] supplies the exact body
/// schema identity and digest. This is the sole physical projection for that
/// representation; it does not independently admit either body.
pub(super) fn record_with_schema<R>(
    coordinate: ControlRecordCoordinate,
    schema_id: SchemaId,
    schema_sha256: SchemaDigest,
    value: &R,
) -> Result<ControlRecord, ControlRecordProjectionError>
where
    R: Serialize,
{
    Ok(ControlRecord::new(
        coordinate,
        schema_id,
        schema_sha256,
        ControlRecordBytes::new(canon::to_canonical_string(value)?.into_bytes())?,
    ))
}

/// Schema access shared by heterogeneous admitted resident records.
pub(super) trait ResidentRecordSchema {
    /// Borrow the exact signed-record schema identity.
    fn schema_id(&self) -> &SchemaId;
    /// Borrow the exact selected schema digest.
    fn schema_sha256(&self) -> &SchemaDigest;
}

impl ResidentRecordSchema for CommonRequestEnvelopeV1 {
    fn schema_id(&self) -> &SchemaId {
        self.envelope_schema_id()
    }

    fn schema_sha256(&self) -> &SchemaDigest {
        self.envelope_schema_sha256()
    }
}

impl ResidentRecordSchema for pale_core::records::OperationLeaseV1 {
    fn schema_id(&self) -> &SchemaId {
        self.schema()
    }

    fn schema_sha256(&self) -> &SchemaDigest {
        self.schema_sha256()
    }
}

macro_rules! impl_resident_record_schema {
    ($($record:ty),+ $(,)?) => {
        $(
            impl ResidentRecordSchema for $record {
                fn schema_id(&self) -> &SchemaId { <$record>::schema_id(self) }
                fn schema_sha256(&self) -> &SchemaDigest { <$record>::schema_sha256(self) }
            }
        )+
    };
}

impl_resident_record_schema!(
    pale_core::records::A2aRouteAdmissionV1,
    pale_core::records::A2aTaskBindingV1,
    pale_core::records::A2aTaskObservationV1,
    pale_core::records::DeploymentBindingV1,
    pale_core::records::DeploymentControlStoreGenesisReceiptV1,
    pale_core::records::DeploymentRuntimeAuthorityV1,
    pale_core::records::LocalAdminCommandV1,
    pale_core::records::PrincipalRecordV1,
    pale_core::records::PrincipalRecoveryPolicyV1,
    pale_core::records::LocalAdminAuthorityV1,
    pale_core::records::ProducerIncarnationAuthorityV1,
    pale_core::records::CheckpointSigningAuthorityV1,
    pale_core::records::PrincipalTrustCheckpointV1,
    pale_core::records::ControlStateCheckpointV1,
    pale_core::records::PrincipalBootstrapReceiptV1,
    pale_core::records::PrincipalResidentWriterFenceV1,
    pale_core::records::PrincipalResidentWriterTerminalReceiptV1,
    pale_core::records::ProducerIncarnationV1,
    pale_core::records::ProducerIncarnationRetirementV1,
    pale_core::records::EffectConflictClaimV1,
    pale_core::records::EffectAdmissionV1,
    pale_core::records::EffectDispatchIntentV1,
    pale_core::records::EffectReceiptV1,
    pale_core::records::ReconciliationCaseV1,
    pale_core::records::ReconciliationCustodyV1,
    pale_core::records::ReconciliationResultV1,
    pale_core::records::EventReferenceV1,
    pale_core::records::EventAcknowledgementV1,
    pale_core::records::InteractionOccurrenceV1,
    pale_core::records::GenealogyEdgeV1,
    pale_core::records::ReconciliationFrontierV1,
    pale_core::records::OperationObservationV1,
    pale_core::records::OperationHandleV1,
    pale_core::records::AdapterFailureV1,
    pale_core::records::CapabilityResultV1,
);
