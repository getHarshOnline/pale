//! Actual prepare/start/pull tests against an owned loopback TLS peer.

use pale_adapter_a2a::{A2aService, A2aServiceLimits, A2aServiceState, A2aTlsTrust};
use pale_ports::{A2aExchangeFacts, A2aExchangePort};
use std::{
    sync::Arc,
    task::{Wake, Waker},
    thread,
    time::Instant,
};

use super::*;

#[path = "server.rs"]
mod server;
use server::{Reply, Server, CA, OTHER_CA};

struct ThreadWake(thread::Thread);
impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

fn next(exchange: &mut dyn A2aExchange) -> Option<Result<A2aExchangeEvidence, A2aExchangeError>> {
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        if let Poll::Ready(value) = exchange.poll_next(&mut context) {
            return value;
        }
        assert!(Instant::now() < deadline, "exchange polling watchdog");
        thread::park_timeout(Duration::from_millis(5));
    }
}

fn service(certificate: &[u8], exchanges: usize) -> A2aService {
    let mut service = A2aService::spawn(
        A2aServiceLimits {
            exchanges: bound(exchanges),
            dns_jobs: bound(1),
            dns_addresses: bound(4),
            exchange: limits(),
            certificate_bytes: bound(4096),
            certificates: bound(2),
        },
        A2aTlsTrust::CertificatesOnly(vec![certificate.to_vec()]),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match service.state() {
            A2aServiceState::Running => return service,
            A2aServiceState::Starting => {}
            state => panic!("unexpected startup state: {state:?}"),
        }
        assert!(Instant::now() < deadline, "service startup watchdog");
        thread::park_timeout(Duration::from_millis(2));
    }
}

fn finish(service: &mut A2aService) {
    service.begin_shutdown();
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        match service.state() {
            A2aServiceState::Stopped(result) => {
                result.unwrap();
                return;
            }
            A2aServiceState::Draining { .. } => {}
            state => panic!("unexpected shutdown state: {state:?}"),
        }
        assert!(Instant::now() < deadline, "service drain watchdog");
        thread::park_timeout(Duration::from_millis(2));
    }
}

fn input(endpoint: &str, streaming: bool, ceilings: A2aExchangeLimits) -> A2aExchangeRequest {
    let operation = OperationFixture::exact();
    let provider = crate::provider_binding_fixtures::provider_binding(
        crate::composition_source_fixtures::participant("a2a-participant"),
    );
    let route = route_at(&provider, &operation, endpoint, streaming);
    request(
        &operation,
        &provider,
        &route,
        "client",
        "0.0.1",
        ceilings,
        if streaming {
            A2aTaskRequestModeV1::SendStreamingMessage
        } else {
            A2aTaskRequestModeV1::SendMessage
        },
    )
}

fn task_response(id: &str) -> Vec<u8> {
    serde_json::to_vec(
        &serde_json::json!({"jsonrpc":"2.0","id":id,"result":{"task":{
            "id":"task-1", "contextId":"context-1", "status":{"state":"TASK_STATE_WORKING"}
        }}}),
    )
    .unwrap()
}

fn response(status: &str, headers: &str, body: &[u8]) -> Reply {
    let mut bytes = format!(
        "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n",
        body.len()
    )
    .into_bytes();
    bytes.extend_from_slice(body);
    Reply::Bytes(bytes)
}

#[test]
fn actual_carrier_sends_exact_prepared_bytes_and_echoes_the_coordinate() {
    let body = task_response("rpc-1");
    let mut server = Server::start(
        response("200 OK", "Content-Type: application/json\r\n", &body),
        false,
    );
    let mut service = service(CA, 1);
    let prepared = service
        .prepare(input(&server.endpoint(), false, limits()))
        .unwrap();
    let expected_bytes = prepared.bytes().to_vec();
    let expected_coordinate = prepared.coordinate().clone();
    let mut exchange = service.start(prepared).unwrap();
    assert!(exchange.coordinate() == &expected_coordinate);
    let evidence = next(exchange.as_mut()).unwrap().unwrap();
    assert_eq!(evidence.bytes, body);
    // Feed the actual encoder/decoder buffers into the exact-byte custody
    // input boundary. This proves no checkpoint or production send admission.
    for (reference, bytes) in [
        ("carrier/request", &expected_bytes),
        ("carrier/response", &evidence.bytes),
    ] {
        let retained = pale_ports::OperationA2aEvidence::new(
            ImmutableRecordReferenceV1 {
                record_ref: record_ref(reference),
                record_sha256: ArtifactDigest::hash_bytes(bytes),
            },
            bytes.clone(),
            None,
            4096,
        )
        .unwrap();
        assert_eq!(retained.bytes(), bytes);
        assert_eq!(retained.sse(), None);
    }
    assert!(
        matches!(evidence.facts, A2aExchangeFacts::Task { task, context, state: A2aRemoteTaskStateV1::Working }
        if task.as_str() == "task-1" && context.as_str() == "context-1")
    );
    assert!(next(exchange.as_mut()).is_none());
    drop(exchange);
    finish(&mut service);
    server.finish();
    assert_eq!(server.accepted(), 1);
    let captured = server.requests();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].body, expected_bytes);
    assert_eq!(
        &ArtifactDigest::hash_bytes(&captured[0].body),
        expected_coordinate.request_sha256()
    );
    let header = captured[0].head.to_ascii_lowercase();
    assert!(header.starts_with("post /rpc?selected=one http/1.1\r\n"));
    assert!(header.contains("\r\na2a-version: 1.0\r\n"));
    assert!(header.contains("\r\ncontent-type: application/json\r\n"));
    let value: serde_json::Value = serde_json::from_slice(&captured[0].body).unwrap();
    assert_eq!(value["id"], "rpc-1");
    assert_eq!(value["method"], "SendMessage");
}

#[test]
fn forged_prepared_body_is_rejected_before_a_connection() {
    let mut server = Server::start(Reply::LostAfterBody, false);
    let mut service = service(CA, 1);
    let prepared = service
        .prepare(input(&server.endpoint(), false, limits()))
        .unwrap();
    let (request, coordinate, mut bytes) = prepared.into_parts();
    bytes.push(b' ');
    let substituted = A2aPreparedExchange::from_encoded(
        request,
        bytes,
        coordinate.tls_trust_input_sha256().clone(),
    )
    .unwrap();
    assert!(matches!(
        service.start(substituted),
        Err(A2aExchangeError::CoordinateMismatch)
    ));
    finish(&mut service);
    server.finish();
    assert_eq!(server.accepted(), 0);
}

#[test]
fn wrong_peer_or_response_identity_never_yields_task_evidence() {
    for (trust, wrong_host, id, expected) in [
        (OTHER_CA, false, "rpc-1", A2aExchangeError::Connection),
        (CA, true, "rpc-1", A2aExchangeError::Connection),
        (
            CA,
            false,
            "another-request",
            A2aExchangeError::CoordinateMismatch,
        ),
    ] {
        let mut server = Server::start(
            response(
                "200 OK",
                "Content-Type: application/json\r\n",
                &task_response(id),
            ),
            wrong_host,
        );
        let mut service = service(trust, 1);
        let prepared = service
            .prepare(input(&server.endpoint(), false, limits()))
            .unwrap();
        let mut exchange = service.start(prepared).unwrap();
        assert!(matches!(next(exchange.as_mut()), Some(Err(error)) if error == expected));
        assert!(next(exchange.as_mut()).is_none());
        drop(exchange);
        finish(&mut service);
        server.finish();
        assert_eq!(server.accepted(), 1);
        if wrong_host || trust == OTHER_CA {
            assert!(server.requests().is_empty());
        }
    }
}

#[test]
fn lost_send_and_retry_statuses_do_not_repeat_the_request() {
    for reply in [
        Reply::LostAfterBody,
        response(
            "429 Too Many Requests",
            "Retry-After: 0\r\nContent-Type: application/json\r\n",
            b"{}",
        ),
        response(
            "503 Service Unavailable",
            "Retry-After: 0\r\nContent-Type: application/json\r\n",
            b"{}",
        ),
    ] {
        let mut server = Server::start(reply, false);
        let mut service = service(CA, 1);
        let prepared = service
            .prepare(input(&server.endpoint(), false, limits()))
            .unwrap();
        let mut exchange = service.start(prepared).unwrap();
        assert!(matches!(next(exchange.as_mut()), Some(Err(_))));
        drop(exchange);
        finish(&mut service);
        server.finish();
        assert_eq!(server.accepted(), 1);
        assert_eq!(server.requests().len(), 1);
    }
}

#[test]
fn redirect_is_not_followed_and_compression_is_not_accepted() {
    let mut target = Server::start(Reply::LostAfterBody, false);
    for reply in [
        response(
            "307 Temporary Redirect",
            &format!("Location: {}\r\n", target.endpoint()),
            b"",
        ),
        response(
            "200 OK",
            "Content-Encoding: gzip\r\nContent-Type: application/json\r\n",
            &task_response("rpc-1"),
        ),
    ] {
        let mut server = Server::start(reply, false);
        let mut service = service(CA, 1);
        let prepared = service
            .prepare(input(&server.endpoint(), false, limits()))
            .unwrap();
        let mut exchange = service.start(prepared).unwrap();
        assert!(matches!(
            next(exchange.as_mut()),
            Some(Err(A2aExchangeError::HttpResponse))
        ));
        drop(exchange);
        finish(&mut service);
        server.finish();
        assert_eq!(server.requests().len(), 1);
    }
    target.finish();
    assert_eq!(target.accepted(), 0);
}

#[test]
fn completed_reader_retains_its_slot_until_released() {
    let mut server = Server::start(
        response(
            "200 OK",
            "Content-Type: application/json\r\n",
            &task_response("rpc-1"),
        ),
        false,
    );
    let mut service = service(CA, 1);
    let mut exchange = service
        .start(
            service
                .prepare(input(&server.endpoint(), false, limits()))
                .unwrap(),
        )
        .unwrap();
    assert!(next(exchange.as_mut()).unwrap().is_ok());
    assert!(next(exchange.as_mut()).is_none());
    let prepared = service
        .prepare(input(&server.endpoint(), false, limits()))
        .unwrap();
    assert!(matches!(
        service.start(prepared),
        Err(A2aExchangeError::LimitExceeded)
    ));
    drop(exchange);
    let mut replacement = service
        .start(
            service
                .prepare(input(&server.endpoint(), false, limits()))
                .unwrap(),
        )
        .unwrap();
    assert!(next(replacement.as_mut()).unwrap().is_ok());
    assert!(next(replacement.as_mut()).is_none());
    drop(replacement);
    finish(&mut service);
    server.finish();
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn local_cancellation_and_shutdown_do_not_send_cancel_task() {
    for shutdown in [false, true] {
        let mut server = Server::start(Reply::Stall, false);
        let mut service = service(CA, 1);
        let mut exchange = service
            .start(
                service
                    .prepare(input(&server.endpoint(), false, limits()))
                    .unwrap(),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while server.requests().is_empty() {
            assert!(
                Instant::now() < deadline,
                "server must observe send before cancellation"
            );
            thread::park_timeout(Duration::from_millis(2));
        }
        if shutdown {
            service.begin_shutdown();
        } else {
            exchange.cancel();
        }
        if shutdown {
            let prepared = service
                .prepare(input(&server.endpoint(), false, limits()))
                .unwrap();
            assert!(matches!(
                service.start(prepared),
                Err(A2aExchangeError::ServiceUnavailable)
            ));
        }
        assert!(matches!(
            next(exchange.as_mut()),
            Some(Err(A2aExchangeError::Cancelled))
        ));
        drop(exchange);
        finish(&mut service);
        server.finish();
        assert_eq!(server.requests().len(), 1);
        assert_eq!(server.accepted(), 1);
    }
}

#[test]
fn sse_backpressure_spends_the_total_deadline_and_keeps_exact_event_bytes() {
    let json = task_response("rpc-1");
    let mut stream = Vec::new();
    for id in 1..=4 {
        stream.extend_from_slice(
            format!(
                "id: event-{id}\nevent: update\ndata: {}\n\n",
                std::str::from_utf8(&json).unwrap()
            )
            .as_bytes(),
        );
    }
    let mut server = Server::start(
        response("200 OK", "Content-Type: text/event-stream\r\n", &stream),
        false,
    );
    let mut service = service(CA, 1);
    let mut ceilings = limits();
    ceilings.total_timeout = Duration::from_millis(500);
    ceilings.connect_timeout = Duration::from_millis(400);
    ceilings.read_timeout = Duration::from_millis(400);
    let prepared = service
        .prepare(input(&server.endpoint(), true, ceilings))
        .unwrap();
    let began = Instant::now();
    let mut exchange = service.start(prepared).unwrap();
    while server.responses() == 0 {
        assert!(
            began.elapsed() < ceilings.connect_timeout,
            "SSE response write watchdog"
        );
        thread::park_timeout(Duration::from_millis(2));
    }
    // Deliberately do not drain the two-item queue until the actual total budget
    // has expired; this is the deadline under test, not an arbitrary startup wait.
    while began.elapsed() < ceilings.total_timeout + Duration::from_millis(200) {
        thread::park_timeout(Duration::from_millis(5));
    }
    for id in 1..=2 {
        let evidence = next(exchange.as_mut()).unwrap().unwrap();
        assert_eq!(evidence.bytes, json);
        assert_eq!(
            evidence.sse,
            Some((format!("event-{id}"), "update".to_owned()))
        );
    }
    assert!(matches!(
        next(exchange.as_mut()),
        Some(Err(A2aExchangeError::Timeout))
    ));
    assert!(next(exchange.as_mut()).is_none());
    drop(exchange);
    finish(&mut service);
    server.finish();
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn response_headers_and_streamed_byte_limits_are_enforced_by_the_carrier() {
    let mut chunked = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n801\r\n".to_vec();
    chunked.extend_from_slice(&vec![b' '; 2049]);
    chunked.extend_from_slice(b"\r\n0\r\n\r\n");
    for (reply, expected) in [
        (Reply::Bytes(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2049\r\nConnection: close\r\n\r\n".to_vec()), A2aExchangeError::HttpResponse),
        (response("200 OK", "Content-Type: text/plain\r\n", &task_response("rpc-1")), A2aExchangeError::HttpResponse),
        (response("200 OK", "Content-Type: application/json\r\nContent-Type: application/json\r\n", &task_response("rpc-1")), A2aExchangeError::HttpResponse),
        (Reply::Bytes(chunked), A2aExchangeError::LimitExceeded),
    ] {
        let mut server = Server::start(reply, false);
        let mut service = service(CA, 1);
        let mut exchange = service.start(service.prepare(input(&server.endpoint(), false, limits())).unwrap()).unwrap();
        assert!(matches!(next(exchange.as_mut()), Some(Err(error)) if error == expected));
        assert!(next(exchange.as_mut()).is_none());
        drop(exchange);
        finish(&mut service);
        server.finish();
        assert_eq!(server.accepted(), 1);
        assert_eq!(server.requests().len(), 1);
    }
}

#[test]
fn rejected_sse_never_becomes_task_evidence() {
    for (stream, expected) in [
        (
            format!(
                "data: {}\n\n",
                String::from_utf8(task_response("wrong-id")).unwrap()
            ),
            A2aExchangeError::CoordinateMismatch,
        ),
        (
            format!(
                "data: {}\n",
                String::from_utf8(task_response("rpc-1")).unwrap()
            ),
            A2aExchangeError::Response,
        ),
    ] {
        let mut server = Server::start(
            response(
                "200 OK",
                "Content-Type: text/event-stream\r\n",
                stream.as_bytes(),
            ),
            false,
        );
        let mut service = service(CA, 1);
        let mut exchange = service
            .start(
                service
                    .prepare(input(&server.endpoint(), true, limits()))
                    .unwrap(),
            )
            .unwrap();
        assert!(matches!(next(exchange.as_mut()), Some(Err(error)) if error == expected));
        assert!(next(exchange.as_mut()).is_none());
        drop(exchange);
        finish(&mut service);
        server.finish();
        assert_eq!(server.requests().len(), 1);
    }
}
