//! Shallow organizational observations; no graph derivation or authority policy.

use pale_application::workspace::OrganizationInspectionRecord as Record;
use pale_core::records as core;

use super::{CopyBudget, OrganizationFailure};

/// Namespace-qualified scope, not a generic resource or resident workspace ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationScopeRef {
    /// Owning namespace.
    pub namespace: String,
    /// Scope-local identity.
    pub scope_id: String,
}

/// Namespace-qualified organization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationRef {
    /// Owning namespace.
    pub namespace: String,
    /// Organization-local identity.
    pub organization_id: String,
}

/// Namespace-qualified group, independently associated with its organization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationGroupRef {
    /// Owning namespace.
    pub namespace: String,
    /// Group-local identity.
    pub group_id: String,
}

/// Namespace-qualified discipline tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationTreeRef {
    /// Owning namespace.
    pub namespace: String,
    /// Tree-local identity.
    pub tree_id: String,
}

/// Namespace-qualified stable role, not its occupant or provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationRoleRef {
    /// Owning namespace.
    pub namespace: String,
    /// Role-local identity.
    pub role_id: String,
}

/// Exact directional-service identity; both scope coordinates participate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationServiceRef {
    /// Contract-owning namespace.
    pub namespace: String,
    /// Contract-local identity.
    pub contract_id: String,
    /// Exact provider scope.
    pub provider_scope: OrganizationScopeRef,
    /// Exact consumer scope.
    pub consumer_scope: OrganizationScopeRef,
}

/// Existing endpoint kind; no identity is inferred from a display label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationNode {
    /// Logical scope.
    Scope(OrganizationScopeRef),
    /// Logical organization.
    Organization(OrganizationRef),
    /// Group.
    Group(OrganizationGroupRef),
    /// Discipline tree.
    DisciplineTree(OrganizationTreeRef),
    /// Stable role.
    Role(OrganizationRoleRef),
    /// Occupant identity, separate from a role.
    Occupant(String),
    /// Provider identity, separate from an occupant.
    Provider(String),
    /// Owner-native resource reference, not its body.
    NativeResource(String),
    /// Object endpoint in the existing owning node-reference grammar.
    Object(String),
}

/// Copied owner interval, never an editor generation or presentation clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrganizationInterval {
    /// Inclusive logical start.
    pub start: u64,
    /// Optional exclusive logical end.
    pub end_exclusive: Option<u64>,
}

/// Typed observation of the owning effective-state enum, with no state algebra.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationFactState {
    /// Selected and effective.
    Present,
    /// Explicitly absent.
    Absent,
    /// Explicitly denied.
    Denied,
    /// Current fact cannot be established.
    Unknown,
    /// Outside the owner's freshness boundary.
    Stale,
    /// Replaced by another owner fact.
    Superseded,
    /// Explicitly revoked.
    Revoked,
}

/// Declared disclosure ceiling, not recipient admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationDisclosure {
    /// Declared private ceiling.
    Private,
    /// Declared protected ceiling.
    Protected,
    /// Declared public ceiling.
    Public,
}

/// Declared effect ceiling; inspection neither evaluates nor grants an effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationEffectCeiling {
    /// No effect permitted by this declaration.
    None,
    /// Declared read ceiling.
    Read,
    /// Declared mutation ceiling.
    Mutate,
}

/// Existing explicit relation class, not a renderer routing style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationRelationClass {
    /// Exact group/tree coordination.
    Coordinates,
    /// Local integrated-roadmap coordination.
    Primary,
    /// Constitutive accepted-needs contribution.
    Leader,
    /// Bounded contribution.
    Supporting,
    /// Exact people-management relation.
    PeopleManagement,
    /// Exact roadmap-coordination relation.
    RoadmapCoordination,
    /// Exact discipline-judgment relation.
    DisciplineJudgment,
    /// Exact standards relation.
    Standards,
    /// Exact review relation.
    Review,
    /// Lens-scoped bounded block.
    BoundedBlock,
    /// Exact escalation relation.
    Escalation,
    /// Exact local acceptance relation.
    Acceptance,
    /// Native presence, not imported ownership.
    NativePresence,
    /// Exact accountable-role collaboration.
    Collaboration,
}

/// Locality selected by the canonical explicit-relation class.
///
/// This describes a declared relation, not native existence, provider admission
/// or permission to cross a boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationRelationLocality {
    /// Endpoints remain within one owner and local container.
    Internal,
    /// Declared native presence without importing native ownership.
    NativeBoundary,
}

/// Class already derived by the application from its dedicated authored fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationDerivedClass {
    /// Already derived from an occupant binding.
    Occupancy,
    /// Already derived from a provider binding.
    ProviderPresence,
    /// Already derived from the topic participation fact.
    LeadershipParticipation,
}

/// Provider-neutral declared projection class, not evidence of realization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationProjectionClass {
    /// Accountable work and local acceptance.
    AccountableWork,
    /// Conversation, membership or bounded coordination.
    Communication,
    /// Relationship or commercial context.
    Relationship,
    /// Native request, result, effect or read-back.
    NativeApplication,
    /// Derived navigation or inspection.
    DerivedView,
    /// Bounded derived context.
    Context,
    /// Causal reference and occurrence lineage.
    Genealogy,
}

/// Lifecycle asserted by the responsibility author, not newly verified by inspection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationResponsibilityLifecycle {
    /// Author asserts verified current owner truth.
    VerifiedCurrent,
    /// Accepted future semantic target.
    AcceptedTarget,
    /// Principal-owner desired configuration.
    OwnerDesiredConfiguration,
    /// Candidate design, not accepted ABI status.
    Candidate,
    /// Implementation remains open.
    ImplementationOpen,
    /// Private population remains open.
    ConfigurationOpen,
    /// Evidence must close before use.
    EvidenceGated,
    /// Historical evidence only.
    Historical,
    /// Rejected proposition retained as genealogy.
    Rejected,
}

/// Independent scalar authority axes; resources have a separate bounded query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationAxes {
    /// Semantic owner.
    pub owner: String,
    /// Declared controller, not controller proof.
    pub controller: String,
    /// Declared authority, not a grant.
    pub authority: String,
    /// Declared disclosure ceiling.
    pub disclosure: OrganizationDisclosure,
}

/// Owner and evidence reference for one separately named lifecycle requirement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationEvidenceRequirement {
    /// Owner that can establish the fact.
    pub owner: String,
    /// Exact evidence reference, without its body.
    pub evidence_ref: String,
}

/// Attribution preserved for a supplied answer or explicit omission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationAttribution {
    /// Exact source coordinate.
    pub source_id: String,
    /// Attributable owner.
    pub owner: String,
    /// Exact evidence reference.
    pub evidence_ref: String,
}

/// Actual attributed responsibility placement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationPlacementAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: OrganizationResponsibilityPlacement,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}

/// Actual attributed generic resource answer; no identity-class cast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationResourceAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: String,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}

/// Provider completion answer, separate from provider acceptance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationProviderCompletionAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: OrganizationEvidenceRequirement,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}

/// Provider acceptance answer, separate from completion and consumer acceptance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationProviderAcceptanceAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: OrganizationEvidenceRequirement,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}

/// Consumer-local consequence acceptance answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationConsumerAcceptanceAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: OrganizationEvidenceRequirement,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}

/// Native result/read-back answer, not a settlement result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationNativeReadBackAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: OrganizationEvidenceRequirement,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}

/// Pale obligation-settlement answer, separate from native-owner acceptance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationSettlementAnswer {
    /// Owner supplied this exact answer.
    Supplied {
        /// Typed value, not evidence that a declared requirement has occurred.
        value: OrganizationEvidenceRequirement,
        /// Original attribution.
        attribution: OrganizationAttribution,
    },
    /// Owner explicitly omitted the answer; not unknown or false.
    Omitted {
        /// Existing owner-defined omission reason.
        reason: String,
        /// Original omission attribution.
        attribution: OrganizationAttribution,
    },
}

// Shared projection mechanics remain private, never an SDK payload parameter.
enum ProjectedAnswer<T> {
    Supplied {
        value: T,
        attribution: OrganizationAttribution,
    },
    Omitted {
        reason: String,
        attribution: OrganizationAttribution,
    },
}

/// Actual responsibility placement without guessing a scope from a resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationResponsibilityPlacement {
    /// Generic local resource exactly as declared; no logical-scope cast.
    VerticalLocal(String),
    /// Exact service coordinate already checked by the owning linker.
    DirectionalService(OrganizationServiceRef),
}

/// Complete source coordinate of an already-derived edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationDerivedSource {
    /// Exact temporal occupant binding.
    OccupantBinding {
        /// Stable role.
        role: OrganizationRoleRef,
        /// Occupant.
        occupant: String,
        /// Source binding interval.
        interval: OrganizationInterval,
    },
    /// Exact temporal provider binding.
    ProviderBinding {
        /// Stable role.
        role: OrganizationRoleRef,
        /// Provider.
        provider: String,
        /// Source binding interval.
        interval: OrganizationInterval,
    },
    /// Topic is only the source coordinate; class/endpoints/interval still identify the edge.
    LeadershipParticipation(String),
}

/// Shallow responsibility header with independent attributed answers.
///
/// Kept in one allocation when held by [`OrganizationRow`], so its complete
/// attribution payload does not determine every other row's inline size.
/// The copied-string budget excludes this allocation and allocator overhead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizationResponsibility {
    /// Exact responsibility derivation identity.
    pub derivation_id: String,
    /// Lifecycle asserted by the author; not reverified by this query.
    pub lifecycle: OrganizationResponsibilityLifecycle,
    /// Original complete-record source/owner attribution.
    pub provenance: OrganizationAttribution,
    /// Actual supplied placement or explicit attributed omission.
    pub placement: OrganizationPlacementAnswer,
    /// Independent provider-local completion requirement.
    pub provider_completion: OrganizationProviderCompletionAnswer,
    /// Independent provider-local acceptance requirement.
    pub provider_acceptance: OrganizationProviderAcceptanceAnswer,
    /// Independent consumer-local consequence acceptance requirement.
    pub consumer_acceptance: OrganizationConsumerAcceptanceAnswer,
    /// Independent supplied native-result/read-back requirement or omission.
    pub native_result: OrganizationNativeReadBackAnswer,
    /// Independent Pale obligation-settlement requirement.
    pub pale_settlement: OrganizationSettlementAnswer,
}

/// Shallow copied records from the existing eighteen-family owner.
///
/// These public values can be copied or constructed as observations, but no
/// query accepts them as authority or reconstructs an opaque record handle.
/// Nested collections are intentionally accessed through their owning handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrganizationRow {
    /// Logical scope and independent axes.
    Scope {
        /// Exact namespace-qualified logical scope.
        scope: OrganizationScopeRef,
        /// Independent owner, controller, authority and disclosure observations; resources are queried separately.
        axes: OrganizationAxes,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Organization and its actual scope.
    Organization {
        /// Exact namespace-qualified organization association.
        organization: OrganizationRef,
        /// Exact namespace-qualified logical scope.
        scope: OrganizationScopeRef,
        /// Independent owner, controller, authority and disclosure observations; resources are queried separately.
        axes: OrganizationAxes,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Group and its actual organization.
    Group {
        /// Actual group association; not substituted for the discipline tree.
        group: OrganizationGroupRef,
        /// Exact namespace-qualified organization association.
        organization: OrganizationRef,
        /// Independent owner, controller, authority and disclosure observations; resources are queried separately.
        axes: OrganizationAxes,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Tree and its actual organization.
    DisciplineTree {
        /// Exact namespace-qualified discipline tree.
        tree: OrganizationTreeRef,
        /// Exact namespace-qualified organization association.
        organization: OrganizationRef,
        /// Independent owner, controller, authority and disclosure observations; resources are queried separately.
        axes: OrganizationAxes,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Role with independent group and tree associations.
    Role {
        /// Exact stable role, independent of its occupant/provider.
        role: OrganizationRoleRef,
        /// Exact namespace-qualified organization association.
        organization: OrganizationRef,
        /// Actual group association; not substituted for the discipline tree.
        group: Option<OrganizationGroupRef>,
        /// Actual optional tree association, independently of group.
        discipline_tree: Option<OrganizationTreeRef>,
        /// Owner-selected role class; its spelling grants no authority.
        role_class: String,
        /// Independent owner, controller, authority and disclosure observations; resources are queried separately.
        axes: OrganizationAxes,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Occupant is not a provider or the role itself.
    OccupantBinding {
        /// Exact stable role, independent of its occupant/provider.
        role: OrganizationRoleRef,
        /// Actual occupant identity, not the role or provider.
        occupant: String,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Provider is not an occupant or the role itself.
    ProviderBinding {
        /// Exact stable role, independent of its occupant/provider.
        role: OrganizationRoleRef,
        /// Actual provider identity, not an occupant or native readiness proof.
        provider: String,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Identity-only communication binding, with separate local-resolution detail.
    CommunicationActor {
        /// Actual provider identity, not an occupant or native readiness proof.
        provider: String,
        /// Canonical provider-native communication handle; not an authorization union.
        handle: String,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Topic and declared visibility; participant and scope lists are separate.
    Leadership {
        /// Exact topic coordinate; derived edges also require endpoints and interval.
        topic: String,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Declared disclosure ceiling, not admitted recipient context.
        disclosure: OrganizationDisclosure,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Exact explicit multigraph edge.
    Relation {
        /// Exact explicit relation identity, preserving parallel relations.
        relation_id: String,
        /// Actual owner-defined class, not a renderer style.
        class: OrganizationRelationClass,
        /// Canonical class-owned locality, not inferred from labels or endpoints.
        locality: OrganizationRelationLocality,
        /// Exact typed directional source endpoint.
        from: OrganizationNode,
        /// Exact typed directional destination endpoint.
        to: OrganizationNode,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Object preserves its actual namespace and both independent associations.
    Object {
        /// Actual owner-selected object identity.
        object_id: String,
        /// Actual owning namespace; not discarded during projection.
        namespace: String,
        /// Exact namespace-qualified logical scope.
        scope: OrganizationScopeRef,
        /// Exact namespace-qualified organization association.
        organization: OrganizationRef,
        /// Owner-selected object class; not inferred from a label.
        object_class: String,
        /// Independent owner, controller, authority and disclosure observations; resources are queried separately.
        axes: OrganizationAxes,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Declared class pairing, not inferred membership.
    PopulationRule {
        /// Owner-selected object class; not inferred from a label.
        object_class: String,
        /// Explicit accountable role class in the population rule.
        accountable_role_class: String,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Named lens and its explicit holders/resolvers.
    GroupLens {
        /// Exact group-lens authority-profile identity.
        profile_id: String,
        /// Exact role carrying the declared lens-scoped profile.
        holder: OrganizationRoleRef,
        /// Existing relation that bounds the lens context.
        alignment_relation: String,
        /// Owner-selected governed lens.
        lens: String,
        /// Actual optional accountable review role.
        review_owner: Option<OrganizationRoleRef>,
        /// Actual optional escalation resolver role.
        escalation_resolver: Option<OrganizationRoleRef>,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Directional service and five non-substituted requirement fields.
    Service {
        /// Complete contract identity, including namespace and both directional scopes.
        contract: OrganizationServiceRef,
        /// Owner of the relationship/contract, not a selected provider.
        contract_owner: String,
        /// Independent provider-local completion requirement.
        provider_completion: OrganizationEvidenceRequirement,
        /// Independent provider-local acceptance requirement.
        provider_acceptance: OrganizationEvidenceRequirement,
        /// Independent consumer-local consequence acceptance requirement.
        consumer_acceptance: OrganizationEvidenceRequirement,
        /// Independent native owner effect/read-back requirement.
        native_read_back: OrganizationEvidenceRequirement,
        /// Independent Pale obligation-settlement requirement.
        pale_settlement: OrganizationEvidenceRequirement,
        /// Declared disclosure ceiling, not admitted recipient context.
        disclosure: OrganizationDisclosure,
        /// Declared effect ceiling; inspection does not grant or perform an effect.
        effect: OrganizationEffectCeiling,
        /// Declared model-use ceiling, not a dispatch decision.
        model_use_permitted: bool,
        /// Declared retention ceiling in seconds, not a live retention receipt.
        retention_seconds: u64,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Desired native links are queried separately and do not prove realization.
    NativeProjection {
        /// Exact native-projection identity.
        projection_id: String,
        /// Actual typed semantic subject of the desired projection.
        subject: OrganizationNode,
        /// Actual owner-defined class, not a renderer style.
        class: OrganizationProjectionClass,
        /// Native owner retaining record/effect truth.
        native_owner: String,
        /// Declared disclosure ceiling, not admitted recipient context.
        disclosure: OrganizationDisclosure,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Declared inspection selection, not authenticated recipient context.
    Inspection {
        /// Exact configured inspection-contract identity.
        inspection_id: String,
        /// Declared principal reference; no authenticated authority is inferred.
        principal: String,
        /// Declared disclosure ceiling, not admitted recipient context.
        disclosure: OrganizationDisclosure,
        /// Optional configured view provider, not proof of availability.
        view_provider: Option<String>,
    },
    /// Source, class, both endpoints and interval jointly preserve parallel edges.
    DerivedRelation {
        /// Complete dedicated source-fact coordinate for this derived edge.
        source: OrganizationDerivedSource,
        /// Actual owner-defined class, not a renderer style.
        class: OrganizationDerivedClass,
        /// Exact typed directional source endpoint.
        from: OrganizationNode,
        /// Exact typed directional destination endpoint.
        to: OrganizationNode,
        /// Owner that issued this exact fact.
        issuer: String,
        /// Actual owner-defined effective state, including distinct negative states.
        state: OrganizationFactState,
        /// Actual effective interval, not editor or presentation time.
        interval: OrganizationInterval,
    },
    /// Shallow responsibility structure; omitted answers retain reason and attribution.
    Responsibility {
        /// Complete shallow responsibility header, not a custody token.
        record: Box<OrganizationResponsibility>,
    },
}

pub(super) fn interval(v: &core::EffectiveIntervalV1) -> OrganizationInterval {
    OrganizationInterval {
        start: v.start,
        end_exclusive: v.end_exclusive,
    }
}

pub(super) fn scope(
    v: &core::LogicalScopeRefV1,
    b: &mut CopyBudget,
) -> Result<OrganizationScopeRef, OrganizationFailure> {
    Ok(OrganizationScopeRef {
        namespace: b.text(v.namespace.as_str())?,
        scope_id: b.text(v.scope_id.as_str())?,
    })
}

fn organization(
    v: &core::LogicalOrganizationRefV1,
    b: &mut CopyBudget,
) -> Result<OrganizationRef, OrganizationFailure> {
    Ok(OrganizationRef {
        namespace: b.text(v.namespace.as_str())?,
        organization_id: b.text(v.organization_id.as_str())?,
    })
}

fn group(
    v: &core::OrganizationalGroupRefV1,
    b: &mut CopyBudget,
) -> Result<OrganizationGroupRef, OrganizationFailure> {
    Ok(OrganizationGroupRef {
        namespace: b.text(v.namespace.as_str())?,
        group_id: b.text(v.group_id.as_str())?,
    })
}

fn tree(
    v: &core::DisciplineTreeRefV1,
    b: &mut CopyBudget,
) -> Result<OrganizationTreeRef, OrganizationFailure> {
    Ok(OrganizationTreeRef {
        namespace: b.text(v.namespace.as_str())?,
        tree_id: b.text(v.tree_id.as_str())?,
    })
}

pub(super) fn role(
    v: &core::OrganizationalRoleRefV1,
    b: &mut CopyBudget,
) -> Result<OrganizationRoleRef, OrganizationFailure> {
    Ok(OrganizationRoleRef {
        namespace: b.text(v.namespace.as_str())?,
        role_id: b.text(v.role_id.as_str())?,
    })
}

fn service(
    v: &core::DirectionalServiceContractRefV1,
    b: &mut CopyBudget,
) -> Result<OrganizationServiceRef, OrganizationFailure> {
    Ok(OrganizationServiceRef {
        namespace: b.text(v.namespace.as_str())?,
        contract_id: b.text(v.contract_id.as_str())?,
        provider_scope: scope(&v.provider_scope, b)?,
        consumer_scope: scope(&v.consumer_scope, b)?,
    })
}

pub(super) fn node(
    v: &core::OrganizationalNodeRefV1,
    b: &mut CopyBudget,
) -> Result<OrganizationNode, OrganizationFailure> {
    use core::OrganizationalNodeRefV1 as N;
    Ok(match v {
        N::Scope { scope: v } => OrganizationNode::Scope(scope(v, b)?),
        N::Organization { organization: v } => OrganizationNode::Organization(organization(v, b)?),
        N::Group { group: v } => OrganizationNode::Group(group(v, b)?),
        N::DisciplineTree { tree: v } => OrganizationNode::DisciplineTree(tree(v, b)?),
        N::Role { role: v } => OrganizationNode::Role(role(v, b)?),
        N::Occupant { occupant } => OrganizationNode::Occupant(b.text(occupant.as_str())?),
        N::Provider { provider } => OrganizationNode::Provider(b.text(provider.as_str())?),
        N::NativeResource { resource } => {
            OrganizationNode::NativeResource(b.text(resource.as_str())?)
        }
        N::Object { object } => OrganizationNode::Object(b.text(object.as_str())?),
    })
}

fn axes(
    v: &core::OrganizationalAuthorityAxesV1,
    b: &mut CopyBudget,
) -> Result<OrganizationAxes, OrganizationFailure> {
    Ok(OrganizationAxes {
        owner: b.text(v.owner.as_str())?,
        controller: b.text(v.controller.as_str())?,
        authority: b.text(v.authority.as_str())?,
        disclosure: disclosure(v.disclosure),
    })
}

fn requirement(
    v: &core::ResponsibilityEvidenceRequirementV1,
    b: &mut CopyBudget,
) -> Result<OrganizationEvidenceRequirement, OrganizationFailure> {
    Ok(OrganizationEvidenceRequirement {
        owner: b.text(v.owner.as_str())?,
        evidence_ref: b.text(v.evidence_ref.as_str())?,
    })
}

pub(super) fn attribution(
    v: &core::ResponsibilityAttributionV1,
    b: &mut CopyBudget,
) -> Result<OrganizationAttribution, OrganizationFailure> {
    Ok(OrganizationAttribution {
        source_id: b.text(v.source_id.as_str())?,
        owner: b.text(v.owner.as_str())?,
        evidence_ref: b.text(v.evidence_ref.as_str())?,
    })
}

fn answer<T, U>(
    v: &core::AttributableAnswerV1<T>,
    b: &mut CopyBudget,
    project: impl FnOnce(&T, &mut CopyBudget) -> Result<U, OrganizationFailure>,
) -> Result<ProjectedAnswer<U>, OrganizationFailure> {
    Ok(match v {
        core::AttributableAnswerV1::Supplied {
            value,
            attribution: a,
        } => ProjectedAnswer::Supplied {
            value: project(value, b)?,
            attribution: attribution(a, b)?,
        },
        core::AttributableAnswerV1::Omitted {
            reason,
            attribution: a,
        } => ProjectedAnswer::Omitted {
            reason: b.text(reason.as_str())?,
            attribution: attribution(a, b)?,
        },
    })
}

fn placement(
    v: &core::ResponsibilityPlacementV1,
    b: &mut CopyBudget,
) -> Result<OrganizationResponsibilityPlacement, OrganizationFailure> {
    Ok(match v {
        core::ResponsibilityPlacementV1::VerticalLocal { scope } => {
            OrganizationResponsibilityPlacement::VerticalLocal(b.text(scope.as_str())?)
        }
        core::ResponsibilityPlacementV1::DirectionalService { service_contract } => {
            OrganizationResponsibilityPlacement::DirectionalService(service(service_contract, b)?)
        }
    })
}

fn derived_source(
    v: &core::OrganizationalDerivedRelationSourceV1,
    b: &mut CopyBudget,
) -> Result<OrganizationDerivedSource, OrganizationFailure> {
    use core::OrganizationalDerivedRelationSourceV1 as S;
    Ok(match v {
        S::OccupantBinding {
            role: r,
            occupant,
            interval: i,
        } => OrganizationDerivedSource::OccupantBinding {
            role: role(r, b)?,
            occupant: b.text(occupant.as_str())?,
            interval: interval(i),
        },
        S::ProviderBinding {
            role: r,
            provider,
            interval: i,
        } => OrganizationDerivedSource::ProviderBinding {
            role: role(r, b)?,
            provider: b.text(provider.as_str())?,
            interval: interval(i),
        },
        S::LeadershipParticipation { topic } => {
            OrganizationDerivedSource::LeadershipParticipation(b.text(topic.as_str())?)
        }
    })
}

pub(super) fn row(
    v: Record<'_>,
    b: &mut CopyBudget,
) -> Result<OrganizationRow, OrganizationFailure> {
    Ok(match v {
        Record::Scope(v) => OrganizationRow::Scope {
            scope: scope(&v.scope, b)?,
            axes: axes(&v.axes, b)?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::Organization(v) => OrganizationRow::Organization {
            organization: organization(&v.organization, b)?,
            scope: scope(&v.scope, b)?,
            axes: axes(&v.axes, b)?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::Group(v) => OrganizationRow::Group {
            group: group(&v.group, b)?,
            organization: organization(&v.organization, b)?,
            axes: axes(&v.axes, b)?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::DisciplineTree(v) => OrganizationRow::DisciplineTree {
            tree: tree(&v.tree, b)?,
            organization: organization(&v.organization, b)?,
            axes: axes(&v.axes, b)?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::Role(v) => OrganizationRow::Role {
            role: role(&v.role, b)?,
            organization: organization(&v.organization, b)?,
            group: v.group.as_ref().map(|v| group(v, b)).transpose()?,
            discipline_tree: v.discipline_tree.as_ref().map(|v| tree(v, b)).transpose()?,
            role_class: b.text(v.role_class.as_str())?,
            axes: axes(&v.axes, b)?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::OccupantBinding(v) => OrganizationRow::OccupantBinding {
            role: role(&v.role, b)?,
            occupant: b.text(v.occupant.as_str())?,
            issuer: b.text(v.issuer.as_str())?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::ProviderBinding(v) => OrganizationRow::ProviderBinding {
            role: role(&v.role, b)?,
            provider: b.text(v.provider.as_str())?,
            issuer: b.text(v.issuer.as_str())?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::CommunicationActor(v) => OrganizationRow::CommunicationActor {
            provider: b.text(v.provider.as_str())?,
            handle: b.text(v.handle.as_str())?,
            issuer: b.text(v.issuer.as_str())?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::Leadership(v) => OrganizationRow::Leadership {
            topic: b.text(v.topic.as_str())?,
            issuer: b.text(v.issuer.as_str())?,
            disclosure: disclosure(v.disclosure),
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::Relation(v) => OrganizationRow::Relation {
            relation_id: b.text(v.relation_id.as_str())?,
            class: relation_class(v.class),
            locality: relation_locality(v.class.locality()),
            from: node(&v.from, b)?,
            to: node(&v.to, b)?,
            issuer: b.text(v.issuer.as_str())?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::Object(v) => OrganizationRow::Object {
            object_id: b.text(v.object_id.as_str())?,
            namespace: b.text(v.namespace.as_str())?,
            scope: scope(&v.scope, b)?,
            organization: organization(&v.organization, b)?,
            object_class: b.text(v.object_class.as_str())?,
            axes: axes(&v.axes, b)?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::PopulationRule(v) => OrganizationRow::PopulationRule {
            object_class: b.text(v.object_class.as_str())?,
            accountable_role_class: b.text(v.accountable_role_class.as_str())?,
            issuer: b.text(v.issuer.as_str())?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::GroupLens(v) => OrganizationRow::GroupLens {
            profile_id: b.text(v.profile_id.as_str())?,
            holder: role(&v.holder, b)?,
            alignment_relation: b.text(v.alignment_relation.as_str())?,
            lens: b.text(v.lens.as_str())?,
            review_owner: v.review_owner.as_ref().map(|v| role(v, b)).transpose()?,
            escalation_resolver: v
                .escalation_resolver
                .as_ref()
                .map(|v| role(v, b))
                .transpose()?,
            issuer: b.text(v.issuer.as_str())?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::Service(v) => OrganizationRow::Service {
            contract: OrganizationServiceRef {
                namespace: b.text(v.namespace.as_str())?,
                contract_id: b.text(v.contract_id.as_str())?,
                provider_scope: scope(&v.provider_scope, b)?,
                consumer_scope: scope(&v.consumer_scope, b)?,
            },
            contract_owner: b.text(v.contract_owner.as_str())?,
            provider_completion: requirement(&v.provider_completion.requirement, b)?,
            provider_acceptance: requirement(&v.provider_acceptance.requirement, b)?,
            consumer_acceptance: requirement(&v.consumer_acceptance.requirement, b)?,
            native_read_back: requirement(&v.native_read_back.requirement, b)?,
            pale_settlement: requirement(&v.pale_settlement.requirement, b)?,
            disclosure: disclosure(v.ceiling.disclosure),
            effect: effect(v.ceiling.effect),
            model_use_permitted: v.ceiling.model_use_permitted,
            retention_seconds: v.ceiling.retention_seconds,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::NativeProjection(v) => OrganizationRow::NativeProjection {
            projection_id: b.text(v.projection_id.as_str())?,
            subject: node(&v.subject, b)?,
            class: projection_class(v.class),
            native_owner: b.text(v.native_owner.as_str())?,
            disclosure: disclosure(v.disclosure),
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::Inspection(v) => OrganizationRow::Inspection {
            inspection_id: b.text(v.inspection_id.as_str())?,
            principal: b.text(v.principal.as_str())?,
            disclosure: disclosure(v.disclosure),
            view_provider: v
                .view_provider
                .as_ref()
                .map(|v| b.text(v.as_str()))
                .transpose()?,
        },
        Record::DerivedRelation(v) => OrganizationRow::DerivedRelation {
            source: derived_source(&v.source, b)?,
            class: derived_class(v.class),
            from: node(&v.from, b)?,
            to: node(&v.to, b)?,
            issuer: b.text(v.issuer.as_str())?,
            state: state(v.state),
            interval: interval(&v.interval),
        },
        Record::Responsibility(v) => OrganizationRow::Responsibility {
            record: Box::new(OrganizationResponsibility {
                derivation_id: b.text(v.derivation_id.as_str())?,
                lifecycle: lifecycle(v.lifecycle),
                provenance: attribution(&v.provenance, b)?,
                placement: placement_answer(&v.placement, b)?,
                provider_completion: provider_completion_answer(&v.provider_completion, b)?,
                provider_acceptance: provider_acceptance_answer(&v.provider_acceptance, b)?,
                consumer_acceptance: consumer_acceptance_answer(&v.consumer_acceptance, b)?,
                native_result: native_read_back_answer(&v.native_result, b)?,
                pale_settlement: settlement_answer(&v.pale_settlement, b)?,
            }),
        },
    })
}

pub(super) fn state(value: core::EffectiveFactStateV1) -> OrganizationFactState {
    match value {
        core::EffectiveFactStateV1::Present => OrganizationFactState::Present,
        core::EffectiveFactStateV1::Absent => OrganizationFactState::Absent,
        core::EffectiveFactStateV1::Denied => OrganizationFactState::Denied,
        core::EffectiveFactStateV1::Unknown => OrganizationFactState::Unknown,
        core::EffectiveFactStateV1::Stale => OrganizationFactState::Stale,
        core::EffectiveFactStateV1::Superseded => OrganizationFactState::Superseded,
        core::EffectiveFactStateV1::Revoked => OrganizationFactState::Revoked,
    }
}

pub(super) fn disclosure(value: core::DisclosureCeilingV1) -> OrganizationDisclosure {
    match value {
        core::DisclosureCeilingV1::Private => OrganizationDisclosure::Private,
        core::DisclosureCeilingV1::Protected => OrganizationDisclosure::Protected,
        core::DisclosureCeilingV1::Public => OrganizationDisclosure::Public,
    }
}

pub(super) fn effect(value: core::WorkspaceEffectCeilingV1) -> OrganizationEffectCeiling {
    match value {
        core::WorkspaceEffectCeilingV1::None => OrganizationEffectCeiling::None,
        core::WorkspaceEffectCeilingV1::Read => OrganizationEffectCeiling::Read,
        core::WorkspaceEffectCeilingV1::Mutate => OrganizationEffectCeiling::Mutate,
    }
}

pub(super) fn relation_class(
    value: core::OrganizationalRelationClassV1,
) -> OrganizationRelationClass {
    match value {
        core::OrganizationalRelationClassV1::Coordinates => OrganizationRelationClass::Coordinates,
        core::OrganizationalRelationClassV1::Primary => OrganizationRelationClass::Primary,
        core::OrganizationalRelationClassV1::Leader => OrganizationRelationClass::Leader,
        core::OrganizationalRelationClassV1::Supporting => OrganizationRelationClass::Supporting,
        core::OrganizationalRelationClassV1::PeopleManagement => {
            OrganizationRelationClass::PeopleManagement
        }
        core::OrganizationalRelationClassV1::RoadmapCoordination => {
            OrganizationRelationClass::RoadmapCoordination
        }
        core::OrganizationalRelationClassV1::DisciplineJudgment => {
            OrganizationRelationClass::DisciplineJudgment
        }
        core::OrganizationalRelationClassV1::Standards => OrganizationRelationClass::Standards,
        core::OrganizationalRelationClassV1::Review => OrganizationRelationClass::Review,
        core::OrganizationalRelationClassV1::BoundedBlock => {
            OrganizationRelationClass::BoundedBlock
        }
        core::OrganizationalRelationClassV1::Escalation => OrganizationRelationClass::Escalation,
        core::OrganizationalRelationClassV1::Acceptance => OrganizationRelationClass::Acceptance,
        core::OrganizationalRelationClassV1::NativePresence => {
            OrganizationRelationClass::NativePresence
        }
        core::OrganizationalRelationClassV1::Collaboration => {
            OrganizationRelationClass::Collaboration
        }
    }
}

pub(super) fn derived_class(
    value: core::OrganizationalDerivedRelationClassV1,
) -> OrganizationDerivedClass {
    match value {
        core::OrganizationalDerivedRelationClassV1::Occupancy => {
            OrganizationDerivedClass::Occupancy
        }
        core::OrganizationalDerivedRelationClassV1::ProviderPresence => {
            OrganizationDerivedClass::ProviderPresence
        }
        core::OrganizationalDerivedRelationClassV1::LeadershipParticipation => {
            OrganizationDerivedClass::LeadershipParticipation
        }
    }
}

pub(super) fn relation_locality(
    value: core::OrganizationalRelationLocalityV1,
) -> OrganizationRelationLocality {
    match value {
        core::OrganizationalRelationLocalityV1::Internal => OrganizationRelationLocality::Internal,
        core::OrganizationalRelationLocalityV1::NativeBoundary => {
            OrganizationRelationLocality::NativeBoundary
        }
    }
}

pub(super) fn projection_class(
    value: core::ResponsibilityProjectionClassV1,
) -> OrganizationProjectionClass {
    match value {
        core::ResponsibilityProjectionClassV1::AccountableWork => {
            OrganizationProjectionClass::AccountableWork
        }
        core::ResponsibilityProjectionClassV1::Communication => {
            OrganizationProjectionClass::Communication
        }
        core::ResponsibilityProjectionClassV1::Relationship => {
            OrganizationProjectionClass::Relationship
        }
        core::ResponsibilityProjectionClassV1::NativeApplication => {
            OrganizationProjectionClass::NativeApplication
        }
        core::ResponsibilityProjectionClassV1::DerivedView => {
            OrganizationProjectionClass::DerivedView
        }
        core::ResponsibilityProjectionClassV1::Context => OrganizationProjectionClass::Context,
        core::ResponsibilityProjectionClassV1::Genealogy => OrganizationProjectionClass::Genealogy,
    }
}

pub(super) fn lifecycle(
    value: core::ResponsibilityDerivationLifecycleV1,
) -> OrganizationResponsibilityLifecycle {
    match value {
        core::ResponsibilityDerivationLifecycleV1::VerifiedCurrent => {
            OrganizationResponsibilityLifecycle::VerifiedCurrent
        }
        core::ResponsibilityDerivationLifecycleV1::AcceptedTarget => {
            OrganizationResponsibilityLifecycle::AcceptedTarget
        }
        core::ResponsibilityDerivationLifecycleV1::OwnerDesiredConfiguration => {
            OrganizationResponsibilityLifecycle::OwnerDesiredConfiguration
        }
        core::ResponsibilityDerivationLifecycleV1::Candidate => {
            OrganizationResponsibilityLifecycle::Candidate
        }
        core::ResponsibilityDerivationLifecycleV1::ImplementationOpen => {
            OrganizationResponsibilityLifecycle::ImplementationOpen
        }
        core::ResponsibilityDerivationLifecycleV1::ConfigurationOpen => {
            OrganizationResponsibilityLifecycle::ConfigurationOpen
        }
        core::ResponsibilityDerivationLifecycleV1::EvidenceGated => {
            OrganizationResponsibilityLifecycle::EvidenceGated
        }
        core::ResponsibilityDerivationLifecycleV1::Historical => {
            OrganizationResponsibilityLifecycle::Historical
        }
        core::ResponsibilityDerivationLifecycleV1::Rejected => {
            OrganizationResponsibilityLifecycle::Rejected
        }
    }
}

pub(super) fn placement_answer(
    value: &core::AttributableAnswerV1<core::ResponsibilityPlacementV1>,
    budget: &mut CopyBudget,
) -> Result<OrganizationPlacementAnswer, OrganizationFailure> {
    Ok(match answer(value, budget, placement)? {
        ProjectedAnswer::Supplied { value, attribution } => {
            OrganizationPlacementAnswer::Supplied { value, attribution }
        }
        ProjectedAnswer::Omitted {
            reason,
            attribution,
        } => OrganizationPlacementAnswer::Omitted {
            reason,
            attribution,
        },
    })
}

pub(super) fn resource_answer(
    value: &core::AttributableAnswerV1<core::WorkspaceResourceId>,
    budget: &mut CopyBudget,
) -> Result<OrganizationResourceAnswer, OrganizationFailure> {
    Ok(
        match answer(value, budget, |value, b| b.text(value.as_str()))? {
            ProjectedAnswer::Supplied { value, attribution } => {
                OrganizationResourceAnswer::Supplied { value, attribution }
            }
            ProjectedAnswer::Omitted {
                reason,
                attribution,
            } => OrganizationResourceAnswer::Omitted {
                reason,
                attribution,
            },
        },
    )
}

pub(super) fn provider_completion_answer(
    value: &core::AttributableAnswerV1<core::ProviderCompletionRequirementV1>,
    budget: &mut CopyBudget,
) -> Result<OrganizationProviderCompletionAnswer, OrganizationFailure> {
    Ok(
        match answer(value, budget, |value, b| requirement(&value.requirement, b))? {
            ProjectedAnswer::Supplied { value, attribution } => {
                OrganizationProviderCompletionAnswer::Supplied { value, attribution }
            }
            ProjectedAnswer::Omitted {
                reason,
                attribution,
            } => OrganizationProviderCompletionAnswer::Omitted {
                reason,
                attribution,
            },
        },
    )
}

pub(super) fn provider_acceptance_answer(
    value: &core::AttributableAnswerV1<core::ProviderAcceptanceRequirementV1>,
    budget: &mut CopyBudget,
) -> Result<OrganizationProviderAcceptanceAnswer, OrganizationFailure> {
    Ok(
        match answer(value, budget, |value, b| requirement(&value.requirement, b))? {
            ProjectedAnswer::Supplied { value, attribution } => {
                OrganizationProviderAcceptanceAnswer::Supplied { value, attribution }
            }
            ProjectedAnswer::Omitted {
                reason,
                attribution,
            } => OrganizationProviderAcceptanceAnswer::Omitted {
                reason,
                attribution,
            },
        },
    )
}

pub(super) fn consumer_acceptance_answer(
    value: &core::AttributableAnswerV1<core::ConsumerAcceptanceRequirementV1>,
    budget: &mut CopyBudget,
) -> Result<OrganizationConsumerAcceptanceAnswer, OrganizationFailure> {
    Ok(
        match answer(value, budget, |value, b| requirement(&value.requirement, b))? {
            ProjectedAnswer::Supplied { value, attribution } => {
                OrganizationConsumerAcceptanceAnswer::Supplied { value, attribution }
            }
            ProjectedAnswer::Omitted {
                reason,
                attribution,
            } => OrganizationConsumerAcceptanceAnswer::Omitted {
                reason,
                attribution,
            },
        },
    )
}

pub(super) fn native_read_back_answer(
    value: &core::AttributableAnswerV1<core::NativeReadBackRequirementV1>,
    budget: &mut CopyBudget,
) -> Result<OrganizationNativeReadBackAnswer, OrganizationFailure> {
    Ok(
        match answer(value, budget, |value, b| requirement(&value.requirement, b))? {
            ProjectedAnswer::Supplied { value, attribution } => {
                OrganizationNativeReadBackAnswer::Supplied { value, attribution }
            }
            ProjectedAnswer::Omitted {
                reason,
                attribution,
            } => OrganizationNativeReadBackAnswer::Omitted {
                reason,
                attribution,
            },
        },
    )
}

pub(super) fn settlement_answer(
    value: &core::AttributableAnswerV1<core::PaleSettlementRequirementV1>,
    budget: &mut CopyBudget,
) -> Result<OrganizationSettlementAnswer, OrganizationFailure> {
    Ok(
        match answer(value, budget, |value, b| requirement(&value.requirement, b))? {
            ProjectedAnswer::Supplied { value, attribution } => {
                OrganizationSettlementAnswer::Supplied { value, attribution }
            }
            ProjectedAnswer::Omitted {
                reason,
                attribution,
            } => OrganizationSettlementAnswer::Omitted {
                reason,
                attribution,
            },
        },
    )
}
