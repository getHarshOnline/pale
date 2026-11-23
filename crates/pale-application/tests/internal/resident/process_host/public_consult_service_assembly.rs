//! Focused laws for the public Consult composition adapters.

use super::*;
use crate::resident::journal::tests::MemoryAuthorityStore;
use crate::resident::operation_host::ResidentOperationHost;
use crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn;
use crate::resident::process_host::consult_recovery::ResidentProcessConsultRecoveryTurn;
use crate::resident::process_host::program::{
    admit_consult_child_recovery_turn, classify_consult_ready_retention,
    ResidentProgramConsultReadyRetention, ResidentProgramConsultRecoveryInputPort,
    ResidentProgramConsultRetentionAdmission, ResidentProgramConsultRetentionFailure,
    ResidentProgramConsultTurn, ResidentProgramConsultTurnKey, ResidentProgramKeyedConsultTurn,
    ResidentProgramRetainedConsultLane, ResidentProgramRetainedConsultTurns,
    ResidentProgramRetainedPhaseTurn,
};
use crate::resident::process_host::tests::{
    ready_application, start::CountingAuthority, task_state, unproved_deployment_for_selector,
    unproved_writer,
};

struct TestStop;

impl ResidentServiceConsultStopInput for TestStop {
    fn stop_meaning(&self) -> ResidentServiceConsultStopMeaning {
        unreachable!("retention-only tests never evaluate stop meaning")
    }
}

struct UnavailableExecutor;

impl ResidentServiceConsultTaskExecutor<(), ()> for UnavailableExecutor {
    fn try_accept(
        &mut self,
        task: ResidentServiceConsultTask<()>,
    ) -> Result<(), ResidentServiceConsultTask<()>> {
        Err(task)
    }

    fn try_completion(&mut self) -> Option<ResidentServiceConsultTaskCompletion<(), ()>> {
        None
    }
}

struct UnavailableStoreResolver;

impl crate::resident::OperationObservationAuthorityStoreResolver for UnavailableStoreResolver {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<
        AuthorityStoreCoordinate,
        crate::resident::OperationObservationAuthorityStoreRejection,
    > {
        unreachable!("unavailable Consult inputs cannot resolve a store")
    }
}

fn unavailable_collection_admission(
    _: &crate::resident::ConsultationCollection,
) -> Result<(), InvalidResidentRecordV1> {
    unreachable!("unavailable Consult inputs cannot admit a collection")
}

fn unavailable_child_recovery(
    _: &crate::resident::ConsultationChildDispatchCandidate,
) -> Result<crate::resident::RecoveredOperationObservation, ()> {
    unreachable!("unavailable Consult inputs cannot recover a child")
}

fn unavailable_child_retention(
    _: &crate::resident::ConsultationChildDispatchCandidate,
    _: crate::resident::RecoveredOperationObservation,
    _: &mut dyn OperationCustodyPort,
    _: &mut dyn AntiRollbackAnchorPort,
) -> crate::resident::ConsultationChildRetentionOutcome<(), ()> {
    unreachable!("unavailable Consult inputs cannot retain a child")
}

struct UnavailableConsultInputs;

struct UnreachablePolicy;

impl crate::resident::process_host::consult_recovery::ResidentConsultationPolicyEvaluator<TestStop>
    for UnreachablePolicy
{
    type Error = ();

    fn evaluate(
        &mut self,
        _: &ImmutableRecordReferenceV1,
        _: &crate::resident::consultation::ResidentConsultationPolicyFrontier,
    ) -> Result<
        crate::resident::consultation::ResidentConsultationSupervisionDecision<TestStop>,
        Self::Error,
    > {
        panic!("a failed scheduler clock must not evaluate Consult policy")
    }
}

impl ResidentServiceConsultInputPort for UnavailableConsultInputs {
    type Work = ();
    type Output = ();
    type ChildRecoveryError = ();
    type RetentionFailure = ();
    type Executor = UnavailableExecutor;
    type StoreResolver = UnavailableStoreResolver;
    type AdmitCollection =
        fn(&crate::resident::ConsultationCollection) -> Result<(), InvalidResidentRecordV1>;
    type RecoverChild = fn(
        &crate::resident::ConsultationChildDispatchCandidate,
    ) -> Result<crate::resident::RecoveredOperationObservation, ()>;
    type RetainChild = fn(
        &crate::resident::ConsultationChildDispatchCandidate,
        crate::resident::RecoveredOperationObservation,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
    ) -> crate::resident::ConsultationChildRetentionOutcome<(), ()>;

    fn with_finite_turn_inputs<Return, UseInputs>(
        &mut self,
        _: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::ConsultationCollectionLimits,
            ResidentServiceConsultTaskLimits,
            &Self::StoreResolver,
            &mut Self::AdmitCollection,
            &mut ResidentServiceConsultExecutor<Self::Executor>,
            &mut Self::RecoverChild,
            &mut Self::RetainChild,
            Option<&mut crate::resident::HistoricalConsultationAdmission<'_>>,
        ) -> Return,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_initial_collection_inputs<Return, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ControlRecordCoordinate,
        _: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs:
            for<'input> FnOnce(ResidentServiceConsultInitialCollectionInputs<'input>) -> Return,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_stop_inputs<Return, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &ControlRecordCoordinate,
        _: &ResidentServiceConsultStopMeaning,
        _: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceConsultStopInputs<'input>) -> Return,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_completion_join_inputs<Return, UseInputs>(
        &mut self,
        _: ResidentServiceConsultJoinSelection<'_>,
        _: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceConsultJoinInputs<'input>) -> Return,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_terminal_child_join_inputs<Return, UseInputs>(
        &mut self,
        _: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentServiceConsultTerminalChildInputs<'input>) -> Return,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }
}

type TestAdapter = ResidentServiceConsultRecoveryAdapter<UnavailableConsultInputs, TestStop>;
type TestTurn = ResidentServiceRetainedConsultTurn<UnavailableConsultInputs, TestStop>;

fn exhausted_turn() -> TestTurn {
    ResidentProgramConsultTurn::PrePolicy(ResidentProcessConsultRecoveryTurn::Exhausted)
}

fn retained_key(name: &str) -> ResidentProgramConsultTurnKey {
    ResidentProgramConsultTurnKey::new(
        super::super::ResidentDeploymentAuthoritySelector {
            principal_id: ResidentPrincipalIdV1::parse("public-consult-principal")
                .expect("principal"),
            deployment_binding_ref: ResidentRecordRefV1::parse("public/consult/deployment")
                .expect("deployment reference"),
            deployment_binding_sha256: DeploymentBindingDigest::parse("1".repeat(64))
                .expect("deployment digest"),
            runtime_authority_ref: ResidentRecordRefV1::parse("public/consult/runtime")
                .expect("runtime reference"),
            runtime_authority_sha256: DeploymentRuntimeAuthorityDigest::parse("2".repeat(64))
                .expect("runtime digest"),
        },
        ControlRecordCoordinate::new(
            ResidentRecordRefV1::parse(format!("public/consult/{name}")).expect("parent reference"),
            ControlRecordDigest::parse("3".repeat(64)).expect("parent digest"),
        ),
    )
}

fn retain_keyed(adapter: &mut TestAdapter, key: ResidentProgramConsultTurnKey) {
    assert!(adapter.turns.retain(key, exhausted_turn()).is_ok());
}

fn saturated_key(
    adapter: &mut TestAdapter,
    key: ResidentProgramConsultTurnKey,
) -> ResidentProgramConsultTurnKey {
    let refusal = adapter
        .retain_keyed_consult_turn(ResidentProgramKeyedConsultTurn::new(key, exhausted_turn()))
        .expect_err("the bounded Consult owner is already full");
    let ResidentProgramConsultRetentionFailure::Saturated(retained) = refusal else {
        panic!("a distinct parent must be refused as saturated")
    };
    retained.into_parts().0
}

#[test]
fn public_consult_composition_rejects_delegate_request_kinds() {
    assert!(admit_invoke_consult_request_kinds(false, false).is_ok());
    assert!(matches!(
        admit_invoke_consult_request_kinds(true, false),
        Err(ResidentServiceAssemblyError::RequestPolicy)
    ));
    assert!(matches!(
        admit_invoke_consult_request_kinds(false, true),
        Err(ResidentServiceAssemblyError::RequestPolicy)
    ));
}

#[test]
fn public_consult_physical_inputs_fail_closed_when_unavailable() {
    let mut adapter = TestAdapter::new(
        UnavailableConsultInputs,
        NonZeroUsize::new(1).expect("one Consult turn"),
    );
    let result =
        <TestAdapter as ResidentProgramConsultRecoveryInputPort<TestStop>>::with_consult_inputs(
            &mut adapter,
            |_, _, _, _, _, _, _, historical| assert!(historical.is_none()),
        );
    assert!(matches!(
        result,
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    ));
}

#[test]
fn public_consult_advance_is_visible_under_its_exact_parent_key() {
    let mut adapter = TestAdapter::new(
        UnavailableConsultInputs,
        NonZeroUsize::new(1).expect("one Consult turn"),
    );
    let key = retained_key("advance-parent");
    retain_keyed(&mut adapter, key.clone());

    assert!(adapter.has_runnable_retained_consult());
    let retained = adapter
        .take_runnable_retained_consult()
        .expect("point-addressed AdvanceConsult is scheduler-visible");
    let (actual, turn) = retained.into_parts();
    assert_eq!(actual, key);
    assert!(matches!(
        turn,
        ResidentProgramConsultTurn::PrePolicy(ResidentProcessConsultRecoveryTurn::Exhausted)
    ));
}

#[test]
fn public_consult_full_owner_returns_multiple_distinct_turns_without_panicking() {
    let mut adapter = TestAdapter::new(
        UnavailableConsultInputs,
        NonZeroUsize::new(1).expect("one Consult turn"),
    );
    let key = retained_key("duplicate-parent");
    retain_keyed(&mut adapter, key.clone());
    let first_waiting = retained_key("first-waiting-parent");
    let second_waiting = retained_key("second-waiting-parent");

    assert!(adapter.has_retained_consult_key(&key));
    assert!(!adapter.can_retain_consult_key(&key));
    assert_eq!(
        saturated_key(&mut adapter, first_waiting.clone()),
        first_waiting
    );
    assert_eq!(
        saturated_key(&mut adapter, second_waiting.clone()),
        second_waiting,
    );
    assert_eq!(
        adapter
            .take_runnable_retained_consult()
            .expect("the original turn remains intact")
            .into_parts()
            .0,
        key,
    );
    assert!(!adapter.has_runnable_retained_consult());
}

#[test]
fn public_consult_scheduler_rotates_across_every_retained_subowner() {
    let mut adapter = TestAdapter::new(
        UnavailableConsultInputs,
        NonZeroUsize::new(1).expect("one Consult turn"),
    );

    for expected in [
        ResidentProgramRetainedConsultLane::StopCustody,
        ResidentProgramRetainedConsultLane::TerminalFailure,
        ResidentProgramRetainedConsultLane::KeyedTurn,
        ResidentProgramRetainedConsultLane::StopCustody,
    ] {
        assert_eq!(adapter.next_retained_consult_lane(), expected);
    }
}

#[test]
fn public_consult_auxiliary_custody_fences_its_exact_parent() {
    let stop_key = retained_key("stop-parent");
    let terminal_key = retained_key("terminal-parent");
    let other_key = retained_key("other-parent");
    let stop = ResidentProgramKeyedConsultTurn::new(stop_key.clone(), ());
    let terminal = ResidentProgramKeyedConsultTurn::new(terminal_key.clone(), ());

    assert!(retained_consult_custody_contains_key(
        Some(&stop),
        Some(&terminal),
        &stop_key,
    ));
    assert!(retained_consult_custody_contains_key(
        Some(&stop),
        Some(&terminal),
        &terminal_key,
    ));
    assert!(!retained_consult_custody_contains_key(
        Some(&stop),
        Some(&terminal),
        &other_key,
    ));

    let admission = ResidentProgramConsultRetentionAdmission::new(
        true,
        vec![stop_key.clone(), terminal_key.clone()],
    );
    assert!(!admission.admits(&stop_key));
    assert!(!admission.admits(&terminal_key));
    assert!(admission.admits(&other_key));
}

#[test]
fn public_consult_auxiliary_custody_rejects_only_its_exact_parent() {
    let stop_key = retained_key("stop-parent");
    let other_key = retained_key("other-parent");
    let stop = ResidentProgramKeyedConsultTurn::new(stop_key.clone(), ());
    let mut turns =
        ResidentProgramRetainedConsultTurns::new(NonZeroUsize::new(2).expect("two Consult turns"));

    let duplicate = retain_unique_consult_turn(
        &mut turns,
        Some(&stop),
        None::<&ResidentProgramKeyedConsultTurn<()>>,
        ResidentProgramKeyedConsultTurn::new(stop_key.clone(), 1_u8),
    )
    .expect_err("auxiliary custody already owns this parent");
    let ResidentProgramConsultRetentionFailure::Duplicate(duplicate) = duplicate else {
        panic!("same-parent auxiliary custody must be classified as duplicate")
    };
    assert_eq!(duplicate.into_parts(), (stop_key, 1));

    let retained = retain_unique_consult_turn(
        &mut turns,
        Some(&stop),
        None::<&ResidentProgramKeyedConsultTurn<()>>,
        ResidentProgramKeyedConsultTurn::new(other_key.clone(), 2_u8),
    );
    assert!(
        retained.is_ok(),
        "an unrelated parent uses the available keyed slot"
    );
    assert_eq!(
        turns
            .take_next()
            .expect("the unrelated parent is retained")
            .into_parts(),
        (other_key, 2),
    );
}

#[test]
fn public_consult_ready_distinguishes_existing_parent_from_saturation() {
    let mut adapter = TestAdapter::new(
        UnavailableConsultInputs,
        NonZeroUsize::new(1).expect("one Consult turn"),
    );
    let retained = retained_key("retained-parent");
    let waiting = retained_key("waiting-parent");

    assert_eq!(
        classify_consult_ready_retention(&adapter, &retained, 1_u8),
        ResidentProgramConsultReadyRetention::Retain(1),
    );
    retain_keyed(&mut adapter, retained.clone());
    assert_eq!(
        classify_consult_ready_retention(&adapter, &retained, 2_u8),
        ResidentProgramConsultReadyRetention::Subsumed,
    );
    assert_eq!(
        classify_consult_ready_retention(&adapter, &waiting, 3_u8),
        ResidentProgramConsultReadyRetention::Deferred(3),
    );
    assert_eq!(
        adapter
            .take_runnable_retained_consult()
            .expect("the retained parent is unchanged")
            .into_parts()
            .0,
        retained,
    );
    assert_eq!(
        classify_consult_ready_retention(&adapter, &waiting, 4_u8),
        ResidentProgramConsultReadyRetention::Retain(4),
    );
}

#[test]
fn public_consult_failed_clock_restores_the_exact_keyed_turn() {
    let store = crate::local_operation_start_fixture::store_coordinate();
    let key = retained_key("clock-parent");
    let selector = key.selector().clone();
    let application_store = MemoryAuthorityStore::default();
    let application = ready_application(&application_store, store.clone(), 1);
    let mut process = ResidentProcessHost::new(
        unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
        vec![unproved_deployment_for_selector(
            &selector,
            store,
            application,
        )],
        NonZeroUsize::new(1).expect("one deployment"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one exact test deployment");
    let mut adapter = TestAdapter::new(
        UnavailableConsultInputs,
        NonZeroUsize::new(1).expect("one Consult turn"),
    );
    retain_keyed(&mut adapter, key.clone());
    adapter.next_retained_lane = ResidentProgramRetainedConsultLane::KeyedTurn;
    let mut authority = CountingAuthority::default();
    let mut policy = UnreachablePolicy;
    let mut clock_reads = 0;
    let turn = adapter.continue_one_retained_consult(
        &mut process,
        &mut [],
        &mut authority,
        &mut policy,
        &mut || {
            clock_reads += 1;
            None
        },
    );

    assert_eq!(turn, ResidentProgramRetainedPhaseTurn::Deferred);
    assert_eq!(clock_reads, 1);
    assert_eq!(authority.calls, 0);
    assert_eq!(
        adapter
            .take_runnable_retained_consult()
            .expect("the failed clock restores the exact keyed turn")
            .into_parts()
            .0,
        key,
    );
}

#[test]
fn public_consult_child_handoff_is_taken_before_another_recovery_scan() {
    let mut adapter = TestAdapter::new(
        UnavailableConsultInputs,
        NonZeroUsize::new(1).expect("one Consult turn"),
    );
    adapter.retain_consult_child_turn(ResidentProcessConsultChildRecoveryTurn::Exhausted);

    assert!(matches!(
        adapter.take_consult_child_turn(),
        Some(ResidentProcessConsultChildRecoveryTurn::Exhausted)
    ));
    adapter.retain_consult_child_turn(ResidentProcessConsultChildRecoveryTurn::Exhausted);
    assert!(matches!(
        adapter.take_consult_child_turn(),
        Some(ResidentProcessConsultChildRecoveryTurn::Exhausted)
    ));
}

#[test]
fn public_consult_full_keyed_owner_does_not_block_terminal_child_recovery() {
    let mut adapter = TestAdapter::new(
        UnavailableConsultInputs,
        NonZeroUsize::new(1).expect("one Consult turn"),
    );
    let waiting_parent = retained_key("waiting-terminal-parent");
    retain_keyed(&mut adapter, waiting_parent.clone());

    crate::resident::process_host::tests::consult_child_recovery_sweep_hostile::with_cold_terminal_consult_child_handoff(
        "full-keyed-owner-terminal-child",
        |handoff| {
            let turn = admit_consult_child_recovery_turn(
                &mut adapter,
                ResidentProcessConsultChildRecoveryTurn::Recovered(handoff),
            )
            .expect("terminal evidence does not allocate a keyed Consult turn");
            assert!(matches!(
                turn,
                ResidentProcessConsultChildRecoveryTurn::Recovered(handoff)
                    if matches!(
                        handoff.state(),
                        crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryState::Terminal(_)
                    )
            ));
        },
    );

    assert_eq!(
        adapter
            .take_runnable_retained_consult()
            .expect("the waiting parent remains unchanged")
            .into_parts()
            .0,
        waiting_parent,
    );
    assert!(adapter.take_consult_child_turn().is_none());
}

// Keep the concrete callback owners type-checked even though this hostile port
// refuses before it lends them.
const _: fn(&crate::resident::ConsultationCollection) -> Result<(), InvalidResidentRecordV1> =
    unavailable_collection_admission;
const _: fn(
    &crate::resident::ConsultationChildDispatchCandidate,
) -> Result<crate::resident::RecoveredOperationObservation, ()> = unavailable_child_recovery;
type ChildRetentionCallback = fn(
    &crate::resident::ConsultationChildDispatchCandidate,
    crate::resident::RecoveredOperationObservation,
    &mut dyn OperationCustodyPort,
    &mut dyn AntiRollbackAnchorPort,
) -> crate::resident::ConsultationChildRetentionOutcome<(), ()>;
const _: ChildRetentionCallback = unavailable_child_retention;
