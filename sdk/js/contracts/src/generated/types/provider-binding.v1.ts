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
