//! Source-supported checkpoint content and logical leaves for admitted operations.
//!
//! The original request's existing identity owner fixes the conflict domain.
//! Its selected profile admits initial insertion and retained observation
//! replacement; neither content nor leaf projection proves signing authority.
//! Recovery indexes that name the successor checkpoint are deliberately absent:
//! including them would make the checkpoint depend on its own digest.

use pale_core::{
    canon,
    records::{
        ArtifactDigest, ChangedControlLeafKeyV1, ControlCheckpointDomainIdV1, ControlDomainEpochV1,
        ControlIncarnationIdV1, ControlLogicalKeyV1, ControlRecordClassV1,
        ControlStateCheckpointV1, ControlStoreIdV1, InvalidResidentRecordV1, OperationHandleIdV1,
        OperationObservationDigest, OperationObservationRevisionV1, OperationObservationStateV1,
        OperationObservationV1, ResidentRecordRefV1, SchemaId,
    },
};
use pale_ports::{AuthorityStoreCoordinate, ControlCheckpointHead, OperationAccumulatorBatch};
use serde::Serialize;

use super::operation_observation_custody::PreparedOperationObservationAppend;

mod a2a;
mod content;
pub(super) use a2a::{a2a_evidence_key, a2a_evidence_leaf, a2a_record_leaf, A2aCustodyLeaf};
mod initial;
mod session_basis;
pub use content::{
    OperationCheckpointProfileError, OperationCheckpointSelection,
    SupportedOperationCheckpointProfile,
};
pub(super) use initial::{
    initial_operation_leaves, operation_identity_leaves, InitialOperationLeafError,
    InitialOperationLeaves,
};
pub(super) use session_basis::operation_session_basis_leaf;

const SCOPE_DOMAIN: &str = "PALE_OPERATION_CHECKPOINT_SCOPE_V1";
pub(super) const KEY_DOMAIN: &str = "PALE_CONTROL_LEAF_KEY_V1";
const VALUE_DOMAIN: &str = "PALE_OPERATION_OBSERVATION_LEAF_V1";
const REQUEST_VALUE_DOMAIN: &str = "PALE_OPERATION_REQUEST_ASSOCIATION_LEAF_V1";
const HANDLE_VALUE_DOMAIN: &str = "PALE_OPERATION_HANDLE_LEAF_V1";
const INITIAL_DOMAIN_EPOCH: u64 = 1;
pub(super) const CONTROL_SCHEMA_VERSION: u32 = 1;

/// One logical replacement; no terminal operation is interpreted as deletion.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct OperationObservationLeafChange {
    pub(super) key: ChangedControlLeafKeyV1,
    pub(super) key_sha256: ArtifactDigest,
    pub(super) predecessor_sha256: ArtifactDigest,
    pub(super) successor_sha256: ArtifactDigest,
}

#[derive(Serialize)]
struct OperationScope<'a> {
    domain: &'static str,
    control_store_id: &'a ControlStoreIdV1,
    control_incarnation_id: &'a ControlIncarnationIdV1,
    checkpoint_domain_id: &'a ControlCheckpointDomainIdV1,
    domain_epoch: ControlDomainEpochV1,
}

#[derive(Serialize)]
struct ObservationLeaf<'a> {
    domain: &'static str,
    key: &'a ChangedControlLeafKeyV1,
    schema_id: &'a SchemaId,
    record_ref: &'a ResidentRecordRefV1,
    record_sha256: &'a OperationObservationDigest,
    revision: &'a OperationObservationRevisionV1,
    state: &'a OperationObservationStateV1,
}

/// Reject a mismatched store or an unencodable canonical leaf.
#[derive(Debug, thiserror::Error)]
pub(super) enum OperationLeafProjectionError {
    #[error("operation observation and checkpoint belong to different stores")]
    Store,
    #[error(transparent)]
    Identity(#[from] InvalidResidentRecordV1),
    #[error(transparent)]
    Canonicalize(#[from] canon::CanonError),
}

/// Project the replacement of one retained observation head.
///
/// The stable key uses store incarnation, selected conflict domain and epoch,
/// then the operation handle identity. Observation revisions, record references,
/// signing keys and current states change the value, not the key. The existing
/// revision and state serializers retain their exact numeric and tagged meanings.
///
/// An observation does not itself mutate reconciliation custody. Its terminal or
/// unknown state must not be used to populate or erase the separate unresolved-
/// custody tree; that projection belongs to the admitted reconciliation transition.
pub(super) fn operation_observation_change(
    append: &PreparedOperationObservationAppend<'_>,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<OperationObservationLeafChange, OperationLeafProjectionError> {
    let store = append.store();
    if checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
    {
        return Err(OperationLeafProjectionError::Store);
    }
    let key = operation_key(store, checkpoint, append.handle().1.operation_handle_id())?;
    let key_sha256 = canonical_hash(&(KEY_DOMAIN, &key))?;
    let (predecessor_ref, predecessor) = append.predecessor();
    let (successor_ref, successor) = append.successor();
    Ok(OperationObservationLeafChange {
        predecessor_sha256: observation_hash(&key, predecessor_ref, predecessor)?,
        successor_sha256: observation_hash(&key, successor_ref, successor)?,
        key,
        key_sha256,
    })
}

fn operation_key(
    store: &AuthorityStoreCoordinate,
    checkpoint: &ControlStateCheckpointV1,
    handle: &OperationHandleIdV1,
) -> Result<ChangedControlLeafKeyV1, OperationLeafProjectionError> {
    control_key(
        store,
        checkpoint.checkpoint_domain_id(),
        *checkpoint.domain_epoch(),
        "operation-observation",
        handle.as_str(),
    )
}

/// Project one admitted selected observation using the same stored leaf as start/replacement.
/// This does not authenticate the checkpoint, the current index, or the native proof.
pub(super) fn operation_observation_membership(
    store: &AuthorityStoreCoordinate,
    checkpoint: &ControlStateCheckpointV1,
    reference: &ResidentRecordRefV1,
    observation: &OperationObservationV1,
) -> Result<pale_ports::AccumulatorMembershipLeaf, OperationLeafProjectionError> {
    if checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
    {
        return Err(OperationLeafProjectionError::Store);
    }
    let key = operation_key(store, checkpoint, observation.operation_handle_id())?;
    pale_ports::AccumulatorMembershipLeaf::new(
        canonical_hash(&(KEY_DOMAIN, &key))?,
        observation_hash(&key, reference, observation)?,
    )
    .map_err(|_| {
        OperationLeafProjectionError::Identity(InvalidResidentRecordV1::NonCanonical {
            relation: "present operation observation leaf",
        })
    })
}

fn control_key(
    store: &AuthorityStoreCoordinate,
    domain: &ControlCheckpointDomainIdV1,
    epoch: ControlDomainEpochV1,
    record_class: &str,
    logical_key: &str,
) -> Result<ChangedControlLeafKeyV1, OperationLeafProjectionError> {
    Ok(ChangedControlLeafKeyV1 {
        record_class: ControlRecordClassV1::parse(record_class)?,
        scope_key_sha256: canonical_hash(&OperationScope {
            domain: SCOPE_DOMAIN,
            control_store_id: store.store_id(),
            control_incarnation_id: store.incarnation_id(),
            checkpoint_domain_id: domain,
            domain_epoch: epoch,
        })?,
        logical_key: ControlLogicalKeyV1::parse(logical_key)?,
    })
}

fn observation_hash(
    key: &ChangedControlLeafKeyV1,
    record_ref: &ResidentRecordRefV1,
    observation: &OperationObservationV1,
) -> Result<ArtifactDigest, canon::CanonError> {
    canonical_hash(&ObservationLeaf {
        domain: VALUE_DOMAIN,
        key,
        schema_id: observation.schema_id(),
        record_ref,
        record_sha256: observation.canonical_operation_observation_sha256(),
        revision: observation.revision(),
        state: observation.state(),
    })
}

pub(super) fn canonical_hash(value: &impl Serialize) -> Result<ArtifactDigest, canon::CanonError> {
    canon::to_canonical_string(value).map(|bytes| ArtifactDigest::hash_bytes(bytes.as_bytes()))
}

/// Match a physical batch to the complete already-admitted checkpoint pair.
///
/// Port heads carry record identity but not profile or roots. Comparing only
/// those heads would allow a physical batch to retain different tree roots.
/// This check preserves every independent coordinate without decoding record
/// JSON or recomputing the accumulator. Construction proof remains required.
pub(super) fn verify_checkpoint_batch(
    store: &AuthorityStoreCoordinate,
    predecessor: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    successor: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    batch: &OperationAccumulatorBatch,
) -> Result<(), InvalidResidentRecordV1> {
    let binding = batch.binding();
    check_batch(binding.store() == store, "operation accumulator store")?;
    for checkpoint in [predecessor.1, successor.1] {
        check_batch(
            checkpoint.control_store_id() == store.store_id()
                && checkpoint.control_incarnation_id() == store.incarnation_id(),
            "operation accumulator checkpoint store",
        )?;
        check_batch(
            checkpoint.checkpoint_profile_ref() == binding.profile_ref()
                && checkpoint.checkpoint_profile_sha256() == binding.profile_sha256(),
            "operation accumulator checkpoint profile",
        )?;
    }
    check_batch(
        matches!(binding.phase(), pale_ports::AccumulatorCheckpointPhase::Successor(head)
            if checkpoint_head_matches(head, predecessor)),
        "operation accumulator predecessor",
    )?;
    check_batch(
        checkpoint_head_matches(binding.successor(), successor),
        "operation accumulator successor",
    )?;
    check_batch(
        batch.state().predecessor_root() == predecessor.1.state_root_sha256()
            && batch.state().successor_root() == successor.1.state_root_sha256(),
        "operation accumulator state roots",
    )?;
    check_batch(
        batch.unresolved().predecessor_root() == predecessor.1.unresolved_custody_root_sha256()
            && batch.unresolved().successor_root() == successor.1.unresolved_custody_root_sha256(),
        "operation accumulator unresolved-custody roots",
    )
}

fn checkpoint_head_matches(
    head: &ControlCheckpointHead,
    (reference, checkpoint): (&ResidentRecordRefV1, &ControlStateCheckpointV1),
) -> bool {
    head.checkpoint_domain_id() == checkpoint.checkpoint_domain_id()
        && head.domain_epoch() == *checkpoint.domain_epoch()
        && head.domain_sequence() == *checkpoint.domain_sequence()
        && head.committed_transaction_sequence() == *checkpoint.committed_transaction_sequence()
        && head.coordinate().record_ref() == reference
        && head.coordinate().record_sha256().as_str()
            == checkpoint.canonical_control_checkpoint_sha256().as_str()
}

fn check_batch(valid: bool, relation: &'static str) -> Result<(), InvalidResidentRecordV1> {
    valid
        .then_some(())
        .ok_or(InvalidResidentRecordV1::NonCanonical { relation })
}
