//! Shared physical operations over principal-trust records and checkpoint heads.
//!
//! These helpers compare opaque admitted bytes and exact hard-frontier
//! coordinates. They do not decode records, construct accumulator roots, or
//! authorize the mutation paired with a checkpoint advance.

use pale_ports::{
    ControlCheckpointHead, ControlRecord, ControlStoreError, ControlStoreErrorKind,
    PrincipalTrustCheckpointHead, PrincipalTrustStoreBinding,
};
use rusqlite::{params, Connection};

pub(crate) mod checkpoint;
pub(crate) mod read;
pub(crate) use read::read_record;

use crate::error::{sqlite, sqlite_integer};

/// Project only the physical coordinate from the already selected principal binding.
pub(crate) fn store_coordinate(
    binding: &PrincipalTrustStoreBinding,
) -> pale_ports::AuthorityStoreCoordinate {
    pale_ports::AuthorityStoreCoordinate::new(
        binding.trust_store_id().clone(),
        binding.trust_store_incarnation_id().clone(),
    )
}

/// Compare the singleton principal-trust-checkpoint head exactly.
pub(crate) fn trust_checkpoint_head_matches(
    connection: &Connection,
    checkpoint: &PrincipalTrustCheckpointHead,
) -> Result<bool, ControlStoreError> {
    let epoch = sqlite_integer(checkpoint.checkpoint_epoch().get())?;
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_trust_checkpoint_head \
             WHERE singleton = 1 AND principal_id = ?1 AND checkpoint_epoch = ?2 \
               AND trust_checkpoint_ref = ?3 AND trust_checkpoint_sha256 = ?4)",
            params![
                checkpoint.principal_id().as_str(),
                epoch,
                checkpoint.coordinate().record_ref().as_str(),
                checkpoint.coordinate().record_sha256().as_str(),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare principal trust-checkpoint head", source))
}

/// Advance the singleton principal-trust frontier by one admitted epoch.
pub(crate) fn advance_trust_checkpoint_head(
    connection: &Connection,
    expected: &PrincipalTrustCheckpointHead,
    next: &PrincipalTrustCheckpointHead,
) -> Result<(), ControlStoreError> {
    if !next.is_exact_successor_of(expected) {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    let expected_epoch = sqlite_integer(expected.checkpoint_epoch().get())?;
    let next_epoch = sqlite_integer(next.checkpoint_epoch().get())?;
    let changed = connection
        .execute(
            "UPDATE pale_principal_trust_checkpoint_head \
             SET checkpoint_epoch = ?1, trust_checkpoint_ref = ?2, \
                 trust_checkpoint_sha256 = ?3 \
             WHERE singleton = 1 AND principal_id = ?4 AND checkpoint_epoch = ?5 \
               AND trust_checkpoint_ref = ?6 AND trust_checkpoint_sha256 = ?7",
            params![
                next_epoch,
                next.coordinate().record_ref().as_str(),
                next.coordinate().record_sha256().as_str(),
                expected.principal_id().as_str(),
                expected_epoch,
                expected.coordinate().record_ref().as_str(),
                expected.coordinate().record_sha256().as_str(),
            ],
        )
        .map_err(|source| sqlite("advance principal trust-checkpoint head", source))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))
    }
}

/// Compare the initialized store header and principal binding exactly.
pub(crate) fn binding_matches(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 \
             FROM pale_control_store_header AS header \
             JOIN pale_principal_trust_store_binding AS binding USING (singleton) \
             WHERE header.singleton = 1 AND header.store_kind = 'principal_trust' \
               AND header.logical_store_id = ?1 AND header.control_incarnation_id = ?2 \
               AND binding.principal_id = ?3)",
            params![
                binding.trust_store_id().as_str(),
                binding.trust_store_incarnation_id().as_str(),
                binding.principal_id().as_str()
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("read principal trust-store binding", source))
}

/// Compare one principal control-checkpoint head projection exactly.
pub(crate) fn control_checkpoint_head_matches(
    connection: &Connection,
    checkpoint: &ControlCheckpointHead,
) -> Result<bool, ControlStoreError> {
    if crate::store::transaction::record_stamp(
        connection,
        crate::ControlStoreKind::PrincipalTrust,
        checkpoint.coordinate(),
    )? != Some(checkpoint.committed_transaction_sequence())
    {
        return Ok(false);
    }
    let epoch = sqlite_integer(checkpoint.domain_epoch().get())?;
    let sequence = sqlite_integer(checkpoint.domain_sequence().get())?;
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_control_checkpoint_head \
             WHERE checkpoint_domain_id = ?1 AND domain_epoch = ?2 \
               AND domain_sequence = ?3 AND control_checkpoint_ref = ?4 \
               AND control_checkpoint_sha256 = ?5)",
            params![
                checkpoint.checkpoint_domain_id().as_str(),
                epoch,
                sequence,
                checkpoint.coordinate().record_ref().as_str(),
                checkpoint.coordinate().record_sha256().as_str(),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare principal control-checkpoint head", source))
}

/// Advance one exact principal hard-conflict domain by one admitted sequence.
pub(crate) fn advance_control_checkpoint_head(
    connection: &Connection,
    expected: &ControlCheckpointHead,
    next: &ControlCheckpointHead,
) -> Result<(), ControlStoreError> {
    if !next.is_exact_successor_of(expected) {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    let expected_epoch = sqlite_integer(expected.domain_epoch().get())?;
    let expected_sequence = sqlite_integer(expected.domain_sequence().get())?;
    let next_epoch = sqlite_integer(next.domain_epoch().get())?;
    let next_sequence = sqlite_integer(next.domain_sequence().get())?;
    let changed = connection
        .execute(
            "UPDATE pale_principal_control_checkpoint_head \
             SET domain_epoch = ?1, domain_sequence = ?2, control_checkpoint_ref = ?3, \
                 control_checkpoint_sha256 = ?4 \
             WHERE checkpoint_domain_id = ?5 AND domain_epoch = ?6 AND domain_sequence = ?7 \
               AND control_checkpoint_ref = ?8 AND control_checkpoint_sha256 = ?9",
            params![
                next_epoch,
                next_sequence,
                next.coordinate().record_ref().as_str(),
                next.coordinate().record_sha256().as_str(),
                expected.checkpoint_domain_id().as_str(),
                expected_epoch,
                expected_sequence,
                expected.coordinate().record_ref().as_str(),
                expected.coordinate().record_sha256().as_str(),
            ],
        )
        .map_err(|source| sqlite("advance principal control-checkpoint head", source))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))
    }
}

/// Insert one application-admitted opaque trust record.
pub(crate) fn insert_record(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    record: &ControlRecord,
) -> Result<(), ControlStoreError> {
    connection
        .execute(
            "INSERT INTO pale_principal_trust_record \
             (record_ref, record_sha256, schema_id, schema_sha256, \
              principal_id, canonical_record_jcs, inserted_transaction_sequence) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, \
              (SELECT logical_transaction_sequence FROM pale_control_store_header WHERE singleton = 1))",
            params![
                record.coordinate().record_ref().as_str(),
                record.coordinate().record_sha256().as_str(),
                record.schema_id().as_str(),
                record.schema_sha256().as_str(),
                binding.principal_id().as_str(),
                record.canonical_record_jcs().as_slice(),
            ],
        )
        .map(drop)
        .map_err(|source| sqlite("insert principal trust record", source))
}

/// Compare one retained record byte-for-byte without decoding it.
pub(crate) fn record_matches(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    record: &ControlRecord,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_trust_record \
             WHERE record_ref = ?1 AND record_sha256 = ?2 \
               AND schema_id = ?3 AND schema_sha256 = ?4 AND principal_id = ?5 \
               AND canonical_record_jcs = ?6)",
            params![
                record.coordinate().record_ref().as_str(),
                record.coordinate().record_sha256().as_str(),
                record.schema_id().as_str(),
                record.schema_sha256().as_str(),
                binding.principal_id().as_str(),
                record.canonical_record_jcs().as_slice(),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare principal trust record", source))
}
