//! Real compiler-produced correction values through Node's copied getters.

use crate::{AuthoringCorrection, AuthoringCorrectionKind, AuthoringIdentityKind};
use pale_sdk::{PaleApi, PortableWorkspaceDocument, PortableWorkspaceRequest};

#[test]
fn compiler_correction_retains_all_seven_fields_after_error_drop() {
    // The local corpus preserves the compiler/LSP's qualified rejected inputs.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/authoring/visibility-widening");
    let mut paths = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    let documents = paths
        .into_iter()
        .map(|path| {
            PortableWorkspaceDocument::new(
                path.file_name().unwrap().to_str().unwrap().to_owned(),
                std::fs::read(path).unwrap(),
            )
        })
        .collect();
    let error = PaleApi::new()
        .unwrap()
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".into(),
            documents,
        ))
        .unwrap_err();
    let [canonical] = error.authoring_corrections() else {
        panic!("one compiler correction")
    };
    let correction = AuthoringCorrection::from_api(canonical);
    let identity = error.diagnostic().identity().as_str().to_owned();
    drop(error);
    assert!(matches!(
        correction.kind(),
        AuthoringCorrectionKind::NarrowImportVisibility
    ));
    assert_eq!(correction.source_label(), "consumer-module.toml");
    let path = correction.path().unwrap();
    assert_eq!(
        path.iter()
            .map(|step| (step.field.as_deref(), step.index))
            .collect::<Vec<_>>(),
        [
            (Some("module"), None),
            (Some("imports"), None),
            (None, Some(0)),
            (Some("visibility_ceiling"), None)
        ]
    );
    assert_eq!(correction.expected_source(), "\"public\"");
    assert_eq!(correction.replacement_source(), "\"protected\"");
    assert!(matches!(
        correction.proof_subject().kind(),
        AuthoringIdentityKind::Export
    ));
    assert_eq!(correction.proof_subject().value(), "candidate::public-api");
    assert_eq!(correction.proof_subject().authored_value(), "public-api");
    assert_eq!(correction.diagnostic_identity(), identity);
}
