//! Append owner-admitted conclusive evidence to an authenticated operation head.
//!
//! This module joins existing observation and checkpoint custody. It does not
//! interpret provider state, remote task state, cancellation acknowledgements,
//! or expiry. The caller must first admit the exact conclusive owner evidence.

use super::*;
use crate::resident::operation_dispatch::{
    commit_prepared_checkpoint, observation_checkpoint_preimage, verify_operation_head_custody,
    OperationAppendFailure, OperationAttemptError, OperationCheckpointInput,
};
use crate::resident::{
    CompletedOperationObservationCustody, DatabaseCommittedOperationObservationAnchorPending,
    OperationCheckpointSelection, OperationObservationAuthorityStoreResolver,
    OperationObservationCustodyAnchorFailure, OperationObservationCustodyCommitFailure,
    PreparedOperationObservationCustody,
};
use pale_core::records::{
    ArtifactDigest, CanonicalSignatureV1, CheckpointProfileDigest,
    CheckpointSigningAuthorityDigest, ControlCheckpointDomainIdV1,
    ControlCheckpointSigningAuthorityAdmissionContextV1, ControlIncarnationIdV1,
    ControlStateCheckpointDigest, ControlStateCheckpointPreimageV1, ControlStateCheckpointV1,
    ControlStoreIdV1, InvalidResidentRecordV1, OperationObservationAdmissionContextV1,
    OperationObservationDigest, OperationObservationPreimageV1, OperationObservationStateV1,
    ResidentKeyIdV1, ResidentPrincipalIdV1, SchemaDigest, SchemaId,
    SignedResidentAdmissionContextV1, SigningKeyEpochV1, UnixTimeV1,
    OPERATION_OBSERVATION_V1_SCHEMA,
};
use pale_ports::{AccumulatorPreparationLimits, AntiRollbackAnchorPort, OperationCustodyPort};

/// Exact inputs for one proof-bearing reconciliation append.
pub(in crate::resident) struct CurrentHeadReconciliationAppendInput<'a> {
    /// Authenticated current operation history and checkpoint, consumed once.
    pub recovered: RecoveredOperationObservation,
    /// Exact admitted handle named by `recovered`.
    pub handle_ref: &'a ResidentRecordRefV1,
    /// Exact admitted handle named by `recovered`.
    pub handle: &'a OperationHandleV1,
    /// Fresh immutable reference for the successor observation.
    pub successor_ref: ResidentRecordRefV1,
    /// Already selected conclusive owner state. This module does not derive it.
    pub state: OperationObservationStateV1,
    /// Time at which the admitted conclusive evidence was observed.
    pub observed_at: UnixTimeV1,
    /// Exact observation schema and signer authority.
    pub signed: SignedResidentAdmissionContextV1<'a>,
    /// Retained checkpoint profile and original-request binding.
    pub selection: OperationCheckpointSelection,
    /// Fresh checkpoint identities for this append.
    pub checkpoint: OperationCheckpointInput,
    /// Exact checkpoint schema identity.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Admitted checkpoint signer authority.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Bounded accumulator preparation work.
    pub accumulator_limits: AccumulatorPreparationLimits,
}

/// Completed database and independent-anchor custody for the conclusive revision.
pub(in crate::resident) struct CompletedCurrentHeadReconciliation {
    successor: (ResidentRecordRefV1, OperationObservationV1),
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: CompletedOperationObservationCustody,
}

impl std::fmt::Debug for CompletedCurrentHeadReconciliation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CompletedCurrentHeadReconciliation")
            .finish_non_exhaustive()
    }
}

impl CompletedCurrentHeadReconciliation {
    /// Borrow the exact conclusive successor.
    pub(in crate::resident) const fn successor(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.successor.0, &self.successor.1)
    }

    /// Borrow the signed checkpoint that durably contains the successor.
    pub(in crate::resident) const fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    /// Consume the completion and return its existing custody proof.
    pub(in crate::resident) fn into_custody(self) -> CompletedOperationObservationCustody {
        self.custody
    }
}

#[derive(Clone)]
struct RetainedCheckpointAuthority {
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

impl RetainedCheckpointAuthority {
    fn new(authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>) -> Self {
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

    fn context(&self) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
        ControlCheckpointSigningAuthorityAdmissionContextV1 {
            authority_ref: &self.authority_ref,
            authority_sha256: &self.authority_sha256,
            principal_id: &self.principal_id,
            control_store_id: &self.control_store_id,
            control_incarnation_id: &self.control_incarnation_id,
            checkpoint_domain_id: &self.checkpoint_domain_id,
            checkpoint_profile_ref: &self.checkpoint_profile_ref,
            checkpoint_profile_sha256: &self.checkpoint_profile_sha256,
            subject_key_id: &self.subject_key_id,
            subject_key_epoch: self.subject_key_epoch,
        }
    }
}

struct CurrentHeadReconciliationCheckpointPlan {
    predecessor: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    checkpoint: OperationCheckpointInput,
    issued_at: UnixTimeV1,
    schema: SchemaDigest,
    authority: RetainedCheckpointAuthority,
    limits: AccumulatorPreparationLimits,
}

struct CurrentHeadReconciliationBasis {
    store: pale_ports::AuthorityStoreCoordinate,
    handle_ref: ResidentRecordRefV1,
    handle: OperationHandleV1,
    predecessor: Box<(ResidentRecordRefV1, OperationObservationV1)>,
    predecessor_checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    successor: Box<(ResidentRecordRefV1, OperationObservationV1)>,
    selection: OperationCheckpointSelection,
    checkpoint: CurrentHeadReconciliationCheckpointPlan,
}

impl CurrentHeadReconciliationBasis {
    const fn successor(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.successor.0, &self.successor.1)
    }

    fn checkpoint_authority(&self) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
        self.checkpoint.authority.context()
    }

    fn prepare_checkpoint(
        &self,
        store_resolver: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<crate::resident::PreparedOperationObservationCheckpoint<'_>, OperationAppendFailure>
    {
        let append = prepare_operation_observation_append(
            OperationObservationTransition::new(
                &self.handle_ref,
                &self.handle,
                &self.predecessor.0,
                &self.predecessor.1,
                &self.successor.0,
                &self.successor.1,
            ),
            store_resolver,
        )?;
        if append.store() != &self.store {
            return Err(OperationAttemptError::Association.into());
        }
        append
            .prepare_checkpoint(
                self.selection.clone(),
                &self.checkpoint.predecessor.0,
                &self.checkpoint.predecessor.1,
                self.checkpoint.limits,
                custody,
            )
            .map_err(Into::into)
    }
}

/// Signed terminal successor and fixed checkpoint plan awaiting physical custody.
#[must_use = "continue custody for this exact reconciliation successor"]
pub(in crate::resident) struct PreparedCurrentHeadReconciliation {
    basis: CurrentHeadReconciliationBasis,
}

impl std::fmt::Debug for PreparedCurrentHeadReconciliation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedCurrentHeadReconciliation")
            .finish_non_exhaustive()
    }
}

/// Exact signed checkpoint awaiting attachment to the same terminal successor.
#[must_use = "attach this exact checkpoint without signing another value"]
pub(in crate::resident) struct IssuedCurrentHeadReconciliationCheckpoint {
    basis: CurrentHeadReconciliationBasis,
    checkpoint: ControlStateCheckpointV1,
}

impl std::fmt::Debug for IssuedCurrentHeadReconciliationCheckpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IssuedCurrentHeadReconciliationCheckpoint")
            .finish_non_exhaustive()
    }
}

/// Exact database candidate retained after a conclusive append could not commit.
#[must_use = "retry or read back only this retained reconciliation candidate"]
pub(in crate::resident) struct CurrentHeadReconciliationCommitFailure {
    basis: CurrentHeadReconciliationBasis,
    checkpoint: ControlStateCheckpointV1,
    failure: OperationObservationCustodyCommitFailure,
}

impl std::fmt::Debug for CurrentHeadReconciliationCommitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CurrentHeadReconciliationCommitFailure")
            .field("reason", self.failure.reason())
            .finish_non_exhaustive()
    }
}

/// Definitively uncommitted candidate that may perform one exact commit attempt.
#[must_use = "commit only this retained reconciliation candidate"]
pub(in crate::resident) struct RetryableCurrentHeadReconciliationCommit {
    basis: CurrentHeadReconciliationBasis,
    checkpoint: ControlStateCheckpointV1,
    physical: PreparedOperationObservationCustody,
}

enum CurrentHeadReconciliationAnchorCustody {
    Pending(Box<DatabaseCommittedOperationObservationAnchorPending>),
    Failed(OperationObservationCustodyAnchorFailure),
}

/// Database-proven reconciliation append awaiting its independent anchor.
#[must_use = "continue only this retained reconciliation anchor transition"]
pub(in crate::resident) struct CurrentHeadReconciliationAnchorPending {
    basis: CurrentHeadReconciliationBasis,
    checkpoint: ControlStateCheckpointV1,
    custody: CurrentHeadReconciliationAnchorCustody,
}

impl std::fmt::Debug for CurrentHeadReconciliationAnchorPending {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CurrentHeadReconciliationAnchorPending")
            .finish_non_exhaustive()
    }
}

/// Exact phase retained when a current-head reconciliation append stops.
#[derive(Debug)]
pub(in crate::resident) enum CurrentHeadReconciliationAppendFailure {
    /// No successor was issued.
    BeforeSuccessor(OperationAppendFailure),
    /// The terminal successor was signed, but its checkpoint was not issued.
    Prepared {
        /// Exact signed successor and fixed checkpoint plan.
        continuation: Box<PreparedCurrentHeadReconciliation>,
        /// Root preparation, checkpoint construction, or checkpoint signing failure.
        failure: Box<OperationAppendFailure>,
    },
    /// The checkpoint was signed once, but physical attachment failed.
    Issued {
        /// Exact signed successor and checkpoint. This phase accepts no signer.
        continuation: Box<IssuedCurrentHeadReconciliationCheckpoint>,
        /// Existing attachment failure.
        failure: Box<OperationAppendFailure>,
    },
    /// Database custody remains unproved for one exact candidate.
    Commit(Box<CurrentHeadReconciliationCommitFailure>),
    /// Database custody is exact and only the independent anchor remains.
    Anchor(Box<CurrentHeadReconciliationAnchorPending>),
}

impl CurrentHeadReconciliationAppendFailure {
    /// Borrow the already-issued successor, when issuance completed.
    pub(in crate::resident) const fn successor(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &OperationObservationV1)> {
        match self {
            Self::BeforeSuccessor(_) => None,
            Self::Prepared { continuation, .. } => Some(continuation.basis.successor()),
            Self::Issued { continuation, .. } => Some(continuation.basis.successor()),
            Self::Commit(continuation) => Some(continuation.basis.successor()),
            Self::Anchor(continuation) => Some(continuation.basis.successor()),
        }
    }
}

impl PreparedCurrentHeadReconciliation {
    /// Borrow the terminal successor already signed by its semantic owner.
    pub(in crate::resident) fn successor(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.basis.successor()
    }

    /// Re-read the exact predecessor, checkpoints, and independent anchor.
    pub(in crate::resident) fn verify_predecessor_custody(
        &self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        maximum_record_bytes: usize,
    ) -> Result<(), OperationAttemptError> {
        verify_operation_head_custody(
            &self.basis.store,
            &self.basis.handle_ref,
            &self.basis.handle,
            (&self.basis.predecessor.0, &self.basis.predecessor.1),
            (
                &self.basis.predecessor_checkpoint.0,
                &self.basis.predecessor_checkpoint.1,
            ),
            (
                &self.basis.checkpoint.predecessor.0,
                &self.basis.checkpoint.predecessor.1,
            ),
            custody,
            anchor,
            maximum_record_bytes,
        )
    }

    /// Sign one fixed checkpoint, attach it, and enter physical custody.
    ///
    /// This phase accepts only the checkpoint signer. It cannot admit evidence,
    /// rotate the observation authority, or issue another terminal observation.
    pub(in crate::resident) fn continue_custody<Sign>(
        self,
        store_resolver: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        sign_checkpoint: Sign,
    ) -> Result<CompletedCurrentHeadReconciliation, CurrentHeadReconciliationAppendFailure>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.continue_custody_with_checkpoint_attachment(
            store_resolver,
            custody,
            anchor,
            sign_checkpoint,
            |prepared, reference, checkpoint| {
                prepared.attach_issued_checkpoint(reference, checkpoint)
            },
        )
    }

    /// Continue through one selected checkpoint-attachment implementation.
    ///
    /// The private seam proves that a failure after signing returns the exact
    /// issued checkpoint and cannot reach another signer.
    pub(in crate::resident) fn continue_custody_with_checkpoint_attachment<Sign, Attach>(
        self,
        store_resolver: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        sign_checkpoint: Sign,
        attach: Attach,
    ) -> Result<CompletedCurrentHeadReconciliation, CurrentHeadReconciliationAppendFailure>
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
            PreparedOperationObservationCustody,
            crate::resident::OperationObservationCustodyProjectionError,
        >,
    {
        let prepared = match self.basis.prepare_checkpoint(store_resolver, custody) {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Err(CurrentHeadReconciliationAppendFailure::Prepared {
                    continuation: Box::new(self),
                    failure: Box::new(failure),
                });
            }
        };
        let authority = self.basis.checkpoint_authority();
        let preimage = match observation_checkpoint_preimage(
            &prepared,
            (
                &self.basis.checkpoint.predecessor.0,
                &self.basis.checkpoint.predecessor.1,
            ),
            &self.basis.checkpoint.checkpoint,
            self.basis.checkpoint.issued_at,
            &self.basis.checkpoint.schema,
            &authority,
        ) {
            Ok(preimage) => preimage,
            Err(failure) => {
                return Err(CurrentHeadReconciliationAppendFailure::Prepared {
                    continuation: Box::new(self),
                    failure: Box::new(OperationAppendFailure::Operation(failure)),
                });
            }
        };
        let checkpoint = match prepared.issue_checkpoint_value(
            preimage,
            &self.basis.checkpoint.schema,
            &authority,
            sign_checkpoint,
        ) {
            Ok(checkpoint) => checkpoint,
            Err(failure) => {
                return Err(CurrentHeadReconciliationAppendFailure::Prepared {
                    continuation: Box::new(self),
                    failure: Box::new(OperationAppendFailure::Projection(failure)),
                });
            }
        };
        let physical = match attach(
            prepared,
            &self.basis.checkpoint.checkpoint.reference,
            &checkpoint,
        ) {
            Ok(physical) => physical,
            Err(failure) => {
                return Err(CurrentHeadReconciliationAppendFailure::Issued {
                    continuation: Box::new(IssuedCurrentHeadReconciliationCheckpoint {
                        basis: self.basis,
                        checkpoint,
                    }),
                    failure: Box::new(OperationAppendFailure::Projection(failure)),
                });
            }
        };
        complete_current_head_reconciliation(self.basis, checkpoint, physical, custody, anchor)
    }
}

impl IssuedCurrentHeadReconciliationCheckpoint {
    /// Borrow the exact signed terminal successor.
    pub(in crate::resident) fn successor(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.basis.successor()
    }

    /// Borrow the exact signed checkpoint retained after issuance.
    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (
            &self.basis.checkpoint.checkpoint.reference,
            &self.checkpoint,
        )
    }

    /// Rebuild roots and attach the already issued checkpoint.
    ///
    /// This continuation accepts no signer and cannot issue another checkpoint.
    pub(in crate::resident) fn continue_custody(
        self,
        store_resolver: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedCurrentHeadReconciliation, CurrentHeadReconciliationAppendFailure> {
        let prepared = match self.basis.prepare_checkpoint(store_resolver, custody) {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Err(CurrentHeadReconciliationAppendFailure::Issued {
                    continuation: Box::new(self),
                    failure: Box::new(failure),
                });
            }
        };
        let physical = match prepared.attach_issued_checkpoint(
            &self.basis.checkpoint.checkpoint.reference,
            &self.checkpoint,
        ) {
            Ok(physical) => physical,
            Err(failure) => {
                return Err(CurrentHeadReconciliationAppendFailure::Issued {
                    continuation: Box::new(self),
                    failure: Box::new(OperationAppendFailure::Projection(failure)),
                });
            }
        };
        complete_current_head_reconciliation(self.basis, self.checkpoint, physical, custody, anchor)
    }
}

impl CurrentHeadReconciliationCommitFailure {
    /// Borrow the terminal successor retained in the exact database candidate.
    pub(in crate::resident) fn successor(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.basis.successor()
    }

    /// Borrow the exact checkpoint retained in the candidate.
    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (
            &self.basis.checkpoint.checkpoint.reference,
            &self.checkpoint,
        )
    }

    /// Recover a mutation-capable token only after definitive non-commit evidence.
    pub(in crate::resident) fn try_into_retryable(
        self,
    ) -> Result<RetryableCurrentHeadReconciliationCommit, Box<Self>> {
        let Self {
            basis,
            checkpoint,
            failure,
        } = self;
        match failure.try_into_retryable() {
            Ok(physical) => Ok(RetryableCurrentHeadReconciliationCommit {
                basis,
                checkpoint,
                physical,
            }),
            Err(failure) => Err(Box::new(Self {
                basis,
                checkpoint,
                failure,
            })),
        }
    }

    /// Read back only the retained candidate after unresolved commit ambiguity.
    pub(in crate::resident) fn reconcile_exact(
        self,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<CurrentHeadReconciliationAnchorPending, Box<Self>> {
        let Self {
            basis,
            checkpoint,
            failure,
        } = self;
        match failure.reconcile_exact(custody) {
            Ok(pending) => Ok(CurrentHeadReconciliationAnchorPending {
                basis,
                checkpoint,
                custody: CurrentHeadReconciliationAnchorCustody::Pending(Box::new(pending)),
            }),
            Err(failure) => Err(Box::new(Self {
                basis,
                checkpoint,
                failure,
            })),
        }
    }
}

impl RetryableCurrentHeadReconciliationCommit {
    /// Attempt the one exact candidate whose prior non-commit was proved.
    pub(in crate::resident) fn commit(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedCurrentHeadReconciliation, CurrentHeadReconciliationAppendFailure> {
        complete_current_head_reconciliation(
            self.basis,
            self.checkpoint,
            self.physical,
            custody,
            anchor,
        )
    }
}

impl CurrentHeadReconciliationAnchorPending {
    /// Borrow the exact database-committed terminal successor.
    pub(in crate::resident) fn successor(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.basis.successor()
    }

    /// Borrow the exact database-committed checkpoint.
    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (
            &self.basis.checkpoint.checkpoint.reference,
            &self.checkpoint,
        )
    }

    /// Continue only the retained independent-anchor transition.
    pub(in crate::resident) fn continue_anchor(
        self,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedCurrentHeadReconciliation, Box<Self>> {
        let Self {
            basis,
            checkpoint,
            custody,
        } = self;
        let completed = match custody {
            CurrentHeadReconciliationAnchorCustody::Pending(pending) => {
                pending.advance_anchor(anchor)
            }
            CurrentHeadReconciliationAnchorCustody::Failed(failure) => {
                match failure.try_into_retryable() {
                    Ok(pending) => pending.advance_anchor(anchor),
                    Err(failure) => failure.continue_after_failed_observation(anchor),
                }
            }
        };
        match completed {
            Ok(custody) => Ok(complete_reconciliation(basis, checkpoint, custody)),
            Err(failure) => Err(Box::new(Self {
                basis,
                checkpoint,
                custody: CurrentHeadReconciliationAnchorCustody::Failed(failure),
            })),
        }
    }
}

/// Append one exact conclusive successor to an authenticated nonterminal head.
///
/// `admit_evidence` must prove the caller-selected result, diagnostic, receipt,
/// or reconciliation evidence. This function neither maps a carrier response nor
/// retries any effect. A commit or anchor ambiguity returns the same signed
/// successor and the existing custody continuation; callers must not reissue it.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn append_current_head_reconciliation<
    Evidence,
    Rotation,
    SignObservation,
    SignCheckpoint,
>(
    input: CurrentHeadReconciliationAppendInput<'_>,
    store_resolver: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admit_evidence: Evidence,
    admit_rotation: Rotation,
    sign_observation: SignObservation,
    sign_checkpoint: SignCheckpoint,
) -> Result<CompletedCurrentHeadReconciliation, CurrentHeadReconciliationAppendFailure>
where
    Evidence: FnOnce(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
    Rotation: FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    SignObservation: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    SignCheckpoint: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let handle_coordinate = record_projection::coordinate(
        input.handle_ref,
        input.handle.canonical_operation_handle_sha256().as_str(),
    )
    .map_err(|error| before(OperationAppendFailure::Projection(error.into())))?;
    if input.recovered.handle != handle_coordinate
        || !input.selection.matches_store(input.recovered.store())
        || !input
            .selection
            .matches_authority(input.checkpoint_authority)
        || input
            .selection
            .verify(
                input.recovered.store(),
                input.handle,
                input.recovered.current_checkpoint().1,
            )
            .is_err()
    {
        return Err(before(OperationAttemptError::Association.into()));
    }
    let predecessor = input.recovered.selected_head();
    let predecessor_checkpoint = input.recovered.selected_head_checkpoint_record();
    if predecessor.1.operation_handle_id() != input.handle.operation_handle_id()
        || predecessor.1.operation_handle_ref() != input.handle_ref
        || predecessor.1.operation_handle_sha256()
            != input.handle.canonical_operation_handle_sha256()
        || predecessor.1.state().is_terminal()
        || !matches!(
            predecessor.1.state(),
            OperationObservationStateV1::OutcomeUnknown { .. }
                | OperationObservationStateV1::Expired { .. }
                | OperationObservationStateV1::Recovering { .. }
        )
        || !input.state.is_terminal()
    {
        return Err(before(OperationAttemptError::Association.into()));
    }
    let signer = input.signed.signing_authority;
    let runtime = signer
        .integrated_runtime_authority()
        .ok_or_else(|| before(OperationAttemptError::Association.into()))?;
    let successor = OperationObservationV1::issue_contextual(
        OperationObservationPreimageV1 {
            schema_id: SchemaId::parse(OPERATION_OBSERVATION_V1_SCHEMA)
                .map_err(|_| before(OperationAttemptError::Encoding.into()))?,
            schema_sha256: input.signed.expected_schema_sha256.clone(),
            operation_handle_id: input.handle.operation_handle_id().clone(),
            operation_handle_ref: input.handle_ref.clone(),
            operation_handle_sha256: input.handle.canonical_operation_handle_sha256().clone(),
            revision: predecessor
                .1
                .revision()
                .checked_successor()
                .ok_or_else(|| before(OperationAttemptError::Association.into()))?,
            predecessor_ref: Some(predecessor.0.clone()),
            predecessor_sha256: Some(
                predecessor
                    .1
                    .canonical_operation_observation_sha256()
                    .clone(),
            ),
            state: input.state,
            observed_at: input.observed_at,
            deployment_runtime_authority_ref: runtime.deployment_runtime_authority_ref.clone(),
            deployment_runtime_authority_sha256: runtime
                .deployment_runtime_authority_sha256
                .clone(),
            issuer_principal_id: signer.principal_id().clone(),
            issuer_key_id: pale_core::records::ResidentKeyIdV1::parse(signer.signer_key_id())
                .map_err(|error| before(OperationAttemptError::Admission(error).into()))?,
            issuer_key_epoch: *signer.signer_key_epoch(),
        },
        &OperationObservationAdmissionContextV1 {
            signed: input.signed,
            handle_ref: input.handle_ref,
            handle: input.handle,
            predecessor: Some(predecessor),
        },
        admit_evidence,
        admit_rotation,
        sign_observation,
    )
    .map_err(|error| before(OperationAttemptError::Admission(error).into()))?;
    let basis = CurrentHeadReconciliationBasis {
        store: input.recovered.store().clone(),
        handle_ref: input.handle_ref.clone(),
        handle: input.handle.clone(),
        predecessor: Box::new((predecessor.0.clone(), predecessor.1.clone())),
        predecessor_checkpoint: Box::new((
            predecessor_checkpoint.0.clone(),
            predecessor_checkpoint.1.clone(),
        )),
        successor: Box::new((input.successor_ref, successor)),
        selection: input.selection,
        checkpoint: CurrentHeadReconciliationCheckpointPlan {
            predecessor: Box::new((
                input.recovered.current_checkpoint().0.clone(),
                input.recovered.current_checkpoint().1.clone(),
            )),
            checkpoint: input.checkpoint,
            issued_at: input.observed_at,
            schema: input.checkpoint_schema.clone(),
            authority: RetainedCheckpointAuthority::new(input.checkpoint_authority),
            limits: input.accumulator_limits,
        },
    };
    PreparedCurrentHeadReconciliation { basis }.continue_custody(
        store_resolver,
        custody,
        anchor,
        sign_checkpoint,
    )
}

fn before(failure: OperationAppendFailure) -> CurrentHeadReconciliationAppendFailure {
    CurrentHeadReconciliationAppendFailure::BeforeSuccessor(failure)
}

fn complete_current_head_reconciliation(
    basis: CurrentHeadReconciliationBasis,
    checkpoint: ControlStateCheckpointV1,
    physical: PreparedOperationObservationCustody,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<CompletedCurrentHeadReconciliation, CurrentHeadReconciliationAppendFailure> {
    let checkpoint_pair = (basis.checkpoint.checkpoint.reference.clone(), checkpoint);
    match commit_prepared_checkpoint(physical, checkpoint_pair, custody, anchor) {
        Ok((custody, checkpoint)) => Ok(CompletedCurrentHeadReconciliation {
            successor: *basis.successor,
            checkpoint,
            custody,
        }),
        Err(OperationAppendFailure::Commit {
            checkpoint,
            failure,
        }) => Err(CurrentHeadReconciliationAppendFailure::Commit(Box::new(
            CurrentHeadReconciliationCommitFailure {
                basis,
                checkpoint: checkpoint.1,
                failure,
            },
        ))),
        Err(OperationAppendFailure::Anchor {
            checkpoint,
            failure,
        }) => Err(CurrentHeadReconciliationAppendFailure::Anchor(Box::new(
            CurrentHeadReconciliationAnchorPending {
                basis,
                checkpoint: checkpoint.1,
                custody: CurrentHeadReconciliationAnchorCustody::Failed(failure),
            },
        ))),
        Err(OperationAppendFailure::Operation(_) | OperationAppendFailure::Projection(_)) => {
            unreachable!("an attached checkpoint has no semantic preparation phase")
        }
    }
}

fn complete_reconciliation(
    basis: CurrentHeadReconciliationBasis,
    checkpoint: ControlStateCheckpointV1,
    custody: CompletedOperationObservationCustody,
) -> CompletedCurrentHeadReconciliation {
    CompletedCurrentHeadReconciliation {
        successor: *basis.successor,
        checkpoint: (basis.checkpoint.checkpoint.reference, checkpoint),
        custody,
    }
}
