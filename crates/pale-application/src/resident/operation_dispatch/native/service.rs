//! Run one already-issued native operation through its existing custody owners.

use super::super::custody::{
    append_checkpoint, append_checkpoint_for_selection, commit_prepared_checkpoint,
    observation_checkpoint_preimage, sample_time,
};
use super::*;
use crate::diagnostic::AdmittedDiagnostic;
use crate::resident::operation_dispatch::checkpoint_authority::RetainedCheckpointAuthority;
use crate::resident::{
    DatabaseCommittedOperationObservationAnchorPending, OperationObservationCustodyAnchorFailure,
    OperationObservationCustodyCommitFailure,
};
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchorPort, OperationCustodyPort, WallClockSourcePort,
};

/// Resolved inputs for one native attempt, not a caller or transport admission.
pub struct NativeOperationInvokeInput<'operation, 'call, 'registry> {
    /// Original issued operation, admitted session, request and selected handler.
    pub attempt: NativeOperationAttemptInput<'operation, 'registry>,
    /// Admitted checkpoint containing the selected resolving observation.
    pub checkpoint: (&'call ResidentRecordRefV1, &'call ControlStateCheckpointV1),
    /// Independently selected checkpoint schema.
    pub checkpoint_schema: &'call SchemaDigest,
    /// Resolved checkpoint signer. The signing callback must establish its
    /// authority at each supplied preimage's issue time, not at predecessor time.
    pub checkpoint_authority: &'call ControlCheckpointSigningAuthorityAdmissionContextV1<'call>,
    /// Resolved observation signer, independently valid at each recording time.
    pub signing: SignedResidentAdmissionContextV1<'call>,
    /// Checkpoint for selection and preparation custody.
    pub preparation_checkpoint: OperationCheckpointInput,
    /// Checkpoint for dispatch custody.
    pub dispatch_checkpoint: OperationCheckpointInput,
    /// Checkpoint for post-handler outcome custody.
    pub outcome_checkpoint: OperationCheckpointInput,
    /// Checkpoint for terminal custody.
    pub terminal_checkpoint: OperationCheckpointInput,
    /// Fresh dispatch observation reference.
    pub dispatch_ref: ResidentRecordRefV1,
    /// Fresh private native-outcome reference.
    pub outcome_ref: ResidentRecordRefV1,
    /// Fresh nonterminal outcome-staging observation reference.
    pub outcome_observation_ref: ResidentRecordRefV1,
    /// Fresh successful result reference.
    pub result_ref: ResidentRecordRefV1,
    /// Fresh failed-handler diagnostic reference.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Fresh terminal observation reference.
    pub terminal_ref: ResidentRecordRefV1,
    /// Selected record-time source; this does not establish clock trust.
    pub clock: &'call dyn WallClockSourcePort,
    /// Existing owner of the observation's store coordinate.
    pub resolve_store: &'call dyn OperationObservationAuthorityStoreResolver,
    /// Bound on each dispatch-custody read.
    pub maximum_record_bytes: usize,
    /// Bound on a failed handler's diagnostic.
    pub maximum_diagnostic_bytes: usize,
    /// Bound on the private success-or-failure outcome record.
    pub maximum_outcome_bytes: usize,
    /// Existing native accumulator preparation bounds.
    pub accumulator_limits: AccumulatorPreparationLimits,
}

/// Exact custody inputs for one handler outcome that has already been produced.
///
/// The predecessor checkpoint must admit the dispatch that produced the
/// outcome. This phase cannot run a handler or prepare another dispatch.
pub(in crate::resident) struct NativeOperationOutcomeRecordingInput<'a> {
    /// Checkpoint that admits the exact dispatched observation.
    pub(in crate::resident) checkpoint: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    /// Fresh private native-outcome reference.
    pub(in crate::resident) outcome_ref: ResidentRecordRefV1,
    /// Fresh nonterminal outcome-staging observation reference.
    pub(in crate::resident) outcome_observation_ref: ResidentRecordRefV1,
    /// Fresh successful result reference.
    pub(in crate::resident) result_ref: ResidentRecordRefV1,
    /// Fresh failed-handler diagnostic reference.
    pub(in crate::resident) diagnostic_ref: ResidentRecordRefV1,
    /// Fresh terminal observation reference.
    pub(in crate::resident) terminal_ref: ResidentRecordRefV1,
    /// Checkpoint identity for outcome staging.
    pub(in crate::resident) outcome_checkpoint: OperationCheckpointInput,
    /// Checkpoint identity for terminal custody.
    pub(in crate::resident) terminal_checkpoint: OperationCheckpointInput,
    /// Selected record-time source; this does not establish clock trust.
    pub(in crate::resident) clock: &'a dyn WallClockSourcePort,
    /// Existing owner of the observation's store coordinate.
    pub(in crate::resident) resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    /// Independently selected checkpoint schema.
    pub(in crate::resident) checkpoint_schema: &'a SchemaDigest,
    /// Resolved checkpoint signer.
    pub(in crate::resident) checkpoint_authority:
        &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Resolved observation signer.
    pub(in crate::resident) signing: SignedResidentAdmissionContextV1<'a>,
    /// Bound on a failed handler's diagnostic.
    pub(in crate::resident) maximum_diagnostic_bytes: usize,
    /// Bound on the private success-or-failure outcome record.
    pub(in crate::resident) maximum_outcome_bytes: usize,
    /// Existing native accumulator preparation bounds.
    pub(in crate::resident) accumulator_limits: AccumulatorPreparationLimits,
}

struct NativeOperationStageCheckpointPlan {
    predecessor: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    checkpoint: OperationCheckpointInput,
    issued_at: UnixTimeV1,
    schema: SchemaDigest,
    authority: RetainedCheckpointAuthority,
    limits: AccumulatorPreparationLimits,
}

struct NativeOperationStageCustodyBasis {
    stage: Box<PreparedNativeOperationOutcomeStage>,
    plan: NativeOperationStageCheckpointPlan,
}

impl std::fmt::Debug for NativeOperationStageCustodyBasis {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeOperationStageCustodyBasis")
            .finish_non_exhaustive()
    }
}

impl NativeOperationStageCustodyBasis {
    fn operation(&self) -> &NativeOperationRecordingAuthority {
        self.stage.operation()
    }

    fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.stage.observation()
    }

    fn checkpoint_authority(&self) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
        self.plan.authority.context()
    }

    fn prepare_checkpoint(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<crate::resident::PreparedOperationObservationCheckpoint<'_>, OperationAppendFailure>
    {
        let append = self.stage.prepare_append(resolve_store)?;
        append
            .prepare_checkpoint(
                self.stage.operation().checkpoint_selection().clone(),
                &self.plan.predecessor.0,
                &self.plan.predecessor.1,
                self.plan.limits,
                custody,
            )
            .map_err(Into::into)
    }
}

/// Exact failed stage preparation plus its fixed checkpoint continuation.
#[must_use = "retry only this exact native-outcome stage preparation"]
pub struct NativeOperationStagePreparationContinuation {
    preparation: NativeOperationOutcomeStagePreparation,
    plan: NativeOperationStageCheckpointPlan,
}

impl std::fmt::Debug for NativeOperationStagePreparationContinuation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeOperationStagePreparationContinuation")
            .finish_non_exhaustive()
    }
}

impl NativeOperationStagePreparationContinuation {
    pub(in crate::resident) fn operation(&self) -> &NativeOperationRecordingAuthority {
        self.preparation.operation()
    }

    pub(in crate::resident) fn signing_authority(&self) -> &SigningAuthorityBindingV1 {
        self.preparation.signing_authority()
    }

    pub(in crate::resident) fn predecessor_checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.plan.predecessor.0, &self.plan.predecessor.1)
    }

    pub(in crate::resident) fn checkpoint_authority(
        &self,
    ) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
        self.plan.authority.context()
    }

    /// Retry the fixed stage preparation without changing its coordinates or time.
    pub(in crate::resident) fn continue_with<Rotation, Sign>(
        self,
        signing: SignedResidentAdmissionContextV1<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedNativeOperationStageCustody, NativeOperationRecordingFailure>
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
        let Self { preparation, plan } = self;
        match preparation.continue_with(signing, admit_rotation, sign) {
            Ok(stage) => Ok(PreparedNativeOperationStageCustody {
                basis: NativeOperationStageCustodyBasis {
                    stage: Box::new(stage),
                    plan,
                },
            }),
            Err(failure) => {
                let (preparation, reason) = failure.into_parts();
                Err(NativeOperationRecordingFailure::StagePreparation {
                    continuation: Box::new(Self { preparation, plan }),
                    reason,
                })
            }
        }
    }
}

/// Exact signed stage and fixed checkpoint inputs awaiting physical custody.
#[must_use = "continue custody for this exact signed native-outcome stage"]
pub struct PreparedNativeOperationStageCustody {
    basis: NativeOperationStageCustodyBasis,
}

impl std::fmt::Debug for PreparedNativeOperationStageCustody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedNativeOperationStageCustody")
            .finish_non_exhaustive()
    }
}

/// One exact signed stage checkpoint awaiting attachment and physical custody.
#[must_use = "attach this issued checkpoint without signing another checkpoint"]
pub struct IssuedNativeOperationStageCheckpoint {
    basis: NativeOperationStageCustodyBasis,
    checkpoint: ControlStateCheckpointV1,
}

impl std::fmt::Debug for IssuedNativeOperationStageCheckpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IssuedNativeOperationStageCheckpoint")
            .finish_non_exhaustive()
    }
}

impl PreparedNativeOperationStageCustody {
    pub(in crate::resident) fn operation(&self) -> &NativeOperationRecordingAuthority {
        self.basis.operation()
    }

    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.basis.observation()
    }

    pub(in crate::resident) fn predecessor_checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (
            &self.basis.plan.predecessor.0,
            &self.basis.plan.predecessor.1,
        )
    }

    pub(in crate::resident) fn checkpoint_authority(
        &self,
    ) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
        self.basis.checkpoint_authority()
    }

    fn issue_checkpoint<Sign>(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        sign: Sign,
    ) -> Result<
        (
            crate::resident::PreparedOperationObservationCheckpoint<'_>,
            ControlStateCheckpointV1,
        ),
        OperationAppendFailure,
    >
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let prepared = self.basis.prepare_checkpoint(resolve_store, custody)?;
        let authority = self.basis.plan.authority.context();
        let preimage = observation_checkpoint_preimage(
            &prepared,
            (
                &self.basis.plan.predecessor.0,
                &self.basis.plan.predecessor.1,
            ),
            &self.basis.plan.checkpoint,
            self.basis.plan.issued_at,
            &self.basis.plan.schema,
            &authority,
        )
        .map_err(OperationAppendFailure::Operation)?;
        let checkpoint = prepared
            .issue_checkpoint_value(preimage, &self.basis.plan.schema, &authority, sign)
            .map_err(OperationAppendFailure::Projection)?;
        Ok((prepared, checkpoint))
    }

    /// Prepare roots, sign the fixed checkpoint, attach it, and enter custody.
    ///
    /// # Errors
    /// Returns the exact pre-sign continuation, the exact issued checkpoint after
    /// attachment failure, or the existing physical custody continuation.
    pub(in crate::resident) fn continue_custody<Sign>(
        self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        sign: Sign,
    ) -> Result<NativeOperationStageFinalization, NativeOperationRecordingFailure>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.continue_custody_with_checkpoint_attachment(
            resolve_store,
            custody,
            anchor,
            sign,
            |prepared, reference, checkpoint| {
                prepared.attach_issued_checkpoint(reference, checkpoint)
            },
        )
    }

    /// Continue through one selected checkpoint-attachment implementation.
    ///
    /// This private seam separates issuance from attachment. A rejected
    /// attachment returns an issued continuation that accepts no signer.
    pub(in crate::resident) fn continue_custody_with_checkpoint_attachment<Sign, Attach>(
        self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        sign: Sign,
        attach: Attach,
    ) -> Result<NativeOperationStageFinalization, NativeOperationRecordingFailure>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Attach: for<'prepared> FnOnce(
            crate::resident::PreparedOperationObservationCheckpoint<'prepared>,
            &ResidentRecordRefV1,
            &ControlStateCheckpointV1,
        ) -> Result<
            crate::resident::PreparedOperationObservationCustody,
            crate::resident::OperationObservationCustodyProjectionError,
        >,
    {
        let (prepared, checkpoint) = match self.issue_checkpoint(resolve_store, custody, sign) {
            Ok(issued) => issued,
            Err(failure) => {
                return Err(NativeOperationRecordingFailure::PreparedStage {
                    continuation: Box::new(self),
                    failure: Box::new(failure),
                });
            }
        };
        let physical = match attach(prepared, &self.basis.plan.checkpoint.reference, &checkpoint) {
            Ok(physical) => physical,
            Err(reason) => {
                return Err(NativeOperationRecordingFailure::IssuedStage {
                    continuation: Box::new(IssuedNativeOperationStageCheckpoint {
                        basis: self.basis,
                        checkpoint,
                    }),
                    failure: Box::new(OperationAppendFailure::Projection(reason)),
                });
            }
        };
        let checkpoint_ref = self.basis.plan.checkpoint.reference.clone();
        complete_stage_custody(
            self.basis,
            physical,
            (checkpoint_ref, checkpoint),
            custody,
            anchor,
        )
    }
}

impl IssuedNativeOperationStageCheckpoint {
    pub(in crate::resident) fn operation(&self) -> &NativeOperationRecordingAuthority {
        self.basis.operation()
    }

    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.basis.observation()
    }

    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.basis.plan.checkpoint.reference, &self.checkpoint)
    }

    /// Rebuild roots and attach the exact already signed checkpoint.
    ///
    /// This continuation accepts no signer and cannot issue another checkpoint.
    pub(in crate::resident) fn continue_custody(
        self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<NativeOperationStageFinalization, NativeOperationRecordingFailure> {
        let prepared = match self.basis.prepare_checkpoint(resolve_store, custody) {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Err(NativeOperationRecordingFailure::IssuedStage {
                    continuation: Box::new(self),
                    failure: Box::new(failure),
                });
            }
        };
        let physical = match prepared
            .attach_issued_checkpoint(&self.basis.plan.checkpoint.reference, &self.checkpoint)
        {
            Ok(physical) => physical,
            Err(reason) => {
                return Err(NativeOperationRecordingFailure::IssuedStage {
                    continuation: Box::new(self),
                    failure: Box::new(OperationAppendFailure::Projection(reason)),
                });
            }
        };
        let checkpoint_ref = self.basis.plan.checkpoint.reference.clone();
        let checkpoint = self.checkpoint.clone();
        complete_stage_custody(
            self.basis,
            physical,
            (checkpoint_ref, checkpoint),
            custody,
            anchor,
        )
    }
}

fn complete_stage_custody(
    basis: NativeOperationStageCustodyBasis,
    physical: crate::resident::PreparedOperationObservationCustody,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<NativeOperationStageFinalization, NativeOperationRecordingFailure> {
    match commit_prepared_checkpoint(physical, checkpoint, custody, anchor) {
        Ok((custody, checkpoint)) => Ok(NativeOperationStageFinalization {
            stage: basis.stage,
            custody,
            checkpoint,
            completed_at: basis.plan.issued_at,
        }),
        Err(OperationAppendFailure::Commit {
            checkpoint,
            failure,
        }) => Err(NativeOperationRecordingFailure::StagingCommit(Box::new(
            NativeOperationStagingCommitFailure {
                stage: basis.stage,
                checkpoint,
                completed_at: basis.plan.issued_at,
                failure,
            },
        ))),
        Err(OperationAppendFailure::Anchor {
            checkpoint,
            failure,
        }) => Err(NativeOperationRecordingFailure::StagingAnchor(Box::new(
            NativeOperationStagingAnchorPending {
                stage: basis.stage,
                checkpoint,
                completed_at: basis.plan.issued_at,
                custody: NativeOperationStageAnchorCustody::Failed(failure),
            },
        ))),
        Err(OperationAppendFailure::Operation(_) | OperationAppendFailure::Projection(_)) => {
            unreachable!("an already prepared custody pair has no semantic preparation phase")
        }
    }
}

/// Exact terminal evidence whose database and independent anchor both completed.
/// This proves neither Event publication nor native-owner acceptance.
#[derive(Debug)]
pub struct CompletedNativeOperation {
    result: Result<CapabilityResultV1, AdmittedDiagnostic>,
    observation: (ResidentRecordRefV1, OperationObservationV1),
    custody: CompletedOperationObservationCustody,
}

impl CompletedNativeOperation {
    /// Borrow the complete signed computation or admitted failed-handler diagnostic.
    ///
    /// # Errors
    /// The error branch is the admitted diagnostic of an actually failed handler,
    /// not a failure of this borrow or of terminal custody.
    pub fn result(&self) -> Result<&CapabilityResultV1, &AdmittedDiagnostic> {
        self.result.as_ref()
    }

    /// Borrow the exact committed terminal observation.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation.0, &self.observation.1)
    }

    /// Borrow the exact physical custody, including its replay classification.
    #[must_use]
    pub const fn custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }
}

/// Terminal-recording inputs for one already admitted native-outcome stage.
pub struct NativeOperationTerminalRecordingInput<'a> {
    /// Fresh successful result reference.
    pub result_ref: ResidentRecordRefV1,
    /// Fresh failed-handler diagnostic reference.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Fresh terminal observation reference.
    pub terminal_ref: ResidentRecordRefV1,
    /// Checkpoint identity for the terminal append.
    pub terminal_checkpoint: OperationCheckpointInput,
    /// Selected record-time source; this does not establish clock trust.
    pub clock: &'a dyn WallClockSourcePort,
    /// Existing owner of the observation's store coordinate.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    /// Independently selected checkpoint schema.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Resolved checkpoint signer.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Resolved observation signer.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Bound on a failed handler's diagnostic.
    pub maximum_diagnostic_bytes: usize,
    /// Existing native accumulator preparation bounds.
    pub accumulator_limits: AccumulatorPreparationLimits,
}

/// An admitted staged head that can proceed only to terminal recording.
#[must_use = "continue terminal recording from the exact admitted staged head"]
pub struct AdmittedStagedNativeOperationOutcome {
    staged: StagedNativeOperationOutcome,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    completed_at: UnixTimeV1,
    recording_floor: UnixTimeV1,
}

impl std::fmt::Debug for AdmittedStagedNativeOperationOutcome {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmittedStagedNativeOperationOutcome")
            .field("staged", &self.staged)
            .finish_non_exhaustive()
    }
}

impl AdmittedStagedNativeOperationOutcome {
    /// Borrow the retained handler disposition without gaining a staging route.
    pub const fn outcome(&self) -> &NativeOperationOutcome {
        self.staged.outcome()
    }

    /// Borrow the exact checkpoint that admitted the staged head.
    #[must_use]
    pub const fn checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    /// Record terminal evidence from this exact admitted staged head.
    ///
    /// No handler or staging operation is reachable here. Failures before a
    /// physical terminal candidate return this staged phase; once a candidate
    /// exists, its custody owner becomes the sole continuation.
    ///
    /// # Errors
    ///
    /// Returns the exact pre-candidate staged continuation, or the exact
    /// terminal custody failure after a physical candidate exists.
    #[allow(clippy::too_many_arguments)]
    pub fn record_terminal<Rotation, ObservationSign, CheckpointSign, ResultSign>(
        self,
        input: NativeOperationTerminalRecordingInput<'_>,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
        sign_result: ResultSign,
    ) -> Result<CompletedNativeOperation, NativeOperationRecordingFailure>
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
        ResultSign: FnOnce(
            &[u8],
            &CapabilityResultPreimageV1,
            &CapabilityResultDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let now = match sample_time(input.clock, self.recording_floor) {
            Ok(now) => now,
            Err(reason) => {
                return Err(NativeOperationRecordingFailure::TerminalPreparation {
                    staged: Box::new(self),
                    failure: Box::new(OperationAppendFailure::Operation(reason)),
                });
            }
        };
        match self.staged.outcome() {
            NativeOperationOutcome::Succeeded(executed) => {
                let prepared = match executed.prepare_completion(
                    NativeOperationCompletionInput {
                        recovered_history: None,
                        result_ref: input.result_ref,
                        observation_ref: input.terminal_ref,
                        completed_at: self.completed_at,
                        observed_at: now,
                        signing: input.signing,
                    },
                    sign_result,
                    admit_rotation,
                    sign_observation,
                ) {
                    Ok(prepared) => prepared,
                    Err(reason) => {
                        return Err(NativeOperationRecordingFailure::TerminalPreparation {
                            staged: Box::new(self),
                            failure: Box::new(OperationAppendFailure::Operation(reason)),
                        });
                    }
                };
                let append = match prepared.prepare_append(input.resolve_store) {
                    Ok(append) => append,
                    Err(reason) => {
                        return Err(NativeOperationRecordingFailure::TerminalPreparation {
                            staged: Box::new(self),
                            failure: Box::new(OperationAppendFailure::Projection(reason)),
                        });
                    }
                };
                let result = prepared.result().clone();
                let observation = (
                    prepared.observation().0.clone(),
                    prepared.observation().1.clone(),
                );
                let completion = append_checkpoint_for_selection(
                    append,
                    self.staged.operation().checkpoint_selection().clone(),
                    (&self.checkpoint.0, &self.checkpoint.1),
                    input.terminal_checkpoint,
                    now,
                    input.checkpoint_schema,
                    input.checkpoint_authority,
                    input.accumulator_limits,
                    custody,
                    anchor,
                    sign_checkpoint,
                );
                drop(prepared);
                let completed = match completion {
                    Ok((completed, _)) => completed,
                    Err(
                        failure @ (OperationAppendFailure::Operation(_)
                        | OperationAppendFailure::Projection(_)),
                    ) => {
                        return Err(NativeOperationRecordingFailure::TerminalPreparation {
                            staged: Box::new(self),
                            failure: Box::new(failure),
                        });
                    }
                    Err(
                        failure @ (OperationAppendFailure::Commit { .. }
                        | OperationAppendFailure::Anchor { .. }),
                    ) => {
                        return Err(NativeOperationRecordingFailure::TerminalCustody {
                            failure: Box::new(failure),
                        });
                    }
                };
                Ok(CompletedNativeOperation {
                    result: Ok(result),
                    observation,
                    custody: completed,
                })
            }
            NativeOperationOutcome::Failed(failed) => {
                let prepared = match failed.prepare_failure(
                    NativeOperationFailureInput {
                        recovered_history: None,
                        diagnostic_ref: input.diagnostic_ref,
                        observation_ref: input.terminal_ref,
                        completed_at: self.completed_at,
                        observed_at: now,
                        signing: input.signing,
                        maximum_diagnostic_bytes: input.maximum_diagnostic_bytes,
                    },
                    admit_rotation,
                    sign_observation,
                ) {
                    Ok(prepared) => prepared,
                    Err(reason) => {
                        return Err(NativeOperationRecordingFailure::TerminalPreparation {
                            staged: Box::new(self),
                            failure: Box::new(OperationAppendFailure::Operation(reason)),
                        });
                    }
                };
                let append = match prepared.prepare_append(input.resolve_store) {
                    Ok(append) => append,
                    Err(reason) => {
                        return Err(NativeOperationRecordingFailure::TerminalPreparation {
                            staged: Box::new(self),
                            failure: Box::new(OperationAppendFailure::Projection(reason)),
                        });
                    }
                };
                let diagnostic = prepared.diagnostic().clone();
                let observation = (
                    prepared.observation().0.clone(),
                    prepared.observation().1.clone(),
                );
                let completion = append_checkpoint_for_selection(
                    append,
                    self.staged.operation().checkpoint_selection().clone(),
                    (&self.checkpoint.0, &self.checkpoint.1),
                    input.terminal_checkpoint,
                    now,
                    input.checkpoint_schema,
                    input.checkpoint_authority,
                    input.accumulator_limits,
                    custody,
                    anchor,
                    sign_checkpoint,
                );
                drop(prepared);
                let completed = match completion {
                    Ok((completed, _)) => completed,
                    Err(
                        failure @ (OperationAppendFailure::Operation(_)
                        | OperationAppendFailure::Projection(_)),
                    ) => {
                        return Err(NativeOperationRecordingFailure::TerminalPreparation {
                            staged: Box::new(self),
                            failure: Box::new(failure),
                        });
                    }
                    Err(
                        failure @ (OperationAppendFailure::Commit { .. }
                        | OperationAppendFailure::Anchor { .. }),
                    ) => {
                        return Err(NativeOperationRecordingFailure::TerminalCustody {
                            failure: Box::new(failure),
                        });
                    }
                };
                Ok(CompletedNativeOperation {
                    result: Err(diagnostic),
                    observation,
                    custody: completed,
                })
            }
        }
    }
}

/// Record terminal evidence from one protected recovered native-outcome stage.
///
/// Recovery preserves the handler's original completion time. Only the new
/// terminal observation time is fenced by the recovered head and current
/// checkpoint. No handler or staging operation is reachable from this path.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn record_recovered_native_operation_outcome<
    Rotation,
    ObservationSign,
    CheckpointSign,
    ResultSign,
>(
    recovered: RecoveredNativeOperationOutcomePhase,
    history: &crate::resident::RecoveredOperationObservation,
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    input: NativeOperationTerminalRecordingInput<'_>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
    sign_result: ResultSign,
) -> Result<CompletedNativeOperation, NativeOperationRecordingFailure>
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
    ResultSign: FnOnce(
        &[u8],
        &CapabilityResultPreimageV1,
        &CapabilityResultDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let completed_at = recovered.completed_at();
    let recording_floor = completed_at
        .max(*history.selected_head().1.observed_at())
        .max(*checkpoint.1.issued_at());
    AdmittedStagedNativeOperationOutcome {
        staged: recovered.into_staged(),
        checkpoint: (checkpoint.0.clone(), checkpoint.1.clone()),
        completed_at,
        recording_floor,
    }
    .record_terminal(
        input,
        custody,
        anchor,
        admit_rotation,
        sign_observation,
        sign_checkpoint,
        sign_result,
    )
}

/// Complete database-and-anchor evidence awaiting semantic stage admission.
#[must_use = "finish the exact committed native-outcome stage"]
pub struct NativeOperationStageFinalization {
    stage: Box<PreparedNativeOperationOutcomeStage>,
    custody: CompletedOperationObservationCustody,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    completed_at: UnixTimeV1,
}

impl NativeOperationStageFinalization {
    pub(in crate::resident) fn operation(&self) -> &NativeOperationRecordingAuthority {
        self.stage.operation()
    }

    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.stage.observation()
    }

    /// Borrow the exact outcome checkpoint retained across finalization.
    #[must_use]
    pub const fn checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    pub(in crate::resident) fn retained_checkpoint_coordinate(
        &self,
    ) -> &pale_ports::ControlRecordCoordinate {
        self.custody
            .candidate()
            .checkpoint()
            .successor()
            .coordinate()
    }

    /// Join the semantic stage to its exact completed physical custody.
    ///
    /// # Errors
    /// Returns a terminal association failure if the completed candidate does
    /// not match the prepared stage or its exact retained checkpoint. No retry
    /// token is returned. Cold recovery is the only continuation after this
    /// invariant failure.
    pub fn finish(
        self,
    ) -> Result<AdmittedStagedNativeOperationOutcome, NativeOperationStageFinalizationFailure> {
        let retained_checkpoint = self.retained_checkpoint_coordinate().clone();
        self.finish_against_retained_checkpoint(&retained_checkpoint)
    }

    /// Join the stage against one already selected physical checkpoint head.
    ///
    /// The process host uses the coordinate retained by this finalization. The
    /// explicit argument keeps the final association check independently
    /// testable without adding another effect or retry path.
    pub(in crate::resident) fn finish_against_retained_checkpoint(
        self,
        retained_checkpoint: &pale_ports::ControlRecordCoordinate,
    ) -> Result<AdmittedStagedNativeOperationOutcome, NativeOperationStageFinalizationFailure> {
        let Self {
            stage,
            custody,
            checkpoint,
            completed_at,
        } = self;
        if retained_checkpoint.record_ref() != &checkpoint.0
            || retained_checkpoint.record_sha256().as_str()
                != checkpoint.1.canonical_control_checkpoint_sha256().as_str()
        {
            return Err(NativeOperationStageFinalizationFailure {
                reason: OperationAttemptError::Association,
            });
        }
        match (*stage).finish(&custody) {
            Ok(staged) => Ok(AdmittedStagedNativeOperationOutcome {
                staged,
                checkpoint,
                completed_at,
                recording_floor: completed_at,
            }),
            Err(failure) => Err(NativeOperationStageFinalizationFailure {
                reason: failure.into_reason(),
            }),
        }
    }
}

/// Semantic stage admission failed after exact physical completion.
pub struct NativeOperationStageFinalizationFailure {
    reason: OperationAttemptError,
}

impl std::fmt::Debug for NativeOperationStageFinalizationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeOperationStageFinalizationFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl NativeOperationStageFinalizationFailure {
    /// Borrow the exact semantic admission failure.
    #[must_use]
    pub const fn reason(&self) -> &OperationAttemptError {
        &self.reason
    }

    /// Consume the terminal semantic-admission refusal.
    pub(in crate::resident) fn into_reason(self) -> OperationAttemptError {
        self.reason
    }
}

enum NativeOperationStageAnchorCustody {
    Pending(Box<DatabaseCommittedOperationObservationAnchorPending>),
    Failed(OperationObservationCustodyAnchorFailure),
}

/// Exact database-proven staging candidate awaiting only its independent anchor.
#[must_use = "continue only the exact native-outcome anchor transition"]
pub struct NativeOperationStagingAnchorPending {
    stage: Box<PreparedNativeOperationOutcomeStage>,
    checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    completed_at: UnixTimeV1,
    custody: NativeOperationStageAnchorCustody,
}

impl std::fmt::Debug for NativeOperationStagingAnchorPending {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeOperationStagingAnchorPending")
            .finish_non_exhaustive()
    }
}

impl NativeOperationStagingAnchorPending {
    pub(in crate::resident) fn operation(&self) -> &NativeOperationRecordingAuthority {
        self.stage.operation()
    }

    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.stage.observation()
    }

    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    /// Continue only the retained anchor transition.
    ///
    /// # Errors
    /// Returns this exact pending phase when the anchor still cannot be proved.
    pub fn continue_anchor(
        self,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<NativeOperationStageFinalization, Self> {
        let Self {
            stage,
            checkpoint,
            completed_at,
            custody,
        } = self;
        let pending = match custody {
            NativeOperationStageAnchorCustody::Pending(pending) => *pending,
            NativeOperationStageAnchorCustody::Failed(failure) => {
                match failure.try_into_retryable() {
                    Ok(pending) => pending,
                    Err(failure) => {
                        return Err(Self {
                            stage,
                            checkpoint,
                            completed_at,
                            custody: NativeOperationStageAnchorCustody::Failed(failure),
                        });
                    }
                }
            }
        };
        match pending.advance_anchor(anchor) {
            Ok(custody) => Ok(NativeOperationStageFinalization {
                stage,
                custody,
                checkpoint: *checkpoint,
                completed_at,
            }),
            Err(failure) => Err(Self {
                stage,
                checkpoint,
                completed_at,
                custody: NativeOperationStageAnchorCustody::Failed(failure),
            }),
        }
    }
}

/// Outcome staging has unresolved database custody for one exact candidate.
#[must_use = "perform exact read-back; do not prepare another staging candidate"]
pub struct NativeOperationStagingCommitFailure {
    stage: Box<PreparedNativeOperationOutcomeStage>,
    checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    completed_at: UnixTimeV1,
    failure: OperationObservationCustodyCommitFailure,
}

impl std::fmt::Debug for NativeOperationStagingCommitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeOperationStagingCommitFailure")
            .field("reason", self.failure.reason())
            .finish_non_exhaustive()
    }
}

impl NativeOperationStagingCommitFailure {
    pub(in crate::resident) fn operation(&self) -> &NativeOperationRecordingAuthority {
        self.stage.operation()
    }

    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.stage.observation()
    }

    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    /// Borrow the existing custody owner's exact classification.
    #[must_use]
    pub const fn reason(&self) -> &crate::resident::OperationObservationCustodyCommitFailureReason {
        self.failure.reason()
    }

    /// Read back the retained exact candidate without committing or rebuilding it.
    ///
    /// # Errors
    /// Returns this phase intact unless the read-back proves the exact successor.
    pub fn reconcile_exact(
        self,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<NativeOperationStagingAnchorPending, Self> {
        let Self {
            stage,
            checkpoint,
            completed_at,
            failure,
        } = self;
        match failure.reconcile_exact(custody) {
            Ok(pending) => Ok(NativeOperationStagingAnchorPending {
                stage,
                checkpoint,
                completed_at,
                custody: NativeOperationStageAnchorCustody::Pending(Box::new(pending)),
            }),
            Err(failure) => Err(Self {
                stage,
                checkpoint,
                completed_at,
                failure,
            }),
        }
    }
}

/// Native outcome recording stopped in one explicit owned phase.
pub enum NativeOperationRecordingFailure {
    /// Time sampling failed before a staging candidate was prepared.
    Unstaged {
        /// The sole outcome value still capable of preparing a stage.
        outcome: Box<UnstagedNativeOperationOutcome>,
        /// Exact clock or time-order refusal.
        reason: OperationAttemptError,
    },
    /// Stage preparation failed before a candidate existed.
    StagePreparation {
        /// Exact stage preparation and fixed checkpoint inputs.
        continuation: Box<NativeOperationStagePreparationContinuation>,
        /// Exact deterministic preparation or signing refusal.
        reason: OperationAttemptError,
    },
    /// A signed stage exists, but physical/checkpoint preparation stopped before custody.
    PreparedStage {
        /// Exact signed stage and fixed checkpoint inputs.
        continuation: Box<PreparedNativeOperationStageCustody>,
        /// Exact pre-custody failure.
        failure: Box<OperationAppendFailure>,
    },
    /// A checkpoint was signed once, but attachment stopped before custody.
    IssuedStage {
        /// Exact issued checkpoint and fixed stage inputs. This accepts no signer.
        continuation: Box<IssuedNativeOperationStageCheckpoint>,
        /// Exact attachment or root-preparation failure.
        failure: Box<OperationAppendFailure>,
    },
    /// Database custody of the exact stage remains unresolved.
    StagingCommit(Box<NativeOperationStagingCommitFailure>),
    /// Database custody is exact; only the retained anchor transition may continue.
    StagingAnchor(Box<NativeOperationStagingAnchorPending>),
    /// Physical stage completion awaits semantic admission.
    StageFinalization(Box<NativeOperationStageFinalizationFailure>),
    /// An admitted staged head stopped before a terminal custody candidate existed.
    TerminalPreparation {
        /// Exact admitted staged head; it cannot be staged again.
        staged: Box<AdmittedStagedNativeOperationOutcome>,
        /// Exact terminal preparation failure.
        failure: Box<OperationAppendFailure>,
    },
    /// One exact terminal candidate owns all remaining custody continuation.
    TerminalCustody {
        /// Database or independent-anchor failure retaining the exact candidate.
        failure: Box<OperationAppendFailure>,
    },
}

impl std::fmt::Debug for NativeOperationRecordingFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Unstaged { .. } => "Unstaged",
            Self::StagePreparation { .. } => "StagePreparation",
            Self::PreparedStage { .. } => "PreparedStage",
            Self::IssuedStage { .. } => "IssuedStage",
            Self::StagingCommit(_) => "StagingCommit",
            Self::StagingAnchor(_) => "StagingAnchor",
            Self::StageFinalization(_) => "StageFinalization",
            Self::TerminalPreparation { .. } => "TerminalPreparation",
            Self::TerminalCustody { .. } => "TerminalCustody",
        };
        formatter
            .debug_struct("NativeOperationRecordingFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// A stopped invocation, retaining the actual unrun call or executed outcome.
/// None of these variants authorizes replay, another handler call or blind retry.
pub enum NativeOperationInvokeFailure<'operation, 'registry> {
    /// No attempt was prepared and no operation-service write occurred.
    Admission(OperationAttemptError),
    /// The unopened attempt remains available with its exact append failure.
    Preparation {
        /// Original prepared attempt, not a replacement handler.
        attempt: Box<PreparedNativeOperationAttempt<'operation, 'registry>>,
        /// Exact preparation or custody failure.
        failure: Box<OperationAppendFailure>,
    },
    /// Preparation is durable, but dispatch could not be signed.
    DispatchPreparation {
        /// Original unopened attempt.
        attempt: Box<PreparedNativeOperationAttempt<'operation, 'registry>>,
        /// Completed preparation custody.
        completed: Box<CompletedOperationObservationCustody>,
        /// Exact preparation checkpoint for continued append construction.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        /// Current-authority, clock or signing refusal.
        reason: Box<OperationAttemptError>,
    },
    /// The unopened dispatch remains available with its exact append failure.
    Dispatch {
        /// Original dispatch. Only fresh New custody can release it.
        dispatch: Box<PreparedNativeOperationDispatch<'operation, 'registry>>,
        /// Exact dispatch preparation or custody failure.
        failure: Box<OperationAppendFailure>,
    },
    /// Original unopened call, time history and New-custody classification.
    Invocation(Box<NativeOperationInvocationRefusal<'operation, 'registry>>),
    /// Actual handler outcome retained in its exact recording phase.
    Recording(NativeOperationRecordingFailure),
}

impl std::fmt::Debug for NativeOperationInvokeFailure<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stage = match self {
            Self::Admission(_) => "Admission",
            Self::Preparation { .. } => "Preparation",
            Self::DispatchPreparation { .. } => "DispatchPreparation",
            Self::Dispatch { .. } => "Dispatch",
            Self::Invocation(_) => "Invocation",
            Self::Recording(_) => "Recording",
        };
        formatter
            .debug_struct("NativeOperationInvokeFailure")
            .field("stage", &stage)
            .finish_non_exhaustive()
    }
}

/// Run a selected body-only native computation and retain its terminal evidence.
///
/// The operation must already be issued or recovered through protected custody.
/// All current authority, key custody, clock trust and store/anchor selection remain with
/// the supplied owners. Recovery restores the immutable operation and request;
/// this service still rechecks current authority before it prepares an attempt.
/// Signers must establish authority for each actual issue time. This entry
/// performs no provider dispatch, context gathering, Event publication or effect.
/// It samples time at preparation, dispatch and completion in addition to the
/// existing release checks. It never retries a physical mutation or a handler.
///
/// # Errors
/// Returns the precise stopped phase, retaining the unopened call or actual
/// outcome and existing owned custody failures. Unknown dispatch commitment
/// cannot be upgraded to permission to execute.
/// One durably dispatched built-in call that has not run its handler.
///
/// This private phase is suitable for a resident task owner: preparation and
/// dispatch custody are complete, while the linear handler remains unopened.
#[must_use = "execute or explicitly retain this exact dispatched native work"]
pub(in crate::resident) struct PreparedNativeOperationWork<'operation, 'call, 'registry> {
    dispatch: PreparedNativeOperationDispatch<'operation, 'registry>,
    dispatch_done: CompletedOperationObservationCustody,
    dispatch_checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    current: AmbientSessionAdmissionContext<'operation>,
    outcome_ref: ResidentRecordRefV1,
    outcome_observation_ref: ResidentRecordRefV1,
    result_ref: ResidentRecordRefV1,
    diagnostic_ref: ResidentRecordRefV1,
    terminal_ref: ResidentRecordRefV1,
    outcome_checkpoint: OperationCheckpointInput,
    terminal_checkpoint: OperationCheckpointInput,
    clock: &'call dyn WallClockSourcePort,
    resolve_store: &'call dyn OperationObservationAuthorityStoreResolver,
    checkpoint_schema: &'call SchemaDigest,
    checkpoint_authority: &'call ControlCheckpointSigningAuthorityAdmissionContextV1<'call>,
    signing: SignedResidentAdmissionContextV1<'call>,
    maximum_record_bytes: usize,
    maximum_diagnostic_bytes: usize,
    maximum_outcome_bytes: usize,
    accumulator_limits: AccumulatorPreparationLimits,
}

impl PreparedNativeOperationWork<'_, '_, '_> {
    /// Borrow the exact durable dispatch offered to the task owner.
    pub(in crate::resident) fn dispatch(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.dispatch.observation_ref, &self.dispatch.observation)
    }
}

/// Retain native preparation and dispatch without running the handler.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn prepare_native_operation_work<
    'a,
    'call,
    'registry,
    Authority,
    Rotation,
    ObservationSign,
    CheckpointSign,
>(
    input: NativeOperationInvokeInput<'a, 'call, 'registry>,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_rotation: &mut Rotation,
    sign_observation: &mut ObservationSign,
    sign_checkpoint: &mut CheckpointSign,
) -> Result<
    PreparedNativeOperationWork<'a, 'call, 'registry>,
    NativeOperationInvokeFailure<'a, 'registry>,
>
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
{
    validate_references(&input).map_err(NativeOperationInvokeFailure::Admission)?;
    let NativeOperationInvokeInput {
        attempt: attempt_input,
        checkpoint,
        checkpoint_schema,
        checkpoint_authority,
        signing,
        preparation_checkpoint,
        dispatch_checkpoint,
        outcome_checkpoint,
        terminal_checkpoint,
        dispatch_ref,
        outcome_ref,
        outcome_observation_ref,
        result_ref,
        diagnostic_ref,
        terminal_ref,
        clock,
        resolve_store,
        maximum_record_bytes,
        maximum_diagnostic_bytes,
        maximum_outcome_bytes,
        accumulator_limits,
    } = input;
    let operation = attempt_input.context.operation;
    let original_current = attempt_input.context.current;
    let now = sample_time(clock, original_current.now.max(*checkpoint.1.issued_at()))
        .map_err(NativeOperationInvokeFailure::Admission)?;
    let current = original_current.at_time(now);
    let attempt = prepare_native_operation_attempt_at(
        attempt_input,
        &current,
        authority,
        &mut *admit_rotation,
        &mut *sign_observation,
    )
    .map_err(NativeOperationInvokeFailure::Admission)?;
    let prepared = attempt
        .prepare_append(resolve_store)
        .map_err(OperationAppendFailure::from)
        .and_then(|append| {
            append_checkpoint(
                append,
                operation,
                checkpoint,
                preparation_checkpoint,
                now,
                checkpoint_schema,
                checkpoint_authority,
                accumulator_limits,
                custody,
                anchor,
                &mut *sign_checkpoint,
            )
        });
    let (prepared_done, preparation_checkpoint) = match prepared {
        Ok(value) => value,
        Err(failure) => {
            return Err(NativeOperationInvokeFailure::Preparation {
                attempt: Box::new(attempt),
                failure: Box::new(failure),
            });
        }
    };
    let dispatch = sample_time(clock, now).and_then(|now| {
        let current = original_current.at_time(now);
        attempt.prepare_dispatch_observation(
            NativeOperationDispatchContext {
                completed: &prepared_done,
                current: &current,
                signing,
                observation_ref: dispatch_ref,
                resolve_store,
            },
            authority,
            &mut *admit_rotation,
            &mut *sign_observation,
        )
    });
    let (observation_ref, observation) = match dispatch {
        Ok(value) => value,
        Err(reason) => {
            return Err(NativeOperationInvokeFailure::DispatchPreparation {
                attempt: Box::new(attempt),
                completed: Box::new(prepared_done),
                checkpoint: Box::new(preparation_checkpoint),
                reason: Box::new(reason),
            });
        }
    };
    let now = *observation.observed_at();
    let dispatch = PreparedNativeOperationDispatch {
        attempt,
        observation_ref,
        observation,
    };
    let appended = dispatch
        .prepare_append(resolve_store)
        .map_err(OperationAppendFailure::from)
        .and_then(|append| {
            append_checkpoint(
                append,
                operation,
                (&preparation_checkpoint.0, &preparation_checkpoint.1),
                dispatch_checkpoint,
                now,
                checkpoint_schema,
                checkpoint_authority,
                accumulator_limits,
                custody,
                anchor,
                &mut *sign_checkpoint,
            )
        });
    let (dispatch_done, dispatch_checkpoint) = match appended {
        Ok(value) => value,
        Err(failure) => {
            return Err(NativeOperationInvokeFailure::Dispatch {
                dispatch: Box::new(dispatch),
                failure: Box::new(failure),
            });
        }
    };
    Ok(PreparedNativeOperationWork {
        dispatch,
        dispatch_done,
        dispatch_checkpoint,
        current: original_current.at_time(now),
        outcome_ref,
        outcome_observation_ref,
        result_ref,
        diagnostic_ref,
        terminal_ref,
        outcome_checkpoint,
        terminal_checkpoint,
        clock,
        resolve_store,
        checkpoint_schema,
        checkpoint_authority,
        signing,
        maximum_record_bytes,
        maximum_diagnostic_bytes,
        maximum_outcome_bytes,
        accumulator_limits,
    })
}

/// Execute one task-owned native call and retain its terminal evidence.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn execute_native_operation_work<
    'operation,
    'call,
    'registry,
    Authority,
    Rotation,
    ObservationSign,
    CheckpointSign,
    ResultSign,
>(
    work: PreparedNativeOperationWork<'operation, 'call, 'registry>,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    mut admit_rotation: Rotation,
    mut sign_observation: ObservationSign,
    mut sign_checkpoint: CheckpointSign,
    mut sign_result: ResultSign,
) -> Result<CompletedNativeOperation, NativeOperationInvokeFailure<'operation, 'registry>>
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
    ResultSign: FnMut(
        &[u8],
        &CapabilityResultPreimageV1,
        &CapabilityResultDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let PreparedNativeOperationWork {
        dispatch,
        dispatch_done,
        dispatch_checkpoint,
        current,
        outcome_ref,
        outcome_observation_ref,
        result_ref,
        diagnostic_ref,
        terminal_ref,
        outcome_checkpoint,
        terminal_checkpoint,
        clock,
        resolve_store,
        checkpoint_schema,
        checkpoint_authority,
        signing,
        maximum_record_bytes,
        maximum_diagnostic_bytes,
        maximum_outcome_bytes,
        accumulator_limits,
    } = work;
    let outcome = dispatch
        .invoke(
            dispatch_done,
            NativeOperationExecutionContext {
                current: &current,
                resolve_store,
                clock,
                maximum_record_bytes,
            },
            authority,
            custody,
            anchor,
        )
        .map_err(|refusal| NativeOperationInvokeFailure::Invocation(Box::new(refusal)))?;
    record_native_operation_outcome(
        outcome,
        NativeOperationOutcomeRecordingInput {
            checkpoint: (&dispatch_checkpoint.0, &dispatch_checkpoint.1),
            outcome_ref,
            outcome_observation_ref,
            result_ref,
            diagnostic_ref,
            terminal_ref,
            outcome_checkpoint,
            terminal_checkpoint,
            clock,
            resolve_store,
            checkpoint_schema,
            checkpoint_authority,
            signing,
            maximum_diagnostic_bytes,
            maximum_outcome_bytes,
            accumulator_limits,
        },
        custody,
        anchor,
        &mut admit_rotation,
        &mut sign_observation,
        &mut sign_checkpoint,
        &mut sign_result,
    )
    .map_err(NativeOperationInvokeFailure::Recording)
}

/// Run a selected body-only native computation and retain its terminal evidence.
///
/// This convenience path composes the private durable-dispatch work phase with
/// its one allowed execution. Resident task owners use those phases separately.
///
/// # Errors
///
/// Returns the exact admission, dispatch, execution, or recording failure. A
/// failure after dispatch can contain durable evidence that requires recovery.
#[allow(clippy::too_many_arguments)]
pub fn invoke_native_operation<
    'a,
    'call,
    'registry,
    Authority,
    Rotation,
    ObservationSign,
    CheckpointSign,
    ResultSign,
>(
    input: NativeOperationInvokeInput<'a, 'call, 'registry>,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    mut admit_rotation: Rotation,
    mut sign_observation: ObservationSign,
    mut sign_checkpoint: CheckpointSign,
    sign_result: ResultSign,
) -> Result<CompletedNativeOperation, NativeOperationInvokeFailure<'a, 'registry>>
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
    ResultSign: FnMut(
        &[u8],
        &CapabilityResultPreimageV1,
        &CapabilityResultDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let work = prepare_native_operation_work(
        input,
        authority,
        custody,
        anchor,
        &mut admit_rotation,
        &mut sign_observation,
        &mut sign_checkpoint,
    )?;
    execute_native_operation_work(
        work,
        authority,
        custody,
        anchor,
        admit_rotation,
        sign_observation,
        sign_checkpoint,
        sign_result,
    )
}

/// Retain one actual handler outcome through the existing staging and terminal path.
///
/// This function accepts no action registry, handler or dispatch input. Every
/// failure preserves the exact linear continuation owned by the recording phase.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn record_native_operation_outcome<
    Rotation,
    ObservationSign,
    CheckpointSign,
    ResultSign,
>(
    outcome: UnstagedNativeOperationOutcome,
    input: NativeOperationOutcomeRecordingInput<'_>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    mut admit_rotation: Rotation,
    mut sign_observation: ObservationSign,
    mut sign_checkpoint: CheckpointSign,
    mut sign_result: ResultSign,
) -> Result<CompletedNativeOperation, NativeOperationRecordingFailure>
where
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
    ResultSign: FnMut(
        &[u8],
        &CapabilityResultPreimageV1,
        &CapabilityResultDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let NativeOperationOutcomeRecordingInput {
        checkpoint,
        outcome_ref,
        outcome_observation_ref,
        result_ref,
        diagnostic_ref,
        terminal_ref,
        outcome_checkpoint,
        terminal_checkpoint,
        clock,
        resolve_store,
        checkpoint_schema,
        checkpoint_authority,
        signing,
        maximum_diagnostic_bytes,
        maximum_outcome_bytes,
        accumulator_limits,
    } = input;
    let outcome_time = match sample_time(clock, *checkpoint.1.issued_at()) {
        Ok(now) => now,
        Err(reason) => {
            return Err(NativeOperationRecordingFailure::Unstaged {
                outcome: Box::new(outcome),
                reason,
            });
        }
    };
    let stage_plan = NativeOperationStageCheckpointPlan {
        predecessor: Box::new((checkpoint.0.clone(), checkpoint.1.clone())),
        checkpoint: outcome_checkpoint,
        issued_at: outcome_time,
        schema: checkpoint_schema.clone(),
        authority: RetainedCheckpointAuthority::new(checkpoint_authority),
        limits: accumulator_limits,
    };
    let stage = match outcome.prepare_stage(
        NativeOperationOutcomeStageInput {
            outcome_ref,
            observation_ref: outcome_observation_ref,
            completed_at: outcome_time,
            observed_at: outcome_time,
            signing,
            maximum_outcome_bytes,
            maximum_diagnostic_bytes,
        },
        &mut admit_rotation,
        &mut sign_observation,
    ) {
        Ok(stage) => stage,
        Err(failure) => {
            let (preparation, reason) = failure.into_parts();
            return Err(NativeOperationRecordingFailure::StagePreparation {
                continuation: Box::new(NativeOperationStagePreparationContinuation {
                    preparation,
                    plan: stage_plan,
                }),
                reason,
            });
        }
    };
    let finalization = PreparedNativeOperationStageCustody {
        basis: NativeOperationStageCustodyBasis {
            stage: Box::new(stage),
            plan: stage_plan,
        },
    }
    .continue_custody(resolve_store, custody, anchor, &mut sign_checkpoint)?;
    let staged = finalization
        .finish()
        .map_err(|failure| NativeOperationRecordingFailure::StageFinalization(Box::new(failure)))?;
    staged.record_terminal(
        NativeOperationTerminalRecordingInput {
            result_ref,
            diagnostic_ref,
            terminal_ref,
            terminal_checkpoint,
            clock,
            resolve_store,
            checkpoint_schema,
            checkpoint_authority,
            signing,
            maximum_diagnostic_bytes,
            accumulator_limits,
        },
        custody,
        anchor,
        &mut admit_rotation,
        &mut sign_observation,
        &mut sign_checkpoint,
        &mut sign_result,
    )
}

fn validate_references(
    input: &NativeOperationInvokeInput<'_, '_, '_>,
) -> Result<(), OperationAttemptError> {
    let context = &input.attempt.context;
    context.validate_original_session()?;
    context.require_context_ready()?;
    let planned = [
        &context.selection_ref,
        &context.observation_ref,
        &input.dispatch_ref,
        &input.outcome_ref,
        &input.outcome_observation_ref,
        &input.result_ref,
        &input.diagnostic_ref,
        &input.terminal_ref,
        &input.preparation_checkpoint.reference,
        &input.dispatch_checkpoint.reference,
        &input.outcome_checkpoint.reference,
        &input.terminal_checkpoint.reference,
    ];
    let original = [
        context.operation.handle().0,
        context.operation.request_coordinate().record_ref(),
        context.operation.handle().1.operation_lease_ref(),
        input.checkpoint.0,
    ];
    for (index, reference) in planned.iter().enumerate() {
        if planned[..index].contains(reference)
            || original.contains(reference)
            || context
                .observation
                .predecessor
                .is_some_and(|(prior, _)| prior == *reference)
        {
            return Err(OperationAttemptError::Association);
        }
    }
    let ids = [
        &input.preparation_checkpoint.id,
        &input.dispatch_checkpoint.id,
        &input.outcome_checkpoint.id,
        &input.terminal_checkpoint.id,
    ];
    if ids.iter().enumerate().any(|(index, id)| {
        ids[..index].contains(id) || *id == input.checkpoint.1.control_checkpoint_id()
    }) {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}
