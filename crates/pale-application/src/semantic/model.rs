//! Source-free repository projections over the sealed Package graph.

use std::collections::BTreeMap;

use pale_core::records::{
    ArtifactDigest, CapabilityBinding, CapabilityIdV1, CapabilityRequirementIdV1,
    DoctrinePlacement, EffectClass, FailurePosture, ModeAxis, ModeDefaultSource, ModeScope,
    OfficialReference, OperationSelection, PackageResourceRefV1,
    ResolvedCapabilityRequirementPolicyV1, ToolRecipe,
};

use crate::packages::{PackageResourceUse, ResourceTextReference, TextRecipe};

/// One exact Package body in dependency-first composition order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyLineage {
    /// Owner-qualified Package resource that supplied the bytes.
    pub owner: PackageResourceRefV1,
    /// Digest of the exact admitted Markdown bytes.
    pub sha256: ArtifactDigest,
}

/// Package-owned recipe materialized once with its owner-qualified identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRecipe {
    /// Exact Package recipe resource.
    pub resource: PackageResourceRefV1,
    /// Provider-neutral procedure admitted from that resource.
    pub procedure: ToolRecipe,
}

/// Whether private configuration selected a provider for one requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedProviderSelection {
    /// No usable provider exists; only the affected operation closes or degrades.
    Unbound(ProviderUnavailability),
    /// Exact private provider selection.
    Bound(CapabilityBinding),
}

/// Why one Package capability requirement has no usable private provider.
///
/// This is compile-time availability evidence, not a runtime observation. The
/// owning requirement's policy decides whether the operation blocks, degrades,
/// or continues without the optional mechanic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderUnavailability {
    /// Private configuration contains no provider for the capability.
    MissingBinding,
    /// Private narrowing removed the capability or effect from this occurrence.
    ExcludedByNarrowing,
    /// The selected provider's effect differs from the Package requirement.
    EffectMismatch,
    /// The selected provider does not implement the Package recipe contract.
    RecipeContractMismatch,
}

impl ResolvedProviderSelection {
    /// Borrow the private binding without manufacturing provider execution availability.
    #[must_use]
    pub fn binding(&self) -> Option<&CapabilityBinding> {
        match self {
            Self::Unbound(_) => None,
            Self::Bound(binding) => Some(binding),
        }
    }
}

/// One Package requirement joined to optional private provider mechanics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCapability {
    /// Requirement identity local to the owning operation.
    pub requirement_id: CapabilityRequirementIdV1,
    /// Model-facing reason for the requirement.
    pub description: String,
    /// Portable capability identity.
    pub capability_id: CapabilityIdV1,
    /// Owner-qualified primary recipe, when declared.
    pub recipe: Option<PackageResourceRefV1>,
    /// Project-selected provider state.
    pub provider: ResolvedProviderSelection,
    /// Consequence class enforced by admission.
    pub effect_class: EffectClass,
    /// Source-free required/degrade/optional policy.
    pub policy: ResolvedCapabilityRequirementPolicyV1,
    /// Recovery posture after an admitted invocation fails.
    pub failure_posture: FailurePosture,
}

impl ResolvedCapability {
    /// Borrow the selected private provider, when present.
    #[must_use]
    pub(crate) fn binding(&self) -> Option<&CapabilityBinding> {
        self.provider.binding()
    }
}

/// Source-free identity and descriptive fields of one selected operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOperationRecord {
    /// Owner-qualified operation identity.
    pub id: PackageResourceRefV1,
    /// Compact operation purpose.
    pub description: String,
    /// Owner-qualified specialist roles available to the operation.
    pub roles: Vec<PackageResourceRefV1>,
    /// Declared operation outputs.
    pub outputs: Vec<String>,
}

/// Closed private state of one selected semantic operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedOperationMechanics {
    /// No private binding exists; no discovery or execution surface is emitted.
    Unbound,
    /// A present binding over a Package operation with zero requirements.
    SemanticOnly {
        /// Private exposure and routing selection for the bound operation.
        selection: OperationSelection,
    },
    /// A present binding over one or more Package requirements.
    CapabilityBearing {
        /// Private exposure and routing selection for the bound operation.
        selection: OperationSelection,
        /// Package requirements joined independently to private provider state.
        capabilities: Vec<ResolvedCapability>,
    },
}

impl ResolvedOperationMechanics {
    /// Selection policy carried only by bound states.
    #[must_use]
    pub fn selection(&self) -> Option<&OperationSelection> {
        match self {
            Self::Unbound => None,
            Self::SemanticOnly { selection } | Self::CapabilityBearing { selection, .. } => {
                Some(selection)
            }
        }
    }

    /// Requirements carried only by the capability-bearing state.
    #[must_use]
    pub fn capabilities(&self) -> &[ResolvedCapability] {
        match self {
            Self::CapabilityBearing { capabilities, .. } => capabilities,
            Self::Unbound | Self::SemanticOnly { .. } => &[],
        }
    }

    /// Whether private configuration bound this operation for exposure.
    #[must_use]
    pub fn is_bound(&self) -> bool {
        !matches!(self, Self::Unbound)
    }
}

/// Operation procedure and dependency closure resolved for one repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOperation {
    /// Source-free identity, purpose, roles, and outputs.
    pub record: ResolvedOperationRecord,
    /// Dependency-composed operation and archetype procedure.
    pub(crate) body: TextRecipe,
    /// Dependency-composed doctrine required by the operation.
    pub(crate) required_doctrine: TextRecipe,
    /// Exact Package bodies contributing to the composed procedure and doctrine.
    pub body_sources: Vec<BodyLineage>,
    /// Owner-qualified doctrine closure in dependency-first order.
    pub doctrine_ids: Vec<PackageResourceRefV1>,
    /// Closed three-state private binding and capability join.
    pub mechanics: ResolvedOperationMechanics,
}

impl ResolvedOperation {
    /// Borrow private selection only when this operation is bound.
    ///
    /// `None` means [`ResolvedOperationMechanics::Unbound`], not semantic-only.
    #[must_use]
    pub fn selection(&self) -> Option<&OperationSelection> {
        self.mechanics.selection()
    }

    /// Borrow requirements only from the capability-bearing state.
    ///
    /// Both unbound and semantic-only operations return an empty slice; inspect
    /// [`Self::mechanics`] when that distinction matters.
    #[must_use]
    pub fn capabilities(&self) -> &[ResolvedCapability] {
        self.mechanics.capabilities()
    }
}

/// Source-free identity and relationship fields of one selected role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoleRecord {
    /// Owner-qualified role identity.
    pub id: PackageResourceRefV1,
    /// Compact role purpose.
    pub description: String,
    /// Owner-qualified role-lens dependencies.
    pub inherits_lenses: Vec<PackageResourceRefV1>,
    /// Stacks for which this role contributes stack-specific doctrine.
    pub applies_to: Vec<PackageResourceRefV1>,
}

/// Specialist role with dependency-composed body and doctrine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRole {
    /// Source-free identity and relationship record.
    pub record: ResolvedRoleRecord,
    /// Dependency-composed role, archetype, and required-doctrine body.
    pub(crate) body: TextRecipe,
    /// Exact Package bodies contributing to the composed role guidance.
    pub body_sources: Vec<BodyLineage>,
    /// Owner-qualified doctrine closure in dependency-first order.
    pub doctrine_ids: Vec<PackageResourceRefV1>,
    /// Stack-page guides reached through reading links instead of body copies.
    pub(crate) required_reading: Vec<PackageResourceRefV1>,
    /// Official references from included doctrine, regardless of its placement.
    pub(crate) official_sources: Vec<OfficialReference>,
}

/// Source-free identity and relationship fields of one doctrine resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDoctrineRecord {
    /// Owner-qualified doctrine identity.
    pub id: PackageResourceRefV1,
    /// Compact doctrine purpose.
    pub description: String,
    /// Consumer placement for the composed doctrine.
    pub placement: DoctrinePlacement,
    /// Doctrine resources required by this record.
    pub requires: Vec<PackageResourceRefV1>,
    /// Related doctrine resources that remain non-authoritative references.
    pub see_also: Vec<PackageResourceRefV1>,
    /// Bounded official references admitted with the Package record.
    pub official_sources: Vec<OfficialReference>,
}

/// Dependency-composed doctrine body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDoctrine {
    /// Admitted document title or owner-qualified identity fallback.
    pub title: String,
    /// Readable declared references, including resources outside the selection.
    pub(crate) related_reading: Vec<crate::packages::ResourceTextReference>,
    /// Source-free doctrine identity and relationships.
    pub record: ResolvedDoctrineRecord,
    /// Dependency-first composed Markdown body.
    pub(crate) body: TextRecipe,
    /// Exact Package bodies contributing to the composed doctrine guidance.
    pub body_sources: Vec<BodyLineage>,
}

/// Source-free selected run-mode record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRunModeRecord {
    /// Owner-qualified run-mode identity.
    pub id: PackageResourceRefV1,
    /// Compact mode purpose.
    pub description: String,
    /// Orthogonal session dimension controlled by this mode.
    pub axis: ModeAxis,
    /// Slash-style aliases that select this mode.
    pub aliases: Vec<String>,
    /// Exact whole-message triggers that select this mode.
    pub exact_triggers: Vec<String>,
    /// Whether the mode may persist as a runtime default.
    pub scope: ModeScope,
}

/// Selected run mode and its dependency-composed optional body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRunMode {
    /// Source-free identity, trigger, axis, and scope record.
    pub record: ResolvedRunModeRecord,
    /// Dependency-composed optional mode guidance.
    pub(crate) body: Option<TextRecipe>,
    /// The same complete composition as [`Self::body`], with admitted headings
    /// shifted once for nesting below a runtime-context heading.
    pub(crate) runtime_body: Option<TextRecipe>,
    /// Exact Package bodies contributing to both body projections.
    pub body_sources: Vec<BodyLineage>,
}

/// Technology/workflow stack selected for one repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStack {
    /// Owner-qualified stack identity.
    pub id: PackageResourceRefV1,
    /// Compact technology or workflow description.
    pub description: String,
    /// More general selected stacks needed by this stack.
    pub extends: Vec<PackageResourceRefV1>,
    /// Doctrine contributed whenever the stack is selected.
    pub doctrine: Vec<PackageResourceRefV1>,
    /// Additional doctrine contributed to applicable specialist roles.
    pub role_doctrine: Vec<PackageResourceRefV1>,
    /// Bounded official references admitted with the stack.
    pub official_sources: Vec<OfficialReference>,
    /// Header-authored stack guidance, when the selected stack owns a page.
    pub(crate) body: Option<TextRecipe>,
}

/// Reading selected for one use of a page, without copying prerequisite bodies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedPageReading {
    /// This use owns a header page, rather than merely including one.
    pub(crate) authored_page: bool,
    pub(crate) required: Vec<PackageResourceRefV1>,
    pub(crate) related: Vec<ResourceTextReference>,
    pub(crate) official_sources: Vec<OfficialReference>,
}

/// One selected policy-family body carried across the source-free boundary.
///
/// The body is content-bearing so renderers never reopen Package source. Its
/// owner-qualified identity prevents equal local IDs in distinct Packages from
/// collapsing into one policy namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPolicyFamily {
    /// Exact Package resource that owns the policy.
    pub id: PackageResourceRefV1,
    /// Human-facing policy title.
    pub title: String,
    /// Compact statement of the policy's purpose.
    pub purpose: String,
    /// Exact admitted Markdown bytes.
    pub(crate) body: TextRecipe,
    /// Digest-bound custody of the composed policy body.
    pub body_sources: Vec<BodyLineage>,
}

/// Closed semantic graph one repository actually receives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositorySemanticPlan {
    /// Header-authored reading kept distinct for each selected use.
    pub(crate) page_reading: BTreeMap<PackageResourceUse, ResolvedPageReading>,
    /// Governing algebra activated by global and repository-local scopes only.
    pub governing_order: pale_core::records::AdmittedGoverningOrderV1,
    /// Selected operations after Package composition and private binding join.
    pub operations: Vec<ResolvedOperation>,
    /// Selected specialist roles after role-lens and stack composition.
    pub roles: Vec<ResolvedRole>,
    /// Selected standalone judgment doctrine.
    pub doctrine: Vec<ResolvedDoctrine>,
    /// Selected doctrine rendered into startup policy surfaces.
    pub startup_policies: Vec<ResolvedDoctrine>,
    /// Selected run modes with content-bearing consumer projections.
    pub run_modes: Vec<ResolvedRunMode>,
    /// Effective owner-qualified default for each selected mode axis.
    pub mode_defaults: BTreeMap<ModeAxis, PackageResourceRefV1>,
    /// Declarative layer that supplied each entry in [`Self::mode_defaults`].
    pub mode_default_sources: BTreeMap<ModeAxis, ModeDefaultSource>,
    /// Selected stack records used by doctrine and role projection.
    pub stacks: Vec<ResolvedStack>,
    /// Selected, content-bearing policy families.
    pub policy_families: Vec<ResolvedPolicyFamily>,
    /// Selected Package recipes, each materialized exactly once.
    pub recipes: Vec<ResolvedRecipe>,
    /// Capabilities whose admitted recipes require repository verification.
    pub repository_verifier_capabilities: Vec<CapabilityIdV1>,
}

/// Render an owner-qualified resource as an unambiguous diagnostic label.
pub(crate) fn resource_label(resource: &PackageResourceRefV1) -> String {
    format!(
        "{}:{}",
        resource.package_id.as_str(),
        resource.resource_id.as_str()
    )
}
