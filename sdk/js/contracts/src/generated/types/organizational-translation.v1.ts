/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type CommunicationActorHandleId = string;
export type WorkspaceResourceId = string;
export type OrganizationalNodeRefV1 =
  | {
      kind: "scope";
      scope: LogicalScopeRefV1;
    }
  | {
      kind: "organization";
      organization: LogicalOrganizationRefV1;
    }
  | {
      group: OrganizationalGroupRefV1;
      kind: "group";
    }
  | {
      kind: "discipline_tree";
      tree: DisciplineTreeRefV1;
    }
  | {
      kind: "role";
      role: OrganizationalRoleRefV1;
    }
  | {
      kind: "occupant";
      occupant: OrganizationalOccupantId;
    }
  | {
      kind: "provider";
      provider: OrganizationalProviderId;
    }
  | {
      kind: "native_resource";
      resource: WorkspaceResourceId;
    }
  | {
      kind: "object";
      object: OrganizationalObjectId;
    };
export type WorkspaceNamespaceId = string;
export type LogicalScopeId = string;
export type LogicalOrganizationId = string;
export type OrganizationalGroupId = string;
export type DisciplineTreeId = string;
export type OrganizationalRoleId = string;
export type OrganizationalOccupantId = string;
export type OrganizationalProviderId = string;
export type OrganizationalObjectId = string;
export type EffectiveFactStateV1 = "present" | "absent" | "denied" | "unknown" | "stale" | "superseded" | "revoked";
export type WorkspaceAuthorityScopeId = string;
export type WorkspaceControllerScopeId = string;
export type DisclosureCeilingV1 = "private" | "protected" | "public";
export type OrganizationalRelationId = string;
export type OrganizationalConditionId = string;
export type OrganizationalLensId = string;
export type GroupLensProfileId = string;
export type GroupLensRightV1 = "standards" | "review" | "bounded_block" | "escalation";
export type InspectionCoordinateClassV1 =
  | "scope_organization"
  | "group_tree"
  | "role_binding"
  | "atomic_object"
  | "group_lens"
  | "directional_service"
  | "acceptance_frontier"
  | "native_projection"
  | "effective_state";
export type OrganizationalInspectionId = string;
export type LeadershipTopicId = string;
export type ResponsibilityProjectionClassV1 =
  | "accountable_work"
  | "communication"
  | "relationship"
  | "native_application"
  | "derived_view"
  | "context"
  | "genealogy";
export type OrganizationalProjectionId = string;
export type OrganizationalObjectClassId = string;
export type OrganizationalRoleClassId = string;
export type OrganizationalRelationClassV1 =
  | "coordinates"
  | "primary"
  | "leader"
  | "supporting"
  | "people_management"
  | "roadmap_coordination"
  | "discipline_judgment"
  | "standards"
  | "review"
  | "bounded_block"
  | "escalation"
  | "acceptance"
  | "native_presence"
  | "collaboration";
export type ServiceAccountId = string;
export type ServiceMandateId = string;
export type ServicePodId = string;
export type ServiceStewardId = string;
export type WorkspaceEffectCeilingV1 = "none" | "read" | "mutate";
export type WorkspaceRightId = string;
export type DirectionalServiceContractId = string;

export interface OrganizationalTranslationV1 {
  /**
   * @maxItems 1024
   */
  communication_actor_bindings: CommunicationActorBindingV1[];
  /**
   * @maxItems 1024
   */
  discipline_trees: DisciplineTreeV1[];
  /**
   * @maxItems 1024
   */
  group_lenses: GroupLensAuthorityProfileV1[];
  /**
   * @maxItems 1024
   */
  groups: OrganizationalGroupV1[];
  inspection?: OrganizationalInspectionV1 | null;
  /**
   * @maxItems 1024
   */
  leadership: LeadershipParticipationV1[];
  /**
   * @maxItems 1024
   */
  native_projections: OrganizationalNativeProjectionV1[];
  /**
   * @maxItems 1024
   */
  objects: OrganizationalObjectV1[];
  /**
   * @maxItems 1024
   */
  occupant_bindings: OrganizationalOccupantBindingV1[];
  /**
   * @maxItems 1024
   */
  organizations: LogicalOrganizationV1[];
  /**
   * @maxItems 1024
   */
  population_rules: ObjectRolePopulationRuleV1[];
  /**
   * @maxItems 1024
   */
  provider_bindings: OrganizationalProviderBindingV1[];
  /**
   * @maxItems 4096
   */
  relations: OrganizationalRelationV1[];
  /**
   * @maxItems 1024
   */
  roles: OrganizationalRoleV1[];
  schema: "pale/organizational-translation.v1";
  schema_version: 1;
  /**
   * @maxItems 1024
   */
  scopes: LogicalScopeV1[];
  /**
   * @maxItems 1024
   */
  services: DirectionalServiceContractV1[];
}
export interface CommunicationActorBindingV1 {
  handle: CommunicationActorHandleId;
  interval: EffectiveIntervalV1;
  issuer: WorkspaceResourceId;
  /**
   * @maxItems 1024
   */
  local_resolutions: CommunicationActorLocalResolutionV1[];
  provider: OrganizationalProviderId;
  state: EffectiveFactStateV1;
}
export interface EffectiveIntervalV1 {
  end_exclusive: number;
  start: number;
}
export interface CommunicationActorLocalResolutionV1 {
  /**
   * @maxItems 1024
   */
  authorized_closure: OrganizationalNodeRefV1[];
  local_node: OrganizationalNodeRefV1;
}
export interface LogicalScopeRefV1 {
  namespace: WorkspaceNamespaceId;
  scope_id: LogicalScopeId;
}
export interface LogicalOrganizationRefV1 {
  namespace: WorkspaceNamespaceId;
  organization_id: LogicalOrganizationId;
}
export interface OrganizationalGroupRefV1 {
  group_id: OrganizationalGroupId;
  namespace: WorkspaceNamespaceId;
}
export interface DisciplineTreeRefV1 {
  namespace: WorkspaceNamespaceId;
  tree_id: DisciplineTreeId;
}
export interface OrganizationalRoleRefV1 {
  namespace: WorkspaceNamespaceId;
  role_id: OrganizationalRoleId;
}
export interface DisciplineTreeV1 {
  axes: OrganizationalAuthorityAxesV1;
  interval: EffectiveIntervalV1;
  organization: LogicalOrganizationRefV1;
  state: EffectiveFactStateV1;
  tree: DisciplineTreeRefV1;
}
export interface OrganizationalAuthorityAxesV1 {
  authority: WorkspaceAuthorityScopeId;
  controller: WorkspaceControllerScopeId;
  disclosure: DisclosureCeilingV1;
  owner: WorkspaceResourceId;
  /**
   * @maxItems 1024
   */
  resources: WorkspaceResourceId[];
}
export interface GroupLensAuthorityProfileV1 {
  alignment_relation: OrganizationalRelationId;
  /**
   * @maxItems 1024
   */
  block_conditions: OrganizationalConditionId[];
  escalation_resolver?: OrganizationalRoleRefV1 | null;
  holder: OrganizationalRoleRefV1;
  interval: EffectiveIntervalV1;
  issuer: WorkspaceResourceId;
  lens: OrganizationalLensId;
  profile_id: GroupLensProfileId;
  review_owner?: OrganizationalRoleRefV1 | null;
  /**
   * @maxItems 4
   */
  rights:
    | []
    | [GroupLensRightV1]
    | [GroupLensRightV1, GroupLensRightV1]
    | [GroupLensRightV1, GroupLensRightV1, GroupLensRightV1]
    | [GroupLensRightV1, GroupLensRightV1, GroupLensRightV1, GroupLensRightV1];
  /**
   * @maxItems 1024
   */
  standards: WorkspaceResourceId[];
  state: EffectiveFactStateV1;
}
export interface OrganizationalGroupV1 {
  axes: OrganizationalAuthorityAxesV1;
  group: OrganizationalGroupRefV1;
  interval: EffectiveIntervalV1;
  organization: LogicalOrganizationRefV1;
  state: EffectiveFactStateV1;
}
export interface OrganizationalInspectionV1 {
  /**
   * @minItems 1
   * @maxItems 9
   */
  coordinates:
    | [InspectionCoordinateClassV1]
    | [InspectionCoordinateClassV1, InspectionCoordinateClassV1]
    | [InspectionCoordinateClassV1, InspectionCoordinateClassV1, InspectionCoordinateClassV1]
    | [
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1
      ]
    | [
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1
      ]
    | [
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1
      ]
    | [
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1
      ]
    | [
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1
      ]
    | [
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1,
        InspectionCoordinateClassV1
      ];
  disclosure: DisclosureCeilingV1;
  inspection_id: OrganizationalInspectionId;
  principal: WorkspaceResourceId;
  /**
   * @minItems 1
   * @maxItems 7
   */
  states:
    | [EffectiveFactStateV1]
    | [EffectiveFactStateV1, EffectiveFactStateV1]
    | [EffectiveFactStateV1, EffectiveFactStateV1, EffectiveFactStateV1]
    | [EffectiveFactStateV1, EffectiveFactStateV1, EffectiveFactStateV1, EffectiveFactStateV1]
    | [EffectiveFactStateV1, EffectiveFactStateV1, EffectiveFactStateV1, EffectiveFactStateV1, EffectiveFactStateV1]
    | [
        EffectiveFactStateV1,
        EffectiveFactStateV1,
        EffectiveFactStateV1,
        EffectiveFactStateV1,
        EffectiveFactStateV1,
        EffectiveFactStateV1
      ]
    | [
        EffectiveFactStateV1,
        EffectiveFactStateV1,
        EffectiveFactStateV1,
        EffectiveFactStateV1,
        EffectiveFactStateV1,
        EffectiveFactStateV1,
        EffectiveFactStateV1
      ];
  view_provider?: WorkspaceResourceId | null;
}
export interface LeadershipParticipationV1 {
  /**
   * @minItems 1
   * @maxItems 1024
   */
  affected_scopes: [LogicalScopeRefV1, ...LogicalScopeRefV1[]];
  disclosure: DisclosureCeilingV1;
  interval: EffectiveIntervalV1;
  issuer: WorkspaceResourceId;
  /**
   * @minItems 1
   * @maxItems 1024
   */
  participants: [OrganizationalRoleRefV1, ...OrganizationalRoleRefV1[]];
  state: EffectiveFactStateV1;
  topic: LeadershipTopicId;
}
export interface OrganizationalNativeProjectionV1 {
  class: ResponsibilityProjectionClassV1;
  disclosure: DisclosureCeilingV1;
  interval: EffectiveIntervalV1;
  native_owner: WorkspaceResourceId;
  /**
   * @maxItems 1024
   */
  native_refs: WorkspaceResourceId[];
  projection_id: OrganizationalProjectionId;
  state: EffectiveFactStateV1;
  subject: OrganizationalNodeRefV1;
}
export interface OrganizationalObjectV1 {
  axes: OrganizationalAuthorityAxesV1;
  interval: EffectiveIntervalV1;
  namespace: WorkspaceNamespaceId;
  object_class: OrganizationalObjectClassId;
  object_id: OrganizationalObjectId;
  organization: LogicalOrganizationRefV1;
  scope: LogicalScopeRefV1;
  state: EffectiveFactStateV1;
}
export interface OrganizationalOccupantBindingV1 {
  interval: EffectiveIntervalV1;
  issuer: WorkspaceResourceId;
  occupant: OrganizationalOccupantId;
  role: OrganizationalRoleRefV1;
  state: EffectiveFactStateV1;
}
export interface LogicalOrganizationV1 {
  axes: OrganizationalAuthorityAxesV1;
  interval: EffectiveIntervalV1;
  organization: LogicalOrganizationRefV1;
  scope: LogicalScopeRefV1;
  state: EffectiveFactStateV1;
}
export interface ObjectRolePopulationRuleV1 {
  accountable_role_class: OrganizationalRoleClassId;
  interval: EffectiveIntervalV1;
  issuer: WorkspaceResourceId;
  object_class: OrganizationalObjectClassId;
  state: EffectiveFactStateV1;
}
export interface OrganizationalProviderBindingV1 {
  interval: EffectiveIntervalV1;
  issuer: WorkspaceResourceId;
  provider: OrganizationalProviderId;
  role: OrganizationalRoleRefV1;
  state: EffectiveFactStateV1;
}
export interface OrganizationalRelationV1 {
  class: OrganizationalRelationClassV1;
  from: OrganizationalNodeRefV1;
  interval: EffectiveIntervalV1;
  issuer: WorkspaceResourceId;
  relation_id: OrganizationalRelationId;
  state: EffectiveFactStateV1;
  to: OrganizationalNodeRefV1;
}
export interface OrganizationalRoleV1 {
  axes: OrganizationalAuthorityAxesV1;
  discipline_tree?: DisciplineTreeRefV1 | null;
  group?: OrganizationalGroupRefV1 | null;
  interval: EffectiveIntervalV1;
  organization: LogicalOrganizationRefV1;
  role: OrganizationalRoleRefV1;
  role_class: OrganizationalRoleClassId;
  state: EffectiveFactStateV1;
}
export interface LogicalScopeV1 {
  axes: OrganizationalAuthorityAxesV1;
  interval: EffectiveIntervalV1;
  scope: LogicalScopeRefV1;
  state: EffectiveFactStateV1;
}
export interface DirectionalServiceContractV1 {
  /**
   * @maxItems 1024
   */
  assignments: ServiceAssignmentV1[];
  ceiling: WorkspacePolicyCeilingV1;
  consumer_acceptance: ConsumerAcceptanceRequirementV1;
  consumer_scope: LogicalScopeRefV1;
  contract_id: DirectionalServiceContractId;
  contract_owner: WorkspaceResourceId;
  interval: EffectiveIntervalV1;
  namespace: WorkspaceNamespaceId;
  native_read_back: NativeReadBackRequirementV1;
  pale_settlement: PaleSettlementRequirementV1;
  provider_acceptance: ProviderAcceptanceRequirementV1;
  provider_completion: ProviderCompletionRequirementV1;
  provider_scope: LogicalScopeRefV1;
  state: EffectiveFactStateV1;
}
export interface ServiceAssignmentV1 {
  account: ServiceAccountId;
  interval: EffectiveIntervalV1;
  mandate: ServiceMandateId;
  /**
   * @maxItems 1024
   */
  pods: ServicePodId[];
  state: EffectiveFactStateV1;
  /**
   * @maxItems 1024
   */
  stewards: ServiceStewardId[];
}
export interface WorkspacePolicyCeilingV1 {
  authorities: WorkspaceAuthorityCeilingV1;
  controllers: WorkspaceControllerCeilingV1;
  disclosure: DisclosureCeilingV1;
  effect: WorkspaceEffectCeilingV1;
  model_use_permitted: boolean;
  retention_seconds: number;
  rights: WorkspaceRightsCeilingV1;
}
export interface WorkspaceAuthorityCeilingV1 {
  /**
   * @minItems 0
   * @maxItems 1024
   */
  permitted: WorkspaceAuthorityScopeId[];
}
export interface WorkspaceControllerCeilingV1 {
  /**
   * @minItems 0
   * @maxItems 1024
   */
  permitted: WorkspaceControllerScopeId[];
}
export interface WorkspaceRightsCeilingV1 {
  /**
   * @minItems 0
   * @maxItems 1024
   */
  permitted: WorkspaceRightId[];
}
export interface ConsumerAcceptanceRequirementV1 {
  requirement: ResponsibilityEvidenceRequirementV1;
}
export interface ResponsibilityEvidenceRequirementV1 {
  evidence_ref: WorkspaceResourceId;
  owner: WorkspaceResourceId;
}
export interface NativeReadBackRequirementV1 {
  requirement: ResponsibilityEvidenceRequirementV1;
}
export interface PaleSettlementRequirementV1 {
  requirement: ResponsibilityEvidenceRequirementV1;
}
export interface ProviderAcceptanceRequirementV1 {
  requirement: ResponsibilityEvidenceRequirementV1;
}
export interface ProviderCompletionRequirementV1 {
  requirement: ResponsibilityEvidenceRequirementV1;
}
