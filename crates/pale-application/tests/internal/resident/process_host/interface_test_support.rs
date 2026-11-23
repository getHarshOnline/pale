//! Resident interface conformance tests.

use pale_ports::{
    ResidentOperationFrameFailure, ResidentOperationFrameRequest, ResidentOperationFrameResponse,
};

use super::*;

#[path = "interface_consult_restart.rs"]
mod consult_restart;

impl OperationConformanceRequestFactory {
    fn conflicting_start_signed_common_request(
        &self,
    ) -> Result<String, OperationConformanceFrontierError> {
        let start = self.admitted_start_request()?;
        canon::to_canonical_string(&evidence::request_test_support::conflicting_start_request(
            &start,
        ))
        .map_err(|_| OperationConformanceFrontierError::Construction)
    }
}

use super::restart_test_support::{serve_operation_frame, CONSULT_INITIAL_FRONTIER_ATTEMPTS};

fn start_consult_frame(signed_common_request: &str) -> Box<[u8]> {
    canon::to_canonical_string(&ResidentOperationFrameRequest::StartConsult {
        signed_common_request: signed_common_request.to_owned(),
    })
    .expect("Consult Start frame canonicalizes")
    .into_bytes()
    .into_boxed_slice()
}

fn start_invoke_frame(signed_common_request: &str) -> Box<[u8]> {
    canon::to_canonical_string(&ResidentOperationFrameRequest::StartInvoke {
        signed_common_request: signed_common_request.to_owned(),
    })
    .expect("Invoke Start frame canonicalizes")
    .into_bytes()
    .into_boxed_slice()
}

#[test]
fn invoke_exact_start_replay_returns_the_original_durable_records() {
    let returned_locator = with_in_memory_invoke_conformance_frontier(
        |service, requests| {
            let frame = start_invoke_frame(requests.start_signed_common_request());
            let records = match serve_operation_frame(service, &frame) {
                ResidentOperationFrameResponse::Started { records } => records,
                other => panic!("the first Invoke Start must reach custody; got {other:?}"),
            };
            let replayed_records = match serve_operation_frame(service, &frame) {
                ResidentOperationFrameResponse::Started { records } => records,
                other => panic!(
                    "an exact Invoke Start replay must return the durable records; got {other:?}"
                ),
            };
            assert_eq!(
                replayed_records, records,
                "exact Invoke Start replay cannot repeat policy, allocation, or signing",
            );
            let handle: serde_json::Value =
                canon::parse_strict(records.operation_handle_canonical().as_bytes())
                    .expect("the operation handle is canonical");
            let locator = ResidentOperationFrameLocator::new(
                records.operation_handle_record_ref().to_owned(),
                handle["canonical_operation_handle_sha256"]
                    .as_str()
                    .expect("the operation handle carries its digest")
                    .to_owned(),
            );
            (locator.clone(), locator)
        },
        |_, _, expected_locator, returned_locator| {
            assert_eq!(&returned_locator, expected_locator);
            returned_locator
        },
    )
    .expect("the admitted Invoke frontier returns an exact replay");

    assert!(!returned_locator.operation_handle_record_ref().is_empty());
    assert!(!returned_locator.operation_handle_sha256().is_empty());
}

#[test]
fn staged_consult_start_exact_resubmission_returns_the_original_durable_start() {
    let returned_locator = with_in_memory_consult_conformance_frontier(
        |service, requests| {
            let exact_frame = start_consult_frame(requests.start_signed_common_request());
            assert_eq!(
                serve_operation_frame(service, &exact_frame),
                ResidentOperationFrameResponse::Rejected {
                    failure: ResidentOperationFrameFailure::ResidentUnavailable,
                },
                "the first Start hands retained physical work to the resident scheduler",
            );

            let conflicting_request = requests
                .conflicting_start_signed_common_request()
                .expect("the conflicting Start request canonicalizes independently");
            let conflicting_frame = start_consult_frame(&conflicting_request);
            assert_ne!(conflicting_frame, exact_frame);
            assert_eq!(
                serve_operation_frame(service, &conflicting_frame),
                ResidentOperationFrameResponse::Rejected {
                    failure: ResidentOperationFrameFailure::ReplayConflict,
                },
                "changed bytes under the same request identity fail even while the original Start remains staged",
            );

            let mut scheduler = crate::resident::ResidentWorkTurnCoordinator::new();
            let mut started = None;
            for _ in 0..CONSULT_INITIAL_FRONTIER_ATTEMPTS {
                match scheduler
                    .run_one(service, crate::resident::ResidentWorkTurnInput::idle())
                    .expect("one bounded initial Consult frontier attempt completes")
                {
                    crate::resident::ResidentWorkTurnOutcome::RetainedPhaseAdvanced
                    | crate::resident::ResidentWorkTurnOutcome::RetainedPhaseDeferred => {}
                    other => panic!(
                        "the initial Consult frontier cannot lose its retained owner; got {other:?}"
                    ),
                }
                match serve_operation_frame(service, &exact_frame) {
                    ResidentOperationFrameResponse::Started { records } => {
                        started = Some(records);
                        break;
                    }
                    ResidentOperationFrameResponse::Rejected {
                        failure: ResidentOperationFrameFailure::ResidentUnavailable,
                    } => {}
                    other => panic!(
                        "an exact Start retry must observe pending or durable work; got {other:?}"
                    ),
                }
            }
            let records = started.expect(
                "bounded scheduling must make the original durable Consult Start observable",
            );
            assert_eq!(
                serve_operation_frame(service, &conflicting_frame),
                ResidentOperationFrameResponse::Rejected {
                    failure: ResidentOperationFrameFailure::ReplayConflict,
                },
                "changed bytes under the same request identity are rejected without consuming the durable result",
            );
            let replayed_records = match serve_operation_frame(service, &exact_frame) {
                ResidentOperationFrameResponse::Started { records } => records,
                other => panic!(
                    "a later exact Start must project the existing durable records; got {other:?}"
                ),
            };
            assert_eq!(
                replayed_records, records,
                "exact Start replay cannot allocate, sign, or retain a second operation",
            );
            assert!(
                service.has_retained_phase(),
                "Continue(None) retains the live keyed Consult continuation after Start",
            );
            let handle: serde_json::Value =
                canon::parse_strict(records.operation_handle_canonical().as_bytes())
                    .expect("the operation handle is canonical");
            let locator = ResidentOperationFrameLocator::new(
                records.operation_handle_record_ref().to_owned(),
                handle["canonical_operation_handle_sha256"]
                    .as_str()
                    .expect("the operation handle carries its digest")
                    .to_owned(),
            );
            (locator.clone(), locator)
        },
        |_, _, expected_locator, activity, returned_locator| {
            assert_eq!(&returned_locator, expected_locator);
            assert_eq!(activity.start_policy_calls(), 1);
            assert_eq!(activity.initial_collection_calls(), 1);
            assert_eq!(activity.finite_turn_calls(), 2);
            assert_eq!(activity.completion_policy_calls(), 0);
            assert_eq!(activity.executor_polls(), 0);
            assert_eq!(activity.child_stop_join_or_acceptance_attempts(), 0);
            assert_eq!(activity.identity_entropy_fills(), 7);
            assert_eq!(activity.handle_signatures(), 1);
            returned_locator
        },
    )
    .expect("the admitted Consult frontier completes one staged Start");

    assert!(!returned_locator.operation_handle_record_ref().is_empty());
    assert!(!returned_locator.operation_handle_sha256().is_empty());
}
