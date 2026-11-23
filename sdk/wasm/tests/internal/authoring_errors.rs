//! Compiler-error values retain their SDK-owned fields after projection.

use crate::{WasmAuthoringCorrection, WasmAuthoringCorrectionKind, WasmAuthoringSourceLocation};
use pale_sdk::{PaleApi, PortableWorkspaceDocument, PortableWorkspaceRequest};

fn visibility_error() -> pale_sdk::ApiError {
    macro_rules! document {
        ($label:literal) => {
            PortableWorkspaceDocument::new(
                $label.to_owned(),
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../interfaces/lsp/tests/fixtures/authoring/visibility-widening/",
                    $label
                ))
                .to_vec(),
            )
        };
    }
    let api = PaleApi::new().expect("embedded contracts");
    api.compile_portable_workspace(PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_owned(),
        vec![
            document!("alpha.toml"),
            document!("beta.toml"),
            document!("consumer-module.toml"),
            document!("consumer.toml"),
            document!("module.toml"),
            document!("pale-workspace.toml"),
            document!("resolution.lock.toml"),
            document!("universe.lock.toml"),
        ],
    ))
    .expect_err("fixture widens an import ceiling")
}

#[test]
fn rejected_compilation_retains_every_correction_field_without_its_error() {
    let error = visibility_error();
    let [expected] = error.authoring_corrections() else {
        panic!("compiler must prove exactly one correction");
    };
    let expected = expected.clone();
    let actual = WasmAuthoringCorrection::from_api(&expected);
    let expected_location = error.authoring_location().expect("source location").clone();
    let location = WasmAuthoringSourceLocation::from_api(&expected_location);
    drop(error);

    assert!(matches!(
        actual.kind(),
        WasmAuthoringCorrectionKind::NarrowImportVisibility
    ));
    assert_eq!(actual.source_label(), expected.source_label());
    assert_eq!(
        actual
            .path()
            .into_iter()
            .map(|segment| segment.inner)
            .collect::<Vec<_>>(),
        expected.path()
    );
    assert_eq!(actual.expected_source(), expected.expected_source());
    assert_eq!(actual.replacement_source(), expected.replacement_source());
    assert_eq!(actual.proof_subject().inner, *expected.proof_subject());
    assert_eq!(
        actual.diagnostic_identity(),
        expected.diagnostic_identity().as_str()
    );
    assert_eq!(location.source_label(), expected_location.source_label());
    assert_eq!(
        location
            .path()
            .into_iter()
            .map(|segment| segment.inner)
            .collect::<Vec<_>>(),
        expected_location.path()
    );
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
fn correction_attachment_does_not_change_canonical_diagnostic_bytes() {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(catch, js_namespace = Reflect, js_name = get)]
        fn property(target: &JsValue, key: &JsValue) -> Result<JsValue, JsValue>;
    }

    let error = visibility_error();
    let expected = error.diagnostic().canonical_json().to_owned();
    let expected_identity = error.diagnostic().identity().as_str().to_owned();
    let projected = crate::diagnostic_error(error);
    assert_eq!(
        property(&projected, &JsValue::from_str("diagnosticJson"))
            .expect("diagnostic bytes property")
            .as_string()
            .expect("diagnostic bytes text"),
        expected
    );
    let corrections =
        property(&projected, &JsValue::from_str("authoringCorrections")).expect("correction array");
    let correction = property(&corrections, &JsValue::from_f64(0.0)).expect("first correction");
    assert_eq!(
        property(&correction, &JsValue::from_str("diagnosticIdentity"))
            .expect("correction diagnostic identity")
            .as_string()
            .expect("diagnostic identity text"),
        expected_identity
    );
}
