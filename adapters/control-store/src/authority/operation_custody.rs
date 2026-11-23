//! Borrow operation custody from the existing deployment connection owner.

use pale_core::records::ControlCheckpointDomainIdV1;
use pale_ports::{
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord, ControlRecordCoordinate,
    ControlStoreError, OperationAccumulatorMembership, OperationAccumulatorPreparation,
    OperationAccumulatorPreparationError, OperationCustodyCandidate, OperationCustodyCommitOutcome,
    OperationCustodyIndexObservation, OperationCustodyIndexQuery, OperationCustodyPort,
    OperationCustodyReadBackOutcome, OperationPublicationCursor, OperationPublicationReadError,
    OperationPublicationSources, PreparedOperationAccumulator,
    VerifiedOperationAccumulatorMembership,
};

use super::DeploymentControlSqliteStore;
use crate::SqliteOperationCustody;

impl DeploymentControlSqliteStore {
    fn with_operation_custody<T, E: From<ControlStoreError>>(
        &self,
        read_or_commit: impl FnOnce(&mut SqliteOperationCustody<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        let mut connection = self.record_custody_connection()?;
        read_or_commit(&mut SqliteOperationCustody {
            connection: &mut connection,
            coordinate: self.coordinate.clone(),
        })
    }
}

// Use the existing borrowed ControlTransactionPort implementation. Journal,
// session, Event and operation custody share this one connection and lock;
// this view neither opens a database nor admits a storage profile.
impl OperationCustodyPort for &DeploymentControlSqliteStore {
    fn read_collection_snapshot(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        maximum_frontier_bytes: usize,
        maximum_journal_payload_bytes: usize,
    ) -> Result<Option<pale_ports::OperationCollectionSnapshot>, ControlStoreError> {
        self.with_operation_custody(|custody| {
            custody.read_collection_snapshot(
                store,
                handle,
                maximum_frontier_bytes,
                maximum_journal_payload_bytes,
            )
        })
    }

    fn read_collection_snapshot_at(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        frontier: &ControlRecordCoordinate,
        maximum_frontier_bytes: usize,
        maximum_journal_payload_bytes: usize,
    ) -> Result<Option<pale_ports::OperationCollectionSnapshot>, ControlStoreError> {
        self.with_operation_custody(|custody| {
            custody.read_collection_snapshot_at(
                store,
                handle,
                frontier,
                maximum_frontier_bytes,
                maximum_journal_payload_bytes,
            )
        })
    }

    fn recover_committed_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
        successor: &pale_ports::OperationAccumulatorCheckpoint,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        self.with_operation_custody(|custody| {
            custody.recover_committed_accumulator(request, successor)
        })
    }

    fn read_a2a_task_bindings(
        &mut self,
        store: &AuthorityStoreCoordinate,
        scope: &pale_ports::OperationA2aBindingScope,
        cursor: Option<&pale_ports::OperationA2aBindingCursor>,
        maximum_bindings: usize,
        maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationA2aBindings, ControlStoreError> {
        self.with_operation_custody(|custody| {
            custody.read_a2a_task_bindings(
                store,
                scope,
                cursor,
                maximum_bindings,
                maximum_body_bytes,
            )
        })
    }
    fn read_operation_session_basis(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        maximum_bytes: usize,
    ) -> Result<Option<pale_ports::OperationSessionBasis>, ControlStoreError> {
        self.with_operation_custody(|custody| {
            custody.read_operation_session_basis(store, handle, maximum_bytes)
        })
    }

    fn read_operation_starts(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&pale_ports::OperationStartCursor>,
        maximum_sources: usize,
        maximum_session_basis_bytes: usize,
    ) -> Result<pale_ports::OperationStartSources, pale_ports::OperationStartReadError> {
        self.with_operation_custody(|custody| {
            custody.read_operation_starts(
                store,
                cursor,
                maximum_sources,
                maximum_session_basis_bytes,
            )
        })
    }

    fn read_a2a_evidence(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_bytes: usize,
    ) -> Result<Option<pale_ports::OperationA2aEvidence>, ControlStoreError> {
        self.with_operation_custody(|custody| {
            custody.read_a2a_evidence(store, coordinate, maximum_bytes)
        })
    }

    fn read_publication_sources(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&OperationPublicationCursor>,
        maximum_sources: usize,
        maximum_body_bytes: usize,
    ) -> Result<OperationPublicationSources, OperationPublicationReadError> {
        self.with_operation_custody(|custody| {
            custody.read_publication_sources(store, cursor, maximum_sources, maximum_body_bytes)
        })
    }

    fn read_checkpoint_head(
        &mut self,
        store: &AuthorityStoreCoordinate,
        domain: &ControlCheckpointDomainIdV1,
        maximum_record_bytes: usize,
    ) -> Result<Option<(ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        self.with_operation_custody(|custody| {
            custody.read_checkpoint_head(store, domain, maximum_record_bytes)
        })
    }

    fn verify_accumulator_membership(
        &mut self,
        request: &OperationAccumulatorMembership,
    ) -> Result<VerifiedOperationAccumulatorMembership, OperationAccumulatorPreparationError> {
        self.with_operation_custody(|custody| custody.verify_accumulator_membership(request))
    }

    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        self.with_operation_custody(|custody| custody.prepare_accumulator(request))
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        store: &AuthorityStoreCoordinate,
        record: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.with_operation_custody(|custody| {
            custody.compare_and_retain_operation_lease(store, record)
        })
    }

    fn read_operation_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        self.with_operation_custody(|custody| {
            custody.read_operation_record(store, coordinate, maximum_body_bytes)
        })
    }

    fn compare_and_commit(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.with_operation_custody(|custody| custody.compare_and_commit(candidate))
    }

    fn observe_exact(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        self.with_operation_custody(|custody| custody.observe_exact(candidate))
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        maximum_record_bytes: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        self.with_operation_custody(|custody| {
            custody.read_index(store, query, maximum_record_bytes)
        })
    }
}
