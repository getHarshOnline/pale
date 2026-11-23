//! Discover retained operation starts without interpreting their session basis.

use pale_core::records::{
    ArtifactDigest, ControlTransactionSequenceV1, SessionAdmissionLeaseDigest,
    COMMON_REQUEST_ENVELOPE_V1_SCHEMA, OPERATION_HANDLE_V1_SCHEMA,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecordCoordinate, ControlStoreError, ControlStoreErrorKind,
    OperationSessionBasis, OperationStartCursor, OperationStartReadError, OperationStartSource,
    OperationStartSources, MAX_OPERATION_SESSION_BASIS_BYTES, MAX_OPERATION_START_SOURCES,
    MAX_OPERATION_START_SOURCE_BYTES,
};
use rusqlite::{params, types::ValueRef, Connection, OptionalExtension, TransactionBehavior};

use super::{decode_record_coordinate, SqliteOperationCustody};
use crate::deployment::{a2a_evidence_reference_exists, require_coordinate, require_store_binding};
use crate::error::{sqlite, sqlite_integer};
use crate::store::transaction::{read as read_transaction, record_stamp};
use crate::ControlStoreKind;

pub(super) fn read(
    custody: &mut SqliteOperationCustody<'_>,
    store: &AuthorityStoreCoordinate,
    cursor: Option<&OperationStartCursor>,
    maximum_sources: usize,
    maximum_session_basis_bytes: usize,
) -> Result<OperationStartSources, OperationStartReadError> {
    if maximum_sources == 0
        || maximum_sources > MAX_OPERATION_START_SOURCES
        || maximum_session_basis_bytes == 0
        || maximum_session_basis_bytes > MAX_OPERATION_START_SOURCE_BYTES
    {
        return Err(OperationStartReadError::Limit);
    }
    require_coordinate(&custody.coordinate, store)?;
    if cursor.is_some_and(|position| position.store() != store) {
        return Err(recovery().into());
    }

    let snapshot = custody
        .connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|source| sqlite("begin operation start source snapshot", source))?;
    require_store_binding(&snapshot, store)?;
    let current = read_transaction(&snapshot, store)?;
    let through = cursor.map_or(current, OperationStartCursor::through);
    if through.get() > current.get() {
        return Err(recovery().into());
    }
    if let Some(position) = cursor {
        let retained = read_cursor_source(&snapshot, position)?.ok_or_else(recovery)?;
        validate_source(&snapshot, &retained, through)?;
        if !retained.matches_cursor(position) {
            return Err(recovery().into());
        }
    }

    let after_transaction = cursor.map_or(0, |position| position.after_transaction().get());
    let after_handle = cursor.map_or("", |position| position.handle().record_ref().as_str());
    let mut metadata = Vec::with_capacity(maximum_sources + 1);
    {
        let mut statement = snapshot
            .prepare(
                "SELECT inserted_transaction_sequence,handle_ref,handle_sha256,request_ref,request_sha256,\
                        session_lease_sha256,basis_sha256,length(canonical_basis_jcs) \
                 FROM pale_deployment_operation_session_basis \
                 INDEXED BY pale_deployment_operation_session_basis_transaction \
                 WHERE inserted_transaction_sequence<=?1 \
                   AND (inserted_transaction_sequence,handle_ref)>(?2,?3) \
                 ORDER BY inserted_transaction_sequence,handle_ref LIMIT ?4",
            )
            .map_err(|source| sqlite("prepare operation start source seek", source))?;
        let mut rows = statement
            .query(params![
                sqlite_integer(through.get())?,
                sqlite_integer(after_transaction)?,
                after_handle,
                sqlite_integer((maximum_sources + 1) as u64)?,
            ])
            .map_err(|source| sqlite("seek operation start sources", source))?;
        while let Some(row) = rows
            .next()
            .map_err(|source| sqlite("read operation start source position", source))?
        {
            metadata.push(decode_source([
                row.get_ref(0)
                    .map_err(|source| sqlite("read operation start transaction", source))?,
                row.get_ref(1)
                    .map_err(|source| sqlite("read operation start handle reference", source))?,
                row.get_ref(2)
                    .map_err(|source| sqlite("read operation start handle digest", source))?,
                row.get_ref(3)
                    .map_err(|source| sqlite("read operation start request reference", source))?,
                row.get_ref(4)
                    .map_err(|source| sqlite("read operation start request digest", source))?,
                row.get_ref(5)
                    .map_err(|source| sqlite("read operation start lease digest", source))?,
                row.get_ref(6)
                    .map_err(|source| sqlite("read operation start basis digest", source))?,
                row.get_ref(7)
                    .map_err(|source| sqlite("read operation start basis length", source))?,
            ])?);
        }
    }

    for source in &metadata {
        validate_source(&snapshot, source, through)?;
    }
    let returned = metadata.len().min(maximum_sources);
    let total_bytes = metadata[..returned]
        .iter()
        .try_fold(0usize, |total, source| {
            total.checked_add(source.basis_length)
        })
        .ok_or(OperationStartReadError::Limit)?;
    if total_bytes > maximum_session_basis_bytes {
        return Err(OperationStartReadError::Limit);
    }

    let has_more = metadata.len() > maximum_sources;
    let mut sources = Vec::with_capacity(returned);
    for source in metadata.into_iter().take(returned) {
        let bytes = read_exact_basis(&snapshot, &source)?;
        let basis = OperationSessionBasis::new(
            source.handle.clone(),
            source.request.clone(),
            source.session_lease_digest.clone(),
            bytes,
            MAX_OPERATION_SESSION_BASIS_BYTES,
        )
        .map_err(|_| recovery())?;
        if basis.basis_sha256() != &source.session_basis_digest {
            return Err(recovery().into());
        }
        sources.push(OperationStartSource::new(source.transaction, basis));
    }
    let continuation = if has_more {
        let last = sources.last().ok_or_else(recovery)?;
        let basis = last.session_basis();
        Some(OperationStartCursor::new(
            store.clone(),
            through,
            last.transaction(),
            basis.handle().clone(),
            basis.request().clone(),
            basis.session_lease_digest().clone(),
            basis.basis_sha256().clone(),
        )?)
    } else {
        None
    };
    snapshot
        .rollback()
        .map_err(|source| sqlite("release operation start source snapshot", source))?;
    Ok(OperationStartSources::new(through, sources, continuation))
}

#[derive(Debug)]
struct Source {
    transaction: ControlTransactionSequenceV1,
    handle: ControlRecordCoordinate,
    request: ControlRecordCoordinate,
    session_lease_digest: SessionAdmissionLeaseDigest,
    session_basis_digest: ArtifactDigest,
    basis_length: usize,
}

impl Source {
    fn matches_cursor(&self, cursor: &OperationStartCursor) -> bool {
        self.transaction == cursor.after_transaction()
            && &self.handle == cursor.handle()
            && &self.request == cursor.request()
            && &self.session_lease_digest == cursor.session_lease_digest()
            && &self.session_basis_digest == cursor.session_basis_digest()
    }
}

fn read_cursor_source(
    connection: &Connection,
    cursor: &OperationStartCursor,
) -> Result<Option<Source>, ControlStoreError> {
    connection
        .query_row(
            "SELECT inserted_transaction_sequence,handle_ref,handle_sha256,request_ref,request_sha256,\
                    session_lease_sha256,basis_sha256,length(canonical_basis_jcs) \
             FROM pale_deployment_operation_session_basis \
             WHERE inserted_transaction_sequence=?1 AND handle_ref=?2",
            params![
                sqlite_integer(cursor.after_transaction().get())?,
                cursor.handle().record_ref().as_str(),
            ],
            |row| {
                Ok(decode_source([
                    row.get_ref(0)?,
                    row.get_ref(1)?,
                    row.get_ref(2)?,
                    row.get_ref(3)?,
                    row.get_ref(4)?,
                    row.get_ref(5)?,
                    row.get_ref(6)?,
                    row.get_ref(7)?,
                ]))
            },
        )
        .optional()
        .map_err(|source| sqlite("read operation start continuation", source))?
        .transpose()
}

fn decode_source(fields: [ValueRef<'_>; 8]) -> Result<Source, ControlStoreError> {
    let [ValueRef::Integer(transaction), handle_ref, handle_digest, request_ref, request_digest, ValueRef::Text(lease), ValueRef::Text(basis_digest), ValueRef::Integer(basis_length)] =
        fields
    else {
        return Err(recovery());
    };
    let transaction = u64::try_from(transaction)
        .ok()
        .and_then(|value| ControlTransactionSequenceV1::new(value).ok())
        .ok_or_else(recovery)?;
    let basis_length = usize::try_from(basis_length)
        .ok()
        .filter(|length| *length > 0 && *length <= MAX_OPERATION_SESSION_BASIS_BYTES)
        .ok_or_else(recovery)?;
    let session_lease_digest =
        SessionAdmissionLeaseDigest::parse(std::str::from_utf8(lease).map_err(|_| recovery())?)
            .map_err(|_| recovery())?;
    let session_basis_digest =
        ArtifactDigest::parse(std::str::from_utf8(basis_digest).map_err(|_| recovery())?)
            .map_err(|_| recovery())?;
    Ok(Source {
        transaction,
        handle: decode_record_coordinate(handle_ref, handle_digest)?,
        request: decode_record_coordinate(request_ref, request_digest)?,
        session_lease_digest,
        session_basis_digest,
        basis_length,
    })
}

fn validate_source(
    connection: &Connection,
    source: &Source,
    through: ControlTransactionSequenceV1,
) -> Result<(), ControlStoreError> {
    if source.transaction.get() > through.get()
        || source.handle.record_ref() == source.request.record_ref()
    {
        return Err(recovery());
    }
    for (coordinate, schema) in [
        (&source.handle, OPERATION_HANDLE_V1_SCHEMA),
        (&source.request, COMMON_REQUEST_ENVELOPE_V1_SCHEMA),
    ] {
        if a2a_evidence_reference_exists(connection, coordinate.record_ref().as_str())?
            || record_stamp(connection, ControlStoreKind::DeploymentControl, coordinate)?
                != Some(source.transaction)
        {
            return Err(recovery());
        }
        let exact: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pale_deployment_control_record \
                 WHERE record_ref=?1 AND record_sha256=?2 AND schema_id=?3)",
                params![
                    coordinate.record_ref().as_str(),
                    coordinate.record_sha256().as_str(),
                    schema,
                ],
                |row| row.get(0),
            )
            .map_err(|error| sqlite("compare operation start record association", error))?;
        if !exact {
            return Err(recovery());
        }
    }
    Ok(())
}

fn read_exact_basis(
    connection: &Connection,
    source: &Source,
) -> Result<Vec<u8>, ControlStoreError> {
    let bytes: Vec<u8> = connection
        .query_row(
            "SELECT canonical_basis_jcs FROM pale_deployment_operation_session_basis \
             WHERE inserted_transaction_sequence=?1 AND handle_ref=?2 AND handle_sha256=?3 \
               AND request_ref=?4 AND request_sha256=?5 AND session_lease_sha256=?6 \
               AND basis_sha256=?7 AND length(canonical_basis_jcs)=?8",
            params![
                sqlite_integer(source.transaction.get())?,
                source.handle.record_ref().as_str(),
                source.handle.record_sha256().as_str(),
                source.request.record_ref().as_str(),
                source.request.record_sha256().as_str(),
                source.session_lease_digest.as_str(),
                source.session_basis_digest.as_str(),
                sqlite_integer(source.basis_length as u64)?,
            ],
            |row| row.get(0),
        )
        .map_err(|error| sqlite("read exact operation start session basis", error))?;
    if bytes.len() != source.basis_length {
        return Err(recovery());
    }
    Ok(bytes)
}

fn recovery() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}
