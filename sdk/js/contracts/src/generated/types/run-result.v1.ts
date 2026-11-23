/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type LogicalExecutionWorkspaceIdV1 = string;
export type Sha256DigestV1 = string;
export type PlanKind = "apply" | "remove";
export type LogicalProjectIdV1 = string;
export type RunTargetResult =
  | {
      outcome: "succeeded";
      receipt_digest: Sha256DigestV1;
      repository_id: LogicalRepositoryIdV1;
    }
  | {
      outcome: "failed_no_effect";
      repository_id: LogicalRepositoryIdV1;
    };
export type LogicalRepositoryIdV1 = string;

export interface RunResultV1 {
  execution_workspace_id: LogicalExecutionWorkspaceIdV1;
  instance_digest: Sha256DigestV1;
  one_shot_compilation_digest: Sha256DigestV1;
  plan_kind: PlanKind;
  profile_digest: Sha256DigestV1;
  project_id: LogicalProjectIdV1;
  run_digest: Sha256DigestV1;
  schema: "pale.agent-surface.run-result.v1";
  schema_manifest_sha256: Sha256DigestV1;
  schema_set_sha256: Sha256DigestV1;
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  targets: RunTargetResult[];
}
