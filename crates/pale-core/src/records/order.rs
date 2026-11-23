//! Scoped governing constraints, narrowing relations, and semantic partial orders.
//!
//! These records keep constraint, specialization, reasoning, execution, and
//! presentation relations distinct. Source order has no semantic force:
//! admission validates identities and canonicalizes each relation set before a
//! compiler derives a deterministic linear view for a particular consumer.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::records::{
    CapabilityIdV1, CapabilityNarrowingV1, CapabilityPolicy, EffectClass, GoverningResourceLinkV1,
    PackageResourceLinkV1, PackageResourceRefV1, PackageResourceUseV1,
    MAX_CAPABILITY_POLICY_AXIS_MEMBERS,
};

/// Maximum declarations in each governing relation of one initial-v1 profile.
pub const MAX_GOVERNING_RELATIONS: usize = 4_096;

/// One parsed governing-relation fragment.
///
/// A Package may export this record like any other semantic resource. It is not
/// a distribution-global order and cannot acquire authority over unrelated
/// Package resources merely because its file was discovered first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GoverningOrderFile {
    /// The single governing-order declaration in this file.
    pub governing_order: GoverningOrderV1,
}

/// Neutral algebra for scoped governing relations.
///
/// [`Self::constraints`], [`Self::narrowing`], and [`Self::order`] are separate
/// relations. In particular, an ordering edge does not create a governing
/// ceiling, and a dependency or constraint does not imply presentation order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GoverningOrderV1 {
    /// Version of the governing-order declaration contract.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Scoped ceiling relations contributed by this semantic fragment.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub constraints: Vec<GoverningConstraintV1>,
    /// Obligations accumulated with contributor provenance.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub obligations: Vec<GoverningObligationV1>,
    /// Exclusive-authority claims checked for conflicting claimants.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub exclusive_authority: Vec<GoverningExclusiveAuthorityClaimV1>,
    /// Proposed specializations that admission must prove non-widening.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub narrowing: Vec<GoverningNarrowingV1>,
    /// Purpose-typed edges in scoped semantic partial orders.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub order: Vec<ScopedOrderRelationV1>,
}

/// Normalized axis-separated Package capability policy in admitted semantic IR.
///
/// In the rendered meet law, `P` and `F` project permitted and forbidden
/// members. [`Self::meet`] is the greatest result no wider than either input
/// and is commutative, associative, and idempotent after normalization.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/capability-policy-meet.display.mathml"
))]
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct AdmittedCapabilityPolicyV1 {
    /// Capabilities permitted by every contributing policy.
    pub permitted_capabilities: BTreeSet<CapabilityIdV1>,
    /// Capabilities forbidden by any contributing policy.
    pub forbidden_capabilities: BTreeSet<CapabilityIdV1>,
    /// Effects permitted by every contributing policy.
    pub permitted_effects: BTreeSet<EffectClass>,
    /// Effects forbidden by any contributing policy.
    pub forbidden_effects: BTreeSet<EffectClass>,
}

impl AdmittedCapabilityPolicyV1 {
    fn from_policy(policy: &CapabilityPolicy) -> Self {
        Self {
            permitted_capabilities: policy.permitted_capabilities.iter().cloned().collect(),
            forbidden_capabilities: policy.forbidden_capabilities.iter().cloned().collect(),
            permitted_effects: policy.permitted_effects.iter().copied().collect(),
            forbidden_effects: policy.forbidden_effects.iter().copied().collect(),
        }
        .normalized()
    }

    fn from_narrowing(narrowing: &CapabilityNarrowingV1) -> Self {
        let policy = CapabilityPolicy {
            permitted_capabilities: narrowing.permitted_capabilities.clone(),
            forbidden_capabilities: narrowing.forbidden_capabilities.clone(),
            permitted_effects: narrowing.permitted_effects.clone(),
            forbidden_effects: narrowing.forbidden_effects.clone(),
        };
        Self::from_policy(&policy)
    }

    /// Compute the deterministic meet of two admitted ceilings.
    #[must_use]
    pub fn meet(&self, other: &Self) -> Self {
        let forbidden_capabilities = self
            .forbidden_capabilities
            .union(&other.forbidden_capabilities)
            .cloned()
            .collect::<BTreeSet<_>>();
        let forbidden_effects = self
            .forbidden_effects
            .union(&other.forbidden_effects)
            .copied()
            .collect::<BTreeSet<_>>();
        let permitted_capabilities = self
            .permitted_capabilities
            .intersection(&other.permitted_capabilities)
            .filter(|value| !forbidden_capabilities.contains(*value))
            .cloned()
            .collect();
        let permitted_effects = self
            .permitted_effects
            .intersection(&other.permitted_effects)
            .filter(|value| !forbidden_effects.contains(*value))
            .copied()
            .collect();
        Self {
            permitted_capabilities,
            forbidden_capabilities,
            permitted_effects,
            forbidden_effects,
        }
    }

    fn normalized(&self) -> Self {
        Self {
            permitted_capabilities: self
                .permitted_capabilities
                .difference(&self.forbidden_capabilities)
                .cloned()
                .collect(),
            forbidden_capabilities: self.forbidden_capabilities.clone(),
            permitted_effects: self
                .permitted_effects
                .difference(&self.forbidden_effects)
                .copied()
                .collect(),
            forbidden_effects: self.forbidden_effects.clone(),
        }
    }
}

/// Meet a private narrowing with an already Package-owned capability ceiling.
///
/// This delegates to [`AdmittedCapabilityPolicyV1::meet`], so Package-to-
/// Package composition and Package-to-private narrowing share one semilattice
/// law. The private record may remove permitted values or add forbiddances; it
/// cannot introduce a capability or effect absent from the Package ceiling.
///
/// # Errors
///
/// Returns [`GoverningAlgebraErrorV1::InvalidCapabilityNarrowing`] for duplicate,
/// contradictory, or widening private axes.
pub fn meet_capability_narrowing(
    package: &CapabilityPolicy,
    narrowing: &CapabilityNarrowingV1,
) -> Result<AdmittedCapabilityPolicyV1, GoverningAlgebraErrorV1> {
    if !capability_policy_is_normalized(package)
        || !capability_narrowing_is_normalized(package, narrowing)
    {
        return Err(GoverningAlgebraErrorV1::InvalidCapabilityNarrowing);
    }
    Ok(AdmittedCapabilityPolicyV1::from_policy(package)
        .meet(&AdmittedCapabilityPolicyV1::from_narrowing(narrowing)))
}

/// One governing ceiling that applies within an explicit semantic scope.
///
/// Admission composes all applicable ceilings by their domain-specific meet.
/// This edge does not transfer the governor's Package ownership or any native
/// authority to the subject.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct GoverningConstraintV1 {
    /// Resource or operation occurrence within which the ceiling applies.
    pub scope: GoverningResourceLinkV1,
    /// Resource whose admitted use must satisfy the ceiling.
    pub subject: GoverningResourceLinkV1,
    /// Package-owned [`CapabilityPolicy`] resource contributing the ceiling.
    pub policy: PackageResourceLinkV1,
}

/// One Package-owned obligation contributed within a semantic scope.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct GoverningObligationV1 {
    /// Resource or occurrence within which the obligation applies.
    pub scope: GoverningResourceLinkV1,
    /// Resource whose admitted use carries the obligation.
    pub subject: GoverningResourceLinkV1,
    /// Portable obligation identity.
    pub obligation: GoverningResourceLinkV1,
}

/// One provenance-preserving accumulated obligation.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct AccumulatedGoverningObligationV1 {
    /// Resource or occurrence within which the obligation applies.
    pub scope: PackageResourceUseV1,
    /// Resource whose use carries the obligation.
    pub subject: PackageResourceUseV1,
    /// Portable obligation identity.
    pub obligation: PackageResourceUseV1,
    /// Every distinct semantic contributor, in canonical order.
    pub contributors: BTreeSet<PackageResourceRefV1>,
}

/// One claim that an authority has exactly one semantic claimant in a scope.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct GoverningExclusiveAuthorityClaimV1 {
    /// Resource or occurrence within which exclusivity applies.
    pub scope: GoverningResourceLinkV1,
    /// Owner-qualified authority being claimed.
    pub authority: GoverningResourceLinkV1,
}

/// One authored specialization claim within a semantic scope.
///
/// This row is not proof. Admission must establish from exact resource kinds
/// and bodies that `narrower` preserves or reduces every applicable ceiling of
/// `broader` before constructing [`ResolvedGoverningNarrowingV1`]. Matching
/// names or Package dependency reachability do not establish the relation.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct GoverningNarrowingV1 {
    /// Resource or occurrence within which the narrowing is used.
    pub scope: GoverningResourceLinkV1,
    /// More specialized resource that may not widen the broader contract.
    pub narrower: GoverningResourceLinkV1,
    /// Reusable parent or interface being narrowed.
    pub broader: GoverningResourceLinkV1,
}

/// Why a semantic partial-order edge exists.
///
/// The variants are deliberately non-substitutable: a deterministic renderer
/// tie-break never becomes an execution or reasoning dependency.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SemanticOrderPurposeV1 {
    /// Dependency-first order for composing semantic bodies.
    Composition,
    /// Reasoning sequence required by one operation or role.
    Reasoning,
    /// Operational prerequisite order without effect authorization.
    Execution,
    /// Author-declared presentation order for one consumer projection.
    Presentation,
}

/// One edge in a scoped, purpose-typed semantic partial order.
///
/// Independent resources have no edge. A compiler may use a stable tie-breaker
/// while topologically sorting, but that derived linearization must not be
/// persisted as another semantic relation.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ScopedOrderRelationV1 {
    /// Resource or occurrence whose local order contains this edge.
    pub scope: GoverningResourceLinkV1,
    /// Semantic reason for ordering the two resources.
    pub purpose: SemanticOrderPurposeV1,
    /// Prerequisite resource.
    pub before: GoverningResourceLinkV1,
    /// Dependent resource.
    pub after: GoverningResourceLinkV1,
}

/// Effective ceiling after meeting every contributor for one subject.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct EffectiveGoverningCeilingV1 {
    /// Semantic scope of the result.
    pub scope: PackageResourceUseV1,
    /// Governed subject.
    pub subject: PackageResourceUseV1,
    /// Canonical set of contributors retained as proof provenance.
    pub contributors: BTreeSet<PackageResourceRefV1>,
    /// Exact Package policy resources whose bodies were met.
    pub policies: BTreeSet<PackageResourceRefV1>,
    /// Deterministic normalized meet of every contributor policy.
    pub policy: AdmittedCapabilityPolicyV1,
}

/// Qualified exclusive-authority claim in admitted semantic IR.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct AdmittedExclusiveAuthorityClaimV1 {
    /// Owner-qualified scope.
    pub scope: PackageResourceUseV1,
    /// Owner-qualified exclusive authority.
    pub authority: PackageResourceUseV1,
    /// Owner-qualified claimant.
    pub claimant: PackageResourceRefV1,
}

/// Qualified narrowing edge whose semantic proof was established by Package admission.
///
/// Core can validate canonical structure and cycles, but refs alone cannot
/// prove that the proposed policy is no broader. The application must prove
/// that law from exact
/// admitted resource kinds and bodies before constructing this record.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ResolvedGoverningNarrowingV1 {
    /// Owner-qualified scope.
    pub scope: PackageResourceUseV1,
    /// More specialized resource.
    pub narrower: PackageResourceUseV1,
    /// Broader resource that may not be widened.
    pub broader: PackageResourceUseV1,
    /// Manifest-owned `GoverningOrder` resource that contributed the edge.
    pub contributor: PackageResourceRefV1,
}

/// Qualified purpose-typed order edge in admitted semantic IR.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct AdmittedScopedOrderRelationV1 {
    /// Owner-qualified scope.
    pub scope: PackageResourceUseV1,
    /// Semantic reason for the edge.
    pub purpose: SemanticOrderPurposeV1,
    /// Owner-qualified prerequisite.
    pub before: PackageResourceUseV1,
    /// Owner-qualified dependent.
    pub after: PackageResourceUseV1,
    /// Manifest-owned `GoverningOrder` resource that contributed the edge.
    pub contributor: PackageResourceRefV1,
}

/// Validated and canonically ordered governing algebra.
///
/// Construction is available only through
/// [`ResolvedGoverningContributionV1::compose`]. Authored
/// [`GoverningOrderV1`] fragments contain unresolved links and therefore cannot
/// be hashed or relabelled as admitted semantic IR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdmittedGoverningOrderV1 {
    /// Governing algebra major.
    #[schemars(extend("const" = 1))]
    schema_version: u32,
    /// Effective ceiling meets with contributor provenance.
    ceilings: Vec<EffectiveGoverningCeilingV1>,
    /// Accumulated obligations with contributor provenance.
    obligations: Vec<AccumulatedGoverningObligationV1>,
    /// Canonical non-conflicting exclusive-authority claims.
    exclusive_authority: Vec<AdmittedExclusiveAuthorityClaimV1>,
    /// Canonical qualified narrowing relation.
    narrowing: Vec<ResolvedGoverningNarrowingV1>,
    /// Canonical purpose-separated partial orders.
    order: Vec<AdmittedScopedOrderRelationV1>,
}

impl AdmittedGoverningOrderV1 {
    /// Effective ceiling meets with contributor provenance.
    #[must_use]
    pub fn ceilings(&self) -> &[EffectiveGoverningCeilingV1] {
        &self.ceilings
    }

    /// Accumulated obligations with every semantic contributor retained.
    #[must_use]
    pub fn obligations(&self) -> &[AccumulatedGoverningObligationV1] {
        &self.obligations
    }

    /// Canonical non-conflicting exclusive-authority claims.
    #[must_use]
    pub fn exclusive_authority(&self) -> &[AdmittedExclusiveAuthorityClaimV1] {
        &self.exclusive_authority
    }

    /// Canonical qualified narrowing relation.
    #[must_use]
    pub fn narrowing(&self) -> &[ResolvedGoverningNarrowingV1] {
        &self.narrowing
    }

    /// Canonical purpose-separated partial orders.
    #[must_use]
    pub fn order(&self) -> &[AdmittedScopedOrderRelationV1] {
        &self.order
    }

    /// Revalidate a deserialized admitted value before it is sealed.
    ///
    /// # Errors
    ///
    /// Returns [`GoverningAlgebraErrorV1`] if any relation is non-canonical,
    /// conflicting, or cyclic under a narrowing or partial-order law.
    pub fn validate(&self) -> Result<(), GoverningAlgebraErrorV1> {
        if self.schema_version != 1 {
            return Err(GoverningAlgebraErrorV1::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        require_canonical("effective-ceiling", &self.ceilings)?;
        require_canonical("accumulated-obligation", &self.obligations)?;
        require_canonical("exclusive-authority", &self.exclusive_authority)?;
        require_canonical("narrowing", &self.narrowing)?;
        require_canonical("partial-order", &self.order)?;
        for ceiling in &self.ceilings {
            if ceiling.contributors.is_empty() || ceiling.policies.is_empty() {
                return Err(GoverningAlgebraErrorV1::EmptyProvenance {
                    relation: "effective-ceiling",
                });
            }
            if ceiling.policy != ceiling.policy.normalized() {
                return Err(GoverningAlgebraErrorV1::NonNormalizedCeiling);
            }
        }
        if self
            .obligations
            .iter()
            .any(|obligation| obligation.contributors.is_empty())
        {
            return Err(GoverningAlgebraErrorV1::EmptyProvenance {
                relation: "accumulated-obligation",
            });
        }
        validate_exclusive_authority(&self.exclusive_authority)?;
        validate_rejected_cycles(&self.narrowing, &self.order)?;
        validate_effective_narrowing(&self.ceilings, &self.obligations, &self.narrowing)
    }
}

/// A governing declaration cannot be admitted under the initial-v1 laws.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GoverningAlgebraErrorV1 {
    /// The declaration uses another schema major.
    #[error("governing-order schema_version must be 1, got {0}")]
    UnsupportedSchemaVersion(u32),
    /// One relation inventory exceeds its fixed admission bound.
    #[error("governing {relation} relation has {actual} entries; maximum is {maximum}")]
    RelationTooLarge {
        /// Stable relation name.
        relation: &'static str,
        /// Observed entry count.
        actual: usize,
        /// Initial-v1 maximum.
        maximum: usize,
    },
    /// One exact declaration is repeated.
    #[error("governing {relation} relation contains a duplicate declaration")]
    DuplicateRelation {
        /// Stable relation name.
        relation: &'static str,
    },
    /// An admitted relation is not strictly ordered or normalized.
    #[error("admitted governing {relation} relation is not canonical")]
    NonCanonicalRelation {
        /// Stable relation name.
        relation: &'static str,
    },
    /// A derived fact lost the Package resources that prove its origin.
    #[error("admitted governing {relation} fact has empty provenance")]
    EmptyProvenance {
        /// Stable derived relation name.
        relation: &'static str,
    },
    /// One resolved Package policy contains duplicate or contradictory axes.
    #[error("CapabilityPolicy resource {policy:?} is not normalized")]
    InvalidCapabilityPolicy {
        /// Exact Package policy resource.
        policy: PackageResourceRefV1,
    },
    /// A resolved constraint has no corresponding admitted policy body.
    #[error("resolved governing constraint references missing CapabilityPolicy {policy:?}")]
    MissingCapabilityPolicy {
        /// Exact Package policy coordinate absent from the supplied inventory.
        policy: PackageResourceRefV1,
    },
    /// Private capability policy is contradictory or widens Package meaning.
    #[error("private capability narrowing is duplicate, contradictory, or widening")]
    InvalidCapabilityNarrowing,
    /// One effective ceiling contains a permitted/forbidden overlap.
    #[error("effective governing ceiling is not normalized")]
    NonNormalizedCeiling,
    /// Two different resources claim one exclusive authority.
    #[error("exclusive authority {authority:?} in scope {scope:?} has conflicting claimants")]
    ExclusiveAuthorityConflict {
        /// Scope containing the conflict.
        scope: PackageResourceUseV1,
        /// Authority claimed by multiple resources.
        authority: Box<PackageResourceUseV1>,
        /// Canonical conflicting claimant set.
        claimants: BTreeSet<PackageResourceRefV1>,
    },
    /// A narrowing relation is cyclic.
    #[error("governing narrowing contains a cycle in scope {scope:?}: {edges:?}")]
    NarrowingCycle {
        /// Scope containing the invalid cycle.
        scope: PackageResourceUseV1,
        /// Stable closed resource path witnessing the cycle.
        members: Vec<PackageResourceUseV1>,
        /// Canonical admitted edges that form the witnessed path.
        edges: Vec<ResolvedGoverningNarrowingV1>,
    },
    /// A purpose-scoped order is cyclic.
    #[error("governing {purpose:?} partial order contains a cycle in scope {scope:?}: {edges:?}")]
    PartialOrderCycle {
        /// Scope containing the invalid cycle.
        scope: PackageResourceUseV1,
        /// Purpose whose order is cyclic.
        purpose: SemanticOrderPurposeV1,
        /// Stable closed resource path witnessing the cycle.
        members: Vec<PackageResourceUseV1>,
        /// Canonical admitted edges that form the witnessed path.
        edges: Vec<AdmittedScopedOrderRelationV1>,
    },
    /// A resource's own policy permits values excluded by a proven broader policy.
    #[error("resolved narrowing widens its broader policy: {edge:?}")]
    NarrowingWidens {
        /// Exact proven narrowing edge.
        edge: Box<ResolvedGoverningNarrowingV1>,
        /// Capability values responsible for widening.
        capabilities: BTreeSet<CapabilityIdV1>,
        /// Effect values responsible for widening.
        effects: BTreeSet<EffectClass>,
    },
    /// A serialized admitted algebra omits a fact implied by a narrowing edge.
    #[error("admitted narrowing closure is incomplete for edge {edge:?}")]
    IncompleteNarrowingClosure {
        /// Exact edge whose broader facts were not retained by the narrower.
        edge: Box<ResolvedGoverningNarrowingV1>,
    },
}

/// Already-resolved ceiling contribution supplied by Package admission.
///
/// The application layer validates every authored [`GoverningResourceLinkV1`]
/// against the selected dependency/export/kind index and stamps `contributor`
/// from the enclosing manifest-owned `GoverningOrder` resource. Core accepts no
/// unresolved source link and therefore cannot fabricate that proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedGoverningConstraintV1 {
    /// Qualified semantic scope.
    pub scope: PackageResourceUseV1,
    /// Qualified governed subject.
    pub subject: PackageResourceUseV1,
    /// Exact admitted `CapabilityPolicy` resource.
    pub policy: PackageResourceRefV1,
    /// Enclosing `GoverningOrder` resource that contributed the relation.
    pub contributor: PackageResourceRefV1,
}

/// Already-resolved obligation contribution supplied by Package admission.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResolvedGoverningObligationV1 {
    /// Qualified semantic scope.
    pub scope: PackageResourceUseV1,
    /// Qualified governed subject.
    pub subject: PackageResourceUseV1,
    /// Qualified obligation identity.
    pub obligation: PackageResourceUseV1,
    /// Enclosing `GoverningOrder` resource that contributed the relation.
    pub contributor: PackageResourceRefV1,
}

/// One Package-resolved contribution fragment.
///
/// The application creates this only after validating the enclosing source
/// fragment. [`Self::compose`] must receive all selected fragments together so
/// a cross-Package meet, conflict, or cycle cannot escape fragment-local work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedGoverningContributionV1 {
    /// Resolved ceiling contributions.
    pub constraints: Vec<ResolvedGoverningConstraintV1>,
    /// Resolved obligation contributions.
    pub obligations: Vec<ResolvedGoverningObligationV1>,
    /// Resolved exclusive-authority claims.
    pub exclusive_authority: Vec<AdmittedExclusiveAuthorityClaimV1>,
    /// Resolved narrowing edges.
    pub narrowing: Vec<ResolvedGoverningNarrowingV1>,
    /// Resolved purpose-typed order edges.
    pub order: Vec<AdmittedScopedOrderRelationV1>,
}

impl ResolvedGoverningContributionV1 {
    /// Compose every selected Package contribution into one admitted algebra.
    ///
    /// # Errors
    ///
    /// Returns [`GoverningAlgebraErrorV1`] for an exceeded bound, duplicate row,
    /// exclusive-authority conflict, malformed policy axis, or invalid
    /// narrowing/purpose-order cycle.
    pub fn compose(
        contributions: impl IntoIterator<Item = Self>,
        capability_policies: &BTreeMap<PackageResourceRefV1, CapabilityPolicy>,
    ) -> Result<AdmittedGoverningOrderV1, GoverningAlgebraErrorV1> {
        let mut constraints = Vec::new();
        let mut obligations = Vec::new();
        let mut exclusive_authority = Vec::new();
        let mut narrowing = Vec::new();
        let mut order = Vec::new();
        for contribution in contributions {
            constraints.extend(contribution.constraints);
            obligations.extend(contribution.obligations);
            exclusive_authority.extend(contribution.exclusive_authority);
            narrowing.extend(contribution.narrowing);
            order.extend(contribution.order);
        }

        let constraints = canonical_resolved_constraints(constraints, capability_policies)?;
        let obligations = canonical_relations("resolved-obligation", obligations)?;
        let exclusive_authority =
            canonical_relations("resolved-exclusive-authority", exclusive_authority)?;
        let narrowing = canonical_relations("resolved-narrowing", narrowing)?;
        let order = canonical_relations("resolved-partial-order", order)?;
        validate_exclusive_authority(&exclusive_authority)?;
        validate_rejected_cycles(&narrowing, &order)?;
        let mut ceilings = meet_ceilings(&constraints);
        let mut obligations = accumulate_obligations(&obligations);
        apply_narrowing(&mut ceilings, &mut obligations, &narrowing)?;
        let admitted = AdmittedGoverningOrderV1 {
            schema_version: 1,
            ceilings,
            obligations,
            exclusive_authority,
            narrowing,
            order,
        };
        admitted.validate()?;
        Ok(admitted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ResolvedCeilingContributionV1 {
    scope: PackageResourceUseV1,
    policy: PackageResourceRefV1,
    subject: PackageResourceUseV1,
    contributor: PackageResourceRefV1,
    effective_policy: AdmittedCapabilityPolicyV1,
}

fn canonical_resolved_constraints(
    constraints: Vec<ResolvedGoverningConstraintV1>,
    capability_policies: &BTreeMap<PackageResourceRefV1, CapabilityPolicy>,
) -> Result<Vec<ResolvedCeilingContributionV1>, GoverningAlgebraErrorV1> {
    if constraints.len() > MAX_GOVERNING_RELATIONS {
        return Err(GoverningAlgebraErrorV1::RelationTooLarge {
            relation: "resolved-constraint",
            actual: constraints.len(),
            maximum: MAX_GOVERNING_RELATIONS,
        });
    }
    for constraint in &constraints {
        let policy_body = capability_policies.get(&constraint.policy).ok_or_else(|| {
            GoverningAlgebraErrorV1::MissingCapabilityPolicy {
                policy: constraint.policy.clone(),
            }
        })?;
        if !capability_policy_is_normalized(policy_body) {
            return Err(GoverningAlgebraErrorV1::InvalidCapabilityPolicy {
                policy: constraint.policy.clone(),
            });
        }
    }
    canonical_relations(
        "resolved-constraint",
        constraints.into_iter().map(|constraint| {
            let policy_body = capability_policies
                .get(&constraint.policy)
                .expect("policy inventory was validated before canonicalization");
            ResolvedCeilingContributionV1 {
                scope: constraint.scope,
                subject: constraint.subject,
                policy: constraint.policy,
                contributor: constraint.contributor,
                effective_policy: AdmittedCapabilityPolicyV1::from_policy(policy_body),
            }
        }),
    )
}

fn capability_policy_is_normalized(policy: &CapabilityPolicy) -> bool {
    let permitted_capabilities = policy
        .permitted_capabilities
        .iter()
        .collect::<BTreeSet<_>>();
    let forbidden_capabilities = policy
        .forbidden_capabilities
        .iter()
        .collect::<BTreeSet<_>>();
    let permitted_effects = policy.permitted_effects.iter().collect::<BTreeSet<_>>();
    let forbidden_effects = policy.forbidden_effects.iter().collect::<BTreeSet<_>>();
    [
        policy.permitted_capabilities.len(),
        policy.forbidden_capabilities.len(),
        policy.permitted_effects.len(),
        policy.forbidden_effects.len(),
    ]
    .into_iter()
    .all(|length| length <= MAX_CAPABILITY_POLICY_AXIS_MEMBERS)
        && permitted_capabilities.len() == policy.permitted_capabilities.len()
        && forbidden_capabilities.len() == policy.forbidden_capabilities.len()
        && permitted_effects.len() == policy.permitted_effects.len()
        && forbidden_effects.len() == policy.forbidden_effects.len()
        && permitted_capabilities.is_disjoint(&forbidden_capabilities)
        && permitted_effects.is_disjoint(&forbidden_effects)
}

fn capability_narrowing_is_normalized(
    package: &CapabilityPolicy,
    narrowing: &CapabilityNarrowingV1,
) -> bool {
    let narrowed = CapabilityPolicy {
        permitted_capabilities: narrowing.permitted_capabilities.clone(),
        forbidden_capabilities: narrowing.forbidden_capabilities.clone(),
        permitted_effects: narrowing.permitted_effects.clone(),
        forbidden_effects: narrowing.forbidden_effects.clone(),
    };
    if !capability_policy_is_normalized(&narrowed) {
        return false;
    }
    let package_capabilities = package
        .permitted_capabilities
        .iter()
        .collect::<BTreeSet<_>>();
    let package_effects = package.permitted_effects.iter().collect::<BTreeSet<_>>();
    narrowing
        .permitted_capabilities
        .iter()
        .all(|capability| package_capabilities.contains(capability))
        && narrowing
            .permitted_effects
            .iter()
            .all(|effect| package_effects.contains(effect))
}

fn canonical_relations<T: Ord>(
    relation: &'static str,
    values: impl IntoIterator<Item = T>,
) -> Result<Vec<T>, GoverningAlgebraErrorV1> {
    let values = values.into_iter().collect::<Vec<_>>();
    let source_len = values.len();
    if source_len > MAX_GOVERNING_RELATIONS {
        return Err(GoverningAlgebraErrorV1::RelationTooLarge {
            relation,
            actual: source_len,
            maximum: MAX_GOVERNING_RELATIONS,
        });
    }
    let unique = values.into_iter().collect::<BTreeSet<_>>();
    if unique.len() < source_len {
        return Err(GoverningAlgebraErrorV1::DuplicateRelation { relation });
    }
    Ok(unique.into_iter().collect())
}

fn require_canonical<T: Ord + Clone + PartialEq>(
    relation: &'static str,
    values: &[T],
) -> Result<(), GoverningAlgebraErrorV1> {
    let canonical = canonical_relations(relation, values.iter().cloned())?;
    if canonical != values {
        return Err(GoverningAlgebraErrorV1::NonCanonicalRelation { relation });
    }
    Ok(())
}

fn meet_ceilings(
    constraints: &[ResolvedCeilingContributionV1],
) -> Vec<EffectiveGoverningCeilingV1> {
    let mut by_subject = BTreeMap::<_, EffectiveGoverningCeilingV1>::new();
    for constraint in constraints {
        let key = (constraint.scope.clone(), constraint.subject.clone());
        by_subject
            .entry(key)
            .and_modify(|effective| {
                effective
                    .contributors
                    .insert(constraint.contributor.clone());
                effective.policies.insert(constraint.policy.clone());
                effective.policy = effective.policy.meet(&constraint.effective_policy);
            })
            .or_insert_with(|| EffectiveGoverningCeilingV1 {
                scope: constraint.scope.clone(),
                subject: constraint.subject.clone(),
                contributors: BTreeSet::from([constraint.contributor.clone()]),
                policies: BTreeSet::from([constraint.policy.clone()]),
                policy: constraint.effective_policy.clone(),
            });
    }
    by_subject.into_values().collect()
}

fn accumulate_obligations(
    obligations: &[ResolvedGoverningObligationV1],
) -> Vec<AccumulatedGoverningObligationV1> {
    let mut accumulated = BTreeMap::<_, AccumulatedGoverningObligationV1>::new();
    for declaration in obligations {
        let key = (
            declaration.scope.clone(),
            declaration.subject.clone(),
            declaration.obligation.clone(),
        );
        accumulated
            .entry(key)
            .and_modify(|entry| {
                entry.contributors.insert(declaration.contributor.clone());
            })
            .or_insert_with(|| AccumulatedGoverningObligationV1 {
                scope: declaration.scope.clone(),
                subject: declaration.subject.clone(),
                obligation: declaration.obligation.clone(),
                contributors: BTreeSet::from([declaration.contributor.clone()]),
            });
    }
    accumulated.into_values().collect()
}

fn apply_narrowing(
    ceilings: &mut Vec<EffectiveGoverningCeilingV1>,
    obligations: &mut Vec<AccumulatedGoverningObligationV1>,
    narrowing: &[ResolvedGoverningNarrowingV1],
) -> Result<(), GoverningAlgebraErrorV1> {
    let base = ceilings
        .iter()
        .cloned()
        .map(|ceiling| ((ceiling.scope.clone(), ceiling.subject.clone()), ceiling))
        .collect::<BTreeMap<_, _>>();
    let mut effective = base.clone();
    let mut accumulated = BTreeMap::<
        (PackageResourceUseV1, PackageResourceUseV1),
        BTreeMap<PackageResourceUseV1, BTreeSet<PackageResourceRefV1>>,
    >::new();
    for obligation in obligations.iter() {
        accumulated
            .entry((obligation.scope.clone(), obligation.subject.clone()))
            .or_default()
            .insert(
                obligation.obligation.clone(),
                obligation.contributors.clone(),
            );
    }
    let mut obligation_count = obligations.len();

    let edges_by_scope = narrowing.iter().fold(
        BTreeMap::<PackageResourceUseV1, Vec<ResolvedGoverningNarrowingV1>>::new(),
        |mut grouped, edge| {
            grouped
                .entry(edge.scope.clone())
                .or_default()
                .push(edge.clone());
            grouped
        },
    );
    for (scope, scope_edges) in edges_by_scope {
        let graph_edges = scope_edges
            .iter()
            .map(|edge| (edge.broader.clone(), edge.narrower.clone()))
            .collect::<Vec<_>>();
        let nodes = topological_order(&graph_edges)
            .expect("narrowing cycles were rejected before closure composition");
        let mut outgoing = BTreeMap::<_, Vec<_>>::new();
        for edge in scope_edges {
            outgoing.entry(edge.broader.clone()).or_default().push(edge);
        }

        for broader_subject in nodes {
            for edge in outgoing.get(&broader_subject).into_iter().flatten() {
                let broader_key = (edge.scope.clone(), edge.broader.clone());
                let narrower_key = (edge.scope.clone(), edge.narrower.clone());
                if let Some(broader) = effective.get(&broader_key).cloned() {
                    if let Some(own) = base.get(&narrower_key) {
                        let capabilities = own
                            .policy
                            .permitted_capabilities
                            .difference(&broader.policy.permitted_capabilities)
                            .cloned()
                            .collect::<BTreeSet<_>>();
                        let effects = own
                            .policy
                            .permitted_effects
                            .difference(&broader.policy.permitted_effects)
                            .copied()
                            .collect::<BTreeSet<_>>();
                        if !capabilities.is_empty() || !effects.is_empty() {
                            return Err(GoverningAlgebraErrorV1::NarrowingWidens {
                                edge: Box::new(edge.clone()),
                                capabilities,
                                effects,
                            });
                        }
                    }

                    let candidate = effective.get(&narrower_key).map_or_else(
                        || EffectiveGoverningCeilingV1 {
                            scope: edge.scope.clone(),
                            subject: edge.narrower.clone(),
                            contributors: broader.contributors.clone(),
                            policies: broader.policies.clone(),
                            policy: broader.policy.clone(),
                        },
                        |narrower| {
                            let mut composed = narrower.clone();
                            composed.policy = composed.policy.meet(&broader.policy);
                            composed
                                .contributors
                                .extend(broader.contributors.iter().cloned());
                            composed.policies.extend(broader.policies.iter().cloned());
                            composed
                        },
                    );
                    effective.insert(narrower_key.clone(), candidate);
                    if effective.len() > MAX_GOVERNING_RELATIONS {
                        return Err(GoverningAlgebraErrorV1::RelationTooLarge {
                            relation: "effective-ceiling",
                            actual: effective.len(),
                            maximum: MAX_GOVERNING_RELATIONS,
                        });
                    }
                }

                let broader_obligations = accumulated
                    .get(&(scope.clone(), edge.broader.clone()))
                    .cloned()
                    .unwrap_or_default();
                let narrower_obligations = accumulated.entry(narrower_key).or_default();
                for (obligation, contributors) in broader_obligations {
                    match narrower_obligations.entry(obligation) {
                        std::collections::btree_map::Entry::Occupied(mut entry) => {
                            entry.get_mut().extend(contributors);
                        }
                        std::collections::btree_map::Entry::Vacant(entry) => {
                            entry.insert(contributors);
                            obligation_count += 1;
                            if obligation_count > MAX_GOVERNING_RELATIONS {
                                return Err(GoverningAlgebraErrorV1::RelationTooLarge {
                                    relation: "accumulated-obligation",
                                    actual: obligation_count,
                                    maximum: MAX_GOVERNING_RELATIONS,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    *ceilings = effective.into_values().collect();
    *obligations = accumulated
        .into_iter()
        .flat_map(|((scope, subject), obligations)| {
            obligations
                .into_iter()
                .map(
                    move |(obligation, contributors)| AccumulatedGoverningObligationV1 {
                        scope: scope.clone(),
                        subject: subject.clone(),
                        obligation,
                        contributors,
                    },
                )
        })
        .collect();
    Ok(())
}

fn topological_order(
    edges: &[(PackageResourceUseV1, PackageResourceUseV1)],
) -> Option<Vec<PackageResourceUseV1>> {
    let mut adjacency = BTreeMap::<_, BTreeSet<_>>::new();
    let mut indegree = BTreeMap::<_, usize>::new();
    for (before, after) in edges {
        indegree.entry(before.clone()).or_default();
        indegree.entry(after.clone()).or_default();
        if adjacency
            .entry(before.clone())
            .or_default()
            .insert(after.clone())
        {
            *indegree.entry(after.clone()).or_default() += 1;
        }
    }
    let mut frontier = indegree
        .iter()
        .filter_map(|(node, degree)| (*degree == 0).then_some(node.clone()))
        .collect::<BTreeSet<_>>();
    let mut ordered = Vec::with_capacity(indegree.len());
    while let Some(node) = frontier.pop_first() {
        if let Some(dependents) = adjacency.get(&node) {
            for dependent in dependents {
                let degree = indegree
                    .get_mut(dependent)
                    .expect("edge target must have an indegree entry");
                *degree -= 1;
                if *degree == 0 {
                    frontier.insert(dependent.clone());
                }
            }
        }
        ordered.push(node);
    }
    (ordered.len() == indegree.len()).then_some(ordered)
}

fn validate_exclusive_authority(
    claims: &[AdmittedExclusiveAuthorityClaimV1],
) -> Result<(), GoverningAlgebraErrorV1> {
    let mut claimants = BTreeMap::<_, BTreeSet<_>>::new();
    for claim in claims {
        claimants
            .entry((claim.scope.clone(), claim.authority.clone()))
            .or_default()
            .insert(claim.claimant.clone());
    }
    for ((scope, authority), claimants) in claimants {
        if claimants.len() > 1 {
            return Err(GoverningAlgebraErrorV1::ExclusiveAuthorityConflict {
                scope,
                authority: Box::new(authority),
                claimants,
            });
        }
    }
    Ok(())
}

fn validate_effective_narrowing(
    ceilings: &[EffectiveGoverningCeilingV1],
    obligations: &[AccumulatedGoverningObligationV1],
    narrowing: &[ResolvedGoverningNarrowingV1],
) -> Result<(), GoverningAlgebraErrorV1> {
    let ceilings = ceilings
        .iter()
        .map(|ceiling| ((&ceiling.scope, &ceiling.subject), ceiling))
        .collect::<BTreeMap<_, _>>();
    for edge in narrowing {
        if let Some(broader) = ceilings.get(&(&edge.scope, &edge.broader)) {
            let Some(narrower) = ceilings.get(&(&edge.scope, &edge.narrower)) else {
                return Err(GoverningAlgebraErrorV1::IncompleteNarrowingClosure {
                    edge: Box::new(edge.clone()),
                });
            };
            let widens_capabilities = !narrower
                .policy
                .permitted_capabilities
                .is_subset(&broader.policy.permitted_capabilities);
            let widens_effects = !narrower
                .policy
                .permitted_effects
                .is_subset(&broader.policy.permitted_effects);
            let loses_provenance = !narrower.contributors.is_superset(&broader.contributors)
                || !narrower.policies.is_superset(&broader.policies);
            if widens_capabilities || widens_effects || loses_provenance {
                return Err(GoverningAlgebraErrorV1::IncompleteNarrowingClosure {
                    edge: Box::new(edge.clone()),
                });
            }
        }
        for obligation in obligations.iter().filter(|obligation| {
            obligation.scope == edge.scope && obligation.subject == edge.broader
        }) {
            let inherited = obligations.iter().find(|candidate| {
                candidate.scope == edge.scope
                    && candidate.subject == edge.narrower
                    && candidate.obligation == obligation.obligation
            });
            if inherited.is_none_or(|inherited| {
                !inherited.contributors.is_superset(&obligation.contributors)
            }) {
                return Err(GoverningAlgebraErrorV1::IncompleteNarrowingClosure {
                    edge: Box::new(edge.clone()),
                });
            }
        }
    }
    Ok(())
}

fn validate_rejected_cycles(
    narrowing: &[ResolvedGoverningNarrowingV1],
    order: &[AdmittedScopedOrderRelationV1],
) -> Result<(), GoverningAlgebraErrorV1> {
    let narrowing_by_scope = group_edges(narrowing.iter().map(|edge| {
        (
            edge.scope.clone(),
            edge.broader.clone(),
            edge.narrower.clone(),
        )
    }));
    for (scope, edges) in narrowing_by_scope {
        if let Some(members) = graph_cycle_witness(&edges) {
            let cycle_pairs = members
                .windows(2)
                .map(|pair| (&pair[0], &pair[1]))
                .collect::<BTreeSet<_>>();
            let edges = narrowing
                .iter()
                .filter(|edge| {
                    edge.scope == scope && cycle_pairs.contains(&(&edge.broader, &edge.narrower))
                })
                .cloned()
                .collect();
            return Err(GoverningAlgebraErrorV1::NarrowingCycle {
                scope,
                members,
                edges,
            });
        }
    }

    let mut partial_orders = BTreeMap::<_, Vec<_>>::new();
    for edge in order {
        partial_orders
            .entry((edge.scope.clone(), edge.purpose))
            .or_default()
            .push((edge.before.clone(), edge.after.clone()));
    }
    for ((scope, purpose), edges) in partial_orders {
        if let Some(members) = graph_cycle_witness(&edges) {
            let cycle_pairs = members
                .windows(2)
                .map(|pair| (&pair[0], &pair[1]))
                .collect::<BTreeSet<_>>();
            let edges = order
                .iter()
                .filter(|edge| {
                    edge.scope == scope
                        && edge.purpose == purpose
                        && cycle_pairs.contains(&(&edge.before, &edge.after))
                })
                .cloned()
                .collect();
            return Err(GoverningAlgebraErrorV1::PartialOrderCycle {
                scope,
                purpose,
                members,
                edges,
            });
        }
    }
    Ok(())
}

fn group_edges(
    edges: impl IntoIterator<
        Item = (
            PackageResourceUseV1,
            PackageResourceUseV1,
            PackageResourceUseV1,
        ),
    >,
) -> BTreeMap<PackageResourceUseV1, Vec<(PackageResourceUseV1, PackageResourceUseV1)>> {
    let mut grouped = BTreeMap::<_, Vec<_>>::new();
    for (scope, before, after) in edges {
        grouped.entry(scope).or_default().push((before, after));
    }
    grouped
}

fn graph_cycle_witness(
    edges: &[(PackageResourceUseV1, PackageResourceUseV1)],
) -> Option<Vec<PackageResourceUseV1>> {
    let mut adjacency = BTreeMap::<_, BTreeSet<_>>::new();
    for (before, after) in edges {
        adjacency
            .entry(before.clone())
            .or_default()
            .insert(after.clone());
        adjacency.entry(after.clone()).or_default();
    }

    fn visit(
        node: &PackageResourceUseV1,
        adjacency: &BTreeMap<PackageResourceUseV1, BTreeSet<PackageResourceUseV1>>,
        state: &mut BTreeMap<PackageResourceUseV1, u8>,
        path: &mut Vec<PackageResourceUseV1>,
    ) -> Option<Vec<PackageResourceUseV1>> {
        state.insert(node.clone(), 1);
        path.push(node.clone());
        for next in adjacency.get(node).into_iter().flatten() {
            match state.get(next).copied().unwrap_or_default() {
                0 => {
                    if let Some(cycle) = visit(next, adjacency, state, path) {
                        return Some(cycle);
                    }
                }
                1 => {
                    let start = path
                        .iter()
                        .position(|member| member == next)
                        .expect("active graph node must be on the DFS path");
                    let mut cycle = path[start..].to_vec();
                    cycle.push(next.clone());
                    return Some(cycle);
                }
                _ => {}
            }
        }
        path.pop();
        state.insert(node.clone(), 2);
        None
    }

    let mut state = BTreeMap::new();
    let mut path = Vec::new();
    for node in adjacency.keys() {
        if state.get(node).copied().unwrap_or_default() == 0 {
            if let Some(cycle) = visit(node, &adjacency, &mut state, &mut path) {
                return Some(cycle);
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "../../tests/internal/records/order.rs"]
mod tests;
