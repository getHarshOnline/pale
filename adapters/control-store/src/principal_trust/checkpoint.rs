//! Read-only successor preparation shared by principal administration paths.

use pale_ports::{
    AccumulatorPreparationBasis, AuthorityStoreCoordinate, ControlStoreError,
    ControlStoreErrorKind, ControlTransactionPort, ControlTransactionPreparation,
    OperationAccumulatorPreparation, OperationAccumulatorPreparationError,
    PreparedOperationAccumulator, PrincipalCheckpointPreparationPort,
};
use rusqlite::{Connection, TransactionBehavior};

use crate::{accumulator, error::sqlite, ControlStoreKind};

/// Prepare principal checkpoint trees without a record-commit capability.
///
/// Construction remains private until store opening and Pale Runtime assembly are
/// qualified. A database connection alone cannot establish current authority.
pub struct SqlitePrincipalCheckpointPreparation<'connection> {
    pub(crate) connection: &'connection mut Connection,
}

impl ControlTransactionPort for SqlitePrincipalCheckpointPreparation<'_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        crate::store::transaction::prepare_snapshot(
            self.connection,
            store,
            ControlStoreKind::PrincipalTrust,
        )
    }
}

impl PrincipalCheckpointPreparationPort for SqlitePrincipalCheckpointPreparation<'_> {
    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        prepare_successor(self.connection, request)
    }
}

impl pale_ports::PrincipalTrustReadPort for SqlitePrincipalCheckpointPreparation<'_> {
    fn read_current_checkpoint(
        &mut self,
        binding: &pale_ports::PrincipalTrustStoreBinding,
        maximum_body_bytes: usize,
    ) -> Result<Option<pale_ports::PrincipalTrustSnapshot>, ControlStoreError> {
        pale_ports::PrincipalTrustReadPort::read_current_checkpoint(
            &mut super::read::SqlitePrincipalTrustReader {
                connection: self.connection,
            },
            binding,
            maximum_body_bytes,
        )
    }

    fn read_record(
        &mut self,
        binding: &pale_ports::PrincipalTrustStoreBinding,
        coordinate: &pale_ports::ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<pale_ports::ControlRecord>, ControlStoreError> {
        pale_ports::PrincipalTrustReadPort::read_record(
            &mut super::read::SqlitePrincipalTrustReader {
                connection: self.connection,
            },
            binding,
            coordinate,
            maximum_body_bytes,
        )
    }
}

/// Compare the current physical checkpoint before computing its replacement.
pub(crate) fn prepare_successor(
    connection: &mut Connection,
    request: &OperationAccumulatorPreparation,
) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
    let AccumulatorPreparationBasis::Successor(prior) = request.basis() else {
        return Err(ControlStoreError::new(ControlStoreErrorKind::CandidateConflict).into());
    };
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|source| sqlite("begin principal checkpoint preparation", source))?;
    let prepared = (|| {
        let is_principal: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pale_control_store_header AS header \
             JOIN pale_principal_trust_store_binding AS binding USING (singleton) \
             WHERE header.singleton=1 AND header.store_kind='principal_trust')",
                [],
                |row| row.get(0),
            )
            .map_err(|source| sqlite("compare principal checkpoint store class", source))?;
        if !is_principal {
            return Err(ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired).into());
        }
        let current = crate::store::transaction::prepare(&transaction, prior.store())?;
        if &current != request.transaction()
            || !super::control_checkpoint_head_matches(&transaction, prior.checkpoint())?
        {
            return Err(ControlStoreError::new(ControlStoreErrorKind::CandidateConflict).into());
        }
        accumulator::preparation::prepare(&transaction, accumulator::Store::PrincipalTrust, request)
    })();
    transaction
        .rollback()
        .map_err(|source| sqlite("release principal checkpoint preparation", source))?;
    prepared
}
