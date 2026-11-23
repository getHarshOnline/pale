//! Test-only entry to the real local operation preparation phases.

use super::local_operation_start::{prepare_start_retained, StartBasis};
use super::*;
use crate::actions::NativeActionRegistry;
use crate::semantic::ResolvedOperation;
use pale_core::records::*;

/// Prepare a root or child operation through the production admission path.
///
/// Tests choose the operation class and optional native registry explicitly.
/// The helper performs no custody mutation and does not issue a locator.
#[allow(clippy::too_many_arguments)]
pub fn prepare_local_start<Authority, HandleSign, ObservationSign, SessionAuthority>(
    input: LocalOperationStartInput<'_>,
    parent: Option<PreparedLocalChildParent>,
    operation_class: OperationClassV1,
    registry: Option<&NativeActionRegistry>,
    session_authority: &mut SessionAuthority,
    admit_compiled_authority: Authority,
    sign_handle: HandleSign,
    sign_observation: ObservationSign,
) -> Result<PreparedLocalOperationStart, LocalOperationStartError>
where
    Authority: FnOnce(
        &CommonRequestEnvelopeV1,
        &AdmittedAmbientSession,
        &AmbientSessionAdmissionContext<'_>,
        &OperationHandlePreimageV1,
        &OperationLeaseV1,
        &ResolvedOperation,
        &LocalOperationPublicationPolicyInput<'_>,
        &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1>,
    SessionAuthority: AmbientSessionAuthorityPort,
    HandleSign: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    ObservationSign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let basis = match parent {
        Some(parent) => StartBasis::Child(Box::new(parent)),
        None => StartBasis::Root,
    };
    prepare_start_retained(
        input,
        (basis, operation_class, registry),
        |_, _, _| Ok(None),
        session_authority,
        admit_compiled_authority,
        sign_handle,
        sign_observation,
    )
    .map_err(super::local_operation_start::RetainedLocalOperationStartFailure::into_reason)
}

/// Retain an existing-evidence Consult plan through the real signed-start path.
/// Fresh-child plans require their separate identity allocator and are refused here.
#[allow(clippy::too_many_arguments)]
pub fn prepare_evidence_consult_start<Authority, HandleSign, ObservationSign, SessionAuthority>(
    input: LocalOperationStartInput<'_>,
    build_collection: fn(&OperationHandleV1) -> ConsultationCollectionAdmission,
    limits: ConsultationCollectionLimits,
    session_authority: &mut SessionAuthority,
    admit_compiled_authority: Authority,
    sign_handle: HandleSign,
    sign_observation: ObservationSign,
) -> Result<PreparedLocalOperationStart, LocalOperationStartError>
where
    Authority: FnOnce(
        &CommonRequestEnvelopeV1,
        &AdmittedAmbientSession,
        &AmbientSessionAdmissionContext<'_>,
        &OperationHandlePreimageV1,
        &OperationLeaseV1,
        &ResolvedOperation,
        &LocalOperationPublicationPolicyInput<'_>,
        &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1>,
    SessionAuthority: AmbientSessionAuthorityPort,
    HandleSign: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    ObservationSign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    use super::consultation::{
        ConsultationStartIntent, ConsultationStartIntentAdmission, ConsultationStartIntentSlot,
        ConsultationStartIntentSource,
    };
    prepare_start_retained(
        input,
        (StartBasis::Root, OperationClassV1::Consult, None),
        |store, reference, handle| {
            let collection = build_collection(handle);
            let slots = collection
                .slots
                .iter()
                .map(|slot| {
                    let ConsultationContributionSource::EvidenceResolved { evidence } =
                        slot.source()
                    else {
                        return Err(LocalOperationStartError::Coordinate);
                    };
                    Ok(ConsultationStartIntentSlot::new(
                        slot.slot_id().clone(),
                        slot.requirement(),
                        slot.requirement_evidence().clone(),
                        slot.participant().clone(),
                        slot.provider_binding().cloned(),
                        slot.context().cloned(),
                        ConsultationStartIntentSource::EvidenceResolved {
                            evidence: evidence.clone(),
                        },
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ConsultationStartIntent::admit_signed(
                store,
                reference,
                handle,
                ConsultationStartIntentAdmission {
                    completion_policy: collection.completion_policy,
                    opened_at: collection.opened_at,
                    deadline: collection.deadline,
                    slots,
                    limits,
                },
            )
            .map(Some)
            .map_err(|_| LocalOperationStartError::Coordinate)
        },
        session_authority,
        admit_compiled_authority,
        sign_handle,
        sign_observation,
    )
    .map_err(super::local_operation_start::RetainedLocalOperationStartFailure::into_reason)
}
