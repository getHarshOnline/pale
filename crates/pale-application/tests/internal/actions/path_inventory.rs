//! Path-inventory proofs for normalization without filesystem reads or error disclosure.

use super::*;

#[test]
fn supplied_paths_are_normalized_without_io_or_error_echo() {
    let output = execute(&PathInventoryInput {
        style: PathStyle::PosixRelative,
        paths: vec![
            "src/./lib.rs".into(),
            "src/bin/../lib.rs".into(),
            "README.md".into(),
        ],
    })
    .expect("valid supplied paths");

    assert_eq!(output.input_count, 3);
    assert_eq!(output.unique_count, 2);
    assert_eq!(output.entries[0].normalized_path, "README.md");
    assert_eq!(output.entries[1].normalized_path, "src/lib.rs");
    assert_eq!(output.entries[1].occurrences, 2);

    let sensitive = "../secret-project-name";
    let error = execute(&PathInventoryInput {
        style: PathStyle::PosixRelative,
        paths: vec![sensitive.into()],
    })
    .expect_err("escaping path");
    assert_eq!(error.code, ActionErrorCode::InvalidPath);
    assert!(!error.message.contains(sensitive));
    assert!(error.message.contains("index 0"));
}
