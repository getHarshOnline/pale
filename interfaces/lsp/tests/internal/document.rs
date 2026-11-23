//! LSP document-store behavior tests.

use super::*;

fn snapshot_bytes(snapshot: &Snapshot) -> usize {
    snapshot
        .documents
        .iter()
        .map(|document| match &document.text {
            SnapshotText::Contiguous(text) => text.len(),
            SnapshotText::Rope(text) => text.len_bytes(),
        })
        .sum()
}

fn uri(value: &str) -> Uri {
    value.parse().expect("test URI is valid")
}

fn baseline(uri_value: &str, text: &str) -> BaselineDocument {
    BaselineDocument {
        uri: uri(uri_value),
        language_id: "pale-toml".to_owned(),
        role: DocumentRole::WorkspaceManifest,
        text: text.to_owned(),
    }
}

fn baseline_with_role(index: usize, role: DocumentRole, text: String) -> BaselineDocument {
    BaselineDocument {
        uri: uri(&format!("file:///repo/source-{index}")),
        language_id: if matches!(
            role,
            DocumentRole::LifecycleGuidance | DocumentRole::PackageText
        ) {
            "pale"
        } else {
            "pale-toml"
        }
        .to_owned(),
        role,
        text,
    }
}

fn workspace_store() -> DocumentStore {
    DocumentStore::new(
        &uri("file:///repo"),
        vec![baseline("file:///repo/pale-workspace.toml", "registered")],
    )
    .unwrap()
}

#[test]
fn initialization_rejects_each_application_owned_ingress_limit() {
    let workspace = PaleApi::authoring_ingress_limits(AuthoringIngressClass::Workspace);
    let oversized = "x".repeat(workspace.max_document_bytes() as usize + 1);
    assert!(matches!(
        DocumentStore::new(
            &uri("file:///repo"),
            vec![baseline("file:///repo/pale-workspace.toml", &oversized)],
        ),
        Err(DocumentError::DocumentBytesExceeded {
            class: "workspace",
            observed,
            maximum,
        }) if observed == workspace.max_document_bytes() + 1
            && maximum == workspace.max_document_bytes()
    ));

    let private = PaleApi::authoring_ingress_limits(AuthoringIngressClass::Private);
    let too_many = (0..=private.max_documents())
        .map(|index| baseline_with_role(index, DocumentRole::LifecycleGuidance, String::new()))
        .collect();
    assert!(matches!(
        DocumentStore::new(&uri("file:///repo"), too_many),
        Err(DocumentError::DocumentCountExceeded {
            class: "private",
            observed,
            maximum,
        }) if observed == private.max_documents() + 1 && maximum == private.max_documents()
    ));

    let full_documents = private.max_total_bytes() / private.max_document_bytes();
    let remainder = private.max_total_bytes() % private.max_document_bytes();
    let mut aggregate = (0..full_documents as usize)
        .map(|index| {
            baseline_with_role(
                index,
                DocumentRole::LifecycleGuidance,
                "x".repeat(private.max_document_bytes() as usize),
            )
        })
        .collect::<Vec<_>>();
    aggregate.push(baseline_with_role(
        aggregate.len(),
        DocumentRole::LifecycleGuidance,
        "x".repeat(remainder as usize + 1),
    ));
    assert!(matches!(
        DocumentStore::new(&uri("file:///repo"), aggregate),
        Err(DocumentError::TotalBytesExceeded {
            class: "private",
            observed,
            maximum,
        }) if observed == private.max_total_bytes() + 1
            && maximum == private.max_total_bytes()
    ));
}

#[test]
fn oversized_open_is_rejected_without_creating_or_advancing_an_overlay() {
    let document_uri = uri("file:///repo/pale-workspace.toml");
    let mut store = workspace_store();
    let before = store.snapshot();
    let revision = store.revision();
    let limit = PaleApi::authoring_ingress_limits(AuthoringIngressClass::Workspace);

    assert!(matches!(
        store.open(
            document_uri,
            "pale-toml".to_owned(),
            1,
            "x".repeat(limit.max_document_bytes() as usize + 1),
        ),
        Err(DocumentError::DocumentBytesExceeded {
            class: "workspace",
            observed,
            maximum,
        }) if observed == limit.max_document_bytes() + 1
            && maximum == limit.max_document_bytes()
    ));
    assert_eq!(store.snapshot(), before);
    assert_eq!(store.revision(), revision);
}

#[test]
fn oversized_change_batch_is_atomic_after_valid_candidate_growth() {
    let document_uri = uri("file:///repo/pale-workspace.toml");
    let mut store = workspace_store();
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            1,
            "a".to_owned(),
        )
        .expect("bounded overlay");
    let before = store.snapshot();
    let revision = store.revision();
    let metrics = store.last_change_metrics;
    let limit = PaleApi::authoring_ingress_limits(AuthoringIngressClass::Workspace);
    let maximum = limit.max_document_bytes() as usize;

    assert!(matches!(
        store.change(
            &document_uri,
            2,
            vec![
                TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: "x".repeat(maximum),
                },
                TextDocumentContentChangeEvent {
                    range: Some(Range::new(
                        Position::new(0, maximum as u32),
                        Position::new(0, maximum as u32),
                    )),
                    range_length: Some(0),
                    text: "y".to_owned(),
                },
            ],
        ),
        Err(DocumentError::DocumentBytesExceeded {
            class: "workspace",
            observed,
            maximum,
        }) if observed == limit.max_document_bytes() + 1
            && maximum == limit.max_document_bytes()
    ));
    assert_eq!(store.snapshot(), before);
    assert_eq!(store.revision(), revision);
    assert_eq!(store.last_change_metrics, metrics);
}

#[test]
fn baseline_and_overlay_form_a_right_biased_union() {
    let root = uri("file:///repo");
    let document_uri = uri("file:///repo/pale-workspace.toml");
    let mut store = DocumentStore::new(
        &root,
        vec![baseline("file:///repo/pale-workspace.toml", "baseline")],
    )
    .unwrap();
    let immutable_baseline = store.baseline_snapshot();
    assert_eq!(store.snapshot().documents()[0].text(), "baseline");

    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            1,
            "overlay".to_owned(),
        )
        .unwrap();
    assert_eq!(store.snapshot().documents()[0].text(), "overlay");
    assert_eq!(store.snapshot().documents()[0].version(), Some(1));
    assert_eq!(immutable_baseline.documents()[0].text(), "baseline");
    assert_eq!(store.baseline_snapshot(), immutable_baseline);

    store.close(&document_uri).unwrap();
    assert_eq!(store.snapshot().documents()[0].text(), "baseline");
    assert_eq!(store.snapshot().documents()[0].version(), None);
}

#[test]
fn save_advances_only_the_saved_baseline_and_close_reveals_it() {
    let document_uri = uri("file:///repo/pale-workspace.toml");
    let mut store = workspace_store();
    let comparison_anchor = store.baseline_snapshot();
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            1,
            "saved".to_owned(),
        )
        .unwrap();
    let open_revision = store.revision();

    store.save(&document_uri, None).unwrap();
    assert_eq!(store.revision(), open_revision);
    assert_eq!(store.baseline_snapshot(), comparison_anchor);

    store
        .change(
            &document_uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "unsaved".to_owned(),
            }],
        )
        .unwrap();
    assert_eq!(store.snapshot().documents()[0].text(), "unsaved");

    store.close(&document_uri).unwrap();
    assert_eq!(store.snapshot().documents()[0].text(), "saved");
    assert_eq!(store.snapshot().documents()[0].version(), None);
    assert_eq!(store.baseline_snapshot(), comparison_anchor);
}

#[test]
fn watched_closed_source_replaces_saved_bytes_and_fingerprints_duplicates() {
    let document_uri = uri("file:///repo/pale-workspace.toml");
    let mut store = workspace_store();
    let stale = store.snapshot();

    assert!(store
        .replace_saved(&document_uri, "external".to_owned())
        .unwrap());
    assert_eq!(store.snapshot().documents()[0].text(), "external");
    assert!(!store.is_current(&stale));
    let revision = store.revision();

    assert!(!store
        .replace_saved(&document_uri, "external".to_owned())
        .unwrap());
    assert_eq!(store.revision(), revision);
}

#[test]
fn watched_delete_and_recreate_never_reveal_stale_saved_bytes() {
    let document_uri = uri("file:///repo/pale-workspace.toml");
    let mut store = workspace_store();
    store
        .replace_saved(&document_uri, "external".to_owned())
        .unwrap();

    assert!(store.mark_unavailable(&document_uri).unwrap());
    assert!(store.snapshot().documents().is_empty());
    let missing_revision = store.revision();
    assert!(!store.mark_unavailable(&document_uri).unwrap());
    assert_eq!(store.revision(), missing_revision);

    assert!(store
        .replace_saved(&document_uri, "recreated".to_owned())
        .unwrap());
    assert_eq!(store.snapshot().documents()[0].text(), "recreated");
}

#[test]
fn open_overlay_remains_authoritative_over_watched_replacement_and_delete() {
    let document_uri = uri("file:///repo/pale-workspace.toml");
    let mut store = workspace_store();
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            1,
            "unsaved".to_owned(),
        )
        .unwrap();

    store
        .replace_saved(&document_uri, "external".to_owned())
        .unwrap();
    assert_eq!(store.snapshot().documents()[0].text(), "unsaved");
    store.close(&document_uri).unwrap();
    assert_eq!(store.snapshot().documents()[0].text(), "external");

    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            2,
            "second-unsaved".to_owned(),
        )
        .unwrap();
    store.mark_unavailable(&document_uri).unwrap();
    assert_eq!(store.snapshot().documents()[0].text(), "second-unsaved");
    store.close(&document_uri).unwrap();
    assert!(store.snapshot().documents().is_empty());
}

#[test]
fn mismatched_save_text_is_rejected_atomically() {
    let document_uri = uri("file:///repo/pale-workspace.toml");
    let mut store = workspace_store();
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            1,
            "overlay".to_owned(),
        )
        .unwrap();
    let before = store.snapshot();

    assert_eq!(
        store.save(&document_uri, Some("different")),
        Err(DocumentError::SaveTextMismatch)
    );
    assert_eq!(store.snapshot(), before);
    store.close(&document_uri).unwrap();
    assert_eq!(store.snapshot().documents()[0].text(), "registered");
}

#[test]
fn duplicate_open_is_monotone() {
    let document_uri = uri("file:///repo/pale-workspace.toml");
    let mut store = workspace_store();
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            4,
            "four".to_owned(),
        )
        .unwrap();
    assert!(matches!(
        store.open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            4,
            "rollback".to_owned()
        ),
        Err(DocumentError::StaleVersion { .. })
    ));
    assert_eq!(store.snapshot().documents()[0].text(), "four");
    store
        .open(document_uri, "pale-toml".to_owned(), 5, "five".to_owned())
        .unwrap();
    assert_eq!(store.snapshot().documents()[0].text(), "five");
}

#[test]
fn encoded_uri_alias_cannot_replace_another_baseline_owner() {
    let mut store = DocumentStore::new(
        &uri("file:///repo"),
        vec![baseline("file:///repo/config/a.toml", "baseline")],
    )
    .unwrap();
    assert!(matches!(
        store.open(
            uri("file:///repo/config/a%2etoml"),
            "pale-toml".to_owned(),
            1,
            "alias".to_owned()
        ),
        Err(DocumentError::DuplicateLogicalLabel(label)) if label == "config/a.toml"
    ));
}

#[test]
fn downstream_source_languages_are_rejected() {
    let mut store = workspace_store();
    for (path, language) in [
        ("src/lib.rs", "rust"),
        ("main.go", "go"),
        ("src/index.ts", "typescript"),
        ("src/main.py", "python"),
        ("Cargo.toml", "toml"),
        ("config/arbitrary.toml", "toml"),
    ] {
        assert!(matches!(
            store.open(
                uri(&format!("file:///repo/{path}")),
                language.to_owned(),
                1,
                String::new()
            ),
            Err(DocumentError::UnsupportedLanguage(rejected)) if rejected == language
        ));
    }
}

#[test]
fn open_buffers_preserve_explicit_roles_and_reject_unregistered_sources() {
    let root = uri("file:///repo");
    let package_uri = uri("file:///repo/packages/review.md");
    let mut store = DocumentStore::new(
        &root,
        vec![BaselineDocument {
            uri: package_uri.clone(),
            language_id: "pale".to_owned(),
            role: DocumentRole::PackageText,
            text: "baseline".to_owned(),
        }],
    )
    .unwrap();

    store
        .open(package_uri, "pale".to_owned(), 1, "overlay".to_owned())
        .unwrap();
    assert_eq!(
        store.snapshot().documents()[0].role(),
        DocumentRole::PackageText
    );
    assert_eq!(
        store.open(
            uri("file:///repo/packages/unregistered.md"),
            "pale".to_owned(),
            1,
            String::new(),
        ),
        Err(DocumentError::UnregisteredRole)
    );
}

#[test]
fn registered_document_language_cannot_change_at_open() {
    let root = uri("file:///repo");
    let document_uri = uri("file:///repo/source");
    for (role, language, other) in [
        (DocumentRole::PackageManifest, "pale-toml", "pale"),
        (DocumentRole::PackagePage, "pale-page", "pale-toml"),
        (DocumentRole::PackageData, "pale-toml", "pale-page"),
    ] {
        let mut store = DocumentStore::new(
            &root,
            vec![BaselineDocument {
                uri: document_uri.clone(),
                language_id: language.to_owned(),
                role,
                text: String::new(),
            }],
        )
        .unwrap();
        assert_eq!(
            store.open(document_uri.clone(), other.to_owned(), 1, String::new()),
            Err(DocumentError::LanguageMismatch)
        );
        store
            .open(document_uri.clone(), language.to_owned(), 1, String::new())
            .unwrap();
        assert_eq!(store.snapshot().documents()[0].role(), role);
    }
}

#[test]
fn utf16_changes_and_versions_are_exact() {
    let mut store = workspace_store();
    let document_uri = uri("file:///repo/pale-workspace.toml");
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            1,
            "a😀b\n".to_owned(),
        )
        .unwrap();
    store
        .change(
            &document_uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: Some(Range::new(Position::new(0, 1), Position::new(0, 3))),
                range_length: Some(2),
                text: "x".to_owned(),
            }],
        )
        .unwrap();
    assert_eq!(store.snapshot().documents()[0].text(), "axb\n");
    assert!(matches!(
        store.change(&document_uri, 2, Vec::new()),
        Err(DocumentError::StaleVersion { .. })
    ));
}

#[test]
fn utf16_surrogate_splits_fail_closed_and_crlf_is_one_line_boundary() {
    let mut store = workspace_store();
    let document_uri = uri("file:///repo/pale-workspace.toml");
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            1,
            "a😀\r\nb".to_owned(),
        )
        .unwrap();
    let before = store.snapshot();
    assert_eq!(
        store.change(
            &document_uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: Some(Range::new(Position::new(0, 2), Position::new(0, 3))),
                range_length: Some(1),
                text: "x".to_owned(),
            }],
        ),
        Err(DocumentError::InvalidRange)
    );
    assert_eq!(store.snapshot(), before);

    store
        .change(
            &document_uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: Some(Range::new(Position::new(1, 0), Position::new(1, 1))),
                range_length: Some(1),
                text: "c".to_owned(),
            }],
        )
        .unwrap();
    assert_eq!(store.snapshot().documents()[0].text(), "a😀\r\nc");
}

#[test]
fn utf16_coordinates_round_trip_across_contiguous_and_rope_custody() {
    let root = uri("file:///repo");
    let document_uri = uri("file:///repo/pale-workspace.toml");
    let text = "\u{feff}\"😀\" = \"protected\"\r\nname = \"é\"\r\n";
    let mut store = DocumentStore::new(
        &root,
        vec![baseline("file:///repo/pale-workspace.toml", text)],
    )
    .unwrap();
    let contiguous = store.snapshot();
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            1,
            text.to_owned(),
        )
        .unwrap();
    let rope = store.snapshot();

    for (snapshot, version) in [(&contiguous, None), (&rope, Some(1))] {
        let document = snapshot.document(&document_uri).expect("fixture document");
        assert_eq!(document.version(), version);

        let emoji_start = text.find('😀').expect("astral scalar");
        let emoji_end = emoji_start + '😀'.len_utf8();
        let emoji_range = document
            .range_at_bytes(emoji_start, emoji_end)
            .expect("astral byte range has exact UTF-16 coordinates");
        assert_eq!(
            emoji_range,
            Range::new(Position::new(0, 2), Position::new(0, 4))
        );
        assert_eq!(
            document.byte_at_position(emoji_range.start),
            Some(emoji_start)
        );
        assert_eq!(document.byte_at_position(emoji_range.end), Some(emoji_end));
        assert_eq!(document.byte_at_position(Position::new(0, 3)), None);

        let bom_end = '\u{feff}'.len_utf8();
        assert_eq!(
            document.range_at_bytes(0, bom_end),
            Some(Range::new(Position::new(0, 0), Position::new(0, 1)))
        );
        assert_eq!(
            document.byte_at_position(Position::new(0, 1)),
            Some(bom_end)
        );

        let second_value_start = text.find("\"é\"").expect("second-line value");
        let second_value_end = second_value_start + "\"é\"".len();
        let second_value_range = document
            .range_at_bytes(second_value_start, second_value_end)
            .expect("CRLF-delimited range has exact UTF-16 coordinates");
        assert_eq!(
            second_value_range,
            Range::new(Position::new(1, 7), Position::new(1, 10))
        );
        assert_eq!(
            document.byte_at_position(second_value_range.start),
            Some(second_value_start)
        );
        assert_eq!(
            document.byte_at_position(second_value_range.end),
            Some(second_value_end)
        );
    }
}

#[test]
fn immutable_rope_snapshots_share_structure_and_never_observe_later_edits() {
    let mut store = workspace_store();
    let document_uri = uri("file:///repo/pale-workspace.toml");
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            1,
            "before".to_owned(),
        )
        .unwrap();
    let frozen = store.snapshot();
    assert_eq!(snapshot_bytes(&frozen), "before".len());

    store
        .change(
            &document_uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: Some(Range::new(Position::new(0, 0), Position::new(0, 6))),
                range_length: Some(6),
                text: "after".to_owned(),
            }],
        )
        .unwrap();

    assert_eq!(frozen.documents()[0].text(), "before");
    assert_eq!(store.snapshot().documents()[0].text(), "after");
}

#[test]
fn labels_are_lexically_confined_to_the_initialized_root() {
    let store = DocumentStore::new(&uri("file:///repo"), Vec::new()).unwrap();
    assert_eq!(
        store
            .logical_label(&uri("file:///repo/config/modules/a.toml"))
            .unwrap(),
        "config/modules/a.toml"
    );
    assert!(matches!(
        store.logical_label(&uri("file:///other/a.toml")),
        Err(DocumentError::OutsideRoot)
    ));
    assert!(matches!(
        store.logical_label(&uri("file:///repo/%2e%2e/secret.toml")),
        Err(DocumentError::OutsideRoot)
    ));
    assert_eq!(
        store
            .logical_label(&uri("file:///repo/config/a%20b.toml"))
            .unwrap(),
        "config/a b.toml"
    );
}

#[test]
fn invalid_change_batch_is_atomic() {
    let mut store = workspace_store();
    let document_uri = uri("file:///repo/pale-workspace.toml");
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            1,
            "abc".to_owned(),
        )
        .unwrap();
    let before = store.snapshot();
    assert!(matches!(
        store.change(
            &document_uri,
            2,
            vec![
                TextDocumentContentChangeEvent {
                    range: Some(Range::new(Position::new(0, 0), Position::new(0, 1))),
                    range_length: Some(1),
                    text: "x".to_owned(),
                },
                TextDocumentContentChangeEvent {
                    range: Some(Range::new(Position::new(9, 0), Position::new(9, 1))),
                    range_length: Some(1),
                    text: "y".to_owned(),
                },
            ]
        ),
        Err(DocumentError::InvalidRange)
    ));
    assert_eq!(store.snapshot(), before);
}

#[test]
fn exhausted_revision_never_mutates_the_store() {
    let document_uri = uri("file:///repo/pale-workspace.toml");
    let mut store = workspace_store();
    store.revision = u64::MAX;
    assert!(matches!(
        store.open(
            document_uri,
            "pale-toml".to_owned(),
            1,
            "original".to_owned(),
        ),
        Err(DocumentError::RevisionExhausted)
    ));
    assert_eq!(store.snapshot().documents()[0].text(), "registered");
}

#[path = "document/representation.rs"]
mod representation;
