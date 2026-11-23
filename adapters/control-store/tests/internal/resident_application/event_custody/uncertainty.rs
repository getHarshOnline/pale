//! Lost commit replies must not become duplicate writes or invented successors.

use super::*;
use pale_ports::{
    ControlRecordCoordinate, ControlStoreError, ControlStoreErrorKind, EventCustodyCommitOutcome,
    EventCustodyHistoryObservation, EventCustodyHistoryRead, EventCustodyPort,
    EventCustodyReadBackOutcome, EventCustodyTransition, EventInboxPage, EventInboxPageRead,
    EventIndexObservation, EventIndexQuery, EventRecordObservation, PaleDerivedOutboxPage,
    PaleDerivedOutboxPageRead,
};

#[derive(Clone, Copy, Debug)]
enum LostReply {
    Committed,
    NotCommitted,
    ContradictoryReadBack,
    UnavailableReadBack,
    ReadBackBoundExceeded,
}

struct FaultyReply<'store> {
    store: &'store DeploymentControlSqliteStore,
    fault: LostReply,
    commits: usize,
    read_backs: usize,
}

impl EventCustodyPort for FaultyReply<'_> {
    fn compare_and_commit(
        &mut self,
        candidate: &EventCustodyTransition,
        maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyCommitOutcome, ControlStoreError> {
        self.commits += 1;
        if matches!(self.fault, LostReply::Committed) {
            self.store
                .compare_and_commit(candidate, maximum_journal_payload_bytes)?;
        }
        Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        ))
    }

    fn observe_exact(
        &mut self,
        candidate: &EventCustodyTransition,
        maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyReadBackOutcome, ControlStoreError> {
        self.read_backs += 1;
        match self.fault {
            LostReply::Committed | LostReply::NotCommitted => self
                .store
                .observe_exact(candidate, maximum_journal_payload_bytes),
            LostReply::ContradictoryReadBack => {
                Ok(EventCustodyReadBackOutcome::ConflictingOrPartial)
            }
            LostReply::UnavailableReadBack => {
                Err(ControlStoreError::new(ControlStoreErrorKind::Storage))
            }
            LostReply::ReadBackBoundExceeded => {
                Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded))
            }
        }
    }

    fn read_committed_members(
        &mut self,
        request: &EventCustodyHistoryRead,
    ) -> Result<EventCustodyHistoryObservation, ControlStoreError> {
        self.store.read_committed_members(request)
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &EventIndexQuery,
        maximum_body_bytes: usize,
    ) -> Result<EventIndexObservation, ControlStoreError> {
        self.store.read_index(store, query, maximum_body_bytes)
    }

    fn read_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        record: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<EventRecordObservation, ControlStoreError> {
        self.store.read_record(store, record, maximum_body_bytes)
    }

    fn read_inbox_page(
        &mut self,
        request: &EventInboxPageRead,
    ) -> Result<EventInboxPage, ControlStoreError> {
        self.store.read_inbox_page(request)
    }

    fn read_pale_derived_outbox_page(
        &mut self,
        request: &PaleDerivedOutboxPageRead,
    ) -> Result<PaleDerivedOutboxPage, ControlStoreError> {
        self.store.read_pale_derived_outbox_page(request)
    }
}

#[test]
fn one_read_back_distinguishes_commit_exclusion_corruption_and_uncertainty() {
    for fault in [
        LostReply::Committed,
        LostReply::NotCommitted,
        LostReply::ContradictoryReadBack,
        LostReply::UnavailableReadBack,
        LostReply::ReadBackBoundExceeded,
    ] {
        let directory = support::private_tempdir();
        let (location, identity, _) = support::initialize(
            &directory,
            ControlStoreKind::DeploymentControl,
            "resident-event-lost-reply",
        );
        let coordinate = AuthorityStoreCoordinate::new(
            identity.logical_store_id().clone(),
            identity.control_incarnation_id().clone(),
        );
        let store =
            DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
                .unwrap();
        let resolver = RecoveryResolver::default();
        let mut source = FixedInputSource::default();
        let mut resident = open_resident(&store, coordinate.clone(), &resolver, &mut source, 1);
        let transaction = (&store).prepare_transaction(&coordinate).unwrap();
        let active = subscription("subscription-active", SubscriptionLifecycle::Active);
        let wake = resident.wake_cursor();
        let mut replies = FaultyReply {
            store: &store,
            fault,
            commits: 0,
            read_backs: 0,
        };
        let result = resident.commit_event_custody(
            &mut replies,
            transaction,
            EventCustodyTransitionInput::AdmitSubscription(&active),
        );
        assert_eq!(replies.commits, 1, "{fault:?}: no automatic repeat commit");
        assert_eq!(replies.read_backs, 1, "{fault:?}: one mandatory read-back");
        match fault {
            LostReply::Committed => {
                let (mut completed, _) = result.unwrap();
                assert_eq!(
                    completed.evidence(),
                    EventCustodyCommitEvidence::RecoveredExactAfterAmbiguousCommit
                );
                assert_eq!(completed.take_continuation(), None);
                assert_eq!(resident.journal_head().unwrap().get(), 1);
                assert_eq!(
                    resident.journal_head(),
                    store.read_journal_head(&coordinate).unwrap()
                );
                assert_ne!(resident.wake_cursor(), wake);
            }
            LostReply::NotCommitted => {
                assert_eq!(
                    result.unwrap_err().class(),
                    ResidentApplicationErrorClass::DurabilityNoMutation
                );
                assert_eq!(resident.journal_head(), None);
                assert_eq!(resident.wake_cursor(), wake);
                // A separate explicit call remains possible after exact exclusion.
                commit_subscription(
                    &mut resident,
                    &store,
                    &coordinate,
                    EventCustodyTransitionInput::AdmitSubscription(&active),
                );
                assert_eq!(resident.journal_head().unwrap().get(), 1);
            }
            LostReply::ContradictoryReadBack
            | LostReply::UnavailableReadBack
            | LostReply::ReadBackBoundExceeded => {
                let expected = if matches!(fault, LostReply::ContradictoryReadBack) {
                    ResidentApplicationErrorClass::DurabilityIntegrity
                } else {
                    ResidentApplicationErrorClass::DurabilityOutcomeUnknown
                };
                assert_eq!(result.unwrap_err().class(), expected);
                assert_eq!(resident.journal_head(), None);
                assert_eq!(resident.wake_cursor(), wake);
                let before = store.connection().unwrap().total_changes();
                let transaction = (&store).prepare_transaction(&coordinate).unwrap();
                let refused = resident
                    .commit_event_custody(
                        &mut replies,
                        transaction,
                        EventCustodyTransitionInput::AdmitSubscription(&active),
                    )
                    .unwrap_err();
                assert_eq!(refused.class(), expected);
                assert_eq!(replies.commits, 1);
                assert_eq!(replies.read_backs, 1);
                assert_eq!(store.connection().unwrap().total_changes(), before);
                assert_eq!(resident.wake_cursor(), wake);
            }
        }
    }
}
