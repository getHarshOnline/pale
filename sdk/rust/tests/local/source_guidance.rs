//! Actual confined observations through the public SDK; no synthetic writer proof.

use super::*;
use pale_sdk::{
    LocalSourceGuidanceBaseline, LocalSourceGuidanceChange, LocalSourceGuidanceIntent,
    LocalSourceGuidancePlanRequest, SourceGuidanceHost,
};

fn plan_request(
    manifest: &str,
    host: SourceGuidanceHost,
    intent: LocalSourceGuidanceIntent,
) -> LocalSourceGuidancePlanRequest {
    LocalSourceGuidancePlanRequest::new(request(manifest.to_owned()), "engine".into(), host, intent)
}

#[test]
fn source_planning_reviews_both_layouts_and_exact_local_baselines_without_writes() {
    let (temporary, manifest) = source_workspace();
    let checkout = temporary.path().canonicalize().unwrap().join("checkout");
    let facts = "Private local acceptance: preserve this exact fact without final newline";
    for entry in ["AGENTS.md", "CLAUDE.md"] {
        fs::write(checkout.join(entry), facts).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(checkout.join(entry), fs::Permissions::from_mode(0o640)).unwrap();
        }
    }
    let api = PaleApi::new().unwrap();
    for (host, entry, state) in [
        (
            SourceGuidanceHost::Codex,
            "AGENTS.md",
            ".pale/source-guidance-state/codex.json",
        ),
        (
            SourceGuidanceHost::ClaudeCode,
            "AGENTS.md",
            ".pale/source-guidance-state/claude-code.json",
        ),
    ] {
        let plan = api
            .plan_local_source_guidance(plan_request(
                &manifest,
                host,
                LocalSourceGuidanceIntent::Apply,
            ))
            .unwrap();
        let report = plan.report();
        assert_eq!(report.repository_id(), "engine");
        assert_eq!(report.host(), host);
        assert_eq!(report.intent(), LocalSourceGuidanceIntent::Apply);
        let projected = report
            .files()
            .iter()
            .find(|f| f.rel_path() == entry)
            .unwrap();
        assert!(matches!(
            projected.baseline(),
            LocalSourceGuidanceBaseline::Present { .. }
        ));
        #[cfg(unix)]
        assert!(matches!(
            projected.baseline(),
            LocalSourceGuidanceBaseline::Present { mode: 0o640, .. }
        ));
        let LocalSourceGuidanceChange::Write { content, .. } = projected.change() else {
            panic!("native entry must be spliced")
        };
        assert!(content.starts_with(facts));
        let owned_state = report
            .files()
            .iter()
            .find(|f| f.rel_path() == state)
            .unwrap();
        let LocalSourceGuidanceChange::Write { mode, content, .. } = owned_state.change() else {
            panic!("private state must be proposed")
        };
        assert_eq!(*mode, 0o600);
        assert!(!content.contains(facts));
        assert!(!content.contains(checkout.to_str().unwrap()));
        assert!(!format!("{plan:?}").contains(facts));
        assert!(!format!("{plan:?}").contains(checkout.to_str().unwrap()));
        let json = serde_json::to_string(report).unwrap();
        assert!(
            json.contains(facts),
            "only deliberate review serialization contains preserved facts"
        );
        assert!(!json.contains(checkout.to_str().unwrap()));
        let repeated = api
            .plan_local_source_guidance(plan_request(
                &manifest,
                host,
                LocalSourceGuidanceIntent::Apply,
            ))
            .unwrap();
        assert_eq!(report.digest(), repeated.report().digest());
        fs::write(checkout.join(entry), format!("{facts}\nNew project fact.")).unwrap();
        let changed = api
            .plan_local_source_guidance(plan_request(
                &manifest,
                host,
                LocalSourceGuidanceIntent::Apply,
            ))
            .unwrap();
        assert_ne!(report.digest(), changed.report().digest());
        fs::write(checkout.join(entry), facts).unwrap();
        let removed = api
            .plan_local_source_guidance(plan_request(
                &manifest,
                host,
                LocalSourceGuidanceIntent::Remove,
            ))
            .unwrap();
        assert!(removed
            .report()
            .files()
            .iter()
            .all(|f| matches!(f.change(), LocalSourceGuidanceChange::Keep)));
        assert!(!checkout.join(state).exists());
        for file in report.files().iter().filter(|f| f.rel_path() != entry) {
            assert!(!checkout.join(file.rel_path()).exists());
        }
    }
    assert_eq!(fs::read_dir(&checkout).unwrap().count(), 2);
    for entry in ["AGENTS.md", "CLAUDE.md"] {
        assert_eq!(fs::read_to_string(checkout.join(entry)).unwrap(), facts);
    }
}

#[test]
fn source_planning_refuses_unowned_files_and_unselected_repositories() {
    let (temporary, manifest) = source_workspace();
    let checkout = temporary.path().canonicalize().unwrap().join("checkout");
    let api = PaleApi::new().unwrap();
    let profile = api
        .compile_local_effective_profile(request(manifest.clone()))
        .unwrap();
    let projection = profile
        .source_guidance("engine", SourceGuidanceHost::Codex)
        .unwrap();
    let file = &projection.files()[0];
    write(&checkout, file.rel_path(), file.content().as_bytes());
    let error = api
        .plan_local_source_guidance(plan_request(
            &manifest,
            SourceGuidanceHost::Codex,
            LocalSourceGuidanceIntent::Apply,
        ))
        .unwrap_err();
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::Blocked
    );
    assert_eq!(error.diagnostic().code(), "source_guidance_plan_rejected");
    assert_eq!(
        error.diagnostic().subject(),
        Some(("source_guidance_file", file.rel_path()))
    );
    assert!(error
        .diagnostic()
        .notes()
        .iter()
        .any(|n| n.contains("without source-guidance ownership")));
    assert_eq!(
        fs::read_to_string(checkout.join(file.rel_path())).unwrap(),
        file.content()
    );
    let unselected = LocalSourceGuidancePlanRequest::new(
        request(manifest),
        "unselected".into(),
        SourceGuidanceHost::Codex,
        LocalSourceGuidanceIntent::Apply,
    );
    assert_eq!(
        api.plan_local_source_guidance(unselected)
            .unwrap_err()
            .diagnostic()
            .disposition(),
        DiagnosticDisposition::InvalidInput
    );
    assert!(!checkout.join("AGENTS.md").exists());
}

#[cfg(unix)]
#[test]
fn source_planning_rejects_symlinked_ancestors_and_oversized_entries() {
    use std::os::unix::fs::symlink;
    let (temporary, manifest) = source_workspace();
    let checkout = temporary.path().canonicalize().unwrap().join("checkout");
    let outside = temporary.path().join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, checkout.join(".pale")).unwrap();
    let api = PaleApi::new().unwrap();
    let error = api
        .plan_local_source_guidance(plan_request(
            &manifest,
            SourceGuidanceHost::Codex,
            LocalSourceGuidanceIntent::Apply,
        ))
        .unwrap_err();
    assert_eq!(
        error.diagnostic().code(),
        "source_guidance_observation_failed"
    );
    assert!(!format!("{error:?}").contains(outside.to_str().unwrap()));
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    fs::remove_file(checkout.join(".pale")).unwrap();
    fs::write(checkout.join("AGENTS.md"), vec![b'x'; 8 * 1024 * 1024 + 1]).unwrap();
    let error = api
        .plan_local_source_guidance(plan_request(
            &manifest,
            SourceGuidanceHost::Codex,
            LocalSourceGuidanceIntent::Apply,
        ))
        .unwrap_err();
    assert_eq!(
        error.diagnostic().code(),
        "source_guidance_observation_failed"
    );
    assert_eq!(
        fs::metadata(checkout.join("AGENTS.md")).unwrap().len(),
        8 * 1024 * 1024 + 1
    );
    assert!(!checkout.join(".pale").exists());
}

#[test]
fn reviewed_public_writes_preserve_both_layouts_and_remove_only_owned_files() {
    use pale_sdk::LocalSourceGuidanceWriteRequest;
    let (temporary, manifest) = source_workspace();
    let checkout = temporary.path().canonicalize().unwrap().join("checkout");
    let api = PaleApi::new().unwrap();
    for (host, entry, state) in [
        (SourceGuidanceHost::Codex, "AGENTS.md", "codex"),
        (SourceGuidanceHost::ClaudeCode, "AGENTS.md", "claude-code"),
    ] {
        fs::write(
            checkout.join(entry),
            "Local acceptance without final newline",
        )
        .unwrap();
        let review = api
            .plan_local_source_guidance(plan_request(
                &manifest,
                host,
                LocalSourceGuidanceIntent::Apply,
            ))
            .unwrap();
        let result = api
            .apply_local_source_guidance(LocalSourceGuidanceWriteRequest::new(
                plan_request(&manifest, host, LocalSourceGuidanceIntent::Apply),
                review.report().digest().to_owned(),
            ))
            .unwrap();
        assert_eq!(result.plan_digest(), review.report().digest());
        assert!(result.changed_files() > 0);
        assert!(fs::read_to_string(checkout.join(entry))
            .unwrap()
            .starts_with("Local acceptance without final newline\n"));
        let clean = api
            .plan_local_source_guidance(plan_request(
                &manifest,
                host,
                LocalSourceGuidanceIntent::Apply,
            ))
            .unwrap();
        assert!(clean.report().publication_order().is_empty());
        assert_eq!(
            api.apply_local_source_guidance(LocalSourceGuidanceWriteRequest::new(
                plan_request(&manifest, host, LocalSourceGuidanceIntent::Apply),
                clean.report().digest().into()
            ))
            .unwrap()
            .changed_files(),
            0
        );
        let current = fs::read_to_string(checkout.join(entry)).unwrap();
        fs::write(
            checkout.join(entry),
            format!("{current}Later local adaptation.\n"),
        )
        .unwrap();
        let remove = api
            .plan_local_source_guidance(plan_request(
                &manifest,
                host,
                LocalSourceGuidanceIntent::Remove,
            ))
            .unwrap();
        api.apply_local_source_guidance(LocalSourceGuidanceWriteRequest::new(
            plan_request(&manifest, host, LocalSourceGuidanceIntent::Remove),
            remove.report().digest().into(),
        ))
        .unwrap();
        assert_eq!(
            fs::read_to_string(checkout.join(entry)).unwrap(),
            "Local acceptance without final newline\nLater local adaptation.\n"
        );
        assert!(!checkout
            .join(format!(".pale/source-guidance-state/{state}.json"))
            .exists());
        assert!(!checkout
            .join(".pale-source-guidance-recovery.json")
            .exists());
    }
}

#[test]
fn public_write_rebuilds_sources_and_baselines_and_never_accepts_an_old_review() {
    use pale_sdk::LocalSourceGuidanceWriteRequest;
    for change_source in [false, true] {
        let (temporary, manifest) = source_workspace();
        let root = temporary.path().canonicalize().unwrap();
        let checkout = root.join("checkout");
        let api = PaleApi::new().unwrap();
        let host = SourceGuidanceHost::Codex;
        fs::write(checkout.join("AGENTS.md"), "local before").unwrap();
        let review = api
            .plan_local_source_guidance(plan_request(
                &manifest,
                host,
                LocalSourceGuidanceIntent::Apply,
            ))
            .unwrap();
        if change_source {
            fs::write(
                root.join("distribution/packages/engine/role.md"),
                "changed after review",
            )
            .unwrap();
        } else {
            fs::write(checkout.join("AGENTS.md"), "independent correction").unwrap();
        }
        let error = api
            .apply_local_source_guidance(LocalSourceGuidanceWriteRequest::new(
                plan_request(&manifest, host, LocalSourceGuidanceIntent::Apply),
                review.report().digest().into(),
            ))
            .unwrap_err();
        if !change_source {
            assert_eq!(error.diagnostic().code(), "source_guidance_review_changed");
            assert_eq!(
                error.diagnostic().effect(),
                DiagnosticEffectCertainty::NotStarted
            );
        }
        assert_eq!(
            fs::read_to_string(checkout.join("AGENTS.md")).unwrap(),
            if change_source {
                "local before"
            } else {
                "independent correction"
            }
        );
        assert!(!checkout.join(".pale-source-guidance.lock").exists());
        assert!(!checkout.join(".pale").exists());
    }
}
