/* Generated from the canonical Pale initial-v1 schema bundle. Do not edit. */

export type DiagnosticDispositionV1 =
  "operational_failure" | "invalid_input" | "blocked" | "internal_failure" | "effect_outcome_unknown";
export type DiagnosticEffectCertaintyV1 = "not_applicable" | "not_started" | "confirmed" | "outcome_unknown";
export type DiagnosticRetryV1 = "not_indicated" | "after_correction" | "after_state_change" | "reconcile_first";
export type Sha256DigestV1 = string;
export type SchemaId = string;
export type DiagnosticSeverityV1 = "error";
export type DiagnosticStageV1 =
  | "input_admission"
  | "contract_validation"
  | "semantic_admission"
  | "package_loading"
  | "workspace_loading"
  | "compilation"
  | "rendering"
  | "operation_admission"
  | "resident_admission"
  | "resident_recovery"
  | "planning"
  | "materialization"
  | "native_action"
  | "host_observation"
  | "internal";

export interface DiagnosticV1 {
  /**
   * @maxItems 16
   */
  causes?:
    | []
    | [string]
    | [string, string]
    | [string, string, string]
    | [string, string, string, string]
    | [string, string, string, string, string]
    | [string, string, string, string, string, string]
    | [string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string, string, string, string, string]
    | [
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string
      ]
    | [
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string
      ];
  code: string;
  disposition: DiagnosticDispositionV1;
  effect: DiagnosticEffectCertaintyV1;
  /**
   * @maxItems 16
   */
  facts?:
    | []
    | [DiagnosticFactV1Wire]
    | [DiagnosticFactV1Wire, DiagnosticFactV1Wire]
    | [DiagnosticFactV1Wire, DiagnosticFactV1Wire, DiagnosticFactV1Wire]
    | [DiagnosticFactV1Wire, DiagnosticFactV1Wire, DiagnosticFactV1Wire, DiagnosticFactV1Wire]
    | [DiagnosticFactV1Wire, DiagnosticFactV1Wire, DiagnosticFactV1Wire, DiagnosticFactV1Wire, DiagnosticFactV1Wire]
    | [
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire
      ]
    | [
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire
      ]
    | [
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire
      ]
    | [
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire
      ]
    | [
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire
      ]
    | [
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire
      ]
    | [
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire
      ]
    | [
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire
      ]
    | [
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire
      ]
    | [
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire
      ]
    | [
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire,
        DiagnosticFactV1Wire
      ];
  generation?: DiagnosticGenerationCorrelationV1Wire | null;
  /**
   * @maxItems 16
   */
  help?:
    | []
    | [string]
    | [string, string]
    | [string, string, string]
    | [string, string, string, string]
    | [string, string, string, string, string]
    | [string, string, string, string, string, string]
    | [string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string, string, string, string, string]
    | [
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string
      ]
    | [
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string
      ];
  /**
   * @maxItems 16
   */
  notes?:
    | []
    | [string]
    | [string, string]
    | [string, string, string]
    | [string, string, string, string]
    | [string, string, string, string, string]
    | [string, string, string, string, string, string]
    | [string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string, string, string, string]
    | [string, string, string, string, string, string, string, string, string, string, string, string, string, string]
    | [
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string
      ]
    | [
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string,
        string
      ];
  receipt?: DiagnosticReceiptCorrelationV1Wire | null;
  request?: DiagnosticRequestCorrelationV1Wire | null;
  retry: DiagnosticRetryV1;
  schema: "pale/diagnostic.v1";
  schema_coordinate?: DiagnosticSchemaCoordinateV1Wire | null;
  schema_sha256: Sha256DigestV1;
  schema_version: 1;
  severity: DiagnosticSeverityV1;
  source?: DiagnosticSourceV1Wire | null;
  stage: DiagnosticStageV1;
  subject?: DiagnosticSubjectV1Wire | null;
  summary: string;
  truncation: DiagnosticTruncationV1Wire;
}
export interface DiagnosticFactV1Wire {
  key: string;
  value: string;
}
export interface DiagnosticGenerationCorrelationV1Wire {
  id: string;
  kind: string;
}
export interface DiagnosticReceiptCorrelationV1Wire {
  id: string;
  kind: string;
}
export interface DiagnosticRequestCorrelationV1Wire {
  id: string;
  kind: string;
}
export interface DiagnosticSchemaCoordinateV1Wire {
  registry_id: string;
  schema_path?: string | null;
  schema_sha256: Sha256DigestV1;
  wire_schema_id?: SchemaId | null;
}
export interface DiagnosticSourceV1Wire {
  byte_end?: number | null;
  byte_start?: number | null;
  column?: number | null;
  label: string;
  line?: number | null;
  value_path?: string | null;
}
export interface DiagnosticSubjectV1Wire {
  id: string;
  kind: string;
}
export interface DiagnosticTruncationV1Wire {
  omitted_causes: number;
  omitted_facts: number;
  omitted_help: number;
  omitted_notes: number;
  shortened_fields: number;
}
