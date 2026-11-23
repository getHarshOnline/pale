//! Neutral heterogeneous organizational records.
//!
//! The grammar represents already owner-constituted scopes, structures,
//! bindings, relations, services, and projections. Labels and topology carry
//! no authority, and every structural family is optional.

use super::*;

/// Registry identity of the initial organizational-translation schema.
pub const ORGANIZATIONAL_TRANSLATION_V1_SCHEMA_ID: &str = "organizational-translation.v1";
/// Wire identity carried by every initial organizational-translation record.
pub const ORGANIZATIONAL_TRANSLATION_V1_SCHEMA: &str = "pale/organizational-translation.v1";
/// Wire version of [`OrganizationalTranslationV1`].
pub const ORGANIZATIONAL_TRANSLATION_V1_SCHEMA_VERSION: u32 = 1;

portable_id!(
    #[doc = "Stable logical-scope identity within an organizational namespace."]
    LogicalScopeId,
    "logical scope id"
);
portable_id!(
    #[doc = "Stable logical-organization identity within a namespace."]
    LogicalOrganizationId,
    "logical organization id"
);
portable_id!(
    #[doc = "Stable optional group identity within a namespace."]
    OrganizationalGroupId,
    "organizational group id"
);
portable_id!(
    #[doc = "Stable optional discipline-tree identity within a namespace."]
    DisciplineTreeId,
    "discipline tree id"
);
portable_id!(
    #[doc = "Stable role identity within a namespace."]
    OrganizationalRoleId,
    "organizational role id"
);
portable_id!(
    #[doc = "Stable occupant identity independent of a role."]
    OrganizationalOccupantId,
    "organizational occupant id"
);
portable_id!(
    #[doc = "Stable provider identity independent of a role or occupant."]
    OrganizationalProviderId,
    "organizational provider id"
);
portable_id!(
    #[doc = "Stable topic identity for bounded leadership participation."]
    LeadershipTopicId,
    "leadership topic id"
);
portable_id!(
    #[doc = "Stable typed organizational-relation identity."]
    OrganizationalRelationId,
    "organizational relation id"
);
portable_id!(
    #[doc = "Stable directional-service contract identity."]
    DirectionalServiceContractId,
    "directional service contract id"
);
portable_id!(
    #[doc = "Stable account identity used by a service contract."]
    ServiceAccountId,
    "service account id"
);
portable_id!(
    #[doc = "Stable mandate identity used by a service contract."]
    ServiceMandateId,
    "service mandate id"
);
portable_id!(
    #[doc = "Stable steward identity used by a service contract."]
    ServiceStewardId,
    "service steward id"
);
portable_id!(
    #[doc = "Stable pod identity used by a service contract."]
    ServicePodId,
    "service pod id"
);
portable_id!(
    #[doc = "Stable object-class identity selected by private configuration."]
    OrganizationalObjectClassId,
    "organizational object class id"
);
portable_id!(
    #[doc = "Stable object identity selected by private configuration."]
    OrganizationalObjectId,
    "organizational object id"
);
portable_id!(
    #[doc = "Stable role-class identity used by population rules."]
    OrganizationalRoleClassId,
    "organizational role class id"
);
portable_id!(
    #[doc = "Stable group-lens authority-profile identity."]
    GroupLensProfileId,
    "group lens profile id"
);
portable_id!(
    #[doc = "Stable lens identity whose meaning is selected by an organization."]
    OrganizationalLensId,
    "organizational lens id"
);
portable_id!(
    #[doc = "Stable condition identity for a bounded block or escalation."]
    OrganizationalConditionId,
    "organizational condition id"
);
portable_id!(
    #[doc = "Stable native-projection identity for organizational semantics."]
    OrganizationalProjectionId,
    "organizational projection id"
);
portable_id!(
    #[doc = "Stable full-matrix inspection-contract identity."]
    OrganizationalInspectionId,
    "organizational inspection id"
);
portable_id!(
    #[doc = "Canonical provider-native communication-actor handle identity."]
    CommunicationActorHandleId,
    "communication actor handle id"
);

/// Maximum records of any one organizational family in a source document.
pub const MAX_ORGANIZATIONAL_RECORDS: usize = 1_024;
/// Maximum typed relations in one organizational source document.
pub const MAX_ORGANIZATIONAL_RELATIONS: usize = 4_096;
/// Maximum members in one nested organizational collection.
pub const MAX_ORGANIZATIONAL_NESTED_MEMBERS: usize = 1_024;
/// Maximum rights carried by one group-lens profile.
pub const MAX_GROUP_LENS_RIGHTS: usize = 4;
/// Maximum coordinate classes carried by one inspection contract.
pub const MAX_INSPECTION_COORDINATES: usize = 9;
/// Maximum effective states carried by one inspection contract.
pub const MAX_INSPECTION_STATES: usize = 7;

/// Namespace-qualified reference to one logical scope.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct LogicalScopeRefV1 {
    /// Namespace owning the scope identity.
    pub namespace: WorkspaceNamespaceId,
    /// Scope identity local to the namespace.
    pub scope_id: LogicalScopeId,
}

/// Namespace-qualified reference to one logical organization.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct LogicalOrganizationRefV1 {
    /// Namespace owning the organization identity.
    pub namespace: WorkspaceNamespaceId,
    /// Organization identity local to the namespace.
    pub organization_id: LogicalOrganizationId,
}

/// Namespace-qualified reference to one group.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalGroupRefV1 {
    /// Namespace owning the group identity.
    pub namespace: WorkspaceNamespaceId,
    /// Group identity local to the namespace.
    pub group_id: OrganizationalGroupId,
}

/// Namespace-qualified reference to one discipline tree.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct DisciplineTreeRefV1 {
    /// Namespace owning the tree identity.
    pub namespace: WorkspaceNamespaceId,
    /// Tree identity local to the namespace.
    pub tree_id: DisciplineTreeId,
}

/// Namespace-qualified reference to one stable role.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalRoleRefV1 {
    /// Namespace owning the role identity.
    pub namespace: WorkspaceNamespaceId,
    /// Role identity local to the namespace.
    pub role_id: OrganizationalRoleId,
}

/// Exact directional-service contract coordinate.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct DirectionalServiceContractRefV1 {
    /// Namespace owning the contract identity.
    pub namespace: WorkspaceNamespaceId,
    /// Contract identity local to the namespace.
    pub contract_id: DirectionalServiceContractId,
    /// Provider-local scope fixed by the contract.
    pub provider_scope: LogicalScopeRefV1,
    /// Consumer-local scope fixed by the contract.
    pub consumer_scope: LogicalScopeRefV1,
}

impl std::fmt::Display for DirectionalServiceContractRefV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}:{}:{}:{}:{}:{}",
            self.namespace,
            self.contract_id,
            self.provider_scope.namespace,
            self.provider_scope.scope_id,
            self.consumer_scope.namespace,
            self.consumer_scope.scope_id
        )
    }
}

impl DirectionalServiceContractRefV1 {
    /// Project this qualified contract coordinate into a generic resource reference.
    ///
    /// Responsibility structural selections carry the common resource grammar,
    /// while placement and organizational services retain this stronger type.
    /// The projection is lossless because both component identities already
    /// satisfy the portable identifier grammar.
    ///
    /// # Panics
    ///
    /// Panics only if a previously validated namespace or contract identity
    /// violates [`WorkspaceResourceId`]'s shared portable grammar.
    #[must_use]
    pub fn resource_id(&self) -> WorkspaceResourceId {
        WorkspaceResourceId::parse(self.to_string())
            .expect("a qualified service coordinate preserves portable resource grammar")
    }
}

/// Inclusive start and optional exclusive end of an effective fact.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct EffectiveIntervalV1 {
    /// Inclusive logical instant or owner revision.
    #[schemars(schema_with = "crate::canon::json_safe_u64_schema")]
    pub start: u64,
    /// Optional exclusive logical instant or owner revision.
    #[schemars(schema_with = "crate::canon::json_safe_u64_schema")]
    pub end_exclusive: Option<u64>,
}

/// Independent owner, controller, authority, resource, and disclosure axes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalAuthorityAxesV1 {
    /// Semantic owner of the fact.
    pub owner: WorkspaceResourceId,
    /// Logical controller scope; this is not controller proof.
    pub controller: WorkspaceControllerScopeId,
    /// Exact authority scope; topology cannot supply it.
    pub authority: WorkspaceAuthorityScopeId,
    /// Canonical zero-to-many resource references.
    #[schemars(length(max = 1024), extend("uniqueItems" = true))]
    pub resources: Vec<WorkspaceResourceId>,
    /// Maximum disclosure for the fact.
    pub disclosure: DisclosureCeilingV1,
}

/// One independently constituted logical scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogicalScopeV1 {
    /// Namespace-qualified identity.
    pub scope: LogicalScopeRefV1,
    /// Independent authority axes.
    pub axes: OrganizationalAuthorityAxesV1,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

/// One independently constituted logical organization reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogicalOrganizationV1 {
    /// Namespace-qualified identity.
    pub organization: LogicalOrganizationRefV1,
    /// Scope within which the organization fact is selected.
    pub scope: LogicalScopeRefV1,
    /// Independent authority axes.
    pub axes: OrganizationalAuthorityAxesV1,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

/// Optional group selected within one logical organization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalGroupV1 {
    /// Namespace-qualified identity.
    pub group: OrganizationalGroupRefV1,
    /// Containing organization.
    pub organization: LogicalOrganizationRefV1,
    /// Independent authority axes.
    pub axes: OrganizationalAuthorityAxesV1,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

/// Optional discipline tree selected within one logical organization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DisciplineTreeV1 {
    /// Namespace-qualified identity.
    pub tree: DisciplineTreeRefV1,
    /// Containing organization.
    pub organization: LogicalOrganizationRefV1,
    /// Independent authority axes.
    pub axes: OrganizationalAuthorityAxesV1,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

/// Stable role with authority independent from label, occupant, and provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalRoleV1 {
    /// Namespace-qualified identity.
    pub role: OrganizationalRoleRefV1,
    /// Organization within which the role is defined.
    pub organization: LogicalOrganizationRefV1,
    /// Optional group association.
    pub group: Option<OrganizationalGroupRefV1>,
    /// Optional discipline-tree association.
    pub discipline_tree: Option<DisciplineTreeRefV1>,
    /// Owner-selected role class; the string grants no authority.
    pub role_class: OrganizationalRoleClassId,
    /// Independent authority axes.
    pub axes: OrganizationalAuthorityAxesV1,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

/// Binding from a stable role to one current occupant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalOccupantBindingV1 {
    /// Stable role being occupied.
    pub role: OrganizationalRoleRefV1,
    /// Occupant identity independent from the role.
    pub occupant: OrganizationalOccupantId,
    /// Issuer of the binding.
    pub issuer: WorkspaceResourceId,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

/// Binding from a stable role to one current software or service provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalProviderBindingV1 {
    /// Stable role being provided.
    pub role: OrganizationalRoleRefV1,
    /// Provider identity independent from the role and occupant.
    pub provider: OrganizationalProviderId,
    /// Issuer of the binding.
    pub issuer: WorkspaceResourceId,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

/// One admitted local resolution for a provider-native communication actor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommunicationActorLocalResolutionV1 {
    /// Attributably bound local person, provider, occupant, or role node.
    pub local_node: OrganizationalNodeRefV1,
    /// Exact already-admitted local closure available to authorized traversal.
    #[schemars(length(max = 1024), extend("uniqueItems" = true))]
    pub authorized_closure: Vec<OrganizationalNodeRefV1>,
}

/// Provider-neutral binding for one canonical communication-actor handle.
///
/// Equal provider and handle coordinates reconcile identity only. Each local
/// node and closure remains separately supplied; the binding creates no role,
/// membership, authority, or foreign edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommunicationActorBindingV1 {
    /// Communication-provider instance.
    pub provider: OrganizationalProviderId,
    /// Canonical provider-native handle.
    pub handle: CommunicationActorHandleId,
    /// Zero-to-many separately attributable local graph resolutions.
    #[schemars(length(max = 1024))]
    pub local_resolutions: Vec<CommunicationActorLocalResolutionV1>,
    /// Issuer of the identity-only binding.
    pub issuer: WorkspaceResourceId,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

wire_enum!(
    /// Neutral class of one explicit organizational relation.
    OrganizationalRelationClassV1 {
        /// Group coordinates an exact selected tree.
        Coordinates => "coordinates",
        /// Local integrated-roadmap coordination.
        Primary => "primary",
        /// Constitutive accepted-needs contribution.
        Leader => "leader",
        /// Bounded contribution.
        Supporting => "supporting",
        /// Exact people-management edge.
        PeopleManagement => "people_management",
        /// Exact roadmap-coordination edge.
        RoadmapCoordination => "roadmap_coordination",
        /// Exact discipline-judgment edge.
        DisciplineJudgment => "discipline_judgment",
        /// Exact standards edge.
        Standards => "standards",
        /// Exact review edge.
        Review => "review",
        /// Lens-scoped bounded block edge.
        BoundedBlock => "bounded_block",
        /// Exact escalation edge.
        Escalation => "escalation",
        /// Exact local acceptance edge.
        Acceptance => "acceptance",
        /// Exact native-presence edge.
        NativePresence => "native_presence",
        /// Exact accountable-role collaboration edge.
        Collaboration => "collaboration"
    }
);

wire_enum!(
    /// Locality law selected by an organizational relation class.
    OrganizationalRelationLocalityV1 {
        /// Both endpoints must remain within one owner and local container.
        Internal => "internal",
        /// The relation records native presence without importing native ownership.
        NativeBoundary => "native_boundary"
    }
);

impl OrganizationalRelationClassV1 {
    /// Return the locality law owned by this closed relation class.
    ///
    /// Cross-owner service and topic leadership are represented by
    /// [`DirectionalServiceContractV1`] and [`LeadershipParticipationV1`], not
    /// by widening an internal relation.
    #[must_use]
    pub const fn locality(self) -> OrganizationalRelationLocalityV1 {
        match self {
            Self::NativePresence => OrganizationalRelationLocalityV1::NativeBoundary,
            Self::Coordinates
            | Self::Primary
            | Self::Leader
            | Self::Supporting
            | Self::PeopleManagement
            | Self::RoadmapCoordination
            | Self::DisciplineJudgment
            | Self::Standards
            | Self::Review
            | Self::BoundedBlock
            | Self::Escalation
            | Self::Acceptance
            | Self::Collaboration => OrganizationalRelationLocalityV1::Internal,
        }
    }

    /// Return whether this class can bound neutral group-lens authority.
    ///
    /// A lens aligns peers through [`Self::Collaboration`]. Management,
    /// acceptance, escalation, and other internal relations retain their own
    /// meanings and cannot silently acquire lens authority.
    #[must_use]
    pub const fn is_group_lens_alignment(self) -> bool {
        matches!(self, Self::Collaboration)
    }
}

/// Dedicated authored fact from which a graph edge was derived.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OrganizationalDerivedRelationSourceV1 {
    /// One exact occupant binding.
    OccupantBinding {
        /// Bound role.
        role: OrganizationalRoleRefV1,
        /// Bound occupant.
        occupant: OrganizationalOccupantId,
        /// Effective interval distinguishing repeated bindings over time.
        interval: EffectiveIntervalV1,
    },
    /// One exact provider binding.
    ProviderBinding {
        /// Provided role.
        role: OrganizationalRoleRefV1,
        /// Bound provider.
        provider: OrganizationalProviderId,
        /// Effective interval distinguishing repeated bindings over time.
        interval: EffectiveIntervalV1,
    },
    /// One exact topic-scoped leadership fact.
    LeadershipParticipation {
        /// Topic owning the participation set.
        topic: LeadershipTopicId,
    },
}

/// Typed endpoint of an organizational relation.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OrganizationalNodeRefV1 {
    /// Logical scope endpoint.
    Scope {
        /// Exact scope reference.
        scope: LogicalScopeRefV1,
    },
    /// Logical organization endpoint.
    Organization {
        /// Exact organization reference.
        organization: LogicalOrganizationRefV1,
    },
    /// Group endpoint.
    Group {
        /// Exact group reference.
        group: OrganizationalGroupRefV1,
    },
    /// Discipline-tree endpoint.
    DisciplineTree {
        /// Exact discipline-tree reference.
        tree: DisciplineTreeRefV1,
    },
    /// Stable-role endpoint.
    Role {
        /// Exact stable-role reference.
        role: OrganizationalRoleRefV1,
    },
    /// Occupant endpoint.
    Occupant {
        /// Exact occupant identity.
        occupant: OrganizationalOccupantId,
    },
    /// Provider endpoint.
    Provider {
        /// Exact provider identity.
        provider: OrganizationalProviderId,
    },
    /// Native resource endpoint.
    NativeResource {
        /// Exact native resource reference.
        resource: WorkspaceResourceId,
    },
    /// Owner-selected object endpoint.
    Object {
        /// Exact object identity.
        object: OrganizationalObjectId,
    },
}

/// One explicit non-authorizing organizational relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalRelationV1 {
    /// Stable relation identity.
    pub relation_id: OrganizationalRelationId,
    /// Closed relation class.
    pub class: OrganizationalRelationClassV1,
    /// Directional source endpoint.
    pub from: OrganizationalNodeRefV1,
    /// Directional destination endpoint.
    pub to: OrganizationalNodeRefV1,
    /// Owner that issued this exact edge.
    pub issuer: WorkspaceResourceId,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

wire_enum!(
    /// Relation classes derived only from their dedicated authored facts.
    OrganizationalDerivedRelationClassV1 {
        /// Derived from [`OrganizationalOccupantBindingV1`].
        Occupancy => "occupancy",
        /// Derived from [`OrganizationalProviderBindingV1`].
        ProviderPresence => "provider_presence",
        /// Derived from [`LeadershipParticipationV1`].
        LeadershipParticipation => "leadership_participation"
    }
);

/// Non-authorizing graph edge derived from one dedicated source fact.
///
/// No caller authors this record directly in [`OrganizationalTranslationV1`].
/// Its issuer, state, and interval are copied deterministically from the
/// binding or leadership fact that remains the semantic source of truth.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalDerivedRelationV1 {
    /// Dedicated fact coordinate retaining derivation provenance.
    pub source: OrganizationalDerivedRelationSourceV1,
    /// Dedicated relation class.
    pub class: OrganizationalDerivedRelationClassV1,
    /// Derived directional source endpoint.
    pub from: OrganizationalNodeRefV1,
    /// Derived directional destination endpoint.
    pub to: OrganizationalNodeRefV1,
    /// Issuer copied from the owning source fact.
    pub issuer: WorkspaceResourceId,
    /// State copied from the owning source fact.
    pub state: EffectiveFactStateV1,
    /// Interval copied from the owning source fact.
    pub interval: EffectiveIntervalV1,
}

/// Optional topic-scoped leadership participation set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LeadershipParticipationV1 {
    /// Stable topic identity.
    pub topic: LeadershipTopicId,
    /// Explicit participating roles.
    #[schemars(length(min = 1, max = 1024), extend("uniqueItems" = true))]
    pub participants: Vec<OrganizationalRoleRefV1>,
    /// Exact affected scopes; unlisted scopes gain no edge or visibility.
    #[schemars(length(min = 1, max = 1024), extend("uniqueItems" = true))]
    pub affected_scopes: Vec<LogicalScopeRefV1>,
    /// Issuer of the topic-scoped participation fact.
    pub issuer: WorkspaceResourceId,
    /// Maximum disclosure of this participation surface.
    pub disclosure: DisclosureCeilingV1,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

/// One owner-selected object and its independent authority axes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalObjectV1 {
    /// Stable object identity.
    pub object_id: OrganizationalObjectId,
    /// Namespace whose local grammar contains the object.
    pub namespace: WorkspaceNamespaceId,
    /// Exact logical scope containing the object.
    pub scope: LogicalScopeRefV1,
    /// Exact organization containing the object.
    pub organization: LogicalOrganizationRefV1,
    /// Owner-selected object class.
    pub object_class: OrganizationalObjectClassId,
    /// Independent authority axes.
    pub axes: OrganizationalAuthorityAxesV1,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

/// Explicit object-class-to-accountable-role population rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObjectRolePopulationRuleV1 {
    /// Owner-selected object class.
    pub object_class: OrganizationalObjectClassId,
    /// Owner-selected accountable role class.
    pub accountable_role_class: OrganizationalRoleClassId,
    /// Issuer of the rule.
    pub issuer: WorkspaceResourceId,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

wire_enum!(
    /// Rights that a group-lens profile may explicitly carry.
    GroupLensRightV1 {
        /// Apply named standards within the lens.
        Standards => "standards",
        /// Perform named review within the lens.
        Review => "review",
        /// Block only under declared lens conditions.
        BoundedBlock => "bounded_block",
        /// Escalate only through the declared resolver.
        Escalation => "escalation"
    }
);

/// Lens-scoped optional standards, review, block, and escalation authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupLensAuthorityProfileV1 {
    /// Stable profile identity.
    pub profile_id: GroupLensProfileId,
    /// Exact stable role carrying the declared lens-scoped rights.
    pub holder: OrganizationalRoleRefV1,
    /// Exact local relation whose context bounds this authority profile.
    pub alignment_relation: OrganizationalRelationId,
    /// Owner-selected governed lens.
    pub lens: OrganizationalLensId,
    /// Role accountable for review when [`GroupLensRightV1::Review`] is selected.
    pub review_owner: Option<OrganizationalRoleRefV1>,
    /// Exact named standards references.
    #[schemars(length(max = 1024), extend("uniqueItems" = true))]
    pub standards: Vec<WorkspaceResourceId>,
    /// Exact conditions under which bounded block is permitted.
    #[schemars(length(max = 1024), extend("uniqueItems" = true))]
    pub block_conditions: Vec<OrganizationalConditionId>,
    /// Exact resolver when [`GroupLensRightV1::Escalation`] is selected.
    pub escalation_resolver: Option<OrganizationalRoleRefV1>,
    /// Canonical set of the only rights carried by this profile.
    #[schemars(length(max = 4), extend("uniqueItems" = true))]
    pub rights: Vec<GroupLensRightV1>,
    /// Issuer of the profile.
    pub issuer: WorkspaceResourceId,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

/// One account/mandate/steward/pod many-to-many service assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServiceAssignmentV1 {
    /// Exact account.
    pub account: ServiceAccountId,
    /// Exact mandate.
    pub mandate: ServiceMandateId,
    /// Zero-to-many stewards; no member is primary by position.
    #[schemars(length(max = 1024), extend("uniqueItems" = true))]
    pub stewards: Vec<ServiceStewardId>,
    /// Zero-to-many pods; no member is primary by position.
    #[schemars(length(max = 1024), extend("uniqueItems" = true))]
    pub pods: Vec<ServicePodId>,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

/// Generic directional service with independently accepted local frontiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DirectionalServiceContractV1 {
    /// Stable contract identity.
    pub contract_id: DirectionalServiceContractId,
    /// Namespace owning the contract identity.
    pub namespace: WorkspaceNamespaceId,
    /// Relationship/contract owner.
    pub contract_owner: WorkspaceResourceId,
    /// Provider-local scope.
    pub provider_scope: LogicalScopeRefV1,
    /// Consumer-local scope.
    pub consumer_scope: LogicalScopeRefV1,
    /// Provider-local completion requirement.
    pub provider_completion: ProviderCompletionRequirementV1,
    /// Provider-local acceptance requirement.
    pub provider_acceptance: ProviderAcceptanceRequirementV1,
    /// Consumer-local consequence acceptance requirement.
    pub consumer_acceptance: ConsumerAcceptanceRequirementV1,
    /// Native effect/read-back requirement.
    pub native_read_back: NativeReadBackRequirementV1,
    /// Pale settlement-classification requirement.
    pub pale_settlement: PaleSettlementRequirementV1,
    /// Explicit many-to-many assignments.
    #[schemars(length(max = 1024))]
    pub assignments: Vec<ServiceAssignmentV1>,
    /// Contract policy ceiling.
    pub ceiling: WorkspacePolicyCeilingV1,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

impl DirectionalServiceContractV1 {
    /// Return the canonical namespace-qualified contract identity.
    #[must_use]
    pub fn contract_ref(&self) -> DirectionalServiceContractRefV1 {
        DirectionalServiceContractRefV1 {
            namespace: self.namespace.clone(),
            contract_id: self.contract_id.clone(),
            provider_scope: self.provider_scope.clone(),
            consumer_scope: self.consumer_scope.clone(),
        }
    }
}

/// Provider-neutral native projection of one organizational node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalNativeProjectionV1 {
    /// Stable projection identity.
    pub projection_id: OrganizationalProjectionId,
    /// Semantic node being projected.
    pub subject: OrganizationalNodeRefV1,
    /// Provider-neutral projection class.
    pub class: ResponsibilityProjectionClassV1,
    /// Native owner retaining record and effect truth.
    pub native_owner: WorkspaceResourceId,
    /// Zero-to-many native references; none is primary.
    #[schemars(length(max = 1024), extend("uniqueItems" = true))]
    pub native_refs: Vec<WorkspaceResourceId>,
    /// Projection-local disclosure ceiling.
    pub disclosure: DisclosureCeilingV1,
    /// Explicit effective state.
    pub state: EffectiveFactStateV1,
    /// Effective interval.
    pub interval: EffectiveIntervalV1,
}

wire_enum!(
    /// Coordinate family selectable by a full-matrix inspection contract.
    InspectionCoordinateClassV1 {
        /// Logical scopes and organizations.
        ScopeOrganization => "scope_organization",
        /// Groups and discipline trees.
        GroupTree => "group_tree",
        /// Stable roles and bindings.
        RoleBinding => "role_binding",
        /// Owner-selected atomic objects.
        AtomicObject => "atomic_object",
        /// Group-lens authority profiles.
        GroupLens => "group_lens",
        /// Directional service assignments.
        DirectionalService => "directional_service",
        /// Independent acceptance and effect frontiers.
        AcceptanceFrontier => "acceptance_frontier",
        /// Native projections.
        NativeProjection => "native_projection",
        /// Explicit negative and unresolved states.
        EffectiveState => "effective_state"
    }
);

impl InspectionCoordinateClassV1 {
    /// Return every coordinate family required by a complete inspection.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::ScopeOrganization,
            Self::GroupTree,
            Self::RoleBinding,
            Self::AtomicObject,
            Self::GroupLens,
            Self::DirectionalService,
            Self::AcceptanceFrontier,
            Self::NativeProjection,
            Self::EffectiveState,
        ]
    }
}

impl EffectiveFactStateV1 {
    /// Return every distinct effective-state value in wire order.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Present,
            Self::Absent,
            Self::Denied,
            Self::Unknown,
            Self::Stale,
            Self::Superseded,
            Self::Revoked,
        ]
    }
}

/// Principal-authorized derived inspection contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalInspectionV1 {
    /// Stable inspection contract identity.
    pub inspection_id: OrganizationalInspectionId,
    /// Principal authorizing the view.
    pub principal: WorkspaceResourceId,
    /// Maximum visible field/disclosure ceiling.
    pub disclosure: DisclosureCeilingV1,
    /// Optional view-provider binding; absence keeps the grammar provider-neutral.
    pub view_provider: Option<WorkspaceResourceId>,
    /// Exact coordinate families selected for dereference.
    #[schemars(length(min = 1, max = 9), extend("uniqueItems" = true))]
    pub coordinates: Vec<InspectionCoordinateClassV1>,
    /// Exact states the view must render distinctly.
    #[schemars(length(min = 1, max = 7), extend("uniqueItems" = true))]
    pub states: Vec<EffectiveFactStateV1>,
}

/// Complete pre-wiring source for organizational translation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalTranslationV1 {
    /// Stable wire schema identity.
    #[schemars(extend("const" = ORGANIZATIONAL_TRANSLATION_V1_SCHEMA))]
    pub schema: String,
    /// Initial wire version.
    #[schemars(extend("const" = ORGANIZATIONAL_TRANSLATION_V1_SCHEMA_VERSION))]
    pub schema_version: u32,
    /// Optional logical scopes.
    #[schemars(length(max = 1024))]
    pub scopes: Vec<LogicalScopeV1>,
    /// Optional logical organizations.
    #[schemars(length(max = 1024))]
    pub organizations: Vec<LogicalOrganizationV1>,
    /// Optional groups.
    #[schemars(length(max = 1024))]
    pub groups: Vec<OrganizationalGroupV1>,
    /// Optional discipline trees.
    #[schemars(length(max = 1024))]
    pub discipline_trees: Vec<DisciplineTreeV1>,
    /// Optional stable roles.
    #[schemars(length(max = 1024))]
    pub roles: Vec<OrganizationalRoleV1>,
    /// Optional occupant bindings.
    #[schemars(length(max = 1024))]
    pub occupant_bindings: Vec<OrganizationalOccupantBindingV1>,
    /// Optional provider bindings.
    #[schemars(length(max = 1024))]
    pub provider_bindings: Vec<OrganizationalProviderBindingV1>,
    /// Optional provider-neutral communication-actor bindings.
    #[schemars(length(max = 1024))]
    pub communication_actor_bindings: Vec<CommunicationActorBindingV1>,
    /// Optional topic-scoped leadership participation.
    #[schemars(length(max = 1024))]
    pub leadership: Vec<LeadershipParticipationV1>,
    /// Explicit typed relations.
    #[schemars(length(max = 4096))]
    pub relations: Vec<OrganizationalRelationV1>,
    /// Optional owner-selected objects.
    #[schemars(length(max = 1024))]
    pub objects: Vec<OrganizationalObjectV1>,
    /// Optional object-class population rules.
    #[schemars(length(max = 1024))]
    pub population_rules: Vec<ObjectRolePopulationRuleV1>,
    /// Optional group-lens authority profiles.
    #[schemars(length(max = 1024))]
    pub group_lenses: Vec<GroupLensAuthorityProfileV1>,
    /// Optional sparse directional-service contracts.
    #[schemars(length(max = 1024))]
    pub services: Vec<DirectionalServiceContractV1>,
    /// Optional zero-to-many native projections.
    #[schemars(length(max = 1024))]
    pub native_projections: Vec<OrganizationalNativeProjectionV1>,
    /// Optional principal-authorized full-matrix inspection contract.
    pub inspection: Option<OrganizationalInspectionV1>,
}

/// Canonically ordered organizational translation accepted by the pure lowerer.
///
/// This IR remains desired-state semantics. It is not a compiled generation,
/// native projection receipt, activation, or owner read-back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationalTranslationIrV1 {
    /// Canonical validated source.
    pub source: OrganizationalTranslationV1,
    /// Canonical non-authorizing edges derived from dedicated source facts.
    #[schemars(length(max = 4096), extend("uniqueItems" = true))]
    pub derived_relations: Vec<OrganizationalDerivedRelationV1>,
}
