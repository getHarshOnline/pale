//! Shared instruction ownership and explicit migration of the earlier Claude entry.

use super::*;

#[test]
fn runtime_hosts_seal_the_same_single_repository_entry() {
    let fixture = super::super::super::fixture(&[]);
    let compilation = compile_portable_inputs(
        fixture.capture_inputs().unwrap(),
        &pale_adapter_fs::LocalSourceFilesystem,
    )
    .unwrap();
    let surfaces = &compilation.repositories()[0].render().surfaces;
    let entries = surfaces
        .iter()
        .filter(|surface| surface.kind == "front-door")
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    let entry = entries[0];
    assert_eq!(entry.rel_path, "AGENTS.md");
    assert_eq!(entry.execution_path_consumers.len(), 2);
    assert!(entry.content().contains("direct-claude"));
    assert!(entry.content().contains("direct-codex"));
    let manifests = surfaces
        .iter()
        .filter(|surface| surface.rel_path.ends_with("/projection-manifest.json"))
        .map(|surface| {
            serde_json::from_str::<pale_core::records::HarnessProjectionManifestV1>(
                surface.content(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(manifests.len(), 2);
    assert_eq!(manifests[0].front_door, manifests[1].front_door);
    assert_eq!(manifests[0].front_door.rel_path, entry.rel_path);
}

fn apply(fixture: &SyntheticWorkspace, host: SourceGuidanceHost, intent: GuidancePlanIntent) {
    let proposed = plan(fixture, host, intent).unwrap();
    let reviewed = proposed.digest().clone();
    pale_adapter_fs::prepare_source_guidance_write(proposed.admit(&reviewed).unwrap())
        .unwrap()
        .publish()
        .unwrap()
        .commit()
        .unwrap();
}

#[test]
fn both_hosts_share_project_entry_and_preserve_each_others_contribution() {
    for first in [SourceGuidanceHost::Codex, SourceGuidanceHost::ClaudeCode] {
        let second = if first == SourceGuidanceHost::Codex {
            SourceGuidanceHost::ClaudeCode
        } else {
            SourceGuidanceHost::Codex
        };
        let fixture = fixture();
        let root = root(&fixture);
        let original = "The repository's own facts and acceptance criteria.\n";
        std::fs::write(root.join("AGENTS.md"), original).unwrap();
        apply(&fixture, first, GuidancePlanIntent::Apply);
        apply(&fixture, second, GuidancePlanIntent::Apply);
        let installed = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
        assert_eq!(installed.matches(original).count(), 1);
        assert!(installed.contains("<!-- pale:source-guidance codex begin -->"));
        assert!(installed.contains("<!-- pale:source-guidance claude-code begin -->"));
        assert!(!root.join("CLAUDE.md").exists());
        for host in [first, second] {
            assert!(plan(&fixture, host, GuidancePlanIntent::Apply)
                .unwrap()
                .publication_order()
                .is_empty());
        }
        apply(&fixture, first, GuidancePlanIntent::Remove);
        assert!(plan(&fixture, second, GuidancePlanIntent::Apply)
            .unwrap()
            .publication_order()
            .is_empty());
        apply(&fixture, second, GuidancePlanIntent::Remove);
        assert_eq!(
            std::fs::read_to_string(root.join("AGENTS.md")).unwrap(),
            original
        );
        assert!(!root.join("CLAUDE.md").exists());
    }
}

#[test]
fn earlier_claude_entry_requires_reviewed_removal_and_recovers_before_migration() {
    use pale_adapter_fs::{plan_source_guidance_recovery, SourceGuidanceRecoveryIntent};
    let host = SourceGuidanceHost::ClaudeCode;
    let fixture = fixture();
    let root = root(&fixture);
    let original = "Existing upstream Claude instructions stay with their owner.\n";
    std::fs::write(root.join("AGENTS.md"), original).unwrap();
    seed(
        &root,
        &plan(&fixture, host, GuidancePlanIntent::Apply).unwrap(),
    );
    std::fs::rename(root.join("AGENTS.md"), root.join("CLAUDE.md")).unwrap();
    let state_path = root.join(ownership_path(host));
    let mut state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
    state["version"] = 1.into();
    std::fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    let earlier = std::fs::read(root.join("CLAUDE.md")).unwrap();
    let error = plan(&fixture, host, GuidancePlanIntent::Apply)
        .err()
        .unwrap();
    assert!(error.to_string().contains("--intent remove"));
    assert!(!root.join("AGENTS.md").exists());
    assert_eq!(std::fs::read(root.join("CLAUDE.md")).unwrap(), earlier);

    let removal = plan(&fixture, host, GuidancePlanIntent::Remove).unwrap();
    assert!(removal
        .files()
        .iter()
        .any(|file| file.rel_path() == "CLAUDE.md"));
    assert!(!removal
        .files()
        .iter()
        .any(|file| file.rel_path() == "AGENTS.md"));
    let digest = removal.digest().clone();
    let applied = pale_adapter_fs::prepare_source_guidance_write(removal.admit(&digest).unwrap())
        .unwrap()
        .publish()
        .unwrap();
    let journal = root.join(pale_adapter_fs::SOURCE_GUIDANCE_RECOVERY_PATH);
    std::fs::set_permissions(&journal, std::fs::Permissions::from_mode(0o400)).unwrap();
    assert!(applied.commit().is_err());
    std::fs::set_permissions(&journal, std::fs::Permissions::from_mode(0o600)).unwrap();
    let recovery = plan_source_guidance_recovery(
        ConfinedRoot::open(&root).unwrap(),
        SourceGuidanceRecoveryIntent::RestoreBefore,
    )
    .unwrap()
    .unwrap();
    let admitted = effective(&fixture)
        .admit_source_guidance_recovery(
            &LogicalRepositoryIdV1::parse("engine").unwrap(),
            host,
            recovery.proposal(),
            recovery.digest(),
        )
        .unwrap();
    recovery.execute(admitted).unwrap();
    assert_eq!(std::fs::read(root.join("CLAUDE.md")).unwrap(), earlier);
    assert!(!root.join("AGENTS.md").exists());

    apply(&fixture, host, GuidancePlanIntent::Remove);
    assert_eq!(
        std::fs::read_to_string(root.join("CLAUDE.md")).unwrap(),
        original
    );
    apply(&fixture, host, GuidancePlanIntent::Apply);
    assert!(std::fs::read_to_string(root.join("AGENTS.md"))
        .unwrap()
        .contains("<!-- pale:source-guidance claude-code begin -->"));
    assert_eq!(
        std::fs::read_to_string(root.join("CLAUDE.md")).unwrap(),
        original
    );
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(state_path).unwrap()).unwrap();
    assert_eq!(state["version"], 2);
}
