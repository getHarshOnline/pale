//! Owned, bounded projections of one admitted effective profile.
//!
//! The application compiler owns every classification projected here. This
//! module performs only an owned translation and deterministic pagination; it
//! does not parse source, recompute closure, inspect a machine, or infer intent.

use std::collections::{BTreeMap, BTreeSet};

use pale_application::compile::{
    EffectiveProfileFinding as ApplicationFinding,
    EffectiveProfileNarrowingReason as ApplicationNarrowingReason,
    EffectiveProfileNativeRealization as ApplicationNativeRealization,
    EffectiveProfileOperationExposure as ApplicationOperationExposure,
    EffectiveProfilePolicyBlock as ApplicationPolicyBlock,
    EffectiveProfileProviderDisposition as ApplicationProviderDisposition,
    EffectiveProfileRequirementDisposition as ApplicationRequirementDisposition,
    EffectiveProfileResourceKind as ApplicationResourceKind,
};
use pale_application::semantic::ProviderUnavailability;
use pale_core::records::{
    CapabilityProvider, CompositionReasonStepV1, EffectClass, ExecutionWorkspaceRepositoryAccessV1,
    ModeAxis, PackageResourceSelectorV1, ParticipantRefV1, ProviderBindingRefV1, ResponsibilityV1,
};

use crate::{AuthoringIdentityKind, PortableWorkspaceSubjectSelector};

/// Page size selected when an effective-profile request supplies zero.
const DEFAULT_PAGE_SIZE: u16 = 64;
/// Maximum effective-profile entries returned in one page.
const MAX_PAGE_SIZE: u16 = 256;

/// Exact semantic coordinates shared by every page of one profile inspection.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
pub struct EffectiveProfileCoordinates {
    /// Principal workspace owning the desired graph.
    pub workspace_id: String,
    /// Selected logical project.
    pub project_id: String,
    /// Selected logical execution workspace.
    pub execution_workspace_id: String,
    /// Selected private machine-binding identity.
    pub machine_id: String,
    /// Canonical portable profile identity.
    pub profile_digest: String,
    /// Canonical private machine-resolved instance identity.
    ///
    /// This coordinate proves which admitted private projection was compiled;
    /// it does not claim machine or native realization.
    pub instance_digest: String,
    /// Identity of the exact project-relative desired topology selection.
    pub selected_topology_digest: String,
    /// Identity of the admitted Package semantic closure.
    pub package_semantic_ir_digest: String,
    /// Identity of the neutral experience-composition proof.
    pub composition_plan_digest: String,
    /// Identity of the admitted workspace semantic graph.
    pub workspace_semantic_ir_digest: String,
    /// Identity of the owner-proof set used for workspace admission.
    pub workspace_admission_evidence_digest: String,
}

/// The five deliberately shallow Profile Explorer sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileSection {
    /// Declared responsibility and selected path evidence.
    IntendedUseCase,
    /// Effective releases, policy, repositories, and semantic selections.
    EffectiveConfiguration,
    /// Operations and their independently typed capability requirements.
    CapabilitiesAndOperations,
    /// Separately qualified machine/native realization.
    MachineRealization,
    /// Exact compiler-owned gaps, narrowing, and policy findings.
    GapsAndExplanations,
}

/// One exact selected configuration coordinate.
///
/// The closed variants avoid delimiter-composed identifiers and optional
/// prose fields. Every value therefore states the exact coordinate family it
/// carries, while the application remains the owner of why it was selected.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileSelection {
    /// Immutable Package release.
    Package {
        /// Package identity.
        package_id: String,
        /// Selected Package version.
        version: String,
        /// Digest of the exact Package release bytes.
        content_digest: String,
    },
    /// Attestor-scoped normative Pack release.
    NormativePack {
        /// Normative Pack identity.
        pack_id: String,
        /// Attestor owning the normative release statement.
        attestor_id: String,
        /// Selected Pack version.
        version: String,
        /// Digest of the exact Pack release bytes.
        content_digest: String,
    },
    /// Source-free Package resource selected for one repository.
    SemanticResource {
        /// Logical repository receiving the resource.
        repository_id: String,
        /// Exact semantic resource family.
        resource_kind: EffectiveProfileResourceKind,
        /// Owner-qualified Package resource identity.
        resource_id: String,
    },
    /// Selected private run policy.
    RunPolicy {
        /// Exact run-policy identity.
        run_policy_id: String,
    },
    /// Selected session model represented by bounded cardinalities.
    Session {
        /// Execution workspace owning the session selection.
        execution_workspace_id: String,
        /// Number of activation roots.
        activation_root_count: u32,
        /// Number of additional roots.
        additional_root_count: u32,
        /// Number of activation paths.
        activation_path_count: u32,
    },
    /// One admitted lifecycle behavior.
    Lifecycle {
        /// Lifecycle behavior identity.
        lifecycle_id: String,
        /// Typed lifecycle event identity.
        event_id: String,
        /// Typed lifecycle action identity.
        action_id: String,
    },
    /// Logical authority-root ceiling.
    AuthorityRoot {
        /// Exact authority-root identity.
        authority_root_id: String,
    },
    /// Volatile forbidden-token authority coordinate.
    DisclosureCeiling {
        /// Exact forbidden-token authority coordinate.
        authority_coordinate: String,
    },
}

/// One normalized selector retained from declared private intent.
///
/// This remains distinct from [`EffectiveProfileSelection::SemanticResource`],
/// which represents a member of the compiler-expanded effective closure.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileDeclaredSelector {
    /// One exact owner-qualified Package resource was declared.
    Exact {
        /// Exact resource selected by the principal declaration.
        resource_id: String,
    },
    /// Every exported resource of one typed family from one Package was declared.
    AllFromPackage {
        /// Package scope of the typed wildcard.
        package_id: String,
    },
}

/// Closed runtime-scoped mode axis used by a declared mode default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileModeAxis {
    /// Amount of process and verification guidance.
    Intensity,
    /// Kind of specialist task stance.
    TaskStance,
}

/// One normalized member of the admitted declared semantic selection.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileDeclaredSelection {
    /// One exact or Package-scoped resource selector.
    Resource {
        /// Semantic family supplied by the enclosing declaration field.
        resource_kind: EffectiveProfileResourceKind,
        /// Exact or Package-scoped selector retained without closure expansion.
        selector: EffectiveProfileDeclaredSelector,
    },
    /// One explicit project-level runtime mode default.
    ModeDefault {
        /// Orthogonal runtime-mode dimension being defaulted.
        axis: EffectiveProfileModeAxis,
        /// Exact owner-qualified run-mode resource.
        resource_id: String,
    },
}

/// Closed Package semantic family of one selected resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileResourceKind {
    /// Operation procedure.
    Operation,
    /// Specialist role.
    Role,
    /// Standalone doctrine.
    Doctrine,
    /// Doctrine placed into startup policy.
    StartupPolicy,
    /// Run-mode declaration.
    RunMode,
    /// Technology or workflow stack.
    Stack,
    /// Policy-family body.
    PolicyFamily,
    /// Reusable Package recipe.
    Recipe,
}

/// Maximum repository access admitted by the execution-workspace edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileRepositoryAccess {
    /// Read-only ceiling.
    Read,
    /// Mutation may be considered by a later independent effect admission.
    Mutate,
}

/// Exact opaque provider reference selected at a path's terminal leaf.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
pub struct EffectiveProfileProviderBindingRef {
    /// Stable provider-binding locator.
    pub provider_binding_id: String,
    /// Domain-separated identity of the complete provider-binding body.
    pub provider_binding_identity: String,
}

/// Portable effect ceiling retained without native-effect authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileEffectClass {
    /// Pure caller-supplied computation.
    Compute,
    /// Non-mutating read.
    Read,
    /// Local command execution without remote write.
    Execute,
    /// Local state mutation.
    Write,
    /// Remote read.
    Network,
    /// Remote or outward-facing write.
    Publish,
}

/// Participant kind owning one selected responsibility contract.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileParticipant {
    /// Contract-facing interface.
    Interface {
        /// Stable interface identity.
        interface_id: String,
    },
    /// Native application.
    Application {
        /// Stable application identity.
        application_id: String,
    },
    /// Participant encoded by the initial-v1 `engine` wire variant.
    Engine {
        /// Stable identity carried by the `engine` wire variant.
        engine_id: String,
    },
    /// Neutral capability.
    Capability {
        /// Stable capability identity.
        capability_id: String,
    },
    /// Terminal leaf.
    TerminalLeaf {
        /// Stable terminal-leaf identity.
        terminal_leaf_id: String,
    },
    /// Plugin participant.
    Plugin {
        /// Stable plugin identity.
        plugin_id: String,
    },
    /// Skill participant.
    Skill {
        /// Stable skill identity.
        skill_id: String,
    },
}

/// One typed responsibility assigned by the semantic owner.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileResponsibility {
    /// Human or agent communication.
    Communication,
    /// Organizational planning, delegation, or acceptance.
    OrganizationalWork,
    /// Customer-relationship or other business-owned state.
    CrmBusinessState,
    /// Engineering work through its native execution owner.
    EngineeringExecution,
    /// Context or memory resolution without ownership transfer.
    ContextMemory,
    /// Terminal harness or model-backed execution.
    TerminalHarnessExecution,
    /// Publication through a native event or outbox boundary.
    NativeEventPublication,
    /// Native effect followed by owner read-back.
    NativeEffectReadBack,
    /// Separately governed extension responsibility.
    Extension {
        /// Namespace owning the extension vocabulary.
        namespace_id: String,
        /// Contract identity within that namespace.
        contract_id: String,
        /// Exact admitted schema member.
        schema_id: String,
        /// SHA-256 of the exact schema bytes.
        schema_sha256: String,
    },
}

/// Exact three-state operation exposure contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileOperationExposure {
    /// No private operation binding exists.
    Unbound,
    /// A binding exists and Package meaning declares no capabilities.
    SemanticOnly,
    /// A binding exists and Package-owned capability requirements are present.
    CapabilityBearing,
}

/// Portable disposition of one Package-owned capability requirement.
///
/// This axis is independent of [`EffectiveProfileOperationExposure`],
/// [`EffectiveProfileProviderDisposition`], and
/// [`EffectiveProfileNativeRealization`]. In particular, an explicit
/// forbidden-set member is blocked, while absence from a positive permitted
/// set is narrowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileRequirementDisposition {
    /// A private provider binding satisfies the portable requirement.
    Satisfied,
    /// The required capability has no private provider binding.
    ProviderBindingAbsent,
    /// A positive private permitted set omitted this occurrence.
    Narrowed {
        /// Exact omitted dimension or dimensions.
        reason: EffectiveProfileNarrowingReason,
    },
    /// An explicit private denial or present provider contradicts policy.
    Blocked {
        /// Exact forbidden or provider-contract dimension.
        reason: EffectiveProfilePolicyBlock,
    },
}

/// Exact positive-set relation that narrowed a requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileNarrowingReason {
    /// The capability is absent from `permitted_capabilities`.
    CapabilityNotPermitted,
    /// The effect is absent from `permitted_effects`.
    EffectNotPermitted,
    /// Both required dimensions are absent from their permitted sets.
    CapabilityAndEffectNotPermitted,
}

/// Exact policy dimension blocking a present provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfilePolicyBlock {
    /// Private policy explicitly forbids the capability.
    CapabilityForbidden,
    /// Private policy explicitly forbids the effect.
    EffectForbidden,
    /// Private policy explicitly forbids both dimensions.
    CapabilityAndEffectForbidden,
    /// Provider effect differs from the requirement effect.
    ProviderEffectMismatch,
    /// Provider does not implement the required recipe contract.
    RecipeContractMismatch,
}

/// Closed provider kind selected by private configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileProviderKind {
    /// Native-harness tool binding.
    HostNative,
    /// Pale-owned bounded Rust mechanic.
    NativeRust,
    /// Admitted connector binding.
    Connector,
}

/// Private provider-selection disposition, independent of native readiness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileProviderDisposition {
    /// A private provider kind is selected; provider execution availability is unknown.
    Bound {
        /// Selected closed provider kind.
        provider: EffectiveProfileProviderKind,
    },
    /// No private binding exists for the capability.
    MissingBinding,
    /// Private narrowing excluded the requirement occurrence.
    ExcludedByNarrowing,
    /// Provider effect contradicts the requirement effect.
    EffectMismatch,
    /// Provider does not implement the required recipe contract.
    RecipeContractMismatch,
}

/// Owner-native realization evidence available to this inspection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileNativeRealization {
    /// No owner-native observation has been admitted.
    Unknown,
}

/// One structured, application-owned finding.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileFinding {
    /// No owner-native observation has qualified the selected machine.
    NativeRealizationUnknown {
        /// Selected private machine-binding identity.
        machine_id: String,
    },
    /// One selected semantic operation has no private operation binding.
    OperationUnbound {
        /// Logical repository containing the operation occurrence.
        repository_id: String,
        /// Owner-qualified Package operation identity.
        operation_id: String,
    },
    /// One capability requirement's exact portable disposition.
    Requirement {
        /// Logical repository containing the requirement occurrence.
        repository_id: String,
        /// Owner-qualified Package operation identity.
        operation_id: String,
        /// Requirement identity local to the operation.
        requirement_id: String,
        /// Portable capability identity.
        capability_id: String,
        /// Application-owned portable disposition.
        disposition: EffectiveProfileRequirementDisposition,
    },
}

/// One typed causal selector retained by a selected execution path.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfilePathReason {
    /// Owner-issued origin receipt.
    OriginReceipt {
        /// Exact origin-receipt contract identity.
        contract_id: String,
    },
    /// Entry contract from which traversal begins.
    EntryContract {
        /// Exact entry-contract identity.
        contract_id: String,
    },
    /// Responsibility contract used by the path.
    ResponsibilityContract {
        /// Exact responsibility-contract identity.
        contract_id: String,
    },
    /// Explicit provider binding required by the path.
    ProviderBinding {
        /// Exact provider-binding identity.
        binding_id: String,
    },
    /// Ordered neutral delegation edge.
    DelegationEdge {
        /// Exact neutral delegation-edge identity.
        edge_id: String,
    },
    /// Terminal contract bounding completion.
    TerminationContract {
        /// Exact termination-contract identity.
        contract_id: String,
    },
    /// Settlement contract required after traversal.
    SettlementContract {
        /// Exact settlement-contract identity.
        contract_id: String,
    },
    /// Continuity contract surviving path completion.
    ContinuityContract {
        /// Exact continuity-contract identity.
        contract_id: String,
    },
}

/// Opaque identity of one explanation retained by an effective-profile handle.
///
/// The value is a domain-separated SHA-256 established by `pale-sdk`. Its
/// representation reveals neither compiler indices nor source coordinates.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub struct EffectiveProfileExplanationHandle(
    #[cfg_attr(
        feature = "wire-schema",
        schemars(length(equal = 64), regex(pattern = r"^[0-9a-f]{64}$"))
    )]
    String,
);

impl EffectiveProfileExplanationHandle {
    /// Validate one handle received at an outer transport boundary.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidEffectiveProfileExplanationHandle`] unless the value is
    /// exactly one lowercase hexadecimal SHA-256 representation.
    pub fn parse(value: String) -> Result<Self, InvalidEffectiveProfileExplanationHandle> {
        if pale_core::is_sha256_hex(&value) {
            Ok(Self(value))
        } else {
            Err(InvalidEffectiveProfileExplanationHandle)
        }
    }

    /// Borrow the opaque lowercase hexadecimal representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> serde::Deserialize<'de> for EffectiveProfileExplanationHandle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// An outer value cannot represent an API-owned profile explanation handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("effective-profile explanation handle must be 64 lowercase hexadecimal characters")]
pub struct InvalidEffectiveProfileExplanationHandle;

/// Bounded compiler evidence retained for one explainable profile entry.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileExplanationEvidence {
    /// Exact declared selector, before dependency or wildcard closure expansion.
    DeclaredSelection {
        /// Normalized declared selection copied from application inspection.
        selection: EffectiveProfileDeclaredSelection,
    },
    /// One application-admitted responsibility with its composition reasons.
    Responsibility {
        /// Exact responsibility contract.
        contract_id: String,
        /// Selected path carrying the contract.
        execution_path_id: String,
        /// Ordered application-owned composition reasons.
        reason_steps: Vec<EffectiveProfilePathReason>,
    },
    /// One application-admitted effective selection.
    EffectiveSelection {
        /// Exact selected configuration coordinate.
        selection: EffectiveProfileSelection,
    },
    /// One selected execution path and its retained composition proof.
    ExecutionPath {
        /// Stable selected path identity.
        selection_id: String,
        /// Ordered application-owned composition reasons.
        reason_steps: Vec<EffectiveProfilePathReason>,
    },
    /// One operation's application-owned three-state classification.
    OperationClassification {
        /// Exact exposure classification.
        exposure: EffectiveProfileOperationExposure,
        /// Independently qualified native evidence.
        native_realization: EffectiveProfileNativeRealization,
    },
    /// One requirement's independent application-owned disposition axes.
    CapabilityClassification {
        /// Portable configuration result.
        requirement_disposition: EffectiveProfileRequirementDisposition,
        /// Private provider selection result.
        provider_disposition: EffectiveProfileProviderDisposition,
        /// Independently qualified native evidence.
        native_realization: EffectiveProfileNativeRealization,
    },
    /// One exact application-owned finding classification.
    FindingClassification {
        /// Typed finding without presentation prose.
        finding: EffectiveProfileFinding,
    },
    /// No owner-native observation was admitted for this machine coordinate.
    MachineRealization {
        /// Independently qualified native evidence.
        realization: EffectiveProfileNativeRealization,
    },
    /// Exact retained configuration evidence not requiring a second inference.
    Configuration,
}

/// One bounded explanation returned from the retained API projection.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
pub struct EffectiveProfileExplanation {
    /// Opaque handle used for exact request/result correlation.
    pub handle: EffectiveProfileExplanationHandle,
    /// Typed application-owned proof or classification evidence.
    pub evidence: EffectiveProfileExplanationEvidence,
}

/// One owned, typed row in an effective-profile section.
///
/// The variants keep repository access, execution-path ceilings, operation
/// exposure, provider selection, and native realization non-substitutable.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum EffectiveProfileEntryDetail {
    /// One normalized declared selector, before effective closure expansion.
    DeclaredSelection {
        /// Exact declared selection retained by application inspection.
        selection: EffectiveProfileDeclaredSelection,
    },
    /// One admitted responsibility contract required by a selected path.
    Responsibility {
        /// Exact responsibility-contract identity.
        contract_id: String,
        /// Execution-path selection carrying this contract.
        execution_path_id: String,
        /// Participant bounded by the contract.
        participant: EffectiveProfileParticipant,
        /// Typed responsibilities assigned by the semantic owner.
        responsibilities: Vec<EffectiveProfileResponsibility>,
        /// Maximum effect classes admitted for this contract.
        authority_ceiling: Vec<EffectiveProfileEffectClass>,
        /// Ordered application-owned reasons for the selected carrying path.
        reason_steps: Vec<EffectiveProfilePathReason>,
        /// Existing causal-proof selector.
        subject: PortableWorkspaceSubjectSelector,
    },
    /// One exact closure, policy, lifecycle, or semantic-resource selection.
    Selection {
        /// Closed structured selection coordinate.
        selection: EffectiveProfileSelection,
    },
    /// Private capability/effect narrowing applied after Package policy.
    CapabilityNarrowing {
        /// Capabilities retained by private configuration.
        permitted_capabilities: Vec<String>,
        /// Capabilities forbidden by private configuration.
        forbidden_capabilities: Vec<String>,
        /// Effects retained by private configuration.
        permitted_effects: Vec<EffectiveProfileEffectClass>,
        /// Effects forbidden by private configuration.
        forbidden_effects: Vec<EffectiveProfileEffectClass>,
    },
    /// One normalized Package-policy meet with exact contributor provenance.
    ///
    /// The application owner applies the canonical capability-policy meet to
    /// the contributors. This projection copies that admitted result; it does
    /// not restate or recompute the meet.
    GoverningCeiling {
        /// Semantic scope of the result.
        scope: String,
        /// Governed Package resource.
        subject_resource: String,
        /// Canonical semantic contributors.
        contributors: Vec<String>,
        /// Exact Package policy resources whose bodies were met.
        policies: Vec<String>,
        /// Capabilities permitted by every contributor.
        permitted_capabilities: Vec<String>,
        /// Capabilities forbidden by any contributor.
        forbidden_capabilities: Vec<String>,
        /// Effects permitted by every contributor.
        permitted_effects: Vec<EffectiveProfileEffectClass>,
        /// Effects forbidden by any contributor.
        forbidden_effects: Vec<EffectiveProfileEffectClass>,
    },
    /// One repository in exact effective materialization order.
    Repository {
        /// Logical repository identity.
        repository_id: String,
        /// Zero-based effective materialization order.
        materialization_order: u32,
        /// Maximum access admitted by the execution-workspace edge.
        access_ceiling: EffectiveProfileRepositoryAccess,
    },
    /// One admitted provider-native projection over a selected repository.
    ProviderRepository {
        /// Canonical logical repository selected by the execution workspace.
        repository_id: String,
        /// Neutral provider-binding identity selected by an experience path.
        provider_binding_id: String,
        /// Opaque provider-owned repository coordinate.
        native_repository_coordinate: String,
        /// Owner-native availability remains separately qualified.
        native_realization: EffectiveProfileNativeRealization,
    },
    /// One selected execution path and its exact responsibility/ceiling proof.
    ExecutionPath {
        /// Stable execution-path selection identity.
        selection_id: String,
        /// Selected experience-path identity.
        experience_path_id: String,
        /// Domain-separated responsibility-path identity.
        responsibility_path_identity: String,
        /// Exact responsibility contracts consumed by the path.
        responsibility_contract_ids: Vec<String>,
        /// Ordered neutral delegation edges traversed by the path.
        work_delegation_edge_ids: Vec<String>,
        /// Explicit provider-binding declarations required by the path.
        provider_binding_ids: Vec<String>,
        /// Portable capability ceiling.
        capability_ceiling: Vec<String>,
        /// Portable effect ceiling.
        effect_ceiling: Vec<EffectiveProfileEffectClass>,
        /// Exact terminal provider binding derived from the path contract.
        ///
        /// `None` is reserved for a native-only path with no terminal leaf.
        terminal_provider_binding: Option<EffectiveProfileProviderBindingRef>,
        /// Native evidence remains independently qualified.
        native_realization: EffectiveProfileNativeRealization,
        /// Ordered causal selectors retained by the composition proof.
        reason_steps: Vec<EffectiveProfilePathReason>,
        /// Existing composition-proof selector.
        subject: PortableWorkspaceSubjectSelector,
    },
    /// One selected operation and its exact private exposure state.
    Operation {
        /// Logical repository containing this occurrence.
        repository_id: String,
        /// Owner-qualified Package operation identity.
        operation_id: String,
        /// Exact three-state operation exposure.
        exposure: EffectiveProfileOperationExposure,
        /// Native availability remains separately qualified.
        native_realization: EffectiveProfileNativeRealization,
    },
    /// One Package-owned capability requirement.
    Capability {
        /// Portable capability identity.
        capability_id: String,
        /// Logical repository containing the occurrence.
        repository_id: String,
        /// Owner-qualified operation identity.
        operation_id: String,
        /// Requirement identity local to the operation.
        requirement_id: String,
        /// Portable effect class.
        effect_class: EffectiveProfileEffectClass,
        /// Portable configuration disposition.
        requirement_disposition: EffectiveProfileRequirementDisposition,
        /// Private provider selection, independently typed.
        provider_disposition: EffectiveProfileProviderDisposition,
        /// Native availability remains separately qualified.
        native_realization: EffectiveProfileNativeRealization,
    },
    /// Separately qualified realization of the profile on one machine.
    MachineRealization {
        /// Selected private machine-binding identity.
        machine_id: String,
        /// Current owner-native evidence ceiling.
        realization: EffectiveProfileNativeRealization,
    },
    /// One exact application-owned gap or narrowing result.
    Finding {
        /// Closed structured finding without presentation prose.
        finding: EffectiveProfileFinding,
    },
}

/// One explainable row in a deterministic effective-profile page.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub struct EffectiveProfileEntry {
    /// Opaque API-owned explanation identity for this exact profile row.
    pub explanation_handle: EffectiveProfileExplanationHandle,
    /// Closed row payload serialized without an additional wrapper object.
    #[serde(flatten)]
    pub detail: EffectiveProfileEntryDetail,
}

/// Bounded effective-profile meaning associated with one authored identity.
///
/// The source identity comes from the canonical authoring index and the rows
/// come from the already retained profile projection. Joining them therefore
/// does not parse source, reconstruct closure, or infer a readiness state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveAuthoringInspection {
    coordinates: EffectiveProfileCoordinates,
    entries: Vec<EffectiveProfileEntry>,
    truncated: bool,
}

impl EffectiveAuthoringInspection {
    /// Exact admitted profile coordinate owning every returned row.
    #[must_use]
    pub const fn coordinates(&self) -> &EffectiveProfileCoordinates {
        &self.coordinates
    }

    /// Existing Profile Explorer rows associated with the authored identity.
    #[must_use]
    pub fn entries(&self) -> &[EffectiveProfileEntry] {
        &self.entries
    }

    /// Whether the first page omitted additional retained rows.
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

impl EffectiveProfileEntry {
    /// Borrow this row's opaque explanation identity.
    #[must_use]
    pub const fn explanation_handle(&self) -> &EffectiveProfileExplanationHandle {
        &self.explanation_handle
    }

    /// Borrow the closed typed row payload.
    #[must_use]
    pub const fn detail(&self) -> &EffectiveProfileEntryDetail {
        &self.detail
    }
}

/// One deterministic page from a retained effective-profile projection.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
pub struct EffectiveProfilePage {
    /// Section represented by this page.
    section: EffectiveProfileSection,
    /// Exact zero-based offset requested for this page.
    ///
    /// This remains the requested coordinate even when it lies beyond the
    /// terminal entry. Language transports use it to reject cross-page reply
    /// substitution without interpreting profile content.
    offset: u32,
    /// Complete entries in the requested bounded window.
    entries: Vec<EffectiveProfileEntry>,
    /// Next zero-based offset, or `None` for the terminal page.
    next_offset: Option<u32>,
    /// API-owned RFC 8785 projection reused by language leaves.
    #[serde(skip)]
    canonical_bytes: Vec<u8>,
}

impl EffectiveProfilePage {
    /// Section represented by this page.
    #[must_use]
    pub const fn section(&self) -> EffectiveProfileSection {
        self.section
    }

    /// Exact zero-based offset requested for this page.
    #[must_use]
    pub const fn offset(&self) -> u32 {
        self.offset
    }

    /// Complete entries in the requested bounded window.
    #[must_use]
    pub fn entries(&self) -> &[EffectiveProfileEntry] {
        &self.entries
    }

    /// Number of entries in this page.
    ///
    /// Page construction caps this collection at `MAX_PAGE_SIZE`, so the
    /// returned count is exact in the portable `u16` page-size domain.
    #[must_use]
    pub fn entry_count(&self) -> u16 {
        u16::try_from(self.entries.len()).unwrap_or(u16::MAX)
    }

    /// Next zero-based offset, or `None` for the terminal page.
    #[must_use]
    pub const fn next_offset(&self) -> Option<u32> {
        self.next_offset
    }

    /// RFC 8785 bytes of this exact bounded page.
    ///
    /// Language leaves forward these bytes instead of recreating the tagged
    /// entry algebra or becoming another canonicalization owner.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

/// Owned binding-safe selector for one bounded inspection page.
///
/// A zero `limit` requests the default of 64 entries. Values above 256 are
/// capped. Editor-generation fencing belongs to the LSP transport because the
/// API handle has no editor lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveProfilePageRequest {
    /// Profile Explorer section to inspect.
    pub section: EffectiveProfileSection,
    /// Zero-based first entry.
    pub offset: u32,
    /// Requested bounded window size.
    pub limit: u16,
}

impl EffectiveProfilePageRequest {
    /// Construct one effect-free page selector.
    #[must_use]
    pub const fn new(section: EffectiveProfileSection, offset: u32, limit: u16) -> Self {
        Self {
            section,
            offset,
            limit,
        }
    }
}

/// Owned projection constructed once from one admitted application phase.
pub(super) struct EffectiveProfileProjection {
    coordinates: EffectiveProfileCoordinates,
    intended_use_case: Vec<RetainedProfileEntry>,
    effective_configuration: Vec<RetainedProfileEntry>,
    capabilities_and_operations: Vec<RetainedProfileEntry>,
    machine_realization: Vec<RetainedProfileEntry>,
    gaps_and_explanations: Vec<RetainedProfileEntry>,
    explanations: BTreeMap<EffectiveProfileExplanationHandle, EffectiveProfileExplanation>,
    /// One small section/index pointer per row-to-subject association. Bodies
    /// remain in the five section vectors; paging never reconstructs the index.
    authoring_entries: BTreeMap<AuthoringSubject, Vec<ProfileEntryCoordinate>>,
}

/// Exact public identity namespace and compiler-owned value used for one join.
type AuthoringSubject = (AuthoringIdentityKind, String);

/// Internal pointer into one of the five immutable profile sections.
#[derive(Clone, Copy)]
struct ProfileEntryCoordinate {
    section: EffectiveProfileSection,
    index: usize,
}

/// Private provenance stays with its row; it is not another lookup index or wire field.
#[derive(Clone)]
struct RetainedProfileEntry {
    entry: EffectiveProfileEntry,
    primary_subject: Option<pale_application::observation::ObservationSubject>,
}

/// Keep a row and its optional application provenance together before assigning handles.
#[derive(Default)]
struct ProfileDetails(
    Vec<(
        EffectiveProfileEntryDetail,
        Option<pale_application::observation::ObservationSubject>,
    )>,
);

impl ProfileDetails {
    fn push(&mut self, detail: EffectiveProfileEntryDetail) {
        self.0.push((detail, None));
    }

    fn push_primary(
        &mut self,
        detail: EffectiveProfileEntryDetail,
        subject: pale_application::observation::ObservationSubject,
    ) {
        self.0.push((detail, Some(subject)));
    }
}

impl Extend<EffectiveProfileEntryDetail> for ProfileDetails {
    fn extend<T: IntoIterator<Item = EffectiveProfileEntryDetail>>(&mut self, details: T) {
        self.0
            .extend(details.into_iter().map(|detail| (detail, None)));
    }
}

impl FromIterator<EffectiveProfileEntryDetail> for ProfileDetails {
    fn from_iter<T: IntoIterator<Item = EffectiveProfileEntryDetail>>(details: T) -> Self {
        let mut result = Self::default();
        result.extend(details);
        result
    }
}

impl EffectiveProfileProjection {
    pub(super) fn observation_page(
        &self,
        query: &pale_application::observation::ObservationProfileQuery<'_>,
        position: (usize, usize),
    ) -> (
        Vec<EffectiveProfileEntry>,
        Option<(usize, usize)>,
        u16,
        pale_application::observation::ObservationProfileMapping,
    ) {
        let sections = self.sections();
        let provenance = sections
            .into_iter()
            .enumerate()
            .skip(position.0)
            .flat_map(|(section, entries)| {
                entries
                    .iter()
                    .skip(if section == position.0 { position.1 } else { 0 })
            })
            .map(|row| row.primary_subject.as_ref());
        let selected = query.select_rows(provenance);
        let entries = selected
            .selected
            .iter()
            .map(|offset| {
                let (section, row) = self
                    .advance_observation_position(position, usize::from(*offset))
                    .expect("selected provenance refers to a retained row");
                self.sections()[section][row].entry.clone()
            })
            .collect();
        let next = self.advance_observation_position(position, usize::from(selected.inspected));
        (entries, next, selected.inspected, selected.mapping)
    }

    fn sections(&self) -> [&[RetainedProfileEntry]; 5] {
        [
            &self.intended_use_case,
            &self.effective_configuration,
            &self.capabilities_and_operations,
            &self.machine_realization,
            &self.gaps_and_explanations,
        ]
    }

    /// Move by row counts, without inspecting or copying preceding row bodies.
    fn advance_observation_position(
        &self,
        (mut section, mut row): (usize, usize),
        mut offset: usize,
    ) -> Option<(usize, usize)> {
        let sections = self.sections();
        while section < sections.len() {
            let remaining = sections[section]
                .len()
                .checked_sub(row)
                .expect("private continuation lies within its retained section");
            if offset < remaining {
                return Some((section, row + offset));
            }
            offset -= remaining;
            section += 1;
            row = 0;
        }
        None
    }

    /// Translate one admitted compiler inspection into stable paged sections.
    pub(super) fn from_compilation(
        compilation: &pale_application::compile::EffectiveProfileCompilation,
    ) -> Self {
        let inspection = compilation.inspection();
        let ApplicationNativeRealization::Unknown { machine_id } = inspection.native_realization();
        let coordinates = EffectiveProfileCoordinates {
            workspace_id: inspection.workspace_id().to_owned(),
            project_id: inspection.project_id().to_owned(),
            execution_workspace_id: inspection.execution_workspace_id().to_owned(),
            machine_id: machine_id.to_owned(),
            profile_digest: inspection.profile_digest().as_str().to_owned(),
            instance_digest: compilation.instance().instance_digest.as_str().to_owned(),
            selected_topology_digest: compilation.selected_topology_digest().to_owned(),
            package_semantic_ir_digest: compilation
                .profile()
                .package_semantic_ir_digest
                .as_str()
                .to_owned(),
            composition_plan_digest: compilation
                .experience_composition_plan()
                .plan_digest
                .as_str()
                .to_owned(),
            workspace_semantic_ir_digest: compilation
                .workspace_semantic_ir_digest()
                .as_str()
                .to_owned(),
            workspace_admission_evidence_digest: compilation
                .workspace_admission_evidence_digest()
                .as_str()
                .to_owned(),
        };

        let mut intended_use_case = ProfileDetails::default();
        let mut effective_configuration = ProfileDetails::default();
        let mut capabilities_and_operations = ProfileDetails::default();
        let mut gaps_and_explanations = ProfileDetails::default();

        let declared = inspection.declared_semantic_selection();
        intended_use_case.extend(
            declared.operations.iter().map(|selector| {
                declared_selection(EffectiveProfileResourceKind::Operation, selector)
            }),
        );
        intended_use_case.extend(
            declared
                .roles
                .iter()
                .map(|selector| declared_selection(EffectiveProfileResourceKind::Role, selector)),
        );
        intended_use_case.extend(
            declared.doctrine.iter().map(|selector| {
                declared_selection(EffectiveProfileResourceKind::Doctrine, selector)
            }),
        );
        intended_use_case.extend(
            declared.run_modes.iter().map(|selector| {
                declared_selection(EffectiveProfileResourceKind::RunMode, selector)
            }),
        );
        intended_use_case.extend(declared.mode_defaults.iter().map(|(axis, resource)| {
            EffectiveProfileEntryDetail::DeclaredSelection {
                selection: EffectiveProfileDeclaredSelection::ModeDefault {
                    axis: EffectiveProfileModeAxis::from(*axis),
                    resource_id: resource.to_string(),
                },
            }
        }));
        intended_use_case.extend(
            declared
                .stacks
                .iter()
                .map(|selector| declared_selection(EffectiveProfileResourceKind::Stack, selector)),
        );

        for path in inspection.execution_paths() {
            for contract in path.responsibility_contracts() {
                intended_use_case.push(EffectiveProfileEntryDetail::Responsibility {
                    contract_id: contract.contract_id().to_owned(),
                    execution_path_id: path.selection_id().as_str().to_owned(),
                    participant: EffectiveProfileParticipant::from(contract.participant()),
                    responsibilities: contract
                        .responsibilities()
                        .iter()
                        .map(EffectiveProfileResponsibility::from)
                        .collect(),
                    authority_ceiling: contract
                        .authority_ceiling()
                        .iter()
                        .copied()
                        .map(EffectiveProfileEffectClass::from)
                        .collect(),
                    reason_steps: path
                        .reason_steps()
                        .iter()
                        .map(EffectiveProfilePathReason::from)
                        .collect(),
                    subject: PortableWorkspaceSubjectSelector::ResponsibilityClosure,
                });
            }
            effective_configuration.push_primary(
                EffectiveProfileEntryDetail::ExecutionPath {
                    selection_id: path.selection_id().as_str().to_owned(),
                    experience_path_id: path.experience_path_id().to_owned(),
                    responsibility_path_identity: path
                        .responsibility_path_identity()
                        .responsibility_path_sha256
                        .as_str()
                        .to_owned(),
                    responsibility_contract_ids: path.responsibility_contract_ids().to_vec(),
                    work_delegation_edge_ids: path.work_delegation_edge_ids().to_vec(),
                    provider_binding_ids: path.provider_binding_ids().to_vec(),
                    capability_ceiling: path.capability_ceiling().to_vec(),
                    effect_ceiling: path
                        .effect_ceiling()
                        .iter()
                        .copied()
                        .map(EffectiveProfileEffectClass::from)
                        .collect(),
                    terminal_provider_binding: path
                        .terminal_provider_binding()
                        .map(EffectiveProfileProviderBindingRef::from),
                    native_realization: EffectiveProfileNativeRealization::Unknown,
                    reason_steps: path
                        .reason_steps()
                        .iter()
                        .map(EffectiveProfilePathReason::from)
                        .collect(),
                    subject: PortableWorkspaceSubjectSelector::ExperienceComposition,
                },
                path.observation_subject(),
            );
        }

        effective_configuration.extend(inspection.package_closure().iter().map(|release| {
            EffectiveProfileEntryDetail::Selection {
                selection: EffectiveProfileSelection::Package {
                    package_id: release.id.as_str().to_owned(),
                    version: release.version.as_str().to_owned(),
                    content_digest: release.content_digest.as_str().to_owned(),
                },
            }
        }));
        effective_configuration.extend(inspection.normative_pack_closure().iter().map(|release| {
            EffectiveProfileEntryDetail::Selection {
                selection: EffectiveProfileSelection::NormativePack {
                    pack_id: release.id.as_str().to_owned(),
                    attestor_id: release.attestor.as_str().to_owned(),
                    version: release.version.as_str().to_owned(),
                    content_digest: release.content_digest.as_str().to_owned(),
                },
            }
        }));

        let run_policy = inspection.run_policy();
        effective_configuration.push(EffectiveProfileEntryDetail::Selection {
            selection: EffectiveProfileSelection::RunPolicy {
                run_policy_id: run_policy.id.as_str().to_owned(),
            },
        });

        if let Some(session) = inspection.session() {
            effective_configuration.push(EffectiveProfileEntryDetail::Selection {
                selection: EffectiveProfileSelection::Session {
                    execution_workspace_id: inspection.execution_workspace_id().to_owned(),
                    activation_root_count: bounded_count(session.activation_roots.len()),
                    additional_root_count: bounded_count(session.additional_roots.len()),
                    activation_path_count: bounded_count(session.activation_path_ids.len()),
                },
            });
        }

        effective_configuration.extend(inspection.lifecycle().iter().map(|behavior| {
            EffectiveProfileEntryDetail::Selection {
                selection: EffectiveProfileSelection::Lifecycle {
                    lifecycle_id: behavior.id.as_str().to_owned(),
                    event_id: behavior.event.as_str().to_owned(),
                    action_id: behavior.action.as_str().to_owned(),
                },
            }
        }));
        effective_configuration.extend(inspection.authority_root_ids().iter().map(|root_id| {
            EffectiveProfileEntryDetail::Selection {
                selection: EffectiveProfileSelection::AuthorityRoot {
                    authority_root_id: root_id.clone(),
                },
            }
        }));
        effective_configuration.extend(inspection.forbidden_token_authority().iter().map(
            |coordinate| EffectiveProfileEntryDetail::Selection {
                selection: EffectiveProfileSelection::DisclosureCeiling {
                    authority_coordinate: coordinate.clone(),
                },
            },
        ));

        if let Some(narrowing) = inspection.capability_narrowing() {
            effective_configuration.push(EffectiveProfileEntryDetail::CapabilityNarrowing {
                permitted_capabilities: narrowing
                    .permitted_capabilities
                    .iter()
                    .map(|value| value.as_str().to_owned())
                    .collect(),
                forbidden_capabilities: narrowing
                    .forbidden_capabilities
                    .iter()
                    .map(|value| value.as_str().to_owned())
                    .collect(),
                permitted_effects: narrowing
                    .permitted_effects
                    .iter()
                    .copied()
                    .map(EffectiveProfileEffectClass::from)
                    .collect(),
                forbidden_effects: narrowing
                    .forbidden_effects
                    .iter()
                    .copied()
                    .map(EffectiveProfileEffectClass::from)
                    .collect(),
            });
        }
        effective_configuration.extend(inspection.governing_order().ceilings().iter().map(
            |ceiling| {
                EffectiveProfileEntryDetail::GoverningCeiling {
                    scope: ceiling.scope.to_string(),
                    subject_resource: ceiling.subject.to_string(),
                    contributors: ceiling
                        .contributors
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                    policies: ceiling.policies.iter().map(ToString::to_string).collect(),
                    permitted_capabilities: ceiling
                        .policy
                        .permitted_capabilities
                        .iter()
                        .map(|value| value.as_str().to_owned())
                        .collect(),
                    forbidden_capabilities: ceiling
                        .policy
                        .forbidden_capabilities
                        .iter()
                        .map(|value| value.as_str().to_owned())
                        .collect(),
                    permitted_effects: ceiling
                        .policy
                        .permitted_effects
                        .iter()
                        .copied()
                        .map(EffectiveProfileEffectClass::from)
                        .collect(),
                    forbidden_effects: ceiling
                        .policy
                        .forbidden_effects
                        .iter()
                        .copied()
                        .map(EffectiveProfileEffectClass::from)
                        .collect(),
                }
            },
        ));

        for repository in inspection.repositories() {
            let repository_id = repository.repository_id().as_str();
            effective_configuration.push_primary(
                EffectiveProfileEntryDetail::Repository {
                    repository_id: repository_id.to_owned(),
                    materialization_order: u32::try_from(repository.materialization_order())
                        .expect("profile repository count is contract-bounded"),
                    access_ceiling: EffectiveProfileRepositoryAccess::from(
                        repository.access_ceiling(),
                    ),
                },
                repository.observation_subject(),
            );
            effective_configuration.extend(repository.selected_resources().into_iter().map(
                |resource| EffectiveProfileEntryDetail::Selection {
                    selection: EffectiveProfileSelection::SemanticResource {
                        repository_id: repository_id.to_owned(),
                        resource_kind: EffectiveProfileResourceKind::from(resource.kind()),
                        resource_id: resource.resource().to_string(),
                    },
                },
            ));

            for operation in repository.operations() {
                let operation_id = operation.operation_id().to_string();
                capabilities_and_operations.push_primary(
                    EffectiveProfileEntryDetail::Operation {
                        repository_id: repository_id.to_owned(),
                        operation_id: operation_id.clone(),
                        exposure: EffectiveProfileOperationExposure::from(operation.exposure()),
                        native_realization: EffectiveProfileNativeRealization::Unknown,
                    },
                    operation.observation_subject(),
                );
                for requirement in operation.requirements() {
                    let requirement_id = requirement.requirement_id().as_str();
                    let disposition = requirement.disposition();
                    capabilities_and_operations.push_primary(
                        EffectiveProfileEntryDetail::Capability {
                            capability_id: requirement.capability_id().as_str().to_owned(),
                            repository_id: repository_id.to_owned(),
                            operation_id: operation_id.clone(),
                            requirement_id: requirement_id.to_owned(),
                            effect_class: EffectiveProfileEffectClass::from(
                                requirement.effect_class(),
                            ),
                            requirement_disposition: EffectiveProfileRequirementDisposition::from(
                                disposition,
                            ),
                            provider_disposition: EffectiveProfileProviderDisposition::from(
                                requirement.provider(),
                            ),
                            native_realization: EffectiveProfileNativeRealization::Unknown,
                        },
                        requirement.observation_subject(),
                    );
                }
            }
        }

        effective_configuration.extend(inspection.provider_repositories().map(|projection| {
            EffectiveProfileEntryDetail::ProviderRepository {
                repository_id: projection.repository_id().as_str().to_owned(),
                provider_binding_id: projection.provider_binding_id().to_owned(),
                native_repository_coordinate: projection.native_repository_coordinate().to_owned(),
                native_realization: EffectiveProfileNativeRealization::Unknown,
            }
        }));

        for finding in inspection.findings() {
            gaps_and_explanations
                .push_primary(project_finding(finding), finding.observation_subject());
        }

        let mut machine_realization = ProfileDetails::default();
        machine_realization.push_primary(
            EffectiveProfileEntryDetail::MachineRealization {
                machine_id: coordinates.machine_id.clone(),
                realization: EffectiveProfileNativeRealization::Unknown,
            },
            inspection.native_realization().observation_subject(),
        );

        // Bind every explanation to the complete admitted compilation,
        // including its private instance and workspace admission evidence.
        // Canonicalize once here; row construction reuses the fixed digest.
        let coordinate_bytes = pale_core::canon::to_canonical_string(&coordinates)
            .expect("API-owned effective-profile coordinates preserve canonical JSON");
        let coordinate_digest = pale_core::sha256_hex(coordinate_bytes.as_bytes());
        let mut explanations = BTreeMap::new();
        let intended_use_case = explainable_entries(
            &coordinate_digest,
            EffectiveProfileSection::IntendedUseCase,
            intended_use_case,
            &mut explanations,
        );
        let effective_configuration = explainable_entries(
            &coordinate_digest,
            EffectiveProfileSection::EffectiveConfiguration,
            effective_configuration,
            &mut explanations,
        );
        let capabilities_and_operations = explainable_entries(
            &coordinate_digest,
            EffectiveProfileSection::CapabilitiesAndOperations,
            capabilities_and_operations,
            &mut explanations,
        );
        let machine_realization = explainable_entries(
            &coordinate_digest,
            EffectiveProfileSection::MachineRealization,
            machine_realization,
            &mut explanations,
        );
        let gaps_and_explanations = explainable_entries(
            &coordinate_digest,
            EffectiveProfileSection::GapsAndExplanations,
            gaps_and_explanations,
            &mut explanations,
        );

        let mut authoring_entries = BTreeMap::new();
        for (section, entries) in [
            (EffectiveProfileSection::IntendedUseCase, &intended_use_case),
            (
                EffectiveProfileSection::EffectiveConfiguration,
                &effective_configuration,
            ),
            (
                EffectiveProfileSection::CapabilitiesAndOperations,
                &capabilities_and_operations,
            ),
            (
                EffectiveProfileSection::MachineRealization,
                &machine_realization,
            ),
            (
                EffectiveProfileSection::GapsAndExplanations,
                &gaps_and_explanations,
            ),
        ] {
            index_authoring_entries(section, entries, &mut authoring_entries);
        }

        Self {
            coordinates,
            intended_use_case,
            effective_configuration,
            capabilities_and_operations,
            machine_realization,
            gaps_and_explanations,
            explanations,
            authoring_entries,
        }
    }

    pub(super) fn coordinates(&self) -> EffectiveProfileCoordinates {
        self.coordinates.clone()
    }

    pub(super) fn explain(
        &self,
        handle: &EffectiveProfileExplanationHandle,
    ) -> Option<EffectiveProfileExplanation> {
        self.explanations.get(handle).cloned()
    }

    pub(super) fn authoring(
        &self,
        kind: AuthoringIdentityKind,
        identity: &str,
    ) -> EffectiveAuthoringInspection {
        let (entries, next_offset) = self.authoring_page(kind, identity, 0, 0);
        EffectiveAuthoringInspection {
            coordinates: self.coordinates.clone(),
            entries,
            truncated: next_offset.is_some(),
        }
    }

    /// Read one slice of the retained association index, without scanning the
    /// preceding rows. The row ceiling does not bound each row's payload bytes.
    pub(super) fn authoring_page(
        &self,
        kind: AuthoringIdentityKind,
        identity: &str,
        offset: usize,
        limit: u16,
    ) -> (Vec<EffectiveProfileEntry>, Option<usize>) {
        let key = (kind, identity.to_owned());
        let coordinates = self
            .authoring_entries
            .get(&key)
            .map_or(&[][..], Vec::as_slice);
        let start = offset.min(coordinates.len());
        let requested = if limit == 0 {
            DEFAULT_PAGE_SIZE
        } else {
            limit.min(MAX_PAGE_SIZE)
        };
        let end = start
            .saturating_add(usize::from(requested))
            .min(coordinates.len());
        let entries = coordinates[start..end]
            .iter()
            .map(|coordinate| {
                self.entry(*coordinate)
                    .expect("authoring associations point into their immutable profile sections")
                    .clone()
            })
            .collect();
        (entries, (end < coordinates.len()).then_some(end))
    }

    fn entry(&self, coordinate: ProfileEntryCoordinate) -> Option<&EffectiveProfileEntry> {
        let entries = match coordinate.section {
            EffectiveProfileSection::IntendedUseCase => &self.intended_use_case,
            EffectiveProfileSection::EffectiveConfiguration => &self.effective_configuration,
            EffectiveProfileSection::CapabilitiesAndOperations => &self.capabilities_and_operations,
            EffectiveProfileSection::MachineRealization => &self.machine_realization,
            EffectiveProfileSection::GapsAndExplanations => &self.gaps_and_explanations,
        };
        entries.get(coordinate.index).map(|row| &row.entry)
    }

    pub(super) fn page(&self, request: EffectiveProfilePageRequest) -> EffectiveProfilePage {
        let EffectiveProfilePageRequest {
            section,
            offset,
            limit,
        } = request;
        let entries = match section {
            EffectiveProfileSection::IntendedUseCase => &self.intended_use_case,
            EffectiveProfileSection::EffectiveConfiguration => &self.effective_configuration,
            EffectiveProfileSection::CapabilitiesAndOperations => &self.capabilities_and_operations,
            EffectiveProfileSection::MachineRealization => &self.machine_realization,
            EffectiveProfileSection::GapsAndExplanations => &self.gaps_and_explanations,
        };
        let start = usize::try_from(offset)
            .unwrap_or(usize::MAX)
            .min(entries.len());
        let requested = if limit == 0 {
            DEFAULT_PAGE_SIZE
        } else {
            limit.min(MAX_PAGE_SIZE)
        };
        let end = start
            .saturating_add(usize::from(requested))
            .min(entries.len());
        let mut page = EffectiveProfilePage {
            section,
            offset,
            entries: entries[start..end]
                .iter()
                .map(|row| row.entry.clone())
                .collect(),
            next_offset: (end < entries.len()).then(|| {
                u32::try_from(end).expect("effective-profile entry count is contract-bounded")
            }),
            canonical_bytes: Vec::new(),
        };
        page.canonical_bytes = pale_core::canon::to_canonical_string(&page)
            .expect("API-owned effective-profile pages preserve the canonical JSON grammar")
            .into_bytes();
        page
    }
}

fn index_authoring_entries(
    section: EffectiveProfileSection,
    entries: &[RetainedProfileEntry],
    index: &mut BTreeMap<AuthoringSubject, Vec<ProfileEntryCoordinate>>,
) {
    for (entry_index, entry) in entries.iter().enumerate() {
        let coordinate = ProfileEntryCoordinate {
            section,
            index: entry_index,
        };
        for subject in authoring_subjects(&entry.entry.detail) {
            index.entry(subject).or_default().push(coordinate);
        }
    }
}

fn authoring_subjects(detail: &EffectiveProfileEntryDetail) -> BTreeSet<AuthoringSubject> {
    let mut subjects = BTreeSet::new();
    let mut insert = |kind, value: &str| {
        subjects.insert((kind, value.to_owned()));
    };
    match detail {
        EffectiveProfileEntryDetail::DeclaredSelection { selection } => match selection {
            EffectiveProfileDeclaredSelection::Resource {
                resource_kind,
                selector,
            } => match selector {
                EffectiveProfileDeclaredSelector::Exact { resource_id } => {
                    insert(resource_authoring_kind(*resource_kind), resource_id);
                }
                EffectiveProfileDeclaredSelector::AllFromPackage { package_id } => {
                    insert(AuthoringIdentityKind::Package, package_id);
                }
            },
            EffectiveProfileDeclaredSelection::ModeDefault { resource_id, .. } => {
                insert(AuthoringIdentityKind::PackageRunMode, resource_id);
            }
        },
        EffectiveProfileEntryDetail::Responsibility {
            contract_id,
            execution_path_id,
            ..
        } => {
            insert(
                AuthoringIdentityKind::CompositionResponsibilityContract,
                contract_id,
            );
            insert(AuthoringIdentityKind::ExecutionPath, execution_path_id);
        }
        EffectiveProfileEntryDetail::Selection { selection } => match selection {
            EffectiveProfileSelection::Package { package_id, .. } => {
                insert(AuthoringIdentityKind::Package, package_id);
            }
            EffectiveProfileSelection::NormativePack { .. }
            | EffectiveProfileSelection::DisclosureCeiling { .. } => {}
            EffectiveProfileSelection::SemanticResource {
                repository_id,
                resource_kind,
                resource_id,
            } => {
                insert(AuthoringIdentityKind::Repository, repository_id);
                insert(resource_authoring_kind(*resource_kind), resource_id);
            }
            EffectiveProfileSelection::RunPolicy { run_policy_id } => {
                insert(AuthoringIdentityKind::RunPolicy, run_policy_id);
            }
            EffectiveProfileSelection::Session {
                execution_workspace_id,
                ..
            } => {
                insert(
                    AuthoringIdentityKind::ExecutionWorkspace,
                    execution_workspace_id,
                );
            }
            EffectiveProfileSelection::Lifecycle { lifecycle_id, .. } => {
                insert(AuthoringIdentityKind::LifecycleBehavior, lifecycle_id);
            }
            EffectiveProfileSelection::AuthorityRoot { authority_root_id } => {
                insert(AuthoringIdentityKind::AuthorityRoot, authority_root_id);
            }
        },
        EffectiveProfileEntryDetail::CapabilityNarrowing {
            permitted_capabilities,
            forbidden_capabilities,
            ..
        } => {
            for capability_id in permitted_capabilities
                .iter()
                .chain(forbidden_capabilities.iter())
            {
                insert(AuthoringIdentityKind::Capability, capability_id);
            }
        }
        EffectiveProfileEntryDetail::GoverningCeiling { policies, .. } => {
            for policy_id in policies {
                insert(AuthoringIdentityKind::PackageCapabilityPolicy, policy_id);
            }
        }
        EffectiveProfileEntryDetail::Repository { repository_id, .. } => {
            insert(AuthoringIdentityKind::Repository, repository_id);
        }
        EffectiveProfileEntryDetail::ProviderRepository {
            repository_id,
            provider_binding_id,
            ..
        } => {
            insert(AuthoringIdentityKind::Repository, repository_id);
            insert(
                AuthoringIdentityKind::CompositionProviderBinding,
                provider_binding_id,
            );
        }
        EffectiveProfileEntryDetail::ExecutionPath {
            selection_id,
            experience_path_id,
            responsibility_contract_ids,
            work_delegation_edge_ids,
            provider_binding_ids,
            capability_ceiling,
            ..
        } => {
            insert(AuthoringIdentityKind::ExecutionPath, selection_id);
            insert(AuthoringIdentityKind::ExperiencePath, experience_path_id);
            for contract_id in responsibility_contract_ids {
                insert(
                    AuthoringIdentityKind::CompositionResponsibilityContract,
                    contract_id,
                );
            }
            for edge_id in work_delegation_edge_ids {
                insert(AuthoringIdentityKind::CompositionEdge, edge_id);
            }
            for provider_binding_id in provider_binding_ids {
                insert(
                    AuthoringIdentityKind::CompositionProviderBinding,
                    provider_binding_id,
                );
            }
            for capability_id in capability_ceiling {
                insert(AuthoringIdentityKind::Capability, capability_id);
            }
        }
        EffectiveProfileEntryDetail::Operation {
            repository_id,
            operation_id,
            ..
        } => {
            insert(AuthoringIdentityKind::Repository, repository_id);
            insert(AuthoringIdentityKind::PackageOperation, operation_id);
        }
        EffectiveProfileEntryDetail::Capability {
            capability_id,
            repository_id,
            operation_id,
            ..
        } => {
            insert(AuthoringIdentityKind::Capability, capability_id);
            insert(AuthoringIdentityKind::Repository, repository_id);
            insert(AuthoringIdentityKind::PackageOperation, operation_id);
        }
        EffectiveProfileEntryDetail::MachineRealization { machine_id, .. } => {
            insert(AuthoringIdentityKind::Machine, machine_id);
        }
        EffectiveProfileEntryDetail::Finding { finding } => match finding {
            EffectiveProfileFinding::NativeRealizationUnknown { machine_id } => {
                insert(AuthoringIdentityKind::Machine, machine_id);
            }
            EffectiveProfileFinding::OperationUnbound {
                repository_id,
                operation_id,
            } => {
                insert(AuthoringIdentityKind::Repository, repository_id);
                insert(AuthoringIdentityKind::PackageOperation, operation_id);
            }
            EffectiveProfileFinding::Requirement {
                repository_id,
                operation_id,
                capability_id,
                ..
            } => {
                insert(AuthoringIdentityKind::Repository, repository_id);
                insert(AuthoringIdentityKind::PackageOperation, operation_id);
                insert(AuthoringIdentityKind::Capability, capability_id);
            }
        },
    }
    subjects
}

const fn resource_authoring_kind(kind: EffectiveProfileResourceKind) -> AuthoringIdentityKind {
    match kind {
        EffectiveProfileResourceKind::Operation => AuthoringIdentityKind::PackageOperation,
        EffectiveProfileResourceKind::Role => AuthoringIdentityKind::PackageRole,
        EffectiveProfileResourceKind::Doctrine | EffectiveProfileResourceKind::StartupPolicy => {
            AuthoringIdentityKind::PackageDoctrine
        }
        EffectiveProfileResourceKind::RunMode => AuthoringIdentityKind::PackageRunMode,
        EffectiveProfileResourceKind::Stack => AuthoringIdentityKind::PackageStack,
        EffectiveProfileResourceKind::PolicyFamily => AuthoringIdentityKind::PackagePolicyFamily,
        EffectiveProfileResourceKind::Recipe => AuthoringIdentityKind::PackageToolRecipe,
    }
}

fn declared_selection(
    resource_kind: EffectiveProfileResourceKind,
    selector: &PackageResourceSelectorV1,
) -> EffectiveProfileEntryDetail {
    let selector = match selector {
        PackageResourceSelectorV1::Exact { resource } => EffectiveProfileDeclaredSelector::Exact {
            resource_id: resource.to_string(),
        },
        PackageResourceSelectorV1::AllFromPackage { package_id } => {
            EffectiveProfileDeclaredSelector::AllFromPackage {
                package_id: package_id.as_str().to_owned(),
            }
        }
    };
    EffectiveProfileEntryDetail::DeclaredSelection {
        selection: EffectiveProfileDeclaredSelection::Resource {
            resource_kind,
            selector,
        },
    }
}

fn explainable_entries(
    coordinate_digest: &str,
    section: EffectiveProfileSection,
    details: ProfileDetails,
    explanations: &mut BTreeMap<EffectiveProfileExplanationHandle, EffectiveProfileExplanation>,
) -> Vec<RetainedProfileEntry> {
    details
        .0
        .into_iter()
        .enumerate()
        .map(|(index, (detail, primary_subject))| {
            let detail_bytes = pale_core::canon::to_canonical_string(&detail)
                .expect("API-owned effective-profile entries preserve canonical JSON");
            let preimage = format!(
                "pale.effective-profile.explanation.v1\0{}\0{}\0{}\0",
                coordinate_digest,
                effective_profile_section_name(section),
                index,
            ) + &detail_bytes;
            let handle =
                EffectiveProfileExplanationHandle(pale_core::sha256_hex(preimage.as_bytes()));
            let explanation = EffectiveProfileExplanation {
                handle: handle.clone(),
                evidence: explanation_evidence(&detail),
            };
            assert!(
                explanations.insert(handle.clone(), explanation).is_none(),
                "domain-separated effective-profile explanation handles are unique",
            );
            RetainedProfileEntry {
                entry: EffectiveProfileEntry {
                    explanation_handle: handle,
                    detail,
                },
                primary_subject,
            }
        })
        .collect()
}

fn explanation_evidence(
    detail: &EffectiveProfileEntryDetail,
) -> EffectiveProfileExplanationEvidence {
    match detail {
        EffectiveProfileEntryDetail::DeclaredSelection { selection } => {
            EffectiveProfileExplanationEvidence::DeclaredSelection {
                selection: selection.clone(),
            }
        }
        EffectiveProfileEntryDetail::Responsibility {
            contract_id,
            execution_path_id,
            reason_steps,
            ..
        } => EffectiveProfileExplanationEvidence::Responsibility {
            contract_id: contract_id.clone(),
            execution_path_id: execution_path_id.clone(),
            reason_steps: reason_steps.clone(),
        },
        EffectiveProfileEntryDetail::Selection { selection } => {
            EffectiveProfileExplanationEvidence::EffectiveSelection {
                selection: selection.clone(),
            }
        }
        EffectiveProfileEntryDetail::ExecutionPath {
            selection_id,
            reason_steps,
            ..
        } => EffectiveProfileExplanationEvidence::ExecutionPath {
            selection_id: selection_id.clone(),
            reason_steps: reason_steps.clone(),
        },
        EffectiveProfileEntryDetail::Operation {
            exposure,
            native_realization,
            ..
        } => EffectiveProfileExplanationEvidence::OperationClassification {
            exposure: *exposure,
            native_realization: *native_realization,
        },
        EffectiveProfileEntryDetail::Capability {
            requirement_disposition,
            provider_disposition,
            native_realization,
            ..
        } => EffectiveProfileExplanationEvidence::CapabilityClassification {
            requirement_disposition: *requirement_disposition,
            provider_disposition: *provider_disposition,
            native_realization: *native_realization,
        },
        EffectiveProfileEntryDetail::Finding { finding } => {
            EffectiveProfileExplanationEvidence::FindingClassification {
                finding: finding.clone(),
            }
        }
        EffectiveProfileEntryDetail::MachineRealization { realization, .. } => {
            EffectiveProfileExplanationEvidence::MachineRealization {
                realization: *realization,
            }
        }
        EffectiveProfileEntryDetail::CapabilityNarrowing { .. }
        | EffectiveProfileEntryDetail::GoverningCeiling { .. }
        | EffectiveProfileEntryDetail::Repository { .. }
        | EffectiveProfileEntryDetail::ProviderRepository { .. } => {
            EffectiveProfileExplanationEvidence::Configuration
        }
    }
}

const fn effective_profile_section_name(section: EffectiveProfileSection) -> &'static str {
    match section {
        EffectiveProfileSection::IntendedUseCase => "intended-use-case",
        EffectiveProfileSection::EffectiveConfiguration => "effective-configuration",
        EffectiveProfileSection::CapabilitiesAndOperations => "capabilities-and-operations",
        EffectiveProfileSection::MachineRealization => "machine-realization",
        EffectiveProfileSection::GapsAndExplanations => "gaps-and-explanations",
    }
}

fn project_finding(finding: ApplicationFinding<'_>) -> EffectiveProfileEntryDetail {
    let finding = match finding {
        ApplicationFinding::NativeRealizationUnknown { machine_id } => {
            EffectiveProfileFinding::NativeRealizationUnknown {
                machine_id: machine_id.to_owned(),
            }
        }
        ApplicationFinding::OperationUnbound {
            repository_id,
            operation_id,
        } => EffectiveProfileFinding::OperationUnbound {
            repository_id: repository_id.as_str().to_owned(),
            operation_id: operation_id.to_string(),
        },
        ApplicationFinding::RequirementDisposition {
            repository_id,
            operation_id,
            requirement_id,
            capability_id,
            disposition,
        } => EffectiveProfileFinding::Requirement {
            repository_id: repository_id.as_str().to_owned(),
            operation_id: operation_id.to_string(),
            requirement_id: requirement_id.as_str().to_owned(),
            capability_id: capability_id.as_str().to_owned(),
            disposition: EffectiveProfileRequirementDisposition::from(disposition),
        },
    };
    EffectiveProfileEntryDetail::Finding { finding }
}

impl From<ModeAxis> for EffectiveProfileModeAxis {
    fn from(value: ModeAxis) -> Self {
        match value {
            ModeAxis::Intensity => Self::Intensity,
            ModeAxis::TaskStance => Self::TaskStance,
        }
    }
}

impl From<ExecutionWorkspaceRepositoryAccessV1> for EffectiveProfileRepositoryAccess {
    fn from(value: ExecutionWorkspaceRepositoryAccessV1) -> Self {
        match value {
            ExecutionWorkspaceRepositoryAccessV1::Read => Self::Read,
            ExecutionWorkspaceRepositoryAccessV1::Mutate => Self::Mutate,
        }
    }
}

impl From<&ProviderBindingRefV1> for EffectiveProfileProviderBindingRef {
    fn from(value: &ProviderBindingRefV1) -> Self {
        Self {
            provider_binding_id: value.provider_binding_id.clone(),
            provider_binding_identity: value
                .provider_binding_identity
                .provider_binding_sha256
                .to_string(),
        }
    }
}

impl From<EffectClass> for EffectiveProfileEffectClass {
    fn from(value: EffectClass) -> Self {
        match value {
            EffectClass::Compute => Self::Compute,
            EffectClass::Read => Self::Read,
            EffectClass::Execute => Self::Execute,
            EffectClass::Write => Self::Write,
            EffectClass::Network => Self::Network,
            EffectClass::Publish => Self::Publish,
        }
    }
}

impl From<&ParticipantRefV1> for EffectiveProfileParticipant {
    fn from(value: &ParticipantRefV1) -> Self {
        match value {
            ParticipantRefV1::Interface { interface_id } => Self::Interface {
                interface_id: interface_id.clone(),
            },
            ParticipantRefV1::Application { application_id } => Self::Application {
                application_id: application_id.clone(),
            },
            ParticipantRefV1::Engine { engine_id } => Self::Engine {
                engine_id: engine_id.clone(),
            },
            ParticipantRefV1::Capability { capability_id } => Self::Capability {
                capability_id: capability_id.clone(),
            },
            ParticipantRefV1::TerminalLeaf { terminal_leaf_id } => Self::TerminalLeaf {
                terminal_leaf_id: terminal_leaf_id.clone(),
            },
            ParticipantRefV1::Plugin { plugin_id } => Self::Plugin {
                plugin_id: plugin_id.clone(),
            },
            ParticipantRefV1::Skill { skill_id } => Self::Skill {
                skill_id: skill_id.clone(),
            },
        }
    }
}

impl From<&ResponsibilityV1> for EffectiveProfileResponsibility {
    fn from(value: &ResponsibilityV1) -> Self {
        match value {
            ResponsibilityV1::Communication => Self::Communication,
            ResponsibilityV1::OrganizationalWork => Self::OrganizationalWork,
            ResponsibilityV1::CrmBusinessState => Self::CrmBusinessState,
            ResponsibilityV1::EngineeringExecution => Self::EngineeringExecution,
            ResponsibilityV1::ContextMemory => Self::ContextMemory,
            ResponsibilityV1::TerminalHarnessExecution => Self::TerminalHarnessExecution,
            ResponsibilityV1::NativeEventPublication => Self::NativeEventPublication,
            ResponsibilityV1::NativeEffectReadBack => Self::NativeEffectReadBack,
            ResponsibilityV1::Extension {
                namespace_id,
                contract_id,
                contract_schema_identity,
            } => Self::Extension {
                namespace_id: namespace_id.clone(),
                contract_id: contract_id.clone(),
                schema_id: contract_schema_identity.schema_id.as_str().to_owned(),
                schema_sha256: contract_schema_identity.schema_sha256.as_str().to_owned(),
            },
        }
    }
}

impl From<ApplicationResourceKind> for EffectiveProfileResourceKind {
    fn from(value: ApplicationResourceKind) -> Self {
        match value {
            ApplicationResourceKind::Operation => Self::Operation,
            ApplicationResourceKind::Role => Self::Role,
            ApplicationResourceKind::Doctrine => Self::Doctrine,
            ApplicationResourceKind::StartupPolicy => Self::StartupPolicy,
            ApplicationResourceKind::RunMode => Self::RunMode,
            ApplicationResourceKind::Stack => Self::Stack,
            ApplicationResourceKind::PolicyFamily => Self::PolicyFamily,
            ApplicationResourceKind::Recipe => Self::Recipe,
        }
    }
}

impl From<ApplicationOperationExposure> for EffectiveProfileOperationExposure {
    fn from(value: ApplicationOperationExposure) -> Self {
        match value {
            ApplicationOperationExposure::Unbound => Self::Unbound,
            ApplicationOperationExposure::SemanticOnly => Self::SemanticOnly,
            ApplicationOperationExposure::CapabilityBearing => Self::CapabilityBearing,
        }
    }
}

impl From<ApplicationRequirementDisposition> for EffectiveProfileRequirementDisposition {
    fn from(value: ApplicationRequirementDisposition) -> Self {
        match value {
            ApplicationRequirementDisposition::Satisfied => Self::Satisfied,
            ApplicationRequirementDisposition::Narrowed(reason) => Self::Narrowed {
                reason: EffectiveProfileNarrowingReason::from(reason),
            },
            ApplicationRequirementDisposition::ProviderBindingAbsent => Self::ProviderBindingAbsent,
            ApplicationRequirementDisposition::PolicyBlocked(reason) => Self::Blocked {
                reason: EffectiveProfilePolicyBlock::from(reason),
            },
        }
    }
}

impl From<ApplicationNarrowingReason> for EffectiveProfileNarrowingReason {
    fn from(value: ApplicationNarrowingReason) -> Self {
        match value {
            ApplicationNarrowingReason::CapabilityNotPermitted => Self::CapabilityNotPermitted,
            ApplicationNarrowingReason::EffectNotPermitted => Self::EffectNotPermitted,
            ApplicationNarrowingReason::CapabilityAndEffectNotPermitted => {
                Self::CapabilityAndEffectNotPermitted
            }
        }
    }
}

impl From<ApplicationPolicyBlock> for EffectiveProfilePolicyBlock {
    fn from(value: ApplicationPolicyBlock) -> Self {
        match value {
            ApplicationPolicyBlock::CapabilityForbidden => Self::CapabilityForbidden,
            ApplicationPolicyBlock::EffectForbidden => Self::EffectForbidden,
            ApplicationPolicyBlock::CapabilityAndEffectForbidden => {
                Self::CapabilityAndEffectForbidden
            }
            ApplicationPolicyBlock::ProviderEffectMismatch => Self::ProviderEffectMismatch,
            ApplicationPolicyBlock::RecipeContractMismatch => Self::RecipeContractMismatch,
        }
    }
}

impl From<&CompositionReasonStepV1> for EffectiveProfilePathReason {
    fn from(value: &CompositionReasonStepV1) -> Self {
        match value {
            CompositionReasonStepV1::OriginReceipt { contract_id } => Self::OriginReceipt {
                contract_id: contract_id.clone(),
            },
            CompositionReasonStepV1::EntryContract { contract_id } => Self::EntryContract {
                contract_id: contract_id.clone(),
            },
            CompositionReasonStepV1::ResponsibilityContract { contract_id } => {
                Self::ResponsibilityContract {
                    contract_id: contract_id.clone(),
                }
            }
            CompositionReasonStepV1::ProviderBinding { binding_id } => Self::ProviderBinding {
                binding_id: binding_id.clone(),
            },
            CompositionReasonStepV1::DelegationEdge { edge_id } => Self::DelegationEdge {
                edge_id: edge_id.clone(),
            },
            CompositionReasonStepV1::TerminationContract { contract_id } => {
                Self::TerminationContract {
                    contract_id: contract_id.clone(),
                }
            }
            CompositionReasonStepV1::SettlementContract { contract_id } => {
                Self::SettlementContract {
                    contract_id: contract_id.clone(),
                }
            }
            CompositionReasonStepV1::ContinuityContract { contract_id } => {
                Self::ContinuityContract {
                    contract_id: contract_id.clone(),
                }
            }
        }
    }
}

impl From<ApplicationProviderDisposition> for EffectiveProfileProviderDisposition {
    fn from(value: ApplicationProviderDisposition) -> Self {
        match value {
            ApplicationProviderDisposition::Bound(provider) => Self::Bound {
                provider: EffectiveProfileProviderKind::from(provider),
            },
            ApplicationProviderDisposition::Unbound(ProviderUnavailability::MissingBinding) => {
                Self::MissingBinding
            }
            ApplicationProviderDisposition::Unbound(
                ProviderUnavailability::ExcludedByNarrowing,
            ) => Self::ExcludedByNarrowing,
            ApplicationProviderDisposition::Unbound(ProviderUnavailability::EffectMismatch) => {
                Self::EffectMismatch
            }
            ApplicationProviderDisposition::Unbound(
                ProviderUnavailability::RecipeContractMismatch,
            ) => Self::RecipeContractMismatch,
        }
    }
}

impl From<CapabilityProvider> for EffectiveProfileProviderKind {
    fn from(value: CapabilityProvider) -> Self {
        match value {
            CapabilityProvider::HostNative => Self::HostNative,
            CapabilityProvider::NativeRust => Self::NativeRust,
            CapabilityProvider::Connector => Self::Connector,
        }
    }
}

fn bounded_count(value: usize) -> u32 {
    u32::try_from(value).expect("effective-profile collection cardinality is contract-bounded")
}

#[cfg(test)]
#[path = "../../tests/internal/effective_profile_projection.rs"]
mod tests;
