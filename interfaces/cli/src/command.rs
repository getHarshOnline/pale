//! Run typed Pale commands from the CLI or an eligible protocol transport.
//!
//! [`CommandDispatcher`] calls the Rust SDK; it does not parse command-line
//! arguments or render results. Requests either supply captured inputs or move
//! retained SDK values between steps. Protocol transports expose only their
//! selected commands, not every variant of [`CommandRequest`].

mod configuration_improvement;
mod diagnostic;
#[cfg(feature = "native-local")]
mod local_materialization;

pub use configuration_improvement::{
    ConfigurationImprovementCommand, ConfigurationImprovementOutcome,
};
pub use diagnostic::DiagnosticPresentation;
#[cfg(feature = "native-local")]
pub use local_materialization::{
    LocalMaterializationCommand, LocalMaterializationOutcome,
    LocalMaterializationSettlementDisposition, LocalSavedPlanPublication,
    LocalSavedPlanPublicationDisposition, LocalSavedPlanPublicationRequest,
};

pub use pale_sdk::{
    capability_inventory, ApiCapability, ApiCapabilityId, ApiError, ApiLeaf,
    AuthenticatedResidentCarrier, AuthenticatedResidentCarrierFailure,
    AuthenticatedResidentCarrierSource, AuthenticatedResidentCarrierSourceFailure,
    AuthoringIdentity, AuthoringIngressClass, AuthoringIngressLimits, AuthoringInspection,
    AuthoringInspectionRequest, AuthoringNavigation, AuthoringNavigationRequest,
    AuthoringPathSegment, CompilationDocument, ConfigurationImprovementCandidateRequest,
    ConfigurationImprovementDisposition, ConfigurationImprovementPreviewSummary,
    ConfigurationImprovementProposalRequest, ConfigurationImprovementReviewResult,
    ConfigurationImprovementSourcePageState, ConfigurationImprovementSources,
    ConfigurationImprovementTerminal, ContractDocument, DiagnosticDisposition, DiagnosticV1,
    EffectiveAuthoringInspection, EffectiveProfileCompilationRequest, EffectiveProfileExplanation,
    EffectiveProfileExplanationHandle, EffectiveProfilePage, EffectiveProfilePageRequest,
    EffectiveProfileSection, MachineLocalEffectCandidate, NativeActionDescriptor,
    NativeActionExecution, NativeActionPayloadRequest, ObservationBackpressureRecovery,
    ObservationCancellation, ObservationCategory, ObservationCausalPath, ObservationCausalSelector,
    ObservationChange, ObservationCursor, ObservationDeltaOutcome, ObservationDeltaRequest,
    ObservationFact, ObservationFilter, ObservationFreshness, ObservationFreshnessClass,
    ObservationGeneration, ObservationHealthState, ObservationLens, ObservationLensDisposition,
    ObservationLensLimits, ObservationLensNode, ObservationLensRequest, ObservationNativeState,
    ObservationPageDisposition, ObservationPageLimits, ObservationPageRequest,
    ObservationProcessIntent, ObservationReadinessState, ObservationReconciliationState,
    ObservationResumeCoordinate, ObservationResynchronization, ObservationRevision,
    ObservationSnapshot, ObservationSnapshotSummary, ObservationSubject, ObservationSubjectKind,
    OneShotAdmittedPlan, OneShotBaselineEntry, OneShotBaselineObservation, OneShotBoundPlan,
    OneShotCompilationRequest, OneShotCompilationSummary, OneShotObservationTargets,
    OneShotPlanCandidate, OneShotPlanKind, OneShotPlanningRequest, OneShotPriorMaterialization,
    OneShotPrivateInputs, OperationHandleLocator, PortableWorkspaceChangeRootPage,
    PortableWorkspaceComparisonSummary, PortableWorkspaceCompilationSummary,
    PortableWorkspaceDocument, PortableWorkspaceExplanation, PortableWorkspaceImpactPage,
    PortableWorkspaceRequest, PortableWorkspaceSubjectSelector, ResidentAttachment,
    ResidentAttachmentBuildError, ResidentConsultAdvanceOutcome, ResidentConsultAdvanceRequest,
    ResidentConsultStartRequest, ResidentDelegateAdvanceOutcome, ResidentDelegateAdvanceRequest,
    ResidentDelegateStartRequest, ResidentHookRequest, ResidentInvokeAdvanceOutcome,
    ResidentInvokeAdvanceRequest, ResidentInvokeStartRequest,
    ResidentOperationCancellationDisposition, ResidentOperationCancellationOutcome,
    ResidentOperationCancellationRequest, ResidentOperationObservation,
    ResidentOperationObservationRequest, ResidentResponseAdmission,
    ResidentResponseAdmissionFailure, StandaloneAuthoringRole,
};

#[cfg(feature = "native-local")]
pub use pale_sdk::{
    CompiledInterfaceIdentity, LocalConfigurationImprovementRequest, LocalControlStorageInspection,
    LocalControlStorageInspectionRequest, LocalNativeActionInputRequest, LocalOneShotCompilation,
    LocalOneShotCompilationRequest, LocalPackageDistributionRequest, LocalSourceGuidancePlan,
    LocalSourceGuidancePlanRequest, LocalSourceGuidanceRecoveryExecutionRequest,
    LocalSourceGuidanceRecoveryPlan, LocalSourceGuidanceRecoveryRequest,
    LocalSourceGuidanceRecoveryResult, LocalSourceGuidanceWriteRequest,
    LocalSourceGuidanceWriteResult, LocalWorkspaceLocator, LocalWorkspaceStatus,
    SourceGuidanceHost, SourceGuidanceProjection,
};

use pale_sdk::{
    AdmittedRepositoryNativeActions, ApiInitializationError, ContractBundleIdentity,
    ContractSchema, EffectiveProfileCompilation, NativeActionRequest, OneShotCompilation,
    PackageCatalogInspection, PackageDistributionInspection, PackageDistributionRequest, PaleApi,
    PortableWorkspaceComparison, RepositoryNativeActionCatalogRequest, ValidatedContractDocument,
};

/// One explicitly typed document for context-free authoring validation.
///
/// The role is selected by the caller rather than inferred from the label or
/// bytes. Debug output reports sizes only so a process carrier cannot disclose
/// private or Package source through routine logging.
pub struct StandaloneAuthoringDocument {
    role: StandaloneAuthoringRole,
    source_label: String,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for StandaloneAuthoringDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StandaloneAuthoringDocument")
            .field("role", &self.role)
            .field("source_label_byte_len", &self.source_label.len())
            .field("source_byte_len", &self.bytes.len())
            .finish()
    }
}

impl StandaloneAuthoringDocument {
    /// Associate source bytes with the grammar selected by the caller.
    ///
    /// Construction does not validate the document or enforce input limits.
    /// The SDK performs those checks when the command is dispatched.
    #[must_use]
    pub fn new(role: StandaloneAuthoringRole, source_label: String, bytes: Vec<u8>) -> Self {
        Self {
            role,
            source_label,
            bytes,
        }
    }
}

/// One context-free structural authoring query.
#[derive(Debug)]
pub struct StandaloneAuthoringInspectionRequest {
    role: StandaloneAuthoringRole,
    path: Vec<AuthoringPathSegment>,
}

impl StandaloneAuthoringInspectionRequest {
    /// Bind an explicit grammar role to a syntax-derived structural path.
    #[must_use]
    pub fn new(role: StandaloneAuthoringRole, path: Vec<AuthoringPathSegment>) -> Self {
        Self { role, path }
    }
}

/// One structural authoring query over a freshly admitted portable workspace.
#[derive(Debug)]
pub struct PortableWorkspaceAuthoringInspectionRequest {
    workspace: PortableWorkspaceRequest,
    inspection: AuthoringInspectionRequest,
}

impl PortableWorkspaceAuthoringInspectionRequest {
    /// Bind captured workspace source to one compiler-owned structural query.
    #[must_use]
    pub fn new(
        workspace: PortableWorkspaceRequest,
        inspection: AuthoringInspectionRequest,
    ) -> Self {
        Self {
            workspace,
            inspection,
        }
    }
}

/// One semantic navigation query over a freshly admitted portable workspace.
#[derive(Debug)]
pub struct PortableWorkspaceAuthoringNavigationRequest {
    workspace: PortableWorkspaceRequest,
    navigation: AuthoringNavigationRequest,
}

impl PortableWorkspaceAuthoringNavigationRequest {
    /// Bind captured workspace source to one exact source occurrence.
    #[must_use]
    pub fn new(
        workspace: PortableWorkspaceRequest,
        navigation: AuthoringNavigationRequest,
    ) -> Self {
        Self {
            workspace,
            navigation,
        }
    }
}

/// Captured source roles shared by effective-profile command journeys.
#[derive(Debug)]
struct EffectiveProfileInputs {
    workspace: PortableWorkspaceRequest,
    profile: EffectiveProfileCompilationRequest,
}

impl EffectiveProfileInputs {
    const fn new(
        workspace: PortableWorkspaceRequest,
        profile: EffectiveProfileCompilationRequest,
    ) -> Self {
        Self { workspace, profile }
    }
}

/// Complete stateless input for one portable workspace explanation.
///
/// The two captured workspaces and the closed API selector remain unadmitted
/// until [`CommandDispatcher::dispatch`] invokes the owning API operation.
#[derive(Debug)]
pub struct PortableWorkspaceExplanationRequest {
    prior: PortableWorkspaceRequest,
    current: PortableWorkspaceRequest,
    selector: PortableWorkspaceSubjectSelector,
}

impl PortableWorkspaceExplanationRequest {
    /// Bind two captured workspaces to one API-owned subject selector.
    #[must_use]
    pub fn new(
        prior: PortableWorkspaceRequest,
        current: PortableWorkspaceRequest,
        selector: PortableWorkspaceSubjectSelector,
    ) -> Self {
        Self {
            prior,
            current,
            selector,
        }
    }
}

/// Complete stateless input for one effective-profile inspection page.
///
/// The dispatcher first admits `workspace`, then compiles `profile` against
/// that exact retained workspace owner, and finally pages the API-owned
/// projection. The request does not reconstruct profile meaning or machine
/// realization in a process leaf.
#[derive(Debug)]
pub struct EffectiveProfileInspectionRequest {
    inputs: EffectiveProfileInputs,
    page: EffectiveProfilePageRequest,
}

impl EffectiveProfileInspectionRequest {
    /// Bind complete captured sources to one bounded API inspection page.
    #[must_use]
    pub fn new(
        workspace: PortableWorkspaceRequest,
        profile: EffectiveProfileCompilationRequest,
        page: EffectiveProfilePageRequest,
    ) -> Self {
        Self {
            inputs: EffectiveProfileInputs::new(workspace, profile),
            page,
        }
    }
}

/// Complete captured input for one API-owned observation snapshot.
///
/// The command façade compiles the exact effective profile once and asks
/// [`PaleApi::observe_effective_profile`] to project the retained state. The
/// request carries no observation policy and cannot fabricate resident or
/// native-owner evidence.
#[derive(Debug)]
pub struct EffectiveProfileObservationRequest {
    inputs: EffectiveProfileInputs,
}

impl EffectiveProfileObservationRequest {
    /// Bind complete captured sources for one observation snapshot.
    #[must_use]
    pub fn new(
        workspace: PortableWorkspaceRequest,
        profile: EffectiveProfileCompilationRequest,
    ) -> Self {
        Self {
            inputs: EffectiveProfileInputs::new(workspace, profile),
        }
    }
}

/// One explanation lookup over a freshly admitted effective profile.
#[derive(Debug)]
pub struct EffectiveProfileExplanationRequest {
    inputs: EffectiveProfileInputs,
    handle: EffectiveProfileExplanationHandle,
}

impl EffectiveProfileExplanationRequest {
    /// Bind complete captured sources to one API-owned explanation handle.
    #[must_use]
    pub fn new(
        workspace: PortableWorkspaceRequest,
        profile: EffectiveProfileCompilationRequest,
        handle: EffectiveProfileExplanationHandle,
    ) -> Self {
        Self {
            inputs: EffectiveProfileInputs::new(workspace, profile),
            handle,
        }
    }
}

/// One structural authoring query over a freshly admitted effective profile.
#[derive(Debug)]
pub struct EffectiveProfileAuthoringInspectionRequest {
    inputs: EffectiveProfileInputs,
    inspection: AuthoringInspectionRequest,
}

impl EffectiveProfileAuthoringInspectionRequest {
    /// Bind complete captured sources to one compiler-owned structural query.
    #[must_use]
    pub fn new(
        workspace: PortableWorkspaceRequest,
        profile: EffectiveProfileCompilationRequest,
        inspection: AuthoringInspectionRequest,
    ) -> Self {
        Self {
            inputs: EffectiveProfileInputs::new(workspace, profile),
            inspection,
        }
    }
}

/// One semantic navigation query over a freshly admitted effective profile.
#[derive(Debug)]
pub struct EffectiveProfileAuthoringNavigationRequest {
    inputs: EffectiveProfileInputs,
    navigation: AuthoringNavigationRequest,
}

impl EffectiveProfileAuthoringNavigationRequest {
    /// Bind complete captured sources to one exact source occurrence.
    #[must_use]
    pub fn new(
        workspace: PortableWorkspaceRequest,
        profile: EffectiveProfileCompilationRequest,
        navigation: AuthoringNavigationRequest,
    ) -> Self {
        Self {
            inputs: EffectiveProfileInputs::new(workspace, profile),
            navigation,
        }
    }
}

/// One effective-profile join keyed by a compiler-resolved authoring identity.
#[derive(Debug)]
pub struct EffectiveProfileAuthoringJoinRequest {
    inputs: EffectiveProfileInputs,
    identity: AuthoringIdentity,
}

impl EffectiveProfileAuthoringJoinRequest {
    /// Bind complete captured sources to an identity returned by authoring navigation.
    #[must_use]
    pub fn new(
        workspace: PortableWorkspaceRequest,
        profile: EffectiveProfileCompilationRequest,
        identity: AuthoringIdentity,
    ) -> Self {
        Self {
            inputs: EffectiveProfileInputs::new(workspace, profile),
            identity,
        }
    }
}

/// One transport-neutral request accepted by the shared command dispatcher.
///
/// Each variant selects a [`PaleApi`] operation or a fixed sequence of SDK
/// operations. A request carries captured inputs, typed identities, or retained
/// SDK values. Those values remain in process; they are not serialized
/// continuation tokens. Application and core records do not cross this boundary.
#[derive(Debug)]
pub enum CommandRequest {
    /// Check local authored content without requiring a published module.
    #[cfg(feature = "native-local")]
    LocalWorkspaceSource(pale_sdk::LocalWorkspaceLocator),
    /// Diagnose explicit local inputs without acquiring runtime authority.
    #[cfg(feature = "native-local")]
    LocalDoctor(pale_sdk::LocalDoctorRequest),
    /// Advance an effect-free configuration review through its SDK-owned stage.
    ConfigurationImprovement(ConfigurationImprovementCommand),
    /// Inspect captured Package bytes without consuming a distribution lock.
    PackageDistribution(PackageDistributionRequest),
    /// Inspect locally selected Package bytes without trusting or replacing the lock.
    #[cfg(feature = "native-local")]
    LocalPackageDistribution(LocalPackageDistributionRequest),
    /// Read and check reusable Package guidance in a local distribution.
    #[cfg(feature = "native-local")]
    LocalPackageCatalog(String),
    /// Check source declarations and regenerate family indexes.
    #[cfg(feature = "native-local")]
    LocalPackageIndexUpdate(String),
    /// Check local Package declarations and explicitly selected caller trees.
    #[cfg(feature = "native-local")]
    LocalPackageCheck(pale_sdk::LocalPackageCheckRequest),
    /// Read captured Package guidance without another declaration parser.
    PackageCatalog(PackageDistributionRequest),
    /// Capture one confined local input and execute it through the closed action registry.
    #[cfg(feature = "native-local")]
    LocalNativeAction(LocalNativeActionInputRequest),
    /// Report SDK version, interface protocol, and target claims compiled into this build.
    #[cfg(feature = "native-local")]
    CompiledInterfaceIdentity,
    /// Classify one exact local control store without creating or repairing it.
    #[cfg(feature = "native-local")]
    LocalControlStorageInspection(LocalControlStorageInspectionRequest),
    /// Resolve, capture, and compile one local workspace selection.
    #[cfg(feature = "native-local")]
    LocalOneShotCompilation(LocalOneShotCompilationRequest),
    /// Capture selected private context without publishing repository files.
    #[cfg(feature = "native-local")]
    LocalRepositoryContext {
        /// Exact source selection, shared with effective-profile compilation.
        selection: LocalOneShotCompilationRequest,
        /// Logical repository selected in that profile.
        repository_id: String,
    },
    /// Capture and project Package judgment without runtime rendering or writes.
    #[cfg(feature = "native-local")]
    LocalSourceGuidance {
        /// Exact source selection, shared with the earlier profile phase.
        selection: LocalOneShotCompilationRequest,
        /// Logical repository selected in that profile.
        repository_id: String,
        /// Explicit native layout, without a claim of installed-host readiness.
        host: SourceGuidanceHost,
    },
    /// Capture native target baselines and plan source-guidance reconciliation.
    #[cfg(feature = "native-local")]
    LocalSourceGuidancePlan(LocalSourceGuidancePlanRequest),
    /// Remember the exact proposal after the caller has displayed it.
    #[cfg(feature = "native-local")]
    LocalSourceGuidanceRemember(Box<LocalSourceGuidancePlan>),
    /// Rebuild current inputs and publish exactly reviewed source guidance.
    #[cfg(feature = "native-local")]
    LocalSourceGuidanceWrite(LocalSourceGuidanceWriteRequest),
    /// Inspect a pending native recovery without effects or effect admission.
    #[cfg(feature = "native-local")]
    LocalSourceGuidanceRecoveryPlan(LocalSourceGuidanceRecoveryRequest),
    /// Rebuild, admit and execute an exact reviewed native recovery decision.
    #[cfg(feature = "native-local")]
    LocalSourceGuidanceRecoveryExecute(LocalSourceGuidanceRecoveryExecutionRequest),
    /// Inspect, plan, or materialize using the SDK value retained by the previous step.
    #[cfg(feature = "native-local")]
    LocalMaterialization(LocalMaterializationCommand),
    /// Capture local review sources without rendering or writing configuration.
    #[cfg(feature = "native-local")]
    LocalConfigurationImprovementCapture(LocalConfigurationImprovementRequest),
    /// Read the identities of the bundled descriptor manifest and schema set.
    ContractBundleIdentity,
    /// Copy the generated schema selected by one registry identity.
    ContractSchema(String),
    /// Structurally validate and canonicalize one bounded JSON document.
    ValidateContractDocument(ContractDocument),
    /// Admit one untrusted diagnostic document through the API-owned boundary.
    DiagnosticAdmission(Vec<u8>),
    /// Read the SDK input limits for one class of authoring source.
    AuthoringIngressLimits(AuthoringIngressClass),
    /// Validate one explicitly typed source without selecting project context.
    StandaloneAuthoringValidation(StandaloneAuthoringDocument),
    /// Inspect one explicitly selected standalone authoring grammar.
    StandaloneAuthoringInspection(StandaloneAuthoringInspectionRequest),
    /// Inspect the complete effect-free native-action registry.
    NativeActionCatalog,
    /// Execute a complete tagged action document supplied by the CLI boundary.
    ///
    /// Protocol carriers select [`Self::NativeActionPayload`] instead, so the
    /// registry, not an untrusted payload, owns the action tag.
    NativeAction(NativeActionRequest),
    /// Execute a payload whose action identity was selected out of band.
    NativeActionPayload(NativeActionPayloadRequest),
    /// Admit one repository-selected action catalog against the compiled registry.
    AdmitRepositoryNativeActions(RepositoryNativeActionCatalogRequest),
    /// Compile one captured portable workspace and return its owned summary.
    PortableWorkspaceCompilation(PortableWorkspaceRequest),
    /// Inspect structural facts retained by one portable workspace compilation.
    PortableWorkspaceAuthoringInspection(PortableWorkspaceAuthoringInspectionRequest),
    /// Resolve one source occurrence retained by a portable workspace compilation.
    PortableWorkspaceAuthoringNavigation(PortableWorkspaceAuthoringNavigationRequest),
    /// Compile and compare two captured portable workspaces, then return the summary.
    PortableWorkspaceComparison {
        /// Complete predecessor workspace source.
        prior: PortableWorkspaceRequest,
        /// Complete current workspace source.
        current: PortableWorkspaceRequest,
    },
    /// Recompile one comparison and page its exact change roots.
    PortableWorkspaceChangeRoots {
        /// Complete predecessor workspace source.
        prior: PortableWorkspaceRequest,
        /// Complete current workspace source.
        current: PortableWorkspaceRequest,
        /// Zero-based page offset.
        offset: u32,
        /// Requested bounded page size; zero selects the API default.
        limit: u16,
    },
    /// Recompile one comparison and page its complete impact partition.
    PortableWorkspaceImpacts {
        /// Complete predecessor workspace source.
        prior: PortableWorkspaceRequest,
        /// Complete current workspace source.
        current: PortableWorkspaceRequest,
        /// Zero-based page offset.
        offset: u32,
        /// Requested bounded page size; zero selects the API default.
        limit: u16,
    },
    /// Recompile one comparison and explain an admitted causal subject.
    PortableWorkspaceExplanation(PortableWorkspaceExplanationRequest),
    /// Compile and page one exact effective-profile inspection.
    EffectiveProfileInspection(EffectiveProfileInspectionRequest),
    /// Resolve one explanation retained by an exact effective-profile compilation.
    EffectiveProfileExplanation(EffectiveProfileExplanationRequest),
    /// Inspect structural facts retained by an effective-profile compilation.
    EffectiveProfileAuthoringInspection(EffectiveProfileAuthoringInspectionRequest),
    /// Resolve one source occurrence retained by an effective-profile compilation.
    EffectiveProfileAuthoringNavigation(EffectiveProfileAuthoringNavigationRequest),
    /// Join one resolved authoring identity to retained effective-profile facts.
    EffectiveProfileAuthoringJoin(EffectiveProfileAuthoringJoinRequest),
    /// Compile one effective profile and retain its API-owned observation snapshot.
    ObservationSnapshot(EffectiveProfileObservationRequest),
    /// Query one bounded page from an API-owned observation snapshot.
    ObservationPage {
        /// Opaque snapshot returned by [`Self::ObservationSnapshot`].
        snapshot: ObservationSnapshot,
        /// Filter, bound, and optional continuation issued by that snapshot.
        request: ObservationPageRequest,
    },
    /// Derive one complete adjacent delta without emitting partial state.
    ObservationDelta {
        /// Current opaque snapshot.
        current: ObservationSnapshot,
        /// Prior opaque snapshot named by the resume coordinate.
        prior: ObservationSnapshot,
        /// Resume, bound, and cooperative cancellation state.
        request: ObservationDeltaRequest,
    },
    /// Resolve one exact causal selector against its issuing snapshot.
    ObservationCausalPath {
        /// Opaque snapshot retaining the compiler-owned proof.
        snapshot: ObservationSnapshot,
        /// Generation-bound subject selector issued from that snapshot.
        selector: ObservationCausalSelector,
    },
    /// Construct one bounded disposable lens from an API-owned snapshot.
    ObservationVisualLens {
        /// Opaque snapshot that remains the semantic source.
        snapshot: ObservationSnapshot,
        /// Filter, bound, proof, and cooperative cancellation selection.
        request: ObservationLensRequest,
    },
    /// Compile one complete captured source family and return its owned summary.
    OneShotCompilation(OneShotCompilationRequest),
    /// Recompile one source family and derive its required observation targets.
    OneShotObservationTargets {
        /// Complete captured one-shot source family.
        compilation: OneShotCompilationRequest,
        /// Logical repository selected from the compilation.
        repository_id: String,
        /// Effect-free planning operation whose targets are required.
        kind: OneShotPlanKind,
        /// Exact caller-supplied prior ownership claim.
        prior: OneShotPriorMaterialization,
    },
    /// Recompile one source family and construct one effect-free plan candidate.
    OneShotPlanning {
        /// Complete captured one-shot source family.
        compilation: OneShotCompilationRequest,
        /// Exact API-owned repository planning request.
        request: OneShotPlanningRequest,
    },
    /// Bind machine-local observations into one complete effect-free plan.
    OneShotMachineEffectBinding {
        /// API-owned plan candidate returned by [`Self::OneShotPlanning`].
        candidate: Box<OneShotPlanCandidate>,
        /// Canonical machine-local observations captured by the host leaf.
        effects: Vec<MachineLocalEffectCandidate>,
    },
    /// Admit saved bytes against one freshly reconstructed bound plan.
    OneShotSavedPlanAdmission {
        /// Complete API-owned plan returned by [`Self::OneShotMachineEffectBinding`].
        rebuilt: Box<OneShotBoundPlan>,
        /// Exact caller-captured saved plan bytes.
        saved_bytes: Vec<u8>,
    },
    /// Compile a local workspace and verify the current executable against its binding.
    #[cfg(feature = "native-local")]
    CurrentExecutableBindingVerification(LocalOneShotCompilationRequest),
    /// Admit one repository-native event and require the selected resident carrier.
    ResidentHook(ResidentHookRequest),
    /// Observe one retained operation through an explicitly supplied resident attachment.
    ResidentOperationObservation(ResidentOperationObservationRequest),
    /// Start one durable root Invoke.
    ResidentInvokeStart(ResidentInvokeStartRequest),
    /// Advance one retained Invoke by at most one bounded owner turn.
    ResidentInvokeAdvance(ResidentInvokeAdvanceRequest),
    /// Start one already-selected durable Consult.
    ResidentConsultStart(ResidentConsultStartRequest),
    /// Advance one retained Consult by at most one bounded turn.
    ResidentConsultAdvance(ResidentConsultAdvanceRequest),
    /// Start one exact pairwise Delegate child before dispatch.
    ResidentDelegateStart(ResidentDelegateStartRequest),
    /// Advance one Delegate through its already-selected dispatch owner.
    ResidentDelegateAdvance(ResidentDelegateAdvanceRequest),
    /// Send one Cancel request without treating acknowledgement as terminal state.
    ResidentOperationCancellation(ResidentOperationCancellationRequest),
}

/// One API-owned result returned by [`CommandDispatcher::dispatch`].
///
/// CLI and MCP leaves may project these DTOs or their canonical bytes into
/// terminal or protocol forms. They may not reinterpret the result or derive a
/// second validation, admission, or diagnostic policy. Structural authoring
/// failures use the same [`ApiError`] channel as every other façade operation.
#[derive(Debug)]
pub enum CommandOutcome {
    /// Source-only content and exact source evidence; no admitted runtime handle.
    #[cfg(feature = "native-local")]
    WorkspaceSource(pale_sdk::WorkspaceSourceInspection),
    /// Complete local diagnosis, retaining failed and unavailable observations.
    #[cfg(feature = "native-local")]
    Doctor(pale_sdk::DoctorInspection),
    /// Exact resident-admitted native protocol bytes and process status.
    ResidentHook(pale_sdk::ResidentHookOutput),
    /// An opaque review stage; only its SDK projections may cross a wire boundary.
    ConfigurationImprovement(ConfigurationImprovementOutcome),
    /// Canonical Package release coordinates available for lock authoring.
    PackageDistribution(PackageDistributionInspection),
    /// Checked declarations and reading routes from the compiler's Package reader.
    PackageCatalog(PackageCatalogInspection),
    /// Compiled interface claims, without executable verification or process-health evidence.
    #[cfg(feature = "native-local")]
    CompiledInterfaceIdentity(CompiledInterfaceIdentity),
    /// Physical store facts, without writer authority or runtime readiness.
    #[cfg(feature = "native-local")]
    LocalControlStorageInspection(LocalControlStorageInspection),
    /// Opaque native composition retaining one exact local compilation.
    #[cfg(feature = "native-local")]
    LocalOneShotCompilation(Box<LocalOneShotCompilation>),
    /// Proposed source-guidance files, separate from any runtime or file plan.
    #[cfg(feature = "native-local")]
    SourceGuidance(SourceGuidanceProjection),
    /// Private context text; never a portable file or a host admission.
    #[cfg(feature = "native-local")]
    RepositoryContext(String),
    /// Opaque source-only proposal; no admission or filesystem effect.
    #[cfg(feature = "native-local")]
    SourceGuidancePlan(Box<LocalSourceGuidancePlan>),
    /// The displayed proposal identity was saved for a later apply.
    #[cfg(feature = "native-local")]
    SourceGuidanceRemembered,
    /// Confirmed source-guidance file publication, without host-use evidence.
    #[cfg(feature = "native-local")]
    SourceGuidanceWrite(LocalSourceGuidanceWriteResult),
    /// Pending recovery review; absence means no journal at observation time.
    #[cfg(feature = "native-local")]
    SourceGuidanceRecoveryPlan(Option<Box<LocalSourceGuidanceRecoveryPlan>>),
    /// Confirmed native recovery, without runtime or host activation.
    #[cfg(feature = "native-local")]
    SourceGuidanceRecoveryResult(LocalSourceGuidanceRecoveryResult),
    /// One retained local inspection, planning, or materialization result.
    #[cfg(feature = "native-local")]
    LocalMaterialization(LocalMaterializationOutcome),
    /// Baseline and candidate source roles ready for the shared review command.
    #[cfg(feature = "native-local")]
    ConfigurationImprovementSources(Box<ConfigurationImprovementSources>),
    /// Exact identity of the complete embedded contract bundle.
    ContractBundleIdentity(ContractBundleIdentity),
    /// Exact generated schema selected by registry identity.
    ContractSchema(ContractSchema),
    /// Structurally admitted document in canonical byte form.
    ValidatedContractDocument(ValidatedContractDocument),
    /// Semantically admitted initial-v1 diagnostic.
    DiagnosticAdmission(Box<DiagnosticV1>),
    /// SDK input limits for one class of authoring source.
    AuthoringIngressLimits(AuthoringIngressLimits),
    /// Successful context-free validation of one explicitly typed source.
    StandaloneAuthoringValidation,
    /// Exact result of a context-free structural authoring query.
    StandaloneAuthoringInspection(AuthoringInspection),
    /// Stable compiled catalog of pure native-action descriptors.
    NativeActionCatalog(Vec<NativeActionDescriptor>),
    /// Canonical result of one pure native-action invocation.
    NativeAction(NativeActionExecution),
    /// Repository-selected native actions after contextual catalog admission.
    AdmittedRepositoryNativeActions(AdmittedRepositoryNativeActions),
    /// Owned identity-and-count projection of a portable workspace compilation.
    PortableWorkspaceCompilationSummary(PortableWorkspaceCompilationSummary),
    /// Exact result of one portable workspace structural authoring query.
    PortableWorkspaceAuthoringInspection(AuthoringInspection),
    /// Semantic navigation resolved by one admitted portable workspace.
    PortableWorkspaceAuthoringNavigation(AuthoringNavigation),
    /// Owned identity-and-count projection of a verified workspace comparison.
    PortableWorkspaceComparisonSummary(PortableWorkspaceComparisonSummary),
    /// One bounded page of exact source-change roots.
    PortableWorkspaceChangeRoots(PortableWorkspaceChangeRootPage),
    /// One bounded page of the complete impact partition.
    PortableWorkspaceImpacts(PortableWorkspaceImpactPage),
    /// One API-owned causal explanation result.
    PortableWorkspaceExplanation(PortableWorkspaceExplanation),
    /// One bounded page from an API-owned effective-profile projection.
    EffectiveProfilePage(EffectiveProfilePage),
    /// Retained explanation for one effective-profile row, when present.
    EffectiveProfileExplanation(Option<EffectiveProfileExplanation>),
    /// Exact result of one effective-profile structural authoring query.
    EffectiveProfileAuthoringInspection(AuthoringInspection),
    /// Semantic navigation resolved by one admitted effective profile.
    EffectiveProfileAuthoringNavigation(AuthoringNavigation),
    /// Effective-profile facts joined to one resolved authoring identity.
    EffectiveProfileAuthoringJoin(EffectiveAuthoringInspection),
    /// Opaque source-free observation snapshot owned by the API façade.
    ObservationSnapshot(ObservationSnapshot),
    /// One bounded page step, cancellation, or explicit cursor resynchronization.
    ObservationPage(Result<ObservationPageDisposition, ObservationResynchronization>),
    /// Complete adjacent delta, resynchronization, cancellation, or backpressure.
    ObservationDelta(ObservationDeltaOutcome),
    /// One retained causal proof, absence, or generation resynchronization.
    ObservationCausalPath(Result<Option<ObservationCausalPath>, ObservationResynchronization>),
    /// One bounded disposable lens step, cancellation, or cursor resynchronization.
    ObservationVisualLens(ObservationLensDisposition),
    /// Owned flat summary of one complete captured compilation.
    OneShotCompilationSummary(Box<OneShotCompilationSummary>),
    /// Exact repository-relative targets required before effect-free planning.
    OneShotObservationTargets(OneShotObservationTargets),
    /// Canonical effect-free repository plan candidate.
    OneShotPlanCandidate(Box<OneShotPlanCandidate>),
    /// Complete effect-free plan with machine-local observations bound.
    OneShotBoundPlan(Box<OneShotBoundPlan>),
    /// Saved-plan admission proof carrying no executed native effect.
    OneShotAdmittedPlan(Box<OneShotAdmittedPlan>),
    /// Compiled interface claims after checking the selected executable binding.
    #[cfg(feature = "native-local")]
    CurrentExecutableBindingVerification(CompiledInterfaceIdentity),
    /// Signed records returned by one admitted resident Observe request.
    ResidentOperationObservation(ResidentOperationObservation),
    /// Signed records returned after one durable Invoke start.
    ResidentInvokeStarted(ResidentOperationObservation),
    /// Honest result of one bounded Invoke turn.
    ResidentInvokeAdvanced(ResidentInvokeAdvanceOutcome),
    /// Signed records returned after one durable Consult start.
    ResidentConsultStarted(ResidentOperationObservation),
    /// Honest result of one bounded Consult turn.
    ResidentConsultAdvanced(ResidentConsultAdvanceOutcome),
    /// Signed records returned after one durable pairwise Delegate start.
    ResidentDelegateStarted(ResidentOperationObservation),
    /// Honest result of one bounded Delegate turn.
    ResidentDelegateAdvanced(ResidentDelegateAdvanceOutcome),
    /// Exact Cancel disposition without an implied terminal state.
    ResidentOperationCancellation(ResidentOperationCancellationOutcome),
}

impl CommandOutcome {
    /// Consume the result of a native-action command.
    ///
    /// The returned value remains an owned API projection rather than an
    /// application or core type.
    ///
    /// # Errors
    ///
    /// Returns the original outcome, boxed without reinterpretation, when it is
    /// not an action result.
    pub fn into_native_action(self) -> Result<NativeActionExecution, Box<Self>> {
        match self {
            Self::NativeAction(execution) => Ok(execution),
            other => Err(Box::new(other)),
        }
    }

    /// Consume repository-native-action admission evidence.
    ///
    /// # Errors
    ///
    /// Returns the original outcome, boxed without reinterpretation, when it is
    /// not repository native-action admission evidence.
    pub fn into_admitted_repository_native_actions(
        self,
    ) -> Result<AdmittedRepositoryNativeActions, Box<Self>> {
        match self {
            Self::AdmittedRepositoryNativeActions(admitted) => Ok(admitted),
            other => Err(Box::new(other)),
        }
    }

    /// Consume the result of a complete one-shot compilation.
    ///
    /// The returned summary is the exact API-owned projection shared by the
    /// CLI, MCP, and eligible language bindings.
    ///
    /// # Errors
    ///
    /// Returns the original outcome, boxed without reinterpretation, when it is
    /// not a one-shot compilation summary.
    pub fn into_one_shot_compilation_summary(
        self,
    ) -> Result<Box<OneShotCompilationSummary>, Box<Self>> {
        match self {
            Self::OneShotCompilationSummary(summary) => Ok(summary),
            other => Err(Box::new(other)),
        }
    }

    /// Consume one effect-free plan candidate for a later binding request.
    ///
    /// # Errors
    ///
    /// Returns the original outcome, boxed without reinterpretation, when it is
    /// not a plan candidate.
    pub fn into_one_shot_plan_candidate(self) -> Result<Box<OneShotPlanCandidate>, Box<Self>> {
        match self {
            Self::OneShotPlanCandidate(candidate) => Ok(candidate),
            other => Err(Box::new(other)),
        }
    }

    /// Consume one complete bound plan for saved-plan admission.
    ///
    /// # Errors
    ///
    /// Returns the original outcome, boxed without reinterpretation, when it is
    /// not a bound plan.
    pub fn into_one_shot_bound_plan(self) -> Result<Box<OneShotBoundPlan>, Box<Self>> {
        match self {
            Self::OneShotBoundPlan(plan) => Ok(plan),
            other => Err(Box::new(other)),
        }
    }

    /// Consume one saved-plan admission proof.
    ///
    /// # Errors
    ///
    /// Returns the original outcome, boxed without reinterpretation, when it is
    /// not an admitted plan.
    pub fn into_one_shot_admitted_plan(self) -> Result<Box<OneShotAdmittedPlan>, Box<Self>> {
        match self {
            Self::OneShotAdmittedPlan(plan) => Ok(plan),
            other => Err(Box::new(other)),
        }
    }
}

/// In-process dispatcher over the narrow [`PaleApi`] façade.
///
/// This type owns no command-line parsing, terminal I/O, filesystem adapter,
/// protocol state, progress, cancellation, resume, or native-effect semantics.
/// A selected native command may ask the SDK to perform bounded observation,
/// publish a plan, or settle admitted effects; the dispatcher only transports
/// the exact owners and results. CLI and MCP projections retain ownership of
/// their presentation concerns.
pub struct CommandDispatcher {
    api: PaleApi,
    resident_attachment: Option<ResidentAttachment>,
}

impl CommandDispatcher {
    /// Construct a dispatcher after verifying the embedded API contracts.
    ///
    /// # Errors
    ///
    /// Returns [`ApiInitializationError`] when the embedded contract registry or
    /// pure native-action catalog is incoherent.
    pub fn new() -> Result<Self, ApiInitializationError> {
        PaleApi::new().map(|api| Self {
            api,
            resident_attachment: None,
        })
    }

    /// Construct a dispatcher with one caller-provisioned resident attachment.
    ///
    /// This does not discover an endpoint, open storage, resolve keys, or create
    /// an attachment or prove the caller's authentication assertions. The
    /// caller must supply the opaque SDK value issued by its trusted native
    /// composition root.
    ///
    /// # Errors
    ///
    /// Returns [`ApiInitializationError`] when the embedded contract registry or
    /// pure native-action catalog is incoherent.
    pub fn with_resident_attachment(
        resident_attachment: ResidentAttachment,
    ) -> Result<Self, ApiInitializationError> {
        PaleApi::new().map(|api| Self {
            api,
            resident_attachment: Some(resident_attachment),
        })
    }

    /// Report whether this dispatcher can route resident observation requests.
    #[must_use]
    pub const fn has_resident_attachment(&self) -> bool {
        self.resident_attachment.is_some()
    }

    /// Report whether this dispatcher can route resident operation mutations.
    #[must_use]
    pub fn has_resident_operation_attachment(&self) -> bool {
        self.resident_attachment
            .as_ref()
            .is_some_and(ResidentAttachment::supports_operation_requests)
    }

    /// Execute one typed command through its single API-owned façade path.
    ///
    /// Semantically equal [`CommandRequest::NativeAction`] and
    /// [`CommandRequest::NativeActionPayload`] values therefore produce the same
    /// API-owned canonical execution or diagnostic bytes. Carrier-specific
    /// decoding, rendering, and exit or protocol policy happen after this
    /// boundary.
    ///
    /// # Errors
    ///
    /// Returns the exact [`ApiError`] produced by the selected façade operation.
    /// The dispatcher does not reclassify or render the diagnostic. A rejected
    /// local materialization settlement is instead returned as an owned
    /// `LocalMaterializationSettlementDisposition` when the `native-local`
    /// feature is enabled, so the same mutated run remains available for an
    /// explicitly selected reconciliation read.
    pub fn dispatch(&self, command: CommandRequest) -> Result<CommandOutcome, ApiError> {
        match command {
            #[cfg(feature = "native-local")]
            CommandRequest::LocalWorkspaceSource(request) => self
                .api
                .inspect_local_workspace_source(request)
                .map(CommandOutcome::WorkspaceSource),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalDoctor(request) => self
                .api
                .inspect_local_doctor(request)
                .map(CommandOutcome::Doctor),
            CommandRequest::ConfigurationImprovement(command) => {
                configuration_improvement::dispatch(&self.api, command)
                    .map(CommandOutcome::ConfigurationImprovement)
            }
            CommandRequest::PackageDistribution(request) => self
                .api
                .inspect_package_distribution(request)
                .map(CommandOutcome::PackageDistribution),
            CommandRequest::PackageCatalog(request) => self
                .api
                .read_package_catalog(request)
                .map(CommandOutcome::PackageCatalog),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalPackageCatalog(root) => self
                .api
                .read_local_package_catalog(root)
                .map(CommandOutcome::PackageCatalog),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalPackageCheck(request) => self
                .api
                .check_local_packages(request)
                .map(CommandOutcome::PackageCatalog),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalPackageIndexUpdate(root) => self
                .api
                .update_local_package_indexes(root)
                .map(CommandOutcome::PackageCatalog),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalPackageDistribution(request) => self
                .api
                .inspect_local_package_distribution(request)
                .map(CommandOutcome::PackageDistribution),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalNativeAction(request) => self
                .api
                .capture_local_native_action_request(request)
                .and_then(|request| self.api.invoke_native_action(request))
                .map(CommandOutcome::NativeAction),
            #[cfg(feature = "native-local")]
            CommandRequest::CompiledInterfaceIdentity => self
                .api
                .compiled_interface_identity()
                .map(CommandOutcome::CompiledInterfaceIdentity),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalControlStorageInspection(request) => self
                .api
                .inspect_local_control_store(request)
                .map(CommandOutcome::LocalControlStorageInspection),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalConfigurationImprovementCapture(request) => self
                .api
                .capture_local_configuration_improvement_sources(request)
                .map(Box::new)
                .map(CommandOutcome::ConfigurationImprovementSources),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalOneShotCompilation(request) => self
                .api
                .compile_local_one_shot(request)
                .map(Box::new)
                .map(CommandOutcome::LocalOneShotCompilation),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalSourceGuidance {
                selection,
                repository_id,
                host,
            } => self
                .api
                .compile_local_effective_profile(selection)?
                .source_guidance(&repository_id, host)
                .map(CommandOutcome::SourceGuidance),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalRepositoryContext {
                selection,
                repository_id,
            } => self
                .api
                .compile_local_effective_profile(selection)?
                .repository_context(&repository_id)
                .map(CommandOutcome::RepositoryContext),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalSourceGuidancePlan(request) => self
                .api
                .plan_local_source_guidance(request)
                .map(Box::new)
                .map(CommandOutcome::SourceGuidancePlan),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalSourceGuidanceWrite(request) => self
                .api
                .apply_local_source_guidance(request)
                .map(CommandOutcome::SourceGuidanceWrite),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalSourceGuidanceRemember(plan) => self
                .api
                .remember_local_source_guidance_plan(&plan)
                .map(|()| CommandOutcome::SourceGuidanceRemembered),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalSourceGuidanceRecoveryPlan(request) => self
                .api
                .plan_local_source_guidance_recovery(request)
                .map(|plan| CommandOutcome::SourceGuidanceRecoveryPlan(plan.map(Box::new))),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalSourceGuidanceRecoveryExecute(request) => self
                .api
                .execute_local_source_guidance_recovery(request)
                .map(CommandOutcome::SourceGuidanceRecoveryResult),
            #[cfg(feature = "native-local")]
            CommandRequest::LocalMaterialization(command) => {
                local_materialization::dispatch(&self.api, command)
                    .map(CommandOutcome::LocalMaterialization)
            }
            CommandRequest::ContractBundleIdentity => Ok(CommandOutcome::ContractBundleIdentity(
                self.api.contract_bundle_identity(),
            )),
            CommandRequest::ContractSchema(schema_id) => self
                .api
                .contract_schema(&schema_id)
                .map(CommandOutcome::ContractSchema),
            CommandRequest::ValidateContractDocument(document) => self
                .api
                .validate_contract_document(document)
                .map(CommandOutcome::ValidatedContractDocument),
            CommandRequest::DiagnosticAdmission(bytes) => self
                .api
                .admit_diagnostic(&bytes)
                .map(Box::new)
                .map(CommandOutcome::DiagnosticAdmission),
            CommandRequest::AuthoringIngressLimits(class) => Ok(
                CommandOutcome::AuthoringIngressLimits(PaleApi::authoring_ingress_limits(class)),
            ),
            CommandRequest::StandaloneAuthoringValidation(document) => {
                let StandaloneAuthoringDocument {
                    role,
                    source_label,
                    bytes,
                } = document;
                self.api
                    .validate_authoring_document(role, source_label, bytes)
                    .map(|()| CommandOutcome::StandaloneAuthoringValidation)
            }
            CommandRequest::StandaloneAuthoringInspection(request) => {
                let StandaloneAuthoringInspectionRequest { role, path } = request;
                self.api
                    .inspect_standalone_authoring(role, path)
                    .map(CommandOutcome::StandaloneAuthoringInspection)
            }
            CommandRequest::NativeActionCatalog => self
                .api
                .native_action_descriptors()
                .map(CommandOutcome::NativeActionCatalog),
            CommandRequest::NativeAction(request) => self
                .api
                .invoke_native_action(request)
                .map(CommandOutcome::NativeAction),
            CommandRequest::NativeActionPayload(request) => self
                .api
                .invoke_native_action_payload(request)
                .map(CommandOutcome::NativeAction),
            CommandRequest::AdmitRepositoryNativeActions(request) => self
                .api
                .admit_repository_native_actions(request)
                .map(CommandOutcome::AdmittedRepositoryNativeActions),
            CommandRequest::PortableWorkspaceCompilation(request) => self
                .api
                .compile_portable_workspace(request)
                .map(|compilation| compilation.summary())
                .map(CommandOutcome::PortableWorkspaceCompilationSummary),
            CommandRequest::PortableWorkspaceAuthoringInspection(request) => {
                let PortableWorkspaceAuthoringInspectionRequest {
                    workspace,
                    inspection,
                } = request;
                self.api
                    .compile_portable_workspace(workspace)
                    .and_then(|workspace| workspace.inspect_authoring(inspection))
                    .map(CommandOutcome::PortableWorkspaceAuthoringInspection)
            }
            CommandRequest::PortableWorkspaceAuthoringNavigation(request) => {
                let PortableWorkspaceAuthoringNavigationRequest {
                    workspace,
                    navigation,
                } = request;
                self.api
                    .compile_portable_workspace(workspace)
                    .map(|workspace| {
                        CommandOutcome::PortableWorkspaceAuthoringNavigation(
                            workspace.navigate_authoring(navigation),
                        )
                    })
            }
            CommandRequest::PortableWorkspaceComparison { prior, current } => self
                .compile_comparison(prior, current)
                .map(|comparison| comparison.summary())
                .map(CommandOutcome::PortableWorkspaceComparisonSummary),
            CommandRequest::PortableWorkspaceChangeRoots {
                prior,
                current,
                offset,
                limit,
            } => self
                .compile_comparison(prior, current)
                .map(|comparison| comparison.change_roots(offset, limit))
                .map(CommandOutcome::PortableWorkspaceChangeRoots),
            CommandRequest::PortableWorkspaceImpacts {
                prior,
                current,
                offset,
                limit,
            } => self
                .compile_comparison(prior, current)
                .map(|comparison| comparison.impacts(offset, limit))
                .map(CommandOutcome::PortableWorkspaceImpacts),
            CommandRequest::PortableWorkspaceExplanation(request) => {
                let PortableWorkspaceExplanationRequest {
                    prior,
                    current,
                    selector,
                } = request;
                self.compile_comparison(prior, current)
                    .and_then(|comparison| comparison.explain_selector(selector))
                    .map(CommandOutcome::PortableWorkspaceExplanation)
            }
            CommandRequest::EffectiveProfileInspection(request) => {
                let EffectiveProfileInspectionRequest { inputs, page } = request;
                self.compile_effective_profile(inputs)
                    .map(|profile| profile.inspect(page))
                    .map(CommandOutcome::EffectiveProfilePage)
            }
            CommandRequest::EffectiveProfileExplanation(request) => {
                let EffectiveProfileExplanationRequest { inputs, handle } = request;
                self.compile_effective_profile(inputs)
                    .map(|profile| profile.explain(&handle))
                    .map(CommandOutcome::EffectiveProfileExplanation)
            }
            CommandRequest::EffectiveProfileAuthoringInspection(request) => {
                let EffectiveProfileAuthoringInspectionRequest { inputs, inspection } = request;
                self.compile_effective_profile(inputs)
                    .and_then(|profile| profile.inspect_authoring(inspection))
                    .map(CommandOutcome::EffectiveProfileAuthoringInspection)
            }
            CommandRequest::EffectiveProfileAuthoringNavigation(request) => {
                let EffectiveProfileAuthoringNavigationRequest { inputs, navigation } = request;
                self.compile_effective_profile(inputs).map(|profile| {
                    CommandOutcome::EffectiveProfileAuthoringNavigation(
                        profile.navigate_authoring(navigation),
                    )
                })
            }
            CommandRequest::EffectiveProfileAuthoringJoin(request) => {
                let EffectiveProfileAuthoringJoinRequest { inputs, identity } = request;
                self.compile_effective_profile(inputs).map(|profile| {
                    CommandOutcome::EffectiveProfileAuthoringJoin(
                        profile.inspect_effective_authoring(&identity),
                    )
                })
            }
            CommandRequest::ObservationSnapshot(request) => self
                .compile_effective_profile(request.inputs)
                .map(|profile| self.api.observe_effective_profile(&profile))
                .map(CommandOutcome::ObservationSnapshot),
            CommandRequest::ObservationPage { snapshot, request } => {
                Ok(CommandOutcome::ObservationPage(snapshot.page(request)))
            }
            CommandRequest::ObservationDelta {
                current,
                prior,
                request,
            } => Ok(CommandOutcome::ObservationDelta(
                current.delta_from(&prior, request),
            )),
            CommandRequest::ObservationCausalPath { snapshot, selector } => Ok(
                CommandOutcome::ObservationCausalPath(snapshot.causal_path(&selector)),
            ),
            CommandRequest::ObservationVisualLens { snapshot, request } => Ok(
                CommandOutcome::ObservationVisualLens(snapshot.visual_lens(request)),
            ),
            CommandRequest::OneShotCompilation(request) => self
                .compile_one_shot(request)
                .map(|compilation| compilation.summary())
                .map(Box::new)
                .map(CommandOutcome::OneShotCompilationSummary),
            CommandRequest::OneShotObservationTargets {
                compilation,
                repository_id,
                kind,
                prior,
            } => self
                .compile_one_shot(compilation)
                .and_then(|compilation| {
                    self.api
                        .one_shot_observation_targets(&compilation, repository_id, kind, &prior)
                })
                .map(CommandOutcome::OneShotObservationTargets),
            CommandRequest::OneShotPlanning {
                compilation,
                request,
            } => self
                .compile_one_shot(compilation)
                .and_then(|compilation| self.api.plan_one_shot(&compilation, request))
                .map(Box::new)
                .map(CommandOutcome::OneShotPlanCandidate),
            CommandRequest::OneShotMachineEffectBinding { candidate, effects } => self
                .api
                .bind_one_shot_machine_effects(*candidate, effects)
                .map(Box::new)
                .map(CommandOutcome::OneShotBoundPlan),
            CommandRequest::OneShotSavedPlanAdmission {
                rebuilt,
                saved_bytes,
            } => self
                .api
                .admit_one_shot_saved_plan(*rebuilt, &saved_bytes)
                .map(Box::new)
                .map(CommandOutcome::OneShotAdmittedPlan),
            #[cfg(feature = "native-local")]
            CommandRequest::CurrentExecutableBindingVerification(request) => self
                .api
                .compile_local_one_shot(request)
                .and_then(|compilation| self.api.verify_current_executable_binding(&compilation))
                .map(CommandOutcome::CurrentExecutableBindingVerification),
            CommandRequest::ResidentHook(request) => match self.api.admit_resident_hook(request) {
                Ok(admission) => self
                    .api
                    .deliver_resident_hook(self.resident_attachment.as_ref(), admission)
                    .map(CommandOutcome::ResidentHook),
                Err(error) => Err(error),
            },
            CommandRequest::ResidentOperationObservation(request) => self
                .api
                .observe_resident_operation(self.resident_attachment.as_ref(), request)
                .map(CommandOutcome::ResidentOperationObservation),
            CommandRequest::ResidentInvokeStart(request) => self
                .api
                .start_resident_invoke(self.resident_attachment.as_ref(), request)
                .map(CommandOutcome::ResidentInvokeStarted),
            CommandRequest::ResidentInvokeAdvance(request) => self
                .api
                .advance_resident_invoke(self.resident_attachment.as_ref(), request)
                .map(CommandOutcome::ResidentInvokeAdvanced),
            CommandRequest::ResidentConsultStart(request) => self
                .api
                .start_resident_consult(self.resident_attachment.as_ref(), request)
                .map(CommandOutcome::ResidentConsultStarted),
            CommandRequest::ResidentConsultAdvance(request) => self
                .api
                .advance_resident_consult(self.resident_attachment.as_ref(), request)
                .map(CommandOutcome::ResidentConsultAdvanced),
            CommandRequest::ResidentDelegateStart(request) => self
                .api
                .start_resident_delegate(self.resident_attachment.as_ref(), request)
                .map(CommandOutcome::ResidentDelegateStarted),
            CommandRequest::ResidentDelegateAdvance(request) => self
                .api
                .advance_resident_delegate(self.resident_attachment.as_ref(), request)
                .map(CommandOutcome::ResidentDelegateAdvanced),
            CommandRequest::ResidentOperationCancellation(request) => self
                .api
                .cancel_resident_operation(self.resident_attachment.as_ref(), request)
                .map(CommandOutcome::ResidentOperationCancellation),
        }
    }

    /// Reconstruct one comparison from its complete captured inputs.
    fn compile_comparison(
        &self,
        prior: PortableWorkspaceRequest,
        current: PortableWorkspaceRequest,
    ) -> Result<PortableWorkspaceComparison, ApiError> {
        let prior = self.api.compile_portable_workspace(prior)?;
        let current = self.api.compile_portable_workspace(current)?;
        self.api.compare_portable_workspaces(&prior, &current)
    }

    /// Compile captured sources and return the compilation to the calling step.
    fn compile_one_shot(
        &self,
        request: OneShotCompilationRequest,
    ) -> Result<OneShotCompilation, ApiError> {
        self.api.compile_one_shot(request)
    }

    /// Compile one exact profile from its freshly admitted portable owner.
    fn compile_effective_profile(
        &self,
        inputs: EffectiveProfileInputs,
    ) -> Result<EffectiveProfileCompilation, ApiError> {
        let EffectiveProfileInputs { workspace, profile } = inputs;
        self.api
            .compile_portable_workspace(workspace)
            .and_then(|workspace| self.api.compile_effective_profile(&workspace, profile))
    }
}

#[cfg(test)]
#[path = "../tests/internal/command.rs"]
mod tests;
