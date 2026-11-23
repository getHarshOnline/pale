//! Protected recovery of an existing immutable locator from current custody.
//! The original start proof is admitted but not exposed as mutable or current state.

use pale_core::{canon, records::*};
use pale_ports::{
    AccumulatorMembershipLimits, AntiRollbackAnchorError, AntiRollbackAnchorPort,
    AuthorityStoreCoordinate, ControlRecord, ControlStoreError,
    OperationAccumulatorPreparationError, OperationCustodyPort, SessionLeaseCustodyPort,
};

use super::IssuedLocalOperation;
use crate::resident::checkpoint_recovery::check_record;
use crate::resident::{
    checkpoint_profile::{
        initial_operation_leaves, operation_identity_leaves, operation_session_basis_leaf,
        OperationCheckpointSelection, SupportedOperationCheckpointProfile,
    },
    lifecycle::GenerationLifecycleRecoveryResolver,
    operation_custody::{read_operation_start_for_request, OperationRecoveryError},
    operation_lease_custody::{
        recover_operation_lease_and_parent, OperationLeaseBodyRecoveryError,
    },
    operation_observation_custody::{
        recover_with_checkpoint, OperationObservationReadmissionError,
        OperationObservationRecoveryAdmission, OperationObservationRecoveryIdentity,
        OperationObservationRecoveryLimits, OperationObservationRecoverySource,
        OperationObservationRecoveryView, OperationObservationSelection,
    },
    record_projection, RecoveredOperationObservation,
};

mod by_handle;
mod identity;
mod pending_expiry;
mod pending_expiry_child;
pub(in crate::resident) use pending_expiry::{
    recover_pending_root_observation, PendingRootObservationError,
};
pub(in crate::resident) mod discovery;
pub(in crate::resident) use by_handle::{
    recover_local_child_operation_from_handle, LocalChildOperationHandleRecoveryInput,
};
pub use by_handle::{
    recover_local_operation_from_handle, LocalOperationHandleRecoveryInput, RecoveredLocalOperation,
};

/// Store, historical authority and limits shared by both recovery entry points.
/// Permission to inspect this store must be established before calling recovery;
/// this input neither grants that permission nor supplies a production backend.
#[derive(Clone, Copy)]
pub struct LocalOperationRecoveryContext<'a> {
    /// Exact admitted deployment-store incarnation, never inferred from a row.
    pub store: &'a AuthorityStoreCoordinate,
    /// Exact source-supported profile bytes already admitted by their owner.
    pub profile: &'a SupportedOperationCheckpointProfile,
    /// Historical handle-signing authority resolved active at issuance.
    pub handle_signing_authority: &'a SigningAuthorityBindingV1,
    /// Resolved signing authority for the checkpoint actually read.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Existing owner of retained generation, profile and historical session parents.
    pub historical: &'a dyn GenerationLifecycleRecoveryResolver,
    /// Maximum bytes per request, handle, checkpoint or private session-basis body.
    pub maximum_record_bytes: usize,
    /// Maximum retained operation-lease body bytes.
    pub maximum_operation_lease_bytes: usize,
    /// Maximum retained parent-session body bytes.
    pub maximum_session_lease_bytes: usize,
    /// Exact selected-leaf and native read/proof attempt ceilings.
    pub membership_limits: AccumulatorMembershipLimits,
    /// Revision, aggregate-byte, and per-record ceilings for the original start proof.
    pub observation_limits: OperationObservationRecoveryLimits,
    /// Checkpoint-history ceilings for the original start proof.
    pub checkpoint_limits: crate::resident::OperationCheckpointAncestryLimits,
}

impl LocalOperationRecoveryContext<'_> {
    fn verify_limits(&self) -> Result<(), LocalOperationRecoveryError> {
        if [
            self.maximum_record_bytes,
            self.maximum_operation_lease_bytes,
            self.maximum_session_lease_bytes,
        ]
        .contains(&0)
            || self.membership_limits.maximum_leaves() < 3
        {
            return Err(LocalOperationRecoveryError::Bounds);
        }
        Ok(())
    }
}

/// Existing historical authority and signature checks used during recovery.
/// Each callback retains its core admission contract and internal work limits.
pub struct LocalOperationRecoveryAdmission<
    Authority,
    HandleSignature,
    CheckpointSignature,
    ObservationSigner,
    ObservationState,
    AuthorityRotation,
    ObservationSignature,
    HistoricalCheckpoint,
> {
    /// Resolve handle authority at issuance; this does not renew its lease.
    pub admit_historical_authority: Authority,
    /// Verify the handle signature with the resolved historical authority.
    pub verify_handle_signature: HandleSignature,
    /// Verify the currently selected checkpoint's signature.
    pub verify_checkpoint_signature: CheckpointSignature,
    /// Existing owner admissions for the exact revision-one observation and checkpoint history.
    pub initial_observation: OperationObservationRecoveryAdmission<
        ObservationSigner,
        ObservationState,
        AuthorityRotation,
        ObservationSignature,
        HistoricalCheckpoint,
    >,
}

/// Recover a root Invoke or Consult locator and its immutable session provenance from current
/// authenticated request, handle and basis membership.
///
/// `request` is the original contextually admitted request, not a new start.
/// The historical authority and signature callbacks have the existing core
/// admission contracts; unresolved evidence must reject. This function performs
/// no signing, custody writes, anchor advance, expiry renewal or native dispatch.
/// It proves the exact revision-one start observation and its first domain checkpoint,
/// but does not return the latest observation. It restores only historical session provenance,
/// never current session admission,
/// and does not grant current observation/continuation authority. A changed head or
/// anchor is a failed attempt, not an internal retry or a new start.
///
/// # Errors
/// Rejects missing, conflicting, misdirected, noncanonical, unadmitted or
/// excessive records, parent/authority failures, and unlike anchor or membership.
pub fn recover_local_operation<
    Authority,
    HandleSignature,
    CheckpointSignature,
    ObservationSigner,
    ObservationState,
    AuthorityRotation,
    ObservationSignature,
    HistoricalCheckpoint,
>(
    context: LocalOperationRecoveryContext<'_>,
    request: &CommonRequestEnvelopeV1,
    operation_port: &mut dyn OperationCustodyPort,
    session_port: &dyn SessionLeaseCustodyPort,
    anchor_port: &mut dyn AntiRollbackAnchorPort,
    admission: LocalOperationRecoveryAdmission<
        Authority,
        HandleSignature,
        CheckpointSignature,
        ObservationSigner,
        ObservationState,
        AuthorityRotation,
        ObservationSignature,
        HistoricalCheckpoint,
    >,
) -> Result<IssuedLocalOperation, LocalOperationRecoveryError>
where
    Authority: FnOnce(
        &OperationHandlePreimageV1,
        Option<&OperationHandleV1>,
    ) -> Result<(), InvalidResidentRecordV1>,
    HandleSignature: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    CheckpointSignature: FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    HistoricalCheckpoint: FnMut(
        &ResidentRecordRefV1,
        ControlStateCheckpointWireV1,
    ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
    ObservationSigner: FnMut(
        &ResidentRecordRefV1,
        &OperationObservationWireV1,
    ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
    ObservationState: FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
    AuthorityRotation: FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    ObservationSignature: FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    recover(
        context,
        request,
        operation_port,
        session_port,
        anchor_port,
        admission,
    )
    .map(|recovered| recovered.operation)
}

/// Historical parent required to recover one exact child, without recursive lookup.
/// The parent must itself be custody-issued or protected-recovered. Its original
/// admitted lease supplies narrowing evidence, never renewed execution authority.
pub struct LocalChildOperationRecoveryContext<'a> {
    /// Existing store, historical admission and explicit recovery limits.
    pub context: LocalOperationRecoveryContext<'a>,
    /// Exact already-protected parent locator in the inherited checkpoint domain.
    pub parent: &'a IssuedLocalOperation,
    /// Original parent operation lease, not a replacement or new grant.
    pub parent_lease: &'a OperationLeaseV1,
}

/// Recover a child Invoke, Consult or Delegate from its exact request and protected parent.
/// Reuses root recovery's historical admission, current membership and anchor
/// fences. The signed child commits the parent identity; its exact pairwise
/// index must agree. No parent liveness check or expiry renewal occurs here.
/// This does not prove a Consult roster or the historical checkpoint co-commit.
/// # Errors
/// Rejects wrong parent/lease/session/generation, missing membership, record
/// substitutions and the existing bounded locator-recovery failures.
pub fn recover_local_child_operation<
    Authority,
    HandleSignature,
    CheckpointSignature,
    ObservationSigner,
    ObservationState,
    AuthorityRotation,
    ObservationSignature,
    HistoricalCheckpoint,
>(
    input: LocalChildOperationRecoveryContext<'_>,
    request: &CommonRequestEnvelopeV1,
    operation_port: &mut dyn OperationCustodyPort,
    session_port: &dyn SessionLeaseCustodyPort,
    anchor_port: &mut dyn AntiRollbackAnchorPort,
    admission: LocalOperationRecoveryAdmission<
        Authority,
        HandleSignature,
        CheckpointSignature,
        ObservationSigner,
        ObservationState,
        AuthorityRotation,
        ObservationSignature,
        HistoricalCheckpoint,
    >,
) -> Result<IssuedLocalOperation, LocalOperationRecoveryError>
where
    Authority: FnOnce(
        &OperationHandlePreimageV1,
        Option<&OperationHandleV1>,
    ) -> Result<(), InvalidResidentRecordV1>,
    HandleSignature: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    CheckpointSignature: FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    HistoricalCheckpoint: FnMut(
        &ResidentRecordRefV1,
        ControlStateCheckpointWireV1,
    ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
    ObservationSigner: FnMut(
        &ResidentRecordRefV1,
        &OperationObservationWireV1,
    ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
    ObservationState: FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
    AuthorityRotation: FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    ObservationSignature: FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    recover_with_parent(
        input.context,
        request,
        Some((input.parent, input.parent_lease)),
        operation_port,
        session_port,
        anchor_port,
        admission,
    )
    .map(|recovered| recovered.operation)
}

// Keep the exact rows already checked by recovery. Handle-based callers compare
// them with the initial lookup without encoding another copy of the handle.
struct RecoveredIdentity {
    operation: IssuedLocalOperation,
    initial_observation: RecoveredOperationObservation,
    handle_record: ControlRecord,
}

/// Borrowed pre-issuance identity used only by the existing observation proof owner.
/// It cannot expose a locator or dispatch work.
struct ProvisionalRecoveredOperation<'a> {
    store: &'a AuthorityStoreCoordinate,
    selection: OperationCheckpointSelection,
    handle_ref: &'a ResidentRecordRefV1,
    handle: &'a OperationHandleV1,
    observation: OperationObservationSelection<'a>,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    limits: OperationObservationRecoveryLimits,
    checkpoint_limits: crate::resident::OperationCheckpointAncestryLimits,
    membership_limits: AccumulatorMembershipLimits,
}

impl OperationObservationRecoverySource for ProvisionalRecoveredOperation<'_> {
    fn recovery_view(&self) -> OperationObservationRecoveryView<'_> {
        OperationObservationRecoveryView {
            store: self.store,
            identity: OperationObservationRecoveryIdentity {
                selection: self.selection.clone(),
                handle_ref: self.handle_ref,
                handle: self.handle,
            },
            observation: self.observation,
            checkpoint_authority: self.checkpoint_authority,
            limits: self.limits,
            checkpoint_limits: self.checkpoint_limits,
            membership_limits: self.membership_limits,
        }
    }
}

fn recover<
    Authority,
    HandleSignature,
    CheckpointSignature,
    ObservationSigner,
    ObservationState,
    AuthorityRotation,
    ObservationSignature,
    HistoricalCheckpoint,
>(
    input: LocalOperationRecoveryContext<'_>,
    request: &CommonRequestEnvelopeV1,
    operation_port: &mut dyn OperationCustodyPort,
    session_port: &dyn SessionLeaseCustodyPort,
    anchor_port: &mut dyn AntiRollbackAnchorPort,
    admission: LocalOperationRecoveryAdmission<
        Authority,
        HandleSignature,
        CheckpointSignature,
        ObservationSigner,
        ObservationState,
        AuthorityRotation,
        ObservationSignature,
        HistoricalCheckpoint,
    >,
) -> Result<RecoveredIdentity, LocalOperationRecoveryError>
where
    Authority: FnOnce(
        &OperationHandlePreimageV1,
        Option<&OperationHandleV1>,
    ) -> Result<(), InvalidResidentRecordV1>,
    HandleSignature: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    CheckpointSignature: FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    HistoricalCheckpoint: FnMut(
        &ResidentRecordRefV1,
        ControlStateCheckpointWireV1,
    ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
    ObservationSigner: FnMut(
        &ResidentRecordRefV1,
        &OperationObservationWireV1,
    ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
    ObservationState: FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
    AuthorityRotation: FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    ObservationSignature: FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    recover_with_parent(
        input,
        request,
        None,
        operation_port,
        session_port,
        anchor_port,
        admission,
    )
}

fn recover_with_parent<
    Authority,
    HandleSignature,
    CheckpointSignature,
    ObservationSigner,
    ObservationState,
    AuthorityRotation,
    ObservationSignature,
    HistoricalCheckpoint,
>(
    input: LocalOperationRecoveryContext<'_>,
    request: &CommonRequestEnvelopeV1,
    parent: Option<(&IssuedLocalOperation, &OperationLeaseV1)>,
    operation_port: &mut dyn OperationCustodyPort,
    session_port: &dyn SessionLeaseCustodyPort,
    anchor_port: &mut dyn AntiRollbackAnchorPort,
    admission: LocalOperationRecoveryAdmission<
        Authority,
        HandleSignature,
        CheckpointSignature,
        ObservationSigner,
        ObservationState,
        AuthorityRotation,
        ObservationSignature,
        HistoricalCheckpoint,
    >,
) -> Result<RecoveredIdentity, LocalOperationRecoveryError>
where
    Authority: FnOnce(
        &OperationHandlePreimageV1,
        Option<&OperationHandleV1>,
    ) -> Result<(), InvalidResidentRecordV1>,
    HandleSignature: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    CheckpointSignature: FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    HistoricalCheckpoint: FnMut(
        &ResidentRecordRefV1,
        ControlStateCheckpointWireV1,
    ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
    ObservationSigner: FnMut(
        &ResidentRecordRefV1,
        &OperationObservationWireV1,
    ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
    ObservationState: FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
    AuthorityRotation: FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    ObservationSignature: FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    input.verify_limits()?;
    let LocalOperationRecoveryAdmission {
        admit_historical_authority,
        verify_handle_signature,
        mut verify_checkpoint_signature,
        initial_observation,
    } = admission;
    let selection = match parent {
        None => input.profile.select(input.store, request)?,
        Some((operation, lease)) => {
            if operation.handle().1.operation_lease_sha256() != lease.operation_lease_digest() {
                return Err(LocalOperationRecoveryError::Association);
            }
            operation
                .checkpoint_selection()
                .child(input.profile, request)?
        }
    };
    if !selection.matches_store(input.store)
        || !selection.matches_authority(input.checkpoint_authority)
    {
        return Err(LocalOperationRecoveryError::Association);
    }
    let start_evidence = read_operation_start_for_request(
        operation_port,
        input.store,
        request,
        input.maximum_record_bytes,
    )?
    .ok_or(LocalOperationRecoveryError::Missing)?;
    let request_record = start_evidence.request().clone();
    let handle_record = start_evidence.handle().clone();
    let (handle, lease, session) = identity::admit_handle(
        input,
        request,
        parent.map(|(operation, lease)| (operation.handle().0, operation.handle().1, lease)),
        &handle_record,
        operation_port,
        session_port,
        (admit_historical_authority, verify_handle_signature),
    )?;
    let session_basis = operation_port
        .read_operation_session_basis(
            input.store,
            handle_record.coordinate(),
            input
                .maximum_record_bytes
                .min(pale_ports::MAX_OPERATION_SESSION_BASIS_BYTES),
        )?
        .ok_or(LocalOperationRecoveryError::Missing)?;
    let (ambient_basis, publication_policy, consultation_start_intent) =
        super::session_basis::decode(
            &session_basis,
            &selection,
            handle_record.coordinate(),
            request_record.coordinate(),
            &session,
            input.maximum_record_bytes,
        )?;
    let current_checkpoint = crate::resident::checkpoint_recovery::read_current_checkpoint(
        crate::resident::checkpoint_recovery::CheckpointRecoveryInput {
            store: input.store,
            selection: &selection,
            handle: &handle,
            authority: input.checkpoint_authority,
            maximum_record_bytes: input.maximum_record_bytes,
        },
        operation_port,
        anchor_port,
        &mut verify_checkpoint_signature,
    )?;
    let mut leaves = operation_identity_leaves(
        &selection,
        (request_record.coordinate().record_ref(), request),
        (handle_record.coordinate().record_ref(), &handle),
    )
    .map_err(|_| LocalOperationRecoveryError::Association)?
    .membership()
    .map_err(|_| LocalOperationRecoveryError::Association)?;
    leaves.push(
        operation_session_basis_leaf(
            &selection,
            (handle_record.coordinate().record_ref(), &handle),
            (request_record.coordinate().record_ref(), request),
            &session,
            &session_basis,
        )
        .and_then(|leaf| leaf.membership())
        .map_err(|_| LocalOperationRecoveryError::Association)?,
    );
    leaves.sort_unstable_by(|left, right| left.key().cmp(right.key()));
    current_checkpoint.verify_membership(
        &selection,
        leaves,
        input.membership_limits,
        operation_port,
        anchor_port,
    )?;
    if let Some(intent) = &consultation_start_intent {
        intent
            .validate_signed_parent(
                input.store,
                handle_record.coordinate().record_ref(),
                &handle,
            )
            .map_err(map_consultation_start_intent_error)?;
    }
    let initial_observation_digest = OperationObservationDigest::parse(
        start_evidence
            .initial_observation()
            .coordinate()
            .record_sha256()
            .as_str()
            .to_owned(),
    )
    .map_err(|_| LocalOperationRecoveryError::Association)?;
    let recovered_start = recover_with_checkpoint(
        ProvisionalRecoveredOperation {
            store: input.store,
            selection: selection.clone(),
            handle_ref: handle_record.coordinate().record_ref(),
            handle: &handle,
            observation: OperationObservationSelection::HistoryMember {
                reference: start_evidence
                    .initial_observation()
                    .coordinate()
                    .record_ref(),
                digest: &initial_observation_digest,
            },
            checkpoint_authority: input.checkpoint_authority,
            limits: input.observation_limits,
            checkpoint_limits: input.checkpoint_limits,
            membership_limits: input.membership_limits,
        },
        operation_port,
        anchor_port,
        initial_observation,
        &mut verify_checkpoint_signature,
        &mut |_, _| {},
    )
    .map_err(map_initial_observation_error)?;
    let (initial_ref, initial) = recovered_start.selected();
    let (checkpoint_ref, checkpoint) = recovered_start.selected_checkpoint_record();
    // Checkpoint sequence is domain-wide, so a later child start need not use
    // sequence one. Its exact four changed keys identify the start transaction.
    let mut expected_start_changes = initial_operation_leaves(
        &selection,
        (request_record.coordinate().record_ref(), request),
        (handle_record.coordinate().record_ref(), &handle),
        (initial_ref, initial),
        &session_basis,
        &session,
    )
    .map_err(|_| LocalOperationRecoveryError::Association)?
    .changes()
    .map_err(|_| LocalOperationRecoveryError::Association)?
    .into_iter()
    .map(|(key, _, _)| key)
    .collect::<Vec<_>>();
    expected_start_changes.sort();
    if initial.revision().get() != 1
        || initial.predecessor_ref().is_some()
        || checkpoint.changed_leaf_keys() != expected_start_changes.as_slice()
        || initial_ref
            != start_evidence
                .initial_observation()
                .coordinate()
                .record_ref()
        || initial.canonical_operation_observation_sha256().as_str()
            != start_evidence
                .initial_observation()
                .coordinate()
                .record_sha256()
                .as_str()
        || canon::to_canonical_string(initial)?.as_bytes()
            != start_evidence
                .initial_observation()
                .canonical_record_jcs()
                .as_slice()
        || checkpoint_ref != start_evidence.checkpoint().coordinate().record_ref()
        || checkpoint.canonical_control_checkpoint_sha256().as_str()
            != start_evidence
                .checkpoint()
                .coordinate()
                .record_sha256()
                .as_str()
        || canon::to_canonical_string(checkpoint)?.as_bytes()
            != start_evidence
                .checkpoint()
                .canonical_record_jcs()
                .as_slice()
    {
        return Err(LocalOperationRecoveryError::Association);
    }
    Ok(RecoveredIdentity {
        operation: IssuedLocalOperation {
            checkpoint_selection: selection,
            request_coordinate: request_record.coordinate().clone(),
            handle_ref: handle_record.coordinate().record_ref().clone(),
            handle,
            initial_observation_ref: initial_ref.clone(),
            initial_observation: initial.clone(),
            operation_lease: lease,
            retained_session:
                crate::resident::session_admission::RetainedAmbientSession::from_protected(
                    session,
                    ambient_basis,
                ),
            publication_policy,
            consultation_start_intent,
        },
        initial_observation: recovered_start,
        handle_record,
    })
}

fn map_consultation_start_intent_error(
    error: crate::resident::consultation::ConsultationStartIntentError,
) -> LocalOperationRecoveryError {
    match error {
        crate::resident::consultation::ConsultationStartIntentError::Bounds => {
            LocalOperationRecoveryError::Bounds
        }
        crate::resident::consultation::ConsultationStartIntentError::Canonical(error) => {
            LocalOperationRecoveryError::Canonical(error)
        }
        crate::resident::consultation::ConsultationStartIntentError::Association
        | crate::resident::consultation::ConsultationStartIntentError::DuplicateSlot
        | crate::resident::consultation::ConsultationStartIntentError::DuplicateChild
        | crate::resident::consultation::ConsultationStartIntentError::Child => {
            LocalOperationRecoveryError::Association
        }
    }
}

fn map_initial_observation_error(
    error: OperationObservationReadmissionError,
) -> LocalOperationRecoveryError {
    match error {
        OperationObservationReadmissionError::Missing => LocalOperationRecoveryError::Missing,
        OperationObservationReadmissionError::Bounds => LocalOperationRecoveryError::Bounds,
        OperationObservationReadmissionError::Lineage
        | OperationObservationReadmissionError::Association => {
            LocalOperationRecoveryError::Association
        }
        OperationObservationReadmissionError::Lookup(error) => {
            LocalOperationRecoveryError::Lookup(error)
        }
        OperationObservationReadmissionError::Custody(error) => {
            LocalOperationRecoveryError::Store(error)
        }
        OperationObservationReadmissionError::Schema(error) => {
            LocalOperationRecoveryError::Schema(error)
        }
        OperationObservationReadmissionError::Canonical(error) => {
            LocalOperationRecoveryError::Canonical(error)
        }
        OperationObservationReadmissionError::Decode(error) => {
            LocalOperationRecoveryError::Decode(error)
        }
        OperationObservationReadmissionError::Record(error)
        | OperationObservationReadmissionError::HistoricalAuthority(error)
        | OperationObservationReadmissionError::StateEvidence(error)
        | OperationObservationReadmissionError::Signature(error) => {
            LocalOperationRecoveryError::Record(error)
        }
        OperationObservationReadmissionError::Checkpoint(error) => error,
    }
}

/// Failed protected recovery; no usable locator or renewed authority is returned.
#[derive(Debug, thiserror::Error)]
pub enum LocalOperationRecoveryError {
    /// Required retained evidence or independent anchor is absent.
    #[error("operation locator recovery evidence is missing")]
    Missing,
    /// A finite input or returned body exceeds its exact ceiling.
    #[error("operation locator recovery bound is invalid or exceeded")]
    Bounds,
    /// Returned evidence does not belong to the exact selected association.
    #[error("operation locator recovery association differs")]
    Association,
    /// Existing physical request conflict or lookup closure failed.
    #[error(transparent)]
    Lookup(#[from] OperationRecoveryError),
    /// Existing historical parent admission failed; nothing is renewed.
    #[error(transparent)]
    Lease(#[from] OperationLeaseBodyRecoveryError),
    /// Contextual record or signature admission rejected the evidence.
    #[error(transparent)]
    Record(#[from] InvalidResidentRecordV1),
    /// The physical record coordinate could not be formed.
    #[error(transparent)]
    Coordinate(#[from] pale_ports::ControlRecordError),
    /// The exact closed checkpoint profile rejected the association.
    #[error(transparent)]
    Profile(#[from] crate::resident::OperationCheckpointProfileError),
    /// Selected schema ownership could not be resolved.
    #[error(transparent)]
    Schema(#[from] pale_contracts::SchemaError),
    /// Canonical encoding or strict JSON parsing failed.
    #[error(transparent)]
    Canonical(#[from] canon::CanonError),
    /// A bounded record did not decode as its owned wire type.
    #[error(transparent)]
    Decode(#[from] serde_json::Error),
    /// The selected finite membership input was invalid.
    #[error(transparent)]
    MembershipInput(#[from] pale_ports::AccumulatorMembershipInputError),
    /// The native snapshot, membership proof or its budget failed.
    #[error(transparent)]
    Membership(#[from] OperationAccumulatorPreparationError),
    /// Physical custody was unavailable, conflicting or incomplete.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// The independent anchor owner could not answer reliably.
    #[error(transparent)]
    Anchor(#[from] AntiRollbackAnchorError),
}
