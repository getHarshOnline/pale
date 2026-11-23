//! Application planning controls. Seeded files are synthetic prior state;
//! direct test writes are not native guarded-apply or recovery qualification.

use std::os::unix::fs::PermissionsExt;

use pale_adapter_fs::{observe_machine_local_file, ConfinedRoot, ReadLimit};
use pale_application::plan::source_guidance::{
    ownership_path, prepare_source_guidance, GuidanceFileChange, GuidancePlanError,
    GuidancePlanIntent, SourceGuidancePlan,
};

use super::*;

#[path = "source_guidance_context.rs"]
mod private_context;

#[path = "source_guidance_shared_entry.rs"]
mod shared_entry;

fn fixture() -> SyntheticWorkspace {
    let fixture = super::fixture(&[]);
    remove_runtime_paths(&fixture);
    let path = &fixture.machine_binding;
    let mut machine: toml::Value = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    machine["repository_checkouts"][0]["checkout_root"] = fixture
        .root
        .path()
        .join("repository")
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .into();
    std::fs::write(path, toml::to_string(&machine).unwrap()).unwrap();
    fixture
}

fn root(fixture: &SyntheticWorkspace) -> PathBuf {
    fixture
        .root
        .path()
        .join("repository")
        .canonicalize()
        .unwrap()
}

fn observe(root: &Path, path: &str) -> pale_ports::MachineLocalFileObservation {
    observe_machine_local_file(
        &ConfinedRoot::open(root).unwrap(),
        path,
        ReadLimit::new(8 * 1024 * 1024),
    )
    .unwrap()
}

fn plan(
    fixture: &SyntheticWorkspace,
    host: SourceGuidanceHost,
    intent: GuidancePlanIntent,
) -> Result<SourceGuidancePlan, GuidancePlanError> {
    let compilation = effective(fixture);
    let root = root(fixture);
    let prepared = prepare_source_guidance(
        &compilation,
        &LogicalRepositoryIdV1::parse("engine").unwrap(),
        host,
        intent,
        observe(&root, &ownership_path(host)),
    )?;
    let targets = prepared
        .required_paths()
        .into_iter()
        .map(|path| {
            let observation = observe(&root, &path);
            (path, observation)
        })
        .collect();
    prepared.finish(targets)
}

/// Establish synthetic previous-generation state only; deliberately no effect proof.
fn seed(root: &Path, plan: &SourceGuidancePlan) {
    for file in plan.files() {
        let path = root.join(file.rel_path());
        match file.change() {
            GuidanceFileChange::Keep => {}
            GuidanceFileChange::Write { content, mode } => {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, content).unwrap();
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode.get()))
                    .unwrap();
            }
            GuidanceFileChange::Delete => std::fs::remove_file(path).unwrap(),
        }
    }
}

#[test]
fn reviewed_admission_binds_exact_plan_and_publication_order() {
    let fixture = fixture();
    let proposed = plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Apply,
    )
    .unwrap();
    let order = proposed.publication_order().to_vec();
    assert_eq!(
        order.last().unwrap(),
        &ownership_path(SourceGuidanceHost::Codex)
    );
    let entry = order.iter().position(|p| p == "AGENTS.md").unwrap();
    let index = order
        .iter()
        .position(|p| p == ".pale/guidance/codex/index.md")
        .unwrap();
    assert!(index < entry);
    assert!(proposed
        .admit(&ArtifactDigest::hash_bytes(b"unreviewed proposal"))
        .is_err());
    let proposed = plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Apply,
    )
    .unwrap();
    let reviewed = proposed.digest().clone();
    let admitted = proposed.admit(&reviewed).unwrap();
    assert_eq!(admitted.digest(), &reviewed);
    let admitted_order = admitted
        .files()
        .iter()
        .filter(|f| !matches!(f.change(), GuidanceFileChange::Keep))
        .map(|f| f.rel_path().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(admitted_order, order);
}

#[test]
fn recovery_scope_accepts_disjoint_owned_inventories_and_rejects_the_next_file() {
    use pale_ports::{MAX_SOURCE_GUIDANCE_OWNED_FILES, MAX_SOURCE_GUIDANCE_TRANSACTION_FILES};
    for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
        let fixture = fixture();
        let proposed = plan(&fixture, host, GuidancePlanIntent::Apply).unwrap();
        let digest = proposed.digest().clone();
        let admitted = proposed.admit(&digest).unwrap();
        let name = if host == SourceGuidanceHost::Codex {
            "codex"
        } else {
            "claude-code"
        };
        let entry = "AGENTS.md";
        let mut files = vec![".gitignore".into(), entry.into(), ownership_path(host)];
        for generation in ["old", "new"] {
            files.extend(
                (0..MAX_SOURCE_GUIDANCE_OWNED_FILES)
                    .map(|i| format!(".pale/guidance/{name}/roles/pale-role-{generation}-{i}.md")),
            );
        }
        assert_eq!(files.len(), MAX_SOURCE_GUIDANCE_TRANSACTION_FILES);
        let compilation = effective(&fixture);
        let repository = LogicalRepositoryIdV1::parse("engine").unwrap();
        for over_budget in [false, true] {
            if over_budget {
                files.push(format!(
                    ".pale/guidance/{name}/roles/pale-role-over-budget.md"
                ));
            }
            let proposal = pale_ports::SourceGuidanceRecoveryProposal::from_native_capture(
                admitted.root().clone(),
                admitted.owner_sha256().clone(),
                digest.clone(),
                files.clone(),
                vec![],
            );
            assert_eq!(
                compilation
                    .admit_source_guidance_recovery(&repository, host, &proposal, &digest)
                    .is_ok(),
                !over_budget
            );
        }
    }
}

#[test]
fn admitted_native_publication_updates_and_retires_both_layouts_with_local_facts_intact() {
    use pale_adapter_fs::prepare_source_guidance_write;
    for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
        let fixture = fixture();
        let root = root(&fixture);
        let entry = "AGENTS.md";
        std::fs::write(root.join(entry), "Project facts without a final newline").unwrap();
        let proposed = plan(&fixture, host, GuidancePlanIntent::Apply).unwrap();
        let reviewed = proposed.digest().clone();
        let paths = proposed
            .files()
            .iter()
            .filter(|f| f.rel_path() != entry)
            .map(|f| f.rel_path().to_owned())
            .collect::<Vec<_>>();
        let applied = prepare_source_guidance_write(proposed.admit(&reviewed).unwrap())
            .unwrap()
            .publish()
            .unwrap();
        assert!(applied.commit().unwrap().changed_files > 0);
        assert!(std::fs::read_to_string(root.join(entry))
            .unwrap()
            .starts_with("Project facts without a final newline\n"));
        let clean = plan(&fixture, host, GuidancePlanIntent::Apply).unwrap();
        assert!(clean.publication_order().is_empty());
        assert!(clean
            .files()
            .iter()
            .all(|f| matches!(f.change(), GuidanceFileChange::Keep)));
        let path = &fixture.manifest;
        let mut manifest: toml::Value =
            toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        manifest["workspace"]["desired_topology"]["project_repositories"][0]["role"]
            ["resource_id"] = "security".into();
        std::fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
        let changed = plan(&fixture, host, GuidancePlanIntent::Apply).unwrap();
        let reviewed = changed.digest().clone();
        prepare_source_guidance_write(changed.admit(&reviewed).unwrap())
            .unwrap()
            .publish()
            .unwrap()
            .commit()
            .unwrap();
        assert!(paths
            .iter()
            .filter(|p| p.ends_with("change.md"))
            .all(|p| !root.join(p).exists()));
        let text = std::fs::read_to_string(root.join(entry)).unwrap();
        std::fs::write(
            root.join(entry),
            format!("{text}Later project adaptation.\n"),
        )
        .unwrap();
        let removed = plan(&fixture, host, GuidancePlanIntent::Remove).unwrap();
        assert_eq!(
            removed.publication_order().first().map(String::as_str),
            Some(entry)
        );
        let owned = removed
            .files()
            .iter()
            .filter(|f| matches!(f.change(), GuidanceFileChange::Delete))
            .map(|f| f.rel_path().to_owned())
            .collect::<Vec<_>>();
        let reviewed = removed.digest().clone();
        prepare_source_guidance_write(removed.admit(&reviewed).unwrap())
            .unwrap()
            .publish()
            .unwrap()
            .commit()
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join(entry)).unwrap(),
            "Project facts without a final newline\nLater project adaptation.\n"
        );
        assert!(owned.iter().all(|p| !root.join(p).exists()));
        assert!(std::fs::read_to_string(root.join(".gitignore"))
            .unwrap()
            .contains("/.pale-source-guidance.lock"));
        assert!(!root
            .join(pale_adapter_fs::SOURCE_GUIDANCE_RECOVERY_PATH)
            .exists());
        assert!(root
            .join(pale_adapter_fs::SOURCE_GUIDANCE_LOCK_PATH)
            .is_file());
    }
}

fn leave_published_journal(fixture: &SyntheticWorkspace, host: SourceGuidanceHost) {
    let proposed = plan(fixture, host, GuidancePlanIntent::Apply).unwrap();
    let reviewed = proposed.digest().clone();
    let applied =
        pale_adapter_fs::prepare_source_guidance_write(proposed.admit(&reviewed).unwrap())
            .unwrap()
            .publish()
            .unwrap();
    let journal = root(fixture).join(pale_adapter_fs::SOURCE_GUIDANCE_RECOVERY_PATH);
    // An actual custody refusal leaves the published files and private journal.
    std::fs::set_permissions(&journal, std::fs::Permissions::from_mode(0o400)).unwrap();
    assert!(applied.commit().is_err());
    std::fs::set_permissions(journal, std::fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn cold_restore_requires_exact_review_and_logical_scope_for_both_layouts() {
    use pale_adapter_fs::{plan_source_guidance_recovery, SourceGuidanceRecoveryIntent};
    for host in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
        let fixture = fixture();
        let root = root(&fixture);
        let entry = "AGENTS.md";
        std::fs::write(root.join(entry), "Local facts survive cold restoration.\n").unwrap();
        leave_published_journal(&fixture, host);
        let recovery = plan_source_guidance_recovery(
            ConfinedRoot::open(&root).unwrap(),
            SourceGuidanceRecoveryIntent::RestoreBefore,
        )
        .unwrap()
        .unwrap();
        let compilation = effective(&fixture);
        let repository = LogicalRepositoryIdV1::parse("engine").unwrap();
        assert!(compilation
            .admit_source_guidance_recovery(
                &repository,
                host,
                recovery.proposal(),
                &ArtifactDigest::hash_bytes(b"unreviewed")
            )
            .is_err());
        let wrong_host = if host == SourceGuidanceHost::Codex {
            SourceGuidanceHost::ClaudeCode
        } else {
            SourceGuidanceHost::Codex
        };
        assert!(compilation
            .admit_source_guidance_recovery(
                &repository,
                wrong_host,
                recovery.proposal(),
                recovery.digest()
            )
            .is_err());
        let claims = recovery.proposal();
        let escaped = pale_ports::SourceGuidanceRecoveryProposal::from_native_capture(
            claims.root().clone(),
            claims.owner_sha256().clone(),
            claims.digest().clone(),
            vec!["README.md".into()],
            vec![],
        );
        assert!(compilation
            .admit_source_guidance_recovery(&repository, host, &escaped, escaped.digest())
            .is_err());
        let admitted = compilation
            .admit_source_guidance_recovery(&repository, host, claims, recovery.digest())
            .unwrap();
        let outcome = recovery.execute(admitted).unwrap();
        assert!(outcome.changed_files > 0);
        assert_eq!(
            std::fs::read_to_string(root.join(entry)).unwrap(),
            "Local facts survive cold restoration.\n"
        );
        assert!(!root.join(ownership_path(host)).exists());
        assert!(!root
            .join(pale_adapter_fs::SOURCE_GUIDANCE_RECOVERY_PATH)
            .exists());
    }
}

#[test]
fn cold_finalize_keeps_only_a_complete_scope_admitted_native_after_state() {
    use pale_adapter_fs::{plan_source_guidance_recovery, SourceGuidanceRecoveryIntent};
    let fixture = fixture();
    let root = root(&fixture);
    let host = SourceGuidanceHost::Codex;
    leave_published_journal(&fixture, host);
    let recovery = plan_source_guidance_recovery(
        ConfinedRoot::open(&root).unwrap(),
        SourceGuidanceRecoveryIntent::FinalizePublished,
    )
    .unwrap()
    .unwrap();
    let compilation = effective(&fixture);
    let repository = LogicalRepositoryIdV1::parse("engine").unwrap();
    let admitted = compilation
        .admit_source_guidance_recovery(&repository, host, recovery.proposal(), recovery.digest())
        .unwrap();
    assert_eq!(recovery.execute(admitted).unwrap().changed_files, 0);
    let clean = plan(&fixture, host, GuidancePlanIntent::Apply).unwrap();
    assert!(clean.publication_order().is_empty());
    assert!(!root
        .join(pale_adapter_fs::SOURCE_GUIDANCE_RECOVERY_PATH)
        .exists());
}

fn write_content<'a>(plan: &'a SourceGuidancePlan, path: &str) -> &'a str {
    match plan
        .files()
        .iter()
        .find(|file| file.rel_path() == path)
        .unwrap()
        .change()
    {
        GuidanceFileChange::Write { content, .. } => content,
        _ => panic!("expected a proposed write to {path}"),
    }
}

#[test]
fn initial_plan_preserves_local_bytes_and_noop_rechecks_every_owned_path() {
    for (host, entry) in [
        (SourceGuidanceHost::Codex, "AGENTS.md"),
        (SourceGuidanceHost::ClaudeCode, "AGENTS.md"),
    ] {
        let fixture = fixture();
        let root = root(&fixture);
        let local = "Local facts, constraints and acceptance criteria; no final newline.";
        std::fs::write(root.join(entry), local).unwrap();
        std::fs::set_permissions(root.join(entry), std::fs::Permissions::from_mode(0o640)).unwrap();
        let proposed = plan(&fixture, host, GuidancePlanIntent::Apply).unwrap();
        assert!(write_content(&proposed, entry)
            .starts_with(&format!("{local}\n<!-- pale:source-guidance ")));
        let state = write_content(&proposed, proposed.ownership_path());
        assert!(
            !state.contains(root.to_str().unwrap()),
            "native root is retained as an opaque digest"
        );
        assert!(
            !state.contains(local),
            "local facts must not become ownership metadata"
        );
        assert_eq!(std::fs::read_to_string(root.join(entry)).unwrap(), local);
        assert!(!root.join(proposed.ownership_path()).exists());
        seed(&root, &proposed);
        assert_eq!(
            std::fs::metadata(root.join(entry))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o640
        );
        assert_eq!(
            std::fs::metadata(root.join(proposed.ownership_path()))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );
        let noop = plan(&fixture, host, GuidancePlanIntent::Apply).unwrap();
        assert!(noop
            .files()
            .iter()
            .all(|file| matches!(file.change(), GuidanceFileChange::Keep)));
        assert_eq!(
            noop.digest(),
            plan(&fixture, host, GuidancePlanIntent::Apply)
                .unwrap()
                .digest()
        );
    }
}

#[test]
fn changed_role_removes_only_superseded_files_and_removal_preserves_new_local_facts() {
    let fixture = fixture();
    let root = root(&fixture);
    std::fs::write(root.join("AGENTS.md"), "Original local facts").unwrap();
    let initial = plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Apply,
    )
    .unwrap();
    seed(&root, &initial);
    let entry = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
    std::fs::write(
        root.join("AGENTS.md"),
        format!("New local constraint\n{entry}Local tail\n"),
    )
    .unwrap();
    std::fs::write(root.join("unrelated.md"), "Independent project knowledge").unwrap();
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&fixture.manifest).unwrap()).unwrap();
    manifest["workspace"]["desired_topology"]["project_repositories"][0]["role"]["resource_id"] =
        "security".into();
    std::fs::write(&fixture.manifest, toml::to_string(&manifest).unwrap()).unwrap();
    let changed = plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Apply,
    )
    .unwrap();
    assert!(changed
        .files()
        .iter()
        .any(|file| file.rel_path().ends_with("change.md")
            && matches!(file.change(), GuidanceFileChange::Delete)));
    assert!(changed
        .files()
        .iter()
        .any(|file| file.rel_path().ends_with("security.md")
            && matches!(file.change(), GuidanceFileChange::Write { .. })));
    assert!(!changed
        .files()
        .iter()
        .any(|file| file.rel_path() == "unrelated.md"));
    seed(&root, &changed);
    let removal = plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Remove,
    )
    .unwrap();
    assert_eq!(
        write_content(&removal, "AGENTS.md"),
        "New local constraint\nOriginal local facts\nLocal tail\n"
    );
    seed(&root, &removal);
    assert_eq!(
        std::fs::read_to_string(root.join("unrelated.md")).unwrap(),
        "Independent project knowledge"
    );
    assert!(!root.join(removal.ownership_path()).exists());
    assert!(plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Remove
    )
    .unwrap()
    .files()
    .iter()
    .all(|file| matches!(file.change(), GuidanceFileChange::Keep)));
}

#[test]
fn unowned_identical_files_and_edited_owned_files_are_collisions() {
    let fixture = fixture();
    let root = root(&fixture);
    let initial = plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Apply,
    )
    .unwrap();
    let file = initial
        .files()
        .iter()
        .find(|file| file.rel_path().ends_with("SKILL.md"))
        .unwrap();
    let path = root.join(file.rel_path());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, write_content(&initial, file.rel_path())).unwrap();
    assert!(matches!(
        plan(
            &fixture,
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Apply
        ),
        Err(GuidancePlanError::Collision { .. })
    ));
    std::fs::remove_file(&path).unwrap();
    seed(&root, &initial);
    std::fs::write(path, "Edited generated advice").unwrap();
    assert!(matches!(
        plan(
            &fixture,
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Apply
        ),
        Err(GuidancePlanError::Collision { .. })
    ));
}

#[test]
fn ownership_cannot_cross_hosts_or_claim_project_files_and_entry_edits_are_not_erased() {
    let fixture = fixture();
    let root = root(&fixture);
    let initial = plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Apply,
    )
    .unwrap();
    seed(&root, &initial);
    let state_path = root.join(initial.ownership_path());
    let original = std::fs::read(&state_path).unwrap();
    let other = root.join(ownership_path(SourceGuidanceHost::ClaudeCode));
    std::fs::write(&other, &original).unwrap();
    assert!(matches!(
        plan(
            &fixture,
            SourceGuidanceHost::ClaudeCode,
            GuidancePlanIntent::Apply
        ),
        Err(GuidancePlanError::Ownership(_))
    ));
    std::fs::remove_file(other).unwrap();
    let mut metadata: serde_json::Value = serde_json::from_slice(&original).unwrap();
    metadata["files"][0]["path"] = "README.md".into();
    std::fs::write(&state_path, serde_json::to_vec(&metadata).unwrap()).unwrap();
    assert!(matches!(
        plan(
            &fixture,
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Remove
        ),
        Err(GuidancePlanError::Ownership(_))
    ));
    std::fs::write(state_path, original).unwrap();
    let entry = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
    let begin = "<!-- pale:source-guidance codex begin -->\n";
    assert!(entry.contains(begin));
    std::fs::write(
        root.join("AGENTS.md"),
        entry.replacen(begin, &format!("{begin}Edited generated advice.\n"), 1),
    )
    .unwrap();
    assert!(matches!(
        plan(
            &fixture,
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Remove
        ),
        Err(GuidancePlanError::Collision { .. })
    ));
}

#[test]
fn readonly_root_mixing_and_runtime_blocks_do_not_construct_guidance_plans() {
    let fixture = fixture();
    let root = root(&fixture);
    std::fs::write(
        root.join("AGENTS.md"),
        format!(
            "{}\nRuntime context\n{}",
            pale_core::MANAGED_BEGIN,
            pale_core::MANAGED_END
        ),
    )
    .unwrap();
    assert!(matches!(
        plan(
            &fixture,
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Apply
        ),
        Err(GuidancePlanError::Collision { .. })
    ));
    std::fs::remove_file(root.join("AGENTS.md")).unwrap();
    let compiled = effective(&fixture);
    for tamper in ["root", "parents", "presence"] {
        let prepared = prepare_source_guidance(
            &compiled,
            &LogicalRepositoryIdV1::parse("engine").unwrap(),
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Apply,
            observe(&root, &ownership_path(SourceGuidanceHost::Codex)),
        )
        .unwrap();
        let mut targets: BTreeMap<_, _> = prepared
            .required_paths()
            .into_iter()
            .map(|path| {
                let observation = observe(&root, &path);
                (path, observation)
            })
            .collect();
        let observed = targets.values_mut().next().unwrap();
        match tamper {
            "root" => observed.root.inode = pale_core::records::FilesystemInodeIdV1::new(0),
            "parents" => {
                assert!(observed.missing_parent_directories.pop().is_some());
            }
            "presence" => {
                observed.bytes = Some(Vec::new());
            }
            _ => unreachable!(),
        }
        assert!(matches!(
            prepared.finish(targets),
            Err(GuidancePlanError::Baseline(_))
        ));
    }
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&fixture.manifest).unwrap()).unwrap();
    manifest["workspace"]["desired_topology"]["execution_workspace_repositories"][0]["access"] =
        "read".into();
    std::fs::write(&fixture.manifest, toml::to_string(&manifest).unwrap()).unwrap();
    assert!(matches!(
        plan(
            &fixture,
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Apply
        ),
        Err(GuidancePlanError::ReadOnly)
    ));
}

#[test]
fn reviewed_identity_changes_with_native_baselines_and_observation_inventories_are_exact() {
    let fixture = fixture();
    let root = root(&fixture);
    std::fs::write(root.join("AGENTS.md"), "Local rule\n").unwrap();
    let first = plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Apply,
    )
    .unwrap();
    std::fs::write(root.join("AGENTS.md"), "Changed local rule\n").unwrap();
    let second = plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Apply,
    )
    .unwrap();
    assert_ne!(
        first.digest(),
        second.digest(),
        "a later writer must not admit the earlier reviewed identity"
    );
    let compiled = effective(&fixture);
    for extra in [false, true] {
        let prepared = prepare_source_guidance(
            &compiled,
            &LogicalRepositoryIdV1::parse("engine").unwrap(),
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Apply,
            observe(&root, &ownership_path(SourceGuidanceHost::Codex)),
        )
        .unwrap();
        let mut targets: BTreeMap<_, _> = prepared
            .required_paths()
            .into_iter()
            .map(|path| {
                let observation = observe(&root, &path);
                (path, observation)
            })
            .collect();
        if extra {
            targets.insert("README.md".into(), observe(&root, "README.md"));
        } else {
            targets.remove("AGENTS.md");
        }
        assert!(matches!(
            prepared.finish(targets),
            Err(GuidancePlanError::Baseline(_))
        ));
    }
}

#[test]
fn newly_created_entry_is_deleted_only_when_it_contains_no_new_project_facts() {
    for local_tail in ["", "New native project constraint\n"] {
        let fixture = fixture();
        let root = root(&fixture);
        let initial = plan(
            &fixture,
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Apply,
        )
        .unwrap();
        seed(&root, &initial);
        if !local_tail.is_empty() {
            let entry = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
            std::fs::write(root.join("AGENTS.md"), format!("{entry}{local_tail}")).unwrap();
        }
        let removal = plan(
            &fixture,
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Remove,
        )
        .unwrap();
        let entry = removal
            .files()
            .iter()
            .find(|file| file.rel_path() == "AGENTS.md")
            .unwrap();
        if local_tail.is_empty() {
            assert!(matches!(entry.change(), GuidanceFileChange::Delete));
        } else {
            assert_eq!(write_content(&removal, "AGENTS.md"), local_tail);
        }
    }
}

#[test]
fn retiring_owned_guidance_does_not_require_projecting_a_new_runtime_operation() {
    let fixture = fixture();
    let root = root(&fixture);
    let initial = plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Apply,
    )
    .unwrap();
    seed(&root, &initial);
    let path = fixture.root.path().join("workspace/capabilities.toml");
    let mut capabilities: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    capabilities["semantic"]["operations"] = toml::from_str::<toml::Value>("items = [{ selection = 'exact', resource = { package_id = 'pale.engineering', resource_id = 'testing/repository' } }]").unwrap()["items"].clone();
    capabilities["operation_bindings"] = toml::from_str::<toml::Value>("items = [{ operation = { package_id = 'pale.engineering', resource_id = 'testing/repository' }, binding = { selection = { kind = 'user_only', aliases = [] } } }]").unwrap()["items"].clone();
    std::fs::write(path, toml::to_string(&capabilities).unwrap()).unwrap();
    assert!(matches!(
        plan(
            &fixture,
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Apply
        ),
        Err(GuidancePlanError::Projection(_))
    ));
    let removal = plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Remove,
    )
    .unwrap();
    assert!(removal
        .files()
        .iter()
        .any(|file| matches!(file.change(), GuidanceFileChange::Delete)));
}

#[test]
fn malformed_and_oversized_private_state_are_rejected_before_target_planning() {
    let fixture = fixture();
    let root = root(&fixture);
    let initial = plan(
        &fixture,
        SourceGuidanceHost::Codex,
        GuidancePlanIntent::Apply,
    )
    .unwrap();
    seed(&root, &initial);
    let path = root.join(initial.ownership_path());
    std::fs::write(&path, "{\"version\":1,\"version\":1}").unwrap();
    assert!(matches!(
        plan(
            &fixture,
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Apply
        ),
        Err(GuidancePlanError::Decode(_))
    ));
    std::fs::write(
        path,
        vec![b' '; pale_application::plan::source_guidance::MAX_OWNERSHIP_BYTES + 1],
    )
    .unwrap();
    assert!(matches!(
        plan(
            &fixture,
            SourceGuidanceHost::Codex,
            GuidancePlanIntent::Remove
        ),
        Err(GuidancePlanError::Limit("ownership bytes"))
    ));
}
