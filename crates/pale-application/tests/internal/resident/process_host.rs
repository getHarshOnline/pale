//! Hostile laws for private multi-deployment resident ownership.

use super::*;

impl ResidentDeploymentAuthoritySelector {
    fn with_test_runtime_authority_ref(&self, runtime_authority_ref: ResidentRecordRefV1) -> Self {
        let mut selector = self.clone();
        selector.runtime_authority_ref = runtime_authority_ref;
        selector
    }
}

#[path = "process_host/consultation.rs"]
mod consultation;

#[path = "process_host/a2a_delegate.rs"]
pub(in crate::resident) mod a2a_delegate;

#[path = "process_host/a2a_get_task.rs"]
mod a2a_get_task;

#[path = "process_host/a2a_cancel_task.rs"]
pub(in crate::resident) mod a2a_cancel_task;

#[path = "process_host/a2a_cancel_task_restart.rs"]
mod a2a_cancel_task_restart;

#[path = "process_host/a2a_send_restart.rs"]
mod a2a_send_restart;

#[path = "process_host/provider.rs"]
mod provider;

#[path = "process_host/consult_provider.rs"]
mod consult_provider;

#[path = "process_host/consult_child_fixture.rs"]
pub(super) mod consult_child_fixture;

#[path = "process_host/publication.rs"]
pub(in crate::resident::process_host) mod publication;

#[path = "process_host/start.rs"]
pub(in crate::resident) mod start;

#[path = "process_host/allocation.rs"]
mod allocation;

#[path = "process_host/invoke_request.rs"]
mod invoke_request;

#[path = "process_host/operation_resolution.rs"]
mod operation_resolution;

#[path = "process_host/operation_locator.rs"]
mod operation_locator;

#[path = "process_host/program.rs"]
mod program;

#[path = "process_host/current_request_admission.rs"]
mod current_request_admission;

#[path = "process_host/program_current_request.rs"]
mod program_current_request;

#[path = "process_host/public_service_assembly.rs"]
mod public_service_assembly;

#[path = "process_host/native_invoke.rs"]
mod native_invoke;

#[path = "process_host/observe_request.rs"]
mod observe_request;

#[path = "process_host/provider_recovery.rs"]
pub(in crate::resident) mod provider_recovery;

#[path = "process_host/provider_resolution_recovery.rs"]
mod provider_resolution_recovery;

#[path = "process_host/restart.rs"]
pub(in crate::resident) mod restart;

#[path = "process_host/operation_recovery_sweep.rs"]
mod operation_recovery_sweep;

#[path = "process_host/consult_recovery.rs"]
mod consult_recovery;

#[path = "process_host/consult_child_recovery_sweep.rs"]
mod consult_child_recovery_sweep;

#[path = "process_host/consult_child_recovery_sweep_hostile.rs"]
pub(in crate::resident) mod consult_child_recovery_sweep_hostile;

#[path = "process_host/consult_completion.rs"]
mod consult_completion;

#[path = "process_host/dispatch_prepared_recovery.rs"]
pub(in crate::resident::process_host) mod dispatch_prepared_recovery;

#[path = "process_host/cancellation_recovery.rs"]
mod cancellation_recovery;
#[path = "process_host/dispatched_cancellation.rs"]
mod dispatched_cancellation;

#[path = "process_host/expiry.rs"]
mod expiry;

#[path = "process_host/reconciliation_recovery.rs"]
mod reconciliation_recovery;

use std::collections::BTreeSet;
use std::num::NonZeroUsize;
use std::time::Instant;

use pale_core::records::{
    ArtifactDigest, CanonicalSignatureV1, CommonRequestEnvelopeV1, CommonRequestEnvelopeWireV1,
    ControlCheckpointSigningAuthorityAdmissionContextV1, ControlIncarnationIdV1,
    ControlStateCheckpointDigest, ControlStateCheckpointPreimageV1, ControlStateCheckpointV1,
    ControlStateCheckpointWireV1, ControlStoreIdV1, DeploymentBindingDigest,
    DeploymentRuntimeAuthorityDigest, IntegratedRuntimeAuthorityV1, InvalidResidentRecordV1,
    OperationHandleDigest, OperationHandlePreimageV1, OperationHandleV1,
    OperationObservationDigest, OperationObservationPreimageV1, OperationObservationStateV1,
    OperationObservationV1, OperationObservationWireV1, OwnerComponentIdV1, ResidentKeyIdV1,
    ResidentPrincipalIdV1, ResidentRecordRefV1, SchemaDigest, SchemaId,
    SignedResidentAdmissionContextV1, SigningAuthorityBindingV1, SigningKeyEpochV1,
    StandaloneOwnerNativeAuthorityV1, WakeEpochV1,
};
use pale_ports::{
    AuthorityStoreCoordinate, LocalIpcCarrierError, LocalIpcFramePort, LocalIpcPeerCredentials,
    OperationCustodyCommitOutcome, ResidentObserveFrameRequest, ResidentOperationFrameLocator,
    ResidentOperationFrameRequest,
};

use super::restart::ResidentProcessOperationLineage;
use crate::local_operation_start_fixture;
use crate::resident::consultation::tests::consultation_task_batch_fixture;
use crate::resident::consultation::{
    ResidentConsultationBatchOpenError, ResidentConsultationTaskLimits,
};
use crate::resident::journal::tests::event_custody::UnavailableEventCustody;
use crate::resident::journal::tests::{limits, MemoryAuthorityStore};

#[derive(Default)]
pub(super) struct ProgramCancellationRetentionProbe {
    pub(super) cancellation:
        Option<Box<crate::resident::process_host::ResidentProcessCancellationCustody>>,
    pub(super) dispatched:
        Option<Box<crate::resident::process_host::ResidentProcessDispatchedCancellationCustody>>,
    pub(super) consultation:
        Option<Box<crate::resident::process_host::ResidentProcessConsultationCancellationCustody>>,
    pub(super) consultation_preparation: Option<
        Box<crate::resident::process_host::ResidentProcessConsultationCancellationPreparation>,
    >,
    pub(super) a2a: Option<crate::resident::process_host::ResidentProcessA2aDelegatePoll>,
    pub(super) retention_calls: usize,
    pub(super) release_calls: usize,
}

impl crate::resident::process_host::program::ResidentProgramCancellationInputPort
    for ProgramCancellationRetentionProbe
{
    type Reservation = ();

    fn reserve_cancellation(
        &mut self,
        _: &crate::resident::process_host::ResidentDeploymentAuthoritySelector,
        _: &CommonRequestEnvelopeV1,
        _: &pale_ports::ControlRecordCoordinate,
    ) -> Result<Self::Reservation, pale_ports::ResidentOperationFrameFailure> {
        Ok(())
    }

    fn release_cancellation(&mut self, _: Self::Reservation) {
        self.release_calls += 1;
    }

    fn retain_dispatched_cancellation(
        &mut self,
        _: Self::Reservation,
        pending: Box<crate::resident::process_host::ResidentProcessDispatchedCancellationCustody>,
    ) {
        assert!(self.dispatched.replace(pending).is_none());
        self.retention_calls += 1;
    }

    fn retain_cancellation_custody(
        &mut self,
        _: Self::Reservation,
        pending: Box<crate::resident::process_host::ResidentProcessCancellationCustody>,
    ) {
        assert!(self.cancellation.replace(pending).is_none());
        self.retention_calls += 1;
    }

    fn retain_a2a_cancellation(
        &mut self,
        _: Self::Reservation,
        pending: crate::resident::process_host::ResidentProcessA2aDelegatePoll,
    ) {
        assert!(self.a2a.replace(pending).is_none());
        self.retention_calls += 1;
    }

    fn retain_consultation_cancellation(
        &mut self,
        _: Self::Reservation,
        pending: Box<crate::resident::process_host::ResidentProcessConsultationCancellationCustody>,
    ) {
        assert!(self.consultation.replace(pending).is_none());
        self.retention_calls += 1;
    }

    fn retain_consultation_cancellation_preparation(
        &mut self,
        _: Self::Reservation,
        pending: Box<
            crate::resident::process_host::ResidentProcessConsultationCancellationPreparation,
        >,
    ) {
        assert!(self.consultation_preparation.replace(pending).is_none());
        self.retention_calls += 1;
    }
}
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_denied_process_host_cancellation_fixture, with_denied_process_host_observe_fixture,
    with_process_host_active_cancellation_fixture,
    with_process_host_before_dispatch_consultation_cancellation_fixture,
    with_process_host_cancellation_dispositions,
    with_process_host_collecting_cancellation_anchor_retry_fixture,
    with_process_host_collecting_cancellation_database_retry_fixture,
    with_process_host_collecting_cancellation_fixture,
    with_process_host_collecting_cancellation_preparation_failure_fixture,
    with_process_host_consultation_cancellation_dispositions, with_process_host_observe_fixture,
    with_process_host_root_consult_policy_fixture, ProcessHostCancellationFixture,
    ProcessHostObserveFixture,
};
use crate::resident::operation_observation_custody::tests::{RecordingAnchor, RecordingStore};
use crate::resident::operation_service::{
    ResidentLocalCancellationCompletionInput, ResidentOperationCancellationAdmission,
    ResidentOperationCancellationInput, ResidentOperationCancellationOutcome,
};
use crate::resident::operation_service::{ResidentOperationService, ResidentOperationTaskState};
use crate::resident::reconstruction::ResidentReconstructionParameters;
use crate::resident::task_owner::ResidentOperationTaskLimits;
use crate::resident::task_reservation::ResidentAggregateTaskLimits;
use crate::resident::{
    LocalOperationHandleRecoveryInput, LocalOperationRecoveryContext, OperationCheckpointInput,
    OperationControlAuthorizationContext, OperationControlRecoveryAdmission,
    OperationControlRecoveryInput,
};
use crate::resident::{RecoveryReferenceKnowledge, ResidentColdOpenInput, ResidentColdOpenOutcome};
use crate::resident_fixture::{synthetic_set_policy, FixedInputSource, RecoveryResolver};
use crate::session_admission_fixture::ProvingAuthority;

#[derive(Debug)]
pub(in crate::resident) struct UnprovedHeldWriter;

#[derive(Clone, Copy)]
struct ExactTestPeer(LocalIpcPeerCredentials);

impl ExactTestPeer {
    const fn new(credentials: LocalIpcPeerCredentials) -> Self {
        Self(credentials)
    }
}

impl crate::resident::ResidentPeerAdmissionPort for ExactTestPeer {
    fn admit_peer(
        &mut self,
        credentials: LocalIpcPeerCredentials,
    ) -> Result<crate::resident::ResidentPeerAdmissionWitness, InvalidResidentRecordV1> {
        if credentials != self.0 {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "resident local peer",
                invariant: "kernel peer coordinates do not match the exact test peer",
            });
        }
        Ok(crate::resident::ResidentPeerAdmissionWitness::new(
            credentials,
        ))
    }
}

#[test]
fn exact_test_peer_rejects_another_process_with_the_same_user_and_group() {
    let expected = LocalIpcPeerCredentials::new(501, 20, Some(8_081));
    let other_process = LocalIpcPeerCredentials::new(501, 20, Some(8_082));
    let mut admission = ExactTestPeer::new(expected);

    assert!(
        crate::resident::ResidentPeerAdmissionPort::admit_peer(&mut admission, expected).is_ok()
    );
    assert!(
        crate::resident::ResidentPeerAdmissionPort::admit_peer(&mut admission, other_process)
            .is_err()
    );
}

/// One exact inbound operation frame used by real process-host composition laws.
struct OperationFramePort {
    peer: LocalIpcPeerCredentials,
    inbound: Option<Box<[u8]>>,
    sent: Vec<u8>,
}

impl OperationFramePort {
    fn new(request: &impl serde::Serialize) -> Self {
        Self::for_peer(request, LocalIpcPeerCredentials::new(501, 20, Some(8_081)))
    }

    fn for_peer(request: &impl serde::Serialize, peer: LocalIpcPeerCredentials) -> Self {
        Self {
            peer,
            inbound: Some(
                pale_core::canon::to_canonical_string(request)
                    .expect("resident frame serializes")
                    .into_bytes()
                    .into_boxed_slice(),
            ),
            sent: Vec::new(),
        }
    }
}

impl LocalIpcFramePort for OperationFramePort {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        Ok(self.peer)
    }

    fn send_frame(&mut self, payload: &[u8], _: Instant) -> Result<(), LocalIpcCarrierError> {
        self.sent.extend_from_slice(payload);
        Ok(())
    }

    fn receive_frame(&mut self, _: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        Ok(self.inbound.take().expect("one inbound operation frame"))
    }
}

fn operation_frame_locator(
    operation: &crate::resident::IssuedLocalOperation,
) -> ResidentOperationFrameLocator {
    ResidentOperationFrameLocator::new(
        operation.handle().0.as_str().to_owned(),
        operation
            .handle()
            .1
            .canonical_operation_handle_sha256()
            .as_str()
            .to_owned(),
    )
}

fn principal(value: &str) -> ResidentPrincipalIdV1 {
    ResidentPrincipalIdV1::parse(value).expect("principal identity")
}

fn store(store_id: &str, incarnation_id: &str) -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse(store_id).expect("store identity"),
        ControlIncarnationIdV1::parse(incarnation_id).expect("store incarnation"),
    )
}

pub(in crate::resident::process_host) fn ready_application<'store>(
    port: &'store MemoryAuthorityStore,
    store: AuthorityStoreCoordinate,
    wake_epoch: u64,
) -> Box<ResidentApplication<'store>> {
    let resolver = RecoveryResolver::default();
    let mut source = FixedInputSource::default();
    let opened = ResidentApplication::cold_open(
        port,
        port,
        &mut UnavailableEventCustody,
        limits().maximum_entry_bytes(),
        ResidentColdOpenInput::new(
            store,
            limits(),
            limits().maximum_entry_bytes(),
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            &resolver,
            &mut source,
            ResidentReconstructionParameters::new(
                synthetic_set_policy(),
                4,
                4,
                WakeEpochV1::new(wake_epoch),
            ),
        ),
    )
    .expect("empty resident cold-open");
    let ResidentColdOpenOutcome::Ready(application) = opened else {
        panic!("empty resident must be ready");
    };
    application
}

pub(in crate::resident::process_host) fn task_state() -> ResidentOperationTaskState {
    task_state_with_provider_capacity(1)
}

fn task_state_with_provider_capacity(maximum_provider_tasks: usize) -> ResidentOperationTaskState {
    ResidentOperationTaskState::new(
        ResidentOperationTaskLimits::new(
            NonZeroUsize::new(maximum_provider_tasks).expect("positive provider-task bound"),
        ),
        NonZeroUsize::new(1).expect("positive consultation-batch bound"),
        ResidentAggregateTaskLimits::new(
            NonZeroUsize::new(maximum_provider_tasks).expect("positive aggregate-task bound"),
        ),
    )
    .expect("bounded operation task state")
}

fn consultation_limits() -> ResidentConsultationTaskLimits {
    ResidentConsultationTaskLimits::new(
        NonZeroUsize::new(1).expect("positive in-flight child bound"),
        NonZeroUsize::new(1).expect("positive batch-member bound"),
    )
}

fn operation_ports() -> (ProvingAuthority, RecordingStore, RecordingAnchor) {
    (
        ProvingAuthority,
        RecordingStore::committing(OperationCustodyCommitOutcome::Committed),
        RecordingAnchor::unavailable_after_preflight(),
    )
}

pub(in crate::resident::process_host) fn unproved_writer(
    principal_id: ResidentPrincipalIdV1,
    mut opened_deployment_stores: Vec<AuthorityStoreCoordinate>,
) -> ReadyPrincipalResidentWriter<UnprovedHeldWriter> {
    opened_deployment_stores.sort();
    ReadyPrincipalResidentWriter {
        principal_id,
        opened_deployment_stores: opened_deployment_stores.into_boxed_slice(),
        held_writer: UnprovedHeldWriter,
    }
}

fn unproved_association(
    principal_id: ResidentPrincipalIdV1,
    store: AuthorityStoreCoordinate,
) -> AdmittedResidentDeploymentAssociation {
    let label = format!(
        "{}/{}",
        store.store_id().as_str(),
        store.incarnation_id().as_str()
    );
    AdmittedResidentDeploymentAssociation {
        principal_id,
        store,
        deployment_binding_ref: ResidentRecordRefV1::parse(format!("binding/{label}"))
            .expect("test deployment-binding reference"),
        deployment_binding_sha256: DeploymentBindingDigest::hash_canonical(&format!(
            "binding/{label}"
        ))
        .expect("test deployment-binding digest"),
        runtime_authority_ref: ResidentRecordRefV1::parse(format!("authority/{label}"))
            .expect("test runtime-authority reference"),
        runtime_authority_sha256: DeploymentRuntimeAuthorityDigest::hash_canonical(&format!(
            "authority/{label}"
        ))
        .expect("test runtime-authority digest"),
    }
}

pub(in crate::resident::process_host) fn unproved_selector(
    principal_id: ResidentPrincipalIdV1,
    store: AuthorityStoreCoordinate,
) -> ResidentDeploymentAuthoritySelector {
    let association = unproved_association(principal_id, store);
    ResidentDeploymentAuthoritySelector {
        principal_id: association.principal_id,
        deployment_binding_ref: association.deployment_binding_ref,
        deployment_binding_sha256: association.deployment_binding_sha256,
        runtime_authority_ref: association.runtime_authority_ref,
        runtime_authority_sha256: association.runtime_authority_sha256,
    }
}

pub(in crate::resident::process_host) fn integrated_signing_authority(
    selector: &ResidentDeploymentAuthoritySelector,
) -> SigningAuthorityBindingV1 {
    SigningAuthorityBindingV1::integrated_runtime(
        selector.principal_id.clone(),
        ResidentKeyIdV1::parse("runtime-key").expect("runtime key"),
        SigningKeyEpochV1::new(1).expect("runtime key epoch"),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: selector.runtime_authority_ref.clone(),
            deployment_runtime_authority_sha256: ArtifactDigest::parse(
                selector.runtime_authority_sha256.as_str(),
            )
            .expect("runtime authority artifact digest"),
            deployment_binding_ref: selector.deployment_binding_ref.clone(),
            deployment_binding_sha256: ArtifactDigest::parse(
                selector.deployment_binding_sha256.as_str(),
            )
            .expect("deployment binding artifact digest"),
            checked_principal_trust_checkpoint_ref: ResidentRecordRefV1::parse("trust/checkpoint")
                .expect("trust checkpoint reference"),
            checked_principal_trust_checkpoint_sha256:
                local_operation_start_fixture::artifact_digest('f'),
        },
    )
    .expect("integrated runtime signing authority")
}

fn owner_native_signing_authority(
    principal_id: ResidentPrincipalIdV1,
) -> SigningAuthorityBindingV1 {
    SigningAuthorityBindingV1::standalone_owner_native(
        principal_id,
        ResidentKeyIdV1::parse("owner-key").expect("owner key"),
        SigningKeyEpochV1::new(1).expect("owner key epoch"),
        StandaloneOwnerNativeAuthorityV1 {
            owner_component_id: OwnerComponentIdV1::parse("owner-component")
                .expect("owner component"),
            owner_authority_schema_id: SchemaId::parse("owner/authority.v1")
                .expect("owner authority schema"),
            owner_authority_schema_sha256: SchemaDigest::parse("4".repeat(64))
                .expect("owner authority schema digest"),
            owner_authority_ref: ResidentRecordRefV1::parse("owner/authority")
                .expect("owner authority reference"),
            owner_authority_sha256: local_operation_start_fixture::artifact_digest('1'),
            owner_contract_manifest_ref: ResidentRecordRefV1::parse("owner/manifest")
                .expect("owner manifest reference"),
            owner_contract_manifest_sha256: local_operation_start_fixture::artifact_digest('2'),
            checked_owner_revocation_frontier_ref: ResidentRecordRefV1::parse(
                "owner/revocation-frontier",
            )
            .expect("owner revocation reference"),
            checked_owner_revocation_frontier_sha256:
                local_operation_start_fixture::artifact_digest('3'),
        },
    )
    .expect("standalone owner-native signing authority")
}

fn unproved_deployment<'store>(
    principal_id: ResidentPrincipalIdV1,
    claimed_store: &AuthorityStoreCoordinate,
    application: Box<ResidentApplication<'store>>,
) -> Result<ReadyResidentDeployment<'store>, ResidentDeploymentJoinError> {
    ReadyResidentDeployment::new(
        unproved_association(principal_id, claimed_store.clone()),
        application,
    )
}

fn with_unproved_operation_service_for_test<Authority, Output>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    access: ResidentDeploymentOperationAccess<'_>,
    authority: &mut Authority,
    use_service: impl FnOnce(
        &AuthorityStoreCoordinate,
        &mut ResidentOperationService<'_, Authority>,
    ) -> Output,
) -> Result<Output, ResidentProcessHostSelectionError> {
    host.with_selected_operation_service(
        selector,
        access,
        authority,
        |application, service, _sessions| {
            application
                .ensure_current_projection()
                .expect("test deployment projection remains current");
            use_service(application.store(), service)
        },
    )
}

/// Prove that the process boundary checks the inputs it will actually commit.
pub(in crate::resident) fn assert_consultation_terminal_association_fences(
    signing_authority: &SigningAuthorityBindingV1,
    recovery_store: &AuthorityStoreCoordinate,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    input: &ConsultationChildJoinInput<'_>,
    checkpoint: &ConsultationSuccessorCommitInput<'_>,
) {
    let selector = selector_from_signing_authority(signing_authority);
    let association = ResidentOperationControlAssociation {
        current_request_authority: signing_authority,
        historical_handle_authority: signing_authority,
        recovery_store,
        checkpoint_authority,
    };
    validate_consultation_terminal_associations(
        &selector,
        recovery_store,
        &association,
        input,
        checkpoint,
    )
    .expect("the exact Consult terminal inputs share one deployment authority");

    let unlike_historical_signer = SigningAuthorityBindingV1::integrated_runtime(
        selector.principal_id.clone(),
        ResidentKeyIdV1::parse("foreign-consult-historical-key").unwrap(),
        SigningKeyEpochV1::new(2).unwrap(),
        signing_authority
            .integrated_runtime_authority()
            .expect("fixture uses integrated runtime authority")
            .clone(),
    )
    .unwrap();
    let unlike_historical_association = ResidentOperationControlAssociation {
        current_request_authority: signing_authority,
        historical_handle_authority: &unlike_historical_signer,
        recovery_store,
        checkpoint_authority,
    };
    assert!(matches!(
        validate_consultation_terminal_associations(
            &selector,
            recovery_store,
            &unlike_historical_association,
            input,
            checkpoint,
        ),
        Err(ResidentProcessHostSelectionError::ConsultationOperationAuthority)
    ));

    let unlike_signer = owner_native_signing_authority(selector.principal_id.clone());
    let unlike_signed = ConsultationChildJoinInput {
        slot_id: input.slot_id,
        child: input.child,
        recorded_at: input.recorded_at,
        parent: input.parent,
        parent_observation: input.parent_observation,
        observation_ref: input.observation_ref.clone(),
        observed_at: input.observed_at,
        signing: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: input.signing.expected_schema_sha256,
            signing_authority: &unlike_signer,
        },
        limits: input.limits,
    };
    assert!(matches!(
        validate_consultation_terminal_associations(
            &selector,
            recovery_store,
            &association,
            &unlike_signed,
            checkpoint,
        ),
        Err(ResidentProcessHostSelectionError::ConsultationSigningAuthority)
    ));

    let unlike_authority_ref =
        ResidentRecordRefV1::parse("authority/foreign-consult-terminal").unwrap();
    let unlike_authority_digest =
        pale_core::records::CheckpointSigningAuthorityDigest::parse("5".repeat(64)).unwrap();
    let unlike_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        authority_ref: &unlike_authority_ref,
        authority_sha256: &unlike_authority_digest,
        ..*checkpoint.authority
    };
    let unlike_checkpoint = ConsultationSuccessorCommitInput {
        accumulator_limits: checkpoint.accumulator_limits,
        identity: OperationCheckpointInput {
            reference: checkpoint.identity.reference.clone(),
            id: checkpoint.identity.id.clone(),
        },
        issued_at: checkpoint.issued_at,
        schema: checkpoint.schema,
        authority: &unlike_authority,
    };
    assert!(matches!(
        validate_consultation_terminal_associations(
            &selector,
            recovery_store,
            &association,
            input,
            &unlike_checkpoint,
        ),
        Err(ResidentProcessHostSelectionError::ConsultationCheckpointAuthority)
    ));

    let child_head = input.child.selected_head();
    let child_checkpoint = input.child.selected_checkpoint_record();
    let foreign_store = store(
        "foreign-consult-terminal",
        "foreign-consult-terminal-incarnation",
    );
    let foreign_child = RecoveredOperationObservation::fixture_for_handle(
        input.child.handle.clone(),
        (child_head.0.clone(), child_head.1.clone()),
        (child_head.0.clone(), child_head.1.clone()),
        (child_checkpoint.0.clone(), child_checkpoint.1.clone()),
        foreign_store,
    );
    let unlike_child = ConsultationChildJoinInput {
        slot_id: input.slot_id,
        child: &foreign_child,
        recorded_at: input.recorded_at,
        parent: input.parent,
        parent_observation: input.parent_observation,
        observation_ref: input.observation_ref.clone(),
        observed_at: input.observed_at,
        signing: input.signing,
        limits: input.limits,
    };
    assert!(matches!(
        validate_consultation_terminal_associations(
            &selector,
            recovery_store,
            &association,
            &unlike_child,
            checkpoint,
        ),
        Err(ResidentProcessHostSelectionError::ConsultationOperationAuthority)
    ));
}

pub(super) fn selector_from_signing_authority(
    binding: &SigningAuthorityBindingV1,
) -> ResidentDeploymentAuthoritySelector {
    let authority = binding
        .integrated_runtime_authority()
        .expect("process-host Observe fixture uses integrated runtime authority");
    ResidentDeploymentAuthoritySelector {
        principal_id: binding.principal_id().clone(),
        deployment_binding_ref: authority.deployment_binding_ref.clone(),
        deployment_binding_sha256: DeploymentBindingDigest::parse(
            authority.deployment_binding_sha256.as_str(),
        )
        .expect("deployment-binding digest has its exact contract type"),
        runtime_authority_ref: authority.deployment_runtime_authority_ref.clone(),
        runtime_authority_sha256: DeploymentRuntimeAuthorityDigest::parse(
            authority.deployment_runtime_authority_sha256.as_str(),
        )
        .expect("runtime-authority digest has its exact contract type"),
    }
}

pub(in crate::resident) fn pairwise_resolution_fixture(
    operation: &crate::resident::IssuedLocalOperation,
    request: CommonRequestEnvelopeV1,
    candidate: &pale_ports::OperationCustodyCandidate,
    checkpoint: ControlStateCheckpointV1,
    signing: &SigningAuthorityBindingV1,
    generation_digest: pale_core::records::CompiledGenerationDigest,
) -> crate::resident::process_host::restart::ResidentProcessRecoveredOperationResolution {
    let selector = selector_from_signing_authority(signing);
    let control =
        restart::initial_operation_control(operation, request, candidate, checkpoint, signing);
    crate::resident::process_host::restart::ResidentProcessRecoveredOperationResolution::pairwise_fixture(
        selector,
        control,
        generation_digest,
        operation.handle().0,
        operation.handle().1,
        crate::resident::process_host::restart::ResidentProcessOperationResolutionPhase::Resolving,
    )
}

pub(in crate::resident::process_host) fn unproved_deployment_for_selector<'store>(
    selector: &ResidentDeploymentAuthoritySelector,
    store: AuthorityStoreCoordinate,
    application: Box<ResidentApplication<'store>>,
) -> ReadyResidentDeployment<'store> {
    ReadyResidentDeployment::new(
        AdmittedResidentDeploymentAssociation {
            principal_id: selector.principal_id.clone(),
            store,
            deployment_binding_ref: selector.deployment_binding_ref.clone(),
            deployment_binding_sha256: selector.deployment_binding_sha256.clone(),
            runtime_authority_ref: selector.runtime_authority_ref.clone(),
            runtime_authority_sha256: selector.runtime_authority_sha256.clone(),
        },
        application,
    )
    .expect("test association and recovered application share one store")
}

fn request_with_signing_authority(
    request: &CommonRequestEnvelopeV1,
    signing_authority_binding: SigningAuthorityBindingV1,
) -> CommonRequestEnvelopeV1 {
    let mut preimage = request.to_preimage();
    preimage.caller.signer_key_id =
        ResidentKeyIdV1::parse(signing_authority_binding.signer_key_id())
            .expect("substituted signer key remains a bounded resident identity");
    preimage.caller.signer_key_epoch = *signing_authority_binding.signer_key_epoch();
    preimage.route.origin_principal_id = signing_authority_binding.principal_id().clone();
    if let Some(runtime) = signing_authority_binding.integrated_runtime_authority() {
        preimage.session_admission.deployment_binding_ref = runtime.deployment_binding_ref.clone();
        preimage.session_admission.deployment_binding_sha256 =
            runtime.deployment_binding_sha256.clone();
    }
    preimage.signing_authority_binding = signing_authority_binding;
    crate::effect_case_lineage_fixture::request(&preimage)
}

fn observation_input_with<'a>(
    base: &LocalOperationObservationInput<'a>,
    request: &'a CommonRequestEnvelopeV1,
    historical_authority: &'a SigningAuthorityBindingV1,
    recovery_store: &'a AuthorityStoreCoordinate,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
) -> LocalOperationObservationInput<'a> {
    LocalOperationObservationInput {
        recovery: OperationControlRecoveryInput {
            operation: LocalOperationHandleRecoveryInput {
                context: LocalOperationRecoveryContext {
                    store: recovery_store,
                    handle_signing_authority: historical_authority,
                    checkpoint_authority,
                    ..base.recovery.operation.context
                },
                expected_handle: base.recovery.operation.expected_handle,
                maximum_parent_depth: base.recovery.operation.maximum_parent_depth,
            },
            observation: base.recovery.observation,
            observation_limits: base.recovery.observation_limits,
            checkpoint_limits: base.recovery.checkpoint_limits,
            membership_limits: base.recovery.membership_limits,
        },
        request,
        authorized_at: base.authorized_at,
    }
}

fn cancellation_input_with<'a>(
    base: &ResidentOperationCancellationInput<'a>,
    request: &'a CommonRequestEnvelopeV1,
    historical_authority: &'a SigningAuthorityBindingV1,
    recovery_store: &'a AuthorityStoreCoordinate,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
) -> ResidentOperationCancellationInput<'a> {
    ResidentOperationCancellationInput {
        recovery: OperationControlRecoveryInput {
            operation: LocalOperationHandleRecoveryInput {
                context: LocalOperationRecoveryContext {
                    store: recovery_store,
                    handle_signing_authority: historical_authority,
                    checkpoint_authority,
                    ..base.recovery.operation.context
                },
                expected_handle: base.recovery.operation.expected_handle,
                maximum_parent_depth: base.recovery.operation.maximum_parent_depth,
            },
            observation: base.recovery.observation,
            observation_limits: base.recovery.observation_limits,
            checkpoint_limits: base.recovery.checkpoint_limits,
            membership_limits: base.recovery.membership_limits,
        },
        request,
        authorized_at: base.authorized_at,
        request_ref: base.request_ref.clone(),
        observation_ref: base.observation_ref.clone(),
        signing: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: base.signing.expected_schema_sha256,
            signing_authority: base.signing.signing_authority,
        },
        checkpoint: OperationCheckpointInput {
            reference: base.checkpoint.reference.clone(),
            id: base.checkpoint.id.clone(),
        },
        checkpoint_issued_at: base.checkpoint_issued_at,
        checkpoint_schema: base.checkpoint_schema,
        checkpoint_authority: base.checkpoint_authority,
        accumulator_limits: base.accumulator_limits,
    }
}

fn local_cancellation_completion(
    input: &ResidentOperationCancellationInput<'_>,
) -> ResidentLocalCancellationCompletionInput {
    ResidentLocalCancellationCompletionInput {
        diagnostic_ref: local_operation_start_fixture::record_ref(
            "process-host/local-cancel-diagnostic",
        ),
        observation_ref: local_operation_start_fixture::record_ref(
            "process-host/local-cancel-terminal",
        ),
        established_at: UnixTimeV1::new(input.authorized_at.get() + 2)
            .expect("terminal cancellation time"),
        checkpoint: OperationCheckpointInput {
            reference: local_operation_start_fixture::record_ref(
                "process-host/local-cancel-terminal-checkpoint",
            ),
            id: pale_core::records::ControlCheckpointIdV1::parse(
                "process-host-local-cancel-terminal",
            )
            .expect("terminal cancellation checkpoint identity"),
        },
        checkpoint_issued_at: UnixTimeV1::new(input.authorized_at.get() + 3)
            .expect("terminal cancellation checkpoint time"),
        maximum_diagnostic_bytes: 1 << 20,
    }
}

fn authority_axis_substitutions(
    selector: &ResidentDeploymentAuthoritySelector,
) -> Vec<ResidentDeploymentAuthoritySelector> {
    let mut substitutions = Vec::new();
    let mut foreign_principal = selector.clone();
    foreign_principal.principal_id = principal("principal-foreign");
    substitutions.push(foreign_principal);
    let mut foreign_binding_ref = selector.clone();
    foreign_binding_ref.deployment_binding_ref =
        ResidentRecordRefV1::parse("binding/foreign").expect("foreign binding reference");
    substitutions.push(foreign_binding_ref);
    let mut foreign_binding_digest = selector.clone();
    foreign_binding_digest.deployment_binding_sha256 =
        DeploymentBindingDigest::hash_canonical("binding/foreign").expect("foreign binding digest");
    substitutions.push(foreign_binding_digest);
    let mut foreign_runtime_ref = selector.clone();
    foreign_runtime_ref.runtime_authority_ref =
        ResidentRecordRefV1::parse("authority/foreign").expect("foreign authority reference");
    substitutions.push(foreign_runtime_ref);
    let mut foreign_runtime_digest = selector.clone();
    foreign_runtime_digest.runtime_authority_sha256 =
        DeploymentRuntimeAuthorityDigest::hash_canonical("authority/foreign")
            .expect("foreign authority digest");
    substitutions.push(foreign_runtime_digest);
    substitutions
}

fn observe_with_unreachable_recovery<Authority>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    access: ResidentDeploymentOperationAccess<'_>,
    authority: &mut Authority,
    input: LocalOperationObservationInput<'_>,
) -> Result<OperationControlObservation, ResidentProcessObservationError> {
    let mut admit_request = |_: &ResidentRecordRefV1,
                             _: CommonRequestEnvelopeWireV1|
     -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not read historical request custody")
    };
    let mut admit_handle_authority = |_: &OperationHandlePreimageV1,
                                      _: Option<&OperationHandleV1>|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not admit a historical handle")
    };
    let mut verify_handle_signature = |_: &[u8],
                                       _: &OperationHandlePreimageV1,
                                       _: &OperationHandleDigest,
                                       _: &CanonicalSignatureV1|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not verify a historical handle")
    };
    let mut admit_checkpoint = |_: &ResidentRecordRefV1,
                                _: ControlStateCheckpointWireV1|
     -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not admit a checkpoint")
    };
    let mut resolve_observation_signer =
        |_: &ResidentRecordRefV1,
         _: &OperationObservationWireV1|
         -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1> {
            panic!("rejected process-host coordinates must not resolve an observation signer")
        };
    let mut admit_observation_state =
        |_: &OperationObservationStateV1| -> Result<(), InvalidResidentRecordV1> {
            panic!("rejected process-host coordinates must not admit observation state")
        };
    let mut admit_rotation = |_: &ResidentRecordRefV1,
                              _: &ArtifactDigest,
                              _: &ResidentRecordRefV1,
                              _: &ArtifactDigest|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not admit authority rotation")
    };
    let mut verify_observation_signature = |_: &[u8],
                                            _: &OperationObservationPreimageV1,
                                            _: &OperationObservationDigest,
                                            _: &CanonicalSignatureV1|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not verify an observation")
    };
    let mut verify_checkpoint_signature = |_: &[u8],
                                           _: &ControlStateCheckpointPreimageV1,
                                           _: &ControlStateCheckpointDigest,
                                           _: &CanonicalSignatureV1|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not verify a checkpoint")
    };
    let mut authorize =
        |_: OperationControlAuthorizationContext<'_>| -> Result<(), InvalidResidentRecordV1> {
            panic!("rejected process-host coordinates must not invoke present authorization")
        };
    host.observe(
        selector,
        access,
        authority,
        input,
        LocalOperationObservationAdmission {
            recovery: OperationControlRecoveryAdmission {
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
            authorize: &mut authorize,
        },
    )
}

fn cancel_with_unreachable_recovery<Authority>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    access: ResidentDeploymentOperationAccess<'_>,
    authority: &mut Authority,
    input: ResidentOperationCancellationInput<'_>,
) -> Result<ResidentProcessCancellationOutcome, ResidentProcessCancellationError> {
    let mut admit_request = |_: &ResidentRecordRefV1,
                             _: CommonRequestEnvelopeWireV1|
     -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not read historical request custody")
    };
    let mut admit_handle_authority = |_: &OperationHandlePreimageV1,
                                      _: Option<&OperationHandleV1>|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not admit a historical handle")
    };
    let mut verify_handle_signature = |_: &[u8],
                                       _: &OperationHandlePreimageV1,
                                       _: &OperationHandleDigest,
                                       _: &CanonicalSignatureV1|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not verify a historical handle")
    };
    let mut admit_checkpoint = |_: &ResidentRecordRefV1,
                                _: ControlStateCheckpointWireV1|
     -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not admit a checkpoint")
    };
    let mut resolve_observation_signer =
        |_: &ResidentRecordRefV1,
         _: &OperationObservationWireV1|
         -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1> {
            panic!("rejected process-host coordinates must not resolve an observation signer")
        };
    let mut admit_observation_state =
        |_: &OperationObservationStateV1| -> Result<(), InvalidResidentRecordV1> {
            panic!("rejected process-host coordinates must not admit observation state")
        };
    let mut admit_recovery_rotation = |_: &ResidentRecordRefV1,
                                       _: &ArtifactDigest,
                                       _: &ResidentRecordRefV1,
                                       _: &ArtifactDigest|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not admit historical authority rotation")
    };
    let mut verify_observation_signature = |_: &[u8],
                                            _: &OperationObservationPreimageV1,
                                            _: &OperationObservationDigest,
                                            _: &CanonicalSignatureV1|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not verify an observation")
    };
    let mut verify_checkpoint_signature = |_: &[u8],
                                           _: &ControlStateCheckpointPreimageV1,
                                           _: &ControlStateCheckpointDigest,
                                           _: &CanonicalSignatureV1|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not verify a checkpoint")
    };
    let mut authorize =
        |_: OperationControlAuthorizationContext<'_>| -> Result<(), InvalidResidentRecordV1> {
            panic!("rejected process-host coordinates must not invoke present authorization")
        };
    let resolve_store = |_: &OperationObservationV1| {
        panic!("rejected process-host coordinates must not resolve an append store")
    };
    let mut admit_append_rotation = |_: &ResidentRecordRefV1,
                                     _: &ArtifactDigest,
                                     _: &ResidentRecordRefV1,
                                     _: &ArtifactDigest|
     -> Result<(), InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not admit append authority rotation")
    };
    let mut sign_observation = |_: &[u8],
                                _: &OperationObservationPreimageV1,
                                _: &OperationObservationDigest|
     -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not sign an observation")
    };
    let mut sign_checkpoint = |_: &[u8],
                               _: &ControlStateCheckpointPreimageV1,
                               _: &ControlStateCheckpointDigest|
     -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        panic!("rejected process-host coordinates must not sign a checkpoint")
    };
    let completion = local_cancellation_completion(&input);
    host.request_cancellation(
        selector,
        access,
        authority,
        input,
        completion,
        ResidentOperationCancellationAdmission {
            recovery: OperationControlRecoveryAdmission {
                admit_request: &mut admit_request,
                admit_handle_authority: &mut admit_handle_authority,
                verify_handle_signature: &mut verify_handle_signature,
                admit_checkpoint: &mut admit_checkpoint,
                resolve_observation_signer: &mut resolve_observation_signer,
                admit_observation_state: &mut admit_observation_state,
                admit_rotation: &mut admit_recovery_rotation,
                verify_observation_signature: &mut verify_observation_signature,
                verify_checkpoint_signature: &mut verify_checkpoint_signature,
            },
            authorize: &mut authorize,
            resolve_store: &resolve_store,
            admit_rotation: &mut admit_append_rotation,
            sign_observation: &mut sign_observation,
            sign_checkpoint: &mut sign_checkpoint,
        },
    )
}

#[test]
fn substituted_caller_coordinate_cannot_create_a_ready_deployment() {
    let port = MemoryAuthorityStore::default();
    let actual = store("deployment-a", "incarnation-a");
    let substituted = store("deployment-b", "incarnation-b");
    let application = ready_application(&port, actual, 1);

    let result = unproved_deployment(principal("principal-a"), &substituted, application);

    assert!(matches!(
        result,
        Err(ResidentDeploymentJoinError::StoreSubstitution)
    ));
}

#[test]
fn duplicate_exact_store_is_rejected_before_a_host_exists() {
    let first_port = MemoryAuthorityStore::default();
    let second_port = MemoryAuthorityStore::default();
    let principal = principal("principal-a");
    let coordinate = store("deployment-a", "incarnation-a");
    let first = unproved_deployment(
        principal.clone(),
        &coordinate,
        ready_application(&first_port, coordinate.clone(), 1),
    )
    .expect("first unproved witness");
    let second = unproved_deployment(
        principal.clone(),
        &coordinate,
        ready_application(&second_port, coordinate.clone(), 2),
    )
    .expect("second unproved witness");
    let writer = unproved_writer(principal, vec![coordinate.clone(), coordinate]);

    let result = ResidentProcessHost::new(
        writer,
        vec![first, second],
        NonZeroUsize::new(2).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    );

    assert!(matches!(
        result,
        Err(ResidentProcessHostBuildError::DuplicateDeploymentStore)
    ));
}

#[test]
fn one_admitted_authority_cannot_select_two_deployment_stores() {
    let first_port = MemoryAuthorityStore::default();
    let second_port = MemoryAuthorityStore::default();
    let principal = principal("principal-a");
    let first_store = store("deployment-a", "incarnation-a");
    let second_store = store("deployment-b", "incarnation-b");
    let first = unproved_deployment(
        principal.clone(),
        &first_store,
        ready_application(&first_port, first_store.clone(), 1),
    )
    .expect("first unproved witness");
    let mut second = unproved_deployment(
        principal.clone(),
        &second_store,
        ready_application(&second_port, second_store.clone(), 2),
    )
    .expect("second unproved witness");
    second.association.deployment_binding_ref = first.association.deployment_binding_ref.clone();
    second.association.deployment_binding_sha256 =
        first.association.deployment_binding_sha256.clone();
    second.association.runtime_authority_ref = first.association.runtime_authority_ref.clone();
    second.association.runtime_authority_sha256 =
        first.association.runtime_authority_sha256.clone();
    let writer = unproved_writer(principal, vec![first_store, second_store]);

    let result = ResidentProcessHost::new(
        writer,
        vec![first, second],
        NonZeroUsize::new(2).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    );

    assert!(matches!(
        result,
        Err(ResidentProcessHostBuildError::DuplicateDeploymentAuthority)
    ));
}

#[test]
fn two_ready_incarnations_of_one_store_are_rejected() {
    let first_port = MemoryAuthorityStore::default();
    let second_port = MemoryAuthorityStore::default();
    let principal = principal("principal-a");
    let first_store = store("deployment-a", "incarnation-a");
    let second_store = store("deployment-a", "incarnation-b");
    let first = unproved_deployment(
        principal.clone(),
        &first_store,
        ready_application(&first_port, first_store.clone(), 1),
    )
    .expect("first unproved witness");
    let second = unproved_deployment(
        principal.clone(),
        &second_store,
        ready_application(&second_port, second_store.clone(), 2),
    )
    .expect("second unproved witness");
    let writer = unproved_writer(principal, vec![first_store, second_store]);

    let result = ResidentProcessHost::new(
        writer,
        vec![first, second],
        NonZeroUsize::new(2).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    );

    assert!(matches!(
        result,
        Err(ResidentProcessHostBuildError::MultipleReadyIncarnations)
    ));
}

#[test]
fn principal_and_opened_store_associations_must_match_the_writer() {
    let first_port = MemoryAuthorityStore::default();
    let second_port = MemoryAuthorityStore::default();
    let third_port = MemoryAuthorityStore::default();
    let first_store = store("deployment-a", "incarnation-a");
    let second_store = store("deployment-b", "incarnation-b");
    let foreign = unproved_deployment(
        principal("principal-b"),
        &first_store,
        ready_application(&first_port, first_store.clone(), 1),
    )
    .expect("foreign unproved witness");
    let writer = unproved_writer(principal("principal-a"), vec![first_store.clone()]);
    assert!(matches!(
        ResidentProcessHost::new(
            writer,
            vec![foreign],
            NonZeroUsize::new(1).expect("positive deployment bound"),
            ResidentOperationHost::new(task_state()),
        ),
        Err(ResidentProcessHostBuildError::PrincipalAssociation)
    ));

    let local = unproved_deployment(
        principal("principal-a"),
        &first_store,
        ready_application(&second_port, first_store.clone(), 2),
    )
    .expect("local unproved witness");
    let mismatched_writer = unproved_writer(principal("principal-a"), vec![second_store.clone()]);
    assert!(matches!(
        ResidentProcessHost::new(
            mismatched_writer,
            vec![local],
            NonZeroUsize::new(1).expect("positive deployment bound"),
            ResidentOperationHost::new(task_state()),
        ),
        Err(ResidentProcessHostBuildError::OpenedStoreSet)
    ));

    let mut substituted_association = unproved_deployment(
        principal("principal-a"),
        &first_store,
        ready_application(&third_port, first_store.clone(), 3),
    )
    .expect("association unproved witness");
    substituted_association.association.store = second_store;
    let exact_writer = unproved_writer(principal("principal-a"), vec![first_store]);
    assert!(matches!(
        ResidentProcessHost::new(
            exact_writer,
            vec![substituted_association],
            NonZeroUsize::new(1).expect("positive deployment bound"),
            ResidentOperationHost::new(task_state()),
        ),
        Err(ResidentProcessHostBuildError::RuntimeAuthorityStoreAssociation)
    ));
}

#[test]
fn exact_selection_never_falls_back_and_unused_ports_remain_untouched() {
    let first_port = MemoryAuthorityStore::default();
    let second_port = MemoryAuthorityStore::default();
    let principal = principal("principal-a");
    let first_store = store("deployment-a", "incarnation-a");
    let second_store = store("deployment-b", "incarnation-b");
    let second_selector = unproved_selector(principal.clone(), second_store.clone());
    let first = unproved_deployment(
        principal.clone(),
        &first_store,
        ready_application(&first_port, first_store.clone(), 1),
    )
    .expect("first unproved witness");
    let second = unproved_deployment(
        principal.clone(),
        &second_store,
        ready_application(&second_port, second_store.clone(), 2),
    )
    .expect("second unproved witness");
    let writer = unproved_writer(principal, vec![second_store.clone(), first_store.clone()]);
    let mut host = ResidentProcessHost::new(
        writer,
        vec![second, first],
        NonZeroUsize::new(2).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("exact deployment set");
    let (mut authority, mut custody, mut anchor) = operation_ports();

    let selected = with_unproved_operation_service_for_test(
        &mut host,
        &second_selector,
        ResidentDeploymentOperationAccess {
            store: &second_store,
            custody: &mut custody,
            sessions: &second_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |store, _service| store.clone(),
    )
    .expect("second exact store is selected");
    assert_eq!(selected, second_store);

    let mut substitutions = Vec::new();
    let mut foreign_principal = second_selector.clone();
    foreign_principal.principal_id =
        ResidentPrincipalIdV1::parse("principal-b").expect("foreign principal identity");
    substitutions.push(foreign_principal);
    let mut foreign_binding_ref = second_selector.clone();
    foreign_binding_ref.deployment_binding_ref =
        ResidentRecordRefV1::parse("binding/foreign").expect("foreign binding reference");
    substitutions.push(foreign_binding_ref);
    let mut foreign_binding_digest = second_selector.clone();
    foreign_binding_digest.deployment_binding_sha256 =
        DeploymentBindingDigest::hash_canonical("binding/foreign").expect("foreign binding digest");
    substitutions.push(foreign_binding_digest);
    let mut foreign_runtime_ref = second_selector.clone();
    foreign_runtime_ref.runtime_authority_ref =
        ResidentRecordRefV1::parse("authority/foreign").expect("foreign authority reference");
    substitutions.push(foreign_runtime_ref);
    let mut foreign_runtime_digest = second_selector.clone();
    foreign_runtime_digest.runtime_authority_sha256 =
        DeploymentRuntimeAuthorityDigest::hash_canonical("authority/foreign")
            .expect("foreign authority digest");
    substitutions.push(foreign_runtime_digest);

    for substituted in &substitutions {
        let result = with_unproved_operation_service_for_test(
            &mut host,
            substituted,
            ResidentDeploymentOperationAccess {
                store: &second_store,
                custody: &mut custody,
                sessions: &second_port,
                anchor: &mut anchor,
            },
            &mut authority,
            |_store, _service| panic!("unlike authority must not enter the service closure"),
        );
        assert!(matches!(
            result,
            Err(ResidentProcessHostSelectionError::UnknownDeploymentAuthority)
        ));
    }

    let substituted_access_store = store("deployment-z", "incarnation-z");
    let result = with_unproved_operation_service_for_test(
        &mut host,
        &second_selector,
        ResidentDeploymentOperationAccess {
            store: &substituted_access_store,
            custody: &mut custody,
            sessions: &second_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_store, _service| panic!("unlike access store must not enter the service closure"),
    );
    assert!(matches!(
        result,
        Err(ResidentProcessHostSelectionError::AccessStoreAssociation)
    ));

    let missing = store("deployment-a", "unknown-incarnation");
    let missing_selector = unproved_selector(
        ResidentPrincipalIdV1::parse("principal-a").expect("principal identity"),
        missing.clone(),
    );
    let result = with_unproved_operation_service_for_test(
        &mut host,
        &missing_selector,
        ResidentDeploymentOperationAccess {
            store: &missing,
            custody: &mut custody,
            sessions: &first_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_store, _service| (),
    );
    assert!(matches!(
        result,
        Err(ResidentProcessHostSelectionError::UnknownDeploymentAuthority)
    ));
    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(anchor.access_counts(), (0, 0));
}

#[test]
fn observation_authority_and_store_joins_fail_before_any_port_io() {
    let deployment_port = MemoryAuthorityStore::default();
    let principal_id = principal("principal-1");
    let selected_store = local_operation_start_fixture::store_coordinate();
    let selector = unproved_selector(principal_id.clone(), selected_store.clone());
    let deployment = unproved_deployment(
        principal_id.clone(),
        &selected_store,
        ready_application(&deployment_port, selected_store.clone(), 1),
    )
    .expect("unproved deployment witness");
    let writer = unproved_writer(principal_id.clone(), vec![selected_store.clone()]);
    let mut host = ResidentProcessHost::new(
        writer,
        vec![deployment],
        NonZeroUsize::new(1).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one exact deployment");
    let integrated = integrated_signing_authority(&selector);
    let checkpoint = local_operation_start_fixture::checkpoint_preimage(1, None);
    let checkpoint_authority = local_operation_start_fixture::checkpoint_authority(&checkpoint);
    let (mut authority, mut custody, mut anchor) = operation_ports();

    host.with_selected_operation_control_service(
        &selector,
        ResidentOperationControlAssociation {
            current_request_authority: &integrated,
            historical_handle_authority: &integrated,
            recovery_store: &selected_store,
            checkpoint_authority: &checkpoint_authority,
        },
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_application, _service, _sessions| (),
    )
    .expect("exact private observation associations");

    let owner_native = owner_native_signing_authority(principal_id.clone());
    let result = host.with_selected_operation_control_service(
        &selector,
        ResidentOperationControlAssociation {
            current_request_authority: &owner_native,
            historical_handle_authority: &integrated,
            recovery_store: &selected_store,
            checkpoint_authority: &checkpoint_authority,
        },
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_application, _service, _sessions| {
            panic!("owner-native request must not enter the observation service")
        },
    );
    assert!(matches!(
        result,
        Err(ResidentProcessHostSelectionError::CurrentRequestAuthority)
    ));

    let mut foreign_selector = selector.clone();
    foreign_selector.runtime_authority_ref = ResidentRecordRefV1::parse("authority/foreign")
        .expect("foreign runtime authority reference");
    let foreign_historical = integrated_signing_authority(&foreign_selector);
    let result = host.with_selected_operation_control_service(
        &selector,
        ResidentOperationControlAssociation {
            current_request_authority: &integrated,
            historical_handle_authority: &foreign_historical,
            recovery_store: &selected_store,
            checkpoint_authority: &checkpoint_authority,
        },
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_application, _service, _sessions| {
            panic!("foreign handle authority must not enter the observation service")
        },
    );
    assert!(matches!(
        result,
        Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority)
    ));

    let foreign_store = store("foreign-store", "foreign-incarnation");
    let result = host.with_selected_operation_control_service(
        &selector,
        ResidentOperationControlAssociation {
            current_request_authority: &integrated,
            historical_handle_authority: &integrated,
            recovery_store: &foreign_store,
            checkpoint_authority: &checkpoint_authority,
        },
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_application, _service, _sessions| {
            panic!("foreign recovery store must not enter the observation service")
        },
    );
    assert!(matches!(
        result,
        Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation)
    ));

    let foreign_principal = principal("principal-z");
    let foreign_checkpoint = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        principal_id: &foreign_principal,
        ..checkpoint_authority
    };
    let result = host.with_selected_operation_control_service(
        &selector,
        ResidentOperationControlAssociation {
            current_request_authority: &integrated,
            historical_handle_authority: &integrated,
            recovery_store: &selected_store,
            checkpoint_authority: &foreign_checkpoint,
        },
        ResidentDeploymentOperationAccess {
            store: &selected_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_application, _service, _sessions| {
            panic!("foreign checkpoint authority must not enter the observation service")
        },
    );
    assert!(matches!(
        result,
        Err(ResidentProcessHostSelectionError::CheckpointAuthorityAssociation)
    ));

    let result = host.with_selected_operation_control_service(
        &selector,
        ResidentOperationControlAssociation {
            current_request_authority: &integrated,
            historical_handle_authority: &integrated,
            recovery_store: &selected_store,
            checkpoint_authority: &checkpoint_authority,
        },
        ResidentDeploymentOperationAccess {
            store: &foreign_store,
            custody: &mut custody,
            sessions: &deployment_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_application, _service, _sessions| {
            panic!("foreign access store must not enter the observation service")
        },
    );
    assert!(matches!(
        result,
        Err(ResidentProcessHostSelectionError::AccessStoreAssociation)
    ));
    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(anchor.access_counts(), (0, 0));
}

#[test]
fn process_host_observe_extracts_every_authority_coordinate_before_real_recovery() {
    with_process_host_observe_fixture(&mut |fixture: ProcessHostObserveFixture<'_>| {
        let ProcessHostObserveFixture {
            input,
            admission,
            signing_authority,
            custody,
            sessions,
            anchor,
        } = fixture;
        let selected_store = input.recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let application_port = MemoryAuthorityStore::default();
        let application = ready_application(&application_port, selected_store.clone(), 1);
        let deployment =
            unproved_deployment_for_selector(&selector, selected_store.clone(), application);
        let writer = unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]);
        let mut host = ResidentProcessHost::new(
            writer,
            vec![deployment],
            NonZeroUsize::new(1).expect("positive deployment bound"),
            ResidentOperationHost::new(task_state()),
        )
        .expect("one exact Observe deployment");
        let mut authority = ProvingAuthority;
        let exact_checkpoint = input.recovery.operation.context.checkpoint_authority;

        // The lending fixture asserts the exact final custody, session, and
        // anchor read counts after this callback returns. Therefore any
        // rejected entrypoint call that performs I/O either consumes a real
        // recovery reply early or makes those final counts fail.
        for substituted in authority_axis_substitutions(&selector) {
            let request = request_with_signing_authority(
                input.request,
                integrated_signing_authority(&substituted),
            );
            let substituted_input = observation_input_with(
                &input,
                &request,
                signing_authority,
                &selected_store,
                exact_checkpoint,
            );
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                substituted_input,
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObservationError::Selection(
                    ResidentProcessHostSelectionError::CurrentRequestAuthority
                ))
            ));
        }

        for substituted in authority_axis_substitutions(&selector) {
            let historical = integrated_signing_authority(&substituted);
            let substituted_input = observation_input_with(
                &input,
                input.request,
                &historical,
                &selected_store,
                exact_checkpoint,
            );
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                substituted_input,
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObservationError::Selection(
                    ResidentProcessHostSelectionError::HistoricalHandleAuthority
                ))
            ));
        }

        let absent_selector = authority_axis_substitutions(&selector)
            .pop()
            .expect("five-axis substitution set is nonempty");
        let absent_authority = integrated_signing_authority(&absent_selector);
        let absent_request =
            request_with_signing_authority(input.request, absent_authority.clone());
        let absent_input = observation_input_with(
            &input,
            &absent_request,
            &absent_authority,
            &selected_store,
            exact_checkpoint,
        );
        let result = observe_with_unreachable_recovery(
            &mut host,
            &absent_selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &mut authority,
            absent_input,
        );
        assert!(matches!(
            result,
            Err(ResidentProcessObservationError::Selection(
                ResidentProcessHostSelectionError::UnknownDeploymentAuthority
            ))
        ));

        let foreign_recovery_store_id =
            ControlStoreIdV1::parse("foreign-recovery").expect("foreign store identity");
        let foreign_recovery_incarnation =
            ControlIncarnationIdV1::parse("foreign-recovery-incarnation")
                .expect("foreign incarnation identity");
        let recovery_store_substitutions = [
            AuthorityStoreCoordinate::new(
                foreign_recovery_store_id,
                selected_store.incarnation_id().clone(),
            ),
            AuthorityStoreCoordinate::new(
                selected_store.store_id().clone(),
                foreign_recovery_incarnation,
            ),
        ];
        for foreign_recovery_store in &recovery_store_substitutions {
            let substituted_input = observation_input_with(
                &input,
                input.request,
                signing_authority,
                foreign_recovery_store,
                exact_checkpoint,
            );
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                substituted_input,
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObservationError::Selection(
                    ResidentProcessHostSelectionError::RecoveryStoreAssociation
                ))
            ));
        }

        let foreign_checkpoint_store =
            ControlStoreIdV1::parse("foreign-checkpoint-store").expect("foreign store identity");
        let foreign_store_checkpoint = ControlCheckpointSigningAuthorityAdmissionContextV1 {
            control_store_id: &foreign_checkpoint_store,
            ..*exact_checkpoint
        };
        let substituted_input = observation_input_with(
            &input,
            input.request,
            signing_authority,
            &selected_store,
            &foreign_store_checkpoint,
        );
        let result = observe_with_unreachable_recovery(
            &mut host,
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &mut authority,
            substituted_input,
        );
        assert!(matches!(
            result,
            Err(ResidentProcessObservationError::Selection(
                ResidentProcessHostSelectionError::CheckpointAuthorityAssociation
            ))
        ));

        let foreign_checkpoint_incarnation =
            ControlIncarnationIdV1::parse("foreign-checkpoint-incarnation")
                .expect("foreign incarnation identity");
        let foreign_incarnation_checkpoint = ControlCheckpointSigningAuthorityAdmissionContextV1 {
            control_incarnation_id: &foreign_checkpoint_incarnation,
            ..*exact_checkpoint
        };
        let substituted_input = observation_input_with(
            &input,
            input.request,
            signing_authority,
            &selected_store,
            &foreign_incarnation_checkpoint,
        );
        let result = observe_with_unreachable_recovery(
            &mut host,
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &mut authority,
            substituted_input,
        );
        assert!(matches!(
            result,
            Err(ResidentProcessObservationError::Selection(
                ResidentProcessHostSelectionError::CheckpointAuthorityAssociation
            ))
        ));

        let foreign_checkpoint_principal = principal("foreign-checkpoint-principal");
        let foreign_principal_checkpoint = ControlCheckpointSigningAuthorityAdmissionContextV1 {
            principal_id: &foreign_checkpoint_principal,
            ..*exact_checkpoint
        };
        let substituted_input = observation_input_with(
            &input,
            input.request,
            signing_authority,
            &selected_store,
            &foreign_principal_checkpoint,
        );
        let result = observe_with_unreachable_recovery(
            &mut host,
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &mut authority,
            substituted_input,
        );
        assert!(matches!(
            result,
            Err(ResidentProcessObservationError::Selection(
                ResidentProcessHostSelectionError::CheckpointAuthorityAssociation
            ))
        ));

        let foreign_access_store_id =
            ControlStoreIdV1::parse("foreign-access").expect("foreign store identity");
        let foreign_access_incarnation =
            ControlIncarnationIdV1::parse("foreign-access-incarnation")
                .expect("foreign incarnation identity");
        let access_store_substitutions = [
            AuthorityStoreCoordinate::new(
                foreign_access_store_id,
                selected_store.incarnation_id().clone(),
            ),
            AuthorityStoreCoordinate::new(
                selected_store.store_id().clone(),
                foreign_access_incarnation,
            ),
        ];
        for foreign_access_store in &access_store_substitutions {
            let exact_input = observation_input_with(
                &input,
                input.request,
                signing_authority,
                &selected_store,
                exact_checkpoint,
            );
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: foreign_access_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                exact_input,
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObservationError::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation
                ))
            ));
        }

        host.observe(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &mut authority,
            input,
            admission,
        )
        .map_err(|failure| match failure {
            ResidentProcessObservationError::Observe(failure) => failure,
            ResidentProcessObservationError::Selection(failure) => {
                panic!("exact fixture reached process-host selection failure: {failure}")
            }
        })
    });
}

#[test]
fn process_host_observe_preserves_application_authorization_failure() {
    with_denied_process_host_observe_fixture(&mut |fixture: ProcessHostObserveFixture<'_>| {
        let ProcessHostObserveFixture {
            input,
            admission,
            signing_authority,
            custody,
            sessions,
            anchor,
        } = fixture;
        let selected_store = input.recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let application_port = MemoryAuthorityStore::default();
        let application = ready_application(&application_port, selected_store.clone(), 1);
        let deployment =
            unproved_deployment_for_selector(&selector, selected_store.clone(), application);
        let writer = unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]);
        let mut host = ResidentProcessHost::new(
            writer,
            vec![deployment],
            NonZeroUsize::new(1).expect("positive deployment bound"),
            ResidentOperationHost::new(task_state()),
        )
        .expect("one exact Observe deployment");
        let mut authority = ProvingAuthority;

        match host.observe(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &mut authority,
            input,
            admission,
        ) {
            Err(ResidentProcessObservationError::Observe(failure)) => Err(failure),
            Err(ResidentProcessObservationError::Selection(failure)) => {
                panic!("exact denied fixture reached process-host selection failure: {failure}")
            }
            Ok(_) => panic!("denied present authority cannot produce an observation"),
        }
    });
}

#[test]
fn process_host_cancel_extracts_every_authority_coordinate_before_real_recovery() {
    with_process_host_active_cancellation_fixture(
        &mut |fixture: ProcessHostCancellationFixture<'_>| {
            let ProcessHostCancellationFixture {
                input,
                admission,
                signing_authority,
                operation_handle: _,
                custody,
                sessions,
                anchor,
            } = fixture;
            let selected_store = input.recovery.operation.context.store.clone();
            let selector = selector_from_signing_authority(signing_authority);
            let application_port = MemoryAuthorityStore::default();
            let application = ready_application(&application_port, selected_store.clone(), 1);
            let deployment =
                unproved_deployment_for_selector(&selector, selected_store.clone(), application);
            let writer =
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]);
            let mut host = ResidentProcessHost::new(
                writer,
                vec![deployment],
                NonZeroUsize::new(1).expect("positive deployment bound"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one exact Cancel deployment");
            let mut authority = ProvingAuthority;
            let exact_checkpoint = input.recovery.operation.context.checkpoint_authority;

            for substituted in authority_axis_substitutions(&selector) {
                let request = request_with_signing_authority(
                    input.request,
                    integrated_signing_authority(&substituted),
                );
                let substituted_input = cancellation_input_with(
                    &input,
                    &request,
                    signing_authority,
                    &selected_store,
                    exact_checkpoint,
                );
                assert!(matches!(
                    cancel_with_unreachable_recovery(
                        &mut host,
                        &selector,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions,
                            anchor,
                        },
                        &mut authority,
                        substituted_input,
                    ),
                    Err(ResidentProcessCancellationError::Selection(
                        ResidentProcessHostSelectionError::CurrentRequestAuthority
                    ))
                ));
            }

            for substituted in authority_axis_substitutions(&selector) {
                let historical = integrated_signing_authority(&substituted);
                let substituted_input = cancellation_input_with(
                    &input,
                    input.request,
                    &historical,
                    &selected_store,
                    exact_checkpoint,
                );
                assert!(matches!(
                    cancel_with_unreachable_recovery(
                        &mut host,
                        &selector,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions,
                            anchor,
                        },
                        &mut authority,
                        substituted_input,
                    ),
                    Err(ResidentProcessCancellationError::Selection(
                        ResidentProcessHostSelectionError::HistoricalHandleAuthority
                    ))
                ));
            }

            for absent_selector in authority_axis_substitutions(&selector) {
                let absent_authority = integrated_signing_authority(&absent_selector);
                let absent_request =
                    request_with_signing_authority(input.request, absent_authority.clone());
                let absent_input = cancellation_input_with(
                    &input,
                    &absent_request,
                    &absent_authority,
                    &selected_store,
                    exact_checkpoint,
                );
                assert!(matches!(
                    cancel_with_unreachable_recovery(
                        &mut host,
                        &absent_selector,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions,
                            anchor,
                        },
                        &mut authority,
                        absent_input,
                    ),
                    Err(ResidentProcessCancellationError::Selection(
                        ResidentProcessHostSelectionError::UnknownDeploymentAuthority
                    ))
                ));
            }

            let recovery_store_substitutions = [
                AuthorityStoreCoordinate::new(
                    ControlStoreIdV1::parse("foreign-cancel-recovery").unwrap(),
                    selected_store.incarnation_id().clone(),
                ),
                AuthorityStoreCoordinate::new(
                    selected_store.store_id().clone(),
                    ControlIncarnationIdV1::parse("foreign-cancel-recovery-incarnation").unwrap(),
                ),
            ];
            for recovery_store in &recovery_store_substitutions {
                let substituted_input = cancellation_input_with(
                    &input,
                    input.request,
                    signing_authority,
                    recovery_store,
                    exact_checkpoint,
                );
                assert!(matches!(
                    cancel_with_unreachable_recovery(
                        &mut host,
                        &selector,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions,
                            anchor,
                        },
                        &mut authority,
                        substituted_input,
                    ),
                    Err(ResidentProcessCancellationError::Selection(
                        ResidentProcessHostSelectionError::RecoveryStoreAssociation
                    ))
                ));
            }

            let foreign_checkpoint_store =
                ControlStoreIdV1::parse("foreign-cancel-checkpoint-store").unwrap();
            let foreign_checkpoint_incarnation =
                ControlIncarnationIdV1::parse("foreign-cancel-checkpoint-incarnation").unwrap();
            let foreign_checkpoint_principal = principal("foreign-cancel-checkpoint-principal");
            let checkpoint_substitutions = [
                ControlCheckpointSigningAuthorityAdmissionContextV1 {
                    control_store_id: &foreign_checkpoint_store,
                    ..*exact_checkpoint
                },
                ControlCheckpointSigningAuthorityAdmissionContextV1 {
                    control_incarnation_id: &foreign_checkpoint_incarnation,
                    ..*exact_checkpoint
                },
                ControlCheckpointSigningAuthorityAdmissionContextV1 {
                    principal_id: &foreign_checkpoint_principal,
                    ..*exact_checkpoint
                },
            ];
            for checkpoint in &checkpoint_substitutions {
                let substituted_input = cancellation_input_with(
                    &input,
                    input.request,
                    signing_authority,
                    &selected_store,
                    checkpoint,
                );
                assert!(matches!(
                    cancel_with_unreachable_recovery(
                        &mut host,
                        &selector,
                        ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions,
                            anchor,
                        },
                        &mut authority,
                        substituted_input,
                    ),
                    Err(ResidentProcessCancellationError::Selection(
                        ResidentProcessHostSelectionError::CheckpointAuthorityAssociation
                    ))
                ));
            }

            let access_store_substitutions = [
                AuthorityStoreCoordinate::new(
                    ControlStoreIdV1::parse("foreign-cancel-access").unwrap(),
                    selected_store.incarnation_id().clone(),
                ),
                AuthorityStoreCoordinate::new(
                    selected_store.store_id().clone(),
                    ControlIncarnationIdV1::parse("foreign-cancel-access-incarnation").unwrap(),
                ),
            ];
            for access_store in &access_store_substitutions {
                let exact_input = cancellation_input_with(
                    &input,
                    input.request,
                    signing_authority,
                    &selected_store,
                    exact_checkpoint,
                );
                assert!(matches!(
                    cancel_with_unreachable_recovery(
                        &mut host,
                        &selector,
                        ResidentDeploymentOperationAccess {
                            store: access_store,
                            custody,
                            sessions,
                            anchor,
                        },
                        &mut authority,
                        exact_input,
                    ),
                    Err(ResidentProcessCancellationError::Selection(
                        ResidentProcessHostSelectionError::AccessStoreAssociation
                    ))
                ));
            }

            let completion = local_cancellation_completion(&input);
            host.request_cancellation(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                input,
                completion,
                admission,
            )
            .map(|outcome| match outcome {
                ResidentProcessCancellationOutcome::Operation(outcome) => outcome,
                ResidentProcessCancellationOutcome::A2aDelegateReadStopped { .. } => {
                    panic!("a non-A2A fixture returned an A2A continuation")
                }
            })
            .map_err(|failure| match failure {
                ResidentProcessCancellationError::Cancellation(failure) => failure,
                ResidentProcessCancellationError::Selection(_) => {
                    panic!("exact fixture reached process-host selection failure")
                }
                ResidentProcessCancellationError::DispatchedCustody(_) => {
                    panic!("exact non-dispatched fixture retained dispatched custody")
                }
                ResidentProcessCancellationError::Custody(_) => {
                    panic!("successful fixture retained unfinished cancellation custody")
                }
            })
        },
    );
}

#[test]
fn process_host_cancel_preserves_application_authorization_failure() {
    with_denied_process_host_cancellation_fixture(
        &mut |fixture: ProcessHostCancellationFixture<'_>| {
            let ProcessHostCancellationFixture {
                input,
                admission,
                signing_authority,
                operation_handle: _,
                custody,
                sessions,
                anchor,
            } = fixture;
            let selected_store = input.recovery.operation.context.store.clone();
            let selector = selector_from_signing_authority(signing_authority);
            let application_port = MemoryAuthorityStore::default();
            let application = ready_application(&application_port, selected_store.clone(), 1);
            let deployment =
                unproved_deployment_for_selector(&selector, selected_store.clone(), application);
            let writer =
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]);
            let mut host = ResidentProcessHost::new(
                writer,
                vec![deployment],
                NonZeroUsize::new(1).expect("positive deployment bound"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one exact Cancel deployment");
            let mut authority = ProvingAuthority;
            let completion = local_cancellation_completion(&input);
            match host.request_cancellation(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                input,
                completion,
                admission,
            ) {
                Err(ResidentProcessCancellationError::Cancellation(failure)) => Err(failure),
                Err(ResidentProcessCancellationError::Selection(_)) => {
                    panic!("exact denied fixture reached process-host selection failure")
                }
                Err(ResidentProcessCancellationError::DispatchedCustody(_)) => {
                    panic!("denied authority reached dispatched custody")
                }
                Err(ResidentProcessCancellationError::Custody(_)) => {
                    panic!("denied authority reached cancellation custody")
                }
                Ok(_) => panic!("denied present authority cannot retain cancellation"),
            }
        },
    );
}

#[test]
fn process_host_cancel_preserves_existing_non_appending_dispositions() {
    with_process_host_cancellation_dispositions(&mut |fixture: ProcessHostCancellationFixture<
        '_,
    >| {
        let ProcessHostCancellationFixture {
            input,
            admission,
            signing_authority,
            operation_handle: _,
            custody,
            sessions,
            anchor,
        } = fixture;
        let selected_store = input.recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let application_port = MemoryAuthorityStore::default();
        let application = ready_application(&application_port, selected_store.clone(), 1);
        let deployment =
            unproved_deployment_for_selector(&selector, selected_store.clone(), application);
        let writer = unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]);
        let mut host = ResidentProcessHost::new(
            writer,
            vec![deployment],
            NonZeroUsize::new(1).expect("positive deployment bound"),
            ResidentOperationHost::new(task_state()),
        )
        .expect("one exact Cancel deployment");
        let mut authority = ProvingAuthority;
        let completion = local_cancellation_completion(&input);
        host.request_cancellation(
            &selector,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody,
                sessions,
                anchor,
            },
            &mut authority,
            input,
            completion,
            admission,
        )
        .map(|outcome| match outcome {
            ResidentProcessCancellationOutcome::Operation(outcome) => outcome,
            ResidentProcessCancellationOutcome::A2aDelegateReadStopped { .. } => {
                panic!("a non-A2A fixture returned an A2A continuation")
            }
        })
        .map_err(|failure| match failure {
            ResidentProcessCancellationError::Cancellation(failure) => failure,
            ResidentProcessCancellationError::Selection(_) => {
                panic!("exact fixture reached process-host selection failure")
            }
            ResidentProcessCancellationError::DispatchedCustody(_) => {
                panic!("non-dispatched disposition retained dispatched custody")
            }
            ResidentProcessCancellationError::Custody(_) => {
                panic!("non-appending disposition retained cancellation custody")
            }
        })
    });
}

#[test]
fn one_operation_owner_survives_selection_of_different_deployments() {
    let first_port = MemoryAuthorityStore::default();
    let second_port = MemoryAuthorityStore::default();
    let principal = principal("principal-a");
    let first_store = store("deployment-a", "incarnation-a");
    let second_store = store("deployment-b", "incarnation-b");
    let first_selector = unproved_selector(principal.clone(), first_store.clone());
    let second_selector = unproved_selector(principal.clone(), second_store.clone());
    let first = unproved_deployment(
        principal.clone(),
        &first_store,
        ready_application(&first_port, first_store.clone(), 1),
    )
    .expect("first unproved witness");
    let second = unproved_deployment(
        principal.clone(),
        &second_store,
        ready_application(&second_port, second_store.clone(), 2),
    )
    .expect("second unproved witness");
    let writer = unproved_writer(principal, vec![first_store.clone(), second_store.clone()]);
    let mut host = ResidentProcessHost::new(
        writer,
        vec![first, second],
        NonZeroUsize::new(2).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("exact deployment set");
    let (mut authority, mut custody, mut anchor) = operation_ports();

    let batch = with_unproved_operation_service_for_test(
        &mut host,
        &first_selector,
        ResidentDeploymentOperationAccess {
            store: &first_store,
            custody: &mut custody,
            sessions: &first_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_store, service| {
            service
                .open_consultation_batch(
                    consultation_task_batch_fixture(
                        "operation/process-host-first",
                        "collection/process-host-first",
                        "process-host-first",
                    ),
                    consultation_limits(),
                )
                .expect("first bounded batch")
        },
    )
    .expect("first deployment selection");

    with_unproved_operation_service_for_test(
        &mut host,
        &second_selector,
        ResidentDeploymentOperationAccess {
            store: &second_store,
            custody: &mut custody,
            sessions: &second_port,
            anchor: &mut anchor,
        },
        &mut authority,
        |_store, service| {
            assert_eq!(service.active_consultation_batches(), 1);
            assert!(service.consultation_batch_mut(&batch).is_some());
            assert_eq!(
                service.open_consultation_batch(
                    consultation_task_batch_fixture(
                        "operation/process-host-second",
                        "collection/process-host-second",
                        "process-host-second",
                    ),
                    consultation_limits(),
                ),
                Err(ResidentConsultationBatchOpenError::Saturated)
            );
        },
    )
    .expect("second deployment selection");

    assert_eq!(custody.mutation_counts(), (0, 0));
    assert_eq!(anchor.access_counts(), (0, 0));
}
