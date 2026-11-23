//! Real compiler-produced correction values through Python's retained getters.

use crate::{PyAuthoringCorrection, PyAuthoringCorrectionKind};
use pale_sdk::{PaleApi, PortableWorkspaceDocument, PortableWorkspaceRequest};

#[test]
fn compiler_correction_retains_all_seven_fields_after_error_drop() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../interfaces/lsp/tests/fixtures/authoring/visibility-widening");
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
    let correction = PyAuthoringCorrection::from_api(canonical);
    let identity = error.diagnostic().identity().as_str().to_owned();
    drop(error);
    assert!(matches!(
        correction.kind(),
        PyAuthoringCorrectionKind::NarrowImportVisibility
    ));
    assert_eq!(correction.source_label(), "consumer-module.toml");
    let path = correction.path();
    assert_eq!(
        path.iter()
            .map(|step| (step.kind(), step.field_value(), step.index_value()))
            .collect::<Vec<_>>(),
        [
            ("field", Some("module"), None),
            ("field", Some("imports"), None),
            ("index", None, Some(0)),
            ("field", Some("visibility_ceiling"), None)
        ]
    );
    assert_eq!(correction.expected_source(), "\"public\"");
    assert_eq!(correction.replacement_source(), "\"protected\"");
    assert_eq!(correction.proof_subject().kind().name(), "export");
    assert_eq!(correction.proof_subject().value(), "candidate::public-api");
    assert_eq!(correction.proof_subject().authored_value(), "public-api");
    assert_eq!(correction.diagnostic_identity(), identity);
}
