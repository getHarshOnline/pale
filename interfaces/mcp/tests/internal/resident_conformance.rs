//! Resident attachment boundary conformance for MCP.

use super::*;
use pale_cli::command::{
    AuthenticatedResidentCarrier, ResidentResponseAdmission, ResidentResponseAdmissionFailure,
};
use pale_interface_test_support::{DeterministicResponseAdmission, InMemoryResidentCarrier};
use std::time::{Duration, Instant};

const APPLICATION_START_FIXTURE: &str = include_str!(
    "../../../../crates/pale-application/tests/fixtures/resident/application-invoke-sdk.v1.json"
);

struct FixtureRecords {
    handle_ref: String,
    handle: Vec<u8>,
    observation_ref: String,
    observation: Vec<u8>,
}

fn fixture_records() -> FixtureRecords {
    let fixture: serde_json::Value =
        serde_json::from_str(APPLICATION_START_FIXTURE).expect("application fixture parses");
    let records = &fixture["response_frame"]["records"];
    FixtureRecords {
        handle_ref: records["operationHandleRecordRef"]
            .as_str()
            .expect("fixture has a handle reference")
            .to_owned(),
        handle: records["operationHandleCanonical"]
            .as_str()
            .expect("fixture has a canonical handle")
            .as_bytes()
            .to_vec(),
        observation_ref: records["operationObservationRecordRef"]
            .as_str()
            .expect("fixture has an observation reference")
            .to_owned(),
        observation: records["operationObservationCanonical"]
            .as_str()
            .expect("fixture has a canonical observation")
            .as_bytes()
            .to_vec(),
    }
}

fn expect_fixture(
    authority: &DeterministicResponseAdmission,
    records: &FixtureRecords,
) -> Result<(), ResidentResponseAdmissionFailure> {
    authority.expect(
        &records.handle_ref,
        &records.handle,
        &records.observation_ref,
        &records.observation,
    )
}

#[test]
fn response_admission_requires_exact_authority_registered_bytes_and_references() {
    let records = fixture_records();
    let authority = DeterministicResponseAdmission::default();
    expect_fixture(&authority, &records).expect("application response is valid test evidence");

    let substituted_observation_ref = "operation-observation-record:v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    assert_eq!(
        authority.admit(
            &records.handle_ref,
            &records.handle,
            substituted_observation_ref,
            &records.observation,
            Instant::now() + Duration::from_secs(1),
        ),
        Err(ResidentResponseAdmissionFailure),
        "a signed observation cannot be relabelled under another record reference",
    );
    assert_eq!(authority.calls(), 0);

    authority
        .admit(
            &records.handle_ref,
            &records.handle,
            &records.observation_ref,
            &records.observation,
            Instant::now() + Duration::from_secs(1),
        )
        .expect("the exact registered response admits");
    assert_eq!(authority.calls(), 1);
    assert_eq!(
        authority
            .last()
            .expect("admission retains its frontier")
            .revision,
        1
    );
}

#[test]
fn response_admission_rejects_tampered_canonical_records_before_state_changes() {
    let records = fixture_records();
    let authority = DeterministicResponseAdmission::default();
    expect_fixture(&authority, &records).expect("application response is valid test evidence");
    let mut tampered = records.observation.clone();
    let position = tampered
        .windows(b"\"revision\":1".len())
        .position(|window| window == b"\"revision\":1")
        .expect("fixture carries revision one");
    let last = position + b"\"revision\":".len();
    tampered[last] = b'2';

    assert_eq!(
        authority.admit(
            &records.handle_ref,
            &records.handle,
            &records.observation_ref,
            &tampered,
            Instant::now() + Duration::from_secs(1),
        ),
        Err(ResidentResponseAdmissionFailure),
    );
    assert_eq!(authority.calls(), 0);
    authority
        .admit(
            &records.handle_ref,
            &records.handle,
            &records.observation_ref,
            &records.observation,
            Instant::now() + Duration::from_secs(1),
        )
        .expect("failed admission does not consume the registered response");
}

#[test]
fn expired_response_admission_does_not_move_the_authority_frontier() {
    let records = fixture_records();
    let authority = DeterministicResponseAdmission::default();
    expect_fixture(&authority, &records).expect("application response is valid test evidence");

    assert_eq!(
        authority.admit(
            &records.handle_ref,
            &records.handle,
            &records.observation_ref,
            &records.observation,
            Instant::now(),
        ),
        Err(ResidentResponseAdmissionFailure),
    );
    assert_eq!(authority.calls(), 0);
    authority
        .admit(
            &records.handle_ref,
            &records.handle,
            &records.observation_ref,
            &records.observation,
            Instant::now() + Duration::from_secs(1),
        )
        .expect("expired admission does not consume or advance the frontier");
}

#[test]
fn channel_carrier_returns_only_the_single_service_response() {
    let (mut carrier, requests, calls) = InMemoryResidentCarrier::bounded(1);
    let worker = std::thread::spawn(move || {
        let exchange = requests
            .recv_timeout(Duration::from_secs(1))
            .expect("one SDK request reaches the service channel");
        assert_eq!(exchange.request(), br#"{"kind":"probe"}"#,);
        exchange.complete(Ok(br#"{"kind":"response"}"#.to_vec().into_boxed_slice()));
    });

    let response = carrier
        .exchange(
            br#"{"kind":"probe"}"#,
            Instant::now() + Duration::from_secs(1),
        )
        .expect("service returns one response");
    worker.join().expect("service worker completes");
    assert_eq!(&*response, br#"{"kind":"response"}"#);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn expired_channel_request_is_not_sent() {
    let (mut carrier, requests, calls) = InMemoryResidentCarrier::bounded(1);
    assert_eq!(
        carrier.exchange(br#"{"kind":"late"}"#, Instant::now()),
        Err(pale_cli::command::AuthenticatedResidentCarrierFailure::NotSent),
    );
    assert!(requests.try_recv().is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "interface-test-support")]
mod same_handle_operation_conformance {
    use std::sync::{atomic::AtomicUsize, Arc};
    use std::time::Duration;

    use pale_application::resident::{
        with_in_memory_consult_conformance_frontier, with_in_memory_invoke_conformance_frontier,
        ResidentWorkTurnCoordinator, ResidentWorkTurnInput, ResidentWorkTurnOutcome,
    };
    use pale_cli::command::ResidentAttachment;
    use pale_interface_test_support::{
        serve_exchange_with_failed_response, serve_exchange_with_response,
        DeterministicResponseAdmission, InMemoryResidentCarrier, PendingResidentExchange,
    };
    use pale_ports::{
        LocalIpcPeerCredentials, ResidentObserveFrameFailure, ResidentObserveFrameRequest,
        ResidentObserveFrameResponse, ResidentOperationFrameLocator,
        ResidentOperationFrameResponse,
    };

    use super::*;
    use crate::conformance::McpConformanceClient;

    const EXPECTED_MCP_HANDOFFS: usize = 5;
    const EXPECTED_PREADVANCED_CONSULT_FINITE_TURNS: usize = 4;
    const EXPECTED_START_ONLY_CONSULT_FINITE_TURNS: usize = 2;
    const EXPECTED_CONSULT_CLOCK_READS: usize = 9;

    #[derive(Debug)]
    struct ProjectedJourney {
        start_handle_ref: String,
        start_handle_sha256: String,
        advance_handle_ref: String,
        advance_handle_sha256: String,
        observe_handle_ref: String,
        observe_handle_sha256: String,
    }

    #[derive(Debug)]
    struct ProjectedConsultJourney {
        operation: ProjectedJourney,
        start_observation_ref: String,
        start_observation_sha256: String,
        start_observation_revision: u64,
        successor_observation_ref: String,
        successor_observation_revision: u64,
        predecessor_observation_ref: String,
        predecessor_observation_sha256: String,
    }

    struct RetainedMcpJourney<Projection = ProjectedJourney> {
        thread: std::thread::JoinHandle<(Projection, ResidentOperationFrameLocator)>,
        handoffs: Arc<AtomicUsize>,
        admission: Arc<DeterministicResponseAdmission>,
    }

    struct PendingColdObserve {
        exchange: PendingResidentExchange,
    }

    struct CompletedColdObserve<Projection = ProjectedJourney> {
        retained: RetainedMcpJourney<Projection>,
        expected_locator: ResidentOperationFrameLocator,
        response_failure: Option<ResidentObserveFrameFailure>,
    }

    #[test]
    fn start_advance_and_cold_observe_preserve_one_handle_through_mcp_projection() {
        let client = std::cell::RefCell::new(None);
        let completed = with_in_memory_invoke_conformance_frontier(
            |service, journey| {
                let start_request = journey.start_signed_common_request().to_owned();
                let journey = journey.clone();
                let (carrier, exchanges, handoffs) = InMemoryResidentCarrier::bounded(1);
                let admission = Arc::new(DeterministicResponseAdmission::default());
                let attachment = ResidentAttachment::provision_operations(
                    Box::new(carrier),
                    Duration::from_secs(2),
                    admission.clone(),
                )
                .expect("positive timeout provisions one operation attachment");
                let client_thread = std::thread::spawn(move || {
                    let client = McpConformanceClient::new(attachment)
                        .expect("MCP projects the attached resident tools");
                    let start = client
                        .call_tool(
                            "start_resident_invoke",
                            object(serde_json::json!({
                                "signedCommonRequest": start_request,
                            })),
                        )
                        .expect("Start crosses MCP, the dispatcher, the SDK, and the application");
                    assert_eq!(start.is_error, Some(false));
                    let start = start
                        .structured_content
                        .expect("Start returns structured signed records");
                    let start_handle_ref = string(&start, "operationHandleRecordRef");
                    let start_handle =
                        string(&start, "operationHandleCanonicalJson").into_bytes();
                    let start_handle_sha256 =
                        canonical_digest(&start_handle, "canonical_operation_handle_sha256");
                    let locator = pale_ports::ResidentOperationFrameLocator::new(
                        start_handle_ref.clone(),
                        start_handle_sha256.clone(),
                    );
                    let wrong_locator = pale_ports::ResidentOperationFrameLocator::new(
                        start_handle_ref.clone(),
                        "0".repeat(64),
                    );
                    let wrong_request = journey
                        .advance_signed_common_request(&wrong_locator)
                        .expect("a well-formed wrong locator can be signed for the negative case");
                    let wrong = client
                        .call_tool(
                            "advance_resident_invoke",
                            object(serde_json::json!({
                                "operationHandleRecordRef": wrong_locator.operation_handle_record_ref(),
                                "operationHandleSha256": wrong_locator.operation_handle_sha256(),
                                "signedCommonRequest": wrong_request,
                            })),
                        )
                        .expect("the wrong-handle attempt returns one bounded MCP result");
                    assert_mcp_diagnostic(
                        &wrong,
                        "resident_operation_outcome_unknown",
                        "resident_recovery",
                        "effect_outcome_unknown",
                        "reconcile_first",
                        "outcome_unknown",
                    );

                    let expired_request = journey
                        .expired_advance_signed_common_request(&locator)
                        .expect("an expired request remains structurally valid and signed");
                    let expired = client
                        .call_tool(
                            "advance_resident_invoke",
                            object(serde_json::json!({
                                "operationHandleRecordRef": &start_handle_ref,
                                "operationHandleSha256": &start_handle_sha256,
                                "signedCommonRequest": expired_request,
                            })),
                        )
                        .expect("the expired request returns one bounded MCP result");
                    assert_mcp_diagnostic(
                        &expired,
                        "resident_operation_admission_denied",
                        "resident_admission",
                        "blocked",
                        "after_state_change",
                        "not_started",
                    );

                    let advance_request = journey
                        .advance_signed_common_request(&locator)
                        .expect("Advance binds the application-issued locator");

                    let advance = client
                        .call_tool(
                            "advance_resident_invoke",
                            object(serde_json::json!({
                                "operationHandleRecordRef": &start_handle_ref,
                                "operationHandleSha256": &start_handle_sha256,
                                "signedCommonRequest": advance_request,
                            })),
                        )
                        .expect("Advance reaches the same application operation");
                    assert_eq!(advance.is_error, Some(false));
                    let advance = advance
                        .structured_content
                        .expect("Advance returns its bounded disposition");
                    assert_eq!(advance["kind"], "pending");
                    let advance_handle_ref =
                        string(&advance["locator"], "operationHandleRecordRef");
                    let advance_handle_sha256 =
                        string(&advance["locator"], "operationHandleSha256");

                    let observe_request = journey
                        .observe_signed_common_request(&locator, &start_handle)
                        .expect("Observe uses the admitted read-only request domain");
                    let observe = client
                        .call_tool(
                            "observe_resident_operation",
                            object(serde_json::json!({
                                "operationHandleRecordRef": &advance_handle_ref,
                                "operationHandleSha256": &advance_handle_sha256,
                                "signedCommonRequest": observe_request,
                            })),
                        )
                        .expect("Observe reads the same application operation");
                    assert_eq!(
                        observe.is_error,
                        Some(false),
                        "Observe returned an MCP diagnostic: {:?}",
                        observe.structured_content,
                    );
                    let observe = observe
                        .structured_content
                        .expect("Observe returns structured signed records");
                    let observe_handle_ref = string(&observe, "operationHandleRecordRef");
                    let observe_handle =
                        string(&observe, "operationHandleCanonicalJson").into_bytes();
                    let observe_handle_sha256 =
                        canonical_digest(&observe_handle, "canonical_operation_handle_sha256");

                    (
                        ProjectedJourney {
                            start_handle_ref,
                            start_handle_sha256,
                            advance_handle_ref,
                            advance_handle_sha256,
                            observe_handle_ref,
                            observe_handle_sha256,
                        },
                        locator,
                    )
                });

                let mut issued_locator = None;
                let mut first_service_turns = 0;
                let mut pending_observe = None;
                for _ in 0..EXPECTED_MCP_HANDOFFS {
                    let exchange = exchanges
                        .recv_timeout(Duration::from_secs(2))
                        .expect("one bounded MCP request reaches the application service");
                    if serde_json::from_slice::<ResidentObserveFrameRequest>(exchange.request())
                        .is_ok()
                    {
                        pending_observe = Some(exchange);
                        break;
                    }
                    let turn = first_service_turns;
                    let response_admission = Arc::clone(&admission);
                    serve_exchange_with_response(
                        service,
                        exchange,
                        LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
                        |response| {
                            let response = response.expect("resident establishes one response");
                            if turn == 0 {
                                let start_response: ResidentOperationFrameResponse =
                                    serde_json::from_slice(response)
                                        .expect("Start response decodes");
                                let records = match start_response {
                                    ResidentOperationFrameResponse::Started { records } => records,
                                    other => panic!(
                                        "the first application turn must start the operation; got {other:?}"
                                    ),
                                };
                                let handle: serde_json::Value = pale_core::canon::parse_strict(
                                    records.operation_handle_canonical().as_bytes(),
                                )
                                .expect("application handle is canonical");
                                issued_locator =
                                    Some(pale_ports::ResidentOperationFrameLocator::new(
                                        records.operation_handle_record_ref().to_owned(),
                                        handle["canonical_operation_handle_sha256"]
                                            .as_str()
                                            .expect("handle carries its digest")
                                            .to_owned(),
                                    ));
                            }
                            admit_operation_records_when_present(
                                &response_admission,
                                response,
                            );
                        },
                    );
                    first_service_turns += 1;
                }

                let pending_observe = pending_observe
                    .expect("Observe reaches the bounded carrier within five MCP requests");
                assert_eq!(first_service_turns, EXPECTED_MCP_HANDOFFS - 1);
                assert_eq!(handoffs.load(Ordering::SeqCst), EXPECTED_MCP_HANDOFFS);
                *client.borrow_mut() = Some(RetainedMcpJourney {
                    thread: client_thread,
                    handoffs,
                    admission,
                });
                (
                    issued_locator.expect("Start returns one application-issued locator"),
                    PendingColdObserve {
                        exchange: pending_observe,
                    },
                )
            },
            |service, _, expected_locator, pending_observe| {
                let PendingColdObserve { exchange } = pending_observe;
                let admission = Arc::clone(
                    &client
                        .borrow()
                        .as_ref()
                        .expect("the start service retained response admission")
                        .admission,
                );
                let mut response_failure = None;
                serve_exchange_with_response(
                    service,
                    exchange,
                    LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
                    |response| {
                        let response = response.expect("Observe establishes one response");
                        response_failure = admit_observe_records(&admission, response);
                    },
                );
                let retained = client
                    .borrow_mut()
                    .take()
                    .expect("one client journey is retained");
                CompletedColdObserve {
                    retained,
                    expected_locator: expected_locator.clone(),
                    response_failure,
                }
            },
        )
        .expect("application conformance evidence constructs one exact journey");

        if let Some(failure) = completed.response_failure {
            panic!("application rejected the Observe request: {failure:?}");
        }
        let (projected, client_locator) = completed
            .retained
            .thread
            .join()
            .expect("MCP journey thread completes");
        assert_eq!(client_locator, completed.expected_locator);
        for handle_ref in [
            &projected.start_handle_ref,
            &projected.advance_handle_ref,
            &projected.observe_handle_ref,
        ] {
            assert_eq!(
                handle_ref,
                completed.expected_locator.operation_handle_record_ref()
            );
        }
        for handle_sha256 in [
            &projected.start_handle_sha256,
            &projected.advance_handle_sha256,
            &projected.observe_handle_sha256,
        ] {
            assert_eq!(
                handle_sha256,
                completed.expected_locator.operation_handle_sha256()
            );
        }
        assert_eq!(
            completed.retained.handoffs.load(Ordering::SeqCst),
            EXPECTED_MCP_HANDOFFS
        );
        assert_eq!(completed.retained.admission.calls(), 2);
        assert_eq!(
            projected.start_handle_ref, projected.observe_handle_ref,
            "the final result remains the Start-issued operation"
        );
    }

    #[test]
    fn consult_start_pending_advance_and_cold_observe_preserve_one_handle() {
        const EXPECTED_CONSULT_HANDOFFS: usize = 5;
        const EXPECTED_RETAINED_PHASE_TURNS: usize = 9;

        let client = std::cell::RefCell::new(None);
        let completed = with_in_memory_consult_conformance_frontier(
            |service, journey| {
                let start_request = journey.start_signed_common_request().to_owned();
                let journey = journey.clone();
                let (carrier, exchanges, handoffs) = InMemoryResidentCarrier::bounded(1);
                let admission = Arc::new(DeterministicResponseAdmission::default());
                let attachment = ResidentAttachment::provision_operations(
                    Box::new(carrier),
                    Duration::from_secs(2),
                    admission.clone(),
                )
                .expect("positive timeout provisions one operation attachment");
                let client_thread = std::thread::spawn(move || {
                    let client = McpConformanceClient::new(attachment)
                        .expect("MCP projects the attached resident tools");
                    let unavailable = client
                        .call_tool(
                            "start_resident_consult",
                            object(serde_json::json!({
                                "signedCommonRequest": start_request.clone(),
                            })),
                        )
                        .expect("the first Consult Start reaches the application");
                    assert_mcp_diagnostic(
                        &unavailable,
                        "resident_operation_outcome_unknown",
                        "resident_recovery",
                        "effect_outcome_unknown",
                        "reconcile_first",
                        "outcome_unknown",
                    );
                    let help = &unavailable
                        .structured_content
                        .as_ref()
                        .expect("the unavailable Start carries a diagnostic")["diagnostic"]
                        ["help"];
                    assert!(help.as_array().is_some_and(|entries| entries.iter().any(|entry| {
                        entry.as_str().is_some_and(|entry| {
                            entry.contains("resubmit the exact signed Start request bytes")
                                && entry.contains("do not mint a new request identity")
                        })
                    })));

                    let uncertain_start = client
                        .call_tool(
                            "start_resident_consult",
                            object(serde_json::json!({
                                "signedCommonRequest": start_request.clone(),
                            })),
                        )
                        .expect("the first exact Start resubmission reaches the application");
                    assert_mcp_diagnostic(
                        &uncertain_start,
                        "resident_operation_failed",
                        "host_observation",
                        "effect_outcome_unknown",
                        "reconcile_first",
                        "outcome_unknown",
                    );
                    let help = &uncertain_start
                        .structured_content
                        .as_ref()
                        .expect("the lost Start response carries a diagnostic")["diagnostic"]
                        ["help"];
                    assert!(help.as_array().is_some_and(|entries| entries.iter().any(|entry| {
                        entry.as_str().is_some_and(|entry| {
                            entry.contains("resubmit the exact signed Start request bytes")
                                && entry.contains("do not mint a new request identity")
                        })
                    })));

                    let start = client
                        .call_tool(
                            "start_resident_consult",
                            object(serde_json::json!({
                                "signedCommonRequest": start_request,
                            })),
                        )
                        .expect("the exact Consult Start resumes after resident work turns");
                    assert_eq!(start.is_error, Some(false));
                    let start = start
                        .structured_content
                        .expect("Consult Start returns structured signed records");
                    let start_handle_ref = string(&start, "operationHandleRecordRef");
                    let start_handle =
                        string(&start, "operationHandleCanonicalJson").into_bytes();
                    let start_handle_sha256 =
                        canonical_digest(&start_handle, "canonical_operation_handle_sha256");
                    let start_observation_ref =
                        string(&start, "operationObservationRecordRef");
                    let start_observation: serde_json::Value = pale_core::canon::parse_strict(
                        string(&start, "operationObservationCanonicalJson").as_bytes(),
                    )
                    .expect("Consult Start observation is canonical");
                    let start_observation_sha256 = string(
                        &start_observation,
                        "canonical_operation_observation_sha256",
                    );
                    let start_observation_revision = start_observation["revision"]
                        .as_u64()
                        .expect("Consult Start observation has a revision");
                    let locator = ResidentOperationFrameLocator::new(
                        start_handle_ref.clone(),
                        start_handle_sha256.clone(),
                    );

                    let advance_request = journey
                        .advance_signed_common_request(&locator)
                        .expect("Consult Advance binds the application-issued locator");
                    let advance = client
                        .call_tool(
                            "advance_resident_consult",
                            object(serde_json::json!({
                                "operationHandleRecordRef": &start_handle_ref,
                                "operationHandleSha256": &start_handle_sha256,
                                "signedCommonRequest": advance_request,
                            })),
                        )
                        .expect("Consult Advance reaches the same application operation");
                    assert_eq!(advance.is_error, Some(false));
                    let advance = advance
                        .structured_content
                        .expect("Consult Advance returns its bounded disposition");
                    assert_eq!(advance["kind"], "pending");
                    let advance_handle_ref =
                        string(&advance["locator"], "operationHandleRecordRef");
                    let advance_handle_sha256 =
                        string(&advance["locator"], "operationHandleSha256");

                    let observe_request = journey
                        .observe_signed_common_request(&locator, &start_handle)
                        .expect("Observe uses the admitted read-only request domain");
                    let observe = client
                        .call_tool(
                            "observe_resident_operation",
                            object(serde_json::json!({
                                "operationHandleRecordRef": &advance_handle_ref,
                                "operationHandleSha256": &advance_handle_sha256,
                                "signedCommonRequest": observe_request,
                            })),
                        )
                        .expect("Observe reads the same Consult operation");
                    assert_eq!(
                        observe.is_error,
                        Some(false),
                        "Observe returned an MCP diagnostic: {:?}",
                        observe.structured_content,
                    );
                    let observe = observe
                        .structured_content
                        .expect("Observe returns structured signed records");
                    let observe_handle_ref = string(&observe, "operationHandleRecordRef");
                    let observe_handle =
                        string(&observe, "operationHandleCanonicalJson").into_bytes();
                    let observe_handle_sha256 =
                        canonical_digest(&observe_handle, "canonical_operation_handle_sha256");
                    let successor_observation_ref =
                        string(&observe, "operationObservationRecordRef");
                    let successor_observation: serde_json::Value = pale_core::canon::parse_strict(
                        string(&observe, "operationObservationCanonicalJson").as_bytes(),
                    )
                    .expect("cold Observe returns one canonical successor observation");
                    let successor_observation_revision = successor_observation["revision"]
                        .as_u64()
                        .expect("Consult successor observation has a revision");
                    let predecessor_observation_ref =
                        string(&successor_observation, "predecessor_ref");
                    let predecessor_observation_sha256 =
                        string(&successor_observation, "predecessor_sha256");

                    (
                        ProjectedConsultJourney {
                            operation: ProjectedJourney {
                                start_handle_ref,
                                start_handle_sha256,
                                advance_handle_ref,
                                advance_handle_sha256,
                                observe_handle_ref,
                                observe_handle_sha256,
                            },
                            start_observation_ref,
                            start_observation_sha256,
                            start_observation_revision,
                            successor_observation_ref,
                            successor_observation_revision,
                            predecessor_observation_ref,
                            predecessor_observation_sha256,
                        },
                        locator,
                    )
                });

                let first_start = exchanges
                    .recv_timeout(Duration::from_secs(2))
                    .expect("the first Consult Start reaches the application service");
                serve_exchange_with_response(
                    service,
                    first_start,
                    LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
                    |response| {
                        let response: ResidentOperationFrameResponse = serde_json::from_slice(
                            response.expect("the first Consult Start establishes one response"),
                        )
                        .expect("the first Consult Start response decodes");
                        assert_eq!(
                            response,
                            ResidentOperationFrameResponse::Rejected {
                                failure: pale_ports::ResidentOperationFrameFailure::ResidentUnavailable,
                            },
                        );
                    },
                );

                let mut scheduler = ResidentWorkTurnCoordinator::new();
                for phase in 0..EXPECTED_RETAINED_PHASE_TURNS {
                    assert_eq!(
                        scheduler
                            .run_one(service, ResidentWorkTurnInput::idle())
                            .expect("one application-owned Consult phase advances"),
                        ResidentWorkTurnOutcome::RetainedPhaseAdvanced,
                        "Consult retained phase {phase} advances once",
                    );
                }

                let uncertain_start = exchanges
                    .recv_timeout(Duration::from_secs(2))
                    .expect("the first exact Start resubmission reaches the application service");
                serve_exchange_with_failed_response(
                    service,
                    uncertain_start,
                    LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
                );

                let exact_start = exchanges
                    .recv_timeout(Duration::from_secs(2))
                    .expect("the exact Start resubmission reaches the application service");
                let mut issued_locator = None;
                let response_admission = Arc::clone(&admission);
                serve_exchange_with_response(
                    service,
                    exact_start,
                    LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
                    |response| {
                        let response = response.expect("the exact Start establishes one response");
                        let start_response: ResidentOperationFrameResponse =
                            serde_json::from_slice(response)
                                .expect("the exact Consult Start response decodes");
                        let records = match start_response {
                            ResidentOperationFrameResponse::Started { records } => records,
                            other => panic!(
                                "the exact Start resubmission must recover the original durable records; got {other:?}"
                            ),
                        };
                        let handle: serde_json::Value = pale_core::canon::parse_strict(
                            records.operation_handle_canonical().as_bytes(),
                        )
                        .expect("application handle is canonical");
                        issued_locator = Some(ResidentOperationFrameLocator::new(
                            records.operation_handle_record_ref().to_owned(),
                            handle["canonical_operation_handle_sha256"]
                                .as_str()
                                .expect("handle carries its digest")
                                .to_owned(),
                        ));
                        admit_operation_records_when_present(&response_admission, response);
                    },
                );

                let advance = exchanges
                    .recv_timeout(Duration::from_secs(2))
                    .expect("Consult Advance reaches the application service");
                let response_admission = Arc::clone(&admission);
                serve_exchange_with_response(
                    service,
                    advance,
                    LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
                    |response| {
                        admit_operation_records_when_present(
                            &response_admission,
                            response.expect("Consult Advance establishes one response"),
                        );
                    },
                );

                let pending_observe = PendingColdObserve {
                    exchange: exchanges
                        .recv_timeout(Duration::from_secs(2))
                        .expect("Observe reaches the carrier after Start and Advance"),
                };
                assert_eq!(handoffs.load(Ordering::SeqCst), EXPECTED_CONSULT_HANDOFFS);
                *client.borrow_mut() = Some(RetainedMcpJourney {
                    thread: client_thread,
                    handoffs,
                    admission,
                });
                (
                    issued_locator.expect("Consult Start returns one application-issued locator"),
                    pending_observe,
                )
            },
            |service, _, expected_locator, activity, pending_observe| {
                let PendingColdObserve { exchange } = pending_observe;
                let admission = Arc::clone(
                    &client
                        .borrow()
                        .as_ref()
                        .expect("the Consult service retained response admission")
                        .admission,
                );
                let mut response_failure = None;
                serve_exchange_with_response(
                    service,
                    exchange,
                    LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
                    |response| {
                        let response = response.expect("Observe establishes one response");
                        response_failure = admit_observe_records(&admission, response);
                    },
                );
                assert_eq!(activity.start_policy_calls(), 1);
                assert_eq!(activity.initial_collection_calls(), 1);
                assert_eq!(
                    activity.finite_turn_calls(),
                    EXPECTED_PREADVANCED_CONSULT_FINITE_TURNS,
                    "the fixed scheduler window advances both retained Consult phases",
                );
                assert_eq!(activity.completion_policy_calls(), 1);
                assert_eq!(activity.executor_polls(), 1);
                assert_eq!(activity.child_stop_join_or_acceptance_attempts(), 0);
                assert_eq!(
                    activity.clock_reads(),
                    EXPECTED_CONSULT_CLOCK_READS,
                    "every submitted frame receives a fresh time observation",
                );
                assert_eq!(activity.identity_entropy_fills(), 7);
                assert_eq!(activity.handle_signatures(), 1);
                let retained = client
                    .borrow_mut()
                    .take()
                    .expect("one Consult client journey is retained");
                CompletedColdObserve {
                    retained,
                    expected_locator: expected_locator.clone(),
                    response_failure,
                }
            },
        )
        .expect("application conformance evidence constructs one Consult journey");

        if let Some(failure) = completed.response_failure {
            panic!("application rejected the Consult Observe request: {failure:?}");
        }
        let (projected, client_locator) = completed
            .retained
            .thread
            .join()
            .expect("Consult MCP journey thread completes");
        assert_eq!(client_locator, completed.expected_locator);
        for handle_ref in [
            &projected.operation.start_handle_ref,
            &projected.operation.advance_handle_ref,
            &projected.operation.observe_handle_ref,
        ] {
            assert_eq!(
                handle_ref,
                completed.expected_locator.operation_handle_record_ref()
            );
        }
        for handle_sha256 in [
            &projected.operation.start_handle_sha256,
            &projected.operation.advance_handle_sha256,
            &projected.operation.observe_handle_sha256,
        ] {
            assert_eq!(
                handle_sha256,
                completed.expected_locator.operation_handle_sha256()
            );
        }
        assert_ne!(
            projected.successor_observation_ref, projected.start_observation_ref,
            "Advance must retain a successor rather than replay the Start observation"
        );
        assert_eq!(
            projected.successor_observation_revision,
            projected.start_observation_revision + 1,
            "the observed Advance successor is the next monotone revision"
        );
        assert_eq!(
            projected.predecessor_observation_ref, projected.start_observation_ref,
            "the Advance successor links to the Start observation reference"
        );
        assert_eq!(
            projected.predecessor_observation_sha256, projected.start_observation_sha256,
            "the Advance successor links to the Start observation digest"
        );
        assert_eq!(
            completed.retained.handoffs.load(Ordering::SeqCst),
            EXPECTED_CONSULT_HANDOFFS
        );
        assert_eq!(completed.retained.admission.calls(), 2);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn local_socket_consult_exact_retry_advance_and_cold_observe_preserve_one_handle() {
        use std::num::NonZeroU32;
        use std::path::PathBuf;
        use std::sync::atomic::Ordering;
        use std::time::Instant;

        use pale_adapter_uds::{UnixSocketCarrier, UnixSocketListener};
        use pale_cli::command::{
            AuthenticatedResidentCarrierFailure, AuthenticatedResidentCarrierSource,
            AuthenticatedResidentCarrierSourceFailure, ResidentResponseAdmission,
            ResidentResponseAdmissionFailure,
        };
        use pale_ports::{LocalIpcCarrierError, LocalIpcFrameLimits, LocalIpcFramePort};

        const MAXIMUM_START_ATTEMPTS: usize = 18;
        const FRAME_LIMIT_BYTES: u32 = 1 << 20;

        struct SocketDirectory {
            path: PathBuf,
        }

        impl SocketDirectory {
            fn create() -> Self {
                static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);
                let temporary_root = std::env::temp_dir();
                for _ in 0..1_024 {
                    let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
                    let path = temporary_root.join(format!(
                        "pale-mcp-consult-{}-{sequence}",
                        std::process::id()
                    ));
                    match std::fs::create_dir(&path) {
                        Ok(()) => return Self { path },
                        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                        Err(error) => panic!("create temporary socket directory: {error}"),
                    }
                }
                panic!("allocate a unique temporary socket directory")
            }

            fn endpoint(&self) -> PathBuf {
                self.path.join("resident.sock")
            }
        }

        impl Drop for SocketDirectory {
            fn drop(&mut self) {
                let endpoint = self.endpoint();
                let _ = std::fs::remove_file(endpoint);
                let _ = std::fs::remove_dir(&self.path);
            }
        }

        struct FixtureResponseAdmission {
            validated: DeterministicResponseAdmission,
        }

        impl FixtureResponseAdmission {
            fn new() -> Self {
                Self {
                    validated: DeterministicResponseAdmission::default(),
                }
            }

            fn calls(&self) -> usize {
                self.validated.calls()
            }
        }

        impl ResidentResponseAdmission for FixtureResponseAdmission {
            fn admit(
                &self,
                operation_handle_record_ref: &str,
                operation_handle_canonical_bytes: &[u8],
                operation_observation_record_ref: &str,
                operation_observation_canonical_bytes: &[u8],
                deadline: Instant,
            ) -> Result<(), ResidentResponseAdmissionFailure> {
                if Instant::now() >= deadline {
                    return Err(ResidentResponseAdmissionFailure);
                }
                // The application fixture uses a deterministic development key.
                // Validate its records and monotone frontier without claiming a
                // production authority or an independent expected-response log.
                self.validated.expect(
                    operation_handle_record_ref,
                    operation_handle_canonical_bytes,
                    operation_observation_record_ref,
                    operation_observation_canonical_bytes,
                )?;
                self.validated.admit(
                    operation_handle_record_ref,
                    operation_handle_canonical_bytes,
                    operation_observation_record_ref,
                    operation_observation_canonical_bytes,
                    deadline,
                )
            }
        }

        struct ConformanceSocketFrame {
            carrier: UnixSocketCarrier,
            last_response: Option<Box<[u8]>>,
        }

        impl ConformanceSocketFrame {
            fn new(carrier: UnixSocketCarrier) -> Self {
                Self {
                    carrier,
                    last_response: None,
                }
            }

            fn take_response(&mut self) -> Box<[u8]> {
                self.last_response
                    .take()
                    .expect("the resident sent one response")
            }
        }

        impl LocalIpcFramePort for ConformanceSocketFrame {
            fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
                // The reusable application frontier deliberately admits one
                // fixed test peer. The SDK connector independently observes and
                // admits the real kernel peer before it sends any Pale bytes.
                Ok(LocalIpcPeerCredentials::new(501, 20, Some(8_081)))
            }

            fn send_frame(
                &mut self,
                payload: &[u8],
                deadline: Instant,
            ) -> Result<(), LocalIpcCarrierError> {
                self.carrier.send_frame(payload, deadline)?;
                self.last_response = Some(payload.to_vec().into_boxed_slice());
                Ok(())
            }

            fn receive_frame(
                &mut self,
                deadline: Instant,
            ) -> Result<Box<[u8]>, LocalIpcCarrierError> {
                self.last_response = None;
                self.carrier.receive_frame(deadline)
            }
        }

        struct ConformanceClientCarrier {
            carrier: UnixSocketCarrier,
        }

        impl AuthenticatedResidentCarrier for ConformanceClientCarrier {
            fn exchange(
                &mut self,
                request: &[u8],
                deadline: Instant,
            ) -> Result<Box<[u8]>, AuthenticatedResidentCarrierFailure> {
                self.carrier
                    .send_frame(request, deadline)
                    .map_err(|_| AuthenticatedResidentCarrierFailure::NotSent)?;
                self.carrier
                    .receive_frame(deadline)
                    .map_err(|_| AuthenticatedResidentCarrierFailure::OutcomeUnknown)
            }
        }

        struct ConformanceClientCarrierSource {
            endpoint: PathBuf,
            limits: LocalIpcFrameLimits,
            peer_admissions: Arc<AtomicUsize>,
        }

        impl AuthenticatedResidentCarrierSource for ConformanceClientCarrierSource {
            fn open(
                &mut self,
                deadline: Instant,
            ) -> Result<
                Box<dyn AuthenticatedResidentCarrier>,
                AuthenticatedResidentCarrierSourceFailure,
            > {
                let carrier = UnixSocketCarrier::connect(&self.endpoint, self.limits, deadline)
                    .map_err(|_| AuthenticatedResidentCarrierSourceFailure)?;
                let peer = carrier
                    .peer_credentials()
                    .map_err(|_| AuthenticatedResidentCarrierSourceFailure)?;
                if peer.process_id() != Some(std::process::id()) {
                    return Err(AuthenticatedResidentCarrierSourceFailure);
                }
                self.peer_admissions.fetch_add(1, Ordering::SeqCst);
                Ok(Box::new(ConformanceClientCarrier { carrier }))
            }
        }

        struct SocketJourney {
            client: std::thread::JoinHandle<ProjectedJourney>,
            connection: ConformanceSocketFrame,
            response_admission: Arc<FixtureResponseAdmission>,
            peer_admissions: Arc<AtomicUsize>,
            socket_directory: SocketDirectory,
        }

        let completed = with_in_memory_consult_conformance_frontier(
            |service, journey| {
                let directory = SocketDirectory::create();
                let endpoint = directory.endpoint();
                let limits = LocalIpcFrameLimits::new(
                    NonZeroU32::new(FRAME_LIMIT_BYTES).expect("positive frame limit"),
                );
                let listener =
                    UnixSocketListener::bind(&endpoint, limits).expect("bind local resident");
                let response_admission = Arc::new(FixtureResponseAdmission::new());
                let peer_admissions = Arc::new(AtomicUsize::new(0));
                let attachment = ResidentAttachment::provision_operations(
                    Box::new(ConformanceClientCarrierSource {
                        endpoint,
                        limits,
                        peer_admissions: Arc::clone(&peer_admissions),
                    }),
                    Duration::from_secs(5),
                    response_admission.clone(),
                )
                .expect("connect the canonical SDK to the selected local resident");

                let start_request = journey.start_signed_common_request().to_owned();
                let requests = journey.clone();
                let client = std::thread::spawn(move || {
                    let client = McpConformanceClient::new(attachment)
                        .expect("MCP projects the attached local resident");
                    let unavailable = client
                        .call_tool(
                            "start_resident_consult",
                            object(serde_json::json!({
                                "signedCommonRequest": start_request.clone(),
                            })),
                        )
                        .expect("the first Consult Start reaches the resident");
                    assert_mcp_diagnostic(
                        &unavailable,
                        "resident_operation_outcome_unknown",
                        "resident_recovery",
                        "effect_outcome_unknown",
                        "reconcile_first",
                        "outcome_unknown",
                    );

                    let mut start = None;
                    for _ in 0..MAXIMUM_START_ATTEMPTS {
                        let retry = client
                            .call_tool(
                                "start_resident_consult",
                                object(serde_json::json!({
                                    "signedCommonRequest": start_request.clone(),
                                })),
                            )
                            .expect("the exact Consult Start retry reaches the resident");
                        if retry.is_error == Some(false) {
                            start = retry.structured_content;
                            break;
                        }
                        assert_mcp_diagnostic(
                            &retry,
                            "resident_operation_outcome_unknown",
                            "resident_recovery",
                            "effect_outcome_unknown",
                            "reconcile_first",
                            "outcome_unknown",
                        );
                    }
                    let start = start.expect(
                        "bounded scheduling makes the original Consult Start records observable",
                    );
                    let start_handle_ref = string(&start, "operationHandleRecordRef");
                    let start_handle = string(&start, "operationHandleCanonicalJson").into_bytes();
                    let start_handle_sha256 =
                        canonical_digest(&start_handle, "canonical_operation_handle_sha256");
                    let locator = ResidentOperationFrameLocator::new(
                        start_handle_ref.clone(),
                        start_handle_sha256.clone(),
                    );

                    let advance_request = requests
                        .advance_signed_common_request(&locator)
                        .expect("Consult Advance binds the Start-issued locator");
                    let advance = client
                        .call_tool(
                            "advance_resident_consult",
                            object(serde_json::json!({
                                "operationHandleRecordRef": &start_handle_ref,
                                "operationHandleSha256": &start_handle_sha256,
                                "signedCommonRequest": advance_request,
                            })),
                        )
                        .expect("Consult Advance reaches the same operation");
                    assert_eq!(advance.is_error, Some(false));
                    let advance = advance
                        .structured_content
                        .expect("Consult Advance returns its disposition");
                    assert_eq!(advance["kind"], "pending");
                    let advance_handle_ref =
                        string(&advance["locator"], "operationHandleRecordRef");
                    let advance_handle_sha256 =
                        string(&advance["locator"], "operationHandleSha256");

                    let observe_request = requests
                        .observe_signed_common_request(&locator, &start_handle)
                        .expect("Observe binds the admitted handle");
                    let observe = client
                        .call_tool(
                            "observe_resident_operation",
                            object(serde_json::json!({
                                "operationHandleRecordRef": &advance_handle_ref,
                                "operationHandleSha256": &advance_handle_sha256,
                                "signedCommonRequest": observe_request,
                            })),
                        )
                        .expect("cold Observe reaches the recovered resident");
                    assert_eq!(observe.is_error, Some(false));
                    let observe = observe
                        .structured_content
                        .expect("cold Observe returns signed records");
                    let observe_handle_ref = string(&observe, "operationHandleRecordRef");
                    let observe_handle =
                        string(&observe, "operationHandleCanonicalJson").into_bytes();
                    let observe_handle_sha256 =
                        canonical_digest(&observe_handle, "canonical_operation_handle_sha256");

                    ProjectedJourney {
                        start_handle_ref,
                        start_handle_sha256,
                        advance_handle_ref,
                        advance_handle_sha256,
                        observe_handle_ref,
                        observe_handle_sha256,
                    }
                });

                let carrier = listener
                    .accept(Instant::now() + Duration::from_secs(5))
                    .expect("accept the SDK connection");
                let mut connection = ConformanceSocketFrame::new(carrier);
                service
                    .serve_frame(
                        &mut connection,
                        Instant::now() + Duration::from_secs(5),
                        Instant::now() + Duration::from_secs(5),
                    )
                    .expect("serve the first Start turn");
                let first: ResidentOperationFrameResponse =
                    serde_json::from_slice(&connection.take_response())
                        .expect("the first Start response decodes");
                assert_eq!(
                    first,
                    ResidentOperationFrameResponse::Rejected {
                        failure: pale_ports::ResidentOperationFrameFailure::ResidentUnavailable,
                    }
                );

                let mut scheduler = ResidentWorkTurnCoordinator::new();
                let mut started = None;
                for _ in 0..MAXIMUM_START_ATTEMPTS {
                    assert!(matches!(
                        scheduler
                            .run_one(service, ResidentWorkTurnInput::idle())
                            .expect("advance one retained Consult phase"),
                        ResidentWorkTurnOutcome::RetainedPhaseAdvanced
                            | ResidentWorkTurnOutcome::RetainedPhaseDeferred
                    ));
                    service
                        .serve_frame(
                            &mut connection,
                            Instant::now() + Duration::from_secs(5),
                            Instant::now() + Duration::from_secs(5),
                        )
                        .expect("serve one exact Start retry");
                    let retry: ResidentOperationFrameResponse =
                        serde_json::from_slice(&connection.take_response())
                            .expect("the exact Start retry response decodes");
                    match retry {
                        ResidentOperationFrameResponse::Started { records } => {
                            started = Some(records);
                            break;
                        }
                        ResidentOperationFrameResponse::Rejected {
                            failure:
                                pale_ports::ResidentOperationFrameFailure::ResidentUnavailable,
                        } => {}
                        other => panic!(
                            "the exact Start retry must remain pending or recover Started; got {other:?}"
                        ),
                    }
                }
                let records = started.expect(
                    "bounded scheduling makes the original Consult Start records observable",
                );
                let handle: serde_json::Value =
                    pale_core::canon::parse_strict(records.operation_handle_canonical().as_bytes())
                        .expect("the Start handle is canonical");
                let locator = ResidentOperationFrameLocator::new(
                    records.operation_handle_record_ref().to_owned(),
                    string(&handle, "canonical_operation_handle_sha256"),
                );

                service
                    .serve_frame(
                        &mut connection,
                        Instant::now() + Duration::from_secs(5),
                        Instant::now() + Duration::from_secs(5),
                    )
                    .expect("serve Consult Advance");
                let advance: ResidentOperationFrameResponse =
                    serde_json::from_slice(&connection.take_response())
                        .expect("Consult Advance response decodes");
                assert!(matches!(
                    advance,
                    ResidentOperationFrameResponse::Pending { .. }
                ));

                (
                    locator,
                    SocketJourney {
                        client,
                        connection,
                        response_admission,
                        peer_admissions,
                        socket_directory: directory,
                    },
                )
            },
            |service, _, expected_locator, activity, mut socket| {
                service
                    .serve_frame(
                        &mut socket.connection,
                        Instant::now() + Duration::from_secs(5),
                        Instant::now() + Duration::from_secs(5),
                    )
                    .expect("serve Observe through the cold-recovered service");
                let observed: ResidentObserveFrameResponse =
                    serde_json::from_slice(&socket.connection.take_response())
                        .expect("cold Observe response decodes");
                assert!(matches!(
                    &observed,
                    ResidentObserveFrameResponse::Observed { .. }
                ), "cold Observe must return signed records; got {observed:?}");

                let projected = socket.client.join().expect("MCP client completes");
                assert_eq!(activity.start_policy_calls(), 1);
                assert_eq!(activity.initial_collection_calls(), 1);
                assert_eq!(
                    activity.finite_turn_calls(),
                    EXPECTED_START_ONLY_CONSULT_FINITE_TURNS,
                    "the socket scheduler stops when the durable Start becomes observable",
                );
                assert_eq!(socket.peer_admissions.load(Ordering::SeqCst), 1);
                assert_eq!(socket.response_admission.calls(), 2);
                assert!(
                    socket.socket_directory.path.is_dir(),
                    "the selected endpoint remains owned for the complete journey",
                );
                (expected_locator.clone(), projected)
            },
        )
        .expect("construct one socket-backed Consult conformance journey");

        let (expected_locator, projected) = completed;
        for handle_ref in [
            &projected.start_handle_ref,
            &projected.advance_handle_ref,
            &projected.observe_handle_ref,
        ] {
            assert_eq!(handle_ref, expected_locator.operation_handle_record_ref());
        }
        for handle_sha256 in [
            &projected.start_handle_sha256,
            &projected.advance_handle_sha256,
            &projected.observe_handle_sha256,
        ] {
            assert_eq!(handle_sha256, expected_locator.operation_handle_sha256());
        }
    }

    fn assert_mcp_diagnostic(
        result: &rmcp::model::CallToolResult,
        code: &str,
        stage: &str,
        disposition: &str,
        retry: &str,
        effect: &str,
    ) {
        assert_eq!(result.is_error, Some(true));
        let diagnostic = &result
            .structured_content
            .as_ref()
            .expect("MCP failure carries one structured diagnostic")["diagnostic"];
        assert_eq!(diagnostic["code"], code);
        assert_eq!(diagnostic["stage"], stage);
        assert_eq!(diagnostic["disposition"], disposition);
        assert_eq!(diagnostic["retry"], retry);
        assert_eq!(diagnostic["effect"], effect);
    }

    fn object(value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        value.as_object().expect("tool input is an object").clone()
    }

    fn string(value: &serde_json::Value, field: &str) -> String {
        value[field]
            .as_str()
            .unwrap_or_else(|| panic!("{field} is a string"))
            .to_owned()
    }

    fn canonical_digest(bytes: &[u8], field: &str) -> String {
        pale_core::canon::parse_strict(bytes).expect("returned record is canonical JSON")[field]
            .as_str()
            .unwrap_or_else(|| panic!("{field} is a string"))
            .to_owned()
    }

    fn admit_operation_records_when_present(
        authority: &DeterministicResponseAdmission,
        response: &[u8],
    ) {
        let response: ResidentOperationFrameResponse =
            serde_json::from_slice(response).expect("operation response decodes");
        let records = match response {
            ResidentOperationFrameResponse::Started { records }
            | ResidentOperationFrameResponse::Observed { records } => records,
            ResidentOperationFrameResponse::CancellationDisposition { .. }
            | ResidentOperationFrameResponse::Pending { .. }
            | ResidentOperationFrameResponse::Unsupported { .. }
            | ResidentOperationFrameResponse::Rejected { .. } => return,
        };
        authority
            .expect(
                records.operation_handle_record_ref(),
                records.operation_handle_canonical().as_bytes(),
                records.operation_observation_record_ref(),
                records.operation_observation_canonical().as_bytes(),
            )
            .expect("application operation evidence passes strict response admission");
    }

    fn admit_observe_records(
        authority: &DeterministicResponseAdmission,
        response: &[u8],
    ) -> Option<ResidentObserveFrameFailure> {
        let response: ResidentObserveFrameResponse =
            serde_json::from_slice(response).expect("Observe response decodes");
        match response {
            ResidentObserveFrameResponse::Observed {
                operation_handle_record_ref,
                operation_handle_canonical,
                operation_observation_record_ref,
                operation_observation_canonical,
            } => {
                authority
                    .expect(
                        &operation_handle_record_ref,
                        operation_handle_canonical.as_bytes(),
                        &operation_observation_record_ref,
                        operation_observation_canonical.as_bytes(),
                    )
                    .expect("application Observe evidence passes strict response admission");
                None
            }
            ResidentObserveFrameResponse::Rejected { failure } => Some(failure),
        }
    }
}
