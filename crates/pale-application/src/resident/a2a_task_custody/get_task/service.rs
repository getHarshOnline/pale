//! One authorized unary `GetTask` read-back over retained task identity.

use super::super::{
    prepare_a2a_task_observation_custody, recovery::RecoveredA2aTaskHistory, A2aTaskCustodyError,
    A2aTaskObservationCustodyInput, KnownA2aTaskBasis, PreparedA2aTaskAppend,
};
use crate::resident::{
    CompletedOperationObservationCustody, IssuedLocalOperation, OperationAttemptError,
    OperationAttemptHistoryInput, OperationCheckpointInput,
    OperationObservationCustodyAnchorFailure, OperationObservationCustodyCommitFailure,
    OperationObservationCustodyProjectionError,
};
use pale_core::records::{
    A2aJsonRpcRequestIdV1, A2aRemoteTaskStateV1, A2aTaskBindingV1,
    A2aTaskObservationAdmissionContextV1, A2aTaskObservationDigest, A2aTaskObservationPreimageV1,
    A2aTaskObservationSourceV1, A2aTaskObservationV1, ArtifactDigest, CanonicalSignatureV1,
    ControlCheckpointSigningAuthorityAdmissionContextV1, ControlStateCheckpointPreimageV1,
    ControlStateCheckpointV1, ImmutableRecordReferenceV1, InvalidResidentRecordV1,
    OperationHandleV1, OperationObservationV1, ResidentKeyIdV1, ResidentRecordRefV1, SchemaDigest,
    SignedResidentAdmissionContextV1, UnixTimeV1, A2A_TASK_OBSERVATION_V1_SCHEMA,
};
use pale_ports::{
    A2aExchange, A2aExchangeCommand, A2aExchangeCoordinate, A2aExchangeError, A2aExchangeFacts,
    A2aExchangeLimits, A2aExchangeParents, A2aExchangePort, A2aExchangeRequest,
    A2aGetTaskEvidencePort, A2aPreparedExchange, AccumulatorMembershipInputError,
    AccumulatorMembershipLimits, AccumulatorPreparationLimits, AntiRollbackAnchorPort,
    AuthorityStoreCoordinate, OperationA2aEvidence, OperationCustodyCandidateError,
    OperationCustodyPort, WallClockSourceError, WallClockSourcePort,
};
use std::task::{Context, Poll};

/// Exact current facts presented to the selected read-authorization owner.
///
/// Historical custody establishes each record's meaning. It does not grant the
/// caller access now. The callback receiving this value must resolve the current
/// caller, disclosure ceiling, revocation state, and any owner-native policy.
pub struct A2aGetTaskReadbackAuthorizationContext<'a> {
    /// Durable operation locator and record.
    pub operation: (&'a ResidentRecordRefV1, &'a OperationHandleV1),
    /// Exact historical provider dispatch behind the binding.
    pub dispatch: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    /// Anchored local head at recovery time.
    pub local_head: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    /// Historical route. Its expiry neither grants nor denies retained read-back.
    pub route: (
        &'a ResidentRecordRefV1,
        &'a pale_core::records::A2aRouteAdmissionV1,
    ),
    /// Known remote task identity.
    pub binding: (&'a ResidentRecordRefV1, &'a A2aTaskBindingV1),
    /// Current admitted remote-history head before this read-back.
    pub remote_head: (&'a ResidentRecordRefV1, &'a A2aTaskObservationV1),
    /// Current authorization time, also fenced by retained history.
    pub authorized_at: UnixTimeV1,
}

/// Inputs for one authorized read of an already identified remote task.
pub struct A2aGetTaskReadbackInput<'a> {
    /// Fully admitted remote and local history. This is not current access.
    pub history: &'a RecoveredA2aTaskHistory,
    /// Exact private provider selection retained by the original dispatch.
    pub attempt: OperationAttemptHistoryInput<'a>,
    /// Fresh caller-assigned JSON-RPC identity for this read only.
    pub request_id: &'a A2aJsonRpcRequestIdV1,
    /// Fresh immutable reference reserved for the exact request bytes.
    pub request_record_ref: ResidentRecordRefV1,
    /// Fresh immutable reference reserved for the exact response bytes.
    pub response_record_ref: ResidentRecordRefV1,
    /// One bounded exchange profile.
    pub limits: A2aExchangeLimits,
    /// Aggregate retained request/response byte ceiling.
    pub maximum_evidence_bytes: usize,
}

/// Existing owners needed to turn completed bytes into a signed successor.
pub struct A2aGetTaskObservationInput<'a> {
    /// The same admitted history used for current authorization.
    pub history: &'a RecoveredA2aTaskHistory,
    /// Durable operation record governing retention and local context.
    pub operation: &'a IssuedLocalOperation,
    /// Fresh immutable reference for the signed successor.
    pub observation_ref: ResidentRecordRefV1,
    /// Exact schema and current runtime signing authority.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Aggregate exact request/response byte ceiling.
    pub maximum_evidence_bytes: usize,
    /// Maximum signed observation bytes before custody preparation.
    pub maximum_record_bytes: usize,
}

/// Authorized inputs before request construction or adapter preparation.
#[must_use = "authorization alone does not prepare or start a network exchange"]
pub(super) struct AuthorizedA2aGetTaskReadback<'a> {
    pub(super) history: &'a RecoveredA2aTaskHistory,
    pub(super) attempt: crate::resident::operation_dispatch::RecoveredProviderAttempt<'a>,
    pub(super) basis: KnownA2aTaskBasis,
    pub(super) request_id: &'a A2aJsonRpcRequestIdV1,
    pub(super) request_record_ref: ResidentRecordRefV1,
    pub(super) response_record_ref: ResidentRecordRefV1,
    pub(super) limits: A2aExchangeLimits,
    pub(super) maximum_evidence_bytes: usize,
}

/// Prepared request bytes. No network request has started.
#[must_use = "prepared GetTask bytes have not been sent"]
pub(super) struct PreparedA2aGetTaskReadback {
    pub(super) basis: KnownA2aTaskBasis,
    pub(super) prepared: A2aPreparedExchange,
    pub(super) request_evidence: OperationA2aEvidence,
    pub(super) response_record_ref: ResidentRecordRefV1,
    pub(super) expected_task: pale_core::records::A2aRemoteTaskIdV1,
    pub(super) expected_context: pale_core::records::A2aRemoteContextIdV1,
    pub(super) maximum_evidence_bytes: usize,
}

/// Exactly one started exchange awaiting its first and only response item.
#[must_use = "the started GetTask exchange must be polled or explicitly cancelled"]
pub struct InFlightA2aGetTaskReadback {
    basis: KnownA2aTaskBasis,
    exchange: Box<dyn A2aExchange>,
    expected_coordinate: A2aExchangeCoordinate,
    request_evidence: OperationA2aEvidence,
    response_record_ref: ResidentRecordRefV1,
    expected_task: pale_core::records::A2aRemoteTaskIdV1,
    expected_context: pale_core::records::A2aRemoteContextIdV1,
    maximum_evidence_bytes: usize,
}

/// One exact Task response received; EOF is still required.
#[must_use = "a unary response is incomplete until the exchange reaches EOF"]
pub struct ReceivedA2aGetTaskReadback {
    basis: KnownA2aTaskBasis,
    exchange: Box<dyn A2aExchange>,
    request_evidence: OperationA2aEvidence,
    response_evidence: OperationA2aEvidence,
    remote_state: A2aRemoteTaskStateV1,
}

/// Exact request and response evidence after unary EOF.
#[must_use = "completed network evidence still requires observation admission and custody"]
pub struct CompletedA2aGetTaskReadback {
    basis: KnownA2aTaskBasis,
    request_evidence: OperationA2aEvidence,
    response_evidence: OperationA2aEvidence,
    remote_state: A2aRemoteTaskStateV1,
    observed_at: UnixTimeV1,
}

/// One wake-driven phase transition without hiding the retained pending state.
pub enum A2aGetTaskReadbackPoll<Pending, Ready> {
    /// No evidence is ready; poll this same value after the supplied wake.
    Pending(Pending),
    /// The phase advanced or ended with a closed error.
    Ready(Result<Ready, A2aGetTaskReadbackError>),
}

/// Failure before remote evidence becomes an admitted, durable observation.
#[derive(Debug, thiserror::Error)]
pub enum A2aGetTaskReadbackError {
    /// Current caller or disclosure policy denied the read.
    #[error(transparent)]
    Authorization(#[from] InvalidResidentRecordV1),
    /// Historical private provider selection did not match the admitted dispatch.
    #[error(transparent)]
    Attempt(#[from] OperationAttemptError),
    /// Handle retention or required historical time ordering was violated.
    #[error("A2A GetTask observation time is outside the retained history interval")]
    Time,
    /// The selected wall-clock source could not produce a portable observation.
    #[error(transparent)]
    Clock(#[from] WallClockSourceError),
    /// Fresh evidence identities or retained history were substituted.
    #[error("A2A GetTask read-back inputs do not match retained history")]
    Association,
    /// Mechanical exchange preparation, start, or response failed.
    #[error(transparent)]
    Exchange(#[from] A2aExchangeError),
    /// Exact request or response bytes could not enter the selected bound.
    #[error(transparent)]
    Evidence(#[from] OperationCustodyCandidateError),
    /// The existing signed-observation owner rejected preparation.
    #[error(transparent)]
    Custody(#[from] A2aTaskCustodyError),
    /// Unary `GetTask` yielded no Task, extra output, SSE, or unlike identity.
    #[error("A2A GetTask did not produce exactly one matching unary Task response")]
    Response,
}

impl From<OperationObservationCustodyProjectionError> for A2aGetTaskReadbackError {
    fn from(value: OperationObservationCustodyProjectionError) -> Self {
        Self::Custody(A2aTaskCustodyError::Projection(value))
    }
}

impl From<AccumulatorMembershipInputError> for A2aGetTaskReadbackError {
    fn from(value: AccumulatorMembershipInputError) -> Self {
        Self::Custody(A2aTaskCustodyError::Membership(value))
    }
}

/// Existing owners and fresh identities for one completed `GetTask` history append.
pub struct A2aGetTaskCommitInput<'a> {
    /// Exact recovered history, operation, signer, and fresh observation identity.
    pub observation: A2aGetTaskObservationInput<'a>,
    /// Fresh immutable identity for the checkpoint retaining the successor.
    pub checkpoint: OperationCheckpointInput,
    /// Trusted checkpoint issuance time.
    pub issued_at: UnixTimeV1,
    /// Independently selected checkpoint schema.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Current checkpoint-signing authority.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Finite accumulator preparation ceilings.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Finite retained-membership proof ceilings.
    pub membership_limits: AccumulatorMembershipLimits,
}

/// Durable `GetTask` successor and its physical custody proof.
///
/// The remote state remains owner-native history. It is not a Pale result,
/// completion, effect receipt, or settlement.
#[must_use = "retain the remote-history custody result"]
pub struct CompletedA2aGetTaskReadbackCustody {
    binding: (ResidentRecordRefV1, A2aTaskBindingV1),
    observation: (ResidentRecordRefV1, A2aTaskObservationV1),
    checkpoint_preimage: ControlStateCheckpointPreimageV1,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: CompletedOperationObservationCustody,
}

pub(in crate::resident) struct A2aGetTaskFinalizer {
    binding: (ResidentRecordRefV1, A2aTaskBindingV1),
    observation: (ResidentRecordRefV1, A2aTaskObservationV1),
    checkpoint_preimage: ControlStateCheckpointPreimageV1,
}

impl A2aGetTaskFinalizer {
    fn finish(
        self,
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
        custody: CompletedOperationObservationCustody,
    ) -> CompletedA2aGetTaskReadbackCustody {
        CompletedA2aGetTaskReadbackCustody {
            binding: self.binding,
            observation: self.observation,
            checkpoint_preimage: self.checkpoint_preimage,
            checkpoint,
            custody,
        }
    }
}

impl CompletedA2aGetTaskReadbackCustody {
    /// Borrow the unchanged durable remote Task binding.
    #[must_use]
    pub const fn binding(&self) -> (&ResidentRecordRefV1, &A2aTaskBindingV1) {
        (&self.binding.0, &self.binding.1)
    }

    /// Borrow the exact predecessor-linked remote-history successor.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &A2aTaskObservationV1) {
        (&self.observation.0, &self.observation.1)
    }

    /// Borrow the exact checkpoint fields admitted before signing.
    #[must_use]
    pub const fn checkpoint_preimage(&self) -> &ControlStateCheckpointPreimageV1 {
        &self.checkpoint_preimage
    }

    /// Borrow the checkpoint retaining the `GetTask` successor.
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

/// Stopped `GetTask` observation custody, separated by the last completed phase.
pub enum A2aGetTaskCommitFailure {
    /// Observation admission or signing failed before root preparation.
    Observation {
        /// Exact completed request/response evidence; no write was attempted.
        completed: Box<CompletedA2aGetTaskReadback>,
        /// Closed semantic or projection failure.
        reason: A2aGetTaskReadbackError,
    },
    /// Accumulator or membership preparation failed before checkpoint issuance.
    Checkpoint {
        /// Exact completed request/response evidence; no write was attempted.
        completed: Box<CompletedA2aGetTaskReadback>,
        /// Closed preparation failure.
        reason: A2aTaskCustodyError,
    },
    /// Checkpoint construction, admission, or signing failed before commitment.
    Issue {
        /// Exact completed request/response evidence; no write was attempted.
        completed: Box<CompletedA2aGetTaskReadback>,
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

pub(in crate::resident) enum ContinuableA2aGetTaskCommitFailure {
    PrePhysical(A2aGetTaskCommitFailure),
    Commit {
        finalizer: Box<A2aGetTaskFinalizer>,
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        failure: OperationObservationCustodyCommitFailure,
    },
    Anchor {
        finalizer: Box<A2aGetTaskFinalizer>,
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        failure: OperationObservationCustodyAnchorFailure,
    },
}

impl ContinuableA2aGetTaskCommitFailure {
    pub(in crate::resident) fn into_public(self) -> A2aGetTaskCommitFailure {
        match self {
            Self::PrePhysical(failure) => failure,
            Self::Commit {
                checkpoint,
                failure,
                ..
            } => A2aGetTaskCommitFailure::Commit {
                checkpoint,
                failure,
            },
            Self::Anchor {
                checkpoint,
                failure,
                ..
            } => A2aGetTaskCommitFailure::Anchor {
                checkpoint,
                failure,
            },
        }
    }

    pub(in crate::resident) fn binding(&self) -> Option<(&ResidentRecordRefV1, &A2aTaskBindingV1)> {
        match self {
            Self::PrePhysical(_) => None,
            Self::Commit { finalizer, .. } | Self::Anchor { finalizer, .. } => {
                Some((&finalizer.binding.0, &finalizer.binding.1))
            }
        }
    }

    /// Recover completed carrier evidence only from a pre-physical failure.
    pub(in crate::resident) fn into_completed(self) -> Result<CompletedA2aGetTaskReadback, Self> {
        let Self::PrePhysical(failure) = self else {
            return Err(self);
        };
        match failure {
            A2aGetTaskCommitFailure::Observation { completed, .. }
            | A2aGetTaskCommitFailure::Checkpoint { completed, .. }
            | A2aGetTaskCommitFailure::Issue { completed, .. } => Ok(*completed),
            failure => Err(Self::PrePhysical(failure)),
        }
    }

    pub(in crate::resident) fn continue_custody(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedA2aGetTaskReadbackCustody, Self> {
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

impl std::fmt::Debug for A2aGetTaskCommitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Observation { .. } => "observation",
            Self::Checkpoint { .. } => "checkpoint",
            Self::Issue { .. } => "issue",
            Self::Commit { .. } => "commit",
            Self::Anchor { .. } => "anchor",
        };
        formatter
            .debug_struct("A2aGetTaskCommitFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Require current read permission before preparing or starting an exchange.
///
/// Historical route expiry is intentionally not checked here. The task binding
/// already fixes the provider and route used to identify the task; current
/// permission comes only from `authorize`. This function performs no I/O.
///
/// # Errors
///
/// Returns an error when retained records disagree, observation time is outside
/// the durable interval, current access is denied, or the private selection is
/// not the one admitted for the historical dispatch.
pub(super) fn authorize_a2a_get_task_readback<'a, Authorize>(
    input: A2aGetTaskReadbackInput<'a>,
    authorized_at: UnixTimeV1,
    authorize: Authorize,
) -> Result<AuthorizedA2aGetTaskReadback<'a>, A2aGetTaskReadbackError>
where
    Authorize:
        FnOnce(A2aGetTaskReadbackAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
{
    let operation_owner = input.attempt.operation;
    let operation = operation_owner.handle();
    let dispatch = input.history.dispatch();
    let local_head = input.history.local_observation().selected_head();
    let remote_head = input.history.observation();
    let selected_remote = input.history.selected_observation();
    let binding = input.history.binding();
    let basis = KnownA2aTaskBasis::capture(
        operation_owner,
        input.attempt.selection,
        input.history,
        authorized_at,
    );
    if input.attempt.dispatched != dispatch.1
        || binding.1.operation_handle_ref() != operation.0
        || binding.1.operation_handle_sha256() != operation.1.canonical_operation_handle_sha256()
        || remote_head.0 != selected_remote.0
        || remote_head.1.canonical_a2a_task_observation_sha256()
            != selected_remote.1.canonical_a2a_task_observation_sha256()
        || input.request_record_ref == input.response_record_ref
        || basis.contains_record_ref(&input.request_record_ref)
        || basis.contains_record_ref(&input.response_record_ref)
        || input.maximum_evidence_bytes == 0
    {
        return Err(A2aGetTaskReadbackError::Association);
    }
    if authorized_at < *remote_head.1.observed_at()
        || authorized_at < *local_head.1.observed_at()
        || authorized_at > *operation.1.retain_until()
        || !basis.admits_exchange_deadline(authorized_at, input.limits.total_timeout)
    {
        return Err(A2aGetTaskReadbackError::Time);
    }
    authorize(A2aGetTaskReadbackAuthorizationContext {
        operation,
        dispatch,
        local_head,
        route: input.history.route(),
        binding: input.history.binding(),
        remote_head,
        authorized_at,
    })?;
    let attempt = input.attempt.recover_provider_attempt()?;
    Ok(AuthorizedA2aGetTaskReadback {
        history: input.history,
        attempt,
        basis,
        request_id: input.request_id,
        request_record_ref: input.request_record_ref,
        response_record_ref: input.response_record_ref,
        limits: input.limits,
        maximum_evidence_bytes: input.maximum_evidence_bytes,
    })
}

/// Authorize, prepare, and start one `GetTask` read-back exactly once.
///
/// This entry only composes the existing phase owners. The returned in-flight
/// value owns the started exchange and can move between wake-driven polls. No
/// history borrow, current authority, mutable port, signer, or custody handle is
/// retained across [`Poll::Pending`].
/// The clock is sampled inside this call, immediately before authorization and
/// the one available prepare/start path.
///
/// # Errors
///
/// Returns the existing authorization, attempt, preparation, or start failure.
/// Once the carrier is called, this function never returns a replayable prepared
/// request.
pub fn start_a2a_get_task_readback<Authorize>(
    input: A2aGetTaskReadbackInput<'_>,
    port: &dyn A2aExchangePort,
    clock: &dyn WallClockSourcePort,
    authorize: Authorize,
) -> Result<InFlightA2aGetTaskReadback, A2aGetTaskReadbackError>
where
    Authorize:
        FnOnce(A2aGetTaskReadbackAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
{
    let authorized_at = clock.observe_unix_time()?;
    authorize_a2a_get_task_readback(input, authorized_at, authorize)?
        .prepare(port)?
        .start(port)
}

impl AuthorizedA2aGetTaskReadback<'_> {
    /// Serialize the exact known-binding request without network I/O.
    ///
    /// # Errors
    ///
    /// Returns an error when the historical provider request cannot be rebuilt,
    /// the protocol rejects the request, or its exact bytes exceed the bound.
    pub(super) fn prepare(
        self,
        port: &dyn A2aExchangePort,
    ) -> Result<PreparedA2aGetTaskReadback, A2aGetTaskReadbackError> {
        let binding = self.history.binding().1;
        let request = A2aExchangeRequest::new(
            self.attempt.provider_request()?,
            A2aExchangeParents {
                operation: self.attempt.operation().handle(),
                observation: self.history.local_observation().selected_head(),
                route: self.history.route(),
            },
            self.request_id,
            A2aExchangeCommand::GetTask(binding),
            self.limits,
        )?;
        let expected_context = request.coordinate().clone();
        let prepared = port.prepare(request)?;
        if prepared.coordinate().context() != &expected_context {
            return Err(A2aGetTaskReadbackError::Association);
        }
        let request_evidence = OperationA2aEvidence::new(
            ImmutableRecordReferenceV1 {
                record_ref: self.request_record_ref,
                record_sha256: pale_core::records::ArtifactDigest::hash_bytes(prepared.bytes()),
            },
            prepared.bytes().to_vec(),
            None,
            self.maximum_evidence_bytes,
        )?;
        Ok(PreparedA2aGetTaskReadback {
            basis: self.basis,
            prepared,
            request_evidence,
            response_record_ref: self.response_record_ref,
            expected_task: binding.remote_task_id().clone(),
            expected_context: binding.remote_context_id().clone(),
            maximum_evidence_bytes: self.maximum_evidence_bytes,
        })
    }
}

impl PreparedA2aGetTaskReadback {
    /// Start the already prepared bytes exactly once.
    ///
    /// # Errors
    ///
    /// Returns an error when the carrier cannot start or substitutes the exact
    /// prepared exchange coordinate.
    pub(super) fn start(
        self,
        port: &dyn A2aExchangePort,
    ) -> Result<InFlightA2aGetTaskReadback, A2aGetTaskReadbackError> {
        let expected_coordinate = self.prepared.coordinate().clone();
        let mut exchange = port.start(self.prepared)?;
        if exchange.coordinate() != &expected_coordinate {
            exchange.cancel();
            return Err(A2aGetTaskReadbackError::Association);
        }
        Ok(InFlightA2aGetTaskReadback {
            basis: self.basis,
            exchange,
            expected_coordinate,
            request_evidence: self.request_evidence,
            response_record_ref: self.response_record_ref,
            expected_task: self.expected_task,
            expected_context: self.expected_context,
            maximum_evidence_bytes: self.maximum_evidence_bytes,
        })
    }
}

impl InFlightA2aGetTaskReadback {
    /// Poll the first response without retrying, reconnecting, or signing.
    pub fn poll_response(
        mut self,
        cx: &mut Context<'_>,
    ) -> A2aGetTaskReadbackPoll<Self, ReceivedA2aGetTaskReadback> {
        if self.exchange.coordinate() != &self.expected_coordinate {
            self.exchange.cancel();
            return A2aGetTaskReadbackPoll::Ready(Err(A2aGetTaskReadbackError::Association));
        }
        let evidence = match self.exchange.poll_next(cx) {
            Poll::Pending => return A2aGetTaskReadbackPoll::Pending(self),
            Poll::Ready(None) => {
                return A2aGetTaskReadbackPoll::Ready(Err(A2aGetTaskReadbackError::Response));
            }
            Poll::Ready(Some(Err(error))) => {
                return A2aGetTaskReadbackPoll::Ready(Err(error.into()));
            }
            Poll::Ready(Some(Ok(evidence))) => evidence,
        };
        let A2aExchangeFacts::Task {
            task,
            context,
            state,
        } = evidence.facts
        else {
            self.exchange.cancel();
            return A2aGetTaskReadbackPoll::Ready(Err(A2aGetTaskReadbackError::Response));
        };
        if task != self.expected_task || context != self.expected_context || evidence.sse.is_some()
        {
            self.exchange.cancel();
            return A2aGetTaskReadbackPoll::Ready(Err(A2aGetTaskReadbackError::Response));
        }
        let response_evidence = match OperationA2aEvidence::new(
            ImmutableRecordReferenceV1 {
                record_ref: self.response_record_ref,
                record_sha256: pale_core::records::ArtifactDigest::hash_bytes(&evidence.bytes),
            },
            evidence.bytes,
            None,
            self.maximum_evidence_bytes
                .saturating_sub(self.request_evidence.byte_len()),
        ) {
            Ok(value) => value,
            Err(error) => {
                self.exchange.cancel();
                return A2aGetTaskReadbackPoll::Ready(Err(error.into()));
            }
        };
        A2aGetTaskReadbackPoll::Ready(Ok(ReceivedA2aGetTaskReadback {
            basis: self.basis,
            exchange: self.exchange,
            request_evidence: self.request_evidence,
            response_evidence,
            remote_state: state,
        }))
    }

    /// Stop local reads. This sends no A2A `CancelTask` request.
    pub fn cancel(mut self) {
        self.exchange.cancel();
    }
}

impl ReceivedA2aGetTaskReadback {
    /// Require exact unary EOF and sample response-observation time there.
    ///
    /// The selected wall-clock source is called only after exact EOF. It does
    /// not establish monotonicity; the retained history supplies the time fences.
    pub fn poll_eof(
        mut self,
        cx: &mut Context<'_>,
        clock: &dyn WallClockSourcePort,
    ) -> A2aGetTaskReadbackPoll<Self, CompletedA2aGetTaskReadback> {
        match self.exchange.poll_next(cx) {
            Poll::Pending => A2aGetTaskReadbackPoll::Pending(self),
            Poll::Ready(None) => {
                let observed_at = match clock.observe_unix_time() {
                    Ok(value) => value,
                    Err(error) => {
                        return A2aGetTaskReadbackPoll::Ready(Err(error.into()));
                    }
                };
                if !self.basis.admits_observation_time(observed_at) {
                    return A2aGetTaskReadbackPoll::Ready(Err(A2aGetTaskReadbackError::Time));
                }
                A2aGetTaskReadbackPoll::Ready(Ok(CompletedA2aGetTaskReadback {
                    basis: self.basis,
                    request_evidence: self.request_evidence,
                    response_evidence: self.response_evidence,
                    remote_state: self.remote_state,
                    observed_at,
                }))
            }
            Poll::Ready(Some(_)) => {
                self.exchange.cancel();
                A2aGetTaskReadbackPoll::Ready(Err(A2aGetTaskReadbackError::Response))
            }
        }
    }

    /// Stop local EOF reads after retaining the received response in memory.
    /// This sends no A2A `CancelTask` request and creates no observation.
    pub fn cancel(mut self) {
        self.exchange.cancel();
    }
}

impl CompletedA2aGetTaskReadback {
    /// Borrow exact bytes for observation admission and atomic custody.
    pub fn evidence(&self) -> (&OperationA2aEvidence, &OperationA2aEvidence) {
        (&self.request_evidence, &self.response_evidence)
    }
    /// Borrow the reported owner-native state. This is not Pale settlement.
    pub const fn remote_state(&self) -> &A2aRemoteTaskStateV1 {
        &self.remote_state
    }
    /// Return the selected observation time.
    pub const fn observed_at(&self) -> UnixTimeV1 {
        self.observed_at
    }
    /// Build the signed-successor input through existing record owners.
    ///
    /// This does not write, commit, anchor, retry the network, or classify the
    /// remote state as a Pale result. A later signing or custody failure must
    /// retain the completed evidence instead of starting another exchange.
    ///
    /// # Errors
    ///
    /// Returns an error when history association, protocol evidence, signing,
    /// store selection, or observation-custody admission fails.
    pub fn prepare_observation<'a, Rotation, Sign, Store>(
        &self,
        input: A2aGetTaskObservationInput<'a>,
        protocol: &dyn A2aGetTaskEvidencePort,
        admit_rotation: Rotation,
        sign: Sign,
        resolve_store: Store,
    ) -> Result<PreparedA2aTaskAppend<'a>, A2aGetTaskReadbackError>
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
        let handle = input.operation.handle();
        if !self
            .basis
            .matches_without_local_head(input.operation, input.history)
        {
            return Err(A2aGetTaskReadbackError::Association);
        }
        let binding = input.history.binding();
        let predecessor = input.history.observation();
        let local = input.history.local_observation().selected_head();
        if binding.1.operation_handle_ref() != handle.0
            || binding.1.operation_handle_sha256() != handle.1.canonical_operation_handle_sha256()
            || input.maximum_evidence_bytes == 0
            || input.maximum_record_bytes == 0
        {
            return Err(A2aGetTaskReadbackError::Association);
        }
        let request = evidence_reference(&self.request_evidence);
        let response = evidence_reference(&self.response_evidence);
        if self.basis.contains_record_ref(&input.observation_ref)
            || input.observation_ref == request.record_ref
            || input.observation_ref == response.record_ref
        {
            return Err(A2aGetTaskReadbackError::Association);
        }
        let preimage = A2aTaskObservationPreimageV1 {
            schema_id: pale_core::records::SchemaId::parse(A2A_TASK_OBSERVATION_V1_SCHEMA)
                .map_err(|_| A2aGetTaskReadbackError::Association)?,
            schema_sha256: input.signing.expected_schema_sha256.clone(),
            task_binding_id: binding.1.task_binding_id().clone(),
            task_binding_ref: binding.0.clone(),
            task_binding_sha256: binding.1.canonical_a2a_task_binding_sha256().clone(),
            revision: predecessor
                .1
                .revision()
                .checked_successor()
                .ok_or(A2aGetTaskReadbackError::Association)?,
            predecessor_ref: Some(predecessor.0.clone()),
            predecessor_sha256: Some(
                predecessor
                    .1
                    .canonical_a2a_task_observation_sha256()
                    .clone(),
            ),
            source: A2aTaskObservationSourceV1::GetTask { request, response },
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
                .map_err(|_| A2aGetTaskReadbackError::Association)?,
            issuer_key_epoch: *input.signing.signing_authority.signer_key_epoch(),
        };
        let evidence = vec![
            self.request_evidence.clone(),
            self.response_evidence.clone(),
        ];
        let maximum_evidence_bytes = input.maximum_evidence_bytes;
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
                new_evidence: evidence,
                retained_evidence: vec![],
                maximum_evidence_bytes,
                maximum_record_bytes: input.maximum_record_bytes,
            },
            |preimage, new, retained| {
                if !retained.is_empty() {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "A2A GetTask observation",
                        invariant: "fresh request and response evidence",
                    });
                }
                let selected = new.iter().collect::<Vec<_>>();
                super::admit_a2a_get_task_evidence(
                    binding,
                    preimage,
                    &selected,
                    std::num::NonZeroUsize::new(maximum_evidence_bytes).ok_or(
                        InvalidResidentRecordV1::ContextualAdmission {
                            record: "A2A GetTask observation",
                            invariant: "positive evidence byte ceiling",
                        },
                    )?,
                    protocol,
                )
            },
            admit_rotation,
            sign,
            resolve_store,
        )?)
    }
}

/// Admit and durably retain one completed `GetTask` response as remote history.
///
/// This service reuses the completed phase's exact request and response bytes,
/// preserves the recovered binding and predecessor, and closes the existing
/// checkpoint, database, and independent-anchor custody phases. It performs no
/// carrier call and never promotes remote Task state to a Pale result or
/// settlement.
///
/// # Errors
///
/// Before commitment, the failure returns the exact completed network evidence.
/// Physical failures retain their existing database or anchor recovery owner and
/// never grant another carrier start.
#[allow(clippy::too_many_arguments)]
pub fn commit_a2a_get_task_readback<Rotation, ObservationSign, Store, CheckpointSign>(
    completed: CompletedA2aGetTaskReadback,
    input: A2aGetTaskCommitInput<'_>,
    protocol: &dyn A2aGetTaskEvidencePort,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    resolve_store: Store,
    sign_checkpoint: CheckpointSign,
) -> Result<CompletedA2aGetTaskReadbackCustody, A2aGetTaskCommitFailure>
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
        &pale_core::records::ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let predecessor = {
        let (reference, checkpoint) = input
            .observation
            .history
            .local_observation()
            .current_checkpoint();
        (reference.clone(), checkpoint.clone())
    };
    commit_a2a_get_task_readback_continuable(
        completed,
        input,
        &predecessor,
        None,
        protocol,
        custody,
        anchor,
        admit_rotation,
        sign_observation,
        resolve_store,
        sign_checkpoint,
    )
    .map_err(ContinuableA2aGetTaskCommitFailure::into_public)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(in crate::resident) fn commit_a2a_get_task_readback_continuable<
    Rotation,
    ObservationSign,
    Store,
    CheckpointSign,
>(
    completed: CompletedA2aGetTaskReadback,
    input: A2aGetTaskCommitInput<'_>,
    physical_predecessor: &(ResidentRecordRefV1, ControlStateCheckpointV1),
    physical_predecessor_custody: Option<&CompletedOperationObservationCustody>,
    protocol: &dyn A2aGetTaskEvidencePort,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    resolve_store: Store,
    sign_checkpoint: CheckpointSign,
) -> Result<CompletedA2aGetTaskReadbackCustody, ContinuableA2aGetTaskCommitFailure>
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
        &pale_core::records::ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    // A durable prepared request supplies its exact physical predecessor and
    // custody. The older in-memory path has no request custody, so its response
    // must follow the current admitted local checkpoint instead of a checkpoint
    // that may have been superseded while the carrier was active.
    let current_predecessor = physical_predecessor_custody.is_none().then(|| {
        let (reference, checkpoint) = input
            .observation
            .history
            .local_observation()
            .current_checkpoint();
        (reference.clone(), checkpoint.clone())
    });
    let physical_predecessor = current_predecessor.as_ref().unwrap_or(physical_predecessor);
    let A2aGetTaskCommitInput {
        observation: observation_input,
        checkpoint,
        issued_at,
        checkpoint_schema,
        checkpoint_authority,
        accumulator_limits,
        membership_limits,
    } = input;
    let selection = observation_input.operation.checkpoint_selection();
    let append = match completed.prepare_observation(
        observation_input,
        protocol,
        admit_rotation,
        sign_observation,
        resolve_store,
    ) {
        Ok(append) => append,
        Err(reason) => {
            return Err(ContinuableA2aGetTaskCommitFailure::PrePhysical(
                A2aGetTaskCommitFailure::Observation {
                    completed: Box::new(completed),
                    reason,
                },
            ));
        }
    };
    let binding = (append.binding().0.clone(), append.binding().1.clone());
    let observation = (
        append.observation().0.clone(),
        append.observation().1.clone(),
    );
    let prepared_result = match physical_predecessor_custody {
        Some(completed) => append.prepare_checkpoint_after_custody(
            selection,
            &physical_predecessor.0,
            &physical_predecessor.1,
            completed,
            accumulator_limits,
            membership_limits,
            custody,
        ),
        None => append.prepare_checkpoint(
            selection,
            &physical_predecessor.0,
            &physical_predecessor.1,
            accumulator_limits,
            membership_limits,
            custody,
        ),
    };
    let prepared = match prepared_result {
        Ok(prepared) => prepared,
        Err(reason) => {
            return Err(ContinuableA2aGetTaskCommitFailure::PrePhysical(
                A2aGetTaskCommitFailure::Checkpoint {
                    completed: Box::new(completed),
                    reason,
                },
            ));
        }
    };
    let preimage = match super::super::service::checkpoint_preimage(
        &prepared,
        (&physical_predecessor.0, &physical_predecessor.1),
        &checkpoint,
        issued_at,
        checkpoint_schema,
        checkpoint_authority,
    ) {
        Ok(preimage) => preimage,
        Err(reason) => {
            return Err(ContinuableA2aGetTaskCommitFailure::PrePhysical(
                A2aGetTaskCommitFailure::Issue {
                    completed: Box::new(completed),
                    reason,
                },
            ));
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
            return Err(ContinuableA2aGetTaskCommitFailure::PrePhysical(
                A2aGetTaskCommitFailure::Issue {
                    completed: Box::new(completed),
                    reason,
                },
            ));
        }
    };
    let checkpoint = (checkpoint_ref, checkpoint_value);
    let finalizer = Box::new(A2aGetTaskFinalizer {
        binding,
        observation,
        checkpoint_preimage,
    });
    let pending = match physical.commit(custody, anchor) {
        Ok(pending) => pending,
        Err(failure) => {
            return Err(ContinuableA2aGetTaskCommitFailure::Commit {
                finalizer,
                checkpoint: Box::new(checkpoint),
                failure,
            });
        }
    };
    let custody = match pending.advance_anchor(anchor) {
        Ok(completed) => completed,
        Err(failure) => {
            return Err(ContinuableA2aGetTaskCommitFailure::Anchor {
                finalizer,
                checkpoint: Box::new(checkpoint),
                failure,
            });
        }
    };
    Ok(finalizer.finish(checkpoint, custody))
}

fn evidence_reference(evidence: &OperationA2aEvidence) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: evidence.coordinate().record_ref().clone(),
        record_sha256: ArtifactDigest::hash_bytes(evidence.bytes()),
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/resident/operation_observation_custody/a2a_exchange/readback.rs"]
mod tests;
