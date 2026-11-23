//! Separate physical custody boundaries for deployment finalization and genesis.
//!
//! The principal trust store commits finalization first: the authenticated local
//! administration command, binding, two delegated authorities, successor trust
//! checkpoint, principal control checkpoint, administration receipt, and the
//! matching successor accumulator batch. Only after exact custody of that atom
//! may the application form a deployment-control-store genesis atom. Genesis is
//! a different database transaction in a different store and contains only its
//! initial control checkpoints, their physical accumulator batches, and the
//! genesis receipt record.
//!
//! These neutral types select no administrator signer, key store, or external
//! anti-rollback anchor. They retain the exact identities required by the owner
//! of those decisions and make a cross-database pretend transaction impossible.

use std::collections::BTreeSet;

use pale_core::records::{
    ControlIncarnationIdV1, ControlStoreIdV1, DeploymentControlStoreGenesisIdV1,
    LocalAdminCommandIdV1, LocalAdminIdempotencyKeyV1, LocalAdminReceiptIdV1,
    ResidentPrincipalIdV1, WorkspaceId, CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA, DEPLOYMENT_BINDING_V1_SCHEMA,
    DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA, DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA,
    LOCAL_ADMIN_COMMAND_V1_SCHEMA, LOCAL_ADMIN_RECEIPT_V1_SCHEMA, MAX_RESIDENT_SET_MEMBERS,
    PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
};

use crate::{
    AccumulatorCheckpointPhase, AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition,
    AntiRollbackAnchorError, AntiRollbackAnchorErrorKind, AntiRollbackAnchorObservation,
    AntiRollbackAnchorPort, AuthorityStoreCoordinate, ControlCheckpointHead,
    ControlDomainAntiRollbackAnchor, ControlRecord, ControlRecordCoordinate, ControlStoreError,
    ControlStoreErrorKind, OperationAccumulatorBatch, PreparedAntiRollbackAnchorTransition,
    PrincipalTrustCheckpointHead, PrincipalTrustMinimumAnchor, PrincipalTrustStoreBinding,
};

/// Exact administration replay identity for the finalization transaction.
///
/// The tuple is scoped by the active administrator authority. Reuse with
/// different candidate bytes is a conflict; exact reuse returns the stored
/// receipt instead of re-running finalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentFinalizationReplayIdentity {
    admin_command_id: LocalAdminCommandIdV1,
    idempotency_key: LocalAdminIdempotencyKeyV1,
    admin_authority: ControlRecordCoordinate,
    command: ControlRecordCoordinate,
    receipt_id: LocalAdminReceiptIdV1,
}

impl DeploymentFinalizationReplayIdentity {
    /// Retain the command, authority, idempotency key, and stable receipt identity.
    #[must_use]
    pub const fn new(
        admin_command_id: LocalAdminCommandIdV1,
        idempotency_key: LocalAdminIdempotencyKeyV1,
        admin_authority: ControlRecordCoordinate,
        command: ControlRecordCoordinate,
        receipt_id: LocalAdminReceiptIdV1,
    ) -> Self {
        Self {
            admin_command_id,
            idempotency_key,
            admin_authority,
            command,
            receipt_id,
        }
    }

    /// Borrow the stable command identity.
    #[must_use]
    pub const fn admin_command_id(&self) -> &LocalAdminCommandIdV1 {
        &self.admin_command_id
    }

    /// Borrow the authority-scoped replay key.
    #[must_use]
    pub const fn idempotency_key(&self) -> &LocalAdminIdempotencyKeyV1 {
        &self.idempotency_key
    }

    /// Borrow the selected administrator authority coordinate.
    #[must_use]
    pub const fn admin_authority(&self) -> &ControlRecordCoordinate {
        &self.admin_authority
    }

    /// Borrow the immutable local-administrator command coordinate.
    #[must_use]
    pub const fn command(&self) -> &ControlRecordCoordinate {
        &self.command
    }

    /// Borrow the stable receipt identity returned on exact replay.
    #[must_use]
    pub const fn receipt_id(&self) -> &LocalAdminReceiptIdV1 {
        &self.receipt_id
    }
}

/// The seven exact signed records co-committed by deployment finalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentFinalizationRecords {
    command: ControlRecord,
    binding: ControlRecord,
    checkpoint_signing_authority: ControlRecord,
    runtime_authority: ControlRecord,
    trust_checkpoint: ControlRecord,
    control_checkpoint: ControlRecord,
    receipt: ControlRecord,
}

impl DeploymentFinalizationRecords {
    /// Retain one record for each initial-v1 finalization role.
    ///
    /// # Errors
    /// Rejects a wrong family or a reused immutable record reference.
    pub fn new(
        command: ControlRecord,
        binding: ControlRecord,
        checkpoint_signing_authority: ControlRecord,
        runtime_authority: ControlRecord,
        trust_checkpoint: ControlRecord,
        control_checkpoint: ControlRecord,
        receipt: ControlRecord,
    ) -> Result<Self, DeploymentFinalizationCandidateError> {
        let records = [
            (&command, LOCAL_ADMIN_COMMAND_V1_SCHEMA),
            (&binding, DEPLOYMENT_BINDING_V1_SCHEMA),
            (
                &checkpoint_signing_authority,
                CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA,
            ),
            (&runtime_authority, DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA),
            (&trust_checkpoint, PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA),
            (&control_checkpoint, CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
            (&receipt, LOCAL_ADMIN_RECEIPT_V1_SCHEMA),
        ];
        if records
            .iter()
            .any(|(record, family)| record.schema_id().as_str() != *family)
        {
            return Err(DeploymentFinalizationCandidateError::RecordFamily);
        }
        let references = records
            .iter()
            .map(|(record, _)| record.coordinate().record_ref())
            .collect::<BTreeSet<_>>();
        if references.len() != records.len() {
            return Err(DeploymentFinalizationCandidateError::ReferenceClosure);
        }
        Ok(Self {
            command,
            binding,
            checkpoint_signing_authority,
            runtime_authority,
            trust_checkpoint,
            control_checkpoint,
            receipt,
        })
    }

    /// Borrow the authenticated administration command row.
    #[must_use]
    pub const fn command(&self) -> &ControlRecord {
        &self.command
    }

    /// Borrow the finalized deployment binding row.
    #[must_use]
    pub const fn binding(&self) -> &ControlRecord {
        &self.binding
    }

    /// Borrow the deployment checkpoint-signer delegation row.
    #[must_use]
    pub const fn checkpoint_signing_authority(&self) -> &ControlRecord {
        &self.checkpoint_signing_authority
    }

    /// Borrow the deployment runtime-authority delegation row.
    #[must_use]
    pub const fn runtime_authority(&self) -> &ControlRecord {
        &self.runtime_authority
    }

    /// Borrow the successor principal-trust checkpoint row.
    #[must_use]
    pub const fn trust_checkpoint(&self) -> &ControlRecord {
        &self.trust_checkpoint
    }

    /// Borrow the successor principal control-checkpoint row.
    #[must_use]
    pub const fn control_checkpoint(&self) -> &ControlRecord {
        &self.control_checkpoint
    }

    /// Borrow the local-administration outcome row.
    #[must_use]
    pub const fn receipt(&self) -> &ControlRecord {
        &self.receipt
    }
}

/// Closed physical candidate for transaction one in deployment initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedDeploymentFinalizationAtom {
    binding: PrincipalTrustStoreBinding,
    replay: DeploymentFinalizationReplayIdentity,
    records: DeploymentFinalizationRecords,
    predecessor_trust: PrincipalTrustCheckpointHead,
    successor_trust: PrincipalTrustCheckpointHead,
    predecessor_control: ControlCheckpointHead,
    successor_control: ControlCheckpointHead,
    accumulator: OperationAccumulatorBatch,
    trust_anchor: PreparedAntiRollbackAnchorTransition,
    control_anchor: PreparedAntiRollbackAnchorTransition,
}

impl PreparedDeploymentFinalizationAtom {
    /// Close exact records, replay identity, heads, and tree mutation over one trust store.
    ///
    /// # Errors
    /// Rejects unlike principal/store scope, non-successor heads, substituted record
    /// coordinates, or an accumulator prepared for another transaction.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        binding: PrincipalTrustStoreBinding,
        replay: DeploymentFinalizationReplayIdentity,
        records: DeploymentFinalizationRecords,
        predecessor_trust: PrincipalTrustCheckpointHead,
        successor_trust: PrincipalTrustCheckpointHead,
        predecessor_control: ControlCheckpointHead,
        successor_control: ControlCheckpointHead,
        accumulator: OperationAccumulatorBatch,
    ) -> Result<Self, DeploymentFinalizationCandidateError> {
        if binding.principal_id() != predecessor_trust.principal_id()
            || binding.principal_id() != successor_trust.principal_id()
            || !successor_trust.is_exact_successor_of(&predecessor_trust)
            || !successor_control.is_exact_successor_of(&predecessor_control)
        {
            return Err(DeploymentFinalizationCandidateError::Frontier);
        }
        if records.command().coordinate() != replay.command()
            || records.trust_checkpoint().coordinate() != successor_trust.coordinate()
            || records.control_checkpoint().coordinate() != successor_control.coordinate()
        {
            return Err(DeploymentFinalizationCandidateError::ReferenceClosure);
        }
        let accumulator_binding = accumulator.binding();
        let store = accumulator_binding.store();
        if store.store_id() != binding.trust_store_id()
            || store.incarnation_id() != binding.trust_store_incarnation_id()
            || accumulator_binding.successor() != &successor_control
            || !matches!(
                accumulator_binding.phase(),
                AccumulatorCheckpointPhase::Successor(previous) if previous == &predecessor_control
            )
        {
            return Err(DeploymentFinalizationCandidateError::AccumulatorClosure);
        }
        let trust_anchor = PreparedAntiRollbackAnchorTransition::successor(
            AntiRollbackAnchor::from(principal_anchor(&predecessor_trust)),
            AntiRollbackAnchor::from(principal_anchor(&successor_trust)),
        )
        .map_err(|_| DeploymentFinalizationCandidateError::AnchorClosure)?;
        let control_anchor = PreparedAntiRollbackAnchorTransition::successor(
            AntiRollbackAnchor::from(control_anchor(&binding, &predecessor_control)),
            AntiRollbackAnchor::from(control_anchor(&binding, &successor_control)),
        )
        .map_err(|_| DeploymentFinalizationCandidateError::AnchorClosure)?;
        Ok(Self {
            binding,
            replay,
            records,
            predecessor_trust,
            successor_trust,
            predecessor_control,
            successor_control,
            accumulator,
            trust_anchor,
            control_anchor,
        })
    }

    /// Borrow the physical principal-store binding.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        &self.binding
    }

    /// Borrow the replay identity that must be compared before mutation.
    #[must_use]
    pub const fn replay(&self) -> &DeploymentFinalizationReplayIdentity {
        &self.replay
    }

    /// Borrow the exact seven-record atom.
    #[must_use]
    pub const fn records(&self) -> &DeploymentFinalizationRecords {
        &self.records
    }

    /// Borrow the prior principal trust head.
    #[must_use]
    pub const fn predecessor_trust(&self) -> &PrincipalTrustCheckpointHead {
        &self.predecessor_trust
    }

    /// Borrow the committed candidate principal trust head.
    #[must_use]
    pub const fn successor_trust(&self) -> &PrincipalTrustCheckpointHead {
        &self.successor_trust
    }

    /// Borrow the prior principal hard-domain head.
    #[must_use]
    pub const fn predecessor_control(&self) -> &ControlCheckpointHead {
        &self.predecessor_control
    }

    /// Borrow the candidate principal hard-domain head.
    #[must_use]
    pub const fn successor_control(&self) -> &ControlCheckpointHead {
        &self.successor_control
    }

    /// Borrow the exact physical tree mutation co-committed in transaction one.
    #[must_use]
    pub const fn accumulator(&self) -> &OperationAccumulatorBatch {
        &self.accumulator
    }

    /// Attempt transaction one and retain exact database proof.
    ///
    /// An ambiguous mutation is never retried. Mandatory exact read-back either
    /// proves this atom, excludes commitment while returning it for a deliberate
    /// retry, or fails closed for recovery.
    ///
    /// # Errors
    /// Returns the complete candidate beside the physical failure or read-back
    /// classification whenever exact database commitment is not established.
    pub fn commit(
        self,
        store: &mut dyn DeploymentFinalizationCustodyPort,
    ) -> Result<DatabaseCommittedDeploymentFinalization, DeploymentFinalizationCommitFailure> {
        match store.compare_and_commit(&self) {
            Ok(DeploymentFinalizationCommitDisposition::New) => {
                Ok(self.database_committed(DeploymentFinalizationCommitEvidence::New))
            }
            Ok(DeploymentFinalizationCommitDisposition::ExactReplay) => {
                Ok(self.database_committed(DeploymentFinalizationCommitEvidence::ExactReplay))
            }
            Err(error) if error.kind() == ControlStoreErrorKind::OutcomeUnknown => {
                match store.observe_exact(&self) {
                    Ok(DeploymentFinalizationDatabaseObservation::Exact) => Ok(self
                        .database_committed(
                            DeploymentFinalizationCommitEvidence::RecoveredExactAfterAmbiguousCommit,
                        )),
                    Ok(DeploymentFinalizationDatabaseObservation::Predecessor) => Err(self.failed(
                        DeploymentFinalizationCommitFailureReason::CommitExcludedByExactReadBack {
                            uncertain: error,
                        },
                    )),
                    Ok(DeploymentFinalizationDatabaseObservation::ConflictingOrPartial) => Err(
                        self.failed(
                            DeploymentFinalizationCommitFailureReason::RecoveryRequiredAfterExactReadBack {
                                uncertain: error,
                            },
                        ),
                    ),
                    Err(read_back) => Err(self.failed(
                        DeploymentFinalizationCommitFailureReason::ExactReadBackFailed {
                            uncertain: error,
                            read_back,
                        },
                    )),
                }
            }
            Err(error) => Err(self.failed(DeploymentFinalizationCommitFailureReason::Store(error))),
        }
    }

    fn database_committed(
        self,
        evidence: DeploymentFinalizationCommitEvidence,
    ) -> DatabaseCommittedDeploymentFinalization {
        DatabaseCommittedDeploymentFinalization {
            candidate: self,
            evidence,
        }
    }

    fn failed(
        self,
        reason: DeploymentFinalizationCommitFailureReason,
    ) -> DeploymentFinalizationCommitFailure {
        DeploymentFinalizationCommitFailure {
            candidate: Box::new(self),
            reason,
        }
    }

    fn dependency(&self) -> DeploymentFinalizationDependency {
        DeploymentFinalizationDependency {
            trust_store: AuthorityStoreCoordinate::new(
                self.binding.trust_store_id().clone(),
                self.binding.trust_store_incarnation_id().clone(),
            ),
            principal_id: self.binding.principal_id().clone(),
            binding: self.records.binding().coordinate().clone(),
            checkpoint_signing_authority: self
                .records
                .checkpoint_signing_authority()
                .coordinate()
                .clone(),
            runtime_authority: self.records.runtime_authority().coordinate().clone(),
            trust_checkpoint: self.successor_trust.clone(),
            administration_receipt: self.records.receipt().coordinate().clone(),
        }
    }
}

/// Exact transaction-one database custody awaiting independently anchored use.
///
/// This is not proof that the trust and principal-domain anchors advanced. The
/// application must complete those retained transitions before it submits the
/// dependency to transaction two.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "database-committed finalization still requires its external anchors"]
pub struct DatabaseCommittedDeploymentFinalization {
    candidate: PreparedDeploymentFinalizationAtom,
    evidence: DeploymentFinalizationCommitEvidence,
}

impl DatabaseCommittedDeploymentFinalization {
    /// Return how exact transaction-one database custody was established.
    #[must_use]
    pub const fn evidence(&self) -> DeploymentFinalizationCommitEvidence {
        self.evidence
    }

    /// Borrow the exact committed candidate for anchor completion and recovery.
    #[must_use]
    pub const fn candidate(&self) -> &PreparedDeploymentFinalizationAtom {
        &self.candidate
    }

    /// Begin the two exact candidate-derived external-anchor transitions.
    ///
    /// Database replay or ambiguous exact read-back observes each anchor before
    /// attempting another comparison. A failed leg retains completed progress
    /// in [`DeploymentFinalizationAnchorFailure::into_pending`].
    pub fn anchors_pending(self) -> DeploymentFinalizationAnchorsPending {
        DeploymentFinalizationAnchorsPending::new(self)
    }
}

/// Private coordinate projection retained only inside the anchored capability.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DeploymentFinalizationDependency {
    trust_store: AuthorityStoreCoordinate,
    principal_id: ResidentPrincipalIdV1,
    binding: ControlRecordCoordinate,
    checkpoint_signing_authority: ControlRecordCoordinate,
    runtime_authority: ControlRecordCoordinate,
    trust_checkpoint: PrincipalTrustCheckpointHead,
    administration_receipt: ControlRecordCoordinate,
}

impl DeploymentFinalizationDependency {
    /// Borrow the physical trust-store incarnation that owns finalization.
    #[must_use]
    const fn trust_store(&self) -> &AuthorityStoreCoordinate {
        &self.trust_store
    }

    /// Borrow the principal owning both transactions.
    #[must_use]
    const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }

    /// Borrow the finalized deployment binding coordinate.
    #[must_use]
    const fn binding(&self) -> &ControlRecordCoordinate {
        &self.binding
    }

    /// Borrow the finalized checkpoint-signer delegation coordinate.
    #[must_use]
    const fn checkpoint_signing_authority(&self) -> &ControlRecordCoordinate {
        &self.checkpoint_signing_authority
    }

    /// Borrow the finalized runtime-authority delegation coordinate.
    #[must_use]
    const fn runtime_authority(&self) -> &ControlRecordCoordinate {
        &self.runtime_authority
    }

    /// Borrow the successor principal trust checkpoint committed by transaction one.
    #[must_use]
    const fn trust_checkpoint(&self) -> &PrincipalTrustCheckpointHead {
        &self.trust_checkpoint
    }

    /// Borrow the administration receipt closing transaction one.
    #[must_use]
    const fn administration_receipt(&self) -> &ControlRecordCoordinate {
        &self.administration_receipt
    }
}

/// Which transaction-one anchor has not yet been established exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentFinalizationAnchorOwner {
    /// Principal-wide minimum accepted trust checkpoint.
    PrincipalTrustMinimum,
    /// Principal trust-store control-domain checkpoint.
    PrincipalControlDomain,
}

/// Evidence retained for one exact transaction-one anchor successor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentFinalizationAnchorEvidence {
    /// The external anchor advanced during this call.
    Advanced,
    /// Observation or compare-and-advance found the exact successor already retained.
    ExactReplay,
    /// An exact observation found the successor after an ambiguous advance.
    RecoveredExactAfterAmbiguousAdvance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeploymentFinalizationAnchorObservationReason {
    DatabaseReplay,
    AdvanceUncertain,
    Refused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeploymentFinalizationAnchorProgress {
    Ready,
    ObserveFirst(DeploymentFinalizationAnchorObservationReason),
    Established(DeploymentFinalizationAnchorEvidence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeploymentFinalizationAnchorLeg {
    transition: PreparedAntiRollbackAnchorTransition,
    progress: DeploymentFinalizationAnchorProgress,
}

impl DeploymentFinalizationAnchorLeg {
    const fn new(
        transition: PreparedAntiRollbackAnchorTransition,
        progress: DeploymentFinalizationAnchorProgress,
    ) -> Self {
        Self {
            transition,
            progress,
        }
    }

    fn complete(
        &mut self,
        port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<DeploymentFinalizationAnchorEvidence, DeploymentFinalizationAnchorFailureReason>
    {
        match self.progress {
            DeploymentFinalizationAnchorProgress::Established(evidence) => return Ok(evidence),
            DeploymentFinalizationAnchorProgress::Ready => {}
            DeploymentFinalizationAnchorProgress::ObserveFirst(reason) => {
                let observed = port
                    .observe(&self.transition)
                    .map_err(DeploymentFinalizationAnchorFailureReason::Observation)?;
                match observed {
                    AntiRollbackAnchorObservation::ExactSuccessor => {
                        let evidence = match reason {
                            DeploymentFinalizationAnchorObservationReason::AdvanceUncertain => {
                                DeploymentFinalizationAnchorEvidence::RecoveredExactAfterAmbiguousAdvance
                            }
                            DeploymentFinalizationAnchorObservationReason::DatabaseReplay
                            | DeploymentFinalizationAnchorObservationReason::Refused => {
                                DeploymentFinalizationAnchorEvidence::ExactReplay
                            }
                        };
                        self.progress = DeploymentFinalizationAnchorProgress::Established(evidence);
                        return Ok(evidence);
                    }
                    AntiRollbackAnchorObservation::ExactPredecessor => {}
                    observation => {
                        return Err(DeploymentFinalizationAnchorFailureReason::NotAdvanced(
                            Box::new(observation),
                        ));
                    }
                }
            }
        }

        match advance_finalization_anchor(port, &self.transition) {
            Ok(evidence) => {
                self.progress = DeploymentFinalizationAnchorProgress::Established(evidence);
                Ok(evidence)
            }
            Err(reason) => {
                self.progress = DeploymentFinalizationAnchorProgress::ObserveFirst(match &reason {
                    DeploymentFinalizationAnchorFailureReason::AmbiguousNotExact { .. }
                    | DeploymentFinalizationAnchorFailureReason::AmbiguousObservationFailed {
                        ..
                    } => DeploymentFinalizationAnchorObservationReason::AdvanceUncertain,
                    DeploymentFinalizationAnchorFailureReason::Observation(_)
                    | DeploymentFinalizationAnchorFailureReason::Port(_)
                    | DeploymentFinalizationAnchorFailureReason::NotAdvanced(_) => {
                        DeploymentFinalizationAnchorObservationReason::Refused
                    }
                });
                Err(reason)
            }
        }
    }
}

/// Database-committed transaction one with recoverable per-anchor progress.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "both finalization anchors remain required before genesis"]
pub struct DeploymentFinalizationAnchorsPending {
    committed: DatabaseCommittedDeploymentFinalization,
    trust_anchor: DeploymentFinalizationAnchorLeg,
    control_anchor: DeploymentFinalizationAnchorLeg,
}

impl DeploymentFinalizationAnchorsPending {
    fn new(committed: DatabaseCommittedDeploymentFinalization) -> Self {
        let progress = match committed.evidence {
            DeploymentFinalizationCommitEvidence::New => {
                DeploymentFinalizationAnchorProgress::Ready
            }
            DeploymentFinalizationCommitEvidence::ExactReplay
            | DeploymentFinalizationCommitEvidence::RecoveredExactAfterAmbiguousCommit => {
                DeploymentFinalizationAnchorProgress::ObserveFirst(
                    DeploymentFinalizationAnchorObservationReason::DatabaseReplay,
                )
            }
        };
        let trust_anchor = DeploymentFinalizationAnchorLeg::new(
            committed.candidate.trust_anchor.clone(),
            progress,
        );
        let control_anchor = DeploymentFinalizationAnchorLeg::new(
            committed.candidate.control_anchor.clone(),
            progress,
        );
        Self {
            committed,
            trust_anchor,
            control_anchor,
        }
    }

    /// Establish or observe both exact candidate-derived anchors in order.
    ///
    /// # Errors
    /// Returns the complete resumable state when either port operation cannot
    /// establish the exact successor. A capability is never returned early.
    pub fn complete(
        mut self,
        port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<AnchoredDeploymentFinalizationCapability, DeploymentFinalizationAnchorFailure> {
        let trust_anchor_evidence = match self.trust_anchor.complete(port) {
            Ok(evidence) => evidence,
            Err(reason) => {
                return Err(DeploymentFinalizationAnchorFailure {
                    pending: Box::new(self),
                    owner: DeploymentFinalizationAnchorOwner::PrincipalTrustMinimum,
                    reason,
                });
            }
        };
        let control_anchor_evidence = match self.control_anchor.complete(port) {
            Ok(evidence) => evidence,
            Err(reason) => {
                return Err(DeploymentFinalizationAnchorFailure {
                    pending: Box::new(self),
                    owner: DeploymentFinalizationAnchorOwner::PrincipalControlDomain,
                    reason,
                });
            }
        };
        let dependency = self.committed.candidate.dependency();
        Ok(AnchoredDeploymentFinalizationCapability {
            dependency,
            database_commit_evidence: self.committed.evidence,
            trust_anchor_evidence,
            control_anchor_evidence,
        })
    }
}

/// Opaque proof that exact transaction-one database custody and both of its
/// candidate-derived external anchors are established.
///
/// No public constructor exists. The only minting path is
/// [`DeploymentFinalizationAnchorsPending::complete`].
#[derive(Debug, PartialEq, Eq)]
#[must_use = "anchored finalization is required by deployment genesis"]
pub struct AnchoredDeploymentFinalizationCapability {
    dependency: DeploymentFinalizationDependency,
    database_commit_evidence: DeploymentFinalizationCommitEvidence,
    trust_anchor_evidence: DeploymentFinalizationAnchorEvidence,
    control_anchor_evidence: DeploymentFinalizationAnchorEvidence,
}

impl AnchoredDeploymentFinalizationCapability {
    /// Return how exact transaction-one database custody was established.
    #[must_use]
    pub const fn database_commit_evidence(&self) -> DeploymentFinalizationCommitEvidence {
        self.database_commit_evidence
    }

    /// Return the exact principal-trust anchor evidence.
    #[must_use]
    pub const fn trust_anchor_evidence(&self) -> DeploymentFinalizationAnchorEvidence {
        self.trust_anchor_evidence
    }

    /// Return the exact principal control-domain anchor evidence.
    #[must_use]
    pub const fn control_anchor_evidence(&self) -> DeploymentFinalizationAnchorEvidence {
        self.control_anchor_evidence
    }

    /// Borrow the physical trust-store incarnation that owns finalization.
    #[must_use]
    pub const fn trust_store(&self) -> &AuthorityStoreCoordinate {
        self.dependency.trust_store()
    }

    /// Borrow the principal owning both transactions.
    #[must_use]
    pub const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        self.dependency.principal_id()
    }

    /// Borrow the finalized deployment binding coordinate.
    #[must_use]
    pub const fn binding(&self) -> &ControlRecordCoordinate {
        self.dependency.binding()
    }

    /// Borrow the finalized checkpoint-signer delegation coordinate.
    #[must_use]
    pub const fn checkpoint_signing_authority(&self) -> &ControlRecordCoordinate {
        self.dependency.checkpoint_signing_authority()
    }

    /// Borrow the finalized runtime-authority delegation coordinate.
    #[must_use]
    pub const fn runtime_authority(&self) -> &ControlRecordCoordinate {
        self.dependency.runtime_authority()
    }

    /// Borrow the successor principal trust checkpoint committed and anchored by transaction one.
    #[must_use]
    pub const fn trust_checkpoint(&self) -> &PrincipalTrustCheckpointHead {
        self.dependency.trust_checkpoint()
    }

    /// Borrow the administration receipt closing transaction one.
    #[must_use]
    pub const fn administration_receipt(&self) -> &ControlRecordCoordinate {
        self.dependency.administration_receipt()
    }
}

/// Transaction-one anchor failure retaining exact resumable progress.
#[derive(Debug, thiserror::Error)]
#[error("deployment finalization anchor {owner:?} is not established: {reason}")]
pub struct DeploymentFinalizationAnchorFailure {
    pending: Box<DeploymentFinalizationAnchorsPending>,
    owner: DeploymentFinalizationAnchorOwner,
    reason: DeploymentFinalizationAnchorFailureReason,
}

impl DeploymentFinalizationAnchorFailure {
    /// Return which exact anchor leg failed.
    #[must_use]
    pub const fn owner(&self) -> DeploymentFinalizationAnchorOwner {
        self.owner
    }

    /// Borrow the failure classification.
    #[must_use]
    pub const fn reason(&self) -> &DeploymentFinalizationAnchorFailureReason {
        &self.reason
    }

    /// Recover all database custody and completed-anchor progress for resumption.
    pub fn into_pending(self) -> DeploymentFinalizationAnchorsPending {
        *self.pending
    }
}

/// Why an exact finalization anchor successor was not established.
#[derive(Debug, thiserror::Error)]
pub enum DeploymentFinalizationAnchorFailureReason {
    /// Observation required before a resumed comparison was unavailable.
    #[error("finalization anchor observation failed")]
    Observation(#[source] AntiRollbackAnchorError),
    /// The anchor failed before an ambiguous outcome.
    #[error("finalization anchor comparison failed")]
    Port(#[source] AntiRollbackAnchorError),
    /// The port reported an unrelated, behind, ahead, missing, or conflicting value.
    #[error("finalization anchor did not match the exact prepared transition")]
    NotAdvanced(Box<AntiRollbackAnchorObservation>),
    /// An ambiguous advance observed something other than the exact successor.
    #[error("ambiguous finalization anchor advance did not observe the exact successor")]
    AmbiguousNotExact {
        /// Original outcome-unknown error.
        uncertain: AntiRollbackAnchorError,
        /// Non-successor state found by the mandatory observation.
        observation: Box<AntiRollbackAnchorObservation>,
    },
    /// Observation failed after an ambiguous anchor advance.
    #[error("finalization anchor observation failed after ambiguous advance")]
    AmbiguousObservationFailed {
        /// Original outcome-unknown error.
        uncertain: AntiRollbackAnchorError,
        /// Error preventing exact observation.
        #[source]
        observe: AntiRollbackAnchorError,
    },
}

fn advance_finalization_anchor(
    port: &mut dyn AntiRollbackAnchorPort,
    transition: &PreparedAntiRollbackAnchorTransition,
) -> Result<DeploymentFinalizationAnchorEvidence, DeploymentFinalizationAnchorFailureReason> {
    match port.compare_and_advance(transition) {
        Ok(AntiRollbackAnchorAdvanceDisposition::Advanced) => {
            Ok(DeploymentFinalizationAnchorEvidence::Advanced)
        }
        Ok(AntiRollbackAnchorAdvanceDisposition::ExactReplay) => {
            Ok(DeploymentFinalizationAnchorEvidence::ExactReplay)
        }
        Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(observation)) => Err(
            DeploymentFinalizationAnchorFailureReason::NotAdvanced(Box::new(observation)),
        ),
        Err(error) if error.kind() == AntiRollbackAnchorErrorKind::OutcomeUnknown => {
            match port.observe(transition) {
                Ok(AntiRollbackAnchorObservation::ExactSuccessor) => {
                    Ok(DeploymentFinalizationAnchorEvidence::RecoveredExactAfterAmbiguousAdvance)
                }
                Ok(observation) => Err(
                    DeploymentFinalizationAnchorFailureReason::AmbiguousNotExact {
                        uncertain: error,
                        observation: Box::new(observation),
                    },
                ),
                Err(observe) => Err(
                    DeploymentFinalizationAnchorFailureReason::AmbiguousObservationFailed {
                        uncertain: error,
                        observe,
                    },
                ),
            }
        }
        Err(error) => Err(DeploymentFinalizationAnchorFailureReason::Port(error)),
    }
}

/// Exact physical binding of the separate deployment control store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentControlStoreBinding {
    store: AuthorityStoreCoordinate,
    principal_id: ResidentPrincipalIdV1,
    workspace_id: WorkspaceId,
    deployment_binding: ControlRecordCoordinate,
    checkpoint_signing_authority: ControlRecordCoordinate,
    runtime_authority: ControlRecordCoordinate,
}

impl DeploymentControlStoreBinding {
    /// Bind one control-store incarnation to its already-finalized authority coordinates.
    #[must_use]
    pub fn new(
        control_store_id: ControlStoreIdV1,
        control_incarnation_id: ControlIncarnationIdV1,
        principal_id: ResidentPrincipalIdV1,
        workspace_id: WorkspaceId,
        deployment_binding: ControlRecordCoordinate,
        checkpoint_signing_authority: ControlRecordCoordinate,
        runtime_authority: ControlRecordCoordinate,
    ) -> Self {
        Self {
            store: AuthorityStoreCoordinate::new(control_store_id, control_incarnation_id),
            principal_id,
            workspace_id,
            deployment_binding,
            checkpoint_signing_authority,
            runtime_authority,
        }
    }

    /// Borrow the deployment control-store coordinate.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the owning principal identity.
    #[must_use]
    pub const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }

    /// Borrow the exact workspace identity from the deployment binding.
    #[must_use]
    pub const fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// Borrow the finalized deployment binding coordinate.
    #[must_use]
    pub const fn deployment_binding(&self) -> &ControlRecordCoordinate {
        &self.deployment_binding
    }

    /// Borrow the finalized checkpoint-signer delegation coordinate.
    #[must_use]
    pub const fn checkpoint_signing_authority(&self) -> &ControlRecordCoordinate {
        &self.checkpoint_signing_authority
    }

    /// Borrow the finalized runtime-authority delegation coordinate.
    #[must_use]
    pub const fn runtime_authority(&self) -> &ControlRecordCoordinate {
        &self.runtime_authority
    }
}

/// Initial control checkpoint rows and the genesis receipt record.
///
/// This container validates record families and reference closure. It does not
/// authenticate any record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentControlStoreGenesisRecords {
    checkpoints: Vec<ControlRecord>,
    receipt: ControlRecord,
}

impl DeploymentControlStoreGenesisRecords {
    /// Retain only initial control checkpoints and the genesis receipt family.
    ///
    /// # Errors
    /// Rejects an empty or oversized set, a wrong family, or a reused immutable
    /// record reference.
    pub fn new(
        checkpoints: Vec<ControlRecord>,
        receipt: ControlRecord,
    ) -> Result<Self, DeploymentControlStoreGenesisCandidateError> {
        if checkpoints.is_empty() || checkpoints.len() > MAX_RESIDENT_SET_MEMBERS {
            return Err(DeploymentControlStoreGenesisCandidateError::CheckpointBound);
        }
        if checkpoints
            .iter()
            .any(|record| record.schema_id().as_str() != CONTROL_STATE_CHECKPOINT_V1_SCHEMA)
            || receipt.schema_id().as_str() != DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA
        {
            return Err(DeploymentControlStoreGenesisCandidateError::RecordFamily);
        }
        let references = checkpoints
            .iter()
            .chain(std::iter::once(&receipt))
            .map(|record| record.coordinate().record_ref())
            .collect::<BTreeSet<_>>();
        if references.len() != checkpoints.len() + 1 {
            return Err(DeploymentControlStoreGenesisCandidateError::ReferenceClosure);
        }
        Ok(Self {
            checkpoints,
            receipt,
        })
    }

    /// Borrow the domain-ordered initial control-checkpoint rows.
    #[must_use]
    pub fn checkpoints(&self) -> &[ControlRecord] {
        &self.checkpoints
    }

    /// Borrow the genesis receipt record.
    #[must_use]
    pub const fn receipt(&self) -> &ControlRecord {
        &self.receipt
    }
}

/// Closed physical candidate for transaction two in deployment initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedDeploymentControlStoreGenesisAtom {
    finalization: DeploymentFinalizationDependency,
    binding: DeploymentControlStoreBinding,
    genesis_id: DeploymentControlStoreGenesisIdV1,
    records: DeploymentControlStoreGenesisRecords,
    checkpoint_heads: Vec<ControlCheckpointHead>,
    accumulators: Vec<OperationAccumulatorBatch>,
}

impl PreparedDeploymentControlStoreGenesisAtom {
    /// Close one separate control-store genesis transaction over finalized authority.
    ///
    /// Database custody alone cannot satisfy this constructor; callers must
    /// first establish both exact transaction-one anchors and present the opaque
    /// [`AnchoredDeploymentFinalizationCapability`].
    ///
    /// ```compile_fail
    /// use pale_ports::{DatabaseCommittedDeploymentFinalization,
    ///     PreparedDeploymentControlStoreGenesisAtom};
    ///
    /// fn database_custody_is_not_genesis_authority(
    ///     committed: &DatabaseCommittedDeploymentFinalization,
    /// ) {
    ///     let _ = PreparedDeploymentControlStoreGenesisAtom::new(
    ///         committed,
    ///         todo!(),
    ///         todo!(),
    ///         todo!(),
    ///         todo!(),
    ///         todo!(),
    ///     );
    /// }
    /// ```
    ///
    /// # Errors
    /// Rejects a mismatched transaction-one dependency, trust-store reuse, an
    /// incomplete or unordered initial checkpoint set, record substitution, or
    /// accumulator batches outside one exact deployment-store transaction.
    pub fn new(
        finalization: &AnchoredDeploymentFinalizationCapability,
        binding: DeploymentControlStoreBinding,
        genesis_id: DeploymentControlStoreGenesisIdV1,
        records: DeploymentControlStoreGenesisRecords,
        checkpoint_heads: Vec<ControlCheckpointHead>,
        accumulators: Vec<OperationAccumulatorBatch>,
    ) -> Result<Self, DeploymentControlStoreGenesisCandidateError> {
        let finalization = finalization.dependency.clone();
        if finalization.principal_id() != binding.principal_id()
            || finalization.binding() != binding.deployment_binding()
            || finalization.checkpoint_signing_authority() != binding.checkpoint_signing_authority()
            || finalization.runtime_authority() != binding.runtime_authority()
        {
            return Err(DeploymentControlStoreGenesisCandidateError::FinalizationDependency);
        }
        if finalization.trust_store() == binding.store() {
            return Err(DeploymentControlStoreGenesisCandidateError::StoreSeparation);
        }
        if checkpoint_heads.is_empty()
            || checkpoint_heads.len() > MAX_RESIDENT_SET_MEMBERS
            || accumulators.len() > MAX_RESIDENT_SET_MEMBERS
        {
            return Err(DeploymentControlStoreGenesisCandidateError::CheckpointBound);
        }
        if checkpoint_heads.len() != records.checkpoints().len()
            || checkpoint_heads.len() != accumulators.len()
            || checkpoint_heads
                .windows(2)
                .any(|pair| pair[0].checkpoint_domain_id() >= pair[1].checkpoint_domain_id())
        {
            return Err(DeploymentControlStoreGenesisCandidateError::CheckpointSet);
        }
        let transaction = checkpoint_heads[0].committed_transaction_sequence();
        for ((record, head), accumulator) in records
            .checkpoints()
            .iter()
            .zip(&checkpoint_heads)
            .zip(&accumulators)
        {
            if record.coordinate() != head.coordinate()
                || head.domain_epoch().get() != 1
                || head.domain_sequence().get() != 1
                || head.committed_transaction_sequence() != transaction
                || accumulator.binding().store() != binding.store()
                || accumulator.binding().successor() != head
                || !matches!(
                    accumulator.binding().phase(),
                    AccumulatorCheckpointPhase::Initial
                )
            {
                return Err(DeploymentControlStoreGenesisCandidateError::AccumulatorClosure);
            }
        }
        Ok(Self {
            finalization,
            binding,
            genesis_id,
            records,
            checkpoint_heads,
            accumulators,
        })
    }

    /// Borrow the exact anchored finalization trust-store coordinate.
    #[must_use]
    pub const fn finalization_trust_store(&self) -> &AuthorityStoreCoordinate {
        self.finalization.trust_store()
    }

    /// Borrow the distinct deployment control-store binding.
    #[must_use]
    pub const fn binding(&self) -> &DeploymentControlStoreBinding {
        &self.binding
    }

    /// Borrow the stable genesis replay identity.
    #[must_use]
    pub const fn genesis_id(&self) -> &DeploymentControlStoreGenesisIdV1 {
        &self.genesis_id
    }

    /// Borrow the exact initial checkpoint and receipt rows.
    #[must_use]
    pub const fn records(&self) -> &DeploymentControlStoreGenesisRecords {
        &self.records
    }

    /// Borrow the complete domain-ordered initial checkpoint heads.
    #[must_use]
    pub fn checkpoint_heads(&self) -> &[ControlCheckpointHead] {
        &self.checkpoint_heads
    }

    /// Borrow the one-for-one initial accumulator batches.
    #[must_use]
    pub fn accumulators(&self) -> &[OperationAccumulatorBatch] {
        &self.accumulators
    }

    /// Attempt transaction two in the distinct deployment control store.
    ///
    /// An ambiguous mutation is never retried; exact read-back is mandatory.
    /// Independent genesis anchors remain pending after database commitment.
    ///
    /// # Errors
    /// Returns the complete candidate beside the physical failure or read-back
    /// classification whenever exact database commitment is not established.
    pub fn commit(
        self,
        store: &mut dyn DeploymentControlStoreGenesisCustodyPort,
    ) -> Result<
        DatabaseCommittedDeploymentControlStoreGenesis,
        DeploymentControlStoreGenesisCommitFailure,
    > {
        match store.compare_and_commit(&self) {
            Ok(DeploymentControlStoreGenesisCommitDisposition::New) => {
                Ok(self.database_committed(DeploymentControlStoreGenesisCommitEvidence::New))
            }
            Ok(DeploymentControlStoreGenesisCommitDisposition::ExactReplay) => {
                Ok(self
                    .database_committed(DeploymentControlStoreGenesisCommitEvidence::ExactReplay))
            }
            Err(error) if error.kind() == ControlStoreErrorKind::OutcomeUnknown => {
                match store.observe_exact(&self) {
                    Ok(DeploymentControlStoreGenesisDatabaseObservation::Exact) => Ok(self
                        .database_committed(
                            DeploymentControlStoreGenesisCommitEvidence::RecoveredExactAfterAmbiguousCommit,
                        )),
                    Ok(DeploymentControlStoreGenesisDatabaseObservation::Empty) => Err(self.failed(
                        DeploymentControlStoreGenesisCommitFailureReason::CommitExcludedByExactReadBack {
                            uncertain: error,
                        },
                    )),
                    Ok(DeploymentControlStoreGenesisDatabaseObservation::ConflictingOrPartial) => {
                        Err(self.failed(
                            DeploymentControlStoreGenesisCommitFailureReason::RecoveryRequiredAfterExactReadBack {
                                uncertain: error,
                            },
                        ))
                    }
                    Err(read_back) => Err(self.failed(
                        DeploymentControlStoreGenesisCommitFailureReason::ExactReadBackFailed {
                            uncertain: error,
                            read_back,
                        },
                    )),
                }
            }
            Err(error) => Err(self.failed(
                DeploymentControlStoreGenesisCommitFailureReason::Store(error),
            )),
        }
    }

    fn database_committed(
        self,
        evidence: DeploymentControlStoreGenesisCommitEvidence,
    ) -> DatabaseCommittedDeploymentControlStoreGenesis {
        DatabaseCommittedDeploymentControlStoreGenesis {
            candidate: self,
            evidence,
        }
    }

    fn failed(
        self,
        reason: DeploymentControlStoreGenesisCommitFailureReason,
    ) -> DeploymentControlStoreGenesisCommitFailure {
        DeploymentControlStoreGenesisCommitFailure {
            candidate: Box::new(self),
            reason,
        }
    }
}

/// Exact transaction-two database custody awaiting genesis anchor completion.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "database-committed genesis still requires its external anchors"]
pub struct DatabaseCommittedDeploymentControlStoreGenesis {
    candidate: PreparedDeploymentControlStoreGenesisAtom,
    evidence: DeploymentControlStoreGenesisCommitEvidence,
}

impl DatabaseCommittedDeploymentControlStoreGenesis {
    /// Return how exact transaction-two database custody was established.
    #[must_use]
    pub const fn evidence(&self) -> DeploymentControlStoreGenesisCommitEvidence {
        self.evidence
    }

    /// Borrow the exact committed genesis for anchor completion and recovery.
    #[must_use]
    pub const fn candidate(&self) -> &PreparedDeploymentControlStoreGenesisAtom {
        &self.candidate
    }
}

fn principal_anchor(head: &PrincipalTrustCheckpointHead) -> PrincipalTrustMinimumAnchor {
    PrincipalTrustMinimumAnchor::new(
        head.principal_id().clone(),
        head.checkpoint_epoch(),
        head.coordinate().clone(),
    )
}

fn control_anchor(
    binding: &PrincipalTrustStoreBinding,
    head: &ControlCheckpointHead,
) -> ControlDomainAntiRollbackAnchor {
    ControlDomainAntiRollbackAnchor::new(
        binding.trust_store_id().clone(),
        binding.trust_store_incarnation_id().clone(),
        head.checkpoint_domain_id().clone(),
        head.domain_epoch(),
        head.domain_sequence(),
        head.coordinate().clone(),
    )
}

/// Invalid physical closure for deployment finalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeploymentFinalizationCandidateError {
    /// Exact candidate heads cannot form the two required anchor transitions.
    #[error("deployment finalization anchors do not close over its exact heads")]
    AnchorClosure,
    /// The physical accumulator does not close over the selected transaction and head.
    #[error("deployment finalization accumulator does not close over transaction one")]
    AccumulatorClosure,
    /// Principal trust or control heads do not form exact successors.
    #[error("deployment finalization heads are not exact successors")]
    Frontier,
    /// One field contains a record from another contract family.
    #[error("deployment finalization record family is not initial-v1 exact")]
    RecordFamily,
    /// Record references are reused or disagree with retained heads/replay identity.
    #[error("deployment finalization record references do not close")]
    ReferenceClosure,
}

/// Invalid physical closure for deployment control-store genesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeploymentControlStoreGenesisCandidateError {
    /// The checkpoint collection is empty or exceeds the admitted resident bound.
    #[error("deployment genesis checkpoint set is empty or exceeds its admitted bound")]
    CheckpointBound,
    /// Initial checkpoint heads are empty, incomplete, or not canonically ordered.
    #[error("deployment genesis checkpoint set is incomplete or unordered")]
    CheckpointSet,
    /// Initial accumulators do not close over one exact genesis transaction.
    #[error("deployment genesis accumulators do not close over transaction two")]
    AccumulatorClosure,
    /// Genesis does not reproduce the exact committed finalization authority coordinates.
    #[error("deployment genesis does not match its finalization dependency")]
    FinalizationDependency,
    /// One field contains a record from another contract family.
    #[error("deployment genesis record family is not initial-v1 exact")]
    RecordFamily,
    /// Record references are reused or disagree with their checkpoint heads.
    #[error("deployment genesis record references do not close")]
    ReferenceClosure,
    /// Genesis attempts to reuse the principal trust-store incarnation.
    #[error("deployment genesis must use a distinct control-store transaction")]
    StoreSeparation,
}

/// Successful finalization compare-and-commit disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentFinalizationCommitDisposition {
    /// Transaction one committed the candidate.
    New,
    /// The store already contained the exact candidate and original receipt.
    ExactReplay,
}

/// Evidence retained after exact transaction-one database custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentFinalizationCommitEvidence {
    /// The adapter reported a new atomic commit.
    New,
    /// The adapter proved byte-exact prior custody directly.
    ExactReplay,
    /// A commit was ambiguous and mandatory read-back proved the exact atom.
    RecoveredExactAfterAmbiguousCommit,
}

/// Exact finalization read-back after restart or an ambiguous commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentFinalizationDatabaseObservation {
    /// The expected predecessor still owns every selected head and no replay row exists.
    Predecessor,
    /// Every record, head, tree mutation, replay key, and receipt matches exactly.
    Exact,
    /// A partial atom, changed replay, or another successor requires recovery.
    ConflictingOrPartial,
}

/// Failed transaction-one transition retaining the complete candidate.
#[derive(Debug, thiserror::Error)]
#[error("deployment finalization did not commit exactly: {reason}")]
pub struct DeploymentFinalizationCommitFailure {
    candidate: Box<PreparedDeploymentFinalizationAtom>,
    reason: DeploymentFinalizationCommitFailureReason,
}

impl DeploymentFinalizationCommitFailure {
    /// Borrow the exact failure classification.
    #[must_use]
    pub const fn reason(&self) -> &DeploymentFinalizationCommitFailureReason {
        &self.reason
    }

    /// Recover the complete candidate for explicit retry or recovery.
    pub fn into_candidate(self) -> PreparedDeploymentFinalizationAtom {
        *self.candidate
    }
}

/// Reason transaction-one commitment remains unproven.
#[derive(Debug, thiserror::Error)]
pub enum DeploymentFinalizationCommitFailureReason {
    /// The store rejected the candidate outside commit ambiguity.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// Exact read-back proved the predecessor remained current.
    #[error("exact finalization read-back excluded commitment")]
    CommitExcludedByExactReadBack {
        /// Native uncertainty from the attempted commit.
        #[source]
        uncertain: ControlStoreError,
    },
    /// Exact read-back found partial or contradictory custody.
    #[error("exact finalization read-back requires explicit recovery")]
    RecoveryRequiredAfterExactReadBack {
        /// Native uncertainty from the attempted commit.
        #[source]
        uncertain: ControlStoreError,
    },
    /// Mandatory exact read-back itself could not complete.
    #[error("exact finalization read-back failed")]
    ExactReadBackFailed {
        /// Native uncertainty from the attempted commit.
        uncertain: ControlStoreError,
        /// Native read-back failure.
        #[source]
        read_back: ControlStoreError,
    },
}

/// Physical custody for transaction one only: principal-store finalization.
/// The sole fenced resident writer is the only permitted caller; implementing
/// this port does not create another administration or database writer.
pub trait DeploymentFinalizationCustodyPort {
    /// Compare replay identity and every predecessor inside one database transaction,
    /// then commit all finalization records, heads, and accumulator positions atomically.
    ///
    /// An exact replay returns the original receipt disposition. Reuse of the
    /// authority-scoped idempotency key with changed bytes is a candidate conflict.
    ///
    /// # Errors
    /// Returns candidate conflict for changed replay or stale predecessors,
    /// recovery required for partial custody, outcome unknown for an ambiguous
    /// commit, and storage failures only when commitment was excluded.
    fn compare_and_commit(
        &mut self,
        candidate: &PreparedDeploymentFinalizationAtom,
    ) -> Result<DeploymentFinalizationCommitDisposition, ControlStoreError>;

    /// Classify exact transaction-one custody without mutation after uncertainty.
    ///
    /// # Errors
    /// Returns a control-store error when one stable physical observation cannot
    /// be completed; it never repairs or retries the candidate.
    fn observe_exact(
        &mut self,
        candidate: &PreparedDeploymentFinalizationAtom,
    ) -> Result<DeploymentFinalizationDatabaseObservation, ControlStoreError>;
}

/// Successful genesis compare-and-commit disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentControlStoreGenesisCommitDisposition {
    /// Transaction two initialized the empty deployment control store.
    New,
    /// The separate store already contained the exact genesis and original receipt.
    ExactReplay,
}

/// Evidence retained after exact transaction-two database custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentControlStoreGenesisCommitEvidence {
    /// The adapter reported a new atomic genesis commit.
    New,
    /// The adapter proved byte-exact prior genesis directly.
    ExactReplay,
    /// A commit was ambiguous and mandatory read-back proved the exact genesis.
    RecoveredExactAfterAmbiguousCommit,
}

/// Exact genesis read-back after restart or an ambiguous commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentControlStoreGenesisDatabaseObservation {
    /// No genesis rows or hard-domain heads exist in the bound deployment store.
    Empty,
    /// Every initial checkpoint, tree, head, and genesis receipt matches exactly.
    Exact,
    /// A partial genesis, changed replay, or another candidate requires recovery.
    ConflictingOrPartial,
}

/// Failed transaction-two transition retaining the complete candidate.
#[derive(Debug, thiserror::Error)]
#[error("deployment control-store genesis did not commit exactly: {reason}")]
pub struct DeploymentControlStoreGenesisCommitFailure {
    candidate: Box<PreparedDeploymentControlStoreGenesisAtom>,
    reason: DeploymentControlStoreGenesisCommitFailureReason,
}

impl DeploymentControlStoreGenesisCommitFailure {
    /// Borrow the exact failure classification.
    #[must_use]
    pub const fn reason(&self) -> &DeploymentControlStoreGenesisCommitFailureReason {
        &self.reason
    }

    /// Recover the complete candidate for explicit retry or recovery.
    pub fn into_candidate(self) -> PreparedDeploymentControlStoreGenesisAtom {
        *self.candidate
    }
}

/// Reason transaction-two commitment remains unproven.
#[derive(Debug, thiserror::Error)]
pub enum DeploymentControlStoreGenesisCommitFailureReason {
    /// The store rejected the candidate outside commit ambiguity.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// Exact read-back proved the deployment store remained empty.
    #[error("exact genesis read-back excluded commitment")]
    CommitExcludedByExactReadBack {
        /// Native uncertainty from the attempted commit.
        #[source]
        uncertain: ControlStoreError,
    },
    /// Exact read-back found partial or contradictory custody.
    #[error("exact genesis read-back requires explicit recovery")]
    RecoveryRequiredAfterExactReadBack {
        /// Native uncertainty from the attempted commit.
        #[source]
        uncertain: ControlStoreError,
    },
    /// Mandatory exact read-back itself could not complete.
    #[error("exact genesis read-back failed")]
    ExactReadBackFailed {
        /// Native uncertainty from the attempted commit.
        uncertain: ControlStoreError,
        /// Native read-back failure.
        #[source]
        read_back: ControlStoreError,
    },
}

/// Physical custody for transaction two only: deployment control-store genesis.
/// The same sole fenced resident writer calls this port only after transaction
/// one and its required independent frontier custody have been established.
pub trait DeploymentControlStoreGenesisCustodyPort {
    /// Initialize one separately bound deployment store or prove exact replay.
    ///
    /// This port never mutates the principal trust store. The application must
    /// prove transaction-one custody before submitting this candidate and must
    /// advance independent genesis anchors only after durable database commit.
    ///
    /// # Errors
    /// Returns candidate conflict for a changed genesis identity or occupied
    /// store, recovery required for partial custody, outcome unknown for an
    /// ambiguous commit, and storage failures only when commitment was excluded.
    fn compare_and_commit(
        &mut self,
        candidate: &PreparedDeploymentControlStoreGenesisAtom,
    ) -> Result<DeploymentControlStoreGenesisCommitDisposition, ControlStoreError>;

    /// Classify exact transaction-two custody without mutation after uncertainty.
    ///
    /// # Errors
    /// Returns a control-store error when the separate deployment store cannot
    /// provide one stable exact observation; it never repairs or initializes it.
    fn observe_exact(
        &mut self,
        candidate: &PreparedDeploymentControlStoreGenesisAtom,
    ) -> Result<DeploymentControlStoreGenesisDatabaseObservation, ControlStoreError>;
}

#[cfg(test)]
#[path = "../tests/internal/deployment_initialization.rs"]
mod tests;
