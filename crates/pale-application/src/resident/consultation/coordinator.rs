//! Private composition of durable consultation collection custody.
//!
//! The roster, child operations, policy decisions, record signing, and physical
//! ports keep their existing owners. This coordinator only sequences those
//! owners over one custody and independent-anchor session. It starts no child,
//! selects no participant or provider, runs no carrier, and synthesizes no
//! parent result.

use super::*;
use pale_ports::{AntiRollbackAnchorPort, OperationCustodyPort};

/// Inputs that open one already-admitted consultation collection.
pub(in crate::resident) struct ResidentConsultationOpenInput<'a> {
    /// Exact admitted collection whose fresh children are already durable.
    pub collection: ConsultationCollection,
    /// Already-issued Consult parent.
    pub parent: &'a IssuedLocalOperation,
    /// Parent's current resolving observation.
    pub parent_observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    /// Checkpoint that retains the resolving observation.
    pub parent_checkpoint: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    /// Fresh observation identity, signer, time, and initial-frontier position.
    pub observation: ConsultationCollectionObservationInput<'a>,
    /// Count and byte ceilings for the complete initial collection.
    pub limits: ConsultationCollectionLimits,
    /// Fresh checkpoint identity, signer, time, and accumulator ceilings.
    pub checkpoint: ConsultationSuccessorCommitInput<'a>,
}

/// Where durable consultation opening stopped.
pub(in crate::resident) enum ResidentConsultationOpenFailure {
    /// Collection meaning or the resolving-parent transition did not admit.
    Admission(ConsultationCollectionError),
    /// The signed collection did not complete database and independent-anchor custody.
    Custody(ConsultationCollectionCommitFailure),
}

impl std::fmt::Debug for ResidentConsultationOpenFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Admission(_) => "Admission",
            Self::Custody(_) => "Custody",
        };
        formatter
            .debug_struct("ResidentConsultationOpenFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Recovery-first disposition of one exact consultation child.
///
/// Only [`Self::EligibleForFirstDispatch`] may enter a child's existing service
/// as a fresh operation. [`Self::ResumeOnly`] has already crossed a child-owned
/// phase and must be resumed or reconciled there. [`Self::TerminalJoin`] can be
/// joined to the parent without running the child again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum ProtectedConsultationChildPhase {
    /// The child remains before provider resolution or dispatch.
    EligibleForFirstDispatch,
    /// The child requires its existing continuation or recovery owner.
    ResumeOnly,
    /// The child has conclusive terminal evidence ready for a parent join.
    TerminalJoin,
}

/// Private resident composition over one operation store and anchor session.
pub(in crate::resident) struct ResidentConsultationCoordinator<'ports> {
    custody: &'ports mut dyn OperationCustodyPort,
    anchor: &'ports mut dyn AntiRollbackAnchorPort,
}

impl<'ports> ResidentConsultationCoordinator<'ports> {
    /// Bind collection transitions to the already-selected custody owners.
    pub(in crate::resident) fn new(
        custody: &'ports mut dyn OperationCustodyPort,
        anchor: &'ports mut dyn AntiRollbackAnchorPort,
    ) -> Self {
        Self { custody, anchor }
    }

    /// Open one admitted roster and expose it only after both custody layers.
    ///
    /// Fresh child handles must already have completed their own start custody.
    /// This method does not start, reserve, dispatch, or cancel them.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn open<
        'operation,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        input: ResidentConsultationOpenInput<'operation>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<CompletedConsultationCollection, ResidentConsultationOpenFailure>
    where
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
        let ResidentConsultationOpenInput {
            collection,
            parent,
            parent_observation,
            parent_checkpoint,
            observation,
            limits,
            checkpoint,
        } = input;
        let prepared = collection
            .prepare_observation(
                parent,
                parent_checkpoint,
                parent_observation,
                observation,
                limits,
                resolve_store,
                self.custody,
                admit_collection,
                admit_rotation,
                sign_observation,
            )
            .map_err(ResidentConsultationOpenFailure::Admission)?;
        commit_consultation_collection(
            prepared,
            ConsultationCollectionCommitInput {
                accumulator_limits: checkpoint.accumulator_limits,
                predecessor: parent_checkpoint,
                identity: checkpoint.identity,
                issued_at: checkpoint.issued_at,
                schema: checkpoint.schema,
                authority: checkpoint.authority,
            },
            resolve_store,
            self.custody,
            self.anchor,
            sign_checkpoint,
        )
        .map_err(ResidentConsultationOpenFailure::Custody)
    }

    /// Recover the exact current collection through one protected operation head.
    ///
    /// The protected observation has already admitted the complete operation
    /// history, its current checkpoint ancestry, and both independent-anchor
    /// observations. Recovery refuses a substituted handle or store before it
    /// reads the collection point index.
    pub(in crate::resident) fn recover<Admit>(
        &mut self,
        operation: &IssuedLocalOperation,
        protected: &RecoveredOperationObservation,
        limits: ConsultationCollectionLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        admit: Admit,
    ) -> Result<Option<RecoveredConsultationCollection>, ConsultationCollectionError>
    where
        Admit: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
    {
        let (handle_ref, handle) = operation.handle();
        if protected.handle.record_ref() != handle_ref
            || protected.handle.record_sha256().as_str()
                != handle.canonical_operation_handle_sha256().as_str()
        {
            return Err(ConsultationCollectionError::Association);
        }
        let observation = protected.selected_head().1;
        let exact_store = protected.store();
        let exact_resolver = |observation: &OperationObservationV1| {
            let resolved = resolve_store.resolve(observation)?;
            if &resolved != exact_store {
                return Err(crate::resident::OperationObservationAuthorityStoreRejection);
            }
            Ok(resolved)
        };
        ConsultationCollection::recover_current(
            operation,
            observation,
            limits,
            &exact_resolver,
            self.custody,
            admit,
        )
    }

    /// Classify one protected child without granting a second release.
    ///
    /// The candidate must remain outstanding in the exact planned parent batch.
    /// The recovered child must name the same handle and store. A child that has
    /// reached provider resolution, dispatch, cancellation, uncertainty, nested
    /// consultation, or expiry is resume-only here. The owning child service
    /// decides its continuation; this coordinator never replays it.
    pub(in crate::resident) fn classify_child(
        &self,
        current: &RecoveredConsultationCollection,
        planned: &ConsultationChildDispatchBatch,
        candidate: &ConsultationChildDispatchCandidate,
        now: UnixTimeV1,
        child: &RecoveredOperationObservation,
    ) -> Result<ProtectedConsultationChildPhase, ConsultationCollectionError> {
        super::service::classify_protected_child(current, planned, candidate, now, child)
    }

    /// Join one protected terminal child through the current parent frontier.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn join_child<
        'operation,
        AdmitEvidence,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        recovered: &RecoveredConsultationCollection,
        input: ConsultationChildJoinInput<'operation>,
        checkpoint: ConsultationSuccessorCommitInput<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
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
        commit_recovered_consultation_child(
            recovered,
            input,
            checkpoint,
            resolve_store,
            self.custody,
            self.anchor,
            admit_evidence,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Stop one recovered collection under its existing policy owner.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn stop<
        'operation,
        AdmitStop,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        recovered: &RecoveredConsultationCollection,
        input: ConsultationStopInput<'operation>,
        checkpoint: ConsultationSuccessorCommitInput<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
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
        commit_recovered_consultation_stop(
            recovered,
            input,
            checkpoint,
            resolve_store,
            self.custody,
            self.anchor,
            admit_stop,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Freeze one recovered collection under an already-authorized cancellation.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn cancel<
        'authority,
        AdmitStop,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        authorized: &AuthorizedOperationCancellation<'authority>,
        recovered: &RecoveredConsultationCollection,
        input: ConsultationCancellationInput<'_>,
        checkpoint: ConsultationSuccessorCommitInput<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
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
        commit_recovered_consultation_cancellation(
            authorized,
            recovered,
            input,
            checkpoint,
            resolve_store,
            self.custody,
            self.anchor,
            admit_stop,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }
}
