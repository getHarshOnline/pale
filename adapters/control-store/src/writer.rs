//! `SQLite` custody for the paired principal resident-writer hard frontier.
//!
//! One immediate transaction compares the current writer and control-checkpoint
//! heads, appends already-admitted canonical records, and advances both heads.
//! The adapter never decodes signed records and grants no OS lock, liveness,
//! takeover, authentication, or resident-runtime authority.

use pale_core::records::PrincipalResidentWriterFenceStateV1;
use pale_ports::{
    ControlStoreError, ControlStoreErrorKind, PreparedPrincipalResidentWriterTransition,
    PrincipalResidentWriterCommitDisposition, PrincipalResidentWriterCustodyPort,
    PrincipalResidentWriterDatabaseObservation, PrincipalResidentWriterHead,
};
use rusqlite::{params, Connection, TransactionBehavior};

use crate::error::{sqlite, sqlite_integer};
use crate::principal_trust::{
    advance_control_checkpoint_head, binding_matches, control_checkpoint_head_matches,
    insert_record, record_matches,
};
use crate::ControlSqliteError;

/// `SQLite` realization of the resident-writer custody port.
///
/// Construction remains crate-private until the surrounding profile, external
/// lock, and Pale Runtime assembly frontiers exist. Tests exercise these production
/// mechanics through an already initialized principal trust connection.
pub struct SqlitePrincipalResidentWriterCustody<'connection> {
    /// Open principal trust connection whose Pale Runtime assembly remains external.
    pub(crate) connection: &'connection mut Connection,
}

impl pale_ports::ControlTransactionPort for SqlitePrincipalResidentWriterCustody<'_> {
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

impl PrincipalResidentWriterCustodyPort for SqlitePrincipalResidentWriterCustody<'_> {
    fn compare_and_commit(
        &mut self,
        candidate: &PreparedPrincipalResidentWriterTransition,
    ) -> Result<PrincipalResidentWriterCommitDisposition, ControlStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|source| sqlite("begin principal resident-writer transition", source))?;
        if !binding_matches(&transaction, candidate.binding())? {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }

        let store = crate::principal_trust::store_coordinate(candidate.binding());
        crate::store::transaction::read(&transaction, &store)?;
        match observe(&transaction, candidate)? {
            PrincipalResidentWriterDatabaseObservation::ExactSuccessor => {
                transaction
                    .rollback()
                    .map_err(|source| sqlite("close exact resident-writer replay", source))?;
                Ok(PrincipalResidentWriterCommitDisposition::ExactReplay)
            }
            PrincipalResidentWriterDatabaseObservation::Predecessor => {
                let prepared = crate::store::transaction::prepare(&transaction, &store)?;
                if candidate
                    .next_head()
                    .checkpoint()
                    .committed_transaction_sequence()
                    != prepared.successor()
                {
                    return Err(ControlStoreError::new(
                        ControlStoreErrorKind::CandidateConflict,
                    ));
                }
                crate::store::transaction::advance(&transaction, &prepared)?;
                for record in candidate.records() {
                    insert_record(&transaction, candidate.binding(), record)?;
                }
                advance_control_checkpoint_head(
                    &transaction,
                    candidate.expected_checkpoint(),
                    candidate.next_head().checkpoint(),
                )?;
                advance_writer_head(&transaction, candidate)?;
                if observe(&transaction, candidate)?
                    != PrincipalResidentWriterDatabaseObservation::ExactSuccessor
                {
                    return Err(ControlStoreError::new(
                        ControlStoreErrorKind::RecoveryRequired,
                    ));
                }
                transaction.commit().map_err(|source| {
                    ControlStoreError::with_source(
                        ControlStoreErrorKind::OutcomeUnknown,
                        ControlSqliteError::sqlite(
                            "commit principal resident-writer transition",
                            source,
                        ),
                    )
                })?;
                Ok(PrincipalResidentWriterCommitDisposition::Committed)
            }
            PrincipalResidentWriterDatabaseObservation::ConflictingOrPartial => {
                let kind = if conflicting_frontier_is_complete(&transaction, candidate)? {
                    ControlStoreErrorKind::CandidateConflict
                } else {
                    ControlStoreErrorKind::RecoveryRequired
                };
                Err(ControlStoreError::new(kind))
            }
        }
    }

    fn observe_exact(
        &mut self,
        candidate: &PreparedPrincipalResidentWriterTransition,
    ) -> Result<PrincipalResidentWriterDatabaseObservation, ControlStoreError> {
        if !binding_matches(self.connection, candidate.binding())? {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        observe(self.connection, candidate)
    }
}

/// Classify only the predecessor and successor coordinates owned by a candidate.
fn observe(
    connection: &Connection,
    candidate: &PreparedPrincipalResidentWriterTransition,
) -> Result<PrincipalResidentWriterDatabaseObservation, ControlStoreError> {
    if successor_matches(connection, candidate)? {
        return Ok(PrincipalResidentWriterDatabaseObservation::ExactSuccessor);
    }
    if predecessor_matches(connection, candidate)? {
        return Ok(PrincipalResidentWriterDatabaseObservation::Predecessor);
    }
    Ok(PrincipalResidentWriterDatabaseObservation::ConflictingOrPartial)
}

/// Require both successor heads and the complete transition evidence exactly.
fn successor_matches(
    connection: &Connection,
    candidate: &PreparedPrincipalResidentWriterTransition,
) -> Result<bool, ControlStoreError> {
    if !successor_heads_match(connection, candidate)? {
        return Ok(false);
    }
    if !predecessor_records_match(connection, candidate)? {
        return Ok(false);
    }
    for record in candidate.records() {
        if !record_matches(connection, candidate.binding(), record)? {
            return Ok(false);
        }
    }
    crate::store::transaction::records_were_inserted_in(
        connection,
        crate::ControlStoreKind::PrincipalTrust,
        &crate::principal_trust::store_coordinate(candidate.binding()),
        candidate
            .next_head()
            .checkpoint()
            .committed_transaction_sequence(),
        candidate.records(),
    )
}

/// Require the exact predecessor heads and absence of every proposed append.
fn predecessor_matches(
    connection: &Connection,
    candidate: &PreparedPrincipalResidentWriterTransition,
) -> Result<bool, ControlStoreError> {
    if !predecessor_frontier_matches(connection, candidate)? {
        return Ok(false);
    }
    for record in candidate.records() {
        if record_ref_exists(connection, record.coordinate().record_ref().as_str())? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Compare only the predecessor heads and their immutable comparison records.
fn predecessor_frontier_matches(
    connection: &Connection,
    candidate: &PreparedPrincipalResidentWriterTransition,
) -> Result<bool, ControlStoreError> {
    if !control_checkpoint_head_matches(connection, candidate.expected_checkpoint())?
        || !predecessor_records_match(connection, candidate)?
    {
        return Ok(false);
    }
    let writer_matches = match (candidate.expected_head(), candidate.expected_record()) {
        (None, None) => writer_head_count(connection)? == 0,
        (Some(head), Some(_)) => writer_head_matches(connection, head)?,
        (None, Some(_)) | (Some(_), None) => false,
    };
    if !writer_matches {
        return Ok(false);
    }
    Ok(true)
}

/// Compare every immutable predecessor byte string carried by the candidate.
fn predecessor_records_match(
    connection: &Connection,
    candidate: &PreparedPrincipalResidentWriterTransition,
) -> Result<bool, ControlStoreError> {
    if !record_matches(
        connection,
        candidate.binding(),
        candidate.expected_checkpoint_record(),
    )? {
        return Ok(false);
    }
    match candidate.expected_record() {
        Some(record) => record_matches(connection, candidate.binding(), record),
        None => Ok(true),
    }
}

/// Compare the two successor heads without assuming every append completed.
fn successor_heads_match(
    connection: &Connection,
    candidate: &PreparedPrincipalResidentWriterTransition,
) -> Result<bool, ControlStoreError> {
    Ok(
        control_checkpoint_head_matches(connection, candidate.next_head().checkpoint())?
            && writer_head_matches(connection, candidate.next_head())?,
    )
}

/// Distinguish a stale complete frontier from partial or contradictory custody.
fn conflicting_frontier_is_complete(
    connection: &Connection,
    candidate: &PreparedPrincipalResidentWriterTransition,
) -> Result<bool, ControlStoreError> {
    if predecessor_frontier_matches(connection, candidate)?
        || successor_heads_match(connection, candidate)?
    {
        return Ok(false);
    }
    if !checkpoint_domain_head_is_complete(
        connection,
        candidate
            .expected_checkpoint()
            .checkpoint_domain_id()
            .as_str(),
    )? {
        return Ok(false);
    }
    Ok(writer_head_count(connection)? == 1 && writer_head_is_complete(connection)?)
}

/// Compare the singleton writer head and its paired checkpoint coordinate.
fn writer_head_matches(
    connection: &Connection,
    head: &PrincipalResidentWriterHead,
) -> Result<bool, ControlStoreError> {
    let generation = sqlite_integer(head.writer_generation().get())?;
    let checkpoint_epoch = sqlite_integer(head.checkpoint().domain_epoch().get())?;
    let checkpoint_sequence = sqlite_integer(head.checkpoint().domain_sequence().get())?;
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_resident_writer_head \
             WHERE singleton = 1 AND writer_fence_id = ?1 AND principal_id = ?2 \
               AND writer_generation = ?3 AND writer_state = ?4 \
               AND writer_fence_ref = ?5 AND writer_fence_sha256 = ?6 \
               AND checkpoint_domain_id = ?7 AND checkpoint_domain_epoch = ?8 \
               AND checkpoint_domain_sequence = ?9 AND control_checkpoint_ref = ?10 \
               AND control_checkpoint_sha256 = ?11)",
            params![
                head.writer_fence_id().as_str(),
                head.principal_id().as_str(),
                generation,
                writer_state(head.state()),
                head.coordinate().record_ref().as_str(),
                head.coordinate().record_sha256().as_str(),
                head.checkpoint().checkpoint_domain_id().as_str(),
                checkpoint_epoch,
                checkpoint_sequence,
                head.checkpoint().coordinate().record_ref().as_str(),
                head.checkpoint().coordinate().record_sha256().as_str(),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare principal resident-writer head", source))
}

/// Insert generation one or compare-and-swap the exact predecessor writer head.
fn advance_writer_head(
    connection: &Connection,
    candidate: &PreparedPrincipalResidentWriterTransition,
) -> Result<(), ControlStoreError> {
    match candidate.expected_head() {
        None => insert_writer_head(connection, candidate.next_head()),
        Some(expected) => update_writer_head(connection, expected, candidate.next_head()),
    }
}

/// Insert the sole generation-one writer head.
fn insert_writer_head(
    connection: &Connection,
    head: &PrincipalResidentWriterHead,
) -> Result<(), ControlStoreError> {
    let values = writer_head_values(head)?;
    connection
        .execute(
            "INSERT INTO pale_principal_resident_writer_head \
             (singleton, writer_fence_id, principal_id, writer_generation, writer_state, \
              writer_fence_ref, writer_fence_sha256, checkpoint_domain_id, \
              checkpoint_domain_epoch, checkpoint_domain_sequence, control_checkpoint_ref, \
              control_checkpoint_sha256) \
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                values.writer_fence_id,
                values.principal_id,
                values.writer_generation,
                values.writer_state,
                values.writer_fence_ref,
                values.writer_fence_sha256,
                values.checkpoint_domain_id,
                values.checkpoint_domain_epoch,
                values.checkpoint_domain_sequence,
                values.control_checkpoint_ref,
                values.control_checkpoint_sha256,
            ],
        )
        .map(drop)
        .map_err(|source| sqlite("insert principal resident-writer head", source))
}

/// Update the singleton only when every predecessor coordinate still matches.
fn update_writer_head(
    connection: &Connection,
    expected: &PrincipalResidentWriterHead,
    next: &PrincipalResidentWriterHead,
) -> Result<(), ControlStoreError> {
    let expected_values = writer_head_values(expected)?;
    let next_values = writer_head_values(next)?;
    let changed = connection
        .execute(
            "UPDATE pale_principal_resident_writer_head SET \
             writer_fence_id = ?1, principal_id = ?2, writer_generation = ?3, writer_state = ?4, \
             writer_fence_ref = ?5, writer_fence_sha256 = ?6, checkpoint_domain_id = ?7, \
             checkpoint_domain_epoch = ?8, checkpoint_domain_sequence = ?9, \
             control_checkpoint_ref = ?10, control_checkpoint_sha256 = ?11 \
             WHERE singleton = 1 AND writer_fence_id = ?12 AND principal_id = ?13 \
               AND writer_generation = ?14 AND writer_state = ?15 \
               AND writer_fence_ref = ?16 AND writer_fence_sha256 = ?17 \
               AND checkpoint_domain_id = ?18 AND checkpoint_domain_epoch = ?19 \
               AND checkpoint_domain_sequence = ?20 AND control_checkpoint_ref = ?21 \
               AND control_checkpoint_sha256 = ?22",
            params![
                next_values.writer_fence_id,
                next_values.principal_id,
                next_values.writer_generation,
                next_values.writer_state,
                next_values.writer_fence_ref,
                next_values.writer_fence_sha256,
                next_values.checkpoint_domain_id,
                next_values.checkpoint_domain_epoch,
                next_values.checkpoint_domain_sequence,
                next_values.control_checkpoint_ref,
                next_values.control_checkpoint_sha256,
                expected_values.writer_fence_id,
                expected_values.principal_id,
                expected_values.writer_generation,
                expected_values.writer_state,
                expected_values.writer_fence_ref,
                expected_values.writer_fence_sha256,
                expected_values.checkpoint_domain_id,
                expected_values.checkpoint_domain_epoch,
                expected_values.checkpoint_domain_sequence,
                expected_values.control_checkpoint_ref,
                expected_values.control_checkpoint_sha256,
            ],
        )
        .map_err(|source| sqlite("advance principal resident-writer head", source))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))
    }
}

/// Borrowed and converted SQL parameters for one writer head.
struct WriterHeadValues<'head> {
    writer_fence_id: &'head str,
    principal_id: &'head str,
    writer_generation: i64,
    writer_state: &'static str,
    writer_fence_ref: &'head str,
    writer_fence_sha256: &'head str,
    checkpoint_domain_id: &'head str,
    checkpoint_domain_epoch: i64,
    checkpoint_domain_sequence: i64,
    control_checkpoint_ref: &'head str,
    control_checkpoint_sha256: &'head str,
}

/// Convert portable integer coordinates once for SQL insertion or comparison.
fn writer_head_values(
    head: &PrincipalResidentWriterHead,
) -> Result<WriterHeadValues<'_>, ControlStoreError> {
    Ok(WriterHeadValues {
        writer_fence_id: head.writer_fence_id().as_str(),
        principal_id: head.principal_id().as_str(),
        writer_generation: sqlite_integer(head.writer_generation().get())?,
        writer_state: writer_state(head.state()),
        writer_fence_ref: head.coordinate().record_ref().as_str(),
        writer_fence_sha256: head.coordinate().record_sha256().as_str(),
        checkpoint_domain_id: head.checkpoint().checkpoint_domain_id().as_str(),
        checkpoint_domain_epoch: sqlite_integer(head.checkpoint().domain_epoch().get())?,
        checkpoint_domain_sequence: sqlite_integer(head.checkpoint().domain_sequence().get())?,
        control_checkpoint_ref: head.checkpoint().coordinate().record_ref().as_str(),
        control_checkpoint_sha256: head.checkpoint().coordinate().record_sha256().as_str(),
    })
}

/// Count the singleton writer head without interpreting it.
fn writer_head_count(connection: &Connection) -> Result<i64, ControlStoreError> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM pale_principal_resident_writer_head",
            [],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("count principal resident-writer heads", source))
}

/// Report whether one proposed immutable record reference already occupies custody.
fn record_ref_exists(connection: &Connection, record_ref: &str) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_trust_record WHERE record_ref = ?1)",
            [record_ref],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("inspect resident-writer record reference", source))
}

/// Verify that the current domain head resolves to an immutable checkpoint row.
fn checkpoint_domain_head_is_complete(
    connection: &Connection,
    checkpoint_domain_id: &str,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_control_checkpoint_head AS head \
             JOIN pale_principal_trust_record AS record \
               ON record.record_ref = head.control_checkpoint_ref \
              AND record.record_sha256 = head.control_checkpoint_sha256 \
             WHERE head.checkpoint_domain_id = ?1)",
            [checkpoint_domain_id],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("verify principal control-checkpoint head", source))
}

/// Verify that the singleton writer head closes over both retained records.
fn writer_head_is_complete(connection: &Connection) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_resident_writer_head AS writer \
             JOIN pale_principal_trust_record AS fence \
               ON fence.record_ref = writer.writer_fence_ref \
              AND fence.record_sha256 = writer.writer_fence_sha256 \
             JOIN pale_principal_control_checkpoint_head AS checkpoint \
               ON checkpoint.checkpoint_domain_id = writer.checkpoint_domain_id \
              AND checkpoint.domain_epoch = writer.checkpoint_domain_epoch \
              AND checkpoint.domain_sequence = writer.checkpoint_domain_sequence \
              AND checkpoint.control_checkpoint_ref = writer.control_checkpoint_ref \
              AND checkpoint.control_checkpoint_sha256 = writer.control_checkpoint_sha256 \
             JOIN pale_principal_trust_record AS checkpoint_record \
               ON checkpoint_record.record_ref = checkpoint.control_checkpoint_ref \
              AND checkpoint_record.record_sha256 = checkpoint.control_checkpoint_sha256 \
             WHERE writer.singleton = 1)",
            [],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("verify principal resident-writer head closure", source))
}

/// Stable SQL spelling of the closed writer state.
const fn writer_state(state: PrincipalResidentWriterFenceStateV1) -> &'static str {
    match state {
        PrincipalResidentWriterFenceStateV1::Active => "active",
        PrincipalResidentWriterFenceStateV1::Retired => "retired",
    }
}
