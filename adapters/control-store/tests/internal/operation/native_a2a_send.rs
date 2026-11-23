//! Delegate-to-A2A dispatch over real `SQLite` custody and an explicit carrier model.
//! The carrier does no network I/O. The test proves the application release fence,
//! not remote task creation, verified HTTPS, or production signing and anchoring.

use super::*;
use crate::resident_fixture::effect_case_lineage_fixture as request_fixture;
use crate::resident_fixture::session_admission_fixture::LocalStartContext;
use pale_application::provider_invocation::{
    ProviderInvocationAdmissionContext, ProviderInvocationSafeFailureProof,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    cell::RefCell,
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};

#[derive(Clone, Copy)]
enum ResponseModel {
    Task,
    TaskWithPendingEof,
    TaskThenExtra,
    ResponseClockUnavailable,
    ResponseClockBackward,
    ResponseClockAfterRetention,
    Extra,
    DirectMessage,
    DirectMessageAnchorUnavailable,
    DirectMessageAnchorOutcomeUnknown,
    Eof,
    StartRejected,
    RetentionTooShort,
    ProcessLostBeforeStart,
    RecoveredResponseClockUnavailable,
    RecoveredResponseClockBackward,
    PreCustodyClockUnavailable,
    ReleaseClockUnavailable,
    CheckpointRejected,
    CommitUnavailable,
    AnchorUnavailable,
}

struct TerminalAnchor<'a> {
    inner: &'a mut dyn AntiRollbackAnchorPort,
    failure: Option<AntiRollbackAnchorErrorKind>,
}

impl AntiRollbackAnchorPort for TerminalAnchor<'_> {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.inner.read(key)
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        match self.failure {
            Some(kind) => Err(AntiRollbackAnchorError::new(kind)),
            None => self.inner.compare_and_advance(transition),
        }
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.inner.observe(transition)
    }
}

#[derive(Clone, Copy)]
enum CustodyInterception {
    None,
    Preparation,
    Checkpoint,
    Evidence,
    CommitUnavailable,
}

struct InterceptingCustody<'port> {
    inner: &'port mut dyn OperationCustodyPort,
    interception: CustodyInterception,
}

impl ControlTransactionPort for InterceptingCustody<'_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        self.inner.prepare_transaction(store)
    }
}

impl OperationCustodyPort for InterceptingCustody<'_> {
    fn read_a2a_evidence(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_bytes: usize,
    ) -> Result<Option<OperationA2aEvidence>, ControlStoreError> {
        let evidence = self
            .inner
            .read_a2a_evidence(store, coordinate, maximum_bytes)?;
        if !matches!(self.interception, CustodyInterception::Evidence) {
            return Ok(evidence);
        }
        let Some(evidence) = evidence else {
            return Ok(None);
        };
        let bytes = evidence.bytes().to_vec();
        Ok(Some(
            OperationA2aEvidence::new(
                ImmutableRecordReferenceV1 {
                    record_ref: record_ref("a2a/substituted-request-evidence"),
                    record_sha256: ArtifactDigest::hash_bytes(&bytes),
                },
                bytes,
                evidence
                    .sse()
                    .map(|(id, kind)| (id.to_owned(), kind.to_owned())),
                maximum_bytes,
            )
            .unwrap(),
        ))
    }

    fn read_a2a_evidence_inserted_at(
        &mut self,
        store: &AuthorityStoreCoordinate,
        transaction: ControlTransactionSequenceV1,
        maximum_items: usize,
        maximum_bytes: usize,
    ) -> Result<Vec<OperationA2aEvidence>, ControlStoreError> {
        self.inner
            .read_a2a_evidence_inserted_at(store, transaction, maximum_items, maximum_bytes)
    }

    fn read_a2a_task_bindings(
        &mut self,
        store: &AuthorityStoreCoordinate,
        scope: &OperationA2aBindingScope,
        cursor: Option<&OperationA2aBindingCursor>,
        maximum_bindings: usize,
        maximum_body_bytes: usize,
    ) -> Result<OperationA2aBindings, ControlStoreError> {
        self.inner.read_a2a_task_bindings(
            store,
            scope,
            cursor,
            maximum_bindings,
            maximum_body_bytes,
        )
    }

    fn read_publication_sources(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&OperationPublicationCursor>,
        maximum_sources: usize,
        maximum_body_bytes: usize,
    ) -> Result<OperationPublicationSources, OperationPublicationReadError> {
        self.inner
            .read_publication_sources(store, cursor, maximum_sources, maximum_body_bytes)
    }

    fn read_checkpoint_head(
        &mut self,
        store: &AuthorityStoreCoordinate,
        domain: &ControlCheckpointDomainIdV1,
        maximum_record_bytes: usize,
    ) -> Result<Option<(ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        self.inner
            .read_checkpoint_head(store, domain, maximum_record_bytes)
    }

    fn verify_accumulator_membership(
        &mut self,
        request: &OperationAccumulatorMembership,
    ) -> Result<VerifiedOperationAccumulatorMembership, OperationAccumulatorPreparationError> {
        self.inner.verify_accumulator_membership(request)
    }

    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        self.inner.prepare_accumulator(request)
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        store: &AuthorityStoreCoordinate,
        record: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.inner.compare_and_retain_operation_lease(store, record)
    }

    fn read_operation_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        self.inner
            .read_operation_record(store, coordinate, maximum_body_bytes)
    }

    fn compare_and_commit(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        if matches!(self.interception, CustodyInterception::CommitUnavailable) {
            return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
        }
        self.inner.compare_and_commit(candidate)
    }

    fn observe_exact(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        self.inner.observe_exact(candidate)
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        maximum_record_bytes: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        let observed = self.inner.read_index(store, query, maximum_record_bytes)?;
        let should_substitute = matches!(
            (self.interception, query),
            (
                CustodyInterception::Preparation,
                OperationCustodyIndexQuery::A2aSendPreparation(_)
            ) | (
                CustodyInterception::Checkpoint,
                OperationCustodyIndexQuery::A2aSendPreparationCheckpoint(_)
            )
        );
        if !should_substitute {
            return Ok(observed);
        }
        let Some(record) = observed.into_record() else {
            return Ok(OperationCustodyIndexObservation::absent(query.clone()));
        };
        let mut value: serde_json::Value =
            serde_json::from_slice(record.canonical_record_jcs().as_slice()).unwrap();
        let object = value.as_object_mut().unwrap();
        match self.interception {
            CustodyInterception::Preparation => {
                object.insert(
                    "native_message_id".to_string(),
                    serde_json::Value::String("substituted-message".to_string()),
                );
            }
            CustodyInterception::Checkpoint => {
                object.insert(
                    "issued_at".to_string(),
                    serde_json::Value::Number(1_u64.into()),
                );
            }
            CustodyInterception::Evidence => unreachable!("evidence uses its exact reader"),
            CustodyInterception::CommitUnavailable => {
                unreachable!("commit interception does not substitute reads")
            }
            CustodyInterception::None => unreachable!("pass-through custody does not substitute"),
        }
        let bytes = pale_core::canon::to_canonical_string(&value)
            .unwrap()
            .into_bytes();
        let substituted = ControlRecord::new(
            ControlRecordCoordinate::new(
                record.coordinate().record_ref().clone(),
                ControlRecordDigest::parse(pale_core::sha256_hex(&bytes)).unwrap(),
            ),
            record.schema_id().clone(),
            record.schema_sha256().clone(),
            ControlRecordBytes::new(bytes).unwrap(),
        );
        OperationCustodyIndexObservation::exact(query.clone(), substituted)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
    }
}

fn operation_cancel_request(
    original: &CommonRequestEnvelopeV1,
    handle: &OperationHandleV1,
    id: &str,
    issued_at: u64,
) -> CommonRequestEnvelopeV1 {
    let mut preimage = original.to_preimage();
    preimage.request_id = ResidentRequestIdV1::parse(format!("{id}-request")).unwrap();
    preimage.correlation_id = ResidentCorrelationIdV1::parse(format!("{id}-correlation")).unwrap();
    preimage.capability_id = CapabilityIdV1::parse(OPERATION_CANCEL_CAPABILITY_ID).unwrap();
    preimage.idempotency_key = RequestIdempotencyKeyV1::parse(format!("{id}-idempotency")).unwrap();
    preimage.issued_at = UnixTimeV1::new(issued_at).unwrap();
    preimage.expires_at = *handle.expires_at();
    preimage.generation = Some(handle.compiled_generation_sha256().clone());
    preimage.fence_token = Some(handle.activation_fence_sha256().clone());
    preimage.body.schema_id = SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).unwrap();
    preimage.body.schema_sha256 = handle.schema_sha256().clone();
    preimage.body.payload = serde_json::to_value(handle).unwrap();
    preimage.body.payload_sha256 = ArtifactDigest::hash_bytes(
        pale_core::canon::to_canonical_string(&preimage.body.payload)
            .unwrap()
            .as_bytes(),
    );
    request_fixture::request(&preimage)
}

struct Store(AuthorityStoreCoordinate);

impl OperationObservationAuthorityStoreResolver for Store {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Ok(self.0.clone())
    }
}

struct ProviderClock {
    times: [u64; 3],
    next: AtomicUsize,
}

impl WallClockSourcePort for ProviderClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        Ok(UnixTimeV1::new(self.times[index]).unwrap())
    }
}

struct ExchangePort {
    tls_trust: ArtifactDigest,
    prepares: Arc<AtomicUsize>,
    starts: Arc<AtomicUsize>,
    response: ResponseModel,
}

impl A2aExchangePort for ExchangePort {
    fn prepare(
        &self,
        request: A2aExchangeRequest,
    ) -> Result<A2aPreparedExchange, A2aExchangeError> {
        self.prepares.fetch_add(1, Ordering::SeqCst);
        assert!(matches!(
            request.input(),
            A2aExchangeInput::Text {
                mode: A2aTaskRequestModeV1::SendMessage,
                message_id: "message-1",
                text: "Inspect the selected workspace."
            }
        ));
        A2aPreparedExchange::from_encoded(
            request,
            br#"{"jsonrpc":"2.0","id":"send-1","method":"SendMessage","params":{"message":{"messageId":"message-1","role":"ROLE_USER","parts":[{"text":"Inspect the selected workspace."}]},"configuration":{"acceptedOutputModes":["text/plain"],"historyLength":0,"returnImmediately":true}}}"#.to_vec(),
            self.tls_trust.clone(),
        )
    }

    fn start(
        &self,
        request: A2aPreparedExchange,
    ) -> Result<Box<dyn A2aExchange>, A2aExchangeError> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        if matches!(self.response, ResponseModel::StartRejected) {
            return Err(A2aExchangeError::ServiceUnavailable);
        }
        Ok(Box::new(StartedExchange {
            coordinate: request.coordinate().clone(),
            response: Some(self.response),
            pending: true,
        }))
    }
}

struct StartedExchange {
    coordinate: A2aExchangeCoordinate,
    response: Option<ResponseModel>,
    pending: bool,
}

struct GetTaskExchangePort {
    tls_trust: ArtifactDigest,
    prepares: Arc<AtomicUsize>,
    starts: Arc<AtomicUsize>,
}

impl A2aExchangePort for GetTaskExchangePort {
    fn prepare(
        &self,
        request: A2aExchangeRequest,
    ) -> Result<A2aPreparedExchange, A2aExchangeError> {
        self.prepares.fetch_add(1, Ordering::SeqCst);
        assert!(matches!(
            request.input(),
            A2aExchangeInput::GetTask { task, context }
                if task.as_str() == "task-1" && context.as_str() == "context-1"
        ));
        A2aPreparedExchange::from_encoded(
            request,
            br#"{"jsonrpc":"2.0","id":"get-readback-1","method":"GetTask","params":{"id":"task-1","historyLength":0}}"#.to_vec(),
            self.tls_trust.clone(),
        )
    }

    fn start(
        &self,
        request: A2aPreparedExchange,
    ) -> Result<Box<dyn A2aExchange>, A2aExchangeError> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(GetTaskStartedExchange {
            coordinate: request.coordinate().clone(),
            phase: 0,
        }))
    }
}

struct GetTaskStartedExchange {
    coordinate: A2aExchangeCoordinate,
    phase: u8,
}

impl A2aExchange for GetTaskStartedExchange {
    fn coordinate(&self) -> &A2aExchangeCoordinate {
        &self.coordinate
    }

    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<A2aExchangeEvidence, A2aExchangeError>>> {
        let phase = self.phase;
        self.phase = self.phase.saturating_add(1);
        match phase {
            0 | 2 => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            1 => Poll::Ready(Some(Ok(A2aExchangeEvidence {
                facts: A2aExchangeFacts::Task {
                    task: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
                    context: A2aRemoteContextIdV1::parse("context-1").unwrap(),
                    state: A2aRemoteTaskStateV1::Completed,
                },
                bytes: br#"{"jsonrpc":"2.0","id":"get-readback-1","result":{"id":"task-1","contextId":"context-1","status":{"state":"TASK_STATE_COMPLETED"}}}"#.to_vec(),
                sse: None,
            }))),
            _ => Poll::Ready(None),
        }
    }

    fn cancel(&mut self) {}
}

struct CancelExchangePort {
    tls_trust: ArtifactDigest,
    prepares: Arc<AtomicUsize>,
    starts: Arc<AtomicUsize>,
    response: CancelResponseModel,
}

#[derive(Clone, Copy)]
enum CancelResponseModel {
    Task,
    TaskThenExtra,
    Eof,
    ProcessLostAfterStart,
    StartRejected,
}

impl A2aExchangePort for CancelExchangePort {
    fn prepare(
        &self,
        request: A2aExchangeRequest,
    ) -> Result<A2aPreparedExchange, A2aExchangeError> {
        self.prepares.fetch_add(1, Ordering::SeqCst);
        assert!(matches!(
            request.input(),
            A2aExchangeInput::CancelTask { task, context }
                if task.as_str() == "task-1" && context.as_str() == "context-1"
        ));
        A2aPreparedExchange::from_encoded(
            request,
            br#"{"jsonrpc":"2.0","id":"cancel-1","method":"CancelTask","params":{"id":"task-1"}}"#
                .to_vec(),
            self.tls_trust.clone(),
        )
    }

    fn start(
        &self,
        request: A2aPreparedExchange,
    ) -> Result<Box<dyn A2aExchange>, A2aExchangeError> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        if matches!(self.response, CancelResponseModel::StartRejected) {
            return Err(A2aExchangeError::ServiceUnavailable);
        }
        Ok(Box::new(CancelStartedExchange {
            coordinate: request.coordinate().clone(),
            response: Some(self.response),
            pending: true,
        }))
    }
}

struct CancelStartedExchange {
    coordinate: A2aExchangeCoordinate,
    response: Option<CancelResponseModel>,
    pending: bool,
}

impl A2aExchange for CancelStartedExchange {
    fn coordinate(&self) -> &A2aExchangeCoordinate {
        &self.coordinate
    }

    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<A2aExchangeEvidence, A2aExchangeError>>> {
        if std::mem::take(&mut self.pending) {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let Some(response) = self.response.take() else {
            return Poll::Ready(None);
        };
        match response {
            CancelResponseModel::Eof => return Poll::Ready(None),
            CancelResponseModel::TaskThenExtra => {
                self.response = Some(CancelResponseModel::Task);
                self.pending = true;
            }
            CancelResponseModel::Task => {
                self.pending = true;
            }
            CancelResponseModel::ProcessLostAfterStart => {
                unreachable!("the fixture loses its process before receiving a response")
            }
            CancelResponseModel::StartRejected => unreachable!("start rejected before polling"),
        }
        Poll::Ready(Some(Ok(A2aExchangeEvidence {
            facts: A2aExchangeFacts::Task {
                task: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
                context: A2aRemoteContextIdV1::parse("context-1").unwrap(),
                state: A2aRemoteTaskStateV1::Canceled,
            },
            bytes: br#"{"jsonrpc":"2.0","id":"cancel-1","result":{"id":"task-1","contextId":"context-1","status":{"state":"TASK_STATE_CANCELED"}}}"#.to_vec(),
            sse: None,
        })))
    }

    fn cancel(&mut self) {}
}

struct OneTimeClock(UnixTimeV1);

impl WallClockSourcePort for OneTimeClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(self.0)
    }
}

struct DelegateClock {
    observed_at: UnixTimeV1,
    failure_at: Option<usize>,
    calls: AtomicUsize,
}

impl DelegateClock {
    fn available() -> Self {
        Self::with_failure(None)
    }

    fn unavailable_at(call: usize) -> Self {
        Self::with_failure(Some(call))
    }

    fn with_failure(failure_at: Option<usize>) -> Self {
        Self {
            observed_at: UnixTimeV1::new(69).unwrap(),
            failure_at,
            calls: AtomicUsize::new(0),
        }
    }
}

impl WallClockSourcePort for DelegateClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if self.failure_at == Some(call) {
            return Err(WallClockSourceError::from_source(std::io::Error::other(
                "delegate clock unavailable",
            )));
        }
        Ok(self.observed_at)
    }
}

struct CountedClock {
    observed_at: UnixTimeV1,
    calls: AtomicUsize,
}

impl CountedClock {
    fn at(observed_at: u64) -> Self {
        Self {
            observed_at: UnixTimeV1::new(observed_at).unwrap(),
            calls: AtomicUsize::new(0),
        }
    }
}

impl WallClockSourcePort for CountedClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.observed_at)
    }
}

struct UnavailableClock;

impl WallClockSourcePort for UnavailableClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Err(WallClockSourceError::from_source(std::io::Error::other(
            "test clock unavailable",
        )))
    }
}

impl A2aExchange for StartedExchange {
    fn coordinate(&self) -> &A2aExchangeCoordinate {
        &self.coordinate
    }

    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<A2aExchangeEvidence, A2aExchangeError>>> {
        if std::mem::take(&mut self.pending) {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        match self.response.take() {
            Some(
                ResponseModel::Task
                | ResponseModel::ResponseClockUnavailable
                | ResponseModel::ResponseClockBackward
                | ResponseModel::ResponseClockAfterRetention,
            ) => Poll::Ready(Some(Ok(task_response_evidence()))),
            Some(ResponseModel::TaskWithPendingEof) => {
                self.response = Some(ResponseModel::Eof);
                self.pending = true;
                Poll::Ready(Some(Ok(task_response_evidence())))
            }
            Some(ResponseModel::TaskThenExtra) => {
                self.response = Some(ResponseModel::Extra);
                Poll::Ready(Some(Ok(task_response_evidence())))
            }
            Some(ResponseModel::Extra) => Poll::Ready(Some(Ok(A2aExchangeEvidence {
                facts: A2aExchangeFacts::DirectMessage,
                bytes: br#"{"jsonrpc":"2.0","id":"send-1","result":{"message":{"messageId":"unexpected-extra","role":"ROLE_AGENT","parts":[]}}}"#.to_vec(),
                sse: None,
            }))),
            Some(
                ResponseModel::DirectMessage
                | ResponseModel::DirectMessageAnchorUnavailable
                | ResponseModel::DirectMessageAnchorOutcomeUnknown,
            ) => Poll::Ready(Some(Ok(A2aExchangeEvidence {
                facts: A2aExchangeFacts::DirectMessage,
                bytes: br#"{"jsonrpc":"2.0","id":"send-1","result":{"message":{"messageId":"message-remote","role":"ROLE_AGENT","parts":[]}}}"#.to_vec(),
                sse: None,
            }))),
            Some(
                ResponseModel::Eof
                | ResponseModel::StartRejected
                | ResponseModel::RetentionTooShort
                | ResponseModel::ProcessLostBeforeStart
                | ResponseModel::RecoveredResponseClockUnavailable
                | ResponseModel::RecoveredResponseClockBackward
                | ResponseModel::PreCustodyClockUnavailable
                | ResponseModel::ReleaseClockUnavailable
                | ResponseModel::CheckpointRejected
                | ResponseModel::CommitUnavailable
                | ResponseModel::AnchorUnavailable,
            )
            | None => Poll::Ready(None),
        }
    }

    fn cancel(&mut self) {}
}

fn task_response_evidence() -> A2aExchangeEvidence {
    A2aExchangeEvidence {
        facts: A2aExchangeFacts::Task {
            task: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
            context: A2aRemoteContextIdV1::parse("context-1").unwrap(),
            state: A2aRemoteTaskStateV1::Working,
        },
        bytes: br#"{"jsonrpc":"2.0","id":"send-1","result":{"task":{"id":"task-1","contextId":"context-1","status":{"state":"TASK_STATE_WORKING"}}}}"#.to_vec(),
        sse: None,
    }
}

struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn poll_context() -> Context<'static> {
    let waker = Box::leak(Box::new(Waker::from(Arc::new(NoopWake))));
    Context::from_waker(waker)
}

fn store_resident_pending<T: 'static>(value: T) -> Option<T> {
    Some(value)
}

fn signed_wire<Preimage, Digest, Wire>(
    preimage: &Preimage,
    digest_name: &str,
    digest: &Digest,
) -> Wire
where
    Preimage: Serialize,
    Digest: Serialize,
    Wire: DeserializeOwned,
{
    let mut value = serde_json::to_value(preimage).unwrap();
    let object = value.as_object_mut().unwrap();
    object.insert(
        digest_name.to_string(),
        serde_json::to_value(digest).unwrap(),
    );
    object.insert(
        "signature".to_string(),
        serde_json::to_value(signature()).unwrap(),
    );
    serde_json::from_value(value).unwrap()
}

fn admitted_reconciliation_frontier(
    signing_authority: &SigningAuthorityBindingV1,
    subject_ref: ResidentRecordRefV1,
    subject_sha256: ArtifactDigest,
) -> ReconciliationFrontierV1 {
    let runtime_authority = signing_authority
        .integrated_runtime_authority()
        .expect("integrated runtime authority");
    let preimage = ReconciliationFrontierPreimageV1 {
        schema_id: SchemaId::parse(RECONCILIATION_FRONTIER_V1_SCHEMA).unwrap(),
        schema_sha256: schema_digest(RECONCILIATION_FRONTIER_V1_SCHEMA_ID),
        deployment_runtime_authority_ref: runtime_authority
            .deployment_runtime_authority_ref
            .clone(),
        deployment_runtime_authority_sha256: runtime_authority
            .deployment_runtime_authority_sha256
            .clone(),
        frontier_id: ReconciliationFrontierIdV1::parse("a2a-send-reconciliation").unwrap(),
        subject: ReconciliationSubjectV1::Projection {
            projection_ref: subject_ref,
            projection_sha256: subject_sha256,
        },
        dimensions: vec![ReconciliationDimensionEntryV1 {
            dimension: ReconciliationDimensionV1::Delivery,
            status: ReconciliationDimensionStatusV1::Pending,
            evidence: None,
        }],
        source_occurrence_refs: Vec::new(),
        owner_record_revisions: Vec::new(),
        generated_at: UnixTimeV1::new(69).unwrap(),
        issuer_principal_id: signing_authority.principal_id().clone(),
        issuer_key_id: ResidentKeyIdV1::parse(signing_authority.signer_key_id()).unwrap(),
        issuer_key_epoch: *signing_authority.signer_key_epoch(),
    };
    let digest = ReconciliationFrontierDigest::hash_canonical(&preimage).unwrap();
    let wire = signed_wire(
        &preimage,
        "canonical_reconciliation_frontier_sha256",
        &digest,
    );
    ReconciliationFrontierV1::admit_contextual_wire(
        wire,
        &SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            signing_authority,
        },
        |_| Ok(()),
        |_, _, _, actual| {
            assert_eq!(actual, &signature());
            Ok(())
        },
    )
    .unwrap()
}

fn exchange_limits(response: ResponseModel) -> A2aExchangeLimits {
    A2aExchangeLimits {
        request_bytes: NonZeroUsize::new(4_096).unwrap(),
        response_bytes: NonZeroUsize::new(4_096).unwrap(),
        evidence_bytes: NonZeroUsize::new(8_192).unwrap(),
        line_bytes: NonZeroUsize::new(2_048).unwrap(),
        event_bytes: NonZeroUsize::new(4_096).unwrap(),
        events: NonZeroUsize::new(8).unwrap(),
        queued_events: NonZeroUsize::new(4).unwrap(),
        connect_timeout: Duration::from_secs(1),
        read_timeout: Duration::from_secs(1),
        total_timeout: Duration::from_secs(
            if matches!(response, ResponseModel::RetentionTooShort) {
                60
            } else {
                2
            },
        ),
    }
}

fn provider_admission<'a>(
    result: CapabilityResultAdmissionContextV1<'a>,
    contracts: &'a ContractSetV1,
    failure_schema: &'a SchemaDigest,
) -> ProviderInvocationAdmissionContext<'a> {
    ProviderInvocationAdmissionContext::new(
        CapabilitySuccessAdmissionContextV1 {
            contract_set_ref: result.expected_request_binding.contract_set_ref(),
            result,
            contract_set: contracts,
        },
        AdapterFailureAdmissionContextV1 {
            expected_schema_sha256: failure_schema,
            expected_request_binding: result.expected_request_binding,
            expected_attempt_id: result.expected_attempt_id,
            expected_implementation_kind: result.expected_implementation_kind,
            expected_implementation_version: result.expected_implementation_version,
            signing_authority: result.signing_authority,
        },
        ProviderInvocationSafeFailureProof::BeforeDispatch,
    )
}

fn admit_fixture_rotation(
    _: &ResidentRecordRefV1,
    _: &ArtifactDigest,
    _: &ResidentRecordRefV1,
    _: &ArtifactDigest,
) -> Result<(), InvalidResidentRecordV1> {
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn recover_known_task(
    operation: &IssuedLocalOperation,
    binding_id: &A2aTaskBindingIdV1,
    route: &A2aRouteAdmissionV1,
    signing: &SigningAuthorityBindingV1,
    checkpoint_fields: &ControlStateCheckpointPreimageV1,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> RecoveredA2aTaskHistory {
    fn matches(reference: &ImmutableRecordReferenceV1, evidence: &[&OperationA2aEvidence]) -> bool {
        evidence.iter().any(|value| {
            value.coordinate().record_ref() == &reference.record_ref
                && value.coordinate().record_sha256().as_str() == reference.record_sha256.as_str()
                && ArtifactDigest::hash_bytes(value.bytes()) == reference.record_sha256
        })
    }

    let coordinate = port.coordinate.clone();
    let authority = checkpoint_authority(checkpoint_fields);
    let local = OperationObservationRecoveryAdmission {
        admit_checkpoint: |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
            ControlStateCheckpointV1::admit_signed_wire(
                wire,
                &checkpoint_fields.schema_sha256,
                &authority,
                |domain, _, _, actual| {
                    assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            )
        },
        resolve_signer: |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
            Ok(signing.clone())
        },
        admit_state: |_: &OperationObservationStateV1| Ok(()),
        admit_rotation: admit_fixture_rotation,
        verify_signature: |domain: &[u8],
                           _: &OperationObservationPreimageV1,
                           _: &OperationObservationDigest,
                           actual: &CanonicalSignatureV1| {
            assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        },
    };
    let mut admit_route = |_: &ResidentRecordRefV1, wire: A2aRouteAdmissionWireV1| {
        A2aRouteAdmissionV1::admit_contextual_wire(
            wire,
            &A2aRouteAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: route.schema_sha256(),
                    signing_authority: signing,
                },
                provider_binding: route.provider_binding(),
                route: route.route(),
            },
            |_| Ok(()),
            |domain, _, _, actual| {
                assert_eq!(domain, A2A_ROUTE_ADMISSION_V1_SIGNATURE_DOMAIN);
                assert_eq!(actual, &signature());
                Ok(())
            },
        )
    };
    let mut resolve_binding_signer =
        |_: &ResidentRecordRefV1, _: &A2aTaskBindingWireV1| Ok(signing.clone());
    let mut admit_binding_evidence =
        |binding: &A2aTaskBindingPreimageV1, evidence: &[&OperationA2aEvidence]| {
            if evidence.len() != 2
                || !matches(&binding.request_evidence, evidence)
                || !matches(&binding.initial_task_evidence, evidence)
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "A2A task binding fixture",
                    invariant: "exact initial request and Task response",
                });
            }
            Ok(())
        };
    let mut verify_binding_signature =
        |domain: &[u8],
         _: &A2aTaskBindingPreimageV1,
         _: &A2aTaskBindingDigest,
         actual: &CanonicalSignatureV1| {
            assert_eq!(domain, A2A_TASK_BINDING_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        };
    let mut resolve_observation_signer =
        |_: &ResidentRecordRefV1, _: &A2aTaskObservationWireV1| Ok(signing.clone());
    let mut admit_observation_evidence =
        |observation: &A2aTaskObservationPreimageV1, evidence: &[&OperationA2aEvidence]| {
            let exact = match &observation.source {
                A2aTaskObservationSourceV1::InitialSendMessageResponse { evidence: expected }
                | A2aTaskObservationSourceV1::InitialStreamingEvent {
                    evidence: expected, ..
                }
                | A2aTaskObservationSourceV1::StreamingEvent {
                    evidence: expected, ..
                } => evidence.len() == 1 && matches(expected, evidence),
                A2aTaskObservationSourceV1::GetTask { request, response }
                | A2aTaskObservationSourceV1::CancelTaskReadBack { request, response } => {
                    evidence.len() == 2 && matches(request, evidence) && matches(response, evidence)
                }
            };
            if !exact {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "A2A task observation fixture",
                    invariant: "exact source evidence",
                });
            }
            Ok(())
        };
    let mut admit_rotation = admit_fixture_rotation;
    let mut verify_observation_signature =
        |domain: &[u8],
         _: &A2aTaskObservationPreimageV1,
         _: &A2aTaskObservationDigest,
         actual: &CanonicalSignatureV1| {
            assert_eq!(domain, A2A_TASK_OBSERVATION_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        };
    recover_a2a_task_history(
        A2aTaskRecoveryInput {
            store: &coordinate,
            operation,
            task_binding_id: binding_id,
            observation: A2aTaskObservationSelection::SelectedHead,
            checkpoint_authority: &authority,
            local_limits: OperationObservationRecoveryLimits::new(16, 1 << 20, 65_536).unwrap(),
            checkpoint_limits: OperationCheckpointAncestryLimits::new(16, 1 << 20, 16).unwrap(),
            membership_limits: AccumulatorMembershipLimits::new(32, 100_000, 1_000_000).unwrap(),
            limits: A2aTaskRecoveryLimits::new(8, 65_536, 1 << 20, 16, 1 << 20).unwrap(),
        },
        port,
        anchor,
        local,
        |domain, _, _, actual| {
            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        },
        A2aTaskRecoveryAdmission {
            admit_route: &mut admit_route,
            resolve_binding_signer: &mut resolve_binding_signer,
            admit_binding_evidence: &mut admit_binding_evidence,
            verify_binding_signature: &mut verify_binding_signature,
            resolve_observation_signer: &mut resolve_observation_signer,
            admit_observation_evidence: &mut admit_observation_evidence,
            admit_rotation: &mut admit_rotation,
            verify_observation_signature: &mut verify_observation_signature,
        },
    )
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn exercise_get_task_readback(
    file: &std::path::Path,
    coordinate: &AuthorityStoreCoordinate,
    operation: &IssuedLocalOperation,
    request: &CommonRequestEnvelopeV1,
    generation: &AdmittedCompiledGeneration,
    selection: &ControlRecord,
    recovered: &RecoveredA2aTaskHistory,
    route: &A2aRouteAdmissionV1,
    signing: &SigningAuthorityBindingV1,
    observation_schema: &SchemaDigest,
    checkpoint_fields: &ControlStateCheckpointPreimageV1,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
) {
    let binding_id = recovered.binding().1.task_binding_id().clone();
    let mut restarted = connection(file);
    let mut restarted_port = SqliteOperationCustody {
        connection: &mut restarted,
        coordinate: coordinate.clone(),
    };
    let recovered = recover_known_task(
        operation,
        &binding_id,
        route,
        signing,
        checkpoint_fields,
        &mut restarted_port,
        anchor,
    );
    drop(restarted_port);
    drop(restarted);

    let local_head = (
        recovered.local_observation().selected_head().0.clone(),
        recovered.local_observation().selected_head().1.clone(),
    );
    let request_id = A2aJsonRpcRequestIdV1::parse("get-readback-1").unwrap();
    let input = |request_record_ref, response_record_ref| A2aGetTaskReadbackInput {
        history: &recovered,
        attempt: OperationAttemptHistoryInput {
            operation,
            request,
            session: operation.historical_session_lease(),
            lease: operation.historical_operation_lease(),
            generation,
            dispatched: recovered.dispatch().1,
            selection,
            maximum_selection_bytes: 65_536,
        },
        request_id: &request_id,
        request_record_ref,
        response_record_ref,
        limits: exchange_limits(ResponseModel::Task),
        maximum_evidence_bytes: 8_192,
    };
    let prepares = Arc::new(AtomicUsize::new(0));
    let starts = Arc::new(AtomicUsize::new(0));
    let exchange = GetTaskExchangePort {
        tls_trust: ArtifactDigest::hash_bytes(b"GetTask fixture TLS trust"),
        prepares: Arc::clone(&prepares),
        starts: Arc::clone(&starts),
    };
    let authorization_clock = CountedClock::at(72);
    assert!(matches!(
        start_a2a_get_task_readback(
            input(
                recovered.binding().0.clone(),
                record_ref("a2a/substituted-get-response")
            ),
            &exchange,
            &authorization_clock,
            |_| panic!("substitution must fail before current authorization")
        ),
        Err(A2aGetTaskReadbackError::Association)
    ));
    assert_eq!(authorization_clock.calls.load(Ordering::SeqCst), 1);
    assert_eq!(prepares.load(Ordering::SeqCst), 0);
    assert_eq!(starts.load(Ordering::SeqCst), 0);

    let near_expiry = operation
        .handle()
        .1
        .retain_until()
        .get()
        .checked_sub(1)
        .expect("fixture retention has a preceding second");
    let authorization_clock = CountedClock::at(near_expiry);
    let authorization_calls = AtomicUsize::new(0);
    assert!(matches!(
        start_a2a_get_task_readback(
            input(
                record_ref("a2a/get-readback-request"),
                record_ref("a2a/get-readback-response")
            ),
            &exchange,
            &authorization_clock,
            |_| {
                authorization_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        ),
        Err(A2aGetTaskReadbackError::Time)
    ));
    assert_eq!(authorization_clock.calls.load(Ordering::SeqCst), 1);
    assert_eq!(authorization_calls.load(Ordering::SeqCst), 0);
    assert_eq!(prepares.load(Ordering::SeqCst), 0);
    assert_eq!(starts.load(Ordering::SeqCst), 0);

    let authorization_clock = CountedClock::at(72);
    let authorization_calls = AtomicUsize::new(0);
    let in_flight = start_a2a_get_task_readback(
        input(
            record_ref("a2a/get-readback-request"),
            record_ref("a2a/get-readback-response"),
        ),
        &exchange,
        &authorization_clock,
        |actual| {
            authorization_calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(actual.operation, operation.handle());
            assert_eq!(actual.dispatch, recovered.dispatch());
            assert_eq!(
                actual.local_head,
                recovered.local_observation().selected_head()
            );
            assert_eq!(actual.route, recovered.route());
            assert_eq!(actual.binding, recovered.binding());
            assert_eq!(actual.remote_head, recovered.observation());
            assert_eq!(actual.authorized_at, UnixTimeV1::new(72).unwrap());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(authorization_clock.calls.load(Ordering::SeqCst), 1);
    assert_eq!(authorization_calls.load(Ordering::SeqCst), 1);
    assert_eq!(prepares.load(Ordering::SeqCst), 1);
    assert_eq!(starts.load(Ordering::SeqCst), 1);

    let mut cx = poll_context();
    let mut stored = Some(match in_flight.poll_response(&mut cx) {
        A2aGetTaskReadbackPoll::Pending(pending) => pending,
        A2aGetTaskReadbackPoll::Ready(_) => panic!("GetTask must first park"),
    });
    let in_flight = stored.take().unwrap();
    let received = match in_flight.poll_response(&mut cx) {
        A2aGetTaskReadbackPoll::Ready(Ok(received)) => received,
        _ => panic!("stored GetTask exchange must yield one Task"),
    };
    let response_clock = CountedClock::at(73);
    let mut stored = Some(match received.poll_eof(&mut cx, &response_clock) {
        A2aGetTaskReadbackPoll::Pending(pending) => pending,
        A2aGetTaskReadbackPoll::Ready(_) => panic!("received Task must park before EOF"),
    });
    assert_eq!(response_clock.calls.load(Ordering::SeqCst), 0);
    let received = stored.take().unwrap();
    let completed = match received.poll_eof(&mut cx, &response_clock) {
        A2aGetTaskReadbackPoll::Ready(Ok(completed)) => completed,
        _ => panic!("stored Task must reach exact unary EOF"),
    };
    assert_eq!(response_clock.calls.load(Ordering::SeqCst), 1);
    assert_eq!(starts.load(Ordering::SeqCst), 1);

    let checkpoint_authority = checkpoint_authority(checkpoint_fields);
    let commit_input =
        |observation_ref, checkpoint_ref: &str, checkpoint_id: &str| A2aGetTaskCommitInput {
            observation: A2aGetTaskObservationInput {
                history: &recovered,
                operation,
                observation_ref,
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: observation_schema,
                    signing_authority: signing,
                },
                maximum_evidence_bytes: 8_192,
                maximum_record_bytes: 65_536,
            },
            checkpoint: OperationCheckpointInput {
                reference: record_ref(checkpoint_ref),
                id: ControlCheckpointIdV1::parse(checkpoint_id).unwrap(),
            },
            issued_at: UnixTimeV1::new(74).unwrap(),
            checkpoint_schema: &checkpoint_fields.schema_sha256,
            checkpoint_authority: &checkpoint_authority,
            accumulator_limits: AccumulatorPreparationLimits::new(
                8, 20_000, 20_000, 8_192, 200_000,
            )
            .unwrap(),
            membership_limits: AccumulatorMembershipLimits::new(8, 20_000, 200_000).unwrap(),
        };
    let observation_signs = AtomicUsize::new(0);
    let rejected = match commit_a2a_get_task_readback(
        completed,
        commit_input(
            record_ref("a2a/get-readback-request"),
            "a2a/rejected-get-checkpoint",
            "a2a-rejected-get-checkpoint",
        ),
        &pale_adapter_a2a::A2aGetTaskDecoder,
        port,
        anchor,
        admit_fixture_rotation,
        |_, _, _| {
            observation_signs.fetch_add(1, Ordering::SeqCst);
            Ok(signature())
        },
        |_| Ok(coordinate.clone()),
        |_, _, _| Ok(signature()),
    ) {
        Ok(_) => panic!("evidence-reference substitution must be rejected"),
        Err(rejected) => rejected,
    };
    let completed = match rejected {
        A2aGetTaskCommitFailure::Observation { completed, .. } => *completed,
        _ => panic!("evidence-reference substitution must fail before custody"),
    };
    assert_eq!(observation_signs.load(Ordering::SeqCst), 0);
    assert_eq!(starts.load(Ordering::SeqCst), 1);

    let completed = commit_a2a_get_task_readback(
        completed,
        commit_input(
            record_ref("a2a/get-readback-observation"),
            "a2a/get-readback-checkpoint",
            "a2a-get-readback-checkpoint",
        ),
        &pale_adapter_a2a::A2aGetTaskDecoder,
        port,
        anchor,
        admit_fixture_rotation,
        |domain, _, _| {
            assert_eq!(domain, A2A_TASK_OBSERVATION_V1_SIGNATURE_DOMAIN);
            Ok(signature())
        },
        |_| Ok(coordinate.clone()),
        |domain, _, _| {
            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
            Ok(signature())
        },
    )
    .unwrap();
    assert_eq!(completed.binding(), recovered.binding());
    assert_eq!(
        completed.observation().1.predecessor_ref().as_ref(),
        Some(recovered.observation().0)
    );
    assert_eq!(
        completed.observation().1.predecessor_sha256().as_ref(),
        Some(
            recovered
                .observation()
                .1
                .canonical_a2a_task_observation_sha256()
        )
    );
    assert_eq!(
        completed.observation().1.remote_state(),
        &A2aRemoteTaskStateV1::Completed
    );
    assert!(matches!(
        completed.observation().1.source(),
        A2aTaskObservationSourceV1::GetTask { request, response }
            if request.record_ref.as_str() == "a2a/get-readback-request"
                && response.record_ref.as_str() == "a2a/get-readback-response"
    ));
    assert!(completed
        .custody()
        .candidate()
        .record(OperationCustodyRecordKind::OperationObservation)
        .is_none());
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    assert_eq!(
        port.compare_and_commit(completed.custody().candidate())
            .unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );

    let observation = (
        completed.observation().0.clone(),
        completed.observation().1.clone(),
    );
    let checkpoint_fields = completed.checkpoint_preimage().clone();
    let mut restarted = connection(file);
    let mut restarted_port = SqliteOperationCustody {
        connection: &mut restarted,
        coordinate: coordinate.clone(),
    };
    let restarted = recover_known_task(
        operation,
        &binding_id,
        route,
        signing,
        &checkpoint_fields,
        &mut restarted_port,
        anchor,
    );
    assert_eq!(restarted.binding(), completed.binding());
    assert_eq!(restarted.observation(), (&observation.0, &observation.1));
    assert_eq!(
        restarted.local_observation().selected_head(),
        (&local_head.0, &local_head.1),
        "remote Task state must not promote Pale operation state"
    );
    assert_eq!(starts.load(Ordering::SeqCst), 1);
}

fn recover_historical_operation_observation(
    operation: &IssuedLocalOperation,
    observation: (&ResidentRecordRefV1, &OperationObservationV1),
    checkpoint_fields: &ControlStateCheckpointPreimageV1,
    signing: &SigningAuthorityBindingV1,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> RecoveredOperationObservation {
    let coordinate = port.coordinate.clone();
    let authority = checkpoint_authority(checkpoint_fields);
    recover_operation_observation(
        OperationObservationRecoveryInput {
            store: &coordinate,
            operation,
            observation: OperationObservationSelection::HistoryMember {
                reference: observation.0,
                digest: observation.1.canonical_operation_observation_sha256(),
            },
            checkpoint_authority: &authority,
            limits: OperationObservationRecoveryLimits::new(16, 1 << 20, 65_536).unwrap(),
            checkpoint_limits: OperationCheckpointAncestryLimits::new(16, 1 << 20, 16).unwrap(),
            membership_limits: AccumulatorMembershipLimits::new(4, 20_000, 200_000).unwrap(),
        },
        port,
        anchor,
        OperationObservationRecoveryAdmission {
            admit_checkpoint: |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                ControlStateCheckpointV1::admit_signed_wire(
                    wire,
                    &checkpoint_fields.schema_sha256,
                    &authority,
                    |_, _, _, actual| {
                        assert_eq!(actual, &signature());
                        Ok(())
                    },
                )
            },
            resolve_signer: |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
                Ok(signing.clone())
            },
            admit_state: |_: &OperationObservationStateV1| Ok(()),
            admit_rotation: admit_fixture_rotation,
            verify_signature: |_: &[u8],
                               _: &OperationObservationPreimageV1,
                               _: &OperationObservationDigest,
                               actual: &CanonicalSignatureV1| {
                assert_eq!(actual, &signature());
                Ok(())
            },
        },
        |_: &[u8],
         _: &ControlStateCheckpointPreimageV1,
         _: &ControlStateCheckpointDigest,
         actual: &CanonicalSignatureV1| {
            assert_eq!(actual, &signature());
            Ok(())
        },
    )
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn retain_response_observation(
    prepared: PreparedA2aSendObservation,
    base: &ControlStateCheckpointPreimageV1,
    checkpoint_name: &str,
    expected_response_evidence: bool,
    resolver: &Store,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> (OperationCustodyCandidate, ControlStateCheckpointV1) {
    let predecessor = prepared.predecessor_checkpoint().1.clone();
    let checkpoint_ref = record_ref(&format!("{checkpoint_name}/record"));
    let checkpoint_authority = checkpoint_authority(base);
    let completed = commit_a2a_send_observation(
        prepared,
        A2aSendObservationCommitInput {
            checkpoint: OperationCheckpointInput {
                reference: checkpoint_ref,
                id: ControlCheckpointIdV1::parse(checkpoint_name).unwrap(),
            },
            clock: &OneTimeClock(UnixTimeV1::new(70).unwrap()),
            checkpoint_schema: &base.schema_sha256,
            checkpoint_authority: &checkpoint_authority,
            accumulator_limits: AccumulatorPreparationLimits::new(
                8, 20_000, 20_000, 8_192, 200_000,
            )
            .unwrap(),
        },
        resolver,
        custody,
        anchor,
        |_, _, _| Ok(signature()),
    )
    .unwrap();
    assert_eq!(
        completed.checkpoint_preimage().changed_leaf_keys.len(),
        1 + usize::from(expected_response_evidence)
    );
    assert_eq!(
        completed.checkpoint_preimage().leaf_count.get(),
        predecessor.leaf_count().get() + u64::from(expected_response_evidence)
    );
    let candidate = completed.custody().candidate().clone();
    let checkpoint = completed.checkpoint().1.clone();
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    (candidate, checkpoint)
}

#[allow(clippy::too_many_arguments)]
fn retain_cancel_disposition(
    prepared: &PreparedA2aCancelTaskDisposition<'_>,
    selection: OperationCheckpointSelection,
    base: &ControlStateCheckpointPreimageV1,
    checkpoint_name: &str,
    expected_response_evidence: usize,
    resolver: &Store,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> (OperationCustodyCandidate, ControlStateCheckpointV1) {
    let (predecessor_ref, predecessor) = prepared.predecessor_checkpoint();
    let prepared = prepared
        .prepare_checkpoint(
            selection,
            AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000).unwrap(),
            resolver,
            custody,
        )
        .unwrap();
    assert_eq!(
        prepared.changed_keys().len(),
        1 + expected_response_evidence
    );
    assert_eq!(
        prepared.leaf_count().get(),
        predecessor.leaf_count().get() + expected_response_evidence as u64
    );
    let mut fields = base.clone();
    fields.control_checkpoint_id = ControlCheckpointIdV1::parse(checkpoint_name).unwrap();
    fields.domain_sequence =
        ControlDomainSequenceV1::new(predecessor.domain_sequence().get() + 1).unwrap();
    fields.previous_control_checkpoint_ref = Some(predecessor_ref.clone());
    fields.previous_control_checkpoint_sha256 =
        Some(predecessor.canonical_control_checkpoint_sha256().clone());
    fields.committed_transaction_sequence = prepared.transaction_sequence();
    fields.state_root_sha256 = prepared.state_root().clone();
    fields.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
    fields.changed_leaf_keys = prepared.changed_keys().to_vec();
    fields.leaf_count = prepared.leaf_count();
    fields.issued_at = UnixTimeV1::new(75).unwrap();
    let (physical, checkpoint) = prepared
        .issue_checkpoint(
            &record_ref(&format!("{checkpoint_name}/record")),
            fields.clone(),
            &fields.schema_sha256,
            &checkpoint_authority(&fields),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    let completed = physical
        .commit(custody, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    let candidate = completed.candidate().clone();
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    (candidate, checkpoint)
}

#[allow(clippy::too_many_arguments)]
fn retain_unrelated_parent_observation(
    operation: &IssuedLocalOperation,
    initial_candidate: &OperationCustodyCandidate,
    signing: &SigningAuthorityBindingV1,
    prior: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    resolver: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> ControlStateCheckpointV1 {
    let initial_record = initial_candidate
        .record(OperationCustodyRecordKind::OperationObservation)
        .unwrap();
    let initial = issue_observation(
        signing,
        operation.handle().0,
        operation.handle().1,
        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
        OperationObservationRevisionV1::new(1).unwrap(),
        None,
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving,
        },
        60,
    );
    assert_eq!(
        initial_record.canonical_record_jcs().as_slice(),
        pale_core::canon::to_canonical_string(&initial)
            .unwrap()
            .as_bytes()
    );
    let successor_ref = record_ref("a2a/unrelated-parent-context");
    let successor = issue_observation(
        signing,
        operation.handle().0,
        operation.handle().1,
        initial.schema_sha256().clone(),
        OperationObservationRevisionV1::new(2).unwrap(),
        Some((initial_record.coordinate().record_ref(), &initial)),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::ContextBound {
                evidence: ImmutableRecordReferenceV1 {
                    record_ref: record_ref("a2a/unrelated-parent-context-evidence"),
                    record_sha256: ArtifactDigest::hash_bytes(
                        b"fixture-only admitted parent context evidence",
                    ),
                },
            },
        },
        74,
    );
    let append = prepare_operation_observation_append(
        OperationObservationTransition::new(
            operation.handle().0,
            operation.handle().1,
            initial_record.coordinate().record_ref(),
            &initial,
            &successor_ref,
            &successor,
        ),
        resolver,
    )
    .unwrap();
    let checkpoint_ref = record_ref("a2a/unrelated-parent-checkpoint");
    let (prepared, fields) =
        super::prepare_append_with_ref(append, operation, prior, custody, 74, &checkpoint_ref);
    let completed = prepared
        .commit(custody, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    read_checkpoint(
        completed.candidate().checkpoint().successor_record(),
        &fields,
    )
}

fn run_case(
    admit_matching_transport: bool,
    response: ResponseModel,
    cancel_response: CancelResponseModel,
) {
    run_case_mode(
        admit_matching_transport,
        RouteProviderMatch::Exact,
        false,
        false,
        LocalStartContext::SelfContained,
        response,
        cancel_response,
    );
}

fn run_case_with_route_provider(
    admit_matching_transport: bool,
    admit_matching_route_provider: bool,
    response: ResponseModel,
    cancel_response: CancelResponseModel,
) {
    run_case_mode(
        admit_matching_transport,
        if admit_matching_route_provider {
            RouteProviderMatch::Exact
        } else {
            RouteProviderMatch::UnlikeId
        },
        false,
        false,
        LocalStartContext::SelfContained,
        response,
        cancel_response,
    );
}

fn run_case_with_route_provider_identity(
    admit_matching_route_provider_identity: bool,
    response: ResponseModel,
    cancel_response: CancelResponseModel,
) {
    run_case_mode(
        true,
        if admit_matching_route_provider_identity {
            RouteProviderMatch::Exact
        } else {
            RouteProviderMatch::UnlikeIdentity
        },
        false,
        false,
        LocalStartContext::SelfContained,
        response,
        cancel_response,
    );
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RouteProviderMatch {
    Exact,
    UnlikeId,
    UnlikeIdentity,
}

fn run_case_mode(
    admit_matching_transport: bool,
    route_provider_match: RouteProviderMatch,
    recover_dispatch: bool,
    exercise_get_task: bool,
    context_selection: LocalStartContext,
    response: ResponseModel,
    cancel_response: CancelResponseModel,
) {
    use crate::resident_fixture::local_operation_start_fixture::with_start_context;

    super::with_host_generation("bounded_compute", |generation, effective| {
        let registry = NativeActionRegistry::new().unwrap();
        with_start_context(
            generation,
            effective,
            if admit_matching_transport {
                "a2a-send"
            } else {
                "a2a-transport-substitution"
            },
            |_| {},
            StartAdmission::AcceptedOwnerModel,
            context_selection,
            |prepared, request, _, _, parents| {
                let directory = crate::test_support::private_tempdir();
                let binding = parents.current.deployment_binding;
                let scope = ControlStoreScope::Deployment(DeploymentStoreScope::new(
                    ResidentWorkspaceIdV1::parse(
                        binding.deployment_terms().workspace_id().as_str(),
                    )
                    .unwrap(),
                    parents.current.deployment_binding_ref.clone(),
                    ArtifactDigest::parse(binding.canonical_binding_sha256().as_str()).unwrap(),
                ));
                let location = ControlStorageRoot::retain(directory.path())
                    .unwrap()
                    .prepare_store_location(scope.clone())
                    .unwrap();
                let coordinate = store_coordinate();
                let initialized = initialize_unadmitted_store(
                    &location,
                    ControlStoreIdentity::new(
                        scope,
                        coordinate.store_id().clone(),
                        coordinate.incarnation_id().clone(),
                    ),
                )
                .unwrap();
                let file = initialized.path();
                let sessions = DeploymentControlSqliteStore::open_unadmitted_candidate(
                    &location,
                    coordinate.clone(),
                )
                .unwrap();
                let session = parents.session.lease();
                let body = SessionLeaseBody::new(
                    coordinate.clone(),
                    session.session_admission_lease_digest().clone(),
                    session.schema_id().clone(),
                    session.schema_sha256().clone(),
                    ControlRecordBytes::new(
                        pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(
                            session,
                        ))
                        .unwrap()
                        .into_bytes(),
                    )
                    .unwrap(),
                );
                assert_eq!(
                    sessions.compare_and_retain(&body).unwrap(),
                    SessionLeaseBodyRetention::Retained
                );
                drop(sessions);
                let mut sql = connection(file);
                let mut port = SqliteOperationCustody {
                    connection: &mut sql,
                    coordinate: coordinate.clone(),
                };
                retain_operation_lease_body(
                    &mut port,
                    &coordinate,
                    &record_ref("start-lease"),
                    parents.lease,
                    1 << 20,
                )
                .unwrap();
                let prepared = prepared
                    .unwrap()
                    .prepare_checkpoint(
                        AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4_096, 100_000)
                            .unwrap(),
                        &mut port,
                    )
                    .unwrap();
                let mut root_fields = checkpoint_preimage(1, None);
                root_fields.checkpoint_domain_id =
                    SupportedOperationCheckpointProfile::conflict_domain(request).unwrap();
                root_fields.checkpoint_profile_sha256 =
                    SupportedOperationCheckpointProfile::content_digest().unwrap();
                root_fields.committed_transaction_sequence = prepared.transaction_sequence();
                root_fields.state_root_sha256 = prepared.state_root().clone();
                root_fields.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
                root_fields.changed_leaf_keys = prepared.changed_keys().to_vec();
                root_fields.leaf_count = prepared.leaf_count();
                root_fields.issued_at = UnixTimeV1::new(61).unwrap();
                let (physical, pending) = prepared
                    .issue_checkpoint(
                        &record_ref("operation/checkpoint-1"),
                        root_fields.clone(),
                        &root_fields.schema_sha256,
                        &checkpoint_authority(&root_fields),
                        |_, _, _| Ok(signature()),
                    )
                    .unwrap();
                let mut anchor = IndependentAnchor::default();
                let retained = physical
                    .commit(&mut port, &mut anchor, 1 << 20)
                    .unwrap()
                    .advance_anchor(&mut anchor)
                    .unwrap();
                let root_candidate = retained.candidate().clone();
                let root_checkpoint =
                    read_checkpoint(root_candidate.checkpoint().successor_record(), &root_fields);
                let root = pending
                    .finish(retained)
                    .unwrap_or_else(|_| panic!("exact root custody"));
                let fixture = child_start::Fixture {
                    operation: &root,
                    request,
                    parents: &parents,
                    generation,
                    checkpoint: &root_fields,
                    file,
                    registry: &registry,
                };
                let custody = RefCell::new((&mut port, &mut anchor));
                child_start::with_delegate_start(
                    &fixture,
                    |lease_ref, lease| {
                        let mut borrowed = custody.borrow_mut();
                        let (port, anchor) = &mut *borrowed;
                        retain_operation_lease_body(*port, &coordinate, lease_ref, lease, 1 << 20)
                            .unwrap();
                        child_start::prepare_parent(
                            &fixture,
                            &[&OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::Resolving,
                            }],
                            port,
                            anchor,
                            || {},
                        )
                        .unwrap()
                    },
                    "a2a-delegate",
                    |prepared, child_request, _, owners, signs| {
                        assert_eq!((owners, signs), (1, 2));
                        let mut borrowed = custody.borrow_mut();
                        let (port, anchor) = &mut *borrowed;
                        let prepared = prepared
                            .unwrap()
                            .prepare_checkpoint(
                                AccumulatorPreparationLimits::new(
                                    4, 10_000, 10_000, 4_096, 100_000,
                                )
                                .unwrap(),
                                *port,
                            )
                            .unwrap();
                        let (previous_ref, previous) = prepared.predecessor().unwrap();
                        assert_eq!(previous, &root_checkpoint);
                        let mut child_fields = root_fields.clone();
                        child_fields.control_checkpoint_id =
                            ControlCheckpointIdV1::parse("a2a-delegate-checkpoint").unwrap();
                        child_fields.domain_sequence =
                            ControlDomainSequenceV1::new(previous.domain_sequence().get() + 1)
                                .unwrap();
                        child_fields.previous_control_checkpoint_ref = Some(previous_ref.clone());
                        child_fields.previous_control_checkpoint_sha256 =
                            Some(previous.canonical_control_checkpoint_sha256().clone());
                        child_fields.committed_transaction_sequence =
                            prepared.transaction_sequence();
                        child_fields.state_root_sha256 = prepared.state_root().clone();
                        child_fields.unresolved_custody_root_sha256 =
                            prepared.unresolved_root().clone();
                        child_fields.changed_leaf_keys = prepared.changed_keys().to_vec();
                        child_fields.leaf_count = prepared.leaf_count();
                        child_fields.issued_at = UnixTimeV1::new(65).unwrap();
                        let (physical, pending_child) = prepared
                            .issue_checkpoint(
                                &record_ref("a2a/delegate-checkpoint"),
                                child_fields.clone(),
                                &child_fields.schema_sha256,
                                &checkpoint_authority(&child_fields),
                                |_, _, _| Ok(signature()),
                            )
                            .unwrap();
                        let retained_child = physical
                            .commit(*port, *anchor, 1 << 20)
                            .unwrap()
                            .advance_anchor(*anchor)
                            .unwrap();
                        let child_candidate = retained_child.candidate().clone();
                        let child = pending_child
                            .finish(retained_child)
                            .unwrap_or_else(|_| panic!("exact child custody"));
                        assert_eq!(
                            child.handle().1.operation_class(),
                            &OperationClassV1::Delegate
                        );

                        child_start::with_admitted_child_session(&parents, |session, current| {
                            let first_record = child_candidate
                                .record(OperationCustodyRecordKind::OperationObservation)
                                .unwrap();
                            let observation_schema =
                                schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
                            let first = issue_observation(
                                parents.signing,
                                child.handle().0,
                                child.handle().1,
                                observation_schema.clone(),
                                OperationObservationRevisionV1::new(1).unwrap(),
                                None,
                                OperationObservationStateV1::Active {
                                    phase: OperationActivePhaseV1::Resolving,
                                },
                                65,
                            );
                            assert_eq!(
                                first_record.canonical_record_jcs().as_slice(),
                                pale_core::canon::to_canonical_string(&first)
                                    .unwrap()
                                    .as_bytes()
                            );
                            let first_observation = OperationObservationAdmissionContextV1 {
                                signed: SignedResidentAdmissionContextV1 {
                                    expected_schema_sha256: &observation_schema,
                                    signing_authority: parents.signing,
                                },
                                handle_ref: child.handle().0,
                                handle: child.handle().1,
                                predecessor: Some((first_record.coordinate().record_ref(), &first)),
                            };
                            let child_checkpoint = read_checkpoint(
                                child_candidate.checkpoint().successor_record(),
                                &child_fields,
                            );
                            let resolve = |_: &OperationObservationV1| Ok(coordinate.clone());
                            let context_binding =
                                context_selection.has_evidence_obligations().then(|| {
                                    let evidence = ImmutableRecordReferenceV1 {
                                        record_ref: record_ref("a2a/delegate-context"),
                                        record_sha256: ArtifactDigest::hash_bytes(
                                            b"admitted A2A delegate context",
                                        ),
                                    };
                                    let prepared = prepare_operation_context_binding(
                                        OperationContextBindingInput {
                                            operation: &child,
                                            request: child_request,
                                            session,
                                            current,
                                            lease: child.historical_operation_lease(),
                                            evidence: evidence.clone(),
                                            observation_ref: record_ref(
                                                "a2a/delegate-context-bound",
                                            ),
                                            observation: &first_observation,
                                        },
                                        &mut ProvingAuthority,
                                        |actual, obligations| {
                                            assert_eq!(actual, &evidence);
                                            assert!(!obligations.is_empty());
                                            assert_eq!(
                                                obligations,
                                                child
                                                    .historical_operation_lease()
                                                    .evidence_obligations()
                                            );
                                            Ok(())
                                        },
                                        |_, _, _, _| Ok(()),
                                        |_, _, _| Ok(signature()),
                                    )
                                    .unwrap();
                                    let (physical, fields) = super::prepare_append_with_ref(
                                        prepared.prepare_append(&resolve).unwrap(),
                                        &child,
                                        (
                                            child_candidate
                                                .checkpoint()
                                                .successor()
                                                .coordinate()
                                                .record_ref(),
                                            &child_checkpoint,
                                        ),
                                        *port,
                                        66,
                                        &record_ref("a2a/delegate-context-checkpoint"),
                                    );
                                    let completed = physical
                                        .commit(*port, *anchor)
                                        .unwrap()
                                        .advance_anchor(*anchor)
                                        .unwrap();
                                    let binding = prepared.finish(&completed, &resolve).unwrap();
                                    let checkpoint_ref = completed
                                        .candidate()
                                        .checkpoint()
                                        .successor()
                                        .coordinate()
                                        .record_ref()
                                        .clone();
                                    let checkpoint = read_checkpoint(
                                        completed.candidate().checkpoint().successor_record(),
                                        &fields,
                                    );
                                    (binding, checkpoint_ref, checkpoint, fields)
                                });
                            let observation = OperationObservationAdmissionContextV1 {
                                signed: first_observation.signed,
                                handle_ref: child.handle().0,
                                handle: child.handle().1,
                                predecessor: Some(context_binding.as_ref().map_or(
                                    (first_record.coordinate().record_ref(), &first),
                                    |(binding, _, _, _)| binding.observation(),
                                )),
                            };
                            let request_binding = child_request.local_request_binding().unwrap();
                            let attempt = CapabilityAttemptIdV1::parse("a2a-attempt-1").unwrap();
                            let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
                            let version = BoundedResidentTextV1::parse("1").unwrap();
                            let contracts = local_start_contract_set();
                            let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
                            let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
                            let admission = provider_admission(
                                CapabilityResultAdmissionContextV1 {
                                    expected_schema_sha256: &result_schema,
                                    expected_request_binding: &request_binding,
                                    expected_attempt_id: &attempt,
                                    expected_implementation_kind: &kind,
                                    expected_implementation_version: &version,
                                    signing_authority: parents.signing,
                                },
                                &contracts,
                                &failure_schema,
                            );
                            let provider = generation
                                .provider_binding("provider-binding-claude-code")
                                .unwrap();
                            let invocation = ProviderInvocationRequest::new(
                                ProviderInvocationCoordinate::new(
                                    generation.record().compiled_generation_digest(),
                                    generation.record().instance_digest(),
                                    child.historical_operation_lease().operation(),
                                    &request_binding,
                                    &attempt,
                                ),
                                provider,
                                &kind,
                                &version,
                            )
                            .unwrap();
                            let resolver = Store(coordinate.clone());
                            let clock = ProviderClock {
                                times: if context_binding.is_some() {
                                    [67, 68, 69]
                                } else {
                                    [66, 67, 68]
                                },
                                next: AtomicUsize::new(0),
                            };
                            let checkpoint_input = |name: &str| OperationCheckpointInput {
                                reference: record_ref(&format!("a2a/provider-{name}-checkpoint")),
                                id: ControlCheckpointIdV1::parse(format!(
                                    "a2a-provider-{name}-checkpoint"
                                ))
                                .unwrap(),
                            };
                            let dispatch_checkpoint = context_binding.as_ref().map_or(
                                (
                                    child_candidate
                                        .checkpoint()
                                        .successor()
                                        .coordinate()
                                        .record_ref(),
                                    &child_checkpoint,
                                ),
                                |(_, reference, checkpoint, _)| (reference, checkpoint),
                            );
                            let dispatch_checkpoint_fields = context_binding
                                .as_ref()
                                .map_or(&child_fields, |(_, _, _, fields)| fields);
                            let retained_dispatch = retain_provider_operation_dispatch(
                                ProviderOperationDispatchInput {
                                    operation: ProviderOperationInput {
                                        context: OperationAttemptContext {
                                            operation: &child,
                                            request: child_request,
                                            session,
                                            current,
                                            lease: child.historical_operation_lease(),
                                            context_binding: context_binding
                                                .as_ref()
                                                .map(|(binding, _, _, _)| binding),
                                            generation,
                                            selection_ref: record_ref("a2a/provider-selection"),
                                            maximum_selection_bytes: 65_536,
                                            observation_ref: record_ref("a2a/provider-resolution"),
                                            observation,
                                        },
                                        invocation,
                                        result_admission: admission,
                                        resolution_ref: record_ref("a2a/provider-resolved"),
                                    },
                                    checkpoint: dispatch_checkpoint,
                                    checkpoint_schema: &dispatch_checkpoint_fields.schema_sha256,
                                    checkpoint_authority: &checkpoint_authority(
                                        dispatch_checkpoint_fields,
                                    ),
                                    signing: observation.signed,
                                    resolution_checkpoint: checkpoint_input("resolved"),
                                    preparation_checkpoint: checkpoint_input("prepared"),
                                    dispatch_checkpoint: checkpoint_input("dispatched"),
                                    dispatch_ref: record_ref("a2a/provider-dispatched"),
                                    clock: &clock,
                                    resolve_store: &resolver,
                                    accumulator_limits: AccumulatorPreparationLimits::new(
                                        1, 10_000, 10_000, 4_096, 100_000,
                                    )
                                    .unwrap(),
                                },
                                &mut ProvingAuthority,
                                *port,
                                *anchor,
                                |_, _, _, _| Ok(()),
                                |_, _, _| Ok(signature()),
                                |_, _, _| Ok(signature()),
                            )
                            .unwrap();
                            assert_eq!(clock.next.load(Ordering::SeqCst), 3);
                            let dispatched_observation = {
                                let (reference, observation) = retained_dispatch.observation();
                                (reference.clone(), observation.clone())
                            };
                            let mut retained_dispatch = Some(retained_dispatch);
                            let recovered_dispatch = if recover_dispatch {
                                let dispatch = dispatched_observation.1.clone();
                                let selection_coordinate = match dispatch.state() {
                                    OperationObservationStateV1::Active {
                                        phase: OperationActivePhaseV1::Dispatched { evidence, .. },
                                    } => ControlRecordCoordinate::new(
                                        evidence.record_ref.clone(),
                                        ControlRecordDigest::parse(evidence.record_sha256.as_str())
                                            .unwrap(),
                                    ),
                                    _ => panic!("provider retention must end at Dispatched"),
                                };
                                drop(retained_dispatch.take());
                                let recovery_authority =
                                    checkpoint_authority(dispatch_checkpoint_fields);
                                let mut reopened = connection(file);
                                let mut reopened_port = SqliteOperationCustody {
                                    connection: &mut reopened,
                                    coordinate: coordinate.clone(),
                                };
                                let selection = reopened_port
                                    .read_operation_record(
                                        &coordinate,
                                        &selection_coordinate,
                                        65_536,
                                    )
                                    .unwrap()
                                    .unwrap();
                                let recovered = recover_operation_observation(
                                    OperationObservationRecoveryInput {
                                        store: &coordinate,
                                        operation: &child,
                                        observation: OperationObservationSelection::SelectedHead,
                                        checkpoint_authority: &recovery_authority,
                                        limits: OperationObservationRecoveryLimits::new(
                                            16,
                                            1 << 20,
                                            65_536,
                                        )
                                        .unwrap(),
                                        checkpoint_limits:
                                            OperationCheckpointAncestryLimits::new(
                                                16,
                                                1 << 20,
                                                16,
                                            )
                                            .unwrap(),
                                        membership_limits:
                                            AccumulatorMembershipLimits::new(
                                                4, 20_000, 200_000,
                                            )
                                            .unwrap(),
                                    },
                                    &mut reopened_port,
                                    *anchor,
                                    OperationObservationRecoveryAdmission {
                                        admit_checkpoint: |_: &ResidentRecordRefV1,
                                                           wire: ControlStateCheckpointWireV1| {
                                            ControlStateCheckpointV1::admit_signed_wire(
                                                wire,
                                                &dispatch_checkpoint_fields.schema_sha256,
                                                &recovery_authority,
                                                |_, _, _, actual| {
                                                    assert_eq!(actual, &signature());
                                                    Ok(())
                                                },
                                            )
                                        },
                                        resolve_signer: |_: &ResidentRecordRefV1,
                                                         _: &OperationObservationWireV1| {
                                            Ok(parents.signing.clone())
                                        },
                                        admit_state: |_: &OperationObservationStateV1| Ok(()),
                                        admit_rotation: |_: &ResidentRecordRefV1,
                                                         _: &ArtifactDigest,
                                                         _: &ResidentRecordRefV1,
                                                         _: &ArtifactDigest| { Ok(()) },
                                        verify_signature: |_: &[u8],
                                                           _: &OperationObservationPreimageV1,
                                                           _: &OperationObservationDigest,
                                                           actual: &CanonicalSignatureV1| {
                                            assert_eq!(actual, &signature());
                                            Ok(())
                                        },
                                    },
                                    |_: &[u8],
                                     _: &ControlStateCheckpointPreimageV1,
                                     _: &ControlStateCheckpointDigest,
                                     actual: &CanonicalSignatureV1| {
                                        assert_eq!(actual, &signature());
                                        Ok(())
                                    },
                                )
                                .unwrap();
                                drop(reopened_port);
                                drop(reopened);
                                Some((dispatch, selection, recovered))
                            } else {
                                None
                            };
                            let send_dispatch = match &recovered_dispatch {
                                Some((dispatch, selection, recovered)) => A2aSendDispatch::recover(
                                    OperationAttemptHistoryInput {
                                        operation: &child,
                                        request: child_request,
                                        session: child.historical_session_lease(),
                                        lease: child.historical_operation_lease(),
                                        generation,
                                        dispatched: dispatch,
                                        selection,
                                        maximum_selection_bytes: 65_536,
                                    },
                                    recovered,
                                )
                                .unwrap(),
                                None => retained_dispatch.take().unwrap().into(),
                            };
                            let replay_dispatch = recovered_dispatch.as_ref().map(
                                |(dispatch, selection, recovered)| {
                                    A2aSendDispatch::recover(
                                        OperationAttemptHistoryInput {
                                            operation: &child,
                                            request: child_request,
                                            session: child.historical_session_lease(),
                                            lease: child.historical_operation_lease(),
                                            generation,
                                            dispatched: dispatch,
                                            selection,
                                            maximum_selection_bytes: 65_536,
                                        },
                                        recovered,
                                    )
                                    .unwrap()
                                },
                            );
                            let provider_ref = ProviderBindingRefV1 {
                                provider_binding_id: if route_provider_match
                                    != RouteProviderMatch::UnlikeId
                                {
                                    provider.id.clone()
                                } else {
                                    "substituted-provider-binding".to_string()
                                },
                                provider_binding_identity: if route_provider_match
                                    != RouteProviderMatch::UnlikeIdentity
                                {
                                    provider.binding_identity.clone()
                                } else {
                                    ProviderBindingIdentityV1 {
                                        provider_binding_sha256: ArtifactDigest::hash_bytes(
                                            b"substituted provider owner",
                                        ),
                                    }
                                },
                            };
                            let route = a2a_custody::route(
                                child.handle().1,
                                parents.signing,
                                provider_ref,
                                "https://agent.example.test/rpc",
                                A2aStreamingAdmissionV1::NotAdvertised,
                            );
                            let expected_tls = ArtifactDigest::hash_bytes(b"selected TLS trust");
                            let actual_tls = if admit_matching_transport {
                                expected_tls.clone()
                            } else {
                                ArtifactDigest::hash_bytes(b"substituted TLS trust")
                            };
                            let disclosures = Arc::new(AtomicUsize::new(0));
                            let prepares = Arc::new(AtomicUsize::new(0));
                            let starts = Arc::new(AtomicUsize::new(0));
                            let exchange = ExchangePort {
                                tls_trust: actual_tls,
                                prepares: Arc::clone(&prepares),
                                starts: Arc::clone(&starts),
                                response,
                            };
                            let now = AmbientSessionAdmissionContext {
                                now: UnixTimeV1::new(69).unwrap(),
                                subject: current.subject,
                                grant: current.grant,
                                current_principal_trust: current.current_principal_trust,
                                capability_revocations: current.capability_revocations,
                                deployment_binding_ref: current.deployment_binding_ref,
                                deployment_binding: current.deployment_binding,
                                deployment_revocations: current.deployment_revocations,
                                runtime_authority: current.runtime_authority,
                                session_revocations: current.session_revocations,
                                route_use: current.route_use,
                                generation: SessionAdmissionLeaseContextV1 {
                                    active_head: current.generation.active_head,
                                    compiled_generation: current.generation.compiled_generation,
                                    effective_profile: current.generation.effective_profile,
                                    activation_fence: current.generation.activation_fence,
                                    signed: current.generation.signed,
                                },
                            };
                            let route_ref = record_ref("a2a/route");
                            let request_id = A2aJsonRpcRequestIdV1::parse("send-1").unwrap();
                            let fields = dispatch_checkpoint_fields.clone();
                            let send_checkpoint_authority =
                                checkpoint_authority(dispatch_checkpoint_fields);
                            let send_clock = match response {
                                ResponseModel::PreCustodyClockUnavailable => {
                                    DelegateClock::unavailable_at(0)
                                }
                                ResponseModel::ReleaseClockUnavailable => {
                                    DelegateClock::unavailable_at(1)
                                }
                                _ => DelegateClock::available(),
                            };
                            let launch = |dispatch,
                                          port: &mut dyn OperationCustodyPort,
                                          anchor: &mut dyn AntiRollbackAnchorPort| {
                                let interception = if matches!(
                                    response,
                                    ResponseModel::CommitUnavailable
                                ) {
                                    CustodyInterception::CommitUnavailable
                                } else {
                                    CustodyInterception::None
                                };
                                let mut intercepted = InterceptingCustody {
                                    inner: port,
                                    interception,
                                };
                                let mut launch_anchor = TerminalAnchor {
                                    inner: anchor,
                                    failure: matches!(
                                        response,
                                        ResponseModel::AnchorUnavailable
                                    )
                                    .then_some(AntiRollbackAnchorErrorKind::Unavailable),
                                };
                                launch_a2a_delegate(
                                    A2aDelegateLaunchInput {
                                        send: A2aSendInput {
                                            dispatch,
                                            current: &now,
                                            route: (&route_ref, &route),
                                            route_predecessor:
                                                OperationCustodyIndexPredecessor::Absent,
                                            route_association: A2aRouteAssociation::Insert,
                                            request_id: &request_id,
                                            message_id: "message-1",
                                            text: "Inspect the selected workspace.",
                                            request_evidence_ref: record_ref("a2a/send-request"),
                                            preparation_ref: record_ref("a2a/send-preparation"),
                                            exchange_limits: exchange_limits(response),
                                            maximum_evidence_bytes: 8_192,
                                            maximum_record_bytes: 65_536,
                                            maximum_dispatch_record_bytes: 65_536,
                                            resolve_store: &resolver,
                                        },
                                        checkpoint: OperationCheckpointInput {
                                            reference: record_ref("a2a/send-checkpoint"),
                                            id: ControlCheckpointIdV1::parse(
                                                "a2a-send-checkpoint",
                                            )
                                            .unwrap(),
                                        },
                                        checkpoint_schema: &fields.schema_sha256,
                                        checkpoint_authority: &send_checkpoint_authority,
                                        response_ref: record_ref("a2a/send-response"),
                                        clock: &send_clock,
                                        accumulator_limits: AccumulatorPreparationLimits::new(
                                            8, 20_000, 20_000, 8_192, 200_000,
                                        )
                                        .unwrap(),
                                        membership_limits: AccumulatorMembershipLimits::new(
                                            8, 20_000, 200_000,
                                        )
                                        .unwrap(),
                                    },
                                    &mut ProvingAuthority,
                                    &mut intercepted,
                                    &mut launch_anchor,
                                    &exchange,
                                    |request, selected, text| {
                                        disclosures.fetch_add(1, Ordering::SeqCst);
                                        assert_eq!(request.provider_binding(), provider);
                                        assert_eq!(selected, &route);
                                        assert_eq!(text, "Inspect the selected workspace.");
                                        Ok(())
                                    },
                                    |evidence, request, trust| {
                                        assert_eq!(evidence, &route.security().evidence);
                                        assert_eq!(request.provider_binding(), provider);
                                        if trust != &expected_tls {
                                            return Err(
                                                InvalidResidentRecordV1::ContextualAdmission {
                                                    record: "A2A transport fixture",
                                                    invariant: "selected TLS trust input",
                                                },
                                            );
                                        }
                                        Ok(())
                                    },
                                    |_, _, _| {
                                        if matches!(response, ResponseModel::CheckpointRejected) {
                                            return Err(
                                                InvalidResidentRecordV1::ContextualAdmission {
                                                    record: "A2A send checkpoint fixture",
                                                    invariant: "accepted checkpoint signature",
                                                },
                                            );
                                        }
                                        Ok(signature())
                                    },
                                )
                            };
                            if matches!(
                                response,
                                ResponseModel::ProcessLostBeforeStart
                                    | ResponseModel::RecoveredResponseClockUnavailable
                                    | ResponseModel::RecoveredResponseClockBackward
                            ) {
                                let prepared = prepare_a2a_send(
                                    A2aSendInput {
                                        dispatch: send_dispatch,
                                        current: &now,
                                        route: (&route_ref, &route),
                                        route_predecessor: OperationCustodyIndexPredecessor::Absent,
                                        route_association: A2aRouteAssociation::Insert,
                                        request_id: &request_id,
                                        message_id: "message-1",
                                        text: "Inspect the selected workspace.",
                                        request_evidence_ref: record_ref("a2a/send-request"),
                                        preparation_ref: record_ref("a2a/send-preparation"),
                                        exchange_limits: exchange_limits(response),
                                        maximum_evidence_bytes: 8_192,
                                        maximum_record_bytes: 65_536,
                                        maximum_dispatch_record_bytes: 65_536,
                                        resolve_store: &resolver,
                                    },
                                    &mut ProvingAuthority,
                                    *port,
                                    *anchor,
                                    &exchange,
                                    |request, selected, text| {
                                        assert_eq!(request.provider_binding(), provider);
                                        assert_eq!(selected, &route);
                                        assert_eq!(text, "Inspect the selected workspace.");
                                        Ok(())
                                    },
                                    |evidence, request, trust| {
                                        assert_eq!(evidence, &route.security().evidence);
                                        assert_eq!(request.provider_binding(), provider);
                                        assert_eq!(trust, &expected_tls);
                                        Ok(())
                                    },
                                )
                                .unwrap();
                                let retained = prepared
                                    .prepare_custody(
                                        OperationCheckpointInput {
                                            reference: record_ref("a2a/send-checkpoint"),
                                            id: ControlCheckpointIdV1::parse("a2a-send-checkpoint")
                                                .unwrap(),
                                        },
                                        UnixTimeV1::new(69).unwrap(),
                                        &fields.schema_sha256,
                                        &send_checkpoint_authority,
                                        AccumulatorPreparationLimits::new(
                                            8, 20_000, 20_000, 8_192, 200_000,
                                        )
                                        .unwrap(),
                                        AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                                            .unwrap(),
                                        *port,
                                        |_, _, _| Ok(signature()),
                                    )
                                    .unwrap()
                                    .commit(*port, *anchor)
                                    .unwrap()
                                    .advance_anchor(*anchor)
                                    .unwrap();
                                let preparation = retained.preparation().clone();
                                drop(retained);
                                assert_eq!(prepares.load(Ordering::SeqCst), 1);
                                assert_eq!(starts.load(Ordering::SeqCst), 0);

                                let selection_coordinate = match dispatched_observation.1.state() {
                                    OperationObservationStateV1::Active {
                                        phase: OperationActivePhaseV1::Dispatched { evidence, .. },
                                    } => ControlRecordCoordinate::new(
                                        evidence.record_ref.clone(),
                                        ControlRecordDigest::parse(evidence.record_sha256.as_str())
                                            .unwrap(),
                                    ),
                                    _ => panic!("provider retention must end at Dispatched"),
                                };
                                let mut reopened = connection(file);
                                let mut reopened_port = SqliteOperationCustody {
                                    connection: &mut reopened,
                                    coordinate: coordinate.clone(),
                                };
                                let selection = reopened_port
                                    .read_operation_record(
                                        &coordinate,
                                        &selection_coordinate,
                                        65_536,
                                    )
                                    .unwrap()
                                    .unwrap();
                                let recovered = recover_historical_operation_observation(
                                    &child,
                                    (&dispatched_observation.0, &dispatched_observation.1),
                                    &fields,
                                    parents.signing,
                                    &mut reopened_port,
                                    *anchor,
                                );
                                let recovery_authority = checkpoint_authority(&fields);
                                let changes_before_recovery =
                                    reopened_port.connection.total_changes();
                                let mut recover_unknown =
                                    |custody: &mut dyn OperationCustodyPort| {
                                        let mut admit_checkpoint =
                                            |_: &ResidentRecordRefV1,
                                             wire: ControlStateCheckpointWireV1| {
                                                ControlStateCheckpointV1::admit_signed_wire(
                                                    wire,
                                                    &fields.schema_sha256,
                                                    &recovery_authority,
                                                    |_, _, _, actual| {
                                                        assert_eq!(actual, &signature());
                                                        Ok(())
                                                    },
                                                )
                                            };
                                        let mut admit_route =
                                            |_: &ResidentRecordRefV1,
                                             wire: A2aRouteAdmissionWireV1| {
                                                A2aRouteAdmissionV1::admit_contextual_wire(
                                                    wire,
                                                    &A2aRouteAdmissionContextV1 {
                                                        signed:
                                                            SignedResidentAdmissionContextV1 {
                                                                expected_schema_sha256:
                                                                    route.schema_sha256(),
                                                                signing_authority: parents.signing,
                                                            },
                                                        provider_binding: route.provider_binding(),
                                                        route: route.route(),
                                                    },
                                                    |_| Ok(()),
                                                    |_, _, _, actual| {
                                                        assert_eq!(actual, &signature());
                                                        Ok(())
                                                    },
                                                )
                                            };
                                        recover_a2a_send_outcome_unknown(
                                            A2aSendRecoveryInput {
                                                history: OperationAttemptHistoryInput {
                                                    operation: &child,
                                                    request: child_request,
                                                    session: child.historical_session_lease(),
                                                    lease: child.historical_operation_lease(),
                                                    generation,
                                                    dispatched: &dispatched_observation.1,
                                                    selection: &selection,
                                                    maximum_selection_bytes: 65_536,
                                                },
                                                recovered: &recovered,
                                                limits: A2aSendRecoveryLimits::new(
                                                    65_536,
                                                    8_192,
                                                    OperationCheckpointAncestryLimits::new(
                                                        16,
                                                        1 << 20,
                                                        16,
                                                    )
                                                    .unwrap(),
                                                    AccumulatorMembershipLimits::new(
                                                        8, 20_000, 200_000,
                                                    )
                                                    .unwrap(),
                                                )
                                                .unwrap(),
                                            },
                                            custody,
                                            *anchor,
                                            A2aSendRecoveryAdmission {
                                                admit_checkpoint: &mut admit_checkpoint,
                                                admit_route: &mut admit_route,
                                            },
                                        )
                                    };
                                for substitution in [
                                    CustodyInterception::Preparation,
                                    CustodyInterception::Checkpoint,
                                    CustodyInterception::Evidence,
                                ] {
                                    let mut substituted = InterceptingCustody {
                                        inner: &mut reopened_port,
                                        interception: substitution,
                                    };
                                    assert!(recover_unknown(&mut substituted).is_err());
                                }
                                let unknown = recover_unknown(&mut reopened_port).unwrap();
                                let exact_replay = recover_unknown(&mut reopened_port).unwrap();
                                assert_eq!(exact_replay.preparation(), unknown.preparation());
                                assert_eq!(exact_replay.route(), unknown.route());
                                assert_eq!(
                                    exact_replay.request_evidence(),
                                    unknown.request_evidence()
                                );
                                assert_eq!(
                                    exact_replay.diagnostic().canonical_bytes(),
                                    unknown.diagnostic().canonical_bytes()
                                );
                                assert_eq!(
                                    reopened_port.connection.total_changes(),
                                    changes_before_recovery,
                                    "exact recovery replay must remain read-only"
                                );
                                assert_eq!(
                                    unknown.diagnostic().wire().effect,
                                    DiagnosticEffectCertaintyV1::OutcomeUnknown
                                );
                                assert_eq!(
                                    unknown.diagnostic().wire().retry,
                                    DiagnosticRetryV1::ReconcileFirst
                                );
                                assert_eq!(unknown.preparation(), &preparation);
                                assert_eq!(starts.load(Ordering::SeqCst), 0);

                                let reconciliation = admitted_reconciliation_frontier(
                                    parents.signing,
                                    preparation.record_ref().clone(),
                                    ArtifactDigest::parse(preparation.record_sha256().as_str())
                                        .unwrap(),
                                );
                                let reconciliation_ref = ReconciliationFrontierReferenceV1 {
                                    record_ref: record_ref("a2a/recovered-send-reconciliation"),
                                    record_sha256: reconciliation
                                        .canonical_reconciliation_frontier_sha256()
                                        .clone(),
                                };
                                let outcome_checkpoint_authority = checkpoint_authority(&fields);
                                let backward_recovery_clock =
                                    OneTimeClock(UnixTimeV1::new(68).unwrap());
                                let forward_recovery_clock =
                                    OneTimeClock(UnixTimeV1::new(70).unwrap());
                                let recovery_clock: &dyn WallClockSourcePort = match response {
                                    ResponseModel::RecoveredResponseClockUnavailable => {
                                        &UnavailableClock
                                    }
                                    ResponseModel::RecoveredResponseClockBackward => {
                                        &backward_recovery_clock
                                    }
                                    _ => &forward_recovery_clock,
                                };
                                let checkpoint_clock = OneTimeClock(UnixTimeV1::new(70).unwrap());
                                let retained = retain_a2a_outcome(
                                    A2aOutcomeRetentionInput::RecoveredOutcomeUnknown(
                                        A2aRecoveredUnknownOutcomeRetention::new(
                                            Box::new(unknown),
                                            A2aOutcomeUnknownObservationInput {
                                                diagnostic_ref: record_ref(
                                                    "a2a/recovered-send-diagnostic",
                                                ),
                                                observation_ref: record_ref(
                                                    "a2a/recovered-send-observation",
                                                ),
                                                reconciliation: reconciliation_ref.clone(),
                                                clock: recovery_clock,
                                                signing: observation.signed,
                                                maximum_diagnostic_bytes: 65_536,
                                            },
                                            A2aSendObservationCommitInput {
                                                checkpoint: OperationCheckpointInput {
                                                    reference: record_ref(
                                                        "a2a/recovered-send-outcome-checkpoint/record",
                                                    ),
                                                    id: ControlCheckpointIdV1::parse(
                                                        "a2a-recovered-send-outcome-checkpoint",
                                                    )
                                                    .unwrap(),
                                                },
                                                clock: &checkpoint_clock,
                                                checkpoint_schema: &fields.schema_sha256,
                                                checkpoint_authority:
                                                    &outcome_checkpoint_authority,
                                                accumulator_limits:
                                                    AccumulatorPreparationLimits::new(
                                                        8, 20_000, 20_000, 8_192, 200_000,
                                                    )
                                                    .unwrap(),
                                            },
                                            &resolver,
                                            |actual| {
                                                assert_eq!(actual, &reconciliation_ref);
                                                Ok(())
                                            },
                                            |_, _, _, _| Ok(()),
                                            |_, _, _| Ok(signature()),
                                            |_, _, _| Ok(signature()),
                                        ),
                                    ),
                                    &mut reopened_port,
                                    *anchor,
                                );
                                let retained = match retained {
                                    Ok(retained) => retained,
                                    Err(
                                        A2aOutcomeRetentionFailure::RecoveredOutcomeUnknownObservation(
                                            failure,
                                        ),
                                    ) if matches!(
                                        response,
                                        ResponseModel::RecoveredResponseClockUnavailable
                                            | ResponseModel::RecoveredResponseClockBackward
                                    ) => {
                                        assert!(matches!(
                                            failure.reason(),
                                            OperationAttemptError::Clock(_)
                                                | OperationAttemptError::TimeOrder
                                        ));
                                        assert_eq!(starts.load(Ordering::SeqCst), 0);
                                        return;
                                    }
                                    Err(other) => {
                                        panic!("unexpected recovered-send retention failure: {other:?}")
                                    }
                                };
                                let RetainedA2aOutcome::OutcomeUnknown(completed) = retained else {
                                    panic!("recovered send must retain outcome unknown")
                                };
                                assert!(matches!(
                                    completed.observation().1.state(),
                                    OperationObservationStateV1::OutcomeUnknown {
                                        reconciliation,
                                        effect_receipt: None,
                                        ..
                                    } if reconciliation == &reconciliation_ref
                                ));
                                assert!(completed.response().is_none());
                                assert_eq!(
                                    completed.diagnostic().wire().effect,
                                    DiagnosticEffectCertaintyV1::OutcomeUnknown
                                );
                                let candidate = completed.custody().candidate();
                                assert_eq!(
                                    candidate
                                        .records()
                                        .iter()
                                        .filter(|record| {
                                            record.kind()
                                                == OperationCustodyRecordKind::OperationObservation
                                        })
                                        .count(),
                                    1
                                );
                                assert!(candidate
                                    .record(OperationCustodyRecordKind::Diagnostic)
                                    .is_some());
                                let observation_record = candidate
                                    .record(OperationCustodyRecordKind::OperationObservation)
                                    .unwrap()
                                    .clone();
                                let latest_key = candidate
                                    .indexes()
                                    .iter()
                                    .find_map(|index| match index {
                                        OperationCustodyIndexMutation::LatestObservation {
                                            key,
                                            ..
                                        } => Some(key.clone()),
                                        _ => None,
                                    })
                                    .unwrap();
                                assert_eq!(
                                    reopened_port
                                        .read_index(
                                            &coordinate,
                                            &OperationCustodyIndexQuery::LatestObservation(
                                                latest_key,
                                            ),
                                            65_536,
                                        )
                                        .unwrap()
                                        .into_record(),
                                    Some(observation_record)
                                );
                                assert_eq!(starts.load(Ordering::SeqCst), 0);
                                return;
                            }

                            let launched = launch(send_dispatch, *port, *anchor);
                            let replay_launch =
                                replay_dispatch.map(|dispatch| launch(dispatch, *port, *anchor));
                            if matches!(
                                response,
                                ResponseModel::PreCustodyClockUnavailable
                                    | ResponseModel::ReleaseClockUnavailable
                                    | ResponseModel::CheckpointRejected
                                    | ResponseModel::CommitUnavailable
                                    | ResponseModel::AnchorUnavailable
                            ) {
                                match (response, launched) {
                                    (
                                        ResponseModel::PreCustodyClockUnavailable,
                                        Err(A2aDelegateLaunchFailure::PreCustodyClock(_)),
                                    ) => {
                                        assert_eq!(send_clock.calls.load(Ordering::SeqCst), 1);
                                    }
                                    (
                                        ResponseModel::ReleaseClockUnavailable,
                                        Err(A2aDelegateLaunchFailure::ReleaseClock {
                                            retained,
                                            reason: _,
                                        }),
                                    ) => {
                                        assert_eq!(send_clock.calls.load(Ordering::SeqCst), 2);
                                        assert_eq!(
                                            retained.custody().database_evidence(),
                                            OperationCustodyDatabaseEvidence::New
                                        );
                                        assert_eq!(
                                            retained.preparation().record_ref(),
                                            &record_ref("a2a/send-preparation")
                                        );
                                    }
                                    (
                                        ResponseModel::CheckpointRejected,
                                        Err(A2aDelegateLaunchFailure::Checkpoint(
                                            A2aSendCheckpointError::Admission(
                                                InvalidResidentRecordV1::ContextualAdmission {
                                                    ..
                                                },
                                            ),
                                        )),
                                    ) => {
                                        assert_eq!(send_clock.calls.load(Ordering::SeqCst), 1);
                                    }
                                    (
                                        ResponseModel::CommitUnavailable,
                                        Err(A2aDelegateLaunchFailure::Custody(failure)),
                                    ) => {
                                        assert!(failure.is_commit_unresolved());
                                        assert_eq!(send_clock.calls.load(Ordering::SeqCst), 1);
                                    }
                                    (
                                        ResponseModel::AnchorUnavailable,
                                        Err(A2aDelegateLaunchFailure::Custody(failure)),
                                    ) => {
                                        assert!(failure.is_anchor_unresolved());
                                        assert_eq!(send_clock.calls.load(Ordering::SeqCst), 1);
                                    }
                                    _ => panic!("unexpected delegate launch phase"),
                                }
                                assert_eq!(starts.load(Ordering::SeqCst), 0);
                                return;
                            }
                            if route_provider_match != RouteProviderMatch::Exact {
                                assert!(matches!(
                                    launched,
                                    Err(A2aDelegateLaunchFailure::Preparation(
                                        A2aSendPreparationError::Association
                                    ))
                                ));
                                assert_eq!(disclosures.load(Ordering::SeqCst), 0);
                                assert_eq!(prepares.load(Ordering::SeqCst), 0);
                                assert_eq!(starts.load(Ordering::SeqCst), 0);
                                return;
                            }
                            assert_eq!(
                                prepares.load(Ordering::SeqCst),
                                1,
                                "a committed delegate rejects replay before carrier preparation"
                            );
                            if !admit_matching_transport {
                                assert!(matches!(
                                    launched,
                                    Err(A2aDelegateLaunchFailure::Preparation(
                                        A2aSendPreparationError::Operation(_)
                                    ))
                                ));
                                assert_eq!(starts.load(Ordering::SeqCst), 0);
                                return;
                            }
                            if let Some(replay_launch) = replay_launch {
                                if let Err(failure) = replay_launch {
                                    if !matches!(
                                        failure,
                                        A2aDelegateLaunchFailure::Preparation(
                                            A2aSendPreparationError::Operation(_)
                                        )
                                    ) {
                                        panic!("unexpected replay disposition: {failure:?}");
                                    }
                                } else {
                                    panic!("a committed delegate must reject replay");
                                }
                            }
                            let awaiting = match launched {
                                Ok(awaiting) => {
                                    assert!(!matches!(
                                        response,
                                        ResponseModel::StartRejected
                                            | ResponseModel::RetentionTooShort
                                    ));
                                    awaiting
                                }
                                Err(A2aDelegateLaunchFailure::Start(
                                    A2aSendStartFailure::Preflight { retained, reason },
                                )) => {
                                    assert!(matches!(response, ResponseModel::RetentionTooShort));
                                    assert!(matches!(
                                        reason,
                                        A2aSendReleaseError::InsufficientRetention
                                    ));
                                    let candidate = retained.custody().candidate();
                                    assert_eq!(
                                        retained.preparation(),
                                        candidate
                                            .record(OperationCustodyRecordKind::A2aSendPreparation,)
                                            .unwrap()
                                            .coordinate()
                                    );
                                    assert_eq!(starts.load(Ordering::SeqCst), 0);
                                    return;
                                }
                                Err(A2aDelegateLaunchFailure::Start(
                                    A2aSendStartFailure::Rejected(not_started),
                                )) => {
                                    assert!(matches!(response, ResponseModel::StartRejected));
                                    let candidate = not_started.send_custody().candidate().clone();
                                    assert_eq!(
                                        not_started.reason(),
                                        A2aExchangeError::ServiceUnavailable
                                    );
                                    assert_eq!(
                                        not_started.evidence().0.wire().code,
                                        "a2a_send_not_started"
                                    );
                                    assert_eq!(
                                        not_started.evidence().0.wire().effect,
                                        DiagnosticEffectCertaintyV1::NotStarted
                                    );
                                    assert_eq!(
                                        not_started.evidence().0.wire().retry,
                                        DiagnosticRetryV1::NotIndicated
                                    );
                                    let outcome_checkpoint_authority =
                                        checkpoint_authority(&fields);
                                    let retained = retain_a2a_outcome(
                                        A2aOutcomeRetentionInput::NotStarted(
                                            A2aNotStartedOutcomeRetention::new(
                                                not_started,
                                                A2aFailedSendObservationInput {
                                                diagnostic_ref: record_ref(
                                                    "a2a/send-not-started-diagnostic",
                                                ),
                                                observation_ref: record_ref(
                                                    "a2a/send-not-started-observation",
                                                ),
                                                clock: &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                                signing: observation.signed,
                                                maximum_diagnostic_bytes: 65_536,
                                            },
                                                A2aSendObservationCommitInput {
                                                    checkpoint: OperationCheckpointInput {
                                                        reference: record_ref(
                                                            "a2a-send-not-started-checkpoint/record",
                                                        ),
                                                        id: ControlCheckpointIdV1::parse(
                                                            "a2a-send-not-started-checkpoint",
                                                        )
                                                        .unwrap(),
                                                    },
                                                    clock: &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                                    checkpoint_schema: &fields.schema_sha256,
                                                    checkpoint_authority:
                                                        &outcome_checkpoint_authority,
                                                    accumulator_limits:
                                                        AccumulatorPreparationLimits::new(
                                                            8, 20_000, 20_000, 8_192, 200_000,
                                                        )
                                                        .unwrap(),
                                                },
                                                &resolver,
                                                |_, _, _, _| Ok(()),
                                                |_, _, _| Ok(signature()),
                                                |_, _, _| Ok(signature()),
                                            ),
                                        ),
                                        *port,
                                        *anchor,
                                    )
                                    .unwrap();
                                    let RetainedA2aOutcome::NotStarted(retained) = retained else {
                                        panic!("queue rejection changed retention branch")
                                    };
                                    assert!(matches!(
                                        retained.observation().1.state(),
                                        OperationObservationStateV1::Failed { result: None, .. }
                                    ));
                                    assert_eq!(
                                        retained
                                            .checkpoint_preimage()
                                            .previous_control_checkpoint_ref
                                            .as_ref(),
                                        Some(&record_ref("a2a/send-checkpoint")),
                                        "retention must succeed the send checkpoint exactly once"
                                    );
                                    let terminal = retained.custody().candidate();
                                    assert!(terminal.a2a_evidence().is_empty());
                                    assert!(terminal
                                        .record(OperationCustodyRecordKind::Diagnostic)
                                        .is_some());
                                    assert_eq!(starts.load(Ordering::SeqCst), 1);
                                    assert_eq!(
                                        port.compare_and_commit(&candidate).unwrap_err().kind(),
                                        ControlStoreErrorKind::CandidateConflict,
                                        "carrier rejection cannot restore the consumed send preparation"
                                    );
                                    return;
                                }
                                Err(other) => {
                                    panic!("unexpected delegate-launch failure: {other:?}")
                                }
                            };
                            assert_eq!(
                                awaiting.send_custody().database_evidence(),
                                OperationCustodyDatabaseEvidence::New
                            );
                            let candidate = awaiting.send_custody().candidate().clone();
                            assert_eq!(starts.load(Ordering::SeqCst), 1);
                            assert_eq!(
                                awaiting.coordinate().request_sha256().as_str(),
                                candidate.a2a_evidence()[0]
                                    .coordinate()
                                    .record_sha256()
                                    .as_str()
                            );
                            assert_eq!(
                                awaiting.send_checkpoint().0,
                                &record_ref("a2a/send-checkpoint")
                            );
                            let mut cx = poll_context();
                            if matches!(response, ResponseModel::TaskWithPendingEof) {
                                let awaiting = match awaiting.poll(
                                    &mut cx,
                                    &pale_adapter_a2a::A2aSendMessageDecoder,
                                    &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                ) {
                                    A2aSendResponsePoll::Pending(awaiting) => awaiting,
                                    A2aSendResponsePoll::Ready(_) => {
                                        panic!("the delegate driver must preserve response Pending")
                                    }
                                };
                                assert!(matches!(&awaiting, AwaitingA2aDelegate::Response(_)));
                                let awaiting = match awaiting.poll(
                                    &mut cx,
                                    &pale_adapter_a2a::A2aSendMessageDecoder,
                                    &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                ) {
                                    A2aSendResponsePoll::Pending(awaiting) => awaiting,
                                    A2aSendResponsePoll::Ready(_) => {
                                        panic!("the delegate driver must preserve EOF Pending")
                                    }
                                };
                                assert!(matches!(&awaiting, AwaitingA2aDelegate::Eof(_)));
                                let completed = match awaiting.poll(
                                    &mut cx,
                                    &pale_adapter_a2a::A2aSendMessageDecoder,
                                    &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                ) {
                                    A2aSendResponsePoll::Ready(Ok(completed)) => completed,
                                    A2aSendResponsePoll::Pending(_) => {
                                        panic!("the exact unary EOF remained pending")
                                    }
                                    A2aSendResponsePoll::Ready(Err(error)) => {
                                        panic!("the delegate driver rejected exact EOF: {error:?}")
                                    }
                                };
                                assert!(matches!(completed, CompletedA2aSendResponse::Task(_)));
                                assert_eq!(starts.load(Ordering::SeqCst), 1);
                                return;
                            }
                            let awaiting = match awaiting {
                                AwaitingA2aDelegate::Response(response) => response,
                                AwaitingA2aDelegate::Eof(_) => {
                                    unreachable!("a fresh launch awaits its first response")
                                }
                            };
                            let coordinate_before_pending = awaiting.coordinate().clone();
                            let checkpoint_before_pending = (
                                awaiting.send_checkpoint().0.clone(),
                                awaiting.send_checkpoint().1.clone(),
                            );
                            let awaiting = match awaiting
                                .poll_response(&mut cx, &pale_adapter_a2a::A2aSendMessageDecoder)
                            {
                                A2aSendResponsePoll::Pending(awaiting) => {
                                    let mut resident_slot = store_resident_pending(awaiting);
                                    let awaiting = resident_slot
                                        .take()
                                        .expect("the resident owns exactly one pending phase");
                                    assert!(awaiting.coordinate() == &coordinate_before_pending);
                                    assert!(awaiting.send_custody().candidate() == &candidate);
                                    assert_eq!(
                                        awaiting.send_checkpoint(),
                                        (
                                            &checkpoint_before_pending.0,
                                            &checkpoint_before_pending.1
                                        ),
                                        "pending storage must preserve the exact admitted frontier"
                                    );
                                    awaiting
                                }
                                A2aSendResponsePoll::Ready(_) => {
                                    panic!("the wake-driven carrier must suspend before response")
                                }
                            };
                            let received = match awaiting
                                .poll_response(&mut cx, &pale_adapter_a2a::A2aSendMessageDecoder)
                            {
                                A2aSendResponsePoll::Ready(Ok(value)) => Some(value),
                                A2aSendResponsePoll::Ready(Err(
                                    A2aSendResponseFailure::OutcomeUnknown(unknown),
                                )) => {
                                    assert!(matches!(response, ResponseModel::Eof));
                                    assert_eq!(
                                        unknown.diagnostic().wire().disposition,
                                        DiagnosticDispositionV1::EffectOutcomeUnknown
                                    );
                                    assert_eq!(
                                        unknown.diagnostic().wire().retry,
                                        DiagnosticRetryV1::ReconcileFirst
                                    );
                                    assert!(unknown.response_evidence().is_none());
                                    let preparation = candidate
                                        .record(OperationCustodyRecordKind::A2aSendPreparation)
                                        .unwrap()
                                        .coordinate();
                                    let frontier = admitted_reconciliation_frontier(
                                        parents.signing,
                                        preparation.record_ref().clone(),
                                        ArtifactDigest::parse(preparation.record_sha256().as_str())
                                            .unwrap(),
                                    );
                                    let frontier_reference = ReconciliationFrontierReferenceV1 {
                                        record_ref: record_ref("a2a/send-reconciliation"),
                                        record_sha256: frontier
                                            .canonical_reconciliation_frontier_sha256()
                                            .clone(),
                                    };
                                    let rejected_checkpoint_authority =
                                        checkpoint_authority(&fields);
                                    let rejected = match retain_a2a_outcome(
                                        A2aOutcomeRetentionInput::OutcomeUnknown(
                                            A2aUnknownOutcomeRetention::new(
                                                unknown,
                                                A2aOutcomeUnknownObservationInput {
                                            diagnostic_ref: record_ref(
                                                "a2a/send-unknown-rejected-diagnostic",
                                            ),
                                            observation_ref: record_ref(
                                                "a2a/send-unknown-rejected-observation",
                                            ),
                                            reconciliation: ReconciliationFrontierReferenceV1 {
                                                record_ref: record_ref(
                                                    "a2a/unretained-reconciliation",
                                                ),
                                                record_sha256: ReconciliationFrontierDigest::parse(
                                                    "f".repeat(64),
                                                )
                                                .unwrap(),
                                            },
                                            clock: &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                            signing: observation.signed,
                                            maximum_diagnostic_bytes: 65_536,
                                        },
                                                A2aSendObservationCommitInput {
                                                    checkpoint: OperationCheckpointInput {
                                                        reference: record_ref(
                                                            "a2a-send-unknown-rejected-checkpoint/record",
                                                        ),
                                                        id: ControlCheckpointIdV1::parse(
                                                            "a2a-send-unknown-rejected-checkpoint",
                                                        )
                                                        .unwrap(),
                                                    },
                                                    clock: &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                                    checkpoint_schema: &fields.schema_sha256,
                                                    checkpoint_authority:
                                                        &rejected_checkpoint_authority,
                                                    accumulator_limits:
                                                        AccumulatorPreparationLimits::new(
                                                            8, 20_000, 20_000, 8_192, 200_000,
                                                        )
                                                        .unwrap(),
                                                },
                                                &resolver,
                                        |_| {
                                            Err(InvalidResidentRecordV1::ContextualAdmission {
                                                record: "reconciliation frontier",
                                                invariant: "exact retained frontier",
                                            })
                                        },
                                        |_, _, _, _| Ok(()),
                                        |_, _, _| Ok(signature()),
                                                |_, _, _| Ok(signature()),
                                            ),
                                        ),
                                        *port,
                                        *anchor,
                                    ) {
                                        Err(
                                            A2aOutcomeRetentionFailure::OutcomeUnknownObservation(
                                                rejected,
                                            ),
                                        ) => rejected,
                                        Err(other) => panic!(
                                            "unexpected reconciliation-substitution failure: {other:?}"
                                        ),
                                        Ok(_) => panic!(
                                            "an unretained reconciliation frontier was admitted"
                                        ),
                                    };
                                    assert!(matches!(
                                        rejected.reason(),
                                        OperationAttemptError::Admission(
                                            InvalidResidentRecordV1::ContextualAdmission {
                                                invariant: "exact retained frontier",
                                                ..
                                            }
                                        )
                                    ));
                                    let unknown = rejected.into_outcome();
                                    let outcome_checkpoint_authority =
                                        checkpoint_authority(&fields);
                                    let retained = retain_a2a_outcome(
                                        A2aOutcomeRetentionInput::OutcomeUnknown(
                                            A2aUnknownOutcomeRetention::new(
                                                Box::new(unknown),
                                            A2aOutcomeUnknownObservationInput {
                                                diagnostic_ref: record_ref(
                                                    "a2a/send-unknown-diagnostic",
                                                ),
                                                observation_ref: record_ref(
                                                    "a2a/send-unknown-observation",
                                                ),
                                                reconciliation: frontier_reference.clone(),
                                                clock: &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                                signing: observation.signed,
                                                maximum_diagnostic_bytes: 65_536,
                                            },
                                                A2aSendObservationCommitInput {
                                                    checkpoint: OperationCheckpointInput {
                                                        reference: record_ref(
                                                            "a2a-send-unknown-checkpoint/record",
                                                        ),
                                                        id: ControlCheckpointIdV1::parse(
                                                            "a2a-send-unknown-checkpoint",
                                                        )
                                                        .unwrap(),
                                                    },
                                                    clock: &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                                    checkpoint_schema: &fields.schema_sha256,
                                                    checkpoint_authority:
                                                        &outcome_checkpoint_authority,
                                                    accumulator_limits:
                                                        AccumulatorPreparationLimits::new(
                                                            8, 20_000, 20_000, 8_192, 200_000,
                                                        )
                                                        .unwrap(),
                                                },
                                                &resolver,
                                            |actual| {
                                                if actual != &frontier_reference {
                                                    return Err(
                                                        InvalidResidentRecordV1::ContextualAdmission {
                                                            record: "reconciliation frontier",
                                                            invariant: "exact retained frontier",
                                                        },
                                                    );
                                                }
                                                assert_eq!(
                                                    actual.record_sha256,
                                                    *frontier
                                                        .canonical_reconciliation_frontier_sha256()
                                                );
                                                Ok(())
                                            },
                                            |_, _, _, _| Ok(()),
                                            |_, _, _| Ok(signature()),
                                                |_, _, _| Ok(signature()),
                                            ),
                                        ),
                                        *port,
                                        *anchor,
                                    )
                                    .unwrap();
                                    let RetainedA2aOutcome::OutcomeUnknown(retained) = retained
                                    else {
                                        panic!("accepted uncertainty changed retention branch")
                                    };
                                    assert!(matches!(
                                        retained.observation().1.state(),
                                        OperationObservationStateV1::OutcomeUnknown { .. }
                                    ));
                                    assert_eq!(
                                        retained
                                            .checkpoint_preimage()
                                            .previous_control_checkpoint_ref
                                            .as_ref(),
                                        Some(&record_ref("a2a/send-checkpoint")),
                                        "uncertainty must succeed the send checkpoint exactly once"
                                    );
                                    let terminal_checkpoint = retained.checkpoint().1.clone();
                                    let terminal = retained.custody().candidate();
                                    assert!(terminal.a2a_evidence().is_empty());
                                    assert!(terminal
                                        .record(OperationCustodyRecordKind::Diagnostic)
                                        .is_some());
                                    assert!(port
                                        .read_a2a_evidence_inserted_at(
                                            &coordinate,
                                            terminal
                                                .checkpoint()
                                                .successor()
                                                .committed_transaction_sequence(),
                                            1,
                                            8_192,
                                        )
                                        .unwrap()
                                        .is_empty());

                                    // Lose every in-memory operation phase before the late
                                    // owner response arrives. The new connection must recover
                                    // both the historical authorized send and its current
                                    // outcome-unknown head; it must not reconstruct release
                                    // authority for `SendMessage`.
                                    let mut task_reopened = connection(file);
                                    let mut task_port = SqliteOperationCustody {
                                        connection: &mut task_reopened,
                                        coordinate: coordinate.clone(),
                                    };
                                    let task_dispatch =
                                        (&dispatched_observation.0, &dispatched_observation.1);
                                    let task_proof = recover_historical_operation_observation(
                                        &child,
                                        task_dispatch,
                                        &fields,
                                        parents.signing,
                                        &mut task_port,
                                        *anchor,
                                    );
                                    assert!(matches!(
                                        task_proof.selected_head().1.state(),
                                        OperationObservationStateV1::OutcomeUnknown { .. }
                                    ));
                                    assert_eq!(task_proof.selected(), task_dispatch);

                                    let request_evidence = candidate.a2a_evidence()[0].clone();
                                    let response = task_response_evidence();
                                    let response_evidence = OperationA2aEvidence::new(
                                        ImmutableRecordReferenceV1 {
                                            record_ref: record_ref("a2a/late-send-response"),
                                            record_sha256: ArtifactDigest::hash_bytes(
                                                &response.bytes,
                                            ),
                                        },
                                        response.bytes,
                                        response.sse,
                                        8_192,
                                    )
                                    .unwrap();
                                    let mut inputs = a2a_custody::fixture::Inputs::new(
                                        child.handle(),
                                        task_dispatch,
                                        (&route_ref, &route),
                                        ImmutableRecordReferenceV1 {
                                            record_ref: request_evidence
                                                .coordinate()
                                                .record_ref()
                                                .clone(),
                                            record_sha256: ArtifactDigest::parse(
                                                request_evidence
                                                    .coordinate()
                                                    .record_sha256()
                                                    .as_str(),
                                            )
                                            .unwrap(),
                                        },
                                        ImmutableRecordReferenceV1 {
                                            record_ref: response_evidence
                                                .coordinate()
                                                .record_ref()
                                                .clone(),
                                            record_sha256: ArtifactDigest::parse(
                                                response_evidence
                                                    .coordinate()
                                                    .record_sha256()
                                                    .as_str(),
                                            )
                                            .unwrap(),
                                        },
                                        80,
                                    );
                                    inputs.binding.json_rpc_request_id = request_id.clone();
                                    inputs.observation.task_binding_sha256 =
                                        A2aTaskBindingDigest::hash_canonical(&inputs.binding)
                                            .unwrap();
                                    assert!(
                                        inputs.binding.bound_at > UnixTimeV1::new(75).unwrap(),
                                        "late Task identity is discovered after route expiry"
                                    );
                                    let route_record = candidate
                                        .record(OperationCustodyRecordKind::A2aRouteAdmission)
                                        .unwrap()
                                        .clone();
                                    let writes_before_missing_request_evidence =
                                        task_port.connection.total_changes();
                                    let binding_signatures = AtomicUsize::new(0);
                                    let observation_signatures = AtomicUsize::new(0);
                                    assert!(matches!(
                                        prepare_a2a_task_binding_custody(
                                            A2aTaskBindingCustodyInput {
                                                operation_custody: &task_proof,
                                                binding_ref: inputs.binding_ref.clone(),
                                                binding: inputs.binding.clone(),
                                                binding_context: inputs.binding_context(
                                                    child.handle(),
                                                    task_dispatch,
                                                    &route,
                                                    parents.signing,
                                                ),
                                                observation_ref: inputs.observation_ref.clone(),
                                                observation: inputs.observation.clone(),
                                                observation_signing:
                                                    SignedResidentAdmissionContextV1 {
                                                        expected_schema_sha256: &inputs
                                                            .observation
                                                            .schema_sha256,
                                                        signing_authority: parents.signing,
                                                    },
                                                route_predecessor:
                                                    OperationCustodyIndexPredecessor::Exact(
                                                        route_record.clone(),
                                                    ),
                                                route_association: A2aRouteAssociation::Retained,
                                                new_evidence: vec![response_evidence.clone()],
                                                retained_evidence: Vec::new(),
                                                maximum_evidence_bytes: 8_192,
                                                maximum_record_bytes: 65_536,
                                            },
                                            |_, _, _, _| {
                                                panic!(
                                                    "missing authorized request evidence must fail before admission"
                                                )
                                            },
                                            |_, _, _| {
                                                binding_signatures.fetch_add(1, Ordering::SeqCst);
                                                Ok(signature())
                                            },
                                            |_, _, _| {
                                                observation_signatures
                                                    .fetch_add(1, Ordering::SeqCst);
                                                Ok(signature())
                                            },
                                            |_| {
                                                panic!(
                                                    "missing authorized request evidence must fail before store resolution"
                                                )
                                            },
                                        ),
                                        Err(A2aTaskCustodyError::Evidence)
                                    ));
                                    assert_eq!(
                                        binding_signatures.load(Ordering::SeqCst),
                                        0,
                                        "missing send evidence cannot authorize a binding signature"
                                    );
                                    assert_eq!(
                                        observation_signatures.load(Ordering::SeqCst),
                                        0,
                                        "missing send evidence cannot authorize an observation signature"
                                    );
                                    assert_eq!(
                                        task_port.connection.total_changes(),
                                        writes_before_missing_request_evidence,
                                        "rejected late evidence cannot alter durable custody"
                                    );
                                    assert!(matches!(
                                        task_proof.selected_head().1.state(),
                                        OperationObservationStateV1::OutcomeUnknown { .. }
                                    ));
                                    assert_eq!(
                                        starts.load(Ordering::SeqCst),
                                        1,
                                        "rejected late evidence cannot replay SendMessage"
                                    );
                                    let append = prepare_a2a_task_binding_custody(
                                        A2aTaskBindingCustodyInput {
                                            operation_custody: &task_proof,
                                            binding_ref: inputs.binding_ref.clone(),
                                            binding: inputs.binding.clone(),
                                            binding_context: inputs.binding_context(
                                                child.handle(),
                                                task_dispatch,
                                                &route,
                                                parents.signing,
                                            ),
                                            observation_ref: inputs.observation_ref.clone(),
                                            observation: inputs.observation.clone(),
                                            observation_signing: SignedResidentAdmissionContextV1 {
                                                expected_schema_sha256: &inputs
                                                    .observation
                                                    .schema_sha256,
                                                signing_authority: parents.signing,
                                            },
                                            route_predecessor:
                                                OperationCustodyIndexPredecessor::Exact(
                                                    route_record,
                                                ),
                                            route_association: A2aRouteAssociation::Retained,
                                            new_evidence: vec![response_evidence.clone()],
                                            retained_evidence: vec![request_evidence.clone()],
                                            maximum_evidence_bytes: 8_192,
                                            maximum_record_bytes: 65_536,
                                        },
                                        |binding, observation, new, retained| {
                                            assert_eq!(binding.bound_at.get(), 80);
                                            assert_eq!(
                                                observation.remote_state,
                                                A2aRemoteTaskStateV1::Working
                                            );
                                            assert_eq!(
                                                new,
                                                std::slice::from_ref(&response_evidence)
                                            );
                                            assert_eq!(
                                                retained,
                                                std::slice::from_ref(&request_evidence)
                                            );
                                            Ok(())
                                        },
                                        |_, _, _| Ok(signature()),
                                        |_, _, _| Ok(signature()),
                                        |_| Ok(coordinate.clone()),
                                    )
                                    .unwrap();
                                    let terminal_checkpoint_ref = terminal
                                        .checkpoint()
                                        .successor()
                                        .coordinate()
                                        .record_ref()
                                        .clone();
                                    let prepared = append
                                        .prepare_checkpoint(
                                            child.checkpoint_selection(),
                                            &terminal_checkpoint_ref,
                                            &terminal_checkpoint,
                                            AccumulatorPreparationLimits::new(
                                                8, 20_000, 20_000, 8_192, 200_000,
                                            )
                                            .unwrap(),
                                            AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                                                .unwrap(),
                                            &mut task_port,
                                        )
                                        .unwrap();
                                    let mut task_fields = fields.clone();
                                    task_fields.control_checkpoint_id =
                                        ControlCheckpointIdV1::parse(
                                            "a2a-late-task-binding-checkpoint",
                                        )
                                        .unwrap();
                                    task_fields.domain_sequence = ControlDomainSequenceV1::new(
                                        terminal_checkpoint.domain_sequence().get() + 1,
                                    )
                                    .unwrap();
                                    task_fields.previous_control_checkpoint_ref =
                                        Some(terminal_checkpoint_ref.clone());
                                    task_fields.previous_control_checkpoint_sha256 = Some(
                                        terminal_checkpoint
                                            .canonical_control_checkpoint_sha256()
                                            .clone(),
                                    );
                                    task_fields.committed_transaction_sequence =
                                        prepared.transaction_sequence();
                                    task_fields.state_root_sha256 = prepared.state_root().clone();
                                    task_fields.unresolved_custody_root_sha256 =
                                        prepared.unresolved_root().clone();
                                    task_fields.changed_leaf_keys =
                                        prepared.changed_keys().to_vec();
                                    task_fields.leaf_count = prepared.leaf_count();
                                    task_fields.issued_at = UnixTimeV1::new(81).unwrap();
                                    let physical = prepared
                                        .issue_checkpoint(
                                            &record_ref("a2a/late-task-binding-checkpoint"),
                                            task_fields.clone(),
                                            &task_fields.schema_sha256,
                                            &checkpoint_authority(&task_fields),
                                            |_, _, _| Ok(signature()),
                                        )
                                        .unwrap();
                                    let retained_task = physical
                                        .commit(&mut task_port, *anchor)
                                        .unwrap()
                                        .advance_anchor(*anchor)
                                        .unwrap();
                                    assert_eq!(
                                        task_port
                                            .compare_and_commit(retained_task.candidate())
                                            .unwrap(),
                                        OperationCustodyCommitOutcome::ExactReplay
                                    );
                                    let recovered_task = recover_known_task(
                                        &child,
                                        &inputs.binding.task_binding_id,
                                        &route,
                                        parents.signing,
                                        &task_fields,
                                        &mut task_port,
                                        *anchor,
                                    );
                                    assert_eq!(recovered_task.binding().1.bound_at().get(), 80);
                                    assert!(matches!(
                                        recovered_task
                                            .local_observation()
                                            .selected_head()
                                            .1
                                            .state(),
                                        OperationObservationStateV1::OutcomeUnknown { .. }
                                    ));
                                    assert_eq!(
                                        starts.load(Ordering::SeqCst),
                                        1,
                                        "late task identification must not replay SendMessage"
                                    );
                                    None
                                }
                                _ => panic!("unexpected first-response phase"),
                            };
                            if let Some(received) = received {
                                let response_clock: &dyn WallClockSourcePort = match response {
                                    ResponseModel::ResponseClockUnavailable => &UnavailableClock,
                                    ResponseModel::ResponseClockBackward => {
                                        &OneTimeClock(UnixTimeV1::new(68).unwrap())
                                    }
                                    ResponseModel::ResponseClockAfterRetention => {
                                        &OneTimeClock(UnixTimeV1::new(121).unwrap())
                                    }
                                    _ => &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                };
                                let completed = match received.poll_eof(&mut cx, response_clock) {
                                    A2aSendResponsePoll::Ready(Ok(value)) => Some(value),
                                    A2aSendResponsePoll::Ready(Err(
                                        A2aSendResponseFailure::OutcomeUnknown(unknown),
                                    )) if matches!(
                                        response,
                                        ResponseModel::ResponseClockUnavailable
                                            | ResponseModel::ResponseClockBackward
                                            | ResponseModel::ResponseClockAfterRetention
                                    ) =>
                                    {
                                        assert!(unknown.response_evidence().is_some());
                                        assert_eq!(starts.load(Ordering::SeqCst), 1);
                                        assert_eq!(
                                            unknown.diagnostic().wire().retry,
                                            DiagnosticRetryV1::ReconcileFirst
                                        );
                                        assert_eq!(
                                            port.compare_and_commit(&candidate).unwrap(),
                                            OperationCustodyCommitOutcome::ExactReplay,
                                            "a rejected response time must not fabricate a successor"
                                        );
                                        return;
                                    }
                                    A2aSendResponsePoll::Ready(Err(
                                        A2aSendResponseFailure::OutcomeUnknown(unknown),
                                    )) if matches!(response, ResponseModel::TaskThenExtra) => {
                                        assert!(unknown.response_evidence().is_some());
                                        assert!(unknown
                                            .diagnostic()
                                            .wire()
                                            .facts
                                            .iter()
                                            .any(|fact| fact.key == "terminal_output"
                                                && fact.value == "additional_item"));
                                        let preparation = candidate
                                            .record(OperationCustodyRecordKind::A2aSendPreparation)
                                            .unwrap()
                                            .coordinate();
                                        let frontier = admitted_reconciliation_frontier(
                                            parents.signing,
                                            preparation.record_ref().clone(),
                                            ArtifactDigest::parse(
                                                preparation.record_sha256().as_str(),
                                            )
                                            .unwrap(),
                                        );
                                        let frontier_reference =
                                            ReconciliationFrontierReferenceV1 {
                                                record_ref: record_ref(
                                                    "a2a/send-extra-reconciliation",
                                                ),
                                                record_sha256: frontier
                                                    .canonical_reconciliation_frontier_sha256()
                                                    .clone(),
                                            };
                                        let prepared = unknown
                                            .prepare_observation(
                                                A2aOutcomeUnknownObservationInput {
                                                    diagnostic_ref: record_ref(
                                                        "a2a/send-extra-diagnostic",
                                                    ),
                                                    observation_ref: record_ref(
                                                        "a2a/send-extra-observation",
                                                    ),
                                                    reconciliation:
                                                        frontier_reference.clone(),
                                                    clock: &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                                    signing: observation.signed,
                                                    maximum_diagnostic_bytes: 65_536,
                                                },
                                                |actual| {
                                                    if actual != &frontier_reference {
                                                        return Err(
                                                            InvalidResidentRecordV1::ContextualAdmission {
                                                                record: "reconciliation frontier",
                                                                invariant: "exact retained frontier",
                                                            },
                                                        );
                                                    }
                                                    Ok(())
                                                },
                                                |_, _, _, _| Ok(()),
                                                |_, _, _| Ok(signature()),
                                            )
                                            .unwrap();
                                        let (terminal, _) = retain_response_observation(
                                            prepared,
                                            &fields,
                                            "a2a-send-extra-checkpoint",
                                            true,
                                            &resolver,
                                            *port,
                                            *anchor,
                                        );
                                        assert_eq!(terminal.a2a_evidence().len(), 1);
                                        None
                                    }
                                    _ => panic!("unexpected unary completion"),
                                };
                                if let Some(completed) = completed {
                                    match (response, completed) {
                                        (
                                            ResponseModel::Task,
                                            CompletedA2aSendResponse::Task(task),
                                        ) => {
                                            assert_eq!(
                                                task.task().0,
                                                &A2aRemoteTaskIdV1::parse("task-1").unwrap()
                                            );
                                            assert_eq!(
                                                task.task().1,
                                                &A2aRemoteContextIdV1::parse("context-1").unwrap()
                                            );
                                            assert_eq!(
                                                task.task().2,
                                                &A2aRemoteTaskStateV1::Working
                                            );
                                            assert_eq!(
                                                task.evidence().0.coordinate(),
                                                &candidate.a2a_evidence()[0].coordinate().clone()
                                            );
                                            let task_dispatch = task.operation_observation();
                                            let task_proof =
                                                recover_historical_operation_observation(
                                                    &child,
                                                    task_dispatch,
                                                    &fields,
                                                    parents.signing,
                                                    port,
                                                    *anchor,
                                                );
                                            assert_eq!(task_proof.selected(), task_dispatch);
                                            let (request_evidence, response_evidence) = {
                                                let (request, response) = task.evidence();
                                                (request.clone(), response.clone())
                                            };
                                            let mut inputs = a2a_custody::fixture::Inputs::new(
                                                child.handle(),
                                                task_dispatch,
                                                (&route_ref, &route),
                                                ImmutableRecordReferenceV1 {
                                                    record_ref: request_evidence
                                                        .coordinate()
                                                        .record_ref()
                                                        .clone(),
                                                    record_sha256: ArtifactDigest::parse(
                                                        request_evidence
                                                            .coordinate()
                                                            .record_sha256()
                                                            .as_str(),
                                                    )
                                                    .unwrap(),
                                                },
                                                ImmutableRecordReferenceV1 {
                                                    record_ref: response_evidence
                                                        .coordinate()
                                                        .record_ref()
                                                        .clone(),
                                                    record_sha256: ArtifactDigest::parse(
                                                        response_evidence
                                                            .coordinate()
                                                            .record_sha256()
                                                            .as_str(),
                                                    )
                                                    .unwrap(),
                                                },
                                                70,
                                            );
                                            inputs.binding.json_rpc_request_id = request_id.clone();
                                            inputs.observation.task_binding_sha256 =
                                                A2aTaskBindingDigest::hash_canonical(
                                                    &inputs.binding,
                                                )
                                                .unwrap();
                                            let task_checkpoint_authority =
                                                checkpoint_authority(&fields);
                                            let retained_task = retain_a2a_outcome(
                                                A2aOutcomeRetentionInput::Task(
                                                    Box::new(A2aTaskOutcomeRetention::new(
                                                        task,
                                                        A2aInitialTaskCommitInput {
                                                    task: A2aInitialTaskCustodyInput {
                                                        operation_custody: &task_proof,
                                                        binding_ref: inputs.binding_ref.clone(),
                                                        binding: inputs.binding.clone(),
                                                        binding_signing:
                                                            SignedResidentAdmissionContextV1 {
                                                                expected_schema_sha256: &inputs
                                                                    .binding
                                                                    .schema_sha256,
                                                                signing_authority: parents.signing,
                                                            },
                                                        observation_ref: inputs
                                                            .observation_ref
                                                            .clone(),
                                                        observation: inputs.observation.clone(),
                                                        observation_signing:
                                                            SignedResidentAdmissionContextV1 {
                                                                expected_schema_sha256: &inputs
                                                                    .observation
                                                                    .schema_sha256,
                                                                signing_authority: parents.signing,
                                                            },
                                                        maximum_evidence_bytes: 8_192,
                                                        maximum_record_bytes: 65_536,
                                                    },
                                                    checkpoint: OperationCheckpointInput {
                                                        reference: record_ref(
                                                            "a2a/task-binding-checkpoint",
                                                        ),
                                                        id: ControlCheckpointIdV1::parse(
                                                            "a2a-task-binding-checkpoint",
                                                        )
                                                        .unwrap(),
                                                    },
                                                    clock: &OneTimeClock(UnixTimeV1::new(71).unwrap()),
                                                    checkpoint_schema: &fields.schema_sha256,
                                                    checkpoint_authority:
                                                        &task_checkpoint_authority,
                                                    accumulator_limits:
                                                        AccumulatorPreparationLimits::new(
                                                            8, 20_000, 20_000, 8_192, 200_000,
                                                        )
                                                        .unwrap(),
                                                    membership_limits:
                                                        AccumulatorMembershipLimits::new(
                                                            8, 20_000, 200_000,
                                                        )
                                                        .unwrap(),
                                                        },
                                                |binding, observation, new, retained| {
                                                    assert_eq!(
                                                        binding.remote_task_id.as_str(),
                                                        "task-1"
                                                    );
                                                    assert_eq!(
                                                        observation.remote_state,
                                                        A2aRemoteTaskStateV1::Working
                                                    );
                                                    assert_eq!(
                                                        new,
                                                        std::slice::from_ref(&response_evidence)
                                                    );
                                                    assert_eq!(
                                                        retained,
                                                        std::slice::from_ref(&request_evidence)
                                                    );
                                                    Ok(())
                                                },
                                                |_, _, _| Ok(signature()),
                                                |_, _, _| Ok(signature()),
                                                |_| Ok(coordinate.clone()),
                                                |_, _, _| Ok(signature()),
                                                    )),
                                                ),
                                                *port,
                                                *anchor,
                                            )
                                            .unwrap();
                                            let RetainedA2aOutcome::Task(retained_task) =
                                                retained_task
                                            else {
                                                panic!("actual Task changed retention branch")
                                            };
                                            assert_eq!(
                                                retained_task.binding().1.remote_task_id().as_str(),
                                                "task-1"
                                            );
                                            assert_eq!(
                                                retained_task.observation().1.remote_state(),
                                                &A2aRemoteTaskStateV1::Working
                                            );
                                            assert_eq!(
                                                retained_task
                                                    .checkpoint_preimage()
                                                    .previous_control_checkpoint_ref
                                                    .as_ref(),
                                                Some(&record_ref("a2a/send-checkpoint")),
                                                "Task binding must succeed the send checkpoint exactly once"
                                            );
                                            let task_fields =
                                                retained_task.checkpoint_preimage().clone();
                                            let task_candidate =
                                                retained_task.custody().candidate().clone();
                                            assert!(task_candidate
                                                .record(OperationCustodyRecordKind::A2aTaskBinding)
                                                .is_some());
                                            assert!(task_candidate
                                                .record(
                                                    OperationCustodyRecordKind::A2aTaskObservation,
                                                )
                                                .is_some());
                                            assert_eq!(
                                                port.compare_and_commit(&task_candidate).unwrap(),
                                                OperationCustodyCommitOutcome::ExactReplay
                                            );

                                            let recovered = recover_known_task(
                                                &child,
                                                &inputs.binding.task_binding_id,
                                                &route,
                                                parents.signing,
                                                &task_fields,
                                                port,
                                                *anchor,
                                            );
                                            let OperationObservationStateV1::Active {
                                                phase:
                                                    OperationActivePhaseV1::Dispatched {
                                                        evidence, ..
                                                    },
                                            } = recovered.dispatch().1.state()
                                            else {
                                                panic!("known Task retains its provider dispatch")
                                            };
                                            let selection = port
                                                .read_operation_record(
                                                    &coordinate,
                                                    &ControlRecordCoordinate::new(
                                                        evidence.record_ref.clone(),
                                                        ControlRecordDigest::parse(
                                                            evidence.record_sha256.as_str(),
                                                        )
                                                        .unwrap(),
                                                    ),
                                                    65_536,
                                                )
                                                .unwrap()
                                                .unwrap();
                                            if exercise_get_task {
                                                exercise_get_task_readback(
                                                    file,
                                                    &coordinate,
                                                    &child,
                                                    child_request,
                                                    generation,
                                                    &selection,
                                                    &recovered,
                                                    &route,
                                                    parents.signing,
                                                    &inputs.observation.schema_sha256,
                                                    &task_fields,
                                                    port,
                                                    *anchor,
                                                );
                                                assert_eq!(starts.load(Ordering::SeqCst), 1);
                                                return;
                                            }
                                            let cancel_request_id =
                                                A2aJsonRpcRequestIdV1::parse("cancel-1").unwrap();
                                            let cancel_prepares = Arc::new(AtomicUsize::new(0));
                                            let cancel_starts = Arc::new(AtomicUsize::new(0));
                                            let cancel_exchange = CancelExchangePort {
                                                tls_trust: expected_tls.clone(),
                                                prepares: Arc::clone(&cancel_prepares),
                                                starts: Arc::clone(&cancel_starts),
                                                response: cancel_response,
                                            };
                                            let cancel_limits =
                                                exchange_limits(ResponseModel::Task);
                                            let cancel_control_request = operation_cancel_request(
                                                child_request,
                                                child.handle().1,
                                                "a2a-cancel",
                                                72,
                                            );
                                            let checkpoint_input = || OperationCheckpointInput {
                                                reference: record_ref("a2a/cancel-checkpoint"),
                                                id: ControlCheckpointIdV1::parse(
                                                    "a2a-cancel-checkpoint",
                                                )
                                                .unwrap(),
                                            };
                                            let cancel_checkpoint_authority =
                                                checkpoint_authority(&task_fields);
                                            let prepare_clock = CountedClock::at(72);
                                            let mut prepare_cancel = || {
                                                prepare_a2a_cancel_task_custody(
                                                    A2aCancelTaskInput {
                                                        request: &cancel_control_request,
                                                        history: &recovered,
                                                        attempt: OperationAttemptHistoryInput {
                                                            operation: &child,
                                                            request: child_request,
                                                            session: child
                                                                .historical_session_lease(),
                                                            lease: child
                                                                .historical_operation_lease(),
                                                            generation,
                                                            dispatched: recovered.dispatch().1,
                                                            selection: &selection,
                                                            maximum_selection_bytes: 65_536,
                                                        },
                                                        request_id: &cancel_request_id,
                                                        request_record_ref: record_ref(
                                                            "a2a/cancel-request",
                                                        ),
                                                        response_record_ref: record_ref(
                                                            "a2a/cancel-response",
                                                        ),
                                                        additional_response_record_ref: record_ref(
                                                            "a2a/cancel-additional-response",
                                                        ),
                                                        cancellation_observation_ref: record_ref(
                                                            "a2a/cancel-requested",
                                                        ),
                                                        signing: observation.signed,
                                                        limits: cancel_limits,
                                                        maximum_evidence_bytes: 8_192,
                                                    },
                                                    &cancel_exchange,
                                                    &prepare_clock,
                                                    |context| {
                                                        assert_eq!(
                                                            context.request,
                                                            &cancel_control_request
                                                        );
                                                        assert_eq!(
                                                            context.original_request,
                                                            child_request
                                                        );
                                                        assert_eq!(
                                                            context
                                                                .binding
                                                                .1
                                                                .remote_task_id()
                                                                .as_str(),
                                                            "task-1"
                                                        );
                                                        assert_eq!(
                                                            context.remote_head.1.remote_state(),
                                                            &A2aRemoteTaskStateV1::Working
                                                        );
                                                        assert_eq!(
                                                            context.operation,
                                                            child.handle()
                                                        );
                                                        assert_eq!(
                                                            context.dispatch,
                                                            recovered.dispatch()
                                                        );
                                                        assert_eq!(
                                                            context.route,
                                                            recovered.route()
                                                        );
                                                        assert_eq!(
                                                            context
                                                                .binding
                                                                .1
                                                                .remote_context_id()
                                                                .as_str(),
                                                            "context-1"
                                                        );
                                                        assert_eq!(
                                                            context.authorized_at,
                                                            UnixTimeV1::new(72).unwrap()
                                                        );
                                                        Ok(())
                                                    },
                                                    |_, _, _, _| Ok(()),
                                                    |_, _, _| Ok(signature()),
                                                    checkpoint_input(),
                                                    UnixTimeV1::new(72).unwrap(),
                                                    &task_fields.schema_sha256,
                                                    &cancel_checkpoint_authority,
                                                    AccumulatorPreparationLimits::new(
                                                        8, 20_000, 20_000, 8_192, 200_000,
                                                    )
                                                    .unwrap(),
                                                    &resolver,
                                                    *port,
                                                    |_, _, _| Ok(signature()),
                                                )
                                                .unwrap()
                                            };
                                            let prepared = prepare_cancel();
                                            let replay = prepare_cancel();
                                            assert_eq!(
                                                prepare_clock.calls.load(Ordering::SeqCst),
                                                2
                                            );
                                            assert_eq!(cancel_prepares.load(Ordering::SeqCst), 2);
                                            assert_eq!(cancel_starts.load(Ordering::SeqCst), 0);
                                            let retained = prepared
                                                .commit(*port, *anchor)
                                                .unwrap()
                                                .advance_anchor(*anchor)
                                                .unwrap();
                                            let replay = replay.commit(*port, *anchor).unwrap();
                                            assert_eq!(
                                                replay.database_evidence(),
                                                OperationCustodyDatabaseEvidence::ExactReplay
                                            );
                                            assert!(matches!(
                                                replay.advance_anchor(*anchor),
                                                Err(A2aCancelTaskError::NotNewDatabaseCommit)
                                            ));
                                            assert_eq!(cancel_starts.load(Ordering::SeqCst), 0);

                                            let cancellation = (
                                                retained.cancellation_observation().0.clone(),
                                                retained.cancellation_observation().1.clone(),
                                            );
                                            let cancel_checkpoint = (
                                                retained.checkpoint().0.clone(),
                                                retained.checkpoint().1.clone(),
                                            );
                                            let interleaved_checkpoint =
                                                retain_unrelated_parent_observation(
                                                    &root,
                                                    &root_candidate,
                                                    parents.signing,
                                                    (&cancel_checkpoint.0, &cancel_checkpoint.1),
                                                    &resolver,
                                                    *port,
                                                    *anchor,
                                                );
                                            let current = recover_known_task(
                                                &child,
                                                &inputs.binding.task_binding_id,
                                                &route,
                                                parents.signing,
                                                &task_fields,
                                                port,
                                                *anchor,
                                            );
                                            assert_eq!(
                                                current.local_observation().selected_head(),
                                                (&cancellation.0, &cancellation.1)
                                            );
                                            assert_eq!(
                                                current.local_observation().current_checkpoint().1,
                                                &interleaved_checkpoint,
                                                "disposition must append from the freshly authenticated domain head",
                                            );
                                            let retained = match start_a2a_cancel_task(
                                                retained,
                                                &current,
                                                &cancel_exchange,
                                                &UnavailableClock,
                                                |_| {
                                                    panic!(
                                                        "clock failure must precede authorization"
                                                    )
                                                },
                                            ) {
                                                Err(A2aCancelTaskStartFailure::Preflight {
                                                    retained,
                                                    reason: A2aCancelTaskError::Clock(_),
                                                }) => *retained,
                                                _ => panic!(
                                                    "unavailable current clock must refuse release"
                                                ),
                                            };
                                            assert_eq!(cancel_starts.load(Ordering::SeqCst), 0);
                                            let denied_clock = CountedClock::at(73);
                                            let retained = match start_a2a_cancel_task(
                                                retained,
                                                &current,
                                                &cancel_exchange,
                                                &denied_clock,
                                                |context| {
                                                    assert_eq!(
                                                        context.local_head,
                                                        (&cancellation.0, &cancellation.1)
                                                    );
                                                    Err(InvalidResidentRecordV1::ContextualAdmission {
                                                        record: "A2A CancelTask test authority",
                                                        invariant: "current cancellation permission",
                                                    })
                                                },
                                            ) {
                                                Err(A2aCancelTaskStartFailure::Preflight {
                                                    retained,
                                                    reason: A2aCancelTaskError::Authorization(_),
                                                }) => *retained,
                                                _ => panic!("current authorization denial must refuse release"),
                                            };
                                            assert_eq!(
                                                denied_clock.calls.load(Ordering::SeqCst),
                                                1
                                            );
                                            assert_eq!(cancel_starts.load(Ordering::SeqCst), 0);
                                            let start_clock = CountedClock::at(73);
                                            let started = match start_a2a_cancel_task(
                                                retained,
                                                &current,
                                                &cancel_exchange,
                                                &start_clock,
                                                |context| {
                                                    assert_eq!(
                                                        context.request,
                                                        &cancel_control_request
                                                    );
                                                    assert_eq!(
                                                        context.original_request,
                                                        child_request
                                                    );
                                                    assert_eq!(
                                                        context.local_head,
                                                        (&cancellation.0, &cancellation.1)
                                                    );
                                                    assert_eq!(context.operation, child.handle());
                                                    assert_eq!(
                                                        context.dispatch,
                                                        current.dispatch()
                                                    );
                                                    assert_eq!(context.route, current.route());
                                                    assert_eq!(context.binding, current.binding());
                                                    assert_eq!(
                                                        context.remote_head,
                                                        current.observation()
                                                    );
                                                    assert_eq!(
                                                        context.authorized_at,
                                                        UnixTimeV1::new(73).unwrap()
                                                    );
                                                    Ok(())
                                                },
                                            ) {
                                                Ok(started) => {
                                                    assert!(!matches!(
                                                        cancel_response,
                                                        CancelResponseModel::StartRejected
                                                    ));
                                                    started
                                                }
                                                Err(A2aCancelTaskStartFailure::NotStarted(
                                                    stopped,
                                                )) => {
                                                    assert!(matches!(
                                                        cancel_response,
                                                        CancelResponseModel::StartRejected
                                                    ));
                                                    assert_eq!(
                                                        stopped.reason(),
                                                        A2aExchangeError::ServiceUnavailable
                                                    );
                                                    assert_eq!(
                                                        stopped
                                                            .request_evidence()
                                                            .coordinate()
                                                            .record_ref()
                                                            .as_str(),
                                                        "a2a/cancel-request"
                                                    );
                                                    assert_eq!(
                                                        stopped.local_custody().database_evidence(),
                                                        OperationCustodyDatabaseEvidence::New
                                                    );
                                                    assert_eq!(
                                                        cancel_starts.load(Ordering::SeqCst),
                                                        1
                                                    );
                                                    let request_coordinate =
                                                        stopped.request_evidence().coordinate();
                                                    let frontier = admitted_reconciliation_frontier(
                                                        parents.signing,
                                                        request_coordinate.record_ref().clone(),
                                                        ArtifactDigest::parse(
                                                            request_coordinate
                                                                .record_sha256()
                                                                .as_str(),
                                                        )
                                                        .unwrap(),
                                                    );
                                                    let frontier_reference =
                                                        ReconciliationFrontierReferenceV1 {
                                                            record_ref: record_ref(
                                                                "a2a/cancel-not-started-reconciliation",
                                                            ),
                                                            record_sha256: frontier
                                                                .canonical_reconciliation_frontier_sha256()
                                                                .clone(),
                                                        };
                                                    let prepared = match stopped
                                                        .prepare_disposition(
                                                            A2aCancelTaskDispositionInput {
                                                                history: &current,
                                                                operation: &child,
                                                                diagnostic_ref: record_ref(
                                                                    "a2a/cancel-not-started-diagnostic",
                                                                ),
                                                                observation_ref: record_ref(
                                                                    "a2a/cancel-not-started-observation",
                                                                ),
                                                                reconciliation:
                                                                    frontier_reference.clone(),
                                                                signing: observation.signed,
                                                                observed_at: UnixTimeV1::new(74)
                                                                    .unwrap(),
                                                                maximum_diagnostic_bytes: 65_536,
                                                            },
                                                            |actual| {
                                                                assert_eq!(
                                                                    actual,
                                                                    &frontier_reference
                                                                );
                                                                Ok(())
                                                            },
                                                            |_, _, _, _| Ok(()),
                                                            |_, _, _| Ok(signature()),
                                                        ) {
                                                        Ok(prepared) => prepared,
                                                        Err(failure) => panic!(
                                                            "carrier rejection disposition failed: {}",
                                                            failure.reason()
                                                        ),
                                                    };
                                                    assert_eq!(
                                                        prepared.diagnostic().wire().effect,
                                                        DiagnosticEffectCertaintyV1::NotStarted
                                                    );
                                                    assert_eq!(
                                                        prepared.diagnostic().wire().retry,
                                                        DiagnosticRetryV1::ReconcileFirst
                                                    );
                                                    assert!(matches!(
                                                        prepared.observation().1.state(),
                                                        OperationObservationStateV1::OutcomeUnknown {
                                                            ..
                                                        }
                                                    ));
                                                    assert!(prepared
                                                        .response_evidence()
                                                        .is_empty());
                                                    let (terminal, _) = retain_cancel_disposition(
                                                        &prepared,
                                                        child.checkpoint_selection(),
                                                        &task_fields,
                                                        "a2a-cancel-not-started-checkpoint",
                                                        0,
                                                        &resolver,
                                                        *port,
                                                        *anchor,
                                                    );
                                                    assert!(terminal.a2a_evidence().is_empty());
                                                    assert!(terminal
                                                        .record(
                                                            OperationCustodyRecordKind::Diagnostic
                                                        )
                                                        .is_some());
                                                    let recovered_unknown = recover_known_task(
                                                        &child,
                                                        &inputs.binding.task_binding_id,
                                                        &route,
                                                        parents.signing,
                                                        &task_fields,
                                                        port,
                                                        *anchor,
                                                    );
                                                    assert!(matches!(
                                                        recovered_unknown
                                                            .local_observation()
                                                            .selected_head()
                                                            .1
                                                            .state(),
                                                        OperationObservationStateV1::OutcomeUnknown {
                                                            ..
                                                        }
                                                    ));
                                                    let retry_control_request =
                                                        operation_cancel_request(
                                                            child_request,
                                                            child.handle().1,
                                                            "a2a-cancel-retry",
                                                            76,
                                                        );
                                                    assert!(matches!(
                                                        prepare_a2a_cancel_task_custody(
                                                            A2aCancelTaskInput {
                                                                request: &retry_control_request,
                                                                history: &recovered_unknown,
                                                                attempt:
                                                                    OperationAttemptHistoryInput {
                                                                        operation: &child,
                                                                        request: child_request,
                                                                        session: child
                                                                            .historical_session_lease(),
                                                                        lease: child
                                                                            .historical_operation_lease(),
                                                                        generation,
                                                                        dispatched: recovered_unknown
                                                                            .dispatch()
                                                                            .1,
                                                                        selection: &selection,
                                                                        maximum_selection_bytes:
                                                                            65_536,
                                                                    },
                                                                request_id: &cancel_request_id,
                                                                request_record_ref: record_ref(
                                                                    "a2a/cancel-retry-request",
                                                                ),
                                                                response_record_ref: record_ref(
                                                                    "a2a/cancel-retry-response",
                                                                ),
                                                                additional_response_record_ref:
                                                                    record_ref(
                                                                        "a2a/cancel-retry-additional-response",
                                                                    ),
                                                                cancellation_observation_ref:
                                                                    record_ref(
                                                                        "a2a/cancel-retry-observation",
                                                                    ),
                                                                signing: observation.signed,
                                                                limits: cancel_limits,
                                                                maximum_evidence_bytes: 8_192,
                                                            },
                                                            &cancel_exchange,
                                                            &OneTimeClock(
                                                                UnixTimeV1::new(76).unwrap(),
                                                            ),
                                                            |_| Ok(()),
                                                            |_, _, _, _| Ok(()),
                                                            |_, _, _| Ok(signature()),
                                                            OperationCheckpointInput {
                                                                reference: record_ref(
                                                                    "a2a/cancel-retry-checkpoint",
                                                                ),
                                                                id: ControlCheckpointIdV1::parse(
                                                                    "a2a-cancel-retry-checkpoint",
                                                                )
                                                                .unwrap(),
                                                            },
                                                            UnixTimeV1::new(76).unwrap(),
                                                            &task_fields.schema_sha256,
                                                            &cancel_checkpoint_authority,
                                                            AccumulatorPreparationLimits::new(
                                                                8, 20_000, 20_000, 8_192, 200_000,
                                                            )
                                                            .unwrap(),
                                                            &resolver,
                                                            *port,
                                                            |_, _, _| Ok(signature()),
                                                        ),
                                                        Err(A2aCancelTaskError::Association)
                                                    ));
                                                    assert_eq!(
                                                        cancel_starts.load(Ordering::SeqCst),
                                                        1,
                                                        "restart-visible uncertainty cannot replay CancelTask"
                                                    );
                                                    return;
                                                }
                                                Err(_) => panic!("unexpected cancellation start"),
                                            };
                                            assert_eq!(start_clock.calls.load(Ordering::SeqCst), 1);
                                            assert_eq!(cancel_starts.load(Ordering::SeqCst), 1);
                                            let mut cx = poll_context();
                                            let mut in_flight_owner = Some(started);
                                            let started = match in_flight_owner
                                                .take()
                                                .unwrap()
                                                .poll_response(
                                                    &mut cx,
                                                    &pale_adapter_a2a::A2aCancelTaskDecoder,
                                                ) {
                                                A2aCancelTaskPoll::Pending(value) => value,
                                                _ => panic!(
                                                    "first CancelTask response poll must pend"
                                                ),
                                            };
                                            assert!(in_flight_owner.is_none());
                                            if matches!(
                                                cancel_response,
                                                CancelResponseModel::ProcessLostAfterStart
                                            ) {
                                                drop(started);
                                                assert_eq!(
                                                    cancel_starts.load(Ordering::SeqCst),
                                                    1,
                                                    "process loss cannot replay the admitted CancelTask",
                                                );
                                                exercise_get_task_readback(
                                                    file,
                                                    &coordinate,
                                                    &child,
                                                    child_request,
                                                    generation,
                                                    &selection,
                                                    &current,
                                                    &route,
                                                    parents.signing,
                                                    &inputs.observation.schema_sha256,
                                                    &task_fields,
                                                    port,
                                                    *anchor,
                                                );
                                                assert_eq!(
                                                    cancel_starts.load(Ordering::SeqCst),
                                                    1,
                                                    "restart recovery uses GetTask, not CancelTask replay",
                                                );
                                                return;
                                            }
                                            let received = match started.poll_response(
                                                &mut cx,
                                                &pale_adapter_a2a::A2aCancelTaskDecoder,
                                            ) {
                                                A2aCancelTaskPoll::Ready(Ok(value)) => value,
                                                A2aCancelTaskPoll::Ready(Err(unknown)) => {
                                                    assert!(matches!(
                                                        cancel_response,
                                                        CancelResponseModel::Eof
                                                    ));
                                                    assert_eq!(
                                                        unknown.reason(),
                                                        A2aCancelTaskUncertainty::MissingResponse
                                                    );
                                                    assert!(unknown.response_evidence().is_empty());
                                                    assert_eq!(
                                                        unknown
                                                            .request_evidence()
                                                            .coordinate()
                                                            .record_ref()
                                                            .as_str(),
                                                        "a2a/cancel-request"
                                                    );
                                                    assert_eq!(
                                                        unknown.local_custody().database_evidence(),
                                                        OperationCustodyDatabaseEvidence::New
                                                    );
                                                    let request_coordinate =
                                                        unknown.request_evidence().coordinate();
                                                    let frontier = admitted_reconciliation_frontier(
                                                        parents.signing,
                                                        request_coordinate.record_ref().clone(),
                                                        ArtifactDigest::parse(
                                                            request_coordinate
                                                                .record_sha256()
                                                                .as_str(),
                                                        )
                                                        .unwrap(),
                                                    );
                                                    let frontier_reference =
                                                        ReconciliationFrontierReferenceV1 {
                                                            record_ref: record_ref(
                                                                "a2a/cancel-missing-reconciliation",
                                                            ),
                                                            record_sha256: frontier
                                                                .canonical_reconciliation_frontier_sha256()
                                                                .clone(),
                                                        };
                                                    let prepared = match unknown
                                                        .prepare_disposition(
                                                            A2aCancelTaskDispositionInput {
                                                                history: &current,
                                                                operation: &child,
                                                                diagnostic_ref: record_ref(
                                                                    "a2a/cancel-missing-diagnostic",
                                                                ),
                                                                observation_ref: record_ref(
                                                                    "a2a/cancel-missing-observation",
                                                                ),
                                                                reconciliation:
                                                                    frontier_reference.clone(),
                                                                signing: observation.signed,
                                                                observed_at: UnixTimeV1::new(74)
                                                                    .unwrap(),
                                                                maximum_diagnostic_bytes: 65_536,
                                                            },
                                                            |actual| {
                                                                assert_eq!(
                                                                    actual,
                                                                    &frontier_reference
                                                                );
                                                                Ok(())
                                                            },
                                                            |_, _, _, _| Ok(()),
                                                            |_, _, _| Ok(signature()),
                                                        ) {
                                                        Ok(prepared) => prepared,
                                                        Err(failure) => panic!(
                                                            "missing-response disposition failed: {}",
                                                            failure.reason()
                                                        ),
                                                    };
                                                    assert_eq!(
                                                        prepared.diagnostic().wire().effect,
                                                        DiagnosticEffectCertaintyV1::OutcomeUnknown
                                                    );
                                                    assert_eq!(
                                                        prepared.diagnostic().wire().disposition,
                                                        DiagnosticDispositionV1::EffectOutcomeUnknown
                                                    );
                                                    assert!(prepared
                                                        .response_evidence()
                                                        .is_empty());
                                                    let (terminal, _) = retain_cancel_disposition(
                                                        &prepared,
                                                        child.checkpoint_selection(),
                                                        &task_fields,
                                                        "a2a-cancel-missing-checkpoint",
                                                        0,
                                                        &resolver,
                                                        *port,
                                                        *anchor,
                                                    );
                                                    assert!(terminal.a2a_evidence().is_empty());
                                                    let recovered_unknown = recover_known_task(
                                                        &child,
                                                        &inputs.binding.task_binding_id,
                                                        &route,
                                                        parents.signing,
                                                        &task_fields,
                                                        port,
                                                        *anchor,
                                                    );
                                                    assert!(matches!(
                                                        recovered_unknown
                                                            .local_observation()
                                                            .selected_head()
                                                            .1
                                                            .state(),
                                                        OperationObservationStateV1::OutcomeUnknown {
                                                            ..
                                                        }
                                                    ));
                                                    return;
                                                }
                                                _ => panic!("exact CancelTask response"),
                                            };
                                            assert!(matches!(
                                                cancel_response,
                                                CancelResponseModel::Task
                                                    | CancelResponseModel::TaskThenExtra
                                            ));
                                            let observation_clock = CountedClock::at(74);
                                            let mut response_owner = Some(received);
                                            let received = match response_owner
                                                .take()
                                                .unwrap()
                                                .poll_eof(&mut cx, &observation_clock)
                                            {
                                                A2aCancelTaskPoll::Pending(value) => value,
                                                _ => panic!("first CancelTask EOF poll must pend"),
                                            };
                                            assert!(response_owner.is_none());
                                            assert_eq!(
                                                observation_clock.calls.load(Ordering::SeqCst),
                                                0,
                                                "response time is sampled only after exact EOF",
                                            );
                                            let completed = match received
                                                .poll_eof(&mut cx, &observation_clock)
                                            {
                                                A2aCancelTaskPoll::Ready(Ok(value)) => {
                                                    assert!(matches!(
                                                        cancel_response,
                                                        CancelResponseModel::Task
                                                    ));
                                                    value
                                                }
                                                A2aCancelTaskPoll::Ready(Err(unknown)) => {
                                                    assert!(matches!(
                                                        cancel_response,
                                                        CancelResponseModel::TaskThenExtra
                                                    ));
                                                    assert_eq!(
                                                        unknown.reason(),
                                                        A2aCancelTaskUncertainty::UnexpectedResponse
                                                    );
                                                    assert_eq!(
                                                        unknown.response_evidence().len(),
                                                        2
                                                    );
                                                    let response_coordinates: Vec<_> = unknown
                                                        .response_evidence()
                                                        .iter()
                                                        .map(|evidence| {
                                                            evidence.coordinate().clone()
                                                        })
                                                        .collect();
                                                    let request_coordinate =
                                                        unknown.request_evidence().coordinate();
                                                    let frontier = admitted_reconciliation_frontier(
                                                        parents.signing,
                                                        request_coordinate.record_ref().clone(),
                                                        ArtifactDigest::parse(
                                                            request_coordinate
                                                                .record_sha256()
                                                                .as_str(),
                                                        )
                                                        .unwrap(),
                                                    );
                                                    let frontier_reference =
                                                        ReconciliationFrontierReferenceV1 {
                                                            record_ref: record_ref(
                                                                "a2a/cancel-extra-reconciliation",
                                                            ),
                                                            record_sha256: frontier
                                                                .canonical_reconciliation_frontier_sha256()
                                                                .clone(),
                                                        };
                                                    let prepared = match unknown
                                                        .prepare_disposition(
                                                            A2aCancelTaskDispositionInput {
                                                                history: &current,
                                                                operation: &child,
                                                                diagnostic_ref: record_ref(
                                                                    "a2a/cancel-extra-diagnostic",
                                                                ),
                                                                observation_ref: record_ref(
                                                                    "a2a/cancel-extra-observation",
                                                                ),
                                                                reconciliation: frontier_reference
                                                                    .clone(),
                                                                signing: observation.signed,
                                                                observed_at: UnixTimeV1::new(74)
                                                                    .unwrap(),
                                                                maximum_diagnostic_bytes: 65_536,
                                                            },
                                                            |actual| {
                                                                assert_eq!(
                                                                    actual,
                                                                    &frontier_reference
                                                                );
                                                                Ok(())
                                                            },
                                                            |_, _, _, _| Ok(()),
                                                            |_, _, _| Ok(signature()),
                                                        ) {
                                                        Ok(prepared) => prepared,
                                                        Err(failure) => panic!(
                                                            "extra-response disposition failed: {}",
                                                            failure.reason()
                                                        ),
                                                    };
                                                    assert_eq!(
                                                        prepared.response_evidence().len(),
                                                        2
                                                    );
                                                    let (terminal, _) = retain_cancel_disposition(
                                                        &prepared,
                                                        child.checkpoint_selection(),
                                                        &task_fields,
                                                        "a2a-cancel-extra-checkpoint",
                                                        2,
                                                        &resolver,
                                                        *port,
                                                        *anchor,
                                                    );
                                                    assert_eq!(terminal.a2a_evidence().len(), 2);
                                                    assert_eq!(
                                                        terminal
                                                            .a2a_evidence()
                                                            .iter()
                                                            .map(|evidence| evidence.coordinate())
                                                            .collect::<Vec<_>>(),
                                                        response_coordinates
                                                            .iter()
                                                            .collect::<Vec<_>>()
                                                    );
                                                    for response_coordinate in &response_coordinates
                                                    {
                                                        assert_eq!(
                                                            port.read_a2a_evidence(
                                                                &coordinate,
                                                                response_coordinate,
                                                                8_192,
                                                            )
                                                            .unwrap()
                                                            .unwrap()
                                                            .coordinate(),
                                                            response_coordinate
                                                        );
                                                    }
                                                    return;
                                                }
                                                _ => panic!("exact unary CancelTask EOF"),
                                            };
                                            assert_eq!(
                                                observation_clock.calls.load(Ordering::SeqCst),
                                                usize::from(matches!(
                                                    cancel_response,
                                                    CancelResponseModel::Task
                                                )),
                                            );
                                            assert_eq!(
                                                completed.remote_state(),
                                                &A2aRemoteTaskStateV1::Canceled
                                            );
                                            assert_eq!(
                                                completed.local_cancellation(),
                                                (&cancellation.0, &cancellation.1)
                                            );
                                            let append = completed
                                                .prepare_observation(
                                                    A2aCancelTaskObservationInput {
                                                        history: &current,
                                                        operation: &child,
                                                        observation_ref: record_ref(
                                                            "a2a/cancel-readback",
                                                        ),
                                                        signing: SignedResidentAdmissionContextV1 {
                                                            expected_schema_sha256: &inputs
                                                                .observation
                                                                .schema_sha256,
                                                            signing_authority: parents.signing,
                                                        },
                                                        maximum_evidence_bytes: 8_192,
                                                        maximum_record_bytes: 65_536,
                                                    },
                                                    &pale_adapter_a2a::A2aCancelTaskDecoder,
                                                    |_, _, _, _| Ok(()),
                                                    |_, _, _| Ok(signature()),
                                                    |_| Ok(coordinate.clone()),
                                                )
                                                .unwrap();
                                            let cancel_remote = append.observation().1.clone();
                                            let readback_checkpoint =
                                                current.local_observation().current_checkpoint();
                                            let prepared = append
                                                .prepare_checkpoint(
                                                    child.checkpoint_selection(),
                                                    readback_checkpoint.0,
                                                    readback_checkpoint.1,
                                                    AccumulatorPreparationLimits::new(
                                                        8, 20_000, 20_000, 8_192, 200_000,
                                                    )
                                                    .unwrap(),
                                                    AccumulatorMembershipLimits::new(
                                                        8, 20_000, 200_000,
                                                    )
                                                    .unwrap(),
                                                    *port,
                                                )
                                                .unwrap();
                                            let mut cancel_fields = task_fields.clone();
                                            cancel_fields.control_checkpoint_id =
                                                ControlCheckpointIdV1::parse(
                                                    "a2a-cancel-readback-checkpoint",
                                                )
                                                .unwrap();
                                            cancel_fields.domain_sequence =
                                                ControlDomainSequenceV1::new(
                                                    readback_checkpoint.1.domain_sequence().get()
                                                        + 1,
                                                )
                                                .unwrap();
                                            cancel_fields.previous_control_checkpoint_ref =
                                                Some(readback_checkpoint.0.clone());
                                            cancel_fields.previous_control_checkpoint_sha256 = Some(
                                                readback_checkpoint
                                                    .1
                                                    .canonical_control_checkpoint_sha256()
                                                    .clone(),
                                            );
                                            cancel_fields.committed_transaction_sequence =
                                                prepared.transaction_sequence();
                                            cancel_fields.state_root_sha256 =
                                                prepared.state_root().clone();
                                            cancel_fields.unresolved_custody_root_sha256 =
                                                prepared.unresolved_root().clone();
                                            cancel_fields.changed_leaf_keys =
                                                prepared.changed_keys().to_vec();
                                            cancel_fields.leaf_count = prepared.leaf_count();
                                            cancel_fields.issued_at = UnixTimeV1::new(75).unwrap();
                                            let physical = prepared
                                                .issue_checkpoint(
                                                    &record_ref("a2a/cancel-readback-checkpoint"),
                                                    cancel_fields.clone(),
                                                    &cancel_fields.schema_sha256,
                                                    &checkpoint_authority(&cancel_fields),
                                                    |_, _, _| Ok(signature()),
                                                )
                                                .unwrap();
                                            let cancel_done = physical
                                                .commit(*port, *anchor)
                                                .unwrap()
                                                .advance_anchor(*anchor)
                                                .unwrap();
                                            assert!(matches!(
                                                cancel_remote.remote_state(),
                                                A2aRemoteTaskStateV1::Canceled
                                            ));
                                            let local = read_operation_observation_for_handle(
                                                *port,
                                                &coordinate,
                                                child.handle().0,
                                                child.handle().1,
                                                65_536,
                                            )
                                            .unwrap()
                                            .unwrap();
                                            assert_eq!(
                                                local.observation().coordinate().record_ref(),
                                                &cancellation.0
                                            );
                                            assert!(matches!(
                                                cancellation.1.state(),
                                                OperationObservationStateV1::CancellationRequested {
                                                    ..
                                                }
                                            ));
                                            assert!(cancel_done.candidate().record(
                                                OperationCustodyRecordKind::OperationObservation
                                            ).is_none());
                                            assert_eq!(
                                                cancel_done
                                                    .candidate()
                                                    .record(
                                                        OperationCustodyRecordKind::A2aTaskObservation
                                                    )
                                                    .unwrap()
                                                    .canonical_record_jcs()
                                                    .as_slice(),
                                                pale_core::canon::to_canonical_string(
                                                    &cancel_remote
                                                )
                                                .unwrap()
                                                .as_bytes(),
                                            );
                                            assert_eq!(
                                                port.compare_and_commit(cancel_done.candidate())
                                                    .unwrap(),
                                                OperationCustodyCommitOutcome::ExactReplay
                                            );
                                            assert_eq!(
                                                cancel_starts.load(Ordering::SeqCst),
                                                1,
                                                "exact custody replay cannot start another carrier",
                                            );
                                            let mut restarted = connection(file);
                                            let mut restarted_port = SqliteOperationCustody {
                                                connection: &mut restarted,
                                                coordinate: coordinate.clone(),
                                            };
                                            let restarted_history = recover_known_task(
                                                &child,
                                                &inputs.binding.task_binding_id,
                                                &route,
                                                parents.signing,
                                                &task_fields,
                                                &mut restarted_port,
                                                *anchor,
                                            );
                                            assert_eq!(
                                                restarted_history.binding(),
                                                current.binding()
                                            );
                                            assert_eq!(restarted_history.route(), current.route());
                                            assert_eq!(
                                                restarted_history.observation().1,
                                                &cancel_remote
                                            );
                                            assert_eq!(
                                                restarted_history
                                                    .local_observation()
                                                    .selected_head(),
                                                (&cancellation.0, &cancellation.1),
                                                "remote cancellation read-back cannot promote Pale operation state",
                                            );
                                            assert_eq!(cancel_starts.load(Ordering::SeqCst), 1);
                                        }
                                        (
                                            ResponseModel::DirectMessage
                                            | ResponseModel::DirectMessageAnchorUnavailable
                                            | ResponseModel::DirectMessageAnchorOutcomeUnknown,
                                            CompletedA2aSendResponse::NoTask(no_task),
                                        ) => {
                                            assert_eq!(
                                                no_task.evidence().0.wire().code,
                                                "a2a_initial_response_has_no_task"
                                            );
                                            assert_eq!(
                                                no_task.evidence().0.wire().effect,
                                                DiagnosticEffectCertaintyV1::Confirmed
                                            );
                                            assert_eq!(
                                                no_task.facts(),
                                                &A2aInitialResponseFacts::DirectMessage
                                            );
                                            assert_eq!(
                                                no_task.evidence().0.wire().stage,
                                                DiagnosticStageV1::HostObservation
                                            );
                                            let response_coordinate =
                                                no_task.evidence().2.coordinate().clone();
                                            let rejected_checkpoint_authority =
                                                checkpoint_authority(&fields);
                                            let duplicate_ref =
                                                record_ref("a2a/send-no-task-duplicate");
                                            let no_task = match retain_a2a_outcome(
                                                A2aOutcomeRetentionInput::NoTask(
                                                    A2aNoTaskOutcomeRetention::new(
                                                        no_task,
                                                        A2aFailedSendObservationInput {
                                                            diagnostic_ref: duplicate_ref.clone(),
                                                            observation_ref: duplicate_ref,
                                                            clock: &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                                            signing: observation.signed,
                                                            maximum_diagnostic_bytes: 65_536,
                                                        },
                                                        A2aSendObservationCommitInput {
                                                            checkpoint:
                                                                OperationCheckpointInput {
                                                                    reference: record_ref(
                                                                        "a2a-send-no-task-rejected-checkpoint/record",
                                                                    ),
                                                                    id: ControlCheckpointIdV1::parse(
                                                                        "a2a-send-no-task-rejected-checkpoint",
                                                                    )
                                                                    .unwrap(),
                                                                },
                                                            clock: &OneTimeClock(UnixTimeV1::new(70).unwrap()),
                                                            checkpoint_schema:
                                                                &fields.schema_sha256,
                                                            checkpoint_authority:
                                                                &rejected_checkpoint_authority,
                                                            accumulator_limits:
                                                                AccumulatorPreparationLimits::new(
                                                                    8, 20_000, 20_000, 8_192,
                                                                    200_000,
                                                                )
                                                                .unwrap(),
                                                        },
                                                        &resolver,
                                                        |_, _, _, _| Ok(()),
                                                        |_, _, _| Ok(signature()),
                                                        |_, _, _| Ok(signature()),
                                                    ),
                                                ),
                                                *port,
                                                *anchor,
                                            ) {
                                                Err(
                                                    A2aOutcomeRetentionFailure::NoTaskObservation(
                                                        rejected,
                                                    ),
                                                ) => {
                                                    assert!(matches!(
                                                        rejected.reason(),
                                                        OperationAttemptError::Association
                                                    ));
                                                    rejected.into_response()
                                                }
                                                Err(other) => panic!(
                                                    "unexpected duplicate-reference failure: {other:?}"
                                                ),
                                                Ok(_) => panic!(
                                                    "duplicate diagnostic and observation references were admitted"
                                                ),
                                            };
                                            let outcome_checkpoint_authority =
                                                checkpoint_authority(&fields);
                                            let anchor_failure = match response {
                                                ResponseModel::DirectMessageAnchorUnavailable => {
                                                    Some(AntiRollbackAnchorErrorKind::Unavailable)
                                                }
                                                ResponseModel::DirectMessageAnchorOutcomeUnknown => {
                                                    Some(
                                                        AntiRollbackAnchorErrorKind::OutcomeUnknown,
                                                    )
                                                }
                                                _ => None,
                                            };
                                            let mut terminal_anchor = TerminalAnchor {
                                                inner: *anchor,
                                                failure: anchor_failure,
                                            };
                                            let observation_clock =
                                                OneTimeClock(UnixTimeV1::new(70).unwrap());
                                            let checkpoint_clock =
                                                OneTimeClock(UnixTimeV1::new(70).unwrap());
                                            let retained = retain_a2a_outcome(
                                                A2aOutcomeRetentionInput::NoTask(
                                                    A2aNoTaskOutcomeRetention::new(
                                                        Box::new(no_task),
                                                    A2aFailedSendObservationInput {
                                                        diagnostic_ref: record_ref(
                                                            "a2a/send-no-task-diagnostic",
                                                        ),
                                                        observation_ref: record_ref(
                                                            "a2a/send-no-task-observation",
                                                        ),
                                                        clock: &observation_clock,
                                                        signing: observation.signed,
                                                        maximum_diagnostic_bytes: 65_536,
                                                    },
                                                        A2aSendObservationCommitInput {
                                                            checkpoint:
                                                                OperationCheckpointInput {
                                                                    reference: record_ref(
                                                                        "a2a-send-no-task-checkpoint/record",
                                                                    ),
                                                                    id: ControlCheckpointIdV1::parse(
                                                                        "a2a-send-no-task-checkpoint",
                                                                    )
                                                                    .unwrap(),
                                                                },
                                                            clock: &checkpoint_clock,
                                                            checkpoint_schema:
                                                                &fields.schema_sha256,
                                                            checkpoint_authority:
                                                                &outcome_checkpoint_authority,
                                                            accumulator_limits:
                                                                AccumulatorPreparationLimits::new(
                                                                    8, 20_000, 20_000, 8_192,
                                                                    200_000,
                                                                )
                                                                .unwrap(),
                                                        },
                                                        &resolver,
                                                    |_, _, _, _| Ok(()),
                                                    |_, _, _| Ok(signature()),
                                                        |_, _, _| Ok(signature()),
                                                    ),
                                                ),
                                                *port,
                                                &mut terminal_anchor,
                                            );
                                            if let Some(expected) = anchor_failure {
                                                let failure = match retained {
                                                    Err(
                                                        A2aOutcomeRetentionFailure::NoTaskCommit(
                                                            A2aSendObservationCommitFailure::Anchor {
                                                                failure,
                                                                ..
                                                            },
                                                        ),
                                                    ) => failure,
                                                    Err(other) => panic!(
                                                        "unexpected task-less anchor failure: {other:?}"
                                                    ),
                                                    Ok(_) => panic!(
                                                        "controlled task-less anchor failure completed"
                                                    ),
                                                };
                                                match (expected, failure.reason()) {
                                                    (
                                                        AntiRollbackAnchorErrorKind::Unavailable,
                                                        OperationCustodyAnchorFailureReason::Port(
                                                            error,
                                                        ),
                                                    ) => assert_eq!(
                                                        error.kind(),
                                                        AntiRollbackAnchorErrorKind::Unavailable
                                                    ),
                                                    (
                                                        AntiRollbackAnchorErrorKind::OutcomeUnknown,
                                                        OperationCustodyAnchorFailureReason::AmbiguousNotExact {
                                                            uncertain,
                                                            observation,
                                                        },
                                                    ) => {
                                                        assert_eq!(
                                                            uncertain.kind(),
                                                            AntiRollbackAnchorErrorKind::OutcomeUnknown
                                                        );
                                                        assert!(matches!(
                                                            observation.as_ref(),
                                                            AntiRollbackAnchorObservation::ExactPredecessor
                                                        ));
                                                    }
                                                    (_, reason) => panic!(
                                                        "unexpected task-less anchor classification: {reason:?}"
                                                    ),
                                                }
                                                assert_eq!(
                                                    starts.load(Ordering::SeqCst),
                                                    1,
                                                    "anchor failure cannot replay the accepted send"
                                                );
                                                return;
                                            }
                                            let retained = retained.unwrap();
                                            let RetainedA2aOutcome::NoTask(retained) = retained
                                            else {
                                                panic!(
                                                    "task-less response changed retention branch"
                                                )
                                            };
                                            assert!(matches!(
                                                retained.observation().1.state(),
                                                OperationObservationStateV1::Failed {
                                                    result: None,
                                                    ..
                                                }
                                            ));
                                            assert_eq!(
                                                retained
                                                    .checkpoint_preimage()
                                                    .previous_control_checkpoint_ref
                                                    .as_ref(),
                                                Some(&record_ref("a2a/send-checkpoint")),
                                                "task-less retention must succeed the send checkpoint exactly once"
                                            );
                                            let terminal = retained.custody().candidate();
                                            assert_eq!(terminal.a2a_evidence().len(), 1);
                                            assert_eq!(
                                                terminal.a2a_evidence()[0].coordinate(),
                                                &response_coordinate
                                            );
                                            assert_eq!(
                                                port.read_a2a_evidence(
                                                    &coordinate,
                                                    &response_coordinate,
                                                    8_192,
                                                )
                                                .unwrap()
                                                .unwrap()
                                                .coordinate(),
                                                &response_coordinate
                                            );
                                            assert!(terminal
                                                .record(OperationCustodyRecordKind::Diagnostic)
                                                .is_some());
                                            assert_eq!(
                                                port.read_a2a_evidence_inserted_at(
                                                    &coordinate,
                                                    terminal
                                                        .checkpoint()
                                                        .successor()
                                                        .committed_transaction_sequence(),
                                                    1,
                                                    8_192,
                                                )
                                                .unwrap()[0]
                                                    .coordinate(),
                                                &response_coordinate,
                                                "the terminal transaction discovers its exact response after in-memory phase loss"
                                            );
                                        }
                                        _ => panic!("response classification changed"),
                                    }
                                }
                            }
                            assert_eq!(
                                port.compare_and_commit(&candidate).unwrap_err().kind(),
                                ControlStoreErrorKind::CandidateConflict,
                                "every retained response advances beyond send preparation"
                            );
                            assert_eq!(starts.load(Ordering::SeqCst), 1);
                        });
                    },
                );
            },
        );
    });
}

#[test]
fn delegate_a2a_send_starts_only_after_new_sqlite_and_anchor_custody() {
    run_case(true, ResponseModel::Task, CancelResponseModel::Task);
}

#[test]
fn delegate_driver_preserves_response_and_eof_pending_phases() {
    run_case(
        true,
        ResponseModel::TaskWithPendingEof,
        CancelResponseModel::Task,
    );
}

#[test]
fn delegate_response_clock_unavailable_preserves_non_replayable_uncertainty() {
    run_case(
        true,
        ResponseModel::ResponseClockUnavailable,
        CancelResponseModel::Task,
    );
}

#[test]
fn delegate_response_clock_cannot_backdate_exact_eof() {
    run_case(
        true,
        ResponseModel::ResponseClockBackward,
        CancelResponseModel::Task,
    );
}

#[test]
fn delegate_response_clock_cannot_move_eof_beyond_retention() {
    run_case(
        true,
        ResponseModel::ResponseClockAfterRetention,
        CancelResponseModel::Task,
    );
}

#[test]
fn delegate_pre_custody_clock_failure_starts_no_carrier() {
    run_case(
        true,
        ResponseModel::PreCustodyClockUnavailable,
        CancelResponseModel::Task,
    );
}

#[test]
fn delegate_release_clock_failure_returns_unopened_anchored_custody() {
    run_case(
        true,
        ResponseModel::ReleaseClockUnavailable,
        CancelResponseModel::Task,
    );
}

#[test]
fn delegate_checkpoint_rejection_starts_no_carrier() {
    run_case(
        true,
        ResponseModel::CheckpointRejected,
        CancelResponseModel::Task,
    );
}

#[test]
fn delegate_commit_failure_starts_no_carrier() {
    run_case(
        true,
        ResponseModel::CommitUnavailable,
        CancelResponseModel::Task,
    );
}

#[test]
fn delegate_anchor_failure_starts_no_carrier() {
    run_case(
        true,
        ResponseModel::AnchorUnavailable,
        CancelResponseModel::Task,
    );
}

#[test]
fn committed_send_recovers_as_outcome_unknown_without_carrier_replay() {
    run_case(
        true,
        ResponseModel::ProcessLostBeforeStart,
        CancelResponseModel::Task,
    );
}

#[test]
fn recovered_send_clock_unavailable_preserves_unknown_custody_without_replay() {
    run_case(
        true,
        ResponseModel::RecoveredResponseClockUnavailable,
        CancelResponseModel::Task,
    );
}

#[test]
fn recovered_send_clock_cannot_backdate_retained_history() {
    run_case(
        true,
        ResponseModel::RecoveredResponseClockBackward,
        CancelResponseModel::Task,
    );
}

#[test]
fn context_bound_recovered_dispatch_starts_a2a_without_provider_replay() {
    run_case_mode(
        true,
        RouteProviderMatch::Exact,
        true,
        false,
        LocalStartContext::EvidenceOnly,
        ResponseModel::Task,
        CancelResponseModel::Task,
    );
}

#[test]
fn restarted_known_task_get_readback_commits_once_without_local_promotion() {
    run_case_mode(
        true,
        RouteProviderMatch::Exact,
        false,
        true,
        LocalStartContext::SelfContained,
        ResponseModel::Task,
        CancelResponseModel::Task,
    );
}

#[test]
fn cancel_task_carrier_rejection_is_not_remote_acceptance() {
    run_case(
        true,
        ResponseModel::Task,
        CancelResponseModel::StartRejected,
    );
}

#[test]
fn cancel_task_eof_is_uncertain_and_never_replayable() {
    run_case(true, ResponseModel::Task, CancelResponseModel::Eof);
}

#[test]
fn cancel_task_process_loss_recovers_with_get_task_without_replay() {
    run_case(
        true,
        ResponseModel::Task,
        CancelResponseModel::ProcessLostAfterStart,
    );
}

#[test]
fn cancel_task_retains_both_responses_when_unary_framing_is_violated() {
    run_case(
        true,
        ResponseModel::Task,
        CancelResponseModel::TaskThenExtra,
    );
}

#[test]
fn unlike_transport_trust_is_rejected_before_any_carrier_start() {
    run_case(false, ResponseModel::Task, CancelResponseModel::Task);
}

#[test]
fn unlike_route_provider_id_is_rejected_before_request_preparation() {
    run_case_with_route_provider(true, false, ResponseModel::Task, CancelResponseModel::Task);
}

#[test]
fn unlike_route_provider_identity_is_rejected_before_request_preparation() {
    run_case_with_route_provider_identity(false, ResponseModel::Task, CancelResponseModel::Task);
}

#[test]
fn direct_message_produces_no_task_binding_disposition() {
    run_case(
        true,
        ResponseModel::DirectMessage,
        CancelResponseModel::Task,
    );
}

#[test]
fn direct_message_retention_preserves_anchor_unavailable_failure() {
    run_case(
        true,
        ResponseModel::DirectMessageAnchorUnavailable,
        CancelResponseModel::Task,
    );
}

#[test]
fn direct_message_retention_preserves_anchor_outcome_unknown() {
    run_case(
        true,
        ResponseModel::DirectMessageAnchorOutcomeUnknown,
        CancelResponseModel::Task,
    );
}

#[test]
fn eof_before_task_reconciles_late_identity_without_send_replay() {
    run_case(true, ResponseModel::Eof, CancelResponseModel::Task);
}

#[test]
fn carrier_rejection_is_durably_not_started_without_remote_task_claim() {
    run_case(
        true,
        ResponseModel::StartRejected,
        CancelResponseModel::Task,
    );
}

#[test]
fn carrier_deadline_must_fit_before_operation_retention_expires() {
    run_case(
        true,
        ResponseModel::RetentionTooShort,
        CancelResponseModel::Task,
    );
}

#[test]
fn additional_unary_item_keeps_first_response_and_redacted_failure_class() {
    run_case(
        true,
        ResponseModel::TaskThenExtra,
        CancelResponseModel::Task,
    );
}
