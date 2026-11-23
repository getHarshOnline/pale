//! Restore a committed expiry stop without selecting policy or signing again.

use super::*;
use crate::resident::{
    CommittedObservationCheckpointRecovery, DatabaseCommittedOperationObservationAnchorPending,
};
use pale_ports::ControlTransactionPreparation;

/// Admit a retained collection under its historical policy, participants and evidence.
/// This must not renew current permission or substitute today's roster policy.
pub type HistoricalConsultationAdmission<'a> =
    dyn FnMut(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1> + 'a;

impl ConsultationCollection {
    /// Read the exact collection named by an independently admitted observation.
    /// Historical policy admission is explicit; current-generation approval must
    /// not substitute for it. This reader grants no continuation and accepts no
    /// later contribution frontier in place of the observation's first stop.
    ///
    /// # Errors
    /// Rejects unavailable or changed custody, exceeded bounds, another parent,
    /// incompatible observation state and refused historical policy admission.
    pub fn recover_historical<Admit>(
        parent: (&ResidentRecordRefV1, &OperationHandleV1),
        observation: &OperationObservationV1,
        limits: ConsultationCollectionLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        admit: Admit,
    ) -> Result<RecoveredConsultationCollection, ConsultationCollectionError>
    where
        Admit: FnOnce(&Self) -> Result<(), InvalidResidentRecordV1>,
    {
        let (handle_ref, handle) = parent;
        if observation.operation_handle_ref() != handle_ref
            || observation.operation_handle_sha256() != handle.canonical_operation_handle_sha256()
        {
            return Err(ConsultationCollectionError::Association);
        }
        let (reference, stopped) = match observation.state() {
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::CollectingConsultation { evidence },
            } => (evidence, false),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ConsultationCollected { evidence },
            } => (evidence, true),
            _ => return Err(ConsultationCollectionError::Association),
        };
        let store = resolve_store.resolve(observation)?;
        let coordinate = ControlRecordCoordinate::new(
            reference.record_ref.clone(),
            ControlRecordDigest::parse(reference.record_sha256.as_str())?,
        );
        let handle_coordinate = ControlRecordCoordinate::new(
            handle_ref.clone(),
            ControlRecordDigest::parse(handle.canonical_operation_handle_sha256().as_str())?,
        );
        let snapshot = custody
            .read_collection_snapshot_at(
                &store,
                &handle_coordinate,
                &coordinate,
                limits.maximum_frontier_bytes,
                limits.maximum_journal_bytes,
            )?
            .ok_or(ConsultationCollectionError::Association)?;
        let physical = pale_ports::OperationCollectionFrontierEvidence::admit(
            snapshot.frontier(),
            &store,
            &handle_coordinate,
            limits.maximum_frontier_bytes,
        )?;
        if snapshot.frontier().coordinate() != &coordinate
            || snapshot.evidence() != &physical
            || snapshot.journal_payload().as_slice().len() > limits.maximum_journal_bytes
        {
            return Err(ConsultationCollectionError::Association);
        }
        let bytes = snapshot.journal_payload().as_slice();
        let collection = Self::deserialize(canon::parse_strict(bytes)?)
            .map_err(|_| ConsultationCollectionError::Decoding)?;
        if canon::to_canonical_string(&collection)?.as_bytes() != bytes {
            return Err(ConsultationCollectionError::Decoding);
        }
        collection.validate_recovered(handle_ref, handle, limits)?;
        if stopped != collection.stop.is_some() || collection.frozen_stop_frontier.is_some() {
            return Err(ConsultationCollectionError::Association);
        }
        admit(&collection)?;
        Ok(RecoveredConsultationCollection {
            collection,
            snapshot,
            store,
        })
    }
}

/// Recover the pending anchor of one already committed consultation expiry stop.
///
/// The caller must independently admit the parent, observation transition,
/// checkpoint pair and both historical collections. This owner reproduces the
/// frozen stop from its predecessor, restores the exact journal attachment and
/// verifies physical custody. It selects no policy, accepts no parent result,
/// releases no child and writes no database records. Later contributions remain
/// outside this first-stop recovery path.
///
/// # Errors
/// Rejects unlike stores, parents, frontiers, chronology or journal sequences;
/// changed coverage or collection content; incomplete custody; and exceeded
/// collection or checkpoint proof bounds.
pub fn recover_committed_consultation_expiry<'a>(
    previous: &RecoveredConsultationCollection,
    stopped: &RecoveredConsultationCollection,
    transition: OperationObservationTransition<'a>,
    checkpoints: CommittedObservationCheckpointRecovery<'a>,
    limits: ConsultationCollectionLimits,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
) -> Result<DatabaseCommittedOperationObservationAnchorPending, ConsultationCollectionError> {
    let append = prepare_operation_observation_append(transition, resolve_store)?;
    let (handle_ref, handle) = append.handle();
    let before = previous.collection();
    let after = stopped.collection();
    before.validate_recovered(handle_ref, handle, limits)?;
    after.validate_recovered(handle_ref, handle, limits)?;
    let stop = after
        .stopped()
        .ok_or(ConsultationCollectionError::Stopped)?;
    if previous.store() != append.store()
        || stopped.store() != append.store()
        || stop.cause != ConsultationStopCause::OperationExpiry
        || append.predecessor().1.observed_at() < &before.updated_at
        || append.successor().1.observed_at() < &after.updated_at
        || append.successor().1.observed_at() >= handle.retain_until()
        || append.successor().1.observed_at() < checkpoints.predecessor.1.issued_at()
    {
        return Err(ConsultationCollectionError::Association);
    }
    // The historical owner has already admitted the stop. Reproducing it here
    // checks its complete meaning without making another policy decision.
    let expected = previous.stop(
        ConsultationStopCause::OperationExpiry,
        stop.reason.clone(),
        stop.stopped_at,
        limits,
        |_, _| Ok(()),
    )?;
    if &expected != after {
        return Err(ConsultationCollectionError::Association);
    }
    let prior_frontier = immutable_frontier_reference(previous.snapshot().frontier())?;
    let next_frontier = immutable_frontier_reference(stopped.snapshot().frontier())?;
    if !matches!(append.predecessor().1.state(),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::CollectingConsultation { evidence }
        } if evidence == &prior_frontier)
        || !matches!(append.successor().1.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ConsultationCollected { evidence }
            } if evidence == &next_frontier)
    {
        return Err(ConsultationCollectionError::Association);
    }
    let snapshot = stopped.snapshot();
    let prior_sequence = previous.snapshot().evidence().journal_sequence();
    let sequence = snapshot.evidence().journal_sequence();
    let committed = checkpoints.successor.1.committed_transaction_sequence();
    if prior_sequence.checked_successor() != Some(sequence)
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
            Some(prior_sequence),
            SequencedAuthorityEntry::new(sequence, after.canonical_bytes(limits)?),
        )?,
        limits.maximum_frontier_bytes,
        limits.maximum_journal_bytes,
    )?;
    if journal.frontier() != snapshot.frontier() {
        return Err(ConsultationCollectionError::Association);
    }
    append
        .recover_committed_checkpoint_with(checkpoints, custody, |candidate| {
            candidate.with_collection_journal_after(
                journal,
                Some(previous.snapshot().frontier().clone()),
            )
        })
        .map_err(ConsultationCollectionError::from)
}
