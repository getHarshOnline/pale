/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type PackageResourceExposureV1 = "internal" | "exported";
export type OfficialSourceUrl = string;
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

export interface PackagePageV1 {
  description: string;
  exposure?: "internal" | "exported";
  guide?: PackageGuideV1 | null;
  /**
   * @maxItems 1024
   */
  official_sources?: OfficialReferenceV1[];
  policy?: PackagePolicyV1 | null;
  role?: PackageRoleV1 | null;
  /**
   * @maxItems 1024
   */
  see_also?: PackageResourceLinkV1[];
  stack?: PackageStackV1 | null;
  task?: PackageTaskV1 | null;
  /**
   * @maxItems 1024
   */
  uses?: PackageResourceLinkV1[];
}
export interface PackageGuideV1 {
  exposure?: PackageResourceExposureV1 | null;
}
export interface OfficialReferenceV1 {
  checked?: string | null;
  jurisdiction?: string | null;
  label: string;
  url: OfficialSourceUrl;
}
export interface PackagePolicyV1 {
  exposure?: PackageResourceExposureV1 | null;
}
export interface PackageRoleV1 {
  /**
   * @maxItems 1024
   */
  applies_to?: PackageResourceLinkV1[];
  exposure?: PackageResourceExposureV1 | null;
  /**
   * @maxItems 1024
   */
  extends?: PackageResourceLinkV1[];
  /**
   * @maxItems 1024
   */
  uses?: PackageResourceLinkV1[];
}
export interface PackageResourceRefV1 {
  package_id: PackageIdV1;
  resource_id: PackageResourceIdV1;
}
export interface PackageStackV1 {
  exposure?: PackageResourceExposureV1 | null;
  /**
   * @maxItems 1024
   */
  extends?: PackageResourceLinkV1[];
  /**
   * @maxItems 1024
   */
  role_uses?: PackageResourceLinkV1[];
  /**
   * @maxItems 1024
   */
  uses?: PackageResourceLinkV1[];
}
export interface PackageTaskV1 {
  capability_policy?: PackageResourceLinkV1 | null;
  /**
   * @maxItems 1024
   */
  capability_requirements?: OperationCapabilityRequirement[];
  exposure?: PackageResourceExposureV1 | null;
  /**
   * @maxItems 1024
   */
  outputs?: string[];
  /**
   * @maxItems 1024
   */
  roles?: PackageResourceLinkV1[];
  /**
   * @maxItems 1024
   */
  uses?: PackageResourceLinkV1[];
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
