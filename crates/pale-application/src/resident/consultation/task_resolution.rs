//! Proof-bearing completion of retained consultation child work.
//!
//! A process-local completion or loss keeps its aggregate reservation until the
//! exact protected child reaches the exact parent collection and that parent
//! successor completes database and independent-anchor custody. This module
//! composes existing owners only. It chooses no consultation completion,
//! quorum, lateness, synthesis, or stop policy.

use pale_core::records::*;
use pale_ports::{AntiRollbackAnchorPort, OperationCustodyPort};

use super::task::{
    ResidentConsultationTaskOwner, ResidentConsultationTaskOwnerPoll, ResidentConsultationTaskPoll,
    ResidentConsultationTerminalJoinPreflightFailure, ResidentConsultationTerminalReleaseFailure,
};
use super::{
    CompletedConsultationCollection, ConsultationChildJoinFailure, ConsultationChildJoinInput,
    ConsultationCollectionCommitFailure, ConsultationCollectionError,
    ConsultationSuccessorCommitInput, ResidentConsultationCompletionReady,
};
use crate::resident::consultation::coordinator::ResidentConsultationCoordinator;
use crate::resident::{
    task_reservation::ResidentAggregateTaskReservations, OperationObservationAuthorityStoreResolver,
};

/// One child outcome whose exact parent join completed both custody layers.
#[allow(
    dead_code,
    reason = "the private result is inspected by the assembled resident task owner"
)]
pub(in crate::resident) struct CompletedResidentConsultationTerminal<Work, Output> {
    collection: CompletedConsultationCollection,
    outcome: ResidentConsultationTaskPoll<Work, Output>,
}

#[allow(
    dead_code,
    reason = "the private result is inspected by the assembled resident task owner"
)]
impl<Work, Output> CompletedResidentConsultationTerminal<Work, Output> {
    /// Borrow the committed parent collection successor.
    #[must_use]
    pub(in crate::resident) const fn collection(&self) -> &CompletedConsultationCollection {
        &self.collection
    }

    /// Borrow the attributable mechanical child outcome.
    #[must_use]
    pub(in crate::resident) const fn outcome(&self) -> &ResidentConsultationTaskPoll<Work, Output> {
        &self.outcome
    }
}

/// One continuation released after protected recovery proved uncertainty.
#[allow(
    dead_code,
    reason = "private result is consumed by process-host composition"
)]
pub(in crate::resident) struct CompletedResidentConsultationNonterminal<Work, Output> {
    outcome: ResidentConsultationTaskPoll<Work, Output>,
}

#[allow(
    dead_code,
    reason = "private result is inspected by process-host composition"
)]
impl<Work, Output> CompletedResidentConsultationNonterminal<Work, Output> {
    /// Borrow the mechanical completion or loss whose capacity was released.
    #[must_use]
    pub(in crate::resident) const fn outcome(&self) -> &ResidentConsultationTaskPoll<Work, Output> {
        &self.outcome
    }
}

/// Why a proved nonterminal child did not release its Consult capacity.
#[allow(
    dead_code,
    reason = "private continuations are consumed by process-host composition"
)]
pub(in crate::resident) enum ResidentConsultationNonterminalSettlementFailure<Work, Output> {
    /// Owner, batch, candidate, dispatch, recovery, or reservation differed.
    Preflight {
        ready: Box<ResidentConsultationCompletionReady<Work, Output>>,
        reason: ResidentConsultationTerminalJoinPreflightFailure,
    },
    /// Exact aggregate release rejected the unchanged continuation.
    Release {
        ready: Box<ResidentConsultationCompletionReady<Work, Output>>,
        reason: ResidentConsultationTerminalReleaseFailure,
    },
}

impl<Work, Output> std::fmt::Debug
    for ResidentConsultationNonterminalSettlementFailure<Work, Output>
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Preflight { .. } => "Preflight",
            Self::Release { .. } => "Release",
        };
        formatter
            .debug_struct("ResidentConsultationNonterminalSettlementFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl<Work, Output> ResidentConsultationNonterminalSettlementFailure<Work, Output> {
    /// Recover the unchanged completion for an exact settlement retry.
    pub(in crate::resident) fn into_ready(
        self,
    ) -> ResidentConsultationCompletionReady<Work, Output> {
        match self {
            Self::Preflight { ready, .. } | Self::Release { ready, .. } => *ready,
        }
    }
}

/// Release one Consult reservation after protected recovery proves uncertainty.
///
/// This function does not mutate the parent collection. The roster slot stays
/// outstanding and cannot be offered again by the retained batch owner.
pub(in crate::resident) fn settle_consultation_nonterminal<Work, Output>(
    tasks: &mut ResidentConsultationTaskOwner,
    reservations: &mut ResidentAggregateTaskReservations,
    ready: ResidentConsultationCompletionReady<Work, Output>,
    child: &crate::resident::RecoveredOperationObservation,
) -> Result<
    CompletedResidentConsultationNonterminal<Work, Output>,
    ResidentConsultationNonterminalSettlementFailure<Work, Output>,
> {
    let (current, poll) = ready.into_parts();
    if let Err(reason) = tasks.require_nonterminal_settlement(&poll, &current, child, reservations)
    {
        return Err(
            ResidentConsultationNonterminalSettlementFailure::Preflight {
                ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                    current, poll,
                )),
                reason,
            },
        );
    }
    match tasks.finish_proved_continuation(poll, reservations) {
        Ok(outcome) => Ok(CompletedResidentConsultationNonterminal { outcome }),
        Err(rejection) => Err(ResidentConsultationNonterminalSettlementFailure::Release {
            ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                current,
                *rejection.poll,
            )),
            reason: rejection.reason,
        }),
    }
}

/// Exact database or anchor candidate retained with its process-local outcome.
#[must_use = "continue the exact parent custody candidate without releasing capacity"]
pub(in crate::resident) struct ResidentConsultationTerminalCustody<Work, Output> {
    failure: ConsultationCollectionCommitFailure,
    poll: ResidentConsultationTaskOwnerPoll<Work, Output>,
}

impl<Work, Output> ResidentConsultationTerminalCustody<Work, Output> {
    /// Borrow the physical store that owns this retained terminal append.
    pub(in crate::resident) fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        self.poll.coordinate.store()
    }
}

impl<Work, Output> std::fmt::Debug for ResidentConsultationTerminalCustody<Work, Output> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentConsultationTerminalCustody")
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}

/// Completed parent custody whose exact aggregate release still remains.
#[must_use = "retry only the exact aggregate release without repeating parent custody"]
pub(in crate::resident) struct ResidentConsultationJoinedTerminal<Work, Output> {
    collection: CompletedConsultationCollection,
    poll: ResidentConsultationTaskOwnerPoll<Work, Output>,
}

impl<Work, Output> ResidentConsultationJoinedTerminal<Work, Output> {
    /// Borrow the physical store whose parent append already reached custody.
    pub(in crate::resident) fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        self.poll.coordinate.store()
    }
}

/// Why proof-bearing consultation completion has not finished.
#[must_use = "every failure retains the exact poll, custody, or joined proof"]
#[allow(
    dead_code,
    reason = "the private continuations are consumed by the assembled resident task owner"
)]
pub(in crate::resident) enum ResidentConsultationTerminalJoinFailure<Work, Output> {
    /// Borrowed association or reservation checks rejected before I/O.
    Preflight {
        ready: Box<ResidentConsultationCompletionReady<Work, Output>>,
        reason: ResidentConsultationTerminalJoinPreflightFailure,
    },
    /// Child or collection admission rejected before a physical candidate existed.
    Admission {
        ready: Box<ResidentConsultationCompletionReady<Work, Output>>,
        reason: ConsultationCollectionError,
    },
    /// Exact database or independent-anchor custody remains incomplete.
    Custody(Box<ResidentConsultationTerminalCustody<Work, Output>>),
    /// A replacement owner or reservation rejected custody continuation before I/O.
    CustodyPreflight {
        pending: Box<ResidentConsultationTerminalCustody<Work, Output>>,
        reason: ResidentConsultationTerminalJoinPreflightFailure,
    },
    /// Parent custody completed, but exact aggregate release was rejected.
    Release {
        joined: Box<ResidentConsultationJoinedTerminal<Work, Output>>,
        reason: ResidentConsultationTerminalReleaseFailure,
    },
    /// A replacement owner or reservation rejected release retry before mutation.
    ReleasePreflight {
        joined: Box<ResidentConsultationJoinedTerminal<Work, Output>>,
        reason: ResidentConsultationTerminalJoinPreflightFailure,
    },
}

impl<Work, Output> std::fmt::Debug for ResidentConsultationTerminalJoinFailure<Work, Output> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Preflight { .. } => "Preflight",
            Self::Admission { .. } => "Admission",
            Self::Custody(_) => "Custody",
            Self::CustodyPreflight { .. } => "CustodyPreflight",
            Self::Release { .. } => "Release",
            Self::ReleasePreflight { .. } => "ReleasePreflight",
        };
        formatter
            .debug_struct("ResidentConsultationTerminalJoinFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Join one exact terminal child and release capacity only after full custody.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn join_consultation_terminal<
    'operation,
    Work,
    Output,
    AdmitEvidence,
    AdmitCollection,
    Rotation,
    ObservationSign,
    CheckpointSign,
>(
    tasks: &mut ResidentConsultationTaskOwner,
    reservations: &mut ResidentAggregateTaskReservations,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    ready: ResidentConsultationCompletionReady<Work, Output>,
    input: ConsultationChildJoinInput<'operation>,
    checkpoint: ConsultationSuccessorCommitInput<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    admit_evidence: AdmitEvidence,
    admit_collection: AdmitCollection,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<
    CompletedResidentConsultationTerminal<Work, Output>,
    ResidentConsultationTerminalJoinFailure<Work, Output>,
>
where
    AdmitEvidence: FnOnce(
        &super::ConsultationRosterSlot,
        &ImmutableRecordReferenceV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    AdmitCollection: FnOnce(&super::ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
    Rotation: FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    ObservationSign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    CheckpointSign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let (current, poll) = ready.into_parts();
    if let Err(reason) = tasks.require_terminal_join(&poll, &current, &input, reservations) {
        return Err(ResidentConsultationTerminalJoinFailure::Preflight {
            ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                current, poll,
            )),
            reason,
        });
    }
    let completed = ResidentConsultationCoordinator::new(custody, anchor).join_child(
        &current,
        input,
        checkpoint,
        resolve_store,
        admit_evidence,
        admit_collection,
        admit_rotation,
        sign_observation,
        sign_checkpoint,
    );
    match completed {
        Ok(collection) => finish_joined(
            tasks,
            reservations,
            ResidentConsultationJoinedTerminal { collection, poll },
        ),
        Err(ConsultationChildJoinFailure::Admission(reason)) => {
            Err(ResidentConsultationTerminalJoinFailure::Admission {
                ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                    current, poll,
                )),
                reason,
            })
        }
        Err(ConsultationChildJoinFailure::Custody(failure)) => {
            Err(ResidentConsultationTerminalJoinFailure::Custody(Box::new(
                ResidentConsultationTerminalCustody { failure, poll },
            )))
        }
    }
}

/// Continue only the exact database or anchor candidate retained by a join.
pub(in crate::resident) fn continue_consultation_terminal_custody<Work, Output>(
    tasks: &mut ResidentConsultationTaskOwner,
    reservations: &mut ResidentAggregateTaskReservations,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    pending: ResidentConsultationTerminalCustody<Work, Output>,
) -> Result<
    CompletedResidentConsultationTerminal<Work, Output>,
    ResidentConsultationTerminalJoinFailure<Work, Output>,
> {
    if let Err(reason) = tasks.require_pending_terminal(&pending.poll, reservations) {
        return Err(ResidentConsultationTerminalJoinFailure::CustodyPreflight {
            pending: Box::new(pending),
            reason,
        });
    }
    let ResidentConsultationTerminalCustody { failure, poll } = pending;
    match failure.continue_custody(custody, anchor) {
        Ok(collection) => finish_joined(
            tasks,
            reservations,
            ResidentConsultationJoinedTerminal { collection, poll },
        ),
        Err(failure) => Err(ResidentConsultationTerminalJoinFailure::Custody(Box::new(
            ResidentConsultationTerminalCustody { failure, poll },
        ))),
    }
}

/// Retry only aggregate release after parent custody is already complete.
pub(in crate::resident) fn retry_consultation_terminal_release<Work, Output>(
    tasks: &mut ResidentConsultationTaskOwner,
    reservations: &mut ResidentAggregateTaskReservations,
    joined: ResidentConsultationJoinedTerminal<Work, Output>,
) -> Result<
    CompletedResidentConsultationTerminal<Work, Output>,
    ResidentConsultationTerminalJoinFailure<Work, Output>,
> {
    if let Err(reason) = tasks.require_pending_terminal(&joined.poll, reservations) {
        return Err(ResidentConsultationTerminalJoinFailure::ReleasePreflight {
            joined: Box::new(joined),
            reason,
        });
    }
    finish_joined(tasks, reservations, joined)
}

fn finish_joined<Work, Output>(
    tasks: &mut ResidentConsultationTaskOwner,
    reservations: &mut ResidentAggregateTaskReservations,
    joined: ResidentConsultationJoinedTerminal<Work, Output>,
) -> Result<
    CompletedResidentConsultationTerminal<Work, Output>,
    ResidentConsultationTerminalJoinFailure<Work, Output>,
> {
    let ResidentConsultationJoinedTerminal { collection, poll } = joined;
    match tasks.finish_proved_continuation(poll, reservations) {
        Ok(outcome) => Ok(CompletedResidentConsultationTerminal {
            collection,
            outcome,
        }),
        Err(rejection) => Err(ResidentConsultationTerminalJoinFailure::Release {
            joined: Box::new(ResidentConsultationJoinedTerminal {
                collection,
                poll: *rejection.poll,
            }),
            reason: rejection.reason,
        }),
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/consultation/task_resolution.rs"]
mod tests;
