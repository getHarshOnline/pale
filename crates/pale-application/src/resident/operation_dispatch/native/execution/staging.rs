//! Durably stage one already-released native outcome before terminal projection.

use super::*;
use crate::actions::matches_native_failure_diagnostic;
use crate::diagnostic::AdmittedDiagnostic;
use crate::resident::{
    record_projection, CompletedOperationObservationCustody, OperationDispatchCustodyError,
};
use pale_core::canon;
use pale_ports::{
    ControlRecord, ControlRecordBytes, ControlRecordCoordinate, ControlRecordDigest,
    OperationCustodyIndexKey, OperationCustodyIndexQuery, OperationCustodyRecord,
    PRIVATE_OPERATION_NATIVE_OUTCOME_V1_SCHEMA,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Private outcome bytes bound to one exact native dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct StagedNativeOutcome {
    handle_ref: ResidentRecordRefV1,
    handle_sha256: OperationHandleDigest,
    attempt_id: CapabilityAttemptIdV1,
    dispatch_ref: ResidentRecordRefV1,
    dispatch_sha256: OperationObservationDigest,
    started_at: UnixTimeV1,
    completed_at: UnixTimeV1,
    outcome: StagedNativeOutcomeBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
enum StagedNativeOutcomeBody {
    Succeeded { execution: ActionExecution },
    Failed { diagnostic: DiagnosticV1Wire },
}

impl StagedNativeOutcome {
    fn record(
        &self,
        reference: ResidentRecordRefV1,
        maximum_bytes: usize,
    ) -> Result<ControlRecord, OperationAttemptError> {
        let bytes = canon::to_canonical_string(self)
            .map_err(|_| OperationAttemptError::Encoding)?
            .into_bytes();
        if bytes.len() > maximum_bytes {
            return Err(OperationAttemptError::BoundExceeded);
        }
        Ok(ControlRecord::new(
            ControlRecordCoordinate::new(
                reference,
                ControlRecordDigest::parse(pale_core::sha256_hex(&bytes))
                    .map_err(|_| OperationAttemptError::Encoding)?,
            ),
            SchemaId::parse(PRIVATE_OPERATION_NATIVE_OUTCOME_V1_SCHEMA)
                .map_err(|_| OperationAttemptError::Encoding)?,
            Self::schema_digest()?,
            ControlRecordBytes::new(bytes).map_err(|_| OperationAttemptError::Encoding)?,
        ))
    }

    fn decode(
        record: &ControlRecord,
        dispatch: (&ResidentRecordRefV1, &OperationObservationV1),
        handle: (&ResidentRecordRefV1, &OperationHandleV1),
        expected_attempt_id: &CapabilityAttemptIdV1,
        maximum_bytes: usize,
    ) -> Result<Self, OperationAttemptError> {
        let bytes = record.canonical_record_jcs().as_slice();
        if bytes.len() > maximum_bytes {
            return Err(OperationAttemptError::BoundExceeded);
        }
        if record.schema_id().as_str() != PRIVATE_OPERATION_NATIVE_OUTCOME_V1_SCHEMA
            || record.schema_sha256() != &Self::schema_digest()?
            || pale_core::sha256_hex(bytes) != record.coordinate().record_sha256().as_str()
        {
            return Err(OperationAttemptError::Encoding);
        }
        let value = canon::parse_strict(bytes).map_err(|_| OperationAttemptError::Encoding)?;
        let staged: Self =
            serde_json::from_value(value).map_err(|_| OperationAttemptError::Encoding)?;
        if canon::to_canonical_string(&staged)
            .map_err(|_| OperationAttemptError::Encoding)?
            .as_bytes()
            != bytes
            || &staged.handle_ref != handle.0
            || &staged.handle_sha256 != handle.1.canonical_operation_handle_sha256()
            || &staged.attempt_id != expected_attempt_id
            || &staged.dispatch_ref != dispatch.0
            || &staged.dispatch_sha256 != dispatch.1.canonical_operation_observation_sha256()
            || staged.started_at < *dispatch.1.observed_at()
            || staged.completed_at < staged.started_at
            || staged.completed_at > *handle.1.retain_until()
        {
            return Err(OperationAttemptError::Association);
        }
        Ok(staged)
    }

    fn schema_digest() -> Result<SchemaDigest, OperationAttemptError> {
        static DIGEST: OnceLock<Result<SchemaDigest, ()>> = OnceLock::new();
        DIGEST
            .get_or_init(|| {
                let schema = schemars::schema_for!(StagedNativeOutcome);
                let canonical = canon::to_canonical_string(&schema).map_err(|_| ())?;
                SchemaDigest::parse(pale_core::sha256_hex(canonical.as_bytes())).map_err(|_| ())
            })
            .as_ref()
            .cloned()
            .map_err(|_| OperationAttemptError::Encoding)
    }
}

/// Fresh private record and observation inputs for one released native outcome.
pub struct NativeOperationOutcomeStageInput<'a> {
    /// Fresh immutable reference for the private native-outcome record.
    pub outcome_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for the signed staging observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Actual handler completion time.
    pub completed_at: UnixTimeV1,
    /// Recording time within handle retention.
    pub observed_at: UnixTimeV1,
    /// Existing observation-signing authority.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Maximum canonical private outcome bytes.
    pub maximum_outcome_bytes: usize,
    /// Maximum canonical diagnostic bytes when staging a handler failure.
    pub maximum_diagnostic_bytes: usize,
}

struct RetainedNativeOperationOutcomeStageInput {
    outcome_ref: ResidentRecordRefV1,
    observation_ref: ResidentRecordRefV1,
    completed_at: UnixTimeV1,
    observed_at: UnixTimeV1,
    observation_schema: SchemaDigest,
    signing_authority: SigningAuthorityBindingV1,
    maximum_outcome_bytes: usize,
    maximum_diagnostic_bytes: usize,
}

impl RetainedNativeOperationOutcomeStageInput {
    fn new(input: NativeOperationOutcomeStageInput<'_>) -> Self {
        Self {
            outcome_ref: input.outcome_ref,
            observation_ref: input.observation_ref,
            completed_at: input.completed_at,
            observed_at: input.observed_at,
            observation_schema: input.signing.expected_schema_sha256.clone(),
            signing_authority: input.signing.signing_authority.clone(),
            maximum_outcome_bytes: input.maximum_outcome_bytes,
            maximum_diagnostic_bytes: input.maximum_diagnostic_bytes,
        }
    }

    fn matches(&self, signing: SignedResidentAdmissionContextV1<'_>) -> bool {
        signing.expected_schema_sha256 == &self.observation_schema
            && signing.signing_authority == &self.signing_authority
    }
}

/// One handler outcome that has not prepared any durable staging candidate.
///
/// This is the only native-outcome phase that can allocate the private outcome
/// record and its staging observation. Preparing consumes this value, so a
/// caller cannot retain a second route to stage the same handler execution
/// under different identities.
#[must_use = "prepare exactly one staging candidate or retain the unstaged outcome"]
pub struct UnstagedNativeOperationOutcome {
    outcome: NativeOperationOutcome,
}

impl std::fmt::Debug for UnstagedNativeOperationOutcome {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UnstagedNativeOperationOutcome")
            .field(
                "disposition",
                &match self.outcome {
                    NativeOperationOutcome::Succeeded(_) => "Succeeded",
                    NativeOperationOutcome::Failed(_) => "Failed",
                },
            )
            .finish_non_exhaustive()
    }
}

impl UnstagedNativeOperationOutcome {
    pub(super) const fn new(outcome: NativeOperationOutcome) -> Self {
        Self { outcome }
    }

    /// Borrow the actual handler disposition without gaining another staging route.
    pub const fn outcome(&self) -> &NativeOperationOutcome {
        &self.outcome
    }

    /// Prepare the sole durable staging candidate for this handler outcome.
    ///
    /// # Errors
    ///
    /// Returns the sole exact preparation with the original coordinates, times,
    /// signer binding, and byte bounds. No staging candidate exists on this
    /// branch, and the handler outcome cannot be recovered as a fresh input.
    pub fn prepare_stage<Rotation, Sign>(
        self,
        input: NativeOperationOutcomeStageInput<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedNativeOperationOutcomeStage, NativeOperationOutcomeStagePreparationFailure>
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
        let signing = input.signing;
        NativeOperationOutcomeStagePreparation {
            outcome: self,
            input: RetainedNativeOperationOutcomeStageInput::new(input),
        }
        .continue_with(signing, admit_rotation, sign)
    }
}

/// Exact stage coordinates and handler outcome retained before stage signing.
#[must_use = "continue this exact native-outcome stage preparation"]
pub(in crate::resident) struct NativeOperationOutcomeStagePreparation {
    outcome: UnstagedNativeOperationOutcome,
    input: RetainedNativeOperationOutcomeStageInput,
}

impl std::fmt::Debug for NativeOperationOutcomeStagePreparation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeOperationOutcomeStagePreparation")
            .finish_non_exhaustive()
    }
}

impl NativeOperationOutcomeStagePreparation {
    /// Borrow the issued operation without gaining another handler or stage route.
    pub(in crate::resident) fn operation(&self) -> &NativeOperationRecordingAuthority {
        &outcome_frame(self.outcome.outcome()).operation
    }

    /// Borrow the exact observation signer retained by this preparation.
    pub(in crate::resident) fn signing_authority(&self) -> &SigningAuthorityBindingV1 {
        &self.input.signing_authority
    }

    /// Retry the same stage preimage with the same admitted observation signer.
    ///
    /// # Errors
    /// Returns this exact preparation when the signer selection differs or the
    /// original deterministic stage preparation still cannot complete.
    pub(in crate::resident) fn continue_with<Rotation, Sign>(
        self,
        signing: SignedResidentAdmissionContextV1<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedNativeOperationOutcomeStage, NativeOperationOutcomeStagePreparationFailure>
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
        if !self.input.matches(signing) {
            return Err(NativeOperationOutcomeStagePreparationFailure {
                preparation: Box::new(self),
                reason: OperationAttemptError::Association,
            });
        }
        let Self {
            outcome: UnstagedNativeOperationOutcome { mut outcome },
            input,
        } = self;
        let prepared = (|| {
            let (frame, body) = match &mut outcome {
                NativeOperationOutcome::Succeeded(executed) => (
                    &mut executed.frame,
                    StagedNativeOutcomeBody::Succeeded {
                        execution: executed.execution.clone(),
                    },
                ),
                NativeOperationOutcome::Failed(failed) => {
                    let diagnostic = failed.stage_diagnostic(input.maximum_diagnostic_bytes)?;
                    (
                        &mut failed.frame,
                        StagedNativeOutcomeBody::Failed {
                            diagnostic: diagnostic.wire().clone(),
                        },
                    )
                }
            };
            frame.validate_terminal(
                &input.outcome_ref,
                &input.observation_ref,
                input.completed_at,
                input.observed_at,
            )?;
            if frame.recording_head.is_some() {
                return Err(OperationAttemptError::Association);
            }
            let OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched { attempt_id, .. },
            } = frame.dispatch.state()
            else {
                return Err(OperationAttemptError::Association);
            };
            if attempt_id != frame.selection.attempt_id() {
                return Err(OperationAttemptError::Association);
            }
            let (handle_ref, handle) = frame.operation.handle();
            let staged = StagedNativeOutcome {
                handle_ref: handle_ref.clone(),
                handle_sha256: handle.canonical_operation_handle_sha256().clone(),
                attempt_id: attempt_id.clone(),
                dispatch_ref: frame.dispatch_ref.clone(),
                dispatch_sha256: frame
                    .dispatch
                    .canonical_operation_observation_sha256()
                    .clone(),
                started_at: frame.started_at,
                completed_at: input.completed_at,
                outcome: body,
            };
            let outcome_record =
                staged.record(input.outcome_ref.clone(), input.maximum_outcome_bytes)?;
            let observation = signing::issue_observation(
                &OperationObservationAdmissionContextV1 {
                    signed: signing,
                    handle_ref,
                    handle,
                    predecessor: Some((&frame.dispatch_ref, &frame.dispatch)),
                },
                frame.dispatch.state().clone(),
                input.observed_at,
                admit_rotation,
                sign,
            )?;
            Ok((outcome_record, input.observation_ref.clone(), observation))
        })();
        match prepared {
            Ok((outcome_record, observation_ref, observation)) => {
                Ok(PreparedNativeOperationOutcomeStage {
                    outcome,
                    outcome_record,
                    observation_ref,
                    observation,
                })
            }
            Err(reason) => Err(NativeOperationOutcomeStagePreparationFailure {
                preparation: Box::new(Self {
                    outcome: UnstagedNativeOperationOutcome { outcome },
                    input,
                }),
                reason,
            }),
        }
    }
}

/// Staging preparation failed before any physical candidate existed.
pub struct NativeOperationOutcomeStagePreparationFailure {
    preparation: Box<NativeOperationOutcomeStagePreparation>,
    reason: OperationAttemptError,
}

impl std::fmt::Debug for NativeOperationOutcomeStagePreparationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeOperationOutcomeStagePreparationFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl NativeOperationOutcomeStagePreparationFailure {
    /// Borrow the exact pre-candidate refusal.
    #[must_use]
    pub const fn reason(&self) -> &OperationAttemptError {
        &self.reason
    }

    pub(in crate::resident) fn into_parts(
        self,
    ) -> (
        NativeOperationOutcomeStagePreparation,
        OperationAttemptError,
    ) {
        (*self.preparation, self.reason)
    }
}

/// One signed staging observation and private outcome awaiting exact custody.
#[must_use = "commit the native outcome and observation together"]
pub struct PreparedNativeOperationOutcomeStage {
    outcome: NativeOperationOutcome,
    outcome_record: ControlRecord,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
}

impl PreparedNativeOperationOutcomeStage {
    /// Borrow the retained handler disposition without gaining a staging method.
    pub const fn outcome(&self) -> &NativeOperationOutcome {
        &self.outcome
    }

    pub(in crate::resident::operation_dispatch::native) fn operation(
        &self,
    ) -> &NativeOperationRecordingAuthority {
        &outcome_frame(&self.outcome).operation
    }

    /// Borrow the signed nonterminal observation.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Prepare the private outcome and observation for one exact append.
    ///
    /// # Errors
    ///
    /// Returns an error when the retained dispatch or observation cannot be
    /// projected into the selected authority store.
    pub fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        let frame = outcome_frame(&self.outcome);
        let (handle_ref, handle) = frame.operation.handle();
        let dispatch = record_projection::coordinate(
            &frame.dispatch_ref,
            frame
                .dispatch
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        Ok(prepare_operation_observation_append(
            OperationObservationTransition::new(
                handle_ref,
                handle,
                &frame.dispatch_ref,
                &frame.dispatch,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?
        .with_native_outcome(
            OperationCustodyRecord::NativeOutcome(self.outcome_record.clone()),
            &dispatch,
        ))
    }

    /// Join exact completed custody back to the retained in-memory outcome.
    ///
    /// # Errors
    ///
    /// Returns an error when the committed records, index mutation, store or
    /// observation differ from the prepared stage.
    pub fn finish(
        mut self,
        completed: &CompletedOperationObservationCustody,
    ) -> Result<StagedNativeOperationOutcome, NativeOperationOutcomeStageFinalizationFailure> {
        let admitted = (|| {
            let candidate = completed.candidate();
            let frame = outcome_frame(&self.outcome);
            let dispatch = record_projection::coordinate(
                &frame.dispatch_ref,
                frame
                    .dispatch
                    .canonical_operation_observation_sha256()
                    .as_str(),
            )
            .map_err(OperationObservationCustodyProjectionError::from)?;
            let expected_index = pale_ports::OperationCustodyIndexMutation::NativeOutcome {
                key: OperationCustodyIndexKey::for_native_outcome(&dispatch),
                expected: pale_ports::OperationCustodyIndexPredecessor::Absent,
                successor: self.outcome_record.coordinate().clone(),
            };
            let observation_record = record_projection::record(
                record_projection::coordinate(
                    &self.observation_ref,
                    self.observation
                        .canonical_operation_observation_sha256()
                        .as_str(),
                )
                .map_err(OperationObservationCustodyProjectionError::from)?,
                &self.observation,
            )
            .map_err(OperationObservationCustodyProjectionError::from)?;
            if candidate.record(pale_ports::OperationCustodyRecordKind::NativeOutcome)
                != Some(&self.outcome_record)
                || !frame
                    .operation
                    .checkpoint_selection()
                    .matches_store(candidate.store())
                || !candidate.indexes().contains(&expected_index)
                || candidate.record(pale_ports::OperationCustodyRecordKind::OperationObservation)
                    != Some(&observation_record)
            {
                return Err(OperationAttemptError::Association);
            }
            outcome_frame_mut(&mut self.outcome).recording_head = Some(Box::new((
                self.observation_ref.clone(),
                self.observation.clone(),
            )));
            Ok(())
        })();
        match admitted {
            Ok(()) => Ok(StagedNativeOperationOutcome {
                outcome: self.outcome,
            }),
            Err(reason) => Err(NativeOperationOutcomeStageFinalizationFailure { reason }),
        }
    }
}

/// Database-and-anchor custody completed, but semantic stage admission failed.
///
/// This is terminal in memory. It returns no stage or handler route. The
/// durable head can continue only through the cold staged-outcome recovery path.
pub struct NativeOperationOutcomeStageFinalizationFailure {
    reason: OperationAttemptError,
}

impl std::fmt::Debug for NativeOperationOutcomeStageFinalizationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeOperationOutcomeStageFinalizationFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl NativeOperationOutcomeStageFinalizationFailure {
    /// Borrow the exact admission mismatch without recovering an unstaged outcome.
    #[must_use]
    pub const fn reason(&self) -> &OperationAttemptError {
        &self.reason
    }

    pub(in crate::resident::operation_dispatch::native) fn into_reason(
        self,
    ) -> OperationAttemptError {
        self.reason
    }
}

/// One handler outcome joined to an admitted, database-and-anchor staged head.
///
/// This phase can prepare terminal evidence but has no staging method.
#[must_use = "record terminal evidence from this admitted staged head"]
pub struct StagedNativeOperationOutcome {
    outcome: NativeOperationOutcome,
}

impl std::fmt::Debug for StagedNativeOperationOutcome {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StagedNativeOperationOutcome")
            .field(
                "disposition",
                &match self.outcome {
                    NativeOperationOutcome::Succeeded(_) => "Succeeded",
                    NativeOperationOutcome::Failed(_) => "Failed",
                },
            )
            .finish_non_exhaustive()
    }
}

impl StagedNativeOperationOutcome {
    /// Borrow the retained handler disposition without gaining another staging route.
    pub const fn outcome(&self) -> &NativeOperationOutcome {
        &self.outcome
    }

    pub(in crate::resident::operation_dispatch::native) fn operation(
        &self,
    ) -> &NativeOperationRecordingAuthority {
        &outcome_frame(&self.outcome).operation
    }
}

/// Protected history and bounds for cold native-outcome recovery.
pub struct NativeOperationOutcomeRecoveryInput<'a> {
    /// Original request/selection joined to the protected dispatched history.
    pub history: OperationAttemptHistoryInput<'a>,
    /// Maximum private native-outcome record bytes.
    pub maximum_outcome_bytes: usize,
}

/// Exact bounded observation slice needed to recover one staged native outcome.
///
/// The ordinary shape selects the staged head and retains its exact dispatch and
/// preparation predecessors. The cancellation shape selects that staged member
/// below its direct cancellation successor. Both shapes come from the same
/// authenticated walk; this owner performs no additional history search.
struct RecoveredNativeOutcomeHistory<'a> {
    preparation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    dispatch: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    stage: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    head: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
}

fn recover_native_outcome_history<'a>(
    recovered: &'a RecoveredOperationObservation,
    handle: (&ResidentRecordRefV1, &OperationHandleV1),
) -> Result<RecoveredNativeOutcomeHistory<'a>, OperationAttemptError> {
    let selected = recovered.selected();
    let head = recovered.selected_head();
    let predecessor = recovered
        .selected_predecessor()
        .ok_or(OperationAttemptError::Association)?;
    let (preparation, dispatch, stage) = if selected == head {
        let preparation = recovered
            .selected_predecessor_predecessor()
            .ok_or(OperationAttemptError::Association)?;
        (preparation, predecessor, selected)
    } else {
        if !matches!(
            head.1.state(),
            OperationObservationStateV1::CancellationRequested { .. }
        ) {
            return Err(OperationAttemptError::Association);
        }
        let preparation = recovered
            .selected_predecessor_predecessor()
            .ok_or(OperationAttemptError::Association)?;
        (preparation, predecessor, selected)
    };
    let (
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::Dispatched {
                    attempt_id,
                    evidence,
                },
        },
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::DispatchPrepared {
                    attempt_id: prepared_attempt_id,
                    evidence: prepared_evidence,
                },
        },
    ) = (dispatch.1.state(), preparation.1.state())
    else {
        return Err(OperationAttemptError::Association);
    };
    if attempt_id != prepared_attempt_id
        || evidence != prepared_evidence
        || stage.1.state() != dispatch.1.state()
        || stage.0 == dispatch.0
        || head.0 == dispatch.0
        || stage
            .1
            .verify_admitted_successor_of(handle.0, handle.1, dispatch.0, dispatch.1)
            .is_err()
    {
        return Err(OperationAttemptError::Association);
    }
    if stage != head
        && (head.0 == stage.0
            || !matches!(
                head.1.state(),
                OperationObservationStateV1::CancellationRequested { .. }
            )
            || head
                .1
                .verify_admitted_successor_of(handle.0, handle.1, stage.0, stage.1)
                .is_err())
    {
        return Err(OperationAttemptError::Association);
    }
    let stage_checkpoint = if stage == head {
        recovered.selected_head_checkpoint_record().1
    } else {
        recovered.selected_checkpoint()
    };
    let head_checkpoint = recovered.selected_head_checkpoint_record().1;
    if stage.1.observed_at() > stage_checkpoint.issued_at()
        || head.1.observed_at() > head_checkpoint.issued_at()
    {
        return Err(OperationAttemptError::TimeOrder);
    }
    Ok(RecoveredNativeOutcomeHistory {
        preparation,
        dispatch,
        stage,
        head,
    })
}

/// Recovered native outcome, incapable of running the handler again.
pub type RecoveredNativeOperationOutcome = StagedNativeOperationOutcome;

/// Private recovery phase that retains the evidence needed for terminal custody.
pub(in crate::resident) struct RecoveredNativeOperationOutcomePhase {
    staged: StagedNativeOperationOutcome,
    completed_at: UnixTimeV1,
    outcome_coordinate: ControlRecordCoordinate,
    result_authority: SigningAuthorityBindingV1,
}

impl RecoveredNativeOperationOutcomePhase {
    pub(in crate::resident) const fn completed_at(&self) -> UnixTimeV1 {
        self.completed_at
    }

    pub(in crate::resident) const fn outcome_coordinate(&self) -> &ControlRecordCoordinate {
        &self.outcome_coordinate
    }

    pub(in crate::resident) const fn result_authority(&self) -> &SigningAuthorityBindingV1 {
        &self.result_authority
    }

    pub(in crate::resident) fn into_staged(self) -> StagedNativeOperationOutcome {
        self.staged
    }
}

/// Recover an exact staged outcome without invoking its native handler.
///
/// # Errors
///
/// Returns an error when protected history, the native-outcome index, private
/// record bytes, action selection, recorded execution or diagnostic evidence
/// does not reproduce the dispatched attempt.
pub fn recover_native_operation_outcome<'operation>(
    input: NativeOperationOutcomeRecoveryInput<'operation>,
    recovered: &RecoveredOperationObservation,
    registry: &NativeActionRegistry,
    contract_set: &'operation ContractSetV1,
    custody: &mut dyn OperationCustodyPort,
) -> Result<RecoveredNativeOperationOutcome, OperationAttemptError> {
    recover_native_operation_outcome_phase(input, recovered, registry, contract_set, custody)
        .map(RecoveredNativeOperationOutcomePhase::into_staged)
}

/// Recover the private outcome together with its terminal-recording evidence.
pub(in crate::resident) fn recover_native_operation_outcome_phase<'operation>(
    input: NativeOperationOutcomeRecoveryInput<'operation>,
    recovered: &RecoveredOperationObservation,
    registry: &NativeActionRegistry,
    contract_set: &'operation ContractSetV1,
    custody: &mut dyn OperationCustodyPort,
) -> Result<RecoveredNativeOperationOutcomePhase, OperationAttemptError> {
    let (handle_ref, handle) = input.history.operation.handle();
    if !input
        .history
        .operation
        .checkpoint_selection()
        .matches_store(recovered.store())
    {
        return Err(OperationAttemptError::Association);
    }
    let history = recover_native_outcome_history(recovered, (handle_ref, handle))?;
    let (dispatch_ref, dispatch) = history.dispatch;
    let (head_ref, head) = history.head;
    if dispatch != input.history.dispatched {
        return Err(OperationAttemptError::Association);
    }
    let attempt_id = match dispatch.state() {
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Dispatched { attempt_id, .. },
        } => attempt_id,
        _ => return Err(OperationAttemptError::Association),
    };
    let selection_ref = match dispatch.state() {
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Dispatched { evidence, .. },
        } => evidence.record_ref.clone(),
        _ => return Err(OperationAttemptError::Association),
    };
    let predecessor = history.preparation;
    let preparation_predecessor_ref = predecessor.1.predecessor_ref().clone();
    let expected_selection_ref = input.history.selection.coordinate().record_ref().clone();
    let attempt = input
        .history
        .recover_native_attempt(registry, contract_set)?;
    if attempt.selection().attempt_id() != attempt_id {
        return Err(OperationAttemptError::Association);
    }
    let (expected_action_id, expected_request_digest) = attempt
        .selection()
        .native_action_identity()
        .map(|(action_id, request_digest)| (action_id.to_owned(), request_digest.clone()))
        .ok_or(OperationAttemptError::Association)?;
    let dispatch_coordinate = record_projection::coordinate(
        dispatch_ref,
        dispatch.canonical_operation_observation_sha256().as_str(),
    )
    .map_err(OperationObservationCustodyProjectionError::from)?;
    let query = OperationCustodyIndexQuery::NativeOutcome(
        OperationCustodyIndexKey::for_native_outcome(&dispatch_coordinate),
    );
    let record = crate::resident::operation_custody::read_index(
        custody,
        recovered.store(),
        query,
        input.maximum_outcome_bytes,
    )
    .map_err(OperationDispatchCustodyError::from)?
    .ok_or(OperationAttemptError::Association)?;
    let staged = StagedNativeOutcome::decode(
        &record,
        (dispatch_ref, dispatch),
        (handle_ref, handle),
        attempt_id,
        input.maximum_outcome_bytes,
    )?;
    if staged.completed_at > *history.stage.1.observed_at() {
        return Err(OperationAttemptError::TimeOrder);
    }
    let operation = attempt.operation();
    let result_authority = attempt.selection().result_authority().clone();
    let (selection, call, contract_set) = attempt.into_parts();
    if selection_ref != expected_selection_ref {
        return Err(OperationAttemptError::Association);
    }
    let frame = NativeOperationExecutionFrame {
        operation: NativeOperationRecordingAuthority::capture(operation),
        selection_ref,
        preparation_observation_ref: predecessor.0.clone(),
        preparation_predecessor_ref,
        selection,
        contract_set: contract_set.clone(),
        dispatch_ref: dispatch_ref.clone(),
        dispatch: dispatch.clone(),
        started_at: staged.started_at,
        recording_head: Some(Box::new((head_ref.clone(), head.clone()))),
    };
    let completed_at = staged.completed_at;
    let outcome_coordinate = record.coordinate().clone();
    match staged.outcome {
        StagedNativeOutcomeBody::Succeeded { execution } => {
            call.validate_recorded_execution(&execution)?;
            Ok(RecoveredNativeOperationOutcomePhase {
                staged: StagedNativeOperationOutcome {
                    outcome: NativeOperationOutcome::Succeeded(Box::new(ExecutedNativeOperation {
                        frame,
                        execution,
                    })),
                },
                completed_at,
                outcome_coordinate,
                result_authority,
            })
        }
        StagedNativeOutcomeBody::Failed { diagnostic } => {
            let bytes = canon::to_canonical_string(&diagnostic)
                .map_err(|_| OperationAttemptError::Encoding)?;
            let diagnostic = AdmittedDiagnostic::from_bytes(bytes.as_bytes())?;
            if !matches_native_failure_diagnostic(
                &diagnostic,
                &expected_action_id,
                expected_request_digest.as_str(),
            ) {
                return Err(OperationAttemptError::Association);
            }
            Ok(RecoveredNativeOperationOutcomePhase {
                staged: StagedNativeOperationOutcome {
                    outcome: NativeOperationOutcome::Failed(Box::new(
                        FailedNativeOperation::from_staged(frame, diagnostic),
                    )),
                },
                completed_at,
                outcome_coordinate,
                result_authority,
            })
        }
    }
}

fn outcome_frame(outcome: &NativeOperationOutcome) -> &NativeOperationExecutionFrame {
    match outcome {
        NativeOperationOutcome::Succeeded(executed) => &executed.frame,
        NativeOperationOutcome::Failed(failed) => &failed.frame,
    }
}

fn outcome_frame_mut(outcome: &mut NativeOperationOutcome) -> &mut NativeOperationExecutionFrame {
    match outcome {
        NativeOperationOutcome::Succeeded(executed) => &mut executed.frame,
        NativeOperationOutcome::Failed(failed) => &mut failed.frame,
    }
}
