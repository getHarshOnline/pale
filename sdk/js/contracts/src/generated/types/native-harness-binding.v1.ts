/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;

export interface NativeHarnessBindingV1 {
  digest: Sha256DigestV1;
  exec: string;
  provider_binding: ProviderBindingRefV1;
  version: string;
}
export interface ProviderBindingRefV1 {
  provider_binding_id: string;
  provider_binding_identity: ProviderBindingIdentityV1;
}
export interface ProviderBindingIdentityV1 {
  provider_binding_sha256: Sha256DigestV1;
}
