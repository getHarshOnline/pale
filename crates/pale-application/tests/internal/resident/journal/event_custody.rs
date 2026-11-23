//! Explicitly unavailable Event custody for lifecycle-only recovery fixtures.

use pale_ports::*;

pub(in crate::resident) struct UnavailableEventCustody;

fn unavailable<T>() -> Result<T, ControlStoreError> {
    Err(ControlStoreError::new(ControlStoreErrorKind::Storage))
}

impl EventCustodyPort for UnavailableEventCustody {
    fn read_committed_members(
        &mut self,
        _: &EventCustodyHistoryRead,
    ) -> Result<EventCustodyHistoryObservation, ControlStoreError> {
        unavailable()
    }
    fn compare_and_commit(
        &mut self,
        _: &EventCustodyTransition,
        _: usize,
    ) -> Result<EventCustodyCommitOutcome, ControlStoreError> {
        unavailable()
    }
    fn observe_exact(
        &mut self,
        _: &EventCustodyTransition,
        _: usize,
    ) -> Result<EventCustodyReadBackOutcome, ControlStoreError> {
        unavailable()
    }
    fn read_index(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &EventIndexQuery,
        _: usize,
    ) -> Result<EventIndexObservation, ControlStoreError> {
        unavailable()
    }
    fn read_record(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecordCoordinate,
        _: usize,
    ) -> Result<EventRecordObservation, ControlStoreError> {
        unavailable()
    }
    fn read_inbox_page(
        &mut self,
        _: &EventInboxPageRead,
    ) -> Result<EventInboxPage, ControlStoreError> {
        unavailable()
    }
    fn read_pale_derived_outbox_page(
        &mut self,
        _: &PaleDerivedOutboxPageRead,
    ) -> Result<PaleDerivedOutboxPage, ControlStoreError> {
        unavailable()
    }
}
