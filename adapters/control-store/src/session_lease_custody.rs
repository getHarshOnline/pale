//! `SQLite` retention of canonical session-admission lease bodies.
//!
//! Rows are immutable recovery inputs keyed by the digest named in resident
//! authority. They grant no session authority and are deliberately retained
//! after a session drain so historical replay does not depend on live memory.

use pale_core::records::{ArtifactDigest, SchemaDigest, SchemaId};
use pale_ports::{
    ControlRecordBytes, ControlStoreError, ControlStoreErrorKind, SessionLeaseBody,
    SessionLeaseBodyRead, SessionLeaseBodyRetention, SessionLeaseCustodyPort,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::authority::DeploymentControlSqliteStore;

/// Physical schema bound applied before allocating one retained schema identity.
const MAX_SESSION_LEASE_SCHEMA_ID_BYTES: usize = 512;
/// Lowercase SHA-256 text length fixed by the physical schema.
const SESSION_LEASE_SCHEMA_DIGEST_BYTES: usize = 64;

impl SessionLeaseCustodyPort for DeploymentControlSqliteStore {
    fn compare_and_retain(
        &self,
        body: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        self.check_session_lease_coordinate(body.store())?;
        let mut connection = self.record_custody_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        crate::store::transaction::read(&transaction, body.store())?;
        let inserted = transaction
            .execute(
                "INSERT INTO pale_deployment_session_lease_body \
                 (session_admission_lease_digest, schema_id, schema_sha256, canonical_wire_jcs, \
                  private_admission_basis_sha256, canonical_private_admission_basis_jcs) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT(session_admission_lease_digest) DO NOTHING",
                params![
                    body.digest().as_str(),
                    body.schema_id().as_str(),
                    body.schema_sha256().as_str(),
                    body.canonical_wire_jcs().as_slice(),
                    body.private_admission_basis_sha256()
                        .map(|digest| digest.as_str()),
                    body.canonical_private_admission_basis_jcs()
                        .map(ControlRecordBytes::as_slice),
                ],
            )
            .map_err(storage)?;
        let disposition = if inserted == 1 {
            let prepared = crate::store::transaction::prepare(&transaction, body.store())?;
            crate::store::transaction::advance(&transaction, &prepared)?;
            SessionLeaseBodyRetention::Retained
        } else {
            let exact = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM pale_deployment_session_lease_body \
                     WHERE session_admission_lease_digest = ?1 \
                     AND schema_id = ?2 AND schema_sha256 = ?3 \
                     AND canonical_wire_jcs = ?4 \
                     AND private_admission_basis_sha256 IS ?5 \
                     AND canonical_private_admission_basis_jcs IS ?6)",
                    params![
                        body.digest().as_str(),
                        body.schema_id().as_str(),
                        body.schema_sha256().as_str(),
                        body.canonical_wire_jcs().as_slice(),
                        body.private_admission_basis_sha256()
                            .map(|digest| digest.as_str()),
                        body.canonical_private_admission_basis_jcs()
                            .map(ControlRecordBytes::as_slice),
                    ],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(recovery)?;
            if !exact {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            SessionLeaseBodyRetention::ExactReplay
        };
        transaction.commit().map_err(|source| {
            ControlStoreError::with_source(ControlStoreErrorKind::OutcomeUnknown, source)
        })?;
        Ok(disposition)
    }

    fn read(
        &self,
        request: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        self.check_session_lease_coordinate(request.store())?;
        let mut connection = self.record_custody_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(storage)?;
        let metadata = transaction
            .query_row(
                "SELECT length(CAST(schema_id AS BLOB)), \
                        length(CAST(schema_sha256 AS BLOB)), length(canonical_wire_jcs), \
                        length(CAST(private_admission_basis_sha256 AS BLOB)), \
                        length(canonical_private_admission_basis_jcs) \
                 FROM pale_deployment_session_lease_body \
                 WHERE session_admission_lease_digest = ?1",
                [request.digest().as_str()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, Option<i64>>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(recovery)?;
        let Some((schema_id_len, schema_sha256_len, byte_len, basis_digest_len, basis_byte_len)) =
            metadata
        else {
            transaction.commit().map_err(storage)?;
            return Ok(None);
        };
        let schema_id_len = usize::try_from(schema_id_len)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        let schema_sha256_len = usize::try_from(schema_sha256_len)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        let byte_len = usize::try_from(byte_len)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        let basis_digest_len = basis_digest_len
            .map(usize::try_from)
            .transpose()
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        let basis_byte_len = basis_byte_len
            .map(usize::try_from)
            .transpose()
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        if !(1..=MAX_SESSION_LEASE_SCHEMA_ID_BYTES).contains(&schema_id_len)
            || schema_sha256_len != SESSION_LEASE_SCHEMA_DIGEST_BYTES
            || byte_len == 0
            || basis_digest_len.is_some() != basis_byte_len.is_some()
            || basis_digest_len.is_some_and(|length| length != SESSION_LEASE_SCHEMA_DIGEST_BYTES)
            || basis_byte_len.is_some_and(|length| length == 0)
            || byte_len
                .checked_add(basis_byte_len.unwrap_or_default())
                .is_none_or(|total| total > request.maximum_body_bytes())
        {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        let (schema_id, schema_sha256, bytes, basis_digest, basis_bytes) = transaction
            .query_row(
                "SELECT schema_id, schema_sha256, canonical_wire_jcs, \
                        private_admission_basis_sha256, canonical_private_admission_basis_jcs \
                 FROM pale_deployment_session_lease_body \
                 WHERE session_admission_lease_digest = ?1",
                [request.digest().as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<Vec<u8>>>(4)?,
                    ))
                },
            )
            .map_err(recovery)?;
        if schema_id.len() != schema_id_len
            || schema_sha256.len() != schema_sha256_len
            || bytes.len() != byte_len
            || basis_digest.as_ref().map(String::len) != basis_digest_len
            || basis_bytes.as_ref().map(Vec::len) != basis_byte_len
        {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        let mut body = SessionLeaseBody::new(
            request.store().clone(),
            request.digest().clone(),
            SchemaId::parse(schema_id).map_err(recovery)?,
            SchemaDigest::parse(schema_sha256).map_err(recovery)?,
            ControlRecordBytes::new(bytes).map_err(recovery)?,
        );
        match (basis_digest, basis_bytes) {
            (Some(digest), Some(bytes)) => {
                let expected = ArtifactDigest::parse(digest).map_err(recovery)?;
                body = body.with_private_admission_basis(
                    ControlRecordBytes::new(bytes).map_err(recovery)?,
                );
                if body.private_admission_basis_sha256() != Some(&expected) {
                    return Err(ControlStoreError::new(
                        ControlStoreErrorKind::RecoveryRequired,
                    ));
                }
            }
            (None, None) => {}
            _ => {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
        }
        transaction.commit().map_err(storage)?;
        Ok(Some(body))
    }
}

/// Preserve native `SQLite` context for failures before commit ambiguity.
fn storage(source: impl std::error::Error + Send + Sync + 'static) -> ControlStoreError {
    ControlStoreError::with_source(ControlStoreErrorKind::Storage, source)
}

/// Preserve native decode/query context for contradictory retained custody.
fn recovery(source: impl std::error::Error + Send + Sync + 'static) -> ControlStoreError {
    ControlStoreError::with_source(ControlStoreErrorKind::RecoveryRequired, source)
}

#[cfg(test)]
#[path = "../tests/internal/session_lease_custody.rs"]
mod tests;
