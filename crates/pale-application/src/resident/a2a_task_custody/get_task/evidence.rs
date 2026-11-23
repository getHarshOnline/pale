//! Join retained `GetTask` bytes to their binding and claimed remote state.

use super::super::reject;
use pale_core::records::{
    A2aTaskBindingV1, A2aTaskObservationPreimageV1, A2aTaskObservationSourceV1,
    ImmutableRecordReferenceV1, InvalidResidentRecordV1, ResidentRecordRefV1,
};
use pale_ports::{A2aGetTaskEvidencePort, OperationA2aEvidence};
use std::num::NonZeroUsize;

/// Validate the request and response behind one `GetTask` observation.
///
/// Use this same check when issuing or recovering an observation. It requires
/// exactly the two referenced evidence records, without SSE metadata. The
/// protocol port decodes those bytes and checks request/response correlation,
/// task identity and context. This function joins the decoded state to the
/// exact admitted binding and observation; it does not copy A2A wire rules.
///
/// This proves protocol meaning only. The caller must separately establish
/// authorized acquisition, HTTPS and endpoint evidence, observation time,
/// signature authority and custody. Historical route expiry does not invalidate
/// these retained bytes or grant permission to make another network request.
/// Remote completion is not a Pale result, effect or settlement.
///
/// # Errors
/// Rejects another source kind, substituted bindings or evidence, unary SSE
/// metadata, an aggregate byte limit violation, invalid protocol bytes, and a
/// claimed state that differs from the decoded response. No I/O occurs.
pub fn admit_a2a_get_task_evidence(
    binding: (&ResidentRecordRefV1, &A2aTaskBindingV1),
    observation: &A2aTaskObservationPreimageV1,
    evidence: &[&OperationA2aEvidence],
    maximum_bytes: NonZeroUsize,
    protocol: &dyn A2aGetTaskEvidencePort,
) -> Result<(), InvalidResidentRecordV1> {
    if observation.task_binding_ref != *binding.0
        || observation.task_binding_id != *binding.1.task_binding_id()
        || observation.task_binding_sha256 != *binding.1.canonical_a2a_task_binding_sha256()
    {
        return Err(reject("exact GetTask binding"));
    }
    let A2aTaskObservationSourceV1::GetTask { request, response } = &observation.source else {
        return Err(reject("GetTask source required"));
    };
    if evidence.len() != 2 || request.record_ref == response.record_ref {
        return Err(reject("two distinct GetTask evidence records"));
    }
    if evidence.iter().any(|value| value.sse().is_some()) {
        return Err(reject("unary GetTask evidence has no SSE metadata"));
    }
    if evidence[0]
        .byte_len()
        .checked_add(evidence[1].byte_len())
        .is_none_or(|bytes| bytes > maximum_bytes.get())
    {
        return Err(reject("bounded aggregate GetTask evidence"));
    }
    let select = |reference: &ImmutableRecordReferenceV1| {
        evidence.iter().find(|value| {
            value.coordinate().record_ref() == &reference.record_ref
                && value.coordinate().record_sha256().as_str() == reference.record_sha256.as_str()
        })
    };
    let request = select(request).ok_or_else(|| reject("exact GetTask request evidence"))?;
    let response = select(response).ok_or_else(|| reject("exact GetTask response evidence"))?;
    let state = protocol
        .read_task_state(
            request.bytes(),
            response.bytes(),
            binding.1.remote_task_id(),
            binding.1.remote_context_id(),
            maximum_bytes,
        )
        .map_err(|_| reject("valid correlated GetTask protocol evidence"))?;
    if state != observation.remote_state {
        return Err(reject("GetTask response state matches the observation"));
    }
    Ok(())
}
