//! Inventory boundaries must survive native encoding and a fresh recovery open.

use super::*;
use pale_ports::{MAX_SOURCE_GUIDANCE_OWNED_FILES, MAX_SOURCE_GUIDANCE_TRANSACTION_FILES};

#[test]
fn initial_and_disjoint_update_inventories_survive_cold_recovery() {
    for update in [false, true] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(".gitignore"), "local rule\n").unwrap();
        let mut paths = vec![
            ".gitignore".to_owned(),
            "AGENTS.md".into(),
            "ownership.json".into(),
        ];
        paths.extend((0..MAX_SOURCE_GUIDANCE_OWNED_FILES).map(|i| format!("new-{i}.md")));
        if update {
            for i in 0..MAX_SOURCE_GUIDANCE_OWNED_FILES {
                let path = format!("old-{i}.md");
                fs::write(root.path().join(&path), "old instruction").unwrap();
                paths.push(path);
            }
        }
        let requested = paths
            .iter()
            .map(|path| {
                (
                    path.as_str(),
                    if path.starts_with("old-") {
                        None
                    } else {
                        Some("proposed\n")
                    },
                )
            })
            .collect::<Vec<_>>();
        let expected = if update {
            MAX_SOURCE_GUIDANCE_TRANSACTION_FILES
        } else {
            MAX_SOURCE_GUIDANCE_OWNED_FILES + 3
        };
        assert_eq!(requested.len(), expected);
        let mut run = prepare_source_guidance_write(effect(root.path(), &requested))
            .unwrap()
            .run;
        let bytes = run.journal.encode(&run.root).unwrap();
        assert_eq!(
            Journal::decode(&bytes, &run.root).unwrap().files.len(),
            expected
        );
        run.persist_journal().unwrap();
        run.publish_file(0, 0).unwrap();
        // Retain durable custody while discarding all in-process recovery state.
        run.terminal = true;
        drop(run);
        let recovery = cold(
            root.path(),
            crate::SourceGuidanceRecoveryIntent::RestoreBefore,
        );
        assert_eq!(recovery.files().len(), expected);
        let grant = recovery_grant(&recovery);
        let result = recovery.execute(grant).unwrap();
        assert_eq!(result.changed_files, 1);
        assert_eq!(
            fs::read_to_string(root.path().join(".gitignore")).unwrap(),
            "local rule\n"
        );
        assert!(!root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH).exists());
        assert!(!root.path().join("new-0.md").exists());
        if update {
            assert_eq!(
                fs::read_to_string(root.path().join("old-4095.md")).unwrap(),
                "old instruction"
            );
        }
    }
}

#[test]
fn over_budget_effect_and_journal_are_refused_before_publication() {
    let root = tempfile::tempdir().unwrap();
    let paths = (0..=MAX_SOURCE_GUIDANCE_TRANSACTION_FILES)
        .map(|i| format!("file-{i}.md"))
        .collect::<Vec<_>>();
    let requested = paths
        .iter()
        .map(|p| (p.as_str(), Some("proposal")))
        .collect::<Vec<_>>();
    assert!(prepare_source_guidance_write(effect(root.path(), &requested)).is_err());
    assert!(!root.path().join(SOURCE_GUIDANCE_RECOVERY_PATH).exists());
    assert!(!root.path().join("file-0.md").exists());

    let confined = ConfinedRoot::open(root.path()).unwrap();
    let admitted = effect(
        root.path(),
        &requested[..MAX_SOURCE_GUIDANCE_TRANSACTION_FILES],
    );
    let mut journal = Journal::from_effect(&admitted, &confined).unwrap();
    journal.files.push(crate::source_guidance::journal::File {
        path: "one-too-many.md".into(),
        prior: None,
        change: crate::source_guidance::journal::Change::Write {
            content: "proposal".into(),
            mode: UnixFileMode::DEFAULT_FILE,
        },
        published: None,
    });
    assert!(journal.encode(&confined).is_err());
    let unchecked = serde_json::to_vec(&journal).unwrap();
    assert!(Journal::decode(&unchecked, &confined).is_err());
}
