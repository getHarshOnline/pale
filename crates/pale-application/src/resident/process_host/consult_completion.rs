//! Process-local routing for an already-polled Consult completion.
//!
//! A shared executor can return work for a different retained Consult parent
//! than the parent currently under supervision. This module rehomes that
//! linear result without polling the executor again or performing custody I/O.

use super::*;
use crate::resident::consultation::ResidentConsultationTaskOwnerPoll;

/// A ready Consult child bound to the deployment that owns its parent.
#[must_use = "join this exact completion through its retained deployment"]
pub(super) struct ResidentProcessConsultationCompletionReady<Work, Output> {
    pub(super) selector: ResidentDeploymentAuthoritySelector,
    pub(super) ready: Box<ResidentConsultationCompletionReady<Work, Output>>,
}

/// A selector refusal that preserves the unbound completion unchanged.
#[must_use = "retry deployment binding with the returned completion"]
pub(super) struct ResidentProcessConsultationCompletionBindingFailure<Work, Output> {
    pub(super) selector: ResidentDeploymentAuthoritySelector,
    pub(super) ready: Box<ResidentConsultationCompletionReady<Work, Output>>,
    pub(super) reason: ResidentProcessHostSelectionError,
}

/// Why an already-polled completion could not return to its retained parent.
pub(super) enum ResidentProcessConsultationCompletionRehomeReason {
    /// The completion came from another process-lifetime task owner.
    TaskOwner,
    /// The selector did not identify exactly one ready deployment.
    Deployment(ResidentProcessHostSelectionError),
    /// The completion coordinate names a different physical store.
    PollStore,
    /// The recovered parent names a different physical store.
    CollectionStore,
    /// The recovered collection belongs to another Consult parent.
    Parent,
    /// The recovered collection is not the roster frontier that owned the task.
    CollectionFrontier,
}

/// An ownership refusal that preserves both linear inputs unchanged.
#[must_use = "retry rehoming with the returned collection and completion"]
pub(super) struct ResidentProcessConsultationCompletionRehomeFailure<Work, Output> {
    pub(super) selector: ResidentDeploymentAuthoritySelector,
    pub(super) current: RecoveredConsultationCollection,
    pub(super) poll: ResidentConsultationTaskOwnerPoll<Work, Output>,
    pub(super) reason: ResidentProcessConsultationCompletionRehomeReason,
}

impl<WriterGuard> ResidentProcessHost<'_, WriterGuard> {
    /// Bind one ready Consult completion to its admitted deployment.
    ///
    /// This phase performs no port I/O. A rejected selector returns the intact
    /// completion so another exact selector can be tried without polling again.
    pub(super) fn bind_consultation_completion<Work, Output>(
        &self,
        selector: ResidentDeploymentAuthoritySelector,
        ready: ResidentConsultationCompletionReady<Work, Output>,
    ) -> Result<
        ResidentProcessConsultationCompletionReady<Work, Output>,
        Box<ResidentProcessConsultationCompletionBindingFailure<Work, Output>>,
    > {
        match self.deployment_authority_index(&selector) {
            Ok(index) if self.deployments[index].store() == ready.store() => {
                Ok(ResidentProcessConsultationCompletionReady {
                    selector,
                    ready: Box::new(ready),
                })
            }
            Ok(_) => Err(Box::new(
                ResidentProcessConsultationCompletionBindingFailure {
                    selector,
                    ready: Box::new(ready),
                    reason: ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation,
                },
            )),
            Err(reason) => Err(Box::new(
                ResidentProcessConsultationCompletionBindingFailure {
                    selector,
                    ready: Box::new(ready),
                    reason,
                },
            )),
        }
    }

    /// Return one foreign executor completion to its exact recovered parent.
    ///
    /// All ownership checks finish before later custody I/O. A refusal returns
    /// the collection and poll unchanged, so the caller never polls twice.
    pub(super) fn rehome_foreign_consultation_completion<Work, Output>(
        &self,
        selector: ResidentDeploymentAuthoritySelector,
        current: RecoveredConsultationCollection,
        poll: ResidentConsultationTaskOwnerPoll<Work, Output>,
    ) -> Result<
        ResidentProcessConsultationCompletionReady<Work, Output>,
        Box<ResidentProcessConsultationCompletionRehomeFailure<Work, Output>>,
    > {
        let reason = if !poll
            .coordinate
            .belongs_to(self.operations.operation_task_owner_instance())
        {
            Some(ResidentProcessConsultationCompletionRehomeReason::TaskOwner)
        } else {
            match self.deployment_authority_index(&selector) {
                Err(reason) => {
                    Some(ResidentProcessConsultationCompletionRehomeReason::Deployment(reason))
                }
                Ok(index) => {
                    let deployment_store = self.deployments[index].store();
                    let expected_frontier = poll.coordinate.collection_frontier();
                    if poll.coordinate.store() != deployment_store {
                        Some(ResidentProcessConsultationCompletionRehomeReason::PollStore)
                    } else if current.store() != deployment_store {
                        Some(ResidentProcessConsultationCompletionRehomeReason::CollectionStore)
                    } else if current.collection().parent() != poll.coordinate.parent() {
                        Some(ResidentProcessConsultationCompletionRehomeReason::Parent)
                    } else {
                        match current.frontier_reference() {
                            Ok(current_frontier) if &current_frontier == expected_frontier => None,
                            Ok(_) | Err(_) => Some(
                                ResidentProcessConsultationCompletionRehomeReason::CollectionFrontier,
                            ),
                        }
                    }
                }
            }
        };

        if let Some(reason) = reason {
            return Err(Box::new(
                ResidentProcessConsultationCompletionRehomeFailure {
                    selector,
                    current,
                    poll,
                    reason,
                },
            ));
        }

        Ok(ResidentProcessConsultationCompletionReady {
            selector,
            ready: Box::new(ResidentConsultationCompletionReady::from_parts(
                current, poll,
            )),
        })
    }
}
