//! Complete Rust-owned producer catalog for registered Pale contracts.
//!
//! The catalog maps every descriptor-manifest identity to exactly one Rust
//! wire-shape producer. It does not select active membership or lifecycle.

use std::collections::BTreeMap;

use schemars::JsonSchema;

use pale_core::engine_wire::{
    DecisionEvidenceLedgerV1, DeferredSurfaceManifestV1, ExpiringRecordEnvelopeV1,
    OperationDenialV1, SessionRuntimeStateV1,
};
use pale_core::records::{
    A2aRouteAdmissionWireV1, A2aTaskBindingWireV1, A2aTaskObservationWireV1, ActionEnvelopeWireV1,
    ActivationFenceWireV1, ActiveGenerationHeadWireV1, AdapterFailureWireV1,
    AdvertisedGenerationHeadWireV1, CapabilityResultWireV1, CapabilityRevocationCheckpointWireV1,
    CapabilitySessionGrantRevocationWireV1, CapabilitySessionGrantWireV1,
    CheckpointSigningAuthorityWireV1, CommonRequestEnvelopeWireV1, CompiledGenerationWireV1,
    ContractSetWireV1, ControlStateCheckpointWireV1, DeploymentAcceptanceRevocationWireV1,
    DeploymentAcceptanceWireV1, DeploymentBindingProposalWireV1, DeploymentBindingWireV1,
    DeploymentControlStoreGenesisReceiptWireV1, DeploymentRuntimeAuthorityWireV1,
    DeploymentTermsWireV1, EffectAdmissionWireV1, EffectConflictClaimWireV1,
    EffectDispatchIntentWireV1, EffectReceiptWireV1, EventAcknowledgementWireV1,
    EventReferenceWireV1, ExecutionOwnerFenceWireV1, GenealogyEdgeWireV1,
    InteractionOccurrenceWireV1, LocalAdminAuthorityWireV1, LocalAdminCommandWireV1,
    LocalAdminReceiptWireV1, LocalRouteBindingWireV1, OperationHandleWireV1, OperationLeaseWireV1,
    OperationObservationWireV1, PrincipalBootstrapReceiptWireV1, PrincipalKeyTransitionWireV1,
    PrincipalRecordWireV1, PrincipalRecoveryPolicyWireV1, PrincipalRecoveryProofWireV1,
    PrincipalResidentWriterFenceWireV1, PrincipalResidentWriterTakeoverReceiptWireV1,
    PrincipalResidentWriterTerminalReceiptWireV1, PrincipalTrustCheckpointWireV1,
    ProducerIncarnationAuthorityWireV1, ProducerIncarnationRetirementWireV1,
    ProducerIncarnationWireV1, PublishedGenerationHeadWireV1, ReconciliationCaseWireV1,
    ReconciliationCustodyWireV1, ReconciliationFrontierWireV1, ReconciliationRequestWireV1,
    ReconciliationResultWireV1, ResourceContinuityProfileWireV1, SessionAdmissionLeaseWireV1,
    TypedDigestWireV1, A2A_ROUTE_ADMISSION_V1_SCHEMA_ID, A2A_TASK_BINDING_V1_SCHEMA_ID,
    A2A_TASK_OBSERVATION_V1_SCHEMA_ID, ACTION_ENVELOPE_V1_SCHEMA_ID, ACTIVATION_FENCE_V1_SCHEMA_ID,
    ACTIVE_GENERATION_HEAD_V1_SCHEMA_ID, ADAPTER_FAILURE_V1_SCHEMA_ID,
    ADVERTISED_GENERATION_HEAD_V1_SCHEMA_ID, CAPABILITY_RESULT_V1_SCHEMA_ID,
    CAPABILITY_REVOCATION_CHECKPOINT_V1_SCHEMA_ID,
    CAPABILITY_SESSION_GRANT_REVOCATION_V1_SCHEMA_ID, CAPABILITY_SESSION_GRANT_V1_SCHEMA_ID,
    CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA_ID, COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID,
    COMPILED_GENERATION_V1_SCHEMA_ID, CONTRACT_SET_V1_SCHEMA_ID,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID, DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SCHEMA_ID,
    DEPLOYMENT_ACCEPTANCE_V1_SCHEMA_ID, DEPLOYMENT_BINDING_PROPOSAL_V1_SCHEMA_ID,
    DEPLOYMENT_BINDING_V1_SCHEMA_ID, DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA_ID,
    DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA_ID, DEPLOYMENT_TERMS_V1_SCHEMA_ID,
    EFFECT_ADMISSION_V1_SCHEMA_ID, EFFECT_CONFLICT_CLAIM_V1_SCHEMA_ID,
    EFFECT_DISPATCH_INTENT_V1_SCHEMA_ID, EFFECT_RECEIPT_V1_SCHEMA_ID,
    EVENT_ACKNOWLEDGEMENT_V1_SCHEMA_ID, EVENT_REFERENCE_V1_SCHEMA_ID,
    EXECUTION_OWNER_FENCE_V1_SCHEMA_ID, GENEALOGY_EDGE_V1_SCHEMA_ID,
    INTERACTION_OCCURRENCE_V1_SCHEMA_ID, LOCAL_ADMIN_AUTHORITY_V1_SCHEMA_ID,
    LOCAL_ADMIN_COMMAND_V1_SCHEMA_ID, LOCAL_ADMIN_RECEIPT_V1_SCHEMA_ID,
    LOCAL_ROUTE_BINDING_V1_SCHEMA_ID, OPERATION_HANDLE_V1_SCHEMA_ID, OPERATION_LEASE_V1_SCHEMA_ID,
    OPERATION_OBSERVATION_V1_SCHEMA_ID, PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SCHEMA_ID,
    PRINCIPAL_KEY_TRANSITION_V1_SCHEMA_ID, PRINCIPAL_RECORD_V1_SCHEMA_ID,
    PRINCIPAL_RECOVERY_POLICY_V1_SCHEMA_ID, PRINCIPAL_RECOVERY_PROOF_V1_SCHEMA_ID,
    PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA_ID,
    PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA_ID,
    PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA_ID,
    PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA_ID, PRODUCER_INCARNATION_AUTHORITY_V1_SCHEMA_ID,
    PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA_ID, PRODUCER_INCARNATION_V1_SCHEMA_ID,
    PUBLISHED_GENERATION_HEAD_V1_SCHEMA_ID, RECONCILIATION_CASE_V1_SCHEMA_ID,
    RECONCILIATION_CUSTODY_V1_SCHEMA_ID, RECONCILIATION_FRONTIER_V1_SCHEMA_ID,
    RECONCILIATION_REQUEST_V1_SCHEMA_ID, RECONCILIATION_RESULT_V1_SCHEMA_ID,
    RESOURCE_CONTINUITY_PROFILE_V1_SCHEMA_ID, SESSION_ADMISSION_LEASE_V1_SCHEMA_ID,
    TYPED_DIGEST_V1_SCHEMA_ID,
};
use pale_core::records::{
    ActionContract, ActionExecution, ActionInput, ActionOutput, AgentSurfacePackageV1,
    AgentSurfacePlanV1, CapabilitiesFile, CapabilityBindingsV1, CapabilityPolicy,
    CompiledContentCoordinateV1, CompositionEdgeV1, ContinuityContractV1, DeferredEffect,
    DestinationConsequenceV1, DiagnosticV1Wire, DistributionLockFile, DoctrineFile,
    EffectiveProjectProfileV1, EntryContractV1, EntryOriginRefV1, ExperienceCompositionPlanV1,
    ExperiencePathV1, GeneratedAgentSurfaceManifestV1, GoverningOrderFile,
    HarnessProjectionManifestV1, HookContextV1, LifecycleEvent, MachineBindingFile,
    MachineLocalRootIdentity, MaterializationLineageV1, MaterializationPlanV1,
    MaterializationReceiptV1, MaterializationStateV1, NativeActionCatalogV1, NativeHarnessBinding,
    NativeOutcomeV1, NativeOwnerTransitionV1, OfficialReference, OperationCapabilityRequirement,
    OperationContractCatalogV1, OperationContractV1, OperationRouteBindingV1, OperationsFile,
    OrganizationalTranslationV1, PackageDataV1, PackagePageV1, ParticipantProviderManifestV1,
    ParticipantRefV1, PathBudgetV1, PathInvocationV1, PolicyFamilyV1, ProviderBindingV1,
    ReconciliationRequirementV1, RepositoryNativeActionCatalogV1, RequestTranslationV1,
    ResolvedProjectInstanceV1, ResponsibilityContractV1, ResponsibilityDerivationV1,
    ResponsibilityV1, ResultTranslationV1, RolesFile, RunModesFile, RunResultV1, SchemaDigest,
    SealedArtifactRef, SettlementContractV1, StacksFile, SurfaceDigest, SurfaceExecutableBinding,
    SurfaceExecutableIdentityV1, SurfaceOwnership, TerminationContractV1, ToolCallMatcher,
    ToolEvidenceContract, ToolRecipe, WorkspaceCompositionSourceV1, WorkspaceDerivationReceiptV1,
    WorkspaceLanguagePrimitivesV1, WorkspaceManifestFile, WorkspaceModuleSourceV1,
    WorkspaceModuleV1, WorkspaceResolutionLockV1, WorkspaceUniverseLockV1,
    ORGANIZATIONAL_TRANSLATION_V1_SCHEMA_ID, RESPONSIBILITY_DERIVATION_V1_SCHEMA_ID,
};

use crate::schema_projection::{
    definition_set, escape_pointer_token, AdmissionRoute, ContractProducer, ExternalDefinitionRef,
    ProjectionKind,
};
use pale_core::records::{NativeToolRequestV1, NATIVE_TOOL_REQUEST_V1_SCHEMA_ID};

/// Number of identities in the complete initial-v1 contract registry.
pub(crate) const CONTRACT_PRODUCER_COUNT: usize = 138;

/// Return every Rust producer in stable descriptor identity order.
///
/// The descriptor manifest must contain exactly this set. Its map, not this
/// function, owns whether an identity is active and the lifecycle attached to
/// it.
#[must_use]
pub(crate) fn contract_producers() -> [ContractProducer; CONTRACT_PRODUCER_COUNT] {
    use AdmissionRoute::{Contextual, Generated, StructuralOnly};

    [
        ContractProducer::typed::<A2aRouteAdmissionWireV1>(
            A2A_ROUTE_ADMISSION_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<A2aTaskBindingWireV1>(A2A_TASK_BINDING_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<A2aTaskObservationWireV1>(
            A2A_TASK_OBSERVATION_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ActionContract>("action-contract.v1", Contextual),
        ContractProducer::typed::<ActionEnvelopeWireV1>(ACTION_ENVELOPE_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<ActionExecution>("action-execution.v1", Contextual),
        ContractProducer::typed::<ActionInput>("action-input.v1", Contextual),
        ContractProducer::typed::<ActionOutput>("action-output.v1", Contextual),
        ContractProducer::typed::<ActivationFenceWireV1>(ACTIVATION_FENCE_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<ActiveGenerationHeadWireV1>(
            ACTIVE_GENERATION_HEAD_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<AdapterFailureWireV1>(ADAPTER_FAILURE_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<AdvertisedGenerationHeadWireV1>(
            ADVERTISED_GENERATION_HEAD_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<AgentSurfacePackageV1>("agent-surface-package.v1", Contextual),
        ContractProducer::typed::<AgentSurfacePlanV1>("agent-surface-plan.v1", Contextual),
        ContractProducer::typed::<CapabilitiesFile>("capabilities.v1", Contextual),
        ContractProducer::typed::<CapabilityBindingsV1>("capability-bindings.v1", Generated),
        ContractProducer::definitions(
            "capability-procedure.v1",
            Contextual,
            capability_procedure,
            capability_procedure_names,
        ),
        ContractProducer::typed::<CapabilityResultWireV1>(
            CAPABILITY_RESULT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<CapabilityRevocationCheckpointWireV1>(
            CAPABILITY_REVOCATION_CHECKPOINT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<CapabilitySessionGrantWireV1>(
            CAPABILITY_SESSION_GRANT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<CapabilitySessionGrantRevocationWireV1>(
            CAPABILITY_SESSION_GRANT_REVOCATION_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<CompositionEdgeV1>("composition-edge.v1", Contextual),
        ContractProducer::definitions(
            "composition-primitives.v1",
            Contextual,
            composition_primitives,
            composition_primitive_names,
        ),
        ContractProducer::typed::<CheckpointSigningAuthorityWireV1>(
            CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<CommonRequestEnvelopeWireV1>(
            COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<CompiledGenerationWireV1>(
            COMPILED_GENERATION_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ContractSetWireV1>(CONTRACT_SET_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<ContinuityContractV1>("continuity-contract.v1", Contextual),
        ContractProducer::typed::<ControlStateCheckpointWireV1>(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<DecisionEvidenceLedgerV1>(
            "decision-evidence-ledger.v1",
            Contextual,
        ),
        ContractProducer::typed::<DeferredEffect>("deferred-effect.v1", StructuralOnly),
        ContractProducer::typed::<DeferredSurfaceManifestV1>(
            "deferred-surface-manifest.v1",
            Generated,
        ),
        ContractProducer::typed::<DeploymentAcceptanceRevocationWireV1>(
            DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<DeploymentAcceptanceWireV1>(
            DEPLOYMENT_ACCEPTANCE_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<DeploymentBindingProposalWireV1>(
            DEPLOYMENT_BINDING_PROPOSAL_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<DeploymentBindingWireV1>(
            DEPLOYMENT_BINDING_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<DeploymentControlStoreGenesisReceiptWireV1>(
            DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<DeploymentRuntimeAuthorityWireV1>(
            DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<DeploymentTermsWireV1>(DEPLOYMENT_TERMS_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<DiagnosticV1Wire>("diagnostic.v1", Contextual),
        ContractProducer::typed::<DistributionLockFile>("distribution-lock.v1", Contextual),
        ContractProducer::typed::<DoctrineFile>("doctrine-entry.v1", Contextual),
        ContractProducer::typed::<EffectAdmissionWireV1>(EFFECT_ADMISSION_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<EffectConflictClaimWireV1>(
            EFFECT_CONFLICT_CLAIM_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<EffectDispatchIntentWireV1>(
            EFFECT_DISPATCH_INTENT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<EffectReceiptWireV1>(EFFECT_RECEIPT_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<EffectiveProjectProfileV1>("effective-profile.v1", Contextual),
        ContractProducer::typed::<EntryContractV1>("entry-contract.v1", Contextual),
        ContractProducer::typed::<EventAcknowledgementWireV1>(
            EVENT_ACKNOWLEDGEMENT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<EventReferenceWireV1>(EVENT_REFERENCE_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<ExecutionOwnerFenceWireV1>(
            EXECUTION_OWNER_FENCE_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ExperienceCompositionPlanV1>(
            "experience-composition-plan.v1",
            Contextual,
        ),
        ContractProducer::typed::<ExperiencePathV1>("experience-path.v1", Contextual),
        ContractProducer::typed::<ExpiringRecordEnvelopeV1<BTreeMap<String, serde_json::Value>>>(
            "expiring-record-envelope.v1",
            Contextual,
        ),
        ContractProducer::typed::<GeneratedAgentSurfaceManifestV1>(
            "generated-manifest.v1",
            Generated,
        ),
        ContractProducer::typed::<GenealogyEdgeWireV1>(GENEALOGY_EDGE_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<GoverningOrderFile>("governing-order.v1", Contextual),
        ContractProducer::typed::<HarnessProjectionManifestV1>("harness-projection.v1", Generated),
        ContractProducer::typed::<HookContextV1>("hook-context.v1", Generated),
        ContractProducer::typed::<InteractionOccurrenceWireV1>(
            INTERACTION_OCCURRENCE_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<WorkspaceManifestFile>("pale-workspace.v1", Contextual),
        ContractProducer::typed::<LifecycleEvent>("lifecycle-event.v1", StructuralOnly),
        ContractProducer::typed::<LocalAdminAuthorityWireV1>(
            LOCAL_ADMIN_AUTHORITY_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<LocalAdminCommandWireV1>(
            LOCAL_ADMIN_COMMAND_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<LocalAdminReceiptWireV1>(
            LOCAL_ADMIN_RECEIPT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<LocalRouteBindingWireV1>(
            LOCAL_ROUTE_BINDING_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<MachineBindingFile>("machine-binding.v1", Contextual),
        ContractProducer::typed::<MachineLocalRootIdentity>(
            "machine-local-root-identity.v1",
            Contextual,
        ),
        ContractProducer::typed::<MaterializationLineageV1>(
            "materialization-lineage.v1",
            Contextual,
        ),
        ContractProducer::typed::<MaterializationPlanV1>("materialization-plan.v1", Contextual),
        ContractProducer::typed::<MaterializationStateV1>("materialization-state.v1", Contextual),
        ContractProducer::typed::<NativeActionCatalogV1>("native-action-catalog.v1", Contextual),
        ContractProducer::typed::<NativeHarnessBinding>("native-harness-binding.v1", Contextual),
        ContractProducer::typed::<NativeToolRequestV1>(
            NATIVE_TOOL_REQUEST_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<OfficialReference>("official-reference.v1", Contextual),
        ContractProducer::typed::<OrganizationalTranslationV1>(
            ORGANIZATIONAL_TRANSLATION_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<OperationContractCatalogV1>("operation-catalog.v1", Generated),
        ContractProducer::typed::<OperationContractV1>("operation-contract.v1", Generated),
        ContractProducer::typed::<OperationDenialV1>("operation-denial.v1", StructuralOnly),
        ContractProducer::typed::<OperationHandleWireV1>(OPERATION_HANDLE_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<OperationLeaseWireV1>(OPERATION_LEASE_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<OperationObservationWireV1>(
            OPERATION_OBSERVATION_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<OperationRouteBindingV1>("operation-route-binding.v1", Generated),
        ContractProducer::typed::<OperationsFile>("operation.v1", Contextual),
        ContractProducer::typed::<PackageDataV1>("package-data.v1", Contextual),
        ContractProducer::typed::<PackagePageV1>("package-page.v1", Contextual),
        ContractProducer::typed::<ParticipantProviderManifestV1>(
            "participant-provider-manifest.v1",
            Contextual,
        ),
        ContractProducer::typed::<PolicyFamilyV1>("policy-family.v1", Contextual),
        ContractProducer::typed::<PrincipalBootstrapReceiptWireV1>(
            PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<PrincipalKeyTransitionWireV1>(
            PRINCIPAL_KEY_TRANSITION_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<PrincipalRecordWireV1>(PRINCIPAL_RECORD_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<PrincipalRecoveryPolicyWireV1>(
            PRINCIPAL_RECOVERY_POLICY_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<PrincipalRecoveryProofWireV1>(
            PRINCIPAL_RECOVERY_PROOF_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<PrincipalResidentWriterFenceWireV1>(
            PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<PrincipalResidentWriterTakeoverReceiptWireV1>(
            PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<PrincipalResidentWriterTerminalReceiptWireV1>(
            PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<PrincipalTrustCheckpointWireV1>(
            PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ProducerIncarnationAuthorityWireV1>(
            PRODUCER_INCARNATION_AUTHORITY_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ProducerIncarnationRetirementWireV1>(
            PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ProducerIncarnationWireV1>(
            PRODUCER_INCARNATION_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ProviderBindingV1>("provider-binding.v1", Contextual),
        ContractProducer::typed::<PublishedGenerationHeadWireV1>(
            PUBLISHED_GENERATION_HEAD_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<MaterializationReceiptV1>("receipt.v1", Contextual),
        ContractProducer::typed::<ReconciliationCaseWireV1>(
            RECONCILIATION_CASE_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ReconciliationCustodyWireV1>(
            RECONCILIATION_CUSTODY_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ReconciliationFrontierWireV1>(
            RECONCILIATION_FRONTIER_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ReconciliationRequestWireV1>(
            RECONCILIATION_REQUEST_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ReconciliationResultWireV1>(
            RECONCILIATION_RESULT_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<RepositoryNativeActionCatalogV1>(
            "repository-native-action-catalog.v1",
            Generated,
        ),
        ContractProducer::typed::<RequestTranslationV1>("request-translation.v1", Contextual),
        ContractProducer::typed::<ResolvedProjectInstanceV1>("resolved-instance.v1", Contextual),
        ContractProducer::typed::<ResourceContinuityProfileWireV1>(
            RESOURCE_CONTINUITY_PROFILE_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ResponsibilityContractV1>(
            "responsibility-contract.v1",
            Contextual,
        ),
        ContractProducer::typed::<ResponsibilityDerivationV1>(
            RESPONSIBILITY_DERIVATION_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<ResultTranslationV1>("result-translation.v1", Contextual),
        ContractProducer::typed::<RolesFile>("role.v1", Contextual),
        ContractProducer::typed::<RunModesFile>("run-mode.v1", Contextual),
        ContractProducer::typed::<RunResultV1>("run-result.v1", Contextual),
        ContractProducer::typed::<SurfaceExecutableBinding>(
            "surface-executable-binding.v1",
            Contextual,
        ),
        ContractProducer::typed::<SealedArtifactRef>("sealed-artifact.v1", Contextual),
        ContractProducer::typed::<SessionAdmissionLeaseWireV1>(
            SESSION_ADMISSION_LEASE_V1_SCHEMA_ID,
            Contextual,
        ),
        ContractProducer::typed::<SessionRuntimeStateV1>("session-runtime-state.v1", Contextual),
        ContractProducer::typed::<SettlementContractV1>("settlement-contract.v1", Contextual),
        ContractProducer::typed::<SchemaDigest>("sha256-digest.v1", StructuralOnly),
        ContractProducer::typed::<StacksFile>("stack.v1", Contextual),
        ContractProducer::typed::<SurfaceDigest>("surface-digest.v1", Contextual),
        ContractProducer::typed::<SurfaceOwnership>("surface-ownership.v1", StructuralOnly),
        ContractProducer::typed::<SurfaceExecutableIdentityV1>(
            "surface-executable-identity.v1",
            Contextual,
        ),
        ContractProducer::typed::<TerminationContractV1>("termination-contract.v1", Contextual),
        ContractProducer::typed::<ToolCallMatcher>("tool-call-matcher.v1", StructuralOnly),
        ContractProducer::typed::<TypedDigestWireV1>(TYPED_DIGEST_V1_SCHEMA_ID, Contextual),
        ContractProducer::typed::<WorkspaceCompositionSourceV1>(
            "workspace-composition-source.v1",
            Contextual,
        ),
        ContractProducer::typed::<WorkspaceDerivationReceiptV1>(
            "workspace-derivation-receipt.v1",
            Contextual,
        ),
        ContractProducer::typed::<WorkspaceLanguagePrimitivesV1>(
            "workspace-language-primitives.v1",
            Contextual,
        ),
        ContractProducer::typed::<WorkspaceModuleSourceV1>(
            "workspace-module-source.v1",
            Contextual,
        ),
        ContractProducer::typed::<WorkspaceModuleV1>("workspace-module.v1", Contextual),
        ContractProducer::typed::<WorkspaceResolutionLockV1>(
            "workspace-resolution-lock.v1",
            Contextual,
        ),
        ContractProducer::typed::<WorkspaceUniverseLockV1>(
            "workspace-universe-lock.v1",
            Contextual,
        ),
    ]
}

pub(crate) fn project_contract_set(
    schema_base_uri: &str,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    project_contract_set_with(schema_base_uri, ContractProducer::project)
}

pub(crate) fn project_authoring_contract_set(
    schema_base_uri: &str,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    project_contract_set_with(schema_base_uri, ContractProducer::project_for_authoring)
}

/// One closed projection from a registered producer into its schema document.
type ContractProjection = fn(
    ContractProducer,
    &str,
    &BTreeMap<String, ExternalDefinitionRef>,
) -> Result<serde_json::Value, String>;

fn project_contract_set_with(
    schema_base_uri: &str,
    project: ContractProjection,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let producers = contract_producers();
    let mut external_refs = BTreeMap::new();
    for producer in producers {
        for name in producer.external_names() {
            let local_reference = format!("#/$defs/{}", escape_pointer_token(&name));
            let uri = if producer.projection_kind() == ProjectionKind::DefinitionSet {
                format!(
                    "{schema_base_uri}{}.json#/$defs/{}",
                    producer.schema_id(),
                    escape_pointer_token(&name)
                )
            } else {
                format!("{schema_base_uri}{}.json", producer.schema_id())
            };
            let destination = ExternalDefinitionRef {
                schema_id: producer.schema_id(),
                uri,
            };
            if let Some(existing) = external_refs.insert(local_reference, destination.clone()) {
                if existing != destination {
                    return Err(format!(
                        "Rust schema name `{name}` is claimed by both `{}` and `{}`",
                        existing.schema_id, destination.schema_id
                    ));
                }
            }
        }
    }

    producers
        .into_iter()
        .map(|producer| {
            project(producer, schema_base_uri, &external_refs)
                .map(|schema| (producer.schema_id().to_string(), schema))
        })
        .collect()
}

fn capability_procedure() -> schemars::Schema {
    definition_set(|generator| {
        let _ = generator.subschema_for::<CapabilityPolicy>();
        let _ = generator.subschema_for::<ToolEvidenceContract>();
        let _ = generator.subschema_for::<ToolRecipe>();
        let _ = generator.subschema_for::<OperationCapabilityRequirement>();
    })
}

fn capability_procedure_names() -> Vec<String> {
    vec![
        CapabilityPolicy::schema_name().into_owned(),
        ToolEvidenceContract::schema_name().into_owned(),
        ToolRecipe::schema_name().into_owned(),
        OperationCapabilityRequirement::schema_name().into_owned(),
    ]
}

fn composition_primitives() -> schemars::Schema {
    definition_set(|generator| {
        let _ = generator.subschema_for::<ParticipantRefV1>();
        let _ = generator.subschema_for::<EntryOriginRefV1>();
        let _ = generator.subschema_for::<PathBudgetV1>();
        let _ = generator.subschema_for::<ResponsibilityV1>();
        let _ = generator.subschema_for::<PathInvocationV1>();
        let _ = generator.subschema_for::<NativeOwnerTransitionV1>();
        let _ = generator.subschema_for::<NativeOutcomeV1>();
        let _ = generator.subschema_for::<DestinationConsequenceV1>();
        let _ = generator.subschema_for::<ReconciliationRequirementV1>();
        let _ = generator.subschema_for::<CompiledContentCoordinateV1>();
    })
}

fn composition_primitive_names() -> Vec<String> {
    vec![
        ParticipantRefV1::schema_name().into_owned(),
        EntryOriginRefV1::schema_name().into_owned(),
        PathBudgetV1::schema_name().into_owned(),
        ResponsibilityV1::schema_name().into_owned(),
        PathInvocationV1::schema_name().into_owned(),
        NativeOwnerTransitionV1::schema_name().into_owned(),
        NativeOutcomeV1::schema_name().into_owned(),
        DestinationConsequenceV1::schema_name().into_owned(),
        ReconciliationRequirementV1::schema_name().into_owned(),
        CompiledContentCoordinateV1::schema_name().into_owned(),
    ]
}

#[cfg(test)]
#[path = "../tests/internal/schema_catalog.rs"]
mod tests;
