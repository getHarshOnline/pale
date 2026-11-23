//! Known-binding `GetTask` evidence and one-shot read-back phases.

mod evidence;
pub use evidence::admit_a2a_get_task_evidence;

mod service;
pub use service::{
    commit_a2a_get_task_readback, start_a2a_get_task_readback, A2aGetTaskCommitFailure,
    A2aGetTaskCommitInput, A2aGetTaskObservationInput, A2aGetTaskReadbackAuthorizationContext,
    A2aGetTaskReadbackError, A2aGetTaskReadbackInput, A2aGetTaskReadbackPoll,
    CompletedA2aGetTaskReadback, CompletedA2aGetTaskReadbackCustody, InFlightA2aGetTaskReadback,
    ReceivedA2aGetTaskReadback,
};
pub(in crate::resident) use service::{
    commit_a2a_get_task_readback_continuable, ContinuableA2aGetTaskCommitFailure,
};

pub(in crate::resident) mod custody;
