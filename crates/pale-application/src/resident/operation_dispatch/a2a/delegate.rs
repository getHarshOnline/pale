//! Start one durable A2A delegation and retain its wake-driven response phase.

use super::*;
use crate::resident::operation_dispatch::{custody::sample_time, OperationCheckpointInput};
use pale_ports::{
    A2aSendMessageEvidencePort, AccumulatorMembershipLimits, AccumulatorPreparationLimits,
    AntiRollbackAnchorPort, OperationCustodyPort, ProviderInvocationRequest, WallClockSourcePort,
};
use std::task::Context;

/// Exact inputs for retaining and starting one A2A `Delegate` request.
pub struct A2aDelegateLaunchInput<'operation, 'call> {
    /// Admitted provider dispatch, route, request text, and finite carrier limits.
    pub send: A2aSendInput<'operation, 'call>,
    /// Fresh checkpoint identity for the exact retained request bytes.
    pub checkpoint: OperationCheckpointInput,
    /// Exact checkpoint schema selected independently of this request.
    pub checkpoint_schema: &'call SchemaDigest,
    /// Current checkpoint-signing authority.
    pub checkpoint_authority: &'call ControlCheckpointSigningAuthorityAdmissionContextV1<'call>,
    /// Fresh immutable identity reserved for the first response item.
    pub response_ref: ResidentRecordRefV1,
    /// Selected record-time source. It does not establish clock trust.
    pub clock: &'call dyn WallClockSourcePort,
    /// Finite accumulator preparation limits for request custody.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Finite retained-membership limits for the selected route.
    pub membership_limits: AccumulatorMembershipLimits,
}

/// A stopped launch phase. No variant permits replay after carrier acceptance.
pub enum A2aDelegateLaunchFailure<'operation, 'call> {
    /// Request preparation failed before any database or carrier effect.
    Preparation(A2aSendPreparationError),
    /// Checkpoint preparation or signing failed before database commitment.
    Checkpoint(A2aSendCheckpointError),
    /// Database or independent-anchor custody remains under its exact owner.
    Custody(Box<A2aSendCustodyFailure<'operation, 'call>>),
    /// The selected time source failed before any request custody was prepared.
    PreCustodyClock(OperationAttemptError),
    /// Request custody is anchored, but release time could not be established.
    ReleaseClock {
        /// Exact unopened request custody. The carrier has not started.
        retained: Box<RetainedA2aSend<'operation, 'call>>,
        /// Closed time-source or time-ordering failure.
        reason: OperationAttemptError,
    },
    /// Release stopped before or after the one allowed carrier start.
    Start(A2aSendStartFailure<'operation, 'call>),
}

impl std::fmt::Debug for A2aDelegateLaunchFailure<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Preparation(_) => "preparation",
            Self::Checkpoint(_) => "checkpoint",
            Self::Custody(_) => "custody",
            Self::PreCustodyClock(_) => "pre_custody_clock",
            Self::ReleaseClock { .. } => "release_clock",
            Self::Start(_) => "start",
        };
        formatter
            .debug_struct("A2aDelegateLaunchFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// One started delegation awaiting its first response or exact unary EOF.
#[must_use = "poll or cancel the started A2A delegation"]
pub enum AwaitingA2aDelegate {
    /// The exchange has not produced its first response item.
    Response(AwaitingA2aSendResponse),
    /// One response item is retained; exact unary EOF is still required.
    Eof(ReceivedA2aSendResponse),
}

impl AwaitingA2aDelegate {
    pub(in crate::resident) fn task_key(
        &self,
    ) -> Result<crate::resident::task_owner::ResidentOperationTaskKey, OperationAttemptError> {
        match self {
            Self::Response(response) => response.task_key(),
            Self::Eof(response) => response.task_key(),
        }
    }

    /// Borrow the exact started exchange identity.
    #[must_use]
    pub fn coordinate(&self) -> &pale_ports::A2aExchangeCoordinate {
        match self {
            Self::Response(response) => response.coordinate(),
            Self::Eof(response) => response.coordinate(),
        }
    }

    /// Borrow the exact send-preparation custody without granting replay.
    #[must_use]
    pub const fn send_custody(&self) -> &crate::resident::CompletedOperationObservationCustody {
        match self {
            Self::Response(response) => response.send_custody(),
            Self::Eof(response) => response.send_custody(),
        }
    }

    /// Borrow the checkpoint that retained the exact request bytes.
    #[must_use]
    pub fn send_checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        match self {
            Self::Response(response) => response.send_checkpoint(),
            Self::Eof(response) => response.send_checkpoint(),
        }
    }

    /// Poll one wake-driven transition without reconnecting or replaying.
    pub fn poll(
        self,
        cx: &mut Context<'_>,
        protocol: &dyn A2aSendMessageEvidencePort,
        clock: &dyn WallClockSourcePort,
    ) -> A2aSendResponsePoll<Self, CompletedA2aSendResponse, A2aSendResponseFailure> {
        match self {
            Self::Response(response) => match response.poll_response(cx, protocol) {
                A2aSendResponsePoll::Pending(response) => {
                    A2aSendResponsePoll::Pending(Self::Response(response))
                }
                A2aSendResponsePoll::Ready(Ok(response)) => poll_eof(response, cx, clock),
                A2aSendResponsePoll::Ready(Err(failure)) => {
                    A2aSendResponsePoll::Ready(Err(failure))
                }
            },
            Self::Eof(response) => poll_eof(response, cx, clock),
        }
    }

    /// Stop local reads and return a non-replayable uncertain outcome.
    #[must_use = "record the uncertain outcome before reconciliation"]
    pub fn cancel(self) -> A2aSendResponseFailure {
        match self {
            Self::Response(response) => response.cancel(),
            Self::Eof(response) => response.cancel(),
        }
    }
}

/// Retain exact request bytes, establish both custody layers, and start once.
///
/// Preparation performs no network I/O. The carrier starts only after a new
/// database commit and exact independent-anchor advance. The returned value is
/// wake-driven and owns no store or anchor borrow across `Pending`.
///
/// # Errors
///
/// Returns the exact stopped phase. A failure after carrier acceptance never
/// returns request replay permission.
#[allow(clippy::too_many_arguments)]
pub fn launch_a2a_delegate<'operation, 'call, Authority, Disclosure, Transport, CheckpointSign>(
    input: A2aDelegateLaunchInput<'operation, 'call>,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    exchange: &dyn A2aExchangePort,
    mut admit_disclosure: Disclosure,
    admit_transport: Transport,
    sign_checkpoint: CheckpointSign,
) -> Result<AwaitingA2aDelegate, A2aDelegateLaunchFailure<'operation, 'call>>
where
    Authority: AmbientSessionAuthorityPort,
    Disclosure: FnMut(
        &ProviderInvocationRequest<'_>,
        &A2aRouteAdmissionV1,
        &str,
    ) -> Result<(), InvalidResidentRecordV1>,
    Transport: FnOnce(
        &ImmutableRecordReferenceV1,
        &ProviderInvocationRequest<'_>,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    CheckpointSign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let original_current = input.send.current;
    let prepared = prepare_a2a_send(
        input.send,
        authority,
        custody,
        anchor,
        exchange,
        |request, route, text| admit_disclosure(request, route, text),
        admit_transport,
    )
    .map_err(A2aDelegateLaunchFailure::Preparation)?;
    let issued_at = sample_time(input.clock, original_current.now)
        .map_err(A2aDelegateLaunchFailure::PreCustodyClock)?;
    let prepared = prepared
        .prepare_custody(
            input.checkpoint,
            issued_at,
            input.checkpoint_schema,
            input.checkpoint_authority,
            input.accumulator_limits,
            input.membership_limits,
            custody,
            sign_checkpoint,
        )
        .map_err(A2aDelegateLaunchFailure::Checkpoint)?;
    let pending = prepared
        .commit(custody, anchor)
        .map_err(A2aDelegateLaunchFailure::Custody)?;
    let retained = pending
        .advance_anchor(anchor)
        .map_err(A2aDelegateLaunchFailure::Custody)?;
    let release_time = match sample_time(input.clock, issued_at) {
        Ok(release_time) => release_time,
        Err(reason) => {
            return Err(A2aDelegateLaunchFailure::ReleaseClock {
                retained: Box::new(retained),
                reason,
            });
        }
    };
    let current = original_current.at_time(release_time);
    let response = retained
        .start_awaiting(
            &current,
            authority,
            custody,
            anchor,
            exchange,
            input.response_ref,
            |request, route, text| admit_disclosure(request, route, text),
        )
        .map_err(A2aDelegateLaunchFailure::Start)?;
    Ok(AwaitingA2aDelegate::Response(response))
}

fn poll_eof(
    response: ReceivedA2aSendResponse,
    cx: &mut Context<'_>,
    clock: &dyn WallClockSourcePort,
) -> A2aSendResponsePoll<AwaitingA2aDelegate, CompletedA2aSendResponse, A2aSendResponseFailure> {
    match response.poll_eof(cx, clock) {
        A2aSendResponsePoll::Pending(response) => {
            A2aSendResponsePoll::Pending(AwaitingA2aDelegate::Eof(response))
        }
        A2aSendResponsePoll::Ready(result) => A2aSendResponsePoll::Ready(result),
    }
}
