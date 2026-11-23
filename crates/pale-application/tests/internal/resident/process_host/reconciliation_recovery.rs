//! Recovered root-Invoke reconciliation through one exact process host.

use super::provider_recovery::{AppendAnchor, AppendCustody};
use super::restart::{association, host_without_generation, restart_input};
use super::*;
use crate::local_operation_start_fixture::{record_ref, schema_digest, signature};
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_reconciliation_fixtures, ProcessHostRestartFixture,
};
use crate::resident::process_host::reconciliation_recovery::*;
use crate::resident::process_host::restart::*;
use crate::resident::{
    OperationCheckpointInput, OperationObservationAuthorityStoreRejection,
    OperationObservationAuthorityStoreResolver,
};
use pale_core::records::{
    ArtifactDigest, CapabilityResultDigest, CapabilityResultReferenceV1, CompiledGenerationDigest,
    ControlCheckpointIdV1, ImmutableRecordReferenceV1, OperationObservationStateV1,
    SignedResidentAdmissionContextV1, CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::ControlStoreErrorKind;
use pale_ports::{AccumulatorPreparationLimits, AuthorityStoreCoordinate};
use std::cell::Cell;
use std::collections::BTreeSet;
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

fn terminal_state(kind: &str) -> OperationObservationStateV1 {
    match kind {
        "succeeded" => OperationObservationStateV1::Succeeded {
            result: CapabilityResultReferenceV1 {
                record_ref: record_ref("reconciliation/result"),
                record_sha256: CapabilityResultDigest::parse("a".repeat(64)).unwrap(),
            },
            effect_receipt: None,
        },
        "failed" => OperationObservationStateV1::Failed {
            result: None,
            diagnostic: ImmutableRecordReferenceV1 {
                record_ref: record_ref("reconciliation/failed"),
                record_sha256: ArtifactDigest::parse("b".repeat(64)).unwrap(),
            },
        },
        "cancelled" => OperationObservationStateV1::Cancelled {
            diagnostic: ImmutableRecordReferenceV1 {
                record_ref: record_ref("reconciliation/cancelled"),
                record_sha256: ArtifactDigest::parse("c".repeat(64)).unwrap(),
            },
        },
        _ => panic!("unknown terminal state"),
    }
}

fn source_name(state: &OperationObservationStateV1) -> Option<&'static str> {
    match state {
        OperationObservationStateV1::OutcomeUnknown { .. } => Some("outcome_unknown"),
        OperationObservationStateV1::Recovering { .. } => Some("recovering"),
        OperationObservationStateV1::Expired { .. } => Some("expired"),
        _ => None,
    }
}

fn limits() -> AccumulatorPreparationLimits {
    AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000)
        .expect("bounded reconciliation accumulator")
}

struct ReconciliationCase<'a> {
    host: ResidentProcessHost<'a, UnprovedHeldWriter>,
    start: ResidentProcessRecoveredOperationStart,
    store: AuthorityStoreCoordinate,
    signing: &'a SigningAuthorityBindingV1,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    custody: &'a mut crate::resident::operation_observation_custody::tests::local_recovery::RecoveryStore,
    anchor: &'a mut crate::resident::operation_observation_custody::tests::local_recovery::ReadAnchor,
}

fn with_outcome_unknown_reconciliation<Run>(mut run: Run)
where
    Run: for<'a> FnMut(ReconciliationCase<'a>) + 'static,
{
    let called = Rc::new(Cell::new(false));
    let did_call = Rc::clone(&called);
    with_process_host_reconciliation_fixtures(&mut move |fixture: ProcessHostRestartFixture<
        '_,
    >| {
        if did_call.get() {
            return;
        }
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            evidence,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let checkpoint_authority = recovery.operation.context.checkpoint_authority;
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_without_generation(&port, &selector, &selected_store);
        let restart = restart_input(recovery, 1024 * 1024);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("outcome-unknown fixture classifies");
        let ResidentProcessOperationRestartDisposition::RecoveryOnly(start) = disposition else {
            return;
        };
        if !matches!(
            start.control.observation().selected_head().1.state(),
            OperationObservationStateV1::OutcomeUnknown { .. }
        ) {
            return;
        }
        did_call.set(true);
        run(ReconciliationCase {
            host,
            start: *start,
            store: selected_store,
            signing: signing_authority,
            checkpoint_authority,
            custody,
            anchor,
        });
    });
    assert!(called.get());
}

fn case_input<'a>(
    case: &ReconciliationCase<'a>,
    suffix: &str,
    state: OperationObservationStateV1,
    signed: SignedResidentAdmissionContextV1<'a>,
    checkpoint_schema: &'a SchemaDigest,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    maximum_record_bytes: usize,
) -> ResidentProcessReconciliationInput<'a> {
    let observed_at = std::cmp::max(
        *case
            .start
            .control
            .observation()
            .selected_head()
            .1
            .observed_at(),
        *case
            .start
            .control
            .observation()
            .current_checkpoint()
            .1
            .issued_at(),
    );
    ResidentProcessReconciliationInput {
        successor_ref: record_ref(&format!("reconciliation/{suffix}/observation")),
        state,
        observed_at,
        signed,
        checkpoint: OperationCheckpointInput {
            reference: record_ref(&format!("reconciliation/{suffix}/checkpoint")),
            id: ControlCheckpointIdV1::parse(format!("reconciliation-{suffix}"))
                .expect("checkpoint identity"),
        },
        checkpoint_schema,
        checkpoint_authority,
        accumulator_limits: limits(),
        maximum_record_bytes,
    }
}

#[test]
fn recovered_uncertain_heads_accept_only_admitted_terminal_evidence_after_generation_eviction() {
    let seen = Rc::new(std::cell::RefCell::new(BTreeSet::new()));
    for target in ["succeeded", "failed", "cancelled"] {
        reconcile_target(target, &seen);
    }
    assert_eq!(seen.borrow().len(), 9);
}

fn reconcile_target(
    target: &'static str,
    seen: &Rc<std::cell::RefCell<BTreeSet<(&'static str, &'static str)>>>,
) {
    let observed = Rc::clone(seen);
    with_process_host_reconciliation_fixtures(&mut move |fixture: ProcessHostRestartFixture<
        '_,
    >| {
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            evidence,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let checkpoint_authority = recovery.operation.context.checkpoint_authority;
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_without_generation(&port, &selector, &selected_store);
        let restart = restart_input(recovery, 1024 * 1024);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("generation-independent head classifies");
        let ResidentProcessOperationRestartDisposition::RecoveryOnly(start) = disposition else {
            return;
        };
        let Some(source) = source_name(start.control.observation().selected_head().1.state())
        else {
            return;
        };
        let observed_at = std::cmp::max(
            *start.control.observation().selected_head().1.observed_at(),
            *start
                .control
                .observation()
                .current_checkpoint()
                .1
                .issued_at(),
        );

        let key = (source, target);
        if observed.borrow().contains(&key) {
            return;
        }
        observed.borrow_mut().insert(key);
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let resolver = SelectedStore(selected_store.clone());
        let admitted = Cell::new(0);
        let observation_signs = Cell::new(0);
        let checkpoint_signs = Cell::new(0);
        let expected = terminal_state(target);
        let mut append_custody = AppendCustody::committing(custody);
        let mut append_anchor = AppendAnchor::available(anchor);
        let completed = host
            .reconcile_recovered_invoke(
                *start,
                ResidentDeploymentReconciliationAccess {
                    store: &selected_store,
                    custody: &mut append_custody,
                    anchor: &mut append_anchor,
                },
                ResidentProcessReconciliationInput {
                    successor_ref: record_ref(&format!(
                        "reconciliation/{source}/{target}/observation"
                    )),
                    state: expected.clone(),
                    observed_at,
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority,
                    },
                    checkpoint: OperationCheckpointInput {
                        reference: record_ref(&format!(
                            "reconciliation/{source}/{target}/checkpoint"
                        )),
                        id: ControlCheckpointIdV1::parse(format!(
                            "reconciliation-{source}-{target}"
                        ))
                        .unwrap(),
                    },
                    checkpoint_schema: &checkpoint_schema,
                    checkpoint_authority,
                    accumulator_limits: limits(),
                    maximum_record_bytes: 1024 * 1024,
                },
                &resolver,
                |state| {
                    admitted.set(admitted.get() + 1);
                    assert_eq!(state, &expected);
                    Ok(())
                },
                |_, _, _, _| Ok(()),
                |_, _, _| {
                    observation_signs.set(observation_signs.get() + 1);
                    Ok(signature())
                },
                |_, _, _| {
                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                    Ok(signature())
                },
            )
            .expect("admitted terminal evidence reaches exact custody");
        assert_eq!(completed.successor().1.state(), &expected);
        assert_eq!(admitted.get(), 1);
        assert_eq!(observation_signs.get(), 1);
        assert_eq!(checkpoint_signs.get(), 1);
        assert_eq!(append_custody.counts(), (1, 1));
        assert_eq!(append_anchor.counts().0, 1);
    });
}

#[test]
fn changed_physical_head_or_anchor_stops_before_evidence_admission() {
    for stale in ["head", "checkpoint", "anchor"] {
        with_outcome_unknown_reconciliation(move |case| {
            match stale {
                "head" => case.custody.remove_current_observation(),
                "checkpoint" => case.custody.substitute_current_checkpoint(),
                "anchor" => case.anchor.make_unavailable(),
                _ => unreachable!(),
            }
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
            let input = case_input(
                &case,
                &format!("stale-{stale}"),
                terminal_state("failed"),
                SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority: case.signing,
                },
                &checkpoint_schema,
                case.checkpoint_authority,
                1024 * 1024,
            );
            let resolver = SelectedStore(case.store.clone());
            let calls = Cell::new(0);
            let result = case.host.reconcile_recovered_invoke(
                case.start,
                ResidentDeploymentReconciliationAccess {
                    store: &case.store,
                    custody: case.custody,
                    anchor: case.anchor,
                },
                input,
                &resolver,
                |_| {
                    calls.set(calls.get() + 1);
                    Ok(())
                },
                |_, _, _, _| panic!("stale custody cannot rotate authority"),
                |_, _, _| panic!("stale custody cannot sign an observation"),
                |_, _, _| panic!("stale custody cannot sign a checkpoint"),
            );
            assert!(matches!(
                result,
                Err(ResidentProcessReconciliationFailure::HeadChanged { .. })
            ));
            assert_eq!(calls.get(), 0);
        });
    }
}

#[test]
fn access_signer_state_and_bounds_are_rejected_before_evidence_or_signing() {
    for rejected_axis in [
        "store",
        "generation",
        "signer",
        "observation_schema",
        "checkpoint",
        "checkpoint_schema",
        "state",
        "bounds",
    ] {
        with_outcome_unknown_reconciliation(move |mut case| {
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
            let foreign_schema = SchemaDigest::parse("e".repeat(64)).unwrap();
            let selected_observation_schema = if rejected_axis == "observation_schema" {
                &foreign_schema
            } else {
                &observation_schema
            };
            let selected_checkpoint_schema = if rejected_axis == "checkpoint_schema" {
                &foreign_schema
            } else {
                &checkpoint_schema
            };
            let foreign_signing =
                owner_native_signing_authority(case.start.selector.principal_id.clone());
            let signing = if rejected_axis == "signer" {
                &foreign_signing
            } else {
                case.signing
            };
            if rejected_axis == "generation" {
                case.start.generation_digest =
                    CompiledGenerationDigest::parse("9".repeat(64)).unwrap();
            }
            let foreign_principal =
                ResidentPrincipalIdV1::parse("foreign-reconciliation-principal").unwrap();
            let foreign_checkpoint = ControlCheckpointSigningAuthorityAdmissionContextV1 {
                principal_id: &foreign_principal,
                ..*case.checkpoint_authority
            };
            let checkpoint_authority = if rejected_axis == "checkpoint" {
                &foreign_checkpoint
            } else {
                case.checkpoint_authority
            };
            let state = if rejected_axis == "state" {
                OperationObservationStateV1::Recovering {
                    reconciliation: pale_core::records::ReconciliationFrontierReferenceV1 {
                        record_ref: record_ref("reconciliation/not-terminal"),
                        record_sha256: pale_core::records::ReconciliationFrontierDigest::parse(
                            "d".repeat(64),
                        )
                        .unwrap(),
                    },
                }
            } else {
                terminal_state("failed")
            };
            let input = case_input(
                &case,
                rejected_axis,
                state,
                SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: selected_observation_schema,
                    signing_authority: signing,
                },
                selected_checkpoint_schema,
                checkpoint_authority,
                if rejected_axis == "bounds" {
                    0
                } else {
                    1024 * 1024
                },
            );
            let foreign_store = AuthorityStoreCoordinate::new(
                pale_core::records::ControlStoreIdV1::parse("foreign-reconciliation-store")
                    .unwrap(),
                pale_core::records::ControlIncarnationIdV1::parse(
                    "foreign-reconciliation-incarnation",
                )
                .unwrap(),
            );
            let access_store = if rejected_axis == "store" {
                &foreign_store
            } else {
                &case.store
            };
            let resolver = SelectedStore(case.store.clone());
            let calls = Cell::new(0);
            let result = case.host.reconcile_recovered_invoke(
                case.start,
                ResidentDeploymentReconciliationAccess {
                    store: access_store,
                    custody: case.custody,
                    anchor: case.anchor,
                },
                input,
                &resolver,
                |_| {
                    calls.set(calls.get() + 1);
                    Ok(())
                },
                |_, _, _, _| panic!("rejected selection cannot rotate authority"),
                |_, _, _| panic!("rejected selection cannot sign an observation"),
                |_, _, _| panic!("rejected selection cannot sign a checkpoint"),
            );
            assert!(matches!(
                result,
                Err(ResidentProcessReconciliationFailure::Selection { .. })
            ));
            assert_eq!(calls.get(), 0);
        });
    }
}

#[test]
fn rejected_owner_evidence_never_signs_or_writes() {
    with_outcome_unknown_reconciliation(|case| {
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let input = case_input(
            &case,
            "evidence-rejected",
            terminal_state("failed"),
            SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &observation_schema,
                signing_authority: case.signing,
            },
            &checkpoint_schema,
            case.checkpoint_authority,
            1024 * 1024,
        );
        let resolver = SelectedStore(case.store.clone());
        let evidence_calls = Cell::new(0);
        let mut append_custody = AppendCustody::committing(case.custody);
        let mut append_anchor = AppendAnchor::available(case.anchor);
        let result = case.host.reconcile_recovered_invoke(
            case.start,
            ResidentDeploymentReconciliationAccess {
                store: &case.store,
                custody: &mut append_custody,
                anchor: &mut append_anchor,
            },
            input,
            &resolver,
            |_| {
                evidence_calls.set(evidence_calls.get() + 1);
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "modeled reconciliation evidence",
                    invariant: "admitted conclusive owner evidence",
                })
            },
            |_, _, _, _| panic!("rejected evidence cannot rotate authority"),
            |_, _, _| panic!("rejected evidence cannot sign an observation"),
            |_, _, _| panic!("rejected evidence cannot sign a checkpoint"),
        );
        assert!(matches!(
            result,
            Err(ResidentProcessReconciliationFailure::BeforeSuccessor(_))
        ));
        assert_eq!(evidence_calls.get(), 1);
        assert_eq!(append_custody.counts(), (0, 0));
        assert_eq!(append_anchor.counts(), (0, 0));
    });
}

#[test]
fn prepared_continuation_rereads_custody_before_checkpoint_signing() {
    with_outcome_unknown_reconciliation(|case| {
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let input = case_input(
            &case,
            "prepared-stale-custody",
            terminal_state("failed"),
            SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &observation_schema,
                signing_authority: case.signing,
            },
            &checkpoint_schema,
            case.checkpoint_authority,
            1024 * 1024,
        );
        let resolver = SelectedStore(case.store.clone());
        let observation_signs = Cell::new(0);
        let checkpoint_signs = Cell::new(0);
        let first = {
            let mut append_custody = AppendCustody::committing(&mut *case.custody);
            let mut append_anchor = AppendAnchor::available(&mut *case.anchor);
            case.host.reconcile_recovered_invoke(
                case.start,
                ResidentDeploymentReconciliationAccess {
                    store: &case.store,
                    custody: &mut append_custody,
                    anchor: &mut append_anchor,
                },
                input,
                &resolver,
                |_| Ok(()),
                |_, _, _, _| Ok(()),
                |_, _, _| {
                    observation_signs.set(observation_signs.get() + 1);
                    Ok(signature())
                },
                |_, _, _| {
                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "modeled checkpoint signer",
                        invariant: "first checkpoint signing unavailable",
                    })
                },
            )
        };
        let Err(ResidentProcessReconciliationFailure::Prepared { continuation, .. }) = first else {
            panic!("checkpoint signing failure retains the prepared successor")
        };

        case.custody.substitute_current_checkpoint();
        let mut append_custody = AppendCustody::committing(&mut *case.custody);
        let mut append_anchor = AppendAnchor::available(&mut *case.anchor);
        let resumed = case.host.continue_prepared_reconciliation(
            *continuation,
            ResidentDeploymentReconciliationAccess {
                store: &case.store,
                custody: &mut append_custody,
                anchor: &mut append_anchor,
            },
            &resolver,
            |_, _, _| panic!("stale predecessor cannot reach checkpoint signing"),
        );
        assert!(matches!(
            resumed,
            Err(ResidentProcessReconciliationFailure::PreparedHeadChanged { .. })
        ));
        assert_eq!(observation_signs.get(), 1);
        assert_eq!(checkpoint_signs.get(), 1);
        assert_eq!(append_custody.counts(), (0, 0));
    });
}

#[test]
fn prepared_commit_and_anchor_continuations_do_not_resign_the_successor() {
    for phase in ["prepared", "commit", "anchor"] {
        with_outcome_unknown_reconciliation(move |case| {
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
            let input = case_input(
                &case,
                phase,
                terminal_state("succeeded"),
                SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority: case.signing,
                },
                &checkpoint_schema,
                case.checkpoint_authority,
                1024 * 1024,
            );
            let resolver = SelectedStore(case.store.clone());
            let observation_signs = Cell::new(0);
            let checkpoint_signs = Cell::new(0);
            let mut append_custody = AppendCustody::committing(case.custody);
            let mut append_anchor = AppendAnchor::available(case.anchor);
            if phase == "commit" {
                append_custody.fail_next_commit(ControlStoreErrorKind::Storage);
            }
            if phase == "anchor" {
                append_anchor.fail_next_advance();
            }
            let first = case.host.reconcile_recovered_invoke(
                case.start,
                ResidentDeploymentReconciliationAccess {
                    store: &case.store,
                    custody: &mut append_custody,
                    anchor: &mut append_anchor,
                },
                input,
                &resolver,
                |_| Ok(()),
                |_, _, _, _| Ok(()),
                |_, _, _| {
                    observation_signs.set(observation_signs.get() + 1);
                    Ok(signature())
                },
                |_, _, _| {
                    checkpoint_signs.set(checkpoint_signs.get() + 1);
                    if phase == "prepared" && checkpoint_signs.get() == 1 {
                        Err(InvalidResidentRecordV1::ContextualAdmission {
                            record: "modeled checkpoint signer",
                            invariant: "first checkpoint signing unavailable",
                        })
                    } else {
                        Ok(signature())
                    }
                },
            );
            let completed = match (phase, first) {
                (
                    "prepared",
                    Err(ResidentProcessReconciliationFailure::Prepared { continuation, .. }),
                ) => {
                    let foreign_store = AuthorityStoreCoordinate::new(
                        pale_core::records::ControlStoreIdV1::parse(
                            "foreign-prepared-continuation-store",
                        )
                        .unwrap(),
                        pale_core::records::ControlIncarnationIdV1::parse(
                            "foreign-prepared-continuation-incarnation",
                        )
                        .unwrap(),
                    );
                    let rejected = case.host.continue_prepared_reconciliation(
                        *continuation,
                        ResidentDeploymentReconciliationAccess {
                            store: &foreign_store,
                            custody: &mut append_custody,
                            anchor: &mut append_anchor,
                        },
                        &resolver,
                        |_, _, _| panic!("wrong-store continuation cannot sign"),
                    );
                    let Err(ResidentProcessReconciliationFailure::Selection {
                        continuation, ..
                    }) = rejected
                    else {
                        panic!("wrong store preserves the prepared continuation")
                    };
                    let ResidentProcessReconciliationContinuation::Prepared(continuation) =
                        *continuation
                    else {
                        panic!("prepared selection retains the same phase")
                    };
                    case.host
                        .continue_prepared_reconciliation(
                            *continuation,
                            ResidentDeploymentReconciliationAccess {
                                store: &case.store,
                                custody: &mut append_custody,
                                anchor: &mut append_anchor,
                            },
                            &resolver,
                            |_, _, _| {
                                checkpoint_signs.set(checkpoint_signs.get() + 1);
                                Ok(signature())
                            },
                        )
                        .expect("prepared successor continues with checkpoint signing only")
                }
                ("commit", Err(ResidentProcessReconciliationFailure::Commit(continuation))) => {
                    let retryable = continuation
                        .try_into_retryable()
                        .expect("definitive non-commit permits the exact retry");
                    case.host
                        .retry_reconciliation_commit(
                            retryable,
                            ResidentDeploymentReconciliationAccess {
                                store: &case.store,
                                custody: &mut append_custody,
                                anchor: &mut append_anchor,
                            },
                        )
                        .expect("exact retained candidate commits")
                }
                ("anchor", Err(ResidentProcessReconciliationFailure::Anchor(continuation))) => case
                    .host
                    .continue_reconciliation_anchor(
                        *continuation,
                        ResidentDeploymentReconciliationAccess {
                            store: &case.store,
                            custody: &mut append_custody,
                            anchor: &mut append_anchor,
                        },
                    )
                    .expect("exact retained anchor transition completes"),
                (_, Ok(_)) => panic!("modeled phase must stop at its exact continuation"),
                (_, Err(error)) => panic!("unexpected reconciliation phase: {error:?}"),
            };
            assert!(matches!(
                completed.successor().1.state(),
                OperationObservationStateV1::Succeeded { .. }
            ));
            assert_eq!(observation_signs.get(), 1);
            assert_eq!(
                checkpoint_signs.get(),
                if phase == "prepared" { 2 } else { 1 }
            );
        });
    }
}
