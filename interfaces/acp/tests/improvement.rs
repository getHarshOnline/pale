//! Composition of an ACP conductor with separately owned Pale review handles.

use pale_acp::AcpConductor;
use pale_sdk::{
    CompilationDocument, ConfigurationImprovementCandidateRequest,
    ConfigurationImprovementDisposition, ConfigurationImprovementPreview,
    ConfigurationImprovementProposalRequest, EffectiveProfileCompilation,
    EffectiveProfileCompilationRequest, EffectiveProfilePageRequest, EffectiveProfileSection,
    OneShotPrivateInputs, PaleApi, PortableWorkspaceCompilation, PortableWorkspaceDocument,
    PortableWorkspaceRequest,
};

fn fixture(member: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/cross-leaf.v1")
            .join(member),
    )
    .unwrap()
}

fn workspace(access: &str) -> PortableWorkspaceRequest {
    let documents = [
        "composition.toml",
        "pale-workspace.toml",
        "workspace-resolution.lock.toml",
        "workspace-universe.lock.toml",
    ]
    .into_iter()
    .map(|label| {
        let bytes = fixture(&format!("one-shot/workspace/{label}"));
        let bytes = if label == "pale-workspace.toml" {
            String::from_utf8(bytes)
                .unwrap()
                .replace("access = \"mutate\"", &format!("access = \"{access}\""))
                .into_bytes()
        } else {
            bytes
        };
        PortableWorkspaceDocument::new(label.to_owned(), bytes)
    })
    .collect();
    PortableWorkspaceRequest::new("pale-workspace.toml".to_owned(), documents)
}

fn private(candidate: bool) -> OneShotPrivateInputs {
    let machine = String::from_utf8(fixture("one-shot/private/machine-binding.toml")).unwrap();
    let machine = if candidate {
        machine.replace("/captured/repository", "/captured/repository-candidate")
    } else {
        machine
    };
    OneShotPrivateInputs::new(
        CompilationDocument::new(
            "capabilities.toml".to_owned(),
            fixture("one-shot/private/capabilities.toml"),
        ),
        CompilationDocument::new("machines/machine.toml".to_owned(), machine.into_bytes()),
        CompilationDocument::new(
            "distribution-lock.toml".to_owned(),
            fixture("one-shot/private/distribution-lock.toml"),
        ),
        Vec::new(),
    )
}

fn baseline(api: &PaleApi) -> (PortableWorkspaceCompilation, EffectiveProfileCompilation) {
    let workspace = api.compile_portable_workspace(workspace("mutate")).unwrap();
    let packages = ["package.toml", "roles.toml", "role.md"]
        .into_iter()
        .map(|name| {
            CompilationDocument::new(
                format!("engine/{name}"),
                fixture(&format!("one-shot/packages/engine/{name}")),
            )
        })
        .collect();
    let profile = api
        .compile_effective_profile(
            &workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private(false),
                packages,
            ),
        )
        .unwrap();
    (workspace, profile)
}

fn request(project: &str) -> ConfigurationImprovementCandidateRequest {
    ConfigurationImprovementCandidateRequest::new(
        project.to_owned(),
        "default".to_owned(),
        "machine".to_owned(),
        private(true),
    )
}

fn assert_preview(preview: &ConfigurationImprovementPreview) {
    let golden: serde_json::Value =
        serde_json::from_slice(&fixture("improvement-journey.json")).unwrap();
    assert_eq!(
        serde_json::to_value(preview.summary()).unwrap(),
        golden["summary"]
    );
    assert_eq!(
        serde_json::to_value(preview.comparison().summary()).unwrap(),
        golden["comparison"]
    );
    assert!(!preview.comparison().change_roots(0, 1).entries().is_empty());
    let impacts = preview.comparison().impacts(0, 1);
    preview
        .comparison()
        .explain(impacts.entries()[0].subject())
        .unwrap();
    for profile in [preview.baseline_profile(), preview.candidate_profile()] {
        let page = profile.inspect(EffectiveProfilePageRequest::new(
            EffectiveProfileSection::EffectiveConfiguration,
            0,
            1,
        ));
        assert!(profile
            .explain(page.entries()[0].explanation_handle())
            .is_some());
    }
}

#[test]
fn both_routes_preserve_complete_review_and_all_terminal_choices() {
    let _conductor = AcpConductor::new().unwrap();
    let api = PaleApi::new().unwrap();
    let (baseline, profile) = baseline(&api);
    let candidate = api.compile_portable_workspace(workspace("read")).unwrap();
    for (source_route, disposition) in [
        (true, ConfigurationImprovementDisposition::Accepted),
        (false, ConfigurationImprovementDisposition::Rejected),
        (true, ConfigurationImprovementDisposition::Cancelled),
        (false, ConfigurationImprovementDisposition::Stale),
    ] {
        let explanation = api
            .explain_configuration_improvement(
                api.inspect_configuration_improvement(&baseline),
                &profile,
            )
            .unwrap();
        let preview = if source_route {
            let proposal = api
                .propose_configuration_improvement(
                    explanation,
                    ConfigurationImprovementProposalRequest::new(
                        "demo".to_owned(),
                        "default".to_owned(),
                        "machine".to_owned(),
                        workspace("read"),
                        private(true),
                    ),
                )
                .unwrap();
            let evaluation = api.evaluate_configuration_improvement(proposal).unwrap();
            let diagnosis = api.diagnose_configuration_improvement(evaluation).unwrap();
            api.preview_configuration_improvement(diagnosis).unwrap()
        } else {
            let prepared = api
                .prepare_configuration_improvement_preview(
                    &explanation,
                    &candidate,
                    request("demo"),
                )
                .unwrap();
            api.commit_configuration_improvement_preview(explanation, prepared)
                .unwrap()
        };
        assert_preview(&preview);
        let expected = preview.summary().canonical_bytes().to_vec();
        let terminal = match disposition {
            ConfigurationImprovementDisposition::Accepted => preview.accept(),
            ConfigurationImprovementDisposition::Rejected => preview.reject(),
            ConfigurationImprovementDisposition::Cancelled => preview.cancel(),
            ConfigurationImprovementDisposition::Stale => preview.mark_stale(),
        };
        assert_eq!(terminal.disposition(), disposition);
        assert_eq!(terminal.preview().canonical_bytes(), expected);
    }
}

#[test]
fn retry_and_foreign_owner_rejection_keep_canonical_diagnostics() {
    let _conductor = AcpConductor::new().unwrap();
    let api = PaleApi::new().unwrap();
    let (baseline, profile) = baseline(&api);
    let candidate = api.compile_portable_workspace(workspace("read")).unwrap();
    let explain = || {
        api.explain_configuration_improvement(
            api.inspect_configuration_improvement(&baseline),
            &profile,
        )
        .unwrap()
    };
    let origin = explain();
    let golden: serde_json::Value =
        serde_json::from_slice(&fixture("improvement-journey.json")).unwrap();
    let context_error = api
        .prepare_configuration_improvement_preview(&origin, &candidate, request("unselected"))
        .err()
        .expect("context drift rejects preparation");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(context_error.diagnostic().canonical_bytes())
            .unwrap(),
        golden["diagnostics"]["context_mismatch"]
    );
    let prepared = api
        .prepare_configuration_improvement_preview(&origin, &candidate, request("demo"))
        .unwrap();
    let owner_error = api
        .commit_configuration_improvement_preview(explain(), prepared)
        .err()
        .expect("equal coordinates cannot replace explanation ownership");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(owner_error.diagnostic().canonical_bytes())
            .unwrap(),
        golden["diagnostics"]["preparation_owner_mismatch"]
    );
    let retry = api
        .prepare_configuration_improvement_preview(&origin, &candidate, request("demo"))
        .unwrap();
    let preview = api
        .commit_configuration_improvement_preview(origin, retry)
        .unwrap();
    assert_preview(&preview);
}
