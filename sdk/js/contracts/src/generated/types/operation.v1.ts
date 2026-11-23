/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type OwnedConfinedRelativePath = string;
export type PackageResourceLinkV1 =
  | {
      resource_id: PackageResourceIdV1;
      scope: "local";
    }
  | {
      resource: PackageResourceRefV1;
      scope: "external";
    };
export type PackageResourceIdV1 = string;
export type PackageIdV1 = string;
export type CapabilityIdV1 = string;
export type EffectClass = "compute" | "read" | "execute" | "write" | "network" | "publish";
export type FailurePosture = "fail_closed" | "fail_open" | "retry" | "steer" | "escalate";
export type CapabilityRequirementIdV1 = string;
export type CapabilityRequirementPolicy =
  | {
      kind: "required_block";
    }
  | {
      degraded_outcome: DegradedOutcomeIdV1;
      fallback: CapabilityFallback;
      kind: "required_degrade";
    }
  | {
      fallback: CapabilityFallback;
      kind: "optional";
    };
export type DegradedOutcomeIdV1 = string;

export interface OperationV1 {
  operations: OperationCatalog;
}
export interface OperationCatalog {
  archetype?: OperationArchetype[];
  operation: OnDemandOperation[];
  schema_version: 1;
}
export interface OperationArchetype {
  description: string;
  doctrine: OwnedConfinedRelativePath;
  doctrine_refs?: PackageResourceLinkV1[];
  extends?: PackageResourceLinkV1[];
  id: PackageResourceIdV1;
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface OnDemandOperation {
  capability_policy?: PackageResourceLinkV1 | null;
  capability_requirements?: OperationCapabilityRequirement[];
  description: string;
  doctrine: OwnedConfinedRelativePath;
  doctrine_refs?: PackageResourceLinkV1[];
  extends?: PackageResourceLinkV1[];
  id: PackageResourceIdV1;
  outputs?: string[];
  roles?: PackageResourceLinkV1[];
}
export interface OperationCapabilityRequirement {
  capability: CapabilityIdV1;
  description: string;
  effect_class: EffectClass;
  failure_posture: FailurePosture;
  id: CapabilityRequirementIdV1;
  policy: CapabilityRequirementPolicy;
  recipe?: PackageResourceLinkV1 | null;
}
export interface CapabilityFallback {
  guidance: string;
  recipe?: PackageResourceLinkV1 | null;
}
