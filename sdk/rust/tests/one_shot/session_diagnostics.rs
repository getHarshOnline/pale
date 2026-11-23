//! Session configuration failures keep their distinct correction and safe context.

use super::*;
use std::error::Error;

fn request() -> OneShotCompilationRequest {
    let manifest =
        String::from_utf8(captured_fixture!("workspace/pale-workspace.toml").to_vec()).unwrap();
    let selection = "execution_path_ids = [\"direct-claude\"]";
    assert!(manifest.contains(selection));
    let manifest = manifest.replace(
        selection,
        &format!(
            "{selection}, session = {{ activation_roots = [\"engine\"], lifecycle_events = [] }}"
        ),
    );
    let composition: toml::Value = toml::from_str(
        std::str::from_utf8(captured_fixture!("workspace/composition.toml")).unwrap(),
    )
    .unwrap();
    let binding = &composition["declarations"]["provider_bindings"][0];
    let mut machine: toml::Value = toml::from_str(
        std::str::from_utf8(captured_fixture!("private/machine-binding.toml")).unwrap(),
    )
    .unwrap();
    let machine_table = machine.as_table_mut().unwrap();
    for (name, value) in [
        (
            "surface_executable",
            serde_json::json!({"exec": "/captured/pale", "digest": "a".repeat(64), "version": "1.0.0", "protocol": "pale.agent-surface-interface.v1"}),
        ),
        (
            "session_state",
            serde_json::json!({"root": "/captured/sessions", "ttl_seconds": 3600}),
        ),
        (
            "durable_evidence",
            serde_json::json!({"root": "/captured/evidence", "retention_seconds": 3600}),
        ),
        (
            "native_harnesses",
            serde_json::json!([{
                "provider_binding": {"provider_binding_id": binding["id"].as_str().unwrap(), "provider_binding_identity": {"provider_binding_sha256": binding["binding_identity"]["provider_binding_sha256"].as_str().unwrap()}},
                "exec": "/captured/claude", "digest": "b".repeat(64), "version": "1.0.0"
            }]),
        ),
    ] {
        machine_table.insert(name.into(), toml::Value::try_from(value).unwrap());
    }
    let workspace = PortableWorkspaceRequest::new(
        "pale-workspace.toml".into(),
        vec![
            PortableWorkspaceDocument::new("pale-workspace.toml".into(), manifest.into_bytes()),
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
    );
    OneShotCompilationRequest::new(
        "demo".into(),
        "default".into(),
        "machine".into(),
        OneShotPrivateInputs::new(
            document(
                "capabilities.toml",
                captured_fixture!("private/capabilities.toml"),
            ),
            CompilationDocument::new(
                "machines/machine.toml".into(),
                toml::to_string(&machine).unwrap().into_bytes(),
            ),
            document(
                "distribution-lock.toml",
                captured_fixture!("private/distribution-lock.toml"),
            ),
            Vec::new(),
        ),
        captured_package_documents(false),
        workspace,
    )
}

#[test]
fn missing_session_event_identifies_the_required_permission() {
    let error = PaleApi::new()
        .unwrap()
        .compile_one_shot(request())
        .unwrap_err();
    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.code(), "one_shot_lifecycle_conflict");
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
        Some(("execution_path", "direct-claude"))
    );
    assert!(error.to_string().contains("lifecycle_events"));
    assert!(!diagnostic.help().is_empty());
    for (key, value) in [
        ("field", "lifecycle_events"),
        ("required_event", "permission-request"),
        ("declared_events", "none"),
    ] {
        assert!(diagnostic
            .facts()
            .iter()
            .any(|fact| fact.key() == key && fact.value() == value));
    }
    assert!(error.source().is_none());
    for visible in [
        error.to_string(),
        format!("{error:?}"),
        diagnostic.canonical_json().to_owned(),
    ] {
        assert!(!visible.contains("/captured/"));
    }
}
