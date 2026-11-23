//! Restore the exact first collection attachment before repairing its anchor.

use super::*;
use crate::resident::{
    CommittedObservationCheckpointRecovery, DatabaseCommittedOperationObservationAnchorPending,
};
use pale_ports::ControlTransactionPreparation;

/// Reconstruct committed opening custody after independent historical admission.
/// This proves the first journal entry and Resolving-to-Collecting transition;
/// later contributions, stops and new issuance are outside this recovery path.
pub(in crate::resident) fn recover_committed_initial_collection<'a>(
    opened: &RecoveredConsultationCollection,
    transition: OperationObservationTransition<'a>,
    checkpoints: CommittedObservationCheckpointRecovery<'a>,
    limits: ConsultationCollectionLimits,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
) -> Result<DatabaseCommittedOperationObservationAnchorPending, ConsultationCollectionError> {
    let append = prepare_operation_observation_append(transition, resolve_store)?;
    let (handle_ref, handle) = append.handle();
    let collection = opened.collection();
    collection.validate_recovered(handle_ref, handle, limits)?;
    if opened.store() != append.store()
        || collection.predecessor_frontier.is_some()
        || collection.frozen_stop_frontier.is_some()
        || collection.stop.is_some()
        || !collection.disagreements.is_empty()
        || !collection.late_outcomes.is_empty()
        || collection.opened_at != collection.updated_at
        || append.successor().1.observed_at() < &collection.updated_at
        || append.successor().1.observed_at() > handle.expires_at()
        || append.successor().1.observed_at() < checkpoints.predecessor.1.issued_at()
        || collection.slots.iter().any(|slot| {
            matches!(
                slot.roster.source,
                ConsultationContributionSource::FreshChild { .. }
            ) && slot.disposition != ConsultationSlotDisposition::Outstanding
        })
    {
        return Err(ConsultationCollectionError::Association);
    }
    let frontier = immutable_frontier_reference(opened.snapshot().frontier())?;
    if !matches!(
        append.predecessor().1.state(),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving
        }
    ) || !matches!(append.successor().1.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::CollectingConsultation { evidence }
            } if evidence == &frontier)
    {
        return Err(ConsultationCollectionError::Association);
    }
    let snapshot = opened.snapshot();
    let committed = checkpoints.successor.1.committed_transaction_sequence();
    if snapshot.evidence().journal_sequence() != JournalSequenceV1::new(1)?
        || snapshot.evidence().committed_transaction_sequence() != *committed
    {
        return Err(ConsultationCollectionError::Association);
    }
    let expected_transaction = committed
        .get()
        .checked_sub(1)
        .ok_or(ConsultationCollectionError::Association)?;
    let journal = OperationCollectionJournal::prepare(
        snapshot.evidence().operation_handle(),
        ControlTransactionPreparation::new(
            append.store().clone(),
            ControlTransactionSequenceV1::new(expected_transaction)?,
        )?,
        PreparedAuthorityAppend::new(
            append.store().clone(),
            None,
            SequencedAuthorityEntry::new(
                JournalSequenceV1::new(1)?,
                collection.canonical_bytes(limits)?,
            ),
        )?,
        limits.maximum_frontier_bytes,
        limits.maximum_journal_bytes,
    )?;
    if journal.frontier() != snapshot.frontier() {
        return Err(ConsultationCollectionError::Association);
    }
    append
        .recover_committed_checkpoint_with(checkpoints, custody, |candidate| {
            candidate.with_collection_journal(journal)
        })
        .map_err(ConsultationCollectionError::from)
}
