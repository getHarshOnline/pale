//! Application-owned principal-bootstrap custody preparation and state transition.
//!
//! [`prepare_principal_bootstrap_custody`] is the sole projection from
//! [`AdmittedPrincipalBootstrapV1::atom_view`] into the neutral physical
//! candidate consumed by [`PrincipalBootstrapCustodyPort`]. The custody
//! lifecycle is deliberately monotone:
//!
//! [`PrincipalBootstrapCustodyPhase`] exposes the three monotone phases:

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/principal-bootstrap-custody.display.mathml"
))]
//!
//! Database commitment precedes every external anchor advance. Complete custody
//! requires the principal trust minimum and every admitted initial control
//! checkpoint. This historical evidence does not establish current authority
//! or enable resident IPC.
use pale_core::canon;
use pale_core::records::AdmittedPrincipalBootstrapV1;
use pale_ports::{
    ControlRecordError, ControlStoreError, ControlStoreErrorKind, PreparedPrincipalBootstrapAtom,
    PrincipalBootstrapCandidateError, PrincipalBootstrapCommitDisposition,
    PrincipalBootstrapControlCheckpoint, PrincipalBootstrapCustodyPort,
    PrincipalBootstrapDatabaseObservation, PrincipalBootstrapFixedRecords, PrincipalBootstrapHead,
    PrincipalTrustCheckpointHead, PrincipalTrustStoreBinding,
};

use super::anchor_progress::{AnchorProgress, ObservationReason};
use super::record_projection::{self, ControlRecordProjectionError};
use pale_ports::{
    ControlDomainAntiRollbackAnchor, PreparedAntiRollbackAnchorTransition,
    PrincipalTrustMinimumAnchor,
};

mod anchors;
pub use anchors::{
    CompletedPrincipalBootstrapCustody, PrincipalBootstrapAnchorFailure,
    PrincipalBootstrapAnchorFailureReason,
};

/// Named phase of the principal-bootstrap custody lifecycle.
///
/// A phase is an application classification, not proof on its own. Callers
/// obtain these phases through preparation, database commit, and
/// [`DatabaseCommittedPrincipalBootstrapAnchorsPending::advance_anchors`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalBootstrapCustodyPhase {
    /// Signed records are admitted and projected, but no database commit is proven.
    Prepared,
    /// Exact database custody is proven; external anchors still need advancement.
    DatabaseCommittedAnchorsPending,
    /// Database custody and all required external anchors are both proven.
    Complete,
}

/// Successful evidence for the database portion of bootstrap custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalBootstrapCommitEvidence {
    /// The adapter committed the candidate into an empty store.
    New,
    /// The adapter proved the store already contained the exact candidate.
    ExactReplay,
    /// A commit was ambiguous and mandatory exact read-back found the candidate.
    RecoveredExactAfterAmbiguousCommit,
}

/// Prepared, semantically admitted atom that has not yet acquired database proof.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "prepared bootstrap custody must be committed or retained for explicit recovery"]
pub struct PreparedPrincipalBootstrapCustody {
    candidate: PreparedPrincipalBootstrapAtom,
    anchors: Vec<PreparedAntiRollbackAnchorTransition>,
}

impl PreparedPrincipalBootstrapCustody {
    /// Return the only phase represented by this type.
    #[must_use]
    pub const fn phase(&self) -> PrincipalBootstrapCustodyPhase {
        PrincipalBootstrapCustodyPhase::Prepared
    }

    /// Borrow the closed physical candidate without granting semantic construction.
    #[must_use]
    pub const fn candidate(&self) -> &PreparedPrincipalBootstrapAtom {
        &self.candidate
    }

    /// Consume the prepared state and attempt the database-atomic transition.
    ///
    /// An ambiguous commit is never retried. This method immediately invokes
    /// [`PrincipalBootstrapCustodyPort::observe_exact`]: exact equality advances
    /// to [`DatabaseCommittedPrincipalBootstrapAnchorsPending`], an empty store
    /// returns the original prepared state, and conflicting or unreadable state
    /// fails closed for explicit recovery.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalBootstrapCommitFailure`] with the complete prepared
    /// state when the adapter rejects the candidate or exact read-back does not
    /// prove it committed.
    pub fn commit(
        self,
        store: &mut dyn PrincipalBootstrapCustodyPort,
    ) -> Result<DatabaseCommittedPrincipalBootstrapAnchorsPending, PrincipalBootstrapCommitFailure>
    {
        match store.compare_and_commit(&self.candidate) {
            Ok(disposition) => Ok(self.database_committed(match disposition {
                PrincipalBootstrapCommitDisposition::New => PrincipalBootstrapCommitEvidence::New,
                PrincipalBootstrapCommitDisposition::ExactReplay => {
                    PrincipalBootstrapCommitEvidence::ExactReplay
                }
            })),
            Err(error) if error.kind() == ControlStoreErrorKind::OutcomeUnknown => {
                match store.observe_exact(&self.candidate) {
                    Ok(PrincipalBootstrapDatabaseObservation::Exact) => Ok(self
                        .database_committed(
                            PrincipalBootstrapCommitEvidence::RecoveredExactAfterAmbiguousCommit,
                        )),
                    Ok(PrincipalBootstrapDatabaseObservation::Empty) => Err(self.failed(
                        PrincipalBootstrapCommitFailureReason::CommitExcludedByExactReadBack {
                            uncertain: error,
                        },
                    )),
                    Ok(PrincipalBootstrapDatabaseObservation::ConflictingOrPartial) => Err(self
                        .failed(
                        PrincipalBootstrapCommitFailureReason::RecoveryRequiredAfterExactReadBack {
                            uncertain: error,
                        },
                    )),
                    Err(read_back) => Err(self.failed(
                        PrincipalBootstrapCommitFailureReason::ExactReadBackFailed {
                            uncertain: error,
                            read_back,
                        },
                    )),
                }
            }
            Err(error) => Err(self.failed(PrincipalBootstrapCommitFailureReason::Store(error))),
        }
    }

    /// Move the closed candidate into database-proven, anchor-pending state.
    fn database_committed(
        self,
        evidence: PrincipalBootstrapCommitEvidence,
    ) -> DatabaseCommittedPrincipalBootstrapAnchorsPending {
        DatabaseCommittedPrincipalBootstrapAnchorsPending {
            candidate: self.candidate,
            evidence,
            anchors: self
                .anchors
                .into_iter()
                .map(|anchor| {
                    let progress = match evidence {
                        PrincipalBootstrapCommitEvidence::New => AnchorProgress::Ready,
                        PrincipalBootstrapCommitEvidence::ExactReplay
                        | PrincipalBootstrapCommitEvidence::RecoveredExactAfterAmbiguousCommit => {
                            AnchorProgress::ObserveFirst(ObservationReason::DatabaseReplay)
                        }
                    };
                    (anchor, progress)
                })
                .collect(),
        }
    }

    /// Retain the prepared state beside a failed transition reason.
    fn failed(
        self,
        reason: PrincipalBootstrapCommitFailureReason,
    ) -> PrincipalBootstrapCommitFailure {
        PrincipalBootstrapCommitFailure {
            prepared: Box::new(self),
            reason,
        }
    }

    /// Construct the prepared state after the owning projection validates closure.
    fn from_candidate(
        candidate: PreparedPrincipalBootstrapAtom,
        anchors: Vec<PreparedAntiRollbackAnchorTransition>,
    ) -> Self {
        Self { candidate, anchors }
    }
}

/// Exact database custody with external anti-rollback anchors still pending.
///
/// This value is not resident admission, runtime readiness, or proof that an
/// external anchor advanced. [`Self::advance_anchors`] completes the retained
/// initial anchors through the separately selected external mechanism.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "database custody remains incomplete until external anchors are proven"]
pub struct DatabaseCommittedPrincipalBootstrapAnchorsPending {
    candidate: PreparedPrincipalBootstrapAtom,
    evidence: PrincipalBootstrapCommitEvidence,
    anchors: Vec<(PreparedAntiRollbackAnchorTransition, AnchorProgress)>,
}

impl DatabaseCommittedPrincipalBootstrapAnchorsPending {
    /// Return the only phase represented by this type.
    #[must_use]
    pub const fn phase(&self) -> PrincipalBootstrapCustodyPhase {
        PrincipalBootstrapCustodyPhase::DatabaseCommittedAnchorsPending
    }

    /// Return how exact physical database custody was established.
    #[must_use]
    pub const fn evidence(&self) -> PrincipalBootstrapCommitEvidence {
        self.evidence
    }

    /// Borrow the immutable committed candidate without asserting anchor custody.
    #[must_use]
    pub const fn candidate(&self) -> &PreparedPrincipalBootstrapAtom {
        &self.candidate
    }
}

/// Failed database transition with the complete prepared state retained.
#[derive(Debug, thiserror::Error)]
#[error("principal bootstrap custody did not advance: {reason}")]
pub struct PrincipalBootstrapCommitFailure {
    prepared: Box<PreparedPrincipalBootstrapCustody>,
    reason: PrincipalBootstrapCommitFailureReason,
}

impl PrincipalBootstrapCommitFailure {
    /// Borrow the exact reason the transition remained unproven.
    #[must_use]
    pub const fn reason(&self) -> &PrincipalBootstrapCommitFailureReason {
        &self.reason
    }

    /// Recover the complete prepared state for explicit diagnosis or recovery.
    pub fn into_prepared(self) -> PreparedPrincipalBootstrapCustody {
        *self.prepared
    }
}

/// Reason a prepared bootstrap candidate did not gain database proof.
#[derive(Debug, thiserror::Error)]
pub enum PrincipalBootstrapCommitFailureReason {
    /// The adapter rejected the candidate before an ambiguous commit boundary.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// Exact read-back after ambiguity proved the candidate did not commit.
    #[error("exact read-back found an empty bootstrap store")]
    CommitExcludedByExactReadBack {
        /// Native uncertainty returned by the attempted database commit.
        #[source]
        uncertain: ControlStoreError,
    },
    /// Exact read-back found partial rows or another complete candidate.
    #[error("exact read-back requires explicit bootstrap recovery")]
    RecoveryRequiredAfterExactReadBack {
        /// Native uncertainty returned by the attempted database commit.
        #[source]
        uncertain: ControlStoreError,
    },
    /// Mandatory exact read-back itself could not complete.
    #[error("exact bootstrap read-back failed")]
    ExactReadBackFailed {
        /// Native uncertainty returned by the attempted database commit.
        uncertain: ControlStoreError,
        /// Native failure returned by mandatory exact read-back.
        #[source]
        read_back: ControlStoreError,
    },
}

/// Failure to project one admitted bootstrap atom into physical custody.
#[derive(Debug, thiserror::Error)]
pub enum PrincipalBootstrapProjectionError {
    /// An admitted initial checkpoint did not begin at the required position.
    #[error(transparent)]
    AnchorTransition(#[from] pale_ports::AntiRollbackAnchorTransitionError),
    /// Canonical signed-record encoding failed.
    #[error("canonicalize admitted principal bootstrap record")]
    Canonicalize(#[from] canon::CanonError),
    /// The checkpoint set cannot be represented by the port's bounded ordinal.
    #[error("principal bootstrap checkpoint count exceeds the initial-v1 physical coordinate")]
    CheckpointCountOutOfRange,
    /// The projected physical candidate contradicted its admitted closure.
    #[error(transparent)]
    Candidate(#[from] PrincipalBootstrapCandidateError),
    /// A shared physical record coordinate or canonical byte string is malformed.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
}

impl From<ControlRecordProjectionError> for PrincipalBootstrapProjectionError {
    fn from(error: ControlRecordProjectionError) -> Self {
        match error {
            ControlRecordProjectionError::Canonicalize(source) => Self::Canonicalize(source),
            ControlRecordProjectionError::Record(source) => Self::Record(source),
        }
    }
}

/// Project one admitted bootstrap atom into the sole physical custody shape.
///
/// Every signed record is encoded once as canonical JCS, and the port candidate
/// closes over the exact references and family-specific digests already proven
/// by [`AdmittedPrincipalBootstrapV1`]. No stored bytes are re-admitted here.
///
/// # Errors
///
/// Returns [`PrincipalBootstrapProjectionError`] if canonical encoding fails,
/// the checkpoint count exceeds `u32`, or the projected physical closure is
/// internally contradictory.
pub fn prepare_principal_bootstrap_custody(
    admitted: &AdmittedPrincipalBootstrapV1,
    accumulators: Vec<pale_ports::OperationAccumulatorBatch>,
) -> Result<PreparedPrincipalBootstrapCustody, PrincipalBootstrapProjectionError> {
    let view = admitted.atom_view();
    let authorities = view.authorities();
    let genesis = authorities.genesis();
    let principal = genesis.principal_record();
    let recovery = genesis.recovery_policy();
    let admin = authorities.local_admin_authority();
    let producer = authorities.producer_authority();
    let checkpoint_authority = authorities.checkpoint_authority();
    let (trust_ref, trust) = view.trust_checkpoint_entry();
    let (receipt_ref, receipt) = view.receipt_entry();

    let principal_coordinate = record_projection::coordinate(
        genesis.principal_record_ref(),
        principal.canonical_record_sha256().as_str(),
    )?;
    let recovery_coordinate = record_projection::coordinate(
        genesis.recovery_policy_ref(),
        recovery.canonical_record_sha256().as_str(),
    )?;
    let admin_coordinate = record_projection::coordinate(
        authorities.local_admin_authority_ref(),
        admin.canonical_admin_authority_sha256().as_str(),
    )?;
    let producer_coordinate = record_projection::coordinate(
        authorities.producer_authority_ref(),
        producer.canonical_producer_authority_sha256().as_str(),
    )?;
    let checkpoint_authority_coordinate = record_projection::coordinate(
        authorities.checkpoint_authority_ref(),
        checkpoint_authority
            .canonical_checkpoint_signing_authority_sha256()
            .as_str(),
    )?;
    let trust_coordinate = record_projection::coordinate(
        trust_ref,
        trust.canonical_trust_checkpoint_sha256().as_str(),
    )?;
    let mut anchors = vec![PreparedAntiRollbackAnchorTransition::initial(
        PrincipalTrustMinimumAnchor::new(
            trust.principal_id().clone(),
            *trust.checkpoint_epoch(),
            trust_coordinate.clone(),
        )
        .into(),
    )?];
    let receipt_coordinate = record_projection::coordinate(
        receipt_ref,
        receipt.canonical_bootstrap_receipt_sha256().as_str(),
    )?;

    let mut records = vec![
        record_projection::record(principal_coordinate.clone(), principal)?,
        record_projection::record(recovery_coordinate.clone(), recovery)?,
        record_projection::record(admin_coordinate.clone(), admin)?,
        record_projection::record(producer_coordinate.clone(), producer)?,
        record_projection::record(
            checkpoint_authority_coordinate.clone(),
            checkpoint_authority,
        )?,
        record_projection::record(trust_coordinate.clone(), trust)?,
    ];
    let mut checkpoints = Vec::with_capacity(view.initial_control_checkpoints().len());
    for (index, (record_ref, checkpoint)) in view.initial_control_checkpoints().enumerate() {
        let ordinal = u32::try_from(index + 1)
            .map_err(|_| PrincipalBootstrapProjectionError::CheckpointCountOutOfRange)?;
        let checkpoint_coordinate = record_projection::coordinate(
            record_ref,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )?;
        anchors.push(PreparedAntiRollbackAnchorTransition::initial(
            ControlDomainAntiRollbackAnchor::new(
                checkpoint.control_store_id().clone(),
                checkpoint.control_incarnation_id().clone(),
                checkpoint.checkpoint_domain_id().clone(),
                *checkpoint.domain_epoch(),
                *checkpoint.domain_sequence(),
                checkpoint_coordinate.clone(),
            )
            .into(),
        )?);
        records.push(record_projection::record(
            checkpoint_coordinate.clone(),
            checkpoint,
        )?);
        checkpoints.push(PrincipalBootstrapControlCheckpoint::new(
            ordinal,
            checkpoint.checkpoint_domain_id().clone(),
            *checkpoint.domain_epoch(),
            *checkpoint.domain_sequence(),
            *checkpoint.committed_transaction_sequence(),
            checkpoint_coordinate,
        )?);
    }
    records.push(record_projection::record(
        receipt_coordinate.clone(),
        receipt,
    )?);

    let control_checkpoint_count = u32::try_from(checkpoints.len())
        .map_err(|_| PrincipalBootstrapProjectionError::CheckpointCountOutOfRange)?;
    let head = PrincipalBootstrapHead::new(
        receipt.bootstrap_id().clone(),
        PrincipalBootstrapFixedRecords::new(
            principal_coordinate,
            recovery_coordinate,
            admin_coordinate,
            producer_coordinate,
            checkpoint_authority_coordinate,
            receipt_coordinate,
        ),
        PrincipalTrustCheckpointHead::new(
            trust.principal_id().clone(),
            *trust.checkpoint_epoch(),
            trust_coordinate,
        ),
        control_checkpoint_count,
        *receipt.committed_at(),
    );
    let binding = PrincipalTrustStoreBinding::new(
        receipt.trust_store_id().clone(),
        receipt.trust_store_incarnation_id().clone(),
        admitted.principal_id().clone(),
    );
    PreparedPrincipalBootstrapAtom::new(binding, records, checkpoints, head, accumulators)
        .map(|candidate| PreparedPrincipalBootstrapCustody::from_candidate(candidate, anchors))
        .map_err(Into::into)
}

#[cfg(test)]
#[path = "../../tests/internal/resident/bootstrap.rs"]
mod tests;
