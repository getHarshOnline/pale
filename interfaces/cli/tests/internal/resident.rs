//! Laws for the injected resident command projection.

use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::command::{
    AuthenticatedResidentCarrier, AuthenticatedResidentCarrierFailure,
    AuthenticatedResidentCarrierSource, AuthenticatedResidentCarrierSourceFailure, CommandOutcome,
    CommandRequest, OperationHandleLocator, ResidentAttachment, ResidentConsultAdvanceOutcome,
    ResidentConsultAdvanceRequest, ResidentConsultStartRequest, ResidentDelegateAdvanceOutcome,
    ResidentDelegateAdvanceRequest, ResidentDelegateStartRequest, ResidentInvokeAdvanceOutcome,
    ResidentInvokeAdvanceRequest, ResidentInvokeStartRequest,
    ResidentOperationCancellationDisposition, ResidentOperationCancellationRequest,
    ResidentOperationObservationRequest, ResidentResponseAdmission,
    ResidentResponseAdmissionFailure,
};

use super::*;

struct ScriptedCarrier {
    sent: Arc<Mutex<Vec<serde_json::Value>>>,
    responses: VecDeque<Result<Box<[u8]>, AuthenticatedResidentCarrierFailure>>,
}

struct SingleCarrierSource {
    carrier: Option<Box<dyn AuthenticatedResidentCarrier>>,
}

impl AuthenticatedResidentCarrierSource for SingleCarrierSource {
    fn open(
        &mut self,
        _deadline: Instant,
    ) -> Result<Box<dyn AuthenticatedResidentCarrier>, AuthenticatedResidentCarrierSourceFailure>
    {
        self.carrier
            .take()
            .ok_or(AuthenticatedResidentCarrierSourceFailure)
    }
}

fn carrier_source(
    carrier: impl AuthenticatedResidentCarrier + 'static,
) -> Box<dyn AuthenticatedResidentCarrierSource> {
    Box::new(SingleCarrierSource {
        carrier: Some(Box::new(carrier)),
    })
}

impl AuthenticatedResidentCarrier for ScriptedCarrier {
    fn exchange(
        &mut self,
        request: &[u8],
        deadline: Instant,
    ) -> Result<Box<[u8]>, AuthenticatedResidentCarrierFailure> {
        assert!(Instant::now() < deadline);
        self.sent
            .lock()
            .expect("resident send log remains available")
            .push(serde_json::from_slice(request).expect("resident request frame is JSON"));
        self.responses.pop_front().unwrap_or(Err(
            AuthenticatedResidentCarrierFailure::ConnectionUnavailable,
        ))
    }
}

struct AcceptEverySignedResponse;

impl ResidentResponseAdmission for AcceptEverySignedResponse {
    fn admit(
        &self,
        _operation_handle_record_ref: &str,
        _operation_handle_canonical_bytes: &[u8],
        _operation_observation_record_ref: &str,
        _operation_observation_canonical_bytes: &[u8],
        deadline: Instant,
    ) -> Result<(), ResidentResponseAdmissionFailure> {
        assert!(Instant::now() < deadline);
        Ok(())
    }
}

struct ExactApplicationInvokeAdmission {
    records: serde_json::Value,
}

#[test]
fn hook_dispatch_uses_the_supplied_attachment_and_preserves_blocking() {
    for (response, expected_code, expected_effect) in [
        (
            br#"{"kind":"handler_unavailable"}"#.as_slice(),
            "resident_hook_handler_unavailable",
            pale_sdk::DiagnosticEffectCertainty::NotStarted,
        ),
        (
            br#"{"failure":"admission_denied","kind":"rejected"}"#.as_slice(),
            "resident_hook_admission_denied",
            pale_sdk::DiagnosticEffectCertainty::NotStarted,
        ),
        (
            br#"{"failure":"resident_unavailable","kind":"rejected"}"#.as_slice(),
            "resident_hook_recovery_unavailable",
            pale_sdk::DiagnosticEffectCertainty::OutcomeUnknown,
        ),
    ] {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let attachment = ResidentAttachment::provision_operations(
            carrier_source(ScriptedCarrier {
                sent: sent.clone(),
                responses: VecDeque::from([Ok(response.to_vec().into_boxed_slice())]),
            }),
            Duration::from_secs(1),
            Arc::new(AcceptEverySignedResponse),
        )
        .unwrap();
        let dispatcher =
            crate::command::CommandDispatcher::with_resident_attachment(attachment).unwrap();
        let native =
            b"{\n\"hook_event_name\":\"SessionStart\",\"session_id\":\"native-session\"\n}";
        let error = dispatcher
            .dispatch(CommandRequest::ResidentHook(
                crate::command::ResidentHookRequest::new(
                    "session-start".into(),
                    "workspace-a".into(),
                    "repository-a".into(),
                    "binding-a".into(),
                    "0".repeat(64),
                    native.to_vec(),
                ),
            ))
            .unwrap_err();
        assert_eq!(error.diagnostic().code(), expected_code);
        assert_eq!(error.diagnostic().effect(), expected_effect);
        let sent = sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0]["nativeEvent"].as_str().unwrap().as_bytes(), native);
    }
}

impl ResidentResponseAdmission for ExactApplicationInvokeAdmission {
    fn admit(
        &self,
        operation_handle_record_ref: &str,
        operation_handle_canonical_bytes: &[u8],
        operation_observation_record_ref: &str,
        operation_observation_canonical_bytes: &[u8],
        deadline: Instant,
    ) -> Result<(), ResidentResponseAdmissionFailure> {
        assert!(Instant::now() < deadline);
        let string = |name: &str| {
            self.records[name]
                .as_str()
                .unwrap_or_else(|| panic!("application fixture record member {name} is a string"))
        };
        if operation_handle_record_ref == string("operationHandleRecordRef")
            && operation_handle_canonical_bytes == string("operationHandleCanonical").as_bytes()
            && operation_observation_record_ref == string("operationObservationRecordRef")
            && operation_observation_canonical_bytes
                == string("operationObservationCanonical").as_bytes()
        {
            Ok(())
        } else {
            Err(ResidentResponseAdmissionFailure)
        }
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _bytes: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "scripted output failure",
        ))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct ResidentFixture {
    handle_ref: String,
    handle_sha256: String,
    signed_request: Vec<u8>,
    observe_response: serde_json::Value,
}

fn fixture() -> ResidentFixture {
    let value: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/pale-application/tests/fixtures/resident/application-observe-sdk.v1.json"
    )))
    .expect("application Observe fixture parses");
    let string = |name: &str| {
        value[name]
            .as_str()
            .unwrap_or_else(|| panic!("fixture member {name} is a string"))
            .to_owned()
    };
    let handle_ref = string("operation_handle_record_ref");
    let handle_sha256 = string("operation_handle_sha256");
    ResidentFixture {
        handle_ref: handle_ref.clone(),
        handle_sha256,
        signed_request: serde_json::to_vec(&value["signed_common_request"])
            .expect("signed request serializes canonically"),
        observe_response: serde_json::json!({
            "kind": "observed",
            "operationHandleRecordRef": handle_ref,
            "operationHandleCanonical": serde_json::to_string(&value["operation_handle_record"])
                .expect("operation handle serializes canonically"),
            "operationObservationRecordRef": value["operation_observation_record_ref"],
            "operationObservationCanonical":
                serde_json::to_string(&value["operation_observation_record"])
                    .expect("operation observation serializes canonically"),
        }),
    }
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

fn bytes(value: serde_json::Value) -> Result<Box<[u8]>, AuthenticatedResidentCarrierFailure> {
    Ok(serde_json::to_vec(&value)
        .expect("scripted resident response serializes")
        .into_boxed_slice())
}

fn locator(fixture: &ResidentFixture) -> OperationHandleLocator {
    OperationHandleLocator::new(fixture.handle_ref.clone(), fixture.handle_sha256.clone())
}

#[test]
fn dispatcher_projects_every_resident_request_through_one_injected_attachment() {
    let fixture = fixture();
    let cancel_frame = application_cancel_fixture()["request_frame"].clone();
    let cancel_locator = cancel_frame["locator"].clone();
    let cancel_request = cancel_frame["signedCommonRequest"]
        .as_str()
        .expect("Cancel fixture contains canonical signed request")
        .as_bytes()
        .to_vec();
    let invoke_advance = application_invoke_advance_fixture()["request_frame"].clone();
    let invoke_locator = invoke_advance["locator"].clone();
    let invoke_request = invoke_advance["signedCommonRequest"]
        .as_str()
        .expect("AdvanceInvoke fixture contains canonical signed request")
        .as_bytes()
        .to_vec();
    let delegate_start = application_delegate_fixture()["request_frame"].clone();
    let delegate_parent = delegate_start["parent"].clone();
    let delegate_start_request = delegate_start["signedCommonRequest"]
        .as_str()
        .expect("StartDelegate fixture contains canonical signed request")
        .as_bytes()
        .to_vec();
    let delegate_advance = application_delegate_advance_fixture()["request_frame"].clone();
    let delegate_locator = delegate_advance["locator"].clone();
    let delegate_advance_request = delegate_advance["signedCommonRequest"]
        .as_str()
        .expect("AdvanceDelegate fixture contains canonical signed request")
        .as_bytes()
        .to_vec();
    let located = |value: &serde_json::Value| {
        OperationHandleLocator::new(
            value["operationHandleRecordRef"]
                .as_str()
                .expect("application fixture locator reference is a string")
                .to_owned(),
            value["operationHandleSha256"]
                .as_str()
                .expect("application fixture locator digest is a string")
                .to_owned(),
        )
    };
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([
                bytes(fixture.observe_response.clone()),
                bytes(serde_json::json!({"kind":"rejected","failure":"invalid_request"})),
                bytes(serde_json::json!({"kind":"pending","locator":invoke_locator.clone()})),
                bytes(serde_json::json!({"kind":"rejected","failure":"invalid_request"})),
                bytes(serde_json::json!({"kind":"pending","locator":invoke_locator.clone()})),
                bytes(serde_json::json!({"kind":"rejected","failure":"invalid_request"})),
                bytes(serde_json::json!({
                    "kind":"unsupported",
                    "locator":delegate_locator,
                    "reason":"delegate_route_not_selected"
                })),
                bytes(serde_json::json!({
                    "kind":"cancellation_disposition",
                    "locator":cancel_locator,
                    "disposition":"terminal_already_established"
                })),
            ]),
        }),
        Duration::from_secs(1),
        Arc::new(AcceptEverySignedResponse),
    )
    .expect("positive timeout provisions the attachment");
    let dispatcher = CommandDispatcher::with_resident_attachment(attachment)
        .expect("dispatcher verifies embedded contracts");

    assert!(matches!(
        dispatcher
            .dispatch(CommandRequest::ResidentOperationObservation(
                ResidentOperationObservationRequest::new(
                    locator(&fixture),
                    fixture.signed_request.clone(),
                )
            ))
            .expect("Observe crosses the injected attachment"),
        CommandOutcome::ResidentOperationObservation(_)
    ));

    let start_invoke = dispatcher
        .dispatch(CommandRequest::ResidentInvokeStart(
            ResidentInvokeStartRequest::new(fixture.signed_request.clone()),
        ))
        .expect_err("the scripted resident rejects Invoke start");
    assert_eq!(
        start_invoke.diagnostic().code(),
        "resident_operation_request_rejected"
    );
    assert!(matches!(
        dispatcher
            .dispatch(CommandRequest::ResidentInvokeAdvance(
                ResidentInvokeAdvanceRequest::new(located(&invoke_locator), invoke_request.clone(),)
            ))
            .expect("Invoke continuation remains pending"),
        CommandOutcome::ResidentInvokeAdvanced(ResidentInvokeAdvanceOutcome::Pending { .. })
    ));

    let start_consult = dispatcher
        .dispatch(CommandRequest::ResidentConsultStart(
            ResidentConsultStartRequest::new(fixture.signed_request.clone()),
        ))
        .expect_err("the scripted resident rejects Consult start");
    assert_eq!(
        start_consult.diagnostic().code(),
        "resident_operation_request_rejected"
    );
    assert!(matches!(
        dispatcher
            .dispatch(CommandRequest::ResidentConsultAdvance(
                ResidentConsultAdvanceRequest::new(located(&invoke_locator), invoke_request,)
            ))
            .expect("Consult continuation remains pending"),
        CommandOutcome::ResidentConsultAdvanced(ResidentConsultAdvanceOutcome::Pending { .. })
    ));

    let start_delegate = dispatcher
        .dispatch(CommandRequest::ResidentDelegateStart(
            ResidentDelegateStartRequest::new(located(&delegate_parent), delegate_start_request),
        ))
        .expect_err("the scripted resident rejects Delegate start");
    assert_eq!(
        start_delegate.diagnostic().code(),
        "resident_operation_request_rejected"
    );
    assert!(matches!(
        dispatcher
            .dispatch(CommandRequest::ResidentDelegateAdvance(
                ResidentDelegateAdvanceRequest::new(
                    located(&delegate_advance["locator"]),
                    delegate_advance_request,
                )
            ))
            .expect("Delegate route absence remains typed"),
        CommandOutcome::ResidentDelegateAdvanced(
            ResidentDelegateAdvanceOutcome::Unsupported { .. }
        )
    ));
    let cancellation = dispatcher
        .dispatch(CommandRequest::ResidentOperationCancellation(
            ResidentOperationCancellationRequest::new(located(&cancel_locator), cancel_request),
        ))
        .expect("cancellation acknowledgement remains typed");
    let CommandOutcome::ResidentOperationCancellation(cancellation) = cancellation else {
        panic!("cancellation returned an unrelated outcome")
    };
    assert_eq!(
        cancellation.disposition(),
        ResidentOperationCancellationDisposition::TerminalAlreadyEstablished
    );

    let sent = sent.lock().expect("resident send log remains available");
    assert_eq!(sent.len(), 8);
    assert_eq!(sent[0]["operationHandleRecordRef"], fixture.handle_ref);
    assert_eq!(sent[7], cancel_frame);
    assert_eq!(
        sent[1..]
            .iter()
            .map(|frame| frame["kind"].as_str().expect("frame kind is a string"))
            .collect::<Vec<_>>(),
        [
            "start_invoke",
            "advance_invoke",
            "start_consult",
            "advance_consult",
            "start_delegate",
            "advance_delegate",
            "cancel",
        ]
    );
}

#[test]
fn application_produced_invoke_start_crosses_the_dispatcher_without_claiming_live_resident_assembly(
) {
    let fixture = application_invoke_fixture();
    let request_frame = fixture["request_frame"].clone();
    let response_frame = fixture["response_frame"].clone();
    assert_eq!(request_frame["kind"], "start_invoke");
    assert_eq!(response_frame["kind"], "started");
    let records = response_frame["records"].clone();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([bytes(response_frame)]),
        }),
        Duration::from_secs(1),
        Arc::new(ExactApplicationInvokeAdmission {
            records: records.clone(),
        }),
    )
    .expect("positive timeout provisions the attachment");
    let dispatcher = CommandDispatcher::with_resident_attachment(attachment)
        .expect("dispatcher verifies embedded contracts");
    let signed_common_request = request_frame["signedCommonRequest"]
        .as_str()
        .expect("StartInvoke frame contains canonical signed request")
        .as_bytes()
        .to_vec();

    let outcome = dispatcher
        .dispatch(CommandRequest::ResidentInvokeStart(
            ResidentInvokeStartRequest::new(signed_common_request),
        ))
        .expect("dispatcher projects the application-produced Started response");
    let CommandOutcome::ResidentInvokeStarted(started) = outcome else {
        panic!("Invoke start returned an unrelated command outcome")
    };

    assert_eq!(
        started.operation_handle_record_ref(),
        records["operationHandleRecordRef"]
            .as_str()
            .expect("handle record reference is a string")
    );
    assert_eq!(
        started.operation_handle_canonical_bytes(),
        records["operationHandleCanonical"]
            .as_str()
            .expect("handle record is canonical JSON")
            .as_bytes()
    );
    assert_eq!(
        started.operation_observation_record_ref(),
        records["operationObservationRecordRef"]
            .as_str()
            .expect("observation record reference is a string")
    );
    assert_eq!(
        started.operation_observation_canonical_bytes(),
        records["operationObservationCanonical"]
            .as_str()
            .expect("observation record is canonical JSON")
            .as_bytes()
    );
    let (projected, completion, subject, obligation) =
        match project_outcome(CommandOutcome::ResidentInvokeStarted(started)) {
            Ok(projected) => projected,
            Err(_) => panic!("CLI must project the SDK-owned resident result"),
        };
    assert_eq!(completion, ResultCompletion::NonTerminal);
    assert_eq!(subject, "resident Invoke start");
    assert_eq!(obligation, OutputObligation::EffectEstablished);
    assert_eq!(
        projected["operationHandleCanonicalJson"],
        records["operationHandleCanonical"]
    );
    assert_eq!(
        projected["operationObservationCanonicalJson"],
        records["operationObservationCanonical"]
    );
    assert!(projected.get("operationHandleCanonicalBytes").is_none());
    assert!(projected
        .get("operationObservationCanonicalBytes")
        .is_none());
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .as_slice(),
        [request_frame]
    );
}

#[test]
fn dispatcher_distinguishes_unknown_transmission_from_an_exhausted_source() {
    let fixture = application_invoke_fixture();
    let request_frame = fixture["request_frame"].clone();
    let signed_common_request = request_frame["signedCommonRequest"]
        .as_str()
        .expect("StartInvoke frame contains canonical signed request")
        .as_bytes()
        .to_vec();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([Err(AuthenticatedResidentCarrierFailure::OutcomeUnknown)]),
        }),
        Duration::from_secs(1),
        Arc::new(AcceptEverySignedResponse),
    )
    .expect("positive timeout provisions the attachment");
    let dispatcher = CommandDispatcher::with_resident_attachment(attachment)
        .expect("dispatcher verifies embedded contracts");

    let first = dispatcher
        .dispatch(CommandRequest::ResidentInvokeStart(
            ResidentInvokeStartRequest::new(signed_common_request.clone()),
        ))
        .expect_err("the first call has an unknown transmission outcome");
    let second = dispatcher
        .dispatch(CommandRequest::ResidentInvokeStart(
            ResidentInvokeStartRequest::new(signed_common_request),
        ))
        .expect_err("the single-carrier source cannot open a later connection");

    assert_eq!(first.diagnostic().code(), "resident_operation_failed");
    assert_eq!(second.diagnostic().code(), "resident_operation_not_sent");
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .len(),
        1,
        "the dispatcher does not reuse the failed carrier"
    );
}

#[test]
fn application_produced_invoke_advance_crosses_the_dispatcher_as_one_honest_pending_turn() {
    let fixture = application_invoke_advance_fixture();
    let request_frame = fixture["request_frame"].clone();
    let response_frame = fixture["response_frame"].clone();
    assert_eq!(request_frame["kind"], "advance_invoke");
    assert_eq!(response_frame["kind"], "pending");
    assert_eq!(request_frame["locator"], response_frame["locator"]);
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([bytes(response_frame)]),
        }),
        Duration::from_secs(1),
        Arc::new(AcceptEverySignedResponse),
    )
    .expect("positive timeout provisions the attachment");
    let dispatcher = CommandDispatcher::with_resident_attachment(attachment)
        .expect("dispatcher verifies embedded contracts");
    let locator = &request_frame["locator"];
    let requested_locator = OperationHandleLocator::new(
        locator["operationHandleRecordRef"]
            .as_str()
            .expect("AdvanceInvoke locator reference is a string")
            .to_owned(),
        locator["operationHandleSha256"]
            .as_str()
            .expect("AdvanceInvoke locator digest is a string")
            .to_owned(),
    );
    let signed_common_request = request_frame["signedCommonRequest"]
        .as_str()
        .expect("AdvanceInvoke frame contains a canonical signed request")
        .as_bytes()
        .to_vec();

    let outcome = dispatcher
        .dispatch(CommandRequest::ResidentInvokeAdvance(
            ResidentInvokeAdvanceRequest::new(requested_locator.clone(), signed_common_request),
        ))
        .expect("dispatcher preserves the application-owned pending continuation");
    let CommandOutcome::ResidentInvokeAdvanced(ResidentInvokeAdvanceOutcome::Pending {
        locator: projected,
    }) = outcome
    else {
        panic!("the pending application continuation must not become an invented outcome")
    };
    assert_eq!(
        projected.operation_handle_record_ref(),
        requested_locator.operation_handle_record_ref()
    );
    assert_eq!(
        projected.operation_handle_sha256(),
        requested_locator.operation_handle_sha256()
    );
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .as_slice(),
        [request_frame],
        "the CLI must not replay AdvanceInvoke",
    );
}

#[test]
fn application_produced_delegate_start_crosses_the_dispatcher_without_claiming_live_resident_assembly(
) {
    let fixture = application_delegate_fixture();
    let request_frame = fixture["request_frame"].clone();
    let response_frame = fixture["response_frame"].clone();
    assert_eq!(request_frame["kind"], "start_delegate");
    assert_eq!(response_frame["kind"], "started");
    let records = response_frame["records"].clone();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([bytes(response_frame)]),
        }),
        Duration::from_secs(1),
        Arc::new(ExactApplicationInvokeAdmission {
            records: records.clone(),
        }),
    )
    .expect("positive timeout provisions the attachment");
    let dispatcher = CommandDispatcher::with_resident_attachment(attachment)
        .expect("dispatcher verifies embedded contracts");
    let parent = &request_frame["parent"];
    let parent_locator = OperationHandleLocator::new(
        parent["operationHandleRecordRef"]
            .as_str()
            .expect("Delegate parent handle record reference is a string")
            .to_owned(),
        parent["operationHandleSha256"]
            .as_str()
            .expect("Delegate parent handle digest is a string")
            .to_owned(),
    );
    let signed_common_request = request_frame["signedCommonRequest"]
        .as_str()
        .expect("StartDelegate frame contains canonical signed request")
        .as_bytes()
        .to_vec();

    let outcome = dispatcher
        .dispatch(CommandRequest::ResidentDelegateStart(
            ResidentDelegateStartRequest::new(parent_locator, signed_common_request),
        ))
        .expect("dispatcher projects the application-produced Delegate Started response");
    let CommandOutcome::ResidentDelegateStarted(started) = outcome else {
        panic!("Delegate start returned an unrelated command outcome")
    };

    assert_eq!(
        started.operation_handle_record_ref(),
        records["operationHandleRecordRef"]
            .as_str()
            .expect("handle record reference is a string")
    );
    assert_eq!(
        started.operation_handle_canonical_bytes(),
        records["operationHandleCanonical"]
            .as_str()
            .expect("handle record is canonical JSON")
            .as_bytes()
    );
    assert_eq!(
        started.operation_observation_record_ref(),
        records["operationObservationRecordRef"]
            .as_str()
            .expect("observation record reference is a string")
    );
    assert_eq!(
        started.operation_observation_canonical_bytes(),
        records["operationObservationCanonical"]
            .as_str()
            .expect("observation record is canonical JSON")
            .as_bytes()
    );
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .as_slice(),
        [request_frame]
    );
}

#[test]
fn application_produced_delegate_advance_crosses_the_dispatcher_once() {
    let fixture = application_delegate_advance_fixture();
    let request_frame = fixture["request_frame"].clone();
    let response_frame = fixture["response_frame"].clone();
    assert_eq!(request_frame["kind"], "advance_delegate");
    assert_eq!(response_frame["kind"], "pending");
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([bytes(response_frame)]),
        }),
        Duration::from_secs(1),
        Arc::new(AcceptEverySignedResponse),
    )
    .expect("positive timeout provisions the attachment");
    let dispatcher = CommandDispatcher::with_resident_attachment(attachment)
        .expect("dispatcher verifies embedded contracts");
    let frame_locator = &request_frame["locator"];
    let locator = OperationHandleLocator::new(
        frame_locator["operationHandleRecordRef"]
            .as_str()
            .expect("Delegate handle record reference is a string")
            .to_owned(),
        frame_locator["operationHandleSha256"]
            .as_str()
            .expect("Delegate handle digest is a string")
            .to_owned(),
    );
    let signed_common_request = request_frame["signedCommonRequest"]
        .as_str()
        .expect("AdvanceDelegate frame contains canonical signed request")
        .as_bytes()
        .to_vec();

    let outcome = dispatcher
        .dispatch(CommandRequest::ResidentDelegateAdvance(
            ResidentDelegateAdvanceRequest::new(locator, signed_common_request),
        ))
        .expect("dispatcher projects the application-produced Pending response");
    let CommandOutcome::ResidentDelegateAdvanced(ResidentDelegateAdvanceOutcome::Pending {
        locator,
    }) = outcome
    else {
        panic!("AdvanceDelegate returned an unrelated command outcome")
    };
    assert_eq!(
        locator.operation_handle_record_ref(),
        frame_locator["operationHandleRecordRef"]
            .as_str()
            .expect("Delegate handle record reference is a string")
    );
    assert_eq!(
        locator.operation_handle_sha256(),
        frame_locator["operationHandleSha256"]
            .as_str()
            .expect("Delegate handle digest is a string")
    );
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .as_slice(),
        [request_frame],
        "the dispatcher must not replay AdvanceDelegate",
    );
}

#[test]
fn application_produced_cancel_crosses_the_dispatcher_without_claiming_live_resident_assembly() {
    let fixture = application_cancel_fixture();
    let request_frame = fixture["request_frame"].clone();
    let response_frame = fixture["response_frame"].clone();
    assert_eq!(request_frame["kind"], "cancel");
    assert_eq!(response_frame["kind"], "cancellation_disposition");
    assert_eq!(response_frame["disposition"], "locally_cancelled");
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([bytes(response_frame)]),
        }),
        Duration::from_secs(1),
        Arc::new(AcceptEverySignedResponse),
    )
    .expect("positive timeout provisions the attachment");
    let dispatcher = CommandDispatcher::with_resident_attachment(attachment)
        .expect("dispatcher verifies embedded contracts");
    let locator = &request_frame["locator"];
    let signed_common_request = request_frame["signedCommonRequest"]
        .as_str()
        .expect("Cancel frame contains canonical signed request")
        .as_bytes()
        .to_vec();

    let outcome = dispatcher
        .dispatch(CommandRequest::ResidentOperationCancellation(
            ResidentOperationCancellationRequest::new(
                OperationHandleLocator::new(
                    locator["operationHandleRecordRef"]
                        .as_str()
                        .expect("Cancel locator reference is a string")
                        .to_owned(),
                    locator["operationHandleSha256"]
                        .as_str()
                        .expect("Cancel locator digest is a string")
                        .to_owned(),
                ),
                signed_common_request,
            ),
        ))
        .expect("dispatcher projects the application-produced cancellation disposition");
    let CommandOutcome::ResidentOperationCancellation(cancelled) = outcome else {
        panic!("Cancel returned an unrelated command outcome")
    };

    assert_eq!(
        cancelled.locator().operation_handle_record_ref(),
        locator["operationHandleRecordRef"]
            .as_str()
            .expect("Cancel locator reference is a string")
    );
    assert_eq!(
        cancelled.locator().operation_handle_sha256(),
        locator["operationHandleSha256"]
            .as_str()
            .expect("Cancel locator digest is a string")
    );
    assert_eq!(
        cancelled.disposition(),
        ResidentOperationCancellationDisposition::LocallyCancelled
    );
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .as_slice(),
        [request_frame]
    );
}

#[test]
fn observe_requires_output_while_effect_capable_results_are_best_effort() {
    let fixture = fixture();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([bytes(fixture.observe_response.clone())]),
        }),
        Duration::from_secs(1),
        Arc::new(AcceptEverySignedResponse),
    )
    .expect("positive timeout provisions the attachment");
    let observe_dispatcher = CommandDispatcher::with_resident_attachment(attachment)
        .expect("dispatcher verifies embedded contracts");
    let observe_request =
        tempfile::NamedTempFile::new().expect("temporary request file is created");
    std::fs::write(observe_request.path(), &fixture.signed_request)
        .expect("signed request fixture is written");
    let observe_arguments = || ResidentLocatedRequestArgs {
        operation_handle_record_ref: fixture.handle_ref.clone(),
        operation_handle_sha256: fixture.handle_sha256.clone(),
        signed_request: observe_request.path().to_owned(),
    };

    let observe_error = run_with_writer(
        &observe_dispatcher,
        &ResidentCommand::Observe(observe_arguments()),
        ResultFormatArg::Json,
        &mut FailingWriter,
    )
    .expect_err("Observe is incomplete when its required output cannot be written");
    let DispatchFailure::Host(observe_error) = observe_error else {
        panic!("Observe output remains a CLI-owned process failure")
    };
    assert!(observe_error.to_string().contains("write command output"));

    let advance_fixture = application_invoke_advance_fixture();
    let request_frame = &advance_fixture["request_frame"];
    let response_frame = advance_fixture["response_frame"].clone();
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([bytes(response_frame)]),
        }),
        Duration::from_secs(1),
        Arc::new(AcceptEverySignedResponse),
    )
    .expect("positive timeout provisions the attachment");
    let advance_dispatcher = CommandDispatcher::with_resident_attachment(attachment)
        .expect("dispatcher verifies embedded contracts");
    let advance_request =
        tempfile::NamedTempFile::new().expect("temporary advance request file is created");
    std::fs::write(
        advance_request.path(),
        request_frame["signedCommonRequest"]
            .as_str()
            .expect("AdvanceInvoke frame contains a canonical signed request"),
    )
    .expect("AdvanceInvoke request fixture is written");
    let locator = &request_frame["locator"];
    let advance_arguments = ResidentLocatedRequestArgs {
        operation_handle_record_ref: locator["operationHandleRecordRef"]
            .as_str()
            .expect("AdvanceInvoke locator reference is a string")
            .to_owned(),
        operation_handle_sha256: locator["operationHandleSha256"]
            .as_str()
            .expect("AdvanceInvoke locator digest is a string")
            .to_owned(),
        signed_request: advance_request.path().to_owned(),
    };
    let effect_output = run_with_writer(
        &advance_dispatcher,
        &ResidentCommand::Invoke {
            step: ResidentInvokeCommand::Advance(advance_arguments),
        },
        ResultFormatArg::Json,
        &mut FailingWriter,
    );
    match effect_output {
        Ok(()) => {}
        Err(DispatchFailure::Api(error)) => panic!(
            "terminal output cannot reclassify a durable operation result: {}",
            error.diagnostic().code()
        ),
        Err(DispatchFailure::Admitted(diagnostic)) => panic!(
            "terminal output cannot reclassify a durable operation result: {}",
            diagnostic.code()
        ),
        Err(DispatchFailure::Host(error)) => {
            panic!("terminal output cannot reclassify a durable operation result: {error}")
        }
        Err(DispatchFailure::ApiWithReconciliation { error, .. }) => panic!(
            "terminal output cannot reclassify a durable operation result: {}",
            error.diagnostic().code()
        ),
    }
    assert_eq!(
        sent.lock()
            .expect("resident send log remains available")
            .len(),
        2
    );
}

#[test]
fn signed_request_limit_is_sdk_owned_and_checked_before_carrier_exchange() {
    let fixture = fixture();
    let exact = tempfile::NamedTempFile::new().expect("exact request file is created");
    std::fs::write(
        exact.path(),
        vec![b'x'; pale_sdk::RESIDENT_RECORD_BYTE_LIMIT],
    )
    .expect("exact-limit request fixture is written");
    let exact_bytes = match read_signed_request(exact.path()) {
        Ok(bytes) => bytes,
        Err(_) => panic!("the exact SDK-owned limit was rejected by CLI input"),
    };
    assert_eq!(exact_bytes.len(), pale_sdk::RESIDENT_RECORD_BYTE_LIMIT);

    let oversized = tempfile::NamedTempFile::new().expect("oversized request file is created");
    std::fs::write(
        oversized.path(),
        vec![b'x'; pale_sdk::RESIDENT_RECORD_BYTE_LIMIT + 1],
    )
    .expect("oversized request fixture is written");
    let error = read_signed_request(oversized.path()).expect_err("the CLI bound is fail-closed");
    let DispatchFailure::Host(error) = error else {
        panic!("host input overflow remains a CLI-owned failure")
    };
    assert!(error.to_string().contains("input limit"));

    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_observation(
        carrier_source(ScriptedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::new(),
        }),
        Duration::from_secs(1),
        Arc::new(AcceptEverySignedResponse),
    )
    .expect("positive timeout provisions the attachment");
    let dispatcher = CommandDispatcher::with_resident_attachment(attachment)
        .expect("dispatcher verifies embedded contracts");
    for path in [exact.path(), oversized.path()] {
        let command = ResidentCommand::Observe(ResidentLocatedRequestArgs {
            operation_handle_record_ref: fixture.handle_ref.clone(),
            operation_handle_sha256: fixture.handle_sha256.clone(),
            signed_request: path.to_owned(),
        });
        run_with_writer(
            &dispatcher,
            &command,
            ResultFormatArg::Json,
            &mut Vec::new(),
        )
        .expect_err("invalid request bytes stop before resident exchange");
    }
    assert!(
        sent.lock()
            .expect("resident send log remains available")
            .is_empty(),
        "neither an invalid exact-limit request nor an oversized request reaches the carrier"
    );
}
