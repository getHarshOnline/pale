//! A signature must cover the original occurrence, not a later parsed substitute.

use super::*;

const EVENT: &[u8] = br#"{ "hook_event_name":"PreToolUse", "session_id":"native-session", "tool_use_id":"native-tool-1", "tool_name":"Read", "tool_input":{"file_path":"source.rs"} }"#;

fn route() -> RepositoryHookRoutingV1 {
    RepositoryHookRoutingV1::parse(&[
        "hook",
        "pre-tool-use",
        "--workspace",
        "workspace-a",
        "--repository",
        "repository-a",
        "--provider-binding",
        "terminal-a",
        "--provider-binding-identity",
        &"a".repeat(64),
    ])
    .unwrap()
}

#[test]
fn preserves_native_bytes_across_body_admission() {
    let prepared = ResidentNativeToolStart::prepare(route(), EVENT).unwrap();
    let admitted = ResidentNativeToolStart::admit(prepared.body()).unwrap();
    assert_eq!(admitted.body(), prepared.body());
    assert_eq!(
        admitted.body().payload["native_event"],
        std::str::from_utf8(EVENT).unwrap()
    );
    assert_eq!(
        admitted.request().native_event_sha256(),
        &ArtifactDigest::hash_bytes(EVENT)
    );
    assert_eq!(admitted.invocation().as_str(), "native-tool-1");
    assert!(!format!("{admitted:?}").contains("native-session"));
    let compact = canon::to_canonical_string(&canon::parse_strict(EVENT).unwrap()).unwrap();
    let different = ResidentNativeToolStart::prepare(route(), compact.as_bytes()).unwrap();
    assert_ne!(
        prepared.body().payload_sha256,
        different.body().payload_sha256
    );
}

#[test]
fn rejects_body_schema_digest_and_route_substitution() {
    let prepared = ResidentNativeToolStart::prepare(route(), EVENT).unwrap();
    let original = prepared.body();
    let mut variants = Vec::new();
    let mut changed = original.clone();
    changed.schema_id = SchemaId::parse("action-input.v1").unwrap();
    variants.push(changed);
    let mut changed = original.clone();
    changed.schema_sha256 = SchemaDigest::hash_bytes(b"another schema");
    variants.push(changed);
    let mut changed = original.clone();
    changed.payload["native_event"] = "{}".into();
    variants.push(changed);
    let mut changed = original.clone();
    changed.payload["workspace_id"] = "another-workspace".into();
    variants.push(changed);
    let mut changed = original.clone();
    changed.payload["unknown"] = true.into();
    changed.payload_sha256 = payload_digest(&changed.payload).unwrap();
    variants.push(changed);
    let mut changed = original.clone();
    changed.payload["event"] = "session-start".into();
    changed.payload_sha256 = payload_digest(&changed.payload).unwrap();
    variants.push(changed);
    for body in variants {
        assert!(ResidentNativeToolStart::admit(&body).is_err());
    }
}

#[test]
fn rejects_ambiguous_and_unidentified_occurrences() {
    let original = canon::parse_strict(EVENT).unwrap();
    for field in ["session_id", "tool_use_id", "tool_name"] {
        let mut event = original.clone();
        event.as_object_mut().unwrap().remove(field);
        assert!(
            ResidentNativeToolStart::prepare(route(), &serde_json::to_vec(&event).unwrap())
                .is_err()
        );
    }
    let duplicate =
        std::str::from_utf8(EVENT)
            .unwrap()
            .replacen('{', "{\"tool_use_id\":\"substitute\",", 1);
    assert!(ResidentNativeToolStart::prepare(route(), duplicate.as_bytes()).is_err());
    assert!(ResidentNativeToolStart::prepare(
        route(),
        &vec![b' '; RESIDENT_HOOK_EVENT_BYTE_LIMIT + 1]
    )
    .is_err());
    let mut event = original;
    event["hook_event_name"] = "PostToolUse".into();
    assert!(
        ResidentNativeToolStart::prepare(route(), &serde_json::to_vec(&event).unwrap()).is_err()
    );
}
