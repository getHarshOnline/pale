//! Read one observation from a bounded, admitted operation history.

use crate::resident::{
    checkpoint_profile::operation_observation_membership, checkpoint_recovery,
    IssuedLocalOperation, LocalOperationRecoveryError, OperationCheckpointSelection,
    OperationRecoveryError,
};
use pale_core::{canon, records::*};
use pale_ports::{
    AccumulatorMembershipLeaf, AccumulatorMembershipLimits, AntiRollbackAnchorPort,
    AuthorityStoreCoordinate, ControlRecord, ControlStoreError, OperationCustodyIndexKey,
    OperationCustodyIndexQuery, OperationCustodyPort,
};
use std::cell::Cell;

mod history;
pub(in crate::resident) use history::read_history;

/// Bounds on retained observation history, separate from native membership work.
/// Per-record bounds also apply to the three initial index reads and every
/// checkpoint read. Their metadata bodies are not observation-history bytes.
/// Observation admission calls are bounded by the revision count. Checkpoint
/// ancestry has separate limits in
/// [`OperationCheckpointAncestryLimits`](crate::resident::OperationCheckpointAncestryLimits).
/// Each callback bounds its internal authority checks. These are not total-memory or SQL caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationObservationRecoveryLimits {
    maximum_revisions: usize,
    maximum_chain_bytes: usize,
    maximum_record_bytes: usize,
}

impl OperationObservationRecoveryLimits {
    /// Preserve nonzero raw history ceilings without a fallback quota.
    /// # Errors
    /// Returns a bound error if any ceiling is zero.
    pub fn new(
        maximum_revisions: usize,
        maximum_chain_bytes: usize,
        maximum_record_bytes: usize,
    ) -> Result<Self, OperationObservationReadmissionError> {
        if [maximum_revisions, maximum_chain_bytes, maximum_record_bytes].contains(&0) {
            return Err(OperationObservationReadmissionError::Bounds);
        }
        Ok(Self {
            maximum_revisions,
            maximum_chain_bytes,
            maximum_record_bytes,
        })
    }
}

/// Which admitted observation to return after checking the complete selected history.
#[derive(Debug, Clone, Copy)]
pub enum OperationObservationSelection<'a> {
    /// Return the head selected by the store at the start of this read.
    SelectedHead,
    /// Return this exact member only if it belongs to the selected head's history.
    HistoryMember {
        /// Existing observation reference, not an operation or checkpoint reference.
        reference: &'a ResidentRecordRefV1,
        /// Canonical observation digest paired with that reference.
        digest: &'a OperationObservationDigest,
    },
}

/// Current inspection permission must be established by the calling service.
/// The borrowed existing locator preserves the original operation and lease;
/// it does not itself grant current inspection or continuation permission.
pub struct OperationObservationRecoveryInput<'a> {
    /// Exact admitted store incarnation, not selected from a returned row.
    pub store: &'a AuthorityStoreCoordinate,
    /// Existing custody-issued or protected-recovered immutable locator.
    pub operation: &'a IssuedLocalOperation,
    /// Requested head or exact history member; neither grants permission to inspect it.
    pub observation: OperationObservationSelection<'a>,
    /// Resolved authority for the current checkpoint actually read.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Finite history and per-body ceilings.
    pub limits: OperationObservationRecoveryLimits,
    /// Separate checkpoint-history count, byte and admission-call ceilings.
    pub checkpoint_limits: checkpoint_recovery::OperationCheckpointAncestryLimits,
    /// Existing native leaf/read/proof ceilings, independent of history work.
    pub membership_limits: AccumulatorMembershipLimits,
}

/// Borrow-scoped operation identity consumed by the internal history proof.
/// Implementations cannot issue or expose an operation locator.
pub(in crate::resident) struct OperationObservationRecoveryIdentity<'a> {
    pub(in crate::resident) selection: crate::resident::OperationCheckpointSelection,
    pub(in crate::resident) handle_ref: &'a ResidentRecordRefV1,
    pub(in crate::resident) handle: &'a OperationHandleV1,
}

/// Private input view shared by issued and pre-issuance locator recovery.
pub(in crate::resident) struct OperationObservationRecoveryView<'a> {
    pub(in crate::resident) store: &'a AuthorityStoreCoordinate,
    pub(in crate::resident) identity: OperationObservationRecoveryIdentity<'a>,
    pub(in crate::resident) observation: OperationObservationSelection<'a>,
    pub(in crate::resident) checkpoint_authority:
        &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub(in crate::resident) limits: OperationObservationRecoveryLimits,
    pub(in crate::resident) checkpoint_limits:
        checkpoint_recovery::OperationCheckpointAncestryLimits,
    pub(in crate::resident) membership_limits: AccumulatorMembershipLimits,
}

pub(in crate::resident) trait OperationObservationRecoverySource {
    fn recovery_view(&self) -> OperationObservationRecoveryView<'_>;
}

impl OperationObservationRecoverySource for OperationObservationRecoveryInput<'_> {
    fn recovery_view(&self) -> OperationObservationRecoveryView<'_> {
        let (handle_ref, handle) = self.operation.handle();
        OperationObservationRecoveryView {
            store: self.store,
            identity: OperationObservationRecoveryIdentity {
                selection: self.operation.checkpoint_selection(),
                handle_ref,
                handle,
            },
            observation: self.observation,
            checkpoint_authority: self.checkpoint_authority,
            limits: self.limits,
            checkpoint_limits: self.checkpoint_limits,
            membership_limits: self.membership_limits,
        }
    }
}

#[derive(Clone, Copy)]
enum FailureStage {
    Context,
    State,
    Rotation,
    Signature,
}

/// Existing record-owner admission callbacks borrowed or owned for a bounded read.
/// This groups inputs only; constructing it establishes no authority or evidence.
pub struct OperationObservationRecoveryAdmission<Signer, State, Rotation, Signature, Checkpoint> {
    /// Resolve historical checkpoint authority at issuance and invoke its core
    /// signed admission. Lookup fields alone do not establish that authority.
    pub admit_checkpoint: Checkpoint,
    /// Resolve historical signer authority using untrusted wire lookup coordinates.
    pub resolve_signer: Signer,
    /// Admit each revision's existing owner-specific state evidence.
    pub admit_state: State,
    /// Prove any changed runtime-authority pair's admitted succession.
    pub admit_rotation: Rotation,
    /// Verify each exact contextual observation signature.
    pub verify_signature: Signature,
}

/// Read the selected head or an exact member of its admitted predecessor chain.
///
/// Resolves bounded historical bytes backward, then invokes the existing core
/// contextual owner forward. Historical signer resolution must prove authority
/// at issuance, not merely trust the wire lookup fields. Referenced state
/// evidence and authority rotations retain their existing owner contracts.
/// Current signed-checkpoint and independent-anchor membership then reject a
/// selected leaf that advanced during the read. There is no internal retry,
/// handler execution, signing, write or renewal. Checkpoint ancestry uses its
/// own bounded backward traversal and the existing core succession law.
///
/// Every revision is admitted, including descendants of a requested history member.
/// The head, not the requested old member, must pass current checkpoint membership
/// and both anchor checks. Selecting history retains at most one additional record;
/// it does not shorten the traversal or reduce its byte and revision charges.
///
/// The result proves membership in this anchored history. It is not a latest-
/// at-return promise, recovered execution frame, or publication/execution permission.
/// The selected observation's checkpoint index is physical evidence only; this
/// function proves its authenticated ancestry, not its claimed co-commit relation.
/// # Errors
/// Rejects incomplete/foreign/malformed history, exceeded bounds, historical
/// authority/state/signature failures, or changed checkpoint/anchor membership.
pub fn recover_operation_observation<
    Signer,
    State,
    Rotation,
    Signature,
    Checkpoint,
    CheckpointSignature,
>(
    input: OperationObservationRecoveryInput<'_>,
    port: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admissions: OperationObservationRecoveryAdmission<
        Signer,
        State,
        Rotation,
        Signature,
        Checkpoint,
    >,
    verify_checkpoint_signature: CheckpointSignature,
) -> Result<RecoveredOperationObservation, OperationObservationReadmissionError>
where
    Checkpoint: FnMut(
        &ResidentRecordRefV1,
        ControlStateCheckpointWireV1,
    ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
    Signer: FnMut(
        &ResidentRecordRefV1,
        &OperationObservationWireV1,
    ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
    State: FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
    Rotation: FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    Signature: FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    CheckpointSignature: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    recover_with_checkpoint(
        input,
        port,
        anchor,
        admissions,
        verify_checkpoint_signature,
        &mut |_, _| {},
    )
}

/// One selected observation and its authenticated head from the same bounded read.
///
/// Only successful recovery constructs this value, after admitting the complete
/// selected history and checking current checkpoint ancestry, membership and both
/// independent-anchor observations. A selected historical member belongs to the
/// retained head's predecessor chain; the endpoints are not independent reads.
///
/// This is anchored historical evidence, not current inspection permission, an
/// execution grant, or a promise that the selected head remains latest at return.
pub struct RecoveredOperationObservation {
    pub(in crate::resident) handle: pale_ports::ControlRecordCoordinate,
    selected: (ResidentRecordRefV1, OperationObservationV1),
    selected_predecessor: Option<(ResidentRecordRefV1, OperationObservationV1)>,
    selected_predecessor_predecessor: Option<(ResidentRecordRefV1, OperationObservationV1)>,
    context_binding: Option<(ResidentRecordRefV1, OperationObservationV1)>,
    pub(in crate::resident) current: checkpoint_recovery::CurrentOperationCheckpoint,
    selected_checkpoint_ref: ResidentRecordRefV1,
    selected_checkpoint: ControlStateCheckpointV1,
    head_checkpoint_ref: ResidentRecordRefV1,
    head_checkpoint: ControlStateCheckpointV1,
    store: AuthorityStoreCoordinate,
    head: Option<(ResidentRecordRefV1, OperationObservationV1)>,
    immediate_predecessor: Option<Box<RecoveredImmediatePredecessor>>,
}

struct RecoveredImmediatePredecessor {
    selected: (ResidentRecordRefV1, OperationObservationV1),
    selected_predecessor: Option<(ResidentRecordRefV1, OperationObservationV1)>,
    selected_predecessor_predecessor: Option<(ResidentRecordRefV1, OperationObservationV1)>,
    checkpoint_ref: ResidentRecordRefV1,
    checkpoint: ControlStateCheckpointV1,
}

impl std::fmt::Debug for RecoveredOperationObservation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecoveredOperationObservation")
            .finish_non_exhaustive()
    }
}

impl RecoveredOperationObservation {
    /// Select the immediate predecessor already admitted with a cancellation head.
    ///
    /// The initial protected read retains this bounded predecessor view and its
    /// authenticated checkpoint. This method performs no custody or anchor read.
    pub(in crate::resident) fn into_immediate_predecessor_selection(
        mut self,
    ) -> Result<Self, OperationObservationReadmissionError> {
        let predecessor = *self
            .immediate_predecessor
            .take()
            .ok_or(OperationObservationReadmissionError::Association)?;
        if self.head.is_some() {
            return Err(OperationObservationReadmissionError::Association);
        }
        self.head = Some(self.selected);
        self.selected = predecessor.selected;
        self.selected_predecessor = predecessor.selected_predecessor;
        self.selected_predecessor_predecessor = predecessor.selected_predecessor_predecessor;
        self.selected_checkpoint_ref = predecessor.checkpoint_ref;
        self.selected_checkpoint = predecessor.checkpoint;
        Ok(self)
    }

    /// Borrow the exact requested member, or the selected head when requested.
    #[must_use]
    pub const fn selected(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.selected.0, &self.selected.1)
    }

    /// Borrow the admitted predecessor of the selected observation, when present.
    ///
    /// This is the exact predecessor recovered from the same bounded history,
    /// not an independent point read. Carrier-specific recovery uses it to
    /// distinguish a cancellation requested before dispatch from one requested
    /// after an attempt crossed dispatch.
    #[must_use]
    pub const fn selected_predecessor(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &OperationObservationV1)> {
        match &self.selected_predecessor {
            Some((reference, observation)) => Some((reference, observation)),
            None => None,
        }
    }

    /// Borrow the admitted predecessor of the selected predecessor, when retained.
    ///
    /// This private witness supports bounded owner recovery across one direct
    /// nonterminal successor. It is collected by the same authenticated walk as
    /// `selected` and is never an independent or unbounded history lookup.
    #[must_use]
    pub(in crate::resident) const fn selected_predecessor_predecessor(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &OperationObservationV1)> {
        match &self.selected_predecessor_predecessor {
            Some((reference, observation)) => Some((reference, observation)),
            None => None,
        }
    }

    /// Borrow the admitted context binding in the selected history, when present.
    ///
    /// Recovery admits this witness with the complete predecessor chain. The
    /// value does not resolve or expose the context body. Continuations use it
    /// only to prove that an evidence-bearing operation passed through its
    /// required durable `ContextBound` phase.
    #[must_use]
    pub(in crate::resident) const fn context_binding(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &OperationObservationV1)> {
        match &self.context_binding {
            Some((reference, observation)) => Some((reference, observation)),
            None => None,
        }
    }

    /// Borrow the authenticated head whose admitted history contains `selected`.
    #[must_use]
    pub const fn selected_head(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        match &self.head {
            Some((reference, observation)) => (reference, observation),
            None => self.selected(),
        }
    }

    /// Borrow the exact admitted store incarnation used for this recovery.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the checkpoint indexed by the selected observation.
    ///
    /// The operation-custody owner proves the immutable index association. This
    /// recovery separately admits the checkpoint as an ancestor of the current
    /// authenticated head. The value does not by itself prove a publication.
    #[must_use]
    pub const fn selected_checkpoint(&self) -> &ControlStateCheckpointV1 {
        &self.selected_checkpoint
    }

    /// Borrow the checkpoint and reference indexed by the selected observation.
    #[must_use]
    pub const fn selected_checkpoint_record(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.selected_checkpoint_ref, &self.selected_checkpoint)
    }

    /// Borrow the checkpoint indexed by the authenticated operation head.
    ///
    /// For a selected historical member this is distinct from that member's
    /// checkpoint. Recovery admits both through the same bounded checkpoint walk.
    #[must_use]
    pub(in crate::resident) const fn selected_head_checkpoint_record(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.head_checkpoint_ref, &self.head_checkpoint)
    }

    /// Borrow the authenticated current checkpoint selected by this recovery.
    ///
    /// This is the append predecessor for new local or remote history. It can
    /// be newer than [`Self::selected_checkpoint`] when the requested history
    /// member was retained by an earlier checkpoint.
    #[must_use]
    pub fn current_checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (self.current.reference(), &self.current.record)
    }

    /// Verify extra application-owned leaves against this recovered checkpoint.
    ///
    /// The caller must construct each leaf through its semantic owner. This
    /// method only reuses the exact checkpoint snapshot and independent anchor
    /// selected by recovery. It does not interpret a leaf or renew authority.
    pub(in crate::resident) fn verify_current_checkpoint_membership(
        &self,
        selection: &OperationCheckpointSelection,
        leaves: Vec<AccumulatorMembershipLeaf>,
        limits: AccumulatorMembershipLimits,
        operation_port: &mut dyn OperationCustodyPort,
        anchor_port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<(), LocalOperationRecoveryError> {
        self.current
            .verify_membership(selection, leaves, limits, operation_port, anchor_port)
    }
}

#[cfg(test)]
#[path = "../../../tests/support/resident/operation_observation_recovery.rs"]
mod recovery_fixture;

pub(in crate::resident) fn recover_with_checkpoint<
    Input,
    Signer,
    State,
    Rotation,
    Signature,
    Checkpoint,
    CheckpointSignature,
>(
    input: Input,
    port: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admissions: OperationObservationRecoveryAdmission<
        Signer,
        State,
        Rotation,
        Signature,
        Checkpoint,
    >,
    verify_checkpoint_signature: CheckpointSignature,
    collect_admitted: &mut dyn FnMut(&ResidentRecordRefV1, &OperationObservationV1),
) -> Result<RecoveredOperationObservation, OperationObservationReadmissionError>
where
    Input: OperationObservationRecoverySource,
    Checkpoint: FnMut(
        &ResidentRecordRefV1,
        ControlStateCheckpointWireV1,
    ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
    Signer: FnMut(
        &ResidentRecordRefV1,
        &OperationObservationWireV1,
    ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
    State: FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
    Rotation: FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    Signature: FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    CheckpointSignature: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    let input = input.recovery_view();
    let OperationObservationRecoveryAdmission {
        mut admit_checkpoint,
        mut resolve_signer,
        mut admit_state,
        mut admit_rotation,
        mut verify_signature,
    } = admissions;
    let selection = &input.identity.selection;
    let handle_ref = input.identity.handle_ref;
    let handle = input.identity.handle;
    if selection
        .initial_basis()
        .map_err(LocalOperationRecoveryError::from)?
        .store()
        != input.store
        || !selection.matches_authority(input.checkpoint_authority)
        || input.checkpoint_authority.principal_id != handle.issuer_principal_id()
    {
        return Err(OperationObservationReadmissionError::Association);
    }
    let evidence = super::read_operation_observation_for_handle(
        port,
        input.store,
        handle_ref,
        handle,
        input.limits.maximum_record_bytes,
    )?
    .ok_or(OperationObservationReadmissionError::Missing)?;
    let selected_coordinate = evidence.observation.coordinate().clone();
    let history = read_history(port, input.store, evidence.observation, input.limits)?;
    let schema = pale_contracts::record_schema_identity(OPERATION_OBSERVATION_V1_SCHEMA_ID)?;
    let mut admitted: Option<(ResidentRecordRefV1, OperationObservationV1)> = None;
    let mut admitted_predecessor = None;
    let mut admitted_predecessor_predecessor = None;
    let mut admitted_predecessor_predecessor_predecessor = None;
    let mut admitted_context_binding = None;
    let mut requested_member = None;
    let mut requested_predecessor = None;
    let mut requested_predecessor_predecessor = None;
    for (record, wire) in history.into_iter().rev() {
        let authority = resolve_signer(record.coordinate().record_ref(), &wire)
            .map_err(OperationObservationReadmissionError::HistoricalAuthority)?;
        let stage = Cell::new(FailureStage::Context);
        let observation = OperationObservationV1::admit_contextual_wire(
            wire,
            &OperationObservationAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema.schema_sha256,
                    signing_authority: &authority,
                },
                handle_ref,
                handle,
                predecessor: admitted
                    .as_ref()
                    .map(|(reference, record)| (reference, record)),
            },
            |state| admit_state(state).inspect_err(|_| stage.set(FailureStage::State)),
            |a, b, c, d| {
                admit_rotation(a, b, c, d).inspect_err(|_| stage.set(FailureStage::Rotation))
            },
            |domain, preimage, digest, signature| {
                verify_signature(domain, preimage, digest, signature)
                    .inspect_err(|_| stage.set(FailureStage::Signature))
            },
        )
        .map_err(|error| match stage.get() {
            FailureStage::Context => OperationObservationReadmissionError::Record(error),
            FailureStage::State => OperationObservationReadmissionError::StateEvidence(error),
            FailureStage::Rotation => {
                OperationObservationReadmissionError::HistoricalAuthority(error)
            }
            FailureStage::Signature => OperationObservationReadmissionError::Signature(error),
        })?;
        if record.coordinate().record_sha256().as_str()
            != observation
                .canonical_operation_observation_sha256()
                .as_str()
            || canon::to_canonical_string(&observation)?.as_bytes()
                != record.canonical_record_jcs().as_slice()
        {
            return Err(OperationObservationReadmissionError::Association);
        }
        // Private collectors may retain exact admitted witnesses, but their
        // caller must discard them unless the final checkpoint/anchor proof succeeds.
        collect_admitted(record.coordinate().record_ref(), &observation);
        if matches!(
            observation.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ContextBound { .. },
            }
        ) {
            admitted_context_binding = Some((
                record.coordinate().record_ref().clone(),
                observation.clone(),
            ));
        }
        match input.observation {
            OperationObservationSelection::HistoryMember { reference, digest }
                if reference == record.coordinate().record_ref()
                    && digest == observation.canonical_operation_observation_sha256() =>
            {
                requested_member = Some((reference.clone(), observation.clone()));
                requested_predecessor = admitted.clone();
                requested_predecessor_predecessor = admitted_predecessor.clone();
            }
            _ => {}
        }
        admitted_predecessor_predecessor_predecessor = admitted_predecessor_predecessor;
        admitted_predecessor_predecessor = admitted_predecessor;
        admitted_predecessor = admitted;
        admitted = Some((record.coordinate().record_ref().clone(), observation));
    }
    let (reference, observation) = admitted.ok_or(OperationObservationReadmissionError::Missing)?;
    if &reference != selected_coordinate.record_ref()
        || observation
            .canonical_operation_observation_sha256()
            .as_str()
            != selected_coordinate.record_sha256().as_str()
    {
        return Err(OperationObservationReadmissionError::Association);
    }
    let current = checkpoint_recovery::read_current_checkpoint(
        checkpoint_recovery::CheckpointRecoveryInput {
            store: input.store,
            selection,
            handle,
            authority: input.checkpoint_authority,
            maximum_record_bytes: input.limits.maximum_record_bytes,
        },
        port,
        anchor,
        verify_checkpoint_signature,
    )?;
    let leaf =
        operation_observation_membership(input.store, &current.record, &reference, &observation)
            .map_err(|_| OperationObservationReadmissionError::Association)?;
    let (selected, selected_predecessor, selected_predecessor_predecessor, head) =
        match input.observation {
            OperationObservationSelection::SelectedHead => (
                (reference, observation),
                admitted_predecessor,
                admitted_predecessor_predecessor,
                None,
            ),
            OperationObservationSelection::HistoryMember { .. } => {
                let Some(requested) = requested_member else {
                    current.verify_ancestry(
                        &evidence.checkpoint,
                        input.limits.maximum_record_bytes,
                        input.checkpoint_limits,
                        port,
                        &mut admit_checkpoint,
                    )?;
                    current.verify_membership(
                        selection,
                        vec![leaf],
                        input.membership_limits,
                        port,
                        anchor,
                    )?;
                    return Err(OperationObservationReadmissionError::Missing);
                };
                (
                    requested,
                    requested_predecessor,
                    requested_predecessor_predecessor,
                    Some((reference, observation)),
                )
            }
        };
    let immediate_predecessor = if head.is_none()
        && matches!(
            selected.1.state(),
            OperationObservationStateV1::CancellationRequested { .. }
        ) {
        selected_predecessor.as_ref().map(|predecessor| {
            (
                predecessor.clone(),
                selected_predecessor_predecessor.clone(),
                admitted_predecessor_predecessor_predecessor.clone(),
            )
        })
    } else {
        None
    };
    let historical_selection = head
        .as_ref()
        .map(|_| &selected)
        .or_else(|| immediate_predecessor.as_ref().map(|selection| &selection.0));
    let historical_checkpoint_record = if let Some(historical) = historical_selection {
        let historical_coordinate = crate::resident::record_projection::coordinate(
            &historical.0,
            historical
                .1
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .map_err(|_| OperationObservationReadmissionError::Association)?;
        let query = OperationCustodyIndexQuery::ObservationCheckpoint(
            OperationCustodyIndexKey::for_observation(&historical_coordinate),
        );
        Some(
            super::super::operation_custody::read_index(
                port,
                input.store,
                query,
                input.limits.maximum_record_bytes,
            )?
            .ok_or(OperationRecoveryError::IncompleteObservationIndexes)?,
        )
    } else {
        None
    };
    let mut checkpoint_records = vec![&evidence.checkpoint];
    if let Some(record) = historical_checkpoint_record.as_ref() {
        checkpoint_records.push(record);
    }
    let admitted_checkpoints = current.verify_ancestry_set(
        &checkpoint_records,
        input.limits.maximum_record_bytes,
        input.checkpoint_limits,
        port,
        &mut admit_checkpoint,
    )?;
    current.verify_membership(selection, vec![leaf], input.membership_limits, port, anchor)?;
    let head_checkpoint_ref = evidence.checkpoint.coordinate().record_ref().clone();
    let head_checkpoint = admitted_checkpoints
        .first()
        .cloned()
        .ok_or(OperationObservationReadmissionError::Association)?;
    let historical_checkpoint = historical_checkpoint_record
        .as_ref()
        .map(|record| {
            let checkpoint = admitted_checkpoints
                .last()
                .cloned()
                .ok_or(OperationObservationReadmissionError::Association)?;
            Ok::<_, OperationObservationReadmissionError>((
                record.coordinate().record_ref().clone(),
                checkpoint,
            ))
        })
        .transpose()?;
    let (selected_checkpoint_ref, selected_checkpoint) = if head.is_some() {
        historical_checkpoint
            .clone()
            .ok_or(OperationObservationReadmissionError::Association)?
    } else {
        (head_checkpoint_ref.clone(), head_checkpoint.clone())
    };
    let immediate_predecessor = immediate_predecessor.zip(historical_checkpoint).map(
        |(
            (selected, selected_predecessor, selected_predecessor_predecessor),
            (checkpoint_ref, checkpoint),
        )| {
            Box::new(RecoveredImmediatePredecessor {
                selected,
                selected_predecessor,
                selected_predecessor_predecessor,
                checkpoint_ref,
                checkpoint,
            })
        },
    );
    Ok(RecoveredOperationObservation {
        handle: crate::resident::record_projection::coordinate(
            handle_ref,
            handle.canonical_operation_handle_sha256().as_str(),
        )
        .map_err(|_| OperationObservationReadmissionError::Association)?,
        selected,
        selected_predecessor,
        selected_predecessor_predecessor,
        context_binding: admitted_context_binding,
        current,
        selected_checkpoint_ref,
        selected_checkpoint,
        head_checkpoint_ref,
        head_checkpoint,
        store: input.store.clone(),
        head,
        immediate_predecessor,
    })
}

/// Precise rejected recovery stage; no renewed execution or inspection grant.
#[derive(Debug, thiserror::Error)]
pub enum OperationObservationReadmissionError {
    /// A selected record, predecessor or requested history member is unavailable.
    /// A reference paired with another digest is not an exact member.
    #[error("operation observation history is missing")]
    Missing,
    /// A finite history count or byte ceiling was exceeded.
    #[error("operation observation recovery bounds exceeded")]
    Bounds,
    /// Claimed predecessor shape or strictly decreasing revision failed.
    #[error("operation observation history is not contiguous")]
    Lineage,
    /// Exact selected store/schema/reference/digest/bytes differ.
    #[error("operation observation recovery association differs")]
    Association,
    /// Physical selected indexes are missing or conflicting.
    #[error(transparent)]
    Lookup(#[from] OperationRecoveryError),
    /// Bounded immutable-body custody read failed.
    #[error(transparent)]
    Custody(#[from] ControlStoreError),
    /// Contract owner identity is unavailable.
    #[error(transparent)]
    Schema(#[from] pale_contracts::SchemaError),
    /// Strict byte parsing/canonical encoding failed.
    #[error(transparent)]
    Canonical(#[from] canon::CanonError),
    /// Structural wire decoding failed.
    #[error(transparent)]
    Decode(#[from] serde_json::Error),
    /// Existing contextual observation lineage admission failed.
    #[error("operation observation contextual admission: {0}")]
    Record(InvalidResidentRecordV1),
    /// Historical signer or runtime-authority rotation was unavailable/rejected.
    #[error("operation observation historical authority: {0}")]
    HistoricalAuthority(InvalidResidentRecordV1),
    /// Referenced result/diagnostic/receipt/reconciliation evidence was rejected.
    #[error("operation observation state evidence: {0}")]
    StateEvidence(InvalidResidentRecordV1),
    /// The historical signature verifier rejected the exact record.
    #[error("operation observation signature: {0}")]
    Signature(InvalidResidentRecordV1),
    /// Existing signed checkpoint, independent anchor or exact membership fence failed.
    #[error(transparent)]
    Checkpoint(#[from] LocalOperationRecoveryError),
}
