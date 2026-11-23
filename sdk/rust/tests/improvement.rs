//! Behavioral laws for the human-governed configuration-improvement lifecycle.

use pale_sdk::{
    CompilationDocument, ConfigurationImprovementCandidateRequest,
    ConfigurationImprovementCoordinateChange, ConfigurationImprovementDisposition,
    ConfigurationImprovementPhase, ConfigurationImprovementPreview,
    ConfigurationImprovementProposalRequest, EffectiveProfileCompilationRequest,
    OneShotPrivateInputs, PaleApi, PortableWorkspaceDocument, PortableWorkspaceImpactState,
    PortableWorkspaceRequest,
};

#[path = "support/conformance.rs"]
mod conformance;

macro_rules! fixture {
    ($member:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cross-leaf.v1/one-shot/",
            $member
        ))
    };
}

fn document(label: &str, bytes: impl Into<Vec<u8>>) -> CompilationDocument {
    CompilationDocument::new(label.to_owned(), bytes.into())
}

fn workspace(access: &str) -> PortableWorkspaceRequest {
    let manifest = String::from_utf8(fixture!("workspace/pale-workspace.toml").to_vec())
        .expect("workspace fixture is UTF-8")
        .replace("access = \"mutate\"", &format!("access = \"{access}\""));
    PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_owned(),
        vec![
            PortableWorkspaceDocument::new(
                "composition.toml".to_owned(),
                fixture!("workspace/composition.toml").to_vec(),
            ),
            PortableWorkspaceDocument::new("pale-workspace.toml".to_owned(), manifest.into_bytes()),
            PortableWorkspaceDocument::new(
                "workspace-resolution.lock.toml".to_owned(),
                fixture!("workspace/workspace-resolution.lock.toml").to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "workspace-universe.lock.toml".to_owned(),
                fixture!("workspace/workspace-universe.lock.toml").to_vec(),
            ),
        ],
    )
}

fn workspace_with_alternate_context() -> PortableWorkspaceRequest {
    let manifest = String::from_utf8(fixture!("workspace/pale-workspace.toml").to_vec())
        .expect("workspace fixture is UTF-8")
        .replace(
            " projects = [",
            " projects = [{ id = \"alternate\", title = \"Alternate\", run_policy = \"standard\", package_seeds = [\"example.engine\"], authority_root_ids = [\"authority\"], capabilities_file = \"capabilities.toml\" }, ",
        )
        .replace(
            " project_repositories = [",
            " project_repositories = [{ project_id = \"alternate\", repository_id = \"engine\", role = { package_id = \"example.engine\", resource_id = \"engine\" }, stacks = [], families = [], materialization_order = 0 }, ",
        )
        .replace(
            " project_execution_workspaces = [",
            " project_execution_workspaces = [{ project_id = \"alternate\", execution_workspace_id = \"alternate\" }, ",
        )
        .replace(
            " execution_workspaces = [",
            " execution_workspaces = [{ id = \"alternate\", title = \"Alternate\", execution_path_ids = [\"direct-claude\"] }, ",
        )
        .replace(
            " execution_workspace_repositories = [",
            " execution_workspace_repositories = [{ execution_workspace_id = \"alternate\", repository_id = \"engine\", access = \"read\" }, ",
        );
    PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_owned(),
        vec![
            PortableWorkspaceDocument::new(
                "composition.toml".to_owned(),
                fixture!("workspace/composition.toml").to_vec(),
            ),
            PortableWorkspaceDocument::new("pale-workspace.toml".to_owned(), manifest.into_bytes()),
            PortableWorkspaceDocument::new(
                "workspace-resolution.lock.toml".to_owned(),
                fixture!("workspace/workspace-resolution.lock.toml").to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "workspace-universe.lock.toml".to_owned(),
                fixture!("workspace/workspace-universe.lock.toml").to_vec(),
            ),
        ],
    )
}

fn distribution_lock(api: &PaleApi, compiler: &str) -> Vec<u8> {
    let lock = String::from_utf8(fixture!("private/distribution-lock.toml").to_vec())
        .expect("distribution fixture is UTF-8")
        .replace("pale-surface@1", compiler);
    let identity = api.contract_bundle_identity();
    let lock = lock
        .lines()
        .map(|line| {
            if line.starts_with("schema_manifest_sha256 = ") {
                format!(
                    "schema_manifest_sha256 = \"{}\"",
                    identity.schema_manifest_sha256()
                )
            } else if line.starts_with("schema_set_sha256 = ") {
                format!("schema_set_sha256 = \"{}\"", identity.schema_set_sha256())
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    lock.into_bytes()
}

fn private(api: &PaleApi, machine_root: &str, compiler: &str) -> OneShotPrivateInputs {
    let machine = String::from_utf8(fixture!("private/machine-binding.toml").to_vec())
        .expect("machine fixture is UTF-8")
        .replace("/captured/repository", machine_root);
    OneShotPrivateInputs::new(
        document("capabilities.toml", fixture!("private/capabilities.toml")),
        document("machines/machine.toml", machine.into_bytes()),
        document("distribution-lock.toml", distribution_lock(api, compiler)),
        Vec::new(),
    )
}

fn packages() -> Vec<CompilationDocument> {
    vec![
        document(
            "engine/package.toml",
            fixture!("packages/engine/package.toml"),
        ),
        document("engine/roles.toml", fixture!("packages/engine/roles.toml")),
        document("engine/role.md", fixture!("packages/engine/role.md")),
    ]
}

fn preview(machine_root: &str) -> ConfigurationImprovementPreview {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let baseline_workspace = api
        .compile_portable_workspace(workspace("mutate"))
        .expect("baseline workspace admits");
    let baseline_profile = api
        .compile_effective_profile(
            &baseline_workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private(&api, "/captured/repository", "pale-surface@1"),
                packages(),
            ),
        )
        .expect("baseline effective profile compiles");

    let inspection = api.inspect_configuration_improvement(&baseline_workspace);
    assert_eq!(inspection.phase(), ConfigurationImprovementPhase::Inspected);
    let explanation = api
        .explain_configuration_improvement(inspection, &baseline_profile)
        .expect("the profile shares the baseline frontier");
    assert_eq!(
        explanation.phase(),
        ConfigurationImprovementPhase::Explained
    );
    let proposal = api
        .propose_configuration_improvement(
            explanation,
            ConfigurationImprovementProposalRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                workspace("mutate"),
                private(&api, machine_root, "pale-surface@1"),
            ),
        )
        .expect("bounded candidate capture succeeds");
    assert_eq!(proposal.phase(), ConfigurationImprovementPhase::Proposed);
    let evaluation = api
        .evaluate_configuration_improvement(proposal)
        .expect("candidate reuses the admitted Package frontier");
    assert_eq!(evaluation.phase(), ConfigurationImprovementPhase::Evaluated);
    assert_eq!(
        evaluation.candidate_principal_universe_digest(),
        baseline_workspace.principal_universe_digest()
    );
    let diagnosis = api
        .diagnose_configuration_improvement(evaluation)
        .expect("admitted owners derive one impact proof");
    assert_eq!(diagnosis.phase(), ConfigurationImprovementPhase::Diagnosed);
    let preview = api
        .preview_configuration_improvement(diagnosis)
        .expect("verified coordinates form a bounded preview");
    assert_eq!(preview.phase(), ConfigurationImprovementPhase::Previewed);
    preview
}

#[test]
fn full_candidate_reuses_package_semantics_and_tracks_private_change() {
    let preview = preview("/captured/repository-candidate");
    let summary = preview.summary();

    assert_eq!(summary.profile_digest(), summary.candidate_profile_digest());
    assert_eq!(
        summary.selected_topology_digest(),
        summary.candidate_selected_topology_digest()
    );
    assert_ne!(
        summary.instance_digest(),
        summary.candidate_instance_digest()
    );
    assert!(matches!(
        summary.selected_topology_change(),
        ConfigurationImprovementCoordinateChange::Unchanged { .. }
    ));
    assert!(matches!(
        summary.profile_change(),
        ConfigurationImprovementCoordinateChange::Unchanged { .. }
    ));
    assert!(matches!(
        summary.instance_change(),
        ConfigurationImprovementCoordinateChange::Changed { .. }
    ));
    assert_eq!(
        preview.candidate_profile().profile_digest(),
        summary.candidate_profile_digest()
    );
    assert_eq!(preview.comparison().change_root_count(), 0);
}

#[test]
fn unlike_explanation_frontier_is_rejected_before_proposal() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let baseline = api
        .compile_portable_workspace(workspace("mutate"))
        .expect("baseline workspace admits");
    let unlike = api
        .compile_portable_workspace(workspace("read"))
        .expect("unlike workspace admits independently");
    let profile = api
        .compile_effective_profile(
            &baseline,
            EffectiveProfileCompilationRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private(&api, "/captured/repository", "pale-surface@1"),
                packages(),
            ),
        )
        .expect("baseline effective profile compiles");

    let error = api
        .explain_configuration_improvement(api.inspect_configuration_improvement(&unlike), &profile)
        .err()
        .expect("unlike semantic owners cannot be joined");
    assert_eq!(
        error.diagnostic().code(),
        "configuration_improvement_frontier_mismatch"
    );
}

#[test]
fn candidate_rejection_and_distribution_drift_are_non_effectful() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let baseline_workspace = api
        .compile_portable_workspace(workspace("mutate"))
        .expect("baseline workspace admits");
    let profile = api
        .compile_effective_profile(
            &baseline_workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private(&api, "/captured/repository", "pale-surface@1"),
                packages(),
            ),
        )
        .expect("baseline effective profile compiles");
    let explanation = api
        .explain_configuration_improvement(
            api.inspect_configuration_improvement(&baseline_workspace),
            &profile,
        )
        .expect("baseline joins its profile");
    let proposal = api
        .propose_configuration_improvement(
            explanation,
            ConfigurationImprovementProposalRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                workspace("mutate"),
                private(&api, "/captured/repository", "pale-surface@2"),
            ),
        )
        .expect("candidate capture is structural");
    let error = api
        .evaluate_configuration_improvement(proposal)
        .err()
        .expect("another distribution frontier needs separate admission");
    assert_eq!(
        error.diagnostic().code(),
        "configuration_improvement_distribution_frontier_mismatch"
    );
}

#[test]
fn invalid_private_candidate_is_rejected_before_preview() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let baseline_workspace = api
        .compile_portable_workspace(workspace("mutate"))
        .expect("baseline workspace admits");
    let baseline_profile = api
        .compile_effective_profile(
            &baseline_workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private(&api, "/captured/repository", "pale-surface@1"),
                packages(),
            ),
        )
        .expect("baseline effective profile compiles");
    let explanation = api
        .explain_configuration_improvement(
            api.inspect_configuration_improvement(&baseline_workspace),
            &baseline_profile,
        )
        .expect("baseline joins its profile");
    let invalid_private = OneShotPrivateInputs::new(
        document("capabilities.toml", b"invalid = [".to_vec()),
        document(
            "machines/machine.toml",
            fixture!("private/machine-binding.toml"),
        ),
        document(
            "distribution-lock.toml",
            distribution_lock(&api, "pale-surface@1"),
        ),
        Vec::new(),
    );
    let proposal = api
        .propose_configuration_improvement(
            explanation,
            ConfigurationImprovementProposalRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                workspace("mutate"),
                invalid_private,
            ),
        )
        .expect("proposal capture does not pretend to parse private TOML");
    let error = api
        .evaluate_configuration_improvement(proposal)
        .err()
        .expect("invalid private source cannot reach preview");
    assert_eq!(
        error.diagnostic().code(),
        "configuration_improvement_candidate_rejected"
    );
}

#[test]
fn failed_preparation_preserves_explanation_and_reuses_the_admitted_candidate() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let baseline_workspace = api
        .compile_portable_workspace(workspace("mutate"))
        .expect("baseline workspace admits");
    let candidate_workspace = api
        .compile_portable_workspace(workspace("read"))
        .expect("candidate workspace admits once before improvement preparation");
    let baseline_profile = api
        .compile_effective_profile(
            &baseline_workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private(&api, "/captured/repository", "pale-surface@1"),
                packages(),
            ),
        )
        .expect("baseline effective profile compiles");
    let explanation = api
        .explain_configuration_improvement(
            api.inspect_configuration_improvement(&baseline_workspace),
            &baseline_profile,
        )
        .expect("baseline joins its profile");
    let invalid_private = OneShotPrivateInputs::new(
        document("capabilities.toml", b"invalid = [".to_vec()),
        document(
            "machines/machine.toml",
            fixture!("private/machine-binding.toml"),
        ),
        document(
            "distribution-lock.toml",
            distribution_lock(&api, "pale-surface@1"),
        ),
        Vec::new(),
    );
    let error = api
        .prepare_configuration_improvement_preview(
            &explanation,
            &candidate_workspace,
            ConfigurationImprovementCandidateRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                invalid_private,
            ),
        )
        .err()
        .expect("invalid private input cannot prepare a preview");
    assert_eq!(
        error.diagnostic().code(),
        "configuration_improvement_candidate_rejected"
    );

    let prepared = api
        .prepare_configuration_improvement_preview(
            &explanation,
            &candidate_workspace,
            ConfigurationImprovementCandidateRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private(&api, "/captured/repository", "pale-surface@1"),
            ),
        )
        .expect("the same explanation remains available for a corrected retry");
    assert_eq!(
        prepared.summary().candidate_principal_universe_digest(),
        candidate_workspace.principal_universe_digest(),
        "preparation must retain the exact already admitted candidate generation",
    );
    let preview = api
        .commit_configuration_improvement_preview(explanation, prepared)
        .expect("the originating explanation commits its own preparation");
    assert_eq!(
        preview.summary().candidate_principal_universe_digest(),
        candidate_workspace.principal_universe_digest(),
    );
}

#[test]
fn candidate_cannot_splice_another_valid_profile_context() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let workspace = api
        .compile_portable_workspace(workspace_with_alternate_context())
        .expect("the workspace admits both review contexts");
    let baseline_profile = api
        .compile_effective_profile(
            &workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private(&api, "/captured/repository", "pale-surface@1"),
                packages(),
            ),
        )
        .expect("the baseline context is valid");
    api.compile_effective_profile(
        &workspace,
        EffectiveProfileCompilationRequest::new(
            "alternate".to_owned(),
            "alternate".to_owned(),
            "machine".to_owned(),
            private(&api, "/captured/repository", "pale-surface@1"),
            packages(),
        ),
    )
    .expect("the candidate context is independently valid");
    let explanation = api
        .explain_configuration_improvement(
            api.inspect_configuration_improvement(&workspace),
            &baseline_profile,
        )
        .expect("baseline joins its profile");

    let error = api
        .prepare_configuration_improvement_preview(
            &explanation,
            &workspace,
            ConfigurationImprovementCandidateRequest::new(
                "alternate".to_owned(),
                "alternate".to_owned(),
                "machine".to_owned(),
                private(&api, "/captured/repository", "pale-surface@1"),
            ),
        )
        .err()
        .expect("one review cannot change its declared profile context");
    assert_eq!(
        error.diagnostic().code(),
        "configuration_improvement_candidate_context_mismatch"
    );
}

#[test]
fn preparation_commits_only_through_its_originating_explanation() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let workspace = api
        .compile_portable_workspace(workspace("mutate"))
        .expect("workspace admits");
    let profile = api
        .compile_effective_profile(
            &workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private(&api, "/captured/repository", "pale-surface@1"),
                packages(),
            ),
        )
        .expect("profile compiles");
    let origin = api
        .explain_configuration_improvement(
            api.inspect_configuration_improvement(&workspace),
            &profile,
        )
        .expect("origin explanation admits");
    let unlike_owner = api
        .explain_configuration_improvement(
            api.inspect_configuration_improvement(&workspace),
            &profile,
        )
        .expect("an independent equal-coordinate explanation admits");
    assert_eq!(origin.profile_digest(), unlike_owner.profile_digest());
    assert_eq!(origin.instance_digest(), unlike_owner.instance_digest());
    let prepared = api
        .prepare_configuration_improvement_preview(
            &origin,
            &workspace,
            ConfigurationImprovementCandidateRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private(&api, "/captured/repository", "pale-surface@1"),
            ),
        )
        .expect("origin prepares its candidate");

    let error = api
        .commit_configuration_improvement_preview(unlike_owner, prepared)
        .err()
        .expect("equal digests cannot substitute for the originating owner");
    assert_eq!(
        error.diagnostic().code(),
        "configuration_improvement_preparation_owner_mismatch"
    );
}

#[test]
fn only_a_preview_can_reach_each_terminal_disposition() {
    let dispositions = [
        preview("/captured/repository").accept().disposition(),
        preview("/captured/repository").reject().disposition(),
        preview("/captured/repository").cancel().disposition(),
        preview("/captured/repository").mark_stale().disposition(),
    ];
    assert_eq!(
        dispositions,
        [
            ConfigurationImprovementDisposition::Accepted,
            ConfigurationImprovementDisposition::Rejected,
            ConfigurationImprovementDisposition::Cancelled,
            ConfigurationImprovementDisposition::Stale,
        ]
    );
}

// Deliberately change JSON spelling without passing through a sorted map again.
fn reversed_summary_members(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(members) => format!(
            "{{{}}}",
            members
                .iter()
                .rev()
                .map(|(key, value)| format!(
                    "{}:{}",
                    serde_json::to_string(key).unwrap(),
                    reversed_summary_members(value)
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
        other => serde_json::to_string(other).unwrap(),
    }
}

#[test]
fn presented_review_accepts_equivalent_json_for_each_explicit_disposition() {
    let api = PaleApi::new().unwrap();
    let presented = preview("/captured/repository-candidate");
    let expected = presented.summary().canonical_bytes().to_vec();
    let value: serde_json::Value = serde_json::from_slice(&expected).unwrap();
    let canonical = String::from_utf8(expected.clone()).unwrap();
    assert!(canonical.contains("\"demo\""));
    let forms = [
        canonical.clone(),
        serde_json::to_string_pretty(&value).unwrap(),
        reversed_summary_members(&value),
        canonical.replace("\"demo\"", "\"\\u0064emo\""),
    ];
    for disposition in [
        ConfigurationImprovementDisposition::Accepted,
        ConfigurationImprovementDisposition::Rejected,
        ConfigurationImprovementDisposition::Cancelled,
        ConfigurationImprovementDisposition::Stale,
    ] {
        for form in &forms {
            // A separately reconstructed lifecycle may match the presented
            // compiled review. This does not prove human assent or outer drift.
            let terminal = api
                .finish_presented_configuration_improvement(
                    preview("/captured/repository-candidate"),
                    form.as_bytes(),
                    disposition,
                )
                .unwrap();
            assert_eq!(terminal.disposition(), disposition);
            assert_eq!(terminal.preview().canonical_bytes(), expected);
        }
    }
}

#[test]
fn borrowing_presented_review_check_preserves_paging_and_explicit_finish() {
    let api = PaleApi::new().unwrap();
    let current = preview("/captured/repository-candidate");
    let summary = current.summary().canonical_bytes().to_vec();
    let expected_page = current.comparison().impacts(0, 1);
    let value: serde_json::Value = serde_json::from_slice(&summary).unwrap();
    for bytes in [
        summary.clone(),
        serde_json::to_vec_pretty(&value).unwrap(),
        reversed_summary_members(&value).into_bytes(),
    ] {
        api.validate_presented_configuration_improvement(&current, &bytes)
            .unwrap();
        assert_eq!(current.comparison().impacts(0, 1), expected_page);
    }
    let error = api
        .validate_presented_configuration_improvement(&current, b"{}")
        .unwrap_err();
    assert_eq!(
        error.diagnostic().code(),
        "configuration_improvement_summary_mismatch"
    );
    assert_eq!(current.comparison().impacts(0, 1), expected_page);
    assert_eq!(current.summary().canonical_bytes(), summary);
    let terminal = api
        .finish_presented_configuration_improvement(
            current,
            &summary,
            ConfigurationImprovementDisposition::Rejected,
        )
        .unwrap();
    assert_eq!(terminal.preview().canonical_bytes(), summary);
    assert_eq!(
        terminal.disposition(),
        ConfigurationImprovementDisposition::Rejected
    );
}

fn assert_presented_rejection(bytes: &[u8], code: &str, stage: pale_sdk::DiagnosticStage) {
    use std::error::Error as _;

    let api = PaleApi::new().unwrap();
    let error = api
        .finish_presented_configuration_improvement(
            preview("/captured/repository-candidate"),
            bytes,
            ConfigurationImprovementDisposition::Accepted,
        )
        .err()
        .expect("rejected presentation must not return a terminal disposition");
    let diagnostic = error.diagnostic();
    let retained = preview("/captured/repository-candidate");
    let validation = api
        .validate_presented_configuration_improvement(&retained, bytes)
        .expect_err("the borrowing check must reject the same presentation");
    assert_eq!(
        validation.diagnostic().canonical_bytes(),
        diagnostic.canonical_bytes()
    );
    assert_eq!(diagnostic.code(), code);
    assert_eq!(diagnostic.stage(), stage);
    assert_eq!(
        diagnostic.disposition(),
        pale_sdk::DiagnosticDisposition::InvalidInput
    );
    assert_eq!(
        diagnostic.retry(),
        pale_sdk::DiagnosticRetry::AfterCorrection
    );
    assert_eq!(
        diagnostic.effect(),
        pale_sdk::DiagnosticEffectCertainty::NotApplicable
    );
    assert!(error.source().is_none());
    assert!(diagnostic.subject().is_none());
    for visible in [
        error.to_string(),
        format!("{error:?}"),
        diagnostic.canonical_json().to_owned(),
    ] {
        assert!(!visible.contains("private-presented-marker"));
    }
}

// Paths come from the actual owned summary, so added fields join this wall
// without a copied list of the production projection's members.
fn summary_member_paths(value: &serde_json::Value, parent: &str, paths: &mut Vec<String>) {
    if let serde_json::Value::Object(members) = value {
        for (key, value) in members {
            let key = key.replace('~', "~0").replace('/', "~1");
            let path = format!("{parent}/{key}");
            paths.push(path.clone());
            summary_member_paths(value, &path, paths);
        }
    }
}

#[test]
fn presented_review_checks_every_field_and_rejects_additional_members() {
    let owned = preview("/captured/repository-candidate");
    let value: serde_json::Value =
        serde_json::from_slice(owned.summary().canonical_bytes()).unwrap();
    let mut paths = Vec::new();
    summary_member_paths(&value, "", &mut paths);
    assert!(!paths.is_empty());
    for path in &paths {
        let mut changed = value.clone();
        *changed.pointer_mut(path).unwrap() = serde_json::Value::Null;
        assert_ne!(changed, value);
        assert_presented_rejection(
            &serde_json::to_vec(&changed).unwrap(),
            "configuration_improvement_summary_mismatch",
            pale_sdk::DiagnosticStage::SemanticAdmission,
        );

        let mut missing = value.clone();
        let (parent, key) = path.rsplit_once('/').unwrap();
        let key = key.replace("~1", "/").replace("~0", "~");
        assert!(missing
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(&key)
            .is_some());
        assert_presented_rejection(
            &serde_json::to_vec(&missing).unwrap(),
            "configuration_improvement_summary_mismatch",
            pale_sdk::DiagnosticStage::SemanticAdmission,
        );
    }
    for path in std::iter::once("").chain(paths.iter().map(String::as_str)) {
        let mut extended = value.clone();
        if let Some(members) = extended.pointer_mut(path).unwrap().as_object_mut() {
            assert!(members
                .insert("private-presented-marker".into(), true.into())
                .is_none());
            assert_presented_rejection(
                &serde_json::to_vec(&extended).unwrap(),
                "configuration_improvement_summary_mismatch",
                pale_sdk::DiagnosticStage::SemanticAdmission,
            );
        }
    }
}

#[test]
fn presented_review_cannot_accept_a_different_private_instance() {
    let api = PaleApi::new().unwrap();
    let presented = preview("/captured/repository");
    let current = preview("/captured/repository-candidate");
    assert_eq!(
        presented.summary().candidate_principal_universe_digest(),
        current.summary().candidate_principal_universe_digest()
    );
    assert_eq!(
        presented.summary().candidate_profile_digest(),
        current.summary().candidate_profile_digest()
    );
    assert_ne!(
        presented.summary().candidate_instance_digest(),
        current.summary().candidate_instance_digest()
    );
    let error = api
        .finish_presented_configuration_improvement(
            current,
            presented.summary().canonical_bytes(),
            ConfigurationImprovementDisposition::Accepted,
        )
        .err()
        .expect("portable equality must not erase the changed private instance");
    assert_eq!(
        error.diagnostic().code(),
        "configuration_improvement_summary_mismatch"
    );
}

#[test]
fn presented_review_bounds_input_before_parsing() {
    let api = PaleApi::new().unwrap();
    let current = preview("/captured/repository-candidate");
    let expected = current.summary().canonical_bytes().to_vec();
    let limit = pale_sdk::ConfigurationImprovementPreviewSummary::MAX_PRESENTED_BYTES;
    assert!(expected.len() < limit);
    let mut padded = expected.clone();
    padded.resize(limit, b' ');
    let terminal = api
        .finish_presented_configuration_improvement(
            current,
            &padded,
            ConfigurationImprovementDisposition::Rejected,
        )
        .unwrap();
    assert_eq!(
        terminal.disposition(),
        ConfigurationImprovementDisposition::Rejected
    );
    assert_eq!(terminal.preview().canonical_bytes(), expected);
    padded.push(b' ');
    assert_presented_rejection(
        &padded,
        "configuration_improvement_summary_too_large",
        pale_sdk::DiagnosticStage::InputAdmission,
    );
    padded[0] = b'!';
    assert_presented_rejection(
        &padded,
        "configuration_improvement_summary_too_large",
        pale_sdk::DiagnosticStage::InputAdmission,
    );
}

#[test]
fn presented_review_uses_strict_json_and_keeps_rejected_bytes_private() {
    let mut rejected = vec![
        b"private-presented-marker".to_vec(),
        br#"{"private-presented-marker":1,"private-presented-marker":2}"#.to_vec(),
        br#"{"private-presented-marker":1,"\u0070rivate-presented-marker":2}"#.to_vec(),
        br#"{"private-presented-marker":1.0}"#.to_vec(),
        br#"{"private-presented-marker":9007199254740992}"#.to_vec(),
        b"{} private-presented-marker".to_vec(),
        vec![0xff],
    ];
    rejected.push(format!("{}0{}", "[".repeat(256), "]".repeat(256)).into_bytes());
    for bytes in rejected {
        assert_presented_rejection(
            &bytes,
            "configuration_improvement_summary_invalid_json",
            pale_sdk::DiagnosticStage::InputAdmission,
        );
    }
    for valid_but_unlike in [b"null".as_slice(), b"[]", b"true", b"0"] {
        assert_presented_rejection(
            valid_but_unlike,
            "configuration_improvement_summary_mismatch",
            pale_sdk::DiagnosticStage::SemanticAdmission,
        );
    }
}

#[test]
fn cross_leaf_review_vector_is_recomputed_from_both_api_routes() {
    let api = PaleApi::new().expect("API initializes");
    let baseline = api.compile_portable_workspace(workspace("mutate")).unwrap();
    let candidate = api.compile_portable_workspace(workspace("read")).unwrap();
    let profile = api
        .compile_effective_profile(
            &baseline,
            EffectiveProfileCompilationRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private(&api, "/captured/repository", "pale-surface@1"),
                packages(),
            ),
        )
        .unwrap();
    let explain = || {
        api.explain_configuration_improvement(
            api.inspect_configuration_improvement(&baseline),
            &profile,
        )
        .unwrap()
    };
    let candidate_request = |project: &str| {
        ConfigurationImprovementCandidateRequest::new(
            project.to_owned(),
            "default".to_owned(),
            "machine".to_owned(),
            private(&api, "/captured/repository-candidate", "pale-surface@1"),
        )
    };
    let origin = explain();
    let context_error = api
        .prepare_configuration_improvement_preview(
            &origin,
            &candidate,
            candidate_request("unselected"),
        )
        .err()
        .expect("a review cannot change its selected context");
    let foreign_preparation = api
        .prepare_configuration_improvement_preview(&origin, &candidate, candidate_request("demo"))
        .unwrap();
    let owner_error = api
        .commit_configuration_improvement_preview(explain(), foreign_preparation)
        .err()
        .expect("equal coordinates are not the originating explanation owner");
    let prepared = api
        .prepare_configuration_improvement_preview(&origin, &candidate, candidate_request("demo"))
        .expect("preparation errors leave the original explanation retryable");
    let prepared_preview = api
        .commit_configuration_improvement_preview(origin, prepared)
        .unwrap();
    let proposal = api
        .propose_configuration_improvement(
            explain(),
            ConfigurationImprovementProposalRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                workspace("read"),
                private(&api, "/captured/repository-candidate", "pale-surface@1"),
            ),
        )
        .unwrap();
    let evaluated = api.evaluate_configuration_improvement(proposal).unwrap();
    let diagnosed = api.diagnose_configuration_improvement(evaluated).unwrap();
    let source_preview = api.preview_configuration_improvement(diagnosed).unwrap();
    let roots = source_preview.change_roots(0, 1);
    let root = roots
        .entries()
        .first()
        .expect("the workspace access change has an exact root");
    let impacts = source_preview.impacts(0, u16::MAX);
    let affected = impacts
        .entries()
        .iter()
        .find(|impact| impact.state() == PortableWorkspaceImpactState::Affected)
        .expect("the changed root reaches at least one candidate subject");
    let explanation = source_preview
        .explain_impact(affected.subject())
        .expect("retained proof explains its affected subject");
    assert_eq!(explanation.subject(), affected.subject());
    assert!(explanation.causal().is_some());
    let source = source_preview
        .inspect_candidate_subject(root.subject(), None, 8, 64, 64)
        .expect("the exact candidate supplies bounded source evidence");
    assert_eq!(source.subject(), root.subject());
    assert_eq!(
        source.semantic_ir_digest(),
        source_preview
            .summary()
            .candidate_workspace_semantic_ir_digest(),
    );
    let bounded_candidate_source = source_preview
        .inspect_candidate_subject(root.subject(), None, 1, 1, 1)
        .expect("the candidate supplies an owner-bound continuation");
    let candidate_cursor = bounded_candidate_source
        .continuation()
        .expect("one-row bounds leave candidate evidence unfinished");
    let baseline_error = baseline
        .inspect_subject(root.subject(), Some(candidate_cursor), 1, 1, 1)
        .expect_err("candidate continuation cannot be used on the baseline owner");
    let baseline_diagnostic: serde_json::Value =
        serde_json::from_slice(baseline_error.diagnostic().canonical_bytes()).unwrap();
    assert_eq!(
        baseline_diagnostic["code"], "workspace_subject_inspection_continuation_mismatch",
        "source evidence must retain the candidate owner even when public digests are equal",
    );
    assert_eq!(
        source_preview.summary().canonical_bytes(),
        prepared_preview.summary().canonical_bytes(),
        "source and retained-candidate routes have one semantic result",
    );
    let summary = prepared_preview.summary();
    let projected = serde_json::to_value(summary).unwrap();
    for (field, actual) in [
        (
            "baseline_workspace_semantic_ir_digest",
            summary.baseline_workspace_semantic_ir_digest(),
        ),
        (
            "baseline_workspace_admission_evidence_digest",
            summary.baseline_workspace_admission_evidence_digest(),
        ),
        (
            "package_semantic_ir_digest",
            summary.package_semantic_ir_digest(),
        ),
        ("composition_plan_digest", summary.composition_plan_digest()),
        (
            "candidate_workspace_semantic_ir_digest",
            summary.candidate_workspace_semantic_ir_digest(),
        ),
        (
            "candidate_workspace_admission_evidence_digest",
            summary.candidate_workspace_admission_evidence_digest(),
        ),
        (
            "candidate_package_semantic_ir_digest",
            summary.candidate_package_semantic_ir_digest(),
        ),
        (
            "candidate_composition_plan_digest",
            summary.candidate_composition_plan_digest(),
        ),
    ] {
        assert_eq!(projected[field], actual, "typed identity {field}");
    }
    let diagnostic = |error: &pale_sdk::ApiError| {
        serde_json::from_slice::<serde_json::Value>(error.diagnostic().canonical_bytes()).unwrap()
    };
    let vector = serde_json::json!({
        "capability_id": "workspace.configuration-improvement",
        "candidate_repository_access": "read",
        "candidate_machine_root": "/captured/repository-candidate",
        "summary": projected,
        "comparison": prepared_preview.comparison().summary(),
        "diagnostics": {
            "context_mismatch": diagnostic(&context_error),
            "preparation_owner_mismatch": diagnostic(&owner_error),
        },
    });
    let expected: serde_json::Value = serde_json::from_slice(include_bytes!(
        "fixtures/cross-leaf.v1/improvement-journey.json"
    ))
    .unwrap();
    conformance::assert_json("improvement-journey.json", &vector, &expected);
}
