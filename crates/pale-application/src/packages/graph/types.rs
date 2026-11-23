//! Admitted, source-free Package graph values.

use super::*;
use std::sync::Arc;

use crate::packages::{
    seal_texts, ResourceTextLink, ResourceTextReference, TextRecipe, TextRecipeBuilder, TextRef,
};

/// Domain separator for the selection-bounded semantic IR.
pub(super) const PACKAGE_SEMANTIC_IR_DOMAIN: &str = "pale.package-semantic-ir.v1";

/// Exact Markdown bytes and their one-time Pale structural projection.
#[derive(Clone)]
pub(crate) struct AdmittedMarkdown {
    text: TextRef,
    digest: pale_core::records::ArtifactDigest,
    projection: crate::markdown::MarkdownProjection,
    links: Vec<ResourceTextLink>,
    related_reading: Vec<ResourceTextReference>,
}

impl std::fmt::Debug for AdmittedMarkdown {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmittedMarkdown")
            .field("bytes", &self.text.len())
            .field("digest", &self.digest)
            .field("title", &self.title())
            .finish()
    }
}

impl AdmittedMarkdown {
    /// Admit exact text and retain its only structural Markdown projection.
    pub(super) fn from_text(owner: PackageResourceRefV1, text: &str) -> Self {
        let digest = pale_core::records::ArtifactDigest::hash_bytes(text.as_bytes());
        let projection = crate::markdown::project(text);
        let (_, text) = seal_texts([(owner, Arc::<str>::from(text))])
            .pop()
            .expect("one admitted Markdown object seals as one reference");
        Self {
            text,
            digest,
            projection,
            links: Vec::new(),
            related_reading: Vec::new(),
        }
    }

    /// Exact admitted UTF-8 source text.
    pub(crate) fn text(&self) -> &str {
        self.text.text()
    }

    /// Exact digest of [`Self::text`].
    pub(crate) fn digest(&self) -> &pale_core::records::ArtifactDigest {
        &self.digest
    }

    /// Pale-selected document title, when present.
    pub(crate) fn title(&self) -> Option<&str> {
        self.projection.title(self.text())
    }

    pub(super) fn authored_links(&self) -> &[crate::markdown::MarkdownLinkSpan] {
        self.projection.links()
    }

    pub(super) fn has_fragment(&self, fragment: &str) -> bool {
        self.projection.has_fragment(fragment)
    }

    pub(super) fn bind_links(&mut self, links: Vec<ResourceTextLink>) {
        self.links = links;
    }

    pub(super) fn links(&self) -> &[ResourceTextLink] {
        &self.links
    }

    pub(super) fn bind_related_reading(&mut self, references: Vec<ResourceTextReference>) {
        self.related_reading = references;
    }

    pub(crate) fn related_reading(&self) -> &[ResourceTextReference] {
        &self.related_reading
    }

    pub(super) fn link_is_in_body(&self, range: &std::ops::Range<usize>) -> bool {
        range.start >= self.projection.body_range().start
            && range.end <= self.projection.body_range().end
    }

    /// Reference the exact body without copying it.
    pub(crate) fn body_recipe(&self) -> TextRecipe {
        let mut recipe = TextRecipeBuilder::default();
        self.projection.append_linked_recipe(
            &self.text,
            self.projection.body_range(),
            0,
            &self.links,
            &mut recipe,
        );
        recipe.finish()
    }

    /// Reference the complete admitted document without copying it.
    pub(crate) fn text_recipe(&self) -> TextRecipe {
        let mut recipe = TextRecipeBuilder::default();
        self.projection.append_linked_recipe(
            &self.text,
            0..self.text.len(),
            0,
            &self.links,
            &mut recipe,
        );
        recipe.finish()
    }

    /// Build a lazy heading-shifted body recipe over admitted spans.
    pub(crate) fn shifted_body_recipe(&self, levels: usize) -> TextRecipe {
        let mut recipe = TextRecipeBuilder::default();
        self.projection.append_linked_recipe(
            &self.text,
            self.projection.body_range(),
            levels,
            &self.links,
            &mut recipe,
        );
        recipe.finish()
    }

    /// Rehome selected Markdown into one generation-fenced store without
    /// copying the immutable source bytes.
    fn into_unsealed(
        self,
    ) -> (
        Arc<str>,
        pale_core::records::ArtifactDigest,
        crate::markdown::MarkdownProjection,
        Vec<ResourceTextLink>,
        Vec<ResourceTextReference>,
    ) {
        let source = self
            .text
            .full_text()
            .expect("an admitted Markdown reference spans its complete immutable object");
        (
            source,
            self.digest,
            self.projection,
            self.links,
            self.related_reading,
        )
    }
}

/// Repack the selected body closure into one immutable text-store generation.
pub(super) fn repack_markdown_bodies(
    bodies: BTreeMap<PackageResourceRefV1, AdmittedMarkdown>,
) -> BTreeMap<PackageResourceRefV1, AdmittedMarkdown> {
    let mut metadata = BTreeMap::new();
    let mut text = Vec::with_capacity(bodies.len());
    for (owner, markdown) in bodies {
        let (source, digest, projection, links, related_reading) = markdown.into_unsealed();
        text.push((owner.clone(), source));
        metadata.insert(owner, (digest, projection, links, related_reading));
    }
    seal_texts(text)
        .into_iter()
        .map(|(owner, text)| {
            let (digest, projection, links, related_reading) = metadata
                .remove(&owner)
                .expect("every sealed selected text object retains its metadata");
            (
                owner,
                AdmittedMarkdown {
                    text,
                    digest,
                    projection,
                    links,
                    related_reading,
                },
            )
        })
        .collect()
}

/// Qualified fallback policy carried by one operation requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) enum AdmittedRequirementPolicy {
    /// The complete result is unavailable without the requirement.
    RequiredBlock,
    /// A named reduced result is available with qualified fallback procedure.
    RequiredDegrade {
        fallback: AdmittedFallback,
        degraded_outcome: DegradedOutcomeIdV1,
    },
    /// The capability may be omitted under a qualified fallback.
    Optional { fallback: AdmittedFallback },
}

impl AdmittedRequirementPolicy {
    /// Convert to the public generated-contract policy without source links.
    pub(crate) fn as_contract(&self) -> ResolvedCapabilityRequirementPolicyV1 {
        match self {
            Self::RequiredBlock => ResolvedCapabilityRequirementPolicyV1::RequiredBlock,
            Self::RequiredDegrade {
                fallback,
                degraded_outcome,
            } => ResolvedCapabilityRequirementPolicyV1::RequiredDegrade {
                fallback: fallback.as_contract(),
                degraded_outcome: degraded_outcome.clone(),
            },
            Self::Optional { fallback } => ResolvedCapabilityRequirementPolicyV1::Optional {
                fallback: fallback.as_contract(),
            },
        }
    }
}

/// Model-facing fallback with an already qualified optional recipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct AdmittedFallback {
    /// Model-facing guidance when the primary requirement is unavailable.
    pub(super) guidance: String,
    /// Qualified fallback recipe, when the Package supplies one.
    pub(super) recipe: Option<PackageResourceRefV1>,
}

impl AdmittedFallback {
    fn as_contract(&self) -> ResolvedCapabilityFallbackV1 {
        ResolvedCapabilityFallbackV1 {
            guidance: self.guidance.clone(),
            recipe: self.recipe.clone(),
        }
    }
}

/// One Package-owned capability requirement after every recipe link is proved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct AdmittedOperationRequirement {
    /// Requirement identity local to the owning operation.
    pub(crate) id: CapabilityRequirementIdV1,
    /// Model-facing reason for the requirement.
    pub(crate) description: String,
    /// Portable capability required from private provider configuration.
    pub(crate) capability: pale_core::records::CapabilityIdV1,
    /// Qualified primary recipe, when declared.
    pub(crate) recipe: Option<PackageResourceRefV1>,
    /// Effect ceiling asserted by the Package procedure.
    pub(crate) effect_class: EffectClass,
    /// Source-free required, degrade, or optional consequence.
    pub(crate) policy: AdmittedRequirementPolicy,
    /// Post-invocation failure posture retained from Package meaning.
    pub(crate) failure_posture: FailurePosture,
}

/// One semantic resource after authored links and body paths have been consumed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum PackageResource {
    /// Doctrine with every relationship owner-qualified.
    Doctrine {
        /// Compact purpose of the doctrine.
        description: String,
        /// Placement law for generated context.
        placement: DoctrinePlacement,
        /// Qualified doctrine inheritance edges.
        extends: Vec<PackageResourceRefV1>,
        /// Qualified required-doctrine edges.
        requires: Vec<PackageResourceRefV1>,
        /// Qualified non-normative related doctrine.
        see_also: Vec<PackageResourceRefV1>,
        /// Declared official references retained as Package meaning.
        official_sources: Vec<OfficialReference>,
    },
    /// Reusable operation archetype with qualified dependencies.
    OperationArchetype {
        /// Compact archetype purpose.
        description: String,
        /// Qualified parent-archetype edges.
        extends: Vec<PackageResourceRefV1>,
        /// Qualified doctrine edges.
        doctrine: Vec<PackageResourceRefV1>,
    },
    /// Concrete on-demand operation and its Package-owned procedure.
    Operation {
        /// Compact operation purpose.
        description: String,
        /// Qualified operation-archetype edges.
        extends: Vec<PackageResourceRefV1>,
        /// Qualified doctrine edges.
        doctrine: Vec<PackageResourceRefV1>,
        /// Qualified specialist-role edges.
        roles: Vec<PackageResourceRefV1>,
        /// Declared semantic outputs.
        outputs: Vec<String>,
        /// Capability requirements with qualified recipe custody.
        requirements: Vec<AdmittedOperationRequirement>,
        /// Qualified Package policy governing the requirements.
        capability_policy: Option<PackageResourceRefV1>,
    },
    /// Reusable role archetype with qualified dependencies.
    RoleArchetype {
        /// Compact archetype purpose.
        description: String,
        /// Qualified parent-archetype edges.
        extends: Vec<PackageResourceRefV1>,
        /// Qualified doctrine edges.
        doctrine: Vec<PackageResourceRefV1>,
    },
    /// Concrete specialist role with qualified lenses and overlays.
    Role {
        /// Compact role purpose.
        description: String,
        /// Qualified role-archetype edges.
        extends: Vec<PackageResourceRefV1>,
        /// Qualified inherited-role lenses.
        inherited_lenses: Vec<PackageResourceRefV1>,
        /// Qualified doctrine edges.
        doctrine: Vec<PackageResourceRefV1>,
        /// Qualified stack overlays.
        stacks: Vec<PackageResourceRefV1>,
    },
    /// Technology or workflow stack overlay.
    Stack {
        /// Compact stack purpose.
        description: String,
        /// Qualified parent-stack edges.
        extends: Vec<PackageResourceRefV1>,
        /// Qualified general doctrine edges.
        doctrine: Vec<PackageResourceRefV1>,
        /// Qualified role-specific doctrine edges.
        role_doctrine: Vec<PackageResourceRefV1>,
        /// Declared official references retained as Package meaning.
        official_sources: Vec<OfficialReference>,
    },
    /// One independently composable run-mode declaration.
    RunMode {
        /// Compact run-mode purpose.
        description: String,
        /// Orthogonal mode axis.
        axis: pale_core::records::ModeAxis,
        /// Additional host-facing names.
        aliases: Vec<String>,
        /// Exact user phrases selecting the mode.
        exact_triggers: Vec<String>,
        /// Qualified same-axis inheritance edges.
        extends: Vec<PackageResourceRefV1>,
        /// Lifetime over which the mode may remain selected.
        scope: pale_core::records::ModeScope,
    },
    /// Policy-family declaration whose Markdown body is retained separately.
    PolicyFamily {
        /// Human-readable family title.
        title: String,
        /// Compact normative purpose.
        purpose: String,
    },
    /// Governing contribution retained in the composition inventory.
    GoverningOrder,
    /// Package-owned capability ceiling retained in the policy inventory.
    CapabilityPolicy,
    /// Package-owned reusable procedure retained in the recipe inventory.
    ToolRecipe,
}

impl PackageResource {
    /// Required source references for navigation; selection keeps each typed use.
    pub(crate) fn requirements(&self) -> Vec<PackageResourceRefV1> {
        self.required_uses()
            .into_iter()
            .map(|usage| usage.resource)
            .collect()
    }

    /// All declared relationships, including optional related reading.
    pub(crate) fn dependencies(&self) -> Vec<PackageResourceRefV1> {
        let mut references = self.requirements();
        if let Self::Doctrine { see_also, .. } = self {
            references.extend(see_also.iter().cloned());
        }
        references
    }
}

pub(super) fn requirement_policy_recipe(
    policy: &AdmittedRequirementPolicy,
) -> impl Iterator<Item = &PackageResourceRefV1> {
    match policy {
        AdmittedRequirementPolicy::RequiredBlock => None,
        AdmittedRequirementPolicy::RequiredDegrade { fallback, .. }
        | AdmittedRequirementPolicy::Optional { fallback } => fallback.recipe.as_ref(),
    }
    .into_iter()
}

/// Required and optional reading for a header-authored use of one page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct AdmittedPageReading {
    pub(crate) required: Vec<PackageResourceRefV1>,
    pub(crate) related: Vec<PackageResourceRefV1>,
    pub(crate) official_sources: Vec<OfficialReference>,
}

/// Qualified resource plus its manifest visibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct AdmittedResourceNode {
    /// Header-authored reading, kept separate from composed legacy bodies.
    pub(crate) page: Option<AdmittedPageReading>,
    /// Visibility across the owning Package boundary.
    pub(crate) exposure: PackageResourceExposureV1,
    /// Source-free admitted resource value.
    pub(crate) resource: PackageResource,
}

impl AdmittedResourceNode {
    pub(crate) fn required_uses(&self) -> Vec<PackageResourceUse> {
        let mut uses = self.resource.required_uses();
        if let Some(page) = &self.page {
            uses.extend(page.required.iter().cloned().map(|resource| {
                PackageResourceUse::new(resource, PackageResourceKindV1::Doctrine)
            }));
        }
        uses
    }
    pub(crate) fn requirements(&self) -> Vec<PackageResourceRefV1> {
        self.required_uses()
            .into_iter()
            .map(|usage| usage.resource)
            .collect()
    }
    pub(crate) fn dependencies(&self) -> Vec<PackageResourceRefV1> {
        let mut references = self.resource.dependencies();
        if let Some(page) = &self.page {
            references.extend(page.required.iter().chain(&page.related).cloned());
        }
        references
    }
}

/// Fully qualified resource inventory before occurrence selection.
#[derive(Debug, Clone)]
pub(crate) struct AdmittedPackageGraph {
    /// Exact releases selected through the distribution lock.
    pub(super) releases: Vec<PackageReleaseV1>,
    /// Qualified admitted resources before occurrence selection.
    pub(in crate::packages) resources: BTreeMap<PackageResourceUse, AdmittedResourceNode>,
    /// Admitted Markdown retained once for every content-bearing resource.
    pub(in crate::packages) bodies: BTreeMap<PackageResourceRefV1, AdmittedMarkdown>,
    /// Package-owned capability ceilings keyed by resource owner.
    pub(super) capability_policies: BTreeMap<PackageResourceRefV1, CapabilityPolicy>,
    /// Package-owned procedures keyed by resource owner.
    pub(super) recipes: BTreeMap<PackageResourceRefV1, ToolRecipe>,
    /// Governing contributions keyed by their exact contributor resource.
    pub(in crate::packages) governing:
        BTreeMap<PackageResourceRefV1, ResolvedGoverningContributionV1>,
}

/// Repository-local result of the positive governing fixed point.
#[derive(Debug, Serialize)]
pub(crate) struct RepositoryPackageSelection {
    /// Complete ordinary and governing dependency closure for this repository.
    pub(crate) resources: BTreeSet<PackageResourceUse>,
    /// Governing algebra activated only by global and repository-local scopes.
    pub(crate) governing_order: AdmittedGoverningOrderV1,
}

/// Selected, dependency-closed, digest-bound Package semantic IR.
#[derive(Debug)]
pub struct SealedPackageGraph {
    /// Exact selected state covered by the canonical digest.
    pub(super) state: SelectedPackageGraphState,
    /// Canonical seal over every retained semantic-IR member.
    pub(super) digest: PackageSemanticIrDigest,
}

/// Complete selected Package graph state before its canonical digest is attached.
#[derive(Debug)]
pub(super) struct SelectedPackageGraphState {
    /// Exact releases represented by the selected closure.
    pub(super) releases: Vec<PackageReleaseV1>,
    /// Global semantic occurrence seeds.
    pub(super) global_seeds: BTreeSet<PackageResourceUse>,
    /// Repository-specific semantic occurrence seeds.
    pub(super) repository_seeds: BTreeMap<LogicalRepositoryIdV1, BTreeSet<PackageResourceUse>>,
    /// Repository-local resource closures and governing results.
    pub(super) repository_selections: BTreeMap<LogicalRepositoryIdV1, RepositoryPackageSelection>,
    /// Dependency-closed selected semantic resources.
    pub(super) resources: BTreeMap<PackageResourceUse, AdmittedResourceNode>,
    /// Single admitted body custody for selected content-bearing resources.
    pub(super) bodies: BTreeMap<PackageResourceRefV1, AdmittedMarkdown>,
    /// Selected Package-owned capability ceilings.
    pub(super) capability_policies: BTreeMap<PackageResourceRefV1, CapabilityPolicy>,
    /// Selected Package-owned reusable procedures.
    pub(super) recipes: BTreeMap<PackageResourceRefV1, ToolRecipe>,
    /// Whole-profile governing result retained for contradiction proof only.
    pub(super) governing_order: AdmittedGoverningOrderV1,
}

/// Package definitions consumed by one resident generation store.
///
/// Repository scopes are membership facts only. Dependency adjacency is
/// projected from each [`AdmittedResourceNode`] into the resident packed graph
/// and is not retained here as a second topology.
pub(crate) struct ResidentPackageParts {
    pub(crate) global_seeds: BTreeSet<PackageResourceUse>,
    pub(crate) repository_seeds: BTreeMap<LogicalRepositoryIdV1, BTreeSet<PackageResourceUse>>,
    pub(crate) resources: BTreeMap<PackageResourceUse, AdmittedResourceNode>,
    pub(crate) bodies: BTreeMap<PackageResourceRefV1, AdmittedMarkdown>,
    pub(crate) capability_policies: BTreeMap<PackageResourceRefV1, CapabilityPolicy>,
    pub(crate) recipes: BTreeMap<PackageResourceRefV1, ToolRecipe>,
    pub(crate) repository_scopes: BTreeMap<LogicalRepositoryIdV1, RepositoryPackageSelection>,
    pub(crate) digest: PackageSemanticIrDigest,
}

#[derive(Serialize)]
pub(super) struct MarkdownSeal<'a> {
    /// Owner-qualified content resource.
    pub(super) resource: &'a PackageResourceRefV1,
    /// Digest of the exact admitted Markdown bytes.
    pub(super) digest: &'a pale_core::records::ArtifactDigest,
    /// Exact admitted Markdown bytes.
    pub(super) text: &'a str,
    /// Qualified navigation is sealed with exact bytes, without reopening source files.
    #[serde(skip_serializing_if = "<[ResourceTextLink]>::is_empty")]
    pub(super) links: &'a [ResourceTextLink],
    /// Declared optional reading retains only its title and Package location.
    #[serde(skip_serializing_if = "<[ResourceTextReference]>::is_empty")]
    pub(super) related_reading: &'a [ResourceTextReference],
}

/// A resource key and its record, serialized as an array entry.
///
/// Keys such as [`PackageResourceRefV1`] contain several fields. JSON object
/// keys must be strings, so the graph stores these pairs in an array.
#[derive(Serialize)]
pub(super) struct ResourceRecord<'a, T, K = PackageResourceRefV1> {
    /// Resource key, including its Package identity.
    pub(super) resource: &'a K,
    /// Record stored under that key.
    pub(super) record: &'a T,
}

#[derive(Serialize)]
pub(super) struct PackageGraphPreimage<'a> {
    /// Domain separator for the initial-v1 semantic IR.
    pub(super) schema: &'static str,
    /// Exact selected Package releases.
    pub(super) releases: &'a [PackageReleaseV1],
    /// Global occurrence seeds.
    pub(super) global_seeds: &'a BTreeSet<PackageResourceUse>,
    /// Repository-local occurrence seeds.
    pub(super) repository_seeds: &'a BTreeMap<LogicalRepositoryIdV1, BTreeSet<PackageResourceUse>>,
    /// Repository-local positive fixed points and governing results.
    pub(super) repository_selections:
        &'a BTreeMap<LogicalRepositoryIdV1, RepositoryPackageSelection>,
    /// Qualified dependency-closed resources.
    pub(super) resources: Vec<ResourceRecord<'a, AdmittedResourceNode, PackageResourceUse>>,
    /// Content identities and exact admitted bytes.
    pub(super) bodies: Vec<MarkdownSeal<'a>>,
    /// Selected capability policies.
    pub(super) capability_policies: Vec<ResourceRecord<'a, CapabilityPolicy>>,
    /// Selected reusable procedures.
    pub(super) recipes: Vec<ResourceRecord<'a, ToolRecipe>>,
    /// Whole-profile governing result retained for contradiction proof only.
    pub(super) governing_order: &'a AdmittedGoverningOrderV1,
}

#[cfg(test)]
#[path = "../../../tests/internal/packages/graph/types.rs"]
mod tests;
