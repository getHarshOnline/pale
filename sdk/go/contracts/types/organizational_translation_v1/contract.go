// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package organizational_translation_v1 contains the generated organizational-translation.v1 data model.
package organizational_translation_v1

import (
	"encoding/json"
	"fmt"
)

type CommunicationActorBindingV1 struct {
	// Handle corresponds to the JSON schema field "handle".
	Handle CommunicationActorHandleID `json:"handle"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Issuer corresponds to the JSON schema field "issuer".
	Issuer WorkspaceResourceID `json:"issuer"`

	// LocalResolutions corresponds to the JSON schema field "local_resolutions".
	LocalResolutions []CommunicationActorLocalResolutionV1 `json:"local_resolutions"`

	// Provider corresponds to the JSON schema field "provider".
	Provider OrganizationalProviderID `json:"provider"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`
}

type CommunicationActorHandleID string

type CommunicationActorLocalResolutionV1 struct {
	// AuthorizedClosure corresponds to the JSON schema field "authorized_closure".
	AuthorizedClosure []OrganizationalNodeRefV1 `json:"authorized_closure"`

	// LocalNode corresponds to the JSON schema field "local_node".
	LocalNode OrganizationalNodeRefV1 `json:"local_node"`
}

type ConsumerAcceptanceRequirementV1 struct {
	// Requirement corresponds to the JSON schema field "requirement".
	Requirement ResponsibilityEvidenceRequirementV1 `json:"requirement"`
}

type DirectionalServiceContractID string

type DirectionalServiceContractV1 struct {
	// Assignments corresponds to the JSON schema field "assignments".
	Assignments []ServiceAssignmentV1 `json:"assignments"`

	// Ceiling corresponds to the JSON schema field "ceiling".
	Ceiling WorkspacePolicyCeilingV1 `json:"ceiling"`

	// ConsumerAcceptance corresponds to the JSON schema field "consumer_acceptance".
	ConsumerAcceptance ConsumerAcceptanceRequirementV1 `json:"consumer_acceptance"`

	// ConsumerScope corresponds to the JSON schema field "consumer_scope".
	ConsumerScope LogicalScopeRefV1 `json:"consumer_scope"`

	// ContractID corresponds to the JSON schema field "contract_id".
	ContractID DirectionalServiceContractID `json:"contract_id"`

	// ContractOwner corresponds to the JSON schema field "contract_owner".
	ContractOwner WorkspaceResourceID `json:"contract_owner"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// NativeReadBack corresponds to the JSON schema field "native_read_back".
	NativeReadBack NativeReadBackRequirementV1 `json:"native_read_back"`

	// PaleSettlement corresponds to the JSON schema field "pale_settlement".
	PaleSettlement PaleSettlementRequirementV1 `json:"pale_settlement"`

	// ProviderAcceptance corresponds to the JSON schema field "provider_acceptance".
	ProviderAcceptance ProviderAcceptanceRequirementV1 `json:"provider_acceptance"`

	// ProviderCompletion corresponds to the JSON schema field "provider_completion".
	ProviderCompletion ProviderCompletionRequirementV1 `json:"provider_completion"`

	// ProviderScope corresponds to the JSON schema field "provider_scope".
	ProviderScope LogicalScopeRefV1 `json:"provider_scope"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`
}

type DisciplineTreeID string

type DisciplineTreeRefV1 struct {
	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// TreeID corresponds to the JSON schema field "tree_id".
	TreeID DisciplineTreeID `json:"tree_id"`
}

type DisciplineTreeV1 struct {
	// Axes corresponds to the JSON schema field "axes".
	Axes OrganizationalAuthorityAxesV1 `json:"axes"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Organization corresponds to the JSON schema field "organization".
	Organization LogicalOrganizationRefV1 `json:"organization"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`

	// Tree corresponds to the JSON schema field "tree".
	Tree DisciplineTreeRefV1 `json:"tree"`
}

type DisclosureCeilingV1 string

const DisclosureCeilingV1Private DisclosureCeilingV1 = "private"
const DisclosureCeilingV1Protected DisclosureCeilingV1 = "protected"
const DisclosureCeilingV1Public DisclosureCeilingV1 = "public"

type EffectiveFactStateV1 string

const EffectiveFactStateV1Absent EffectiveFactStateV1 = "absent"
const EffectiveFactStateV1Denied EffectiveFactStateV1 = "denied"
const EffectiveFactStateV1Present EffectiveFactStateV1 = "present"
const EffectiveFactStateV1Revoked EffectiveFactStateV1 = "revoked"
const EffectiveFactStateV1Stale EffectiveFactStateV1 = "stale"
const EffectiveFactStateV1Superseded EffectiveFactStateV1 = "superseded"
const EffectiveFactStateV1Unknown EffectiveFactStateV1 = "unknown"

type EffectiveIntervalV1 struct {
	// EndExclusive corresponds to the JSON schema field "end_exclusive".
	EndExclusive int `json:"end_exclusive"`

	// Start corresponds to the JSON schema field "start".
	Start int `json:"start"`
}

type GroupLensAuthorityProfileV1 struct {
	// AlignmentRelation corresponds to the JSON schema field "alignment_relation".
	AlignmentRelation OrganizationalRelationID `json:"alignment_relation"`

	// BlockConditions corresponds to the JSON schema field "block_conditions".
	BlockConditions []OrganizationalConditionID `json:"block_conditions"`

	// EscalationResolver corresponds to the JSON schema field "escalation_resolver".
	EscalationResolver *OrganizationalRoleRefV1 `json:"escalation_resolver,omitempty,omitzero"`

	// Holder corresponds to the JSON schema field "holder".
	Holder OrganizationalRoleRefV1 `json:"holder"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Issuer corresponds to the JSON schema field "issuer".
	Issuer WorkspaceResourceID `json:"issuer"`

	// Lens corresponds to the JSON schema field "lens".
	Lens OrganizationalLensID `json:"lens"`

	// ProfileID corresponds to the JSON schema field "profile_id".
	ProfileID GroupLensProfileID `json:"profile_id"`

	// ReviewOwner corresponds to the JSON schema field "review_owner".
	ReviewOwner *OrganizationalRoleRefV1 `json:"review_owner,omitempty,omitzero"`

	// Rights corresponds to the JSON schema field "rights".
	Rights []GroupLensRightV1 `json:"rights"`

	// Standards corresponds to the JSON schema field "standards".
	Standards []WorkspaceResourceID `json:"standards"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`
}

type GroupLensProfileID string

type GroupLensRightV1 string

const GroupLensRightV1BoundedBlock GroupLensRightV1 = "bounded_block"
const GroupLensRightV1Escalation GroupLensRightV1 = "escalation"
const GroupLensRightV1Review GroupLensRightV1 = "review"
const GroupLensRightV1Standards GroupLensRightV1 = "standards"

type InspectionCoordinateClassV1 string

const InspectionCoordinateClassV1AcceptanceFrontier InspectionCoordinateClassV1 = "acceptance_frontier"
const InspectionCoordinateClassV1AtomicObject InspectionCoordinateClassV1 = "atomic_object"
const InspectionCoordinateClassV1DirectionalService InspectionCoordinateClassV1 = "directional_service"
const InspectionCoordinateClassV1EffectiveState InspectionCoordinateClassV1 = "effective_state"
const InspectionCoordinateClassV1GroupLens InspectionCoordinateClassV1 = "group_lens"
const InspectionCoordinateClassV1GroupTree InspectionCoordinateClassV1 = "group_tree"
const InspectionCoordinateClassV1NativeProjection InspectionCoordinateClassV1 = "native_projection"
const InspectionCoordinateClassV1RoleBinding InspectionCoordinateClassV1 = "role_binding"
const InspectionCoordinateClassV1ScopeOrganization InspectionCoordinateClassV1 = "scope_organization"

type LeadershipParticipationV1 struct {
	// AffectedScopes corresponds to the JSON schema field "affected_scopes".
	AffectedScopes []LogicalScopeRefV1 `json:"affected_scopes"`

	// Disclosure corresponds to the JSON schema field "disclosure".
	Disclosure DisclosureCeilingV1 `json:"disclosure"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Issuer corresponds to the JSON schema field "issuer".
	Issuer WorkspaceResourceID `json:"issuer"`

	// Participants corresponds to the JSON schema field "participants".
	Participants []OrganizationalRoleRefV1 `json:"participants"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`

	// Topic corresponds to the JSON schema field "topic".
	Topic LeadershipTopicID `json:"topic"`
}

type LeadershipTopicID string

type LogicalOrganizationID string

type LogicalOrganizationRefV1 struct {
	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// OrganizationID corresponds to the JSON schema field "organization_id".
	OrganizationID LogicalOrganizationID `json:"organization_id"`
}

type LogicalOrganizationV1 struct {
	// Axes corresponds to the JSON schema field "axes".
	Axes OrganizationalAuthorityAxesV1 `json:"axes"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Organization corresponds to the JSON schema field "organization".
	Organization LogicalOrganizationRefV1 `json:"organization"`

	// Scope corresponds to the JSON schema field "scope".
	Scope LogicalScopeRefV1 `json:"scope"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`
}

type LogicalScopeID string

type LogicalScopeRefV1 struct {
	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// ScopeID corresponds to the JSON schema field "scope_id".
	ScopeID LogicalScopeID `json:"scope_id"`
}

type LogicalScopeV1 struct {
	// Axes corresponds to the JSON schema field "axes".
	Axes OrganizationalAuthorityAxesV1 `json:"axes"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Scope corresponds to the JSON schema field "scope".
	Scope LogicalScopeRefV1 `json:"scope"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`
}

type NativeReadBackRequirementV1 struct {
	// Requirement corresponds to the JSON schema field "requirement".
	Requirement ResponsibilityEvidenceRequirementV1 `json:"requirement"`
}

type ObjectRolePopulationRuleV1 struct {
	// AccountableRoleClass corresponds to the JSON schema field
	// "accountable_role_class".
	AccountableRoleClass OrganizationalRoleClassID `json:"accountable_role_class"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Issuer corresponds to the JSON schema field "issuer".
	Issuer WorkspaceResourceID `json:"issuer"`

	// ObjectClass corresponds to the JSON schema field "object_class".
	ObjectClass OrganizationalObjectClassID `json:"object_class"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`
}

type OrganizationalAuthorityAxesV1 struct {
	// Authority corresponds to the JSON schema field "authority".
	Authority WorkspaceAuthorityScopeID `json:"authority"`

	// Controller corresponds to the JSON schema field "controller".
	Controller WorkspaceControllerScopeID `json:"controller"`

	// Disclosure corresponds to the JSON schema field "disclosure".
	Disclosure DisclosureCeilingV1 `json:"disclosure"`

	// Owner corresponds to the JSON schema field "owner".
	Owner WorkspaceResourceID `json:"owner"`

	// Resources corresponds to the JSON schema field "resources".
	Resources []WorkspaceResourceID `json:"resources"`
}

type OrganizationalConditionID string

type OrganizationalGroupID string

type OrganizationalGroupRefV1 struct {
	// GroupID corresponds to the JSON schema field "group_id".
	GroupID OrganizationalGroupID `json:"group_id"`

	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`
}

type OrganizationalGroupV1 struct {
	// Axes corresponds to the JSON schema field "axes".
	Axes OrganizationalAuthorityAxesV1 `json:"axes"`

	// Group corresponds to the JSON schema field "group".
	Group OrganizationalGroupRefV1 `json:"group"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Organization corresponds to the JSON schema field "organization".
	Organization LogicalOrganizationRefV1 `json:"organization"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`
}

type OrganizationalInspectionID string

type OrganizationalInspectionV1 struct {
	// Coordinates corresponds to the JSON schema field "coordinates".
	Coordinates []InspectionCoordinateClassV1 `json:"coordinates"`

	// Disclosure corresponds to the JSON schema field "disclosure".
	Disclosure DisclosureCeilingV1 `json:"disclosure"`

	// InspectionID corresponds to the JSON schema field "inspection_id".
	InspectionID OrganizationalInspectionID `json:"inspection_id"`

	// Principal corresponds to the JSON schema field "principal".
	Principal WorkspaceResourceID `json:"principal"`

	// States corresponds to the JSON schema field "states".
	States []EffectiveFactStateV1 `json:"states"`

	// ViewProvider corresponds to the JSON schema field "view_provider".
	ViewProvider *WorkspaceResourceID `json:"view_provider,omitempty,omitzero"`
}

type OrganizationalLensID string

type OrganizationalNativeProjectionV1 struct {
	// Class corresponds to the JSON schema field "class".
	Class ResponsibilityProjectionClassV1 `json:"class"`

	// Disclosure corresponds to the JSON schema field "disclosure".
	Disclosure DisclosureCeilingV1 `json:"disclosure"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// NativeOwner corresponds to the JSON schema field "native_owner".
	NativeOwner WorkspaceResourceID `json:"native_owner"`

	// NativeRefs corresponds to the JSON schema field "native_refs".
	NativeRefs []WorkspaceResourceID `json:"native_refs"`

	// ProjectionID corresponds to the JSON schema field "projection_id".
	ProjectionID OrganizationalProjectionID `json:"projection_id"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`

	// Subject corresponds to the JSON schema field "subject".
	Subject OrganizationalNodeRefV1 `json:"subject"`
}

type OrganizationalNodeRefDisciplineTreeV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Tree corresponds to the JSON schema field "tree".
	Tree DisciplineTreeRefV1 `json:"tree"`
}

type OrganizationalNodeRefGroupV1 struct {
	// Group corresponds to the JSON schema field "group".
	Group OrganizationalGroupRefV1 `json:"group"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type OrganizationalNodeRefNativeResourceV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Resource corresponds to the JSON schema field "resource".
	Resource WorkspaceResourceID `json:"resource"`
}

type OrganizationalNodeRefObjectV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Object corresponds to the JSON schema field "object".
	Object OrganizationalObjectID `json:"object"`
}

type OrganizationalNodeRefOccupantV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Occupant corresponds to the JSON schema field "occupant".
	Occupant OrganizationalOccupantID `json:"occupant"`
}

type OrganizationalNodeRefOrganizationV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Organization corresponds to the JSON schema field "organization".
	Organization LogicalOrganizationRefV1 `json:"organization"`
}

type OrganizationalNodeRefProviderV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Provider corresponds to the JSON schema field "provider".
	Provider OrganizationalProviderID `json:"provider"`
}

type OrganizationalNodeRefRoleV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Role corresponds to the JSON schema field "role".
	Role OrganizationalRoleRefV1 `json:"role"`
}

type OrganizationalNodeRefScopeV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Scope corresponds to the JSON schema field "scope".
	Scope LogicalScopeRefV1 `json:"scope"`
}

type OrganizationalObjectClassID string

type OrganizationalObjectID string

type OrganizationalObjectV1 struct {
	// Axes corresponds to the JSON schema field "axes".
	Axes OrganizationalAuthorityAxesV1 `json:"axes"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// ObjectClass corresponds to the JSON schema field "object_class".
	ObjectClass OrganizationalObjectClassID `json:"object_class"`

	// ObjectID corresponds to the JSON schema field "object_id".
	ObjectID OrganizationalObjectID `json:"object_id"`

	// Organization corresponds to the JSON schema field "organization".
	Organization LogicalOrganizationRefV1 `json:"organization"`

	// Scope corresponds to the JSON schema field "scope".
	Scope LogicalScopeRefV1 `json:"scope"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`
}

type OrganizationalOccupantBindingV1 struct {
	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Issuer corresponds to the JSON schema field "issuer".
	Issuer WorkspaceResourceID `json:"issuer"`

	// Occupant corresponds to the JSON schema field "occupant".
	Occupant OrganizationalOccupantID `json:"occupant"`

	// Role corresponds to the JSON schema field "role".
	Role OrganizationalRoleRefV1 `json:"role"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`
}

type OrganizationalOccupantID string

type OrganizationalProjectionID string

type OrganizationalProviderBindingV1 struct {
	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Issuer corresponds to the JSON schema field "issuer".
	Issuer WorkspaceResourceID `json:"issuer"`

	// Provider corresponds to the JSON schema field "provider".
	Provider OrganizationalProviderID `json:"provider"`

	// Role corresponds to the JSON schema field "role".
	Role OrganizationalRoleRefV1 `json:"role"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`
}

type OrganizationalProviderID string

type OrganizationalRelationClassV1 string

const OrganizationalRelationClassV1Acceptance OrganizationalRelationClassV1 = "acceptance"
const OrganizationalRelationClassV1BoundedBlock OrganizationalRelationClassV1 = "bounded_block"
const OrganizationalRelationClassV1Collaboration OrganizationalRelationClassV1 = "collaboration"
const OrganizationalRelationClassV1Coordinates OrganizationalRelationClassV1 = "coordinates"
const OrganizationalRelationClassV1DisciplineJudgment OrganizationalRelationClassV1 = "discipline_judgment"
const OrganizationalRelationClassV1Escalation OrganizationalRelationClassV1 = "escalation"
const OrganizationalRelationClassV1Leader OrganizationalRelationClassV1 = "leader"
const OrganizationalRelationClassV1NativePresence OrganizationalRelationClassV1 = "native_presence"
const OrganizationalRelationClassV1PeopleManagement OrganizationalRelationClassV1 = "people_management"
const OrganizationalRelationClassV1Primary OrganizationalRelationClassV1 = "primary"
const OrganizationalRelationClassV1Review OrganizationalRelationClassV1 = "review"
const OrganizationalRelationClassV1RoadmapCoordination OrganizationalRelationClassV1 = "roadmap_coordination"
const OrganizationalRelationClassV1Standards OrganizationalRelationClassV1 = "standards"
const OrganizationalRelationClassV1Supporting OrganizationalRelationClassV1 = "supporting"

type OrganizationalRelationID string

type OrganizationalRelationV1 struct {
	// Class corresponds to the JSON schema field "class".
	Class OrganizationalRelationClassV1 `json:"class"`

	// From corresponds to the JSON schema field "from".
	From OrganizationalNodeRefV1 `json:"from"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Issuer corresponds to the JSON schema field "issuer".
	Issuer WorkspaceResourceID `json:"issuer"`

	// RelationID corresponds to the JSON schema field "relation_id".
	RelationID OrganizationalRelationID `json:"relation_id"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`

	// To corresponds to the JSON schema field "to".
	To OrganizationalNodeRefV1 `json:"to"`
}

type OrganizationalRoleClassID string

type OrganizationalRoleID string

type OrganizationalRoleRefV1 struct {
	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// RoleID corresponds to the JSON schema field "role_id".
	RoleID OrganizationalRoleID `json:"role_id"`
}

type OrganizationalRoleV1 struct {
	// Axes corresponds to the JSON schema field "axes".
	Axes OrganizationalAuthorityAxesV1 `json:"axes"`

	// DisciplineTree corresponds to the JSON schema field "discipline_tree".
	DisciplineTree *DisciplineTreeRefV1 `json:"discipline_tree,omitempty,omitzero"`

	// Group corresponds to the JSON schema field "group".
	Group *OrganizationalGroupRefV1 `json:"group,omitempty,omitzero"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Organization corresponds to the JSON schema field "organization".
	Organization LogicalOrganizationRefV1 `json:"organization"`

	// Role corresponds to the JSON schema field "role".
	Role OrganizationalRoleRefV1 `json:"role"`

	// RoleClass corresponds to the JSON schema field "role_class".
	RoleClass OrganizationalRoleClassID `json:"role_class"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`
}

type OrganizationalTranslationV1 struct {
	// CommunicationActorBindings corresponds to the JSON schema field
	// "communication_actor_bindings".
	CommunicationActorBindings []CommunicationActorBindingV1 `json:"communication_actor_bindings"`

	// DisciplineTrees corresponds to the JSON schema field "discipline_trees".
	DisciplineTrees []DisciplineTreeV1 `json:"discipline_trees"`

	// GroupLenses corresponds to the JSON schema field "group_lenses".
	GroupLenses []GroupLensAuthorityProfileV1 `json:"group_lenses"`

	// Groups corresponds to the JSON schema field "groups".
	Groups []OrganizationalGroupV1 `json:"groups"`

	// Inspection corresponds to the JSON schema field "inspection".
	Inspection *OrganizationalInspectionV1 `json:"inspection,omitempty,omitzero"`

	// Leadership corresponds to the JSON schema field "leadership".
	Leadership []LeadershipParticipationV1 `json:"leadership"`

	// NativeProjections corresponds to the JSON schema field "native_projections".
	NativeProjections []OrganizationalNativeProjectionV1 `json:"native_projections"`

	// Objects corresponds to the JSON schema field "objects".
	Objects []OrganizationalObjectV1 `json:"objects"`

	// OccupantBindings corresponds to the JSON schema field "occupant_bindings".
	OccupantBindings []OrganizationalOccupantBindingV1 `json:"occupant_bindings"`

	// Organizations corresponds to the JSON schema field "organizations".
	Organizations []LogicalOrganizationV1 `json:"organizations"`

	// PopulationRules corresponds to the JSON schema field "population_rules".
	PopulationRules []ObjectRolePopulationRuleV1 `json:"population_rules"`

	// ProviderBindings corresponds to the JSON schema field "provider_bindings".
	ProviderBindings []OrganizationalProviderBindingV1 `json:"provider_bindings"`

	// Relations corresponds to the JSON schema field "relations".
	Relations []OrganizationalRelationV1 `json:"relations"`

	// Roles corresponds to the JSON schema field "roles".
	Roles []OrganizationalRoleV1 `json:"roles"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// Scopes corresponds to the JSON schema field "scopes".
	Scopes []LogicalScopeV1 `json:"scopes"`

	// Services corresponds to the JSON schema field "services".
	Services []DirectionalServiceContractV1 `json:"services"`
}

type PaleSettlementRequirementV1 struct {
	// Requirement corresponds to the JSON schema field "requirement".
	Requirement ResponsibilityEvidenceRequirementV1 `json:"requirement"`
}

type ProviderAcceptanceRequirementV1 struct {
	// Requirement corresponds to the JSON schema field "requirement".
	Requirement ResponsibilityEvidenceRequirementV1 `json:"requirement"`
}

type ProviderCompletionRequirementV1 struct {
	// Requirement corresponds to the JSON schema field "requirement".
	Requirement ResponsibilityEvidenceRequirementV1 `json:"requirement"`
}

type ResponsibilityEvidenceRequirementV1 struct {
	// EvidenceRef corresponds to the JSON schema field "evidence_ref".
	EvidenceRef WorkspaceResourceID `json:"evidence_ref"`

	// Owner corresponds to the JSON schema field "owner".
	Owner WorkspaceResourceID `json:"owner"`
}

type ResponsibilityProjectionClassV1 string

const ResponsibilityProjectionClassV1AccountableWork ResponsibilityProjectionClassV1 = "accountable_work"
const ResponsibilityProjectionClassV1Communication ResponsibilityProjectionClassV1 = "communication"
const ResponsibilityProjectionClassV1Context ResponsibilityProjectionClassV1 = "context"
const ResponsibilityProjectionClassV1DerivedView ResponsibilityProjectionClassV1 = "derived_view"
const ResponsibilityProjectionClassV1Genealogy ResponsibilityProjectionClassV1 = "genealogy"
const ResponsibilityProjectionClassV1NativeApplication ResponsibilityProjectionClassV1 = "native_application"
const ResponsibilityProjectionClassV1Relationship ResponsibilityProjectionClassV1 = "relationship"

type ServiceAccountID string

type ServiceAssignmentV1 struct {
	// Account corresponds to the JSON schema field "account".
	Account ServiceAccountID `json:"account"`

	// Interval corresponds to the JSON schema field "interval".
	Interval EffectiveIntervalV1 `json:"interval"`

	// Mandate corresponds to the JSON schema field "mandate".
	Mandate ServiceMandateID `json:"mandate"`

	// Pods corresponds to the JSON schema field "pods".
	Pods []ServicePodID `json:"pods"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`

	// Stewards corresponds to the JSON schema field "stewards".
	Stewards []ServiceStewardID `json:"stewards"`
}

type ServiceMandateID string

type ServicePodID string

type ServiceStewardID string

type WorkspaceAuthorityCeilingV1 struct {
	// Permitted corresponds to the JSON schema field "permitted".
	Permitted []WorkspaceAuthorityScopeID `json:"permitted"`
}

type WorkspaceAuthorityScopeID string

type WorkspaceControllerCeilingV1 struct {
	// Permitted corresponds to the JSON schema field "permitted".
	Permitted []WorkspaceControllerScopeID `json:"permitted"`
}

type WorkspaceControllerScopeID string

type WorkspaceEffectCeilingV1 string

const WorkspaceEffectCeilingV1Mutate WorkspaceEffectCeilingV1 = "mutate"
const WorkspaceEffectCeilingV1None WorkspaceEffectCeilingV1 = "none"
const WorkspaceEffectCeilingV1Read WorkspaceEffectCeilingV1 = "read"

type WorkspaceNamespaceID string

type WorkspacePolicyCeilingV1 struct {
	// Authorities corresponds to the JSON schema field "authorities".
	Authorities WorkspaceAuthorityCeilingV1 `json:"authorities"`

	// Controllers corresponds to the JSON schema field "controllers".
	Controllers WorkspaceControllerCeilingV1 `json:"controllers"`

	// Disclosure corresponds to the JSON schema field "disclosure".
	Disclosure DisclosureCeilingV1 `json:"disclosure"`

	// Effect corresponds to the JSON schema field "effect".
	Effect WorkspaceEffectCeilingV1 `json:"effect"`

	// ModelUsePermitted corresponds to the JSON schema field "model_use_permitted".
	ModelUsePermitted bool `json:"model_use_permitted"`

	// RetentionSeconds corresponds to the JSON schema field "retention_seconds".
	RetentionSeconds int `json:"retention_seconds"`

	// Rights corresponds to the JSON schema field "rights".
	Rights WorkspaceRightsCeilingV1 `json:"rights"`
}

type WorkspaceResourceID string

type WorkspaceRightID string

type WorkspaceRightsCeilingV1 struct {
	// Permitted corresponds to the JSON schema field "permitted".
	Permitted []WorkspaceRightID `json:"permitted"`
}

type OrganizationalNodeRefV1 struct {
	Scope          *OrganizationalNodeRefScopeV1          `json:"-"`
	Organization   *OrganizationalNodeRefOrganizationV1   `json:"-"`
	Group          *OrganizationalNodeRefGroupV1          `json:"-"`
	DisciplineTree *OrganizationalNodeRefDisciplineTreeV1 `json:"-"`
	Role           *OrganizationalNodeRefRoleV1           `json:"-"`
	Occupant       *OrganizationalNodeRefOccupantV1       `json:"-"`
	Provider       *OrganizationalNodeRefProviderV1       `json:"-"`
	NativeResource *OrganizationalNodeRefNativeResourceV1 `json:"-"`
	Object         *OrganizationalNodeRefObjectV1         `json:"-"`
}

func (value OrganizationalNodeRefV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Scope != nil {
		if value.Scope.Kind != "scope" {
			return nil, fmt.Errorf("OrganizationalNodeRefV1: selected variant has discriminator %q", value.Scope.Kind)
		}
		candidate, err := json.Marshal(value.Scope)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Organization != nil {
		if value.Organization.Kind != "organization" {
			return nil, fmt.Errorf("OrganizationalNodeRefV1: selected variant has discriminator %q", value.Organization.Kind)
		}
		candidate, err := json.Marshal(value.Organization)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Group != nil {
		if value.Group.Kind != "group" {
			return nil, fmt.Errorf("OrganizationalNodeRefV1: selected variant has discriminator %q", value.Group.Kind)
		}
		candidate, err := json.Marshal(value.Group)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.DisciplineTree != nil {
		if value.DisciplineTree.Kind != "discipline_tree" {
			return nil, fmt.Errorf("OrganizationalNodeRefV1: selected variant has discriminator %q", value.DisciplineTree.Kind)
		}
		candidate, err := json.Marshal(value.DisciplineTree)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Role != nil {
		if value.Role.Kind != "role" {
			return nil, fmt.Errorf("OrganizationalNodeRefV1: selected variant has discriminator %q", value.Role.Kind)
		}
		candidate, err := json.Marshal(value.Role)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Occupant != nil {
		if value.Occupant.Kind != "occupant" {
			return nil, fmt.Errorf("OrganizationalNodeRefV1: selected variant has discriminator %q", value.Occupant.Kind)
		}
		candidate, err := json.Marshal(value.Occupant)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Provider != nil {
		if value.Provider.Kind != "provider" {
			return nil, fmt.Errorf("OrganizationalNodeRefV1: selected variant has discriminator %q", value.Provider.Kind)
		}
		candidate, err := json.Marshal(value.Provider)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NativeResource != nil {
		if value.NativeResource.Kind != "native_resource" {
			return nil, fmt.Errorf("OrganizationalNodeRefV1: selected variant has discriminator %q", value.NativeResource.Kind)
		}
		candidate, err := json.Marshal(value.NativeResource)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Object != nil {
		if value.Object.Kind != "object" {
			return nil, fmt.Errorf("OrganizationalNodeRefV1: selected variant has discriminator %q", value.Object.Kind)
		}
		candidate, err := json.Marshal(value.Object)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("OrganizationalNodeRefV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *OrganizationalNodeRefV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = OrganizationalNodeRefV1{}
	switch discriminator.Kind {
	case "scope":
		var variant OrganizationalNodeRefScopeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Scope = &variant
	case "organization":
		var variant OrganizationalNodeRefOrganizationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Organization = &variant
	case "group":
		var variant OrganizationalNodeRefGroupV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Group = &variant
	case "discipline_tree":
		var variant OrganizationalNodeRefDisciplineTreeV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.DisciplineTree = &variant
	case "role":
		var variant OrganizationalNodeRefRoleV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Role = &variant
	case "occupant":
		var variant OrganizationalNodeRefOccupantV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Occupant = &variant
	case "provider":
		var variant OrganizationalNodeRefProviderV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Provider = &variant
	case "native_resource":
		var variant OrganizationalNodeRefNativeResourceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NativeResource = &variant
	case "object":
		var variant OrganizationalNodeRefObjectV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Object = &variant
	default:
		return fmt.Errorf("OrganizationalNodeRefV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
