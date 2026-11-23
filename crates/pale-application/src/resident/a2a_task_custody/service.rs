//! Commit the first identified remote Task through existing custody owners.

use super::*;
use crate::resident::{
    CompletedOperationObservationCustody, OperationCheckpointInput,
    OperationObservationCustodyAnchorFailure, OperationObservationCustodyCommitFailure,
};

/// Exact inputs for one initial Task binding and revision-one observation.
pub struct A2aInitialTaskCommitInput<'a> {
    /// Task binding, first observation, signing contexts, and evidence ceilings.
    pub task: super::super::operation_dispatch::A2aInitialTaskCustodyInput<'a>,
    /// Fresh identity for the checkpoint that retains the Task atom.
    pub checkpoint: OperationCheckpointInput,
    /// Selected checkpoint-time source. Retained custody supplies its lower bound.
    pub clock: &'a dyn WallClockSourcePort,
    /// Exact checkpoint schema selected independently of this Task.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Current checkpoint-signing authority.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Finite accumulator preparation ceilings.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Finite retained-membership proof ceilings.
    pub membership_limits: AccumulatorMembershipLimits,
}

/// Durable first Task binding, remote observation, and physical custody proof.
///
/// Remote state remains owner-native evidence. This value is not a Pale result,
/// acceptance, settlement, or authority transfer.
#[must_use = "retain the Task custody result or continue through its explicit history owner"]
pub struct CompletedA2aInitialTaskCustody {
    binding: (ResidentRecordRefV1, A2aTaskBindingV1),
    observation: (ResidentRecordRefV1, A2aTaskObservationV1),
    checkpoint_preimage: ControlStateCheckpointPreimageV1,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: CompletedOperationObservationCustody,
}

pub(in crate::resident) struct A2aInitialTaskFinalizer {
    binding: (ResidentRecordRefV1, A2aTaskBindingV1),
    observation: (ResidentRecordRefV1, A2aTaskObservationV1),
    checkpoint_preimage: ControlStateCheckpointPreimageV1,
}

impl A2aInitialTaskFinalizer {
    fn finish(
        self,
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
        custody: CompletedOperationObservationCustody,
    ) -> CompletedA2aInitialTaskCustody {
        CompletedA2aInitialTaskCustody {
            binding: self.binding,
            observation: self.observation,
            checkpoint_preimage: self.checkpoint_preimage,
            checkpoint,
            custody,
        }
    }
}

impl CompletedA2aInitialTaskCustody {
    /// Borrow the exact durable binding between the Pale operation and remote Task.
    #[must_use]
    pub const fn binding(&self) -> (&ResidentRecordRefV1, &A2aTaskBindingV1) {
        (&self.binding.0, &self.binding.1)
    }

    /// Borrow the exact revision-one remote Task observation.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &A2aTaskObservationV1) {
        (&self.observation.0, &self.observation.1)
    }

    /// Borrow the checkpoint that retained the complete Task atom.
    #[must_use]
    pub const fn checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    /// Borrow the exact checkpoint fields that were admitted and signed.
    #[must_use]
    pub const fn checkpoint_preimage(&self) -> &ControlStateCheckpointPreimageV1 {
        &self.checkpoint_preimage
    }

    /// Borrow physical database and independent-anchor custody evidence.
    #[must_use]
    pub const fn custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }
}

/// Stopped initial Task custody, separated by the last completed phase.
pub enum A2aInitialTaskCommitFailure {
    /// Binding or revision-one observation admission failed before root preparation.
    Preparation {
        /// Exact completed response; no binding custody was attempted.
        response: Box<super::super::operation_dispatch::CompletedA2aTaskResponse>,
        /// Closed semantic or projection failure.
        reason: A2aTaskCustodyError,
    },
    /// Accumulator preparation or retained-membership proof failed before signing.
    Checkpoint {
        /// Exact completed response; no binding custody was attempted.
        response: Box<super::super::operation_dispatch::CompletedA2aTaskResponse>,
        /// Closed preparation failure.
        reason: A2aTaskCustodyError,
    },
    /// Checkpoint construction, admission, or signing failed before commitment.
    Issue {
        /// Exact completed response; no database commitment was attempted.
        response: Box<super::super::operation_dispatch::CompletedA2aTaskResponse>,
        /// Closed checkpoint failure.
        reason: A2aTaskCustodyError,
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

impl std::fmt::Debug for A2aInitialTaskCommitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Preparation { .. } => "preparation",
            Self::Checkpoint { .. } => "checkpoint",
            Self::Issue { .. } => "issue",
            Self::Commit { .. } => "commit",
            Self::Anchor { .. } => "anchor",
        };
        formatter
            .debug_struct("A2aInitialTaskCommitFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

pub(in crate::resident) enum ContinuableA2aInitialTaskCommitFailure {
    PrePhysical(A2aInitialTaskCommitFailure),
    Commit {
        finalizer: Box<A2aInitialTaskFinalizer>,
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        failure: OperationObservationCustodyCommitFailure,
    },
    Anchor {
        finalizer: Box<A2aInitialTaskFinalizer>,
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        failure: OperationObservationCustodyAnchorFailure,
    },
}

impl ContinuableA2aInitialTaskCommitFailure {
    pub(in crate::resident) fn into_public(self) -> A2aInitialTaskCommitFailure {
        match self {
            Self::PrePhysical(failure) => failure,
            Self::Commit {
                checkpoint,
                failure,
                ..
            } => A2aInitialTaskCommitFailure::Commit {
                checkpoint,
                failure,
            },
            Self::Anchor {
                checkpoint,
                failure,
                ..
            } => A2aInitialTaskCommitFailure::Anchor {
                checkpoint,
                failure,
            },
        }
    }

    pub(in crate::resident) fn continue_custody(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedA2aInitialTaskCustody, Self> {
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

/// Admit and durably retain one actual Task returned by `SendMessage`.
///
/// This service reuses the request and response evidence already owned by the
/// completed unary exchange. It creates no Task for a direct Message, performs
/// no network request, and does not change the local operation's result or
/// settlement state.
///
/// # Errors
///
/// Returns a phase-typed failure. Before commitment, the exact completed
/// response is returned for a corrected attempt. Physical failures retain their
/// existing database or anchor recovery owner and never permit send replay.
#[allow(clippy::too_many_arguments)]
pub fn commit_a2a_initial_task_response<
    'a,
    Evidence,
    BindingSign,
    ObservationSign,
    Store,
    CheckpointSign,
>(
    response: Box<super::super::operation_dispatch::CompletedA2aTaskResponse>,
    input: A2aInitialTaskCommitInput<'a>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_evidence: Evidence,
    sign_binding: BindingSign,
    sign_observation: ObservationSign,
    resolve_store: Store,
    sign_checkpoint: CheckpointSign,
) -> Result<CompletedA2aInitialTaskCustody, A2aInitialTaskCommitFailure>
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
    Store:
        FnOnce(&A2aTaskObservationV1) -> Result<AuthorityStoreCoordinate, InvalidResidentRecordV1>,
    CheckpointSign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    commit_a2a_initial_task_response_continuable(
        response,
        input,
        custody,
        anchor,
        admit_evidence,
        sign_binding,
        sign_observation,
        resolve_store,
        sign_checkpoint,
    )
    .map_err(ContinuableA2aInitialTaskCommitFailure::into_public)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(in crate::resident) fn commit_a2a_initial_task_response_continuable<
    'a,
    Evidence,
    BindingSign,
    ObservationSign,
    Store,
    CheckpointSign,
>(
    response: Box<super::super::operation_dispatch::CompletedA2aTaskResponse>,
    input: A2aInitialTaskCommitInput<'a>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_evidence: Evidence,
    sign_binding: BindingSign,
    sign_observation: ObservationSign,
    resolve_store: Store,
    sign_checkpoint: CheckpointSign,
) -> Result<CompletedA2aInitialTaskCustody, ContinuableA2aInitialTaskCommitFailure>
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
    Store:
        FnOnce(&A2aTaskObservationV1) -> Result<AuthorityStoreCoordinate, InvalidResidentRecordV1>,
    CheckpointSign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let predecessor = {
        let (reference, checkpoint) = response.send_checkpoint();
        (reference.clone(), checkpoint.clone())
    };
    let issuance_floor = std::cmp::max(response.observed_at(), *predecessor.1.issued_at());
    let issued_at =
        match crate::resident::operation_dispatch::sample_time(input.clock, issuance_floor) {
            Ok(issued_at) => issued_at,
            Err(reason) => {
                return Err(ContinuableA2aInitialTaskCommitFailure::PrePhysical(
                    A2aInitialTaskCommitFailure::Preparation {
                        response,
                        reason: A2aTaskCustodyError::Time(reason),
                    },
                ));
            }
        };
    if issued_at > response.retain_until() {
        return Err(ContinuableA2aInitialTaskCommitFailure::PrePhysical(
            A2aInitialTaskCommitFailure::Preparation {
                response,
                reason: A2aTaskCustodyError::Time(
                    crate::resident::OperationAttemptError::RetentionElapsed,
                ),
            },
        ));
    }
    let selection = response.checkpoint_selection();
    let append = match response.prepare_task_custody(
        input.task,
        admit_evidence,
        sign_binding,
        sign_observation,
        resolve_store,
    ) {
        Ok(append) => append,
        Err(reason) => {
            return Err(ContinuableA2aInitialTaskCommitFailure::PrePhysical(
                A2aInitialTaskCommitFailure::Preparation { response, reason },
            ));
        }
    };
    let binding = (append.binding().0.clone(), append.binding().1.clone());
    let observation = (
        append.observation().0.clone(),
        append.observation().1.clone(),
    );
    let prepared = match append.prepare_checkpoint(
        selection,
        &predecessor.0,
        &predecessor.1,
        input.accumulator_limits,
        input.membership_limits,
        custody,
    ) {
        Ok(prepared) => prepared,
        Err(reason) => {
            return Err(ContinuableA2aInitialTaskCommitFailure::PrePhysical(
                A2aInitialTaskCommitFailure::Checkpoint { response, reason },
            ));
        }
    };
    let preimage = match checkpoint_preimage(
        &prepared,
        (&predecessor.0, &predecessor.1),
        &input.checkpoint,
        issued_at,
        input.checkpoint_schema,
        input.checkpoint_authority,
    ) {
        Ok(preimage) => preimage,
        Err(reason) => {
            return Err(ContinuableA2aInitialTaskCommitFailure::PrePhysical(
                A2aInitialTaskCommitFailure::Issue { response, reason },
            ));
        }
    };
    let checkpoint_ref = input.checkpoint.reference;
    let checkpoint_preimage = preimage.clone();
    let (physical, checkpoint_value) = match prepared.issue_checkpoint_with_value(
        &checkpoint_ref,
        preimage,
        input.checkpoint_schema,
        input.checkpoint_authority,
        sign_checkpoint,
    ) {
        Ok(value) => value,
        Err(reason) => {
            return Err(ContinuableA2aInitialTaskCommitFailure::PrePhysical(
                A2aInitialTaskCommitFailure::Issue { response, reason },
            ));
        }
    };
    let checkpoint = (checkpoint_ref, checkpoint_value);
    let finalizer = Box::new(A2aInitialTaskFinalizer {
        binding,
        observation,
        checkpoint_preimage,
    });
    let pending = match physical.commit(custody, anchor) {
        Ok(pending) => pending,
        Err(failure) => {
            return Err(ContinuableA2aInitialTaskCommitFailure::Commit {
                finalizer,
                checkpoint: Box::new(checkpoint),
                failure,
            });
        }
    };
    let custody = match pending.advance_anchor(anchor) {
        Ok(completed) => completed,
        Err(failure) => {
            return Err(ContinuableA2aInitialTaskCommitFailure::Anchor {
                finalizer,
                checkpoint: Box::new(checkpoint),
                failure,
            });
        }
    };
    Ok(finalizer.finish(checkpoint, custody))
}

pub(super) fn checkpoint_preimage(
    prepared: &PreparedA2aTaskCheckpoint<'_>,
    predecessor: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    identity: &OperationCheckpointInput,
    issued_at: UnixTimeV1,
    schema: &SchemaDigest,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
) -> Result<ControlStateCheckpointPreimageV1, A2aTaskCustodyError> {
    let sequence = predecessor
        .1
        .domain_sequence()
        .get()
        .checked_add(1)
        .ok_or(A2aTaskCustodyError::Evidence)?;
    Ok(ControlStateCheckpointPreimageV1 {
        schema_id: predecessor.1.schema_id().clone(),
        schema_sha256: schema.clone(),
        control_checkpoint_id: identity.id.clone(),
        control_store_id: authority.control_store_id.clone(),
        control_incarnation_id: authority.control_incarnation_id.clone(),
        checkpoint_domain_id: authority.checkpoint_domain_id.clone(),
        domain_epoch: *predecessor.1.domain_epoch(),
        domain_sequence: ControlDomainSequenceV1::new(sequence)?,
        previous_control_checkpoint_ref: Some(predecessor.0.clone()),
        previous_control_checkpoint_sha256: Some(
            predecessor.1.canonical_control_checkpoint_sha256().clone(),
        ),
        checkpoint_profile_ref: authority.checkpoint_profile_ref.clone(),
        checkpoint_profile_sha256: authority.checkpoint_profile_sha256.clone(),
        control_schema_version: *predecessor.1.control_schema_version(),
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
