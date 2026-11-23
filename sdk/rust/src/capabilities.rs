//! Identify API operations and the interfaces allowed to expose them.

/// Stable identity of one operation exposed by the public façade.
///
/// The variants identify use cases, not Rust methods or transport commands. A
/// leaf (such as the CLI or a language binding) presents eligible operations
/// in its own environment. [`crate::PaleApi`] or one of its opaque result
/// handles still implements the operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApiCapabilityId {
    /// Inspect one canonical `Delegate` handle and syntax-check its claimed record reference.
    A2aDelegateHandleInspection,
    /// Read the identity of the bundled schemas.
    ContractBundleIdentity,
    /// Look up one selected public generated schema.
    ContractSchemaLookup,
    /// Validate and canonicalize one selected public contract document.
    ContractDocumentValidation,
    /// Project one typed owning failure into `diagnostic.v1`.
    DiagnosticProjection,
    /// Admit exact canonical `diagnostic.v1` bytes.
    DiagnosticAdmission,
    /// Inspect captured Package bytes for lock authoring or repair.
    PackageDistributionInspection,
    /// Read checked Package declarations and reading links for authoring tools.
    PackageCatalogInspection,
    /// Read the input limits for one kind of authoring source.
    AuthoringIngressLimits,
    /// Validate one standalone authoring document at its owning grammar.
    StandaloneAuthoringValidation,
    /// Inspect generated grammar facts for one standalone authoring role.
    StandaloneAuthoringInspection,
    /// Compile a captured path-free workspace source family.
    PortableWorkspaceCompilation,
    /// Inspect authoring grammar retained by an admitted portable workspace.
    PortableWorkspaceAuthoringInspection,
    /// Resolve one source occurrence retained by a portable workspace.
    PortableWorkspaceAuthoringNavigation,
    /// Page exact source occurrences using a retained portable-workspace owner.
    PortableWorkspaceAuthoringOccurrences,
    /// Map captured source to proof subjects and page their direct occurrences.
    PortableWorkspaceSourceSubjects,
    /// Compare two admitted portable workspace compilations.
    PortableWorkspaceComparison,
    /// Discover selected subject identities across a retained comparison pair.
    PortableWorkspaceComparisonSubjects,
    /// Compare one subject's membership and typed value in a retained pair.
    PortableWorkspaceSubjectComparison,
    /// Page direct proof inputs from one retained compilation.
    PortableWorkspaceDirectDependencies,
    /// Scan direct proof consumers within candidate and returned-row limits.
    PortableWorkspaceDirectDependents,
    /// Inspect one subject's fact, direct inputs and source locations.
    PortableWorkspaceSubjectInspection,
    /// Page retained organization records, typed details and exact source mappings.
    PortableWorkspaceOrganizationInspection,
    /// Page the change-root partition of a workspace comparison.
    PortableWorkspaceChangeRoots,
    /// Page the affected and unaffected partition of a workspace comparison.
    PortableWorkspaceImpacts,
    /// Page the verified change roots contributing to one comparison subject.
    PortableWorkspaceContributingRoots,
    /// Explain one comparison subject from its retained causal proof.
    PortableWorkspaceExplanation,
    /// Compile one selected effective profile over an admitted workspace.
    EffectiveProfileCompilation,
    /// Page one admitted effective-profile inspection.
    EffectiveProfileInspection,
    /// Explain one retained effective-profile finding.
    EffectiveProfileExplanation,
    /// Inspect authoring grammar retained by an effective-profile compilation.
    EffectiveProfileAuthoringInspection,
    /// Resolve one source occurrence retained by an effective-profile compilation.
    EffectiveProfileAuthoringNavigation,
    /// Page exact source occurrences using a retained effective-profile owner.
    EffectiveProfileAuthoringOccurrences,
    /// Join one resolved authoring identity to effective-profile facts.
    EffectiveProfileAuthoringJoin,
    /// Page all profile rows associated with one retained authoring identity.
    EffectiveProfileAuthoringPages,
    /// Conduct one human-governed configuration review without mutating source.
    ConfigurationImprovement,
    /// Project one admitted effective profile into a bounded observation snapshot.
    ObservationSnapshot,
    /// Page one generation-bound observation snapshot.
    ObservationPage,
    /// Associate an observed subject with rows from its retained profile.
    ObservationProfileInspection,
    /// Derive one complete contiguous observation delta.
    ObservationDelta,
    /// Read one retained application-owned causal path.
    ObservationCausalPath,
    /// Build one bounded disposable observation lens.
    ObservationVisualLens,
    /// Inspect the closed effect-free native-action catalog.
    NativeActionCatalog,
    /// Invoke one complete tagged effect-free native-action request.
    NativeActionInvocation,
    /// Invoke one carrier-selected effect-free native-action payload.
    NativeActionPayloadInvocation,
    /// Admit a repository-bound native-action catalog.
    RepositoryNativeActionAdmission,
    /// Admit one repository-native event at the resident hook boundary.
    ResidentHookAdmission,
    /// Correlate captured Codex command items without carrier or operation authority.
    CodexCommandObservation,
    /// Observe one retained operation through a caller-provisioned resident attachment.
    ResidentOperationObservation,
    /// Start one durable root Invoke.
    ResidentInvokeStart,
    /// Advance one retained Invoke by at most one bounded owner turn.
    ResidentInvokeAdvance,
    /// Start one already-selected durable Consult.
    ResidentConsultStart,
    /// Advance one retained Consult by at most one bounded turn.
    ResidentConsultAdvance,
    /// Start one pairwise Delegate child under its exact parent.
    ResidentDelegateStart,
    /// Advance one Delegate through its already-selected dispatch owner.
    ResidentDelegateAdvance,
    /// Send one Cancel request without equating acknowledgement with terminal state.
    ResidentOperationCancellation,
    /// Compile one complete captured host/workspace source family.
    OneShotCompilation,
    /// Inspect an admitted one-shot compilation through its owned summary.
    OneShotCompilationSummary,
    /// Derive the observations required before effect-free planning.
    OneShotObservationTargets,
    /// Construct an effect-free plan candidate from admitted observations.
    OneShotPlanning,
    /// Bind machine-local effect candidates into one complete plan.
    OneShotMachineEffectBinding,
    /// Admit saved plan bytes against one freshly reconstructed plan.
    OneShotSavedPlanAdmission,
    /// Inspect local module content without publication or runtime admission.
    LocalWorkspaceSourceInspection,
    /// Diagnose selected local configuration and machine observations without repair.
    LocalDoctorInspection,
    /// Resolve, capture, and compile one source-backed local workspace.
    LocalOneShotCompilation,
    /// Capture and inspect one workspace-selected local Package distribution.
    LocalPackageDistributionInspection,
    /// Capture a local distribution and read its checked Package catalog.
    LocalPackageCatalogInspection,
    /// Regenerate checked Package family indexes in their source directories.
    LocalPackageIndexUpdate,
    /// Capture one confined local file into a path-free native-action request.
    LocalNativeActionInputCapture,
    /// Compare one local repository with its admitted desired projection.
    LocalRepositoryStatus,
    /// Construct one complete local materialization plan from trusted observations.
    LocalOneShotPlanning,
    /// Publish one complete local plan into a confined output directory.
    LocalOneShotPlanPublication,
    /// Admit saved plans and target effects for one complete local run.
    LocalMaterializationRunAdmission,
    /// Settle one admitted local run with repository-scoped evidence.
    LocalMaterializationRunSettlement,
    /// Inspect bounded read-back after an unknown local settlement outcome.
    LocalMaterializationReconciliationInspection,
    /// Construct the interface identity compiled into the local façade.
    CompiledInterfaceIdentity,
    /// Verify this process against one workspace-bound executable binding.
    CurrentExecutableBindingVerification,
    /// Inspect one canonical local control store without opening a runtime.
    LocalControlStorageInspection,
    /// Capture baseline and candidate configuration-review roles from local workspaces.
    LocalConfigurationImprovementCapture,
    /// Capture local configuration and compile a review preview without writing it.
    LocalConfigurationImprovementPreview,
    /// Capture exact native baselines and propose source-guidance reconciliation.
    LocalSourceGuidancePlanning,
    /// Rebuild, admit and publish an exact reviewed source-guidance proposal.
    LocalSourceGuidanceWrite,
    /// Inspect a pending recovery with current native evidence and logical scope.
    LocalSourceGuidanceRecoveryPlanning,
    /// Rebuild and execute an exact reviewed native recovery decision.
    LocalSourceGuidanceRecoveryExecution,
}

impl ApiCapabilityId {
    /// Return the stable transport-neutral spelling of this capability.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::A2aDelegateHandleInspection => "a2a.delegate-handle-inspection",
            Self::ContractBundleIdentity => "contract.bundle-identity",
            Self::ContractSchemaLookup => "contract.schema-lookup",
            Self::ContractDocumentValidation => "contract.document-validation",
            Self::DiagnosticProjection => "diagnostic.projection",
            Self::DiagnosticAdmission => "diagnostic.admission",
            Self::PackageDistributionInspection => "package-distribution.inspection",
            Self::PackageCatalogInspection => "package-catalog.inspection",
            Self::AuthoringIngressLimits => "authoring.ingress-limits",
            Self::StandaloneAuthoringValidation => "authoring.standalone-validation",
            Self::StandaloneAuthoringInspection => "authoring.standalone-inspection",
            Self::PortableWorkspaceCompilation => "workspace.portable-compilation",
            Self::PortableWorkspaceAuthoringInspection => "workspace.portable-authoring-inspection",
            Self::PortableWorkspaceAuthoringNavigation => "workspace.portable-authoring-navigation",
            Self::PortableWorkspaceAuthoringOccurrences => {
                "workspace.portable-authoring-occurrences"
            }
            Self::PortableWorkspaceSourceSubjects => "workspace.portable-source-subjects",
            Self::PortableWorkspaceComparison => "workspace.portable-comparison",
            Self::PortableWorkspaceComparisonSubjects => "workspace.portable-comparison-subjects",
            Self::PortableWorkspaceSubjectComparison => "workspace.portable-subject-comparison",
            Self::PortableWorkspaceDirectDependencies => "workspace.portable-direct-dependencies",
            Self::PortableWorkspaceDirectDependents => "workspace.portable-direct-dependents",
            Self::PortableWorkspaceSubjectInspection => "workspace.portable-subject-inspection",
            Self::PortableWorkspaceOrganizationInspection => {
                "workspace.portable-organization-inspection"
            }
            Self::PortableWorkspaceChangeRoots => "workspace.portable-change-roots",
            Self::PortableWorkspaceImpacts => "workspace.portable-impacts",
            Self::PortableWorkspaceContributingRoots => "workspace.portable-contributing-roots",
            Self::PortableWorkspaceExplanation => "workspace.portable-explanation",
            Self::EffectiveProfileCompilation => "workspace.effective-profile-compilation",
            Self::EffectiveProfileInspection => "workspace.effective-profile-inspection",
            Self::EffectiveProfileExplanation => "workspace.effective-profile-explanation",
            Self::EffectiveProfileAuthoringInspection => {
                "workspace.effective-profile-authoring-inspection"
            }
            Self::EffectiveProfileAuthoringNavigation => {
                "workspace.effective-profile-authoring-navigation"
            }
            Self::EffectiveProfileAuthoringOccurrences => {
                "workspace.effective-profile-authoring-occurrences"
            }
            Self::EffectiveProfileAuthoringJoin => "workspace.effective-profile-authoring-join",
            Self::EffectiveProfileAuthoringPages => "workspace.effective-profile-authoring-pages",
            Self::ConfigurationImprovement => "workspace.configuration-improvement",
            Self::ObservationSnapshot => "observation.snapshot",
            Self::ObservationPage => "observation.page",
            Self::ObservationProfileInspection => "observation.profile-inspection",
            Self::ObservationDelta => "observation.delta",
            Self::ObservationCausalPath => "observation.causal-path",
            Self::ObservationVisualLens => "observation.visual-lens",
            Self::NativeActionCatalog => "native-action.catalog",
            Self::NativeActionInvocation => "native-action.invocation",
            Self::NativeActionPayloadInvocation => "native-action.payload-invocation",
            Self::RepositoryNativeActionAdmission => "native-action.repository-admission",
            Self::ResidentHookAdmission => "resident-hook.admission",
            Self::CodexCommandObservation => "resident-hook.codex-command-observation",
            Self::ResidentOperationObservation => "resident.operation-observation",
            Self::ResidentInvokeStart => "resident.invoke-start",
            Self::ResidentInvokeAdvance => "resident.invoke-advance",
            Self::ResidentConsultStart => "resident.consult-start",
            Self::ResidentConsultAdvance => "resident.consult-advance",
            Self::ResidentDelegateStart => "resident.delegate-start",
            Self::ResidentDelegateAdvance => "resident.delegate-advance",
            Self::ResidentOperationCancellation => "resident.operation-cancellation",
            Self::OneShotCompilation => "one-shot.compilation",
            Self::OneShotCompilationSummary => "one-shot.compilation-summary",
            Self::OneShotObservationTargets => "one-shot.observation-targets",
            Self::OneShotPlanning => "one-shot.planning",
            Self::OneShotMachineEffectBinding => "one-shot.machine-effect-binding",
            Self::OneShotSavedPlanAdmission => "one-shot.saved-plan-admission",
            Self::LocalWorkspaceSourceInspection => "local.workspace-source-inspection",
            Self::LocalDoctorInspection => "local.doctor-inspection",
            Self::LocalOneShotCompilation => "local.one-shot-compilation",
            Self::LocalPackageDistributionInspection => "local.package-distribution-inspection",
            Self::LocalPackageCatalogInspection => "local.package-catalog-inspection",
            Self::LocalPackageIndexUpdate => "local.package-index-update",
            Self::LocalNativeActionInputCapture => "local.native-action-input-capture",
            Self::LocalRepositoryStatus => "local.repository-status",
            Self::LocalOneShotPlanning => "local.one-shot-planning",
            Self::LocalOneShotPlanPublication => "local.one-shot-plan-publication",
            Self::LocalMaterializationRunAdmission => "local.materialization-run-admission",
            Self::LocalMaterializationRunSettlement => "local.materialization-run-settlement",
            Self::LocalMaterializationReconciliationInspection => {
                "local.materialization-reconciliation-inspection"
            }
            Self::CompiledInterfaceIdentity => "local.compiled-interface-identity",
            Self::CurrentExecutableBindingVerification => {
                "local.current-executable-binding-verification"
            }
            Self::LocalControlStorageInspection => "local.control-storage-inspection",
            Self::LocalConfigurationImprovementCapture => "local.configuration-improvement-capture",
            Self::LocalConfigurationImprovementPreview => "local.configuration-improvement-preview",
            Self::LocalSourceGuidancePlanning => "local.source-guidance-planning",
            Self::LocalSourceGuidanceWrite => "local.source-guidance-write",
            Self::LocalSourceGuidanceRecoveryPlanning => "local.source-guidance-recovery-planning",
            Self::LocalSourceGuidanceRecoveryExecution => {
                "local.source-guidance-recovery-execution"
            }
        }
    }
}

/// Execution locality of one API capability.
///
/// Locality is independent of [`ApiCapabilityEffect`]. In particular, a
/// [`Self::Native`] capability can consume host-specific captured input while
/// remaining effect-free.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiCapabilityClass {
    /// Deterministic computation over owned, path-free values.
    Portable,
    /// Computation over captured host, workspace, or repository context.
    Native,
    /// An operation backed by the resident operation service.
    Resident,
}

/// Truthful identity of one capability input or output contract.
///
/// [`Self::PublicSchema`] names an embedded generated contract admitted by the
/// façade. [`Self::ApiProjection`] names an owned API DTO or opaque handle for
/// which no public generated schema currently exists. The latter is not a
/// schema identity and must not be passed to contract lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiCapabilityContract {
    /// Registry identity of an embedded public generated schema.
    PublicSchema(&'static str),
    /// Stable name of an API-owned projection or opaque handle.
    ApiProjection(&'static str),
}

impl ApiCapabilityContract {
    /// Return the schema identity or projection name carried by this value.
    #[must_use]
    pub const fn identity(self) -> &'static str {
        match self {
            Self::PublicSchema(identity) | Self::ApiProjection(identity) => identity,
        }
    }
}

/// Consequence class of a public capability invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiCapabilityEffect {
    /// The invocation cannot plan or cause an external effect.
    EffectFree,
    /// The invocation constructs an immutable plan candidate but causes no effect.
    EffectFreePlanning,
    /// The invocation admits authority for a later effect but causes no effect itself.
    EffectFreeAdmission,
    /// The invocation may cause a confined native external effect.
    NativeEffect,
    /// The invocation may change resident custody or continue an admitted dispatch.
    ResidentEffect,
}

/// How a public capability returns control to its caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiInvocationMode {
    /// The current call returns its result or diagnostic directly.
    Immediate,
}

/// Cancellation semantics owned by one public capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiCancellationSemantics {
    /// No application-owned operation handle or cancellation transition exists.
    Unavailable,
    /// Cancellation is sampled before bounded projection and returns a typed outcome.
    CooperativePreflight,
}

/// One selected consumer leaf over the public façade.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApiLeaf {
    /// Direct use of the Rust façade.
    Rust,
    /// Typed command dispatch and terminal projection.
    Cli,
    /// Agent transport over the CLI's typed dispatcher.
    Mcp,
    /// Continuous Pale and workspace-configuration authoring support.
    Lsp,
    /// Interactive stable-v1 coding-agent session projection.
    Acp,
    /// Selected but contract-gated remote-agent delegation boundary.
    A2a,
    /// Generated TypeScript contracts, validators, and pure builders.
    TypeScript,
    /// Native Node.js bindings.
    Node,
    /// Complete idiomatic Go bindings.
    Go,
    /// Native Python bindings.
    Python,
    /// Capability-confined portable WebAssembly bindings.
    Wasm,
}

/// Closed conformance metadata for one current API use case.
///
/// This record never selects or invokes an implementation. It is a static
/// allowlist used by leaves and their conformance tests; semantic dispatch
/// remains in the owning API operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiCapability {
    id: ApiCapabilityId,
    class: ApiCapabilityClass,
    input: Option<ApiCapabilityContract>,
    output: Option<ApiCapabilityContract>,
    effect: ApiCapabilityEffect,
    invocation: ApiInvocationMode,
    cancellation: ApiCancellationSemantics,
    eligible_leaves: &'static [ApiLeaf],
}

impl ApiCapability {
    /// Return the stable use-case identity.
    #[must_use]
    pub const fn id(self) -> ApiCapabilityId {
        self.id
    }

    /// Return the operation's portable, native, or resident locality.
    #[must_use]
    pub const fn class(self) -> ApiCapabilityClass {
        self.class
    }

    /// Return the input contract, or `None` when the operation has no input.
    #[must_use]
    pub const fn input(self) -> Option<ApiCapabilityContract> {
        self.input
    }

    /// Return the output contract, or `None` when the operation has no output.
    #[must_use]
    pub const fn output(self) -> Option<ApiCapabilityContract> {
        self.output
    }

    /// Return the invocation's consequence class.
    #[must_use]
    pub const fn effect(self) -> ApiCapabilityEffect {
        self.effect
    }

    /// Return how the invocation yields its result.
    #[must_use]
    pub const fn invocation(self) -> ApiInvocationMode {
        self.invocation
    }

    /// Return the application-owned cancellation semantics.
    #[must_use]
    pub const fn cancellation(self) -> ApiCancellationSemantics {
        self.cancellation
    }

    /// Return the deliberately unequal leaf allowlist in stable order.
    #[must_use]
    pub const fn eligible_leaves(self) -> &'static [ApiLeaf] {
        self.eligible_leaves
    }

    /// Report whether one leaf may project this capability.
    #[must_use]
    pub fn eligible_for(self, leaf: ApiLeaf) -> bool {
        self.eligible_leaves.contains(&leaf)
    }
}

/// Build one static capability row without exposing a public constructor.
const fn capability(
    id: ApiCapabilityId,
    class: ApiCapabilityClass,
    input: Option<ApiCapabilityContract>,
    output: Option<ApiCapabilityContract>,
    effect: ApiCapabilityEffect,
    eligible_leaves: &'static [ApiLeaf],
) -> ApiCapability {
    capability_with_cancellation(
        id,
        class,
        input,
        output,
        effect,
        ApiCancellationSemantics::Unavailable,
        eligible_leaves,
    )
}

/// Build a capability row with explicit cooperative cancellation semantics.
const fn capability_with_cancellation(
    id: ApiCapabilityId,
    class: ApiCapabilityClass,
    input: Option<ApiCapabilityContract>,
    output: Option<ApiCapabilityContract>,
    effect: ApiCapabilityEffect,
    cancellation: ApiCancellationSemantics,
    eligible_leaves: &'static [ApiLeaf],
) -> ApiCapability {
    ApiCapability {
        id,
        class,
        input,
        output,
        effect,
        invocation: ApiInvocationMode::Immediate,
        cancellation,
        eligible_leaves,
    }
}

use ApiCapabilityClass::{Native, Portable, Resident};
use ApiCapabilityContract::{ApiProjection, PublicSchema};
#[cfg(feature = "native-local")]
use ApiCapabilityEffect::NativeEffect;
use ApiCapabilityEffect::{EffectFree, EffectFreeAdmission, EffectFreePlanning, ResidentEffect};
use ApiCapabilityId::{
    A2aDelegateHandleInspection, AuthoringIngressLimits, ConfigurationImprovement,
    ContractBundleIdentity, ContractDocumentValidation, ContractSchemaLookup, DiagnosticAdmission,
    DiagnosticProjection, EffectiveProfileAuthoringInspection, EffectiveProfileAuthoringJoin,
    EffectiveProfileAuthoringNavigation, EffectiveProfileAuthoringOccurrences,
    EffectiveProfileAuthoringPages, EffectiveProfileCompilation, EffectiveProfileExplanation,
    EffectiveProfileInspection, NativeActionCatalog, NativeActionInvocation,
    NativeActionPayloadInvocation, ObservationCausalPath, ObservationDelta, ObservationPage,
    ObservationProfileInspection, ObservationSnapshot, ObservationVisualLens, OneShotCompilation,
    OneShotCompilationSummary, OneShotMachineEffectBinding, OneShotObservationTargets,
    OneShotPlanning, OneShotSavedPlanAdmission, PackageCatalogInspection,
    PackageDistributionInspection, PortableWorkspaceAuthoringInspection,
    PortableWorkspaceAuthoringNavigation, PortableWorkspaceAuthoringOccurrences,
    PortableWorkspaceChangeRoots, PortableWorkspaceComparison, PortableWorkspaceComparisonSubjects,
    PortableWorkspaceCompilation, PortableWorkspaceContributingRoots,
    PortableWorkspaceDirectDependencies, PortableWorkspaceDirectDependents,
    PortableWorkspaceExplanation, PortableWorkspaceImpacts,
    PortableWorkspaceOrganizationInspection, PortableWorkspaceSourceSubjects,
    PortableWorkspaceSubjectComparison, PortableWorkspaceSubjectInspection,
    RepositoryNativeActionAdmission, ResidentConsultAdvance, ResidentConsultStart,
    ResidentDelegateAdvance, ResidentDelegateStart, ResidentHookAdmission, ResidentInvokeAdvance,
    ResidentInvokeStart, ResidentOperationCancellation, ResidentOperationObservation,
    StandaloneAuthoringInspection, StandaloneAuthoringValidation,
};
#[cfg(feature = "native-local")]
use ApiCapabilityId::{
    CompiledInterfaceIdentity, CurrentExecutableBindingVerification,
    LocalConfigurationImprovementCapture, LocalConfigurationImprovementPreview,
    LocalControlStorageInspection, LocalDoctorInspection,
    LocalMaterializationReconciliationInspection, LocalMaterializationRunAdmission,
    LocalMaterializationRunSettlement, LocalNativeActionInputCapture, LocalOneShotCompilation,
    LocalOneShotPlanPublication, LocalOneShotPlanning, LocalPackageCatalogInspection,
    LocalPackageDistributionInspection, LocalPackageIndexUpdate, LocalRepositoryStatus,
    LocalWorkspaceSourceInspection,
};
use ApiLeaf::{A2a, Acp, Cli, Go, Lsp, Mcp, Node, Python, Rust, TypeScript, Wasm};

/// Leaves that inspect a durable Delegate identity without launching it.
const A2A_DELEGATE_HANDLE_LEAVES: &[ApiLeaf] = &[Rust, A2a];

/// Leaves that can inspect and validate the portable public contract bundle.
const CONTRACT_LEAVES: &[ApiLeaf] = &[Rust, Cli, Mcp, TypeScript, Node, Go, Python, Wasm];
/// Leaves that compile, compare, and explain portable workspaces.
const PORTABLE_COMPUTE_LEAVES: &[ApiLeaf] = &[Rust, Cli, Mcp, Lsp, Acp, Node, Go, Python, Wasm];
/// Leaves that directly expose canonical diagnostic construction.
const DIAGNOSTIC_PROJECTION_LEAVES: &[ApiLeaf] = &[Rust, Cli, Acp, Node, Go, Python, Wasm];
/// Leaves that directly admit canonical diagnostic bytes.
const DIAGNOSTIC_ADMISSION_LEAVES: &[ApiLeaf] = &[Rust, Cli, Node, Go, Python, Wasm];
/// Leaves that inspect captured Package distribution bytes and release coordinates.
const PACKAGE_INSPECTION_LEAVES: &[ApiLeaf] = &[Rust, Cli, Lsp, Node, Go, Python];
/// Leaves that inspect built-in actions and invoke one carrier-selected payload.
const CATALOG_PAYLOAD_ACTION_LEAVES: &[ApiLeaf] = &[Rust, Cli, Mcp, Node, Go, Python, Wasm];
/// Leaves that admit the complete tagged union instead of descriptor-bound payloads.
const TAGGED_ACTION_LEAVES: &[ApiLeaf] = &[Rust, Cli, Node, Go, Python, Wasm];
/// Leaves allowed to consume captured host or repository context.
const ONE_SHOT_PLANNING_LEAVES: &[ApiLeaf] = &[Rust, Cli, Mcp, Node, Go, Python];
/// Leaves selected for authority-bearing one-shot admission.
const ONE_SHOT_ADMISSION_LEAVES: &[ApiLeaf] = &[Rust, Cli, Node, Go, Python];
/// Leaves that expose API-owned local filesystem and host journeys.
#[cfg(feature = "native-local")]
const NATIVE_LOCAL_LEAVES: &[ApiLeaf] = &[Rust, Cli, Node, Go, Python];
/// Leaves that can admit repository-selected action catalogs today.
const REPOSITORY_ACTION_ADMISSION_LEAVES: &[ApiLeaf] = &[Rust, Cli, Node, Go, Python];
/// Leaves that receive native repository events before an authenticated carrier exists.
const RESIDENT_HOOK_LEAVES: &[ApiLeaf] = &[Rust, Cli, Node, Go, Python];
/// Leaves that can receive a pre-signed read-only resident Observe request.
const RESIDENT_OBSERVE_LEAVES: &[ApiLeaf] = &[Rust, Cli, Mcp, A2a];
/// Leaves that can start and advance root Invoke and Consult journeys.
const RESIDENT_OPERATION_LEAVES: &[ApiLeaf] = &[Rust, Cli, Mcp];
/// Leaves that can start and advance an already-authorized Delegate child.
const RESIDENT_DELEGATE_LEAVES: &[ApiLeaf] = &[Rust, Cli, Mcp, A2a];
/// Leaves that can request cancellation without claiming terminal state.
const RESIDENT_CANCELLATION_LEAVES: &[ApiLeaf] = &[Rust, Cli, Mcp, A2a];
/// Leaves that compile, inspect, and explain the selected effective profile.
const PROFILE_INSPECTION_LEAVES: &[ApiLeaf] = &[Rust, Cli, Mcp, Lsp, Acp, Node, Go, Python];
/// Leaves that inspect profile proof and source joins after compilation.
const PROFILE_PROOF_LEAVES: &[ApiLeaf] = &[Rust, Cli, Lsp, Acp, Node, Go, Python];
/// Leaves selected for the currently registered native observation operations.
const OBSERVATION_LEAVES: &[ApiLeaf] = &[Rust, Cli, Node, Go, Python];
/// Leaves that expose the standalone ingress ceilings directly.
const AUTHORING_LIMIT_LEAVES: &[ApiLeaf] = &[Rust, Cli, Acp, Node, Go, Python, Wasm];
/// Leaves that project portable compiler-owned authoring intelligence.
const AUTHORING_LEAVES: &[ApiLeaf] = &[Rust, Cli, Lsp, Acp, Node, Go, Python, Wasm];
/// Leaves with a complete current projection of the human-governed review typestate.
const CONFIGURATION_IMPROVEMENT_LEAVES: &[ApiLeaf] = &[Rust, Cli, Mcp, Lsp, Acp, Node, Go, Python];
/// Leaves that invoke the standalone authoring validator directly.
const STANDALONE_AUTHORING_VALIDATION_LEAVES: &[ApiLeaf] =
    &[Rust, Cli, Lsp, Acp, Node, Go, Python, Wasm];

/// Current closed capability inventory in stable identity order.
const CAPABILITIES: &[ApiCapability] = &[
    capability(
        A2aDelegateHandleInspection,
        Portable,
        Some(ApiProjection("a2a-delegate-handle-request")),
        Some(ApiProjection("a2a-delegate-handle")),
        EffectFree,
        A2A_DELEGATE_HANDLE_LEAVES,
    ),
    capability(
        ContractBundleIdentity,
        Portable,
        None,
        Some(ApiProjection("contract-bundle-identity")),
        EffectFree,
        CONTRACT_LEAVES,
    ),
    capability(
        ContractSchemaLookup,
        Portable,
        Some(ApiProjection("contract-schema-identity")),
        Some(ApiProjection("contract-schema")),
        EffectFree,
        CONTRACT_LEAVES,
    ),
    capability(
        ContractDocumentValidation,
        Portable,
        Some(ApiProjection("contract-document")),
        Some(ApiProjection("validated-contract-document")),
        EffectFree,
        CONTRACT_LEAVES,
    ),
    capability(
        DiagnosticProjection,
        Portable,
        Some(ApiProjection("diagnostic-projection")),
        Some(PublicSchema("diagnostic.v1")),
        EffectFree,
        DIAGNOSTIC_PROJECTION_LEAVES,
    ),
    capability(
        DiagnosticAdmission,
        Portable,
        Some(PublicSchema("diagnostic.v1")),
        Some(PublicSchema("diagnostic.v1")),
        EffectFree,
        DIAGNOSTIC_ADMISSION_LEAVES,
    ),
    capability(
        PackageDistributionInspection,
        Portable,
        Some(ApiProjection("package-distribution-request")),
        Some(ApiProjection("package-distribution-inspection")),
        EffectFree,
        PACKAGE_INSPECTION_LEAVES,
    ),
    capability(
        PackageCatalogInspection,
        Portable,
        Some(ApiProjection("package-distribution-request")),
        Some(ApiProjection("package-catalog-inspection")),
        EffectFree,
        &[Rust, Cli],
    ),
    capability(
        AuthoringIngressLimits,
        Portable,
        Some(ApiProjection("authoring-ingress-class")),
        Some(ApiProjection("authoring-ingress-limits")),
        EffectFree,
        AUTHORING_LIMIT_LEAVES,
    ),
    capability(
        StandaloneAuthoringValidation,
        Portable,
        Some(ApiProjection("standalone-authoring-document")),
        Some(ApiProjection("standalone-authoring-validation")),
        EffectFree,
        STANDALONE_AUTHORING_VALIDATION_LEAVES,
    ),
    capability(
        StandaloneAuthoringInspection,
        Portable,
        Some(ApiProjection("standalone-authoring-inspection-request")),
        Some(ApiProjection("authoring-inspection")),
        EffectFree,
        AUTHORING_LEAVES,
    ),
    capability(
        PortableWorkspaceCompilation,
        Portable,
        Some(ApiProjection("portable-workspace-request")),
        Some(ApiProjection("portable-workspace-compilation")),
        EffectFree,
        PORTABLE_COMPUTE_LEAVES,
    ),
    capability(
        PortableWorkspaceAuthoringInspection,
        Portable,
        Some(ApiProjection(
            "portable-workspace-authoring-inspection-request",
        )),
        Some(ApiProjection("authoring-inspection")),
        EffectFree,
        AUTHORING_LEAVES,
    ),
    capability(
        PortableWorkspaceAuthoringNavigation,
        Portable,
        Some(ApiProjection(
            "portable-workspace-authoring-navigation-request",
        )),
        Some(ApiProjection("authoring-navigation")),
        EffectFree,
        AUTHORING_LEAVES,
    ),
    capability(
        PortableWorkspaceAuthoringOccurrences,
        Portable,
        Some(ApiProjection("authoring-occurrence-page-request")),
        Some(ApiProjection("authoring-occurrence-page")),
        EffectFree,
        &[Rust, Node, Go, Python, Wasm],
    ),
    capability(
        PortableWorkspaceSourceSubjects,
        Portable,
        Some(ApiProjection("portable-workspace-source-subject-request")),
        Some(ApiProjection("portable-workspace-source-subject-result")),
        EffectFree,
        &[Rust, Node],
    ),
    capability(
        PortableWorkspaceComparison,
        Portable,
        Some(ApiProjection("portable-workspace-comparison-input")),
        Some(ApiProjection("portable-workspace-comparison")),
        EffectFree,
        PORTABLE_COMPUTE_LEAVES,
    ),
    capability(
        PortableWorkspaceComparisonSubjects,
        Portable,
        Some(ApiProjection(
            "portable-workspace-comparison-subject-page-request",
        )),
        Some(ApiProjection("portable-workspace-comparison-subject-page")),
        EffectFree,
        &[Rust, Node, Python],
    ),
    capability(
        PortableWorkspaceSubjectComparison,
        Portable,
        Some(ApiProjection("portable-workspace-subject-selector")),
        Some(ApiProjection("portable-workspace-subject-comparison")),
        EffectFree,
        &[Rust, Node, Go, Python, Wasm],
    ),
    capability(
        PortableWorkspaceDirectDependencies,
        Portable,
        Some(ApiProjection(
            "portable-workspace-direct-dependency-request",
        )),
        Some(ApiProjection("portable-workspace-dependency-page")),
        EffectFree,
        &[Rust, Node, Python],
    ),
    capability(
        PortableWorkspaceDirectDependents,
        Portable,
        Some(ApiProjection("portable-workspace-direct-dependent-request")),
        Some(ApiProjection("portable-workspace-dependent-page")),
        EffectFree,
        &[Rust, Node],
    ),
    capability(
        PortableWorkspaceSubjectInspection,
        Portable,
        Some(ApiProjection(
            "portable-workspace-subject-inspection-request",
        )),
        Some(ApiProjection("portable-workspace-subject-inspection")),
        EffectFree,
        &[Rust, Node],
    ),
    capability(
        PortableWorkspaceOrganizationInspection,
        Portable,
        Some(ApiProjection(
            "portable-workspace-organization-inspection-request",
        )),
        Some(ApiProjection("portable-workspace-organization-page")),
        EffectFree,
        &[Rust, Node],
    ),
    capability(
        PortableWorkspaceChangeRoots,
        Portable,
        Some(ApiProjection("portable-workspace-change-root-page-request")),
        Some(ApiProjection("portable-workspace-change-root-page")),
        EffectFree,
        PORTABLE_COMPUTE_LEAVES,
    ),
    capability(
        PortableWorkspaceImpacts,
        Portable,
        Some(ApiProjection("portable-workspace-impact-page-request")),
        Some(ApiProjection("portable-workspace-impact-page")),
        EffectFree,
        PORTABLE_COMPUTE_LEAVES,
    ),
    capability(
        PortableWorkspaceContributingRoots,
        Portable,
        Some(ApiProjection(
            "portable-workspace-contributing-root-page-request",
        )),
        Some(ApiProjection("portable-workspace-contributing-root-page")),
        EffectFree,
        &[Rust, Node],
    ),
    capability(
        PortableWorkspaceExplanation,
        Portable,
        Some(ApiProjection("portable-workspace-explanation-request")),
        Some(ApiProjection("portable-workspace-explanation")),
        EffectFree,
        PORTABLE_COMPUTE_LEAVES,
    ),
    capability(
        EffectiveProfileCompilation,
        Native,
        Some(ApiProjection("effective-profile-compilation-request")),
        Some(ApiProjection("effective-profile-compilation")),
        EffectFree,
        PROFILE_INSPECTION_LEAVES,
    ),
    capability(
        EffectiveProfileInspection,
        Native,
        Some(ApiProjection("effective-profile-page-request")),
        Some(ApiProjection("effective-profile-page")),
        EffectFree,
        PROFILE_INSPECTION_LEAVES,
    ),
    capability(
        EffectiveProfileExplanation,
        Native,
        Some(ApiProjection("effective-profile-explanation-handle")),
        Some(ApiProjection("effective-profile-explanation")),
        EffectFree,
        PROFILE_INSPECTION_LEAVES,
    ),
    capability(
        EffectiveProfileAuthoringInspection,
        Native,
        Some(ApiProjection(
            "effective-profile-authoring-inspection-request",
        )),
        Some(ApiProjection("authoring-inspection")),
        EffectFree,
        PROFILE_PROOF_LEAVES,
    ),
    capability(
        EffectiveProfileAuthoringNavigation,
        Native,
        Some(ApiProjection(
            "effective-profile-authoring-navigation-request",
        )),
        Some(ApiProjection("authoring-navigation")),
        EffectFree,
        PROFILE_PROOF_LEAVES,
    ),
    capability(
        EffectiveProfileAuthoringOccurrences,
        Native,
        Some(ApiProjection("authoring-occurrence-page-request")),
        Some(ApiProjection("authoring-occurrence-page")),
        EffectFree,
        &[Rust, Node, Go, Python],
    ),
    capability(
        EffectiveProfileAuthoringJoin,
        Native,
        Some(ApiProjection("effective-profile-authoring-identity")),
        Some(ApiProjection("effective-authoring-inspection")),
        EffectFree,
        PROFILE_PROOF_LEAVES,
    ),
    capability(
        EffectiveProfileAuthoringPages,
        Native,
        Some(ApiProjection("effective-authoring-page-request")),
        Some(ApiProjection("effective-authoring-page")),
        EffectFree,
        &[Rust, Node, Go, Python],
    ),
    capability(
        ConfigurationImprovement,
        Native,
        Some(ApiProjection("configuration-improvement-transition")),
        Some(ApiProjection("configuration-improvement-state")),
        EffectFree,
        CONFIGURATION_IMPROVEMENT_LEAVES,
    ),
    capability(
        ObservationSnapshot,
        Native,
        Some(ApiProjection("effective-profile-compilation")),
        Some(ApiProjection("observation-snapshot")),
        EffectFree,
        OBSERVATION_LEAVES,
    ),
    capability(
        ObservationPage,
        Native,
        Some(ApiProjection("observation-page-request")),
        Some(ApiProjection("observation-page")),
        EffectFree,
        OBSERVATION_LEAVES,
    ),
    capability(
        ObservationProfileInspection,
        Native,
        Some(ApiProjection("observation-profile-inspection-request")),
        Some(ApiProjection("observation-profile-page")),
        EffectFree,
        &[Rust, Node, Go, Python],
    ),
    capability_with_cancellation(
        ObservationDelta,
        Native,
        Some(ApiProjection("observation-delta-request")),
        Some(ApiProjection("observation-delta-outcome")),
        EffectFree,
        ApiCancellationSemantics::CooperativePreflight,
        OBSERVATION_LEAVES,
    ),
    capability(
        ObservationCausalPath,
        Native,
        Some(ApiProjection("observation-causal-selector")),
        Some(ApiProjection("observation-causal-path")),
        EffectFree,
        OBSERVATION_LEAVES,
    ),
    capability_with_cancellation(
        ObservationVisualLens,
        Native,
        Some(ApiProjection("observation-lens-request")),
        Some(ApiProjection("observation-lens-disposition")),
        EffectFree,
        ApiCancellationSemantics::CooperativePreflight,
        OBSERVATION_LEAVES,
    ),
    capability(
        NativeActionCatalog,
        Portable,
        None,
        Some(ApiProjection("native-action-descriptors")),
        EffectFree,
        CATALOG_PAYLOAD_ACTION_LEAVES,
    ),
    capability(
        NativeActionInvocation,
        Portable,
        Some(PublicSchema("action-input.v1")),
        Some(PublicSchema("action-execution.v1")),
        EffectFree,
        TAGGED_ACTION_LEAVES,
    ),
    capability(
        NativeActionPayloadInvocation,
        Portable,
        Some(ApiProjection("native-action-payload-request")),
        Some(PublicSchema("action-execution.v1")),
        EffectFree,
        CATALOG_PAYLOAD_ACTION_LEAVES,
    ),
    capability(
        RepositoryNativeActionAdmission,
        Native,
        Some(PublicSchema("repository-native-action-catalog.v1")),
        Some(ApiProjection("admitted-repository-native-actions")),
        EffectFree,
        REPOSITORY_ACTION_ADMISSION_LEAVES,
    ),
    capability(
        ResidentHookAdmission,
        Resident,
        Some(ApiProjection("resident-hook-request")),
        Some(ApiProjection("resident-hook-admission")),
        EffectFree,
        RESIDENT_HOOK_LEAVES,
    ),
    capability(
        ApiCapabilityId::CodexCommandObservation,
        Resident,
        Some(ApiProjection("codex-command-observation-input")),
        Some(ApiProjection("codex-command-observation")),
        EffectFree,
        &[Rust],
    ),
    capability(
        ResidentOperationObservation,
        Resident,
        Some(ApiProjection("resident-operation-observation-request")),
        Some(ApiProjection("resident-operation-observation")),
        EffectFree,
        RESIDENT_OBSERVE_LEAVES,
    ),
    capability(
        ResidentInvokeStart,
        Resident,
        Some(ApiProjection("resident-invoke-start-request")),
        Some(ApiProjection("resident-operation-observation")),
        ResidentEffect,
        RESIDENT_OPERATION_LEAVES,
    ),
    capability(
        ResidentInvokeAdvance,
        Resident,
        Some(ApiProjection("resident-invoke-advance-request")),
        Some(ApiProjection("resident-invoke-advance-outcome")),
        ResidentEffect,
        RESIDENT_OPERATION_LEAVES,
    ),
    capability(
        ResidentConsultStart,
        Resident,
        Some(ApiProjection("resident-consult-start-request")),
        Some(ApiProjection("resident-operation-observation")),
        ResidentEffect,
        RESIDENT_OPERATION_LEAVES,
    ),
    capability(
        ResidentConsultAdvance,
        Resident,
        Some(ApiProjection("resident-consult-advance-request")),
        Some(ApiProjection("resident-consult-advance-outcome")),
        ResidentEffect,
        RESIDENT_OPERATION_LEAVES,
    ),
    capability(
        ResidentDelegateStart,
        Resident,
        Some(ApiProjection("resident-delegate-start-request")),
        Some(ApiProjection("resident-operation-observation")),
        ResidentEffect,
        RESIDENT_DELEGATE_LEAVES,
    ),
    capability(
        ResidentDelegateAdvance,
        Resident,
        Some(ApiProjection("resident-delegate-advance-request")),
        Some(ApiProjection("resident-delegate-advance-outcome")),
        ResidentEffect,
        RESIDENT_DELEGATE_LEAVES,
    ),
    capability(
        ResidentOperationCancellation,
        Resident,
        Some(ApiProjection("resident-operation-cancellation-request")),
        Some(ApiProjection("resident-operation-cancellation-outcome")),
        ResidentEffect,
        RESIDENT_CANCELLATION_LEAVES,
    ),
    capability(
        OneShotCompilation,
        Native,
        Some(ApiProjection("one-shot-compilation-request")),
        Some(ApiProjection("one-shot-compilation")),
        EffectFree,
        ONE_SHOT_PLANNING_LEAVES,
    ),
    capability(
        OneShotCompilationSummary,
        Native,
        Some(ApiProjection("one-shot-compilation")),
        Some(ApiProjection("one-shot-compilation-summary")),
        EffectFree,
        ONE_SHOT_PLANNING_LEAVES,
    ),
    capability(
        OneShotObservationTargets,
        Native,
        Some(ApiProjection("one-shot-observation-target-request")),
        Some(ApiProjection("one-shot-observation-targets")),
        EffectFree,
        ONE_SHOT_PLANNING_LEAVES,
    ),
    capability(
        OneShotPlanning,
        Native,
        Some(ApiProjection("one-shot-planning-request")),
        Some(ApiProjection("one-shot-plan-candidate")),
        EffectFreePlanning,
        ONE_SHOT_PLANNING_LEAVES,
    ),
    capability(
        OneShotMachineEffectBinding,
        Native,
        Some(ApiProjection("one-shot-machine-effect-binding-request")),
        Some(ApiProjection("one-shot-bound-plan")),
        EffectFreePlanning,
        ONE_SHOT_ADMISSION_LEAVES,
    ),
    capability(
        OneShotSavedPlanAdmission,
        Native,
        Some(ApiProjection("one-shot-saved-plan-admission-request")),
        Some(ApiProjection("one-shot-admitted-plan")),
        EffectFreeAdmission,
        ONE_SHOT_ADMISSION_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalWorkspaceSourceInspection,
        Native,
        Some(ApiProjection("local-workspace-locator")),
        Some(ApiProjection("workspace-source-inspection")),
        EffectFree,
        &[Rust, Cli],
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalDoctorInspection,
        Native,
        Some(ApiProjection("local-doctor-request")),
        Some(ApiProjection("doctor-inspection")),
        EffectFree,
        &[Rust, Cli],
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalOneShotCompilation,
        Native,
        Some(ApiProjection("local-one-shot-compilation-request")),
        Some(ApiProjection("local-one-shot-compilation")),
        EffectFree,
        NATIVE_LOCAL_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalPackageDistributionInspection,
        Native,
        Some(ApiProjection("local-package-distribution-request")),
        Some(ApiProjection("package-distribution-inspection")),
        EffectFree,
        NATIVE_LOCAL_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalPackageCatalogInspection,
        Native,
        Some(ApiProjection("local-package-catalog-root")),
        Some(ApiProjection("package-catalog-inspection")),
        EffectFree,
        &[Rust, Cli],
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalPackageIndexUpdate,
        Native,
        Some(ApiProjection("local-package-catalog-root")),
        Some(ApiProjection("package-catalog-inspection")),
        NativeEffect,
        &[Rust, Cli],
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalNativeActionInputCapture,
        Native,
        Some(ApiProjection("local-native-action-input-request")),
        Some(ApiProjection("native-action-request")),
        EffectFree,
        NATIVE_LOCAL_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalRepositoryStatus,
        Native,
        Some(ApiProjection("local-repository-status-request")),
        Some(ApiProjection("local-repository-status")),
        EffectFree,
        NATIVE_LOCAL_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalOneShotPlanning,
        Native,
        Some(ApiProjection("local-one-shot-plan-request")),
        Some(ApiProjection("local-one-shot-plan")),
        EffectFreePlanning,
        NATIVE_LOCAL_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalOneShotPlanPublication,
        Native,
        Some(ApiProjection("local-one-shot-plan-publication-request")),
        Some(ApiProjection("local-one-shot-plan-publication")),
        NativeEffect,
        NATIVE_LOCAL_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalMaterializationRunAdmission,
        Native,
        Some(ApiProjection("local-materialization-admission-request")),
        Some(ApiProjection("local-materialization-admitted-run")),
        EffectFreeAdmission,
        NATIVE_LOCAL_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalMaterializationRunSettlement,
        Native,
        Some(ApiProjection("local-materialization-admitted-run")),
        Some(ApiProjection("local-materialization-run-result")),
        NativeEffect,
        NATIVE_LOCAL_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalMaterializationReconciliationInspection,
        Native,
        Some(ApiProjection("local-materialization-admitted-run")),
        Some(ApiProjection("local-materialization-reconciliation")),
        EffectFree,
        NATIVE_LOCAL_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        CompiledInterfaceIdentity,
        Native,
        None,
        Some(ApiProjection("compiled-interface-identity")),
        EffectFree,
        NATIVE_LOCAL_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        CurrentExecutableBindingVerification,
        Native,
        Some(ApiProjection("local-one-shot-compilation")),
        Some(ApiProjection("compiled-interface-identity")),
        EffectFree,
        NATIVE_LOCAL_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalControlStorageInspection,
        Native,
        Some(ApiProjection("local-control-storage-inspection-request")),
        Some(ApiProjection("local-control-storage-inspection")),
        EffectFree,
        NATIVE_LOCAL_LEAVES,
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalConfigurationImprovementCapture,
        Native,
        Some(ApiProjection("local-configuration-improvement-request")),
        Some(ApiProjection("configuration-improvement-sources")),
        EffectFree,
        &[Rust, Cli],
    ),
    #[cfg(feature = "native-local")]
    capability(
        LocalConfigurationImprovementPreview,
        Native,
        Some(ApiProjection("local-configuration-improvement-request")),
        Some(ApiProjection("configuration-improvement-preview")),
        EffectFree,
        &[Rust, Node, Go, Python],
    ),
    #[cfg(feature = "native-local")]
    capability(
        ApiCapabilityId::LocalSourceGuidancePlanning,
        Native,
        Some(ApiProjection("local-source-guidance-plan-request")),
        Some(ApiProjection("local-source-guidance-plan-report")),
        EffectFreePlanning,
        &[Rust, Cli],
    ),
    #[cfg(feature = "native-local")]
    capability(
        ApiCapabilityId::LocalSourceGuidanceWrite,
        Native,
        Some(ApiProjection("local-source-guidance-write-request")),
        Some(ApiProjection("local-source-guidance-write-result")),
        NativeEffect,
        &[Rust, Cli],
    ),
    #[cfg(feature = "native-local")]
    capability(
        ApiCapabilityId::LocalSourceGuidanceRecoveryPlanning,
        Native,
        Some(ApiProjection("local-source-guidance-recovery-request")),
        Some(ApiProjection("local-source-guidance-recovery-plan")),
        EffectFreePlanning,
        &[Rust, Cli],
    ),
    #[cfg(feature = "native-local")]
    capability(
        ApiCapabilityId::LocalSourceGuidanceRecoveryExecution,
        Native,
        Some(ApiProjection(
            "local-source-guidance-recovery-execution-request",
        )),
        Some(ApiProjection("local-source-guidance-recovery-result")),
        NativeEffect,
        &[Rust, Cli],
    ),
];

/// Return the immutable current API capability inventory.
///
/// The slice is conformance and exposure metadata only. Looking up a row does
/// not initialize [`crate::PaleApi`], dispatch an operation, grant authority,
/// or advertise implementation readiness for an eligible leaf.
#[must_use]
pub const fn capability_inventory() -> &'static [ApiCapability] {
    CAPABILITIES
}
