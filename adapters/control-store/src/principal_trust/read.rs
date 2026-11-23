//! Bounded physical trust-head discovery. Stored bytes remain unadmitted.

use pale_core::records::{
    ControlDomainEpochV1, ControlDomainSequenceV1, ControlTransactionSequenceV1,
    PrincipalTrustCheckpointEpochV1, ResidentRecordRefV1, SchemaDigest, SchemaId,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA, MAX_RESIDENT_ID_BYTES,
    PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
};
use pale_ports::{
    ControlCheckpointHead, ControlRecord, ControlRecordBytes, ControlRecordCoordinate,
    ControlRecordDigest, ControlStoreError, ControlStoreErrorKind,
    PrincipalControlCheckpointChainReadLimits, PrincipalControlCheckpointChainReadPort,
    PrincipalControlCheckpointChainSnapshot, PrincipalControlCheckpointStoredRecord,
    PrincipalTrustCheckpointHead, PrincipalTrustCheckpointRecord,
    PrincipalTrustCheckpointSetReadLimits, PrincipalTrustCheckpointSetReadPort,
    PrincipalTrustCheckpointSetSnapshot, PrincipalTrustReadPort, PrincipalTrustSnapshot,
    PrincipalTrustStoreBinding,
};
use rusqlite::{types::ValueRef, Connection, OptionalExtension, Transaction, TransactionBehavior};

use super::{binding_matches, store_coordinate};
use crate::error::sqlite;

/// Read-only principal trust custody over an already opened native connection.
///
/// Construction is crate-private until native runtime assembly admits the store.
/// Reading a head does not admit a signature, trust root, or independent anchor.
pub struct SqlitePrincipalTrustReader<'connection> {
    pub(crate) connection: &'connection mut Connection,
}

impl PrincipalControlCheckpointChainReadPort for SqlitePrincipalTrustReader<'_> {
    fn read_control_checkpoint_chain(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        initial: &ControlCheckpointHead,
        limits: PrincipalControlCheckpointChainReadLimits,
    ) -> Result<Option<PrincipalControlCheckpointChainSnapshot>, ControlStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin principal control-checkpoint chain read", source))?;
        if !binding_matches(&transaction, binding)? {
            return Err(recovery());
        }
        let observed_through =
            crate::store::transaction::read(&transaction, &store_coordinate(binding))?;
        let summary = control_checkpoint_chain_summary(
            &transaction,
            binding,
            initial.checkpoint_domain_id().as_str(),
        )?;
        if summary.invalid_family_rows != 0
            || summary.maximum_inserted_transaction > observed_through.get()
        {
            return Err(recovery());
        }
        if summary.domain_rows > limits.maximum_checkpoints()
            || summary.total_body_bytes > limits.maximum_record_bytes()
        {
            return Err(bound_exceeded());
        }
        let current = read_control_checkpoint_head(
            &transaction,
            initial.checkpoint_domain_id().as_str(),
            observed_through,
        )?;
        let Some(current) = current else {
            if summary.domain_rows != 0 || summary.total_body_bytes != 0 {
                return Err(recovery());
            }
            transaction.rollback().map_err(|source| {
                sqlite(
                    "release absent principal control-checkpoint chain read",
                    source,
                )
            })?;
            return Ok(None);
        };
        let coordinates = control_checkpoint_coordinates(
            &transaction,
            binding,
            initial.checkpoint_domain_id().as_str(),
        )?;
        if coordinates.len() != summary.domain_rows {
            return Err(recovery());
        }
        let mut records = Vec::with_capacity(coordinates.len());
        let mut copied = 0usize;
        for (inserted_in, coordinate) in coordinates {
            let remaining = limits
                .maximum_record_bytes()
                .checked_sub(copied)
                .ok_or_else(bound_exceeded)?;
            let record = read_record_bounded(&transaction, binding, &coordinate, remaining)?
                .ok_or_else(recovery)?;
            copied = copied
                .checked_add(record.canonical_record_jcs().as_slice().len())
                .ok_or_else(bound_exceeded)?;
            records.push(PrincipalControlCheckpointStoredRecord::new(
                inserted_in,
                record,
            ));
        }
        if copied != summary.total_body_bytes {
            return Err(recovery());
        }
        let snapshot = PrincipalControlCheckpointChainSnapshot::new(
            binding.clone(),
            initial.clone(),
            current,
            observed_through,
            records.into_boxed_slice(),
        )
        .map_err(|_| recovery())?;
        transaction
            .rollback()
            .map_err(|source| sqlite("release principal control-checkpoint chain read", source))?;
        Ok(Some(snapshot))
    }
}

impl PrincipalTrustCheckpointSetReadPort for SqlitePrincipalTrustReader<'_> {
    fn read_checkpoint_set(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        limits: PrincipalTrustCheckpointSetReadLimits,
    ) -> Result<Option<PrincipalTrustCheckpointSetSnapshot>, ControlStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin principal trust checkpoint-set read", source))?;
        if !binding_matches(&transaction, binding)? {
            return Err(recovery());
        }
        let observed_through =
            crate::store::transaction::read(&transaction, &store_coordinate(binding))?;
        let summary = checkpoint_set_summary(&transaction, binding)?;
        if summary.total_family_rows != summary.bound_principal_rows
            || summary.invalid_transaction_rows != 0
            || summary.maximum_inserted_transaction > observed_through.get()
        {
            return Err(recovery());
        }
        if summary.bound_principal_rows > limits.maximum_checkpoints()
            || summary.total_body_bytes > limits.maximum_record_bytes()
        {
            return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
        }
        let current_head =
            read_current_checkpoint_head_in(&transaction, binding, observed_through)?;
        let Some(current_head) = current_head else {
            if summary.bound_principal_rows != 0 || summary.total_body_bytes != 0 {
                return Err(recovery());
            }
            transaction.rollback().map_err(|source| {
                sqlite("release absent principal trust checkpoint-set read", source)
            })?;
            return Ok(None);
        };
        let coordinates = checkpoint_coordinates(&transaction, binding)?;
        if coordinates.len() != summary.bound_principal_rows {
            return Err(recovery());
        }
        let mut records = Vec::with_capacity(coordinates.len());
        let mut copied = 0usize;
        for (inserted_in, coordinate) in coordinates {
            let remaining = limits
                .maximum_record_bytes()
                .checked_sub(copied)
                .ok_or_else(bound_exceeded)?;
            let record = read_record_bounded(&transaction, binding, &coordinate, remaining)?
                .ok_or_else(recovery)?;
            copied = copied
                .checked_add(record.canonical_record_jcs().as_slice().len())
                .ok_or_else(bound_exceeded)?;
            records.push(PrincipalTrustCheckpointRecord::new(inserted_in, record));
        }
        if copied != summary.total_body_bytes {
            return Err(recovery());
        }
        let checkpoint_set = PrincipalTrustCheckpointSetSnapshot::new(
            binding.clone(),
            current_head,
            observed_through,
            records.into_boxed_slice(),
        )
        .map_err(|_| recovery())?;
        transaction
            .rollback()
            .map_err(|source| sqlite("release principal trust checkpoint-set read", source))?;
        Ok(Some(checkpoint_set))
    }
}

impl PrincipalTrustReadPort for SqlitePrincipalTrustReader<'_> {
    fn read_current_checkpoint(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        maximum_body_bytes: usize,
    ) -> Result<Option<PrincipalTrustSnapshot>, ControlStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin principal trust read", source))?;
        let snapshot = read_current_checkpoint_in(&transaction, binding, maximum_body_bytes)?;
        transaction
            .rollback()
            .map_err(|source| sqlite("release principal trust read", source))?;
        Ok(snapshot)
    }

    fn read_record(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin principal trust record read", source))?;
        if !binding_matches(&transaction, binding)? {
            return Err(recovery());
        }
        let through_transaction =
            crate::store::transaction::read(&transaction, &store_coordinate(binding))?;
        let record = read_record_bounded(&transaction, binding, coordinate, maximum_body_bytes)?;
        if record.is_some() {
            let inserted = crate::store::transaction::record_stamp(
                &transaction,
                crate::ControlStoreKind::PrincipalTrust,
                coordinate,
            )?
            .ok_or_else(recovery)?;
            if inserted.get() > through_transaction.get() {
                return Err(recovery());
            }
        }
        transaction
            .rollback()
            .map_err(|source| sqlite("release principal trust record read", source))?;
        Ok(record)
    }
}

/// Read one coherent head/body association inside the caller's read transaction.
///
/// The head's insertion stamp is positive and no later than the observed store
/// transaction. It need not equal that transaction: unrelated domains can advance.
/// A trust checkpoint has no signed transaction scalar to compare here.
pub(crate) fn read_current_checkpoint_in(
    transaction: &Transaction<'_>,
    binding: &PrincipalTrustStoreBinding,
    maximum_body_bytes: usize,
) -> Result<Option<PrincipalTrustSnapshot>, ControlStoreError> {
    if !binding_matches(transaction, binding)? {
        return Err(recovery());
    }
    let through_transaction =
        crate::store::transaction::read(transaction, &store_coordinate(binding))?;
    let Some(head) = read_current_checkpoint_head_in(transaction, binding, through_transaction)?
    else {
        return Ok(None);
    };
    let record = read_record_bounded(transaction, binding, head.coordinate(), maximum_body_bytes)?
        .ok_or_else(recovery)?;
    if record.schema_id().as_str() != PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA {
        return Err(recovery());
    }
    PrincipalTrustSnapshot::new(binding.clone(), head, record)
        .map(Some)
        .map_err(|_| recovery())
}

/// Read one current trust head without copying its immutable record body.
fn read_current_checkpoint_head_in(
    transaction: &Transaction<'_>,
    binding: &PrincipalTrustStoreBinding,
    observed_through: ControlTransactionSequenceV1,
) -> Result<Option<PrincipalTrustCheckpointHead>, ControlStoreError> {
    let head = transaction
        .query_row(
            "SELECT principal_id, checkpoint_epoch, trust_checkpoint_ref, trust_checkpoint_sha256 \
         FROM pale_principal_trust_checkpoint_head WHERE singleton = 1",
            [],
            |row| {
                let principal = row.get_ref(0)?;
                let epoch = row.get_ref(1)?;
                let reference = row.get_ref(2)?;
                let digest = row.get_ref(3)?;
                Ok((|| {
                    if text(principal, MAX_RESIDENT_ID_BYTES)? != binding.principal_id().as_str() {
                        return Err(recovery());
                    }
                    let ValueRef::Integer(epoch) = epoch else {
                        return Err(recovery());
                    };
                    let epoch = PrincipalTrustCheckpointEpochV1::new(
                        u64::try_from(epoch).map_err(|_| recovery())?,
                    )
                    .map_err(|_| recovery())?;
                    let reference = ResidentRecordRefV1::parse(
                        text(reference, MAX_RESIDENT_ID_BYTES)?.to_owned(),
                    )
                    .map_err(|_| recovery())?;
                    let digest = ControlRecordDigest::parse(text(digest, 64)?.to_owned())
                        .map_err(|_| recovery())?;
                    Ok(PrincipalTrustCheckpointHead::new(
                        binding.principal_id().clone(),
                        epoch,
                        ControlRecordCoordinate::new(reference, digest),
                    ))
                })())
            },
        )
        .optional()
        .map_err(|source| sqlite("read principal trust head", source))?
        .transpose()?;
    let Some(head) = head else {
        return Ok(None);
    };
    let stamp = crate::store::transaction::record_stamp(
        transaction,
        crate::ControlStoreKind::PrincipalTrust,
        head.coordinate(),
    )?
    .ok_or_else(recovery)?;
    if stamp > observed_through {
        return Err(recovery());
    }
    Ok(Some(head))
}

/// Read one existing record through the shared physical decoder.
///
/// Existing producer point reads have no body-budget argument. They retain that
/// behavior; bounded trust discovery supplies its explicit limit to the same owner.
pub(crate) fn read_record(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    coordinate: &ControlRecordCoordinate,
) -> Result<Option<ControlRecord>, ControlStoreError> {
    read_record_bounded(connection, binding, coordinate, usize::MAX)
}

/// Check borrowed metadata and body length before creating any owned copies.
///
/// All fields come from one `SQLite` row. The Rust body copy is bounded; `SQLite`'s
/// internal page loading and memory are not. No raw-body hash is substituted for
/// a signed record's owner-defined digest, and no JSON/signature admission occurs.
pub(crate) fn read_record_bounded(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    coordinate: &ControlRecordCoordinate,
    maximum_body_bytes: usize,
) -> Result<Option<ControlRecord>, ControlStoreError> {
    connection
        .query_row(
            "SELECT principal_id, record_sha256, schema_id, schema_sha256, canonical_record_jcs \
         FROM pale_principal_trust_record WHERE record_ref = ?1",
            [coordinate.record_ref().as_str()],
            |row| {
                let principal = row.get_ref(0)?;
                let digest = row.get_ref(1)?;
                let schema = row.get_ref(2)?;
                let schema_digest = row.get_ref(3)?;
                let body = row.get_ref(4)?;
                Ok((|| {
                    if text(principal, MAX_RESIDENT_ID_BYTES)? != binding.principal_id().as_str()
                        || text(digest, 64)? != coordinate.record_sha256().as_str()
                    {
                        return Err(recovery());
                    }
                    let schema = text(schema, 512)?;
                    let schema_digest = text(schema_digest, 64)?;
                    let ValueRef::Blob(body) = body else {
                        return Err(recovery());
                    };
                    if body.is_empty() {
                        return Err(recovery());
                    }
                    if body.len() > maximum_body_bytes {
                        return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
                    }
                    let schema = SchemaId::parse(schema.to_owned()).map_err(|_| recovery())?;
                    let schema_digest =
                        SchemaDigest::parse(schema_digest.to_owned()).map_err(|_| recovery())?;
                    let bytes = ControlRecordBytes::new(body.to_vec()).map_err(|_| recovery())?;
                    Ok(ControlRecord::new(
                        coordinate.clone(),
                        schema,
                        schema_digest,
                        bytes,
                    ))
                })())
            },
        )
        .optional()
        .map_err(|source| sqlite("read principal trust record", source))?
        .transpose()
}

/// Check the caller's UTF-8 byte ceiling before copying stored text.
fn text(value: ValueRef<'_>, maximum_bytes: usize) -> Result<&str, ControlStoreError> {
    let ValueRef::Text(bytes) = value else {
        return Err(recovery());
    };
    if bytes.is_empty() || bytes.len() > maximum_bytes {
        return Err(recovery());
    }
    std::str::from_utf8(bytes).map_err(|_| recovery())
}

fn recovery() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}

fn bound_exceeded() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::BoundExceeded)
}

#[derive(Clone, Copy)]
struct PhysicalCheckpointSetSummary {
    total_family_rows: usize,
    bound_principal_rows: usize,
    total_body_bytes: usize,
    invalid_transaction_rows: usize,
    maximum_inserted_transaction: u64,
}

fn checkpoint_set_summary(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
) -> Result<PhysicalCheckpointSetSummary, ControlStoreError> {
    connection
        .query_row(
            "SELECT COUNT(*), \
                    COALESCE(SUM(CASE WHEN principal_id=?1 THEN 1 ELSE 0 END),0), \
                    COALESCE(SUM(CASE WHEN principal_id=?1 THEN length(canonical_record_jcs) ELSE 0 END),0), \
                    COALESCE(SUM(CASE WHEN principal_id=?1 AND inserted_transaction_sequence<=0 THEN 1 ELSE 0 END),0), \
                    COALESCE(MAX(CASE WHEN principal_id=?1 THEN inserted_transaction_sequence ELSE 0 END),0) \
               FROM pale_principal_trust_record WHERE schema_id=?2",
            rusqlite::params![
                binding.principal_id().as_str(),
                PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .map_err(|source| sqlite("summarize principal trust checkpoint set", source))
        .and_then(|(total, bound, bytes, invalid_transactions, transaction)| {
            Ok(PhysicalCheckpointSetSummary {
                total_family_rows: usize::try_from(total).map_err(|_| recovery())?,
                bound_principal_rows: usize::try_from(bound).map_err(|_| recovery())?,
                total_body_bytes: usize::try_from(bytes).map_err(|_| recovery())?,
                invalid_transaction_rows: usize::try_from(invalid_transactions)
                    .map_err(|_| recovery())?,
                maximum_inserted_transaction: u64::try_from(transaction)
                    .map_err(|_| recovery())?,
            })
        })
}

fn checkpoint_coordinates(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
) -> Result<Vec<(ControlTransactionSequenceV1, ControlRecordCoordinate)>, ControlStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT inserted_transaction_sequence,record_ref,record_sha256 FROM pale_principal_trust_record \
              WHERE principal_id=?1 AND schema_id=?2 \
              ORDER BY inserted_transaction_sequence,record_ref",
        )
        .map_err(|source| sqlite("prepare principal trust checkpoint coordinates", source))?;
    let rows = statement
        .query_map(
            rusqlite::params![
                binding.principal_id().as_str(),
                PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA
            ],
            |row| {
                let inserted_in = row.get_ref(0)?;
                let reference = row.get_ref(1)?;
                let digest = row.get_ref(2)?;
                Ok((|| {
                    let ValueRef::Integer(inserted_in) = inserted_in else {
                        return Err(recovery());
                    };
                    let inserted_in = ControlTransactionSequenceV1::new(
                        u64::try_from(inserted_in).map_err(|_| recovery())?,
                    )
                    .map_err(|_| recovery())?;
                    Ok((
                        inserted_in,
                        ControlRecordCoordinate::new(
                            ResidentRecordRefV1::parse(
                                text(reference, MAX_RESIDENT_ID_BYTES)?.to_owned(),
                            )
                            .map_err(|_| recovery())?,
                            ControlRecordDigest::parse(text(digest, 64)?.to_owned())
                                .map_err(|_| recovery())?,
                        ),
                    ))
                })())
            },
        )
        .map_err(|source| sqlite("read principal trust checkpoint coordinates", source))?;
    rows.map(|row| {
        row.map_err(|source| sqlite("decode principal trust checkpoint coordinate", source))?
    })
    .collect()
}

#[derive(Clone, Copy)]
struct PhysicalControlCheckpointChainSummary {
    domain_rows: usize,
    total_body_bytes: usize,
    invalid_family_rows: usize,
    maximum_inserted_transaction: u64,
}

/// Summarize the complete selected family before any record body reaches Rust.
///
/// An invalid JSON row cannot safely be classified as another domain, so it is
/// recovery evidence rather than absence. `SQLite` evaluates lengths and the
/// domain discriminator without allocating one Rust buffer per record.
fn control_checkpoint_chain_summary(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    checkpoint_domain_id: &str,
) -> Result<PhysicalControlCheckpointChainSummary, ControlStoreError> {
    connection
        .query_row(
            "SELECT \
                COALESCE(SUM(CASE WHEN principal_id=?1 AND json_valid(CAST(canonical_record_jcs AS TEXT))=1 \
                    AND json_extract(CAST(canonical_record_jcs AS TEXT),'$.checkpoint_domain_id')=?2 THEN 1 ELSE 0 END),0), \
                COALESCE(SUM(CASE WHEN principal_id=?1 AND json_valid(CAST(canonical_record_jcs AS TEXT))=1 \
                    AND json_extract(CAST(canonical_record_jcs AS TEXT),'$.checkpoint_domain_id')=?2 THEN length(canonical_record_jcs) ELSE 0 END),0), \
                COALESCE(SUM(CASE WHEN principal_id<>?1 \
                    OR json_valid(CAST(canonical_record_jcs AS TEXT))<>1 \
                    OR COALESCE(json_type(CAST(canonical_record_jcs AS TEXT),'$.checkpoint_domain_id'),'')<>'text' \
                    THEN 1 ELSE 0 END),0), \
                COALESCE(MAX(CASE WHEN principal_id=?1 AND json_valid(CAST(canonical_record_jcs AS TEXT))=1 \
                    AND json_extract(CAST(canonical_record_jcs AS TEXT),'$.checkpoint_domain_id')=?2 THEN inserted_transaction_sequence ELSE 0 END),0) \
             FROM pale_principal_trust_record WHERE schema_id=?3",
            rusqlite::params![
                binding.principal_id().as_str(),
                checkpoint_domain_id,
                CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .map_err(|source| sqlite("summarize principal control-checkpoint chain", source))
        .and_then(|(rows, bytes, invalid, maximum_transaction)| {
            Ok(PhysicalControlCheckpointChainSummary {
                domain_rows: usize::try_from(rows).map_err(|_| recovery())?,
                total_body_bytes: usize::try_from(bytes).map_err(|_| recovery())?,
                invalid_family_rows: usize::try_from(invalid).map_err(|_| recovery())?,
                maximum_inserted_transaction: u64::try_from(maximum_transaction)
                    .map_err(|_| recovery())?,
            })
        })
}

fn read_control_checkpoint_head(
    connection: &Connection,
    checkpoint_domain_id: &str,
    observed_through: ControlTransactionSequenceV1,
) -> Result<Option<ControlCheckpointHead>, ControlStoreError> {
    let row = connection
        .query_row(
            "SELECT domain_epoch,domain_sequence,control_checkpoint_ref,control_checkpoint_sha256 \
             FROM pale_principal_control_checkpoint_head WHERE checkpoint_domain_id=?1",
            [checkpoint_domain_id],
            |row| {
                let epoch = row.get_ref(0)?;
                let sequence = row.get_ref(1)?;
                let reference = row.get_ref(2)?;
                let digest = row.get_ref(3)?;
                Ok((|| {
                    let ValueRef::Integer(epoch) = epoch else {
                        return Err(recovery());
                    };
                    let ValueRef::Integer(sequence) = sequence else {
                        return Err(recovery());
                    };
                    Ok((
                        ControlDomainEpochV1::new(u64::try_from(epoch).map_err(|_| recovery())?)
                            .map_err(|_| recovery())?,
                        ControlDomainSequenceV1::new(
                            u64::try_from(sequence).map_err(|_| recovery())?,
                        )
                        .map_err(|_| recovery())?,
                        ControlRecordCoordinate::new(
                            ResidentRecordRefV1::parse(
                                text(reference, MAX_RESIDENT_ID_BYTES)?.to_owned(),
                            )
                            .map_err(|_| recovery())?,
                            ControlRecordDigest::parse(text(digest, 64)?.to_owned())
                                .map_err(|_| recovery())?,
                        ),
                    ))
                })())
            },
        )
        .optional()
        .map_err(|source| sqlite("read principal control-checkpoint head", source))?
        .transpose()?;
    let Some((epoch, sequence, coordinate)) = row else {
        return Ok(None);
    };
    let inserted_in = crate::store::transaction::record_stamp(
        connection,
        crate::ControlStoreKind::PrincipalTrust,
        &coordinate,
    )?
    .ok_or_else(recovery)?;
    if inserted_in > observed_through {
        return Err(recovery());
    }
    Ok(Some(ControlCheckpointHead::new(
        pale_core::records::ControlCheckpointDomainIdV1::parse(checkpoint_domain_id.to_owned())
            .map_err(|_| recovery())?,
        epoch,
        sequence,
        inserted_in,
        coordinate,
    )))
}

fn control_checkpoint_coordinates(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    checkpoint_domain_id: &str,
) -> Result<Vec<(ControlTransactionSequenceV1, ControlRecordCoordinate)>, ControlStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT inserted_transaction_sequence,record_ref,record_sha256 \
             FROM pale_principal_trust_record WHERE principal_id=?1 AND schema_id=?2 \
               AND json_valid(CAST(canonical_record_jcs AS TEXT))=1 \
               AND json_extract(CAST(canonical_record_jcs AS TEXT),'$.checkpoint_domain_id')=?3 \
             ORDER BY inserted_transaction_sequence,record_ref",
        )
        .map_err(|source| sqlite("prepare principal control-checkpoint coordinates", source))?;
    let rows = statement
        .query_map(
            rusqlite::params![
                binding.principal_id().as_str(),
                CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
                checkpoint_domain_id,
            ],
            |row| {
                let inserted = row.get_ref(0)?;
                let reference = row.get_ref(1)?;
                let digest = row.get_ref(2)?;
                Ok((|| {
                    let ValueRef::Integer(inserted) = inserted else {
                        return Err(recovery());
                    };
                    Ok((
                        ControlTransactionSequenceV1::new(
                            u64::try_from(inserted).map_err(|_| recovery())?,
                        )
                        .map_err(|_| recovery())?,
                        ControlRecordCoordinate::new(
                            ResidentRecordRefV1::parse(
                                text(reference, MAX_RESIDENT_ID_BYTES)?.to_owned(),
                            )
                            .map_err(|_| recovery())?,
                            ControlRecordDigest::parse(text(digest, 64)?.to_owned())
                                .map_err(|_| recovery())?,
                        ),
                    ))
                })())
            },
        )
        .map_err(|source| sqlite("read principal control-checkpoint coordinates", source))?;
    rows.map(|row| {
        row.map_err(|source| sqlite("decode principal control-checkpoint coordinate", source))?
    })
    .collect()
}
