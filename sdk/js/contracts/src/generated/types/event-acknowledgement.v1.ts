/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type UnixTimeV1 = number;
export type ResidentRecordRefV1 = string;
export type Sha256DigestV1 = string;
export type ResidentNodeIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type PrincipalKeyEpochV1 = number;
export type EventIdV1 = string;
export type ProducerEpochV1 = string;
export type ConsumerStateVersionV1 = string;
export type DeliveryAttemptIdV1 = string;
export type EventAcknowledgementDispositionV1 = "accepted" | "duplicate" | "rejected" | "quarantined";
export type ResidentReasonV1 = string;
export type CanonicalSignatureV1 = string;
export type EventSubscriptionIdV1 = string;

export interface EventAcknowledgementV1 {
  acknowledged_at: UnixTimeV1;
  acknowledging_node_delegation_ref: ResidentRecordRefV1;
  acknowledging_node_delegation_sha256: Sha256DigestV1;
  acknowledging_node_id: ResidentNodeIdV1;
  acknowledging_principal_id: ResidentPrincipalIdV1;
  acknowledging_principal_key_epoch: PrincipalKeyEpochV1;
  canonical_acknowledgement_sha256: Sha256DigestV1;
  canonical_event_identity: CanonicalEventIdentityV1;
  canonical_event_sha256: Sha256DigestV1;
  consumer_state_version?: ConsumerStateVersionV1;
  delivery_attempt_id: DeliveryAttemptIdV1;
  destination_node_id?: ResidentNodeIdV1;
  destination_principal_id: ResidentPrincipalIdV1;
  disposition: EventAcknowledgementDispositionV1;
  duplicate_of_ack_ref?: ResidentRecordRefV1;
  duplicate_of_ack_sha256?: Sha256DigestV1;
  quarantine_ref?: ResidentRecordRefV1;
  quarantine_sha256?: Sha256DigestV1;
  reason_code?: ResidentReasonV1;
  schema_id: "pale/event-acknowledgement.v1";
  schema_sha256: Sha256DigestV1;
  signature: CanonicalSignatureV1;
  subscription_id: EventSubscriptionIdV1;
}
export interface CanonicalEventIdentityV1 {
  event_id: EventIdV1;
  origin_principal_id: ResidentPrincipalIdV1;
  producer_epoch: ProducerEpochV1;
}
