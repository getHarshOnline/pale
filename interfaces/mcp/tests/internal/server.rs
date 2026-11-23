//! Laws for server capabilities and diagnostic carriage.

use super::*;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[path = "resident_conformance.rs"]
mod resident_conformance;

struct UnusedAuthenticatedCarrier;

struct SingleCarrierSource {
    carrier: Option<Box<dyn pale_cli::command::AuthenticatedResidentCarrier>>,
}

impl pale_cli::command::AuthenticatedResidentCarrierSource for SingleCarrierSource {
    fn open(
        &mut self,
        _deadline: std::time::Instant,
    ) -> Result<
        Box<dyn pale_cli::command::AuthenticatedResidentCarrier>,
        pale_cli::command::AuthenticatedResidentCarrierSourceFailure,
    > {
        self.carrier
            .take()
            .ok_or(pale_cli::command::AuthenticatedResidentCarrierSourceFailure)
    }
}

fn carrier_source(
    carrier: impl pale_cli::command::AuthenticatedResidentCarrier + 'static,
) -> Box<dyn pale_cli::command::AuthenticatedResidentCarrierSource> {
    Box::new(SingleCarrierSource {
        carrier: Some(Box::new(carrier)),
    })
}

impl pale_cli::command::AuthenticatedResidentCarrier for UnusedAuthenticatedCarrier {
    fn exchange(
        &mut self,
        _request: &[u8],
        _deadline: std::time::Instant,
    ) -> Result<Box<[u8]>, pale_cli::command::AuthenticatedResidentCarrierFailure> {
        Err(pale_cli::command::AuthenticatedResidentCarrierFailure::ConnectionUnavailable)
    }
}

struct RejectingResponseAdmission;

impl pale_cli::command::ResidentResponseAdmission for RejectingResponseAdmission {
    fn admit(
        &self,
        _operation_handle_record_ref: &str,
        _operation_handle_canonical_bytes: &[u8],
        _operation_observation_record_ref: &str,
        _operation_observation_canonical_bytes: &[u8],
        _deadline: std::time::Instant,
    ) -> Result<(), pale_cli::command::ResidentResponseAdmissionFailure> {
        Err(pale_cli::command::ResidentResponseAdmissionFailure)
    }
}

fn attached_dispatcher() -> CommandDispatcher {
    let attachment = pale_cli::command::ResidentAttachment::provision_operations(
        carrier_source(UnusedAuthenticatedCarrier),
        std::time::Duration::from_secs(1),
        Arc::new(RejectingResponseAdmission),
    )
    .expect("positive timeout provisions an attachment");
    CommandDispatcher::with_resident_attachment(attachment).unwrap()
}

struct ApplicationObserveCarrier {
    expected_request: serde_json::Value,
    response: Option<Box<[u8]>>,
}

struct ScriptedOperationCarrier {
    calls: Arc<AtomicUsize>,
    sent: Arc<Mutex<Vec<serde_json::Value>>>,
    responses: VecDeque<Result<Box<[u8]>, pale_cli::command::AuthenticatedResidentCarrierFailure>>,
}

impl pale_cli::command::AuthenticatedResidentCarrier for ScriptedOperationCarrier {
    fn exchange(
        &mut self,
        request: &[u8],
        deadline: std::time::Instant,
    ) -> Result<Box<[u8]>, pale_cli::command::AuthenticatedResidentCarrierFailure> {
        assert!(std::time::Instant::now() < deadline);
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.sent
            .lock()
            .expect("resident send log remains available")
            .push(serde_json::from_slice(request).expect("resident request frame is JSON"));
        self.responses.pop_front().unwrap_or(Err(
            pale_cli::command::AuthenticatedResidentCarrierFailure::ConnectionUnavailable,
        ))
    }
}

impl pale_cli::command::AuthenticatedResidentCarrier for ApplicationObserveCarrier {
    fn exchange(
        &mut self,
        request: &[u8],
        deadline: std::time::Instant,
    ) -> Result<Box<[u8]>, pale_cli::command::AuthenticatedResidentCarrierFailure> {
        assert!(std::time::Instant::now() < deadline);
        let request: serde_json::Value =
            serde_json::from_slice(request).expect("resident request frame is JSON");
        assert_eq!(request, self.expected_request);
        self.response
            .take()
            .ok_or(pale_cli::command::AuthenticatedResidentCarrierFailure::ConnectionUnavailable)
    }
}

struct AcceptingResponseAdmission;

impl pale_cli::command::ResidentResponseAdmission for AcceptingResponseAdmission {
    fn admit(
        &self,
        _operation_handle_record_ref: &str,
        _operation_handle_canonical_bytes: &[u8],
        _operation_observation_record_ref: &str,
        _operation_observation_canonical_bytes: &[u8],
        deadline: std::time::Instant,
    ) -> Result<(), pale_cli::command::ResidentResponseAdmissionFailure> {
        assert!(std::time::Instant::now() < deadline);
        Ok(())
    }
}

fn application_observe_fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/pale-application/tests/fixtures/resident/application-observe-sdk.v1.json"
    )))
    .expect("application-owned Observe fixture parses")
}

fn application_invoke_fixture() -> serde_json::Value {
    let value: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/pale-application/tests/fixtures/resident/application-invoke-sdk.v1.json"
    )))
    .expect("application-owned Invoke fixture parses");
    assert_eq!(value["fixture_version"], 1);
    value
}

fn application_invoke_advance_fixture() -> serde_json::Value {
    let value: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/pale-application/tests/fixtures/resident/application-invoke-advance-sdk.v1.json"
    )))
    .expect("application-owned AdvanceInvoke fixture parses");
    assert_eq!(value["fixture_version"], 1);
    value
}

fn application_delegate_fixture() -> serde_json::Value {
    let value: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/pale-application/tests/fixtures/resident/application-delegate-sdk.v1.json"
    )))
    .expect("application-owned Delegate fixture parses");
    assert_eq!(value["fixture_version"], 1);
    value
}

fn application_delegate_advance_fixture() -> serde_json::Value {
    let value: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/pale-application/tests/fixtures/resident/application-delegate-advance-sdk.v1.json"
    )))
    .expect("application-owned AdvanceDelegate fixture parses");
    assert_eq!(value["fixture_version"], 1);
    value
}

fn application_cancel_fixture() -> serde_json::Value {
    let value: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/pale-application/tests/fixtures/resident/application-cancel-sdk.v1.json"
    )))
    .expect("application-owned Cancel fixture parses");
    assert_eq!(value["fixture_version"], 1);
    value
}

fn resident_operation_arguments(fixture: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "operationHandleRecordRef": fixture["operation_handle_record_ref"],
        "operationHandleSha256": fixture["operation_handle_sha256"],
        "signedCommonRequest": serde_json::to_string(&fixture["signed_common_request"])
            .expect("signed request serializes canonically"),
    })
}

fn scripted_response(
    value: serde_json::Value,
) -> Result<Box<[u8]>, pale_cli::command::AuthenticatedResidentCarrierFailure> {
    Ok(serde_json::to_vec(&value)
        .expect("scripted response serializes")
        .into_boxed_slice())
}

fn execute_complete(
    runtime: &tokio::runtime::Runtime,
    server: &PaleMcpServer,
    name: &str,
    arguments: serde_json::Value,
) -> CallToolResult {
    let response = runtime
        .block_on(
            server.execute_tool(
                CallToolRequestParams::new(name.to_owned()).with_arguments(
                    arguments
                        .as_object()
                        .expect("tool arguments are an object")
                        .clone(),
                ),
            ),
        )
        .expect("attached MCP tool returns a protocol result");
    let CallToolResponse::Complete(result) = response else {
        panic!("resident tools do not request interactive input")
    };
    result
}

#[test]
fn application_observe_fixture_crosses_the_sdk_dispatcher_and_mcp_projection() {
    let fixture = application_observe_fixture();
    let signed_common_request = serde_json::to_string(&fixture["signed_common_request"])
        .expect("signed request serializes canonically");
    let expected_request = serde_json::json!({
        "operationHandleRecordRef": fixture["operation_handle_record_ref"],
        "operationHandleSha256": fixture["operation_handle_sha256"],
        "signedCommonRequest": signed_common_request,
    });
    let response = serde_json::json!({
        "kind": "observed",
        "operationHandleRecordRef": fixture["operation_handle_record_ref"],
        "operationHandleCanonical": serde_json::to_string(&fixture["operation_handle_record"])
            .expect("operation handle serializes canonically"),
        "operationObservationRecordRef": fixture["operation_observation_record_ref"],
        "operationObservationCanonical":
            serde_json::to_string(&fixture["operation_observation_record"])
                .expect("operation observation serializes canonically"),
    });
    let attachment = pale_cli::command::ResidentAttachment::provision_observation(
        carrier_source(ApplicationObserveCarrier {
            expected_request: expected_request.clone(),
            response: Some(
                serde_json::to_vec(&response)
                    .expect("Observe response serializes canonically")
                    .into_boxed_slice(),
            ),
        }),
        std::time::Duration::from_secs(1),
        Arc::new(AcceptingResponseAdmission),
    )
    .expect("positive timeout provisions an attachment");
    let server = PaleMcpServer::new(
        CommandDispatcher::with_resident_attachment(attachment)
            .expect("dispatcher verifies its embedded contracts"),
    )
    .expect("attached MCP server projects its routes");
    assert!(server.get_tool("observe_resident_operation").is_some());
    for unavailable in [
        "start_resident_invoke",
        "advance_resident_invoke",
        "start_resident_consult",
        "advance_resident_consult",
        "start_resident_delegate",
        "advance_resident_delegate",
        "cancel_resident_operation",
    ] {
        assert!(
            server.get_tool(unavailable).is_none(),
            "read-only attachment advertised {unavailable}"
        );
    }
    assert!(server
        .get_info()
        .instructions
        .as_deref()
        .is_some_and(|value| value.contains("read-only resident Observe")));

    let arguments = expected_request
        .as_object()
        .expect("Observe arguments are an object")
        .clone();
    let result = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("current-thread runtime builds")
        .block_on(server.execute_tool(
            CallToolRequestParams::new("observe_resident_operation").with_arguments(arguments),
        ))
        .expect("MCP Observe reaches the supplied resident carrier");

    let CallToolResponse::Complete(result) = result else {
        panic!("resident Observe completes without interactive input or task materialization");
    };
    assert_eq!(
        result.structured_content,
        Some(serde_json::json!({
            "operationHandleRecordRef": fixture["operation_handle_record_ref"],
            "operationHandleCanonicalJson": serde_json::to_string(&fixture["operation_handle_record"])
                .expect("handle record canonicalizes"),
            "operationObservationRecordRef": fixture["operation_observation_record_ref"],
            "operationObservationCanonicalJson":
                serde_json::to_string(&fixture["operation_observation_record"])
                    .expect("observation record canonicalizes"),
        }))
    );
}

#[test]
fn application_produced_invoke_start_crosses_mcp_without_claiming_live_resident_assembly() {
    let fixture = application_invoke_fixture();
    let request_frame = fixture["request_frame"].clone();
    let response_frame = fixture["response_frame"].clone();
    assert_eq!(request_frame["kind"], "start_invoke");
    assert_eq!(response_frame["kind"], "started");
    let records = response_frame["records"].clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = pale_cli::command::ResidentAttachment::provision_operations(
        carrier_source(ScriptedOperationCarrier {
            calls: Arc::clone(&calls),
            sent: Arc::clone(&sent),
            responses: VecDeque::from([scripted_response(response_frame)]),
        }),
        std::time::Duration::from_secs(1),
        Arc::new(AcceptingResponseAdmission),
    )
    .expect("positive timeout provisions the attachment");
    let server = PaleMcpServer::new(
        CommandDispatcher::with_resident_attachment(attachment)
            .expect("dispatcher verifies embedded contracts"),
    )
    .expect("attached MCP server projects operation routes");
    let signed_common_request = request_frame["signedCommonRequest"]
        .as_str()
        .expect("StartInvoke frame contains canonical signed request");

    let result = execute_complete(
        &tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("current-thread runtime builds"),
        &server,
        "start_resident_invoke",
        serde_json::json!({"signedCommonRequest": signed_common_request}),
    );

    assert_eq!(result.is_error, Some(false));
    assert_eq!(
        result.structured_content,
        Some(serde_json::json!({
            "operationHandleRecordRef": records["operationHandleRecordRef"],
            "operationHandleCanonicalJson": records["operationHandleCanonical"],
            "operationObservationRecordRef": records["operationObservationRecordRef"],
            "operationObservationCanonicalJson": records["operationObservationCanonical"],
        }))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .as_slice(),
        [request_frame]
    );
}

#[test]
fn application_produced_invoke_advance_crosses_mcp_as_one_honest_pending_turn() {
    let fixture = application_invoke_advance_fixture();
    let request_frame = fixture["request_frame"].clone();
    let response_frame = fixture["response_frame"].clone();
    assert_eq!(request_frame["kind"], "advance_invoke");
    assert_eq!(response_frame["kind"], "pending");
    assert_eq!(request_frame["locator"], response_frame["locator"]);
    let calls = Arc::new(AtomicUsize::new(0));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = pale_cli::command::ResidentAttachment::provision_operations(
        carrier_source(ScriptedOperationCarrier {
            calls: Arc::clone(&calls),
            sent: Arc::clone(&sent),
            responses: VecDeque::from([scripted_response(response_frame)]),
        }),
        std::time::Duration::from_secs(1),
        Arc::new(AcceptingResponseAdmission),
    )
    .expect("positive timeout provisions the attachment");
    let server = PaleMcpServer::new(
        CommandDispatcher::with_resident_attachment(attachment)
            .expect("dispatcher verifies embedded contracts"),
    )
    .expect("attached MCP server projects operation routes");
    let locator = &request_frame["locator"];
    let signed_common_request = request_frame["signedCommonRequest"]
        .as_str()
        .expect("AdvanceInvoke frame contains a canonical signed request");

    let result = execute_complete(
        &tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("current-thread runtime builds"),
        &server,
        "advance_resident_invoke",
        serde_json::json!({
            "operationHandleRecordRef": locator["operationHandleRecordRef"],
            "operationHandleSha256": locator["operationHandleSha256"],
            "signedCommonRequest": signed_common_request,
        }),
    );

    assert_eq!(result.is_error, Some(false));
    assert_eq!(
        result.structured_content,
        Some(serde_json::json!({
            "kind": "pending",
            "locator": locator,
        }))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .as_slice(),
        [request_frame],
        "MCP must not replay AdvanceInvoke",
    );
}

#[test]
fn application_produced_delegate_start_crosses_mcp_without_claiming_live_resident_assembly() {
    let fixture = application_delegate_fixture();
    let request_frame = fixture["request_frame"].clone();
    let response_frame = fixture["response_frame"].clone();
    assert_eq!(request_frame["kind"], "start_delegate");
    assert_eq!(response_frame["kind"], "started");
    let records = response_frame["records"].clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = pale_cli::command::ResidentAttachment::provision_operations(
        carrier_source(ScriptedOperationCarrier {
            calls: Arc::clone(&calls),
            sent: Arc::clone(&sent),
            responses: VecDeque::from([scripted_response(response_frame)]),
        }),
        std::time::Duration::from_secs(1),
        Arc::new(AcceptingResponseAdmission),
    )
    .expect("positive timeout provisions the attachment");
    let server = PaleMcpServer::new(
        CommandDispatcher::with_resident_attachment(attachment)
            .expect("dispatcher verifies embedded contracts"),
    )
    .expect("attached MCP server projects operation routes");
    let parent = &request_frame["parent"];
    let signed_common_request = request_frame["signedCommonRequest"]
        .as_str()
        .expect("StartDelegate frame contains canonical signed request");

    let result = execute_complete(
        &tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("current-thread runtime builds"),
        &server,
        "start_resident_delegate",
        serde_json::json!({
            "parentOperationHandleRecordRef": parent["operationHandleRecordRef"],
            "parentOperationHandleSha256": parent["operationHandleSha256"],
            "signedCommonRequest": signed_common_request,
        }),
    );

    assert_eq!(result.is_error, Some(false));
    assert_eq!(
        result.structured_content,
        Some(serde_json::json!({
            "operationHandleRecordRef": records["operationHandleRecordRef"],
            "operationHandleCanonicalJson": records["operationHandleCanonical"],
            "operationObservationRecordRef": records["operationObservationRecordRef"],
            "operationObservationCanonicalJson": records["operationObservationCanonical"],
        }))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .as_slice(),
        [request_frame]
    );
}

#[test]
fn application_produced_delegate_advance_crosses_mcp_once() {
    let fixture = application_delegate_advance_fixture();
    let request_frame = fixture["request_frame"].clone();
    let response_frame = fixture["response_frame"].clone();
    assert_eq!(request_frame["kind"], "advance_delegate");
    assert_eq!(response_frame["kind"], "pending");
    let calls = Arc::new(AtomicUsize::new(0));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = pale_cli::command::ResidentAttachment::provision_operations(
        carrier_source(ScriptedOperationCarrier {
            calls: Arc::clone(&calls),
            sent: Arc::clone(&sent),
            responses: VecDeque::from([scripted_response(response_frame)]),
        }),
        std::time::Duration::from_secs(1),
        Arc::new(AcceptingResponseAdmission),
    )
    .expect("positive timeout provisions the attachment");
    let server = PaleMcpServer::new(
        CommandDispatcher::with_resident_attachment(attachment)
            .expect("dispatcher verifies embedded contracts"),
    )
    .expect("attached MCP server projects operation routes");
    let locator = &request_frame["locator"];
    let signed_common_request = request_frame["signedCommonRequest"]
        .as_str()
        .expect("AdvanceDelegate frame contains canonical signed request");

    let result = execute_complete(
        &tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("current-thread runtime builds"),
        &server,
        "advance_resident_delegate",
        serde_json::json!({
            "operationHandleRecordRef": locator["operationHandleRecordRef"],
            "operationHandleSha256": locator["operationHandleSha256"],
            "signedCommonRequest": signed_common_request,
        }),
    );

    assert_eq!(result.is_error, Some(false));
    assert_eq!(
        result.structured_content,
        Some(serde_json::json!({
            "kind": "pending",
            "locator": {
                "operationHandleRecordRef": locator["operationHandleRecordRef"],
                "operationHandleSha256": locator["operationHandleSha256"],
            },
        }))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .as_slice(),
        [request_frame],
        "MCP must not replay AdvanceDelegate",
    );
}

#[test]
fn application_produced_cancel_crosses_mcp_without_claiming_live_resident_assembly() {
    let fixture = application_cancel_fixture();
    let request_frame = fixture["request_frame"].clone();
    let response_frame = fixture["response_frame"].clone();
    assert_eq!(request_frame["kind"], "cancel");
    assert_eq!(response_frame["kind"], "cancellation_disposition");
    assert_eq!(response_frame["disposition"], "locally_cancelled");
    let calls = Arc::new(AtomicUsize::new(0));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = pale_cli::command::ResidentAttachment::provision_operations(
        carrier_source(ScriptedOperationCarrier {
            calls: Arc::clone(&calls),
            sent: Arc::clone(&sent),
            responses: VecDeque::from([scripted_response(response_frame)]),
        }),
        std::time::Duration::from_secs(1),
        Arc::new(AcceptingResponseAdmission),
    )
    .expect("positive timeout provisions the attachment");
    let server = PaleMcpServer::new(
        CommandDispatcher::with_resident_attachment(attachment)
            .expect("dispatcher verifies embedded contracts"),
    )
    .expect("attached MCP server projects operation routes");
    let locator = &request_frame["locator"];
    let signed_common_request = request_frame["signedCommonRequest"]
        .as_str()
        .expect("Cancel frame contains canonical signed request");

    let result = execute_complete(
        &tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("current-thread runtime builds"),
        &server,
        "cancel_resident_operation",
        serde_json::json!({
            "operationHandleRecordRef": locator["operationHandleRecordRef"],
            "operationHandleSha256": locator["operationHandleSha256"],
            "signedCommonRequest": signed_common_request,
        }),
    );

    assert_eq!(result.is_error, Some(false));
    assert_eq!(
        result.structured_content,
        Some(serde_json::json!({
            "locator": {
                "operationHandleRecordRef": locator["operationHandleRecordRef"],
                "operationHandleSha256": locator["operationHandleSha256"],
            },
            "disposition": "locally_cancelled",
        }))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .as_slice(),
        [request_frame]
    );
}

#[test]
fn attached_operation_tools_preserve_pending_unsupported_and_cancellation_results() {
    let fixture = application_observe_fixture();
    let root_signed_request = resident_operation_arguments(&fixture)["signedCommonRequest"].clone();
    let cancel_request = application_cancel_fixture()["request_frame"].clone();
    let cancel_locator = cancel_request["locator"].clone();
    let cancel_arguments = serde_json::json!({
        "operationHandleRecordRef": cancel_locator["operationHandleRecordRef"],
        "operationHandleSha256": cancel_locator["operationHandleSha256"],
        "signedCommonRequest": cancel_request["signedCommonRequest"],
    });
    let invoke_advance = application_invoke_advance_fixture()["request_frame"].clone();
    let invoke_locator = invoke_advance["locator"].clone();
    let invoke_arguments = serde_json::json!({
        "operationHandleRecordRef": invoke_locator["operationHandleRecordRef"],
        "operationHandleSha256": invoke_locator["operationHandleSha256"],
        "signedCommonRequest": invoke_advance["signedCommonRequest"],
    });
    let delegate_start = application_delegate_fixture()["request_frame"].clone();
    let delegate_parent = delegate_start["parent"].clone();
    let delegate_advance = application_delegate_advance_fixture()["request_frame"].clone();
    let delegate_locator = delegate_advance["locator"].clone();
    let delegate_arguments = serde_json::json!({
        "operationHandleRecordRef": delegate_locator["operationHandleRecordRef"],
        "operationHandleSha256": delegate_locator["operationHandleSha256"],
        "signedCommonRequest": delegate_advance["signedCommonRequest"],
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = pale_cli::command::ResidentAttachment::provision_operations(
        carrier_source(ScriptedOperationCarrier {
            calls: Arc::clone(&calls),
            sent: Arc::clone(&sent),
            responses: VecDeque::from([
                scripted_response(serde_json::json!({
                    "kind":"rejected",
                    "failure":"invalid_request"
                })),
                scripted_response(serde_json::json!({
                    "kind":"rejected",
                    "failure":"invalid_request"
                })),
                scripted_response(serde_json::json!({
                    "kind":"rejected",
                    "failure":"invalid_request"
                })),
                scripted_response(serde_json::json!({
                    "kind":"pending",
                    "locator":invoke_locator.clone()
                })),
                scripted_response(serde_json::json!({
                    "kind":"pending",
                    "locator":invoke_locator.clone()
                })),
                scripted_response(serde_json::json!({
                    "kind":"unsupported",
                    "locator":delegate_locator.clone(),
                    "reason":"delegate_route_not_selected"
                })),
                scripted_response(serde_json::json!({
                    "kind":"cancellation_disposition",
                    "locator":cancel_locator,
                    "disposition":"terminal_already_established"
                })),
            ]),
        }),
        std::time::Duration::from_secs(1),
        Arc::new(AcceptingResponseAdmission),
    )
    .expect("positive timeout provisions the attachment");
    let server = PaleMcpServer::new(
        CommandDispatcher::with_resident_attachment(attachment)
            .expect("dispatcher verifies embedded contracts"),
    )
    .expect("attached MCP server projects operation routes");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("current-thread runtime builds");

    for (name, arguments) in [
        (
            "start_resident_invoke",
            serde_json::json!({"signedCommonRequest": root_signed_request.clone()}),
        ),
        (
            "start_resident_consult",
            serde_json::json!({"signedCommonRequest": root_signed_request}),
        ),
        (
            "start_resident_delegate",
            serde_json::json!({
                "parentOperationHandleRecordRef": delegate_parent["operationHandleRecordRef"],
                "parentOperationHandleSha256": delegate_parent["operationHandleSha256"],
                "signedCommonRequest": delegate_start["signedCommonRequest"],
            }),
        ),
    ] {
        let result = execute_complete(&runtime, &server, name, arguments);
        assert_eq!(result.is_error, Some(true));
        assert_eq!(
            result
                .structured_content
                .as_ref()
                .expect("API failure has structured content")["diagnostic"]["code"],
            "resident_operation_request_rejected"
        );
    }

    for name in ["advance_resident_invoke", "advance_resident_consult"] {
        let result = execute_complete(&runtime, &server, name, invoke_arguments.clone());
        assert_eq!(result.is_error, Some(false));
        assert_eq!(
            result.structured_content,
            Some(serde_json::json!({
                "kind":"pending",
                "locator":{
                    "operationHandleRecordRef":invoke_locator["operationHandleRecordRef"],
                    "operationHandleSha256":invoke_locator["operationHandleSha256"],
                }
            }))
        );
    }

    let unsupported = execute_complete(
        &runtime,
        &server,
        "advance_resident_delegate",
        delegate_arguments,
    );
    assert_eq!(unsupported.is_error, Some(false));
    assert_eq!(
        unsupported.structured_content,
        Some(serde_json::json!({
            "kind":"unsupported",
            "locator":{
                "operationHandleRecordRef":delegate_locator["operationHandleRecordRef"],
                "operationHandleSha256":delegate_locator["operationHandleSha256"],
            },
            "reason":"delegate_route_not_selected"
        }))
    );

    let cancellation = execute_complete(
        &runtime,
        &server,
        "cancel_resident_operation",
        cancel_arguments,
    );
    assert_eq!(cancellation.is_error, Some(false));
    assert_eq!(
        cancellation.structured_content,
        Some(serde_json::json!({
            "locator":{
                "operationHandleRecordRef":cancel_request["locator"]["operationHandleRecordRef"],
                "operationHandleSha256":cancel_request["locator"]["operationHandleSha256"],
            },
            "disposition":"terminal_already_established"
        }))
    );

    assert_eq!(calls.load(Ordering::SeqCst), 7);
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .iter()
            .map(|frame| frame["kind"].as_str().expect("frame kind is a string"))
            .collect::<Vec<_>>(),
        [
            "start_invoke",
            "start_consult",
            "start_delegate",
            "advance_invoke",
            "advance_consult",
            "advance_delegate",
            "cancel",
        ]
    );
}

#[test]
fn outcome_unknown_and_an_exhausted_source_remain_distinct() {
    let request_frame = application_invoke_advance_fixture()["request_frame"].clone();
    let locator = &request_frame["locator"];
    let arguments = serde_json::json!({
        "operationHandleRecordRef": locator["operationHandleRecordRef"],
        "operationHandleSha256": locator["operationHandleSha256"],
        "signedCommonRequest": request_frame["signedCommonRequest"],
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = pale_cli::command::ResidentAttachment::provision_operations(
        carrier_source(ScriptedOperationCarrier {
            calls: Arc::clone(&calls),
            sent: Arc::clone(&sent),
            responses: VecDeque::from([Err(
                pale_cli::command::AuthenticatedResidentCarrierFailure::OutcomeUnknown,
            )]),
        }),
        std::time::Duration::from_secs(1),
        Arc::new(AcceptingResponseAdmission),
    )
    .expect("positive timeout provisions the attachment");
    let server = PaleMcpServer::new(
        CommandDispatcher::with_resident_attachment(attachment)
            .expect("dispatcher verifies embedded contracts"),
    )
    .expect("attached MCP server projects operation routes");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("current-thread runtime builds");

    let first = execute_complete(
        &runtime,
        &server,
        "advance_resident_invoke",
        arguments.clone(),
    );
    let second = execute_complete(&runtime, &server, "advance_resident_invoke", arguments);
    assert_eq!(first.is_error, Some(true));
    assert_eq!(second.is_error, Some(true));
    let first_structured = first
        .structured_content
        .expect("unknown outcome has structured diagnostic content");
    assert_eq!(
        first_structured["diagnostic"]["code"],
        "resident_operation_failed"
    );
    assert_eq!(first_structured["diagnostic"]["effect"], "outcome_unknown");
    assert_eq!(first_structured["diagnostic"]["retry"], "reconcile_first");
    assert_eq!(
        first_structured["authoringCorrections"],
        serde_json::json!([])
    );
    let second_structured = second
        .structured_content
        .expect("the exhausted source has structured diagnostic content");
    assert_eq!(
        second_structured["diagnostic"]["code"],
        "resident_operation_not_sent"
    );
    assert_eq!(second_structured["diagnostic"]["effect"], "not_started");
    assert_eq!(
        second_structured["diagnostic"]["retry"],
        "after_state_change"
    );
    assert_eq!(
        second_structured["authoringCorrections"],
        serde_json::json!([])
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .as_slice(),
        [request_frame],
        "the single-carrier source supplies no second transport",
    );
}

#[test]
fn server_advertises_only_immutable_resources_and_effect_free_tools() {
    let server = PaleMcpServer::new(CommandDispatcher::new().unwrap()).unwrap();
    let info = server.get_info();
    assert_eq!(
        info.protocol_version,
        rmcp::model::ProtocolVersion::V_2026_07_28
    );
    assert_eq!(info.server_info.name, "pale-mcp");
    assert_eq!(info.server_info.version, "0.0.1");
    assert!(info.capabilities.tools.is_some());
    assert!(info.capabilities.resources.is_some());
    assert!(info.capabilities.prompts.is_none());
    assert!(info.capabilities.logging.is_none());
    assert!(info.capabilities.completions.is_none());
    assert!(!info.capabilities.supports_tasks());
    assert!(info.capabilities.experimental.is_none());
    assert!(info.capabilities.extensions.is_none());
    for unavailable in [
        "observe_resident_operation",
        "start_resident_invoke",
        "advance_resident_invoke",
        "start_resident_consult",
        "advance_resident_consult",
        "start_resident_delegate",
        "advance_resident_delegate",
        "cancel_resident_operation",
    ] {
        assert!(
            server.get_tool(unavailable).is_none(),
            "stateless MCP must not advertise {unavailable}"
        );
    }
    assert!(info
        .instructions
        .as_deref()
        .is_some_and(|value| value.contains("pure deterministic Pale mechanics")));
}

#[test]
fn attached_server_advertises_resident_tools_and_uncertainty_guidance() {
    let server = PaleMcpServer::new(attached_dispatcher()).unwrap();
    let info = server.get_info();
    assert!(server.get_tool("observe_resident_operation").is_some());
    assert!(server.get_tool("start_resident_invoke").is_some());
    assert!(server.get_tool("start_resident_consult").is_some());
    let advance_consult = server
        .get_tool("advance_resident_consult")
        .expect("an attached server exposes bounded Consult advancement");
    assert!(advance_consult
        .description
        .as_deref()
        .is_some_and(|description| description.contains("exact retained compiled policy")));
    assert!(server.get_tool("start_resident_delegate").is_some());
    assert!(server.get_tool("cancel_resident_operation").is_some());
    assert!(info.instructions.as_deref().is_some_and(|value| {
        ["Invoke", "Consult", "Delegate", "Observe", "Cancel"]
            .into_iter()
            .all(|operation| value.contains(operation))
            && value.contains("Cancel acknowledgement does not prove terminal cancellation")
            && value.contains("Observe or reconcile before any retry")
    }));
}

#[test]
fn api_failure_projection_preserves_the_exact_canonical_diagnostic() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let error = dispatcher
        .dispatch(pale_cli::command::CommandRequest::ValidateContractDocument(
            pale_cli::command::ContractDocument::new(
                "action-input.v1".to_string(),
                b"not JSON".to_vec(),
            ),
        ))
        .unwrap_err();
    let result = encode_api_failure(&error).unwrap();
    assert_eq!(result.is_error, Some(true));
    assert_eq!(
        result.structured_content,
        Some(serde_json::json!({
            "diagnostic": serde_json::from_slice::<serde_json::Value>(error.diagnostic().canonical_bytes()).unwrap(),
            "diagnosticIdentity": error.diagnostic().identity(),
            "authoringCorrections": [],
            "authoringLocation": null,
        }))
    );
    assert_eq!(
        serde_json::to_value(&result.content).unwrap(),
        serde_json::json!([{
            "type": "text",
            "text": format!("{}: {}", error.diagnostic().code(), error.diagnostic().summary()),
        }]),
    );
}

#[test]
fn resource_failure_projection_preserves_the_exact_canonical_diagnostic() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let error = dispatcher
        .dispatch(pale_cli::command::CommandRequest::ContractSchema(
            "not-a-selected-schema".to_owned(),
        ))
        .unwrap_err();
    let projected = resources::diagnostic_resource_error(error.diagnostic());

    assert_eq!(
        projected.data,
        Some(
            serde_json::from_slice(error.diagnostic().canonical_bytes())
                .expect("canonical API diagnostic decodes at the MCP resource edge")
        )
    );
}

#[test]
fn saturation_rejects_the_real_tool_path_before_payload_decoding() {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let server = PaleMcpServer::new(CommandDispatcher::new().unwrap()).unwrap();
            let held = (0..MAX_CONCURRENT_REQUESTS)
                .map(|_| acquire_worker(&server.permits).unwrap())
                .collect::<Vec<_>>();
            let malformed = CallToolRequestParams::new("validate_contract");

            let overloaded = server.execute_tool(malformed.clone()).await.unwrap_err();
            assert!(overloaded.message.contains("busy"));

            drop(held);
            let decoded = server.execute_tool(malformed).await.unwrap_err();
            assert!(!decoded.message.contains("busy"));
            assert_eq!(decoded.code, rmcp::model::ErrorCode::INVALID_PARAMS);
        });
}
