//! Retain an unopened native call after a read-only release refusal.

use super::super::cancellation::{
    NativeCancellationPoint, NativeOperationCancellationInput, PreparedNativeOperationCancellation,
};
use super::*;

mod expiry;
pub use expiry::{
    NativeOperationExpiryInput, NativeOperationExpiryRefusal, PreparedNativeOperationExpiry,
};

/// Current cancellation authority and bounded reads for an unopened retained call.
/// Original dispatch custody comes only from the refusal, not this input.
pub struct NativeOperationRefusalCancellationInput<'a> {
    /// Current authority of the same original admitted session.
    pub current: &'a AmbientSessionAdmissionContext<'a>,
    /// Independently admitted observation signer for that runtime.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh reference for the cancellation diagnostic.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Fresh reference for the cancelled observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Diagnostic byte ceiling checked before observation signing.
    pub maximum_diagnostic_bytes: usize,
    /// Per-record ceiling for current dispatch-custody reads.
    pub maximum_record_bytes: usize,
    /// Existing time source; supplying it does not establish clock trust.
    pub clock: &'a dyn WallClockSourcePort,
    /// Existing owner of the operation's storage coordinate.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
}

#[cfg(test)]
#[path = "../../../../../tests/internal/resident/operation_dispatch_selection/refusal_time.rs"]
mod tests;

/// The handler has not run. Its original call and dispatch confirmation stay in memory.
///
/// This is neither a retry recommendation nor durable recovery evidence. It
/// cannot reconstruct a call after a crash or upgrade replay/uncertain custody.
/// Re-entry consumes this value, so it cannot release the same call twice.
#[must_use = "retain this refusal for inspection or an explicit checked re-entry"]
pub struct NativeOperationInvocationRefusal<'operation, 'registry> {
    retained: Box<RefusedRelease<'operation, 'registry>>,
}

struct RefusedRelease<'operation, 'registry> {
    release: NativeRelease<'operation, 'registry>,
    reason: OperationAttemptError,
}

/// Failed cancellation preparation with the original unopened call still retained.
/// The cancellation reason is separate from the original invocation refusal.
#[must_use = "inspect the failure and retain or explicitly recover the original refusal"]
pub struct NativeOperationCancellationRefusal<'operation, 'registry> {
    retained: Box<(
        OperationAttemptError,
        NativeOperationInvocationRefusal<'operation, 'registry>,
    )>,
}

impl std::fmt::Debug for NativeOperationCancellationRefusal<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("NativeOperationCancellationRefusal { retained: .. }")
    }
}

impl std::fmt::Display for NativeOperationCancellationRefusal<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("native cancellation was not prepared")
    }
}

impl std::error::Error for NativeOperationCancellationRefusal<'_, '_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.retained.0)
    }
}

impl<'operation, 'registry> NativeOperationCancellationRefusal<'operation, 'registry> {
    /// Inspect the typed cancellation failure without exposing the retained call.
    #[must_use]
    pub fn reason(&self) -> &OperationAttemptError {
        &self.retained.0
    }

    /// Recover the same refusal with its complete, possibly closed time history.
    /// This grants no retry permission or renewed execution budget.
    pub fn into_invocation_refusal(
        self,
    ) -> NativeOperationInvocationRefusal<'operation, 'registry> {
        self.retained.1
    }
}

pub(super) struct NativeRelease<'operation, 'registry> {
    pub(super) dispatch: PreparedNativeOperationDispatch<'operation, 'registry>,
    pub(super) completed: CompletedOperationObservationCustody,
    pub(super) started: Instant,
    pub(super) time: ReleaseTime,
}

pub(super) enum ReleaseTime {
    Unobserved {
        floor: UnixTimeV1,
    },
    Observed {
        floor: UnixTimeV1,
        greatest: UnixTimeV1,
    },
    Closed {
        floor: UnixTimeV1,
        greatest: Option<UnixTimeV1>,
    },
}

impl ReleaseTime {
    fn coordinates(&self) -> (UnixTimeV1, Option<UnixTimeV1>) {
        match *self {
            Self::Unobserved { floor } => (floor, None),
            Self::Observed { floor, greatest } => (floor, Some(greatest)),
            Self::Closed { floor, greatest } => (floor, greatest),
        }
    }

    fn retain_floor(&mut self, now: UnixTimeV1) {
        let (Self::Unobserved { floor }
        | Self::Observed { floor, .. }
        | Self::Closed { floor, .. }) = self;
        *floor = (*floor).max(now);
    }

    fn close(&mut self) {
        let (floor, greatest) = self.coordinates();
        *self = Self::Closed { floor, greatest };
    }
}

impl std::fmt::Debug for NativeOperationInvocationRefusal<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("NativeOperationInvocationRefusal { retained: .. }")
    }
}

impl std::fmt::Display for NativeOperationInvocationRefusal<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("native operation was not released")
    }
}

impl std::error::Error for NativeOperationInvocationRefusal<'_, '_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.retained.reason)
    }
}

impl<'operation, 'registry> NativeOperationInvocationRefusal<'operation, 'registry> {
    /// Inspect the original typed failure without exposing the retained call.
    #[must_use]
    pub fn reason(&self) -> &OperationAttemptError {
        &self.retained.reason
    }

    /// Prepare terminal cancellation without releasing the retained handler.
    ///
    /// Rechecks the original session, request and lease and the original new
    /// dispatch custody. The signed observation follows that exact dispatch,
    /// not its earlier preparation. Failure returns the original refusal with
    /// its updated time history. Success permanently discards the unopened
    /// handler: no release-capable call can coexist with a detached cancellation
    /// append. Preparation performs bounded reads but no writes or execution.
    ///
    /// Cancellation needs current same-session authority; this path does not
    /// grant control after execution-authority expiry or across sessions. The
    /// handler's elapsed execution budget does not prevent cancellation, and
    /// cancellation never reopens a closed release clock. Three clock reads
    /// retain the original time floor and high-water mark, including on refusal.
    /// Current authority is rechecked after custody reads; a final time check
    /// catches expiry during those checks without renewing execution authority.
    /// The final append must still compare its exact predecessor and advance
    /// the independent anchor before cancellation can be reported as committed.
    /// Sequential read-back is not a cross-process lock.
    ///
    /// # Errors
    /// Rejects stale authority, replay or uncertain original custody, changed
    /// heads or anchors, wrong signer, reused references, bounds and signing
    /// failure. No refusal is converted to an automatic retry or native effect.
    pub fn prepare_cancellation<Authority, Rotation, Sign>(
        mut self,
        input: NativeOperationRefusalCancellationInput<'_>,
        authority: &mut Authority,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<
        PreparedNativeOperationCancellation<'operation>,
        NativeOperationCancellationRefusal<'operation, 'registry>,
    >
    where
        Authority: AmbientSessionAuthorityPort,
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
        let prepared = (|| {
            let release = &mut self.retained.release;
            release.time.retain_floor(input.current.now);
            let before = release.observe_time(input.clock)?;
            release.validate_current(&input.current.at_time(before), authority)?;
            release.dispatch.attempt.context.require_context_ready()?;
            release
                .dispatch
                .prepare_append(input.resolve_store)?
                .verify_fresh_dispatch_custody(
                    &release.completed,
                    custody,
                    anchor,
                    input.maximum_record_bytes,
                )?;
            let now = release.observe_time(input.clock)?;
            release.validate_current(&input.current.at_time(now), authority)?;
            let recorded_at = release.observe_time(input.clock)?;
            let current = input.current.at_time(recorded_at);
            if let Err(error) = release.dispatch.attempt.context.validate_time_at(&current) {
                release.time.close();
                return Err(error);
            }
            let dispatch = &release.dispatch;
            dispatch.attempt.prepare_cancellation_evidence(
                dispatch.observation(),
                NativeCancellationPoint::BeforeRelease,
                NativeOperationCancellationInput {
                    completed: &release.completed,
                    current: &current,
                    signing: input.signing,
                    diagnostic_ref: input.diagnostic_ref,
                    observation_ref: input.observation_ref,
                    maximum_diagnostic_bytes: input.maximum_diagnostic_bytes,
                    resolve_store: input.resolve_store,
                },
                admit_rotation,
                sign,
            )
        })();
        prepared.map_err(|reason| NativeOperationCancellationRefusal {
            retained: Box::new((reason, self)),
        })
    }

    /// Explicitly repeat release checks using the same unopened call and custody.
    ///
    /// All authority, exact new custody, anchor and time checks run again. No
    /// database commit or anchor advance occurs. The first monotonic start and
    /// greatest observed wall time survive re-entry. An observed temporal
    /// refusal remains closed, even if another clock or context is supplied.
    /// No error is classified as an automatic retry or effect disposition.
    ///
    /// # Errors
    /// Returns retained refusal on failed checks, including replay and recovered
    /// uncertainty. A completed handler instead returns its terminal outcome;
    /// neither that outcome nor this API can recover its consumed call.
    pub fn recheck(
        self,
        input: NativeOperationExecutionContext<'_>,
        authority: &mut impl AmbientSessionAuthorityPort,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<UnstagedNativeOperationOutcome, Self> {
        if matches!(self.retained.release.time, ReleaseTime::Closed { .. }) {
            return Err(self);
        }
        let RefusedRelease { release, .. } = *self.retained;
        release.run(input, authority, custody, anchor)
    }
}

impl<'operation, 'registry> NativeRelease<'operation, 'registry> {
    pub(super) fn run(
        mut self,
        input: NativeOperationExecutionContext<'_>,
        authority: &mut impl AmbientSessionAuthorityPort,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<
        UnstagedNativeOperationOutcome,
        NativeOperationInvocationRefusal<'operation, 'registry>,
    > {
        match self.validate(input, authority, custody, anchor) {
            Ok(release_time) => Ok(self.dispatch.execute(release_time)),
            Err(reason) => Err(NativeOperationInvocationRefusal {
                retained: Box::new(RefusedRelease {
                    release: self,
                    reason,
                }),
            }),
        }
    }

    fn observe_time(
        &mut self,
        clock: &dyn WallClockSourcePort,
    ) -> Result<UnixTimeV1, OperationAttemptError> {
        let observed = clock.observe_unix_time()?;
        let (floor, previous) = self.time.coordinates();
        if observed < floor || previous.is_some_and(|greatest| observed < greatest) {
            self.time.close();
            return Err(OperationAttemptError::TimeOrder);
        }
        self.time = match self.time {
            ReleaseTime::Closed { .. } => ReleaseTime::Closed {
                floor,
                greatest: Some(observed),
            },
            _ => ReleaseTime::Observed {
                floor,
                greatest: observed,
            },
        };
        Ok(observed)
    }

    fn validate_current(
        &mut self,
        current: &AmbientSessionAdmissionContext<'_>,
        authority: &mut impl AmbientSessionAuthorityPort,
    ) -> Result<(), OperationAttemptError> {
        let context = &self.dispatch.attempt.context;
        let result = context.validate_current_at(current, authority);
        if result.is_err() && context.has_temporal_refusal(current) {
            // The owning pure time checks identify temporal refusal without
            // copying their laws or interpreting a generic association error.
            self.time.close();
        }
        result
    }

    fn validate(
        &mut self,
        input: NativeOperationExecutionContext<'_>,
        authority: &mut impl AmbientSessionAuthorityPort,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<UnixTimeV1, OperationAttemptError> {
        // Supplied lower bounds survive even when the next clock read fails.
        // They never substitute for an actual clock observation.
        self.time.retain_floor(input.current.now);
        self.check_deadline()?;
        let before = self.observe_time(input.clock)?;
        self.validate_current(&input.current.at_time(before), authority)?;
        self.dispatch.attempt.context.require_context_ready()?;
        self.dispatch
            .prepare_append(input.resolve_store)?
            .verify_fresh_dispatch_custody(
                &self.completed,
                custody,
                anchor,
                input.maximum_record_bytes,
            )?;
        let now = self.observe_time(input.clock)?;
        let current = input.current.at_time(now);
        self.validate_current(&current, authority)?;
        signing::validate_runtime_signer(
            self.dispatch.attempt.selection.result_authority(),
            &current,
            CAPABILITY_RESULT_V1_SCHEMA,
        )?;
        let release_time = self.observe_time(input.clock)?;
        if let Err(error) = self
            .dispatch
            .attempt
            .context
            .validate_time_at(&current.at_time(release_time))
        {
            self.time.close();
            return Err(error);
        }
        self.check_deadline()?;
        Ok(release_time)
    }

    fn check_deadline(&mut self) -> Result<(), OperationAttemptError> {
        if self.started.elapsed()
            >= Duration::from_millis(self.dispatch.attempt.context.lease.timeout_millis().get())
        {
            self.time.close();
            return Err(OperationAttemptError::Deadline);
        }
        Ok(())
    }
}
