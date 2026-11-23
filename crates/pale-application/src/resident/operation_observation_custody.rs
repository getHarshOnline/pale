//! Typed projection and physical custody for admitted operation observations.
//!
//! [`OperationObservationV1`] remains the semantic owner of handle lineage,
//! state evidence, chronology, signatures, and runtime-authority rotation. This
//! module accepts those already-admitted values, indexes the latest observation
//! by stable handle identity, requires the checkpoint owner to prove that the signed
//! checkpoint represents the exact append, and delegates atomic custody to the
//! existing operation port. It is a resident-assembly seam, not the live
//! application-owned `Observe` operation.

use pale_core::canon;
use pale_core::records::{
    ControlStateCheckpointV1, InvalidResidentRecordV1, OperationHandleIdV1, OperationHandleV1,
    OperationObservationV1, ResidentRecordRefV1,
};
use pale_ports::{
    AntiRollbackAnchorTransitionError, AuthorityStoreCoordinate, ControlCheckpointHead,
    ControlRecord, ControlRecordCoordinate, ControlRecordError, OperationA2aEvidence,
    OperationCustodyCandidate, OperationCustodyCandidateError, OperationCustodyCheckpoint,
    OperationCustodyIndexKey, OperationCustodyIndexMutation, OperationCustodyIndexPredecessor,
    OperationCustodyIndexQuery, OperationCustodyPort, OperationCustodyRecord,
    OperationCustodyTransitionKind,
};
use serde::Serialize;

use super::operation_custody::{
    prepare_operation_observation_candidate, read_index,
    OperationObservationCandidatePreparationError, OperationRecoveryError,
    PreparedOperationObservationCustody,
};
use super::record_projection::{self, ControlRecordProjectionError};

mod dispatch_custody;
#[allow(
    dead_code,
    reason = "the private reconciliation append awaits assembled resident attachment"
)]
pub(in crate::resident) mod reconciliation_append;
mod recovery;
pub use dispatch_custody::OperationDispatchCustodyError;
pub(in crate::resident) use recovery::{
    read_history, recover_with_checkpoint, OperationObservationRecoveryIdentity,
    OperationObservationRecoverySource, OperationObservationRecoveryView,
};
pub use recovery::{
    recover_operation_observation, OperationObservationReadmissionError,
    OperationObservationRecoveryAdmission, OperationObservationRecoveryInput,
    OperationObservationRecoveryLimits, OperationObservationSelection,
    RecoveredOperationObservation,
};

/// Domain separator for handle identity within one control-store incarnation.
const OPERATION_HANDLE_KEY_DOMAIN: &[u8] = b"PALE_OPERATION_HANDLE_INDEX_KEY_V1\0";

#[derive(Serialize)]
struct OperationHandleKeyPreimage<'a> {
    initial_v1_domain: &'static str,
    operation_handle_id: &'a str,
}

/// Already-admitted handle and exact observation successor pair.
///
/// Construction does not itself admit the records. Callers may create this
/// value only after [`OperationObservationV1::issue_contextual`] or
/// [`OperationObservationV1::admit_contextual_wire`] has accepted `successor`
/// against the supplied handle and predecessor.
#[derive(Clone, Copy)]
pub struct OperationObservationTransition<'a> {
    handle_ref: &'a ResidentRecordRefV1,
    handle: &'a OperationHandleV1,
    predecessor_ref: &'a ResidentRecordRefV1,
    predecessor: &'a OperationObservationV1,
    successor_ref: &'a ResidentRecordRefV1,
    successor: &'a OperationObservationV1,
}

impl<'a> OperationObservationTransition<'a> {
    /// Bind already-admitted records to their exact immutable references.
    #[must_use]
    pub const fn new(
        handle_ref: &'a ResidentRecordRefV1,
        handle: &'a OperationHandleV1,
        predecessor_ref: &'a ResidentRecordRefV1,
        predecessor: &'a OperationObservationV1,
        successor_ref: &'a ResidentRecordRefV1,
        successor: &'a OperationObservationV1,
    ) -> Self {
        Self {
            handle_ref,
            handle,
            predecessor_ref,
            predecessor,
            successor_ref,
            successor,
        }
    }
}

/// Exact admitted checkpoint pair plus its construction verifier.
#[derive(Clone, Copy)]
pub struct OperationObservationCheckpointTransition<'a> {
    predecessor_ref: &'a ResidentRecordRefV1,
    predecessor: &'a ControlStateCheckpointV1,
    successor_ref: &'a ResidentRecordRefV1,
    successor: &'a ControlStateCheckpointV1,
    verifier: &'a dyn OperationObservationCheckpointConstructionVerifier,
}

impl<'a> OperationObservationCheckpointTransition<'a> {
    /// Bind one admitted checkpoint successor to its accumulator-proof owner.
    #[must_use]
    pub const fn new(
        predecessor_ref: &'a ResidentRecordRefV1,
        predecessor: &'a ControlStateCheckpointV1,
        successor_ref: &'a ResidentRecordRefV1,
        successor: &'a ControlStateCheckpointV1,
        verifier: &'a dyn OperationObservationCheckpointConstructionVerifier,
    ) -> Self {
        Self {
            predecessor_ref,
            predecessor,
            successor_ref,
            successor,
            verifier,
        }
    }
}

/// Exact observation append shown to the checkpoint-construction owner.
pub struct OperationObservationCheckpointConstruction<'a> {
    store: &'a AuthorityStoreCoordinate,
    transition: &'a OperationObservationTransition<'a>,
    predecessor_record: &'a ControlRecord,
    successor_record: &'a ControlRecord,
    associated_records: &'a [OperationCustodyRecord],
    associated_indexes: &'a [OperationCustodyIndexMutation],
    a2a_evidence: &'a [OperationA2aEvidence],
    latest_observation_mutation: &'a OperationCustodyIndexMutation,
    checkpoint: &'a OperationCustodyCheckpoint,
}

impl OperationObservationCheckpointConstruction<'_> {
    /// Borrow the selected deployment-control store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        self.store
    }

    /// Borrow the exact admitted handle.
    #[must_use]
    pub const fn handle(&self) -> (&ResidentRecordRefV1, &OperationHandleV1) {
        (self.transition.handle_ref, self.transition.handle)
    }

    /// Borrow the exact predecessor observation.
    #[must_use]
    pub const fn predecessor(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (self.transition.predecessor_ref, self.transition.predecessor)
    }

    /// Borrow the exact successor observation.
    #[must_use]
    pub const fn successor(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (self.transition.successor_ref, self.transition.successor)
    }

    /// Borrow the exact projected predecessor record.
    #[must_use]
    pub const fn predecessor_record(&self) -> &ControlRecord {
        self.predecessor_record
    }

    /// Borrow the exact projected successor record.
    #[must_use]
    pub const fn successor_record(&self) -> &ControlRecord {
        self.successor_record
    }

    /// Borrow the additional records retained by this observation.
    ///
    /// The application admits either no record, one existing single-record
    /// shape, or the exact adapter-failure, capability-result, and diagnostic
    /// atom required by a safe provider failure. The custody port rejects every
    /// other Observe shape.
    #[must_use]
    pub const fn associated_records(&self) -> &[OperationCustodyRecord] {
        self.associated_records
    }

    /// Borrow private indexes inseparable from the associated records.
    #[must_use]
    pub const fn associated_indexes(&self) -> &[OperationCustodyIndexMutation] {
        self.associated_indexes
    }

    /// Borrow the bounded A2A protocol items retained with this observation.
    #[must_use]
    pub const fn a2a_evidence(&self) -> &[OperationA2aEvidence] {
        self.a2a_evidence
    }

    /// Borrow the exact private latest-observation mutation.
    #[must_use]
    pub const fn latest_observation_mutation(&self) -> &OperationCustodyIndexMutation {
        self.latest_observation_mutation
    }

    /// Borrow the exact projected checkpoint transition.
    #[must_use]
    pub const fn checkpoint(&self) -> &OperationCustodyCheckpoint {
        self.checkpoint
    }
}

/// Owner boundary proving that a signed checkpoint folds this exact append.
pub trait OperationObservationCheckpointConstructionVerifier {
    /// Verify the complete observation/index mutation represented by the successor.
    ///
    /// # Errors
    ///
    /// Returns [`OperationObservationCheckpointConstructionRejection`] when the
    /// signed checkpoint was not constructed from this exact mutation.
    fn verify(
        &self,
        construction: &OperationObservationCheckpointConstruction<'_>,
    ) -> Result<(), OperationObservationCheckpointConstructionRejection>;
}

impl<F> OperationObservationCheckpointConstructionVerifier for F
where
    F: Fn(
        &OperationObservationCheckpointConstruction<'_>,
    ) -> Result<(), OperationObservationCheckpointConstructionRejection>,
{
    fn verify(
        &self,
        construction: &OperationObservationCheckpointConstruction<'_>,
    ) -> Result<(), OperationObservationCheckpointConstructionRejection> {
        self(construction)
    }
}

/// The checkpoint owner rejected the projected observation mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("control checkpoint does not represent the exact operation-observation mutation")]
pub struct OperationObservationCheckpointConstructionRejection;

/// Owner boundary resolving an observation's admitted runtime authority to its store.
///
/// The resolver receives the exact signed successor rather than a caller-
/// selected store coordinate. Its implementation must resolve the observation's
/// runtime-authority reference and digest to the admitted authority record,
/// verify its principal, key, epoch, and permitted schema, then return that
/// authority's deployment-control store incarnation.
pub trait OperationObservationAuthorityStoreResolver {
    /// Resolve the exact authority-owned store for one admitted observation.
    ///
    /// # Errors
    ///
    /// Returns [`OperationObservationAuthorityStoreRejection`] when the
    /// observation's runtime authority cannot be resolved exactly.
    fn resolve(
        &self,
        observation: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection>;
}

impl<F> OperationObservationAuthorityStoreResolver for F
where
    F: Fn(
        &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection>,
{
    fn resolve(
        &self,
        observation: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        self(observation)
    }
}

/// The runtime-authority owner could not bind an observation to one exact store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("operation observation runtime authority does not resolve to one admitted store")]
pub struct OperationObservationAuthorityStoreRejection;

/// Projection failure before operation-custody access.
#[derive(Debug, thiserror::Error)]
pub enum OperationObservationCustodyProjectionError {
    /// Immutable profile content or its original-request association differs.
    #[error(transparent)]
    Profile(#[from] super::checkpoint_profile::OperationCheckpointProfileError),
    /// Logical changes exceed preparation bounds or use invalid leaf encodings.
    #[error(transparent)]
    AccumulatorInput(#[from] pale_ports::AccumulatorPreparationInputError),
    /// Read-only preparation failed; budget exhaustion is distinct from corruption.
    #[error(transparent)]
    AccumulatorPreparation(#[from] pale_ports::OperationAccumulatorPreparationError),
    /// Physical tree changes do not form the requested checkpoint update.
    #[error(transparent)]
    AccumulatorBatch(#[from] pale_ports::AccumulatorBatchError),
    /// An admitted semantic record failed its invariant verification.
    #[error(transparent)]
    Semantic(#[from] InvalidResidentRecordV1),
    /// Handle and predecessor/successor records do not form one exact lineage.
    #[error("operation observations do not form one exact handle-local successor")]
    Lineage,
    /// The runtime-authority owner rejected the observation-to-store join.
    #[error(transparent)]
    RuntimeAuthority(#[from] OperationObservationAuthorityStoreRejection),
    /// Checkpoints do not belong to the authority-derived store incarnation.
    #[error("operation checkpoint does not belong to the authority-derived store")]
    StoreScope,
    /// A canonical private key or record could not be encoded.
    #[error(transparent)]
    Canonicalize(#[from] canon::CanonError),
    /// A projected immutable record was not physically admissible.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
    /// The closed operation-custody candidate rejected the projection.
    #[error(transparent)]
    Candidate(#[from] OperationCustodyCandidateError),
    /// The checkpoint could not form the exact external-anchor successor.
    #[error(transparent)]
    Anchor(#[from] AntiRollbackAnchorTransitionError),
    /// The checkpoint owner rejected the exact mutation witness.
    #[error(transparent)]
    Checkpoint(#[from] OperationObservationCheckpointConstructionRejection),
}

impl From<ControlRecordProjectionError> for OperationObservationCustodyProjectionError {
    fn from(value: ControlRecordProjectionError) -> Self {
        match value {
            ControlRecordProjectionError::Canonicalize(error) => Self::Canonicalize(error),
            ControlRecordProjectionError::Record(error) => Self::Record(error),
        }
    }
}

/// Project one already-admitted observation append before constructing its checkpoint.
///
/// Recheck the handle-local lineage and resolve its authority-owned store,
/// then prepare the canonical records and latest-observation mutation once.
/// The result can be inspected by a checkpoint producer, but cannot commit,
/// advance an anchor, or grant a continuation. It does not select an
/// accumulator profile or prove that a checkpoint represents this append.
///
/// # Errors
///
/// Returns [`OperationObservationCustodyProjectionError`] when semantic lineage,
/// runtime-authority resolution, or canonical projection fails. This function
/// does not call custody or anchor ports; the supplied resolver owns its own work.
pub fn prepare_operation_observation_append<'a>(
    transition: OperationObservationTransition<'a>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
) -> Result<PreparedOperationObservationAppend<'a>, OperationObservationCustodyProjectionError> {
    verify_transition(&transition)?;
    let store = resolve_store.resolve(transition.successor)?;
    let latest_observation_key = operation_handle_key(transition.handle.operation_handle_id())?;
    let predecessor = project_observation(transition.predecessor_ref, transition.predecessor)?;
    let successor = project_observation(transition.successor_ref, transition.successor)?;
    let successor_coordinate = successor.coordinate().clone();
    let latest_observation_mutation = OperationCustodyIndexMutation::LatestObservation {
        key: latest_observation_key,
        expected: OperationCustodyIndexPredecessor::Exact(predecessor.clone()),
        successor: successor_coordinate,
    };
    Ok(PreparedOperationObservationAppend {
        store,
        transition,
        predecessor_record: predecessor,
        successor_record: successor,
        associated_records: Vec::new(),
        associated_indexes: Vec::new(),
        a2a_evidence: Vec::new(),
        latest_observation_mutation,
    })
}

/// Validated observation mutation awaiting an exact checkpoint construction proof.
///
/// Records are borrowed from the admitted transition; their canonical physical
/// projections are owned here. This is neither persisted observation evidence
/// nor permission to continue execution. Dropping it has no storage effect.
///
/// ```compile_fail
/// use pale_application::resident::PreparedOperationObservationAppend;
/// use pale_ports::{AntiRollbackAnchorPort, OperationCustodyPort};
/// fn cannot_commit_before_attachment(
///     append: PreparedOperationObservationAppend<'_>,
///     store: &mut dyn OperationCustodyPort,
///     anchor: &mut dyn AntiRollbackAnchorPort,
/// ) {
///     append.commit(store, anchor);
/// }
/// ```
#[must_use = "the append needs an exact checkpoint before physical custody"]
pub struct PreparedOperationObservationAppend<'a> {
    store: AuthorityStoreCoordinate,
    transition: OperationObservationTransition<'a>,
    predecessor_record: ControlRecord,
    successor_record: ControlRecord,
    associated_records: Vec<OperationCustodyRecord>,
    associated_indexes: Vec<OperationCustodyIndexMutation>,
    a2a_evidence: Vec<OperationA2aEvidence>,
    latest_observation_mutation: OperationCustodyIndexMutation,
}

impl std::fmt::Debug for PreparedOperationObservationAppend<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedOperationObservationAppend")
            .finish_non_exhaustive()
    }
}

impl PreparedOperationObservationAppend<'_> {
    /// Borrow the store incarnation resolved from the observation's authority.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the admitted handle to which this append belongs.
    #[must_use]
    pub const fn handle(&self) -> (&ResidentRecordRefV1, &OperationHandleV1) {
        (self.transition.handle_ref, self.transition.handle)
    }

    /// Borrow the admitted predecessor observation.
    #[must_use]
    pub const fn predecessor(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (self.transition.predecessor_ref, self.transition.predecessor)
    }

    /// Borrow the admitted successor observation.
    #[must_use]
    pub const fn successor(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (self.transition.successor_ref, self.transition.successor)
    }

    /// Borrow the predecessor bytes used by the exact compare-and-set mutation.
    #[must_use]
    pub const fn predecessor_record(&self) -> &ControlRecord {
        &self.predecessor_record
    }

    /// Borrow the canonical signed bytes to append.
    #[must_use]
    pub const fn successor_record(&self) -> &ControlRecord {
        &self.successor_record
    }

    /// Borrow the attempt selection or result retained with this observation.
    #[must_use]
    pub fn associated_record(&self) -> Option<&OperationCustodyRecord> {
        (self.associated_records.len() == 1).then(|| &self.associated_records[0])
    }

    /// Borrow every associated record selected by the application phase.
    #[must_use]
    pub fn associated_records(&self) -> &[OperationCustodyRecord] {
        &self.associated_records
    }

    /// Borrow private indexes attached by the owning application phase.
    #[must_use]
    pub(in crate::resident) fn associated_indexes(&self) -> &[OperationCustodyIndexMutation] {
        &self.associated_indexes
    }

    // Only an application phase may attach the record it has semantically joined.
    pub(in crate::resident) fn with_associated_record(
        mut self,
        record: OperationCustodyRecord,
    ) -> Self {
        self.associated_records = vec![record];
        self
    }

    // Only native outcome custody may add the exact dispatch-to-outcome index.
    pub(in crate::resident) fn with_native_outcome(
        mut self,
        record: OperationCustodyRecord,
        dispatch: &ControlRecordCoordinate,
    ) -> Self {
        debug_assert_eq!(
            record.kind(),
            pale_ports::OperationCustodyRecordKind::NativeOutcome
        );
        let successor = record.record().coordinate().clone();
        self.associated_records = vec![record];
        self.associated_indexes = vec![OperationCustodyIndexMutation::NativeOutcome {
            key: OperationCustodyIndexKey::for_native_outcome(dispatch),
            expected: OperationCustodyIndexPredecessor::Absent,
            successor,
        }];
        self
    }

    // Only provider completion may attach this exact three-owner failure atom.
    pub(in crate::resident) fn with_provider_failure_records(
        mut self,
        failure: OperationCustodyRecord,
        result: OperationCustodyRecord,
        diagnostic: OperationCustodyRecord,
    ) -> Self {
        debug_assert_eq!(
            failure.kind(),
            pale_ports::OperationCustodyRecordKind::AdapterFailure
        );
        debug_assert_eq!(
            result.kind(),
            pale_ports::OperationCustodyRecordKind::CapabilityResult
        );
        debug_assert_eq!(
            diagnostic.kind(),
            pale_ports::OperationCustodyRecordKind::Diagnostic
        );
        self.associated_records = vec![failure, result, diagnostic];
        self
    }

    // Only an A2A owner may attach the bounded bytes whose semantic roles it admitted.
    pub(in crate::resident) fn with_a2a_evidence_items(
        mut self,
        evidence: Vec<OperationA2aEvidence>,
    ) -> Self {
        self.a2a_evidence = evidence;
        self
    }

    /// Borrow the bounded A2A protocol items selected for this append.
    #[must_use]
    pub fn a2a_evidence(&self) -> &[OperationA2aEvidence] {
        &self.a2a_evidence
    }

    /// Borrow the primary index mutation, before checkpoint-derived recovery indexes.
    #[must_use]
    pub const fn latest_observation_mutation(&self) -> &OperationCustodyIndexMutation {
        &self.latest_observation_mutation
    }

    /// Attach a checkpoint pair whose construction owner proves this exact mutation.
    ///
    /// The existing closed candidate derives the observation-to-checkpoint
    /// recovery index only after verification. This does not decide which
    /// logical leaves an accumulator profile authenticates. The result still
    /// requires physical commit and independent anchor confirmation.
    /// The supplied physical batch must match both signed checkpoints' roots,
    /// immutable profile and store; matching record heads alone is insufficient.
    ///
    /// # Errors
    ///
    /// Returns an error for unlike checkpoint succession or store incarnation,
    /// failed construction proof, invalid closed candidate, or invalid anchor
    /// transition. Observation records are not reprojected; only the attached
    /// checkpoints are projected here. This phase does not call custody or
    /// anchor ports; the supplied construction verifier owns its own I/O.
    pub fn attach_checkpoint(
        self,
        checkpoints: OperationObservationCheckpointTransition<'_>,
        accumulator: pale_ports::OperationAccumulatorBatch,
    ) -> Result<PreparedOperationObservationCustody, OperationObservationCustodyProjectionError>
    {
        checkpoints
            .successor
            .verify_successor_of(checkpoints.predecessor_ref, checkpoints.predecessor)?;
        if checkpoints.predecessor.control_store_id() != self.store.store_id()
            || checkpoints.predecessor.control_incarnation_id() != self.store.incarnation_id()
            || checkpoints.successor.control_store_id() != self.store.store_id()
            || checkpoints.successor.control_incarnation_id() != self.store.incarnation_id()
        {
            return Err(OperationObservationCustodyProjectionError::StoreScope);
        }
        super::checkpoint_profile::verify_checkpoint_batch(
            &self.store,
            (checkpoints.predecessor_ref, checkpoints.predecessor),
            (checkpoints.successor_ref, checkpoints.successor),
            &accumulator,
        )?;
        let checkpoint = project_checkpoint(&checkpoints)?;
        checkpoints
            .verifier
            .verify(&OperationObservationCheckpointConstruction {
                store: &self.store,
                transition: &self.transition,
                predecessor_record: &self.predecessor_record,
                successor_record: &self.successor_record,
                associated_records: &self.associated_records,
                associated_indexes: &self.associated_indexes,
                a2a_evidence: &self.a2a_evidence,
                latest_observation_mutation: &self.latest_observation_mutation,
                checkpoint: &checkpoint,
            })?;
        let mut records = vec![OperationCustodyRecord::OperationObservation(
            self.successor_record,
        )];
        records.extend(self.associated_records);
        let mut indexes = vec![self.latest_observation_mutation];
        indexes.extend(self.associated_indexes);
        let candidate = OperationCustodyCandidate::new(
            self.store,
            OperationCustodyTransitionKind::Observe,
            records,
            indexes,
            checkpoint,
            accumulator,
        )?;
        let candidate = if self.a2a_evidence.is_empty() {
            candidate
        } else {
            candidate.with_a2a_evidence_items(self.a2a_evidence)?
        };
        prepare_operation_observation_candidate(candidate).map_err(|error| match error {
            OperationObservationCandidatePreparationError::NotObservation => {
                OperationObservationCustodyProjectionError::Lineage
            }
            OperationObservationCandidatePreparationError::Anchor(error) => {
                OperationObservationCustodyProjectionError::Anchor(error)
            }
        })
    }
}

fn verify_transition(
    transition: &OperationObservationTransition<'_>,
) -> Result<(), OperationObservationCustodyProjectionError> {
    transition.successor.verify_admitted_successor_of(
        transition.handle_ref,
        transition.handle,
        transition.predecessor_ref,
        transition.predecessor,
    )?;
    require(
        transition.predecessor_ref != transition.successor_ref
            && transition.predecessor.operation_handle_ref() == transition.handle_ref
            && transition.predecessor.operation_handle_sha256()
                == transition.handle.canonical_operation_handle_sha256(),
    )
}

/// Read retained observation evidence for an exact already-admitted handle.
///
/// The caller must separately establish permission to inspect `coordinate`.
/// This lookup compares the stored handle with the supplied immutable reference
/// and canonical value, selects its latest-observation index once, and reads
/// the immutable checkpoint indexed by that selected observation. It performs
/// at most three point reads, each bounded by `maximum_record_bytes`, and never
/// writes, consults an anchor, requests cancellation, or enables execution.
///
/// These reads are not one storage snapshot. A later append may advance the
/// head while the selected observation and its committing checkpoint remain
/// historical evidence. The result does not promise latest-at-return state,
/// authenticate semantic linkage, or re-admit current authority. In particular,
/// a correctly directed record with the expected schema still needs contextual
/// admission of its contents before any public `Observe` result or action.
///
/// # Errors
///
/// Returns [`OperationRecoveryError`] for a conflicting handle, partial indexes,
/// misdirected or oversized responses, projection failure, or storage failure.
/// `None` means both handle and latest-observation indexes were absent; a
/// missing half or committing checkpoint is never treated as absence.
pub fn read_operation_observation_for_handle(
    store: &mut dyn OperationCustodyPort,
    coordinate: &AuthorityStoreCoordinate,
    handle_ref: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
    maximum_record_bytes: usize,
) -> Result<Option<OperationObservationRecoveryEvidence>, OperationRecoveryError> {
    let key = operation_handle_key(handle.operation_handle_id())?;
    let expected_handle = record_projection::record(
        record_projection::coordinate(
            handle_ref,
            handle.canonical_operation_handle_sha256().as_str(),
        )?,
        handle,
    )?;
    let retained_handle = read_index(
        store,
        coordinate,
        OperationCustodyIndexQuery::OperationHandle(key.clone()),
        maximum_record_bytes,
    )?;
    if retained_handle
        .as_ref()
        .is_some_and(|retained| retained != &expected_handle)
    {
        return Err(OperationRecoveryError::HandleConflict);
    }
    let observation = read_index(
        store,
        coordinate,
        OperationCustodyIndexQuery::LatestObservation(key),
        maximum_record_bytes,
    )?;
    match (retained_handle, observation) {
        (None, None) => Ok(None),
        (Some(handle), Some(observation)) => {
            let checkpoint = read_index(
                store,
                coordinate,
                OperationCustodyIndexQuery::ObservationCheckpoint(
                    OperationCustodyIndexKey::for_observation(observation.coordinate()),
                ),
                maximum_record_bytes,
            )?
            .ok_or(OperationRecoveryError::IncompleteObservationIndexes)?;
            Ok(Some(OperationObservationRecoveryEvidence {
                store: coordinate.clone(),
                handle,
                observation,
                checkpoint,
            }))
        }
        (Some(_), None) | (None, Some(_)) => {
            Err(OperationRecoveryError::IncompleteObservationIndexes)
        }
    }
}

/// Physical evidence selected by one handle-local observation lookup.
///
/// The handle is byte-exact with the caller's admitted value. Observation and
/// checkpoint records are schema-role checked, not contextually re-admitted.
/// Their signatures, semantic linkage, retention and checkpoint ancestry need
/// their existing admission owners before an operation can be exposed. A head
/// may advance after selection: this is neither a shared snapshot nor a promise
/// of latest-at-return state, and grants no current authority or action.
#[derive(Debug)]
pub struct OperationObservationRecoveryEvidence {
    store: AuthorityStoreCoordinate,
    handle: ControlRecord,
    observation: ControlRecord,
    checkpoint: ControlRecord,
}

impl OperationObservationRecoveryEvidence {
    /// Borrow the deployment-control incarnation queried for these records.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the exact retained handle without renewing its authority.
    #[must_use]
    pub const fn handle(&self) -> &ControlRecord {
        &self.handle
    }

    /// Borrow the immutable observation selected when its latest index was read.
    #[must_use]
    pub const fn observation(&self) -> &ControlRecord {
        &self.observation
    }

    /// Borrow the checkpoint indexed by the selected observation, not a later head.
    #[must_use]
    pub const fn checkpoint(&self) -> &ControlRecord {
        &self.checkpoint
    }
}

/// Keep conflicting contents under the same stable identity in the same slot.
///
/// The port scopes this key by store incarnation and index role. Handle
/// references and digests remain in the stored records and exact predecessor
/// checks; including them here would let changed content evade those checks.
pub(super) fn operation_handle_key(
    handle_id: &OperationHandleIdV1,
) -> Result<OperationCustodyIndexKey, ControlRecordProjectionError> {
    let canonical = canon::to_canonical_string(&OperationHandleKeyPreimage {
        initial_v1_domain: "operation_handle_identity",
        operation_handle_id: handle_id.as_str(),
    })?;
    let mut bytes = Vec::with_capacity(OPERATION_HANDLE_KEY_DOMAIN.len() + canonical.len());
    bytes.extend_from_slice(OPERATION_HANDLE_KEY_DOMAIN);
    bytes.extend_from_slice(canonical.as_bytes());
    OperationCustodyIndexKey::parse(pale_core::sha256_hex(&bytes)).map_err(Into::into)
}

fn project_observation(
    record_ref: &ResidentRecordRefV1,
    observation: &OperationObservationV1,
) -> Result<ControlRecord, OperationObservationCustodyProjectionError> {
    let coordinate = record_projection::coordinate(
        record_ref,
        observation
            .canonical_operation_observation_sha256()
            .as_str(),
    )?;
    record_projection::record(coordinate, observation).map_err(Into::into)
}

fn project_checkpoint(
    checkpoints: &OperationObservationCheckpointTransition<'_>,
) -> Result<OperationCustodyCheckpoint, OperationObservationCustodyProjectionError> {
    let predecessor =
        project_checkpoint_record(checkpoints.predecessor_ref, checkpoints.predecessor)?;
    let successor = project_checkpoint_record(checkpoints.successor_ref, checkpoints.successor)?;
    OperationCustodyCheckpoint::new(
        pale_ports::OperationCheckpointPhase::Successor {
            predecessor: ControlCheckpointHead::new(
                checkpoints.predecessor.checkpoint_domain_id().clone(),
                *checkpoints.predecessor.domain_epoch(),
                *checkpoints.predecessor.domain_sequence(),
                *checkpoints.predecessor.committed_transaction_sequence(),
                predecessor.coordinate().clone(),
            ),
            predecessor_record: Box::new(predecessor),
        },
        ControlCheckpointHead::new(
            checkpoints.successor.checkpoint_domain_id().clone(),
            *checkpoints.successor.domain_epoch(),
            *checkpoints.successor.domain_sequence(),
            *checkpoints.successor.committed_transaction_sequence(),
            successor.coordinate().clone(),
        ),
        successor,
    )
    .map_err(Into::into)
}

fn project_checkpoint_record(
    record_ref: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<ControlRecord, OperationObservationCustodyProjectionError> {
    let coordinate = record_projection::coordinate(
        record_ref,
        checkpoint.canonical_control_checkpoint_sha256().as_str(),
    )?;
    record_projection::record(coordinate, checkpoint).map_err(Into::into)
}

fn require(exact: bool) -> Result<(), OperationObservationCustodyProjectionError> {
    exact
        .then_some(())
        .ok_or(OperationObservationCustodyProjectionError::Lineage)
}

#[cfg(test)]
#[path = "../../tests/internal/resident/operation_observation_custody.rs"]
pub(in crate::resident) mod tests;
