//! Deployment-bound `SQLite` custody for durable Pale operations.
//!
//! One immediate transaction compares an exact store incarnation, private
//! operation indexes, and a hard-domain checkpoint; appends already-admitted
//! opaque records; and advances every selected head atomically. Application
//! policy retains operation admission, provider dispatch, consultation,
//! cancellation, result, and settlement meaning.

use pale_core::records::ResidentRecordRefV1;
use pale_ports::{
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecordCoordinate, ControlRecordDigest,
    ControlStoreError, ControlStoreErrorKind, OperationCustodyCandidate,
    OperationCustodyCommitOutcome, OperationCustodyIndexKey, OperationCustodyIndexMutation,
    OperationCustodyIndexObservation, OperationCustodyIndexPredecessor, OperationCustodyIndexQuery,
    OperationCustodyPort, OperationCustodyReadBackOutcome,
};
use rusqlite::{params, types::ValueRef, Connection, OptionalExtension, TransactionBehavior};

use crate::deployment::{
    a2a_evidence_reference_exists, insert_record, read_record_bounded, record_is_complete,
    record_matches, record_reference_exists, require_coordinate, require_store_binding,
};
use crate::error::{sqlite, sqlite_integer};
use crate::store::journal;
use crate::ControlSqliteError;

mod a2a;
mod a2a_discovery;
mod checkpoint_frontier;
mod collection_journal;
mod collection_snapshot;
mod committed_accumulator;
use crate::accumulator::{self as checkpoint_accumulator, Store as AccumulatorStore};
mod publication;
mod session_basis;
mod start_discovery;

/// `SQLite` realization of atomic operation custody.
///
/// Construction remains crate-private until the deployment-storage profile and
/// resident assembly are admitted. A committed or replayed transition proves
/// only exact physical custody; it grants no dispatch, cancellation, native
/// effect, acceptance, or settlement authority, and it cannot replace any
/// independently required anti-rollback anchor advancement.
pub struct SqliteOperationCustody<'connection> {
    /// Open deployment-control connection whose Pale Runtime assembly remains external.
    pub(crate) connection: &'connection mut Connection,
    /// Exact store incarnation represented by [`Self::connection`].
    pub(crate) coordinate: AuthorityStoreCoordinate,
}

impl pale_ports::ControlTransactionPort for SqliteOperationCustody<'_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, ControlStoreError> {
        require_coordinate(&self.coordinate, store)?;
        crate::store::transaction::prepare_snapshot(
            self.connection,
            store,
            crate::ControlStoreKind::DeploymentControl,
        )
    }
}

impl OperationCustodyPort for SqliteOperationCustody<'_> {
    fn recover_committed_accumulator(
        &mut self,
        request: &pale_ports::OperationAccumulatorPreparation,
        successor: &pale_ports::OperationAccumulatorCheckpoint,
    ) -> Result<
        pale_ports::PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        self.recover_accumulator(request, successor)
    }

    fn read_collection_snapshot(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        maximum_frontier_bytes: usize,
        maximum_journal_payload_bytes: usize,
    ) -> Result<Option<pale_ports::OperationCollectionSnapshot>, ControlStoreError> {
        collection_snapshot::read(
            self,
            store,
            handle,
            None,
            maximum_frontier_bytes,
            maximum_journal_payload_bytes,
        )
    }

    fn read_collection_snapshot_at(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        frontier: &ControlRecordCoordinate,
        maximum_frontier_bytes: usize,
        maximum_journal_payload_bytes: usize,
    ) -> Result<Option<pale_ports::OperationCollectionSnapshot>, ControlStoreError> {
        collection_snapshot::read(
            self,
            store,
            handle,
            Some(frontier),
            maximum_frontier_bytes,
            maximum_journal_payload_bytes,
        )
    }

    fn read_a2a_task_bindings(
        &mut self,
        store: &AuthorityStoreCoordinate,
        scope: &pale_ports::OperationA2aBindingScope,
        cursor: Option<&pale_ports::OperationA2aBindingCursor>,
        maximum_bindings: usize,
        maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationA2aBindings, ControlStoreError> {
        a2a_discovery::read(
            self,
            store,
            scope,
            cursor,
            maximum_bindings,
            maximum_body_bytes,
        )
    }
    fn read_operation_session_basis(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        maximum_bytes: usize,
    ) -> Result<Option<pale_ports::OperationSessionBasis>, ControlStoreError> {
        session_basis::read(self, store, handle, maximum_bytes)
    }

    fn read_operation_starts(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&pale_ports::OperationStartCursor>,
        maximum_sources: usize,
        maximum_session_basis_bytes: usize,
    ) -> Result<pale_ports::OperationStartSources, pale_ports::OperationStartReadError> {
        start_discovery::read(
            self,
            store,
            cursor,
            maximum_sources,
            maximum_session_basis_bytes,
        )
    }

    fn read_a2a_evidence(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_bytes: usize,
    ) -> Result<Option<pale_ports::OperationA2aEvidence>, ControlStoreError> {
        a2a::read(self, store, coordinate, maximum_bytes)
    }

    fn read_a2a_evidence_inserted_at(
        &mut self,
        store: &AuthorityStoreCoordinate,
        transaction: pale_core::records::ControlTransactionSequenceV1,
        maximum_items: usize,
        maximum_bytes: usize,
    ) -> Result<Vec<pale_ports::OperationA2aEvidence>, ControlStoreError> {
        a2a::read_inserted_at(self, store, transaction, maximum_items, maximum_bytes)
    }

    fn read_publication_sources(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&pale_ports::OperationPublicationCursor>,
        maximum_sources: usize,
        maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationPublicationSources, pale_ports::OperationPublicationReadError>
    {
        publication::read(self, store, cursor, maximum_sources, maximum_body_bytes)
    }

    fn read_checkpoint_head(
        &mut self,
        store: &AuthorityStoreCoordinate,
        domain: &pale_core::records::ControlCheckpointDomainIdV1,
        maximum_record_bytes: usize,
    ) -> Result<Option<(ControlCheckpointHead, pale_ports::ControlRecord)>, ControlStoreError> {
        require_coordinate(&self.coordinate, store)?;
        if maximum_record_bytes == 0 {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin current operation checkpoint read", source))?;
        require_store_binding(&transaction, store)?;
        let current_transaction = crate::store::transaction::read(&transaction, store)?;
        let row = transaction.query_row(
            "SELECT domain_epoch, domain_sequence, control_checkpoint_ref, control_checkpoint_sha256 \
             FROM pale_deployment_operation_checkpoint_head WHERE checkpoint_domain_id=?1",
            [domain.as_str()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, decode_record_coordinate(row.get_ref(2)?, row.get_ref(3)?))),
        ).optional().map_err(|source| sqlite("read current operation checkpoint", source))?;
        let result = if let Some((epoch, sequence, coordinate)) = row {
            let epoch = u64::try_from(epoch)
                .ok()
                .and_then(|value| pale_core::records::ControlDomainEpochV1::new(value).ok())
                .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
            let sequence = u64::try_from(sequence)
                .ok()
                .and_then(|value| pale_core::records::ControlDomainSequenceV1::new(value).ok())
                .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
            let coordinate = coordinate?;
            let complete: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM pale_deployment_operation_accumulator_frontier \
                 WHERE checkpoint_domain_id=?1 AND domain_epoch=?2 AND control_checkpoint_ref=?3 AND control_checkpoint_sha256=?4)",
                params![domain.as_str(), sqlite_integer(epoch.get())?, coordinate.record_ref().as_str(), coordinate.record_sha256().as_str()],
                |row| row.get(0),
            ).map_err(|source| sqlite("read current operation accumulator closure", source))?;
            if !complete {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            let record = read_record_bounded(&transaction, &coordinate, maximum_record_bytes)?
                .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
            let inserted = crate::store::transaction::record_stamp(
                &transaction,
                crate::ControlStoreKind::DeploymentControl,
                &coordinate,
            )?
            .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
            if inserted.get() > current_transaction.get() {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            Some((
                ControlCheckpointHead::new(domain.clone(), epoch, sequence, inserted, coordinate),
                record,
            ))
        } else {
            if !checkpoint_accumulator::domain_is_absent(
                &transaction,
                AccumulatorStore::DeploymentOperation,
                domain,
            )? {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            None
        };
        transaction
            .rollback()
            .map_err(|source| sqlite("release current checkpoint snapshot", source))?;
        Ok(result)
    }

    fn verify_accumulator_membership(
        &mut self,
        request: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        match request.implementation() {
            pale_ports::AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1 => {}
        }
        let selected = request.checkpoint();
        require_coordinate(&self.coordinate, selected.store())?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin accumulator membership snapshot", source))?;
        require_store_binding(&transaction, selected.store())?;
        if !record_is_complete(&transaction, selected.checkpoint().coordinate())? {
            return Err(ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired).into());
        }
        if !checkpoint_matches(&transaction, selected.checkpoint())? {
            let kind = if checkpoint_frontier::current_frontier_is_complete(
                &transaction,
                selected.checkpoint(),
                selected.profile_ref(),
                selected.profile_sha256(),
            )? {
                ControlStoreErrorKind::CandidateConflict
            } else {
                ControlStoreErrorKind::RecoveryRequired
            };
            return Err(ControlStoreError::new(kind).into());
        }
        let result = checkpoint_accumulator::preparation::verify_membership(
            &transaction,
            AccumulatorStore::DeploymentOperation,
            request,
        )?;
        transaction
            .rollback()
            .map_err(|source| sqlite("release accumulator membership snapshot", source))?;
        Ok(result)
    }

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
        require_coordinate(&self.coordinate, request.basis().store())?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin accumulator preparation snapshot", source))?;
        require_store_binding(&transaction, request.basis().store())?;
        let current_transaction =
            crate::store::transaction::read(&transaction, request.basis().store())?;
        if let pale_ports::AccumulatorPreparationBasis::Successor(prior) = request.basis() {
            if !record_is_complete(&transaction, prior.checkpoint().coordinate())? {
                return Err(ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired).into());
            }
            if !checkpoint_matches(&transaction, prior.checkpoint())? {
                let kind = if checkpoint_frontier::current_frontier_is_complete(
                    &transaction,
                    prior.checkpoint(),
                    prior.profile_ref(),
                    prior.profile_sha256(),
                )? {
                    ControlStoreErrorKind::CandidateConflict
                } else {
                    ControlStoreErrorKind::RecoveryRequired
                };
                return Err(ControlStoreError::new(kind).into());
            }
        }
        if current_transaction != request.transaction().expected() {
            return Err(ControlStoreError::new(ControlStoreErrorKind::CandidateConflict).into());
        }
        let prepared = checkpoint_accumulator::preparation::prepare(
            &transaction,
            AccumulatorStore::DeploymentOperation,
            request,
        )?;
        transaction
            .rollback()
            .map_err(|source| sqlite("release accumulator preparation snapshot", source))?;
        Ok(prepared)
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        store: &AuthorityStoreCoordinate,
        record: &pale_ports::ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        require_coordinate(&self.coordinate, store)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|source| sqlite("begin operation-lease retention", source))?;
        require_store_binding(&transaction, store)?;
        if record_matches(&transaction, record)? {
            let current = crate::store::transaction::read(&transaction, store)?;
            let inserted = crate::store::transaction::record_stamp(
                &transaction,
                crate::ControlStoreKind::DeploymentControl,
                record.coordinate(),
            )?;
            if inserted.is_none_or(|inserted| inserted.get() > current.get()) {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            transaction
                .rollback()
                .map_err(|source| sqlite("close exact operation-lease replay", source))?;
            return Ok(OperationCustodyCommitOutcome::ExactReplay);
        }
        if record_reference_exists(&transaction, record.coordinate().record_ref().as_str())? {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        let prepared = crate::store::transaction::prepare(&transaction, store)?;
        crate::store::transaction::advance(&transaction, &prepared)?;
        insert_record(&transaction, record)?;
        if !record_matches(&transaction, record)? {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        transaction.commit().map_err(|source| {
            ControlStoreError::with_source(
                ControlStoreErrorKind::OutcomeUnknown,
                ControlSqliteError::sqlite("commit operation-lease retention", source),
            )
        })?;
        Ok(OperationCustodyCommitOutcome::Committed)
    }

    fn read_operation_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<pale_ports::ControlRecord>, ControlStoreError> {
        require_coordinate(&self.coordinate, store)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin bounded operation record read", source))?;
        require_store_binding(&transaction, store)?;
        let record = read_record_bounded(&transaction, coordinate, maximum_body_bytes)?;
        transaction
            .commit()
            .map_err(|source| sqlite("close bounded operation record read", source))?;
        Ok(record)
    }

    fn compare_and_commit(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        checkpoint_accumulator::validate(candidate.accumulator())?;
        if let Some(pair) = candidate.collection_journal() {
            journal::require_journal_bound(pair.append(), pair.maximum_journal_payload_bytes())?;
        }
        require_coordinate(&self.coordinate, candidate.store())?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|source| sqlite("begin operation custody transition", source))?;
        require_store_binding(&transaction, candidate.store())?;

        match observe(&transaction, candidate)? {
            OperationCustodyReadBackOutcome::ExactSuccessor => {
                transaction
                    .rollback()
                    .map_err(|source| sqlite("close exact operation custody replay", source))?;
                Ok(OperationCustodyCommitOutcome::ExactReplay)
            }
            OperationCustodyReadBackOutcome::Predecessor => {
                crate::store::transaction::advance(
                    &transaction,
                    candidate.accumulator().binding().transaction(),
                )?;
                append_candidate_records(&transaction, candidate)?;
                session_basis::append(&transaction, candidate)?;
                a2a::append(&transaction, candidate)?;
                a2a_discovery::append(&transaction, candidate)?;
                if let Some(pair) = candidate.collection_journal() {
                    collection_journal::append_collection_entry(
                        &transaction,
                        pair.operation_handle(),
                        pair.append(),
                    )?;
                }
                checkpoint_accumulator::apply(
                    &transaction,
                    AccumulatorStore::DeploymentOperation,
                    candidate.accumulator(),
                )?;
                for mutation in candidate.indexes() {
                    apply_index_mutation(&transaction, mutation)?;
                }
                advance_checkpoint(
                    &transaction,
                    candidate.checkpoint().phase(),
                    candidate.checkpoint().successor(),
                )?;
                if observe(&transaction, candidate)?
                    != OperationCustodyReadBackOutcome::ExactSuccessor
                {
                    return Err(ControlStoreError::new(
                        ControlStoreErrorKind::RecoveryRequired,
                    ));
                }
                transaction.commit().map_err(|source| {
                    ControlStoreError::with_source(
                        ControlStoreErrorKind::OutcomeUnknown,
                        ControlSqliteError::sqlite("commit operation custody transition", source),
                    )
                })?;
                Ok(OperationCustodyCommitOutcome::Committed)
            }
            OperationCustodyReadBackOutcome::ConflictingOrPartial => Err(ControlStoreError::new(
                classify_conflict(&transaction, candidate)?,
            )),
        }
    }

    fn observe_exact(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        checkpoint_accumulator::validate(candidate.accumulator())?;
        if let Some(pair) = candidate.collection_journal() {
            journal::require_journal_bound(pair.append(), pair.maximum_journal_payload_bytes())?;
        }
        require_coordinate(&self.coordinate, candidate.store())?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin operation custody read-back", source))?;
        require_store_binding(&transaction, candidate.store())?;
        let observed = match observe(&transaction, candidate) {
            Err(error)
                if candidate.collection_journal().is_some()
                    && error.kind() == ControlStoreErrorKind::RecoveryRequired =>
            {
                OperationCustodyReadBackOutcome::ConflictingOrPartial
            }
            result => result?,
        };
        transaction
            .commit()
            .map_err(|source| sqlite("close operation custody read-back", source))?;
        Ok(observed)
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        maximum_record_bytes: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        require_coordinate(&self.coordinate, store)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin bounded operation-index read", source))?;
        require_store_binding(&transaction, store)?;
        let (role, key) = query_parts(query);
        let observed = match read_index_coordinate(&transaction, role, key)? {
            None => OperationCustodyIndexObservation::absent(query.clone()),
            Some(coordinate) => {
                let record = read_record_bounded(&transaction, &coordinate, maximum_record_bytes)?
                    .ok_or_else(|| {
                        ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
                    })?;
                OperationCustodyIndexObservation::exact(query.clone(), record)
                    .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?
            }
        };
        transaction
            .commit()
            .map_err(|source| sqlite("close bounded operation-index read", source))?;
        Ok(observed)
    }
}

fn collection_read_error(error: pale_ports::OperationCollectionJournalError) -> ControlStoreError {
    let kind = if matches!(
        error,
        pale_ports::OperationCollectionJournalError::BoundExceeded
    ) {
        ControlStoreErrorKind::BoundExceeded
    } else {
        ControlStoreErrorKind::RecoveryRequired
    };
    ControlStoreError::with_source(kind, error)
}

/// Classify the complete predecessor, complete successor, or every other state.
fn observe(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
    if let Some(pair) = candidate.collection_journal() {
        collection_journal::require_journal_head_consistency(
            connection,
            pair.operation_handle(),
            pair.maximum_journal_payload_bytes(),
        )?;
    }
    let journal_successor = match candidate.collection_journal() {
        Some(pair) => collection_journal::journal_successor_matches(
            connection,
            pair.operation_handle(),
            pair.append(),
            pair.maximum_journal_payload_bytes(),
        )?,
        None => true,
    };
    if journal_successor && successor_matches(connection, candidate)? {
        return Ok(OperationCustodyReadBackOutcome::ExactSuccessor);
    }
    let journal_predecessor = match candidate.collection_journal() {
        Some(pair) => collection_journal::journal_predecessor_matches(
            connection,
            pair.operation_handle(),
            pair.append(),
        )?,
        None => true,
    };
    if journal_predecessor && predecessor_matches(connection, candidate)? {
        return Ok(OperationCustodyReadBackOutcome::Predecessor);
    }
    Ok(OperationCustodyReadBackOutcome::ConflictingOrPartial)
}

/// Require every immutable append, private-index successor, and checkpoint.
fn successor_matches(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    if !a2a::retained_match(connection, candidate)?
        || !a2a_discovery::retained_match(connection, candidate)?
    {
        return Ok(false);
    }
    if !checkpoint_accumulator::matches(
        connection,
        AccumulatorStore::DeploymentOperation,
        candidate.accumulator(),
        true,
    )? {
        return Ok(false);
    }
    if let Some(predecessor) = candidate.checkpoint().predecessor_record() {
        if !record_matches(connection, predecessor)? {
            return Ok(false);
        }
    }
    if !record_matches(connection, candidate.checkpoint().successor_record())? {
        return Ok(false);
    }
    if !candidate_stamps_match(connection, candidate)? {
        return Ok(false);
    }
    for record in candidate.records() {
        if !record_matches(connection, record.record())? {
            return Ok(false);
        }
    }
    for mutation in candidate.indexes() {
        let (role, key, _, successor) = mutation_parts(mutation);
        if !index_points_to(connection, role, key, successor)? {
            return Ok(false);
        }
    }
    checkpoint_matches(connection, candidate.checkpoint().successor())
}

/// Require the exact index/checkpoint predecessor and absence of every append.
fn predecessor_matches(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    if !a2a::retained_match(connection, candidate)?
        || !a2a_discovery::retained_match(connection, candidate)?
        || a2a_discovery::material_present(connection, candidate)?
        || !a2a::new_evidence_absent(connection, candidate)?
        || session_basis::material_present(connection, candidate)?
    {
        return Ok(false);
    }
    if !checkpoint_accumulator::matches(
        connection,
        AccumulatorStore::DeploymentOperation,
        candidate.accumulator(),
        false,
    )? {
        return Ok(false);
    }
    if let Some(predecessor) = candidate.checkpoint().predecessor_record() {
        if !record_matches(connection, predecessor)? {
            return Ok(false);
        }
    }
    for record in candidate.records() {
        if record_reference_exists(
            connection,
            record.record().coordinate().record_ref().as_str(),
        )? || a2a_evidence_reference_exists(
            connection,
            record.record().coordinate().record_ref().as_str(),
        )? {
            return Ok(false);
        }
    }
    if record_reference_exists(
        connection,
        candidate
            .checkpoint()
            .successor_record()
            .coordinate()
            .record_ref()
            .as_str(),
    )? || a2a_evidence_reference_exists(
        connection,
        candidate
            .checkpoint()
            .successor()
            .coordinate()
            .record_ref()
            .as_str(),
    )? {
        return Ok(false);
    }
    for mutation in candidate.indexes() {
        let (role, key, expected, _) = mutation_parts(mutation);
        if !index_matches(connection, role, key, expected)? {
            return Ok(false);
        }
    }
    match candidate.checkpoint().phase() {
        pale_ports::OperationCheckpointPhase::Initial => checkpoint_accumulator::domain_is_absent(
            connection,
            AccumulatorStore::DeploymentOperation,
            candidate.checkpoint().successor().checkpoint_domain_id(),
        ),
        pale_ports::OperationCheckpointPhase::Successor { predecessor, .. } => {
            checkpoint_matches(connection, predecessor)
        }
    }
}

/// Distinguish stale complete state from partial or contradictory custody.
fn classify_conflict(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<ControlStoreErrorKind, ControlStoreError> {
    if let Some(pair) = candidate.collection_journal() {
        if !collection_journal::journal_frontier_is_complete(connection, pair.operation_handle())? {
            return Ok(ControlStoreErrorKind::RecoveryRequired);
        }
        let head = collection_journal::read_journal_head(connection, pair.operation_handle())?;
        let exact_entry = collection_journal::journal_entry_matches(
            connection,
            pair.operation_handle(),
            pair.append(),
            pair.maximum_journal_payload_bytes(),
        )?;
        // Later unrelated journal entries do not erase this exact historical
        // pair, but they also cannot be rewound by replaying it.
        if head.is_some_and(|head| head > pair.append().entry().sequence())
            && exact_entry
            && successor_matches(connection, candidate)?
        {
            return Ok(ControlStoreErrorKind::CandidateConflict);
        }
        // An unrelated complete journal winner can stale this pair while the
        // operation remains at its exact predecessor. No candidate rows exist.
        if predecessor_matches(connection, candidate)? {
            return Ok(
                if !exact_entry
                    && head.is_some_and(|head| Some(head) > pair.append().expected_head())
                {
                    ControlStoreErrorKind::CandidateConflict
                } else {
                    ControlStoreErrorKind::RecoveryRequired
                },
            );
        }
    }
    let binding = candidate.accumulator().binding();
    let complete_frontier = match binding.phase() {
        pale_ports::AccumulatorCheckpointPhase::Initial => {
            checkpoint_frontier::initial_frontier_is_complete(
                connection,
                binding.successor(),
                binding.profile_ref(),
                binding.profile_sha256(),
            )?
        }
        pale_ports::AccumulatorCheckpointPhase::Successor(predecessor) => {
            if checkpoint_matches(connection, predecessor)? {
                return Ok(ControlStoreErrorKind::RecoveryRequired);
            }
            checkpoint_frontier::current_frontier_is_complete(
                connection,
                predecessor,
                binding.profile_ref(),
                binding.profile_sha256(),
            )?
        }
    };
    if !expected_records_match(connection, candidate)?
        || !complete_frontier
        || !selected_indexes_are_complete(connection, candidate)?
    {
        return Ok(ControlStoreErrorKind::RecoveryRequired);
    }
    // Successors require a strictly advanced complete frontier. Initial writers
    // can conflict at sequence one, but never across epoch/profile identities.
    let material_is_present = candidate_material_is_present(connection, candidate)?;
    if checkpoint_matches(connection, candidate.checkpoint().successor())?
        || (material_is_present && !historical_candidate_is_complete(connection, candidate)?)
    {
        return Ok(ControlStoreErrorKind::RecoveryRequired);
    }
    Ok(ControlStoreErrorKind::CandidateConflict)
}

/// Compare immutable records used as exact candidate predecessors.
fn expected_records_match(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    if !a2a::retained_match(connection, candidate)?
        || !a2a_discovery::retained_match(connection, candidate)?
        || !a2a_discovery::transaction_is_complete(connection, candidate)?
    {
        return Ok(false);
    }
    if let Some(predecessor) = candidate.checkpoint().predecessor_record() {
        if !record_matches(connection, predecessor)? {
            return Ok(false);
        }
    }
    for mutation in candidate.indexes() {
        let (_, _, predecessor, _) = mutation_parts(mutation);
        if let OperationCustodyIndexPredecessor::Exact(expected) = predecessor {
            if !record_matches(connection, expected)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Detect any retained atom member, including the exact attached journal bytes.
fn candidate_material_is_present(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    if a2a::new_material_present(connection, candidate)?
        || a2a_discovery::material_present(connection, candidate)?
        || session_basis::material_present(connection, candidate)?
    {
        return Ok(true);
    }
    for record in candidate.records() {
        if record_reference_exists(
            connection,
            record.record().coordinate().record_ref().as_str(),
        )? || a2a_evidence_reference_exists(
            connection,
            record.record().coordinate().record_ref().as_str(),
        )? {
            return Ok(true);
        }
    }
    if let Some(pair) = candidate.collection_journal() {
        if collection_journal::journal_entry_matches(
            connection,
            pair.operation_handle(),
            pair.append(),
            pair.maximum_journal_payload_bytes(),
        )? {
            return Ok(true);
        }
    }
    let checkpoint_ref = candidate
        .checkpoint()
        .successor_record()
        .coordinate()
        .record_ref()
        .as_str();
    Ok(record_reference_exists(connection, checkpoint_ref)?
        || a2a_evidence_reference_exists(connection, checkpoint_ref)?)
}

/// Distinguish complete stale history from orphaned candidate material.
///
/// Immutable start indexes must continue to name their admitted records. The
/// Local and A2A latest-observation indexes may advance beyond this historical candidate,
/// but its current coordinate must still close over an immutable record.
fn historical_candidate_is_complete(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    if !a2a_discovery::historical_match(connection, candidate)? {
        return Ok(false);
    }
    if let Some(pair) = candidate.collection_journal() {
        if !collection_journal::journal_entry_matches(
            connection,
            pair.operation_handle(),
            pair.append(),
            pair.maximum_journal_payload_bytes(),
        )? {
            return Ok(false);
        }
    }
    if !candidate_stamps_match(connection, candidate)? {
        return Ok(false);
    }
    for record in candidate.records() {
        if !record_matches(connection, record.record())? {
            return Ok(false);
        }
    }
    if !record_matches(connection, candidate.checkpoint().successor_record())? {
        return Ok(false);
    }
    for mutation in candidate.indexes() {
        let (role, key, _, successor) = mutation_parts(mutation);
        let complete = if matches!(
            role,
            IndexRole::LatestObservation
                | IndexRole::LatestA2aTaskObservation
                | IndexRole::CollectionHead
        ) {
            index_is_complete(connection, role, key)?
                && read_index_coordinate(connection, role, key)?.is_some()
        } else {
            index_points_to(connection, role, key, successor)?
        };
        if !complete {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Require every selected present index to close over its immutable record.
fn selected_indexes_are_complete(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    for mutation in candidate.indexes() {
        let (role, key, expected, _) = mutation_parts(mutation);
        let present = read_index_coordinate(connection, role, key)?;
        let complete = match (present, expected) {
            (None, OperationCustodyIndexPredecessor::Absent) => true,
            (None, OperationCustodyIndexPredecessor::Exact(_)) => false,
            (Some(coordinate), _) => record_is_complete(connection, &coordinate)?,
        };
        if !complete {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Append candidate and checkpoint records before any head references them.
fn append_candidate_records(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<(), ControlStoreError> {
    for record in candidate.records() {
        insert_record(connection, record.record())?;
    }
    insert_record(connection, candidate.checkpoint().successor_record())
}

/// Bind only actual new atom members, not previously retained dependencies.
fn candidate_stamps_match(
    connection: &Connection,
    candidate: &OperationCustodyCandidate,
) -> Result<bool, ControlStoreError> {
    let expected = candidate.accumulator().binding().transaction().successor();
    if !crate::store::transaction::records_were_inserted_in(
        connection,
        crate::ControlStoreKind::DeploymentControl,
        candidate.store(),
        expected,
        candidate
            .records()
            .iter()
            .map(|member| member.record())
            .chain(std::iter::once(candidate.checkpoint().successor_record())),
    )? {
        return Ok(false);
    }
    if !a2a::new_members_match(connection, candidate)?
        || !a2a_discovery::members_match(connection, candidate)?
        || !session_basis::members_match(connection, candidate)?
    {
        return Ok(false);
    }
    // Close the optional roles over intact rows in this transaction.
    // This does not reconstruct deleted or relabeled rows: the application
    // must resolve the evidence named by the authenticated observation.
    let selected = candidate
        .records()
        .iter()
        .filter(|member| {
            matches!(
                member.kind(),
                pale_ports::OperationCustodyRecordKind::CapabilityResult
                    | pale_ports::OperationCustodyRecordKind::AdapterFailure
                    | pale_ports::OperationCustodyRecordKind::AttemptSelection
                    | pale_ports::OperationCustodyRecordKind::Diagnostic
                    | pale_ports::OperationCustodyRecordKind::CollectionFrontier
                    | pale_ports::OperationCustodyRecordKind::NativeOutcome
                    | pale_ports::OperationCustodyRecordKind::HookResponse
            )
        })
        .map(pale_ports::OperationCustodyRecord::record)
        .collect::<Vec<_>>();
    let mut statement = connection
        .prepare(
            "SELECT schema_id, record_ref, record_sha256 \
             FROM pale_deployment_control_record \
             WHERE schema_id IN (?1,?2,?3,?4,?5,?6,?7) AND inserted_transaction_sequence=?8",
        )
        .map_err(|source| sqlite("prepare operation optional record closure", source))?;
    let rows = statement
        .query_map(
            params![
                pale_core::records::ADAPTER_FAILURE_V1_SCHEMA,
                pale_core::records::CAPABILITY_RESULT_V1_SCHEMA,
                pale_ports::PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA,
                pale_core::records::DIAGNOSTIC_V1_SCHEMA,
                pale_ports::PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_SCHEMA,
                pale_ports::PRIVATE_OPERATION_NATIVE_OUTCOME_V1_SCHEMA,
                pale_ports::PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA,
                sqlite_integer(expected.get())?,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .map_err(|source| sqlite("read operation optional record closure", source))?;
    for row in rows {
        let (schema, reference, digest) =
            row.map_err(|source| sqlite("decode operation optional record closure", source))?;
        if !selected.iter().any(|record| {
            record.schema_id().as_str() == schema
                && record.coordinate().record_ref().as_str() == reference
                && record.coordinate().record_sha256().as_str() == digest
        }) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Apply one exact insert or compare-and-swap private-index mutation.
fn apply_index_mutation(
    connection: &Connection,
    mutation: &OperationCustodyIndexMutation,
) -> Result<(), ControlStoreError> {
    let (role, key, expected, successor) = mutation_parts(mutation);
    let changed = match expected {
        OperationCustodyIndexPredecessor::Absent => connection.execute(
            "INSERT INTO pale_deployment_operation_index \
             (index_role, index_key, record_ref, record_sha256) VALUES (?1, ?2, ?3, ?4)",
            params![
                role.as_str(),
                key.as_str(),
                successor.record_ref().as_str(),
                successor.record_sha256().as_str(),
            ],
        ),
        OperationCustodyIndexPredecessor::Exact(record) if record.coordinate() == successor => {
            return if index_matches(connection, role, key, expected)? {
                Ok(())
            } else {
                Err(ControlStoreError::new(
                    ControlStoreErrorKind::CandidateConflict,
                ))
            };
        }
        OperationCustodyIndexPredecessor::Exact(expected) => connection.execute(
            "UPDATE pale_deployment_operation_index \
             SET record_ref = ?1, record_sha256 = ?2 \
             WHERE index_role = ?3 AND index_key = ?4 \
               AND record_ref = ?5 AND record_sha256 = ?6",
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
    .map_err(|source| sqlite("advance operation index", source))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))
    }
}

/// Compare one private index with its exact absent-or-record predecessor.
fn index_matches(
    connection: &Connection,
    role: IndexRole,
    key: &OperationCustodyIndexKey,
    expected: &OperationCustodyIndexPredecessor,
) -> Result<bool, ControlStoreError> {
    let observed = read_index_coordinate(connection, role, key)?;
    match (observed, expected) {
        (None, OperationCustodyIndexPredecessor::Absent) => Ok(true),
        (Some(observed), OperationCustodyIndexPredecessor::Exact(expected))
            if observed == *expected.coordinate() =>
        {
            record_matches(connection, expected)
        }
        _ => Ok(false),
    }
}

/// Compare one private index with the exact proposed successor coordinate.
fn index_points_to(
    connection: &Connection,
    role: IndexRole,
    key: &OperationCustodyIndexKey,
    successor: &ControlRecordCoordinate,
) -> Result<bool, ControlStoreError> {
    Ok(read_index_coordinate(connection, role, key)?.as_ref() == Some(successor))
}

/// Read one private index coordinate without scanning another role or key.
fn read_index_coordinate(
    connection: &Connection,
    role: IndexRole,
    key: &OperationCustodyIndexKey,
) -> Result<Option<ControlRecordCoordinate>, ControlStoreError> {
    let row = connection
        .query_row(
            "SELECT record_ref, record_sha256 FROM pale_deployment_operation_index \
             WHERE index_role = ?1 AND index_key = ?2",
            params![role.as_str(), key.as_str()],
            |row| Ok(decode_record_coordinate(row.get_ref(0)?, row.get_ref(1)?)),
        )
        .optional()
        .map_err(|source| sqlite("read operation index", source))?;
    row.transpose()
}

/// Decode one persisted immutable-record coordinate without accepting malformed rows.
fn decode_record_coordinate(
    record_ref: ValueRef<'_>,
    record_sha256: ValueRef<'_>,
) -> Result<ControlRecordCoordinate, ControlStoreError> {
    let (ValueRef::Text(record_ref), ValueRef::Text(record_sha256)) = (record_ref, record_sha256)
    else {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    };
    if record_ref.is_empty()
        || record_ref.len() > pale_core::records::MAX_RESIDENT_ID_BYTES
        || record_sha256.len() != 64
    {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    let record_ref = std::str::from_utf8(record_ref)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let record_sha256 = std::str::from_utf8(record_sha256)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let reference = ResidentRecordRefV1::parse(record_ref)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let digest = ControlRecordDigest::parse(record_sha256)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    Ok(ControlRecordCoordinate::new(reference, digest))
}

/// Check that one present private index closes over its immutable record.
fn index_is_complete(
    connection: &Connection,
    role: IndexRole,
    key: &OperationCustodyIndexKey,
) -> Result<bool, ControlStoreError> {
    let Some(coordinate) = read_index_coordinate(connection, role, key)? else {
        return Ok(true);
    };
    record_is_complete(connection, &coordinate)
}

/// Compare one exact operation hard-domain checkpoint head.
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
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_operation_checkpoint_head \
             WHERE checkpoint_domain_id = ?1 AND domain_epoch = ?2 AND domain_sequence = ?3 \
               AND control_checkpoint_ref = ?4 AND control_checkpoint_sha256 = ?5)",
            params![
                checkpoint.checkpoint_domain_id().as_str(),
                sqlite_integer(checkpoint.domain_epoch().get())?,
                sqlite_integer(checkpoint.domain_sequence().get())?,
                checkpoint.coordinate().record_ref().as_str(),
                checkpoint.coordinate().record_sha256().as_str(),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare operation checkpoint head", source))
}

/// Check that a current operation checkpoint resolves to its immutable record.
fn checkpoint_frontier_is_complete(
    connection: &Connection,
    domain_id: &str,
) -> Result<bool, ControlStoreError> {
    let row = connection
        .query_row(
            "SELECT control_checkpoint_ref, control_checkpoint_sha256 \
             FROM pale_deployment_operation_checkpoint_head \
             WHERE checkpoint_domain_id = ?1",
            [domain_id],
            |row| Ok(decode_record_coordinate(row.get_ref(0)?, row.get_ref(1)?)),
        )
        .optional()
        .map_err(|source| sqlite("read operation checkpoint head", source))?;
    let Some(coordinate) = row.transpose()? else {
        return Ok(false);
    };
    record_is_complete(connection, &coordinate)
}

/// Compare-and-swap one exact operation hard-domain checkpoint head.
fn advance_checkpoint(
    connection: &Connection,
    phase: &pale_ports::OperationCheckpointPhase,
    successor: &ControlCheckpointHead,
) -> Result<(), ControlStoreError> {
    let changed = match phase {
        pale_ports::OperationCheckpointPhase::Initial => connection.execute(
            "INSERT INTO pale_deployment_operation_checkpoint_head \
             (checkpoint_domain_id,domain_epoch,domain_sequence,control_checkpoint_ref,control_checkpoint_sha256) VALUES (?1,?2,?3,?4,?5)",
            params![successor.checkpoint_domain_id().as_str(), sqlite_integer(successor.domain_epoch().get())?,
                sqlite_integer(successor.domain_sequence().get())?, successor.coordinate().record_ref().as_str(), successor.coordinate().record_sha256().as_str()],
        ),
        pale_ports::OperationCheckpointPhase::Successor { predecessor: expected, .. } => connection.execute(
            "UPDATE pale_deployment_operation_checkpoint_head \
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
        ),
    }.map_err(|source| sqlite("advance operation checkpoint head", source))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))
    }
}

/// Closed physical index vocabulary stored in the source-owned migration.
#[derive(Clone, Copy, PartialEq, Eq)]
enum IndexRole {
    NativeOutcome,
    A2aSendPreparation,
    A2aSendPreparationCheckpoint,
    CurrentA2aGetTask,
    A2aGetTaskCheckpoint,
    RequestForHandle,
    StartObservation,
    ObservationCheckpoint,
    RequestEvidence,
    RequestOperation,
    OperationHandle,
    ParentChildMembership,
    CollectionHead,
    LatestObservation,
    A2aTaskBinding,
    LatestA2aTaskObservation,
    FirstA2aTaskObservation,
    A2aTaskObservationCheckpoint,
}

impl IndexRole {
    /// Return the migration-constrained physical spelling.
    const fn as_str(self) -> &'static str {
        match self {
            Self::NativeOutcome => "native_outcome",
            Self::A2aSendPreparation => "a2a_send_preparation",
            Self::A2aSendPreparationCheckpoint => "a2a_send_preparation_checkpoint",
            Self::CurrentA2aGetTask => "current_a2a_get_task",
            Self::A2aGetTaskCheckpoint => "a2a_get_task_checkpoint",
            Self::RequestForHandle => "request_for_handle",
            Self::StartObservation => "start_observation",
            Self::ObservationCheckpoint => "observation_checkpoint",
            Self::RequestEvidence => "request_evidence",
            Self::RequestOperation => "request_operation",
            Self::OperationHandle => "operation_handle",
            Self::ParentChildMembership => "parent_child_membership",
            Self::CollectionHead => "collection_head",
            Self::LatestObservation => "latest_observation",
            Self::A2aTaskBinding => "a2a_task_binding",
            Self::LatestA2aTaskObservation => "latest_a2a_task_observation",
            Self::FirstA2aTaskObservation => "first_a2a_task_observation",
            Self::A2aTaskObservationCheckpoint => "a2a_task_observation_checkpoint",
        }
    }
}

/// Project a closed mutation without erasing its private index role.
const fn mutation_parts(
    mutation: &OperationCustodyIndexMutation,
) -> (
    IndexRole,
    &OperationCustodyIndexKey,
    &OperationCustodyIndexPredecessor,
    &ControlRecordCoordinate,
) {
    match mutation {
        OperationCustodyIndexMutation::NativeOutcome {
            key,
            expected,
            successor,
        } => (IndexRole::NativeOutcome, key, expected, successor),
        OperationCustodyIndexMutation::A2aSendPreparation {
            key,
            expected,
            successor,
        } => (IndexRole::A2aSendPreparation, key, expected, successor),
        OperationCustodyIndexMutation::A2aSendPreparationCheckpoint { key, successor } => (
            IndexRole::A2aSendPreparationCheckpoint,
            key,
            &OperationCustodyIndexPredecessor::Absent,
            successor,
        ),
        OperationCustodyIndexMutation::CurrentA2aGetTask {
            key,
            expected,
            successor,
        } => (IndexRole::CurrentA2aGetTask, key, expected, successor),
        OperationCustodyIndexMutation::A2aGetTaskCheckpoint { key, successor } => (
            IndexRole::A2aGetTaskCheckpoint,
            key,
            &OperationCustodyIndexPredecessor::Absent,
            successor,
        ),
        OperationCustodyIndexMutation::A2aTaskBinding {
            key,
            expected,
            successor,
            ..
        } => (IndexRole::A2aTaskBinding, key, expected, successor),
        OperationCustodyIndexMutation::LatestA2aTaskObservation {
            key,
            expected,
            successor,
        } => (
            IndexRole::LatestA2aTaskObservation,
            key,
            expected,
            successor,
        ),
        OperationCustodyIndexMutation::FirstA2aTaskObservation { key, successor } => (
            IndexRole::FirstA2aTaskObservation,
            key,
            &OperationCustodyIndexPredecessor::Absent,
            successor,
        ),
        OperationCustodyIndexMutation::A2aTaskObservationCheckpoint { key, successor } => (
            IndexRole::A2aTaskObservationCheckpoint,
            key,
            &OperationCustodyIndexPredecessor::Absent,
            successor,
        ),
        OperationCustodyIndexMutation::RequestForHandle { key, successor } => (
            IndexRole::RequestForHandle,
            key,
            &OperationCustodyIndexPredecessor::Absent,
            successor,
        ),
        OperationCustodyIndexMutation::StartObservation { key, successor } => (
            IndexRole::StartObservation,
            key,
            &OperationCustodyIndexPredecessor::Absent,
            successor,
        ),
        OperationCustodyIndexMutation::ObservationCheckpoint { key, successor } => (
            IndexRole::ObservationCheckpoint,
            key,
            &OperationCustodyIndexPredecessor::Absent,
            successor,
        ),
        OperationCustodyIndexMutation::RequestEvidence {
            key,
            expected,
            successor,
        } => (IndexRole::RequestEvidence, key, expected, successor),
        OperationCustodyIndexMutation::RequestOperation {
            key,
            expected,
            successor,
        } => (IndexRole::RequestOperation, key, expected, successor),
        OperationCustodyIndexMutation::OperationHandle {
            key,
            expected,
            successor,
        } => (IndexRole::OperationHandle, key, expected, successor),
        OperationCustodyIndexMutation::ParentChildMembership {
            key,
            expected,
            successor,
        } => (IndexRole::ParentChildMembership, key, expected, successor),
        OperationCustodyIndexMutation::CollectionHead {
            key,
            expected,
            successor,
        } => (IndexRole::CollectionHead, key, expected, successor),
        OperationCustodyIndexMutation::LatestObservation {
            key,
            expected,
            successor,
        } => (IndexRole::LatestObservation, key, expected, successor),
    }
}

/// Project a point query into its migration-constrained role and key.
const fn query_parts(query: &OperationCustodyIndexQuery) -> (IndexRole, &OperationCustodyIndexKey) {
    match query {
        OperationCustodyIndexQuery::NativeOutcome(key) => (IndexRole::NativeOutcome, key),
        OperationCustodyIndexQuery::A2aSendPreparation(key) => (IndexRole::A2aSendPreparation, key),
        OperationCustodyIndexQuery::A2aSendPreparationCheckpoint(key) => {
            (IndexRole::A2aSendPreparationCheckpoint, key)
        }
        OperationCustodyIndexQuery::CurrentA2aGetTask(key) => (IndexRole::CurrentA2aGetTask, key),
        OperationCustodyIndexQuery::A2aGetTaskCheckpoint(key) => {
            (IndexRole::A2aGetTaskCheckpoint, key)
        }
        OperationCustodyIndexQuery::A2aTaskBinding(key) => (IndexRole::A2aTaskBinding, key),
        OperationCustodyIndexQuery::LatestA2aTaskObservation(key) => {
            (IndexRole::LatestA2aTaskObservation, key)
        }
        OperationCustodyIndexQuery::FirstA2aTaskObservation(key) => {
            (IndexRole::FirstA2aTaskObservation, key)
        }
        OperationCustodyIndexQuery::A2aTaskObservationCheckpoint(key) => {
            (IndexRole::A2aTaskObservationCheckpoint, key)
        }
        OperationCustodyIndexQuery::RequestForHandle(key) => (IndexRole::RequestForHandle, key),
        OperationCustodyIndexQuery::StartObservation(key) => (IndexRole::StartObservation, key),
        OperationCustodyIndexQuery::ObservationCheckpoint(key) => {
            (IndexRole::ObservationCheckpoint, key)
        }
        OperationCustodyIndexQuery::RequestEvidence(key) => (IndexRole::RequestEvidence, key),
        OperationCustodyIndexQuery::RequestOperation(key) => (IndexRole::RequestOperation, key),
        OperationCustodyIndexQuery::OperationHandle(key) => (IndexRole::OperationHandle, key),
        OperationCustodyIndexQuery::ParentChildMembership(key) => {
            (IndexRole::ParentChildMembership, key)
        }
        OperationCustodyIndexQuery::CollectionHead(key) => (IndexRole::CollectionHead, key),
        OperationCustodyIndexQuery::LatestObservation(key) => (IndexRole::LatestObservation, key),
    }
}

#[cfg(test)]
#[path = "../tests/internal/operation.rs"]
pub(crate) mod tests;
