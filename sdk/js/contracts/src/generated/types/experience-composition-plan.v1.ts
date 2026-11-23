/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type ParticipantRefV1 =
  | {
      interface_id: string;
      kind: "interface";
    }
  | {
      application_id: string;
      kind: "application";
    }
  | {
      engine_id: string;
      kind: "engine";
    }
  | {
      capability_id: string;
      kind: "capability";
    }
  | {
      kind: "terminal_leaf";
      terminal_leaf_id: string;
    }
  | {
      kind: "plugin";
      plugin_id: string;
    }
  | {
      kind: "skill";
      skill_id: string;
    };
export type CompositionEdgeV1 =
  | {
      /**
       * @maxItems 6
       */
      authority_ceiling:
        | []
        | [EffectClass]
        | [EffectClass, EffectClass]
        | [EffectClass, EffectClass, EffectClass]
        | [EffectClass, EffectClass, EffectClass, EffectClass]
        | [EffectClass, EffectClass, EffectClass, EffectClass, EffectClass]
        | [EffectClass, EffectClass, EffectClass, EffectClass, EffectClass, EffectClass];
      budget: PathBudgetV1;
      from: ParticipantRefV1;
      id: string;
      kind: "work_delegation";
      request_translation_id: string;
      responsibility_contract_id: string;
      result_translation_id: string;
      to: ParticipantRefV1;
    }
  | {
      embedded: ParticipantRefV1;
      host: ParticipantRefV1;
      id: string;
      kind: "ui_embedding";
    }
  | {
      dependency: ParticipantRefV1;
      id: string;
      kind: "plugin_dependency";
      plugin: ParticipantRefV1;
    }
  | {
      application: ParticipantRefV1;
      caller: ParticipantRefV1;
      id: string;
      kind: "native_application_call";
      provider_binding_id: string;
      request_translation_id: string;
      result_translation_id: string;
    }
  | {
      id: string;
      kind: "terminal_leaf";
      owner: ParticipantRefV1;
      terminal_leaf: ParticipantRefV1;
      termination_contract_id: string;
    }
  | {
      id: string;
      kind: "observation_settlement";
      observer: ParticipantRefV1;
      settlement_contract_id: string;
      subject: ParticipantRefV1;
    };
export type EffectClass = "compute" | "read" | "execute" | "write" | "network" | "publish";
export type EntryOriginRefV1 =
  | {
      kind: "user";
      user_entry_id: string;
    }
  | {
      interface_entry_id: string;
      kind: "interface";
    }
  | {
      kind: "participant";
      participant_entry_id: string;
    };
export type ResponsibilityV1 =
  | {
      kind: "communication";
    }
  | {
      kind: "organizational_work";
    }
  | {
      kind: "crm_business_state";
    }
  | {
      kind: "engineering_execution";
    }
  | {
      kind: "context_memory";
    }
  | {
      kind: "terminal_harness_execution";
    }
  | {
      kind: "native_event_publication";
    }
  | {
      kind: "native_effect_read_back";
    }
  | {
      contract_id: string;
      contract_schema_identity: ContractSchemaIdentityV1;
      kind: "extension";
      namespace_id: string;
    };
export type SchemaId = string;
export type NativeOutcomeV1 =
  | {
      edge_id: string;
      invocation_identity: InvocationIdentityV1;
      kind: "read_only_result";
      outcome_contract_id: string;
    }
  | {
      edge_id: string;
      invocation_identity: InvocationIdentityV1;
      kind: "effectful_outcome";
      native_effect_identity: NativeEffectIdentityV1;
      outcome_contract_id: string;
    };
export type SettlementDispositionV1 =
  "native_receipt_required" | "read_back_required" | "owner_acceptance_required" | "unsettled";
export type TerminationKindV1 = "completed" | "cancelled" | "failed" | "expired" | "effect_unknown";
export type CompositionGraphKindV1 =
  | "provider_compatibility"
  | "work_delegation"
  | "ui_embedding"
  | "plugin_dependency"
  | "native_application_call"
  | "terminal_leaf"
  | "observation_settlement";
export type CompositionReasonStepV1 =
  | {
      contract_id: string;
      reason: "origin_receipt";
    }
  | {
      contract_id: string;
      reason: "entry_contract";
    }
  | {
      contract_id: string;
      reason: "responsibility_contract";
    }
  | {
      binding_id: string;
      reason: "provider_binding";
    }
  | {
      edge_id: string;
      reason: "delegation_edge";
    }
  | {
      contract_id: string;
      reason: "termination_contract";
    }
  | {
      contract_id: string;
      reason: "settlement_contract";
    }
  | {
      contract_id: string;
      reason: "continuity_contract";
    };

export interface ExperienceCompositionPlanV1 {
  declarations: ExperienceCompositionDeclarationsV1;
  delegation_graph: {
    /**
     * @maxItems 4096
     */
    edges: string[];
    kind: CompositionGraphKindV1;
    /**
     * @maxItems 1024
     */
    nodes: ParticipantRefV1[];
  } & {
    kind?: "work_delegation";
    [k: string]: any;
  };
  evidence_settlement_graph: {
    /**
     * @maxItems 4096
     */
    edges: string[];
    kind: CompositionGraphKindV1;
    /**
     * @maxItems 1024
     */
    nodes: ParticipantRefV1[];
  } & {
    kind?: "observation_settlement";
    [k: string]: any;
  };
  native_application_graph: {
    /**
     * @maxItems 4096
     */
    edges: string[];
    kind: CompositionGraphKindV1;
    /**
     * @maxItems 1024
     */
    nodes: ParticipantRefV1[];
  } & {
    kind?: "native_application_call";
    [k: string]: any;
  };
  plan_digest: Sha256DigestV1;
  plugin_dependency_graph: {
    /**
     * @maxItems 4096
     */
    edges: string[];
    kind: CompositionGraphKindV1;
    /**
     * @maxItems 1024
     */
    nodes: ParticipantRefV1[];
  } & {
    kind?: "plugin_dependency";
    [k: string]: any;
  };
  provider_compatibility_graph: {
    /**
     * @maxItems 4096
     */
    edges: string[];
    kind: CompositionGraphKindV1;
    /**
     * @maxItems 1024
     */
    nodes: ParticipantRefV1[];
  } & {
    kind?: "provider_compatibility";
    [k: string]: any;
  };
  /**
   * @maxItems 256
   */
  reason_paths: CompositionReasonPathV1[];
  schema: "pale/experience-composition-plan.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  terminal_leaf_graph: {
    /**
     * @maxItems 4096
     */
    edges: string[];
    kind: CompositionGraphKindV1;
    /**
     * @maxItems 1024
     */
    nodes: ParticipantRefV1[];
  } & {
    kind?: "terminal_leaf";
    [k: string]: any;
  };
  ui_embedding_graph: {
    /**
     * @maxItems 4096
     */
    edges: string[];
    kind: CompositionGraphKindV1;
    /**
     * @maxItems 1024
     */
    nodes: ParticipantRefV1[];
  } & {
    kind?: "ui_embedding";
    [k: string]: any;
  };
}
export interface ExperienceCompositionDeclarationsV1 {
  /**
   * @maxItems 1024
   */
  continuity_contracts: ContinuityContractV1[];
  /**
   * @maxItems 4096
   */
  edge_contracts: CompositionEdgeV1[];
  /**
   * @maxItems 1024
   */
  entry_contracts: EntryContractV1[];
  /**
   * @maxItems 1024
   */
  participant_provider_manifests: ParticipantProviderManifestV1[];
  /**
   * @maxItems 1024
   */
  provider_bindings: ProviderBindingV1[];
  /**
   * @maxItems 1024
   */
  request_translations: RequestTranslationV1[];
  /**
   * @maxItems 1024
   */
  responsibility_contracts: ResponsibilityContractV1[];
  /**
   * @maxItems 1024
   */
  result_translations: ResultTranslationV1[];
  /**
   * @maxItems 256
   */
  selected_experience_paths: ExperiencePathV1[];
  /**
   * @maxItems 1024
   */
  settlement_contracts: SettlementContractV1[];
  /**
   * @maxItems 1024
   */
  termination_contracts: TerminationContractV1[];
}
export interface ContinuityContractV1 {
  id: string;
  identity: ContinuityIdentityV1;
  /**
   * @maxItems 256
   */
  obligations: string[];
  owner: ParticipantRefV1;
}
export interface ContinuityIdentityV1 {
  continuity_sha256: Sha256DigestV1;
}
export interface PathBudgetV1 {
  max_cost_units: number;
  max_depth: number;
  max_effect_count: number;
  max_fan_out: number;
  max_recursion_iterations: number;
  max_resource_units: number;
  max_time_ms: number;
}
export interface EntryContractV1 {
  entry: ParticipantRefV1;
  id: string;
  origin: EntryOriginRefV1;
  responsibility_contract_id: string;
}
export interface ParticipantProviderManifestV1 {
  id: string;
  /**
   * @maxItems 1024
   */
  participants: ParticipantRefV1[];
  /**
   * @maxItems 1024
   */
  providers: ProviderCompatibilityDeclarationV1[];
}
export interface ProviderCompatibilityDeclarationV1 {
  /**
   * @maxItems 256
   */
  capability_ids: string[];
  /**
   * @maxItems 256
   */
  compatible_provider_ids: string[];
  id: string;
  participant: ParticipantRefV1;
}
export interface ProviderBindingV1 {
  adapter_plugin: AdapterPluginRefV1;
  artifact_coordinates: ProviderArtifactCoordinatesV1;
  authority_coordinates: ProviderAuthorityCoordinatesV1;
  behavior_coordinates: ProviderBehaviorCoordinatesV1;
  binding_identity: ProviderBindingIdentityV1;
  communication_actor_handle: CommunicationActorHandleV1 | null;
  conformance_coordinates: ProviderConformanceCoordinatesV1;
  exchange_coordinates: ProviderExchangeCoordinatesV1;
  generation_coordinates: ProviderGenerationCoordinatesV1;
  id: string;
  lifecycle_coordinates: ProviderLifecycleCoordinatesV1;
  logical_provider_instance: LogicalProviderInstanceRefV1;
  mapping_coordinates: ProviderMappingCoordinatesV1;
  native_authority_origin: NativeAuthorityOriginRefV1;
  participant: ParticipantRefV1;
  physical_deployment: PhysicalDeploymentRefV1;
  protocol_coordinates: ProviderProtocolCoordinatesV1;
  provider_family: ProviderFamilyRefV1;
  responsibility: ResponsibilityV1;
  /**
   * @minItems 1
   * @maxItems 256
   */
  responsibility_contract_ids: [string, ...string[]];
  /**
   * @minItems 1
   * @maxItems 256
   */
  responsibility_schema_identities: [ContractSchemaIdentityV1, ...ContractSchemaIdentityV1[]];
}
export interface AdapterPluginRefV1 {
  adapter_plugin_id: string;
}
export interface ProviderArtifactCoordinatesV1 {
  build: string;
  executable: string;
  image: string;
  package: string;
  source: string;
}
export interface ProviderAuthorityCoordinatesV1 {
  field: string;
  native: string;
}
export interface ProviderBehaviorCoordinatesV1 {
  degraded: string;
  removal: string;
}
export interface ProviderBindingIdentityV1 {
  provider_binding_sha256: Sha256DigestV1;
}
export interface CommunicationActorHandleV1 {
  canonical_opaque_handle: string;
  display_form: string | null;
  equality_scope: string;
  lifecycle_evidence_contract_id: string;
  namespace_id: string;
  native_authorship_verifier_id: string;
  reference_mapping_id: string;
  succession_evidence_contract_id: string;
}
export interface ProviderConformanceCoordinatesV1 {
  conformance: string;
  evidence: string;
  /**
   * @maxItems 256
   */
  per_capability_evidence: CapabilityEvidenceCoordinateV1[];
}
export interface CapabilityEvidenceCoordinateV1 {
  capability_id: string;
  evidence_contract_id: string;
}
export interface ProviderExchangeCoordinatesV1 {
  cancel: string;
  cursor: string;
  drain: string;
  outbox: string;
  request: string;
  result: string;
}
export interface ProviderGenerationCoordinatesV1 {
  deployment_activation_identity: DeploymentActivationIdentityV1;
  native_data_generation_identity: NativeDataGenerationIdentityV1;
  pale_admission_generation_identity: PaleAdmissionGenerationIdentityV1;
}
export interface DeploymentActivationIdentityV1 {
  deployment_activation_sha256: Sha256DigestV1;
}
export interface NativeDataGenerationIdentityV1 {
  native_data_generation_sha256: Sha256DigestV1;
}
export interface PaleAdmissionGenerationIdentityV1 {
  pale_admission_generation_sha256: Sha256DigestV1;
}
export interface ProviderLifecycleCoordinatesV1 {
  failure: string;
  lifecycle: string;
}
export interface LogicalProviderInstanceRefV1 {
  logical_provider_instance_id: string;
}
export interface ProviderMappingCoordinatesV1 {
  entity: string;
  event: string;
  field: string;
  terminology: string;
}
export interface NativeAuthorityOriginRefV1 {
  native_authority_origin_id: string;
}
export interface PhysicalDeploymentRefV1 {
  physical_deployment_id: string;
}
export interface ProviderProtocolCoordinatesV1 {
  configuration: string;
  /**
   * @maxItems 256
   */
  extensions: string[];
  protocol: string;
}
export interface ProviderFamilyRefV1 {
  provider_family_id: string;
}
export interface ContractSchemaIdentityV1 {
  schema_id: SchemaId;
  schema_sha256: Sha256DigestV1;
}
export interface RequestTranslationV1 {
  from: ParticipantRefV1;
  id: string;
  input_contract: string;
  output_contract: string;
  preserves_authority: boolean;
  to: ParticipantRefV1;
}
export interface ResponsibilityContractV1 {
  /**
   * @maxItems 6
   */
  authority_ceiling:
    | []
    | [EffectClass]
    | [EffectClass, EffectClass]
    | [EffectClass, EffectClass, EffectClass]
    | [EffectClass, EffectClass, EffectClass, EffectClass]
    | [EffectClass, EffectClass, EffectClass, EffectClass, EffectClass]
    | [EffectClass, EffectClass, EffectClass, EffectClass, EffectClass, EffectClass];
  id: string;
  participant: ParticipantRefV1;
  /**
   * @minItems 1
   * @maxItems 256
   */
  responsibilities: [ResponsibilityV1, ...ResponsibilityV1[]];
}
export interface ResultTranslationV1 {
  from: ParticipantRefV1;
  id: string;
  input_contract: string;
  output_contract: string;
  preserves_evidence: boolean;
  to: ParticipantRefV1;
}
export interface ExperiencePathV1 {
  assurance_subject_id: string;
  budget: PathBudgetV1;
  /**
   * @maxItems 256
   */
  capability_ceiling: string[];
  compiled_content: CompiledContentCoordinateV1;
  continuity_contract_id: string;
  /**
   * @maxItems 256
   */
  destination_consequences: DestinationConsequenceV1[];
  /**
   * @maxItems 6
   */
  effect_ceiling:
    | []
    | [EffectClass]
    | [EffectClass, EffectClass]
    | [EffectClass, EffectClass, EffectClass]
    | [EffectClass, EffectClass, EffectClass, EffectClass]
    | [EffectClass, EffectClass, EffectClass, EffectClass, EffectClass]
    | [EffectClass, EffectClass, EffectClass, EffectClass, EffectClass, EffectClass];
  entry_contract_id: string;
  evidence_carrier_id: string;
  id: string;
  identity: ResponsibilityPathIdentityV1;
  /**
   * @maxItems 256
   */
  invocations: PathInvocationV1[];
  /**
   * @maxItems 256
   */
  native_outcomes: NativeOutcomeV1[];
  /**
   * @maxItems 256
   */
  native_owner_transitions: NativeOwnerTransitionV1[];
  origin_receipt_contract_id: string;
  privacy_policy_id: string;
  /**
   * @maxItems 256
   */
  provider_binding_ids: string[];
  /**
   * @maxItems 256
   */
  reconciliation: ReconciliationRequirementV1[];
  /**
   * @maxItems 256
   */
  responsibility_contract_ids: string[];
  retention_policy_id: string;
  settlement_contract_id: string;
  termination_contract_id: string;
  /**
   * @maxItems 256
   */
  work_delegation_edge_ids: string[];
}
export interface CompiledContentCoordinateV1 {
  compiled_content_identity: CompiledContentIdentityV1;
  source_content_identity: SourceContentIdentityV1;
}
export interface CompiledContentIdentityV1 {
  compiled_content_sha256: Sha256DigestV1;
}
export interface SourceContentIdentityV1 {
  source_content_sha256: Sha256DigestV1;
}
export interface DestinationConsequenceV1 {
  consequence_contract_id: string;
  destination: ParticipantRefV1;
}
export interface ResponsibilityPathIdentityV1 {
  responsibility_path_sha256: Sha256DigestV1;
}
export interface PathInvocationV1 {
  edge_id: string;
  identity: InvocationIdentityV1;
}
export interface InvocationIdentityV1 {
  invocation_sha256: Sha256DigestV1;
}
export interface NativeEffectIdentityV1 {
  native_effect_sha256: Sha256DigestV1;
}
export interface NativeOwnerTransitionV1 {
  edge_id: string;
  native_owner_id: string;
  receipt_contract_id: string;
  transition_contract_id: string;
}
export interface ReconciliationRequirementV1 {
  frontier_id: string;
  obligation_contract_id: string;
}
export interface SettlementContractV1 {
  disposition: SettlementDispositionV1;
  id: string;
  identity: EvidenceSettlementIdentityV1;
  native_receipt_contract: string | null;
  observer: ParticipantRefV1;
}
export interface EvidenceSettlementIdentityV1 {
  evidence_settlement_sha256: Sha256DigestV1;
}
export interface TerminationContractV1 {
  id: string;
  /**
   * @minItems 1
   * @maxItems 5
   */
  kinds:
    | [TerminationKindV1]
    | [TerminationKindV1, TerminationKindV1]
    | [TerminationKindV1, TerminationKindV1, TerminationKindV1]
    | [TerminationKindV1, TerminationKindV1, TerminationKindV1, TerminationKindV1]
    | [TerminationKindV1, TerminationKindV1, TerminationKindV1, TerminationKindV1, TerminationKindV1];
  owner: ParticipantRefV1;
  terminal_leaf: ParticipantRefV1 | null;
}
export interface CompositionReasonPathV1 {
  path_id: string;
  /**
   * @minItems 3
   * @maxItems 1024
   */
  reasons: [CompositionReasonStepV1, CompositionReasonStepV1, CompositionReasonStepV1, ...CompositionReasonStepV1[]];
}
