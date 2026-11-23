//! Typed configuration, compiled-profile, runtime, and evidence records.
//!
//! These Rust types own Pale wire shape and structural constraints. The
//! descriptor manifest separately owns registration, canonical schema identity,
//! lifecycle, and active-set membership; deterministic JSON Schemas are sealed
//! consumer projections of both authorities. Catalog members, governing stages,
//! and their order remain declarative inputs rather than enum variants or
//! hardcoded execution order.
//!
//! Construction or deserialization establishes record shape only. It does not
//! establish a claimed schema identity, digest, authority, or observed effect;
//! the producing or admitting layer must verify those relationships.

/// Declare a closed enum whose Rust value, display value, and serialized value
/// all derive from one explicit wire spelling.
macro_rules! wire_enum {
    (
        $(#[$meta:meta])*
        $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident => $wire:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
            schemars::JsonSchema,
        )]
        pub enum $name {
            $(
                $(#[$variant_meta])*
                #[serde(rename = $wire)]
                $variant
            ),+
        }

        impl $name {
            /// Stable wire spelling used by canonical records and schemas.
            #[must_use]
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

/// Declare a closed wire enum whose exhaustive ordered inventory is generated
/// from the same variant list as its type and wire spellings.
macro_rules! wire_enum_all {
    (
        $(#[$meta:meta])*
        $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident => $wire:literal
            ),+ $(,)?
        }
    ) => {
        wire_enum!(
            $(#[$meta])*
            $name {
                $(
                    $(#[$variant_meta])*
                    $variant => $wire
                ),+
            }
        );

        impl $name {
            /// Borrow every declared variant in stable declaration order.
            ///
            /// Domain-specific public methods give this inventory its semantic
            /// name; this generated helper prevents those methods from keeping
            /// a second list or exposing an incidental array cardinality.
            pub(crate) const fn declared_variants() -> &'static [Self] {
                &[$(Self::$variant),+]
            }
        }
    };
}

mod action;
mod binding;
mod bounded_tool;
mod capability;
mod composition;
mod computed;
mod connector;
mod diagnostic;
mod digests;
mod doctrine_entry;
mod enums;
mod json_pointer;
mod lifecycle;
mod native_tool;
mod operation;
mod operation_contract;
mod order;
mod package;
mod package_data;
mod package_page;
mod package_use;
mod policy;
mod release;
mod resident;
mod role;
mod run_mode;
mod schema_identity;
mod sealed_artifact;
mod stack;
mod tool_matcher;
mod workspace_language;

pub use action::{
    native_action_host_tool_name, native_action_tool_name, ActionBoundary, ActionContract,
    ActionEffect, ActionError, ActionErrorCode, ActionExecution, ActionInput, ActionInputKind,
    ActionOutput, ActionOutputKind, ActionReceipt, ActionRequest, ActionResult,
    ClassifiedTraceEvent, ComplianceCheckResult, CompliancePromotion, ComplianceStep,
    ComplianceTemporalRule, ComplianceTraceGradeInput, ComplianceTraceGradeOutput, ConfigInvariant,
    ConfigInvariantEvidence, ConfigInvariantExpectation, ConfigInvariantResult,
    ConfigInvariantScanInput, ConfigInvariantScanOutput, ConfigTextRecord, ConvergedFinding,
    CostAggregateInput, CostAggregateOutput, CostAggregateRow, CostSnapshot, EvidenceMatch,
    EvidenceRule, EvidenceSelectionInput, EvidenceSelectionMode, EvidenceSelectionOutput,
    FindingDisposition, LoopEvent, LoopRecommendation, LoopStatusInput, LoopStatusOutput,
    LoopToolKind, NativeActionCatalogV1, OverdueWakeup, PathInventoryEntry, PathInventoryInput,
    PathInventoryOutput, PathStyle, RankedCandidate, RankingCandidate, RankingScore, RankingWeight,
    RepositoryNativeActionCatalogV1, ReviewConvergenceInput, ReviewConvergenceOutput,
    ReviewFinding, ReviewRecommendation, ReviewSeverity, RubricCategory, RubricCategoryScore,
    RubricCheckResult, RubricEvaluationInput, RubricEvaluationOutput, RubricFailure,
    ServiceDefinition, ServiceDefinitionInput, ServiceDefinitionOutput, ServiceLaunch,
    SkillHealthInput, SkillHealthOutput, SkillHealthRow, SkillHealthSeries, SkillHealthTrend,
    SkillRunObservation, SkillWindowRate, StaleToolUse, VerifierDimension, WeightedRankingInput,
    WeightedRankingOutput, COMPLIANCE_TRACE_GRADE_ACTION_ID, CONFIG_INVARIANT_SCAN_ACTION_ID,
    COST_AGGREGATE_ACTION_ID, EVIDENCE_SELECTION_ACTION_ID, LOOP_STATUS_REDUCE_ACTION_ID,
    NATIVE_ACTION_CATALOG_V1_SCHEMA, NATIVE_ACTION_CATALOG_V1_SCHEMA_VERSION,
    NATIVE_ACTION_SERVER_NAME, PATH_INVENTORY_ACTION_ID, REVIEW_BLOCKING_CONFIDENCE_MICROS,
    REVIEW_CONVERGENCE_REDUCE_ACTION_ID, RUBRIC_EVALUATION_ACTION_ID,
    SERVICE_DEFINITION_MAX_ARGUMENTS_V1, SERVICE_DEFINITION_MAX_ARGUMENT_BYTES_V1,
    SERVICE_DEFINITION_MAX_DEPENDENCIES_V1, SERVICE_DEFINITION_MAX_KIND_BYTES_V1,
    SERVICE_DEFINITION_MAX_NAME_BYTES_V1, SERVICE_DEFINITION_MAX_PORTS_V1,
    SERVICE_DEFINITION_MAX_PROGRAM_BYTES_V1, SERVICE_DEFINITION_MAX_SERVICES_V1,
    SERVICE_DEFINITION_NORMALIZE_ACTION_ID, SKILL_HEALTH_AGGREGATE_ACTION_ID,
    WEIGHTED_RANKING_ACTION_ID,
};
pub use binding::{
    AuthorityAccess, AuthorityAccessClass, BoundaryCapability, CapabilitiesFile,
    DistributionLockFile, DistributionMeta, DurableEvidenceBinding, EvidenceRetentionSeconds,
    MachineBindingFile, MachineMeta, MaterializationLineageBinding, NativeHarnessBinding,
    ProviderRepositoryProjectionV1, ReadingOrderEntry, RepositoryCheckoutBindingV1,
    RepositoryCommandCapabilities, RouteRecordLimit, SessionModel, SessionRuntimeSelection,
    SessionStateBinding, SessionTtlSeconds, SurfaceExecutableBinding, UserPreferences,
    WorkspaceManifestFile, WorkspaceMeta, MAX_EXPERIENCE_COMPOSITION_SOURCES,
    MAX_ORGANIZATIONAL_TRANSLATION_SOURCES, MAX_RESPONSIBILITY_DERIVATION_SOURCES,
};
pub use bounded_tool::{BoundedTool, ToolCatalog, ToolFile};
pub use capability::{
    CapabilityBinding, CapabilityBindingsV1, CapabilityFallback, CapabilityIdV1,
    CapabilityNarrowingV1, CapabilityPolicy, CapabilityProviderBinding, CapabilityRequirementIdV1,
    CapabilityRequirementPolicy, CapabilityUseKind, DegradedOutcomeIdV1,
    InvalidCapabilityProcedureIdV1, OperationBinding, OperationBindingEntryV1,
    OperationCapabilityRequirement, OperationSelection, ProofCeilingIdV1, ProviderToolBindingV1,
    ResolvedCapabilityFallbackV1, ResolvedCapabilityProviderV1,
    ResolvedCapabilityRequirementPolicyV1, ResolvedCapabilityUseV1, ResolvedToolRecipeV1,
    SemanticSelection, ToolContractIdV1, ToolEvidenceContract, ToolOperationIdV1, ToolRecipe,
    MAX_CAPABILITY_POLICY_AXIS_MEMBERS,
};
pub use composition::{
    AdapterPluginRefV1, CapabilityEvidenceCoordinateV1, CommunicationActorHandleV1,
    CompiledContentCoordinateV1, CompiledContentIdentityV1, CompositionEdgeV1,
    CompositionGraphKindV1, CompositionGraphViewV1, CompositionReasonPathV1,
    CompositionReasonStepV1, ContinuityContractV1, ContinuityIdentityV1, ContractSchemaIdentityV1,
    DeploymentActivationIdentityV1, DestinationConsequenceV1, EntryContractV1, EntryOriginRefV1,
    EvidenceSettlementIdentityV1, ExperienceCompositionDeclarationsV1,
    ExperienceCompositionPlanPreimageV1, ExperienceCompositionPlanV1, ExperiencePathPreimageV1,
    ExperiencePathV1, InvocationIdentityV1, LogicalProviderInstanceRefV1,
    NativeAuthorityOriginRefV1, NativeDataGenerationIdentityV1, NativeEffectIdentityV1,
    NativeOutcomeV1, NativeOwnerTransitionV1, PaleAdmissionGenerationIdentityV1,
    ParticipantProviderManifestV1, ParticipantRefV1, PathBudgetV1, PathInvocationV1,
    PhysicalDeploymentRefV1, ProviderArtifactCoordinatesV1, ProviderAuthorityCoordinatesV1,
    ProviderBehaviorCoordinatesV1, ProviderBindingIdentityV1, ProviderBindingPreimageV1,
    ProviderBindingRefV1, ProviderBindingV1, ProviderCompatibilityDeclarationV1,
    ProviderConformanceCoordinatesV1, ProviderExchangeCoordinatesV1, ProviderFamilyRefV1,
    ProviderGenerationCoordinatesV1, ProviderGenerationPreimageV1, ProviderLifecycleCoordinatesV1,
    ProviderMappingCoordinatesV1, ProviderProtocolCoordinatesV1, ReconciliationRequirementV1,
    RequestTranslationV1, ResponsibilityContractV1, ResponsibilityPathIdentityV1, ResponsibilityV1,
    ResultTranslationV1, SettlementContractV1, SettlementDispositionV1, SourceContentIdentityV1,
    TerminationContractV1, TerminationKindV1, WorkspaceCompositionSourceV1,
    EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA, EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_ID,
    EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_VERSION, RESPONSIBILITY_CONTRACT_V1_SCHEMA_ID,
    WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA, WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA_ID,
    WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA_VERSION,
};
pub use computed::{
    harness_injection_path, harness_projection_manifest_path, provider_binding_projection_root,
    AgentSurfacePlanPreimageV1, AgentSurfacePlanSealErrorV1, AgentSurfacePlanV1, BaselineEntry,
    BaselineStateV1, DeferredEffect, EffectiveProjectProfilePreimageV1,
    EffectiveProjectProfileSealErrorV1, EffectiveProjectProfileV1, ExistingMachineLocalFileV1,
    FilesystemDeviceIdV1, FilesystemInodeIdV1, GeneratedAgentSurfaceManifestPreimageV1,
    GeneratedAgentSurfaceManifestV1, HarnessProjectionManifestV1, HookContextV1,
    InvalidUnixFileMode, MachineLocalDirectoryIdentity, MachineLocalEffectKind,
    MachineLocalEffectObservationV1, MachineLocalEffectTransitionV1, MachineLocalEffectV1,
    MachineLocalFileIdentityV1, MachineLocalPostFileV1, MachineLocalReceiptOperationV1,
    MachineLocalRootIdentity, ManagedFrontDoorDigest, MaterializationLineagePreimageV1,
    MaterializationLineageV1, MaterializationPlanPreimageV1, MaterializationPlanV1,
    MaterializationReceiptPreimageV1, MaterializationReceiptSealErrorV1,
    MaterializationReceiptTransitionV1, MaterializationReceiptV1, MaterializationStatePreimageV1,
    MaterializationStateTransitionV1, MaterializationStateV1, Operation, PlannedFileV1,
    PriorMaterializationStateV1, ProfileRepository, ReceiptOperation, RepositoryVerifierBinding,
    ResolvedProjectInstancePreimageV1, ResolvedProjectInstanceV1, RunResultPreimageV1, RunResultV1,
    RunTargetResult, SealedRunMode, SurfaceCreationObservationV1, SurfaceDigest,
    SurfaceExecutableIdentityV1, SurfaceOperationObservationV1, SurfaceOperationTransitionV1,
    UnchangedDispositionV1, UnixFileMode, AGENT_SURFACE_PLAN_V1_SCHEMA,
    EFFECTIVE_PROJECT_PROFILE_SCHEMA, EFFECTIVE_PROJECT_PROFILE_SCHEMA_VERSION,
    HARNESS_PROJECTION_SCHEMA, HARNESS_PROJECTION_SCHEMA_VERSION, MATERIALIZATION_LINEAGE_SCHEMA,
    MATERIALIZATION_LINEAGE_SCHEMA_VERSION, MATERIALIZATION_PLAN_V1_SCHEMA,
    MATERIALIZATION_PLAN_V1_SCHEMA_VERSION, MATERIALIZATION_RECEIPT_V1_SCHEMA,
    MATERIALIZATION_RECEIPT_V1_SCHEMA_VERSION, MATERIALIZATION_STATE_PATH,
    MATERIALIZATION_STATE_SCHEMA, MATERIALIZATION_STATE_SCHEMA_VERSION,
    RESOLVED_PROJECT_INSTANCE_SCHEMA, RUN_RESULT_V1_SCHEMA, RUN_RESULT_V1_SCHEMA_VERSION,
    SURFACE_INTERFACE_PROTOCOL,
};
pub use connector::{ConnectorCatalog, ConnectorContract, ConnectorFile};
pub use diagnostic::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticFactV1Wire,
    DiagnosticGenerationCorrelationV1Wire, DiagnosticReceiptCorrelationV1Wire,
    DiagnosticRequestCorrelationV1Wire, DiagnosticRetryV1, DiagnosticSchemaCoordinateV1Wire,
    DiagnosticSeverityV1, DiagnosticSourceV1Wire, DiagnosticStageV1, DiagnosticSubjectV1Wire,
    DiagnosticTruncationV1Wire, DiagnosticV1Wire, DIAGNOSTIC_V1_SCHEMA, DIAGNOSTIC_V1_SCHEMA_ID,
    DIAGNOSTIC_V1_SCHEMA_VERSION,
};
pub use digests::{
    schema_set_digest, A2aRouteAdmissionDigest, A2aTaskBindingDigest, A2aTaskObservationDigest,
    ActionContractDigest, ActionEnvelopeDigest, ActionReceiptDigest, ActionRequestDigest,
    ActionResultDigest, ActivationFenceDigest, ActiveGenerationHeadDigest, AdapterFailureDigest,
    AdvertisedGenerationHeadDigest, ArtifactDigest, CapabilityResultDigest,
    CapabilityRevocationCheckpointDigest, CapabilitySessionGrantDigest,
    CapabilitySessionGrantRevocationDigest, CheckpointProfileDigest,
    CheckpointSigningAuthorityDigest, CheckpointSigningAuthorityHeadsRootDigest,
    CommonRequestEnvelopeDigest, CompatibilityProfileDigest, CompilationInputDigest,
    CompiledGenerationDigest, ConfidentialityPolicyDigest, ContractSetDigest,
    ControlStateCheckpointDigest, DecisionEvidenceDigest, DeploymentAcceptanceDigest,
    DeploymentAcceptanceRevocationDigest, DeploymentAcceptanceRevocationHeadsRootDigest,
    DeploymentBindingDigest, DeploymentBindingProposalDigest,
    DeploymentControlStoreGenesisReceiptDigest, DeploymentEnvironmentDigest,
    DeploymentRuntimeAuthorityDigest, DeploymentRuntimeAuthorityHeadsRootDigest,
    DeploymentTermsDigest, DistributionLockDigest, DistributionLockDigestError,
    DistributionReleaseDigest, EffectAdmissionDigest, EffectConflictClaimDigest,
    EffectDispatchIntentDigest, EffectReceiptDigest, EventAcknowledgementDigest,
    EventReferenceDigest, ExecutableDigest, ExecutableIdentityDigest, ExecutionOwnerFenceDigest,
    ExperienceCompositionPlanDigest, ExperienceCompositionSourceDigest,
    FederationSpaceHomeHeadsRootDigest, GenealogyEdgeDigest, HarnessActivityIdentity,
    InstanceDigest, InteractionOccurrenceDigest, InvalidDigest, InvalidHostIdentity,
    LocalAdminAuthorityDigest, LocalAdminCandidateSetDigest, LocalAdminCommandDigest,
    LocalAdminReceiptDigest, LocalRouteBindingDigest, MachineOverlayDigest, ManifestDigest,
    MaterializationClosureDigest, MaterializationLineageDigest, MaterializationStateDigest,
    NodeDelegationHeadsRootDigest, NodeDelegationRevocationHeadsRootDigest, NormativePackDigest,
    OneShotCompilationDigest, OperationBindingDigest, OperationCatalogDigest,
    OperationContractDigest, OperationHandleDigest, OperationLeaseDigest,
    OperationObservationDigest, OrganizationalTranslationClosureDigest,
    OwnerRecordSigningAuthorityHeadsRootDigest, OwnerSignerPublicationFrontierHeadsRootDigest,
    PackageDigest, PackageSemanticIrDigest, PlanDigest, PrincipalAdmittedUniverseDigest,
    PrincipalBootstrapReceiptDigest, PrincipalKeyTransitionDigest, PrincipalRecordDigest,
    PrincipalRecoveryPolicyDigest, PrincipalRecoveryProofDigest,
    PrincipalResidentWriterFenceDigest, PrincipalResidentWriterTakeoverReceiptDigest,
    PrincipalResidentWriterTerminalReceiptDigest, PrincipalTrustCheckpointDigest,
    ProcessingRoleDigest, ProducerIncarnationAuthorityDigest, ProducerIncarnationDigest,
    ProducerIncarnationHeadsRootDigest, ProducerIncarnationRetirementDigest, ProfileDigest,
    ProviderBindingClosureDigest, PublishedGenerationHeadDigest, ReceiptDigest,
    ReconciliationCaseDigest, ReconciliationCustodyDigest, ReconciliationFrontierDigest,
    ReconciliationRequestDigest, ReconciliationResultDigest, RepositoryRenderDigest,
    ResourceContinuityProfileDigest, ResponsibilityDerivationClosureDigest, RunDigest,
    SchemaDigest, SchemaManifestDigest, SchemaSetDigest, SchemaSetDigestError, SchemaSetMember,
    SessionAdmissionLeaseDigest, SessionIdentityDigest, TargetRootDigest, ToolAuthorizationDigest,
    TurnIdentityDigest, WorkspaceAdmissionEvidenceDigest, WorkspaceComponentDigest,
    WorkspaceDerivationReceiptDigest, WorkspaceImportSurfaceDigest, WorkspaceModuleDigest,
    WorkspaceProtectedDeliveryProofDigest, WorkspacePublisherProofDigest,
    WorkspaceResolutionLockDigest, WorkspaceSemanticIrDigest, WorkspaceUniverseLockDigest,
};
pub use doctrine_entry::{
    DoctrineCatalog, DoctrineEntry, DoctrineFile, DoctrinePlacement, InvalidOfficialSourceUrl,
    OfficialReference, OfficialSourceUrl,
};
pub use enums::{
    Approval, Availability, BindingState, CapabilityProvider, CapabilityUnavailability,
    ChildPropagation, EffectClass, FailurePosture, LifecycleAction, LifecycleEvent, Mechanics,
    ModeAxis, ModeDefaultSource, ModeScope, OperationSupport, Outcome, PlanKind, PredicateKind,
    ProviderExecutionAvailability, ReasoningEffort, Requirement, StateScope, SurfaceAction,
    SurfaceOwnership, ToolObservability,
};
pub use json_pointer::{InvalidNonRootJsonPointer, NonRootJsonPointer};
pub use lifecycle::{
    AdmittedLifecycleBehaviorV1, InvalidLifecycleBehavior, InvalidLifecycleBehaviorIdV1,
    LifecycleBehavior, LifecycleBehaviorCatalog, LifecycleBehaviorIdV1, LifecycleFallbackV1,
};
pub use native_tool::{NativeToolRequestV1, NATIVE_TOOL_REQUEST_V1_SCHEMA_ID};
pub use operation::{OnDemandOperation, OperationArchetype, OperationCatalog, OperationsFile};
pub use operation_contract::{
    CatalogOperationStateV1, HostCommandConstraint, HostCommandSource, HostToolBinding,
    HostToolEffectBinding, HostedProviderExecutionV1, InvalidOperationContractFamilyV1,
    NativeRustProviderExecutionV1, OperationCapabilityContractV1,
    OperationCapabilityProviderStateV1, OperationCatalogEntryV1,
    OperationContractCatalogPreimageV1, OperationContractCatalogV1, OperationContractMechanicsV1,
    OperationContractPreimageV1, OperationContractV1, OperationDispatchV1,
    OperationRouteBindingPreimageV1, OperationRouteBindingV1, OperationSemanticDependencyV1,
    RouteCapabilityProviderV1, SemanticDependencyKind,
};
pub use order::{
    meet_capability_narrowing, AccumulatedGoverningObligationV1, AdmittedCapabilityPolicyV1,
    AdmittedExclusiveAuthorityClaimV1, AdmittedGoverningOrderV1, AdmittedScopedOrderRelationV1,
    EffectiveGoverningCeilingV1, GoverningAlgebraErrorV1, GoverningConstraintV1,
    GoverningExclusiveAuthorityClaimV1, GoverningNarrowingV1, GoverningObligationV1,
    GoverningOrderFile, GoverningOrderV1, ResolvedGoverningConstraintV1,
    ResolvedGoverningContributionV1, ResolvedGoverningNarrowingV1, ResolvedGoverningObligationV1,
    ScopedOrderRelationV1, SemanticOrderPurposeV1, MAX_GOVERNING_RELATIONS,
};
pub use package::{
    AgentSurfacePackageV1, FamilyFile, InvalidPackageIdV1, InvalidPackageManifestV1,
    InvalidPackageResourceIdV1, PackageDependencyV1, PackageIdV1, PackageLicense, PackageMeta,
    PackageResourceExposureV1, PackageResourceIdV1, PackageResourceKindV1, PackageResourceLinkV1,
    PackageResourceRefV1, PackageResourceSelectorV1, PackageResourceV1, PackageSourceRoleV1,
    PackageSourceV1, PolicyFamilyV1, MAX_PACKAGE_DEPENDENCIES, MAX_PACKAGE_ID_BYTES,
    MAX_PACKAGE_RESOURCES, MAX_PACKAGE_SOURCES,
};
pub use package_data::PackageDataV1;
pub use package_page::{
    PackageGuideV1, PackagePageV1, PackagePolicyV1, PackageRoleV1, PackageStackV1, PackageTaskV1,
};
pub use package_use::{GoverningResourceLinkV1, PackageResourceUseV1};
pub use policy::{InvalidRunPolicy, InvalidRunPolicyIdV1, RunPolicyEntry, RunPolicyIdV1};
pub use release::{
    canonicalize_release_closures, validate_release_closures, CanonicalReleaseClosuresV1,
    InvalidNormativePackIdV1, InvalidReleaseLabelV1, NormativePackAttestorV1, NormativePackIdV1,
    NormativePackReleaseV1, NormativePackVersionV1, PackageReleaseV1, PackageVersionV1,
    ReleaseClosureErrorV1, MAX_RELEASE_LABEL_BYTES, MAX_SELECTED_NORMATIVE_PACK_RELEASES,
    MAX_SELECTED_PACKAGE_RELEASES,
};
pub use resident::*;
pub use role::{RoleArchetype, RoleCatalog, RolesFile, SpecialistRole};
pub use run_mode::{RunMode, RunModeCatalog, RunModesFile};
pub use schema_identity::{
    ExpiringRecordContract, InvalidSchemaId, MaterializationContractIdentity, RecordSchemaIdentity,
    SchemaId, SchemaRegistryIdentity,
};
pub use sealed_artifact::SealedArtifactRef;
pub use stack::{Stack, StackCatalog, StacksFile};
pub use tool_matcher::{InvalidToolCallMatcher, ToolCallMatcher};
pub use workspace_language::*;
