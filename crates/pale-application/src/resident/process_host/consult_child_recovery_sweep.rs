//! One-source cold recovery for children retained by a Consult roster.
//!
//! One turn protects one child start, proves its immediate Consult parent,
//! re-admits the parent's retained roster, and classifies the child without
//! dispatching or polling it. A relevant source keeps this sweep's cursor until
//! an exact owner accepts the handoff or terminal parent custody is retained.

use super::*;
use crate::resident::consultation::{
    coordinator::ResidentConsultationCoordinator, ConsultationChildJoinFailure,
    ConsultationChildTerminalRetention, ConsultationCollectionCommitFailure,
    ConsultationContributionSource,
};
use crate::resident::local_operation_start::{
    discover_local_operation_start, LocalOperationStartDiscoveryError,
    LocalOperationStartDiscoveryTurn,
};
use crate::resident::process_host::operation_recovery_sweep::{
    admit_discovered_start, ResidentProcessOperationRecoverySourceError,
};
use crate::resident::process_host::restart::{
    ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError,
    ResidentProcessRecoveredAttempt, ResidentProcessRecoveredConsultChildLineage,
    ResidentProcessRecoveredLocalCancellation, ResidentProcessRecoveredOperationResolution,
    ResidentProcessRecoveredOperationStart, ResidentProcessRecoveredPreparedAttempt,
    ResidentProcessRecoveredProviderResolution,
};
use crate::resident::RecoveredOperationControl;

/// A source owned by another existing recovery sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResidentProcessConsultChildOtherOwner {
    /// An unparented operation belongs to a root recovery sweep.
    Root,
    /// A parented Delegate belongs to the pairwise Delegate sweep.
    PairwiseDelegate,
}

/// One retained child phase with no owner authorized by this stage.
#[must_use = "preserve the exact child phase until its owner accepts it"]
pub(super) enum ResidentProcessConsultChildPending {
    RecoveryOnly(Box<ResidentProcessRecoveredOperationStart>),
    LocalCancellation(Box<ResidentProcessRecoveredLocalCancellation>),
    OperationResolution(Box<ResidentProcessRecoveredOperationResolution>),
    NativePrepared(Box<ResidentProcessRecoveredPreparedAttempt>),
    ProviderPrepared(Box<ResidentProcessRecoveredPreparedAttempt>),
    ProviderResolved(Box<ResidentProcessRecoveredProviderResolution>),
    Native(Box<ResidentProcessRecoveredAttempt>),
    Provider(Box<ResidentProcessRecoveredAttempt>),
    NestedConsult(Box<ResidentProcessRecoveredOperationStart>),
}

/// Classified child state bound to its exact current parent and roster.
#[must_use = "commit terminal custody or retain this exact recovery handoff"]
pub(super) enum ResidentProcessConsultChildRecoveryState {
    Terminal(Box<ResidentProcessRecoveredOperationStart>),
    Pending(Box<ResidentProcessConsultChildPending>),
}

/// Exact source, parent, roster, and child state held before cursor advancement.
#[must_use = "an exact owner must accept this handoff before the source can advance"]
pub(super) struct ResidentProcessConsultChildRecoveryHandoff {
    deployment_index: usize,
    expected: LocalOperationStartDiscoveryAttempt,
    successor: LocalOperationStartDiscoveryAttempt,
    selector: ResidentDeploymentAuthoritySelector,
    parent: Box<RecoveredOperationControl>,
    current: RecoveredConsultationCollection,
    state: ResidentProcessConsultChildRecoveryState,
}

/// Cursor capability retained while an admitted pending child phase runs.
///
/// The capability contains no phase outcome. It permits advancement only after
/// the existing phase owner reports a durable completion or handoff.
#[must_use = "advance only after the admitted pending child owner succeeds"]
pub(super) struct ResidentProcessConsultChildPendingCursor {
    deployment_index: usize,
    expected: LocalOperationStartDiscoveryAttempt,
    successor: LocalOperationStartDiscoveryAttempt,
    selector: ResidentDeploymentAuthoritySelector,
}

impl ResidentProcessConsultChildRecoveryHandoff {
    pub(super) const fn parent(&self) -> &RecoveredOperationControl {
        &self.parent
    }

    pub(super) const fn current(&self) -> &RecoveredConsultationCollection {
        &self.current
    }

    pub(super) const fn state(&self) -> &ResidentProcessConsultChildRecoveryState {
        &self.state
    }

    /// Borrow the exact recovered nested-Consult start without transferring the
    /// source cursor or handoff custody.
    pub(super) fn nested_consult_start(&self) -> Option<&ResidentProcessRecoveredOperationStart> {
        let ResidentProcessConsultChildRecoveryState::Pending(pending) = &self.state else {
            return None;
        };
        let ResidentProcessConsultChildPending::NestedConsult(start) = pending.as_ref() else {
            return None;
        };
        Some(start)
    }

    /// Separate one non-Consult pending phase from its still-fixed cursor.
    pub(super) fn into_pending_disposition(
        self: Box<Self>,
    ) -> Result<
        (
            ResidentProcessConsultChildPendingCursor,
            ResidentProcessOperationRestartDisposition,
        ),
        Box<Self>,
    > {
        let Self {
            deployment_index,
            expected,
            successor,
            selector,
            parent,
            current,
            state,
        } = *self;
        let ResidentProcessConsultChildRecoveryState::Pending(pending) = state else {
            return Err(Box::new(Self {
                deployment_index,
                expected,
                successor,
                selector,
                parent,
                current,
                state,
            }));
        };
        let disposition = match *pending {
            ResidentProcessConsultChildPending::RecoveryOnly(start) => {
                ResidentProcessOperationRestartDisposition::RecoveryOnly(start)
            }
            ResidentProcessConsultChildPending::LocalCancellation(recovered) => {
                ResidentProcessOperationRestartDisposition::LocalCancellationRequested(recovered)
            }
            ResidentProcessConsultChildPending::OperationResolution(recovered) => {
                ResidentProcessOperationRestartDisposition::OperationResolution(recovered)
            }
            ResidentProcessConsultChildPending::NativePrepared(recovered) => {
                ResidentProcessOperationRestartDisposition::NativePrepared(recovered)
            }
            ResidentProcessConsultChildPending::ProviderPrepared(recovered) => {
                ResidentProcessOperationRestartDisposition::ProviderPrepared(recovered)
            }
            ResidentProcessConsultChildPending::ProviderResolved(recovered) => {
                ResidentProcessOperationRestartDisposition::ProviderResolved(recovered)
            }
            ResidentProcessConsultChildPending::Native(recovered) => {
                ResidentProcessOperationRestartDisposition::Native(recovered)
            }
            ResidentProcessConsultChildPending::Provider(recovered) => {
                ResidentProcessOperationRestartDisposition::Provider(recovered)
            }
            ResidentProcessConsultChildPending::NestedConsult(start) => {
                return Err(Box::new(Self {
                    deployment_index,
                    expected,
                    successor,
                    selector,
                    parent,
                    current,
                    state: ResidentProcessConsultChildRecoveryState::Pending(Box::new(
                        ResidentProcessConsultChildPending::NestedConsult(start),
                    )),
                }));
            }
        };
        Ok((
            ResidentProcessConsultChildPendingCursor {
                deployment_index,
                expected,
                successor,
                selector,
            },
            disposition,
        ))
    }

    /// Separate a nested Consult phase from its still-fixed child cursor.
    pub(super) fn into_nested_consult(
        self: Box<Self>,
    ) -> Result<
        (
            ResidentProcessConsultChildPendingCursor,
            Box<ResidentProcessRecoveredOperationStart>,
        ),
        Box<Self>,
    > {
        let Self {
            deployment_index,
            expected,
            successor,
            selector,
            parent,
            current,
            state,
        } = *self;
        let ResidentProcessConsultChildRecoveryState::Pending(pending) = state else {
            return Err(Box::new(Self {
                deployment_index,
                expected,
                successor,
                selector,
                parent,
                current,
                state,
            }));
        };
        let ResidentProcessConsultChildPending::NestedConsult(start) = *pending else {
            return Err(Box::new(Self {
                deployment_index,
                expected,
                successor,
                selector,
                parent,
                current,
                state: ResidentProcessConsultChildRecoveryState::Pending(pending),
            }));
        };
        Ok((
            ResidentProcessConsultChildPendingCursor {
                deployment_index,
                expected,
                successor,
                selector,
            },
            start,
        ))
    }
}

/// Fresh identities and current authorities for one cold terminal parent join.
pub(super) struct ResidentProcessConsultChildTerminalCommitInput<'a> {
    pub(super) association: ResidentOperationControlAssociation<'a>,
    pub(super) recorded_at: UnixTimeV1,
    pub(super) observation_ref: ResidentRecordRefV1,
    pub(super) observed_at: UnixTimeV1,
    pub(super) signing: pale_core::records::SignedResidentAdmissionContextV1<'a>,
    pub(super) limits: ConsultationCollectionLimits,
    pub(super) checkpoint: ConsultationSuccessorCommitInput<'a>,
}

/// Exact database/anchor continuation with its source cursor still fixed.
#[must_use = "continue this exact custody candidate before advancing the source"]
pub(super) struct ResidentProcessConsultChildTerminalCustodyPending {
    handoff: Box<ResidentProcessConsultChildRecoveryHandoff>,
    failure: ConsultationCollectionCommitFailure,
}

/// Completed custody or one exact retained physical continuation.
#[must_use = "preserve completed custody or its exact continuation"]
pub(super) enum ResidentProcessConsultChildTerminalCommitTurn {
    Completed(Box<CompletedConsultationCollection>),
    CustodyPending(Box<ResidentProcessConsultChildTerminalCustodyPending>),
}

/// Rejection before terminal custody had a safe successor owner.
pub(super) enum ResidentProcessConsultChildTerminalCommitError<'a> {
    Selection {
        handoff: Box<ResidentProcessConsultChildRecoveryHandoff>,
        input: Box<ResidentProcessConsultChildTerminalCommitInput<'a>>,
        reason: ResidentProcessHostSelectionError,
    },
    State {
        handoff: Box<ResidentProcessConsultChildRecoveryHandoff>,
        input: Box<ResidentProcessConsultChildTerminalCommitInput<'a>>,
    },
    Admission {
        handoff: Box<ResidentProcessConsultChildRecoveryHandoff>,
        reason: ConsultationCollectionError,
    },
    Preparation {
        handoff: Box<ResidentProcessConsultChildRecoveryHandoff>,
        reason: ConsultationCollectionError,
    },
    Issue {
        handoff: Box<ResidentProcessConsultChildRecoveryHandoff>,
        reason: ConsultationCollectionError,
    },
}

impl ResidentProcessConsultChildTerminalCommitError<'_> {
    /// Recover the exact child handoff after a pre-custody refusal.
    pub(super) fn into_handoff(self) -> Box<ResidentProcessConsultChildRecoveryHandoff> {
        match self {
            Self::Selection { handoff, .. }
            | Self::State { handoff, .. }
            | Self::Admission { handoff, .. }
            | Self::Preparation { handoff, .. }
            | Self::Issue { handoff, .. } => handoff,
        }
    }
}

/// One bounded Consult-child recovery turn.
#[must_use = "preserve any returned child recovery handoff"]
pub(super) enum ResidentProcessConsultChildRecoveryTurn {
    Exhausted,
    SkippedOtherOwner(ResidentProcessConsultChildOtherOwner),
    AlreadyRetained(Box<RecoveredConsultationCollection>),
    Recovered(Box<ResidentProcessConsultChildRecoveryHandoff>),
}

impl ResidentProcessConsultChildRecoveryTurn {
    /// Return whether this exact handoff must create a new keyed Consult turn.
    ///
    /// Terminal evidence and phases owned by another recovery path do not use
    /// keyed Consult capacity. Only a recovered nested Consult needs that
    /// reservation before its source cursor can advance.
    pub(super) fn requires_keyed_consult_turn(&self) -> bool {
        matches!(
            self,
            Self::Recovered(handoff)
                if matches!(
                    handoff.state(),
                    ResidentProcessConsultChildRecoveryState::Pending(pending)
                        if matches!(
                            pending.as_ref(),
                            ResidentProcessConsultChildPending::NestedConsult(_)
                        )
                )
        )
    }
}

/// Why one source could not reach a safe owner boundary.
pub(super) enum ResidentProcessConsultChildRecoveryError {
    Discovery(LocalOperationStartDiscoveryError),
    Source(ResidentProcessOperationRecoverySourceError),
    Restart(ResidentProcessOperationRestartError),
    ParentCollection(ResidentProcessConsultationCollectionRecoveryError),
    ParentCollectionAbsent,
    ParentIntent,
    Roster,
    TerminalRetention(ConsultationCollectionError),
    HostileA2a,
}

impl std::fmt::Debug for ResidentProcessConsultChildRecoveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Discovery(_) => "Discovery",
            Self::Source(_) => "Source",
            Self::Restart(_) => "Restart",
            Self::ParentCollection(_) => "ParentCollection",
            Self::ParentCollectionAbsent => "ParentCollectionAbsent",
            Self::ParentIntent => "ParentIntent",
            Self::Roster => "Roster",
            Self::TerminalRetention(_) => "TerminalRetention",
            Self::HostileA2a => "HostileA2a",
        };
        formatter
            .debug_struct("ResidentProcessConsultChildRecoveryError")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Advance a Consult-child source only after its phase owner succeeded.
    pub(super) fn complete_recovered_consult_child_pending(
        &mut self,
        access_store: &AuthorityStoreCoordinate,
        cursor: ResidentProcessConsultChildPendingCursor,
    ) -> Result<(), ()> {
        if self.deployment_authority_index(&cursor.selector).ok() != Some(cursor.deployment_index)
            || self
                .deployments
                .get(cursor.deployment_index)
                .map(ReadyResidentDeployment::store)
                != Some(access_store)
            || self
                .consult_child_recovery_attempts
                .get(cursor.deployment_index)
                != Some(&cursor.expected)
        {
            return Err(());
        }
        self.consult_child_recovery_attempts[cursor.deployment_index] = cursor.successor;
        Ok(())
    }

    /// Recover and classify at most one committed Consult child.
    ///
    /// Relevant child sources keep the cursor unchanged. Root and pairwise
    /// Delegate sources advance to their already-established recovery owners.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn recover_one_consult_child_start<Authority, AdmitCollection>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        restart: ResidentProcessOperationRestartInput<'_>,
        mut admission: OperationControlRecoveryAdmission<'_>,
        maximum_session_basis_bytes: usize,
        authority: &mut Authority,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        admit_collection: AdmitCollection,
    ) -> Result<ResidentProcessConsultChildRecoveryTurn, ResidentProcessConsultChildRecoveryError>
    where
        AdmitCollection: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
    {
        let index = self
            .deployment_authority_index(selector)
            .map_err(|reason| {
                ResidentProcessConsultChildRecoveryError::Restart(
                    ResidentProcessOperationRestartError::Selection(reason),
                )
            })?;
        if self.deployments[index].store() != access.store {
            return Err(ResidentProcessConsultChildRecoveryError::Restart(
                ResidentProcessHostSelectionError::AccessStoreAssociation.into(),
            ));
        }
        let attempt = self.consult_child_recovery_attempts[index].clone();
        let maximum_record_bytes = restart.operation.maximum_record_bytes;
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let discovered = discover_local_operation_start(
            attempt.clone(),
            &mut *custody,
            maximum_session_basis_bytes,
        )
        .map_err(ResidentProcessConsultChildRecoveryError::Discovery)?;
        let pending = match discovered {
            LocalOperationStartDiscoveryTurn::Exhausted(exhausted) => {
                self.consult_child_recovery_attempts[index] = exhausted;
                return Ok(ResidentProcessConsultChildRecoveryTurn::Exhausted);
            }
            LocalOperationStartDiscoveryTurn::Pending(pending) => pending,
        };
        let (request, evidence) = admit_discovered_start(
            store,
            pending.source(),
            &mut *custody,
            maximum_record_bytes,
            &mut *admission.admit_request,
        )
        .map_err(ResidentProcessConsultChildRecoveryError::Source)?;
        let request_preimage = request.to_preimage();
        let association = ResidentOperationControlAssociation {
            current_request_authority: &request_preimage.signing_authority_binding,
            historical_handle_authority: restart.operation.handle_signing_authority,
            recovery_store: restart.operation.store,
            checkpoint_authority: restart.operation.checkpoint_authority,
        };
        let protected = self
            .protect_existing_operation_start(
                ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &restart,
                &mut admission,
            )
            .map_err(ResidentProcessConsultChildRecoveryError::Restart)?;
        match (
            protected.parent_operation().is_some(),
            protected.operation_class(),
        ) {
            (false, _) => {
                self.consult_child_recovery_attempts[index] = pending.advance();
                return Ok(ResidentProcessConsultChildRecoveryTurn::SkippedOtherOwner(
                    ResidentProcessConsultChildOtherOwner::Root,
                ));
            }
            (true, OperationClassV1::Delegate) => {
                self.consult_child_recovery_attempts[index] = pending.advance();
                return Ok(ResidentProcessConsultChildRecoveryTurn::SkippedOtherOwner(
                    ResidentProcessConsultChildOtherOwner::PairwiseDelegate,
                ));
            }
            (true, OperationClassV1::Invoke | OperationClassV1::Consult) => {}
        }
        let (protected, parent_control) = self
            .recover_current_parent_for_protected_child(
                protected,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &restart,
                &mut admission,
            )
            .map_err(ResidentProcessConsultChildRecoveryError::Restart)?;
        let parent_operation = parent_control.operation().operation();
        if parent_operation.handle().1.operation_class() != &OperationClassV1::Consult {
            return Err(ResidentProcessConsultChildRecoveryError::Roster);
        }
        let successor = pending.advance();
        let intent = parent_operation
            .consultation_start_intent()
            .ok_or(ResidentProcessConsultChildRecoveryError::ParentIntent)?;
        let collection_limits = intent
            .validate_parent(store, parent_operation)
            .map_err(|_| ResidentProcessConsultChildRecoveryError::ParentIntent)?;
        let current = self
            .recover_consultation_collection(
                selector,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                authority,
                parent_operation,
                parent_control.observation(),
                collection_limits,
                resolve_store,
                admit_collection,
            )
            .map_err(ResidentProcessConsultChildRecoveryError::ParentCollection)?
            .ok_or(ResidentProcessConsultChildRecoveryError::ParentCollectionAbsent)?;
        if !intent.matches_collection(current.collection()) {
            return Err(ResidentProcessConsultChildRecoveryError::ParentIntent);
        }
        let child_coordinate = immutable_operation_coordinate(protected.operation())
            .map_err(ResidentProcessConsultChildRecoveryError::Restart)?;
        let mut matching = current.collection().slots().iter().filter(|slot| {
            matches!(
                slot.roster().source(),
                ConsultationContributionSource::FreshChild { child }
                    if child == &child_coordinate
            )
        });
        let slot = matching
            .next()
            .ok_or(ResidentProcessConsultChildRecoveryError::Roster)?;
        if matching.next().is_some() {
            return Err(ResidentProcessConsultChildRecoveryError::Roster);
        }
        let disposition = self
            .classify_protected_recovered_consult_child_operation(
                protected,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                restart,
                &mut admission,
                ResidentProcessRecoveredConsultChildLineage {
                    parent: &parent_control,
                    collection: &current,
                    slot: slot.roster(),
                },
            )
            .map_err(ResidentProcessConsultChildRecoveryError::Restart)?;
        let parent = Box::new(parent_control);
        let state = classify_recovered_consult_child_state(disposition)?;
        if let ResidentProcessConsultChildRecoveryState::Terminal(child) = &state {
            match current
                .child_terminal_retention(slot.roster().slot_id(), child.control.observation())
                .map_err(ResidentProcessConsultChildRecoveryError::TerminalRetention)?
            {
                ConsultationChildTerminalRetention::Absent => {}
                ConsultationChildTerminalRetention::Exact => {
                    self.consult_child_recovery_attempts[index] = successor;
                    return Ok(ResidentProcessConsultChildRecoveryTurn::AlreadyRetained(
                        Box::new(current),
                    ));
                }
            }
        }
        Ok(ResidentProcessConsultChildRecoveryTurn::Recovered(
            Box::new(ResidentProcessConsultChildRecoveryHandoff {
                deployment_index: index,
                expected: attempt,
                successor,
                selector: selector.clone(),
                parent,
                current,
                state,
            }),
        ))
    }

    /// Join one cold-recovered terminal child directly into its current parent.
    ///
    /// Admission failure preserves the child handoff and leaves the cursor on
    /// the same source. A database or anchor interruption also leaves the cursor
    /// fixed and returns the exact physical continuation. Only complete database
    /// and independent-anchor custody advances this sweep.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn commit_recovered_consult_child_terminal<
        'operation,
        AdmitEvidence,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        handoff: Box<ResidentProcessConsultChildRecoveryHandoff>,
        input: ResidentProcessConsultChildTerminalCommitInput<'operation>,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        admit_evidence: AdmitEvidence,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessConsultChildTerminalCommitTurn,
        ResidentProcessConsultChildTerminalCommitError<'operation>,
    >
    where
        AdmitEvidence: FnOnce(
            &crate::resident::consultation::ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
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
        let selection =
            self.validate_cold_consult_child_terminal_commit(&handoff, access.store, &input);
        if let Err(reason) = selection {
            return Err(ResidentProcessConsultChildTerminalCommitError::Selection {
                handoff,
                input: Box::new(input),
                reason,
            });
        }
        let ResidentProcessConsultChildRecoveryState::Terminal(child) = &handoff.state else {
            return Err(ResidentProcessConsultChildTerminalCommitError::State {
                handoff,
                input: Box::new(input),
            });
        };
        let lineage = child
            .lineage
            .consult_child()
            .expect("cold Consult-child terminal retained admitted child lineage");
        let parent_operation = handoff.parent.operation().operation();
        let parent_observation = handoff.parent.observation();
        let parent_head = parent_observation.selected_head();
        let child_observation = child.control.observation();
        let ResidentProcessConsultChildTerminalCommitInput {
            association: _,
            recorded_at,
            observation_ref,
            observed_at,
            signing,
            limits,
            checkpoint,
        } = input;
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let committed = ResidentConsultationCoordinator::new(custody, anchor).join_child(
            &handoff.current,
            ConsultationChildJoinInput {
                slot_id: lineage.slot_id(),
                child: child_observation,
                recorded_at,
                parent: parent_operation,
                parent_observation: parent_head,
                observation_ref,
                observed_at,
                signing,
                limits,
            },
            checkpoint,
            resolve_store,
            admit_evidence,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        );
        match committed {
            Ok(completed) => {
                self.consult_child_recovery_attempts[handoff.deployment_index] =
                    handoff.successor.clone();
                Ok(ResidentProcessConsultChildTerminalCommitTurn::Completed(
                    Box::new(completed),
                ))
            }
            Err(ConsultationChildJoinFailure::Admission(reason)) => {
                Err(ResidentProcessConsultChildTerminalCommitError::Admission { handoff, reason })
            }
            Err(ConsultationChildJoinFailure::Custody(failure)) => match failure {
                ConsultationCollectionCommitFailure::Preparation { reason, .. } => Err(
                    ResidentProcessConsultChildTerminalCommitError::Preparation { handoff, reason },
                ),
                ConsultationCollectionCommitFailure::Issue { reason, .. } => {
                    Err(ResidentProcessConsultChildTerminalCommitError::Issue { handoff, reason })
                }
                failure @ (ConsultationCollectionCommitFailure::Commit { .. }
                | ConsultationCollectionCommitFailure::Anchor { .. }) => Ok(
                    ResidentProcessConsultChildTerminalCommitTurn::CustodyPending(Box::new(
                        ResidentProcessConsultChildTerminalCustodyPending { handoff, failure },
                    )),
                ),
            },
        }
    }

    /// Continue only retained database or anchor custody from a terminal join.
    ///
    /// The cursor advances only after both custody layers complete. Any further
    /// interruption returns the same exact capability with the cursor unchanged.
    pub(super) fn continue_recovered_consult_child_terminal_custody(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        pending: Box<ResidentProcessConsultChildTerminalCustodyPending>,
    ) -> Result<
        CompletedConsultationCollection,
        Box<ResidentProcessConsultChildTerminalCustodyPending>,
    > {
        let index = pending.handoff.deployment_index;
        if self
            .deployments
            .get(index)
            .map(ReadyResidentDeployment::store)
            != Some(access.store)
            || self.consult_child_recovery_attempts.get(index) != Some(&pending.handoff.expected)
        {
            return Err(pending);
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        match pending.failure.continue_custody(custody, anchor) {
            Ok(completed) => {
                self.consult_child_recovery_attempts[index] = pending.handoff.successor.clone();
                Ok(completed)
            }
            Err(failure) => Err(Box::new(
                ResidentProcessConsultChildTerminalCustodyPending {
                    handoff: pending.handoff,
                    failure,
                },
            )),
        }
    }

    fn validate_cold_consult_child_terminal_commit(
        &self,
        handoff: &ResidentProcessConsultChildRecoveryHandoff,
        access_store: &AuthorityStoreCoordinate,
        input: &ResidentProcessConsultChildTerminalCommitInput<'_>,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(&handoff.selector)?;
        let selected_store = self.deployments[index].store();
        if index != handoff.deployment_index
            || selected_store != access_store
            || handoff.current.store() != selected_store
            || self.consult_child_recovery_attempts.get(index) != Some(&handoff.expected)
        {
            return Err(ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation);
        }
        if !handoff
            .selector
            .matches_signing_authority(input.association.current_request_authority)
        {
            return Err(ResidentProcessHostSelectionError::CurrentRequestAuthority);
        }
        if !handoff
            .selector
            .matches_signing_authority(input.association.historical_handle_authority)
        {
            return Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority);
        }
        if input.association.recovery_store != selected_store {
            return Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation);
        }
        if !checkpoint_authority_matches(
            &handoff.selector,
            selected_store,
            input.association.checkpoint_authority,
        ) || !checkpoint_authority_matches(
            &handoff.selector,
            selected_store,
            input.checkpoint.authority,
        ) {
            return Err(ResidentProcessHostSelectionError::CheckpointAuthorityAssociation);
        }
        if !handoff
            .selector
            .matches_operation(handoff.parent.operation().operation())
            || handoff.parent.operation().store() != selected_store
            || handoff.parent.observation().store() != selected_store
        {
            return Err(ResidentProcessHostSelectionError::ConsultationParentAuthority);
        }
        Ok(())
    }
}

fn immutable_operation_coordinate(
    operation: &crate::resident::IssuedLocalOperation,
) -> Result<ImmutableRecordReferenceV1, ResidentProcessOperationRestartError> {
    let (reference, handle) = operation.handle();
    Ok(ImmutableRecordReferenceV1 {
        record_ref: reference.clone(),
        record_sha256: ArtifactDigest::parse(handle.canonical_operation_handle_sha256().as_str())
            .map_err(|_| ResidentProcessOperationRestartError::State)?,
    })
}

fn classify_recovered_consult_child_state(
    disposition: ResidentProcessOperationRestartDisposition,
) -> Result<ResidentProcessConsultChildRecoveryState, ResidentProcessConsultChildRecoveryError> {
    let pending = match disposition {
        ResidentProcessOperationRestartDisposition::NativeHookResponse(_) => {
            return Err(ResidentProcessConsultChildRecoveryError::Restart(
                ResidentProcessOperationRestartError::State,
            ));
        }
        ResidentProcessOperationRestartDisposition::Terminal(start) => {
            return Ok(ResidentProcessConsultChildRecoveryState::Terminal(start));
        }
        ResidentProcessOperationRestartDisposition::RecoveryOnly(start) => {
            ResidentProcessConsultChildPending::RecoveryOnly(start)
        }
        ResidentProcessOperationRestartDisposition::LocalCancellationRequested(recovered) => {
            ResidentProcessConsultChildPending::LocalCancellation(recovered)
        }
        ResidentProcessOperationRestartDisposition::OperationResolution(recovered) => {
            ResidentProcessConsultChildPending::OperationResolution(recovered)
        }
        ResidentProcessOperationRestartDisposition::NativePrepared(recovered) => {
            ResidentProcessConsultChildPending::NativePrepared(recovered)
        }
        ResidentProcessOperationRestartDisposition::ProviderPrepared(recovered) => {
            ResidentProcessConsultChildPending::ProviderPrepared(recovered)
        }
        ResidentProcessOperationRestartDisposition::ProviderResolved(recovered) => {
            ResidentProcessConsultChildPending::ProviderResolved(recovered)
        }
        ResidentProcessOperationRestartDisposition::Consult(start) => {
            ResidentProcessConsultChildPending::NestedConsult(start)
        }
        ResidentProcessOperationRestartDisposition::Native(recovered) => {
            ResidentProcessConsultChildPending::Native(recovered)
        }
        ResidentProcessOperationRestartDisposition::Provider(recovered) => {
            ResidentProcessConsultChildPending::Provider(recovered)
        }
        ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(_)
        | ResidentProcessOperationRestartDisposition::A2aTask(_)
        | ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(_) => {
            return Err(ResidentProcessConsultChildRecoveryError::HostileA2a);
        }
    };
    Ok(ResidentProcessConsultChildRecoveryState::Pending(Box::new(
        pending,
    )))
}
