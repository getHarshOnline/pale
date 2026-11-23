//! The resident's state: which generation of the configuration is live, which
//! processes it should run, and how that state survives a restart.
//!
//! This module builds mainly on `compile`, `composition`, `hooks`,
//! `operation_admission`, `packages`, `plan`, `render`, `semantic` and
//! `workspace`; `decision_evidence`, `failure`, `observation` and
//! `operation_admission` build on it. The resident keeps its records behind a
//! neutral durable-custody boundary.
//!
//! The application folds generation lifecycle and explicit supervisor desired
//! state as independent projections of one product state. It admits immutable
//! generation bodies, reconstructs that authority after restart, and exposes
//! process intents only after their desired state is proven durable. Storage
//! selection, native process control, provider admission, and runtime effects
//! remain outside this module.
//!
//! Local deployment administration follows a separate typed custody path. An
//! authenticated [`DeploymentFinalizationSubmission`] can become a signed
//! [`SignedDeploymentFinalizationInput`], then move through principal-store commit,
//! independent anchors, deployment-store genesis, and its domain anchors. The
//! caller supplies the authenticated records, custody ports, signature policy,
//! and anchor mechanism. These phases select none of them and expose no direct
//! database access to an SDK or interface.
//!
//! Mathematical invariants remain with their enforcing owners: durable state
//! transition and replay with [`ResidentApplication`], immutable definition
//! alignment with the private definition store, publication ordering with
//! [`GenerationLifecycle`], and custody/reconstruction laws with their typed
//! boundaries below.

#[allow(
    dead_code,
    reason = "the private A2A CancelTask owner awaits assembled resident attachment"
)]
mod a2a_cancel_task_owner;
#[allow(
    dead_code,
    reason = "the private A2A task owner awaits assembled resident attachment"
)]
mod a2a_delegate_task_service;
#[allow(
    dead_code,
    reason = "the private A2A GetTask owner awaits assembled resident attachment"
)]
mod a2a_get_task_owner;
mod a2a_task_custody;
pub use a2a_task_custody::{
    admit_a2a_get_task_evidence, commit_a2a_get_task_readback, commit_a2a_initial_task_response,
    discover_a2a_task_bindings, prepare_a2a_cancel_task_custody, prepare_a2a_task_binding_custody,
    prepare_a2a_task_observation_custody, recover_a2a_task_history, start_a2a_cancel_task,
    start_a2a_get_task_readback, A2aCancelTaskAuthorizationContext,
    A2aCancelTaskDispositionFailure, A2aCancelTaskDispositionInput, A2aCancelTaskError,
    A2aCancelTaskInput, A2aCancelTaskNotStarted, A2aCancelTaskObservationInput,
    A2aCancelTaskOutcomeUnknown, A2aCancelTaskPoll, A2aCancelTaskStartFailure,
    A2aCancelTaskUncertainty, A2aGetTaskCommitFailure, A2aGetTaskCommitInput,
    A2aGetTaskObservationInput, A2aGetTaskReadbackAuthorizationContext, A2aGetTaskReadbackError,
    A2aGetTaskReadbackInput, A2aGetTaskReadbackPoll, A2aInitialTaskCommitFailure,
    A2aInitialTaskCommitInput, A2aRouteAssociation, A2aTaskBindingCustodyInput,
    A2aTaskCustodyError, A2aTaskObservationCustodyInput, A2aTaskObservationSelection,
    A2aTaskRecoveryAdmission, A2aTaskRecoveryError, A2aTaskRecoveryInput, A2aTaskRecoveryLimits,
    CompletedA2aCancelTaskReadback, CompletedA2aGetTaskReadback,
    CompletedA2aGetTaskReadbackCustody, CompletedA2aInitialTaskCustody,
    DatabaseCommittedA2aCancelTaskAnchorPending, DiscoveredA2aTaskBinding,
    DiscoveredA2aTaskBindings, InFlightA2aCancelTask, InFlightA2aGetTaskReadback,
    PreparedA2aCancelTaskCustody, PreparedA2aCancelTaskDisposition, PreparedA2aTaskAppend,
    PreparedA2aTaskCheckpoint, ReceivedA2aCancelTaskResponse, ReceivedA2aGetTaskReadback,
    RecoveredA2aTaskHistory, RetainedA2aCancelTask,
};
pub(in crate::resident) use a2a_task_custody::{
    commit_a2a_get_task_readback_continuable, commit_a2a_initial_task_response_continuable,
    ContinuableA2aGetTaskCommitFailure, ContinuableA2aInitialTaskCommitFailure,
};
mod anchor_progress;
mod application;
mod authority;
mod bootstrap;
mod checkpoint_profile;
mod checkpoint_recovery;
pub use checkpoint_profile::{
    OperationCheckpointProfileError, OperationCheckpointSelection,
    SupportedOperationCheckpointProfile,
};
pub use checkpoint_recovery::OperationCheckpointAncestryLimits;
#[allow(
    dead_code,
    reason = "the private cold-start classifier awaits assembled resident cold-open"
)]
mod cold_start;
mod concurrency;
mod coordination;
mod custody_phase;
pub use custody_phase::{AnchorAdvanceEvidence, AnchorAdvanceFailureReason};
mod definition;
mod deployment_finalization;
#[allow(
    dead_code,
    reason = "deployment cold-start admission awaits assembled resident cold-open"
)]
mod deployment_finalization_cold_start;
mod deployment_initialization;
pub use deployment_finalization::{
    AdmittedDeploymentFinalization, DeploymentFinalizationCandidate, DeploymentFinalizationError,
    DeploymentFinalizationSubmission,
};
pub use deployment_initialization::{
    AnchoredDeploymentFinalization, CompletedDeploymentInitialization,
    DatabaseCommittedDeploymentFinalizationAnchorsPending,
    DeploymentFinalizationAnchorPhaseFailure, DeploymentFinalizationCommitPhaseFailure,
    DeploymentFinalizationPreparationFailure, DeploymentFinalizationPreparationFailureReason,
    DeploymentFinalizationReceiptParameters, DeploymentGenesisAnchorPhaseFailure,
    DeploymentGenesisAnchorPhaseFailureReason, DeploymentGenesisAnchorsPending,
    DeploymentGenesisCommitPhaseFailure, DeploymentGenesisPreparationFailure,
    DeploymentGenesisPreparationFailureReason, PreparedDeploymentFinalization,
    PreparedDeploymentGenesis, SignedDeploymentFinalizationInput,
    SignedDeploymentGenesisCheckpoint, SignedDeploymentGenesisInput,
};
mod consultation;
mod effect_case;
mod event_custody;
mod generation;
mod hook;
mod journal;
mod lifecycle;
#[allow(
    dead_code,
    reason = "some private local-operation recovery continuations are not yet composed"
)]
mod local_operation_coordinator;
mod local_operation_start;
mod native_request;
mod observation_checkpoint;
mod operation_control;
mod operation_custody;
mod operation_dispatch;
mod operation_expiry;
pub use operation_expiry::recover_committed_pre_dispatch_expiry;
#[allow(
    dead_code,
    reason = "ResidentProgram assembles this private owner; native listener composition remains external"
)]
mod operation_host;
mod operation_lease_custody;
mod operation_observation_custody;
#[allow(
    dead_code,
    reason = "ResidentProgram reaches this private service through the process host; no public application attachment is exposed"
)]
mod operation_service;
#[allow(
    dead_code,
    reason = "the private operation-start allocator awaits the assembled resident attachment"
)]
mod operation_start_allocation;
mod principal_checkpoint;
#[allow(
    dead_code,
    reason = "cold-start composition has not yet consumed admitted principal control lineage"
)]
mod principal_control_checkpoint_recovery;
#[allow(
    dead_code,
    reason = "the private trust-frontier closure awaits assembled resident cold-open"
)]
mod principal_trust_frontier;
#[allow(
    dead_code,
    reason = "cold-start composition has not yet consumed admitted trust lineage"
)]
mod principal_trust_recovery;
#[allow(
    dead_code,
    reason = "ResidentProgram assembles this private host from explicit writer and deployment readiness evidence"
)]
mod process_host;
mod process_reconciliation;
mod process_recovery;
#[allow(
    dead_code,
    reason = "the private provider task service awaits the assembled resident host"
)]
mod provider_task_service;
pub use principal_checkpoint::{
    ConstructedDeploymentFinalization, PreparedDeploymentFinalizationCheckpoint,
    PreparedPrincipalBootstrapCheckpoint, PreparedPrincipalBootstrapFrontier,
    PrincipalCheckpointError, SupportedPrincipalTrustCheckpointProfile,
};
mod producer;
mod reconstruction;
mod record_projection;
mod service_policy;
mod service_recovery;
mod session_admission;
mod session_lease_custody;
#[allow(
    dead_code,
    reason = "ResidentProgram retains this private task owner through ResidentOperationHost"
)]
mod task_owner;
mod task_reservation;
mod trust_transition;
mod working_set;
mod writer;
pub use process_host::frame_service::{
    ResidentFrameOutcome, ResidentFrameService, ResidentFrameServiceError,
};
#[cfg(feature = "interface-test-support")]
pub use process_host::interface_conformance::{
    exercise_consult_child_ambiguous_restart, exercise_consult_child_collection_anchor_restart,
    exercise_consult_child_prefix_service_restart, exercise_consult_child_service_restart,
    exercise_consult_child_suffix_service_restart, exercise_consult_service_expiry_restart,
    exercise_consult_service_restart, with_in_memory_consult_conformance_frontier,
    with_in_memory_invoke_conformance_frontier,
    with_in_memory_invoke_conformance_frontier_for_peer, ConsultConformanceActivity,
    OperationConformanceFrontierError, OperationConformanceRequestFactory,
};
pub use process_host::service_assembly::{
    ResidentPeerAdmissionPort, ResidentPeerAdmissionWitness,
    ResidentServiceA2aCancelRecoveryInputs, ResidentServiceA2aDelegateLaunchInputs,
    ResidentServiceA2aDelegateLaunchPlan, ResidentServiceA2aPreTaskRecoveryInputs,
    ResidentServiceA2aTaskObservationInputs, ResidentServiceAssemblyError,
    ResidentServiceCallerSignaturePort, ResidentServiceCancellationAllocation,
    ResidentServiceCancellationInputPort, ResidentServiceCancellationInputs,
    ResidentServiceCapabilityRevocationEvidence, ResidentServiceCheckpointSigner,
    ResidentServiceConsultCompletionContext, ResidentServiceConsultControlAssociation,
    ResidentServiceConsultDecision, ResidentServiceConsultExecutor,
    ResidentServiceConsultInitialCollectionInputs, ResidentServiceConsultInputPort,
    ResidentServiceConsultJoinInputs, ResidentServiceConsultJoinSelection,
    ResidentServiceConsultPolicy, ResidentServiceConsultPolicyContext,
    ResidentServiceConsultPolicyPort, ResidentServiceConsultStopInput,
    ResidentServiceConsultStopInputs, ResidentServiceConsultStopMeaning,
    ResidentServiceConsultTask, ResidentServiceConsultTaskCompletion,
    ResidentServiceConsultTaskExecutor, ResidentServiceConsultTaskLimits,
    ResidentServiceConsultTerminalChildInputs, ResidentServiceConsultationRosterPolicy,
    ResidentServiceConsultationRosterSlot, ResidentServiceConsultationRosterSource,
    ResidentServiceCurrentRequestEvidence, ResidentServiceCurrentRequestEvidencePort,
    ResidentServiceCurrentRequestSelector, ResidentServiceCurrentWriterAdmissionPort,
    ResidentServiceCurrentWriterEvidence, ResidentServiceDelegateInputPort,
    ResidentServiceDelegatePolicyContext, ResidentServiceDelegatePolicyPort,
    ResidentServiceDeploymentOpen, ResidentServiceDeploymentRevocationEvidence,
    ResidentServiceDeploymentRevocationHeadEvidence, ResidentServiceDeploymentSelector,
    ResidentServiceGrantEvidence, ResidentServiceGrantRevocationEvidence,
    ResidentServiceInvokeBackgroundRecoveryInputs,
    ResidentServiceInvokeConsultDelegateObserveCancelOwners,
    ResidentServiceInvokeConsultObserveCancelOwners, ResidentServiceInvokeCryptographyPort,
    ResidentServiceInvokeInputPort, ResidentServiceInvokeObserveCancelOwners,
    ResidentServiceInvokePolicyContext, ResidentServiceInvokePolicyPort,
    ResidentServiceInvokeRecoveryInputs, ResidentServiceInvokeRecoveryLimits,
    ResidentServiceLocalCancellationCompletionAllocation, ResidentServiceNativePreparedInputs,
    ResidentServiceNativeRecoveryInputs, ResidentServiceObservationSigner,
    ResidentServiceObserveInputPort, ResidentServiceObserveInputs, ResidentServiceObserveOwners,
    ResidentServiceObserveReplayAdmission, ResidentServiceObserveReplayDisposition,
    ResidentServiceObserveReplayError, ResidentServiceOneShotCheckpointSigner,
    ResidentServiceOneShotObservationSigner, ResidentServiceOneShotRotationAdmission,
    ResidentServiceOperationLeasePolicy, ResidentServiceOperationRequestPolicies,
    ResidentServiceOperationResolutionInputs, ResidentServiceOperationResolutionRoute,
    ResidentServiceOperationStartPolicy, ResidentServicePreDispatchExpiryContinuationInputs,
    ResidentServicePreDispatchExpiryPreparationInputs, ResidentServiceProcessInput,
    ResidentServiceProviderPreparedInputs, ResidentServiceProviderRecoveryInputs,
    ResidentServiceProviderResolvedInputs, ResidentServiceProviderTask,
    ResidentServiceProviderTaskCompletion, ResidentServiceProviderTaskExecutor,
    ResidentServicePublicationCustodyPort, ResidentServicePublicationInputPort,
    ResidentServicePublicationInputs, ResidentServiceReconciliationAdmission,
    ResidentServiceResultSigner, ResidentServiceRotationAdmission,
    ResidentServiceSessionRevocationEvidence, ResidentServiceTaskLimits,
    ResidentServiceTransportAdmission,
};
pub use process_host::work_turn::{
    ResidentReadyFrame, ResidentWorkSource, ResidentWorkTurnCoordinator, ResidentWorkTurnError,
    ResidentWorkTurnInput, ResidentWorkTurnOutcome,
};
pub use process_recovery::{
    with_recovered_resident_service_process, ResidentServicePhysicalProcessRecoveryError,
    ResidentServicePhysicalProcessRecoveryInput,
};
pub use service_policy::{
    ResidentServiceExecutionOwnerFence, ResidentServicePolicyError, ResidentServicePolicyInput,
    ResidentServicePolicyProjection,
};
pub use service_recovery::{
    ResidentServiceDeploymentAuthorityAdmissionPort, ResidentServiceDeploymentAuthorityRecords,
    ResidentServiceHistoricalKeyPort, ResidentServiceHistoricalKeyResolution,
    ResidentServicePhysicalTrustRecoveryInput, ResidentServiceTrustEvidencePorts,
    ResidentServiceTrustFrontier, ResidentServiceTrustPhysicalPorts,
    ResidentServiceTrustRecoveryError, ResidentServiceTrustRecoveryInput,
    ResidentServiceTrustSignaturePort,
};
pub use trust_transition::AdmittedPrincipalTrustCheckpointTransition;

pub use definition::ResidentRelation;
pub use observation_checkpoint::{
    CommittedObservationCheckpointRecovery, PreparedOperationObservationCheckpoint,
};

pub use application::{
    ResidentApplication, ResidentApplicationError, ResidentApplicationErrorClass,
    ResidentColdOpenInput, ResidentColdOpenOutcome, ResidentCommitReceipt,
    ResidentDesiredStatePersistence, ResidentReadOnlyDegradedState, ResidentRecoveryQuarantine,
    ResidentRecoveryQuarantineClass, ResidentSessionAdmissionError,
    ResidentSupervisorReconciliation, ResidentWakeDisposition,
};
pub use effect_case::{
    lookup_effect_case_replay, prepare_effect_case_transition,
    read_reconciliation_operation_association, recover_reconciliation_effect_attempt,
    AdmittedEffectCaseCheckpointTransition, AdmittedEffectCaseRecord,
    CompletedEffectCaseTransition, DatabaseCommittedEffectCaseAnchorPending,
    EffectCaseAnchorEvidence, EffectCaseAnchorFailure, EffectCaseAnchorFailureReason,
    EffectCaseCheckpointConstruction, EffectCaseCheckpointConstructionRejection,
    EffectCaseCheckpointConstructionVerifier, EffectCaseCommitFailure,
    EffectCaseCommitFailureReason, EffectCaseCommitSuccess, EffectCaseDatabaseCommitEvidence,
    EffectCaseDisposition, EffectCaseProjectionError, EffectCaseReplayDisposition,
    EffectCaseReplayLookupError, EffectCaseTransitionInput, EffectDispatchAuthorization,
    PreparedEffectCaseTransition, ReconciliationEffectAttemptInput,
    ReconciliationOperationAssociation, ReconciliationOperationAssociationError,
    ReconciliationOperationAssociationInput, RecoveredReconciliationEffectAttempt,
};
pub use event_custody::{
    admit_a2a_task_predecessor_publication, admit_a2a_task_publication_plan,
    admit_local_operation_publication_plan, advance_a2a_task_publication,
    advance_initial_event_delivery, advance_initial_event_delivery_cursor,
    advance_local_operation_publication, classify_event_identity, classify_genealogy_edge,
    classify_interaction_occurrence, classify_reconciliation_frontier, plan_event_fanout,
    prepare_event_custody_transition, read_pending_pale_derived_publications,
    record_initial_event_acknowledgement, recover_initial_event_delivery,
    sweep_local_operation_publications, A2aTaskPredecessorPublication,
    A2aTaskPublicationProjectionError, A2aTaskPublicationRecoveryLimits,
    A2aTaskPublicationRecoveryRequest, A2aTaskPublicationReferences,
    A2aTaskPublicationSourceRecoveryError, AdmittedA2aTaskPublicationPlan, AdmittedEventRecord,
    AdmittedEventSubscription, AdmittedLocalOperationPublicationPlan, AdmittedOwnerPublication,
    CompletedEventCustodyTransition, DerivedEventRecordDisposition, EventCustodyCommitEvidence,
    EventCustodyCommitFailure, EventCustodyCommitFailureReason, EventCustodyContinuation,
    EventCustodyContinuationKind, EventCustodyProjectionError, EventCustodyRecoveryError,
    EventCustodyTransitionInput, EventDeliveryAttempt, EventDeliveryCursor,
    EventDeliveryDisposition, EventDeliveryOrdinal, EventDeliveryState, EventDeliveryStream,
    EventIdentityDisposition, EventIdentityLookupError, EventPublicationRaceClosure,
    EventPublicationRaceRejection, EventPublicationRaceVerifier, EventPublicationState,
    EventSourceFrontier, EventSubscriptionDeclaration, InitialEventAcknowledgementAdvance,
    InitialEventAcknowledgementAuthority, InitialEventAcknowledgementInput,
    InitialEventDeliveryAdvance, InitialEventDeliveryCursorAdvance, InitialEventDeliveryError,
    InitialEventDeliveryPlan, InitialEventDeliveryRecovery, InitialEventDeliveryRecoveryLimits,
    LocalOperationPredecessorPublication, LocalOperationPublicationAdvance,
    LocalOperationPublicationAdvanceError, LocalOperationPublicationCurrent,
    LocalOperationPublicationIssuance, LocalOperationPublicationPolicy,
    LocalOperationPublicationPolicyInput, LocalOperationPublicationProjectionError,
    LocalOperationPublicationReadmission, LocalOperationPublicationRecoveryLimits,
    LocalOperationPublicationRecoveryRequest, LocalOperationPublicationReferences,
    LocalOperationPublicationSigningContexts, LocalOperationPublicationSourceRecoveryError,
    LocalOperationPublicationStage, LocalOperationPublicationSweepAttempt,
    LocalOperationPublicationSweepError, LocalOperationPublicationSweepLimits,
    LocalOperationPublicationSweepProgress, OperationPublicationRevision,
    PaleDerivedPublicationRetention, PendingPaleDerivedPublication,
    PendingPaleDerivedPublicationPage, PreparedEventCustodyTransition,
    ReconciliationFrontierDisposition, RecordedEventAcknowledgement,
    RecoveredA2aTaskPublicationSource, SubscriptionDeliveryTarget, SubscriptionLifecycle,
};

pub use bootstrap::{
    prepare_principal_bootstrap_custody, CompletedPrincipalBootstrapCustody,
    DatabaseCommittedPrincipalBootstrapAnchorsPending, PreparedPrincipalBootstrapCustody,
    PrincipalBootstrapAnchorFailure, PrincipalBootstrapAnchorFailureReason,
    PrincipalBootstrapCommitEvidence, PrincipalBootstrapCommitFailure,
    PrincipalBootstrapCommitFailureReason, PrincipalBootstrapCustodyPhase,
    PrincipalBootstrapProjectionError,
};
pub use concurrency::{
    resident_cancellation_pair, ResidentAdmissionLimits, ResidentAdmissionSaturation,
    ResidentAdmissionScope, ResidentAdmissionSnapshot, ResidentCancellation,
    ResidentCancellationTrigger, ResidentCompilationCoordinator, ResidentCompilationExecution,
    ResidentCompilationOutcome, ResidentRequestControl, ResidentRequestInterruption,
    ResidentWorkCoordinationError,
};

pub use consultation::{
    commit_consultation_collection, commit_recovered_consultation_cancellation,
    commit_recovered_consultation_child, commit_recovered_consultation_disagreement,
    commit_recovered_consultation_stop, recover_committed_consultation_expiry,
    CompletedConsultationCollection, ConsultationCancellationFailure,
    ConsultationCancellationInput, ConsultationChildDispatchBatch,
    ConsultationChildDispatchCandidate, ConsultationChildJoinFailure, ConsultationChildJoinInput,
    ConsultationChildRetentionOutcome, ConsultationCollection, ConsultationCollectionAdmission,
    ConsultationCollectionCommitFailure, ConsultationCollectionCommitInput,
    ConsultationCollectionError, ConsultationCollectionLimits,
    ConsultationCollectionObservationInput, ConsultationCollectionPredecessor,
    ConsultationCollectionSlot, ConsultationCollectionStop, ConsultationContribution,
    ConsultationContributionSource, ConsultationDisagreement,
    ConsultationDisagreementCommitFailure, ConsultationDisagreementCommitInput,
    ConsultationRequirement, ConsultationRequirementSummary, ConsultationRosterSlot,
    ConsultationSlotDisposition, ConsultationSlotId, ConsultationStopCause,
    ConsultationStopFailure, ConsultationStopInput, ConsultationSuccessorCommitInput,
    HistoricalConsultationAdmission, LateConsultationDisposition, LateConsultationOutcome,
    PreparedConsultationCollection, RecoveredConsultationCollection,
};
pub use generation::{
    AdmittedCompiledGeneration, AppliedMaterializationCoordinate, AppliedRepositoryMaterialization,
    CompiledGenerationAdmissionError, GenerationActivationCompatibility, GenerationInputChange,
    GenerationSuccessorImpact, PackageResourceResidentCoordinate, RemovedPackageResourceBridge,
    RemovedRepositoryBridge, RemovedWorkspaceRootBridge, RepositoryResidentCoordinate,
    ResidentCompositionDefinitionKind, ResidentDefinitionCoordinate,
    ResidentDefinitionFailureCoordinate, ResidentDefinitionInvariant, ResidentImpactQueryError,
    ResidentImpactReasonPath, ResidentImpactReasonStep, ResidentRelationshipEvidenceCoordinate,
    ResidentWorkspaceCausalCoordinate,
};
pub(crate) use hook::response_policy::NativeHookPermission;
pub use hook::{
    AdmittedResidentHookSession, AdmittedResidentHookWritePolicy, AuthenticatedResidentHookRequest,
    CallerBoundResidentHookRequest, CodexCommandObservation, CodexCommandObservationError,
    CodexCommandTerminalStatus, CorrelatedResidentHookOperation, NativeHookPermissionRequest,
    NativeHookResponseAuthorityError, NativeHookResponseAuthorityPort, ResidentHookCallerBinding,
    ResidentHookFrameAdmissionError, ResidentHookIngressAdmissionError, ResidentHookIngressRequest,
    ResidentHookNativeToolBinding, ResidentHookNativeToolBindingError,
    ResidentHookOperationRegistry, ResidentHookOperationRegistryError,
    ResidentHookOperationWritePolicyError, ResidentHookPolicyAdmissionError,
    ResidentHookSessionAdmissionError, ResidentHookTransportUnavailable,
    ResidentHookWritePolicyEvaluation, ResidentNativeToolStart, ResidentNativeToolStartError,
    RESIDENT_HOOK_EVENT_BYTE_LIMIT,
};
pub use lifecycle::{
    GenerationLifecycle, GenerationLifecycleError, GenerationLifecycleRecoveryResolver,
    GenerationLifecycleSnapshot, GenerationLifecycleTransition, GenerationRetentionReason,
    ReclamationDisposition, RecoveryReferenceKnowledge, SessionGenerationPin,
    SessionGenerationPinSnapshot,
};
pub use local_operation_start::{
    prepare_local_child_parent, recover_local_child_operation, recover_local_operation,
    recover_local_operation_from_handle, IssuedLocalOperation, LocalChildOperationRecoveryContext,
    LocalOperationCheckpointFailure, LocalOperationHandleRecoveryInput,
    LocalOperationRecoveryAdmission, LocalOperationRecoveryContext, LocalOperationRecoveryError,
    LocalOperationStartError, LocalOperationStartInput, PendingLocalOperationStart,
    PreparedLocalChildParent, PreparedLocalOperationCheckpoint, PreparedLocalOperationStart,
    PreparedLocalOperationStartFailure, RecoveredLocalOperation,
};
pub(in crate::resident) use local_operation_start::{
    recover_local_child_operation_from_handle, LocalChildOperationHandleRecoveryInput,
};
pub use operation_dispatch::{
    HookResponseCheckpointSigner, HookResponseObservationSigner, HookResponseRotation,
    NativeHookResponseInputs,
};
#[cfg(any(test, feature = "control-store-test-support"))]
#[path = "../../tests/support/resident_hook_response_phase.rs"]
#[doc(hidden)]
pub mod hook_response_phase_test_support;
#[cfg(any(test, feature = "control-store-test-support"))]
#[doc(hidden)]
pub use process_host::service_assembly::hook_response_restart_test_support;
#[cfg(any(test, feature = "control-store-test-support"))]
#[path = "../../tests/support/consultation_recovery.rs"]
#[doc(hidden)]
pub mod consultation_recovery_test_support;
#[cfg(any(test, feature = "control-store-test-support"))]
#[path = "../../tests/support/resident_local_operation_start.rs"]
#[doc(hidden)]
pub mod local_operation_start_test_support;
pub use operation_control::{
    authorize_operation_cancellation, authorize_operation_observation, observe_local_operation,
    recover_operation_control, AuthorizedOperationCancellation, AuthorizedOperationObservation,
    DispatchedOperationCancellationRequestInput, LocalOperationCancellationCompletionInput,
    LocalOperationCancellationRequestInput, LocalOperationObservationAdmission,
    LocalOperationObservationError, LocalOperationObservationInput,
    OperationCancellationCompletionError, OperationCancellationDisposition,
    OperationCancellationRecoveryError, OperationCancellationRequestError,
    OperationControlAuthorizationContext, OperationControlError, OperationControlInput,
    OperationControlObservation, OperationControlRecoveryAdmission, OperationControlRecoveryError,
    OperationControlRecoveryInput, PreparedDispatchedOperationCancellationRequest,
    PreparedLocalOperationCancellationCompletion, PreparedLocalOperationCancellationRequest,
    RecoveredLocalOperationCancellationRequest, RecoveredOperationControl,
    OPERATION_CANCEL_CAPABILITY_ID, OPERATION_OBSERVE_CAPABILITY_ID,
};
pub(in crate::resident) use operation_control::{
    recover_child_operation_control, validate_recovered_operation_cancellation,
    ChildOperationControlRecoveryInput,
};
pub use operation_custody::{
    prepare_operation_start, read_operation_start, read_operation_start_for_request,
    CompletedOperationObservationCustody, CompletedOperationStartCustody,
    DatabaseCommittedOperationObservationAnchorPending,
    DatabaseCommittedOperationStartAnchorPending, OperationCustodyAnchorEvidence,
    OperationCustodyAnchorFailureReason, OperationCustodyDatabaseEvidence,
    OperationCustodyDatabaseFailureReason, OperationObservationCustodyAnchorFailure,
    OperationObservationCustodyCommitFailure, OperationObservationCustodyCommitFailureReason,
    OperationRecoveryError, OperationStartAnchorFailure, OperationStartCommitFailure,
    OperationStartCommitFailureReason, OperationStartPreparationError,
    OperationStartRecoveryEvidence, PreparedOperationObservationCustody, PreparedOperationStart,
};
pub use operation_dispatch::{
    admit_native_operation_failure, admit_native_operation_result, commit_a2a_send_observation,
    continue_consumed_provider_recording, invoke_native_operation, invoke_provider_operation,
    launch_a2a_delegate, prepare_a2a_send, prepare_native_operation_attempt,
    prepare_operation_context_binding, prepare_provider_resolution,
    record_consumed_provider_operation, recover_a2a_send_outcome_unknown,
    recover_native_operation_dispatch, recover_native_operation_outcome, retain_a2a_outcome,
    retain_provider_operation_dispatch, validate_operation_attempt_result,
    A2aDelegateLaunchFailure, A2aDelegateLaunchInput, A2aFailedSendObservationInput,
    A2aInitialResponseFacts, A2aInitialTaskCustodyInput, A2aNoTaskObservationFailure,
    A2aNoTaskOutcomeRetention, A2aNotStartedObservationFailure, A2aNotStartedOutcomeRetention,
    A2aOutcomeRetentionFailure, A2aOutcomeRetentionInput, A2aOutcomeUnknownObservationFailure,
    A2aOutcomeUnknownObservationInput, A2aRecoveredUnknownOutcomeRetention, A2aSendCheckpointError,
    A2aSendCustodyFailure, A2aSendDiagnosticFailure, A2aSendDispatch, A2aSendInput,
    A2aSendNotStarted, A2aSendObservationCommitFailure, A2aSendObservationCommitInput,
    A2aSendOutcomeUnknown, A2aSendPreparationError, A2aSendRecoveryAdmission, A2aSendRecoveryError,
    A2aSendRecoveryInput, A2aSendRecoveryLimits, A2aSendReleaseError, A2aSendResponseFailure,
    A2aSendResponsePoll, A2aSendStartFailure, A2aTaskOutcomeRetention, A2aUnknownOutcomeRetention,
    AdmittedStagedNativeOperationOutcome, AwaitingA2aDelegate, AwaitingA2aSendResponse,
    CompletedA2aNoTaskResponse, CompletedA2aSendObservationCustody, CompletedA2aSendResponse,
    CompletedA2aTaskResponse, CompletedNativeOperation, CompletedOperationContextBinding,
    CompletedProviderOperation, ConsumedProviderOperation, ConsumedProviderRecordingContinuation,
    DatabaseCommittedA2aSendAnchorPending, ExecutedNativeOperation, FailedNativeOperation,
    InFlightA2aSend, NativeOperationAttemptInput, NativeOperationCancellationInput,
    NativeOperationCancellationRefusal, NativeOperationCompletionInput,
    NativeOperationDispatchContext, NativeOperationExecutionContext, NativeOperationExpiryInput,
    NativeOperationExpiryRefusal, NativeOperationFailureEvidenceInput, NativeOperationFailureInput,
    NativeOperationInvocationRefusal, NativeOperationInvokeFailure, NativeOperationInvokeInput,
    NativeOperationOutcome, NativeOperationOutcomeRecoveryInput, NativeOperationOutcomeStageInput,
    NativeOperationOutcomeStagePreparationFailure, NativeOperationRecordingFailure,
    NativeOperationRecoveryContext, NativeOperationRefusalCancellationInput,
    NativeOperationResultInput, NativeOperationStageFinalization,
    NativeOperationStageFinalizationFailure, NativeOperationStagingAnchorPending,
    NativeOperationStagingCommitFailure, NativeOperationTerminalEvidence,
    NativeOperationTerminalRecordingInput, OperationAppendFailure, OperationAttemptContext,
    OperationAttemptError, OperationAttemptHistoryInput, OperationAttemptResultInput,
    OperationCheckpointInput, OperationContextBindingInput, PreparedA2aSend,
    PreparedA2aSendCheckpoint, PreparedA2aSendCustody, PreparedA2aSendObservation,
    PreparedNativeOperationAttempt, PreparedNativeOperationCancellation,
    PreparedNativeOperationCompletion, PreparedNativeOperationDispatch,
    PreparedNativeOperationExpiry, PreparedNativeOperationFailure,
    PreparedNativeOperationOutcomeStage, PreparedOperationContextBinding,
    PreparedProviderOperationAttempt, PreparedProviderOperationDispatch,
    PreparedProviderResolution, PreparedRecoveredA2aSendObservation,
    ProviderOperationDispatchContext, ProviderOperationDispatchFailure,
    ProviderOperationDispatchInput, ProviderOperationInput, ProviderOperationInvokeFailure,
    ProviderOperationInvokeInput, ProviderOperationInvokeResult,
    ProviderOperationPreparationFailure, ProviderOperationRecordingConflict,
    ProviderOperationRecordingCustody, ProviderOperationRecordingFailure,
    ProviderOperationRecordingFinalization, ProviderOperationRecordingFinalizationFailure,
    ProviderOperationRecordingPreparationError, ProviderOperationRecordingRecoveryFailure,
    ProviderOperationRetentionFailure, ProviderOperationUncertainty, ReceivedA2aSendResponse,
    RecoveredA2aSendDiagnostic, RecoveredA2aSendObservationFailure, RecoveredA2aSendOutcomeUnknown,
    RecoveredNativeOperationDispatch, RecoveredNativeOperationOutcome, RetainedA2aOutcome,
    RetainedA2aSend, RetainedProviderOperationDispatch, StagedNativeOperationOutcome,
    UncertainProviderOperation, UnstagedNativeOperationOutcome,
};
pub use operation_lease_custody::{
    recover_operation_lease_body, retain_operation_lease_body, OperationLeaseBodyRecoveryError,
};
pub use operation_observation_custody::{
    prepare_operation_observation_append, read_operation_observation_for_handle,
    recover_operation_observation, OperationDispatchCustodyError,
    OperationObservationAuthorityStoreRejection, OperationObservationAuthorityStoreResolver,
    OperationObservationCheckpointConstruction,
    OperationObservationCheckpointConstructionRejection,
    OperationObservationCheckpointConstructionVerifier, OperationObservationCheckpointTransition,
    OperationObservationCustodyProjectionError, OperationObservationReadmissionError,
    OperationObservationRecoveryAdmission, OperationObservationRecoveryEvidence,
    OperationObservationRecoveryInput, OperationObservationRecoveryLimits,
    OperationObservationSelection, OperationObservationTransition,
    PreparedOperationObservationAppend, RecoveredOperationObservation,
};
pub use process_reconciliation::{
    classify_restart, AbsentProcessState, DesiredProcessState, DesiredStateTransition,
    InvalidProcessGeneration, InvalidRestartPolicy, IsolatedProcessClaim,
    PendingSupervisorReconciliation, ProcessClaimMismatch, ProcessFenceClaim, ProcessGeneration,
    ProcessIsolationAction, ProcessIsolationPolicy, ProcessObservationClaim, RestartDisposition,
    RestartPolicy, ServiceDirective, ServiceDirectiveAction, ServicePrerequisiteFailure,
    ServicePrerequisiteFailureReason, ServiceProbeClaim, ServiceProbeUnhealthyReason,
    ServiceStartHold, StopSentinelIntent, SupervisedService, SupervisorCommand,
    SupervisorObservationError, SupervisorPlan, SupervisorPlanError, SupervisorReconciliation,
    SupervisorReconciliationStep, SupervisorServiceDeclaration, SupervisorServiceId,
    SupervisorSnapshot,
};
pub use producer::{
    prepare_initial_principal_producer_incarnation,
    prepare_principal_producer_incarnation_retirement,
    prepare_principal_producer_incarnation_successor,
    recover_principal_producer_incarnation_frontier, AdmittedProducerControlCheckpointTransition,
    AdmittedProducerIncarnation, AdmittedProducerIncarnationRetirement,
    CompletedPrincipalProducerIncarnationCustody,
    DatabaseCommittedPrincipalProducerIncarnationAnchorsPending,
    PreparedPrincipalProducerIncarnationCustody, PrincipalProducerCheckpointConstruction,
    PrincipalProducerCheckpointConstructionRejection,
    PrincipalProducerCheckpointConstructionVerifier, PrincipalProducerIncarnationAnchorFailure,
    PrincipalProducerIncarnationAnchorFailureReason, PrincipalProducerIncarnationCommitEvidence,
    PrincipalProducerIncarnationCommitFailure, PrincipalProducerIncarnationCommitFailureReason,
    PrincipalProducerIncarnationCustodyPhase, PrincipalProducerIncarnationProjectionContext,
    PrincipalProducerIncarnationProjectionError, PrincipalProducerIncarnationRecoveryAdmission,
    RecoveredPrincipalProducerIncarnationFrontier,
};
pub use reconstruction::{
    ResidentGenerationInputSource, ResidentGenerationInputSourceError,
    ResidentGenerationReconstructionInput, ResidentReconstructionParameters,
};
pub use session_admission::{
    admit_ambient_session, AdmittedAmbientSession, AmbientSessionAdmissionContext,
    AmbientSessionAdmissionError, AmbientSessionAuthorityError, AmbientSessionAuthorityPort,
    AmbientSessionRouteUse, CapabilityGrantAuthority, CapabilityGrantRevocationHead,
    CapabilityRevocationFrontier, CurrentPrincipalTrustHead,
    DeploymentAcceptanceRevocationFrontier, DeploymentAcceptanceRevocationHead,
    DeploymentRuntimeAuthorityHead, SessionRevocationFrontier,
};
pub use writer::{
    prepare_clean_principal_resident_writer_retirement,
    prepare_clean_principal_resident_writer_successor,
    prepare_initial_principal_resident_writer_transition,
    CleanPrincipalResidentWriterTransitionPreimage, PrincipalResidentWriterProjectionError,
};
