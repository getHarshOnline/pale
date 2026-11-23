//! `SQLite` compare-and-set custody for an admitted principal bootstrap atom.
//!
//! The adapter compares opaque canonical bytes and typed physical coordinates;
//! it never decodes a signed record or constructs semantic trust. The closing
//! head is inserted last inside one immediate transaction, so any observable
//! complete state has the exact row cardinality and reference closure prepared
//! by [`PreparedPrincipalBootstrapAtom`].

use pale_ports::{
    ControlStoreError, ControlStoreErrorKind, PreparedPrincipalBootstrapAtom,
    PrincipalBootstrapCommitDisposition, PrincipalBootstrapCustodyPort,
    PrincipalBootstrapDatabaseObservation, PrincipalBootstrapHead,
};
use rusqlite::{params, Connection, Transaction, TransactionBehavior};

use crate::accumulator::{self, Store as AccumulatorStore};
use crate::error::{sqlite, sqlite_integer};
use crate::principal_trust::{
    binding_matches, insert_record, record_matches, trust_checkpoint_head_matches,
};
use crate::ControlSqliteError;

/// `SQLite` realization of the narrow principal-bootstrap custody port.
///
/// Construction stays crate-private until the surrounding store-open and
/// storage-profile admission path is complete. That prevents an arbitrary
/// [`Connection`] from being mistaken for a runtime-admitted trust store while
/// still compiling and testing the production physical mechanics.
pub struct SqlitePrincipalBootstrapCustody<'connection> {
    /// Open connection whose surrounding Pale Runtime assembly remains external.
    pub(crate) connection: &'connection mut Connection,
}

impl pale_ports::ControlTransactionPort for SqlitePrincipalBootstrapCustody<'_> {
    fn prepare_transaction(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, ControlStoreError> {
        crate::store::transaction::prepare_snapshot(
            self.connection,
            store,
            crate::ControlStoreKind::PrincipalTrust,
        )
    }
}

impl PrincipalBootstrapCustodyPort for SqlitePrincipalBootstrapCustody<'_> {
    fn prepare_accumulator(
        &mut self,
        request: &pale_ports::OperationAccumulatorPreparation,
    ) -> Result<
        pale_ports::PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        match request.implementation() {
            pale_ports::AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1 => {}
        }
        if !matches!(
            request.basis(),
            pale_ports::AccumulatorPreparationBasis::Initial(_)
        ) {
            return Err(ControlStoreError::new(ControlStoreErrorKind::CandidateConflict).into());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin principal accumulator preparation", source))?;
        let prepared = (|| {
            let is_principal: bool = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM pale_control_store_header AS header \
                 JOIN pale_principal_trust_store_binding AS binding USING (singleton) \
                 WHERE header.singleton = 1 AND header.store_kind = 'principal_trust')",
                    [],
                    |row| row.get(0),
                )
                .map_err(|source| sqlite("compare principal accumulator store class", source))?;
            if !is_principal {
                return Err(ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired).into());
            }
            let current = crate::store::transaction::read(&transaction, request.basis().store())?;
            if current != request.transaction().expected() {
                return Err(
                    ControlStoreError::new(ControlStoreErrorKind::CandidateConflict).into(),
                );
            }
            match row_counts(&transaction)?.classify() {
                DatabaseShape::Empty => {}
                DatabaseShape::Complete => {
                    return Err(
                        ControlStoreError::new(ControlStoreErrorKind::CandidateConflict).into(),
                    )
                }
                DatabaseShape::PartialOrConflicting => {
                    return Err(
                        ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired).into(),
                    )
                }
            }
            accumulator::preparation::prepare(
                &transaction,
                AccumulatorStore::PrincipalTrust,
                request,
            )
        })();
        transaction
            .rollback()
            .map_err(|source| sqlite("release principal accumulator preparation", source))?;
        prepared
    }

    fn compare_and_commit(
        &mut self,
        candidate: &PreparedPrincipalBootstrapAtom,
    ) -> Result<PrincipalBootstrapCommitDisposition, ControlStoreError> {
        for batch in candidate.accumulators() {
            accumulator::validate(batch)?;
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|source| sqlite("begin principal bootstrap", source))?;
        if !binding_matches(&transaction, candidate.binding())? {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        let store = crate::principal_trust::store_coordinate(candidate.binding());
        crate::store::transaction::read(&transaction, &store)?;
        let observed = row_counts(&transaction)?;
        match observed.classify() {
            DatabaseShape::Empty => {
                let prepared = crate::store::transaction::prepare(&transaction, &store)?;
                if candidate
                    .accumulators()
                    .iter()
                    .any(|batch| batch.binding().transaction() != &prepared)
                {
                    return Err(ControlStoreError::new(
                        ControlStoreErrorKind::CandidateConflict,
                    ));
                }
                if candidate.control_checkpoints().iter().any(|checkpoint| {
                    checkpoint.committed_transaction_sequence() != prepared.successor()
                }) {
                    return Err(ControlStoreError::new(
                        ControlStoreErrorKind::CandidateConflict,
                    ));
                }
                crate::store::transaction::advance(&transaction, &prepared)?;
                insert_candidate(&transaction, candidate)?;
                if !candidate_matches(&transaction, candidate, row_counts(&transaction)?)? {
                    return Err(ControlStoreError::new(
                        ControlStoreErrorKind::RecoveryRequired,
                    ));
                }
                transaction.commit().map_err(|source| {
                    ControlStoreError::with_source(
                        ControlStoreErrorKind::OutcomeUnknown,
                        ControlSqliteError::sqlite("commit principal bootstrap", source),
                    )
                })?;
                Ok(PrincipalBootstrapCommitDisposition::New)
            }
            DatabaseShape::Complete if candidate_matches(&transaction, candidate, observed)? => {
                transaction
                    .rollback()
                    .map_err(|source| sqlite("close exact bootstrap replay", source))?;
                Ok(PrincipalBootstrapCommitDisposition::ExactReplay)
            }
            DatabaseShape::Complete => {
                // An exact closing head promises this candidate's retained
                // records and tree. Missing or different custody behind that
                // head is damage, not evidence of a competing bootstrap.
                let kind = if head_matches(&transaction, candidate.head())? {
                    ControlStoreErrorKind::RecoveryRequired
                } else {
                    ControlStoreErrorKind::CandidateConflict
                };
                Err(ControlStoreError::new(kind))
            }
            DatabaseShape::PartialOrConflicting => Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            )),
        }
    }

    fn observe_exact(
        &mut self,
        candidate: &PreparedPrincipalBootstrapAtom,
    ) -> Result<PrincipalBootstrapDatabaseObservation, ControlStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin principal bootstrap read-back", source))?;
        let observation = observe_exact_in(&transaction, candidate);
        transaction
            .rollback()
            .map_err(|source| sqlite("release principal bootstrap read-back", source))?;
        observation
    }
}

/// Compare the complete atom within one read snapshot of the selected store.
fn observe_exact_in(
    transaction: &Transaction<'_>,
    candidate: &PreparedPrincipalBootstrapAtom,
) -> Result<PrincipalBootstrapDatabaseObservation, ControlStoreError> {
    for batch in candidate.accumulators() {
        accumulator::validate(batch)?;
    }
    if !binding_matches(transaction, candidate.binding())? {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    let store = crate::principal_trust::store_coordinate(candidate.binding());
    crate::store::transaction::read(transaction, &store)?;
    let observed = row_counts(transaction)?;
    match observed.classify() {
        DatabaseShape::Empty => Ok(PrincipalBootstrapDatabaseObservation::Empty),
        DatabaseShape::Complete if candidate_matches(transaction, candidate, observed)? => {
            Ok(PrincipalBootstrapDatabaseObservation::Exact)
        }
        DatabaseShape::Complete | DatabaseShape::PartialOrConflicting => {
            Ok(PrincipalBootstrapDatabaseObservation::ConflictingOrPartial)
        }
    }
}

/// Coarse physical bootstrap shape before exact candidate comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DatabaseShape {
    Empty,
    Complete,
    PartialOrConflicting,
}

/// Physical row cardinalities observed without interpreting record bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RowCounts {
    records: i64,
    heads: i64,
    trust_checkpoint_heads: i64,
    checkpoints: i64,
    checkpoint_heads: i64,
    declared_checkpoints: i64,
    accumulator_positions: i64,
    accumulator_frontiers: i64,
}

impl RowCounts {
    /// Reduce physical cardinalities to the only three initial-v1 shapes.
    fn classify(self) -> DatabaseShape {
        if self
            == (Self {
                records: 0,
                heads: 0,
                trust_checkpoint_heads: 0,
                checkpoints: 0,
                checkpoint_heads: 0,
                declared_checkpoints: 0,
                accumulator_positions: 0,
                accumulator_frontiers: 0,
            })
        {
            return DatabaseShape::Empty;
        }
        if self.heads == 1
            && self.trust_checkpoint_heads == 1
            && self.checkpoints > 0
            && self.checkpoint_heads == self.checkpoints
            && self.declared_checkpoints == self.checkpoints
            && self.records
                == i64::from(PreparedPrincipalBootstrapAtom::FIXED_RECORD_COUNT) + self.checkpoints
        {
            DatabaseShape::Complete
        } else {
            DatabaseShape::PartialOrConflicting
        }
    }
}

/// Observe custody, head, and checkpoint-index cardinalities together.
fn row_counts(connection: &Connection) -> Result<RowCounts, ControlStoreError> {
    connection
        .query_row(
            "SELECT \
             (SELECT COUNT(*) FROM pale_principal_trust_record), \
             (SELECT COUNT(*) FROM pale_principal_trust_bootstrap_head), \
             (SELECT COUNT(*) FROM pale_principal_trust_checkpoint_head), \
             (SELECT COUNT(*) FROM pale_principal_bootstrap_control_checkpoint), \
             (SELECT COUNT(*) FROM pale_principal_control_checkpoint_head), \
             COALESCE((SELECT initial_control_checkpoint_count \
                       FROM pale_principal_trust_bootstrap_head WHERE singleton = 1), 0), \
             (SELECT COUNT(*) FROM pale_principal_control_accumulator_position), \
             (SELECT COUNT(*) FROM pale_principal_control_accumulator_frontier)",
            [],
            |row| {
                Ok(RowCounts {
                    records: row.get(0)?,
                    heads: row.get(1)?,
                    trust_checkpoint_heads: row.get(2)?,
                    checkpoints: row.get(3)?,
                    checkpoint_heads: row.get(4)?,
                    declared_checkpoints: row.get(5)?,
                    accumulator_positions: row.get(6)?,
                    accumulator_frontiers: row.get(7)?,
                })
            },
        )
        .map_err(|source| sqlite("classify principal bootstrap rows", source))
}

/// Insert all custody and checkpoint rows, then the singleton closing head.
fn insert_candidate(
    connection: &Connection,
    candidate: &PreparedPrincipalBootstrapAtom,
) -> Result<(), ControlStoreError> {
    for record in candidate.records() {
        insert_record(connection, candidate.binding(), record)?;
    }
    for checkpoint in candidate.control_checkpoints() {
        let domain_epoch = sqlite_integer(checkpoint.domain_epoch().get())?;
        let domain_sequence = sqlite_integer(checkpoint.domain_sequence().get())?;
        connection
            .execute(
                "INSERT INTO pale_principal_bootstrap_control_checkpoint \
                 (bootstrap_id, checkpoint_ordinal, checkpoint_domain_id, domain_epoch, \
                  domain_sequence, control_checkpoint_ref, control_checkpoint_sha256) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    candidate.head().bootstrap_id().as_str(),
                    checkpoint.ordinal(),
                    checkpoint.checkpoint_domain_id().as_str(),
                    domain_epoch,
                    domain_sequence,
                    checkpoint.coordinate().record_ref().as_str(),
                    checkpoint.coordinate().record_sha256().as_str(),
                ],
            )
            .map_err(|source| sqlite("insert principal bootstrap checkpoint index", source))?;
        connection
            .execute(
                "INSERT INTO pale_principal_control_checkpoint_head \
                 (checkpoint_domain_id, domain_epoch, domain_sequence, \
                  control_checkpoint_ref, control_checkpoint_sha256) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    checkpoint.checkpoint_domain_id().as_str(),
                    domain_epoch,
                    domain_sequence,
                    checkpoint.coordinate().record_ref().as_str(),
                    checkpoint.coordinate().record_sha256().as_str(),
                ],
            )
            .map_err(|source| sqlite("insert principal control-checkpoint head", source))?;
    }
    insert_trust_checkpoint_head(connection, candidate.head())?;
    for batch in candidate.accumulators() {
        accumulator::apply(connection, AccumulatorStore::PrincipalTrust, batch)?;
    }
    insert_head(connection, candidate.head())
}

/// Seed the mutable principal-trust frontier from the admitted epoch-one record.
fn insert_trust_checkpoint_head(
    connection: &Connection,
    head: &PrincipalBootstrapHead,
) -> Result<(), ControlStoreError> {
    let checkpoint = head.trust_checkpoint();
    let epoch = sqlite_integer(checkpoint.checkpoint_epoch().get())?;
    connection
        .execute(
            "INSERT INTO pale_principal_trust_checkpoint_head \
             (singleton, principal_id, checkpoint_epoch, trust_checkpoint_ref, \
              trust_checkpoint_sha256) VALUES (1, ?1, ?2, ?3, ?4)",
            params![
                checkpoint.principal_id().as_str(),
                epoch,
                checkpoint.coordinate().record_ref().as_str(),
                checkpoint.coordinate().record_sha256().as_str(),
            ],
        )
        .map(drop)
        .map_err(|source| sqlite("insert principal trust-checkpoint head", source))
}

/// Insert the singleton that closes the complete physical candidate set.
fn insert_head(
    connection: &Connection,
    head: &PrincipalBootstrapHead,
) -> Result<(), ControlStoreError> {
    let logical_committed_at = sqlite_integer(head.logical_committed_at().get())?;
    connection
        .execute(
            "INSERT INTO pale_principal_trust_bootstrap_head \
             (singleton, bootstrap_id, principal_record_ref, principal_record_sha256, \
              recovery_policy_ref, recovery_policy_sha256, initial_admin_authority_ref, \
              initial_admin_authority_sha256, initial_producer_incarnation_authority_ref, \
              initial_producer_incarnation_authority_sha256, \
              initial_checkpoint_signing_authority_ref, \
              initial_checkpoint_signing_authority_sha256, trust_checkpoint_ref, \
              trust_checkpoint_sha256, bootstrap_receipt_ref, bootstrap_receipt_sha256, \
              initial_control_checkpoint_count, logical_committed_at) \
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                head.bootstrap_id().as_str(),
                head.principal_record().record_ref().as_str(),
                head.principal_record().record_sha256().as_str(),
                head.recovery_policy().record_ref().as_str(),
                head.recovery_policy().record_sha256().as_str(),
                head.admin_authority().record_ref().as_str(),
                head.admin_authority().record_sha256().as_str(),
                head.producer_authority().record_ref().as_str(),
                head.producer_authority().record_sha256().as_str(),
                head.checkpoint_authority().record_ref().as_str(),
                head.checkpoint_authority().record_sha256().as_str(),
                head.trust_checkpoint().coordinate().record_ref().as_str(),
                head.trust_checkpoint()
                    .coordinate()
                    .record_sha256()
                    .as_str(),
                head.receipt().record_ref().as_str(),
                head.receipt().record_sha256().as_str(),
                head.control_checkpoint_count(),
                logical_committed_at,
            ],
        )
        .map(drop)
        .map_err(|source| sqlite("close principal bootstrap candidate set", source))
}

/// Compare every retained byte string, ordered checkpoint, and closing coordinate.
fn candidate_matches(
    connection: &Connection,
    candidate: &PreparedPrincipalBootstrapAtom,
    counts: RowCounts,
) -> Result<bool, ControlStoreError> {
    let expected_checkpoints = i64::from(candidate.head().control_checkpoint_count());
    let accumulator_positions =
        candidate
            .accumulators()
            .iter()
            .try_fold(0_i64, |total, batch| {
                batch
                    .state()
                    .mutations()
                    .len()
                    .checked_add(batch.unresolved().mutations().len())
                    .and_then(|count| i64::try_from(count).ok())
                    .and_then(|count| total.checked_add(count))
                    .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
            })?;
    let accumulator_frontiers = i64::try_from(candidate.accumulators().len())
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    if counts
        != (RowCounts {
            records: i64::from(PreparedPrincipalBootstrapAtom::FIXED_RECORD_COUNT)
                + expected_checkpoints,
            heads: 1,
            trust_checkpoint_heads: 1,
            checkpoints: expected_checkpoints,
            checkpoint_heads: expected_checkpoints,
            declared_checkpoints: expected_checkpoints,
            accumulator_positions,
            accumulator_frontiers,
        })
    {
        return Ok(false);
    }
    for batch in candidate.accumulators() {
        if !accumulator::matches(connection, AccumulatorStore::PrincipalTrust, batch, true)? {
            return Ok(false);
        }
    }
    for record in candidate.records() {
        if !record_matches(connection, candidate.binding(), record)? {
            return Ok(false);
        }
    }
    let Some(first) = candidate.control_checkpoints().first() else {
        return Ok(false);
    };
    let committed = first.committed_transaction_sequence();
    if candidate
        .control_checkpoints()
        .iter()
        .any(|checkpoint| checkpoint.committed_transaction_sequence() != committed)
        || !crate::store::transaction::records_were_inserted_in(
            connection,
            crate::ControlStoreKind::PrincipalTrust,
            &crate::principal_trust::store_coordinate(candidate.binding()),
            committed,
            candidate.records(),
        )?
    {
        // All retained candidate bytes matched above. Their inconsistent atom
        // insertion metadata is damaged custody, not a different bootstrap.
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    if !trust_checkpoint_head_matches(connection, candidate.head().trust_checkpoint())? {
        return Ok(false);
    }
    for checkpoint in candidate.control_checkpoints() {
        let domain_epoch = sqlite_integer(checkpoint.domain_epoch().get())?;
        let domain_sequence = sqlite_integer(checkpoint.domain_sequence().get())?;
        let matches = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pale_principal_bootstrap_control_checkpoint \
                 WHERE bootstrap_id = ?1 AND checkpoint_ordinal = ?2 \
                   AND checkpoint_domain_id = ?3 AND domain_epoch = ?4 \
                   AND domain_sequence = ?5 AND control_checkpoint_ref = ?6 \
                   AND control_checkpoint_sha256 = ?7)",
                params![
                    candidate.head().bootstrap_id().as_str(),
                    checkpoint.ordinal(),
                    checkpoint.checkpoint_domain_id().as_str(),
                    domain_epoch,
                    domain_sequence,
                    checkpoint.coordinate().record_ref().as_str(),
                    checkpoint.coordinate().record_sha256().as_str(),
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|source| sqlite("compare principal bootstrap checkpoint index", source))?;
        if !matches {
            return Ok(false);
        }
        let head_matches = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pale_principal_control_checkpoint_head \
                 WHERE checkpoint_domain_id = ?1 AND domain_epoch = ?2 \
                   AND domain_sequence = ?3 AND control_checkpoint_ref = ?4 \
                   AND control_checkpoint_sha256 = ?5)",
                params![
                    checkpoint.checkpoint_domain_id().as_str(),
                    domain_epoch,
                    domain_sequence,
                    checkpoint.coordinate().record_ref().as_str(),
                    checkpoint.coordinate().record_sha256().as_str(),
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|source| sqlite("compare principal control-checkpoint head", source))?;
        if !head_matches {
            return Ok(false);
        }
    }
    head_matches(connection, candidate.head())
}

/// Compare the singleton closing row as one SQL projection.
fn head_matches(
    connection: &Connection,
    head: &PrincipalBootstrapHead,
) -> Result<bool, ControlStoreError> {
    let logical_committed_at = sqlite_integer(head.logical_committed_at().get())?;
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_trust_bootstrap_head \
             WHERE singleton = 1 AND bootstrap_id = ?1 \
               AND principal_record_ref = ?2 AND principal_record_sha256 = ?3 \
               AND recovery_policy_ref = ?4 AND recovery_policy_sha256 = ?5 \
               AND initial_admin_authority_ref = ?6 AND initial_admin_authority_sha256 = ?7 \
               AND initial_producer_incarnation_authority_ref = ?8 \
               AND initial_producer_incarnation_authority_sha256 = ?9 \
               AND initial_checkpoint_signing_authority_ref = ?10 \
               AND initial_checkpoint_signing_authority_sha256 = ?11 \
               AND trust_checkpoint_ref = ?12 AND trust_checkpoint_sha256 = ?13 \
               AND bootstrap_receipt_ref = ?14 AND bootstrap_receipt_sha256 = ?15 \
               AND initial_control_checkpoint_count = ?16 AND logical_committed_at = ?17)",
            params![
                head.bootstrap_id().as_str(),
                head.principal_record().record_ref().as_str(),
                head.principal_record().record_sha256().as_str(),
                head.recovery_policy().record_ref().as_str(),
                head.recovery_policy().record_sha256().as_str(),
                head.admin_authority().record_ref().as_str(),
                head.admin_authority().record_sha256().as_str(),
                head.producer_authority().record_ref().as_str(),
                head.producer_authority().record_sha256().as_str(),
                head.checkpoint_authority().record_ref().as_str(),
                head.checkpoint_authority().record_sha256().as_str(),
                head.trust_checkpoint().coordinate().record_ref().as_str(),
                head.trust_checkpoint()
                    .coordinate()
                    .record_sha256()
                    .as_str(),
                head.receipt().record_ref().as_str(),
                head.receipt().record_sha256().as_str(),
                head.control_checkpoint_count(),
                logical_committed_at,
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare principal bootstrap closing head", source))
}

#[cfg(test)]
#[path = "../tests/internal/bootstrap.rs"]
mod tests;
