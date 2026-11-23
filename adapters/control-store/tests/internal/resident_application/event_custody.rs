//! Event state commits and recovers through the resident's real database connection.

use super::*;
use pale_application::resident::{
    AdmittedEventSubscription, CompletedEventCustodyTransition, EventCustodyCommitEvidence,
    EventCustodyTransitionInput, EventSubscriptionDeclaration, SubscriptionLifecycle,
};
use pale_core::records::{
    EventSubscriptionIdV1, EventTypeV1, ResidentPrincipalIdV1, ResidentRecordRefV1, SourceOwnerV1,
    UnixTimeV1,
};
use pale_ports::ControlTransactionPort;

#[path = "event_custody/uncertainty.rs"]
mod uncertainty;

#[path = "event_custody/local_operation.rs"]
mod local_operation;

#[path = "event_custody/initial_delivery.rs"]
mod initial_delivery;

fn subscription(reference: &str, lifecycle: SubscriptionLifecycle) -> AdmittedEventSubscription {
    AdmittedEventSubscription::admit(
        ResidentRecordRefV1::parse(reference).unwrap(),
        EventSubscriptionDeclaration {
            subscription_id: EventSubscriptionIdV1::parse("resident-subscription").unwrap(),
            source_owner: SourceOwnerV1::parse("native-owner").unwrap(),
            event_types: vec![EventTypeV1::parse("owner.record.committed").unwrap()],
            destination_principal_id: ResidentPrincipalIdV1::parse("principal-1").unwrap(),
            destination_node_id: None,
            ingress_frontier: JournalSequenceV1::new(1).unwrap(),
        },
        lifecycle,
    )
    .unwrap()
}

fn commit_subscription(
    resident: &mut ResidentApplication<'_>,
    mut store: &DeploymentControlSqliteStore,
    coordinate: &AuthorityStoreCoordinate,
    input: EventCustodyTransitionInput<'_>,
) -> CompletedEventCustodyTransition {
    let transaction = store.prepare_transaction(coordinate).unwrap();
    let (mut completed, _) = resident
        .commit_event_custody(&mut store, transaction, input)
        .expect("record, index and resident journal commit together");
    assert_eq!(completed.evidence(), EventCustodyCommitEvidence::New);
    assert_eq!(completed.take_continuation(), None);
    let member = completed.candidate().members().first().unwrap();
    let inserted: i64 = store.connection().unwrap().query_row(
        "SELECT inserted_transaction_sequence FROM pale_deployment_control_record WHERE record_ref=?1",
        [member.record().coordinate().record_ref().as_str()],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(
        inserted,
        i64::try_from(completed.candidate().transaction().successor().get()).unwrap()
    );
    completed
}

#[test]
fn event_history_recovers_after_index_retirement_and_later_process_checkpoint() {
    // Tail replay must accept the earlier active record after its index moved.
    // A later checkpoint must also retain the Event basis across a process delta.
    for checkpoint in [false, true] {
        let directory = support::private_tempdir();
        let (location, identity, _) = support::initialize(
            &directory,
            ControlStoreKind::DeploymentControl,
            "resident-event-reopen",
        );
        let coordinate = AuthorityStoreCoordinate::new(
            identity.logical_store_id().clone(),
            identity.control_incarnation_id().clone(),
        );
        let resolver = RecoveryResolver::default();
        let active = subscription("subscription-active", SubscriptionLifecycle::Active);
        let retired = subscription(
            "subscription-retired",
            SubscriptionLifecycle::Retired {
                retired_at: UnixTimeV1::new(2).unwrap(),
            },
        );
        {
            let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
                &location,
                coordinate.clone(),
            )
            .unwrap();
            let mut source = FixedInputSource::default();
            let mut resident = open_resident(&store, coordinate.clone(), &resolver, &mut source, 1);
            commit_subscription(
                &mut resident,
                &store,
                &coordinate,
                EventCustodyTransitionInput::AdmitSubscription(&active),
            );
            commit_subscription(
                &mut resident,
                &store,
                &coordinate,
                EventCustodyTransitionInput::RetireSubscription {
                    predecessor: &active,
                    successor: &retired,
                },
            );
            resident
                .commit_supervisor_reconciliation(reconciliation(
                    &CompiledGenerationDigest::parse("a".repeat(64)).unwrap(),
                    DesiredProcessState::Stopped,
                ))
                .unwrap();
            assert_eq!(resident.journal_head().unwrap().get(), 3);
            if checkpoint {
                resident.checkpoint().unwrap();
            }
        }
        let store =
            DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
                .unwrap();
        let before = store.connection().unwrap().total_changes();
        let mut source = FixedInputSource::default();
        let resident = open_resident(&store, coordinate, &resolver, &mut source, 2);
        assert_eq!(resident.journal_head().unwrap().get(), 3);
        assert_eq!(
            resident.desired_process_state(),
            Some(DesiredProcessState::Stopped)
        );
        assert_eq!(store.connection().unwrap().total_changes(), before);
    }
}

#[test]
fn retained_event_bytes_and_transaction_membership_are_required_on_reopen() {
    let faults = [
        "UPDATE pale_deployment_control_record SET canonical_record_jcs=x'7b7d'",
        "UPDATE pale_deployment_control_record SET schema_id='pale:unlike-schema'",
        "UPDATE pale_deployment_control_record SET schema_sha256=printf('%064d',0)",
        "UPDATE pale_deployment_control_record SET inserted_transaction_sequence=1",
        "DELETE FROM pale_deployment_event_index; DELETE FROM pale_deployment_control_record",
        "DELETE FROM pale_deployment_authority_journal WHERE journal_sequence=1",
    ];
    for checkpoint in [false, true] {
        for (fault_index, fault) in faults.iter().enumerate() {
            let directory = support::private_tempdir();
            let (location, identity, _) = support::initialize(
                &directory,
                ControlStoreKind::DeploymentControl,
                "resident-event-corruption",
            );
            let coordinate = AuthorityStoreCoordinate::new(
                identity.logical_store_id().clone(),
                identity.control_incarnation_id().clone(),
            );
            let resolver = RecoveryResolver::default();
            {
                let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
                    &location,
                    coordinate.clone(),
                )
                .unwrap();
                let mut source = FixedInputSource::default();
                let mut resident =
                    open_resident(&store, coordinate.clone(), &resolver, &mut source, 1);
                let active = subscription("subscription-active", SubscriptionLifecycle::Active);
                commit_subscription(
                    &mut resident,
                    &store,
                    &coordinate,
                    EventCustodyTransitionInput::AdmitSubscription(&active),
                );
                resident
                    .commit_supervisor_reconciliation(reconciliation(
                        &CompiledGenerationDigest::parse("a".repeat(64)).unwrap(),
                        DesiredProcessState::Stopped,
                    ))
                    .unwrap();
                if checkpoint {
                    resident.checkpoint().unwrap();
                }
            }
            let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
                &location,
                coordinate.clone(),
            )
            .unwrap();
            store.connection().unwrap().execute_batch(fault).unwrap();
            let before = store.connection().unwrap().total_changes();
            let mut source = FixedInputSource::default();
            let result = ResidentApplication::cold_open(
                &store,
                &store,
                &mut &store,
                recovery_limits().maximum_entry_bytes(),
                ResidentColdOpenInput::new(
                    coordinate,
                    recovery_limits(),
                    recovery_limits().maximum_entry_bytes(),
                    RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                    &resolver,
                    &mut source,
                    reconstruction_parameters(2),
                ),
            );
            // A hole in an unchecked tail preserves only its valid prefix.
            // Missing custody for a selected checkpoint is a fatal contradiction.
            match result {
                Ok(ResidentColdOpenOutcome::Degraded(degraded))
                    if !checkpoint && fault_index == 5 =>
                {
                    assert_eq!(degraded.quarantine().valid_through(), None);
                    assert_eq!(degraded.quarantine().first_untrusted_sequence().get(), 1);
                    assert_eq!(degraded.quarantine().durable_head().get(), 2);
                }
                Err(error) if checkpoint || fault_index != 5 => {
                    assert_eq!(
                        error.class(),
                        ResidentApplicationErrorClass::DurabilityIntegrity,
                        "checkpoint={checkpoint}, fault={fault}: {error}"
                    );
                }
                _ => panic!("unexpected recovery for checkpoint={checkpoint}, fault={fault}"),
            }
            assert_eq!(store.connection().unwrap().total_changes(), before);
        }
    }
}

#[test]
fn rejected_event_transaction_leaves_no_record_index_journal_or_wake() {
    // Fail each physical write stage, including the final journal-head update.
    for table in [
        "pale_deployment_control_record",
        "pale_deployment_event_index",
        "pale_deployment_authority_journal",
        "pale_deployment_authority_head",
    ] {
        let directory = support::private_tempdir();
        let (location, identity, _) = support::initialize(
            &directory,
            ControlStoreKind::DeploymentControl,
            "resident-event-rollback",
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
        let active = subscription("subscription-active", SubscriptionLifecycle::Active);
        let wake = resident.wake_cursor();
        let transaction = (&store).prepare_transaction(&coordinate).unwrap();
        let verb = if table == "pale_deployment_authority_head" {
            "UPDATE"
        } else {
            "INSERT"
        };
        store.connection().unwrap().execute_batch(&format!(
            "CREATE TEMP TRIGGER refuse_event BEFORE {verb} ON {table} BEGIN SELECT RAISE(ABORT,'injected failure'); END;"
        )).unwrap();
        let error = resident
            .commit_event_custody(
                &mut &store,
                transaction.clone(),
                EventCustodyTransitionInput::AdmitSubscription(&active),
            )
            .unwrap_err();
        assert_eq!(
            error.class(),
            ResidentApplicationErrorClass::DurabilityNoMutation
        );
        assert_eq!(resident.journal_head(), None);
        assert_eq!(store.read_journal_head(&coordinate).unwrap(), None);
        assert_eq!(resident.wake_cursor(), wake);
        for empty_table in [
            "pale_deployment_control_record",
            "pale_deployment_event_index",
            "pale_deployment_authority_journal",
        ] {
            let count: i64 = store
                .connection()
                .unwrap()
                .query_row(&format!("SELECT COUNT(*) FROM {empty_table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "rollback at {table}: {empty_table}");
        }
        let after = (&store).prepare_transaction(&coordinate).unwrap();
        assert_eq!(
            after, transaction,
            "logical transaction head also rolls back"
        );
    }
}

#[test]
fn competing_event_commit_closes_the_old_resident_writer() {
    let directory = support::private_tempdir();
    let (location, identity, _) = support::initialize(
        &directory,
        ControlStoreKind::DeploymentControl,
        "resident-event-writer-race",
    );
    let coordinate = AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    );
    let store =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
            .unwrap();
    let resolver = RecoveryResolver::default();
    let mut first_source = FixedInputSource::default();
    let mut other_source = FixedInputSource::default();
    let mut first = open_resident(&store, coordinate.clone(), &resolver, &mut first_source, 1);
    let mut other = open_resident(&store, coordinate.clone(), &resolver, &mut other_source, 2);
    let stale_transaction = (&store).prepare_transaction(&coordinate).unwrap();
    let prior_wake = first.wake_cursor();
    let active = subscription("subscription-active", SubscriptionLifecycle::Active);
    let winning = subscription("subscription-winning", SubscriptionLifecycle::Active);
    commit_subscription(
        &mut other,
        &store,
        &coordinate,
        EventCustodyTransitionInput::AdmitSubscription(&winning),
    );
    let error = first
        .commit_event_custody(
            &mut &store,
            stale_transaction,
            EventCustodyTransitionInput::AdmitSubscription(&active),
        )
        .unwrap_err();
    assert_eq!(
        error.class(),
        ResidentApplicationErrorClass::DurabilityConflict
    );
    assert_eq!(first.journal_head(), None);
    assert_eq!(first.wake_cursor(), prior_wake);
    let before = store.connection().unwrap().total_changes();
    let fresh_transaction = (&store).prepare_transaction(&coordinate).unwrap();
    let error = first
        .commit_event_custody(
            &mut &store,
            fresh_transaction,
            EventCustodyTransitionInput::AdmitSubscription(&active),
        )
        .unwrap_err();
    assert_eq!(
        error.class(),
        ResidentApplicationErrorClass::DurabilityConflict
    );
    assert_eq!(store.connection().unwrap().total_changes(), before);
    assert_eq!(
        store.read_journal_head(&coordinate).unwrap().unwrap().get(),
        1
    );
    assert_eq!(first.wake_cursor(), prior_wake);
}

#[test]
fn exact_event_replay_installs_the_existing_successor_without_another_transaction() {
    let directory = support::private_tempdir();
    let (location, identity, _) = support::initialize(
        &directory,
        ControlStoreKind::DeploymentControl,
        "resident-event-replay",
    );
    let coordinate = AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    );
    let store =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
            .unwrap();
    let resolver = RecoveryResolver::default();
    let mut first_source = FixedInputSource::default();
    let mut other_source = FixedInputSource::default();
    let mut first = open_resident(&store, coordinate.clone(), &resolver, &mut first_source, 1);
    let mut other = open_resident(&store, coordinate.clone(), &resolver, &mut other_source, 2);
    let transaction = (&store).prepare_transaction(&coordinate).unwrap();
    let active = subscription("subscription-active", SubscriptionLifecycle::Active);
    commit_subscription(
        &mut other,
        &store,
        &coordinate,
        EventCustodyTransitionInput::AdmitSubscription(&active),
    );
    let before = store.connection().unwrap().total_changes();
    let (mut replay, _) = first
        .commit_event_custody(
            &mut &store,
            transaction,
            EventCustodyTransitionInput::AdmitSubscription(&active),
        )
        .unwrap();
    assert_eq!(replay.evidence(), EventCustodyCommitEvidence::ExactReplay);
    assert_eq!(replay.take_continuation(), None);
    assert_eq!(store.connection().unwrap().total_changes(), before);
    assert_eq!(first.journal_head(), other.journal_head());
}

#[test]
fn event_recovery_byte_limits_do_not_report_valid_custody_as_corrupt() {
    for checkpoint in [false, true] {
        let directory = support::private_tempdir();
        let (location, identity, _) = support::initialize(
            &directory,
            ControlStoreKind::DeploymentControl,
            "resident-event-byte-limit",
        );
        let coordinate = AuthorityStoreCoordinate::new(
            identity.logical_store_id().clone(),
            identity.control_incarnation_id().clone(),
        );
        let resolver = RecoveryResolver::default();
        let (journal_bytes, exact_bytes) = {
            let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
                &location,
                coordinate.clone(),
            )
            .unwrap();
            let mut source = FixedInputSource::default();
            let mut resident = open_resident(&store, coordinate.clone(), &resolver, &mut source, 1);
            let active = subscription("subscription-active", SubscriptionLifecycle::Active);
            let completed = commit_subscription(
                &mut resident,
                &store,
                &coordinate,
                EventCustodyTransitionInput::AdmitSubscription(&active),
            );
            let journal_bytes = completed
                .candidate()
                .journal_append()
                .entry()
                .bytes()
                .as_slice()
                .len();
            let exact_bytes = journal_bytes
                + completed
                    .candidate()
                    .members()
                    .iter()
                    .map(|member| member.record().canonical_record_jcs().as_slice().len())
                    .sum::<usize>();
            assert!(exact_bytes > journal_bytes);
            resident
                .commit_supervisor_reconciliation(reconciliation(
                    &CompiledGenerationDigest::parse("a".repeat(64)).unwrap(),
                    DesiredProcessState::Stopped,
                ))
                .unwrap();
            if checkpoint {
                resident.checkpoint().unwrap();
            }
            (journal_bytes, exact_bytes)
        };
        let store =
            DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
                .unwrap();
        let before = store.connection().unwrap().total_changes();
        for limit in [
            journal_bytes - 1,
            journal_bytes,
            exact_bytes - 1,
            exact_bytes,
        ] {
            let mut source = FixedInputSource::default();
            let result = ResidentApplication::cold_open(
                &store,
                &store,
                &mut &store,
                limit,
                ResidentColdOpenInput::new(
                    coordinate.clone(),
                    recovery_limits(),
                    recovery_limits().maximum_entry_bytes(),
                    RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                    &resolver,
                    &mut source,
                    reconstruction_parameters(2),
                ),
            );
            match result {
                Err(error) if limit < exact_bytes => assert_eq!(
                    error.class(),
                    ResidentApplicationErrorClass::DurabilityBoundExceeded,
                    "checkpoint={checkpoint}, limit={limit}: {error}"
                ),
                Ok(ResidentColdOpenOutcome::Ready(resident)) if limit == exact_bytes => {
                    assert_eq!(resident.journal_head().unwrap().get(), 2)
                }
                _ => panic!("unexpected byte-limit result: checkpoint={checkpoint}, limit={limit}"),
            }
            assert_eq!(store.connection().unwrap().total_changes(), before);
        }
    }
}
