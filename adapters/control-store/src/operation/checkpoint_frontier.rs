//! Deployment-specific competing checkpoint classification.

use crate::error::{sqlite, sqlite_integer};
use pale_ports::ControlStoreError;
use rusqlite::{params, Connection};

/// Strictly newer current physical frontier closure only. Same-position
/// disagreement and rollback are not ordinary writer conflicts. A stale
/// candidate's old mutable nodes are not compared after a complete winner.
pub(super) fn current_frontier_is_complete(
    connection: &Connection,
    predecessor: &pale_ports::ControlCheckpointHead,
    profile_ref: &pale_core::records::ResidentRecordRefV1,
    profile_sha256: &pale_core::records::CheckpointProfileDigest,
) -> Result<bool, ControlStoreError> {
    let newer = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pale_deployment_operation_checkpoint_head h \
         JOIN pale_deployment_operation_accumulator_frontier a ON a.checkpoint_domain_id=h.checkpoint_domain_id AND a.domain_epoch=h.domain_epoch \
         AND a.control_checkpoint_ref=h.control_checkpoint_ref AND a.control_checkpoint_sha256=h.control_checkpoint_sha256 \
         JOIN pale_deployment_control_record r ON r.record_ref=a.control_checkpoint_ref AND r.record_sha256=a.control_checkpoint_sha256 \
         WHERE h.checkpoint_domain_id=?1 \
         AND (h.domain_epoch>?2 OR (h.domain_epoch=?2 AND h.domain_sequence>?5 AND a.checkpoint_profile_ref=?3 AND a.checkpoint_profile_sha256=?4)) \
         AND (h.control_checkpoint_ref<>?6 OR h.control_checkpoint_sha256<>?7) \
         AND length(a.state_root_sha256)=64 AND a.state_root_sha256 NOT GLOB '*[^0-9a-f]*' \
         AND length(a.unresolved_root_sha256)=64 AND a.unresolved_root_sha256 NOT GLOB '*[^0-9a-f]*')",
        params![predecessor.checkpoint_domain_id().as_str(), sqlite_integer(predecessor.domain_epoch().get())?,
            profile_ref.as_str(), profile_sha256.as_str(), sqlite_integer(predecessor.domain_sequence().get())?,
            predecessor.coordinate().record_ref().as_str(), predecessor.coordinate().record_sha256().as_str()], |row| row.get::<_, bool>(0)
    ).map_err(|error| sqlite("compare current accumulator frontier closure", error))?;
    Ok(newer
        && super::checkpoint_frontier_is_complete(
            connection,
            predecessor.checkpoint_domain_id().as_str(),
        )?)
}

/// A competing initial writer may have won at sequence one. This is not
/// cross-epoch recovery: exact domain epoch and immutable profile must agree.
/// Historical classification deliberately does not compare old mutable nodes.
pub(super) fn initial_frontier_is_complete(
    connection: &Connection,
    initial: &pale_ports::ControlCheckpointHead,
    profile_ref: &pale_core::records::ResidentRecordRefV1,
    profile_sha256: &pale_core::records::CheckpointProfileDigest,
) -> Result<bool, ControlStoreError> {
    let complete = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pale_deployment_operation_checkpoint_head h \
         JOIN pale_deployment_operation_accumulator_frontier a ON a.checkpoint_domain_id=h.checkpoint_domain_id AND a.domain_epoch=h.domain_epoch \
         AND a.control_checkpoint_ref=h.control_checkpoint_ref AND a.control_checkpoint_sha256=h.control_checkpoint_sha256 \
         JOIN pale_deployment_control_record r ON r.record_ref=a.control_checkpoint_ref AND r.record_sha256=a.control_checkpoint_sha256 \
         WHERE h.checkpoint_domain_id=?1 AND h.domain_epoch=?2 AND h.domain_sequence>=1 \
         AND a.checkpoint_profile_ref=?3 AND a.checkpoint_profile_sha256=?4 \
         AND length(a.state_root_sha256)=64 AND a.state_root_sha256 NOT GLOB '*[^0-9a-f]*' \
         AND length(a.unresolved_root_sha256)=64 AND a.unresolved_root_sha256 NOT GLOB '*[^0-9a-f]*')",
        params![initial.checkpoint_domain_id().as_str(), sqlite_integer(initial.domain_epoch().get())?, profile_ref.as_str(), profile_sha256.as_str()],
        |row| row.get::<_, bool>(0),
    ).map_err(|error| sqlite("compare current initial-operation frontier closure", error))?;
    Ok(complete
        && super::checkpoint_frontier_is_complete(
            connection,
            initial.checkpoint_domain_id().as_str(),
        )?)
}
