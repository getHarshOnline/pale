//! Exercises readable bundle publication, verification, and immutable coordinate derivation.

use super::*;

fn live_bundle_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("codegen crate is below the repository root")
        .join("contracts/bundle/agent-surface.v1")
        .join(BUNDLE_MANIFEST_NAME)
}

#[test]
fn generated_json_has_deterministic_two_space_indentation_and_one_newline() {
    let bytes = pretty_json_document(&serde_json::json!({"z": 1, "a": [2, 1]}))
        .expect("pretty JSON document");
    assert_eq!(
        bytes,
        b"{\n  \"a\": [\n    2,\n    1\n  ],\n  \"z\": 1\n}\n"
    );
}

#[test]
fn atomic_writer_publishes_once_and_verifier_rejects_drift() {
    let directory = tempfile::tempdir().expect("test directory");
    let destination = directory.path().join("bundle");
    let schema_bytes = b"{\n  \"$id\": \"https://example.test/example.v1.json\"\n}\n".to_vec();
    let manifest_bytes = b"{\n  \"registry\": \"test\"\n}\n".to_vec();
    let schema_sha256 = SchemaDigest::hash_bytes(&schema_bytes);
    let bundle = ContractBundle {
        manifest_bytes,
        manifest_canonical_preimage: "{\"registry\":\"test\"}".into(),
        schemas: BTreeMap::from([(
            "example.v1".into(),
            GeneratedSchema {
                path: PathBuf::from("schemas/example.v1.json"),
                bytes: schema_bytes,
                schema_uri: "https://example.test/example.v1.json".into(),
                wire_schema_id: None,
                wire_schema_version: None,
                schema_sha256: schema_sha256.clone(),
                raw_schema_sha256_carriage: "absent".into(),
                manifest_entry_canonical: "{}".into(),
            },
        )]),
        coordinate: BundleCoordinate {
            schema_manifest_sha256: SchemaManifestDigest::hash_canonical(
                &serde_json::json!({"registry": "test"}),
            )
            .expect("manifest digest"),
            schema_set_sha256: schema_set_digest([SchemaSetMember {
                schema_id: "example.v1".into(),
                schema_sha256: schema_sha256.clone(),
            }])
            .expect("set digest"),
        },
    };

    assert_eq!(
        bundle.manifest_canonical_preimage(),
        "{\"registry\":\"test\"}"
    );
    let descriptor = bundle
        .schema_descriptors()
        .next()
        .expect("generated descriptor");
    assert_eq!(descriptor.schema_id(), "example.v1");
    assert_eq!(
        descriptor.source_path(),
        Path::new("schemas/example.v1.json")
    );
    assert_eq!(
        descriptor.schema_uri(),
        "https://example.test/example.v1.json"
    );
    assert_eq!(descriptor.wire_schema_id(), None);
    assert_eq!(descriptor.wire_schema_version(), None);
    assert_eq!(descriptor.schema_sha256(), &schema_sha256);
    assert_eq!(descriptor.raw_schema_sha256_carriage(), "absent");
    assert_eq!(descriptor.manifest_entry_canonical(), "{}");
    assert_eq!(
        descriptor.bytes(),
        b"{\n  \"$id\": \"https://example.test/example.v1.json\"\n}\n"
    );

    write_bundle_atomically(&bundle, &destination).expect("atomic publish");
    verify_bundle(&bundle, &destination).expect("exact verification");
    assert!(write_bundle_atomically(&bundle, &destination).is_err());
    fs::write(destination.join("schemas/example.v1.json"), b"{}\n").expect("corrupt test output");
    assert!(verify_bundle(&bundle, &destination).is_err());
}

#[test]
fn qualification_rejects_a_route_that_disagrees_with_manifest_custody() {
    let manifest_path = live_bundle_manifest();
    let mut manifest = read_descriptor_manifest(&manifest_path).expect("admit live manifest");
    manifest
        .schemas
        .get_mut("hook-context.v1")
        .expect("generated projection descriptor")
        .lifecycle
        .role = LifecycleRole::DurableState;

    let error = generate_bundle(manifest).expect_err("route/custody mismatch must reject");
    assert!(
        error
            .to_string()
            .contains("qualification route for `hook-context.v1` disagrees"),
        "unexpected qualification error: {error}"
    );
}

#[test]
fn qualification_rejects_an_unproved_pale_owned_witness() {
    let manifest_path = live_bundle_manifest();
    let mut manifest = read_descriptor_manifest(&manifest_path).expect("admit live manifest");
    let descriptor = manifest
        .schemas
        .get_mut("deployment-acceptance.v1")
        .expect("external owner descriptor");
    descriptor.record_authority = RecordAuthority::PaleRuntime;
    descriptor.producers[0].actor_type = ActorType::ProtocolParticipant;
    descriptor.producers[0].implementation_state = ImplementationState::PlannedUnproved;

    let error = generate_bundle(manifest).expect_err("unproved Pale owner must reject");
    assert!(
        error.to_string().contains(
            "Pale-owned qualification `deployment-acceptance.v1` has no observed owner witness"
        ),
        "unexpected qualification error: {error}"
    );
}

#[test]
fn observed_protocol_participants_are_admitted_without_changing_actor_kind() {
    let manifest_path = live_bundle_manifest();
    let mut manifest = read_descriptor_manifest(&manifest_path).expect("admit live manifest");
    let consumer = manifest
        .schemas
        .get_mut("diagnostic.v1")
        .expect("diagnostic descriptor")
        .consumers
        .first_mut()
        .expect("diagnostic consumer");
    consumer.actor_type = ActorType::ProtocolParticipant;
    consumer.implementation_state = ImplementationState::Observed;

    generate_bundle(manifest).expect("an evidenced protocol participant remains correctly typed");
}
