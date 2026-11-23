/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;

export interface SurfaceExecutableBindingV1 {
  digest: Sha256DigestV1;
  exec: string;
  protocol: "pale.agent-surface-interface.v1";
  version: string;
}
