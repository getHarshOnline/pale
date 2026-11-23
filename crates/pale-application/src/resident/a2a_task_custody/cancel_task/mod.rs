//! Durable known-task cancellation and exact remote read-back.

mod checkpoint;
mod service;

pub(in crate::resident) use checkpoint::ContinuableA2aCancelTaskCustodyFailure;
pub use checkpoint::{
    start_a2a_cancel_task, DatabaseCommittedA2aCancelTaskAnchorPending,
    PreparedA2aCancelTaskCustody, RetainedA2aCancelTask,
};
pub(in crate::resident) use service::{
    commit_a2a_cancel_task_disposition_continuable, commit_a2a_cancel_task_readback_continuable,
    A2aCancelTaskDispositionCommitInput, A2aCancelTaskDispositionPhase,
    A2aCancelTaskReadbackCommitInput, CompletedA2aCancelTaskDispositionCustody,
    CompletedA2aCancelTaskReadbackCustody, ContinuableA2aCancelTaskDispositionCommitFailure,
    ContinuableA2aCancelTaskReadbackCommitFailure,
};
pub use service::{
    prepare_a2a_cancel_task_custody, A2aCancelTaskAuthorizationContext,
    A2aCancelTaskDispositionFailure, A2aCancelTaskDispositionInput, A2aCancelTaskError,
    A2aCancelTaskInput, A2aCancelTaskNotStarted, A2aCancelTaskObservationInput,
    A2aCancelTaskOutcomeUnknown, A2aCancelTaskPoll, A2aCancelTaskStartFailure,
    A2aCancelTaskUncertainty, CompletedA2aCancelTaskReadback, InFlightA2aCancelTask,
    PreparedA2aCancelTaskDisposition, ReceivedA2aCancelTaskResponse,
};
