//! Shared physical operations over deployment-control records.
//!
//! These helpers own the one opaque immutable-record table used by resident
//! effect and Event custody. Domain-specific indexes and transitions remain in
//! their respective adapters.

use pale_core::records::{ResidentRecordRefV1, SchemaDigest, SchemaId};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlRecordBytes, ControlRecordCoordinate,
    ControlRecordDigest, ControlStoreError, ControlStoreErrorKind,
};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::sqlite;

/// Compare the complete immutable deployment-store binding, including the
/// authority coordinates retained by the genesis closure.
pub(crate) fn require_genesis_binding(
    connection: &Connection,
    binding: &pale_ports::DeploymentControlStoreBinding,
) -> Result<(), ControlStoreError> {
    require_store_binding(connection, binding.store())?;
    let matches = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_control_store_binding \
         WHERE singleton=1 AND workspace_id=?1 AND deployment_binding_ref=?2 \
           AND deployment_binding_sha256=?3)",
            params![
                binding.workspace_id().as_str(),
                binding.deployment_binding().record_ref().as_str(),
                binding.deployment_binding().record_sha256().as_str()
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|source| sqlite("compare deployment genesis store binding", source))?;
    matches
        .then_some(())
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
}

/// Maximum schema-identity bytes accepted before allocating a stored value.
const MAX_CONTROL_RECORD_SCHEMA_ID_BYTES: usize = 512;
/// Fixed lowercase SHA-256 byte length used by stored record metadata.
const CONTROL_RECORD_DIGEST_BYTES: usize = 64;

/// Reject cross-store or cross-incarnation substitution before physical I/O.
pub(crate) fn require_coordinate(
    expected: &AuthorityStoreCoordinate,
    supplied: &AuthorityStoreCoordinate,
) -> Result<(), ControlStoreError> {
    (expected == supplied)
        .then_some(())
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
}

/// Require the database header to name the exact deployment-store incarnation.
pub(crate) fn require_store_binding(
    connection: &Connection,
    coordinate: &AuthorityStoreCoordinate,
) -> Result<(), ControlStoreError> {
    let matches = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_control_store_header \
             WHERE singleton = 1 AND store_kind = 'deployment_control' \
               AND logical_store_id = ?1 AND control_incarnation_id = ?2)",
            params![
                coordinate.store_id().as_str(),
                coordinate.incarnation_id().as_str(),
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|source| sqlite("compare deployment-control store binding", source))?;
    matches
        .then_some(())
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
}

/// Insert one already-admitted opaque deployment record exactly once.
pub(crate) fn insert_record(
    connection: &Connection,
    record: &ControlRecord,
) -> Result<(), ControlStoreError> {
    if a2a_evidence_reference_exists(connection, record.coordinate().record_ref().as_str())? {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    connection
        .execute(
            "INSERT INTO pale_deployment_control_record \
             (record_ref, record_sha256, schema_id, schema_sha256, canonical_record_jcs, \
              inserted_transaction_sequence) \
             VALUES (?1, ?2, ?3, ?4, ?5, \
              (SELECT logical_transaction_sequence FROM pale_control_store_header WHERE singleton = 1))",
            params![
                record.coordinate().record_ref().as_str(),
                record.coordinate().record_sha256().as_str(),
                record.schema_id().as_str(),
                record.schema_sha256().as_str(),
                record.canonical_record_jcs().as_slice(),
            ],
        )
        .map(drop)
        .map_err(|source| sqlite("insert deployment-control record", source))
}

/// Preserve one immutable-reference namespace across signed and raw custody.
pub(crate) fn a2a_evidence_reference_exists(
    connection: &Connection,
    reference: &str,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_operation_a2a_evidence \
             WHERE record_ref=?1)",
            params![reference],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare A2A protocol-evidence reference", source))
}

/// Compare one immutable deployment record byte-for-byte.
pub(crate) fn record_matches(
    connection: &Connection,
    record: &ControlRecord,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_control_record \
             WHERE record_ref = ?1 AND record_sha256 = ?2 AND schema_id = ?3 \
               AND schema_sha256 = ?4 AND canonical_record_jcs = ?5)",
            params![
                record.coordinate().record_ref().as_str(),
                record.coordinate().record_sha256().as_str(),
                record.schema_id().as_str(),
                record.schema_sha256().as_str(),
                record.canonical_record_jcs().as_slice(),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare deployment-control record", source))
}

/// Read one immutable deployment record, rejecting reference reuse.
pub(crate) fn read_record(
    connection: &Connection,
    coordinate: &ControlRecordCoordinate,
) -> Result<Option<ControlRecord>, ControlStoreError> {
    let row = connection
        .query_row(
            "SELECT record_sha256, schema_id, schema_sha256, canonical_record_jcs \
             FROM pale_deployment_control_record WHERE record_ref = ?1",
            [coordinate.record_ref().as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|source| sqlite("read deployment-control record", source))?;
    let Some((record_sha256, schema_id, schema_sha256, bytes)) = row else {
        return Ok(None);
    };
    if record_sha256 != coordinate.record_sha256().as_str() {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    let record_ref = ResidentRecordRefV1::parse(coordinate.record_ref().as_str())
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let digest = ControlRecordDigest::parse(record_sha256)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let schema_id = SchemaId::parse(schema_id)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let schema_sha256 = SchemaDigest::parse(schema_sha256)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let bytes = ControlRecordBytes::new(bytes)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    Ok(Some(ControlRecord::new(
        ControlRecordCoordinate::new(record_ref, digest),
        schema_id,
        schema_sha256,
        bytes,
    )))
}

/// Validated physical metadata; no body or semantic admission is retained.
struct StoredRecordMetadata {
    schema_id: SchemaId,
    schema_sha256: SchemaDigest,
    body_bytes: usize,
}

/// Validate one coordinate and its stored metadata without copying the body.
/// The caller holds a consistent read transaction across these queries.
fn read_record_metadata(
    connection: &Connection,
    coordinate: &ControlRecordCoordinate,
) -> Result<Option<StoredRecordMetadata>, ControlStoreError> {
    let metadata = connection
        .query_row(
            "SELECT record_sha256 = ?2, length(CAST(record_sha256 AS BLOB)), \
                    length(CAST(schema_id AS BLOB)), length(CAST(schema_sha256 AS BLOB)), \
                    length(canonical_record_jcs), typeof(canonical_record_jcs) = 'blob' \
             FROM pale_deployment_control_record WHERE record_ref = ?1",
            params![
                coordinate.record_ref().as_str(),
                coordinate.record_sha256().as_str(),
            ],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, bool>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|source| sqlite("read bounded deployment-record metadata", source))?;
    let Some((digest_exact, digest_len, schema_id_len, schema_sha256_len, byte_len, body_is_blob)) =
        metadata
    else {
        return Ok(None);
    };
    let digest_len = usize::try_from(digest_len)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let schema_id_len = usize::try_from(schema_id_len)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let schema_sha256_len = usize::try_from(schema_sha256_len)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let byte_len = usize::try_from(byte_len)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    if !digest_exact
        || digest_len != CONTROL_RECORD_DIGEST_BYTES
        || !(1..=MAX_CONTROL_RECORD_SCHEMA_ID_BYTES).contains(&schema_id_len)
        || schema_sha256_len != CONTROL_RECORD_DIGEST_BYTES
        || byte_len == 0
        || !body_is_blob
    {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    let (schema_id, schema_sha256) = connection
        .query_row(
            "SELECT schema_id, schema_sha256 \
             FROM pale_deployment_control_record WHERE record_ref = ?1",
            [coordinate.record_ref().as_str()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|source| sqlite("read bounded deployment record", source))?;
    if schema_id.len() != schema_id_len || schema_sha256.len() != schema_sha256_len {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    let schema_id = SchemaId::parse(schema_id)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let schema_sha256 = SchemaDigest::parse(schema_sha256)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    Ok(Some(StoredRecordMetadata {
        schema_id,
        schema_sha256,
        body_bytes: byte_len,
    }))
}

/// Read the body length after validating its coordinate and stored metadata.
/// The caller keeps the same read transaction for any later bounded body read.
pub(crate) fn record_body_length(
    connection: &Connection,
    coordinate: &ControlRecordCoordinate,
) -> Result<Option<usize>, ControlStoreError> {
    Ok(read_record_metadata(connection, coordinate)?.map(|metadata| metadata.body_bytes))
}

/// Check physical record completeness without copying an opaque body.
/// This preserves coordinate, schema and nonempty-body validation; it does not
/// verify canonical JSON, a signature, or an index role's semantic schema.
pub(crate) fn record_is_complete(
    connection: &Connection,
    coordinate: &ControlRecordCoordinate,
) -> Result<bool, ControlStoreError> {
    Ok(read_record_metadata(connection, coordinate)?.is_some())
}

/// Read one exact immutable deployment record under an explicit body ceiling.
///
/// Metadata lengths are checked before copying schema strings into Rust-owned
/// values. The body is copied only after its length fits the caller's ceiling.
/// The caller holds one consistent read transaction across all queries. `SQLite`
/// page reads and internal allocations are separate from this Rust copy bound.
pub(crate) fn read_record_bounded(
    connection: &Connection,
    coordinate: &ControlRecordCoordinate,
    maximum_body_bytes: usize,
) -> Result<Option<ControlRecord>, ControlStoreError> {
    let Some(metadata) = read_record_metadata(connection, coordinate)? else {
        return Ok(None);
    };
    if metadata.body_bytes > maximum_body_bytes {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    let bytes: Vec<u8> = connection
        .query_row(
            "SELECT canonical_record_jcs FROM pale_deployment_control_record WHERE record_ref = ?1",
            [coordinate.record_ref().as_str()],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("read bounded deployment record body", source))?;
    if bytes.len() != metadata.body_bytes {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    let bytes = ControlRecordBytes::new(bytes)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    Ok(Some(ControlRecord::new(
        coordinate.clone(),
        metadata.schema_id,
        metadata.schema_sha256,
        bytes,
    )))
}

/// Test whether any row already owns an immutable deployment record reference.
pub(crate) fn record_reference_exists(
    connection: &Connection,
    record_ref: &str,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_control_record \
             WHERE record_ref = ?1)",
            [record_ref],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("test deployment-control record reference", source))
}
