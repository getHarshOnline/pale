//! Commit one prepared A2A send disposition through existing custody owners.

use super::*;
use crate::resident::{
    operation_dispatch::observation_checkpoint_preimage, CompletedOperationObservationCustody,
    OperationCheckpointInput, OperationCheckpointSelection,
    OperationObservationCustodyAnchorFailure, OperationObservationCustodyCommitFailure,
    PreparedOperationObservationCheckpoint,
};
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchorPort, OperationCustodyPort, WallClockSourcePort,
};

/// Exact checkpoint inputs for one prepared send disposition.
pub struct A2aSendObservationCommitInput<'a> {
    /// Fresh identity for the checkpoint that retains the disposition.
    pub checkpoint: OperationCheckpointInput,
    /// Selected checkpoint-time source. Retained custody supplies its lower bound.
    pub clock: &'a dyn WallClockSourcePort,
    /// Exact checkpoint schema selected independently of the response.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Current checkpoint-signing authority.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Finite accumulator preparation ceilings.
    pub accumulator_limits: AccumulatorPreparationLimits,
}

/// Durable failed or outcome-unknown send observation and physical custody proof.
///
/// The response remains protocol evidence. This value does not create a remote
/// Task, permit send replay, or turn remote state into a Pale result.
#[must_use = "retain the A2A send disposition or continue through its recovery owner"]
pub struct CompletedA2aSendObservationCustody {
    observation: (ResidentRecordRefV1, OperationObservationV1),
    diagnostic: crate::diagnostic::AdmittedDiagnostic,
    response: Option<pale_ports::ControlRecordCoordinate>,
    checkpoint_preimage: ControlStateCheckpointPreimageV1,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: CompletedOperationObservationCustody,
}

pub(in crate::resident) struct A2aSendObservationFinalizer {
    observation: (ResidentRecordRefV1, OperationObservationV1),
    diagnostic: crate::diagnostic::AdmittedDiagnostic,
    response: Option<pale_ports::ControlRecordCoordinate>,
    checkpoint_preimage: ControlStateCheckpointPreimageV1,
}

impl A2aSendObservationFinalizer {
    fn finish(
        self,
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
        custody: CompletedOperationObservationCustody,
    ) -> CompletedA2aSendObservationCustody {
        CompletedA2aSendObservationCustody {
            observation: self.observation,
            diagnostic: self.diagnostic,
            response: self.response,
            checkpoint_preimage: self.checkpoint_preimage,
            checkpoint,
            custody,
        }
    }
}

impl CompletedA2aSendObservationCustody {
    /// Borrow the exact failed or outcome-unknown operation observation.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation.0, &self.observation.1)
    }

    /// Borrow the admitted diagnostic retained with the observation.
    #[must_use]
    pub const fn diagnostic(&self) -> &crate::diagnostic::AdmittedDiagnostic {
        &self.diagnostic
    }

    /// Borrow the retained first-response coordinate, when one was received.
    #[must_use]
    pub const fn response(&self) -> Option<&pale_ports::ControlRecordCoordinate> {
        self.response.as_ref()
    }

    /// Borrow the checkpoint fields admitted before signing.
    #[must_use]
    pub const fn checkpoint_preimage(&self) -> &ControlStateCheckpointPreimageV1 {
        &self.checkpoint_preimage
    }

    /// Borrow the checkpoint that retained the complete disposition atom.
    #[must_use]
    pub const fn checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    /// Borrow physical database and independent-anchor custody evidence.
    #[must_use]
    pub const fn custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }
}

/// Stopped response custody, separated by the last completed phase.
pub enum A2aSendObservationCommitFailure<Observation = PreparedA2aSendObservation> {
    /// Accumulator preparation failed before checkpoint issuance.
    Preparation {
        /// Exact prepared disposition; no database commitment was attempted.
        observation: Box<Observation>,
        /// Closed preparation failure.
        reason: OperationObservationCustodyProjectionError,
    },
    /// Checkpoint construction failed before signing or commitment.
    Checkpoint {
        /// Exact prepared disposition; no database commitment was attempted.
        observation: Box<Observation>,
        /// Closed semantic failure.
        reason: OperationAttemptError,
    },
    /// Checkpoint admission, signing, or physical projection failed.
    Issue {
        /// Exact prepared disposition; no database commitment was attempted.
        observation: Box<Observation>,
        /// Closed issuance failure.
        reason: OperationObservationCustodyProjectionError,
    },
    /// Database commitment failed or its outcome requires exact recovery.
    Commit {
        /// Issued checkpoint, not proof of physical custody.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        /// Existing commit failure owns the candidate and retry law.
        failure: OperationObservationCustodyCommitFailure,
    },
    /// Database custody exists but the independent anchor is not established.
    Anchor {
        /// Issued checkpoint whose database candidate is retained by the failure.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        /// Existing anchor failure owns the pending phase and recovery law.
        failure: OperationObservationCustodyAnchorFailure,
    },
}

impl<Observation> std::fmt::Debug for A2aSendObservationCommitFailure<Observation> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Preparation { .. } => "preparation",
            Self::Checkpoint { .. } => "checkpoint",
            Self::Issue { .. } => "issue",
            Self::Commit { .. } => "commit",
            Self::Anchor { .. } => "anchor",
        };
        formatter
            .debug_struct("A2aSendObservationCommitFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl<Observation> A2aSendObservationCommitFailure<Observation> {
    /// Recover the exact prepared disposition when no physical commit was attempted.
    #[must_use]
    pub fn into_prepared(self) -> Option<Observation> {
        match self {
            Self::Preparation { observation, .. }
            | Self::Checkpoint { observation, .. }
            | Self::Issue { observation, .. } => Some(*observation),
            Self::Commit { .. } | Self::Anchor { .. } => None,
        }
    }
}

pub(in crate::resident) enum ContinuableA2aSendObservationCommitFailure<Observation> {
    PrePhysical(A2aSendObservationCommitFailure<Observation>),
    Commit {
        finalizer: Box<A2aSendObservationFinalizer>,
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        failure: OperationObservationCustodyCommitFailure,
    },
    Anchor {
        finalizer: Box<A2aSendObservationFinalizer>,
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        failure: OperationObservationCustodyAnchorFailure,
    },
}

impl<Observation> ContinuableA2aSendObservationCommitFailure<Observation> {
    pub(in crate::resident) fn into_public(self) -> A2aSendObservationCommitFailure<Observation> {
        match self {
            Self::PrePhysical(failure) => failure,
            Self::Commit {
                checkpoint,
                failure,
                ..
            } => A2aSendObservationCommitFailure::Commit {
                checkpoint,
                failure,
            },
            Self::Anchor {
                checkpoint,
                failure,
                ..
            } => A2aSendObservationCommitFailure::Anchor {
                checkpoint,
                failure,
            },
        }
    }

    pub(in crate::resident) fn continue_custody(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedA2aSendObservationCustody, Self> {
        let (finalizer, checkpoint, pending) = match self {
            Self::PrePhysical(_) => return Err(self),
            Self::Commit {
                finalizer,
                checkpoint,
                failure,
            } => {
                let pending = match failure.reconcile_exact(custody) {
                    Ok(pending) => pending,
                    Err(failure) => match failure.try_into_retryable() {
                        Ok(prepared) => match prepared.commit(custody, anchor) {
                            Ok(pending) => pending,
                            Err(failure) => {
                                return Err(Self::Commit {
                                    finalizer,
                                    checkpoint,
                                    failure,
                                });
                            }
                        },
                        Err(failure) => {
                            return Err(Self::Commit {
                                finalizer,
                                checkpoint,
                                failure,
                            });
                        }
                    },
                };
                (finalizer, checkpoint, pending)
            }
            Self::Anchor {
                finalizer,
                checkpoint,
                failure,
            } => match failure.try_into_retryable() {
                Ok(pending) => (finalizer, checkpoint, pending),
                Err(failure) => match failure.continue_after_failed_observation(anchor) {
                    Ok(completed) => return Ok(finalizer.finish(*checkpoint, completed)),
                    Err(failure) => {
                        return Err(Self::Anchor {
                            finalizer,
                            checkpoint,
                            failure,
                        });
                    }
                },
            },
        };
        match pending.advance_anchor(anchor) {
            Ok(completed) => Ok(finalizer.finish(*checkpoint, completed)),
            Err(failure) => Err(Self::Anchor {
                finalizer,
                checkpoint,
                failure,
            }),
        }
    }
}

/// Retain one prepared task-less, failed, or outcome-unknown send disposition.
///
/// This service prepares accumulator roots, issues the exact successor
/// checkpoint, commits the response evidence and diagnostic with the operation
/// observation, and advances the independent anchor. It performs no carrier
/// call, response replay, Task creation, reconciliation, or settlement.
///
/// # Errors
/// Before database commitment, the failure returns the exact prepared
/// disposition. Physical failures retain their existing database or anchor
/// recovery owner and never restore send permission.
#[allow(clippy::too_many_arguments)]
pub fn commit_a2a_send_observation<CheckpointSign>(
    observation: PreparedA2aSendObservation,
    input: A2aSendObservationCommitInput<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    sign_checkpoint: CheckpointSign,
) -> Result<CompletedA2aSendObservationCustody, A2aSendObservationCommitFailure>
where
    CheckpointSign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    commit_prepared_a2a_send_observation_continuable(
        observation,
        input,
        resolve_store,
        custody,
        anchor,
        sign_checkpoint,
    )
    .map_err(ContinuableA2aSendObservationCommitFailure::into_public)
}

pub(super) fn commit_a2a_send_observation_continuable<CheckpointSign>(
    observation: PreparedA2aSendObservation,
    input: A2aSendObservationCommitInput<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    sign_checkpoint: CheckpointSign,
) -> Result<
    CompletedA2aSendObservationCustody,
    ContinuableA2aSendObservationCommitFailure<PreparedA2aSendObservation>,
>
where
    CheckpointSign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    commit_prepared_a2a_send_observation_continuable(
        observation,
        input,
        resolve_store,
        custody,
        anchor,
        sign_checkpoint,
    )
}

pub(super) fn commit_recovered_a2a_send_observation_continuable<'recovery, CheckpointSign>(
    observation: PreparedRecoveredA2aSendObservation<'recovery>,
    input: A2aSendObservationCommitInput<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    sign_checkpoint: CheckpointSign,
) -> Result<
    CompletedA2aSendObservationCustody,
    ContinuableA2aSendObservationCommitFailure<PreparedRecoveredA2aSendObservation<'recovery>>,
>
where
    CheckpointSign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    commit_prepared_a2a_send_observation_continuable(
        observation,
        input,
        resolve_store,
        custody,
        anchor,
        sign_checkpoint,
    )
}

trait PreparedA2aSendDisposition {
    fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1);

    fn diagnostic(&self) -> &crate::diagnostic::AdmittedDiagnostic;

    fn response_coordinate(&self) -> Option<pale_ports::ControlRecordCoordinate>;

    fn predecessor_checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1);

    fn checkpoint_selection(&self) -> OperationCheckpointSelection;

    fn retain_until(&self) -> UnixTimeV1;

    fn prepare_checkpoint<'a>(
        &'a self,
        selection: OperationCheckpointSelection,
        limits: AccumulatorPreparationLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    >;
}

impl PreparedA2aSendDisposition for PreparedA2aSendObservation {
    fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.observation()
    }

    fn diagnostic(&self) -> &crate::diagnostic::AdmittedDiagnostic {
        self.diagnostic()
    }

    fn response_coordinate(&self) -> Option<pale_ports::ControlRecordCoordinate> {
        self.response_evidence()
            .map(|evidence| evidence.coordinate().clone())
    }

    fn predecessor_checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        self.predecessor_checkpoint()
    }

    fn checkpoint_selection(&self) -> OperationCheckpointSelection {
        self.checkpoint_selection()
    }

    fn retain_until(&self) -> UnixTimeV1 {
        self.retain_until()
    }

    fn prepare_checkpoint<'a>(
        &'a self,
        selection: OperationCheckpointSelection,
        limits: AccumulatorPreparationLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        self.prepare_checkpoint(selection, limits, resolve_store, custody)
    }
}

impl PreparedA2aSendDisposition for PreparedRecoveredA2aSendObservation<'_> {
    fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.observation()
    }

    fn diagnostic(&self) -> &crate::diagnostic::AdmittedDiagnostic {
        self.diagnostic()
    }

    fn response_coordinate(&self) -> Option<pale_ports::ControlRecordCoordinate> {
        None
    }

    fn predecessor_checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        self.predecessor_checkpoint()
    }

    fn checkpoint_selection(&self) -> OperationCheckpointSelection {
        self.checkpoint_selection()
    }

    fn retain_until(&self) -> UnixTimeV1 {
        self.retain_until()
    }

    fn prepare_checkpoint<'a>(
        &'a self,
        _selection: OperationCheckpointSelection,
        limits: AccumulatorPreparationLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        self.prepare_checkpoint(limits, resolve_store, custody)
    }
}

#[allow(clippy::too_many_arguments)]
fn commit_prepared_a2a_send_observation_continuable<Observation, CheckpointSign>(
    observation: Observation,
    input: A2aSendObservationCommitInput<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    sign_checkpoint: CheckpointSign,
) -> Result<
    CompletedA2aSendObservationCustody,
    ContinuableA2aSendObservationCommitFailure<Observation>,
>
where
    Observation: PreparedA2aSendDisposition,
    CheckpointSign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let predecessor = {
        let (reference, checkpoint) = observation.predecessor_checkpoint();
        (reference.clone(), checkpoint.clone())
    };
    let issuance_floor = std::cmp::max(
        *observation.observation().1.observed_at(),
        *predecessor.1.issued_at(),
    );
    let issued_at = match crate::resident::operation_dispatch::custody::sample_time(
        input.clock,
        issuance_floor,
    ) {
        Ok(issued_at) => issued_at,
        Err(reason) => {
            return Err(ContinuableA2aSendObservationCommitFailure::PrePhysical(
                A2aSendObservationCommitFailure::Checkpoint {
                    observation: Box::new(observation),
                    reason,
                },
            ));
        }
    };
    if issued_at > observation.retain_until() {
        return Err(ContinuableA2aSendObservationCommitFailure::PrePhysical(
            A2aSendObservationCommitFailure::Checkpoint {
                observation: Box::new(observation),
                reason: OperationAttemptError::RetentionElapsed,
            },
        ));
    }
    let selection = observation.checkpoint_selection();
    let prepared = match observation.prepare_checkpoint(
        selection,
        input.accumulator_limits,
        resolve_store,
        custody,
    ) {
        Ok(prepared) => prepared,
        Err(reason) => {
            return Err(ContinuableA2aSendObservationCommitFailure::PrePhysical(
                A2aSendObservationCommitFailure::Preparation {
                    observation: Box::new(observation),
                    reason,
                },
            ));
        }
    };
    let preimage = match observation_checkpoint_preimage(
        &prepared,
        (&predecessor.0, &predecessor.1),
        &input.checkpoint,
        issued_at,
        input.checkpoint_schema,
        input.checkpoint_authority,
    ) {
        Ok(preimage) => preimage,
        Err(reason) => {
            drop(prepared);
            return Err(ContinuableA2aSendObservationCommitFailure::PrePhysical(
                A2aSendObservationCommitFailure::Checkpoint {
                    observation: Box::new(observation),
                    reason,
                },
            ));
        }
    };
    let checkpoint_ref = input.checkpoint.reference;
    let checkpoint_preimage = preimage.clone();
    let issued = prepared.issue_checkpoint(
        &checkpoint_ref,
        preimage,
        input.checkpoint_schema,
        input.checkpoint_authority,
        sign_checkpoint,
    );
    let (physical, checkpoint_value) = match issued {
        Ok(value) => value,
        Err(reason) => {
            return Err(ContinuableA2aSendObservationCommitFailure::PrePhysical(
                A2aSendObservationCommitFailure::Issue {
                    observation: Box::new(observation),
                    reason,
                },
            ));
        }
    };
    let retained_observation = (
        observation.observation().0.clone(),
        observation.observation().1.clone(),
    );
    let diagnostic = observation.diagnostic().clone();
    let response = observation.response_coordinate();
    let checkpoint = (checkpoint_ref, checkpoint_value);
    let finalizer = Box::new(A2aSendObservationFinalizer {
        observation: retained_observation,
        diagnostic,
        response,
        checkpoint_preimage,
    });
    let pending = match physical.commit(custody, anchor) {
        Ok(pending) => pending,
        Err(failure) => {
            return Err(ContinuableA2aSendObservationCommitFailure::Commit {
                finalizer,
                checkpoint: Box::new(checkpoint),
                failure,
            });
        }
    };
    let custody = match pending.advance_anchor(anchor) {
        Ok(completed) => completed,
        Err(failure) => {
            return Err(ContinuableA2aSendObservationCommitFailure::Anchor {
                finalizer,
                checkpoint: Box::new(checkpoint),
                failure,
            });
        }
    };
    Ok(finalizer.finish(checkpoint, custody))
}
