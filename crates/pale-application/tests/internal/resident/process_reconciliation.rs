//! Law tests for the pure supervisor plan, claim, and reconciliation phases.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use pale_core::records::{
    ArtifactDigest, CompiledGenerationDigest, ExecutableDigest, ResidentProcessInstanceIdV1,
    ServiceDefinition, ServiceLaunch,
};

use super::*;

fn digest<T>(value: char) -> T
where
    T: TryFrom<String>,
    T::Error: std::fmt::Debug,
{
    T::try_from(value.to_string().repeat(64)).expect("fixture digest must be valid")
}

fn definition(name: &str, precedence: u32, port: u16, dependencies: &[&str]) -> ServiceDefinition {
    ServiceDefinition {
        name: name.to_owned(),
        kind: "daemon".to_owned(),
        precedence,
        launch: ServiceLaunch {
            program: format!("/managed/{name}"),
            args: vec!["serve".to_owned()],
        },
        ports: vec![port],
        depends_on: dependencies
            .iter()
            .map(|dependency| (*dependency).to_owned())
            .collect(),
    }
}

fn declaration(
    name: &str,
    precedence: u32,
    port: u16,
    dependencies: &[&str],
    digest_byte: u8,
) -> SupervisorServiceDeclaration {
    SupervisorServiceDeclaration::new(
        definition(name, precedence, port, dependencies),
        ExecutableDigest::hash_bytes(&[digest_byte]),
        ArtifactDigest::hash_bytes(&[digest_byte, digest_byte]),
        RestartPolicy::new(4, 100, 800).expect("fixture restart policy must be valid"),
    )
}

fn plan() -> SupervisorPlan {
    SupervisorPlan::admit(
        digest::<CompiledGenerationDigest>('a'),
        vec![
            declaration("web", 30, 8080, &["api"], 3),
            declaration("database", 10, 3309, &[], 1),
            declaration("api", 20, 4000, &["database"], 2),
        ],
    )
    .expect("fixture supervisor plan must be valid")
}

fn expected_claims(plan: &SupervisorPlan) -> BTreeMap<SupervisorServiceId, ProcessFenceClaim> {
    plan.services()
        .enumerate()
        .map(|(index, service)| {
            let id = service.id().clone();
            let claim = plan
                .bind_expected_claim(
                    &id,
                    ResidentProcessInstanceIdV1::parse(format!("process-{index}"))
                        .expect("fixture process id must be valid"),
                    ProcessGeneration::new(1).expect("fixture generation must be nonzero"),
                )
                .expect("fixture service must belong to plan");
            (id, claim)
        })
        .collect()
}

fn healthy_observations(
    claims: &BTreeMap<SupervisorServiceId, ProcessFenceClaim>,
) -> BTreeMap<SupervisorServiceId, ProcessObservationClaim> {
    claims
        .iter()
        .map(|(service, claim)| {
            (
                service.clone(),
                ProcessObservationClaim::FencedRunning {
                    fence: claim.clone(),
                    probe: ServiceProbeClaim::Healthy,
                },
            )
        })
        .collect()
}

fn isolation_policy() -> ProcessIsolationPolicy {
    ProcessIsolationPolicy::new(
        ProcessIsolationAction::Terminate,
        ProcessIsolationAction::Quarantine,
    )
}

fn service_steps(reconciliation: &SupervisorReconciliation) -> Vec<&ServiceDirective> {
    reconciliation
        .steps()
        .iter()
        .filter_map(|step| match step {
            SupervisorReconciliationStep::Service(directive) => Some(directive),
            SupervisorReconciliationStep::ApplyStopSentinel(_) => None,
        })
        .collect()
}

#[test]
fn reconciliation_encodes_control_phases_and_process_order_in_one_sequence() {
    let plan = plan();
    assert_eq!(
        plan.services()
            .map(|service| service.id().as_str())
            .collect::<Vec<_>>(),
        ["database", "api", "web"]
    );
    assert_eq!(
        plan.services()
            .last()
            .expect("web service must exist")
            .dependencies()
            .iter()
            .map(SupervisorServiceId::as_str)
            .collect::<Vec<_>>(),
        ["database", "api"]
    );

    let claims = expected_claims(&plan);
    let observations = healthy_observations(&claims);
    let snapshot = plan
        .admit_claimed_snapshot(claims, observations)
        .expect("complete fixture claims must be structurally valid");

    let pending_running = snapshot.reconcile(DesiredProcessState::Running, isolation_policy());
    assert_eq!(
        pending_running.compiled_generation(),
        plan.compiled_generation()
    );
    assert_eq!(
        pending_running.desired_process_state(),
        DesiredProcessState::Running
    );
    let running = pending_running.into_committed();
    assert_eq!(running.compiled_generation(), plan.compiled_generation());
    assert!(matches!(
        running.steps()[0],
        SupervisorReconciliationStep::ApplyStopSentinel(StopSentinelIntent::EnsureAbsent)
    ));
    let running_services = service_steps(&running);
    assert_eq!(
        running_services
            .iter()
            .map(|directive| directive.service().as_str())
            .collect::<Vec<_>>(),
        ["database", "api", "web"]
    );
    assert!(running_services.iter().all(|directive| matches!(
        directive.action(),
        ServiceDirectiveAction::RetainMatching(_)
    )));

    let pending_stopped = snapshot.reconcile(DesiredProcessState::Stopped, isolation_policy());
    assert_eq!(
        pending_stopped.compiled_generation(),
        plan.compiled_generation()
    );
    assert_eq!(
        pending_stopped.desired_process_state(),
        DesiredProcessState::Stopped
    );
    let stopped = pending_stopped.into_committed();
    assert_eq!(stopped.compiled_generation(), plan.compiled_generation());
    assert!(matches!(
        stopped.steps()[0],
        SupervisorReconciliationStep::ApplyStopSentinel(StopSentinelIntent::EnsurePresent)
    ));
    let stopped_services = service_steps(&stopped);
    assert_eq!(
        stopped_services
            .iter()
            .map(|directive| directive.service().as_str())
            .collect::<Vec<_>>(),
        ["web", "api", "database"]
    );
    assert!(stopped_services
        .iter()
        .all(|directive| matches!(directive.action(), ServiceDirectiveAction::StopMatching(_))));
}

#[test]
fn up_and_down_are_idempotent_and_own_the_keepalive_sentinel_intent() {
    assert_eq!(
        DesiredProcessState::Running.transition(SupervisorCommand::Up),
        DesiredStateTransition::Unchanged(DesiredProcessState::Running)
    );
    assert_eq!(
        DesiredProcessState::Stopped.transition(SupervisorCommand::Down),
        DesiredStateTransition::Unchanged(DesiredProcessState::Stopped)
    );
    assert!(matches!(
        DesiredProcessState::Stopped.transition(SupervisorCommand::Up),
        DesiredStateTransition::Changed {
            previous: DesiredProcessState::Stopped,
            next: DesiredProcessState::Running,
        }
    ));
    assert_eq!(
        DesiredProcessState::Running.stop_sentinel(),
        StopSentinelIntent::EnsureAbsent
    );
    assert_eq!(
        DesiredProcessState::Stopped.stop_sentinel(),
        StopSentinelIntent::EnsurePresent
    );
}

#[test]
fn starts_are_blocked_by_the_complete_transitive_prerequisite_closure() {
    let plan = plan();
    let claims = expected_claims(&plan);
    let mut observations = healthy_observations(&claims);
    let database = plan
        .services()
        .next()
        .expect("database must exist")
        .id()
        .clone();
    let web = plan.services().last().expect("web must exist").id().clone();
    observations.insert(
        database,
        ProcessObservationClaim::Absent(AbsentProcessState::Startable),
    );
    observations.insert(
        web,
        ProcessObservationClaim::Absent(AbsentProcessState::Startable),
    );
    let snapshot = plan
        .admit_claimed_snapshot(claims, observations)
        .expect("claims must be structurally valid");
    let reconciliation = snapshot
        .reconcile(DesiredProcessState::Running, isolation_policy())
        .into_committed();
    let services = service_steps(&reconciliation);
    assert!(matches!(
        services[2].action(),
        ServiceDirectiveAction::Hold(ServiceStartHold::Dependencies(blockers))
            if blockers.len() == 1
                && blockers[0].service().as_str() == "database"
                && blockers[0].reason() == ServicePrerequisiteFailureReason::ProcessAbsent
    ));
}

#[test]
fn restart_backoff_is_bounded_and_the_failure_ceiling_is_closed() {
    let policy = RestartPolicy::new(4, 100, 350).expect("policy must be valid");
    let pending = |failures| AbsentProcessState::BackoffPending {
        consecutive_failures: NonZeroU32::new(failures).expect("failures must be positive"),
    };
    assert!(matches!(
        classify_restart(policy, pending(1)),
        RestartDisposition::Backoff { delay_ms } if delay_ms.get() == 100
    ));
    assert!(matches!(
        classify_restart(policy, pending(3)),
        RestartDisposition::Backoff { delay_ms } if delay_ms.get() == 350
    ));
    assert!(matches!(
        classify_restart(policy, pending(4)),
        RestartDisposition::CrashLoop { consecutive_failures }
            if consecutive_failures.get() == 4
    ));
    assert!(matches!(
        classify_restart(
            RestartPolicy::new(u32::MAX, 2, u64::MAX).expect("policy must be valid"),
            pending(u32::MAX - 1)
        ),
        RestartDisposition::Backoff { delay_ms } if delay_ms.get() == u64::MAX
    ));
    let saturation_policy = RestartPolicy::new(100, 1, u64::MAX).expect("policy must be valid");
    assert!(matches!(
        classify_restart(saturation_policy, pending(64)),
        RestartDisposition::Backoff { delay_ms } if delay_ms.get() == 1_u64 << 63
    ));
    assert!(matches!(
        classify_restart(saturation_policy, pending(65)),
        RestartDisposition::Backoff { delay_ms } if delay_ms.get() == u64::MAX
    ));
    assert!(matches!(
        classify_restart(
            policy,
            AbsentProcessState::BackoffElapsed {
                consecutive_failures: NonZeroU32::new(4).expect("failures must be positive"),
            }
        ),
        RestartDisposition::CrashLoop { .. }
    ));
}

#[test]
fn process_generation_and_restart_policy_reject_every_closed_boundary() {
    assert_eq!(
        ProcessGeneration::new(0),
        Err(InvalidProcessGeneration::Zero)
    );
    assert_eq!(
        ProcessGeneration::new(u64::MAX)
            .expect("maximum generation is representable")
            .checked_successor(),
        Err(InvalidProcessGeneration::Exhausted)
    );
    assert_eq!(
        RestartPolicy::new(0, 1, 1),
        Err(InvalidRestartPolicy::ZeroFailures)
    );
    assert_eq!(
        RestartPolicy::new(1, 0, 1),
        Err(InvalidRestartPolicy::ZeroInitialDelay)
    );
    assert_eq!(
        RestartPolicy::new(1, 1, 0),
        Err(InvalidRestartPolicy::ZeroMaximumDelay)
    );
    assert_eq!(
        RestartPolicy::new(1, 2, 1),
        Err(InvalidRestartPolicy::DescendingDelay)
    );
}

#[test]
fn matching_is_structural_and_every_mismatch_isolated_by_explicit_policy() {
    let plan = plan();
    let claims = expected_claims(&plan);
    let mut observations = healthy_observations(&claims);
    let database = plan
        .services()
        .next()
        .expect("database must exist")
        .id()
        .clone();
    let expected = claims.get(&database).expect("database claim must exist");
    observations.insert(
        database.clone(),
        ProcessObservationClaim::FencedRunning {
            fence: ProcessFenceClaim::new(
                database.clone(),
                expected.process_instance_id().clone(),
                expected
                    .process_generation()
                    .checked_successor()
                    .expect("fixture generation must advance"),
                expected.compiled_generation().clone(),
                expected.executable_digest.clone(),
                expected.configuration_digest.clone(),
            ),
            probe: ServiceProbeClaim::Healthy,
        },
    );
    let snapshot = plan
        .admit_claimed_snapshot(claims, observations)
        .expect("mismatching observation remains a valid raw claim");
    let reconciliation = snapshot
        .reconcile(DesiredProcessState::Running, isolation_policy())
        .into_committed();
    assert!(matches!(
        service_steps(&reconciliation)[0].action(),
        ServiceDirectiveAction::Isolate {
            mismatch: ProcessClaimMismatch::FencedMismatch,
            action: ProcessIsolationAction::Terminate,
            ..
        }
    ));

    let claims = expected_claims(&plan);
    let mut observations = healthy_observations(&claims);
    observations.insert(
        database,
        ProcessObservationClaim::UnidentifiedRunning {
            process_instance_id: ResidentProcessInstanceIdV1::parse("unknown-child")
                .expect("fixture process id must be valid"),
        },
    );
    let snapshot = plan
        .admit_claimed_snapshot(claims, observations)
        .expect("unidentified observation remains a valid raw claim");
    for desired in [DesiredProcessState::Running, DesiredProcessState::Stopped] {
        let reconciliation = snapshot
            .reconcile(desired, isolation_policy())
            .into_committed();
        let database_directive = service_steps(&reconciliation)
            .into_iter()
            .find(|directive| directive.service().as_str() == "database")
            .expect("database directive must exist");
        assert!(matches!(
            database_directive.action(),
            ServiceDirectiveAction::Isolate {
                mismatch: ProcessClaimMismatch::Unidentified,
                action: ProcessIsolationAction::Quarantine,
                ..
            }
        ));
    }
}

#[test]
fn unknown_and_unhealthy_probe_claims_block_only_dependent_starts() {
    let plan = plan();
    for (probe, expected_reason) in [
        (
            ServiceProbeClaim::Unknown,
            ServicePrerequisiteFailureReason::ProbeUnknown,
        ),
        (
            ServiceProbeClaim::Unhealthy(ServiceProbeUnhealthyReason::ProbeFailed),
            ServicePrerequisiteFailureReason::ProbeUnhealthy(
                ServiceProbeUnhealthyReason::ProbeFailed,
            ),
        ),
    ] {
        let claims = expected_claims(&plan);
        let mut observations = healthy_observations(&claims);
        let database = plan
            .services()
            .next()
            .expect("database must exist")
            .id()
            .clone();
        let api = plan.services().nth(1).expect("api must exist").id().clone();
        let database_claim = claims
            .get(&database)
            .expect("database claim must exist")
            .clone();
        observations.insert(
            database,
            ProcessObservationClaim::FencedRunning {
                fence: database_claim,
                probe,
            },
        );
        observations.insert(
            api,
            ProcessObservationClaim::Absent(AbsentProcessState::Startable),
        );
        let snapshot = plan
            .admit_claimed_snapshot(claims, observations)
            .expect("claims must be structurally valid");
        let reconciliation = snapshot
            .reconcile(DesiredProcessState::Running, isolation_policy())
            .into_committed();
        assert!(matches!(
            service_steps(&reconciliation)[1].action(),
            ServiceDirectiveAction::Hold(ServiceStartHold::Dependencies(blockers))
                if blockers.len() == 1 && blockers[0].reason() == expected_reason
        ));
    }
}

#[test]
fn snapshot_rejects_each_substituted_plan_owned_claim_axis() {
    let plan = plan();
    let claims = expected_claims(&plan);
    let service = plan
        .services()
        .next()
        .expect("database must exist")
        .id()
        .clone();
    let expected = claims.get(&service).expect("database claim must exist");
    let other_service = plan.services().nth(1).expect("api must exist").id().clone();
    let substitutions = [
        ProcessFenceClaim::new(
            other_service,
            expected.process_instance_id().clone(),
            expected.process_generation(),
            expected.compiled_generation().clone(),
            expected.executable_digest.clone(),
            expected.configuration_digest.clone(),
        ),
        ProcessFenceClaim::new(
            service.clone(),
            expected.process_instance_id().clone(),
            expected.process_generation(),
            digest::<CompiledGenerationDigest>('b'),
            expected.executable_digest.clone(),
            expected.configuration_digest.clone(),
        ),
        ProcessFenceClaim::new(
            service.clone(),
            expected.process_instance_id().clone(),
            expected.process_generation(),
            expected.compiled_generation().clone(),
            ExecutableDigest::hash_bytes(b"substituted executable"),
            expected.configuration_digest.clone(),
        ),
        ProcessFenceClaim::new(
            service.clone(),
            expected.process_instance_id().clone(),
            expected.process_generation(),
            expected.compiled_generation().clone(),
            expected.executable_digest.clone(),
            ArtifactDigest::hash_bytes(b"substituted configuration"),
        ),
    ];
    for substitution in substitutions {
        let mut substituted = claims.clone();
        substituted.insert(service.clone(), substitution);
        let observations = healthy_observations(&substituted);
        assert!(matches!(
            plan.admit_claimed_snapshot(substituted, observations),
            Err(SupervisorObservationError::ExpectedClaimMismatch { service: failed })
                if failed == service
        ));
    }
}

#[test]
fn snapshot_rejects_missing_extra_and_cross_service_process_aliases() {
    let plan = plan();
    let claims = expected_claims(&plan);
    let observations = healthy_observations(&claims);
    let first = plan
        .services()
        .next()
        .expect("database must exist")
        .id()
        .clone();
    let second = plan.services().nth(1).expect("api must exist").id().clone();

    let mut missing_claim = claims.clone();
    missing_claim.remove(&first);
    assert!(matches!(
        plan.admit_claimed_snapshot(missing_claim, observations.clone()),
        Err(SupervisorObservationError::ExpectedClaimSetMismatch)
    ));

    let mut extra_observation = observations.clone();
    extra_observation.insert(
        SupervisorServiceId::from_admitted("extra".to_owned()),
        ProcessObservationClaim::Absent(AbsentProcessState::Startable),
    );
    assert!(matches!(
        plan.admit_claimed_snapshot(claims.clone(), extra_observation),
        Err(SupervisorObservationError::ObservationSetMismatch)
    ));

    let first_process = claims
        .get(&first)
        .expect("first claim must exist")
        .process_instance_id()
        .clone();
    let mut aliased_claims = claims.clone();
    let second_claim = aliased_claims
        .get_mut(&second)
        .expect("second claim must exist");
    second_claim.process_instance_id = first_process;
    let aliased_observations = healthy_observations(&aliased_claims);
    assert!(matches!(
        plan.admit_claimed_snapshot(aliased_claims, aliased_observations),
        Err(SupervisorObservationError::CrossServiceProcessAlias { .. })
    ));

    let mut aliased_observations = observations;
    aliased_observations.insert(
        second,
        ProcessObservationClaim::UnidentifiedRunning {
            process_instance_id: claims
                .get(&first)
                .expect("first claim must exist")
                .process_instance_id()
                .clone(),
        },
    );
    assert!(matches!(
        plan.admit_claimed_snapshot(claims, aliased_observations),
        Err(SupervisorObservationError::CrossServiceProcessAlias { .. })
    ));
}
