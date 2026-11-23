//! Resident frame ingress tests.

use std::cell::Cell;

use pale_ports::ResidentOperationFrameLocator;

use super::*;

struct CountingPort {
    inbound: Option<Box<[u8]>>,
    reads: Cell<usize>,
    writes: Cell<usize>,
    fail_send: bool,
    verified_key: Option<[u8; 32]>,
}

impl CountingPort {
    fn new(bytes: impl Into<Box<[u8]>>) -> Self {
        Self {
            inbound: Some(bytes.into()),
            reads: Cell::new(0),
            writes: Cell::new(0),
            fail_send: false,
            verified_key: None,
        }
    }

    fn with_failed_send(mut self) -> Self {
        self.fail_send = true;
        self
    }
}

impl LocalIpcFramePort for CountingPort {
    fn authenticated_caller_key(&self) -> Option<[u8; 32]> {
        if self.reads.get() == 1 {
            self.verified_key
        } else {
            None
        }
    }

    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        Ok(LocalIpcPeerCredentials::new(501, 20, Some(8_081)))
    }

    fn send_frame(&mut self, _: &[u8], _: Instant) -> Result<(), LocalIpcCarrierError> {
        self.writes.set(self.writes.get() + 1);
        if self.fail_send {
            Err(LocalIpcCarrierError::io(
                LocalIpcCarrierOperation::SendFrame,
                io::Error::other("injected send failure"),
            ))
        } else {
            Ok(())
        }
    }

    fn receive_frame(&mut self, _: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        self.reads.set(self.reads.get() + 1);
        self.inbound
            .take()
            .ok_or_else(|| closed_ingress_error(LocalIpcCarrierOperation::ReceiveFrame))
    }
}

fn canonical(value: &impl serde::Serialize) -> Box<[u8]> {
    canon::to_canonical_string(value)
        .expect("frame serializes canonically")
        .into_bytes()
        .into_boxed_slice()
}

fn locator() -> ResidentOperationFrameLocator {
    ResidentOperationFrameLocator::new("operations/handle-1".to_owned(), "a".repeat(64))
}

fn assert_ingress_rejects(bytes: &[u8]) {
    let mut carrier = CountingPort::new(bytes.to_vec().into_boxed_slice());
    assert!(matches!(
        PrefetchedResidentFrame::receive(&mut carrier, Instant::now()),
        Err(ResidentFrameIngressError::Frame)
    ));
    assert_eq!(carrier.reads.get(), 1);
    assert_eq!(carrier.writes.get(), 0);
}

#[test]
fn every_operation_kind_selects_only_the_operation_owner() {
    let requests = [
        ResidentOperationFrameRequest::StartInvoke {
            signed_common_request: "{}".to_owned(),
        },
        ResidentOperationFrameRequest::AdvanceInvoke {
            locator: locator(),
            signed_common_request: "{}".to_owned(),
        },
        ResidentOperationFrameRequest::StartConsult {
            signed_common_request: "{}".to_owned(),
        },
        ResidentOperationFrameRequest::AdvanceConsult {
            locator: locator(),
            signed_common_request: "{}".to_owned(),
        },
        ResidentOperationFrameRequest::StartDelegate {
            parent: locator(),
            signed_common_request: "{}".to_owned(),
        },
        ResidentOperationFrameRequest::AdvanceDelegate {
            locator: locator(),
            signed_common_request: "{}".to_owned(),
        },
        ResidentOperationFrameRequest::Cancel {
            locator: locator(),
            signed_common_request: "{}".to_owned(),
        },
    ];

    for request in requests {
        assert_eq!(
            classify(&canonical(&request)).expect("current operation frame classifies"),
            ResidentRequestFrameKind::Operation
        );
    }
}

#[test]
fn observe_selects_only_the_observe_owner() {
    let request = ResidentObserveFrameRequest::new(
        "operations/handle-1".to_owned(),
        "a".repeat(64),
        "{}".to_owned(),
    );
    assert_eq!(
        classify(&canonical(&request)).expect("Observe frame classifies"),
        ResidentRequestFrameKind::Observe
    );
}

#[test]
fn cross_kind_and_malformed_frames_have_no_fallback() {
    let cross_kind = br#"{"kind":"start_invoke","operationHandleRecordRef":"operations/handle-1","operationHandleSha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","signedCommonRequest":"{}"}"#;
    for bytes in [
        cross_kind.as_slice(),
        br#"{"kind":"unknown","signedCommonRequest":"{}"}"#,
        br#" {"operationHandleRecordRef":"operations/handle-1","operationHandleSha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","signedCommonRequest":"{}"}"#,
        br#"{"signedCommonRequest":"{}"}"#,
        b"",
    ] {
        assert_ingress_rejects(bytes);
    }
    let oversized = vec![b'x'; super::super::MAX_RESIDENT_REQUEST_FRAME_BYTES + 1];
    assert_ingress_rejects(&oversized);
}

#[test]
fn ingress_reads_once_and_prefetched_port_writes_once() {
    let request = ResidentObserveFrameRequest::new(
        "operations/handle-1".to_owned(),
        "a".repeat(64),
        "{}".to_owned(),
    );
    let mut carrier = CountingPort::new(canonical(&request));
    let deadline = Instant::now()
        .checked_add(std::time::Duration::from_secs(1))
        .expect("test deadline");
    let frame = PrefetchedResidentFrame::receive(&mut carrier, deadline)
        .expect("one physical frame is retained");
    assert_eq!(carrier.reads.get(), 1);
    assert_eq!(carrier.writes.get(), 0);

    let mut port = frame.port(&mut carrier);
    assert!(!port
        .receive_frame(deadline)
        .expect("the existing host receives the retained frame")
        .is_empty());
    assert!(port.receive_frame(deadline).is_err());
    port.send_frame(b"{}", deadline)
        .expect("one response is forwarded");
    assert!(port.send_frame(b"{}", deadline).is_err());
    drop(port);

    assert_eq!(carrier.reads.get(), 1);
    assert_eq!(carrier.writes.get(), 1);
}

#[test]
fn failed_response_attempt_is_not_retried() {
    let request = ResidentObserveFrameRequest::new(
        "operations/handle-1".to_owned(),
        "a".repeat(64),
        "{}".to_owned(),
    );
    let mut carrier = CountingPort::new(canonical(&request)).with_failed_send();
    let deadline = Instant::now();
    let frame = PrefetchedResidentFrame::receive(&mut carrier, deadline)
        .expect("one physical frame is retained");
    let mut port = frame.port(&mut carrier);
    assert!(port.send_frame(b"{}", deadline).is_err());
    assert!(port.send_frame(b"{}", deadline).is_err());
    drop(port);

    assert_eq!(carrier.reads.get(), 1);
    assert_eq!(carrier.writes.get(), 1);
}

#[test]
fn prefetched_frame_keeps_the_key_proved_for_its_received_bytes() {
    let request =
        ResidentObserveFrameRequest::new("operations/handle-1".into(), "a".repeat(64), "{}".into());
    let mut carrier = CountingPort::new(canonical(&request));
    carrier.verified_key = Some([7; 32]);
    assert_eq!(carrier.authenticated_caller_key(), None);
    let deadline = Instant::now();
    let frame = PrefetchedResidentFrame::receive(&mut carrier, deadline).unwrap();
    carrier.verified_key = Some([8; 32]);
    let mut port = frame.port(&mut carrier);
    assert_eq!(port.authenticated_caller_key(), Some([7; 32]));
    port.receive_frame(deadline).unwrap();
    assert_eq!(port.authenticated_caller_key(), Some([7; 32]));
}
