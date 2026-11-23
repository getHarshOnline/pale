//! New diagnostic projections preserve the established wire and text rules.

use super::*;
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1,
    DiagnosticSourceV1Wire, DiagnosticStageV1,
};

fn projection() -> DiagnosticProjection {
    DiagnosticProjection {
        code: "workspace.binding_missing".to_string(),
        stage: DiagnosticStageV1::SemanticAdmission,
        disposition: DiagnosticDispositionV1::InvalidInput,
        summary: "binding token=private is missing".to_string(),
        subject: Some(("operation".to_string(), "compile".to_string())),
        source: Some(DiagnosticSourceV1Wire {
            label: "workspace/profile.toml".to_string(),
            line: Some(4),
            column: Some(7),
            byte_start: Some(18),
            byte_end: Some(25),
            value_path: Some("operation_bindings[0].provider".to_string()),
        }),
        schema_reference: Some((
            "capabilities.v1".to_string(),
            Some("/properties/operation_bindings".to_string()),
        )),
        causes: vec!["provider token=private is absent".to_string()],
        notes: vec!["selected binding".to_string()],
        help: vec!["correct the source".to_string()],
        facts: vec![("binding".to_string(), "compile".to_string())],
        retry: DiagnosticRetryV1::AfterCorrection,
        effect: DiagnosticEffectCertaintyV1::NotApplicable,
        request: Some(("operation".to_string(), "request-1".to_string())),
        generation: Some(("compiled".to_string(), "generation-1".to_string())),
        receipt: Some(("preparation".to_string(), "receipt-1".to_string())),
        omitted_causes: 2,
    }
}

#[test]
fn projected_record_matches_the_complete_existing_wire_shape() {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1").unwrap();
    let descriptor = pale_contracts::schema_descriptor("capabilities.v1").unwrap();
    let admitted = projection().admit(&identity).unwrap();
    let expected = serde_json::json!({
        "schema": identity.schema.as_str(), "schema_sha256": identity.schema_sha256.as_str(),
        "schema_version": 1, "code": "workspace.binding_missing",
        "stage": "semantic_admission", "severity": "error", "disposition": "invalid_input",
        "summary": "binding token=[REDACTED] is missing",
        "subject": {"kind": "operation", "id": "compile"},
        "source": {"label": "workspace/profile.toml", "line": 4, "column": 7,
            "byte_start": 18, "byte_end": 25, "value_path": "operation_bindings[0].provider"},
        "schema_coordinate": {"registry_id": "capabilities.v1", "schema_sha256": descriptor.schema_sha256,
            "schema_path": "/properties/operation_bindings"},
        "causes": ["provider token=[REDACTED] is absent"], "notes": ["selected binding"],
        "help": ["correct the source"], "facts": [{"key": "binding", "value": "compile"}],
        "retry": "after_correction", "effect": "not_applicable",
        "request": {"kind": "operation", "id": "request-1"},
        "generation": {"kind": "compiled", "id": "generation-1"},
        "receipt": {"kind": "preparation", "id": "receipt-1"},
        "truncation": {"shortened_fields": 0, "omitted_causes": 2, "omitted_notes": 0,
            "omitted_help": 0, "omitted_facts": 0}
    });
    let canonical = pale_core::canon::to_canonical_string(&expected).unwrap();
    assert_eq!(admitted.canonical_bytes(), canonical.as_bytes());
    let mut preimage = b"pale.diagnostic.identity.v1\0".to_vec();
    preimage.extend_from_slice(canonical.as_bytes());
    assert_eq!(admitted.identity(), pale_core::sha256_hex(&preimage));
    let cold = AdmittedDiagnostic::from_bytes(admitted.canonical_bytes()).unwrap();
    assert_eq!(cold, admitted);
}

#[test]
fn retained_collections_preserve_order_and_count_only_actual_shortening() {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1").unwrap();
    let mut input = projection();
    input.summary = "界".repeat(4_200);
    input.causes = (0..18).map(|index| format!("cause-{index}")).collect();
    input.notes = (0..19).map(|index| format!("note-{index}")).collect();
    input.help = (0..21).map(|index| format!("help-{index}")).collect();
    input.facts = (0..23)
        .map(|index| (format!("fact_{index}"), format!("value-{index}")))
        .collect();
    // Discarded fields must not contribute to the count of shortened fields.
    input.causes[17] = "x".repeat(5_000);
    input.notes[18] = "x".repeat(5_000);
    input.help[20] = "x".repeat(5_000);
    input.facts[22].1 = "x".repeat(5_000);
    input.omitted_causes = u32::MAX;
    let admitted = input.admit(&identity).unwrap();
    let wire = admitted.wire();
    assert_eq!(wire.summary, format!("{}…", "界".repeat(4_095)));
    assert_eq!(wire.summary.chars().count(), 4_096);
    assert_eq!(
        wire.causes,
        (0..16)
            .map(|index| format!("cause-{index}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        wire.notes,
        (0..16)
            .map(|index| format!("note-{index}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        wire.help,
        (0..16)
            .map(|index| format!("help-{index}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        wire.facts
            .iter()
            .map(|fact| (fact.key.clone(), fact.value.clone()))
            .collect::<Vec<_>>(),
        (0..16)
            .map(|index| (format!("fact_{index}"), format!("value-{index}")))
            .collect::<Vec<_>>()
    );
    assert_eq!(wire.truncation.shortened_fields, 1);
    assert_eq!(wire.truncation.omitted_causes, u32::MAX);
    assert_eq!(wire.truncation.omitted_notes, 3);
    assert_eq!(wire.truncation.omitted_help, 5);
    assert_eq!(wire.truncation.omitted_facts, 7);
}

#[test]
fn redaction_precedes_bounds_and_fact_keys_require_schema_and_uniqueness() {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1").unwrap();
    let mut input = projection();
    input.summary = format!("token={}", "界".repeat(5_000));
    let admitted = input.admit(&identity).unwrap();
    assert_eq!(admitted.wire().summary, "token=[REDACTED]");
    assert_eq!(admitted.wire().truncation.shortened_fields, 0);

    let mut input = projection();
    input.facts = vec![
        (format!("{}a", "k".repeat(128)), "first".to_string()),
        (format!("{}b", "k".repeat(128)), "second".to_string()),
    ];
    // Shortening adds an ellipsis, which is outside the fact-key grammar.
    // Structural admission must reject it before semantic uniqueness is checked.
    let Err(DiagnosticAdmissionError::Contract(pale_contracts::SchemaError::Invalid(violations))) =
        input.admit(&identity)
    else {
        panic!("shortened fact keys must still satisfy their registered grammar")
    };
    for path in ["/facts/0/key", "/facts/1/key"] {
        assert!(violations.violations().iter().any(|violation| {
            violation.keyword() == "pattern" && violation.instance_path() == path
        }));
    }

    let mut input = projection();
    let key = "k".repeat(128);
    input.facts = vec![
        (key.clone(), "first".to_string()),
        (key, "second".to_string()),
    ];
    assert!(matches!(
        input.admit(&identity),
        Err(DiagnosticAdmissionError::DuplicateFactKey)
    ));
}

#[test]
fn projected_source_schema_and_effect_claims_still_require_admission() {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1").unwrap();
    let mut input = projection();
    input.source.as_mut().unwrap().label = "/private/input.toml".to_string();
    assert!(matches!(
        input.admit(&identity),
        Err(DiagnosticAdmissionError::UnsafeSourceLabel)
    ));

    let mut input = projection();
    input.schema_reference = Some(("not-registered.v1".to_string(), None));
    assert!(matches!(
        input.admit(&identity),
        Err(DiagnosticAdmissionError::UnknownSchemaCoordinate)
    ));

    let mut input = projection();
    input.disposition = DiagnosticDispositionV1::EffectOutcomeUnknown;
    input.effect = DiagnosticEffectCertaintyV1::OutcomeUnknown;
    assert!(matches!(
        input.admit(&identity),
        Err(DiagnosticAdmissionError::EffectSemanticsMismatch)
    ));

    let mut input = projection();
    input.stage = DiagnosticStageV1::ResidentRecovery;
    input.disposition = DiagnosticDispositionV1::Blocked;
    input.retry = DiagnosticRetryV1::ReconcileFirst;
    let admitted = input.admit(&identity).unwrap();
    assert_eq!(
        admitted.wire().effect,
        DiagnosticEffectCertaintyV1::NotApplicable
    );
}
