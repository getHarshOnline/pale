//! Conduct an outward `Delegate` through the canonical Pale SDK.
//!
//! This crate builds on `pale-sdk`; an embedding application calls it to start,
//! advance, observe or cancel delegated work.
//!
//! The application owns admission, durable custody, recovery, and the selected
//! A2A route. Protocol decoding and HTTPS mechanics belong to the native A2A
//! adapter. Callers must supply an authenticated resident attachment; this
//! interface does not discover a resident or duplicate its state machine.

use pale_sdk::{
    capability_inventory, A2aDelegateHandle, A2aDelegateHandleRequest, ApiCapability,
    ApiCapabilityId, ApiError, ApiLeaf, PaleApi, ResidentAttachment,
    ResidentDelegateAdvanceOutcome, ResidentDelegateAdvanceRequest, ResidentDelegateStartRequest,
    ResidentOperationCancellationOutcome, ResidentOperationCancellationRequest,
    ResidentOperationObservation, ResidentOperationObservationRequest,
};

/// Return the SDK capability for effect-free `Delegate` handle inspection.
///
/// The capability admits only a durable identity. It is not live delegation
/// availability and constructs no resident attachment or transport. `None`
/// reports an SDK/interface version mismatch instead of inventing readiness.
#[must_use]
pub fn delegate_handle_capability() -> Option<ApiCapability> {
    capability_inventory().iter().copied().find(|capability| {
        capability.id() == ApiCapabilityId::A2aDelegateHandleInspection
            && capability.eligible_for(ApiLeaf::A2a)
    })
}

/// Inspect one exact retained `Delegate` handle without launching it.
///
/// # Errors
///
/// Returns the SDK's structured invalid-input diagnostic when the reference,
/// canonical contract, self-carried digest, or operation class differs.
pub fn inspect_delegate_handle(
    api: &PaleApi,
    request: A2aDelegateHandleRequest,
) -> Result<A2aDelegateHandle, ApiError> {
    api.inspect_a2a_delegate_handle(request)
}

/// Start one already-authorized pairwise `Delegate` child.
///
/// The resident returns the durable child handle before its selected provider
/// or A2A route dispatches work. This interface neither selects that route nor
/// interprets an external A2A response.
///
/// # Errors
///
/// Returns the SDK's structured diagnostic when the parent, signed request,
/// attachment, or resident response fails admission.
pub fn start_delegate(
    api: &PaleApi,
    attachment: &ResidentAttachment,
    request: ResidentDelegateStartRequest,
) -> Result<ResidentOperationObservation, ApiError> {
    api.start_resident_delegate(Some(attachment), request)
}

/// Advance one retained `Delegate` through its already-selected dispatch owner.
///
/// Each call performs at most one resident-owned turn. A pending result is not
/// permission to repeat an uncertain external send.
///
/// # Errors
///
/// Returns the SDK's structured diagnostic when the request, attachment, or
/// resident response fails admission.
pub fn advance_delegate(
    api: &PaleApi,
    attachment: &ResidentAttachment,
    request: ResidentDelegateAdvanceRequest,
) -> Result<ResidentDelegateAdvanceOutcome, ApiError> {
    api.advance_resident_delegate(Some(attachment), request)
}

/// Observe the current signed records for one retained operation.
///
/// A remote A2A task state remains evidence about that task. It does not become
/// Pale completion, acceptance, settlement, or effect evidence.
///
/// # Errors
///
/// Returns the SDK's structured diagnostic when the request, attachment, or
/// returned records fail admission.
pub fn observe_operation(
    api: &PaleApi,
    attachment: &ResidentAttachment,
    request: ResidentOperationObservationRequest,
) -> Result<ResidentOperationObservation, ApiError> {
    api.observe_resident_operation(Some(attachment), request)
}

/// Request cancellation of one retained operation.
///
/// The returned disposition reports what the resident established. It never
/// turns a carrier acknowledgement or remote cancellation response into a
/// terminal Pale state.
///
/// # Errors
///
/// Returns the SDK's structured diagnostic when the request, attachment, or
/// resident response fails admission.
pub fn cancel_operation(
    api: &PaleApi,
    attachment: &ResidentAttachment,
    request: ResidentOperationCancellationRequest,
) -> Result<ResidentOperationCancellationOutcome, ApiError> {
    api.cancel_resident_operation(Some(attachment), request)
}
