//! Real protocol qualification of retained `GetTask` bytes, without an HTTP send.
//! Initial task identification, route authority and other sources remain fixtures.

use super::*;
use std::num::NonZeroUsize;

pub(super) fn request() -> OperationA2aEvidence {
    raw(
        "a2a/get-request",
        b"{ \"jsonrpc\":\"2.0\", \"id\":\"get-1\", \"method\":\"GetTask\", \"params\":{\"id\":\"task-1\",\"historyLength\":0} }\n",
    )
}

pub(super) fn response(reference: &str, state: A2aRemoteTaskStateV1) -> OperationA2aEvidence {
    let state = match state {
        A2aRemoteTaskStateV1::Working => "TASK_STATE_WORKING",
        A2aRemoteTaskStateV1::Completed => "TASK_STATE_COMPLETED",
        _ => panic!("the existing GetTask journey supplies only these states"),
    };
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "jsonrpc":"2.0", "id":"get-1", "result": {
            "id":"task-1", "contextId":"context-1", "status":{"state":state}
        }
    }))
    .unwrap();
    raw(reference, &bytes)
}

pub(super) fn admit(
    binding: (&ResidentRecordRefV1, &A2aTaskBindingV1),
    observation: &A2aTaskObservationPreimageV1,
    evidence: &[&OperationA2aEvidence],
) -> Result<(), InvalidResidentRecordV1> {
    admit_a2a_get_task_evidence(
        binding,
        observation,
        evidence,
        NonZeroUsize::new(8192).unwrap(),
        &pale_adapter_a2a::A2aGetTaskDecoder,
    )
}

fn admit_with_limit(
    binding: (&ResidentRecordRefV1, &A2aTaskBindingV1),
    observation: &A2aTaskObservationPreimageV1,
    evidence: &[&OperationA2aEvidence],
    maximum_bytes: usize,
) -> Result<(), InvalidResidentRecordV1> {
    admit_a2a_get_task_evidence(
        binding,
        observation,
        evidence,
        NonZeroUsize::new(maximum_bytes).unwrap(),
        &pale_adapter_a2a::A2aGetTaskDecoder,
    )
}

pub(super) fn admit_append(
    binding: (&ResidentRecordRefV1, &A2aTaskBindingV1),
    observation: &A2aTaskObservationPreimageV1,
    new: &[OperationA2aEvidence],
    retained: &[OperationA2aEvidence],
) -> Result<(), InvalidResidentRecordV1> {
    if matches!(
        observation.source,
        A2aTaskObservationSourceV1::GetTask { .. }
    ) {
        let evidence = new.iter().chain(retained).collect::<Vec<_>>();
        admit(binding, observation, &evidence)
    } else {
        // The streaming fixture remains its explicit source admission model.
        Ok(())
    }
}

pub(super) struct Rejected {
    pub label: &'static str,
    pub observation: A2aTaskObservationPreimageV1,
    pub evidence: Vec<OperationA2aEvidence>,
}

pub(super) fn rejected(
    observation: &A2aTaskObservationPreimageV1,
    request: &OperationA2aEvidence,
    response: &OperationA2aEvidence,
) -> Vec<Rejected> {
    let mut result = Vec::new();
    let mut different_state = observation.clone();
    different_state.remote_state = A2aRemoteTaskStateV1::Completed;
    result.push(Rejected {
        label: "claimed remote state differs from decoded Task",
        observation: different_state,
        evidence: vec![request.clone(), response.clone()],
    });
    for (label, pointer, replacement) in [
        ("unlike RPC response", "/id", "other-get"),
        ("unlike Task", "/result/id", "other-task"),
        ("unlike context", "/result/contextId", "other-context"),
    ] {
        let mut body: serde_json::Value = serde_json::from_slice(response.bytes()).unwrap();
        *body.pointer_mut(pointer).unwrap() = replacement.into();
        let changed_response = raw(
            response.coordinate().record_ref().as_str(),
            &serde_json::to_vec_pretty(&body).unwrap(),
        );
        let mut changed = observation.clone();
        changed.source = A2aTaskObservationSourceV1::GetTask {
            request: fixture::evidence(request.coordinate().record_ref().as_str(), request.bytes()),
            response: fixture::evidence(
                changed_response.coordinate().record_ref().as_str(),
                changed_response.bytes(),
            ),
        };
        result.push(Rejected {
            label,
            observation: changed,
            evidence: vec![request.clone(), changed_response],
        });
    }
    let malformed = raw(
        response.coordinate().record_ref().as_str(),
        b"{ invalid JSON",
    );
    let mut malformed_observation = observation.clone();
    malformed_observation.source = A2aTaskObservationSourceV1::GetTask {
        request: fixture::evidence(request.coordinate().record_ref().as_str(), request.bytes()),
        response: fixture::evidence(
            malformed.coordinate().record_ref().as_str(),
            malformed.bytes(),
        ),
    };
    result.push(Rejected {
        label: "malformed response with exact retained digest",
        observation: malformed_observation,
        evidence: vec![request.clone(), malformed],
    });
    let mut unlike_reference = observation.clone();
    if let A2aTaskObservationSourceV1::GetTask { response, .. } = &mut unlike_reference.source {
        response.record_ref = record_ref("a2a/not-selected-response");
    }
    result.push(Rejected {
        label: "unlike selected response reference",
        observation: unlike_reference,
        evidence: vec![request.clone(), response.clone()],
    });
    result
}

pub(super) fn boundaries(
    binding: (&ResidentRecordRefV1, &A2aTaskBindingV1),
    observation: &A2aTaskObservationPreimageV1,
    request: &OperationA2aEvidence,
    response: &OperationA2aEvidence,
) {
    let exact = request.byte_len() + response.byte_len();
    admit_with_limit(binding, observation, &[request, response], exact).unwrap();
    admit_with_limit(binding, observation, &[response, request], exact).unwrap();
    assert!(admit_with_limit(binding, observation, &[request, response], exact - 1).is_err());
    assert!(admit_with_limit(
        (&record_ref("unlike-binding"), binding.1),
        observation,
        &[request, response],
        exact
    )
    .is_err());

    let mut cases = rejected(observation, request, response);
    for (label, changed) in [
        ("unlike binding ID", {
            let mut changed = observation.clone();
            changed.task_binding_id = A2aTaskBindingIdV1::parse("unlike-binding").unwrap();
            changed
        }),
        ("unlike binding digest", {
            let mut changed = observation.clone();
            changed.task_binding_sha256 =
                A2aTaskBindingDigest::hash_canonical(&"unlike binding").unwrap();
            changed
        }),
        ("unlike source kind", {
            let mut changed = observation.clone();
            changed.source = A2aTaskObservationSourceV1::InitialSendMessageResponse {
                evidence: fixture::evidence(
                    response.coordinate().record_ref().as_str(),
                    response.bytes(),
                ),
            };
            changed
        }),
        ("same reference for both roles", {
            let mut changed = observation.clone();
            let reference =
                fixture::evidence(request.coordinate().record_ref().as_str(), request.bytes());
            changed.source = A2aTaskObservationSourceV1::GetTask {
                request: reference.clone(),
                response: reference,
            };
            changed
        }),
    ] {
        cases.push(Rejected {
            label,
            observation: changed,
            evidence: vec![request.clone(), response.clone()],
        });
    }
    for (label, evidence) in [
        ("omitted response", vec![request.clone()]),
        ("duplicate request", vec![request.clone(), request.clone()]),
        (
            "extra evidence",
            vec![
                request.clone(),
                response.clone(),
                raw("a2a/extra", b"extra"),
            ],
        ),
        (
            "altered bytes and coordinate",
            vec![
                request.clone(),
                raw(
                    response.coordinate().record_ref().as_str(),
                    b"different exact bytes",
                ),
            ],
        ),
        (
            "unary SSE metadata",
            vec![
                request.clone(),
                stream_raw(
                    response.coordinate().record_ref().as_str(),
                    response.bytes(),
                    "not-unary",
                ),
            ],
        ),
    ] {
        cases.push(Rejected {
            label,
            observation: observation.clone(),
            evidence,
        });
    }
    for rejected in cases {
        let evidence = rejected.evidence.iter().collect::<Vec<_>>();
        assert!(
            admit_with_limit(binding, &rejected.observation, &evidence, 8192).is_err(),
            "{}",
            rejected.label
        );
    }
}
