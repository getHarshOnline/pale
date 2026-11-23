//! Checkpoint, database, and anchor phases for one prepared A2A request.

use super::*;
use crate::resident::{
    checkpoint_profile::{
        self, a2a_evidence_leaf, a2a_record_leaf, A2aCustodyLeaf, OperationLeafProjectionError,
    },
    operation_custody::{
        DatabaseCommittedOperationObservationAnchorPending,
        OperationObservationCandidatePreparationError,
    },
    CompletedOperationObservationCustody, OperationCheckpointSelection,
    PreparedOperationObservationCustody,
};
use pale_ports::{
    A2aExchange, A2aExchangeFacts, A2aExchangeInput, AccumulatorLeafChange,
    AccumulatorMembershipInputError, AccumulatorMembershipLeaf, AccumulatorMembershipLimits,
    AccumulatorPreparationBasis, AccumulatorPreparationLimits, AntiRollbackAnchor,
    AntiRollbackAnchorError, AntiRollbackAnchorPort, ControlCheckpointHead,
    ControlDomainAntiRollbackAnchor, ControlStoreError, OperationAccumulatorCheckpoint,
    OperationAccumulatorMembership, OperationAccumulatorPreparation, OperationCheckpointPhase,
    OperationCustodyCandidate, OperationCustodyCheckpoint, OperationCustodyIndexQuery,
    OperationCustodyRecordKind, OperationCustodyTransitionKind, PreparedOperationAccumulator,
    StateLeafChanges, UnresolvedCustodyLeafChanges, WallClockSourcePort,
};
use std::{
    task::{Context, Poll},
    time::Duration,
};

/// Exact accumulator roots for the prepared send; still no network permission.
#[must_use = "sign and retain this checkpoint before starting the A2A exchange"]
pub struct PreparedA2aSendCheckpoint<'operation, 'call> {
    append: PreparedA2aSend<'operation, 'call>,
    selection: OperationCheckpointSelection,
    predecessor_ref: ResidentRecordRefV1,
    predecessor: ControlStateCheckpointV1,
    changed_keys: Vec<ChangedControlLeafKeyV1>,
    leaf_count: ControlLeafCountV1,
    accumulator: PreparedOperationAccumulator,
}

/// Closed A2A send candidate awaiting its one database commit.
#[must_use = "commit and anchor the prepared A2A send before release"]
pub struct PreparedA2aSendCustody<'operation, 'call> {
    physical: PreparedOperationObservationCustody,
    release: A2aSendRelease<'operation, 'call>,
}

/// Database-committed send preparation awaiting independent anchoring.
#[must_use = "advance the independent anchor before releasing the A2A exchange"]
pub struct DatabaseCommittedA2aSendAnchorPending<'operation, 'call> {
    physical: DatabaseCommittedOperationObservationAnchorPending,
    release: A2aSendRelease<'operation, 'call>,
}

enum A2aSendCustodyFailurePhase {
    Commit(crate::resident::OperationObservationCustodyCommitFailure),
    Anchor(crate::resident::OperationObservationCustodyAnchorFailure),
    RecoveryOnly(CompletedOperationObservationCustody),
}

/// Exact send-preparation custody that has not yet granted carrier release.
///
/// This owner retains both the physical candidate and the A2A release payload.
/// It can continue only the existing database or anchor transition. Exact
/// recovery of an ambiguous database commit remains recovery-only and never
/// regains `SendMessage` permission.
#[must_use = "continue exact A2A custody or recover it without replaying SendMessage"]
pub struct A2aSendCustodyFailure<'operation, 'call> {
    release: A2aSendRelease<'operation, 'call>,
    phase: A2aSendCustodyFailurePhase,
}

impl std::fmt::Debug for A2aSendCustodyFailure<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match &self.phase {
            A2aSendCustodyFailurePhase::Commit(_) => "commit",
            A2aSendCustodyFailurePhase::Anchor(_) => "anchor",
            A2aSendCustodyFailurePhase::RecoveryOnly(_) => "recovery_only",
        };
        formatter
            .debug_struct("A2aSendCustodyFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Newly retained request bytes and preparation, still unopened by the carrier.
#[must_use = "recheck current authority and exact custody before starting the exchange"]
pub struct RetainedA2aSend<'operation, 'call> {
    custody: CompletedOperationObservationCustody,
    release: A2aSendRelease<'operation, 'call>,
}

/// One started exchange. It has no local result or remote Task claim yet.
#[must_use = "poll or cancel the started A2A exchange; dropping it proves no remote outcome"]
pub struct InFlightA2aSend {
    exchange: Box<dyn A2aExchange>,
    custody: CompletedOperationObservationCustody,
    context: A2aSendResponseContext,
}

struct A2aSendRelease<'operation, 'call> {
    dispatch: A2aProviderDispatch<'operation>,
    route: (&'call ResidentRecordRefV1, &'call A2aRouteAdmissionV1),
    exchange: A2aPreparedExchange,
    request_evidence: OperationA2aEvidence,
    preparation: ControlRecordCoordinate,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    maximum_evidence_bytes: usize,
    maximum_record_bytes: usize,
}

struct A2aSendResponseContext {
    dispatch: PostAdmissionA2aDispatch,
    route: (ResidentRecordRefV1, A2aRouteAdmissionV1),
    expected: pale_ports::A2aExchangeCoordinate,
    request_evidence: OperationA2aEvidence,
    preparation: ControlRecordCoordinate,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    maximum_evidence_bytes: usize,
}

/// Immutable facts admitted before the carrier starts.
///
/// These copies are historical association evidence only. They carry no
/// current session, provider, custody, anchor, signer, or replay authority.
struct PostAdmissionA2aDispatch {
    operation: PostAdmissionA2aOperation,
    selection_ref: ResidentRecordRefV1,
    preparation_observation_ref: ResidentRecordRefV1,
    observation: (ResidentRecordRefV1, OperationObservationV1),
}

struct PostAdmissionA2aOperation {
    checkpoint_selection: OperationCheckpointSelection,
    request_coordinate: ControlRecordCoordinate,
    handle: (ResidentRecordRefV1, OperationHandleV1),
}

impl PostAdmissionA2aDispatch {
    fn capture(dispatch: &A2aProviderDispatch<'_>) -> Self {
        let operation = dispatch.operation();
        let (handle_ref, handle) = operation.handle();
        let (observation_ref, observation) = dispatch.observation();
        Self {
            operation: PostAdmissionA2aOperation {
                checkpoint_selection: operation.checkpoint_selection(),
                request_coordinate: operation.request_coordinate().clone(),
                handle: (handle_ref.clone(), handle.clone()),
            },
            selection_ref: dispatch.selection_ref().clone(),
            preparation_observation_ref: dispatch.preparation_observation_ref().clone(),
            observation: (observation_ref.clone(), observation.clone()),
        }
    }

    fn task_key(
        &self,
        store: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<crate::resident::task_owner::ResidentOperationTaskKey, OperationAttemptError> {
        let record_sha256 = ArtifactDigest::parse(
            self.observation
                .1
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .map_err(|_| OperationAttemptError::Encoding)?;
        Ok(crate::resident::task_owner::ResidentOperationTaskKey::new(
            store.clone(),
            ImmutableRecordReferenceV1 {
                record_ref: self.observation.0.clone(),
                record_sha256,
            },
        ))
    }

    const fn operation(&self) -> &PostAdmissionA2aOperation {
        &self.operation
    }

    fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation.0, &self.observation.1)
    }

    const fn selection_ref(&self) -> &ResidentRecordRefV1 {
        &self.selection_ref
    }

    const fn preparation_observation_ref(&self) -> &ResidentRecordRefV1 {
        &self.preparation_observation_ref
    }
}

impl PostAdmissionA2aOperation {
    fn checkpoint_selection(&self) -> OperationCheckpointSelection {
        self.checkpoint_selection.clone()
    }

    const fn request_coordinate(&self) -> &ControlRecordCoordinate {
        &self.request_coordinate
    }

    const fn handle(&self) -> (&ResidentRecordRefV1, &OperationHandleV1) {
        (&self.handle.0, &self.handle.1)
    }
}

/// One started send with its fresh response identity fixed before polling.
#[must_use = "poll the same exchange until one response or a closed uncertain outcome"]
pub struct AwaitingA2aSendResponse {
    exchange: Box<dyn A2aExchange>,
    custody: CompletedOperationObservationCustody,
    context: A2aSendResponseContext,
    response_ref: ResidentRecordRefV1,
}

/// One retained response item awaiting exact unary end-of-stream.
#[must_use = "a unary A2A response is incomplete until the exchange reaches EOF"]
pub struct ReceivedA2aSendResponse {
    exchange: Box<dyn A2aExchange>,
    custody: CompletedOperationObservationCustody,
    context: A2aSendResponseContext,
    response: OperationA2aEvidence,
    facts: A2aInitialResponseFacts,
}

/// Valid first response classifications before Pale assigns any local outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A2aInitialResponseFacts {
    /// The response contains an actual remote Task identity.
    Task {
        /// Remote-owned task identity.
        task: A2aRemoteTaskIdV1,
        /// Remote-owned context identity.
        context: A2aRemoteContextIdV1,
        /// Remote-owned state, not a Pale result or settlement.
        state: A2aRemoteTaskStateV1,
    },
    /// The remote endpoint returned a Message without a Task.
    DirectMessage,
    /// A status event is not the required initial Task response.
    Status,
    /// An artifact event is not the required initial Task response.
    Artifact,
    /// The remote endpoint returned a JSON-RPC error.
    RemoteError,
    /// Streaming metadata appeared on the selected unary route.
    UnexpectedStreaming,
}

/// One completed unary response, classified without inventing Task identity.
pub enum CompletedA2aSendResponse {
    /// An actual Task was identified and may proceed to existing task custody.
    Task(Box<CompletedA2aTaskResponse>),
    /// The response cannot create a Task binding.
    NoTask(Box<CompletedA2aNoTaskResponse>),
}

/// Exact Task response evidence after unary EOF.
#[must_use = "the identified Task still requires signed binding and history custody"]
pub struct CompletedA2aTaskResponse {
    custody: CompletedOperationObservationCustody,
    context: A2aSendResponseContext,
    response: OperationA2aEvidence,
    task: A2aRemoteTaskIdV1,
    remote_context: A2aRemoteContextIdV1,
    state: A2aRemoteTaskStateV1,
    observed_at: UnixTimeV1,
}

/// Fresh signed records needed to continue an identified Task into existing custody.
pub struct A2aInitialTaskCustodyInput<'a> {
    /// Anchored recovery of the exact dispatched Pale operation.
    pub operation_custody: &'a crate::resident::RecoveredOperationObservation,
    /// Fresh immutable binding reference.
    pub binding_ref: ResidentRecordRefV1,
    /// Candidate binding. The response phase rechecks every exchange-owned field.
    pub binding: A2aTaskBindingPreimageV1,
    /// Exact binding schema and current signer.
    pub binding_signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh immutable initial remote-history reference.
    pub observation_ref: ResidentRecordRefV1,
    /// Candidate revision-one observation for this exact Task response.
    pub observation: A2aTaskObservationPreimageV1,
    /// Exact observation schema and current signer.
    pub observation_signing: SignedResidentAdmissionContextV1<'a>,
    /// Aggregate exact request and response byte ceiling.
    pub maximum_evidence_bytes: usize,
    /// Maximum canonical bytes for each signed record.
    pub maximum_record_bytes: usize,
}

/// Exact task-less or incompatible response after unary EOF.
#[must_use = "retain the diagnostic and response evidence; no Task binding exists"]
pub struct CompletedA2aNoTaskResponse {
    custody: CompletedOperationObservationCustody,
    context: A2aSendResponseContext,
    response: OperationA2aEvidence,
    facts: A2aInitialResponseFacts,
    diagnostic: crate::diagnostic::AdmittedDiagnostic,
    observed_at: UnixTimeV1,
}

/// A started send whose Task outcome cannot be established from retained evidence.
#[must_use = "reconcile this outcome; never replay the original SendMessage request"]
pub struct A2aSendOutcomeUnknown {
    custody: CompletedOperationObservationCustody,
    context: A2aSendResponseContext,
    response: Option<OperationA2aEvidence>,
    diagnostic: crate::diagnostic::AdmittedDiagnostic,
}

/// A retained send that the carrier proved it did not accept.
#[must_use = "record the failed observation; no remote request started"]
pub struct A2aSendNotStarted {
    custody: CompletedOperationObservationCustody,
    context: A2aSendResponseContext,
    reason: A2aExchangeError,
    diagnostic: crate::diagnostic::AdmittedDiagnostic,
}

/// Fresh identities and authority for a confirmed failed send.
pub struct A2aFailedSendObservationInput<'a> {
    /// Fresh immutable diagnostic reference.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Fresh immutable terminal observation reference.
    pub observation_ref: ResidentRecordRefV1,
    /// Selected record-time source. Historical custody supplies its lower bound.
    pub clock: &'a dyn WallClockSourcePort,
    /// Independently admitted observation signer.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Maximum canonical diagnostic bytes retained with the observation.
    pub maximum_diagnostic_bytes: usize,
}

/// Fresh identities, authority, and existing recovery frontier for an uncertain send.
pub struct A2aOutcomeUnknownObservationInput<'a> {
    /// Fresh immutable diagnostic reference.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Fresh immutable outcome-unknown observation reference.
    pub observation_ref: ResidentRecordRefV1,
    /// Exact existing reconciliation frontier governing recovery.
    pub reconciliation: ReconciliationFrontierReferenceV1,
    /// Selected record-time source. Historical custody supplies its lower bound.
    pub clock: &'a dyn WallClockSourcePort,
    /// Independently admitted observation signer.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Maximum canonical diagnostic bytes retained with the observation.
    pub maximum_diagnostic_bytes: usize,
}

/// Signed terminal or uncertain observation with its diagnostic and response bytes.
#[must_use = "retain the observation, diagnostic, and any response evidence atomically"]
pub struct PreparedA2aSendObservation {
    custody: CompletedOperationObservationCustody,
    context: A2aSendResponseContext,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    diagnostic: crate::diagnostic::AdmittedDiagnostic,
    diagnostic_record: ControlRecord,
    response: Option<OperationA2aEvidence>,
}

/// Rejected task-less recording with the exact response phase retained.
pub struct A2aNoTaskObservationFailure {
    response: Box<CompletedA2aNoTaskResponse>,
    reason: OperationAttemptError,
}

/// Rejected not-started recording with the exact retained phase preserved.
pub struct A2aNotStartedObservationFailure {
    not_started: Box<A2aSendNotStarted>,
    reason: OperationAttemptError,
}

/// Rejected uncertainty recording with the exact non-replayable phase retained.
pub struct A2aOutcomeUnknownObservationFailure {
    outcome: Box<A2aSendOutcomeUnknown>,
    reason: OperationAttemptError,
}

enum A2aSendDiagnosticFailureKind {
    NotStarted(A2aExchangeError),
    NoTask {
        facts: A2aInitialResponseFacts,
        observed_at: UnixTimeV1,
    },
    OutcomeUnknown(&'static str),
}

/// Reconstructed A2A disposition after the exact diagnostic becomes admissible.
#[must_use = "retain the recovered A2A disposition through durable outcome custody"]
pub enum RecoveredA2aSendDiagnostic {
    /// The carrier proved that it did not accept the request.
    NotStarted(Box<A2aSendNotStarted>),
    /// The initial response was valid but identified no remote Task.
    NoTask(Box<CompletedA2aNoTaskResponse>),
    /// The carrier effect or remote outcome remains unknown.
    OutcomeUnknown(Box<A2aSendOutcomeUnknown>),
}

/// Diagnostic admission failed; all request and response evidence remains owned.
pub struct A2aSendDiagnosticFailure {
    custody: CompletedOperationObservationCustody,
    context: A2aSendResponseContext,
    response: Option<OperationA2aEvidence>,
    kind: A2aSendDiagnosticFailureKind,
    reason: crate::diagnostic::DiagnosticAdmissionError,
}

impl std::fmt::Debug for A2aNoTaskObservationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("A2aNoTaskObservationFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for A2aOutcomeUnknownObservationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("A2aOutcomeUnknownObservationFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for A2aNotStartedObservationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("A2aNotStartedObservationFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl A2aNoTaskObservationFailure {
    /// Borrow the closed reason; the response phase remains owned here.
    #[must_use]
    pub const fn reason(&self) -> &OperationAttemptError {
        &self.reason
    }
}

impl A2aNoTaskObservationFailure {
    /// Recover the exact task-less response phase for a corrected recording attempt.
    pub fn into_response(self) -> CompletedA2aNoTaskResponse {
        *self.response
    }
}

impl A2aOutcomeUnknownObservationFailure {
    /// Borrow the closed reason; the uncertain phase remains owned here.
    #[must_use]
    pub const fn reason(&self) -> &OperationAttemptError {
        &self.reason
    }
}

impl A2aOutcomeUnknownObservationFailure {
    /// Recover the exact uncertain phase for a corrected recording attempt.
    pub fn into_outcome(self) -> A2aSendOutcomeUnknown {
        *self.outcome
    }
}

impl A2aNotStartedObservationFailure {
    /// Borrow the closed reason; the not-started phase remains owned here.
    #[must_use]
    pub const fn reason(&self) -> &OperationAttemptError {
        &self.reason
    }
}

impl A2aNotStartedObservationFailure {
    /// Recover the exact not-started phase for a corrected recording attempt.
    pub fn into_not_started(self) -> A2aSendNotStarted {
        *self.not_started
    }
}

impl A2aSendDiagnosticFailure {
    /// Borrow the admission failure without losing carrier evidence.
    #[must_use]
    pub const fn reason(&self) -> &crate::diagnostic::DiagnosticAdmissionError {
        &self.reason
    }

    /// Borrow the retained request and optional first response bytes.
    #[must_use]
    pub fn evidence(&self) -> (&OperationA2aEvidence, Option<&OperationA2aEvidence>) {
        (&self.context.request_evidence, self.response.as_ref())
    }

    /// Borrow the send-preparation custody. It never permits replay.
    #[must_use]
    pub const fn send_custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }

    /// Borrow the task-less facts when response classification was complete.
    #[must_use]
    pub const fn no_task_facts(&self) -> Option<&A2aInitialResponseFacts> {
        match &self.kind {
            A2aSendDiagnosticFailureKind::NotStarted(_) => None,
            A2aSendDiagnosticFailureKind::NoTask { facts, .. } => Some(facts),
            A2aSendDiagnosticFailureKind::OutcomeUnknown(_) => None,
        }
    }

    /// Borrow the carrier rejection when no remote request started.
    #[must_use]
    pub const fn not_started_reason(&self) -> Option<A2aExchangeError> {
        match &self.kind {
            A2aSendDiagnosticFailureKind::NotStarted(reason) => Some(*reason),
            A2aSendDiagnosticFailureKind::NoTask { .. }
            | A2aSendDiagnosticFailureKind::OutcomeUnknown(_) => None,
        }
    }

    /// Borrow the recovery summary when the carrier outcome was uncertain.
    #[must_use]
    pub const fn outcome_unknown_summary(&self) -> Option<&'static str> {
        match &self.kind {
            A2aSendDiagnosticFailureKind::NotStarted(_)
            | A2aSendDiagnosticFailureKind::NoTask { .. } => None,
            A2aSendDiagnosticFailureKind::OutcomeUnknown(summary) => Some(*summary),
        }
    }

    /// Retry only diagnostic construction from the retained carrier evidence.
    ///
    /// This method performs no carrier, storage, signing, anchor, or clock I/O.
    /// It never recreates request bytes and never restores `SendMessage`
    /// permission.
    ///
    /// # Errors
    ///
    /// Returns the complete unchanged evidence owner when the diagnostic remains
    /// inadmissible.
    pub fn retry_diagnostic(self: Box<Self>) -> Result<RecoveredA2aSendDiagnostic, Box<Self>> {
        let Self {
            custody,
            context,
            response,
            kind,
            reason: original_reason,
        } = *self;
        match kind {
            A2aSendDiagnosticFailureKind::NotStarted(reason) => {
                match not_started(custody, context, reason) {
                    Ok(value) => Ok(RecoveredA2aSendDiagnostic::NotStarted(Box::new(value))),
                    Err(failure) => Err(failure),
                }
            }
            A2aSendDiagnosticFailureKind::NoTask { facts, observed_at } => {
                match no_task_diagnostic(&context, &facts) {
                    Ok(diagnostic) => {
                        let Some(response) = response else {
                            return Err(Box::new(Self {
                                custody,
                                context,
                                response: None,
                                kind: A2aSendDiagnosticFailureKind::NoTask { facts, observed_at },
                                reason: original_reason,
                            }));
                        };
                        Ok(RecoveredA2aSendDiagnostic::NoTask(Box::new(
                            CompletedA2aNoTaskResponse {
                                custody,
                                context,
                                response,
                                facts,
                                diagnostic,
                                observed_at,
                            },
                        )))
                    }
                    Err(reason) => Err(Box::new(Self {
                        custody,
                        context,
                        response,
                        kind: A2aSendDiagnosticFailureKind::NoTask { facts, observed_at },
                        reason,
                    })),
                }
            }
            A2aSendDiagnosticFailureKind::OutcomeUnknown(summary) => {
                match unknown_response(custody, context, response, summary) {
                    A2aSendResponseFailure::OutcomeUnknown(value) => {
                        Ok(RecoveredA2aSendDiagnostic::OutcomeUnknown(value))
                    }
                    A2aSendResponseFailure::Diagnostic(failure) => Err(failure),
                }
            }
        }
    }
}

enum A2aResponseObservationKind {
    Failed,
    OutcomeUnknown(ReconciliationFrontierReferenceV1),
}

/// One wake-driven response transition that preserves its pending phase.
pub enum A2aSendResponsePoll<Pending, Ready, Failure> {
    /// No item is ready. Poll this same value after the supplied wake.
    Pending(Pending),
    /// The phase advanced or closed with an explicit disposition.
    Ready(Result<Ready, Failure>),
}

/// Failure while classifying a started send. No variant permits request replay.
pub enum A2aSendResponseFailure {
    /// The carrier outcome requires reconciliation before any new attempt.
    OutcomeUnknown(Box<A2aSendOutcomeUnknown>),
    /// Pale could not admit its own diagnostic contract after carrier start.
    Diagnostic(Box<A2aSendDiagnosticFailure>),
}

impl std::fmt::Debug for A2aSendResponseFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutcomeUnknown(_) => {
                formatter.write_str("A2aSendResponseFailure::OutcomeUnknown")
            }
            Self::Diagnostic(failure) => formatter
                .debug_tuple("A2aSendResponseFailure::Diagnostic")
                .field(&failure.reason)
                .finish(),
        }
    }
}

/// A send stopped either before or after its one allowed carrier start.
pub enum A2aSendStartFailure<'operation, 'call> {
    /// No carrier start occurred; the exact retained preparation is recoverable.
    Preflight {
        /// Exact unopened preparation.
        retained: Box<RetainedA2aSend<'operation, 'call>>,
        /// Closed reason for refusing release.
        reason: A2aSendReleaseError,
    },
    /// The carrier rejected the request before queue acceptance.
    Rejected(Box<A2aSendNotStarted>),
    /// The carrier accepted the request but returned an unlike exchange coordinate.
    OutcomeUnknown(Box<A2aSendOutcomeUnknown>),
    /// Pale could not admit the required not-started or uncertain diagnostic.
    Diagnostic(Box<A2aSendDiagnosticFailure>),
}

impl std::fmt::Debug for A2aSendStartFailure<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Preflight { reason, .. } => formatter
                .debug_struct("A2aSendStartFailure")
                .field("phase", &"preflight")
                .field("reason", reason)
                .finish_non_exhaustive(),
            Self::Rejected(rejected) => formatter
                .debug_struct("A2aSendStartFailure")
                .field("phase", &"rejected")
                .field("reason", &rejected.reason)
                .finish_non_exhaustive(),
            Self::OutcomeUnknown(_) => formatter
                .debug_struct("A2aSendStartFailure")
                .field("phase", &"outcome_unknown")
                .finish_non_exhaustive(),
            Self::Diagnostic(failure) => formatter
                .debug_struct("A2aSendStartFailure")
                .field("phase", &"diagnostic")
                .field("reason", &failure.reason)
                .finish_non_exhaustive(),
        }
    }
}

/// Closed reason why retained request bytes could not be released.
#[derive(Debug, thiserror::Error)]
pub enum A2aSendReleaseError {
    /// Current operation authority is unavailable or changed.
    #[error(transparent)]
    Operation(#[from] OperationAttemptError),
    /// Stored preparation, request bytes, checkpoint, route, or carrier differs.
    #[error("retained A2A send custody does not match the selected request")]
    Association,
    /// A bounded custody read failed.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// Independent anchor evidence could not be read.
    #[error(transparent)]
    Anchor(#[from] AntiRollbackAnchorError),
    /// The carrier's complete deadline would outlive operation retention.
    #[error("A2A exchange deadline exceeds the operation retention period")]
    InsufficientRetention,
}

/// Failure while constructing the send-preparation checkpoint.
#[derive(Debug, thiserror::Error)]
pub enum A2aSendCheckpointError {
    /// One retained parent, checkpoint, or authority coordinate was substituted.
    #[error("A2A send checkpoint does not match its retained dispatch and route")]
    Association,
    /// Physical projection or checkpoint construction failed.
    #[error(transparent)]
    Projection(#[from] OperationObservationCustodyProjectionError),
    /// Retained membership input was invalid.
    #[error(transparent)]
    Membership(#[from] AccumulatorMembershipInputError),
    /// Core checkpoint admission rejected the candidate.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
}

impl<'operation, 'call> PreparedA2aSend<'operation, 'call> {
    /// Prove retained route membership and prepare the send-preparation leaves.
    ///
    /// A fresh dispatch uses its committing checkpoint. Protected recovery uses
    /// the authenticated current checkpoint whose state still selects the same
    /// `Dispatched` operation head. Unresolved custody remains unchanged.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected checkpoint, retained route, evidence,
    /// membership proof, physical transaction, or finite bound differs.
    pub fn prepare_checkpoint(
        self,
        selection: OperationCheckpointSelection,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &ControlStateCheckpointV1,
        limits: AccumulatorPreparationLimits,
        membership_limits: AccumulatorMembershipLimits,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<PreparedA2aSendCheckpoint<'operation, 'call>, A2aSendCheckpointError> {
        let (expected_ref, expected) = self.dispatch.checkpoint();
        selection
            .verify(
                &self.store,
                self.dispatch.operation().handle().1,
                predecessor,
            )
            .map_err(|_| A2aSendCheckpointError::Association)?;
        if expected_ref != predecessor_ref || expected != predecessor {
            return Err(A2aSendCheckpointError::Association);
        }
        if let A2aProviderDispatch::Fresh(dispatch) = &self.dispatch {
            if dispatch
                .custody()
                .candidate()
                .checkpoint()
                .successor_record()
                != &checkpoint_record(predecessor_ref, predecessor, self.maximum_record_bytes)?
            {
                return Err(A2aSendCheckpointError::Association);
            }
        }
        let prior = OperationAccumulatorCheckpoint::new(
            self.store.clone(),
            predecessor.checkpoint_profile_ref().clone(),
            predecessor.checkpoint_profile_sha256().clone(),
            head(predecessor_ref, predecessor)?,
            predecessor.state_root_sha256().clone(),
            predecessor.unresolved_custody_root_sha256().clone(),
        );
        let mut changes = Vec::new();
        let mut changed_keys = Vec::new();
        let mut retained = Vec::new();
        let mut inserted = 0u64;
        for record in &self.records {
            let (class, logical) = match record.kind() {
                pale_ports::OperationCustodyRecordKind::A2aSendPreparation => (
                    "a2a-send-preparation",
                    record.record().coordinate().record_ref().as_str(),
                ),
                pale_ports::OperationCustodyRecordKind::A2aRouteAdmission => (
                    "a2a-route-admission",
                    record.record().coordinate().record_ref().as_str(),
                ),
                _ => return Err(A2aSendCheckpointError::Association),
            };
            push_change(
                a2a_record_leaf(
                    &self.store,
                    predecessor,
                    class,
                    logical,
                    record.record().coordinate(),
                )
                .map_err(leaf_error)?,
                &mut changes,
                &mut changed_keys,
            )?;
            inserted += 1;
        }
        for record in &self.dependencies.retained_records {
            match record.schema_id().as_str() {
                A2A_ROUTE_ADMISSION_V1_SCHEMA => retained.push(AccumulatorMembershipLeaf::new(
                    a2a_record_leaf(
                        &self.store,
                        predecessor,
                        "a2a-route-admission",
                        record.coordinate().record_ref().as_str(),
                        record.coordinate(),
                    )
                    .map_err(leaf_error)?
                    .hashed_key,
                    a2a_record_leaf(
                        &self.store,
                        predecessor,
                        "a2a-route-admission",
                        record.coordinate().record_ref().as_str(),
                        record.coordinate(),
                    )
                    .map_err(leaf_error)?
                    .value,
                )?),
                OPERATION_HANDLE_V1_SCHEMA | OPERATION_OBSERVATION_V1_SCHEMA => {}
                _ => return Err(A2aSendCheckpointError::Association),
            }
        }
        if let Some(route) = &self.dependencies.associated_route {
            push_change(
                a2a_record_leaf(
                    &self.store,
                    predecessor,
                    "a2a-route-admission",
                    route.coordinate().record_ref().as_str(),
                    route.coordinate(),
                )
                .map_err(leaf_error)?,
                &mut changes,
                &mut changed_keys,
            )?;
            inserted += 1;
        }
        let evidence_leaf = a2a_evidence_leaf(&self.store, predecessor, &self.request_evidence)
            .map_err(leaf_error)?;
        push_change(evidence_leaf, &mut changes, &mut changed_keys)?;
        inserted += 1;
        if self.dependencies.new_evidence.as_slice() != [self.request_evidence.clone()]
            || !self.dependencies.retained_evidence.is_empty()
        {
            return Err(A2aSendCheckpointError::Association);
        }
        if !retained.is_empty() {
            retained.sort_by(|left, right| left.key().cmp(right.key()));
            let request = OperationAccumulatorMembership::new(
                prior.clone(),
                retained,
                membership_limits,
                selection.implementation(),
            )?;
            let verified = custody
                .verify_accumulator_membership(&request)
                .map_err(OperationObservationCustodyProjectionError::from)?;
            if verified.request() != &request {
                return Err(A2aSendCheckpointError::Association);
            }
        }
        changes.sort_by(|left, right| left.key().cmp(right.key()));
        changed_keys.sort();
        let leaf_count = ControlLeafCountV1::new(
            predecessor
                .leaf_count()
                .get()
                .checked_add(inserted)
                .ok_or(A2aSendCheckpointError::Association)?,
        )?;
        let state = StateLeafChanges::new(changes, limits.maximum_leaf_changes())
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let unresolved =
            UnresolvedCustodyLeafChanges::new(Vec::new(), limits.maximum_leaf_changes())
                .map_err(OperationObservationCustodyProjectionError::from)?;
        let transaction = custody
            .prepare_transaction(&self.store)
            .map_err(pale_ports::OperationAccumulatorPreparationError::from)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let request = OperationAccumulatorPreparation::new(
            AccumulatorPreparationBasis::Successor(prior),
            state,
            unresolved,
            limits,
            selection.implementation(),
            transaction,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let accumulator = custody
            .prepare_accumulator(&request)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        if accumulator.request() != &request {
            return Err(A2aSendCheckpointError::Association);
        }
        Ok(PreparedA2aSendCheckpoint {
            append: self,
            selection,
            predecessor_ref: predecessor_ref.clone(),
            predecessor: predecessor.clone(),
            changed_keys,
            leaf_count,
            accumulator,
        })
    }

    /// Prepare roots, issue the successor checkpoint, and close send custody.
    ///
    /// This combines the adjacent pre-commit phases that all describe the same
    /// retained request. It performs no database write, anchor advance, or
    /// carrier start.
    ///
    /// # Errors
    ///
    /// Returns an error when the predecessor, route membership, accumulator
    /// roots, checkpoint authority, or canonical checkpoint is not exact.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_custody<Sign>(
        self,
        identity: OperationCheckpointInput,
        issued_at: UnixTimeV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        limits: AccumulatorPreparationLimits,
        membership_limits: AccumulatorMembershipLimits,
        custody: &mut dyn OperationCustodyPort,
        sign: Sign,
    ) -> Result<PreparedA2aSendCustody<'operation, 'call>, A2aSendCheckpointError>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let selection = self.dispatch.operation().checkpoint_selection();
        let (predecessor_ref, predecessor) = self.predecessor_checkpoint();
        let prepared = self.prepare_checkpoint(
            selection,
            &predecessor_ref,
            &predecessor,
            limits,
            membership_limits,
            custody,
        )?;
        let preimage = a2a_send_checkpoint_preimage(
            &prepared,
            (&predecessor_ref, &predecessor),
            &identity,
            issued_at,
            schema,
            authority,
        )?;
        prepared.issue_checkpoint(&identity.reference, preimage, schema, authority, sign)
    }
}

impl PreparedA2aSendCheckpoint<'_, '_> {
    /// Return the transaction sequence assigned to the prepared successor.
    #[must_use]
    pub fn transaction_sequence(&self) -> ControlTransactionSequenceV1 {
        self.accumulator.request().transaction().successor()
    }

    /// Return the state root produced by the prepared accumulator update.
    #[must_use]
    pub fn state_root(&self) -> &ArtifactDigest {
        self.accumulator.state_root()
    }

    /// Return the unchanged unresolved-custody root.
    #[must_use]
    pub fn unresolved_root(&self) -> &ArtifactDigest {
        self.accumulator.unresolved_root()
    }

    /// Return the logical leaf keys changed by this send preparation.
    #[must_use]
    pub fn changed_keys(&self) -> &[ChangedControlLeafKeyV1] {
        &self.changed_keys
    }

    /// Return the number of leaves in the prepared successor state.
    #[must_use]
    pub const fn leaf_count(&self) -> ControlLeafCountV1 {
        self.leaf_count
    }
}

impl<'operation, 'call> PreparedA2aSendCheckpoint<'operation, 'call> {
    /// Sign the exact successor and close the physical send-preparation atom.
    ///
    /// # Errors
    ///
    /// Returns an error when the preimage, signing authority, predecessor,
    /// accumulator roots, changed keys, or projected custody differs.
    #[allow(clippy::too_many_arguments)]
    pub fn issue_checkpoint<Sign>(
        self,
        successor_ref: &ResidentRecordRefV1,
        preimage: ControlStateCheckpointPreimageV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<PreparedA2aSendCustody<'operation, 'call>, A2aSendCheckpointError>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let basis = self.accumulator.basis();
        if !self.selection.matches_authority(authority)
            || authority.principal_id
                != self
                    .append
                    .dispatch
                    .operation()
                    .handle()
                    .1
                    .issuer_principal_id()
            || &preimage.control_store_id != basis.store().store_id()
            || &preimage.control_incarnation_id != basis.store().incarnation_id()
            || &preimage.checkpoint_domain_id != basis.domain()
            || preimage.domain_epoch != basis.epoch()
            || preimage.committed_transaction_sequence != self.transaction_sequence()
            || &preimage.checkpoint_profile_ref != basis.profile_ref()
            || &preimage.checkpoint_profile_sha256 != basis.profile_sha256()
            || preimage.control_schema_version != checkpoint_profile::CONTROL_SCHEMA_VERSION
            || &preimage.state_root_sha256 != self.state_root()
            || &preimage.unresolved_custody_root_sha256 != self.unresolved_root()
            || preimage.changed_leaf_keys != self.changed_keys
            || preimage.leaf_count != self.leaf_count
            || preimage.issued_at < *self.append.dispatch.observation().1.observed_at()
        {
            return Err(A2aSendCheckpointError::Association);
        }
        let successor = ControlStateCheckpointV1::issue_successor_contextual(
            preimage,
            &self.predecessor_ref,
            &self.predecessor,
            schema,
            authority,
            sign,
        )?;
        let successor_head = head(successor_ref, &successor)?;
        let checkpoint = OperationCustodyCheckpoint::new(
            OperationCheckpointPhase::Successor {
                predecessor: head(&self.predecessor_ref, &self.predecessor)?,
                predecessor_record: Box::new(checkpoint_record(
                    &self.predecessor_ref,
                    &self.predecessor,
                    self.append.maximum_record_bytes,
                )?),
            },
            successor_head.clone(),
            checkpoint_record(successor_ref, &successor, self.append.maximum_record_bytes)?,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let batch = self
            .accumulator
            .bind_successor(successor_head)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let PreparedA2aSend {
            dispatch,
            route,
            preparation,
            request_evidence,
            exchange,
            store,
            records,
            indexes,
            dependencies,
            maximum_evidence_bytes,
            maximum_record_bytes,
        } = self.append;
        let release = A2aSendRelease {
            dispatch,
            route,
            exchange,
            request_evidence,
            preparation: preparation.coordinate().clone(),
            checkpoint: (successor_ref.clone(), successor),
            maximum_evidence_bytes,
            maximum_record_bytes,
        };
        let candidate = OperationCustodyCandidate::new_a2a_custody(
            store,
            OperationCustodyTransitionKind::PrepareA2aSend,
            records,
            indexes,
            checkpoint,
            batch,
            dependencies,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let physical =
            crate::resident::operation_custody::prepare_operation_observation_candidate(candidate)
                .map_err(|error| match error {
                    OperationObservationCandidatePreparationError::NotObservation => {
                        A2aSendCheckpointError::Association
                    }
                    OperationObservationCandidatePreparationError::Anchor(error) => {
                        A2aSendCheckpointError::Projection(error.into())
                    }
                })?;
        Ok(PreparedA2aSendCustody { physical, release })
    }
}

impl<'operation, 'call> PreparedA2aSendCustody<'operation, 'call> {
    /// Commit the closed preparation and leave its independent anchor pending.
    ///
    /// # Errors
    ///
    /// Returns an error when the database commit or its anchor preflight fails.
    pub fn commit(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<
        DatabaseCommittedA2aSendAnchorPending<'operation, 'call>,
        Box<A2aSendCustodyFailure<'operation, 'call>>,
    > {
        match self.physical.commit(custody, anchor) {
            Ok(physical) => Ok(DatabaseCommittedA2aSendAnchorPending {
                physical,
                release: self.release,
            }),
            Err(failure) => Err(Box::new(A2aSendCustodyFailure {
                release: self.release,
                phase: A2aSendCustodyFailurePhase::Commit(failure),
            })),
        }
    }
}

impl<'operation, 'call> DatabaseCommittedA2aSendAnchorPending<'operation, 'call> {
    /// Report whether this call made a new database commit or recovered one.
    #[must_use]
    pub const fn database_evidence(&self) -> crate::resident::OperationCustodyDatabaseEvidence {
        self.physical.database_evidence()
    }

    /// Advance the independent anchor for the exact committed checkpoint.
    ///
    /// Only a database commit made by this call can yield a release-capable
    /// value. Recovered or replayed commits remain observable but cannot send.
    ///
    /// # Errors
    ///
    /// Returns an error when anchoring fails or the commit was not new.
    pub fn advance_anchor(
        self,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<RetainedA2aSend<'operation, 'call>, Box<A2aSendCustodyFailure<'operation, 'call>>>
    {
        let database_evidence = self.physical.database_evidence();
        let custody = match self.physical.advance_anchor(anchor) {
            Ok(custody) => custody,
            Err(failure) => {
                return Err(Box::new(A2aSendCustodyFailure {
                    release: self.release,
                    phase: A2aSendCustodyFailurePhase::Anchor(failure),
                }));
            }
        };
        if database_evidence != crate::resident::OperationCustodyDatabaseEvidence::New {
            return Err(Box::new(A2aSendCustodyFailure {
                release: self.release,
                phase: A2aSendCustodyFailurePhase::RecoveryOnly(custody),
            }));
        }
        Ok(RetainedA2aSend {
            custody,
            release: self.release,
        })
    }
}

impl<'operation, 'call> A2aSendCustodyFailure<'operation, 'call> {
    /// Report whether the exact database commit still needs resolution.
    #[must_use]
    pub const fn is_commit_unresolved(&self) -> bool {
        matches!(self.phase, A2aSendCustodyFailurePhase::Commit(_))
    }

    /// Report whether the exact independent anchor still needs resolution.
    #[must_use]
    pub const fn is_anchor_unresolved(&self) -> bool {
        matches!(self.phase, A2aSendCustodyFailurePhase::Anchor(_))
    }

    /// Continue only the retained physical candidate and anchor transition.
    ///
    /// An ambiguous commit receives exact read-back first. A database or anchor
    /// mutation is retried only when its existing owner proves that retry is
    /// safe. A recovered commit cannot regain carrier-release permission.
    ///
    /// # Errors
    ///
    /// Returns this exact owner when custody remains unresolved or has become
    /// recovery-only.
    pub fn continue_exact(
        self: Box<Self>,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<RetainedA2aSend<'operation, 'call>, Box<Self>> {
        let Self { release, phase } = *self;
        let pending = match phase {
            A2aSendCustodyFailurePhase::Commit(failure) => match failure.reconcile_exact(custody) {
                Ok(pending) => pending,
                Err(failure) => match failure.try_into_retryable() {
                    Ok(prepared) => match prepared.commit(custody, anchor) {
                        Ok(pending) => pending,
                        Err(failure) => {
                            return Err(Box::new(Self {
                                release,
                                phase: A2aSendCustodyFailurePhase::Commit(failure),
                            }));
                        }
                    },
                    Err(failure) => {
                        return Err(Box::new(Self {
                            release,
                            phase: A2aSendCustodyFailurePhase::Commit(failure),
                        }));
                    }
                },
            },
            A2aSendCustodyFailurePhase::Anchor(failure) => match failure.try_into_retryable() {
                Ok(pending) => pending,
                Err(failure) => match failure.continue_after_failed_observation(anchor) {
                    Ok(completed) => {
                        if completed.database_evidence()
                            != crate::resident::OperationCustodyDatabaseEvidence::New
                        {
                            return Err(Box::new(Self {
                                release,
                                phase: A2aSendCustodyFailurePhase::RecoveryOnly(completed),
                            }));
                        }
                        return Ok(RetainedA2aSend {
                            custody: completed,
                            release,
                        });
                    }
                    Err(failure) => {
                        return Err(Box::new(Self {
                            release,
                            phase: A2aSendCustodyFailurePhase::Anchor(failure),
                        }));
                    }
                },
            },
            A2aSendCustodyFailurePhase::RecoveryOnly(custody) => {
                return Err(Box::new(Self {
                    release,
                    phase: A2aSendCustodyFailurePhase::RecoveryOnly(custody),
                }));
            }
        };
        let database_evidence = pending.database_evidence();
        let completed = match pending.advance_anchor(anchor) {
            Ok(completed) => completed,
            Err(failure) => {
                return Err(Box::new(Self {
                    release,
                    phase: A2aSendCustodyFailurePhase::Anchor(failure),
                }));
            }
        };
        if database_evidence != crate::resident::OperationCustodyDatabaseEvidence::New {
            return Err(Box::new(Self {
                release,
                phase: A2aSendCustodyFailurePhase::RecoveryOnly(completed),
            }));
        }
        Ok(RetainedA2aSend {
            custody: completed,
            release,
        })
    }

    /// Report whether exact recovery closed custody without restoring send permission.
    #[must_use]
    pub const fn is_recovery_only(&self) -> bool {
        matches!(self.phase, A2aSendCustodyFailurePhase::RecoveryOnly(_))
    }
}

impl RetainedA2aSend<'_, '_> {
    /// Return the exact operation custody that permits this one send.
    #[must_use]
    pub const fn custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }

    /// Return the coordinate of the retained send-preparation record.
    #[must_use]
    pub const fn preparation(&self) -> &ControlRecordCoordinate {
        &self.release.preparation
    }

    /// Return the lower time bound for a later carrier release.
    #[must_use]
    pub fn minimum_release_time(&self) -> UnixTimeV1 {
        *self.release.checkpoint.1.issued_at()
    }
}

impl<'operation, 'call> RetainedA2aSend<'operation, 'call> {
    /// Validate the response identity before carrier start, then start once.
    ///
    /// Keeping response-identity admission in the preflight prevents a local
    /// aliasing error from consuming a request whose remote outcome would then
    /// be unknown.
    ///
    /// # Errors
    ///
    /// Returns the unopened retained phase when the response identity or any
    /// ordinary release check fails. After carrier start, no failure restores
    /// replay permission.
    #[allow(clippy::too_many_arguments)]
    pub fn start_awaiting<Authority, Disclosure>(
        self,
        current: &AmbientSessionAdmissionContext<'_>,
        authority: &mut Authority,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        carrier: &dyn A2aExchangePort,
        response_ref: ResidentRecordRefV1,
        admit_disclosure: Disclosure,
    ) -> Result<AwaitingA2aSendResponse, A2aSendStartFailure<'operation, 'call>>
    where
        Authority: AmbientSessionAuthorityPort,
        Disclosure: FnOnce(
            &ProviderInvocationRequest<'_>,
            &A2aRouteAdmissionV1,
            &str,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let operation = self.release.dispatch.operation();
        if !response_reference_is_valid(
            [
                operation.handle().0,
                operation.handle().1.operation_lease_ref(),
                operation.request_coordinate().record_ref(),
                self.release.dispatch.selection_ref(),
                self.release.dispatch.preparation_observation_ref(),
                self.release.dispatch.observation().0,
                self.release.route.0,
                self.release.request_evidence.coordinate().record_ref(),
                self.release.preparation.record_ref(),
                &self.release.checkpoint.0,
            ],
            &self.release.request_evidence,
            self.release.maximum_evidence_bytes,
            &response_ref,
        ) {
            return Err(A2aSendStartFailure::Preflight {
                retained: Box::new(self),
                reason: A2aSendReleaseError::Association,
            });
        }
        let started = self.start(
            current,
            authority,
            custody,
            anchor,
            carrier,
            admit_disclosure,
        )?;
        Ok(AwaitingA2aSendResponse {
            exchange: started.exchange,
            custody: started.custody,
            context: started.context,
            response_ref,
        })
    }

    /// Recheck current authority and exact new custody, then start once.
    ///
    /// A preflight refusal returns the unopened value. Once `start` is called,
    /// no error path returns a replayable request.
    ///
    /// # Errors
    ///
    /// Returns the unopened retained phase when preflight admission fails. Once
    /// carrier start begins, returns only a non-replayable carrier failure.
    pub fn start<Authority, Disclosure>(
        self,
        current: &AmbientSessionAdmissionContext<'_>,
        authority: &mut Authority,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        carrier: &dyn A2aExchangePort,
        admit_disclosure: Disclosure,
    ) -> Result<InFlightA2aSend, A2aSendStartFailure<'operation, 'call>>
    where
        Authority: AmbientSessionAuthorityPort,
        Disclosure: FnOnce(
            &ProviderInvocationRequest<'_>,
            &A2aRouteAdmissionV1,
            &str,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let preflight = (|| {
            self.release
                .dispatch
                .validate_current_at(current, authority)?;
            let request = self.release.dispatch.provider_request()?;
            let (route_ref, route) = self.release.route;
            validate_route(
                route,
                route_ref,
                self.release.dispatch.operation().handle().1,
                self.release.dispatch.observation().1.observed_at(),
                &request,
                current.now,
            )
            .map_err(|_| A2aSendReleaseError::Association)?;
            let text = match self.release.exchange.request().input() {
                A2aExchangeInput::Text {
                    mode: A2aTaskRequestModeV1::SendMessage,
                    text,
                    ..
                } => text,
                _ => return Err(A2aSendReleaseError::Association),
            };
            validate_exchange_retention(
                current.now,
                self.release.dispatch.operation().handle().1.retain_until(),
                self.release.exchange.coordinate().limits().total_timeout,
            )?;
            admit_disclosure(&request, route, text).map_err(OperationAttemptError::from)?;
            verify_release_custody(&self, custody, anchor)
        })();
        if let Err(reason) = preflight {
            return Err(A2aSendStartFailure::Preflight {
                retained: Box::new(self),
                reason,
            });
        }
        let RetainedA2aSend { custody, release } = self;
        let A2aSendRelease {
            dispatch,
            route,
            exchange: prepared,
            request_evidence,
            preparation,
            checkpoint,
            maximum_evidence_bytes,
            maximum_record_bytes: _,
        } = release;
        let expected = prepared.coordinate().clone();
        let dispatch = PostAdmissionA2aDispatch::capture(&dispatch);
        let route = (route.0.clone(), route.1.clone());
        let context = A2aSendResponseContext {
            dispatch,
            route,
            expected: expected.clone(),
            request_evidence,
            preparation,
            checkpoint,
            maximum_evidence_bytes,
        };
        let mut exchange = match carrier.start(prepared) {
            Ok(exchange) => exchange,
            Err(reason) => {
                return Err(match not_started(custody, context, reason) {
                    Ok(not_started) => A2aSendStartFailure::Rejected(Box::new(not_started)),
                    Err(failure) => A2aSendStartFailure::Diagnostic(failure),
                });
            }
        };
        if exchange.coordinate() != &expected {
            exchange.cancel();
            return Err(
                match unknown_response(
                    custody,
                    context,
                    None,
                    "the A2A carrier accepted the request but returned an unlike exchange coordinate",
                ) {
                    A2aSendResponseFailure::OutcomeUnknown(outcome) => {
                        A2aSendStartFailure::OutcomeUnknown(outcome)
                    }
                    A2aSendResponseFailure::Diagnostic(failure) => {
                        A2aSendStartFailure::Diagnostic(failure)
                    }
                },
            );
        }
        Ok(InFlightA2aSend {
            exchange,
            custody,
            context,
        })
    }
}

impl InFlightA2aSend {
    /// Borrow the exact started exchange identity.
    #[must_use]
    pub fn coordinate(&self) -> &pale_ports::A2aExchangeCoordinate {
        self.exchange.coordinate()
    }
}

impl InFlightA2aSend {
    /// Stop local reads and retain a non-replayable uncertain outcome.
    ///
    /// Local cancellation does not prove that the remote system did not accept
    /// the request. The returned phase must therefore enter reconciliation; it
    /// never restores permission to send the same request again.
    #[must_use = "retain the uncertain outcome and record it before reconciliation"]
    pub fn cancel(mut self) -> A2aSendResponseFailure {
        self.exchange.cancel();
        unknown_response_with_fact(
            self.custody,
            self.context,
            None,
            "local response observation stopped after the A2A request started",
            Some(("uncertainty", "local_read_cancelled")),
        )
    }

    /// Fix the immutable response-evidence identity before reading carrier output.
    ///
    /// # Errors
    ///
    /// Returns an association error when the response identity aliases any record
    /// already used by this send or when no evidence bytes can be retained.
    pub fn await_response(
        self,
        response_ref: ResidentRecordRefV1,
    ) -> Result<AwaitingA2aSendResponse, A2aSendReleaseError> {
        let context = &self.context;
        let operation = context.dispatch.operation();
        if !response_reference_is_valid(
            [
                operation.handle().0,
                operation.handle().1.operation_lease_ref(),
                operation.request_coordinate().record_ref(),
                context.dispatch.selection_ref(),
                context.dispatch.preparation_observation_ref(),
                context.dispatch.observation().0,
                &context.route.0,
                context.request_evidence.coordinate().record_ref(),
                context.preparation.record_ref(),
                &context.checkpoint.0,
            ],
            &context.request_evidence,
            context.maximum_evidence_bytes,
            &response_ref,
        ) {
            return Err(A2aSendReleaseError::Association);
        }
        Ok(AwaitingA2aSendResponse {
            exchange: self.exchange,
            custody: self.custody,
            context: self.context,
            response_ref,
        })
    }
}

impl AwaitingA2aSendResponse {
    pub(in crate::resident) fn task_key(
        &self,
    ) -> Result<crate::resident::task_owner::ResidentOperationTaskKey, OperationAttemptError> {
        self.context
            .dispatch
            .task_key(self.custody.candidate().store())
    }
    /// Borrow the exact started exchange identity.
    #[must_use]
    pub fn coordinate(&self) -> &pale_ports::A2aExchangeCoordinate {
        self.exchange.coordinate()
    }

    /// Borrow the send-preparation custody without granting request replay.
    #[must_use]
    pub const fn send_custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }

    /// Borrow the checkpoint that retained the exact request bytes.
    #[must_use]
    pub fn send_checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.context.checkpoint.0, &self.context.checkpoint.1)
    }

    /// Poll the first carrier item without retrying or reconnecting.
    pub fn poll_response(
        mut self,
        cx: &mut Context<'_>,
        protocol: &dyn A2aSendMessageEvidencePort,
    ) -> A2aSendResponsePoll<Self, ReceivedA2aSendResponse, A2aSendResponseFailure> {
        if !exchange_matches_context(self.exchange.as_ref(), &self.context) {
            self.exchange.cancel();
            return A2aSendResponsePoll::Ready(Err(unknown_response(
                self.custody,
                self.context,
                None,
                "the started A2A exchange changed identity before its response",
            )));
        }
        let evidence = match self.exchange.poll_next(cx) {
            Poll::Pending => return A2aSendResponsePoll::Pending(self),
            Poll::Ready(None) => {
                return A2aSendResponsePoll::Ready(Err(unknown_response(
                    self.custody,
                    self.context,
                    None,
                    "the A2A exchange ended before Pale received a Task response",
                )));
            }
            Poll::Ready(Some(Err(_))) => {
                return A2aSendResponsePoll::Ready(Err(unknown_response(
                    self.custody,
                    self.context,
                    None,
                    "the A2A exchange failed before Pale identified a remote Task",
                )));
            }
            Poll::Ready(Some(Ok(evidence))) => evidence,
        };
        if evidence.bytes.len() > self.context.expected.limits().response_bytes.get() {
            self.exchange.cancel();
            return A2aSendResponsePoll::Ready(Err(unknown_response(
                self.custody,
                self.context,
                None,
                "the A2A response exceeded the selected response-byte limit",
            )));
        }
        let remaining = self
            .context
            .maximum_evidence_bytes
            .saturating_sub(self.context.request_evidence.byte_len());
        let carrier_facts = evidence.facts.clone();
        let has_sse = evidence.sse.is_some();
        let response = match OperationA2aEvidence::new(
            ImmutableRecordReferenceV1 {
                record_ref: self.response_ref,
                record_sha256: ArtifactDigest::hash_bytes(&evidence.bytes),
            },
            evidence.bytes,
            evidence.sse.clone(),
            remaining,
        ) {
            Ok(value) => value,
            Err(_) => {
                self.exchange.cancel();
                return A2aSendResponsePoll::Ready(Err(unknown_response(
                    self.custody,
                    self.context,
                    None,
                    "the A2A response could not enter the selected evidence bound",
                )));
            }
        };
        let maximum_bytes = match std::num::NonZeroUsize::new(self.context.maximum_evidence_bytes) {
            Some(value) => value,
            None => {
                self.exchange.cancel();
                return A2aSendResponsePoll::Ready(Err(unknown_response(
                    self.custody,
                    self.context,
                    Some(response),
                    "the A2A response could not be qualified within the selected evidence bound",
                )));
            }
        };
        let admitted_facts = protocol.read_initial_response(
            self.context.request_evidence.bytes(),
            response.bytes(),
            self.context.expected.context().request_id(),
            maximum_bytes,
        );
        let admitted_facts = match admitted_facts {
            Ok(facts) if facts == carrier_facts => facts,
            Ok(_) | Err(_) => {
                self.exchange.cancel();
                return A2aSendResponsePoll::Ready(Err(unknown_response(
                    self.custody,
                    self.context,
                    Some(response),
                    "the A2A carrier facts did not match qualified response bytes",
                )));
            }
        };
        let facts = response_facts(admitted_facts, has_sse);
        A2aSendResponsePoll::Ready(Ok(ReceivedA2aSendResponse {
            exchange: self.exchange,
            custody: self.custody,
            context: self.context,
            response,
            facts,
        }))
    }

    /// Stop local reads and retain a non-replayable uncertain outcome.
    #[must_use = "retain the uncertain outcome and record it before reconciliation"]
    pub fn cancel(mut self) -> A2aSendResponseFailure {
        self.exchange.cancel();
        unknown_response_with_fact(
            self.custody,
            self.context,
            None,
            "local response observation stopped after the A2A request started",
            Some(("uncertainty", "local_read_cancelled")),
        )
    }
}

impl ReceivedA2aSendResponse {
    pub(in crate::resident) fn task_key(
        &self,
    ) -> Result<crate::resident::task_owner::ResidentOperationTaskKey, OperationAttemptError> {
        self.context
            .dispatch
            .task_key(self.custody.candidate().store())
    }
    /// Borrow the exact started exchange identity.
    #[must_use]
    pub fn coordinate(&self) -> &pale_ports::A2aExchangeCoordinate {
        self.exchange.coordinate()
    }

    /// Borrow the send-preparation custody without granting request replay.
    #[must_use]
    pub const fn send_custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }

    /// Borrow the checkpoint that retained the exact request bytes.
    #[must_use]
    pub fn send_checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.context.checkpoint.0, &self.context.checkpoint.1)
    }

    /// Require exact unary EOF before classifying the retained response.
    pub fn poll_eof(
        mut self,
        cx: &mut Context<'_>,
        clock: &dyn WallClockSourcePort,
    ) -> A2aSendResponsePoll<Self, CompletedA2aSendResponse, A2aSendResponseFailure> {
        match self.exchange.poll_next(cx) {
            Poll::Pending => A2aSendResponsePoll::Pending(self),
            Poll::Ready(None) => {
                let observed_at = match sample_response_time(&self.context, clock) {
                    Ok(observed_at) => observed_at,
                    Err(_) => {
                        return A2aSendResponsePoll::Ready(Err(unknown_response(
                            self.custody,
                            self.context,
                            Some(self.response),
                            "the A2A response time could not be admitted at terminal EOF",
                        )));
                    }
                };
                match self.facts {
                    A2aInitialResponseFacts::Task {
                        task,
                        context,
                        state,
                    } => A2aSendResponsePoll::Ready(Ok(CompletedA2aSendResponse::Task(Box::new(
                        CompletedA2aTaskResponse {
                            custody: self.custody,
                            context: self.context,
                            response: self.response,
                            task,
                            remote_context: context,
                            state,
                            observed_at,
                        },
                    )))),
                    facts => match no_task_diagnostic(&self.context, &facts) {
                        Ok(diagnostic) => {
                            A2aSendResponsePoll::Ready(Ok(CompletedA2aSendResponse::NoTask(
                                Box::new(CompletedA2aNoTaskResponse {
                                    custody: self.custody,
                                    context: self.context,
                                    response: self.response,
                                    facts,
                                    diagnostic,
                                    observed_at,
                                }),
                            )))
                        }
                        Err(error) => {
                            A2aSendResponsePoll::Ready(Err(A2aSendResponseFailure::Diagnostic(
                                Box::new(A2aSendDiagnosticFailure {
                                    custody: self.custody,
                                    context: self.context,
                                    response: Some(self.response),
                                    kind: A2aSendDiagnosticFailureKind::NoTask {
                                        facts,
                                        observed_at,
                                    },
                                    reason: error,
                                }),
                            )))
                        }
                    },
                }
            }
            Poll::Ready(Some(Ok(_))) => {
                self.exchange.cancel();
                A2aSendResponsePoll::Ready(Err(unknown_response_with_fact(
                    self.custody,
                    self.context,
                    Some(self.response),
                    "the unary A2A send produced an additional carrier item",
                    Some(("terminal_output", "additional_item")),
                )))
            }
            Poll::Ready(Some(Err(reason))) => {
                self.exchange.cancel();
                A2aSendResponsePoll::Ready(Err(unknown_response_with_fact(
                    self.custody,
                    self.context,
                    Some(self.response),
                    "the unary A2A send failed while Pale awaited terminal EOF",
                    Some(("carrier_failure", exchange_error_kind(reason))),
                )))
            }
        }
    }

    /// Stop local EOF reads and retain the response in an uncertain outcome.
    #[must_use = "retain the uncertain outcome and record it before reconciliation"]
    pub fn cancel(mut self) -> A2aSendResponseFailure {
        self.exchange.cancel();
        unknown_response_with_fact(
            self.custody,
            self.context,
            Some(self.response),
            "local EOF observation stopped after Pale retained the first A2A response",
            Some(("uncertainty", "local_read_cancelled")),
        )
    }
}

impl CompletedA2aTaskResponse {
    /// Borrow the exact request and response bytes admitted for this Task.
    #[must_use]
    pub fn evidence(&self) -> (&OperationA2aEvidence, &OperationA2aEvidence) {
        (&self.context.request_evidence, &self.response)
    }

    /// Borrow the remote Task identity and reported owner-native state.
    #[must_use]
    pub const fn task(
        &self,
    ) -> (
        &A2aRemoteTaskIdV1,
        &A2aRemoteContextIdV1,
        &A2aRemoteTaskStateV1,
    ) {
        (&self.task, &self.remote_context, &self.state)
    }

    /// Return when Pale observed exact unary completion for this response.
    #[must_use]
    pub const fn observed_at(&self) -> UnixTimeV1 {
        self.observed_at
    }

    pub(in crate::resident) fn retain_until(&self) -> UnixTimeV1 {
        *self.context.dispatch.operation().handle().1.retain_until()
    }

    /// Borrow the send-preparation custody. It is not Task-binding custody.
    #[must_use]
    pub const fn send_custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }

    /// Borrow the exact dispatched operation observation that opened this send.
    #[must_use]
    pub fn operation_observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.context.dispatch.observation()
    }

    /// Borrow the send-preparation checkpoint that retained the request bytes.
    #[must_use]
    pub fn send_checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.context.checkpoint.0, &self.context.checkpoint.1)
    }

    pub(in crate::resident) fn checkpoint_selection(&self) -> OperationCheckpointSelection {
        self.context.dispatch.operation().checkpoint_selection()
    }

    pub(in crate::resident) fn process_task_key(
        &self,
    ) -> Result<crate::resident::task_owner::ResidentOperationTaskKey, OperationAttemptError> {
        self.context
            .dispatch
            .task_key(self.custody.candidate().store())
    }
}

impl CompletedA2aTaskResponse {
    /// Continue this exact Task into the existing binding and remote-history owner.
    ///
    /// The retained `SendMessage` request is reused as predecessor evidence; only
    /// the exact Task response is inserted. This method does not send another
    /// request and never promotes remote state to a Pale result or settlement.
    ///
    /// # Errors
    ///
    /// Rejects substituted Task, request, route, operation, evidence, signing,
    /// or physical-history inputs before any custody mutation.
    pub fn prepare_task_custody<'a, Evidence, BindingSign, ObservationSign, Store>(
        &'a self,
        input: A2aInitialTaskCustodyInput<'a>,
        admit_evidence: Evidence,
        sign_binding: BindingSign,
        sign_observation: ObservationSign,
        resolve_store: Store,
    ) -> Result<crate::resident::PreparedA2aTaskAppend<'a>, crate::resident::A2aTaskCustodyError>
    where
        Evidence: FnOnce(
            &A2aTaskBindingPreimageV1,
            &A2aTaskObservationPreimageV1,
            &[OperationA2aEvidence],
            &[OperationA2aEvidence],
        ) -> Result<(), InvalidResidentRecordV1>,
        BindingSign: FnOnce(
            &[u8],
            &A2aTaskBindingPreimageV1,
            &A2aTaskBindingDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &A2aTaskObservationPreimageV1,
            &A2aTaskObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Store: FnOnce(
            &A2aTaskObservationV1,
        )
            -> Result<pale_ports::AuthorityStoreCoordinate, InvalidResidentRecordV1>,
    {
        let context = &self.context;
        let response = &self.response;
        let task = &self.task;
        let remote_context = &self.remote_context;
        let state = &self.state;
        let handle = context.dispatch.operation().handle();
        let retained_dispatch = context.dispatch.observation();
        let dispatch = input.operation_custody.selected();
        let route = (&context.route.0, &context.route.1);
        let request = immutable_evidence_reference(&context.request_evidence)
            .ok_or(crate::resident::A2aTaskCustodyError::Evidence)?;
        let response_reference = immutable_evidence_reference(response)
            .ok_or(crate::resident::A2aTaskCustodyError::Evidence)?;
        let attempt = dispatch
            .1
            .state()
            .dispatched_attempt_id()
            .ok_or(crate::resident::A2aTaskCustodyError::Evidence)?;
        if retained_dispatch != dispatch
            || input.maximum_evidence_bytes < context.maximum_evidence_bytes
            || input.binding.operation_handle_id != *handle.1.operation_handle_id()
            || input.binding.operation_handle_ref != *handle.0
            || input.binding.operation_handle_sha256
                != *handle.1.canonical_operation_handle_sha256()
            || input.binding.operation_observation_ref != *dispatch.0
            || input.binding.operation_observation_sha256
                != *dispatch.1.canonical_operation_observation_sha256()
            || input.binding.route_admission_id != *route.1.route_admission_id()
            || input.binding.route_admission_ref != *route.0
            || input.binding.route_admission_sha256
                != *route.1.canonical_a2a_route_admission_sha256()
            || input.binding.attempt_id != *attempt
            || input.binding.json_rpc_request_id != *context.expected.context().request_id()
            || input.binding.request_mode != A2aTaskRequestModeV1::SendMessage
            || input.binding.bound_at != self.observed_at
            || &input.binding.remote_task_id != task
            || &input.binding.remote_context_id != remote_context
            || input.binding.request_evidence != request
            || input.binding.initial_task_evidence != response_reference
            || input.observation.task_binding_id != input.binding.task_binding_id
            || input.observation.task_binding_ref != input.binding_ref
            || input.observation.revision.get() != 1
            || input.observation.predecessor_ref.is_some()
            || input.observation.predecessor_sha256.is_some()
            || &input.observation.remote_state != state
            || input.observation.observed_at != self.observed_at
            || !matches!(
                &input.observation.source,
                A2aTaskObservationSourceV1::InitialSendMessageResponse { evidence }
                    if evidence == &response_reference
            )
        {
            return Err(crate::resident::A2aTaskCustodyError::Evidence);
        }
        let route_coordinate = record_projection::coordinate(
            route.0,
            route.1.canonical_a2a_route_admission_sha256().as_str(),
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let route_record = record_projection::record(route_coordinate, route.1)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        crate::resident::prepare_a2a_task_binding_custody(
            crate::resident::A2aTaskBindingCustodyInput {
                operation_custody: input.operation_custody,
                binding_ref: input.binding_ref,
                binding: input.binding,
                binding_context: A2aTaskBindingAdmissionContextV1 {
                    signed: input.binding_signing,
                    operation_handle_ref: handle.0,
                    operation_handle: handle.1,
                    operation_observation_ref: dispatch.0,
                    operation_observation: dispatch.1,
                    route_admission_ref: route.0,
                    route_admission: route.1,
                },
                observation_ref: input.observation_ref,
                observation: input.observation,
                observation_signing: input.observation_signing,
                route_predecessor: OperationCustodyIndexPredecessor::Exact(route_record),
                route_association: crate::resident::A2aRouteAssociation::Retained,
                new_evidence: vec![response.clone()],
                retained_evidence: vec![context.request_evidence.clone()],
                maximum_evidence_bytes: input.maximum_evidence_bytes,
                maximum_record_bytes: input.maximum_record_bytes,
            },
            admit_evidence,
            sign_binding,
            sign_observation,
            resolve_store,
        )
    }
}

impl A2aSendNotStarted {
    /// Borrow the carrier rejection that proves queue acceptance did not occur.
    #[must_use]
    pub const fn reason(&self) -> A2aExchangeError {
        self.reason
    }

    /// Borrow the admitted diagnostic and exact retained request bytes.
    #[must_use]
    pub fn evidence(
        &self,
    ) -> (
        &crate::diagnostic::AdmittedDiagnostic,
        &OperationA2aEvidence,
    ) {
        (&self.diagnostic, &self.context.request_evidence)
    }

    /// Borrow the send-preparation custody. It never proves a remote effect.
    #[must_use]
    pub const fn send_custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }

    pub(in crate::resident) fn process_task_key(
        &self,
    ) -> Result<crate::resident::task_owner::ResidentOperationTaskKey, OperationAttemptError> {
        self.context
            .dispatch
            .task_key(self.custody.candidate().store())
    }
}

impl A2aSendNotStarted {
    /// Sign a failed observation for a request rejected before queue acceptance.
    ///
    /// The exact request remains evidence only. This transition does not infer a
    /// remote Task, effect, result, receipt, or permission to retry.
    ///
    /// # Errors
    /// Rejects reused references, backward time, elapsed retention, oversized
    /// diagnostics, or observation admission and signing failures.
    pub fn prepare_observation<Rotation, Sign>(
        self,
        input: A2aFailedSendObservationInput<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedA2aSendObservation, A2aNotStartedObservationFailure>
    where
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let observed_at = sample_response_time(&self.context, input.clock);
        let issued = observed_at.and_then(|observed_at| {
            validate_response_observation_input(
                &self.context,
                None,
                None,
                &input.diagnostic_ref,
                &input.observation_ref,
                observed_at,
            )
            .and_then(|()| {
                issue_response_observation(
                    &self.context,
                    &self.diagnostic,
                    &input.diagnostic_ref,
                    A2aResponseObservationKind::Failed,
                    observed_at,
                    input.signing,
                    input.maximum_diagnostic_bytes,
                    admit_rotation,
                    sign,
                )
            })
        });
        match issued {
            Ok((observation, diagnostic_record)) => Ok(PreparedA2aSendObservation {
                custody: self.custody,
                context: self.context,
                observation_ref: input.observation_ref,
                observation,
                diagnostic: self.diagnostic,
                diagnostic_record,
                response: None,
            }),
            Err(reason) => Err(A2aNotStartedObservationFailure {
                not_started: Box::new(self),
                reason,
            }),
        }
    }
}

impl CompletedA2aNoTaskResponse {
    /// Borrow the typed reason why no Task binding can be created.
    #[must_use]
    pub const fn facts(&self) -> &A2aInitialResponseFacts {
        &self.facts
    }

    /// Return when Pale observed exact unary completion for this response.
    #[must_use]
    pub const fn observed_at(&self) -> UnixTimeV1 {
        self.observed_at
    }

    /// Borrow the admitted diagnostic and exact response bytes.
    #[must_use]
    pub fn evidence(
        &self,
    ) -> (
        &crate::diagnostic::AdmittedDiagnostic,
        &OperationA2aEvidence,
        &OperationA2aEvidence,
    ) {
        (
            &self.diagnostic,
            &self.context.request_evidence,
            &self.response,
        )
    }

    /// Borrow the send-preparation custody. No Task binding exists.
    #[must_use]
    pub const fn send_custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }

    pub(in crate::resident) fn process_task_key(
        &self,
    ) -> Result<crate::resident::task_owner::ResidentOperationTaskKey, OperationAttemptError> {
        self.context
            .dispatch
            .task_key(self.custody.candidate().store())
    }
}

impl CompletedA2aNoTaskResponse {
    /// Sign a failed Pale observation for this confirmed task-less response.
    ///
    /// The exact response bytes and diagnostic remain separate records in one
    /// checkpoint transaction. No Task binding, result, receipt, retry, or
    /// remote-state promotion is created.
    ///
    /// # Errors
    /// Rejects reused references, backward time, elapsed retention, oversized
    /// diagnostics, or observation admission and signing failures.
    pub fn prepare_observation<Rotation, Sign>(
        self,
        input: A2aFailedSendObservationInput<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedA2aSendObservation, A2aNoTaskObservationFailure>
    where
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let observed_at = sample_response_time(&self.context, input.clock);
        let issued = observed_at.and_then(|observed_at| {
            if observed_at < self.observed_at {
                return Err(OperationAttemptError::TimeOrder);
            }
            validate_response_observation_input(
                &self.context,
                Some(&self.response),
                None,
                &input.diagnostic_ref,
                &input.observation_ref,
                observed_at,
            )
            .and_then(|()| {
                issue_response_observation(
                    &self.context,
                    &self.diagnostic,
                    &input.diagnostic_ref,
                    A2aResponseObservationKind::Failed,
                    observed_at,
                    input.signing,
                    input.maximum_diagnostic_bytes,
                    admit_rotation,
                    sign,
                )
            })
        });
        match issued {
            Ok((observation, diagnostic_record)) => Ok(PreparedA2aSendObservation {
                custody: self.custody,
                context: self.context,
                observation_ref: input.observation_ref,
                observation,
                diagnostic: self.diagnostic,
                diagnostic_record,
                response: Some(self.response),
            }),
            Err(reason) => Err(A2aNoTaskObservationFailure {
                response: Box::new(self),
                reason,
            }),
        }
    }
}

impl A2aSendOutcomeUnknown {
    /// Borrow the uncertainty diagnostic. It always requires reconciliation first.
    #[must_use]
    pub const fn diagnostic(&self) -> &crate::diagnostic::AdmittedDiagnostic {
        &self.diagnostic
    }

    /// Borrow any exact response item retained before the outcome became uncertain.
    #[must_use]
    pub const fn response_evidence(&self) -> Option<&OperationA2aEvidence> {
        self.response.as_ref()
    }

    /// Borrow the exact request bytes. They are evidence, not replay permission.
    #[must_use]
    pub const fn request_evidence(&self) -> &OperationA2aEvidence {
        &self.context.request_evidence
    }

    /// Borrow the send-preparation custody that preceded the uncertain carrier call.
    #[must_use]
    pub const fn send_custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }

    pub(in crate::resident) fn process_task_key(
        &self,
    ) -> Result<crate::resident::task_owner::ResidentOperationTaskKey, OperationAttemptError> {
        self.context
            .dispatch
            .task_key(self.custody.candidate().store())
    }
}

impl A2aSendOutcomeUnknown {
    /// Sign an outcome-unknown observation without replaying the original send.
    ///
    /// Any response item retained before uncertainty is committed with the
    /// diagnostic. The caller must supply an existing reconciliation frontier;
    /// this method does not fabricate one or infer remote Task identity.
    ///
    /// # Errors
    /// Rejects reused references, backward time, elapsed retention, oversized
    /// diagnostics, or observation admission and signing failures.
    pub fn prepare_observation<Reconciliation, Rotation, Sign>(
        self,
        input: A2aOutcomeUnknownObservationInput<'_>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedA2aSendObservation, A2aOutcomeUnknownObservationFailure>
    where
        Reconciliation:
            FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let observed_at = sample_response_time(&self.context, input.clock);
        let issued = observed_at.and_then(|observed_at| {
            validate_response_observation_input(
                &self.context,
                self.response.as_ref(),
                Some(&input.reconciliation),
                &input.diagnostic_ref,
                &input.observation_ref,
                observed_at,
            )
            .and_then(|()| {
                admit_reconciliation(&input.reconciliation).map_err(OperationAttemptError::from)
            })
            .and_then(|()| {
                issue_response_observation(
                    &self.context,
                    &self.diagnostic,
                    &input.diagnostic_ref,
                    A2aResponseObservationKind::OutcomeUnknown(input.reconciliation.clone()),
                    observed_at,
                    input.signing,
                    input.maximum_diagnostic_bytes,
                    admit_rotation,
                    sign,
                )
            })
        });
        match issued {
            Ok((observation, diagnostic_record)) => Ok(PreparedA2aSendObservation {
                custody: self.custody,
                context: self.context,
                observation_ref: input.observation_ref,
                observation,
                diagnostic: self.diagnostic,
                diagnostic_record,
                response: self.response,
            }),
            Err(reason) => Err(A2aOutcomeUnknownObservationFailure {
                outcome: Box::new(self),
                reason,
            }),
        }
    }
}

impl PreparedA2aSendObservation {
    /// Borrow the admitted diagnostic; physical retention is not yet proved.
    #[must_use]
    pub const fn diagnostic(&self) -> &crate::diagnostic::AdmittedDiagnostic {
        &self.diagnostic
    }

    /// Borrow the signed operation observation awaiting checkpoint custody.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Borrow the anchored send-preparation custody that preceded this response.
    #[must_use]
    pub const fn send_custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }

    /// Borrow the response item retained with this observation, when one exists.
    #[must_use]
    pub const fn response_evidence(&self) -> Option<&OperationA2aEvidence> {
        self.response.as_ref()
    }

    pub(super) fn checkpoint_selection(&self) -> OperationCheckpointSelection {
        self.context.dispatch.operation().checkpoint_selection()
    }

    pub(super) fn retain_until(&self) -> UnixTimeV1 {
        *self.context.dispatch.operation().handle().1.retain_until()
    }

    /// Borrow the exact send-preparation checkpoint this response succeeds.
    #[must_use]
    pub fn predecessor_checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.context.checkpoint.0, &self.context.checkpoint.1)
    }

    /// Prepare the diagnostic, observation, and exact response bytes for one checkpoint.
    ///
    /// The predecessor is fixed by the send-preparation custody that released
    /// this carrier call. A caller cannot substitute another current checkpoint
    /// or omit the response-evidence leaf.
    ///
    /// # Errors
    /// Preserves lineage, authority-owned storage, and physical projection failures.
    pub fn prepare_checkpoint<'a>(
        &'a self,
        selection: OperationCheckpointSelection,
        limits: AccumulatorPreparationLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        crate::resident::PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        let operation = self.context.dispatch.operation();
        let (handle_ref, handle) = operation.handle();
        let predecessor = self.context.dispatch.observation();
        let append = prepare_operation_observation_append(
            OperationObservationTransition::new(
                handle_ref,
                handle,
                predecessor.0,
                predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?
        .with_associated_record(OperationCustodyRecord::Diagnostic(
            self.diagnostic_record.clone(),
        ));
        let append = match &self.response {
            Some(response) => append.with_a2a_evidence_items(vec![response.clone()]),
            None => append,
        };
        let (predecessor_ref, predecessor) = self.predecessor_checkpoint();
        append.prepare_checkpoint(selection, predecessor_ref, predecessor, limits, custody)
    }
}

fn a2a_send_checkpoint_preimage(
    prepared: &PreparedA2aSendCheckpoint<'_, '_>,
    prior: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    identity: &OperationCheckpointInput,
    issued_at: UnixTimeV1,
    schema: &SchemaDigest,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
) -> Result<ControlStateCheckpointPreimageV1, A2aSendCheckpointError> {
    let sequence = prior
        .1
        .domain_sequence()
        .get()
        .checked_add(1)
        .ok_or(A2aSendCheckpointError::Association)?;
    Ok(ControlStateCheckpointPreimageV1 {
        schema_id: prior.1.schema_id().clone(),
        schema_sha256: schema.clone(),
        control_checkpoint_id: identity.id.clone(),
        control_store_id: authority.control_store_id.clone(),
        control_incarnation_id: authority.control_incarnation_id.clone(),
        checkpoint_domain_id: authority.checkpoint_domain_id.clone(),
        domain_epoch: *prior.1.domain_epoch(),
        domain_sequence: ControlDomainSequenceV1::new(sequence)?,
        previous_control_checkpoint_ref: Some(prior.0.clone()),
        previous_control_checkpoint_sha256: Some(
            prior.1.canonical_control_checkpoint_sha256().clone(),
        ),
        checkpoint_profile_ref: authority.checkpoint_profile_ref.clone(),
        checkpoint_profile_sha256: authority.checkpoint_profile_sha256.clone(),
        control_schema_version: *prior.1.control_schema_version(),
        committed_transaction_sequence: prepared.transaction_sequence(),
        state_root_sha256: prepared.state_root().clone(),
        leaf_count: prepared.leaf_count(),
        changed_leaf_keys: prepared.changed_keys().to_vec(),
        unresolved_custody_root_sha256: prepared.unresolved_root().clone(),
        issued_at,
        checkpoint_signing_authority_ref: authority.authority_ref.clone(),
        checkpoint_signing_authority_sha256: authority.authority_sha256.clone(),
        issuer_principal_id: authority.principal_id.clone(),
        issuer_key_id: authority.subject_key_id.clone(),
        issuer_key_epoch: authority.subject_key_epoch,
    })
}

#[allow(clippy::too_many_arguments)]
fn issue_response_observation<Rotation, Sign>(
    context: &A2aSendResponseContext,
    diagnostic: &crate::diagnostic::AdmittedDiagnostic,
    diagnostic_ref: &ResidentRecordRefV1,
    kind: A2aResponseObservationKind,
    observed_at: UnixTimeV1,
    signing_context: SignedResidentAdmissionContextV1<'_>,
    maximum_diagnostic_bytes: usize,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<(OperationObservationV1, ControlRecord), OperationAttemptError>
where
    Rotation: FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    Sign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    if diagnostic.canonical_bytes().len() > maximum_diagnostic_bytes {
        return Err(OperationAttemptError::BoundExceeded);
    }
    let (diagnostic_sha256, diagnostic_record) =
        record_projection::diagnostic_record(diagnostic_ref, diagnostic)
            .map_err(OperationObservationCustodyProjectionError::from)?;
    let state = match kind {
        A2aResponseObservationKind::Failed => OperationObservationStateV1::Failed {
            result: None,
            diagnostic: ImmutableRecordReferenceV1 {
                record_ref: diagnostic_ref.clone(),
                record_sha256: diagnostic_sha256,
            },
        },
        A2aResponseObservationKind::OutcomeUnknown(reconciliation) => {
            OperationObservationStateV1::OutcomeUnknown {
                diagnostic: ImmutableRecordReferenceV1 {
                    record_ref: diagnostic_ref.clone(),
                    record_sha256: diagnostic_sha256,
                },
                reconciliation,
                effect_receipt: None,
            }
        }
    };
    let (handle_ref, handle) = context.dispatch.operation().handle();
    let predecessor = context.dispatch.observation();
    let observation = signing::issue_observation(
        &OperationObservationAdmissionContextV1 {
            signed: signing_context,
            handle_ref,
            handle,
            predecessor: Some(predecessor),
        },
        state,
        observed_at,
        admit_rotation,
        sign,
    )?;
    Ok((observation, diagnostic_record))
}

fn validate_response_observation_input(
    context: &A2aSendResponseContext,
    response: Option<&OperationA2aEvidence>,
    reconciliation: Option<&ReconciliationFrontierReferenceV1>,
    diagnostic_ref: &ResidentRecordRefV1,
    observation_ref: &ResidentRecordRefV1,
    observed_at: UnixTimeV1,
) -> Result<(), OperationAttemptError> {
    let operation = context.dispatch.operation();
    let (handle_ref, handle) = operation.handle();
    let dispatch = context.dispatch.observation();
    if observed_at < *dispatch.1.observed_at() || observed_at < *context.checkpoint.1.issued_at() {
        return Err(OperationAttemptError::TimeOrder);
    }
    if observed_at > *handle.retain_until() {
        return Err(OperationAttemptError::RetentionElapsed);
    }
    let aliases = [
        handle_ref,
        handle.operation_lease_ref(),
        operation.request_coordinate().record_ref(),
        context.dispatch.selection_ref(),
        context.dispatch.preparation_observation_ref(),
        dispatch.0,
        &context.route.0,
        context.request_evidence.coordinate().record_ref(),
        context.preparation.record_ref(),
    ];
    if diagnostic_ref == observation_ref
        || aliases
            .iter()
            .any(|reference| *reference == diagnostic_ref || *reference == observation_ref)
        || response.is_some_and(|evidence| {
            evidence.coordinate().record_ref() == diagnostic_ref
                || evidence.coordinate().record_ref() == observation_ref
        })
        || reconciliation.is_some_and(|frontier| {
            &frontier.record_ref == diagnostic_ref
                || &frontier.record_ref == observation_ref
                || aliases.contains(&&frontier.record_ref)
                || response.is_some_and(|evidence| {
                    evidence.coordinate().record_ref() == &frontier.record_ref
                })
        })
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

fn sample_response_time(
    context: &A2aSendResponseContext,
    clock: &dyn WallClockSourcePort,
) -> Result<UnixTimeV1, OperationAttemptError> {
    let dispatch_time = *context.dispatch.observation().1.observed_at();
    let checkpoint_time = *context.checkpoint.1.issued_at();
    let floor = std::cmp::max(dispatch_time, checkpoint_time);
    let observed_at = crate::resident::operation_dispatch::custody::sample_time(clock, floor)?;
    if observed_at > *context.dispatch.operation().handle().1.retain_until() {
        return Err(OperationAttemptError::RetentionElapsed);
    }
    Ok(observed_at)
}

fn validate_exchange_retention(
    now: UnixTimeV1,
    retain_until: &UnixTimeV1,
    total_timeout: Duration,
) -> Result<(), A2aSendReleaseError> {
    let rounded_timeout = total_timeout
        .as_secs()
        .checked_add(u64::from(total_timeout.subsec_nanos() != 0))
        .ok_or(A2aSendReleaseError::InsufficientRetention)?;
    let deadline = now
        .get()
        .checked_add(rounded_timeout)
        .ok_or(A2aSendReleaseError::InsufficientRetention)?;
    if deadline > retain_until.get() {
        return Err(A2aSendReleaseError::InsufficientRetention);
    }
    Ok(())
}

fn response_reference_is_valid<const N: usize>(
    aliases: [&ResidentRecordRefV1; N],
    request_evidence: &OperationA2aEvidence,
    maximum_evidence_bytes: usize,
    response_ref: &ResidentRecordRefV1,
) -> bool {
    maximum_evidence_bytes > request_evidence.byte_len() && !aliases.contains(&response_ref)
}

fn exchange_matches_context(exchange: &dyn A2aExchange, context: &A2aSendResponseContext) -> bool {
    exchange.coordinate().request_sha256().as_str()
        == context
            .request_evidence
            .coordinate()
            .record_sha256()
            .as_str()
        && exchange.coordinate() == &context.expected
}

fn response_facts(facts: A2aExchangeFacts, has_sse: bool) -> A2aInitialResponseFacts {
    if has_sse {
        return A2aInitialResponseFacts::UnexpectedStreaming;
    }
    match facts {
        A2aExchangeFacts::Task {
            task,
            context,
            state,
        } => A2aInitialResponseFacts::Task {
            task,
            context,
            state,
        },
        A2aExchangeFacts::DirectMessage => A2aInitialResponseFacts::DirectMessage,
        A2aExchangeFacts::Status { .. } => A2aInitialResponseFacts::Status,
        A2aExchangeFacts::Artifact { .. } => A2aInitialResponseFacts::Artifact,
        A2aExchangeFacts::RemoteError { .. } => A2aInitialResponseFacts::RemoteError,
    }
}

fn immutable_evidence_reference(
    evidence: &OperationA2aEvidence,
) -> Option<ImmutableRecordReferenceV1> {
    Some(ImmutableRecordReferenceV1 {
        record_ref: evidence.coordinate().record_ref().clone(),
        record_sha256: ArtifactDigest::parse(evidence.coordinate().record_sha256().as_str())
            .ok()?,
    })
}

fn no_task_diagnostic(
    context: &A2aSendResponseContext,
    facts: &A2aInitialResponseFacts,
) -> Result<crate::diagnostic::AdmittedDiagnostic, crate::diagnostic::DiagnosticAdmissionError> {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1")
        .map_err(crate::diagnostic::DiagnosticAdmissionError::Registry)?;
    crate::diagnostic::DiagnosticProjection {
        code: "a2a_initial_response_has_no_task".to_string(),
        stage: DiagnosticStageV1::HostObservation,
        disposition: DiagnosticDispositionV1::OperationalFailure,
        summary: "the A2A send completed without an admissible remote Task".to_string(),
        subject: Some((
            "operation".to_string(),
            context
                .dispatch
                .operation()
                .handle()
                .1
                .operation_handle_id()
                .as_str()
                .to_string(),
        )),
        source: None,
        schema_reference: None,
        causes: Vec::new(),
        notes: vec!["Pale did not create an A2A task binding.".to_string()],
        help: Vec::new(),
        facts: vec![(
            "response_kind".to_string(),
            response_kind(facts).to_string(),
        )],
        retry: DiagnosticRetryV1::NotIndicated,
        effect: DiagnosticEffectCertaintyV1::Confirmed,
        request: Some((
            "a2a_json_rpc".to_string(),
            context.expected.context().request_id().as_str().to_string(),
        )),
        generation: None,
        receipt: None,
        omitted_causes: 0,
    }
    .admit(&identity)
}

fn not_started(
    custody: CompletedOperationObservationCustody,
    context: A2aSendResponseContext,
    reason: A2aExchangeError,
) -> Result<A2aSendNotStarted, Box<A2aSendDiagnosticFailure>> {
    let identity = match pale_contracts::record_schema_identity("diagnostic.v1") {
        Ok(identity) => identity,
        Err(error) => {
            return Err(Box::new(A2aSendDiagnosticFailure {
                custody,
                context,
                response: None,
                kind: A2aSendDiagnosticFailureKind::NotStarted(reason),
                reason: crate::diagnostic::DiagnosticAdmissionError::Registry(error),
            }));
        }
    };
    let diagnostic = crate::diagnostic::DiagnosticProjection {
        code: "a2a_send_not_started".to_string(),
        stage: DiagnosticStageV1::HostObservation,
        disposition: DiagnosticDispositionV1::OperationalFailure,
        summary: "the A2A carrier rejected the request before queue acceptance".to_string(),
        subject: Some((
            "operation".to_string(),
            context
                .dispatch
                .operation()
                .handle()
                .1
                .operation_handle_id()
                .as_str()
                .to_string(),
        )),
        source: None,
        schema_reference: None,
        causes: Vec::new(),
        notes: vec!["No remote Task or effect is established by this attempt.".to_string()],
        help: Vec::new(),
        facts: vec![(
            "carrier_rejection".to_string(),
            exchange_error_kind(reason).to_string(),
        )],
        retry: DiagnosticRetryV1::NotIndicated,
        effect: DiagnosticEffectCertaintyV1::NotStarted,
        request: Some((
            "a2a_json_rpc".to_string(),
            context.expected.context().request_id().as_str().to_string(),
        )),
        generation: None,
        receipt: None,
        omitted_causes: 0,
    }
    .admit(&identity);
    match diagnostic {
        Ok(diagnostic) => Ok(A2aSendNotStarted {
            custody,
            context,
            reason,
            diagnostic,
        }),
        Err(error) => Err(Box::new(A2aSendDiagnosticFailure {
            custody,
            context,
            response: None,
            kind: A2aSendDiagnosticFailureKind::NotStarted(reason),
            reason: error,
        })),
    }
}

fn unknown_response(
    custody: CompletedOperationObservationCustody,
    context: A2aSendResponseContext,
    response: Option<OperationA2aEvidence>,
    summary: &'static str,
) -> A2aSendResponseFailure {
    unknown_response_with_fact(custody, context, response, summary, None)
}

fn unknown_response_with_fact(
    custody: CompletedOperationObservationCustody,
    context: A2aSendResponseContext,
    response: Option<OperationA2aEvidence>,
    summary: &'static str,
    fact: Option<(&'static str, &'static str)>,
) -> A2aSendResponseFailure {
    let identity = match pale_contracts::record_schema_identity("diagnostic.v1") {
        Ok(value) => value,
        Err(error) => {
            return A2aSendResponseFailure::Diagnostic(Box::new(A2aSendDiagnosticFailure {
                custody,
                context,
                response,
                kind: A2aSendDiagnosticFailureKind::OutcomeUnknown(summary),
                reason: crate::diagnostic::DiagnosticAdmissionError::Registry(error),
            }));
        }
    };
    let diagnostic = crate::diagnostic::DiagnosticProjection {
        code: "a2a_send_outcome_unknown".to_string(),
        stage: DiagnosticStageV1::ResidentRecovery,
        disposition: DiagnosticDispositionV1::EffectOutcomeUnknown,
        summary: summary.to_string(),
        subject: Some((
            "operation".to_string(),
            context
                .dispatch
                .operation()
                .handle()
                .1
                .operation_handle_id()
                .as_str()
                .to_string(),
        )),
        source: None,
        schema_reference: None,
        causes: Vec::new(),
        notes: vec!["The original SendMessage request must not be replayed.".to_string()],
        help: vec!["Reconcile owner-native state before selecting another action.".to_string()],
        facts: fact
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect(),
        retry: DiagnosticRetryV1::ReconcileFirst,
        effect: DiagnosticEffectCertaintyV1::OutcomeUnknown,
        request: Some((
            "a2a_json_rpc".to_string(),
            context.expected.context().request_id().as_str().to_string(),
        )),
        generation: None,
        receipt: None,
        omitted_causes: 0,
    }
    .admit(&identity);
    match diagnostic {
        Ok(diagnostic) => A2aSendResponseFailure::OutcomeUnknown(Box::new(A2aSendOutcomeUnknown {
            custody,
            context,
            response,
            diagnostic,
        })),
        Err(error) => A2aSendResponseFailure::Diagnostic(Box::new(A2aSendDiagnosticFailure {
            custody,
            context,
            response,
            kind: A2aSendDiagnosticFailureKind::OutcomeUnknown(summary),
            reason: error,
        })),
    }
}

const fn response_kind(facts: &A2aInitialResponseFacts) -> &'static str {
    match facts {
        A2aInitialResponseFacts::Task { .. } => "task",
        A2aInitialResponseFacts::DirectMessage => "direct_message",
        A2aInitialResponseFacts::Status => "status",
        A2aInitialResponseFacts::Artifact => "artifact",
        A2aInitialResponseFacts::RemoteError => "remote_error",
        A2aInitialResponseFacts::UnexpectedStreaming => "unexpected_streaming",
    }
}

const fn exchange_error_kind(reason: A2aExchangeError) -> &'static str {
    match reason {
        A2aExchangeError::InvalidLimits => "invalid_limits",
        A2aExchangeError::CoordinateMismatch => "coordinate_mismatch",
        A2aExchangeError::ClientAuthenticationUnavailable => "client_authentication_unavailable",
        A2aExchangeError::StreamingUnavailable => "streaming_unavailable",
        A2aExchangeError::LimitExceeded => "limit_exceeded",
        A2aExchangeError::ServiceUnavailable => "service_unavailable",
        A2aExchangeError::Cancelled => "cancelled",
        A2aExchangeError::Timeout => "timeout",
        A2aExchangeError::Connection => "connection",
        A2aExchangeError::HttpResponse => "http_response",
        A2aExchangeError::Response => "response",
    }
}

fn verify_release_custody(
    retained: &RetainedA2aSend<'_, '_>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<(), A2aSendReleaseError> {
    use crate::resident::OperationCustodyDatabaseEvidence;

    let candidate = retained.custody.candidate();
    if retained.custody.database_evidence() != OperationCustodyDatabaseEvidence::New
        || candidate.kind() != OperationCustodyTransitionKind::PrepareA2aSend
        || candidate
            .record(OperationCustodyRecordKind::A2aSendPreparation)
            .is_none_or(|record| record.coordinate() != &retained.release.preparation)
        || candidate.a2a_evidence().len() != 1
        || candidate.a2a_evidence()[0].bytes() != retained.release.exchange.bytes()
    {
        return Err(A2aSendReleaseError::Association);
    }
    let dispatch = record_projection::coordinate(
        retained.release.dispatch.observation().0,
        retained
            .release
            .dispatch
            .observation()
            .1
            .canonical_operation_observation_sha256()
            .as_str(),
    )
    .map_err(OperationObservationCustodyProjectionError::from)
    .map_err(OperationAttemptError::from)?;
    let preparation_query = OperationCustodyIndexQuery::A2aSendPreparation(
        pale_ports::OperationCustodyIndexKey::for_a2a_send_preparation(&dispatch),
    );
    let preparation = custody.read_index(
        candidate.store(),
        &preparation_query,
        retained.release.maximum_record_bytes,
    )?;
    if preparation
        .record()
        .is_none_or(|record| record.coordinate() != &retained.release.preparation)
    {
        return Err(A2aSendReleaseError::Association);
    }
    let checkpoint_query = OperationCustodyIndexQuery::A2aSendPreparationCheckpoint(
        pale_ports::OperationCustodyIndexKey::for_a2a_send_preparation_checkpoint(
            &retained.release.preparation,
        ),
    );
    let checkpoint = custody.read_index(
        candidate.store(),
        &checkpoint_query,
        retained.release.maximum_record_bytes,
    )?;
    if checkpoint.record() != Some(candidate.checkpoint().successor_record()) {
        return Err(A2aSendReleaseError::Association);
    }
    let evidence = &candidate.a2a_evidence()[0];
    if custody
        .read_a2a_evidence(
            candidate.store(),
            evidence.coordinate(),
            retained
                .release
                .exchange
                .coordinate()
                .limits()
                .evidence_bytes
                .get(),
        )?
        .as_ref()
        != Some(evidence)
    {
        return Err(A2aSendReleaseError::Association);
    }
    let checkpoint = candidate.checkpoint().successor();
    if !custody
        .read_checkpoint_head(
            candidate.store(),
            checkpoint.checkpoint_domain_id(),
            retained.release.maximum_record_bytes,
        )?
        .is_some_and(|(head, record)| {
            &head == checkpoint && &record == candidate.checkpoint().successor_record()
        })
    {
        return Err(A2aSendReleaseError::Association);
    }
    let expected = AntiRollbackAnchor::from(ControlDomainAntiRollbackAnchor::new(
        candidate.store().store_id().clone(),
        candidate.store().incarnation_id().clone(),
        checkpoint.checkpoint_domain_id().clone(),
        checkpoint.domain_epoch(),
        checkpoint.domain_sequence(),
        checkpoint.coordinate().clone(),
    ));
    if anchor.read(&expected.key())?.as_ref() != Some(&expected) {
        return Err(A2aSendReleaseError::Association);
    }
    Ok(())
}

fn head(
    reference: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<ControlCheckpointHead, A2aSendCheckpointError> {
    Ok(ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        *checkpoint.domain_epoch(),
        *checkpoint.domain_sequence(),
        *checkpoint.committed_transaction_sequence(),
        record_projection::coordinate(
            reference,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )
        .map_err(OperationObservationCustodyProjectionError::from)?,
    ))
}

fn checkpoint_record(
    reference: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
    maximum_record_bytes: usize,
) -> Result<ControlRecord, A2aSendCheckpointError> {
    let record = record_projection::record(
        record_projection::coordinate(
            reference,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )
        .map_err(OperationObservationCustodyProjectionError::from)?,
        checkpoint,
    )
    .map_err(OperationObservationCustodyProjectionError::from)?;
    if record.canonical_record_jcs().as_slice().len() > maximum_record_bytes {
        return Err(A2aSendCheckpointError::Association);
    }
    Ok(record)
}

fn push_change(
    leaf: A2aCustodyLeaf,
    changes: &mut Vec<AccumulatorLeafChange>,
    keys: &mut Vec<ChangedControlLeafKeyV1>,
) -> Result<(), A2aSendCheckpointError> {
    changes.push(
        AccumulatorLeafChange::new(leaf.hashed_key, None, Some(leaf.value))
            .map_err(OperationObservationCustodyProjectionError::from)?,
    );
    keys.push(leaf.key);
    Ok(())
}

fn leaf_error(error: OperationLeafProjectionError) -> A2aSendCheckpointError {
    match error {
        OperationLeafProjectionError::Store => {
            OperationObservationCustodyProjectionError::StoreScope
        }
        OperationLeafProjectionError::Identity(error) => error.into(),
        OperationLeafProjectionError::Canonicalize(error) => error.into(),
    }
    .into()
}

#[cfg(test)]
#[path = "../../../../tests/internal/resident/operation_dispatch/a2a_checkpoint.rs"]
pub(in crate::resident) mod test_support;
