/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type Sha256DigestV1 = string;
export type EventCausationIdV1 = string;
export type DataClassificationV1 = string;
export type EventCorrelationIdV1 = string;
export type EventIdV1 = string;
export type EventTypeV1 = string;
export type UnixTimeV1 = number;
export type LifecycleNamespaceV1 = string;
export type EventOrderingKeyV1 = string;
export type ResidentNodeIdV1 = string;
export type ResidentPrincipalIdV1 = string;
export type ResidentRecordRefV1 = string;
export type ProducerEpochV1 = string;
export type OwnerEventSequenceV1 = number;
export type SourceOwnerV1 = string;
export type SchemaId = string;
export type SourceVersionV1 = string;
export type ResidentControllerIdV1 = string;
export type DeploymentEnvironmentV1 = string;
export type RelationshipScopeV1 = string;

export interface EventReferenceV1 {
  canonical_event_sha256: Sha256DigestV1;
  causation_id?: EventCausationIdV1;
  classification: DataClassificationV1;
  correlation_id: EventCorrelationIdV1;
  event_id: EventIdV1;
  event_type: EventTypeV1;
  event_version: number;
  expires_at?: UnixTimeV1;
  lifecycle_namespace: LifecycleNamespaceV1;
  occurred_at: UnixTimeV1;
  ordering_key?: EventOrderingKeyV1;
  origin_node_id: ResidentNodeIdV1;
  origin_principal_id: ResidentPrincipalIdV1;
  payload_ref?: ResidentRecordRefV1;
  payload_sha256?: Sha256DigestV1;
  producer_epoch: ProducerEpochV1;
  recorded_at: UnixTimeV1;
  schema_id: "pale/event-reference.v1";
  schema_sha256: Sha256DigestV1;
  sequence?: OwnerEventSequenceV1;
  source_owner: SourceOwnerV1;
  source_record_content_sha256: Sha256DigestV1;
  source_record_ref: ResidentRecordRefV1;
  source_record_schema_id: SchemaId;
  source_record_schema_sha256: Sha256DigestV1;
  source_version: SourceVersionV1;
  target_scope: EventScopeV1;
}
export interface EventScopeV1 {
  controller_scope: ResidentControllerIdV1;
  environment: DeploymentEnvironmentV1;
  principal_scope: ResidentPrincipalIdV1;
  relationship_scope?: RelationshipScopeV1 | null;
}
