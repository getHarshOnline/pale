//! Private ownership and exact deployment selection for one resident process.
//!
//! This module joins already-proved process-local ownership. Its private program
//! composition retains a caller-acquired writer guard, admits the exact physical
//! writer records, derives deployment associations from the admitted trust
//! frontier, and cold-opens the complete selected store set. It does not discover
//! a listener, endpoint, store, key, principal, deployment, provider, or policy.

use std::num::NonZeroUsize;

use pale_core::records::{
    ArtifactDigest, CanonicalSignatureV1, CapabilityIdV1, CommonRequestEnvelopeDigest,
    CommonRequestEnvelopeV1, ControlCheckpointIdV1,
    ControlCheckpointSigningAuthorityAdmissionContextV1, ControlStateCheckpointDigest,
    ControlStateCheckpointPreimageV1, ControlStateCheckpointV1, ControlStateCheckpointWireV1,
    DeploymentBindingDigest, DeploymentRuntimeAuthorityDigest, EffectClass,
    ImmutableRecordReferenceV1, InvalidResidentRecordV1, InvocationCorrelationV1,
    LeaseBudgetUnitsV1, LeaseTimeoutMillisV1, OperationClassV1, OperationHandleAdmissionContextV1,
    OperationHandlePreimageV1, OperationLeaseIdV1, OperationLeaseRequestV1, OperationLeaseV1,
    OperationObservationDigest, OperationObservationPreimageV1, OperationObservationStateV1,
    OperationObservationWireV1, PackageResourceRefV1, PrincipalResidentWriterFenceStateV1,
    PrincipalResidentWriterFenceV1, ResidentKeyIdV1, ResidentPrincipalIdV1, ResidentRecordRefV1,
    SchemaDigest, SigningAuthorityBindingV1, UnixTimeV1, CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
    OPERATION_HANDLE_V1_SCHEMA_ID, OPERATION_LEASE_V1_SCHEMA_ID,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AntiRollbackAnchorPort, AuthorityStoreCoordinate, CryptographicRandomSourcePort,
    OperationCustodyPort, PrincipalResidentWriterSnapshot, PrincipalWriterGuardPort,
    ProviderInvocationObservation, SessionLeaseCustodyPort,
};

use super::consultation::service::task::{
    ResidentConsultationTerminalJoinPreflightFailure, ResidentConsultationTerminalReleaseFailure,
};
use super::consultation::{
    CompletedConsultationCollection, CompletedResidentConsultationTerminal,
    ConsultationChildDispatchCandidate, ConsultationChildJoinInput,
    ConsultationChildRetentionOutcome, ConsultationCollection, ConsultationCollectionCommitFailure,
    ConsultationCollectionError, ConsultationCollectionLimits, ConsultationRequirementSummary,
    ConsultationStopInput, ConsultationSuccessorCommitInput, RecoveredConsultationCollection,
    ResidentConsultationBatchCoordinate, ResidentConsultationBatchOpenError,
    ResidentConsultationCompletionReady, ResidentConsultationJoinedTerminal,
    ResidentConsultationOpenFailure, ResidentConsultationOpenInput,
    ResidentConsultationStopCommitFailure, ResidentConsultationStopCustody,
    ResidentConsultationStopSelected, ResidentConsultationSupervisionDecision,
    ResidentConsultationTaskLimits, ResidentConsultationTerminalCustody,
    ResidentConsultationTerminalJoinFailure, ResidentConsultationTurnDisposition,
    ResidentConsultationTurnNeedsRecovery,
};
use super::local_operation_coordinator::{
    ResidentLocalOperationCheckpointInput, ResidentLocalOperationCoordinator,
    ResidentLocalOperationStartFailure, ResidentLocalOperationStartInput,
};
use super::local_operation_start::LocalOperationStartDiscoveryAttempt;
use super::operation_custody::{
    operation_request_key, read_operation_start_for_request, OperationRecoveryError,
    OperationStartRecoveryEvidence,
};
use super::operation_dispatch::{
    OperationAppendFailure, OperationAttemptHistoryInput, OwnedProviderInvocationRequest,
    ProviderOperationDispatchInput, ProviderOperationInvokeInput, ProviderOperationInvokeResult,
    RecoveredProviderDispatchRelease, RetainedProviderOperationDispatch,
};
use super::operation_host::ResidentOperationHost;
use super::operation_lease_custody::retain_operation_lease_body;
use super::operation_service::{
    ResidentConsultationCancellationCustody, ResidentConsultationCancellationDisposition,
    ResidentConsultationCancellationFailure, ResidentConsultationCancellationOutcome,
    ResidentConsultationCancellationRequestFailure, ResidentDispatchedCancellationCustody,
    ResidentLocalCancellationCompletionInput, ResidentOperationCancellationAdmission,
    ResidentOperationCancellationFailure, ResidentOperationCancellationInput,
    ResidentOperationCancellationOutcome, ResidentOperationService,
    ResidentProviderDispatchRetentionFailure,
};
use super::operation_start_allocation::{
    FreshOperationStartAllocation, OperationStartAllocation, OperationStartAllocationError,
};
use super::provider_task_service::{
    ResidentProviderCompletionPoll, ResidentProviderCompletionRecoveryRejection,
    ResidentProviderRecordingConflictRejection, ResidentProviderRecordingContinuationRejection,
    ResidentProviderRecordingCustodyRejection, ResidentProviderRecordingFinalizationRejection,
    ResidentProviderTaskAcceptanceFailure, ResidentProviderTaskPoll,
    ResidentProviderTaskRecoveryFailure, ResidentRecoveredProviderTaskAcceptanceFailure,
};
use super::task_owner::{
    ResidentOperationTaskCancellationState, ResidentOperationTaskExecutor,
    ResidentOperationTaskKey, ResidentOperationTaskOwnerInstance,
    ResidentProviderAcceptedAggregateTransition, ResidentProviderTaskAggregateTransitionOutcome,
    ResidentProviderTaskRefusal, ResidentRecoveredProviderTaskAggregateTransitionOutcome,
    ResidentRecoveredProviderTaskRefusal,
};
use super::ResidentApplication;
use super::{
    CompletedOperationObservationCustody, LocalOperationObservationAdmission,
    LocalOperationObservationError, LocalOperationObservationInput, OperationControlObservation,
    OperationControlRecoveryAdmission, OperationObservationRecoveryAdmission,
    OperationObservationRecoveryInput, PreparedLocalChildParent, RecoveredOperationControl,
    RecoveredOperationObservation,
};
use crate::provider_invocation::{
    ProviderInvocationAdmissionContext, ProviderInvocationAdmissionError,
    ProviderInvocationEvidenceAdmission,
};

type ProcessProviderTaskOutput =
    Result<ProviderInvocationObservation, ProviderInvocationAdmissionError>;

#[derive(Clone, Copy)]
enum ResidentProcessProviderLaunchKind {
    Invoke,
    PairwiseDelegate,
}

/// Why one recovered Consult child could not contribute terminal evidence.
#[derive(Debug, thiserror::Error)]
enum ResidentProcessConsultationChildTerminalError {
    /// The selected deployment, parent, child, or retained lineage differed.
    #[error(transparent)]
    Selection(#[from] ResidentProcessHostSelectionError),
    /// The consultation owner rejected the terminal contribution.
    #[error(transparent)]
    Consultation(#[from] ConsultationCollectionError),
}

mod a2a_cancel_task;
mod a2a_delegate;
mod a2a_delegate_recovery_owner;
use a2a_delegate::ResidentProcessA2aDelegatePoll;
mod a2a_get_task;
mod a2a_task_routes;
mod cancellation_recovery;
mod consult_child;
mod consult_child_recovery_sweep;
mod consult_completion;
use consult_completion::{
    ResidentProcessConsultationCompletionBindingFailure, ResidentProcessConsultationCompletionReady,
};
mod consult_provider;
mod consult_recovery;
mod consult_start;
pub(in crate::resident) mod current_request;
mod delegate_recovery_sweep;
mod dispatch_prepared_recovery;
mod expiry;
#[path = "frame_service.rs"]
pub(in crate::resident) mod frame_service;
#[cfg(feature = "interface-test-support")]
pub(in crate::resident) mod interface_conformance;
pub(in crate::resident) mod native_hook_start;
mod native_invoke;
mod observe_request;
mod observe_request_host;
mod operation_locator;
mod operation_recovery_sweep;
mod operation_request_host;
pub(in crate::resident) mod operation_resolution;
mod program;
mod provider_recovery;
mod provider_resolution_recovery;
mod publication;
mod reconciliation_recovery;
mod request_admission;
mod resident_frame_ingress;
pub(in crate::resident) mod restart;
#[path = "process_host/service_assembly.rs"]
pub(in crate::resident) mod service_assembly;
mod start_factories;
pub(in crate::resident) mod work_turn;

/// Maximum request bytes accepted by every private resident frame owner.
pub(in crate::resident) const MAX_RESIDENT_REQUEST_FRAME_BYTES: usize = 262_144;

use a2a_delegate::ResidentProcessA2aDelegateRoutes;
use a2a_task_routes::ResidentProcessA2aTaskRoutes;
use restart::{
    ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError,
    ResidentProcessOperationRestartInput, ResidentProcessRecoveredAttempt,
};

/// Return whether two witnesses name the same complete checkpoint authority.
fn same_checkpoint_authority(
    left: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    right: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
) -> bool {
    left.authority_ref == right.authority_ref
        && left.authority_sha256 == right.authority_sha256
        && left.principal_id == right.principal_id
        && left.control_store_id == right.control_store_id
        && left.control_incarnation_id == right.control_incarnation_id
        && left.checkpoint_domain_id == right.checkpoint_domain_id
        && left.checkpoint_profile_ref == right.checkpoint_profile_ref
        && left.checkpoint_profile_sha256 == right.checkpoint_profile_sha256
        && left.subject_key_id == right.subject_key_id
        && left.subject_key_epoch == right.subject_key_epoch
}

/// Sole-writer ownership retained for the complete process-host lifetime.
struct ReadyPrincipalResidentWriter<WriterGuard> {
    principal_id: ResidentPrincipalIdV1,
    opened_deployment_stores: Box<[AuthorityStoreCoordinate]>,
    held_writer: WriterGuard,
}

impl<WriterGuard> ReadyPrincipalResidentWriter<WriterGuard> {
    /// Join a held native guard to exact admitted current-writer evidence.
    ///
    /// The record-specific admission owner must authenticate both physical
    /// records before this pure join. This function reproduces their canonical
    /// bytes, rejects a retired fence, and retains the guard beside the complete
    /// sorted deployment-store set.
    fn recover(
        held_writer: WriterGuard,
        snapshot: &PrincipalResidentWriterSnapshot,
        admitted: AdmittedCurrentPrincipalWriter,
        mut opened_deployment_stores: Vec<AuthorityStoreCoordinate>,
    ) -> Result<Self, ResidentPrincipalWriterJoinError>
    where
        WriterGuard: PrincipalWriterGuardPort,
    {
        if held_writer.principal_id() != snapshot.binding().principal_id()
            || admitted.writer.principal_id() != snapshot.binding().principal_id()
            || snapshot.head().principal_id() != snapshot.binding().principal_id()
        {
            return Err(ResidentPrincipalWriterJoinError::PrincipalAssociation);
        }
        if snapshot.head().state() != PrincipalResidentWriterFenceStateV1::Active
            || admitted.writer.state() != &PrincipalResidentWriterFenceStateV1::Active
        {
            return Err(ResidentPrincipalWriterJoinError::InactiveWriter);
        }
        let writer = super::record_projection::record(
            snapshot.writer_record().coordinate().clone(),
            &admitted.writer,
        )?;
        let checkpoint = super::record_projection::record(
            snapshot.checkpoint_record().coordinate().clone(),
            &admitted.checkpoint,
        )?;
        if &writer != snapshot.writer_record()
            || &checkpoint != snapshot.checkpoint_record()
            || snapshot.head().coordinate() != writer.coordinate()
            || snapshot.head().checkpoint().coordinate() != checkpoint.coordinate()
            || snapshot.head().writer_fence_id() != admitted.writer.writer_fence_id()
            || snapshot.head().writer_generation() != *admitted.writer.writer_generation()
        {
            return Err(ResidentPrincipalWriterJoinError::PhysicalRecordAssociation);
        }
        opened_deployment_stores.sort();
        if opened_deployment_stores
            .windows(2)
            .any(|pair| pair[0] == pair[1])
            || opened_deployment_stores.windows(2).any(|pair| {
                pair[0].store_id() == pair[1].store_id()
                    && pair[0].incarnation_id() != pair[1].incarnation_id()
            })
        {
            return Err(ResidentPrincipalWriterJoinError::OpenedStoreSet);
        }
        Ok(Self {
            principal_id: snapshot.binding().principal_id().clone(),
            opened_deployment_stores: opened_deployment_stores.into_boxed_slice(),
            held_writer,
        })
    }

    /// Borrow the principal whose stores the retained writer protects.
    const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }

    /// Borrow the exact deployment-store incarnations admitted at acquisition.
    fn opened_deployment_stores(&self) -> &[AuthorityStoreCoordinate] {
        &self.opened_deployment_stores
    }
}

/// Record-specific admission output for one physical current-writer snapshot.
struct AdmittedCurrentPrincipalWriter {
    writer: PrincipalResidentWriterFenceV1,
    checkpoint: ControlStateCheckpointV1,
}

/// Authority boundary that admits the two exact current-writer records.
trait CurrentPrincipalWriterAdmissionPort {
    /// Admit the signed writer fence and the checkpoint named by its physical head.
    fn admit_current_writer(
        &mut self,
        snapshot: &PrincipalResidentWriterSnapshot,
    ) -> Result<AdmittedCurrentPrincipalWriter, InvalidResidentRecordV1>;
}

impl<F> CurrentPrincipalWriterAdmissionPort for F
where
    F: FnMut(
        &PrincipalResidentWriterSnapshot,
    ) -> Result<AdmittedCurrentPrincipalWriter, InvalidResidentRecordV1>,
{
    fn admit_current_writer(
        &mut self,
        snapshot: &PrincipalResidentWriterSnapshot,
    ) -> Result<AdmittedCurrentPrincipalWriter, InvalidResidentRecordV1> {
        self(snapshot)
    }
}

/// Why a held writer guard and admitted physical head do not form one owner.
#[derive(Debug, thiserror::Error)]
enum ResidentPrincipalWriterJoinError {
    /// Native guard, physical store, head, and admitted record name unlike principals.
    #[error("held writer guard and admitted writer evidence name different principals")]
    PrincipalAssociation,
    /// The current writer is cleanly retired and cannot own a live process.
    #[error("current principal writer fence is not active")]
    InactiveWriter,
    /// Admitted typed records do not reproduce the exact physical snapshot.
    #[error("admitted current-writer records do not reproduce the physical snapshot")]
    PhysicalRecordAssociation,
    /// The selected opened-store set repeats a store or mixes incarnations.
    #[error("opened deployment-store set is not exact")]
    OpenedStoreSet,
    /// Canonical physical projection of an admitted record failed.
    #[error(transparent)]
    Projection(#[from] super::record_projection::ControlRecordProjectionError),
}

/// Exact admitted association retained beside one recovered deployment application.
///
/// The binding and runtime-authority references and digests, principal, and
/// physical store are its protected scope.
/// This private coordinate is not a second durable record or admission proof.
struct AdmittedResidentDeploymentAssociation {
    principal_id: ResidentPrincipalIdV1,
    store: AuthorityStoreCoordinate,
    deployment_binding_ref: ResidentRecordRefV1,
    deployment_binding_sha256: DeploymentBindingDigest,
    runtime_authority_ref: ResidentRecordRefV1,
    runtime_authority_sha256: DeploymentRuntimeAuthorityDigest,
}

impl AdmittedResidentDeploymentAssociation {
    /// Derive one private runtime selector from an admitted trust frontier member.
    fn from_recovered(
        recovered: &super::principal_trust_frontier::RecoveredDeploymentRuntimeAuthority,
    ) -> Self {
        let authority = recovered.authority();
        Self {
            principal_id: authority.principal_id().clone(),
            store: AuthorityStoreCoordinate::new(
                authority.control_store_id().clone(),
                authority.control_incarnation_id().clone(),
            ),
            deployment_binding_ref: recovered.binding_ref().clone(),
            deployment_binding_sha256: recovered.binding().canonical_binding_sha256().clone(),
            runtime_authority_ref: recovered.record_ref().clone(),
            runtime_authority_sha256: authority
                .canonical_deployment_runtime_authority_sha256()
                .clone(),
        }
    }

    /// Borrow the principal admitted by the runtime authority.
    const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }

    /// Borrow the exact deployment-store incarnation admitted by the authority.
    const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Return whether this association names one complete admitted authority.
    fn matches(&self, selector: &ResidentDeploymentAuthoritySelector) -> bool {
        self.principal_id == selector.principal_id
            && self.deployment_binding_ref == selector.deployment_binding_ref
            && self.deployment_binding_sha256 == selector.deployment_binding_sha256
            && self.runtime_authority_ref == selector.runtime_authority_ref
            && self.runtime_authority_sha256 == selector.runtime_authority_sha256
    }
}

/// Complete private selector for one admitted deployment-runtime association.
///
/// This coordinate selects already-admitted process state. It is not authority,
/// a discovery result, or permission to inspect the selected deployment. The
/// private program derives it only from a retained admitted association.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResidentDeploymentAuthoritySelector {
    principal_id: ResidentPrincipalIdV1,
    deployment_binding_ref: ResidentRecordRefV1,
    deployment_binding_sha256: DeploymentBindingDigest,
    runtime_authority_ref: ResidentRecordRefV1,
    runtime_authority_sha256: DeploymentRuntimeAuthorityDigest,
}

impl ResidentDeploymentAuthoritySelector {
    /// Derive the selector paired with one admitted deployment association.
    fn from_association(association: &AdmittedResidentDeploymentAssociation) -> Self {
        Self {
            principal_id: association.principal_id.clone(),
            deployment_binding_ref: association.deployment_binding_ref.clone(),
            deployment_binding_sha256: association.deployment_binding_sha256.clone(),
            runtime_authority_ref: association.runtime_authority_ref.clone(),
            runtime_authority_sha256: association.runtime_authority_sha256.clone(),
        }
    }

    /// Return whether one signing-authority binding names this exact deployment.
    fn matches_signing_authority(&self, binding: &SigningAuthorityBindingV1) -> bool {
        let Some(authority) = binding.integrated_runtime_authority() else {
            return false;
        };
        binding.principal_id() == &self.principal_id
            && authority.deployment_binding_ref == self.deployment_binding_ref
            && authority.deployment_binding_sha256.as_str()
                == self.deployment_binding_sha256.as_str()
            && authority.deployment_runtime_authority_ref == self.runtime_authority_ref
            && authority.deployment_runtime_authority_sha256.as_str()
                == self.runtime_authority_sha256.as_str()
    }

    /// Return whether current session authority names this exact deployment.
    fn matches_current_session(&self, current: &super::AmbientSessionAdmissionContext<'_>) -> bool {
        let Some(runtime) = current.runtime_authority else {
            return false;
        };
        current.deployment_binding_ref == &self.deployment_binding_ref
            && current.deployment_binding.canonical_binding_sha256()
                == &self.deployment_binding_sha256
            && runtime.authority_ref == &self.runtime_authority_ref
            && runtime
                .authority
                .canonical_deployment_runtime_authority_sha256()
                .as_str()
                == self.runtime_authority_sha256.as_str()
            && runtime.authority.principal_id() == &self.principal_id
    }

    /// Return whether one issued handle belongs to this deployment authority.
    fn matches_operation(&self, operation: &super::IssuedLocalOperation) -> bool {
        self.matches_handle(operation.handle().1)
    }

    /// Return whether one admitted handle belongs to this deployment authority.
    fn matches_handle(&self, handle: &pale_core::records::OperationHandleV1) -> bool {
        handle.deployment_runtime_authority_ref() == &self.runtime_authority_ref
            && handle.deployment_runtime_authority_sha256().as_str()
                == self.runtime_authority_sha256.as_str()
            && handle.issuer_principal_id() == &self.principal_id
    }

    /// Return whether an admitted session names this deployment authority.
    fn matches_admitted_session(&self, session: &super::AdmittedAmbientSession) -> bool {
        let lease = session.lease();
        lease.deployment_runtime_authority_ref() == &self.runtime_authority_ref
            && lease.deployment_runtime_authority_sha256().as_str()
                == self.runtime_authority_sha256.as_str()
            && lease.issuer_principal_id() == &self.principal_id
    }
}

/// One preallocated process-host route slot before provider acceptance.
struct ResidentProcessProviderRouteReservation {
    owner_instance: super::task_owner::ResidentOperationTaskOwnerInstance,
    selector: ResidentDeploymentAuthoritySelector,
}

/// Exact route retained for a provider task or its unsettled completion.
struct ResidentProcessProviderRoute {
    key: ResidentOperationTaskKey,
    selector: ResidentDeploymentAuthoritySelector,
}

/// Linear route custody retained after provider progress leaves task ownership.
#[must_use = "release only after exact durable provider settlement"]
struct ResidentProcessProviderRouteCustody {
    owner_instance: super::task_owner::ResidentOperationTaskOwnerInstance,
    key: ResidentOperationTaskKey,
    selector: ResidentDeploymentAuthoritySelector,
}

/// Bounded process-local join from accepted provider tasks to deployments.
///
/// The vector reserves its complete capacity at host construction. A route
/// reservation is acquired before executor acceptance, so a successful
/// acceptance never needs a fallible allocation to remain routable.
struct ResidentProcessProviderRoutes {
    owner_instance: super::task_owner::ResidentOperationTaskOwnerInstance,
    maximum_outstanding: usize,
    reserved: usize,
    checked_out: usize,
    active: Vec<ResidentProcessProviderRoute>,
}

impl ResidentProcessProviderRoutes {
    fn new(
        owner_instance: super::task_owner::ResidentOperationTaskOwnerInstance,
        limits: super::task_owner::ResidentOperationTaskLimits,
    ) -> Result<Self, ResidentProcessHostBuildError> {
        let maximum_outstanding = limits.maximum_in_flight().get();
        let mut active = Vec::new();
        active
            .try_reserve_exact(maximum_outstanding)
            .map_err(|_| ResidentProcessHostBuildError::ProviderRouteAllocation)?;
        Ok(Self {
            owner_instance,
            maximum_outstanding,
            reserved: 0,
            checked_out: 0,
            active,
        })
    }

    fn reserve(
        &mut self,
        selector: ResidentDeploymentAuthoritySelector,
    ) -> Result<ResidentProcessProviderRouteReservation, ()> {
        if self.active.len() + self.reserved + self.checked_out == self.maximum_outstanding {
            return Err(());
        }
        self.reserved += 1;
        Ok(ResidentProcessProviderRouteReservation {
            owner_instance: self.owner_instance.clone(),
            selector,
        })
    }

    fn commit(
        &mut self,
        reservation: ResidentProcessProviderRouteReservation,
        key: ResidentOperationTaskKey,
    ) -> Result<(), Box<ResidentProcessProviderRouteReservation>> {
        if !reservation.owner_instance.is_same(&self.owner_instance) {
            return Err(Box::new(reservation));
        }
        debug_assert!(self.reserved > 0);
        debug_assert!(
            self.active.len() + self.reserved + self.checked_out <= self.maximum_outstanding
        );
        debug_assert!(self.active.iter().all(|route| route.key != key));
        self.reserved -= 1;
        self.active.push(ResidentProcessProviderRoute {
            key,
            selector: reservation.selector,
        });
        Ok(())
    }

    fn release_reservation(
        &mut self,
        reservation: ResidentProcessProviderRouteReservation,
    ) -> Result<(), Box<ResidentProcessProviderRouteReservation>> {
        if !reservation.owner_instance.is_same(&self.owner_instance) || self.reserved == 0 {
            return Err(Box::new(reservation));
        }
        self.reserved -= 1;
        Ok(())
    }

    fn selector(
        &self,
        key: &ResidentOperationTaskKey,
    ) -> Option<&ResidentDeploymentAuthoritySelector> {
        self.active
            .iter()
            .find(|route| &route.key == key)
            .map(|route| &route.selector)
    }

    fn check_out(
        &mut self,
        key: &ResidentOperationTaskKey,
    ) -> Option<ResidentProcessProviderRouteCustody> {
        let index = self.active.iter().position(|route| &route.key == key)?;
        let route = self.active.remove(index);
        self.checked_out += 1;
        Some(ResidentProcessProviderRouteCustody {
            owner_instance: self.owner_instance.clone(),
            key: route.key,
            selector: route.selector,
        })
    }

    fn release_custody(
        &mut self,
        custody: ResidentProcessProviderRouteCustody,
    ) -> Result<(), Box<ResidentProcessProviderRouteCustody>> {
        if !custody.owner_instance.is_same(&self.owner_instance) || self.checked_out == 0 {
            return Err(Box::new(custody));
        }
        self.checked_out -= 1;
        Ok(())
    }
}

/// Store-bound ports acquired by the future authenticated attachment owner.
///
/// The private value records the store those ports are claimed to serve. Its
/// absent production constructor is intentional: this module cannot establish
/// the native association, authenticate a client, or grant disclosure access.
/// The process host only rejects an unlike claim before it lends any port.
struct ResidentDeploymentOperationAccess<'a> {
    store: &'a AuthorityStoreCoordinate,
    custody: &'a mut dyn OperationCustodyPort,
    sessions: &'a dyn SessionLeaseCustodyPort,
    anchor: &'a mut dyn AntiRollbackAnchorPort,
}

/// Store-bound owners for advancing one authenticated recovered head.
///
/// Reconciliation needs neither current session custody nor a compiled
/// generation. The recovered operation already carries its admitted historical
/// request and lease associations.
struct ResidentDeploymentReconciliationAccess<'a> {
    store: &'a AuthorityStoreCoordinate,
    custody: &'a mut dyn OperationCustodyPort,
    anchor: &'a mut dyn AntiRollbackAnchorPort,
}

/// Store-bound custody access for one read-only start-allocation decision.
///
/// This deliberately excludes ambient authority, session custody, and the
/// independent anchor. The allocation phase can therefore inspect one exact
/// request and draw fresh identities, but it cannot admit, retain, sign,
/// commit, anchor, or dispatch an operation.
struct ResidentDeploymentOperationLookup<'a> {
    store: &'a AuthorityStoreCoordinate,
    custody: &'a mut dyn OperationCustodyPort,
}

/// Store-bound custody access for retaining one already-admitted lease body.
///
/// This capsule permits only the exact immutable lease write owned by the
/// lease-retention phase. It exposes no session store, anchor, or task owner.
struct ResidentDeploymentOperationLeaseAccess<'a> {
    store: &'a AuthorityStoreCoordinate,
    custody: &'a mut dyn OperationCustodyPort,
}

/// Admitted coordinates and bounds shared by local operation start preflight.
///
/// This borrowing capsule excludes every allocated identity. It carries only
/// the exact authority associations that the process host can check before the
/// request lookup or first entropy draw.
#[derive(Clone, Copy)]
struct ResidentProcessOperationStartContext<'a> {
    session: &'a super::AdmittedAmbientSession,
    current: &'a super::AmbientSessionAdmissionContext<'a>,
    request: &'a CommonRequestEnvelopeV1,
    handle_signing_authority: &'a SigningAuthorityBindingV1,
    semantic_checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    issuance_checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    maximum_record_bytes: usize,
}

/// Selector-bound result of one request-first operation allocation.
#[must_use = "preserve the exact existing evidence or fresh allocation"]
enum ResidentProcessOperationStartAllocation {
    /// Physical evidence already exists and grants no issued-operation authority.
    Existing(Box<ResidentProcessExistingOperationStart>),
    /// Fresh identities remain inert under their exact process and deployment owner.
    Fresh(Box<ResidentProcessFreshOperationStart>),
}

/// Existing operation evidence found through an exact request lookup.
///
/// The recovered operation can have another admitted class. This value grants
/// no continuation, cancellation, or carrier authority.
struct ResidentProcessExistingOperationStart {
    selector: ResidentDeploymentAuthoritySelector,
    evidence: Box<OperationStartRecoveryEvidence>,
}

/// Fresh linear identity allocation before lease, signing, or custody work.
#[must_use = "complete or explicitly retain this exact fresh allocation"]
struct ResidentProcessFreshOperationStart {
    owner_instance: ResidentOperationTaskOwnerInstance,
    selector: ResidentDeploymentAuthoritySelector,
    request_ref: ResidentRecordRefV1,
    request_sha256: CommonRequestEnvelopeDigest,
    maximum_record_bytes: usize,
    allocation: Box<FreshOperationStartAllocation>,
}

/// Application-derived lease policy without an operation-lease identity.
///
/// The process host supplies the allocated identity. Keeping it out of this
/// basis prevents a caller from substituting another lease while reusing the
/// selector-bound allocation.
#[derive(Clone)]
struct ResidentProcessOperationLeaseBasis {
    capabilities: Vec<CapabilityIdV1>,
    effects: Vec<EffectClass>,
    read_set: Vec<ResidentRecordRefV1>,
    evidence_obligations: Vec<ResidentRecordRefV1>,
    budget_units: LeaseBudgetUnitsV1,
    timeout_millis: LeaseTimeoutMillisV1,
    expires_at: UnixTimeV1,
    invocation_correlation: InvocationCorrelationV1,
}

/// Bounded root-Invoke policy selected by the retained application owner.
///
/// The request supplies none of these limits. The operation identity is also
/// deliberately absent: it comes only from the signed request after the exact
/// retained generation proves that the operation exists.
struct ResidentProcessInvokePolicy {
    checkpoint_profile: super::checkpoint_profile::SupportedOperationCheckpointProfile,
    publication_policy: super::LocalOperationPublicationPolicy,
    maximum_record_bytes: usize,
    lease: ResidentProcessOperationLeaseBasis,
    handle_expires_at: UnixTimeV1,
    handle_retain_until: UnixTimeV1,
    accumulator_limits: pale_ports::AccumulatorPreparationLimits,
}

/// Bounded root-Consult policy selected by the retained application owner.
///
/// This policy carries only compiled operation limits and an ordered,
/// identity-free roster. The signed request still supplies the exact parent
/// operation; the process host allocates every durable identity after replay
/// admission.
struct ResidentProcessConsultPolicy {
    start: start_factories::ResidentStartConsultPolicy<ResidentProcessConsultFreshChildPolicy>,
    lease: ResidentProcessOperationLeaseBasis,
}

/// Identity-free meaning for one fresh child selected in a Consult roster.
///
/// The signed request and compiled limits are retained for later admission.
/// This value carries no child identity, parent reference, private start intent,
/// custody state, route, or signer.
#[derive(Clone)]
struct ResidentProcessConsultFreshChildPolicy {
    request: Box<CommonRequestEnvelopeV1>,
    operation_class: OperationClassV1,
    start: start_factories::ResidentCompiledOperationStartPolicy,
    lease: ResidentProcessOperationLeaseBasis,
}

/// Bounded pairwise-Delegate policy selected by the retained application owner.
struct ResidentProcessDelegatePolicy {
    start: start_factories::ResidentStartDelegatePolicy,
    lease: ResidentProcessOperationLeaseBasis,
}

/// Exact admitted facts visible to the application-owned Invoke policy.
struct ResidentProcessInvokePolicyContext<'a> {
    request: &'a CommonRequestEnvelopeV1,
    session: &'a super::AdmittedAmbientSession,
    operation: &'a crate::semantic::ResolvedOperation,
}

/// Exact admitted facts visible to the application-owned Consult policy.
struct ResidentProcessConsultPolicyContext<'a> {
    request: &'a CommonRequestEnvelopeV1,
    session: &'a super::AdmittedAmbientSession,
    operation: &'a crate::semantic::ResolvedOperation,
}

/// Exact admitted facts visible to the application-owned Delegate policy.
struct ResidentProcessDelegatePolicyContext<'a> {
    request: &'a CommonRequestEnvelopeV1,
    session: &'a super::AdmittedAmbientSession,
    operation: &'a crate::semantic::ResolvedOperation,
    parent: &'a PreparedLocalChildParent,
}

/// Mandatory retained policy owner for one root Invoke.
///
/// This is an application-private policy seam, not a request callback. It can
/// narrow only the already-admitted session and compiled operation and returns
/// no identity, operation coordinate, provider route, or storage choice.
trait ResidentProcessInvokePolicyEvaluator {
    fn evaluate_invoke(
        &mut self,
        context: ResidentProcessInvokePolicyContext<'_>,
    ) -> Result<ResidentProcessInvokePolicy, ResidentProcessHostSelectionError>;
}

/// Mandatory retained policy owner for one root Consult.
///
/// The evaluator can narrow only an already admitted session and exact compiled
/// operation. It returns no identity, store, signer, route, or provider choice.
trait ResidentProcessConsultPolicyEvaluator {
    fn evaluate_consult(
        &mut self,
        context: ResidentProcessConsultPolicyContext<'_>,
    ) -> Result<ResidentProcessConsultPolicy, ResidentProcessHostSelectionError>;
}

/// Mandatory retained policy owner for one pairwise Delegate child.
trait ResidentProcessDelegatePolicyEvaluator {
    fn evaluate_delegate(
        &mut self,
        context: ResidentProcessDelegatePolicyContext<'_>,
    ) -> Result<ResidentProcessDelegatePolicy, ResidentProcessHostSelectionError>;
}

impl ResidentProcessOperationLeaseBasis {
    /// Join policy to the signed operation and the freshly allocated lease identity.
    fn into_request(
        self,
        operation_lease_id: OperationLeaseIdV1,
        operation: PackageResourceRefV1,
    ) -> OperationLeaseRequestV1 {
        OperationLeaseRequestV1 {
            operation_lease_id,
            operation,
            capabilities: self.capabilities,
            effects: self.effects,
            read_set: self.read_set,
            evidence_obligations: self.evidence_obligations,
            budget_units: self.budget_units,
            timeout_millis: self.timeout_millis,
            expires_at: self.expires_at,
            invocation_correlation: self.invocation_correlation,
        }
    }
}

/// Non-identity meaning needed to turn one retained lease into a root Invoke.
///
/// Record identities, schema digests, request binding, issuer coordinates,
/// current time, and the fixed root-Invoke class are derived by the process
/// host. They cannot be supplied through this basis.
struct ResidentProcessInvokeStartBasis<'a> {
    checkpoint_profile: &'a super::checkpoint_profile::SupportedOperationCheckpointProfile,
    request: &'a CommonRequestEnvelopeV1,
    session: &'a super::AdmittedAmbientSession,
    current: &'a super::AmbientSessionAdmissionContext<'a>,
    publication_policy: super::LocalOperationPublicationPolicyInput<'a>,
    handle_signing_authority: &'a SigningAuthorityBindingV1,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    handle_expires_at: UnixTimeV1,
    handle_retain_until: UnixTimeV1,
    accumulator_limits: pale_ports::AccumulatorPreparationLimits,
}

/// Input for one request-first root Invoke.
struct ResidentProcessRootInput<'a> {
    request: &'a CommonRequestEnvelopeV1,
    session: &'a super::AdmittedAmbientSession,
    current: &'a super::AmbientSessionAdmissionContext<'a>,
    handle_signing_authority: &'a SigningAuthorityBindingV1,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    fresh_start_admission: ResidentProcessFreshStartAdmission,
}

/// Process-local capacity available after the durable duplicate lookup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentProcessFreshStartAdmission {
    /// A previously absent request may enter the fresh-start path.
    Available,
    /// Existing work remains observable, but no new start may be retained.
    CapacityExhausted,
}

/// An admitted child lease that has not yet proved exact physical retention.
#[must_use = "retain or explicitly preserve this exact admitted lease"]
struct ResidentProcessAdmittedOperationLease {
    fresh: Box<ResidentProcessFreshOperationStart>,
    lease: OperationLeaseV1,
}

/// Exact selector-bound lease body retained before durable operation start.
#[must_use = "start or explicitly preserve this exact retained lease"]
struct ResidentProcessRetainedOperationLease {
    fresh: Box<ResidentProcessFreshOperationStart>,
    lease: OperationLeaseV1,
}

/// Parent evidence protected before a Delegate request can allocate identities.
#[must_use = "continue this exact protected parent into Delegate allocation"]
struct ResidentProcessProtectedDelegateParent {
    selector: ResidentDeploymentAuthoritySelector,
    store: AuthorityStoreCoordinate,
    parent: PreparedLocalChildParent,
    lease: OperationLeaseV1,
}

/// Fresh Delegate identities paired with the parent protected before allocation.
#[must_use = "admit and retain this exact Delegate lease or preserve the phase"]
struct ResidentProcessFreshDelegateOperationStart {
    fresh: Box<ResidentProcessFreshOperationStart>,
    parent: Box<ResidentProcessProtectedDelegateParent>,
}

/// Delegate lease custody that retains the same previously protected parent.
#[must_use = "start the exact Delegate child with this retained parent and lease"]
struct ResidentProcessRetainedDelegateOperationLease {
    retained: ResidentProcessRetainedOperationLease,
    parent: Box<ResidentProcessProtectedDelegateParent>,
}

/// Result of a Delegate lookup after its exact parent was protected.
#[must_use = "preserve existing evidence or the exact protected fresh phase"]
enum ResidentProcessDelegateOperationStartAllocation {
    Existing {
        existing: Box<ResidentProcessExistingOperationStart>,
        parent: Box<ResidentProcessProtectedDelegateParent>,
    },
    Fresh(Box<ResidentProcessFreshDelegateOperationStart>),
}

/// Lease lineage checked after admission and before physical custody.
enum ResidentProcessOperationLeaseLineage<'a> {
    Independent,
    DelegateChild(&'a OperationLeaseV1),
}

/// Failure while converting one fresh allocation into a retained lease body.
#[must_use = "preserve the returned exact fresh or admitted phase"]
enum ResidentProcessOperationLeaseFailure {
    /// A process, deployment, request, signer, or checkpoint fence failed.
    Selection {
        pending: Box<ResidentProcessFreshOperationStart>,
        reason: ResidentProcessHostSelectionError,
    },
    /// The embedded operation-lease schema could not be resolved.
    Contract {
        pending: Box<ResidentProcessFreshOperationStart>,
        reason: pale_contracts::SchemaError,
    },
    /// Current ambient authority rejected the allocated operation lease.
    Admission {
        pending: Box<ResidentProcessFreshOperationStart>,
        reason: super::AmbientSessionAdmissionError,
    },
    /// An admitted Delegate lease did not narrow its protected parent lease.
    Narrowing {
        pending: Box<ResidentProcessAdmittedOperationLease>,
        reason: InvalidResidentRecordV1,
    },
    /// Exact lease retention was not proved; the admitted owner is preserved.
    Custody {
        pending: Box<ResidentProcessAdmittedOperationLease>,
        reason: super::OperationCustodyDatabaseFailureReason,
    },
}

/// Delegate parent protection failed before operation identity allocation.
enum ResidentProcessDelegateParentProtectionError {
    Selection(ResidentProcessHostSelectionError),
    Protection(super::LocalOperationStartError),
}

/// Delegate allocation failed while preserving the already protected parent.
#[must_use = "preserve the exact protected parent on allocation failure"]
struct ResidentProcessDelegateOperationAllocationFailure {
    parent: Box<ResidentProcessProtectedDelegateParent>,
    reason: ResidentProcessOperationAllocationError,
}

/// Delegate lease work failed while preserving its protected parent.
#[must_use = "preserve the protected parent and exact returned lease phase"]
struct ResidentProcessDelegateOperationLeaseFailure {
    parent: Box<ResidentProcessProtectedDelegateParent>,
    reason: ResidentProcessOperationLeaseFailure,
}

/// Allocation stopped before returning an existing or fresh selector-bound phase.
#[derive(Debug)]
enum ResidentProcessOperationAllocationError {
    /// Deployment or authority association failed before request custody access.
    Selection(ResidentProcessHostSelectionError),
    /// The sole request lookup or bounded entropy allocation failed.
    Allocation(OperationStartAllocationError),
}

/// Request-first outcome for one root Invoke.
#[must_use = "preserve the fresh allocation or the exact existing disposition"]
enum ResidentProcessInvokeRequestResolution {
    /// No durable start exists for the request; the returned allocation is inert.
    FreshStart(Box<ResidentProcessFreshOperationStart>),
    /// An existing operation was recovered and classified without resuming it.
    ExistingOperation(Box<ResidentProcessOperationRestartDisposition>),
}

/// Failure before one Invoke request has a fresh or existing disposition.
#[derive(Debug)]
enum ResidentProcessInvokeRequestResolutionError {
    /// Request lookup or fresh identity allocation failed.
    Allocation(ResidentProcessOperationAllocationError),
    /// Existing operation recovery or classification failed.
    ExistingOperation(ResidentProcessOperationRestartError),
}

/// A new durable start or the exact existing operation.
#[must_use = "preserve the issued operation or exact existing disposition"]
enum ResidentProcessRootOutcome {
    /// A previously absent request now has complete durable start custody.
    Started(Box<super::IssuedLocalOperation>),
    /// Existing work was classified without lease admission or restart.
    ExistingOperation(Box<ResidentProcessOperationRestartDisposition>),
}

/// A new durable Consult parent or the exact existing operation.
#[must_use = "preserve the consultation phase or exact existing disposition"]
enum ResidentProcessConsultRootOutcome {
    /// A previously absent request now owns one durable Consult start phase.
    Started(Box<consult_start::ResidentProcessConsultationStarted>),
    /// Existing work was classified without allocating or starting another parent.
    ExistingOperation(Box<ResidentProcessOperationRestartDisposition>),
}

/// Admit the exact root class selected before a duplicate request lookup.
fn admit_existing_root_start_kind(
    disposition: ResidentProcessOperationRestartDisposition,
    expected: OperationClassV1,
) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError> {
    let operation = operation_locator::restart_disposition_operation(&disposition);
    admit_existing_root_operation(operation, expected)?;
    Ok(disposition)
}

/// Admit one recovered unparented operation only under the selected root kind.
fn admit_existing_root_operation(
    operation: &super::IssuedLocalOperation,
    expected: OperationClassV1,
) -> Result<(), ResidentProcessOperationRestartError> {
    let handle = operation.handle().1;
    if handle.operation_class() != &expected || handle.parent_operation().is_some() {
        return Err(ResidentProcessOperationRestartError::State);
    }
    Ok(())
}

/// Admit an existing Delegate only under the parent protected for this request.
fn admit_existing_delegate_start(
    disposition: ResidentProcessOperationRestartDisposition,
    selected_parent: &ResidentProcessProtectedDelegateParent,
) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError> {
    let operation = operation_locator::restart_disposition_operation(&disposition);
    admit_existing_delegate_operation(operation, selected_parent)?;
    Ok(disposition)
}

/// Admit one recovered Delegate only under the exact protected parent and lease.
fn admit_existing_delegate_operation(
    operation: &super::IssuedLocalOperation,
    selected_parent: &ResidentProcessProtectedDelegateParent,
) -> Result<(), ResidentProcessOperationRestartError> {
    let (_, handle) = operation.handle();
    let Some(parent) = handle.parent_operation() else {
        return Err(ResidentProcessOperationRestartError::State);
    };
    let (expected_parent_ref, expected_parent_handle) = selected_parent.parent.operation();
    if handle.operation_class() != &OperationClassV1::Delegate
        || operation.consultation_start_intent().is_some()
        || selected_parent.parent.lease() != &selected_parent.lease
        || parent.operation_handle_id != *expected_parent_handle.operation_handle_id()
        || parent.operation_handle_ref != *expected_parent_ref
        || parent.operation_handle_sha256
            != *expected_parent_handle.canonical_operation_handle_sha256()
        || operation
            .historical_operation_lease()
            .verify_against_parent_operation(&selected_parent.lease)
            .is_err()
    {
        return Err(ResidentProcessOperationRestartError::State);
    }
    Ok(())
}

/// The phase that stopped request-first Invoke handling.
#[must_use = "preserve the exact lower-phase failure"]
enum ResidentProcessRootError {
    /// The request is absent and process-local capacity cannot retain new work.
    Capacity,
    /// The exact compiled operation has no retained application policy.
    Policy(ResidentProcessHostSelectionError),
    /// Request lookup, identity allocation, or existing-work classification failed.
    Resolution(ResidentProcessInvokeRequestResolutionError),
    /// Fresh lease admission or retention failed and retained its linear phase.
    Lease(ResidentProcessOperationLeaseFailure),
    /// Durable start failed and retained its lease or exact start phase.
    Start(ResidentProcessAllocatedInvokeStartError),
}

/// The phase that stopped request-first Consult handling.
#[must_use = "preserve the exact lower-phase failure"]
enum ResidentProcessConsultRootError {
    /// The request is absent and process-local capacity cannot retain new work.
    Capacity,
    /// The exact compiled operation has no retained application policy.
    Policy(ResidentProcessHostSelectionError),
    /// Request lookup, identity allocation, or existing-work classification failed.
    Resolution(ResidentProcessInvokeRequestResolutionError),
    /// Fresh lease admission or retention failed and retained its linear phase.
    Lease(ResidentProcessOperationLeaseFailure),
    /// Factory input or durable Consult start failed after lease retention.
    Start(ResidentProcessAllocatedConsultStartError),
}

/// The phase that stopped request-first pairwise Delegate handling.
#[must_use = "preserve the exact lower-phase failure"]
enum ResidentProcessDelegateRootError {
    Capacity,
    Policy(ResidentProcessHostSelectionError),
    Resolution(ResidentProcessOperationRestartError),
    Allocation(ResidentProcessDelegateOperationAllocationFailure),
    Lease(ResidentProcessDelegateOperationLeaseFailure),
    Start(ResidentProcessAllocatedDelegateStartError),
}

/// Failure while starting one allocated Consult without losing its linear phase.
#[must_use = "preserve the retained lease or exact start-phase continuation"]
enum ResidentProcessAllocatedConsultStartError {
    /// The retained inputs did not construct one exact Consult start.
    Factory {
        pending: Box<ResidentProcessRetainedOperationLease>,
        reason: start_factories::ResidentOperationStartFactoryError,
    },
    /// Deployment or store selection failed before the start owner ran.
    Selection {
        pending: Box<ResidentProcessRetainedOperationLease>,
        reason: ResidentProcessHostSelectionError,
    },
    /// Semantic preparation failed before a physical candidate existed.
    Preparation {
        pending: Box<ResidentProcessRetainedOperationLease>,
        reason: super::LocalOperationStartError,
    },
    /// The signed parent is fixed; child admission must consume this exact roster.
    Roster(Box<ResidentProcessConsultationRosterStartContinuation>),
    /// The retained exact phase is the only safe start continuation.
    Continuation(Box<ResidentProcessOperationStartContinuation>),
}

/// Failure while starting one allocated Delegate without losing its parent or lease.
#[must_use = "preserve the protected parent, retained lease, or start-phase continuation"]
enum ResidentProcessAllocatedDelegateStartError {
    Factory {
        pending: Box<ResidentProcessRetainedDelegateOperationLease>,
        reason: start_factories::ResidentOperationStartFactoryError,
    },
    Selection {
        pending: Box<ResidentProcessRetainedDelegateOperationLease>,
        reason: ResidentProcessHostSelectionError,
    },
    Preparation {
        pending: Box<ResidentProcessRetainedDelegateOperationLease>,
        reason: super::LocalOperationStartError,
    },
    Continuation(Box<ResidentProcessOperationStartContinuation>),
}

impl From<ResidentProcessHostSelectionError> for ResidentProcessOperationAllocationError {
    fn from(reason: ResidentProcessHostSelectionError) -> Self {
        Self::Selection(reason)
    }
}

/// Exact fresh allocation returned unchanged when its process fence fails.
#[must_use = "retry only with the returned exact allocation"]
struct ResidentProcessOperationAllocationFenceFailure {
    pending: Box<ResidentProcessFreshOperationStart>,
    reason: ResidentProcessHostSelectionError,
}

/// Existing operation-control coordinates that must belong to one deployment.
///
/// The values remain owned and admitted by their existing semantic owners. This
/// private borrowing capsule only prevents one composition call from mixing
/// request, historical handle, recovery-store, and checkpoint scopes.
struct ResidentOperationControlAssociation<'a> {
    current_request_authority: &'a SigningAuthorityBindingV1,
    historical_handle_authority: &'a SigningAuthorityBindingV1,
    recovery_store: &'a AuthorityStoreCoordinate,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
}

/// One recovered application whose principal and runtime association is retained.
///
/// The application remains the durable authority owner for its exact store. A
/// future production owner must construct this wrapper only after joining the
/// cold-open result to the active principal writer and admitted deployment
/// runtime authority. This type does not perform that admission.
struct ReadyResidentDeployment<'store> {
    association: AdmittedResidentDeploymentAssociation,
    application: Box<ResidentApplication<'store>>,
}

impl ReadyResidentDeployment<'_> {
    /// Borrow the exact store coordinate from the application itself.
    ///
    /// No caller-supplied key is retained, so a substituted coordinate cannot
    /// redirect this application.
    fn store(&self) -> &AuthorityStoreCoordinate {
        self.application.store()
    }

    /// Join an already-admitted association to its recovered application.
    ///
    /// This constructor admits no authority. The private program derives the
    /// association from the admitted current-principal trust frontier before it
    /// calls this join.
    fn new<'store>(
        association: AdmittedResidentDeploymentAssociation,
        application: Box<ResidentApplication<'store>>,
    ) -> Result<ReadyResidentDeployment<'store>, ResidentDeploymentJoinError> {
        if application.store() != association.store() {
            return Err(ResidentDeploymentJoinError::StoreSubstitution);
        }
        Ok(ReadyResidentDeployment {
            association,
            application,
        })
    }
}

/// Private process-lifetime ownership for ready deployments and operation work.
///
/// The sorted deployment set is fixed at construction. Selection uses the
/// complete admitted principal, binding, and runtime-authority coordinate, then
/// joins the selected application to access for its exact physical store. It
/// never falls back to the only deployment, a store identity, or a newer
/// incarnation. The shared operation host retains one task owner across every
/// selected deployment.
///
/// Field order is intentional: operation state and deployment applications are
/// dropped before the opaque writer/lock capability. [`program::ResidentProgram`]
/// owns the corresponding application-private readiness path. A native
/// composition root must still keep all borrowed physical stores alive until the
/// program and its held writer guard are dropped.
pub(in crate::resident) struct ResidentProcessHost<'store, WriterGuard> {
    operations: ResidentOperationHost,
    provider_routes: ResidentProcessProviderRoutes,
    a2a_delegate_routes: ResidentProcessA2aDelegateRoutes,
    a2a_task_routes: ResidentProcessA2aTaskRoutes,
    operation_recovery_attempts: Box<[LocalOperationStartDiscoveryAttempt]>,
    consult_recovery_attempts: Box<[LocalOperationStartDiscoveryAttempt]>,
    consult_child_recovery_attempts: Box<[LocalOperationStartDiscoveryAttempt]>,
    delegate_recovery_attempts: Box<[LocalOperationStartDiscoveryAttempt]>,
    publication_attempts: Box<[super::event_custody::LocalOperationPublicationSweepAttempt]>,
    deployments: Box<[ReadyResidentDeployment<'store>]>,
    writer: ReadyPrincipalResidentWriter<WriterGuard>,
}

/// Disjoint process-host custody for one provider preparation recovered in place.
///
/// The application stays immutably borrowed while the process-wide task and
/// route owners remain independently mutable. This is the only shape that can
/// carry generation-borrowing recovered meaning into the existing task owner
/// without creating a second application or operation owner.
struct ResidentProcessProviderRecoveryDriver<'host, 'store> {
    application: &'host ResidentApplication<'store>,
    operations: &'host mut ResidentOperationHost,
    routes: &'host mut ResidentProcessProviderRoutes,
    selector: ResidentDeploymentAuthoritySelector,
    store: AuthorityStoreCoordinate,
}

impl<'host, 'store> ResidentProcessProviderRecoveryDriver<'host, 'store> {
    fn application(&self) -> &'host ResidentApplication<'store> {
        self.application
    }
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Borrow the selected application separately from the process task owners.
    fn provider_recovery_driver(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<ResidentProcessProviderRecoveryDriver<'_, 'store>, ResidentProcessHostSelectionError>
    {
        let index = self.deployment_authority_index(selector)?;
        let Self {
            operations,
            provider_routes,
            deployments,
            ..
        } = self;
        let application = deployments[index].application.as_ref();
        Ok(ResidentProcessProviderRecoveryDriver {
            application,
            operations,
            routes: provider_routes,
            selector: selector.clone(),
            store: application.store().clone(),
        })
    }
    /// Join already-proved writer, deployment, and operation ownership.
    ///
    /// This pure constructor performs no port I/O. The private program builds its
    /// readiness inputs from admitted writer and deployment evidence.
    fn new(
        writer: ReadyPrincipalResidentWriter<WriterGuard>,
        mut deployments: Vec<ReadyResidentDeployment<'store>>,
        maximum_ready_deployments: NonZeroUsize,
        operations: ResidentOperationHost,
    ) -> Result<Self, ResidentProcessHostBuildError> {
        let provider_routes = ResidentProcessProviderRoutes::new(
            operations.operation_task_owner_instance().clone(),
            operations.provider_task_limits(),
        )?;
        let a2a_delegate_routes = ResidentProcessA2aDelegateRoutes::new(
            operations.operation_task_owner_instance().clone(),
            operations.aggregate_task_limits(),
        )?;
        let a2a_task_routes = ResidentProcessA2aTaskRoutes::new(
            operations.operation_task_owner_instance().clone(),
            operations.aggregate_task_limits(),
        )?;
        if deployments.len() > maximum_ready_deployments.get() {
            return Err(ResidentProcessHostBuildError::DeploymentBoundExceeded);
        }
        deployments.sort_by(|left, right| left.store().cmp(right.store()));
        validate_deployment_stores(&deployments)?;
        validate_deployment_authorities(&deployments)?;
        if deployments
            .iter()
            .any(|deployment| deployment.association.principal_id() != writer.principal_id())
        {
            return Err(ResidentProcessHostBuildError::PrincipalAssociation);
        }
        if deployments
            .iter()
            .any(|deployment| deployment.association.store() != deployment.store())
        {
            return Err(ResidentProcessHostBuildError::RuntimeAuthorityStoreAssociation);
        }
        if writer.opened_deployment_stores().len() != deployments.len()
            || writer
                .opened_deployment_stores()
                .iter()
                .zip(deployments.iter())
                .any(|(opened, deployment)| opened != deployment.store())
        {
            return Err(ResidentProcessHostBuildError::OpenedStoreSet);
        }
        let mut publication_attempts = Vec::new();
        publication_attempts
            .try_reserve_exact(deployments.len())
            .map_err(|_| ResidentProcessHostBuildError::PublicationAttemptAllocation)?;
        publication_attempts.extend(deployments.iter().map(|deployment| {
            super::event_custody::LocalOperationPublicationSweepAttempt::new(
                deployment.store().clone(),
            )
        }));
        let mut operation_recovery_attempts = Vec::new();
        operation_recovery_attempts
            .try_reserve_exact(deployments.len())
            .map_err(|_| ResidentProcessHostBuildError::OperationRecoveryAttemptAllocation)?;
        operation_recovery_attempts.extend(deployments.iter().map(|deployment| {
            LocalOperationStartDiscoveryAttempt::new(deployment.store().clone())
        }));
        let mut consult_recovery_attempts = Vec::new();
        consult_recovery_attempts
            .try_reserve_exact(deployments.len())
            .map_err(|_| ResidentProcessHostBuildError::ConsultRecoveryAttemptAllocation)?;
        consult_recovery_attempts.extend(deployments.iter().map(|deployment| {
            LocalOperationStartDiscoveryAttempt::new(deployment.store().clone())
        }));
        let mut delegate_recovery_attempts = Vec::new();
        delegate_recovery_attempts
            .try_reserve_exact(deployments.len())
            .map_err(|_| ResidentProcessHostBuildError::DelegateRecoveryAttemptAllocation)?;
        delegate_recovery_attempts.extend(deployments.iter().map(|deployment| {
            LocalOperationStartDiscoveryAttempt::new(deployment.store().clone())
        }));
        let mut consult_child_recovery_attempts = Vec::new();
        consult_child_recovery_attempts
            .try_reserve_exact(deployments.len())
            .map_err(|_| ResidentProcessHostBuildError::ConsultChildRecoveryAttemptAllocation)?;
        consult_child_recovery_attempts.extend(deployments.iter().map(|deployment| {
            LocalOperationStartDiscoveryAttempt::new(deployment.store().clone())
        }));
        Ok(Self {
            operations,
            provider_routes,
            a2a_delegate_routes,
            a2a_task_routes,
            operation_recovery_attempts: operation_recovery_attempts.into_boxed_slice(),
            consult_recovery_attempts: consult_recovery_attempts.into_boxed_slice(),
            consult_child_recovery_attempts: consult_child_recovery_attempts.into_boxed_slice(),
            delegate_recovery_attempts: delegate_recovery_attempts.into_boxed_slice(),
            publication_attempts: publication_attempts.into_boxed_slice(),
            deployments: deployments.into_boxed_slice(),
            writer,
        })
    }

    /// Select exactly one admitted deployment authority without a store fallback.
    fn deployment_authority_index(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
    ) -> Result<usize, ResidentProcessHostSelectionError> {
        let mut matches = self
            .deployments
            .iter()
            .enumerate()
            .filter(|(_, deployment)| deployment.association.matches(selector));
        let Some((index, _)) = matches.next() else {
            return Err(ResidentProcessHostSelectionError::UnknownDeploymentAuthority);
        };
        if matches.next().is_some() {
            return Err(ResidentProcessHostSelectionError::AmbiguousDeploymentAuthority);
        }
        Ok(index)
    }

    /// Lend one selected application and the single process-wide operation owner.
    ///
    /// The access-store association is checked before the custody, session, or
    /// anchor port can enter the closure. Neither value can escape the borrow.
    /// The application is lent even when a use case needs only the operation
    /// service so later in-process composition cannot silently select state from
    /// another deployment or create a second application owner.
    fn with_selected_operation_service<Authority, Output>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        use_service: impl FnOnce(
            &ResidentApplication<'store>,
            &mut ResidentOperationService<'_, Authority>,
            &dyn SessionLeaseCustodyPort,
        ) -> Output,
    ) -> Result<Output, ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(selector)?;
        if self.deployments[index].store() != access.store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        let Self {
            operations,
            deployments,
            ..
        } = self;
        let application = deployments[index].application.as_ref();
        let ResidentDeploymentOperationAccess {
            custody,
            sessions,
            anchor,
            ..
        } = access;
        Ok(
            operations.with_operation_service(authority, custody, anchor, |service| {
                use_service(application, service, sessions)
            }),
        )
    }

    /// Join current and historical operation-control authority before lending any port.
    fn with_selected_operation_control_service<Authority, Output>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        use_service: impl FnOnce(
            &ResidentApplication<'store>,
            &mut ResidentOperationService<'_, Authority>,
            &dyn SessionLeaseCustodyPort,
        ) -> Output,
    ) -> Result<Output, ResidentProcessHostSelectionError> {
        if !selector.matches_signing_authority(association.current_request_authority) {
            return Err(ResidentProcessHostSelectionError::CurrentRequestAuthority);
        }
        if !selector.matches_signing_authority(association.historical_handle_authority) {
            return Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority);
        }
        let index = self.deployment_authority_index(selector)?;
        let selected_store = self.deployments[index].store();
        if association.recovery_store != selected_store {
            return Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation);
        }
        if association.checkpoint_authority.principal_id != &selector.principal_id
            || association.checkpoint_authority.control_store_id != selected_store.store_id()
            || association.checkpoint_authority.control_incarnation_id
                != selected_store.incarnation_id()
        {
            return Err(ResidentProcessHostSelectionError::CheckpointAuthorityAssociation);
        }
        self.with_selected_operation_service(selector, access, authority, use_service)
    }

    /// Protect one exact active Delegate parent before any child identity allocation.
    #[allow(clippy::too_many_arguments)]
    fn protect_delegate_parent<
        Signer,
        State,
        Rotation,
        Signature,
        Checkpoint,
        CheckpointSignature,
    >(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        recovery: OperationObservationRecoveryInput<'_>,
        parent_lease: &OperationLeaseV1,
        admissions: OperationObservationRecoveryAdmission<
            Signer,
            State,
            Rotation,
            Signature,
            Checkpoint,
        >,
        verify_checkpoint_signature: CheckpointSignature,
    ) -> Result<ResidentProcessProtectedDelegateParent, ResidentProcessDelegateParentProtectionError>
    where
        Checkpoint: FnMut(
            &ResidentRecordRefV1,
            ControlStateCheckpointWireV1,
        ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
        Signer: FnMut(
            &ResidentRecordRefV1,
            &OperationObservationWireV1,
        ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
        State: FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Signature: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        CheckpointSignature: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let index = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessDelegateParentProtectionError::Selection)?;
        if self.deployments[index].store() != access.store
            || recovery.store != access.store
            || !selector.matches_operation(recovery.operation)
            || recovery.checkpoint_authority.principal_id != &selector.principal_id
            || recovery.checkpoint_authority.control_store_id != access.store.store_id()
            || recovery.checkpoint_authority.control_incarnation_id != access.store.incarnation_id()
        {
            return Err(ResidentProcessDelegateParentProtectionError::Selection(
                ResidentProcessHostSelectionError::DelegateParentAuthority,
            ));
        }
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions: _,
            anchor,
        } = access;
        let parent = super::prepare_local_child_parent(
            recovery,
            parent_lease,
            custody,
            anchor,
            admissions,
            verify_checkpoint_signature,
        )
        .map_err(ResidentProcessDelegateParentProtectionError::Protection)?;
        if parent.store() != store || !selector.matches_handle(parent.operation().1) {
            return Err(ResidentProcessDelegateParentProtectionError::Selection(
                ResidentProcessHostSelectionError::DelegateParentAuthority,
            ));
        }
        Ok(ResidentProcessProtectedDelegateParent {
            selector: selector.clone(),
            store: store.clone(),
            parent,
            lease: parent_lease.clone(),
        })
    }

    /// Allocate a Delegate child only after its exact parent is protected.
    fn allocate_delegate_operation_start(
        &self,
        parent: ResidentProcessProtectedDelegateParent,
        access: ResidentDeploymentOperationLookup<'_>,
        context: ResidentProcessOperationStartContext<'_>,
        random: &dyn CryptographicRandomSourcePort,
    ) -> Result<
        ResidentProcessDelegateOperationStartAllocation,
        ResidentProcessDelegateOperationAllocationFailure,
    > {
        if parent.store != *access.store {
            return Err(ResidentProcessDelegateOperationAllocationFailure {
                parent: Box::new(parent),
                reason: ResidentProcessOperationAllocationError::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation,
                ),
            });
        }
        let selector = parent.selector.clone();
        match self.allocate_operation_start(&selector, access, context, random) {
            Ok(ResidentProcessOperationStartAllocation::Existing(existing)) => {
                Ok(ResidentProcessDelegateOperationStartAllocation::Existing {
                    existing,
                    parent: Box::new(parent),
                })
            }
            Ok(ResidentProcessOperationStartAllocation::Fresh(fresh)) => {
                Ok(ResidentProcessDelegateOperationStartAllocation::Fresh(
                    Box::new(ResidentProcessFreshDelegateOperationStart {
                        fresh,
                        parent: Box::new(parent),
                    }),
                ))
            }
            Err(reason) => Err(ResidentProcessDelegateOperationAllocationFailure {
                parent: Box::new(parent),
                reason,
            }),
        }
    }

    /// Allocate one request-first operation identity phase under one deployment.
    ///
    /// All derivable deployment, session, request, signer, checkpoint, and store
    /// associations are joined before the selected custody port is lent. The
    /// operation-start allocation owner then performs the sole request lookup
    /// and, if absent, exactly one bounded allocation. No authority, signing, write,
    /// anchor, lease, task, native-action, or provider path is available here.
    fn allocate_operation_start(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationLookup<'_>,
        context: ResidentProcessOperationStartContext<'_>,
        random: &dyn CryptographicRandomSourcePort,
    ) -> Result<ResidentProcessOperationStartAllocation, ResidentProcessOperationAllocationError>
    {
        self.validate_operation_start_context_associations(selector, access.store, &context)?;
        let selected = selector.clone();
        let owner_instance = self.operations.operation_task_owner_instance().clone();
        let request_sha256 = context.request.canonical_request_sha256().clone();
        let maximum_record_bytes = context.maximum_record_bytes;
        let selected_store = access.store.clone();
        let allocated = super::operation_start_allocation::begin_operation_start_allocation(
            access.custody,
            random,
            &selected_store,
            context.request,
            maximum_record_bytes,
        )
        .map_err(ResidentProcessOperationAllocationError::Allocation)?;
        Ok(match allocated {
            OperationStartAllocation::Existing(evidence) => {
                ResidentProcessOperationStartAllocation::Existing(Box::new(
                    ResidentProcessExistingOperationStart {
                        selector: selected,
                        evidence,
                    },
                ))
            }
            OperationStartAllocation::Fresh(allocation) => {
                let request_ref = allocation.request_ref().clone();
                ResidentProcessOperationStartAllocation::Fresh(Box::new(
                    ResidentProcessFreshOperationStart {
                        owner_instance,
                        selector: selected,
                        request_ref,
                        request_sha256,
                        maximum_record_bytes,
                        allocation,
                    },
                ))
            }
        })
    }

    /// Read one existing root start before asking the fresh-start policy owner.
    ///
    /// Exact request replay must remain observable after its original start
    /// continuation moves to another resident lane. This read uses only the
    /// already-admitted recovery ceiling. Absence grants no allocation and the
    /// normal request-first allocation path repeats the lookup after policy
    /// selection, so a concurrent winner remains authoritative.
    fn read_existing_root_operation_start(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationLookup<'_>,
        context: ResidentProcessOperationStartContext<'_>,
    ) -> Result<
        Option<ResidentProcessExistingOperationStart>,
        ResidentProcessOperationAllocationError,
    > {
        self.validate_operation_start_context_associations(selector, access.store, &context)?;
        read_operation_start_for_request(
            access.custody,
            access.store,
            context.request,
            context.maximum_record_bytes,
        )
        .map(|evidence| {
            evidence.map(|evidence| ResidentProcessExistingOperationStart {
                selector: selector.clone(),
                evidence: Box::new(evidence),
            })
        })
        .map_err(OperationStartAllocationError::Recovery)
        .map_err(ResidentProcessOperationAllocationError::Allocation)
    }

    /// Resolve one Invoke request without rebuilding existing work.
    #[allow(clippy::too_many_arguments)]
    fn resolve_invoke_request(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        expected_class: OperationClassV1,
        access: ResidentDeploymentOperationAccess<'_>,
        context: ResidentProcessOperationStartContext<'_>,
        random: &dyn CryptographicRandomSourcePort,
        restart: ResidentProcessOperationRestartInput<'_>,
        admission: OperationControlRecoveryAdmission<'_>,
    ) -> Result<ResidentProcessInvokeRequestResolution, ResidentProcessInvokeRequestResolutionError>
    {
        let request = context.request;
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let allocation = self
            .allocate_operation_start(
                selector,
                ResidentDeploymentOperationLookup {
                    store,
                    custody: &mut *custody,
                },
                context,
                random,
            )
            .map_err(ResidentProcessInvokeRequestResolutionError::Allocation)?;
        match allocation {
            ResidentProcessOperationStartAllocation::Fresh(fresh) => {
                Ok(ResidentProcessInvokeRequestResolution::FreshStart(fresh))
            }
            ResidentProcessOperationStartAllocation::Existing(existing) => {
                let request = request.to_preimage();
                let association = ResidentOperationControlAssociation {
                    current_request_authority: &request.signing_authority_binding,
                    historical_handle_authority: restart.operation.handle_signing_authority,
                    recovery_store: restart.operation.store,
                    checkpoint_authority: restart.operation.checkpoint_authority,
                };
                self.classify_existing_operation(
                    *existing,
                    association,
                    ResidentDeploymentOperationAccess {
                        store,
                        custody,
                        sessions,
                        anchor,
                    },
                    restart,
                    admission,
                )
                .and_then(|disposition| admit_existing_root_start_kind(disposition, expected_class))
                .map(Box::new)
                .map(ResidentProcessInvokeRequestResolution::ExistingOperation)
                .map_err(ResidentProcessInvokeRequestResolutionError::ExistingOperation)
            }
        }
    }

    /// Resolve one root Invoke request. Start it only if no durable start exists.
    #[allow(clippy::too_many_arguments)]
    fn invoke_request<
        Authority,
        PolicyEvaluator,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessRootInput<'_>,
        random: &dyn CryptographicRandomSourcePort,
        restart: ResidentProcessOperationRestartInput<'_>,
        admission: OperationControlRecoveryAdmission<'_>,
        authority: &mut Authority,
        policy_evaluator: &mut PolicyEvaluator,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProcessRootOutcome, ResidentProcessRootError>
    where
        Authority: super::AmbientSessionAuthorityPort,
        PolicyEvaluator: ResidentProcessInvokePolicyEvaluator,
        CompiledAuthority: FnOnce(
            &pale_core::records::CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &pale_core::records::OperationHandlePreimageV1,
            &pale_core::records::OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &pale_core::records::OperationHandlePreimageV1,
            &pale_core::records::OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
                &pale_core::records::ControlCheckpointIdV1,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessRootInput {
            request,
            session,
            current,
            handle_signing_authority,
            checkpoint_authority,
            fresh_start_admission,
        } = input;
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let existing = self
            .read_existing_root_operation_start(
                selector,
                ResidentDeploymentOperationLookup {
                    store,
                    custody: &mut *custody,
                },
                ResidentProcessOperationStartContext {
                    session,
                    current,
                    request,
                    handle_signing_authority,
                    semantic_checkpoint_authority: checkpoint_authority,
                    issuance_checkpoint_authority: checkpoint_authority,
                    maximum_record_bytes: restart.operation.maximum_record_bytes,
                },
            )
            .map_err(ResidentProcessInvokeRequestResolutionError::Allocation)
            .map_err(ResidentProcessRootError::Resolution)?;
        if let Some(existing) = existing {
            let request_preimage = request.to_preimage();
            let association = ResidentOperationControlAssociation {
                current_request_authority: &request_preimage.signing_authority_binding,
                historical_handle_authority: restart.operation.handle_signing_authority,
                recovery_store: restart.operation.store,
                checkpoint_authority: restart.operation.checkpoint_authority,
            };
            let disposition = self
                .classify_existing_operation(
                    existing,
                    association,
                    ResidentDeploymentOperationAccess {
                        store,
                        custody: &mut *custody,
                        sessions,
                        anchor: &mut *anchor,
                    },
                    restart,
                    admission,
                )
                .and_then(|disposition| {
                    admit_existing_root_start_kind(disposition, OperationClassV1::Invoke)
                })
                .map_err(ResidentProcessInvokeRequestResolutionError::ExistingOperation)
                .map_err(ResidentProcessRootError::Resolution)?;
            return Ok(ResidentProcessRootOutcome::ExistingOperation(Box::new(
                disposition,
            )));
        }
        if fresh_start_admission == ResidentProcessFreshStartAdmission::CapacityExhausted {
            return Err(ResidentProcessRootError::Capacity);
        }
        let generation_digest = session.lease().compiled_generation_digest();
        let index = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessRootError::Policy)?;
        let operation = self.deployments[index]
            .application
            .generation(generation_digest)
            .and_then(|retained| {
                retained.operation(session.lease().repository_id(), request.operation())
            })
            .ok_or(ResidentProcessRootError::Policy(
                ResidentProcessHostSelectionError::StartRequestAssociation,
            ))?;
        let policy = policy_evaluator
            .evaluate_invoke(ResidentProcessInvokePolicyContext {
                request,
                session,
                operation,
            })
            .map_err(ResidentProcessRootError::Policy)?;
        native_hook_start::admit_invocation_policy(request, &policy.lease)
            .map_err(ResidentProcessRootError::Policy)?;
        let ResidentProcessInvokePolicy {
            checkpoint_profile,
            publication_policy,
            maximum_record_bytes,
            lease,
            handle_expires_at,
            handle_retain_until,
            accumulator_limits,
        } = policy;
        let resolved = self
            .resolve_invoke_request(
                selector,
                OperationClassV1::Invoke,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                ResidentProcessOperationStartContext {
                    session,
                    current,
                    request,
                    handle_signing_authority,
                    semantic_checkpoint_authority: checkpoint_authority,
                    issuance_checkpoint_authority: checkpoint_authority,
                    maximum_record_bytes,
                },
                random,
                restart,
                admission,
            )
            .map_err(ResidentProcessRootError::Resolution)?;
        let fresh = match resolved {
            ResidentProcessInvokeRequestResolution::FreshStart(fresh) => fresh,
            ResidentProcessInvokeRequestResolution::ExistingOperation(disposition) => {
                return Ok(ResidentProcessRootOutcome::ExistingOperation(disposition));
            }
        };
        let checkpoint_id = fresh.allocation.checkpoint_id().clone();
        let retained = self
            .admit_and_retain_fresh_operation_lease(
                *fresh,
                ResidentDeploymentOperationLeaseAccess {
                    store,
                    custody: &mut *custody,
                },
                ResidentProcessOperationStartContext {
                    session,
                    current,
                    request,
                    handle_signing_authority,
                    semantic_checkpoint_authority: checkpoint_authority,
                    issuance_checkpoint_authority: checkpoint_authority,
                    maximum_record_bytes,
                },
                authority,
                lease,
            )
            .map_err(ResidentProcessRootError::Lease)?;
        self.start_allocated_invoke(
            retained,
            ResidentDeploymentOperationAccess {
                store,
                custody,
                sessions,
                anchor,
            },
            authority,
            ResidentProcessInvokeStartBasis {
                checkpoint_profile: &checkpoint_profile,
                request,
                session,
                current,
                publication_policy: publication_policy.as_input(),
                handle_signing_authority,
                checkpoint_authority,
                handle_expires_at,
                handle_retain_until,
                accumulator_limits,
            },
            admit_compiled_authority,
            sign_handle,
            sign_observation,
            move |prepared| prepare_checkpoint_preimage(prepared, &checkpoint_id),
            sign_checkpoint,
        )
        .map(Box::new)
        .map(ResidentProcessRootOutcome::Started)
        .map_err(ResidentProcessRootError::Start)
    }

    /// Resolve one root Consult request. Start it only if no durable start exists.
    #[allow(clippy::too_many_arguments)]
    fn consult_request<
        Authority,
        PolicyEvaluator,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessRootInput<'_>,
        random: &dyn CryptographicRandomSourcePort,
        restart: ResidentProcessOperationRestartInput<'_>,
        admission: OperationControlRecoveryAdmission<'_>,
        authority: &mut Authority,
        policy_evaluator: &mut PolicyEvaluator,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProcessConsultRootOutcome, ResidentProcessConsultRootError>
    where
        Authority: super::AmbientSessionAuthorityPort,
        PolicyEvaluator: ResidentProcessConsultPolicyEvaluator,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &pale_core::records::OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
                &pale_core::records::ControlCheckpointIdV1,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessRootInput {
            request,
            session,
            current,
            handle_signing_authority,
            checkpoint_authority,
            fresh_start_admission,
        } = input;
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let existing = self
            .read_existing_root_operation_start(
                selector,
                ResidentDeploymentOperationLookup {
                    store,
                    custody: &mut *custody,
                },
                ResidentProcessOperationStartContext {
                    session,
                    current,
                    request,
                    handle_signing_authority,
                    semantic_checkpoint_authority: checkpoint_authority,
                    issuance_checkpoint_authority: checkpoint_authority,
                    maximum_record_bytes: restart.operation.maximum_record_bytes,
                },
            )
            .map_err(ResidentProcessInvokeRequestResolutionError::Allocation)
            .map_err(ResidentProcessConsultRootError::Resolution)?;
        if let Some(existing) = existing {
            let request_preimage = request.to_preimage();
            let association = ResidentOperationControlAssociation {
                current_request_authority: &request_preimage.signing_authority_binding,
                historical_handle_authority: restart.operation.handle_signing_authority,
                recovery_store: restart.operation.store,
                checkpoint_authority: restart.operation.checkpoint_authority,
            };
            let disposition = self
                .classify_existing_operation(
                    existing,
                    association,
                    ResidentDeploymentOperationAccess {
                        store,
                        custody: &mut *custody,
                        sessions,
                        anchor: &mut *anchor,
                    },
                    restart,
                    admission,
                )
                .and_then(|disposition| {
                    admit_existing_root_start_kind(disposition, OperationClassV1::Consult)
                })
                .map_err(ResidentProcessInvokeRequestResolutionError::ExistingOperation)
                .map_err(ResidentProcessConsultRootError::Resolution)?;
            return Ok(ResidentProcessConsultRootOutcome::ExistingOperation(
                Box::new(disposition),
            ));
        }
        if fresh_start_admission == ResidentProcessFreshStartAdmission::CapacityExhausted {
            return Err(ResidentProcessConsultRootError::Capacity);
        }
        let generation_digest = session.lease().compiled_generation_digest();
        let index = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessConsultRootError::Policy)?;
        let operation = self.deployments[index]
            .application
            .generation(generation_digest)
            .and_then(|retained| {
                retained.operation(session.lease().repository_id(), request.operation())
            })
            .ok_or(ResidentProcessConsultRootError::Policy(
                ResidentProcessHostSelectionError::StartRequestAssociation,
            ))?;
        let policy = policy_evaluator
            .evaluate_consult(ResidentProcessConsultPolicyContext {
                request,
                session,
                operation,
            })
            .map_err(ResidentProcessConsultRootError::Policy)?;
        let ResidentProcessConsultPolicy { start, lease } = policy;
        let maximum_record_bytes = start.maximum_record_bytes();
        let resolved = self
            .resolve_invoke_request(
                selector,
                OperationClassV1::Consult,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                ResidentProcessOperationStartContext {
                    session,
                    current,
                    request,
                    handle_signing_authority,
                    semantic_checkpoint_authority: checkpoint_authority,
                    issuance_checkpoint_authority: checkpoint_authority,
                    maximum_record_bytes,
                },
                random,
                restart,
                admission,
            )
            .map_err(ResidentProcessConsultRootError::Resolution)?;
        let fresh = match resolved {
            ResidentProcessInvokeRequestResolution::FreshStart(fresh) => fresh,
            ResidentProcessInvokeRequestResolution::ExistingOperation(disposition) => {
                return Ok(ResidentProcessConsultRootOutcome::ExistingOperation(
                    disposition,
                ));
            }
        };
        let checkpoint_id = fresh.allocation.checkpoint_id().clone();
        let retained = self
            .admit_and_retain_fresh_operation_lease(
                *fresh,
                ResidentDeploymentOperationLeaseAccess {
                    store,
                    custody: &mut *custody,
                },
                ResidentProcessOperationStartContext {
                    session,
                    current,
                    request,
                    handle_signing_authority,
                    semantic_checkpoint_authority: checkpoint_authority,
                    issuance_checkpoint_authority: checkpoint_authority,
                    maximum_record_bytes,
                },
                authority,
                lease,
            )
            .map_err(ResidentProcessConsultRootError::Lease)?;
        self.start_allocated_consult(
            retained,
            ResidentDeploymentOperationAccess {
                store,
                custody,
                sessions,
                anchor,
            },
            authority,
            request,
            session,
            current,
            handle_signing_authority,
            checkpoint_authority,
            start,
            admit_compiled_authority,
            sign_handle,
            sign_observation,
            move |prepared| prepare_checkpoint_preimage(prepared, &checkpoint_id),
            sign_checkpoint,
        )
        .map(Box::new)
        .map(ResidentProcessConsultRootOutcome::Started)
        .map_err(ResidentProcessConsultRootError::Start)
    }

    /// Resolve one pairwise Delegate request after its exact parent is protected.
    #[allow(clippy::too_many_arguments)]
    fn delegate_request<
        Authority,
        PolicyEvaluator,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessRootInput<'_>,
        parent: PreparedLocalChildParent,
        random: &dyn CryptographicRandomSourcePort,
        restart: ResidentProcessOperationRestartInput<'_>,
        admission: OperationControlRecoveryAdmission<'_>,
        authority: &mut Authority,
        policy_evaluator: &mut PolicyEvaluator,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProcessRootOutcome, ResidentProcessDelegateRootError>
    where
        Authority: super::AmbientSessionAuthorityPort,
        PolicyEvaluator: ResidentProcessDelegatePolicyEvaluator,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &pale_core::records::OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
                &pale_core::records::ControlCheckpointIdV1,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessRootInput {
            request,
            session,
            current,
            handle_signing_authority,
            checkpoint_authority,
            fresh_start_admission,
        } = input;
        if fresh_start_admission == ResidentProcessFreshStartAdmission::CapacityExhausted {
            return Err(ResidentProcessDelegateRootError::Capacity);
        }
        let index = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessDelegateRootError::Policy)?;
        let operation = self.deployments[index]
            .application
            .generation(session.lease().compiled_generation_digest())
            .and_then(|retained| {
                retained.operation(session.lease().repository_id(), request.operation())
            })
            .ok_or(ResidentProcessDelegateRootError::Policy(
                ResidentProcessHostSelectionError::StartRequestAssociation,
            ))?;
        let ResidentProcessDelegatePolicy { start, lease } = policy_evaluator
            .evaluate_delegate(ResidentProcessDelegatePolicyContext {
                request,
                session,
                operation,
                parent: &parent,
            })
            .map_err(ResidentProcessDelegateRootError::Policy)?;
        let maximum_record_bytes = start.maximum_record_bytes();
        let protected_parent = ResidentProcessProtectedDelegateParent {
            selector: selector.clone(),
            store: parent.store().clone(),
            lease: parent.lease().clone(),
            parent,
        };
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let start_context = ResidentProcessOperationStartContext {
            session,
            current,
            request,
            handle_signing_authority,
            semantic_checkpoint_authority: checkpoint_authority,
            issuance_checkpoint_authority: checkpoint_authority,
            maximum_record_bytes,
        };
        let allocated = self
            .allocate_delegate_operation_start(
                protected_parent,
                ResidentDeploymentOperationLookup {
                    store,
                    custody: &mut *custody,
                },
                start_context,
                random,
            )
            .map_err(ResidentProcessDelegateRootError::Allocation)?;
        let fresh = match allocated {
            ResidentProcessDelegateOperationStartAllocation::Existing { existing, parent } => {
                let request_preimage = request.to_preimage();
                let association = ResidentOperationControlAssociation {
                    current_request_authority: &request_preimage.signing_authority_binding,
                    historical_handle_authority: restart.operation.handle_signing_authority,
                    recovery_store: restart.operation.store,
                    checkpoint_authority: restart.operation.checkpoint_authority,
                };
                let mut admission = admission;
                let protected = self
                    .protect_existing_operation_start(
                        *existing,
                        association,
                        ResidentDeploymentOperationAccess {
                            store,
                            custody: &mut *custody,
                            sessions,
                            anchor: &mut *anchor,
                        },
                        &restart,
                        &mut admission,
                    )
                    .map_err(ResidentProcessDelegateRootError::Resolution)?;
                let disposition = self
                    .classify_protected_delegate_child_operation(
                        protected,
                        ResidentDeploymentOperationAccess {
                            store,
                            custody,
                            sessions,
                            anchor,
                        },
                        restart,
                        &mut admission,
                    )
                    .and_then(|disposition| admit_existing_delegate_start(disposition, &parent))
                    .map_err(ResidentProcessDelegateRootError::Resolution)?;
                return Ok(ResidentProcessRootOutcome::ExistingOperation(Box::new(
                    disposition,
                )));
            }
            ResidentProcessDelegateOperationStartAllocation::Fresh(fresh) => *fresh,
        };
        let checkpoint_id = fresh.fresh.allocation.checkpoint_id().clone();
        let retained = self
            .admit_and_retain_fresh_delegate_operation_lease(
                fresh,
                ResidentDeploymentOperationLeaseAccess {
                    store,
                    custody: &mut *custody,
                },
                start_context,
                authority,
                lease,
            )
            .map_err(ResidentProcessDelegateRootError::Lease)?;
        self.start_allocated_delegate(
            retained,
            ResidentDeploymentOperationAccess {
                store,
                custody,
                sessions,
                anchor,
            },
            authority,
            request,
            session,
            current,
            handle_signing_authority,
            checkpoint_authority,
            start,
            admit_compiled_authority,
            sign_handle,
            sign_observation,
            move |prepared| prepare_checkpoint_preimage(prepared, &checkpoint_id),
            sign_checkpoint,
        )
        .map(Box::new)
        .map(ResidentProcessRootOutcome::Started)
        .map_err(ResidentProcessDelegateRootError::Start)
    }

    /// Recheck only the process and deployment fence around one fresh allocation.
    ///
    /// The allocation is returned unchanged on both success and failure. This
    /// does not admit it, extract its identities, or permit a start.
    fn reselect_fresh_operation_start(
        &self,
        pending: ResidentProcessFreshOperationStart,
        access_store: &AuthorityStoreCoordinate,
    ) -> Result<ResidentProcessFreshOperationStart, ResidentProcessOperationAllocationFenceFailure>
    {
        let reason = if !pending
            .owner_instance
            .is_same(self.operations.operation_task_owner_instance())
        {
            Some(ResidentProcessHostSelectionError::StartProcessOwner)
        } else {
            match self.deployment_authority_index(&pending.selector) {
                Ok(index) if self.deployments[index].store() == access_store => None,
                Ok(_) => Some(ResidentProcessHostSelectionError::AccessStoreAssociation),
                Err(reason) => Some(reason),
            }
        };
        match reason {
            Some(reason) => Err(ResidentProcessOperationAllocationFenceFailure {
                pending: Box::new(pending),
                reason,
            }),
            None => Ok(pending),
        }
    }

    /// Admit and retain the lease allocated for one exact fresh operation start.
    ///
    /// Every selector and request fence is repeated before ambient authority or
    /// custody can run. Admission receives the allocated lease identity, while
    /// retention receives the allocated store and record reference. A failure
    /// returns the last exact linear phase and never retries an uncertain write.
    fn admit_and_retain_fresh_operation_lease<Authority>(
        &self,
        pending: ResidentProcessFreshOperationStart,
        access: ResidentDeploymentOperationLeaseAccess<'_>,
        context: ResidentProcessOperationStartContext<'_>,
        authority: &mut Authority,
        basis: ResidentProcessOperationLeaseBasis,
    ) -> Result<ResidentProcessRetainedOperationLease, ResidentProcessOperationLeaseFailure>
    where
        Authority: super::AmbientSessionAuthorityPort,
    {
        self.admit_and_retain_fresh_operation_lease_with_lineage(
            pending,
            access,
            context,
            authority,
            basis,
            ResidentProcessOperationLeaseLineage::Independent,
        )
    }

    /// Admit and retain one fresh Delegate lease under its protected parent.
    fn admit_and_retain_fresh_delegate_operation_lease<Authority>(
        &self,
        pending: ResidentProcessFreshDelegateOperationStart,
        access: ResidentDeploymentOperationLeaseAccess<'_>,
        context: ResidentProcessOperationStartContext<'_>,
        authority: &mut Authority,
        basis: ResidentProcessOperationLeaseBasis,
    ) -> Result<
        ResidentProcessRetainedDelegateOperationLease,
        ResidentProcessDelegateOperationLeaseFailure,
    >
    where
        Authority: super::AmbientSessionAuthorityPort,
    {
        let ResidentProcessFreshDelegateOperationStart { fresh, parent } = pending;
        if parent.store != *access.store || parent.selector != fresh.selector {
            return Err(ResidentProcessDelegateOperationLeaseFailure {
                parent,
                reason: ResidentProcessOperationLeaseFailure::Selection {
                    pending: fresh,
                    reason: ResidentProcessHostSelectionError::DelegateParentAuthority,
                },
            });
        }
        let retained = match self.admit_and_retain_fresh_operation_lease_with_lineage(
            *fresh,
            access,
            context,
            authority,
            basis,
            ResidentProcessOperationLeaseLineage::DelegateChild(&parent.lease),
        ) {
            Ok(retained) => retained,
            Err(reason) => {
                return Err(ResidentProcessDelegateOperationLeaseFailure { parent, reason });
            }
        };
        Ok(ResidentProcessRetainedDelegateOperationLease { retained, parent })
    }

    fn admit_and_retain_fresh_operation_lease_with_lineage<Authority>(
        &self,
        pending: ResidentProcessFreshOperationStart,
        access: ResidentDeploymentOperationLeaseAccess<'_>,
        context: ResidentProcessOperationStartContext<'_>,
        authority: &mut Authority,
        basis: ResidentProcessOperationLeaseBasis,
        lineage: ResidentProcessOperationLeaseLineage<'_>,
    ) -> Result<ResidentProcessRetainedOperationLease, ResidentProcessOperationLeaseFailure>
    where
        Authority: super::AmbientSessionAuthorityPort,
    {
        let pending = match self.reselect_fresh_operation_start(pending, access.store) {
            Ok(pending) => pending,
            Err(failure) => {
                return Err(ResidentProcessOperationLeaseFailure::Selection {
                    pending: failure.pending,
                    reason: failure.reason,
                });
            }
        };
        if let Err(reason) = self.validate_operation_start_context_associations(
            &pending.selector,
            access.store,
            &context,
        ) {
            return Err(ResidentProcessOperationLeaseFailure::Selection {
                pending: Box::new(pending),
                reason,
            });
        }
        if pending.maximum_record_bytes != context.maximum_record_bytes
            || pending.request_ref != *pending.allocation.request_ref()
            || pending.request_sha256 != *context.request.canonical_request_sha256()
            || pending.allocation.store() != access.store
        {
            return Err(ResidentProcessOperationLeaseFailure::Selection {
                pending: Box::new(pending),
                reason: ResidentProcessHostSelectionError::StartAllocationAssociation,
            });
        }
        let schema = match pale_contracts::record_schema_identity(OPERATION_LEASE_V1_SCHEMA_ID) {
            Ok(schema) => schema,
            Err(reason) => {
                return Err(ResidentProcessOperationLeaseFailure::Contract {
                    pending: Box::new(pending),
                    reason,
                });
            }
        };
        let request = basis.into_request(
            pending.allocation.lease_id().clone(),
            context.request.operation().clone(),
        );
        let lease = match context.session.admit_operation(
            schema.schema_sha256,
            request,
            context.current,
            authority,
        ) {
            Ok(lease) => lease,
            Err(reason) => {
                return Err(ResidentProcessOperationLeaseFailure::Admission {
                    pending: Box::new(pending),
                    reason,
                });
            }
        };
        let admitted = ResidentProcessAdmittedOperationLease {
            fresh: Box::new(pending),
            lease,
        };
        if let ResidentProcessOperationLeaseLineage::DelegateChild(parent_lease) = lineage {
            if let Err(reason) = admitted.lease.verify_against_parent_operation(parent_lease) {
                return Err(ResidentProcessOperationLeaseFailure::Narrowing {
                    pending: Box::new(admitted),
                    reason,
                });
            }
        }
        if let Err(reason) = retain_operation_lease_body(
            access.custody,
            access.store,
            admitted.fresh.allocation.lease_ref(),
            &admitted.lease,
            admitted.fresh.maximum_record_bytes,
        ) {
            return Err(ResidentProcessOperationLeaseFailure::Custody {
                pending: Box::new(admitted),
                reason,
            });
        }
        Ok(ResidentProcessRetainedOperationLease {
            fresh: admitted.fresh,
            lease: admitted.lease,
        })
    }

    /// Start one allocated root Invoke from its exact retained lease.
    ///
    /// The caller supplies semantic meaning but no record identity or schema
    /// digest. The process host derives those values from the fresh allocation,
    /// embedded contracts, admitted session, and selected authorities before it
    /// enters the existing semantic and physical start owners.
    #[allow(clippy::too_many_arguments)]
    fn start_allocated_invoke<
        Authority,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        pending: ResidentProcessRetainedOperationLease,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        basis: ResidentProcessInvokeStartBasis<'_>,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessAllocatedInvokeStartError>
    where
        Authority: super::AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &pale_core::records::CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &pale_core::records::OperationHandlePreimageV1,
            &pale_core::records::OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &pale_core::records::OperationHandlePreimageV1,
            &pale_core::records::OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessRetainedOperationLease { fresh, lease } = pending;
        let fresh = match self.reselect_fresh_operation_start(*fresh, access.store) {
            Ok(fresh) => Box::new(fresh),
            Err(failure) => {
                return Err(ResidentProcessAllocatedInvokeStartError::Selection {
                    pending: Box::new(ResidentProcessRetainedOperationLease {
                        fresh: failure.pending,
                        lease,
                    }),
                    reason: failure.reason,
                });
            }
        };
        let pending = ResidentProcessRetainedOperationLease { fresh, lease };
        if pending.fresh.request_ref != *pending.fresh.allocation.request_ref()
            || pending.fresh.request_sha256 != *basis.request.canonical_request_sha256()
            || pending.fresh.allocation.store() != access.store
            || pending.lease.operation_lease_id() != pending.fresh.allocation.lease_id()
        {
            return Err(ResidentProcessAllocatedInvokeStartError::Selection {
                pending: Box::new(pending),
                reason: ResidentProcessHostSelectionError::StartAllocationAssociation,
            });
        }

        let handle_schema =
            match pale_contracts::record_schema_identity(OPERATION_HANDLE_V1_SCHEMA_ID) {
                Ok(schema) => schema,
                Err(reason) => {
                    return Err(ResidentProcessAllocatedInvokeStartError::Contract {
                        pending: Box::new(pending),
                        reason,
                    });
                }
            };
        let observation_schema =
            match pale_contracts::record_schema_identity(OPERATION_OBSERVATION_V1_SCHEMA_ID) {
                Ok(schema) => schema,
                Err(reason) => {
                    return Err(ResidentProcessAllocatedInvokeStartError::Contract {
                        pending: Box::new(pending),
                        reason,
                    });
                }
            };
        let checkpoint_schema =
            match pale_contracts::record_schema_identity(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID) {
                Ok(schema) => schema,
                Err(reason) => {
                    return Err(ResidentProcessAllocatedInvokeStartError::Contract {
                        pending: Box::new(pending),
                        reason,
                    });
                }
            };
        let request_binding = match basis.request.local_request_binding() {
            Ok(binding) => binding,
            Err(reason) => {
                return Err(ResidentProcessAllocatedInvokeStartError::Preparation {
                    pending: Box::new(pending),
                    reason: super::LocalOperationStartError::Admission(reason),
                });
            }
        };
        let issuer_key_id =
            match ResidentKeyIdV1::parse(basis.handle_signing_authority.signer_key_id()) {
                Ok(key) => key,
                Err(reason) => {
                    return Err(ResidentProcessAllocatedInvokeStartError::Preparation {
                        pending: Box::new(pending),
                        reason: super::LocalOperationStartError::Admission(reason),
                    });
                }
            };

        let selected = pending.fresh.selector.clone();
        let maximum_record_bytes = pending.fresh.maximum_record_bytes;
        let result = {
            let handle = OperationHandlePreimageV1 {
                schema_id: handle_schema.schema.clone(),
                schema_sha256: handle_schema.schema_sha256.clone(),
                operation_handle_id: pending.fresh.allocation.handle_id().clone(),
                operation_class: OperationClassV1::Invoke,
                parent_operation: None,
                request_binding: request_binding.clone(),
                operation_lease_ref: pending.fresh.allocation.lease_ref().clone(),
                operation_lease_sha256: pending.lease.operation_lease_digest().clone(),
                compiled_generation_sha256: basis
                    .session
                    .lease()
                    .compiled_generation_digest()
                    .clone(),
                activation_fence_sha256: basis.session.lease().activation_fence_digest().clone(),
                instance_sha256: basis.session.lease().instance_digest().clone(),
                created_at: basis.current.now,
                expires_at: basis.handle_expires_at,
                retain_until: basis.handle_retain_until,
                deployment_runtime_authority_ref: basis.session.runtime_authority_ref().clone(),
                deployment_runtime_authority_sha256: basis
                    .session
                    .runtime_authority_sha256()
                    .clone(),
                issuer_principal_id: selected.principal_id.clone(),
                issuer_key_id,
                issuer_key_epoch: *basis.handle_signing_authority.signer_key_epoch(),
            };
            let context = OperationHandleAdmissionContextV1 {
                signed: pale_core::records::SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &handle_schema.schema_sha256,
                    signing_authority: basis.handle_signing_authority,
                },
                request_binding: &request_binding,
                operation_lease_ref: pending.fresh.allocation.lease_ref(),
                operation_lease: &pending.lease,
                parent_operation: None,
                compiled_generation_sha256: basis.session.lease().compiled_generation_digest(),
                activation_fence_sha256: basis.session.lease().activation_fence_digest(),
                instance_sha256: basis.session.lease().instance_digest(),
            };
            match self.with_selected_operation_service(
                &selected,
                access,
                authority,
                |application, service, _sessions| {
                    let Some(generation) = application
                        .generation(basis.session.lease().compiled_generation_digest())
                    else {
                        return Err(EitherProcessInvokeStartFailure::Selection(
                            ResidentProcessHostSelectionError::StartRequestAssociation,
                        ));
                    };
                    let input = ResidentLocalOperationStartInput {
                        semantic: super::LocalOperationStartInput {
                            checkpoint_profile: basis.checkpoint_profile,
                            request_ref: pending.fresh.allocation.request_ref().clone(),
                            request: basis.request,
                            handle_ref: pending.fresh.allocation.handle_ref().clone(),
                            handle,
                            context: &context,
                            session: basis.session,
                            current: basis.current,
                            generation,
                            publication_policy: basis.publication_policy,
                            checkpoint_authority: basis.checkpoint_authority,
                            observation_ref: pending.fresh.allocation.observation_ref().clone(),
                            observation_schema: observation_schema.schema_sha256,
                        },
                        accumulator_limits: basis.accumulator_limits,
                        checkpoint:
                            super::local_operation_coordinator::ResidentLocalOperationCheckpointInput {
                                id: pending.fresh.allocation.checkpoint_id().clone(),
                                reference: pending.fresh.allocation.checkpoint_ref().clone(),
                                schema: &checkpoint_schema.schema_sha256,
                                authority: basis.checkpoint_authority,
                            },
                        maximum_record_bytes,
                    };
                    if input.semantic.handle.issuer_key_id.as_str()
                        != basis.handle_signing_authority.signer_key_id()
                        || input.semantic.handle.issuer_key_epoch
                            != *basis.handle_signing_authority.signer_key_epoch()
                    {
                        return Err(EitherProcessInvokeStartFailure::Selection(
                            ResidentProcessHostSelectionError::StartHandleSigner,
                        ));
                    }
                    if input.semantic.handle.deployment_runtime_authority_ref
                        != selected.runtime_authority_ref
                        || input
                            .semantic
                            .handle
                            .deployment_runtime_authority_sha256
                            .as_str()
                            != selected.runtime_authority_sha256.as_str()
                        || input.semantic.handle.issuer_principal_id != selected.principal_id
                    {
                        return Err(EitherProcessInvokeStartFailure::Selection(
                            ResidentProcessHostSelectionError::StartHandlePreimageAuthority,
                        ));
                    }
                        service.start_invoke(
                            input,
                            admit_compiled_authority,
                            sign_handle,
                            sign_observation,
                            prepare_checkpoint_preimage,
                            sign_checkpoint,
                        )
                    .map_err(EitherProcessInvokeStartFailure::Start)
                },
            ) {
                Ok(result) => result,
                Err(reason) => Err(EitherProcessInvokeStartFailure::Selection(reason)),
            }
        };
        match result {
            Ok(issued) => Ok(issued),
            Err(EitherProcessInvokeStartFailure::Selection(reason)) => {
                Err(ResidentProcessAllocatedInvokeStartError::Selection {
                    pending: Box::new(pending),
                    reason,
                })
            }
            Err(EitherProcessInvokeStartFailure::Start(failure)) => match failure {
                ResidentLocalOperationStartFailure::Preparation(reason) => {
                    Err(ResidentProcessAllocatedInvokeStartError::Preparation {
                        pending: Box::new(pending),
                        reason,
                    })
                }
                failure => Err(ResidentProcessAllocatedInvokeStartError::Continuation(
                    Box::new(ResidentProcessOperationStartContinuation {
                        owner_instance: self.operations.operation_task_owner_instance().clone(),
                        selector: selected,
                        maximum_record_bytes,
                        failure: Box::new(failure),
                    }),
                )),
            },
        }
    }

    /// Start one allocated root Consult from its exact retained lease.
    ///
    /// Parent signing happens before any child allocation. A failure preserves
    /// either the retained lease or the exact signed-parent roster phase.
    #[allow(clippy::too_many_arguments)]
    fn start_allocated_consult<
        Authority,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        pending: ResidentProcessRetainedOperationLease,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        request: &CommonRequestEnvelopeV1,
        session: &super::AdmittedAmbientSession,
        current: &super::AmbientSessionAdmissionContext<'_>,
        handle_signing_authority: &SigningAuthorityBindingV1,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        policy: start_factories::ResidentStartConsultPolicy<ResidentProcessConsultFreshChildPolicy>,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        _sign_observation: ObservationSign,
        _prepare_checkpoint_preimage: CheckpointPreimage,
        _sign_checkpoint: CheckpointSign,
    ) -> Result<
        consult_start::ResidentProcessConsultationStarted,
        ResidentProcessAllocatedConsultStartError,
    >
    where
        Authority: super::AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &pale_core::records::OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let selected = pending.fresh.selector.clone();
        let index = match self.deployment_authority_index(&selected) {
            Ok(index) => index,
            Err(reason) => {
                return Err(ResidentProcessAllocatedConsultStartError::Selection {
                    pending: Box::new(pending),
                    reason,
                });
            }
        };
        if self.deployments[index].store() != access.store {
            return Err(ResidentProcessAllocatedConsultStartError::Selection {
                pending: Box::new(pending),
                reason: ResidentProcessHostSelectionError::AccessStoreAssociation,
            });
        }
        let Some(generation) = self.deployments[index]
            .application
            .generation(session.lease().compiled_generation_digest())
        else {
            return Err(ResidentProcessAllocatedConsultStartError::Selection {
                pending: Box::new(pending),
                reason: ResidentProcessHostSelectionError::StartRequestAssociation,
            });
        };
        let prepared = start_factories::with_start_consult_owner_input(
            &pending,
            start_factories::ResidentOperationStartFactoryBasis::new(
                &selected,
                request,
                session,
                current,
                generation,
                handle_signing_authority,
                checkpoint_authority,
            ),
            policy,
            |input, roster| {
                let ResidentLocalOperationStartInput {
                    semantic,
                    accumulator_limits,
                    checkpoint,
                    maximum_record_bytes,
                } = input;
                let parent_checkpoint = ResidentProcessPreparedStartCheckpoint {
                    accumulator_limits,
                    checkpoint_id: checkpoint.id,
                    checkpoint_ref: checkpoint.reference,
                    checkpoint_schema: (*checkpoint.schema).clone(),
                    maximum_record_bytes,
                };
                super::local_operation_start::issue_local_consult_parent_start(
                    semantic,
                    authority,
                    admit_compiled_authority,
                    sign_handle,
                )
                .map(|parent| {
                    (
                        start_factories::ResidentConsultationRosterPending::new(parent, roster),
                        parent_checkpoint,
                    )
                })
            },
        );
        match prepared {
            Err(reason) => Err(ResidentProcessAllocatedConsultStartError::Factory {
                pending: Box::new(pending),
                reason,
            }),
            Ok(Err(reason)) => Err(ResidentProcessAllocatedConsultStartError::Preparation {
                pending: Box::new(pending),
                reason,
            }),
            Ok(Ok((pending, parent_checkpoint))) => {
                Err(ResidentProcessAllocatedConsultStartError::Roster(Box::new(
                    ResidentProcessConsultationRosterStartContinuation {
                        owner_instance: self.operations.operation_task_owner_instance().clone(),
                        selector: selected,
                        parent_checkpoint,
                        pending,
                    },
                )))
            }
        }
    }

    /// Start one allocated pairwise Delegate from its protected parent and lease.
    #[allow(clippy::too_many_arguments)]
    fn start_allocated_delegate<
        Authority,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        pending: ResidentProcessRetainedDelegateOperationLease,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        request: &CommonRequestEnvelopeV1,
        session: &super::AdmittedAmbientSession,
        current: &super::AmbientSessionAdmissionContext<'_>,
        handle_signing_authority: &SigningAuthorityBindingV1,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        policy: start_factories::ResidentStartDelegatePolicy,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessAllocatedDelegateStartError>
    where
        Authority: super::AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &pale_core::records::OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let selected = pending.retained.fresh.selector.clone();
        let maximum_record_bytes = pending.retained.fresh.maximum_record_bytes;
        let result = self.with_selected_operation_service(
            &selected,
            access,
            authority,
            |application, service, _sessions| {
                let generation = application
                    .generation(session.lease().compiled_generation_digest())
                    .ok_or(EitherProcessConsultStartFailure::Selection(
                        ResidentProcessHostSelectionError::StartRequestAssociation,
                    ))?;
                start_factories::with_start_delegate_owner_input(
                    &pending,
                    start_factories::ResidentOperationStartFactoryBasis::new(
                        &selected,
                        request,
                        session,
                        current,
                        generation,
                        handle_signing_authority,
                        checkpoint_authority,
                    ),
                    policy,
                    |input, parent| {
                        service.start_delegate_child(
                            input,
                            parent,
                            admit_compiled_authority,
                            sign_handle,
                            sign_observation,
                            prepare_checkpoint_preimage,
                            sign_checkpoint,
                        )
                    },
                )
                .map_err(EitherProcessConsultStartFailure::Factory)?
                .map_err(EitherProcessConsultStartFailure::Start)
            },
        );
        let result = match result {
            Ok(result) => result,
            Err(reason) => {
                return Err(ResidentProcessAllocatedDelegateStartError::Selection {
                    pending: Box::new(pending),
                    reason,
                });
            }
        };
        match result {
            Ok(issued) => Ok(issued),
            Err(EitherProcessConsultStartFailure::Factory(reason)) => {
                Err(ResidentProcessAllocatedDelegateStartError::Factory {
                    pending: Box::new(pending),
                    reason,
                })
            }
            Err(EitherProcessConsultStartFailure::Selection(reason)) => {
                Err(ResidentProcessAllocatedDelegateStartError::Selection {
                    pending: Box::new(pending),
                    reason,
                })
            }
            Err(EitherProcessConsultStartFailure::Start(failure)) => match failure {
                ResidentLocalOperationStartFailure::Preparation(reason) => {
                    Err(ResidentProcessAllocatedDelegateStartError::Preparation {
                        pending: Box::new(pending),
                        reason,
                    })
                }
                failure => Err(ResidentProcessAllocatedDelegateStartError::Continuation(
                    Box::new(ResidentProcessOperationStartContinuation {
                        owner_instance: self.operations.operation_task_owner_instance().clone(),
                        selector: selected,
                        maximum_record_bytes,
                        failure: Box::new(failure),
                    }),
                )),
            },
        }
    }

    /// Start one admitted root Consult through one exact deployment.
    ///
    /// Roster selection and child creation remain separate. A returned handle
    /// has completed database and independent-anchor custody; no collection or
    /// dispatch work can begin inside this call.
    #[allow(clippy::too_many_arguments)]
    fn start_consult<
        Authority,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentLocalOperationStartInput<'_>,
        intent: Option<&super::consultation::ConsultationStartIntent>,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessConsultStartError>
    where
        Authority: super::AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &pale_core::records::OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.validate_operation_start_associations(selector, access.store, &input)?;
        let selected = selector.clone();
        let maximum_record_bytes = input.maximum_record_bytes;
        let result = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.start_consult(
                        input,
                        intent,
                        admit_compiled_authority,
                        sign_handle,
                        sign_observation,
                        prepare_checkpoint_preimage,
                        sign_checkpoint,
                    )
                },
            )
            .expect("Consult start deployment and access were selected before lending ports");
        result.map_err(|failure| {
            self.bind_consult_start_failure(selected, maximum_record_bytes, failure)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn start_consult_with_intent_builder<
        Authority,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        Intent,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentLocalOperationStartInput<'_>,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        build_intent: Intent,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessConsultStartError>
    where
        Authority: super::AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &pale_core::records::OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Intent: FnOnce(
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &pale_core::records::OperationHandleV1,
        ) -> Result<
            super::consultation::ConsultationStartIntent,
            super::LocalOperationStartError,
        >,
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.validate_operation_start_associations(selector, access.store, &input)?;
        let selected = selector.clone();
        let maximum_record_bytes = input.maximum_record_bytes;
        let result = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.start_consult_with_intent_builder(
                        input,
                        admit_compiled_authority,
                        sign_handle,
                        sign_observation,
                        build_intent,
                        prepare_checkpoint_preimage,
                        sign_checkpoint,
                    )
                },
            )
            .expect("Consult start deployment and access were selected before lending ports");
        result.map_err(|failure| {
            self.bind_consult_start_failure(selected, maximum_record_bytes, failure)
        })
    }

    /// Start one admitted Invoke or nested Consult child of a protected Consult.
    ///
    /// The parent transaction remains the membership owner. This join checks
    /// deployment and store associations before lending ports, then delegates
    /// authority narrowing and durable child custody to their existing owners.
    #[allow(clippy::too_many_arguments)]
    fn start_consultation_child<
        Authority,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentLocalOperationStartInput<'_>,
        parent: PreparedLocalChildParent,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessConsultStartError>
    where
        Authority: super::AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &pale_core::records::OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.validate_consultation_child_start_associations(
            selector,
            access.store,
            &input,
            &parent,
        )?;
        let selected = selector.clone();
        let maximum_record_bytes = input.maximum_record_bytes;
        let ResidentDeploymentOperationAccess {
            store: _,
            custody,
            sessions: _,
            anchor,
        } = access;
        let result = ResidentLocalOperationCoordinator::new(authority, custody, anchor)
            .start_consultation_child(
                input,
                parent,
                admit_compiled_authority,
                sign_handle,
                sign_observation,
                prepare_checkpoint_preimage,
                sign_checkpoint,
            );
        result.map_err(|failure| {
            self.bind_consult_start_failure(selected, maximum_record_bytes, failure)
        })
    }

    /// Start one pairwise `Delegate` child under an exact protected parent.
    ///
    /// The child does not enter a Consult roster. Parent class does not grant
    /// authority; the existing child admission proves the exact active parent,
    /// generation, lease, and narrowed authority.
    #[allow(clippy::too_many_arguments)]
    fn start_delegate_child<
        Authority,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentLocalOperationStartInput<'_>,
        parent: PreparedLocalChildParent,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessDelegateStartError>
    where
        Authority: super::AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &pale_core::records::OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.validate_delegate_child_start_associations(selector, access.store, &input, &parent)?;
        let selected = selector.clone();
        let maximum_record_bytes = input.maximum_record_bytes;
        let result = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.start_delegate_child(
                        input,
                        parent,
                        admit_compiled_authority,
                        sign_handle,
                        sign_observation,
                        prepare_checkpoint_preimage,
                        sign_checkpoint,
                    )
                },
            )
            .expect("Delegate child deployment and access were selected before lending ports");
        result.map_err(|failure| {
            self.bind_delegate_start_failure(selected, maximum_record_bytes, failure)
        })
    }

    fn bind_consult_start_failure(
        &self,
        selector: ResidentDeploymentAuthoritySelector,
        maximum_record_bytes: usize,
        failure: ResidentLocalOperationStartFailure,
    ) -> ResidentProcessConsultStartError {
        match failure {
            ResidentLocalOperationStartFailure::Preparation(reason) => {
                ResidentProcessConsultStartError::Preparation(reason)
            }
            failure => ResidentProcessConsultStartError::Continuation(Box::new(
                ResidentProcessOperationStartContinuation {
                    owner_instance: self.operations.operation_task_owner_instance().clone(),
                    selector: selector.clone(),
                    maximum_record_bytes,
                    failure: Box::new(failure),
                },
            )),
        }
    }

    fn bind_delegate_start_failure(
        &self,
        selector: ResidentDeploymentAuthoritySelector,
        maximum_record_bytes: usize,
        failure: ResidentLocalOperationStartFailure,
    ) -> ResidentProcessDelegateStartError {
        match failure {
            ResidentLocalOperationStartFailure::Preparation(reason) => {
                ResidentProcessDelegateStartError::Preparation(reason)
            }
            failure => ResidentProcessDelegateStartError::Continuation(Box::new(
                ResidentProcessOperationStartContinuation {
                    owner_instance: self.operations.operation_task_owner_instance().clone(),
                    selector,
                    maximum_record_bytes,
                    failure: Box::new(failure),
                },
            )),
        }
    }

    fn validate_consultation_child_start_associations(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &AuthorityStoreCoordinate,
        input: &ResidentLocalOperationStartInput<'_>,
        parent: &PreparedLocalChildParent,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        self.validate_operation_start_associations(selector, access_store, input)?;
        let (parent_ref, parent_handle) = parent.operation();
        let expected_parent = input
            .semantic
            .handle
            .parent_operation
            .as_ref()
            .ok_or(ResidentProcessHostSelectionError::ConsultationChildAssociation)?;
        if parent.store() != access_store
            || !selector.matches_handle(parent_handle)
            || parent_handle.operation_class() != &OperationClassV1::Consult
        {
            return Err(ResidentProcessHostSelectionError::ConsultationParentAuthority);
        }
        if !matches!(
            input.semantic.handle.operation_class,
            OperationClassV1::Invoke | OperationClassV1::Consult
        ) || expected_parent.operation_handle_ref != *parent_ref
            || expected_parent.operation_handle_id != *parent_handle.operation_handle_id()
            || expected_parent.operation_handle_sha256.as_str()
                != parent_handle.canonical_operation_handle_sha256().as_str()
        {
            return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation);
        }
        Ok(())
    }

    fn validate_delegate_child_start_associations(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &AuthorityStoreCoordinate,
        input: &ResidentLocalOperationStartInput<'_>,
        parent: &PreparedLocalChildParent,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        self.validate_operation_start_associations(selector, access_store, input)?;
        let (parent_ref, parent_handle) = parent.operation();
        let expected_parent = input
            .semantic
            .handle
            .parent_operation
            .as_ref()
            .ok_or(ResidentProcessHostSelectionError::DelegateChildAssociation)?;
        if parent.store() != access_store || !selector.matches_handle(parent_handle) {
            return Err(ResidentProcessHostSelectionError::DelegateParentAuthority);
        }
        if input.semantic.handle.operation_class != OperationClassV1::Delegate
            || expected_parent.operation_handle_ref != *parent_ref
            || expected_parent.operation_handle_id != *parent_handle.operation_handle_id()
            || expected_parent.operation_handle_sha256.as_str()
                != parent_handle.canonical_operation_handle_sha256().as_str()
        {
            return Err(ResidentProcessHostSelectionError::DelegateChildAssociation);
        }
        Ok(())
    }

    /// Reselect one retained start phase before lending any remaining owner.
    fn continue_selected_operation_start<Authority>(
        &mut self,
        pending: ResidentProcessOperationStartContinuation,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        continue_phase: impl FnOnce(
            &mut ResidentLocalOperationCoordinator<'_, Authority>,
            ResidentLocalOperationStartFailure,
            usize,
        ) -> Result<
            super::IssuedLocalOperation,
            ResidentLocalOperationStartFailure,
        >,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessOperationStartContinuationFailure> {
        if !pending
            .owner_instance
            .is_same(self.operations.operation_task_owner_instance())
        {
            return Err(ResidentProcessOperationStartContinuationFailure {
                pending: Box::new(pending),
                reason: ResidentProcessOperationStartContinuationError::Selection(
                    ResidentProcessHostSelectionError::StartProcessOwner,
                ),
            });
        }
        let index = match self.deployment_authority_index(&pending.selector) {
            Ok(index) => index,
            Err(reason) => {
                return Err(ResidentProcessOperationStartContinuationFailure {
                    pending: Box::new(pending),
                    reason: ResidentProcessOperationStartContinuationError::Selection(reason),
                });
            }
        };
        if self.deployments[index].store() != access.store {
            return Err(ResidentProcessOperationStartContinuationFailure {
                pending: Box::new(pending),
                reason: ResidentProcessOperationStartContinuationError::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation,
                ),
            });
        }
        let ResidentProcessOperationStartContinuation {
            owner_instance: _,
            selector,
            maximum_record_bytes,
            failure,
        } = pending;
        let ResidentDeploymentOperationAccess {
            store: _,
            custody,
            sessions: _,
            anchor,
        } = access;
        let result = continue_phase(
            &mut ResidentLocalOperationCoordinator::new(authority, custody, anchor),
            *failure,
            maximum_record_bytes,
        );
        result.map_err(|failure| ResidentProcessOperationStartContinuationFailure {
            pending: Box::new(ResidentProcessOperationStartContinuation {
                owner_instance: self.operations.operation_task_owner_instance().clone(),
                selector,
                maximum_record_bytes,
                failure: Box::new(failure),
            }),
            reason: ResidentProcessOperationStartContinuationError::Phase,
        })
    }

    /// Continue only final checkpoint attachment or durable custody.
    ///
    /// Earlier phases use their phase-specific methods so this path never asks
    /// for signers or deterministic preparation inputs that are already spent.
    fn continue_operation_start<Authority>(
        &mut self,
        pending: ResidentProcessOperationStartContinuation,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessOperationStartContinuationFailure> {
        self.continue_selected_operation_start(
            pending,
            access,
            authority,
            |coordinator, failure, maximum_record_bytes| match failure {
                ResidentLocalOperationStartFailure::Custody(failure) => coordinator
                    .continue_start_custody(failure, maximum_record_bytes)
                    .map_err(ResidentLocalOperationStartFailure::Custody),
                ResidentLocalOperationStartFailure::CheckpointAttachment(failure) => {
                    coordinator.continue_checkpoint_attachment(*failure, maximum_record_bytes)
                }
                failure => Err(failure),
            },
        )
    }

    /// Commit one semantically prepared start through the selected physical owner.
    ///
    /// Consult roster preparation reaches this boundary only after the parent
    /// handle, ordered child identities, private start intent, and initial
    /// observation are fixed. Selection failure therefore returns that exact
    /// prepared parent. Once physical work begins, the ordinary start
    /// continuation becomes the only retry authority.
    #[allow(clippy::too_many_arguments)]
    fn commit_prepared_operation_start<Authority, CheckpointPreimage, CheckpointSign>(
        &mut self,
        owner_instance: ResidentOperationTaskOwnerInstance,
        selector: ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        prepared: super::PreparedLocalOperationStart,
        accumulator_limits: pale_ports::AccumulatorPreparationLimits,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        maximum_record_bytes: usize,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessPreparedOperationStartFailure>
    where
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let selected = self
            .deployment_authority_index(&selector)
            .ok()
            .filter(|index| self.deployments[*index].store() == access.store);
        if !owner_instance.is_same(self.operations.operation_task_owner_instance())
            || selected.is_none()
        {
            return Err(ResidentProcessPreparedOperationStartFailure::Prepared {
                owner_instance,
                selector: Box::new(selector),
                prepared: Box::new(prepared),
            });
        }
        let ResidentDeploymentOperationAccess {
            store: _,
            custody,
            sessions: _,
            anchor,
        } = access;
        ResidentLocalOperationCoordinator::new(authority, custody, anchor)
            .finish_semantic_start(
                prepared,
                accumulator_limits,
                checkpoint,
                maximum_record_bytes,
                prepare_checkpoint_preimage,
                sign_checkpoint,
            )
            .map_err(|failure| {
                ResidentProcessPreparedOperationStartFailure::Continuation(Box::new(
                    ResidentProcessOperationStartContinuation {
                        owner_instance: self.operations.operation_task_owner_instance().clone(),
                        selector,
                        maximum_record_bytes,
                        failure: Box::new(failure),
                    },
                ))
            })
    }

    /// Continue initial-observation signing around one already signed handle.
    #[allow(clippy::too_many_arguments)]
    fn continue_handle_issued_operation_start<
        Authority,
        ObservationSign,
        Intent,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        pending: ResidentProcessOperationStartContinuation,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        sign_observation: ObservationSign,
        build_intent: Intent,
        accumulator_limits: pale_ports::AccumulatorPreparationLimits,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessOperationStartContinuationFailure>
    where
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Intent: FnOnce(
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &pale_core::records::OperationHandleV1,
        ) -> Result<
            Option<super::consultation::ConsultationStartIntent>,
            super::LocalOperationStartError,
        >,
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.continue_selected_operation_start(
            pending,
            access,
            authority,
            move |coordinator, failure, maximum_record_bytes| match failure {
                ResidentLocalOperationStartFailure::HandleIssued(failure) => coordinator
                    .continue_handle_issued_semantic_start(
                        *failure,
                        sign_observation,
                        build_intent,
                        accumulator_limits,
                        checkpoint,
                        maximum_record_bytes,
                        prepare_checkpoint_preimage,
                        sign_checkpoint,
                    ),
                failure => Err(failure),
            },
        )
    }

    /// Continue deterministic semantic projection after both start records are signed.
    #[allow(clippy::too_many_arguments)]
    fn continue_signed_semantic_operation_start<
        Authority,
        Intent,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        pending: ResidentProcessOperationStartContinuation,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        build_intent: Intent,
        accumulator_limits: pale_ports::AccumulatorPreparationLimits,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessOperationStartContinuationFailure>
    where
        Intent: FnOnce(
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &pale_core::records::OperationHandleV1,
        ) -> Result<
            Option<super::consultation::ConsultationStartIntent>,
            super::LocalOperationStartError,
        >,
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.continue_selected_operation_start(
            pending,
            access,
            authority,
            move |coordinator, failure, maximum_record_bytes| match failure {
                ResidentLocalOperationStartFailure::SignedSemanticStart(failure) => coordinator
                    .continue_signed_semantic_start(
                        *failure,
                        build_intent,
                        accumulator_limits,
                        checkpoint,
                        maximum_record_bytes,
                        prepare_checkpoint_preimage,
                        sign_checkpoint,
                    ),
                failure => Err(failure),
            },
        )
    }

    /// Continue native checkpoint preparation around the exact signed start.
    #[allow(clippy::too_many_arguments)]
    fn continue_signed_operation_start<Authority, CheckpointPreimage, CheckpointSign>(
        &mut self,
        pending: ResidentProcessOperationStartContinuation,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        accumulator_limits: pale_ports::AccumulatorPreparationLimits,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessOperationStartContinuationFailure>
    where
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.continue_selected_operation_start(
            pending,
            access,
            authority,
            move |coordinator, failure, maximum_record_bytes| match failure {
                ResidentLocalOperationStartFailure::SignedStart(failure) => coordinator
                    .continue_signed_start(
                        *failure,
                        accumulator_limits,
                        checkpoint,
                        maximum_record_bytes,
                        prepare_checkpoint_preimage,
                        sign_checkpoint,
                    ),
                failure => Err(failure),
            },
        )
    }

    /// Continue checkpoint preimage construction after native preparation completed.
    #[allow(clippy::too_many_arguments)]
    fn continue_prepared_checkpoint_operation_start<Authority, CheckpointPreimage, CheckpointSign>(
        &mut self,
        pending: ResidentProcessOperationStartContinuation,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessOperationStartContinuationFailure>
    where
        CheckpointPreimage:
            FnOnce(
                &super::PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, super::LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.continue_selected_operation_start(
            pending,
            access,
            authority,
            move |coordinator, failure, maximum_record_bytes| match failure {
                ResidentLocalOperationStartFailure::PreparedCheckpoint {
                    continuation,
                    failure: _,
                } => coordinator.continue_prepared_checkpoint(
                    *continuation,
                    checkpoint,
                    maximum_record_bytes,
                    prepare_checkpoint_preimage,
                    sign_checkpoint,
                ),
                failure => Err(failure),
            },
        )
    }

    /// Continue checkpoint validation or signing from the retained prepared root.
    fn continue_checkpoint_issuance_operation_start<Authority, CheckpointSign>(
        &mut self,
        pending: ResidentProcessOperationStartContinuation,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        sign_checkpoint: CheckpointSign,
    ) -> Result<super::IssuedLocalOperation, ResidentProcessOperationStartContinuationFailure>
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.continue_selected_operation_start(
            pending,
            access,
            authority,
            move |coordinator, failure, maximum_record_bytes| match failure {
                ResidentLocalOperationStartFailure::CheckpointIssuance {
                    continuation,
                    preimage,
                } => coordinator.continue_checkpoint_issuance(
                    *continuation,
                    checkpoint,
                    preimage,
                    maximum_record_bytes,
                    sign_checkpoint,
                ),
                failure => Err(failure),
            },
        )
    }

    /// Check every deployment association visible before the start owner runs.
    fn validate_operation_start_associations(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &AuthorityStoreCoordinate,
        input: &ResidentLocalOperationStartInput<'_>,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        let semantic = &input.semantic;
        self.validate_operation_start_context_associations(
            selector,
            access_store,
            &ResidentProcessOperationStartContext {
                session: semantic.session,
                current: semantic.current,
                request: semantic.request,
                handle_signing_authority: semantic.context.signed.signing_authority,
                semantic_checkpoint_authority: semantic.checkpoint_authority,
                issuance_checkpoint_authority: input.checkpoint.authority,
                maximum_record_bytes: input.maximum_record_bytes,
            },
        )?;
        let handle_signing_authority = semantic.context.signed.signing_authority;
        if semantic.handle.issuer_key_id.as_str() != handle_signing_authority.signer_key_id()
            || semantic.handle.issuer_key_epoch != *handle_signing_authority.signer_key_epoch()
        {
            return Err(ResidentProcessHostSelectionError::StartHandleSigner);
        }
        if semantic.handle.deployment_runtime_authority_ref != selector.runtime_authority_ref
            || semantic.handle.deployment_runtime_authority_sha256.as_str()
                != selector.runtime_authority_sha256.as_str()
            || semantic.handle.issuer_principal_id != selector.principal_id
        {
            return Err(ResidentProcessHostSelectionError::StartHandlePreimageAuthority);
        }
        Ok(())
    }

    /// Check every deployment association before the allocation request lookup.
    fn validate_operation_start_context_associations(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &AuthorityStoreCoordinate,
        context: &ResidentProcessOperationStartContext<'_>,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(selector)?;
        let selected_store = self.deployments[index].store();
        if selected_store != access_store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        if !selector.matches_current_session(context.current)
            || !selector
                .matches_signing_authority(context.current.generation.signed.signing_authority)
        {
            return Err(ResidentProcessHostSelectionError::StartCurrentAuthority);
        }
        if !selector.matches_admitted_session(context.session) {
            return Err(ResidentProcessHostSelectionError::StartSessionAuthority);
        }
        let session = context.session.lease();
        let admission = context.request.session_admission();
        let application = &self.deployments[index].application;
        if &admission.session_admission_id != session.session_admission_id()
            || &admission.session_admission_lease_digest != session.session_admission_lease_digest()
            || application
                .generation(session.compiled_generation_digest())
                .and_then(|generation| {
                    generation.operation(session.repository_id(), context.request.operation())
                })
                .is_none()
        {
            return Err(ResidentProcessHostSelectionError::StartRequestAssociation);
        }
        if !selector
            .matches_signing_authority(&context.request.to_preimage().signing_authority_binding)
        {
            return Err(ResidentProcessHostSelectionError::CurrentRequestAuthority);
        }
        let handle_signing_authority = context.handle_signing_authority;
        if !selector.matches_signing_authority(handle_signing_authority) {
            return Err(ResidentProcessHostSelectionError::StartHandleAuthority);
        }
        for checkpoint in [
            context.semantic_checkpoint_authority,
            context.issuance_checkpoint_authority,
        ] {
            if checkpoint.principal_id != &selector.principal_id
                || checkpoint.control_store_id != selected_store.store_id()
                || checkpoint.control_incarnation_id != selected_store.incarnation_id()
            {
                return Err(ResidentProcessHostSelectionError::CheckpointAuthorityAssociation);
            }
        }
        if !same_checkpoint_authority(
            context.semantic_checkpoint_authority,
            context.issuance_checkpoint_authority,
        ) {
            return Err(ResidentProcessHostSelectionError::StartCheckpointAuthority);
        }
        Ok(())
    }

    /// Run application-owned Observe through one exact admitted deployment.
    ///
    /// This private join rejects the current request authority, historical
    /// handle authority, recovery store, checkpoint principal/store, and access
    /// store before any custody, session, or anchor I/O. It adds no observation
    /// semantics; successful joins delegate to [`ResidentOperationService::observe`].
    fn observe<Authority>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: LocalOperationObservationInput<'_>,
        admission: LocalOperationObservationAdmission<'_>,
    ) -> Result<OperationControlObservation, ResidentProcessObservationError> {
        let request = input.request.to_preimage();
        let context = &input.recovery.operation.context;
        self.with_selected_operation_control_service(
            selector,
            ResidentOperationControlAssociation {
                current_request_authority: &request.signing_authority_binding,
                historical_handle_authority: context.handle_signing_authority,
                recovery_store: context.store,
                checkpoint_authority: context.checkpoint_authority,
            },
            access,
            authority,
            // Observe needs no mutable application projection after the exact
            // deployment joins above. It is still lent by the aggregate so this
            // route shares the same ownership shape as later resident use cases.
            |_application, service, sessions| service.observe(input, sessions, admission),
        )?
        .map_err(ResidentProcessObservationError::Observe)
    }

    /// Run application-owned Cancel through one exact admitted deployment.
    ///
    /// This private join applies the same current-request, historical-handle,
    /// recovery-store, checkpoint, and access-store association checks as
    /// Observe before any custody, session, or anchor I/O. It adds no
    /// cancellation semantics; successful joins delegate to
    /// [`ResidentOperationService::request_cancellation`].
    fn request_cancellation<Authority>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentOperationCancellationInput<'_>,
        local_completion: ResidentLocalCancellationCompletionInput,
        admission: ResidentOperationCancellationAdmission<'_>,
    ) -> Result<ResidentProcessCancellationOutcome, ResidentProcessCancellationError> {
        let request = input.request.to_preimage();
        let context = &input.recovery.operation.context;
        let outcome = self.with_selected_operation_control_service(
            selector,
            ResidentOperationControlAssociation {
                current_request_authority: &request.signing_authority_binding,
                historical_handle_authority: context.handle_signing_authority,
                recovery_store: context.store,
                checkpoint_authority: context.checkpoint_authority,
            },
            access,
            authority,
            |_application, service, sessions| {
                service.request_cancellation(input, Some(local_completion), sessions, admission)
            },
        )?;
        let outcome = bind_cancellation_custody(selector, outcome)?;
        Ok(self.bind_process_cancellation_outcome(selector, outcome))
    }

    /// Continue Cancel from one exact point recovery admitted by the process owner.
    #[allow(clippy::too_many_arguments)]
    fn request_cancellation_from_recovered<Authority>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        recovered: RecoveredOperationControl,
        input: ResidentOperationCancellationInput<'_>,
        local_completion: ResidentLocalCancellationCompletionInput,
        admission: ResidentOperationCancellationAdmission<'_>,
    ) -> Result<ResidentProcessCancellationOutcome, ResidentProcessCancellationError> {
        let request = input.request.to_preimage();
        let context = &input.recovery.operation.context;
        let outcome = self.with_selected_operation_control_service(
            selector,
            ResidentOperationControlAssociation {
                current_request_authority: &request.signing_authority_binding,
                historical_handle_authority: context.handle_signing_authority,
                recovery_store: context.store,
                checkpoint_authority: context.checkpoint_authority,
            },
            access,
            authority,
            |_application, service, _sessions| {
                service.request_cancellation_from_recovered(
                    recovered,
                    input,
                    Some(local_completion),
                    admission,
                )
            },
        )?;
        let outcome = bind_cancellation_custody(selector, outcome)?;
        Ok(self.bind_process_cancellation_outcome(selector, outcome))
    }

    /// Continue one exact pre-dispatch cancellation custody candidate.
    fn continue_cancellation_custody<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: Box<ResidentProcessCancellationCustody>,
    ) -> Result<(), ResidentProcessCancellationContinuationFailure> {
        let index = match self.deployment_authority_index(&pending.selector) {
            Ok(index) => index,
            Err(reason) => {
                return Err(ResidentProcessCancellationContinuationFailure {
                    pending,
                    reason: ResidentProcessCancellationContinuationError::Selection(reason),
                });
            }
        };
        if self.deployments[index].store() != access.store {
            return Err(ResidentProcessCancellationContinuationFailure {
                pending,
                reason: ResidentProcessCancellationContinuationError::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation,
                ),
            });
        }
        let ResidentProcessCancellationCustody { selector, pending } = *pending;
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let result =
            self.operations
                .with_operation_service(authority, custody, anchor, |service| {
                    service.continue_cancellation_custody(pending)
                });
        result.map_err(|pending| ResidentProcessCancellationContinuationFailure {
            pending: Box::new(ResidentProcessCancellationCustody { selector, pending }),
            reason: ResidentProcessCancellationContinuationError::Custody,
        })
    }

    /// Continue one exact dispatched-cancellation custody candidate.
    ///
    /// The embedded deployment and supplied access store are rechecked before
    /// the candidate can re-enter its process-local task owner. This path does
    /// not rebuild, re-sign, or replay the retained cancellation.
    fn continue_dispatched_cancellation_custody<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: Box<ResidentProcessDispatchedCancellationCustody>,
    ) -> Result<
        ResidentProcessCancellationOutcome,
        ResidentProcessDispatchedCancellationContinuationFailure,
    > {
        let index = match self.deployment_authority_index(&pending.selector) {
            Ok(index) => index,
            Err(reason) => {
                return Err(ResidentProcessDispatchedCancellationContinuationFailure {
                    pending,
                    reason: ResidentProcessDispatchedCancellationContinuationError::Selection(
                        reason,
                    ),
                });
            }
        };
        if self.deployments[index].store() != access.store {
            return Err(ResidentProcessDispatchedCancellationContinuationFailure {
                pending,
                reason: ResidentProcessDispatchedCancellationContinuationError::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation,
                ),
            });
        }
        let ResidentProcessDispatchedCancellationCustody { selector, pending } = *pending;
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let result =
            self.operations
                .with_operation_service(authority, custody, anchor, |service| {
                    service.continue_dispatched_cancellation_custody(pending)
                });
        let outcome =
            result.map_err(
                |pending| ResidentProcessDispatchedCancellationContinuationFailure {
                    pending: Box::new(ResidentProcessDispatchedCancellationCustody {
                        selector: selector.clone(),
                        pending,
                    }),
                    reason: ResidentProcessDispatchedCancellationContinuationError::Custody,
                },
            )?;
        Ok(self.bind_process_cancellation_outcome(&selector, outcome))
    }

    /// Join a lower cancellation result to any process-owned deployment route.
    fn bind_process_cancellation_outcome(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        outcome: ResidentOperationCancellationOutcome,
    ) -> ResidentProcessCancellationOutcome {
        match outcome {
            ResidentOperationCancellationOutcome::A2aDelegateReadStopped {
                custody,
                checkpoint,
                continuation,
            } => {
                let (key, poll) = continuation.into_parts();
                ResidentProcessCancellationOutcome::A2aDelegateReadStopped {
                    custody,
                    checkpoint,
                    continuation: self.join_routed_a2a_poll(key, selector.clone(), poll),
                }
            }
            outcome => ResidentProcessCancellationOutcome::Operation(outcome),
        }
    }

    /// Prove one recovered pairwise Delegate before provider release.
    ///
    /// The recovered child, optional context binding, provider input, checkpoint,
    /// and deployment must agree. Only then does the existing provider owner
    /// retain the dispatch and offer it to the process task executor.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    fn launch_recovered_pairwise_provider<
        'operation,
        'call,
        Authority,
        Executor,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        recovered: &'operation restart::ResidentProcessRecoveredOperationResolution,
        access: ResidentDeploymentOperationAccess<'_>,
        resolution: operation_resolution::ResidentProcessOperationResolutionInput<'operation>,
        completed_context: Option<
            &operation_resolution::CompletedResidentProcessOperationContext<'_>,
        >,
        authority: &mut Authority,
        dispatch: ProviderOperationDispatchInput<'operation, 'call>,
        invocation: &ProviderOperationInvokeInput<'_>,
        executor: &mut Executor,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        Result<
            ResidentOperationTaskKey,
            Box<ResidentProcessProviderLaunchFailure<'operation, 'call>>,
        >,
        operation_resolution::ResidentProcessOperationResolutionFailure,
    >
    where
        Authority: super::AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<
            OwnedProviderInvocationRequest,
            ProcessProviderTaskOutput,
        >,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let selector = &recovered.start.selector;
        let index = self.deployment_authority_index(selector)?;
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let completed = if let Some(completed) = completed_context {
            Self::validate_completed_operation_context(recovered, completed)?;
            Some(Self::completed_operation_context_parts(completed))
        } else {
            None
        };
        let validation = {
            let application = self.deployments[index].application.as_ref();
            Self::with_operation_resolution_context_in(
                application,
                selector,
                recovered,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                resolution,
                authority,
                completed,
                |resolved, checkpoint, _, _| {
                    validate_recovered_pairwise_provider_dispatch(
                        application,
                        selector,
                        recovered,
                        &resolved,
                        checkpoint,
                        &dispatch,
                    )
                },
            )?
        };
        if let Err(reason) = validation {
            return Ok(Err(Box::new(
                ResidentProcessProviderLaunchFailure::Selection {
                    dispatch: Box::new(dispatch),
                    reason,
                },
            )));
        }
        Ok(self.launch_provider_as(
            ResidentProcessProviderLaunchKind::PairwiseDelegate,
            selector,
            ResidentDeploymentOperationAccess {
                store,
                custody,
                sessions,
                anchor,
            },
            authority,
            dispatch,
            invocation,
            executor,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        ))
    }

    /// Retain and release one already-issued external-provider dispatch.
    ///
    /// The complete deployment association and a preallocated route slot are
    /// established before durable retention or executor I/O. Durable retention
    /// completes before the only ordinary provider call can enter the executor.
    #[allow(clippy::too_many_arguments)]
    fn launch_provider<
        'operation,
        'call,
        Authority,
        Executor,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        dispatch: ProviderOperationDispatchInput<'operation, 'call>,
        invocation: &ProviderOperationInvokeInput<'_>,
        executor: &mut Executor,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentOperationTaskKey,
        Box<ResidentProcessProviderLaunchFailure<'operation, 'call>>,
    >
    where
        Authority: super::AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<
            OwnedProviderInvocationRequest,
            ProcessProviderTaskOutput,
        >,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.launch_provider_as(
            ResidentProcessProviderLaunchKind::Invoke,
            selector,
            access,
            authority,
            dispatch,
            invocation,
            executor,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn launch_provider_as<
        'operation,
        'call,
        Authority,
        Executor,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        kind: ResidentProcessProviderLaunchKind,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        dispatch: ProviderOperationDispatchInput<'operation, 'call>,
        invocation: &ProviderOperationInvokeInput<'_>,
        executor: &mut Executor,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentOperationTaskKey,
        Box<ResidentProcessProviderLaunchFailure<'operation, 'call>>,
    >
    where
        Authority: super::AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<
            OwnedProviderInvocationRequest,
            ProcessProviderTaskOutput,
        >,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let index = match self.deployment_authority_index(selector) {
            Ok(index) => index,
            Err(reason) => {
                return Err(Box::new(ResidentProcessProviderLaunchFailure::Selection {
                    dispatch: Box::new(dispatch),
                    reason,
                }));
            }
        };
        let selected_store = self.deployments[index].store().clone();
        if access.store != &selected_store {
            return Err(Box::new(ResidentProcessProviderLaunchFailure::Selection {
                dispatch: Box::new(dispatch),
                reason: ResidentProcessHostSelectionError::AccessStoreAssociation,
            }));
        }
        if let Err(reason) =
            validate_provider_launch_associations(selector, &selected_store, &dispatch, invocation)
        {
            return Err(Box::new(ResidentProcessProviderLaunchFailure::Selection {
                dispatch: Box::new(dispatch),
                reason,
            }));
        }
        let reservation = match self.provider_routes.reserve(selector.clone()) {
            Ok(reservation) => reservation,
            Err(()) => {
                return Err(Box::new(
                    ResidentProcessProviderLaunchFailure::RouteSaturated(Box::new(dispatch)),
                ));
            }
        };

        let launched = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    let retained = match service.retain_provider_dispatch(
                        dispatch,
                        admit_rotation,
                        sign_observation,
                        sign_checkpoint,
                    ) {
                        Ok(retained) => retained,
                        Err(failure) => {
                            return ResidentProcessProviderLaunchDisposition::Rejected(
                                ResidentProcessProviderLaunchFailure::Retention {
                                    reservation,
                                    failure,
                                },
                            );
                        }
                    };
                    if retained.custody().candidate().store() != &selected_store {
                        return ResidentProcessProviderLaunchDisposition::Rejected(
                            ResidentProcessProviderLaunchFailure::RetainedStoreAssociation {
                                reservation,
                                retained: Box::new(retained),
                            },
                        );
                    }
                    let accepted = match kind {
                        ResidentProcessProviderLaunchKind::Invoke => {
                            service.accept_retained_provider(retained, invocation, executor)
                        }
                        ResidentProcessProviderLaunchKind::PairwiseDelegate => service
                            .accept_retained_pairwise_delegate_provider(
                                retained, invocation, executor,
                            ),
                    };
                    match accepted {
                        Ok(key) => {
                            ResidentProcessProviderLaunchDisposition::Accepted { reservation, key }
                        }
                        Err(failure) => {
                            if let Some(key) = accepted_provider_key(&failure).cloned() {
                                ResidentProcessProviderLaunchDisposition::AcceptedWithFailure {
                                    reservation,
                                    key,
                                    failure,
                                }
                            } else {
                                ResidentProcessProviderLaunchDisposition::Rejected(
                                    ResidentProcessProviderLaunchFailure::Acceptance {
                                        reservation,
                                        failure,
                                    },
                                )
                            }
                        }
                    }
                },
            )
            .expect("deployment and access were selected before provider retention");

        match launched {
            ResidentProcessProviderLaunchDisposition::Accepted { reservation, key } => {
                self.provider_routes
                    .commit(reservation, key.clone())
                    .unwrap_or_else(|_| unreachable!("route reservation came from this host"));
                Ok(key)
            }
            ResidentProcessProviderLaunchDisposition::AcceptedWithFailure {
                reservation,
                key,
                failure,
            } => {
                let selector = reservation.selector.clone();
                self.provider_routes
                    .commit(reservation, key.clone())
                    .unwrap_or_else(|_| unreachable!("route reservation came from this host"));
                Err(Box::new(
                    ResidentProcessProviderLaunchFailure::AcceptedWithFailure {
                        selector,
                        key,
                        failure,
                    },
                ))
            }
            ResidentProcessProviderLaunchDisposition::Rejected(failure) => Err(Box::new(failure)),
        }
    }

    /// Poll one provider completion and attach its retained deployment route.
    ///
    /// Polling performs no custody, signing, checkpoint, or application I/O.
    /// Completed and lost work move the route slot into the returned linear
    /// phase. A rejected aggregate transition leaves its route active.
    fn poll_provider_task<Executor>(
        &mut self,
        executor: &mut Executor,
    ) -> Option<ResidentProcessProviderCompletionPoll>
    where
        Executor: ResidentOperationTaskExecutor<
            OwnedProviderInvocationRequest,
            ProcessProviderTaskOutput,
        >,
    {
        let completion = self.operations.poll_provider_task(executor)?;
        match &completion {
            ResidentProviderCompletionPoll::Stale(_) => {
                return Some(ResidentProcessProviderCompletionPoll::Stale(Box::new(
                    completion,
                )));
            }
            ResidentProviderCompletionPoll::AggregateTransitionRejected {
                completion: rejected,
                ..
            } => {
                let Some(selector) = self.provider_routes.selector(rejected.key()).cloned() else {
                    return Some(ResidentProcessProviderCompletionPoll::Unrouted(Box::new(
                        completion,
                    )));
                };
                return Some(ResidentProcessProviderCompletionPoll::Routed(Box::new(
                    ResidentProcessProviderCompletion {
                        route: ResidentProcessProviderCompletionRoute::Active { selector },
                        completion,
                    },
                )));
            }
            ResidentProviderCompletionPoll::Completed(_)
            | ResidentProviderCompletionPoll::Lost(_) => {}
        }
        let key = match &completion {
            ResidentProviderCompletionPoll::Completed(completed) => completed.key(),
            ResidentProviderCompletionPoll::Lost(lost) => lost.key(),
            ResidentProviderCompletionPoll::Stale(_)
            | ResidentProviderCompletionPoll::AggregateTransitionRejected { .. } => {
                unreachable!("nonterminal poll branches returned above")
            }
        };
        let Some(custody) = self.provider_routes.check_out(key) else {
            return Some(ResidentProcessProviderCompletionPoll::Unrouted(Box::new(
                completion,
            )));
        };
        Some(ResidentProcessProviderCompletionPoll::Routed(Box::new(
            ResidentProcessProviderCompletion {
                route: ResidentProcessProviderCompletionRoute::CheckedOut(custody),
                completion,
            },
        )))
    }

    /// Retry only a rejected aggregate transition for already-returned work.
    ///
    /// The retained active route is checked out only after the aggregate owner
    /// accepts the completion or loss. This method performs no executor,
    /// custody, signing, checkpoint, or application I/O.
    fn continue_provider_aggregate_transition(
        &mut self,
        progress: ResidentProcessProviderCompletion,
    ) -> ResidentProcessProviderCompletion {
        let ResidentProcessProviderCompletion { route, completion } = progress;
        let (selector, completion) = match (route, completion) {
            (
                ResidentProcessProviderCompletionRoute::Active { selector },
                ResidentProviderCompletionPoll::AggregateTransitionRejected { completion, .. },
            ) => (selector, completion),
            (route, completion) => {
                return ResidentProcessProviderCompletion { route, completion };
            }
        };
        let key = completion.key().clone();
        if self.provider_routes.selector(&key) != Some(&selector) {
            return ResidentProcessProviderCompletion {
                route: ResidentProcessProviderCompletionRoute::Active { selector },
                completion: ResidentProviderCompletionPoll::AggregateTransitionRejected {
                    completion,
                    reason:
                        super::task_reservation::ResidentTaskReservationTransitionFailure::NotFound,
                },
            };
        }
        let completion = self
            .operations
            .retry_provider_completion_transition(*completion);
        let route = match &completion {
            ResidentProviderCompletionPoll::Completed(completed) => {
                self.provider_routes.check_out(completed.key())
            }
            ResidentProviderCompletionPoll::Lost(lost) => {
                self.provider_routes.check_out(lost.key())
            }
            ResidentProviderCompletionPoll::AggregateTransitionRejected { .. }
            | ResidentProviderCompletionPoll::Stale(_) => None,
        };
        ResidentProcessProviderCompletion {
            route: route.map_or(
                ResidentProcessProviderCompletionRoute::Active { selector },
                ResidentProcessProviderCompletionRoute::CheckedOut,
            ),
            completion,
        }
    }

    /// Release only the route slot for work proved not to have been accepted.
    ///
    /// The returned value preserves the lower retained phase. Provider work,
    /// durable dispatch, signatures, and checkpoints are not repeated or erased.
    fn abandon_unaccepted_provider_launch<'operation, 'call>(
        &mut self,
        failure: ResidentProcessProviderLaunchFailure<'operation, 'call>,
    ) -> Result<
        ResidentProcessProviderAbandonedLaunch<'operation, 'call>,
        Box<ResidentProcessProviderLaunchFailure<'operation, 'call>>,
    > {
        match failure {
            ResidentProcessProviderLaunchFailure::Retention {
                reservation,
                failure,
            } => match self.provider_routes.release_reservation(reservation) {
                Ok(()) => Ok(ResidentProcessProviderAbandonedLaunch::Retention(failure)),
                Err(reservation) => {
                    Err(Box::new(ResidentProcessProviderLaunchFailure::Retention {
                        reservation: *reservation,
                        failure,
                    }))
                }
            },
            ResidentProcessProviderLaunchFailure::RetainedStoreAssociation {
                reservation,
                retained,
            } => match self.provider_routes.release_reservation(reservation) {
                Ok(()) => {
                    Ok(ResidentProcessProviderAbandonedLaunch::RetainedStoreAssociation(retained))
                }
                Err(reservation) => Err(Box::new(
                    ResidentProcessProviderLaunchFailure::RetainedStoreAssociation {
                        reservation: *reservation,
                        retained,
                    },
                )),
            },
            ResidentProcessProviderLaunchFailure::Acceptance {
                reservation,
                failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
            } if is_unaccepted_provider_aggregate_transition(&refusal) => {
                Err(Box::new(ResidentProcessProviderLaunchFailure::Acceptance {
                    reservation,
                    failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
                }))
            }
            ResidentProcessProviderLaunchFailure::Acceptance {
                reservation,
                failure,
            } => match self.provider_routes.release_reservation(reservation) {
                Ok(()) => Ok(ResidentProcessProviderAbandonedLaunch::Acceptance(failure)),
                Err(reservation) => {
                    Err(Box::new(ResidentProcessProviderLaunchFailure::Acceptance {
                        reservation: *reservation,
                        failure,
                    }))
                }
            },
            failure => Err(Box::new(failure)),
        }
    }

    /// Repair the lower aggregate refusal, then release its host route slot.
    ///
    /// The embedded deployment selector, process owner, and caller-supplied
    /// access store are checked before the linear lower transition is consumed.
    /// This path has no executor, custody, anchor, retention, or signer access.
    fn continue_refused_provider_aggregate_transition<'operation, 'call>(
        &mut self,
        failure: ResidentProcessProviderLaunchFailure<'operation, 'call>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<
        ResidentProviderTaskRefusal<'operation>,
        Box<ResidentProcessProviderLaunchFailure<'operation, 'call>>,
    > {
        let ResidentProcessProviderLaunchFailure::Acceptance {
            reservation,
            failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
        } = failure
        else {
            return Err(Box::new(failure));
        };
        if !reservation
            .owner_instance
            .is_same(&self.provider_routes.owner_instance)
            || !is_unaccepted_provider_aggregate_transition(&refusal)
        {
            return Err(Box::new(ResidentProcessProviderLaunchFailure::Acceptance {
                reservation,
                failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
            }));
        }
        let selected = self.deployment_authority_index(&reservation.selector).ok();
        if selected.is_none()
            || selected.is_some_and(|index| self.deployments[index].store() != access.store)
        {
            return Err(Box::new(ResidentProcessProviderLaunchFailure::Acceptance {
                reservation,
                failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
            }));
        }
        match self
            .operations
            .continue_refused_provider_aggregate_transition(refusal)
        {
            Ok(refusal) => {
                self.provider_routes
                    .release_reservation(reservation)
                    .unwrap_or_else(|_| {
                        unreachable!("the route reservation was fenced before lower repair")
                    });
                Ok(refusal)
            }
            Err(refusal) => Err(Box::new(ResidentProcessProviderLaunchFailure::Acceptance {
                reservation,
                failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
            })),
        }
    }

    /// Retry only process-local acceptance of an unopened retained provider call.
    ///
    /// Durable retention, release preflight, signing, and checkpoint work are
    /// not repeated. A protocol-loss or aggregate-after-acceptance outcome
    /// captures the route and remains non-replayable.
    fn continue_refused_provider_acceptance<'operation, 'call, Authority, Executor>(
        &mut self,
        failure: ResidentProcessProviderLaunchFailure<'operation, 'call>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        executor: &mut Executor,
    ) -> Result<
        ResidentOperationTaskKey,
        Box<ResidentProcessProviderLaunchFailure<'operation, 'call>>,
    >
    where
        Executor: ResidentOperationTaskExecutor<
            OwnedProviderInvocationRequest,
            ProcessProviderTaskOutput,
        >,
    {
        let ResidentProcessProviderLaunchFailure::Acceptance {
            reservation,
            failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
        } = failure
        else {
            return Err(Box::new(failure));
        };
        if !reservation
            .owner_instance
            .is_same(&self.provider_routes.owner_instance)
        {
            return Err(Box::new(ResidentProcessProviderLaunchFailure::Acceptance {
                reservation,
                failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
            }));
        }
        let selector = reservation.selector.clone();
        let selected = match self.deployment_authority_index(&selector) {
            Ok(index) if self.deployments[index].store() == access.store => index,
            _ => {
                return Err(Box::new(ResidentProcessProviderLaunchFailure::Acceptance {
                    reservation,
                    failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
                }));
            }
        };
        debug_assert_eq!(self.deployments[selected].store(), access.store);
        let retried = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_provider_acceptance(refusal, executor)
                },
            )
            .expect("route and access were selected before provider acceptance retry");
        match retried {
            Ok(key) => {
                self.provider_routes
                    .commit(reservation, key.clone())
                    .unwrap_or_else(|_| unreachable!("route reservation came from this host"));
                Ok(key)
            }
            Err(refusal) => {
                if let Some(key) = accepted_provider_refusal_key(&refusal).cloned() {
                    let selector = reservation.selector.clone();
                    self.provider_routes
                        .commit(reservation, key.clone())
                        .unwrap_or_else(|_| unreachable!("route reservation came from this host"));
                    Err(Box::new(
                        ResidentProcessProviderLaunchFailure::AcceptedWithFailure {
                            selector,
                            key,
                            failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
                        },
                    ))
                } else {
                    Err(Box::new(ResidentProcessProviderLaunchFailure::Acceptance {
                        reservation,
                        failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
                    }))
                }
            }
        }
    }

    /// Repair only aggregate occupancy after executor acceptance already won.
    ///
    /// The embedded deployment route and caller-supplied access store are
    /// checked before the exact Offered capability is consumed. This path has
    /// no executor, custody, anchor, retention, or signer access.
    fn continue_accepted_provider_aggregate_transition<'operation, 'call>(
        &mut self,
        failure: ResidentProcessProviderLaunchFailure<'operation, 'call>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<
        ResidentOperationTaskKey,
        Box<ResidentProcessProviderLaunchFailure<'operation, 'call>>,
    > {
        let ResidentProcessProviderLaunchFailure::AcceptedWithFailure {
            selector,
            key,
            failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
        } = failure
        else {
            return Err(Box::new(failure));
        };
        let selected = self.deployment_authority_index(&selector).ok();
        if selected.is_none()
            || self.provider_routes.selector(&key) != Some(&selector)
            || selected.is_some_and(|index| self.deployments[index].store() != access.store)
            || accepted_provider_refusal_key(&refusal) != Some(&key)
        {
            return Err(Box::new(
                ResidentProcessProviderLaunchFailure::AcceptedWithFailure {
                    selector,
                    key,
                    failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
                },
            ));
        }
        match self
            .operations
            .continue_accepted_provider_aggregate_transition(refusal)
        {
            Ok(ResidentProviderAcceptedAggregateTransition::Accepted(accepted)) => {
                if accepted == key {
                    Ok(accepted)
                } else {
                    unreachable!("the aggregate continuation preserves the accepted key")
                }
            }
            Ok(ResidentProviderAcceptedAggregateTransition::AcceptedWithFailure {
                key: accepted,
                refusal,
            }) => {
                if accepted != key {
                    unreachable!("the aggregate continuation preserves the accepted key")
                }
                Err(Box::new(
                    ResidentProcessProviderLaunchFailure::AcceptedWithFailure {
                        selector,
                        key,
                        failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(*refusal),
                    },
                ))
            }
            Err(refusal) => Err(Box::new(
                ResidentProcessProviderLaunchFailure::AcceptedWithFailure {
                    selector,
                    key,
                    failure: ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal),
                },
            )),
        }
    }
}

impl<'host, 'store> ResidentProcessProviderRecoveryDriver<'host, 'store> {
    /// Transfer one exact recovered release to the existing process task owner.
    fn launch<Authority, Executor>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        release: RecoveredProviderDispatchRelease,
        invocation: &ProviderOperationInvokeInput<'_>,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, Box<ResidentProcessRecoveredProviderLaunchFailure>>
    where
        Authority: super::AmbientSessionAuthorityPort,
        Executor: ResidentOperationTaskExecutor<
            OwnedProviderInvocationRequest,
            ProcessProviderTaskOutput,
        >,
    {
        let observation = release.observation().1;
        if access.store != &self.store
            || release.store() != &self.store
            || !self.selector.matches_operation(release.operation())
            || !self.selector.matches_current_session(invocation.current)
            || observation.deployment_runtime_authority_ref()
                != &self.selector.runtime_authority_ref
            || observation.deployment_runtime_authority_sha256().as_str()
                != self.selector.runtime_authority_sha256.as_str()
            || observation.issuer_principal_id() != &self.selector.principal_id
        {
            return Err(Box::new(
                ResidentProcessRecoveredProviderLaunchFailure::Selection {
                    release: Box::new(release),
                    reason: ResidentProcessHostSelectionError::ProviderOperationAuthority,
                },
            ));
        }
        let reservation = match self.routes.reserve(self.selector.clone()) {
            Ok(reservation) => reservation,
            Err(()) => {
                return Err(Box::new(
                    ResidentProcessRecoveredProviderLaunchFailure::RouteSaturated(Box::new(
                        release,
                    )),
                ));
            }
        };
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let accepted =
            self.operations
                .with_operation_service(authority, custody, anchor, |service| {
                    service.accept_recovered_provider(release, invocation, executor)
                });
        match accepted {
            Ok(key) => {
                self.routes
                    .commit(reservation, key.clone())
                    .unwrap_or_else(|_| unreachable!("route reservation came from this driver"));
                Ok(key)
            }
            Err(failure) => {
                if let Some(key) = accepted_recovered_provider_key(&failure).cloned() {
                    let selector = reservation.selector.clone();
                    self.routes
                        .commit(reservation, key.clone())
                        .unwrap_or_else(|_| {
                            unreachable!("route reservation came from this driver")
                        });
                    Err(Box::new(
                        ResidentProcessRecoveredProviderLaunchFailure::AcceptedWithFailure {
                            selector,
                            key,
                            failure,
                        },
                    ))
                } else {
                    Err(Box::new(
                        ResidentProcessRecoveredProviderLaunchFailure::Acceptance {
                            reservation,
                            failure,
                        },
                    ))
                }
            }
        }
    }

    /// Poll one provider task without leaving this split process owner.
    fn poll<Executor>(
        &mut self,
        executor: &mut Executor,
    ) -> Option<ResidentProcessProviderCompletionPoll>
    where
        Executor: ResidentOperationTaskExecutor<
            OwnedProviderInvocationRequest,
            ProcessProviderTaskOutput,
        >,
    {
        let completion = self.operations.poll_provider_task(executor)?;
        match &completion {
            ResidentProviderCompletionPoll::Stale(_) => {
                return Some(ResidentProcessProviderCompletionPoll::Stale(Box::new(
                    completion,
                )));
            }
            ResidentProviderCompletionPoll::AggregateTransitionRejected {
                completion: rejected,
                ..
            } => {
                let Some(selector) = self.routes.selector(rejected.key()).cloned() else {
                    return Some(ResidentProcessProviderCompletionPoll::Unrouted(Box::new(
                        completion,
                    )));
                };
                return Some(ResidentProcessProviderCompletionPoll::Routed(Box::new(
                    ResidentProcessProviderCompletion {
                        route: ResidentProcessProviderCompletionRoute::Active { selector },
                        completion,
                    },
                )));
            }
            ResidentProviderCompletionPoll::Completed(_)
            | ResidentProviderCompletionPoll::Lost(_) => {}
        }
        let key = match &completion {
            ResidentProviderCompletionPoll::Completed(completed) => completed.key(),
            ResidentProviderCompletionPoll::Lost(lost) => lost.key(),
            ResidentProviderCompletionPoll::Stale(_)
            | ResidentProviderCompletionPoll::AggregateTransitionRejected { .. } => {
                unreachable!("nonterminal branches returned above")
            }
        };
        let Some(custody) = self.routes.check_out(key) else {
            return Some(ResidentProcessProviderCompletionPoll::Unrouted(Box::new(
                completion,
            )));
        };
        Some(ResidentProcessProviderCompletionPoll::Routed(Box::new(
            ResidentProcessProviderCompletion {
                route: ResidentProcessProviderCompletionRoute::CheckedOut(custody),
                completion,
            },
        )))
    }

    /// Recover exact dispatch history before recording an accepted completion.
    #[allow(clippy::too_many_arguments)]
    fn record_completed<
        'operation,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        progress: ResidentProcessProviderCompletion,
        attempt: &'operation ResidentProcessRecoveredAttempt,
        maximum_selection_bytes: usize,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        admission: ProviderInvocationAdmissionContext<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
        evidence: &dyn ProviderInvocationEvidenceAdmission,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessProviderRecording<'static>,
        Box<ResidentProcessProviderCompletionRecordingFailure<'static>>,
    >
    where
        'host: 'operation,
        'store: 'operation,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Reconciliation: FnMut(
            &pale_core::records::ReconciliationFrontierReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let ResidentProcessProviderCompletion { route, completion } = progress;
        let (route, completed) = match (route, completion) {
            (
                ResidentProcessProviderCompletionRoute::CheckedOut(route),
                ResidentProviderCompletionPoll::Completed(completed),
            ) => (route, *completed),
            (route, completion) => {
                return Err(Box::new(
                    ResidentProcessProviderCompletionRecordingFailure::Phase(
                        ResidentProcessProviderCompletion { route, completion },
                    ),
                ));
            }
        };
        let history = match self.recovered_history(attempt, maximum_selection_bytes) {
            Ok(history) => history,
            Err(reason) => {
                return Err(Box::new(
                    ResidentProcessProviderCompletionRecordingFailure::Selection {
                        route,
                        completed: Box::new(completed),
                        reason,
                    },
                ));
            }
        };
        let recovered = attempt.start.control.observation();
        if let Err(reason) = self.validate_recovered_recording_route(
            &route,
            access.store,
            &history,
            recovered,
            input,
        ) {
            return Err(Box::new(
                ResidentProcessProviderCompletionRecordingFailure::Selection {
                    route,
                    completed: Box::new(completed),
                    reason,
                },
            ));
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let result =
            self.operations
                .with_operation_service(authority, custody, anchor, |service| {
                    service.continue_provider_completion_recovery(
                        completed,
                        history,
                        recovered,
                        admission,
                        input,
                        evidence,
                        admit_reconciliation,
                        admit_rotation,
                        sign_observation,
                        sign_checkpoint,
                    )
                });
        match result {
            Ok(progress) => Ok(self.finish_progress(route, progress)),
            Err(rejection) => Err(Box::new(
                ResidentProcessProviderCompletionRecordingFailure::Aggregate { route, rejection },
            )),
        }
    }

    /// Recover exact dispatch history before recording an executor-lost task.
    #[allow(clippy::too_many_arguments)]
    fn record_lost<
        'operation,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        progress: ResidentProcessProviderCompletion,
        attempt: &'operation ResidentProcessRecoveredAttempt,
        maximum_selection_bytes: usize,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: &ProviderOperationInvokeInput<'_>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessProviderRecording<'static>,
        Box<ResidentProcessProviderLossRecordingFailure>,
    >
    where
        'host: 'operation,
        'store: 'operation,
        Authority: pale_ports::AmbientSessionAuthorityPort,
        Reconciliation: FnOnce(
            &pale_core::records::ReconciliationFrontierReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessProviderCompletion { route, completion } = progress;
        let (route, lost) = match (route, completion) {
            (
                ResidentProcessProviderCompletionRoute::CheckedOut(route),
                ResidentProviderCompletionPoll::Lost(lost),
            ) => (route, *lost),
            (route, completion) => {
                return Err(Box::new(
                    ResidentProcessProviderLossRecordingFailure::Phase(
                        ResidentProcessProviderCompletion { route, completion },
                    ),
                ));
            }
        };
        let history = match self.recovered_history(attempt, maximum_selection_bytes) {
            Ok(history) => history,
            Err(reason) => {
                return Err(Box::new(
                    ResidentProcessProviderLossRecordingFailure::Selection {
                        route,
                        lost: Box::new(lost),
                        reason,
                    },
                ));
            }
        };
        let recovered = attempt.start.control.observation();
        if let Err(reason) = self.validate_recovered_recording_route(
            &route,
            access.store,
            &history,
            recovered,
            input,
        ) {
            return Err(Box::new(
                ResidentProcessProviderLossRecordingFailure::Selection {
                    route,
                    lost: Box::new(lost),
                    reason,
                },
            ));
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let result =
            self.operations
                .with_operation_service(authority, custody, anchor, |service| {
                    service.recover_lost_provider(
                        lost,
                        history,
                        recovered,
                        input,
                        admit_reconciliation,
                        admit_rotation,
                        sign_observation,
                        sign_checkpoint,
                    )
                });
        match result {
            Ok(progress) => Ok(self.finish_progress(route, progress)),
            Err(failure) => Err(Box::new(
                ResidentProcessProviderLossRecordingFailure::Recovery { route, failure },
            )),
        }
    }

    fn recovered_history<'operation>(
        &self,
        attempt: &'operation ResidentProcessRecoveredAttempt,
        maximum_selection_bytes: usize,
    ) -> Result<OperationAttemptHistoryInput<'operation>, ResidentProcessHostSelectionError>
    where
        'host: 'operation,
        'store: 'operation,
    {
        let generation = self
            .application
            .generation(&attempt.start.generation_digest)
            .ok_or(ResidentProcessHostSelectionError::HistoricalProviderAuthority)?;
        let operation = attempt.start.control.operation();
        Ok(OperationAttemptHistoryInput {
            operation: operation.operation(),
            request: operation.request(),
            session: operation.operation().historical_session_lease(),
            lease: operation.operation().historical_operation_lease(),
            generation,
            dispatched: attempt.start.control.observation().selected().1,
            selection: &attempt.selection,
            maximum_selection_bytes,
        })
    }

    fn validate_recovered_recording_route(
        &self,
        route: &ResidentProcessProviderRouteCustody,
        access_store: &AuthorityStoreCoordinate,
        history: &OperationAttemptHistoryInput<'_>,
        recovered: &RecoveredOperationObservation,
        input: &ProviderOperationInvokeInput<'_>,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        if !route.owner_instance.is_same(&self.routes.owner_instance)
            || route.selector != self.selector
            || access_store != &self.store
            || route.key.store() != &self.store
            || recovered.store() != &self.store
        {
            return Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation);
        }
        let dispatch_digest = ArtifactDigest::parse(
            history
                .dispatched
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .map_err(|_| ResidentProcessHostSelectionError::ProviderTaskAssociation)?;
        if route.key.dispatch().record_ref != *recovered.selected().0
            || route.key.dispatch().record_sha256 != dispatch_digest
            || recovered.selected().1 != history.dispatched
            || !self.selector.matches_operation(history.operation)
            || !self.selector.matches_current_session(input.current)
            || !self
                .selector
                .matches_signing_authority(input.signing.signing_authority)
            || !checkpoint_authority_matches(
                &self.selector,
                &self.store,
                input.checkpoint_authority,
            )
        {
            return Err(ResidentProcessHostSelectionError::ProviderTaskAssociation);
        }
        Ok(())
    }

    fn finish_progress<'operation>(
        &mut self,
        route: ResidentProcessProviderRouteCustody,
        progress: ResidentProviderTaskPoll<'operation>,
    ) -> ResidentProcessProviderRecording<'operation> {
        match progress {
            ResidentProviderTaskPoll::Recorded {
                result,
                cancellation,
            } => match self.routes.release_custody(route) {
                Ok(()) => ResidentProcessProviderRecording::Recorded {
                    result,
                    cancellation,
                },
                Err(route) => ResidentProcessProviderRecording::RouteReleaseRejected {
                    route: *route,
                    result,
                    cancellation,
                },
            },
            progress => ResidentProcessProviderRecording::Pending { route, progress },
        }
    }
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Record one returned provider completion through its exact deployment.
    ///
    /// Every derivable deployment, owner, store, historical session, handle,
    /// observation, and signer association is checked before custody is lent.
    /// Nonterminal outcomes retain the checked-out route; only a fully recorded
    /// result releases process-host capacity.
    #[allow(clippy::too_many_arguments)]
    fn record_completed_provider<
        'operation,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        progress: ResidentProcessProviderCompletion,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        history: OperationAttemptHistoryInput<'operation>,
        recovered: &'operation RecoveredOperationObservation,
        admission: ProviderInvocationAdmissionContext<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
        evidence: &dyn ProviderInvocationEvidenceAdmission,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessProviderRecording<'static>,
        Box<ResidentProcessProviderCompletionRecordingFailure<'static>>,
    >
    where
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Reconciliation: FnMut(
            &pale_core::records::ReconciliationFrontierReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let ResidentProcessProviderCompletion { route, completion } = progress;
        let (route, completed) = match (route, completion) {
            (
                ResidentProcessProviderCompletionRoute::CheckedOut(route),
                ResidentProviderCompletionPoll::Completed(completed),
            ) => (route, *completed),
            (route, completion) => {
                return Err(Box::new(
                    ResidentProcessProviderCompletionRecordingFailure::Phase(
                        ResidentProcessProviderCompletion { route, completion },
                    ),
                ));
            }
        };
        if let Err(reason) =
            self.validate_provider_recording_route(&route, access.store, &history, recovered, input)
        {
            return Err(Box::new(
                ResidentProcessProviderCompletionRecordingFailure::Selection {
                    route,
                    completed: Box::new(completed),
                    reason,
                },
            ));
        }
        let selector = route.selector.clone();
        let result = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_provider_completion_recovery(
                        completed,
                        history,
                        recovered,
                        admission,
                        input,
                        evidence,
                        admit_reconciliation,
                        admit_rotation,
                        sign_observation,
                        sign_checkpoint,
                    )
                },
            )
            .expect("route and access were selected before provider recording");
        match result {
            Ok(progress) => Ok(self.finish_provider_progress(route, progress)),
            Err(rejection) => Err(Box::new(
                ResidentProcessProviderCompletionRecordingFailure::Aggregate { route, rejection },
            )),
        }
    }

    /// Record one executor-lost provider task through its exact deployment.
    #[allow(clippy::too_many_arguments)]
    fn record_lost_provider<
        'operation,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        progress: ResidentProcessProviderCompletion,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        history: OperationAttemptHistoryInput<'operation>,
        recovered: &'operation RecoveredOperationObservation,
        input: &ProviderOperationInvokeInput<'_>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessProviderRecording<'static>,
        Box<ResidentProcessProviderLossRecordingFailure>,
    >
    where
        Authority: pale_ports::AmbientSessionAuthorityPort,
        Reconciliation: FnOnce(
            &pale_core::records::ReconciliationFrontierReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessProviderCompletion { route, completion } = progress;
        let (route, lost) = match (route, completion) {
            (
                ResidentProcessProviderCompletionRoute::CheckedOut(route),
                ResidentProviderCompletionPoll::Lost(lost),
            ) => (route, *lost),
            (route, completion) => {
                return Err(Box::new(
                    ResidentProcessProviderLossRecordingFailure::Phase(
                        ResidentProcessProviderCompletion { route, completion },
                    ),
                ));
            }
        };
        if let Err(reason) =
            self.validate_provider_recording_route(&route, access.store, &history, recovered, input)
        {
            return Err(Box::new(
                ResidentProcessProviderLossRecordingFailure::Selection {
                    route,
                    lost: Box::new(lost),
                    reason,
                },
            ));
        }
        let selector = route.selector.clone();
        let result = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.recover_lost_provider(
                        lost,
                        history,
                        recovered,
                        input,
                        admit_reconciliation,
                        admit_rotation,
                        sign_observation,
                        sign_checkpoint,
                    )
                },
            )
            .expect("route and access were selected before provider loss recovery");
        match result {
            Ok(progress) => Ok(self.finish_provider_progress(route, progress)),
            Err(failure) => Err(Box::new(
                ResidentProcessProviderLossRecordingFailure::Recovery { route, failure },
            )),
        }
    }

    fn validate_provider_recording_route(
        &self,
        route: &ResidentProcessProviderRouteCustody,
        access_store: &AuthorityStoreCoordinate,
        history: &OperationAttemptHistoryInput<'_>,
        recovered: &RecoveredOperationObservation,
        input: &ProviderOperationInvokeInput<'_>,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        if !route
            .owner_instance
            .is_same(&self.provider_routes.owner_instance)
        {
            return Err(ResidentProcessHostSelectionError::ProviderRouteOwner);
        }
        let index = self.deployment_authority_index(&route.selector)?;
        let store = self.deployments[index].store();
        if store != access_store || route.key.store() != store || recovered.store() != store {
            return Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation);
        }
        let Ok(history_digest) = ArtifactDigest::parse(
            history
                .dispatched
                .canonical_operation_observation_sha256()
                .as_str(),
        ) else {
            return Err(ResidentProcessHostSelectionError::ProviderTaskAssociation);
        };
        if route.key.dispatch().record_ref != *recovered.selected().0
            || route.key.dispatch().record_sha256 != history_digest
            || recovered.selected().1 != history.dispatched
            || history.session != history.operation.historical_session_lease()
            || history.lease != history.operation.historical_operation_lease()
            || !history
                .operation
                .checkpoint_selection()
                .matches_store(store)
        {
            return Err(ResidentProcessHostSelectionError::ProviderTaskAssociation);
        }
        if !route.selector.matches_operation(history.operation)
            || !route.selector.matches_current_session(input.current)
        {
            return Err(ResidentProcessHostSelectionError::ProviderCurrentAuthority);
        }
        if history.session.deployment_runtime_authority_ref()
            != &route.selector.runtime_authority_ref
            || history
                .session
                .deployment_runtime_authority_sha256()
                .as_str()
                != route.selector.runtime_authority_sha256.as_str()
            || history.session.issuer_principal_id() != &route.selector.principal_id
            || history.dispatched.deployment_runtime_authority_ref()
                != &route.selector.runtime_authority_ref
            || history
                .dispatched
                .deployment_runtime_authority_sha256()
                .as_str()
                != route.selector.runtime_authority_sha256.as_str()
            || history.dispatched.issuer_principal_id() != &route.selector.principal_id
        {
            return Err(ResidentProcessHostSelectionError::HistoricalProviderAuthority);
        }
        if !route
            .selector
            .matches_signing_authority(input.signing.signing_authority)
        {
            return Err(ResidentProcessHostSelectionError::ProviderObservationAuthority);
        }
        if !checkpoint_authority_matches(&route.selector, store, input.checkpoint_authority) {
            return Err(ResidentProcessHostSelectionError::ProviderCheckpointAuthority);
        }
        Ok(())
    }

    fn finish_provider_progress<'operation>(
        &mut self,
        route: ResidentProcessProviderRouteCustody,
        progress: ResidentProviderTaskPoll<'operation>,
    ) -> ResidentProcessProviderRecording<'operation> {
        match progress {
            ResidentProviderTaskPoll::Recorded {
                result,
                cancellation,
            } => match self.provider_routes.release_custody(route) {
                Ok(()) => ResidentProcessProviderRecording::Recorded {
                    result,
                    cancellation,
                },
                Err(route) => ResidentProcessProviderRecording::RouteReleaseRejected {
                    route: *route,
                    result,
                    cancellation,
                },
            },
            progress => ResidentProcessProviderRecording::Pending { route, progress },
        }
    }

    /// Retry only aggregate release after durable provider recording completed.
    ///
    /// Selection and access are rechecked against the embedded route. No durable
    /// record, signature, anchor update, or provider call is repeated.
    fn continue_provider_aggregate_release<Authority>(
        &mut self,
        recording: ResidentProcessProviderRecording<'static>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> Result<
        ResidentProcessProviderRecording<'static>,
        Box<ResidentProcessProviderRecordingContinuationFailure<'static>>,
    > {
        let (route, result, cancellation, failure) = match recording {
            ResidentProcessProviderRecording::Pending {
                route,
                progress:
                    ResidentProviderTaskPoll::AggregateReleaseRejected {
                        result,
                        cancellation,
                        failure,
                    },
            } => (route, *result, cancellation, failure),
            recording => {
                return Err(Box::new(
                    ResidentProcessProviderRecordingContinuationFailure::Phase(recording),
                ));
            }
        };
        let selector = route.selector.clone();
        if let Err(reason) = self.validate_provider_route_access(&route, access.store) {
            return Err(Box::new(
                ResidentProcessProviderRecordingContinuationFailure::Selection {
                    recording: ResidentProcessProviderRecording::Pending {
                        route,
                        progress: ResidentProviderTaskPoll::AggregateReleaseRejected {
                            result: Box::new(result),
                            cancellation,
                            failure,
                        },
                    },
                    reason,
                },
            ));
        }
        let progress = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_provider_aggregate_release(result, cancellation, failure)
                },
            )
            .expect("route and access were selected before aggregate release retry");
        Ok(self.finish_provider_progress(route, progress))
    }

    /// Retry only process-host route release after durable settlement.
    ///
    /// The durable result is already final. This continuation consumes the
    /// exact route capability and performs no deployment selection, port I/O,
    /// signature, anchor update, aggregate transition, or provider call.
    fn continue_provider_route_release(
        &mut self,
        recording: ResidentProcessProviderRecording<'static>,
    ) -> Result<
        ResidentProcessProviderRecording<'static>,
        Box<ResidentProcessProviderRecording<'static>>,
    > {
        let ResidentProcessProviderRecording::RouteReleaseRejected {
            route,
            result,
            cancellation,
        } = recording
        else {
            return Err(Box::new(recording));
        };
        match self.provider_routes.release_custody(route) {
            Ok(()) => Ok(ResidentProcessProviderRecording::Recorded {
                result,
                cancellation,
            }),
            Err(route) => Err(Box::new(
                ResidentProcessProviderRecording::RouteReleaseRejected {
                    route: *route,
                    result,
                    cancellation,
                },
            )),
        }
    }

    /// Retry protected history recovery for the same accepted completion.
    #[allow(clippy::too_many_arguments)]
    fn continue_provider_completion_recovery<
        'operation,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        recording: ResidentProcessProviderRecording<'static>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        history: OperationAttemptHistoryInput<'operation>,
        recovered: &'operation RecoveredOperationObservation,
        admission: ProviderInvocationAdmissionContext<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
        evidence: &dyn ProviderInvocationEvidenceAdmission,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessProviderRecording<'static>,
        Box<ResidentProcessProviderCompletionRecordingFailure<'static>>,
    >
    where
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Reconciliation: FnMut(
            &pale_core::records::ReconciliationFrontierReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let progress = match recording {
            ResidentProcessProviderRecording::Pending {
                route,
                progress: ResidentProviderTaskPoll::RecoveryRejected { completed, .. },
            } => ResidentProcessProviderCompletion {
                route: ResidentProcessProviderCompletionRoute::CheckedOut(route),
                completion: ResidentProviderCompletionPoll::Completed(completed),
            },
            recording => {
                return Err(Box::new(
                    ResidentProcessProviderCompletionRecordingFailure::RecordedPhase(recording),
                ));
            }
        };
        self.record_completed_provider(
            progress,
            access,
            authority,
            history,
            recovered,
            admission,
            input,
            evidence,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Continue one consumed result after a preparation-only recording failure.
    #[allow(clippy::too_many_arguments)]
    fn continue_provider_recording<
        'operation,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        recording: ResidentProcessProviderRecording<'operation>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: &ProviderOperationInvokeInput<'_>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessProviderRecording<'operation>,
        Box<ResidentProcessProviderRecordingContinuationFailure<'operation>>,
    >
    where
        Reconciliation: FnOnce(
            &pale_core::records::ReconciliationFrontierReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let (route, continuation, reason) = match recording {
            ResidentProcessProviderRecording::Pending {
                route,
                progress:
                    ResidentProviderTaskPoll::Preparation {
                        continuation,
                        reason,
                    },
            } => (route, *continuation, reason),
            recording => {
                return Err(Box::new(
                    ResidentProcessProviderRecordingContinuationFailure::Phase(recording),
                ));
            }
        };
        let selector = route.selector.clone();
        let selection = self
            .validate_provider_route_access(&route, access.store)
            .and_then(|()| {
                if !selector.matches_current_session(input.current)
                    || !selector.matches_signing_authority(input.signing.signing_authority)
                    || !checkpoint_authority_matches(
                        &selector,
                        access.store,
                        input.checkpoint_authority,
                    )
                {
                    Err(ResidentProcessHostSelectionError::ProviderCurrentAuthority)
                } else {
                    Ok(())
                }
            });
        if let Err(selection) = selection {
            return Err(Box::new(
                ResidentProcessProviderRecordingContinuationFailure::Selection {
                    recording: ResidentProcessProviderRecording::Pending {
                        route,
                        progress: ResidentProviderTaskPoll::Preparation {
                            continuation: Box::new(continuation),
                            reason,
                        },
                    },
                    reason: selection,
                },
            ));
        }
        let result = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_provider_recording(
                        continuation,
                        input,
                        admit_reconciliation,
                        admit_rotation,
                        sign_observation,
                        sign_checkpoint,
                    )
                },
            )
            .expect("route and access were selected before provider recording retry");
        match result {
            Ok(progress) => Ok(self.finish_provider_progress(route, progress)),
            Err(rejection) => Err(Box::new(
                ResidentProcessProviderRecordingContinuationFailure::Preparation {
                    route,
                    rejection,
                },
            )),
        }
    }

    /// Rejoin consumed provider evidence after a concurrent durable head.
    #[allow(clippy::too_many_arguments)]
    fn continue_provider_after_concurrent_head<
        'operation,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        recording: ResidentProcessProviderRecording<'operation>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        recovered: &RecoveredOperationObservation,
        input: &ProviderOperationInvokeInput<'_>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessProviderRecording<'operation>,
        Box<ResidentProcessProviderRecordingContinuationFailure<'operation>>,
    >
    where
        Reconciliation: FnOnce(
            &pale_core::records::ReconciliationFrontierReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let (route, conflict) = match recording {
            ResidentProcessProviderRecording::Pending {
                route,
                progress: ResidentProviderTaskPoll::ConcurrentHead(conflict),
            } => (route, *conflict),
            recording => {
                return Err(Box::new(
                    ResidentProcessProviderRecordingContinuationFailure::Phase(recording),
                ));
            }
        };
        let selector = route.selector.clone();
        let selection = self
            .validate_provider_route_access(&route, access.store)
            .and_then(|()| {
                if recovered.store() != access.store {
                    Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation)
                } else if !selector.matches_current_session(input.current)
                    || !selector.matches_signing_authority(input.signing.signing_authority)
                    || !checkpoint_authority_matches(
                        &selector,
                        access.store,
                        input.checkpoint_authority,
                    )
                {
                    Err(ResidentProcessHostSelectionError::ProviderCurrentAuthority)
                } else {
                    Ok(())
                }
            });
        if let Err(selection) = selection {
            return Err(Box::new(
                ResidentProcessProviderRecordingContinuationFailure::Selection {
                    recording: ResidentProcessProviderRecording::Pending {
                        route,
                        progress: ResidentProviderTaskPoll::ConcurrentHead(Box::new(conflict)),
                    },
                    reason: selection,
                },
            ));
        }
        let result = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_provider_after_concurrent_head(
                        conflict,
                        recovered,
                        input,
                        admit_reconciliation,
                        admit_rotation,
                        sign_observation,
                        sign_checkpoint,
                    )
                },
            )
            .expect("route and access were selected before provider head recovery");
        match result {
            Ok(progress) => Ok(self.finish_provider_progress(route, progress)),
            Err(rejection) => Err(Box::new(
                ResidentProcessProviderRecordingContinuationFailure::ConcurrentHead {
                    route,
                    rejection,
                },
            )),
        }
    }

    /// Continue only an already-created provider recording custody candidate.
    fn continue_provider_recording_custody<'operation, Authority>(
        &mut self,
        recording: ResidentProcessProviderRecording<'operation>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> Result<
        ResidentProcessProviderRecording<'operation>,
        Box<ResidentProcessProviderRecordingContinuationFailure<'operation>>,
    > {
        let (route, custody) = match recording {
            ResidentProcessProviderRecording::Pending {
                route,
                progress: ResidentProviderTaskPoll::Custody(custody),
            } => (route, *custody),
            recording => {
                return Err(Box::new(
                    ResidentProcessProviderRecordingContinuationFailure::Phase(recording),
                ));
            }
        };
        let selector = route.selector.clone();
        if let Err(reason) = self.validate_provider_route_access(&route, access.store) {
            return Err(Box::new(
                ResidentProcessProviderRecordingContinuationFailure::Selection {
                    recording: ResidentProcessProviderRecording::Pending {
                        route,
                        progress: ResidentProviderTaskPoll::Custody(Box::new(custody)),
                    },
                    reason,
                },
            ));
        }
        let result = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_provider_recording_custody(custody)
                },
            )
            .expect("route and access were selected before provider custody retry");
        match result {
            Ok(progress) => Ok(self.finish_provider_progress(route, progress)),
            Err(rejection) => Err(Box::new(
                ResidentProcessProviderRecordingContinuationFailure::Custody { route, rejection },
            )),
        }
    }

    /// Retry only semantic admission of already-completed provider custody.
    fn finish_provider_recording<'operation, Authority>(
        &mut self,
        recording: ResidentProcessProviderRecording<'operation>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> Result<
        ResidentProcessProviderRecording<'operation>,
        Box<ResidentProcessProviderRecordingContinuationFailure<'operation>>,
    > {
        let (route, finalization, reason) = match recording {
            ResidentProcessProviderRecording::Pending {
                route,
                progress:
                    ResidentProviderTaskPoll::Finalization {
                        finalization,
                        reason,
                    },
            } => (route, *finalization, reason),
            recording => {
                return Err(Box::new(
                    ResidentProcessProviderRecordingContinuationFailure::Phase(recording),
                ));
            }
        };
        let selector = route.selector.clone();
        if let Err(selection) = self.validate_provider_route_access(&route, access.store) {
            return Err(Box::new(
                ResidentProcessProviderRecordingContinuationFailure::Selection {
                    recording: ResidentProcessProviderRecording::Pending {
                        route,
                        progress: ResidentProviderTaskPoll::Finalization {
                            finalization: Box::new(finalization),
                            reason,
                        },
                    },
                    reason: selection,
                },
            ));
        }
        let result = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| service.finish_provider_recording(finalization),
            )
            .expect("route and access were selected before provider finalization retry");
        match result {
            Ok(progress) => Ok(self.finish_provider_progress(route, progress)),
            Err(rejection) => Err(Box::new(
                ResidentProcessProviderRecordingContinuationFailure::Finalization {
                    route,
                    rejection,
                },
            )),
        }
    }

    fn validate_provider_route_access(
        &self,
        route: &ResidentProcessProviderRouteCustody,
        access_store: &AuthorityStoreCoordinate,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        if !route
            .owner_instance
            .is_same(&self.provider_routes.owner_instance)
        {
            return Err(ResidentProcessHostSelectionError::ProviderRouteOwner);
        }
        let index = self.deployment_authority_index(&route.selector)?;
        let store = self.deployments[index].store();
        if store != access_store || route.key.store() != store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        Ok(())
    }

    /// Commit one already-admitted initial Consult collection.
    ///
    /// The Consult parent and every fresh child must already be durable. Exact
    /// deployment, store, signer, and parent associations are checked before
    /// custody ports are lent. This method neither plans nor releases work.
    #[allow(clippy::too_many_arguments)]
    fn open_consultation_collection<
        Authority,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentConsultationOpenInput<'_>,
        resolve_store: &dyn super::OperationObservationAuthorityStoreResolver,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<CompletedConsultationCollection, ResidentProcessConsultationOpenError>
    where
        AdmitCollection: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.validate_consultation_open_associations(selector, access.store, &input)?;
        let selected = selector.clone();
        let result = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.open_consultation_collection(
                        input,
                        resolve_store,
                        admit_collection,
                        admit_rotation,
                        sign_observation,
                        sign_checkpoint,
                    )
                },
            )
            .expect("Consult collection deployment was selected before lending ports");
        match result {
            Ok(completed) => Ok(completed),
            Err(ResidentConsultationOpenFailure::Admission(reason)) => {
                Err(ResidentProcessConsultationOpenError::Admission(reason))
            }
            Err(ResidentConsultationOpenFailure::Custody(failure)) => {
                Err(ResidentProcessConsultationOpenError::Custody(Box::new(
                    ResidentProcessConsultationOpenCustody {
                        selector: selected,
                        failure,
                    },
                )))
            }
        }
    }

    /// Continue one retained initial-collection custody phase.
    fn continue_consultation_collection_custody<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: Box<ResidentProcessConsultationOpenCustody>,
    ) -> Result<CompletedConsultationCollection, ResidentProcessConsultationOpenContinuationFailure>
    {
        let index = match self.deployment_authority_index(&pending.selector) {
            Ok(index) => index,
            Err(reason) => {
                return Err(ResidentProcessConsultationOpenContinuationFailure {
                    pending,
                    reason: ResidentProcessConsultationOpenContinuationError::Selection(reason),
                });
            }
        };
        if self.deployments[index].store() != access.store {
            return Err(ResidentProcessConsultationOpenContinuationFailure {
                pending,
                reason: ResidentProcessConsultationOpenContinuationError::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation,
                ),
            });
        }
        let ResidentProcessConsultationOpenCustody { selector, failure } = *pending;
        let result = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.continue_consultation_collection_custody(failure)
                },
            )
            .expect("Consult collection deployment was selected before custody retry");
        result.map_err(
            |failure| ResidentProcessConsultationOpenContinuationFailure {
                pending: Box::new(ResidentProcessConsultationOpenCustody { selector, failure }),
                reason: ResidentProcessConsultationOpenContinuationError::Custody,
            },
        )
    }

    fn validate_consultation_open_associations(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &AuthorityStoreCoordinate,
        input: &ResidentConsultationOpenInput<'_>,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(selector)?;
        let selected_store = self.deployments[index].store();
        if selected_store != access_store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        let (parent_ref, parent_handle) = input.parent.handle();
        let parent = input.collection.parent();
        if !selector.matches_operation(input.parent)
            || parent_handle.operation_class() != &OperationClassV1::Consult
            || parent.record_ref != *parent_ref
            || parent.record_sha256.as_str()
                != parent_handle.canonical_operation_handle_sha256().as_str()
        {
            return Err(ResidentProcessHostSelectionError::ConsultationParentAuthority);
        }
        let parent_observation = input.parent_observation.1;
        if parent_observation.operation_handle_ref() != parent_ref
            || parent_observation.operation_handle_sha256().as_str()
                != parent_handle.canonical_operation_handle_sha256().as_str()
            || parent_observation.deployment_runtime_authority_ref()
                != &selector.runtime_authority_ref
            || parent_observation
                .deployment_runtime_authority_sha256()
                .as_str()
                != selector.runtime_authority_sha256.as_str()
            || parent_observation.issuer_principal_id() != &selector.principal_id
        {
            return Err(ResidentProcessHostSelectionError::ConsultationOperationAuthority);
        }
        if !selector.matches_signing_authority(input.observation.signing.signing_authority) {
            return Err(ResidentProcessHostSelectionError::ConsultationSigningAuthority);
        }
        if !checkpoint_authority_matches(selector, selected_store, input.checkpoint.authority) {
            return Err(ResidentProcessHostSelectionError::ConsultationCheckpointAuthority);
        }
        Ok(())
    }

    /// Recover one current Consult collection through its selected deployment.
    ///
    /// The protected observation and operation must name the same selected
    /// deployment and store before custody ports are lent. Recovery selects no
    /// roster, child, stopping rule, provider, synthesis, or transport.
    #[allow(clippy::too_many_arguments)]
    fn recover_consultation_collection<Authority, AdmitCollection>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        operation: &super::IssuedLocalOperation,
        protected: &RecoveredOperationObservation,
        limits: ConsultationCollectionLimits,
        resolve_store: &dyn super::OperationObservationAuthorityStoreResolver,
        admit_collection: AdmitCollection,
    ) -> Result<
        Option<RecoveredConsultationCollection>,
        ResidentProcessConsultationCollectionRecoveryError,
    >
    where
        AdmitCollection: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
    {
        let index = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessConsultationCollectionRecoveryError::Selection)?;
        let selected_store = self.deployments[index].store();
        if selected_store != access.store
            || protected.store() != selected_store
            || !selector.matches_operation(operation)
        {
            return Err(
                ResidentProcessConsultationCollectionRecoveryError::Selection(
                    ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation,
                ),
            );
        }
        self.with_selected_operation_service(
            selector,
            access,
            authority,
            |_application, service, _sessions| {
                service.recover_consultation_collection(
                    operation,
                    protected,
                    limits,
                    resolve_store,
                    admit_collection,
                )
            },
        )
        .map_err(ResidentProcessConsultationCollectionRecoveryError::Selection)?
        .map_err(ResidentProcessConsultationCollectionRecoveryError::Recovery)
    }

    /// Open the finite first batch from one durably recovered collection.
    ///
    /// Recovery must happen after collection custody. Planning preserves roster
    /// order, requiredness, recipient context, provider binding, and deadline.
    /// This method retains the batch but does not dispatch a child.
    fn open_initial_consultation_batch(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &AuthorityStoreCoordinate,
        parent: &super::IssuedLocalOperation,
        current: &RecoveredConsultationCollection,
        now: UnixTimeV1,
        task_limits: ResidentConsultationTaskLimits,
    ) -> Result<ResidentConsultationBatchCoordinate, ResidentProcessConsultationBatchOpenError>
    {
        let index = self.deployment_authority_index(selector)?;
        let selected_store = self.deployments[index].store();
        let (parent_ref, parent_handle) = parent.handle();
        let collection_parent = current.collection().parent();
        if selected_store != access_store || current.store() != selected_store {
            return Err(ResidentProcessConsultationBatchOpenError::Selection(
                ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation,
            ));
        }
        if !selector.matches_operation(parent)
            || parent_handle.operation_class() != &OperationClassV1::Consult
            || collection_parent.record_ref != *parent_ref
            || collection_parent.record_sha256.as_str()
                != parent_handle.canonical_operation_handle_sha256().as_str()
        {
            return Err(ResidentProcessConsultationBatchOpenError::Selection(
                ResidentProcessHostSelectionError::ConsultationParentAuthority,
            ));
        }
        let planned = current
            .plan_child_dispatch(now)
            .map_err(ResidentProcessConsultationBatchOpenError::Planning)?;
        self.operations
            .open_consultation_batch(planned, task_limits)
            .map_err(ResidentProcessConsultationBatchOpenError::Open)
    }

    /// Return whether the process task owner still retains this exact batch.
    #[must_use]
    fn retains_consultation_batch(&self, expected: &ResidentConsultationBatchCoordinate) -> bool {
        self.operations.retains_consultation_batch(expected)
    }

    /// Run consultation-specific Cancel through one exact admitted deployment.
    ///
    /// Selection rejects every cross-deployment substitution before a port is
    /// lent. The application service then recovers, authorizes, and routes the
    /// request once. A retained physical continuation is wrapped with the same
    /// deployment selector before it can leave the host.
    #[allow(clippy::too_many_arguments)]
    fn request_consultation_cancellation<Authority, AdmitStop, AdmitCollection>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentOperationCancellationInput<'_>,
        admission: ResidentOperationCancellationAdmission<'_>,
        collection_limits: ConsultationCollectionLimits,
        admit_stop: AdmitStop,
        admit_collection: AdmitCollection,
    ) -> Result<
        ResidentConsultationCancellationDisposition,
        ResidentProcessConsultationCancellationError,
    >
    where
        AdmitStop: FnOnce(
            &ConsultationCollection,
            &ConsultationRequirementSummary,
        ) -> Result<(), pale_core::records::InvalidResidentRecordV1>,
        AdmitCollection: FnMut(
            &ConsultationCollection,
        ) -> Result<(), pale_core::records::InvalidResidentRecordV1>,
    {
        let request = input.request.to_preimage();
        let context = &input.recovery.operation.context;
        let outcome = self.with_selected_operation_control_service(
            selector,
            ResidentOperationControlAssociation {
                current_request_authority: &request.signing_authority_binding,
                historical_handle_authority: context.handle_signing_authority,
                recovery_store: context.store,
                checkpoint_authority: context.checkpoint_authority,
            },
            access,
            authority,
            |_application, service, sessions| {
                service.request_consultation_cancellation(
                    input,
                    sessions,
                    admission,
                    collection_limits,
                    admit_stop,
                    admit_collection,
                )
            },
        )?;
        match outcome {
            Err(ResidentConsultationCancellationRequestFailure::Consultation(
                ResidentConsultationCancellationFailure::Preparation(pending),
            )) => Err(ResidentProcessConsultationCancellationError::Preparation(
                Box::new(ResidentProcessConsultationCancellationPreparation {
                    selector: selector.clone(),
                    pending,
                }),
            )),
            Err(ResidentConsultationCancellationRequestFailure::Consultation(
                ResidentConsultationCancellationFailure::Custody(pending),
            )) => Err(ResidentProcessConsultationCancellationError::Custody(
                Box::new(ResidentProcessConsultationCancellationCustody {
                    selector: selector.clone(),
                    pending,
                }),
            )),
            Err(reason) => Err(ResidentProcessConsultationCancellationError::Cancellation(
                reason,
            )),
            Ok(outcome) => Ok(outcome),
        }
    }

    /// Continue Consult cancellation from one exact admitted point recovery.
    #[allow(clippy::too_many_arguments)]
    fn request_consultation_cancellation_from_recovered<Authority, AdmitStop, AdmitCollection>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        recovered: RecoveredOperationControl,
        input: ResidentOperationCancellationInput<'_>,
        admission: ResidentOperationCancellationAdmission<'_>,
        collection_limits: ConsultationCollectionLimits,
        admit_stop: AdmitStop,
        admit_collection: AdmitCollection,
    ) -> Result<
        ResidentConsultationCancellationDisposition,
        ResidentProcessConsultationCancellationError,
    >
    where
        AdmitStop: FnOnce(
            &ConsultationCollection,
            &ConsultationRequirementSummary,
        ) -> Result<(), pale_core::records::InvalidResidentRecordV1>,
        AdmitCollection: FnMut(
            &ConsultationCollection,
        ) -> Result<(), pale_core::records::InvalidResidentRecordV1>,
    {
        let request = input.request.to_preimage();
        let context = &input.recovery.operation.context;
        let outcome = self.with_selected_operation_control_service(
            selector,
            ResidentOperationControlAssociation {
                current_request_authority: &request.signing_authority_binding,
                historical_handle_authority: context.handle_signing_authority,
                recovery_store: context.store,
                checkpoint_authority: context.checkpoint_authority,
            },
            access,
            authority,
            |_application, service, _sessions| {
                service.request_consultation_cancellation_from_recovered(
                    recovered,
                    input,
                    admission,
                    collection_limits,
                    admit_stop,
                    admit_collection,
                )
            },
        )?;
        match outcome {
            Err(ResidentConsultationCancellationRequestFailure::Consultation(
                ResidentConsultationCancellationFailure::Preparation(pending),
            )) => Err(ResidentProcessConsultationCancellationError::Preparation(
                Box::new(ResidentProcessConsultationCancellationPreparation {
                    selector: selector.clone(),
                    pending,
                }),
            )),
            Err(ResidentConsultationCancellationRequestFailure::Consultation(
                ResidentConsultationCancellationFailure::Custody(pending),
            )) => Err(ResidentProcessConsultationCancellationError::Custody(
                Box::new(ResidentProcessConsultationCancellationCustody {
                    selector: selector.clone(),
                    pending,
                }),
            )),
            Err(reason) => Err(ResidentProcessConsultationCancellationError::Cancellation(
                reason,
            )),
            Ok(outcome) => Ok(outcome),
        }
    }

    /// Continue one exact consultation-cancellation custody candidate.
    ///
    /// The embedded selector is rechecked before the candidate enters the
    /// process-wide task owner. Failure returns that same linear candidate;
    /// this path never rebuilds, re-signs, or replays it.
    fn continue_consultation_cancellation_custody<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: Box<ResidentProcessConsultationCancellationCustody>,
    ) -> Result<
        ResidentConsultationCancellationOutcome,
        ResidentProcessConsultationCancellationContinuationFailure,
    > {
        let index = match self.deployment_authority_index(&pending.selector) {
            Ok(index) => index,
            Err(reason) => {
                return Err(ResidentProcessConsultationCancellationContinuationFailure {
                    pending,
                    reason: ResidentProcessConsultationCancellationContinuationError::Selection(
                        reason,
                    ),
                });
            }
        };
        if self.deployments[index].store() != access.store {
            return Err(ResidentProcessConsultationCancellationContinuationFailure {
                pending,
                reason: ResidentProcessConsultationCancellationContinuationError::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation,
                ),
            });
        }
        let ResidentProcessConsultationCancellationCustody { selector, pending } = *pending;
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let result =
            self.operations
                .with_operation_service(authority, custody, anchor, |service| {
                    service.continue_consultation_cancellation_custody(pending)
                });
        result.map_err(
            |pending| ResidentProcessConsultationCancellationContinuationFailure {
                pending: Box::new(ResidentProcessConsultationCancellationCustody {
                    selector,
                    pending,
                }),
                reason: ResidentProcessConsultationCancellationContinuationError::Custody,
            },
        )
    }

    /// Preserve one exact pre-write Consult cancellation owner through its deployment.
    fn continue_consultation_cancellation_preparation<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: Box<ResidentProcessConsultationCancellationPreparation>,
    ) -> Result<(), ResidentProcessConsultationCancellationPreparationContinuationFailure> {
        let index = match self.deployment_authority_index(&pending.selector) {
            Ok(index) => index,
            Err(reason) => {
                return Err(
                    ResidentProcessConsultationCancellationPreparationContinuationFailure {
                        pending,
                        reason: ResidentProcessConsultationCancellationContinuationError::Selection(
                            reason,
                        ),
                    },
                );
            }
        };
        if self.deployments[index].store() != access.store {
            return Err(
                ResidentProcessConsultationCancellationPreparationContinuationFailure {
                    pending,
                    reason: ResidentProcessConsultationCancellationContinuationError::Selection(
                        ResidentProcessHostSelectionError::AccessStoreAssociation,
                    ),
                },
            );
        }
        let ResidentProcessConsultationCancellationPreparation { selector, pending } = *pending;
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let result =
            self.operations
                .with_operation_service(authority, custody, anchor, |service| {
                    service.continue_consultation_cancellation_preparation(*pending)
                });
        result.map_err(|pending| {
            ResidentProcessConsultationCancellationPreparationContinuationFailure {
                pending: Box::new(ResidentProcessConsultationCancellationPreparation {
                    selector,
                    pending,
                }),
                reason: ResidentProcessConsultationCancellationContinuationError::Custody,
            }
        })
    }

    /// Run one recovered Consult turn through one exact admitted deployment.
    ///
    /// Recovery uses the selected deployment's ordinary operation facade. That
    /// facade and its custody and anchor borrows end before the process task
    /// owner receives the recovered collection.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    fn supervise_consultation_turn<
        Authority,
        Work,
        Output,
        RecoveryError,
        Recover,
        ChildRecoveryError,
        RecoverChild,
        RetentionFailure,
        Retain,
        Executor,
        StopInput,
        Decide,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        task_limits: ResidentConsultationTaskLimits,
        observed_at: UnixTimeV1,
        recover: Recover,
        executor: &mut Executor,
        decide: Decide,
        recover_child: RecoverChild,
        retain: Retain,
    ) -> Result<
        ResidentConsultationTurnDisposition<
            Work,
            Output,
            RecoveryError,
            ChildRecoveryError,
            RetentionFailure,
            StopInput,
        >,
        ResidentProcessHostSelectionError,
    >
    where
        Recover: FnOnce(
            &ResidentApplication<'store>,
            &mut ResidentOperationService<'_, Authority>,
            &dyn SessionLeaseCustodyPort,
        ) -> Result<RecoveredConsultationCollection, RecoveryError>,
        Executor: ResidentOperationTaskExecutor<Work, Output>,
        Decide: FnOnce(
            &RecoveredConsultationCollection,
            UnixTimeV1,
        ) -> ResidentConsultationSupervisionDecision<StopInput>,
        RecoverChild: FnOnce(
            &ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, ChildRecoveryError>,
        Retain: FnOnce(
            &ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
            &mut dyn OperationCustodyPort,
            &mut dyn AntiRollbackAnchorPort,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
    {
        let selected_store = self.deployments[self.deployment_authority_index(selector)?]
            .store()
            .clone();
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let recovered = self.with_selected_operation_control_service(
            selector,
            association,
            ResidentDeploymentOperationAccess {
                store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            authority,
            recover,
        )?;
        let current = match recovered {
            Ok(current) => current,
            Err(reason) => {
                return Ok(ResidentConsultationTurnDisposition::NeedsRecovery(
                    Box::new(ResidentConsultationTurnNeedsRecovery::Collection(reason)),
                ));
            }
        };
        if current.store() != &selected_store {
            return Err(ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation);
        }
        Ok(self.operations.supervise_recovered_consultation_turn(
            task_limits,
            observed_at,
            current,
            executor,
            decide,
            recover_child,
            |candidate, child| retain(candidate, child, custody, anchor),
        ))
    }

    /// Admit one recovered child terminal into its exact Consult slot.
    ///
    /// The retained lineage selects the parent and slot. Child success remains
    /// attributable evidence; this method does not accept it as the parent result.
    fn admit_consultation_child_terminal<Admit>(
        &self,
        parent: &restart::ResidentProcessRecoveredOperationStart,
        current: &RecoveredConsultationCollection,
        child: &restart::ResidentProcessRecoveredOperationStart,
        recorded_at: UnixTimeV1,
        limits: ConsultationCollectionLimits,
        admit: Admit,
    ) -> Result<ConsultationCollection, ResidentProcessConsultationChildTerminalError>
    where
        Admit: FnOnce(
            &super::consultation::ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let Some(lineage) = child.lineage.consult_child() else {
            return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation.into());
        };
        let parent_operation = parent.control.operation().operation();
        let child_operation = child.control.operation().operation();
        let (parent_ref, parent_handle) = parent_operation.handle();
        let (child_ref, child_handle) = child_operation.handle();
        if parent.selector != child.selector
            || parent.generation_digest != child.generation_digest
            || parent.control.operation().store() != current.store()
            || child.control.operation().store() != current.store()
            || current.collection().parent().record_ref != *parent_ref
            || current.collection().parent().record_sha256.as_str()
                != parent_handle.canonical_operation_handle_sha256().as_str()
            || lineage.child().record_ref != *child_ref
            || lineage.child().record_sha256.as_str()
                != child_handle.canonical_operation_handle_sha256().as_str()
            || !child.lineage.admits(child_ref, child_handle)
            || parent_handle.operation_class() != &OperationClassV1::Consult
        {
            return Err(ResidentProcessHostSelectionError::ConsultationChildAssociation.into());
        }
        current
            .admit_recovered_child_terminal(
                lineage.slot_id(),
                child.control.observation(),
                recorded_at,
                limits,
                admit,
            )
            .map_err(Into::into)
    }

    /// Bind one Package-selected Consult stop to its admitted deployment.
    ///
    /// This phase performs no port I/O. A rejected selector returns the intact
    /// stop so another exact selector can be tried without running policy again.
    fn bind_consultation_stop<'operation>(
        &self,
        selector: ResidentDeploymentAuthoritySelector,
        selected: ResidentConsultationStopSelected<ConsultationStopInput<'operation>>,
    ) -> Result<
        ResidentProcessConsultationStopSelected<'operation>,
        Box<ResidentProcessConsultationStopBindingFailure<'operation>>,
    > {
        match self.deployment_authority_index(&selector) {
            Ok(index) if self.deployments[index].store() == selected.store() => {
                Ok(ResidentProcessConsultationStopSelected {
                    selector,
                    selected: Box::new(selected),
                })
            }
            Ok(_) => Err(Box::new(ResidentProcessConsultationStopBindingFailure {
                selector,
                selected: Box::new(selected),
                reason: ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation,
            })),
            Err(reason) => Err(Box::new(ResidentProcessConsultationStopBindingFailure {
                selector,
                selected: Box::new(selected),
                reason,
            })),
        }
    }

    /// Commit one bound Consult stop through exact database and anchor custody.
    ///
    /// The process host checks the deployment, current request, historical
    /// handle, store, and checkpoint associations before it borrows either
    /// physical port. The application service retains semantic stop,
    /// collection, and observation-signing admission before a custody write.
    /// The returned phase always retains the sole safe next action.
    #[allow(clippy::too_many_arguments)]
    fn commit_consultation_stop<
        'operation,
        Authority,
        AdmitStop,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        selected: ResidentProcessConsultationStopSelected<'operation>,
        checkpoint: ConsultationSuccessorCommitInput<'_>,
        resolve_store: &dyn super::OperationObservationAuthorityStoreResolver,
        admit_stop: AdmitStop,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        CompletedConsultationCollection,
        ResidentProcessConsultationStopCommitFailure<'operation>,
    >
    where
        AdmitStop: FnOnce(
            &ConsultationCollection,
            &ConsultationRequirementSummary,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessConsultationStopSelected { selector, selected } = selected;
        let phase_store = selected.store().clone();
        let index = match self.validate_consultation_stop_selection(
            &selector,
            association,
            access.store,
            &selected,
            &checkpoint,
        ) {
            Ok(index) => index,
            Err(reason) => {
                return Err(ResidentProcessConsultationStopCommitFailure::Selection {
                    selected: Box::new(ResidentProcessConsultationStopSelected {
                        selector,
                        selected,
                    }),
                    reason,
                });
            }
        };
        let Self {
            operations,
            deployments,
            ..
        } = self;
        debug_assert_eq!(deployments[index].store(), access.store);
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let result = operations.with_operation_service(authority, custody, anchor, |service| {
            service.commit_selected_consultation_stop(
                *selected,
                checkpoint,
                resolve_store,
                admit_stop,
                admit_collection,
                admit_rotation,
                sign_observation,
                sign_checkpoint,
            )
        });
        match result {
            Ok(completed) => Ok(completed),
            Err(ResidentConsultationStopCommitFailure::Admission { selected, reason }) => {
                Err(ResidentProcessConsultationStopCommitFailure::Admission {
                    selected: Box::new(ResidentProcessConsultationStopSelected {
                        selector,
                        selected,
                    }),
                    reason,
                })
            }
            Err(ResidentConsultationStopCommitFailure::Custody(pending)) => {
                Err(ResidentProcessConsultationStopCommitFailure::Custody(
                    Box::new(ResidentProcessConsultationStopCustody {
                        selector,
                        store: phase_store,
                        pending,
                    }),
                ))
            }
        }
    }

    /// Continue only the retained database or anchor candidate for one stop.
    ///
    /// The embedded deployment and store are reselected before physical ports
    /// are borrowed. This method never reconstructs or repeats stop admission.
    fn continue_consultation_stop_custody<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessConsultationStopCustody,
    ) -> Result<CompletedConsultationCollection, ResidentProcessConsultationStopContinuationFailure>
    {
        let ResidentProcessConsultationStopCustody {
            selector,
            store,
            pending,
        } = pending;
        let index = match self.validate_consultation_stop_phase_selection(
            &selector,
            access.store,
            &store,
        ) {
            Ok(index) => index,
            Err(reason) => {
                return Err(ResidentProcessConsultationStopContinuationFailure {
                    pending: Box::new(ResidentProcessConsultationStopCustody {
                        selector,
                        store,
                        pending,
                    }),
                    reason: ResidentProcessConsultationStopContinuationError::Selection(reason),
                });
            }
        };
        let Self {
            operations,
            deployments,
            ..
        } = self;
        debug_assert_eq!(deployments[index].store(), access.store);
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let result = operations.with_operation_service(authority, custody, anchor, |service| {
            service.continue_consultation_stop_custody(*pending)
        });
        result.map_err(
            |pending| ResidentProcessConsultationStopContinuationFailure {
                pending: Box::new(ResidentProcessConsultationStopCustody {
                    selector,
                    store,
                    pending: Box::new(pending),
                }),
                reason: ResidentProcessConsultationStopContinuationError::Custody,
            },
        )
    }

    /// Validate process-host authority and store associations before port I/O.
    fn validate_consultation_stop_selection(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        association: ResidentOperationControlAssociation<'_>,
        access_store: &AuthorityStoreCoordinate,
        selected: &ResidentConsultationStopSelected<ConsultationStopInput<'_>>,
        checkpoint: &ConsultationSuccessorCommitInput<'_>,
    ) -> Result<usize, ResidentProcessHostSelectionError> {
        if !selector.matches_signing_authority(association.current_request_authority) {
            return Err(ResidentProcessHostSelectionError::CurrentRequestAuthority);
        }
        if !selector.matches_signing_authority(association.historical_handle_authority) {
            return Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority);
        }
        let index = self.deployment_authority_index(selector)?;
        let selected_store = self.deployments[index].store();
        if association.recovery_store != selected_store {
            return Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation);
        }
        if !checkpoint_authority_matches(selector, selected_store, association.checkpoint_authority)
        {
            return Err(ResidentProcessHostSelectionError::CheckpointAuthorityAssociation);
        }
        self.validate_consultation_stop_phase_selection(selector, access_store, selected.store())?;
        if !checkpoint_authority_matches(selector, selected_store, checkpoint.authority)
            || !same_checkpoint_authority(association.checkpoint_authority, checkpoint.authority)
        {
            return Err(ResidentProcessHostSelectionError::ConsultationCheckpointAuthority);
        }
        Ok(index)
    }

    /// Validate one retained stop phase before custody continuation.
    fn validate_consultation_stop_phase_selection(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &AuthorityStoreCoordinate,
        phase_store: &AuthorityStoreCoordinate,
    ) -> Result<usize, ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(selector)?;
        let selected_store = self.deployments[index].store();
        if access_store != selected_store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        if phase_store != selected_store {
            return Err(ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation);
        }
        Ok(index)
    }

    /// Join one ready Consult child through its exact deployment and release capacity.
    ///
    /// Deployment, request-authority, checkpoint-authority, access-store, and
    /// recovered-store checks finish before custody is borrowed. Every refusal
    /// returns the same linear completion, custody, or release phase.
    #[allow(clippy::too_many_arguments)]
    fn join_consultation_terminal<
        'operation,
        Authority,
        Work,
        Output,
        AdmitEvidence,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        ready: ResidentProcessConsultationCompletionReady<Work, Output>,
        input: ConsultationChildJoinInput<'operation>,
        checkpoint: ConsultationSuccessorCommitInput<'_>,
        resolve_store: &dyn super::OperationObservationAuthorityStoreResolver,
        admit_evidence: AdmitEvidence,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        CompletedResidentConsultationTerminal<Work, Output>,
        ResidentProcessConsultationTerminalJoinFailure<Work, Output>,
    >
    where
        AdmitEvidence: FnOnce(
            &super::consultation::ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessConsultationCompletionReady { selector, ready } = ready;
        let index = match self.validate_consultation_terminal_selection(
            &selector,
            association,
            access.store,
            ready.store(),
            &input,
            &checkpoint,
        ) {
            Ok(index) => index,
            Err(reason) => {
                return Err(ResidentProcessConsultationTerminalJoinFailure::Selection {
                    ready: Box::new(ResidentProcessConsultationCompletionReady { selector, ready }),
                    reason,
                });
            }
        };
        let Self {
            operations,
            deployments,
            ..
        } = self;
        debug_assert_eq!(deployments[index].store(), access.store);
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let result = operations.with_operation_service(authority, custody, anchor, |service| {
            service.join_consultation_terminal(
                *ready,
                input,
                checkpoint,
                resolve_store,
                admit_evidence,
                admit_collection,
                admit_rotation,
                sign_observation,
                sign_checkpoint,
            )
        });
        result.map_err(|failure| map_process_consultation_terminal_failure(selector, failure))
    }

    /// Continue only the retained database or anchor candidate for one Consult join.
    ///
    /// The embedded deployment is reselected before the physical ports are
    /// borrowed. This method never reconstructs or repeats the child join.
    fn continue_consultation_terminal_custody<Authority, Work, Output>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessConsultationTerminalCustody<Work, Output>,
    ) -> Result<
        CompletedResidentConsultationTerminal<Work, Output>,
        ResidentProcessConsultationTerminalJoinFailure<Work, Output>,
    > {
        let ResidentProcessConsultationTerminalCustody { selector, pending } = pending;
        let index = match self.validate_consultation_terminal_phase_selection(
            &selector,
            access.store,
            pending.store(),
        ) {
            Ok(index) => index,
            Err(reason) => {
                return Err(
                    ResidentProcessConsultationTerminalJoinFailure::CustodySelection {
                        pending: Box::new(ResidentProcessConsultationTerminalCustody {
                            selector,
                            pending,
                        }),
                        reason,
                    },
                );
            }
        };
        let Self {
            operations,
            deployments,
            ..
        } = self;
        debug_assert_eq!(deployments[index].store(), access.store);
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let result = operations.with_operation_service(authority, custody, anchor, |service| {
            service.continue_consultation_terminal_custody(*pending)
        });
        result.map_err(|failure| map_process_consultation_terminal_failure(selector, failure))
    }

    /// Retry only aggregate capacity release after a Consult join reached custody.
    ///
    /// No operation, checkpoint, database, anchor, signer, or executor work is
    /// repeated. The embedded deployment and task owner remain exact.
    fn retry_consultation_terminal_release<Work, Output>(
        &mut self,
        joined: ResidentProcessConsultationJoinedTerminal<Work, Output>,
    ) -> Result<
        CompletedResidentConsultationTerminal<Work, Output>,
        ResidentProcessConsultationTerminalJoinFailure<Work, Output>,
    > {
        let ResidentProcessConsultationJoinedTerminal { selector, joined } = joined;
        if let Err(reason) = self.validate_consultation_terminal_phase_selection(
            &selector,
            joined.store(),
            joined.store(),
        ) {
            return Err(
                ResidentProcessConsultationTerminalJoinFailure::ReleaseSelection {
                    joined: Box::new(ResidentProcessConsultationJoinedTerminal {
                        selector,
                        joined,
                    }),
                    reason,
                },
            );
        }
        self.operations
            .retry_consultation_terminal_release(*joined)
            .map_err(|failure| map_process_consultation_terminal_failure(selector, failure))
    }

    /// Validate every borrowed authority and store before a terminal join.
    fn validate_consultation_terminal_selection(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        association: ResidentOperationControlAssociation<'_>,
        access_store: &AuthorityStoreCoordinate,
        phase_store: &AuthorityStoreCoordinate,
        input: &ConsultationChildJoinInput<'_>,
        checkpoint: &ConsultationSuccessorCommitInput<'_>,
    ) -> Result<usize, ResidentProcessHostSelectionError> {
        if !selector.matches_signing_authority(association.current_request_authority) {
            return Err(ResidentProcessHostSelectionError::CurrentRequestAuthority);
        }
        if !selector.matches_signing_authority(association.historical_handle_authority) {
            return Err(ResidentProcessHostSelectionError::HistoricalHandleAuthority);
        }
        let index = self.deployment_authority_index(selector)?;
        let selected_store = self.deployments[index].store();
        if association.recovery_store != selected_store {
            return Err(ResidentProcessHostSelectionError::RecoveryStoreAssociation);
        }
        if !checkpoint_authority_matches(selector, selected_store, association.checkpoint_authority)
        {
            return Err(ResidentProcessHostSelectionError::CheckpointAuthorityAssociation);
        }
        self.validate_consultation_terminal_phase_selection(selector, access_store, phase_store)?;
        validate_consultation_terminal_associations(
            selector,
            selected_store,
            &association,
            input,
            checkpoint,
        )?;
        Ok(index)
    }

    /// Validate a retained Consult terminal phase before any process mutation.
    fn validate_consultation_terminal_phase_selection(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &AuthorityStoreCoordinate,
        phase_store: &AuthorityStoreCoordinate,
    ) -> Result<usize, ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(selector)?;
        let selected_store = self.deployments[index].store();
        if access_store != selected_store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        if phase_store != selected_store {
            return Err(ResidentProcessHostSelectionError::RecoveredCollectionStoreAssociation);
        }
        Ok(index)
    }
}

/// Check the actual terminal inputs after deployment selection and before I/O.
fn validate_consultation_terminal_associations(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    association: &ResidentOperationControlAssociation<'_>,
    input: &ConsultationChildJoinInput<'_>,
    checkpoint: &ConsultationSuccessorCommitInput<'_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let (_, parent_handle) = input.parent.handle();
    let historical_session = input.parent.historical_session_lease();
    let (_, child_head) = input.child.selected_head();
    if parent_handle.operation_class() != &OperationClassV1::Consult
        || !selector.matches_operation(input.parent)
        || parent_handle.issuer_key_id().as_str()
            != association.historical_handle_authority.signer_key_id()
        || parent_handle.issuer_key_epoch()
            != association.historical_handle_authority.signer_key_epoch()
        || historical_session.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
        || historical_session
            .deployment_runtime_authority_sha256()
            .as_str()
            != selector.runtime_authority_sha256.as_str()
        || historical_session.issuer_principal_id() != &selector.principal_id
        || input.child.store() != store
        || !operation_observation_matches_deployment(selector, input.parent_observation.1)
        || !operation_observation_matches_deployment(selector, child_head)
    {
        return Err(ResidentProcessHostSelectionError::ConsultationOperationAuthority);
    }
    if !selector.matches_signing_authority(input.signing.signing_authority) {
        return Err(ResidentProcessHostSelectionError::ConsultationSigningAuthority);
    }
    let current_checkpoint = input.child.current_checkpoint().1;
    if current_checkpoint.control_store_id() != store.store_id()
        || current_checkpoint.control_incarnation_id() != store.incarnation_id()
        || current_checkpoint.issuer_principal_id() != &selector.principal_id
        || !checkpoint_authority_matches(selector, store, checkpoint.authority)
        || !same_checkpoint_authority(association.checkpoint_authority, checkpoint.authority)
    {
        return Err(ResidentProcessHostSelectionError::ConsultationCheckpointAuthority);
    }
    Ok(())
}

fn operation_observation_matches_deployment(
    selector: &ResidentDeploymentAuthoritySelector,
    observation: &pale_core::records::OperationObservationV1,
) -> bool {
    observation.deployment_runtime_authority_ref() == &selector.runtime_authority_ref
        && observation.deployment_runtime_authority_sha256().as_str()
            == selector.runtime_authority_sha256.as_str()
        && observation.issuer_principal_id() == &selector.principal_id
}

fn validate_provider_launch_associations(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    dispatch: &ProviderOperationDispatchInput<'_, '_>,
    invocation: &ProviderOperationInvokeInput<'_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let context = &dispatch.operation.context;
    let (handle_ref, handle) = context.operation.handle();
    if !selector.matches_current_session(context.current)
        || !selector.matches_current_session(invocation.current)
    {
        return Err(ResidentProcessHostSelectionError::ProviderCurrentAuthority);
    }
    if !selector.matches_operation(context.operation)
        || context.observation.handle_ref != handle_ref
        || context.observation.handle != handle
    {
        return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
    }
    let historical_session = context.operation.historical_session_lease();
    if context.session.lease() != historical_session
        || context.lease != context.operation.historical_operation_lease()
        || historical_session.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
        || historical_session
            .deployment_runtime_authority_sha256()
            .as_str()
            != selector.runtime_authority_sha256.as_str()
        || historical_session.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::HistoricalProviderAuthority);
    }
    if !selector.matches_signing_authority(context.observation.signed.signing_authority)
        || !selector.matches_signing_authority(dispatch.signing.signing_authority)
        || !selector.matches_signing_authority(invocation.signing.signing_authority)
    {
        return Err(ResidentProcessHostSelectionError::ProviderObservationAuthority);
    }
    if !checkpoint_authority_matches(selector, store, dispatch.checkpoint_authority)
        || !checkpoint_authority_matches(selector, store, invocation.checkpoint_authority)
    {
        return Err(ResidentProcessHostSelectionError::ProviderCheckpointAuthority);
    }
    Ok(())
}

fn validate_recovered_pairwise_provider_dispatch(
    application: &ResidentApplication<'_>,
    selector: &ResidentDeploymentAuthoritySelector,
    recovered: &restart::ResidentProcessRecoveredOperationResolution,
    resolved: &super::operation_dispatch::OperationAttemptContext<'_>,
    checkpoint: (
        &ResidentRecordRefV1,
        &pale_core::records::ControlStateCheckpointV1,
    ),
    dispatch: &ProviderOperationDispatchInput<'_, '_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let lineage = &recovered.start.lineage;
    if !matches!(
        lineage,
        restart::ResidentProcessOperationLineage::PairwiseDelegate(_)
    ) || !lineage.admits(resolved.operation.handle().0, resolved.operation.handle().1)
        || resolved.operation.handle().1.operation_class() != &OperationClassV1::Delegate
    {
        return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
    }
    let supplied = &dispatch.operation.context;
    let same_binding = match (resolved.context_binding, supplied.context_binding) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            let (left_ref, left_observation) = left.observation();
            let (right_ref, right_observation) = right.observation();
            left_ref == right_ref
                && left_observation.canonical_operation_observation_sha256()
                    == right_observation.canonical_operation_observation_sha256()
        }
        _ => false,
    };
    if !selector.matches_operation(supplied.operation)
        || !selector.matches_current_session(supplied.current)
        || resolved.operation.handle() != supplied.operation.handle()
        || resolved.request != supplied.request
        || resolved.session.lease() != supplied.session.lease()
        || resolved.lease != supplied.lease
        || resolved.generation.record().compiled_generation_digest()
            != supplied.generation.record().compiled_generation_digest()
        || resolved.generation.record().instance_digest()
            != supplied.generation.record().instance_digest()
        || resolved.selection_ref != supplied.selection_ref
        || resolved.maximum_selection_bytes != supplied.maximum_selection_bytes
        || resolved.observation_ref != supplied.observation_ref
        || resolved.observation.handle_ref != supplied.observation.handle_ref
        || resolved.observation.handle != supplied.observation.handle
        || resolved.observation.predecessor != supplied.observation.predecessor
        || !same_binding
        || dispatch.checkpoint.0 != checkpoint.0
        || dispatch.checkpoint.1 != checkpoint.1
        || !selector.matches_signing_authority(dispatch.signing.signing_authority)
        || !checkpoint_authority_matches(
            selector,
            application.store(),
            dispatch.checkpoint_authority,
        )
    {
        return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
    }
    Ok(())
}

fn checkpoint_authority_matches(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
) -> bool {
    authority.principal_id == &selector.principal_id
        && authority.control_store_id == store.store_id()
        && authority.control_incarnation_id == store.incarnation_id()
}

fn accepted_provider_key<'a>(
    failure: &'a ResidentProviderTaskAcceptanceFailure<'_>,
) -> Option<&'a ResidentOperationTaskKey> {
    match failure {
        ResidentProviderTaskAcceptanceFailure::AggregateSaturated(_)
        | ResidentProviderTaskAcceptanceFailure::Preflight(_) => None,
        ResidentProviderTaskAcceptanceFailure::TaskOwner(refusal) => {
            accepted_provider_refusal_key(refusal)
        }
    }
}

fn accepted_provider_refusal_key<'a>(
    refusal: &'a ResidentProviderTaskRefusal<'_>,
) -> Option<&'a ResidentOperationTaskKey> {
    match refusal {
        ResidentProviderTaskRefusal::ExecutorProtocolLoss { expected_key, .. } => {
            Some(expected_key)
        }
        ResidentProviderTaskRefusal::AggregateTransition { outcome, .. } => {
            match outcome.as_ref() {
                ResidentProviderTaskAggregateTransitionOutcome::Accepted { key } => Some(key),
                ResidentProviderTaskAggregateTransitionOutcome::Refused(refusal) => {
                    accepted_provider_refusal_key(refusal)
                }
            }
        }
        ResidentProviderTaskRefusal::Preparation { .. }
        | ResidentProviderTaskRefusal::Duplicate(_)
        | ResidentProviderTaskRefusal::Saturated { .. }
        | ResidentProviderTaskRefusal::Executor { .. }
        | ResidentProviderTaskRefusal::OfferIdentityExhausted(_) => None,
    }
}

fn accepted_recovered_provider_key(
    failure: &ResidentRecoveredProviderTaskAcceptanceFailure,
) -> Option<&ResidentOperationTaskKey> {
    match failure {
        ResidentRecoveredProviderTaskAcceptanceFailure::AggregateSaturated(_)
        | ResidentRecoveredProviderTaskAcceptanceFailure::Preflight(_) => None,
        ResidentRecoveredProviderTaskAcceptanceFailure::TaskOwner(refusal) => {
            accepted_recovered_provider_refusal_key(refusal)
        }
    }
}

fn accepted_recovered_provider_refusal_key(
    refusal: &ResidentRecoveredProviderTaskRefusal,
) -> Option<&ResidentOperationTaskKey> {
    match refusal {
        ResidentRecoveredProviderTaskRefusal::ExecutorProtocolLoss { expected_key, .. } => {
            Some(expected_key)
        }
        ResidentRecoveredProviderTaskRefusal::AggregateTransition { outcome, .. } => {
            match outcome.as_ref() {
                ResidentRecoveredProviderTaskAggregateTransitionOutcome::Accepted { key } => {
                    Some(key)
                }
                ResidentRecoveredProviderTaskAggregateTransitionOutcome::Refused(refusal) => {
                    accepted_recovered_provider_refusal_key(refusal)
                }
            }
        }
        ResidentRecoveredProviderTaskRefusal::Duplicate(_)
        | ResidentRecoveredProviderTaskRefusal::Saturated { .. }
        | ResidentRecoveredProviderTaskRefusal::Executor { .. }
        | ResidentRecoveredProviderTaskRefusal::OfferIdentityExhausted(_) => None,
    }
}

fn is_unaccepted_provider_aggregate_transition(refusal: &ResidentProviderTaskRefusal<'_>) -> bool {
    match refusal {
        ResidentProviderTaskRefusal::AggregateTransition { outcome, .. } => {
            match outcome.as_ref() {
                ResidentProviderTaskAggregateTransitionOutcome::Refused(refusal) => {
                    accepted_provider_refusal_key(refusal).is_none()
                }
                ResidentProviderTaskAggregateTransitionOutcome::Accepted { .. } => false,
            }
        }
        ResidentProviderTaskRefusal::Preparation { .. }
        | ResidentProviderTaskRefusal::Duplicate(_)
        | ResidentProviderTaskRefusal::Saturated { .. }
        | ResidentProviderTaskRefusal::Executor { .. }
        | ResidentProviderTaskRefusal::OfferIdentityExhausted(_)
        | ResidentProviderTaskRefusal::ExecutorProtocolLoss { .. } => false,
    }
}

fn validate_deployment_stores(
    deployments: &[ReadyResidentDeployment<'_>],
) -> Result<(), ResidentProcessHostBuildError> {
    for pair in deployments.windows(2) {
        let left = pair[0].store();
        let right = pair[1].store();
        if left == right {
            return Err(ResidentProcessHostBuildError::DuplicateDeploymentStore);
        }
        if left.store_id() == right.store_id() {
            return Err(ResidentProcessHostBuildError::MultipleReadyIncarnations);
        }
    }
    Ok(())
}

fn validate_deployment_authorities(
    deployments: &[ReadyResidentDeployment<'_>],
) -> Result<(), ResidentProcessHostBuildError> {
    for (index, left) in deployments.iter().enumerate() {
        if deployments[(index + 1)..]
            .iter()
            .any(|right| authority_associations_equal(&left.association, &right.association))
        {
            return Err(ResidentProcessHostBuildError::DuplicateDeploymentAuthority);
        }
    }
    Ok(())
}

fn authority_associations_equal(
    left: &AdmittedResidentDeploymentAssociation,
    right: &AdmittedResidentDeploymentAssociation,
) -> bool {
    left.principal_id == right.principal_id
        && left.deployment_binding_ref == right.deployment_binding_ref
        && left.deployment_binding_sha256 == right.deployment_binding_sha256
        && left.runtime_authority_ref == right.runtime_authority_ref
        && left.runtime_authority_sha256 == right.runtime_authority_sha256
}

enum ResidentProcessProviderLaunchDisposition<'operation, 'call> {
    Accepted {
        reservation: ResidentProcessProviderRouteReservation,
        key: ResidentOperationTaskKey,
    },
    AcceptedWithFailure {
        reservation: ResidentProcessProviderRouteReservation,
        key: ResidentOperationTaskKey,
        failure: ResidentProviderTaskAcceptanceFailure<'operation>,
    },
    Rejected(ResidentProcessProviderLaunchFailure<'operation, 'call>),
}

/// Provider launch stopped while retaining every non-replayable phase.
#[must_use = "retry only the returned exact provider phase"]
enum ResidentProcessProviderLaunchFailure<'operation, 'call> {
    /// Exact deployment or authority selection failed before any I/O.
    Selection {
        dispatch: Box<ProviderOperationDispatchInput<'operation, 'call>>,
        reason: ResidentProcessHostSelectionError,
    },
    /// The process-host route ledger was full before durable retention.
    RouteSaturated(Box<ProviderOperationDispatchInput<'operation, 'call>>),
    /// Durable retention stopped with its existing exact continuation.
    Retention {
        reservation: ResidentProcessProviderRouteReservation,
        failure: ResidentProviderDispatchRetentionFailure<'operation, 'call>,
    },
    /// Retained custody resolved to another physical deployment store.
    RetainedStoreAssociation {
        reservation: ResidentProcessProviderRouteReservation,
        retained: Box<RetainedProviderOperationDispatch<'operation>>,
    },
    /// The unopened retained call did not transfer to the executor.
    Acceptance {
        reservation: ResidentProcessProviderRouteReservation,
        failure: ResidentProviderTaskAcceptanceFailure<'operation>,
    },
    /// The executor may own the call despite a protocol or aggregate fault.
    ///
    /// The route was captured before this failure was returned. The call must
    /// not be retained, signed, preflighted, or offered again.
    AcceptedWithFailure {
        selector: ResidentDeploymentAuthoritySelector,
        key: ResidentOperationTaskKey,
        failure: ResidentProviderTaskAcceptanceFailure<'operation>,
    },
}

/// Recovered provider release stopped while retaining every linear phase.
#[must_use = "retry only the returned exact recovered provider phase"]
enum ResidentProcessRecoveredProviderLaunchFailure {
    Selection {
        release: Box<RecoveredProviderDispatchRelease>,
        reason: ResidentProcessHostSelectionError,
    },
    RouteSaturated(Box<RecoveredProviderDispatchRelease>),
    Acceptance {
        reservation: ResidentProcessProviderRouteReservation,
        failure: ResidentRecoveredProviderTaskAcceptanceFailure,
    },
    AcceptedWithFailure {
        selector: ResidentDeploymentAuthoritySelector,
        key: ResidentOperationTaskKey,
        failure: ResidentRecoveredProviderTaskAcceptanceFailure,
    },
}

/// Lower provider phase preserved after explicit release of an unopened route.
#[must_use = "retain or deliberately settle the returned lower provider phase"]
enum ResidentProcessProviderAbandonedLaunch<'operation, 'call> {
    Retention(ResidentProviderDispatchRetentionFailure<'operation, 'call>),
    RetainedStoreAssociation(Box<RetainedProviderOperationDispatch<'operation>>),
    Acceptance(ResidentProviderTaskAcceptanceFailure<'operation>),
}

/// Provider completion joined to the deployment retained before acceptance.
#[must_use = "continue the completion through its retained deployment"]
struct ResidentProcessProviderCompletion {
    route: ResidentProcessProviderCompletionRoute,
    completion: ResidentProviderCompletionPoll,
}

enum ResidentProcessProviderCompletionRoute {
    /// Aggregate transition failed, so task and route remain active.
    Active {
        selector: ResidentDeploymentAuthoritySelector,
    },
    /// Returned work left the task owner and carries linear route custody.
    CheckedOut(ResidentProcessProviderRouteCustody),
}

/// Provider durable-recording progress with its exact process-host route.
#[must_use = "continue nonterminal provider progress or retain its settled result"]
enum ResidentProcessProviderRecording<'operation> {
    /// Durable recording and aggregate release both completed.
    Recorded {
        result: ProviderOperationInvokeResult,
        cancellation: ResidentOperationTaskCancellationState,
    },
    /// Existing lower recording phase still owns durable progress.
    Pending {
        route: ResidentProcessProviderRouteCustody,
        progress: ResidentProviderTaskPoll<'operation>,
    },
    /// Durable recording succeeded but the process-host route release was rejected.
    RouteReleaseRejected {
        route: ResidentProcessProviderRouteCustody,
        result: ProviderOperationInvokeResult,
        cancellation: ResidentOperationTaskCancellationState,
    },
}

/// Why a returned provider completion did not enter durable recording.
#[must_use = "retry with the returned exact route and completion phase"]
enum ResidentProcessProviderCompletionRecordingFailure<'operation> {
    Phase(ResidentProcessProviderCompletion),
    RecordedPhase(ResidentProcessProviderRecording<'operation>),
    Selection {
        route: ResidentProcessProviderRouteCustody,
        completed: Box<super::provider_task_service::ResidentCompletedProviderTask>,
        reason: ResidentProcessHostSelectionError,
    },
    Aggregate {
        route: ResidentProcessProviderRouteCustody,
        rejection: ResidentProviderCompletionRecoveryRejection,
    },
}

/// Why a returned provider loss did not enter protected uncertainty custody.
#[must_use = "retry with the returned exact route and loss phase"]
enum ResidentProcessProviderLossRecordingFailure {
    Phase(ResidentProcessProviderCompletion),
    Selection {
        route: ResidentProcessProviderRouteCustody,
        lost: Box<super::provider_task_service::ResidentLostProviderTask>,
        reason: ResidentProcessHostSelectionError,
    },
    Recovery {
        route: ResidentProcessProviderRouteCustody,
        failure: ResidentProviderTaskRecoveryFailure,
    },
}

/// Host-fence failure while continuing one lower provider recording phase.
#[must_use = "retry with the unchanged host-fenced recording phase"]
enum ResidentProcessProviderRecordingContinuationFailure<'operation> {
    Phase(ResidentProcessProviderRecording<'operation>),
    Selection {
        recording: ResidentProcessProviderRecording<'operation>,
        reason: ResidentProcessHostSelectionError,
    },
    Preparation {
        route: ResidentProcessProviderRouteCustody,
        rejection: ResidentProviderRecordingContinuationRejection<'operation>,
    },
    ConcurrentHead {
        route: ResidentProcessProviderRouteCustody,
        rejection: ResidentProviderRecordingConflictRejection<'operation>,
    },
    Custody {
        route: ResidentProcessProviderRouteCustody,
        rejection: ResidentProviderRecordingCustodyRejection,
    },
    Finalization {
        route: ResidentProcessProviderRouteCustody,
        rejection: ResidentProviderRecordingFinalizationRejection,
    },
}

/// One non-blocking process-host provider poll.
#[must_use = "provider progress can retain route and settlement custody"]
enum ResidentProcessProviderCompletionPoll {
    /// Exact deployment route joined to accepted provider progress.
    Routed(Box<ResidentProcessProviderCompletion>),
    /// Completion belongs to no task accepted by this process owner.
    Stale(Box<ResidentProviderCompletionPoll>),
    /// Accepted progress had no retained route, which is an internal defect.
    Unrouted(Box<ResidentProviderCompletionPoll>),
}

/// Structural failure while joining already-proved process-local ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
enum ResidentProcessHostBuildError {
    /// Complete provider-route capacity could not be reserved.
    #[error("provider route ledger could not reserve its configured capacity")]
    ProviderRouteAllocation,
    /// Complete A2A route capacity could not be reserved.
    #[error("A2A delegate route ledger could not reserve its configured capacity")]
    A2aDelegateRouteAllocation,
    /// Complete binding-qualified A2A task-route capacity could not be reserved.
    #[error("A2A task route ledger could not reserve its configured capacity")]
    A2aTaskRouteAllocation,
    /// Per-deployment operation-recovery traversal state could not be reserved.
    #[error("operation recovery traversal ledger could not reserve the deployment set")]
    OperationRecoveryAttemptAllocation,
    /// Per-deployment Consult-recovery traversal state could not be reserved.
    #[error("Consult recovery traversal ledger could not reserve the deployment set")]
    ConsultRecoveryAttemptAllocation,
    /// Per-deployment Consult-child recovery traversal state could not be reserved.
    #[error("Consult child recovery traversal ledger could not reserve the deployment set")]
    ConsultChildRecoveryAttemptAllocation,
    /// Per-deployment Delegate-recovery traversal state could not be reserved.
    #[error("Delegate recovery traversal ledger could not reserve the deployment set")]
    DelegateRecoveryAttemptAllocation,
    /// Per-deployment publication traversal state could not be reserved.
    #[error("publication traversal ledger could not reserve the deployment set")]
    PublicationAttemptAllocation,
    /// The complete deployment set exceeds its explicit process bound.
    #[error("ready deployment set exceeds its explicit process bound")]
    DeploymentBoundExceeded,
    /// Two applications name the same exact physical store.
    #[error("ready deployment set repeats one exact store incarnation")]
    DuplicateDeploymentStore,
    /// Two ready applications name different incarnations of one deployment store.
    #[error("ready deployment set contains multiple incarnations of one store")]
    MultipleReadyIncarnations,
    /// Two stores claim the same complete admitted deployment authority.
    #[error("ready deployment set repeats one admitted deployment authority")]
    DuplicateDeploymentAuthority,
    /// A deployment association names a different principal from the writer.
    #[error("ready deployment does not belong to the admitted principal writer")]
    PrincipalAssociation,
    /// The retained runtime authority names a different store from the application.
    #[error("deployment runtime authority and resident application name different stores")]
    RuntimeAuthorityStoreAssociation,
    /// The applications do not equal the complete store set admitted by the writer.
    #[error("ready deployments do not equal the writer's opened store set")]
    OpenedStoreSet,
}

/// Failure while joining one operation journey to an exact deployment.
#[derive(Debug, thiserror::Error)]
enum ResidentProcessHostSelectionError {
    /// No ready application retains the complete requested authority association.
    #[error("the exact deployment-runtime authority is not ready in this process")]
    UnknownDeploymentAuthority,
    /// More than one ready application claims the same complete authority association.
    #[error("the deployment-runtime authority does not select one application")]
    AmbiguousDeploymentAuthority,
    /// The acquired operation ports claim a different physical store.
    #[error("operation access and selected application name different stores")]
    AccessStoreAssociation,
    /// Current publication evidence belongs to another deployment or principal.
    #[error("current publication authority does not name the selected deployment")]
    PublicationCurrentAuthority,
    /// A live A2A task route belongs to another binding or deployment selector.
    #[error("A2A task route does not match the selected deployment and binding")]
    A2aTaskRouteAssociation,
    /// The fresh caller request is not signed under the selected deployment authority.
    #[error("current operation-control request names a different deployment authority")]
    CurrentRequestAuthority,
    /// The recovered handle authority is not the selected deployment authority.
    #[error("historical operation handle names a different deployment authority")]
    HistoricalHandleAuthority,
    /// Protected recovery names a different physical store.
    #[error("operation recovery and selected application name different stores")]
    RecoveryStoreAssociation,
    /// Reconciliation does not name one root Invoke and its exact current head.
    #[error("reconciliation does not belong to the selected root Invoke head")]
    ReconciliationOperationAuthority,
    /// Retained request, session, or operation-lease evidence is unlike the handle.
    #[error("reconciliation history does not belong to the selected root Invoke")]
    ReconciliationHistoricalAuthority,
    /// Reconciliation observation or record signer names another deployment.
    #[error("reconciliation signer does not belong to the selected deployment")]
    ReconciliationSigningAuthority,
    /// Reconciliation checkpoint evidence names another deployment or profile.
    #[error("reconciliation checkpoint does not belong to the selected operation")]
    ReconciliationCheckpointAuthority,
    /// Reconciliation requires a positive physical record-read ceiling.
    #[error("reconciliation record bound is invalid")]
    ReconciliationBounds,
    /// Recovered pre-attempt Invoke state does not belong to the selected operation.
    #[error("Invoke resolution does not belong to the selected operation")]
    OperationResolutionOperationAuthority,
    /// Current session authority cannot continue this recovered Invoke.
    #[error("current session authority cannot continue Invoke resolution")]
    OperationResolutionCurrentAuthority,
    /// Historical session or lease authority differs from the recovered Invoke.
    #[error("historical session or lease does not belong to Invoke resolution")]
    OperationResolutionHistoricalAuthority,
    /// Request or observation signing authority differs from the selected deployment.
    #[error("Invoke resolution signer does not belong to the selected deployment")]
    OperationResolutionSigningAuthority,
    /// The recovered observation head differs from the selected deployment or time.
    #[error("Invoke resolution observation does not remain current and selected")]
    OperationResolutionObservationAuthority,
    /// Context checkpoint authority differs from the recovered operation domain.
    #[error("Invoke resolution checkpoint does not belong to the recovered operation")]
    OperationResolutionCheckpointAuthority,
    /// The recovery checkpoint authority names a different principal or store.
    #[error("checkpoint authority and selected application do not share one scope")]
    CheckpointAuthorityAssociation,
    /// The A2A continuation names another issued operation or physical store.
    #[error("A2A dispatch does not belong to the selected operation and store")]
    A2aOperationAssociation,
    /// Current A2A authority names another deployment.
    #[error("current A2A authority does not name the selected deployment")]
    A2aCurrentAuthority,
    /// The admitted A2A route was issued by another deployment authority.
    #[error("A2A route does not belong to the selected deployment authority")]
    A2aRouteAuthority,
    /// Process-local A2A deployment route custody differs from the selected deployment.
    #[error("process-local A2A deployment route does not match its selected deployment")]
    A2aProcessRouteAssociation,
    /// Current session heads or their signing witness name another deployment.
    #[error("current start authority does not name the selected deployment")]
    StartCurrentAuthority,
    /// The already-admitted session lease names another deployment authority.
    #[error("admitted start session does not name the selected deployment")]
    StartSessionAuthority,
    /// Request session or operation identity differs from admitted retained state.
    #[error("start request does not name the admitted session and compiled operation")]
    StartRequestAssociation,
    /// The operation-handle signing witness names another deployment authority.
    #[error("start handle signer does not name the selected deployment")]
    StartHandleAuthority,
    /// The handle issuer key is not the key selected by its signing witness.
    #[error("start handle issuer and signing witness name different keys")]
    StartHandleSigner,
    /// The unsigned operation-handle coordinates name another deployment.
    #[error("start handle preimage does not name the selected deployment")]
    StartHandlePreimageAuthority,
    /// The fresh allocation no longer matches its exact request, store, or limit.
    #[error("fresh start allocation does not match its selected request and store")]
    StartAllocationAssociation,
    /// Semantic selection and issuance supplied unlike checkpoint witnesses.
    #[error("start checkpoint witnesses are not the same exact authority")]
    StartCheckpointAuthority,
    /// The continuation belongs to another process operation owner.
    #[error("start custody belongs to another process operation owner")]
    StartProcessOwner,
    /// Current provider continuation authority names another deployment.
    #[error("provider continuation authority does not name the selected deployment")]
    ProviderCurrentAuthority,
    /// The issued provider operation belongs to another deployment authority.
    #[error("provider operation does not belong to the selected deployment")]
    ProviderOperationAuthority,
    /// A provider observation signer belongs to another deployment authority.
    #[error("provider observation signer does not belong to the selected deployment")]
    ProviderObservationAuthority,
    /// A provider checkpoint signer belongs to another principal or store.
    #[error("provider checkpoint signer does not belong to the selected deployment")]
    ProviderCheckpointAuthority,
    /// The process-local route capability belongs to another task owner.
    #[error("provider route belongs to another process task owner")]
    ProviderRouteOwner,
    /// The returned provider task does not match its retained route.
    #[error("provider task does not match the retained deployment route")]
    ProviderTaskAssociation,
    /// Historical session or dispatch evidence names another deployment.
    #[error("historical provider evidence names another deployment authority")]
    HistoricalProviderAuthority,
    /// The issued native root operation belongs to another deployment or class.
    #[error("native operation is not the selected deployment's root Invoke")]
    NativeOperationAuthority,
    /// Current native-operation session evidence names another deployment authority.
    #[error("current native-operation authority does not name the selected deployment")]
    NativeCurrentAuthority,
    /// Historical native-operation parents name another deployment authority.
    #[error("historical native-operation evidence names another deployment authority")]
    HistoricalNativeAuthority,
    /// A native-operation signer belongs to another deployment authority.
    #[error("native operation signer does not belong to the selected deployment")]
    NativeSigningAuthority,
    /// A native-operation checkpoint belongs to another principal or store.
    #[error("native operation checkpoint does not belong to the selected deployment")]
    NativeCheckpointAuthority,
    /// The recovered local cancellation does not belong to this root Invoke.
    #[error("local cancellation does not belong to the selected root Invoke")]
    LocalCancellationOperationAuthority,
    /// Current session authority no longer names the selected deployment.
    #[error("current local-cancellation authority does not name the selected deployment")]
    LocalCancellationCurrentAuthority,
    /// Historical session, lease, or generation evidence names another authority.
    #[error("historical local-cancellation evidence does not name the selected deployment")]
    LocalCancellationHistoricalAuthority,
    /// The retained cancellation head and pre-dispatch predecessor do not match.
    #[error("local cancellation history is not one exact pre-dispatch request")]
    LocalCancellationHistory,
    /// Original, cancellation, or successor signing authority differs.
    #[error("local cancellation signer does not belong to the selected deployment")]
    LocalCancellationSigningAuthority,
    /// Current checkpoint or its signer belongs to another principal or store.
    #[error("local cancellation checkpoint does not belong to the selected deployment")]
    LocalCancellationCheckpointAuthority,
    /// The retained compiled generation is unavailable from the selected application.
    #[error("local cancellation compiled generation is unavailable")]
    LocalCancellationGeneration,
    /// A finite local-cancellation recovery or projection bound is zero.
    #[error("local cancellation bounds are invalid")]
    LocalCancellationBounds,
    /// The expiry candidate is not this deployment's root Invoke.
    #[error("pre-dispatch expiry does not belong to the selected root Invoke")]
    PreDispatchExpiryOperationAuthority,
    /// Historical session or lease evidence names another authority.
    #[error("pre-dispatch expiry history does not belong to the selected deployment")]
    PreDispatchExpiryHistoricalAuthority,
    /// Request, observation, or successor signing authority differs.
    #[error("pre-dispatch expiry signer does not belong to the selected deployment")]
    PreDispatchExpirySigningAuthority,
    /// Checkpoint identity or authority differs from the selected store.
    #[error("pre-dispatch expiry checkpoint does not belong to the selected deployment")]
    PreDispatchExpiryCheckpointAuthority,
    /// The retained compiled generation is unavailable from the selected application.
    #[error("pre-dispatch expiry compiled generation is unavailable")]
    PreDispatchExpiryGeneration,
    /// Recovered consultation state names a different physical store.
    #[error("recovered consultation and selected application name different stores")]
    RecoveredCollectionStoreAssociation,
    /// Parent, child, or observation evidence belongs to another deployment.
    #[error("Consult terminal evidence does not belong to the selected deployment")]
    ConsultationOperationAuthority,
    /// The protected Consult parent belongs to another deployment or store.
    #[error("Consult parent does not belong to the selected deployment")]
    ConsultationParentAuthority,
    /// The child handle does not name the protected Consult parent exactly.
    #[error("Consult child does not name the protected parent")]
    ConsultationChildAssociation,
    /// The protected Delegate parent belongs to another deployment or store.
    #[error("Delegate parent does not belong to the selected deployment")]
    DelegateParentAuthority,
    /// The Delegate child does not name the protected parent exactly.
    #[error("Delegate child does not name the protected parent")]
    DelegateChildAssociation,
    /// The parent successor signer belongs to another deployment.
    #[error("Consult terminal signer does not belong to the selected deployment")]
    ConsultationSigningAuthority,
    /// Parent or successor checkpoint authority belongs to another deployment.
    #[error("Consult terminal checkpoint does not belong to the selected deployment")]
    ConsultationCheckpointAuthority,
}

/// Exact deployment fence around one unfinished durable local-operation start.
#[must_use = "continue the exact start phase through its retained deployment"]
struct ResidentProcessOperationStartContinuation {
    owner_instance: ResidentOperationTaskOwnerInstance,
    selector: ResidentDeploymentAuthoritySelector,
    maximum_record_bytes: usize,
    failure: Box<ResidentLocalOperationStartFailure>,
}

/// Exact retry state after a prepared semantic start reaches its physical owner.
#[must_use = "retry only the returned prepared parent or physical continuation"]
enum ResidentProcessPreparedOperationStartFailure {
    /// Process-owner or deployment selection failed before physical work began.
    Prepared {
        owner_instance: ResidentOperationTaskOwnerInstance,
        selector: Box<ResidentDeploymentAuthoritySelector>,
        prepared: Box<super::PreparedLocalOperationStart>,
    },
    /// Physical work began; the ordinary start continuation is now authoritative.
    Continuation(Box<ResidentProcessOperationStartContinuation>),
}

/// A signed Consult parent plus the exact declarative roster beneath it.
///
/// The public program must next admit each fresh child request in roster order.
/// This phase prevents that continuation from signing another parent or
/// reallocating an already materialized child prefix.
#[must_use = "admit and materialize this exact Consult roster"]
struct ResidentProcessConsultationRosterStartContinuation {
    owner_instance: ResidentOperationTaskOwnerInstance,
    selector: ResidentDeploymentAuthoritySelector,
    parent_checkpoint: ResidentProcessPreparedStartCheckpoint,
    pending:
        start_factories::ResidentConsultationRosterPending<ResidentProcessConsultFreshChildPolicy>,
}

/// Exact physical start inputs not retained by an ordinary continuation.
///
/// The checkpoint authority is intentionally absent. The later continuation
/// must resolve and revalidate it from the selected deployment before use.
struct ResidentProcessPreparedStartCheckpoint {
    accumulator_limits: pale_ports::AccumulatorPreparationLimits,
    checkpoint_id: ControlCheckpointIdV1,
    checkpoint_ref: ResidentRecordRefV1,
    checkpoint_schema: SchemaDigest,
    maximum_record_bytes: usize,
}

impl ResidentProcessConsultationRosterStartContinuation {
    /// Split the deployment fence from the exact parent-first roster phase.
    fn into_parts(
        self,
    ) -> (
        ResidentOperationTaskOwnerInstance,
        ResidentDeploymentAuthoritySelector,
        ResidentProcessPreparedStartCheckpoint,
        start_factories::ResidentConsultationRosterPending<ResidentProcessConsultFreshChildPolicy>,
    ) {
        (
            self.owner_instance,
            self.selector,
            self.parent_checkpoint,
            self.pending,
        )
    }
}

/// Internal split between deployment selection and the existing start owner.
enum EitherProcessInvokeStartFailure {
    Selection(ResidentProcessHostSelectionError),
    Start(ResidentLocalOperationStartFailure),
}

/// Internal split between retained Consult input construction and start phases.
enum EitherProcessConsultStartFailure {
    Selection(ResidentProcessHostSelectionError),
    Factory(start_factories::ResidentOperationStartFactoryError),
    Start(ResidentLocalOperationStartFailure),
}

/// Failure before an admitted Consult parent or child reached durable custody.
#[must_use = "preserve any returned start-phase continuation"]
enum ResidentProcessConsultStartError {
    /// Deployment, store, parent, or child selection failed before port I/O.
    Selection(ResidentProcessHostSelectionError),
    /// Semantic admission or signing stopped before a physical candidate existed.
    Preparation(super::LocalOperationStartError),
    /// The exact retained start phase owns the only safe continuation.
    Continuation(Box<ResidentProcessOperationStartContinuation>),
}

/// Failure before a pairwise Delegate child reached durable custody.
#[must_use = "preserve any returned start-phase continuation"]
enum ResidentProcessDelegateStartError {
    /// Deployment, store, parent, or child selection failed before port I/O.
    Selection(ResidentProcessHostSelectionError),
    /// Semantic admission or signing stopped before a physical candidate existed.
    Preparation(super::LocalOperationStartError),
    /// The exact retained start phase owns the only safe continuation.
    Continuation(Box<ResidentProcessOperationStartContinuation>),
}

impl From<ResidentProcessHostSelectionError> for ResidentProcessDelegateStartError {
    fn from(reason: ResidentProcessHostSelectionError) -> Self {
        Self::Selection(reason)
    }
}

impl From<ResidentProcessHostSelectionError> for ResidentProcessConsultStartError {
    fn from(reason: ResidentProcessHostSelectionError) -> Self {
        Self::Selection(reason)
    }
}

/// Exact deployment fence around an unfinished initial Consult collection.
#[must_use = "continue the exact collection custody phase through its deployment"]
struct ResidentProcessConsultationOpenCustody {
    selector: ResidentDeploymentAuthoritySelector,
    failure: ConsultationCollectionCommitFailure,
}

/// Failure before an initial Consult collection reached complete custody.
#[must_use = "preserve any returned collection custody continuation"]
enum ResidentProcessConsultationOpenError {
    /// Deployment, store, parent, signer, or checkpoint selection failed.
    Selection(ResidentProcessHostSelectionError),
    /// Collection meaning or the resolving-parent transition did not admit.
    Admission(ConsultationCollectionError),
    /// The signed collection retains the only safe physical continuation.
    Custody(Box<ResidentProcessConsultationOpenCustody>),
}

impl From<ResidentProcessHostSelectionError> for ResidentProcessConsultationOpenError {
    fn from(reason: ResidentProcessHostSelectionError) -> Self {
        Self::Selection(reason)
    }
}

/// Why initial Consult collection custody did not continue.
enum ResidentProcessConsultationOpenContinuationError {
    /// The retained deployment or supplied store did not select one owner.
    Selection(ResidentProcessHostSelectionError),
    /// The lower custody owner did not prove a safe completion.
    Custody,
}

/// Retained collection custody plus the reason it could not advance.
#[must_use = "retry only the returned exact collection continuation"]
struct ResidentProcessConsultationOpenContinuationFailure {
    pending: Box<ResidentProcessConsultationOpenCustody>,
    reason: ResidentProcessConsultationOpenContinuationError,
}

/// Failure while deriving and retaining the first finite Consult batch.
enum ResidentProcessConsultationBatchOpenError {
    /// Deployment, parent, or store selection did not match.
    Selection(ResidentProcessHostSelectionError),
    /// The recovered collection could not produce a current batch.
    Planning(ConsultationCollectionError),
    /// The process-local bounded task owner refused the batch.
    Open(ResidentConsultationBatchOpenError),
}

/// Failure before one selected Consult collection completed protected recovery.
enum ResidentProcessConsultationCollectionRecoveryError {
    /// Deployment, parent, protected head, or store selection did not match.
    Selection(ResidentProcessHostSelectionError),
    /// Existing collection recovery rejected stored or currently admitted meaning.
    Recovery(ConsultationCollectionError),
}

impl From<ResidentProcessHostSelectionError> for ResidentProcessConsultationBatchOpenError {
    fn from(reason: ResidentProcessHostSelectionError) -> Self {
        Self::Selection(reason)
    }
}

/// Failure before an allocated Invoke reached complete durable custody.
#[must_use = "preserve the returned retained lease or start-phase continuation"]
enum ResidentProcessAllocatedInvokeStartError {
    /// Deployment selection failed before start authority or custody access.
    Selection {
        pending: Box<ResidentProcessRetainedOperationLease>,
        reason: ResidentProcessHostSelectionError,
    },
    /// An embedded start schema could not be resolved.
    Contract {
        pending: Box<ResidentProcessRetainedOperationLease>,
        reason: pale_contracts::SchemaError,
    },
    /// Semantic admission or signing stopped; no blanket retry is authorized.
    Preparation {
        pending: Box<ResidentProcessRetainedOperationLease>,
        reason: super::LocalOperationStartError,
    },
    /// The retained exact start phase owns the only allowed continuation.
    Continuation(Box<ResidentProcessOperationStartContinuation>),
}

impl std::fmt::Debug for ResidentProcessOperationStartContinuation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProcessOperationStartContinuation")
            .field("owner_instance", &self.owner_instance)
            .field("selector", &self.selector)
            .field("maximum_record_bytes", &self.maximum_record_bytes)
            .field("failure", &self.failure)
            .finish()
    }
}

/// Why an exact process-host operation-start continuation remains unfinished.
#[derive(Debug)]
enum ResidentProcessOperationStartContinuationError {
    /// Embedded deployment or supplied access did not select one exact owner.
    Selection(ResidentProcessHostSelectionError),
    /// The selected lower start phase remains incomplete.
    Phase,
}

/// Retained selector-bound start phase plus its latest continuation failure.
#[must_use = "retry only the returned exact start continuation"]
struct ResidentProcessOperationStartContinuationFailure {
    pending: Box<ResidentProcessOperationStartContinuation>,
    reason: ResidentProcessOperationStartContinuationError,
}

/// Failure before or during private process-host observation composition.
#[derive(Debug, thiserror::Error)]
enum ResidentProcessObservationError {
    /// Exact deployment, authority, recovery, or access association failed.
    #[error(transparent)]
    Selection(#[from] ResidentProcessHostSelectionError),
    /// The existing application-owned Observe use case rejected the operation.
    #[error(transparent)]
    Observe(#[from] LocalOperationObservationError),
}

/// Cancellation outcome after joining process-owned deployment custody.
#[must_use = "retain A2A uncertainty before releasing its route or capacity"]
enum ResidentProcessCancellationOutcome {
    /// No A2A route changed ownership; preserve the application disposition.
    Operation(ResidentOperationCancellationOutcome),
    /// Durable Cancel stopped one local response read and checked out its route.
    A2aDelegateReadStopped {
        /// Completed physical custody for the cancellation request.
        custody: CompletedOperationObservationCustody,
        /// Exact independently anchored request checkpoint.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        /// Deployment-qualified uncertainty holding the route and capacity.
        continuation: ResidentProcessA2aDelegatePoll,
    },
}

/// Failure before or during private process-host cancellation composition.
enum ResidentProcessCancellationError {
    /// Exact deployment, authority, recovery, or access association failed.
    Selection(ResidentProcessHostSelectionError),
    /// The existing application-owned Cancel use case rejected the operation.
    Cancellation(ResidentOperationCancellationFailure),
    /// A pre-dispatch cancellation append owns an exact physical candidate.
    Custody(Box<ResidentProcessCancellationCustody>),
    /// A dispatched cancellation append exists under one exact deployment.
    DispatchedCustody(Box<ResidentProcessDispatchedCancellationCustody>),
}

fn bind_cancellation_custody(
    selector: &ResidentDeploymentAuthoritySelector,
    outcome: Result<ResidentOperationCancellationOutcome, ResidentOperationCancellationFailure>,
) -> Result<ResidentOperationCancellationOutcome, ResidentProcessCancellationError> {
    match outcome {
        Err(ResidentOperationCancellationFailure::Custody(
            pending @ (OperationAppendFailure::Commit { .. }
            | OperationAppendFailure::Anchor { .. }),
        )) => Err(ResidentProcessCancellationError::Custody(Box::new(
            ResidentProcessCancellationCustody {
                selector: selector.clone(),
                pending,
            },
        ))),
        Err(ResidentOperationCancellationFailure::DispatchedCustody(pending)) => {
            Err(ResidentProcessCancellationError::DispatchedCustody(
                Box::new(ResidentProcessDispatchedCancellationCustody {
                    selector: selector.clone(),
                    pending,
                }),
            ))
        }
        Err(reason) => Err(ResidentProcessCancellationError::Cancellation(reason)),
        Ok(outcome) => Ok(outcome),
    }
}

/// Exact deployment fence around one retained pre-dispatch cancellation append.
#[must_use = "continue the retained candidate through its original deployment"]
struct ResidentProcessCancellationCustody {
    selector: ResidentDeploymentAuthoritySelector,
    pending: OperationAppendFailure,
}

/// Why a selector-bound pre-dispatch continuation remains unfinished.
enum ResidentProcessCancellationContinuationError {
    Selection(ResidentProcessHostSelectionError),
    Custody,
}

/// Linear pre-dispatch cancellation continuation plus its latest failure.
#[must_use = "retry only the returned exact continuation"]
struct ResidentProcessCancellationContinuationFailure {
    pending: Box<ResidentProcessCancellationCustody>,
    reason: ResidentProcessCancellationContinuationError,
}

impl From<ResidentProcessHostSelectionError> for ResidentProcessCancellationError {
    fn from(failure: ResidentProcessHostSelectionError) -> Self {
        Self::Selection(failure)
    }
}

/// Exact deployment fence around one retained dispatched-cancellation append.
#[must_use = "continue the retained candidate through its original deployment"]
struct ResidentProcessDispatchedCancellationCustody {
    selector: ResidentDeploymentAuthoritySelector,
    pending: Box<ResidentDispatchedCancellationCustody>,
}

impl std::fmt::Debug for ResidentProcessDispatchedCancellationCustody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProcessDispatchedCancellationCustody")
            .field("selector", &self.selector)
            .field("pending", &self.pending)
            .finish()
    }
}

/// Why a selector-bound dispatched-cancellation continuation remains unfinished.
#[derive(Debug)]
enum ResidentProcessDispatchedCancellationContinuationError {
    /// The embedded deployment or supplied access did not select one exact owner.
    Selection(ResidentProcessHostSelectionError),
    /// The exact retained physical candidate still lacks complete custody.
    Custody,
}

/// Linear dispatched-cancellation continuation plus its latest failure.
#[must_use = "retry only the returned exact continuation"]
struct ResidentProcessDispatchedCancellationContinuationFailure {
    pending: Box<ResidentProcessDispatchedCancellationCustody>,
    reason: ResidentProcessDispatchedCancellationContinuationError,
}

/// Exact deployment fence around one retained consultation-cancellation append.
#[must_use = "continue the retained candidate through its original deployment"]
struct ResidentProcessConsultationCancellationCustody {
    selector: ResidentDeploymentAuthoritySelector,
    pending: Box<ResidentConsultationCancellationCustody>,
}

/// Exact deployment fence around a pre-write Consult cancellation owner.
#[must_use = "preserve the exact preparation or issue continuation"]
struct ResidentProcessConsultationCancellationPreparation {
    selector: ResidentDeploymentAuthoritySelector,
    pending: Box<ConsultationCollectionCommitFailure>,
}

impl std::fmt::Debug for ResidentProcessConsultationCancellationCustody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProcessConsultationCancellationCustody")
            .field("selector", &self.selector)
            .field("pending", &self.pending)
            .finish()
    }
}

/// Failure before or during one consultation-specific cancellation request.
#[must_use = "preserve any returned custody continuation"]
enum ResidentProcessConsultationCancellationError {
    /// Exact deployment, authority, recovery, checkpoint, or access selection failed.
    Selection(ResidentProcessHostSelectionError),
    /// Protected recovery, current authority, or consultation admission failed.
    Cancellation(ResidentConsultationCancellationRequestFailure),
    /// Preparation or issuance stopped with its signed material still owned.
    Preparation(Box<ResidentProcessConsultationCancellationPreparation>),
    /// A physical append exists and must continue under its embedded deployment.
    Custody(Box<ResidentProcessConsultationCancellationCustody>),
}

impl From<ResidentProcessHostSelectionError> for ResidentProcessConsultationCancellationError {
    fn from(reason: ResidentProcessHostSelectionError) -> Self {
        Self::Selection(reason)
    }
}

/// Classification retained beside an unfinished process-host continuation.
#[derive(Debug)]
enum ResidentProcessConsultationCancellationContinuationError {
    /// The embedded deployment or supplied access did not select one exact owner.
    Selection(ResidentProcessHostSelectionError),
    /// The exact retained physical candidate still lacks complete custody.
    Custody,
}

/// Linear continuation plus the reason it could not yet complete.
#[must_use = "retry only the returned exact continuation"]
struct ResidentProcessConsultationCancellationContinuationFailure {
    pending: Box<ResidentProcessConsultationCancellationCustody>,
    reason: ResidentProcessConsultationCancellationContinuationError,
}

/// Exact pre-write owner plus the reason it remains deferred.
#[must_use = "retry only the returned exact continuation"]
struct ResidentProcessConsultationCancellationPreparationContinuationFailure {
    pending: Box<ResidentProcessConsultationCancellationPreparation>,
    reason: ResidentProcessConsultationCancellationContinuationError,
}

/// One selected Consult stop bound to the deployment that owns its collection.
#[must_use = "commit this exact stop through its retained deployment"]
struct ResidentProcessConsultationStopSelected<'operation> {
    selector: ResidentDeploymentAuthoritySelector,
    selected: Box<ResidentConsultationStopSelected<ConsultationStopInput<'operation>>>,
}

/// A selector refusal that preserves the unbound stop unchanged.
#[must_use = "retry deployment binding with the returned selected stop"]
struct ResidentProcessConsultationStopBindingFailure<'operation> {
    selector: ResidentDeploymentAuthoritySelector,
    selected: Box<ResidentConsultationStopSelected<ConsultationStopInput<'operation>>>,
    reason: ResidentProcessHostSelectionError,
}

/// A retained stop append bound to its original deployment and store.
#[must_use = "continue this exact stop append through its retained deployment"]
struct ResidentProcessConsultationStopCustody {
    selector: ResidentDeploymentAuthoritySelector,
    store: AuthorityStoreCoordinate,
    pending: Box<ResidentConsultationStopCustody>,
}

/// Exact phase returned when a process-host Consult stop cannot commit.
#[must_use = "preserve and continue the exact returned linear phase"]
enum ResidentProcessConsultationStopCommitFailure<'operation> {
    /// Deployment or authority selection rejected before custody I/O.
    Selection {
        selected: Box<ResidentProcessConsultationStopSelected<'operation>>,
        reason: ResidentProcessHostSelectionError,
    },
    /// Stop or collection admission rejected before custody I/O.
    Admission {
        selected: Box<ResidentProcessConsultationStopSelected<'operation>>,
        reason: ConsultationCollectionError,
    },
    /// Database or independent-anchor custody remains incomplete.
    Custody(Box<ResidentProcessConsultationStopCustody>),
}

/// Why an exact selected-stop continuation remains unfinished.
enum ResidentProcessConsultationStopContinuationError {
    /// The embedded deployment or supplied store did not select one owner.
    Selection(ResidentProcessHostSelectionError),
    /// The lower custody owner did not prove a safe completion.
    Custody,
}

/// Retained stop custody plus the reason it could not advance.
#[must_use = "retry only the returned exact stop continuation"]
struct ResidentProcessConsultationStopContinuationFailure {
    pending: Box<ResidentProcessConsultationStopCustody>,
    reason: ResidentProcessConsultationStopContinuationError,
}

/// A retained terminal append bound to its original deployment.
#[must_use = "continue this exact terminal append through its retained deployment"]
struct ResidentProcessConsultationTerminalCustody<Work, Output> {
    selector: ResidentDeploymentAuthoritySelector,
    pending: Box<ResidentConsultationTerminalCustody<Work, Output>>,
}

impl<Work, Output> std::fmt::Debug for ResidentProcessConsultationTerminalCustody<Work, Output> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProcessConsultationTerminalCustody")
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

/// A completed terminal append whose aggregate capacity still needs release.
#[must_use = "retry only aggregate release through the retained deployment"]
struct ResidentProcessConsultationJoinedTerminal<Work, Output> {
    selector: ResidentDeploymentAuthoritySelector,
    joined: Box<ResidentConsultationJoinedTerminal<Work, Output>>,
}

/// Exact phase returned when a process-host Consult terminal join stops.
#[must_use = "preserve and continue the exact returned linear phase"]
enum ResidentProcessConsultationTerminalJoinFailure<Work, Output> {
    /// Deployment or authority selection rejected before custody I/O.
    Selection {
        ready: Box<ResidentProcessConsultationCompletionReady<Work, Output>>,
        reason: ResidentProcessHostSelectionError,
    },
    /// Lower terminal preflight rejected the exact ready completion.
    Preflight {
        ready: Box<ResidentProcessConsultationCompletionReady<Work, Output>>,
        reason: ResidentConsultationTerminalJoinPreflightFailure,
    },
    /// Evidence or collection admission rejected before custody I/O.
    Admission {
        ready: Box<ResidentProcessConsultationCompletionReady<Work, Output>>,
        reason: ConsultationCollectionError,
    },
    /// Database or anchor custody remains incomplete.
    Custody(Box<ResidentProcessConsultationTerminalCustody<Work, Output>>),
    /// A replacement task owner rejected custody continuation.
    CustodyPreflight {
        pending: Box<ResidentProcessConsultationTerminalCustody<Work, Output>>,
        reason: ResidentConsultationTerminalJoinPreflightFailure,
    },
    /// Deployment selection rejected before custody continuation.
    CustodySelection {
        pending: Box<ResidentProcessConsultationTerminalCustody<Work, Output>>,
        reason: ResidentProcessHostSelectionError,
    },
    /// Parent custody completed, but aggregate capacity did not release.
    Release {
        joined: Box<ResidentProcessConsultationJoinedTerminal<Work, Output>>,
        reason: ResidentConsultationTerminalReleaseFailure,
    },
    /// A replacement task owner rejected aggregate-release retry.
    ReleasePreflight {
        joined: Box<ResidentProcessConsultationJoinedTerminal<Work, Output>>,
        reason: ResidentConsultationTerminalJoinPreflightFailure,
    },
    /// Deployment selection rejected before aggregate-release retry.
    ReleaseSelection {
        joined: Box<ResidentProcessConsultationJoinedTerminal<Work, Output>>,
        reason: ResidentProcessHostSelectionError,
    },
}

fn map_process_consultation_terminal_failure<Work, Output>(
    selector: ResidentDeploymentAuthoritySelector,
    failure: ResidentConsultationTerminalJoinFailure<Work, Output>,
) -> ResidentProcessConsultationTerminalJoinFailure<Work, Output> {
    match failure {
        ResidentConsultationTerminalJoinFailure::Preflight { ready, reason } => {
            ResidentProcessConsultationTerminalJoinFailure::Preflight {
                ready: Box::new(ResidentProcessConsultationCompletionReady { selector, ready }),
                reason,
            }
        }
        ResidentConsultationTerminalJoinFailure::Admission { ready, reason } => {
            ResidentProcessConsultationTerminalJoinFailure::Admission {
                ready: Box::new(ResidentProcessConsultationCompletionReady { selector, ready }),
                reason,
            }
        }
        ResidentConsultationTerminalJoinFailure::Custody(pending) => {
            ResidentProcessConsultationTerminalJoinFailure::Custody(Box::new(
                ResidentProcessConsultationTerminalCustody { selector, pending },
            ))
        }
        ResidentConsultationTerminalJoinFailure::CustodyPreflight { pending, reason } => {
            ResidentProcessConsultationTerminalJoinFailure::CustodyPreflight {
                pending: Box::new(ResidentProcessConsultationTerminalCustody { selector, pending }),
                reason,
            }
        }
        ResidentConsultationTerminalJoinFailure::Release { joined, reason } => {
            ResidentProcessConsultationTerminalJoinFailure::Release {
                joined: Box::new(ResidentProcessConsultationJoinedTerminal { selector, joined }),
                reason,
            }
        }
        ResidentConsultationTerminalJoinFailure::ReleasePreflight { joined, reason } => {
            ResidentProcessConsultationTerminalJoinFailure::ReleasePreflight {
                joined: Box::new(ResidentProcessConsultationJoinedTerminal { selector, joined }),
                reason,
            }
        }
    }
}

/// Failure while joining admitted deployment association to recovered state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
enum ResidentDeploymentJoinError {
    /// The association differs from the application's recovered store.
    #[error("deployment association attempted to substitute the application's store coordinate")]
    StoreSubstitution,
}

#[cfg(test)]
#[path = "../../tests/internal/resident/process_host.rs"]
pub(in crate::resident) mod tests;
