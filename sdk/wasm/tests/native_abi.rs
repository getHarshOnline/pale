//! Node-hosted WebAssembly ABI smoke tests.

use pale_wasm::PaleWasm;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = Reflect, js_name = get)]
    fn test_javascript_property(target: &JsValue, key: &JsValue) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen_test]
fn bundle_identity_crosses_the_wasm_boundary() {
    let api = pale_sdk::PaleApi::new().expect("API initializes");
    let wasm = PaleWasm::new().expect("WASM façade initializes");
    let expected = api.contract_bundle_identity();
    let actual = wasm.contract_bundle_identity();
    assert_eq!(
        actual.schema_manifest_sha256(),
        expected.schema_manifest_sha256()
    );
    assert_eq!(actual.schema_set_sha256(), expected.schema_set_sha256());
    assert_eq!(actual.member_count(), expected.member_count());
}

#[wasm_bindgen_test]
fn canonical_diagnostic_crosses_the_wasm_error_boundary() {
    let api = pale_sdk::PaleApi::new().expect("API initializes");
    let failure = api
        .contract_schema("not-a-schema")
        .expect_err("unknown schema is rejected");
    let expected_summary = failure.diagnostic().summary().to_owned();
    let expected_json = failure.diagnostic().canonical_json().to_owned();
    let wasm = PaleWasm::new().expect("WASM façade initializes");
    let actual = match wasm.contract_schema("not-a-schema") {
        Err(diagnostic) => diagnostic,
        Ok(_) => panic!("unknown schema must be rejected"),
    };
    let message = test_javascript_property(&actual, &JsValue::from_str("message"))
        .expect("native error exposes message")
        .as_string()
        .expect("message is a string");
    let diagnostic_json = test_javascript_property(&actual, &JsValue::from_str("diagnosticJson"))
        .expect("native error exposes canonical diagnostic text")
        .as_string()
        .expect("diagnosticJson is a string");
    assert_eq!(message, expected_summary);
    assert_eq!(diagnostic_json, expected_json);
}
