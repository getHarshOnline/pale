//! Resident-service composition over an exact retained A2A binding.
//!
//! The fake carrier performs no network I/O. Signatures, route evidence, and
//! accumulator work are explicit test models and are not production claims.

use super::*;
use crate::local_operation_start_fixture::{
    checkpoint_authority, checkpoint_preimage, issue_observation, signature, store_coordinate,
};
use crate::resident::a2a_get_task_owner::{a2a_get_task_key, ResidentA2aGetTaskPoll};
use crate::resident::operation_observation_custody::tests::native_execution as custody;
use crate::resident::operation_service::{
    ResidentA2aGetTaskReleaseFailure, ResidentOperationService, ResidentOperationTaskState,
};
use crate::resident::task_owner::ResidentOperationTaskLimits;
use crate::resident::task_reservation::ResidentAggregateTaskLimits;
use crate::resident::{
    prepare_a2a_task_binding_custody, prepare_operation_observation_append, A2aGetTaskCommitInput,
    A2aGetTaskObservationInput, A2aGetTaskReadbackInput, A2aRouteAssociation,
    A2aTaskBindingCustodyInput, A2aTaskObservationSelection, A2aTaskRecoveryAdmission,
    A2aTaskRecoveryInput, A2aTaskRecoveryLimits, OperationAttemptHistoryInput,
    OperationCheckpointAncestryLimits, OperationCheckpointInput,
    OperationObservationAuthorityStoreRejection, OperationObservationAuthorityStoreResolver,
    OperationObservationRecoveryAdmission, OperationObservationRecoveryLimits,
    OperationObservationTransition, RecoveredA2aTaskHistory, RecoveredOperationObservation,
};
use crate::session_admission_fixture::{LocalStartContext, ProvingAuthority};
use pale_ports::{
    A2aExchange, A2aExchangeError, A2aExchangeEvidence, A2aExchangeFacts, A2aExchangeInput,
    A2aExchangeLimits, A2aExchangePort, A2aExchangeRequest, A2aPreparedExchange,
    AccumulatorMembershipLimits, AccumulatorPreparationLimits, AntiRollbackAnchorPort,
    OperationA2aEvidence, OperationCustodyIndexPredecessor, OperationCustodyPort,
    OperationCustodyRecordKind, WallClockSourceError, WallClockSourcePort,
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};

fn nonzero(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).expect("positive fixture bound")
}

pub(in crate::resident) fn exchange_limits() -> A2aExchangeLimits {
    A2aExchangeLimits {
        request_bytes: nonzero(1024),
        response_bytes: nonzero(2048),
        evidence_bytes: nonzero(4096),
        line_bytes: nonzero(512),
        event_bytes: nonzero(1024),
        events: nonzero(4),
        queued_events: nonzero(2),
        connect_timeout: Duration::from_secs(1),
        read_timeout: Duration::from_secs(2),
        total_timeout: Duration::from_secs(3),
    }
}

fn evidence(reference: &str, bytes: &[u8]) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: record_ref(reference),
        record_sha256: ArtifactDigest::hash_bytes(bytes),
    }
}

fn raw_evidence(reference: &str, bytes: &[u8]) -> OperationA2aEvidence {
    OperationA2aEvidence::new(evidence(reference, bytes), bytes.to_vec(), None, 8_192)
        .expect("bounded A2A evidence")
}

fn checkpoint_signing_authority(
    checkpoint: &ControlStateCheckpointV1,
) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
    ControlCheckpointSigningAuthorityAdmissionContextV1 {
        authority_ref: checkpoint.checkpoint_signing_authority_ref(),
        authority_sha256: checkpoint.checkpoint_signing_authority_sha256(),
        principal_id: checkpoint.issuer_principal_id(),
        control_store_id: checkpoint.control_store_id(),
        control_incarnation_id: checkpoint.control_incarnation_id(),
        checkpoint_domain_id: checkpoint.checkpoint_domain_id(),
        checkpoint_profile_ref: checkpoint.checkpoint_profile_ref(),
        checkpoint_profile_sha256: checkpoint.checkpoint_profile_sha256(),
        subject_key_id: checkpoint.issuer_key_id(),
        subject_key_epoch: *checkpoint.issuer_key_epoch(),
    }
}

struct ReadbackStore;

impl OperationObservationAuthorityStoreResolver for ReadbackStore {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<pale_ports::AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection>
    {
        Ok(store_coordinate())
    }
}

fn route(
    handle: &OperationHandleV1,
    authority: &SigningAuthorityBindingV1,
    provider: ProviderBindingRefV1,
) -> A2aRouteAdmissionV1 {
    let binding = handle.request_binding();
    let selected_route = CommonRequestRouteV1 {
        origin_principal_id: handle.issuer_principal_id().clone(),
        origin_node_id: ResidentNodeIdV1::parse("node-1").unwrap(),
        destination_principal_id: ResidentPrincipalIdV1::parse("remote-principal").unwrap(),
        destination_node_id: Some(ResidentNodeIdV1::parse("remote-node").unwrap()),
        route_id: ResidentRouteIdV1::parse("readback-route").unwrap(),
        route_generation: *binding.route_generation(),
        transport: BoundedResidentTextV1::parse("a2a-json-rpc2-https").unwrap(),
        route_binding_schema_id: binding.route_binding_schema_id().clone(),
        route_binding_schema_sha256: binding.route_binding_schema_sha256().clone(),
        route_binding_ref: binding.route_binding_ref().clone(),
        route_binding_sha256: binding.route_binding_sha256().clone(),
        contract_set_ref: binding.contract_set_ref().clone(),
        contract_set_sha256: binding.contract_set_sha256().clone(),
    };
    let schema = schema_digest(A2A_ROUTE_ADMISSION_V1_SCHEMA_ID);
    A2aRouteAdmissionV1::issue_contextual(
        A2aRouteAdmissionPreimageV1 {
            schema_id: SchemaId::parse(A2A_ROUTE_ADMISSION_V1_SCHEMA).unwrap(),
            schema_sha256: schema.clone(),
            route_admission_id: A2aRouteAdmissionIdV1::parse("readback-route").unwrap(),
            provider_binding: provider.clone(),
            route: selected_route.clone(),
            endpoint: A2aHttpsEndpointV1::parse("https://agent.example.test/a2a").unwrap(),
            protocol_version: A2aProtocolVersionV1::V1_0,
            request_binding: A2aRequestBindingV1::JsonRpc2OverVerifiedHttps,
            agent_card: evidence("a2a/readback-card", b"fixture Agent Card"),
            streaming: A2aStreamingAdmissionV1::NotAdvertised,
            security: A2aSecurityAdmissionV1 {
                selected_scheme_ids: vec![],
                evidence: evidence("a2a/readback-security", b"fixture HTTPS trust"),
            },
            observed_at: UnixTimeV1::new(10).unwrap(),
            fresh_until: UnixTimeV1::new(70).unwrap(),
            expires_at: UnixTimeV1::new(75).unwrap(),
            deployment_runtime_authority_ref: handle.deployment_runtime_authority_ref().clone(),
            deployment_runtime_authority_sha256: handle
                .deployment_runtime_authority_sha256()
                .clone(),
            issuer_principal_id: handle.issuer_principal_id().clone(),
            issuer_key_id: handle.issuer_key_id().clone(),
            issuer_key_epoch: *handle.issuer_key_epoch(),
        },
        &A2aRouteAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema,
                signing_authority: authority,
            },
            provider_binding: &provider,
            route: &selected_route,
        },
        |_| Ok(()),
        |_, _, _| Ok(signature()),
    )
    .expect("exact route fixture")
}

struct RetainedRemoteFixture {
    task_binding_id: A2aTaskBindingIdV1,
    route_ref: ResidentRecordRefV1,
    route: A2aRouteAdmissionV1,
    binding_ref: ResidentRecordRefV1,
    observation_ref: ResidentRecordRefV1,
    checkpoint_ref: ResidentRecordRefV1,
}

#[allow(clippy::too_many_arguments)]
fn retain_remote_history(
    attempt: &OperationAttemptHistoryInput<'_>,
    dispatch_ref: ResidentRecordRefV1,
    signing: &SigningAuthorityBindingV1,
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    store: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
) -> (RetainedRemoteFixture, ControlStateCheckpointV1) {
    let operation = attempt.operation;
    let handle = operation.handle();
    let route_ref = record_ref("a2a/readback-route");
    let route = route(
        handle.1,
        signing,
        attempt
            .generation
            .provider_binding("provider-binding-claude-code")
            .map(|provider| ProviderBindingRefV1 {
                provider_binding_id: provider.id.clone(),
                provider_binding_identity: provider.binding_identity.clone(),
            })
            .expect("compiled provider fixture"),
    );
    let request_evidence =
        raw_evidence("a2a/original-send-request", b"fixture SendMessage request");
    let response_evidence = raw_evidence("a2a/original-send-response", b"fixture Task response");
    let binding_ref = record_ref("a2a/readback-binding");
    let binding_schema = schema_digest(A2A_TASK_BINDING_V1_SCHEMA_ID);
    let task_binding_id = A2aTaskBindingIdV1::parse("readback-binding").unwrap();
    let binding = A2aTaskBindingPreimageV1 {
        schema_id: SchemaId::parse(A2A_TASK_BINDING_V1_SCHEMA).unwrap(),
        schema_sha256: binding_schema.clone(),
        task_binding_id: task_binding_id.clone(),
        operation_handle_id: handle.1.operation_handle_id().clone(),
        operation_handle_ref: handle.0.clone(),
        operation_handle_sha256: handle.1.canonical_operation_handle_sha256().clone(),
        operation_observation_ref: dispatch_ref.clone(),
        operation_observation_sha256: attempt
            .dispatched
            .canonical_operation_observation_sha256()
            .clone(),
        route_admission_id: route.route_admission_id().clone(),
        route_admission_ref: route_ref.clone(),
        route_admission_sha256: route.canonical_a2a_route_admission_sha256().clone(),
        attempt_id: attempt
            .dispatched
            .state()
            .dispatched_attempt_id()
            .expect("dispatched attempt")
            .clone(),
        json_rpc_request_id: A2aJsonRpcRequestIdV1::parse("send-1").unwrap(),
        request_mode: A2aTaskRequestModeV1::SendMessage,
        remote_task_id: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
        remote_context_id: A2aRemoteContextIdV1::parse("context-1").unwrap(),
        request_evidence: evidence("a2a/original-send-request", b"fixture SendMessage request"),
        initial_task_evidence: evidence("a2a/original-send-response", b"fixture Task response"),
        bound_at: UnixTimeV1::new(64).unwrap(),
        deployment_runtime_authority_ref: handle.1.deployment_runtime_authority_ref().clone(),
        deployment_runtime_authority_sha256: handle.1.deployment_runtime_authority_sha256().clone(),
        issuer_principal_id: handle.1.issuer_principal_id().clone(),
        issuer_key_id: handle.1.issuer_key_id().clone(),
        issuer_key_epoch: *handle.1.issuer_key_epoch(),
    };
    let binding_context = A2aTaskBindingAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &binding_schema,
            signing_authority: signing,
        },
        operation_handle_ref: handle.0,
        operation_handle: handle.1,
        operation_observation_ref: &dispatch_ref,
        operation_observation: attempt.dispatched,
        route_admission_ref: &route_ref,
        route_admission: &route,
    };
    let observation_ref = record_ref("a2a/readback-observation-1");
    let observation_schema = schema_digest(A2A_TASK_OBSERVATION_V1_SCHEMA_ID);
    let observation = A2aTaskObservationPreimageV1 {
        schema_id: SchemaId::parse(A2A_TASK_OBSERVATION_V1_SCHEMA).unwrap(),
        schema_sha256: observation_schema.clone(),
        task_binding_id: task_binding_id.clone(),
        task_binding_ref: binding_ref.clone(),
        task_binding_sha256: A2aTaskBindingDigest::hash_canonical(&binding).unwrap(),
        revision: A2aTaskObservationRevisionV1::new(1).unwrap(),
        predecessor_ref: None,
        predecessor_sha256: None,
        source: A2aTaskObservationSourceV1::InitialSendMessageResponse {
            evidence: evidence("a2a/original-send-response", b"fixture Task response"),
        },
        remote_state: A2aRemoteTaskStateV1::Working,
        reconciliation: None,
        observed_at: UnixTimeV1::new(64).unwrap(),
        deployment_runtime_authority_ref: handle.1.deployment_runtime_authority_ref().clone(),
        deployment_runtime_authority_sha256: handle.1.deployment_runtime_authority_sha256().clone(),
        issuer_principal_id: handle.1.issuer_principal_id().clone(),
        issuer_key_id: handle.1.issuer_key_id().clone(),
        issuer_key_epoch: *handle.1.issuer_key_epoch(),
    };
    let local = RecoveredOperationObservation::fixture(
        operation,
        (dispatch_ref.clone(), attempt.dispatched.clone()),
        (dispatch_ref.clone(), attempt.dispatched.clone()),
        (checkpoint.0.clone(), checkpoint.1.clone()),
        store_coordinate(),
    );
    let append = prepare_a2a_task_binding_custody(
        A2aTaskBindingCustodyInput {
            operation_custody: &local,
            binding_ref: binding_ref.clone(),
            binding,
            binding_context,
            observation_ref: observation_ref.clone(),
            observation,
            observation_signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &observation_schema,
                signing_authority: signing,
            },
            route_predecessor: OperationCustodyIndexPredecessor::Absent,
            route_association: A2aRouteAssociation::Insert,
            new_evidence: vec![request_evidence, response_evidence],
            retained_evidence: vec![],
            maximum_evidence_bytes: 8_192,
            maximum_record_bytes: 65_536,
        },
        |_, _, evidence, retained| {
            assert_eq!(evidence.len(), 2);
            assert!(retained.is_empty());
            Ok(())
        },
        |_, _, _| Ok(signature()),
        |_, _, _| Ok(signature()),
        |_| Ok(store_coordinate()),
    )
    .expect("initial remote history must be admitted before retention");
    let prepared = append
        .prepare_checkpoint(
            operation.checkpoint_selection(),
            checkpoint.0,
            checkpoint.1,
            AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000).unwrap(),
            AccumulatorMembershipLimits::new(8, 20_000, 200_000).unwrap(),
            store,
        )
        .expect("initial remote history must prepare exact roots");
    let mut fields = checkpoint_preimage(
        checkpoint.1.domain_sequence().get() + 1,
        Some((
            checkpoint.0.clone(),
            checkpoint.1.canonical_control_checkpoint_sha256().clone(),
        )),
    );
    fields.checkpoint_domain_id = checkpoint.1.checkpoint_domain_id().clone();
    fields.checkpoint_profile_ref = checkpoint.1.checkpoint_profile_ref().clone();
    fields.checkpoint_profile_sha256 = checkpoint.1.checkpoint_profile_sha256().clone();
    fields.committed_transaction_sequence = prepared.transaction_sequence();
    fields.state_root_sha256 = prepared.state_root().clone();
    fields.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
    fields.changed_leaf_keys = prepared.changed_keys().to_vec();
    fields.leaf_count = prepared.leaf_count();
    fields.issued_at = UnixTimeV1::new(64).unwrap();
    let checkpoint_ref = record_ref("a2a/readback-initial-checkpoint");
    let authority = checkpoint_authority(&fields);
    let (physical, checkpoint_value) = prepared
        .issue_checkpoint_with_value(
            &checkpoint_ref,
            fields.clone(),
            &fields.schema_sha256,
            &authority,
            |_, _, _| Ok(signature()),
        )
        .expect("initial remote history must issue one checkpoint");
    physical
        .commit(store, anchor)
        .expect("initial remote history must enter custody")
        .advance_anchor(anchor)
        .expect("initial remote history must advance the anchor");
    (
        RetainedRemoteFixture {
            task_binding_id,
            route_ref,
            route,
            binding_ref,
            observation_ref,
            checkpoint_ref,
        },
        checkpoint_value,
    )
}

#[allow(clippy::too_many_arguments)]
struct LocalObservationAppend<'a> {
    operation: &'a IssuedLocalOperation,
    predecessor: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    checkpoint: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    observation_ref: ResidentRecordRefV1,
    observation: &'a OperationObservationV1,
    checkpoint_ref: ResidentRecordRefV1,
}

fn retain_local_observation(
    input: LocalObservationAppend<'_>,
    store: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> (ResidentRecordRefV1, ControlStateCheckpointV1) {
    let append = prepare_operation_observation_append(
        OperationObservationTransition::new(
            input.operation.handle().0,
            input.operation.handle().1,
            input.predecessor.0,
            input.predecessor.1,
            &input.observation_ref,
            input.observation,
        ),
        &ReadbackStore,
    )
    .expect("the local observation successor must project before custody");
    let prepared = append
        .prepare_checkpoint(
            input.operation.checkpoint_selection(),
            input.checkpoint.0,
            input.checkpoint.1,
            AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000).unwrap(),
            store,
        )
        .expect("the local observation successor must prepare exact roots");
    let mut fields = checkpoint_preimage(
        input.checkpoint.1.domain_sequence().get() + 1,
        Some((
            input.checkpoint.0.clone(),
            input
                .checkpoint
                .1
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
    );
    fields.checkpoint_domain_id = input.checkpoint.1.checkpoint_domain_id().clone();
    fields.checkpoint_profile_ref = input.checkpoint.1.checkpoint_profile_ref().clone();
    fields.checkpoint_profile_sha256 = input.checkpoint.1.checkpoint_profile_sha256().clone();
    fields.committed_transaction_sequence = prepared.transaction_sequence();
    fields.state_root_sha256 = prepared.state_root().clone();
    fields.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
    fields.changed_leaf_keys = prepared.changed_keys().to_vec();
    fields.leaf_count = prepared.leaf_count();
    fields.issued_at = *input.observation.observed_at();
    let authority = checkpoint_authority(&fields);
    let (physical, checkpoint_value) = prepared
        .issue_checkpoint(
            &input.checkpoint_ref,
            fields.clone(),
            &fields.schema_sha256,
            &authority,
            |_, _, _| Ok(signature()),
        )
        .expect("the local observation checkpoint must be issued exactly once");
    physical
        .commit(store, anchor)
        .expect("the local observation must commit")
        .advance_anchor(anchor)
        .expect("the local observation checkpoint must advance the anchor");
    (input.checkpoint_ref, checkpoint_value)
}

fn retain_local_cancellation_head(
    operation: &IssuedLocalOperation,
    predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    signing: &SigningAuthorityBindingV1,
    store: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> (
    ResidentRecordRefV1,
    OperationObservationV1,
    ResidentRecordRefV1,
    ControlStateCheckpointV1,
) {
    let observation_ref = record_ref("a2a/readback-local-cancellation");
    let observation = issue_observation(
        signing,
        operation.handle().0,
        operation.handle().1,
        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
        predecessor.1.revision().checked_successor().unwrap(),
        Some(predecessor),
        OperationObservationStateV1::CancellationRequested {
            request: ImmutableRecordReferenceV1 {
                record_ref: record_ref("a2a/readback-cancellation-request"),
                record_sha256: ArtifactDigest::hash_bytes(b"fixture cancellation request"),
            },
        },
        66,
    );
    let checkpoint_ref = record_ref("a2a/readback-cancellation-checkpoint");
    let (_, checkpoint_value) = retain_local_observation(
        LocalObservationAppend {
            operation,
            predecessor,
            checkpoint,
            observation_ref: observation_ref.clone(),
            observation: &observation,
            checkpoint_ref: checkpoint_ref.clone(),
        },
        store,
        anchor,
    );
    (
        observation_ref,
        observation,
        checkpoint_ref,
        checkpoint_value,
    )
}

fn recover_task_history(
    service: &mut ResidentOperationService<'_, ProvingAuthority>,
    operation: &IssuedLocalOperation,
    retained: &RetainedRemoteFixture,
    checkpoint: &ControlStateCheckpointV1,
    signing: &SigningAuthorityBindingV1,
    observation: A2aTaskObservationSelection<'_>,
    verify_fixture_bytes: bool,
) -> RecoveredA2aTaskHistory {
    let checkpoint_signer = checkpoint_signing_authority(checkpoint);
    let local_admission = OperationObservationRecoveryAdmission {
        admit_checkpoint: |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
            ControlStateCheckpointV1::admit_signed_wire(
                wire,
                checkpoint.schema_sha256(),
                &checkpoint_signer,
                |_: &[u8],
                 _: &ControlStateCheckpointPreimageV1,
                 _: &ControlStateCheckpointDigest,
                 actual: &CanonicalSignatureV1| {
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            )
        },
        resolve_signer: |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
            Ok(signing.clone())
        },
        admit_state: |_: &OperationObservationStateV1| Ok(()),
        admit_rotation: |_: &ResidentRecordRefV1,
                         _: &ArtifactDigest,
                         _: &ResidentRecordRefV1,
                         _: &ArtifactDigest|
         -> Result<(), InvalidResidentRecordV1> {
            panic!("the local fixture does not rotate authority")
        },
        verify_signature: |_: &[u8],
                           _: &OperationObservationPreimageV1,
                           _: &OperationObservationDigest,
                           actual: &CanonicalSignatureV1| {
            assert_eq!(actual, &signature());
            Ok(())
        },
    };
    let mut admit_route = |reference: &ResidentRecordRefV1, wire: A2aRouteAdmissionWireV1| {
        assert_eq!(reference, &retained.route_ref);
        A2aRouteAdmissionV1::admit_contextual_wire(
            wire,
            &A2aRouteAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: retained.route.schema_sha256(),
                    signing_authority: signing,
                },
                provider_binding: retained.route.provider_binding(),
                route: retained.route.route(),
            },
            |_| Ok(()),
            |_, _, _, actual| {
                assert_eq!(actual, &signature());
                Ok(())
            },
        )
    };
    let mut resolve_binding_signer = |reference: &ResidentRecordRefV1, _: &A2aTaskBindingWireV1| {
        assert_eq!(reference, &retained.binding_ref);
        Ok(signing.clone())
    };
    let mut admit_binding_evidence =
        |_: &A2aTaskBindingPreimageV1, evidence: &[&OperationA2aEvidence]| {
            assert_eq!(evidence.len(), 2);
            if verify_fixture_bytes {
                assert_eq!(evidence[0].bytes(), b"fixture SendMessage request");
                assert_eq!(evidence[1].bytes(), b"fixture Task response");
            }
            Ok(())
        };
    let mut verify_binding_signature =
        |_: &[u8],
         _: &A2aTaskBindingPreimageV1,
         _: &A2aTaskBindingDigest,
         actual: &CanonicalSignatureV1| {
            assert_eq!(actual, &signature());
            Ok(())
        };
    let mut resolve_observation_signer =
        |_: &ResidentRecordRefV1, _: &A2aTaskObservationWireV1| Ok(signing.clone());
    let mut admit_observation_evidence =
        |preimage: &A2aTaskObservationPreimageV1, evidence: &[&OperationA2aEvidence]| {
            match &preimage.source {
                A2aTaskObservationSourceV1::InitialSendMessageResponse { .. } => {
                    assert_eq!(evidence.len(), 1);
                    if verify_fixture_bytes {
                        assert_eq!(evidence[0].bytes(), b"fixture Task response");
                    }
                }
                A2aTaskObservationSourceV1::GetTask { .. } => {
                    assert_eq!(evidence.len(), 2);
                }
                _ => panic!("the readback fixture retains no other remote source"),
            }
            Ok(())
        };
    let mut admit_remote_rotation = |_: &ResidentRecordRefV1,
                                     _: &ArtifactDigest,
                                     _: &ResidentRecordRefV1,
                                     _: &ArtifactDigest|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("the remote fixture does not rotate authority")
    };
    let mut verify_observation_signature =
        |_: &[u8],
         _: &A2aTaskObservationPreimageV1,
         _: &A2aTaskObservationDigest,
         actual: &CanonicalSignatureV1| {
            assert_eq!(actual, &signature());
            Ok(())
        };
    let coordinate = store_coordinate();
    service
        .recover_a2a_task_history(
            A2aTaskRecoveryInput {
                store: &coordinate,
                operation,
                task_binding_id: &retained.task_binding_id,
                observation,
                checkpoint_authority: &checkpoint_signer,
                local_limits: OperationObservationRecoveryLimits::new(8, 1 << 20, 65_536).unwrap(),
                checkpoint_limits: OperationCheckpointAncestryLimits::new(16, 1 << 20, 16).unwrap(),
                membership_limits: AccumulatorMembershipLimits::new(32, 100_000, 1_000_000)
                    .unwrap(),
                limits: A2aTaskRecoveryLimits::new(4, 65_536, 1 << 20, 8, 16_384).unwrap(),
            },
            local_admission,
            |_: &[u8],
             _: &ControlStateCheckpointPreimageV1,
             _: &ControlStateCheckpointDigest,
             actual: &CanonicalSignatureV1| {
                assert_eq!(actual, &signature());
                Ok(())
            },
            A2aTaskRecoveryAdmission {
                admit_route: &mut admit_route,
                resolve_binding_signer: &mut resolve_binding_signer,
                admit_binding_evidence: &mut admit_binding_evidence,
                verify_binding_signature: &mut verify_binding_signature,
                resolve_observation_signer: &mut resolve_observation_signer,
                admit_observation_evidence: &mut admit_observation_evidence,
                admit_rotation: &mut admit_remote_rotation,
                verify_observation_signature: &mut verify_observation_signature,
            },
        )
        .expect("service must recover the exact retained A2A history")
}

/// Recover the same remote binding after a newly committed local successor.
///
/// Higher-owner tests use this helper to prove that fresh preflight consumes
/// the durable `CancellationRequested` head instead of an in-memory surrogate.
pub(in crate::resident) fn recover_a2a_task_history_after_local_commit(
    previous: &RecoveredA2aTaskHistory,
    operation: &IssuedLocalOperation,
    checkpoint: &ControlStateCheckpointV1,
    signing: &SigningAuthorityBindingV1,
    authority: &mut ProvingAuthority,
    store: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> RecoveredA2aTaskHistory {
    recover_a2a_task_history_after_local_commit_with_evidence(
        previous, operation, checkpoint, signing, authority, store, anchor, true,
    )
}

/// Recover production-backed Task history after a new local successor.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn recover_retained_a2a_task_history_after_local_commit(
    previous: &RecoveredA2aTaskHistory,
    operation: &IssuedLocalOperation,
    checkpoint: &ControlStateCheckpointV1,
    signing: &SigningAuthorityBindingV1,
    authority: &mut ProvingAuthority,
    store: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> RecoveredA2aTaskHistory {
    recover_a2a_task_history_after_local_commit_with_evidence(
        previous, operation, checkpoint, signing, authority, store, anchor, false,
    )
}

#[allow(clippy::too_many_arguments)]
fn recover_a2a_task_history_after_local_commit_with_evidence(
    previous: &RecoveredA2aTaskHistory,
    operation: &IssuedLocalOperation,
    checkpoint: &ControlStateCheckpointV1,
    signing: &SigningAuthorityBindingV1,
    authority: &mut ProvingAuthority,
    store: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    verify_fixture_bytes: bool,
) -> RecoveredA2aTaskHistory {
    let retained = RetainedRemoteFixture {
        task_binding_id: previous.binding().1.task_binding_id().clone(),
        route_ref: previous.route().0.clone(),
        route: previous.route().1.clone(),
        binding_ref: previous.binding().0.clone(),
        observation_ref: previous.observation().0.clone(),
        checkpoint_ref: previous
            .local_observation()
            .selected_head_checkpoint_record()
            .0
            .clone(),
    };
    let mut tasks = ResidentOperationTaskState::new(
        ResidentOperationTaskLimits::new(nonzero(1)),
        nonzero(1),
        ResidentAggregateTaskLimits::new(nonzero(2)),
    )
    .expect("bounded recovery task state");
    let mut service = ResidentOperationService::new(authority, store, anchor, &mut tasks);
    recover_task_history(
        &mut service,
        operation,
        &retained,
        checkpoint,
        signing,
        A2aTaskObservationSelection::SelectedHead,
        verify_fixture_bytes,
    )
}

/// Recover Task history from custody created by the production A2A send path.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn recover_retained_a2a_task_history(
    task: &crate::resident::CompletedA2aInitialTaskCustody,
    route: (&ResidentRecordRefV1, &A2aRouteAdmissionV1),
    attempt: &OperationAttemptHistoryInput<'_>,
    signing: &SigningAuthorityBindingV1,
    authority: &mut ProvingAuthority,
    store: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> RecoveredA2aTaskHistory {
    let retained = RetainedRemoteFixture {
        task_binding_id: task.binding().1.task_binding_id().clone(),
        route_ref: route.0.clone(),
        route: route.1.clone(),
        binding_ref: task.binding().0.clone(),
        observation_ref: task.observation().0.clone(),
        checkpoint_ref: task.checkpoint().0.clone(),
    };
    let mut tasks = ResidentOperationTaskState::new(
        ResidentOperationTaskLimits::new(nonzero(1)),
        nonzero(1),
        ResidentAggregateTaskLimits::new(nonzero(2)),
    )
    .expect("bounded recovery task state");
    let mut service = ResidentOperationService::new(authority, store, anchor, &mut tasks);
    recover_task_history(
        &mut service,
        attempt.operation,
        &retained,
        task.checkpoint().1,
        signing,
        A2aTaskObservationSelection::SelectedHead,
        false,
    )
}

enum Step {
    Pending,
    Task,
    ResponseFailure,
    Eof,
}

struct Exchange {
    coordinate: pale_ports::A2aExchangeCoordinate,
    request_id: String,
    cancellations: Arc<AtomicUsize>,
    steps: VecDeque<Step>,
}

impl A2aExchange for Exchange {
    fn coordinate(&self) -> &pale_ports::A2aExchangeCoordinate {
        &self.coordinate
    }

    fn poll_next(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<A2aExchangeEvidence, A2aExchangeError>>> {
        match self.steps.pop_front().unwrap_or(Step::Eof) {
            Step::Pending => Poll::Pending,
            Step::Task => Poll::Ready(Some(Ok(A2aExchangeEvidence {
                facts: A2aExchangeFacts::Task {
                    task: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
                    context: A2aRemoteContextIdV1::parse("context-1").unwrap(),
                    state: A2aRemoteTaskStateV1::Completed,
                },
                bytes: format!(
                    r#"{{"jsonrpc":"2.0","id":"{}","result":{{"id":"task-1","contextId":"context-1","status":{{"state":"TASK_STATE_COMPLETED"}}}}}}"#,
                    self.request_id,
                )
                .into_bytes(),
                sse: None,
            }))),
            Step::ResponseFailure => Poll::Ready(Some(Err(A2aExchangeError::Timeout))),
            Step::Eof => Poll::Ready(None),
        }
    }

    fn cancel(&mut self) {
        self.cancellations.fetch_add(1, Ordering::SeqCst);
    }
}

pub(in crate::resident) struct GetTaskPort {
    prepares: Cell<usize>,
    starts: Cell<usize>,
    cancellations: Arc<AtomicUsize>,
    request_ids: RefCell<VecDeque<String>>,
    fail_start: bool,
    fail_response: bool,
}

impl Default for GetTaskPort {
    fn default() -> Self {
        Self {
            prepares: Cell::new(0),
            starts: Cell::new(0),
            cancellations: Arc::new(AtomicUsize::new(0)),
            request_ids: RefCell::new(VecDeque::new()),
            fail_start: false,
            fail_response: false,
        }
    }
}

impl GetTaskPort {
    /// Return a carrier that closes while starting the prepared request.
    pub(in crate::resident) fn start_failure() -> Self {
        Self {
            fail_start: true,
            ..Self::default()
        }
    }

    /// Return a carrier that starts successfully, then closes on response poll.
    pub(in crate::resident) fn response_failure() -> Self {
        Self {
            fail_response: true,
            ..Self::default()
        }
    }

    /// Return how many physical exchanges this fixture started.
    pub(in crate::resident) fn starts(&self) -> usize {
        self.starts.get()
    }

    /// Return how many requests this fixture prepared.
    pub(in crate::resident) fn prepares(&self) -> usize {
        self.prepares.get()
    }

    /// Return how many local response reads this fixture stopped.
    pub(in crate::resident) fn read_stops(&self) -> usize {
        self.cancellations.load(Ordering::SeqCst)
    }
}

impl A2aExchangePort for GetTaskPort {
    fn prepare(
        &self,
        request: A2aExchangeRequest,
    ) -> Result<A2aPreparedExchange, A2aExchangeError> {
        self.prepares.set(self.prepares.get() + 1);
        assert!(matches!(
            request.input(),
            A2aExchangeInput::GetTask { task, context }
                if task.as_str() == "task-1" && context.as_str() == "context-1"
        ));
        let request_id = request.coordinate().request_id().as_str().to_owned();
        self.request_ids.borrow_mut().push_back(request_id.clone());
        A2aPreparedExchange::from_encoded(
            request,
            format!(
                r#"{{"jsonrpc":"2.0","id":"{request_id}","method":"GetTask","params":{{"id":"task-1","historyLength":0}}}}"#,
            )
            .into_bytes(),
            ArtifactDigest::hash_bytes(b"fixture TLS trust"),
        )
    }

    fn start(
        &self,
        prepared: A2aPreparedExchange,
    ) -> Result<Box<dyn A2aExchange>, A2aExchangeError> {
        self.starts.set(self.starts.get() + 1);
        if self.fail_start {
            return Err(A2aExchangeError::Connection);
        }
        let request_id = self
            .request_ids
            .borrow_mut()
            .pop_front()
            .expect("prepared request identity");
        Ok(Box::new(Exchange {
            coordinate: prepared.coordinate().clone(),
            request_id,
            cancellations: Arc::clone(&self.cancellations),
            steps: if self.fail_response {
                [Step::Pending, Step::ResponseFailure].into_iter().collect()
            } else {
                [Step::Pending, Step::Task, Step::Pending, Step::Eof]
                    .into_iter()
                    .collect()
            },
        }))
    }
}

pub(in crate::resident) struct Clock(UnixTimeV1);

impl Clock {
    pub(in crate::resident) fn at(seconds: u64) -> Self {
        Self(UnixTimeV1::new(seconds).expect("fixture time"))
    }
}

impl WallClockSourcePort for Clock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(self.0)
    }
}

pub(in crate::resident) fn poll_context() -> Context<'static> {
    struct ThreadWake;
    impl Wake for ThreadWake {
        fn wake(self: std::sync::Arc<Self>) {}
    }
    let waker = Box::leak(Box::new(Waker::from(std::sync::Arc::new(ThreadWake))));
    Context::from_waker(waker)
}

/// Build one admitted remote Task history, prove the lower happy journey, then
/// lend that same retained history to a higher-owner hostile test.
#[derive(Default)]
pub(in crate::resident) struct A2aTaskHistoryFixture {
    pub(in crate::resident) prove_lower_happy_journey: bool,
    pub(in crate::resident) expected_callback_appends: usize,
    pub(in crate::resident) additional_read_backs: usize,
    pub(in crate::resident) additional_anchor_advances: usize,
    pub(in crate::resident) unknown_exact_commits: Vec<usize>,
    pub(in crate::resident) unavailable_read_backs: Vec<usize>,
    pub(in crate::resident) unavailable_advances: Vec<usize>,
}

pub(in crate::resident) fn with_recovered_a2a_task_history(
    fixture: A2aTaskHistoryFixture,
    run: impl FnOnce(
        &RecoveredA2aTaskHistory,
        OperationAttemptHistoryInput<'_>,
        &SigningAuthorityBindingV1,
        &ControlStateCheckpointV1,
        &mut ProvingAuthority,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
        crate::resident::AdmittedCompiledGeneration,
    ) -> bool,
) {
    let A2aTaskHistoryFixture {
        prove_lower_happy_journey,
        expected_callback_appends,
        additional_read_backs,
        additional_anchor_advances,
        unknown_exact_commits,
        unavailable_read_backs,
        unavailable_advances,
    } = fixture;
    with_selection_recovery_context(
        LocalStartContext::SelfContained,
        |selection,
         selected,
         _,
         parents,
         initial_checkpoint,
         initial_record,
         initial_candidate,
         restart_generation| {
            let handle = selected.operation.handle();
            let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = issue_observation(
                parents.signing,
                handle.0,
                handle.1,
                schema.clone(),
                OperationObservationRevisionV1::new(1).unwrap(),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                60,
            );
            assert_eq!(
                initial_record.canonical_record_jcs().as_slice(),
                pale_core::canon::to_canonical_string(&initial)
                    .unwrap()
                    .as_bytes()
            );
            let selection_ref = record_ref("a2a/readback-selection");
            let selection_record = selection.record(selection_ref.clone(), 65_536).unwrap();
            let selection_evidence = ImmutableRecordReferenceV1 {
                record_ref: selection_ref,
                record_sha256: ArtifactDigest::parse(
                    selection_record.coordinate().record_sha256().as_str(),
                )
                .unwrap(),
            };
            let resolution_ref = record_ref("a2a/readback-resolution");
            let resolution = issue_observation(
                parents.signing,
                handle.0,
                handle.1,
                schema.clone(),
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((initial_record.coordinate().record_ref(), &initial)),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ProviderResolved {
                        provider_binding: selection.provider().unwrap().clone(),
                    },
                },
                61,
            );
            let preparation_ref = record_ref("a2a/readback-preparation");
            let preparation = issue_observation(
                parents.signing,
                handle.0,
                handle.1,
                schema,
                OperationObservationRevisionV1::new(3).unwrap(),
                Some((&resolution_ref, &resolution)),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::DispatchPrepared {
                        attempt_id: selection.attempt_id().clone(),
                        evidence: selection_evidence.clone(),
                    },
                },
                62,
            );
            let dispatch_ref = record_ref("a2a/readback-dispatch");
            let dispatch = issue_observation(
                parents.signing,
                handle.0,
                handle.1,
                schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                OperationObservationRevisionV1::new(4).unwrap(),
                Some((&preparation_ref, &preparation)),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Dispatched {
                        attempt_id: selection.attempt_id().clone(),
                        evidence: selection_evidence,
                    },
                },
                63,
            );
            let attempt = || OperationAttemptHistoryInput {
                operation: selected.operation,
                request: selected.request,
                session: selected.operation.historical_session_lease(),
                lease: selected.lease,
                generation: selected.generation,
                dispatched: &dispatch,
                selection: &selection_record,
                maximum_selection_bytes: 65_536,
            };
            let checkpoint_ref = initial_candidate
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref()
                .clone();
            let retained_records = vec![initial_record.clone(), selection_record.clone()];
            let request_id = A2aJsonRpcRequestIdV1::parse("get-readback-1").unwrap();
            let port = GetTaskPort::default();
            let lease_record = crate::resident::record_projection::record(
                crate::resident::record_projection::coordinate(
                    &record_ref("start-lease"),
                    parents.lease.operation_lease_digest().as_str(),
                )
                .unwrap(),
                parents.lease,
            )
            .unwrap();
            let local_history_appends = 3;
            let expected_appends = local_history_appends
                + 1
                + 2 * usize::from(prove_lower_happy_journey)
                + expected_callback_appends;
            let offset_faults = |faults: Vec<usize>| {
                faults
                    .into_iter()
                    .map(|ordinal| ordinal + local_history_appends)
                    .collect::<Vec<_>>()
            };
            let unknown_exact_commits = offset_faults(unknown_exact_commits);
            let unavailable_read_backs = offset_faults(unavailable_read_backs);
            let unavailable_advances = offset_faults(unavailable_advances);
            custody::with_service_custody_faults(
                initial_checkpoint,
                Some(initial_candidate),
                Vec::new(),
                Some(lease_record),
                retained_records,
                expected_appends,
                expected_appends,
                expected_appends + additional_read_backs,
                expected_appends + additional_anchor_advances,
                expected_appends,
                &[],
                &unknown_exact_commits,
                &unavailable_read_backs,
                &unavailable_advances,
                &[],
                &[],
                Vec::new(),
                false,
                None,
                Default::default(),
                |store, anchor| {
                    let resolution_checkpoint = retain_local_observation(
                        LocalObservationAppend {
                            operation: selected.operation,
                            predecessor: (initial_record.coordinate().record_ref(), &initial),
                            checkpoint: (&checkpoint_ref, initial_checkpoint),
                            observation_ref: resolution_ref.clone(),
                            observation: &resolution,
                            checkpoint_ref: record_ref("a2a/readback-resolution-checkpoint"),
                        },
                        store,
                        anchor,
                    );
                    let preparation_checkpoint = retain_local_observation(
                        LocalObservationAppend {
                            operation: selected.operation,
                            predecessor: (&resolution_ref, &resolution),
                            checkpoint: (&resolution_checkpoint.0, &resolution_checkpoint.1),
                            observation_ref: preparation_ref.clone(),
                            observation: &preparation,
                            checkpoint_ref: record_ref("a2a/readback-preparation-checkpoint"),
                        },
                        store,
                        anchor,
                    );
                    let dispatch_checkpoint = retain_local_observation(
                        LocalObservationAppend {
                            operation: selected.operation,
                            predecessor: (&preparation_ref, &preparation),
                            checkpoint: (&preparation_checkpoint.0, &preparation_checkpoint.1),
                            observation_ref: dispatch_ref.clone(),
                            observation: &dispatch,
                            checkpoint_ref: record_ref("a2a/readback-dispatch-checkpoint"),
                        },
                        store,
                        anchor,
                    );
                    let (retained, retained_checkpoint) = retain_remote_history(
                        &attempt(),
                        dispatch_ref.clone(),
                        parents.signing,
                        (&dispatch_checkpoint.0, &dispatch_checkpoint.1),
                        store,
                        anchor,
                    );
                    let mut authority = ProvingAuthority;
                    let mut tasks = ResidentOperationTaskState::new(
                        ResidentOperationTaskLimits::new(nonzero(1)),
                        nonzero(1),
                        ResidentAggregateTaskLimits::new(nonzero(2)),
                    )
                    .expect("bounded task state");
                    let history =
                        {
                            let mut service = ResidentOperationService::new(
                                &mut authority,
                                store,
                                anchor,
                                &mut tasks,
                            );
                            let checkpoint_signer =
                                checkpoint_signing_authority(&retained_checkpoint);
                            let local_admission = OperationObservationRecoveryAdmission {
                        admit_checkpoint:
                            |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                                ControlStateCheckpointV1::admit_signed_wire(
                                wire,
                                retained_checkpoint.schema_sha256(),
                                &checkpoint_signer,
                                |_: &[u8],
                                 _: &ControlStateCheckpointPreimageV1,
                                 _: &ControlStateCheckpointDigest,
                                 actual: &CanonicalSignatureV1| {
                                    assert_eq!(actual, &signature());
                                    Ok(())
                                },
                            )
                            },
                        resolve_signer:
                            |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
                                Ok(parents.signing.clone())
                            },
                        admit_state: |_: &OperationObservationStateV1| Ok(()),
                        admit_rotation:
                            |_: &ResidentRecordRefV1,
                             _: &ArtifactDigest,
                             _: &ResidentRecordRefV1,
                             _: &ArtifactDigest|
                             -> Result<(), InvalidResidentRecordV1> {
                                panic!("the local fixture does not rotate authority")
                            },
                        verify_signature:
                            |_: &[u8],
                             _: &OperationObservationPreimageV1,
                             _: &OperationObservationDigest,
                             actual: &CanonicalSignatureV1| {
                                assert_eq!(actual, &signature());
                                Ok(())
                            },
                    };
                            let mut admit_route =
                                |reference: &ResidentRecordRefV1, wire: A2aRouteAdmissionWireV1| {
                                    assert_eq!(reference, &retained.route_ref);
                                    A2aRouteAdmissionV1::admit_contextual_wire(
                                        wire,
                                        &A2aRouteAdmissionContextV1 {
                                            signed: SignedResidentAdmissionContextV1 {
                                                expected_schema_sha256: retained
                                                    .route
                                                    .schema_sha256(),
                                                signing_authority: parents.signing,
                                            },
                                            provider_binding: retained.route.provider_binding(),
                                            route: retained.route.route(),
                                        },
                                        |_| Ok(()),
                                        |_, _, _, actual| {
                                            assert_eq!(actual, &signature());
                                            Ok(())
                                        },
                                    )
                                };
                            let mut resolve_binding_signer =
                                |reference: &ResidentRecordRefV1, _: &A2aTaskBindingWireV1| {
                                    assert_eq!(reference, &retained.binding_ref);
                                    Ok(parents.signing.clone())
                                };
                            let mut admit_binding_evidence =
                        |_: &A2aTaskBindingPreimageV1, evidence: &[&OperationA2aEvidence]| {
                            assert_eq!(evidence.len(), 2);
                            assert_eq!(evidence[0].bytes(), b"fixture SendMessage request");
                            assert_eq!(evidence[1].bytes(), b"fixture Task response");
                            Ok(())
                        };
                            let mut verify_binding_signature =
                                |_: &[u8],
                                 _: &A2aTaskBindingPreimageV1,
                                 _: &A2aTaskBindingDigest,
                                 actual: &CanonicalSignatureV1| {
                                    assert_eq!(actual, &signature());
                                    Ok(())
                                };
                            let mut resolve_observation_signer =
                                |reference: &ResidentRecordRefV1, _: &A2aTaskObservationWireV1| {
                                    assert_eq!(reference, &retained.observation_ref);
                                    Ok(parents.signing.clone())
                                };
                            let mut admit_observation_evidence =
                        |_: &A2aTaskObservationPreimageV1, evidence: &[&OperationA2aEvidence]| {
                            assert_eq!(evidence.len(), 1);
                            assert_eq!(evidence[0].bytes(), b"fixture Task response");
                            Ok(())
                        };
                            let mut admit_remote_rotation = |_: &ResidentRecordRefV1,
                                                             _: &ArtifactDigest,
                                                             _: &ResidentRecordRefV1,
                                                             _: &ArtifactDigest|
                             -> Result<
                                (),
                                InvalidResidentRecordV1,
                            > {
                                panic!("the remote fixture does not rotate authority")
                            };
                            let mut verify_observation_signature =
                                |_: &[u8],
                                 _: &A2aTaskObservationPreimageV1,
                                 _: &A2aTaskObservationDigest,
                                 actual: &CanonicalSignatureV1| {
                                    assert_eq!(actual, &signature());
                                    Ok(())
                                };
                            let coordinate = store_coordinate();
                            let history = service
                            .recover_a2a_task_history(
                            A2aTaskRecoveryInput {
                                store: &coordinate,
                                operation: selected.operation,
                                task_binding_id: &retained.task_binding_id,
                                observation: A2aTaskObservationSelection::SelectedHead,
                                checkpoint_authority: &checkpoint_signer,
                                local_limits: OperationObservationRecoveryLimits::new(
                                    4,
                                    1 << 20,
                                    65_536,
                                )
                                .unwrap(),
                                checkpoint_limits: OperationCheckpointAncestryLimits::new(
                                    16,
                                    1 << 20,
                                    16,
                                )
                                .unwrap(),
                                membership_limits: AccumulatorMembershipLimits::new(
                                    32, 100_000, 1_000_000,
                                )
                                .unwrap(),
                                limits: A2aTaskRecoveryLimits::new(1, 65_536, 1 << 20, 2, 8_192)
                                    .unwrap(),
                            },
                            local_admission,
                            |_: &[u8],
                             _: &ControlStateCheckpointPreimageV1,
                             _: &ControlStateCheckpointDigest,
                             actual: &CanonicalSignatureV1| {
                                assert_eq!(actual, &signature());
                                Ok(())
                            },
                            A2aTaskRecoveryAdmission {
                                admit_route: &mut admit_route,
                                resolve_binding_signer: &mut resolve_binding_signer,
                                admit_binding_evidence: &mut admit_binding_evidence,
                                verify_binding_signature: &mut verify_binding_signature,
                                resolve_observation_signer: &mut resolve_observation_signer,
                                admit_observation_evidence: &mut admit_observation_evidence,
                                admit_rotation: &mut admit_remote_rotation,
                                verify_observation_signature: &mut verify_observation_signature,
                            },
                        )
                            .expect("service must recover the retained A2A history");
                            assert_eq!(history.route().0, &retained.route_ref);
                            assert_eq!(history.binding().0, &retained.binding_ref);
                            assert_eq!(history.observation().0, &retained.observation_ref);
                            history
                        };
                    if run(
                        &history,
                        attempt(),
                        parents.signing,
                        initial_checkpoint,
                        &mut authority,
                        store,
                        anchor,
                        parents,
                        restart_generation,
                    ) {
                        assert!(!prove_lower_happy_journey);
                        return;
                    }
                    assert!(prove_lower_happy_journey);
                    let (awaiting, read_key, mut cx) = {
                        let mut service = ResidentOperationService::new(
                            &mut authority,
                            store,
                            anchor,
                            &mut tasks,
                        );
                        let substituted = service.start_a2a_get_task_readback(
                            A2aGetTaskReadbackInput {
                                history: &history,
                                attempt: attempt(),
                                request_id: &request_id,
                                request_record_ref: history.binding().0.clone(),
                                response_record_ref: record_ref("a2a/substituted-response"),
                                limits: exchange_limits(),
                                maximum_evidence_bytes: 8_192,
                            },
                            &port,
                            &Clock(UnixTimeV1::new(65).unwrap()),
                            |_| panic!("substituted identity must fail before authorization"),
                        );
                        assert!(matches!(
                            substituted,
                            Err(crate::resident::A2aGetTaskReadbackError::Association)
                        ));
                        assert_eq!(port.prepares.get(), 0);
                        assert_eq!(port.starts.get(), 0);
                        let read_input = A2aGetTaskReadbackInput {
                            history: &history,
                            attempt: attempt(),
                            request_id: &request_id,
                            request_record_ref: record_ref("a2a/get-readback-request"),
                            response_record_ref: record_ref("a2a/get-readback-response"),
                            limits: exchange_limits(),
                            maximum_evidence_bytes: 8_192,
                        };
                        let read_key = a2a_get_task_key(&read_input);
                        if service
                            .start_owned_a2a_get_task(
                                read_key.clone(),
                                read_input,
                                &port,
                                &Clock(UnixTimeV1::new(65).unwrap()),
                                |actual| {
                                    assert_eq!(actual.binding, history.binding());
                                    assert_eq!(actual.remote_head, history.observation());
                                    Ok(())
                                },
                            )
                            .is_err()
                        {
                            panic!("exact known binding must start one owned GetTask");
                        }
                        let mut cx = poll_context();
                        assert!(matches!(
                            service.poll_owned_a2a_get_task(
                                &read_key,
                                &mut cx,
                                &Clock(UnixTimeV1::new(66).unwrap()),
                            ),
                            ResidentA2aGetTaskPoll::PendingResponse
                        ));
                        assert!(matches!(
                            service.poll_owned_a2a_get_task(
                                &read_key,
                                &mut cx,
                                &Clock(UnixTimeV1::new(66).unwrap()),
                            ),
                            ResidentA2aGetTaskPoll::ResponseReceived
                        ));
                        assert!(matches!(
                            service.poll_owned_a2a_get_task(
                                &read_key,
                                &mut cx,
                                &Clock(UnixTimeV1::new(66).unwrap()),
                            ),
                            ResidentA2aGetTaskPoll::PendingEof
                        ));
                        let awaiting = match service.poll_owned_a2a_get_task(
                            &read_key,
                            &mut cx,
                            &Clock(UnixTimeV1::new(66).unwrap()),
                        ) {
                            ResidentA2aGetTaskPoll::AwaitingCustody(awaiting) => *awaiting,
                            _ => panic!("one unary response must await durable history custody"),
                        };
                        assert_eq!(port.prepares.get(), 1);
                        assert_eq!(
                            port.starts.get(),
                            1,
                            "GetTask carrier must never be replayed"
                        );
                        (awaiting, read_key, cx)
                    };
                    let (_, cancellation, _, cancellation_checkpoint) =
                        retain_local_cancellation_head(
                            selected.operation,
                            history.local_observation().selected_head(),
                            (&retained.checkpoint_ref, &retained_checkpoint),
                            parents.signing,
                            store,
                            anchor,
                        );
                    let mut service =
                        ResidentOperationService::new(&mut authority, store, anchor, &mut tasks);
                    let fresh_history = recover_task_history(
                        &mut service,
                        selected.operation,
                        &retained,
                        &cancellation_checkpoint,
                        parents.signing,
                        A2aTaskObservationSelection::SelectedHead,
                        true,
                    );
                    assert_eq!(
                        fresh_history.local_observation().selected_head().1,
                        &cancellation
                    );
                    assert_eq!(port.starts.get(), 1);
                    let history = fresh_history;

                    let observation_schema = schema_digest(A2A_TASK_OBSERVATION_V1_SCHEMA_ID);
                    let checkpoint_authority =
                        checkpoint_signing_authority(&cancellation_checkpoint);
                    let pending_custody = match service.retain_owned_a2a_get_task(
                        awaiting,
                        A2aGetTaskCommitInput {
                            observation: A2aGetTaskObservationInput {
                                history: &history,
                                operation: selected.operation,
                                observation_ref: record_ref("a2a/get-readback-observation"),
                                signing: SignedResidentAdmissionContextV1 {
                                    expected_schema_sha256: &observation_schema,
                                    signing_authority: parents.signing,
                                },
                                maximum_evidence_bytes: 8_192,
                                maximum_record_bytes: 65_536,
                            },
                            checkpoint: OperationCheckpointInput {
                                reference: record_ref("a2a/get-readback-checkpoint"),
                                id: ControlCheckpointIdV1::parse("a2a-get-readback-checkpoint")
                                    .unwrap(),
                            },
                            issued_at: UnixTimeV1::new(67).unwrap(),
                            checkpoint_schema: cancellation_checkpoint.schema_sha256(),
                            checkpoint_authority: &checkpoint_authority,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                8, 20_000, 20_000, 8_192, 200_000,
                            )
                            .unwrap(),
                            membership_limits: AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                                .unwrap(),
                        },
                        &pale_adapter_a2a::A2aGetTaskDecoder,
                        |_, _, _, _| Ok(()),
                        |_, _, _| Ok(signature()),
                        |_| Ok(store_coordinate()),
                        |_, _, _| Ok(signature()),
                    ) {
                        Ok(pending_custody) => pending_custody,
                        Err(_) => panic!("read-back must reach database and anchor custody"),
                    };
                    let exact_release = match service
                        .reject_get_task_release_with_replacement_for_test(pending_custody)
                    {
                        ResidentA2aGetTaskReleaseFailure::Settlement {
                            key,
                            custody,
                            release,
                        } => (key, custody, release),
                        _ => panic!("one-shot aggregate refusal must preserve exact custody"),
                    };
                    let mut crossed_key = read_key.clone();
                    crossed_key.task_binding_id =
                        A2aTaskBindingIdV1::parse("readback-crossed-binding")
                            .expect("crossed binding identity");
                    crossed_key.task_binding_ref = record_ref("a2a/readback-crossed-binding");
                    crossed_key.task_binding_sha256 = A2aTaskBindingDigest::parse(
                        ArtifactDigest::hash_bytes(b"readback crossed binding").to_string(),
                    )
                    .expect("crossed binding digest");
                    let crossed_release = service.get_task_release_failure_for_test(crossed_key);
                    assert_eq!(service.outstanding_get_task_work_for_test(), 2);

                    let crossed = ResidentA2aGetTaskReleaseFailure::Settlement {
                        key: exact_release.0,
                        custody: exact_release.1,
                        release: Box::new(crossed_release),
                    };
                    let (key, custody, crossed_release) =
                        match service.retry_owned_a2a_get_task_release(crossed) {
                            Err(ResidentA2aGetTaskReleaseFailure::Settlement {
                                key,
                                custody,
                                release,
                            }) => (key, custody, release),
                            _ => panic!("a crossed settlement must preserve both reservations"),
                        };
                    assert_eq!(service.outstanding_get_task_work_for_test(), 2);
                    assert_eq!(port.starts.get(), 1);

                    let exact = ResidentA2aGetTaskReleaseFailure::Settlement {
                        key,
                        custody,
                        release: exact_release.2,
                    };
                    let completed = match service.retry_owned_a2a_get_task_release(exact) {
                        Ok(completed) => completed,
                        Err(_) => panic!("exact settlement must release its aggregate capacity"),
                    };
                    assert_eq!(service.outstanding_get_task_work_for_test(), 1);
                    service.retry_get_task_release_for_test(*crossed_release);
                    assert_eq!(service.outstanding_get_task_work_for_test(), 0);
                    assert_eq!(completed.binding(), history.binding());
                    assert_eq!(
                        completed.observation().1.remote_state(),
                        &A2aRemoteTaskStateV1::Completed
                    );
                    assert!(matches!(
                        completed.observation().1.source(),
                        A2aTaskObservationSourceV1::GetTask { request, response }
                            if request.record_ref.as_str() == "a2a/get-readback-request"
                                && response.record_ref.as_str() == "a2a/get-readback-response"
                    ));
                    assert!(completed
                        .custody()
                        .candidate()
                        .record(OperationCustodyRecordKind::OperationObservation)
                        .is_none());
                    assert!(matches!(
                        history.local_observation().selected_head().1.state(),
                        OperationObservationStateV1::CancellationRequested { .. }
                    ));

                    let historical = recover_task_history(
                        &mut service,
                        selected.operation,
                        &retained,
                        completed.checkpoint().1,
                        parents.signing,
                        A2aTaskObservationSelection::HistoryMember {
                            reference: history.observation().0,
                            digest: history
                                .observation()
                                .1
                                .canonical_a2a_task_observation_sha256(),
                        },
                        true,
                    );
                    assert_ne!(historical.selected_observation(), historical.observation());
                    let selected_head_prepares = port.prepares.get();
                    let selected_head_starts = port.starts.get();
                    let historical_request_id =
                        A2aJsonRpcRequestIdV1::parse("get-readback-historical").unwrap();
                    let historical_start = service.start_a2a_get_task_readback(
                        A2aGetTaskReadbackInput {
                            history: &historical,
                            attempt: attempt(),
                            request_id: &historical_request_id,
                            request_record_ref: record_ref("a2a/get-historical-request"),
                            response_record_ref: record_ref("a2a/get-historical-response"),
                            limits: exchange_limits(),
                            maximum_evidence_bytes: 8_192,
                        },
                        &port,
                        &Clock(UnixTimeV1::new(68).unwrap()),
                        |_| panic!("a historical remote member must fail before authorization"),
                    );
                    assert!(matches!(
                        historical_start,
                        Err(crate::resident::A2aGetTaskReadbackError::Association)
                    ));
                    assert_eq!(port.prepares.get(), selected_head_prepares);
                    assert_eq!(port.starts.get(), selected_head_starts);

                    let stop_before_response_id =
                        A2aJsonRpcRequestIdV1::parse("get-readback-stop-response").unwrap();
                    let stop_before_response = A2aGetTaskReadbackInput {
                        history: &history,
                        attempt: attempt(),
                        request_id: &stop_before_response_id,
                        request_record_ref: record_ref("a2a/get-stop-response-request"),
                        response_record_ref: record_ref("a2a/get-stop-response-response"),
                        limits: exchange_limits(),
                        maximum_evidence_bytes: 8_192,
                    };
                    let stop_before_response_key = a2a_get_task_key(&stop_before_response);
                    assert!(service
                        .start_owned_a2a_get_task(
                            stop_before_response_key.clone(),
                            stop_before_response,
                            &port,
                            &Clock(UnixTimeV1::new(67).unwrap()),
                            |_| Ok(()),
                        )
                        .is_ok());
                    let stopped = match service
                        .stop_owned_a2a_get_task_response_read(&stop_before_response_key)
                    {
                        ResidentA2aGetTaskPoll::ReadStopped(stopped) => *stopped,
                        _ => panic!("an awaiting-response read must stop locally"),
                    };
                    assert!(service
                        .finish_stopped_a2a_get_task_response_read(stopped)
                        .is_ok());

                    let stop_before_eof_id =
                        A2aJsonRpcRequestIdV1::parse("get-readback-stop-eof").unwrap();
                    let stop_before_eof = A2aGetTaskReadbackInput {
                        history: &history,
                        attempt: attempt(),
                        request_id: &stop_before_eof_id,
                        request_record_ref: record_ref("a2a/get-stop-eof-request"),
                        response_record_ref: record_ref("a2a/get-stop-eof-response"),
                        limits: exchange_limits(),
                        maximum_evidence_bytes: 8_192,
                    };
                    let stop_before_eof_key = a2a_get_task_key(&stop_before_eof);
                    assert!(service
                        .start_owned_a2a_get_task(
                            stop_before_eof_key.clone(),
                            stop_before_eof,
                            &port,
                            &Clock(UnixTimeV1::new(67).unwrap()),
                            |_| Ok(()),
                        )
                        .is_ok());
                    assert!(matches!(
                        service.poll_owned_a2a_get_task(
                            &stop_before_eof_key,
                            &mut cx,
                            &Clock(UnixTimeV1::new(68).unwrap()),
                        ),
                        ResidentA2aGetTaskPoll::PendingResponse
                    ));
                    assert!(matches!(
                        service.poll_owned_a2a_get_task(
                            &stop_before_eof_key,
                            &mut cx,
                            &Clock(UnixTimeV1::new(68).unwrap()),
                        ),
                        ResidentA2aGetTaskPoll::ResponseReceived
                    ));
                    let stopped =
                        match service.stop_owned_a2a_get_task_response_read(&stop_before_eof_key) {
                            ResidentA2aGetTaskPoll::ReadStopped(stopped) => *stopped,
                            _ => panic!("an awaiting-EOF read must stop locally"),
                        };
                    assert!(service
                        .finish_stopped_a2a_get_task_response_read(stopped)
                        .is_ok());
                    assert_eq!(port.starts.get(), 3);
                    assert_eq!(port.cancellations.load(Ordering::SeqCst), 2);
                },
            );
        },
    );
}

#[test]
fn service_reads_one_exact_binding_and_retains_remote_history_without_local_promotion() {
    with_recovered_a2a_task_history(
        A2aTaskHistoryFixture {
            prove_lower_happy_journey: true,
            ..A2aTaskHistoryFixture::default()
        },
        |_, _, _, _, _, _, _, _, _| false,
    );
}
