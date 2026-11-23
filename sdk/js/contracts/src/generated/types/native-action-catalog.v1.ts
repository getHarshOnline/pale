/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type ActionBoundary = "pure";
export type Sha256DigestV1 = string;
export type ActionEffect = "compute";
export type ActionInputKind =
  | "compliance_trace_grade"
  | "config_invariant_scan"
  | "cost_aggregate"
  | "loop_status_reduce"
  | "review_convergence_reduce"
  | "rubric_evaluation"
  | "service_definition_normalize"
  | "skill_health_aggregate"
  | "weighted_ranking"
  | "path_inventory"
  | "evidence_selection";
export type ActionOutputKind =
  | "compliance_trace_grade"
  | "config_invariant_scan"
  | "cost_aggregate"
  | "loop_status_reduce"
  | "review_convergence_reduce"
  | "rubric_evaluation"
  | "service_definition_normalize"
  | "skill_health_aggregate"
  | "weighted_ranking"
  | "path_inventory"
  | "evidence_selection";

export interface NativeActionCatalogV1 {
  contracts: ActionContractV1[];
  schema: "pale/native-action-catalog.v1";
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
}
export interface ActionContractV1 {
  boundary: ActionBoundary;
  contract_digest: Sha256DigestV1;
  description: string;
  effect: ActionEffect;
  id: string;
  input_kind: ActionInputKind;
  output_kind: ActionOutputKind;
  schema: "pale/action-contract.v1";
  schema_version: 1;
  version: 1;
}
