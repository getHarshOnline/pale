//! Binding laws for API-owned configuration improvement.

use super::*;

fn workspace_documents_with_access(access: &str) -> Vec<crate::PyCompilationDocument> {
    let manifest = String::from_utf8(
        include_bytes!(concat!(
            "../fixtures/cross-leaf.v1/one-shot",
            "/workspace/pale-workspace.toml"
        ))
        .to_vec(),
    )
    .expect("workspace fixture is UTF-8")
    .replace("access = \"mutate\"", &format!("access = \"{access}\""));
    vec![
        document(
            "composition.toml",
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/composition.toml"
            )),
        ),
        document("pale-workspace.toml", manifest.as_bytes()),
        document(
            "workspace-resolution.lock.toml",
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/workspace-resolution.lock.toml"
            )),
        ),
        document(
            "workspace-universe.lock.toml",
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/workspace-universe.lock.toml"
            )),
        ),
    ]
}

fn portable_documents_with_access(access: &str) -> Vec<(String, Vec<u8>)> {
    let manifest = String::from_utf8(
        include_bytes!(concat!(
            "../fixtures/cross-leaf.v1/one-shot",
            "/workspace/pale-workspace.toml"
        ))
        .to_vec(),
    )
    .expect("workspace fixture is UTF-8")
    .replace("access = \"mutate\"", &format!("access = \"{access}\""));
    vec![
        (
            "composition.toml".to_owned(),
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/composition.toml"
            ))
            .to_vec(),
        ),
        ("pale-workspace.toml".to_owned(), manifest.into_bytes()),
        (
            "workspace-resolution.lock.toml".to_owned(),
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/workspace-resolution.lock.toml"
            ))
            .to_vec(),
        ),
        (
            "workspace-universe.lock.toml".to_owned(),
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/workspace-universe.lock.toml"
            ))
            .to_vec(),
        ),
    ]
}

fn compile_profile(
    python: &PalePython,
    py: pyo3::Python<'_>,
    workspace: &crate::PyPortableWorkspaceCompilation,
    identity: &crate::PyContractBundleIdentity,
) -> crate::PyEffectiveProfileCompilation {
    python
        .compile_effective_profile(
            py,
            workspace,
            crate::PyEffectiveProfileCompilationRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private_inputs(identity),
                package_documents(),
            ),
        )
        .expect("baseline profile compiles")
}

fn explain(
    python: &PalePython,
    py: pyo3::Python<'_>,
    workspace: &crate::PyPortableWorkspaceCompilation,
    profile: &crate::PyEffectiveProfileCompilation,
) -> crate::PyConfigurationImprovementExplanation {
    let inspection = python.inspect_configuration_improvement(py, workspace);
    assert_eq!(
        inspection.workspace_id(py).expect("inspection is live"),
        "act-ws"
    );
    assert_eq!(
        inspection
            .principal_universe_digest(py)
            .expect("inspection is live")
            .len(),
        64
    );
    python
        .explain_configuration_improvement(py, &inspection, profile)
        .expect("baseline and profile share one owner")
}

fn direct_preview(
    python: &PalePython,
    py: pyo3::Python<'_>,
    workspace: &crate::PyPortableWorkspaceCompilation,
    profile: &crate::PyEffectiveProfileCompilation,
    identity: &crate::PyContractBundleIdentity,
) -> crate::PyConfigurationImprovementPreview {
    let explanation = explain(python, py, workspace, profile);
    let proposal = python
        .propose_configuration_improvement(
            py,
            &explanation,
            crate::PyConfigurationImprovementProposalRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                crate::PyOneShotWorkspaceInputs::new(
                    "pale-workspace.toml".to_owned(),
                    workspace_documents_with_access("read"),
                ),
                private_inputs_at(identity, "/captured/repository-candidate"),
            ),
        )
        .expect("candidate source is captured");
    assert!(explanation.phase(py).is_err());
    assert_eq!(
        proposal.phase(py).expect("proposal is live"),
        crate::PyConfigurationImprovementPhase::Proposed
    );
    let evaluation = python
        .evaluate_configuration_improvement(py, &proposal)
        .expect("candidate admits");
    assert!(proposal.phase(py).is_err());
    assert_eq!(
        evaluation
            .candidate_profile_digest(py)
            .expect("evaluation is live")
            .len(),
        64
    );
    assert_eq!(
        evaluation
            .candidate_instance_digest(py)
            .expect("evaluation is live")
            .len(),
        64
    );
    let diagnosis = python
        .diagnose_configuration_improvement(py, &evaluation)
        .expect("candidate impact is derived");
    assert!(evaluation.phase(py).is_err());
    assert_eq!(
        diagnosis
            .derivation_receipt_digest(py)
            .expect("diagnosis is live")
            .len(),
        64
    );
    let preview = python
        .preview_configuration_improvement(py, &diagnosis)
        .expect("diagnosis projects");
    assert!(diagnosis.phase(py).is_err());
    preview
}

fn prepared_preview(
    python: &PalePython,
    py: pyo3::Python<'_>,
    baseline: &crate::PyPortableWorkspaceCompilation,
    profile: &crate::PyEffectiveProfileCompilation,
    identity: &crate::PyContractBundleIdentity,
) -> crate::PyConfigurationImprovementPreview {
    let candidate = python
        .compile_portable_workspace(
            py,
            "pale-workspace.toml".to_owned(),
            portable_documents_with_access("read"),
        )
        .expect("candidate workspace admits once");
    let explanation = explain(python, py, baseline, profile);
    let prepared = python
        .prepare_configuration_improvement_preview(
            py,
            &explanation,
            &candidate,
            crate::PyConfigurationImprovementCandidateRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                private_inputs_at(identity, "/captured/repository-candidate"),
            ),
        )
        .expect("admitted candidate is prepared without recompiling workspace source");
    python
        .commit_configuration_improvement_preview(py, &explanation, &prepared)
        .expect("exact explanation commits its preparation")
}

#[test]
fn direct_and_prepared_routes_share_the_golden_preview_and_retained_queries() {
    pyo3::Python::initialize();
    pyo3::Python::attach(|py| {
        let python = PalePython::new(py).expect("Python leaf initializes");
        let identity = python.contract_bundle_identity();
        let baseline = python
            .compile_portable_workspace(
                py,
                "pale-workspace.toml".to_owned(),
                portable_documents_with_access("mutate"),
            )
            .expect("baseline workspace admits");
        let profile = compile_profile(&python, py, &baseline, &identity);
        let direct = direct_preview(&python, py, &baseline, &profile, &identity);
        let prepared = prepared_preview(&python, py, &baseline, &profile, &identity);
        let expected = include_bytes!(concat!(
            "../fixtures/cross-leaf.v1/",
            "improvement-journey.json"
        ));
        let expected: serde_json::Value =
            serde_json::from_slice(expected).expect("golden fixture is JSON");
        let direct_summary = direct.summary(py).expect("preview is live");
        let prepared_summary = prepared.summary(py).expect("preview is live");
        let direct_bytes = direct_summary.canonical_bytes(py);
        let direct_value: serde_json::Value =
            serde_json::from_slice(direct_bytes.bind(py).as_bytes())
                .expect("direct summary is JSON");
        assert_eq!(direct_value, expected["summary"]);
        assert_eq!(
            direct_bytes.bind(py).as_bytes(),
            prepared_summary.canonical_bytes(py).bind(py).as_bytes()
        );

        let comparison = direct.comparison(py).expect("preview is live");
        assert_eq!(
            comparison
                .receipt_digest(py)
                .expect("live comparison is queryable"),
            expected["comparison"]["receipt_digest"]
        );
        assert_eq!(
            comparison
                .prior_semantic_ir_digest(py)
                .expect("live comparison is queryable"),
            expected["comparison"]["prior_semantic_ir_digest"]
        );
        assert_eq!(
            comparison
                .current_semantic_ir_digest(py)
                .expect("live comparison is queryable"),
            expected["comparison"]["current_semantic_ir_digest"]
        );
        assert_eq!(
            comparison
                .change_root_count(py)
                .expect("live comparison is queryable"),
            expected["comparison"]["change_root_count"]
                .as_u64()
                .expect("golden count is unsigned") as u32
        );
        assert_eq!(
            comparison
                .affected_count(py)
                .expect("live comparison is queryable"),
            expected["comparison"]["affected_count"]
                .as_u64()
                .expect("golden count is unsigned") as u32
        );
        assert_eq!(
            comparison
                .unaffected_count(py)
                .expect("live comparison is queryable"),
            expected["comparison"]["unaffected_count"]
                .as_u64()
                .expect("golden count is unsigned") as u32
        );
        let impacts = comparison.impacts(py, 0, 1).expect("impact page projects");
        let subject = impacts.entries()[0].subject();
        assert!(
            comparison
                .explain(py, &subject)
                .expect("retained comparison explains its subject")
                .subject()
                .kind()
                == subject.kind()
        );
        let baseline_profile = direct
            .baseline_profile(py)
            .expect("baseline profile remains retained");
        let candidate_profile = direct
            .candidate_profile(py)
            .expect("candidate profile remains retained");
        assert_eq!(
            baseline_profile.profile_digest(),
            direct_summary.profile_digest()
        );
        assert_eq!(
            candidate_profile.profile_digest(),
            direct_summary.candidate_profile_digest()
        );
        let navigation = candidate_profile.navigate_authoring(
            py,
            "pale-workspace.toml".to_owned(),
            vec![
                crate::PyAuthoringInstancePathSegment::field("workspace".to_owned()),
                crate::PyAuthoringInstancePathSegment::field("workspace_id".to_owned()),
            ],
        );
        assert_eq!(
            navigation
                .identity()
                .expect("workspace identity resolves")
                .value(),
            "act-ws"
        );

        direct.accept(py).expect("preview accepts exactly once");
        assert!(direct.accept(py).is_err());
        assert!(direct.phase(py).is_err());
        assert!(direct.summary(py).is_err());
        assert!(direct.comparison(py).is_err());
        assert!(comparison.impacts(py, 0, 1).is_err());
        assert_eq!(baseline_profile.workspace_id(), "act-ws");
    });
}

#[test]
fn preparation_is_retryable_and_commit_rejects_an_equal_coordinate_foreign_owner() {
    pyo3::Python::initialize();
    pyo3::Python::attach(|py| {
        let python = PalePython::new(py).expect("Python leaf initializes");
        let identity = python.contract_bundle_identity();
        let baseline = python
            .compile_portable_workspace(
                py,
                "pale-workspace.toml".to_owned(),
                portable_documents_with_access("mutate"),
            )
            .expect("baseline workspace admits");
        let candidate = python
            .compile_portable_workspace(
                py,
                "pale-workspace.toml".to_owned(),
                portable_documents_with_access("read"),
            )
            .expect("candidate workspace admits");
        let profile = compile_profile(&python, py, &baseline, &identity);
        let owner = explain(&python, py, &baseline, &profile);
        let foreign = explain(&python, py, &baseline, &profile);

        let rejected = python.prepare_configuration_improvement_preview(
            py,
            &owner,
            &candidate,
            crate::PyConfigurationImprovementCandidateRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                crate::PyOneShotPrivateInputs::new(
                    document("capabilities.toml", b"invalid = ["),
                    document("machines/machine.toml", b"invalid = ["),
                    document("distribution-lock.toml", b"invalid = ["),
                    Vec::new(),
                ),
            ),
        );
        assert!(rejected.is_err());

        let prepared = python
            .prepare_configuration_improvement_preview(
                py,
                &owner,
                &candidate,
                crate::PyConfigurationImprovementCandidateRequest::new(
                    "demo".to_owned(),
                    "default".to_owned(),
                    "machine".to_owned(),
                    private_inputs_at(&identity, "/captured/repository-candidate"),
                ),
            )
            .expect("failed preparation leaves its explanation retryable");
        let error = python
            .commit_configuration_improvement_preview(py, &foreign, &prepared)
            .err()
            .expect("equal coordinates cannot replace the private owner token");
        let diagnostic = error
            .value(py)
            .getattr("diagnostic")
            .expect("API error keeps the canonical diagnostic");
        assert_eq!(
            diagnostic
                .getattr("code")
                .expect("diagnostic code")
                .extract::<String>()
                .expect("code is a string"),
            "configuration_improvement_preparation_owner_mismatch"
        );
        assert!(prepared.summary(py).is_err());

        let replacement = python
            .prepare_configuration_improvement_preview(
                py,
                &owner,
                &candidate,
                crate::PyConfigurationImprovementCandidateRequest::new(
                    "demo".to_owned(),
                    "default".to_owned(),
                    "machine".to_owned(),
                    private_inputs_at(&identity, "/captured/repository-candidate"),
                ),
            )
            .expect("the exact owner remains available after a foreign commit attempt");
        assert!(python
            .commit_configuration_improvement_preview(py, &owner, &replacement)
            .is_ok());
        assert!(replacement.summary(py).is_err());
        assert!(python
            .commit_configuration_improvement_preview(py, &owner, &replacement)
            .is_err());
    });
}

#[test]
fn all_four_terminal_dispositions_are_explicit_and_effect_free() {
    pyo3::Python::initialize();
    pyo3::Python::attach(|py| {
        let python = PalePython::new(py).expect("Python leaf initializes");
        let identity = python.contract_bundle_identity();
        let baseline = python
            .compile_portable_workspace(
                py,
                "pale-workspace.toml".to_owned(),
                portable_documents_with_access("mutate"),
            )
            .expect("baseline workspace admits");
        let profile = compile_profile(&python, py, &baseline, &identity);
        let accepted = direct_preview(&python, py, &baseline, &profile, &identity)
            .accept(py)
            .expect("accept consumes one preview");
        let rejected = direct_preview(&python, py, &baseline, &profile, &identity)
            .reject(py)
            .expect("reject consumes one preview");
        let cancelled = direct_preview(&python, py, &baseline, &profile, &identity)
            .cancel(py)
            .expect("cancel consumes one preview");
        let stale = direct_preview(&python, py, &baseline, &profile, &identity)
            .mark_stale(py)
            .expect("caller-proved staleness consumes one preview");
        assert_eq!(
            accepted.disposition(),
            crate::PyConfigurationImprovementDisposition::Accepted
        );
        assert_eq!(
            rejected.disposition(),
            crate::PyConfigurationImprovementDisposition::Rejected
        );
        assert_eq!(
            cancelled.disposition(),
            crate::PyConfigurationImprovementDisposition::Cancelled
        );
        assert_eq!(
            stale.disposition(),
            crate::PyConfigurationImprovementDisposition::Stale
        );
        for terminal in [&accepted, &rejected, &cancelled, &stale] {
            assert_eq!(terminal.preview().workspace_id(), "act-ws");
        }
    });
}

#[test]
fn retained_handles_are_thread_safe_and_only_one_terminal_wins() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<crate::PyConfigurationImprovementInspection>();
    assert_send_sync::<crate::PyConfigurationImprovementExplanation>();
    assert_send_sync::<crate::PyConfigurationImprovementProposal>();
    assert_send_sync::<crate::PyConfigurationImprovementEvaluation>();
    assert_send_sync::<crate::PyConfigurationImprovementDiagnosis>();
    assert_send_sync::<crate::PyPreparedConfigurationImprovementPreview>();
    assert_send_sync::<crate::PyConfigurationImprovementPreview>();
    assert_send_sync::<crate::PyConfigurationImprovementComparison>();

    pyo3::Python::initialize();
    let preview = pyo3::Python::attach(|py| {
        let python = PalePython::new(py).expect("Python leaf initializes");
        let identity = python.contract_bundle_identity();
        let baseline = python
            .compile_portable_workspace(
                py,
                "pale-workspace.toml".to_owned(),
                portable_documents_with_access("mutate"),
            )
            .expect("baseline workspace admits");
        let profile = compile_profile(&python, py, &baseline, &identity);
        direct_preview(&python, py, &baseline, &profile, &identity)
    });
    let preview = std::sync::Arc::new(preview);
    let attempts = [
        {
            let preview = std::sync::Arc::clone(&preview);
            std::thread::spawn(move || pyo3::Python::attach(|py| preview.accept(py).is_ok()))
        },
        {
            let preview = std::sync::Arc::clone(&preview);
            std::thread::spawn(move || pyo3::Python::attach(|py| preview.cancel(py).is_ok()))
        },
    ];
    assert_eq!(
        attempts
            .into_iter()
            .map(|attempt| attempt.join().expect("terminal thread returns"))
            .filter(|accepted| *accepted)
            .count(),
        1
    );
}
