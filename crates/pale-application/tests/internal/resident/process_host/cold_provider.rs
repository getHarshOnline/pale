//! Cold-restart laws for provider-backed Consult children.

use super::super::{
    a2a_cancel_task,
    provider::{ProviderClock, ProviderStore},
};
use super::*;
use crate::resident::consultation::ResidentConsultationTaskLossCommitFailure;
use crate::resident::operation_dispatch::test_support::RetainedProviderRecoveryFixture;
use crate::resident::process_host::consult_provider::test_support::project_cold_loss_offered_for_test;

#[allow(clippy::too_many_arguments)]
fn classify_cold_provider_consult_child(
    fixture: ConsultChildFixture<'_>,
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
    selected_dispatch: Option<&ImmutableRecordReferenceV1>,
    maximum_parent_depth: usize,
) -> ResidentProcessOperationRestartDisposition {
    let ConsultChildFixture {
        child,
        parent,
        parents,
        ..
    } = fixture;
    let RecoveredConsultChild {
        parent_start,
        current,
        batch,
        candidate,
        observed_at,
        ..
    } = recovered_child;
    let selected_store = child.initial_candidate.store().clone();
    let selector = selector_from_signing_authority(parents.signing);
    let session = parents.session.lease();
    let sessions = RestartSessions(SessionLeaseBody::new(
        selected_store.clone(),
        session.session_admission_lease_digest().clone(),
        session.schema_id().clone(),
        session.schema_sha256().clone(),
        pale_ports::ControlRecordBytes::new(
            pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(session))
                .expect("session wire")
                .into_bytes(),
        )
        .expect("bounded session wire"),
    ));
    let mut historical = RecoveryResolver::default();
    historical.insert_generation(parents.current.generation.compiled_generation);
    historical.insert_active(parents.current.generation.active_head);
    historical.insert_fence(parents.current.generation.activation_fence);
    historical.insert_effective_profile(parents.current.generation.effective_profile);
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
    let observation_limits =
        OperationObservationRecoveryLimits::new(8, 1 << 20, 1 << 20).expect("observation bounds");
    let checkpoint_limits =
        OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).expect("checkpoint bounds");
    let membership_limits =
        AccumulatorMembershipLimits::new(8, 20_000, 200_000).expect("membership bounds");
    let checkpoint_profile = crate::local_operation_start_fixture::profile();
    let mut checkpoint_fields = checkpoint_preimage_at_store(
        child.checkpoint.domain_sequence().get() + 1,
        Some((
            child.checkpoint_ref.clone(),
            child
                .checkpoint
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
        &selected_store,
    );
    checkpoint_fields.checkpoint_domain_id = child.checkpoint.checkpoint_domain_id().clone();
    checkpoint_fields.checkpoint_profile_ref = child.checkpoint.checkpoint_profile_ref().clone();
    checkpoint_fields.checkpoint_profile_sha256 =
        child.checkpoint.checkpoint_profile_sha256().clone();
    checkpoint_fields.domain_epoch = *child.checkpoint.domain_epoch();
    let checkpoint_signing = checkpoint_authority(&checkpoint_fields);
    let recovery = || OperationControlRecoveryInput {
        operation: LocalOperationHandleRecoveryInput {
            context: LocalOperationRecoveryContext {
                store: &selected_store,
                profile: &checkpoint_profile,
                handle_signing_authority: parents.signing,
                checkpoint_authority: &checkpoint_signing,
                historical: &historical,
                maximum_record_bytes: 1 << 20,
                maximum_operation_lease_bytes: 1 << 20,
                maximum_session_lease_bytes: 1 << 20,
                membership_limits,
                observation_limits,
                checkpoint_limits,
            },
            expected_handle: &child_handle,
            maximum_parent_depth,
        },
        observation: OperationObservationSelection::SelectedHead,
        observation_limits,
        checkpoint_limits,
        membership_limits,
    };
    let mut admit_request =
        |reference: &pale_core::records::ResidentRecordRefV1,
         wire: pale_core::records::CommonRequestEnvelopeWireV1| {
            let expected = if reference == &child.request_ref {
                &child.request
            } else if reference == child.parent_request.0.record_ref() {
                &child.parent_request.1
            } else {
                let cancellation = a2a_cancel_task::cancellation_request(
                    &child.request,
                    child.operation.handle().1,
                );
                assert_eq!(
                    pale_core::canon::to_canonical_string(&wire)
                        .expect("cancellation request wire encoding"),
                    pale_core::canon::to_canonical_string(&cancellation)
                        .expect("expected cancellation request encoding")
                );
                return Ok(cancellation);
            };
            assert_eq!(
                pale_core::canon::to_canonical_string(&wire).expect("request wire encoding"),
                pale_core::canon::to_canonical_string(expected).expect("expected request encoding")
            );
            Ok(expected.clone())
        };
    let mut admit_handle_authority =
        |preimage: &pale_core::records::OperationHandlePreimageV1,
         recovered_parent: Option<&pale_core::records::OperationHandleV1>| {
            if &preimage.operation_handle_id == child.operation.handle().1.operation_handle_id() {
                assert_eq!(
                    recovered_parent
                        .expect("child recovery retains its Consult parent")
                        .canonical_operation_handle_sha256(),
                    parent.handle().1.canonical_operation_handle_sha256()
                );
            } else {
                assert_eq!(
                    &preimage.operation_handle_id,
                    parent.handle().1.operation_handle_id()
                );
                assert!(recovered_parent.is_none());
            }
            Ok(())
        };
    let mut verify_handle_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
    let mut admit_checkpoint = |_: &_, wire| {
        pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
            wire,
            &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
            &checkpoint_signing,
            |_, _, _, _| Ok(()),
        )
    };
    let mut resolve_observation_signer = |_: &_, _: &_| Ok(parents.signing.clone());
    let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
    let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
    let mut verify_observation_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
    let mut verify_checkpoint_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
    let driver = host
        .consultation_child_driver(&selector)
        .expect("cold provider Consult classification driver");
    driver
        .classify_child_operation(
            native_recovery_association(parents.signing, &native_recovery_input(recovery())),
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions: &sessions,
                anchor,
            },
            native_recovery_input(recovery()),
            &mut OperationControlRecoveryAdmission {
                admit_request: &mut admit_request,
                admit_handle_authority: &mut admit_handle_authority,
                verify_handle_signature: &mut verify_handle_signature,
                admit_checkpoint: &mut admit_checkpoint,
                resolve_observation_signer: &mut resolve_observation_signer,
                admit_observation_state: &mut admit_observation_state,
                admit_rotation: &mut admit_rotation,
                verify_observation_signature: &mut verify_observation_signature,
                verify_checkpoint_signature: &mut verify_checkpoint_signature,
            },
            parent_start,
            current,
            batch,
            candidate,
            selected_dispatch,
            observed_at,
        )
        .expect("cold provider Consult child classifies")
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ColdProviderConsultCase {
    CurrentHead,
    CancellationPredecessor,
    Collision,
    OfferedContinuation,
}

#[allow(clippy::too_many_arguments)]
fn run_provider_consult_cold_case(
    case: ColdProviderConsultCase,
    fixture: ConsultChildFixture<'_>,
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
) {
    let ConsultChildFixture {
        label,
        child,
        generation,
        parents,
        ..
    } = fixture;
    let selected_store = child.initial_candidate.store().clone();
    let initial_record = child
        .initial_candidate
        .record(OperationCustodyRecordKind::OperationObservation)
        .expect("child initial observation");
    with_provider_selection_for_operation(
        &child.operation,
        &child.request_ref,
        &child.request,
        generation,
        &parents,
        "attempt-1",
        record_ref("process-consult-provider/selection"),
        |provider_request, provider_admission, attempt, selection_record| {
            let dispatch_floor = parents.current.now.max(*child.checkpoint.issued_at()).get();
            let frontier = ProviderInvocationEvidenceFrontier::new(
                ProviderInvocationEvidenceSubject::from_request(provider_request),
                record_ref("provider-evidence"),
                artifact_digest('5'),
                UnixTimeV1::new(dispatch_floor + 4).expect("provider evidence time"),
                UnixTimeV1::new(dispatch_floor + 14).expect("provider evidence expiry"),
            )
            .expect("provider evidence frontier");
            let coordinate =
                ProviderInvocationObservationCoordinate::capture(provider_request, frontier)
                    .expect("provider observation coordinate");
            let provider = ResultPort::failure(safe_failure_at(
                &child.request,
                parents.signing,
                attempt,
                coordinate,
                UnixTimeV1::new(dispatch_floor + 6).expect("provider start time"),
                UnixTimeV1::new(dispatch_floor + 9).expect("provider completion time"),
            ));
            with_provider_dispatch_inputs(
                label,
                &selected_store,
                provider_request,
                &child.operation,
                &child.request,
                generation,
                provider_admission,
                &parents,
                (&child.checkpoint_ref, &child.checkpoint),
                initial_record,
                &selection_record,
                |dispatch, _invocation, lost_invocation, provider_clock, provider_store| {
                    run_provider_consult_test_path(|| {
                        let classified = classify_cold_provider_consult_child(
                            fixture,
                            recovered_child,
                            host,
                            custody,
                            anchor,
                            None,
                            1,
                        );
                        let ResidentProcessOperationRestartDisposition::OperationResolution(source) =
                            classified
                        else {
                            panic!("unopened provider child remains at Invoke resolution")
                        };
                        let mut retention_authority = ProvingAuthority;
                        let retained = retain_provider_operation_dispatch(
                            dispatch,
                            &mut retention_authority,
                            custody,
                            anchor,
                            |_, _, _, _| Ok(()),
                            |_, _, _| Ok(signature()),
                            |_, _, _| Ok(signature()),
                        )
                        .expect("provider dispatch retained without a provider call");
                        let dispatch_recovery = retained.recovery_fixture();
                        drop(retained);
                        run_provider_consult_cold_dispatch_case(
                            case,
                            fixture,
                            recovered_child,
                            host,
                            custody,
                            anchor,
                            *source,
                            dispatch_recovery,
                            lost_invocation,
                            &provider,
                            provider_clock,
                            provider_store,
                        );
                    })
                },
            )
        },
    )
}

struct ColdProviderConsultRecoveryFixture {
    selected_store: AuthorityStoreCoordinate,
    sessions: RestartSessions,
    historical: RecoveryResolver,
    child_handle: pale_ports::ControlRecordCoordinate,
    observation_limits: OperationObservationRecoveryLimits,
    checkpoint_limits: OperationCheckpointAncestryLimits,
    membership_limits: AccumulatorMembershipLimits,
    checkpoint_profile: crate::resident::SupportedOperationCheckpointProfile,
    checkpoint_fields: pale_core::records::ControlStateCheckpointPreimageV1,
}

impl ColdProviderConsultRecoveryFixture {
    fn for_child(fixture: ConsultChildFixture<'_>) -> Box<Self> {
        let ConsultChildFixture { child, parents, .. } = fixture;
        let selected_store = child.initial_candidate.store().clone();
        let session = parents.session.lease();
        let sessions = RestartSessions(SessionLeaseBody::new(
            selected_store.clone(),
            session.session_admission_lease_digest().clone(),
            session.schema_id().clone(),
            session.schema_sha256().clone(),
            pale_ports::ControlRecordBytes::new(
                pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(session))
                    .expect("session wire")
                    .into_bytes(),
            )
            .expect("bounded session wire"),
        ));
        let mut historical = RecoveryResolver::default();
        historical.insert_generation(parents.current.generation.compiled_generation);
        historical.insert_active(parents.current.generation.active_head);
        historical.insert_fence(parents.current.generation.activation_fence);
        historical.insert_effective_profile(parents.current.generation.effective_profile);
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
        let observation_limits = OperationObservationRecoveryLimits::new(8, 1 << 20, 1 << 20)
            .expect("observation bounds");
        let checkpoint_limits =
            OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).expect("checkpoint bounds");
        let membership_limits =
            AccumulatorMembershipLimits::new(8, 20_000, 200_000).expect("membership bounds");
        let checkpoint_profile = crate::local_operation_start_fixture::profile();
        let mut checkpoint_fields = checkpoint_preimage_at_store(
            child.checkpoint.domain_sequence().get() + 1,
            Some((
                child.checkpoint_ref.clone(),
                child
                    .checkpoint
                    .canonical_control_checkpoint_sha256()
                    .clone(),
            )),
            &selected_store,
        );
        checkpoint_fields.checkpoint_domain_id = child.checkpoint.checkpoint_domain_id().clone();
        checkpoint_fields.checkpoint_profile_ref =
            child.checkpoint.checkpoint_profile_ref().clone();
        checkpoint_fields.checkpoint_profile_sha256 =
            child.checkpoint.checkpoint_profile_sha256().clone();
        checkpoint_fields.domain_epoch = *child.checkpoint.domain_epoch();
        Box::new(Self {
            selected_store,
            sessions,
            historical,
            child_handle,
            observation_limits,
            checkpoint_limits,
            membership_limits,
            checkpoint_profile,
            checkpoint_fields,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn run_provider_consult_cold_loss_case(
    case: ColdProviderConsultCase,
    fixture: ConsultChildFixture<'_>,
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
    attempt: &ResidentProcessRecoveredAttempt,
    dispatch_recovery: &RetainedProviderRecoveryFixture<'_>,
    lost_invocation: &ProviderOperationInvokeInput<'_>,
    provider: &ResultPort,
    provider_clock: &ProviderClock,
    provider_store: &ProviderStore,
) {
    let ConsultChildFixture {
        child,
        parent,
        parents,
        ..
    } = fixture;
    let RecoveredConsultChild {
        parent_start,
        current,
        batch,
        candidate,
        observed_at,
        ..
    } = recovered_child;
    let recovery_fixture = ColdProviderConsultRecoveryFixture::for_child(fixture);
    let selected_store = &recovery_fixture.selected_store;
    let selector = selector_from_signing_authority(parents.signing);
    let checkpoint_signing = checkpoint_authority(&recovery_fixture.checkpoint_fields);
    let recovery = || OperationControlRecoveryInput {
        operation: LocalOperationHandleRecoveryInput {
            context: LocalOperationRecoveryContext {
                store: selected_store,
                profile: &recovery_fixture.checkpoint_profile,
                handle_signing_authority: parents.signing,
                checkpoint_authority: &checkpoint_signing,
                historical: &recovery_fixture.historical,
                maximum_record_bytes: 1 << 20,
                maximum_operation_lease_bytes: 1 << 20,
                maximum_session_lease_bytes: 1 << 20,
                membership_limits: recovery_fixture.membership_limits,
                observation_limits: recovery_fixture.observation_limits,
                checkpoint_limits: recovery_fixture.checkpoint_limits,
            },
            expected_handle: &recovery_fixture.child_handle,
            maximum_parent_depth: 1,
        },
        observation: OperationObservationSelection::SelectedHead,
        observation_limits: recovery_fixture.observation_limits,
        checkpoint_limits: recovery_fixture.checkpoint_limits,
        membership_limits: recovery_fixture.membership_limits,
    };
    let mut admit_request =
        |reference: &pale_core::records::ResidentRecordRefV1,
         wire: pale_core::records::CommonRequestEnvelopeWireV1| {
            let expected = if reference == &child.request_ref {
                &child.request
            } else if reference == child.parent_request.0.record_ref() {
                &child.parent_request.1
            } else {
                panic!("unexpected cold Consult request")
            };
            assert_eq!(
                pale_core::canon::to_canonical_string(&wire).expect("request wire encoding"),
                pale_core::canon::to_canonical_string(expected).expect("expected request encoding")
            );
            Ok(expected.clone())
        };
    let mut admit_handle_authority =
        |preimage: &pale_core::records::OperationHandlePreimageV1,
         recovered_parent: Option<&pale_core::records::OperationHandleV1>| {
            if &preimage.operation_handle_id == child.operation.handle().1.operation_handle_id() {
                assert_eq!(
                    recovered_parent
                        .expect("cold provider child retains parent")
                        .canonical_operation_handle_sha256(),
                    parent.handle().1.canonical_operation_handle_sha256()
                );
            } else {
                assert_eq!(
                    &preimage.operation_handle_id,
                    parent.handle().1.operation_handle_id()
                );
                assert!(recovered_parent.is_none());
            }
            Ok(())
        };
    let mut verify_handle_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
    let mut admit_checkpoint = |_: &_, wire| {
        pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
            wire,
            &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
            &checkpoint_signing,
            |_, _, _, _| Ok(()),
        )
    };
    let mut resolve_observation_signer = |_: &_, _: &_| Ok(parents.signing.clone());
    let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
    let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
    let mut verify_observation_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
    let mut verify_checkpoint_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
    let mut recovery_admission = OperationControlRecoveryAdmission {
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
    let cold_restart = native_recovery_input(recovery());
    let recording_calls = Rc::new(Cell::new(0));
    let recovered_dispatch = attempt.start.control.observation().selected();
    let expected_dispatch = ResidentOperationTaskKey::new(
        selected_store.clone(),
        ImmutableRecordReferenceV1 {
            record_ref: recovered_dispatch.0.clone(),
            record_sha256: ArtifactDigest::parse(
                recovered_dispatch
                    .1
                    .canonical_operation_observation_sha256()
                    .as_str(),
            )
            .expect("cold dispatch digest"),
        },
    );
    let evidence = Evidence::valid();
    let task_limits = ResidentConsultationTaskLimits::new(
        NonZeroUsize::new(1).expect("one child in flight"),
        NonZeroUsize::new(1).expect("one roster member"),
    );
    let turn = ResidentProcessConsultProviderColdLossTurn {
        attempt,
        access: ResidentDeploymentOperationAccess {
            store: selected_store,
            custody,
            sessions: &recovery_fixture.sessions,
            anchor,
        },
        task_limits,
        current: current.clone(),
        candidate,
        history: dispatch_recovery.history(),
        input: lost_invocation,
        evidence: &evidence,
        recovery: ResidentProcessConsultProviderRecordedRecovery {
            association: native_recovery_association(parents.signing, &cold_restart),
            restart: Box::new(|| native_recovery_input(recovery())),
            admission: &mut recovery_admission,
            parent: parent_start,
            planned: batch,
            observed_at,
        },
        recording: accepting_provider_recording_attempt(Rc::clone(&recording_calls)),
    };
    match case {
        ColdProviderConsultCase::CurrentHead => assert_cold_current_head_loss(
            recovered_child,
            host,
            &selector,
            turn,
            &expected_dispatch,
            &recording_calls,
            provider,
        ),
        ColdProviderConsultCase::CancellationPredecessor => {
            assert_cold_cancellation_predecessor_loss(
                recovered_child,
                host,
                &selector,
                turn,
                &expected_dispatch,
                &recording_calls,
                provider,
            )
        }
        ColdProviderConsultCase::Collision => assert_cold_collision_refusal(
            attempt,
            host,
            &selector,
            turn,
            &expected_dispatch,
            &recording_calls,
            provider,
            provider_clock,
            provider_store,
        ),
        ColdProviderConsultCase::OfferedContinuation => assert_cold_offered_continuation(
            host,
            &selector,
            turn,
            &expected_dispatch,
            &recording_calls,
            provider,
        ),
    }
}
fn cold_current_head_attempt(
    source: ResidentProcessRecoveredOperationResolution,
    dispatch_recovery: &RetainedProviderRecoveryFixture<'_>,
) -> ResidentProcessRecoveredAttempt {
    let ResidentProcessRecoveredOperationResolution {
        start:
            ResidentProcessRecoveredOperationStart {
                selector,
                control,
                generation_digest,
                lineage,
            },
        ..
    } = source;
    let (operation, _) = control.into_parts();
    ResidentProcessRecoveredAttempt {
        start: ResidentProcessRecoveredOperationStart {
            selector,
            control: RecoveredOperationControl::fixture(
                operation,
                dispatch_recovery.recovered(dispatch_recovery.checkpoint()),
            ),
            generation_digest,
            lineage,
        },
        dispatch: ResidentProcessAttemptObservation::CurrentHead,
        selection: dispatch_recovery.selection(),
    }
}

#[allow(clippy::too_many_arguments)]
fn commit_cold_provider_cancellation_predecessor(
    fixture: ConsultChildFixture<'_>,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
    dispatch_recovery: &RetainedProviderRecoveryFixture<'_>,
) {
    let ConsultChildFixture {
        child,
        parent,
        parents,
        ..
    } = fixture;
    let selected_store = child.initial_candidate.store().clone();
    let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let session = parents.session.lease();
    let sessions = RestartSessions(SessionLeaseBody::new(
        selected_store.clone(),
        session.session_admission_lease_digest().clone(),
        session.schema_id().clone(),
        session.schema_sha256().clone(),
        pale_ports::ControlRecordBytes::new(
            pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(session))
                .expect("session wire")
                .into_bytes(),
        )
        .expect("bounded session wire"),
    ));
    let mut historical = RecoveryResolver::default();
    historical.insert_generation(parents.current.generation.compiled_generation);
    historical.insert_active(parents.current.generation.active_head);
    historical.insert_fence(parents.current.generation.activation_fence);
    historical.insert_effective_profile(parents.current.generation.effective_profile);
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
    let observation_limits =
        OperationObservationRecoveryLimits::new(8, 1 << 20, 1 << 20).expect("observation bounds");
    let checkpoint_limits =
        OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).expect("checkpoint bounds");
    let membership_limits =
        AccumulatorMembershipLimits::new(8, 20_000, 200_000).expect("membership bounds");
    let checkpoint_profile = crate::local_operation_start_fixture::profile();
    let mut checkpoint_fields = checkpoint_preimage_at_store(
        child.checkpoint.domain_sequence().get() + 1,
        Some((
            child.checkpoint_ref.clone(),
            child
                .checkpoint
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
        &selected_store,
    );
    checkpoint_fields.checkpoint_domain_id = child.checkpoint.checkpoint_domain_id().clone();
    checkpoint_fields.checkpoint_profile_ref = child.checkpoint.checkpoint_profile_ref().clone();
    checkpoint_fields.checkpoint_profile_sha256 =
        child.checkpoint.checkpoint_profile_sha256().clone();
    checkpoint_fields.domain_epoch = *child.checkpoint.domain_epoch();
    let checkpoint_signing = checkpoint_authority(&checkpoint_fields);
    let recovery = || OperationControlRecoveryInput {
        operation: LocalOperationHandleRecoveryInput {
            context: LocalOperationRecoveryContext {
                store: &selected_store,
                profile: &checkpoint_profile,
                handle_signing_authority: parents.signing,
                checkpoint_authority: &checkpoint_signing,
                historical: &historical,
                maximum_record_bytes: 1 << 20,
                maximum_operation_lease_bytes: 1 << 20,
                maximum_session_lease_bytes: 1 << 20,
                membership_limits,
                observation_limits,
                checkpoint_limits,
            },
            expected_handle: &child_handle,
            maximum_parent_depth: 1,
        },
        observation: OperationObservationSelection::SelectedHead,
        observation_limits,
        checkpoint_limits,
        membership_limits,
    };
    let cancellation_request =
        a2a_cancel_task::cancellation_request(&child.request, child.operation.handle().1);
    let mut admit_request =
        |reference: &pale_core::records::ResidentRecordRefV1,
         wire: pale_core::records::CommonRequestEnvelopeWireV1| {
            let expected = if reference == &child.request_ref {
                &child.request
            } else if reference == child.parent_request.0.record_ref() {
                &child.parent_request.1
            } else {
                &cancellation_request
            };
            assert_eq!(
                pale_core::canon::to_canonical_string(&wire).expect("request wire encoding"),
                pale_core::canon::to_canonical_string(expected).expect("expected request encoding")
            );
            Ok(expected.clone())
        };
    let mut admit_handle_authority =
        |preimage: &pale_core::records::OperationHandlePreimageV1,
         recovered_parent: Option<&pale_core::records::OperationHandleV1>| {
            if &preimage.operation_handle_id == child.operation.handle().1.operation_handle_id() {
                assert_eq!(
                    recovered_parent
                        .expect("cancelled provider child retains parent")
                        .canonical_operation_handle_sha256(),
                    parent.handle().1.canonical_operation_handle_sha256()
                );
            } else {
                assert_eq!(
                    &preimage.operation_handle_id,
                    parent.handle().1.operation_handle_id()
                );
                assert!(recovered_parent.is_none());
            }
            Ok(())
        };
    let mut verify_handle_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
    let mut admit_checkpoint = |_: &_, wire| {
        pale_core::records::ControlStateCheckpointV1::admit_signed_wire(
            wire,
            &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
            &checkpoint_signing,
            |_, _, _, _| Ok(()),
        )
    };
    let mut resolve_observation_signer = |_: &_, _: &_| Ok(parents.signing.clone());
    let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
    let mut admit_rotation = |_: &_, _: &_, _: &_, _: &_| Ok(());
    let mut verify_observation_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
    let mut verify_checkpoint_signature = |_: &[u8], _: &_, _: &_, _: &_| Ok(());
    let recovered = recover_operation_control(
        recovery(),
        custody,
        &sessions,
        anchor,
        OperationControlRecoveryAdmission {
            admit_request: &mut admit_request,
            admit_handle_authority: &mut admit_handle_authority,
            verify_handle_signature: &mut verify_handle_signature,
            admit_checkpoint: &mut admit_checkpoint,
            resolve_observation_signer: &mut resolve_observation_signer,
            admit_observation_state: &mut admit_observation_state,
            admit_rotation: &mut admit_rotation,
            verify_observation_signature: &mut verify_observation_signature,
            verify_checkpoint_signature: &mut verify_checkpoint_signature,
        },
    )
    .expect("cold provider dispatch recovers before cancellation");
    let cancellation_time =
        UnixTimeV1::new(dispatch_recovery.history().dispatched.observed_at().get() + 1)
            .expect("cold provider cancellation time");
    let authorized = recovered
        .authorize_cancellation(&cancellation_request, cancellation_time, |_| Ok(()))
        .expect("cold provider cancellation authority");
    let prepared = authorized
        .prepare_dispatched_request(
            DispatchedOperationCancellationRequestInput {
                request_ref: record_ref("process-consult-provider/cold-cancellation-request"),
                observation_ref: record_ref("process-consult-provider/cold-cancellation-requested"),
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority: parents.signing,
                },
            },
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .expect("cold provider cancellation preparation");
    let prior = prepared.predecessor_checkpoint();
    let cancellation_resolver = SelectedStore::available(selected_store.clone());
    let prepared_checkpoint = prepared
        .prepare_checkpoint(
            AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000)
                .expect("cancellation accumulator limits"),
            &cancellation_resolver,
            custody,
        )
        .expect("cold provider cancellation checkpoint");
    let (_, cancellation_checkpoint) = complete_prepared_checkpoint(
        prepared_checkpoint,
        prior,
        OperationCheckpointInput {
            reference: record_ref("process-consult-provider/cold-cancellation-checkpoint"),
            id: ControlCheckpointIdV1::parse("process-consult-provider-cold-cancellation")
                .expect("cancellation checkpoint identity"),
        },
        cancellation_time,
        &checkpoint_fields.schema_sha256,
        &checkpoint_signing,
        custody,
        anchor,
        |_, _, _| Ok(signature()),
    )
    .expect("cold provider cancellation custody");
    let cancellation_evidence_key = crate::resident::checkpoint_profile::a2a_evidence_key(
        &selected_store,
        &cancellation_checkpoint.1,
        prepared.request_record().coordinate(),
    )
    .expect("cancellation evidence key");
    assert!(
        cancellation_checkpoint
            .1
            .changed_leaf_keys()
            .binary_search(&cancellation_evidence_key)
            .is_err(),
        "ordinary operation cancellation is not A2A evidence"
    );
    let cancelled = recover_operation_control(
        recovery(),
        custody,
        &sessions,
        anchor,
        OperationControlRecoveryAdmission {
            admit_request: &mut admit_request,
            admit_handle_authority: &mut admit_handle_authority,
            verify_handle_signature: &mut verify_handle_signature,
            admit_checkpoint: &mut admit_checkpoint,
            resolve_observation_signer: &mut resolve_observation_signer,
            admit_observation_state: &mut admit_observation_state,
            admit_rotation: &mut admit_rotation,
            verify_observation_signature: &mut verify_observation_signature,
            verify_checkpoint_signature: &mut verify_checkpoint_signature,
        },
    )
    .expect("cold cancellation head recovery");
    let recovered_cancellation = cancelled
        .recover_local_cancellation_request(custody, 1 << 20, |_, _| {
            Ok(cancellation_request.clone())
        })
        .expect("ordinary cancellation request recovers locally");
    drop(recovered_cancellation);
}

#[allow(clippy::too_many_arguments)]
fn cold_cancellation_predecessor_attempt(
    fixture: ConsultChildFixture<'_>,
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
    dispatch_recovery: &RetainedProviderRecoveryFixture<'_>,
) -> ResidentProcessRecoveredAttempt {
    commit_cold_provider_cancellation_predecessor(fixture, custody, anchor, dispatch_recovery);
    let classified = classify_cold_provider_consult_child(
        fixture,
        recovered_child,
        host,
        custody,
        anchor,
        None,
        1,
    );
    let ResidentProcessOperationRestartDisposition::Provider(attempt) = classified else {
        panic!("cancelled provider dispatch retains its exact provider history")
    };
    assert!(matches!(
        attempt.dispatch,
        ResidentProcessAttemptObservation::CancellationPredecessor
    ));
    *attempt
}

#[allow(clippy::too_many_arguments)]
fn run_provider_consult_cold_dispatch_case(
    case: ColdProviderConsultCase,
    fixture: ConsultChildFixture<'_>,
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
    source: ResidentProcessRecoveredOperationResolution,
    dispatch_recovery: RetainedProviderRecoveryFixture<'_>,
    mut lost_invocation: ProviderOperationInvokeInput<'_>,
    provider: &ResultPort,
    provider_clock: &ProviderClock,
    provider_store: &ProviderStore,
) {
    let attempt = match case {
        ColdProviderConsultCase::CancellationPredecessor => {
            drop(source);
            cold_cancellation_predecessor_attempt(
                fixture,
                recovered_child,
                host,
                custody,
                anchor,
                &dispatch_recovery,
            )
        }
        ColdProviderConsultCase::CurrentHead
        | ColdProviderConsultCase::Collision
        | ColdProviderConsultCase::OfferedContinuation => {
            cold_current_head_attempt(source, &dispatch_recovery)
        }
    };
    match case {
        ColdProviderConsultCase::CancellationPredecessor => assert!(matches!(
            attempt.dispatch,
            ResidentProcessAttemptObservation::CancellationPredecessor
        )),
        ColdProviderConsultCase::CurrentHead
        | ColdProviderConsultCase::Collision
        | ColdProviderConsultCase::OfferedContinuation => assert!(matches!(
            attempt.dispatch,
            ResidentProcessAttemptObservation::CurrentHead
        )),
    }
    if case == ColdProviderConsultCase::Collision {
        let recovered = attempt.start.control.observation();
        lost_invocation.result_ref = attempt
            .start
            .control
            .operation()
            .operation()
            .handle()
            .0
            .clone();
        lost_invocation.diagnostic_ref = lost_invocation.result_ref.clone();
        lost_invocation.reconciliation.record_ref =
            attempt.selection.coordinate().record_ref().clone();
        lost_invocation.terminal_ref = recovered.selected().0.clone();
        lost_invocation.terminal_checkpoint.reference = recovered
            .selected_predecessor()
            .expect("dispatch preparation")
            .0
            .clone();
        lost_invocation.terminal_checkpoint.id = recovered
            .current_checkpoint()
            .1
            .control_checkpoint_id()
            .clone();
    }
    run_provider_consult_cold_loss_case(
        case,
        fixture,
        recovered_child,
        host,
        custody,
        anchor,
        &attempt,
        &dispatch_recovery,
        &lost_invocation,
        provider,
        provider_clock,
        provider_store,
    );
}

#[allow(clippy::too_many_arguments)]
fn assert_cold_current_head_loss<'input, 'operation: 'call, 'call>(
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    turn: ResidentProcessConsultProviderColdLossTurn<'input, 'operation, 'call>,
    expected_dispatch: &ResidentOperationTaskKey,
    recording_calls: &Rc<Cell<usize>>,
    provider: &ResultPort,
) {
    let parent_head = recovered_child
        .parent_start
        .control
        .observation()
        .selected_head()
        .0
        .clone();
    let parent_frontier = recovered_child.batch.collection_frontier().clone();
    let parent_slots = recovered_child.current.collection().slots().to_vec();
    let completion = host
        .lose_cold_consultation_provider_child(selector, turn)
        .unwrap_or_else(|_| panic!("current cold dispatch must reach durable loss"));
    let ResidentProcessConsultProviderCompletionDisposition::Nonterminal(completed) = completion
    else {
        panic!("cold provider loss remains nonterminal")
    };
    let ResidentConsultationTaskPoll::Lost(lost) = completed.outcome() else {
        panic!("cold provider loss retains the exact child")
    };
    assert_eq!(lost.candidate(), recovered_child.candidate);
    assert_eq!(lost.dispatch(), expected_dispatch);
    assert_eq!(
        lost.cancellation_state(),
        ResidentOperationTaskCancellationState::Unnotified
    );
    assert_eq!(recording_calls.get(), 1);
    assert_eq!(provider.calls(), 0);
    assert_eq!(host.operations.in_flight_consultation_tasks(), 0);
    assert_eq!(host.operations.outstanding_task_reservations(), 0);
    assert_eq!(
        recovered_child
            .parent_start
            .control
            .observation()
            .selected_head()
            .0,
        &parent_head
    );
    assert_eq!(
        recovered_child.batch.collection_frontier(),
        &parent_frontier
    );
    assert_eq!(
        recovered_child.current.collection().slots(),
        parent_slots.as_slice()
    );
    let slot = recovered_child
        .current
        .collection()
        .slots()
        .iter()
        .find(|slot| slot.roster().slot_id() == recovered_child.candidate.slot_id())
        .expect("cold Consult slot remains present");
    assert!(matches!(
        slot.disposition(),
        ConsultationSlotDisposition::Outstanding
    ));
}

#[allow(clippy::too_many_arguments)]
fn assert_cold_cancellation_predecessor_loss<'input, 'operation: 'call, 'call>(
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    turn: ResidentProcessConsultProviderColdLossTurn<'input, 'operation, 'call>,
    expected_dispatch: &ResidentOperationTaskKey,
    recording_calls: &Rc<Cell<usize>>,
    provider: &ResultPort,
) {
    let parent_head = recovered_child
        .parent_start
        .control
        .observation()
        .selected_head()
        .0
        .clone();
    let parent_frontier = recovered_child.batch.collection_frontier().clone();
    let parent_slots = recovered_child.current.collection().slots().to_vec();
    let completion = host
        .lose_cold_consultation_provider_child(selector, turn)
        .unwrap_or_else(|_| panic!("cancellation predecessor must reach durable cold loss"));
    let ResidentProcessConsultProviderCompletionDisposition::Nonterminal(completed) = completion
    else {
        panic!("cancelled cold provider loss remains nonterminal")
    };
    let ResidentConsultationTaskPoll::Lost(lost) = completed.outcome() else {
        panic!("cancelled cold provider loss retains the exact child")
    };
    assert_eq!(lost.candidate(), recovered_child.candidate);
    assert_eq!(lost.dispatch(), expected_dispatch);
    assert_eq!(
        lost.cancellation_state(),
        ResidentOperationTaskCancellationState::Unnotified
    );
    assert_eq!(recording_calls.get(), 1);
    assert_eq!(provider.calls(), 0);
    assert_eq!(host.operations.in_flight_consultation_tasks(), 0);
    assert_eq!(host.operations.outstanding_task_reservations(), 0);
    assert_eq!(
        recovered_child
            .parent_start
            .control
            .observation()
            .selected_head()
            .0,
        &parent_head
    );
    assert_eq!(
        recovered_child.batch.collection_frontier(),
        &parent_frontier
    );
    assert_eq!(
        recovered_child.current.collection().slots(),
        parent_slots.as_slice()
    );
    let slot = recovered_child
        .current
        .collection()
        .slots()
        .iter()
        .find(|slot| slot.roster().slot_id() == recovered_child.candidate.slot_id())
        .expect("cancelled cold Consult slot remains present");
    assert!(matches!(
        slot.disposition(),
        ConsultationSlotDisposition::Outstanding
    ));
}

#[allow(clippy::too_many_arguments)]
fn assert_cold_collision_refusal<'input, 'operation: 'call, 'call>(
    attempt: &ResidentProcessRecoveredAttempt,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    turn: ResidentProcessConsultProviderColdLossTurn<'input, 'operation, 'call>,
    expected_dispatch: &ResidentOperationTaskKey,
    recording_calls: &Rc<Cell<usize>>,
    provider: &ResultPort,
    provider_clock: &ProviderClock,
    provider_store: &ProviderStore,
) {
    let reads_before_refusal = provider_clock.reads();
    let stores_before_refusal = provider_store.calls();
    let failure = match host.lose_cold_consultation_provider_child(selector, turn) {
        Err(failure) => failure,
        Ok(_) => panic!("colliding cold outputs must refuse"),
    };
    let ResidentProcessConsultProviderColdLossFailure::Selection(refusal) = *failure else {
        panic!("colliding cold outputs refuse before local loss ownership")
    };
    let (returned, reason) = refusal.into_parts();
    assert!(matches!(
        reason,
        ResidentProcessHostSelectionError::ProviderOperationAuthority
    ));
    assert_eq!(&returned.attempt.start.selector, selector);
    assert_eq!(
        expected_dispatch.dispatch().record_ref,
        *attempt.start.control.observation().selected().0
    );
    assert_eq!(provider_clock.reads(), reads_before_refusal);
    assert_eq!(provider_store.calls(), stores_before_refusal);
    assert_eq!(recording_calls.get(), 0);
    assert_eq!(provider.calls(), 0);
    assert_eq!(host.operations.outstanding_task_reservations(), 0);
}

#[allow(clippy::too_many_arguments)]
fn assert_cold_offered_continuation<'input, 'operation: 'call, 'call>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    turn: ResidentProcessConsultProviderColdLossTurn<'input, 'operation, 'call>,
    expected_dispatch: &ResidentOperationTaskKey,
    recording_calls: &Rc<Cell<usize>>,
    provider: &ResultPort,
) {
    let recovered = turn.attempt;
    let expected_candidate = turn.candidate;
    let (dispatch_ref, dispatch) = recovered.start.control.observation().selected();
    let driver = host
        .consultation_child_driver(selector)
        .expect("Offered continuation selects its Consult owner");
    let reconciled = driver
        .operations
        .reconcile_consultation_batch(turn.recovery.planned.clone(), turn.task_limits)
        .expect("Offered continuation reopens its exact batch");
    let coordinate = match reconciled {
        crate::resident::consultation::service::task::ResidentConsultationBatchReconciliation::Reused(coordinate)
        | crate::resident::consultation::service::task::ResidentConsultationBatchReconciliation::Opened(coordinate)
        | crate::resident::consultation::service::task::ResidentConsultationBatchReconciliation::Advanced { opened: coordinate, .. } => coordinate,
        crate::resident::consultation::service::task::ResidentConsultationBatchReconciliation::Busy { .. } => {
            panic!("Offered continuation owns one available cold batch")
        }
    };
    let prepared = driver
        .operations
        .prepare_recovered_consultation_loss(
            &coordinate,
            turn.candidate,
            &turn.current,
            turn.recovery.observed_at,
            recovered.start.control.operation().store(),
            recovered.start.control.operation().operation(),
            dispatch_ref,
            dispatch,
        )
        .expect("Offered continuation prepares its historical loss");
    let pending = driver
        .operations
        .offer_recovered_consultation_loss(prepared)
        .expect("Offered continuation owns one aggregate offer");
    assert_eq!(pending.dispatch(), expected_dispatch);
    driver
        .operations
        .set_consultation_reservation_phase_for_test(
            pending.dispatch(),
            pending.coordinate(),
            crate::resident::task_reservation::ResidentTaskReservationPhase::Accepted,
        );
    let lower_failure = match driver
        .operations
        .retry_recovered_consultation_loss::<
            ResidentProcessConsultProviderChildWork<'_, '_>,
            ResidentProcessConsultProviderChildOutput,
        >(pending)
    {
        Err(failure) => failure,
        Ok(_) => panic!("wrong aggregate phase must retain the Offered owner"),
    };
    drop(driver);
    let ResidentConsultationTaskLossCommitFailure::Offered { pending, reason } = lower_failure
    else {
        panic!("wrong aggregate phase remains an Offered continuation")
    };
    assert!(matches!(
        reason,
        crate::resident::task_reservation::ResidentTaskReservationTransitionFailure::Phase {
            expected: crate::resident::task_reservation::ResidentTaskReservationPhase::Offered,
            actual: crate::resident::task_reservation::ResidentTaskReservationPhase::Accepted,
        }
    ));
    let process_failure = project_cold_loss_offered_for_test(selector, turn, pending, reason);
    let ResidentProcessConsultProviderColdLossFailure::Offered(continuation) = *process_failure
    else {
        panic!("lower Offered owner projects one process-host continuation")
    };
    let pending_address = continuation.pending() as *const _;
    let mut wrong_selector = selector.clone();
    wrong_selector.principal_id = pale_core::records::ResidentPrincipalIdV1::parse(
        "process-consult-provider-cold-offered-wrong-principal",
    )
    .expect("wrong Offered-continuation principal");
    let failure =
        match host.continue_cold_consultation_provider_loss(&wrong_selector, *continuation) {
            Err(failure) => failure,
            Ok(_) => panic!("wrong selector must retain the complete Offered continuation"),
        };
    let ResidentProcessConsultProviderColdLossFailure::Offered(continuation) = *failure else {
        panic!("wrong selector returns the exact Offered continuation")
    };
    assert_eq!(continuation.pending() as *const _, pending_address);
    assert_eq!(continuation.pending().dispatch(), expected_dispatch);
    assert_eq!(recording_calls.get(), 0);
    assert_eq!(provider.calls(), 0);
    host.operations.set_consultation_reservation_phase_for_test(
        continuation.pending().dispatch(),
        continuation.pending().coordinate(),
        crate::resident::task_reservation::ResidentTaskReservationPhase::Offered,
    );
    let completion = host
        .continue_cold_consultation_provider_loss(selector, *continuation)
        .unwrap_or_else(|_| panic!("correct selector retries only the retained Offered owner"));
    let ResidentProcessConsultProviderCompletionDisposition::Nonterminal(completed) = completion
    else {
        panic!("Offered retry remains a nonterminal cold loss")
    };
    let ResidentConsultationTaskPoll::Lost(lost) = completed.outcome() else {
        panic!("Offered retry hands the exact loss to durable recording")
    };
    assert_eq!(lost.candidate(), expected_candidate);
    assert_eq!(lost.dispatch(), expected_dispatch);
    assert_eq!(recording_calls.get(), 1);
    assert_eq!(provider.calls(), 0);
    assert_eq!(host.operations.in_flight_consultation_tasks(), 0);
    assert_eq!(host.operations.outstanding_task_reservations(), 0);
}

pub(super) fn run_provider_consult_cold_current_head_tail(
    fixture: ConsultChildFixture<'_>,
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
) {
    run_provider_consult_cold_case(
        ColdProviderConsultCase::CurrentHead,
        fixture,
        recovered_child,
        host,
        custody,
        anchor,
    );
}

pub(super) fn run_provider_consult_cold_cancellation_predecessor_tail(
    fixture: ConsultChildFixture<'_>,
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
) {
    run_provider_consult_cold_case(
        ColdProviderConsultCase::CancellationPredecessor,
        fixture,
        recovered_child,
        host,
        custody,
        anchor,
    );
}

pub(super) fn run_provider_consult_cold_collision_tail(
    fixture: ConsultChildFixture<'_>,
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
) {
    run_provider_consult_cold_case(
        ColdProviderConsultCase::Collision,
        fixture,
        recovered_child,
        host,
        custody,
        anchor,
    );
}

pub(super) fn run_provider_consult_cold_offered_tail(
    fixture: ConsultChildFixture<'_>,
    recovered_child: RecoveredConsultChild<'_>,
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
) {
    run_provider_consult_cold_case(
        ColdProviderConsultCase::OfferedContinuation,
        fixture,
        recovered_child,
        host,
        custody,
        anchor,
    );
}
