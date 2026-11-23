//! SDK-owned A2A delegate-handle admission tests.

use pale_core::records::OperationHandleDigest;

use super::*;

const HANDLE_REF: &str = "operations/delegate-handle-1";

fn sha(character: char) -> String {
    std::iter::repeat_n(character, 64).collect()
}

fn canonical(value: &serde_json::Value) -> Vec<u8> {
    pale_core::canon::to_canonical_string(value)
        .expect("fixture is canonical")
        .into_bytes()
}

fn schema_sha256(schema_id: &str) -> String {
    pale_contracts::schema_descriptor(schema_id)
        .expect("schema is embedded")
        .schema_sha256
        .to_owned()
}

fn handle_preimage(operation_class: &str) -> serde_json::Value {
    serde_json::json!({
        "schema_id": "pale/operation-handle.v1",
        "schema_sha256": schema_sha256("operation-handle.v1"),
        "operation_handle_id": "delegate-operation-1",
        "operation_class": operation_class,
        "request_binding": {
            "request_id": "request-1",
            "correlation_id": "correlation-1",
            "capability_id": "remote.specialist.review",
            "canonical_request_sha256": sha('2'),
            "session_grant_id": "grant-1",
            "session_grant_sha256": sha('3'),
            "deployment_binding_ref": "deployment-binding-1",
            "deployment_binding_sha256": sha('4'),
            "route_binding_schema_id": "pale/operation-route-binding.v1",
            "route_binding_schema_sha256": sha('5'),
            "route_binding_ref": "route-binding-1",
            "route_binding_sha256": sha('6'),
            "route_generation": 1,
            "contract_set_ref": "contract-set-1",
            "contract_set_sha256": sha('7')
        },
        "operation_lease_ref": "operation-lease-1",
        "operation_lease_sha256": sha('8'),
        "compiled_generation_sha256": sha('9'),
        "activation_fence_sha256": sha('a'),
        "instance_sha256": sha('b'),
        "created_at": 10,
        "expires_at": 20,
        "retain_until": 30,
        "deployment_runtime_authority_ref": "runtime-authority-1",
        "deployment_runtime_authority_sha256": sha('c'),
        "issuer_principal_id": "principal-1",
        "issuer_key_id": "key-1",
        "issuer_key_epoch": 1
    })
}

fn sealed_handle(operation_class: &str) -> Vec<u8> {
    let preimage = handle_preimage(operation_class);
    let digest = OperationHandleDigest::hash_canonical(&preimage)
        .expect("handle preimage hashes")
        .as_str()
        .to_owned();
    let mut handle = preimage;
    handle["canonical_operation_handle_sha256"] = serde_json::json!(digest);
    handle["signature"] = serde_json::json!("A".repeat(86));
    canonical(&handle)
}

#[test]
fn exact_delegate_handle_becomes_an_owned_projection_without_authority() {
    let bytes = sealed_handle("delegate");
    let expected = pale_core::canon::parse_strict(&bytes).expect("fixture parses");
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let inspected = api
        .inspect_a2a_delegate_handle(A2aDelegateHandleRequest::new(
            HANDLE_REF.to_owned(),
            bytes.clone(),
        ))
        .expect("exact Delegate handle is structurally admitted");

    assert_eq!(inspected.record_ref(), HANDLE_REF);
    assert_eq!(inspected.operation_handle_id(), "delegate-operation-1");
    assert_eq!(
        inspected.operation_handle_sha256(),
        expected["canonical_operation_handle_sha256"]
            .as_str()
            .expect("digest")
    );
    assert_eq!(inspected.canonical_handle(), bytes);
}

#[test]
fn other_operation_classes_and_substituted_bytes_fail_closed() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let wrong_class = api
        .inspect_a2a_delegate_handle(A2aDelegateHandleRequest::new(
            HANDLE_REF.to_owned(),
            sealed_handle("invoke"),
        ))
        .expect_err("Invoke is not a Delegate identity");
    assert_eq!(
        wrong_class.diagnostic().code(),
        "a2a_delegate_handle_invalid"
    );

    let mut substituted = sealed_handle("delegate");
    let index = substituted
        .iter()
        .position(|byte| *byte == b'd')
        .expect("fixture includes delegate text");
    substituted[index] = b'D';
    let changed = api
        .inspect_a2a_delegate_handle(A2aDelegateHandleRequest::new(
            HANDLE_REF.to_owned(),
            substituted,
        ))
        .expect_err("substituted bytes fail canonical contract or digest admission");
    assert_eq!(changed.diagnostic().code(), "a2a_delegate_handle_invalid");
}

#[test]
fn invalid_reference_and_noncanonical_json_are_rejected() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    for request in [
        A2aDelegateHandleRequest::new(String::new(), sealed_handle("delegate")),
        A2aDelegateHandleRequest::new(
            HANDLE_REF.to_owned(),
            b"{ \"not\": \"canonical\" }".to_vec(),
        ),
    ] {
        let error = api
            .inspect_a2a_delegate_handle(request)
            .expect_err("invalid boundary input is rejected");
        assert_eq!(error.diagnostic().code(), "a2a_delegate_handle_invalid");
    }
}
