//! Core-owned success production and unchanged signed-wire admission laws.
//! Existing fixtures model resolved schema and signer authority explicitly.

use super::*;
use std::cell::RefCell;

fn preimage(fixture: &AdmissionFixture) -> CapabilityResultPreimageV1 {
    result_preimage(
        fixture.request.clone(),
        CapabilityResultStatusV1::Succeeded,
        Some(
            CapabilitySuccessV1::inline(
                SchemaId::parse("example/result.v1").unwrap(),
                schema_digest('2'),
                serde_json::json!({"answer":42}),
            )
            .unwrap(),
        ),
        None,
    )
}

fn issue(
    value: CapabilityResultPreimageV1,
    context: &CapabilitySuccessAdmissionContextV1<'_>,
    signs: &Cell<usize>,
) -> Result<CapabilityResultV1, InvalidResidentRecordV1> {
    CapabilityResultV1::issue_success_contextual(
        value,
        context,
        |_| None,
        |_, _, _| Ok(()),
        |_, _, _| {
            signs.set(signs.get() + 1);
            Ok(signature())
        },
    )
}

#[test]
fn inline_issuance_re_admits_with_identical_preimage_digest_signature_and_bytes() {
    let fixture = AdmissionFixture::new();
    let value = preimage(&fixture);
    assert_eq!(value.success.as_ref().unwrap(), &success_payload());
    let expected = CapabilityResultV1::seal(value.clone(), signature()).unwrap();
    let events = RefCell::new(Vec::new());
    let issued = CapabilityResultV1::issue_success_contextual(
        value.clone(),
        &fixture.success_context(),
        |_| panic!("inline payload does not resolve an immutable reference"),
        |id, digest, bytes| {
            events.borrow_mut().push("payload");
            assert_eq!(id.as_str(), "example/result.v1");
            assert_eq!(digest, &schema_digest('2'));
            assert_eq!(bytes, br#"{"answer":42}"#);
            Ok(())
        },
        |domain, actual, digest| {
            events.borrow_mut().push("sign");
            assert_eq!(domain, CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &value);
            assert_eq!(digest, expected.canonical_result_sha256());
            Ok(signature())
        },
    )
    .unwrap();
    assert_eq!(*events.borrow(), ["payload", "sign"]);
    assert_eq!(issued, expected);
    events.borrow_mut().clear();
    let admitted = CapabilityResultV1::admit_success_wire(
        wire(&issued),
        &fixture.success_context(),
        |_| panic!("inline payload does not resolve"),
        |_, _, bytes| {
            events.borrow_mut().push("payload");
            assert_eq!(bytes, br#"{"answer":42}"#);
            Ok(())
        },
        |domain, actual, digest, signed| {
            events.borrow_mut().push("verify");
            assert_eq!(domain, CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &value);
            assert_eq!(digest, issued.canonical_result_sha256());
            assert_eq!(signed, issued.signature());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(*events.borrow(), ["verify", "payload"]);
    assert_eq!(
        crate::canon::to_canonical_string(&admitted).unwrap(),
        crate::canon::to_canonical_string(&issued).unwrap()
    );
}

#[test]
fn schema_preflight_preserves_exact_request_set_reference_digest_and_member() {
    let fixture = AdmissionFixture::new();
    let id = SchemaId::parse("example/result.v1").unwrap();
    let digest = schema_digest('2');
    fixture
        .success_context()
        .validate_result_schema(&id, &digest)
        .unwrap();
    assert!(fixture
        .success_context()
        .validate_result_schema(&SchemaId::parse("example/other.v1").unwrap(), &digest)
        .is_err());
    assert!(fixture
        .success_context()
        .validate_result_schema(&id, &schema_digest('f'))
        .is_err());
    let other_ref = resident_ref("other-contract-set");
    let wrong_ref = CapabilitySuccessAdmissionContextV1 {
        contract_set_ref: &other_ref,
        ..fixture.success_context()
    };
    assert!(wrong_ref.validate_result_schema(&id, &digest).is_err());
    let mut other_set = fixture.contracts.clone();
    // A structurally valid different set retains the same result member.
    let mut other_preimage = ContractSetPreimageV1 {
        schema_id: other_set.schema_id().clone(),
        schema_sha256: other_set.schema_sha256().clone(),
        contract_set_id: ContractSetIdV1::parse("contract-set-2").unwrap(),
        entries: other_set.entries().clone(),
        compatibility_profile_ref: other_set.compatibility_profile_ref().clone(),
        compatibility_profile_sha256: other_set.compatibility_profile_sha256().clone(),
    };
    other_set = ContractSetV1::seal(other_preimage.clone()).unwrap();
    let wrong_set = CapabilitySuccessAdmissionContextV1 {
        contract_set: &other_set,
        ..fixture.success_context()
    };
    assert!(wrong_set.validate_result_schema(&id, &digest).is_err());
    for context in [wrong_ref, wrong_set] {
        let signs = Cell::new(0);
        assert!(issue(preimage(&fixture), &context, &signs).is_err());
        assert_eq!(signs.get(), 0);
    }
    // Restore the full original set identity: equivalent owned values remain valid.
    other_preimage.contract_set_id = fixture.contracts.contract_set_id().clone();
    let equal_set = ContractSetV1::seal(other_preimage).unwrap();
    CapabilitySuccessAdmissionContextV1 {
        contract_set: &equal_set,
        ..fixture.success_context()
    }
    .validate_result_schema(&id, &digest)
    .unwrap();
}

#[test]
fn issuance_rejects_transposed_request_attempt_implementation_and_signer_before_signing() {
    let fixture = AdmissionFixture::new();
    let mut values = Vec::new();
    for (member, replacement) in [
        ("request_id", serde_json::json!("request-2")),
        ("session_grant_id", serde_json::json!("grant-2")),
        ("route_binding_ref", serde_json::json!("route-2")),
        ("contract_set_ref", serde_json::json!("contract-set-2")),
        ("canonical_request_sha256", serde_json::json!(hex('e'))),
    ] {
        let mut value = preimage(&fixture);
        value.request_binding = request_with(&fixture.request, member, replacement);
        values.push(value);
    }
    let mut value = preimage(&fixture);
    value.attempt_id = CapabilityAttemptIdV1::parse("attempt-2").unwrap();
    values.push(value);
    let mut value = preimage(&fixture);
    value.implementation_kind = BoundedResidentTextV1::parse("other").unwrap();
    values.push(value);
    let mut value = preimage(&fixture);
    value.implementation_version = BoundedResidentTextV1::parse("2").unwrap();
    values.push(value);
    let mut value = preimage(&fixture);
    value.signing_authority_binding = signing_authority_for("producer-2", "producer-key-2");
    values.push(value);
    let mut value = preimage(&fixture);
    value.issuer_principal_id = ResidentPrincipalIdV1::parse("producer-2").unwrap();
    values.push(value);
    let mut value = preimage(&fixture);
    value.issuer_key_id = ResidentKeyIdV1::parse("producer-key-2").unwrap();
    values.push(value);
    let mut value = preimage(&fixture);
    value.issuer_key_epoch = SigningKeyEpochV1::new(2).unwrap();
    values.push(value);
    for value in values {
        let signs = Cell::new(0);
        assert!(issue(value, &fixture.success_context(), &signs).is_err());
        assert_eq!(signs.get(), 0);
    }
}

#[test]
fn issuance_rejects_schema_chronology_branch_and_representation_before_signing() {
    let fixture = AdmissionFixture::new();
    let mut values = Vec::new();
    let mut value = preimage(&fixture);
    value.schema_id = SchemaId::parse("example/not-capability-result.v1").unwrap();
    values.push(value);
    let mut value = preimage(&fixture);
    value.schema_sha256 = schema_digest('f');
    values.push(value);
    let mut value = preimage(&fixture);
    value.completed_at = UnixTimeV1::new(9).unwrap();
    values.push(value);
    let mut value = preimage(&fixture);
    value.status = CapabilityResultStatusV1::Failed;
    values.push(value);
    let mut value = preimage(&fixture);
    value.success = None;
    values.push(value);
    let mut value = preimage(&fixture);
    value.success.as_mut().unwrap().immutable_result_ref = Some(resident_ref("also-immutable"));
    values.push(value);
    let mut value = preimage(&fixture);
    value.success.as_mut().unwrap().inline_payload = None;
    values.push(value);
    let mut value = preimage(&fixture);
    value.success.as_mut().unwrap().result_schema_sha256 = schema_digest('f');
    values.push(value);
    let mut value = preimage(&fixture);
    value.success = Some(
        CapabilitySuccessV1::inline(
            SchemaId::parse("example/unselected.v1").unwrap(),
            schema_digest('2'),
            serde_json::json!({"answer":42}),
        )
        .unwrap(),
    );
    values.push(value);
    for value in values {
        let signs = Cell::new(0);
        assert!(issue(value, &fixture.success_context(), &signs).is_err());
        assert_eq!(signs.get(), 0);
    }
}

#[test]
fn inline_constructor_hashes_exact_canonical_utf8_and_enforces_existing_limits() {
    let id = SchemaId::parse("example/result.v1").unwrap();
    let schema = schema_digest('2');
    // Quotes take two bytes; each remaining Unicode character takes two bytes.
    let boundary = serde_json::Value::String("é".repeat((MAX_RESIDENT_CONTENT_BYTES - 2) / 2));
    let value = CapabilitySuccessV1::inline(id.clone(), schema.clone(), boundary.clone()).unwrap();
    let canonical = crate::canon::to_canonical_string(&boundary).unwrap();
    assert_eq!(canonical.len(), MAX_RESIDENT_CONTENT_BYTES);
    assert_eq!(
        value.result_payload_sha256(),
        &ArtifactDigest::hash_bytes(canonical.as_bytes())
    );
    assert_eq!(value.inline_payload(), Some(&boundary));
    assert!(value.immutable_result_ref().is_none());
    let oversized = serde_json::Value::String("x".repeat(MAX_RESIDENT_CONTENT_BYTES - 1));
    let members =
        serde_json::Value::Array(vec![serde_json::Value::Null; MAX_RESIDENT_JSON_MEMBERS + 1]);
    let mut depth = serde_json::Value::Null;
    for _ in 0..MAX_RESIDENT_JSON_DEPTH {
        depth = serde_json::Value::Array(vec![depth]);
    }
    for payload in [
        oversized,
        members,
        depth,
        serde_json::json!(crate::canon::JSON_SAFE_INTEGER_MAX + 1),
    ] {
        assert!(CapabilitySuccessV1::inline(id.clone(), schema.clone(), payload).is_err());
    }
}

#[test]
fn issuer_rechecks_payload_bounds_digest_and_admission_before_signing() {
    let fixture = AdmissionFixture::new();
    let mut wrong_digest = preimage(&fixture);
    wrong_digest.success.as_mut().unwrap().result_payload_sha256 = artifact_digest('f');
    let mut oversized = preimage(&fixture);
    oversized.success.as_mut().unwrap().inline_payload = Some(serde_json::Value::String(
        "x".repeat(MAX_RESIDENT_CONTENT_BYTES),
    ));
    for value in [wrong_digest, oversized] {
        let signs = Cell::new(0);
        assert!(issue(value, &fixture.success_context(), &signs).is_err());
        assert_eq!(signs.get(), 0);
    }
    let signs = Cell::new(0);
    let result = CapabilityResultV1::issue_success_contextual(
        preimage(&fixture),
        &fixture.success_context(),
        |_| None,
        |_, _, _| Err(contextual("test payload", "actual payload owner rejected")),
        |_, _, _| {
            signs.set(signs.get() + 1);
            Ok(signature())
        },
    );
    assert!(result.is_err());
    assert_eq!(signs.get(), 0);
}

#[test]
fn immutable_issuance_reuses_exact_resolution_and_preserves_wire_callback_order() {
    let fixture = AdmissionFixture::new();
    let bytes = b"owner-retained-result".to_vec();
    let mut value = preimage(&fixture);
    value.success = Some(immutable_success(&bytes));
    let events = RefCell::new(Vec::new());
    let issued = CapabilityResultV1::issue_success_contextual(
        value,
        &fixture.success_context(),
        |reference| {
            events.borrow_mut().push("resolve");
            assert_eq!(reference.as_str(), "immutable-result-1");
            Some(bytes.clone())
        },
        |_, _, actual| {
            events.borrow_mut().push("payload");
            assert_eq!(actual, bytes);
            Ok(())
        },
        |_, _, _| {
            events.borrow_mut().push("sign");
            Ok(signature())
        },
    )
    .unwrap();
    assert_eq!(*events.borrow(), ["resolve", "payload", "sign"]);
    events.borrow_mut().clear();
    let admitted = CapabilityResultV1::admit_success_wire(
        wire(&issued),
        &fixture.success_context(),
        |_| {
            events.borrow_mut().push("resolve");
            Some(bytes.clone())
        },
        |_, _, _| {
            events.borrow_mut().push("payload");
            Ok(())
        },
        |_, _, _, _| {
            events.borrow_mut().push("verify");
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(*events.borrow(), ["verify", "resolve", "payload"]);
    assert_eq!(admitted, issued);
    for resolved in [
        None,
        Some(b"different bytes".to_vec()),
        Some(vec![0; MAX_RESIDENT_CONTENT_BYTES + 1]),
    ] {
        let mut value = preimage(&fixture);
        value.success = Some(immutable_success(&bytes));
        let signs = Cell::new(0);
        assert!(CapabilityResultV1::issue_success_contextual(
            value,
            &fixture.success_context(),
            |_| resolved.clone(),
            |_, _, _| Ok(()),
            |_, _, _| {
                signs.set(signs.get() + 1);
                Ok(signature())
            },
        )
        .is_err());
        assert_eq!(signs.get(), 0);
    }
}

#[test]
fn signer_failure_returns_no_record_and_bad_wire_signature_stops_payload_work() {
    let fixture = AdmissionFixture::new();
    let payloads = Cell::new(0);
    let result = CapabilityResultV1::issue_success_contextual(
        preimage(&fixture),
        &fixture.success_context(),
        |_| None,
        |_, _, _| {
            payloads.set(payloads.get() + 1);
            Ok(())
        },
        |_, _, _| Err(contextual("test signer", "signing failed")),
    );
    assert!(result.is_err());
    assert_eq!(payloads.get(), 1);
    let record = CapabilityResultV1::seal(preimage(&fixture), signature()).unwrap();
    payloads.set(0);
    assert!(CapabilityResultV1::admit_success_wire(
        wire(&record),
        &fixture.success_context(),
        |_| panic!("no resolution after bad signature"),
        |_, _, _| {
            payloads.set(payloads.get() + 1);
            Ok(())
        },
        |_, _, _, _| Err(contextual("test signer", "signature rejected")),
    )
    .is_err());
    assert_eq!(payloads.get(), 0);
}
