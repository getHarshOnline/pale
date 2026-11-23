//! Close one completed A2A send branch through its existing custody owner.
//!
//! This module composes branch-specific admission and retention. It owns no
//! carrier, identity allocation, signing authority, storage selection, or
//! anchor policy, and therefore cannot replay an accepted request or invent a
//! remote Task.

use super::*;
use crate::resident::{
    commit_a2a_initial_task_response_continuable, A2aInitialTaskCommitFailure,
    A2aInitialTaskCommitInput, CompletedA2aInitialTaskCustody,
    ContinuableA2aInitialTaskCommitFailure,
};
use pale_ports::{AntiRollbackAnchorPort, OperationCustodyPort};

type EvidenceAdmission<'a> = Box<
    dyn FnOnce(
            &A2aTaskBindingPreimageV1,
            &A2aTaskObservationPreimageV1,
            &[OperationA2aEvidence],
            &[OperationA2aEvidence],
        ) -> Result<(), InvalidResidentRecordV1>
        + 'a,
>;
type BindingSigner<'a> = Box<
    dyn FnOnce(
            &[u8],
            &A2aTaskBindingPreimageV1,
            &A2aTaskBindingDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
        + 'a,
>;
type TaskObservationSigner<'a> = Box<
    dyn FnOnce(
            &[u8],
            &A2aTaskObservationPreimageV1,
            &A2aTaskObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
        + 'a,
>;
type StoreResolver<'a> = Box<
    dyn FnOnce(
            &A2aTaskObservationV1,
        ) -> Result<pale_ports::AuthorityStoreCoordinate, InvalidResidentRecordV1>
        + 'a,
>;
type RotationAdmission<'a> = Box<
    dyn FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>
        + 'a,
>;
type ReconciliationAdmission<'a> =
    Box<dyn FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1> + 'a>;
type OperationObservationSigner<'a> = Box<
    dyn FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
        + 'a,
>;
type CheckpointSigner<'a> = Box<
    dyn FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
        + 'a,
>;

/// Caller-supplied continuation for one response that contains an actual Task.
pub struct A2aTaskOutcomeRetention<'a> {
    response: Box<CompletedA2aTaskResponse>,
    input: A2aInitialTaskCommitInput<'a>,
    admit_evidence: EvidenceAdmission<'a>,
    sign_binding: BindingSigner<'a>,
    sign_observation: TaskObservationSigner<'a>,
    resolve_store: StoreResolver<'a>,
    sign_checkpoint: CheckpointSigner<'a>,
}

impl<'a> A2aTaskOutcomeRetention<'a> {
    /// Bind an actual remote Task using fresh caller-owned identities and owners.
    #[allow(clippy::too_many_arguments)]
    pub fn new<Evidence, BindingSign, ObservationSign, Store, CheckpointSign>(
        response: Box<CompletedA2aTaskResponse>,
        input: A2aInitialTaskCommitInput<'a>,
        admit_evidence: Evidence,
        sign_binding: BindingSign,
        sign_observation: ObservationSign,
        resolve_store: Store,
        sign_checkpoint: CheckpointSign,
    ) -> Self
    where
        Evidence: FnOnce(
                &A2aTaskBindingPreimageV1,
                &A2aTaskObservationPreimageV1,
                &[OperationA2aEvidence],
                &[OperationA2aEvidence],
            ) -> Result<(), InvalidResidentRecordV1>
            + 'a,
        BindingSign: FnOnce(
                &[u8],
                &A2aTaskBindingPreimageV1,
                &A2aTaskBindingDigest,
            ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
            + 'a,
        ObservationSign: FnOnce(
                &[u8],
                &A2aTaskObservationPreimageV1,
                &A2aTaskObservationDigest,
            ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
            + 'a,
        Store: FnOnce(
                &A2aTaskObservationV1,
            )
                -> Result<pale_ports::AuthorityStoreCoordinate, InvalidResidentRecordV1>
            + 'a,
        CheckpointSign: FnOnce(
                &[u8],
                &ControlStateCheckpointPreimageV1,
                &ControlStateCheckpointDigest,
            ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
            + 'a,
    {
        Self {
            response,
            input,
            admit_evidence: Box::new(admit_evidence),
            sign_binding: Box::new(sign_binding),
            sign_observation: Box::new(sign_observation),
            resolve_store: Box::new(resolve_store),
            sign_checkpoint: Box::new(sign_checkpoint),
        }
    }

    pub(in crate::resident) fn process_association(
        &self,
    ) -> Result<
        (
            crate::resident::task_owner::ResidentOperationTaskKey,
            A2aOutcomeRetentionKind,
        ),
        OperationAttemptError,
    > {
        Ok((
            self.response.process_task_key()?,
            A2aOutcomeRetentionKind::Task,
        ))
    }
}

/// Caller-supplied continuation for one completed response without a Task.
pub struct A2aNoTaskOutcomeRetention<'a> {
    response: Box<CompletedA2aNoTaskResponse>,
    observation: A2aFailedSendObservationInput<'a>,
    commit: A2aSendObservationCommitInput<'a>,
    resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    admit_rotation: RotationAdmission<'a>,
    sign_observation: OperationObservationSigner<'a>,
    sign_checkpoint: CheckpointSigner<'a>,
}

impl<'a> A2aNoTaskOutcomeRetention<'a> {
    /// Retain a direct Message or other task-less response without task binding.
    #[allow(clippy::too_many_arguments)]
    pub fn new<Rotation, ObservationSign, CheckpointSign>(
        response: Box<CompletedA2aNoTaskResponse>,
        observation: A2aFailedSendObservationInput<'a>,
        commit: A2aSendObservationCommitInput<'a>,
        resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Self
    where
        Rotation: FnOnce(
                &ResidentRecordRefV1,
                &ArtifactDigest,
                &ResidentRecordRefV1,
                &ArtifactDigest,
            ) -> Result<(), InvalidResidentRecordV1>
            + 'a,
        ObservationSign: FnOnce(
                &[u8],
                &OperationObservationPreimageV1,
                &OperationObservationDigest,
            ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
            + 'a,
        CheckpointSign: FnOnce(
                &[u8],
                &ControlStateCheckpointPreimageV1,
                &ControlStateCheckpointDigest,
            ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
            + 'a,
    {
        Self {
            response,
            observation,
            commit,
            resolve_store,
            admit_rotation: Box::new(admit_rotation),
            sign_observation: Box::new(sign_observation),
            sign_checkpoint: Box::new(sign_checkpoint),
        }
    }

    pub(in crate::resident) fn process_association(
        &self,
    ) -> Result<
        (
            crate::resident::task_owner::ResidentOperationTaskKey,
            A2aOutcomeRetentionKind,
        ),
        OperationAttemptError,
    > {
        Ok((
            self.response.process_task_key()?,
            A2aOutcomeRetentionKind::NoTask,
        ))
    }
}

/// Caller-supplied continuation for one request rejected before queue acceptance.
pub struct A2aNotStartedOutcomeRetention<'a> {
    not_started: Box<A2aSendNotStarted>,
    observation: A2aFailedSendObservationInput<'a>,
    commit: A2aSendObservationCommitInput<'a>,
    resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    admit_rotation: RotationAdmission<'a>,
    sign_observation: OperationObservationSigner<'a>,
    sign_checkpoint: CheckpointSigner<'a>,
}

impl<'a> A2aNotStartedOutcomeRetention<'a> {
    /// Retain a confirmed queue rejection without inferring a remote effect.
    #[allow(clippy::too_many_arguments)]
    pub fn new<Rotation, ObservationSign, CheckpointSign>(
        not_started: Box<A2aSendNotStarted>,
        observation: A2aFailedSendObservationInput<'a>,
        commit: A2aSendObservationCommitInput<'a>,
        resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Self
    where
        Rotation: FnOnce(
                &ResidentRecordRefV1,
                &ArtifactDigest,
                &ResidentRecordRefV1,
                &ArtifactDigest,
            ) -> Result<(), InvalidResidentRecordV1>
            + 'a,
        ObservationSign: FnOnce(
                &[u8],
                &OperationObservationPreimageV1,
                &OperationObservationDigest,
            ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
            + 'a,
        CheckpointSign: FnOnce(
                &[u8],
                &ControlStateCheckpointPreimageV1,
                &ControlStateCheckpointDigest,
            ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
            + 'a,
    {
        Self {
            not_started,
            observation,
            commit,
            resolve_store,
            admit_rotation: Box::new(admit_rotation),
            sign_observation: Box::new(sign_observation),
            sign_checkpoint: Box::new(sign_checkpoint),
        }
    }

    pub(in crate::resident) fn process_association(
        &self,
    ) -> Result<
        (
            crate::resident::task_owner::ResidentOperationTaskKey,
            A2aOutcomeRetentionKind,
        ),
        OperationAttemptError,
    > {
        Ok((
            self.not_started.process_task_key()?,
            A2aOutcomeRetentionKind::NotStarted,
        ))
    }
}

/// Caller-supplied continuation for one accepted send with an uncertain outcome.
pub struct A2aUnknownOutcomeRetention<'a> {
    outcome: Box<A2aSendOutcomeUnknown>,
    observation: A2aOutcomeUnknownObservationInput<'a>,
    commit: A2aSendObservationCommitInput<'a>,
    resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    admit_reconciliation: ReconciliationAdmission<'a>,
    admit_rotation: RotationAdmission<'a>,
    sign_observation: OperationObservationSigner<'a>,
    sign_checkpoint: CheckpointSigner<'a>,
}

impl<'a> A2aUnknownOutcomeRetention<'a> {
    /// Retain uncertainty with an existing reconciliation frontier and no replay.
    #[allow(clippy::too_many_arguments)]
    pub fn new<Reconciliation, Rotation, ObservationSign, CheckpointSign>(
        outcome: Box<A2aSendOutcomeUnknown>,
        observation: A2aOutcomeUnknownObservationInput<'a>,
        commit: A2aSendObservationCommitInput<'a>,
        resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Self
    where
        Reconciliation:
            FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1> + 'a,
        Rotation: FnOnce(
                &ResidentRecordRefV1,
                &ArtifactDigest,
                &ResidentRecordRefV1,
                &ArtifactDigest,
            ) -> Result<(), InvalidResidentRecordV1>
            + 'a,
        ObservationSign: FnOnce(
                &[u8],
                &OperationObservationPreimageV1,
                &OperationObservationDigest,
            ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
            + 'a,
        CheckpointSign: FnOnce(
                &[u8],
                &ControlStateCheckpointPreimageV1,
                &ControlStateCheckpointDigest,
            ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
            + 'a,
    {
        Self {
            outcome,
            observation,
            commit,
            resolve_store,
            admit_reconciliation: Box::new(admit_reconciliation),
            admit_rotation: Box::new(admit_rotation),
            sign_observation: Box::new(sign_observation),
            sign_checkpoint: Box::new(sign_checkpoint),
        }
    }

    pub(in crate::resident) fn process_association(
        &self,
    ) -> Result<
        (
            crate::resident::task_owner::ResidentOperationTaskKey,
            A2aOutcomeRetentionKind,
        ),
        OperationAttemptError,
    > {
        Ok((
            self.outcome.process_task_key()?,
            A2aOutcomeRetentionKind::OutcomeUnknown,
        ))
    }
}

/// Caller-supplied continuation for a retained send recovered after restart.
///
/// The recovered phase has no carrier or release method. This continuation can
/// only retain the existing outcome-unknown disposition.
pub struct A2aRecoveredUnknownOutcomeRetention<'a> {
    outcome: Box<RecoveredA2aSendOutcomeUnknown<'a>>,
    observation: A2aOutcomeUnknownObservationInput<'a>,
    commit: A2aSendObservationCommitInput<'a>,
    resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    admit_reconciliation: ReconciliationAdmission<'a>,
    admit_rotation: RotationAdmission<'a>,
    sign_observation: OperationObservationSigner<'a>,
    sign_checkpoint: CheckpointSigner<'a>,
}

impl<'a> A2aRecoveredUnknownOutcomeRetention<'a> {
    /// Continue exact recovered send custody into one durable uncertainty record.
    #[allow(clippy::too_many_arguments)]
    pub fn new<Reconciliation, Rotation, ObservationSign, CheckpointSign>(
        outcome: Box<RecoveredA2aSendOutcomeUnknown<'a>>,
        observation: A2aOutcomeUnknownObservationInput<'a>,
        commit: A2aSendObservationCommitInput<'a>,
        resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Self
    where
        Reconciliation:
            FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1> + 'a,
        Rotation: FnOnce(
                &ResidentRecordRefV1,
                &ArtifactDigest,
                &ResidentRecordRefV1,
                &ArtifactDigest,
            ) -> Result<(), InvalidResidentRecordV1>
            + 'a,
        ObservationSign: FnOnce(
                &[u8],
                &OperationObservationPreimageV1,
                &OperationObservationDigest,
            ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
            + 'a,
        CheckpointSign: FnOnce(
                &[u8],
                &ControlStateCheckpointPreimageV1,
                &ControlStateCheckpointDigest,
            ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
            + 'a,
    {
        Self {
            outcome,
            observation,
            commit,
            resolve_store,
            admit_reconciliation: Box::new(admit_reconciliation),
            admit_rotation: Box::new(admit_rotation),
            sign_observation: Box::new(sign_observation),
            sign_checkpoint: Box::new(sign_checkpoint),
        }
    }

    pub(in crate::resident) fn process_association(
        &self,
    ) -> Result<
        (
            crate::resident::task_owner::ResidentOperationTaskKey,
            A2aOutcomeRetentionKind,
        ),
        OperationAttemptError,
    > {
        Ok((
            self.outcome.process_task_key()?,
            A2aOutcomeRetentionKind::OutcomeUnknown,
        ))
    }
}

/// Closed completed/failure branches accepted by A2A outcome retention.
pub enum A2aOutcomeRetentionInput<'a> {
    /// A completed response carrying an actual remote Task identity.
    Task(Box<A2aTaskOutcomeRetention<'a>>),
    /// A completed response that cannot create a Task binding.
    NoTask(A2aNoTaskOutcomeRetention<'a>),
    /// A carrier rejection before queue acceptance.
    NotStarted(A2aNotStartedOutcomeRetention<'a>),
    /// An accepted or cancelled send whose remote outcome is uncertain.
    OutcomeUnknown(A2aUnknownOutcomeRetention<'a>),
    /// A durably retained send recovered after restart with unknown remote effect.
    RecoveredOutcomeUnknown(A2aRecoveredUnknownOutcomeRetention<'a>),
}

impl A2aOutcomeRetentionInput<'_> {
    fn process_association(
        &self,
    ) -> Result<
        (
            crate::resident::task_owner::ResidentOperationTaskKey,
            A2aOutcomeRetentionKind,
        ),
        OperationAttemptError,
    > {
        match self {
            Self::Task(input) => input.process_association(),
            Self::NoTask(input) => input.process_association(),
            Self::NotStarted(input) => input.process_association(),
            Self::OutcomeUnknown(input) => input.process_association(),
            Self::RecoveredOutcomeUnknown(input) => input.process_association(),
        }
    }
}

/// Private terminal branch used to fence `ProcessHost` retention before I/O.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum A2aOutcomeRetentionKind {
    Task,
    NoTask,
    NotStarted,
    OutcomeUnknown,
}

/// Durable result of closing exactly one A2A send branch.
#[must_use = "retain the typed A2A outcome custody result"]
pub enum RetainedA2aOutcome {
    /// Actual Task binding and revision-one remote history custody.
    Task(CompletedA2aInitialTaskCustody),
    /// Task-less response retained as a failed local observation.
    NoTask(CompletedA2aSendObservationCustody),
    /// Queue rejection retained as a failed local observation.
    NotStarted(CompletedA2aSendObservationCustody),
    /// Accepted-send uncertainty retained for reconciliation.
    OutcomeUnknown(CompletedA2aSendObservationCustody),
}

impl RetainedA2aOutcome {
    pub(in crate::resident) const fn process_kind(&self) -> A2aOutcomeRetentionKind {
        match self {
            Self::Task(_) => A2aOutcomeRetentionKind::Task,
            Self::NoTask(_) => A2aOutcomeRetentionKind::NoTask,
            Self::NotStarted(_) => A2aOutcomeRetentionKind::NotStarted,
            Self::OutcomeUnknown(_) => A2aOutcomeRetentionKind::OutcomeUnknown,
        }
    }

    pub(in crate::resident) fn matches_process_association(
        &self,
        key: &crate::resident::task_owner::ResidentOperationTaskKey,
        kind: A2aOutcomeRetentionKind,
    ) -> bool {
        match (kind, self) {
            (A2aOutcomeRetentionKind::Task, Self::Task(value)) => {
                let binding = value.binding().1;
                value.custody().candidate().store() == key.store()
                    && binding.operation_observation_ref() == &key.dispatch().record_ref
                    && binding.operation_observation_sha256().as_str()
                        == key.dispatch().record_sha256.as_str()
            }
            (A2aOutcomeRetentionKind::NoTask, Self::NoTask(value))
            | (A2aOutcomeRetentionKind::NotStarted, Self::NotStarted(value))
            | (A2aOutcomeRetentionKind::OutcomeUnknown, Self::OutcomeUnknown(value)) => {
                let observation = value.observation().1;
                value.custody().candidate().store() == key.store()
                    && observation.predecessor_ref().as_ref() == Some(&key.dispatch().record_ref)
                    && observation
                        .predecessor_sha256()
                        .as_ref()
                        .is_some_and(|digest| {
                            digest.as_str() == key.dispatch().record_sha256.as_str()
                        })
            }
            _ => false,
        }
    }
}

/// Phase-typed failure from the selected A2A retention branch.
#[derive(Debug)]
pub enum A2aOutcomeRetentionFailure<'a> {
    /// The actual-Task custody owner stopped.
    Task(A2aInitialTaskCommitFailure),
    /// Task-less observation admission stopped before checkpoint custody.
    NoTaskObservation(A2aNoTaskObservationFailure),
    /// Task-less observation checkpoint, database, or anchor custody stopped.
    NoTaskCommit(A2aSendObservationCommitFailure),
    /// Queue-rejection observation admission stopped before checkpoint custody.
    NotStartedObservation(A2aNotStartedObservationFailure),
    /// Queue-rejection observation checkpoint, database, or anchor custody stopped.
    NotStartedCommit(A2aSendObservationCommitFailure),
    /// Uncertain-outcome observation admission stopped before checkpoint custody.
    OutcomeUnknownObservation(A2aOutcomeUnknownObservationFailure),
    /// Uncertain-outcome checkpoint, database, or anchor custody stopped.
    OutcomeUnknownCommit(A2aSendObservationCommitFailure),
    /// Recovered uncertainty observation admission stopped before checkpoint custody.
    RecoveredOutcomeUnknownObservation(RecoveredA2aSendObservationFailure<'a>),
    /// Recovered uncertainty checkpoint, database, or anchor custody stopped.
    RecoveredOutcomeUnknownCommit(
        A2aSendObservationCommitFailure<PreparedRecoveredA2aSendObservation<'a>>,
    ),
}

enum A2aOutcomeRetentionContinuation<'a> {
    Task(ContinuableA2aInitialTaskCommitFailure),
    NoTaskObservation(A2aNoTaskObservationFailure),
    NoTaskCommit(
        super::response::ContinuableA2aSendObservationCommitFailure<PreparedA2aSendObservation>,
    ),
    NotStartedObservation(A2aNotStartedObservationFailure),
    NotStartedCommit(
        super::response::ContinuableA2aSendObservationCommitFailure<PreparedA2aSendObservation>,
    ),
    OutcomeUnknownObservation(A2aOutcomeUnknownObservationFailure),
    OutcomeUnknownCommit(
        super::response::ContinuableA2aSendObservationCommitFailure<PreparedA2aSendObservation>,
    ),
    RecoveredOutcomeUnknownObservation(RecoveredA2aSendObservationFailure<'a>),
    RecoveredOutcomeUnknownCommit(
        super::response::ContinuableA2aSendObservationCommitFailure<
            PreparedRecoveredA2aSendObservation<'a>,
        >,
    ),
}

pub(in crate::resident) struct ContinuableA2aOutcomeRetentionFailure<'a> {
    association: Option<(
        crate::resident::task_owner::ResidentOperationTaskKey,
        A2aOutcomeRetentionKind,
    )>,
    continuation: Box<A2aOutcomeRetentionContinuation<'a>>,
}

impl<'a> ContinuableA2aOutcomeRetentionFailure<'a> {
    fn new(
        association: Option<(
            crate::resident::task_owner::ResidentOperationTaskKey,
            A2aOutcomeRetentionKind,
        )>,
        continuation: A2aOutcomeRetentionContinuation<'a>,
    ) -> Self {
        Self {
            association,
            continuation: Box::new(continuation),
        }
    }

    pub(in crate::resident) fn matches_process_association(
        &self,
        key: &crate::resident::task_owner::ResidentOperationTaskKey,
        kind: A2aOutcomeRetentionKind,
    ) -> bool {
        self.association
            .as_ref()
            .is_some_and(|(actual_key, actual_kind)| actual_key == key && *actual_kind == kind)
    }

    fn into_public(self) -> A2aOutcomeRetentionFailure<'a> {
        match *self.continuation {
            A2aOutcomeRetentionContinuation::Task(failure) => {
                A2aOutcomeRetentionFailure::Task(failure.into_public())
            }
            A2aOutcomeRetentionContinuation::NoTaskObservation(failure) => {
                A2aOutcomeRetentionFailure::NoTaskObservation(failure)
            }
            A2aOutcomeRetentionContinuation::NoTaskCommit(failure) => {
                A2aOutcomeRetentionFailure::NoTaskCommit(failure.into_public())
            }
            A2aOutcomeRetentionContinuation::NotStartedObservation(failure) => {
                A2aOutcomeRetentionFailure::NotStartedObservation(failure)
            }
            A2aOutcomeRetentionContinuation::NotStartedCommit(failure) => {
                A2aOutcomeRetentionFailure::NotStartedCommit(failure.into_public())
            }
            A2aOutcomeRetentionContinuation::OutcomeUnknownObservation(failure) => {
                A2aOutcomeRetentionFailure::OutcomeUnknownObservation(failure)
            }
            A2aOutcomeRetentionContinuation::OutcomeUnknownCommit(failure) => {
                A2aOutcomeRetentionFailure::OutcomeUnknownCommit(failure.into_public())
            }
            A2aOutcomeRetentionContinuation::RecoveredOutcomeUnknownObservation(failure) => {
                A2aOutcomeRetentionFailure::RecoveredOutcomeUnknownObservation(failure)
            }
            A2aOutcomeRetentionContinuation::RecoveredOutcomeUnknownCommit(failure) => {
                A2aOutcomeRetentionFailure::RecoveredOutcomeUnknownCommit(failure.into_public())
            }
        }
    }

    pub(in crate::resident) fn continue_custody(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<RetainedA2aOutcome, Self> {
        let Self {
            association,
            continuation,
        } = self;
        match *continuation {
            A2aOutcomeRetentionContinuation::Task(failure) => failure
                .continue_custody(custody, anchor)
                .map(RetainedA2aOutcome::Task)
                .map_err(|failure| {
                    Self::new(association, A2aOutcomeRetentionContinuation::Task(failure))
                }),
            A2aOutcomeRetentionContinuation::NoTaskCommit(failure) => failure
                .continue_custody(custody, anchor)
                .map(RetainedA2aOutcome::NoTask)
                .map_err(|failure| {
                    Self::new(
                        association,
                        A2aOutcomeRetentionContinuation::NoTaskCommit(failure),
                    )
                }),
            A2aOutcomeRetentionContinuation::NotStartedCommit(failure) => failure
                .continue_custody(custody, anchor)
                .map(RetainedA2aOutcome::NotStarted)
                .map_err(|failure| {
                    Self::new(
                        association,
                        A2aOutcomeRetentionContinuation::NotStartedCommit(failure),
                    )
                }),
            A2aOutcomeRetentionContinuation::OutcomeUnknownCommit(failure) => failure
                .continue_custody(custody, anchor)
                .map(RetainedA2aOutcome::OutcomeUnknown)
                .map_err(|failure| {
                    Self::new(
                        association,
                        A2aOutcomeRetentionContinuation::OutcomeUnknownCommit(failure),
                    )
                }),
            A2aOutcomeRetentionContinuation::RecoveredOutcomeUnknownCommit(failure) => failure
                .continue_custody(custody, anchor)
                .map(RetainedA2aOutcome::OutcomeUnknown)
                .map_err(|failure| {
                    Self::new(
                        association,
                        A2aOutcomeRetentionContinuation::RecoveredOutcomeUnknownCommit(failure),
                    )
                }),
            continuation => Err(Self::new(association, continuation)),
        }
    }
}

/// Retain one completed A2A send branch through its existing semantic and
/// physical owners.
///
/// The selected branch supplies every fresh reference, signing context,
/// reconciliation proof, store resolver, and checkpoint input. This function
/// performs no transport call and cannot turn a task-less response into a Task.
///
/// # Errors
///
/// Returns the selected branch's exact admission, checkpoint, database, or
/// independent-anchor failure without granting request replay.
#[allow(clippy::too_many_lines)]
pub fn retain_a2a_outcome<'a>(
    input: A2aOutcomeRetentionInput<'a>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<RetainedA2aOutcome, A2aOutcomeRetentionFailure<'a>> {
    retain_a2a_outcome_continuable(input, custody, anchor)
        .map_err(ContinuableA2aOutcomeRetentionFailure::into_public)
}

#[allow(clippy::too_many_lines)]
pub(in crate::resident) fn retain_a2a_outcome_continuable<'a>(
    input: A2aOutcomeRetentionInput<'a>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<RetainedA2aOutcome, ContinuableA2aOutcomeRetentionFailure<'a>> {
    let association = input.process_association().ok();
    match input {
        A2aOutcomeRetentionInput::Task(input) => commit_a2a_initial_task_response_continuable(
            input.response,
            input.input,
            custody,
            anchor,
            input.admit_evidence,
            input.sign_binding,
            input.sign_observation,
            input.resolve_store,
            input.sign_checkpoint,
        )
        .map(RetainedA2aOutcome::Task)
        .map_err(|failure| {
            ContinuableA2aOutcomeRetentionFailure::new(
                association,
                A2aOutcomeRetentionContinuation::Task(failure),
            )
        }),
        A2aOutcomeRetentionInput::NoTask(input) => {
            let prepared = input
                .response
                .prepare_observation(
                    input.observation,
                    input.admit_rotation,
                    input.sign_observation,
                )
                .map_err(|failure| {
                    ContinuableA2aOutcomeRetentionFailure::new(
                        association.clone(),
                        A2aOutcomeRetentionContinuation::NoTaskObservation(failure),
                    )
                })?;
            super::response::commit_a2a_send_observation_continuable(
                prepared,
                input.commit,
                input.resolve_store,
                custody,
                anchor,
                input.sign_checkpoint,
            )
            .map(RetainedA2aOutcome::NoTask)
            .map_err(|failure| {
                ContinuableA2aOutcomeRetentionFailure::new(
                    association,
                    A2aOutcomeRetentionContinuation::NoTaskCommit(failure),
                )
            })
        }
        A2aOutcomeRetentionInput::NotStarted(input) => {
            let prepared = input
                .not_started
                .prepare_observation(
                    input.observation,
                    input.admit_rotation,
                    input.sign_observation,
                )
                .map_err(|failure| {
                    ContinuableA2aOutcomeRetentionFailure::new(
                        association.clone(),
                        A2aOutcomeRetentionContinuation::NotStartedObservation(failure),
                    )
                })?;
            super::response::commit_a2a_send_observation_continuable(
                prepared,
                input.commit,
                input.resolve_store,
                custody,
                anchor,
                input.sign_checkpoint,
            )
            .map(RetainedA2aOutcome::NotStarted)
            .map_err(|failure| {
                ContinuableA2aOutcomeRetentionFailure::new(
                    association,
                    A2aOutcomeRetentionContinuation::NotStartedCommit(failure),
                )
            })
        }
        A2aOutcomeRetentionInput::OutcomeUnknown(input) => {
            let prepared = input
                .outcome
                .prepare_observation(
                    input.observation,
                    input.admit_reconciliation,
                    input.admit_rotation,
                    input.sign_observation,
                )
                .map_err(|failure| {
                    ContinuableA2aOutcomeRetentionFailure::new(
                        association.clone(),
                        A2aOutcomeRetentionContinuation::OutcomeUnknownObservation(failure),
                    )
                })?;
            super::response::commit_a2a_send_observation_continuable(
                prepared,
                input.commit,
                input.resolve_store,
                custody,
                anchor,
                input.sign_checkpoint,
            )
            .map(RetainedA2aOutcome::OutcomeUnknown)
            .map_err(|failure| {
                ContinuableA2aOutcomeRetentionFailure::new(
                    association,
                    A2aOutcomeRetentionContinuation::OutcomeUnknownCommit(failure),
                )
            })
        }
        A2aOutcomeRetentionInput::RecoveredOutcomeUnknown(input) => {
            let prepared = input
                .outcome
                .prepare_observation(
                    input.observation,
                    input.admit_reconciliation,
                    input.admit_rotation,
                    input.sign_observation,
                )
                .map_err(|failure| {
                    ContinuableA2aOutcomeRetentionFailure::new(
                        association.clone(),
                        A2aOutcomeRetentionContinuation::RecoveredOutcomeUnknownObservation(
                            failure,
                        ),
                    )
                })?;
            super::response::commit_recovered_a2a_send_observation_continuable(
                prepared,
                input.commit,
                input.resolve_store,
                custody,
                anchor,
                input.sign_checkpoint,
            )
            .map(RetainedA2aOutcome::OutcomeUnknown)
            .map_err(|failure| {
                ContinuableA2aOutcomeRetentionFailure::new(
                    association,
                    A2aOutcomeRetentionContinuation::RecoveredOutcomeUnknownCommit(failure),
                )
            })
        }
    }
}
