//! Native-local façade behavior over one source-backed workspace fixture.

#![cfg(feature = "native-local")]
use std::fs;
use std::path::Path;

#[path = "support/local_workspace.rs"]
mod local_fixture;
use local_fixture::{local_workspace, request, source_workspace, write};

use pale_sdk::{
    CompilationDocument, ConfigurationImprovementProposalRequest, ConfigurationImprovementSources,
    DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticRetry,
    EffectiveProfileCompilationRequest, LocalConfigurationImprovementRequest,
    LocalMaterializationAction, LocalMaterializationAdmissionRequest, LocalMaterializationDrift,
    LocalMaterializationMachineEffectKind, LocalMaterializationOwnershipChange,
    LocalMaterializationRunOutcome, LocalMaterializationTargetOutcome,
    LocalNativeActionInputRequest, LocalOneShotCompilationRequest, LocalOneShotPlanRequest,
    LocalPackageDistributionRequest, LocalWorkspaceLocator, OneShotPlanKind, OneShotPrivateInputs,
    PaleApi, PortableWorkspaceDocument, PortableWorkspaceRequest,
};

fn local_workspace_with_two_repositories() -> (tempfile::TempDir, String) {
    let (temporary, manifest) = local_workspace();
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary workspace path is canonical");
    let workspace_path = root.join("pale-workspace.toml");
    let workspace = fs::read_to_string(&workspace_path).expect("workspace manifest is readable");
    let workspace = workspace
        .replace(
            "repositories = [{ id = \"engine\", title = \"Engine\" }]",
            "repositories = [{ id = \"engine\", title = \"Engine\" }, { id = \"mirror\", title = \"Mirror\" }]",
        )
        .replace(
            "project_repositories = [{ project_id = \"demo\", repository_id = \"engine\", role = { package_id = \"example.engine\", resource_id = \"engine\" }, stacks = [], families = [], materialization_order = 0 }]",
            "project_repositories = [{ project_id = \"demo\", repository_id = \"engine\", role = { package_id = \"example.engine\", resource_id = \"engine\" }, stacks = [], families = [], materialization_order = 0 }, { project_id = \"demo\", repository_id = \"mirror\", role = { package_id = \"example.engine\", resource_id = \"engine\" }, stacks = [], families = [], materialization_order = 1 }]",
        )
        .replace(
            "execution_workspace_repositories = [{ execution_workspace_id = \"default\", repository_id = \"engine\", access = \"mutate\" }]",
            "execution_workspace_repositories = [{ execution_workspace_id = \"default\", repository_id = \"engine\", access = \"mutate\" }, { execution_workspace_id = \"default\", repository_id = \"mirror\", access = \"mutate\" }]",
        );
    fs::write(workspace_path, workspace).expect("two-repository manifest is written");
    let mirror = root.join("mirror");
    fs::create_dir(&mirror).expect("second checkout is created");
    let machine_path = root.join("machines/machine.toml");
    let mut machine = fs::read_to_string(&machine_path).expect("machine binding is readable");
    machine.push_str(&format!(
        "\n[[repository_checkouts]]\nrepository_id = \"mirror\"\ncheckout_root = {:?}\n",
        mirror.display().to_string(),
    ));
    fs::write(machine_path, machine).expect("second checkout binding is written");
    (temporary, manifest)
}

fn package_request(manifest: String) -> LocalPackageDistributionRequest {
    LocalPackageDistributionRequest::new(
        LocalWorkspaceLocator::new(manifest),
        "demo".to_owned(),
        "default".to_owned(),
        "machine".to_owned(),
    )
}

fn improvement_request(
    baseline_manifest: String,
    candidate_manifest: String,
) -> LocalConfigurationImprovementRequest {
    LocalConfigurationImprovementRequest::new(
        LocalWorkspaceLocator::new(baseline_manifest),
        LocalWorkspaceLocator::new(candidate_manifest),
        "demo".to_owned(),
        "default".to_owned(),
        "machine".to_owned(),
    )
}

fn source_document(root: &Path, label: &str) -> CompilationDocument {
    CompilationDocument::new(
        label.to_owned(),
        fs::read(root.join(label)).expect("direct source member is readable"),
    )
}

fn workspace_request(root: &Path) -> PortableWorkspaceRequest {
    PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_owned(),
        [
            "composition.toml",
            "pale-workspace.toml",
            "workspace-resolution.lock.toml",
            "workspace-universe.lock.toml",
        ]
        .into_iter()
        .map(|label| {
            PortableWorkspaceDocument::new(
                label.to_owned(),
                fs::read(root.join(label)).expect("direct workspace member is readable"),
            )
        })
        .collect(),
    )
}

fn private_inputs(root: &Path) -> OneShotPrivateInputs {
    OneShotPrivateInputs::new(
        source_document(root, "capabilities.toml"),
        source_document(root, "machines/machine.toml"),
        source_document(root, "distribution-lock.toml"),
        Vec::new(),
    )
}

fn direct_improvement_sources(
    baseline_root: &Path,
    candidate_root: &Path,
) -> ConfigurationImprovementSources {
    ConfigurationImprovementSources::new(
        workspace_request(baseline_root),
        EffectiveProfileCompilationRequest::new(
            "demo".to_owned(),
            "default".to_owned(),
            "machine".to_owned(),
            private_inputs(baseline_root),
            ["engine/package.toml", "engine/role.md", "engine/roles.toml"]
                .into_iter()
                .map(|label| source_document(&baseline_root.join("distribution/packages"), label))
                .collect(),
        ),
        ConfigurationImprovementProposalRequest::new(
            "demo".to_owned(),
            "default".to_owned(),
            "machine".to_owned(),
            workspace_request(candidate_root),
            private_inputs(candidate_root),
        ),
    )
}

fn bind_current_executable(root: &Path) {
    let executable = std::env::current_exe()
        .expect("test executable is available")
        .canonicalize()
        .expect("test executable is canonical");
    let digest = pale_core::records::ExecutableDigest::hash_bytes(
        &fs::read(&executable).expect("test executable bytes are readable"),
    );
    let machine_path = root.join("machines/machine.toml");
    let mut machine = fs::read_to_string(&machine_path).expect("machine binding is readable");
    machine.push_str(&format!(
        "\n[surface_executable]\nexec = {:?}\ndigest = {:?}\nversion = {:?}\nprotocol = {:?}\n",
        executable.display().to_string(),
        digest.to_string(),
        env!("CARGO_PKG_VERSION"),
        pale_core::SURFACE_INTERFACE_PROTOCOL,
    ));
    fs::write(machine_path, machine).expect("surface executable binding is appended");
}

#[test]
fn local_capture_reuses_the_path_free_one_shot_compiler() {
    let (_temporary, manifest) = local_workspace();
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let local = api
        .compile_local_one_shot(request(manifest.clone()))
        .expect("source-backed fixture compiles through the canonical capture");

    assert_eq!(local.summary().project_id(), "demo");
    assert_eq!(local.summary().execution_workspace_id(), "default");
    assert_eq!(local.summary().repository_count(), 1);
    assert_eq!(
        local.compilation().summary().one_shot_compilation_digest(),
        local.summary().one_shot_compilation_digest()
    );
    assert!(!format!("{local:?}").contains(&manifest));

    let error = api
        .verify_current_executable_binding(&local)
        .expect_err("the fixture deliberately binds no surface executable");
    assert_eq!(
        error.diagnostic().code(),
        "current_executable_binding_verification_failed"
    );
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::Blocked
    );
    assert_eq!(
        error.diagnostic().retry(),
        DiagnosticRetry::AfterStateChange
    );
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
}

#[path = "local/source_guidance.rs"]
mod source_guidance;

#[test]
fn local_source_guidance_stops_before_runtime_rendering_and_target_writes() {
    let (temporary, manifest) = source_workspace();
    let root = temporary.path().canonicalize().unwrap();
    let api = PaleApi::new().unwrap();
    let profile = api
        .compile_local_effective_profile(request(manifest.clone()))
        .unwrap();
    for host in [
        pale_sdk::SourceGuidanceHost::Codex,
        pale_sdk::SourceGuidanceHost::ClaudeCode,
    ] {
        let guidance = profile.source_guidance("engine", host).unwrap();
        assert!(guidance
            .files()
            .iter()
            .any(|file| file.content().contains("Exact retained role body.")));
        assert!(!root
            .join("checkout")
            .join(guidance.entrypoint_path())
            .exists());
        assert!(guidance
            .files()
            .iter()
            .all(|file| !root.join("checkout").join(file.rel_path()).exists()));
    }
    assert!(api.compile_local_one_shot(request(manifest)).is_err());
    drop(temporary);
    assert!(profile
        .source_guidance("engine", pale_sdk::SourceGuidanceHost::Codex)
        .is_ok());
}

#[test]
fn local_configuration_review_matches_the_path_free_source_preview() {
    let (baseline, baseline_manifest) = local_workspace();
    let (candidate, candidate_manifest) = local_workspace();
    let baseline_root = baseline
        .path()
        .canonicalize()
        .expect("baseline workspace is canonical");
    let candidate_root = candidate
        .path()
        .canonicalize()
        .expect("candidate workspace is canonical");
    fs::remove_dir_all(candidate_root.join("distribution/packages"))
        .expect("candidate Package source is deliberately unavailable");
    let api = PaleApi::new().expect("embedded API dependencies are coherent");

    let captured = api
        .capture_local_configuration_improvement_sources(improvement_request(
            baseline_manifest.clone(),
            candidate_manifest.clone(),
        ))
        .expect("both local source roots are captured without effect");
    let local = api
        .preview_configuration_improvement_sources(captured)
        .expect("captured local sources use the canonical preview path");
    let direct = api
        .preview_configuration_improvement_sources(direct_improvement_sources(
            &baseline_root,
            &candidate_root,
        ))
        .expect("the same path-free sources produce a preview");

    assert_eq!(
        local.summary().canonical_bytes(),
        direct.summary().canonical_bytes()
    );
    assert_eq!(
        local.comparison().summary().canonical_bytes(),
        direct.comparison().summary().canonical_bytes()
    );
    let composed = api
        .preview_local_configuration_improvement(improvement_request(
            baseline_manifest,
            candidate_manifest,
        ))
        .expect("the native entry reuses capture and the canonical preview");
    assert_eq!(
        composed.summary().canonical_bytes(),
        local.summary().canonical_bytes()
    );
    assert_eq!(
        composed.comparison().summary().canonical_bytes(),
        local.comparison().summary().canonical_bytes()
    );
}

#[test]
fn local_configuration_review_request_debug_redacts_paths_and_selectors() {
    let request = LocalConfigurationImprovementRequest::new(
        LocalWorkspaceLocator::new("/private/baseline-marker/pale-workspace.toml".to_owned()),
        LocalWorkspaceLocator::new("/private/candidate-marker/pale-workspace.toml".to_owned()),
        "private-project-marker".to_owned(),
        "private-execution-marker".to_owned(),
        "private-machine-marker".to_owned(),
    );
    let debug = format!("{request:?}");

    for private in [
        "baseline-marker",
        "candidate-marker",
        "private-project-marker",
        "private-execution-marker",
        "private-machine-marker",
    ] {
        assert!(!debug.contains(private));
    }
    assert!(debug.contains("baseline_workspace"));
    assert!(debug.contains("candidate_workspace"));
    assert!(debug.contains("project_id_byte_len"));
}

#[test]
fn local_configuration_review_rejects_bad_paths_and_selectors_through_one_boundary() {
    let (_baseline, baseline_manifest) = local_workspace();
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let relative_candidate = "private/candidate/pale-workspace.toml";
    let path_error = api
        .capture_local_configuration_improvement_sources(improvement_request(
            baseline_manifest.clone(),
            relative_candidate.to_owned(),
        ))
        .expect_err("a relative candidate locator cannot select filesystem authority");
    assert_eq!(path_error.diagnostic().code(), "local_workspace_rejected");
    assert_eq!(
        path_error.diagnostic().disposition(),
        DiagnosticDisposition::InvalidInput
    );
    assert!(!format!("{path_error:?}").contains(relative_candidate));
    let preview_error = api
        .preview_local_configuration_improvement(improvement_request(
            baseline_manifest.clone(),
            relative_candidate.to_owned(),
        ))
        .err()
        .expect("the composed preview preserves local capture rejection");
    assert_eq!(
        preview_error.diagnostic().canonical_bytes(),
        path_error.diagnostic().canonical_bytes()
    );

    let (_candidate, candidate_manifest) = local_workspace();
    let selector = "private-project-selector";
    let selector_error = api
        .capture_local_configuration_improvement_sources(LocalConfigurationImprovementRequest::new(
            LocalWorkspaceLocator::new(baseline_manifest),
            LocalWorkspaceLocator::new(candidate_manifest),
            selector.to_owned(),
            "default".to_owned(),
            "machine".to_owned(),
        ))
        .expect_err("an unknown common selector cannot start a local review");
    assert_eq!(
        selector_error.diagnostic().code(),
        "local_workspace_rejected"
    );
    assert_eq!(
        selector_error.diagnostic().disposition(),
        DiagnosticDisposition::InvalidInput
    );
    assert!(!format!("{selector_error:?}").contains(selector));
}

#[test]
fn relative_locator_is_rejected_without_disclosing_it() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let locator = "private/workspace/pale-workspace.toml";
    let error = api
        .compile_local_one_shot(request(locator.to_owned()))
        .expect_err("a relative host locator cannot select filesystem authority");

    assert_eq!(error.diagnostic().code(), "local_workspace_rejected");
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::InvalidInput
    );
    assert_eq!(error.diagnostic().retry(), DiagnosticRetry::AfterCorrection);
    assert!(!format!("{error:?}").contains(locator));
}

#[test]
fn compiled_interface_identity_is_the_admitted_initial_v1_record() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let identity = api
        .compiled_interface_identity()
        .expect("compiled interface identity is coherent");
    let value = pale_core::canon::parse_strict(identity.canonical_bytes())
        .expect("executable identity is strict canonical JSON");

    pale_contracts::validate_value(&value, "surface-executable-identity.v1")
        .expect("executable identity satisfies its embedded contract");
    assert_eq!(identity.version(), "0.0.1");
    assert_eq!(identity.protocol(), pale_core::SURFACE_INTERFACE_PROTOCOL);
    assert_eq!(value["version"], identity.version());
    assert_eq!(value["protocol"], identity.protocol());
    assert_eq!(value["os"], identity.os());
    assert_eq!(value["arch"], identity.arch());

    let (temporary, manifest) = local_workspace();
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary workspace path is canonical");
    bind_current_executable(&root);
    let compilation = api
        .compile_local_one_shot(request(manifest))
        .expect("current executable binding is captured");
    let verified = api
        .verify_current_executable_binding(&compilation)
        .expect("current executable satisfies the admitted binding");
    assert_eq!(verified, identity);
}

#[test]
fn local_package_inspection_captures_only_the_selected_distribution() {
    let (_temporary, manifest) = local_workspace();
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let inspection = api
        .inspect_local_package_distribution(package_request(manifest))
        .expect("the selected local Package tree is inspected through its canonical owner");

    assert_eq!(inspection.packages().len(), 1);
    assert_eq!(inspection.packages()[0].id(), "example.engine");
    assert!(!inspection.canonical_bytes().is_empty());
}

#[test]
fn confined_action_capture_returns_path_free_bytes_and_rejects_escape() {
    let temporary = tempfile::tempdir().expect("temporary action root is created");
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary action root is canonical");
    let body = br#"{"kind":"path_inventory","payload":{"style":"posix_relative","paths":["src/./lib.rs"]}}"#;
    write(&root, "input/action.json", body);
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let request = api
        .capture_local_native_action_request(LocalNativeActionInputRequest::new(
            "pale.compute.path-inventory".to_owned(),
            root.to_string_lossy().into_owned(),
            "input/action.json".to_owned(),
        ))
        .expect("the confined source is captured into an owned request");
    let execution = api
        .invoke_native_action(request)
        .expect("the canonical registry executes the captured request");
    assert_eq!(execution.action_id(), "pale.compute.path-inventory");

    let escaped = "../outside.json";
    let error = api
        .capture_local_native_action_request(LocalNativeActionInputRequest::new(
            "pale.compute.path-inventory".to_owned(),
            root.to_string_lossy().into_owned(),
            escaped.to_owned(),
        ))
        .expect_err("parent traversal cannot escape the retained root");
    assert_eq!(error.diagnostic().code(), "local_action_input_rejected");
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::InvalidInput
    );
    assert!(!format!("{error:?}").contains(escaped));
}

#[test]
fn confined_action_capture_leaves_payload_parsing_to_the_action_owner() {
    let temporary = tempfile::tempdir().expect("temporary action root is created");
    let root = temporary.path().canonicalize().unwrap();
    let body = b"{ malformed action JSON";
    let action = "pale.compute.path-inventory";
    write(&root, "input/malformed.json", body);
    let api = PaleApi::new().unwrap();
    let captured = api
        .capture_local_native_action_request(LocalNativeActionInputRequest::new(
            action.to_owned(),
            root.to_string_lossy().into_owned(),
            "input/malformed.json".to_owned(),
        ))
        .expect("a confined bounded file is captured without parsing its action payload");
    let captured_error = api
        .invoke_native_action(captured)
        .expect_err("the action owner rejects malformed JSON after capture");
    let direct_error = api
        .invoke_native_action(pale_sdk::NativeActionRequest::new(
            action.to_owned(),
            body.to_vec(),
        ))
        .expect_err("direct invocation reaches the same canonical parser");
    assert_eq!(
        captured_error.diagnostic().canonical_bytes(),
        direct_error.diagnostic().canonical_bytes()
    );
    assert_eq!(
        captured_error.diagnostic().disposition(),
        DiagnosticDisposition::InvalidInput
    );
    assert_eq!(fs::read(root.join("input/malformed.json")).unwrap(), body);
}

#[test]
fn local_status_and_plans_preserve_apply_adopt_and_remove_laws() {
    let (_temporary, manifest) = local_workspace();
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_local_one_shot(request(manifest))
        .expect("the local fixture compiles before materialization observation");

    let status = api
        .inspect_local_repository_status(&compilation, "engine".to_owned())
        .expect("unowned local status is compared without mutation");
    assert!(!status.is_clean());
    assert!(status.drift().iter().any(|reason| matches!(
        reason,
        LocalMaterializationDrift::Ownership {
            change: LocalMaterializationOwnershipChange::Initialize
        }
    )));
    assert!(status
        .drift()
        .iter()
        .any(|reason| matches!(reason, LocalMaterializationDrift::Surface { .. })));

    let apply = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Apply),
        )
        .expect("ordinary apply plans absent desired targets as creates");
    let repeated = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Apply),
        )
        .expect("the same observations reproduce the same plan");
    assert_eq!(
        apply.document().canonical_bytes(),
        repeated.document().canonical_bytes()
    );
    assert!(apply.counts().create() > 0);
    assert_eq!(apply.counts().adopt(), 0);
    assert_eq!(apply.counts().machine_local_effects(), 0);
    assert_eq!(
        apply.counts().total_operations(),
        apply.counts().operations()
    );

    let adoption = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Adopt),
        )
        .expect("explicit adoption still creates targets proved absent");
    assert_eq!(adoption.document().kind(), OneShotPlanKind::Adopt);
    assert!(adoption.counts().create() > 0);
    assert_eq!(adoption.counts().adopt(), 0);

    let removal = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Remove),
        )
        .expect("removal reconstructs the explicit unowned transition");
    assert_eq!(removal.document().kind(), OneShotPlanKind::Remove);
    assert_eq!(removal.counts().delete(), 0);
}

#[test]
fn workspace_status_orders_repositories_and_encodes_each_drift_reason() {
    let (temporary, manifest) = local_workspace_with_two_repositories();
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_local_one_shot(request(manifest))
        .expect("the two-repository fixture compiles");

    let status = api
        .inspect_local_workspace_status(&compilation)
        .expect("every selected repository is compared");
    let repeated = api
        .inspect_local_workspace_status(&compilation)
        .expect("the same observations reproduce the same result");

    assert!(!status.is_clean());
    assert_eq!(
        status
            .repositories()
            .iter()
            .map(|repository| repository.repository_id())
            .collect::<Vec<_>>(),
        vec!["engine", "mirror"]
    );
    assert!(status
        .repositories()
        .iter()
        .all(|repository| !repository.is_clean()));
    assert_eq!(status.canonical_bytes(), repeated.canonical_bytes());
    let value: serde_json::Value = serde_json::from_slice(status.canonical_bytes()).unwrap();
    assert_eq!(value["clean"], false);
    assert_eq!(value["repositories"][0]["repositoryId"], "engine");
    assert_eq!(value["repositories"][1]["repositoryId"], "mirror");
    for repository in value["repositories"].as_array().unwrap() {
        for drift in repository["drift"].as_array().unwrap() {
            let keys = drift
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(keys, vec!["action", "kind", "path"]);
        }
    }
    let rows = value["repositories"][0]["drift"].as_array().unwrap();
    assert!(rows.iter().any(|row| {
        row["kind"] == "ownership"
            && row["path"] == ".pale/materialization-state.json"
            && row["action"] == "initialize"
    }));
    assert!(!String::from_utf8_lossy(status.canonical_bytes())
        .contains(&temporary.path().display().to_string()));
}

#[test]
fn workspace_status_encodes_one_clean_aggregate_after_apply() {
    let (temporary, manifest) = local_workspace();
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary workspace path is canonical");
    bind_current_executable(&root);
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_local_one_shot(request(manifest))
        .expect("the executable-bound fixture compiles");
    let plans = root.join("plans");
    let evidence = root.join("receipts");
    let plan = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Apply),
        )
        .expect("apply is planned");
    api.publish_local_one_shot_plan(&plan, plans.display().to_string())
        .expect("the apply plan is published");
    let mut admitted = api
        .admit_local_materialization_run(
            &compilation,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Apply,
                plans.display().to_string(),
                evidence.display().to_string(),
            ),
        )
        .expect("the apply plan is admitted");
    api.settle_local_materialization_run(&mut admitted)
        .expect("apply establishes the desired local state");

    let status = api
        .inspect_local_workspace_status(&compilation)
        .expect("the settled workspace is compared");

    assert!(status.is_clean());
    assert_eq!(status.repositories().len(), 1);
    assert_eq!(status.repositories()[0].repository_id(), "engine");
    assert!(status.repositories()[0].drift().is_empty());
    assert_eq!(
        status.canonical_bytes(),
        br#"{"clean":true,"repositories":[{"drift":[],"repositoryId":"engine"}]}"#
    );
}

#[test]
fn public_plan_apply_and_remove_journey_preserves_evidence_and_git_local_bytes() {
    let (temporary, manifest) = local_workspace();
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary workspace path is canonical");
    bind_current_executable(&root);
    let checkout = root.join("checkout");
    let exclude = checkout.join(".git/info/exclude");
    fs::create_dir_all(exclude.parent().expect("exclude has a parent"))
        .expect("ordinary Git metadata is created");
    let user_exclude = b"# user-owned exclusion\nlocal.cache\n";
    fs::write(&exclude, user_exclude).expect("user-owned exclude bytes are written");

    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_local_one_shot(request(manifest))
        .expect("executable-bound fixture compiles");
    let plans = root.join("plans");
    let evidence = root.join("receipts");
    let apply = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Apply),
        )
        .expect("apply is planned from trusted observations");
    assert_eq!(apply.counts().machine_local_effects(), 1);
    let publication = api
        .publish_local_one_shot_plan(&apply, plans.display().to_string())
        .expect("complete apply plan is published");
    assert_eq!(publication.repository_id(), "engine");
    assert_eq!(publication.relative_target(), "engine.apply.json");
    let saved_apply = fs::read(plans.join(publication.relative_target()))
        .expect("published apply plan is readable");
    let repeated_publication = api
        .publish_local_one_shot_plan(&apply, plans.display().to_string())
        .expect("publishing the exact plan again is idempotent");
    assert_eq!(repeated_publication, publication);
    assert_eq!(
        fs::read(plans.join(publication.relative_target()))
            .expect("idempotently published apply plan remains readable"),
        saved_apply
    );
    assert_eq!(
        pale_core::canon::parse_strict(&saved_apply).unwrap(),
        pale_core::canon::parse_strict(apply.document().canonical_bytes()).unwrap()
    );
    assert!(saved_apply.windows(3).any(|window| window == b"\n  "));
    assert!(saved_apply.ends_with(b"\n"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(plans.join(publication.relative_target()))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );
    }

    let mut admitted = api
        .admit_local_materialization_run(
            &compilation,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Apply,
                plans.display().to_string(),
                evidence.display().to_string(),
            ),
        )
        .expect("saved apply plan and target effects are admitted read-only");
    assert!(
        !evidence.exists(),
        "admission does not create evidence state"
    );
    assert!(!checkout.join(".pale").exists());
    let apply_result = api
        .settle_local_materialization_run(&mut admitted)
        .expect("apply settles with repository-scoped evidence");
    assert_eq!(
        apply_result.outcome(),
        LocalMaterializationRunOutcome::Succeeded
    );
    assert_eq!(apply_result.targets().len(), 1);
    let target = &apply_result.targets()[0];
    assert_eq!(
        target.outcome(),
        LocalMaterializationTargetOutcome::Succeeded
    );
    assert!(target.changed_surface_count() > 0);
    assert!(target.local_git_exclude_changed());
    assert!(target.failure().is_none());
    let run_file =
        fs::read(evidence.join("run-result.apply.json")).expect("apply run result is published");
    assert!(run_file.windows(3).any(|window| window == b"\n  "));
    assert_eq!(
        pale_core::canon::parse_strict(&run_file).unwrap(),
        pale_core::canon::parse_strict(apply_result.canonical_bytes()).unwrap()
    );
    let receipt = fs::read_dir(&evidence)
        .expect("evidence directory is readable")
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .ends_with(".receipt.json")
        })
        .expect("an immutable apply receipt is present");
    let receipt_bytes = fs::read(receipt.path()).expect("apply receipt is readable");
    assert!(receipt_bytes.windows(3).any(|window| window == b"\n  "));
    assert!(fs::read(&exclude)
        .expect("managed exclude is readable")
        .starts_with(user_exclude));

    let pending = evidence.join(".pending-native-run-result.json");
    let pending_bytes = b"foreign pending record\n";
    fs::write(&pending, pending_bytes).expect("foreign pending record is written");
    let remove = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Remove),
        )
        .expect("owned removal is reconstructed without a native harness");
    api.publish_local_one_shot_plan(&remove, plans.display().to_string())
        .expect("complete remove plan is published");
    let mut removal = api
        .admit_local_materialization_run(
            &compilation,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Remove,
                plans.display().to_string(),
                evidence.display().to_string(),
            ),
        )
        .expect("saved removal is re-admitted from current owned state");
    let remove_result = api
        .settle_local_materialization_run(&mut removal)
        .expect("remove settles independently");
    assert_eq!(
        remove_result.outcome(),
        LocalMaterializationRunOutcome::Succeeded
    );
    assert!(remove_result.targets()[0].changed_surface_count() > 0);
    assert_eq!(
        fs::read(&exclude).expect("user exclude bytes survive managed removal"),
        user_exclude
    );
    assert_eq!(
        fs::read(&pending).expect("foreign pending record remains untouched"),
        pending_bytes
    );
    assert!(!checkout.join(".pale/materialization-state.json").exists());
}

#[test]
fn runtime_is_reverified_before_evidence_or_repository_effects() {
    let (temporary, manifest) = local_workspace();
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary workspace path is canonical");
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_local_one_shot(request(manifest))
        .expect("compilation-only fixture compiles");
    let plan = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Apply),
        )
        .expect("apply plan is reconstructed");
    let plans = root.join("plans");
    let evidence = root.join("receipts");
    api.publish_local_one_shot_plan(&plan, plans.display().to_string())
        .expect("saved plan is published before effect admission");
    let mut admitted = api
        .admit_local_materialization_run(
            &compilation,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Apply,
                plans.display().to_string(),
                evidence.display().to_string(),
            ),
        )
        .expect("saved plan admission is effect-free and does not require runtime execution");
    let error = api
        .settle_local_materialization_run(&mut admitted)
        .expect_err("a missing surface executable is rejected at the final effect boundary");
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::Blocked
    );
    assert_eq!(
        error.diagnostic().retry(),
        DiagnosticRetry::AfterStateChange
    );
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotStarted
    );
    assert!(!evidence.exists());
    assert!(!root.join("checkout/.pale").exists());
}

#[test]
fn saved_plan_admission_reports_invalid_input_without_an_effect_boundary() {
    let (temporary, manifest) = local_workspace();
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary workspace path is canonical");
    let plans = root.join("plans");
    fs::create_dir(&plans).expect("empty saved-plan directory is created");
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_local_one_shot(request(manifest))
        .expect("fixture compiles");
    let error = api
        .admit_local_materialization_run(
            &compilation,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Apply,
                plans.display().to_string(),
                root.join("receipts").display().to_string(),
            ),
        )
        .expect_err("missing saved plan is invalid input");
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::InvalidInput
    );
    assert_eq!(error.diagnostic().retry(), DiagnosticRetry::AfterCorrection);
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
    assert!(!root.join("receipts").exists());
}

#[test]
fn immutable_receipt_collision_restores_the_repository_and_returns_failed_no_effect() {
    let (temporary, manifest) = local_workspace();
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary workspace path is canonical");
    bind_current_executable(&root);
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_local_one_shot(request(manifest))
        .expect("executable-bound fixture compiles");
    let plan = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Apply),
        )
        .expect("apply plan is reconstructed");
    let plans = root.join("plans");
    let evidence = root.join("receipts");
    api.publish_local_one_shot_plan(&plan, plans.display().to_string())
        .expect("saved plan is published");
    let mut admitted = api
        .admit_local_materialization_run(
            &compilation,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Apply,
                plans.display().to_string(),
                evidence.display().to_string(),
            ),
        )
        .expect("saved plan is admitted before evidence collision");
    let before = api
        .inspect_local_repository_status(&compilation, "engine".to_owned())
        .expect("pre-collision status is observable");
    fs::create_dir(&evidence).expect("evidence directory is created by the fixture");
    let receipt_name = format!(
        "engine.apply.{}.receipt.json",
        admitted.targets()[0].plan_digest()
    );
    let foreign_receipt = b"foreign immutable evidence\n";
    fs::write(evidence.join(&receipt_name), foreign_receipt)
        .expect("colliding immutable evidence is written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            evidence.join(&receipt_name),
            fs::Permissions::from_mode(0o600),
        )
        .expect("colliding receipt has the admitted mode");
    }

    let result = api
        .settle_local_materialization_run(&mut admitted)
        .expect("receipt collision is a settled failed-no-effect result");
    assert_eq!(
        result.outcome(),
        LocalMaterializationRunOutcome::FailedNoEffect
    );
    assert_eq!(result.succeeded_target_count(), 0);
    assert_eq!(result.targets().len(), 1);
    let target = &result.targets()[0];
    assert_eq!(
        target.outcome(),
        LocalMaterializationTargetOutcome::FailedNoEffect
    );
    assert_eq!(target.changed_surface_count(), 0);
    let diagnostic = target
        .failure()
        .expect("failed target retains a diagnostic");
    assert_eq!(diagnostic.effect(), DiagnosticEffectCertainty::Confirmed);
    assert_eq!(
        fs::read(evidence.join(receipt_name)).expect("foreign receipt remains untouched"),
        foreign_receipt
    );
    let after = api
        .inspect_local_repository_status(&compilation, "engine".to_owned())
        .expect("restored status is observable");
    assert_eq!(after, before);
    assert!(!root
        .join("checkout/.pale/materialization-state.json")
        .exists());
}

#[test]
fn evidence_capacity_is_admitted_under_lock_before_the_first_target_write() {
    let (temporary, manifest) = local_workspace();
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary workspace path is canonical");
    bind_current_executable(&root);
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_local_one_shot(request(manifest))
        .expect("executable-bound fixture compiles");
    let plan = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Apply),
        )
        .expect("apply plan is reconstructed");
    let plans = root.join("plans");
    let evidence = root.join("receipts");
    api.publish_local_one_shot_plan(&plan, plans.display().to_string())
        .expect("saved plan is published");
    let mut admitted = api
        .admit_local_materialization_run(
            &compilation,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Apply,
                plans.display().to_string(),
                evidence.display().to_string(),
            ),
        )
        .expect("target admission is read-only");
    fs::create_dir(&evidence).expect("evidence fixture directory is created");
    for index in 0..4_095 {
        fs::write(evidence.join(format!("foreign-{index:04}")), [])
            .expect("capacity entry is written");
    }

    let error = api
        .settle_local_materialization_run(&mut admitted)
        .expect_err("insufficient evidence capacity rejects the run before target writes");
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::Blocked
    );
    assert_eq!(
        error.diagnostic().retry(),
        DiagnosticRetry::AfterStateChange
    );
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::Confirmed
    );
    assert_eq!(
        error.diagnostic().subject(),
        Some(("operation", "local_materialization_evidence_preparation"))
    );
    let facts = error
        .diagnostic()
        .facts()
        .into_iter()
        .map(|fact| (fact.key().to_owned(), fact.value().to_owned()))
        .collect::<Vec<_>>();
    assert_eq!(
        facts,
        vec![
            ("effect_scope".to_owned(), "evidence_preparation".to_owned()),
            (
                "evidence_auxiliary_effect".to_owned(),
                "confirmed".to_owned(),
            ),
            ("repository_effect".to_owned(), "not_started".to_owned()),
        ]
    );
    assert!(!root.join("checkout/.pale").exists());
    assert!(evidence.join(".pale-receipts.lock").exists());
}

#[test]
fn explicit_adoption_settles_matching_unowned_surfaces_only_when_requested() {
    let (temporary, manifest) = local_workspace();
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary workspace path is canonical");
    bind_current_executable(&root);
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_local_one_shot(request(manifest))
        .expect("executable-bound fixture compiles");
    let plans = root.join("plans");
    let first_evidence = root.join("first-receipts");
    let apply = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Apply),
        )
        .expect("initial apply is planned");
    api.publish_local_one_shot_plan(&apply, plans.display().to_string())
        .expect("initial apply plan is published");
    let mut admitted_apply = api
        .admit_local_materialization_run(
            &compilation,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Apply,
                plans.display().to_string(),
                first_evidence.display().to_string(),
            ),
        )
        .expect("initial apply is admitted");
    api.settle_local_materialization_run(&mut admitted_apply)
        .expect("initial apply establishes exact generated bytes");

    fs::remove_file(root.join("checkout/.pale/materialization-state.json"))
        .expect("fixture removes only repository-local ownership evidence");
    fs::remove_dir_all(root.join("lineage"))
        .expect("fixture removes the independent ownership lineage");
    fs::create_dir(root.join("lineage")).expect("empty lineage root is recreated");
    let ordinary = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Apply),
        )
        .expect("ordinary apply can describe matching unowned bytes without adopting them");
    assert_eq!(ordinary.counts().adopt(), 0);
    let adopt = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Adopt),
        )
        .expect("explicit adoption accepts exact matching unowned surfaces");
    assert!(adopt.counts().adopt() > 0);
    api.publish_local_one_shot_plan(&adopt, plans.display().to_string())
        .expect("adoption uses the stored apply plan identity");
    let adoption_evidence = root.join("adoption-receipts");
    let mut admitted_adoption = api
        .admit_local_materialization_run(
            &compilation,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Adopt,
                plans.display().to_string(),
                adoption_evidence.display().to_string(),
            ),
        )
        .expect("explicit adoption is admitted from the same current observations");
    let result = api
        .settle_local_materialization_run(&mut admitted_adoption)
        .expect("explicit adoption settles with immutable evidence");
    assert_eq!(result.outcome(), LocalMaterializationRunOutcome::Succeeded);
    assert!(result.targets()[0].changed_surface_count() > 0);
}

#[test]
fn multi_repository_run_reports_a_settled_prefix_without_claiming_workspace_atomicity() {
    let (temporary, manifest) = local_workspace_with_two_repositories();
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary workspace path is canonical");
    bind_current_executable(&root);
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_local_one_shot(request(manifest))
        .expect("two-repository fixture compiles");
    assert_eq!(compilation.summary().repository_count(), 2);
    let plans = root.join("plans");
    for repository in compilation.summary().repositories() {
        let plan = api
            .plan_local_one_shot(
                &compilation,
                LocalOneShotPlanRequest::new(
                    repository.repository_id().to_owned(),
                    OneShotPlanKind::Apply,
                ),
            )
            .expect("repository apply plan is reconstructed");
        api.publish_local_one_shot_plan(&plan, plans.display().to_string())
            .expect("repository apply plan is published");
    }
    let evidence = root.join("receipts");
    let mut admitted = api
        .admit_local_materialization_run(
            &compilation,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Apply,
                plans.display().to_string(),
                evidence.display().to_string(),
            ),
        )
        .expect("all repository targets are admitted before settlement");
    let second = admitted
        .targets()
        .iter()
        .find(|target| target.repository_id() == "mirror")
        .expect("mirror is the second materialization target");
    fs::create_dir(&evidence).expect("evidence fixture directory is created");
    let collision = evidence.join(format!(
        "mirror.apply.{}.receipt.json",
        second.plan_digest()
    ));
    fs::write(&collision, b"foreign immutable evidence\n")
        .expect("second target receipt collision is written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&collision, fs::Permissions::from_mode(0o600))
            .expect("receipt collision has owner-only mode");
    }

    let result = api
        .settle_local_materialization_run(&mut admitted)
        .expect("receipt collision settles a repository-scoped partial run");
    assert_eq!(result.outcome(), LocalMaterializationRunOutcome::Partial);
    assert_eq!(result.succeeded_target_count(), 1);
    assert_eq!(result.targets().len(), 2);
    assert_eq!(
        result.targets()[0].outcome(),
        LocalMaterializationTargetOutcome::Succeeded
    );
    assert_eq!(
        result.targets()[1].outcome(),
        LocalMaterializationTargetOutcome::FailedNoEffect
    );
    assert!(root
        .join("checkout/.pale/materialization-state.json")
        .exists());
    assert!(!root
        .join("mirror/.pale/materialization-state.json")
        .exists());
}

#[test]
fn local_drift_status_labels_preserve_each_status_coordinate() {
    let surface = LocalMaterializationDrift::Surface {
        relative_path: "generated/pale.md".to_owned(),
        action: LocalMaterializationAction::Create,
    };
    let ownership = LocalMaterializationDrift::Ownership {
        change: LocalMaterializationOwnershipChange::Initialize,
    };
    let machine_local = LocalMaterializationDrift::MachineLocal {
        kind: LocalMaterializationMachineEffectKind::GitExcludeProtection,
        relative_path: "info/exclude".to_owned(),
        action: LocalMaterializationAction::Splice,
    };

    assert_eq!(surface.status_label(), "generated/pale.md");
    assert_eq!(ownership.status_label(), ".pale/materialization-state.json");
    assert_eq!(
        machine_local.status_label(),
        "git_exclude_protection:info/exclude"
    );
}

#[test]
fn linked_worktree_status_and_plan_use_only_the_bound_common_git_directory() {
    let (temporary, manifest) = local_workspace();
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary workspace path is canonical");
    let checkout = root.join("checkout");
    let common = root.join("common.git");
    let worktree = common.join("worktrees/engine");
    let exclude = common.join("info/exclude");
    fs::create_dir_all(&worktree).expect("linked-worktree metadata is created");
    fs::create_dir_all(exclude.parent().expect("exclude has a parent"))
        .expect("common Git info directory is created");
    fs::write(
        checkout.join(".git"),
        format!("gitdir: {}\n", worktree.display()),
    )
    .expect("checkout gitfile is written");
    fs::write(
        worktree.join("gitdir"),
        format!("{}\n", checkout.join(".git").display()),
    )
    .expect("linked-worktree backlink is written");
    fs::write(worktree.join("commondir"), "../..\n")
        .expect("linked-worktree common directory is written");
    fs::write(&exclude, "# user-owned exclusion\n").expect("user-owned Git exclusion is written");

    let machine_path = root.join("machines/machine.toml");
    let mut machine: toml::Value =
        toml::from_str(&fs::read_to_string(&machine_path).expect("machine binding is readable"))
            .expect("machine binding is TOML");
    machine
        .get_mut("repository_checkouts")
        .and_then(toml::Value::as_array_mut)
        .and_then(|checkouts| checkouts.first_mut())
        .and_then(toml::Value::as_table_mut)
        .expect("the machine binding selects one repository checkout")
        .insert(
            "git_metadata_root".to_owned(),
            toml::Value::String(common.display().to_string()),
        );
    fs::write(
        &machine_path,
        toml::to_string(&machine).expect("machine binding serializes"),
    )
    .expect("machine binding is updated");

    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_local_one_shot(request(manifest.clone()))
        .expect("the linked-worktree fixture compiles");
    let before = fs::read(&exclude).expect("exclude baseline is readable");
    let status = api
        .inspect_local_repository_status(&compilation, "engine".to_owned())
        .expect("status resolves the verified common Git directory");
    assert!(status.drift().iter().any(|drift| {
        drift.status_label() == "git_exclude_protection:info/exclude"
            && matches!(
                drift,
                LocalMaterializationDrift::MachineLocal {
                    action: LocalMaterializationAction::Splice,
                    ..
                }
            )
    }));
    let plan = api
        .plan_local_one_shot(
            &compilation,
            LocalOneShotPlanRequest::new("engine".to_owned(), OneShotPlanKind::Apply),
        )
        .expect("planning resolves the verified common Git directory");
    assert_eq!(plan.counts().machine_local_effects(), 1);
    assert_eq!(
        plan.counts().total_operations(),
        plan.counts().operations() + 1
    );
    assert_eq!(
        fs::read(&exclude).expect("read-only calls preserve the exclude file"),
        before
    );
    assert!(!format!("{plan:?}").contains(&common.display().to_string()));

    let other_common = root.join("other.git");
    fs::create_dir_all(&other_common).expect("mismatching common directory is created");
    machine
        .get_mut("repository_checkouts")
        .and_then(toml::Value::as_array_mut)
        .and_then(|checkouts| checkouts.first_mut())
        .and_then(toml::Value::as_table_mut)
        .expect("the machine binding selects one repository checkout")
        .insert(
            "git_metadata_root".to_owned(),
            toml::Value::String(other_common.display().to_string()),
        );
    fs::write(
        machine_path,
        toml::to_string(&machine).expect("mismatching machine binding serializes"),
    )
    .expect("mismatching machine binding is written");
    let mismatched = api
        .compile_local_one_shot(request(manifest))
        .expect("the mismatching host binding remains a captured compilation");
    let error = api
        .inspect_local_repository_status(&mismatched, "engine".to_owned())
        .expect_err("a linked worktree cannot redirect outside its bound common directory");
    assert_eq!(
        error.diagnostic().code(),
        "local_git_protection_observation_failed"
    );
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::OperationalFailure
    );
    let error_debug = format!("{error:?}");
    assert!(!error_debug.contains(&common.display().to_string()));
    assert!(!error_debug.contains(&other_common.display().to_string()));
    assert_eq!(
        fs::read(&exclude).expect("failed observation preserves the exclude file"),
        before
    );
}

#[path = "local/doctor.rs"]
mod doctor;

#[path = "local/workspace_source.rs"]
mod workspace_source;
