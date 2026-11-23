//! Reconstruct a committed transition without relaxing protected checkpoint reads.

use super::*;
use pale_ports::{
    AccumulatorPreparationBasis, OperationAccumulatorCheckpoint, OperationAccumulatorPreparation,
    OperationAccumulatorPreparationError, PreparedOperationAccumulator,
};

impl SqliteOperationCustody<'_> {
    pub(super) fn recover_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
        successor: &OperationAccumulatorCheckpoint,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        match request.implementation() {
            pale_ports::AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1 => {}
        }
        require_coordinate(&self.coordinate, request.basis().store())?;
        let AccumulatorPreparationBasis::Successor(prior) = request.basis() else {
            return Err(ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired).into());
        };
        if successor.store() != prior.store()
            || successor.profile_ref() != prior.profile_ref()
            || successor.profile_sha256() != prior.profile_sha256()
            || !successor
                .checkpoint()
                .is_exact_successor_of(prior.checkpoint())
            || successor.checkpoint().committed_transaction_sequence()
                != request.transaction().successor()
        {
            return Err(ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired).into());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|source| sqlite("begin committed accumulator snapshot", source))?;
        require_store_binding(&transaction, prior.store())?;
        if crate::store::transaction::read(&transaction, prior.store())?
            != request.transaction().successor()
            || !record_is_complete(&transaction, prior.checkpoint().coordinate())?
            || !record_is_complete(&transaction, successor.checkpoint().coordinate())?
            || !checkpoint_matches(&transaction, successor.checkpoint())?
        {
            return Err(ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired).into());
        }
        let recovered = checkpoint_accumulator::preparation::recover_committed(
            &transaction,
            AccumulatorStore::DeploymentOperation,
            request,
            successor,
        )?;
        transaction
            .rollback()
            .map_err(|source| sqlite("release committed accumulator snapshot", source))?;
        Ok(recovered)
    }
}
