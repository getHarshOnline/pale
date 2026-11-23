//! Bounded restart-safe projection of committed local operation observations.
//!
//! The worker scans one frozen storage prefix in transaction order. It retains
//! the current source in memory until every publication stage exists, so a
//! one-stage commit cannot accidentally move the cursor past incomplete work.
//! Losing the attempt only causes a bounded rescan from the beginning; exact

//! replay cannot create another continuation.

use pale_core::{canon, records::*};
use pale_ports::{
    AccumulatorMembershipLimits, AuthorityStoreCoordinate, ControlRecordCoordinate,
    ControlRecordDigest, EventCustodyPort, OperationCustodyPort, OperationPublicationCursor,
    OperationPublicationReadError, OperationPublicationSource, OperationPublicationSourceKind,
    MAX_OPERATION_PUBLICATION_BODY_BYTES,
};

use super::local_operation_projection::admit_recovered_local_operation_publication_plan;
use super::local_operation_publication::read_a2a_task_predecessor_publication;
use super::local_operation_publication::read_local_operation_predecessor_publication;
use super::{
    admit_a2a_task_publication_plan, advance_a2a_task_publication,
    advance_local_operation_publication, A2aTaskPublicationReferences, EventCustodyCommitEvidence,
    EventPublicationRaceVerifier, EventPublicationState, LocalOperationPublicationAdvance,
    LocalOperationPublicationAdvanceError, LocalOperationPublicationIssuance,
    LocalOperationPublicationReadmission, LocalOperationPublicationReferences,
    LocalOperationPublicationSigningContexts, LocalOperationPublicationStage,
};
use crate::resident::{
    A2aTaskRecoveryLimits, IssuedLocalOperation, OperationCheckpointAncestryLimits,
    OperationObservationRecoveryLimits, RecoveredA2aTaskHistory, RecoveredOperationControl,
    ResidentApplication, ResidentCommitReceipt,
};

/// Aggregate ceilings for one process-local traversal attempt.
#[derive(Debug, Clone, Copy)]
pub struct LocalOperationPublicationSweepLimits {
    maximum_pages: usize,
    maximum_sources: usize,
    maximum_body_bytes: usize,
    maximum_publication_bytes: usize,
}

impl LocalOperationPublicationSweepLimits {
    /// Retain positive aggregate and per-publication ceilings.
    ///
    /// # Errors
    /// Rejects any zero ceiling. No default capacity is inferred.
    pub fn new(
        maximum_pages: usize,
        maximum_sources: usize,
        maximum_body_bytes: usize,
        maximum_publication_bytes: usize,
    ) -> Result<Self, LocalOperationPublicationSweepError> {
        if [
            maximum_pages,
            maximum_sources,
            maximum_body_bytes,
            maximum_publication_bytes,
        ]
        .contains(&0)
        {
            return Err(LocalOperationPublicationSweepError::Bounds);
        }
        Ok(Self {
            maximum_pages,
            maximum_sources,
            maximum_body_bytes,
            maximum_publication_bytes,
        })
    }
}

#[derive(Debug)]
struct PendingPublicationSource {
    source: OperationPublicationSource,
    continuation: Option<OperationPublicationCursor>,
}

#[derive(Debug)]
enum PublicationSweepPosition {
    BeforeFirst,
    Pending(Box<PendingPublicationSource>),
    BetweenSources(OperationPublicationCursor),
    Exhausted,
}

/// Process-local traversal state for one exact immutable storage prefix.
///
/// This is not a durable cursor. A process restart begins another bounded scan
/// and safely readmits existing stages. The pending source is retained so the
/// cursor advances only after that source is fully projected.
#[derive(Debug)]
pub struct LocalOperationPublicationSweepAttempt {
    store: AuthorityStoreCoordinate,
    through: Option<ControlTransactionSequenceV1>,
    position: PublicationSweepPosition,
    pages_examined: usize,
    sources_examined: usize,
    source_body_bytes_examined: usize,
}

impl LocalOperationPublicationSweepAttempt {
    /// Begin at the first committed source in one exact store incarnation.
    #[must_use]
    pub fn new(store: AuthorityStoreCoordinate) -> Self {
        Self {
            store,
            through: None,
            position: PublicationSweepPosition::BeforeFirst,
            pages_examined: 0,
            sources_examined: 0,
            source_body_bytes_examined: 0,
        }
    }

    /// Borrow the exact store selected for this attempt.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the number of physical source pages read so far.
    #[must_use]
    pub const fn pages_examined(&self) -> usize {
        self.pages_examined
    }

    /// Return the number of source pairs retained from those pages.
    #[must_use]
    pub const fn sources_examined(&self) -> usize {
        self.sources_examined
    }

    /// Return the combined source-body bytes charged to this attempt.
    #[must_use]
    pub const fn source_body_bytes_examined(&self) -> usize {
        self.source_body_bytes_examined
    }
}

/// Ceilings that the exact-source recovery callback must apply.
///
/// A current and, for successors, one predecessor observation may each consume
/// these ceilings. Source enumeration bytes are counted separately by the
/// sweep attempt.
#[derive(Debug, Clone, Copy)]
pub struct LocalOperationPublicationRecoveryLimits {
    maximum_parent_depth: usize,
    observation: OperationObservationRecoveryLimits,
    checkpoint: OperationCheckpointAncestryLimits,
    membership: AccumulatorMembershipLimits,
}

/// Recovery ceilings applied to one exact A2A publication source.
#[derive(Debug, Clone, Copy)]
pub struct A2aTaskPublicationRecoveryLimits {
    local: LocalOperationPublicationRecoveryLimits,
    remote: A2aTaskRecoveryLimits,
}

impl A2aTaskPublicationRecoveryLimits {
    /// Combine the local operation and remote task-history ceilings.
    #[must_use]
    pub const fn new(
        local: LocalOperationPublicationRecoveryLimits,
        remote: A2aTaskRecoveryLimits,
    ) -> Self {
        Self { local, remote }
    }

    /// Return the existing local-operation recovery ceilings.
    #[must_use]
    pub const fn local(&self) -> LocalOperationPublicationRecoveryLimits {
        self.local
    }

    /// Return the exact remote history and protocol-evidence ceilings.
    #[must_use]
    pub const fn remote(&self) -> A2aTaskRecoveryLimits {
        self.remote
    }
}

impl LocalOperationPublicationRecoveryLimits {
    /// Retain the exact existing protected-recovery ceilings.
    #[must_use]
    pub const fn new(
        maximum_parent_depth: usize,
        observation: OperationObservationRecoveryLimits,
        checkpoint: OperationCheckpointAncestryLimits,
        membership: AccumulatorMembershipLimits,
    ) -> Self {
        Self {
            maximum_parent_depth,
            observation,
            checkpoint,
            membership,
        }
    }

    /// Maximum number of parent-operation edges recovered for this source.
    #[must_use]
    pub const fn maximum_parent_depth(&self) -> usize {
        self.maximum_parent_depth
    }

    /// Observation-history ceilings for each exact recovery.
    #[must_use]
    pub const fn observation(&self) -> OperationObservationRecoveryLimits {
        self.observation
    }

    /// Checkpoint-ancestry ceilings for each exact recovery.
    #[must_use]
    pub const fn checkpoint(&self) -> OperationCheckpointAncestryLimits {
        self.checkpoint
    }

    /// Accumulator membership ceilings for each exact recovery.
    #[must_use]
    pub const fn membership(&self) -> AccumulatorMembershipLimits {
        self.membership
    }
}

/// Exact source coordinates that an authority resolver must recover.
///
/// The values are lookup inputs only. The callback must return the canonical
/// protected recovery product; callers cannot construct that product directly.
pub struct LocalOperationPublicationRecoveryRequest<'a> {
    store: &'a AuthorityStoreCoordinate,
    operation_handle: ControlRecordCoordinate,
    observation: ControlRecordCoordinate,
    limits: LocalOperationPublicationRecoveryLimits,
}

/// Exact source coordinates that A2A publication recovery must resolve.
pub struct A2aTaskPublicationRecoveryRequest<'a> {
    store: &'a AuthorityStoreCoordinate,
    task_binding: ControlRecordCoordinate,
    observation: ControlRecordCoordinate,
    checkpoint: Option<ControlRecordCoordinate>,
    limits: A2aTaskPublicationRecoveryLimits,
}

impl A2aTaskPublicationRecoveryRequest<'_> {
    /// Borrow the selected store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        self.store
    }

    /// Borrow the exact immutable task-binding coordinate.
    #[must_use]
    pub const fn task_binding(&self) -> &ControlRecordCoordinate {
        &self.task_binding
    }

    /// Borrow the exact remote-observation coordinate.
    #[must_use]
    pub const fn observation(&self) -> &ControlRecordCoordinate {
        &self.observation
    }

    /// Borrow the checkpoint physically paired with the source observation.
    #[must_use]
    pub const fn checkpoint(&self) -> Option<&ControlRecordCoordinate> {
        self.checkpoint.as_ref()
    }

    /// Return all recovery ceilings selected for this source.
    #[must_use]
    pub const fn limits(&self) -> A2aTaskPublicationRecoveryLimits {
        self.limits
    }
}

/// Protected operation, remote history, and admitted source checkpoint.
pub struct RecoveredA2aTaskPublicationSource {
    operation: IssuedLocalOperation,
    history: RecoveredA2aTaskHistory,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
}

impl RecoveredA2aTaskPublicationSource {
    /// Retain already recovered evidence for projection admission.
    ///
    /// Construction does not publish, send, or grant disclosure. The projector
    /// still verifies every exact association against the physical source.
    #[must_use]
    pub fn new(
        operation: IssuedLocalOperation,
        history: RecoveredA2aTaskHistory,
        checkpoint_ref: ResidentRecordRefV1,
        checkpoint: ControlStateCheckpointV1,
    ) -> Self {
        Self {
            operation,
            history,
            checkpoint: (checkpoint_ref, checkpoint),
        }
    }

    const fn operation(&self) -> &IssuedLocalOperation {
        &self.operation
    }

    const fn history(&self) -> &RecoveredA2aTaskHistory {
        &self.history
    }

    const fn checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }
}

impl LocalOperationPublicationRecoveryRequest<'_> {
    /// Borrow the selected store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        self.store
    }

    /// Borrow the exact immutable operation-handle coordinate.
    #[must_use]
    pub const fn operation_handle(&self) -> &ControlRecordCoordinate {
        &self.operation_handle
    }

    /// Borrow the exact observation member to recover.
    #[must_use]
    pub const fn observation(&self) -> &ControlRecordCoordinate {
        &self.observation
    }

    /// Return the ceilings that exact protected recovery must enforce.
    #[must_use]
    pub const fn limits(&self) -> LocalOperationPublicationRecoveryLimits {
        self.limits
    }
}

/// Current producer and signer evidence used only for a stage proved absent.
pub struct LocalOperationPublicationCurrent<'a> {
    /// Current producer-allocation record reference.
    pub producer_allocation_ref: &'a ResidentRecordRefV1,
    /// Admitted current producer incarnation.
    pub producer: &'a ProducerIncarnationV1,
    /// Current writer-fence record reference.
    pub writer_fence_ref: &'a ResidentRecordRefV1,
    /// Admitted current writer fence.
    pub writer_fence: &'a PrincipalResidentWriterFenceV1,
    /// Whether the publication preceded a separately admitted retirement.
    pub publication_state: EventPublicationState<'a>,
    /// Cross-store race closure owned by producer publication.
    pub race_verifier: &'a dyn EventPublicationRaceVerifier,
    /// Current signer contexts for newly issued derived records.
    pub signing_contexts: LocalOperationPublicationSigningContexts<'a>,
}

/// Why exact protected recovery could not be supplied for one source.
#[derive(Debug, thiserror::Error)]
pub enum LocalOperationPublicationSourceRecoveryError {
    /// The exact historical authority or admission context is unavailable.
    #[error("local operation publication source recovery context is unavailable")]
    Unavailable,
    /// Canonical operation and observation recovery rejected the source.
    #[error(transparent)]
    Control(#[from] crate::resident::OperationControlRecoveryError),
}

/// Why exact protected A2A source recovery could not be supplied.
#[derive(Debug, thiserror::Error)]
pub enum A2aTaskPublicationSourceRecoveryError {
    /// The exact historical authority or admission context is unavailable.
    #[error("A2A task publication source recovery context is unavailable")]
    Unavailable,
    /// Protected A2A recovery rejected the source.
    #[error(transparent)]
    Task(#[from] crate::resident::A2aTaskRecoveryError),
}

/// Source-family revision established by one publication stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationPublicationRevision {
    /// Revision in the local operation-observation history.
    Operation(OperationObservationRevisionV1),
    /// Revision in the remote A2A task-observation history.
    A2aTask(A2aTaskObservationRevisionV1),
}

/// Observable result of one bounded worker call.
pub enum LocalOperationPublicationSweepProgress {
    /// One stage was established by a new commit or exact replay.
    StageEstablished {
        /// Exact store incarnation whose source was established.
        store: AuthorityStoreCoordinate,
        /// Transaction that inserted the source observation and checkpoint.
        source_transaction: ControlTransactionSequenceV1,
        /// Exact source observation reference.
        source_ref: ResidentRecordRefV1,
        /// Admitted handle-local observation revision.
        revision: OperationPublicationRevision,
        /// Stage established by this call.
        stage: LocalOperationPublicationStage,
        /// Whether the commit was new, exact replay, or recovered after ambiguity.
        evidence: EventCustodyCommitEvidence,
        /// Durable resident journal coordinate for the established stage.
        receipt: Box<ResidentCommitReceipt>,
    },
    /// The current source completed and scanning can continue.
    ContinueScan,
    /// Every supported source in the exact frozen prefix is complete.
    SupportedLocalPrefixComplete {
        /// Exact store incarnation whose prefix was inspected.
        store: AuthorityStoreCoordinate,
        /// Inclusive storage transaction frontier inspected by this attempt.
        through: ControlTransactionSequenceV1,
        /// Number of physical source pairs examined.
        sources_examined: usize,
    },
    /// The caller-selected aggregate attempt budget was exhausted.
    BudgetExhausted,
}

/// A bounded local publication sweep could not establish a safe next step.
#[derive(Debug, thiserror::Error)]
pub enum LocalOperationPublicationSweepError {
    /// One or more required ceilings are zero or arithmetic exceeded `usize`.
    #[error("local operation publication sweep bounds are invalid or exhausted")]
    Bounds,
    /// Storage could not return one complete bounded source page.
    #[error(transparent)]
    SourceRead(#[from] OperationPublicationReadError),
    /// Untrusted source bytes could not identify one exact recovery coordinate.
    #[error("local operation publication source coordinates are malformed")]
    InvalidSource,
    /// The traversal and resident application select different store incarnations.
    #[error("local operation publication sweep and resident select different stores")]
    Association,
    /// Protected historical recovery rejected the selected source.
    #[error(transparent)]
    Recovery(#[from] LocalOperationPublicationSourceRecoveryError),
    /// Protected remote-task recovery rejected the selected source.
    #[error(transparent)]
    A2aRecovery(#[from] A2aTaskPublicationSourceRecoveryError),
    /// A2A source projection rejected unlike evidence or identity.
    #[error(transparent)]
    A2aProjection(#[from] super::A2aTaskPublicationProjectionError),
    /// Publication projection or stage custody failed.
    #[error(transparent)]
    Advance(#[from] LocalOperationPublicationAdvanceError),
}

/// Advance at most one durable stage in one frozen local-source traversal.
///
/// The recovery callback must resolve authority for each exact handle and
/// observation rather than applying one current key to the whole store. The
/// returned recovery products can only be produced by protected application
/// recovery. Local and A2A sources enter the same publication stages; neither
/// remote state nor Event delivery establishes an operation result or effect.
///
/// # Errors
/// Rejects malformed source coordinates, unavailable historical recovery,
/// conflicting publication evidence, storage failure, or invalid ceilings.
#[allow(clippy::too_many_arguments)]
pub fn sweep_local_operation_publications<P, RecoverLocal, RecoverA2a>(
    attempt: &mut LocalOperationPublicationSweepAttempt,
    limits: LocalOperationPublicationSweepLimits,
    recovery_limits: LocalOperationPublicationRecoveryLimits,
    a2a_recovery_limits: A2aTaskPublicationRecoveryLimits,
    source_port: &mut dyn OperationCustodyPort,
    application: &mut ResidentApplication<'_>,
    event_port: &mut P,
    recover_local: &mut RecoverLocal,
    recover_a2a: &mut RecoverA2a,
    current: LocalOperationPublicationCurrent<'_>,
    issuance: &mut LocalOperationPublicationIssuance<'_>,
    readmission: &mut LocalOperationPublicationReadmission<'_>,
) -> Result<LocalOperationPublicationSweepProgress, LocalOperationPublicationSweepError>
where
    P: EventCustodyPort + pale_ports::ControlTransactionPort,
    RecoverLocal:
        FnMut(
            &LocalOperationPublicationRecoveryRequest<'_>,
        )
            -> Result<RecoveredOperationControl, LocalOperationPublicationSourceRecoveryError>,
    RecoverA2a:
        FnMut(
            &A2aTaskPublicationRecoveryRequest<'_>,
        )
            -> Result<RecoveredA2aTaskPublicationSource, A2aTaskPublicationSourceRecoveryError>,
{
    if attempt.store != *application.store() {
        return Err(LocalOperationPublicationSweepError::Association);
    }
    if !matches!(attempt.position, PublicationSweepPosition::Pending(_)) {
        match load_next_source(attempt, limits, source_port)? {
            LocalOperationPublicationSweepProgress::ContinueScan => {}
            terminal => return Ok(terminal),
        }
    }

    let PublicationSweepPosition::Pending(pending) = &attempt.position else {
        return Err(LocalOperationPublicationSweepError::InvalidSource);
    };
    let source_transaction = pending.source.transaction();
    let source_ref = pending
        .source
        .observation()
        .coordinate()
        .record_ref()
        .clone();
    let advanced = match pending.source.kind() {
        OperationPublicationSourceKind::OperationObservation => {
            let current_request =
                recovery_request(&attempt.store, &pending.source, recovery_limits)?;
            let recovered = recover_local(&current_request)?;
            let revision = OperationPublicationRevision::Operation(
                *recovered.observation().selected().1.revision(),
            );
            let predecessor = predecessor_publication(
                &attempt.store,
                &recovered,
                event_port,
                recover_local,
                readmission,
                recovery_limits,
                limits.maximum_publication_bytes,
            )?;
            let parent_start = parent_start_publication(
                &attempt.store,
                &recovered,
                event_port,
                readmission,
                limits.maximum_publication_bytes,
            )?;
            let references = LocalOperationPublicationReferences::derive_for_operation(
                &attempt.store,
                recovered.observation().selected().0,
                recovered.observation().selected().1,
                recovered.operation().operation().handle().1,
            )
            .map_err(LocalOperationPublicationAdvanceError::from)?;
            let plan = admit_recovered_local_operation_publication_plan(
                &attempt.store,
                &pending.source,
                recovered.observation(),
                recovered.operation(),
                &references,
                predecessor,
                parent_start,
            )
            .map_err(LocalOperationPublicationAdvanceError::from)?;
            let advance = advance_local_operation_publication(
                application,
                event_port,
                &plan,
                current.producer_allocation_ref,
                current.producer,
                current.writer_fence_ref,
                current.writer_fence,
                current.publication_state,
                current.race_verifier,
                current.signing_contexts,
                issuance,
                readmission,
                limits.maximum_publication_bytes,
            )?;
            (revision, advance)
        }
        OperationPublicationSourceKind::A2aTaskObservation => {
            let request =
                a2a_recovery_request(&attempt.store, &pending.source, a2a_recovery_limits)?;
            let recovered = recover_a2a(&request)?;
            let revision = OperationPublicationRevision::A2aTask(
                *recovered.history().selected_observation().1.revision(),
            );
            let predecessor = a2a_predecessor_publication(
                &attempt.store,
                &recovered,
                event_port,
                recover_a2a,
                readmission,
                a2a_recovery_limits,
                limits.maximum_publication_bytes,
            )?;
            let references = A2aTaskPublicationReferences::derive(
                &attempt.store,
                recovered.history().selected_observation().0,
                recovered.history().selected_observation().1,
            )?;
            let plan = admit_a2a_task_publication_plan(
                &attempt.store,
                &pending.source,
                recovered.history(),
                recovered.operation(),
                recovered.checkpoint(),
                &references,
                predecessor,
            )?;
            let advance = advance_a2a_task_publication(
                application,
                event_port,
                &plan,
                current.producer_allocation_ref,
                current.producer,
                current.writer_fence_ref,
                current.writer_fence,
                current.publication_state,
                current.race_verifier,
                current.signing_contexts,
                issuance,
                readmission,
                limits.maximum_publication_bytes,
            )?;
            (revision, advance)
        }
    };
    let (revision, advance) = advanced;
    match advance {
        LocalOperationPublicationAdvance::Established {
            stage,
            evidence,
            receipt,
        } => Ok(LocalOperationPublicationSweepProgress::StageEstablished {
            store: attempt.store.clone(),
            source_transaction,
            source_ref,
            revision,
            stage,
            evidence,
            receipt,
        }),
        LocalOperationPublicationAdvance::Complete => {
            finish_pending_source(attempt);
            completion_or_continue(attempt)
        }
    }
}

fn load_next_source(
    attempt: &mut LocalOperationPublicationSweepAttempt,
    limits: LocalOperationPublicationSweepLimits,
    port: &mut dyn OperationCustodyPort,
) -> Result<LocalOperationPublicationSweepProgress, LocalOperationPublicationSweepError> {
    if matches!(attempt.position, PublicationSweepPosition::Exhausted) {
        return completion_or_continue(attempt);
    }
    if attempt.pages_examined >= limits.maximum_pages
        || attempt.sources_examined >= limits.maximum_sources
        || attempt.source_body_bytes_examined >= limits.maximum_body_bytes
    {
        return Ok(LocalOperationPublicationSweepProgress::BudgetExhausted);
    }
    let remaining_bytes = limits
        .maximum_body_bytes
        .checked_sub(attempt.source_body_bytes_examined)
        .filter(|remaining| *remaining > 0)
        .ok_or(LocalOperationPublicationSweepError::Bounds)?;
    let page = match port.read_publication_sources(
        &attempt.store,
        match &attempt.position {
            PublicationSweepPosition::BeforeFirst => None,
            PublicationSweepPosition::BetweenSources(cursor) => Some(cursor),
            PublicationSweepPosition::Exhausted => return completion_or_continue(attempt),
            PublicationSweepPosition::Pending(_) => {
                return Err(LocalOperationPublicationSweepError::InvalidSource);
            }
        },
        1,
        remaining_bytes.min(MAX_OPERATION_PUBLICATION_BODY_BYTES),
    ) {
        Ok(page) => page,
        Err(OperationPublicationReadError::Limit) => {
            return Ok(LocalOperationPublicationSweepProgress::BudgetExhausted);
        }
        Err(error) => return Err(error.into()),
    };
    attempt.pages_examined = attempt
        .pages_examined
        .checked_add(1)
        .ok_or(LocalOperationPublicationSweepError::Bounds)?;
    match attempt.through {
        None => attempt.through = Some(page.through()),
        Some(through) if through == page.through() => {}
        Some(_) => return Err(LocalOperationPublicationSweepError::InvalidSource),
    }
    if page.sources.len() > 1 || (page.sources.is_empty() && page.continuation.is_some()) {
        return Err(LocalOperationPublicationSweepError::InvalidSource);
    }
    let Some(source) = page.sources.into_iter().next() else {
        attempt.position = PublicationSweepPosition::Exhausted;
        return completion_or_continue(attempt);
    };
    let source_bytes = source
        .observation()
        .canonical_record_jcs()
        .as_slice()
        .len()
        .checked_add(source.checkpoint().canonical_record_jcs().as_slice().len())
        .ok_or(LocalOperationPublicationSweepError::Bounds)?;
    attempt.source_body_bytes_examined = attempt
        .source_body_bytes_examined
        .checked_add(source_bytes)
        .filter(|total| *total <= limits.maximum_body_bytes)
        .ok_or(LocalOperationPublicationSweepError::Bounds)?;
    attempt.sources_examined = attempt
        .sources_examined
        .checked_add(1)
        .filter(|total| *total <= limits.maximum_sources)
        .ok_or(LocalOperationPublicationSweepError::Bounds)?;
    attempt.position = PublicationSweepPosition::Pending(Box::new(PendingPublicationSource {
        source,
        continuation: page.continuation,
    }));
    Ok(LocalOperationPublicationSweepProgress::ContinueScan)
}

fn finish_pending_source(attempt: &mut LocalOperationPublicationSweepAttempt) {
    let position = std::mem::replace(&mut attempt.position, PublicationSweepPosition::Exhausted);
    let PublicationSweepPosition::Pending(pending) = position else {
        panic!("a source is finished only after bounded loading")
    };
    attempt.position = pending.continuation.map_or(
        PublicationSweepPosition::Exhausted,
        PublicationSweepPosition::BetweenSources,
    );
}

fn completion_or_continue(
    attempt: &LocalOperationPublicationSweepAttempt,
) -> Result<LocalOperationPublicationSweepProgress, LocalOperationPublicationSweepError> {
    if !matches!(attempt.position, PublicationSweepPosition::Exhausted) {
        Ok(LocalOperationPublicationSweepProgress::ContinueScan)
    } else {
        Ok(
            LocalOperationPublicationSweepProgress::SupportedLocalPrefixComplete {
                store: attempt.store.clone(),
                through: attempt
                    .through
                    .ok_or(LocalOperationPublicationSweepError::InvalidSource)?,
                sources_examined: attempt.sources_examined,
            },
        )
    }
}

fn recovery_request<'a>(
    store: &'a AuthorityStoreCoordinate,
    source: &OperationPublicationSource,
    limits: LocalOperationPublicationRecoveryLimits,
) -> Result<LocalOperationPublicationRecoveryRequest<'a>, LocalOperationPublicationSweepError> {
    if source.kind() != OperationPublicationSourceKind::OperationObservation {
        return Err(LocalOperationPublicationSweepError::InvalidSource);
    }
    let value = canon::parse_strict(source.observation().canonical_record_jcs().as_slice())
        .map_err(|_| LocalOperationPublicationSweepError::InvalidSource)?;
    let wire: OperationObservationWireV1 = serde_json::from_value(value)
        .map_err(|_| LocalOperationPublicationSweepError::InvalidSource)?;
    let handle_digest = ControlRecordDigest::parse(wire.operation_handle_sha256().as_str())
        .map_err(|_| LocalOperationPublicationSweepError::InvalidSource)?;
    Ok(LocalOperationPublicationRecoveryRequest {
        store,
        operation_handle: ControlRecordCoordinate::new(
            wire.operation_handle_ref().clone(),
            handle_digest,
        ),
        observation: source.observation().coordinate().clone(),
        limits,
    })
}

fn a2a_recovery_request<'a>(
    store: &'a AuthorityStoreCoordinate,
    source: &OperationPublicationSource,
    limits: A2aTaskPublicationRecoveryLimits,
) -> Result<A2aTaskPublicationRecoveryRequest<'a>, LocalOperationPublicationSweepError> {
    if source.kind() != OperationPublicationSourceKind::A2aTaskObservation {
        return Err(LocalOperationPublicationSweepError::InvalidSource);
    }
    let value = canon::parse_strict(source.observation().canonical_record_jcs().as_slice())
        .map_err(|_| LocalOperationPublicationSweepError::InvalidSource)?;
    let wire: A2aTaskObservationWireV1 = serde_json::from_value(value)
        .map_err(|_| LocalOperationPublicationSweepError::InvalidSource)?;
    let binding_digest = ControlRecordDigest::parse(wire.task_binding_sha256().as_str())
        .map_err(|_| LocalOperationPublicationSweepError::InvalidSource)?;
    Ok(A2aTaskPublicationRecoveryRequest {
        store,
        task_binding: ControlRecordCoordinate::new(wire.task_binding_ref().clone(), binding_digest),
        observation: source.observation().coordinate().clone(),
        checkpoint: Some(source.checkpoint().coordinate().clone()),
        limits,
    })
}

fn a2a_predecessor_publication<RecoverA2a>(
    store: &AuthorityStoreCoordinate,
    recovered: &RecoveredA2aTaskPublicationSource,
    event_port: &mut dyn EventCustodyPort,
    recover: &mut RecoverA2a,
    readmission: &mut LocalOperationPublicationReadmission<'_>,
    recovery_limits: A2aTaskPublicationRecoveryLimits,
    maximum_publication_bytes: usize,
) -> Result<Option<super::A2aTaskPredecessorPublication>, LocalOperationPublicationSweepError>
where
    RecoverA2a:
        FnMut(
            &A2aTaskPublicationRecoveryRequest<'_>,
        )
            -> Result<RecoveredA2aTaskPublicationSource, A2aTaskPublicationSourceRecoveryError>,
{
    let observation = recovered.history().selected_observation().1;
    let (Some(predecessor_ref), Some(predecessor_digest)) = (
        observation.predecessor_ref().as_ref(),
        observation.predecessor_sha256().as_ref(),
    ) else {
        return Ok(None);
    };
    let (binding_ref, binding) = recovered.history().binding();
    let request = A2aTaskPublicationRecoveryRequest {
        store,
        task_binding: ControlRecordCoordinate::new(
            binding_ref.clone(),
            ControlRecordDigest::parse(binding.canonical_a2a_task_binding_sha256().as_str())
                .map_err(|_| LocalOperationPublicationSweepError::InvalidSource)?,
        ),
        observation: ControlRecordCoordinate::new(
            predecessor_ref.clone(),
            ControlRecordDigest::parse(predecessor_digest.as_str())
                .map_err(|_| LocalOperationPublicationSweepError::InvalidSource)?,
        ),
        checkpoint: None,
        limits: recovery_limits,
    };
    let predecessor = recover(&request)?;
    Ok(Some(read_a2a_task_predecessor_publication(
        event_port,
        store,
        predecessor.history(),
        predecessor.operation(),
        readmission,
        maximum_publication_bytes,
    )?))
}

fn predecessor_publication<Recover>(
    store: &AuthorityStoreCoordinate,
    recovered: &RecoveredOperationControl,
    event_port: &mut dyn EventCustodyPort,
    recover: &mut Recover,
    readmission: &mut LocalOperationPublicationReadmission<'_>,
    recovery_limits: LocalOperationPublicationRecoveryLimits,
    maximum_publication_bytes: usize,
) -> Result<Option<super::LocalOperationPredecessorPublication>, LocalOperationPublicationSweepError>
where
    Recover:
        FnMut(
            &LocalOperationPublicationRecoveryRequest<'_>,
        )
            -> Result<RecoveredOperationControl, LocalOperationPublicationSourceRecoveryError>,
{
    let observation = recovered.observation().selected().1;
    let (Some(predecessor_ref), Some(predecessor_digest)) = (
        observation.predecessor_ref().as_ref(),
        observation.predecessor_sha256().as_ref(),
    ) else {
        return Ok(None);
    };
    let predecessor_coordinate = ControlRecordCoordinate::new(
        predecessor_ref.clone(),
        ControlRecordDigest::parse(predecessor_digest.as_str())
            .map_err(|_| LocalOperationPublicationSweepError::InvalidSource)?,
    );
    let request = LocalOperationPublicationRecoveryRequest {
        store,
        operation_handle: recovered.observation().handle.clone(),
        observation: predecessor_coordinate,
        limits: recovery_limits,
    };
    let predecessor = recover(&request)?;
    read_local_operation_predecessor_publication(
        event_port,
        store,
        predecessor.observation(),
        recovered.operation(),
        readmission,
        maximum_publication_bytes,
    )
    .map(Some)
    .map_err(Into::into)
}

fn parent_start_publication(
    store: &AuthorityStoreCoordinate,
    recovered: &RecoveredOperationControl,
    event_port: &mut dyn EventCustodyPort,
    readmission: &mut LocalOperationPublicationReadmission<'_>,
    maximum_publication_bytes: usize,
) -> Result<Option<super::LocalOperationPredecessorPublication>, LocalOperationPublicationSweepError>
{
    let observation = recovered.observation().selected().1;
    if observation.revision().get() != 1
        || recovered
            .operation()
            .operation()
            .handle()
            .1
            .parent_operation()
            .is_none()
    {
        return Ok(None);
    }
    let parent = recovered
        .operation()
        .recovered_parent()
        .ok_or(LocalOperationPublicationSweepError::InvalidSource)?;
    let parent_start = parent
        .initial_observation()
        .ok_or(LocalOperationPublicationSweepError::InvalidSource)?;
    read_local_operation_predecessor_publication(
        event_port,
        store,
        parent_start,
        parent,
        readmission,
        maximum_publication_bytes,
    )
    .map(Some)
    .map_err(Into::into)
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/event_custody/local_operation_sweep.rs"]
mod tests;
