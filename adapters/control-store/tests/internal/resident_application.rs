//! Resident-application clean reopen over the real unadmitted `SQLite` candidate.

use std::collections::{BTreeMap, BTreeSet};

use crate::resident_fixture::shared_resident::{
    admit_and_activate, live_workspace, reconstruction_input, resolver_for_active,
    session_lease_for_active, FixedInputSource, RecoveryResolver,
};

use pale_application::resident::{
    AbsentProcessState, DesiredProcessState, GenerationLifecycleRecoveryResolver,
    ProcessGeneration, ProcessIsolationAction, ProcessIsolationPolicy, ProcessObservationClaim,
    RecoveryReferenceKnowledge, ResidentApplication, ResidentApplicationErrorClass,
    ResidentColdOpenInput, ResidentColdOpenOutcome, ResidentDesiredStatePersistence,
    ResidentGenerationInputSource, ResidentReconstructionParameters, RestartPolicy, SupervisorPlan,
    SupervisorServiceDeclaration,
};
use pale_core::records::{
    ArtifactDigest, CompiledGenerationDigest, ExecutableDigest, JournalSequenceV1,
    ResidentProcessInstanceIdV1, ServiceDefinition, ServiceLaunch, WakeEpochV1,
};
use pale_kernel::AdaptiveNodeSetPolicy;
use pale_ports::{
    AuthorityRecoveryReadLimits, AuthorityStoreCoordinate, DurableAuthorityStorePort,
    RecoveryCheckpointRead,
};

use super::DeploymentControlSqliteStore;
use crate::test_support as support;
use crate::ControlStoreKind;

#[path = "resident_application/event_custody.rs"]
mod event_custody;

fn recovery_limits() -> AuthorityRecoveryReadLimits {
    AuthorityRecoveryReadLimits::new(1 << 20, 1 << 20, 8, 64).expect("bounded recovery limits")
}

fn reconstruction_parameters(epoch: u64) -> ResidentReconstructionParameters {
    ResidentReconstructionParameters::new(
        AdaptiveNodeSetPolicy::new(32, 5_000).expect("bounded node-set policy"),
        8,
        8,
        WakeEpochV1::new(epoch),
    )
}

fn reconciliation(
    generation: &CompiledGenerationDigest,
    desired: DesiredProcessState,
) -> pale_application::resident::PendingSupervisorReconciliation {
    let plan = SupervisorPlan::admit(
        generation.clone(),
        vec![SupervisorServiceDeclaration::new(
            ServiceDefinition {
                name: "resident".to_owned(),
                kind: "daemon".to_owned(),
                precedence: 1,
                launch: ServiceLaunch {
                    program: "/unadmitted/pale".to_owned(),
                    args: Vec::new(),
                },
                ports: Vec::new(),
                depends_on: Vec::new(),
            },
            ExecutableDigest::hash_bytes(b"unadmitted-resident-executable"),
            ArtifactDigest::hash_bytes(b"unadmitted-resident-configuration"),
            RestartPolicy::new(3, 10, 40).expect("bounded restart policy"),
        )],
    )
    .expect("single-service supervisor plan");
    let service = plan.services().next().expect("service exists").id().clone();
    let expected = plan
        .bind_expected_claim(
            &service,
            ResidentProcessInstanceIdV1::parse("resident-process").expect("process identity"),
            ProcessGeneration::new(1).expect("process generation"),
        )
        .expect("expected claim");
    plan.admit_claimed_snapshot(
        BTreeMap::from([(service.clone(), expected)]),
        BTreeMap::from([(
            service,
            ProcessObservationClaim::Absent(AbsentProcessState::Startable),
        )]),
    )
    .expect("complete process snapshot")
    .reconcile(
        desired,
        ProcessIsolationPolicy::new(
            ProcessIsolationAction::Terminate,
            ProcessIsolationAction::Quarantine,
        ),
    )
}

fn open_resident<'store>(
    mut store: &'store DeploymentControlSqliteStore,
    coordinate: AuthorityStoreCoordinate,
    resolver: &dyn GenerationLifecycleRecoveryResolver,
    source: &mut dyn ResidentGenerationInputSource,
    epoch: u64,
) -> ResidentApplication<'store> {
    let opened = ResidentApplication::cold_open(
        store,
        store,
        &mut store,
        recovery_limits().maximum_entry_bytes(),
        ResidentColdOpenInput::new(
            coordinate,
            recovery_limits(),
            recovery_limits().maximum_entry_bytes(),
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            resolver,
            source,
            reconstruction_parameters(epoch),
        ),
    )
    .expect("resident cold-open succeeds");
    match opened {
        ResidentColdOpenOutcome::Ready(resident) => *resident,
        ResidentColdOpenOutcome::Degraded(_) => panic!("empty valid store cannot be degraded"),
    }
}

#[test]
fn resident_stopped_state_recovers_from_real_sqlite_without_launch_authority() {
    let directory = support::private_tempdir();
    let (location, identity, _) = support::initialize(
        &directory,
        ControlStoreKind::DeploymentControl,
        "resident-application",
    );
    let coordinate = AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    );
    let resolver = RecoveryResolver::default();
    let generation =
        CompiledGenerationDigest::parse("a".repeat(64)).expect("unadmitted generation");

    {
        let store =
            DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
                .expect("open unadmitted adapter candidate");
        let mut source = FixedInputSource::default();
        let mut resident = open_resident(&store, coordinate.clone(), &resolver, &mut source, 1);
        let prior_wake = resident.wake_cursor();
        let rejected = resident
            .commit_supervisor_reconciliation(reconciliation(
                &generation,
                DesiredProcessState::Running,
            ))
            .expect_err("an empty resident cannot persist running intent");
        assert_eq!(rejected.class(), ResidentApplicationErrorClass::Lifecycle);
        assert_eq!(resident.journal_head(), None);
        assert_eq!(resident.desired_process_state(), None);
        assert_eq!(resident.wake_cursor(), prior_wake);
        let committed = resident
            .commit_supervisor_reconciliation(reconciliation(
                &generation,
                DesiredProcessState::Stopped,
            ))
            .expect("desired state commits through the real adapter");
        assert!(matches!(
            committed.persistence(),
            ResidentDesiredStatePersistence::Committed(_)
        ));
        assert_eq!(
            resident.journal_head(),
            Some(JournalSequenceV1::new(1).expect("positive sequence"))
        );
        assert_eq!(
            resident.desired_process_state(),
            Some(DesiredProcessState::Stopped)
        );
        resident.checkpoint().expect("checkpoint commits");
        assert_eq!(source.calls, 1);
    }

    let store =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
            .expect("reopen unadmitted adapter candidate");
    let mut source = FixedInputSource::default();
    let mut resident = open_resident(&store, coordinate, &resolver, &mut source, 2);
    assert_eq!(
        resident.journal_head(),
        Some(JournalSequenceV1::new(1).expect("positive sequence"))
    );
    assert_eq!(
        resident.desired_process_state(),
        Some(DesiredProcessState::Stopped)
    );
    let prior_head = resident.journal_head();
    let prior_wake = resident.wake_cursor();
    let rejected = resident
        .commit_supervisor_reconciliation(reconciliation(&generation, DesiredProcessState::Running))
        .expect_err("reopening stopped state does not admit a generation");
    assert_eq!(rejected.class(), ResidentApplicationErrorClass::Lifecycle);
    assert_eq!(resident.journal_head(), prior_head);
    assert_eq!(
        resident.desired_process_state(),
        Some(DesiredProcessState::Stopped)
    );
    assert_eq!(resident.wake_cursor(), prior_wake);
    let replay = resident
        .commit_supervisor_reconciliation(reconciliation(&generation, DesiredProcessState::Stopped))
        .expect("identical desired state reuses its durable frontier");
    assert!(matches!(
        replay.persistence(),
        ResidentDesiredStatePersistence::AlreadyCurrent { journal_sequence }
            if *journal_sequence == JournalSequenceV1::new(1).expect("positive sequence")
    ));
    assert_eq!(resident.journal_head(), prior_head);
    assert_eq!(resident.wake_cursor(), prior_wake);
    assert_eq!(source.calls, 1);
}

#[test]
fn admitted_running_generation_survives_sqlite_checkpoint_and_tail_recovery() {
    for checkpoint_sequence in [3, 4] {
        let directory = support::private_tempdir();
        let (location, identity, _) = support::initialize(
            &directory,
            ControlStoreKind::DeploymentControl,
            "admitted-resident-recovery",
        );
        let coordinate = AuthorityStoreCoordinate::new(
            identity.logical_store_id().clone(),
            identity.control_incarnation_id().clone(),
        );
        let (workspace, probe) = live_workspace();
        let fixture = {
            let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
                &location,
                coordinate.clone(),
            )
            .expect("open real SQLite candidate");
            let mut source = FixedInputSource::default();
            let mut resident = open_resident(
                &store,
                coordinate.clone(),
                &RecoveryResolver::default(),
                &mut source,
                1,
            );
            assert_eq!(source.requested, Some(BTreeSet::new()));
            let fixture = admit_and_activate(&mut resident, &workspace, &probe)
                .expect("actual compilation and contextual heads admit");
            assert_eq!(resident.journal_head().unwrap().get(), 3);
            if checkpoint_sequence == 3 {
                resident
                    .checkpoint()
                    .expect("checkpoint before Running tail");
            }
            let running = resident
                .commit_supervisor_reconciliation(reconciliation(
                    fixture.generation.compiled_generation_digest(),
                    DesiredProcessState::Running,
                ))
                .expect("Running commits for the exact active compilation");
            assert!(matches!(
                running.persistence(),
                ResidentDesiredStatePersistence::Committed(receipt)
                    if receipt.journal_sequence().get() == 4
            ));
            if checkpoint_sequence == 4 {
                resident.checkpoint().expect("checkpoint includes Running");
            }
            fixture
        };

        // Both the resident aggregate and the first SQLite connection are gone.
        let store =
            DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
                .expect("reopen exact physical store coordinate");
        let resolver = resolver_for_active(&fixture);
        let expected = BTreeSet::from([fixture.generation.compiled_generation_digest().clone()]);
        let changes_before = store.connection().unwrap().total_changes();
        let mut source = FixedInputSource {
            inputs: vec![reconstruction_input(&workspace, &probe, &fixture)],
            ..FixedInputSource::default()
        };
        let mut recovered = open_resident(&store, coordinate.clone(), &resolver, &mut source, 2);
        assert_eq!(source.calls, 1);
        assert_eq!(source.requested.as_ref(), Some(&expected));
        assert_eq!(
            recovered.active_generation().unwrap().record(),
            &fixture.generation
        );
        assert_eq!(
            recovered
                .active_generation()
                .unwrap()
                .record()
                .profile_digest(),
            &fixture.profile.profile_digest
        );
        assert_eq!(
            recovered
                .generation_digests()
                .cloned()
                .collect::<BTreeSet<_>>(),
            expected
        );
        assert_eq!(
            recovered.lifecycle().active.as_ref(),
            Some(fixture.active.active_generation_head_digest())
        );
        assert_eq!(
            recovered.lifecycle().activation_fence.as_ref(),
            Some(fixture.fence.activation_fence_digest())
        );
        assert_eq!(recovered.journal_head().unwrap().get(), 4);
        assert_eq!(
            recovered.desired_process_state(),
            Some(DesiredProcessState::Running)
        );
        assert_eq!(recovered.wake_cursor().epoch(), WakeEpochV1::new(2));
        let wake_before = recovered.wake_cursor();
        let replay = recovered
            .commit_supervisor_reconciliation(reconciliation(
                fixture.generation.compiled_generation_digest(),
                DesiredProcessState::Running,
            ))
            .expect("exact Running reuse performs no append");
        assert!(matches!(
            replay.persistence(),
            ResidentDesiredStatePersistence::AlreadyCurrent { journal_sequence }
                if journal_sequence.get() == 4
        ));
        assert_eq!(recovered.wake_cursor(), wake_before);
        assert_eq!(store.connection().unwrap().total_changes(), changes_before);
        drop(recovered);

        let (other_workspace, other_probe) = live_workspace();
        let changed = reconstruction_input(&other_workspace, &other_probe, &fixture);
        for inputs in [Vec::new(), vec![changed]] {
            let mut source = FixedInputSource {
                inputs,
                ..FixedInputSource::default()
            };
            let result = ResidentApplication::cold_open(
                &store,
                &store,
                &mut &store,
                recovery_limits().maximum_entry_bytes(),
                ResidentColdOpenInput::new(
                    coordinate.clone(),
                    recovery_limits(),
                    recovery_limits().maximum_entry_bytes(),
                    RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                    &resolver,
                    &mut source,
                    reconstruction_parameters(3),
                ),
            );
            let Err(error) = result else {
                panic!("missing or substituted compilation cannot rebuild an admitted resident");
            };
            assert_eq!(error.class(), ResidentApplicationErrorClass::Reconstruction);
            assert_eq!(source.calls, 1);
            assert_eq!(source.requested.as_ref(), Some(&expected));
            assert_eq!(
                store.read_journal_head(&coordinate).unwrap().unwrap().get(),
                4
            );
            assert_eq!(store.connection().unwrap().total_changes(), changes_before);
        }
    }
}

#[test]
fn session_pin_and_drain_survive_sqlite_checkpoint_and_tail_recovery() {
    let directory = support::private_tempdir();
    let (location, identity, _) = support::initialize(
        &directory,
        ControlStoreKind::DeploymentControl,
        "resident-session-recovery",
    );
    let coordinate = AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    );
    let (workspace, probe) = live_workspace();
    let (fixture, first, second) = {
        let store =
            DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
                .expect("open real SQLite candidate");
        let mut source = FixedInputSource::default();
        let mut resident = open_resident(
            &store,
            coordinate.clone(),
            &RecoveryResolver::default(),
            &mut source,
            1,
        );
        let fixture = admit_and_activate(&mut resident, &workspace, &probe)
            .expect("actual compilation and contextual heads admit");
        assert_eq!(resident.journal_head().unwrap().get(), 3);
        let first = session_lease_for_active(&fixture, "session-checkpoint-a", 10);
        let second = session_lease_for_active(&fixture, "session-checkpoint-b", 20);

        let first_pin = resident
            .pin_session(&first)
            .expect("first exact session lease pins");
        assert_eq!(first_pin.journal_sequence().get(), 4);
        let second_pin = resident
            .pin_session(&second)
            .expect("second exact session lease pins");
        assert_eq!(second_pin.journal_sequence().get(), 5);
        assert_eq!(resident.journal_head().unwrap().get(), 5);
        resident
            .checkpoint()
            .expect("checkpoint includes both session pins");
        let checkpoints = store
            .read_recovery_checkpoints(&RecoveryCheckpointRead::new(
                coordinate.clone(),
                recovery_limits(),
            ))
            .expect("physical checkpoint remains readable");
        assert_eq!(
            checkpoints
                .iter()
                .map(|candidate| candidate.through_sequence().get())
                .collect::<Vec<_>>(),
            vec![5]
        );

        let drained = resident
            .drain_session(
                first.session_admission_id(),
                first.session_admission_lease_digest(),
            )
            .expect("first session drains after the checkpoint");
        assert_eq!(drained.journal_sequence().get(), 6);
        assert_eq!(resident.journal_head().unwrap().get(), 6);
        let pins = resident
            .session_pins()
            .expect("session projection is coherent");
        assert_eq!(pins.len(), 1);
        assert!(!pins.contains_key(first.session_admission_id()));
        assert!(pins.contains_key(second.session_admission_id()));
        assert_eq!(
            resident
                .sessions_retaining_generation(fixture.generation.compiled_generation_digest())
                .expect("reverse session index is coherent"),
            vec![second.session_admission_id().clone()]
        );
        (fixture, first, second)
    };

    // Both the resident aggregate and the first SQLite connection are gone.
    let store =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
            .expect("reopen exact physical store coordinate");
    let resolver = resolver_for_active(&fixture);
    let expected_generation = fixture.generation.compiled_generation_digest().clone();
    let changes_before = store.connection().unwrap().total_changes();
    let mut source = FixedInputSource {
        inputs: vec![reconstruction_input(&workspace, &probe, &fixture)],
        ..FixedInputSource::default()
    };
    let recovered = open_resident(&store, coordinate, &resolver, &mut source, 2);

    assert_eq!(source.calls, 1);
    assert_eq!(
        source.requested,
        Some(BTreeSet::from([expected_generation.clone()]))
    );
    assert_eq!(recovered.journal_head().unwrap().get(), 6);
    let pins = recovered
        .session_pins()
        .expect("recovered session projection is coherent");
    assert_eq!(pins.len(), 1);
    assert_eq!(pins, recovered.lifecycle().session_pins);
    assert!(!pins.contains_key(first.session_admission_id()));
    let retained = pins
        .get(second.session_admission_id())
        .expect("undrained session survives recovery");
    assert_eq!(
        &retained.lease_digest,
        second.session_admission_lease_digest()
    );
    assert_eq!(&retained.generation, &expected_generation);
    assert_eq!(
        recovered
            .sessions_retaining_generation(&expected_generation)
            .expect("recovered reverse session index is coherent"),
        vec![second.session_admission_id().clone()]
    );
    let wake = recovered.wake_cursor();
    assert_eq!(wake.epoch(), WakeEpochV1::new(2));
    assert_eq!(wake.journal_sequence().unwrap().get(), 6);
    assert!(recovered
        .wake_after(wake)
        .expect("fresh recovery wake cursor is readable")
        .is_empty());
    assert_eq!(store.connection().unwrap().total_changes(), changes_before);
}
