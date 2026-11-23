//! Bounded physical evidence reads used before resident admission.

use pale_core::records::{
    ArtifactDigest, CheckpointProfileDigest, ControlCheckpointDomainIdV1, ControlDomainEpochV1,
    ControlDomainSequenceV1, ControlIncarnationIdV1, ControlStoreIdV1,
    ControlTransactionSequenceV1, DeploymentControlStoreGenesisIdV1,
    PrincipalResidentWriterFenceIdV1, PrincipalResidentWriterFenceStateV1,
    PrincipalResidentWriterGenerationV1, ResidentRecordRefV1, CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
    DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA, MAX_RESIDENT_ID_BYTES,
    PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
};
use pale_ports::{
    AccumulatorCheckpointBinding, AccumulatorCheckpointPhase, AccumulatorMutation,
    AccumulatorPosition, AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord,
    ControlRecordCoordinate, ControlRecordDigest, ControlStoreError, ControlStoreErrorKind,
    ControlTransactionPreparation, DeploymentControlStoreBinding,
    DeploymentControlStoreGenesisReadLimits, DeploymentControlStoreGenesisReadPort,
    DeploymentControlStoreGenesisRecords, DeploymentControlStoreGenesisSnapshot,
    OperationAccumulatorBatch, PrincipalResidentWriterHead, PrincipalResidentWriterReadPort,
    PrincipalResidentWriterRecordSetReadLimits, PrincipalResidentWriterRecordSetReadPort,
    PrincipalResidentWriterRecordSetSnapshot, PrincipalResidentWriterSnapshot,
    PrincipalResidentWriterStoredRecord, PrincipalTrustStoreBinding, StateAccumulatorBatch,
    UnresolvedCustodyAccumulatorBatch,
};
use rusqlite::{params, types::ValueRef, Connection, OptionalExtension, TransactionBehavior};

use crate::error::sqlite;

mod finalization;
pub use finalization::SqliteDeploymentFinalizationSetReader;

const PRINCIPAL_WRITER_CHECKPOINT_DOMAIN: &str = "principal-resident-writer";
const MAX_ACCUMULATOR_NODE_BYTES: usize = 132;

/// Read-only principal-store evidence over an already opened connection.
///
/// Construction is crate-private until resident assembly admits the store.
pub struct SqlitePrincipalColdStartReader<'connection> {
    pub(crate) connection: &'connection mut Connection,
}

impl PrincipalResidentWriterReadPort for SqlitePrincipalColdStartReader<'_> {
    fn read_current_writer(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        maximum_record_bytes: usize,
    ) -> Result<Option<PrincipalResidentWriterSnapshot>, ControlStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin principal writer read", source))?;
        if !crate::principal_trust::binding_matches(&transaction, binding)? {
            return Err(recovery());
        }
        let through = crate::store::transaction::read(
            &transaction,
            &crate::principal_trust::store_coordinate(binding),
        )?;
        let raw = read_writer_head(&transaction, binding)?;
        let Some(raw) = raw else {
            if writer_material_exists(&transaction)? {
                return Err(recovery());
            }
            transaction
                .rollback()
                .map_err(|source| sqlite("release absent principal writer read", source))?;
            return Ok(None);
        };
        if raw.checkpoint_domain.as_str() != PRINCIPAL_WRITER_CHECKPOINT_DOMAIN {
            return Err(recovery());
        }
        let checkpoint_coordinate =
            ControlRecordCoordinate::new(raw.checkpoint_ref.clone(), raw.checkpoint_digest.clone());
        let checkpoint_stamp = crate::store::transaction::record_stamp(
            &transaction,
            crate::ControlStoreKind::PrincipalTrust,
            &checkpoint_coordinate,
        )?
        .ok_or_else(recovery)?;
        if checkpoint_stamp.get() > through.get()
            || !checkpoint_head_matches(&transaction, &raw, &checkpoint_coordinate)?
        {
            return Err(recovery());
        }
        let writer_coordinate =
            ControlRecordCoordinate::new(raw.writer_ref.clone(), raw.writer_digest.clone());
        let writer_stamp = crate::store::transaction::record_stamp(
            &transaction,
            crate::ControlStoreKind::PrincipalTrust,
            &writer_coordinate,
        )?
        .ok_or_else(recovery)?;
        if writer_stamp != checkpoint_stamp {
            return Err(recovery());
        }
        let writer_record = crate::principal_trust::read::read_record_bounded(
            &transaction,
            binding,
            &writer_coordinate,
            maximum_record_bytes,
        )?
        .ok_or_else(recovery)?;
        let remaining = maximum_record_bytes
            .checked_sub(writer_record.canonical_record_jcs().as_slice().len())
            .ok_or_else(bound)?;
        let checkpoint_record = crate::principal_trust::read::read_record_bounded(
            &transaction,
            binding,
            &checkpoint_coordinate,
            remaining,
        )?
        .ok_or_else(recovery)?;
        if writer_record.schema_id().as_str() != PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA
            || checkpoint_record.schema_id().as_str() != CONTROL_STATE_CHECKPOINT_V1_SCHEMA
        {
            return Err(recovery());
        }
        let checkpoint = ControlCheckpointHead::new(
            raw.checkpoint_domain,
            raw.checkpoint_epoch,
            raw.checkpoint_sequence,
            checkpoint_stamp,
            checkpoint_coordinate,
        );
        let head = PrincipalResidentWriterHead::new(
            raw.writer_fence_id,
            binding.principal_id().clone(),
            raw.writer_generation,
            raw.writer_state,
            writer_coordinate,
            checkpoint,
        );
        let snapshot = PrincipalResidentWriterSnapshot::new(
            binding.clone(),
            head,
            writer_record,
            checkpoint_record,
        )
        .map_err(|_| recovery())?;
        transaction
            .rollback()
            .map_err(|source| sqlite("release principal writer read", source))?;
        Ok(Some(snapshot))
    }
}

impl PrincipalResidentWriterRecordSetReadPort for SqlitePrincipalColdStartReader<'_> {
    fn read_writer_record_set(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        limits: PrincipalResidentWriterRecordSetReadLimits,
    ) -> Result<Option<PrincipalResidentWriterRecordSetSnapshot>, ControlStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin principal writer record-set read", source))?;
        if !crate::principal_trust::binding_matches(&transaction, binding)? {
            return Err(recovery());
        }
        let observed_through = crate::store::transaction::read(
            &transaction,
            &crate::principal_trust::store_coordinate(binding),
        )?;
        if !writer_family_rows_match_binding(&transaction, binding)?
            || !writer_checkpoint_rows_match_binding(&transaction, binding)?
        {
            return Err(recovery());
        }
        let Some(raw) = read_writer_head(&transaction, binding)? else {
            if writer_material_exists(&transaction)? {
                return Err(recovery());
            }
            transaction.rollback().map_err(|source| {
                sqlite("release absent principal writer record-set read", source)
            })?;
            return Ok(None);
        };
        if raw.checkpoint_domain.as_str() != PRINCIPAL_WRITER_CHECKPOINT_DOMAIN {
            return Err(recovery());
        }
        let checkpoint_coordinate =
            ControlRecordCoordinate::new(raw.checkpoint_ref.clone(), raw.checkpoint_digest.clone());
        let checkpoint_stamp = crate::store::transaction::record_stamp(
            &transaction,
            crate::ControlStoreKind::PrincipalTrust,
            &checkpoint_coordinate,
        )?
        .ok_or_else(recovery)?;
        let writer_coordinate =
            ControlRecordCoordinate::new(raw.writer_ref.clone(), raw.writer_digest.clone());
        let writer_stamp = crate::store::transaction::record_stamp(
            &transaction,
            crate::ControlStoreKind::PrincipalTrust,
            &writer_coordinate,
        )?
        .ok_or_else(recovery)?;
        if checkpoint_stamp > observed_through
            || writer_stamp != checkpoint_stamp
            || !checkpoint_head_matches(&transaction, &raw, &checkpoint_coordinate)?
        {
            return Err(recovery());
        }
        let writer_coordinates =
            writer_record_coordinates(&transaction, binding, limits.maximum_writer_records())?;
        let checkpoint_coordinates = writer_checkpoint_coordinates(
            &transaction,
            binding,
            limits.maximum_checkpoint_records(),
        )?;
        let mut remaining = limits.maximum_record_bytes();
        let writer_records =
            read_stored_records(&transaction, binding, writer_coordinates, &mut remaining)?;
        let checkpoint_records = read_stored_records(
            &transaction,
            binding,
            checkpoint_coordinates,
            &mut remaining,
        )?;
        let head = PrincipalResidentWriterHead::new(
            raw.writer_fence_id,
            binding.principal_id().clone(),
            raw.writer_generation,
            raw.writer_state,
            writer_coordinate,
            ControlCheckpointHead::new(
                raw.checkpoint_domain,
                raw.checkpoint_epoch,
                raw.checkpoint_sequence,
                checkpoint_stamp,
                checkpoint_coordinate,
            ),
        );
        let snapshot = PrincipalResidentWriterRecordSetSnapshot::new(
            binding.clone(),
            head,
            observed_through,
            writer_records.into_boxed_slice(),
            checkpoint_records.into_boxed_slice(),
        )
        .map_err(|_| recovery())?;
        transaction
            .rollback()
            .map_err(|source| sqlite("release principal writer record-set read", source))?;
        Ok(Some(snapshot))
    }
}

fn writer_record_coordinates(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    maximum_records: usize,
) -> Result<Vec<(ControlTransactionSequenceV1, ControlRecordCoordinate)>, ControlStoreError> {
    record_coordinates(
        connection,
        binding,
        "SELECT inserted_transaction_sequence,record_ref,record_sha256 \
         FROM pale_principal_trust_record WHERE principal_id=?1 \
         AND schema_id IN (?2,?3,?4) ORDER BY inserted_transaction_sequence,record_ref LIMIT ?5",
        maximum_records,
        "read principal writer record coordinates",
    )
}

fn writer_family_rows_match_binding(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT COUNT(*),COALESCE(SUM(CASE WHEN principal_id=?1 THEN 1 ELSE 0 END),0) \
             FROM pale_principal_trust_record WHERE schema_id IN (?2,?3,?4)",
            params![
                binding.principal_id().as_str(),
                PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
                pale_core::records::PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA,
                pale_core::records::PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA,
            ],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map(|(total, bound)| total >= 0 && total == bound)
        .map_err(|source| sqlite("compare principal writer record ownership", source))
}

fn writer_checkpoint_rows_match_binding(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT COUNT(*),COALESCE(SUM(CASE WHEN checkpoint.principal_id=?1 THEN 1 ELSE 0 END),0) \
             FROM pale_principal_trust_record AS checkpoint \
             WHERE checkpoint.schema_id=?2 \
             AND EXISTS (SELECT 1 FROM pale_principal_trust_record AS writer \
               WHERE writer.schema_id IN (?3,?4,?5) \
               AND writer.inserted_transaction_sequence=checkpoint.inserted_transaction_sequence)",
            params![
                binding.principal_id().as_str(),
                CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
                PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
                pale_core::records::PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA,
                pale_core::records::PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA,
            ],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map(|(total, bound)| total >= 0 && total == bound)
        .map_err(|source| sqlite("compare principal writer checkpoint ownership", source))
}

fn writer_checkpoint_coordinates(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    maximum_records: usize,
) -> Result<Vec<(ControlTransactionSequenceV1, ControlRecordCoordinate)>, ControlStoreError> {
    let limit = maximum_records.checked_add(1).ok_or_else(bound)?;
    let mut statement = connection
        .prepare(
            "SELECT checkpoint.inserted_transaction_sequence,checkpoint.record_ref,checkpoint.record_sha256 \
             FROM pale_principal_trust_record AS checkpoint \
             WHERE checkpoint.principal_id=?1 AND checkpoint.schema_id=?2 \
             AND EXISTS (SELECT 1 FROM pale_principal_trust_record AS writer \
               WHERE writer.principal_id=?1 \
               AND writer.schema_id IN (?3,?4,?5) \
               AND writer.inserted_transaction_sequence=checkpoint.inserted_transaction_sequence) \
             ORDER BY checkpoint.inserted_transaction_sequence,checkpoint.record_ref LIMIT ?6",
        )
        .map_err(|source| sqlite("prepare principal writer checkpoint coordinates", source))?;
    let rows = statement
        .query_map(
            params![
                binding.principal_id().as_str(),
                CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
                PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
                pale_core::records::PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA,
                pale_core::records::PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA,
                i64::try_from(limit).map_err(|_| bound())?,
            ],
            decode_stored_coordinate,
        )
        .map_err(|source| sqlite("read principal writer checkpoint coordinates", source))?;
    collect_bounded_coordinates(rows, maximum_records)
}

fn record_coordinates(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    query: &str,
    maximum_records: usize,
    operation: &'static str,
) -> Result<Vec<(ControlTransactionSequenceV1, ControlRecordCoordinate)>, ControlStoreError> {
    let limit = maximum_records.checked_add(1).ok_or_else(bound)?;
    let mut statement = connection
        .prepare(query)
        .map_err(|source| sqlite(operation, source))?;
    let rows = statement
        .query_map(
            params![
                binding.principal_id().as_str(),
                PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
                pale_core::records::PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA,
                pale_core::records::PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA,
                i64::try_from(limit).map_err(|_| bound())?,
            ],
            decode_stored_coordinate,
        )
        .map_err(|source| sqlite(operation, source))?;
    collect_bounded_coordinates(rows, maximum_records)
}

fn decode_stored_coordinate(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<
    Result<(ControlTransactionSequenceV1, ControlRecordCoordinate), ControlStoreError>,
> {
    let inserted_in = row.get_ref(0)?;
    let reference = row.get_ref(1)?;
    let digest = row.get_ref(2)?;
    Ok((|| {
        Ok((
            ControlTransactionSequenceV1::new(positive(inserted_in)?).map_err(|_| recovery())?,
            ControlRecordCoordinate::new(record_ref(reference)?, record_digest(digest)?),
        ))
    })())
}

fn collect_bounded_coordinates(
    rows: rusqlite::MappedRows<
        '_,
        impl FnMut(
            &rusqlite::Row<'_>,
        ) -> rusqlite::Result<
            Result<(ControlTransactionSequenceV1, ControlRecordCoordinate), ControlStoreError>,
        >,
    >,
    maximum_records: usize,
) -> Result<Vec<(ControlTransactionSequenceV1, ControlRecordCoordinate)>, ControlStoreError> {
    let coordinates = rows
        .map(|row| {
            row.map_err(|source| sqlite("decode principal writer record coordinate", source))?
        })
        .collect::<Result<Vec<_>, _>>()?;
    if coordinates.len() > maximum_records {
        return Err(bound());
    }
    Ok(coordinates)
}

fn read_stored_records(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    coordinates: Vec<(ControlTransactionSequenceV1, ControlRecordCoordinate)>,
    remaining: &mut usize,
) -> Result<Vec<PrincipalResidentWriterStoredRecord>, ControlStoreError> {
    coordinates
        .into_iter()
        .map(|(inserted_in, coordinate)| {
            let record = crate::principal_trust::read::read_record_bounded(
                connection,
                binding,
                &coordinate,
                *remaining,
            )?
            .ok_or_else(recovery)?;
            *remaining = remaining
                .checked_sub(record.canonical_record_jcs().as_slice().len())
                .ok_or_else(bound)?;
            Ok(PrincipalResidentWriterStoredRecord::new(
                inserted_in,
                record,
            ))
        })
        .collect()
}

struct RawWriterHead {
    writer_fence_id: PrincipalResidentWriterFenceIdV1,
    writer_generation: PrincipalResidentWriterGenerationV1,
    writer_state: PrincipalResidentWriterFenceStateV1,
    writer_ref: ResidentRecordRefV1,
    writer_digest: ControlRecordDigest,
    checkpoint_domain: ControlCheckpointDomainIdV1,
    checkpoint_epoch: ControlDomainEpochV1,
    checkpoint_sequence: ControlDomainSequenceV1,
    checkpoint_ref: ResidentRecordRefV1,
    checkpoint_digest: ControlRecordDigest,
}

fn read_writer_head(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
) -> Result<Option<RawWriterHead>, ControlStoreError> {
    connection
        .query_row(
            "SELECT writer_fence_id, principal_id, writer_generation, writer_state, \
             writer_fence_ref, writer_fence_sha256, checkpoint_domain_id, \
             checkpoint_domain_epoch, checkpoint_domain_sequence, \
             control_checkpoint_ref, control_checkpoint_sha256 \
             FROM pale_principal_resident_writer_head WHERE singleton=1",
            [],
            |row| {
                let writer_fence_id = row.get_ref(0)?;
                let principal_id = row.get_ref(1)?;
                let writer_generation = row.get_ref(2)?;
                let writer_state = row.get_ref(3)?;
                let writer_ref = row.get_ref(4)?;
                let writer_digest = row.get_ref(5)?;
                let checkpoint_domain = row.get_ref(6)?;
                let checkpoint_epoch = row.get_ref(7)?;
                let checkpoint_sequence = row.get_ref(8)?;
                let checkpoint_ref = row.get_ref(9)?;
                let checkpoint_digest = row.get_ref(10)?;
                Ok((|| {
                    if text(principal_id, MAX_RESIDENT_ID_BYTES)? != binding.principal_id().as_str()
                    {
                        return Err(recovery());
                    }
                    let writer_generation = positive(writer_generation)?;
                    let checkpoint_epoch = positive(checkpoint_epoch)?;
                    let checkpoint_sequence = positive(checkpoint_sequence)?;
                    Ok(RawWriterHead {
                        writer_fence_id: PrincipalResidentWriterFenceIdV1::parse(
                            text(writer_fence_id, MAX_RESIDENT_ID_BYTES)?.to_owned(),
                        )
                        .map_err(|_| recovery())?,
                        writer_generation: PrincipalResidentWriterGenerationV1::new(
                            writer_generation,
                        )
                        .map_err(|_| recovery())?,
                        writer_state: match text(writer_state, 7)? {
                            "active" => PrincipalResidentWriterFenceStateV1::Active,
                            "retired" => PrincipalResidentWriterFenceStateV1::Retired,
                            _ => return Err(recovery()),
                        },
                        writer_ref: record_ref(writer_ref)?,
                        writer_digest: record_digest(writer_digest)?,
                        checkpoint_domain: ControlCheckpointDomainIdV1::parse(
                            text(checkpoint_domain, MAX_RESIDENT_ID_BYTES)?.to_owned(),
                        )
                        .map_err(|_| recovery())?,
                        checkpoint_epoch: ControlDomainEpochV1::new(checkpoint_epoch)
                            .map_err(|_| recovery())?,
                        checkpoint_sequence: ControlDomainSequenceV1::new(checkpoint_sequence)
                            .map_err(|_| recovery())?,
                        checkpoint_ref: record_ref(checkpoint_ref)?,
                        checkpoint_digest: record_digest(checkpoint_digest)?,
                    })
                })())
            },
        )
        .optional()
        .map_err(|source| sqlite("read principal resident-writer head", source))?
        .transpose()
}

fn writer_material_exists(connection: &Connection) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_trust_record \
             WHERE schema_id IN (?1, ?2, ?3) LIMIT 1) \
             OR EXISTS(SELECT 1 FROM pale_principal_control_checkpoint_head \
             WHERE checkpoint_domain_id=?4 LIMIT 1)",
            params![
                PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
                pale_core::records::PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA,
                pale_core::records::PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA,
                PRINCIPAL_WRITER_CHECKPOINT_DOMAIN,
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("classify absent principal writer", source))
}

fn checkpoint_head_matches(
    connection: &Connection,
    raw: &RawWriterHead,
    coordinate: &ControlRecordCoordinate,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_control_checkpoint_head \
             WHERE checkpoint_domain_id=?1 AND domain_epoch=?2 AND domain_sequence=?3 \
               AND control_checkpoint_ref=?4 AND control_checkpoint_sha256=?5)",
            params![
                raw.checkpoint_domain.as_str(),
                sqlite_integer(raw.checkpoint_epoch.get())?,
                sqlite_integer(raw.checkpoint_sequence.get())?,
                coordinate.record_ref().as_str(),
                coordinate.record_sha256().as_str(),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare resident-writer checkpoint head", source))
}

/// Read-only deployment-store evidence over an already opened connection.
///
/// Construction is crate-private until resident assembly admits the store.
pub struct SqliteDeploymentGenesisReader<'connection> {
    pub(crate) connection: &'connection mut Connection,
}

impl DeploymentControlStoreGenesisReadPort for SqliteDeploymentGenesisReader<'_> {
    fn read_genesis(
        &mut self,
        binding: &DeploymentControlStoreBinding,
        limits: DeploymentControlStoreGenesisReadLimits,
    ) -> Result<Option<DeploymentControlStoreGenesisSnapshot>, ControlStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin deployment genesis discovery", source))?;
        crate::deployment::require_genesis_binding(&transaction, binding)?;
        let through = crate::store::transaction::read(&transaction, binding.store())?;
        let Some(close) = read_genesis_close(&transaction, binding)? else {
            if deployment_genesis_material_exists(&transaction)? {
                return Err(recovery());
            }
            transaction
                .rollback()
                .map_err(|source| sqlite("release absent deployment genesis read", source))?;
            return Ok(None);
        };
        if close.committed_transaction.get() > through.get() {
            return Err(recovery());
        }
        let summary = summarize_genesis(&transaction, &close)?;
        if summary.checkpoints > limits.maximum_checkpoints()
            || summary.record_bytes > limits.maximum_record_bytes()
            || summary.mutations > limits.maximum_mutations()
        {
            return Err(bound());
        }
        if summary.checkpoints != close.checkpoint_count {
            return Err(recovery());
        }
        let mut remaining = summary.record_bytes;
        let receipt = read_deployment_record(
            &transaction,
            &close.receipt,
            close.committed_transaction,
            &mut remaining,
        )?;
        if receipt.schema_id().as_str() != DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA
            || pale_core::sha256_hex(receipt.canonical_record_jcs().as_slice())
                != close.receipt_body_sha256.as_str()
        {
            return Err(recovery());
        }
        let checkpoint_evidence = read_genesis_checkpoints(
            &transaction,
            binding,
            &close,
            summary.mutations,
            &mut remaining,
        )?;
        if remaining != 0 {
            return Err(recovery());
        }
        let expected_count = close.checkpoint_count.checked_add(1).ok_or_else(recovery)?;
        let inserted_count: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM pale_deployment_control_record \
                 WHERE inserted_transaction_sequence=?1",
                [sqlite_integer(close.committed_transaction.get())?],
                |row| row.get(0),
            )
            .map_err(|source| sqlite("count deployment genesis records", source))?;
        if usize::try_from(inserted_count).ok() != Some(expected_count) {
            return Err(recovery());
        }
        let records =
            DeploymentControlStoreGenesisRecords::new(checkpoint_evidence.records, receipt)
                .map_err(|_| recovery())?;
        let snapshot = DeploymentControlStoreGenesisSnapshot::new(
            binding.clone(),
            close.genesis_id,
            close.finalization_trust_store,
            close.committed_transaction,
            close.candidate_identity_sha256,
            records,
            checkpoint_evidence.heads,
            checkpoint_evidence.accumulators,
        )
        .map_err(|_| recovery())?;
        if crate::deployment_initialization::genesis_snapshot_identity(&snapshot)
            != snapshot.candidate_identity_sha256().as_str()
        {
            return Err(recovery());
        }
        transaction
            .rollback()
            .map_err(|source| sqlite("release deployment genesis discovery", source))?;
        Ok(Some(snapshot))
    }
}

struct GenesisSummary {
    checkpoints: usize,
    record_bytes: usize,
    mutations: usize,
}

fn summarize_genesis(
    connection: &Connection,
    close: &RawGenesisClose,
) -> Result<GenesisSummary, ControlStoreError> {
    let (checkpoints, mutations, record_count, record_bytes): (i64, i64, i64, i64) = connection
        .query_row(
            "SELECT \
             (SELECT COUNT(*) FROM pale_deployment_control_store_genesis_checkpoint), \
             (SELECT COUNT(*) FROM pale_deployment_control_store_genesis_mutation), \
             (SELECT COUNT(*) FROM pale_deployment_control_record \
                WHERE inserted_transaction_sequence=?1), \
             COALESCE((SELECT SUM(length(canonical_record_jcs)) \
                FROM pale_deployment_control_record WHERE inserted_transaction_sequence=?1),0)",
            [sqlite_integer(close.committed_transaction.get())?],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|source| sqlite("summarize deployment genesis", source))?;
    let checkpoints = usize::try_from(checkpoints).map_err(|_| recovery())?;
    let mutations = usize::try_from(mutations).map_err(|_| recovery())?;
    let record_count = usize::try_from(record_count).map_err(|_| recovery())?;
    let record_bytes = usize::try_from(record_bytes).map_err(|_| recovery())?;
    if record_count != checkpoints.checked_add(1).ok_or_else(recovery)? || record_bytes == 0 {
        return Err(recovery());
    }
    Ok(GenesisSummary {
        checkpoints,
        record_bytes,
        mutations,
    })
}

struct RawGenesisClose {
    genesis_id: DeploymentControlStoreGenesisIdV1,
    finalization_trust_store: AuthorityStoreCoordinate,
    receipt: ControlRecordCoordinate,
    receipt_body_sha256: ArtifactDigest,
    committed_transaction: ControlTransactionSequenceV1,
    candidate_identity_sha256: ArtifactDigest,
    checkpoint_count: usize,
}

struct GenesisCheckpointEvidence {
    heads: Vec<ControlCheckpointHead>,
    records: Vec<ControlRecord>,
    accumulators: Vec<OperationAccumulatorBatch>,
}

fn read_genesis_close(
    connection: &Connection,
    binding: &DeploymentControlStoreBinding,
) -> Result<Option<RawGenesisClose>, ControlStoreError> {
    connection
        .query_row(
            "SELECT genesis_id, principal_id, workspace_id, deployment_binding_ref, \
             deployment_binding_sha256, checkpoint_signer_ref, checkpoint_signer_sha256, \
             runtime_authority_ref, runtime_authority_sha256, finalization_store_id, \
             finalization_incarnation_id, receipt_ref, receipt_sha256, receipt_body_sha256, \
             transaction_sequence, checkpoint_count, candidate_identity_sha256 \
             FROM pale_deployment_control_store_genesis WHERE singleton=1",
            [],
            |row| {
                let genesis_id = row.get_ref(0)?;
                let principal_id = row.get_ref(1)?;
                let workspace_id = row.get_ref(2)?;
                let deployment_binding_ref = row.get_ref(3)?;
                let deployment_binding_digest = row.get_ref(4)?;
                let checkpoint_signer_ref = row.get_ref(5)?;
                let checkpoint_signer_digest = row.get_ref(6)?;
                let runtime_authority_ref = row.get_ref(7)?;
                let runtime_authority_digest = row.get_ref(8)?;
                let finalization_store_id = row.get_ref(9)?;
                let finalization_incarnation_id = row.get_ref(10)?;
                let receipt_ref = row.get_ref(11)?;
                let receipt_digest = row.get_ref(12)?;
                let receipt_body_digest = row.get_ref(13)?;
                let transaction_sequence = row.get_ref(14)?;
                let checkpoint_count = row.get_ref(15)?;
                let candidate_identity = row.get_ref(16)?;
                Ok((|| {
                    if text(principal_id, MAX_RESIDENT_ID_BYTES)? != binding.principal_id().as_str()
                        || text(workspace_id, MAX_RESIDENT_ID_BYTES)?
                            != binding.workspace_id().as_str()
                        || record_ref(deployment_binding_ref)?
                            != *binding.deployment_binding().record_ref()
                        || record_digest(deployment_binding_digest)?
                            != *binding.deployment_binding().record_sha256()
                        || record_ref(checkpoint_signer_ref)?
                            != *binding.checkpoint_signing_authority().record_ref()
                        || record_digest(checkpoint_signer_digest)?
                            != *binding.checkpoint_signing_authority().record_sha256()
                        || record_ref(runtime_authority_ref)?
                            != *binding.runtime_authority().record_ref()
                        || record_digest(runtime_authority_digest)?
                            != *binding.runtime_authority().record_sha256()
                    {
                        return Err(recovery());
                    }
                    let checkpoint_count =
                        usize::try_from(positive(checkpoint_count)?).map_err(|_| recovery())?;
                    if checkpoint_count > pale_core::records::MAX_RESIDENT_SET_MEMBERS {
                        return Err(recovery());
                    }
                    let candidate_identity_sha256 =
                        ArtifactDigest::parse(text(candidate_identity, 64)?.to_owned())
                            .map_err(|_| recovery())?;
                    Ok(RawGenesisClose {
                        genesis_id: DeploymentControlStoreGenesisIdV1::parse(
                            text(genesis_id, MAX_RESIDENT_ID_BYTES)?.to_owned(),
                        )
                        .map_err(|_| recovery())?,
                        finalization_trust_store: AuthorityStoreCoordinate::new(
                            ControlStoreIdV1::parse(
                                text(finalization_store_id, MAX_RESIDENT_ID_BYTES)?.to_owned(),
                            )
                            .map_err(|_| recovery())?,
                            ControlIncarnationIdV1::parse(
                                text(finalization_incarnation_id, MAX_RESIDENT_ID_BYTES)?
                                    .to_owned(),
                            )
                            .map_err(|_| recovery())?,
                        ),
                        receipt: ControlRecordCoordinate::new(
                            record_ref(receipt_ref)?,
                            record_digest(receipt_digest)?,
                        ),
                        receipt_body_sha256: ArtifactDigest::parse(
                            text(receipt_body_digest, 64)?.to_owned(),
                        )
                        .map_err(|_| recovery())?,
                        committed_transaction: ControlTransactionSequenceV1::new(positive(
                            transaction_sequence,
                        )?)
                        .map_err(|_| recovery())?,
                        candidate_identity_sha256,
                        checkpoint_count,
                    })
                })())
            },
        )
        .optional()
        .map_err(|source| sqlite("read deployment genesis singleton", source))?
        .transpose()
}

fn read_genesis_checkpoints(
    connection: &Connection,
    binding: &DeploymentControlStoreBinding,
    close: &RawGenesisClose,
    summarized_mutations: usize,
    remaining: &mut usize,
) -> Result<GenesisCheckpointEvidence, ControlStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT checkpoint_ordinal, checkpoint_domain_id, domain_epoch, domain_sequence, \
             checkpoint_ref, checkpoint_sha256, checkpoint_body_sha256, \
             checkpoint_profile_ref, checkpoint_profile_sha256, state_root_sha256, \
             unresolved_root_sha256, mutation_count \
             FROM pale_deployment_control_store_genesis_checkpoint \
             WHERE genesis_id=?1 ORDER BY checkpoint_ordinal",
        )
        .map_err(|source| sqlite("prepare deployment genesis checkpoints", source))?;
    let mut rows = statement
        .query([close.genesis_id.as_str()])
        .map_err(|source| sqlite("query deployment genesis checkpoints", source))?;
    let mut heads = Vec::with_capacity(close.checkpoint_count);
    let mut records = Vec::with_capacity(close.checkpoint_count);
    let mut accumulators = Vec::with_capacity(close.checkpoint_count);
    let mut expected_mutations = 0_usize;
    while let Some(row) = rows
        .next()
        .map_err(|source| sqlite("step deployment genesis checkpoints", source))?
    {
        let ordinal = usize::try_from(nonnegative(row_value(row, 0)?)?).map_err(|_| recovery())?;
        if ordinal != heads.len() || ordinal >= close.checkpoint_count {
            return Err(recovery());
        }
        let coordinate = ControlRecordCoordinate::new(
            record_ref(row_value(row, 4)?)?,
            record_digest(row_value(row, 5)?)?,
        );
        let body_sha256 = ArtifactDigest::parse(text(row_value(row, 6)?, 64)?.to_owned())
            .map_err(|_| recovery())?;
        let profile_ref =
            ResidentRecordRefV1::parse(text(row_value(row, 7)?, MAX_RESIDENT_ID_BYTES)?.to_owned())
                .map_err(|_| recovery())?;
        let profile_sha256 =
            CheckpointProfileDigest::parse(text(row_value(row, 8)?, 64)?.to_owned())
                .map_err(|_| recovery())?;
        let state_root = ArtifactDigest::parse(text(row_value(row, 9)?, 64)?.to_owned())
            .map_err(|_| recovery())?;
        let unresolved_root = ArtifactDigest::parse(text(row_value(row, 10)?, 64)?.to_owned())
            .map_err(|_| recovery())?;
        let mutation_count =
            usize::try_from(nonnegative(row_value(row, 11)?)?).map_err(|_| recovery())?;
        expected_mutations = expected_mutations
            .checked_add(mutation_count)
            .ok_or_else(recovery)?;
        let record = read_deployment_record(
            connection,
            &coordinate,
            close.committed_transaction,
            remaining,
        )?;
        if record.schema_id().as_str() != CONTROL_STATE_CHECKPOINT_V1_SCHEMA
            || pale_core::sha256_hex(record.canonical_record_jcs().as_slice())
                != body_sha256.as_str()
        {
            return Err(recovery());
        }
        let head = ControlCheckpointHead::new(
            ControlCheckpointDomainIdV1::parse(
                text(row_value(row, 1)?, MAX_RESIDENT_ID_BYTES)?.to_owned(),
            )
            .map_err(|_| recovery())?,
            ControlDomainEpochV1::new(positive(row_value(row, 2)?)?).map_err(|_| recovery())?,
            ControlDomainSequenceV1::new(positive(row_value(row, 3)?)?).map_err(|_| recovery())?,
            close.committed_transaction,
            coordinate,
        );
        let (state_mutations, unresolved_mutations) =
            read_genesis_mutations(connection, close, ordinal, mutation_count)?;
        let predecessor_transaction = close
            .committed_transaction
            .get()
            .checked_sub(1)
            .and_then(|value| ControlTransactionSequenceV1::new(value).ok())
            .ok_or_else(recovery)?;
        let preparation =
            ControlTransactionPreparation::new(binding.store().clone(), predecessor_transaction)
                .map_err(|_| recovery())?;
        let accumulator_binding = AccumulatorCheckpointBinding::new(
            binding.store().clone(),
            profile_ref,
            profile_sha256,
            AccumulatorCheckpointPhase::Initial,
            head.clone(),
            preparation,
        )
        .map_err(|_| recovery())?;
        let zero = ArtifactDigest::parse("0".repeat(64)).map_err(|_| recovery())?;
        let maximum_positions = mutation_count.max(1);
        let state = StateAccumulatorBatch::new(
            zero.clone(),
            state_root,
            state_mutations,
            maximum_positions,
        )
        .map_err(|_| recovery())?;
        let unresolved = UnresolvedCustodyAccumulatorBatch::new(
            zero,
            unresolved_root,
            unresolved_mutations,
            maximum_positions,
        )
        .map_err(|_| recovery())?;
        let accumulator = OperationAccumulatorBatch::new(accumulator_binding, state, unresolved)
            .map_err(|_| recovery())?;
        crate::accumulator::validate(&accumulator)?;
        heads.push(head);
        records.push(record);
        accumulators.push(accumulator);
    }
    if heads.len() != close.checkpoint_count {
        return Err(recovery());
    }
    if expected_mutations != summarized_mutations {
        return Err(recovery());
    }
    Ok(GenesisCheckpointEvidence {
        heads,
        records,
        accumulators,
    })
}

fn read_genesis_mutations(
    connection: &Connection,
    close: &RawGenesisClose,
    ordinal: usize,
    expected_count: usize,
) -> Result<(Vec<AccumulatorMutation>, Vec<AccumulatorMutation>), ControlStoreError> {
    let retained_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pale_deployment_control_store_genesis_mutation \
             WHERE genesis_id=?1 AND checkpoint_ordinal=?2",
            params![close.genesis_id.as_str(), sqlite_integer(ordinal as u64)?],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("count deployment genesis mutations", source))?;
    if usize::try_from(retained_count).ok() != Some(expected_count) {
        return Err(recovery());
    }
    let mut statement = connection
        .prepare(
            "SELECT tree_kind,position_kind,position_key,successor_node_bytes \
             FROM pale_deployment_control_store_genesis_mutation \
             WHERE genesis_id=?1 AND checkpoint_ordinal=?2 \
             ORDER BY CASE tree_kind WHEN 'state' THEN 0 WHEN 'unresolved' THEN 1 ELSE 2 END, \
                      CASE position_kind WHEN 'branch' THEN 0 WHEN 'leaf' THEN 1 ELSE 2 END, \
                      position_key",
        )
        .map_err(|source| sqlite("prepare deployment genesis mutations", source))?;
    let mut rows = statement
        .query(params![
            close.genesis_id.as_str(),
            sqlite_integer(ordinal as u64)?
        ])
        .map_err(|source| sqlite("read deployment genesis mutations", source))?;
    let mut state = Vec::new();
    let mut unresolved = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|source| sqlite("step deployment genesis mutations", source))?
    {
        let tree = text(row_value(row, 0)?, 10)?;
        let position_kind = text(row_value(row, 1)?, 6)?;
        let key = blob_33(row_value(row, 2)?)?;
        let position = match position_kind {
            "branch" => AccumulatorPosition::Branch {
                height: key[0],
                path: key[1..].try_into().map_err(|_| recovery())?,
            },
            "leaf" if key[0] == 0 => {
                AccumulatorPosition::Leaf(key[1..].try_into().map_err(|_| recovery())?)
            }
            _ => return Err(recovery()),
        };
        let successor = optional_node(row_value(row, 3)?)?;
        let mutation =
            AccumulatorMutation::new(position, None, successor, MAX_ACCUMULATOR_NODE_BYTES)
                .map_err(|_| recovery())?;
        match tree {
            "state" => state.push(mutation),
            "unresolved" => unresolved.push(mutation),
            _ => return Err(recovery()),
        }
    }
    if state.len().checked_add(unresolved.len()) != Some(expected_count) {
        return Err(recovery());
    }
    Ok((state, unresolved))
}

fn read_deployment_record(
    connection: &Connection,
    coordinate: &ControlRecordCoordinate,
    expected_transaction: ControlTransactionSequenceV1,
    remaining: &mut usize,
) -> Result<ControlRecord, ControlStoreError> {
    let length =
        crate::deployment::record_body_length(connection, coordinate)?.ok_or_else(recovery)?;
    if length > *remaining {
        return Err(bound());
    }
    let record = crate::deployment::read_record_bounded(connection, coordinate, length)?
        .ok_or_else(recovery)?;
    if crate::store::transaction::record_stamp(
        connection,
        crate::ControlStoreKind::DeploymentControl,
        coordinate,
    )? != Some(expected_transaction)
    {
        return Err(recovery());
    }
    *remaining -= length;
    Ok(record)
}

fn deployment_genesis_material_exists(connection: &Connection) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_control_record LIMIT 1) \
             OR EXISTS(SELECT 1 FROM pale_deployment_operation_checkpoint_head LIMIT 1) \
             OR EXISTS(SELECT 1 FROM pale_deployment_operation_accumulator_frontier LIMIT 1) \
             OR EXISTS(SELECT 1 FROM pale_deployment_operation_accumulator_position LIMIT 1) \
             OR EXISTS(SELECT 1 FROM pale_deployment_control_store_genesis_checkpoint LIMIT 1) \
             OR EXISTS(SELECT 1 FROM pale_deployment_control_store_genesis_mutation LIMIT 1)",
            [],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("classify absent deployment genesis", source))
}

fn text(value: ValueRef<'_>, maximum_bytes: usize) -> Result<&str, ControlStoreError> {
    let ValueRef::Text(bytes) = value else {
        return Err(recovery());
    };
    if bytes.is_empty() || bytes.len() > maximum_bytes {
        return Err(recovery());
    }
    std::str::from_utf8(bytes).map_err(|_| recovery())
}

fn row_value<'row>(
    row: &'row rusqlite::Row<'_>,
    index: usize,
) -> Result<ValueRef<'row>, ControlStoreError> {
    row.get_ref(index)
        .map_err(|source| sqlite("read deployment genesis field", source))
}

fn record_ref(value: ValueRef<'_>) -> Result<ResidentRecordRefV1, ControlStoreError> {
    ResidentRecordRefV1::parse(text(value, MAX_RESIDENT_ID_BYTES)?.to_owned())
        .map_err(|_| recovery())
}

fn record_digest(value: ValueRef<'_>) -> Result<ControlRecordDigest, ControlStoreError> {
    ControlRecordDigest::parse(text(value, 64)?.to_owned()).map_err(|_| recovery())
}

fn blob_33(value: ValueRef<'_>) -> Result<[u8; 33], ControlStoreError> {
    let ValueRef::Blob(bytes) = value else {
        return Err(recovery());
    };
    bytes.try_into().map_err(|_| recovery())
}

fn optional_node(value: ValueRef<'_>) -> Result<Option<Vec<u8>>, ControlStoreError> {
    match value {
        ValueRef::Null => Ok(None),
        ValueRef::Blob(bytes) if matches!(bytes.len(), 32 | 66 | 99 | 132) => {
            Ok(Some(bytes.to_vec()))
        }
        _ => Err(recovery()),
    }
}

fn positive(value: ValueRef<'_>) -> Result<u64, ControlStoreError> {
    let ValueRef::Integer(value) = value else {
        return Err(recovery());
    };
    let value = u64::try_from(value).map_err(|_| recovery())?;
    (value > 0).then_some(value).ok_or_else(recovery)
}

fn nonnegative(value: ValueRef<'_>) -> Result<u64, ControlStoreError> {
    let ValueRef::Integer(value) = value else {
        return Err(recovery());
    };
    u64::try_from(value).map_err(|_| recovery())
}

fn sqlite_integer(value: u64) -> Result<i64, ControlStoreError> {
    i64::try_from(value).map_err(|_| recovery())
}

fn bound() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::BoundExceeded)
}

fn recovery() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}

#[cfg(test)]
#[path = "../tests/internal/cold_start.rs"]
mod tests;
