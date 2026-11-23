//! The event and credential must come from the same single carrier read.

use pale_core::records::{ArtifactDigest, LifecycleEvent};
use pale_ports::{LocalIpcPeerCredentials, ResidentHookFrameRequest};

use super::*;

struct Frame {
    bytes: Option<Box<[u8]>>,
    key_after_read: Option<[u8; 32]>,
    reads: usize,
}

impl LocalIpcFramePort for Frame {
    fn authenticated_caller_key(&self) -> Option<[u8; 32]> {
        (self.reads == 1).then_some(self.key_after_read).flatten()
    }

    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        panic!("kernel credentials cannot replace frame authentication")
    }

    fn receive_frame(&mut self, _: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        self.reads += 1;
        Ok(self.bytes.take().expect("one frame read"))
    }

    fn send_frame(&mut self, _: &[u8], _: Instant) -> Result<(), LocalIpcCarrierError> {
        panic!("admission must not emit a response")
    }
}

fn wire(payload: &str) -> ResidentHookFrameRequest {
    ResidentHookFrameRequest::new(
        [
            "hook".into(),
            LifecycleEvent::PreToolUse.as_str().into(),
            "--workspace".into(),
            "ws.main".into(),
            "--repository".into(),
            "repo.main".into(),
            "--provider-binding".into(),
            "terminal-provider".into(),
            "--provider-binding-identity".into(),
            "a".repeat(64),
        ],
        payload.into(),
    )
}

fn receive(
    bytes: &[u8],
    key: Option<[u8; 32]>,
) -> Result<AuthenticatedResidentHookRequest, ResidentHookFrameAdmissionError> {
    AuthenticatedResidentHookRequest::receive(
        &mut Frame {
            bytes: Some(bytes.into()),
            key_after_read: key,
            reads: 0,
        },
        Instant::now(),
    )
}

#[test]
fn captures_exact_bytes_and_key_after_one_read() {
    let payload = "{ \"hook_event_name\":\"PreToolUse\",\"session_id\":\"first\" }";
    let bytes = canon::to_canonical_string(&wire(payload)).unwrap();
    let mut carrier = Frame {
        bytes: Some(bytes.into_bytes().into_boxed_slice()),
        key_after_read: Some([7; 32]),
        reads: 0,
    };
    let captured = AuthenticatedResidentHookRequest::receive(&mut carrier, Instant::now()).unwrap();
    carrier.key_after_read = Some([9; 32]);
    carrier.bytes = Some(
        canon::to_canonical_string(&wire(
            r#"{"hook_event_name":"PreToolUse","session_id":"substitute"}"#,
        ))
        .unwrap()
        .into_bytes()
        .into_boxed_slice(),
    );
    assert_eq!(carrier.reads, 1);
    assert_eq!(captured.caller_key, [7; 32]);
    assert_eq!(captured.request.event().session_id(), Some("first"));
    assert_eq!(
        captured.request.native_event_sha256(),
        &ArtifactDigest::hash_bytes(payload.as_bytes())
    );
    assert!(!format!("{captured:?}").contains("first"));
}

#[test]
fn rejects_unauthenticated_malformed_and_substituted_frames() {
    let valid = canon::to_canonical_string(&wire(r#"{"hook_event_name":"PreToolUse"}"#)).unwrap();
    assert!(matches!(
        receive(valid.as_bytes(), None),
        Err(ResidentHookFrameAdmissionError::Unauthenticated)
    ));
    let value: serde_json::Value = serde_json::from_str(&valid).unwrap();
    let mut wrong_kind = value.clone();
    wrong_kind["kind"] = "start_invoke".into();
    let mut extra = value.clone();
    extra["signedCommonRequest"] = "{}".into();
    let mut wrong_route = value;
    wrong_route["arguments"][0] = "observe".into();
    let invalid = [
        String::new(),
        format!(" {valid}"),
        valid.replacen('{', "{\"kind\":\"hook\",", 1),
        canon::to_canonical_string(&wrong_kind).unwrap(),
        canon::to_canonical_string(&extra).unwrap(),
        canon::to_canonical_string(&wrong_route).unwrap(),
        canon::to_canonical_string(&wire(r#"{"hook_event_name":"PostToolUse"}"#)).unwrap(),
        canon::to_canonical_string(&wire(
            r#"{"hook_event_name":"PreToolUse","session_id":"a","session_id":"b"}"#,
        ))
        .unwrap(),
        "x".repeat(crate::resident::process_host::MAX_RESIDENT_REQUEST_FRAME_BYTES + 1),
    ];
    for bytes in invalid {
        assert!(matches!(
            receive(bytes.as_bytes(), Some([7; 32])),
            Err(ResidentHookFrameAdmissionError::InvalidFrame)
        ));
    }
}
