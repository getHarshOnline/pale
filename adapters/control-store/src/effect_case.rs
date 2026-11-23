//! Deployment-bound `SQLite` custody for admitted effect-case transitions.
//!
//! One immediate transaction compares the exact store, typed index, and
//! checkpoint predecessor; appends opaque admitted records; and advances every
//! selected head atomically. Semantic admission and native effects remain
//! outside this adapter.

use pale_core::records::ResidentRecordRefV1;
use pale_ports::{
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecordCoordinate, ControlRecordDigest,
    ControlStoreError, ControlStoreErrorKind, EffectCaseCheckpointMutation,
    EffectCaseCommitOutcome, EffectCaseCustodyPort, EffectCaseIndexAssertion, EffectCaseIndexKey,
    EffectCaseIndexMutation, EffectCaseIndexObservation, EffectCaseIndexPredecessor,
    EffectCaseIndexQuery, EffectCaseReadBackOutcome, EffectCaseRecordObservation,
    EffectCaseTransitionCandidate,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use crate::deployment::{
    insert_record, read_record as read_record_value, record_matches, record_reference_exists,
    require_coordinate, require_store_binding,
};
use crate::error::{sqlite, sqlite_integer};
use crate::ControlSqliteError;

/// `SQLite` realization of atomic effect-case custody.
///
/// Construction remains crate-private until deployment storage and resident
/// assembly are admitted. Tests exercise the production transaction over an
/// explicitly initialized local connection.
pub struct SqliteEffectCaseCustody<'connection> {
    /// Open deployment-control connection whose Pale Runtime assembly remains external.
    pub(crate) connection: &'connection mut Connection,
    /// Exact store incarnation represented by [`Self::connection`].
    pub(crate) coordinate: AuthorityStoreCoordinate,
}

impl pale_ports::ControlTransactionPort for SqliteEffectCaseCustody<'_> {
    fn prepare_transaction(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, ControlStoreError> {
        require_coordinate(&self.coordinate, store)?;
        crate::store::transaction::prepare_snapshot(
            self.connection,
            store,
            crate::ControlStoreKind::DeploymentControl,
        )
    }
}

impl EffectCaseCustodyPort for SqliteEffectCaseCustody<'_> {
    fn compare_and_commit(
        &mut self,
        candidate: &EffectCaseTransitionCandidate,
    ) -> Result<EffectCaseCommitOutcome, ControlStoreError> {
        require_coordinate(&self.coordinate, candidate.store())?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|source| sqlite("begin effect-case transition", source))?;
        require_store_binding(&transaction, candidate.store())?;
        crate::store::transaction::read(&transaction, candidate.store())?;

        match observe(&transaction, candidate)? {
            EffectCaseReadBackOutcome::ExactSuccessor => {
                transaction
                    .rollback()
                    .map_err(|source| sqlite("close exact effect-case replay", source))?;
                Ok(EffectCaseCommitOutcome::ExactReplay)
            }
            EffectCaseReadBackOutcome::Predecessor => {
                let prepared = crate::store::transaction::prepare(&transaction, candidate.store())?;
                if let EffectCaseCheckpointMutation::Advance(checkpoint) =
                    candidate.checkpoint_mutation()
                {
                    if checkpoint.successor().committed_transaction_sequence()
                        != prepared.successor()
                    {
                        return Err(ControlStoreError::new(
                            ControlStoreErrorKind::CandidateConflict,
                        ));
                    }
                }
                crate::store::transaction::advance(&transaction, &prepared)?;
                append_candidate_records(&transaction, candidate)?;
                for mutation in candidate.index_mutations() {
                    apply_index_mutation(&transaction, mutation)?;
                }
                advance_checkpoint(&transaction, candidate.checkpoint_mutation())?;
                if observe(&transaction, candidate)? != EffectCaseReadBackOutcome::ExactSuccessor {
                    return Err(ControlStoreError::new(
                        ControlStoreErrorKind::RecoveryRequired,
                    ));
                }
                transaction.commit().map_err(|source| {
                    ControlStoreError::with_source(
                        ControlStoreErrorKind::OutcomeUnknown,
                        ControlSqliteError::sqlite("commit effect-case transition", source),
                    )
                })?;
                Ok(EffectCaseCommitOutcome::Committed)
            }
            EffectCaseReadBackOutcome::ConflictingOrPartial => {
                let kind = classify_conflict(&transaction, candidate)?;
                Err(ControlStoreError::new(kind))
            }
        }
    }

    fn observe_exact(
        &mut self,
        candidate: &EffectCaseTransitionCandidate,
    ) -> Result<EffectCaseReadBackOutcome, ControlStoreError> {
        require_coordinate(&self.coordinate, candidate.store())?;
        require_store_binding(self.connection, candidate.store())?;
        observe(self.connection, candidate)
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &EffectCaseIndexQuery,
    ) -> Result<EffectCaseIndexObservation, ControlStoreError> {
        require_coordinate(&self.coordinate, store)?;
        require_store_binding(self.connection, store)?;
        let role = query_role(query);
        let Some(coordinate) = read_index_coordinate(self.connection, role, query.key())? else {
            return Ok(EffectCaseIndexObservation::absent(query.clone()));
        };
        let record = read_record_value(self.connection, &coordinate)?
            .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        EffectCaseIndexObservation::exact_record(query.clone(), record)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
    }

    fn read_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
    ) -> Result<EffectCaseRecordObservation, ControlStoreError> {
        require_coordinate(&self.coordinate, store)?;
        require_store_binding(self.connection, store)?;
        match read_record_value(self.connection, coordinate)? {
            Some(record) => EffectCaseRecordObservation::exact(coordinate.clone(), record)
                .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)),
            None => Ok(EffectCaseRecordObservation::absent(coordinate.clone())),
        }
    }
}

/// Classify only the exact predecessor and successor represented by `candidate`.
fn observe(
    connection: &Connection,
    candidate: &EffectCaseTransitionCandidate,
) -> Result<EffectCaseReadBackOutcome, ControlStoreError> {
    if successor_matches(connection, candidate)? {
        return Ok(EffectCaseReadBackOutcome::ExactSuccessor);
    }
    if predecessor_matches(connection, candidate)? {
        return Ok(EffectCaseReadBackOutcome::Predecessor);
    }
    Ok(EffectCaseReadBackOutcome::ConflictingOrPartial)
}

/// Require every appended record and selected successor head byte-for-byte.
fn successor_matches(
    connection: &Connection,
    candidate: &EffectCaseTransitionCandidate,
) -> Result<bool, ControlStoreError> {
    if !expected_records_match(connection, candidate)? {
        return Ok(false);
    }
    for member in candidate.members() {
        if !record_matches(connection, member.record())? {
            return Ok(false);
        }
    }
    if let Some(record) = candidate.successor_checkpoint_record() {
        if !record_matches(connection, record)? {
            return Ok(false);
        }
    }
    let committed = if let Some(checkpoint) = candidate.successor_checkpoint() {
        checkpoint.committed_transaction_sequence()
    } else {
        let Some(first) = candidate.members().first() else {
            return Ok(false);
        };
        let Some(inserted) = crate::store::transaction::record_stamp(
            connection,
            crate::ControlStoreKind::DeploymentControl,
            first.record().coordinate(),
        )?
        else {
            return Ok(false);
        };
        inserted
    };
    if !crate::store::transaction::records_were_inserted_in(
        connection,
        crate::ControlStoreKind::DeploymentControl,
        candidate.store(),
        committed,
        candidate
            .members()
            .iter()
            .map(|member| member.record())
            .chain(candidate.successor_checkpoint_record()),
    )? {
        return Ok(false);
    }
    for mutation in candidate.index_mutations() {
        let (role, key, _, successor) = mutation_parts(mutation);
        if !index_points_to(connection, role, key, successor)? {
            return Ok(false);
        }
    }
    for assertion in candidate.index_assertions() {
        let (role, key, expected) = assertion_parts(assertion);
        if !index_matches(connection, role, key, expected)? {
            return Ok(false);
        }
    }
    checkpoint_successor_matches(connection, candidate.checkpoint_mutation())
}

/// Require the exact predecessor and absence of every proposed append.
fn predecessor_matches(
    connection: &Connection,
    candidate: &EffectCaseTransitionCandidate,
) -> Result<bool, ControlStoreError> {
    if !expected_records_match(connection, candidate)? {
        return Ok(false);
    }
    for member in candidate.members() {
        if record_reference_exists(
            connection,
            member.record().coordinate().record_ref().as_str(),
        )? {
            return Ok(false);
        }
    }
    if let Some(record) = candidate.successor_checkpoint_record() {
        if record_reference_exists(connection, record.coordinate().record_ref().as_str())? {
            return Ok(false);
        }
    }
    for mutation in candidate.index_mutations() {
        let (role, key, expected, _) = mutation_parts(mutation);
        if !index_matches(connection, role, key, expected)? {
            return Ok(false);
        }
    }
    for assertion in candidate.index_assertions() {
        let (role, key, expected) = assertion_parts(assertion);
        if !index_matches(connection, role, key, expected)? {
            return Ok(false);
        }
    }
    checkpoint_predecessor_matches(connection, candidate.checkpoint_mutation())
}

/// Compare every immutable predecessor record supplied by the candidate.
fn expected_records_match(
    connection: &Connection,
    candidate: &EffectCaseTransitionCandidate,
) -> Result<bool, ControlStoreError> {
    if let Some(record) = candidate.expected_checkpoint_record() {
        if !record_matches(connection, record)? {
            return Ok(false);
        }
    }
    for mutation in candidate.index_mutations() {
        let (_, _, expected, _) = mutation_parts(mutation);
        if let EffectCaseIndexPredecessor::Exact(record) = expected {
            if !record_matches(connection, record)? {
                return Ok(false);
            }
        }
    }
    for assertion in candidate.index_assertions() {
        let (_, _, expected) = assertion_parts(assertion);
        if let EffectCaseIndexPredecessor::Exact(record) = expected {
            if !record_matches(connection, record)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Distinguish stale complete state from candidate-shaped partial custody.
fn classify_conflict(
    connection: &Connection,
    candidate: &EffectCaseTransitionCandidate,
) -> Result<ControlStoreErrorKind, ControlStoreError> {
    if !expected_records_match(connection, candidate)?
        || candidate_append_is_present(connection, candidate)?
        || !observed_frontier_is_complete(connection, candidate)?
    {
        Ok(ControlStoreErrorKind::RecoveryRequired)
    } else {
        Ok(ControlStoreErrorKind::CandidateConflict)
    }
}

/// Detect any candidate append when the complete successor was not observed.
fn candidate_append_is_present(
    connection: &Connection,
    candidate: &EffectCaseTransitionCandidate,
) -> Result<bool, ControlStoreError> {
    for member in candidate.members() {
        if record_reference_exists(
            connection,
            member.record().coordinate().record_ref().as_str(),
        )? {
            return Ok(true);
        }
    }
    if let Some(record) = candidate.successor_checkpoint_record() {
        if record_reference_exists(connection, record.coordinate().record_ref().as_str())? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Prove each relevant currently present head resolves to an immutable record.
fn observed_frontier_is_complete(
    connection: &Connection,
    candidate: &EffectCaseTransitionCandidate,
) -> Result<bool, ControlStoreError> {
    for mutation in candidate.index_mutations() {
        let (role, key, _, _) = mutation_parts(mutation);
        if !observed_index_is_complete(connection, role, key)? {
            return Ok(false);
        }
    }
    for assertion in candidate.index_assertions() {
        let (role, key, _) = assertion_parts(assertion);
        if !observed_index_is_complete(connection, role, key)? {
            return Ok(false);
        }
    }
    match candidate.checkpoint_mutation() {
        EffectCaseCheckpointMutation::Unchanged => Ok(true),
        EffectCaseCheckpointMutation::Advance(transition) => observed_checkpoint_is_complete(
            connection,
            transition.expected().checkpoint_domain_id().as_str(),
        ),
    }
}

/// Append immutable candidate records before any head can reference them.
fn append_candidate_records(
    connection: &Connection,
    candidate: &EffectCaseTransitionCandidate,
) -> Result<(), ControlStoreError> {
    for member in candidate.members() {
        insert_record(connection, member.record())?;
    }
    if let Some(record) = candidate.successor_checkpoint_record() {
        insert_record(connection, record)?;
    }
    Ok(())
}

/// Compare one typed index with its expected absent or exact record state.
fn index_matches(
    connection: &Connection,
    role: IndexRole,
    key: &EffectCaseIndexKey,
    expected: &EffectCaseIndexPredecessor,
) -> Result<bool, ControlStoreError> {
    let observed = read_index_coordinate(connection, role, key)?;
    match (observed, expected) {
        (None, EffectCaseIndexPredecessor::Absent) => Ok(true),
        (Some(observed), EffectCaseIndexPredecessor::Exact(expected))
            if &observed == expected.coordinate() =>
        {
            record_matches(connection, expected)
        }
        _ => Ok(false),
    }
}

/// Compare one typed index with its exact proposed successor.
fn index_points_to(
    connection: &Connection,
    role: IndexRole,
    key: &EffectCaseIndexKey,
    successor: &ControlRecordCoordinate,
) -> Result<bool, ControlStoreError> {
    Ok(read_index_coordinate(connection, role, key)?.as_ref() == Some(successor))
}

/// Read one typed index coordinate without scanning another role or key.
fn read_index_coordinate(
    connection: &Connection,
    role: IndexRole,
    key: &EffectCaseIndexKey,
) -> Result<Option<ControlRecordCoordinate>, ControlStoreError> {
    let row = connection
        .query_row(
            "SELECT record_ref, record_sha256 FROM pale_deployment_effect_case_index \
             WHERE index_role = ?1 AND index_key = ?2",
            params![role.as_str(), key.as_str()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|source| sqlite("read effect-case index", source))?;
    row.map(|(record_ref, record_sha256)| {
        let record_ref = ResidentRecordRefV1::parse(record_ref)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        let record_sha256 = ControlRecordDigest::parse(record_sha256)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        Ok(ControlRecordCoordinate::new(record_ref, record_sha256))
    })
    .transpose()
}

/// Check that a present index head resolves to its exact immutable record.
fn observed_index_is_complete(
    connection: &Connection,
    role: IndexRole,
    key: &EffectCaseIndexKey,
) -> Result<bool, ControlStoreError> {
    let Some(coordinate) = read_index_coordinate(connection, role, key)? else {
        return Ok(true);
    };
    Ok(read_record_value(connection, &coordinate)?.is_some())
}

/// Apply one exact insert or compare-and-swap index mutation.
fn apply_index_mutation(
    connection: &Connection,
    mutation: &EffectCaseIndexMutation,
) -> Result<(), ControlStoreError> {
    let (role, key, expected, successor) = mutation_parts(mutation);
    let changed = match expected {
        EffectCaseIndexPredecessor::Absent => connection.execute(
            "INSERT INTO pale_deployment_effect_case_index \
             (index_role, index_key, record_ref, record_sha256) VALUES (?1, ?2, ?3, ?4)",
            params![
                role.as_str(),
                key.as_str(),
                successor.record_ref().as_str(),
                successor.record_sha256().as_str(),
            ],
        ),
        EffectCaseIndexPredecessor::Exact(expected) => connection.execute(
            "UPDATE pale_deployment_effect_case_index \
             SET record_ref = ?1, record_sha256 = ?2 \
             WHERE index_role = ?3 AND index_key = ?4 AND record_ref = ?5 AND record_sha256 = ?6",
            params![
                successor.record_ref().as_str(),
                successor.record_sha256().as_str(),
                role.as_str(),
                key.as_str(),
                expected.coordinate().record_ref().as_str(),
                expected.coordinate().record_sha256().as_str(),
            ],
        ),
    }
    .map_err(|source| sqlite("advance effect-case index", source))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))
    }
}

/// Compare the selected checkpoint predecessor, or preserve an unchanged domain.
fn checkpoint_predecessor_matches(
    connection: &Connection,
    mutation: &EffectCaseCheckpointMutation,
) -> Result<bool, ControlStoreError> {
    match mutation {
        EffectCaseCheckpointMutation::Unchanged => Ok(true),
        EffectCaseCheckpointMutation::Advance(transition) => {
            checkpoint_matches(connection, transition.expected())
        }
    }
}

/// Compare the selected checkpoint successor, or preserve an unchanged domain.
fn checkpoint_successor_matches(
    connection: &Connection,
    mutation: &EffectCaseCheckpointMutation,
) -> Result<bool, ControlStoreError> {
    match mutation {
        EffectCaseCheckpointMutation::Unchanged => Ok(true),
        EffectCaseCheckpointMutation::Advance(transition) => {
            checkpoint_matches(connection, transition.successor())
        }
    }
}

/// Compare one exact hard-domain checkpoint head.
fn checkpoint_matches(
    connection: &Connection,
    checkpoint: &ControlCheckpointHead,
) -> Result<bool, ControlStoreError> {
    if crate::store::transaction::record_stamp(
        connection,
        crate::ControlStoreKind::DeploymentControl,
        checkpoint.coordinate(),
    )? != Some(checkpoint.committed_transaction_sequence())
    {
        return Ok(false);
    }
    let epoch = sqlite_integer(checkpoint.domain_epoch().get())?;
    let sequence = sqlite_integer(checkpoint.domain_sequence().get())?;
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_effect_case_checkpoint_head \
             WHERE checkpoint_domain_id = ?1 AND domain_epoch = ?2 AND domain_sequence = ?3 \
               AND control_checkpoint_ref = ?4 AND control_checkpoint_sha256 = ?5)",
            params![
                checkpoint.checkpoint_domain_id().as_str(),
                epoch,
                sequence,
                checkpoint.coordinate().record_ref().as_str(),
                checkpoint.coordinate().record_sha256().as_str(),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare effect-case checkpoint head", source))
}

/// Check that a current domain head resolves to an immutable checkpoint record.
fn observed_checkpoint_is_complete(
    connection: &Connection,
    domain_id: &str,
) -> Result<bool, ControlStoreError> {
    let row = connection
        .query_row(
            "SELECT control_checkpoint_ref, control_checkpoint_sha256 \
             FROM pale_deployment_effect_case_checkpoint_head WHERE checkpoint_domain_id = ?1",
            [domain_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|source| sqlite("read effect-case checkpoint head", source))?;
    let Some((record_ref, record_sha256)) = row else {
        return Ok(true);
    };
    let coordinate = ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(record_ref)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?,
        ControlRecordDigest::parse(record_sha256)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?,
    );
    Ok(read_record_value(connection, &coordinate)?.is_some())
}

/// Compare-and-swap one exact hard-domain checkpoint head.
fn advance_checkpoint(
    connection: &Connection,
    mutation: &EffectCaseCheckpointMutation,
) -> Result<(), ControlStoreError> {
    let EffectCaseCheckpointMutation::Advance(transition) = mutation else {
        return Ok(());
    };
    let expected = transition.expected();
    let successor = transition.successor();
    let changed = connection
        .execute(
            "UPDATE pale_deployment_effect_case_checkpoint_head \
             SET domain_epoch = ?1, domain_sequence = ?2, control_checkpoint_ref = ?3, \
                 control_checkpoint_sha256 = ?4 \
             WHERE checkpoint_domain_id = ?5 AND domain_epoch = ?6 AND domain_sequence = ?7 \
               AND control_checkpoint_ref = ?8 AND control_checkpoint_sha256 = ?9",
            params![
                sqlite_integer(successor.domain_epoch().get())?,
                sqlite_integer(successor.domain_sequence().get())?,
                successor.coordinate().record_ref().as_str(),
                successor.coordinate().record_sha256().as_str(),
                expected.checkpoint_domain_id().as_str(),
                sqlite_integer(expected.domain_epoch().get())?,
                sqlite_integer(expected.domain_sequence().get())?,
                expected.coordinate().record_ref().as_str(),
                expected.coordinate().record_sha256().as_str(),
            ],
        )
        .map_err(|source| sqlite("advance effect-case checkpoint head", source))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))
    }
}

/// Closed physical index vocabulary stored in the shared table.
#[derive(Clone, Copy)]
enum IndexRole {
    RequestIdentity,
    LogicalAction,
    ActionDisposition,
    ActiveClaim,
    ActiveCustody,
    ActionIntent,
    CaseResult,
}

impl IndexRole {
    /// Return the migration-constrained physical spelling.
    const fn as_str(self) -> &'static str {
        match self {
            Self::RequestIdentity => "request_identity",
            Self::LogicalAction => "logical_action",
            Self::ActionDisposition => "action_disposition",
            Self::ActiveClaim => "active_claim",
            Self::ActiveCustody => "active_custody",
            Self::ActionIntent => "action_intent",
            Self::CaseResult => "case_result",
        }
    }
}

/// Project a closed mutation without erasing its index role.
fn mutation_parts(
    mutation: &EffectCaseIndexMutation,
) -> (
    IndexRole,
    &EffectCaseIndexKey,
    &EffectCaseIndexPredecessor,
    &ControlRecordCoordinate,
) {
    match mutation {
        EffectCaseIndexMutation::RequestIdentity {
            key,
            expected,
            successor,
        } => (IndexRole::RequestIdentity, key, expected, successor),
        EffectCaseIndexMutation::LogicalAction {
            key,
            expected,
            successor,
        } => (IndexRole::LogicalAction, key, expected, successor),
        EffectCaseIndexMutation::ActionDisposition {
            key,
            expected,
            successor,
        } => (IndexRole::ActionDisposition, key, expected, successor),
        EffectCaseIndexMutation::ActiveClaim {
            key,
            expected,
            successor,
        } => (IndexRole::ActiveClaim, key, expected, successor),
        EffectCaseIndexMutation::ActiveCustody {
            key,
            expected,
            successor,
        } => (IndexRole::ActiveCustody, key, expected, successor),
        EffectCaseIndexMutation::ActionIntent {
            key,
            expected,
            successor,
        } => (IndexRole::ActionIntent, key, expected, successor),
        EffectCaseIndexMutation::CaseResult {
            key,
            expected,
            successor,
        } => (IndexRole::CaseResult, key, expected, successor),
    }
}

/// Project a closed assertion without erasing its index role.
fn assertion_parts(
    assertion: &EffectCaseIndexAssertion,
) -> (IndexRole, &EffectCaseIndexKey, &EffectCaseIndexPredecessor) {
    match assertion {
        EffectCaseIndexAssertion::LogicalAction { key, expected } => {
            (IndexRole::LogicalAction, key, expected)
        }
        EffectCaseIndexAssertion::ActionDisposition { key, expected } => {
            (IndexRole::ActionDisposition, key, expected)
        }
        EffectCaseIndexAssertion::ActiveClaim { key, expected } => {
            (IndexRole::ActiveClaim, key, expected)
        }
        EffectCaseIndexAssertion::ActiveCustody { key, expected } => {
            (IndexRole::ActiveCustody, key, expected)
        }
        EffectCaseIndexAssertion::ActionIntent { key, expected } => {
            (IndexRole::ActionIntent, key, expected)
        }
        EffectCaseIndexAssertion::CaseResult { key, expected } => {
            (IndexRole::CaseResult, key, expected)
        }
    }
}

/// Project a point query into its migration-constrained role.
const fn query_role(query: &EffectCaseIndexQuery) -> IndexRole {
    match query {
        EffectCaseIndexQuery::RequestIdentity(_) => IndexRole::RequestIdentity,
        EffectCaseIndexQuery::LogicalAction(_) => IndexRole::LogicalAction,
        EffectCaseIndexQuery::ActionDisposition(_) => IndexRole::ActionDisposition,
        EffectCaseIndexQuery::ActiveClaim(_) => IndexRole::ActiveClaim,
        EffectCaseIndexQuery::ActiveCustody(_) => IndexRole::ActiveCustody,
        EffectCaseIndexQuery::ActionIntent(_) => IndexRole::ActionIntent,
        EffectCaseIndexQuery::CaseResult(_) => IndexRole::CaseResult,
    }
}

#[cfg(test)]
#[path = "../tests/internal/effect_case.rs"]
mod tests;
