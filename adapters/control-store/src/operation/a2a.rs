//! Exact protocol-byte custody attached to the existing operation transaction.
//! Raw payloads are never interpreted or recanonicalized by this adapter.

use pale_core::records::{
    ArtifactDigest, ControlTransactionSequenceV1, ImmutableRecordReferenceV1, ResidentRecordRefV1,
    A2A_ROUTE_ADMISSION_V1_SCHEMA, A2A_TASK_BINDING_V1_SCHEMA, A2A_TASK_OBSERVATION_V1_SCHEMA,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecordCoordinate, ControlRecordDigest, ControlStoreError,
    ControlStoreErrorKind, OperationA2aEvidence, OperationCheckpointPhase,
    OperationCustodyCandidate,
};
use rusqlite::{params, types::ValueRef, Connection, OptionalExtension, TransactionBehavior};

use super::SqliteOperationCustody;
use crate::deployment::{
    a2a_evidence_reference_exists as reference_exists, record_matches, record_reference_exists,
    require_coordinate, require_store_binding,
};
use crate::error::{sqlite, sqlite_integer};

fn recovery_required() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}

/// Read one exact raw coordinate with bounds checked before copying its body.
pub(super) fn read(
    port: &mut SqliteOperationCustody<'_>,
    store: &AuthorityStoreCoordinate,
    coordinate: &ControlRecordCoordinate,
    maximum_bytes: usize,
) -> Result<Option<OperationA2aEvidence>, ControlStoreError> {
    require_coordinate(&port.coordinate, store)?;
    if maximum_bytes == 0 {
        return Err(recovery_required());
    }
    let transaction = port
        .connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|source| sqlite("begin A2A protocol-evidence read", source))?;
    require_store_binding(&transaction, store)?;
    if record_reference_exists(&transaction, coordinate.record_ref().as_str())? {
        return Err(recovery_required());
    }
    let current = crate::store::transaction::read(&transaction, store)?;
    let evidence = transaction
        .query_row(
            "SELECT record_sha256, evidence_bytes, sse_event_id, sse_event_type, \
             inserted_transaction_sequence FROM pale_deployment_operation_a2a_evidence \
             WHERE record_ref=?1",
            [coordinate.record_ref().as_str()],
            |row| {
                Ok(decode(
                    coordinate,
                    maximum_bytes,
                    current.get(),
                    [
                        row.get_ref(0)?,
                        row.get_ref(1)?,
                        row.get_ref(2)?,
                        row.get_ref(3)?,
                        row.get_ref(4)?,
                    ],
                ))
            },
        )
        .optional()
        .map_err(|source| sqlite("read bounded A2A protocol evidence", source))?
        .transpose()?;
    transaction
        .rollback()
        .map_err(|source| sqlite("release A2A protocol-evidence read", source))?;
    Ok(evidence)
}

/// Read the exact bounded raw members inserted by one admitted transaction.
pub(super) fn read_inserted_at(
    port: &mut SqliteOperationCustody<'_>,
    store: &AuthorityStoreCoordinate,
    transaction_sequence: ControlTransactionSequenceV1,
    maximum_items: usize,
    maximum_bytes: usize,
) -> Result<Vec<OperationA2aEvidence>, ControlStoreError> {
    require_coordinate(&port.coordinate, store)?;
    if maximum_items == 0 || maximum_bytes == 0 {
        return Err(recovery_required());
    }
    let transaction = port
        .connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|source| sqlite("begin A2A transaction-evidence read", source))?;
    require_store_binding(&transaction, store)?;
    let current = crate::store::transaction::read(&transaction, store)?;
    if transaction_sequence.get() > current.get() {
        return Err(recovery_required());
    }
    let limit = maximum_items
        .checked_add(1)
        .and_then(|value| i64::try_from(value).ok())
        .ok_or_else(recovery_required)?;
    let mut statement = transaction
        .prepare(
            "SELECT record_ref,record_sha256,evidence_bytes,sse_event_id,sse_event_type,\
             inserted_transaction_sequence FROM pale_deployment_operation_a2a_evidence \
             INDEXED BY pale_deployment_operation_a2a_evidence_transaction \
             WHERE inserted_transaction_sequence=?1 ORDER BY record_ref LIMIT ?2",
        )
        .map_err(|source| sqlite("prepare A2A transaction-evidence read", source))?;
    let mut rows = statement
        .query(params![sqlite_integer(transaction_sequence.get())?, limit])
        .map_err(|source| sqlite("read A2A transaction evidence", source))?;
    let mut evidence = Vec::new();
    let mut remaining = maximum_bytes;
    while let Some(row) = rows
        .next()
        .map_err(|source| sqlite("advance A2A transaction-evidence read", source))?
    {
        if evidence.len() == maximum_items {
            return Err(recovery_required());
        }
        let ValueRef::Text(reference) = row
            .get_ref(0)
            .map_err(|source| sqlite("decode A2A transaction-evidence reference", source))?
        else {
            return Err(recovery_required());
        };
        let reference = std::str::from_utf8(reference)
            .ok()
            .and_then(|value| ResidentRecordRefV1::parse(value.to_owned()).ok())
            .ok_or_else(recovery_required)?;
        if record_reference_exists(&transaction, reference.as_str())? {
            return Err(recovery_required());
        }
        let ValueRef::Text(digest) = row
            .get_ref(1)
            .map_err(|source| sqlite("decode A2A transaction-evidence digest", source))?
        else {
            return Err(recovery_required());
        };
        let digest = std::str::from_utf8(digest)
            .ok()
            .and_then(|value| ControlRecordDigest::parse(value.to_owned()).ok())
            .ok_or_else(recovery_required)?;
        let coordinate = ControlRecordCoordinate::new(reference, digest);
        let item = decode(
            &coordinate,
            remaining,
            current.get(),
            [
                row.get_ref(1)
                    .map_err(|source| sqlite("decode A2A transaction digest", source))?,
                row.get_ref(2)
                    .map_err(|source| sqlite("decode A2A transaction bytes", source))?,
                row.get_ref(3)
                    .map_err(|source| sqlite("decode A2A transaction SSE ID", source))?,
                row.get_ref(4)
                    .map_err(|source| sqlite("decode A2A transaction SSE type", source))?,
                row.get_ref(5)
                    .map_err(|source| sqlite("decode A2A transaction stamp", source))?,
            ],
        )?;
        remaining = remaining
            .checked_sub(item.byte_len())
            .ok_or_else(recovery_required)?;
        evidence.push(item);
    }
    drop(rows);
    drop(statement);
    transaction
        .rollback()
        .map_err(|source| sqlite("release A2A transaction-evidence read", source))?;
    Ok(evidence)
}

fn decode(
    coordinate: &ControlRecordCoordinate,
    maximum_bytes: usize,
    current_transaction: u64,
    fields: [ValueRef<'_>; 5],
) -> Result<OperationA2aEvidence, ControlStoreError> {
    let [ValueRef::Text(digest), ValueRef::Blob(bytes), event_id, event_type, ValueRef::Integer(stamp)] =
        fields
    else {
        return Err(recovery_required());
    };
    if digest != coordinate.record_sha256().as_str().as_bytes()
        || u64::try_from(stamp)
            .ok()
            .is_none_or(|stamp| stamp == 0 || stamp > current_transaction)
    {
        return Err(recovery_required());
    }
    let sse = match (event_id, event_type) {
        (ValueRef::Null, ValueRef::Null) => None,
        (ValueRef::Text(id), ValueRef::Text(kind)) => Some((id, kind)),
        _ => return Err(recovery_required()),
    };
    let length = sse.map_or(Some(bytes.len()), |(id, kind)| {
        bytes.len().checked_add(id.len())?.checked_add(kind.len())
    });
    if length.is_none_or(|length| length > maximum_bytes) {
        return Err(recovery_required());
    }
    let sse = sse
        .map(|(id, kind)| {
            Ok::<_, ControlStoreError>((
                std::str::from_utf8(id)
                    .map_err(|_| recovery_required())?
                    .to_owned(),
                std::str::from_utf8(kind)
                    .map_err(|_| recovery_required())?
                    .to_owned(),
            ))
        })
        .transpose()?;
    OperationA2aEvidence::new(
        ImmutableRecordReferenceV1 {
            record_ref: coordinate.record_ref().clone(),
            record_sha256: ArtifactDigest::parse(coordinate.record_sha256().as_str())
                .map_err(|_| recovery_required())?,
        },
        bytes.to_vec(),
        sse,
        maximum_bytes,
    )
    .map_err(|_| recovery_required())
}

/// Compare dependencies selected as already retained, never as optional upserts.
/// The application proves their membership under this predecessor checkpoint;
/// a route receiving its first domain association instead predates the physical
/// transaction. The adapter rejects later insertion or cross-namespace aliasing.
pub(super) fn retained_match(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    if candidate.a2a_retained_records().is_empty()
        && candidate.a2a_retained_evidence().is_empty()
        && candidate.a2a_associated_route().is_none()
    {
        return Ok(true);
    }
    let OperationCheckpointPhase::Successor { predecessor, .. } = candidate.checkpoint().phase()
    else {
        return Ok(false);
    };
    let maximum_stamp = predecessor.committed_transaction_sequence().get();
    let physical_predecessor = candidate
        .accumulator()
        .binding()
        .transaction()
        .expected()
        .get();
    for (record, record_maximum_stamp) in candidate
        .a2a_retained_records()
        .iter()
        .map(|record| (record, maximum_stamp))
        .chain(
            candidate
                .a2a_associated_route()
                .map(|record| (record, physical_predecessor)),
        )
    {
        if !record_matches(connection, record)?
            || reference_exists(connection, record.coordinate().record_ref().as_str())?
        {
            return Ok(false);
        }
        let stamp = crate::store::transaction::record_stamp(
            connection,
            crate::ControlStoreKind::DeploymentControl,
            record.coordinate(),
        )?;
        if stamp.is_none_or(|stamp| stamp.get() > record_maximum_stamp) {
            return Ok(false);
        }
    }
    for evidence in candidate.a2a_retained_evidence() {
        if !evidence_matches(connection, evidence, 1, maximum_stamp)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// New raw evidence must be absent; exact present bytes are not a predecessor.
pub(super) fn new_evidence_absent(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    for evidence in candidate.a2a_evidence() {
        let reference = evidence.coordinate().record_ref().as_str();
        if reference_exists(connection, reference)?
            || record_reference_exists(connection, reference)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Raw-only partial custody must remain detectable after unrelated advancement.
pub(super) fn new_material_present(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    for evidence in candidate.a2a_evidence() {
        let reference = evidence.coordinate().record_ref().as_str();
        if reference_exists(connection, reference)?
            || record_reference_exists(connection, reference)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Append only the raw members explicitly selected as new in this atom.
pub(super) fn append(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<(), ControlStoreError> {
    for evidence in candidate.a2a_evidence() {
        let (event_id, event_type) = evidence
            .sse()
            .map_or((None, None), |(id, kind)| (Some(id), Some(kind)));
        connection
            .execute(
                "INSERT INTO pale_deployment_operation_a2a_evidence \
                 (record_ref,record_sha256,evidence_bytes,sse_event_id,sse_event_type,inserted_transaction_sequence) \
                 VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    evidence.coordinate().record_ref().as_str(),
                    evidence.coordinate().record_sha256().as_str(),
                    evidence.bytes(),
                    event_id,
                    event_type,
                    sqlite_integer(candidate.accumulator().binding().transaction().successor().get())?,
                ],
            )
            .map_err(|source| sqlite("append A2A protocol evidence", source))?;
    }
    Ok(())
}

fn evidence_matches(
    connection: &Connection,
    evidence: &OperationA2aEvidence,
    minimum_stamp: u64,
    maximum_stamp: u64,
) -> Result<bool, ControlStoreError> {
    if record_reference_exists(connection, evidence.coordinate().record_ref().as_str())? {
        return Ok(false);
    }
    let (event_id, event_type) = evidence
        .sse()
        .map_or((None, None), |(id, kind)| (Some(id), Some(kind)));
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_operation_a2a_evidence \
             WHERE record_ref=?1 AND record_sha256=?2 AND evidence_bytes=?3 \
               AND sse_event_id IS ?4 AND sse_event_type IS ?5 \
               AND inserted_transaction_sequence BETWEEN ?6 AND ?7)",
            params![
                evidence.coordinate().record_ref().as_str(),
                evidence.coordinate().record_sha256().as_str(),
                evidence.bytes(),
                event_id,
                event_type,
                sqlite_integer(minimum_stamp)?,
                sqlite_integer(maximum_stamp)?,
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare exact A2A protocol evidence", source))
}

/// Require exact raw members at the candidate's one physical transaction and
/// no unselected A2A members. Indexed LIMIT queries visit at most the selected
/// bounded member count plus one; they never scan the historical prefix.
pub(super) fn new_members_match(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    for record in candidate
        .records()
        .iter()
        .map(pale_ports::OperationCustodyRecord::record)
        .chain(std::iter::once(candidate.checkpoint().successor_record()))
    {
        if reference_exists(connection, record.coordinate().record_ref().as_str())? {
            return Ok(false);
        }
    }
    let stamp = candidate
        .accumulator()
        .binding()
        .transaction()
        .successor()
        .get();
    for evidence in candidate.a2a_evidence() {
        if !evidence_matches(connection, evidence, stamp, stamp)? {
            return Ok(false);
        }
    }
    let mut statement = connection
        .prepare(
            "SELECT record_ref FROM pale_deployment_operation_a2a_evidence \
             WHERE inserted_transaction_sequence=?1 LIMIT ?2",
        )
        .map_err(|source| sqlite("prepare A2A protocol atom closure", source))?;
    let mut rows = statement
        .query(params![
            sqlite_integer(stamp)?,
            i64::try_from(candidate.a2a_evidence().len() + 1).map_err(|_| recovery_required())?,
        ])
        .map_err(|source| sqlite("read A2A protocol atom closure", source))?;
    while let Some(row) = rows
        .next()
        .map_err(|source| sqlite("advance A2A protocol atom closure", source))?
    {
        let ValueRef::Text(reference) = row
            .get_ref(0)
            .map_err(|source| sqlite("decode A2A protocol atom closure", source))?
        else {
            return Ok(false);
        };
        if !candidate
            .a2a_evidence()
            .iter()
            .any(|evidence| evidence.coordinate().record_ref().as_str().as_bytes() == reference)
        {
            return Ok(false);
        }
    }
    let mut statement = connection
        .prepare(
            "SELECT record_ref FROM pale_deployment_control_record \
             WHERE schema_id IN (?1,?2,?3,?4,?5,?6) AND inserted_transaction_sequence=?7 LIMIT ?8",
        )
        .map_err(|source| sqlite("prepare A2A record atom closure", source))?;
    let mut rows = statement
        .query(params![
            A2A_ROUTE_ADMISSION_V1_SCHEMA,
            A2A_TASK_BINDING_V1_SCHEMA,
            A2A_TASK_OBSERVATION_V1_SCHEMA,
            pale_ports::PRIVATE_OPERATION_A2A_SEND_PREPARATION_V1_SCHEMA,
            pale_ports::PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA,
            pale_ports::PRIVATE_OPERATION_A2A_GET_TASK_LOCAL_DISPOSITION_V1_SCHEMA,
            sqlite_integer(stamp)?,
            i64::try_from(candidate.records().len() + 1).map_err(|_| recovery_required())?,
        ])
        .map_err(|source| sqlite("read A2A record atom closure", source))?;
    while let Some(row) = rows
        .next()
        .map_err(|source| sqlite("advance A2A record atom closure", source))?
    {
        let ValueRef::Text(reference) = row
            .get_ref(0)
            .map_err(|source| sqlite("decode A2A record atom closure", source))?
        else {
            return Ok(false);
        };
        if !candidate.records().iter().any(|member| {
            member
                .record()
                .coordinate()
                .record_ref()
                .as_str()
                .as_bytes()
                == reference
        }) {
            return Ok(false);
        }
    }
    Ok(true)
}
