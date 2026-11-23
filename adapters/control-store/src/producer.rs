//! `SQLite` custody for principal-wide producer-incarnation transitions.
//!
//! One immediate transaction appends already-admitted records and advances the
//! producer, signed principal-trust, and hard-domain checkpoint heads together.
//! The adapter compares opaque bytes and typed physical coordinates only; it
//! does not generate epochs, verify signatures, select leaf meaning, advance
//! external anchors, or authorize owner-outbox writes.

use pale_core::records::{
    ProducerEpochV1, ProducerIdV1, ProducerIncarnationHeadStateV1, ResidentRecordRefV1,
};
use pale_ports::{
    ControlRecordCoordinate, ControlRecordDigest, ControlStoreError, ControlStoreErrorKind,
    PreparedPrincipalProducerIncarnationTransition, PrincipalProducerIncarnationCommitDisposition,
    PrincipalProducerIncarnationCustodyPort, PrincipalProducerIncarnationDatabaseObservation,
    PrincipalProducerIncarnationHead, PrincipalProducerIncarnationHeadCursor,
    PrincipalProducerIncarnationHeadPage, PrincipalProducerIncarnationHeadPageBoundary,
    PrincipalProducerIncarnationHeadPageEntry, PrincipalProducerIncarnationHeadPageRead,
    PrincipalProducerIncarnationReadObservation, PrincipalProducerIncarnationReadPort,
    PrincipalProducerIncarnationRecordSet, PrincipalProducerIncarnationTransitionKind,
    PrincipalTrustSnapshot, PrincipalTrustStoreBinding,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::accumulator::{self, Store as AccumulatorStore};
use crate::error::sqlite;
use crate::principal_trust::{
    advance_control_checkpoint_head, advance_trust_checkpoint_head, binding_matches,
    control_checkpoint_head_matches, insert_record, read_record, record_matches,
    trust_checkpoint_head_matches,
};
use crate::ControlSqliteError;

/// `SQLite` realization of the producer-incarnation custody port.
///
/// Construction remains crate-private until external anti-rollback anchors and
/// resident admission exist. Tests exercise the production transaction through
/// an already initialized principal trust connection.
pub struct SqlitePrincipalProducerIncarnationCustody<'connection> {
    /// Open principal trust connection whose Pale Runtime assembly remains external.
    pub(crate) connection: &'connection mut Connection,
}

impl pale_ports::ControlTransactionPort for SqlitePrincipalProducerIncarnationCustody<'_> {
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

impl PrincipalProducerIncarnationCustodyPort for SqlitePrincipalProducerIncarnationCustody<'_> {
    fn prepare_accumulator(
        &mut self,
        request: &pale_ports::OperationAccumulatorPreparation,
    ) -> Result<
        pale_ports::PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        crate::principal_trust::checkpoint::prepare_successor(self.connection, request)
    }

    fn compare_and_commit(
        &mut self,
        candidate: &PreparedPrincipalProducerIncarnationTransition,
    ) -> Result<PrincipalProducerIncarnationCommitDisposition, ControlStoreError> {
        if let Some(batch) = candidate.accumulator() {
            accumulator::validate(batch)?;
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|source| sqlite("begin principal producer-incarnation transition", source))?;
        let store = crate::principal_trust::store_coordinate(candidate.binding());
        match observe(&transaction, candidate)? {
            PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor => {
                transaction
                    .rollback()
                    .map_err(|source| sqlite("close exact producer-incarnation replay", source))?;
                Ok(PrincipalProducerIncarnationCommitDisposition::ExactReplay)
            }
            PrincipalProducerIncarnationDatabaseObservation::Predecessor => {
                let prepared = crate::store::transaction::prepare(&transaction, &store)?;
                if candidate.next_checkpoint().committed_transaction_sequence()
                    != prepared.successor()
                    || candidate
                        .accumulator()
                        .is_some_and(|batch| batch.binding().transaction() != &prepared)
                {
                    return Err(ControlStoreError::new(
                        ControlStoreErrorKind::CandidateConflict,
                    ));
                }
                crate::store::transaction::advance(&transaction, &prepared)?;
                for record in candidate.records() {
                    insert_record(&transaction, candidate.binding(), record)?;
                }
                append_producer_transition(&transaction, candidate)?;
                advance_producer_head(&transaction, candidate)?;
                advance_trust_checkpoint_head(
                    &transaction,
                    candidate.expected_trust(),
                    candidate.next_trust(),
                )?;
                advance_control_checkpoint_head(
                    &transaction,
                    candidate.expected_checkpoint(),
                    candidate.next_checkpoint(),
                )?;
                if let Some(batch) = candidate.accumulator() {
                    accumulator::apply(&transaction, AccumulatorStore::PrincipalTrust, batch)?;
                }
                if observe(&transaction, candidate)?
                    != PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor
                {
                    return Err(ControlStoreError::new(
                        ControlStoreErrorKind::RecoveryRequired,
                    ));
                }
                transaction.commit().map_err(|source| {
                    ControlStoreError::with_source(
                        ControlStoreErrorKind::OutcomeUnknown,
                        ControlSqliteError::sqlite(
                            "commit principal producer-incarnation transition",
                            source,
                        ),
                    )
                })?;
                Ok(PrincipalProducerIncarnationCommitDisposition::Committed)
            }
            PrincipalProducerIncarnationDatabaseObservation::ConflictingOrPartial => {
                let selected_heads_match = candidate.accumulator().is_some()
                    && (successor_heads_match(&transaction, candidate)?
                        || predecessor_heads_match(&transaction, candidate)?);
                let kind = if !selected_heads_match
                    && conflicting_frontier_is_complete(&transaction, candidate)?
                {
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
        candidate: &PreparedPrincipalProducerIncarnationTransition,
    ) -> Result<PrincipalProducerIncarnationDatabaseObservation, ControlStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin producer-incarnation read-back", source))?;
        let observation = observe(&transaction, candidate);
        transaction
            .rollback()
            .map_err(|source| sqlite("close producer-incarnation read-back", source))?;
        observation
    }
}

impl PrincipalProducerIncarnationReadPort for SqlitePrincipalProducerIncarnationCustody<'_> {
    fn read_by_epoch(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        producer_epoch: &ProducerEpochV1,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        if !binding_matches(self.connection, binding)? {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        read_producer_by_epoch(self.connection, binding, producer_epoch)
    }

    fn read_head(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        producer_id: &ProducerIdV1,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        if !binding_matches(self.connection, binding)? {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        let epoch = self
            .connection
            .query_row(
                "SELECT producer_epoch FROM pale_principal_producer_incarnation_head \
                 WHERE origin_principal_id = ?1 AND producer_id = ?2",
                params![binding.principal_id().as_str(), producer_id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|source| sqlite("read producer-incarnation head epoch", source))?;
        let Some(epoch) = epoch else {
            return Ok(PrincipalProducerIncarnationReadObservation::Absent);
        };
        let epoch = ProducerEpochV1::parse(epoch)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        match read_producer_by_epoch(self.connection, binding, &epoch)? {
            PrincipalProducerIncarnationReadObservation::Exact(records)
                if records.head().producer_id() == producer_id
                    && producer_head_matches(self.connection, records.head())? =>
            {
                Ok(PrincipalProducerIncarnationReadObservation::Exact(records))
            }
            PrincipalProducerIncarnationReadObservation::Absent
            | PrincipalProducerIncarnationReadObservation::Exact(_) => {
                Ok(PrincipalProducerIncarnationReadObservation::ConflictingOrPartial)
            }
            partial @ PrincipalProducerIncarnationReadObservation::ConflictingOrPartial => {
                Ok(partial)
            }
        }
    }

    fn read_by_allocation(
        &mut self,
        snapshot: &PrincipalTrustSnapshot,
        allocation: &ControlRecordCoordinate,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        require_snapshot(self.connection, snapshot)?;
        let epoch = self
            .connection
            .query_row(
                "SELECT producer_epoch FROM pale_principal_producer_incarnation \
                 WHERE origin_principal_id = ?1 AND allocation_ref = ?2 \
                   AND allocation_sha256 = ?3",
                params![
                    snapshot.binding().principal_id().as_str(),
                    allocation.record_ref().as_str(),
                    allocation.record_sha256().as_str(),
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|source| sqlite("read producer allocation epoch", source))?;
        let observation = match epoch {
            Some(epoch) => {
                let epoch = ProducerEpochV1::parse(epoch)
                    .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
                match read_producer_by_epoch(self.connection, snapshot.binding(), &epoch)? {
                    PrincipalProducerIncarnationReadObservation::Exact(records)
                        if records.head().allocation() == allocation =>
                    {
                        PrincipalProducerIncarnationReadObservation::Exact(records)
                    }
                    PrincipalProducerIncarnationReadObservation::Absent
                    | PrincipalProducerIncarnationReadObservation::Exact(_) => {
                        PrincipalProducerIncarnationReadObservation::ConflictingOrPartial
                    }
                    partial @ PrincipalProducerIncarnationReadObservation::ConflictingOrPartial => {
                        partial
                    }
                }
            }
            None => PrincipalProducerIncarnationReadObservation::Absent,
        };
        require_snapshot(self.connection, snapshot)?;
        Ok(observation)
    }

    fn read_heads_page(
        &mut self,
        request: &PrincipalProducerIncarnationHeadPageRead,
    ) -> Result<PrincipalProducerIncarnationHeadPage, ControlStoreError> {
        require_snapshot(self.connection, request.snapshot())?;

        let limit = request
            .maximum_items()
            .checked_add(1)
            .and_then(|value| i64::try_from(value).ok())
            .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
        let after = request
            .after()
            .map_or("", |cursor| cursor.producer_id().as_str());
        let producer_ids = {
            let mut statement = self
                .connection
                .prepare(
                    "SELECT producer_id FROM pale_principal_producer_incarnation_head \
                     WHERE origin_principal_id = ?1 AND producer_id > ?2 \
                     ORDER BY producer_id ASC LIMIT ?3",
                )
                .map_err(|source| sqlite("prepare producer-head page", source))?;
            let rows = statement
                .query_map(
                    params![request.binding().principal_id().as_str(), after, limit],
                    |row| row.get::<_, String>(0),
                )
                .map_err(|source| sqlite("query producer-head page", source))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|source| sqlite("read producer-head page", source))?
        };
        let has_more = producer_ids.len() > request.maximum_items();
        let mut entries = Vec::with_capacity(producer_ids.len().min(request.maximum_items()));
        for producer_id in producer_ids.into_iter().take(request.maximum_items()) {
            let producer_id = ProducerIdV1::parse(producer_id)
                .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
            let observation = self.read_head(request.binding(), &producer_id)?;
            let PrincipalProducerIncarnationReadObservation::Exact(records) = observation else {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            };
            entries.push(
                PrincipalProducerIncarnationHeadPageEntry::new(
                    PrincipalProducerIncarnationHeadCursor::new(producer_id),
                    *records,
                )
                .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?,
            );
        }
        require_snapshot(self.connection, request.snapshot())?;
        let boundary = if has_more {
            let resume_after = entries
                .last()
                .map(|entry| entry.cursor().clone())
                .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
            PrincipalProducerIncarnationHeadPageBoundary::More { resume_after }
        } else {
            PrincipalProducerIncarnationHeadPageBoundary::Complete
        };
        PrincipalProducerIncarnationHeadPage::new(request, entries, boundary)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
    }
}

/// Reconstruct one allocation and optional retirement by the epoch's unique key.
fn read_producer_by_epoch(
    connection: &Connection,
    binding: &PrincipalTrustStoreBinding,
    producer_epoch: &ProducerEpochV1,
) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
    let row = connection
        .query_row(
            "SELECT allocation.producer_id, allocation.allocation_ref, \
                    allocation.allocation_sha256, allocation.previous_allocation_ref, \
                    allocation.previous_allocation_sha256, retirement.retirement_ref, \
                    retirement.retirement_sha256, retirement.origin_principal_id, \
                    retirement.producer_id, retirement.producer_epoch \
             FROM pale_principal_producer_incarnation AS allocation \
             LEFT JOIN pale_principal_producer_incarnation_retirement AS retirement \
               ON retirement.allocation_ref = allocation.allocation_ref \
              AND retirement.allocation_sha256 = allocation.allocation_sha256 \
             WHERE allocation.origin_principal_id = ?1 AND allocation.producer_epoch = ?2",
            params![binding.principal_id().as_str(), producer_epoch.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                ))
            },
        )
        .optional()
        .map_err(|source| sqlite("read producer incarnation by epoch", source))?;
    let Some((
        producer_id,
        allocation_ref,
        allocation_sha256,
        previous_allocation_ref,
        previous_allocation_sha256,
        retirement_ref,
        retirement_sha256,
        retirement_principal_id,
        retirement_producer_id,
        retirement_producer_epoch,
    )) = row
    else {
        return Ok(PrincipalProducerIncarnationReadObservation::Absent);
    };
    let retirement_fields = [
        retirement_ref.is_some(),
        retirement_sha256.is_some(),
        retirement_principal_id.is_some(),
        retirement_producer_id.is_some(),
        retirement_producer_epoch.is_some(),
    ];
    if retirement_fields
        .iter()
        .any(|present| *present != retirement_fields[0])
        || retirement_principal_id
            .as_deref()
            .is_some_and(|principal| principal != binding.principal_id().as_str())
        || retirement_producer_id
            .as_deref()
            .is_some_and(|retired_producer| retired_producer != producer_id.as_str())
        || retirement_producer_epoch
            .as_deref()
            .is_some_and(|retired_epoch| retired_epoch != producer_epoch.as_str())
    {
        return Ok(PrincipalProducerIncarnationReadObservation::ConflictingOrPartial);
    }
    let producer_id = ProducerIdV1::parse(producer_id)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let allocation = coordinate(allocation_ref, allocation_sha256)?;
    let previous_fields = [
        previous_allocation_ref.is_some(),
        previous_allocation_sha256.is_some(),
    ];
    if previous_fields[0] != previous_fields[1] {
        return Ok(PrincipalProducerIncarnationReadObservation::ConflictingOrPartial);
    }
    let previous_allocation = previous_allocation_ref
        .zip(previous_allocation_sha256)
        .map(|(reference, digest)| coordinate(reference, digest))
        .transpose()?;
    let retirement = retirement_ref
        .zip(retirement_sha256)
        .map(|(reference, digest)| coordinate(reference, digest))
        .transpose()?;
    let state = if retirement.is_some() {
        ProducerIncarnationHeadStateV1::Retired
    } else {
        ProducerIncarnationHeadStateV1::Active
    };
    let head = PrincipalProducerIncarnationHead::new(
        binding.principal_id().clone(),
        producer_id,
        producer_epoch.clone(),
        state,
        allocation.clone(),
        retirement.clone(),
    )
    .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    if current_head_names_epoch(connection, &head)? && !producer_head_matches(connection, &head)? {
        return Ok(PrincipalProducerIncarnationReadObservation::ConflictingOrPartial);
    }
    let Some(allocation_record) = read_record(connection, binding, &allocation)? else {
        return Ok(PrincipalProducerIncarnationReadObservation::ConflictingOrPartial);
    };
    let retirement_record = match retirement {
        Some(ref coordinate) => match read_record(connection, binding, coordinate)? {
            Some(record) => Some(record),
            None => return Ok(PrincipalProducerIncarnationReadObservation::ConflictingOrPartial),
        },
        None => None,
    };
    PrincipalProducerIncarnationRecordSet::new(
        head,
        previous_allocation,
        allocation_record,
        retirement_record,
    )
    .map(Box::new)
    .map(PrincipalProducerIncarnationReadObservation::Exact)
    .or(Ok(
        PrincipalProducerIncarnationReadObservation::ConflictingOrPartial,
    ))
}

/// Require one unchanged trust-store/checkpoint observation coordinate.
fn require_snapshot(
    connection: &Connection,
    snapshot: &PrincipalTrustSnapshot,
) -> Result<(), ControlStoreError> {
    if binding_matches(connection, snapshot.binding())?
        && trust_checkpoint_head_matches(connection, snapshot.through())?
        && record_matches(connection, snapshot.binding(), snapshot.checkpoint_record())?
    {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ))
    }
}

/// Test whether the mutable producer head currently names the selected epoch.
fn current_head_names_epoch(
    connection: &Connection,
    head: &PrincipalProducerIncarnationHead,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_producer_incarnation_head \
             WHERE origin_principal_id = ?1 AND producer_id = ?2 AND producer_epoch = ?3)",
            params![
                head.origin_principal_id().as_str(),
                head.producer_id().as_str(),
                head.producer_epoch().as_str(),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare producer head epoch", source))
}

/// Parse one immutable record coordinate from constrained `SQLite` text.
fn coordinate(
    reference: String,
    digest: String,
) -> Result<ControlRecordCoordinate, ControlStoreError> {
    let reference = ResidentRecordRefV1::parse(reference)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let digest = ControlRecordDigest::parse(digest)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    Ok(ControlRecordCoordinate::new(reference, digest))
}

/// Classify one database snapshot, including its store binding and transaction head.
/// A concurrent writer may advance after this snapshot; this is not authority
/// to retry or continue the operation.
fn observe(
    connection: &Transaction<'_>,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<PrincipalProducerIncarnationDatabaseObservation, ControlStoreError> {
    if let Some(batch) = candidate.accumulator() {
        accumulator::validate(batch)?;
    } else {
        // A caller cannot advance an existing tree by dropping its batch.
        let has_tree: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_control_accumulator_frontier WHERE checkpoint_domain_id=?1) \
             OR EXISTS(SELECT 1 FROM pale_principal_control_accumulator_position WHERE checkpoint_domain_id=?1)",
            [candidate.expected_checkpoint().checkpoint_domain_id().as_str()],
            |row| row.get(0),
        ).map_err(|source| sqlite("compare producer tree custody presence", source))?;
        if has_tree {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
    }
    if !binding_matches(connection, candidate.binding())? {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    crate::store::transaction::read(
        connection,
        &crate::principal_trust::store_coordinate(candidate.binding()),
    )?;
    if successor_matches(connection, candidate)? {
        return Ok(PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor);
    }
    if predecessor_matches(connection, candidate)? {
        return Ok(PrincipalProducerIncarnationDatabaseObservation::Predecessor);
    }
    Ok(PrincipalProducerIncarnationDatabaseObservation::ConflictingOrPartial)
}

/// Require all successor heads and every predecessor and appended byte string.
fn successor_matches(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<bool, ControlStoreError> {
    if let Some(batch) = candidate.accumulator() {
        if !accumulator::matches(connection, AccumulatorStore::PrincipalTrust, batch, true)? {
            return Ok(false);
        }
    }
    if !successor_heads_match(connection, candidate)?
        || !predecessor_records_match(connection, candidate)?
    {
        return Ok(false);
    }
    for record in candidate.records() {
        if !record_matches(connection, candidate.binding(), record)? {
            return Ok(false);
        }
    }
    if !crate::store::transaction::records_were_inserted_in(
        connection,
        crate::ControlStoreKind::PrincipalTrust,
        &crate::principal_trust::store_coordinate(candidate.binding()),
        candidate.next_checkpoint().committed_transaction_sequence(),
        candidate.records(),
    )? {
        return Ok(false);
    }
    transition_rows_match(connection, candidate)
}

/// Require all predecessor heads and the absence of every proposed append.
fn predecessor_matches(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<bool, ControlStoreError> {
    if let Some(batch) = candidate.accumulator() {
        if !accumulator::matches(connection, AccumulatorStore::PrincipalTrust, batch, false)? {
            return Ok(false);
        }
    }
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

/// Compare the exact predecessor heads and all immutable predecessor bytes.
fn predecessor_frontier_matches(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<bool, ControlStoreError> {
    Ok(predecessor_heads_match(connection, candidate)?
        && predecessor_records_match(connection, candidate)?)
}

/// Compare predecessor head coordinates without assuming immutable bytes match.
fn predecessor_heads_match(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<bool, ControlStoreError> {
    if !trust_checkpoint_head_matches(connection, candidate.expected_trust())?
        || !control_checkpoint_head_matches(connection, candidate.expected_checkpoint())?
    {
        return Ok(false);
    }
    match candidate.expected_producer() {
        Some(head) => producer_head_matches(connection, head),
        None => Ok(producer_head_count(connection, candidate.next_producer())? == 0),
    }
}

/// Compare every immutable predecessor byte carried by the candidate.
fn predecessor_records_match(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<bool, ControlStoreError> {
    if !record_matches(
        connection,
        candidate.binding(),
        candidate.expected_trust_record(),
    )? || !record_matches(
        connection,
        candidate.binding(),
        candidate.expected_checkpoint_record(),
    )? {
        return Ok(false);
    }
    for record in candidate.expected_producer_records() {
        if !record_matches(connection, candidate.binding(), record)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Compare all three intended successor heads.
fn successor_heads_match(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<bool, ControlStoreError> {
    Ok(
        producer_head_matches(connection, candidate.next_producer())?
            && trust_checkpoint_head_matches(connection, candidate.next_trust())?
            && control_checkpoint_head_matches(connection, candidate.next_checkpoint())?,
    )
}

/// Distinguish a stale complete frontier from partial or contradictory state.
fn conflicting_frontier_is_complete(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<bool, ControlStoreError> {
    if predecessor_heads_match(connection, candidate)?
        || successor_heads_match(connection, candidate)?
        || !trust_head_is_complete(connection)?
        || !checkpoint_domain_head_is_complete(
            connection,
            candidate
                .expected_checkpoint()
                .checkpoint_domain_id()
                .as_str(),
        )?
    {
        return Ok(false);
    }
    let producer_count = producer_head_count(connection, candidate.next_producer())?;
    match candidate.expected_producer() {
        None => Ok(producer_count == 0
            || (producer_count == 1
                && producer_head_key_is_complete(connection, candidate.next_producer())?)),
        Some(_) => Ok(producer_count == 1
            && producer_head_key_is_complete(connection, candidate.next_producer())?),
    }
}

/// Append allocation or retirement metadata after immutable record insertion.
fn append_producer_transition(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<(), ControlStoreError> {
    match candidate.kind() {
        PrincipalProducerIncarnationTransitionKind::InitialAllocation
        | PrincipalProducerIncarnationTransitionKind::SuccessorAllocation => {
            insert_allocation(connection, candidate)
        }
        PrincipalProducerIncarnationTransitionKind::Retirement => {
            insert_retirement(connection, candidate)
        }
    }
}

/// Insert one immutable allocation index row with its exact predecessor.
fn insert_allocation(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<(), ControlStoreError> {
    let next = candidate.next_producer();
    if let Some((record_ref, record_sha256)) = connection
        .query_row(
            "SELECT allocation_ref, allocation_sha256 \
             FROM pale_principal_producer_incarnation \
             WHERE origin_principal_id = ?1 AND producer_epoch = ?2",
            params![
                next.origin_principal_id().as_str(),
                next.producer_epoch().as_str()
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|source| sqlite("check producer-epoch uniqueness", source))?
    {
        let kind = if record_ref == next.allocation().record_ref().as_str()
            && record_sha256 == next.allocation().record_sha256().as_str()
        {
            ControlStoreErrorKind::RecoveryRequired
        } else {
            ControlStoreErrorKind::CandidateConflict
        };
        return Err(ControlStoreError::new(kind));
    }
    let previous = candidate
        .expected_producer()
        .map(PrincipalProducerIncarnationHead::allocation);
    connection
        .execute(
            "INSERT INTO pale_principal_producer_incarnation \
             (allocation_ref, allocation_sha256, origin_principal_id, producer_id, \
              producer_epoch, previous_allocation_ref, previous_allocation_sha256) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                next.allocation().record_ref().as_str(),
                next.allocation().record_sha256().as_str(),
                next.origin_principal_id().as_str(),
                next.producer_id().as_str(),
                next.producer_epoch().as_str(),
                previous.map(|value| value.record_ref().as_str()),
                previous.map(|value| value.record_sha256().as_str()),
            ],
        )
        .map(drop)
        .map_err(|source| sqlite("insert producer-incarnation allocation", source))
}

/// Insert the append-only retirement for the exact current allocation.
fn insert_retirement(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<(), ControlStoreError> {
    let next = candidate.next_producer();
    let retirement = next
        .retirement()
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    connection
        .execute(
            "INSERT INTO pale_principal_producer_incarnation_retirement \
             (retirement_ref, retirement_sha256, origin_principal_id, producer_id, \
              producer_epoch, allocation_ref, allocation_sha256) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                retirement.record_ref().as_str(),
                retirement.record_sha256().as_str(),
                next.origin_principal_id().as_str(),
                next.producer_id().as_str(),
                next.producer_epoch().as_str(),
                next.allocation().record_ref().as_str(),
                next.allocation().record_sha256().as_str(),
            ],
        )
        .map(drop)
        .map_err(|source| sqlite("insert producer-incarnation retirement", source))
}

/// Insert generation one or compare-and-swap the exact producer predecessor.
fn advance_producer_head(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<(), ControlStoreError> {
    match candidate.expected_producer() {
        None => insert_producer_head(connection, candidate.next_producer()),
        Some(expected) => update_producer_head(connection, expected, candidate.next_producer()),
    }
}

/// Insert one previously absent producer-key head.
fn insert_producer_head(
    connection: &Connection,
    head: &PrincipalProducerIncarnationHead,
) -> Result<(), ControlStoreError> {
    let values = producer_head_values(head);
    connection
        .execute(
            "INSERT INTO pale_principal_producer_incarnation_head \
             (origin_principal_id, producer_id, producer_epoch, producer_state, \
              allocation_ref, allocation_sha256, retirement_ref, retirement_sha256) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                values.principal_id,
                values.producer_id,
                values.producer_epoch,
                values.state,
                values.allocation_ref,
                values.allocation_sha256,
                values.retirement_ref,
                values.retirement_sha256,
            ],
        )
        .map(drop)
        .map_err(|source| sqlite("insert producer-incarnation head", source))
}

/// Compare-and-swap one exact producer-key head.
fn update_producer_head(
    connection: &Connection,
    expected: &PrincipalProducerIncarnationHead,
    next: &PrincipalProducerIncarnationHead,
) -> Result<(), ControlStoreError> {
    let expected = producer_head_values(expected);
    let next = producer_head_values(next);
    let changed = connection
        .execute(
            "UPDATE pale_principal_producer_incarnation_head SET \
             producer_epoch = ?1, producer_state = ?2, allocation_ref = ?3, \
             allocation_sha256 = ?4, retirement_ref = ?5, retirement_sha256 = ?6 \
             WHERE origin_principal_id = ?7 AND producer_id = ?8 AND producer_epoch = ?9 \
               AND producer_state = ?10 AND allocation_ref = ?11 AND allocation_sha256 = ?12 \
               AND retirement_ref IS ?13 AND retirement_sha256 IS ?14",
            params![
                next.producer_epoch,
                next.state,
                next.allocation_ref,
                next.allocation_sha256,
                next.retirement_ref,
                next.retirement_sha256,
                expected.principal_id,
                expected.producer_id,
                expected.producer_epoch,
                expected.state,
                expected.allocation_ref,
                expected.allocation_sha256,
                expected.retirement_ref,
                expected.retirement_sha256,
            ],
        )
        .map_err(|source| sqlite("advance producer-incarnation head", source))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))
    }
}

/// Compare one exact producer head.
fn producer_head_matches(
    connection: &Connection,
    head: &PrincipalProducerIncarnationHead,
) -> Result<bool, ControlStoreError> {
    let values = producer_head_values(head);
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_producer_incarnation_head \
             WHERE origin_principal_id = ?1 AND producer_id = ?2 AND producer_epoch = ?3 \
               AND producer_state = ?4 AND allocation_ref = ?5 AND allocation_sha256 = ?6 \
               AND retirement_ref IS ?7 AND retirement_sha256 IS ?8)",
            params![
                values.principal_id,
                values.producer_id,
                values.producer_epoch,
                values.state,
                values.allocation_ref,
                values.allocation_sha256,
                values.retirement_ref,
                values.retirement_sha256,
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare producer-incarnation head", source))
}

/// Compare allocation/retirement index rows introduced by the candidate.
fn transition_rows_match(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<bool, ControlStoreError> {
    match candidate.kind() {
        PrincipalProducerIncarnationTransitionKind::InitialAllocation
        | PrincipalProducerIncarnationTransitionKind::SuccessorAllocation => {
            allocation_row_matches(connection, candidate)
        }
        PrincipalProducerIncarnationTransitionKind::Retirement => {
            retirement_row_matches(connection, candidate)
        }
    }
}

/// Compare the immutable allocation index row for this transition.
fn allocation_row_matches(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<bool, ControlStoreError> {
    let next = candidate.next_producer();
    let previous = candidate
        .expected_producer()
        .map(PrincipalProducerIncarnationHead::allocation);
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_producer_incarnation \
             WHERE allocation_ref = ?1 AND allocation_sha256 = ?2 \
               AND origin_principal_id = ?3 AND producer_id = ?4 AND producer_epoch = ?5 \
               AND previous_allocation_ref IS ?6 AND previous_allocation_sha256 IS ?7)",
            params![
                next.allocation().record_ref().as_str(),
                next.allocation().record_sha256().as_str(),
                next.origin_principal_id().as_str(),
                next.producer_id().as_str(),
                next.producer_epoch().as_str(),
                previous.map(|value| value.record_ref().as_str()),
                previous.map(|value| value.record_sha256().as_str()),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare producer-incarnation allocation", source))
}

/// Compare the immutable retirement index row for this transition.
fn retirement_row_matches(
    connection: &Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Result<bool, ControlStoreError> {
    let next = candidate.next_producer();
    let retirement = next
        .retirement()
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_producer_incarnation_retirement \
             WHERE retirement_ref = ?1 AND retirement_sha256 = ?2 \
               AND origin_principal_id = ?3 AND producer_id = ?4 AND producer_epoch = ?5 \
               AND allocation_ref = ?6 AND allocation_sha256 = ?7)",
            params![
                retirement.record_ref().as_str(),
                retirement.record_sha256().as_str(),
                next.origin_principal_id().as_str(),
                next.producer_id().as_str(),
                next.producer_epoch().as_str(),
                next.allocation().record_ref().as_str(),
                next.allocation().record_sha256().as_str(),
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare producer-incarnation retirement", source))
}

/// Borrow stable SQL values for one typed producer head.
fn producer_head_values(head: &PrincipalProducerIncarnationHead) -> ProducerHeadValues<'_> {
    ProducerHeadValues {
        principal_id: head.origin_principal_id().as_str(),
        producer_id: head.producer_id().as_str(),
        producer_epoch: head.producer_epoch().as_str(),
        state: producer_state(head.state()),
        allocation_ref: head.allocation().record_ref().as_str(),
        allocation_sha256: head.allocation().record_sha256().as_str(),
        retirement_ref: head.retirement().map(|value| value.record_ref().as_str()),
        retirement_sha256: head
            .retirement()
            .map(|value| value.record_sha256().as_str()),
    }
}

/// Borrowed SQL parameter projection for one producer head.
struct ProducerHeadValues<'head> {
    principal_id: &'head str,
    producer_id: &'head str,
    producer_epoch: &'head str,
    state: &'static str,
    allocation_ref: &'head str,
    allocation_sha256: &'head str,
    retirement_ref: Option<&'head str>,
    retirement_sha256: Option<&'head str>,
}

/// Stable adapter-private spelling for the closed producer head state.
const fn producer_state(state: ProducerIncarnationHeadStateV1) -> &'static str {
    match state {
        ProducerIncarnationHeadStateV1::Active => "active",
        ProducerIncarnationHeadStateV1::Retired => "retired",
    }
}

/// Count the selected producer-key head without inspecting another producer.
fn producer_head_count(
    connection: &Connection,
    head: &PrincipalProducerIncarnationHead,
) -> Result<i64, ControlStoreError> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM pale_principal_producer_incarnation_head \
             WHERE origin_principal_id = ?1 AND producer_id = ?2",
            params![
                head.origin_principal_id().as_str(),
                head.producer_id().as_str()
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("count producer-incarnation head", source))
}

/// Prove the selected producer-key row closes over its immutable records.
fn producer_head_key_is_complete(
    connection: &Connection,
    head: &PrincipalProducerIncarnationHead,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 \
             FROM pale_principal_producer_incarnation_head AS head \
             JOIN pale_principal_producer_incarnation AS allocation \
               ON allocation.allocation_ref = head.allocation_ref \
              AND allocation.allocation_sha256 = head.allocation_sha256 \
             LEFT JOIN pale_principal_producer_incarnation_retirement AS retirement \
               ON retirement.retirement_ref = head.retirement_ref \
              AND retirement.retirement_sha256 = head.retirement_sha256 \
             WHERE head.origin_principal_id = ?1 AND head.producer_id = ?2 \
               AND ((head.producer_state = 'active' AND head.retirement_ref IS NULL \
                     AND retirement.retirement_ref IS NULL) \
                    OR (head.producer_state = 'retired' AND retirement.retirement_ref IS NOT NULL)))",
            params![
                head.origin_principal_id().as_str(),
                head.producer_id().as_str()
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("verify producer-incarnation head closure", source))
}

/// Prove the singleton trust head closes over an immutable record.
fn trust_head_is_complete(connection: &Connection) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_principal_trust_checkpoint_head AS head \
             JOIN pale_principal_trust_record AS record \
               ON record.record_ref = head.trust_checkpoint_ref \
              AND record.record_sha256 = head.trust_checkpoint_sha256 \
             WHERE head.singleton = 1)",
            [],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("verify principal trust-checkpoint head closure", source))
}

/// Prove one selected hard-domain head closes over an immutable record.
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
        .map_err(|source| sqlite("verify principal control-checkpoint head closure", source))
}

/// Test whether any immutable record already occupies a proposed reference.
fn record_ref_exists(connection: &Connection, record_ref: &str) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT 1 FROM pale_principal_trust_record WHERE record_ref = ?1",
            [record_ref],
            |_| Ok(true),
        )
        .optional()
        .map(Option::unwrap_or_default)
        .map_err(|source| sqlite("test producer transition record reference", source))
}

#[cfg(test)]
#[path = "../tests/internal/producer.rs"]
mod tests;
