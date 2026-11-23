//! Application-owned unary read-back phases over a deterministic fake carrier.

use super::*;
use crate::{
    local_operation_start_fixture::record_ref,
    resident::operation_observation_custody::tests::{
        a2a_exchange::{limits, prepared, route},
        OperationFixture,
    },
};
use pale_core::records::{
    A2aRemoteContextIdV1, A2aRemoteTaskIdV1, A2aRemoteTaskStateV1, A2aRouteAdmissionDigest,
    A2aTaskBindingDigest, A2aTaskObservationDigest, ArtifactDigest, ImmutableRecordReferenceV1,
    OperationHandleDigest, OperationLeaseDigest, OperationObservationDigest, UnixTimeV1,
};
use pale_ports::{
    A2aExchangeEvidence, A2aExchangeFacts, A2aExchangePort, ControlRecordCoordinate,
    OperationA2aEvidence, WallClockSourceError, WallClockSourcePort,
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    task::{Wake, Waker},
};

type A2aGetTaskReadbackBasis = KnownA2aTaskBasis;

enum Step {
    Pending,
    Item(A2aExchangeEvidence),
    Error(A2aExchangeError),
    Eof,
}

struct FakeExchange {
    coordinate: A2aExchangeCoordinate,
    steps: VecDeque<Step>,
    cancelled: Arc<AtomicBool>,
}

impl A2aExchange for FakeExchange {
    fn coordinate(&self) -> &A2aExchangeCoordinate {
        &self.coordinate
    }

    fn poll_next(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<A2aExchangeEvidence, A2aExchangeError>>> {
        match self.steps.pop_front().unwrap_or(Step::Eof) {
            Step::Pending => Poll::Pending,
            Step::Item(item) => Poll::Ready(Some(Ok(item))),
            Step::Error(error) => Poll::Ready(Some(Err(error))),
            Step::Eof => Poll::Ready(None),
        }
    }

    fn cancel(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

struct FakePort {
    starts: Cell<usize>,
    steps: RefCell<Option<VecDeque<Step>>>,
    substitute: RefCell<Option<A2aExchangeCoordinate>>,
    cancelled: Arc<AtomicBool>,
}

impl FakePort {
    fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            starts: Cell::new(0),
            steps: RefCell::new(Some(steps.into_iter().collect())),
            substitute: RefCell::new(None),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl A2aExchangePort for FakePort {
    fn prepare(&self, _: A2aExchangeRequest) -> Result<A2aPreparedExchange, A2aExchangeError> {
        panic!("the phase test starts bytes prepared by the real constructor")
    }

    fn start(
        &self,
        prepared: A2aPreparedExchange,
    ) -> Result<Box<dyn A2aExchange>, A2aExchangeError> {
        self.starts.set(self.starts.get() + 1);
        let coordinate = self
            .substitute
            .borrow_mut()
            .take()
            .unwrap_or_else(|| prepared.coordinate().clone());
        Ok(Box::new(FakeExchange {
            coordinate,
            steps: self.steps.borrow_mut().take().unwrap_or_default(),
            cancelled: Arc::clone(&self.cancelled),
        }))
    }
}

struct NoopWake;
impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn context() -> Context<'static> {
    let waker = Box::leak(Box::new(Waker::from(Arc::new(NoopWake))));
    Context::from_waker(waker)
}

struct Clock {
    calls: AtomicUsize,
    observed_at: Option<UnixTimeV1>,
}

impl Clock {
    fn at(observed_at: u64) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            observed_at: Some(UnixTimeV1::new(observed_at).unwrap()),
        }
    }

    fn unavailable() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            observed_at: None,
        }
    }
}

impl WallClockSourcePort for Clock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.observed_at.ok_or_else(|| {
            WallClockSourceError::from_source(std::io::Error::other("test clock unavailable"))
        })
    }
}

fn task(state: A2aRemoteTaskStateV1) -> A2aExchangeEvidence {
    A2aExchangeEvidence {
        facts: A2aExchangeFacts::Task {
            task: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
            context: A2aRemoteContextIdV1::parse("context-1").unwrap(),
            state,
        },
        bytes: br#"{"jsonrpc":"2.0","id":"get-1","result":{"id":"task-1","contextId":"context-1","status":{"state":"TASK_STATE_WORKING"}}}"#.to_vec(),
        sse: None,
    }
}

fn prepared_with_limit(maximum_evidence_bytes: usize) -> PreparedA2aGetTaskReadback {
    let operation = OperationFixture::exact();
    let provider = crate::provider_binding_fixtures::provider_binding(
        crate::composition_source_fixtures::participant("a2a-participant"),
    );
    let route = route(&provider, &operation);
    let prepared = prepared(&operation, &provider, &route, "client", "0.0.1", limits());
    let request_evidence = OperationA2aEvidence::new(
        ImmutableRecordReferenceV1 {
            record_ref: record_ref("readback/request"),
            record_sha256: ArtifactDigest::hash_bytes(prepared.bytes()),
        },
        prepared.bytes().to_vec(),
        None,
        maximum_evidence_bytes,
    )
    .unwrap();
    PreparedA2aGetTaskReadback {
        basis: A2aGetTaskReadbackBasis {
            operation: (
                record_ref("readback/operation"),
                OperationHandleDigest::parse("1".repeat(64)).unwrap(),
            ),
            request: ControlRecordCoordinate::new(
                record_ref("readback/operation-request"),
                pale_ports::ControlRecordDigest::parse("7".repeat(64)).unwrap(),
            ),
            operation_lease: (
                record_ref("readback/operation-lease"),
                OperationLeaseDigest::parse("8".repeat(64)).unwrap(),
            ),
            selection: ControlRecordCoordinate::new(
                record_ref("readback/attempt-selection"),
                pale_ports::ControlRecordDigest::parse("9".repeat(64)).unwrap(),
            ),
            dispatch: (
                record_ref("readback/dispatch"),
                OperationObservationDigest::parse("2".repeat(64)).unwrap(),
            ),
            local_head: (
                record_ref("readback/local-head"),
                OperationObservationDigest::parse("3".repeat(64)).unwrap(),
            ),
            route: (
                record_ref("readback/route"),
                A2aRouteAdmissionDigest::parse("4".repeat(64)).unwrap(),
            ),
            binding: (
                record_ref("readback/binding"),
                A2aTaskBindingDigest::parse("5".repeat(64)).unwrap(),
            ),
            remote_head: (
                record_ref("readback/remote-head"),
                A2aTaskObservationDigest::parse("6".repeat(64)).unwrap(),
            ),
            authorized_at: UnixTimeV1::new(20).unwrap(),
            earliest_observation_at: UnixTimeV1::new(20).unwrap(),
            retain_until: UnixTimeV1::new(40).unwrap(),
        },
        prepared,
        request_evidence,
        response_record_ref: record_ref("readback/response"),
        expected_task: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
        expected_context: A2aRemoteContextIdV1::parse("context-1").unwrap(),
        maximum_evidence_bytes,
    }
}

#[test]
fn pending_phases_retain_one_started_exchange_until_task_and_eof() {
    let port = FakePort::new([
        Step::Pending,
        Step::Item(task(A2aRemoteTaskStateV1::Working)),
        Step::Pending,
        Step::Eof,
    ]);
    let in_flight = prepared_with_limit(4096).start(&port).unwrap();
    assert_eq!(port.starts.get(), 1);
    let mut cx = context();
    let in_flight = match in_flight.poll_response(&mut cx) {
        A2aGetTaskReadbackPoll::Pending(value) => value,
        A2aGetTaskReadbackPoll::Ready(_) => panic!("first poll must remain pending"),
    };
    let received = match in_flight.poll_response(&mut cx) {
        A2aGetTaskReadbackPoll::Ready(Ok(value)) => value,
        _ => panic!("second poll must yield the one Task"),
    };
    let clock = Clock::at(30);
    let received = match received.poll_eof(&mut cx, &clock) {
        A2aGetTaskReadbackPoll::Pending(value) => value,
        A2aGetTaskReadbackPoll::Ready(_) => panic!("EOF can remain pending"),
    };
    assert_eq!(
        clock.calls.load(Ordering::SeqCst),
        0,
        "pending must not sample wall time"
    );
    let completed = match received.poll_eof(&mut cx, &clock) {
        A2aGetTaskReadbackPoll::Ready(Ok(value)) => value,
        _ => panic!("the next poll must close the unary exchange"),
    };
    assert_eq!(port.starts.get(), 1);
    assert_eq!(completed.remote_state(), &A2aRemoteTaskStateV1::Working);
    assert_eq!(completed.observed_at(), UnixTimeV1::new(30).unwrap());
    assert_eq!(clock.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        completed.evidence().0.coordinate().record_ref().as_str(),
        "readback/request"
    );
    assert_eq!(
        completed.evidence().1.coordinate().record_ref().as_str(),
        "readback/response"
    );
}

#[test]
fn local_cancellation_closes_response_and_eof_reads_without_remote_cancel() {
    let response_port = FakePort::new([Step::Pending]);
    prepared_with_limit(4096)
        .start(&response_port)
        .expect("started response read")
        .cancel();
    assert!(response_port.cancelled.load(Ordering::SeqCst));
    assert_eq!(response_port.starts.get(), 1);

    let eof_port = FakePort::new([Step::Item(task(A2aRemoteTaskStateV1::Working))]);
    let mut cx = context();
    let received = match prepared_with_limit(4096)
        .start(&eof_port)
        .expect("started EOF read")
        .poll_response(&mut cx)
    {
        A2aGetTaskReadbackPoll::Ready(Ok(received)) => received,
        _ => panic!("matching response must enter EOF phase"),
    };
    received.cancel();
    assert!(eof_port.cancelled.load(Ordering::SeqCst));
    assert_eq!(eof_port.starts.get(), 1);
}

#[test]
fn missing_unlike_sse_and_extra_responses_close_without_observation() {
    let mut unlike = task(A2aRemoteTaskStateV1::Working);
    unlike.facts = A2aExchangeFacts::DirectMessage;
    let mut sse = task(A2aRemoteTaskStateV1::Working);
    sse.sse = Some(("event-1".into(), "message".into()));
    for steps in [
        vec![Step::Eof],
        vec![Step::Item(unlike)],
        vec![Step::Item(sse)],
    ] {
        let port = FakePort::new(steps);
        let mut cx = context();
        let result = prepared_with_limit(4096)
            .start(&port)
            .unwrap()
            .poll_response(&mut cx);
        assert!(matches!(
            result,
            A2aGetTaskReadbackPoll::Ready(Err(A2aGetTaskReadbackError::Response))
        ));
    }
    let port = FakePort::new([
        Step::Item(task(A2aRemoteTaskStateV1::Working)),
        Step::Item(task(A2aRemoteTaskStateV1::Working)),
    ]);
    let mut cx = context();
    let received = match prepared_with_limit(4096)
        .start(&port)
        .unwrap()
        .poll_response(&mut cx)
    {
        A2aGetTaskReadbackPoll::Ready(Ok(value)) => value,
        _ => panic!("first Task must reach the EOF phase"),
    };
    assert!(matches!(
        received.poll_eof(&mut cx, &Clock::at(30)),
        A2aGetTaskReadbackPoll::Ready(Err(A2aGetTaskReadbackError::Response))
    ));
    assert!(port.cancelled.load(Ordering::SeqCst));
}

#[test]
fn start_coordinate_substitution_and_post_response_error_are_closed() {
    let operation = OperationFixture::exact();
    let provider = crate::provider_binding_fixtures::provider_binding(
        crate::composition_source_fixtures::participant("a2a-participant"),
    );
    let route = route(&provider, &operation);
    let substitute = prepared(
        &operation,
        &provider,
        &route,
        "different-client",
        "0.0.1",
        limits(),
    )
    .coordinate()
    .clone();
    let port = FakePort::new([]);
    *port.substitute.borrow_mut() = Some(substitute);
    assert!(matches!(
        prepared_with_limit(4096).start(&port),
        Err(A2aGetTaskReadbackError::Association)
    ));
    assert!(port.cancelled.load(Ordering::SeqCst));

    let port = FakePort::new([
        Step::Item(task(A2aRemoteTaskStateV1::Working)),
        Step::Error(A2aExchangeError::Connection),
    ]);
    let mut cx = context();
    let received = match prepared_with_limit(4096)
        .start(&port)
        .unwrap()
        .poll_response(&mut cx)
    {
        A2aGetTaskReadbackPoll::Ready(Ok(value)) => value,
        _ => panic!("first Task must be retained before EOF"),
    };
    assert!(matches!(
        received.poll_eof(&mut cx, &Clock::at(30)),
        A2aGetTaskReadbackPoll::Ready(Err(A2aGetTaskReadbackError::Response))
    ));
    assert_eq!(port.starts.get(), 1);
}

#[test]
fn eof_time_cannot_backdate_a_response_past_retention() {
    let port = FakePort::new([Step::Item(task(A2aRemoteTaskStateV1::Working)), Step::Eof]);
    let mut cx = context();
    let received = match prepared_with_limit(4096)
        .start(&port)
        .unwrap()
        .poll_response(&mut cx)
    {
        A2aGetTaskReadbackPoll::Ready(Ok(value)) => value,
        _ => panic!("the matching Task must reach EOF admission"),
    };
    assert!(matches!(
        received.poll_eof(&mut cx, &Clock::at(41)),
        A2aGetTaskReadbackPoll::Ready(Err(A2aGetTaskReadbackError::Time))
    ));
    assert_eq!(port.starts.get(), 1);
}

#[test]
fn eof_clock_failure_retains_a_typed_failure_without_fabricating_time() {
    let port = FakePort::new([Step::Item(task(A2aRemoteTaskStateV1::Working)), Step::Eof]);
    let mut cx = context();
    let received = match prepared_with_limit(4096)
        .start(&port)
        .unwrap()
        .poll_response(&mut cx)
    {
        A2aGetTaskReadbackPoll::Ready(Ok(value)) => value,
        _ => panic!("the matching Task must reach EOF admission"),
    };
    let clock = Clock::unavailable();
    assert!(matches!(
        received.poll_eof(&mut cx, &clock),
        A2aGetTaskReadbackPoll::Ready(Err(A2aGetTaskReadbackError::Clock(_)))
    ));
    assert_eq!(clock.calls.load(Ordering::SeqCst), 1);
    assert_eq!(port.starts.get(), 1);
}

#[test]
fn aggregate_evidence_bound_rejects_response_without_restart() {
    let request_bytes = prepared_with_limit(4096).request_evidence.byte_len();
    let port = FakePort::new([Step::Item(task(A2aRemoteTaskStateV1::Working))]);
    let mut cx = context();
    let result = prepared_with_limit(request_bytes)
        .start(&port)
        .unwrap()
        .poll_response(&mut cx);
    assert!(matches!(
        result,
        A2aGetTaskReadbackPoll::Ready(Err(A2aGetTaskReadbackError::Evidence(_)))
    ));
    assert_eq!(port.starts.get(), 1);
}
