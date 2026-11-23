//! Application-owned `CancelTask` response phases over a deterministic carrier.

use super::*;
use crate::{
    local_operation_start_fixture::record_ref,
    resident::{
        operation_observation_custody::tests::{
            a2a_exchange::{limits, prepared, route},
            dispatch_custody::complete,
            OperationFixture,
        },
        OperationCustodyDatabaseEvidence,
    },
};
use pale_adapter_a2a::A2aCancelTaskDecoder;
use pale_core::records::{
    A2aRemoteContextIdV1, A2aRemoteTaskIdV1, A2aRemoteTaskStateV1, A2aRouteAdmissionDigest,
    A2aTaskBindingDigest, A2aTaskObservationDigest, ArtifactDigest, ImmutableRecordReferenceV1,
    OperationHandleDigest, OperationLeaseDigest, OperationObservationDigest, UnixTimeV1,
};
use pale_ports::{
    A2aExchangeEvidence, A2aExchangeFacts, ControlRecordCoordinate, WallClockSourceError,
    WallClockSourcePort,
};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    task::{Wake, Waker},
};

enum Step {
    Pending,
    Item(A2aExchangeEvidence),
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
            Step::Eof => Poll::Ready(None),
        }
    }

    fn cancel(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
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
    observed_at: UnixTimeV1,
}

impl Clock {
    fn at(observed_at: u64) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            observed_at: UnixTimeV1::new(observed_at).unwrap(),
        }
    }
}

impl WallClockSourcePort for Clock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.observed_at)
    }
}

fn cancel_request() -> Vec<u8> {
    br#"{"jsonrpc":"2.0","id":"cancel-1","method":"CancelTask","params":{"id":"task-1"}}"#.to_vec()
}

fn task(state: A2aRemoteTaskStateV1) -> A2aExchangeEvidence {
    A2aExchangeEvidence {
        facts: A2aExchangeFacts::Task {
            task: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
            context: A2aRemoteContextIdV1::parse("context-1").unwrap(),
            state,
        },
        bytes: br#"{"jsonrpc":"2.0","id":"cancel-1","result":{"id":"task-1","contextId":"context-1","status":{"state":"TASK_STATE_CANCELED"}}}"#
            .to_vec(),
        sse: None,
    }
}

fn basis() -> KnownA2aTaskBasis {
    KnownA2aTaskBasis {
        operation: (
            record_ref("cancel/operation"),
            OperationHandleDigest::parse("1".repeat(64)).unwrap(),
        ),
        request: ControlRecordCoordinate::new(
            record_ref("cancel/operation-request"),
            pale_ports::ControlRecordDigest::parse("7".repeat(64)).unwrap(),
        ),
        operation_lease: (
            record_ref("cancel/operation-lease"),
            OperationLeaseDigest::parse("8".repeat(64)).unwrap(),
        ),
        selection: ControlRecordCoordinate::new(
            record_ref("cancel/attempt-selection"),
            pale_ports::ControlRecordDigest::parse("9".repeat(64)).unwrap(),
        ),
        dispatch: (
            record_ref("cancel/dispatch"),
            OperationObservationDigest::parse("2".repeat(64)).unwrap(),
        ),
        local_head: (
            record_ref("cancel/local-head"),
            OperationObservationDigest::parse("3".repeat(64)).unwrap(),
        ),
        route: (
            record_ref("cancel/route"),
            A2aRouteAdmissionDigest::parse("4".repeat(64)).unwrap(),
        ),
        binding: (
            record_ref("cancel/binding"),
            A2aTaskBindingDigest::parse("5".repeat(64)).unwrap(),
        ),
        remote_head: (
            record_ref("cancel/remote-head"),
            A2aTaskObservationDigest::parse("6".repeat(64)).unwrap(),
        ),
        authorized_at: UnixTimeV1::new(20).unwrap(),
        earliest_observation_at: UnixTimeV1::new(20).unwrap(),
        retain_until: UnixTimeV1::new(40).unwrap(),
    }
}

fn started_phase(
    steps: impl IntoIterator<Item = Step>,
    maximum_evidence_bytes: usize,
) -> (InFlightA2aCancelTask, Arc<AtomicBool>) {
    let operation = OperationFixture::exact();
    let provider = crate::provider_binding_fixtures::provider_binding(
        crate::composition_source_fixtures::participant("a2a-participant"),
    );
    let route = route(&provider, &operation);
    let prepared = prepared(&operation, &provider, &route, "client", "0.0.1", limits());
    let request = cancel_request();
    let request_evidence = OperationA2aEvidence::new(
        ImmutableRecordReferenceV1 {
            record_ref: record_ref("cancel/request"),
            record_sha256: ArtifactDigest::hash_bytes(&request),
        },
        request,
        None,
        maximum_evidence_bytes,
    )
    .unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancellation = operation.successor();
    let custody = complete(&operation, None, OperationCustodyDatabaseEvidence::New);
    let expected = prepared.coordinate().clone();
    (
        InFlightA2aCancelTask {
            exchange: Box::new(FakeExchange {
                coordinate: expected.clone(),
                steps: steps.into_iter().collect(),
                cancelled: Arc::clone(&cancelled),
            }),
            custody,
            context: A2aCancelTaskResponseContext {
                basis: basis(),
                request_evidence,
                response_record_ref: record_ref("cancel/response"),
                additional_response_record_ref: record_ref("cancel/additional-response"),
                expected_task: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
                expected_context: A2aRemoteContextIdV1::parse("context-1").unwrap(),
                request_id: A2aJsonRpcRequestIdV1::parse("cancel-1").unwrap(),
                cancellation: (cancellation.0.clone(), cancellation.1.clone()),
                maximum_evidence_bytes,
                start_authorized_at: UnixTimeV1::new(20).unwrap(),
            },
            expected,
        },
        cancelled,
    )
}

#[test]
fn pending_cancel_readback_retains_one_exchange_until_task_and_eof() {
    let (in_flight, cancelled) = started_phase(
        [
            Step::Pending,
            Step::Item(task(A2aRemoteTaskStateV1::Canceled)),
            Step::Pending,
            Step::Eof,
        ],
        4096,
    );
    let mut cx = context();
    let in_flight = match in_flight.poll_response(&mut cx, &A2aCancelTaskDecoder) {
        A2aCancelTaskPoll::Pending(value) => value,
        A2aCancelTaskPoll::Ready(_) => panic!("the response can remain pending"),
    };
    let received = match in_flight.poll_response(&mut cx, &A2aCancelTaskDecoder) {
        A2aCancelTaskPoll::Ready(Ok(value)) => value,
        _ => panic!("the exact Task must advance to EOF admission"),
    };
    let clock = Clock::at(30);
    let received = match received.poll_eof(&mut cx, &clock) {
        A2aCancelTaskPoll::Pending(value) => value,
        A2aCancelTaskPoll::Ready(_) => panic!("EOF can remain pending"),
    };
    assert_eq!(clock.calls.load(Ordering::SeqCst), 0);
    let completed = match received.poll_eof(&mut cx, &clock) {
        A2aCancelTaskPoll::Ready(Ok(value)) => value,
        _ => panic!("unary EOF must complete the read-back"),
    };
    assert_eq!(completed.remote_state(), &A2aRemoteTaskStateV1::Canceled);
    assert_eq!(clock.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        completed.evidence().0.coordinate().record_ref().as_str(),
        "cancel/request"
    );
    assert_eq!(
        completed.evidence().1.coordinate().record_ref().as_str(),
        "cancel/response"
    );
    assert!(!cancelled.load(Ordering::SeqCst));
}

#[test]
fn local_stop_and_non_unary_response_become_non_replayable_uncertainty() {
    let (in_flight, cancelled) = started_phase([Step::Pending], 4096);
    let outcome = in_flight.cancel();
    assert_eq!(
        outcome.reason(),
        A2aCancelTaskUncertainty::LocalReadCancelled
    );
    assert!(outcome.response_evidence().is_empty());
    assert!(cancelled.load(Ordering::SeqCst));

    let (in_flight, cancelled) = started_phase(
        [
            Step::Item(task(A2aRemoteTaskStateV1::Canceled)),
            Step::Item(task(A2aRemoteTaskStateV1::Canceled)),
        ],
        4096,
    );
    let mut cx = context();
    let received = match in_flight.poll_response(&mut cx, &A2aCancelTaskDecoder) {
        A2aCancelTaskPoll::Ready(Ok(value)) => value,
        _ => panic!("the first exact Task must be retained"),
    };
    let outcome = match received.poll_eof(&mut cx, &Clock::at(30)) {
        A2aCancelTaskPoll::Ready(Err(value)) => value,
        _ => panic!("a second Task violates unary framing"),
    };
    assert_eq!(
        outcome.reason(),
        A2aCancelTaskUncertainty::UnexpectedResponse
    );
    assert_eq!(outcome.response_evidence().len(), 2);
    assert!(cancelled.load(Ordering::SeqCst));
}

#[test]
fn protocol_or_aggregate_bound_failure_preserves_outcome_uncertainty() {
    let request_bytes = cancel_request().len();
    let (in_flight, cancelled) = started_phase(
        [Step::Item(task(A2aRemoteTaskStateV1::Canceled))],
        request_bytes,
    );
    let mut cx = context();
    let outcome = match in_flight.poll_response(&mut cx, &A2aCancelTaskDecoder) {
        A2aCancelTaskPoll::Ready(Err(value)) => value,
        _ => panic!("the aggregate evidence ceiling must close the response"),
    };
    assert_eq!(outcome.reason(), A2aCancelTaskUncertainty::EvidenceBound);
    assert!(outcome.response_evidence().is_empty());
    assert!(cancelled.load(Ordering::SeqCst));

    let mut wrong = task(A2aRemoteTaskStateV1::Working);
    wrong.bytes = task(A2aRemoteTaskStateV1::Canceled).bytes;
    let (in_flight, cancelled) = started_phase([Step::Item(wrong)], 4096);
    let outcome = match in_flight.poll_response(&mut cx, &A2aCancelTaskDecoder) {
        A2aCancelTaskPoll::Ready(Err(value)) => value,
        _ => panic!("decoded bytes and carrier facts must agree"),
    };
    assert_eq!(
        outcome.reason(),
        A2aCancelTaskUncertainty::ProtocolQualification
    );
    assert_eq!(outcome.response_evidence().len(), 1);
    assert!(cancelled.load(Ordering::SeqCst));
}
