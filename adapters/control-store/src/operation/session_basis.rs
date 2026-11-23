//! Immutable operation-scoped session provenance in the existing start atom.
//! Storage compares opaque bytes; session admission remains application-owned.

use pale_core::records::{
    SessionAdmissionLeaseDigest, COMMON_REQUEST_ENVELOPE_V1_SCHEMA, OPERATION_HANDLE_V1_SCHEMA,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecordCoordinate, ControlStoreError, ControlStoreErrorKind,
    OperationCustodyCandidate, OperationSessionBasis, MAX_OPERATION_SESSION_BASIS_BYTES,
};
use rusqlite::{params, types::ValueRef, Connection, OptionalExtension, TransactionBehavior};

use super::{decode_record_coordinate, SqliteOperationCustody};
use crate::deployment::{a2a_evidence_reference_exists, require_coordinate, require_store_binding};
use crate::error::{sqlite, sqlite_integer};

fn recovery_required() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}

/// Point read with the body bounded before copying or application decoding.
pub(super) fn read(
    port: &mut SqliteOperationCustody<'_>,
    store: &AuthorityStoreCoordinate,
    handle: &ControlRecordCoordinate,
    maximum_bytes: usize,
) -> Result<Option<OperationSessionBasis>, ControlStoreError> {
    require_coordinate(&port.coordinate, store)?;
    let bound = maximum_bytes.min(MAX_OPERATION_SESSION_BASIS_BYTES);
    if bound == 0 {
        return Err(recovery_required());
    }
    let transaction = port
        .connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|source| sqlite("begin operation session basis read", source))?;
    require_store_binding(&transaction, store)?;
    let current = crate::store::transaction::read(&transaction, store)?;
    let selected = transaction.query_row(
        "SELECT handle_sha256, request_ref, request_sha256, session_lease_sha256, basis_sha256, \
         CASE WHEN length(canonical_basis_jcs) BETWEEN 1 AND ?2 THEN canonical_basis_jcs ELSE NULL END, \
         inserted_transaction_sequence FROM pale_deployment_operation_session_basis WHERE handle_ref=?1",
        params![handle.record_ref().as_str(), sqlite_integer(bound as u64)?],
        |row| Ok(decode(handle, bound, current.get(), [row.get_ref(0)?, row.get_ref(1)?, row.get_ref(2)?, row.get_ref(3)?, row.get_ref(4)?, row.get_ref(5)?, row.get_ref(6)?])),
    ).optional().map_err(|source| sqlite("read bounded operation session basis", source))?.transpose()?;
    let result = if let Some((basis, stamp)) = selected {
        if !association_matches(&transaction, &basis, stamp)? {
            return Err(recovery_required());
        }
        Some(basis)
    } else {
        None
    };
    transaction
        .rollback()
        .map_err(|source| sqlite("release operation session basis read", source))?;
    Ok(result)
}

fn decode(
    handle: &ControlRecordCoordinate,
    maximum_bytes: usize,
    current: u64,
    fields: [ValueRef<'_>; 7],
) -> Result<(OperationSessionBasis, u64), ControlStoreError> {
    let [ValueRef::Text(handle_digest), request_ref, request_digest, ValueRef::Text(lease), ValueRef::Text(digest), ValueRef::Blob(bytes), ValueRef::Integer(stamp)] =
        fields
    else {
        return Err(recovery_required());
    };
    let stamp = u64::try_from(stamp).map_err(|_| recovery_required())?;
    if handle_digest != handle.record_sha256().as_str().as_bytes()
        || lease.len() != 64
        || digest.len() != 64
        || bytes.is_empty()
        || bytes.len() > maximum_bytes
        || stamp == 0
        || stamp > current
    {
        return Err(recovery_required());
    }
    let request = decode_record_coordinate(request_ref, request_digest)?;
    let lease = SessionAdmissionLeaseDigest::parse(
        std::str::from_utf8(lease).map_err(|_| recovery_required())?,
    )
    .map_err(|_| recovery_required())?;
    let basis = OperationSessionBasis::new(
        handle.clone(),
        request,
        lease,
        bytes.to_vec(),
        maximum_bytes,
    )
    .map_err(|_| recovery_required())?;
    if basis.basis_sha256().as_str().as_bytes() != digest {
        return Err(recovery_required());
    }
    Ok((basis, stamp))
}

/// The attachment has no independent reference namespace. Its two keys must
/// name the signed start records, not raw evidence, in this same transaction.
fn association_matches(
    connection: &Connection,
    basis: &OperationSessionBasis,
    stamp: u64,
) -> Result<bool, ControlStoreError> {
    for (coordinate, schema) in [
        (basis.handle(), OPERATION_HANDLE_V1_SCHEMA),
        (basis.request(), COMMON_REQUEST_ENVELOPE_V1_SCHEMA),
    ] {
        if a2a_evidence_reference_exists(connection, coordinate.record_ref().as_str())? {
            return Ok(false);
        }
        let present: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_control_record WHERE record_ref=?1 AND record_sha256=?2 AND schema_id=?3 AND inserted_transaction_sequence=?4)",
            params![coordinate.record_ref().as_str(), coordinate.record_sha256().as_str(), schema, sqlite_integer(stamp)?], |row| row.get(0),
        ).map_err(|source| sqlite("compare operation session basis association", source))?;
        if !present {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn material_present(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    let Some(basis) = candidate.session_basis() else {
        return Ok(false);
    };
    connection.query_row("SELECT EXISTS(SELECT 1 FROM pale_deployment_operation_session_basis WHERE handle_ref=?1)", [basis.handle().record_ref().as_str()], |row| row.get(0))
        .map_err(|source| sqlite("read operation session basis presence", source))
}

pub(super) fn append(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<(), ControlStoreError> {
    let Some(basis) = candidate.session_basis() else {
        return Ok(());
    };
    let stamp = candidate
        .accumulator()
        .binding()
        .transaction()
        .successor()
        .get();
    if !association_matches(connection, basis, stamp)? {
        return Err(recovery_required());
    }
    connection.execute(
        "INSERT INTO pale_deployment_operation_session_basis \
         (handle_ref,handle_sha256,request_ref,request_sha256,session_lease_sha256,basis_sha256,canonical_basis_jcs,inserted_transaction_sequence) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![basis.handle().record_ref().as_str(), basis.handle().record_sha256().as_str(), basis.request().record_ref().as_str(), basis.request().record_sha256().as_str(), basis.session_lease_digest().as_str(), basis.basis_sha256().as_str(), basis.canonical_basis_jcs(), sqlite_integer(stamp)?],
    ).map_err(|source| sqlite("append operation session basis", source))?;
    Ok(())
}

/// Require the exact selected row and no unrelated basis in this atomic T.
pub(super) fn members_match(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    let stamp = candidate
        .accumulator()
        .binding()
        .transaction()
        .successor()
        .get();
    let selected = candidate.session_basis();
    if let Some(basis) = selected {
        if !association_matches(connection, basis, stamp)? {
            return Ok(false);
        }
        let exact: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_operation_session_basis \
             WHERE handle_ref=?1 AND handle_sha256=?2 AND request_ref=?3 AND request_sha256=?4 \
             AND session_lease_sha256=?5 AND basis_sha256=?6 AND canonical_basis_jcs=?7 AND inserted_transaction_sequence=?8)",
            params![basis.handle().record_ref().as_str(), basis.handle().record_sha256().as_str(), basis.request().record_ref().as_str(), basis.request().record_sha256().as_str(), basis.session_lease_digest().as_str(), basis.basis_sha256().as_str(), basis.canonical_basis_jcs(), sqlite_integer(stamp)?], |row| row.get(0),
        ).map_err(|source| sqlite("compare exact operation session basis", source))?;
        if !exact {
            return Ok(false);
        }
    }
    let extra: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_operation_session_basis \
         WHERE inserted_transaction_sequence=?1 AND (?2 IS NULL OR handle_ref<>?2))",
            params![
                sqlite_integer(stamp)?,
                selected.map(|basis| basis.handle().record_ref().as_str())
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare operation session basis atom closure", source))?;
    Ok(!extra)
}
