//! Hook delivery preserves native bytes and blocks every unsupported response.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pale_sdk::{
    AuthenticatedResidentCarrier, AuthenticatedResidentCarrierFailure,
    AuthenticatedResidentCarrierSource, AuthenticatedResidentCarrierSourceFailure,
    DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticRetry, PaleApi, ResidentAttachment,
    ResidentHookRequest, ResidentResponseAdmission, ResidentResponseAdmissionFailure,
};

type Sent = Arc<Mutex<Vec<Vec<u8>>>>;

struct Source(Option<Carrier>);
struct Carrier {
    sent: Sent,
    response: Result<Box<[u8]>, AuthenticatedResidentCarrierFailure>,
    late: bool,
}

impl AuthenticatedResidentCarrierSource for Source {
    fn open(
        &mut self,
        _: Instant,
    ) -> Result<Box<dyn AuthenticatedResidentCarrier>, AuthenticatedResidentCarrierSourceFailure>
    {
        self.0
            .take()
            .map(|carrier| Box::new(carrier) as Box<dyn AuthenticatedResidentCarrier>)
            .ok_or(AuthenticatedResidentCarrierSourceFailure)
    }
}

impl AuthenticatedResidentCarrier for Carrier {
    fn exchange(
        &mut self,
        request: &[u8],
        deadline: Instant,
    ) -> Result<Box<[u8]>, AuthenticatedResidentCarrierFailure> {
        self.sent.lock().unwrap().push(request.to_vec());
        if self.late {
            std::thread::sleep(
                deadline.saturating_duration_since(Instant::now()) + Duration::from_millis(1),
            );
        }
        self.response.clone()
    }
}

struct NoSignedRecords;
impl ResidentResponseAdmission for NoSignedRecords {
    fn admit(
        &self,
        _: &str,
        _: &[u8],
        _: &str,
        _: &[u8],
        _: Instant,
    ) -> Result<(), ResidentResponseAdmissionFailure> {
        panic!("a blocking hook reply carries no signed operation records")
    }
}

fn attachment(
    response: Result<Box<[u8]>, AuthenticatedResidentCarrierFailure>,
    observation_only: bool,
    late: bool,
) -> (ResidentAttachment, Sent) {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let source = Box::new(Source(Some(Carrier {
        sent: sent.clone(),
        response,
        late,
    })));
    let attachment = if observation_only {
        ResidentAttachment::provision_observation(
            source,
            Duration::from_millis(100),
            Arc::new(NoSignedRecords),
        )
    } else {
        ResidentAttachment::provision_hooks(source, Duration::from_millis(100))
    }
    .unwrap();
    if !observation_only {
        assert!(!attachment.supports_observation_requests());
        assert!(!attachment.supports_operation_requests());
    }
    (attachment, sent)
}

fn request(payload: &[u8]) -> ResidentHookRequest {
    ResidentHookRequest::new(
        "session-start".into(),
        "workspace-a".into(),
        "repository-a".into(),
        "binding-a".into(),
        "0".repeat(64),
        payload.to_vec(),
    )
}

const NATIVE: &[u8] =
    b"{\n  \"session_id\": \"private-session\", \"hook_event_name\": \"SessionStart\"\n}\n";
const UNAVAILABLE: &[u8] = br#"{"kind":"handler_unavailable"}"#;

#[test]
fn admission_refusals_keep_their_reason_and_reconciliation_requirement() {
    let api = PaleApi::new().unwrap();
    for (failure, code, disposition, retry, effect) in [
        (
            "invalid_request",
            "resident_hook_request_rejected",
            DiagnosticDisposition::InvalidInput,
            DiagnosticRetry::AfterCorrection,
            DiagnosticEffectCertainty::NotStarted,
        ),
        (
            "replay_conflict",
            "resident_hook_replay_conflict",
            DiagnosticDisposition::EffectOutcomeUnknown,
            DiagnosticRetry::ReconcileFirst,
            DiagnosticEffectCertainty::OutcomeUnknown,
        ),
        (
            "admission_denied",
            "resident_hook_admission_denied",
            DiagnosticDisposition::Blocked,
            DiagnosticRetry::AfterStateChange,
            DiagnosticEffectCertainty::NotStarted,
        ),
        (
            "configuration_unavailable",
            "resident_hook_configuration_unavailable",
            DiagnosticDisposition::Blocked,
            DiagnosticRetry::AfterStateChange,
            DiagnosticEffectCertainty::NotStarted,
        ),
        (
            "resident_unavailable",
            "resident_hook_recovery_unavailable",
            DiagnosticDisposition::EffectOutcomeUnknown,
            DiagnosticRetry::ReconcileFirst,
            DiagnosticEffectCertainty::OutcomeUnknown,
        ),
    ] {
        let response = format!(r#"{{"failure":"{failure}","kind":"rejected"}}"#);
        let (attachment, sent) = attachment(Ok(response.into_bytes().into()), false, false);
        let error = api
            .deliver_resident_hook(
                Some(&attachment),
                api.admit_resident_hook(request(NATIVE)).unwrap(),
            )
            .unwrap_err();
        assert_eq!(error.diagnostic().code(), code);
        assert_eq!(error.diagnostic().disposition(), disposition);
        assert_eq!(error.diagnostic().retry(), retry);
        assert_eq!(error.diagnostic().effect(), effect);
        assert_eq!(sent.lock().unwrap().len(), 1);
        assert!(std::error::Error::source(&error).is_none());
        assert!(!format!("{error:?}").contains("private-session"));
    }
}

#[test]
fn exact_native_bytes_and_route_cross_once_but_unavailable_never_allows() {
    let (attachment, sent) = attachment(Ok(UNAVAILABLE.into()), false, false);
    let api = PaleApi::new().unwrap();
    let admission = api.admit_resident_hook(request(NATIVE)).unwrap();
    assert!(!format!("{admission:?}").contains("private-session"));
    let error = api
        .deliver_resident_hook(Some(&attachment), admission)
        .unwrap_err();
    assert_eq!(
        error.diagnostic().code(),
        "resident_hook_handler_unavailable"
    );
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::Blocked
    );
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotStarted
    );
    assert_eq!(
        error.diagnostic().retry(),
        DiagnosticRetry::AfterStateChange
    );
    let sent = sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    let frame: serde_json::Value = serde_json::from_slice(&sent[0]).unwrap();
    assert_eq!(frame["nativeEvent"].as_str().unwrap().as_bytes(), NATIVE);
    assert_eq!(frame["kind"], "hook");
    assert_eq!(
        frame["arguments"],
        serde_json::json!([
            "hook",
            "session-start",
            "--workspace",
            "workspace-a",
            "--repository",
            "repository-a",
            "--provider-binding",
            "binding-a",
            "--provider-binding-identity",
            "0".repeat(64)
        ])
    );
}

#[test]
fn absent_and_observation_only_attachments_do_not_send() {
    let (attachment, sent) = attachment(Ok(UNAVAILABLE.into()), true, false);
    let api = PaleApi::new().unwrap();
    for selected in [None, Some(&attachment)] {
        let admission = api.admit_resident_hook(request(NATIVE)).unwrap();
        let error = api.deliver_resident_hook(selected, admission).unwrap_err();
        assert_eq!(
            error.diagnostic().code(),
            "resident_hook_transport_unavailable"
        );
        assert_eq!(
            error.diagnostic().effect(),
            DiagnosticEffectCertainty::NotStarted
        );
    }
    assert!(sent.lock().unwrap().is_empty());
}

#[test]
fn malformed_unknown_duplicate_extended_and_noncanonical_replies_stay_uncertain() {
    let api = PaleApi::new().unwrap();
    let replies: &[&[u8]] = &[
        b"",
        b"{",
        br#"{"kind":"allow"}"#,
        br#"{"kind":"handler_unavailable","kind":"handler_unavailable"}"#,
        br#"{"context":"private-context","kind":"handler_unavailable"}"#,
        br#"{"failure":"unknown","kind":"rejected"}"#,
        br#"{"kind":"rejected"}"#,
        br#"{"failure":"admission_denied","kind":"rejected","private":"private-context"}"#,
        br#"{"failure":"admission_denied","failure":"invalid_request","kind":"rejected"}"#,
        b"{\"kind\":\"handler_unavailable\"}\n",
    ];
    for reply in replies {
        let (attachment, sent) = attachment(Ok((*reply).into()), false, false);
        let admission = api.admit_resident_hook(request(NATIVE)).unwrap();
        let error = api
            .deliver_resident_hook(Some(&attachment), admission)
            .unwrap_err();
        assert_eq!(error.diagnostic().code(), "resident_hook_outcome_unknown");
        assert_eq!(
            error.diagnostic().effect(),
            DiagnosticEffectCertainty::OutcomeUnknown
        );
        assert_eq!(error.diagnostic().retry(), DiagnosticRetry::ReconcileFirst);
        assert!(std::error::Error::source(&error).is_none());
        assert!(!format!("{error:?}").contains("private-context"));
        assert_eq!(sent.lock().unwrap().len(), 1);
        // A malformed reply closes the carrier; the exhausted factory cannot resend it.
        let next = api
            .deliver_resident_hook(
                Some(&attachment),
                api.admit_resident_hook(request(NATIVE)).unwrap(),
            )
            .unwrap_err();
        assert_eq!(next.diagnostic().code(), "resident_hook_not_sent");
        assert_eq!(sent.lock().unwrap().len(), 1);
    }
}

#[test]
fn carrier_certainty_is_preserved_without_automatic_retry() {
    let api = PaleApi::new().unwrap();
    for (failure, expected) in [
        (
            AuthenticatedResidentCarrierFailure::NotSent,
            DiagnosticEffectCertainty::NotStarted,
        ),
        (
            AuthenticatedResidentCarrierFailure::ConnectionUnavailable,
            DiagnosticEffectCertainty::NotStarted,
        ),
        (
            AuthenticatedResidentCarrierFailure::OutcomeUnknown,
            DiagnosticEffectCertainty::OutcomeUnknown,
        ),
    ] {
        let (attachment, sent) = attachment(Err(failure), false, false);
        let error = api
            .deliver_resident_hook(
                Some(&attachment),
                api.admit_resident_hook(request(NATIVE)).unwrap(),
            )
            .unwrap_err();
        assert_eq!(error.diagnostic().effect(), expected);
        assert_eq!(sent.lock().unwrap().len(), 1);
    }
}

#[test]
fn late_unavailable_reply_does_not_establish_a_timely_result() {
    let (attachment, sent) = attachment(Ok(UNAVAILABLE.into()), false, true);
    let api = PaleApi::new().unwrap();
    let error = api
        .deliver_resident_hook(
            Some(&attachment),
            api.admit_resident_hook(request(NATIVE)).unwrap(),
        )
        .unwrap_err();
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::OutcomeUnknown
    );
    assert_eq!(sent.lock().unwrap().len(), 1);
}

#[test]
fn native_event_that_exceeds_the_complete_wire_bound_is_not_sent() {
    let (attachment, sent) = attachment(Ok(UNAVAILABLE.into()), false, false);
    let api = PaleApi::new().unwrap();
    let payload = serde_json::to_vec(&serde_json::json!({
        "hook_event_name":"SessionStart", "session_id":"native", "extra":"a".repeat(262_144)
    }))
    .unwrap();
    let error = api
        .deliver_resident_hook(
            Some(&attachment),
            api.admit_resident_hook(request(&payload)).unwrap(),
        )
        .unwrap_err();
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotStarted
    );
    assert!(sent.lock().unwrap().is_empty());
}
