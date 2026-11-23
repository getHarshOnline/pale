//! Shared diagnostic admission preserves canonical identity and source semantics.

use super::*;

fn canonical(value: &serde_json::Value) -> Vec<u8> {
    pale_core::canon::to_canonical_string(value)
        .unwrap()
        .into_bytes()
}

fn value() -> serde_json::Value {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1").unwrap();
    let capabilities = pale_contracts::schema_descriptor("capabilities.v1").unwrap();
    serde_json::json!({
        "schema": identity.schema.as_str(),
        "schema_sha256": identity.schema_sha256.as_str(),
        "schema_version": 1,
        "code": "workspace.binding_missing",
        "stage": "semantic_admission",
        "severity": "error",
        "disposition": "invalid_input",
        "summary": "one selected binding is missing",
        "subject": {"kind": "operation", "id": "compile"},
        "source": {
            "label": "workspace/profile.toml", "line": 4, "column": 7,
            "byte_start": 18, "byte_end": 25,
            "value_path": "operation_bindings[0].provider"
        },
        "schema_coordinate": {
            "registry_id": "capabilities.v1",
            "schema_sha256": capabilities.schema_sha256,
            "schema_path": "/properties/operation_bindings"
        },
        "facts": [{"key": "binding", "value": "compile"}],
        "retry": "after_correction", "effect": "not_applicable",
        "truncation": {
            "shortened_fields": 0, "omitted_causes": 0, "omitted_notes": 0,
            "omitted_help": 0, "omitted_facts": 0
        }
    })
}

#[test]
fn admitted_bytes_and_domain_identity_match_the_existing_wire_owner() {
    let expected = value();
    let bytes = canonical(&expected);
    let admitted = AdmittedDiagnostic::from_bytes(&bytes).unwrap();
    assert_eq!(admitted.canonical_bytes(), bytes);
    assert_eq!(serde_json::to_value(admitted.wire()).unwrap(), expected);
    let mut identity_preimage = b"pale.diagnostic.identity.v1\0".to_vec();
    identity_preimage.extend_from_slice(&bytes);
    assert_eq!(
        admitted.identity(),
        pale_core::sha256_hex(&identity_preimage)
    );
    assert_ne!(admitted.identity(), pale_core::sha256_hex(&bytes));
    let readmitted = AdmittedDiagnostic::from_bytes(admitted.canonical_bytes()).unwrap();
    assert_eq!(readmitted.identity(), admitted.identity());
    assert_eq!(readmitted.canonical_bytes(), admitted.canonical_bytes());
}

#[test]
fn cold_admission_does_not_redact_rewrite_or_recount_recorded_content() {
    let mut expected = value();
    // This is input evidence, not a request to publish or disclose its text.
    // Reapplying projection would replace the credential-shaped value and
    // silently change the identity of the retained diagnostic.
    expected["summary"] = serde_json::json!("stored token=original-value");
    expected["truncation"]["shortened_fields"] = serde_json::json!(7);
    expected["truncation"]["omitted_causes"] = serde_json::json!(3);
    let bytes = canonical(&expected);
    let admitted = AdmittedDiagnostic::from_bytes(&bytes).unwrap();
    assert_eq!(admitted.canonical_bytes(), bytes);
    assert_eq!(admitted.wire().summary, "stored token=original-value");
    assert_eq!(admitted.wire().truncation.shortened_fields, 7);
    assert_eq!(admitted.wire().truncation.omitted_causes, 3);
    // The pre-existing byte ingress accepts JSON whitespace and emits JCS;
    // it preserves content rather than asserting that input was already JCS.
    let pretty = serde_json::to_vec_pretty(&expected).unwrap();
    let normalized = AdmittedDiagnostic::from_bytes(&pretty).unwrap();
    assert_eq!(normalized.canonical_bytes(), bytes);
    assert_eq!(normalized.identity(), admitted.identity());
}

#[test]
fn strict_schema_and_identity_failures_keep_their_typed_sources() {
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(br#"{"schema":1,"schema":1}"#),
        Err(DiagnosticAdmissionError::StrictJson(_))
    ));
    let mut unknown = value();
    unknown["unexpected"] = serde_json::json!(true);
    let error = AdmittedDiagnostic::from_bytes(&canonical(&unknown)).unwrap_err();
    let DiagnosticAdmissionError::Contract(pale_contracts::SchemaError::Invalid(violations)) =
        error
    else {
        panic!("unknown fields must retain structured schema violations");
    };
    assert!(!violations.violations().is_empty());
    assert!(violations
        .violations()
        .iter()
        .any(|violation| violation.keyword() == "additionalProperties"));
    let mut foreign = value();
    foreign["schema_sha256"] = serde_json::json!("0".repeat(64));
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(&canonical(&foreign)),
        Err(DiagnosticAdmissionError::Contract(
            pale_contracts::SchemaError::IdentityMismatch(_)
        ))
    ));
}

#[test]
fn source_coordinates_remain_logical_and_relationally_complete() {
    for label in [
        "/private/path",
        "\\host\\path",
        "~private",
        "C:\\private",
        "safe/../private",
        "safe\\..\\private",
        "bad\nlabel",
    ] {
        let mut unsafe_value = value();
        unsafe_value["source"]["label"] = serde_json::json!(label);
        assert!(
            matches!(
                AdmittedDiagnostic::from_bytes(&canonical(&unsafe_value)),
                Err(DiagnosticAdmissionError::UnsafeSourceLabel)
            ),
            "{label:?}"
        );
    }
    let mut missing_line = value();
    missing_line["source"]
        .as_object_mut()
        .unwrap()
        .remove("line");
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(&canonical(&missing_line)),
        Err(DiagnosticAdmissionError::ColumnWithoutLine)
    ));
    let mut zero = value();
    zero["source"]["line"] = serde_json::json!(0);
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(&canonical(&zero)),
        Err(DiagnosticAdmissionError::ZeroSourcePosition)
    ));
    let mut missing_end = value();
    missing_end["source"]
        .as_object_mut()
        .unwrap()
        .remove("byte_end");
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(&canonical(&missing_end)),
        Err(DiagnosticAdmissionError::IncompleteByteSpan)
    ));
    let mut reversed = value();
    reversed["source"]["byte_end"] = serde_json::json!(17);
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(&canonical(&reversed)),
        Err(DiagnosticAdmissionError::ReversedByteSpan)
    ));
}

#[test]
fn schema_coordinates_cannot_select_foreign_identity_or_invalid_pointer() {
    let mut changed = value();
    changed["schema_coordinate"]["schema_path"] = serde_json::json!("properties/x");
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(&canonical(&changed)),
        Err(DiagnosticAdmissionError::InvalidSchemaPath)
    ));
    changed = value();
    changed["schema_coordinate"]["registry_id"] = serde_json::json!("not-registered.v1");
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(&canonical(&changed)),
        Err(DiagnosticAdmissionError::UnknownSchemaCoordinate)
    ));
    changed = value();
    changed["schema_coordinate"]["schema_sha256"] = serde_json::json!("0".repeat(64));
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(&canonical(&changed)),
        Err(DiagnosticAdmissionError::SchemaCoordinateMismatch)
    ));
    changed = value();
    changed["schema_coordinate"]["wire_schema_id"] = serde_json::json!("pale/diagnostic.v1");
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(&canonical(&changed)),
        Err(DiagnosticAdmissionError::SchemaCoordinateMismatch)
    ));
}

#[test]
fn effect_uncertainty_and_fact_uniqueness_keep_existing_semantics() {
    let mut changed = value();
    changed["effect"] = serde_json::json!("outcome_unknown");
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(&canonical(&changed)),
        Err(DiagnosticAdmissionError::EffectSemanticsMismatch)
    ));
    changed["disposition"] = serde_json::json!("effect_outcome_unknown");
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(&canonical(&changed)),
        Err(DiagnosticAdmissionError::EffectSemanticsMismatch)
    ));
    changed["retry"] = serde_json::json!("reconcile_first");
    AdmittedDiagnostic::from_bytes(&canonical(&changed)).unwrap();
    changed["effect"] = serde_json::json!("not_applicable");
    changed["disposition"] = serde_json::json!("blocked");
    AdmittedDiagnostic::from_bytes(&canonical(&changed)).unwrap();
    changed = value();
    changed["facts"] =
        serde_json::json!([{"key":"same", "value":"one"}, {"key":"same", "value":"two"}]);
    assert!(matches!(
        AdmittedDiagnostic::from_bytes(&canonical(&changed)),
        Err(DiagnosticAdmissionError::DuplicateFactKey)
    ));
}
