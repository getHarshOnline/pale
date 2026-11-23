//! Physical cold-recovery proof for one durable Consult child.

use std::cell::RefCell;
use std::num::NonZeroUsize;
use std::rc::Rc;

use super::consult_child_fixture::{
    with_process_consult_invoke_child_generations, ConsultChildFixtureKind,
};
use super::restart::{cold_restart_sessions, host_with_generation, restart_input};
use super::start::CountingAuthority;
use super::*;
use crate::local_operation_start_fixture::{
    checkpoint_authority, record_ref, schema_digest, signature,
};
use crate::resident::consultation::service::{
    commit_consultation_collection, ConsultationCollectionCommitInput,
};
use crate::resident::consultation::tests::{
    consultation_supervision_fixture_for_self_contained_operation_in_store,
    prepare_collection_accumulator, recover_completed_collection, CollectionCommitPort,
};
use crate::resident::consultation::{
    ConsultationCollectionLimits, ConsultationCollectionObservationInput,
    RecoveredConsultationCollection,
};
use crate::resident::operation_dispatch::OperationCheckpointInput;
use crate::resident::operation_observation_custody::tests::local_start;
use crate::resident::operation_observation_custody::tests::native_execution::with_service_custody_faults_and_collection;
use crate::resident::process_host::consult_child_recovery_sweep::{
    ResidentProcessConsultChildOtherOwner, ResidentProcessConsultChildPending,
    ResidentProcessConsultChildRecoveryError, ResidentProcessConsultChildRecoveryState,
    ResidentProcessConsultChildRecoveryTurn,
};
use crate::resident::{
    LocalOperationHandleRecoveryInput, LocalOperationRecoveryContext,
    OperationCheckpointAncestryLimits, OperationControlRecoveryAdmission,
    OperationControlRecoveryInput, OperationObservationRecoveryLimits,
    OperationObservationSelection,
};
use pale_core::canon;
use pale_core::records::{
    ArtifactDigest, CanonicalSignatureV1, CommonRequestEnvelopeV1, CommonRequestEnvelopeWireV1,
    ControlCheckpointIdV1, ControlStateCheckpointDigest, ControlStateCheckpointPreimageV1,
    ControlStateCheckpointV1, ControlStateCheckpointWireV1, ControlTransactionSequenceV1,
    ImmutableRecordReferenceV1, InvalidResidentRecordV1, OperationHandleDigest,
    OperationHandlePreimageV1, OperationHandleV1, OperationObservationAdmissionContextV1,
    OperationObservationDigest, OperationObservationPreimageV1, OperationObservationStateV1,
    OperationObservationV1, OperationObservationWireV1, ResidentRecordRefV1,
    SignedResidentAdmissionContextV1, UnixTimeV1, CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AccumulatorMembershipLimits, AccumulatorPreparationLimits, ControlRecord,
    OperationCollectionSnapshot, OperationCustodyCandidate, OperationCustodyRecordKind,
};

/// Logical collection committed for a cold-child test and the physical witness exposed later.
pub(super) struct ColdConsultCollectionSelection {
    collection: RecoveredConsultationCollection,
    snapshot: ColdConsultSnapshotSelection,
}

enum ColdConsultSnapshotSelection {
    Current,
    Absent,
    Replacement(Box<OperationCollectionSnapshot>),
}

impl ColdConsultCollectionSelection {
    /// Commit this logical collection and expose its matching retained snapshot.
    pub(super) const fn exact(collection: RecoveredConsultationCollection) -> Self {
        Self {
            collection,
            snapshot: ColdConsultSnapshotSelection::Current,
        }
    }

    /// Commit this logical collection but model a missing physical point read.
    pub(super) const fn absent(collection: RecoveredConsultationCollection) -> Self {
        Self {
            collection,
            snapshot: ColdConsultSnapshotSelection::Absent,
        }
    }

    /// Commit this logical collection but return an unlike physical point read.
    pub(super) fn replacement(
        collection: RecoveredConsultationCollection,
        snapshot: OperationCollectionSnapshot,
    ) -> Self {
        Self {
            collection,
            snapshot: ColdConsultSnapshotSelection::Replacement(Box::new(snapshot)),
        }
    }
}

fn reborrow_admission<'short>(
    admission: &'short mut OperationControlRecoveryAdmission<'_>,
) -> OperationControlRecoveryAdmission<'short> {
    OperationControlRecoveryAdmission {
        admit_request: &mut *admission.admit_request,
        admit_handle_authority: &mut *admission.admit_handle_authority,
        verify_handle_signature: &mut *admission.verify_handle_signature,
        admit_checkpoint: &mut *admission.admit_checkpoint,
        resolve_observation_signer: &mut *admission.resolve_observation_signer,
        admit_observation_state: &mut *admission.admit_observation_state,
        admit_rotation: &mut *admission.admit_rotation,
        verify_observation_signature: &mut *admission.verify_observation_signature,
        verify_checkpoint_signature: &mut *admission.verify_checkpoint_signature,
    }
}

fn request_admission<'a>(
    child_ref: &'a ResidentRecordRefV1,
    child: &'a CommonRequestEnvelopeV1,
    parent_ref: &'a ResidentRecordRefV1,
    parent: &'a CommonRequestEnvelopeV1,
) -> impl FnMut(
    &ResidentRecordRefV1,
    CommonRequestEnvelopeWireV1,
) -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1>
       + 'a {
    move |reference, wire| {
        let expected = if reference == child_ref {
            child
        } else if reference == parent_ref {
            parent
        } else {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "common request envelope",
                invariant: "known retained Consult-child request coordinate",
            });
        };
        let preimage = expected.to_preimage();
        CommonRequestEnvelopeV1::admit_contextual_wire(
            wire,
            &super::restart::request_admission_context(&preimage),
            |_, _, actual| {
                if actual != expected.caller_signature() {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "common request envelope",
                        invariant: "retained caller signature",
                    });
                }
                Ok(())
            },
        )
    }
}

#[test]
fn cold_sweep_recovers_one_real_persisted_consult_child_without_dispatch() {
    std::thread::Builder::new()
        .name("cold-consult-child-recovery".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(run_cold_sweep_recovers_one_real_persisted_consult_child_without_dispatch)
        .expect("bounded cold Consult-child test thread")
        .join()
        .expect("cold Consult-child recovery test completes");
}

fn run_cold_sweep_recovers_one_real_persisted_consult_child_without_dispatch() {
    with_cold_consult_child_sweep(
        "cold-consult-child-recovery",
        |_, _, _, current| ColdConsultCollectionSelection::exact(current),
        |_, parent, current, recover| {
            let first =
                recover().unwrap_or_else(|reason| panic!("root source classification: {reason:?}"));
            assert!(matches!(
                first,
                ResidentProcessConsultChildRecoveryTurn::SkippedOtherOwner(
                    ResidentProcessConsultChildOtherOwner::Root
                )
            ));

            let second = recover()
                .unwrap_or_else(|reason| panic!("Consult-child classification: {reason:?}"));
            let ResidentProcessConsultChildRecoveryTurn::Recovered(handoff) = second else {
                panic!("the second retained source is the exact Consult child")
            };
            assert_eq!(
                handoff.parent().operation().operation().handle(),
                parent.handle()
            );
            assert_eq!(handoff.current().collection(), current.collection());
            assert!(matches!(
                handoff.state(),
                ResidentProcessConsultChildRecoveryState::Pending(pending)
                    if matches!(
                        pending.as_ref(),
                        ResidentProcessConsultChildPending::OperationResolution(_)
                    )
            ));
            drop(handoff);

            assert!(matches!(
                recover().unwrap_or_else(|reason| panic!("fixed Consult-child source: {reason:?}")),
                ResidentProcessConsultChildRecoveryTurn::Recovered(_)
            ));
        },
    );
}

/// Run one physical root-Consult and child through the cold child-sweep owner.
///
/// The fixture keeps all request, handle, session, checkpoint and collection
/// admission inside one place. Callers choose only the stored collection
/// snapshot and inspect the resulting recovery turns. The fixture proves that
/// this classification path performs no custody write, anchor advance,
/// provider reservation or task acceptance.
pub(super) fn with_cold_consult_child_sweep<R>(
    label: &'static str,
    select_collection: impl FnOnce(
        &crate::resident::consultation::tests::IssuedTestChild,
        &crate::resident::IssuedLocalOperation,
        &pale_ports::AuthorityStoreCoordinate,
        RecoveredConsultationCollection,
    ) -> ColdConsultCollectionSelection,
    run: impl FnOnce(
        &crate::resident::consultation::tests::IssuedTestChild,
        &crate::resident::IssuedLocalOperation,
        &RecoveredConsultationCollection,
        &mut dyn FnMut() -> Result<
            ResidentProcessConsultChildRecoveryTurn,
            ResidentProcessConsultChildRecoveryError,
        >,
    ) -> R,
) -> R {
    with_cold_consult_child_sweep_staged(
        label,
        select_collection,
        |_, _, _| None,
        run,
        |result, _, _, _, _, _, _, _, _, _, _, _, _, _| result,
    )
}

/// Run cold recovery with an optional later child candidate, then retain access
/// to the same host and physical ports for a terminal-owner test.
#[allow(clippy::too_many_arguments)]
pub(super) fn with_cold_consult_child_sweep_staged<Recovered, R>(
    label: &'static str,
    select_collection: impl FnOnce(
        &crate::resident::consultation::tests::IssuedTestChild,
        &crate::resident::IssuedLocalOperation,
        &pale_ports::AuthorityStoreCoordinate,
        RecoveredConsultationCollection,
    ) -> ColdConsultCollectionSelection,
    extend_history: impl FnOnce(
        &crate::resident::consultation::tests::IssuedTestChild,
        &OperationCustodyCandidate,
        &ControlStateCheckpointV1,
    ) -> Option<(OperationCustodyCandidate, ControlStateCheckpointV1)>,
    recover_run: impl FnOnce(
        &crate::resident::consultation::tests::IssuedTestChild,
        &crate::resident::IssuedLocalOperation,
        &RecoveredConsultationCollection,
        &mut dyn FnMut() -> Result<
            ResidentProcessConsultChildRecoveryTurn,
            ResidentProcessConsultChildRecoveryError,
        >,
    ) -> Recovered,
    finish: impl FnOnce(
        Recovered,
        &crate::resident::consultation::tests::IssuedTestChild,
        &crate::resident::IssuedLocalOperation,
        &RecoveredConsultationCollection,
        &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
        &ResidentDeploymentAuthoritySelector,
        &pale_ports::AuthorityStoreCoordinate,
        &super::restart::ColdRestartSessions,
        &crate::resident_fixture::RecoveryResolver,
        &mut dyn pale_ports::OperationCustodyPort,
        &mut dyn pale_ports::AntiRollbackAnchorPort,
        Vec<OperationCustodyCandidate>,
        Vec<ControlRecord>,
        Box<
            dyn FnOnce(
                    OperationCustodyCandidate,
                    ControlStateCheckpointV1,
                    OperationCollectionSnapshot,
                ) -> Result<
                    ResidentProcessConsultChildRecoveryTurn,
                    ResidentProcessConsultChildRecoveryError,
                > + '_,
        >,
    ) -> R,
) -> R {
    with_process_consult_invoke_child_generations(
        label,
        ConsultChildFixtureKind::SelfContained,
        NonZeroUsize::new(2).expect("current and fresh cold hosts"),
        |child,
         parent,
         parent_candidate,
         _parent_checkpoint,
         lease_custody,
         _generation,
         parents,
         mut host_generations| {
            let fresh_host_generation = host_generations.pop().expect("fresh cold-host generation");
            let host_generation = host_generations
                .pop()
                .expect("initial cold-host generation");
            assert!(host_generations.is_empty());
            let selected_store = child.initial_candidate.store().clone();
            let selector = selector_from_signing_authority(parents.signing);
            let child_coordinate = ImmutableRecordReferenceV1 {
                record_ref: child.operation.handle().0.clone(),
                record_sha256: ArtifactDigest::parse(
                    child
                        .operation
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str(),
                )
                .expect("child handle digest"),
            };
            let modeled = consultation_supervision_fixture_for_self_contained_operation_in_store(
                parent.handle().0,
                parent.handle().1,
                label,
                child_coordinate,
                false,
                selected_store.clone(),
            );
            let selected = select_collection(child, parent, &selected_store, modeled);
            let collection_time = selected.collection.collection().stopped().map_or_else(
                || {
                    UnixTimeV1::new(child.checkpoint.issued_at().get() + 1)
                        .expect("collection observation time")
                },
                |_| selected.collection.collection().deadline(),
            );
            let parent_initial_record = parent_candidate
                .record(OperationCustodyRecordKind::OperationObservation)
                .expect("retained Consult parent initial observation");
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let parent_initial = OperationObservationV1::admit_contextual_wire(
                serde_json::from_value(
                    canon::parse_strict(parent_initial_record.canonical_record_jcs().as_slice())
                        .expect("retained Consult parent observation JSON"),
                )
                .expect("retained Consult parent observation wire"),
                &OperationObservationAdmissionContextV1 {
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: parents.signing,
                    },
                    handle_ref: parent.handle().0,
                    handle: parent.handle().1,
                    predecessor: None,
                },
                |_| Ok(()),
                |_, _, _, _| panic!("the initial parent observation cannot rotate authority"),
                |_, _, _, _| Ok(()),
            )
            .expect("retained Consult parent observation admits");
            let limits = ConsultationCollectionLimits::new(1, 65_536, 65_536)
                .expect("bounded one-child Consult collection");
            let next_transaction = ControlTransactionSequenceV1::new(
                child.checkpoint.committed_transaction_sequence().get() + 1,
            )
            .expect("successor collection transaction");
            let mut collection_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: next_transaction,
                physical: local_start::absent_start_store(&child.parent_request.1),
            };
            let prepared_collection = selected
                .collection
                .collection()
                .prepare_observation(
                    parent,
                    (&child.checkpoint_ref, &child.checkpoint),
                    (
                        parent_initial_record.coordinate().record_ref(),
                        &parent_initial,
                    ),
                    ConsultationCollectionObservationInput {
                        observation_ref: record_ref(&format!("observation/{label}-collecting")),
                        predecessor_collection: None,
                        observed_at: collection_time,
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: parents.signing,
                        },
                    },
                    limits,
                    &local_start::StartStore(parents.session),
                    &mut collection_custody,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                )
                .expect("one-child Consult collection observation prepares");
            let next_sequence = child.checkpoint.domain_sequence().get() + 1;
            let mut authority_source =
                crate::local_operation_start_fixture::checkpoint_preimage_at_store(
                    next_sequence,
                    Some((
                        child.checkpoint_ref.clone(),
                        child
                            .checkpoint
                            .canonical_control_checkpoint_sha256()
                            .clone(),
                    )),
                    &selected_store,
                );
            authority_source.checkpoint_domain_id = child.checkpoint.checkpoint_domain_id().clone();
            authority_source.checkpoint_profile_sha256 =
                crate::resident::SupportedOperationCheckpointProfile::content_digest()
                    .expect("supported checkpoint profile");
            let successor_authority = checkpoint_authority(&authority_source);
            let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
            let mut collection_anchor = local_start::successor_anchor();
            let completed_collection = commit_consultation_collection(
                prepared_collection,
                ConsultationCollectionCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 1024, 1024, 512, 65_536,
                    )
                    .expect("bounded collection accumulator"),
                    predecessor: (&child.checkpoint_ref, &child.checkpoint),
                    identity: OperationCheckpointInput {
                        reference: record_ref(&format!("checkpoint/{label}-collecting")),
                        id: ControlCheckpointIdV1::parse(format!("{label}-collecting-checkpoint"))
                            .expect("collection checkpoint identity"),
                    },
                    issued_at: collection_time,
                    schema: &checkpoint_schema,
                    authority: &successor_authority,
                },
                &local_start::StartStore(parents.session),
                &mut collection_custody,
                &mut collection_anchor,
                |_, _, _| Ok(signature()),
            )
            .unwrap_or_else(|_| panic!("one-child Consult collection reaches physical custody"));
            let collection_candidate = completed_collection.custody().candidate().clone();
            let collection_checkpoint = completed_collection.checkpoint().1.clone();
            let current = recover_completed_collection(&completed_collection);
            let collection_snapshot = match selected.snapshot {
                ColdConsultSnapshotSelection::Current => Some(current.snapshot().clone()),
                ColdConsultSnapshotSelection::Absent => None,
                ColdConsultSnapshotSelection::Replacement(snapshot) => Some(*snapshot),
            };
            let extension = extend_history(child, &collection_candidate, &collection_checkpoint);
            let (service_checkpoint, service_candidate, preceding_candidates) =
                match extension.as_ref() {
                    Some((candidate, checkpoint)) => (
                        checkpoint,
                        candidate,
                        vec![
                            parent_candidate,
                            child.initial_candidate.clone(),
                            collection_candidate,
                        ],
                    ),
                    None => (
                        &collection_checkpoint,
                        &collection_candidate,
                        vec![parent_candidate, child.initial_candidate.clone()],
                    ),
                };
            let mut fresh_preceding_candidates = preceding_candidates.clone();
            fresh_preceding_candidates.push(service_candidate.clone());
            let retained_records = vec![lease_custody.parent, lease_custody.child];
            let fresh_retained_records = retained_records.clone();
            let publication_records = retained_records.clone();
            let publication_candidates = fresh_preceding_candidates.clone();
            let checkpoint_authority = super::a2a_cancel_task::checkpoint_signer(&child.checkpoint);
            let (sessions, historical) = cold_restart_sessions(parents, &selected_store);
            let (fresh_sessions, _) = cold_restart_sessions(parents, &selected_store);
            let application_store = MemoryAuthorityStore::default();
            let mut host = host_with_generation(
                &application_store,
                &selector,
                &selected_store,
                host_generation,
            );

            with_service_custody_faults_and_collection(
                service_checkpoint,
                Some(service_candidate),
                preceding_candidates,
                None,
                retained_records,
                0,
                0,
                0,
                0,
                0,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                Vec::new(),
                false,
                None,
                Rc::new(RefCell::new(None)),
                collection_snapshot,
                |custody, anchor| {
                    let profile = crate::local_operation_start_fixture::profile();
                    let observation_limits =
                        OperationObservationRecoveryLimits::new(16, 1 << 20, 1 << 20)
                            .expect("bounded observation recovery");
                    let checkpoint_limits = OperationCheckpointAncestryLimits::new(16, 1 << 20, 16)
                        .expect("bounded checkpoint recovery");
                    let membership_limits =
                        AccumulatorMembershipLimits::new(64, 100_000, 1_000_000)
                            .expect("bounded membership recovery");
                    let recovery_context = LocalOperationRecoveryContext {
                        store: &selected_store,
                        profile: &profile,
                        handle_signing_authority: parents.signing,
                        checkpoint_authority: &checkpoint_authority,
                        historical: &historical,
                        maximum_record_bytes: 1 << 20,
                        maximum_operation_lease_bytes: 1 << 20,
                        maximum_session_lease_bytes: 1 << 20,
                        membership_limits,
                        observation_limits,
                        checkpoint_limits,
                    };
                    let child_handle = crate::resident::record_projection::coordinate(
                        child.operation.handle().0,
                        child
                            .operation
                            .handle()
                            .1
                            .canonical_operation_handle_sha256()
                            .as_str(),
                    )
                    .expect("child recovery coordinate");
                    let restart = || {
                        restart_input(
                            OperationControlRecoveryInput {
                                operation: LocalOperationHandleRecoveryInput {
                                    context: recovery_context,
                                    expected_handle: &child_handle,
                                    maximum_parent_depth: 1,
                                },
                                observation: OperationObservationSelection::SelectedHead,
                                observation_limits,
                                checkpoint_limits,
                                membership_limits,
                            },
                            1 << 20,
                        )
                    };

                    let child_ref = child.operation.request_coordinate().record_ref();
                    let parent_ref = child.parent_request.0.record_ref();
                    let child_handle = child.operation.handle().1;
                    let parent_handle = parent.handle().1;
                    let mut admit_request = request_admission(
                        child_ref,
                        &child.request,
                        parent_ref,
                        &child.parent_request.1,
                    );
                    let mut admit_handle_authority =
                        |preimage: &OperationHandlePreimageV1,
                         recovered_parent: Option<&OperationHandleV1>| {
                            if preimage.operation_handle_id == *child_handle.operation_handle_id() {
                                if recovered_parent != Some(parent_handle) {
                                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                                        record: "operation handle",
                                        invariant: "exact recovered Consult parent",
                                    });
                                }
                            } else if preimage.operation_handle_id
                                != *parent_handle.operation_handle_id()
                                || recovered_parent.is_some()
                            {
                                return Err(InvalidResidentRecordV1::ContextualAdmission {
                                    record: "operation handle",
                                    invariant: "one admitted root Consult parent",
                                });
                            }
                            Ok(())
                        };
                    let mut verify_handle_signature =
                        |_: &[u8],
                         _: &OperationHandlePreimageV1,
                         _: &OperationHandleDigest,
                         actual: &CanonicalSignatureV1| {
                            if actual != &signature() {
                                return Err(InvalidResidentRecordV1::ContextualAdmission {
                                    record: "operation handle",
                                    invariant: "retained handle signature",
                                });
                            }
                            Ok(())
                        };
                    let mut admit_checkpoint =
                        |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                            ControlStateCheckpointV1::admit_signed_wire(
                                wire,
                                child.checkpoint.schema_sha256(),
                                &checkpoint_authority,
                                |_, _, _, actual| {
                                    if actual != &signature() {
                                        return Err(InvalidResidentRecordV1::ContextualAdmission {
                                            record: "control checkpoint",
                                            invariant: "retained checkpoint signature",
                                        });
                                    }
                                    Ok(())
                                },
                            )
                        };
                    let mut resolve_observation_signer =
                        |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
                            Ok(parents.signing.clone())
                        };
                    let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
                    let mut admit_rotation =
                        |_: &ResidentRecordRefV1,
                         _: &ArtifactDigest,
                         _: &ResidentRecordRefV1,
                         _: &ArtifactDigest| { Ok(()) };
                    let mut verify_observation_signature =
                        |_: &[u8],
                         _: &OperationObservationPreimageV1,
                         _: &OperationObservationDigest,
                         actual: &CanonicalSignatureV1| {
                            if actual != &signature() {
                                return Err(InvalidResidentRecordV1::ContextualAdmission {
                                    record: "operation observation",
                                    invariant: "retained observation signature",
                                });
                            }
                            Ok(())
                        };
                    let mut verify_checkpoint_signature =
                        |_: &[u8],
                         _: &ControlStateCheckpointPreimageV1,
                         _: &ControlStateCheckpointDigest,
                         actual: &CanonicalSignatureV1| {
                            if actual != &signature() {
                                return Err(InvalidResidentRecordV1::ContextualAdmission {
                                    record: "control checkpoint",
                                    invariant: "retained checkpoint signature",
                                });
                            }
                            Ok(())
                        };
                    let mut admission = OperationControlRecoveryAdmission {
                        admit_request: &mut admit_request,
                        admit_handle_authority: &mut admit_handle_authority,
                        verify_handle_signature: &mut verify_handle_signature,
                        admit_checkpoint: &mut admit_checkpoint,
                        resolve_observation_signer: &mut resolve_observation_signer,
                        admit_observation_state: &mut admit_observation_state,
                        admit_rotation: &mut admit_rotation,
                        verify_observation_signature: &mut verify_observation_signature,
                        verify_checkpoint_signature: &mut verify_checkpoint_signature,
                    };
                    let resolve_store =
                        |_: &pale_core::records::OperationObservationV1| Ok(selected_store.clone());
                    let mut authority = CountingAuthority::default();

                    let recovered = {
                        let mut recover = || {
                            host.recover_one_consult_child_start(
                                &selector,
                                ResidentDeploymentOperationAccess {
                                    store: &selected_store,
                                    custody: &mut *custody,
                                    sessions: &sessions,
                                    anchor: &mut *anchor,
                                },
                                restart(),
                                reborrow_admission(&mut admission),
                                1 << 20,
                                &mut authority,
                                &resolve_store,
                                |_| Ok(()),
                            )
                        };
                        recover_run(child, parent, &current, &mut recover)
                    };
                    assert_eq!(host.provider_routes.reserved, 0);
                    assert_eq!(host.operations.outstanding_task_reservations(), 0);
                    let fresh_selector = selector.clone();
                    let fresh_store = selected_store.clone();
                    let fresh_recover = Box::new(
                        move |successor_candidate: OperationCustodyCandidate,
                              successor_checkpoint: ControlStateCheckpointV1,
                              successor_snapshot: OperationCollectionSnapshot| {
                            with_service_custody_faults_and_collection(
                                &successor_checkpoint,
                                Some(&successor_candidate),
                                fresh_preceding_candidates,
                                None,
                                fresh_retained_records,
                                0,
                                0,
                                0,
                                0,
                                0,
                                &[],
                                &[],
                                &[],
                                &[],
                                &[],
                                &[],
                                Vec::new(),
                                false,
                                None,
                                Rc::new(RefCell::new(None)),
                                Some(successor_snapshot),
                                |fresh_custody, fresh_anchor| {
                                    let fresh_application_store = MemoryAuthorityStore::default();
                                    let mut fresh_host = host_with_generation(
                                        &fresh_application_store,
                                        &fresh_selector,
                                        &fresh_store,
                                        fresh_host_generation,
                                    );
                                    let mut fresh_authority = CountingAuthority::default();
                                    let fresh_resolve_store =
                                        |_: &pale_core::records::OperationObservationV1| {
                                            Ok(fresh_store.clone())
                                        };
                                    let first = fresh_host.recover_one_consult_child_start(
                                        &fresh_selector,
                                        ResidentDeploymentOperationAccess {
                                            store: &fresh_store,
                                            custody: fresh_custody,
                                            sessions: &fresh_sessions,
                                            anchor: fresh_anchor,
                                        },
                                        restart(),
                                        reborrow_admission(&mut admission),
                                        1 << 20,
                                        &mut fresh_authority,
                                        &fresh_resolve_store,
                                        |_| Ok(()),
                                    )?;
                                    assert!(matches!(
                                        first,
                                        ResidentProcessConsultChildRecoveryTurn::SkippedOtherOwner(
                                            ResidentProcessConsultChildOtherOwner::Root
                                        )
                                    ));
                                    let result = fresh_host.recover_one_consult_child_start(
                                        &fresh_selector,
                                        ResidentDeploymentOperationAccess {
                                            store: &fresh_store,
                                            custody: fresh_custody,
                                            sessions: &fresh_sessions,
                                            anchor: fresh_anchor,
                                        },
                                        restart(),
                                        reborrow_admission(&mut admission),
                                        1 << 20,
                                        &mut fresh_authority,
                                        &fresh_resolve_store,
                                        |_| Ok(()),
                                    );
                                    assert_eq!(fresh_host.provider_routes.reserved, 0);
                                    assert_eq!(
                                        fresh_host.operations.outstanding_task_reservations(),
                                        0
                                    );
                                    result
                                },
                            )
                        },
                    );
                    finish(
                        recovered,
                        child,
                        parent,
                        &current,
                        &mut host,
                        &selector,
                        &selected_store,
                        &sessions,
                        &historical,
                        &mut *custody,
                        &mut *anchor,
                        publication_candidates,
                        publication_records,
                        fresh_recover,
                    )
                },
            )
        },
    )
}
