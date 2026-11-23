//! Private logical leaves for remote history and its exact protocol evidence.

use super::*;
use pale_ports::{ControlRecordCoordinate, OperationA2aEvidence};

pub(super) const A2A_VALUE_DOMAIN: &str = "PALE_A2A_CUSTODY_LEAF_V1";

/// Exact key and authenticated value; metadata is never inferred from raw digest.
pub(in crate::resident) struct A2aCustodyLeaf {
    pub(in crate::resident) key: ChangedControlLeafKeyV1,
    pub(in crate::resident) hashed_key: ArtifactDigest,
    pub(in crate::resident) value: ArtifactDigest,
}

pub(in crate::resident) fn a2a_record_leaf(
    store: &AuthorityStoreCoordinate,
    checkpoint: &ControlStateCheckpointV1,
    class: &str,
    logical: &str,
    coordinate: &ControlRecordCoordinate,
) -> Result<A2aCustodyLeaf, OperationLeafProjectionError> {
    leaf(
        store,
        checkpoint,
        class,
        logical,
        &(coordinate.record_ref(), coordinate.record_sha256()),
    )
}

pub(in crate::resident) fn a2a_evidence_leaf(
    store: &AuthorityStoreCoordinate,
    checkpoint: &ControlStateCheckpointV1,
    evidence: &OperationA2aEvidence,
) -> Result<A2aCustodyLeaf, OperationLeafProjectionError> {
    leaf(
        store,
        checkpoint,
        "a2a-evidence",
        evidence.coordinate().record_ref().as_str(),
        &(evidence.coordinate(), evidence.sse()),
    )
}

/// Project the logical key for one A2A evidence coordinate.
///
/// A signed checkpoint can use this key to show that its transaction changed
/// the A2A evidence position. The key does not prove the evidence bytes or the
/// leaf value; those require the exact retained row and a membership proof.
pub(in crate::resident) fn a2a_evidence_key(
    store: &AuthorityStoreCoordinate,
    checkpoint: &ControlStateCheckpointV1,
    coordinate: &ControlRecordCoordinate,
) -> Result<ChangedControlLeafKeyV1, OperationLeafProjectionError> {
    if checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
    {
        return Err(OperationLeafProjectionError::Store);
    }
    control_key(
        store,
        checkpoint.checkpoint_domain_id(),
        *checkpoint.domain_epoch(),
        "a2a-evidence",
        coordinate.record_ref().as_str(),
    )
}

fn leaf(
    store: &AuthorityStoreCoordinate,
    checkpoint: &ControlStateCheckpointV1,
    class: &str,
    logical: &str,
    value: &impl Serialize,
) -> Result<A2aCustodyLeaf, OperationLeafProjectionError> {
    if checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
    {
        return Err(OperationLeafProjectionError::Store);
    }
    let key = control_key(
        store,
        checkpoint.checkpoint_domain_id(),
        *checkpoint.domain_epoch(),
        class,
        logical,
    )?;
    Ok(A2aCustodyLeaf {
        hashed_key: canonical_hash(&(KEY_DOMAIN, &key))?,
        value: canonical_hash(&(A2A_VALUE_DOMAIN, &key, value))?,
        key,
    })
}
