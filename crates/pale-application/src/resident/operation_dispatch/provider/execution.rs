//! Release one retained `HostNative` computation and record admitted terminal evidence.

use super::super::custody::sample_time;
use super::*;
use crate::diagnostic::{
    project_failure, AdmittedDiagnostic, CauseProjection, FailureSpecification,
};
use crate::provider_invocation::{
    AdmittedProviderInvocationOutcome, PreparedProviderInvocation,
    ProviderInvocationAdmissionContext, ProviderInvocationAdmissionError,
    ProviderInvocationEvidenceAdmission,
};
use crate::resident::operation_dispatch::CompletedRecoveredDispatchCustody;
use crate::resident::record_projection;
use crate::resident::{
    prepare_operation_observation_append, CompletedOperationObservationCustody,
    DatabaseCommittedOperationObservationAnchorPending, OperationCheckpointSelection,
    OperationCustodyDatabaseFailureReason, OperationObservationAuthorityStoreResolver,
    OperationObservationCustodyAnchorFailure, OperationObservationCustodyCommitFailure,
    OperationObservationCustodyCommitFailureReason, OperationObservationTransition,
    PreparedOperationObservationCheckpoint, RecoveredOperationObservation,
};
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchorPort, AuthorityStoreCoordinate, ControlRecord,
    ControlStoreErrorKind, OperationCustodyIndexMutation, OperationCustodyIndexPredecessor,
    OperationCustodyPort, OperationCustodyRecord, OperationCustodyRecordKind,
    ProviderInvocationCoordinate, ProviderInvocationObservation, ProviderInvocationPort,
    ProviderInvocationRequest, WallClockSourcePort,
};
use std::time::{Duration, Instant};

/// Current authority, terminal identities, and custody inputs for one `HostNative` call.
pub struct ProviderOperationInvokeInput<'a> {
    /// Current independently resolved session authority.
    pub current: &'a AmbientSessionAdmissionContext<'a>,
    /// Independently selected observation signer at terminal recording time.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh checkpoint identity for the successor observation.
    pub terminal_checkpoint: OperationCheckpointInput,
    /// Fresh immutable result reference.
    pub result_ref: ResidentRecordRefV1,
    /// Fresh diagnostic reference used by a safe failure or uncertain outcome.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Existing reconciliation frontier used only when the call has no
    /// conclusive admitted outcome.
    pub reconciliation: ReconciliationFrontierReferenceV1,
    /// Fresh successor observation reference.
    pub terminal_ref: ResidentRecordRefV1,
    /// Exact checkpoint schema selected by the resident.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Resolved checkpoint signer for the new checkpoint.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Selected wall-clock source; this is not itself clock trust.
    pub clock: &'a dyn WallClockSourcePort,
    /// Owner of the operation observation store coordinate.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    /// Maximum bytes for each release-custody read.
    pub maximum_record_bytes: usize,
    /// Maximum canonical bytes for a projected provider-failure diagnostic.
    pub maximum_diagnostic_bytes: usize,
    /// Existing accumulator preparation limits.
    pub accumulator_limits: AccumulatorPreparationLimits,
}

/// Owned mechanical projection of one already admitted provider request.
///
/// This value clones only the coordinates already present in
/// [`ProviderInvocationRequest`]. It carries no session, custody, signing,
/// evidence-admission, or replay authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::resident) struct OwnedProviderInvocationRequest {
    compiled_generation_digest: CompiledGenerationDigest,
    instance_digest: InstanceDigest,
    operation: PackageResourceRefV1,
    provider_binding: ProviderBindingV1,
    request_binding: RequestBindingV1,
    attempt_id: CapabilityAttemptIdV1,
    implementation_kind: BoundedResidentTextV1,
    implementation_version: BoundedResidentTextV1,
}

impl OwnedProviderInvocationRequest {
    pub(in crate::resident) fn capture(request: ProviderInvocationRequest<'_>) -> Self {
        Self {
            compiled_generation_digest: request.compiled_generation_digest().clone(),
            instance_digest: request.instance_digest().clone(),
            operation: request.operation().clone(),
            provider_binding: request.provider_binding().clone(),
            request_binding: request.request_binding().clone(),
            attempt_id: request.attempt_id().clone(),
            implementation_kind: request.implementation_kind().clone(),
            implementation_version: request.implementation_version().clone(),
        }
    }

    /// Reconstruct the existing borrowed adapter request.
    pub(in crate::resident) fn borrowed(
        &self,
    ) -> Result<ProviderInvocationRequest<'_>, ProviderInvocationAdmissionError> {
        ProviderInvocationRequest::new(
            ProviderInvocationCoordinate::new(
                &self.compiled_generation_digest,
                &self.instance_digest,
                &self.operation,
                &self.request_binding,
                &self.attempt_id,
            ),
            &self.provider_binding,
            &self.implementation_kind,
            &self.implementation_version,
        )
        .map_err(ProviderInvocationAdmissionError::from)
    }
}

/// Unopened application basis retained until a mechanical executor accepts one call.
pub(in crate::resident) struct ProviderOperationCallBasis<'operation> {
    retained: RetainedProviderOperationDispatch<'operation>,
    released_at: UnixTimeV1,
}

/// Owned coordinate retained after a mechanical executor accepts one call.
///
/// The coordinate identifies the exact durable dispatch and its release time.
/// It contains no borrowed operation phase, provider-call permission, storage
/// authority, or recording authority. A completion must rejoin protected
/// operation history before it can be admitted or recorded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::resident) struct ProviderOperationContinuationCoordinate {
    store: pale_ports::AuthorityStoreCoordinate,
    dispatch: ImmutableRecordReferenceV1,
    released_at: UnixTimeV1,
}

impl ProviderOperationContinuationCoordinate {
    /// Borrow the exact durable dispatch selected for this accepted call.
    #[must_use]
    pub(in crate::resident) const fn dispatch(&self) -> &ImmutableRecordReferenceV1 {
        &self.dispatch
    }

    /// Borrow the exact store incarnation that owns the durable dispatch.
    #[must_use]
    pub(in crate::resident) const fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the wall-clock release time checked before mechanical execution.
    #[must_use]
    pub(in crate::resident) const fn released_at(&self) -> UnixTimeV1 {
        self.released_at
    }

    fn matches(&self, recovered: &RecoveredOperationObservation) -> bool {
        if &self.store != recovered.store() {
            return false;
        }
        let selected = recovered.selected();
        let Ok(record_sha256) =
            ArtifactDigest::parse(selected.1.canonical_operation_observation_sha256().as_str())
        else {
            return false;
        };
        self.dispatch.record_ref == *selected.0 && self.dispatch.record_sha256 == record_sha256
    }
}

/// Provider call that passed every application-owned check available before I/O.
#[must_use = "submit the prepared call once or retain its unopened dispatch"]
pub(in crate::resident) struct PreparedProviderOperationCall<'operation> {
    basis: ProviderOperationCallBasis<'operation>,
    request: OwnedProviderInvocationRequest,
}

type ProviderOperationTaskParts<'operation> = (
    pale_ports::AuthorityStoreCoordinate,
    ImmutableRecordReferenceV1,
    ProviderOperationContinuationCoordinate,
    ProviderOperationCallBasis<'operation>,
    OwnedProviderInvocationRequest,
);
type ProviderOperationTaskPartsFailure<'operation> = (
    Box<PreparedProviderOperationCall<'operation>>,
    OperationAttemptError,
);

impl<'operation> PreparedProviderOperationCall<'operation> {
    /// Separate application basis from mechanical work for executor acceptance.
    pub(in crate::resident) fn into_task_parts(
        self,
    ) -> Result<ProviderOperationTaskParts<'operation>, ProviderOperationTaskPartsFailure<'operation>>
    {
        let (reference, observation) = self.basis.retained.observation();
        let record_sha256 = match ArtifactDigest::parse(
            observation
                .canonical_operation_observation_sha256()
                .as_str(),
        ) {
            Ok(record_sha256) => record_sha256,
            Err(_) => return Err((Box::new(self), OperationAttemptError::Encoding)),
        };
        let key = ImmutableRecordReferenceV1 {
            record_ref: reference.clone(),
            record_sha256,
        };
        let store = self.basis.retained.custody().candidate().store().clone();
        let continuation = ProviderOperationContinuationCoordinate {
            store: store.clone(),
            dispatch: key.clone(),
            released_at: self.basis.released_at,
        };
        Ok((store, key, continuation, self.basis, self.request))
    }

    /// Restore an unopened call after mechanical executor refusal.
    pub(in crate::resident) const fn from_task_parts(
        basis: ProviderOperationCallBasis<'operation>,
        request: OwnedProviderInvocationRequest,
    ) -> Self {
        Self { basis, request }
    }
}

/// Linear same-process release after a recovered preparation reached a new dispatch.
///
/// This value exists only when the process that reconstructed `Dispatched`
/// also retained its exact new database-and-anchor custody. Historical
/// `Dispatched` recovery cannot construct it and therefore cannot replay the
/// provider call.
#[must_use = "prepare this recovered provider call once or retain its release token"]
pub(in crate::resident) struct RecoveredProviderDispatchRelease {
    dispatch: PreparedRecoveredProviderDispatch,
    custody: CompletedRecoveredDispatchCustody,
}

impl RecoveredProviderDispatchRelease {
    pub(in crate::resident) fn new(
        dispatch: PreparedRecoveredProviderDispatch,
        custody: CompletedRecoveredDispatchCustody,
    ) -> Result<
        Self,
        Box<(
            PreparedRecoveredProviderDispatch,
            CompletedRecoveredDispatchCustody,
        )>,
    > {
        if dispatch.observation() != custody.observation() {
            return Err(Box::new((dispatch, custody)));
        }
        Ok(Self { dispatch, custody })
    }

    pub(in crate::resident) fn operation(&self) -> &IssuedLocalOperation {
        self.dispatch.dispatch().operation()
    }

    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.dispatch.observation()
    }

    pub(in crate::resident) fn store(&self) -> &AuthorityStoreCoordinate {
        self.custody.custody().candidate().store()
    }

    /// Borrow the exact retained preparation that precedes this dispatch.
    pub(in crate::resident) fn preparation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.dispatch.dispatch().prepared_observation()
    }

    /// Borrow the checkpoint that preceded this same-process dispatch.
    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        self.dispatch.dispatch().checkpoint()
    }

    /// Borrow the immutable provider selection retained by this dispatch.
    pub(in crate::resident) fn selection_ref(&self) -> &ResidentRecordRefV1 {
        self.dispatch.dispatch().selection_ref()
    }

    /// Reconstruct the exact provider request without releasing it.
    pub(in crate::resident) fn provider_request(
        &self,
    ) -> Result<ProviderInvocationRequest<'_>, OperationAttemptError> {
        self.dispatch.provider_request()
    }

    /// Check an independently carried recording context against this release.
    pub(in crate::resident) fn admits_recording_context(
        &self,
        admission: &ProviderInvocationAdmissionContext<'_>,
    ) -> bool {
        self.dispatch.admits_recording_context(admission)
    }
}

/// Provider work admitted from one same-process recovered dispatch.
#[must_use = "offer the prepared recovered call once or retain it unopened"]
pub(in crate::resident) struct PreparedRecoveredProviderOperationCall {
    continuation: ProviderOperationContinuationCoordinate,
    request: OwnedProviderInvocationRequest,
}

impl PreparedRecoveredProviderOperationCall {
    pub(in crate::resident) fn into_task_parts(
        self,
    ) -> (
        AuthorityStoreCoordinate,
        ImmutableRecordReferenceV1,
        ProviderOperationContinuationCoordinate,
        OwnedProviderInvocationRequest,
    ) {
        (
            self.continuation.store.clone(),
            self.continuation.dispatch.clone(),
            self.continuation,
            self.request,
        )
    }

    pub(in crate::resident) const fn from_task_parts(
        continuation: ProviderOperationContinuationCoordinate,
        request: OwnedProviderInvocationRequest,
    ) -> Self {
        Self {
            continuation,
            request,
        }
    }
}

/// Recovered release preflight stopped without losing its linear capability.
pub(in crate::resident) struct RecoveredProviderCallPreparationFailure {
    release: Box<RecoveredProviderDispatchRelease>,
    reason: OperationAttemptError,
}

impl std::fmt::Debug for RecoveredProviderCallPreparationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecoveredProviderCallPreparationFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl RecoveredProviderCallPreparationFailure {
    pub(in crate::resident) fn into_parts(
        self,
    ) -> (RecoveredProviderDispatchRelease, OperationAttemptError) {
        (*self.release, self.reason)
    }
}

/// Perform every application check available before releasing a recovered call.
///
/// This ordinary provider route accepts only `Invoke`. Other operation classes
/// remain unopened for their owning route.
///
/// Only a newly committed dispatch from this process can pass the custody
/// check. An exact historical dispatch remains valid evidence but can never
/// regain provider-call authority.
pub(in crate::resident) fn prepare_recovered_provider_operation_call<Authority>(
    release: RecoveredProviderDispatchRelease,
    input: &ProviderOperationInvokeInput<'_>,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<PreparedRecoveredProviderOperationCall, RecoveredProviderCallPreparationFailure>
where
    Authority: AmbientSessionAuthorityPort,
{
    if release.operation().handle().1.operation_class() != &OperationClassV1::Invoke {
        return Err(RecoveredProviderCallPreparationFailure {
            release: Box::new(release),
            reason: OperationAttemptError::Association,
        });
    }
    if let Err(reason) = validate_recovered_provider_call_references(&release, input) {
        return Err(RecoveredProviderCallPreparationFailure {
            release: Box::new(release),
            reason,
        });
    }
    let started = Instant::now();
    let floor = input
        .current
        .now
        .max(*release.dispatch.observation().1.observed_at());
    let prepared = (|| {
        let before = sample_time(input.clock, floor)?;
        release
            .dispatch
            .validate_current_at(&input.current.at_time(before), authority)?;
        release
            .dispatch
            .prepare_append(input.resolve_store)?
            .verify_fresh_dispatch_custody(
                release.custody.custody(),
                custody,
                anchor,
                input.maximum_record_bytes,
            )?;
        let checked_at = sample_time(input.clock, before)?;
        release
            .dispatch
            .validate_current_at(&input.current.at_time(checked_at), authority)?;
        let released_at = sample_time(input.clock, checked_at)?;
        if released_at >= release.dispatch.request_expires_at()
            || started.elapsed()
                >= Duration::from_millis(
                    release
                        .operation()
                        .historical_operation_lease()
                        .timeout_millis()
                        .get(),
                )
        {
            return Err(OperationAttemptError::Deadline);
        }
        signing::validate_runtime_signer(
            input.signing.signing_authority,
            &input.current.at_time(released_at),
            OPERATION_OBSERVATION_V1_SCHEMA,
        )?;
        signing::validate_runtime_signer(
            release.dispatch.result_authority(),
            &input.current.at_time(released_at),
            CAPABILITY_RESULT_V1_SCHEMA,
        )?;
        release.dispatch.validate_prepared_call()?;
        let record_sha256 = ArtifactDigest::parse(
            release
                .dispatch
                .observation()
                .1
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .map_err(|_| OperationAttemptError::Encoding)?;
        let continuation = ProviderOperationContinuationCoordinate {
            store: release.custody.custody().candidate().store().clone(),
            dispatch: ImmutableRecordReferenceV1 {
                record_ref: release.dispatch.observation().0.clone(),
                record_sha256,
            },
            released_at,
        };
        Ok(continuation)
    })();
    match prepared {
        Ok(continuation) => {
            let (_, _, request, _) = release.dispatch.into_parts();
            Ok(PreparedRecoveredProviderOperationCall {
                continuation,
                request,
            })
        }
        Err(reason) => Err(RecoveredProviderCallPreparationFailure {
            release: Box::new(release),
            reason,
        }),
    }
}

/// Terminal `HostNative` evidence committed through the database and independent anchor.
#[derive(Debug)]
pub struct CompletedProviderOperation {
    outcome: AdmittedProviderInvocationOutcome,
    diagnostic: Option<AdmittedDiagnostic>,
    observation: (ResidentRecordRefV1, OperationObservationV1),
    custody: CompletedOperationObservationCustody,
}

impl CompletedProviderOperation {
    /// Borrow the exact admitted provider outcome retained by this transition.
    #[must_use]
    pub const fn outcome(&self) -> &AdmittedProviderInvocationOutcome {
        &self.outcome
    }

    /// Borrow the shared diagnostic attached to a safe provider failure.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&AdmittedDiagnostic> {
        self.diagnostic.as_ref()
    }

    /// Borrow the committed conclusive observation.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation.0, &self.observation.1)
    }

    /// Borrow the completed database-and-anchor custody.
    #[must_use]
    pub const fn custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }
}

/// One consumed `HostNative` invocation result.
pub enum ProviderOperationInvokeResult {
    /// A signed success or safe failure reached conclusive custody.
    Completed(Box<CompletedProviderOperation>),
    /// The call was consumed without conclusive admitted evidence.
    ///
    /// The predecessor-linked observation retains the supplied reconciliation
    /// frontier. It grants neither a replay nor a new attempt.
    OutcomeUnknown(Box<UncertainProviderOperation>),
}

/// Why one consumed provider call requires reconciliation.
#[derive(Debug)]
pub enum ProviderOperationUncertainty {
    /// Returned evidence failed its existing contextual admission.
    ResponseRejected(ProviderInvocationAdmissionError),
    /// Admitted evidence did not belong to the retained selection.
    ResponseAssociation {
        /// Admitted evidence retained for diagnosis, never replay.
        outcome: Box<AdmittedProviderInvocationOutcome>,
        /// Exact association refusal.
        reason: Box<OperationAttemptError>,
    },
    /// The selected provider could not produce signed conclusive evidence.
    Unavailable(crate::provider_invocation::ProviderInvocationUnavailableAttempt),
    /// Recovery found an anchored dispatch with no retained terminal evidence.
    ///
    /// The provider may already have been called. This state can only proceed
    /// into reconciliation custody; it never reconstructs release authority.
    ColdRestartAfterDispatch,
}

/// Durable outcome-unknown custody for one consumed provider call.
#[derive(Debug)]
pub struct UncertainProviderOperation {
    uncertainty: ProviderOperationUncertainty,
    diagnostic: AdmittedDiagnostic,
    observation: (ResidentRecordRefV1, OperationObservationV1),
    custody: CompletedOperationObservationCustody,
}

enum ProviderOperationFinalDisposition {
    Completed {
        outcome: AdmittedProviderInvocationOutcome,
        diagnostic: Option<AdmittedDiagnostic>,
    },
    OutcomeUnknown {
        uncertainty: ProviderOperationUncertainty,
        diagnostic: AdmittedDiagnostic,
    },
}

impl ProviderOperationFinalDisposition {
    fn into_recording(self) -> ProviderOperationRecordingDisposition {
        match self {
            Self::Completed { outcome, .. } => {
                ProviderOperationRecordingDisposition::Outcome(outcome)
            }
            Self::OutcomeUnknown { uncertainty, .. } => {
                ProviderOperationRecordingDisposition::Uncertainty(uncertainty)
            }
        }
    }
}

/// Semantic owner retained after an exact provider terminal candidate exists.
///
/// This value carries the already-issued semantic records across physical
/// custody recovery. It cannot call a provider, rebuild a candidate, allocate
/// another identity, or sign another record.
#[must_use = "finish the exact provider terminal candidate or retain its finalizer"]
struct ProviderOperationTerminalFinalizer {
    disposition: ProviderOperationFinalDisposition,
    observation: (ResidentRecordRefV1, OperationObservationV1),
    expected_store: AuthorityStoreCoordinate,
    predecessor_record: ControlRecord,
    expected_records: Box<[OperationCustodyRecord]>,
}

impl ProviderOperationTerminalFinalizer {
    fn into_disposition(self) -> ProviderOperationRecordingDisposition {
        self.disposition.into_recording()
    }

    fn finish(
        self,
        custody: CompletedOperationObservationCustody,
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    ) -> Result<ProviderOperationInvokeResult, ProviderOperationRecordingFinalizationFailure> {
        let candidate = custody.candidate();
        let Some(observation_record) = self
            .expected_records
            .iter()
            .find(|record| record.kind() == OperationCustodyRecordKind::OperationObservation)
            .map(OperationCustodyRecord::record)
        else {
            return Err(ProviderOperationRecordingFinalizationFailure {
                finalization: Box::new(ProviderOperationRecordingFinalization {
                    finalizer: Box::new(self),
                    custody,
                    checkpoint,
                }),
                reason: OperationAttemptError::Association,
            });
        };
        let checkpoint_record = record_projection::record(
            match record_projection::coordinate(
                &checkpoint.0,
                checkpoint.1.canonical_control_checkpoint_sha256().as_str(),
            ) {
                Ok(coordinate) => coordinate,
                Err(_) => {
                    return Err(ProviderOperationRecordingFinalizationFailure {
                        finalization: Box::new(ProviderOperationRecordingFinalization {
                            finalizer: Box::new(self),
                            custody,
                            checkpoint,
                        }),
                        reason: OperationAttemptError::Association,
                    });
                }
            },
            &checkpoint.1,
        );
        let Ok(checkpoint_record) = checkpoint_record else {
            return Err(ProviderOperationRecordingFinalizationFailure {
                finalization: Box::new(ProviderOperationRecordingFinalization {
                    finalizer: Box::new(self),
                    custody,
                    checkpoint,
                }),
                reason: OperationAttemptError::Association,
            });
        };
        let latest_matches = candidate.indexes().iter().any(|index| {
            matches!(
                index,
                OperationCustodyIndexMutation::LatestObservation {
                    expected: OperationCustodyIndexPredecessor::Exact(predecessor),
                    successor,
                    ..
                } if predecessor == &self.predecessor_record
                    && successor == observation_record.coordinate()
            )
        });
        let records_match = candidate.records().len() == self.expected_records.len()
            && self
                .expected_records
                .iter()
                .all(|expected| candidate.records().iter().any(|actual| actual == expected));
        let indexes_match = candidate.indexes().len() == 2
            && candidate.indexes().iter().any(|index| {
                matches!(
                    index,
                    OperationCustodyIndexMutation::ObservationCheckpoint { successor, .. }
                        if successor == checkpoint_record.coordinate()
                )
            });
        if candidate.store() != &self.expected_store
            || candidate.kind() != pale_ports::OperationCustodyTransitionKind::Observe
            || candidate.session_basis().is_some()
            || candidate.collection_journal().is_some()
            || candidate.has_auxiliary_evidence()
            || candidate.checkpoint().successor_record() != &checkpoint_record
            || !latest_matches
            || !indexes_match
            || !records_match
        {
            return Err(ProviderOperationRecordingFinalizationFailure {
                finalization: Box::new(ProviderOperationRecordingFinalization {
                    finalizer: Box::new(self),
                    custody,
                    checkpoint,
                }),
                reason: OperationAttemptError::Association,
            });
        }
        Ok(match self.disposition {
            ProviderOperationFinalDisposition::Completed {
                outcome,
                diagnostic,
            } => ProviderOperationInvokeResult::Completed(Box::new(CompletedProviderOperation {
                outcome,
                diagnostic,
                observation: self.observation,
                custody,
            })),
            ProviderOperationFinalDisposition::OutcomeUnknown {
                uncertainty,
                diagnostic,
            } => ProviderOperationInvokeResult::OutcomeUnknown(Box::new(
                UncertainProviderOperation {
                    uncertainty,
                    diagnostic,
                    observation: self.observation,
                    custody,
                },
            )),
        })
    }
}

enum ProviderOperationAnchorCustody {
    Pending(Box<DatabaseCommittedOperationObservationAnchorPending>),
    Failed(OperationObservationCustodyAnchorFailure),
}

/// Exact provider terminal candidate with unresolved physical custody.
///
/// The candidate is retained by the existing custody owner. Continuing this
/// value performs only exact read-back, a proven-safe retry of that same
/// candidate, or its already-issued anchor transition.
#[must_use = "continue custody of the exact provider terminal candidate"]
pub struct ProviderOperationRecordingCustody {
    finalizer: Box<ProviderOperationTerminalFinalizer>,
    checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    phase: ProviderOperationRecordingCustodyPhase,
}

enum ProviderOperationRecordingCustodyPhase {
    Commit(OperationObservationCustodyCommitFailure),
    Anchor(ProviderOperationAnchorCustody),
}

impl std::fmt::Debug for ProviderOperationRecordingCustody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self.phase {
            ProviderOperationRecordingCustodyPhase::Commit(_) => "Commit",
            ProviderOperationRecordingCustodyPhase::Anchor(_) => "Anchor",
        };
        formatter
            .debug_struct("ProviderOperationRecordingCustody")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl ProviderOperationRecordingCustody {
    /// Continue only the retained physical candidate.
    ///
    /// An unresolved commit first receives one exact read-back. A candidate is
    /// recommitted only when the custody owner proves that retry is safe. No
    /// observation, checkpoint, signature, or provider call is reconstructed.
    ///
    /// # Errors
    ///
    /// Returns the unchanged custody continuation when exact read-back cannot
    /// establish the commit, when a retained candidate is not safe to retry,
    /// or when the already-issued anchor transition remains unresolved.
    pub fn continue_custody(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<ProviderOperationRecordingFinalization, Self> {
        let Self {
            finalizer,
            checkpoint,
            phase,
        } = self;
        let anchor_phase = match phase {
            ProviderOperationRecordingCustodyPhase::Commit(failure) => {
                match failure.reconcile_exact(custody) {
                    Ok(pending) => ProviderOperationAnchorCustody::Pending(Box::new(pending)),
                    Err(failure) => match failure.try_into_retryable() {
                        Ok(prepared) => match prepared.commit(custody, anchor) {
                            Ok(pending) => {
                                ProviderOperationAnchorCustody::Pending(Box::new(pending))
                            }
                            Err(failure) => {
                                return Err(Self {
                                    finalizer,
                                    checkpoint,
                                    phase: ProviderOperationRecordingCustodyPhase::Commit(failure),
                                });
                            }
                        },
                        Err(failure) => {
                            return Err(Self {
                                finalizer,
                                checkpoint,
                                phase: ProviderOperationRecordingCustodyPhase::Commit(failure),
                            });
                        }
                    },
                }
            }
            ProviderOperationRecordingCustodyPhase::Anchor(phase) => phase,
        };
        let pending = match anchor_phase {
            ProviderOperationAnchorCustody::Pending(pending) => *pending,
            ProviderOperationAnchorCustody::Failed(failure) => match failure.try_into_retryable() {
                Ok(pending) => pending,
                Err(failure) => {
                    return Err(Self {
                        finalizer,
                        checkpoint,
                        phase: ProviderOperationRecordingCustodyPhase::Anchor(
                            ProviderOperationAnchorCustody::Failed(failure),
                        ),
                    });
                }
            },
        };
        match pending.advance_anchor(anchor) {
            Ok(custody) => Ok(ProviderOperationRecordingFinalization {
                finalizer,
                custody,
                checkpoint: *checkpoint,
            }),
            Err(failure) => Err(Self {
                finalizer,
                checkpoint,
                phase: ProviderOperationRecordingCustodyPhase::Anchor(
                    ProviderOperationAnchorCustody::Failed(failure),
                ),
            }),
        }
    }
}

/// Exact completed physical custody awaiting provider semantic admission.
#[must_use = "finish semantic admission of the exact provider terminal candidate"]
pub struct ProviderOperationRecordingFinalization {
    finalizer: Box<ProviderOperationTerminalFinalizer>,
    custody: CompletedOperationObservationCustody,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
}

impl ProviderOperationRecordingFinalization {
    /// Join the exact completed physical candidate to its retained semantics.
    ///
    /// # Errors
    ///
    /// Returns the finalization token and refusal when completed custody does
    /// not match the exact retained physical candidate.
    pub fn finish(
        self,
    ) -> Result<ProviderOperationInvokeResult, ProviderOperationRecordingFinalizationFailure> {
        self.finalizer.finish(self.custody, self.checkpoint)
    }
}

/// Semantic admission failed without discarding completed physical custody.
pub struct ProviderOperationRecordingFinalizationFailure {
    finalization: Box<ProviderOperationRecordingFinalization>,
    reason: OperationAttemptError,
}

impl std::fmt::Debug for ProviderOperationRecordingFinalizationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderOperationRecordingFinalizationFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl ProviderOperationRecordingFinalizationFailure {
    /// Borrow the exact semantic association refusal.
    #[must_use]
    pub const fn reason(&self) -> &OperationAttemptError {
        &self.reason
    }

    /// Recover the unchanged post-custody finalization token.
    pub fn into_finalization(self) -> ProviderOperationRecordingFinalization {
        *self.finalization
    }

    pub(in crate::resident) fn into_parts(
        self,
    ) -> (
        ProviderOperationRecordingFinalization,
        OperationAttemptError,
    ) {
        (*self.finalization, self.reason)
    }
}

impl UncertainProviderOperation {
    /// Borrow the exact post-call uncertainty class.
    #[must_use]
    pub const fn uncertainty(&self) -> &ProviderOperationUncertainty {
        &self.uncertainty
    }

    /// Borrow the diagnostic that requires reconciliation before another action.
    #[must_use]
    pub const fn diagnostic(&self) -> &AdmittedDiagnostic {
        &self.diagnostic
    }

    /// Borrow the committed predecessor-linked uncertainty observation.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation.0, &self.observation.1)
    }

    /// Borrow the completed database-and-anchor custody.
    #[must_use]
    pub const fn custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }
}

enum ProviderOperationRecordingDisposition {
    Outcome(AdmittedProviderInvocationOutcome),
    Uncertainty(ProviderOperationUncertainty),
}

/// Immutable application evidence needed after a recovered provider call has
/// already been consumed.
///
/// This deliberately retains no request, recovered attempt, session authority,
/// or provider-call capability. It is only the exact signed recording basis
/// that a preparation failure must carry into a later input-lending turn.
struct OwnedRecoveredProviderRecordingBasis {
    checkpoint_selection: OperationCheckpointSelection,
    request_ref: ResidentRecordRefV1,
    handle_ref: ResidentRecordRefV1,
    handle: OperationHandleV1,
    observation: (ResidentRecordRefV1, OperationObservationV1),
    recording_predecessor: (ResidentRecordRefV1, OperationObservationV1),
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    selection_ref: ResidentRecordRefV1,
    preparation_observation_ref: ResidentRecordRefV1,
    attempt_id: CapabilityAttemptIdV1,
}

impl OwnedRecoveredProviderRecordingBasis {
    fn retain(recovered: &RecoveredProviderOperationDispatch<'_>) -> Self {
        let operation = recovered.operation();
        Self {
            checkpoint_selection: operation.checkpoint_selection(),
            request_ref: operation.request_coordinate().record_ref().clone(),
            handle_ref: operation.handle().0.clone(),
            handle: operation.handle().1.clone(),
            observation: (
                recovered.observation().0.clone(),
                recovered.observation().1.clone(),
            ),
            recording_predecessor: (
                recovered.recording_predecessor().0.clone(),
                recovered.recording_predecessor().1.clone(),
            ),
            checkpoint: (
                recovered.checkpoint().0.clone(),
                recovered.checkpoint().1.clone(),
            ),
            selection_ref: recovered.selection_ref().clone(),
            preparation_observation_ref: recovered.preparation_observation_ref().clone(),
            attempt_id: recovered.attempt_id().clone(),
        }
    }
}

enum ProviderOperationRecordingSource<'operation> {
    Retained(Box<RetainedProviderOperationDispatch<'operation>>),
    Recovered(Box<OwnedRecoveredProviderRecordingBasis>),
}

impl ProviderOperationRecordingSource<'_> {
    fn handle(&self) -> (&ResidentRecordRefV1, &OperationHandleV1) {
        match self {
            Self::Retained(dispatch) => dispatch.context().operation.handle(),
            Self::Recovered(basis) => (&basis.handle_ref, &basis.handle),
        }
    }

    fn checkpoint_selection(&self) -> OperationCheckpointSelection {
        match self {
            Self::Retained(dispatch) => dispatch.context().operation.checkpoint_selection(),
            Self::Recovered(basis) => basis.checkpoint_selection.clone(),
        }
    }

    fn request_ref(&self) -> &ResidentRecordRefV1 {
        match self {
            Self::Retained(dispatch) => dispatch
                .context()
                .operation
                .request_coordinate()
                .record_ref(),
            Self::Recovered(basis) => &basis.request_ref,
        }
    }

    fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        match self {
            Self::Retained(dispatch) => dispatch.observation(),
            Self::Recovered(basis) => (&basis.observation.0, &basis.observation.1),
        }
    }

    fn recording_predecessor(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        match self {
            Self::Retained(dispatch) => dispatch.observation(),
            Self::Recovered(basis) => (
                &basis.recording_predecessor.0,
                &basis.recording_predecessor.1,
            ),
        }
    }

    fn checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        match self {
            Self::Retained(dispatch) => dispatch.checkpoint(),
            Self::Recovered(basis) => (&basis.checkpoint.0, &basis.checkpoint.1),
        }
    }

    fn selection_ref(&self) -> &ResidentRecordRefV1 {
        match self {
            Self::Retained(dispatch) => &dispatch.context().selection_ref,
            Self::Recovered(basis) => &basis.selection_ref,
        }
    }

    fn preparation_observation_ref(&self) -> &ResidentRecordRefV1 {
        match self {
            Self::Retained(dispatch) => &dispatch.context().observation_ref,
            Self::Recovered(basis) => &basis.preparation_observation_ref,
        }
    }

    fn attempt_id(&self) -> &CapabilityAttemptIdV1 {
        match self {
            Self::Retained(dispatch) => dispatch.dispatch.attempt.selection.attempt_id(),
            Self::Recovered(basis) => &basis.attempt_id,
        }
    }

    fn validate_original_session(&self) -> Result<(), OperationAttemptError> {
        match self {
            Self::Retained(dispatch) => dispatch.context().validate_original_session(),
            Self::Recovered(_) => Ok(()),
        }
    }
}

/// A provider call that was consumed and can only continue into recording.
///
/// This value deliberately exposes no provider request or release method. A
/// failure before physical commitment can return it for another recording
/// attempt without granting another provider call.
pub struct ConsumedProviderOperation<'operation> {
    dispatch: ProviderOperationRecordingSource<'operation>,
    released_at: UnixTimeV1,
    disposition: ProviderOperationRecordingDisposition,
}

impl ConsumedProviderOperation<'_> {
    /// Borrow admitted conclusive evidence when the provider returned it.
    #[must_use]
    pub const fn outcome(&self) -> Option<&AdmittedProviderInvocationOutcome> {
        match &self.disposition {
            ProviderOperationRecordingDisposition::Outcome(outcome) => Some(outcome),
            ProviderOperationRecordingDisposition::Uncertainty(_) => None,
        }
    }

    /// Borrow the exact reason this call requires reconciliation.
    #[must_use]
    pub const fn uncertainty(&self) -> Option<&ProviderOperationUncertainty> {
        match &self.disposition {
            ProviderOperationRecordingDisposition::Outcome(_) => None,
            ProviderOperationRecordingDisposition::Uncertainty(uncertainty) => Some(uncertainty),
        }
    }
}

/// A consumed provider outcome plus the exact head where recording can resume.
///
/// This continuation has no provider request or release method. It can only
/// record the outcome already held by [`ConsumedProviderOperation`]. A fresh
/// continuation starts at the retained `Dispatched` observation. After a
/// concurrent head wins, protected recovery can create a replacement that
/// follows that exact admitted head and its current checkpoint.
#[must_use = "record the consumed provider outcome or retain its continuation"]
pub struct ConsumedProviderRecordingContinuation<'operation> {
    operation: ConsumedProviderOperation<'operation>,
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
}

impl ConsumedProviderRecordingContinuation<'_> {
    /// Borrow admitted conclusive evidence when the provider returned it.
    #[must_use]
    pub const fn outcome(&self) -> Option<&AdmittedProviderInvocationOutcome> {
        self.operation.outcome()
    }

    /// Borrow the exact reason this call requires reconciliation.
    #[must_use]
    pub const fn uncertainty(&self) -> Option<&ProviderOperationUncertainty> {
        self.operation.uncertainty()
    }
}

/// A definite competing operation head with the consumed provider call retained.
///
/// The failed candidate did not commit. This value cannot retry it or invoke
/// the provider. Protected history must first establish a safe recording head.
#[must_use = "recover the current operation head before recording the consumed outcome"]
pub struct ProviderOperationRecordingConflict<'operation> {
    operation: ConsumedProviderOperation<'operation>,
}

impl<'operation> ProviderOperationRecordingConflict<'operation> {
    /// Borrow admitted conclusive evidence without gaining recording authority.
    #[must_use]
    pub const fn outcome(&self) -> Option<&AdmittedProviderInvocationOutcome> {
        self.operation.outcome()
    }

    /// Borrow the exact uncertainty without gaining recording authority.
    #[must_use]
    pub const fn uncertainty(&self) -> Option<&ProviderOperationUncertainty> {
        self.operation.uncertainty()
    }

    /// Bind the consumed outcome to one protected current history.
    ///
    /// Recovery must select the exact original `Dispatched` observation. Its
    /// authenticated head may still be that dispatch, or one direct
    /// `CancellationRequested` successor. No other state is accepted. This
    /// method performs no write, signing, provider call, or cancellation call.
    ///
    /// # Errors
    ///
    /// Returns the conflict unchanged when the store, selected dispatch,
    /// current head, or direct cancellation lineage does not match.
    pub fn recover(
        self,
        recovered: &RecoveredOperationObservation,
    ) -> Result<
        ConsumedProviderRecordingContinuation<'operation>,
        ProviderOperationRecordingRecoveryFailure<'operation>,
    > {
        match recovered_provider_recording_position(&self.operation.dispatch, recovered) {
            Ok((predecessor, checkpoint)) => Ok(ConsumedProviderRecordingContinuation {
                operation: self.operation,
                predecessor,
                checkpoint,
            }),
            Err(reason) => Err(ProviderOperationRecordingRecoveryFailure {
                conflict: Box::new(self),
                reason,
            }),
        }
    }
}

/// Protected-history validation failed without discarding the consumed outcome.
pub struct ProviderOperationRecordingRecoveryFailure<'operation> {
    conflict: Box<ProviderOperationRecordingConflict<'operation>>,
    reason: OperationAttemptError,
}

impl std::fmt::Debug for ProviderOperationRecordingRecoveryFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderOperationRecordingRecoveryFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl<'operation> ProviderOperationRecordingRecoveryFailure<'operation> {
    /// Borrow the exact recovery refusal.
    #[must_use]
    pub const fn reason(&self) -> &OperationAttemptError {
        &self.reason
    }

    /// Recover the unchanged consumed-outcome conflict for another protected read.
    pub fn into_conflict(self) -> ProviderOperationRecordingConflict<'operation> {
        *self.conflict
    }
}

/// Recording can be rebuilt because no custody candidate committed.
#[derive(Debug)]
pub enum ProviderOperationRecordingPreparationError {
    /// Clock, signer, admission, bound, or lineage validation failed.
    Operation(OperationAttemptError),
    /// Record or append projection failed before commitment.
    Projection(OperationObservationCustodyProjectionError),
    /// The store-global transaction advanced while this operation head stayed unchanged.
    StoreTransactionDrift,
}

/// A consumed provider call stopped while recording its known disposition.
pub enum ProviderOperationRecordingFailure<'operation> {
    /// No candidate committed. The same consumed call may be recorded again
    /// after correcting this preparation failure.
    Preparation {
        /// The consumed call and exact safe recording head.
        continuation: Box<ConsumedProviderRecordingContinuation<'operation>>,
        /// Exact pre-commit recording failure.
        reason: Box<ProviderOperationRecordingPreparationError>,
    },
    /// Another complete operation head won the database comparison.
    ConcurrentHead {
        /// Consumed provider outcome with no permission to retry or replay.
        conflict: Box<ProviderOperationRecordingConflict<'operation>>,
    },
    /// An exact physical candidate exists. Its existing custody failure owns
    /// read-back, retry, and anchor completion; recording must not be rebuilt.
    Custody {
        /// Exact provider semantics joined to the existing physical candidate.
        custody: Box<ProviderOperationRecordingCustody>,
    },
    /// Physical custody completed, but its candidate did not match the
    /// retained provider semantics.
    Finalization {
        /// Exact completed custody and semantic owner retained for diagnosis.
        failure: Box<ProviderOperationRecordingFinalizationFailure>,
    },
}

/// Cold-restart uncertainty stopped before exact provider terminal settlement.
pub(in crate::resident) enum RecoveredProviderUnknownRecordingFailure {
    /// Observation signing did not complete; protected recovery may be repeated.
    Preparation(Box<OperationAppendFailure>),
    /// The terminal observation is signed; continue this exact candidate.
    Prepared(Box<PreparedRecoveredProviderUnknown>),
    /// The checkpoint is signed; attach this exact value without signing again.
    Issued(Box<IssuedRecoveredProviderUnknownCheckpoint>),
    /// One exact physical candidate owns all further custody work.
    Custody(Box<ProviderOperationRecordingCustody>),
    /// Physical custody completed but semantic association remains unproved.
    Finalization(Box<ProviderOperationRecordingFinalizationFailure>),
}

impl std::fmt::Debug for RecoveredProviderUnknownRecordingFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Preparation(_) => "Preparation",
            Self::Prepared(_) => "Prepared",
            Self::Issued(_) => "Issued",
            Self::Custody(_) => "Custody",
            Self::Finalization(_) => "Finalization",
        };
        formatter
            .debug_struct("RecoveredProviderUnknownRecordingFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Signed cold-restart uncertainty whose checkpoint is not yet signed.
///
/// This private continuation owns every semantic value needed to rebuild the
/// exact physical append. It cannot call the provider or sign another terminal
/// observation.
#[must_use = "continue the exact signed uncertainty candidate"]
pub(in crate::resident) struct PreparedRecoveredProviderUnknown {
    terminal: Box<OwnedRecoveredProviderUnknownTerminal>,
    checkpoint: OperationCheckpointInput,
    checkpoint_issued_at: UnixTimeV1,
    checkpoint_schema: SchemaDigest,
    checkpoint_authority: OwnedRecoveredProviderCheckpointAuthority,
    accumulator_limits: AccumulatorPreparationLimits,
    failure: Option<OperationAppendFailure>,
}

struct OwnedRecoveredProviderCheckpointAuthority {
    authority_ref: ResidentRecordRefV1,
    authority_sha256: CheckpointSigningAuthorityDigest,
    principal_id: ResidentPrincipalIdV1,
    control_store_id: ControlStoreIdV1,
    control_incarnation_id: ControlIncarnationIdV1,
    checkpoint_domain_id: ControlCheckpointDomainIdV1,
    checkpoint_profile_ref: ResidentRecordRefV1,
    checkpoint_profile_sha256: CheckpointProfileDigest,
    subject_key_id: ResidentKeyIdV1,
    subject_key_epoch: SigningKeyEpochV1,
}

impl OwnedRecoveredProviderCheckpointAuthority {
    fn retain(authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>) -> Self {
        Self {
            authority_ref: authority.authority_ref.clone(),
            authority_sha256: authority.authority_sha256.clone(),
            principal_id: authority.principal_id.clone(),
            control_store_id: authority.control_store_id.clone(),
            control_incarnation_id: authority.control_incarnation_id.clone(),
            checkpoint_domain_id: authority.checkpoint_domain_id.clone(),
            checkpoint_profile_ref: authority.checkpoint_profile_ref.clone(),
            checkpoint_profile_sha256: authority.checkpoint_profile_sha256.clone(),
            subject_key_id: authority.subject_key_id.clone(),
            subject_key_epoch: authority.subject_key_epoch,
        }
    }

    fn matches(&self, authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>) -> bool {
        authority.authority_ref == &self.authority_ref
            && authority.authority_sha256 == &self.authority_sha256
            && authority.principal_id == &self.principal_id
            && authority.control_store_id == &self.control_store_id
            && authority.control_incarnation_id == &self.control_incarnation_id
            && authority.checkpoint_domain_id == &self.checkpoint_domain_id
            && authority.checkpoint_profile_ref == &self.checkpoint_profile_ref
            && authority.checkpoint_profile_sha256 == &self.checkpoint_profile_sha256
            && authority.subject_key_id == &self.subject_key_id
            && authority.subject_key_epoch == self.subject_key_epoch
    }
}

impl std::fmt::Debug for PreparedRecoveredProviderUnknown {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedRecoveredProviderUnknown")
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}

/// Exact signed checkpoint awaiting attachment to the same uncertainty append.
#[must_use = "attach this exact checkpoint without calling a signer"]
pub(in crate::resident) struct IssuedRecoveredProviderUnknownCheckpoint {
    prepared: Box<PreparedRecoveredProviderUnknown>,
    checkpoint_value: Box<ControlStateCheckpointV1>,
    failure: Option<OperationAppendFailure>,
}

impl std::fmt::Debug for IssuedRecoveredProviderUnknownCheckpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IssuedRecoveredProviderUnknownCheckpoint")
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}

struct OwnedRecoveredProviderUnknownTerminal {
    checkpoint_selection: crate::resident::checkpoint_profile::OperationCheckpointSelection,
    handle_ref: ResidentRecordRefV1,
    handle: OperationHandleV1,
    predecessor_checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    expected_store: AuthorityStoreCoordinate,
    diagnostic_record: ControlRecord,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    uncertainty: ProviderOperationUncertainty,
    diagnostic: AdmittedDiagnostic,
}

impl OwnedRecoveredProviderUnknownTerminal {
    fn prepare_checkpoint<'a>(
        &'a self,
        limits: AccumulatorPreparationLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        prepare_operation_observation_append(
            OperationObservationTransition::new(
                &self.handle_ref,
                &self.handle,
                &self.predecessor.0,
                &self.predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?
        .with_associated_record(OperationCustodyRecord::Diagnostic(
            self.diagnostic_record.clone(),
        ))
        .prepare_checkpoint(
            self.checkpoint_selection.clone(),
            &self.predecessor_checkpoint.0,
            &self.predecessor_checkpoint.1,
            limits,
            custody,
        )
    }

    fn into_finalizer(
        self,
    ) -> Result<Box<ProviderOperationTerminalFinalizer>, (Box<Self>, OperationAppendFailure)> {
        let predecessor_coordinate = match record_projection::coordinate(
            &self.predecessor.0,
            self.predecessor
                .1
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .map_err(OperationObservationCustodyProjectionError::from)
        {
            Ok(coordinate) => coordinate,
            Err(failure) => return Err((Box::new(self), failure.into())),
        };
        let predecessor_record =
            match record_projection::record(predecessor_coordinate, &self.predecessor.1)
                .map_err(OperationObservationCustodyProjectionError::from)
            {
                Ok(record) => record,
                Err(failure) => return Err((Box::new(self), failure.into())),
            };
        let observation_coordinate = match record_projection::coordinate(
            &self.observation_ref,
            self.observation
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .map_err(OperationObservationCustodyProjectionError::from)
        {
            Ok(coordinate) => coordinate,
            Err(failure) => return Err((Box::new(self), failure.into())),
        };
        let observation_record =
            match record_projection::record(observation_coordinate, &self.observation)
                .map_err(OperationObservationCustodyProjectionError::from)
            {
                Ok(record) => record,
                Err(failure) => return Err((Box::new(self), failure.into())),
            };
        if !self
            .checkpoint_selection
            .matches_store(&self.expected_store)
        {
            return Err((Box::new(self), OperationAttemptError::Association.into()));
        }
        Ok(Box::new(ProviderOperationTerminalFinalizer {
            disposition: ProviderOperationFinalDisposition::OutcomeUnknown {
                uncertainty: self.uncertainty,
                diagnostic: self.diagnostic,
            },
            observation: (self.observation_ref, self.observation),
            expected_store: self.expected_store,
            predecessor_record,
            expected_records: vec![
                OperationCustodyRecord::OperationObservation(observation_record),
                OperationCustodyRecord::Diagnostic(self.diagnostic_record.clone()),
            ]
            .into_boxed_slice(),
        }))
    }
}

impl PreparedRecoveredProviderUnknown {
    /// Borrow the last exact refusal without consuming the retry phase.
    #[must_use]
    pub(in crate::resident) const fn failure(&self) -> Option<&OperationAppendFailure> {
        self.failure.as_ref()
    }
}

impl IssuedRecoveredProviderUnknownCheckpoint {
    /// Borrow the last exact refusal without consuming the issued checkpoint.
    #[must_use]
    pub(in crate::resident) const fn failure(&self) -> Option<&OperationAppendFailure> {
        self.failure.as_ref()
    }
}

impl std::fmt::Debug for ProviderOperationRecordingFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stage = match self {
            Self::Preparation { .. } => "Preparation",
            Self::ConcurrentHead { .. } => "ConcurrentHead",
            Self::Custody { .. } => "Custody",
            Self::Finalization { .. } => "Finalization",
        };
        formatter
            .debug_struct("ProviderOperationRecordingFailure")
            .field("stage", &stage)
            .finish_non_exhaustive()
    }
}

impl<'operation> ProviderOperationRecordingFailure<'operation> {
    /// Recover the consumed call only when no custody candidate committed.
    pub fn into_retryable(self) -> Option<ConsumedProviderRecordingContinuation<'operation>> {
        match self {
            Self::Preparation { continuation, .. } => Some(*continuation),
            Self::ConcurrentHead { .. } | Self::Custody { .. } | Self::Finalization { .. } => None,
        }
    }

    /// Recover a consumed outcome only after a definite competing head.
    #[must_use]
    pub fn into_concurrent_head(self) -> Option<ProviderOperationRecordingConflict<'operation>> {
        match self {
            Self::ConcurrentHead { conflict } => Some(*conflict),
            Self::Preparation { .. } | Self::Custody { .. } | Self::Finalization { .. } => None,
        }
    }
}

/// Where a `HostNative` invocation stopped before release or durable recording.
pub enum ProviderOperationInvokeFailure<'operation> {
    /// Input identities were inconsistent before release checks.
    Admission {
        /// Original unopened dispatch. No provider call occurred.
        dispatch: Box<RetainedProviderOperationDispatch<'operation>>,
        /// Exact admission refusal.
        reason: Box<OperationAttemptError>,
    },
    /// The unopened dispatch failed current authority, time, or fresh-custody checks.
    Release {
        /// Original unopened dispatch. No provider call occurred.
        dispatch: Box<RetainedProviderOperationDispatch<'operation>>,
        /// Exact refusal.
        reason: Box<OperationAttemptError>,
    },
    /// Provider-invocation preparation failed before the provider call.
    Preparation {
        /// Original unopened dispatch. No provider call occurred.
        dispatch: Box<RetainedProviderOperationDispatch<'operation>>,
        /// Exact provider invocation preparation refusal.
        reason: ProviderInvocationAdmissionError,
    },
    /// The provider call was consumed, but its disposition did not reach
    /// complete recording custody.
    Recording(ProviderOperationRecordingFailure<'operation>),
}

impl std::fmt::Debug for ProviderOperationInvokeFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stage = match self {
            Self::Admission { .. } => "Admission",
            Self::Release { .. } => "Release",
            Self::Preparation { .. } => "Preparation",
            Self::Recording(_) => "Recording",
        };
        formatter
            .debug_struct("ProviderOperationInvokeFailure")
            .field("stage", &stage)
            .finish_non_exhaustive()
    }
}

impl<'operation> ProviderOperationInvokeFailure<'operation> {
    /// Recover a dispatch when validation stopped before provider release.
    ///
    /// Admission, current-authority release, and provider preparation failures
    /// occur before the provider port is called. No later failure can recover
    /// release authority because the call was consumed or terminal recording
    /// may have committed.
    pub fn into_unopened_dispatch(self) -> Option<RetainedProviderOperationDispatch<'operation>> {
        match self {
            Self::Admission { dispatch, .. }
            | Self::Release { dispatch, .. }
            | Self::Preparation { dispatch, .. } => Some(*dispatch),
            _ => None,
        }
    }
}

/// Perform every fresh application check available before provider I/O.
///
/// This ordinary provider route accepts only `Invoke`. Other operation classes
/// remain unopened for their owning route.
///
/// The returned value owns only an admitted mechanical request and the exact
/// retained application basis needed after completion. No provider is called.
pub(in crate::resident) fn prepare_provider_operation_call<'operation, Authority>(
    retained: RetainedProviderOperationDispatch<'operation>,
    input: &ProviderOperationInvokeInput<'_>,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<PreparedProviderOperationCall<'operation>, ProviderOperationInvokeFailure<'operation>>
where
    Authority: AmbientSessionAuthorityPort,
{
    prepare_provider_operation_call_for_class(
        retained,
        input,
        authority,
        custody,
        anchor,
        OperationClassV1::Invoke,
    )
}

/// Perform fresh release preflight for one pairwise Delegate provider call.
pub(in crate::resident) fn prepare_pairwise_delegate_provider_call<'operation, Authority>(
    retained: RetainedProviderOperationDispatch<'operation>,
    input: &ProviderOperationInvokeInput<'_>,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<PreparedProviderOperationCall<'operation>, ProviderOperationInvokeFailure<'operation>>
where
    Authority: AmbientSessionAuthorityPort,
{
    prepare_provider_operation_call_for_class(
        retained,
        input,
        authority,
        custody,
        anchor,
        OperationClassV1::Delegate,
    )
}

fn prepare_provider_operation_call_for_class<'operation, Authority>(
    retained: RetainedProviderOperationDispatch<'operation>,
    input: &ProviderOperationInvokeInput<'_>,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    expected_class: OperationClassV1,
) -> Result<PreparedProviderOperationCall<'operation>, ProviderOperationInvokeFailure<'operation>>
where
    Authority: AmbientSessionAuthorityPort,
{
    if retained.operation().handle().1.operation_class() != &expected_class {
        return Err(ProviderOperationInvokeFailure::Admission {
            dispatch: Box::new(retained),
            reason: Box::new(OperationAttemptError::Association),
        });
    }
    if let Err(reason) = validate_references(&retained, input) {
        return Err(ProviderOperationInvokeFailure::Admission {
            dispatch: Box::new(retained),
            reason: Box::new(reason),
        });
    }
    let started = Instant::now();
    let released_at = match validate_release(&retained, input, started, authority, custody, anchor)
    {
        Ok(released_at) => released_at,
        Err(reason) => {
            return Err(ProviderOperationInvokeFailure::Release {
                dispatch: Box::new(retained),
                reason: Box::new(reason),
            });
        }
    };

    let request = match retained.provider_request() {
        Ok(request) => request,
        Err(reason) => {
            return Err(ProviderOperationInvokeFailure::Admission {
                dispatch: Box::new(retained),
                reason: Box::new(reason),
            });
        }
    };
    let owned_request = OwnedProviderInvocationRequest::capture(request);
    let request = match owned_request.borrowed() {
        Ok(request) => request,
        Err(reason) => {
            return Err(ProviderOperationInvokeFailure::Preparation {
                dispatch: Box::new(retained),
                reason,
            });
        }
    };
    let prepared = match PreparedProviderInvocation::prepare(request, EffectClass::Compute) {
        Ok(prepared) => prepared,
        Err(reason) => {
            return Err(ProviderOperationInvokeFailure::Preparation {
                dispatch: Box::new(retained),
                reason,
            });
        }
    };
    match prepared.bind(retained.dispatch.attempt.result_admission) {
        Ok(_) => {}
        Err(reason) => {
            return Err(ProviderOperationInvokeFailure::Preparation {
                dispatch: Box::new(retained),
                reason,
            });
        }
    }
    Ok(PreparedProviderOperationCall {
        basis: ProviderOperationCallBasis {
            retained,
            released_at,
        },
        request: owned_request,
    })
}

/// Run only the synchronous mechanical provider call.
///
/// A resident host must execute this function away from its single-writer event
/// path. The function performs no application evidence admission or custody.
pub(in crate::resident) fn invoke_owned_provider_invocation<Provider: ProviderInvocationPort>(
    request: &OwnedProviderInvocationRequest,
    provider: &Provider,
) -> Result<ProviderInvocationObservation, ProviderInvocationAdmissionError> {
    request.borrowed().map(|request| provider.invoke(request))
}

/// Admit and record one raw mechanical provider completion.
///
/// This function cannot call a provider. All response rejection, association,
/// unavailable and custody outcomes retain the existing application semantics.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn record_provider_operation_completion<
    'operation,
    Rotation,
    ObservationSign,
    CheckpointSign,
    Reconciliation,
>(
    basis: ProviderOperationCallBasis<'operation>,
    request: &OwnedProviderInvocationRequest,
    observation: Result<ProviderInvocationObservation, ProviderInvocationAdmissionError>,
    input: &ProviderOperationInvokeInput<'_>,
    evidence: &dyn ProviderInvocationEvidenceAdmission,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_reconciliation: Reconciliation,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<ProviderOperationInvokeResult, ProviderOperationRecordingFailure<'operation>>
where
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
    Reconciliation:
        FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
{
    let ProviderOperationCallBasis {
        retained,
        released_at,
    } = basis;
    let outcome = observation.and_then(|observation| {
        let request = request.borrowed()?;
        PreparedProviderInvocation::prepare(request, EffectClass::Compute)?
            .bind(retained.dispatch.attempt.result_admission)?
            .admit_observation(observation, evidence)
    });
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(reason) => {
            return record_consumed_provider_operation(
                ConsumedProviderOperation {
                    dispatch: ProviderOperationRecordingSource::Retained(Box::new(retained)),
                    released_at,
                    disposition: ProviderOperationRecordingDisposition::Uncertainty(
                        ProviderOperationUncertainty::ResponseRejected(reason),
                    ),
                },
                input,
                custody,
                anchor,
                admit_reconciliation,
                admit_rotation,
                sign_observation,
                sign_checkpoint,
            );
        }
    };
    let association = match &outcome {
        AdmittedProviderInvocationOutcome::Succeeded(success) => retained
            .dispatch
            .attempt
            .selection
            .validate_success(success, &retained.dispatch.attempt.context.parents())
            .map_err(OperationAttemptError::from),
        AdmittedProviderInvocationOutcome::Failed(failure) => retained
            .dispatch
            .attempt
            .selection
            .validate_failure(failure, &retained.dispatch.attempt.context.parents())
            .map_err(OperationAttemptError::from)
            .and_then(|()| validate_failure_reference(&retained, input, failure)),
        AdmittedProviderInvocationOutcome::Unavailable(_) => Ok(()),
    }
    .and_then(|()| validate_outcome_release_time(&outcome, released_at));
    if let Err(reason) = association {
        return record_consumed_provider_operation(
            ConsumedProviderOperation {
                dispatch: ProviderOperationRecordingSource::Retained(Box::new(retained)),
                released_at,
                disposition: ProviderOperationRecordingDisposition::Uncertainty(
                    ProviderOperationUncertainty::ResponseAssociation {
                        outcome: Box::new(outcome),
                        reason: Box::new(reason),
                    },
                ),
            },
            input,
            custody,
            anchor,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        );
    }
    let disposition = match outcome {
        AdmittedProviderInvocationOutcome::Unavailable(unavailable) => {
            ProviderOperationRecordingDisposition::Uncertainty(
                ProviderOperationUncertainty::Unavailable(unavailable),
            )
        }
        outcome => ProviderOperationRecordingDisposition::Outcome(outcome),
    };
    record_consumed_provider_operation(
        ConsumedProviderOperation {
            dispatch: ProviderOperationRecordingSource::Retained(Box::new(retained)),
            released_at,
            disposition,
        },
        input,
        custody,
        anchor,
        admit_reconciliation,
        admit_rotation,
        sign_observation,
        sign_checkpoint,
    )
}

/// Rejoin one accepted process-local call to protected durable history.
///
/// This preparation consumes no provider response. It proves that the owned
/// continuation coordinate, reconstructed request, independently supplied
/// result admission, current safe head, checkpoint, database state and anchor
/// all describe the same dispatched attempt.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn prepare_recovered_provider_completion<'operation>(
    continuation: &ProviderOperationContinuationCoordinate,
    request: &OwnedProviderInvocationRequest,
    history: OperationAttemptHistoryInput<'operation>,
    recovered: &'operation RecoveredOperationObservation,
    admission: ProviderInvocationAdmissionContext<'operation>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_record_bytes: usize,
) -> Result<RecoveredProviderOperationDispatch<'operation>, OperationAttemptError> {
    if !continuation.matches(recovered) {
        return Err(OperationAttemptError::Association);
    }
    let recovered = recover_provider_dispatch_under_cancellation_request(history, recovered)?;
    let provider_request = recovered.provider_request()?;
    if request != &OwnedProviderInvocationRequest::capture(provider_request) {
        return Err(OperationAttemptError::Association);
    }
    admission
        .validate_for(recovered.provider_request()?)
        .map_err(|_| OperationAttemptError::Association)?;
    recovered.verify_preparation_custody(custody, anchor, maximum_record_bytes)?;
    Ok(recovered)
}

/// Admit and record one provider completion after protected history recovery.
///
/// The provider call cannot be replayed through this function. Rejected or
/// unlike evidence becomes the existing reconciliation-bearing uncertainty
/// disposition. Recording failures retain the consumed outcome through the
/// existing continuation and competing-head owners.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn record_recovered_provider_operation_completion<
    'operation,
    Rotation,
    ObservationSign,
    CheckpointSign,
    Reconciliation,
>(
    continuation: ProviderOperationContinuationCoordinate,
    request: &OwnedProviderInvocationRequest,
    observation: Result<ProviderInvocationObservation, ProviderInvocationAdmissionError>,
    recovered: RecoveredProviderOperationDispatch<'operation>,
    admission: ProviderInvocationAdmissionContext<'operation>,
    input: &ProviderOperationInvokeInput<'_>,
    evidence: &dyn ProviderInvocationEvidenceAdmission,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_reconciliation: Reconciliation,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<ProviderOperationInvokeResult, ProviderOperationRecordingFailure<'static>>
where
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
    Reconciliation:
        FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
{
    let outcome = observation.and_then(|observation| {
        let provider_request = request.borrowed()?;
        PreparedProviderInvocation::prepare(provider_request, EffectClass::Compute)?
            .bind(admission)?
            .admit_observation(observation, evidence)
    });
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(reason) => {
            return record_consumed_provider_operation(
                ConsumedProviderOperation {
                    dispatch: ProviderOperationRecordingSource::Recovered(Box::new(
                        OwnedRecoveredProviderRecordingBasis::retain(&recovered),
                    )),
                    released_at: continuation.released_at(),
                    disposition: ProviderOperationRecordingDisposition::Uncertainty(
                        ProviderOperationUncertainty::ResponseRejected(reason),
                    ),
                },
                input,
                custody,
                anchor,
                admit_reconciliation,
                admit_rotation,
                sign_observation,
                sign_checkpoint,
            );
        }
    };
    let association = recovered
        .validate_outcome(&outcome)
        .and_then(|()| validate_recovered_failure_reference(&recovered, input, &outcome))
        .and_then(|()| validate_outcome_release_time(&outcome, continuation.released_at()));
    if let Err(reason) = association {
        return record_consumed_provider_operation(
            ConsumedProviderOperation {
                dispatch: ProviderOperationRecordingSource::Recovered(Box::new(
                    OwnedRecoveredProviderRecordingBasis::retain(&recovered),
                )),
                released_at: continuation.released_at(),
                disposition: ProviderOperationRecordingDisposition::Uncertainty(
                    ProviderOperationUncertainty::ResponseAssociation {
                        outcome: Box::new(outcome),
                        reason: Box::new(reason),
                    },
                ),
            },
            input,
            custody,
            anchor,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        );
    }
    let disposition = match outcome {
        AdmittedProviderInvocationOutcome::Unavailable(unavailable) => {
            ProviderOperationRecordingDisposition::Uncertainty(
                ProviderOperationUncertainty::Unavailable(unavailable),
            )
        }
        outcome => ProviderOperationRecordingDisposition::Outcome(outcome),
    };
    record_consumed_provider_operation(
        ConsumedProviderOperation {
            dispatch: ProviderOperationRecordingSource::Recovered(Box::new(
                OwnedRecoveredProviderRecordingBasis::retain(&recovered),
            )),
            released_at: continuation.released_at(),
            disposition,
        },
        input,
        custody,
        anchor,
        admit_reconciliation,
        admit_rotation,
        sign_observation,
        sign_checkpoint,
    )
}

/// Invoke one retained `HostNative` compute attempt exactly once.
///
/// This compatibility composition now uses the same separated preflight,
/// mechanical call, and single-writer recording phases as the resident task
/// owner. It remains synchronous for existing callers.
///
/// # Errors
/// Returns the exact stopped phase. Admission, release, and provider
/// preparation refusals retain the unopened dispatch.
#[allow(clippy::too_many_arguments)]
pub fn invoke_provider_operation<
    'operation,
    Authority,
    Rotation,
    ObservationSign,
    CheckpointSign,
    Provider,
    Reconciliation,
>(
    retained: RetainedProviderOperationDispatch<'operation>,
    input: ProviderOperationInvokeInput<'_>,
    provider: &Provider,
    evidence: &dyn ProviderInvocationEvidenceAdmission,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_reconciliation: Reconciliation,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<ProviderOperationInvokeResult, ProviderOperationInvokeFailure<'operation>>
where
    Authority: AmbientSessionAuthorityPort,
    Rotation: FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    ObservationSign: FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    CheckpointSign: FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    Provider: ProviderInvocationPort,
    Reconciliation:
        FnMut(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
{
    let prepared = prepare_provider_operation_call(retained, &input, authority, custody, anchor)?;
    let observation = invoke_owned_provider_invocation(&prepared.request, provider);
    record_provider_operation_completion(
        prepared.basis,
        &prepared.request,
        observation,
        &input,
        evidence,
        custody,
        anchor,
        admit_reconciliation,
        admit_rotation,
        sign_observation,
        sign_checkpoint,
    )
    .map_err(ProviderOperationInvokeFailure::Recording)
}

/// Record one already-consumed provider call without reopening the provider.
///
/// A pre-commit failure returns a continuation for the same recording head. A
/// definite competing head returns the consumed outcome without retry
/// permission; protected recovery must select the original dispatch and a safe
/// current successor first. Ambiguous database or anchor failures remain with
/// the existing physical custody owner and cannot be rebuilt.
///
/// # Errors
/// Returns a retryable preparation failure when no candidate committed, or the
/// exact physical custody failure when candidate custody is uncertain.
#[allow(clippy::too_many_arguments)]
pub fn record_consumed_provider_operation<
    'operation,
    Rotation,
    ObservationSign,
    CheckpointSign,
    Reconciliation,
>(
    operation: ConsumedProviderOperation<'operation>,
    input: &ProviderOperationInvokeInput<'_>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_reconciliation: Reconciliation,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<ProviderOperationInvokeResult, ProviderOperationRecordingFailure<'operation>>
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
    let predecessor = (
        operation.dispatch.recording_predecessor().0.clone(),
        operation.dispatch.recording_predecessor().1.clone(),
    );
    let checkpoint = (
        operation.dispatch.checkpoint().0.clone(),
        operation.dispatch.checkpoint().1.clone(),
    );
    continue_consumed_provider_recording(
        ConsumedProviderRecordingContinuation {
            operation,
            predecessor,
            checkpoint,
        },
        input,
        custody,
        anchor,
        admit_reconciliation,
        admit_rotation,
        sign_observation,
        sign_checkpoint,
    )
}

/// Record restart ambiguity for one protected provider dispatch.
///
/// Recovery rejoins the exact private selection, dispatched observation,
/// checkpoint, database head, and independent anchor before constructing a new
/// `OutcomeUnknown` successor. The provider port is deliberately absent from
/// this API: an anchored `Dispatched` head may describe a call that already
/// happened, so restart can only retain reconciliation custody.
///
/// # Errors
///
/// Returns a pre-commit operation or projection failure when the recovered
/// history, supplied identities, signer, or reconciliation frontier does not
/// match. Database or anchor uncertainty returns the existing physical custody
/// owner and must be completed through that owner's exact read-back rules.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn record_recovered_provider_operation_unknown<
    'operation,
    Rotation,
    ObservationSign,
    CheckpointSign,
    Reconciliation,
>(
    history: OperationAttemptHistoryInput<'operation>,
    recovered: &'operation RecoveredOperationObservation,
    input: &ProviderOperationInvokeInput<'_>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_reconciliation: Reconciliation,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<ProviderOperationInvokeResult, RecoveredProviderUnknownRecordingFailure>
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
    let prepared = prepare_recovered_provider_unknown_recording(
        &history,
        recovered,
        input,
        custody,
        anchor,
        admit_reconciliation,
        admit_rotation,
        sign_observation,
    )?;
    continue_prepared_recovered_provider_unknown(
        prepared,
        input.resolve_store,
        custody,
        anchor,
        input.checkpoint_authority,
        sign_checkpoint,
    )
}

#[allow(clippy::too_many_arguments)]
fn prepare_recovered_provider_unknown_recording<
    'operation,
    Rotation,
    ObservationSign,
    Reconciliation,
>(
    history: &OperationAttemptHistoryInput<'operation>,
    recovered: &'operation RecoveredOperationObservation,
    input: &ProviderOperationInvokeInput<'_>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_reconciliation: Reconciliation,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
) -> Result<PreparedRecoveredProviderUnknown, RecoveredProviderUnknownRecordingFailure>
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
{
    macro_rules! before_observation_signature {
        ($expression:expr) => {
            match $expression {
                Ok(value) => value,
                Err(failure) => {
                    return Err(RecoveredProviderUnknownRecordingFailure::Preparation(
                        Box::new(failure.into()),
                    ))
                }
            }
        };
    }

    let recovered = before_observation_signature!(prepare_recovered_provider_unknown(
        history, recovered, input,
    ));
    before_observation_signature!(recovered.verify_preparation_custody(
        custody,
        anchor,
        input.maximum_record_bytes,
    ));
    let expected_store = recovered.store().clone();
    let source = ProviderOperationRecordingSource::Recovered(Box::new(
        OwnedRecoveredProviderRecordingBasis::retain(&recovered),
    ));
    let predecessor = (
        source.recording_predecessor().0.clone(),
        source.recording_predecessor().1.clone(),
    );
    let predecessor_checkpoint = (source.checkpoint().0.clone(), source.checkpoint().1.clone());
    before_observation_signature!(
        admit_reconciliation(&input.reconciliation).map_err(OperationAttemptError::from)
    );
    let now = before_observation_signature!(recording_time(
        input,
        (&predecessor.0, &predecessor.1),
        (&predecessor_checkpoint.0, &predecessor_checkpoint.1),
    ));
    before_observation_signature!(signing::validate_runtime_signer(
        input.signing.signing_authority,
        &input.current.at_time(now),
        OPERATION_OBSERVATION_V1_SCHEMA,
    ));
    let uncertainty = ProviderOperationUncertainty::ColdRestartAfterDispatch;
    let diagnostic = before_observation_signature!(provider_uncertainty_diagnostic(
        source.handle().1,
        source.attempt_id(),
        &uncertainty,
    ));
    if diagnostic.canonical_bytes().len() > input.maximum_diagnostic_bytes {
        return Err(RecoveredProviderUnknownRecordingFailure::Preparation(
            Box::new(OperationAttemptError::BoundExceeded.into()),
        ));
    }
    let (diagnostic_sha256, diagnostic_record) = before_observation_signature!(
        record_projection::diagnostic_record(&input.diagnostic_ref, &diagnostic)
            .map_err(OperationObservationCustodyProjectionError::from)
    );
    let observation = before_observation_signature!(issue_terminal_observation(
        &source,
        (&predecessor.0, &predecessor.1),
        input,
        OperationObservationStateV1::OutcomeUnknown {
            diagnostic: ImmutableRecordReferenceV1 {
                record_ref: input.diagnostic_ref.clone(),
                record_sha256: diagnostic_sha256,
            },
            reconciliation: input.reconciliation.clone(),
            effect_receipt: None,
        },
        now,
        admit_rotation,
        sign_observation,
    ));
    Ok(PreparedRecoveredProviderUnknown {
        terminal: Box::new(OwnedRecoveredProviderUnknownTerminal {
            checkpoint_selection: source.checkpoint_selection(),
            handle_ref: source.handle().0.clone(),
            handle: source.handle().1.clone(),
            predecessor_checkpoint,
            predecessor,
            expected_store,
            diagnostic_record,
            observation_ref: input.terminal_ref.clone(),
            observation,
            uncertainty,
            diagnostic,
        }),
        checkpoint: OperationCheckpointInput {
            reference: input.terminal_checkpoint.reference.clone(),
            id: input.terminal_checkpoint.id.clone(),
        },
        checkpoint_issued_at: now,
        checkpoint_schema: input.checkpoint_schema.clone(),
        checkpoint_authority: OwnedRecoveredProviderCheckpointAuthority::retain(
            input.checkpoint_authority,
        ),
        accumulator_limits: input.accumulator_limits,
        failure: None,
    })
}

pub(in crate::resident) fn continue_prepared_recovered_provider_unknown<CheckpointSign>(
    pending: PreparedRecoveredProviderUnknown,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    sign_checkpoint: CheckpointSign,
) -> Result<ProviderOperationInvokeResult, RecoveredProviderUnknownRecordingFailure>
where
    CheckpointSign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    continue_prepared_recovered_provider_unknown_with_attachment(
        pending,
        resolve_store,
        custody,
        anchor,
        checkpoint_authority,
        sign_checkpoint,
        |prepared, reference, checkpoint| prepared.attach_issued_checkpoint(reference, checkpoint),
    )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn continue_prepared_recovered_provider_unknown_with_attachment<
    CheckpointSign,
    Attach,
>(
    mut pending: PreparedRecoveredProviderUnknown,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    sign_checkpoint: CheckpointSign,
    attach: Attach,
) -> Result<ProviderOperationInvokeResult, RecoveredProviderUnknownRecordingFailure>
where
    CheckpointSign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    Attach: for<'prepared> FnOnce(
        PreparedOperationObservationCheckpoint<'prepared>,
        &ResidentRecordRefV1,
        &ControlStateCheckpointV1,
    ) -> Result<
        crate::resident::PreparedOperationObservationCustody,
        OperationObservationCustodyProjectionError,
    >,
{
    if !pending.checkpoint_authority.matches(checkpoint_authority) {
        pending.failure = Some(OperationAttemptError::Association.into());
        return Err(RecoveredProviderUnknownRecordingFailure::Prepared(
            Box::new(pending),
        ));
    }
    let prepared = match pending.terminal.prepare_checkpoint(
        pending.accumulator_limits,
        resolve_store,
        custody,
    ) {
        Ok(prepared) => prepared,
        Err(failure) => {
            pending.failure = Some(failure.into());
            return Err(RecoveredProviderUnknownRecordingFailure::Prepared(
                Box::new(pending),
            ));
        }
    };
    let preimage = match observation_checkpoint_preimage(
        &prepared,
        (
            &pending.terminal.predecessor_checkpoint.0,
            &pending.terminal.predecessor_checkpoint.1,
        ),
        &pending.checkpoint,
        pending.checkpoint_issued_at,
        &pending.checkpoint_schema,
        checkpoint_authority,
    ) {
        Ok(preimage) => preimage,
        Err(failure) => {
            drop(prepared);
            pending.failure = Some(failure.into());
            return Err(RecoveredProviderUnknownRecordingFailure::Prepared(
                Box::new(pending),
            ));
        }
    };
    let checkpoint_value = match prepared.issue_checkpoint_value(
        preimage,
        &pending.checkpoint_schema,
        checkpoint_authority,
        sign_checkpoint,
    ) {
        Ok(checkpoint) => checkpoint,
        Err(failure) => {
            drop(prepared);
            pending.failure = Some(failure.into());
            return Err(RecoveredProviderUnknownRecordingFailure::Prepared(
                Box::new(pending),
            ));
        }
    };
    let physical = match attach(prepared, &pending.checkpoint.reference, &checkpoint_value) {
        Ok(physical) => physical,
        Err(failure) => {
            return Err(RecoveredProviderUnknownRecordingFailure::Issued(Box::new(
                IssuedRecoveredProviderUnknownCheckpoint {
                    prepared: Box::new(pending),
                    checkpoint_value: Box::new(checkpoint_value),
                    failure: Some(failure.into()),
                },
            )));
        }
    };
    finish_recovered_provider_unknown_physical(pending, checkpoint_value, physical, custody, anchor)
}

pub(in crate::resident) fn continue_issued_recovered_provider_unknown(
    mut pending: IssuedRecoveredProviderUnknownCheckpoint,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<ProviderOperationInvokeResult, RecoveredProviderUnknownRecordingFailure> {
    let prepared = match pending.prepared.terminal.prepare_checkpoint(
        pending.prepared.accumulator_limits,
        resolve_store,
        custody,
    ) {
        Ok(prepared) => prepared,
        Err(failure) => {
            pending.failure = Some(failure.into());
            return Err(RecoveredProviderUnknownRecordingFailure::Issued(Box::new(
                pending,
            )));
        }
    };
    let physical = match prepared.attach_issued_checkpoint(
        &pending.prepared.checkpoint.reference,
        &pending.checkpoint_value,
    ) {
        Ok(physical) => physical,
        Err(failure) => {
            pending.failure = Some(failure.into());
            return Err(RecoveredProviderUnknownRecordingFailure::Issued(Box::new(
                pending,
            )));
        }
    };
    finish_recovered_provider_unknown_physical(
        *pending.prepared,
        *pending.checkpoint_value,
        physical,
        custody,
        anchor,
    )
}

fn finish_recovered_provider_unknown_physical(
    mut pending: PreparedRecoveredProviderUnknown,
    checkpoint_value: ControlStateCheckpointV1,
    physical: crate::resident::PreparedOperationObservationCustody,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<ProviderOperationInvokeResult, RecoveredProviderUnknownRecordingFailure> {
    let terminal = pending.terminal;
    let finalizer = match terminal.into_finalizer() {
        Ok(finalizer) => finalizer,
        Err((terminal, failure)) => {
            pending.terminal = terminal;
            return Err(RecoveredProviderUnknownRecordingFailure::Issued(Box::new(
                IssuedRecoveredProviderUnknownCheckpoint {
                    prepared: Box::new(pending),
                    checkpoint_value: Box::new(checkpoint_value),
                    failure: Some(failure),
                },
            )));
        }
    };
    let checkpoint = (pending.checkpoint.reference, checkpoint_value);
    match commit_prepared_checkpoint(physical, checkpoint, custody, anchor) {
        Ok((completed, checkpoint)) => finalizer.finish(completed, checkpoint).map_err(|failure| {
            RecoveredProviderUnknownRecordingFailure::Finalization(Box::new(failure))
        }),
        Err(failure) => Err(RecoveredProviderUnknownRecordingFailure::Custody(Box::new(
            provider_custody_continuation(finalizer, failure),
        ))),
    }
}

/// Purely validate one historical provider dispatch and all planned output references.
pub(in crate::resident) fn validate_recovered_provider_unknown(
    history: &OperationAttemptHistoryInput<'_>,
    recovered: &RecoveredOperationObservation,
    input: &ProviderOperationInvokeInput<'_>,
) -> Result<(), OperationAttemptError> {
    prepare_recovered_provider_unknown(history, recovered, input).map(drop)
}

fn prepare_recovered_provider_unknown<'operation>(
    history: &OperationAttemptHistoryInput<'operation>,
    recovered: &'operation RecoveredOperationObservation,
    input: &ProviderOperationInvokeInput<'_>,
) -> Result<RecoveredProviderOperationDispatch<'operation>, OperationAttemptError> {
    let recovered = recover_provider_dispatch_under_cancellation_request(
        OperationAttemptHistoryInput {
            operation: history.operation,
            request: history.request,
            session: history.session,
            lease: history.lease,
            generation: history.generation,
            dispatched: history.dispatched,
            selection: history.selection,
            maximum_selection_bytes: history.maximum_selection_bytes,
        },
        recovered,
    )?;
    validate_recovered_references(&recovered, input)?;
    recovered.provider_request()?;
    Ok(recovered)
}

enum ProviderTerminalRecordingFailure {
    Preparation {
        disposition: Box<ProviderOperationRecordingDisposition>,
        failure: Box<OperationAppendFailure>,
    },
    Append {
        finalizer: Box<ProviderOperationTerminalFinalizer>,
        failure: OperationAppendFailure,
    },
    Finalization(ProviderOperationRecordingFinalizationFailure),
}

fn provider_custody_continuation(
    finalizer: Box<ProviderOperationTerminalFinalizer>,
    failure: OperationAppendFailure,
) -> ProviderOperationRecordingCustody {
    match failure {
        OperationAppendFailure::Commit {
            checkpoint,
            failure,
        } => ProviderOperationRecordingCustody {
            finalizer,
            checkpoint,
            phase: ProviderOperationRecordingCustodyPhase::Commit(failure),
        },
        OperationAppendFailure::Anchor {
            checkpoint,
            failure,
        } => ProviderOperationRecordingCustody {
            finalizer,
            checkpoint,
            phase: ProviderOperationRecordingCustodyPhase::Anchor(
                ProviderOperationAnchorCustody::Failed(failure),
            ),
        },
        OperationAppendFailure::Operation(_) | OperationAppendFailure::Projection(_) => {
            unreachable!("pre-custody failures cannot create a custody continuation")
        }
    }
}

/// Continue recording an already-consumed provider outcome at its admitted head.
///
/// This function cannot invoke or reconstruct a provider request. The
/// continuation was either retained after a pre-commit failure or created from
/// protected recovery after a definite competing head.
///
/// # Errors
///
/// Returns the continuation after a pre-commit failure, the consumed outcome
/// after a definite concurrent head, or the exact custody owner after a commit
/// or anchor outcome that cannot be rebuilt safely.
#[allow(clippy::too_many_arguments)]
pub fn continue_consumed_provider_recording<
    'operation,
    Rotation,
    ObservationSign,
    CheckpointSign,
    Reconciliation,
>(
    continuation: ConsumedProviderRecordingContinuation<'operation>,
    input: &ProviderOperationInvokeInput<'_>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_reconciliation: Reconciliation,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<ProviderOperationInvokeResult, ProviderOperationRecordingFailure<'operation>>
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
    let ConsumedProviderRecordingContinuation {
        operation,
        predecessor,
        checkpoint,
    } = continuation;
    let ConsumedProviderOperation {
        dispatch,
        released_at,
        disposition,
    } = operation;
    if let Err(reason) = validate_recording_source_references(&dispatch, input)
        .and_then(|()| validate_recording_position(&dispatch, &predecessor, &checkpoint, input))
    {
        return Err(ProviderOperationRecordingFailure::Preparation {
            continuation: Box::new(ConsumedProviderRecordingContinuation {
                operation: ConsumedProviderOperation {
                    dispatch,
                    released_at,
                    disposition,
                },
                predecessor,
                checkpoint,
            }),
            reason: Box::new(ProviderOperationRecordingPreparationError::Operation(
                reason,
            )),
        });
    }
    let recorded = match disposition {
        ProviderOperationRecordingDisposition::Outcome(outcome) => record_terminal(
            &dispatch,
            (&predecessor.0, &predecessor.1),
            (&checkpoint.0, &checkpoint.1),
            input,
            outcome,
            released_at,
            custody,
            anchor,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        ),
        ProviderOperationRecordingDisposition::Uncertainty(uncertainty) => record_unknown_inner(
            &dispatch,
            (&predecessor.0, &predecessor.1),
            (&checkpoint.0, &checkpoint.1),
            input,
            uncertainty,
            custody,
            anchor,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        ),
    };
    match recorded {
        Ok(result) => Ok(result),
        Err(ProviderTerminalRecordingFailure::Finalization(failure)) => {
            Err(ProviderOperationRecordingFailure::Finalization {
                failure: Box::new(failure),
            })
        }
        Err(ProviderTerminalRecordingFailure::Append {
            finalizer,
            failure: OperationAppendFailure::Operation(reason),
        }) => Err(ProviderOperationRecordingFailure::Preparation {
            continuation: Box::new(ConsumedProviderRecordingContinuation {
                operation: ConsumedProviderOperation {
                    dispatch,
                    released_at,
                    disposition: finalizer.into_disposition(),
                },
                predecessor,
                checkpoint,
            }),
            reason: Box::new(ProviderOperationRecordingPreparationError::Operation(
                reason,
            )),
        }),
        Err(ProviderTerminalRecordingFailure::Append {
            finalizer,
            failure: OperationAppendFailure::Projection(reason),
        }) => {
            let operation = ConsumedProviderOperation {
                dispatch,
                released_at,
                disposition: finalizer.into_disposition(),
            };
            if is_definite_projection_head_conflict(
                &reason,
                &operation.dispatch,
                &predecessor,
                input,
                custody,
            ) {
                Err(ProviderOperationRecordingFailure::ConcurrentHead {
                    conflict: Box::new(ProviderOperationRecordingConflict { operation }),
                })
            } else {
                Err(ProviderOperationRecordingFailure::Preparation {
                    continuation: Box::new(ConsumedProviderRecordingContinuation {
                        operation,
                        predecessor,
                        checkpoint,
                    }),
                    reason: Box::new(ProviderOperationRecordingPreparationError::Projection(
                        reason,
                    )),
                })
            }
        }
        Err(ProviderTerminalRecordingFailure::Preparation {
            disposition,
            failure,
        }) => match *failure {
            OperationAppendFailure::Operation(reason) => {
                Err(ProviderOperationRecordingFailure::Preparation {
                    continuation: Box::new(ConsumedProviderRecordingContinuation {
                        operation: ConsumedProviderOperation {
                            dispatch,
                            released_at,
                            disposition: *disposition,
                        },
                        predecessor,
                        checkpoint,
                    }),
                    reason: Box::new(ProviderOperationRecordingPreparationError::Operation(
                        reason,
                    )),
                })
            }
            OperationAppendFailure::Projection(reason) => {
                let operation = ConsumedProviderOperation {
                    dispatch,
                    released_at,
                    disposition: *disposition,
                };
                if is_definite_projection_head_conflict(
                    &reason,
                    &operation.dispatch,
                    &predecessor,
                    input,
                    custody,
                ) {
                    Err(ProviderOperationRecordingFailure::ConcurrentHead {
                        conflict: Box::new(ProviderOperationRecordingConflict { operation }),
                    })
                } else {
                    Err(ProviderOperationRecordingFailure::Preparation {
                        continuation: Box::new(ConsumedProviderRecordingContinuation {
                            operation,
                            predecessor,
                            checkpoint,
                        }),
                        reason: Box::new(ProviderOperationRecordingPreparationError::Projection(
                            reason,
                        )),
                    })
                }
            }
            OperationAppendFailure::Commit { .. } | OperationAppendFailure::Anchor { .. } => {
                unreachable!("preparation failures are limited to operation and projection")
            }
        },
        Err(ProviderTerminalRecordingFailure::Append {
            finalizer,
            failure: failure @ OperationAppendFailure::Commit { .. },
        }) if is_definite_candidate_conflict(&failure) => {
            match classify_candidate_conflict(&dispatch, &predecessor, input, custody) {
                CandidateConflictDisposition::ConcurrentHead => {
                    Err(ProviderOperationRecordingFailure::ConcurrentHead {
                        conflict: Box::new(ProviderOperationRecordingConflict {
                            operation: ConsumedProviderOperation {
                                dispatch,
                                released_at,
                                disposition: finalizer.into_disposition(),
                            },
                        }),
                    })
                }
                CandidateConflictDisposition::StoreTransactionDrift => {
                    drop(failure);
                    Err(ProviderOperationRecordingFailure::Preparation {
                        continuation: Box::new(ConsumedProviderRecordingContinuation {
                            operation: ConsumedProviderOperation {
                                dispatch,
                                released_at,
                                disposition: finalizer.into_disposition(),
                            },
                            predecessor,
                            checkpoint,
                        }),
                        reason: Box::new(
                            ProviderOperationRecordingPreparationError::StoreTransactionDrift,
                        ),
                    })
                }
                CandidateConflictDisposition::Unclassified => {
                    Err(ProviderOperationRecordingFailure::Custody {
                        custody: Box::new(provider_custody_continuation(finalizer, failure)),
                    })
                }
            }
        }
        Err(ProviderTerminalRecordingFailure::Append {
            finalizer,
            failure:
                failure
                @ (OperationAppendFailure::Commit { .. } | OperationAppendFailure::Anchor { .. }),
        }) => Err(ProviderOperationRecordingFailure::Custody {
            custody: Box::new(provider_custody_continuation(finalizer, failure)),
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn record_unknown_inner<Rotation, ObservationSign, CheckpointSign, Reconciliation>(
    retained: &ProviderOperationRecordingSource<'_>,
    predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    input: &ProviderOperationInvokeInput<'_>,
    uncertainty: ProviderOperationUncertainty,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_reconciliation: Reconciliation,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<ProviderOperationInvokeResult, ProviderTerminalRecordingFailure>
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
    macro_rules! retain_uncertainty_on_failure {
        ($expression:expr) => {
            match $expression {
                Ok(value) => value,
                Err(failure) => {
                    return Err(ProviderTerminalRecordingFailure::Preparation {
                        disposition: Box::new(ProviderOperationRecordingDisposition::Uncertainty(
                            uncertainty,
                        )),
                        failure: Box::new(failure.into()),
                    })
                }
            }
        };
    }
    retain_uncertainty_on_failure!(
        admit_reconciliation(&input.reconciliation).map_err(OperationAttemptError::from)
    );
    let now = retain_uncertainty_on_failure!(recording_time(input, predecessor, checkpoint));
    retain_uncertainty_on_failure!(signing::validate_runtime_signer(
        input.signing.signing_authority,
        &input.current.at_time(now),
        OPERATION_OBSERVATION_V1_SCHEMA,
    ));
    let diagnostic = retain_uncertainty_on_failure!(provider_uncertainty_diagnostic(
        retained.handle().1,
        retained.attempt_id(),
        &uncertainty,
    ));
    if diagnostic.canonical_bytes().len() > input.maximum_diagnostic_bytes {
        return Err(ProviderTerminalRecordingFailure::Preparation {
            disposition: Box::new(ProviderOperationRecordingDisposition::Uncertainty(
                uncertainty,
            )),
            failure: Box::new(OperationAttemptError::BoundExceeded.into()),
        });
    }
    let (diagnostic_sha256, diagnostic_record) = retain_uncertainty_on_failure!(
        record_projection::diagnostic_record(&input.diagnostic_ref, &diagnostic)
            .map_err(OperationObservationCustodyProjectionError::from)
    );
    let observation = retain_uncertainty_on_failure!(issue_terminal_observation(
        retained,
        predecessor,
        input,
        OperationObservationStateV1::OutcomeUnknown {
            diagnostic: ImmutableRecordReferenceV1 {
                record_ref: input.diagnostic_ref.clone(),
                record_sha256: diagnostic_sha256,
            },
            reconciliation: input.reconciliation.clone(),
            effect_receipt: None,
        },
        now,
        admit_rotation,
        sign_observation,
    ));
    let associated = OperationCustodyRecord::Diagnostic(diagnostic_record);
    let append = retain_uncertainty_on_failure!(terminal_append(
        retained,
        predecessor,
        &input.terminal_ref,
        &observation,
        input.resolve_store,
    ))
    .with_associated_record(associated.clone());
    let finalizer = prepare_provider_terminal_finalizer(
        retained,
        predecessor,
        &input.terminal_ref,
        &observation,
        append.store().clone(),
        vec![associated],
        ProviderOperationFinalDisposition::OutcomeUnknown {
            uncertainty,
            diagnostic,
        },
    )
    .map_err(|failure| ProviderTerminalRecordingFailure::Preparation {
        disposition: Box::new(failure.0),
        failure: Box::new(failure.1),
    })?;
    commit_provider_terminal(
        append,
        retained,
        checkpoint,
        input,
        now,
        custody,
        anchor,
        sign_checkpoint,
        finalizer,
    )
}

fn validate_release(
    retained: &RetainedProviderOperationDispatch<'_>,
    input: &ProviderOperationInvokeInput<'_>,
    started: Instant,
    authority: &mut impl AmbientSessionAuthorityPort,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<UnixTimeV1, OperationAttemptError> {
    let context = retained.context();
    let floor = input
        .current
        .now
        .max(*retained.observation().1.observed_at());
    let before = sample_time(input.clock, floor)?;
    context.validate_current_at(&input.current.at_time(before), authority)?;
    context.require_context_ready()?;
    retained.verify_release_custody(
        input.resolve_store,
        custody,
        anchor,
        input.maximum_record_bytes,
    )?;
    let checked_at = sample_time(input.clock, before)?;
    context.validate_current_at(&input.current.at_time(checked_at), authority)?;
    let release_at = sample_time(input.clock, checked_at)?;
    context.validate_time_at(&input.current.at_time(release_at))?;
    signing::validate_runtime_signer(
        input.signing.signing_authority,
        &input.current.at_time(release_at),
        OPERATION_OBSERVATION_V1_SCHEMA,
    )?;
    if started.elapsed() >= Duration::from_millis(context.lease.timeout_millis().get()) {
        return Err(OperationAttemptError::Deadline);
    }
    Ok(release_at)
}

#[allow(clippy::too_many_arguments)]
fn prepare_provider_terminal_finalizer(
    retained: &ProviderOperationRecordingSource<'_>,
    predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
    observation_ref: &ResidentRecordRefV1,
    observation: &OperationObservationV1,
    expected_store: AuthorityStoreCoordinate,
    associated: Vec<OperationCustodyRecord>,
    disposition: ProviderOperationFinalDisposition,
) -> Result<
    Box<ProviderOperationTerminalFinalizer>,
    Box<(
        ProviderOperationRecordingDisposition,
        OperationAppendFailure,
    )>,
> {
    let projected = (|| {
        let predecessor_record = record_projection::record(
            record_projection::coordinate(
                predecessor.0,
                predecessor
                    .1
                    .canonical_operation_observation_sha256()
                    .as_str(),
            )
            .map_err(OperationObservationCustodyProjectionError::from)?,
            predecessor.1,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let observation_record = record_projection::record(
            record_projection::coordinate(
                observation_ref,
                observation
                    .canonical_operation_observation_sha256()
                    .as_str(),
            )
            .map_err(OperationObservationCustodyProjectionError::from)?,
            observation,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        if !retained
            .checkpoint_selection()
            .matches_store(&expected_store)
        {
            return Err(OperationAppendFailure::from(
                OperationAttemptError::Association,
            ));
        }
        let mut expected_records = Vec::with_capacity(associated.len() + 1);
        expected_records.push(OperationCustodyRecord::OperationObservation(
            observation_record,
        ));
        expected_records.extend(associated);
        Ok((predecessor_record, expected_records))
    })();
    match projected {
        Ok((predecessor_record, expected_records)) => {
            Ok(Box::new(ProviderOperationTerminalFinalizer {
                disposition,
                observation: (observation_ref.clone(), observation.clone()),
                expected_store,
                predecessor_record,
                expected_records: expected_records.into_boxed_slice(),
            }))
        }
        Err(failure) => Err(Box::new((disposition.into_recording(), failure))),
    }
}

#[allow(clippy::too_many_arguments)]
fn commit_provider_terminal<CheckpointSign>(
    append: PreparedOperationObservationAppend<'_>,
    retained: &ProviderOperationRecordingSource<'_>,
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    input: &ProviderOperationInvokeInput<'_>,
    now: UnixTimeV1,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    sign_checkpoint: CheckpointSign,
    finalizer: Box<ProviderOperationTerminalFinalizer>,
) -> Result<ProviderOperationInvokeResult, ProviderTerminalRecordingFailure>
where
    CheckpointSign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let completed = append
        .prepare_checkpoint(
            retained.checkpoint_selection(),
            checkpoint.0,
            checkpoint.1,
            input.accumulator_limits,
            custody,
        )
        .map_err(OperationAppendFailure::from)
        .and_then(|prepared| {
            complete_prepared_checkpoint(
                prepared,
                checkpoint,
                OperationCheckpointInput {
                    reference: input.terminal_checkpoint.reference.clone(),
                    id: input.terminal_checkpoint.id.clone(),
                },
                now,
                input.checkpoint_schema,
                input.checkpoint_authority,
                custody,
                anchor,
                sign_checkpoint,
            )
        });
    match completed {
        Ok((custody, checkpoint)) => finalizer
            .finish(custody, checkpoint)
            .map_err(ProviderTerminalRecordingFailure::Finalization),
        Err(failure) => Err(ProviderTerminalRecordingFailure::Append { finalizer, failure }),
    }
}

#[allow(clippy::too_many_arguments)]
fn record_terminal<Rotation, ObservationSign, CheckpointSign>(
    retained: &ProviderOperationRecordingSource<'_>,
    predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    input: &ProviderOperationInvokeInput<'_>,
    outcome: AdmittedProviderInvocationOutcome,
    released_at: UnixTimeV1,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<ProviderOperationInvokeResult, ProviderTerminalRecordingFailure>
where
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
    macro_rules! retain_outcome_on_failure {
        ($expression:expr) => {
            match $expression {
                Ok(value) => value,
                Err(failure) => {
                    return Err(ProviderTerminalRecordingFailure::Preparation {
                        disposition: Box::new(ProviderOperationRecordingDisposition::Outcome(
                            outcome.clone(),
                        )),
                        failure: Box::new(failure.into()),
                    })
                }
            }
        };
    }
    let now = retain_outcome_on_failure!(recording_time(input, predecessor, checkpoint));
    retain_outcome_on_failure!(signing::validate_runtime_signer(
        input.signing.signing_authority,
        &input.current.at_time(now),
        OPERATION_OBSERVATION_V1_SCHEMA,
    ));
    match &outcome {
        AdmittedProviderInvocationOutcome::Succeeded(success) => {
            let result = success.result();
            retain_outcome_on_failure!(validate_result_time(retained, result, released_at, now,));
            let result_record = retain_outcome_on_failure!(record_projection::coordinate(
                &input.result_ref,
                result.canonical_result_sha256().as_str(),
            )
            .and_then(|coordinate| record_projection::record(coordinate, result))
            .map_err(OperationObservationCustodyProjectionError::from));
            let observation = retain_outcome_on_failure!(issue_terminal_observation(
                retained,
                predecessor,
                input,
                OperationObservationStateV1::Succeeded {
                    result: CapabilityResultReferenceV1 {
                        record_ref: input.result_ref.clone(),
                        record_sha256: result.canonical_result_sha256().clone(),
                    },
                    effect_receipt: None,
                },
                now,
                admit_rotation,
                sign_observation,
            ));
            let associated = OperationCustodyRecord::CapabilityResult(result_record);
            let append = retain_outcome_on_failure!(terminal_append(
                retained,
                predecessor,
                &input.terminal_ref,
                &observation,
                input.resolve_store,
            ))
            .with_associated_record(associated.clone());
            let finalizer = prepare_provider_terminal_finalizer(
                retained,
                predecessor,
                &input.terminal_ref,
                &observation,
                append.store().clone(),
                vec![associated],
                ProviderOperationFinalDisposition::Completed {
                    outcome: outcome.clone(),
                    diagnostic: None,
                },
            )
            .map_err(|failure| ProviderTerminalRecordingFailure::Preparation {
                disposition: Box::new(failure.0),
                failure: Box::new(failure.1),
            })?;
            commit_provider_terminal(
                append,
                retained,
                checkpoint,
                input,
                now,
                custody,
                anchor,
                sign_checkpoint,
                finalizer,
            )
        }
        AdmittedProviderInvocationOutcome::Failed(failure) => {
            let result = failure.result();
            retain_outcome_on_failure!(validate_result_time(retained, result, released_at, now,));
            let diagnostic = retain_outcome_on_failure!(provider_failure_diagnostic(failure));
            if diagnostic.canonical_bytes().len() > input.maximum_diagnostic_bytes {
                return Err(ProviderTerminalRecordingFailure::Preparation {
                    disposition: Box::new(ProviderOperationRecordingDisposition::Outcome(
                        outcome.clone(),
                    )),
                    failure: Box::new(OperationAttemptError::BoundExceeded.into()),
                });
            }
            let result_record = retain_outcome_on_failure!(record_projection::coordinate(
                &input.result_ref,
                result.canonical_result_sha256().as_str(),
            )
            .and_then(|coordinate| record_projection::record(coordinate, result))
            .map_err(OperationObservationCustodyProjectionError::from));
            let failure_record = retain_outcome_on_failure!(record_projection::coordinate(
                failure.failure_ref(),
                failure.failure().canonical_failure_sha256().as_str(),
            )
            .and_then(|coordinate| record_projection::record(coordinate, failure.failure()))
            .map_err(OperationObservationCustodyProjectionError::from));
            let (diagnostic_sha256, diagnostic_record) = retain_outcome_on_failure!(
                record_projection::diagnostic_record(&input.diagnostic_ref, &diagnostic,)
                    .map_err(OperationObservationCustodyProjectionError::from)
            );
            let observation = retain_outcome_on_failure!(issue_terminal_observation(
                retained,
                predecessor,
                input,
                OperationObservationStateV1::Failed {
                    result: Some(CapabilityResultReferenceV1 {
                        record_ref: input.result_ref.clone(),
                        record_sha256: result.canonical_result_sha256().clone(),
                    }),
                    diagnostic: ImmutableRecordReferenceV1 {
                        record_ref: input.diagnostic_ref.clone(),
                        record_sha256: diagnostic_sha256,
                    },
                },
                now,
                admit_rotation,
                sign_observation,
            ));
            let associated = vec![
                OperationCustodyRecord::AdapterFailure(failure_record),
                OperationCustodyRecord::CapabilityResult(result_record),
                OperationCustodyRecord::Diagnostic(diagnostic_record),
            ];
            let append = retain_outcome_on_failure!(terminal_append(
                retained,
                predecessor,
                &input.terminal_ref,
                &observation,
                input.resolve_store,
            ))
            .with_provider_failure_records(
                associated[0].clone(),
                associated[1].clone(),
                associated[2].clone(),
            );
            let finalizer = prepare_provider_terminal_finalizer(
                retained,
                predecessor,
                &input.terminal_ref,
                &observation,
                append.store().clone(),
                associated,
                ProviderOperationFinalDisposition::Completed {
                    outcome: outcome.clone(),
                    diagnostic: Some(diagnostic),
                },
            )
            .map_err(|failure| ProviderTerminalRecordingFailure::Preparation {
                disposition: Box::new(failure.0),
                failure: Box::new(failure.1),
            })?;
            commit_provider_terminal(
                append,
                retained,
                checkpoint,
                input,
                now,
                custody,
                anchor,
                sign_checkpoint,
                finalizer,
            )
        }
        AdmittedProviderInvocationOutcome::Unavailable(_) => {
            Err(ProviderTerminalRecordingFailure::Preparation {
                disposition: Box::new(ProviderOperationRecordingDisposition::Outcome(outcome)),
                failure: Box::new(OperationAttemptError::Association.into()),
            })
        }
    }
}

fn issue_terminal_observation<Rotation, Sign>(
    retained: &ProviderOperationRecordingSource<'_>,
    predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
    input: &ProviderOperationInvokeInput<'_>,
    state: OperationObservationStateV1,
    now: UnixTimeV1,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<OperationObservationV1, OperationAttemptError>
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
    let (handle_ref, handle) = retained.handle();
    signing::issue_observation(
        &OperationObservationAdmissionContextV1 {
            signed: input.signing,
            handle_ref,
            handle,
            predecessor: Some(predecessor),
        },
        state,
        now,
        admit_rotation,
        sign,
    )
}

fn terminal_append<'a, 'operation>(
    retained: &'a ProviderOperationRecordingSource<'operation>,
    predecessor: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    observation_ref: &'a ResidentRecordRefV1,
    observation: &'a OperationObservationV1,
    resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
) -> Result<PreparedOperationObservationAppend<'a>, OperationObservationCustodyProjectionError> {
    let (handle_ref, handle) = retained.handle();
    prepare_operation_observation_append(
        OperationObservationTransition::new(
            handle_ref,
            handle,
            predecessor.0,
            predecessor.1,
            observation_ref,
            observation,
        ),
        resolve_store,
    )
}

type OwnedProviderRecordingPosition = (
    (ResidentRecordRefV1, OperationObservationV1),
    (ResidentRecordRefV1, ControlStateCheckpointV1),
);

fn recovered_provider_recording_position(
    retained: &ProviderOperationRecordingSource<'_>,
    recovered: &RecoveredOperationObservation,
) -> Result<OwnedProviderRecordingPosition, OperationAttemptError> {
    let dispatched = retained.observation();
    let selected = recovered.selected();
    let head = recovered.selected_head();
    let same_dispatch = selected.0 == dispatched.0 && selected.1 == dispatched.1;
    let safe_head = if head.0 == dispatched.0 && head.1 == dispatched.1 {
        true
    } else {
        matches!(
            head.1.state(),
            OperationObservationStateV1::CancellationRequested { .. }
        ) && head.1.predecessor_ref().as_ref() == Some(dispatched.0)
            && head.1.predecessor_sha256().as_ref()
                == Some(dispatched.1.canonical_operation_observation_sha256())
    };
    if !retained
        .checkpoint_selection()
        .matches_store(recovered.store())
        || !same_dispatch
        || !safe_head
        || head.1.state().is_terminal()
    {
        return Err(OperationAttemptError::Association);
    }
    let checkpoint = recovered.current_checkpoint();
    Ok((
        (head.0.clone(), head.1.clone()),
        (checkpoint.0.clone(), checkpoint.1.clone()),
    ))
}

fn validate_recording_position(
    retained: &ProviderOperationRecordingSource<'_>,
    predecessor: &(ResidentRecordRefV1, OperationObservationV1),
    checkpoint: &(ResidentRecordRefV1, ControlStateCheckpointV1),
    input: &ProviderOperationInvokeInput<'_>,
) -> Result<(), OperationAttemptError> {
    let (handle_ref, handle) = retained.handle();
    if predecessor.1.operation_handle_ref() != handle_ref
        || predecessor.1.operation_handle_sha256() != handle.canonical_operation_handle_sha256()
        || predecessor.1.state().is_terminal()
    {
        return Err(OperationAttemptError::Association);
    }
    let planned = [
        &input.result_ref,
        &input.diagnostic_ref,
        &input.reconciliation.record_ref,
        &input.terminal_ref,
        &input.terminal_checkpoint.reference,
    ];
    let request_ref = match predecessor.1.state() {
        OperationObservationStateV1::CancellationRequested { request } => Some(&request.record_ref),
        _ => None,
    };
    if planned.iter().any(|reference| {
        *reference == &predecessor.0
            || *reference == &checkpoint.0
            || predecessor.1.predecessor_ref().as_ref() == Some(*reference)
            || request_ref == Some(*reference)
    }) {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../../tests/internal/resident/operation_dispatch/provider/execution.rs"]
mod tests;

fn recording_time(
    input: &ProviderOperationInvokeInput<'_>,
    predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
) -> Result<UnixTimeV1, OperationAttemptError> {
    let floor = input
        .current
        .now
        .max(*predecessor.1.observed_at())
        .max(*checkpoint.1.issued_at());
    sample_time(input.clock, floor)
}

fn is_definite_candidate_conflict(failure: &OperationAppendFailure) -> bool {
    matches!(
        failure,
        OperationAppendFailure::Commit { failure, .. }
            if matches!(
                failure.reason(),
                OperationObservationCustodyCommitFailureReason::Database(
                    OperationCustodyDatabaseFailureReason::Store(error)
                ) if error.kind() == ControlStoreErrorKind::CandidateConflict
            )
    )
}

enum CandidateConflictDisposition {
    ConcurrentHead,
    StoreTransactionDrift,
    Unclassified,
}

fn classify_candidate_conflict(
    retained: &ProviderOperationRecordingSource<'_>,
    predecessor: &(ResidentRecordRefV1, OperationObservationV1),
    input: &ProviderOperationInvokeInput<'_>,
    custody: &mut dyn OperationCustodyPort,
) -> CandidateConflictDisposition {
    let Ok(store) = input.resolve_store.resolve(&predecessor.1) else {
        return CandidateConflictDisposition::Unclassified;
    };
    if !retained.checkpoint_selection().matches_store(&store) {
        return CandidateConflictDisposition::Unclassified;
    }
    let (handle_ref, handle) = retained.handle();
    let Ok(Some(current)) = crate::resident::read_operation_observation_for_handle(
        custody,
        &store,
        handle_ref,
        handle,
        input.maximum_record_bytes,
    ) else {
        return CandidateConflictDisposition::Unclassified;
    };
    if current.observation().coordinate().record_ref() != &predecessor.0
        || current.observation().coordinate().record_sha256().as_str()
            != predecessor
                .1
                .canonical_operation_observation_sha256()
                .as_str()
    {
        CandidateConflictDisposition::ConcurrentHead
    } else {
        CandidateConflictDisposition::StoreTransactionDrift
    }
}

fn is_definite_projection_head_conflict(
    failure: &OperationObservationCustodyProjectionError,
    retained: &ProviderOperationRecordingSource<'_>,
    predecessor: &(ResidentRecordRefV1, OperationObservationV1),
    input: &ProviderOperationInvokeInput<'_>,
    custody: &mut dyn OperationCustodyPort,
) -> bool {
    if !matches!(
        failure,
        OperationObservationCustodyProjectionError::AccumulatorPreparation(
            pale_ports::OperationAccumulatorPreparationError::Store(error)
        ) if error.kind() == ControlStoreErrorKind::CandidateConflict
    ) {
        return false;
    }
    let Ok(store) = input.resolve_store.resolve(&predecessor.1) else {
        return false;
    };
    if !retained.checkpoint_selection().matches_store(&store) {
        return false;
    }
    let (handle_ref, handle) = retained.handle();
    let Ok(Some(current)) = crate::resident::read_operation_observation_for_handle(
        custody,
        &store,
        handle_ref,
        handle,
        input.maximum_record_bytes,
    ) else {
        return false;
    };
    current.observation().coordinate().record_ref() != &predecessor.0
        || current.observation().coordinate().record_sha256().as_str()
            != predecessor
                .1
                .canonical_operation_observation_sha256()
                .as_str()
}

fn validate_failure_reference(
    retained: &RetainedProviderOperationDispatch<'_>,
    input: &ProviderOperationInvokeInput<'_>,
    failure: &crate::provider_invocation::AdmittedProviderInvocationFailure,
) -> Result<(), OperationAttemptError> {
    let context = retained.context();
    let planned = [
        &input.result_ref,
        &input.diagnostic_ref,
        &input.reconciliation.record_ref,
        &input.terminal_ref,
        &input.terminal_checkpoint.reference,
    ];
    let existing = [
        context.operation.handle().0,
        context.operation.handle().1.operation_lease_ref(),
        context.operation.request_coordinate().record_ref(),
        &context.selection_ref,
        &context.observation_ref,
        retained.observation().0,
        retained.checkpoint().0,
    ];
    if planned.contains(&failure.failure_ref()) || existing.contains(&failure.failure_ref()) {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

fn validate_recovered_failure_reference(
    recovered: &RecoveredProviderOperationDispatch<'_>,
    input: &ProviderOperationInvokeInput<'_>,
    outcome: &AdmittedProviderInvocationOutcome,
) -> Result<(), OperationAttemptError> {
    let AdmittedProviderInvocationOutcome::Failed(failure) = outcome else {
        return Ok(());
    };
    let operation = recovered.operation();
    let planned = [
        &input.result_ref,
        &input.diagnostic_ref,
        &input.reconciliation.record_ref,
        &input.terminal_ref,
        &input.terminal_checkpoint.reference,
    ];
    let existing = [
        operation.handle().0,
        operation.handle().1.operation_lease_ref(),
        operation.request_coordinate().record_ref(),
        recovered.selection_ref(),
        recovered.preparation_observation_ref(),
        recovered.observation().0,
        recovered.recording_predecessor().0,
        recovered.checkpoint().0,
    ];
    if planned.contains(&failure.failure_ref()) || existing.contains(&failure.failure_ref()) {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

fn validate_result_time(
    retained: &ProviderOperationRecordingSource<'_>,
    result: &CapabilityResultV1,
    released_at: UnixTimeV1,
    observed_at: UnixTimeV1,
) -> Result<(), OperationAttemptError> {
    if result.started_at() < &released_at
        || result.started_at() < retained.observation().1.observed_at()
        || result.completed_at() < result.started_at()
        || result.completed_at() > &observed_at
        || observed_at > *retained.handle().1.retain_until()
    {
        return Err(OperationAttemptError::TimeOrder);
    }
    Ok(())
}

fn validate_outcome_release_time(
    outcome: &AdmittedProviderInvocationOutcome,
    released_at: UnixTimeV1,
) -> Result<(), OperationAttemptError> {
    let result = match outcome {
        AdmittedProviderInvocationOutcome::Succeeded(success) => success.result(),
        AdmittedProviderInvocationOutcome::Failed(failure) => failure.result(),
        AdmittedProviderInvocationOutcome::Unavailable(_) => return Ok(()),
    };
    if result.started_at() < &released_at {
        Err(OperationAttemptError::TimeOrder)
    } else {
        Ok(())
    }
}

#[derive(Debug)]
struct ProviderFailureDiagnosticSource;

impl std::fmt::Display for ProviderFailureDiagnosticSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("provider returned admitted safe-failure evidence")
    }
}

impl std::error::Error for ProviderFailureDiagnosticSource {}

#[derive(Debug)]
struct ProviderUncertaintyDiagnosticSource;

impl std::fmt::Display for ProviderUncertaintyDiagnosticSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("provider call has no conclusive admitted outcome")
    }
}

impl std::error::Error for ProviderUncertaintyDiagnosticSource {}

fn provider_uncertainty_diagnostic(
    handle: &OperationHandleV1,
    attempt_id: &CapabilityAttemptIdV1,
    uncertainty: &ProviderOperationUncertainty,
) -> Result<AdmittedDiagnostic, OperationAttemptError> {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1")
        .map_err(crate::diagnostic::DiagnosticAdmissionError::Registry)?;
    let (code, summary) = match uncertainty {
        ProviderOperationUncertainty::ResponseRejected(_) => (
            "provider_response_rejected",
            "The provider response did not pass admission.",
        ),
        ProviderOperationUncertainty::ResponseAssociation { .. } => (
            "provider_response_mismatch",
            "The admitted provider response does not belong to this attempt.",
        ),
        ProviderOperationUncertainty::Unavailable(_) => (
            "provider_outcome_unavailable",
            "The provider did not return conclusive signed evidence.",
        ),
        ProviderOperationUncertainty::ColdRestartAfterDispatch => (
            "provider_dispatch_recovered_without_outcome",
            "Recovery found a provider dispatch with no retained conclusive outcome.",
        ),
    };
    Ok(project_failure(
        &identity,
        &ProviderUncertaintyDiagnosticSource,
        FailureSpecification {
            code,
            stage: DiagnosticStageV1::ResidentRecovery,
            disposition: DiagnosticDispositionV1::OperationalFailure,
            summary,
            subject: Some(DiagnosticSubjectV1Wire {
                kind: "operation".to_string(),
                id: handle.operation_handle_id().as_str().to_string(),
            }),
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::ReconcileFirst,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: Some(DiagnosticRequestCorrelationV1Wire {
                kind: "capability_attempt".to_string(),
                id: attempt_id.as_str().to_string(),
            }),
            notes: vec!["The consumed provider call cannot be replayed."],
            help: vec!["Reconcile retained evidence before selecting another action."],
            facts: Vec::new(),
        },
        CauseProjection::Omitted,
    )?)
}

fn provider_failure_diagnostic(
    failure: &crate::provider_invocation::AdmittedProviderInvocationFailure,
) -> Result<AdmittedDiagnostic, OperationAttemptError> {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1")
        .map_err(crate::diagnostic::DiagnosticAdmissionError::Registry)?;
    let (code, disposition, retry) = match failure.failure().failure_class() {
        AdapterFailureClassV1::Denied | AdapterFailureClassV1::Conflict => (
            "provider_attempt_blocked",
            DiagnosticDispositionV1::Blocked,
            DiagnosticRetryV1::AfterStateChange,
        ),
        AdapterFailureClassV1::Invalid | AdapterFailureClassV1::UnsupportedSchema => (
            "provider_attempt_invalid",
            DiagnosticDispositionV1::InvalidInput,
            DiagnosticRetryV1::AfterCorrection,
        ),
        AdapterFailureClassV1::Unavailable
        | AdapterFailureClassV1::RateLimited
        | AdapterFailureClassV1::Timeout => (
            "provider_attempt_unavailable",
            DiagnosticDispositionV1::OperationalFailure,
            DiagnosticRetryV1::AfterStateChange,
        ),
        AdapterFailureClassV1::Unknown | AdapterFailureClassV1::Partial => (
            "provider_attempt_requires_reconciliation",
            DiagnosticDispositionV1::OperationalFailure,
            DiagnosticRetryV1::ReconcileFirst,
        ),
        AdapterFailureClassV1::Internal => (
            "provider_attempt_internal_failure",
            DiagnosticDispositionV1::InternalFailure,
            DiagnosticRetryV1::NotIndicated,
        ),
    };
    Ok(project_failure(
        &identity,
        &ProviderFailureDiagnosticSource,
        FailureSpecification {
            code,
            stage: DiagnosticStageV1::HostObservation,
            disposition,
            summary: "The provider could not complete this attempt safely.",
            subject: Some(DiagnosticSubjectV1Wire {
                kind: "provider_binding".to_string(),
                id: failure
                    .coordinate()
                    .provider_binding_identity()
                    .provider_binding_sha256
                    .as_str()
                    .to_string(),
            }),
            schema_coordinate: None,
            location: None,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: Some(DiagnosticRequestCorrelationV1Wire {
                kind: "capability_attempt".to_string(),
                id: failure.result().attempt_id().as_str().to_string(),
            }),
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
        CauseProjection::Omitted,
    )?)
}

fn validate_recording_source_references(
    retained: &ProviderOperationRecordingSource<'_>,
    input: &ProviderOperationInvokeInput<'_>,
) -> Result<(), OperationAttemptError> {
    retained.validate_original_session()?;
    let (handle_ref, handle) = retained.handle();
    let planned = [
        &input.result_ref,
        &input.diagnostic_ref,
        &input.reconciliation.record_ref,
        &input.terminal_ref,
        &input.terminal_checkpoint.reference,
    ];
    let existing = [
        handle_ref,
        handle.operation_lease_ref(),
        retained.request_ref(),
        retained.selection_ref(),
        retained.preparation_observation_ref(),
        retained.observation().0,
        retained.checkpoint().0,
    ];
    if planned.iter().enumerate().any(|(index, reference)| {
        planned[..index].contains(reference) || existing.contains(reference)
    }) || input.terminal_checkpoint.id == *retained.checkpoint().1.control_checkpoint_id()
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

fn validate_references(
    retained: &RetainedProviderOperationDispatch<'_>,
    input: &ProviderOperationInvokeInput<'_>,
) -> Result<(), OperationAttemptError> {
    retained.context().validate_original_session()?;
    let context = retained.context();
    let planned = [
        &input.result_ref,
        &input.diagnostic_ref,
        &input.reconciliation.record_ref,
        &input.terminal_ref,
        &input.terminal_checkpoint.reference,
    ];
    let existing = [
        context.operation.handle().0,
        context.operation.handle().1.operation_lease_ref(),
        context.operation.request_coordinate().record_ref(),
        &context.selection_ref,
        &context.observation_ref,
        retained.observation().0,
        retained.checkpoint().0,
    ];
    if planned.iter().enumerate().any(|(index, reference)| {
        planned[..index].contains(reference) || existing.contains(reference)
    }) || input.terminal_checkpoint.id == *retained.checkpoint().1.control_checkpoint_id()
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

/// Reject colliding recovered-call outputs before consulting any external owner.
pub(in crate::resident) fn validate_recovered_provider_call_references(
    release: &RecoveredProviderDispatchRelease,
    input: &ProviderOperationInvokeInput<'_>,
) -> Result<(), OperationAttemptError> {
    let operation = release.operation();
    let planned = [
        &input.result_ref,
        &input.diagnostic_ref,
        &input.reconciliation.record_ref,
        &input.terminal_ref,
        &input.terminal_checkpoint.reference,
    ];
    let existing = [
        operation.handle().0,
        operation.handle().1.operation_lease_ref(),
        operation.request_coordinate().record_ref(),
        release.selection_ref(),
        release.preparation().0,
        release.observation().0,
        release.checkpoint().0,
    ];
    if planned.iter().enumerate().any(|(index, reference)| {
        planned[..index].contains(reference) || existing.contains(reference)
    }) || input.terminal_checkpoint.id == *release.checkpoint().1.control_checkpoint_id()
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

fn validate_recovered_references(
    recovered: &RecoveredProviderOperationDispatch<'_>,
    input: &ProviderOperationInvokeInput<'_>,
) -> Result<(), OperationAttemptError> {
    let operation = recovered.operation();
    let planned = [
        &input.result_ref,
        &input.diagnostic_ref,
        &input.reconciliation.record_ref,
        &input.terminal_ref,
        &input.terminal_checkpoint.reference,
    ];
    let existing = [
        operation.handle().0,
        operation.handle().1.operation_lease_ref(),
        operation.request_coordinate().record_ref(),
        recovered.selection_ref(),
        recovered.preparation_observation_ref(),
        recovered.observation().0,
        recovered.recording_predecessor().0,
        recovered.checkpoint().0,
    ];
    let cancellation_request_ref = match recovered.recording_predecessor().1.state() {
        OperationObservationStateV1::CancellationRequested { request } => Some(&request.record_ref),
        _ => None,
    };
    if planned.iter().enumerate().any(|(index, reference)| {
        planned[..index].contains(reference)
            || existing.contains(reference)
            || cancellation_request_ref == Some(*reference)
    }) || input.terminal_checkpoint.id == *recovered.checkpoint().1.control_checkpoint_id()
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}
