//! One authorized, durably retained `CancelTask` request and exact read-back.

use super::super::{
    prepare_a2a_task_observation_custody, A2aTaskCustodyError, A2aTaskObservationCustodyInput,
    KnownA2aTaskBasis, PreparedA2aTaskAppend, RecoveredA2aTaskHistory,
};
use crate::resident::{
    operation_dispatch::signing, validate_recovered_operation_cancellation, IssuedLocalOperation,
    OperationAttemptError, OperationAttemptHistoryInput, OperationControlError,
    OperationObservationCustodyProjectionError,
};
use pale_core::records::*;
use pale_ports::{
    A2aCancelTaskEvidencePort, A2aExchange, A2aExchangeCommand, A2aExchangeCoordinate,
    A2aExchangeError, A2aExchangeFacts, A2aExchangeLimits, A2aExchangeParents, A2aExchangePort,
    A2aExchangeRequest, A2aPreparedExchange, AuthorityStoreCoordinate, OperationA2aEvidence,
    OperationCustodyCandidateError, WallClockSourceError, WallClockSourcePort,
};
use std::{
    num::NonZeroUsize,
    task::{Context, Poll},
};

/// Current facts presented to the cancellation-authorization owner.
///
/// Retained history proves identity and lineage. The callback must establish
/// the current caller, cancellation authority, disclosure ceiling, revocation
/// state, and any owner-native policy.
pub struct A2aCancelTaskAuthorizationContext<'a> {
    /// Fresh signed same-handle `operation.cancel` request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Original admitted request that started the operation.
    pub original_request: &'a CommonRequestEnvelopeV1,
    /// Durable Pale operation.
    pub operation: (&'a ResidentRecordRefV1, &'a OperationHandleV1),
    /// Historical provider dispatch that identified the remote Task.
    pub dispatch: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    /// Exact local observation head at this authorization point.
    pub local_head: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    /// Historical admitted route. Its old expiry does not itself decide cancellation access.
    pub route: (&'a ResidentRecordRefV1, &'a A2aRouteAdmissionV1),
    /// Known remote Task identity.
    pub binding: (&'a ResidentRecordRefV1, &'a A2aTaskBindingV1),
    /// Current admitted remote-history head.
    pub remote_head: (&'a ResidentRecordRefV1, &'a A2aTaskObservationV1),
    /// Time at which current authority is evaluated.
    pub authorized_at: UnixTimeV1,
}

/// Inputs for one cancellation request against an already identified Task.
pub struct A2aCancelTaskInput<'a> {
    /// Fresh signed same-handle `operation.cancel` request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Fully admitted local and remote history. This is not current permission.
    pub history: &'a RecoveredA2aTaskHistory,
    /// Exact retained private provider selection behind the original dispatch.
    pub attempt: OperationAttemptHistoryInput<'a>,
    /// Fresh JSON-RPC request identity.
    pub request_id: &'a A2aJsonRpcRequestIdV1,
    /// Fresh immutable reference for exact request bytes.
    pub request_record_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for exact response bytes.
    pub response_record_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for a second item that violates unary framing.
    pub additional_response_record_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for the local `CancellationRequested` observation.
    pub cancellation_observation_ref: ResidentRecordRefV1,
    /// Independently admitted observation signer.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// One bounded exchange profile.
    pub limits: A2aExchangeLimits,
    /// Aggregate retained request/response byte ceiling.
    pub maximum_evidence_bytes: usize,
}

/// Fresh fields needed for the remote Task-history successor.
pub struct A2aCancelTaskObservationInput<'a> {
    /// History recovered after the local cancellation request was committed.
    pub history: &'a RecoveredA2aTaskHistory,
    /// Original admitted operation owner.
    pub operation: &'a IssuedLocalOperation,
    /// Fresh immutable reference for the remote-history successor.
    pub observation_ref: ResidentRecordRefV1,
    /// Exact remote-history schema and current runtime signer.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Aggregate exact request/response evidence ceiling.
    pub maximum_evidence_bytes: usize,
    /// Maximum signed observation bytes.
    pub maximum_record_bytes: usize,
}

/// Private physical-custody inputs for one successful cancellation read-back.
pub(in crate::resident) struct A2aCancelTaskReadbackCommitInput<'a> {
    pub(in crate::resident) observation: A2aCancelTaskObservationInput<'a>,
    pub(in crate::resident) checkpoint: crate::resident::OperationCheckpointInput,
    pub(in crate::resident) issued_at: UnixTimeV1,
    pub(in crate::resident) checkpoint_schema: &'a SchemaDigest,
    pub(in crate::resident) checkpoint_authority:
        &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub(in crate::resident) accumulator_limits: pale_ports::AccumulatorPreparationLimits,
    pub(in crate::resident) membership_limits: pale_ports::AccumulatorMembershipLimits,
}

/// Private physical-custody inputs for one incomplete cancellation disposition.
pub(in crate::resident) struct A2aCancelTaskDispositionCommitInput<'a> {
    pub(in crate::resident) disposition: A2aCancelTaskDispositionInput<'a>,
    pub(in crate::resident) checkpoint: crate::resident::OperationCheckpointInput,
    pub(in crate::resident) checkpoint_issued_at: UnixTimeV1,
    pub(in crate::resident) checkpoint_schema: &'a SchemaDigest,
    pub(in crate::resident) checkpoint_authority:
        &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub(in crate::resident) accumulator_limits: pale_ports::AccumulatorPreparationLimits,
}

/// Inputs for the durable local disposition of an incomplete cancellation call.
pub struct A2aCancelTaskDispositionInput<'a> {
    /// History recovered after the cancellation request was committed.
    pub history: &'a RecoveredA2aTaskHistory,
    /// Exact original operation owner.
    pub operation: &'a IssuedLocalOperation,
    /// Fresh immutable reference for the diagnostic.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for the local successor observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Existing reconciliation frontier selected by its semantic owner.
    pub reconciliation: ReconciliationFrontierReferenceV1,
    /// Exact current runtime signer and observation schema.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Time at which the carrier disposition was observed.
    pub observed_at: UnixTimeV1,
    /// Maximum canonical diagnostic bytes.
    pub maximum_diagnostic_bytes: usize,
}

/// Current permission was established; no bytes have been serialized.
#[must_use = "authorization alone does not prepare or retain a cancellation request"]
pub(super) struct AuthorizedA2aCancelTask<'a> {
    request: &'a CommonRequestEnvelopeV1,
    original_request: &'a CommonRequestEnvelopeV1,
    history: &'a RecoveredA2aTaskHistory,
    operation: &'a IssuedLocalOperation,
    attempt: crate::resident::operation_dispatch::RecoveredProviderAttempt<'a>,
    basis: KnownA2aTaskBasis,
    request_id: &'a A2aJsonRpcRequestIdV1,
    request_record_ref: ResidentRecordRefV1,
    response_record_ref: ResidentRecordRefV1,
    additional_response_record_ref: ResidentRecordRefV1,
    cancellation_observation_ref: ResidentRecordRefV1,
    signing: SignedResidentAdmissionContextV1<'a>,
    limits: A2aExchangeLimits,
    maximum_evidence_bytes: usize,
}

/// Signed cancellation request and exact bytes awaiting checkpoint custody.
#[must_use = "retain and anchor the cancellation request before carrier release"]
pub(super) struct PreparedA2aCancelTask<'a> {
    pub(super) release: A2aCancelTaskRelease<'a>,
}

pub(super) struct A2aCancelTaskRelease<'a> {
    pub(super) request: &'a CommonRequestEnvelopeV1,
    pub(super) original_request: &'a CommonRequestEnvelopeV1,
    pub(super) history: &'a RecoveredA2aTaskHistory,
    pub(super) operation: &'a IssuedLocalOperation,
    pub(super) basis: KnownA2aTaskBasis,
    pub(super) prepared: A2aPreparedExchange,
    pub(super) request_evidence: OperationA2aEvidence,
    pub(super) response_record_ref: ResidentRecordRefV1,
    pub(super) additional_response_record_ref: ResidentRecordRefV1,
    pub(super) expected_task: A2aRemoteTaskIdV1,
    pub(super) expected_context: A2aRemoteContextIdV1,
    pub(super) cancellation: (ResidentRecordRefV1, OperationObservationV1),
    pub(super) maximum_evidence_bytes: usize,
}

pub(super) struct A2aCancelTaskResponseContext {
    pub(super) basis: KnownA2aTaskBasis,
    pub(super) request_evidence: OperationA2aEvidence,
    pub(super) response_record_ref: ResidentRecordRefV1,
    pub(super) additional_response_record_ref: ResidentRecordRefV1,
    pub(super) expected_task: A2aRemoteTaskIdV1,
    pub(super) expected_context: A2aRemoteContextIdV1,
    pub(super) request_id: A2aJsonRpcRequestIdV1,
    pub(super) cancellation: (ResidentRecordRefV1, OperationObservationV1),
    pub(super) maximum_evidence_bytes: usize,
    pub(super) start_authorized_at: UnixTimeV1,
}

/// One started cancellation request. No remote outcome is known yet.
#[must_use = "poll or cancel the started request; never replay it after acceptance"]
pub struct InFlightA2aCancelTask {
    pub(super) exchange: Box<dyn A2aExchange>,
    pub(super) custody: crate::resident::CompletedOperationObservationCustody,
    pub(super) context: A2aCancelTaskResponseContext,
    pub(super) expected: A2aExchangeCoordinate,
}

/// A durably retained cancellation request rejected before carrier acceptance.
#[must_use = "the request did not start; preserve its exact custody and reason"]
pub struct A2aCancelTaskNotStarted {
    pub(super) custody: crate::resident::CompletedOperationObservationCustody,
    pub(super) context: A2aCancelTaskResponseContext,
    pub(super) reason: A2aExchangeError,
}

/// A cancellation request that may have reached the remote agent.
///
/// This phase deliberately exposes no replay method. A caller must reconcile
/// the remote Task before it can make another effectful decision.
#[must_use = "reconcile the remote Task; never replay this cancellation request"]
pub struct A2aCancelTaskOutcomeUnknown {
    pub(super) custody: crate::resident::CompletedOperationObservationCustody,
    pub(super) context: A2aCancelTaskResponseContext,
    pub(super) responses: Vec<OperationA2aEvidence>,
    pub(super) reason: A2aCancelTaskUncertainty,
}

/// Typed incomplete carrier outcome retained without any request replay method.
pub(in crate::resident) enum A2aCancelTaskDispositionPhase {
    NotStarted(Box<A2aCancelTaskNotStarted>),
    OutcomeUnknown(Box<A2aCancelTaskOutcomeUnknown>),
}

/// Signed local uncertainty and its diagnostic before checkpoint custody.
#[must_use = "commit the diagnostic and uncertainty observation before reconciliation"]
pub struct PreparedA2aCancelTaskDisposition<'a> {
    custody: crate::resident::CompletedOperationObservationCustody,
    context: A2aCancelTaskResponseContext,
    kind: A2aCancelDispositionKind,
    operation: &'a IssuedLocalOperation,
    current_checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    diagnostic: crate::diagnostic::AdmittedDiagnostic,
    diagnostic_record: pale_ports::ControlRecord,
    responses: Vec<OperationA2aEvidence>,
}

/// Recovered cancellation uncertainty awaiting exact checkpoint construction.
///
/// This phase is created only from a request that cold-restart admission has
/// already proved under the current cancellation head. It owns no carrier and
/// exposes no way to rebuild or release `CancelTask`.
#[must_use = "retain the recovered uncertainty under its exact checkpoint"]
pub(in crate::resident) struct PreparedRecoveredA2aCancelTaskDisposition<'a> {
    operation: &'a IssuedLocalOperation,
    current_checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    diagnostic_record: pale_ports::ControlRecord,
}

/// A disposition could not be admitted; the original phase remains owned.
pub struct A2aCancelTaskDispositionFailure<Phase> {
    phase: Box<Phase>,
    reason: A2aCancelTaskError,
}

/// Why an accepted cancellation request lacks a complete unary read-back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A2aCancelTaskUncertainty {
    /// The carrier accepted bytes but reported another exchange coordinate.
    UnlikeExchangeCoordinate,
    /// The carrier ended before returning a response item.
    MissingResponse,
    /// The accepted exchange failed while Pale was reading its response.
    Carrier(A2aExchangeError),
    /// The response shape, Task identity, context, or unary framing differed.
    UnexpectedResponse,
    /// Exact response bytes exceeded the selected aggregate evidence ceiling.
    EvidenceBound,
    /// Independent protocol qualification rejected the exact response bytes.
    ProtocolQualification,
    /// The selected observation clock failed after an exact response arrived.
    ObservationClock,
    /// The response time fell outside the retained operation interval.
    ObservationTime,
    /// The local reader stopped without evidence of remote cancellation.
    LocalReadCancelled,
}

/// A carrier start stopped before or after queue acceptance.
pub enum A2aCancelTaskStartFailure<'a> {
    /// Current post-commit release checks failed before any carrier call.
    Preflight {
        /// Exact retained request, still unopened by the carrier.
        retained: Box<super::RetainedA2aCancelTask<'a>>,
        /// Closed local reason for refusing release.
        reason: A2aCancelTaskError,
    },
    /// The carrier proved it did not accept the request.
    NotStarted(Box<A2aCancelTaskNotStarted>),
    /// The carrier accepted the request, but its outcome is uncertain.
    OutcomeUnknown(Box<A2aCancelTaskOutcomeUnknown>),
}

/// One exact Task response received; unary EOF is still required.
#[must_use = "a cancellation read-back is incomplete until unary EOF"]
pub struct ReceivedA2aCancelTaskResponse {
    exchange: Box<dyn A2aExchange>,
    custody: crate::resident::CompletedOperationObservationCustody,
    context: A2aCancelTaskResponseContext,
    response_evidence: OperationA2aEvidence,
    remote_state: A2aRemoteTaskStateV1,
}

/// Exact request/response evidence after unary EOF.
#[must_use = "the remote read-back still requires predecessor-linked task-history custody"]
pub struct CompletedA2aCancelTaskReadback {
    custody: crate::resident::CompletedOperationObservationCustody,
    context: A2aCancelTaskResponseContext,
    response_evidence: OperationA2aEvidence,
    remote_state: A2aRemoteTaskStateV1,
    observed_at: UnixTimeV1,
}

/// Successful remote-history custody that still proves no Pale terminal outcome.
pub(in crate::resident) struct CompletedA2aCancelTaskReadbackCustody {
    binding: (ResidentRecordRefV1, A2aTaskBindingV1),
    observation: (ResidentRecordRefV1, A2aTaskObservationV1),
    checkpoint_preimage: ControlStateCheckpointPreimageV1,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: crate::resident::CompletedOperationObservationCustody,
}

/// Durable local disposition that remains distinct from remote Task history.
pub(in crate::resident) struct CompletedA2aCancelTaskDispositionCustody {
    binding: (ResidentRecordRefV1, A2aTaskBindingV1),
    cancellation: (ResidentRecordRefV1, OperationObservationDigest),
    request_custody: crate::resident::CompletedOperationObservationCustody,
    disposition_custody: crate::resident::CompletedOperationObservationCustody,
}

pub(in crate::resident) struct A2aCancelTaskDispositionFinalizer {
    binding: (ResidentRecordRefV1, A2aTaskBindingV1),
    cancellation: (ResidentRecordRefV1, OperationObservationDigest),
    request_custody: crate::resident::CompletedOperationObservationCustody,
}

/// Exact incomplete-disposition phase retained across physical failure.
pub(in crate::resident) enum ContinuableA2aCancelTaskDispositionCommitFailure {
    PrePhysical {
        phase: Box<A2aCancelTaskDispositionPhase>,
        reason: A2aCancelTaskError,
    },
    Commit {
        finalizer: Box<A2aCancelTaskDispositionFinalizer>,
        failure: crate::resident::OperationObservationCustodyCommitFailure,
    },
    Anchor {
        finalizer: Box<A2aCancelTaskDispositionFinalizer>,
        failure: crate::resident::OperationObservationCustodyAnchorFailure,
    },
}

impl A2aCancelTaskDispositionFinalizer {
    fn finish(
        self,
        custody: crate::resident::CompletedOperationObservationCustody,
    ) -> CompletedA2aCancelTaskDispositionCustody {
        CompletedA2aCancelTaskDispositionCustody {
            binding: self.binding,
            cancellation: self.cancellation,
            request_custody: self.request_custody,
            disposition_custody: custody,
        }
    }
}

impl CompletedA2aCancelTaskDispositionCustody {
    /// Borrow the exact binding retained by this local disposition.
    pub(in crate::resident) const fn binding(&self) -> (&ResidentRecordRefV1, &A2aTaskBindingV1) {
        (&self.binding.0, &self.binding.1)
    }

    /// Borrow the exact local cancellation observation this disposition closes.
    pub(in crate::resident) const fn cancellation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationDigest) {
        (&self.cancellation.0, &self.cancellation.1)
    }

    /// Borrow physical custody of the local cancellation request.
    pub(in crate::resident) const fn request_custody(
        &self,
    ) -> &crate::resident::CompletedOperationObservationCustody {
        &self.request_custody
    }

    /// Borrow physical custody of the local disposition successor.
    pub(in crate::resident) const fn disposition_custody(
        &self,
    ) -> &crate::resident::CompletedOperationObservationCustody {
        &self.disposition_custody
    }
}

impl ContinuableA2aCancelTaskDispositionCommitFailure {
    /// Borrow the binding after a physical candidate exists.
    pub(in crate::resident) fn binding(&self) -> Option<(&ResidentRecordRefV1, &A2aTaskBindingV1)> {
        match self {
            Self::PrePhysical { .. } => None,
            Self::Commit { finalizer, .. } | Self::Anchor { finalizer, .. } => {
                Some((&finalizer.binding.0, &finalizer.binding.1))
            }
        }
    }

    /// Borrow the physical store after a candidate exists.
    pub(in crate::resident) fn store(&self) -> Option<&AuthorityStoreCoordinate> {
        match self {
            Self::PrePhysical { .. } => None,
            Self::Commit { failure, .. } => Some(failure.candidate().store()),
            Self::Anchor { failure, .. } => Some(failure.candidate().store()),
        }
    }

    /// Borrow the exact local cancellation closed by a physical candidate.
    pub(in crate::resident) fn cancellation(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &OperationObservationDigest)> {
        match self {
            Self::PrePhysical { .. } => None,
            Self::Commit { finalizer, .. } | Self::Anchor { finalizer, .. } => {
                Some((&finalizer.cancellation.0, &finalizer.cancellation.1))
            }
        }
    }

    /// Recover the incomplete carrier phase only before physical custody began.
    pub(in crate::resident) fn into_phase(
        self,
    ) -> Result<(A2aCancelTaskDispositionPhase, A2aCancelTaskError), Self> {
        match self {
            Self::PrePhysical { phase, reason } => Ok((*phase, reason)),
            other => Err(other),
        }
    }

    /// Continue only exact database or independent-anchor custody.
    pub(in crate::resident) fn continue_custody(
        self,
        custody: &mut dyn pale_ports::OperationCustodyPort,
        anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
    ) -> Result<CompletedA2aCancelTaskDispositionCustody, Self> {
        let (finalizer, pending) = match self {
            Self::PrePhysical { .. } => return Err(self),
            Self::Commit { finalizer, failure } => {
                let pending = match failure.reconcile_exact(custody) {
                    Ok(pending) => pending,
                    Err(failure) => match failure.try_into_retryable() {
                        Ok(prepared) => match prepared.commit(custody, anchor) {
                            Ok(pending) => pending,
                            Err(failure) => return Err(Self::Commit { finalizer, failure }),
                        },
                        Err(failure) => return Err(Self::Commit { finalizer, failure }),
                    },
                };
                (finalizer, pending)
            }
            Self::Anchor { finalizer, failure } => match failure.try_into_retryable() {
                Ok(pending) => (finalizer, pending),
                Err(failure) => match failure.continue_after_failed_observation(anchor) {
                    Ok(completed) => return Ok(finalizer.finish(completed)),
                    Err(failure) => return Err(Self::Anchor { finalizer, failure }),
                },
            },
        };
        match pending.advance_anchor(anchor) {
            Ok(completed) => Ok(finalizer.finish(completed)),
            Err(failure) => Err(Self::Anchor { finalizer, failure }),
        }
    }
}

pub(in crate::resident) struct A2aCancelTaskReadbackFinalizer {
    binding: (ResidentRecordRefV1, A2aTaskBindingV1),
    observation: (ResidentRecordRefV1, A2aTaskObservationV1),
    checkpoint_preimage: ControlStateCheckpointPreimageV1,
}

/// Exact successful-readback custody phase retained after physical failure.
pub(in crate::resident) enum ContinuableA2aCancelTaskReadbackCommitFailure {
    PrePhysical {
        completed: Box<CompletedA2aCancelTaskReadback>,
        reason: A2aCancelTaskError,
    },
    Commit {
        finalizer: Box<A2aCancelTaskReadbackFinalizer>,
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        failure: crate::resident::OperationObservationCustodyCommitFailure,
    },
    Anchor {
        finalizer: Box<A2aCancelTaskReadbackFinalizer>,
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        failure: crate::resident::OperationObservationCustodyAnchorFailure,
    },
}

impl A2aCancelTaskReadbackFinalizer {
    fn finish(
        self,
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
        custody: crate::resident::CompletedOperationObservationCustody,
    ) -> CompletedA2aCancelTaskReadbackCustody {
        CompletedA2aCancelTaskReadbackCustody {
            binding: self.binding,
            observation: self.observation,
            checkpoint_preimage: self.checkpoint_preimage,
            checkpoint,
            custody,
        }
    }
}

impl CompletedA2aCancelTaskReadbackCustody {
    /// Borrow the exact binding retained with the remote-history successor.
    pub(in crate::resident) const fn binding(&self) -> (&ResidentRecordRefV1, &A2aTaskBindingV1) {
        (&self.binding.0, &self.binding.1)
    }

    /// Borrow physical custody of the predecessor-linked remote history.
    pub(in crate::resident) const fn custody(
        &self,
    ) -> &crate::resident::CompletedOperationObservationCustody {
        &self.custody
    }

    /// Borrow the predecessor-linked remote-history successor.
    pub(in crate::resident) const fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &A2aTaskObservationV1) {
        (&self.observation.0, &self.observation.1)
    }

    /// Borrow the exact checkpoint preimage issued for this custody.
    pub(in crate::resident) const fn checkpoint_preimage(
        &self,
    ) -> &ControlStateCheckpointPreimageV1 {
        &self.checkpoint_preimage
    }

    /// Borrow the signed checkpoint retained with remote history.
    pub(in crate::resident) const fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }
}

impl ContinuableA2aCancelTaskReadbackCommitFailure {
    /// Borrow the binding when physical custody has already been prepared.
    pub(in crate::resident) fn binding(&self) -> Option<(&ResidentRecordRefV1, &A2aTaskBindingV1)> {
        match self {
            Self::PrePhysical { .. } => None,
            Self::Commit { finalizer, .. } | Self::Anchor { finalizer, .. } => {
                Some((&finalizer.binding.0, &finalizer.binding.1))
            }
        }
    }

    /// Borrow the physical store after a candidate was issued.
    pub(in crate::resident) fn store(&self) -> Option<&pale_ports::AuthorityStoreCoordinate> {
        match self {
            Self::PrePhysical { .. } => None,
            Self::Commit { failure, .. } => Some(failure.candidate().store()),
            Self::Anchor { failure, .. } => Some(failure.candidate().store()),
        }
    }

    /// Recover carrier evidence only when no physical custody was attempted.
    pub(in crate::resident) fn into_completed(
        self,
    ) -> Result<(CompletedA2aCancelTaskReadback, A2aCancelTaskError), Self> {
        match self {
            Self::PrePhysical { completed, reason } => Ok((*completed, reason)),
            other => Err(other),
        }
    }

    /// Continue only exact database or independent-anchor custody.
    pub(in crate::resident) fn continue_custody(
        self,
        custody: &mut dyn pale_ports::OperationCustodyPort,
        anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
    ) -> Result<CompletedA2aCancelTaskReadbackCustody, Self> {
        let (finalizer, checkpoint, pending) = match self {
            Self::PrePhysical { .. } => return Err(self),
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

/// One wake-driven transition that preserves its pending phase.
pub enum A2aCancelTaskPoll<Pending, Ready> {
    /// No carrier item is ready.
    Pending(Pending),
    /// The phase advanced or closed.
    Ready(Result<Ready, Box<A2aCancelTaskOutcomeUnknown>>),
}

/// Closed failure before a remote read-back becomes durable Task history.
#[derive(Debug, thiserror::Error)]
pub enum A2aCancelTaskError {
    /// The signed same-handle Cancel request did not match current protected history.
    #[error(transparent)]
    Control(#[from] OperationControlError),
    /// Current caller or cancellation policy denied the request.
    #[error(transparent)]
    Authorization(#[from] InvalidResidentRecordV1),
    /// Historical provider selection did not match the admitted dispatch.
    #[error(transparent)]
    Attempt(#[from] OperationAttemptError),
    /// The application-owned diagnostic could not be admitted.
    #[error(transparent)]
    Diagnostic(#[from] crate::diagnostic::DiagnosticAdmissionError),
    /// Required time ordering or complete exchange retention failed.
    #[error("A2A CancelTask time is outside the retained operation interval")]
    Time,
    /// The selected wall clock could not produce a portable current time.
    #[error(transparent)]
    Clock(#[from] WallClockSourceError),
    /// An immutable coordinate, phase, or retained history member was substituted.
    #[error("A2A CancelTask inputs do not match retained history")]
    Association,
    /// Mechanical exchange preparation, start, or response failed.
    #[error(transparent)]
    Exchange(#[from] A2aExchangeError),
    /// Exact protocol bytes could not enter the selected bound.
    #[error(transparent)]
    Evidence(#[from] OperationCustodyCandidateError),
    /// Observation or checkpoint projection failed.
    #[error(transparent)]
    Projection(#[from] OperationObservationCustodyProjectionError),
    /// Signed Task-history custody preparation failed.
    #[error(transparent)]
    TaskCustody(#[from] A2aTaskCustodyError),
    /// A replayed or ambiguously recovered database commit cannot release the request.
    #[error("A2A CancelTask release requires a database commit made by this call")]
    NotNewDatabaseCommit,
    /// Database custody of the request and local observation failed.
    #[error(transparent)]
    Commit(#[from] crate::resident::OperationObservationCustodyCommitFailure),
    /// Independent anchoring of the request and local observation failed.
    #[error(transparent)]
    Anchor(#[from] crate::resident::OperationObservationCustodyAnchorFailure),
}

/// Establish current cancellation authority before any request bytes exist.
///
/// # Errors
///
/// Returns an error when retained operation and Task history do not match, time
/// or retention is invalid, current authority rejects the request, or the exact
/// provider attempt cannot be recovered.
fn authorize_a2a_cancel_task<'a, Authorize>(
    input: A2aCancelTaskInput<'a>,
    authorized_at: UnixTimeV1,
    authorize: Authorize,
) -> Result<AuthorizedA2aCancelTask<'a>, A2aCancelTaskError>
where
    Authorize: FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
{
    let operation = input.attempt.operation;
    let original_request = input.attempt.request;
    let handle = operation.handle();
    let local_head = input.history.local_observation().selected_head();
    let remote_head = input.history.observation();
    let binding = input.history.binding();
    let basis = KnownA2aTaskBasis::capture(
        operation,
        input.attempt.selection,
        input.history,
        authorized_at,
    );
    let fresh = [
        &input.request_record_ref,
        &input.response_record_ref,
        &input.additional_response_record_ref,
        &input.cancellation_observation_ref,
    ];
    if input.attempt.dispatched != input.history.dispatch().1
        || binding.1.operation_handle_ref() != handle.0
        || binding.1.operation_handle_sha256() != handle.1.canonical_operation_handle_sha256()
        || !matches!(
            local_head.1.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched { .. }
            }
        )
        || fresh.iter().enumerate().any(|(index, reference)| {
            basis.contains_record_ref(reference) || fresh[..index].contains(reference)
        })
        || input.maximum_evidence_bytes == 0
    {
        return Err(A2aCancelTaskError::Association);
    }
    if authorized_at < *remote_head.1.observed_at()
        || authorized_at < *local_head.1.observed_at()
        || authorized_at > *handle.1.retain_until()
        || !basis.admits_exchange_deadline(authorized_at, input.limits.total_timeout)
    {
        return Err(A2aCancelTaskError::Time);
    }
    validate_recovered_operation_cancellation(
        input.request,
        original_request,
        operation,
        input.history.local_observation(),
        authorized_at,
    )?;
    let attempt = input.attempt.recover_provider_attempt()?;
    // Reconstructing the request joins the retained selection to the original
    // request, lease, generation, provider, and dispatched attempt before the
    // current authority owner sees any remote Task facts.
    attempt.provider_request()?;
    authorize(A2aCancelTaskAuthorizationContext {
        request: input.request,
        original_request,
        operation: handle,
        dispatch: input.history.dispatch(),
        local_head,
        route: input.history.route(),
        binding,
        remote_head,
        authorized_at,
    })?;
    Ok(AuthorizedA2aCancelTask {
        request: input.request,
        original_request,
        history: input.history,
        operation,
        attempt,
        basis,
        request_id: input.request_id,
        request_record_ref: input.request_record_ref,
        response_record_ref: input.response_record_ref,
        additional_response_record_ref: input.additional_response_record_ref,
        cancellation_observation_ref: input.cancellation_observation_ref,
        signing: input.signing,
        limits: input.limits,
        maximum_evidence_bytes: input.maximum_evidence_bytes,
    })
}

/// Authorize and prepare one durable `CancelTask` custody candidate.
///
/// The trusted clock is sampled inside this call before current authorization.
/// Exchange preparation serializes exact bytes but performs no carrier I/O. The
/// returned phase still requires database commitment and independent anchoring;
/// it cannot start the carrier.
///
/// # Errors
///
/// Returns an existing clock, authorization, association, preparation,
/// observation, or checkpoint-custody error without starting the carrier.
#[allow(clippy::too_many_arguments)]
pub fn prepare_a2a_cancel_task_custody<'a, Authorize, Rotation, ObservationSign, CheckpointSign>(
    input: A2aCancelTaskInput<'a>,
    port: &dyn A2aExchangePort,
    clock: &dyn WallClockSourcePort,
    authorize: Authorize,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    checkpoint: crate::resident::OperationCheckpointInput,
    checkpoint_issued_at: UnixTimeV1,
    checkpoint_schema: &SchemaDigest,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    accumulator_limits: pale_ports::AccumulatorPreparationLimits,
    resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    sign_checkpoint: CheckpointSign,
) -> Result<super::PreparedA2aCancelTaskCustody<'a>, A2aCancelTaskError>
where
    Authorize: FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
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
    let authorized_at = clock.observe_unix_time()?;
    authorize_a2a_cancel_task(input, authorized_at, authorize)?
        .prepare(port, admit_rotation, sign_observation)?
        .prepare_custody(
            checkpoint,
            checkpoint_issued_at,
            checkpoint_schema,
            checkpoint_authority,
            accumulator_limits,
            resolve_store,
            custody,
            sign_checkpoint,
        )
}

impl<'a> AuthorizedA2aCancelTask<'a> {
    /// Prepare exact bytes and the local cancellation-request observation.
    ///
    /// No network request starts. The returned append must be committed and
    /// independently anchored before a carrier can receive these bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the official request cannot be prepared, the
    /// carrier substitutes its coordinate, the evidence bound is exceeded, or
    /// the signed cancellation observation cannot be admitted.
    fn prepare<Rotation, Sign>(
        self,
        port: &dyn A2aExchangePort,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedA2aCancelTask<'a>, A2aCancelTaskError>
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
        let binding = self.history.binding().1;
        let local_head = self.history.local_observation().selected_head();
        let request = A2aExchangeRequest::new(
            self.attempt.provider_request()?,
            A2aExchangeParents {
                operation: self.operation.handle(),
                observation: local_head,
                route: self.history.route(),
            },
            self.request_id,
            A2aExchangeCommand::CancelTask(binding),
            self.limits,
        )?;
        let expected = request.coordinate().clone();
        let prepared = port.prepare(request)?;
        if prepared.coordinate().context() != &expected {
            return Err(A2aCancelTaskError::Association);
        }
        let request_evidence = OperationA2aEvidence::new(
            ImmutableRecordReferenceV1 {
                record_ref: self.request_record_ref,
                record_sha256: ArtifactDigest::hash_bytes(prepared.bytes()),
            },
            prepared.bytes().to_vec(),
            None,
            self.maximum_evidence_bytes,
        )?;
        let (handle_ref, handle) = self.operation.handle();
        let observation = signing::issue_observation(
            &OperationObservationAdmissionContextV1 {
                signed: self.signing,
                handle_ref,
                handle,
                predecessor: Some(local_head),
            },
            OperationObservationStateV1::CancellationRequested {
                request: evidence_reference(&request_evidence),
            },
            self.basis.authorized_at,
            admit_rotation,
            sign,
        )?;
        Ok(PreparedA2aCancelTask {
            release: A2aCancelTaskRelease {
                request: self.request,
                original_request: self.original_request,
                history: self.history,
                operation: self.operation,
                basis: self.basis,
                prepared,
                request_evidence,
                response_record_ref: self.response_record_ref,
                additional_response_record_ref: self.additional_response_record_ref,
                expected_task: binding.remote_task_id().clone(),
                expected_context: binding.remote_context_id().clone(),
                cancellation: (self.cancellation_observation_ref, observation),
                maximum_evidence_bytes: self.maximum_evidence_bytes,
            },
        })
    }
}

impl InFlightA2aCancelTask {
    /// Stop local response reads without claiming that remote cancellation stopped.
    pub fn cancel(self) -> A2aCancelTaskOutcomeUnknown {
        self.into_outcome_unknown(A2aCancelTaskUncertainty::LocalReadCancelled, Vec::new())
    }

    /// Poll one exact Task response and requalify its bytes independently.
    pub fn poll_response(
        mut self,
        cx: &mut Context<'_>,
        protocol: &dyn A2aCancelTaskEvidencePort,
    ) -> A2aCancelTaskPoll<Self, ReceivedA2aCancelTaskResponse> {
        if self.exchange.coordinate() != &self.expected {
            return A2aCancelTaskPoll::Ready(Err(Box::new(self.into_outcome_unknown(
                A2aCancelTaskUncertainty::UnlikeExchangeCoordinate,
                Vec::new(),
            ))));
        }
        let evidence = match self.exchange.poll_next(cx) {
            Poll::Pending => return A2aCancelTaskPoll::Pending(self),
            Poll::Ready(None) => {
                return A2aCancelTaskPoll::Ready(Err(Box::new(self.into_outcome_unknown(
                    A2aCancelTaskUncertainty::MissingResponse,
                    Vec::new(),
                ))));
            }
            Poll::Ready(Some(Err(error))) => {
                return A2aCancelTaskPoll::Ready(Err(Box::new(
                    self.into_outcome_unknown(A2aCancelTaskUncertainty::Carrier(error), Vec::new()),
                )));
            }
            Poll::Ready(Some(Ok(evidence))) => evidence,
        };
        let remaining = self
            .context
            .maximum_evidence_bytes
            .saturating_sub(self.context.request_evidence.byte_len());
        let response_evidence = match OperationA2aEvidence::new(
            ImmutableRecordReferenceV1 {
                record_ref: self.context.response_record_ref.clone(),
                record_sha256: ArtifactDigest::hash_bytes(&evidence.bytes),
            },
            evidence.bytes,
            None,
            remaining,
        ) {
            Ok(value) => value,
            Err(_) => {
                return A2aCancelTaskPoll::Ready(Err(Box::new(
                    self.into_outcome_unknown(A2aCancelTaskUncertainty::EvidenceBound, Vec::new()),
                )));
            }
        };
        let A2aExchangeFacts::Task {
            task,
            context,
            state,
        } = evidence.facts
        else {
            return A2aCancelTaskPoll::Ready(Err(Box::new(self.into_outcome_unknown(
                A2aCancelTaskUncertainty::UnexpectedResponse,
                vec![response_evidence],
            ))));
        };
        if task != self.context.expected_task
            || context != self.context.expected_context
            || evidence.sse.is_some()
        {
            return A2aCancelTaskPoll::Ready(Err(Box::new(self.into_outcome_unknown(
                A2aCancelTaskUncertainty::UnexpectedResponse,
                vec![response_evidence],
            ))));
        }
        let Some(maximum) = NonZeroUsize::new(self.context.maximum_evidence_bytes) else {
            return A2aCancelTaskPoll::Ready(Err(Box::new(self.into_outcome_unknown(
                A2aCancelTaskUncertainty::EvidenceBound,
                vec![response_evidence],
            ))));
        };
        match protocol.read_task_state_after_cancel(
            self.context.request_evidence.bytes(),
            response_evidence.bytes(),
            &self.context.expected_task,
            &self.context.expected_context,
            maximum,
        ) {
            Ok(qualified) if qualified == state => {}
            Ok(_) | Err(_) => {
                return A2aCancelTaskPoll::Ready(Err(Box::new(self.into_outcome_unknown(
                    A2aCancelTaskUncertainty::ProtocolQualification,
                    vec![response_evidence],
                ))));
            }
        }
        A2aCancelTaskPoll::Ready(Ok(ReceivedA2aCancelTaskResponse {
            exchange: self.exchange,
            custody: self.custody,
            context: self.context,
            response_evidence,
            remote_state: state,
        }))
    }

    fn into_outcome_unknown(
        mut self,
        reason: A2aCancelTaskUncertainty,
        responses: Vec<OperationA2aEvidence>,
    ) -> A2aCancelTaskOutcomeUnknown {
        self.exchange.cancel();
        A2aCancelTaskOutcomeUnknown {
            custody: self.custody,
            context: self.context,
            responses,
            reason,
        }
    }
}

impl ReceivedA2aCancelTaskResponse {
    /// Require unary EOF and sample the remote observation time there.
    pub fn poll_eof(
        mut self,
        cx: &mut Context<'_>,
        clock: &dyn WallClockSourcePort,
    ) -> A2aCancelTaskPoll<Self, CompletedA2aCancelTaskReadback> {
        match self.exchange.poll_next(cx) {
            Poll::Pending => A2aCancelTaskPoll::Pending(self),
            Poll::Ready(None) => {
                let observed_at = match clock.observe_unix_time() {
                    Ok(value) => value,
                    Err(_) => {
                        return A2aCancelTaskPoll::Ready(Err(Box::new(self.into_outcome_unknown(
                            A2aCancelTaskUncertainty::ObservationClock,
                            None,
                        ))));
                    }
                };
                if observed_at < self.context.start_authorized_at
                    || !self.context.basis.admits_observation_time(observed_at)
                {
                    return A2aCancelTaskPoll::Ready(Err(Box::new(
                        self.into_outcome_unknown(A2aCancelTaskUncertainty::ObservationTime, None),
                    )));
                }
                A2aCancelTaskPoll::Ready(Ok(CompletedA2aCancelTaskReadback {
                    custody: self.custody,
                    context: self.context,
                    response_evidence: self.response_evidence,
                    remote_state: self.remote_state,
                    observed_at,
                }))
            }
            Poll::Ready(Some(Err(error))) => A2aCancelTaskPoll::Ready(Err(Box::new(
                self.into_outcome_unknown(A2aCancelTaskUncertainty::Carrier(error), None),
            ))),
            Poll::Ready(Some(Ok(additional))) => {
                let remaining = self
                    .context
                    .maximum_evidence_bytes
                    .saturating_sub(self.context.request_evidence.byte_len())
                    .saturating_sub(self.response_evidence.byte_len());
                let additional_evidence = OperationA2aEvidence::new(
                    ImmutableRecordReferenceV1 {
                        record_ref: self.context.additional_response_record_ref.clone(),
                        record_sha256: ArtifactDigest::hash_bytes(&additional.bytes),
                    },
                    additional.bytes,
                    None,
                    remaining,
                );
                match additional_evidence {
                    Ok(additional_evidence) => {
                        A2aCancelTaskPoll::Ready(Err(Box::new(self.into_outcome_unknown(
                            A2aCancelTaskUncertainty::UnexpectedResponse,
                            Some(additional_evidence),
                        ))))
                    }
                    Err(_) => A2aCancelTaskPoll::Ready(Err(Box::new(
                        self.into_outcome_unknown(A2aCancelTaskUncertainty::EvidenceBound, None),
                    ))),
                }
            }
        }
    }

    /// Stop local EOF reads. This does not prove remote inactivity or safe replay.
    pub fn cancel(self) -> A2aCancelTaskOutcomeUnknown {
        self.into_outcome_unknown(A2aCancelTaskUncertainty::LocalReadCancelled, None)
    }

    fn into_outcome_unknown(
        mut self,
        reason: A2aCancelTaskUncertainty,
        additional: Option<OperationA2aEvidence>,
    ) -> A2aCancelTaskOutcomeUnknown {
        self.exchange.cancel();
        let mut responses = vec![self.response_evidence];
        if let Some(additional) = additional {
            responses.push(additional);
        }
        A2aCancelTaskOutcomeUnknown {
            custody: self.custody,
            context: self.context,
            responses,
            reason,
        }
    }
}

impl A2aCancelTaskNotStarted {
    /// Borrow the exact local cancellation observation this phase closes.
    pub(in crate::resident) const fn local_cancellation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.context.cancellation.0, &self.context.cancellation.1)
    }

    /// Borrow the carrier rejection that established non-acceptance.
    #[must_use]
    pub const fn reason(&self) -> A2aExchangeError {
        self.reason
    }

    /// Borrow physical custody of the local cancellation request.
    #[must_use]
    pub const fn local_custody(&self) -> &crate::resident::CompletedOperationObservationCustody {
        &self.custody
    }

    /// Borrow the exact request bytes. They do not grant permission to retry.
    #[must_use]
    pub const fn request_evidence(&self) -> &OperationA2aEvidence {
        &self.context.request_evidence
    }

    /// Admit a durable uncertainty successor without replaying `CancelTask`.
    ///
    /// # Errors
    ///
    /// Returns the intact phase when current history, reconciliation, time,
    /// references, diagnostics, projection, or signing cannot be admitted.
    pub fn prepare_disposition<'a, Reconciliation, Rotation, Sign>(
        self,
        input: A2aCancelTaskDispositionInput<'a>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedA2aCancelTaskDisposition<'a>, A2aCancelTaskDispositionFailure<Self>>
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
        let kind = A2aCancelDispositionKind::NotStarted(self.reason);
        prepare_cancel_disposition(
            self.custody,
            self.context,
            Vec::new(),
            input,
            kind,
            admit_reconciliation,
            admit_rotation,
            sign,
        )
        .map_err(|failure| {
            let (custody, context, responses, reason) = *failure;
            A2aCancelTaskDispositionFailure {
                phase: Box::new({
                    debug_assert!(responses.is_empty());
                    Self {
                        custody,
                        context,
                        reason: match kind {
                            A2aCancelDispositionKind::NotStarted(reason) => reason,
                            A2aCancelDispositionKind::OutcomeUnknown(_)
                            | A2aCancelDispositionKind::RestartAfterRequestCustody => {
                                unreachable!()
                            }
                        },
                    }
                }),
                reason,
            }
        })
    }
}

impl A2aCancelTaskOutcomeUnknown {
    /// Borrow the exact local cancellation observation this phase closes.
    pub(in crate::resident) const fn local_cancellation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.context.cancellation.0, &self.context.cancellation.1)
    }

    /// Return the closed reason that requires remote reconciliation.
    #[must_use]
    pub const fn reason(&self) -> A2aCancelTaskUncertainty {
        self.reason
    }

    /// Borrow the exact request bytes. This value exposes no replay method.
    #[must_use]
    pub const fn request_evidence(&self) -> &OperationA2aEvidence {
        &self.context.request_evidence
    }

    /// Borrow every bounded response item retained before uncertainty.
    #[must_use]
    pub fn response_evidence(&self) -> &[OperationA2aEvidence] {
        &self.responses
    }

    /// Borrow physical custody of the local cancellation request.
    #[must_use]
    pub const fn local_custody(&self) -> &crate::resident::CompletedOperationObservationCustody {
        &self.custody
    }

    /// Admit a durable uncertainty successor without replaying `CancelTask`.
    ///
    /// # Errors
    ///
    /// Returns the intact phase when current history, reconciliation, time,
    /// references, diagnostics, projection, or signing cannot be admitted.
    pub fn prepare_disposition<'a, Reconciliation, Rotation, Sign>(
        self,
        input: A2aCancelTaskDispositionInput<'a>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedA2aCancelTaskDisposition<'a>, A2aCancelTaskDispositionFailure<Self>>
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
        let kind = A2aCancelDispositionKind::OutcomeUnknown(self.reason);
        prepare_cancel_disposition(
            self.custody,
            self.context,
            self.responses,
            input,
            kind,
            admit_reconciliation,
            admit_rotation,
            sign,
        )
        .map_err(|failure| {
            let (custody, context, responses, reason) = *failure;
            A2aCancelTaskDispositionFailure {
                phase: Box::new(Self {
                    custody,
                    context,
                    responses,
                    reason: match kind {
                        A2aCancelDispositionKind::OutcomeUnknown(reason) => reason,
                        A2aCancelDispositionKind::NotStarted(_)
                        | A2aCancelDispositionKind::RestartAfterRequestCustody => unreachable!(),
                    },
                }),
                reason,
            }
        })
    }
}

impl<'a> A2aCancelTaskDispositionInput<'a> {
    /// Prepare explicit uncertainty after cold restart found durable request custody.
    ///
    /// The caller supplies the exact retained request evidence and request identity
    /// proved by restart admission. This method signs one predecessor-linked
    /// `OutcomeUnknown` successor. It has no carrier, replay, or Task-mutation path.
    pub(in crate::resident) fn prepare_recovered_request_uncertainty<
        Reconciliation,
        Rotation,
        Sign,
    >(
        self,
        request_evidence: &OperationA2aEvidence,
        request_id: &A2aJsonRpcRequestIdV1,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedRecoveredA2aCancelTaskDisposition<'a>, A2aCancelTaskError>
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
        let local_head = self.history.local_observation().selected_head();
        let current_checkpoint = self.history.local_observation().current_checkpoint();
        let request_reference = evidence_reference(request_evidence);
        let binding = self.history.binding();
        let route = self.history.route();
        let remote = self.history.observation();
        let selected_remote = self.history.selected_observation();
        let (handle_ref, handle) = self.operation.handle();
        if binding.1.operation_handle_ref() != handle_ref
            || binding.1.operation_handle_sha256() != handle.canonical_operation_handle_sha256()
            || !matches!(
                local_head.1.state(),
                OperationObservationStateV1::CancellationRequested { request }
                    if request == &request_reference
            )
            || self.maximum_diagnostic_bytes == 0
        {
            return Err(A2aCancelTaskError::Association);
        }
        if self.observed_at < *local_head.1.observed_at()
            || self.observed_at < *current_checkpoint.1.issued_at()
            || self.observed_at < *remote.1.observed_at()
            || self.observed_at > *handle.retain_until()
        {
            return Err(A2aCancelTaskError::Time);
        }
        let fresh = [
            &self.diagnostic_ref,
            &self.observation_ref,
            &self.reconciliation.record_ref,
        ];
        let retained = [
            handle_ref,
            local_head.0,
            &request_reference.record_ref,
            route.0,
            binding.0,
            remote.0,
            selected_remote.0,
        ];
        if fresh.iter().enumerate().any(|(index, reference)| {
            retained.contains(reference) || fresh[..index].contains(reference)
        }) {
            return Err(A2aCancelTaskError::Association);
        }
        admit_reconciliation(&self.reconciliation).map_err(A2aCancelTaskError::Authorization)?;
        let diagnostic = cancel_disposition_diagnostic_for_request(
            request_id,
            self.operation,
            A2aCancelDispositionKind::RestartAfterRequestCustody,
        )?;
        if diagnostic.canonical_bytes().len() > self.maximum_diagnostic_bytes {
            return Err(A2aCancelTaskError::Attempt(
                OperationAttemptError::BoundExceeded,
            ));
        }
        let (diagnostic_sha256, diagnostic_record) =
            crate::resident::record_projection::diagnostic_record(
                &self.diagnostic_ref,
                &diagnostic,
            )
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let observation = signing::issue_observation(
            &OperationObservationAdmissionContextV1 {
                signed: self.signing,
                handle_ref,
                handle,
                predecessor: Some(local_head),
            },
            OperationObservationStateV1::OutcomeUnknown {
                diagnostic: ImmutableRecordReferenceV1 {
                    record_ref: self.diagnostic_ref,
                    record_sha256: diagnostic_sha256,
                },
                reconciliation: self.reconciliation,
                effect_receipt: None,
            },
            self.observed_at,
            admit_rotation,
            sign,
        )?;
        Ok(PreparedRecoveredA2aCancelTaskDisposition {
            operation: self.operation,
            current_checkpoint: (current_checkpoint.0.clone(), current_checkpoint.1.clone()),
            predecessor: (local_head.0.clone(), local_head.1.clone()),
            observation_ref: self.observation_ref,
            observation,
            diagnostic_record,
        })
    }
}

impl A2aCancelTaskDispositionPhase {
    /// Borrow the exact local cancellation observation this phase closes.
    pub(in crate::resident) fn local_cancellation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        match self {
            Self::NotStarted(phase) => phase.local_cancellation(),
            Self::OutcomeUnknown(phase) => phase.local_cancellation(),
        }
    }

    fn prepare_disposition<'a, Reconciliation, Rotation, Sign>(
        self,
        input: A2aCancelTaskDispositionInput<'a>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedA2aCancelTaskDisposition<'a>, Box<(Self, A2aCancelTaskError)>>
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
        match self {
            Self::NotStarted(phase) => (*phase)
                .prepare_disposition(input, admit_reconciliation, admit_rotation, sign)
                .map_err(|failure| {
                    let reason = failure.reason;
                    Box::new((Self::NotStarted(failure.phase), reason))
                }),
            Self::OutcomeUnknown(phase) => (*phase)
                .prepare_disposition(input, admit_reconciliation, admit_rotation, sign)
                .map_err(|failure| {
                    let reason = failure.reason;
                    Box::new((Self::OutcomeUnknown(failure.phase), reason))
                }),
        }
    }
}

impl<Phase> A2aCancelTaskDispositionFailure<Phase> {
    /// Borrow the reason without discarding the carrier phase.
    #[must_use]
    pub const fn reason(&self) -> &A2aCancelTaskError {
        &self.reason
    }

    /// Recover the original phase for an explicitly corrected admission attempt.
    #[must_use]
    pub fn into_phase(self) -> Phase {
        *self.phase
    }
}

impl PreparedA2aCancelTaskDisposition<'_> {
    /// Borrow custody of the local cancellation request that this disposition closes.
    #[must_use]
    pub const fn local_custody(&self) -> &crate::resident::CompletedOperationObservationCustody {
        &self.custody
    }

    /// Borrow the admitted diagnostic before physical custody.
    #[must_use]
    pub const fn diagnostic(&self) -> &crate::diagnostic::AdmittedDiagnostic {
        &self.diagnostic
    }

    /// Borrow the signed local uncertainty successor.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Borrow every exact response item retained with this disposition.
    #[must_use]
    pub fn response_evidence(&self) -> &[OperationA2aEvidence] {
        &self.responses
    }

    /// Borrow the authenticated current checkpoint used as the append predecessor.
    #[must_use]
    pub const fn predecessor_checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.current_checkpoint.0, &self.current_checkpoint.1)
    }

    fn into_phase(self) -> A2aCancelTaskDispositionPhase {
        match self.kind {
            A2aCancelDispositionKind::NotStarted(reason) => {
                debug_assert!(self.responses.is_empty());
                A2aCancelTaskDispositionPhase::NotStarted(Box::new(A2aCancelTaskNotStarted {
                    custody: self.custody,
                    context: self.context,
                    reason,
                }))
            }
            A2aCancelDispositionKind::OutcomeUnknown(reason) => {
                A2aCancelTaskDispositionPhase::OutcomeUnknown(Box::new(
                    A2aCancelTaskOutcomeUnknown {
                        custody: self.custody,
                        context: self.context,
                        responses: self.responses,
                        reason,
                    },
                ))
            }
            A2aCancelDispositionKind::RestartAfterRequestCustody => {
                unreachable!("restart-only disposition never becomes a carrier phase")
            }
        }
    }

    /// Prepare diagnostic, observation, and response evidence for one checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an error when lineage, checkpoint roots, store ownership, bounds,
    /// or the physical evidence collection differs from the admitted disposition.
    pub fn prepare_checkpoint<'a>(
        &'a self,
        selection: crate::resident::OperationCheckpointSelection,
        limits: pale_ports::AccumulatorPreparationLimits,
        resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
        custody: &mut dyn pale_ports::OperationCustodyPort,
    ) -> Result<
        crate::resident::PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        let (handle_ref, handle) = self.operation.handle();
        let predecessor = (&self.context.cancellation.0, &self.context.cancellation.1);
        let append = crate::resident::prepare_operation_observation_append(
            crate::resident::OperationObservationTransition::new(
                handle_ref,
                handle,
                predecessor.0,
                predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?
        .with_associated_record(pale_ports::OperationCustodyRecord::Diagnostic(
            self.diagnostic_record.clone(),
        ));
        let append = if self.responses.is_empty() {
            append
        } else {
            append.with_a2a_evidence_items(self.responses.clone())
        };
        append.prepare_checkpoint(
            selection,
            &self.current_checkpoint.0,
            &self.current_checkpoint.1,
            limits,
            custody,
        )
    }
}

impl PreparedRecoveredA2aCancelTaskDisposition<'_> {
    /// Borrow the signed uncertainty successor prepared for physical custody.
    pub(in crate::resident) const fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Borrow the authenticated checkpoint immediately before this successor.
    pub(in crate::resident) const fn predecessor_checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.current_checkpoint.0, &self.current_checkpoint.1)
    }

    /// Prepare the diagnostic and uncertainty observation as one checkpoint atom.
    pub(in crate::resident) fn prepare_checkpoint<'a>(
        &'a self,
        limits: pale_ports::AccumulatorPreparationLimits,
        resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
        custody: &mut dyn pale_ports::OperationCustodyPort,
    ) -> Result<
        crate::resident::PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        let (handle_ref, handle) = self.operation.handle();
        crate::resident::prepare_operation_observation_append(
            crate::resident::OperationObservationTransition::new(
                handle_ref,
                handle,
                &self.predecessor.0,
                &self.predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?
        .with_associated_record(pale_ports::OperationCustodyRecord::Diagnostic(
            self.diagnostic_record.clone(),
        ))
        .prepare_checkpoint(
            self.operation.checkpoint_selection(),
            &self.current_checkpoint.0,
            &self.current_checkpoint.1,
            limits,
            custody,
        )
    }
}

#[derive(Clone, Copy)]
enum A2aCancelDispositionKind {
    NotStarted(A2aExchangeError),
    OutcomeUnknown(A2aCancelTaskUncertainty),
    RestartAfterRequestCustody,
}

type CancelDispositionFailure = Box<(
    crate::resident::CompletedOperationObservationCustody,
    A2aCancelTaskResponseContext,
    Vec<OperationA2aEvidence>,
    A2aCancelTaskError,
)>;

#[allow(clippy::too_many_arguments)]
fn prepare_cancel_disposition<'a, Reconciliation, Rotation, Sign>(
    custody: crate::resident::CompletedOperationObservationCustody,
    context: A2aCancelTaskResponseContext,
    responses: Vec<OperationA2aEvidence>,
    input: A2aCancelTaskDispositionInput<'a>,
    kind: A2aCancelDispositionKind,
    admit_reconciliation: Reconciliation,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<PreparedA2aCancelTaskDisposition<'a>, CancelDispositionFailure>
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
    let result = (|| {
        let local_head = input.history.local_observation().selected_head();
        let current_checkpoint = input.history.local_observation().current_checkpoint();
        let cancellation = (&context.cancellation.0, &context.cancellation.1);
        let request_reference = evidence_reference(&context.request_evidence);
        if !context
            .basis
            .matches_without_local_head(input.operation, input.history)
            || local_head != cancellation
            || !matches!(
                cancellation.1.state(),
                OperationObservationStateV1::CancellationRequested { request }
                    if request == &request_reference
            )
            || input.maximum_diagnostic_bytes == 0
            || responses.len() > 2
            || context
                .request_evidence
                .byte_len()
                .checked_add(responses.iter().map(OperationA2aEvidence::byte_len).sum())
                .is_none_or(|total| total > context.maximum_evidence_bytes)
        {
            return Err(A2aCancelTaskError::Association);
        }
        if input.observed_at < *cancellation.1.observed_at()
            || input.observed_at < *current_checkpoint.1.issued_at()
            || input.observed_at < context.start_authorized_at
            || input.observed_at > context.basis.retain_until()
        {
            return Err(A2aCancelTaskError::Time);
        }
        let fresh = [
            &input.diagnostic_ref,
            &input.observation_ref,
            &input.reconciliation.record_ref,
        ];
        if fresh.iter().enumerate().any(|(index, reference)| {
            context.basis.contains_record_ref(reference)
                || **reference == context.cancellation.0
                || **reference == request_reference.record_ref
                || fresh[..index].contains(reference)
                || responses
                    .iter()
                    .any(|evidence| evidence.coordinate().record_ref() == *reference)
        }) {
            return Err(A2aCancelTaskError::Association);
        }
        admit_reconciliation(&input.reconciliation).map_err(A2aCancelTaskError::Authorization)?;
        let diagnostic = cancel_disposition_diagnostic(&context, input.operation, kind)?;
        if diagnostic.canonical_bytes().len() > input.maximum_diagnostic_bytes {
            return Err(A2aCancelTaskError::Attempt(
                OperationAttemptError::BoundExceeded,
            ));
        }
        let (diagnostic_sha256, diagnostic_record) =
            crate::resident::record_projection::diagnostic_record(
                &input.diagnostic_ref,
                &diagnostic,
            )
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let (handle_ref, handle) = input.operation.handle();
        let observation = signing::issue_observation(
            &OperationObservationAdmissionContextV1 {
                signed: input.signing,
                handle_ref,
                handle,
                predecessor: Some(cancellation),
            },
            OperationObservationStateV1::OutcomeUnknown {
                diagnostic: ImmutableRecordReferenceV1 {
                    record_ref: input.diagnostic_ref.clone(),
                    record_sha256: diagnostic_sha256,
                },
                reconciliation: input.reconciliation,
                effect_receipt: None,
            },
            input.observed_at,
            admit_rotation,
            sign,
        )?;
        Ok((
            diagnostic,
            diagnostic_record,
            observation,
            (current_checkpoint.0.clone(), current_checkpoint.1.clone()),
        ))
    })();
    match result {
        Ok((diagnostic, diagnostic_record, observation, current_checkpoint)) => {
            Ok(PreparedA2aCancelTaskDisposition {
                custody,
                context,
                kind,
                operation: input.operation,
                current_checkpoint,
                observation_ref: input.observation_ref,
                observation,
                diagnostic,
                diagnostic_record,
                responses,
            })
        }
        Err(reason) => Err(Box::new((custody, context, responses, reason))),
    }
}

fn cancel_disposition_diagnostic(
    context: &A2aCancelTaskResponseContext,
    operation: &IssuedLocalOperation,
    kind: A2aCancelDispositionKind,
) -> Result<crate::diagnostic::AdmittedDiagnostic, crate::diagnostic::DiagnosticAdmissionError> {
    cancel_disposition_diagnostic_for_request(&context.request_id, operation, kind)
}

fn cancel_disposition_diagnostic_for_request(
    request_id: &A2aJsonRpcRequestIdV1,
    operation: &IssuedLocalOperation,
    kind: A2aCancelDispositionKind,
) -> Result<crate::diagnostic::AdmittedDiagnostic, crate::diagnostic::DiagnosticAdmissionError> {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1")
        .map_err(crate::diagnostic::DiagnosticAdmissionError::Registry)?;
    let (code, stage, disposition, summary, fact_key, fact_value, effect) = match kind {
        A2aCancelDispositionKind::NotStarted(reason) => (
            "a2a_cancel_not_started",
            DiagnosticStageV1::HostObservation,
            DiagnosticDispositionV1::OperationalFailure,
            "the A2A carrier rejected CancelTask before queue acceptance",
            "carrier_rejection",
            exchange_error_kind(reason),
            DiagnosticEffectCertaintyV1::NotStarted,
        ),
        A2aCancelDispositionKind::OutcomeUnknown(reason) => (
            "a2a_cancel_outcome_unknown",
            DiagnosticStageV1::ResidentRecovery,
            DiagnosticDispositionV1::EffectOutcomeUnknown,
            "the A2A CancelTask outcome is unknown",
            "uncertainty",
            uncertainty_kind(reason),
            DiagnosticEffectCertaintyV1::OutcomeUnknown,
        ),
        A2aCancelDispositionKind::RestartAfterRequestCustody => (
            "a2a_cancel_outcome_unknown",
            DiagnosticStageV1::ResidentRecovery,
            DiagnosticDispositionV1::EffectOutcomeUnknown,
            "the retained A2A CancelTask request has an unknown remote outcome after restart",
            "uncertainty",
            "restart_after_request_custody",
            DiagnosticEffectCertaintyV1::OutcomeUnknown,
        ),
    };
    crate::diagnostic::DiagnosticProjection {
        code: code.to_string(),
        stage,
        disposition,
        summary: summary.to_string(),
        subject: Some((
            "operation".to_string(),
            operation
                .handle()
                .1
                .operation_handle_id()
                .as_str()
                .to_string(),
        )),
        source: None,
        schema_reference: None,
        causes: Vec::new(),
        notes: vec!["Remote Task state is not promoted to a Pale outcome.".to_string()],
        help: vec!["Read back the remote Task before selecting another action.".to_string()],
        facts: vec![(fact_key.to_string(), fact_value.to_string())],
        retry: DiagnosticRetryV1::ReconcileFirst,
        effect,
        request: Some(("a2a_json_rpc".to_string(), request_id.as_str().to_string())),
        generation: None,
        receipt: None,
        omitted_causes: 0,
    }
    .admit(&identity)
}

const fn uncertainty_kind(reason: A2aCancelTaskUncertainty) -> &'static str {
    match reason {
        A2aCancelTaskUncertainty::UnlikeExchangeCoordinate => "unlike_exchange_coordinate",
        A2aCancelTaskUncertainty::MissingResponse => "missing_response",
        A2aCancelTaskUncertainty::Carrier(_) => "carrier_failure",
        A2aCancelTaskUncertainty::UnexpectedResponse => "unexpected_response",
        A2aCancelTaskUncertainty::EvidenceBound => "evidence_bound",
        A2aCancelTaskUncertainty::ProtocolQualification => "protocol_qualification",
        A2aCancelTaskUncertainty::ObservationClock => "observation_clock",
        A2aCancelTaskUncertainty::ObservationTime => "observation_time",
        A2aCancelTaskUncertainty::LocalReadCancelled => "local_read_cancelled",
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

impl CompletedA2aCancelTaskReadback {
    /// Borrow exact request and response evidence.
    #[must_use]
    pub fn evidence(&self) -> (&OperationA2aEvidence, &OperationA2aEvidence) {
        (&self.context.request_evidence, &self.response_evidence)
    }

    /// Borrow the reported remote Task state. This is not local cancellation proof.
    #[must_use]
    pub const fn remote_state(&self) -> &A2aRemoteTaskStateV1 {
        &self.remote_state
    }

    /// Borrow the committed local cancellation observation.
    #[must_use]
    pub fn local_cancellation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.context.cancellation.0, &self.context.cancellation.1)
    }

    /// Borrow physical custody of the local cancellation request.
    #[must_use]
    pub const fn local_custody(&self) -> &crate::resident::CompletedOperationObservationCustody {
        &self.custody
    }

    /// Prepare one predecessor-linked remote Task observation.
    ///
    /// # Errors
    ///
    /// Returns an error when the recovered operation, cancellation request,
    /// remote predecessor, evidence, signature, or authority-store selection is
    /// not the exact history represented by this read-back.
    pub fn prepare_observation<'a, Rotation, Sign, Store>(
        &self,
        input: A2aCancelTaskObservationInput<'a>,
        protocol: &dyn A2aCancelTaskEvidencePort,
        admit_rotation: Rotation,
        sign: Sign,
        resolve_store: Store,
    ) -> Result<PreparedA2aTaskAppend<'a>, A2aCancelTaskError>
    where
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &A2aTaskObservationPreimageV1,
            &A2aTaskObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Store: FnOnce(
            &A2aTaskObservationV1,
        ) -> Result<AuthorityStoreCoordinate, InvalidResidentRecordV1>,
    {
        let local = input.history.local_observation().selected_head();
        let cancellation = self.local_cancellation();
        let handle = input.operation.handle();
        if !self
            .context
            .basis
            .matches_without_local_head(input.operation, input.history)
            || local != cancellation
            || !matches!(
                local.1.state(),
                OperationObservationStateV1::CancellationRequested { request }
                    if request == &evidence_reference(&self.context.request_evidence)
            )
            || input.maximum_evidence_bytes == 0
            || input.maximum_record_bytes == 0
        {
            return Err(A2aCancelTaskError::Association);
        }
        let binding = input.history.binding();
        let predecessor = input.history.observation();
        let request = evidence_reference(&self.context.request_evidence);
        let response = evidence_reference(&self.response_evidence);
        if self
            .context
            .basis
            .contains_record_ref(&input.observation_ref)
            || input.observation_ref == request.record_ref
            || input.observation_ref == response.record_ref
            || input.observation_ref == *local.0
        {
            return Err(A2aCancelTaskError::Association);
        }
        let preimage = A2aTaskObservationPreimageV1 {
            schema_id: SchemaId::parse(A2A_TASK_OBSERVATION_V1_SCHEMA)
                .map_err(|_| A2aCancelTaskError::Association)?,
            schema_sha256: input.signing.expected_schema_sha256.clone(),
            task_binding_id: binding.1.task_binding_id().clone(),
            task_binding_ref: binding.0.clone(),
            task_binding_sha256: binding.1.canonical_a2a_task_binding_sha256().clone(),
            revision: predecessor
                .1
                .revision()
                .checked_successor()
                .ok_or(A2aCancelTaskError::Association)?,
            predecessor_ref: Some(predecessor.0.clone()),
            predecessor_sha256: Some(
                predecessor
                    .1
                    .canonical_a2a_task_observation_sha256()
                    .clone(),
            ),
            source: A2aTaskObservationSourceV1::CancelTaskReadBack {
                request: request.clone(),
                response: response.clone(),
            },
            remote_state: self.remote_state,
            reconciliation: None,
            observed_at: self.observed_at,
            deployment_runtime_authority_ref: handle.1.deployment_runtime_authority_ref().clone(),
            deployment_runtime_authority_sha256: handle
                .1
                .deployment_runtime_authority_sha256()
                .clone(),
            issuer_principal_id: input.signing.signing_authority.principal_id().clone(),
            issuer_key_id: ResidentKeyIdV1::parse(input.signing.signing_authority.signer_key_id())
                .map_err(|_| A2aCancelTaskError::Association)?,
            issuer_key_epoch: *input.signing.signing_authority.signer_key_epoch(),
        };
        let maximum = NonZeroUsize::new(input.maximum_evidence_bytes)
            .ok_or(A2aCancelTaskError::Association)?;
        let request_evidence = self.context.request_evidence.clone();
        let response_evidence = self.response_evidence.clone();
        Ok(prepare_a2a_task_observation_custody(
            A2aTaskObservationCustodyInput {
                operation_custody: input.history.local_observation(),
                operation_observation_ref: local.0,
                observation_ref: input.observation_ref,
                observation: preimage,
                context: A2aTaskObservationAdmissionContextV1 {
                    signed: input.signing,
                    task_binding_ref: binding.0,
                    task_binding: binding.1,
                    route_admission_ref: input.history.route().0,
                    route_admission: input.history.route().1,
                    operation_handle_ref: handle.0,
                    operation_handle: handle.1,
                    operation_observation: local.1,
                    predecessor: Some(predecessor),
                },
                new_evidence: vec![response_evidence],
                retained_evidence: vec![request_evidence],
                maximum_evidence_bytes: input.maximum_evidence_bytes,
                maximum_record_bytes: input.maximum_record_bytes,
            },
            |actual, new, retained| {
                if new.len() != 1 || retained.len() != 1 {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "A2A CancelTask observation",
                        invariant: "one retained request and one new response",
                    });
                }
                let A2aTaskObservationSourceV1::CancelTaskReadBack {
                    request: actual_request,
                    response: actual_response,
                } = &actual.source
                else {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "A2A CancelTask observation",
                        invariant: "CancelTask read-back source",
                    });
                };
                if actual_request != &evidence_reference(&retained[0])
                    || actual_response != &evidence_reference(&new[0])
                {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "A2A CancelTask observation",
                        invariant: "exact request and response evidence",
                    });
                }
                let state = protocol
                    .read_task_state_after_cancel(
                        retained[0].bytes(),
                        new[0].bytes(),
                        binding.1.remote_task_id(),
                        binding.1.remote_context_id(),
                        maximum,
                    )
                    .map_err(|_| InvalidResidentRecordV1::ContextualAdmission {
                        record: "A2A CancelTask observation",
                        invariant: "qualified official protocol evidence",
                    })?;
                if state != actual.remote_state {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "A2A CancelTask observation",
                        invariant: "reported state matches exact response bytes",
                    });
                }
                Ok(())
            },
            admit_rotation,
            sign,
            resolve_store,
        )?)
    }
}

/// Retain one incomplete `CancelTask` disposition without replay authority.
///
/// Every pre-physical failure returns the exact carrier phase. Database and
/// anchor failures retain the issued candidate and the original request
/// custody for exact continuation.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(in crate::resident) fn commit_a2a_cancel_task_disposition_continuable<
    Reconciliation,
    Rotation,
    ObservationSign,
    CheckpointSign,
>(
    phase: A2aCancelTaskDispositionPhase,
    input: A2aCancelTaskDispositionCommitInput<'_>,
    resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
    admit_reconciliation: Reconciliation,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<
    CompletedA2aCancelTaskDispositionCustody,
    ContinuableA2aCancelTaskDispositionCommitFailure,
>
where
    Reconciliation:
        FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
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
    let A2aCancelTaskDispositionCommitInput {
        disposition,
        checkpoint,
        checkpoint_issued_at,
        checkpoint_schema,
        checkpoint_authority,
        accumulator_limits,
    } = input;
    let binding = {
        let (reference, value) = disposition.history.binding();
        (reference.clone(), value.clone())
    };
    let cancellation = {
        let (reference, value) = phase.local_cancellation();
        (
            reference.clone(),
            value.canonical_operation_observation_sha256().clone(),
        )
    };
    let selection = disposition.operation.checkpoint_selection();
    let prepared = match phase.prepare_disposition(
        disposition,
        admit_reconciliation,
        admit_rotation,
        sign_observation,
    ) {
        Ok(prepared) => prepared,
        Err(failure) => {
            let (phase, reason) = *failure;
            return Err(
                ContinuableA2aCancelTaskDispositionCommitFailure::PrePhysical {
                    phase: Box::new(phase),
                    reason,
                },
            );
        }
    };
    let predecessor = {
        let (reference, value) = prepared.predecessor_checkpoint();
        (reference.clone(), value.clone())
    };
    let checkpoint_prepared =
        match prepared.prepare_checkpoint(selection, accumulator_limits, resolve_store, custody) {
            Ok(checkpoint) => checkpoint,
            Err(reason) => {
                return Err(
                    ContinuableA2aCancelTaskDispositionCommitFailure::PrePhysical {
                        phase: Box::new(prepared.into_phase()),
                        reason: A2aCancelTaskError::Projection(reason),
                    },
                );
            }
        };
    let preimage = match crate::resident::operation_dispatch::observation_checkpoint_preimage(
        &checkpoint_prepared,
        (&predecessor.0, &predecessor.1),
        &checkpoint,
        checkpoint_issued_at,
        checkpoint_schema,
        checkpoint_authority,
    ) {
        Ok(preimage) => preimage,
        Err(reason) => {
            drop(checkpoint_prepared);
            return Err(
                ContinuableA2aCancelTaskDispositionCommitFailure::PrePhysical {
                    phase: Box::new(prepared.into_phase()),
                    reason: A2aCancelTaskError::Attempt(reason),
                },
            );
        }
    };
    let checkpoint_ref = checkpoint.reference;
    let (physical, _checkpoint_value) = match checkpoint_prepared.issue_checkpoint(
        &checkpoint_ref,
        preimage,
        checkpoint_schema,
        checkpoint_authority,
        sign_checkpoint,
    ) {
        Ok(value) => value,
        Err(reason) => {
            return Err(
                ContinuableA2aCancelTaskDispositionCommitFailure::PrePhysical {
                    phase: Box::new(prepared.into_phase()),
                    reason: A2aCancelTaskError::Projection(reason),
                },
            );
        }
    };
    let finalizer = Box::new(A2aCancelTaskDispositionFinalizer {
        binding,
        cancellation,
        request_custody: prepared.custody,
    });
    let pending = match physical.commit(custody, anchor) {
        Ok(pending) => pending,
        Err(failure) => {
            return Err(ContinuableA2aCancelTaskDispositionCommitFailure::Commit {
                finalizer,
                failure,
            });
        }
    };
    match pending.advance_anchor(anchor) {
        Ok(completed) => Ok(finalizer.finish(completed)),
        Err(failure) => {
            Err(ContinuableA2aCancelTaskDispositionCommitFailure::Anchor { finalizer, failure })
        }
    }
}

/// Retain one successful `CancelTask` response as remote Task history.
///
/// Every pre-physical failure returns the completed carrier evidence. Database
/// and anchor failures retain the exact issued candidate for continuation.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(in crate::resident) fn commit_a2a_cancel_task_readback_continuable<
    Rotation,
    ObservationSign,
    Store,
    CheckpointSign,
>(
    completed: CompletedA2aCancelTaskReadback,
    input: A2aCancelTaskReadbackCommitInput<'_>,
    protocol: &dyn A2aCancelTaskEvidencePort,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    resolve_store: Store,
    sign_checkpoint: CheckpointSign,
) -> Result<CompletedA2aCancelTaskReadbackCustody, ContinuableA2aCancelTaskReadbackCommitFailure>
where
    Rotation: FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
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
    let A2aCancelTaskReadbackCommitInput {
        observation,
        checkpoint,
        issued_at,
        checkpoint_schema,
        checkpoint_authority,
        accumulator_limits,
        membership_limits,
    } = input;
    let predecessor = {
        let (reference, value) = observation.history.local_observation().current_checkpoint();
        (reference.clone(), value.clone())
    };
    let selection = observation.operation.checkpoint_selection();
    let append = match completed.prepare_observation(
        observation,
        protocol,
        admit_rotation,
        sign_observation,
        resolve_store,
    ) {
        Ok(append) => append,
        Err(reason) => {
            return Err(ContinuableA2aCancelTaskReadbackCommitFailure::PrePhysical {
                completed: Box::new(completed),
                reason,
            });
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
        accumulator_limits,
        membership_limits,
        custody,
    ) {
        Ok(prepared) => prepared,
        Err(reason) => {
            return Err(ContinuableA2aCancelTaskReadbackCommitFailure::PrePhysical {
                completed: Box::new(completed),
                reason: A2aCancelTaskError::TaskCustody(reason),
            });
        }
    };
    let preimage = match super::super::service::checkpoint_preimage(
        &prepared,
        (&predecessor.0, &predecessor.1),
        &checkpoint,
        issued_at,
        checkpoint_schema,
        checkpoint_authority,
    ) {
        Ok(preimage) => preimage,
        Err(reason) => {
            return Err(ContinuableA2aCancelTaskReadbackCommitFailure::PrePhysical {
                completed: Box::new(completed),
                reason: A2aCancelTaskError::TaskCustody(reason),
            });
        }
    };
    let checkpoint_ref = checkpoint.reference;
    let checkpoint_preimage = preimage.clone();
    let (physical, checkpoint_value) = match prepared.issue_checkpoint_with_value(
        &checkpoint_ref,
        preimage,
        checkpoint_schema,
        checkpoint_authority,
        sign_checkpoint,
    ) {
        Ok(value) => value,
        Err(reason) => {
            return Err(ContinuableA2aCancelTaskReadbackCommitFailure::PrePhysical {
                completed: Box::new(completed),
                reason: A2aCancelTaskError::TaskCustody(reason),
            });
        }
    };
    let checkpoint = (checkpoint_ref, checkpoint_value);
    let finalizer = Box::new(A2aCancelTaskReadbackFinalizer {
        binding,
        observation,
        checkpoint_preimage,
    });
    let pending = match physical.commit(custody, anchor) {
        Ok(pending) => pending,
        Err(failure) => {
            return Err(ContinuableA2aCancelTaskReadbackCommitFailure::Commit {
                finalizer,
                checkpoint: Box::new(checkpoint),
                failure,
            });
        }
    };
    let completed = match pending.advance_anchor(anchor) {
        Ok(completed) => completed,
        Err(failure) => {
            return Err(ContinuableA2aCancelTaskReadbackCommitFailure::Anchor {
                finalizer,
                checkpoint: Box::new(checkpoint),
                failure,
            });
        }
    };
    Ok(finalizer.finish(checkpoint, completed))
}

fn evidence_reference(evidence: &OperationA2aEvidence) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: evidence.coordinate().record_ref().clone(),
        record_sha256: ArtifactDigest::hash_bytes(evidence.bytes()),
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/resident/operation_observation_custody/a2a_exchange/cancel_readback.rs"]
mod tests;
