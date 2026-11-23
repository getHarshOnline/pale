//! Derived binding association and one bounded operation-prefix seek.

use super::*;
use crate::store::transaction::{read as read_transaction, record_stamp};
use crate::ControlStoreKind;
use pale_core::records::{
    ControlTransactionSequenceV1, A2A_TASK_BINDING_V1_SCHEMA, A2A_TASK_OBSERVATION_V1_SCHEMA,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA, OPERATION_HANDLE_V1_SCHEMA,
};
use pale_ports::{
    OperationA2aBindingCursor, OperationA2aBindingScope, OperationA2aBindingSource,
    OperationA2aBindings, MAX_OPERATION_A2A_BINDINGS, MAX_OPERATION_A2A_BINDING_BYTES,
};

struct Target<'a> {
    operation_key: &'a OperationCustodyIndexKey,
    binding_key: &'a OperationCustodyIndexKey,
    binding: &'a ControlRecordCoordinate,
    handle: &'a ControlRecordCoordinate,
    fresh: bool,
}

fn target(candidate: &OperationCustodyCandidate) -> Result<Option<Target<'_>>, ControlStoreError> {
    let Some((operation_key, binding_key, binding)) =
        candidate.indexes().iter().find_map(|i| match i {
            OperationCustodyIndexMutation::A2aTaskBinding {
                operation_key,
                key,
                successor,
                ..
            } => Some((operation_key, key, successor)),
            _ => None,
        })
    else {
        return Ok(None);
    };
    let handle = candidate
        .a2a_retained_records()
        .iter()
        .find(|record| record.schema_id().as_str() == OPERATION_HANDLE_V1_SCHEMA)
        .ok_or_else(recovery)?
        .coordinate();
    Ok(Some(Target {
        operation_key,
        binding_key,
        binding,
        handle,
        fresh: candidate.kind() == pale_ports::OperationCustodyTransitionKind::BindA2aTask,
    }))
}

/// Unlike scope is never repaired; follow-up observations retain the original row.
pub(super) fn retained_match(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    let Some(target) = target(candidate)? else {
        return Ok(true);
    };
    if !scope_matches(connection, target.operation_key, target.handle)? {
        return Ok(false);
    }
    Ok(target.fresh || relation_matches(connection, &target)?)
}

fn scope_matches(
    connection: &Connection,
    key: &OperationCustodyIndexKey,
    handle: &ControlRecordCoordinate,
) -> Result<bool, ControlStoreError> {
    Ok(
        read_index_coordinate(connection, IndexRole::OperationHandle, key)?.as_ref()
            == Some(handle)
            && record_is_complete(connection, handle)?
            && schema_matches(connection, handle, OPERATION_HANDLE_V1_SCHEMA)?,
    )
}

fn relation_matches(
    connection: &Connection,
    target: &Target<'_>,
) -> Result<bool, ControlStoreError> {
    let stamp = record_stamp(
        connection,
        ControlStoreKind::DeploymentControl,
        target.binding,
    )?;
    let Some(stamp) = stamp else {
        return Ok(false);
    };
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pale_deployment_operation_a2a_binding WHERE operation_key=?1 AND binding_key=?2 AND record_ref=?3 AND record_sha256=?4 AND inserted_transaction_sequence=?5)",
        params![target.operation_key.as_str(), target.binding_key.as_str(),
            target.binding.record_ref().as_str(), target.binding.record_sha256().as_str(), sqlite_integer(stamp.get())?],
        |row| row.get(0)).map_err(|e| sqlite("compare retained task discovery relation", e))
}

/// Detect conflicting aliases too, not only a fully matching relation row.
pub(super) fn material_present(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    let Some(target) = target(candidate)?.filter(|t| t.fresh) else {
        return Ok(false);
    };
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pale_deployment_operation_a2a_binding WHERE binding_key=?1 OR record_ref=?2)",
        params![target.binding_key.as_str(), target.binding.record_ref().as_str()], |row| row.get(0))
        .map_err(|e| sqlite("find task discovery candidate material", e))
}

pub(super) fn append(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<(), ControlStoreError> {
    let Some(target) = target(candidate)?.filter(|t| t.fresh) else {
        return Ok(());
    };
    connection.execute(
        "INSERT INTO pale_deployment_operation_a2a_binding (operation_key,binding_key,record_ref,record_sha256,inserted_transaction_sequence) VALUES (?1,?2,?3,?4,?5)",
        params![target.operation_key.as_str(), target.binding_key.as_str(), target.binding.record_ref().as_str(),
            target.binding.record_sha256().as_str(), sqlite_integer(candidate.accumulator().binding().transaction().successor().get())?])
        .map_err(|e| sqlite("append task discovery relation", e))?;
    Ok(())
}

/// Close this physical transaction over exactly its selected fresh relation.
/// The T-leading index also bounds the zero-row test on other operation atoms.
pub(super) fn members_match(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    let target = target(candidate)?.filter(|t| t.fresh);
    let mut statement = connection.prepare(
        "SELECT operation_key,binding_key,record_ref,record_sha256 FROM pale_deployment_operation_a2a_binding INDEXED BY pale_deployment_operation_a2a_binding_transaction WHERE inserted_transaction_sequence=?1 LIMIT 2")
        .map_err(|e| sqlite("prepare task relation transaction closure", e))?;
    let mut rows = statement
        .query([sqlite_integer(
            candidate
                .accumulator()
                .binding()
                .transaction()
                .successor()
                .get(),
        )?])
        .map_err(|e| sqlite("read task relation transaction closure", e))?;
    let Some(row) = rows
        .next()
        .map_err(|e| sqlite("read task relation member", e))?
    else {
        return Ok(target.is_none());
    };
    let Some(target) = target else {
        return Ok(false);
    };
    let operation_key = decode_key(
        row.get_ref(0)
            .map_err(|e| sqlite("read operation key", e))?,
    )?;
    let binding_key = decode_key(row.get_ref(1).map_err(|e| sqlite("read binding key", e))?)?;
    let coordinate = decode_record_coordinate(
        row.get_ref(2)
            .map_err(|e| sqlite("read binding reference", e))?,
        row.get_ref(3)
            .map_err(|e| sqlite("read binding digest", e))?,
    )?;
    if &operation_key != target.operation_key
        || &binding_key != target.binding_key
        || &coordinate != target.binding
    {
        return Ok(false);
    }
    Ok(rows
        .next()
        .map_err(|e| sqlite("check extra task relation", e))?
        .is_none())
}

pub(super) fn historical_match(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    let Some(target) = target(candidate)? else {
        return Ok(true);
    };
    Ok(
        scope_matches(connection, target.operation_key, target.handle)?
            && relation_matches(connection, &target)?,
    )
}

/// Distinguish one complete competing Bind from orphaned relation material.
/// Both indexed seeks inspect at most two rows; there is no history scan.
pub(super) fn transaction_is_complete(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    let transaction = sqlite_integer(
        candidate
            .accumulator()
            .binding()
            .transaction()
            .successor()
            .get(),
    )?;
    let bindings = {
        let mut statement = connection.prepare(
            "SELECT record_ref,record_sha256 FROM pale_deployment_control_record INDEXED BY pale_deployment_control_record_source_order WHERE schema_id=?1 AND inserted_transaction_sequence=?2 LIMIT 2"
        ).map_err(|e| sqlite("prepare competing binding closure", e))?;
        let mut rows = statement
            .query(params![A2A_TASK_BINDING_V1_SCHEMA, transaction])
            .map_err(|e| sqlite("read competing binding closure", e))?;
        let mut bindings = Vec::with_capacity(2);
        while let Some(row) = rows
            .next()
            .map_err(|e| sqlite("read competing binding", e))?
        {
            bindings.push(decode_record_coordinate(
                row.get_ref(0)
                    .map_err(|e| sqlite("read competing binding reference", e))?,
                row.get_ref(1)
                    .map_err(|e| sqlite("read competing binding digest", e))?,
            )?);
        }
        bindings
    };
    let mut statement = connection.prepare(
        "SELECT binding_key,inserted_transaction_sequence,record_ref,record_sha256,operation_key FROM pale_deployment_operation_a2a_binding INDEXED BY pale_deployment_operation_a2a_binding_transaction WHERE inserted_transaction_sequence=?1 LIMIT 2"
    ).map_err(|e| sqlite("prepare competing task relation closure", e))?;
    let mut rows = statement
        .query([transaction])
        .map_err(|e| sqlite("read competing task relation closure", e))?;
    let Some(row) = rows
        .next()
        .map_err(|e| sqlite("read competing task relation", e))?
    else {
        return Ok(bindings.is_empty());
    };
    let source = decode_row(row)?;
    if bindings.as_slice() != [source.coordinate.clone()] {
        return Ok(false);
    }
    let operation_key = decode_key(
        row.get_ref(4)
            .map_err(|e| sqlite("read competing operation key", e))?,
    )?;
    let Some(handle) =
        read_index_coordinate(connection, IndexRole::OperationHandle, &operation_key)?
    else {
        return Ok(false);
    };
    if !scope_matches(connection, &operation_key, &handle)?
        || record_stamp(connection, ControlStoreKind::DeploymentControl, &handle)?
            .is_none_or(|stamp| stamp.get() > source.transaction.get())
    {
        return Ok(false);
    }
    validate_source(connection, &source)?;
    Ok(rows
        .next()
        .map_err(|e| sqlite("check competing task relation cardinality", e))?
        .is_none())
}

pub(super) fn read(
    custody: &mut SqliteOperationCustody<'_>,
    store: &AuthorityStoreCoordinate,
    scope: &OperationA2aBindingScope,
    cursor: Option<&OperationA2aBindingCursor>,
    maximum_bindings: usize,
    maximum_body_bytes: usize,
) -> Result<OperationA2aBindings, ControlStoreError> {
    if maximum_bindings == 0
        || maximum_bindings > MAX_OPERATION_A2A_BINDINGS
        || maximum_body_bytes == 0
        || maximum_body_bytes > MAX_OPERATION_A2A_BINDING_BYTES
    {
        return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
    }
    require_coordinate(&custody.coordinate, store)?;
    if cursor.is_some_and(|c| c.store() != store || c.scope() != scope) {
        return Err(recovery());
    }
    let snapshot = custody
        .connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|e| sqlite("begin task discovery snapshot", e))?;
    require_store_binding(&snapshot, store)?;
    if !scope_matches(&snapshot, scope.operation_key(), scope.handle())? {
        return Err(recovery());
    }
    let current = read_transaction(&snapshot, store)?;
    let through = cursor.map_or(current, OperationA2aBindingCursor::through);
    if through.get() > current.get() {
        return Err(recovery());
    }
    if record_stamp(
        &snapshot,
        ControlStoreKind::DeploymentControl,
        scope.handle(),
    )?
    .is_none_or(|stamp| stamp.get() > through.get())
    {
        return Err(recovery());
    }
    if let Some(cursor) = cursor {
        let saved = snapshot.query_row(
            "SELECT binding_key,inserted_transaction_sequence,record_ref,record_sha256 FROM pale_deployment_operation_a2a_binding WHERE operation_key=?1 AND inserted_transaction_sequence=?2 AND record_ref=?3",
            params![scope.operation_key().as_str(), sqlite_integer(cursor.after_transaction().get())?, cursor.after_reference().as_str()],
            |row| Ok(decode_row(row))).optional().map_err(|e| sqlite("read task discovery continuation", e))?
            .transpose()?.ok_or_else(recovery)?;
        validate_source(&snapshot, &saved)?;
    }
    let coordinates = {
        let mut statement = snapshot.prepare(
            "SELECT binding_key,inserted_transaction_sequence,record_ref,record_sha256 FROM pale_deployment_operation_a2a_binding INDEXED BY pale_deployment_operation_a2a_binding_order WHERE operation_key=?1 AND inserted_transaction_sequence<=?2 AND (inserted_transaction_sequence,record_ref)>(?3,?4) ORDER BY inserted_transaction_sequence,record_ref LIMIT ?5")
            .map_err(|e| sqlite("prepare operation task discovery seek", e))?;
        let mut rows = statement
            .query(params![
                scope.operation_key().as_str(),
                sqlite_integer(through.get())?,
                sqlite_integer(cursor.map_or(0, |c| c.after_transaction().get()))?,
                cursor.map_or("", |c| c.after_reference().as_str()),
                sqlite_integer((maximum_bindings + 1) as u64)?
            ])
            .map_err(|e| sqlite("seek operation task bindings", e))?;
        let mut coordinates = Vec::with_capacity(maximum_bindings + 1);
        while let Some(row) = rows
            .next()
            .map_err(|e| sqlite("read task binding position", e))?
        {
            coordinates.push(decode_row(row)?);
        }
        coordinates
    };
    let has_more = coordinates.len() > maximum_bindings;
    let mut remaining = maximum_body_bytes;
    let mut bindings = Vec::with_capacity(maximum_bindings);
    for source in coordinates.into_iter().take(maximum_bindings) {
        validate_source(&snapshot, &source)?;
        let length: i64 = snapshot.query_row(
            "SELECT length(canonical_record_jcs) FROM pale_deployment_control_record WHERE record_ref=?1 AND record_sha256=?2",
            params![source.coordinate.record_ref().as_str(), source.coordinate.record_sha256().as_str()], |row| row.get(0))
            .map_err(|e| sqlite("read binding byte bound", e))?;
        let length = usize::try_from(length)
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(recovery)?;
        if length > remaining {
            return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
        }
        let binding =
            read_record_bounded(&snapshot, &source.coordinate, remaining)?.ok_or_else(recovery)?;
        remaining -= binding.canonical_record_jcs().as_slice().len();
        bindings.push(OperationA2aBindingSource::new(
            source.key,
            source.transaction,
            binding,
        )?);
    }
    let continuation = if has_more {
        let last = bindings.last().ok_or_else(recovery)?;
        Some(OperationA2aBindingCursor::new(
            store.clone(),
            scope.clone(),
            through,
            last.transaction(),
            last.binding().coordinate().record_ref().clone(),
        )?)
    } else {
        None
    };
    snapshot
        .rollback()
        .map_err(|e| sqlite("close task discovery snapshot", e))?;
    Ok(OperationA2aBindings {
        bindings,
        continuation,
    })
}

struct Source {
    key: OperationCustodyIndexKey,
    transaction: ControlTransactionSequenceV1,
    coordinate: ControlRecordCoordinate,
}
fn decode_row(row: &rusqlite::Row<'_>) -> Result<Source, ControlStoreError> {
    let stamp: i64 = row
        .get(1)
        .map_err(|e| sqlite("read task binding transaction", e))?;
    let transaction = u64::try_from(stamp)
        .ok()
        .and_then(|v| ControlTransactionSequenceV1::new(v).ok())
        .ok_or_else(recovery)?;
    Ok(Source {
        key: decode_key(
            row.get_ref(0)
                .map_err(|e| sqlite("read task binding key", e))?,
        )?,
        transaction,
        coordinate: decode_record_coordinate(
            row.get_ref(2)
                .map_err(|e| sqlite("read task binding reference", e))?,
            row.get_ref(3)
                .map_err(|e| sqlite("read task binding digest", e))?,
        )?,
    })
}
fn decode_key(value: ValueRef<'_>) -> Result<OperationCustodyIndexKey, ControlStoreError> {
    let ValueRef::Text(bytes) = value else {
        return Err(recovery());
    };
    if bytes.len() != 64 {
        return Err(recovery());
    }
    OperationCustodyIndexKey::parse(std::str::from_utf8(bytes).map_err(|_| recovery())?)
        .map_err(|_| recovery())
}
fn validate_source(connection: &Connection, source: &Source) -> Result<(), ControlStoreError> {
    if read_index_coordinate(connection, IndexRole::A2aTaskBinding, &source.key)?.as_ref()
        != Some(&source.coordinate)
        || !record_is_complete(connection, &source.coordinate)?
        || record_stamp(
            connection,
            ControlStoreKind::DeploymentControl,
            &source.coordinate,
        )? != Some(source.transaction)
        || !schema_matches(connection, &source.coordinate, A2A_TASK_BINDING_V1_SCHEMA)?
    {
        return Err(recovery());
    }
    let first = read_index_coordinate(connection, IndexRole::FirstA2aTaskObservation, &source.key)?
        .ok_or_else(recovery)?;
    let checkpoint = read_index_coordinate(
        connection,
        IndexRole::A2aTaskObservationCheckpoint,
        &OperationCustodyIndexKey::for_observation(&first),
    )?
    .ok_or_else(recovery)?;
    for (coordinate, schema) in [
        (&first, A2A_TASK_OBSERVATION_V1_SCHEMA),
        (&checkpoint, CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
    ] {
        if !record_is_complete(connection, coordinate)?
            || !schema_matches(connection, coordinate, schema)?
            || record_stamp(connection, ControlStoreKind::DeploymentControl, coordinate)?
                != Some(source.transaction)
        {
            return Err(recovery());
        }
    }
    Ok(())
}
fn schema_matches(
    connection: &Connection,
    coordinate: &ControlRecordCoordinate,
    schema: &str,
) -> Result<bool, ControlStoreError> {
    connection.query_row("SELECT EXISTS(SELECT 1 FROM pale_deployment_control_record WHERE record_ref=?1 AND record_sha256=?2 AND schema_id=?3)",
        params![coordinate.record_ref().as_str(), coordinate.record_sha256().as_str(), schema], |row| row.get(0))
        .map_err(|e| sqlite("compare task discovery record family", e))
}
fn recovery() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}
