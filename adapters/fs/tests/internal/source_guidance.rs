//! Native mechanism fixtures. Port construction stands for application admission;
//! the actual compiler-to-adapter journey is tested in pale-application.

use super::*;
use pale_core::records::UnixFileMode;
use pale_ports::{SourceGuidanceEffectFile, SourceGuidanceFileChange};
use std::{fs, os::unix::fs::PermissionsExt};

#[path = "source_guidance_limits.rs"]
mod limits;

fn effect(root: &Path, requested: &[(&str, Option<&str>)]) -> AdmittedSourceGuidanceEffect {
    let confined = ConfinedRoot::open(root).unwrap();
    let files = requested
        .iter()
        .map(|(path, content)| {
            SourceGuidanceEffectFile::from_application_admission(
                (*path).to_owned(),
                observe_machine_local_file(&confined, path, TARGET_LIMIT).unwrap(),
                match content {
                    Some(content) => SourceGuidanceFileChange::Write {
                        content: (*content).to_owned(),
                        mode: UnixFileMode::new(0o640).unwrap(),
                    },
                    None => SourceGuidanceFileChange::Delete,
                },
            )
        })
        .collect();
    AdmittedSourceGuidanceEffect::from_application_admission(
        confined.machine_local_identity().unwrap(),
        ArtifactDigest::hash_bytes(b"native mechanism fixture"),
        ArtifactDigest::hash_bytes(b"native mechanism fixture owner"),
        files,
    )
}

#[test]
fn stale_baseline_is_refused_before_target_or_journal_writes() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.md"), "original").unwrap();
    let admitted = effect(root.path(), &[("a.md", Some("desired"))]);
    fs::write(root.path().join("a.md"), "new local fact").unwrap();
    assert!(prepare_source_guidance_write(admitted).is_err());
    assert_eq!(
        fs::read_to_string(root.path().join("a.md")).unwrap(),
        "new local fact"
    );
    assert!(!root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH).exists());
}

#[test]
fn commit_publishes_exact_files_and_removes_private_recovery_state() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("old.md"), "prior").unwrap();
    let admitted = effect(
        root.path(),
        &[
            ("nested/a.md", Some("alpha")),
            ("nested/deeper/b.md", Some("beta")),
            ("old.md", None),
        ],
    );
    let applied = prepare_source_guidance_write(admitted)
        .unwrap()
        .publish()
        .unwrap();
    let journal = root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH);
    assert_eq!(
        fs::metadata(&journal).unwrap().permissions().mode() & 0o7777,
        0o600
    );
    let bytes = fs::read_to_string(&journal).unwrap();
    assert!(bytes.contains("prior"));
    assert!(!bytes.contains(root.path().canonicalize().unwrap().to_str().unwrap()));
    let observed = applied.commit().unwrap();
    assert_eq!(observed.changed_files, 3);
    assert_eq!(
        fs::read_to_string(root.path().join("nested/a.md")).unwrap(),
        "alpha"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("nested/deeper/b.md")).unwrap(),
        "beta"
    );
    assert!(!root.path().join("old.md").exists());
    assert!(!journal.exists());
    assert!(root.path().join(SOURCE_GUIDANCE_LOCK_PATH).is_file());
}

#[test]
fn rollback_and_unfinished_drop_restore_bytes_modes_and_created_directories() {
    for explicit in [true, false] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a.md"), "original private fact").unwrap();
        fs::set_permissions(root.path().join("a.md"), fs::Permissions::from_mode(0o600)).unwrap();
        let admitted = effect(
            root.path(),
            &[
                ("new/deeper/a.md", Some("generated")),
                ("a.md", Some("revised")),
            ],
        );
        let applied = prepare_source_guidance_write(admitted)
            .unwrap()
            .publish()
            .unwrap();
        if explicit {
            applied.rollback().unwrap();
        } else {
            drop(applied);
        }
        assert_eq!(
            fs::read_to_string(root.path().join("a.md")).unwrap(),
            "original private fact"
        );
        assert_eq!(
            fs::metadata(root.path().join("a.md"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );
        assert!(!root.path().join("new").exists());
        assert!(!root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH).exists());
    }
}

#[test]
fn later_edits_are_preserved_and_pending_recovery_blocks_another_writer() {
    for commit in [false, true] {
        for identical_replacement in [false, true] {
            let root = tempfile::tempdir().unwrap();
            fs::write(root.path().join("a.md"), "original").unwrap();
            let admitted = effect(root.path(), &[("a.md", Some("generated"))]);
            let applied = prepare_source_guidance_write(admitted)
                .unwrap()
                .publish()
                .unwrap();
            let correction = if identical_replacement {
                "generated"
            } else {
                "third party correction"
            };
            let replacement = root.path().join("replacement.md");
            fs::write(&replacement, correction).unwrap();
            fs::set_permissions(&replacement, fs::Permissions::from_mode(0o640)).unwrap();
            fs::rename(replacement, root.path().join("a.md")).unwrap();
            if commit {
                let error = applied.commit().unwrap_err();
                assert_eq!(
                    error.effect_visibility(),
                    crate::EffectVisibility::MayBeVisible
                );
            } else {
                assert!(applied.rollback().is_err());
            }
            assert_eq!(
                fs::read_to_string(root.path().join("a.md")).unwrap(),
                correction
            );
            assert!(root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH).is_file());
            let next = effect(root.path(), &[("a.md", Some("another request"))]);
            assert!(prepare_source_guidance_write(next).is_err());
            assert_eq!(
                fs::read_to_string(root.path().join("a.md")).unwrap(),
                correction
            );
        }
    }
}

#[test]
fn a_later_baseline_collision_restores_only_the_already_published_prefix() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.md"), "alpha prior").unwrap();
    fs::write(root.path().join("b.md"), "beta prior").unwrap();
    let admitted = effect(
        root.path(),
        &[("a.md", Some("alpha after")), ("b.md", Some("beta after"))],
    );
    let mut run = prepare_source_guidance_write(admitted).unwrap().run;
    run.persist_journal().unwrap();
    run.publish_file(0, 0).unwrap();
    fs::write(root.path().join("b.md"), "independent correction").unwrap();
    assert!(run.publish_file(1, 1).is_err());
    run.restore().unwrap();
    run.terminal = true;
    assert_eq!(
        fs::read_to_string(root.path().join("a.md")).unwrap(),
        "alpha prior"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("b.md")).unwrap(),
        "independent correction"
    );
    assert!(!root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH).exists());
}

#[test]
fn journal_replacement_is_preserved_during_commit_failure() {
    let root = tempfile::tempdir().unwrap();
    let admitted = effect(root.path(), &[("a.md", Some("generated"))]);
    let applied = prepare_source_guidance_write(admitted)
        .unwrap()
        .publish()
        .unwrap();
    fs::write(
        root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH),
        "independent recovery note",
    )
    .unwrap();
    assert!(applied.commit().is_err());
    assert_eq!(
        fs::read_to_string(root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH)).unwrap(),
        "independent recovery note"
    );
    assert_eq!(
        fs::read_to_string(root.path().join("a.md")).unwrap(),
        "generated"
    );
}

#[test]
fn publication_failure_reports_not_started_restored_and_uncertain_targets() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.md"), "before").unwrap();
    let prepared =
        prepare_source_guidance_write(effect(root.path(), &[("a.md", Some("after"))])).unwrap();
    fs::write(
        root.path().join("a.md"),
        "independent edit before publication",
    )
    .unwrap();
    assert_eq!(
        prepared.publish_reported().unwrap_err().state(),
        SourceGuidancePublicationFailureState::NotStarted
    );
    for independent_edit in [false, true] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a.md"), "before").unwrap();
        let mut run =
            prepare_source_guidance_write(effect(root.path(), &[("a.md", Some("after"))]))
                .unwrap()
                .run;
        run.persist_journal().unwrap();
        run.publish_file(0, 0).unwrap();
        if independent_edit {
            fs::write(root.path().join("a.md"), "later local edit").unwrap();
        }
        let failed = PreparedSourceGuidanceWrite { run }
            .publish_reported()
            .unwrap_err();
        assert_eq!(
            failed.state(),
            if independent_edit {
                SourceGuidancePublicationFailureState::OutcomeUnknown
            } else {
                SourceGuidancePublicationFailureState::Restored
            }
        );
        assert_eq!(
            fs::read_to_string(root.path().join("a.md")).unwrap(),
            if independent_edit {
                "later local edit"
            } else {
                "before"
            }
        );
    }
}

fn cold(
    root: &Path,
    intent: crate::SourceGuidanceRecoveryIntent,
) -> crate::SourceGuidanceRecoveryPlan {
    crate::plan_source_guidance_recovery(ConfinedRoot::open(root).unwrap(), intent)
        .unwrap()
        .unwrap()
}
fn recovery_grant(
    plan: &crate::SourceGuidanceRecoveryPlan,
) -> pale_ports::AdmittedSourceGuidanceRecovery {
    // Mechanical fixture admission only; application tests prove logical scope.
    pale_ports::AdmittedSourceGuidanceRecovery::from_application_admission(plan.proposal().clone())
}

#[test]
fn process_interruption_retains_a_journal_and_recovers_without_blind_republication() {
    use crate::{
        SourceGuidanceRecoveryEvidence as Evidence, SourceGuidanceRecoveryIntent as Intent,
    };
    use std::process::{Command, Stdio};
    for stage in ["checkpointed", "uncheckpointed", "restoring", "committed"] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a.md"), "alpha prior").unwrap();
        fs::write(root.path().join("b.md"), "beta prior").unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "source_guidance::transaction::tests::interrupted_child",
                "--ignored",
                "--nocapture",
            ])
            .env("PALE_SOURCE_GUIDANCE_FIXTURE_ROOT", root.path())
            .env("PALE_SOURCE_GUIDANCE_FIXTURE_STAGE", stage)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("native interruption fixture timed out");
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        let mut stderr = String::new();
        use std::io::Read;
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert_eq!(status.code(), Some(27), "{stage}: {stderr}");
        let admitted = effect(root.path(), &[("a.md", Some("another request"))]);
        assert!(prepare_source_guidance_write(admitted).is_err());
        let intent = if stage == "committed" {
            Intent::FinalizePublished
        } else {
            Intent::RestoreBefore
        };
        if stage == "committed" {
            assert!(crate::plan_source_guidance_recovery(
                ConfinedRoot::open(root.path()).unwrap(),
                Intent::RestoreBefore
            )
            .is_err());
        }
        let plan = cold(root.path(), intent);
        if stage == "uncheckpointed" {
            assert_eq!(
                plan.files()[0].evidence(),
                Evidence::UncheckpointedAfterImage
            );
        }
        let before_inode = fs::metadata(root.path().join("a.md")).unwrap();
        let outcome = plan
            .execute(recovery_grant_for(root.path(), intent))
            .unwrap();
        assert_eq!(outcome.intent, intent);
        let suffix = if stage == "committed" {
            "after"
        } else {
            "prior"
        };
        assert_eq!(
            fs::read_to_string(root.path().join("a.md")).unwrap(),
            format!("alpha {suffix}")
        );
        assert_eq!(
            fs::read_to_string(root.path().join("b.md")).unwrap(),
            format!("beta {suffix}")
        );
        if stage == "restoring" {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(
                before_inode.ino(),
                fs::metadata(root.path().join("a.md")).unwrap().ino(),
                "matching before-image must not be rewritten"
            );
        }
        assert!(!root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH).exists());
    }
}
fn recovery_grant_for(
    root: &Path,
    intent: crate::SourceGuidanceRecoveryIntent,
) -> pale_ports::AdmittedSourceGuidanceRecovery {
    recovery_grant(&cold(root, intent))
}

#[test]
fn cold_recovery_preserves_local_contents_and_refuses_replaced_after_images() {
    use crate::SourceGuidanceRecoveryIntent as Intent;
    let root = tempfile::tempdir().unwrap();
    let admitted = effect(root.path(), &[("nested/generated.md", Some("generated"))]);
    let mut applied = prepare_source_guidance_write(admitted)
        .unwrap()
        .publish()
        .unwrap();
    applied.run.terminal = true;
    drop(applied);
    fs::write(root.path().join("nested/local.md"), "local fact").unwrap();
    let plan = cold(root.path(), Intent::RestoreBefore);
    let grant = recovery_grant(&plan);
    let outcome = plan.execute(grant).unwrap();
    assert_eq!(outcome.retained_directories, vec!["nested"]);
    assert_eq!(
        fs::read_to_string(root.path().join("nested/local.md")).unwrap(),
        "local fact"
    );
    assert!(!root.path().join("nested/generated.md").exists());
    let admitted = effect(root.path(), &[("nested/generated.md", Some("generated"))]);
    let mut applied = prepare_source_guidance_write(admitted)
        .unwrap()
        .publish()
        .unwrap();
    applied.run.terminal = true;
    drop(applied);
    let replacement = root.path().join("replacement");
    fs::write(&replacement, "generated").unwrap();
    fs::set_permissions(&replacement, fs::Permissions::from_mode(0o640)).unwrap();
    fs::rename(replacement, root.path().join("nested/generated.md")).unwrap();
    assert!(crate::plan_source_guidance_recovery(
        ConfinedRoot::open(root.path()).unwrap(),
        Intent::RestoreBefore
    )
    .is_err());
    assert!(root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH).exists());
}

#[test]
fn changed_recovery_observations_and_malformed_journals_never_authorize_mutation() {
    use crate::SourceGuidanceRecoveryIntent as Intent;
    for change_journal in [false, true] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a.md"), "prior").unwrap();
        let admitted = effect(root.path(), &[("a.md", Some("after"))]);
        let mut applied = prepare_source_guidance_write(admitted)
            .unwrap()
            .publish()
            .unwrap();
        applied.run.terminal = true;
        drop(applied);
        let plan = cold(root.path(), Intent::RestoreBefore);
        let grant = recovery_grant(&plan);
        if change_journal {
            fs::write(
                root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH),
                "{malformed",
            )
            .unwrap();
        } else {
            fs::write(root.path().join("a.md"), "local correction").unwrap();
        }
        assert!(plan.execute(grant).is_err());
        assert_eq!(
            fs::read_to_string(root.path().join("a.md")).unwrap(),
            if change_journal {
                "after"
            } else {
                "local correction"
            }
        );
        assert!(crate::plan_source_guidance_recovery(
            ConfinedRoot::open(root.path()).unwrap(),
            Intent::RestoreBefore
        )
        .is_err());
    }
}

#[test]
fn finalize_requires_the_complete_after_state_and_retains_successful_publication() {
    use crate::SourceGuidanceRecoveryIntent as Intent;
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.md"), "alpha prior").unwrap();
    fs::write(root.path().join("b.md"), "beta prior").unwrap();
    let admitted = effect(
        root.path(),
        &[("a.md", Some("alpha after")), ("b.md", Some("beta after"))],
    );
    let mut run = prepare_source_guidance_write(admitted).unwrap().run;
    run.persist_journal().unwrap();
    run.publish_file(0, 0).unwrap();
    run.terminal = true;
    drop(run);
    assert!(crate::plan_source_guidance_recovery(
        ConfinedRoot::open(root.path()).unwrap(),
        Intent::FinalizePublished
    )
    .is_err());
    let plan = cold(root.path(), Intent::RestoreBefore);
    let grant = recovery_grant(&plan);
    plan.execute(grant).unwrap();
    let admitted = effect(
        root.path(),
        &[("a.md", Some("alpha after")), ("b.md", Some("beta after"))],
    );
    let mut applied = prepare_source_guidance_write(admitted)
        .unwrap()
        .publish()
        .unwrap();
    applied.run.terminal = true;
    drop(applied);
    let plan = cold(root.path(), Intent::FinalizePublished);
    let grant = recovery_grant(&plan);
    assert_eq!(plan.execute(grant).unwrap().changed_files, 0);
    assert_eq!(
        fs::read_to_string(root.path().join("a.md")).unwrap(),
        "alpha after"
    );
    assert!(!root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH).exists());
}

#[test]
fn corrupt_private_journal_claims_are_rejected_without_changing_targets() {
    use crate::SourceGuidanceRecoveryIntent as Intent;
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.md"), "alpha prior").unwrap();
    fs::write(root.path().join("b.md"), "beta prior").unwrap();
    let admitted = effect(
        root.path(),
        &[("a.md", Some("alpha after")), ("b.md", Some("beta after"))],
    );
    let mut applied = prepare_source_guidance_write(admitted)
        .unwrap()
        .publish()
        .unwrap();
    applied.run.terminal = true;
    drop(applied);
    let path = root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH);
    let original: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for case in [
        "version",
        "root",
        "escape",
        "reserved",
        "duplicate",
        "before",
        "post",
        "prefix",
        "directory",
        "unknown",
        "mode",
    ] {
        let mut value = original.clone();
        match case {
            "version" => value["version"] = 2.into(),
            "root" => value["root_sha256"] = "0".repeat(64).into(),
            "escape" => value["files"][0]["path"] = "../outside.md".into(),
            "reserved" => value["files"][0]["path"] = SOURCE_GUIDANCE_LOCK_PATH.into(),
            "duplicate" => value["files"][1]["path"] = "a.md".into(),
            "before" => value["files"][0]["prior"]["content"] = "corrupted original".into(),
            "post" => value["files"][0]["published"]["state"]["sha256"] = "0".repeat(64).into(),
            "prefix" => {
                value["phase"] = "writing".into();
                value["files"][0]["published"] = serde_json::Value::Null;
            }
            "directory" => {
                value["created_directories"] =
                    serde_json::json!([{"rel_path":"unrelated","device":"1","inode":"1"}])
            }
            "unknown" => value["unowned_extension"] = true.into(),
            "mode" => {}
            _ => unreachable!(),
        }
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        fs::set_permissions(
            &path,
            fs::Permissions::from_mode(if case == "mode" { 0o644 } else { 0o600 }),
        )
        .unwrap();
        assert!(
            crate::plan_source_guidance_recovery(
                ConfinedRoot::open(root.path()).unwrap(),
                Intent::RestoreBefore
            )
            .is_err(),
            "{case}"
        );
        assert_eq!(
            fs::read_to_string(root.path().join("a.md")).unwrap(),
            "alpha after"
        );
        assert_eq!(
            fs::read_to_string(root.path().join("b.md")).unwrap(),
            "beta after"
        );
    }
}

#[test]
#[ignore = "invoked only by the bounded native interruption parent"]
fn interrupted_child() {
    use crate::SourceGuidanceRecoveryIntent as Intent;
    let path = std::env::var_os("PALE_SOURCE_GUIDANCE_FIXTURE_ROOT")
        .expect("explicit synthetic fixture root");
    let stage =
        std::env::var("PALE_SOURCE_GUIDANCE_FIXTURE_STAGE").expect("explicit interruption phase");
    let root = Path::new(&path);
    let admitted = effect(
        root,
        &[("a.md", Some("alpha after")), ("b.md", Some("beta after"))],
    );
    let mut run = prepare_source_guidance_write(admitted).unwrap().run;
    match stage.as_str() {
        "checkpointed" => {
            run.persist_journal().unwrap();
            run.publish_file(0, 0).unwrap();
        }
        "uncheckpointed" => {
            run.persist_journal().unwrap();
            run.publish_leaf(0).unwrap();
        }
        "restoring" => {
            run.publish().unwrap();
            run.terminal = true;
            drop(run);
            let plan = cold(root, Intent::RestoreBefore);
            let confined = ConfinedRoot::open(root).unwrap();
            let observed =
                observe_machine_local_file(&confined, SOURCE_GUIDANCE_RECOVERY_PATH, JOURNAL_LIMIT)
                    .unwrap();
            let mut journal = Journal::decode(observed.bytes.as_ref().unwrap(), &confined).unwrap();
            journal.phase = Phase::Restoring;
            crate::source_guidance::journal::persist(&confined, &journal, &mut Some(observed))
                .unwrap();
            crate::source_guidance::recovery::restore_file(&confined, &plan.files()[0]).unwrap();
        }
        "committed" => {
            run.publish().unwrap();
            run.journal.phase = Phase::Committed;
            run.persist_journal().unwrap();
        }
        _ => panic!("unknown interruption phase"),
    }
    std::process::exit(27);
}
