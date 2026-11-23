//! Application closure for principal-wide producer-incarnation transitions.
//!
//! The signed record families admit allocation, retirement, and checkpoint
//! meaning. This module consumes those admitted values and complete current
//! producer-head set `H`, then proves the finite update to `H'` and its exact
//! trust-checkpoint root. In the rendered law, `key` is the canonical
//! [`ProducerIncarnationHeadV1`] ordering and `headRoot(C(n))` is the root
//! authenticated by trust checkpoint `C` at sequence `n`:

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/principal-producer-head-update.display.mathml"
))]
//!
//! It closes the resulting producer, principal-trust, and hard-domain heads
//! into one physical candidate. The candidate carries no authority to generate
//! an epoch, infer a checkpoint accumulator, advance an external anchor, admit
//! an owner outbox, or activate a runtime.
//!
//! Restart recovery enumerates the complete physical head set under one exact
//! signed trust-checkpoint head, re-admits every allocation and optional
//! retirement, and reproduces the checkpoint root before a later transition
//! may consume that frontier.
use pale_core::canon;
use pale_core::records::{
    producer_incarnation_heads_root_sha256, ControlStateCheckpointV1, InvalidResidentRecordV1,
    PrincipalTrustCheckpointV1, ProducerEpochV1, ProducerIncarnationHeadAdmissionStateV1,
    ProducerIncarnationHeadStateV1, ProducerIncarnationHeadV1, ProducerIncarnationHeadsRootDigest,
    ProducerIncarnationRetirementV1, ProducerIncarnationRetirementWireV1, ProducerIncarnationV1,
    ProducerIncarnationWireV1, ResidentRecordRefV1, MAX_RESIDENT_SET_MEMBERS,
};
use pale_ports::{
    ControlCheckpointHead, ControlRecord, ControlRecordError, ControlStoreError,
    ControlStoreErrorKind, PreparedPrincipalProducerIncarnationTransition,
    PrincipalProducerIncarnationCandidateError, PrincipalProducerIncarnationCommitDisposition,
    PrincipalProducerIncarnationCustodyPort, PrincipalProducerIncarnationDatabaseObservation,
    PrincipalProducerIncarnationExpectedFrontier, PrincipalProducerIncarnationHead,
    PrincipalProducerIncarnationHeadPageBoundary, PrincipalProducerIncarnationHeadPageError,
    PrincipalProducerIncarnationHeadPageRead, PrincipalProducerIncarnationReadPort,
    PrincipalProducerIncarnationRecordSet, PrincipalProducerIncarnationSuccessorFrontier,
    PrincipalProducerIncarnationTransitionKind, PrincipalTrustCheckpointHead,
    PrincipalTrustSnapshot, PrincipalTrustSnapshotError, PrincipalTrustStoreBinding,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::BTreeSet;

use super::record_projection::{self, ControlRecordProjectionError};
use super::session_admission::CurrentPrincipalTrustHead;
use super::trust_transition::{AdmittedPrincipalTrustCheckpointTransition, TrustHeadChange};

mod anchors;
pub use anchors::{
    CompletedPrincipalProducerIncarnationCustody, PrincipalProducerIncarnationAnchorFailure,
    PrincipalProducerIncarnationAnchorFailureReason,
};

/// One admitted producer-allocation record paired with its immutable reference.
#[derive(Clone, Copy)]
pub struct AdmittedProducerIncarnation<'a> {
    record_ref: &'a ResidentRecordRefV1,
    record: &'a ProducerIncarnationV1,
}

impl<'a> AdmittedProducerIncarnation<'a> {
    /// Bind an immutable reference to one contextually admitted allocation.
    #[must_use]
    pub const fn new(
        record_ref: &'a ResidentRecordRefV1,
        record: &'a ProducerIncarnationV1,
    ) -> Self {
        Self { record_ref, record }
    }
}

/// One admitted producer-retirement record paired with its immutable reference.
#[derive(Clone, Copy)]
pub struct AdmittedProducerIncarnationRetirement<'a> {
    record_ref: &'a ResidentRecordRefV1,
    record: &'a ProducerIncarnationRetirementV1,
}

impl<'a> AdmittedProducerIncarnationRetirement<'a> {
    /// Bind an immutable reference to one contextually admitted retirement.
    #[must_use]
    pub const fn new(
        record_ref: &'a ResidentRecordRefV1,
        record: &'a ProducerIncarnationRetirementV1,
    ) -> Self {
        Self { record_ref, record }
    }
}

/// Exact admitted hard-domain checkpoint pair plus its owner construction verifier.
///
/// This value is not proof by construction. Preparation invokes the bound
/// verifier with the exact producer mutation and both canonical producer-head
/// roots; only that successful call turns the pair into application evidence.
#[derive(Clone, Copy)]
pub struct AdmittedProducerControlCheckpointTransition<'a> {
    predecessor_ref: &'a ResidentRecordRefV1,
    predecessor: &'a ControlStateCheckpointV1,
    successor_ref: &'a ResidentRecordRefV1,
    successor: &'a ControlStateCheckpointV1,
    checkpoint_verifier: &'a dyn PrincipalProducerCheckpointConstructionVerifier,
}

impl<'a> AdmittedProducerControlCheckpointTransition<'a> {
    /// Bind the admitted pair to the owner that constructed its accumulator.
    #[must_use]
    pub const fn new(
        predecessor_ref: &'a ResidentRecordRefV1,
        predecessor: &'a ControlStateCheckpointV1,
        successor_ref: &'a ResidentRecordRefV1,
        successor: &'a ControlStateCheckpointV1,
        checkpoint_verifier: &'a dyn PrincipalProducerCheckpointConstructionVerifier,
    ) -> Self {
        Self {
            predecessor_ref,
            predecessor,
            successor_ref,
            successor,
            checkpoint_verifier,
        }
    }
}

/// Typed view presented to the owner of control-checkpoint construction.
///
/// The view binds one exact producer mutation to the admitted principal-trust
/// and hard-domain checkpoint successors selected for the same transaction.
/// It carries the canonical producer-head roots before and after the mutation.
/// The verifier must establish that the supplied control checkpoint's
/// accumulator, changed-leaf set, and transaction coordinate represent this
/// exact mutation. Pale deliberately does not reconstruct that owner-owned
/// accumulator algorithm here.
pub struct PrincipalProducerCheckpointConstruction<'view, 'records> {
    mutation: &'view ProducerTransition<'records>,
    trust: &'view AdmittedPrincipalTrustCheckpointTransition<'records>,
    control: &'view AdmittedProducerControlCheckpointTransition<'records>,
    predecessor_root: &'view ProducerIncarnationHeadsRootDigest,
    successor_root: &'view ProducerIncarnationHeadsRootDigest,
}

impl PrincipalProducerCheckpointConstruction<'_, '_> {
    /// Return the exact producer lifecycle transition being checkpointed.
    #[must_use]
    pub const fn transition_kind(&self) -> PrincipalProducerIncarnationTransitionKind {
        self.mutation.kind()
    }

    /// Borrow the allocation appended by an initial or successor transition.
    #[must_use]
    pub const fn appended_allocation(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &ProducerIncarnationV1)> {
        self.mutation.appended_allocation()
    }

    /// Borrow the retirement appended by a retirement transition.
    #[must_use]
    pub const fn appended_retirement(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &ProducerIncarnationRetirementV1)> {
        self.mutation.appended_retirement()
    }

    /// Borrow the exact predecessor allocation when one must already exist.
    #[must_use]
    pub const fn predecessor_allocation(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &ProducerIncarnationV1)> {
        self.mutation.predecessor_allocation()
    }

    /// Borrow the predecessor retirement required by successor allocation.
    #[must_use]
    pub const fn predecessor_retirement(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &ProducerIncarnationRetirementV1)> {
        self.mutation.predecessor_retirement()
    }

    /// Borrow the producer-head root authenticated by the predecessor trust checkpoint.
    #[must_use]
    pub const fn predecessor_producer_heads_root(&self) -> &ProducerIncarnationHeadsRootDigest {
        self.predecessor_root
    }

    /// Borrow the producer-head root authenticated by the successor trust checkpoint.
    #[must_use]
    pub const fn successor_producer_heads_root(&self) -> &ProducerIncarnationHeadsRootDigest {
        self.successor_root
    }

    /// Borrow the exact admitted principal-trust checkpoint pair.
    #[must_use]
    pub const fn trust_checkpoints(
        &self,
    ) -> (
        &ResidentRecordRefV1,
        &PrincipalTrustCheckpointV1,
        &ResidentRecordRefV1,
        &PrincipalTrustCheckpointV1,
    ) {
        (
            self.trust.predecessor_ref,
            self.trust.predecessor,
            self.trust.successor_ref,
            self.trust.successor,
        )
    }

    /// Borrow the exact admitted control-checkpoint pair.
    #[must_use]
    pub const fn control_checkpoints(
        &self,
    ) -> (
        &ResidentRecordRefV1,
        &ControlStateCheckpointV1,
        &ResidentRecordRefV1,
        &ControlStateCheckpointV1,
    ) {
        (
            self.control.predecessor_ref,
            self.control.predecessor,
            self.control.successor_ref,
            self.control.successor,
        )
    }
}

/// Owner boundary that proves a control checkpoint was constructed for one exact mutation.
///
/// Implementations own accumulator construction and changed-leaf interpretation.
/// Returning success proves only that the admitted successor represents the
/// supplied [`PrincipalProducerCheckpointConstruction`]; it does not advance an
/// external anchor or make the producer safe for owner-outbox publication.
pub trait PrincipalProducerCheckpointConstructionVerifier {
    /// Verify the exact mutation-to-checkpoint construction relation.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalProducerCheckpointConstructionRejection`] when the
    /// checkpoint does not represent the supplied mutation exactly.
    fn verify(
        &self,
        construction: &PrincipalProducerCheckpointConstruction<'_, '_>,
    ) -> Result<(), PrincipalProducerCheckpointConstructionRejection>;
}

impl<F> PrincipalProducerCheckpointConstructionVerifier for F
where
    F: Fn(
        &PrincipalProducerCheckpointConstruction<'_, '_>,
    ) -> Result<(), PrincipalProducerCheckpointConstructionRejection>,
{
    fn verify(
        &self,
        construction: &PrincipalProducerCheckpointConstruction<'_, '_>,
    ) -> Result<(), PrincipalProducerCheckpointConstructionRejection> {
        self(construction)
    }
}

/// Owner checkpoint construction rejected the exact producer mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("control checkpoint does not represent the exact producer-incarnation mutation")]
pub struct PrincipalProducerCheckpointConstructionRejection;

/// Restart-recovered semantic producer-head frontier.
///
/// This value exists only after bounded physical enumeration, contextual
/// re-admission of every allocation and optional retirement, and equality
/// between the resulting canonical root and one signed principal-trust
/// checkpoint. It does not grant authority to mutate any producer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredPrincipalProducerIncarnationFrontier {
    through: PrincipalTrustCheckpointHead,
    heads: Box<[ProducerIncarnationHeadV1]>,
}

/// Contextual owner boundary for restart-time producer record admission.
///
/// Recovery owns physical lineage traversal, strict canonical decoding, epoch
/// uniqueness within the recovered principal, and equality between admitted
/// records and stored bytes. An implementation supplies only the authority,
/// selected-schema, namespace, and signature witnesses required by the core
/// record families. Recovery independently verifies the returned record against
/// both the signed bytes and the physical predecessor coordinate, so an
/// implementation cannot fabricate a head or reorder history.
pub trait PrincipalProducerIncarnationRecoveryAdmission {
    /// Admit one decoded allocation against the exact recovered predecessor state.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when schema, authority, namespace,
    /// signature, predecessor, chronology, or owner-native epoch admission fails.
    fn admit_allocation(
        &mut self,
        wire: ProducerIncarnationWireV1,
        head_state: &ProducerIncarnationHeadAdmissionStateV1<'_>,
    ) -> Result<ProducerIncarnationV1, InvalidResidentRecordV1>;

    /// Admit one decoded retirement against its exact recovered active allocation.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when schema, authority, namespace,
    /// signature, allocation identity, or chronology admission fails.
    fn admit_retirement(
        &mut self,
        wire: ProducerIncarnationRetirementWireV1,
        head_state: &ProducerIncarnationHeadAdmissionStateV1<'_>,
    ) -> Result<ProducerIncarnationRetirementV1, InvalidResidentRecordV1>;
}

impl RecoveredPrincipalProducerIncarnationFrontier {
    #[cfg(feature = "interface-test-support")]
    pub(in crate::resident) fn empty_for_interface_conformance(
        through: PrincipalTrustCheckpointHead,
    ) -> Self {
        Self {
            through,
            heads: Box::new([]),
        }
    }

    /// Borrow the exact signed trust-checkpoint head fencing this frontier.
    #[must_use]
    pub const fn through(&self) -> &PrincipalTrustCheckpointHead {
        &self.through
    }

    /// Borrow every current producer head in canonical key order.
    #[must_use]
    pub const fn heads(&self) -> &[ProducerIncarnationHeadV1] {
        &self.heads
    }
}

/// Recover the complete producer-head frontier under one admitted checkpoint.
///
/// `admission` supplies owner-resolved schema, authority, namespace, and
/// signature witnesses. This function owns the rest of the recovery law: it
/// walks every physical predecessor under one snapshot, strictly decodes the
/// canonical bytes, admits history from oldest to newest, rejects repeated
/// principal-wide epochs, and compares every admitted record with its stored
/// bytes before accepting the signed head root.
///
/// In the rendered law, `P` is the recovered producer set, `A[p,i]` is producer
/// `p`'s allocation at lineage index `i`, `R[p,i]` is its required retirement
/// before a successor, `N(p)` is its final lineage index, `E` is the set of all
/// recovered epochs, `H` is the canonical current-head set, and `headRoot(C)`
/// is the producer-head root in the admitted principal-trust checkpoint `C`.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/principal-producer-frontier-recovery.display.mathml"
))]
///
/// # Errors
///
/// Returns [`PrincipalProducerIncarnationProjectionError`] for an invalid
/// checkpoint/store binding, unstable or malformed physical page, failed
/// contextual record admission, physical/semantic substitution, collection
/// overflow, or canonical-root mismatch.
pub fn recover_principal_producer_incarnation_frontier(
    store: &mut dyn PrincipalProducerIncarnationReadPort,
    binding: PrincipalTrustStoreBinding,
    current_trust: CurrentPrincipalTrustHead<'_>,
    maximum_page_items: usize,
    admission: &mut dyn PrincipalProducerIncarnationRecoveryAdmission,
) -> Result<
    RecoveredPrincipalProducerIncarnationFrontier,
    PrincipalProducerIncarnationProjectionError,
> {
    current_trust.checkpoint.verify()?;
    if current_trust.checkpoint.principal_id() != binding.principal_id()
        || current_trust.checkpoint.trust_store_id() != binding.trust_store_id()
        || current_trust.checkpoint.trust_store_incarnation_id()
            != binding.trust_store_incarnation_id()
    {
        return Err(PrincipalProducerIncarnationProjectionError::CurrentHeadSet);
    }
    let (through, checkpoint_record) =
        trust_checkpoint_projection(current_trust.checkpoint_ref, current_trust.checkpoint)?;
    let snapshot = PrincipalTrustSnapshot::new(binding, through.clone(), checkpoint_record)?;
    let mut after = None;
    let mut heads = Vec::new();
    let mut admitted_epochs = BTreeSet::new();
    loop {
        let request = PrincipalProducerIncarnationHeadPageRead::new(
            snapshot.clone(),
            after,
            maximum_page_items,
        )?;
        let page = store.read_heads_page(&request)?;
        if page.request() != &request {
            return Err(PrincipalProducerIncarnationProjectionError::CurrentHeadSet);
        }
        for entry in page.entries() {
            if heads.len() == MAX_RESIDENT_SET_MEMBERS {
                return Err(InvalidResidentRecordV1::BoundExceeded {
                    relation: "producer incarnation heads",
                    observed: heads.len() + 1,
                    maximum: MAX_RESIDENT_SET_MEMBERS,
                }
                .into());
            }
            let lineage = read_producer_lineage(store, &snapshot, entry.records())?;
            let semantic = admit_producer_lineage(&lineage, &mut admitted_epochs, admission)?;
            if !semantic_head_matches_physical(&semantic, entry.records().head()) {
                return Err(PrincipalProducerIncarnationProjectionError::ProducerHead);
            }
            heads.push(semantic);
        }
        match page.boundary() {
            PrincipalProducerIncarnationHeadPageBoundary::Complete => break,
            PrincipalProducerIncarnationHeadPageBoundary::More { resume_after } => {
                after = Some(resume_after.clone());
            }
        }
    }
    let root = producer_incarnation_heads_root_sha256(&heads)?;
    admit_current_head_set(&root, current_trust.checkpoint)?;
    Ok(RecoveredPrincipalProducerIncarnationFrontier {
        through,
        heads: heads.into_boxed_slice(),
    })
}

/// Recover one producer's physical history without crossing the fixed snapshot.
fn read_producer_lineage(
    store: &mut dyn PrincipalProducerIncarnationReadPort,
    snapshot: &PrincipalTrustSnapshot,
    current: &PrincipalProducerIncarnationRecordSet,
) -> Result<Vec<PrincipalProducerIncarnationRecordSet>, PrincipalProducerIncarnationProjectionError>
{
    let mut lineage = vec![current.clone()];
    let mut visited = BTreeSet::from([current.allocation().coordinate().clone()]);
    let mut predecessor = current.previous_allocation().cloned();
    while let Some(coordinate) = predecessor {
        if lineage.len() == MAX_RESIDENT_SET_MEMBERS {
            return Err(PrincipalProducerIncarnationProjectionError::LineageBoundExceeded);
        }
        if !visited.insert(coordinate.clone()) {
            return Err(PrincipalProducerIncarnationProjectionError::LineageCycle);
        }
        let records = match store.read_by_allocation(snapshot, &coordinate)? {
            pale_ports::PrincipalProducerIncarnationReadObservation::Exact(records) => *records,
            pale_ports::PrincipalProducerIncarnationReadObservation::Absent => {
                return Err(PrincipalProducerIncarnationProjectionError::MissingPredecessor);
            }
            pale_ports::PrincipalProducerIncarnationReadObservation::ConflictingOrPartial => {
                return Err(PrincipalProducerIncarnationProjectionError::ProducerHead);
            }
        };
        if records.allocation().coordinate() != &coordinate
            || records.head().origin_principal_id() != snapshot.binding().principal_id()
            || records.head().producer_id() != current.head().producer_id()
        {
            return Err(PrincipalProducerIncarnationProjectionError::ProducerHead);
        }
        predecessor = records.previous_allocation().cloned();
        lineage.push(records);
    }
    lineage.reverse();
    Ok(lineage)
}

/// Admit one physical producer lineage from its initial allocation to its head.
fn admit_producer_lineage(
    lineage: &[PrincipalProducerIncarnationRecordSet],
    admitted_epochs: &mut BTreeSet<ProducerEpochV1>,
    admission: &mut dyn PrincipalProducerIncarnationRecoveryAdmission,
) -> Result<ProducerIncarnationHeadV1, PrincipalProducerIncarnationProjectionError> {
    struct AdmittedIncarnation {
        allocation_ref: ResidentRecordRefV1,
        allocation: ProducerIncarnationV1,
        retirement: Option<ProducerIncarnationRetirementV1>,
    }

    let mut admitted = Vec::<AdmittedIncarnation>::with_capacity(lineage.len());
    for (index, physical) in lineage.iter().enumerate() {
        let allocation_wire = decode_canonical_record::<ProducerIncarnationWireV1>(
            physical.allocation(),
            "producer incarnation",
        )?;
        let allocation = if index == 0 {
            admission.admit_allocation(
                allocation_wire,
                &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
            )?
        } else {
            let predecessor = admitted
                .last()
                .ok_or(PrincipalProducerIncarnationProjectionError::MissingPredecessor)?;
            let retirement = predecessor
                .retirement
                .as_ref()
                .ok_or(PrincipalProducerIncarnationProjectionError::MissingPredecessor)?;
            let head_state = ProducerIncarnationHeadAdmissionStateV1::retired(
                &predecessor.allocation_ref,
                &predecessor.allocation,
                retirement,
            )?;
            admission.admit_allocation(allocation_wire, &head_state)?
        };
        if !physical_predecessor_matches_admitted(physical, &allocation) {
            return Err(PrincipalProducerIncarnationProjectionError::PhysicalPredecessorMismatch);
        }
        if !admitted_epochs.insert(allocation.producer_epoch().clone()) {
            return Err(PrincipalProducerIncarnationProjectionError::DuplicateProducerEpoch);
        }
        require_exact_stored_record(physical.allocation(), &allocation)?;

        let allocation_ref = physical.allocation().coordinate().record_ref().clone();
        let retirement = physical
            .retirement()
            .map(
                |record| -> Result<_, PrincipalProducerIncarnationProjectionError> {
                    let wire = decode_canonical_record::<ProducerIncarnationRetirementWireV1>(
                        record,
                        "producer incarnation retirement",
                    )?;
                    let head_state = ProducerIncarnationHeadAdmissionStateV1::active(
                        &allocation_ref,
                        &allocation,
                    )?;
                    let retirement = admission.admit_retirement(wire, &head_state)?;
                    require_exact_stored_record(record, &retirement)?;
                    Ok(retirement)
                },
            )
            .transpose()?;
        let semantic = match retirement.as_ref() {
            Some(retirement) => {
                ProducerIncarnationHeadV1::retired(&allocation_ref, &allocation, retirement)?
            }
            None => ProducerIncarnationHeadV1::active(&allocation_ref, &allocation)?,
        };
        if !semantic_head_matches_physical(&semantic, physical.head()) {
            return Err(PrincipalProducerIncarnationProjectionError::ProducerHead);
        }
        admitted.push(AdmittedIncarnation {
            allocation_ref,
            allocation,
            retirement,
        });
    }
    let head = admitted
        .last()
        .ok_or(PrincipalProducerIncarnationProjectionError::MissingPredecessor)?;
    match head.retirement.as_ref() {
        Some(retirement) => Ok(ProducerIncarnationHeadV1::retired(
            &head.allocation_ref,
            &head.allocation,
            retirement,
        )?),
        None => Ok(ProducerIncarnationHeadV1::active(
            &head.allocation_ref,
            &head.allocation,
        )?),
    }
}

/// Compare separately indexed physical lineage with the admitted signed record.
fn physical_predecessor_matches_admitted(
    physical: &PrincipalProducerIncarnationRecordSet,
    allocation: &ProducerIncarnationV1,
) -> bool {
    match (
        physical.previous_allocation(),
        allocation.previous_incarnation_ref().as_ref(),
        allocation.previous_incarnation_sha256().as_ref(),
    ) {
        (None, None, None) => true,
        (Some(physical), Some(record_ref), Some(record_sha256)) => {
            physical.record_ref() == record_ref
                && physical.record_sha256().as_str() == record_sha256.as_str()
        }
        _ => false,
    }
}

/// Strictly decode one stored canonical record without normalizing its bytes.
fn decode_canonical_record<W>(
    record: &ControlRecord,
    family: &'static str,
) -> Result<W, PrincipalProducerIncarnationProjectionError>
where
    W: DeserializeOwned + Serialize,
{
    let bytes = record.canonical_record_jcs().as_slice();
    let value = canon::parse_strict(bytes)
        .map_err(PrincipalProducerIncarnationProjectionError::StoredRecordSyntax)?;
    if canon::to_canonical_string(&value)?.as_bytes() != bytes {
        return Err(
            PrincipalProducerIncarnationProjectionError::NonCanonicalStoredRecord { family },
        );
    }
    serde_json::from_value(value).map_err(|source| {
        PrincipalProducerIncarnationProjectionError::StoredRecordDecode { family, source }
    })
}

/// Require an admitted semantic record to reproduce its physical row exactly.
fn require_exact_stored_record<R>(
    physical: &ControlRecord,
    semantic: &R,
) -> Result<(), PrincipalProducerIncarnationProjectionError>
where
    R: Serialize + record_projection::ResidentRecordSchema,
{
    let projected = record_projection::record(physical.coordinate().clone(), semantic)?;
    if projected == *physical {
        Ok(())
    } else {
        Err(PrincipalProducerIncarnationProjectionError::ProducerHead)
    }
}

/// Complete recovered semantic frontier required to project one mutation.
///
/// The recovered head set must be fenced by the exact predecessor checkpoint
/// in `trust`; a frontier recovered under another checkpoint cannot be reused.
#[derive(Clone, Copy)]
pub struct PrincipalProducerIncarnationProjectionContext<'a> {
    frontier: &'a RecoveredPrincipalProducerIncarnationFrontier,
    trust: AdmittedPrincipalTrustCheckpointTransition<'a>,
    control: AdmittedProducerControlCheckpointTransition<'a>,
}

impl<'a> PrincipalProducerIncarnationProjectionContext<'a> {
    /// Bind one recovered head frontier to both admitted checkpoint advances.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalProducerIncarnationProjectionError::CurrentHeadSet`]
    /// unless `frontier` was recovered under the exact trust predecessor.
    pub fn new(
        frontier: &'a RecoveredPrincipalProducerIncarnationFrontier,
        trust: AdmittedPrincipalTrustCheckpointTransition<'a>,
        control: AdmittedProducerControlCheckpointTransition<'a>,
    ) -> Result<Self, PrincipalProducerIncarnationProjectionError> {
        let expected = trust_checkpoint_projection(trust.predecessor_ref, trust.predecessor)?.0;
        if frontier.through != expected {
            return Err(PrincipalProducerIncarnationProjectionError::CurrentHeadSet);
        }
        Ok(Self {
            frontier,
            trust,
            control,
        })
    }
}

/// Named physical-custody phase of a producer-incarnation transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalProducerIncarnationCustodyPhase {
    /// Semantic closure is proven but the database has not accepted the candidate.
    Prepared,
    /// Exact database custody is proven; required external anchors remain pending.
    DatabaseCommittedAnchorsPending,
    /// The exact database transition and both external successors were established.
    Completed,
}

/// Evidence establishing the database portion of producer custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalProducerIncarnationCommitEvidence {
    /// The adapter committed the candidate from the exact predecessor frontier.
    New,
    /// The adapter proved that the complete candidate was already present.
    ExactReplay,
    /// Mandatory read-back after an ambiguous commit found the exact successor.
    RecoveredExactAfterAmbiguousCommit,
}

/// Prepared producer transition with no physical-custody proof yet.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "prepared producer custody must be committed or retained for recovery"]
pub struct PreparedPrincipalProducerIncarnationCustody {
    candidate: PreparedPrincipalProducerIncarnationTransition,
    control_store: pale_ports::AuthorityStoreCoordinate,
}

impl PreparedPrincipalProducerIncarnationCustody {
    /// Return the only phase represented by this type.
    #[must_use]
    pub const fn phase(&self) -> PrincipalProducerIncarnationCustodyPhase {
        PrincipalProducerIncarnationCustodyPhase::Prepared
    }

    /// Borrow the closed physical candidate without granting commit authority.
    #[must_use]
    pub const fn candidate(&self) -> &PreparedPrincipalProducerIncarnationTransition {
        &self.candidate
    }

    /// Consume the prepared state and attempt the atomic database transition.
    ///
    /// A [`ControlStoreErrorKind::OutcomeUnknown`] result is never
    /// retried. The method immediately performs exact read-back through
    /// [`PrincipalProducerIncarnationCustodyPort::observe_exact`]. Only the
    /// complete successor advances to database-proven, anchor-pending state.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalProducerIncarnationCommitFailure`] with the complete
    /// prepared state when database custody is not proven exactly.
    pub fn commit(
        self,
        store: &mut dyn PrincipalProducerIncarnationCustodyPort,
    ) -> Result<
        DatabaseCommittedPrincipalProducerIncarnationAnchorsPending,
        PrincipalProducerIncarnationCommitFailure,
    > {
        match store.compare_and_commit(&self.candidate) {
            Ok(disposition) => Ok(self.database_committed(match disposition {
                PrincipalProducerIncarnationCommitDisposition::Committed => {
                    PrincipalProducerIncarnationCommitEvidence::New
                }
                PrincipalProducerIncarnationCommitDisposition::ExactReplay => {
                    PrincipalProducerIncarnationCommitEvidence::ExactReplay
                }
            })),
            Err(error) if error.kind() == ControlStoreErrorKind::OutcomeUnknown => {
                match store.observe_exact(&self.candidate) {
                    Ok(PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor) => Ok(self
                        .database_committed(
                            PrincipalProducerIncarnationCommitEvidence::RecoveredExactAfterAmbiguousCommit,
                        )),
                    Ok(PrincipalProducerIncarnationDatabaseObservation::Predecessor) => Err(self
                        .failed(
                            PrincipalProducerIncarnationCommitFailureReason::CommitExcludedByExactReadBack {
                                uncertain: error,
                            },
                        )),
                    Ok(PrincipalProducerIncarnationDatabaseObservation::ConflictingOrPartial) => {
                        Err(self.failed(
                            PrincipalProducerIncarnationCommitFailureReason::RecoveryRequiredAfterExactReadBack {
                                uncertain: error,
                            },
                        ))
                    }
                    Err(read_back) => Err(self.failed(
                        PrincipalProducerIncarnationCommitFailureReason::ExactReadBackFailed {
                            uncertain: error,
                            read_back,
                        },
                    )),
                }
            }
            Err(error) => Err(
                self.failed(PrincipalProducerIncarnationCommitFailureReason::Store(
                    error,
                )),
            ),
        }
    }

    fn from_candidate(
        candidate: PreparedPrincipalProducerIncarnationTransition,
        control_store: pale_ports::AuthorityStoreCoordinate,
    ) -> Self {
        Self {
            candidate,
            control_store,
        }
    }

    fn database_committed(
        self,
        evidence: PrincipalProducerIncarnationCommitEvidence,
    ) -> DatabaseCommittedPrincipalProducerIncarnationAnchorsPending {
        DatabaseCommittedPrincipalProducerIncarnationAnchorsPending {
            candidate: self.candidate,
            control_store: self.control_store,
            evidence,
            trust_anchor: anchors::AnchorProgress::after_database(evidence),
            control_anchor: anchors::AnchorProgress::after_database(evidence),
        }
    }

    fn failed(
        self,
        reason: PrincipalProducerIncarnationCommitFailureReason,
    ) -> PrincipalProducerIncarnationCommitFailure {
        PrincipalProducerIncarnationCommitFailure {
            prepared: Box::new(self),
            reason,
        }
    }
}

/// Exact database custody with the trust minimum and hard-domain anchor pending.
///
/// This state is not owner-outbox admission, runtime readiness, or proof of
/// rollback-safe producer-epoch non-reuse. [`Self::advance_anchors`] completes
/// only the retained candidate's two anchor advances. Later use must check
/// current authority, heads and fences independently.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "database custody remains incomplete until external anchors are proven"]
pub struct DatabaseCommittedPrincipalProducerIncarnationAnchorsPending {
    candidate: PreparedPrincipalProducerIncarnationTransition,
    control_store: pale_ports::AuthorityStoreCoordinate,
    evidence: PrincipalProducerIncarnationCommitEvidence,
    trust_anchor: anchors::AnchorProgress,
    control_anchor: anchors::AnchorProgress,
}

impl DatabaseCommittedPrincipalProducerIncarnationAnchorsPending {
    /// Return the only phase represented by this type.
    #[must_use]
    pub const fn phase(&self) -> PrincipalProducerIncarnationCustodyPhase {
        PrincipalProducerIncarnationCustodyPhase::DatabaseCommittedAnchorsPending
    }

    /// Return how exact database custody was established.
    #[must_use]
    pub const fn evidence(&self) -> PrincipalProducerIncarnationCommitEvidence {
        self.evidence
    }

    /// Borrow the exact committed candidate for later anchor binding.
    #[must_use]
    pub const fn candidate(&self) -> &PreparedPrincipalProducerIncarnationTransition {
        &self.candidate
    }
}

/// Failed database transition with the complete prepared state retained.
#[derive(Debug, thiserror::Error)]
#[error("producer-incarnation custody did not advance: {reason}")]
pub struct PrincipalProducerIncarnationCommitFailure {
    prepared: Box<PreparedPrincipalProducerIncarnationCustody>,
    reason: PrincipalProducerIncarnationCommitFailureReason,
}

impl PrincipalProducerIncarnationCommitFailure {
    /// Borrow the exact reason database custody remained unproven.
    #[must_use]
    pub const fn reason(&self) -> &PrincipalProducerIncarnationCommitFailureReason {
        &self.reason
    }

    /// Recover the complete prepared state for explicit diagnosis or recovery.
    pub fn into_prepared(self) -> PreparedPrincipalProducerIncarnationCustody {
        *self.prepared
    }
}

/// Reason a prepared producer candidate did not gain database proof.
#[derive(Debug, thiserror::Error)]
pub enum PrincipalProducerIncarnationCommitFailureReason {
    /// The adapter rejected the candidate before an ambiguous commit boundary.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// Exact read-back proved that the predecessor remains current.
    #[error("exact read-back excluded producer-incarnation commit")]
    CommitExcludedByExactReadBack {
        /// Native uncertainty returned by the attempted database commit.
        #[source]
        uncertain: ControlStoreError,
    },
    /// Exact read-back found a partial or contradictory frontier.
    #[error("exact read-back requires explicit producer-incarnation recovery")]
    RecoveryRequiredAfterExactReadBack {
        /// Native uncertainty returned by the attempted database commit.
        #[source]
        uncertain: ControlStoreError,
    },
    /// Exact read-back itself could not complete.
    #[error("exact producer-incarnation read-back failed")]
    ExactReadBackFailed {
        /// Native uncertainty returned by the attempted database commit.
        uncertain: ControlStoreError,
        /// Native failure returned by mandatory exact read-back.
        #[source]
        read_back: ControlStoreError,
    },
}

/// Failure to close admitted producer records over exact checkpoint frontiers.
#[derive(Debug, thiserror::Error)]
pub enum PrincipalProducerIncarnationProjectionError {
    /// An admitted record no longer reproduces its canonical unsigned digest.
    #[error(transparent)]
    InvalidRecord(#[from] InvalidResidentRecordV1),
    /// Canonical signed-record encoding failed.
    #[error("canonicalize admitted producer-incarnation custody record")]
    Canonicalize(#[from] canon::CanonError),
    /// Stored bytes are not valid strict portable JSON.
    #[error("stored producer-incarnation record violates strict JSON syntax")]
    StoredRecordSyntax(#[source] canon::CanonError),
    /// Stored strict JSON is not the exact canonical JCS encoding.
    #[error("stored {family} record is not canonical JCS")]
    NonCanonicalStoredRecord {
        /// Stable record-family label used by diagnostics.
        family: &'static str,
    },
    /// Stored canonical bytes do not decode as the selected wire family.
    #[error("stored {family} record does not decode as its selected wire family")]
    StoredRecordDecode {
        /// Stable record-family label used by diagnostics.
        family: &'static str,
        /// Typed wire-decoding failure.
        #[source]
        source: serde_json::Error,
    },
    /// A shared physical record coordinate or byte string is malformed.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
    /// A bounded physical discovery page contradicts its fixed request.
    #[error(transparent)]
    Page(#[from] PrincipalProducerIncarnationHeadPageError),
    /// The selected physical trust head contradicts its record or binding.
    #[error(transparent)]
    Snapshot(#[from] PrincipalTrustSnapshotError),
    /// The physical trust store could not prove a stable bounded observation.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// The supplied complete producer-head set is not the predecessor frontier.
    #[error("producer-incarnation head set does not match the predecessor trust checkpoint")]
    CurrentHeadSet,
    /// The selected allocation, retirement, or successor does not match its head.
    #[error("producer-incarnation record does not match the selected current head")]
    ProducerHead,
    /// A successor names an allocation absent from the fixed physical snapshot.
    #[error("producer-incarnation predecessor is absent from the fixed recovery snapshot")]
    MissingPredecessor,
    /// Physical traversal metadata differs from the admitted signed predecessor.
    #[error("physical producer predecessor does not match the admitted allocation record")]
    PhysicalPredecessorMismatch,
    /// Physical predecessor coordinates contain a cycle.
    #[error("producer-incarnation predecessor lineage contains a cycle")]
    LineageCycle,
    /// One physical predecessor lineage exceeds the shared resident set bound.
    #[error("producer-incarnation predecessor lineage exceeds the resident set bound")]
    LineageBoundExceeded,
    /// One principal-wide producer epoch appears in more than one recovered allocation.
    #[error("producer epoch is repeated within the recovered principal frontier")]
    DuplicateProducerEpoch,
    /// The trust checkpoint changes more than the selected producer-head root.
    #[error("principal trust checkpoint does not encode only this producer transition")]
    TrustCheckpointClosure,
    /// The hard-domain checkpoint does not close over the selected trust transition.
    #[error("producer hard-domain checkpoint does not close over this transition")]
    ControlCheckpointClosure,
    /// The checkpoint-construction owner rejected the exact producer mutation.
    #[error(transparent)]
    CheckpointConstruction(#[from] PrincipalProducerCheckpointConstructionRejection),
    /// The projected physical candidate contradicts its finite head algebra.
    #[error(transparent)]
    Candidate(#[from] PrincipalProducerIncarnationCandidateError),
}

impl From<ControlRecordProjectionError> for PrincipalProducerIncarnationProjectionError {
    fn from(error: ControlRecordProjectionError) -> Self {
        match error {
            ControlRecordProjectionError::Canonicalize(source) => Self::Canonicalize(source),
            ControlRecordProjectionError::Record(source) => Self::Record(source),
        }
    }
}

/// Prepare the first active allocation for one principal and producer key.
///
/// # Errors
///
/// Returns [`PrincipalProducerIncarnationProjectionError`] unless the key is
/// absent, both checkpoint transitions close exactly, and the allocation
/// produces the successor checkpoint's canonical producer-head root.
pub fn prepare_initial_principal_producer_incarnation(
    allocation: AdmittedProducerIncarnation<'_>,
    context: PrincipalProducerIncarnationProjectionContext<'_>,
) -> Result<PreparedPrincipalProducerIncarnationCustody, PrincipalProducerIncarnationProjectionError>
{
    prepare_transition(ProducerTransition::Initial(allocation), context)
}

/// Prepare append-only retirement of the exact active allocation head.
///
/// # Errors
///
/// Returns [`PrincipalProducerIncarnationProjectionError`] unless the
/// allocation is the current active head and the retirement and checkpoint
/// records close the exact finite transition.
pub fn prepare_principal_producer_incarnation_retirement(
    allocation: AdmittedProducerIncarnation<'_>,
    retirement: AdmittedProducerIncarnationRetirement<'_>,
    context: PrincipalProducerIncarnationProjectionContext<'_>,
) -> Result<PreparedPrincipalProducerIncarnationCustody, PrincipalProducerIncarnationProjectionError>
{
    prepare_transition(
        ProducerTransition::Retirement {
            allocation,
            retirement,
        },
        context,
    )
}

/// Prepare a fresh allocation after the exact current head was retired.
///
/// # Errors
///
/// Returns [`PrincipalProducerIncarnationProjectionError`] unless the supplied
/// predecessor allocation and retirement are the current retired head and the
/// successor allocation closes the exact producer and checkpoint transition.
pub fn prepare_principal_producer_incarnation_successor(
    predecessor: AdmittedProducerIncarnation<'_>,
    retirement: AdmittedProducerIncarnationRetirement<'_>,
    successor: AdmittedProducerIncarnation<'_>,
    context: PrincipalProducerIncarnationProjectionContext<'_>,
) -> Result<PreparedPrincipalProducerIncarnationCustody, PrincipalProducerIncarnationProjectionError>
{
    prepare_transition(
        ProducerTransition::Successor {
            predecessor,
            retirement,
            successor,
        },
        context,
    )
}

#[derive(Clone, Copy)]
enum ProducerTransition<'a> {
    Initial(AdmittedProducerIncarnation<'a>),
    Retirement {
        allocation: AdmittedProducerIncarnation<'a>,
        retirement: AdmittedProducerIncarnationRetirement<'a>,
    },
    Successor {
        predecessor: AdmittedProducerIncarnation<'a>,
        retirement: AdmittedProducerIncarnationRetirement<'a>,
        successor: AdmittedProducerIncarnation<'a>,
    },
}

fn prepare_transition(
    transition: ProducerTransition<'_>,
    context: PrincipalProducerIncarnationProjectionContext<'_>,
) -> Result<PreparedPrincipalProducerIncarnationCustody, PrincipalProducerIncarnationProjectionError>
{
    transition.verify()?;
    let predecessor_root = producer_incarnation_heads_root_sha256(context.frontier.heads())?;
    admit_current_head_set(&predecessor_root, context.trust.predecessor)?;
    let key = transition.key();
    let expected_semantic = transition.expected_semantic_head()?;
    let next_semantic = transition.next_semantic_head()?;
    let next_heads = apply_semantic_head_update(
        context.frontier.heads(),
        key,
        expected_semantic.as_ref(),
        next_semantic,
    )?;
    let successor_root = producer_incarnation_heads_root_sha256(&next_heads)?;
    admit_trust_transition(&context.trust, &predecessor_root, &successor_root)?;
    admit_control_transition(&context.trust, &context.control)?;
    context
        .control
        .checkpoint_verifier
        .verify(&PrincipalProducerCheckpointConstruction {
            mutation: &transition,
            trust: &context.trust,
            control: &context.control,
            predecessor_root: &predecessor_root,
            successor_root: &successor_root,
        })?;

    let binding = trust_binding(context.trust.predecessor);
    let expected_trust =
        trust_checkpoint_projection(context.trust.predecessor_ref, context.trust.predecessor)?;
    let next_trust =
        trust_checkpoint_projection(context.trust.successor_ref, context.trust.successor)?;
    let expected_control = control_checkpoint_projection(
        context.control.predecessor_ref,
        context.control.predecessor,
    )?;
    let next_control =
        control_checkpoint_projection(context.control.successor_ref, context.control.successor)?;
    let projected = transition.project_physical()?;

    let candidate = PreparedPrincipalProducerIncarnationTransition::new(
        binding,
        projected.kind,
        PrincipalProducerIncarnationExpectedFrontier::new(
            projected.expected,
            expected_trust,
            expected_control,
        ),
        vec![projected.appended, next_trust.1, next_control.1],
        PrincipalProducerIncarnationSuccessorFrontier::new(
            projected.next,
            next_trust.0,
            next_control.0,
        ),
    )?;
    Ok(PreparedPrincipalProducerIncarnationCustody::from_candidate(
        candidate,
        pale_ports::AuthorityStoreCoordinate::new(
            context.control.successor.control_store_id().clone(),
            context.control.successor.control_incarnation_id().clone(),
        ),
    ))
}

/// Apply one exact insertion or replacement without changing any other head.
fn apply_semantic_head_update(
    current: &[ProducerIncarnationHeadV1],
    key: (
        &pale_core::records::ResidentPrincipalIdV1,
        &pale_core::records::ProducerIdV1,
    ),
    expected: Option<&ProducerIncarnationHeadV1>,
    next: ProducerIncarnationHeadV1,
) -> Result<Vec<ProducerIncarnationHeadV1>, PrincipalProducerIncarnationProjectionError> {
    let index = current.binary_search_by(|head| head.key().cmp(&key));
    match (index, expected) {
        (Err(index), None) => {
            let mut updated = current.to_vec();
            updated.insert(index, next);
            Ok(updated)
        }
        (Ok(index), Some(expected)) if current[index] == *expected => {
            let mut updated = current.to_vec();
            updated[index] = next;
            Ok(updated)
        }
        _ => Err(PrincipalProducerIncarnationProjectionError::ProducerHead),
    }
}

/// Compare one re-admitted semantic head with its physical storage coordinate.
fn semantic_head_matches_physical(
    semantic: &ProducerIncarnationHeadV1,
    physical: &PrincipalProducerIncarnationHead,
) -> bool {
    semantic.origin_principal_id() == physical.origin_principal_id()
        && semantic.producer_id() == physical.producer_id()
        && semantic.producer_epoch() == physical.producer_epoch()
        && semantic.state() == physical.state()
        && semantic.incarnation_ref() == physical.allocation().record_ref()
        && semantic.incarnation_sha256().as_str() == physical.allocation().record_sha256().as_str()
}

impl ProducerTransition<'_> {
    const fn kind(&self) -> PrincipalProducerIncarnationTransitionKind {
        match self {
            Self::Initial(_) => PrincipalProducerIncarnationTransitionKind::InitialAllocation,
            Self::Retirement { .. } => PrincipalProducerIncarnationTransitionKind::Retirement,
            Self::Successor { .. } => {
                PrincipalProducerIncarnationTransitionKind::SuccessorAllocation
            }
        }
    }

    const fn appended_allocation(&self) -> Option<(&ResidentRecordRefV1, &ProducerIncarnationV1)> {
        match self {
            Self::Initial(allocation)
            | Self::Successor {
                successor: allocation,
                ..
            } => Some((allocation.record_ref, allocation.record)),
            Self::Retirement { .. } => None,
        }
    }

    const fn appended_retirement(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &ProducerIncarnationRetirementV1)> {
        match self {
            Self::Retirement { retirement, .. } => Some((retirement.record_ref, retirement.record)),
            Self::Initial(_) | Self::Successor { .. } => None,
        }
    }

    const fn predecessor_allocation(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &ProducerIncarnationV1)> {
        match self {
            Self::Initial(_) => None,
            Self::Retirement { allocation, .. } => Some((allocation.record_ref, allocation.record)),
            Self::Successor { predecessor, .. } => {
                Some((predecessor.record_ref, predecessor.record))
            }
        }
    }

    const fn predecessor_retirement(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &ProducerIncarnationRetirementV1)> {
        match self {
            Self::Successor { retirement, .. } => Some((retirement.record_ref, retirement.record)),
            Self::Initial(_) | Self::Retirement { .. } => None,
        }
    }

    fn verify(self) -> Result<(), InvalidResidentRecordV1> {
        match self {
            Self::Initial(allocation) => allocation.record.verify(),
            Self::Retirement {
                allocation,
                retirement,
            } => {
                allocation.record.verify()?;
                retirement.record.verify()
            }
            Self::Successor {
                predecessor,
                retirement,
                successor,
            } => {
                predecessor.record.verify()?;
                retirement.record.verify()?;
                successor.record.verify()
            }
        }
    }

    fn key(
        &self,
    ) -> (
        &pale_core::records::ResidentPrincipalIdV1,
        &pale_core::records::ProducerIdV1,
    ) {
        let allocation = match self {
            Self::Initial(allocation) => allocation,
            Self::Retirement { allocation, .. } => allocation,
            Self::Successor { successor, .. } => successor,
        };
        (
            allocation.record.origin_principal_id(),
            allocation.record.producer_id(),
        )
    }

    fn expected_semantic_head(
        &self,
    ) -> Result<Option<ProducerIncarnationHeadV1>, InvalidResidentRecordV1> {
        match self {
            Self::Initial(_) => Ok(None),
            Self::Retirement { allocation, .. } => Ok(Some(ProducerIncarnationHeadV1::active(
                allocation.record_ref,
                allocation.record,
            )?)),
            Self::Successor {
                predecessor,
                retirement,
                ..
            } => Ok(Some(ProducerIncarnationHeadV1::retired(
                predecessor.record_ref,
                predecessor.record,
                retirement.record,
            )?)),
        }
    }

    fn next_semantic_head(&self) -> Result<ProducerIncarnationHeadV1, InvalidResidentRecordV1> {
        match self {
            Self::Initial(allocation)
            | Self::Successor {
                successor: allocation,
                ..
            } => ProducerIncarnationHeadV1::active(allocation.record_ref, allocation.record),
            Self::Retirement {
                allocation,
                retirement,
            } => ProducerIncarnationHeadV1::retired(
                allocation.record_ref,
                allocation.record,
                retirement.record,
            ),
        }
    }

    fn project_physical(
        &self,
    ) -> Result<ProjectedProducerTransition, PrincipalProducerIncarnationProjectionError> {
        match self {
            Self::Initial(allocation) => Ok(ProjectedProducerTransition {
                kind: PrincipalProducerIncarnationTransitionKind::InitialAllocation,
                expected: None,
                appended: record_projection::record(allocation.coordinate()?, allocation.record)?,
                next: allocation.physical_head(None)?,
            }),
            Self::Retirement {
                allocation,
                retirement,
            } => {
                let allocation_coordinate = allocation.coordinate()?;
                Ok(ProjectedProducerTransition {
                    kind: PrincipalProducerIncarnationTransitionKind::Retirement,
                    expected: Some((
                        allocation.physical_head(None)?,
                        vec![record_projection::record(
                            allocation_coordinate,
                            allocation.record,
                        )?],
                    )),
                    appended: record_projection::record(
                        retirement.coordinate()?,
                        retirement.record,
                    )?,
                    next: allocation.physical_head(Some(*retirement))?,
                })
            }
            Self::Successor {
                predecessor,
                retirement,
                successor,
            } => Ok(ProjectedProducerTransition {
                kind: PrincipalProducerIncarnationTransitionKind::SuccessorAllocation,
                expected: Some((
                    predecessor.physical_head(Some(*retirement))?,
                    vec![
                        record_projection::record(predecessor.coordinate()?, predecessor.record)?,
                        record_projection::record(retirement.coordinate()?, retirement.record)?,
                    ],
                )),
                appended: record_projection::record(successor.coordinate()?, successor.record)?,
                next: successor.physical_head(None)?,
            }),
        }
    }
}

struct ProjectedProducerTransition {
    kind: PrincipalProducerIncarnationTransitionKind,
    expected: Option<(PrincipalProducerIncarnationHead, Vec<ControlRecord>)>,
    appended: ControlRecord,
    next: PrincipalProducerIncarnationHead,
}

impl AdmittedProducerIncarnation<'_> {
    fn coordinate(
        self,
    ) -> Result<pale_ports::ControlRecordCoordinate, ControlRecordProjectionError> {
        record_projection::coordinate(
            self.record_ref,
            self.record.canonical_incarnation_sha256().as_str(),
        )
    }

    fn physical_head(
        self,
        retirement: Option<AdmittedProducerIncarnationRetirement<'_>>,
    ) -> Result<PrincipalProducerIncarnationHead, PrincipalProducerIncarnationProjectionError> {
        PrincipalProducerIncarnationHead::new(
            self.record.origin_principal_id().clone(),
            self.record.producer_id().clone(),
            self.record.producer_epoch().clone(),
            if retirement.is_some() {
                ProducerIncarnationHeadStateV1::Retired
            } else {
                ProducerIncarnationHeadStateV1::Active
            },
            self.coordinate()?,
            retirement
                .map(AdmittedProducerIncarnationRetirement::coordinate)
                .transpose()?,
        )
        .map_err(Into::into)
    }
}

impl AdmittedProducerIncarnationRetirement<'_> {
    fn coordinate(
        self,
    ) -> Result<pale_ports::ControlRecordCoordinate, ControlRecordProjectionError> {
        record_projection::coordinate(
            self.record_ref,
            self.record.canonical_retirement_sha256().as_str(),
        )
    }
}

fn admit_current_head_set(
    root: &ProducerIncarnationHeadsRootDigest,
    checkpoint: &PrincipalTrustCheckpointV1,
) -> Result<(), PrincipalProducerIncarnationProjectionError> {
    if root == checkpoint.producer_incarnation_heads_root_sha256() {
        Ok(())
    } else {
        Err(PrincipalProducerIncarnationProjectionError::CurrentHeadSet)
    }
}

fn admit_trust_transition(
    transition: &AdmittedPrincipalTrustCheckpointTransition<'_>,
    predecessor_root: &ProducerIncarnationHeadsRootDigest,
    successor_root: &ProducerIncarnationHeadsRootDigest,
) -> Result<(), PrincipalProducerIncarnationProjectionError> {
    let predecessor = transition.predecessor;
    let successor = transition.successor;
    let exact_roots = predecessor_root == predecessor.producer_incarnation_heads_root_sha256()
        && successor_root == successor.producer_incarnation_heads_root_sha256();
    let exact_operation = transition.preserves_other_heads(TrustHeadChange::ProducerIncarnations);
    if exact_roots && exact_operation {
        Ok(())
    } else {
        Err(PrincipalProducerIncarnationProjectionError::TrustCheckpointClosure)
    }
}

fn admit_control_transition(
    trust: &AdmittedPrincipalTrustCheckpointTransition<'_>,
    control: &AdmittedProducerControlCheckpointTransition<'_>,
) -> Result<(), PrincipalProducerIncarnationProjectionError> {
    control
        .successor
        .verify_successor_of(control.predecessor_ref, control.predecessor)?;
    let expected = control_checkpoint_head(control.predecessor_ref, control.predecessor)?;
    let next = control_checkpoint_head(control.successor_ref, control.successor)?;
    let exact = next.is_exact_successor_of(&expected)
        && control.predecessor.control_store_id() == trust.predecessor.principal_control_store_id()
        && control.predecessor.control_incarnation_id()
            == trust.predecessor.principal_control_incarnation_id()
        && control.successor.control_store_id() == trust.successor.principal_control_store_id()
        && control.successor.control_incarnation_id()
            == trust.successor.principal_control_incarnation_id()
        && control.predecessor.issuer_principal_id() == trust.predecessor.principal_id()
        && control.successor.issuer_principal_id() == trust.successor.principal_id();
    if exact {
        Ok(())
    } else {
        Err(PrincipalProducerIncarnationProjectionError::ControlCheckpointClosure)
    }
}

fn trust_binding(checkpoint: &PrincipalTrustCheckpointV1) -> PrincipalTrustStoreBinding {
    PrincipalTrustStoreBinding::new(
        checkpoint.trust_store_id().clone(),
        checkpoint.trust_store_incarnation_id().clone(),
        checkpoint.principal_id().clone(),
    )
}

fn trust_checkpoint_projection(
    record_ref: &ResidentRecordRefV1,
    checkpoint: &PrincipalTrustCheckpointV1,
) -> Result<
    (PrincipalTrustCheckpointHead, ControlRecord),
    PrincipalProducerIncarnationProjectionError,
> {
    let coordinate = record_projection::coordinate(
        record_ref,
        checkpoint.canonical_trust_checkpoint_sha256().as_str(),
    )?;
    Ok((
        PrincipalTrustCheckpointHead::new(
            checkpoint.principal_id().clone(),
            *checkpoint.checkpoint_epoch(),
            coordinate.clone(),
        ),
        record_projection::record(coordinate, checkpoint)?,
    ))
}

fn control_checkpoint_head(
    record_ref: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<ControlCheckpointHead, ControlRecordProjectionError> {
    Ok(ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        *checkpoint.domain_epoch(),
        *checkpoint.domain_sequence(),
        *checkpoint.committed_transaction_sequence(),
        record_projection::coordinate(
            record_ref,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )?,
    ))
}

fn control_checkpoint_projection(
    record_ref: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<(ControlCheckpointHead, ControlRecord), PrincipalProducerIncarnationProjectionError> {
    let head = control_checkpoint_head(record_ref, checkpoint)?;
    Ok((
        head.clone(),
        record_projection::record(head.coordinate().clone(), checkpoint)?,
    ))
}

#[cfg(test)]
#[path = "../../tests/internal/resident/producer.rs"]
mod tests;
