//! Borrow Event custody from the existing deployment connection owner.

use pale_ports::{
    AuthorityStoreCoordinate, ControlRecordCoordinate, ControlStoreError,
    EventCustodyCommitOutcome, EventCustodyHistoryObservation, EventCustodyHistoryRead,
    EventCustodyPort, EventCustodyReadBackOutcome, EventCustodyTransition, EventInboxPage,
    EventInboxPageRead, EventIndexObservation, EventIndexQuery, EventRecordObservation,
    PaleDerivedOutboxPage, PaleDerivedOutboxPageRead,
};

use super::DeploymentControlSqliteStore;
use crate::SqliteEventCustody;

impl pale_ports::ControlTransactionPort for &DeploymentControlSqliteStore {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, ControlStoreError> {
        self.with_event_custody(|custody| {
            pale_ports::ControlTransactionPort::prepare_transaction(custody, store)
        })
    }
}

impl DeploymentControlSqliteStore {
    fn with_event_custody<T>(
        &self,
        read_or_commit: impl FnOnce(&mut SqliteEventCustody<'_>) -> Result<T, ControlStoreError>,
    ) -> Result<T, ControlStoreError> {
        let mut connection = self.record_custody_connection()?;
        read_or_commit(&mut SqliteEventCustody {
            connection: &mut connection,
            coordinate: self.coordinate.clone(),
        })
    }
}

// This borrowed view shares the journal/session connection and lock. It does
// not open another database or select a production durability profile.
impl EventCustodyPort for &DeploymentControlSqliteStore {
    fn read_committed_members(
        &mut self,
        request: &EventCustodyHistoryRead,
    ) -> Result<EventCustodyHistoryObservation, ControlStoreError> {
        self.with_event_custody(|custody| custody.read_committed_members(request))
    }

    fn compare_and_commit(
        &mut self,
        transition: &EventCustodyTransition,
        maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyCommitOutcome, ControlStoreError> {
        self.with_event_custody(|custody| {
            custody.compare_and_commit(transition, maximum_journal_payload_bytes)
        })
    }

    fn observe_exact(
        &mut self,
        transition: &EventCustodyTransition,
        maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyReadBackOutcome, ControlStoreError> {
        self.with_event_custody(|custody| {
            custody.observe_exact(transition, maximum_journal_payload_bytes)
        })
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &EventIndexQuery,
        maximum_body_bytes: usize,
    ) -> Result<EventIndexObservation, ControlStoreError> {
        self.with_event_custody(|custody| custody.read_index(store, query, maximum_body_bytes))
    }

    fn read_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<EventRecordObservation, ControlStoreError> {
        self.with_event_custody(|custody| {
            custody.read_record(store, coordinate, maximum_body_bytes)
        })
    }

    fn read_inbox_page(
        &mut self,
        request: &EventInboxPageRead,
    ) -> Result<EventInboxPage, ControlStoreError> {
        self.with_event_custody(|custody| custody.read_inbox_page(request))
    }

    fn read_pale_derived_outbox_page(
        &mut self,
        request: &PaleDerivedOutboxPageRead,
    ) -> Result<PaleDerivedOutboxPage, ControlStoreError> {
        self.with_event_custody(|custody| custody.read_pale_derived_outbox_page(request))
    }
}
