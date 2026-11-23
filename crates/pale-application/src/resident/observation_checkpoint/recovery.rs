//! Recover only the pending anchor of an independently admitted stored append.

use super::*;
use crate::resident::DatabaseCommittedOperationObservationAnchorPending;
use pale_core::records::ControlTransactionSequenceV1;
use pale_ports::OperationAccumulatorPreparationError;

/// Exact stored checkpoint pair and bounds for committed observation recovery.
///
/// The caller must admit both signed records and the observation transition
/// through their semantic owners before using this input. A successful result
/// still requires independent anchoring and grants no execution authority.
pub struct CommittedObservationCheckpointRecovery<'a> {
    /// Supported immutable checkpoint profile bound to the original request.
    pub selection: OperationCheckpointSelection,
    /// Immutable reference and admitted checkpoint immediately before the append.
    pub predecessor: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    /// Immutable reference and admitted checkpoint committed with the append.
    pub successor: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    /// Shared finite bounds for physical proof reconstruction.
    pub limits: AccumulatorPreparationLimits,
}

impl<'a> PreparedOperationObservationAppend<'a> {
    /// Reconstruct exact committed custody and return only its pending anchor.
    ///
    /// No signer, identity allocator, database write, or response delivery is
    /// invoked. The native port proves the logical transition backwards from
    /// the stored successor to the admitted predecessor, then exact read-back
    /// verifies all candidate records and indexes. Compound append owners must
    /// first restore their associated records; this method cannot infer them.
    ///
    /// # Errors
    /// Rejects changed heads or transactions, mismatched signed changes or
    /// roots, incomplete custody, and exhausted limits. Protected readers keep
    /// requiring exact anchor agreement throughout recovery.
    pub fn recover_committed_checkpoint(
        self,
        input: CommittedObservationCheckpointRecovery<'a>,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        DatabaseCommittedOperationObservationAnchorPending,
        OperationObservationCustodyProjectionError,
    > {
        self.recover_committed_checkpoint_with(input, custody, Ok)
    }

    /// Restore the compound owner's exact attachments before physical read-back.
    pub(in crate::resident) fn recover_committed_checkpoint_with(
        self,
        input: CommittedObservationCheckpointRecovery<'a>,
        custody: &mut dyn OperationCustodyPort,
        attach: impl FnOnce(
            PreparedOperationObservationCustody,
        ) -> Result<
            PreparedOperationObservationCustody,
            OperationObservationCustodyProjectionError,
        >,
    ) -> Result<
        DatabaseCommittedOperationObservationAnchorPending,
        OperationObservationCustodyProjectionError,
    > {
        let (prior_ref, prior) = input.predecessor;
        let (next_ref, next) = input.successor;
        next.verify_successor_of(prior_ref, prior)?;
        input
            .selection
            .verify(self.store(), self.handle().1, next)?;
        let expected = next
            .committed_transaction_sequence()
            .get()
            .checked_sub(1)
            .ok_or(OperationObservationCheckpointConstructionRejection)?;
        let transaction = ControlTransactionPreparation::new(
            self.store().clone(),
            ControlTransactionSequenceV1::new(expected)?,
        )?;
        let successor = OperationAccumulatorCheckpoint::new(
            self.store().clone(),
            next.checkpoint_profile_ref().clone(),
            next.checkpoint_profile_sha256().clone(),
            checkpoint_head(next_ref, next)?,
            next.state_root_sha256().clone(),
            next.unresolved_custody_root_sha256().clone(),
        );
        let prepared = self.prepare_checkpoint_with(
            input.selection,
            prior_ref,
            prior,
            input.limits,
            transaction,
            |request| custody.recover_committed_accumulator(request, &successor),
        )?;
        if next.changed_leaf_keys() != &prepared.changed_keys
            || *next.leaf_count() != prepared.leaf_count
            || *next.committed_transaction_sequence() != prepared.transaction_sequence()
            || next.state_root_sha256() != prepared.state_root()
            || next.unresolved_custody_root_sha256() != prepared.unresolved_root()
        {
            return Err(OperationObservationCheckpointConstructionRejection.into());
        }
        attach(prepared.attach_issued_checkpoint(next_ref, next)?)?
            .recover_committed(custody)
            .map_err(|error| OperationAccumulatorPreparationError::from(error).into())
    }
}
