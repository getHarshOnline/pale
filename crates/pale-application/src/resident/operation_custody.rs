//! Phase-typed physical custody for durable operation transitions.
//!
//! The ordinary start-replay path resolves the request identity before
//! attempting its historical transaction. An observation append accepts only
//! an already-admitted closed physical candidate; semantic handle, predecessor,
//! state-evidence, signature, and authority-rotation admission happens before
//! this seam. A database commit is only one half of either custody proof. Even
//! after the independently retained hard-domain anchor proves the signed
//! checkpoint successor, this module exposes no operation handle, lifecycle
//! decision, or continuation authority.

use pale_core::{canon, records::CommonRequestEnvelopeV1};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorError, AntiRollbackAnchorObservation,
    AntiRollbackAnchorPort, AntiRollbackAnchorTransitionError, ControlDomainAntiRollbackAnchor,
    ControlRecord, ControlRecordError, ControlStoreError, ControlStoreErrorKind,
    OperationCustodyCandidate, OperationCustodyCommitOutcome, OperationCustodyIndexKey,
    OperationCustodyIndexMutation, OperationCustodyIndexQuery, OperationCustodyPort,
    OperationCustodyReadBackOutcome, OperationCustodyRecordKind, OperationCustodyTransitionKind,
    PreparedAntiRollbackAnchorTransition,
};

use super::custody_phase::{
    advance_anchor_with_exact_observation, commit_with_exact_readback, AnchorAdvanceEvidence,
    AnchorAdvanceFailureReason, DatabaseCommitDisposition, DatabaseCommitEvidence,
    DatabaseCommitFailureReason, DatabaseReadBackDisposition,
};
use super::record_projection;

/// Keeps operation request lookup separate from effect-case request lookup.
const OPERATION_REQUEST_KEY_DOMAIN: &[u8] = b"PALE_OPERATION_REQUEST_IDENTITY_INDEX_KEY_V1\0";

/// Prepare a durable root or child start and its matching hard-domain anchor.
///
/// # Errors
///
/// Returns [`OperationStartPreparationError`] when `candidate` is an observation
/// append or its checkpoint cannot form the required exact anchor successor.
pub fn prepare_operation_start(
    candidate: OperationCustodyCandidate,
) -> Result<PreparedOperationStart, OperationStartPreparationError> {
    if !matches!(
        candidate.kind(),
        OperationCustodyTransitionKind::StartRoot | OperationCustodyTransitionKind::StartChild
    ) {
        return Err(OperationStartPreparationError::NotStart);
    }
    let request_key = candidate
        .indexes()
        .iter()
        .find_map(|index| match index {
            OperationCustodyIndexMutation::RequestOperation { key, .. } => Some(key.clone()),
            _ => None,
        })
        .ok_or(OperationStartPreparationError::MissingRequestIndex)?;
    let anchor = prepare_anchor(&candidate)?;
    let request = candidate
        .record(OperationCustodyRecordKind::CommonRequest)
        .cloned()
        .ok_or(OperationStartPreparationError::MissingRequestRecord)?;
    Ok(PreparedOperationStart {
        candidate,
        request_key,
        anchor,
        request,
    })
}

/// Prepare physical custody for one already-admitted observation candidate.
///
/// This function proves only the closed [`OperationCustodyCandidate`] shape and
/// its hard-domain checkpoint. The caller must first contextually admit the
/// typed handle, predecessor, successor state and signature through
/// [`pale_core::records::OperationObservationV1`]. No lifecycle transition is
/// selected or authorized here.
///
/// # Errors
///
/// Returns [`OperationObservationCandidatePreparationError`] when `candidate` is
/// a start or its checkpoint cannot form the required exact anchor successor.
pub(super) fn prepare_operation_observation_candidate(
    candidate: OperationCustodyCandidate,
) -> Result<PreparedOperationObservationCustody, OperationObservationCandidatePreparationError> {
    if !matches!(
        candidate.kind(),
        OperationCustodyTransitionKind::Observe
            | OperationCustodyTransitionKind::PrepareA2aSend
            | OperationCustodyTransitionKind::PrepareA2aGetTask
            | OperationCustodyTransitionKind::DisposeA2aGetTask
            | OperationCustodyTransitionKind::BindA2aTask
            | OperationCustodyTransitionKind::ObserveA2aTask
    ) {
        return Err(OperationObservationCandidatePreparationError::NotObservation);
    }
    let anchor = prepare_anchor(&candidate)?;
    Ok(PreparedOperationObservationCustody { candidate, anchor })
}

fn prepare_anchor(
    candidate: &OperationCustodyCandidate,
) -> Result<PreparedAntiRollbackAnchorTransition, AntiRollbackAnchorTransitionError> {
    let checkpoint = candidate.checkpoint();
    let anchor = |head: &pale_ports::ControlCheckpointHead| {
        AntiRollbackAnchor::from(ControlDomainAntiRollbackAnchor::new(
            candidate.store().store_id().clone(),
            candidate.store().incarnation_id().clone(),
            head.checkpoint_domain_id().clone(),
            head.domain_epoch(),
            head.domain_sequence(),
            head.coordinate().clone(),
        ))
    };
    let next = anchor(checkpoint.successor());
    match checkpoint.phase() {
        pale_ports::OperationCheckpointPhase::Initial => {
            PreparedAntiRollbackAnchorTransition::initial(next)
        }
        pale_ports::OperationCheckpointPhase::Successor { predecessor, .. } => {
            PreparedAntiRollbackAnchorTransition::successor(anchor(predecessor), next)
        }
    }
}

/// Prepared start before request lookup or physical mutation.
#[derive(Debug)]
#[must_use = "prepared operation custody must be committed or retained"]
pub struct PreparedOperationStart {
    candidate: OperationCustodyCandidate,
    request_key: OperationCustodyIndexKey,
    anchor: PreparedAntiRollbackAnchorTransition,
    request: ControlRecord,
}

impl PreparedOperationStart {
    /// Resolve an ordinary duplicate by request identity, otherwise commit once.
    ///
    /// Commit ambiguity performs exactly one exact read-back and never retries.
    /// A concurrent candidate conflict gets one fresh request lookup. When
    /// another start won, this phase returns
    /// [`OperationRecoveryError::ExistingStartRequiresRecovery`] so an
    /// operation-specific resolver can re-admit that exact committed operation
    /// without replaying this stale candidate.
    /// `maximum_record_bytes` bounds each recovered body in both lookups; it
    /// does not replace the candidate's prior admission or input limits.
    ///
    /// # Errors
    ///
    /// Returns [`OperationStartCommitFailure`] when lookup is inconsistent,
    /// the independent anchor does not prove the predecessor, storage rejects
    /// the transaction, or one exact read-back cannot classify an
    /// outcome-unknown commit.
    pub fn commit(
        self,
        store: &mut dyn OperationCustodyPort,
        anchor_port: &mut dyn AntiRollbackAnchorPort,
        maximum_record_bytes: usize,
    ) -> Result<DatabaseCommittedOperationStartAnchorPending, OperationStartCommitFailure> {
        match read_operation_start(
            store,
            self.candidate.store(),
            &self.request_key,
            &self.request,
            maximum_record_bytes,
        ) {
            Ok(Some(evidence)) => {
                return Err(OperationStartCommitFailure::lookup(
                    self,
                    OperationRecoveryError::ExistingStartRequiresRecovery {
                        evidence: Box::new(evidence),
                    },
                ));
            }
            Ok(None) => {}
            Err(reason) => return Err(OperationStartCommitFailure::lookup(self, reason)),
        }

        if let Err(reason) = require_operation_anchor_predecessor(anchor_port, &self.anchor) {
            return Err(OperationStartCommitFailure::anchor_precondition(
                self, reason,
            ));
        }

        let database = commit_operation_candidate(store, &self.candidate);

        let evidence = match database {
            Ok(evidence) => evidence,
            Err(OperationCustodyDatabaseFailureReason::Store(conflict))
                if conflict.kind() == ControlStoreErrorKind::CandidateConflict =>
            {
                return match read_operation_start(
                    store,
                    self.candidate.store(),
                    &self.request_key,
                    &self.request,
                    maximum_record_bytes,
                ) {
                    Ok(Some(evidence)) => Err(OperationStartCommitFailure::lookup(
                        self,
                        OperationRecoveryError::ExistingStartRequiresRecovery {
                            evidence: Box::new(evidence),
                        },
                    )),
                    Ok(None) => Err(OperationStartCommitFailure::database(
                        self,
                        OperationCustodyDatabaseFailureReason::Store(conflict),
                    )),
                    Err(reentry) => Err(OperationStartCommitFailure::concurrent(
                        self, conflict, reentry,
                    )),
                };
            }
            Err(reason) => return Err(OperationStartCommitFailure::database(self, reason)),
        };
        Ok(self.pending(evidence))
    }

    fn pending(
        self,
        database_evidence: OperationCustodyDatabaseEvidence,
    ) -> DatabaseCommittedOperationStartAnchorPending {
        DatabaseCommittedOperationStartAnchorPending {
            candidate: self.candidate,
            anchor: self.anchor,
            database_evidence,
        }
    }
}

fn commit_operation_candidate(
    store: &mut dyn OperationCustodyPort,
    candidate: &OperationCustodyCandidate,
) -> Result<OperationCustodyDatabaseEvidence, OperationCustodyDatabaseFailureReason> {
    commit_with_exact_readback(
        store,
        candidate,
        |store, candidate| {
            store
                .compare_and_commit(candidate)
                .map(|outcome| match outcome {
                    OperationCustodyCommitOutcome::Committed => {
                        DatabaseCommitDisposition::Committed
                    }
                    OperationCustodyCommitOutcome::ExactReplay => {
                        DatabaseCommitDisposition::ExactReplay
                    }
                })
        },
        |store, candidate| {
            store.observe_exact(candidate).map(|outcome| match outcome {
                OperationCustodyReadBackOutcome::Predecessor => {
                    DatabaseReadBackDisposition::Predecessor
                }
                OperationCustodyReadBackOutcome::ExactSuccessor => {
                    DatabaseReadBackDisposition::ExactSuccessor
                }
                OperationCustodyReadBackOutcome::ConflictingOrPartial => {
                    DatabaseReadBackDisposition::ConflictingOrPartial
                }
            })
        },
    )
    .map(Into::into)
    .map_err(Into::into)
}

/// Prepared physical append with no semantic observation authority.
#[derive(Debug)]
#[must_use = "prepared observation custody must be committed or retained"]
pub struct PreparedOperationObservationCustody {
    candidate: OperationCustodyCandidate,
    anchor: PreparedAntiRollbackAnchorTransition,
}

impl PreparedOperationObservationCustody {
    /// Read back a reconstructed append without ever attempting a database write.
    pub(in crate::resident) fn recover_committed(
        self,
        store: &mut dyn OperationCustodyPort,
    ) -> Result<DatabaseCommittedOperationObservationAnchorPending, ControlStoreError> {
        match store.observe_exact(&self.candidate)? {
            OperationCustodyReadBackOutcome::ExactSuccessor => {
                Ok(self.pending(OperationCustodyDatabaseEvidence::ExactReplay))
            }
            OperationCustodyReadBackOutcome::Predecessor
            | OperationCustodyReadBackOutcome::ConflictingOrPartial => Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            )),
        }
    }

    /// Attach an application-admitted collection journal to this exact append.
    ///
    /// The journal already carries the same transaction fence as the accumulator.
    /// Rebuilding the prepared anchor is mechanical: collection material does
    /// not change the signed checkpoint coordinate. This method grants no roster,
    /// stopping-policy, or child-dispatch authority.
    pub(in crate::resident) fn with_collection_journal(
        self,
        journal: pale_ports::OperationCollectionJournal,
    ) -> Result<
        Self,
        super::operation_observation_custody::OperationObservationCustodyProjectionError,
    > {
        self.with_collection_journal_after(journal, None)
    }

    /// Attach a successor collection after its exact private frontier.
    pub(in crate::resident) fn with_collection_journal_after(
        self,
        journal: pale_ports::OperationCollectionJournal,
        predecessor: Option<pale_ports::ControlRecord>,
    ) -> Result<
        Self,
        super::operation_observation_custody::OperationObservationCustodyProjectionError,
    > {
        let candidate = self
            .candidate
            .with_collection_journal_after(journal, predecessor)?;
        prepare_operation_observation_candidate(candidate).map_err(|error| match error {
            OperationObservationCandidatePreparationError::NotObservation => {
                super::operation_observation_custody::OperationObservationCustodyProjectionError::Lineage
            }
            OperationObservationCandidatePreparationError::Anchor(error) => error.into(),
        })
    }

    /// Consume the prepared candidate and attempt one atomic append.
    ///
    /// Commit ambiguity performs exactly one exact read-back and never retries.
    /// Ordinary historical replay after a later observation is intentionally
    /// excluded: recovery reads the immutable observation/checkpoint index instead.
    ///
    /// # Errors
    ///
    /// Returns [`OperationObservationCustodyCommitFailure`] with the complete
    /// prepared value when the independent predecessor or exact database
    /// custody remains unproven.
    pub fn commit(
        self,
        store: &mut dyn OperationCustodyPort,
        anchor_port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<
        DatabaseCommittedOperationObservationAnchorPending,
        OperationObservationCustodyCommitFailure,
    > {
        match store.observe_exact(&self.candidate) {
            Ok(OperationCustodyReadBackOutcome::ExactSuccessor) => {
                return Ok(self.pending(OperationCustodyDatabaseEvidence::ExactReplay));
            }
            Ok(OperationCustodyReadBackOutcome::Predecessor) => {
                if let Err(reason) = require_operation_anchor_predecessor(anchor_port, &self.anchor)
                {
                    return Err(OperationObservationCustodyCommitFailure {
                        prepared: Box::new(self),
                        reason: OperationObservationCustodyCommitFailureReason::AnchorPrecondition(
                            reason,
                        ),
                    });
                }
            }
            Ok(OperationCustodyReadBackOutcome::ConflictingOrPartial) => {
                return Err(OperationObservationCustodyCommitFailure {
                    prepared: Box::new(self),
                    reason: OperationObservationCustodyCommitFailureReason::Database(
                        OperationCustodyDatabaseFailureReason::Store(ControlStoreError::new(
                            ControlStoreErrorKind::RecoveryRequired,
                        )),
                    ),
                });
            }
            Err(error) => {
                return Err(OperationObservationCustodyCommitFailure {
                    prepared: Box::new(self),
                    reason: OperationObservationCustodyCommitFailureReason::Database(
                        OperationCustodyDatabaseFailureReason::Store(error),
                    ),
                });
            }
        }
        match commit_operation_candidate(store, &self.candidate) {
            Ok(database_evidence) => Ok(self.pending(database_evidence)),
            Err(reason) => Err(OperationObservationCustodyCommitFailure {
                prepared: Box::new(self),
                reason: OperationObservationCustodyCommitFailureReason::Database(reason),
            }),
        }
    }

    fn pending(
        self,
        database_evidence: OperationCustodyDatabaseEvidence,
    ) -> DatabaseCommittedOperationObservationAnchorPending {
        DatabaseCommittedOperationObservationAnchorPending {
            candidate: self.candidate,
            anchor: self.anchor,
            database_evidence,
        }
    }
}

/// Require the exact independently retained predecessor before any new database write.
fn require_operation_anchor_predecessor(
    port: &mut dyn AntiRollbackAnchorPort,
    transition: &PreparedAntiRollbackAnchorTransition,
) -> Result<(), OperationCustodyAnchorFailureReason> {
    match port
        .observe(transition)
        .map(|observation| classify_ahead(transition, observation))
    {
        Ok(observation) if expected_anchor_observation(transition, &observation) => Ok(()),
        Ok(observation) => Err(OperationCustodyAnchorFailureReason::NotAdvanced(Box::new(
            observation,
        ))),
        Err(error) => Err(OperationCustodyAnchorFailureReason::Port(error)),
    }
}

// Absence remains Missing in the port owner. Only the explicitly initial
// transition expects it; a missing successor anchor is rollback/recovery.
fn expected_anchor_observation(
    transition: &PreparedAntiRollbackAnchorTransition,
    observation: &AntiRollbackAnchorObservation,
) -> bool {
    matches!(
        (transition.expected(), observation),
        (None, AntiRollbackAnchorObservation::Missing)
            | (Some(_), AntiRollbackAnchorObservation::ExactPredecessor)
    )
}

/// Exact database evidence retained independently of hard-domain anchoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationCustodyDatabaseEvidence {
    /// The complete transaction committed during this call.
    New,
    /// The complete exact transaction already existed.
    ExactReplay,
    /// One read-back found the exact successor after outcome uncertainty.
    RecoveredExactAfterAmbiguousCommit,
}

/// Database-proven observation append awaiting its independent anchor.
///
/// This phase grants no semantic [`pale_core::records::OperationObservationV1`]
/// access and no provider, Event, Genealogy, or continuation permission.
#[must_use = "database custody alone is not an admitted observation"]
pub struct DatabaseCommittedOperationObservationAnchorPending {
    candidate: OperationCustodyCandidate,
    anchor: PreparedAntiRollbackAnchorTransition,
    database_evidence: OperationCustodyDatabaseEvidence,
}

impl std::fmt::Debug for DatabaseCommittedOperationObservationAnchorPending {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DatabaseCommittedOperationObservationAnchorPending")
            .field("database_evidence", &self.database_evidence)
            .finish_non_exhaustive()
    }
}

impl DatabaseCommittedOperationObservationAnchorPending {
    /// Return how exact database custody was established.
    #[must_use]
    pub const fn database_evidence(&self) -> OperationCustodyDatabaseEvidence {
        self.database_evidence
    }

    /// Borrow the exact committed observation transition.
    #[must_use]
    pub const fn candidate(&self) -> &OperationCustodyCandidate {
        &self.candidate
    }

    /// Prove the independently retained hard-domain checkpoint.
    ///
    /// # Errors
    ///
    /// Returns [`OperationObservationCustodyAnchorFailure`] with this pending
    /// phase intact when the anchor remains unproved.
    pub fn advance_anchor(
        self,
        port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedOperationObservationCustody, OperationObservationCustodyAnchorFailure>
    {
        match establish_operation_anchor(port, &self.anchor, self.database_evidence) {
            Ok(anchor_evidence) => {
                let Self {
                    candidate,
                    database_evidence,
                    ..
                } = self;
                Ok(CompletedOperationObservationCustody {
                    candidate,
                    database_evidence,
                    anchor_evidence,
                })
            }
            Err(reason) => Err(OperationObservationCustodyAnchorFailure {
                pending: Box::new(self),
                reason: Box::new(reason),
            }),
        }
    }
}

/// Database-proven start that grants neither handle access nor continuation.
#[must_use = "database custody alone is not operation eligibility"]
pub struct DatabaseCommittedOperationStartAnchorPending {
    candidate: OperationCustodyCandidate,
    anchor: PreparedAntiRollbackAnchorTransition,
    database_evidence: OperationCustodyDatabaseEvidence,
}

impl std::fmt::Debug for DatabaseCommittedOperationStartAnchorPending {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DatabaseCommittedOperationStartAnchorPending")
            .field("database_evidence", &self.database_evidence)
            .finish_non_exhaustive()
    }
}

impl DatabaseCommittedOperationStartAnchorPending {
    /// Return how exact database custody was established.
    #[must_use]
    pub const fn database_evidence(&self) -> OperationCustodyDatabaseEvidence {
        self.database_evidence
    }

    /// Borrow the exact database-committed start transition.
    #[must_use]
    pub const fn candidate(&self) -> &OperationCustodyCandidate {
        &self.candidate
    }

    /// Prove the independently retained hard-domain checkpoint.
    ///
    /// Re-entry first observes an existing or replayed start. Only the exact
    /// successor closes historical custody without replaying the database
    /// transaction. A numerically later checkpoint is not ancestry proof.
    /// Every attempted advance occurs once; outcome uncertainty permits one
    /// observation only.
    ///
    /// # Errors
    ///
    /// Returns [`OperationStartAnchorFailure`] with this pending phase intact
    /// when the anchor is absent, behind, conflicting, unavailable, or remains
    /// ambiguous after its sole observation.
    pub fn advance_anchor(
        self,
        port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedOperationStartCustody, OperationStartAnchorFailure> {
        let evidence = match establish_operation_anchor(port, &self.anchor, self.database_evidence)
        {
            Ok(evidence) => evidence,
            Err(reason) => return Err(self.failed(reason)),
        };
        Ok(CompletedOperationStartCustody {
            candidate: self.candidate,
            database_evidence: self.database_evidence,
            anchor_evidence: evidence,
        })
    }

    fn failed(self, reason: OperationCustodyAnchorFailureReason) -> OperationStartAnchorFailure {
        OperationStartAnchorFailure {
            pending: Box::new(self),
            reason: Box::new(reason),
        }
    }
}

fn establish_operation_anchor(
    port: &mut dyn AntiRollbackAnchorPort,
    anchor: &PreparedAntiRollbackAnchorTransition,
    database_evidence: OperationCustodyDatabaseEvidence,
) -> Result<OperationCustodyAnchorEvidence, OperationCustodyAnchorFailureReason> {
    match database_evidence {
        OperationCustodyDatabaseEvidence::ExactReplay => match port
            .observe(anchor)
            .map(|observation| classify_ahead(anchor, observation))
        {
            Ok(AntiRollbackAnchorObservation::ExactSuccessor) => {
                Ok(OperationCustodyAnchorEvidence::ExactReplay)
            }
            Ok(observation) if expected_anchor_observation(anchor, &observation) => {
                advance_operation_anchor(port, anchor)
            }
            Ok(observation) => Err(OperationCustodyAnchorFailureReason::NotAdvanced(Box::new(
                observation,
            ))),
            Err(error) => Err(OperationCustodyAnchorFailureReason::Port(error)),
        },
        OperationCustodyDatabaseEvidence::New
        | OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit => {
            match advance_operation_anchor(port, anchor)? {
                OperationCustodyAnchorEvidence::ExactReplay => {
                    Err(OperationCustodyAnchorFailureReason::NotAdvanced(Box::new(
                        AntiRollbackAnchorObservation::ExactSuccessor,
                    )))
                }
                evidence => Ok(evidence),
            }
        }
    }
}

fn advance_operation_anchor(
    port: &mut dyn AntiRollbackAnchorPort,
    transition: &PreparedAntiRollbackAnchorTransition,
) -> Result<OperationCustodyAnchorEvidence, OperationCustodyAnchorFailureReason> {
    match advance_anchor_with_exact_observation(port, transition) {
        Ok(evidence) => Ok(evidence.into()),
        Err(AnchorAdvanceFailureReason::NotAdvanced(observation)) => {
            Err(OperationCustodyAnchorFailureReason::NotAdvanced(Box::new(
                classify_ahead(transition, *observation),
            )))
        }
        Err(AnchorAdvanceFailureReason::AmbiguousNotExact {
            uncertain,
            observation,
        }) => Err(OperationCustodyAnchorFailureReason::AmbiguousNotExact {
            uncertain,
            observation: Box::new(classify_ahead(transition, *observation)),
        }),
        Err(reason) => Err(reason.into()),
    }
}

/// Reclassify every claimed later value through the prepared transition SSOT.
fn classify_ahead(
    transition: &PreparedAntiRollbackAnchorTransition,
    observation: AntiRollbackAnchorObservation,
) -> AntiRollbackAnchorObservation {
    match observation {
        AntiRollbackAnchorObservation::Ahead { observed } => {
            transition.classify_observed(Some(&observed))
        }
        observation => observation,
    }
}

/// Hard-domain evidence establishing the exact operation-custody checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationCustodyAnchorEvidence {
    /// The external anchor advanced during this call.
    Advanced,
    /// The external anchor already contained the exact successor.
    ExactReplay,
    /// One observation found the exact successor after an ambiguous advance.
    RecoveredExactAfterAmbiguousAdvance,
}

/// Physical start custody whose database and hard-domain anchors are both proven.
///
/// This is not an admitted [`pale_core::records::OperationHandleV1`] and
/// grants no dispatch or observation authority.
#[derive(Debug)]
pub struct CompletedOperationStartCustody {
    candidate: OperationCustodyCandidate,
    database_evidence: OperationCustodyDatabaseEvidence,
    anchor_evidence: OperationCustodyAnchorEvidence,
}

/// Physical observation custody whose database and anchor are both proven.
///
/// The completed value contains custody evidence only. It is not the operation
/// service's [`crate::resident::AdmittedAmbientSession`], an observation
/// result, or permission to project Event/Genealogy consequences.
#[doc = include_str!("../../tests/resident/observation_custody.md")]
#[derive(Debug)]
pub struct CompletedOperationObservationCustody {
    candidate: OperationCustodyCandidate,
    database_evidence: OperationCustodyDatabaseEvidence,
    anchor_evidence: OperationCustodyAnchorEvidence,
}

impl CompletedOperationObservationCustody {
    /// Borrow the exact observation transition whose custody completed.
    #[must_use]
    pub const fn candidate(&self) -> &OperationCustodyCandidate {
        &self.candidate
    }

    /// Return how exact database custody was established.
    #[must_use]
    pub const fn database_evidence(&self) -> OperationCustodyDatabaseEvidence {
        self.database_evidence
    }

    /// Return how independent hard-domain custody was established.
    #[must_use]
    pub const fn anchor_evidence(&self) -> OperationCustodyAnchorEvidence {
        self.anchor_evidence
    }
}

impl CompletedOperationStartCustody {
    /// Borrow the exact start transition whose custody completed.
    #[must_use]
    pub const fn candidate(&self) -> &OperationCustodyCandidate {
        &self.candidate
    }

    /// Return how exact database custody was established.
    #[must_use]
    pub const fn database_evidence(&self) -> OperationCustodyDatabaseEvidence {
        self.database_evidence
    }

    /// Return how independent hard-domain custody was established.
    #[must_use]
    pub const fn anchor_evidence(&self) -> OperationCustodyAnchorEvidence {
        self.anchor_evidence
    }
}

/// Find an operation's original start from its already-admitted signed request.
///
/// The lookup key uses the caller identity, signing epoch, and request ID.
/// Different signed contents under that same tuple conflict with the stored
/// request; changing contents cannot allocate another lookup slot. A retry needs
/// only the signed request, not the internal reference assigned at its start.
/// The returned evidence preserves that original stored reference.
///
/// This encodes the request once and uses the same lookup as [`read_operation_start`]. It
/// performs at most four point reads and no writes. Returned bytes are historical
/// evidence, not an admitted handle or renewed permission to run the operation.
/// The caller must separately establish permission to inspect the selected store.
/// Each returned body is limited to `maximum_record_bytes`; the limit is not
/// inferred from the current request or from a replacement start candidate.
///
/// # Errors
///
/// Returns [`OperationRecoveryError`] if encoding fails, request contents
/// conflict, indexes are incomplete, or storage cannot answer the exact lookup.
pub fn read_operation_start_for_request(
    store: &mut dyn OperationCustodyPort,
    coordinate: &pale_ports::AuthorityStoreCoordinate,
    request: &CommonRequestEnvelopeV1,
    maximum_record_bytes: usize,
) -> Result<Option<OperationStartRecoveryEvidence>, OperationRecoveryError> {
    let key = operation_request_key(request)?;
    let canonical_request = canon::to_canonical_string(request)?;
    read_operation_start_matching(store, coordinate, &key, maximum_record_bytes, |stored| {
        stored.schema_id() == request.envelope_schema_id()
            && stored.schema_sha256() == request.envelope_schema_sha256()
            && stored.coordinate().record_sha256().as_str()
                == request.canonical_request_sha256().as_str()
            && stored.canonical_record_jcs().as_slice() == canonical_request.as_bytes()
    })
}

pub(super) fn operation_request_key(
    request: &CommonRequestEnvelopeV1,
) -> Result<OperationCustodyIndexKey, OperationRecoveryError> {
    let canonical = canon::to_canonical_string(
        &record_projection::RequestIdentityKeyPreimage::from(request),
    )?;
    let mut bytes = Vec::with_capacity(OPERATION_REQUEST_KEY_DOMAIN.len() + canonical.len());
    bytes.extend_from_slice(OPERATION_REQUEST_KEY_DOMAIN);
    bytes.extend_from_slice(canonical.as_bytes());
    OperationCustodyIndexKey::parse(pale_core::sha256_hex(&bytes)).map_err(Into::into)
}

/// Read an operation's original start records without preparing or retrying a start.
///
/// `request_key` identifies the admitted request within `coordinate`; `expected_request`
/// supplies its exact signed bytes for conflict detection. The lookup returns
/// the original handle, initial observation, and committing checkpoint, even
/// after later observations advance. It performs at most four point reads and
/// never writes or accesses the anti-rollback anchor.
/// `maximum_record_bytes` limits each returned body independently. The four
/// immutable lookups are not a shared storage snapshot.
///
/// `None` means neither request index exists. Returned records still require
/// semantic and checkpoint-ancestry admission before exposing an operation;
/// this read does not authorize continuation or dispatch.
///
/// # Errors
///
/// Returns [`OperationRecoveryError`] for conflicting request bytes,
/// incomplete indexes, a misdirected response, or a storage failure. Partial
/// evidence is not treated as an absent operation.
pub fn read_operation_start(
    store: &mut dyn OperationCustodyPort,
    coordinate: &pale_ports::AuthorityStoreCoordinate,
    request_key: &OperationCustodyIndexKey,
    expected_request: &ControlRecord,
    maximum_record_bytes: usize,
) -> Result<Option<OperationStartRecoveryEvidence>, OperationRecoveryError> {
    read_operation_start_matching(
        store,
        coordinate,
        request_key,
        maximum_record_bytes,
        |stored| stored == expected_request,
    )
}

/// Share point reads while preserving each caller's exact request comparison.
fn read_operation_start_matching(
    store: &mut dyn OperationCustodyPort,
    coordinate: &pale_ports::AuthorityStoreCoordinate,
    request_key: &OperationCustodyIndexKey,
    maximum_record_bytes: usize,
    matches_request: impl FnOnce(&ControlRecord) -> bool,
) -> Result<Option<OperationStartRecoveryEvidence>, OperationRecoveryError> {
    let Some((request, handle)) = read_operation_identity_matching(
        store,
        coordinate,
        request_key,
        maximum_record_bytes,
        matches_request,
    )?
    else {
        return Ok(None);
    };
    let initial_observation = read_index(
        store,
        coordinate,
        OperationCustodyIndexQuery::StartObservation(request_key.clone()),
        maximum_record_bytes,
    )?
    .ok_or(OperationRecoveryError::IncompleteRequestIndexes)?;
    let checkpoint = read_index(
        store,
        coordinate,
        OperationCustodyIndexQuery::ObservationCheckpoint(
            OperationCustodyIndexKey::for_observation(initial_observation.coordinate()),
        ),
        maximum_record_bytes,
    )?
    .ok_or(OperationRecoveryError::IncompleteRequestIndexes)?;
    Ok(Some(OperationStartRecoveryEvidence {
        store: coordinate.clone(),
        request,
        handle,
        initial_observation,
        checkpoint,
    }))
}

fn read_operation_identity_matching(
    store: &mut dyn OperationCustodyPort,
    coordinate: &pale_ports::AuthorityStoreCoordinate,
    request_key: &OperationCustodyIndexKey,
    maximum_record_bytes: usize,
    matches_request: impl FnOnce(&ControlRecord) -> bool,
) -> Result<Option<(ControlRecord, ControlRecord)>, OperationRecoveryError> {
    let operation_query = OperationCustodyIndexQuery::RequestOperation(request_key.clone());
    let operation = read_index(store, coordinate, operation_query, maximum_record_bytes)?;
    let evidence_query = OperationCustodyIndexQuery::RequestEvidence(request_key.clone());
    let evidence = read_index(store, coordinate, evidence_query, maximum_record_bytes)?;
    if let Some(request) = &evidence {
        if !matches_request(request) {
            return Err(OperationRecoveryError::RequestConflict);
        }
    }
    match (operation, evidence) {
        (Some(handle), Some(request)) => Ok(Some((request, handle))),
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => Err(OperationRecoveryError::IncompleteRequestIndexes),
    }
}

pub(super) fn read_index(
    store: &mut dyn OperationCustodyPort,
    coordinate: &pale_ports::AuthorityStoreCoordinate,
    query: OperationCustodyIndexQuery,
    maximum_record_bytes: usize,
) -> Result<Option<ControlRecord>, OperationRecoveryError> {
    let observation = store
        .read_index(coordinate, &query, maximum_record_bytes)
        .map_err(OperationRecoveryError::Store)?;
    if observation.query() != &query {
        return Err(OperationRecoveryError::MisdirectedObservation);
    }
    if observation
        .record()
        .is_some_and(|record| record.canonical_record_jcs().as_slice().len() > maximum_record_bytes)
    {
        return Err(OperationRecoveryError::RecordBoundExceeded);
    }
    Ok(observation.into_record())
}

/// Immutable physical evidence recovered for an existing request.
///
/// Records are schema-role checked, not contextually re-admitted here. Before
/// exposing an operation, recovery must verify their signed semantic linkage
/// and checkpoint ancestry against the independent anti-rollback anchor.
/// Neither a later latest-observation head nor a new start candidate replaces
/// these original records. This value grants no dispatch authority.
#[derive(Debug)]
pub struct OperationStartRecoveryEvidence {
    store: pale_ports::AuthorityStoreCoordinate,
    request: ControlRecord,
    handle: ControlRecord,
    initial_observation: ControlRecord,
    checkpoint: ControlRecord,
}

impl OperationStartRecoveryEvidence {
    /// Borrow the exact deployment-control incarnation queried for these records.
    #[must_use]
    pub const fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the exact caller-signed request matched during duplicate lookup.
    #[must_use]
    pub const fn request(&self) -> &ControlRecord {
        &self.request
    }

    /// Borrow the stored handle bytes without granting semantic admission.
    #[must_use]
    pub const fn handle(&self) -> &ControlRecord {
        &self.handle
    }

    /// Borrow the initial observation, even after its latest head advances.
    #[must_use]
    pub const fn initial_observation(&self) -> &ControlRecord {
        &self.initial_observation
    }

    /// Borrow the signed checkpoint that committed the initial observation.
    #[must_use]
    pub const fn checkpoint(&self) -> &ControlRecord {
        &self.checkpoint
    }
}

/// Candidate preparation failure before any custody access.
#[derive(Debug, thiserror::Error)]
pub enum OperationStartPreparationError {
    /// Observation append candidates do not enter the start-only phase.
    #[error("operation custody candidate is not a root or child start")]
    NotStart,
    /// A validated start unexpectedly lacks its request-operation index.
    #[error("operation start lacks its request-operation index")]
    MissingRequestIndex,
    /// A validated start unexpectedly lacks its common-request record.
    #[error("operation start lacks its common-request record")]
    MissingRequestRecord,
    /// The checkpoint cannot form an exact external-anchor successor.
    #[error(transparent)]
    Anchor(#[from] AntiRollbackAnchorTransitionError),
}

/// Physical operation recovery failed before evidence could be returned.
#[derive(Debug, thiserror::Error)]
pub enum OperationRecoveryError {
    /// A typed request, exact handle value, or private identity key could not be
    /// canonically encoded.
    #[error(transparent)]
    Canonicalize(#[from] canon::CanonError),
    /// A projected request/handle record or private lookup key was malformed.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
    /// The store could not answer one exact point query.
    #[error(transparent)]
    Store(ControlStoreError),
    /// The store answered a different typed query than the one requested.
    #[error("operation custody returned a misdirected point observation")]
    MisdirectedObservation,
    /// A nonconforming port returned a body above the caller's read limit.
    #[error("operation custody returned a record above the recovery byte limit")]
    RecordBoundExceeded,
    /// The request identity is already bound to different signed evidence.
    #[error("operation request identity conflicts with stored request evidence")]
    RequestConflict,
    /// Request evidence and request-operation mapping are not both present.
    #[error("operation request indexes are incomplete")]
    IncompleteRequestIndexes,
    /// The stable handle identity is bound to different immutable handle contents.
    #[error("operation handle identity conflicts with stored handle evidence")]
    HandleConflict,
    /// Handle/latest-observation custody or its committing checkpoint is incomplete.
    #[error("operation observation indexes are incomplete")]
    IncompleteObservationIndexes,
    /// Existing request and handle bytes need their original committed checkpoint
    /// and complete semantic re-admission before a handle can be exposed.
    #[error("existing operation start requires exact semantic and checkpoint recovery")]
    ExistingStartRequiresRecovery {
        /// Original immutable evidence awaiting semantic and anchor re-admission.
        evidence: Box<OperationStartRecoveryEvidence>,
    },
}

impl From<record_projection::ControlRecordProjectionError> for OperationRecoveryError {
    fn from(value: record_projection::ControlRecordProjectionError) -> Self {
        match value {
            record_projection::ControlRecordProjectionError::Canonicalize(error) => {
                Self::Canonicalize(error)
            }
            record_projection::ControlRecordProjectionError::Record(error) => Self::Record(error),
        }
    }
}

/// Closed database-custody failure classification shared by operation transitions.
#[derive(Debug, thiserror::Error)]
pub enum OperationCustodyDatabaseFailureReason {
    /// Storage rejected the transaction before outcome ambiguity.
    #[error(transparent)]
    Store(ControlStoreError),
    /// Read-back proved that an ambiguous commit did not advance.
    #[error("ambiguous operation custody was excluded by exact read-back")]
    CommitExcludedByReadBack {
        /// Original outcome-unknown database error.
        uncertain: ControlStoreError,
    },
    /// Read-back found partial, contradictory, or competing custody.
    #[error("ambiguous operation custody requires explicit recovery")]
    RecoveryRequired {
        /// Original outcome-unknown database error.
        uncertain: ControlStoreError,
    },
    /// The sole exact read-back failed after outcome uncertainty.
    #[error("exact operation-custody read-back failed after ambiguous commit")]
    ReadBackFailed {
        /// Original outcome-unknown database error.
        uncertain: ControlStoreError,
        /// Read-back failure preventing classification.
        #[source]
        read_back: ControlStoreError,
    },
}

/// Failed request lookup, anchor precondition, or database custody.
#[derive(Debug, thiserror::Error)]
#[error("operation start custody commit did not complete: {reason}")]
pub struct OperationStartCommitFailure {
    prepared: Box<PreparedOperationStart>,
    reason: OperationStartCommitFailureReason,
}

impl OperationStartCommitFailure {
    fn lookup(prepared: PreparedOperationStart, reason: OperationRecoveryError) -> Self {
        Self {
            prepared: Box::new(prepared),
            reason: OperationStartCommitFailureReason::Lookup(reason),
        }
    }

    fn database(
        prepared: PreparedOperationStart,
        reason: OperationCustodyDatabaseFailureReason,
    ) -> Self {
        Self {
            prepared: Box::new(prepared),
            reason: OperationStartCommitFailureReason::Database(reason),
        }
    }

    fn anchor_precondition(
        prepared: PreparedOperationStart,
        reason: OperationCustodyAnchorFailureReason,
    ) -> Self {
        Self {
            prepared: Box::new(prepared),
            reason: OperationStartCommitFailureReason::AnchorPrecondition(reason),
        }
    }

    fn concurrent(
        prepared: PreparedOperationStart,
        conflict: ControlStoreError,
        reentry: OperationRecoveryError,
    ) -> Self {
        Self {
            prepared: Box::new(prepared),
            reason: OperationStartCommitFailureReason::ConcurrentConflict {
                conflict,
                reentry: Box::new(reentry),
            },
        }
    }

    /// Borrow the closed reason why database custody did not complete.
    #[must_use]
    pub const fn reason(&self) -> &OperationStartCommitFailureReason {
        &self.reason
    }

    /// Recover a mutation-capable start only after definitive non-commit evidence.
    ///
    /// A storage failure occurred before commit ambiguity, an excluded
    /// ambiguous commit was read back at the exact predecessor, or a read-only
    /// anchor preflight failed before database mutation. Every other failure
    /// retains this object so outcome uncertainty, partial state, or a
    /// competing candidate cannot be converted into a blind retry.
    ///
    /// # Errors
    ///
    /// Returns this unchanged failure when another commit is not proved safe.
    pub fn try_into_retryable(self) -> Result<PreparedOperationStart, Self> {
        let retryable = matches!(
            &self.reason,
            OperationStartCommitFailureReason::Database(
                OperationCustodyDatabaseFailureReason::Store(error)
            ) if error.kind() == ControlStoreErrorKind::Storage
        ) || matches!(
            &self.reason,
            OperationStartCommitFailureReason::Database(
                OperationCustodyDatabaseFailureReason::CommitExcludedByReadBack { .. }
            )
        ) || matches!(
            &self.reason,
            OperationStartCommitFailureReason::AnchorPrecondition(
                OperationCustodyAnchorFailureReason::Port(_)
            )
        );
        if retryable {
            Ok(*self.prepared)
        } else {
            Err(self)
        }
    }

    /// Perform one later exact read-back after an ambiguous start commit.
    ///
    /// This never commits and never replaces the retained candidate. Only the
    /// exact complete successor advances to anchor custody. Every other result
    /// leaves this failure intact for explicit recovery.
    pub(in crate::resident) fn reconcile_exact(
        self,
        store: &mut dyn OperationCustodyPort,
    ) -> Result<DatabaseCommittedOperationStartAnchorPending, Self> {
        let unresolved = matches!(
            &self.reason,
            OperationStartCommitFailureReason::Database(
                OperationCustodyDatabaseFailureReason::RecoveryRequired { .. }
                    | OperationCustodyDatabaseFailureReason::ReadBackFailed { .. }
            )
        );
        if !unresolved {
            return Err(self);
        }
        match store.observe_exact(&self.prepared.candidate) {
            Ok(OperationCustodyReadBackOutcome::ExactSuccessor) => Ok((*self.prepared)
                .pending(OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit)),
            Ok(OperationCustodyReadBackOutcome::Predecessor)
            | Ok(OperationCustodyReadBackOutcome::ConflictingOrPartial)
            | Err(_) => Err(self),
        }
    }
}

/// Stage at which one operation start failed before anchoring.
#[derive(Debug, thiserror::Error)]
pub enum OperationStartCommitFailureReason {
    /// Ordinary request lookup could not establish a safe start or replay.
    #[error(transparent)]
    Lookup(OperationRecoveryError),
    /// The independent anchor did not prove the database predecessor.
    #[error("operation-start anchor precondition failed before database mutation")]
    AnchorPrecondition(OperationCustodyAnchorFailureReason),
    /// The one allowed database commit did not establish exact custody.
    #[error(transparent)]
    Database(OperationCustodyDatabaseFailureReason),
    /// A concurrent conflict could not be resolved as the same request.
    #[error("concurrent operation-start conflict did not resolve by request identity")]
    ConcurrentConflict {
        /// Original candidate-conflict error.
        conflict: ControlStoreError,
        /// Exact request lookup failure after the conflict.
        #[source]
        reentry: Box<OperationRecoveryError>,
    },
}

/// Candidate-shape failure before observation custody access.
#[derive(Debug, thiserror::Error)]
pub(super) enum OperationObservationCandidatePreparationError {
    /// Root and child starts do not enter the observation-only phase.
    #[error("operation custody candidate is not an observation append")]
    NotObservation,
    /// The checkpoint cannot form an exact external-anchor successor.
    #[error(transparent)]
    Anchor(#[from] AntiRollbackAnchorTransitionError),
}

/// Failed anchor precondition or database custody with the candidate retained.
#[derive(Debug, thiserror::Error)]
#[error("operation observation custody commit did not complete: {reason}")]
pub struct OperationObservationCustodyCommitFailure {
    prepared: Box<PreparedOperationObservationCustody>,
    reason: OperationObservationCustodyCommitFailureReason,
}

impl OperationObservationCustodyCommitFailure {
    /// Borrow the closed precondition or database-custody failure reason.
    #[must_use]
    pub const fn reason(&self) -> &OperationObservationCustodyCommitFailureReason {
        &self.reason
    }

    /// Borrow the exact physical candidate retained by this failure.
    pub(in crate::resident) const fn candidate(&self) -> &OperationCustodyCandidate {
        &self.prepared.candidate
    }

    /// Recover a mutation-capable append only after definitive non-commit evidence.
    /// Read-only anchor-port failure occurs before database mutation.
    ///
    /// # Errors
    ///
    /// Returns this unchanged failure after a competing candidate, partial
    /// state, or failed ambiguity read-back so the caller cannot blindly retry.
    pub fn try_into_retryable(self) -> Result<PreparedOperationObservationCustody, Self> {
        let retryable = matches!(
            &self.reason,
            OperationObservationCustodyCommitFailureReason::Database(
                OperationCustodyDatabaseFailureReason::Store(error)
            )
                if error.kind() == ControlStoreErrorKind::Storage
        ) || matches!(
            &self.reason,
            OperationObservationCustodyCommitFailureReason::Database(
                OperationCustodyDatabaseFailureReason::CommitExcludedByReadBack { .. }
            )
        ) || matches!(
            &self.reason,
            OperationObservationCustodyCommitFailureReason::AnchorPrecondition(
                OperationCustodyAnchorFailureReason::Port(_)
            )
        );
        if retryable {
            Ok(*self.prepared)
        } else {
            Err(self)
        }
    }

    /// Perform one later exact read-back after the original ambiguous commit
    /// could not be classified.
    ///
    /// This never commits and never replaces the retained candidate. It can
    /// advance only when storage proves that exact candidate is the complete
    /// successor. A predecessor, conflict, partial state, or read failure keeps
    /// this failure unchanged for explicit recovery.
    pub(in crate::resident) fn reconcile_exact(
        self,
        store: &mut dyn OperationCustodyPort,
    ) -> Result<DatabaseCommittedOperationObservationAnchorPending, Self> {
        let unresolved = matches!(
            &self.reason,
            OperationObservationCustodyCommitFailureReason::Database(
                OperationCustodyDatabaseFailureReason::RecoveryRequired { .. }
                    | OperationCustodyDatabaseFailureReason::ReadBackFailed { .. }
            )
        );
        if !unresolved {
            return Err(self);
        }
        match store.observe_exact(&self.prepared.candidate) {
            Ok(OperationCustodyReadBackOutcome::ExactSuccessor) => Ok((*self.prepared)
                .pending(OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit)),
            Ok(OperationCustodyReadBackOutcome::Predecessor)
            | Ok(OperationCustodyReadBackOutcome::ConflictingOrPartial)
            | Err(_) => Err(self),
        }
    }
}

/// Stage at which one operation observation failed before anchoring.
#[derive(Debug, thiserror::Error)]
pub enum OperationObservationCustodyCommitFailureReason {
    /// The independent anchor did not prove the database predecessor.
    #[error("operation-observation anchor precondition failed before database mutation")]
    AnchorPrecondition(OperationCustodyAnchorFailureReason),
    /// The one allowed database commit did not establish exact custody.
    #[error(transparent)]
    Database(OperationCustodyDatabaseFailureReason),
}

/// Failed hard-domain anchoring with observation database custody retained.
#[derive(Debug, thiserror::Error)]
#[error("operation-observation anti-rollback anchor did not complete: {reason}")]
pub struct OperationObservationCustodyAnchorFailure {
    pending: Box<DatabaseCommittedOperationObservationAnchorPending>,
    reason: Box<OperationCustodyAnchorFailureReason>,
}

impl OperationObservationCustodyAnchorFailure {
    /// Borrow the exact hard-domain failure classification.
    #[must_use]
    pub fn reason(&self) -> &OperationCustodyAnchorFailureReason {
        self.reason.as_ref()
    }

    /// Borrow the exact database-committed candidate awaiting its anchor.
    pub(in crate::resident) const fn candidate(&self) -> &OperationCustodyCandidate {
        self.pending.candidate()
    }

    /// Recover anchor mutation only when prior evidence proves no advancement.
    ///
    /// # Errors
    ///
    /// Returns this unchanged failure after outcome uncertainty or any state
    /// other than the exact predecessor, preventing a blind anchor retry.
    pub fn try_into_retryable(
        self,
    ) -> Result<DatabaseCommittedOperationObservationAnchorPending, Self> {
        if anchor_retry_is_safe(self.reason.as_ref()) {
            Ok(*self.pending)
        } else {
            Err(self)
        }
    }

    /// Re-observe an anchor whose first post-advance observation was unavailable.
    ///
    /// The exact successor completes custody without another advance. The exact
    /// predecessor proves that the retained transition is still safe to attempt
    /// once. Any other observation or observation failure returns this unchanged
    /// capability; it cannot authorize a different transition.
    ///
    /// # Errors
    ///
    /// Returns the unchanged failure when this is not the failed-observation
    /// phase, when the later observation is unavailable or unlike the prepared
    /// transition, or when the predecessor-authorized advance remains unresolved.
    pub(in crate::resident) fn continue_after_failed_observation(
        self,
        port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedOperationObservationCustody, Self> {
        if !matches!(
            self.reason.as_ref(),
            OperationCustodyAnchorFailureReason::AmbiguousObservationFailed { .. }
        ) {
            return Err(self);
        }
        let observation = match port
            .observe(&self.pending.anchor)
            .map(|observation| classify_ahead(&self.pending.anchor, observation))
        {
            Ok(observation) => observation,
            Err(_) => return Err(self),
        };
        match observation {
            AntiRollbackAnchorObservation::ExactSuccessor => {
                let pending = *self.pending;
                Ok(CompletedOperationObservationCustody {
                    candidate: pending.candidate,
                    database_evidence: pending.database_evidence,
                    anchor_evidence:
                        OperationCustodyAnchorEvidence::RecoveredExactAfterAmbiguousAdvance,
                })
            }
            AntiRollbackAnchorObservation::ExactPredecessor => {
                let pending = *self.pending;
                let anchor_evidence = match advance_operation_anchor(port, &pending.anchor) {
                    Ok(evidence) => evidence,
                    Err(reason) => {
                        return Err(Self {
                            pending: Box::new(pending),
                            reason: Box::new(reason),
                        });
                    }
                };
                Ok(CompletedOperationObservationCustody {
                    candidate: pending.candidate,
                    database_evidence: pending.database_evidence,
                    anchor_evidence,
                })
            }
            AntiRollbackAnchorObservation::Missing
            | AntiRollbackAnchorObservation::Behind { .. }
            | AntiRollbackAnchorObservation::Ahead { .. }
            | AntiRollbackAnchorObservation::Conflicting { .. } => Err(self),
        }
    }
}

/// Failed hard-domain anchoring with database custody retained.
#[derive(Debug, thiserror::Error)]
#[error("operation-start anti-rollback anchor did not complete: {reason}")]
pub struct OperationStartAnchorFailure {
    pending: Box<DatabaseCommittedOperationStartAnchorPending>,
    reason: Box<OperationCustodyAnchorFailureReason>,
}

impl OperationStartAnchorFailure {
    /// Borrow the exact hard-domain failure classification.
    #[must_use]
    pub fn reason(&self) -> &OperationCustodyAnchorFailureReason {
        self.reason.as_ref()
    }

    /// Recover anchor mutation only when prior evidence proves no advancement.
    ///
    /// # Errors
    ///
    /// Returns this unchanged failure after outcome uncertainty or any state
    /// other than the exact predecessor, preventing a blind anchor retry.
    pub fn try_into_retryable(self) -> Result<DatabaseCommittedOperationStartAnchorPending, Self> {
        if anchor_retry_is_safe(self.reason.as_ref()) {
            Ok(*self.pending)
        } else {
            Err(self)
        }
    }

    /// Re-observe an anchor when the first post-advance observation failed.
    ///
    /// The exact successor completes custody without another advance. The exact
    /// predecessor permits one advance of the retained transition. Missing,
    /// behind, ahead, conflicting, or unavailable state remains fenced.
    pub(in crate::resident) fn continue_after_failed_observation(
        self,
        port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedOperationStartCustody, Self> {
        if !matches!(
            self.reason.as_ref(),
            OperationCustodyAnchorFailureReason::AmbiguousObservationFailed { .. }
        ) {
            return Err(self);
        }
        let observation = match port
            .observe(&self.pending.anchor)
            .map(|observation| classify_ahead(&self.pending.anchor, observation))
        {
            Ok(observation) => observation,
            Err(_) => return Err(self),
        };
        match observation {
            AntiRollbackAnchorObservation::ExactSuccessor => {
                let pending = *self.pending;
                Ok(CompletedOperationStartCustody {
                    candidate: pending.candidate,
                    database_evidence: pending.database_evidence,
                    anchor_evidence:
                        OperationCustodyAnchorEvidence::RecoveredExactAfterAmbiguousAdvance,
                })
            }
            AntiRollbackAnchorObservation::ExactPredecessor => {
                let pending = *self.pending;
                let anchor_evidence = match advance_operation_anchor(port, &pending.anchor) {
                    Ok(evidence) => evidence,
                    Err(reason) => return Err(pending.failed(reason)),
                };
                Ok(CompletedOperationStartCustody {
                    candidate: pending.candidate,
                    database_evidence: pending.database_evidence,
                    anchor_evidence,
                })
            }
            AntiRollbackAnchorObservation::Missing
            | AntiRollbackAnchorObservation::Behind { .. }
            | AntiRollbackAnchorObservation::Ahead { .. }
            | AntiRollbackAnchorObservation::Conflicting { .. } => Err(self),
        }
    }
}

fn anchor_retry_is_safe(reason: &OperationCustodyAnchorFailureReason) -> bool {
    match reason {
        OperationCustodyAnchorFailureReason::Port(error) => {
            error.kind() == pale_ports::AntiRollbackAnchorErrorKind::Unavailable
        }
        OperationCustodyAnchorFailureReason::NotAdvanced(observation)
        | OperationCustodyAnchorFailureReason::AmbiguousNotExact { observation, .. } => {
            matches!(
                observation.as_ref(),
                AntiRollbackAnchorObservation::ExactPredecessor
            )
        }
        OperationCustodyAnchorFailureReason::AmbiguousObservationFailed { .. } => false,
    }
}

/// Closed hard-domain failure classification.
#[derive(Debug, thiserror::Error)]
pub enum OperationCustodyAnchorFailureReason {
    /// The anchor was unavailable before outcome ambiguity.
    #[error(transparent)]
    Port(AntiRollbackAnchorError),
    /// The independently retained state did not prove this checkpoint.
    #[error("operation-custody anchor did not match the prepared comparison")]
    NotAdvanced(Box<AntiRollbackAnchorObservation>),
    /// An ambiguous advance did not observe the exact or a later successor.
    #[error("ambiguous operation-custody anchor advance remained non-successor")]
    AmbiguousNotExact {
        /// Original outcome-unknown anchor error.
        uncertain: AntiRollbackAnchorError,
        /// Non-successor state returned by the sole observation.
        observation: Box<AntiRollbackAnchorObservation>,
    },
    /// The sole observation failed after an ambiguous anchor advance.
    #[error("operation-custody anchor observation failed after ambiguous advance")]
    AmbiguousObservationFailed {
        /// Original outcome-unknown anchor error.
        uncertain: AntiRollbackAnchorError,
        /// Observation failure preventing classification.
        #[source]
        observe: AntiRollbackAnchorError,
    },
}

impl From<DatabaseCommitEvidence> for OperationCustodyDatabaseEvidence {
    fn from(value: DatabaseCommitEvidence) -> Self {
        match value {
            DatabaseCommitEvidence::New => Self::New,
            DatabaseCommitEvidence::ExactReplay => Self::ExactReplay,
            DatabaseCommitEvidence::RecoveredExactAfterAmbiguousCommit => {
                Self::RecoveredExactAfterAmbiguousCommit
            }
        }
    }
}

impl From<DatabaseCommitFailureReason> for OperationCustodyDatabaseFailureReason {
    fn from(value: DatabaseCommitFailureReason) -> Self {
        match value {
            DatabaseCommitFailureReason::Store(error) => Self::Store(error),
            DatabaseCommitFailureReason::CommitExcludedByReadBack { uncertain } => {
                Self::CommitExcludedByReadBack { uncertain }
            }
            DatabaseCommitFailureReason::RecoveryRequired { uncertain } => {
                Self::RecoveryRequired { uncertain }
            }
            DatabaseCommitFailureReason::ReadBackFailed {
                uncertain,
                read_back,
            } => Self::ReadBackFailed {
                uncertain,
                read_back,
            },
        }
    }
}

impl From<AnchorAdvanceEvidence> for OperationCustodyAnchorEvidence {
    fn from(value: AnchorAdvanceEvidence) -> Self {
        match value {
            AnchorAdvanceEvidence::Advanced => Self::Advanced,
            AnchorAdvanceEvidence::ExactReplay => Self::ExactReplay,
            AnchorAdvanceEvidence::RecoveredExactAfterAmbiguousAdvance => {
                Self::RecoveredExactAfterAmbiguousAdvance
            }
        }
    }
}

impl From<AnchorAdvanceFailureReason> for OperationCustodyAnchorFailureReason {
    fn from(value: AnchorAdvanceFailureReason) -> Self {
        match value {
            AnchorAdvanceFailureReason::Port(error) => Self::Port(error),
            AnchorAdvanceFailureReason::NotAdvanced(observation) => Self::NotAdvanced(observation),
            AnchorAdvanceFailureReason::AmbiguousNotExact {
                uncertain,
                observation,
            } => Self::AmbiguousNotExact {
                uncertain,
                observation,
            },
            AnchorAdvanceFailureReason::AmbiguousObservationFailed { uncertain, observe } => {
                Self::AmbiguousObservationFailed { uncertain, observe }
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/operation_custody.rs"]
mod tests;
