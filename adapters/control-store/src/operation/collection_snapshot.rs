//! Bounded current or historical collection reads with the actual indexed head.

use super::*;

pub(super) fn read(
    custody: &mut SqliteOperationCustody<'_>,
    store: &AuthorityStoreCoordinate,
    handle: &ControlRecordCoordinate,
    requested_frontier: Option<&ControlRecordCoordinate>,
    maximum_frontier_bytes: usize,
    maximum_journal_payload_bytes: usize,
) -> Result<Option<pale_ports::OperationCollectionSnapshot>, ControlStoreError> {
    require_coordinate(&custody.coordinate, store)?;
    if maximum_frontier_bytes == 0 || maximum_journal_payload_bytes == 0 {
        return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
    }
    let transaction = custody
        .connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|source| sqlite("begin consultation collection point read", source))?;
    require_store_binding(&transaction, store)?;
    collection_journal::require_journal_head_consistency(
        &transaction,
        handle,
        maximum_journal_payload_bytes,
    )?;
    let key = OperationCustodyIndexKey::for_collection_head(handle);
    let Some(coordinate) = read_index_coordinate(&transaction, IndexRole::CollectionHead, &key)?
    else {
        transaction
            .rollback()
            .map_err(|source| sqlite("close absent consultation collection read", source))?;
        return Ok(None);
    };
    let frontier = read_record_bounded(
        &transaction,
        requested_frontier.unwrap_or(&coordinate),
        maximum_frontier_bytes,
    )?
    .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let evidence = pale_ports::OperationCollectionFrontierEvidence::admit(
        &frontier,
        store,
        handle,
        maximum_frontier_bytes,
    )
    .map_err(collection_read_error)?;
    let inserted = crate::store::transaction::record_stamp(
        &transaction,
        crate::ControlStoreKind::DeploymentControl,
        frontier.coordinate(),
    )?
    .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let current = crate::store::transaction::read(&transaction, store)?;
    if inserted != evidence.committed_transaction_sequence() || inserted.get() > current.get() {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    let payload = collection_journal::read_journal_entry(
        &transaction,
        handle,
        evidence.journal_sequence(),
        maximum_journal_payload_bytes,
    )?
    .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let head = pale_ports::OperationCollectionHeadObservation::for_handle(handle, coordinate);
    let snapshot = pale_ports::OperationCollectionSnapshot::new(frontier, &evidence, head, payload)
        .map_err(collection_read_error)?;
    transaction
        .rollback()
        .map_err(|source| sqlite("close consultation collection point read", source))?;
    Ok(Some(snapshot))
}
