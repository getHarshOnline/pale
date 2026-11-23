//! Discover immutable operation sources without scanning records or replaying work.

use pale_core::records::{
    ControlTransactionSequenceV1, A2A_TASK_OBSERVATION_V1_SCHEMA, OPERATION_OBSERVATION_V1_SCHEMA,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlRecordCoordinate, ControlStoreError,
    ControlStoreErrorKind, OperationCustodyIndexKey, OperationPublicationCursor,
    OperationPublicationReadError, OperationPublicationSource, OperationPublicationSourceKind,
    OperationPublicationSources, MAX_OPERATION_PUBLICATION_BODY_BYTES,
    MAX_OPERATION_PUBLICATION_SOURCES,
};
use rusqlite::{params, Connection, TransactionBehavior};

use super::{decode_record_coordinate, read_index_coordinate, IndexRole, SqliteOperationCustody};
use crate::deployment::{read_record_bounded, require_coordinate, require_store_binding};
use crate::error::{sqlite, sqlite_integer};
use crate::store::transaction::{read as read_transaction, record_stamp};
use crate::ControlStoreKind;

pub(super) fn read(
    custody: &mut SqliteOperationCustody<'_>,
    store: &AuthorityStoreCoordinate,
    cursor: Option<&OperationPublicationCursor>,
    maximum_sources: usize,
    maximum_body_bytes: usize,
) -> Result<OperationPublicationSources, OperationPublicationReadError> {
    if maximum_sources == 0
        || maximum_sources > MAX_OPERATION_PUBLICATION_SOURCES
        || maximum_body_bytes == 0
        || maximum_body_bytes > MAX_OPERATION_PUBLICATION_BODY_BYTES
    {
        return Err(OperationPublicationReadError::Limit);
    }
    require_coordinate(&custody.coordinate, store)?;
    if cursor.is_some_and(|cursor| cursor.store() != store) {
        return Err(recovery().into());
    }
    let snapshot = custody
        .connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|source| sqlite("begin operation publication source snapshot", source))?;
    require_store_binding(&snapshot, store)?;
    let current = read_transaction(&snapshot, store)?;
    let through = cursor.map_or(current, OperationPublicationCursor::through);
    if through.get() > current.get() {
        return Err(recovery().into());
    }
    // The schema-leading index supports two bounded seeks, not an unbounded
    // cross-family sort. Merge coordinates before reading any record bodies.
    let mut local = read_coordinates(
        &snapshot,
        cursor,
        through,
        maximum_sources,
        OperationPublicationSourceKind::OperationObservation,
    )?
    .into_iter()
    .peekable();
    let mut remote = read_coordinates(
        &snapshot,
        cursor,
        through,
        maximum_sources,
        OperationPublicationSourceKind::A2aTaskObservation,
    )?
    .into_iter()
    .peekable();
    let mut remaining = maximum_body_bytes;
    let mut sources = Vec::with_capacity(maximum_sources);
    while sources.len() < maximum_sources {
        let source = match (local.peek(), remote.peek()) {
            (Some(left), Some(right)) => {
                if left.order_key() <= right.order_key() {
                    local.next()
                } else {
                    remote.next()
                }
            }
            (Some(_), None) => local.next(),
            (None, Some(_)) => remote.next(),
            (None, None) => break,
        }
        .ok_or_else(recovery)?;
        let role = match source.kind {
            OperationPublicationSourceKind::OperationObservation => {
                IndexRole::ObservationCheckpoint
            }
            OperationPublicationSourceKind::A2aTaskObservation => {
                IndexRole::A2aTaskObservationCheckpoint
            }
        };
        let checkpoint = read_index_coordinate(
            &snapshot,
            role,
            &OperationCustodyIndexKey::for_observation(&source.coordinate),
        )?
        .ok_or_else(recovery)?;
        if record_stamp(&snapshot, ControlStoreKind::DeploymentControl, &checkpoint)?
            != Some(source.transaction)
        {
            return Err(recovery().into());
        }
        let observation = read_bounded(&snapshot, &source.coordinate, &mut remaining)?;
        let checkpoint = read_bounded(&snapshot, &checkpoint, &mut remaining)?;
        let retained =
            OperationPublicationSource::new(source.transaction, observation, checkpoint)?;
        if retained.kind() != source.kind {
            return Err(recovery().into());
        }
        sources.push(retained);
    }
    let has_more = local.peek().is_some() || remote.peek().is_some();
    let continuation = if has_more {
        let last = sources.last().ok_or_else(recovery)?;
        Some(OperationPublicationCursor::new(
            store.clone(),
            through,
            last.transaction(),
            last.observation().coordinate().record_ref().clone(),
        )?)
    } else {
        None
    };
    snapshot
        .rollback()
        .map_err(|source| sqlite("release operation publication source snapshot", source))?;
    Ok(OperationPublicationSources::new(
        through,
        sources,
        continuation,
    ))
}

struct SourceCoordinate {
    transaction: ControlTransactionSequenceV1,
    coordinate: ControlRecordCoordinate,
    kind: OperationPublicationSourceKind,
}

impl SourceCoordinate {
    fn order_key(&self) -> (u64, &str) {
        (
            self.transaction.get(),
            self.coordinate.record_ref().as_str(),
        )
    }
}

fn read_coordinates(
    connection: &Connection,
    cursor: Option<&OperationPublicationCursor>,
    through: ControlTransactionSequenceV1,
    maximum_sources: usize,
    kind: OperationPublicationSourceKind,
) -> Result<Vec<SourceCoordinate>, OperationPublicationReadError> {
    let schema = match kind {
        OperationPublicationSourceKind::OperationObservation => OPERATION_OBSERVATION_V1_SCHEMA,
        OperationPublicationSourceKind::A2aTaskObservation => A2A_TASK_OBSERVATION_V1_SCHEMA,
    };
    let after_transaction = cursor.map_or(0, |cursor| cursor.after_transaction().get());
    let after_reference = cursor.map_or("", |cursor| cursor.after_reference().as_str());
    let mut coordinates = Vec::with_capacity(maximum_sources + 1);
    let mut statement = connection.prepare(
            "SELECT inserted_transaction_sequence, record_ref, record_sha256 \
             FROM pale_deployment_control_record INDEXED BY pale_deployment_control_record_source_order \
             WHERE schema_id=?1 AND inserted_transaction_sequence<=?2 \
               AND (inserted_transaction_sequence, record_ref)>(?3, ?4) \
             ORDER BY inserted_transaction_sequence, record_ref LIMIT ?5"
        ).map_err(|source| sqlite("prepare operation publication source seek", source))?;
    let mut rows = statement
        .query(params![
            schema,
            sqlite_integer(through.get())?,
            sqlite_integer(after_transaction)?,
            after_reference,
            sqlite_integer((maximum_sources + 1) as u64)?,
        ])
        .map_err(|source| sqlite("seek operation publication sources", source))?;
    while let Some(row) = rows
        .next()
        .map_err(|source| sqlite("read source coordinate", source))?
    {
        let stamp: i64 = row
            .get(0)
            .map_err(|source| sqlite("read source transaction", source))?;
        let stamp = u64::try_from(stamp)
            .ok()
            .and_then(|stamp| ControlTransactionSequenceV1::new(stamp).ok())
            .ok_or_else(recovery)?;
        let coordinate = decode_record_coordinate(
            row.get_ref(1)
                .map_err(|source| sqlite("read source reference", source))?,
            row.get_ref(2)
                .map_err(|source| sqlite("read source digest", source))?,
        )?;
        coordinates.push(SourceCoordinate {
            transaction: stamp,
            coordinate,
            kind,
        });
    }
    Ok(coordinates)
}

fn read_bounded(
    connection: &Connection,
    coordinate: &ControlRecordCoordinate,
    remaining: &mut usize,
) -> Result<ControlRecord, OperationPublicationReadError> {
    let length: i64 = connection
        .query_row(
            "SELECT length(canonical_record_jcs) FROM pale_deployment_control_record \
         WHERE record_ref=?1 AND record_sha256=?2",
            params![
                coordinate.record_ref().as_str(),
                coordinate.record_sha256().as_str()
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("read source body length", source))?;
    let length = usize::try_from(length)
        .ok()
        .filter(|length| *length > 0)
        .ok_or_else(recovery)?;
    if length > *remaining {
        return Err(OperationPublicationReadError::Limit);
    }
    let record = read_record_bounded(connection, coordinate, *remaining)?.ok_or_else(recovery)?;
    *remaining -= record.canonical_record_jcs().as_slice().len();
    Ok(record)
}

fn recovery() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}
