//! Deployment-bound `SQLite` custody for Event delivery and Genealogy views.
//!
//! One immediate transaction appends to the existing authority journal, stores
//! immutable records in the shared deployment record table, and advances only
//! the typed Event indexes selected by the prepared transition. Owner payloads,
//! semantic admission, producer authority, notification, and native effects
//! remain outside this adapter.

use pale_core::records::{JournalSequenceV1, ResidentRecordRefV1};
use pale_ports::{
    AuthorityByteClass, AuthorityStoreCoordinate, ControlRecordCoordinate, ControlRecordDigest,
    ControlStoreError, ControlStoreErrorKind, EventCustodyCommitOutcome, EventCustodyPort,
    EventCustodyReadBackOutcome, EventCustodyTransition, EventCustodyTransitionKind,
    EventInboxCursor, EventInboxPage, EventInboxPageBoundary, EventInboxPageEntry,
    EventInboxPageRead, EventIndexKey, EventIndexMutation, EventIndexObservation,
    EventIndexPredecessor, EventIndexQuery, EventRecordObservation, PaleDerivedOutboxCursor,
    PaleDerivedOutboxPage, PaleDerivedOutboxPageBoundary, PaleDerivedOutboxPageEntry,
    PaleDerivedOutboxPageRead,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use crate::deployment::{
    insert_record, read_record, read_record_bounded, record_body_length, record_matches,
    record_reference_exists, require_coordinate, require_store_binding,
};
use crate::error::{sqlite, sqlite_integer};
use crate::frame;
use crate::ControlSqliteError;

use crate::store::journal::{
    append_authority_entry, journal_entry_matches, journal_frontier_is_complete,
    journal_predecessor_matches, journal_successor_matches, read_journal_head,
    require_journal_bound,
};

mod history;

/// `SQLite` realization of atomic Event/Genealogy custody.
///
/// Construction remains crate-private until the storage profile and resident
/// assembly are admitted. Neither [`EventCustodyCommitOutcome::Committed`] nor
/// [`EventCustodyCommitOutcome::ExactReplay`] grants notification or fanout
/// authority. Application recovery must decide whether an exact observation
/// belongs to the current attempt before any continuation.
pub struct SqliteEventCustody<'connection> {
    /// Open deployment-control connection whose Pale Runtime assembly remains external.
    pub(crate) connection: &'connection mut Connection,
    /// Exact store incarnation represented by [`Self::connection`].
    pub(crate) coordinate: AuthorityStoreCoordinate,
}

impl pale_ports::ControlTransactionPort for SqliteEventCustody<'_> {
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

impl EventCustodyPort for SqliteEventCustody<'_> {
    fn read_committed_members(
        &mut self,
        request: &pale_ports::EventCustodyHistoryRead,
    ) -> Result<pale_ports::EventCustodyHistoryObservation, ControlStoreError> {
        require_coordinate(&self.coordinate, request.store())?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin Event history read", source))?;
        let result = history::read(&transaction, request)?;
        transaction
            .rollback()
            .map_err(|source| sqlite("close Event history read", source))?;
        Ok(result)
    }

    fn compare_and_commit(
        &mut self,
        transition: &EventCustodyTransition,
        maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyCommitOutcome, ControlStoreError> {
        require_journal_bound(transition.journal_append(), maximum_journal_payload_bytes)?;
        require_coordinate(&self.coordinate, transition.store())?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|source| sqlite("begin Event custody transition", source))?;
        require_store_binding(&transaction, transition.store())?;
        crate::store::transaction::read(&transaction, transition.store())?;

        match observe(&transaction, transition, maximum_journal_payload_bytes)? {
            EventCustodyReadBackOutcome::ExactSuccessor => {
                transaction
                    .rollback()
                    .map_err(|source| sqlite("close exact Event custody replay", source))?;
                Ok(EventCustodyCommitOutcome::ExactReplay)
            }
            EventCustodyReadBackOutcome::Predecessor => {
                crate::store::transaction::advance(&transaction, transition.transaction())?;
                for member in transition.members() {
                    insert_record(&transaction, member.record())?;
                }
                for mutation in transition.index_mutations() {
                    apply_index_mutation(&transaction, mutation)?;
                }
                apply_inbox_order(&transaction, transition)?;
                apply_inbox_version(&transaction, transition)?;
                append_authority_entry(&transaction, transition.journal_append())?;
                if observe(&transaction, transition, maximum_journal_payload_bytes)?
                    != EventCustodyReadBackOutcome::ExactSuccessor
                {
                    return Err(ControlStoreError::new(
                        ControlStoreErrorKind::RecoveryRequired,
                    ));
                }
                transaction.commit().map_err(|source| {
                    ControlStoreError::with_source(
                        ControlStoreErrorKind::OutcomeUnknown,
                        ControlSqliteError::sqlite("commit Event custody transition", source),
                    )
                })?;
                Ok(EventCustodyCommitOutcome::Committed)
            }
            EventCustodyReadBackOutcome::ConflictingOrPartial => {
                let kind =
                    classify_conflict(&transaction, transition, maximum_journal_payload_bytes)?;
                Err(ControlStoreError::new(kind))
            }
        }
    }

    fn observe_exact(
        &mut self,
        transition: &EventCustodyTransition,
        maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyReadBackOutcome, ControlStoreError> {
        require_journal_bound(transition.journal_append(), maximum_journal_payload_bytes)?;
        require_coordinate(&self.coordinate, transition.store())?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin exact Event read-back", source))?;
        require_store_binding(&transaction, transition.store())?;
        let result = match observe(&transaction, transition, maximum_journal_payload_bytes) {
            // Known corrupt custody is a contradictory read-back, not missing
            // evidence. A byte-limit failure still leaves the outcome unknown.
            Err(error) if error.kind() == ControlStoreErrorKind::RecoveryRequired => {
                EventCustodyReadBackOutcome::ConflictingOrPartial
            }
            observed => observed?,
        };
        transaction
            .rollback()
            .map_err(|source| sqlite("close exact Event read-back", source))?;
        Ok(result)
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &EventIndexQuery,
        maximum_body_bytes: usize,
    ) -> Result<EventIndexObservation, ControlStoreError> {
        require_positive_body_limit(maximum_body_bytes)?;
        require_coordinate(&self.coordinate, store)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin bounded Event index read", source))?;
        require_store_binding(&transaction, store)?;
        let observed = if let Some(coordinate) = read_index_coordinate(&transaction, query)? {
            let record = read_event_record_bounded(&transaction, &coordinate, maximum_body_bytes)?
                .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
            EventIndexObservation::exact_record(query.clone(), record)
                .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?
        } else {
            EventIndexObservation::absent(query.clone())
        };
        transaction
            .rollback()
            .map_err(|source| sqlite("close bounded Event index read", source))?;
        Ok(observed)
    }

    fn read_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<EventRecordObservation, ControlStoreError> {
        require_positive_body_limit(maximum_body_bytes)?;
        require_coordinate(&self.coordinate, store)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin bounded Event record read", source))?;
        require_store_binding(&transaction, store)?;
        let observed =
            match read_event_record_bounded(&transaction, coordinate, maximum_body_bytes)? {
                Some(record) => EventRecordObservation::exact(coordinate.clone(), record)
                    .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)),
                None => Ok(EventRecordObservation::absent(coordinate.clone())),
            }?;
        transaction
            .rollback()
            .map_err(|source| sqlite("close bounded Event record read", source))?;
        Ok(observed)
    }

    fn read_inbox_page(
        &mut self,
        request: &EventInboxPageRead,
    ) -> Result<EventInboxPage, ControlStoreError> {
        require_coordinate(&self.coordinate, request.store())?;
        require_store_binding(self.connection, request.store())?;
        read_inbox_page(self.connection, request)
    }

    fn read_pale_derived_outbox_page(
        &mut self,
        request: &PaleDerivedOutboxPageRead,
    ) -> Result<PaleDerivedOutboxPage, ControlStoreError> {
        require_coordinate(&self.coordinate, request.store())?;
        require_store_binding(self.connection, request.store())?;
        read_pale_derived_outbox_page(self.connection, request)
    }
}

fn require_positive_body_limit(maximum_body_bytes: usize) -> Result<(), ControlStoreError> {
    if maximum_body_bytes == 0 {
        Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded))
    } else {
        Ok(())
    }
}

fn read_event_record_bounded(
    connection: &Connection,
    coordinate: &ControlRecordCoordinate,
    maximum_body_bytes: usize,
) -> Result<Option<pale_ports::ControlRecord>, ControlStoreError> {
    let Some(body_bytes) = record_body_length(connection, coordinate)? else {
        return Ok(None);
    };
    if body_bytes > maximum_body_bytes {
        return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
    }
    read_record_bounded(connection, coordinate, maximum_body_bytes)
}

/// Classify the complete predecessor, complete successor, or every other state.
fn observe(
    connection: &Connection,
    transition: &EventCustodyTransition,
    maximum_journal_payload_bytes: usize,
) -> Result<EventCustodyReadBackOutcome, ControlStoreError> {
    if successor_matches(connection, transition, maximum_journal_payload_bytes)? {
        return Ok(EventCustodyReadBackOutcome::ExactSuccessor);
    }
    if predecessor_matches(connection, transition)? {
        return Ok(EventCustodyReadBackOutcome::Predecessor);
    }
    Ok(EventCustodyReadBackOutcome::ConflictingOrPartial)
}

/// Require the journal append, immutable records, indexes, and inbox projection.
fn successor_matches(
    connection: &Connection,
    transition: &EventCustodyTransition,
    maximum_journal_payload_bytes: usize,
) -> Result<bool, ControlStoreError> {
    if !journal_successor_matches(
        connection,
        transition.journal_append(),
        maximum_journal_payload_bytes,
    )? {
        return Ok(false);
    }
    for member in transition.members() {
        if !record_matches(connection, member.record())? {
            return Ok(false);
        }
    }
    if !crate::store::transaction::records_were_inserted_in(
        connection,
        crate::ControlStoreKind::DeploymentControl,
        transition.store(),
        transition.transaction().successor(),
        transition.members().iter().map(|member| member.record()),
    )? {
        return Ok(false);
    }
    for mutation in transition.index_mutations() {
        if !index_points_to(connection, mutation.query(), mutation.successor())? {
            return Ok(false);
        }
    }
    for assertion in transition.index_assertions() {
        if !index_matches(connection, assertion.query(), assertion.expected())? {
            return Ok(false);
        }
    }
    Ok(inbox_order_successor_matches(connection, transition)?
        && inbox_version_successor_matches(connection, transition)?)
}

/// Require the exact journal/index predecessor and absence of every append.
fn predecessor_matches(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<bool, ControlStoreError> {
    if !journal_predecessor_matches(connection, transition.journal_append())? {
        return Ok(false);
    }
    if crate::store::transaction::read(connection, transition.store())?
        != transition.transaction().expected()
    {
        return Ok(false);
    }
    for member in transition.members() {
        if record_reference_exists(
            connection,
            member.record().coordinate().record_ref().as_str(),
        )? {
            return Ok(false);
        }
    }
    for mutation in transition.index_mutations() {
        if !index_matches(connection, mutation.query(), mutation.expected())? {
            return Ok(false);
        }
    }
    for assertion in transition.index_assertions() {
        if !index_matches(connection, assertion.query(), assertion.expected())? {
            return Ok(false);
        }
    }
    Ok(inbox_order_predecessor_matches(connection, transition)?
        && inbox_version_predecessor_matches(connection, transition)?)
}

/// Distinguish stale complete state from candidate-shaped partial custody.
fn classify_conflict(
    connection: &Connection,
    transition: &EventCustodyTransition,
    maximum_journal_payload_bytes: usize,
) -> Result<ControlStoreErrorKind, ControlStoreError> {
    if candidate_material_is_present(connection, transition, maximum_journal_payload_bytes)?
        || !journal_frontier_is_complete(connection)?
        || !selected_indexes_are_complete(connection, transition)?
        || !selected_inbox_projection_is_complete(connection, transition)?
    {
        Ok(ControlStoreErrorKind::RecoveryRequired)
    } else {
        Ok(ControlStoreErrorKind::CandidateConflict)
    }
}

/// Require the stable order and version head selected by an inbox transition.
fn selected_inbox_projection_is_complete(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<bool, ControlStoreError> {
    let Some(mutation) = inbox_delivery_mutation(transition) else {
        return Ok(true);
    };
    let Some(order) = read_inbox_order(connection, mutation.query().key())? else {
        return Ok(false);
    };
    let current = read_index_coordinate(connection, mutation.query())?;
    Ok(
        read_latest_inbox_version(connection, mutation.query().key(), None)?.is_some_and(
            |version| {
                current.as_ref() == Some(&version.record)
                    && order.initial_sequence <= version.mutation_sequence
            },
        ),
    )
}

/// Detect candidate bytes at the current frontier rather than stale history.
fn candidate_material_is_present(
    connection: &Connection,
    transition: &EventCustodyTransition,
    maximum_journal_payload_bytes: usize,
) -> Result<bool, ControlStoreError> {
    let candidate_sequence = transition.journal_append().entry().sequence();
    if read_journal_head(connection)?.is_some_and(|head| head > candidate_sequence) {
        return Ok(false);
    }
    if journal_entry_matches(
        connection,
        transition.journal_append(),
        maximum_journal_payload_bytes,
    )? {
        return Ok(true);
    }
    for member in transition.members() {
        if record_reference_exists(
            connection,
            member.record().coordinate().record_ref().as_str(),
        )? {
            return Ok(true);
        }
    }
    Ok(inbox_order_candidate_is_present(connection, transition)?
        || inbox_version_candidate_is_present(connection, transition)?)
}

/// Require each selected current index pointer to close over an immutable row.
fn selected_indexes_are_complete(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<bool, ControlStoreError> {
    for mutation in transition.index_mutations() {
        if !index_is_complete(connection, mutation.query())? {
            return Ok(false);
        }
    }
    for assertion in transition.index_assertions() {
        if !index_is_complete(connection, assertion.query())? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Insert one exact typed index or compare-and-swap its immutable predecessor.
fn apply_index_mutation(
    connection: &Connection,
    mutation: &EventIndexMutation,
) -> Result<(), ControlStoreError> {
    let role = query_role(mutation.query());
    let key = mutation.query().key().as_str();
    let successor = mutation.successor();
    let changed = match mutation.expected() {
        EventIndexPredecessor::Absent => connection.execute(
            "INSERT INTO pale_deployment_event_index \
             (index_role, index_key, record_ref, record_sha256) VALUES (?1, ?2, ?3, ?4)",
            params![
                role,
                key,
                successor.record_ref().as_str(),
                successor.record_sha256().as_str(),
            ],
        ),
        EventIndexPredecessor::Exact(expected) => connection.execute(
            "UPDATE pale_deployment_event_index \
             SET record_ref = ?1, record_sha256 = ?2 \
             WHERE index_role = ?3 AND index_key = ?4 \
               AND record_ref = ?5 AND record_sha256 = ?6",
            params![
                successor.record_ref().as_str(),
                successor.record_sha256().as_str(),
                role,
                key,
                expected.coordinate().record_ref().as_str(),
                expected.coordinate().record_sha256().as_str(),
            ],
        ),
    }
    .map_err(|source| sqlite("advance Event index", source))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))
    }
}

/// Compare one typed index with an exact absent-or-record predecessor.
fn index_matches(
    connection: &Connection,
    query: &EventIndexQuery,
    expected: &EventIndexPredecessor,
) -> Result<bool, ControlStoreError> {
    let observed = read_index_coordinate(connection, query)?;
    match (observed, expected) {
        (None, EventIndexPredecessor::Absent) => Ok(true),
        (Some(observed), EventIndexPredecessor::Exact(expected))
            if observed == *expected.coordinate() =>
        {
            record_matches(connection, expected)
        }
        _ => Ok(false),
    }
}

/// Compare one typed index with its exact proposed successor coordinate.
fn index_points_to(
    connection: &Connection,
    query: &EventIndexQuery,
    successor: &ControlRecordCoordinate,
) -> Result<bool, ControlStoreError> {
    Ok(read_index_coordinate(connection, query)?.as_ref() == Some(successor))
}

/// Read one typed index coordinate without scanning another role or key.
fn read_index_coordinate(
    connection: &Connection,
    query: &EventIndexQuery,
) -> Result<Option<ControlRecordCoordinate>, ControlStoreError> {
    let row = connection
        .query_row(
            "SELECT record_ref, record_sha256 FROM pale_deployment_event_index \
             WHERE index_role = ?1 AND index_key = ?2",
            params![query_role(query), query.key().as_str()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|source| sqlite("read Event index", source))?;
    row.map(|(record_ref, record_sha256)| decode_record_coordinate(record_ref, record_sha256))
        .transpose()
}

/// Decode one persisted immutable-record coordinate without accepting malformed rows.
fn decode_record_coordinate(
    record_ref: String,
    record_sha256: String,
) -> Result<ControlRecordCoordinate, ControlStoreError> {
    let reference = ResidentRecordRefV1::parse(record_ref)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let digest = ControlRecordDigest::parse(record_sha256)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    Ok(ControlRecordCoordinate::new(reference, digest))
}

/// Check that one present index closes over its exact immutable record.
fn index_is_complete(
    connection: &Connection,
    query: &EventIndexQuery,
) -> Result<bool, ControlStoreError> {
    let Some(coordinate) = read_index_coordinate(connection, query)? else {
        return Ok(true);
    };
    Ok(read_record(connection, &coordinate)?.is_some())
}

/// Insert the stable inbox order only for a delivery's first admission.
fn apply_inbox_order(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<(), ControlStoreError> {
    if transition.kind() != EventCustodyTransitionKind::EnqueueDelivery {
        return Ok(());
    }
    let subscription = subscription_key(transition)?;
    let delivery = inbox_delivery_key(transition)?;
    connection
        .execute(
            "INSERT INTO pale_deployment_event_inbox_order \
             (subscription_key, journal_sequence, delivery_key) VALUES (?1, ?2, ?3)",
            params![
                subscription.as_str(),
                sqlite_integer(transition.journal_append().entry().sequence().get())?,
                delivery.as_str(),
            ],
        )
        .map(drop)
        .map_err(|source| sqlite("insert Event inbox order", source))
}

/// Append the exact inbox state produced by every inbox-index mutation.
fn apply_inbox_version(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<(), ControlStoreError> {
    let Some(mutation) = inbox_delivery_mutation(transition) else {
        return Ok(());
    };
    let delivery_key = mutation.query().key();
    let order = read_inbox_order(connection, delivery_key)?
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let successor = mutation.successor();
    connection
        .execute(
            "INSERT INTO pale_deployment_event_inbox_version \
             (subscription_key, initial_journal_sequence, delivery_key, \
              mutation_journal_sequence, record_ref, record_sha256) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                order.subscription_key.as_str(),
                sqlite_integer(order.initial_sequence.get())?,
                delivery_key.as_str(),
                sqlite_integer(transition.journal_append().entry().sequence().get())?,
                successor.record_ref().as_str(),
                successor.record_sha256().as_str(),
            ],
        )
        .map(drop)
        .map_err(|source| sqlite("append Event inbox version", source))
}

/// Compare the stable inbox order attached to an initial delivery.
fn inbox_order_successor_matches(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<bool, ControlStoreError> {
    match transition.kind() {
        EventCustodyTransitionKind::EnqueueDelivery => inbox_order_matches(connection, transition),
        EventCustodyTransitionKind::EnqueueReplay
        | EventCustodyTransitionKind::RecordAcknowledgement => {
            inbox_order_exists(connection, transition)
        }
        _ => Ok(true),
    }
}

/// Require no initial-delivery order row before its transition commits.
fn inbox_order_predecessor_matches(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<bool, ControlStoreError> {
    match transition.kind() {
        EventCustodyTransitionKind::EnqueueDelivery => {
            Ok(!inbox_order_candidate_is_present(connection, transition)?)
        }
        EventCustodyTransitionKind::EnqueueReplay
        | EventCustodyTransitionKind::RecordAcknowledgement => {
            inbox_order_exists(connection, transition)
        }
        _ => Ok(true),
    }
}

/// Compare the version row produced at the transition's journal coordinate.
fn inbox_version_successor_matches(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<bool, ControlStoreError> {
    let Some(mutation) = inbox_delivery_mutation(transition) else {
        return Ok(true);
    };
    let candidate_sequence = transition.journal_append().entry().sequence();
    Ok(
        read_latest_inbox_version(connection, mutation.query().key(), Some(candidate_sequence))?
            .is_some_and(|version| {
                version.mutation_sequence == candidate_sequence
                    && version.record == *mutation.successor()
            }),
    )
}

/// Compare the version history with the exact inbox-index predecessor.
fn inbox_version_predecessor_matches(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<bool, ControlStoreError> {
    let Some(mutation) = inbox_delivery_mutation(transition) else {
        return Ok(true);
    };
    let latest = read_latest_inbox_version(connection, mutation.query().key(), None)?;
    match mutation.expected() {
        EventIndexPredecessor::Absent => Ok(latest.is_none()),
        EventIndexPredecessor::Exact(expected) => Ok(latest.is_some_and(|version| {
            transition
                .journal_append()
                .expected_head()
                .is_some_and(|head| version.mutation_sequence <= head)
                && version.record == *expected.coordinate()
        })),
    }
}

/// Test whether an initial-delivery order row already exists for the candidate.
fn inbox_order_candidate_is_present(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<bool, ControlStoreError> {
    if transition.kind() != EventCustodyTransitionKind::EnqueueDelivery {
        return Ok(false);
    }
    Ok(read_inbox_order(connection, inbox_delivery_key(transition)?)?.is_some())
}

/// Compare the exact initial-delivery ordering coordinate.
fn inbox_order_matches(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<bool, ControlStoreError> {
    let subscription = subscription_key(transition)?;
    let delivery = inbox_delivery_key(transition)?;
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_event_inbox_order \
             WHERE subscription_key = ?1 AND journal_sequence = ?2 AND delivery_key = ?3)",
            params![
                subscription.as_str(),
                sqlite_integer(transition.journal_append().entry().sequence().get())?,
                delivery.as_str(),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare Event inbox order", source))
}

/// Test for one stable initial-delivery order row by subscription and delivery key.
fn inbox_order_exists(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<bool, ControlStoreError> {
    let delivery = inbox_delivery_key(transition)?;
    let observed = read_inbox_order(connection, delivery)?;
    match transition.kind() {
        EventCustodyTransitionKind::EnqueueReplay => {
            let expected = subscription_key(transition)?;
            Ok(observed.is_some_and(|order| order.subscription_key == *expected))
        }
        EventCustodyTransitionKind::RecordAcknowledgement => Ok(observed.is_some()),
        _ => Ok(false),
    }
}

/// Stable coordinates retained when one delivery first enters an inbox.
struct InboxOrderCoordinate {
    /// Subscription whose inbox owns the delivery.
    subscription_key: EventIndexKey,
    /// Journal sequence that fixes the delivery's paging position.
    initial_sequence: JournalSequenceV1,
}

/// One immutable inbox state at its mutation journal coordinate.
struct InboxVersionCoordinate {
    /// Journal sequence whose transition produced [`Self::record`].
    mutation_sequence: JournalSequenceV1,
    /// Exact immutable inbox-state record selected at that sequence.
    record: ControlRecordCoordinate,
}

/// Read a delivery's unique stable inbox-order coordinate without a scan.
fn read_inbox_order(
    connection: &Connection,
    delivery_key: &EventIndexKey,
) -> Result<Option<InboxOrderCoordinate>, ControlStoreError> {
    connection
        .query_row(
            "SELECT subscription_key, journal_sequence \
             FROM pale_deployment_event_inbox_order WHERE delivery_key = ?1",
            [delivery_key.as_str()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|source| sqlite("read Event inbox order", source))?
        .map(|(subscription_key, initial_sequence)| {
            Ok(InboxOrderCoordinate {
                subscription_key: EventIndexKey::new(
                    ControlRecordDigest::parse(subscription_key).map_err(|_| {
                        ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
                    })?,
                ),
                initial_sequence: decode_sequence(initial_sequence)?,
            })
        })
        .transpose()
}

/// Read the greatest inbox version at or before an optional fixed frontier.
fn read_latest_inbox_version(
    connection: &Connection,
    delivery_key: &EventIndexKey,
    through: Option<JournalSequenceV1>,
) -> Result<Option<InboxVersionCoordinate>, ControlStoreError> {
    let through = through
        .map(JournalSequenceV1::get)
        .map(sqlite_integer)
        .transpose()?;
    connection
        .query_row(
            "SELECT mutation_journal_sequence, record_ref, record_sha256 \
             FROM pale_deployment_event_inbox_version \
             WHERE delivery_key = ?1 AND (?2 IS NULL OR mutation_journal_sequence <= ?2) \
             ORDER BY mutation_journal_sequence DESC LIMIT 1",
            params![delivery_key.as_str(), through],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|source| sqlite("read Event inbox version", source))?
        .map(|(mutation_sequence, record_ref, record_sha256)| {
            Ok(InboxVersionCoordinate {
                mutation_sequence: decode_sequence(mutation_sequence)?,
                record: decode_record_coordinate(record_ref, record_sha256)?,
            })
        })
        .transpose()
}

/// Test whether the candidate journal coordinate already owns an inbox version.
fn inbox_version_candidate_is_present(
    connection: &Connection,
    transition: &EventCustodyTransition,
) -> Result<bool, ControlStoreError> {
    let Some(mutation) = inbox_delivery_mutation(transition) else {
        return Ok(false);
    };
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_event_inbox_version \
             WHERE delivery_key = ?1 AND mutation_journal_sequence = ?2)",
            params![
                mutation.query().key().as_str(),
                sqlite_integer(transition.journal_append().entry().sequence().get())?,
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("test Event inbox version", source))
}

/// Recover the asserted subscription key from a delivery transition.
fn subscription_key(
    transition: &EventCustodyTransition,
) -> Result<&EventIndexKey, ControlStoreError> {
    transition
        .index_assertions()
        .iter()
        .find_map(|assertion| match assertion.query() {
            EventIndexQuery::Subscription(key) => Some(key),
            _ => None,
        })
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
}

/// Recover the mutated inbox-delivery key from a delivery transition.
fn inbox_delivery_key(
    transition: &EventCustodyTransition,
) -> Result<&EventIndexKey, ControlStoreError> {
    transition
        .index_mutations()
        .iter()
        .find_map(|mutation| match mutation.query() {
            EventIndexQuery::InboxDelivery(key) => Some(key),
            _ => None,
        })
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
}

/// Recover the sole inbox-delivery mutation from a validated transition.
fn inbox_delivery_mutation(transition: &EventCustodyTransition) -> Option<&EventIndexMutation> {
    transition
        .index_mutations()
        .iter()
        .find(|mutation| matches!(mutation.query(), EventIndexQuery::InboxDelivery(_)))
}

/// Read one bounded inbox page under the request's fixed journal frontier.
fn read_inbox_page(
    connection: &Connection,
    request: &EventInboxPageRead,
) -> Result<EventInboxPage, ControlStoreError> {
    if read_journal_head(connection)?.is_none_or(|head| head < request.through()) {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    let limit = request
        .maximum_items()
        .checked_add(1)
        .and_then(|value| i64::try_from(value).ok())
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let after_sequence = request
        .after()
        .map_or(0, |cursor| cursor.journal_sequence().get());
    let after_key = request
        .after()
        .map_or("", |cursor| cursor.delivery_key().as_str());
    let mut statement = connection
        .prepare(
            "SELECT ordering.journal_sequence, ordering.delivery_key, \
                    version.record_ref, version.record_sha256 \
             FROM pale_deployment_event_inbox_order AS ordering \
             LEFT JOIN pale_deployment_event_inbox_version AS version \
               ON version.subscription_key = ordering.subscription_key \
              AND version.initial_journal_sequence = ordering.journal_sequence \
              AND version.delivery_key = ordering.delivery_key \
              AND version.mutation_journal_sequence = ( \
                  SELECT MAX(candidate.mutation_journal_sequence) \
                  FROM pale_deployment_event_inbox_version AS candidate \
                  WHERE candidate.subscription_key = ordering.subscription_key \
                    AND candidate.initial_journal_sequence = ordering.journal_sequence \
                    AND candidate.delivery_key = ordering.delivery_key \
                    AND candidate.mutation_journal_sequence <= ?2 \
              ) \
             WHERE ordering.subscription_key = ?1 \
               AND ordering.journal_sequence <= ?2 \
               AND (ordering.journal_sequence > ?3 \
                    OR (ordering.journal_sequence = ?3 AND ordering.delivery_key > ?4)) \
             ORDER BY ordering.journal_sequence ASC, ordering.delivery_key ASC LIMIT ?5",
        )
        .map_err(|source| sqlite("prepare Event inbox page", source))?;
    let mut rows = statement
        .query(params![
            request.subscription_key().as_str(),
            sqlite_integer(request.through().get())?,
            sqlite_integer(after_sequence)?,
            after_key,
            limit,
        ])
        .map_err(|source| sqlite("query Event inbox page", source))?;
    let mut entries = Vec::new();
    let mut total_record_bytes = 0usize;
    let mut has_more = false;
    while let Some(row) = rows
        .next()
        .map_err(|source| sqlite("read Event inbox page", source))?
    {
        let sequence = decode_sequence(
            row.get::<_, i64>(0)
                .map_err(|source| sqlite("decode Event inbox sequence", source))?,
        )?;
        let delivery_key = EventIndexKey::new(
            ControlRecordDigest::parse(
                row.get::<_, String>(1)
                    .map_err(|source| sqlite("decode Event inbox key", source))?,
            )
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?,
        );
        if entries.len() == request.maximum_items() {
            has_more = true;
            break;
        }
        let record_ref = row
            .get::<_, Option<String>>(2)
            .map_err(|source| sqlite("decode Event inbox reference", source))?
            .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        let record_sha256 = row
            .get::<_, Option<String>>(3)
            .map_err(|source| sqlite("decode Event inbox digest", source))?
            .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        let coordinate = decode_record_coordinate(record_ref, record_sha256)?;
        let Some(record_bytes) = record_body_length(connection, &coordinate)? else {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        };
        if record_bytes > request.maximum_record_bytes() {
            return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
        }
        total_record_bytes = total_record_bytes
            .checked_add(record_bytes)
            .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::BoundExceeded))?;
        if total_record_bytes > request.maximum_total_record_bytes() {
            return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
        }
        let record = read_record_bounded(connection, &coordinate, record_bytes)?
            .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        entries.push(EventInboxPageEntry::new(
            EventInboxCursor::new(sequence, delivery_key),
            record,
        ));
    }
    let boundary = if has_more {
        let resume_after = entries
            .last()
            .map(|entry| entry.cursor().clone())
            .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        EventInboxPageBoundary::More { resume_after }
    } else {
        EventInboxPageBoundary::Complete
    };
    EventInboxPage::new(request, entries, boundary)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
}

/// Read current Pale-derived retention heads in stable key order.
fn read_pale_derived_outbox_page(
    connection: &Connection,
    request: &PaleDerivedOutboxPageRead,
) -> Result<PaleDerivedOutboxPage, ControlStoreError> {
    let limit = request
        .maximum_items()
        .checked_add(1)
        .and_then(|value| i64::try_from(value).ok())
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let after_key = request
        .after()
        .map_or("", |cursor| cursor.publication_key().as_str());
    let mut statement = connection
        .prepare(
            "SELECT index_key, record_ref, record_sha256 \
             FROM pale_deployment_event_index \
             WHERE index_role = ?1 AND index_key > ?2 \
             ORDER BY index_key ASC LIMIT ?3",
        )
        .map_err(|source| sqlite("prepare Pale-derived outbox page", source))?;
    let mut rows = statement
        .query(params![PALE_DERIVED_OUTBOX_INDEX_ROLE, after_key, limit])
        .map_err(|source| sqlite("query Pale-derived outbox page", source))?;
    let mut entries = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|source| sqlite("read Pale-derived outbox page", source))?
    {
        let publication_key = EventIndexKey::new(
            ControlRecordDigest::parse(
                row.get::<_, String>(0)
                    .map_err(|source| sqlite("decode Pale-derived outbox key", source))?,
            )
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?,
        );
        let coordinate = decode_record_coordinate(
            row.get::<_, String>(1)
                .map_err(|source| sqlite("decode Pale-derived outbox reference", source))?,
            row.get::<_, String>(2)
                .map_err(|source| sqlite("decode Pale-derived outbox digest", source))?,
        )?;
        let record = read_record(connection, &coordinate)?
            .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        entries.push(PaleDerivedOutboxPageEntry::new(
            PaleDerivedOutboxCursor::new(publication_key),
            record,
        ));
    }
    let has_more = entries.len() > request.maximum_items();
    if has_more {
        entries.pop();
    }
    let boundary = if has_more {
        let resume_after = entries
            .last()
            .map(|entry| entry.cursor().clone())
            .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        PaleDerivedOutboxPageBoundary::More { resume_after }
    } else {
        PaleDerivedOutboxPageBoundary::Complete
    };
    PaleDerivedOutboxPage::new(request, entries, boundary)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
}

/// Admit one signed-portable journal sequence recovered from `SQLite`.
fn decode_sequence(value: i64) -> Result<JournalSequenceV1, ControlStoreError> {
    u64::try_from(value)
        .ok()
        .and_then(|value| JournalSequenceV1::new(value).ok())
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
}

/// Stable migration-constrained spelling of the Pale-derived outbox role.
const PALE_DERIVED_OUTBOX_INDEX_ROLE: &str = "pale_derived_outbox";

/// Stable migration-constrained spelling of one closed Event index role.
const fn query_role(query: &EventIndexQuery) -> &'static str {
    match query {
        EventIndexQuery::EventIdentity(_) => "event_identity",
        EventIndexQuery::EventCommitWitness(_) => "event_commit_witness",
        EventIndexQuery::TransitionWitness(_) => "transition_witness",
        EventIndexQuery::OwnerSource(_) => "owner_source",
        EventIndexQuery::IdentityConflict(_) => "identity_conflict",
        EventIndexQuery::Subscription(_) => "subscription",
        EventIndexQuery::InboxDelivery(_) => "inbox_delivery",
        EventIndexQuery::DeliveryAttempt(_) => "delivery_attempt",
        EventIndexQuery::DeliveryStream(_) => "delivery_stream",
        EventIndexQuery::Acknowledgement(_) => "acknowledgement",
        EventIndexQuery::DeliveryCursor(_) => "delivery_cursor",
        EventIndexQuery::SourceFrontier(_) => "source_frontier",
        EventIndexQuery::InteractionOccurrence(_) => "interaction_occurrence",
        EventIndexQuery::GenealogyEdge(_) => "genealogy_edge",
        EventIndexQuery::ReconciliationFrontier(_) => "reconciliation_frontier",
        EventIndexQuery::PaleDerivedOutbox(_) => PALE_DERIVED_OUTBOX_INDEX_ROLE,
    }
}

#[cfg(test)]
#[path = "../tests/internal/event.rs"]
mod tests;
