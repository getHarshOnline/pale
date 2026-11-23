//! Real terminal carriage after the native source-capture boundary.

use super::*;

#[path = "../support/authoring_failure.rs"]
mod fixture;

fn files(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn visit(
        root: &Path,
        directory: &Path,
        output: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, output);
            } else {
                output.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut output = std::collections::BTreeMap::new();
    visit(root, root, &mut output);
    output
}

#[test]
fn local_compiler_errors_return_evidence_without_changing_source() {
    for contract_rejection in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let manifest = write_workspace(root.path());
        let manifest_text = std::fs::read_to_string(&manifest)
            .unwrap()
            .replace("workspace_id = \"syn-ws\"", "workspace_id = \"synthetic\"")
            .replace(
                "module_manifests = []",
                "module_manifests = [\"module.toml\", \"consumer-module.toml\"]",
            )
            .replace(
                "source_components = []",
                "source_components = [\"alpha.toml\", \"beta.toml\", \"consumer.toml\"]",
            );
        std::fs::write(&manifest, manifest_text).unwrap();
        for (label, mut content) in fixture::documents() {
            // Keep the native topology, machine and distribution selections.
            if label == "pale-workspace.toml" {
                continue;
            }
            let target = match label {
                "resolution.lock.toml" => "workspace-resolution.lock.toml",
                "universe.lock.toml" => "workspace-universe.lock.toml",
                _ => label,
            };
            if contract_rejection && label == "consumer-module.toml" {
                content = content.replace("schema_version = 1", "schema_version = 2");
            }
            std::fs::write(root.path().join(target), content).unwrap();
        }
        let before = files(root.path());
        let api = pale_sdk::PaleApi::new().unwrap();
        let error = api
            .compile_local_one_shot(pale_sdk::LocalOneShotCompilationRequest::new(
                pale_sdk::LocalWorkspaceLocator::new(manifest.to_str().unwrap().to_owned()),
                "demo".to_owned(),
                "default".to_owned(),
                "synthetic".to_owned(),
            ))
            .expect_err("native capture reaches the rejected compiler source");
        assert_eq!(
            error.authoring_corrections().len(),
            usize::from(!contract_rejection),
            "{}",
            String::from_utf8_lossy(error.diagnostic().canonical_bytes())
        );
        assert!(error.authoring_location().is_some());
        let expected = pale_cli::command::DiagnosticPresentation::from_api_error(&error)
            .to_json()
            .unwrap();
        for format in ["json", "human"] {
            let output = Command::new(bin())
                .args([
                    "--error-format",
                    format,
                    "--workspace",
                    manifest.to_str().unwrap(),
                    "--project",
                    "demo",
                    "--execution-workspace",
                    "default",
                    "--machine",
                    "synthetic",
                    "validate",
                ])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(4));
            assert!(output.stdout.is_empty());
            let text = String::from_utf8(output.stderr).unwrap();
            if format == "json" {
                assert_eq!(text, format!("{expected}\n"));
                let value: serde_json::Value = serde_json::from_str(&text).unwrap();
                assert_eq!(
                    value["diagnostic"],
                    serde_json::from_slice::<serde_json::Value>(
                        error.diagnostic().canonical_bytes()
                    )
                    .unwrap()
                );
                assert_eq!(
                    value["diagnosticIdentity"],
                    error.diagnostic().identity().as_str()
                );
                assert_eq!(
                    value["authoringCorrections"],
                    serde_json::to_value(error.authoring_corrections()).unwrap()
                );
                assert_eq!(
                    value["authoringLocation"],
                    serde_json::to_value(error.authoring_location()).unwrap()
                );
            } else {
                assert!(text.contains(error.diagnostic().code()));
                assert!(text.contains("Source location:"));
                assert_eq!(
                    text.contains("Suggested edits (not applied):"),
                    !contract_rejection
                );
                if !contract_rejection {
                    assert!(text.contains("\\\"public\\\""));
                    assert!(text.contains("\\\"protected\\\""));
                }
            }
            assert_eq!(files(root.path()), before);
        }
    }
}
