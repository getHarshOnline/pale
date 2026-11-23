/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type AttributableAnswerForResponsibilityAdopterModuleV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ResponsibilityAdopterModuleV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type WorkspaceResourceId = string;
export type WorkspaceDerivationSourceId = string;
export type WorkspaceExportId = string;
export type WorkspaceModuleId = string;
export type WorkspaceReleaseId = string;
export type WorkspaceNamespaceId = string;
export type ResponsibilityOmissionReasonId = string;
export type AttributableAnswerForResponsibilityCeilingsV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ResponsibilityCeilingsV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type WorkspaceAuthorityScopeId = string;
export type WorkspaceControllerScopeId = string;
export type DisclosureCeilingV1 = "private" | "protected" | "public";
export type WorkspaceEffectCeilingV1 = "none" | "read" | "mutate";
export type WorkspaceRightId = string;
export type AttributableAnswerForConsumerAcceptanceRequirementV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ConsumerAcceptanceRequirementV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type ResponsibilityDerivationId = string;
export type AttributableAnswerForResponsibilityDirectPathV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ResponsibilityDirectPathV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type ResponsibilityGenealogyLinkId = string;
export type ResponsibilityOccurrenceId = string;
export type ResponsibilityProjectionId = string;
export type AttributableAnswerForResponsibilityIncumbentV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ResponsibilityIncumbentV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type AttributableAnswerForResponsibilitySilenceBoundaryV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ResponsibilitySilenceBoundaryV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type ResponsibilitySilenceClassV1 = "communication" | "accountable_work" | "relationship_state";
export type ResponsibilityDerivationLifecycleV1 =
  | "verified_current"
  | "accepted_target"
  | "owner_desired_configuration"
  | "candidate"
  | "implementation_open"
  | "configuration_open"
  | "evidence_gated"
  | "historical"
  | "rejected";
export type AttributableAnswerForResponsibilityNativeOwnerV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ResponsibilityNativeOwnerV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type ResponsibilityOwnerClassV1 = "principal" | "relationship_contract" | "native_application" | "module";
export type ResponsibilityProjectionClassV1 =
  | "accountable_work"
  | "communication"
  | "relationship"
  | "native_application"
  | "derived_view"
  | "context"
  | "genealogy";
export type EffectiveFactStateV1 = "present" | "absent" | "denied" | "unknown" | "stale" | "superseded" | "revoked";
export type AttributableAnswerForNativeReadBackRequirementV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: NativeReadBackRequirementV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type AttributableAnswerForPaleSettlementRequirementV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: PaleSettlementRequirementV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type AttributableAnswerForResponsibilityPlacementV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ResponsibilityPlacementV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type ResponsibilityPlacementV1 =
  | {
      kind: "vertical_local";
      scope: WorkspaceResourceId;
    }
  | {
      kind: "directional_service";
      service_contract: DirectionalServiceContractRefV1;
    };
export type LogicalScopeId = string;
export type DirectionalServiceContractId = string;
export type AttributableAnswerForProviderAcceptanceRequirementV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ProviderAcceptanceRequirementV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type AttributableAnswerForProviderCompletionRequirementV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ProviderCompletionRequirementV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type AttributableAnswerForResponsibilityPublisherModuleV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ResponsibilityPublisherModuleV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type AttributableAnswerForResponsibilityReadinessV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ResponsibilityReadinessV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type AttributableAnswerForResponsibilityRequestFramerV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ResponsibilityRequestFramerV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type ResponsibilityResidenceClassV1 =
  "package" | "normative_pack" | "native_application" | "configuration" | "reference";
export type AttributableAnswerForBoundedResponsibilityV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: BoundedResponsibilityV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type ResponsibilityObjectId = string;
export type ResponsibilityOutcomeId = string;
export type ResponsibilityVerbId = string;
export type AttributableAnswerForResponsibilitySafeFailureV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ResponsibilitySafeFailureV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type ResponsibilitySafeFailureV1 =
  "deny" | "wait" | "degrade" | "remain_native" | "compensate" | "owner_escalation";
export type StructuralObjectKindV1 =
  | "logical_organization"
  | "stable_role"
  | "occupant_provider"
  | "directional_service_contract"
  | "accountable_work_projection"
  | "communication_projection"
  | "relationship_projection"
  | "native_application"
  | "package"
  | "normative_pack"
  | "reference_only";
export type AttributableAnswerForWorkspaceResourceId =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: WorkspaceResourceId;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type AttributableAnswerForResponsibilityTriggerV1 =
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "supplied";
      value: ResponsibilityTriggerV1;
    }
  | {
      attribution: ResponsibilityAttributionV1;
      disposition: "omitted";
      reason: ResponsibilityOmissionReasonId;
    };
export type ResponsibilityTriggerV1 =
  | {
      cadence: WorkspaceResourceId;
      kind: "recurring_cadence";
    }
  | {
      event: WorkspaceResourceId;
      kind: "owner_event";
    }
  | {
      event: WorkspaceResourceId;
      kind: "native_event";
    }
  | {
      kind: "bounded_request";
      request_contract: WorkspaceResourceId;
    };

export interface ResponsibilityDerivationV1 {
  adopter_module: AttributableAnswerForResponsibilityAdopterModuleV1;
  ceilings: AttributableAnswerForResponsibilityCeilingsV1;
  consumer_acceptance: AttributableAnswerForConsumerAcceptanceRequirementV1;
  derivation_id: ResponsibilityDerivationId;
  direct_path: AttributableAnswerForResponsibilityDirectPathV1;
  /**
   * @maxItems 4096
   */
  genealogy_links: ResponsibilityGenealogyLinkV1[];
  incumbent: AttributableAnswerForResponsibilityIncumbentV1;
  lawful_silence: AttributableAnswerForResponsibilitySilenceBoundaryV1;
  lifecycle: ResponsibilityDerivationLifecycleV1;
  native_owner: AttributableAnswerForResponsibilityNativeOwnerV1;
  /**
   * @maxItems 1024
   */
  native_projections: ResponsibilityNativeProjectionV1[];
  native_result: AttributableAnswerForNativeReadBackRequirementV1;
  pale_settlement: AttributableAnswerForPaleSettlementRequirementV1;
  placement: AttributableAnswerForResponsibilityPlacementV1;
  provenance: ResponsibilityAttributionV1;
  provider_acceptance: AttributableAnswerForProviderAcceptanceRequirementV1;
  provider_completion: AttributableAnswerForProviderCompletionRequirementV1;
  publisher_module: AttributableAnswerForResponsibilityPublisherModuleV1;
  readiness: AttributableAnswerForResponsibilityReadinessV1;
  request_framer: AttributableAnswerForResponsibilityRequestFramerV1;
  /**
   * @minItems 1
   * @maxItems 1024
   */
  residence: [ResponsibilityResidenceV1, ...ResponsibilityResidenceV1[]];
  responsibility: AttributableAnswerForBoundedResponsibilityV1;
  safe_failure: AttributableAnswerForResponsibilitySafeFailureV1;
  schema: "pale/responsibility-derivation.v1";
  schema_version: 1;
  /**
   * @minItems 11
   * @maxItems 11
   */
  structural_objects: [
    ResponsibilityStructuralSelectionV1,
    ResponsibilityStructuralSelectionV1,
    ResponsibilityStructuralSelectionV1,
    ResponsibilityStructuralSelectionV1,
    ResponsibilityStructuralSelectionV1,
    ResponsibilityStructuralSelectionV1,
    ResponsibilityStructuralSelectionV1,
    ResponsibilityStructuralSelectionV1,
    ResponsibilityStructuralSelectionV1,
    ResponsibilityStructuralSelectionV1,
    ResponsibilityStructuralSelectionV1
  ];
  trigger: AttributableAnswerForResponsibilityTriggerV1;
}
export interface ResponsibilityAttributionV1 {
  evidence_ref: WorkspaceResourceId;
  owner: WorkspaceResourceId;
  source_id: WorkspaceDerivationSourceId;
}
export interface ResponsibilityAdopterModuleV1 {
  adoption_owner: WorkspaceResourceId;
  imported_export_id: WorkspaceExportId;
  imported_module_id: WorkspaceModuleId;
  imported_release_id: WorkspaceReleaseId;
  local_namespace: WorkspaceNamespaceId;
  module_id: WorkspaceModuleId;
}
export interface ResponsibilityCeilingsV1 {
  policy: WorkspacePolicyCeilingV1;
  /**
   * @maxItems 1024
   */
  resources: WorkspaceResourceId[];
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
export interface ResponsibilityDirectPathV1 {
  availability_policy: WorkspaceResourceId;
  native_owner: WorkspaceResourceId;
  path: WorkspaceResourceId;
}
export interface ResponsibilityGenealogyLinkV1 {
  link_id: ResponsibilityGenealogyLinkId;
  /**
   * @minItems 1
   * @maxItems 1024
   */
  occurrences: [ResponsibilityOccurrenceId, ...ResponsibilityOccurrenceId[]];
  /**
   * @maxItems 1024
   */
  projections: ResponsibilityProjectionId[];
  /**
   * @minItems 1
   * @maxItems 1024
   */
  responsibilities: [ResponsibilityDerivationId, ...ResponsibilityDerivationId[]];
}
export interface ResponsibilityIncumbentV1 {
  mechanism: WorkspaceResourceId;
  retained_evidence: WorkspaceResourceId;
  writer: WorkspaceResourceId;
}
export interface ResponsibilitySilenceBoundaryV1 {
  /**
   * @maxItems 3
   */
  silent_surfaces:
    | []
    | [ResponsibilitySilenceClassV1]
    | [ResponsibilitySilenceClassV1, ResponsibilitySilenceClassV1]
    | [ResponsibilitySilenceClassV1, ResponsibilitySilenceClassV1, ResponsibilitySilenceClassV1];
}
export interface ResponsibilityNativeOwnerV1 {
  authority_resolver: WorkspaceResourceId;
  owner: WorkspaceResourceId;
  owner_class: ResponsibilityOwnerClassV1;
}
export interface ResponsibilityNativeProjectionV1 {
  class: ResponsibilityProjectionClassV1;
  native_owner: WorkspaceResourceId;
  /**
   * @maxItems 1024
   */
  native_refs: WorkspaceResourceId[];
  projection_id: ResponsibilityProjectionId;
  state: EffectiveFactStateV1;
}
export interface NativeReadBackRequirementV1 {
  requirement: ResponsibilityEvidenceRequirementV1;
}
export interface PaleSettlementRequirementV1 {
  requirement: ResponsibilityEvidenceRequirementV1;
}
export interface DirectionalServiceContractRefV1 {
  consumer_scope: LogicalScopeRefV1;
  contract_id: DirectionalServiceContractId;
  namespace: WorkspaceNamespaceId;
  provider_scope: LogicalScopeRefV1;
}
export interface LogicalScopeRefV1 {
  namespace: WorkspaceNamespaceId;
  scope_id: LogicalScopeId;
}
export interface ProviderAcceptanceRequirementV1 {
  requirement: ResponsibilityEvidenceRequirementV1;
}
export interface ProviderCompletionRequirementV1 {
  requirement: ResponsibilityEvidenceRequirementV1;
}
export interface ResponsibilityPublisherModuleV1 {
  export_id: WorkspaceExportId;
  module_id: WorkspaceModuleId;
  release_id: WorkspaceReleaseId;
  semantic_owner: WorkspaceResourceId;
}
export interface ResponsibilityReadinessV1 {
  failure_proof: WorkspaceResourceId;
  native_read_back_proof: WorkspaceResourceId;
  operation_contract: WorkspaceResourceId;
  recovery_proof: WorkspaceResourceId;
  replay_proof: WorkspaceResourceId;
  rollback_proof: WorkspaceResourceId;
}
export interface ResponsibilityRequestFramerV1 {
  actor: WorkspaceResourceId;
  authority_scope: WorkspaceAuthorityScopeId;
  owner: WorkspaceResourceId;
}
export interface ResponsibilityResidenceV1 {
  class: ResponsibilityResidenceClassV1;
  owner: WorkspaceResourceId;
  resource: WorkspaceResourceId;
}
export interface BoundedResponsibilityV1 {
  object: ResponsibilityObjectId;
  outcome: ResponsibilityOutcomeId;
  verb: ResponsibilityVerbId;
}
export interface ResponsibilityStructuralSelectionV1 {
  kind: StructuralObjectKindV1;
  selection: AttributableAnswerForWorkspaceResourceId;
}
