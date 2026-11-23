//! Compile, inspect, and review Pale configuration through one shared API.
//!
//! This crate builds on `pale-application`, `pale-contracts`, `pale-core` and
//! `pale-ports`, and composes the filesystem, process, Unix-socket and
//! control-store adapters for local work. The CLI, LSP, ACP and A2A interfaces,
//! the agent host and the Go, JavaScript, Python and WebAssembly bindings build
//! on it; MCP reaches it through the CLI.
//!
//! Rust callers and language bindings use these application operations without
//! implementing another compiler or diagnostic policy. Requests contain owned
//! input data. Compilation returns opaque handles that retain validated state;
//! callers use their methods instead of inspecting compiler internals.
//!
//! # First call
//!
//! Read a schema embedded in this binary and handle an unavailable schema:
//!
//! ```
//! use pale_sdk::PaleApi;
//!
//! let api = PaleApi::new()?;
//! let schema = api.contract_schema("diagnostic.v1")?;
//! assert_eq!(schema.registry_id(), "diagnostic.v1");
//! println!("{}: {} bytes", schema.schema_uri(), schema.bytes().len());
//!
//! if let Err(error) = api.contract_schema("not-a-selected-schema") {
//!     eprintln!("{}", error.diagnostic().summary());
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! This lookup reads embedded data. It does not start a runtime, contact a
//! provider, or change files. [`ApiError`] carries a structured [`DiagnosticV1`];
//! use its fields rather than parsing a failure message. Initialization can
//! fail separately with [`ApiInitializationError`].
//!
//! # Configuration workflows
//!
//! - [`PaleApi::compile_portable_workspace`] compiles the workspace documents
//!   supplied by the caller. Its handles support comparisons, pages of changes
//!   and affected items, and explanations tied to those compilations.
//! - [`EffectiveProfileCompilation`] lets callers inspect a profile while authoring.
//!   Snapshots, pages, changes, explanations and visual views use the facts kept
//!   in that compilation. They make no new observations of the machine.
//! - [`PaleApi::inspect_configuration_improvement`] starts a review of a
//!   proposed change. Checking or accepting a proposal does not apply it.
//! - [`OneShotCompilation`] summarizes a complete captured compilation and
//!   prepares repository plans for a separate review and application step.
//!
//! Other operations inspect Package inputs to create or repair a lock without
//! relying on the old lock, validate selected public contracts, run built-in
//! computations without external effects, and check or construct structured
//! diagnostics. [`capability_inventory`] lists the operations each interface
//! may expose.
//!
//! # Resident operations
//!
//! A resident is Pale's long-running local service. Connect to one through a
//! caller-supplied [`ResidentAttachment`]. Its carrier chooses the endpoint and
//! authenticates the peer. Its response checker verifies signatures, authority
//! changes and protection against restoring older state. The SDK does not find,
//! start or administer the resident.
//!
//! Starting an operation returns a durable handle before dispatch. Each advance
//! performs one step within its limits. Observe reads the current signed records;
//! Cancel requests cancellation, but does not confirm that the operation ended.
//!
//! # Local operations and their limits
//!
//! The opt-in `native-local` feature adds filesystem and process methods. Use
//! them to inspect local state, save plans, check saved plans before writing,
//! apply, adopt or remove files under a checked plan, and inspect evidence
//! when a write's outcome is uncertain. Enabling the feature makes these methods
//! available; it runs nothing and grants no permission to act. Local adapters
//! remain internal, and callers still supply any authenticated resident connection.
//! Inspecting the control store reports its storage state, not resident health.
//!
//! The hook API checks that a host event matches its configured route. It stops
//! at that check: it cannot deliver the event to the resident, evaluate policy or
//! perform an action. Portable comparisons likewise describe configuration;
//! changing the active generation, observing a provider, checking a plan for writes
//! and establishing readiness to act each require their own checks.
//!
//! Installation and activation sit outside this API. Compiler internals, native
//! path types, adapters, provider payloads, resident-generation bodies and decoded
//! machine bindings remain internal. Interfaces own terminal rendering and exit
//! status; the Pale application owns the rules for verifying receipts.

mod a2a;
mod actions;
mod authoring;
mod capabilities;
mod contracts;
mod diagnostic;
mod distribution;
mod effective_profile;
mod error;
mod improvement;
#[cfg(feature = "native-local")]
mod local;
mod observation;
mod one_shot;
mod portable_workspace;
mod repository_actions;
mod resident_attachment;
#[cfg(all(
    feature = "native-local",
    any(target_os = "linux", target_os = "macos")
))]
#[path = "resident_attachment/connection.rs"]
mod resident_connection;
#[cfg(all(
    feature = "native-local",
    any(target_os = "linux", target_os = "macos")
))]
pub use resident_connection::{LocalAgentCarrierSource, LocalAgentCarrierSourceError};
#[cfg(all(
    feature = "native-local",
    any(target_os = "linux", target_os = "macos")
))]
#[path = "resident_attachment/configuration.rs"]
mod resident_hook_configuration;
#[cfg(all(
    feature = "native-local",
    any(target_os = "linux", target_os = "macos")
))]
pub use resident_hook_configuration::ResidentHookConfigurationError;
mod resident_hook;
mod resident_observation;
mod resident_operation;

use pale_application::actions::NativeActionRegistry;
use pale_core::records::{RecordSchemaIdentity, SchemaRegistryIdentity};

pub use a2a::{A2aDelegateHandle, A2aDelegateHandleRequest};
pub use actions::{
    NativeActionDescriptor, NativeActionExecution, NativeActionPayloadRequest, NativeActionRequest,
};
pub use authoring::{
    AuthoringCorrection, AuthoringCorrectionKind, AuthoringFieldFact, AuthoringFieldPresence,
    AuthoringIdentity, AuthoringIdentityKind, AuthoringIngressClass, AuthoringIngressLimits,
    AuthoringInspection, AuthoringInspectionRequest, AuthoringInspectionState,
    AuthoringInstancePathSegment, AuthoringLiteral, AuthoringLiteralFact, AuthoringNavigation,
    AuthoringNavigationRequest, AuthoringNodeFact, AuthoringOccurrence, AuthoringOccurrencePage,
    AuthoringOccurrencePageCursor, AuthoringOccurrencePageRequest, AuthoringOccurrenceRole,
    AuthoringPathSegment, AuthoringSourceLocation, AuthoringValueKind,
    PortableWorkspaceSourceInspection, PortableWorkspaceSourceSubject,
    PortableWorkspaceSubjectOccurrences, StandaloneAuthoringRole, AUTHORING_PATH_SEGMENT_LIMIT,
    MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES,
};
pub use capabilities::{
    capability_inventory, ApiCancellationSemantics, ApiCapability, ApiCapabilityClass,
    ApiCapabilityContract, ApiCapabilityEffect, ApiCapabilityId, ApiInvocationMode, ApiLeaf,
};
pub use contracts::{
    ContractBundleIdentity, ContractDocument, ContractSchema, ValidatedContractDocument,
    WorkspaceModuleCandidate,
};
pub use diagnostic::{
    DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticFact, DiagnosticIdentity,
    DiagnosticProjection, DiagnosticProjectionSource, DiagnosticProjector, DiagnosticRetry,
    DiagnosticSchemaCoordinate, DiagnosticSeverity, DiagnosticSource, DiagnosticStage,
    DiagnosticTruncation, DiagnosticV1,
};
#[cfg(feature = "native-local")]
pub use distribution::LocalPackageCheckRequest;
pub use distribution::{
    PackageCatalogInspection, PackageDistributionInspection, PackageDistributionRequest,
    PackageReleaseIdentity,
};
pub use effective_profile::{
    EffectiveAuthoringContinuation, EffectiveAuthoringInspection, EffectiveAuthoringPage,
    EffectiveProfileCompilation, EffectiveProfileCompilationRequest, EffectiveProfileCoordinates,
    EffectiveProfileDeclaredSelection, EffectiveProfileDeclaredSelector,
    EffectiveProfileEffectClass, EffectiveProfileEntry, EffectiveProfileEntryDetail,
    EffectiveProfileExplanation, EffectiveProfileExplanationEvidence,
    EffectiveProfileExplanationHandle, EffectiveProfileFinding, EffectiveProfileModeAxis,
    EffectiveProfileNarrowingReason, EffectiveProfileNativeRealization,
    EffectiveProfileOperationExposure, EffectiveProfilePage, EffectiveProfilePageRequest,
    EffectiveProfileParticipant, EffectiveProfilePathReason, EffectiveProfilePolicyBlock,
    EffectiveProfileProviderBindingRef, EffectiveProfileProviderDisposition,
    EffectiveProfileProviderKind, EffectiveProfileRepositoryAccess,
    EffectiveProfileRequirementDisposition, EffectiveProfileResourceKind,
    EffectiveProfileResponsibility, EffectiveProfileSection, EffectiveProfileSelection,
    InvalidEffectiveProfileExplanationHandle, SourceGuidanceFile, SourceGuidanceHost,
    SourceGuidanceProjection,
};
pub use error::{ApiError, ApiInitializationError};
pub use improvement::{
    ConfigurationImprovementCandidateRequest, ConfigurationImprovementCoordinateChange,
    ConfigurationImprovementDiagnosis, ConfigurationImprovementDisposition,
    ConfigurationImprovementEvaluation, ConfigurationImprovementExplanation,
    ConfigurationImprovementInspection, ConfigurationImprovementPhase,
    ConfigurationImprovementPreview, ConfigurationImprovementPreviewSummary,
    ConfigurationImprovementProposal, ConfigurationImprovementProposalRequest,
    ConfigurationImprovementReviewResult, ConfigurationImprovementSourcePageState,
    ConfigurationImprovementSources, ConfigurationImprovementTerminal,
    PreparedConfigurationImprovementPreview,
};
#[cfg(feature = "native-local")]
pub use local::{
    CompiledInterfaceIdentity, DoctorInspection, DurablePublicationMechanism,
    LocalConfigurationImprovementRequest, LocalControlStorageInspection,
    LocalControlStorageInspectionRejection, LocalControlStorageInspectionRequest,
    LocalControlStorageInspectionState, LocalControlStorageProfileRequirement,
    LocalControlStoreKind, LocalDoctorRequest, LocalMaterializationAction,
    LocalMaterializationAdmissionRequest, LocalMaterializationAdmittedRun,
    LocalMaterializationDrift, LocalMaterializationMachineEffectKind,
    LocalMaterializationOwnershipChange, LocalMaterializationOwnershipObservation,
    LocalMaterializationReceiptObservation, LocalMaterializationReconciliation,
    LocalMaterializationRunOutcome, LocalMaterializationRunResult,
    LocalMaterializationTargetAdmission, LocalMaterializationTargetOutcome,
    LocalMaterializationTargetResult, LocalNativeActionInputRequest, LocalOneShotCompilation,
    LocalOneShotCompilationRequest, LocalOneShotPlan, LocalOneShotPlanCounts,
    LocalOneShotPlanPublication, LocalOneShotPlanRequest, LocalPackageDistributionRequest,
    LocalRepositoryStatus, LocalSourceGuidanceBaseline, LocalSourceGuidanceChange,
    LocalSourceGuidanceCoordinationEffect, LocalSourceGuidanceFile, LocalSourceGuidanceIntent,
    LocalSourceGuidancePlan, LocalSourceGuidancePlanReport, LocalSourceGuidancePlanRequest,
    LocalSourceGuidanceRecoveryDirectory, LocalSourceGuidanceRecoveryDirectoryState,
    LocalSourceGuidanceRecoveryEvidence, LocalSourceGuidanceRecoveryExecutionRequest,
    LocalSourceGuidanceRecoveryFile, LocalSourceGuidanceRecoveryIntent,
    LocalSourceGuidanceRecoveryPlan, LocalSourceGuidanceRecoveryPlanReport,
    LocalSourceGuidanceRecoveryRequest, LocalSourceGuidanceRecoveryResult,
    LocalSourceGuidanceWriteRequest, LocalSourceGuidanceWriteResult, LocalWorkspaceLocator,
    LocalWorkspaceStatus, WorkspaceSourceInspection,
};
pub use observation::{
    ObservationBackpressureRecovery, ObservationCancellation, ObservationCategory,
    ObservationCausalPath, ObservationCausalSelector, ObservationCausalStep, ObservationChange,
    ObservationCursor, ObservationDelta, ObservationDeltaOutcome, ObservationDeltaRequest,
    ObservationFact, ObservationFilter, ObservationFreshness, ObservationFreshnessClass,
    ObservationGeneration, ObservationHealthState, ObservationLens, ObservationLensCursor,
    ObservationLensDisposition, ObservationLensLimits, ObservationLensNode, ObservationLensRequest,
    ObservationNativeState, ObservationPage, ObservationPageCursor, ObservationPageDisposition,
    ObservationPageLimits, ObservationPageRequest, ObservationProcessIntent,
    ObservationProfileAssociation, ObservationProfileContinuation, ObservationProfileError,
    ObservationProfileMapping, ObservationProfilePage, ObservationReadinessState,
    ObservationReconciliationState, ObservationResumeCoordinate, ObservationResynchronization,
    ObservationResynchronizationReason, ObservationRevision, ObservationSnapshot,
    ObservationSnapshotSummary, ObservationStepCompletion, ObservationStepLimit,
    ObservationSubject, ObservationSubjectKind,
};
pub use one_shot::{
    CompilationDocument, ExecutionPathSummary, MachineLocalEffectCandidate, OneShotAdmittedPlan,
    OneShotBaselineEntry, OneShotBaselineObservation, OneShotBoundPlan, OneShotCompilation,
    OneShotCompilationRequest, OneShotCompilationSummary, OneShotObservationTarget,
    OneShotObservationTargets, OneShotPlanCandidate, OneShotPlanKind, OneShotPlanningRequest,
    OneShotPriorMaterialization, OneShotPrivateInputs, RepositorySummary,
};
pub use portable_workspace::{
    InvalidPortableWorkspaceSubjectSelector, OrganizationAttribution, OrganizationAxes,
    OrganizationCommunicationResolution, OrganizationCommunicationResolutionHandle,
    OrganizationComparisonSide, OrganizationConsumerAcceptanceAnswer, OrganizationCorrespondence,
    OrganizationCorrespondenceCursor, OrganizationCorrespondenceState, OrganizationCursor,
    OrganizationDerivedClass, OrganizationDerivedSource, OrganizationDetailCursor,
    OrganizationDetailPage, OrganizationDetailParent, OrganizationDetailRows,
    OrganizationDisclosure, OrganizationEffectCeiling, OrganizationEndpointLookup,
    OrganizationEntry, OrganizationEvidenceRequirement, OrganizationFactState,
    OrganizationGenealogyHandle, OrganizationGenealogyLink, OrganizationGroupRef,
    OrganizationInspectionCoordinate, OrganizationInterval, OrganizationLensRight,
    OrganizationNativeReadBackAnswer, OrganizationNode, OrganizationPage,
    OrganizationPlacementAnswer, OrganizationProjectionClass, OrganizationProviderAcceptanceAnswer,
    OrganizationProviderCompletionAnswer, OrganizationRecordHandle, OrganizationRef,
    OrganizationRelationClass, OrganizationRelationLocality, OrganizationResidence,
    OrganizationResidenceClass, OrganizationResourceAnswer, OrganizationResponsibility,
    OrganizationResponsibilityLifecycle, OrganizationResponsibilityPlacement,
    OrganizationResponsibilityProjection, OrganizationResponsibilityProjectionHandle,
    OrganizationRoleRef, OrganizationRow, OrganizationScopeRef, OrganizationServiceAssignment,
    OrganizationServiceAssignmentHandle, OrganizationServiceRef, OrganizationSettlementAnswer,
    OrganizationSourceCursor, OrganizationSourceMapping, OrganizationSourcePage,
    OrganizationStructuralKind, OrganizationStructuralSelection, OrganizationTreeRef,
    PortableWorkspaceCausalExplanation, PortableWorkspaceCausalStep, PortableWorkspaceChangeKind,
    PortableWorkspaceChangeRoot, PortableWorkspaceChangeRootPage, PortableWorkspaceComparison,
    PortableWorkspaceComparisonSubjectCursor, PortableWorkspaceComparisonSubjectPage,
    PortableWorkspaceComparisonSummary, PortableWorkspaceCompilation,
    PortableWorkspaceCompilationSummary, PortableWorkspaceContributingRootsCursor,
    PortableWorkspaceContributingRootsPage, PortableWorkspaceContributingRootsState,
    PortableWorkspaceDependencyCursor, PortableWorkspaceDependencyPage,
    PortableWorkspaceDependentCursor, PortableWorkspaceDependentPage,
    PortableWorkspaceDerivationRule, PortableWorkspaceDocument, PortableWorkspaceExplanation,
    PortableWorkspaceExplanationFrontier, PortableWorkspaceExplanationState, PortableWorkspaceFact,
    PortableWorkspaceImpact, PortableWorkspaceImpactPage, PortableWorkspaceImpactState,
    PortableWorkspaceProfileContext, PortableWorkspaceProvenance, PortableWorkspaceRequest,
    PortableWorkspaceSubject, PortableWorkspaceSubjectComparison,
    PortableWorkspaceSubjectInspection, PortableWorkspaceSubjectInspectionCursor,
    PortableWorkspaceSubjectKind, PortableWorkspaceSubjectSelector, PortableWorkspaceSubjectValues,
    PortableWorkspaceValueComparison, RecipientQualifiedComparisonSubjectCursor,
    RecipientQualifiedComparisonSubjectPage, RecipientQualifiedContributingRootsCursor,
    RecipientQualifiedContributingRootsPage, RecipientQualifiedDependencyCursor,
    RecipientQualifiedDependencyPage, RecipientQualifiedOrganizationCorrespondence,
    RecipientQualifiedOrganizationCorrespondenceCursor,
    RecipientQualifiedOrganizationCorrespondenceState, RecipientQualifiedOrganizationCursor,
    RecipientQualifiedOrganizationEntry, RecipientQualifiedOrganizationPage,
    RecipientQualifiedOrganizationRecordHandle, RecipientQualifiedOrganizationSourceCursor,
    RecipientQualifiedOrganizationSourceMapping, RecipientQualifiedOrganizationSourcePage,
    RecipientQualifiedSubjectInspection, RecipientQualifiedSubjectInspectionCursor,
    RecipientQualifiedWorkspaceAccessError, RecipientQualifiedWorkspaceComparisonInspection,
    RecipientQualifiedWorkspaceComparisonOwnerMismatch, RecipientQualifiedWorkspaceInspection,
    RecipientQualifiedWorkspaceInspectionError, RecipientQualifiedWorkspaceOwnerMismatch,
    RecipientQualifiedWorkspacePairSummary, MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES,
    MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES, MAX_DEPENDENT_PAGE_INPUT_COMPARISONS,
    MAX_ORGANIZATION_COPY_BYTES,
};
pub use repository_actions::{
    AdmittedRepositoryNativeActions, RepositoryNativeActionCatalogRequest,
};
pub use resident_attachment::{
    AuthenticatedResidentCarrier, AuthenticatedResidentCarrierFailure,
    AuthenticatedResidentCarrierSource, AuthenticatedResidentCarrierSourceFailure,
    ResidentAttachment, ResidentAttachmentBuildError, ResidentResponseAdmission,
    ResidentResponseAdmissionFailure, RESIDENT_RECORD_BYTE_LIMIT,
};
pub use resident_hook::{
    CodexCommandObservation, CodexCommandTerminalStatus, PreparedResidentNativeTool,
    ResidentHookAdmission, ResidentHookOutput, ResidentHookRequest, ResidentNativeRequestSigner,
    ResidentNativeRequestSigningFailure, ResidentNativeRequestSigningInput,
    StartedResidentNativeTool,
};
pub use resident_observation::{
    OperationHandleLocator, ResidentOperationObservation, ResidentOperationObservationPreflight,
    ResidentOperationObservationRequest,
};
pub use resident_operation::{
    ResidentConsultAdvanceOutcome, ResidentConsultAdvanceRequest, ResidentConsultStartRequest,
    ResidentDelegateAdvanceOutcome, ResidentDelegateAdvanceRequest, ResidentDelegateStartRequest,
    ResidentInvokeAdvanceOutcome, ResidentInvokeAdvanceRequest, ResidentInvokeStartRequest,
    ResidentOperationCancellationDisposition, ResidentOperationCancellationOutcome,
    ResidentOperationCancellationRequest, ResidentOperationUnsupportedReason,
};

/// Immutable API handle for Pale's checked contracts and built-in computations.
///
/// Construction verifies the complete embedded contract registry and computation
/// catalog. The handle holds no filesystem, process, network or provider access,
/// and no permission to install, activate or perform effects. With `native-local`,
/// explicit method calls can create internal adapters for local checks and writes
/// within their declared limits. Those adapters and machine paths never become part
/// of the handle's public state.
pub struct PaleApi {
    /// Exact embedded bundle coordinate captured at construction.
    schema_registry: SchemaRegistryIdentity,
    /// Self-carried identity used for every API-generated diagnostic.
    diagnostic_identity: RecordSchemaIdentity,
    /// Verified embedded member count narrowed to the portable DTO width.
    schema_count: u32,
    /// Closed effect-free registry behind [`Self::invoke_native_action`].
    actions: NativeActionRegistry,
}

impl PaleApi {
    /// Create an API handle from the embedded Pale contract bundle.
    ///
    /// # Errors
    ///
    /// Returns [`ApiInitializationError`] if the generated registry is inconsistent,
    /// its member count exceeds the SDK's supported range, the diagnostic contract
    /// lacks its schema identity, or a built-in computation contradicts its schema.
    pub fn new() -> Result<Self, ApiInitializationError> {
        let schema_registry = pale_contracts::schema_registry_identity()
            .map_err(ApiInitializationError::contracts)?;
        let diagnostic_identity =
            pale_contracts::record_schema_identity(pale_core::records::DIAGNOSTIC_V1_SCHEMA_ID)
                .map_err(ApiInitializationError::contracts)?;
        let schema_count = u32::try_from(
            pale_contracts::schema_count().map_err(ApiInitializationError::contracts)?,
        )
        .map_err(|_| ApiInitializationError::schema_count_overflow())?;
        let actions = NativeActionRegistry::new().map_err(ApiInitializationError::actions)?;
        Ok(Self {
            schema_registry,
            diagnostic_identity,
            schema_count,
            actions,
        })
    }
}
