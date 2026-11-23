//! Execute finite consultation child batches and complete collection custody.

#[path = "stop_resolution.rs"]
mod stop_resolution;
#[allow(
    dead_code,
    reason = "private Consult supervision awaits the assembled resident host"
)]
mod supervision;
#[allow(
    dead_code,
    reason = "private Consult execution awaits the assembled resident host"
)]
pub(in crate::resident) mod task;
#[path = "task_resolution.rs"]
mod task_resolution;

pub(in crate::resident) use stop_resolution::{
    commit_selected_consultation_stop, continue_consultation_stop_custody,
    ResidentConsultationStopCommitFailure, ResidentConsultationStopCustody,
};
pub(in crate::resident) use supervision::{
    apply_consultation_supervision_decision, supervise_consultation_pre_policy_turn,
    supervise_consultation_turn, supervise_recovered_consultation_pre_policy_turn,
    supervise_recovered_consultation_turn, ResidentConsultationCompletionReady,
    ResidentConsultationOfferRecovery, ResidentConsultationPolicyFrontier,
    ResidentConsultationPrePolicyNeedsRecovery, ResidentConsultationPrePolicyTurn,
    ResidentConsultationStopSelected, ResidentConsultationSupervisionDecision,
    ResidentConsultationTurnDisposition, ResidentConsultationTurnNeedsRecovery,
    ResidentConsultationTurnPending, ResidentConsultationTurnProgress,
    ResidentConsultationTurnStopped,
};
pub(in crate::resident) use task_resolution::{
    continue_consultation_terminal_custody, join_consultation_terminal,
    retry_consultation_terminal_release, settle_consultation_nonterminal,
    CompletedResidentConsultationNonterminal, CompletedResidentConsultationTerminal,
    ResidentConsultationJoinedTerminal, ResidentConsultationNonterminalSettlementFailure,
    ResidentConsultationTerminalCustody, ResidentConsultationTerminalJoinFailure,
};

use super::*;
use crate::resident::{
    operation_dispatch::{observation_checkpoint_preimage, OperationCheckpointInput},
    CompletedOperationObservationCustody, OperationObservationCustodyAnchorFailure,
    OperationObservationCustodyCommitFailure,
};
use pale_ports::{AntiRollbackAnchorPort, OperationCustodyPort, WallClockSourceError};

/// Why one planned child was not released for execution.
pub(in crate::resident) enum ConsultationChildDispatchReleaseFailure<LookupError> {
    /// Fresh parent collection recovery or semantic readmission failed.
    ParentLookup(LookupError),
    /// Protected child recovery failed before release classification.
    ChildLookup(LookupError),
    /// The release-time host clock observation failed.
    Clock(WallClockSourceError),
    /// The recovered collection no longer admitted the exact planned batch.
    Revalidation(ConsultationCollectionError),
    /// The child already crossed its first-release boundary.
    ResumeOnly,
    /// The child already has terminal evidence and must be joined, not released.
    TerminalJoin,
}

impl<LookupError> std::fmt::Debug for ConsultationChildDispatchReleaseFailure<LookupError> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::ParentLookup(_) => "ParentLookup",
            Self::ChildLookup(_) => "ChildLookup",
            Self::Clock(reason) => {
                let _redacted = reason;
                "Clock"
            }
            Self::Revalidation(reason) => {
                let _redacted = reason;
                "Revalidation"
            }
            Self::ResumeOnly => "ResumeOnly",
            Self::TerminalJoin => "TerminalJoin",
        };
        formatter
            .debug_struct("ConsultationChildDispatchReleaseFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

pub(super) fn classify_protected_child(
    current: &RecoveredConsultationCollection,
    planned: &ConsultationChildDispatchBatch,
    candidate: &ConsultationChildDispatchCandidate,
    now: UnixTimeV1,
    child: &RecoveredOperationObservation,
) -> Result<ProtectedConsultationChildPhase, ConsultationCollectionError> {
    let candidate = revalidate_planned_child(current, planned, candidate, now)?;
    classify_recovered_child(current, &candidate, child)
}

/// Classify one released child against a legitimate successor parent head.
///
/// Sibling evidence may advance the parent frontier and remove other members
/// from the outstanding set. The exact target must still be present with the
/// same immutable roster meaning, parent, and deadline.
pub(super) fn classify_released_child(
    current: &RecoveredConsultationCollection,
    planned: &ConsultationChildDispatchBatch,
    candidate: &ConsultationChildDispatchCandidate,
    now: UnixTimeV1,
    child: &RecoveredOperationObservation,
) -> Result<ProtectedConsultationChildPhase, ConsultationCollectionError> {
    let candidate = revalidate_released_child(current, planned, candidate, now)?;
    classify_recovered_child(current, &candidate, child)
}

fn classify_recovered_child(
    current: &RecoveredConsultationCollection,
    candidate: &ConsultationChildDispatchCandidate,
    child: &RecoveredOperationObservation,
) -> Result<ProtectedConsultationChildPhase, ConsultationCollectionError> {
    if child.store() != current.store()
        || child.handle.record_ref() != &candidate.child.record_ref
        || child.handle.record_sha256().as_str() != candidate.child.record_sha256.as_str()
    {
        return Err(ConsultationCollectionError::Child);
    }
    Ok(match child.selected_head().1.state() {
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving | OperationActivePhaseV1::ContextBound { .. },
        } => ProtectedConsultationChildPhase::EligibleForFirstDispatch,
        OperationObservationStateV1::Succeeded { .. }
        | OperationObservationStateV1::Failed { .. }
        | OperationObservationStateV1::Cancelled { .. } => {
            ProtectedConsultationChildPhase::TerminalJoin
        }
        OperationObservationStateV1::Active { .. }
        | OperationObservationStateV1::CancellationRequested { .. }
        | OperationObservationStateV1::Recovering { .. }
        | OperationObservationStateV1::OutcomeUnknown { .. }
        | OperationObservationStateV1::Expired { .. } => {
            ProtectedConsultationChildPhase::ResumeOnly
        }
    })
}

pub(super) fn revalidate_planned_child(
    current: &RecoveredConsultationCollection,
    planned: &ConsultationChildDispatchBatch,
    candidate: &ConsultationChildDispatchCandidate,
    now: UnixTimeV1,
) -> Result<ConsultationChildDispatchCandidate, ConsultationCollectionError> {
    current
        .revalidate_child_dispatch(planned, now)?
        .candidates
        .into_iter()
        .find(|current| current == candidate)
        .ok_or(ConsultationCollectionError::Association)
}

fn revalidate_released_child(
    current: &RecoveredConsultationCollection,
    planned: &ConsultationChildDispatchBatch,
    candidate: &ConsultationChildDispatchCandidate,
    now: UnixTimeV1,
) -> Result<ConsultationChildDispatchCandidate, ConsultationCollectionError> {
    let planned_candidate = planned
        .candidates
        .iter()
        .find(|planned_candidate| *planned_candidate == candidate)
        .ok_or(ConsultationCollectionError::Association)?;
    let current_batch = current.plan_child_dispatch(now)?;
    if current_batch.parent != planned.parent || current_batch.deadline != planned.deadline {
        return Err(ConsultationCollectionError::Association);
    }
    current_batch
        .candidates
        .into_iter()
        .find(|current_candidate| current_candidate == planned_candidate)
        .ok_or(ConsultationCollectionError::Association)
}

/// Inputs that join one recovered child to its durable parent consultation.
pub struct ConsultationChildJoinInput<'a> {
    /// Exact roster slot selected by the admitted consultation policy.
    pub slot_id: &'a ConsultationSlotId,
    /// Protected child history whose authenticated head supplies the outcome.
    pub child: &'a RecoveredOperationObservation,
    /// Time at which the parent admits the child outcome.
    pub recorded_at: UnixTimeV1,
    /// Parent Consult operation that owns the recovered collection.
    pub parent: &'a IssuedLocalOperation,
    /// Current parent observation named by the recovered collection frontier.
    pub parent_observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    /// Fresh parent observation reference for the collection successor.
    pub observation_ref: ResidentRecordRefV1,
    /// Time at which the parent observes the joined collection state.
    pub observed_at: UnixTimeV1,
    /// Current admitted signer for the parent observation.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Count and byte ceilings for the complete collection successor.
    pub limits: ConsultationCollectionLimits,
}

/// Checkpoint inputs shared by one consultation collection successor.
pub struct ConsultationSuccessorCommitInput<'a> {
    /// Native work and byte ceilings for the successor accumulator.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Fresh immutable identity for the successor checkpoint.
    pub identity: OperationCheckpointInput,
    /// Time at which the successor checkpoint is issued.
    pub issued_at: UnixTimeV1,
    /// Admitted schema identity for the successor checkpoint.
    pub schema: &'a SchemaDigest,
    /// Current authority admitted to sign the successor checkpoint.
    pub authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
}

/// Phase-typed failure while joining one protected child outcome.
pub enum ConsultationChildJoinFailure {
    /// The recovered child, collection successor, or parent observation did not admit.
    Admission(ConsultationCollectionError),
    /// The exact prepared parent successor stopped in physical custody.
    Custody(ConsultationCollectionCommitFailure),
}

impl std::fmt::Debug for ConsultationChildJoinFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Admission(_) => "Admission",
            Self::Custody(_) => "Custody",
        };
        formatter
            .debug_struct("ConsultationChildJoinFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for ConsultationChildJoinFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admission(source) => {
                write!(formatter, "consultation child did not join: {source}")
            }
            Self::Custody(source) => write!(
                formatter,
                "joined consultation did not reach custody: {source}"
            ),
        }
    }
}

impl std::error::Error for ConsultationChildJoinFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Admission(source) => Some(source),
            Self::Custody(source) => std::error::Error::source(source),
        }
    }
}

/// Join one protected child terminal state and commit the parent successor.
///
/// The child history is not replayed and the parent does not adopt the child's
/// result as its own terminal result. The exact result or diagnostic enters the
/// private consultation roster, then the existing observation/checkpoint
/// transaction commits that successor. Synthesis and stopping remain separate
/// admitted consultation transitions.
///
/// # Errors
/// Returns an admission failure before a physical candidate exists, or the
/// existing custody owner after candidate preparation.
#[allow(clippy::too_many_arguments)]
pub fn commit_recovered_consultation_child<
    'operation,
    AdmitEvidence,
    AdmitCollection,
    Rotation,
    ObservationSign,
    CheckpointSign,
>(
    recovered: &RecoveredConsultationCollection,
    input: ConsultationChildJoinInput<'operation>,
    checkpoint: ConsultationSuccessorCommitInput<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_evidence: AdmitEvidence,
    admit_collection: AdmitCollection,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<CompletedConsultationCollection, ConsultationChildJoinFailure>
where
    AdmitEvidence: FnOnce(
        &ConsultationRosterSlot,
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
    // The child inherits the parent's conflict domain. Its authenticated
    // current checkpoint is therefore the physical head that the parent join
    // must extend. The parent observation below remains the semantic
    // predecessor of the new parent observation.
    let current_checkpoint = input.child.current_checkpoint();
    let successor = recovered
        .admit_recovered_child_terminal(
            input.slot_id,
            input.child,
            input.recorded_at,
            input.limits,
            admit_evidence,
        )
        .map_err(ConsultationChildJoinFailure::Admission)?;
    let prepared = successor
        .prepare_observation(
            input.parent,
            current_checkpoint,
            input.parent_observation,
            ConsultationCollectionObservationInput {
                observation_ref: input.observation_ref,
                predecessor_collection: Some(ConsultationCollectionPredecessor::recovered(
                    recovered,
                )),
                observed_at: input.observed_at,
                signing: input.signing,
            },
            input.limits,
            resolve_store,
            custody,
            admit_collection,
            admit_rotation,
            sign_observation,
        )
        .map_err(ConsultationChildJoinFailure::Admission)?;
    commit_consultation_collection(
        prepared,
        ConsultationCollectionCommitInput {
            accumulator_limits: checkpoint.accumulator_limits,
            predecessor: current_checkpoint,
            identity: checkpoint.identity,
            issued_at: checkpoint.issued_at,
            schema: checkpoint.schema,
            authority: checkpoint.authority,
        },
        resolve_store,
        custody,
        anchor,
        sign_checkpoint,
    )
    .map_err(ConsultationChildJoinFailure::Custody)
}

/// Inputs that retain one admitted disagreement in a recovered consultation.
pub struct ConsultationDisagreementCommitInput<'a> {
    /// Distinct fulfilled slots whose evidence disagrees.
    pub slots: Vec<ConsultationSlotId>,
    /// Exact admitted evidence establishing the disagreement.
    pub evidence: ImmutableRecordReferenceV1,
    /// Time at which the collection records the disagreement.
    pub recorded_at: UnixTimeV1,
    /// Parent Consult operation that owns the recovered collection.
    pub parent: &'a IssuedLocalOperation,
    /// Current parent observation named by the recovered collection frontier.
    pub parent_observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    /// Current parent checkpoint used as the next custody predecessor.
    pub parent_checkpoint: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    /// Fresh parent observation reference for the collection successor.
    pub observation_ref: ResidentRecordRefV1,
    /// Time at which the parent observes the disagreement successor.
    pub observed_at: UnixTimeV1,
    /// Current admitted signer for the parent observation.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Count and byte ceilings for the complete collection successor.
    pub limits: ConsultationCollectionLimits,
}

/// Phase-typed failure while retaining one consultation disagreement.
pub enum ConsultationDisagreementCommitFailure {
    /// The disagreement, collection successor, or parent observation did not admit.
    Admission(ConsultationCollectionError),
    /// The exact prepared parent successor stopped in physical custody.
    Custody(ConsultationCollectionCommitFailure),
}

impl std::fmt::Debug for ConsultationDisagreementCommitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Admission(_) => "Admission",
            Self::Custody(_) => "Custody",
        };
        formatter
            .debug_struct("ConsultationDisagreementCommitFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for ConsultationDisagreementCommitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admission(source) => {
                write!(
                    formatter,
                    "consultation disagreement did not admit: {source}"
                )
            }
            Self::Custody(source) => write!(
                formatter,
                "consultation disagreement did not reach custody: {source}"
            ),
        }
    }
}

impl std::error::Error for ConsultationDisagreementCommitFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Admission(source) => Some(source),
            Self::Custody(source) => std::error::Error::source(source),
        }
    }
}

/// Retain one admitted disagreement and commit the parent successor.
///
/// The disagreement keeps its attributable fulfilled slots and exact evidence.
/// It does not vote, rank, synthesize, stop collection, or change the parent
/// operation's responsibility. The existing observation/checkpoint transaction
/// commits the private roster successor.
///
/// # Errors
/// Returns an admission failure before a physical candidate exists, or the
/// existing custody owner after candidate preparation.
#[allow(clippy::too_many_arguments)]
pub fn commit_recovered_consultation_disagreement<
    'operation,
    AdmitDisagreement,
    AdmitCollection,
    Rotation,
    ObservationSign,
    CheckpointSign,
>(
    recovered: &RecoveredConsultationCollection,
    input: ConsultationDisagreementCommitInput<'operation>,
    checkpoint: ConsultationSuccessorCommitInput<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_disagreement: AdmitDisagreement,
    admit_collection: AdmitCollection,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<CompletedConsultationCollection, ConsultationDisagreementCommitFailure>
where
    AdmitDisagreement: FnOnce(
        &[ConsultationCollectionSlot],
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
    let successor = recovered
        .record_disagreement(
            input.slots,
            input.evidence,
            input.recorded_at,
            input.limits,
            admit_disagreement,
        )
        .map_err(ConsultationDisagreementCommitFailure::Admission)?;
    let prepared = successor
        .prepare_observation(
            input.parent,
            input.parent_checkpoint,
            input.parent_observation,
            ConsultationCollectionObservationInput {
                observation_ref: input.observation_ref,
                predecessor_collection: Some(ConsultationCollectionPredecessor::recovered(
                    recovered,
                )),
                observed_at: input.observed_at,
                signing: input.signing,
            },
            input.limits,
            resolve_store,
            custody,
            admit_collection,
            admit_rotation,
            sign_observation,
        )
        .map_err(ConsultationDisagreementCommitFailure::Admission)?;
    commit_consultation_collection(
        prepared,
        ConsultationCollectionCommitInput {
            accumulator_limits: checkpoint.accumulator_limits,
            predecessor: input.parent_checkpoint,
            identity: checkpoint.identity,
            issued_at: checkpoint.issued_at,
            schema: checkpoint.schema,
            authority: checkpoint.authority,
        },
        resolve_store,
        custody,
        anchor,
        sign_checkpoint,
    )
    .map_err(ConsultationDisagreementCommitFailure::Custody)
}

/// Fresh identities and bounds for one consultation cancellation request.
pub struct ConsultationCancellationInput<'a> {
    /// Fresh immutable identity for the admitted signed request.
    pub request_ref: ResidentRecordRefV1,
    /// Fresh immutable identity for the nonterminal parent observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Current admitted runtime signer for the parent observation.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Count and byte ceilings for the stopped collection.
    pub limits: ConsultationCollectionLimits,
}

/// Phase-typed failure while requesting cancellation of a consultation.
pub enum ConsultationCancellationFailure {
    /// Current cancellation authority or request projection did not admit.
    Authorization(OperationCancellationRequestError),
    /// The collection stop or parent observation did not admit.
    Admission(ConsultationCollectionError),
    /// The atomic request, collection, observation, and checkpoint commit stopped.
    Custody(ConsultationCollectionCommitFailure),
}

impl std::fmt::Debug for ConsultationCancellationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Authorization(_) => "Authorization",
            Self::Admission(_) => "Admission",
            Self::Custody(_) => "Custody",
        };
        formatter
            .debug_struct("ConsultationCancellationFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for ConsultationCancellationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Authorization(source) => {
                write!(
                    formatter,
                    "consultation cancellation was not authorized: {source}"
                )
            }
            Self::Admission(source) => {
                write!(
                    formatter,
                    "consultation cancellation did not admit: {source}"
                )
            }
            Self::Custody(source) => write!(
                formatter,
                "consultation cancellation did not reach custody: {source}"
            ),
        }
    }
}

impl std::error::Error for ConsultationCancellationFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Authorization(source) => Some(source),
            Self::Admission(source) => Some(source),
            Self::Custody(source) => std::error::Error::source(source),
        }
    }
}

/// Freeze an open consultation and durably request parent cancellation.
///
/// The exact signed request, stopped private roster, nonterminal
/// `CancellationRequested` observation, and checkpoint commit atomically. The
/// function does not contact any child, claim terminal cancellation, discard
/// missing coverage, or prevent later child evidence from entering the stopped
/// roster during retention.
///
/// # Errors
/// Returns an authorization failure before semantic preparation, an admission
/// failure before a physical candidate exists, or the existing custody owner
/// after candidate preparation.
#[allow(clippy::too_many_arguments)]
pub fn commit_recovered_consultation_cancellation<
    'authority,
    AdmitStop,
    AdmitCollection,
    Rotation,
    ObservationSign,
    CheckpointSign,
>(
    authorized: &AuthorizedOperationCancellation<'authority>,
    recovered: &RecoveredConsultationCollection,
    input: ConsultationCancellationInput<'_>,
    checkpoint: ConsultationSuccessorCommitInput<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_stop: AdmitStop,
    admit_collection: AdmitCollection,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<CompletedConsultationCollection, ConsultationCancellationFailure>
where
    AdmitStop: FnOnce(
        &ConsultationCollection,
        &ConsultationRequirementSummary,
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
    if !matches!(
        authorized.disposition(),
        OperationCancellationDisposition::CollectingConsultation { .. }
    ) {
        return Err(ConsultationCancellationFailure::Authorization(
            OperationCancellationRequestError::Routing,
        ));
    }
    let request_record = authorized
        .project_request_record(&input.request_ref, &input.observation_ref)
        .map_err(ConsultationCancellationFailure::Authorization)?;
    let request = ImmutableRecordReferenceV1 {
        record_ref: input.request_ref,
        record_sha256: ArtifactDigest::parse(
            authorized.request().canonical_request_sha256().as_str(),
        )
        .map_err(|_| {
            ConsultationCancellationFailure::Authorization(
                OperationCancellationRequestError::Association,
            )
        })?,
    };
    let observed_at = authorized.authorized_at();
    let successor = recovered
        .stop(
            ConsultationStopCause::Cancellation,
            request.clone(),
            observed_at,
            input.limits,
            admit_stop,
        )
        .map_err(ConsultationCancellationFailure::Admission)?;
    let operation = authorized.recovered_operation().operation();
    let observation = authorized.recovered_observation().selected_head();
    let predecessor_checkpoint = authorized.recovered_observation().current_checkpoint();
    let prepared = successor
        .prepare_cancellation_observation(
            operation,
            predecessor_checkpoint,
            observation,
            ConsultationCollectionObservationInput {
                observation_ref: input.observation_ref,
                predecessor_collection: Some(ConsultationCollectionPredecessor::recovered(
                    recovered,
                )),
                observed_at,
                signing: input.signing,
            },
            request,
            request_record,
            input.limits,
            resolve_store,
            custody,
            admit_collection,
            admit_rotation,
            sign_observation,
        )
        .map_err(ConsultationCancellationFailure::Admission)?;
    commit_consultation_collection(
        prepared,
        ConsultationCollectionCommitInput {
            accumulator_limits: checkpoint.accumulator_limits,
            predecessor: predecessor_checkpoint,
            identity: checkpoint.identity,
            issued_at: checkpoint.issued_at,
            schema: checkpoint.schema,
            authority: checkpoint.authority,
        },
        resolve_store,
        custody,
        anchor,
        sign_checkpoint,
    )
    .map_err(ConsultationCancellationFailure::Custody)
}

/// Inputs that stop one recovered consultation under its selected policy.
#[derive(Clone)]
pub struct ConsultationStopInput<'a> {
    /// Mechanical reason that collection ends.
    pub cause: ConsultationStopCause,
    /// Exact policy or boundary evidence supporting the stop.
    pub reason: ImmutableRecordReferenceV1,
    /// Time at which collection stops.
    pub stopped_at: UnixTimeV1,
    /// Parent Consult operation that owns the recovered collection.
    pub parent: &'a IssuedLocalOperation,
    /// Current parent observation named by the recovered collection frontier.
    pub parent_observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    /// Current parent checkpoint used as the next custody predecessor.
    pub parent_checkpoint: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    /// Fresh parent observation reference for the stopped collection.
    pub observation_ref: ResidentRecordRefV1,
    /// Time at which the parent observes the stopped collection.
    pub observed_at: UnixTimeV1,
    /// Current admitted signer for the parent observation.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Count and byte ceilings for the stopped collection.
    pub limits: ConsultationCollectionLimits,
}

/// Phase-typed failure while stopping one recovered consultation.
pub enum ConsultationStopFailure {
    /// The stop policy, boundary, or parent observation did not admit.
    Admission(ConsultationCollectionError),
    /// The exact stopped successor did not complete physical custody.
    Custody(ConsultationCollectionCommitFailure),
}

impl std::fmt::Debug for ConsultationStopFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Admission(_) => "Admission",
            Self::Custody(_) => "Custody",
        };
        formatter
            .debug_struct("ConsultationStopFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for ConsultationStopFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admission(source) => write!(formatter, "consultation did not stop: {source}"),
            Self::Custody(source) => {
                write!(
                    formatter,
                    "stopped consultation did not reach custody: {source}"
                )
            }
        }
    }
}

impl std::error::Error for ConsultationStopFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Admission(source) => Some(source),
            Self::Custody(source) => std::error::Error::source(source),
        }
    }
}

/// Stop one recovered collection and commit its exact frozen result.
///
/// The caller-selected policy callback decides whether this stop is permitted.
/// The application computes and freezes missing coverage, signs the parent
/// observation, and commits the collection frontier and checkpoint together.
/// This function does not synthesize, accept, settle, or cancel child work.
///
/// # Errors
/// Returns an admission failure before physical preparation, or the existing
/// custody owner after a checkpoint candidate can exist.
#[allow(clippy::too_many_arguments)]
pub fn commit_recovered_consultation_stop<
    'operation,
    AdmitStop,
    AdmitCollection,
    Rotation,
    ObservationSign,
    CheckpointSign,
>(
    recovered: &RecoveredConsultationCollection,
    input: ConsultationStopInput<'operation>,
    checkpoint: ConsultationSuccessorCommitInput<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_stop: AdmitStop,
    admit_collection: AdmitCollection,
    admit_rotation: Rotation,
    sign_observation: ObservationSign,
    sign_checkpoint: CheckpointSign,
) -> Result<CompletedConsultationCollection, ConsultationStopFailure>
where
    AdmitStop: FnOnce(
        &ConsultationCollection,
        &ConsultationRequirementSummary,
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
    let successor = recovered
        .stop(
            input.cause,
            input.reason,
            input.stopped_at,
            input.limits,
            admit_stop,
        )
        .map_err(ConsultationStopFailure::Admission)?;
    let prepared = successor
        .prepare_observation(
            input.parent,
            input.parent_checkpoint,
            input.parent_observation,
            ConsultationCollectionObservationInput {
                observation_ref: input.observation_ref,
                predecessor_collection: Some(ConsultationCollectionPredecessor::recovered(
                    recovered,
                )),
                observed_at: input.observed_at,
                signing: input.signing,
            },
            input.limits,
            resolve_store,
            custody,
            admit_collection,
            admit_rotation,
            sign_observation,
        )
        .map_err(ConsultationStopFailure::Admission)?;
    commit_consultation_collection(
        prepared,
        ConsultationCollectionCommitInput {
            accumulator_limits: checkpoint.accumulator_limits,
            predecessor: input.parent_checkpoint,
            identity: checkpoint.identity,
            issued_at: checkpoint.issued_at,
            schema: checkpoint.schema,
            authority: checkpoint.authority,
        },
        resolve_store,
        custody,
        anchor,
        sign_checkpoint,
    )
    .map_err(ConsultationStopFailure::Custody)
}

/// Complete inputs for committing one signed collection observation.
pub struct ConsultationCollectionCommitInput<'a> {
    /// Native work and byte ceilings for the successor accumulator.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Exact checkpoint that precedes this collection observation.
    pub predecessor: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    /// Fresh immutable identity for the successor checkpoint.
    pub identity: OperationCheckpointInput,
    /// Time at which the successor checkpoint is issued.
    pub issued_at: UnixTimeV1,
    /// Admitted schema identity for the successor checkpoint.
    pub schema: &'a SchemaDigest,
    /// Current authority admitted to sign the successor checkpoint.
    pub authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
}

/// Committed collection observation with its semantic and physical evidence.
pub struct CompletedConsultationCollection {
    prepared: PreparedConsultationCollection,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: CompletedOperationObservationCustody,
}

impl CompletedConsultationCollection {
    /// Borrow the signed collection observation that reached custody.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.prepared.observation()
    }

    /// Borrow the private roster journal committed with the observation.
    #[must_use]
    pub const fn journal(&self) -> &OperationCollectionJournal {
        self.prepared.journal()
    }

    /// Borrow the checkpoint that committed the collection observation.
    #[must_use]
    pub fn checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    /// Borrow the exact database and independent-anchor custody evidence.
    #[must_use]
    pub const fn custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }

    /// Consume the committed collection after physical custody completes.
    ///
    /// The returned values own the exact database, anchor, and checkpoint
    /// evidence. They retain no recovered-operation borrow.
    pub(in crate::resident) fn into_custody(
        self,
    ) -> (
        CompletedOperationObservationCustody,
        Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    ) {
        (self.custody, Box::new(self.checkpoint))
    }
}

impl std::fmt::Debug for CompletedConsultationCollection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CompletedConsultationCollection")
            .field("checkpoint_ref", &self.checkpoint.0)
            .field("database_evidence", &self.custody.database_evidence())
            .field("anchor_evidence", &self.custody.anchor_evidence())
            .finish_non_exhaustive()
    }
}

/// Stopped collection commit with the signed observation retained.
#[must_use = "resume or explicitly retain the exact consultation collection custody state"]
pub enum ConsultationCollectionCommitFailure {
    /// Accumulator preparation failed before a checkpoint was issued.
    Preparation {
        /// Original signed observation and private roster journal.
        prepared: Box<PreparedConsultationCollection>,
        /// Exact preparation rejection.
        reason: ConsultationCollectionError,
    },
    /// Checkpoint issuance or journal attachment failed before a write.
    Issue {
        /// Original signed observation and private roster journal.
        prepared: Box<PreparedConsultationCollection>,
        /// Exact issuance rejection.
        reason: ConsultationCollectionError,
    },
    /// Database custody did not complete.
    Commit {
        /// Original signed observation and private roster journal.
        prepared: Box<PreparedConsultationCollection>,
        /// Exact issued checkpoint; it is not proof that the database committed.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        /// Failure owner retaining the exact physical candidate.
        failure: OperationObservationCustodyCommitFailure,
    },
    /// Database custody exists, but the independent anchor did not complete.
    Anchor {
        /// Original signed observation and private roster journal.
        prepared: Box<PreparedConsultationCollection>,
        /// Exact checkpoint retained by the database transaction.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        /// Failure owner retaining the database-proven pending phase.
        failure: OperationObservationCustodyAnchorFailure,
    },
}

impl ConsultationCollectionCommitFailure {
    /// Continue only the exact retained database or anchor custody candidate.
    ///
    /// Preparation and issuance failures remain unchanged because no physical
    /// candidate exists. An unresolved database commit receives exact read-back
    /// before a proven-safe retry of the same candidate. The method never rebuilds
    /// or re-signs a collection observation or checkpoint.
    ///
    /// # Errors
    ///
    /// Returns the unchanged phase owner when custody cannot advance safely.
    pub(in crate::resident) fn continue_custody(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedConsultationCollection, Self> {
        let (prepared, checkpoint, pending) = match self {
            Self::Preparation { .. } | Self::Issue { .. } => return Err(self),
            Self::Commit {
                prepared,
                checkpoint,
                failure,
            } => {
                let pending = match failure.reconcile_exact(custody) {
                    Ok(pending) => pending,
                    Err(failure) => match failure.try_into_retryable() {
                        Ok(candidate) => match candidate.commit(custody, anchor) {
                            Ok(pending) => pending,
                            Err(failure) => {
                                return Err(Self::Commit {
                                    prepared,
                                    checkpoint,
                                    failure,
                                });
                            }
                        },
                        Err(failure) => {
                            return Err(Self::Commit {
                                prepared,
                                checkpoint,
                                failure,
                            });
                        }
                    },
                };
                (prepared, checkpoint, pending)
            }
            Self::Anchor {
                prepared,
                checkpoint,
                failure,
            } => {
                let pending = match failure.try_into_retryable() {
                    Ok(pending) => pending,
                    Err(failure) => {
                        return Err(Self::Anchor {
                            prepared,
                            checkpoint,
                            failure,
                        });
                    }
                };
                (prepared, checkpoint, pending)
            }
        };
        match pending.advance_anchor(anchor) {
            Ok(custody) => Ok(CompletedConsultationCollection {
                prepared: *prepared,
                checkpoint: *checkpoint,
                custody,
            }),
            Err(failure) => Err(Self::Anchor {
                prepared,
                checkpoint,
                failure,
            }),
        }
    }
}

impl std::fmt::Debug for ConsultationCollectionCommitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = formatter.debug_struct("ConsultationCollectionCommitFailure");
        match self {
            Self::Preparation { reason, .. } => {
                debug.field("phase", &"Preparation").field("reason", reason)
            }
            Self::Issue { reason, .. } => debug.field("phase", &"Issue").field("reason", reason),
            Self::Commit { .. } => debug.field("phase", &"Commit"),
            Self::Anchor { .. } => debug.field("phase", &"Anchor"),
        };
        debug.finish_non_exhaustive()
    }
}

impl std::fmt::Display for ConsultationCollectionCommitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Preparation { reason, .. } => write!(
                formatter,
                "consultation collection preparation did not complete: {reason}"
            ),
            Self::Issue { reason, .. } => write!(
                formatter,
                "consultation collection checkpoint could not be issued: {reason}"
            ),
            Self::Commit { failure, .. } => write!(
                formatter,
                "consultation collection database custody did not complete: {failure}"
            ),
            Self::Anchor { failure, .. } => write!(
                formatter,
                "consultation collection anti-rollback anchor did not complete: {failure}"
            ),
        }
    }
}

impl std::error::Error for ConsultationCollectionCommitFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Preparation { reason, .. } => Some(reason),
            Self::Issue { reason, .. } => Some(reason),
            Self::Commit { failure, .. } => Some(failure),
            Self::Anchor { failure, .. } => Some(failure),
        }
    }
}

/// Prepare roots, issue the checkpoint, and complete physical collection custody.
///
/// Failures before checkpoint issuance return the semantic preparation. Once
/// physical custody can have changed, the failure retains the exact database or
/// anchor recovery owner together with the immutable collection and checkpoint
/// material needed to report completion. This function performs no child
/// dispatch, synthesis, publication, or native-owner effect.
///
/// # Errors
///
/// Returns a phase-typed failure. Only pre-write failures return the original
/// prepared collection; custody failures require their retained recovery path.
pub fn commit_consultation_collection<Sign>(
    prepared: PreparedConsultationCollection,
    input: ConsultationCollectionCommitInput<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    sign: Sign,
) -> Result<CompletedConsultationCollection, ConsultationCollectionCommitFailure>
where
    Sign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let checkpoint =
        match prepared.prepare_checkpoint(input.accumulator_limits, resolve_store, custody) {
            Ok(checkpoint) => checkpoint,
            Err(reason) => {
                return Err(ConsultationCollectionCommitFailure::Preparation {
                    prepared: Box::new(prepared),
                    reason,
                });
            }
        };
    let preimage = match observation_checkpoint_preimage(
        checkpoint.prepared(),
        input.predecessor,
        &input.identity,
        input.issued_at,
        input.schema,
        input.authority,
    )
    .map_err(ConsultationCollectionError::from)
    {
        Ok(preimage) => preimage,
        Err(reason) => {
            drop(checkpoint);
            return Err(ConsultationCollectionCommitFailure::Issue {
                prepared: Box::new(prepared),
                reason,
            });
        }
    };
    let checkpoint_ref = input.identity.reference;
    let (physical, checkpoint_value) = match checkpoint.issue_checkpoint(
        &checkpoint_ref,
        preimage,
        input.schema,
        input.authority,
        sign,
    ) {
        Ok(issued) => issued,
        Err(reason) => {
            return Err(ConsultationCollectionCommitFailure::Issue {
                prepared: Box::new(prepared),
                reason,
            });
        }
    };
    let checkpoint = (checkpoint_ref, checkpoint_value);
    let pending = match physical.commit(custody, anchor) {
        Ok(pending) => pending,
        Err(failure) => {
            return Err(ConsultationCollectionCommitFailure::Commit {
                prepared: Box::new(prepared),
                checkpoint: Box::new(checkpoint),
                failure,
            });
        }
    };
    let custody = match pending.advance_anchor(anchor) {
        Ok(completed) => completed,
        Err(failure) => {
            return Err(ConsultationCollectionCommitFailure::Anchor {
                prepared: Box::new(prepared),
                checkpoint: Box::new(checkpoint),
                failure,
            });
        }
    };
    Ok(CompletedConsultationCollection {
        prepared,
        checkpoint,
        custody,
    })
}
