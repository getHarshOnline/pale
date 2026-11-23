//! Read-only recovery laws for one retained A2A send.

use super::*;
use crate::local_operation_start_fixture::signature;
use crate::resident::process_host::tests::a2a_cancel_task::checkpoint_signer;
use crate::resident::process_host::tests::a2a_delegate::with_retained_process_a2a_task;
use pale_core::records::{
    A2aRouteAdmissionContextV1, A2aRouteAdmissionV1, A2aTaskBindingAdmissionContextV1,
    A2aTaskBindingPreimageV1, A2aTaskBindingV1, A2aTaskRequestModeV1, ControlStateCheckpointV1,
    ResidentRecordRefV1, SignedResidentAdmissionContextV1,
};

fn binding_preimage_with_request_mode(
    binding: &A2aTaskBindingV1,
    request_mode: A2aTaskRequestModeV1,
) -> A2aTaskBindingPreimageV1 {
    A2aTaskBindingPreimageV1 {
        schema_id: binding.schema_id().clone(),
        schema_sha256: binding.schema_sha256().clone(),
        task_binding_id: binding.task_binding_id().clone(),
        operation_handle_id: binding.operation_handle_id().clone(),
        operation_handle_ref: binding.operation_handle_ref().clone(),
        operation_handle_sha256: binding.operation_handle_sha256().clone(),
        operation_observation_ref: binding.operation_observation_ref().clone(),
        operation_observation_sha256: binding.operation_observation_sha256().clone(),
        route_admission_id: binding.route_admission_id().clone(),
        route_admission_ref: binding.route_admission_ref().clone(),
        route_admission_sha256: binding.route_admission_sha256().clone(),
        attempt_id: binding.attempt_id().clone(),
        json_rpc_request_id: binding.json_rpc_request_id().clone(),
        request_mode,
        remote_task_id: binding.remote_task_id().clone(),
        remote_context_id: binding.remote_context_id().clone(),
        request_evidence: binding.request_evidence().clone(),
        initial_task_evidence: binding.initial_task_evidence().clone(),
        bound_at: *binding.bound_at(),
        deployment_runtime_authority_ref: binding.deployment_runtime_authority_ref().clone(),
        deployment_runtime_authority_sha256: binding.deployment_runtime_authority_sha256().clone(),
        issuer_principal_id: binding.issuer_principal_id().clone(),
        issuer_key_id: binding.issuer_key_id().clone(),
        issuer_key_epoch: *binding.issuer_key_epoch(),
    }
}

#[test]
fn recovered_send_rejects_a_binding_from_another_request_mode() {
    with_retained_process_a2a_task(|fixture| {
        let checkpoint_authority = checkpoint_signer(&fixture.checkpoint);
        let route = fixture.history.route();
        let operation = fixture.attempt.operation;
        let recovered = fixture
            .provider_recovery
            .recovered((fixture.checkpoint_ref.clone(), fixture.checkpoint.clone()));
        let mut admit_checkpoint = |_: &ResidentRecordRefV1, wire| {
            ControlStateCheckpointV1::admit_signed_wire(
                wire,
                fixture.checkpoint.schema_sha256(),
                &checkpoint_authority,
                |_, _, _, actual| {
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            )
        };
        let mut admit_route = |reference: &ResidentRecordRefV1, wire| {
            assert_eq!(reference, route.0);
            A2aRouteAdmissionV1::admit_contextual_wire(
                wire,
                &A2aRouteAdmissionContextV1 {
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: route.1.schema_sha256(),
                        signing_authority: fixture.signing,
                    },
                    provider_binding: route.1.provider_binding(),
                    route: route.1.route(),
                },
                |_| Ok(()),
                |_, _, _, actual| {
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            )
        };
        let send = recover_a2a_send_preparation(
            A2aSendRecoveryInput {
                history: fixture.provider_recovery.history(),
                recovered: &recovered,
                limits: A2aSendRecoveryLimits::new(
                    1 << 20,
                    8_192,
                    OperationCheckpointAncestryLimits::new(16, 1 << 20, 16).unwrap(),
                    AccumulatorMembershipLimits::new(64, 100_000, 1_000_000).unwrap(),
                )
                .unwrap(),
            },
            fixture.custody,
            fixture.anchor,
            A2aSendRecoveryAdmission {
                admit_checkpoint: &mut admit_checkpoint,
                admit_route: &mut admit_route,
            },
            A2aSendRecoveryHead::Dispatched,
        )
        .expect("retained SendMessage preparation");
        assert!(send.matches_binding(fixture.history.binding().1));

        let preimage = binding_preimage_with_request_mode(
            fixture.history.binding().1,
            A2aTaskRequestModeV1::SendStreamingMessage,
        );
        let unlike = A2aTaskBindingV1::issue_contextual(
            preimage,
            &A2aTaskBindingAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: fixture.history.binding().1.schema_sha256(),
                    signing_authority: fixture.signing,
                },
                operation_handle_ref: operation.handle().0,
                operation_handle: operation.handle().1,
                operation_observation_ref: fixture.history.dispatch().0,
                operation_observation: fixture.history.dispatch().1,
                route_admission_ref: route.0,
                route_admission: route.1,
            },
            |_| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .expect("context-issued binding from another request mode");
        assert!(!send.matches_binding(&unlike));
    });
}
