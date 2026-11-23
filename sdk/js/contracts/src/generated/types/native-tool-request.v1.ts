/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type LifecycleEventV1 =
  | "session-start"
  | "user-prompt-submit"
  | "user-prompt-expansion"
  | "permission-request"
  | "pre-tool-use"
  | "post-tool-use"
  | "post-tool-use-failure"
  | "subagent-start"
  | "subagent-stop"
  | "pre-compact"
  | "post-compact"
  | "stop"
  | "session-end";
export type Sha256DigestV1 = string;
export type LogicalRepositoryIdV1 = string;
export type WorkspaceId = string;

export interface NativeToolRequestV1 {
  event: LifecycleEventV1;
  native_event: string;
  provider_binding: ProviderBindingRefV1;
  repository_id: LogicalRepositoryIdV1;
  workspace_id: WorkspaceId;
}
export interface ProviderBindingRefV1 {
  provider_binding_id: string;
  provider_binding_identity: ProviderBindingIdentityV1;
}
export interface ProviderBindingIdentityV1 {
  provider_binding_sha256: Sha256DigestV1;
}
