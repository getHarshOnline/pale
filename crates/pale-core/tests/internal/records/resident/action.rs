//! Exercises contextual action-envelope admission, bounded payloads, and canonical identity.

use super::*;

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn preimage(payload: serde_json::Value) -> ActionEnvelopePreimageV1 {
    let canonical = crate::canon::to_canonical_string(&payload).expect("canonical payload");
    ActionEnvelopePreimageV1 {
        action_id: ActionIdV1::parse("action-1").expect("action id"),
        target_system: ActionTargetSystemV1::parse("crm").expect("target system"),
        target_identity: ActionTargetIdentityV1::parse("contact-1").expect("target identity"),
        conflict_resource_key_schema_id: SchemaId::parse("example/resource-key.v1")
            .expect("resource key schema"),
        conflict_resource_key_schema_sha256: SchemaDigest::parse(sha('a'))
            .expect("resource key schema digest"),
        conflict_resource_key: serde_json::json!({"contact":"contact-1"}),
        operation: ActionOperationV1::parse("update").expect("operation"),
        business_idempotency_key: BusinessIdempotencyKeyV1::parse("business-1")
            .expect("business key"),
        expected_precondition_digest: None,
        payload_schema_id: SchemaId::parse("example/action-payload.v1").expect("payload schema"),
        payload_schema_sha256: SchemaDigest::parse(sha('b')).expect("payload schema digest"),
        payload_hash: ArtifactDigest::hash_bytes(canonical.as_bytes()),
        inline_payload: Some(payload),
        immutable_payload_ref: None,
        effect_policy: ResidentRecordRefV1::parse("effect-policy").expect("effect policy"),
        reconciliation_policy: ResidentRecordRefV1::parse("reconciliation-policy")
            .expect("reconciliation policy"),
    }
}

#[test]
fn action_wire_requires_explicit_payload_admission() {
    let action = ActionEnvelopeV1::admit(preimage(serde_json::json!({"status":"ready"})), |_| None)
        .expect("inline action admits");
    let wire: ActionEnvelopeWireV1 =
        serde_json::from_value(serde_json::to_value(&action).expect("action wire serializes"))
            .expect("structural wire decodes");
    let admitted = ActionEnvelopeV1::admit_wire(wire, |_| None)
        .expect("explicit contextual admission succeeds");
    assert_eq!(admitted.action_id(), action.action_id());
    assert_eq!(
        admitted
            .canonical_action_body_sha256()
            .expect("action digest"),
        action
            .canonical_action_body_sha256()
            .expect("action digest")
    );
}

#[test]
fn action_json_bounds_cover_depth_and_member_count() {
    let mut deep = serde_json::json!(null);
    for _ in 0..=32 {
        deep = serde_json::json!([deep]);
    }
    assert!(matches!(
        ActionEnvelopeV1::admit(preimage(deep), |_| None),
        Err(InvalidResidentRecordV1::BoundExceeded { .. })
    ));

    let members = (0..=4_096)
        .map(|index| serde_json::Value::from(index as u64))
        .collect();
    assert!(matches!(
        ActionEnvelopeV1::admit(preimage(serde_json::Value::Array(members)), |_| None),
        Err(InvalidResidentRecordV1::BoundExceeded { .. })
    ));
}
