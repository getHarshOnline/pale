//! Cold-restart laws before root Invoke implementation selection.

use super::provider_recovery::{AppendAnchor, AppendCustody};
use super::restart::{association, host_with_generation, restart_input};
use super::*;
use crate::local_operation_start_fixture::{record_ref, schema_digest, signature};
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_operation_resolution_restart_fixtures, ProcessHostRestartFixture,
};
use crate::resident::process_host::operation_resolution::*;
use crate::resident::process_host::restart::*;
use crate::resident::{
    OperationAttemptError, OperationCheckpointInput, OperationObservationAuthorityStoreRejection,
    OperationObservationAuthorityStoreResolver,
};
use crate::session_admission_fixture::ProvingAuthority;
use pale_core::records::{
    ControlCheckpointIdV1, ImmutableRecordReferenceV1, OperationActivePhaseV1,
    OperationObservationStateV1, SignedResidentAdmissionContextV1, UnixTimeV1,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID, OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AccumulatorPreparationLimits, AuthorityStoreCoordinate, ControlStoreErrorKind,
    OperationCustodyReadBackOutcome,
};
use std::cell::Cell;
use std::rc::Rc;

struct SelectedStore(AuthorityStoreCoordinate);

impl OperationObservationAuthorityStoreResolver for SelectedStore {
    fn resolve(
        &self,
        _: &pale_core::records::OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Ok(self.0.clone())
    }
}

fn accumulator_limits() -> AccumulatorPreparationLimits {
    AccumulatorPreparationLimits::new(2, 20_000, 20_000, 8_192, 200_000)
        .expect("bounded context checkpoint")
}

#[derive(Debug, PartialEq, Eq)]
struct HandoffSummary {
    context_bound: bool,
    predecessor_phase: &'static str,
    selection_ref: ResidentRecordRefV1,
    observation_ref: ResidentRecordRefV1,
    checkpoint_ref: ResidentRecordRefV1,
}

fn summarize_handoff(
    context: crate::resident::OperationAttemptContext<'_>,
    checkpoint: (
        &ResidentRecordRefV1,
        &pale_core::records::ControlStateCheckpointV1,
    ),
) -> Result<HandoffSummary, ()> {
    let predecessor = context
        .observation
        .predecessor
        .expect("one exact predecessor");
    let predecessor_phase = match predecessor.1.state() {
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving,
        } => "resolving",
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::ContextBound { .. },
        } => "context_bound",
        _ => panic!("pre-attempt handoff has one admitted phase"),
    };
    Ok(HandoffSummary {
        context_bound: context.context_binding.is_some(),
        predecessor_phase,
        selection_ref: context.selection_ref,
        observation_ref: context.observation_ref,
        checkpoint_ref: checkpoint.0.clone(),
    })
}

#[test]
fn restart_classifies_only_compatible_root_operation_resolution_phases() {
    let case = Rc::new(Cell::new(0));
    let counted = Rc::clone(&case);
    with_process_host_operation_resolution_restart_fixtures(&mut move |fixture| {
        let ordinal = counted.replace(counted.get() + 1);
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let input = restart_input(recovery, 1 << 20);
        let result = host.classify_existing_operation(
            ResidentProcessExistingOperationStart {
                selector,
                evidence: Box::new(evidence),
            },
            association(signing_authority, &input),
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            input,
            admission,
        );
        if ordinal == 3 {
            assert!(matches!(
                result,
                Err(ResidentProcessOperationRestartError::State)
            ));
            return;
        }
        let ResidentProcessOperationRestartDisposition::OperationResolution(recovered) =
            result.expect("compatible pre-attempt phase")
        else {
            panic!("compatible root Invoke has one pre-attempt owner")
        };
        assert_eq!(
            recovered.phase,
            if ordinal == 2 {
                ResidentProcessOperationResolutionPhase::ContextBound
            } else {
                ResidentProcessOperationResolutionPhase::Resolving
            }
        );
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
    });
    assert_eq!(case.get(), 4);
}

#[test]
fn self_contained_and_cold_context_handoffs_recheck_custody_without_effects() {
    let case = Rc::new(Cell::new(0));
    let counted = Rc::clone(&case);
    with_process_host_operation_resolution_restart_fixtures(&mut move |fixture| {
        let ordinal = counted.replace(counted.get() + 1);
        if ordinal == 1 || ordinal == 3 {
            return;
        }
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            session,
            current,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let input = restart_input(recovery, 1 << 20);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &input),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
            .expect("pre-attempt classification");
        let ResidentProcessOperationRestartDisposition::OperationResolution(recovered) =
            disposition
        else {
            panic!("pre-attempt Invoke disposition")
        };
        let now = current.at_time(UnixTimeV1::new(current.now.get() + 10).unwrap());
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let summary = host
            .with_recovered_operation_resolution(
                &recovered,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                ResidentProcessOperationResolutionInput {
                    session,
                    current: &now,
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority,
                    },
                    selection_ref: record_ref(&format!("invoke-resolution/{ordinal}/selection")),
                    observation_ref: record_ref(&format!(
                        "invoke-resolution/{ordinal}/observation"
                    )),
                    maximum_selection_bytes: 1 << 20,
                    maximum_record_bytes: 1 << 20,
                },
                &mut ProvingAuthority,
                summarize_handoff,
            )
            .expect("current authority and custody")
            .expect("owner accepts the exact handoff");
        assert_eq!(summary.context_bound, ordinal == 2);
        assert_eq!(
            summary.predecessor_phase,
            if ordinal == 2 {
                "context_bound"
            } else {
                "resolving"
            }
        );
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
    });
    assert_eq!(case.get(), 4);
}

#[test]
fn evidence_resolution_retains_context_before_the_owner_handoff() {
    let seen = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(0));
    let counted_seen = Rc::clone(&seen);
    let counted_completed = Rc::clone(&completed);
    with_process_host_operation_resolution_restart_fixtures(&mut move |fixture| {
        let ordinal = counted_seen.replace(counted_seen.get() + 1);
        if ordinal != 1 {
            return;
        }
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            session,
            current,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let checkpoint_authority = recovery.operation.context.checkpoint_authority;
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let input = restart_input(recovery, 1 << 20);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &input),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
            .expect("evidence-bearing resolving classification");
        let ResidentProcessOperationRestartDisposition::OperationResolution(recovered) =
            disposition
        else {
            panic!("evidence-bearing resolving has one owner")
        };
        let now = current.at_time(UnixTimeV1::new(current.now.get() + 10).unwrap());
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let resolver = SelectedStore(selected_store.clone());
        let evidence_ref = ImmutableRecordReferenceV1 {
            record_ref: record_ref("invoke-resolution/context-evidence"),
            record_sha256: ArtifactDigest::hash_bytes(b"invoke-resolution-context"),
        };
        let context_input = |evidence, observation_ref, checkpoint_ref, checkpoint_id: &str| {
            ResidentProcessOperationContextInput {
                session,
                current: &now,
                evidence,
                observation_ref,
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority,
                },
                checkpoint: OperationCheckpointInput {
                    reference: checkpoint_ref,
                    id: ControlCheckpointIdV1::parse(checkpoint_id).unwrap(),
                },
                checkpoint_issued_at: UnixTimeV1::new(now.now.get() + 1).unwrap(),
                checkpoint_authority,
                maximum_record_bytes: 1 << 20,
            }
        };
        let admitted = Cell::new(0);
        let observation_signs = Cell::new(0);
        let checkpoint_ref = recovered
            .start
            .control
            .observation()
            .current_checkpoint()
            .0
            .clone();
        let rejected = host.prepare_recovered_operation_context(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            context_input(
                ImmutableRecordReferenceV1 {
                    record_ref: checkpoint_ref,
                    record_sha256: ArtifactDigest::hash_bytes(b"checkpoint-alias"),
                },
                record_ref("invoke-resolution/alias-observation"),
                record_ref("invoke-resolution/alias-checkpoint"),
                "invoke-resolution-alias-checkpoint",
            ),
            &mut ProvingAuthority,
            |_, _| unreachable!("aliased evidence cannot reach context admission"),
            |_, _, _, _| unreachable!("aliased evidence cannot reach rotation admission"),
            |_, _, _| unreachable!("aliased evidence cannot reach signing"),
        );
        assert!(matches!(
            rejected,
            Err(ResidentProcessOperationResolutionFailure::Selection(
                ResidentProcessHostSelectionError::OperationResolutionCheckpointAuthority
            ))
        ));
        let prepared = host
            .prepare_recovered_operation_context(
                &recovered,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                context_input(
                    evidence_ref.clone(),
                    record_ref("invoke-resolution/context-bound"),
                    record_ref("invoke-resolution/context-checkpoint"),
                    "invoke-resolution-context-checkpoint",
                ),
                &mut ProvingAuthority,
                |actual, obligations| {
                    admitted.set(admitted.get() + 1);
                    assert_eq!(actual, &evidence_ref);
                    assert!(!obligations.is_empty());
                    Ok(())
                },
                |_, _, _, _| Ok(()),
                |_, _, _| {
                    observation_signs.set(observation_signs.get() + 1);
                    Ok(signature())
                },
            )
            .expect("context evidence admission");
        assert_eq!((admitted.get(), observation_signs.get()), (1, 1));

        let mut append_custody = AppendCustody::committing(custody);
        let mut append_anchor = AppendAnchor::available(anchor);
        let checkpoint = host
            .prepare_operation_context_checkpoint(
                &prepared,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                &resolver,
                accumulator_limits(),
            )
            .expect("selected context checkpoint")
            .expect("context roots");
        let checkpoint_signs = Cell::new(0);
        let issued = host
            .issue_operation_context_checkpoint(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                checkpoint,
                &checkpoint_schema,
                checkpoint_authority,
                |_, _, _| {
                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                    Ok(signature())
                },
            )
            .expect("selected checkpoint issuer")
            .expect("context checkpoint signs once");
        let completed_context = host
            .retain_operation_context_checkpoint(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                issued,
                &resolver,
                accumulator_limits(),
            )
            .expect("selected context retention")
            .unwrap_or_else(|_| panic!("context custody completes"));
        assert_eq!(checkpoint_signs.get(), 1);
        assert_eq!(append_custody.counts(), (1, 1));
        assert_eq!(append_anchor.counts(), (1, 1));
        assert_eq!(
            completed_context.observation().1.state(),
            &OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ContextBound {
                    evidence: evidence_ref,
                },
            }
        );
        let later = current.at_time(UnixTimeV1::new(now.now.get() + 2).unwrap());
        let summary = host
            .with_completed_operation_context(
                &recovered,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                ResidentProcessOperationResolutionInput {
                    session,
                    current: &later,
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority,
                    },
                    selection_ref: record_ref("invoke-resolution/selection"),
                    observation_ref: record_ref("invoke-resolution/preparation"),
                    maximum_selection_bytes: 1 << 20,
                    maximum_record_bytes: 1 << 20,
                },
                &completed_context,
                &mut ProvingAuthority,
                summarize_handoff,
            )
            .expect("completed context authority")
            .expect("owner accepts completed context");
        assert!(summary.context_bound);
        assert_eq!(summary.predecessor_phase, "context_bound");
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
        counted_completed.set(counted_completed.get() + 1);
    });
    assert_eq!(seen.get(), 4);
    assert_eq!(completed.get(), 1);
}

#[test]
fn context_binding_commit_and_anchor_ambiguity_keep_exact_continuations() {
    let seen = Rc::new(Cell::new(0));
    let completed = Rc::new(Cell::new(0));
    let counted_seen = Rc::clone(&seen);
    let counted_completed = Rc::clone(&completed);
    with_process_host_operation_resolution_restart_fixtures(&mut move |fixture| {
        let ordinal = counted_seen.replace(counted_seen.get() + 1);
        if ordinal != 1 {
            return;
        }
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            session,
            current,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let checkpoint_authority = recovery.operation.context.checkpoint_authority;
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let input = restart_input(recovery, 1 << 20);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &input),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
            .expect("resolving classification");
        let ResidentProcessOperationRestartDisposition::OperationResolution(recovered) =
            disposition
        else {
            panic!("resolving disposition")
        };
        let now = current.at_time(UnixTimeV1::new(current.now.get() + 10).unwrap());
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let resolver = SelectedStore(selected_store.clone());
        let prepared = host
            .prepare_recovered_operation_context(
                &recovered,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                ResidentProcessOperationContextInput {
                    session,
                    current: &now,
                    evidence: ImmutableRecordReferenceV1 {
                        record_ref: record_ref("invoke-ambiguity/evidence"),
                        record_sha256: ArtifactDigest::hash_bytes(b"invoke-ambiguity-evidence"),
                    },
                    observation_ref: record_ref("invoke-ambiguity/context-bound"),
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority,
                    },
                    checkpoint: OperationCheckpointInput {
                        reference: record_ref("invoke-ambiguity/checkpoint"),
                        id: ControlCheckpointIdV1::parse("invoke-ambiguity-checkpoint").unwrap(),
                    },
                    checkpoint_issued_at: UnixTimeV1::new(now.now.get() + 1).unwrap(),
                    checkpoint_authority,
                    maximum_record_bytes: 1 << 20,
                },
                &mut ProvingAuthority,
                |_, obligations| {
                    assert!(!obligations.is_empty());
                    Ok(())
                },
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
            )
            .expect("context preparation");
        let mut append_custody = AppendCustody::committing(custody);
        let mut append_anchor = AppendAnchor::available(anchor);
        let checkpoint = host
            .prepare_operation_context_checkpoint(
                &prepared,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                &resolver,
                accumulator_limits(),
            )
            .expect("selected checkpoint")
            .expect("prepared roots");
        let signs = Cell::new(0);
        let issued = host
            .issue_operation_context_checkpoint(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                checkpoint,
                &checkpoint_schema,
                checkpoint_authority,
                |_, _, _| {
                    signs.set(signs.get() + 1);
                    Ok(signature())
                },
            )
            .expect("selected issuer")
            .expect("checkpoint signs once");
        append_custody.fail_next_commit(ControlStoreErrorKind::OutcomeUnknown);
        append_custody.push_read_back(OperationCustodyReadBackOutcome::Predecessor);
        append_custody.fail_next_read_back(ControlStoreErrorKind::Storage);
        append_custody.push_read_back(OperationCustodyReadBackOutcome::ExactSuccessor);
        let first = host
            .retain_operation_context_checkpoint(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                issued,
                &resolver,
                accumulator_limits(),
            )
            .expect("selected retention");
        let Err(ResidentProcessOperationContextCustodyFailure::Commit(failure)) = first else {
            panic!("unknown commit retains one exact candidate")
        };
        let pending = host
            .reconcile_operation_context_checkpoint(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                *failure,
            )
            .expect("selected reconciliation")
            .unwrap_or_else(|_| panic!("exact read-back resolves the candidate"));
        let completed_context = host
            .continue_operation_context_anchor(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    sessions,
                    anchor: &mut append_anchor,
                },
                pending,
                &resolver,
            )
            .expect("selected anchor continuation")
            .unwrap_or_else(|_| panic!("exact anchor continuation completes"));
        assert_eq!(signs.get(), 1);
        assert_eq!(
            completed_context.custody().candidate().kind(),
            pale_ports::OperationCustodyTransitionKind::Observe
        );
        assert_eq!(append_custody.counts(), (1, 3));
        assert_eq!(append_anchor.counts(), (1, 1));
        counted_completed.set(counted_completed.get() + 1);
    });
    assert_eq!(seen.get(), 4);
    assert_eq!(completed.get(), 1);
}

#[test]
fn cold_handoff_stops_before_owner_code_when_head_or_anchor_changes() {
    let seen = Rc::new(Cell::new(0));
    let rejected = Rc::new(Cell::new(0));
    let owner_calls = Rc::new(Cell::new(0));
    let counted_seen = Rc::clone(&seen);
    let counted_rejected = Rc::clone(&rejected);
    let counted_owner_calls = Rc::clone(&owner_calls);
    with_process_host_operation_resolution_restart_fixtures(&mut move |fixture| {
        let ordinal = counted_seen.replace(counted_seen.get() + 1);
        if ordinal != 0 && ordinal != 2 {
            return;
        }
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            session,
            current,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let input = restart_input(recovery, 1 << 20);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &input),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
            .expect("pre-attempt classification");
        let ResidentProcessOperationRestartDisposition::OperationResolution(recovered) =
            disposition
        else {
            panic!("pre-attempt Invoke disposition")
        };
        if ordinal == 0 {
            custody.remove_current_observation();
        } else {
            anchor.make_unavailable();
        }
        let now = current.at_time(UnixTimeV1::new(current.now.get() + 10).unwrap());
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let result = host.with_recovered_operation_resolution(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            ResidentProcessOperationResolutionInput {
                session,
                current: &now,
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority,
                },
                selection_ref: record_ref("invoke-hostile/selection"),
                observation_ref: record_ref("invoke-hostile/observation"),
                maximum_selection_bytes: 1 << 20,
                maximum_record_bytes: 1 << 20,
            },
            &mut ProvingAuthority,
            |_, _| {
                counted_owner_calls.set(counted_owner_calls.get() + 1);
                Ok::<(), ()>(())
            },
        );
        assert!(matches!(
            result,
            Err(ResidentProcessOperationResolutionFailure::Admission(_))
        ));
        counted_rejected.set(counted_rejected.get() + 1);
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
    });
    assert_eq!(seen.get(), 4);
    assert_eq!(rejected.get(), 2);
    assert_eq!(owner_calls.get(), 0);
}

#[test]
fn cold_handoff_rejects_zero_bounds_and_aliased_attempt_references() {
    let seen = Rc::new(Cell::new(0));
    let rejected = Rc::new(Cell::new(0));
    let owner_calls = Rc::new(Cell::new(0));
    let counted_seen = Rc::clone(&seen);
    let counted_rejected = Rc::clone(&rejected);
    let counted_owner_calls = Rc::clone(&owner_calls);
    with_process_host_operation_resolution_restart_fixtures(&mut move |fixture| {
        let ordinal = counted_seen.replace(counted_seen.get() + 1);
        if ordinal != 0 && ordinal != 2 {
            return;
        }
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            session,
            current,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let input = restart_input(recovery, 1 << 20);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &input),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
            .expect("pre-attempt classification");
        let ResidentProcessOperationRestartDisposition::OperationResolution(recovered) =
            disposition
        else {
            panic!("pre-attempt Invoke disposition")
        };
        let now = current.at_time(UnixTimeV1::new(current.now.get() + 10).unwrap());
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let selection_ref = if ordinal == 0 {
            record_ref("invoke-hostile/zero-bound-selection")
        } else {
            match recovered
                .start
                .control
                .observation()
                .selected_head()
                .1
                .state()
            {
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ContextBound { evidence },
                } => evidence.record_ref.clone(),
                _ => panic!("context-bound evidence"),
            }
        };
        let result = host.with_recovered_operation_resolution(
            &recovered,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            ResidentProcessOperationResolutionInput {
                session,
                current: &now,
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority,
                },
                selection_ref,
                observation_ref: record_ref("invoke-hostile/distinct-observation"),
                maximum_selection_bytes: if ordinal == 0 { 0 } else { 1 << 20 },
                maximum_record_bytes: 1 << 20,
            },
            &mut ProvingAuthority,
            |_, _| {
                counted_owner_calls.set(counted_owner_calls.get() + 1);
                Ok::<(), ()>(())
            },
        );
        assert!(matches!(
            result,
            Err(ResidentProcessOperationResolutionFailure::Admission(
                OperationAttemptError::BoundExceeded | OperationAttemptError::Association
            ))
        ));
        counted_rejected.set(counted_rejected.get() + 1);
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.operations.outstanding_task_reservations(), 0);
    });
    assert_eq!(seen.get(), 4);
    assert_eq!(rejected.get(), 2);
    assert_eq!(owner_calls.get(), 0);
}
