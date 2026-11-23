//! Hostile source and roster admission for the cold Consult-child sweep.

use super::consult_child_recovery_sweep::{
    with_cold_consult_child_sweep, with_cold_consult_child_sweep_staged,
    ColdConsultCollectionSelection,
};
use super::*;
use crate::local_operation_start_fixture::{
    checkpoint_authority, issue_observation, record_ref, schema_digest, signature,
};
use crate::resident::consultation::tests::{
    consultation_supervision_fixture_for_operation_in_store,
    consultation_supervision_fixture_for_self_contained_operation_in_store,
    consultation_supervision_fixture_in_store,
    consultation_supervision_fixture_with_retained_evidence_in_store,
    prepare_collection_accumulator, recover_completed_collection, CollectionCommitPort,
};
use crate::resident::consultation::{
    ConsultationCollectionLimits, ConsultationSuccessorCommitInput,
};
use crate::resident::operation_dispatch::OperationCheckpointInput;
use crate::resident::operation_observation_custody::tests::{
    local_start, native_execution, RecordingAnchor, RecordingStore,
};
use crate::resident::process_host::consult_child_recovery_sweep::{
    ResidentProcessConsultChildOtherOwner, ResidentProcessConsultChildRecoveryError,
    ResidentProcessConsultChildRecoveryState, ResidentProcessConsultChildRecoveryTurn,
    ResidentProcessConsultChildTerminalCommitInput, ResidentProcessConsultChildTerminalCommitTurn,
};
use crate::resident::process_host::ResidentOperationControlAssociation;
use crate::resident::{prepare_operation_observation_append, OperationObservationTransition};
use pale_core::canon;
use pale_core::records::{
    ArtifactDigest, CapabilityResultDigest, CapabilityResultReferenceV1, ControlCheckpointIdV1,
    ImmutableRecordReferenceV1, InvalidResidentRecordV1, OperationObservationAdmissionContextV1,
    OperationObservationRevisionV1, OperationObservationStateV1, OperationObservationV1,
    SignedResidentAdmissionContextV1, UnixTimeV1, CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};

fn child_coordinate(
    child: &crate::resident::consultation::tests::IssuedTestChild,
) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: child.operation.handle().0.clone(),
        record_sha256: ArtifactDigest::parse(
            child
                .operation
                .handle()
                .1
                .canonical_operation_handle_sha256()
                .as_str(),
        )
        .expect("child operation-handle digest"),
    }
}
use pale_ports::{
    AccumulatorPreparationLimits, ControlRecord, ControlStoreError, ControlStoreErrorKind,
    OperationCustodyCandidate, OperationCustodyCommitOutcome, OperationCustodyReadBackOutcome,
};

fn successful_terminal_candidate(
    child: &crate::resident::consultation::tests::IssuedTestChild,
    collection_candidate: &OperationCustodyCandidate,
    collection_checkpoint: &pale_core::records::ControlStateCheckpointV1,
) -> Option<(
    OperationCustodyCandidate,
    pale_core::records::ControlStateCheckpointV1,
)> {
    let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let request = child.request.to_preimage();
    let signing = &request.signing_authority_binding;
    let initial_ref = child.initial_record.coordinate().record_ref();
    let initial = OperationObservationV1::admit_contextual_wire(
        serde_json::from_value(
            canon::parse_strict(child.initial_record.canonical_record_jcs().as_slice())
                .expect("initial child observation JSON"),
        )
        .expect("initial child observation wire"),
        &OperationObservationAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema,
                signing_authority: signing,
            },
            handle_ref: child.operation.handle().0,
            handle: child.operation.handle().1,
            predecessor: None,
        },
        |_| Ok(()),
        |_, _, _, _| panic!("an initial child observation cannot rotate authority"),
        |_, _, _, actual| {
            if actual == &signature() {
                Ok(())
            } else {
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "operation observation",
                    invariant: "retained child observation signature",
                })
            }
        },
    )
    .expect("initial child observation admits");
    let result = CapabilityResultReferenceV1 {
        record_ref: record_ref("result/cold-consult-child-success"),
        record_sha256: CapabilityResultDigest::parse(
            ArtifactDigest::hash_bytes(b"cold consult child result").as_str(),
        )
        .expect("capability-result digest"),
    };
    let terminal_ref = record_ref("observation/cold-consult-child-success");
    let terminal = issue_observation(
        signing,
        child.operation.handle().0,
        child.operation.handle().1,
        schema,
        OperationObservationRevisionV1::new(initial.revision().get() + 1)
            .expect("terminal child revision"),
        Some((initial_ref, &initial)),
        OperationObservationStateV1::Succeeded {
            result,
            effect_receipt: None,
        },
        collection_checkpoint.issued_at().get() + 1,
    );
    let store = collection_candidate.store().clone();
    let resolve_store = |_: &OperationObservationV1| Ok(store.clone());
    let append = prepare_operation_observation_append(
        OperationObservationTransition::new(
            child.operation.handle().0,
            child.operation.handle().1,
            initial_ref,
            &initial,
            &terminal_ref,
            &terminal,
        ),
        &resolve_store,
    )
    .expect("terminal child observation projects");
    let (completed, checkpoint) = native_execution::complete_with_selection(
        append,
        child.operation.checkpoint_selection(),
        (
            collection_candidate
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref(),
            collection_checkpoint,
        ),
        crate::resident::OperationCustodyDatabaseEvidence::New,
        collection_checkpoint.issued_at().get() + 1,
    );
    Some((completed.candidate().clone(), checkpoint))
}
fn assert_root_then_same_error(
    recover: &mut dyn FnMut() -> Result<
        ResidentProcessConsultChildRecoveryTurn,
        ResidentProcessConsultChildRecoveryError,
    >,
    matches_expected: impl Fn(&ResidentProcessConsultChildRecoveryError) -> bool,
) {
    assert!(matches!(
        recover().expect("root source belongs to the root sweep"),
        ResidentProcessConsultChildRecoveryTurn::SkippedOtherOwner(
            ResidentProcessConsultChildOtherOwner::Root
        )
    ));
    for attempt in 0..2 {
        let reason = match recover() {
            Err(reason) => reason,
            Ok(_) => panic!("the hostile child source must remain rejected"),
        };
        assert!(
            matches_expected(&reason),
            "unexpected hostile rejection on attempt {attempt}: {reason:?}"
        );
    }
}

#[test]
fn missing_parent_collection_witness_keeps_the_exact_child_source_for_retry() {
    with_cold_consult_child_sweep(
        "cold-consult-child-absent-parent-collection",
        |_, _, _, current| ColdConsultCollectionSelection::absent(current),
        |_, _, _, recover| {
            assert_root_then_same_error(recover, |reason| {
                matches!(
                    reason,
                    ResidentProcessConsultChildRecoveryError::ParentCollection(_)
                )
            });
        },
    )
}

#[test]
fn unlike_parent_collection_never_advances_or_reclassifies_the_child() {
    with_cold_consult_child_sweep(
        "cold-consult-child-unlike-parent",
        |_, _, store, current| {
            ColdConsultCollectionSelection::replacement(
                current,
                consultation_supervision_fixture_in_store(
                    "operation/unlike-consult-parent",
                    "cold-consult-child-unlike-parent",
                    100,
                    false,
                    store.clone(),
                )
                .snapshot()
                .clone(),
            )
        },
        |_, _, _, recover| {
            assert_root_then_same_error(recover, |reason| {
                matches!(
                    reason,
                    ResidentProcessConsultChildRecoveryError::ParentCollection(_)
                )
            });
        },
    )
}

#[test]
fn committed_roster_substitutions_keep_the_child_source_fixed_without_dispatch() {
    for (label, substitution) in [
        ("cold-consult-child-unlike-child", RosterSubstitution::Child),
        ("cold-consult-child-unlike-slot", RosterSubstitution::Slot),
        (
            "cold-consult-child-unlike-context",
            RosterSubstitution::Context,
        ),
    ] {
        with_cold_consult_child_sweep(
            label,
            |child, parent, store, _| {
                let exact_child = child_coordinate(child);
                let collection = match substitution {
                    RosterSubstitution::Child => {
                        consultation_supervision_fixture_for_self_contained_operation_in_store(
                            parent.handle().0,
                            parent.handle().1,
                            label,
                            ImmutableRecordReferenceV1 {
                                record_ref: record_ref("operation/unlike-consult-child"),
                                record_sha256: ArtifactDigest::hash_bytes(b"unlike Consult child"),
                            },
                            false,
                            store.clone(),
                        )
                    }
                    RosterSubstitution::Slot => {
                        consultation_supervision_fixture_for_self_contained_operation_in_store(
                            parent.handle().0,
                            parent.handle().1,
                            "substituted-slot",
                            exact_child,
                            false,
                            store.clone(),
                        )
                    }
                    RosterSubstitution::Context => {
                        consultation_supervision_fixture_for_operation_in_store(
                            parent.handle().0,
                            parent.handle().1,
                            label,
                            exact_child,
                            false,
                            store.clone(),
                        )
                    }
                };
                ColdConsultCollectionSelection::exact(collection)
            },
            |_, _, _, recover| {
                assert_root_then_same_error(recover, |reason| {
                    matches!(
                        reason,
                        ResidentProcessConsultChildRecoveryError::ParentIntent
                    )
                });
            },
        );
    }
}

#[derive(Clone, Copy)]
enum RosterSubstitution {
    Child,
    Slot,
    Context,
}

#[test]
fn cold_terminal_child_commits_directly_into_its_recovered_parent() {
    prove_cold_terminal_commit(
        "cold-consult-child-terminal-success",
        TerminalCommitCase::Complete,
        TerminalParentCase::Collecting,
    );
}

#[test]
fn stopped_parent_retains_late_terminal_once_and_fresh_host_does_not_rewrite_it() {
    prove_cold_terminal_commit(
        "cold-consult-child-terminal-stopped-parent",
        TerminalCommitCase::Complete,
        TerminalParentCase::Stopped,
    );
}

#[test]
fn same_reference_unlike_digest_is_rejected_without_a_second_join() {
    with_cold_consult_child_sweep_staged(
        "cold-consult-child-unlike-retained-terminal",
        |child, parent, store, _| {
            ColdConsultCollectionSelection::exact(
                consultation_supervision_fixture_with_retained_evidence_in_store(
                    parent.handle().0,
                    parent.handle().1,
                    "cold-consult-child-unlike-retained-terminal",
                    child_coordinate(child),
                    ImmutableRecordReferenceV1 {
                        record_ref: record_ref("result/cold-consult-child-success"),
                        record_sha256: ArtifactDigest::hash_bytes(
                            b"unlike retained Consult child result",
                        ),
                    },
                    store.clone(),
                ),
            )
        },
        successful_terminal_candidate,
        |_, _, _, recover| {
            assert_root_then_same_error(recover, |reason| {
                matches!(
                    reason,
                    ResidentProcessConsultChildRecoveryError::TerminalRetention(
                        crate::resident::consultation::ConsultationCollectionError::Contribution
                    )
                )
            });
        },
        |(), _, _, _, _, _, _, _, _, _, _, _, _, _| {},
    );
}

#[test]
fn cold_terminal_child_keeps_its_cursor_through_database_and_anchor_interruptions() {
    prove_cold_terminal_commit(
        "cold-consult-child-terminal-database-retry",
        TerminalCommitCase::DatabaseRetry,
        TerminalParentCase::Collecting,
    );
    prove_cold_terminal_commit(
        "cold-consult-child-terminal-anchor-retry",
        TerminalCommitCase::AnchorRetry,
        TerminalParentCase::Collecting,
    );
}

#[derive(Clone, Copy)]
enum TerminalCommitCase {
    Complete,
    DatabaseRetry,
    AnchorRetry,
}

#[derive(Clone, Copy)]
enum TerminalParentCase {
    Collecting,
    Stopped,
}

fn prove_cold_terminal_commit(
    label: &'static str,
    case: TerminalCommitCase,
    parent_case: TerminalParentCase,
) {
    with_cold_terminal_commit(label, case, parent_case, |_, _, _, _, _, _, _, _| {});
}

/// Carry one terminal child through cold recovery and a database interruption.
///
/// The callback receives the exact completed parent custody plus every physical
/// candidate and lease record needed to recover the later publication sources.
pub(in crate::resident) fn with_completed_cold_consult_child_database_retry<R>(
    label: &'static str,
    run: impl FnOnce(
        &crate::resident::consultation::tests::IssuedTestChild,
        &crate::resident::IssuedLocalOperation,
        &pale_ports::AuthorityStoreCoordinate,
        &super::restart::ColdRestartSessions,
        &crate::resident_fixture::RecoveryResolver,
        Vec<OperationCustodyCandidate>,
        Vec<ControlRecord>,
        crate::resident::CompletedConsultationCollection,
    ) -> R,
) -> R {
    with_cold_terminal_commit(
        label,
        TerminalCommitCase::DatabaseRetry,
        TerminalParentCase::Collecting,
        run,
    )
}

/// Lend one exact terminal Consult-child handoff before physical commit inputs.
pub(in crate::resident::process_host) fn with_cold_terminal_consult_child_handoff<R>(
    label: &'static str,
    run: impl FnOnce(
        Box<
            crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryHandoff,
        >,
    ) -> R,
) -> R {
    with_cold_consult_child_sweep_staged(
        label,
        |child, parent, store, current| {
            let _ = (child, parent, store);
            ColdConsultCollectionSelection::exact(current)
        },
        successful_terminal_candidate,
        |_, _, _, recover| {
            assert!(matches!(
                recover().expect("root source belongs to the root sweep"),
                ResidentProcessConsultChildRecoveryTurn::SkippedOtherOwner(
                    ResidentProcessConsultChildOtherOwner::Root
                )
            ));
            let ResidentProcessConsultChildRecoveryTurn::Recovered(handoff) =
                recover().expect("terminal Consult child recovers")
            else {
                panic!("the second retained start is the terminal Consult child")
            };
            assert!(matches!(
                handoff.state(),
                ResidentProcessConsultChildRecoveryState::Terminal(_)
            ));
            handoff
        },
        |handoff, _, _, _, _, _, _, _, _, _, _, _, _, _| run(handoff),
    )
}

fn with_cold_terminal_commit<R>(
    label: &'static str,
    case: TerminalCommitCase,
    parent_case: TerminalParentCase,
    run: impl FnOnce(
        &crate::resident::consultation::tests::IssuedTestChild,
        &crate::resident::IssuedLocalOperation,
        &pale_ports::AuthorityStoreCoordinate,
        &super::restart::ColdRestartSessions,
        &crate::resident_fixture::RecoveryResolver,
        Vec<OperationCustodyCandidate>,
        Vec<ControlRecord>,
        crate::resident::CompletedConsultationCollection,
    ) -> R,
) -> R {
    with_cold_consult_child_sweep_staged(
        label,
        move |child, parent, store, current| {
            let current = match parent_case {
                TerminalParentCase::Collecting => current,
                TerminalParentCase::Stopped => {
                    consultation_supervision_fixture_for_self_contained_operation_in_store(
                        parent.handle().0,
                        parent.handle().1,
                        label,
                        child_coordinate(child),
                        true,
                        store.clone(),
                    )
                }
            };
            ColdConsultCollectionSelection::exact(current)
        },
        successful_terminal_candidate,
        |_, _, _, recover| {
            assert!(matches!(
                recover().expect("root source belongs to the root sweep"),
                ResidentProcessConsultChildRecoveryTurn::SkippedOtherOwner(
                    ResidentProcessConsultChildOtherOwner::Root
                )
            ));
            let ResidentProcessConsultChildRecoveryTurn::Recovered(handoff) =
                recover().expect("terminal Consult child recovers")
            else {
                panic!("the second retained start is the terminal Consult child")
            };
            assert!(matches!(
                handoff.state(),
                ResidentProcessConsultChildRecoveryState::Terminal(_)
            ));
            handoff
        },
        |handoff: Box<crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryHandoff>,
         child,
         parent,
         _,
         host: &mut crate::resident::process_host::ResidentProcessHost<
            '_,
            UnprovedHeldWriter,
         >,
         selector,
         selected_store,
         sessions: &super::restart::ColdRestartSessions,
         historical: &crate::resident_fixture::RecoveryResolver,
         _,
         _,
         mut publication_candidates: Vec<OperationCustodyCandidate>,
         publication_records: Vec<ControlRecord>,
         fresh_recover| {
            let deployment_index = host
                .deployment_authority_index(selector)
                .expect("selected Consult-child deployment");
            let cursor_before_terminal_custody =
                host.consult_child_recovery_attempts[deployment_index].clone();
            let frozen_stop = handoff
                .current()
                .collection()
                .stopped()
                .map(|stop| stop.summary().clone());
            let parent_checkpoint = handoff
                .parent()
                .observation()
                .selected_checkpoint_record();
            let parent_checkpoint = (parent_checkpoint.0.clone(), parent_checkpoint.1.clone());
            let terminal_transaction = match handoff.state() {
                ResidentProcessConsultChildRecoveryState::Terminal(child) => {
                    child
                        .control
                        .observation()
                        .selected_checkpoint_record()
                        .1
                        .committed_transaction_sequence()
                        .get()
                        + 1
                }
                ResidentProcessConsultChildRecoveryState::Pending(_) => {
                    panic!("the terminal commit test requires terminal child custody")
                }
            };
            let next_sequence = parent_checkpoint.1.domain_sequence().get() + 1;
            let mut authority_source =
                crate::local_operation_start_fixture::checkpoint_preimage_at_store(
                    next_sequence,
                    Some((
                        parent_checkpoint.0.clone(),
                        parent_checkpoint
                            .1
                            .canonical_control_checkpoint_sha256()
                            .clone(),
                    )),
                    selected_store,
                );
            authority_source.checkpoint_domain_id =
                parent_checkpoint.1.checkpoint_domain_id().clone();
            authority_source.checkpoint_profile_ref =
                parent_checkpoint.1.checkpoint_profile_ref().clone();
            authority_source.checkpoint_profile_sha256 =
                parent_checkpoint.1.checkpoint_profile_sha256().clone();
            authority_source.domain_epoch = *parent_checkpoint.1.domain_epoch();
            let authority = checkpoint_authority(&authority_source);
            let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let signing = child.request.to_preimage().signing_authority_binding;
            let physical = match case {
                TerminalCommitCase::Complete | TerminalCommitCase::AnchorRetry => {
                    RecordingStore::committing(OperationCustodyCommitOutcome::Committed)
                        .at_store(selected_store)
                }
                TerminalCommitCase::DatabaseRetry => RecordingStore::failing_commit(
                    ControlStoreError::new(ControlStoreErrorKind::Storage),
                    [
                        Ok(OperationCustodyReadBackOutcome::Predecessor),
                        Err(ControlStoreError::new(ControlStoreErrorKind::Storage)),
                        Err(ControlStoreError::new(ControlStoreErrorKind::Storage)),
                        Ok(OperationCustodyReadBackOutcome::ExactSuccessor),
                    ],
                )
                .at_store(selected_store),
            };
            let mut custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: pale_core::records::ControlTransactionSequenceV1::new(
                    terminal_transaction,
                )
                .expect("terminal join transaction"),
                physical,
            };
            let mut anchor = match case {
                TerminalCommitCase::Complete | TerminalCommitCase::DatabaseRetry => {
                    local_start::successor_anchor()
                }
                TerminalCommitCase::AnchorRetry => RecordingAnchor::unavailable_after_preflight(),
            };
            if matches!(case, TerminalCommitCase::DatabaseRetry) {
                anchor.allow_exact_successor_observation();
            }
            let expected_result = ImmutableRecordReferenceV1 {
                record_ref: record_ref("result/cold-consult-child-success"),
                record_sha256: ArtifactDigest::hash_bytes(b"cold consult child result"),
            };
            let resolve_store = |_: &OperationObservationV1| Ok(selected_store.clone());
            let committed = host
                .commit_recovered_consult_child_terminal(
                    ResidentDeploymentOperationAccess {
                        store: selected_store,
                        custody: &mut custody,
                        sessions,
                        anchor: &mut anchor,
                    },
                    handoff,
                    ResidentProcessConsultChildTerminalCommitInput {
                        association: ResidentOperationControlAssociation {
                            current_request_authority: &signing,
                            historical_handle_authority: &signing,
                            recovery_store: selected_store,
                            checkpoint_authority: &authority,
                        },
                        recorded_at: UnixTimeV1::new(parent_checkpoint.1.issued_at().get() + 1)
                            .expect("terminal join time"),
                        observation_ref: record_ref(
                            "observation/cold-consult-parent-terminal-join",
                        ),
                        observed_at: UnixTimeV1::new(parent_checkpoint.1.issued_at().get() + 1)
                            .expect("terminal parent observation time"),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: &signing,
                        },
                        limits: ConsultationCollectionLimits::new(1, 65_536, 65_536)
                            .expect("terminal collection limits"),
                        checkpoint: ConsultationSuccessorCommitInput {
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                1, 65_536, 65_536, 8_192, 200_000,
                            )
                            .expect("terminal checkpoint limits"),
                            identity: OperationCheckpointInput {
                                reference: record_ref(
                                    "checkpoint/cold-consult-parent-terminal-join",
                                ),
                                id: ControlCheckpointIdV1::parse(
                                    "cold-consult-parent-terminal-join",
                                )
                                .expect("terminal checkpoint identity"),
                            },
                            issued_at: UnixTimeV1::new(parent_checkpoint.1.issued_at().get() + 1)
                                .expect("terminal checkpoint time"),
                            schema: &checkpoint_schema,
                            authority: &authority,
                        },
                    },
                    &resolve_store,
                    |_, evidence| {
                        assert_eq!(evidence, &expected_result);
                        Ok(())
                    },
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("terminal cold child joins its exact parent"));
            let completed = match (case, committed) {
                (
                    TerminalCommitCase::Complete,
                    ResidentProcessConsultChildTerminalCommitTurn::Completed(completed),
                ) => {
                    assert_ne!(
                        host.consult_child_recovery_attempts[deployment_index],
                        cursor_before_terminal_custody,
                    );
                    *completed
                }
                (
                    TerminalCommitCase::DatabaseRetry,
                    ResidentProcessConsultChildTerminalCommitTurn::CustodyPending(pending),
                ) => {
                    assert_eq!(
                        host.consult_child_recovery_attempts[deployment_index],
                        cursor_before_terminal_custody,
                    );
                    assert_eq!(custody.physical.mutation_counts(), (1, 1));
                    let pending = match host.continue_recovered_consult_child_terminal_custody(
                        ResidentDeploymentOperationAccess {
                            store: selected_store,
                            custody: &mut custody,
                            sessions,
                            anchor: &mut anchor,
                        },
                        pending,
                    ) {
                        Err(pending) => pending,
                        Ok(_) => panic!("one unavailable read-back keeps terminal custody pending"),
                    };
                    assert_eq!(
                        host.consult_child_recovery_attempts[deployment_index],
                        cursor_before_terminal_custody,
                    );
                    assert_eq!(custody.physical.mutation_counts(), (1, 2));
                    let pending = match host.continue_recovered_consult_child_terminal_custody(
                        ResidentDeploymentOperationAccess {
                            store: selected_store,
                            custody: &mut custody,
                            sessions,
                            anchor: &mut anchor,
                        },
                        pending,
                    ) {
                        Err(pending) => pending,
                        Ok(_) => panic!("a second unavailable read-back keeps custody pending"),
                    };
                    assert_eq!(
                        host.consult_child_recovery_attempts[deployment_index],
                        cursor_before_terminal_custody,
                    );
                    assert_eq!(custody.physical.mutation_counts(), (1, 3));
                    let completed = host
                        .continue_recovered_consult_child_terminal_custody(
                            ResidentDeploymentOperationAccess {
                                store: selected_store,
                                custody: &mut custody,
                                sessions,
                                anchor: &mut anchor,
                            },
                            pending,
                        )
                        .unwrap_or_else(|_| {
                            panic!("exact database read-back completes the same join")
                        });
                    assert_eq!(custody.physical.mutation_counts(), (1, 4));
                    assert_ne!(
                        host.consult_child_recovery_attempts[deployment_index],
                        cursor_before_terminal_custody,
                    );
                    completed
                }
                (
                    TerminalCommitCase::AnchorRetry,
                    ResidentProcessConsultChildTerminalCommitTurn::CustodyPending(pending),
                ) => {
                    assert_eq!(
                        host.consult_child_recovery_attempts[deployment_index],
                        cursor_before_terminal_custody,
                    );
                    assert_eq!(custody.physical.mutation_counts(), (1, 1));
                    anchor.allow_next_advance();
                    let completed = host.continue_recovered_consult_child_terminal_custody(
                        ResidentDeploymentOperationAccess {
                            store: selected_store,
                            custody: &mut custody,
                            sessions,
                            anchor: &mut anchor,
                        },
                        pending,
                    )
                    .unwrap_or_else(|_| panic!("the same retained anchor transition completes"));
                    assert_ne!(
                        host.consult_child_recovery_attempts[deployment_index],
                        cursor_before_terminal_custody,
                    );
                    completed
                }
                _ => panic!("terminal commit produced the wrong physical disposition"),
            };
            let recovered = recover_completed_collection(&completed);
            assert_eq!(
                recovered.collection().stopped().map(|stop| stop.summary()),
                frozen_stop.as_ref(),
                "late evidence cannot rewrite the frozen stop summary",
            );
            assert_eq!(
                recovered.collection().late_outcomes().len(),
                usize::from(frozen_stop.is_some()),
                "only a stopped parent records the terminal as late evidence",
            );
            let repeated = fresh_recover(
                completed.custody().candidate().clone(),
                completed.checkpoint().1.clone(),
                recovered.snapshot().clone(),
            )
            .unwrap_or_else(|reason| match reason {
                ResidentProcessConsultChildRecoveryError::Restart(reason) => {
                    panic!("fresh-host retained terminal restart: {reason:?}")
                }
                other => panic!("fresh-host retained terminal classification: {other:?}"),
            });
            let ResidentProcessConsultChildRecoveryTurn::AlreadyRetained(current) = repeated else {
                panic!("a fresh host must classify the exact retained terminal once")
            };
            assert_eq!(current.collection(), recovered.collection());
            publication_candidates.push(completed.custody().candidate().clone());
            run(
                child,
                parent,
                selected_store,
                sessions,
                historical,
                publication_candidates,
                publication_records,
                completed,
            )
        },
    )
}
