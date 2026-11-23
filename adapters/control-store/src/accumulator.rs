//! Exact physical custody of producer-verified accumulator batches.
//! No root construction or signer authority is inferred from stored bytes.

use pale_ports::{
    AccumulatorMutation, AccumulatorPosition, ControlStoreError, ControlStoreErrorKind,
    OperationAccumulatorBatch,
};
use rusqlite::{params, types::ValueRef, Connection};

use crate::error::{sqlite, sqlite_integer};

mod codec;
pub(crate) mod preparation;
mod storage;
pub(crate) use storage::Store;

use codec::valid_bytes;

fn invalid() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}

/// Prove absence of indexed domain evidence, not just one epoch/profile slice.
/// The caller separately verifies this connection's exact store incarnation.
/// This does not interpret unreferenced record bodies as admitted domain state.
pub(crate) fn domain_is_absent(
    connection: &Connection,
    storage: Store,
    domain: &pale_core::records::ControlCheckpointDomainIdV1,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            storage.statements().domain_absent,
            [domain.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| sqlite("compare whole initial operation domain absence", error))
}

/// Check the concrete physical codec before any database operation.
pub(crate) fn validate(batch: &OperationAccumulatorBatch) -> Result<(), ControlStoreError> {
    for mutation in batch
        .state()
        .mutations()
        .iter()
        .chain(batch.unresolved().mutations())
    {
        for bytes in mutation
            .predecessor()
            .into_iter()
            .chain(mutation.successor())
        {
            if !valid_bytes(mutation.position(), bytes) {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

pub(crate) fn position_parts(position: AccumulatorPosition) -> (&'static str, [u8; 33]) {
    let mut key = [0; 33];
    match position {
        AccumulatorPosition::Branch { height, path } => {
            key[0] = height;
            key[1..].copy_from_slice(&path);
            ("branch", key)
        }
        AccumulatorPosition::Leaf(path) => {
            key[1..].copy_from_slice(&path);
            ("leaf", key)
        }
    }
}

fn position_matches(
    connection: &Connection,
    storage: Store,
    batch: &OperationAccumulatorBatch,
    tree: &str,
    mutation: &AccumulatorMutation,
    successor: bool,
) -> Result<bool, ControlStoreError> {
    let binding = batch.binding();
    let (kind, key) = position_parts(mutation.position());
    let mut statement = connection
        .prepare(storage.statements().position)
        .map_err(|error| sqlite("prepare accumulator position comparison", error))?;
    let mut rows = statement
        .query(params![
            binding.successor().checkpoint_domain_id().as_str(),
            sqlite_integer(binding.successor().domain_epoch().get())?,
            binding.profile_ref().as_str(),
            binding.profile_sha256().as_str(),
            tree,
            kind,
            key.as_slice()
        ])
        .map_err(|error| sqlite("read accumulator position", error))?;
    let expected = if successor {
        mutation.successor()
    } else {
        mutation.predecessor()
    };
    let Some(row) = rows
        .next()
        .map_err(|error| sqlite("read accumulator position row", error))?
    else {
        return Ok(expected.is_none());
    };
    match row
        .get_ref(0)
        .map_err(|error| sqlite("borrow accumulator position bytes", error))?
    {
        ValueRef::Blob(bytes) if valid_bytes(mutation.position(), bytes) => {
            Ok(expected == Some(bytes))
        }
        _ => Err(invalid()),
    }
}

/// Compare both roots and all touched positions; untouched trees remain explicit.
pub(crate) fn matches(
    connection: &Connection,
    storage: Store,
    batch: &OperationAccumulatorBatch,
    successor: bool,
) -> Result<bool, ControlStoreError> {
    if !frontier_matches(connection, storage, batch, successor)? {
        return Ok(false);
    }
    for (tree, mutations) in [
        ("state", batch.state().mutations()),
        ("unresolved", batch.unresolved().mutations()),
    ] {
        for mutation in mutations {
            if !position_matches(connection, storage, batch, tree, mutation, successor)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn frontier_matches(
    connection: &Connection,
    storage: Store,
    batch: &OperationAccumulatorBatch,
    successor: bool,
) -> Result<bool, ControlStoreError> {
    let binding = batch.binding();
    let (head, state, unresolved) = match (successor, binding.phase()) {
        (true, _) => (
            binding.successor(),
            batch.state().successor_root(),
            batch.unresolved().successor_root(),
        ),
        (false, pale_ports::AccumulatorCheckpointPhase::Initial) => {
            return domain_is_absent(
                connection,
                storage,
                binding.successor().checkpoint_domain_id(),
            );
        }
        (false, pale_ports::AccumulatorCheckpointPhase::Successor(predecessor)) => (
            predecessor,
            batch.state().predecessor_root(),
            batch.unresolved().predecessor_root(),
        ),
    };
    selected_frontier_matches(
        connection,
        storage,
        binding.profile_ref(),
        binding.profile_sha256(),
        head,
        state,
        unresolved,
    )
}

fn selected_frontier_matches(
    connection: &Connection,
    storage: Store,
    profile_ref: &pale_core::records::ResidentRecordRefV1,
    profile_sha256: &pale_core::records::CheckpointProfileDigest,
    head: &pale_ports::ControlCheckpointHead,
    state: &pale_core::records::ArtifactDigest,
    unresolved: &pale_core::records::ArtifactDigest,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            storage.statements().frontier_matches,
            params![
                head.checkpoint_domain_id().as_str(),
                sqlite_integer(head.domain_epoch().get())?,
                profile_ref.as_str(),
                profile_sha256.as_str(),
                head.coordinate().record_ref().as_str(),
                head.coordinate().record_sha256().as_str(),
                state.as_str(),
                unresolved.as_str()
            ],
            |row| row.get(0),
        )
        .map_err(|error| sqlite("compare accumulator frontier", error))
}

/// Apply only touched rows inside the caller's existing immediate transaction.
/// The caller already compared complete predecessors and appends the checkpoint
/// record before the frontier's foreign key can refer to it.
pub(crate) fn apply(
    connection: &Connection,
    storage: Store,
    batch: &OperationAccumulatorBatch,
) -> Result<(), ControlStoreError> {
    let binding = batch.binding();
    for (tree, mutations) in [
        ("state", batch.state().mutations()),
        ("unresolved", batch.unresolved().mutations()),
    ] {
        for mutation in mutations {
            let (kind, key) = position_parts(mutation.position());
            if let Some(bytes) = mutation.successor() {
                connection
                    .execute(
                        storage.statements().upsert_position,
                        params![
                            binding.successor().checkpoint_domain_id().as_str(),
                            sqlite_integer(binding.successor().domain_epoch().get())?,
                            binding.profile_ref().as_str(),
                            binding.profile_sha256().as_str(),
                            tree,
                            kind,
                            key.as_slice(),
                            bytes
                        ],
                    )
                    .map_err(|error| sqlite("write accumulator successor position", error))?;
            } else {
                connection
                    .execute(
                        storage.statements().remove_position,
                        params![
                            binding.successor().checkpoint_domain_id().as_str(),
                            sqlite_integer(binding.successor().domain_epoch().get())?,
                            binding.profile_ref().as_str(),
                            binding.profile_sha256().as_str(),
                            tree,
                            kind,
                            key.as_slice()
                        ],
                    )
                    .map_err(|error| sqlite("remove accumulator predecessor position", error))?;
            }
        }
    }
    let successor = binding.successor();
    let changed = match binding.phase() {
        pale_ports::AccumulatorCheckpointPhase::Initial => connection.execute(
            storage.statements().insert_frontier,
            params![
                successor.checkpoint_domain_id().as_str(),
                sqlite_integer(successor.domain_epoch().get())?,
                binding.profile_ref().as_str(),
                binding.profile_sha256().as_str(),
                successor.coordinate().record_ref().as_str(),
                successor.coordinate().record_sha256().as_str(),
                batch.state().successor_root().as_str(),
                batch.unresolved().successor_root().as_str()
            ],
        ),
        pale_ports::AccumulatorCheckpointPhase::Successor(expected) => connection.execute(
            storage.statements().advance_frontier,
            params![
                successor.coordinate().record_ref().as_str(),
                successor.coordinate().record_sha256().as_str(),
                batch.state().successor_root().as_str(),
                batch.unresolved().successor_root().as_str(),
                expected.checkpoint_domain_id().as_str(),
                sqlite_integer(expected.domain_epoch().get())?,
                binding.profile_ref().as_str(),
                binding.profile_sha256().as_str(),
                expected.coordinate().record_ref().as_str(),
                expected.coordinate().record_sha256().as_str(),
                batch.state().predecessor_root().as_str(),
                batch.unresolved().predecessor_root().as_str()
            ],
        ),
    }
    .map_err(|error| sqlite("advance accumulator frontier", error))?;
    if changed != 1 {
        return Err(invalid());
    }
    Ok(())
}
