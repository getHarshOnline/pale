//! Bounded physical reconstruction of deployment-finalization closures.

use pale_core::records::{
    ArtifactDigest, CheckpointProfileDigest, ControlCheckpointDomainIdV1, ControlDomainEpochV1,
    ControlDomainSequenceV1, ControlTransactionSequenceV1, LocalAdminCommandIdV1,
    LocalAdminIdempotencyKeyV1, LocalAdminReceiptIdV1, PrincipalTrustCheckpointEpochV1,
    ResidentRecordRefV1, CONTROL_STATE_CHECKPOINT_V1_SCHEMA, DEPLOYMENT_BINDING_V1_SCHEMA,
    DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA, LOCAL_ADMIN_AUTHORITY_V1_SCHEMA, MAX_RESIDENT_ID_BYTES,
    PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
};
use pale_ports::{
    AccumulatorCheckpointBinding, AccumulatorCheckpointPhase, AccumulatorMutation,
    AccumulatorPosition, AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord,
    ControlRecordCoordinate, ControlRecordDigest, ControlStoreError, ControlStoreErrorKind,
    ControlTransactionPreparation, DeploymentFinalizationRecords,
    DeploymentFinalizationReplayIdentity, DeploymentFinalizationSetReadLimits,
    DeploymentFinalizationSetReadPort, DeploymentFinalizationSetSnapshot,
    DeploymentFinalizationSnapshot, OperationAccumulatorBatch, PrincipalTrustCheckpointHead,
    PrincipalTrustStoreBinding, StateAccumulatorBatch, UnresolvedCustodyAccumulatorBatch,
};
use rusqlite::{params, types::ValueRef, Connection, OptionalExtension, TransactionBehavior};

use crate::error::{sqlite, sqlite_integer};

const MAX_ACCUMULATOR_NODE_BYTES: usize = 132;

/// Read-only finalization evidence over an already opened principal connection.
///
/// Construction is crate-private until resident assembly admits the store.
pub struct SqliteDeploymentFinalizationSetReader<'connection> {
    pub(crate) connection: &'connection mut Connection,
}

impl DeploymentFinalizationSetReadPort for SqliteDeploymentFinalizationSetReader<'_> {
    fn read_finalization_set(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        limits: DeploymentFinalizationSetReadLimits,
    ) -> Result<Option<DeploymentFinalizationSetSnapshot>, ControlStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin deployment finalization set read", source))?;
        if !crate::principal_trust::binding_matches(&transaction, binding)? {
            return Err(recovery());
        }
        let observed_through = crate::store::transaction::read(
            &transaction,
            &crate::principal_trust::store_coordinate(binding),
        )?;
        let close_count = finalization_close_count(&transaction, binding)?;
        if close_count > limits.maximum_finalizations() {
            return Err(bound());
        }
        if close_count == 0 {
            if unclosed_finalization_material_exists(&transaction)? {
                return Err(recovery());
            }
            transaction.rollback().map_err(|source| {
                sqlite("release absent deployment finalization set read", source)
            })?;
            return Ok(None);
        }
        let closes = read_closes(&transaction, binding, close_count)?;
        if closes.len() != close_count
            || closes
                .windows(2)
                .any(|pair| pair[0].committed_transaction >= pair[1].committed_transaction)
            || closes
                .last()
                .is_some_and(|close| close.committed_transaction > observed_through)
        {
            return Err(recovery());
        }
        let total_mutations = closes.iter().try_fold(0_usize, |total, close| {
            total.checked_add(close.mutation_count).ok_or_else(recovery)
        })?;
        if finalization_mutation_count(&transaction)? != total_mutations
            || unclosed_finalization_material_exists(&transaction)?
        {
            return Err(recovery());
        }
        if total_mutations > limits.maximum_mutations() {
            return Err(bound());
        }
        require_record_set_copy_bound(&transaction, &closes, limits.maximum_record_bytes())?;

        let mut finalizations = Vec::with_capacity(closes.len());
        for close in closes {
            let records = read_records(&transaction, binding, &close)?;
            let accumulator = read_accumulator(&transaction, binding, &close)?;
            crate::accumulator::validate(&accumulator)?;
            require_transaction_record_set(&transaction, &close)?;
            let snapshot = DeploymentFinalizationSnapshot::new(
                binding.clone(),
                close.replay,
                records.admin_authority,
                records.finalization,
                close.predecessor_trust,
                records.predecessor_trust,
                close.successor_trust,
                close.predecessor_control,
                records.predecessor_control,
                close.successor_control,
                accumulator,
                close.committed_transaction,
                close.candidate_identity_sha256,
            )
            .map_err(|_| recovery())?;
            if crate::deployment_initialization::finalization_snapshot_identity(&snapshot)
                != snapshot.candidate_identity_sha256().as_str()
            {
                return Err(recovery());
            }
            finalizations.push(snapshot);
        }
        let snapshot = DeploymentFinalizationSetSnapshot::new(
            binding.clone(),
            observed_through,
            finalizations.into_boxed_slice(),
        )
        .map_err(|_| recovery())?;
        transaction
            .rollback()
            .map_err(|source| sqlite("release deployment finalization set read", source))?;
        Ok(Some(snapshot))
    }
}

struct RawFinalizationClose {
    replay: DeploymentFinalizationReplayIdentity,
    record_coordinates: [ControlRecordCoordinate; 7],
    predecessor_trust: PrincipalTrustCheckpointHead,
    successor_trust: PrincipalTrustCheckpointHead,
    predecessor_control: ControlCheckpointHead,
    successor_control: ControlCheckpointHead,
    committed_transaction: ControlTransactionSequenceV1,
    profile_ref: ResidentRecordRefV1,
    profile_sha256: CheckpointProfileDigest,
    predecessor_state_root: ArtifactDigest,
    successor_state_root: ArtifactDigest,
    predecessor_unresolved_root: ArtifactDigest,
    successor_unresolved_root: ArtifactDigest,
    mutation_count: usize,
    record_body_sha256: [ArtifactDigest; 7],
    candidate_identity_sha256: ArtifactDigest,
}

impl RawFinalizationClose {
    fn record_coordinates(&self) -> [ControlRecordCoordinate; 10] {
        [
            self.replay.admin_authority().clone(),
            self.predecessor_trust.coordinate().clone(),
            self.predecessor_control.coordinate().clone(),
            self.record_coordinates[0].clone(),
            self.record_coordinates[1].clone(),
            self.record_coordinates[2].clone(),
            self.record_coordinates[3].clone(),
            self.record_coordinates[4].clone(),
            self.record_coordinates[5].clone(),
            self.record_coordinates[6].clone(),
        ]
    }
}

fn finalization_close_count(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
) -> Result<usize, ControlStoreError> {
    let (total, bound): (i64, i64) = connection
        .query_row(
            "SELECT COUNT(*),COALESCE(SUM(CASE WHEN principal_id=?1 THEN 1 ELSE 0 END),0) \
             FROM pale_principal_deployment_finalization",
            [binding.principal_id().as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|source| sqlite("count deployment finalization closes", source))?;
    if total != bound {
        return Err(recovery());
    }
    usize::try_from(total).map_err(|_| recovery())
}

fn read_closes(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    expected_count: usize,
) -> Result<Vec<RawFinalizationClose>, ControlStoreError> {
    let mut statement = connection
        .prepare(
            "SELECT admin_authority_ref,admin_authority_sha256,idempotency_key,admin_command_id,\
             command_ref,command_sha256,receipt_id,receipt_ref,receipt_sha256,binding_ref,binding_sha256,\
             checkpoint_signer_ref,checkpoint_signer_sha256,runtime_authority_ref,runtime_authority_sha256,\
             trust_checkpoint_ref,trust_checkpoint_sha256,control_checkpoint_ref,control_checkpoint_sha256,\
             transaction_sequence,principal_id,predecessor_trust_epoch,predecessor_trust_ref,\
             predecessor_trust_sha256,successor_trust_epoch,successor_trust_ref,successor_trust_sha256,\
             control_domain_id,predecessor_control_epoch,predecessor_control_sequence,\
             predecessor_control_ref,predecessor_control_sha256,successor_control_epoch,\
             successor_control_sequence,successor_control_ref,successor_control_sha256,\
             checkpoint_profile_ref,checkpoint_profile_sha256,predecessor_state_root_sha256,\
             successor_state_root_sha256,predecessor_unresolved_root_sha256,\
             successor_unresolved_root_sha256,mutation_count,command_body_sha256,binding_body_sha256,\
             checkpoint_signer_body_sha256,runtime_authority_body_sha256,trust_checkpoint_body_sha256,\
             control_checkpoint_body_sha256,receipt_body_sha256,candidate_identity_sha256, \
             (SELECT predecessor_record.inserted_transaction_sequence \
                FROM pale_principal_trust_record AS predecessor_record \
               WHERE predecessor_record.record_ref=finalization.predecessor_control_ref \
                 AND predecessor_record.record_sha256=finalization.predecessor_control_sha256) \
             FROM pale_principal_deployment_finalization AS finalization \
             ORDER BY transaction_sequence,admin_authority_ref,admin_authority_sha256,idempotency_key",
        )
        .map_err(|source| sqlite("prepare deployment finalization closes", source))?;
    let mut rows = statement
        .query([])
        .map_err(|source| sqlite("read deployment finalization closes", source))?;
    let mut closes = Vec::with_capacity(expected_count);
    while let Some(row) = rows
        .next()
        .map_err(|source| sqlite("read deployment finalization close", source))?
    {
        closes.push(parse_close(row, binding)?);
    }
    Ok(closes)
}

fn parse_close(
    row: &rusqlite::Row<'_>,
    binding: &PrincipalTrustStoreBinding,
) -> Result<RawFinalizationClose, ControlStoreError> {
    let row = PhysicalRow(row);
    let principal = text(row.get_ref(20)?, MAX_RESIDENT_ID_BYTES)?;
    if principal != binding.principal_id().as_str() {
        return Err(recovery());
    }
    let committed_transaction = transaction_sequence(row.get_ref(19)?)?;
    let predecessor_trust = PrincipalTrustCheckpointHead::new(
        binding.principal_id().clone(),
        PrincipalTrustCheckpointEpochV1::new(positive(row.get_ref(21)?)?)
            .map_err(|_| recovery())?,
        coordinate(row.get_ref(22)?, row.get_ref(23)?)?,
    );
    let successor_trust = PrincipalTrustCheckpointHead::new(
        binding.principal_id().clone(),
        PrincipalTrustCheckpointEpochV1::new(positive(row.get_ref(24)?)?)
            .map_err(|_| recovery())?,
        coordinate(row.get_ref(25)?, row.get_ref(26)?)?,
    );
    let domain = ControlCheckpointDomainIdV1::parse(
        text(row.get_ref(27)?, MAX_RESIDENT_ID_BYTES)?.to_owned(),
    )
    .map_err(|_| recovery())?;
    let predecessor_control = ControlCheckpointHead::new(
        domain.clone(),
        ControlDomainEpochV1::new(positive(row.get_ref(28)?)?).map_err(|_| recovery())?,
        ControlDomainSequenceV1::new(positive(row.get_ref(29)?)?).map_err(|_| recovery())?,
        transaction_sequence(row.get_ref(51)?)?,
        coordinate(row.get_ref(30)?, row.get_ref(31)?)?,
    );
    let successor_control = ControlCheckpointHead::new(
        domain,
        ControlDomainEpochV1::new(positive(row.get_ref(32)?)?).map_err(|_| recovery())?,
        ControlDomainSequenceV1::new(positive(row.get_ref(33)?)?).map_err(|_| recovery())?,
        committed_transaction,
        coordinate(row.get_ref(34)?, row.get_ref(35)?)?,
    );
    let command = coordinate(row.get_ref(4)?, row.get_ref(5)?)?;
    let replay = DeploymentFinalizationReplayIdentity::new(
        LocalAdminCommandIdV1::parse(text(row.get_ref(3)?, MAX_RESIDENT_ID_BYTES)?.to_owned())
            .map_err(|_| recovery())?,
        LocalAdminIdempotencyKeyV1::parse(text(row.get_ref(2)?, MAX_RESIDENT_ID_BYTES)?.to_owned())
            .map_err(|_| recovery())?,
        coordinate(row.get_ref(0)?, row.get_ref(1)?)?,
        command.clone(),
        LocalAdminReceiptIdV1::parse(text(row.get_ref(6)?, MAX_RESIDENT_ID_BYTES)?.to_owned())
            .map_err(|_| recovery())?,
    );
    Ok(RawFinalizationClose {
        replay,
        record_coordinates: [
            command,
            coordinate(row.get_ref(9)?, row.get_ref(10)?)?,
            coordinate(row.get_ref(11)?, row.get_ref(12)?)?,
            coordinate(row.get_ref(13)?, row.get_ref(14)?)?,
            coordinate(row.get_ref(15)?, row.get_ref(16)?)?,
            coordinate(row.get_ref(17)?, row.get_ref(18)?)?,
            coordinate(row.get_ref(7)?, row.get_ref(8)?)?,
        ],
        predecessor_trust,
        successor_trust,
        predecessor_control,
        successor_control,
        committed_transaction,
        profile_ref: record_ref(row.get_ref(36)?)?,
        profile_sha256: CheckpointProfileDigest::parse(text(row.get_ref(37)?, 64)?.to_owned())
            .map_err(|_| recovery())?,
        predecessor_state_root: artifact_digest(row.get_ref(38)?)?,
        successor_state_root: artifact_digest(row.get_ref(39)?)?,
        predecessor_unresolved_root: artifact_digest(row.get_ref(40)?)?,
        successor_unresolved_root: artifact_digest(row.get_ref(41)?)?,
        mutation_count: usize::try_from(nonnegative(row.get_ref(42)?)?).map_err(|_| recovery())?,
        record_body_sha256: [
            artifact_digest(row.get_ref(43)?)?,
            artifact_digest(row.get_ref(44)?)?,
            artifact_digest(row.get_ref(45)?)?,
            artifact_digest(row.get_ref(46)?)?,
            artifact_digest(row.get_ref(47)?)?,
            artifact_digest(row.get_ref(48)?)?,
            artifact_digest(row.get_ref(49)?)?,
        ],
        candidate_identity_sha256: artifact_digest(row.get_ref(50)?)?,
    })
}

struct PhysicalRow<'row>(&'row rusqlite::Row<'row>);

impl PhysicalRow<'_> {
    fn get_ref(&self, index: usize) -> Result<ValueRef<'_>, ControlStoreError> {
        self.0
            .get_ref(index)
            .map_err(|source| sqlite("read deployment finalization field", source))
    }
}

struct ReadFinalizationRecords {
    admin_authority: ControlRecord,
    predecessor_trust: ControlRecord,
    predecessor_control: ControlRecord,
    finalization: DeploymentFinalizationRecords,
}

fn read_records(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    close: &RawFinalizationClose,
) -> Result<ReadFinalizationRecords, ControlStoreError> {
    let coordinates = close.record_coordinates();
    let mut records = Vec::with_capacity(coordinates.len());
    for coordinate in &coordinates {
        records.push(
            crate::principal_trust::read::read_record_bounded(
                connection,
                binding,
                coordinate,
                usize::MAX,
            )?
            .ok_or_else(recovery)?,
        );
    }
    let mut records = records.into_iter();
    let admin_authority = records.next().ok_or_else(recovery)?;
    let predecessor_trust = records.next().ok_or_else(recovery)?;
    let predecessor_control = records.next().ok_or_else(recovery)?;
    let finalization_records = records.collect::<Vec<_>>();
    let [command, deployment_binding, checkpoint_signer, runtime_authority, trust_checkpoint, control_checkpoint, receipt] =
        finalization_records.try_into().map_err(|_| recovery())?;
    if admin_authority.schema_id().as_str() != LOCAL_ADMIN_AUTHORITY_V1_SCHEMA
        || predecessor_trust.schema_id().as_str() != PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA
        || predecessor_control.schema_id().as_str() != CONTROL_STATE_CHECKPOINT_V1_SCHEMA
    {
        return Err(recovery());
    }
    let body_records = [
        &command,
        &deployment_binding,
        &checkpoint_signer,
        &runtime_authority,
        &trust_checkpoint,
        &control_checkpoint,
        &receipt,
    ];
    for (record, expected) in body_records.into_iter().zip(&close.record_body_sha256) {
        if pale_core::sha256_hex(record.canonical_record_jcs().as_slice()) != expected.as_str() {
            return Err(recovery());
        }
    }
    let finalization = DeploymentFinalizationRecords::new(
        command,
        deployment_binding,
        checkpoint_signer,
        runtime_authority,
        trust_checkpoint,
        control_checkpoint,
        receipt,
    )
    .map_err(|_| recovery())?;
    Ok(ReadFinalizationRecords {
        admin_authority,
        predecessor_trust,
        predecessor_control,
        finalization,
    })
}

fn require_record_set_copy_bound(
    connection: &Connection,
    closes: &[RawFinalizationClose],
    maximum_record_bytes: usize,
) -> Result<(), ControlStoreError> {
    let mut total = 0_usize;
    for close in closes {
        for coordinate in close.record_coordinates() {
            let observed = connection
                .query_row(
                    "SELECT record_sha256,length(canonical_record_jcs) FROM pale_principal_trust_record WHERE record_ref=?1",
                    [coordinate.record_ref().as_str()],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()
                .map_err(|source| sqlite("measure deployment finalization record", source))?
                .ok_or_else(recovery)?;
            if observed.0 != coordinate.record_sha256().as_str() {
                return Err(recovery());
            }
            let length = usize::try_from(observed.1).map_err(|_| recovery())?;
            if length == 0 {
                return Err(recovery());
            }
            total = total.checked_add(length).ok_or_else(bound)?;
        }
    }
    if total > maximum_record_bytes {
        return Err(bound());
    }
    Ok(())
}

fn read_accumulator(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    close: &RawFinalizationClose,
) -> Result<OperationAccumulatorBatch, ControlStoreError> {
    let actual_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pale_principal_deployment_finalization_mutation \
             WHERE admin_authority_ref=?1 AND admin_authority_sha256=?2 AND idempotency_key=?3",
            params![
                close.replay.admin_authority().record_ref().as_str(),
                close.replay.admin_authority().record_sha256().as_str(),
                close.replay.idempotency_key().as_str(),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("count deployment finalization mutations", source))?;
    let actual_count = usize::try_from(actual_count).map_err(|_| recovery())?;
    if actual_count != close.mutation_count {
        return Err(recovery());
    }
    let mut statement = connection
        .prepare(
            "SELECT admin_authority_ref,admin_authority_sha256,idempotency_key,\
                    tree_kind,position_kind,position_key,predecessor_node_bytes,successor_node_bytes \
             FROM pale_principal_deployment_finalization_mutation \
             WHERE admin_authority_ref=?1 AND admin_authority_sha256=?2 AND idempotency_key=?3 \
             ORDER BY CASE tree_kind WHEN 'state' THEN 0 WHEN 'unresolved' THEN 1 ELSE 2 END,\
                      CASE position_kind WHEN 'branch' THEN 0 WHEN 'leaf' THEN 1 ELSE 2 END,position_key",
        )
        .map_err(|source| sqlite("prepare deployment finalization mutations", source))?;
    let mut rows = statement
        .query(params![
            close.replay.admin_authority().record_ref().as_str(),
            close.replay.admin_authority().record_sha256().as_str(),
            close.replay.idempotency_key().as_str(),
        ])
        .map_err(|source| sqlite("read deployment finalization mutations", source))?;
    let mut state = Vec::new();
    let mut unresolved = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|source| sqlite("read deployment finalization mutation row", source))?
    {
        let authority_ref = text(
            row.get_ref(0)
                .map_err(|source| sqlite("read finalization mutation authority", source))?,
            MAX_RESIDENT_ID_BYTES,
        )?;
        let authority_digest = text(
            row.get_ref(1)
                .map_err(|source| sqlite("read finalization mutation authority digest", source))?,
            64,
        )?;
        let idempotency_key = text(
            row.get_ref(2)
                .map_err(|source| sqlite("read finalization mutation replay key", source))?,
            MAX_RESIDENT_ID_BYTES,
        )?;
        if authority_ref != close.replay.admin_authority().record_ref().as_str()
            || authority_digest != close.replay.admin_authority().record_sha256().as_str()
            || idempotency_key != close.replay.idempotency_key().as_str()
        {
            return Err(recovery());
        }
        let tree = text(
            row.get_ref(3)
                .map_err(|source| sqlite("read finalization tree", source))?,
            10,
        )?;
        let position_kind = text(
            row.get_ref(4)
                .map_err(|source| sqlite("read finalization position kind", source))?,
            6,
        )?;
        let key = blob_33(
            row.get_ref(5)
                .map_err(|source| sqlite("read finalization position", source))?,
        )?;
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
        let predecessor = optional_node(
            row.get_ref(6)
                .map_err(|source| sqlite("read finalization predecessor node", source))?,
        )?;
        let successor = optional_node(
            row.get_ref(7)
                .map_err(|source| sqlite("read finalization successor node", source))?,
        )?;
        let mutation =
            AccumulatorMutation::new(position, predecessor, successor, MAX_ACCUMULATOR_NODE_BYTES)
                .map_err(|_| recovery())?;
        match tree {
            "state" => state.push(mutation),
            "unresolved" => unresolved.push(mutation),
            _ => return Err(recovery()),
        }
    }
    drop(rows);
    drop(statement);

    let expected_transaction = predecessor_transaction(close.committed_transaction)?;
    let store = AuthorityStoreCoordinate::new(
        binding.trust_store_id().clone(),
        binding.trust_store_incarnation_id().clone(),
    );
    let preparation = ControlTransactionPreparation::new(store.clone(), expected_transaction)
        .map_err(|_| recovery())?;
    let accumulator_binding = AccumulatorCheckpointBinding::new(
        store,
        close.profile_ref.clone(),
        close.profile_sha256.clone(),
        AccumulatorCheckpointPhase::Successor(close.predecessor_control.clone()),
        close.successor_control.clone(),
        preparation,
    )
    .map_err(|_| recovery())?;
    let state = StateAccumulatorBatch::new(
        close.predecessor_state_root.clone(),
        close.successor_state_root.clone(),
        state,
        close.mutation_count.max(1),
    )
    .map_err(|_| recovery())?;
    let unresolved = UnresolvedCustodyAccumulatorBatch::new(
        close.predecessor_unresolved_root.clone(),
        close.successor_unresolved_root.clone(),
        unresolved,
        close.mutation_count.max(1),
    )
    .map_err(|_| recovery())?;
    OperationAccumulatorBatch::new(accumulator_binding, state, unresolved).map_err(|_| recovery())
}

fn require_transaction_record_set(
    connection: &Connection,
    close: &RawFinalizationClose,
) -> Result<(), ControlStoreError> {
    let inserted_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM (SELECT 1 FROM pale_principal_trust_record WHERE inserted_transaction_sequence=?1 LIMIT 8)",
            [sqlite_integer(close.committed_transaction.get())?],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("count deployment finalization transaction records", source))?;
    if inserted_count != 7 {
        return Err(recovery());
    }
    for coordinate in [
        close.replay.admin_authority(),
        close.predecessor_trust.coordinate(),
        close.predecessor_control.coordinate(),
    ] {
        let stamp = crate::store::transaction::record_stamp(
            connection,
            crate::ControlStoreKind::PrincipalTrust,
            coordinate,
        )?
        .ok_or_else(recovery)?;
        if stamp.get() >= close.committed_transaction.get() {
            return Err(recovery());
        }
    }
    for coordinate in &close.record_coordinates {
        if crate::store::transaction::record_stamp(
            connection,
            crate::ControlStoreKind::PrincipalTrust,
            coordinate,
        )? != Some(close.committed_transaction)
        {
            return Err(recovery());
        }
    }
    Ok(())
}

fn finalization_mutation_count(connection: &Connection) -> Result<usize, ControlStoreError> {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pale_principal_deployment_finalization_mutation",
            [],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("count deployment finalization mutation set", source))?;
    usize::try_from(count).map_err(|_| recovery())
}

fn unclosed_finalization_material_exists(
    connection: &Connection,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT \
             EXISTS(SELECT 1 FROM pale_principal_deployment_finalization_mutation AS mutation \
               LEFT JOIN pale_principal_deployment_finalization AS finalization \
                 ON finalization.admin_authority_ref=mutation.admin_authority_ref \
                AND finalization.admin_authority_sha256=mutation.admin_authority_sha256 \
                AND finalization.idempotency_key=mutation.idempotency_key \
              WHERE finalization.admin_authority_ref IS NULL LIMIT 1) \
             OR EXISTS(SELECT 1 FROM pale_principal_trust_record AS record \
              WHERE record.schema_id=?1 AND NOT EXISTS( \
                SELECT 1 FROM pale_principal_deployment_finalization AS finalization \
                 WHERE finalization.binding_ref=record.record_ref \
                   AND finalization.binding_sha256=record.record_sha256) LIMIT 1) \
             OR EXISTS(SELECT 1 FROM pale_principal_trust_record AS record \
              WHERE record.schema_id=?2 AND NOT EXISTS( \
                SELECT 1 FROM pale_principal_deployment_finalization AS finalization \
                 WHERE finalization.runtime_authority_ref=record.record_ref \
                   AND finalization.runtime_authority_sha256=record.record_sha256) LIMIT 1)",
            params![
                DEPLOYMENT_BINDING_V1_SCHEMA,
                DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA,
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("classify unclosed deployment finalization material", source))
}

fn predecessor_transaction(
    transaction: ControlTransactionSequenceV1,
) -> Result<ControlTransactionSequenceV1, ControlStoreError> {
    let predecessor = transaction.get().checked_sub(1).ok_or_else(recovery)?;
    ControlTransactionSequenceV1::new(predecessor).map_err(|_| recovery())
}

fn coordinate(
    reference: ValueRef<'_>,
    digest: ValueRef<'_>,
) -> Result<ControlRecordCoordinate, ControlStoreError> {
    Ok(ControlRecordCoordinate::new(
        record_ref(reference)?,
        ControlRecordDigest::parse(text(digest, 64)?.to_owned()).map_err(|_| recovery())?,
    ))
}

fn record_ref(value: ValueRef<'_>) -> Result<ResidentRecordRefV1, ControlStoreError> {
    ResidentRecordRefV1::parse(text(value, MAX_RESIDENT_ID_BYTES)?.to_owned())
        .map_err(|_| recovery())
}

fn artifact_digest(value: ValueRef<'_>) -> Result<ArtifactDigest, ControlStoreError> {
    ArtifactDigest::parse(text(value, 64)?.to_owned()).map_err(|_| recovery())
}

fn transaction_sequence(
    value: ValueRef<'_>,
) -> Result<ControlTransactionSequenceV1, ControlStoreError> {
    ControlTransactionSequenceV1::new(positive(value)?).map_err(|_| recovery())
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

fn text(value: ValueRef<'_>, maximum_bytes: usize) -> Result<&str, ControlStoreError> {
    let ValueRef::Text(bytes) = value else {
        return Err(recovery());
    };
    if bytes.is_empty() || bytes.len() > maximum_bytes {
        return Err(recovery());
    }
    std::str::from_utf8(bytes).map_err(|_| recovery())
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

fn bound() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::BoundExceeded)
}

fn recovery() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}
