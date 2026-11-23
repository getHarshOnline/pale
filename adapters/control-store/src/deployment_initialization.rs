//! Atomic `SQLite` custody for the two owner-local deployment-initialization transactions.
//!
//! The ledgers in this module are append-only physical closure. They compare
//! typed coordinates and opaque bytes only; they do not decode records, select
//! a signer, an anchor, a path, or imply a cross-database transaction.

use pale_ports::{
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord, ControlStoreError,
    ControlStoreErrorKind, DeploymentControlStoreGenesisCommitDisposition,
    DeploymentControlStoreGenesisCustodyPort, DeploymentControlStoreGenesisDatabaseObservation,
    DeploymentControlStoreGenesisSnapshot, DeploymentFinalizationCommitDisposition,
    DeploymentFinalizationCustodyPort, DeploymentFinalizationDatabaseObservation,
    DeploymentFinalizationRecords, DeploymentFinalizationReplayIdentity,
    DeploymentFinalizationSnapshot, OperationAccumulatorBatch,
    PreparedDeploymentControlStoreGenesisAtom, PreparedDeploymentFinalizationAtom,
    PrincipalTrustCheckpointHead, PrincipalTrustStoreBinding,
};
use rusqlite::{params, Connection, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::accumulator::{self, Store as AccumulatorStore};
use crate::deployment::{
    insert_record as insert_deployment_record, record_matches as deployment_record_matches,
};
use crate::error::{sqlite, sqlite_integer};
use crate::principal_trust::{
    advance_control_checkpoint_head, advance_trust_checkpoint_head, binding_matches,
    control_checkpoint_head_matches, insert_record as insert_principal_record,
    record_matches as principal_record_matches, store_coordinate, trust_checkpoint_head_matches,
};
use crate::{ControlSqliteError, ControlStoreKind};

const MAX_GENESIS_CHECKPOINTS: usize = 1_024;

/// Principal-trust-store transaction-one custody.
pub struct SqliteDeploymentFinalizationCustody<'connection> {
    pub(crate) connection: &'connection mut Connection,
    /// Runtime-selected opaque record ceiling; zero is never an admitted profile.
    pub(crate) maximum_record_bytes: usize,
}

/// Deployment-control-store transaction-two custody.
pub struct SqliteDeploymentControlStoreGenesisCustody<'connection> {
    pub(crate) connection: &'connection mut Connection,
    pub(crate) coordinate: AuthorityStoreCoordinate,
    /// Runtime-selected opaque record ceiling; zero is never an admitted profile.
    pub(crate) maximum_record_bytes: usize,
}

fn conflict() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::CandidateConflict)
}
fn recovery() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}
fn require_bound<'a>(
    records: impl IntoIterator<Item = &'a ControlRecord>,
    maximum: usize,
) -> Result<(), ControlStoreError> {
    if maximum == 0
        || records
            .into_iter()
            .any(|record| record.canonical_record_jcs().as_slice().len() > maximum)
    {
        return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
    }
    Ok(())
}

impl pale_ports::ControlTransactionPort for SqliteDeploymentFinalizationCustody<'_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, ControlStoreError> {
        crate::store::transaction::prepare_snapshot(
            self.connection,
            store,
            ControlStoreKind::PrincipalTrust,
        )
    }
}

impl DeploymentFinalizationCustodyPort for SqliteDeploymentFinalizationCustody<'_> {
    fn compare_and_commit(
        &mut self,
        candidate: &PreparedDeploymentFinalizationAtom,
    ) -> Result<DeploymentFinalizationCommitDisposition, ControlStoreError> {
        require_finalization_bound(candidate, self.maximum_record_bytes)?;
        accumulator::validate(candidate.accumulator())?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|source| sqlite("begin deployment finalization", source))?;
        require_finalization_store(&transaction, candidate)?;
        match observe_finalization(&transaction, candidate)? {
            DeploymentFinalizationDatabaseObservation::Exact => {
                transaction
                    .rollback()
                    .map_err(|source| sqlite("close exact finalization replay", source))?;
                Ok(DeploymentFinalizationCommitDisposition::ExactReplay)
            }
            DeploymentFinalizationDatabaseObservation::ConflictingOrPartial => {
                if finalization_key_exists(&transaction, candidate)? {
                    if finalization_identity_matches(&transaction, candidate)? {
                        Err(recovery())
                    } else {
                        Err(conflict())
                    }
                } else if finalization_competing_successor_is_complete(&transaction, candidate)? {
                    Err(conflict())
                } else {
                    Err(recovery())
                }
            }
            DeploymentFinalizationDatabaseObservation::Predecessor => {
                let store = store_coordinate(candidate.binding());
                let prepared = crate::store::transaction::prepare(&transaction, &store)?;
                if candidate.accumulator().binding().transaction() != &prepared
                    || candidate
                        .successor_control()
                        .committed_transaction_sequence()
                        != prepared.successor()
                {
                    return Err(conflict());
                }
                crate::store::transaction::advance(&transaction, &prepared)?;
                for record in finalization_records(candidate) {
                    insert_principal_record(&transaction, candidate.binding(), record)?;
                }
                advance_trust_checkpoint_head(
                    &transaction,
                    candidate.predecessor_trust(),
                    candidate.successor_trust(),
                )?;
                advance_control_checkpoint_head(
                    &transaction,
                    candidate.predecessor_control(),
                    candidate.successor_control(),
                )?;
                accumulator::apply(
                    &transaction,
                    AccumulatorStore::PrincipalTrust,
                    candidate.accumulator(),
                )?;
                insert_finalization_mutations(&transaction, candidate)?;
                insert_finalization_close(&transaction, candidate, prepared.successor())?;
                if observe_finalization(&transaction, candidate)?
                    != DeploymentFinalizationDatabaseObservation::Exact
                {
                    return Err(recovery());
                }
                transaction.commit().map_err(|source| {
                    ControlStoreError::with_source(
                        ControlStoreErrorKind::OutcomeUnknown,
                        ControlSqliteError::sqlite("commit deployment finalization", source),
                    )
                })?;
                Ok(DeploymentFinalizationCommitDisposition::New)
            }
        }
    }

    fn observe_exact(
        &mut self,
        candidate: &PreparedDeploymentFinalizationAtom,
    ) -> Result<DeploymentFinalizationDatabaseObservation, ControlStoreError> {
        require_finalization_bound(candidate, self.maximum_record_bytes)?;
        accumulator::validate(candidate.accumulator())?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin deployment finalization read-back", source))?;
        require_finalization_store(&transaction, candidate)?;
        let observed = observe_finalization(&transaction, candidate)?;
        transaction
            .rollback()
            .map_err(|source| sqlite("close deployment finalization read-back", source))?;
        Ok(observed)
    }
}

fn require_finalization_bound(
    candidate: &PreparedDeploymentFinalizationAtom,
    maximum: usize,
) -> Result<(), ControlStoreError> {
    require_bound(finalization_records(candidate), maximum)
}

fn finalization_records(candidate: &PreparedDeploymentFinalizationAtom) -> [&ControlRecord; 7] {
    let r = candidate.records();
    [
        r.command(),
        r.binding(),
        r.checkpoint_signing_authority(),
        r.runtime_authority(),
        r.trust_checkpoint(),
        r.control_checkpoint(),
        r.receipt(),
    ]
}

fn require_finalization_store(
    connection: &Connection,
    candidate: &PreparedDeploymentFinalizationAtom,
) -> Result<(), ControlStoreError> {
    if !binding_matches(connection, candidate.binding())? {
        return Err(recovery());
    }
    crate::store::transaction::read(connection, &store_coordinate(candidate.binding()))?;
    if crate::store::transaction::record_stamp(
        connection,
        ControlStoreKind::PrincipalTrust,
        candidate.replay().admin_authority(),
    )?
    .is_none()
    {
        return Err(recovery());
    }
    Ok(())
}

fn finalization_competing_successor_is_complete(
    connection: &Connection,
    candidate: &PreparedDeploymentFinalizationAtom,
) -> Result<bool, ControlStoreError> {
    if trust_checkpoint_head_matches(connection, candidate.predecessor_trust())?
        && control_checkpoint_head_matches(connection, candidate.predecessor_control())?
    {
        // Candidate material beside the still-exact predecessor is partial
        // custody, not a complete competing successor.
        return Ok(false);
    }
    let complete = connection.query_row(
        "SELECT \
         EXISTS(SELECT 1 FROM pale_principal_trust_checkpoint_head h JOIN pale_principal_trust_record r ON r.record_ref=h.trust_checkpoint_ref AND r.record_sha256=h.trust_checkpoint_sha256 WHERE h.singleton=1) \
         AND EXISTS(SELECT 1 FROM pale_principal_control_checkpoint_head h \
           JOIN pale_principal_trust_record r ON r.record_ref=h.control_checkpoint_ref AND r.record_sha256=h.control_checkpoint_sha256 \
           JOIN pale_principal_control_accumulator_frontier f \
             ON f.checkpoint_domain_id=h.checkpoint_domain_id AND f.domain_epoch=h.domain_epoch \
            AND f.control_checkpoint_ref=h.control_checkpoint_ref AND f.control_checkpoint_sha256=h.control_checkpoint_sha256 \
           WHERE h.checkpoint_domain_id=?1)",
        [candidate.successor_control().checkpoint_domain_id().as_str()],
        |row| row.get(0),
    ).map_err(|source| sqlite("classify complete competing finalization", source))?;
    Ok(complete)
}

fn finalization_identity_matches(
    connection: &Connection,
    candidate: &PreparedDeploymentFinalizationAtom,
) -> Result<bool, ControlStoreError> {
    let replay = candidate.replay();
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_deployment_finalization \
             WHERE admin_authority_ref=?1 AND admin_authority_sha256=?2 \
               AND idempotency_key=?3 AND candidate_identity_sha256=?4)",
            params![
                replay.admin_authority().record_ref().as_str(),
                replay.admin_authority().record_sha256().as_str(),
                replay.idempotency_key().as_str(),
                finalization_identity(candidate)
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare finalization candidate identity", source))
}

fn finalization_mutation_material_exists(
    connection: &Connection,
    candidate: &PreparedDeploymentFinalizationAtom,
) -> Result<bool, ControlStoreError> {
    let replay = candidate.replay();
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_deployment_finalization_mutation \
             WHERE admin_authority_ref=?1 AND admin_authority_sha256=?2 \
               AND idempotency_key=?3 LIMIT 1)",
            params![
                replay.admin_authority().record_ref().as_str(),
                replay.admin_authority().record_sha256().as_str(),
                replay.idempotency_key().as_str()
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare orphan finalization closure", source))
}

fn finalization_key_exists(
    connection: &Connection,
    candidate: &PreparedDeploymentFinalizationAtom,
) -> Result<bool, ControlStoreError> {
    let replay = candidate.replay();
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pale_principal_deployment_finalization WHERE admin_authority_ref=?1 AND admin_authority_sha256=?2 AND idempotency_key=?3)",
        params![replay.admin_authority().record_ref().as_str(), replay.admin_authority().record_sha256().as_str(), replay.idempotency_key().as_str()], |row| row.get(0))
        .map_err(|source| sqlite("compare finalization replay key", source))
}

fn finalization_close_matches(
    connection: &Connection,
    candidate: &PreparedDeploymentFinalizationAtom,
) -> Result<bool, ControlStoreError> {
    let replay = candidate.replay();
    let r = candidate.records();
    let pt = candidate.predecessor_trust();
    let st = candidate.successor_trust();
    let pc = candidate.predecessor_control();
    let sc = candidate.successor_control();
    let batch = candidate.accumulator();
    let transaction = batch.binding().transaction().successor();
    let coordinates_match: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pale_principal_deployment_finalization WHERE \
         admin_authority_ref=?1 AND admin_authority_sha256=?2 AND idempotency_key=?3 AND admin_command_id=?4 \
         AND command_ref=?5 AND command_sha256=?6 AND receipt_id=?7 AND receipt_ref=?8 AND receipt_sha256=?9 \
         AND binding_ref=?10 AND binding_sha256=?11 AND checkpoint_signer_ref=?12 AND checkpoint_signer_sha256=?13 \
         AND runtime_authority_ref=?14 AND runtime_authority_sha256=?15 AND trust_checkpoint_ref=?16 AND trust_checkpoint_sha256=?17 \
         AND control_checkpoint_ref=?18 AND control_checkpoint_sha256=?19 AND transaction_sequence=?20 AND principal_id=?21 \
         AND predecessor_trust_epoch=?22 AND predecessor_trust_ref=?23 AND predecessor_trust_sha256=?24 \
         AND successor_trust_epoch=?25 AND successor_trust_ref=?26 AND successor_trust_sha256=?27 AND control_domain_id=?28 \
         AND predecessor_control_epoch=?29 AND predecessor_control_sequence=?30 AND predecessor_control_ref=?31 AND predecessor_control_sha256=?32 \
         AND successor_control_epoch=?33 AND successor_control_sequence=?34 AND successor_control_ref=?35 AND successor_control_sha256=?36 \
         AND checkpoint_profile_ref=?37 AND checkpoint_profile_sha256=?38 AND predecessor_state_root_sha256=?39 \
         AND successor_state_root_sha256=?40 AND predecessor_unresolved_root_sha256=?41 AND successor_unresolved_root_sha256=?42 AND mutation_count=?43)",
        params![replay.admin_authority().record_ref().as_str(), replay.admin_authority().record_sha256().as_str(), replay.idempotency_key().as_str(), replay.admin_command_id().as_str(),
            r.command().coordinate().record_ref().as_str(), r.command().coordinate().record_sha256().as_str(), replay.receipt_id().as_str(), r.receipt().coordinate().record_ref().as_str(), r.receipt().coordinate().record_sha256().as_str(),
            r.binding().coordinate().record_ref().as_str(), r.binding().coordinate().record_sha256().as_str(), r.checkpoint_signing_authority().coordinate().record_ref().as_str(), r.checkpoint_signing_authority().coordinate().record_sha256().as_str(),
            r.runtime_authority().coordinate().record_ref().as_str(), r.runtime_authority().coordinate().record_sha256().as_str(), r.trust_checkpoint().coordinate().record_ref().as_str(), r.trust_checkpoint().coordinate().record_sha256().as_str(),
            r.control_checkpoint().coordinate().record_ref().as_str(), r.control_checkpoint().coordinate().record_sha256().as_str(), sqlite_integer(transaction.get())?, candidate.binding().principal_id().as_str(),
            sqlite_integer(pt.checkpoint_epoch().get())?, pt.coordinate().record_ref().as_str(), pt.coordinate().record_sha256().as_str(), sqlite_integer(st.checkpoint_epoch().get())?, st.coordinate().record_ref().as_str(), st.coordinate().record_sha256().as_str(), sc.checkpoint_domain_id().as_str(),
            sqlite_integer(pc.domain_epoch().get())?, sqlite_integer(pc.domain_sequence().get())?, pc.coordinate().record_ref().as_str(), pc.coordinate().record_sha256().as_str(), sqlite_integer(sc.domain_epoch().get())?, sqlite_integer(sc.domain_sequence().get())?, sc.coordinate().record_ref().as_str(), sc.coordinate().record_sha256().as_str(),
            batch.binding().profile_ref().as_str(), batch.binding().profile_sha256().as_str(), batch.state().predecessor_root().as_str(), batch.state().successor_root().as_str(), batch.unresolved().predecessor_root().as_str(), batch.unresolved().successor_root().as_str(), sqlite_integer(mutation_count(batch) as u64)?],
        |row| row.get(0)).map_err(|source| sqlite("compare finalization closing row", source))?;
    if !coordinates_match {
        return Ok(false);
    }
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pale_principal_deployment_finalization WHERE admin_authority_ref=?1 AND admin_authority_sha256=?2 AND idempotency_key=?3 AND command_body_sha256=?4 AND binding_body_sha256=?5 AND checkpoint_signer_body_sha256=?6 AND runtime_authority_body_sha256=?7 AND trust_checkpoint_body_sha256=?8 AND control_checkpoint_body_sha256=?9 AND receipt_body_sha256=?10 AND candidate_identity_sha256=?11)",
        params![replay.admin_authority().record_ref().as_str(), replay.admin_authority().record_sha256().as_str(), replay.idempotency_key().as_str(), body_sha256(r.command()), body_sha256(r.binding()), body_sha256(r.checkpoint_signing_authority()), body_sha256(r.runtime_authority()), body_sha256(r.trust_checkpoint()), body_sha256(r.control_checkpoint()), body_sha256(r.receipt()), finalization_identity(candidate)],
        |row| row.get(0),
    ).map_err(|source| sqlite("compare finalization retained body identities", source))
}

fn body_sha256(record: &ControlRecord) -> String {
    pale_core::sha256_hex(record.canonical_record_jcs().as_slice())
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn hash_text(hasher: &mut Sha256, value: &str) {
    hash_field(hasher, value.as_bytes());
}

fn hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_be_bytes());
}

fn hash_coordinate(hasher: &mut Sha256, coordinate: &pale_ports::ControlRecordCoordinate) {
    hash_text(hasher, coordinate.record_ref().as_str());
    hash_text(hasher, coordinate.record_sha256().as_str());
}

fn hash_record(hasher: &mut Sha256, record: &ControlRecord) {
    hash_coordinate(hasher, record.coordinate());
    hash_text(hasher, record.schema_id().as_str());
    hash_text(hasher, record.schema_sha256().as_str());
    hash_field(hasher, record.canonical_record_jcs().as_slice());
}

fn hash_head(hasher: &mut Sha256, head: &ControlCheckpointHead) {
    hash_text(hasher, head.checkpoint_domain_id().as_str());
    hash_u64(hasher, head.domain_epoch().get());
    hash_u64(hasher, head.domain_sequence().get());
    hash_u64(hasher, head.committed_transaction_sequence().get());
    hash_coordinate(hasher, head.coordinate());
}

fn hash_batch(hasher: &mut Sha256, batch: &OperationAccumulatorBatch) {
    let binding = batch.binding();
    hash_text(hasher, binding.store().store_id().as_str());
    hash_text(hasher, binding.store().incarnation_id().as_str());
    hash_text(hasher, binding.profile_ref().as_str());
    hash_text(hasher, binding.profile_sha256().as_str());
    match binding.phase() {
        pale_ports::AccumulatorCheckpointPhase::Initial => hasher.update([0]),
        pale_ports::AccumulatorCheckpointPhase::Successor(predecessor) => {
            hasher.update([1]);
            hash_head(hasher, predecessor);
        }
    }
    hash_head(hasher, binding.successor());
    hash_u64(hasher, binding.transaction().expected().get());
    hash_u64(hasher, binding.transaction().successor().get());
    for (tree, predecessor_root, successor_root, mutations) in [
        (
            0_u8,
            batch.state().predecessor_root().as_str(),
            batch.state().successor_root().as_str(),
            batch.state().mutations(),
        ),
        (
            1_u8,
            batch.unresolved().predecessor_root().as_str(),
            batch.unresolved().successor_root().as_str(),
            batch.unresolved().mutations(),
        ),
    ] {
        hasher.update([tree]);
        hash_text(hasher, predecessor_root);
        hash_text(hasher, successor_root);
        hash_u64(hasher, mutations.len() as u64);
        for mutation in mutations {
            let (kind, key) = accumulator::position_parts(mutation.position());
            hash_text(hasher, kind);
            hash_field(hasher, &key);
            for node in [mutation.predecessor(), mutation.successor()] {
                match node {
                    Some(bytes) => {
                        hasher.update([1]);
                        hash_field(hasher, bytes);
                    }
                    None => hasher.update([0]),
                }
            }
        }
    }
}

fn finalization_identity(candidate: &PreparedDeploymentFinalizationAtom) -> String {
    hash_finalization_closure(
        candidate.binding(),
        candidate.replay(),
        candidate.records(),
        candidate.predecessor_trust(),
        candidate.successor_trust(),
        candidate.predecessor_control(),
        candidate.successor_control(),
        candidate.accumulator(),
    )
}

pub(crate) fn finalization_snapshot_identity(snapshot: &DeploymentFinalizationSnapshot) -> String {
    hash_finalization_closure(
        snapshot.binding(),
        snapshot.replay(),
        snapshot.records(),
        snapshot.predecessor_trust(),
        snapshot.successor_trust(),
        snapshot.predecessor_control(),
        snapshot.successor_control(),
        snapshot.accumulator(),
    )
}

#[allow(clippy::too_many_arguments)]
fn hash_finalization_closure(
    binding: &PrincipalTrustStoreBinding,
    replay: &DeploymentFinalizationReplayIdentity,
    records: &DeploymentFinalizationRecords,
    predecessor_trust: &PrincipalTrustCheckpointHead,
    successor_trust: &PrincipalTrustCheckpointHead,
    predecessor_control: &ControlCheckpointHead,
    successor_control: &ControlCheckpointHead,
    accumulator: &OperationAccumulatorBatch,
) -> String {
    let mut hasher = Sha256::new();
    hash_text(&mut hasher, binding.trust_store_id().as_str());
    hash_text(&mut hasher, binding.trust_store_incarnation_id().as_str());
    hash_text(&mut hasher, binding.principal_id().as_str());
    hash_text(&mut hasher, replay.admin_command_id().as_str());
    hash_text(&mut hasher, replay.idempotency_key().as_str());
    hash_coordinate(&mut hasher, replay.admin_authority());
    hash_coordinate(&mut hasher, replay.command());
    hash_text(&mut hasher, replay.receipt_id().as_str());
    for record in [
        records.command(),
        records.binding(),
        records.checkpoint_signing_authority(),
        records.runtime_authority(),
        records.trust_checkpoint(),
        records.control_checkpoint(),
        records.receipt(),
    ] {
        hash_record(&mut hasher, record);
    }
    hash_text(&mut hasher, predecessor_trust.principal_id().as_str());
    hash_u64(&mut hasher, predecessor_trust.checkpoint_epoch().get());
    hash_coordinate(&mut hasher, predecessor_trust.coordinate());
    hash_text(&mut hasher, successor_trust.principal_id().as_str());
    hash_u64(&mut hasher, successor_trust.checkpoint_epoch().get());
    hash_coordinate(&mut hasher, successor_trust.coordinate());
    hash_head(&mut hasher, predecessor_control);
    hash_head(&mut hasher, successor_control);
    hash_batch(&mut hasher, accumulator);
    hex::encode(hasher.finalize())
}

fn hash_genesis_closure(
    finalization_trust_store: &AuthorityStoreCoordinate,
    binding: &pale_ports::DeploymentControlStoreBinding,
    genesis_id: &pale_core::records::DeploymentControlStoreGenesisIdV1,
    records: &pale_ports::DeploymentControlStoreGenesisRecords,
    checkpoint_heads: &[ControlCheckpointHead],
    accumulators: &[OperationAccumulatorBatch],
) -> String {
    let mut hasher = Sha256::new();
    hash_text(&mut hasher, finalization_trust_store.store_id().as_str());
    hash_text(
        &mut hasher,
        finalization_trust_store.incarnation_id().as_str(),
    );
    hash_text(&mut hasher, binding.store().store_id().as_str());
    hash_text(&mut hasher, binding.store().incarnation_id().as_str());
    hash_text(&mut hasher, binding.principal_id().as_str());
    hash_text(&mut hasher, binding.workspace_id().as_str());
    hash_coordinate(&mut hasher, binding.deployment_binding());
    hash_coordinate(&mut hasher, binding.checkpoint_signing_authority());
    hash_coordinate(&mut hasher, binding.runtime_authority());
    hash_text(&mut hasher, genesis_id.as_str());
    hash_record(&mut hasher, records.receipt());
    hash_u64(&mut hasher, checkpoint_heads.len() as u64);
    for ((record, head), batch) in records
        .checkpoints()
        .iter()
        .zip(checkpoint_heads)
        .zip(accumulators)
    {
        hash_record(&mut hasher, record);
        hash_head(&mut hasher, head);
        hash_batch(&mut hasher, batch);
    }
    hex::encode(hasher.finalize())
}

fn genesis_identity(candidate: &PreparedDeploymentControlStoreGenesisAtom) -> String {
    hash_genesis_closure(
        candidate.finalization_trust_store(),
        candidate.binding(),
        candidate.genesis_id(),
        candidate.records(),
        candidate.checkpoint_heads(),
        candidate.accumulators(),
    )
}

pub(crate) fn genesis_snapshot_identity(
    snapshot: &DeploymentControlStoreGenesisSnapshot,
) -> String {
    hash_genesis_closure(
        snapshot.finalization_trust_store(),
        snapshot.binding(),
        snapshot.genesis_id(),
        snapshot.records(),
        snapshot.checkpoint_heads(),
        snapshot.accumulators(),
    )
}

fn mutation_count(batch: &OperationAccumulatorBatch) -> usize {
    batch.state().mutations().len() + batch.unresolved().mutations().len()
}

fn finalization_mutations_match(
    connection: &Connection,
    candidate: &PreparedDeploymentFinalizationAtom,
) -> Result<bool, ControlStoreError> {
    let replay = candidate.replay();
    let expected = mutation_count(candidate.accumulator());
    let actual: i64 = connection.query_row("SELECT count(*) FROM (SELECT 1 FROM pale_principal_deployment_finalization_mutation WHERE admin_authority_ref=?1 AND admin_authority_sha256=?2 AND idempotency_key=?3 LIMIT ?4)",
        params![replay.admin_authority().record_ref().as_str(), replay.admin_authority().record_sha256().as_str(), replay.idempotency_key().as_str(), sqlite_integer((expected + 1) as u64)?], |row| row.get(0))
        .map_err(|source| sqlite("count finalization mutation closure", source))?;
    if usize::try_from(actual).ok() != Some(expected) {
        return Ok(false);
    }
    for (tree, mutations) in [
        ("state", candidate.accumulator().state().mutations()),
        (
            "unresolved",
            candidate.accumulator().unresolved().mutations(),
        ),
    ] {
        for mutation in mutations {
            let (kind, key) = accumulator::position_parts(mutation.position());
            let matches = connection.query_row("SELECT EXISTS(SELECT 1 FROM pale_principal_deployment_finalization_mutation WHERE admin_authority_ref=?1 AND admin_authority_sha256=?2 AND idempotency_key=?3 AND tree_kind=?4 AND position_kind=?5 AND position_key=?6 AND predecessor_node_bytes IS ?7 AND successor_node_bytes IS ?8)",
                params![replay.admin_authority().record_ref().as_str(), replay.admin_authority().record_sha256().as_str(), replay.idempotency_key().as_str(), tree, kind, key.as_slice(), mutation.predecessor(), mutation.successor()], |row| row.get::<_,bool>(0))
                .map_err(|source| sqlite("compare finalization mutation closure", source))?;
            if !matches {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn observe_finalization(
    connection: &Connection,
    candidate: &PreparedDeploymentFinalizationAtom,
) -> Result<DeploymentFinalizationDatabaseObservation, ControlStoreError> {
    if finalization_key_exists(connection, candidate)? {
        if !finalization_close_matches(connection, candidate)? {
            return Ok(DeploymentFinalizationDatabaseObservation::ConflictingOrPartial);
        }
        let tx = candidate.accumulator().binding().transaction().successor();
        let store = store_coordinate(candidate.binding());
        let mut exact = crate::store::transaction::records_were_inserted_in(
            connection,
            ControlStoreKind::PrincipalTrust,
            &store,
            tx,
            finalization_records(candidate),
        )? && finalization_mutations_match(connection, candidate)?;
        for coordinate in [
            candidate.replay().admin_authority(),
            candidate.predecessor_trust().coordinate(),
            candidate.predecessor_control().coordinate(),
        ] {
            exact &= crate::store::transaction::record_stamp(
                connection,
                ControlStoreKind::PrincipalTrust,
                coordinate,
            )?
            .is_some();
        }
        for record in finalization_records(candidate) {
            exact &= principal_record_matches(connection, candidate.binding(), record)?;
        }
        return Ok(if exact {
            DeploymentFinalizationDatabaseObservation::Exact
        } else {
            DeploymentFinalizationDatabaseObservation::ConflictingOrPartial
        });
    }
    let predecessor = trust_checkpoint_head_matches(connection, candidate.predecessor_trust())?
        && control_checkpoint_head_matches(connection, candidate.predecessor_control())?
        && accumulator::matches(
            connection,
            AccumulatorStore::PrincipalTrust,
            candidate.accumulator(),
            false,
        )?;
    let candidate_material = finalization_mutation_material_exists(connection, candidate)?
        || finalization_records(candidate)
            .into_iter()
            .try_fold(false, |found, record| {
                let exists = crate::store::transaction::record_stamp(
                    connection,
                    ControlStoreKind::PrincipalTrust,
                    record.coordinate(),
                )?
                .is_some();
                Ok::<_, ControlStoreError>(found || exists)
            })?;
    Ok(if predecessor && !candidate_material {
        DeploymentFinalizationDatabaseObservation::Predecessor
    } else {
        DeploymentFinalizationDatabaseObservation::ConflictingOrPartial
    })
}

fn insert_finalization_mutations(
    connection: &Connection,
    candidate: &PreparedDeploymentFinalizationAtom,
) -> Result<(), ControlStoreError> {
    let replay = candidate.replay();
    for (tree, mutations) in [
        ("state", candidate.accumulator().state().mutations()),
        (
            "unresolved",
            candidate.accumulator().unresolved().mutations(),
        ),
    ] {
        for mutation in mutations {
            let (kind, key) = accumulator::position_parts(mutation.position());
            connection.execute(
            "INSERT INTO pale_principal_deployment_finalization_mutation (admin_authority_ref,admin_authority_sha256,idempotency_key,tree_kind,position_kind,position_key,predecessor_node_bytes,successor_node_bytes) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![replay.admin_authority().record_ref().as_str(), replay.admin_authority().record_sha256().as_str(), replay.idempotency_key().as_str(), tree, kind, key.as_slice(), mutation.predecessor(), mutation.successor()])
            .map_err(|source| sqlite("insert finalization mutation closure", source))?;
        }
    }
    Ok(())
}

fn insert_finalization_close(
    connection: &Connection,
    candidate: &PreparedDeploymentFinalizationAtom,
    tx: pale_core::records::ControlTransactionSequenceV1,
) -> Result<(), ControlStoreError> {
    let replay = candidate.replay();
    let r = candidate.records();
    let pt = candidate.predecessor_trust();
    let st = candidate.successor_trust();
    let pc = candidate.predecessor_control();
    let sc = candidate.successor_control();
    let b = candidate.accumulator();
    connection.execute("INSERT INTO pale_principal_deployment_finalization (admin_authority_ref,admin_authority_sha256,idempotency_key,admin_command_id,command_ref,command_sha256,receipt_id,receipt_ref,receipt_sha256,binding_ref,binding_sha256,checkpoint_signer_ref,checkpoint_signer_sha256,runtime_authority_ref,runtime_authority_sha256,trust_checkpoint_ref,trust_checkpoint_sha256,control_checkpoint_ref,control_checkpoint_sha256,transaction_sequence,principal_id,predecessor_trust_epoch,predecessor_trust_ref,predecessor_trust_sha256,successor_trust_epoch,successor_trust_ref,successor_trust_sha256,control_domain_id,predecessor_control_epoch,predecessor_control_sequence,predecessor_control_ref,predecessor_control_sha256,successor_control_epoch,successor_control_sequence,successor_control_ref,successor_control_sha256,checkpoint_profile_ref,checkpoint_profile_sha256,predecessor_state_root_sha256,successor_state_root_sha256,predecessor_unresolved_root_sha256,successor_unresolved_root_sha256,mutation_count,command_body_sha256,binding_body_sha256,checkpoint_signer_body_sha256,runtime_authority_body_sha256,trust_checkpoint_body_sha256,control_checkpoint_body_sha256,receipt_body_sha256,candidate_identity_sha256) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,?45,?46,?47,?48,?49,?50,?51)",
        params![replay.admin_authority().record_ref().as_str(),replay.admin_authority().record_sha256().as_str(),replay.idempotency_key().as_str(),replay.admin_command_id().as_str(),r.command().coordinate().record_ref().as_str(),r.command().coordinate().record_sha256().as_str(),replay.receipt_id().as_str(),r.receipt().coordinate().record_ref().as_str(),r.receipt().coordinate().record_sha256().as_str(),r.binding().coordinate().record_ref().as_str(),r.binding().coordinate().record_sha256().as_str(),r.checkpoint_signing_authority().coordinate().record_ref().as_str(),r.checkpoint_signing_authority().coordinate().record_sha256().as_str(),r.runtime_authority().coordinate().record_ref().as_str(),r.runtime_authority().coordinate().record_sha256().as_str(),r.trust_checkpoint().coordinate().record_ref().as_str(),r.trust_checkpoint().coordinate().record_sha256().as_str(),r.control_checkpoint().coordinate().record_ref().as_str(),r.control_checkpoint().coordinate().record_sha256().as_str(),sqlite_integer(tx.get())?,candidate.binding().principal_id().as_str(),sqlite_integer(pt.checkpoint_epoch().get())?,pt.coordinate().record_ref().as_str(),pt.coordinate().record_sha256().as_str(),sqlite_integer(st.checkpoint_epoch().get())?,st.coordinate().record_ref().as_str(),st.coordinate().record_sha256().as_str(),sc.checkpoint_domain_id().as_str(),sqlite_integer(pc.domain_epoch().get())?,sqlite_integer(pc.domain_sequence().get())?,pc.coordinate().record_ref().as_str(),pc.coordinate().record_sha256().as_str(),sqlite_integer(sc.domain_epoch().get())?,sqlite_integer(sc.domain_sequence().get())?,sc.coordinate().record_ref().as_str(),sc.coordinate().record_sha256().as_str(),b.binding().profile_ref().as_str(),b.binding().profile_sha256().as_str(),b.state().predecessor_root().as_str(),b.state().successor_root().as_str(),b.unresolved().predecessor_root().as_str(),b.unresolved().successor_root().as_str(),sqlite_integer(mutation_count(b) as u64)?,body_sha256(r.command()),body_sha256(r.binding()),body_sha256(r.checkpoint_signing_authority()),body_sha256(r.runtime_authority()),body_sha256(r.trust_checkpoint()),body_sha256(r.control_checkpoint()),body_sha256(r.receipt()),finalization_identity(candidate)])
        .map(drop).map_err(|source| sqlite("insert finalization closing row", source))
}

impl pale_ports::ControlTransactionPort for SqliteDeploymentControlStoreGenesisCustody<'_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, ControlStoreError> {
        crate::deployment::require_coordinate(&self.coordinate, store)?;
        crate::store::transaction::prepare_snapshot(
            self.connection,
            store,
            ControlStoreKind::DeploymentControl,
        )
    }
}

impl DeploymentControlStoreGenesisCustodyPort for SqliteDeploymentControlStoreGenesisCustody<'_> {
    fn compare_and_commit(
        &mut self,
        candidate: &PreparedDeploymentControlStoreGenesisAtom,
    ) -> Result<DeploymentControlStoreGenesisCommitDisposition, ControlStoreError> {
        require_genesis_candidate(&self.coordinate, candidate, self.maximum_record_bytes)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|source| sqlite("begin deployment genesis", source))?;
        crate::deployment::require_genesis_binding(&transaction, candidate.binding())?;
        match observe_genesis(&transaction, candidate)? {
            DeploymentControlStoreGenesisDatabaseObservation::Exact => {
                transaction
                    .rollback()
                    .map_err(|source| sqlite("close exact deployment genesis replay", source))?;
                Ok(DeploymentControlStoreGenesisCommitDisposition::ExactReplay)
            }
            DeploymentControlStoreGenesisDatabaseObservation::ConflictingOrPartial => {
                if genesis_close_exists(&transaction)? {
                    if genesis_identity_matches(&transaction, candidate)? {
                        Err(recovery())
                    } else {
                        Err(conflict())
                    }
                } else {
                    Err(recovery())
                }
            }
            DeploymentControlStoreGenesisDatabaseObservation::Empty => {
                let prepared =
                    crate::store::transaction::prepare(&transaction, candidate.binding().store())?;
                if candidate
                    .accumulators()
                    .iter()
                    .any(|batch| batch.binding().transaction() != &prepared)
                    || candidate
                        .checkpoint_heads()
                        .iter()
                        .any(|head| head.committed_transaction_sequence() != prepared.successor())
                {
                    return Err(conflict());
                }
                crate::store::transaction::advance(&transaction, &prepared)?;
                for record in candidate.records().checkpoints() {
                    insert_deployment_record(&transaction, record)?;
                }
                insert_deployment_record(&transaction, candidate.records().receipt())?;
                for (ordinal, (head, batch)) in candidate
                    .checkpoint_heads()
                    .iter()
                    .zip(candidate.accumulators())
                    .enumerate()
                {
                    insert_initial_checkpoint(&transaction, head)?;
                    accumulator::apply(&transaction, AccumulatorStore::DeploymentOperation, batch)?;
                    insert_genesis_checkpoint_closure(
                        &transaction,
                        candidate,
                        ordinal,
                        head,
                        batch,
                    )?;
                }
                insert_genesis_close(&transaction, candidate, prepared.successor())?;
                if observe_genesis(&transaction, candidate)?
                    != DeploymentControlStoreGenesisDatabaseObservation::Exact
                {
                    return Err(recovery());
                }
                transaction.commit().map_err(|source| {
                    ControlStoreError::with_source(
                        ControlStoreErrorKind::OutcomeUnknown,
                        ControlSqliteError::sqlite("commit deployment genesis", source),
                    )
                })?;
                Ok(DeploymentControlStoreGenesisCommitDisposition::New)
            }
        }
    }
    fn observe_exact(
        &mut self,
        candidate: &PreparedDeploymentControlStoreGenesisAtom,
    ) -> Result<DeploymentControlStoreGenesisDatabaseObservation, ControlStoreError> {
        require_genesis_candidate(&self.coordinate, candidate, self.maximum_record_bytes)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin deployment genesis read-back", source))?;
        crate::deployment::require_genesis_binding(&transaction, candidate.binding())?;
        let observed = observe_genesis(&transaction, candidate)?;
        transaction
            .rollback()
            .map_err(|source| sqlite("close deployment genesis read-back", source))?;
        Ok(observed)
    }
}

fn require_genesis_candidate(
    coordinate: &AuthorityStoreCoordinate,
    candidate: &PreparedDeploymentControlStoreGenesisAtom,
    maximum: usize,
) -> Result<(), ControlStoreError> {
    crate::deployment::require_coordinate(coordinate, candidate.binding().store())?;
    if candidate.finalization_trust_store() == candidate.binding().store()
        || candidate.checkpoint_heads().is_empty()
        || candidate.checkpoint_heads().len() > MAX_GENESIS_CHECKPOINTS
    {
        return Err(recovery());
    }
    require_bound(
        candidate
            .records()
            .checkpoints()
            .iter()
            .chain(std::iter::once(candidate.records().receipt())),
        maximum,
    )?;
    for batch in candidate.accumulators() {
        accumulator::validate(batch)?;
    }
    Ok(())
}
fn genesis_close_exists(connection: &Connection) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_control_store_genesis WHERE singleton=1)",
            [],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare deployment genesis close", source))
}
fn genesis_identity_matches(
    connection: &Connection,
    candidate: &PreparedDeploymentControlStoreGenesisAtom,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_control_store_genesis \
             WHERE singleton=1 AND candidate_identity_sha256=?1)",
            [genesis_identity(candidate)],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare deployment genesis candidate identity", source))
}
fn genesis_close_matches(
    connection: &Connection,
    c: &PreparedDeploymentControlStoreGenesisAtom,
) -> Result<bool, ControlStoreError> {
    let b = c.binding();
    let r = c.records().receipt();
    let tx = c.accumulators()[0].binding().transaction().successor();
    let coordinates_match: bool = connection.query_row(
    "SELECT EXISTS(SELECT 1 FROM pale_deployment_control_store_genesis WHERE singleton=1 AND genesis_id=?1 AND principal_id=?2 AND workspace_id=?3 AND deployment_binding_ref=?4 AND deployment_binding_sha256=?5 AND checkpoint_signer_ref=?6 AND checkpoint_signer_sha256=?7 AND runtime_authority_ref=?8 AND runtime_authority_sha256=?9 AND finalization_store_id=?10 AND finalization_incarnation_id=?11 AND receipt_ref=?12 AND receipt_sha256=?13 AND transaction_sequence=?14 AND checkpoint_count=?15)",
    params![c.genesis_id().as_str(),b.principal_id().as_str(),b.workspace_id().as_str(),b.deployment_binding().record_ref().as_str(),b.deployment_binding().record_sha256().as_str(),b.checkpoint_signing_authority().record_ref().as_str(),b.checkpoint_signing_authority().record_sha256().as_str(),b.runtime_authority().record_ref().as_str(),b.runtime_authority().record_sha256().as_str(),c.finalization_trust_store().store_id().as_str(),c.finalization_trust_store().incarnation_id().as_str(),r.coordinate().record_ref().as_str(),r.coordinate().record_sha256().as_str(),sqlite_integer(tx.get())?,sqlite_integer(c.checkpoint_heads().len() as u64)?],|row|row.get(0)).map_err(|source|sqlite("compare deployment genesis closing row",source))?;
    if !coordinates_match {
        return Ok(false);
    }
    connection.query_row(
        "SELECT receipt_body_sha256=?1 AND candidate_identity_sha256=?2 FROM pale_deployment_control_store_genesis WHERE singleton=1",
        params![body_sha256(r), genesis_identity(c)],
        |row| row.get(0),
    ).map_err(|source| sqlite("compare deployment genesis receipt identity", source))
}

fn genesis_members_match(
    connection: &Connection,
    c: &PreparedDeploymentControlStoreGenesisAtom,
) -> Result<bool, ControlStoreError> {
    let checkpoint_rows: i64 = connection
        .query_row(
            "SELECT count(*) FROM (SELECT 1 FROM pale_deployment_control_store_genesis_checkpoint WHERE genesis_id=?1 LIMIT ?2)",
            params![c.genesis_id().as_str(), sqlite_integer((c.checkpoint_heads().len() + 1) as u64)?],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("count genesis checkpoint closure", source))?;
    if usize::try_from(checkpoint_rows).ok() != Some(c.checkpoint_heads().len()) {
        return Ok(false);
    }
    for (ordinal, (head, batch)) in c
        .checkpoint_heads()
        .iter()
        .zip(c.accumulators())
        .enumerate()
    {
        let expected_mutations = mutation_count(batch);
        let count: i64=connection.query_row("SELECT count(*) FROM (SELECT 1 FROM pale_deployment_control_store_genesis_mutation WHERE genesis_id=?1 AND checkpoint_ordinal=?2 LIMIT ?3)",params![c.genesis_id().as_str(),sqlite_integer(ordinal as u64)?,sqlite_integer((expected_mutations + 1) as u64)?],|row|row.get(0)).map_err(|source|sqlite("count genesis mutation closure",source))?;
        if usize::try_from(count).ok() != Some(expected_mutations) {
            return Ok(false);
        }
        let member=connection.query_row("SELECT EXISTS(SELECT 1 FROM pale_deployment_control_store_genesis_checkpoint WHERE genesis_id=?1 AND checkpoint_ordinal=?2 AND checkpoint_domain_id=?3 AND domain_epoch=?4 AND domain_sequence=?5 AND checkpoint_ref=?6 AND checkpoint_sha256=?7 AND checkpoint_profile_ref=?8 AND checkpoint_profile_sha256=?9 AND state_root_sha256=?10 AND unresolved_root_sha256=?11 AND mutation_count=?12 AND checkpoint_body_sha256=?13)",params![c.genesis_id().as_str(),sqlite_integer(ordinal as u64)?,head.checkpoint_domain_id().as_str(),sqlite_integer(head.domain_epoch().get())?,sqlite_integer(head.domain_sequence().get())?,head.coordinate().record_ref().as_str(),head.coordinate().record_sha256().as_str(),batch.binding().profile_ref().as_str(),batch.binding().profile_sha256().as_str(),batch.state().successor_root().as_str(),batch.unresolved().successor_root().as_str(),sqlite_integer(mutation_count(batch) as u64)?,body_sha256(&c.records().checkpoints()[ordinal])],|row|row.get::<_,bool>(0)).map_err(|source|sqlite("compare genesis checkpoint closure",source))?;
        if !member {
            return Ok(false);
        }
        for (tree, mutations) in [
            ("state", batch.state().mutations()),
            ("unresolved", batch.unresolved().mutations()),
        ] {
            for mutation in mutations {
                let (kind, key) = accumulator::position_parts(mutation.position());
                let exact=connection.query_row("SELECT EXISTS(SELECT 1 FROM pale_deployment_control_store_genesis_mutation WHERE genesis_id=?1 AND checkpoint_ordinal=?2 AND tree_kind=?3 AND position_kind=?4 AND position_key=?5 AND successor_node_bytes IS ?6)",params![c.genesis_id().as_str(),sqlite_integer(ordinal as u64)?,tree,kind,key.as_slice(),mutation.successor()],|row|row.get::<_,bool>(0)).map_err(|source|sqlite("compare genesis mutation closure",source))?;
                if !exact {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}
fn observe_genesis(
    connection: &Connection,
    c: &PreparedDeploymentControlStoreGenesisAtom,
) -> Result<DeploymentControlStoreGenesisDatabaseObservation, ControlStoreError> {
    if genesis_close_exists(connection)? {
        if !genesis_close_matches(connection, c)? || !genesis_members_match(connection, c)? {
            return Ok(DeploymentControlStoreGenesisDatabaseObservation::ConflictingOrPartial);
        }
        let tx = c.accumulators()[0].binding().transaction().successor();
        let records = c
            .records()
            .checkpoints()
            .iter()
            .chain(std::iter::once(c.records().receipt()));
        if !crate::store::transaction::records_were_inserted_in(
            connection,
            ControlStoreKind::DeploymentControl,
            c.binding().store(),
            tx,
            records,
        )? {
            return Ok(DeploymentControlStoreGenesisDatabaseObservation::ConflictingOrPartial);
        }
        for record in c
            .records()
            .checkpoints()
            .iter()
            .chain(std::iter::once(c.records().receipt()))
        {
            if !deployment_record_matches(connection, record)? {
                return Ok(DeploymentControlStoreGenesisDatabaseObservation::ConflictingOrPartial);
            }
        }
        return Ok(DeploymentControlStoreGenesisDatabaseObservation::Exact);
    }
    let empty:bool=connection.query_row("SELECT NOT EXISTS(SELECT 1 FROM pale_deployment_control_record LIMIT 1) AND NOT EXISTS(SELECT 1 FROM pale_deployment_operation_checkpoint_head LIMIT 1) AND NOT EXISTS(SELECT 1 FROM pale_deployment_operation_accumulator_frontier LIMIT 1) AND NOT EXISTS(SELECT 1 FROM pale_deployment_operation_accumulator_position LIMIT 1) AND NOT EXISTS(SELECT 1 FROM pale_deployment_control_store_genesis_checkpoint LIMIT 1) AND NOT EXISTS(SELECT 1 FROM pale_deployment_control_store_genesis_mutation LIMIT 1)",[],|row|row.get(0)).map_err(|source|sqlite("classify empty deployment genesis",source))?;
    Ok(if empty {
        DeploymentControlStoreGenesisDatabaseObservation::Empty
    } else {
        DeploymentControlStoreGenesisDatabaseObservation::ConflictingOrPartial
    })
}
fn insert_initial_checkpoint(
    connection: &Connection,
    h: &ControlCheckpointHead,
) -> Result<(), ControlStoreError> {
    connection.execute("INSERT INTO pale_deployment_operation_checkpoint_head (checkpoint_domain_id,domain_epoch,domain_sequence,control_checkpoint_ref,control_checkpoint_sha256) VALUES (?1,?2,?3,?4,?5)",params![h.checkpoint_domain_id().as_str(),sqlite_integer(h.domain_epoch().get())?,sqlite_integer(h.domain_sequence().get())?,h.coordinate().record_ref().as_str(),h.coordinate().record_sha256().as_str()]).map(drop).map_err(|source|sqlite("insert deployment genesis checkpoint head",source))
}
fn insert_genesis_checkpoint_closure(
    connection: &Connection,
    c: &PreparedDeploymentControlStoreGenesisAtom,
    ordinal: usize,
    h: &ControlCheckpointHead,
    b: &OperationAccumulatorBatch,
) -> Result<(), ControlStoreError> {
    connection.execute("INSERT INTO pale_deployment_control_store_genesis_checkpoint (genesis_id,checkpoint_ordinal,checkpoint_domain_id,domain_epoch,domain_sequence,checkpoint_ref,checkpoint_sha256,checkpoint_profile_ref,checkpoint_profile_sha256,state_root_sha256,unresolved_root_sha256,mutation_count,checkpoint_body_sha256) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![c.genesis_id().as_str(),sqlite_integer(ordinal as u64)?,h.checkpoint_domain_id().as_str(),sqlite_integer(h.domain_epoch().get())?,sqlite_integer(h.domain_sequence().get())?,h.coordinate().record_ref().as_str(),h.coordinate().record_sha256().as_str(),b.binding().profile_ref().as_str(),b.binding().profile_sha256().as_str(),b.state().successor_root().as_str(),b.unresolved().successor_root().as_str(),sqlite_integer(mutation_count(b) as u64)?,body_sha256(&c.records().checkpoints()[ordinal])]).map_err(|source|sqlite("insert genesis checkpoint closure",source))?;
    for (tree, mutations) in [
        ("state", b.state().mutations()),
        ("unresolved", b.unresolved().mutations()),
    ] {
        for mutation in mutations {
            let (kind, key) = accumulator::position_parts(mutation.position());
            connection.execute("INSERT INTO pale_deployment_control_store_genesis_mutation (genesis_id,checkpoint_ordinal,tree_kind,position_kind,position_key,successor_node_bytes) VALUES (?1,?2,?3,?4,?5,?6)",params![c.genesis_id().as_str(),sqlite_integer(ordinal as u64)?,tree,kind,key.as_slice(),mutation.successor()]).map_err(|source|sqlite("insert genesis mutation closure",source))?;
        }
    }
    Ok(())
}
fn insert_genesis_close(
    connection: &Connection,
    c: &PreparedDeploymentControlStoreGenesisAtom,
    tx: pale_core::records::ControlTransactionSequenceV1,
) -> Result<(), ControlStoreError> {
    let b = c.binding();
    let r = c.records().receipt();
    connection.execute("INSERT INTO pale_deployment_control_store_genesis (singleton,genesis_id,principal_id,workspace_id,deployment_binding_ref,deployment_binding_sha256,checkpoint_signer_ref,checkpoint_signer_sha256,runtime_authority_ref,runtime_authority_sha256,finalization_store_id,finalization_incarnation_id,receipt_ref,receipt_sha256,transaction_sequence,checkpoint_count,receipt_body_sha256,candidate_identity_sha256) VALUES (1,?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",params![c.genesis_id().as_str(),b.principal_id().as_str(),b.workspace_id().as_str(),b.deployment_binding().record_ref().as_str(),b.deployment_binding().record_sha256().as_str(),b.checkpoint_signing_authority().record_ref().as_str(),b.checkpoint_signing_authority().record_sha256().as_str(),b.runtime_authority().record_ref().as_str(),b.runtime_authority().record_sha256().as_str(),c.finalization_trust_store().store_id().as_str(),c.finalization_trust_store().incarnation_id().as_str(),r.coordinate().record_ref().as_str(),r.coordinate().record_sha256().as_str(),sqlite_integer(tx.get())?,sqlite_integer(c.checkpoint_heads().len() as u64)?,body_sha256(r),genesis_identity(c)]).map(drop).map_err(|source|sqlite("insert deployment genesis closing row",source))
}

#[cfg(test)]
#[path = "../tests/internal/deployment_initialization_custody.rs"]
mod tests;
