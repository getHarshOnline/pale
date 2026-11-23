//! An incomplete execution workspace remains inspectable but cannot render a host plan.

use super::*;

fn workspace() -> PortableWorkspaceRequest {
    let manifest =
        String::from_utf8(captured_fixture!("workspace/pale-workspace.toml").to_vec()).unwrap();
    let selection = "execution_path_ids = [\"direct-claude\"]";
    assert!(manifest.contains(selection));
    PortableWorkspaceRequest::new(
        "pale-workspace.toml".into(),
        vec![
            PortableWorkspaceDocument::new(
                "pale-workspace.toml".into(),
                manifest
                    .replace(selection, "execution_path_ids = []")
                    .into_bytes(),
            ),
            PortableWorkspaceDocument::new(
                "composition.toml".into(),
                captured_fixture!("workspace/composition.toml").to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "workspace-resolution.lock.toml".into(),
                captured_fixture!("workspace/workspace-resolution.lock.toml").to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "workspace-universe.lock.toml".into(),
                captured_fixture!("workspace/workspace-universe.lock.toml").to_vec(),
            ),
        ],
    )
}

#[test]
fn missing_execution_paths_report_the_configuration_fix_before_rendering() {
    let api = PaleApi::new().unwrap();
    let portable = api.compile_portable_workspace(workspace()).unwrap();
    api.compile_effective_profile(&portable, captured_effective_profile_request(false))
        .expect("an unfinished workspace remains available to inspection and recovery");
    let error = api
        .compile_one_shot(captured_request_with_workspace(false, workspace()))
        .expect_err("host plans require a selected execution path");
    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.code(), "one_shot_execution_paths_missing");
    assert_eq!(diagnostic.stage(), DiagnosticStage::SemanticAdmission);
    assert_eq!(
        diagnostic.disposition(),
        DiagnosticDisposition::InvalidInput
    );
    assert_eq!(diagnostic.retry(), DiagnosticRetry::AfterCorrection);
    assert_eq!(
        diagnostic.effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
    assert_eq!(
        diagnostic.subject(),
        Some(("execution_workspace", "default"))
    );
    assert!(diagnostic.canonical_json().contains("execution_path_ids"));
    assert!(error.to_string().contains("execution_path_ids"));
    assert!(!error.to_string().contains("application invariant"));
}
