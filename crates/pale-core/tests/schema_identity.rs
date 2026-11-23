//! Wire schema-identity grammar and typed-carriage tests.

use pale_core::records::{NativeActionCatalogV1, RecordSchemaIdentity, SchemaDigest, SchemaId};

#[test]
fn schema_id_projection_matches_its_admission_grammar() {
    let schema = serde_json::to_value(schemars::schema_for!(SchemaId)).unwrap();
    assert_eq!(
        schema.get("minLength").and_then(serde_json::Value::as_u64),
        Some(1)
    );
    assert_eq!(
        schema.get("maxLength").and_then(serde_json::Value::as_u64),
        Some(256)
    );
    assert_eq!(
        schema.get("pattern").and_then(serde_json::Value::as_str),
        Some("^[a-z0-9](?:[a-z0-9._/-]*[a-z0-9])?$")
    );
}

#[test]
fn schema_id_accepts_canonical_wire_vocabularies_and_round_trips() {
    for value in [
        "pale.agent-surface.receipt.v1",
        "pale/reconciliation-frontier.v1",
        "pale/internal_dispatch-evidence.v1",
    ] {
        let id = SchemaId::parse(value).expect("canonical schema ID");
        assert_eq!(id.as_str(), value);
        let encoded = serde_json::to_string(&id).expect("encode schema ID");
        assert_eq!(
            serde_json::from_str::<SchemaId>(&encoded).expect("decode schema ID"),
            id
        );
    }
}

#[test]
fn schema_id_rejects_ambiguous_or_unbounded_spellings() {
    for value in [
        "", ".pale.v1", "pale.v1/", "Pale.v1", "pale v1", "pale\nv1", "pale:v1",
    ] {
        assert!(SchemaId::parse(value).is_err(), "accepted `{value:?}`");
    }
    assert!(SchemaId::parse("x".repeat(257)).is_err());
}

#[test]
fn record_identity_keeps_wire_id_and_raw_digest_in_distinct_typed_members() {
    let identity = RecordSchemaIdentity {
        schema: SchemaId::parse("pale/example.v1").unwrap(),
        schema_sha256: SchemaDigest::hash_bytes(b"exact schema bytes"),
    };
    let value = serde_json::to_value(&identity).unwrap();
    assert_eq!(value["schema"], "pale/example.v1");
    assert_eq!(
        value["schema_sha256"],
        pale_core::sha256_hex(b"exact schema bytes")
    );
}

#[test]
fn self_carried_record_schema_id_is_wire_compatible_and_rejects_bad_decode() {
    let record = NativeActionCatalogV1 {
        schema: SchemaId::parse("pale/native-action-catalog.v1").unwrap(),
        schema_sha256: SchemaDigest::hash_bytes(b"schema"),
        schema_version: 1,
        contracts: Vec::new(),
    };
    let mut value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["schema"], "pale/native-action-catalog.v1");
    assert!(serde_json::from_value::<NativeActionCatalogV1>(value.clone()).is_ok());

    value["schema"] = serde_json::Value::String("Pale/native-action-catalog.v1".into());
    assert!(serde_json::from_value::<NativeActionCatalogV1>(value).is_err());
}
