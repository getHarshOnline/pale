//! Typed digest-domain, host-identity, streaming, and schema-set tests.

use pale_core::records::{
    schema_set_digest, ArtifactDigest, ProfileDigest, SchemaDigest, SchemaSetMember,
    SessionIdentityDigest, TurnIdentityDigest,
};

#[test]
fn typed_hash_constructors_preserve_the_canonical_digest_contract() {
    let artifact = b"exact artifact bytes";
    assert_eq!(
        ArtifactDigest::hash_bytes(artifact).as_str(),
        pale_core::sha256_hex(artifact)
    );

    let profile_preimage = serde_json::json!({"z": 1, "a": ["stable"]});
    assert_eq!(
        ProfileDigest::hash_canonical(&profile_preimage)
            .unwrap()
            .as_str(),
        pale_core::canon::digest(&profile_preimage).unwrap()
    );
}

#[test]
fn host_identity_digests_retain_no_raw_identity() {
    let session = SessionIdentityDigest::try_from_host_identity("session-private").unwrap();
    let turn = TurnIdentityDigest::try_from_host_identity("turn-private").unwrap();

    assert_eq!(session.as_str(), pale_core::sha256_hex(b"session-private"));
    assert_eq!(turn.as_str(), pale_core::sha256_hex(b"turn-private"));
    assert!(!serde_json::to_string(&session)
        .unwrap()
        .contains("session-private"));
    assert!(!serde_json::to_string(&turn)
        .unwrap()
        .contains("turn-private"));
}

#[test]
fn host_identity_bounds_are_shared_but_errors_name_the_domain() {
    for invalid in ["", "line\nbreak"] {
        assert!(SessionIdentityDigest::try_from_host_identity(invalid)
            .unwrap_err()
            .to_string()
            .contains("session identity"));
        assert!(TurnIdentityDigest::try_from_host_identity(invalid)
            .unwrap_err()
            .to_string()
            .contains("turn identity"));
    }

    let oversized = "x".repeat(1_025);
    assert!(SessionIdentityDigest::try_from_host_identity(&oversized).is_err());
    assert!(TurnIdentityDigest::try_from_host_identity(&oversized).is_err());
}

#[test]
fn streaming_and_in_memory_hashes_are_identical() {
    let bytes = vec![0x5a; 128 * 1024 + 7];
    assert_eq!(
        pale_core::sha256_reader(bytes.as_slice()).unwrap(),
        pale_core::sha256_hex(&bytes)
    );
}

#[test]
fn schema_set_digest_is_order_independent_and_domain_separated() {
    let first = SchemaSetMember {
        schema_id: "z.v1".into(),
        schema_sha256: SchemaDigest::hash_bytes(b"z"),
    };
    let second = SchemaSetMember {
        schema_id: "a.v1".into(),
        schema_sha256: SchemaDigest::hash_bytes(b"a"),
    };

    let forward = schema_set_digest([first.clone(), second.clone()]).unwrap();
    let reverse = schema_set_digest([second.clone(), first.clone()]).unwrap();
    assert_eq!(forward, reverse);

    let sorted = [second, first];
    let canonical = pale_core::canon::to_canonical_string(&sorted).unwrap();
    let mut preimage = b"PALE_SCHEMA_SET_V1\0".to_vec();
    preimage.extend_from_slice(canonical.as_bytes());
    assert_eq!(forward.as_str(), pale_core::sha256_hex(&preimage));
    assert_ne!(
        forward.as_str(),
        pale_core::sha256_hex(canonical.as_bytes())
    );
}

#[test]
fn schema_set_digest_rejects_duplicate_registry_ids() {
    let member = SchemaSetMember {
        schema_id: "duplicate.v1".into(),
        schema_sha256: SchemaDigest::hash_bytes(b"schema"),
    };
    let error = schema_set_digest([member.clone(), member]).unwrap_err();
    assert!(error.to_string().contains("duplicate schema id"));
}

#[test]
fn digest_wire_projection_owns_the_exact_lowercase_shape() {
    let schema = serde_json::to_value(schemars::schema_for!(SchemaDigest)).unwrap();
    assert_eq!(
        schema.get("type").and_then(serde_json::Value::as_str),
        Some("string")
    );
    assert_eq!(
        schema.get("pattern").and_then(serde_json::Value::as_str),
        Some("^[0-9a-f]{64}$")
    );
}
