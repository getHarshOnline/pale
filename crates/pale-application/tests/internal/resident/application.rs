//! End-to-end resident application-root and diagnostic-cause laws.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as _;

use pale_core::records::{
    ArtifactDigest, CompiledGenerationDigest, ExecutableDigest, JournalSequenceV1,
    ResidentProcessInstanceIdV1, RingCursorV1, ServiceDefinition, ServiceLaunch, WakeEpochV1,
};
use pale_kernel::WakeTokenRing;
use pale_ports::{DurableAuthorityStoreError, DurableAuthorityStoreErrorKind, LocalIpcFramePort};

use super::*;
use crate::observation::{
    ObservationBackpressureRecovery, ObservationCancellation, ObservationCategory,
    ObservationChange, ObservationDeltaOutcome, ObservationDeltaRequest, ObservationFact,
    ObservationFilter, ObservationFreshness, ObservationGeneration, ObservationHealthState,
    ObservationNativeState, ObservationPage, ObservationPageDisposition, ObservationPageRequest,
    ObservationProcessIntent, ObservationReadinessState, ObservationRevision, ObservationSnapshot,
    ObservationSubject,
};
use crate::resident::generation::tests::retarget_workspace_identity;
use crate::resident::journal::tests::event_custody::UnavailableEventCustody;
use crate::resident::journal::tests::{limits, store, MemoryAuthorityStore};
use crate::resident_fixture::{
    active, admit_and_activate, advertised, applied_evidence, compile_live, fence, live_workspace,
    published, reconstruction_input, resolver_for_active, session_lease_for_active,
    session_lease_for_profile, synthetic_set_policy, ActiveResidentFixture, FixedInputSource,
    RecoveryResolver,
};

use crate::resident::working_set::ResidentSessionWorkingSet;
use crate::resident::{
    CallerBoundResidentHookRequest, DesiredProcessState, PendingSupervisorReconciliation,
    ProcessGeneration, ProcessIsolationAction, ProcessIsolationPolicy, ProcessObservationClaim,
    ResidentGenerationReconstructionInput, ResidentHookCallerBinding, ResidentHookIngressRequest,
    ResidentHookSessionAdmissionError, RestartPolicy, ServiceProbeClaim, StopSentinelIntent,
    SupervisorPlan, SupervisorReconciliationStep, SupervisorServiceDeclaration,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn ready_page(disposition: ObservationPageDisposition) -> ObservationPage {
    let ObservationPageDisposition::Ready(page) = disposition else {
        panic!("active request must return a page");
    };
    page
}

impl ResidentApplication<'_> {
    /// Retain one already-admitted generation for process-host recovery tests.
    pub(in crate::resident) fn fixture_retain_generation(
        &mut self,
        generation: AdmittedCompiledGeneration,
    ) {
        self.generations.insert(
            generation.record().compiled_generation_digest().clone(),
            generation,
        );
    }
}

impl ResidentHookCallerBinding {
    /// Bind a request to the caller key its test carrier authenticated.
    pub(in crate::resident) fn admit_request<'binding, 'request>(
        &'binding self,
        carrier: &dyn LocalIpcFramePort,
        request: &'request ResidentHookIngressRequest,
    ) -> Result<CallerBoundResidentHookRequest<'binding, 'request>, ResidentHookSessionAdmissionError>
    {
        self.admit_authenticated_request(carrier.authenticated_caller_key(), request)
    }
}

#[path = "application/session_admission.rs"]
mod session_admission;

fn parameters(epoch: u64) -> ResidentReconstructionParameters {
    ResidentReconstructionParameters::new(synthetic_set_policy(), 8, 8, WakeEpochV1::new(epoch))
}

pub(in crate::resident) fn empty_application(
    port: &MemoryAuthorityStore,
    starting_cursor: RingCursorV1,
) -> ResidentApplication<'_> {
    empty_application_with_session_capacity(port, starting_cursor, 8)
}

fn empty_application_with_session_capacity(
    port: &MemoryAuthorityStore,
    starting_cursor: RingCursorV1,
    resident_session_capacity: usize,
) -> ResidentApplication<'_> {
    let ring = WakeTokenRing::new(8, WakeEpochV1::new(1), starting_cursor, None)
        .expect("bounded wake ring");
    let coordinator = ResidentAuthorityCoordinator::resume(
        crate::resident::journal::tests::empty_journal(port),
        ring,
    )
    .expect("empty authority frontiers align");
    let sessions =
        ResidentSessionWorkingSet::rebuild(coordinator.lifecycle(), resident_session_capacity)
            .expect("empty resident-session projection");
    ResidentApplication {
        coordinator,
        session_lease_port: port,
        maximum_session_lease_body_bytes: limits().maximum_entry_bytes(),
        generations: BTreeMap::new(),
        sessions: ResidentSessionProjection::ready(sessions),
    }
}

fn pending_reconciliation(
    generation: &CompiledGenerationDigest,
    desired: DesiredProcessState,
) -> PendingSupervisorReconciliation {
    let plan = SupervisorPlan::admit(
        generation.clone(),
        vec![SupervisorServiceDeclaration::new(
            ServiceDefinition {
                name: "memory".to_owned(),
                kind: "daemon".to_owned(),
                precedence: 1,
                launch: ServiceLaunch {
                    program: "/managed/memory".to_owned(),
                    args: vec!["serve".to_owned()],
                },
                ports: vec![3309],
                depends_on: vec![],
            },
            ExecutableDigest::hash_bytes(b"memory-executable"),
            ArtifactDigest::hash_bytes(b"memory-launch"),
            RestartPolicy::new(3, 10, 40).expect("restart policy"),
        )],
    )
    .expect("single service plan admits");
    let service = plan.services().next().expect("service exists").id().clone();
    let fence = plan
        .bind_expected_claim(
            &service,
            ResidentProcessInstanceIdV1::parse("memory-process").expect("process identity"),
            ProcessGeneration::new(1).expect("process generation"),
        )
        .expect("service claim binds");
    let claims = BTreeMap::from([(service.clone(), fence.clone())]);
    let observations = BTreeMap::from([(
        service,
        ProcessObservationClaim::FencedRunning {
            fence,
            probe: ServiceProbeClaim::Healthy,
        },
    )]);
    plan.admit_claimed_snapshot(claims, observations)
        .expect("complete claims admit")
        .reconcile(
            desired,
            ProcessIsolationPolicy::new(
                ProcessIsolationAction::Terminate,
                ProcessIsolationAction::Quarantine,
            ),
        )
}

#[test]
fn resident_session_capacity_rejection_precedes_append_and_wake() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application_with_session_capacity(&port, RingCursorV1::new(0), 1);
    let (workspace, probe) = live_workspace();
    let fixture = admit_and_activate(&mut resident, &workspace, &probe)?;
    let first = session_lease_for_active(&fixture, "session-capacity-a", 10);
    let _ = resident.pin_session(&first)?;
    assert_eq!(port.session_body_count(), 1);
    let before_lifecycle = resident.lifecycle();
    let before_projection = resident.session_pins()?;
    let before_wake = resident.wake_cursor();
    let before_head = resident.journal_head();
    let second = session_lease_for_active(&fixture, "session-capacity-b", 20);

    let error = resident
        .pin_session(&second)
        .expect_err("the explicit session-pin bound must refuse another body");

    assert_eq!(
        error.class(),
        ResidentApplicationErrorClass::ProjectionBoundExceeded
    );
    assert_eq!(port.append_calls(), 4);
    assert_eq!(resident.journal_head(), before_head);
    assert_eq!(resident.wake_cursor(), before_wake);
    assert_eq!(resident.lifecycle(), before_lifecycle);
    assert_eq!(resident.session_pins()?, before_projection);
    assert_eq!(port.session_body_count(), 1);
    Ok(())
}

#[test]
fn session_body_failure_precedes_journal_and_projection() -> TestResult {
    for kind in [
        pale_ports::ControlStoreErrorKind::Storage,
        pale_ports::ControlStoreErrorKind::OutcomeUnknown,
    ] {
        let port = MemoryAuthorityStore::default();
        let mut resident = empty_application(&port, RingCursorV1::new(0));
        let (workspace, probe) = live_workspace();
        let fixture = admit_and_activate(&mut resident, &workspace, &probe)?;
        let lease = session_lease_for_active(&fixture, "session-body-failure", 10);
        let before_lifecycle = resident.lifecycle();
        let before_projection = resident.session_pins()?;
        let before_wake = resident.wake_cursor();
        let before_head = resident.journal_head();
        let before_appends = port.append_calls();
        port.enqueue_session_body(Err(pale_ports::ControlStoreError::new(kind)));

        let error = resident
            .pin_session(&lease)
            .expect_err("unproved body custody cannot release a journal pin");

        assert_eq!(
            error.class(),
            ResidentApplicationErrorClass::DurabilityNoMutation
        );
        assert_eq!(port.session_body_count(), 0);
        assert_eq!(port.append_calls(), before_appends);
        assert_eq!(resident.journal_head(), before_head);
        assert_eq!(resident.wake_cursor(), before_wake);
        assert_eq!(resident.lifecycle(), before_lifecycle);
        assert_eq!(resident.session_pins()?, before_projection);
    }
    Ok(())
}

#[test]
fn ambiguous_session_body_retention_requires_exact_readback_before_pin() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (workspace, probe) = live_workspace();
    let fixture = admit_and_activate(&mut resident, &workspace, &probe)?;
    let lease = session_lease_for_active(&fixture, "session-body-ambiguous", 10);
    let prior_head = resident.journal_head();
    port.enqueue_session_body_retain_then_ambiguous();

    let receipt = resident.pin_session(&lease)?;

    assert_eq!(prior_head.map(JournalSequenceV1::get), Some(3));
    assert_eq!(receipt.journal_sequence().get(), 4);
    assert_eq!(port.session_body_count(), 1);
    assert_eq!(resident.journal_head().map(JournalSequenceV1::get), Some(4));
    assert_eq!(resident.session_pins()?, resident.lifecycle().session_pins);
    Ok(())
}

#[test]
fn session_pin_append_failure_changes_no_semantic_or_mechanical_state() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (workspace, probe) = live_workspace();
    let fixture = admit_and_activate(&mut resident, &workspace, &probe)?;
    let lease = session_lease_for_active(&fixture, "session-no-append", 10);
    let before_lifecycle = resident.lifecycle();
    let before_projection = resident.session_pins()?;
    let before_wake = resident.wake_cursor();
    port.enqueue(Err(DurableAuthorityStoreErrorKind::Unavailable.into()));

    let error = resident
        .pin_session(&lease)
        .expect_err("unavailable custody proves no append");

    assert_eq!(
        error.class(),
        ResidentApplicationErrorClass::DurabilityNoMutation
    );
    assert_eq!(resident.lifecycle(), before_lifecycle);
    assert_eq!(resident.session_pins()?, before_projection);
    assert_eq!(resident.wake_cursor(), before_wake);
    assert_eq!(port.session_body_count(), 1);
    let _ = resident.pin_session(&lease)?;
    assert_eq!(port.session_body_count(), 1);
    assert_eq!(resident.session_pins()?, resident.lifecycle().session_pins);
    Ok(())
}

#[test]
fn session_pin_conflict_closes_current_projection_until_recovery() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (workspace, probe) = live_workspace();
    let fixture = admit_and_activate(&mut resident, &workspace, &probe)?;
    let lease = session_lease_for_active(&fixture, "session-stale-writer", 10);
    let before_lifecycle = resident.lifecycle();
    let before_wake = resident.wake_cursor();
    port.enqueue(Err(DurableAuthorityStoreErrorKind::Conflict.into()));

    let error = resident
        .pin_session(&lease)
        .expect_err("store conflict proves this writer stale");

    assert_eq!(
        error.class(),
        ResidentApplicationErrorClass::DurabilityConflict
    );
    assert_eq!(resident.lifecycle(), before_lifecycle);
    assert_eq!(resident.wake_cursor(), before_wake);
    assert_eq!(
        resident
            .session_pins()
            .expect_err("a stale writer cannot project current session pins")
            .class(),
        ResidentApplicationErrorClass::DurabilityConflict
    );
    assert_eq!(
        ObservationSnapshot::from_resident(&resident)
            .expect_err("a stale writer cannot project a fresh observation")
            .class(),
        ResidentApplicationErrorClass::DurabilityConflict
    );
    let closed = resident
        .pin_session(&lease)
        .expect_err("stale writer remains closed until cold recovery");
    assert_eq!(
        closed.class(),
        ResidentApplicationErrorClass::DurabilityConflict
    );
    assert_eq!(port.append_calls(), 4);
    Ok(())
}

#[test]
fn ambiguous_session_pin_append_rebuilds_only_the_durable_outcome() -> TestResult {
    for commit_before_error in [false, true] {
        let port = MemoryAuthorityStore::default();
        let mut resident = empty_application(&port, RingCursorV1::new(0));
        let (workspace, probe) = live_workspace();
        let fixture = admit_and_activate(&mut resident, &workspace, &probe)?;
        let lease = session_lease_for_active(&fixture, "session-ambiguous", 10);
        let before_lifecycle = resident.lifecycle();
        let before_wake = resident.wake_cursor();
        if commit_before_error {
            port.enqueue_commit_then_ambiguous();
        } else {
            port.enqueue(Err(DurableAuthorityStoreErrorKind::AmbiguousCommit.into()));
        }

        let error = resident
            .pin_session(&lease)
            .expect_err("ambiguous append cannot authorize a local projection");

        assert_eq!(
            error.class(),
            ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        );
        assert_eq!(resident.lifecycle(), before_lifecycle);
        assert_eq!(
            resident
                .session_pins()
                .expect_err("unknown durable authority cannot be projected as current")
                .class(),
            ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        );
        assert_eq!(
            ObservationSnapshot::from_resident(&resident)
                .expect_err("unknown durable authority cannot be observed as fresh")
                .class(),
            ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        );
        assert_eq!(resident.wake_cursor(), before_wake);
        assert_eq!(port.append_calls(), 4);
        let closed = resident
            .pin_session(&lease)
            .expect_err("unknown append outcome closes the writer");
        assert_eq!(
            closed.class(),
            ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        );
        assert_eq!(port.append_calls(), 4);

        let input = reconstruction_input(&workspace, &probe, &fixture);
        let resolver = resolver_for_active(&fixture);
        drop(resident);
        let mut source = FixedInputSource {
            requested: None,
            calls: 0,
            inputs: vec![input],
        };
        let opened = ResidentApplication::cold_open(
            &port,
            &port,
            &mut UnavailableEventCustody,
            limits().maximum_entry_bytes(),
            ResidentColdOpenInput::new(
                store(),
                limits(),
                limits().maximum_entry_bytes(),
                RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                &resolver,
                &mut source,
                parameters(2),
            ),
        )?;
        let ResidentColdOpenOutcome::Ready(recovered) = opened else {
            return Err(std::io::Error::other("valid ambiguous outcome was quarantined").into());
        };
        assert_eq!(
            recovered.journal_head().map(JournalSequenceV1::get),
            Some(if commit_before_error { 4 } else { 3 })
        );
        assert_eq!(
            recovered
                .session_pins()?
                .contains_key(lease.session_admission_id()),
            commit_before_error
        );
        assert_eq!(
            recovered.session_pins()?,
            recovered.lifecycle().session_pins
        );
    }
    Ok(())
}

#[test]
fn session_drain_append_failure_changes_no_semantic_or_mechanical_state() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (workspace, probe) = live_workspace();
    let fixture = admit_and_activate(&mut resident, &workspace, &probe)?;
    let lease = session_lease_for_active(&fixture, "session-drain-no-append", 10);
    let _ = resident.pin_session(&lease)?;
    let before_lifecycle = resident.lifecycle();
    let before_projection = resident.session_pins()?;
    let before_wake = resident.wake_cursor();
    port.enqueue(Err(DurableAuthorityStoreErrorKind::Unavailable.into()));

    let error = resident
        .drain_session(
            lease.session_admission_id(),
            lease.session_admission_lease_digest(),
        )
        .expect_err("unavailable custody proves no drain append");

    assert_eq!(
        error.class(),
        ResidentApplicationErrorClass::DurabilityNoMutation
    );
    assert_eq!(resident.lifecycle(), before_lifecycle);
    assert_eq!(resident.session_pins()?, before_projection);
    assert_eq!(resident.wake_cursor(), before_wake);
    let _ = resident.drain_session(
        lease.session_admission_id(),
        lease.session_admission_lease_digest(),
    )?;
    assert!(resident.session_pins()?.is_empty());
    Ok(())
}

#[test]
fn ambiguous_session_drain_rebuilds_only_the_durable_outcome() -> TestResult {
    for commit_before_error in [false, true] {
        let port = MemoryAuthorityStore::default();
        let mut resident = empty_application(&port, RingCursorV1::new(0));
        let (workspace, probe) = live_workspace();
        let fixture = admit_and_activate(&mut resident, &workspace, &probe)?;
        let lease = session_lease_for_active(&fixture, "session-drain-ambiguous", 10);
        let _ = resident.pin_session(&lease)?;
        let before_lifecycle = resident.lifecycle();
        let before_wake = resident.wake_cursor();
        if commit_before_error {
            port.enqueue_commit_then_ambiguous();
        } else {
            port.enqueue(Err(DurableAuthorityStoreErrorKind::AmbiguousCommit.into()));
        }

        let error = resident
            .drain_session(
                lease.session_admission_id(),
                lease.session_admission_lease_digest(),
            )
            .expect_err("ambiguous drain cannot authorize a local projection");

        assert_eq!(
            error.class(),
            ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        );
        assert_eq!(resident.lifecycle(), before_lifecycle);
        assert_eq!(
            resident
                .session_pins()
                .expect_err("unknown drain outcome cannot be projected as current")
                .class(),
            ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        );
        assert_eq!(resident.wake_cursor(), before_wake);
        assert_eq!(port.append_calls(), 5);

        let input = reconstruction_input(&workspace, &probe, &fixture);
        let resolver = resolver_for_active(&fixture);
        drop(resident);
        let mut source = FixedInputSource {
            requested: None,
            calls: 0,
            inputs: vec![input],
        };
        let opened = ResidentApplication::cold_open(
            &port,
            &port,
            &mut UnavailableEventCustody,
            limits().maximum_entry_bytes(),
            ResidentColdOpenInput::new(
                store(),
                limits(),
                limits().maximum_entry_bytes(),
                RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                &resolver,
                &mut source,
                parameters(3),
            ),
        )?;
        let ResidentColdOpenOutcome::Ready(recovered) = opened else {
            return Err(std::io::Error::other("valid ambiguous drain was quarantined").into());
        };
        assert_eq!(
            recovered.journal_head().map(JournalSequenceV1::get),
            Some(if commit_before_error { 5 } else { 4 })
        );
        assert_eq!(
            recovered
                .session_pins()?
                .contains_key(lease.session_admission_id()),
            !commit_before_error
        );
        assert_eq!(
            recovered.session_pins()?,
            recovered.lifecycle().session_pins
        );
    }
    Ok(())
}

#[test]
fn session_pin_apply_precedes_even_a_failed_wake_projection() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(u64::MAX));
    let (workspace, probe) = live_workspace();
    let fixture = admit_and_activate(&mut resident, &workspace, &probe)?;
    let lease = session_lease_for_active(&fixture, "session-wake-replay", 10);
    let wake_before = resident.wake_cursor();

    let pin = resident.pin_session(&lease)?;

    assert!(matches!(
        pin.wake(),
        ResidentWakeDisposition::DurableReplayRequired { journal_sequence, .. }
            if journal_sequence.get() == 4
    ));
    assert_eq!(resident.wake_cursor(), wake_before);
    assert_eq!(resident.session_pins()?, resident.lifecycle().session_pins);

    let drain = resident.drain_session(
        lease.session_admission_id(),
        lease.session_admission_lease_digest(),
    )?;

    assert!(matches!(
        drain.wake(),
        ResidentWakeDisposition::DurableReplayRequired { journal_sequence, .. }
            if journal_sequence.get() == 5
    ));
    assert_eq!(resident.wake_cursor(), wake_before);
    assert!(resident.lifecycle().session_pins.is_empty());
    assert!(resident.session_pins()?.is_empty());
    Ok(())
}

#[test]
fn post_append_projection_failure_suppresses_wake_and_closes_mutation() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (workspace, probe) = live_workspace();
    let fixture = admit_and_activate(&mut resident, &workspace, &probe)?;
    let lease = session_lease_for_active(&fixture, "session-projection-poison", 10);
    let _ = resident.pin_session(&lease)?;
    let transition = resident.coordinator.lifecycle().drain_session(
        lease.session_admission_id(),
        lease.session_admission_lease_digest(),
    )?;
    let applied = resident.sessions.prepare_drain(
        lease.session_admission_id(),
        lease.session_admission_lease_digest(),
    )?;
    let stale = resident.sessions.prepare_drain(
        lease.session_admission_id(),
        lease.session_admission_lease_digest(),
    )?;
    resident.sessions.apply(applied)?;
    let wake_before = resident.wake_cursor();

    let error = resident
        .commit_session_pin_projection(transition, stale)
        .expect_err("stale prepared apply must require cold recovery");

    assert_eq!(
        error.class(),
        ResidentApplicationErrorClass::ProjectionRecoveryRequired
    );
    assert_eq!(resident.journal_head().map(JournalSequenceV1::get), Some(5));
    assert_eq!(resident.wake_cursor(), wake_before);
    assert_eq!(port.append_calls(), 5);
    let closed = resident
        .pin_session(&lease)
        .expect_err("projection failure must close later mutation");
    assert_eq!(
        closed.class(),
        ResidentApplicationErrorClass::ProjectionRecoveryRequired
    );
    assert_eq!(
        ObservationSnapshot::from_resident(&resident)
            .expect_err("observation cannot expose a half-applied resident")
            .class(),
        ResidentApplicationErrorClass::ProjectionRecoveryRequired
    );
    assert_eq!(port.append_calls(), 5);

    let input = reconstruction_input(&workspace, &probe, &fixture);
    let resolver = resolver_for_active(&fixture);
    drop(resident);
    let mut source = FixedInputSource {
        requested: None,
        calls: 0,
        inputs: vec![input],
    };
    let opened = ResidentApplication::cold_open(
        &port,
        &port,
        &mut UnavailableEventCustody,
        limits().maximum_entry_bytes(),
        ResidentColdOpenInput::new(
            store(),
            limits(),
            limits().maximum_entry_bytes(),
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            &resolver,
            &mut source,
            parameters(3),
        ),
    )?;
    let ResidentColdOpenOutcome::Ready(recovered) = opened else {
        return Err(std::io::Error::other("committed drain did not recover").into());
    };
    assert!(recovered.lifecycle().session_pins.is_empty());
    assert!(recovered.session_pins()?.is_empty());
    let _ = ObservationSnapshot::from_resident(&recovered)?;
    Ok(())
}

#[test]
fn desired_state_commit_is_the_only_path_to_process_directives() {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (workspace, probe) = live_workspace();
    let fixture = admit_and_activate(&mut resident, &workspace, &probe)
        .expect("actual compilation admits and activates");
    let generation = fixture.generation.compiled_generation_digest();
    let prior_appends = port.append_calls();

    let committed = resident
        .commit_supervisor_reconciliation(pending_reconciliation(
            generation,
            DesiredProcessState::Running,
        ))
        .expect("desired state commits before directives");
    match committed.persistence() {
        ResidentDesiredStatePersistence::Committed(receipt) => {
            assert_eq!(receipt.journal_sequence().get(), 4);
            assert_eq!(
                receipt.desired_process_state(),
                Some(DesiredProcessState::Running)
            );
        }
        ResidentDesiredStatePersistence::AlreadyCurrent { .. } => {
            panic!("first desired state cannot already be current");
        }
    }
    assert_eq!(
        committed.reconciliation().desired_process_state(),
        DesiredProcessState::Running
    );
    assert!(matches!(
        committed.reconciliation().steps()[0],
        SupervisorReconciliationStep::ApplyStopSentinel(StopSentinelIntent::EnsureAbsent)
    ));
    assert_eq!(committed.reconciliation().compiled_generation(), generation);
    assert_eq!(port.append_calls(), prior_appends + 1);

    let repeated = resident
        .commit_supervisor_reconciliation(pending_reconciliation(
            generation,
            DesiredProcessState::Running,
        ))
        .expect("same desired state reuses its frontier");
    assert!(matches!(
        repeated.persistence(),
        ResidentDesiredStatePersistence::AlreadyCurrent { journal_sequence }
            if journal_sequence.get() == 4
    ));
    assert_eq!(
        port.append_calls(),
        prior_appends + 1,
        "idempotence emits no journal noise"
    );

    port.enqueue(Err(DurableAuthorityStoreErrorKind::Unavailable.into()));
    let error = resident
        .commit_supervisor_reconciliation(pending_reconciliation(
            generation,
            DesiredProcessState::Stopped,
        ))
        .expect_err("unproven persistence cannot release stop directives");
    assert_eq!(
        error.class(),
        ResidentApplicationErrorClass::DurabilityNoMutation
    );
    assert_eq!(
        resident.desired_process_state(),
        Some(DesiredProcessState::Running)
    );
    assert_eq!(port.append_calls(), prior_appends + 2);
}

#[test]
fn supervisor_running_requires_an_active_generation_before_persistence() {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let unknown = CompiledGenerationDigest::parse("a".repeat(64)).expect("synthetic digest");
    let error = resident
        .commit_supervisor_reconciliation(pending_reconciliation(
            &unknown,
            DesiredProcessState::Running,
        ))
        .expect_err("an arbitrary generation cannot start an empty resident");
    assert!(matches!(
        error.0,
        ResidentApplicationErrorKind::Lifecycle(GenerationLifecycleError::MissingPrecondition(_))
    ));
    assert_eq!(port.append_calls(), 0);
    assert_eq!(resident.desired_process_state(), None);

    let (workspace, probe) = live_workspace();
    let fixture = admit_and_activate(&mut resident, &workspace, &probe)
        .expect("actual compilation admits and activates");
    let generation = fixture.generation.compiled_generation_digest();
    let prior_appends = port.append_calls();
    let prior_cursor = resident.wake_cursor();
    let _ = resident
        .generations
        .remove(generation)
        .expect("remove retained body for fault test");
    let error = resident
        .commit_supervisor_reconciliation(pending_reconciliation(
            generation,
            DesiredProcessState::Running,
        ))
        .expect_err("active digest without body custody cannot release running intents");
    assert_eq!(
        error.class(),
        ResidentApplicationErrorClass::MissingGeneration
    );
    assert_eq!(port.append_calls(), prior_appends);
    assert_eq!(resident.wake_cursor(), prior_cursor);
    assert_eq!(resident.desired_process_state(), None);
}

#[test]
fn supervisor_rejects_old_running_plans_but_preserves_old_stopped_intents() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let prior = activate_successor(&mut resident)?;
    let active = resident
        .active_generation()
        .expect("successor active")
        .record()
        .compiled_generation_digest()
        .clone();
    let _ = resident.commit_supervisor_reconciliation(pending_reconciliation(
        &active,
        DesiredProcessState::Running,
    ))?;
    let prior_appends = port.append_calls();
    let prior_head = resident.journal_head();
    let prior_cursor = resident.wake_cursor();
    for generation in [
        prior.clone(),
        CompiledGenerationDigest::parse("a".repeat(64))?,
    ] {
        let error = resident
            .commit_supervisor_reconciliation(pending_reconciliation(
                &generation,
                DesiredProcessState::Running,
            ))
            .expect_err("already-running desired state cannot bypass generation comparison");
        assert!(matches!(
            error.0,
            ResidentApplicationErrorKind::Lifecycle(GenerationLifecycleError::CoordinateMismatch(
                _
            ))
        ));
        assert_eq!(port.append_calls(), prior_appends);
        assert_eq!(resident.journal_head(), prior_head);
        assert_eq!(resident.wake_cursor(), prior_cursor);
    }
    let stopped = resident.commit_supervisor_reconciliation(pending_reconciliation(
        &prior,
        DesiredProcessState::Stopped,
    ))?;
    assert_eq!(stopped.reconciliation().compiled_generation(), &prior);
    assert!(matches!(
        stopped.reconciliation().steps()[0],
        SupervisorReconciliationStep::ApplyStopSentinel(StopSentinelIntent::EnsurePresent)
    ));
    assert_eq!(
        resident
            .active_generation()
            .expect("active body retained")
            .record()
            .compiled_generation_digest(),
        &active
    );
    assert_eq!(port.append_calls(), prior_appends + 1);
    Ok(())
}

fn activate_successor(
    resident: &mut ResidentApplication<'_>,
) -> Result<CompiledGenerationDigest, ResidentApplicationError> {
    let (workspace, probe) = live_workspace();
    let first_compilation = compile_live(&workspace, &probe);
    let first_materializations = vec![applied_evidence(&first_compilation).into_materialization()];
    let first = resident.admit_generation(
        first_compilation,
        first_materializations,
        synthetic_set_policy(),
    )?;
    let first_record = resident
        .generation(&first)
        .expect("admitted first generation")
        .record()
        .clone();
    let first_published = published(&first_record, 1, None);
    assert_eq!(
        resident
            .publish(first_published.clone())?
            .journal_sequence()
            .get(),
        1
    );
    let first_advertised = advertised(&first_published, 2, None);
    assert_eq!(
        resident
            .advertise(first_advertised.clone())?
            .journal_sequence()
            .get(),
        2
    );
    let first_fence = fence(&first_record, 1, None);
    let first_active = active(&first_advertised, &first_fence, 3, None);
    assert_eq!(
        resident
            .activate(first_active.clone(), first_fence.clone())?
            .journal_sequence()
            .get(),
        3
    );

    retarget_workspace_identity(&workspace, "resident-application-successor");
    let second_compilation = compile_live(&workspace, &probe);
    let second_materializations =
        vec![applied_evidence(&second_compilation).into_materialization()];
    let second = resident.admit_generation(
        second_compilation,
        second_materializations,
        synthetic_set_policy(),
    )?;
    let second_record = resident
        .generation(&second)
        .expect("admitted successor generation")
        .record()
        .clone();
    let second_published = published(&second_record, 4, Some(&first_published));
    assert_eq!(
        resident
            .publish(second_published.clone())?
            .journal_sequence()
            .get(),
        4
    );
    let second_advertised = advertised(&second_published, 5, Some(&first_advertised));
    assert_eq!(
        resident
            .advertise(second_advertised.clone())?
            .journal_sequence()
            .get(),
        5
    );
    let second_fence = fence(&second_record, 2, Some(&first_fence));
    let second_active = active(&second_advertised, &second_fence, 6, Some(&first_active));
    assert_eq!(
        resident
            .activate(second_active, second_fence)?
            .journal_sequence()
            .get(),
        6
    );
    assert_eq!(
        resident.reclamation_disposition(&first),
        ReclamationDisposition::Eligible
    );
    Ok(first)
}

#[test]
fn compiled_generations_preserve_session_custody_across_forward_rollback() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (workspace, probe) = live_workspace();
    let first = admit_and_activate(&mut resident, &workspace, &probe)?;
    let first_digest = first.generation.compiled_generation_digest().clone();
    let first_lease = session_lease_for_active(&first, "session-before-successor", 10);
    let _ = resident.pin_session(&first_lease)?;
    let _ = resident.retain_for_rollback(first_digest.clone())?;

    retarget_workspace_identity(&workspace, "resident-forward-rollback");
    let second_compilation = compile_live(&workspace, &probe);
    let second_profile = second_compilation.profile().clone();
    let second_materializations =
        vec![applied_evidence(&second_compilation).into_materialization()];
    let second_digest = resident.admit_generation(
        second_compilation,
        second_materializations,
        synthetic_set_policy(),
    )?;
    let second_generation = resident
        .generation(&second_digest)
        .expect("admitted successor generation")
        .record()
        .clone();
    let second_published = published(&second_generation, 6, Some(&first.published));
    let _ = resident.publish(second_published.clone())?;
    let second_advertised = advertised(&second_published, 7, Some(&first.advertised));
    let _ = resident.advertise(second_advertised.clone())?;
    let second_fence = fence(&second_generation, 2, Some(&first.fence));
    let second_active = active(&second_advertised, &second_fence, 8, Some(&first.active));
    let _ = resident.activate(second_active.clone(), second_fence.clone())?;

    assert_eq!(
        resident.session_pins()?[first_lease.session_admission_id()].generation,
        first_digest
    );
    let stale_lease = session_lease_for_profile(
        &first.generation,
        &first.active,
        &first.fence,
        &first.profile,
        "session-stale-after-successor",
        10,
    );
    let before_stale_attempt = resident.journal_head();
    let _ = resident
        .pin_session(&stale_lease)
        .expect_err("a prior active coordinate cannot admit a new session");
    assert_eq!(resident.journal_head(), before_stale_attempt);

    let second_fixture = ActiveResidentFixture {
        generation: second_generation,
        profile: second_profile,
        published: second_published.clone(),
        advertised: second_advertised.clone(),
        fence: second_fence.clone(),
        active: second_active.clone(),
    };
    let second_lease = session_lease_for_active(&second_fixture, "session-on-successor", 10);
    let _ = resident.pin_session(&second_lease)?;

    let rollback_published = published(&first.generation, 10, Some(&second_published));
    let _ = resident.publish(rollback_published.clone())?;
    let rollback_advertised = advertised(&rollback_published, 11, Some(&second_advertised));
    let _ = resident.advertise(rollback_advertised.clone())?;
    let rollback_fence = fence(&first.generation, 3, Some(&second_fence));
    let rollback_active = active(
        &rollback_advertised,
        &rollback_fence,
        12,
        Some(&second_active),
    );
    let _ = resident.rollback(rollback_active.clone(), rollback_fence.clone())?;

    let rollback_fixture = ActiveResidentFixture {
        generation: first.generation.clone(),
        profile: first.profile.clone(),
        published: rollback_published,
        advertised: rollback_advertised,
        fence: rollback_fence,
        active: rollback_active,
    };
    let rebound_lease = session_lease_for_active(&rollback_fixture, "session-after-rollback", 10);
    let _ = resident.pin_session(&rebound_lease)?;

    let pins = resident.session_pins()?;
    assert_eq!(
        pins[first_lease.session_admission_id()].generation,
        first_digest
    );
    assert_eq!(
        pins[second_lease.session_admission_id()].generation,
        second_digest
    );
    assert_eq!(
        pins[rebound_lease.session_admission_id()].generation,
        *rollback_fixture.generation.compiled_generation_digest()
    );
    assert_eq!(
        resident
            .active_generation()
            .map(|generation| generation.record().compiled_generation_digest()),
        Some(rollback_fixture.generation.compiled_generation_digest())
    );
    Ok(())
}

#[test]
fn application_root_owns_generation_authority_checkpoint_and_cold_rebuild() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let resolver = RecoveryResolver::default();
    let mut empty_source = FixedInputSource::default();
    let opened = ResidentApplication::cold_open(
        &port,
        &port,
        &mut UnavailableEventCustody,
        limits().maximum_entry_bytes(),
        ResidentColdOpenInput::new(
            store(),
            limits(),
            limits().maximum_entry_bytes(),
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            &resolver,
            &mut empty_source,
            parameters(1),
        ),
    )?;
    let ResidentColdOpenOutcome::Ready(mut resident) = opened else {
        return Err(std::io::Error::other("empty authority was quarantined").into());
    };
    assert_eq!(empty_source.calls, 1);

    let (workspace, probe) = live_workspace();
    let compilation = compile_live(&workspace, &probe);
    let materializations = vec![applied_evidence(&compilation).into_materialization()];
    let generation =
        resident.admit_generation(compilation, materializations, synthetic_set_policy())?;
    let record = resident
        .generation(&generation)
        .ok_or_else(|| std::io::Error::other("admitted generation is absent"))?
        .record()
        .clone();

    let published_head = published(&record, 1, None);
    let published_receipt = resident.publish(published_head.clone())?;
    assert_eq!(published_receipt.journal_sequence().get(), 1);
    let advertised_head = advertised(&published_head, 2, None);
    let advertised_receipt = resident.advertise(advertised_head.clone())?;
    assert_eq!(advertised_receipt.journal_sequence().get(), 2);
    let activation_fence = fence(&record, 1, None);
    let active_head = active(&advertised_head, &activation_fence, 3, None);
    let active_receipt = resident.activate(active_head.clone(), activation_fence.clone())?;
    assert_eq!(active_receipt.journal_sequence().get(), 3);
    assert_eq!(
        resident
            .active_generation()
            .map(AdmittedCompiledGeneration::record),
        Some(&record)
    );
    let running = resident.commit_supervisor_reconciliation(pending_reconciliation(
        &generation,
        DesiredProcessState::Running,
    ))?;
    assert_eq!(running.reconciliation().compiled_generation(), &generation);
    resident.checkpoint()?;
    drop(resident);

    let rebuild_compilation = compile_live(&workspace, &probe);
    let rebuild_materializations =
        vec![applied_evidence(&rebuild_compilation).into_materialization()];
    let mut recovery_source = FixedInputSource {
        requested: None,
        calls: 0,
        inputs: vec![ResidentGenerationReconstructionInput::new(
            generation.clone(),
            rebuild_compilation,
            rebuild_materializations,
        )],
    };
    let mut recovery_resolver = RecoveryResolver::default();
    recovery_resolver.insert_generation(&record);
    recovery_resolver.insert_published(&published_head);
    recovery_resolver.insert_advertised(&advertised_head);
    recovery_resolver.insert_fence(&activation_fence);
    recovery_resolver.insert_active(&active_head);
    let reopened = ResidentApplication::cold_open(
        &port,
        &port,
        &mut UnavailableEventCustody,
        limits().maximum_entry_bytes(),
        ResidentColdOpenInput::new(
            store(),
            limits(),
            limits().maximum_entry_bytes(),
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            &recovery_resolver,
            &mut recovery_source,
            parameters(2),
        ),
    )?;
    let ResidentColdOpenOutcome::Ready(mut recovered) = reopened else {
        return Err(std::io::Error::other("complete authority was quarantined").into());
    };
    assert_eq!(recovery_source.calls, 1);
    assert_eq!(recovered.journal_head().map(|value| value.get()), Some(4));
    assert_eq!(
        recovered
            .active_generation()
            .map(AdmittedCompiledGeneration::record),
        Some(&record)
    );
    assert_eq!(recovered.wake_cursor().epoch(), WakeEpochV1::new(2));
    let prior_appends = port.append_calls();
    let running = recovered.commit_supervisor_reconciliation(pending_reconciliation(
        &generation,
        DesiredProcessState::Running,
    ))?;
    assert_eq!(running.reconciliation().compiled_generation(), &generation);
    assert!(matches!(
        running.persistence(),
        ResidentDesiredStatePersistence::AlreadyCurrent { journal_sequence }
            if journal_sequence.get() == 4
    ));
    assert_eq!(port.append_calls(), prior_appends);
    Ok(())
}

#[test]
fn resident_observation_projects_durable_frontier_without_inventing_liveness() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let before_admission = ObservationSnapshot::from_resident(&resident)?;
    let page_request = ObservationPageRequest::new(ObservationFilter::all(), 64);
    let before_page = ready_page(
        before_admission
            .page(page_request.clone())
            .expect("initial page"),
    );
    let (workspace, probe) = live_workspace();
    let compilation = compile_live(&workspace, &probe);
    let materializations = vec![applied_evidence(&compilation).into_materialization()];
    let generation =
        resident.admit_generation(compilation, materializations, synthetic_set_policy())?;
    let unpublished = ObservationSnapshot::from_resident(&resident)?;
    assert_eq!(unpublished.cursor(), before_admission.cursor());
    assert_eq!(
        ready_page(
            unpublished
                .page(page_request.clone())
                .expect("unpublished page"),
        ),
        before_page
    );
    assert_eq!(
        unpublished.omitted_fact_count(),
        before_admission.omitted_fact_count()
    );
    assert!(matches!(
        unpublished.delta_from(
            &before_admission,
            ObservationDeltaRequest {
                resume: before_admission.resume_coordinate(),
                maximum_changes: 8,
                cancellation: ObservationCancellation::Active,
            }
        ),
        ObservationDeltaOutcome::CaughtUp(_)
    ));
    let record = resident
        .generation(&generation)
        .ok_or_else(|| std::io::Error::other("admitted generation is absent"))?
        .record()
        .clone();
    let published_head = published(&record, 1, None);
    assert_eq!(
        resident
            .publish(published_head.clone())?
            .journal_sequence()
            .get(),
        1
    );
    let published_snapshot = ObservationSnapshot::from_resident(&resident)?;
    let ObservationDeltaOutcome::Delta(delta) = published_snapshot.delta_from(
        &unpublished,
        ObservationDeltaRequest {
            resume: unpublished.resume_coordinate(),
            maximum_changes: 8,
            cancellation: ObservationCancellation::Active,
        },
    ) else {
        panic!("publication must expose the complete change at its journal revision");
    };
    let published_page = ready_page(
        published_snapshot
            .page(page_request)
            .expect("published page"),
    );
    assert_eq!(delta.from, *unpublished.cursor());
    assert_eq!(delta.to, *published_snapshot.cursor());
    assert_eq!(delta.changes.len(), 2);
    assert_eq!(
        delta.changes,
        published_page
            .facts
            .into_iter()
            .map(ObservationChange::Upsert)
            .collect::<Vec<_>>()
    );
    assert!(matches!(
        published_snapshot.delta_from(
            &unpublished,
            ObservationDeltaRequest {
                resume: unpublished.resume_coordinate(),
                maximum_changes: 1,
                cancellation: ObservationCancellation::Active,
            }
        ),
        ObservationDeltaOutcome::Backpressured {
            pending_changes: 2,
            maximum_changes: 1,
            recovery: ObservationBackpressureRecovery::RequestSnapshot,
            ..
        }
    ));
    let advertised_head = advertised(&published_head, 2, None);
    assert_eq!(
        resident
            .advertise(advertised_head.clone())?
            .journal_sequence()
            .get(),
        2
    );
    let activation_fence = fence(&record, 1, None);
    assert_eq!(
        resident
            .activate(
                active(&advertised_head, &activation_fence, 3, None),
                activation_fence,
            )?
            .journal_sequence()
            .get(),
        3
    );
    resident.commit_supervisor_reconciliation(pending_reconciliation(
        &generation,
        DesiredProcessState::Running,
    ))?;

    let snapshot = ObservationSnapshot::from_resident(&resident)?;
    assert_eq!(
        snapshot.cursor().generation(),
        &ObservationGeneration::Resident {
            authority_store: store(),
            active_generation: Some(generation.clone()),
        }
    );
    assert_eq!(
        snapshot.cursor().revision(),
        &ObservationRevision::ResidentJournal(Some(
            pale_core::records::JournalSequenceV1::new(4)
                .expect("fourth durable resident transition"),
        ))
    );

    let page = ready_page(
        snapshot
            .page(ObservationPageRequest::new(ObservationFilter::all(), 64))
            .expect("a first-page request has no foreign continuation"),
    );
    assert_eq!(page.snapshot_omitted_count, 0);
    assert!(page.facts.iter().any(|fact| matches!(
        fact,
        ObservationFact::Readiness {
            subject: ObservationSubject::CompiledGeneration(digest),
            state: ObservationReadinessState::Unproven,
            freshness: ObservationFreshness::Fresh,
        } if digest == &generation
    )));
    assert!(page.facts.iter().any(|fact| matches!(
        fact,
        ObservationFact::NativeOwner {
            subject: ObservationSubject::CompiledGeneration(digest),
            state: ObservationNativeState::Unknown,
            freshness: ObservationFreshness::Unknown,
        } if digest == &generation
    )));
    assert!(page.facts.iter().any(|fact| matches!(
        fact,
        ObservationFact::ProcessIntent {
            state: ObservationProcessIntent::Running,
            freshness: ObservationFreshness::Fresh,
        }
    )));
    assert!(page.facts.iter().any(|fact| matches!(
        fact,
        ObservationFact::Health {
            subject: ObservationSubject::ResidentProcess,
            state: ObservationHealthState::Unknown,
            freshness: ObservationFreshness::Unknown,
        }
    )));
    assert_eq!(
        page.facts
            .iter()
            .filter(|fact| fact.category() == ObservationCategory::NativeOwner)
            .count(),
        1
    );
    Ok(())
}

#[test]
fn resident_observation_survives_cold_reopen_without_unpublished_candidates() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (workspace, probe) = live_workspace();
    let fixture = admit_and_activate(&mut resident, &workspace, &probe)?;
    resident.checkpoint()?;
    // Retain the original reconstruction input before changing the temporary source.
    let input = reconstruction_input(&workspace, &probe, &fixture);
    let resolver = resolver_for_active(&fixture);
    let before = ObservationSnapshot::from_resident(&resident)?;
    let request = ObservationPageRequest::new(ObservationFilter::all(), 1);
    let first = ready_page(before.page(request.clone()).expect("first retained fact"));
    let continuation = request
        .clone()
        .after(first.continuation.clone().expect("second retained fact"));
    let second = ready_page(
        before
            .page(continuation.clone())
            .expect("continued retained fact"),
    );
    assert!(second.continuation.is_some());

    retarget_workspace_identity(&workspace, "resident-unpublished-successor");
    let candidate_compilation = compile_live(&workspace, &probe);
    let materializations = vec![applied_evidence(&candidate_compilation).into_materialization()];
    let candidate = resident.admit_generation(
        candidate_compilation,
        materializations,
        synthetic_set_policy(),
    )?;
    assert_ne!(&candidate, fixture.generation.compiled_generation_digest());
    assert!(resident.generation(&candidate).is_some());
    let unpublished = ObservationSnapshot::from_resident(&resident)?;
    assert_eq!(unpublished.cursor(), before.cursor());
    assert_eq!(
        ready_page(unpublished.page(request.clone()).expect("same first page")),
        first
    );
    assert_eq!(
        ready_page(
            unpublished
                .page(continuation.clone())
                .expect("same continuation"),
        ),
        second
    );
    assert_eq!(
        unpublished.omitted_fact_count(),
        before.omitted_fact_count()
    );
    drop(resident);

    let mut source = FixedInputSource {
        requested: None,
        calls: 0,
        inputs: vec![input],
    };
    let opened = ResidentApplication::cold_open(
        &port,
        &port,
        &mut UnavailableEventCustody,
        limits().maximum_entry_bytes(),
        ResidentColdOpenInput::new(
            store(),
            limits(),
            limits().maximum_entry_bytes(),
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            &resolver,
            &mut source,
            parameters(2),
        ),
    )?;
    let ResidentColdOpenOutcome::Ready(recovered) = opened else {
        return Err(std::io::Error::other("checkpointed authority was quarantined").into());
    };
    assert_eq!(source.calls, 1);
    assert_eq!(
        source.requested,
        Some(BTreeSet::from([fixture
            .generation
            .compiled_generation_digest()
            .clone(),]))
    );
    assert!(recovered.generation(&candidate).is_none());
    let reopened = ObservationSnapshot::from_resident(&recovered)?;
    assert_eq!(reopened.cursor(), before.cursor());
    assert_eq!(
        ready_page(reopened.page(request).expect("reopened first page")),
        first
    );
    assert_eq!(
        ready_page(reopened.page(continuation).expect("reopened continuation"),),
        second
    );
    assert_eq!(reopened.omitted_fact_count(), before.omitted_fact_count());
    Ok(())
}

#[test]
fn error_classes_preserve_durability_and_coordination_causes() {
    let durable = ResidentApplicationError::from(ResidentAuthorityCoordinationError::Journal(
        AuthorityJournalError::Store(DurableAuthorityStoreError::with_source(
            DurableAuthorityStoreErrorKind::Unavailable,
            std::io::Error::other("native store unavailable"),
        )),
    ));
    assert_eq!(
        durable.class(),
        ResidentApplicationErrorClass::DurabilityNoMutation
    );
    assert!(durable.source().is_some());
    let mut source = Some(&durable as &(dyn std::error::Error + 'static));
    let native = loop {
        let error = source.expect("native source remains in the typed chain");
        if let Some(native) = error.downcast_ref::<std::io::Error>() {
            break native;
        }
        source = error.source();
    };
    assert_eq!(native.to_string(), "native store unavailable");

    let coordination =
        ResidentApplicationError::from(ResidentAuthorityCoordinationError::FrontierMismatch {
            journal_head: None,
            wake_head: pale_core::records::JournalSequenceV1::new(1).ok(),
        });
    assert_eq!(
        coordination.class(),
        ResidentApplicationErrorClass::Coordination
    );
    assert!(coordination.source().is_some());
}

#[test]
fn durability_classes_preserve_retry_and_effect_certainty() {
    use crate::resident::event_custody::journal::EventJournalCustodyError;

    let classify = |source| {
        ResidentApplicationError::from(ResidentAuthorityCoordinationError::Journal(
            AuthorityJournalError::Store(source),
        ))
        .class()
    };

    assert_eq!(
        classify(DurableAuthorityStoreErrorKind::Conflict.into()),
        ResidentApplicationErrorClass::DurabilityConflict
    );
    assert_eq!(
        classify(DurableAuthorityStoreErrorKind::AmbiguousCommit.into()),
        ResidentApplicationErrorClass::DurabilityOutcomeUnknown
    );
    assert_eq!(
        classify(
            DurableAuthorityStoreErrorKind::Corrupt(pale_ports::AuthorityByteClass::JournalEntry,)
                .into()
        ),
        ResidentApplicationErrorClass::DurabilityIntegrity
    );
    assert_eq!(
        classify(
            DurableAuthorityStoreErrorKind::BoundExceeded {
                class: pale_ports::AuthorityByteClass::JournalEntry,
            }
            .into()
        ),
        ResidentApplicationErrorClass::DurabilityBoundExceeded
    );
    assert_eq!(
        classify(DurableAuthorityStoreErrorKind::Unavailable.into()),
        ResidentApplicationErrorClass::DurabilityNoMutation
    );
    let checkpoint = ResidentApplicationError::from(ResidentAuthorityCoordinationError::Journal(
        AuthorityJournalError::CheckpointProjectionOutcomeUnknown(
            DurableAuthorityStoreErrorKind::AmbiguousCommit.into(),
        ),
    ));
    assert_eq!(
        checkpoint.class(),
        ResidentApplicationErrorClass::CheckpointProjectionOutcomeUnknown
    );
    for (error, expected) in [
        (
            EventJournalCustodyError::BoundExceeded,
            ResidentApplicationErrorClass::DurabilityBoundExceeded,
        ),
        (
            EventJournalCustodyError::Candidate(
                pale_ports::EventCustodyCandidateError::HistoryByteLimit,
            ),
            ResidentApplicationErrorClass::DurabilityBoundExceeded,
        ),
        (
            EventJournalCustodyError::Store(pale_ports::ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::Storage,
            )),
            ResidentApplicationErrorClass::DurabilityNoMutation,
        ),
        (
            EventJournalCustodyError::Correspondence,
            ResidentApplicationErrorClass::DurabilityIntegrity,
        ),
    ] {
        let error = ResidentApplicationError::from(ResidentAuthorityCoordinationError::Journal(
            AuthorityJournalError::EventCustody(Box::new(error)),
        ));
        assert_eq!(error.class(), expected);
    }
}

#[test]
fn failed_publish_preserves_candidate_body_and_authority_frontier() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let resolver = RecoveryResolver::default();
    let mut source = FixedInputSource::default();
    let opened = ResidentApplication::cold_open(
        &port,
        &port,
        &mut UnavailableEventCustody,
        limits().maximum_entry_bytes(),
        ResidentColdOpenInput::new(
            store(),
            limits(),
            limits().maximum_entry_bytes(),
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            &resolver,
            &mut source,
            parameters(20),
        ),
    )?;
    let ResidentColdOpenOutcome::Ready(mut resident) = opened else {
        return Err(std::io::Error::other("empty authority was quarantined").into());
    };
    let (workspace, probe) = live_workspace();
    let compilation = compile_live(&workspace, &probe);
    let materializations = vec![applied_evidence(&compilation).into_materialization()];
    let generation =
        resident.admit_generation(compilation, materializations, synthetic_set_policy())?;
    let record = resident
        .generation(&generation)
        .ok_or_else(|| std::io::Error::other("admitted generation is absent"))?
        .record()
        .clone();
    let before = resident.lifecycle();
    let page_request = ObservationPageRequest::new(ObservationFilter::all(), 64);
    let before_page = ready_page(
        ObservationSnapshot::from_resident(&resident)?
            .page(page_request.clone())
            .expect("candidate-only snapshot"),
    );
    assert!(before_page.facts.is_empty());
    port.enqueue(Err(DurableAuthorityStoreErrorKind::Unavailable.into()));

    let error = resident
        .publish(published(&record, 1, None))
        .expect_err("pre-append failure must remain visible");

    assert_eq!(
        error.class(),
        ResidentApplicationErrorClass::DurabilityNoMutation
    );
    assert_eq!(port.append_calls(), 1);
    assert_eq!(resident.journal_head(), None);
    assert_eq!(resident.lifecycle(), before);
    assert!(resident.generation(&generation).is_some());
    assert_eq!(
        ready_page(
            ObservationSnapshot::from_resident(&resident)?
                .page(page_request)
                .expect("failed publication preserves the snapshot"),
        ),
        before_page,
    );
    Ok(())
}

#[test]
fn failed_reclamation_restores_body_and_authority_frontier() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let reclaimable = activate_successor(&mut resident)?;
    let before = resident.lifecycle();
    port.enqueue(Err(DurableAuthorityStoreErrorKind::Unavailable.into()));

    let error = resident
        .reclaim_generation(&reclaimable)
        .expect_err("pre-append failure must preserve resident custody");

    assert_eq!(
        error.class(),
        ResidentApplicationErrorClass::DurabilityNoMutation
    );
    assert_eq!(resident.journal_head().map(|value| value.get()), Some(6));
    assert_eq!(resident.lifecycle(), before);
    assert!(resident.generation(&reclaimable).is_some());
    assert_eq!(
        resident
            .reclaim_generation(&reclaimable)?
            .journal_sequence()
            .get(),
        7
    );
    assert!(resident.generation(&reclaimable).is_none());
    Ok(())
}

#[test]
fn ambiguous_reclamation_restores_body_and_closes_mutation() -> TestResult {
    for commit_before_error in [false, true] {
        let port = MemoryAuthorityStore::default();
        let mut resident = empty_application(&port, RingCursorV1::new(0));
        let reclaimable = activate_successor(&mut resident)?;
        let before = resident.lifecycle();
        if commit_before_error {
            port.enqueue_commit_then_ambiguous();
        } else {
            port.enqueue(Err(DurableAuthorityStoreErrorKind::AmbiguousCommit.into()));
        }

        let error = resident
            .reclaim_generation(&reclaimable)
            .expect_err("ambiguous append must preserve resident custody until recovery");

        assert_eq!(
            error.class(),
            ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        );
        assert_eq!(resident.journal_head().map(|value| value.get()), Some(6));
        assert_eq!(resident.lifecycle(), before);
        assert!(resident.generation(&reclaimable).is_some());
        let closed = resident
            .reclaim_generation(&reclaimable)
            .expect_err("unknown append outcome must close the writer");
        assert_eq!(
            closed.class(),
            ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        );
    }
    Ok(())
}

#[test]
fn committed_reclamation_drops_body_when_wake_publication_fails() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(u64::MAX));
    let reclaimable = activate_successor(&mut resident)?;

    let receipt = resident.reclaim_generation(&reclaimable)?;

    assert_eq!(receipt.journal_sequence().get(), 7);
    assert!(matches!(
        receipt.wake(),
        ResidentWakeDisposition::DurableReplayRequired {
            journal_sequence,
            ..
        } if journal_sequence.get() == 7
    ));
    assert!(resident.generation(&reclaimable).is_none());
    assert_eq!(
        resident.reclamation_disposition(&reclaimable),
        ReclamationDisposition::Eligible
    );
    Ok(())
}

#[test]
fn degraded_open_is_read_only_cause_preserving_and_skips_generation_inputs() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let resolver = RecoveryResolver::default();
    let mut empty_source = FixedInputSource::default();
    let opened = ResidentApplication::cold_open(
        &port,
        &port,
        &mut UnavailableEventCustody,
        limits().maximum_entry_bytes(),
        ResidentColdOpenInput::new(
            store(),
            limits(),
            limits().maximum_entry_bytes(),
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            &resolver,
            &mut empty_source,
            parameters(10),
        ),
    )?;
    let ResidentColdOpenOutcome::Ready(mut resident) = opened else {
        return Err(std::io::Error::other("empty authority was quarantined").into());
    };
    let (workspace, probe) = live_workspace();
    let compilation = compile_live(&workspace, &probe);
    let materializations = vec![applied_evidence(&compilation).into_materialization()];
    let generation =
        resident.admit_generation(compilation, materializations, synthetic_set_policy())?;
    let record = resident
        .generation(&generation)
        .ok_or_else(|| std::io::Error::other("admitted generation is absent"))?
        .record()
        .clone();
    let published_receipt = resident.publish(published(&record, 1, None))?;
    assert_eq!(published_receipt.journal_sequence().get(), 1);
    drop(resident);

    port.quarantine_from(1);
    let mut forbidden_source = FixedInputSource::default();
    let empty_resolver = RecoveryResolver::default();
    let quarantined = ResidentApplication::cold_open(
        &port,
        &port,
        &mut UnavailableEventCustody,
        limits().maximum_entry_bytes(),
        ResidentColdOpenInput::new(
            store(),
            limits(),
            limits().maximum_entry_bytes(),
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            &empty_resolver,
            &mut forbidden_source,
            parameters(11),
        ),
    )?;
    let ResidentColdOpenOutcome::Degraded(degraded) = quarantined else {
        return Err(std::io::Error::other("untrusted suffix produced authority").into());
    };
    assert_eq!(forbidden_source.calls, 0);
    assert_eq!(degraded.lifecycle().retained_generations.len(), 0);
    assert_eq!(port.entry_count(), 1);
    let quarantine = degraded.quarantine();
    assert_eq!(
        quarantine.class(),
        ResidentRecoveryQuarantineClass::PhysicalIntegrity
    );
    assert!(quarantine.source().is_some());
    assert!(quarantine.to_string().contains("journal sequence"));
    Ok(())
}
