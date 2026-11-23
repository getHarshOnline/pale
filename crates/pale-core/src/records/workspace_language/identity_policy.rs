//! Portable identities and policy algebra for workspace-language contracts.
//!
//! These declarations describe desired semantic ceilings and logical owner
//! references. They neither prove native authority nor authorize an effect.

use super::*;

portable_id!(
    #[doc = "Principal-owned portable workspace identity."]
    WorkspaceId,
    "workspace id"
);
portable_id!(
    #[doc = "Independently governed module identity."]
    WorkspaceModuleId,
    "module id"
);
portable_id!(
    #[doc = "Immutable release identity within one module."]
    WorkspaceReleaseId,
    "release id"
);
portable_id!(
    #[doc = "Finer-grained source-component identity."]
    WorkspaceComponentId,
    "component id"
);
portable_id!(
    #[doc = "Namespace owned or imported by a module."]
    WorkspaceNamespaceId,
    "namespace id"
);
portable_id!(
    #[doc = "Declaration identity within a namespace."]
    WorkspaceDeclarationId,
    "declaration id"
);
portable_id!(
    #[doc = "Named export surface of one release."]
    WorkspaceExportId,
    "export id"
);
portable_id!(
    #[doc = "Logical resource identity; never a machine path."]
    WorkspaceResourceId,
    "resource id"
);
portable_id!(
    #[doc = "Stable publisher identity."]
    WorkspacePublisherId,
    "publisher id"
);
portable_id!(
    #[doc = "Publisher root or delegated release-key identity."]
    WorkspacePublisherKeyId,
    "publisher key id"
);
portable_id!(
    #[doc = "Stable recovery authority identity."]
    WorkspaceRecoveryAuthorityId,
    "recovery authority id"
);
portable_id!(
    #[doc = "Named policy obligation."]
    WorkspaceObligationId,
    "obligation id"
);
portable_id!(
    #[doc = "Exclusive authority scope identity."]
    WorkspaceAuthorityScopeId,
    "authority scope id"
);
portable_id!(
    #[doc = "Owner-defined permitted-use right identity."]
    WorkspaceRightId,
    "workspace right id"
);
portable_id!(
    #[doc = "Owner-defined controller scope identity."]
    WorkspaceControllerScopeId,
    "workspace controller scope id"
);
portable_id!(
    #[doc = "Logical source root retained by a workspace derivation receipt."]
    WorkspaceDerivationSourceId,
    "derivation source id"
);
wire_enum!(
    /// Visibility of one module export.
    WorkspaceVisibilityV1 {
        /// Any admitted importer may resolve the declared surface.
        Public => "public",
        /// Resolution requires a separately verified protected-delivery grant.
        Protected => "protected",
        /// The surface is confined to its declaring module.
        Private => "private"
    }
);

/// Bounded set of rights admitted by one policy ceiling.
///
/// The meet is set intersection. An empty set admits no right; it is the
/// narrowest value, never a wildcard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRightsCeilingV1 {
    /// Canonically ordered, duplicate-free admitted rights.
    #[schemars(schema_with = "workspace_rights_schema")]
    pub permitted: Vec<WorkspaceRightId>,
}

/// Bounded set of logical controller scopes admitted by one policy ceiling.
///
/// The meet is set intersection. An empty set admits no controller scope and
/// carries no implicit fallback to a native owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceControllerCeilingV1 {
    /// Canonically ordered, duplicate-free controller identifiers.
    #[schemars(schema_with = "workspace_controller_scopes_schema")]
    pub permitted: Vec<WorkspaceControllerScopeId>,
}

/// Bounded set of logical authority scopes admitted by one policy ceiling.
///
/// The meet is set intersection. An empty set admits no authority scope and is
/// not evidence that an unnamed authority may act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceAuthorityCeilingV1 {
    /// Canonically ordered, duplicate-free authority scopes.
    #[schemars(schema_with = "workspace_authority_scopes_schema")]
    pub permitted: Vec<WorkspaceAuthorityScopeId>,
}

fn workspace_rights_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    bounded_array_schema::<WorkspaceRightId>(generator, MAX_WORKSPACE_POLICY_MEMBERS, 0)
}

fn workspace_controller_scopes_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    bounded_array_schema::<WorkspaceControllerScopeId>(generator, MAX_WORKSPACE_POLICY_MEMBERS, 0)
}

fn workspace_authority_scopes_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    bounded_array_schema::<WorkspaceAuthorityScopeId>(generator, MAX_WORKSPACE_POLICY_MEMBERS, 0)
}

macro_rules! policy_set_lattice {
    ($type:ty) => {
        impl $type {
            /// Return the canonical intersection admitted by both operands.
            #[must_use]
            pub fn meet(&self, other: &Self) -> Self {
                let left = self
                    .permitted
                    .iter()
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>();
                let right = other
                    .permitted
                    .iter()
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>();
                Self {
                    permitted: left.intersection(&right).cloned().collect(),
                }
            }

            /// Report whether every member admitted by `self` is admitted by `other`.
            #[must_use]
            pub fn is_narrower_or_equal(&self, other: &Self) -> bool {
                let other = other
                    .permitted
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>();
                self.permitted.iter().all(|item| other.contains(item))
            }
        }
    };
}

policy_set_lattice!(WorkspaceRightsCeilingV1);
policy_set_lattice!(WorkspaceControllerCeilingV1);
policy_set_lattice!(WorkspaceAuthorityCeilingV1);

impl WorkspaceVisibilityV1 {
    /// Return the stricter visibility admitted by both operands.
    ///
    /// The order rendered on [`WorkspacePolicyCeilingV1`] makes this meet
    /// preserve or narrow visibility, never widen it.
    #[must_use]
    pub const fn meet(self, other: Self) -> Self {
        if self.rank() <= other.rank() {
            self
        } else {
            other
        }
    }

    /// Report whether `self` is equal to or stricter than `other`.
    #[must_use]
    pub const fn is_narrower_or_equal(self, other: Self) -> bool {
        self.rank() <= other.rank()
    }

    const fn rank(self) -> u8 {
        match self {
            Self::Private => 0,
            Self::Protected => 1,
            Self::Public => 2,
        }
    }
}

wire_enum!(
    /// How an authorized declaration surface may be delivered.
    WorkspaceDeliveryV1 {
        /// Only the declared interface is delivered.
        Interface => "interface",
        /// Source may be delivered under the governing ceiling.
        Source => "source"
    }
);

impl WorkspaceDeliveryV1 {
    /// Report whether `self` requests no more disclosure than `other`.
    ///
    /// Interface delivery is strictly narrower than source delivery. This
    /// comparison does not establish that either delivery is authorized.
    #[must_use]
    pub const fn is_narrower_or_equal(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Interface, Self::Interface | Self::Source) | (Self::Source, Self::Source)
        )
    }
}

wire_enum!(
    /// Whether an already admitted declaration may continue to another hop.
    WorkspacePropagationV1 {
        /// The exact authorized closure may continue under equal or narrower policy.
        Transitive => "transitive",
        /// Current local use remains lawful but the next hop is denied.
        Prevent => "prevent"
    }
);

impl WorkspacePropagationV1 {
    /// Combine two propagation rules without permitting a denied next hop.
    ///
    /// `prevent` is absorbing: once any admitted path forbids onward
    /// propagation, later composition cannot restore `transitive` delivery.
    #[must_use]
    pub const fn meet(self, other: Self) -> Self {
        if matches!(self, Self::Prevent) || matches!(other, Self::Prevent) {
            Self::Prevent
        } else {
            Self::Transitive
        }
    }

    /// Report whether `self` is equal to or stricter than `other`.
    #[must_use]
    pub const fn is_narrower_or_equal(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Prevent, Self::Prevent | Self::Transitive)
                | (Self::Transitive, Self::Transitive)
        )
    }
}

wire_enum!(
    /// Consumer policy for moving to a successor release.
    WorkspaceAdoptionPolicyV1 {
        /// Every successor requires explicit consumer adoption.
        Manual => "manual"
    }
);

wire_enum!(
    /// Claimed publisher release state, independently recorded from consumer adoption.
    ///
    /// This wire value is not proof. Semantic admission must verify the exact
    /// publisher evidence for a selected [`WorkspaceResolvedReleaseV1`] before
    /// its [`WorkspaceUniverseLockV1`] coordinate can be admitted.
    WorkspaceReleaseStateV1 {
        /// The resolution record claims an unpublished candidate.
        Candidate => "candidate",
        /// The resolution record claims publication; admission still requires proof.
        Published => "published",
        /// The resolution record claims that the publisher revoked this coordinate.
        Revoked => "revoked"
    }
);

wire_enum!(
    /// Dependency kind used when checking declaration cycles.
    DeclarationDependencyKindV1 {
        /// Positive monotone declaration dependency.
        Positive => "positive",
        /// Negation through a cycle is not admissible.
        Negation => "negation",
        /// Native or machine effect dependency is not admissible in a declaration SCC.
        Effect => "effect",
        /// Arbitrary callback dependency is not admissible.
        Callback => "callback",
        /// Unbounded generative dependency is not admissible.
        UnboundedGeneration => "unbounded_generation"
    }
);

wire_enum!(
    /// Maximum disclosure admitted for a declaration surface.
    DisclosureCeilingV1 {
        /// Private use inside the declaring module only.
        Private => "private",
        /// Protected delivery under a verified grant.
        Protected => "protected",
        /// Public interface delivery.
        Public => "public"
    }
);

wire_enum!(
    /// Maximum effect class admitted by a portable declaration.
    WorkspaceEffectCeilingV1 {
        /// No effect-bearing operation is admitted.
        None => "none",
        /// Read-only interaction may be admitted later by the applicable owner.
        Read => "read",
        /// Mutation may be admitted later by the applicable owner.
        Mutate => "mutate"
    }
);

wire_enum!(
    /// Typed target of a logical resource selection.
    WorkspaceResourceKindV1 {
        /// Principal-owned logical resource.
        Principal => "principal",
        /// Independently constituted organization reference.
        Organization => "organization",
        /// Native application contract reference.
        NativeApplication => "native_application",
        /// Participant or responsibility-provider reference.
        Participant => "participant",
        /// Pale Package selection.
        Package => "package",
        /// Owner-attested normative Pack reference.
        NormativePack => "normative_pack"
    }
);

/// Declared publisher and key identifiers; this reference is not authorization proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspacePublisherRefV1 {
    /// Stable publisher identity.
    pub publisher_id: WorkspacePublisherId,
    /// Stable publisher root-key reference; no key material is carried.
    pub root_key_id: WorkspacePublisherKeyId,
    /// Recovery authority used by the publisher's own procedure.
    pub recovery_authority_id: WorkspaceRecoveryAuthorityId,
    /// Declared delegated release-key identifier; no authorization proof is carried.
    pub delegated_release_key_id: WorkspacePublisherKeyId,
}

/// Seven-dimensional policy ceiling; import edges may only make it stricter.
///
/// In the rendered componentwise order, [`Self::meet`] computes the greatest
/// lower bound. Scalar axes use minimum or conjunction and owner-defined set
/// axes use intersection. Thus the operator is commutative, associative, and
/// idempotent, and its result is below both inputs.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/workspace-policy-meet.display.mathml"
))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspacePolicyCeilingV1 {
    /// Maximum disclosure.
    pub disclosure: DisclosureCeilingV1,
    /// Maximum effect class.
    pub effect: WorkspaceEffectCeilingV1,
    /// Whether model use is permitted at all.
    pub model_use_permitted: bool,
    /// Maximum retention in seconds; zero means no retention.
    #[serde(deserialize_with = "deserialize_json_safe_u64")]
    #[schemars(schema_with = "crate::canon::json_safe_u64_schema")]
    pub retention_seconds: u64,
    /// Independently admitted use rights.
    pub rights: WorkspaceRightsCeilingV1,
    /// Independently admitted logical controllers; this is not controller proof.
    pub controllers: WorkspaceControllerCeilingV1,
    /// Independently admitted logical authorities; this is not authority proof.
    pub authorities: WorkspaceAuthorityCeilingV1,
}

/// A logical set inside a [`WorkspacePolicyCeilingV1`] is not bounded and canonical.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidWorkspacePolicyCeilingV1 {
    /// One policy-set axis exceeds the shared initial-v1 member ceiling.
    #[error("workspace policy axis `{axis}` has {observed} members; maximum is {limit}")]
    Limit {
        /// Independently ordered policy axis.
        axis: WorkspaceCeilingSetAxisV1,
        /// Number of supplied members.
        observed: usize,
        /// Shared maximum for one policy-set axis.
        limit: usize,
    },
    /// One policy-set axis repeats a logical member.
    #[error("workspace policy axis `{axis}` repeats member `{member}`")]
    Duplicate {
        /// Independently ordered policy axis.
        axis: WorkspaceCeilingSetAxisV1,
        /// Repeated canonical member identity.
        member: String,
    },
}

impl WorkspacePolicyCeilingV1 {
    /// Validate and canonically order every set-valued policy axis.
    ///
    /// Rights, controllers, and authorities share the single
    /// [`MAX_WORKSPACE_POLICY_MEMBERS`] bound. Sorting is deterministic and a
    /// repeated member is rejected rather than silently discarded.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspacePolicyCeilingV1`] when an axis exceeds the
    /// shared bound or repeats a member.
    pub fn canonicalize_policy_sets(&mut self) -> Result<(), InvalidWorkspacePolicyCeilingV1> {
        canonicalize_workspace_ceiling_set(
            &mut self.rights.permitted,
            WorkspaceCeilingSetAxisV1::Rights,
        )?;
        canonicalize_workspace_ceiling_set(
            &mut self.controllers.permitted,
            WorkspaceCeilingSetAxisV1::Controller,
        )?;
        canonicalize_workspace_ceiling_set(
            &mut self.authorities.permitted,
            WorkspaceCeilingSetAxisV1::Authority,
        )
    }

    /// Intersect two ceilings into the greatest policy admitted by both.
    ///
    /// Disclosure, effect, model use, retention, rights, controller scopes, and
    /// authority scopes remain independent. Obligations and exclusive-authority
    /// claims combine separately on their own owning types.
    #[must_use]
    pub fn meet(&self, other: &Self) -> Self {
        Self {
            disclosure: narrower_disclosure(self.disclosure, other.disclosure),
            effect: narrower_effect(self.effect, other.effect),
            model_use_permitted: self.model_use_permitted && other.model_use_permitted,
            retention_seconds: self.retention_seconds.min(other.retention_seconds),
            rights: self.rights.meet(&other.rights),
            controllers: self.controllers.meet(&other.controllers),
            authorities: self.authorities.meet(&other.authorities),
        }
    }

    /// Report whether `self` admits no capability wider than `other`.
    #[must_use]
    pub fn is_narrower_or_equal(&self, other: &Self) -> bool {
        disclosure_rank(self.disclosure) <= disclosure_rank(other.disclosure)
            && effect_rank(self.effect) <= effect_rank(other.effect)
            && (!self.model_use_permitted || other.model_use_permitted)
            && self.retention_seconds <= other.retention_seconds
            && self.rights.is_narrower_or_equal(&other.rights)
            && self.controllers.is_narrower_or_equal(&other.controllers)
            && self.authorities.is_narrower_or_equal(&other.authorities)
    }

    /// Name every independent dimension on which `self` widens `other`.
    #[must_use]
    pub fn widening_dimensions(&self, other: &Self) -> Vec<WorkspacePolicyAxisV1> {
        let mut axes = Vec::new();
        if disclosure_rank(self.disclosure) > disclosure_rank(other.disclosure) {
            axes.push(WorkspacePolicyAxisV1::Disclosure);
        }
        if effect_rank(self.effect) > effect_rank(other.effect) {
            axes.push(WorkspacePolicyAxisV1::Effect);
        }
        if self.model_use_permitted && !other.model_use_permitted {
            axes.push(WorkspacePolicyAxisV1::ModelUse);
        }
        if self.retention_seconds > other.retention_seconds {
            axes.push(WorkspacePolicyAxisV1::Retention);
        }
        if !self.rights.is_narrower_or_equal(&other.rights) {
            axes.push(WorkspacePolicyAxisV1::Rights);
        }
        if !self.controllers.is_narrower_or_equal(&other.controllers) {
            axes.push(WorkspacePolicyAxisV1::Controller);
        }
        if !self.authorities.is_narrower_or_equal(&other.authorities) {
            axes.push(WorkspacePolicyAxisV1::Authority);
        }
        axes
    }
}

/// Bound and sort one policy-set axis without erasing a repeated member.
/// One bounded logical-set family carried by a workspace ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceCeilingSetAxisV1 {
    /// Explicit resource selectors surrounding a policy ceiling.
    Resource,
    /// Permitted-use rights.
    Rights,
    /// Logical controller scopes.
    Controller,
    /// Logical authority scopes.
    Authority,
}

impl WorkspaceCeilingSetAxisV1 {
    /// Return the stable diagnostic spelling for this set family.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Resource => "resource",
            Self::Rights => "rights",
            Self::Controller => "controller",
            Self::Authority => "authority",
        }
    }
}

impl std::fmt::Display for WorkspaceCeilingSetAxisV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Bound and canonically order one set-valued workspace ceiling axis.
///
/// The function is the single source of truth for resource, right, controller,
/// and authority sets carried by workspace, responsibility, and organizational
/// records. It rejects duplicates instead of erasing them.
///
/// # Errors
///
/// Returns [`InvalidWorkspacePolicyCeilingV1`] when `members` exceeds
/// [`MAX_WORKSPACE_POLICY_MEMBERS`] or contains a repeated member.
pub fn canonicalize_workspace_ceiling_set<T>(
    members: &mut [T],
    axis: WorkspaceCeilingSetAxisV1,
) -> Result<(), InvalidWorkspacePolicyCeilingV1>
where
    T: Ord + std::fmt::Display,
{
    if members.len() > MAX_WORKSPACE_POLICY_MEMBERS {
        return Err(InvalidWorkspacePolicyCeilingV1::Limit {
            axis,
            observed: members.len(),
            limit: MAX_WORKSPACE_POLICY_MEMBERS,
        });
    }
    members.sort();
    if let Some(pair) = members.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(InvalidWorkspacePolicyCeilingV1::Duplicate {
            axis,
            member: pair[0].to_string(),
        });
    }
    Ok(())
}

wire_enum!(
    /// One independently ordered dimension of a workspace policy ceiling.
    WorkspacePolicyAxisV1 {
        /// Disclosure ceiling.
        Disclosure => "disclosure",
        /// Effect ceiling.
        Effect => "effect",
        /// Model-use permission.
        ModelUse => "model_use",
        /// Retention duration.
        Retention => "retention",
        /// Use-right set.
        Rights => "rights",
        /// Controller set.
        Controller => "controller",
        /// Authority set.
        Authority => "authority"
    }
);

const fn disclosure_rank(value: DisclosureCeilingV1) -> u8 {
    match value {
        DisclosureCeilingV1::Private => 0,
        DisclosureCeilingV1::Protected => 1,
        DisclosureCeilingV1::Public => 2,
    }
}

const fn effect_rank(value: WorkspaceEffectCeilingV1) -> u8 {
    match value {
        WorkspaceEffectCeilingV1::None => 0,
        WorkspaceEffectCeilingV1::Read => 1,
        WorkspaceEffectCeilingV1::Mutate => 2,
    }
}

const fn narrower_disclosure(
    left: DisclosureCeilingV1,
    right: DisclosureCeilingV1,
) -> DisclosureCeilingV1 {
    if disclosure_rank(left) <= disclosure_rank(right) {
        left
    } else {
        right
    }
}

const fn narrower_effect(
    left: WorkspaceEffectCeilingV1,
    right: WorkspaceEffectCeilingV1,
) -> WorkspaceEffectCeilingV1 {
    if effect_rank(left) <= effect_rank(right) {
        left
    } else {
        right
    }
}

fn deserialize_json_safe_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = u64::deserialize(deserializer)?;
    if value > crate::canon::JSON_SAFE_INTEGER_MAX {
        return Err(serde::de::Error::custom(
            "integer exceeds the canonical JSON exactness bound",
        ));
    }
    Ok(value)
}

impl WorkspaceObligationId {
    /// Accumulate every obligation from two canonical or noncanonical inputs.
    ///
    /// The returned set is sorted and duplicate-free. Accumulation is monotone:
    /// neither input can remove an obligation contributed by the other.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspacePolicyCombinationError::ObligationLimit`] when an
    /// input or the effective union exceeds [`MAX_WORKSPACE_OBLIGATIONS`].
    pub fn try_union_sets(
        left: &[Self],
        right: &[Self],
    ) -> Result<Vec<Self>, WorkspacePolicyCombinationError> {
        for observed in [left.len(), right.len()] {
            if observed > MAX_WORKSPACE_OBLIGATIONS {
                return Err(WorkspacePolicyCombinationError::ObligationLimit {
                    observed,
                    limit: MAX_WORKSPACE_OBLIGATIONS,
                });
            }
        }
        let mut obligations = left.iter().chain(right).cloned().collect::<Vec<_>>();
        obligations.sort();
        obligations.dedup();
        if obligations.len() > MAX_WORKSPACE_OBLIGATIONS {
            return Err(WorkspacePolicyCombinationError::ObligationLimit {
                observed: obligations.len(),
                limit: MAX_WORKSPACE_OBLIGATIONS,
            });
        }
        Ok(obligations)
    }
}

/// One authority scope that exactly one declared authority may own.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceExclusiveAuthorityClaimV1 {
    /// Exclusive scope.
    pub scope: WorkspaceAuthorityScopeId,
    /// Declared logical resolver identifier; it is not native authority proof.
    pub authority_resolver: WorkspaceResourceId,
}

impl WorkspaceExclusiveAuthorityClaimV1 {
    /// Combine two claim sets without silently choosing an authority.
    ///
    /// Identical claims collapse to one canonical entry. Two different logical
    /// resolvers for one scope remain a static declarative conflict.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspacePolicyCombinationError::AuthorityConflict`] when one
    /// scope names two distinct resolvers, or
    /// [`WorkspacePolicyCombinationError::AuthorityLimit`] when an input or the
    /// effective set exceeds [`MAX_WORKSPACE_AUTHORITY_CLAIMS`]. This does not
    /// establish native authority for any resolver.
    pub fn try_combine_sets(
        left: &[Self],
        right: &[Self],
    ) -> Result<Vec<Self>, WorkspacePolicyCombinationError> {
        for observed in [left.len(), right.len()] {
            if observed > MAX_WORKSPACE_AUTHORITY_CLAIMS {
                return Err(WorkspacePolicyCombinationError::AuthorityLimit {
                    observed,
                    limit: MAX_WORKSPACE_AUTHORITY_CLAIMS,
                });
            }
        }
        let mut claims = left.iter().chain(right).cloned().collect::<Vec<_>>();
        claims.sort();
        claims.dedup();
        if claims.len() > MAX_WORKSPACE_AUTHORITY_CLAIMS {
            return Err(WorkspacePolicyCombinationError::AuthorityLimit {
                observed: claims.len(),
                limit: MAX_WORKSPACE_AUTHORITY_CLAIMS,
            });
        }
        for pair in claims.windows(2) {
            if pair[0].scope == pair[1].scope {
                return Err(WorkspacePolicyCombinationError::AuthorityConflict {
                    scope: pair[0].scope.clone(),
                    left: pair[0].authority_resolver.clone(),
                    right: pair[1].authority_resolver.clone(),
                });
            }
        }
        Ok(claims)
    }
}

/// A bounded workspace-policy accumulation cannot produce one lawful set.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkspacePolicyCombinationError {
    /// The effective obligation set exceeds its semantic-surface bound.
    #[error("workspace obligation count {observed} exceeds {limit}")]
    ObligationLimit {
        /// Observed input or effective-set cardinality.
        observed: usize,
        /// Maximum admitted cardinality.
        limit: usize,
    },
    /// The effective exclusive-authority set exceeds its semantic-surface bound.
    #[error("workspace exclusive-authority count {observed} exceeds {limit}")]
    AuthorityLimit {
        /// Observed input or effective-set cardinality.
        observed: usize,
        /// Maximum admitted cardinality.
        limit: usize,
    },
    /// Two declarative resolvers claim one exclusive authority scope.
    #[error("workspace authority scope `{scope}` names both `{left}` and `{right}`")]
    AuthorityConflict {
        /// Scope that cannot be resolved declaratively.
        scope: WorkspaceAuthorityScopeId,
        /// First resolver in canonical order.
        left: WorkspaceResourceId,
        /// Second resolver in canonical order.
        right: WorkspaceResourceId,
    },
}
