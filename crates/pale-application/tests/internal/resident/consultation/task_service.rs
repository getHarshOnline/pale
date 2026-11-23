//! Process-local laws for bounded consultation child work.

use super::*;
use crate::resident::operation_host::ResidentOperationHost;
use crate::resident::operation_observation_custody::tests::{RecordingAnchor, RecordingStore};
use crate::resident::operation_service::ResidentOperationTaskState;
use crate::resident::task_owner::{
    ResidentOperationTask, ResidentOperationTaskCancellationState, ResidentOperationTaskCompletion,
    ResidentOperationTaskExecutor, ResidentOperationTaskLimits,
};
use crate::resident::task_reservation::{
    ResidentAggregateTaskLimits, ResidentAggregateTaskReservations, ResidentTaskReservationPhase,
    ResidentTaskReservationRoute, ResidentTaskReservationTransitionFailure,
};
use crate::resident::{ConsultationRequirement, ConsultationSlotId};
use crate::session_admission_fixture::ProvingAuthority;
use pale_core::records::{
    ArtifactDigest, ControlIncarnationIdV1, ControlStoreIdV1, ImmutableRecordReferenceV1,
    ParticipantRefV1, ResidentRecordRefV1, UnixTimeV1,
};
use pale_ports::{
    AuthorityStoreCoordinate, OperationCustodyCommitOutcome, WallClockSourceError,
    WallClockSourcePort,
};
use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
};

use fixtures::*;

#[path = "task_service/admission.rs"]
mod admission;
#[path = "task_service/batch_lifecycle.rs"]
mod batch_lifecycle;
#[path = "task_service/completion.rs"]
mod completion;
#[path = "task_service/fixtures.rs"]
mod fixtures;
#[path = "task_service/operation_host.rs"]
mod operation_host;
#[path = "task_service/ownership.rs"]
mod ownership;
#[path = "task_service/recovered_loss.rs"]
mod recovered_loss;
#[path = "task_service/reservations.rs"]
mod reservations;
