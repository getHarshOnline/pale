//! Reconstruct saved native responses as history, without renewing effect authority.

use super::*;
use crate::resident::{RecoveredOperationObservation, ResidentNativeToolStart};

/// The protected response phase, not evidence of emission or host acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum RecoveredHookResponsePhase {
    Prepared,
    Dispatched,
}

/// Exact saved output rejoined to the protected operation and native occurrence.
/// The enclosing recovery owner retains the original signed request.
/// This owner has no conversion to a releasable frame or live hook registration.
pub(in crate::resident) struct RecoveredNativeHookResponse {
    pub(in crate::resident) phase: RecoveredHookResponsePhase,
    pub(in crate::resident) record: ControlRecord,
}

impl std::fmt::Debug for RecoveredNativeHookResponse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecoveredNativeHookResponse")
            .field("phase", &self.phase)
            .field(
                "response_bytes",
                &self.record.canonical_record_jcs().as_slice().len(),
            )
            .finish_non_exhaustive()
    }
}

/// Read the response named by protected observation history and recheck its head.
/// Original session and operation leases may have expired: this read neither
/// re-admits permission nor registers the occurrence for another native effect.
pub(in crate::resident) fn recover_native_hook_response(
    operation: &IssuedLocalOperation,
    request: &CommonRequestEnvelopeV1,
    observation: &RecoveredOperationObservation,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_response_bytes: usize,
    maximum_record_bytes: usize,
) -> Result<RecoveredNativeHookResponse, OperationAttemptError> {
    let selected = observation.selected().1;
    let (phase, attempt, evidence) = match selected.state() {
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::DispatchPrepared {
                    attempt_id,
                    evidence,
                },
        } => (RecoveredHookResponsePhase::Prepared, attempt_id, evidence),
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::Dispatched {
                    attempt_id,
                    evidence,
                },
        } => (RecoveredHookResponsePhase::Dispatched, attempt_id, evidence),
        _ => return Err(OperationAttemptError::Association),
    };
    if !operation
        .checkpoint_selection()
        .matches_store(observation.store())
        || observation.selected().1.operation_handle_ref() != operation.handle().0
        || observation.selected().1.operation_handle_sha256()
            != operation.handle().1.canonical_operation_handle_sha256()
    {
        return Err(OperationAttemptError::Association);
    }
    if operation.handle().1.operation_class() != &OperationClassV1::Invoke
        || operation.handle().1.parent_operation().is_some()
    {
        return Err(OperationAttemptError::Association);
    }
    if maximum_record_bytes == 0 || maximum_response_bytes == 0 {
        return Err(OperationAttemptError::BoundExceeded);
    }
    let native = ResidentNativeToolStart::admit(request.body())
        .map_err(|_| OperationAttemptError::Association)?;
    let session = operation.historical_session_lease();
    let event = native.request().event();
    let activity = HarnessActivityIdentity::try_from_host(event.session_id(), event.turn_id())
        .map_err(|_| OperationAttemptError::Association)?;
    if request.canonical_request_sha256().as_str()
        != operation.request_coordinate().record_sha256().as_str()
        || native
            .request()
            .routing()
            .identity()
            .workspace_id()
            .as_str()
            != request.workspace_id().as_str()
        || native.request().routing().identity().repository_id() != session.repository_id()
        || !matches!(session.native_session_correlation(), NativeSessionCorrelationV1::Observed { native_session_identity, .. } if Some(native_session_identity) == activity.as_ref())
        || !matches!(operation.historical_operation_lease().invocation_correlation(), InvocationCorrelationV1::Observed { native_invocation_id, .. } if native_invocation_id == native.invocation())
    {
        return Err(OperationAttemptError::Association);
    }
    if phase == RecoveredHookResponsePhase::Dispatched {
        let predecessor = observation
            .selected_predecessor()
            .ok_or(OperationAttemptError::Association)?
            .1;
        if !matches!(predecessor.state(), OperationObservationStateV1::Active { phase: OperationActivePhaseV1::DispatchPrepared { attempt_id, evidence: prepared } } if attempt_id == attempt && prepared == evidence)
        {
            return Err(OperationAttemptError::Association);
        }
    }
    let coordinate = ControlRecordCoordinate::new(
        evidence.record_ref.clone(),
        ControlRecordDigest::parse(evidence.record_sha256.as_str())
            .map_err(|_| OperationAttemptError::Association)?,
    );
    let record = custody
        .read_operation_record(observation.store(), &coordinate, maximum_response_bytes)
        .map_err(crate::resident::OperationDispatchCustodyError::from)?
        .ok_or(OperationAttemptError::Association)?;
    let bytes = record.canonical_record_jcs().as_slice();
    if bytes.len() > maximum_response_bytes {
        return Err(OperationAttemptError::BoundExceeded);
    }
    let value =
        pale_core::canon::parse_strict(bytes).map_err(|_| OperationAttemptError::Encoding)?;
    let response: ResidentHookResponseRecord =
        serde_json::from_value(value).map_err(|_| OperationAttemptError::Encoding)?;
    let canonical = pale_core::canon::to_canonical_string(&response)
        .map_err(|_| OperationAttemptError::Encoding)?;
    if record.coordinate() != &coordinate
        || response.schema_id.as_str() != pale_ports::PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA
        || response.handle_sha256 != *operation.handle().1.canonical_operation_handle_sha256()
        || response.native_request_sha256 != request.body().payload_sha256
        || canonical.as_bytes() != bytes
        || response_record(evidence.record_ref.clone(), &response, &canonical)? != record
    {
        return Err(OperationAttemptError::Association);
    }
    super::super::verify_recovered_operation_head_custody(
        operation,
        observation,
        custody,
        anchor,
        maximum_record_bytes,
    )?;
    Ok(RecoveredNativeHookResponse { phase, record })
}
