//! Public-boundary tests for path-free sovereign workspace compilation.

use std::error::Error as _;

use pale_sdk::{
    ApiError, AuthoringInstancePathSegment, DiagnosticDisposition, DiagnosticEffectCertainty,
    DiagnosticRetry, DiagnosticSeverity, DiagnosticStage, PaleApi, PortableWorkspaceChangeKind,
    PortableWorkspaceDocument, PortableWorkspaceExplanationFrontier,
    PortableWorkspaceExplanationState, PortableWorkspaceFact, PortableWorkspaceImpactState,
    PortableWorkspaceProvenance, PortableWorkspaceRequest, PortableWorkspaceSubject,
    PortableWorkspaceSubjectKind, PortableWorkspaceSubjectSelector, PortableWorkspaceSubjectValues,
    PortableWorkspaceValueComparison, MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES,
};

#[path = "portable_workspace/comparison_subjects.rs"]
mod comparison_subjects;
#[path = "support/portable_workspace_inputs.rs"]
mod inputs;
use inputs::{
    empty_documents, empty_request, unadopted_documents, wide_dependency_request, POPULATED_ALPHA,
    POPULATED_BETA, POPULATED_MODULE, POPULATED_RESOLUTION_LOCK, POPULATED_ROOT,
    POPULATED_UNIVERSE_LOCK, RESOLUTION_LOCK, ROOT, UNIVERSE_LOCK,
};
const CANDIDATE_RESOLUTION_LOCK: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/release-candidate/resolution.lock.toml"
));
const CANDIDATE_UNIVERSE_LOCK: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/release-candidate/universe.lock.toml"
));
const SOURCE_INCOMPLETE_ROOT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/source-incomplete/pale-workspace.toml"
));
const SOURCE_INCOMPLETE_MODULE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/source-incomplete/module.toml"
));
const SOURCE_INCOMPLETE_RESOLUTION_LOCK: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/source-incomplete/resolution.lock.toml"
));
const SOURCE_INCOMPLETE_UNIVERSE_LOCK: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/source-incomplete/universe.lock.toml"
));

fn api() -> PaleApi {
    PaleApi::new().expect("embedded API dependencies are coherent")
}

#[test]
fn comparison_custody_uses_exact_ordered_compilation_handles() {
    let api = api();
    let prior = api.compile_portable_workspace(empty_request()).unwrap();
    let current = api.compile_portable_workspace(empty_request()).unwrap();
    let equal_bytes = api.compile_portable_workspace(empty_request()).unwrap();
    let comparison = api.compare_portable_workspaces(&prior, &current).unwrap();

    assert!(comparison.compares(&prior, &current));
    assert!(!comparison.compares(&current, &prior));
    assert!(!comparison.compares(&prior, &equal_bytes));
    assert_eq!(
        current.semantic_ir_digest(),
        equal_bytes.semantic_ir_digest()
    );
    assert_eq!(
        current.principal_universe_digest(),
        equal_bytes.principal_universe_digest()
    );
}

#[path = "portable_workspace/contributing_roots.rs"]
mod contributing_roots;
#[path = "portable_workspace/direct_dependents.rs"]
mod direct_dependents;
#[path = "portable_workspace/organization.rs"]
mod organization;

fn source_path(last: &str) -> Vec<AuthoringInstancePathSegment> {
    vec![
        AuthoringInstancePathSegment::Field("declarations".into()),
        AuthoringInstancePathSegment::Index(0),
        AuthoringInstancePathSegment::Field(last.into()),
    ]
}

fn subject_page(
    result: pale_sdk::PortableWorkspaceSubjectOccurrences,
) -> (bool, pale_sdk::AuthoringOccurrencePage) {
    match result {
        pale_sdk::PortableWorkspaceSubjectOccurrences::Selected(page) => (true, page),
        pale_sdk::PortableWorkspaceSubjectOccurrences::Unselected(page) => (false, page),
    }
}

#[test]
fn source_subjects_preserve_named_meaning_and_exact_compilation_custody() {
    use pale_sdk::{
        AuthoringNavigationRequest, AuthoringOccurrenceRole, PortableWorkspaceSourceSubject,
    };

    let api = api();
    let workspace = api.compile_portable_workspace(empty_request()).unwrap();
    let rebuilt = api.compile_portable_workspace(empty_request()).unwrap();
    let inspection = workspace
        .source_subject(AuthoringNavigationRequest::new(
            "pale-workspace.toml".into(),
            Vec::new(),
        ))
        .unwrap();
    assert!(inspection.belongs_to(&workspace));
    assert!(!inspection.belongs_to(&rebuilt));
    assert_eq!(workspace.semantic_ir_digest(), rebuilt.semantic_ir_digest());
    let PortableWorkspaceSourceSubject::Selected {
        occurrence,
        subject,
    } = inspection.result()
    else {
        panic!("the captured root source is selected");
    };
    assert_eq!(occurrence.role(), AuthoringOccurrenceRole::Definition);
    assert_eq!(occurrence.location(), inspection.location());
    assert_eq!(
        subject.selector(),
        PortableWorkspaceSubjectSelector::Source {
            source_id: "workspace-manifest".into(),
        }
    );
    let unsupported = workspace
        .source_subject(AuthoringNavigationRequest::new(
            "pale-workspace.toml".into(),
            vec![
                AuthoringInstancePathSegment::Field("workspace".into()),
                AuthoringInstancePathSegment::Field("distribution_lock".into()),
            ],
        ))
        .unwrap();
    assert!(matches!(
        unsupported.result(),
        PortableWorkspaceSourceSubject::NoDenotation { .. }
    ));
    let missing = workspace
        .source_subject(AuthoringNavigationRequest::new(
            "pale-workspace.toml".into(),
            source_path("no_such_field"),
        ))
        .unwrap();
    assert!(matches!(
        missing.result(),
        PortableWorkspaceSourceSubject::NoOccurrence
    ));
    assert_eq!(missing.location().path(), source_path("no_such_field"));
    let too_deep = workspace
        .source_subject(AuthoringNavigationRequest::new(
            "pale-workspace.toml".into(),
            vec![AuthoringInstancePathSegment::Index(0); 65],
        ))
        .unwrap_err();
    assert_eq!(too_deep.diagnostic().code(), "authoring_path_too_deep");
    drop(workspace);
    drop(rebuilt);
    drop(api);
    assert_eq!(inspection.location().source_label(), "pale-workspace.toml");
    assert!(matches!(
        inspection.result(),
        PortableWorkspaceSourceSubject::Selected { .. }
    ));
}

#[test]
fn subject_occurrences_reuse_navigation_rows_but_not_its_cursors() {
    use pale_sdk::{
        AuthoringNavigationRequest, AuthoringOccurrencePageRequest, PortableWorkspaceSourceSubject,
    };

    let api = api();
    let workspace = api.compile_portable_workspace(empty_request()).unwrap();
    let rebuilt = api.compile_portable_workspace(empty_request()).unwrap();
    let location = AuthoringNavigationRequest::new("pale-workspace.toml".into(), Vec::new());
    let identity = workspace
        .navigate_authoring(location.clone())
        .identity()
        .unwrap()
        .clone();
    let inspection = workspace.source_subject(location).unwrap();
    let PortableWorkspaceSourceSubject::Selected { subject, .. } = inspection.result() else {
        panic!()
    };
    let (_, first) = subject_page(workspace.subject_occurrences(subject, None, 1, 1).unwrap());
    assert_eq!(first.inspected(), 1);
    let cursor = first.continuation().unwrap();
    let identity_request = AuthoringOccurrencePageRequest::new(identity.clone(), 1, 1);
    let identity_first = workspace
        .authoring_occurrences(identity_request.clone())
        .unwrap();
    let other = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Source {
        source_id: "universe-lock".into(),
    })
    .unwrap();
    for rejected in [
        rebuilt.subject_occurrences(subject, Some(cursor), 1, 1),
        workspace.subject_occurrences(&other, Some(cursor), 1, 1),
        workspace.subject_occurrences(subject, Some(cursor), 2, 1),
        workspace.subject_occurrences(subject, Some(cursor), 1, 2),
        workspace.subject_occurrences(subject, identity_first.continuation(), 1, 1),
    ] {
        assert_eq!(
            rejected.unwrap_err().diagnostic().code(),
            "authoring_continuation_mismatch"
        );
    }
    assert_eq!(
        workspace
            .authoring_occurrences(identity_request.after(cursor.clone()))
            .unwrap_err()
            .diagnostic()
            .code(),
        "authoring_continuation_mismatch"
    );
    assert!(workspace
        .subject_occurrences(subject, Some(cursor), 1, 1)
        .is_ok());

    let mut expected = Vec::new();
    let request = AuthoringOccurrencePageRequest::new(identity, 256, 256);
    let mut page = workspace.authoring_occurrences(request.clone()).unwrap();
    loop {
        expected.extend_from_slice(page.occurrences());
        let Some(cursor) = page.continuation().cloned() else {
            break;
        };
        page = workspace
            .authoring_occurrences(request.clone().after(cursor))
            .unwrap();
    }
    assert!(!expected.is_empty());
    for (inspected, returned) in [(0, 0), (1, 1), (2, 1), (1, 2), (64, 256), (257, u16::MAX)] {
        let mut actual = Vec::new();
        let mut after = None;
        loop {
            let (selected, page) = subject_page(
                workspace
                    .subject_occurrences(subject, after.as_ref(), inspected, returned)
                    .unwrap(),
            );
            assert!(selected);
            assert!(
                page.inspected()
                    <= if inspected == 0 {
                        64
                    } else {
                        inspected.min(256)
                    }
            );
            assert!(
                page.occurrences().len()
                    <= usize::from(if returned == 0 { 64 } else { returned.min(256) })
            );
            actual.extend_from_slice(page.occurrences());
            after = page.continuation().cloned();
            if after.is_none() {
                break;
            }
        }
        assert_eq!(actual, expected);
    }
    let (_, normalized) = subject_page(workspace.subject_occurrences(subject, None, 0, 1).unwrap());
    let raw_cursor = normalized.continuation().unwrap();
    assert_eq!(
        workspace
            .subject_occurrences(subject, Some(raw_cursor), 64, 1)
            .unwrap_err()
            .diagnostic()
            .code(),
        "authoring_continuation_mismatch"
    );
}

#[test]
fn selected_lock_record_with_no_direct_named_source_is_not_absent() {
    let api = api();
    let workspace = api.compile_portable_workspace(empty_request()).unwrap();
    let comparison = api
        .compare_portable_workspaces(&workspace, &workspace)
        .unwrap();
    let lock_record = all_impact_subjects(&comparison)
        .into_iter()
        .find_map(|(subject, _)| {
            (subject.kind() == PortableWorkspaceSubjectKind::UniverseLock).then_some(subject)
        })
        .expect("real compilation includes the universe-lock record");
    let mut after = None;
    loop {
        let (selected, page) = subject_page(
            workspace
                .subject_occurrences(&lock_record, after.as_ref(), 2, 2)
                .unwrap(),
        );
        assert!(selected);
        assert!(page.occurrences().is_empty());
        after = page.continuation().cloned();
        if after.is_none() {
            break;
        }
    }
    let proof = workspace
        .direct_dependencies(&lock_record, None, 256)
        .unwrap();
    assert!(proof.root().is_some());
    assert!(!proof.dependencies().is_empty());
    let absent = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Module {
        module_id: "not-selected-here".into(),
    })
    .unwrap();
    let (selected, page) = subject_page(
        workspace
            .subject_occurrences(&absent, None, 256, 256)
            .unwrap(),
    );
    assert!(!selected);
    assert!(page.is_complete());
    assert!(page.occurrences().is_empty());
}

#[test]
fn direct_dependencies_page_real_proof_inputs_without_a_comparison() {
    let api = api();
    let workspace = api.compile_portable_workspace(empty_request()).unwrap();
    let subject = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
        workspace_id: workspace.workspace_id().to_owned(),
    })
    .unwrap();
    let reference = workspace.direct_dependencies(&subject, None, 256).unwrap();
    assert!(reference.continuation().is_none());
    assert!(reference.dependencies().len() > 1);
    assert!(reference.copied_string_bytes() <= MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES);
    assert_eq!(reference.root().unwrap().subject(), &subject);
    assert_eq!(
        reference.principal_universe_digest(),
        workspace.principal_universe_digest()
    );
    assert_eq!(
        reference.semantic_ir_digest(),
        workspace.semantic_ir_digest()
    );
    for limit in [0, 1, 2, 64, 256, u16::MAX] {
        let mut collected = Vec::new();
        let mut cursor = None;
        loop {
            let page = workspace
                .direct_dependencies(&subject, cursor.as_ref(), limit)
                .unwrap();
            assert_eq!(page.root(), reference.root());
            collected.extend_from_slice(page.dependencies());
            cursor = page.continuation().cloned();
            if cursor.is_none() {
                break;
            }
        }
        assert_eq!(collected, reference.dependencies());
    }
    let mut pending = reference
        .dependencies()
        .iter()
        .map(|step| step.subject().clone())
        .collect::<Vec<_>>();
    let mut visited = std::collections::BTreeSet::new();
    let mut leaves = std::collections::BTreeSet::new();
    while let Some(subject) = pending.pop() {
        let page = workspace.direct_dependencies(&subject, None, 256).unwrap();
        let root = page
            .root()
            .expect("input subject belongs to this retained proof");
        if !visited.insert(root.node_digest().to_owned()) {
            continue;
        }
        assert!(page.continuation().is_none());
        if page.dependencies().is_empty() {
            let PortableWorkspaceFact::SourceContent { source_path, .. } = root.fact() else {
                panic!("this fixture's semantic leaves must be source facts");
            };
            leaves.insert(source_path.clone());
        }
        pending.extend(
            page.dependencies()
                .iter()
                .map(|step| step.subject().clone()),
        );
    }
    // Lock admission evidence is retained, but is not a semantic-IR input.
    assert_eq!(
        leaves,
        std::collections::BTreeSet::from(["pale-workspace.toml".to_owned()])
    );
    for selector in [
        PortableWorkspaceSubjectSelector::ResolutionLock,
        PortableWorkspaceSubjectSelector::UniverseLock,
    ] {
        let subject = PortableWorkspaceSubject::try_from(selector).unwrap();
        let page = workspace.direct_dependencies(&subject, None, 256).unwrap();
        assert!(page.root().is_some());
        assert!(page.continuation().is_none());
        for input in page.dependencies() {
            if let PortableWorkspaceFact::SourceContent { source_path, .. } = input.fact() {
                leaves.insert(source_path.clone());
            }
        }
    }
    assert_eq!(
        leaves,
        std::collections::BTreeSet::from([
            "pale-workspace.toml".to_owned(),
            "resolution.lock.toml".to_owned(),
            "universe.lock.toml".to_owned(),
        ])
    );
    let absent = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
        workspace_id: "absent-workspace".to_owned(),
    })
    .unwrap();
    let missing = workspace.direct_dependencies(&absent, None, 1).unwrap();
    assert!(missing.root().is_none());
    assert!(missing.dependencies().is_empty());
    assert!(missing.continuation().is_none());
    let wire: serde_json::Value = serde_json::from_slice(reference.canonical_bytes()).unwrap();
    assert_eq!(
        wire["root"]["nodeDigest"],
        reference.root().unwrap().node_digest()
    );
    assert_eq!(
        wire["dependencies"].as_array().unwrap().len(),
        reference.dependencies().len()
    );
    assert_eq!(wire["copiedStringBytes"], reference.copied_string_bytes());
    assert_eq!(wire["isComplete"], true);
    assert_eq!(
        workspace
            .direct_dependencies(&subject, None, 256)
            .unwrap()
            .canonical_bytes(),
        reference.canonical_bytes()
    );
}

#[test]
fn direct_dependency_continuations_bind_owner_full_subject_and_raw_limit() {
    let api = api();
    let workspace = api.compile_portable_workspace(empty_request()).unwrap();
    let other = api.compile_portable_workspace(empty_request()).unwrap();
    let subject = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
        workspace_id: workspace.workspace_id().to_owned(),
    })
    .unwrap();
    let page = workspace.direct_dependencies(&subject, None, 1).unwrap();
    let cursor = page
        .continuation()
        .expect("fixture must have several direct inputs");
    let other_subject = page.dependencies()[0].subject();
    let same_kind =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
            workspace_id: "different-workspace".to_owned(),
        })
        .unwrap();
    for error in [
        other
            .direct_dependencies(&subject, Some(cursor), 1)
            .unwrap_err(),
        workspace
            .direct_dependencies(other_subject, Some(cursor), 1)
            .unwrap_err(),
        workspace
            .direct_dependencies(&same_kind, Some(cursor), 1)
            .unwrap_err(),
        workspace
            .direct_dependencies(&subject, Some(cursor), 2)
            .unwrap_err(),
    ] {
        assert_eq!(
            error.diagnostic().code(),
            "workspace_dependency_continuation_mismatch"
        );
    }
    assert!(workspace
        .direct_dependencies(&subject, Some(cursor), 1)
        .is_ok());
    let copied = page.clone();
    drop(page);
    drop(workspace);
    assert!(copied.root().is_some());
    assert_eq!(copied.dependencies().len(), 1);
    assert!(copied.continuation().is_some());
}

fn empty_request_with_root(root: &[u8]) -> PortableWorkspaceRequest {
    PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_string(),
        vec![
            PortableWorkspaceDocument::new("pale-workspace.toml".to_string(), root.to_vec()),
            PortableWorkspaceDocument::new(
                "resolution.lock.toml".to_string(),
                RESOLUTION_LOCK.to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "universe.lock.toml".to_string(),
                UNIVERSE_LOCK.to_vec(),
            ),
        ],
    )
}

#[test]
fn wide_compiled_dependencies_preserve_defaults_caps_and_raw_limit_fences() {
    let api = api();
    let workspace = api
        .compile_portable_workspace(wide_dependency_request())
        .unwrap();
    let subject =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::ResponsibilityClosure)
            .unwrap();
    for limit in [0, 1, 2, 64, 256, 257, u16::MAX] {
        let first = workspace
            .direct_dependencies(&subject, None, limit)
            .unwrap();
        let row_bound = if limit == 0 { 64 } else { limit.min(256) };
        assert!(!first.dependencies().is_empty());
        assert!(first.dependencies().len() <= usize::from(row_bound));
        assert!(first.copied_string_bytes() <= MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES);
        if limit <= 2 && limit != 0 {
            assert_eq!(first.dependencies().len(), usize::from(limit));
        }
        if limit >= 256 {
            assert!(
                first.dependencies().len() < usize::from(row_bound),
                "the copied-string ceiling, not the row ceiling, must split this fixture"
            );
        }
        let mut all = first.dependencies().to_vec();
        let mut cursor = first.continuation().cloned();
        assert!(cursor.is_some());
        while let Some(next) = cursor {
            let page = workspace
                .direct_dependencies(&subject, Some(&next), limit)
                .unwrap();
            assert!(!page.dependencies().is_empty());
            assert!(page.dependencies().len() <= usize::from(row_bound));
            assert!(page.copied_string_bytes() <= MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES);
            all.extend_from_slice(page.dependencies());
            cursor = page.continuation().cloned();
        }
        assert_eq!(all.len(), 301); // 300 attributable records plus the manifest.
        assert!(all
            .windows(2)
            .all(|pair| pair[0].node_digest() < pair[1].node_digest()));
    }
    for (requested, unlike) in [(0, 64), (257, u16::MAX)] {
        let page = workspace
            .direct_dependencies(&subject, None, requested)
            .unwrap();
        let error = workspace
            .direct_dependencies(&subject, page.continuation(), unlike)
            .unwrap_err();
        assert_eq!(
            error.diagnostic().code(),
            "workspace_dependency_continuation_mismatch"
        );
        assert!(workspace
            .direct_dependencies(&subject, page.continuation(), requested)
            .is_ok());
    }
}

fn assert_composed_subject_pages(
    workspace: &pale_sdk::PortableWorkspaceCompilation,
    subject: &PortableWorkspaceSubject,
    limits: (u16, u16, u16),
) -> (bool, bool) {
    let (input_limit, inspected_limit, returned_limit) = limits;
    let first_reference = workspace.direct_dependencies(subject, None, 256).unwrap();
    let root = first_reference.root().cloned();
    let reference_json: serde_json::Value =
        serde_json::from_slice(first_reference.canonical_bytes()).unwrap();
    let root_json = reference_json["root"].clone();
    let mut expected_input_json = reference_json["dependencies"].as_array().unwrap().clone();
    let mut expected_inputs = first_reference.dependencies().to_vec();
    let mut input_cursor = first_reference.continuation().cloned();
    while let Some(cursor) = input_cursor {
        let page = workspace
            .direct_dependencies(subject, Some(&cursor), 256)
            .unwrap();
        expected_inputs.extend_from_slice(page.dependencies());
        let encoded: serde_json::Value = serde_json::from_slice(page.canonical_bytes()).unwrap();
        expected_input_json.extend(encoded["dependencies"].as_array().unwrap().iter().cloned());
        input_cursor = page.continuation().cloned();
    }
    let mut expected_sources = Vec::new();
    let mut source_cursor = None;
    loop {
        let (selected, page) = subject_page(
            workspace
                .subject_occurrences(subject, source_cursor.as_ref(), 256, 256)
                .unwrap(),
        );
        assert_eq!(selected, root.is_some());
        expected_sources.extend_from_slice(page.occurrences());
        source_cursor = page.continuation().cloned();
        if source_cursor.is_none() {
            break;
        }
    }
    let mut cursor = None;
    let mut inputs = Vec::new();
    let mut sources = Vec::new();
    let mut inputs_done = false;
    let mut sources_done = false;
    let mut inputs_finished_first = false;
    let mut sources_finished_first = false;
    for _ in 0..20_000 {
        let page = workspace
            .inspect_subject(
                subject,
                cursor.as_ref(),
                input_limit,
                inspected_limit,
                returned_limit,
            )
            .unwrap();
        assert!(page.belongs_to(workspace));
        assert_eq!(page.subject(), subject);
        assert_eq!(page.root(), root.as_ref());
        assert_eq!(page.workspace_id(), workspace.workspace_id());
        assert_eq!(
            page.principal_universe_digest(),
            workspace.principal_universe_digest()
        );
        assert_eq!(page.semantic_ir_digest(), workspace.semantic_ir_digest());
        assert_eq!(
            page.admission_evidence_digest(),
            workspace.admission_evidence_digest()
        );
        if inputs_done {
            assert!(page.dependencies().is_empty());
        }
        if sources_done {
            assert!(page.occurrences().is_empty());
            assert_eq!(page.inspected(), 0);
        }
        assert!(
            page.dependencies().len()
                <= usize::from(if input_limit == 0 {
                    64
                } else {
                    input_limit.min(256)
                })
        );
        assert!(
            page.inspected()
                <= if inspected_limit == 0 {
                    64
                } else {
                    inspected_limit.min(256)
                }
        );
        assert!(page.occurrences().len() <= usize::from(page.inspected()));
        assert!(
            page.occurrences().len()
                <= usize::from(if returned_limit == 0 {
                    64
                } else {
                    returned_limit.min(256)
                })
        );
        let encoded: serde_json::Value = serde_json::from_slice(page.canonical_bytes()).unwrap();
        assert_eq!(encoded.as_object().unwrap().len(), 14);
        assert!(encoded.get("root").is_some());
        assert!(encoded.get("recordAnchor").is_some());
        assert_eq!(encoded["subject"], reference_json["subject"]);
        assert_eq!(encoded["root"], root_json);
        assert_eq!(encoded["workspaceId"], page.workspace_id());
        assert_eq!(
            encoded["principalUniverseDigest"],
            page.principal_universe_digest()
        );
        assert_eq!(encoded["semanticIrDigest"], page.semantic_ir_digest());
        assert_eq!(
            encoded["admissionEvidenceDigest"],
            page.admission_evidence_digest()
        );
        assert_eq!(encoded["inputsComplete"], page.inputs_complete());
        assert_eq!(encoded["sourcesComplete"], page.sources_complete());
        assert_eq!(encoded["inspected"], page.inspected());
        assert_eq!(
            encoded["dependencyCopiedStringBytes"],
            page.dependency_copied_string_bytes()
        );
        assert_eq!(
            encoded["sourceCopiedStringBytes"],
            page.source_copied_string_bytes()
        );
        assert_eq!(
            encoded["dependencies"].as_array().unwrap(),
            &expected_input_json[inputs.len()..inputs.len() + page.dependencies().len()]
        );
        assert_eq!(
            encoded["occurrences"],
            serde_json::Value::Array(
                page.occurrences()
                    .iter()
                    .map(|row| serde_json::json!({
                        "location": row.location(), "role": row.role(),
                    }))
                    .collect()
            )
        );
        assert!(page.record_anchor().is_none()); // These actual public fixtures do not admit imported modules.
        assert!(encoded["recordAnchor"].is_null());
        assert_eq!(
            pale_core::canon::to_canonical_string(&encoded)
                .unwrap()
                .as_bytes(),
            page.canonical_bytes()
        );
        assert_eq!(
            format!("{page:?}"),
            "PortableWorkspaceSubjectInspection { .. }"
        );
        inputs.extend_from_slice(page.dependencies());
        sources.extend_from_slice(page.occurrences());
        inputs_done = page.inputs_complete();
        sources_done = page.sources_complete();
        inputs_finished_first |= inputs_done && !sources_done;
        sources_finished_first |= sources_done && !inputs_done;
        assert_eq!(page.is_complete(), page.continuation().is_none());
        cursor = page.continuation().cloned();
        if cursor.is_none() {
            break;
        }
    }
    assert!(inputs_done && sources_done);
    assert_eq!(inputs, expected_inputs);
    assert_eq!(sources, expected_sources);
    (inputs_finished_first, sources_finished_first)
}

#[test]
fn composed_subject_inspection_matches_existing_owners_and_independent_progress() {
    let api = api();
    let empty = api.compile_portable_workspace(empty_request()).unwrap();
    let subject = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
        workspace_id: empty.workspace_id().to_owned(),
    })
    .unwrap();
    assert_composed_subject_pages(&empty, &subject, (1, 1, 1));
    let comparison = api.compare_portable_workspaces(&empty, &empty).unwrap();
    let lock = all_impact_subjects(&comparison)
        .into_iter()
        .find_map(|(subject, _)| {
            (subject.kind() == PortableWorkspaceSubjectKind::UniverseLock).then_some(subject)
        })
        .unwrap();
    let (inputs_first, _) = assert_composed_subject_pages(&empty, &lock, (256, 1, 1));
    assert!(inputs_first);
    let wide = api
        .compile_portable_workspace(wide_dependency_request())
        .unwrap();
    let closure =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::ResponsibilityClosure)
            .unwrap();
    let (_, sources_first) = assert_composed_subject_pages(&wide, &closure, (1, 256, 256));
    assert!(sources_first);
    for limits in [(0, 0, 0), (257, 257, 257), (u16::MAX, u16::MAX, u16::MAX)] {
        assert_composed_subject_pages(&wide, &closure, limits);
    }
}

#[test]
fn composed_inspection_binds_raw_limits_even_after_a_section_is_done() {
    let api = api();
    let workspace = std::sync::Arc::new(
        api.compile_portable_workspace(wide_dependency_request())
            .unwrap(),
    );
    let recompiled = api
        .compile_portable_workspace(wide_dependency_request())
        .unwrap();
    let subject =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::ResponsibilityClosure)
            .unwrap();
    let unlike = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
        workspace_id: workspace.workspace_id().into(),
    })
    .unwrap();
    for original in [(0, 0, 0), (257, 257, 257), (256, 1, 1), (1, 256, 256)] {
        let (inputs, inspected, returned) = original;
        let first = workspace
            .inspect_subject(&subject, None, inputs, inspected, returned)
            .unwrap();
        let cursor = first.continuation().unwrap().clone();
        let different = |value| match value {
            0 => 64,
            257 => u16::MAX,
            _ => value + 1,
        };
        for changed in [
            (different(inputs), inspected, returned),
            (inputs, different(inspected), returned),
            (inputs, inspected, different(returned)),
        ] {
            let error = workspace
                .inspect_subject(&subject, Some(&cursor), changed.0, changed.1, changed.2)
                .unwrap_err();
            assert_eq!(
                error.diagnostic().code(),
                "workspace_subject_inspection_continuation_mismatch"
            );
            assert_eq!(
                error.diagnostic().disposition(),
                DiagnosticDisposition::InvalidInput
            );
        }
        assert!(recompiled
            .inspect_subject(&subject, Some(&cursor), inputs, inspected, returned)
            .is_err());
        assert!(workspace
            .inspect_subject(&unlike, Some(&cursor), inputs, inspected, returned)
            .is_err());
        let alias = std::sync::Arc::clone(&workspace);
        let alias_copy = std::sync::Arc::clone(&alias);
        assert!(alias_copy
            .inspect_subject(&subject, Some(&cursor), inputs, inspected, returned)
            .is_ok());
        assert!(first.belongs_to(&alias_copy));
        assert!(!first.belongs_to(&recompiled));
        assert_eq!(
            format!("{cursor:?}"),
            "PortableWorkspaceSubjectInspectionCursor { .. }"
        );
        let mut cursor = Some(cursor);
        loop {
            let page = alias
                .inspect_subject(&subject, cursor.as_ref(), inputs, inspected, returned)
                .unwrap();
            if page.inputs_complete() != page.sources_complete() {
                let next = page.continuation().unwrap();
                let changed = if page.inputs_complete() {
                    (different(inputs), inspected, returned)
                } else {
                    (inputs, different(inspected), returned)
                };
                assert!(alias
                    .inspect_subject(&subject, Some(next), changed.0, changed.1, changed.2)
                    .is_err());
                break;
            }
            cursor = page.continuation().cloned();
            if cursor.is_none() {
                break;
            }
        }
    }
}

#[test]
fn composed_unselected_source_keeps_exact_owner_when_semantic_digests_match() {
    use pale_sdk::{AuthoringNavigationRequest, PortableWorkspaceSourceSubject};
    let api = api();
    let compile = |label| {
        api.compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".into(),
            unadopted_documents(label),
        ))
        .unwrap()
    };
    let first = compile("alpha.toml");
    let renamed = compile("renamed-alpha.toml");
    assert_eq!(first.semantic_ir_digest(), renamed.semantic_ir_digest());
    let source = first
        .source_subject(AuthoringNavigationRequest::new(
            "alpha.toml".into(),
            source_path("id"),
        ))
        .unwrap();
    let PortableWorkspaceSourceSubject::Unselected { subject, .. } = source.result() else {
        panic!("actual unadopted declaration")
    };
    assert_composed_subject_pages(&first, subject, (1, 1, 1));
    let initial = first.inspect_subject(subject, None, 1, 1, 1).unwrap();
    assert!(initial.root().is_none());
    assert!(initial.inputs_complete());
    assert!(initial.belongs_to(&first));
    assert!(!initial.belongs_to(&renamed));
    assert!(renamed
        .inspect_subject(subject, initial.continuation(), 1, 1, 1)
        .is_err());
    let terminal = first.inspect_subject(subject, None, 256, 256, 256).unwrap();
    assert!(terminal.is_complete());
    assert!(terminal.belongs_to(&first));
    assert!(!terminal.belongs_to(&renamed));
    assert_eq!(
        terminal.occurrences()[0].location().source_label(),
        "alpha.toml"
    );
}

fn nested_empty_documents() -> Vec<PortableWorkspaceDocument> {
    let nested_root = std::str::from_utf8(ROOT)
        .expect("fixture is UTF-8")
        .replace(
            "resolution_lock = \"resolution.lock.toml\"",
            "resolution_lock = \"locks/resolution.lock.toml\"",
        )
        .replace(
            "universe_lock = \"universe.lock.toml\"",
            "universe_lock = \"locks/universe.lock.toml\"",
        )
        .into_bytes();
    vec![
        PortableWorkspaceDocument::new("config/pale-workspace.toml".to_string(), nested_root),
        PortableWorkspaceDocument::new(
            "config/locks/resolution.lock.toml".to_string(),
            RESOLUTION_LOCK.to_vec(),
        ),
        PortableWorkspaceDocument::new(
            "config/locks/universe.lock.toml".to_string(),
            UNIVERSE_LOCK.to_vec(),
        ),
    ]
}

fn all_impact_subjects(
    comparison: &pale_sdk::PortableWorkspaceComparison,
) -> Vec<(PortableWorkspaceSubject, PortableWorkspaceImpactState)> {
    let mut offset = 0;
    let mut subjects = Vec::new();
    loop {
        let page = comparison.impacts(offset, 2);
        subjects.extend(
            page.entries()
                .iter()
                .map(|impact| (impact.subject().clone(), impact.state())),
        );
        let Some(next) = page.next_offset() else {
            break;
        };
        offset = next;
    }
    subjects
}

fn populated_documents() -> Vec<PortableWorkspaceDocument> {
    populated_documents_with_alpha(POPULATED_ALPHA)
}

#[test]
fn unselected_source_mapping_retains_custody_when_semantic_digests_match() {
    use pale_sdk::{
        AuthoringNavigationRequest, AuthoringOccurrenceRole, PortableWorkspaceSourceSubject,
    };

    let api = api();
    let compile = |label| {
        api.compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".into(),
            unadopted_documents(label),
        ))
        .unwrap()
    };
    let prior = compile("alpha.toml");
    let current = compile("renamed-alpha.toml");
    assert_eq!(prior.semantic_ir_digest(), current.semantic_ir_digest());

    let query = |label: &str| AuthoringNavigationRequest::new(label.into(), source_path("id"));
    let inspection = prior.source_subject(query("alpha.toml")).unwrap();
    assert!(inspection.belongs_to(&prior));
    assert!(!inspection.belongs_to(&current));
    let PortableWorkspaceSourceSubject::Unselected {
        occurrence,
        subject,
    } = inspection.result()
    else {
        panic!("a resolved but unadopted source does not establish proof membership");
    };
    assert_eq!(occurrence.role(), AuthoringOccurrenceRole::Definition);
    assert_eq!(occurrence.location().source_label(), "alpha.toml");
    assert_eq!(
        subject.selector(),
        PortableWorkspaceSubjectSelector::Declaration {
            owner_module_id: "candidate".into(),
            namespace_id: "alpha".into(),
            declaration_id: "common".into(),
        }
    );
    assert!(matches!(
        current
            .source_subject(query("alpha.toml"))
            .unwrap()
            .result(),
        PortableWorkspaceSourceSubject::NoOccurrence
    ));
    let renamed = current.source_subject(query("renamed-alpha.toml")).unwrap();
    let PortableWorkspaceSourceSubject::Unselected {
        occurrence: renamed_occurrence,
        subject: renamed_subject,
    } = renamed.result()
    else {
        panic!("the renamed captured definition remains unselected");
    };
    assert_eq!(subject, renamed_subject);
    assert_eq!(
        renamed_occurrence.location().source_label(),
        "renamed-alpha.toml"
    );

    let (_, first) = subject_page(prior.subject_occurrences(subject, None, 1, 1).unwrap());
    let cursor = first
        .continuation()
        .expect("the fixture contains several indexed occurrences");
    assert_eq!(
        current
            .subject_occurrences(subject, Some(cursor), 1, 1)
            .unwrap_err()
            .diagnostic()
            .code(),
        "authoring_continuation_mismatch"
    );
    for (owner, namespace) in [("another-module", "alpha"), ("candidate", "beta")] {
        let other =
            PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Declaration {
                owner_module_id: owner.into(),
                namespace_id: namespace.into(),
                declaration_id: "common".into(),
            })
            .unwrap();
        assert_eq!(
            prior
                .subject_occurrences(&other, Some(cursor), 1, 1)
                .unwrap_err()
                .diagnostic()
                .code(),
            "authoring_continuation_mismatch"
        );
    }
    for (inspected, returned, changed_inspected, changed_returned) in [
        (0, 1, 64, 1),
        (1, 0, 1, 64),
        (257, 1, u16::MAX, 1),
        (1, 257, 1, u16::MAX),
    ] {
        let (_, page) = subject_page(
            prior
                .subject_occurrences(subject, None, inspected, returned)
                .unwrap(),
        );
        let cursor = page
            .continuation()
            .expect("one work or result row keeps a continuation");
        assert_eq!(
            prior
                .subject_occurrences(subject, Some(cursor), changed_inspected, changed_returned)
                .unwrap_err()
                .diagnostic()
                .code(),
            "authoring_continuation_mismatch"
        );
    }
    for (compilation, expected_label) in [(&prior, "alpha.toml"), (&current, "renamed-alpha.toml")]
    {
        let mut after = None;
        let mut definitions = Vec::new();
        loop {
            let (selected, page) = subject_page(
                compilation
                    .subject_occurrences(subject, after.as_ref(), 1, 1)
                    .unwrap(),
            );
            assert!(!selected);
            definitions.extend(
                page.occurrences()
                    .iter()
                    .filter(|occurrence| occurrence.role() == AuthoringOccurrenceRole::Definition)
                    .map(|occurrence| occurrence.location().source_label().to_owned()),
            );
            after = page.continuation().cloned();
            if after.is_none() {
                break;
            }
        }
        assert_eq!(definitions, [expected_label]);
    }
    drop(first);
    drop(prior);
    drop(current);
    drop(api);
    assert_eq!(inspection.location().source_label(), "alpha.toml");
    assert!(matches!(
        inspection.result(),
        PortableWorkspaceSourceSubject::Unselected { .. }
    ));
    assert_eq!(renamed.location().source_label(), "renamed-alpha.toml");
}

fn populated_documents_with_alpha(alpha: &[u8]) -> Vec<PortableWorkspaceDocument> {
    [
        ("pale-workspace.toml", POPULATED_ROOT),
        ("module.toml", POPULATED_MODULE),
        ("alpha.toml", alpha),
        ("beta.toml", POPULATED_BETA),
        ("resolution.lock.toml", POPULATED_RESOLUTION_LOCK),
        ("universe.lock.toml", POPULATED_UNIVERSE_LOCK),
    ]
    .into_iter()
    .map(|(label, bytes)| PortableWorkspaceDocument::new(label.to_string(), bytes.to_vec()))
    .collect()
}

fn candidate_release_documents() -> Vec<PortableWorkspaceDocument> {
    [
        ("pale-workspace.toml", POPULATED_ROOT),
        ("module.toml", POPULATED_MODULE),
        ("alpha.toml", POPULATED_ALPHA),
        ("beta.toml", POPULATED_BETA),
        ("resolution.lock.toml", CANDIDATE_RESOLUTION_LOCK),
        ("universe.lock.toml", CANDIDATE_UNIVERSE_LOCK),
    ]
    .into_iter()
    .map(|(label, bytes)| PortableWorkspaceDocument::new(label.to_string(), bytes.to_vec()))
    .collect()
}

fn source_incomplete_documents() -> Vec<PortableWorkspaceDocument> {
    [
        ("pale-workspace.toml", SOURCE_INCOMPLETE_ROOT),
        ("module.toml", SOURCE_INCOMPLETE_MODULE),
        ("resolution.lock.toml", SOURCE_INCOMPLETE_RESOLUTION_LOCK),
        ("universe.lock.toml", SOURCE_INCOMPLETE_UNIVERSE_LOCK),
    ]
    .into_iter()
    .map(|(label, bytes)| PortableWorkspaceDocument::new(label.to_string(), bytes.to_vec()))
    .collect()
}

fn assert_diagnostic_tuple(
    error: &ApiError,
    code: &str,
    stage: DiagnosticStage,
    disposition: DiagnosticDisposition,
    retry: DiagnosticRetry,
) {
    assert_eq!(error.diagnostic().code(), code);
    assert_eq!(error.diagnostic().severity(), DiagnosticSeverity::Error);
    assert_eq!(error.diagnostic().stage(), stage);
    assert_eq!(error.diagnostic().disposition(), disposition);
    assert_eq!(error.diagnostic().retry(), retry);
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
    assert!(error.source().is_none());
}

fn assert_facts(error: &ApiError, expected: &[(&str, &str)]) {
    let facts = error.diagnostic().facts();
    let actual = facts
        .iter()
        .map(|fact| (fact.key(), fact.value()))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn portable_workspace_compilation_has_deterministic_admitted_observations() {
    let api = api();
    let first = api
        .compile_portable_workspace(empty_request())
        .expect("the empty sovereign workspace needs no publisher proof");
    let second = api
        .compile_portable_workspace(empty_request())
        .expect("the same captured bytes compile identically");

    assert_eq!(first.workspace_id(), "empty-adopter");
    assert_eq!(first.principal_universe_digest().len(), 64);
    assert_eq!(
        first.principal_universe_digest(),
        second.principal_universe_digest()
    );
    assert_eq!(first.semantic_ir_digest(), second.semantic_ir_digest());
    assert_eq!(
        first.admission_evidence_digest(),
        second.admission_evidence_digest()
    );
    assert_eq!(first.source_document_count(), 3);
    assert!(first.derivation_source_count() >= 3);
    assert!(first.derivation_node_count() >= first.derivation_source_count());
    assert_eq!(first.admission_evidence_count(), 0);
    assert_eq!(first.adopted_release_count(), 0);
    assert_eq!(first.package_release_count(), 0);
    assert_eq!(first.normative_pack_release_count(), 0);

    let summary = first.summary();
    assert_eq!(summary.workspace_id(), first.workspace_id());
    assert_eq!(
        summary.principal_universe_digest(),
        first.principal_universe_digest()
    );
    assert_eq!(summary.semantic_ir_digest(), first.semantic_ir_digest());
    assert_eq!(
        summary.admission_evidence_digest(),
        first.admission_evidence_digest()
    );
    assert_eq!(
        summary.source_document_count(),
        first.source_document_count()
    );
    assert_eq!(
        summary.derivation_source_count(),
        first.derivation_source_count()
    );
    assert_eq!(
        summary.derivation_node_count(),
        first.derivation_node_count()
    );
    assert_eq!(
        summary.admission_evidence_count(),
        first.admission_evidence_count()
    );
    assert_eq!(
        summary.adopted_release_count(),
        first.adopted_release_count()
    );
    assert_eq!(
        summary.package_release_count(),
        first.package_release_count()
    );
    assert_eq!(
        summary.normative_pack_release_count(),
        first.normative_pack_release_count()
    );
    let projected: serde_json::Value =
        serde_json::from_slice(summary.canonical_bytes()).expect("summary is canonical JSON");
    assert_eq!(projected["workspace_id"], summary.workspace_id());
    assert_eq!(
        projected["principal_universe_digest"],
        summary.principal_universe_digest()
    );
    assert_eq!(
        projected["derivation_node_count"],
        summary.derivation_node_count()
    );
    assert_eq!(
        pale_core::canon::to_canonical_string(&projected)
            .expect("summary value canonicalizes")
            .as_bytes(),
        summary.canonical_bytes()
    );

    let debug = format!("{first:?}");
    assert!(debug.contains("empty-adopter"));
    assert!(!debug.contains("distribution_lock"));
    assert!(!debug.contains("project_set_root"));
}

#[test]
fn portable_workspace_comparison_projects_the_complete_unchanged_partition() {
    let api = api();
    let prior = api
        .compile_portable_workspace(empty_request())
        .expect("the predecessor admits");
    let current = api
        .compile_portable_workspace(empty_request())
        .expect("the identical current workspace admits");
    let comparison = api
        .compare_portable_workspaces(&prior, &current)
        .expect("two admitted universes produce an exact receipt");

    assert_eq!(comparison.change_root_count(), 0);
    assert_eq!(comparison.affected_count(), 0);
    assert_eq!(
        comparison.unaffected_count(),
        current.derivation_node_count()
    );
    assert!(comparison.change_roots(0, 0).entries().is_empty());
    let default_page = comparison.impacts(0, 0);
    assert!(default_page
        .entries()
        .iter()
        .all(|impact| impact.contributing_root_count() == 0));
    assert_eq!(
        default_page.entries().len(),
        usize::try_from(comparison.unaffected_count())
            .unwrap()
            .min(usize::from(
                pale_sdk::PortableWorkspaceComparison::DEFAULT_PAGE_SIZE
            ))
    );
    let default_page_bytes = default_page.canonical_bytes();
    let default_page_projection: serde_json::Value =
        serde_json::from_slice(default_page_bytes).unwrap();
    assert_eq!(
        default_page_projection["entries"].as_array().unwrap().len(),
        default_page.entries().len()
    );
    assert_eq!(
        default_page_bytes,
        pale_core::canon::to_canonical_string(&default_page_projection)
            .unwrap()
            .into_bytes()
    );
    let exact_end = comparison.impacts(comparison.unaffected_count(), 1);
    assert!(exact_end.entries().is_empty());
    assert!(exact_end.next_offset().is_none());
    let beyond_end = comparison.impacts(comparison.unaffected_count().saturating_add(1), 1);
    assert!(beyond_end.entries().is_empty());
    assert!(beyond_end.next_offset().is_none());
    let subjects = all_impact_subjects(&comparison);
    assert_eq!(
        u32::try_from(subjects.len()).unwrap(),
        comparison.unaffected_count()
    );
    assert!(subjects
        .iter()
        .all(|(_, state)| *state == PortableWorkspaceImpactState::Unaffected));
    for (subject, _) in &subjects {
        assert!(matches!(
            comparison.compare_subject(subject).values(),
            PortableWorkspaceSubjectValues::Shared {
                comparison: PortableWorkspaceValueComparison::Equal,
                ..
            }
        ));
    }
    assert_eq!(
        subjects
            .iter()
            .map(|(subject, _)| subject)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        subjects.len()
    );

    let subject = &subjects[0].0;
    let explanation = comparison
        .explain(subject)
        .expect("an admitted unchanged subject has a total classification");
    assert_eq!(
        explanation.state(),
        PortableWorkspaceExplanationState::Unaffected
    );
    assert_eq!(explanation.subject(), subject);
    assert!(explanation.causal().is_none());
    let projected: serde_json::Value =
        serde_json::from_slice(explanation.canonical_bytes()).unwrap();
    assert_eq!(projected["state"], "unaffected");
    assert!(projected["causal"].is_null());

    let summary = comparison.summary();
    assert_eq!(
        summary.prior_principal_universe_digest(),
        comparison.prior_principal_universe_digest()
    );
    assert_eq!(
        summary.current_principal_universe_digest(),
        comparison.current_principal_universe_digest()
    );
    assert_eq!(
        summary.prior_semantic_ir_digest(),
        comparison.prior_semantic_ir_digest()
    );
    assert_eq!(
        summary.current_semantic_ir_digest(),
        comparison.current_semantic_ir_digest()
    );
    assert_eq!(summary.receipt_digest(), comparison.receipt_digest());
    assert_eq!(summary.change_root_count(), comparison.change_root_count());
    assert_eq!(summary.affected_count(), comparison.affected_count());
    assert_eq!(summary.unaffected_count(), comparison.unaffected_count());
    let projected: serde_json::Value =
        serde_json::from_slice(summary.canonical_bytes()).expect("summary is canonical JSON");
    assert_eq!(projected["receipt_digest"], summary.receipt_digest());
    assert_eq!(projected["unaffected_count"], summary.unaffected_count());
    assert_eq!(
        pale_core::canon::to_canonical_string(&projected)
            .expect("summary value canonicalizes")
            .as_bytes(),
        summary.canonical_bytes()
    );
}

#[test]
fn portable_subject_selectors_validate_and_round_trip_every_closed_coordinate() {
    use PortableWorkspaceSubjectSelector as Selector;

    let selectors = vec![
        Selector::Source {
            source_id: "source.one".to_owned(),
        },
        Selector::Component {
            component_id: "component.one".to_owned(),
        },
        Selector::Module {
            module_id: "module.one".to_owned(),
        },
        Selector::Declaration {
            owner_module_id: "module.one".to_owned(),
            namespace_id: "namespace.one".to_owned(),
            declaration_id: "declaration.one".to_owned(),
        },
        Selector::Import {
            importer_module_id: "module.importer".to_owned(),
            upstream_module_id: "module.upstream".to_owned(),
            upstream_release_id: "release.one".to_owned(),
            export_id: "export.one".to_owned(),
            source_namespace_id: "namespace.source".to_owned(),
            local_namespace_id: "namespace.local".to_owned(),
        },
        Selector::Extension {
            owner_module_id: "module.one".to_owned(),
            extension_namespace_id: "namespace.extension".to_owned(),
            target_namespace_id: "namespace.target".to_owned(),
            target_declaration_id: "declaration.target".to_owned(),
        },
        Selector::Policy {
            owner_module_id: "module.one".to_owned(),
            namespace_id: "namespace.one".to_owned(),
            declaration_id: "policy.one".to_owned(),
        },
        Selector::ResponsibilityClosure,
        Selector::OrganizationalTranslationClosure,
        Selector::ExperienceComposition,
        Selector::Resource {
            owner_module_id: "module.one".to_owned(),
            resource_id: "resource.one".to_owned(),
        },
        Selector::ResolutionLock,
        Selector::UniverseLock,
        Selector::Workspace {
            workspace_id: "workspace.one".to_owned(),
        },
    ];

    for selector in selectors {
        let subject = PortableWorkspaceSubject::try_from(selector.clone())
            .expect("every selector member obeys the portable identity grammar");
        assert_eq!(subject.selector(), selector);
    }

    let error = PortableWorkspaceSubject::try_from(Selector::Workspace {
        workspace_id: String::new(),
    })
    .expect_err("an empty workspace identity is never admitted");
    assert_eq!(error.field(), "workspace_id");
    assert!(!error.to_string().contains("workspace.one"));
}

#[test]
fn portable_workspace_comparison_explains_reachability_not_output_inequality() {
    let api = api();
    let prior = api
        .compile_portable_workspace(empty_request())
        .expect("the predecessor admits");
    let mut comment_only_root = ROOT.to_vec();
    comment_only_root.extend_from_slice(b"\n# causal-source-change-with-equal-semantics\n");
    let current = api
        .compile_portable_workspace(empty_request_with_root(&comment_only_root))
        .expect("a TOML comment does not change admitted workspace meaning");
    assert_eq!(prior.semantic_ir_digest(), current.semantic_ir_digest());

    let comparison = api
        .compare_portable_workspaces(&prior, &current)
        .expect("source custody changes remain explainable with equal semantic output");
    drop(prior);
    drop(current);
    assert_eq!(comparison.change_root_count(), 1);
    let roots = comparison.change_roots(0, u16::MAX);
    let root = &roots.entries()[0];
    assert_eq!(root.kind(), PortableWorkspaceChangeKind::Modified);
    assert_eq!(root.subject().kind(), PortableWorkspaceSubjectKind::Source);
    assert!(matches!(
        root.prior_fact(),
        Some(PortableWorkspaceFact::SourceContent { .. })
    ));
    assert!(matches!(
        root.current_fact(),
        Some(PortableWorkspaceFact::SourceContent { .. })
    ));
    assert_ne!(root.prior_fact(), root.current_fact());
    let source_comparison = comparison.compare_subject(root.subject());
    assert!(matches!(
        source_comparison.values(),
        PortableWorkspaceSubjectValues::Shared {
            prior,
            current,
            comparison: PortableWorkspaceValueComparison::Changed,
        } if Some(prior) == root.prior_fact() && Some(current) == root.current_fact()
    ));
    assert!(roots.next_offset().is_none());
    let root_projection: serde_json::Value =
        serde_json::from_slice(roots.canonical_bytes()).unwrap();
    assert_eq!(root_projection["entries"][0]["kind"], "modified");
    assert_eq!(
        root_projection["entries"][0]["rootDigest"],
        root.root_digest()
    );

    let subjects = all_impact_subjects(&comparison);
    let target = subjects
        .iter()
        .find_map(|(subject, state)| {
            (*state == PortableWorkspaceImpactState::Affected
                && subject.kind() == PortableWorkspaceSubjectKind::Workspace)
                .then_some(subject)
        })
        .expect("the root source reaches the sealed workspace fact");
    let selector = target.selector();
    let value_comparison = comparison
        .compare_subject_selector(selector.clone())
        .expect("the exact subject selector is valid");
    assert!(matches!(
        value_comparison.values(),
        PortableWorkspaceSubjectValues::Shared {
            prior,
            current,
            comparison: PortableWorkspaceValueComparison::Equal,
        } if prior == current
    ));
    assert_eq!(value_comparison, comparison.compare_subject(target));
    assert_eq!(
        value_comparison.receipt_digest(),
        comparison.receipt_digest()
    );
    assert_eq!(value_comparison.subject(), target);
    let wire: serde_json::Value =
        serde_json::from_slice(value_comparison.canonical_bytes()).unwrap();
    assert_eq!(wire["receiptDigest"], comparison.receipt_digest());
    assert_eq!(wire["values"]["membership"], "shared");
    assert_eq!(wire["values"]["comparison"], "equal");
    assert_eq!(wire["values"]["priorFact"], wire["values"]["currentFact"]);
    assert_eq!(wire["values"]["currentFact"]["kind"], "semantic_ir");
    assert_eq!(
        pale_core::canon::to_canonical_string(&wire)
            .unwrap()
            .as_bytes(),
        value_comparison.canonical_bytes()
    );
    let reconstructed = PortableWorkspaceSubject::try_from(selector)
        .expect("a projected subject selector reconstructs without hidden state");
    let first = comparison
        .explain(&reconstructed)
        .expect("the affected current subject has a causal path");
    let second = comparison
        .explain(target)
        .expect("the same query remains deterministic");
    assert_eq!(first, second);
    assert_eq!(first.state(), PortableWorkspaceExplanationState::Affected);
    let causal = first.causal().expect("affected state carries one witness");
    assert_eq!(causal.root().root_digest(), root.root_digest());
    assert_eq!(
        causal.frontier(),
        PortableWorkspaceExplanationFrontier::Current
    );
    assert_eq!(causal.steps().first().unwrap().subject(), root.subject());
    assert_eq!(causal.steps().last().unwrap().subject(), &reconstructed);
    assert!(matches!(
        causal.steps().first().unwrap().provenance(),
        PortableWorkspaceProvenance::Source { .. }
    ));
    assert!(matches!(
        causal.steps().first().unwrap().fact(),
        PortableWorkspaceFact::SourceContent { .. }
    ));
    assert!(matches!(
        causal.steps().last().unwrap().provenance(),
        PortableWorkspaceProvenance::Compilation
    ));
    assert!(matches!(
        causal.steps().last().unwrap().fact(),
        PortableWorkspaceFact::SemanticIr { .. }
    ));
    let explanation_projection: serde_json::Value =
        serde_json::from_slice(first.canonical_bytes()).unwrap();
    assert_eq!(explanation_projection["state"], "affected");
    assert_eq!(
        explanation_projection["causal"]["steps"]
            .as_array()
            .unwrap()
            .len(),
        causal.steps().len()
    );
}

#[test]
fn portable_subject_comparison_preserves_absence_and_source_membership_in_both_directions() {
    use pale_core::records::{
        ExperienceCompositionDeclarationsV1, SchemaId, WorkspaceCompositionSourceV1,
        WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA, WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA_VERSION,
    };
    let api = api();
    let prior = api.compile_portable_workspace(empty_request()).unwrap();
    let composition = WorkspaceCompositionSourceV1 {
        schema: SchemaId::parse(WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA).unwrap(),
        schema_version: WORKSPACE_COMPOSITION_SOURCE_V1_SCHEMA_VERSION,
        declarations: ExperienceCompositionDeclarationsV1::empty(),
    };
    let root = std::str::from_utf8(ROOT).unwrap().replace(
        "experience_compositions = []",
        "experience_compositions = [\"composition.toml\"]",
    );
    let mut documents = empty_documents();
    documents[0] = PortableWorkspaceDocument::new("pale-workspace.toml".into(), root.into_bytes());
    documents.push(PortableWorkspaceDocument::new(
        "composition.toml".into(),
        toml::to_string(&composition).unwrap().into_bytes(),
    ));
    let current = api
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".into(),
            documents,
        ))
        .expect("explicit empty composition adds source custody without a provider");
    let forward = api.compare_portable_workspaces(&prior, &current).unwrap();
    let reverse = api.compare_portable_workspaces(&current, &prior).unwrap();
    drop(prior);
    drop(current);

    let source = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Source {
        source_id: "experience-composition".into(),
    })
    .unwrap();
    let added = forward.compare_subject(&source);
    let removed = reverse.compare_subject(&source);
    let (
        PortableWorkspaceSubjectValues::CurrentOnly { current },
        PortableWorkspaceSubjectValues::PriorOnly { prior },
    ) = (added.values(), removed.values())
    else {
        panic!("reversing the admitted pair reverses membership");
    };
    assert_eq!(prior, current);
    assert!(
        matches!(current, PortableWorkspaceFact::SourceContent { source_path, .. }
        if source_path == "composition.toml")
    );
    for (result, membership, present, absent) in [
        (&added, "current_only", "currentFact", "priorFact"),
        (&removed, "prior_only", "priorFact", "currentFact"),
    ] {
        let wire: serde_json::Value = serde_json::from_slice(result.canonical_bytes()).unwrap();
        assert_eq!(wire["values"]["membership"], membership);
        assert_eq!(wire["values"][present]["kind"], "source_content");
        assert!(wire["values"].get(absent).is_none());
        assert!(wire["values"].get("comparison").is_none());
    }
    let closure =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::ExperienceComposition)
            .unwrap();
    assert!(matches!(
        forward.compare_subject(&closure).values(),
        PortableWorkspaceSubjectValues::Shared {
            comparison: PortableWorkspaceValueComparison::Equal,
            ..
        }
    ));
    assert_eq!(
        forward.compare_subject(&closure).values(),
        reverse.compare_subject(&closure).values()
    );

    let absent = forward
        .compare_subject_selector(PortableWorkspaceSubjectSelector::Source {
            source_id: "not-a-selected-source".into(),
        })
        .unwrap();
    assert_eq!(absent.values(), &PortableWorkspaceSubjectValues::Absent);
    let wire: serde_json::Value = serde_json::from_slice(absent.canonical_bytes()).unwrap();
    assert_eq!(wire["values"], serde_json::json!({"membership": "absent"}));
    let invalid = forward
        .compare_subject_selector(PortableWorkspaceSubjectSelector::Source {
            source_id: String::new(),
        })
        .unwrap_err();
    assert_eq!(invalid.diagnostic().code(), "workspace_subject_rejected");
}

#[test]
fn portable_subject_comparison_retains_changed_source_labels_with_equal_bytes() {
    let api = api();
    let prior = api.compile_portable_workspace(empty_request()).unwrap();
    let renamed = api
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "renamed.toml".into(),
            vec![
                PortableWorkspaceDocument::new("renamed.toml".into(), ROOT.to_vec()),
                PortableWorkspaceDocument::new(
                    "resolution.lock.toml".into(),
                    RESOLUTION_LOCK.to_vec(),
                ),
                PortableWorkspaceDocument::new("universe.lock.toml".into(), UNIVERSE_LOCK.to_vec()),
            ],
        ))
        .unwrap();
    let comparison = api.compare_portable_workspaces(&prior, &renamed).unwrap();
    let source = comparison
        .compare_subject_selector(PortableWorkspaceSubjectSelector::Source {
            source_id: "workspace-manifest".into(),
        })
        .unwrap();
    let PortableWorkspaceSubjectValues::Shared {
        prior:
            PortableWorkspaceFact::SourceContent {
                source_path: prior_path,
                source_digest: prior_digest,
                byte_len: prior_len,
            },
        current:
            PortableWorkspaceFact::SourceContent {
                source_path: current_path,
                source_digest: current_digest,
                byte_len: current_len,
            },
        comparison: PortableWorkspaceValueComparison::Changed,
    } = source.values()
    else {
        panic!("source label participates in typed output equality");
    };
    assert_eq!(prior_path, "pale-workspace.toml");
    assert_eq!(current_path, "renamed.toml");
    assert_eq!(prior_digest, current_digest);
    assert_eq!(prior_len, current_len);
}

#[test]
fn portable_workspace_rejects_missing_duplicate_and_path_like_documents() {
    let api = api();
    let missing = PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_string(),
        vec![PortableWorkspaceDocument::new(
            "pale-workspace.toml".to_string(),
            ROOT.to_vec(),
        )],
    );
    let error = api
        .compile_portable_workspace(missing)
        .expect_err("manifest-selected locks must be present");
    assert_diagnostic_tuple(
        &error,
        "workspace_source_rejected",
        DiagnosticStage::WorkspaceLoading,
        DiagnosticDisposition::InvalidInput,
        DiagnosticRetry::AfterCorrection,
    );
    assert_eq!(
        error
            .diagnostic()
            .source()
            .expect("missing logical member is identified")
            .label(),
        "resolution.lock.toml"
    );

    let duplicate = PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_string(),
        vec![
            PortableWorkspaceDocument::new("pale-workspace.toml".to_string(), ROOT.to_vec()),
            PortableWorkspaceDocument::new("pale-workspace.toml".to_string(), ROOT.to_vec()),
        ],
    );
    assert_eq!(
        api.compile_portable_workspace(duplicate)
            .expect_err("logical source labels form a unique map")
            .diagnostic()
            .code(),
        "workspace_source_rejected"
    );

    let escaped = PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_string(),
        vec![PortableWorkspaceDocument::new(
            "/private/host/workspace.toml".to_string(),
            ROOT.to_vec(),
        )],
    );
    let error = api
        .compile_portable_workspace(escaped)
        .expect_err("host paths never become portable source labels");
    assert_eq!(error.diagnostic().code(), "workspace_source_rejected");
    assert!(error.diagnostic().source().is_none());
    assert!(!error
        .diagnostic()
        .canonical_json()
        .contains("/private/host"));
    assert!(error.source().is_none());
}

#[test]
fn portable_workspace_rejects_an_unsafe_root_before_document_conversion() {
    let root = "/private/host/root-manifest.toml";
    let deferred_document = "/private/host/deferred-document.toml";
    let error = api()
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            root.to_string(),
            vec![PortableWorkspaceDocument::new(
                deferred_document.to_string(),
                ROOT.to_vec(),
            )],
        ))
        .expect_err("the root gate precedes document conversion");

    assert_diagnostic_tuple(
        &error,
        "workspace_source_rejected",
        DiagnosticStage::WorkspaceLoading,
        DiagnosticDisposition::InvalidInput,
        DiagnosticRetry::AfterCorrection,
    );
    assert!(error.diagnostic().source().is_none());
    let canonical = error.diagnostic().canonical_json();
    assert!(!canonical.contains(root));
    assert!(!canonical.contains(deferred_document));
    let debug = format!("{error:?}");
    assert!(!debug.contains(root));
    assert!(!debug.contains(deferred_document));
}

#[test]
fn portable_workspace_diagnostic_retains_structure_without_source_text() {
    let marker = "credential=never-render-this-source";
    let request = PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_string(),
        vec![PortableWorkspaceDocument::new(
            "pale-workspace.toml".to_string(),
            format!("[workspace\n{marker}").into_bytes(),
        )],
    );
    let error = api()
        .compile_portable_workspace(request)
        .expect_err("malformed TOML fails before semantic admission");

    assert_eq!(error.diagnostic().code(), "workspace_source_rejected");
    let source = error
        .diagnostic()
        .source()
        .expect("the TOML parser provides an honest logical coordinate");
    assert_eq!(source.label(), "pale-workspace.toml");
    assert!(source.line().is_some());
    assert!(!error.diagnostic().canonical_json().contains(marker));
    assert!(!format!("{error:?}").contains(marker));
}

#[test]
fn portable_contract_failure_retains_safe_schema_coordinates() {
    let marker = "never-render-contract-source";
    let invalid_root = format!(
        "{}\n# {marker}\n",
        std::str::from_utf8(ROOT)
            .expect("fixture is UTF-8")
            .replace("schema_version = 1", "schema_version = 2")
    );
    let request = PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_string(),
        vec![
            PortableWorkspaceDocument::new(
                "pale-workspace.toml".to_string(),
                invalid_root.into_bytes(),
            ),
            PortableWorkspaceDocument::new(
                "resolution.lock.toml".to_string(),
                RESOLUTION_LOCK.to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "universe.lock.toml".to_string(),
                UNIVERSE_LOCK.to_vec(),
            ),
        ],
    );
    let error = api()
        .compile_portable_workspace(request)
        .expect_err("the authored wire major is contract-invalid");

    assert_diagnostic_tuple(
        &error,
        "workspace_contract_rejected",
        DiagnosticStage::ContractValidation,
        DiagnosticDisposition::InvalidInput,
        DiagnosticRetry::AfterCorrection,
    );
    let source = error
        .diagnostic()
        .source()
        .expect("the safe logical document is retained");
    assert_eq!(source.label(), "pale-workspace.toml");
    assert_eq!(source.value_path(), Some("/workspace/schema_version"));
    let authoring = error
        .authoring_location()
        .expect("schema admission retains an owner-resolved editor coordinate");
    assert_eq!(authoring.source_label(), "pale-workspace.toml");
    assert_eq!(
        authoring.path(),
        &[
            AuthoringInstancePathSegment::Field("workspace".into()),
            AuthoringInstancePathSegment::Field("schema_version".into()),
        ]
    );
    let schema = error
        .diagnostic()
        .schema_coordinate()
        .expect("contract rejection carries its exact generated schema");
    assert_eq!(schema.registry_id(), "pale-workspace.v1");
    assert_eq!(schema.wire_schema_id(), None);
    assert_eq!(
        schema.schema_sha256(),
        pale_contracts::schema_descriptor("pale-workspace.v1")
            .unwrap()
            .schema_sha256
    );
    assert_eq!(
        schema.schema_path(),
        Some("/$defs/WorkspaceMeta/properties/schema_version/const")
    );
    assert!(!error.diagnostic().canonical_json().contains(marker));
}

#[test]
fn unrepresentable_source_identity_projects_the_original_record_field() {
    use pale_core::records::{
        WorkspaceComponentId, WorkspaceLanguagePrimitivesV1, WorkspaceModuleId, WorkspaceModuleV1,
        WorkspaceResolutionLockV1, WorkspaceSourceComponentRefV1, WorkspaceUniverseLockV1,
    };

    for (family, label, field_path, first_overflow) in [
        ("module", "module.toml", &["module", "module_id"][..], 249),
        ("component", "alpha.toml", &["component_id"][..], 246),
    ] {
        for length in [first_overflow, 255] {
            let logical_id = "x".repeat(length);
            let mut module: WorkspaceModuleV1 =
                toml::from_str(std::str::from_utf8(POPULATED_MODULE).unwrap()).unwrap();
            let mut alpha: WorkspaceLanguagePrimitivesV1 =
                toml::from_str(std::str::from_utf8(POPULATED_ALPHA).unwrap()).unwrap();
            let mut beta: WorkspaceLanguagePrimitivesV1 =
                toml::from_str(std::str::from_utf8(POPULATED_BETA).unwrap()).unwrap();
            if family == "module" {
                let owner = WorkspaceModuleId::parse(logical_id.clone()).unwrap();
                module.module.module_id = owner.clone();
                alpha.module_id = owner.clone();
                beta.module_id = owner.clone();
                for export in &mut module.module.exports {
                    for declaration in &mut export.declarations {
                        declaration.owner_module = owner.clone();
                    }
                }
            } else {
                alpha.component_id = WorkspaceComponentId::parse(logical_id.clone()).unwrap();
            }
            module.module.source_components = [&alpha, &beta]
                .into_iter()
                .map(|component| WorkspaceSourceComponentRefV1 {
                    component_id: component.component_id.clone(),
                    component_digest: component.component_digest().unwrap(),
                })
                .collect();
            let module = WorkspaceModuleV1::seal(module.module).unwrap();
            let mut resolution: WorkspaceResolutionLockV1 =
                toml::from_str(std::str::from_utf8(POPULATED_RESOLUTION_LOCK).unwrap()).unwrap();
            for release in &mut resolution.releases {
                release.module_id = module.module.module_id.clone();
                release.module_digest = module.module_digest.clone();
            }
            let resolution = WorkspaceResolutionLockV1::seal(resolution.releases).unwrap();
            let mut universe: WorkspaceUniverseLockV1 =
                toml::from_str(std::str::from_utf8(POPULATED_UNIVERSE_LOCK).unwrap()).unwrap();
            for release in &mut universe.adopted_releases {
                release.module_id = module.module.module_id.clone();
                release.module_digest = module.module_digest.clone();
            }
            let universe = WorkspaceUniverseLockV1::seal(
                universe.workspace_id,
                resolution.resolution_digest.clone(),
                universe.adopted_releases,
            )
            .unwrap();
            let module = toml::to_string(&module).unwrap();
            let alpha = toml::to_string(&alpha).unwrap();
            let beta = toml::to_string(&beta).unwrap();
            let resolution = toml::to_string(&resolution).unwrap();
            let universe = toml::to_string(&universe).unwrap();
            // Resealing is required before typed module decoding. The source
            // identity fails later; no publisher proof is invented for this
            // negative request.
            let documents = [
                ("pale-workspace.toml", POPULATED_ROOT),
                ("module.toml", module.as_bytes()),
                ("alpha.toml", alpha.as_bytes()),
                ("beta.toml", beta.as_bytes()),
                ("resolution.lock.toml", resolution.as_bytes()),
                ("universe.lock.toml", universe.as_bytes()),
            ]
            .into_iter()
            .map(|(source_label, bytes)| {
                PortableWorkspaceDocument::new(source_label.into(), bytes.to_vec())
            })
            .collect();
            let error = api()
                .compile_portable_workspace(PortableWorkspaceRequest::new(
                    "pale-workspace.toml".into(),
                    documents,
                ))
                .unwrap_err();
            assert_diagnostic_tuple(
                &error,
                "workspace_source_rejected",
                DiagnosticStage::WorkspaceLoading,
                DiagnosticDisposition::InvalidInput,
                DiagnosticRetry::AfterCorrection,
            );
            let diagnostic = error.diagnostic();
            assert_eq!(
                diagnostic.summary(),
                "the record ID and source-family prefix cannot form a derivation source identity"
            );
            let source = diagnostic.source().unwrap();
            assert_eq!(source.label(), label);
            assert_eq!(source.value_path(), Some(field_path.join(".").as_str()));
            assert_eq!(source.line(), None);
            assert_eq!(source.column(), None);
            assert!(diagnostic.schema_coordinate().is_none());
            let location = error.authoring_location().unwrap();
            assert_eq!(location.source_label(), label);
            assert_eq!(
                location.path(),
                field_path
                    .iter()
                    .map(|field| AuthoringInstancePathSegment::Field((*field).into()))
                    .collect::<Vec<_>>()
            );
            assert!(!diagnostic.canonical_json().contains(&logical_id));
            assert!(!format!("{error:?}").contains(&logical_id));
        }
    }
}

#[test]
fn portable_lock_digest_failures_explain_the_family_without_exposing_source_text() {
    for (kind, digest_field) in [
        ("resolution", "resolution_digest"),
        ("universe", "universe_digest"),
    ] {
        let original = if kind == "resolution" {
            RESOLUTION_LOCK
        } else {
            UNIVERSE_LOCK
        };
        let mut lock: toml::Value = toml::from_str(std::str::from_utf8(original).unwrap()).unwrap();
        lock[digest_field] = toml::Value::String("0".repeat(64));
        let marker = "private-source-comment-not-diagnostic";
        let changed = format!("# {marker}\n{}", toml::to_string(&lock).unwrap()).into_bytes();
        let documents = vec![
            PortableWorkspaceDocument::new("pale-workspace.toml".into(), ROOT.to_vec()),
            PortableWorkspaceDocument::new(
                "resolution.lock.toml".into(),
                if kind == "resolution" {
                    changed.clone()
                } else {
                    RESOLUTION_LOCK.to_vec()
                },
            ),
            PortableWorkspaceDocument::new(
                "universe.lock.toml".into(),
                if kind == "universe" {
                    changed
                } else {
                    UNIVERSE_LOCK.to_vec()
                },
            ),
        ];
        let error = api()
            .compile_portable_workspace(PortableWorkspaceRequest::new(
                "pale-workspace.toml".into(),
                documents,
            ))
            .expect_err("changing a claimed digest cannot admit a workspace");
        assert_diagnostic_tuple(
            &error,
            "workspace_semantics_rejected",
            DiagnosticStage::SemanticAdmission,
            DiagnosticDisposition::InvalidInput,
            DiagnosticRetry::AfterCorrection,
        );
        assert_facts(
            &error,
            &[("lock_kind", kind), ("violation", "digest_mismatch")],
        );
        assert_eq!(
            error.diagnostic().summary(),
            "The workspace lock digest does not match its contents."
        );
        assert_eq!(
            error.diagnostic().help(),
            &["Use a lock sealed from the intended inputs; do not edit the digest alone."]
        );
        assert!(
            error.source().is_none(),
            "raw compiler sources remain private"
        );
        assert!(!error.diagnostic().canonical_json().contains(marker));
        assert!(
            error.diagnostic().source().is_none(),
            "a filename is not inferred from the lock kind"
        );
    }
}

#[test]
fn portable_typed_source_rejection_keeps_contract_stage_and_private_cause() {
    let mut module: toml::Value =
        toml::from_str(std::str::from_utf8(POPULATED_MODULE).unwrap()).unwrap();
    assert_ne!(
        module["module_digest"].as_str(),
        Some("0".repeat(64).as_str())
    );
    module["module_digest"] = toml::Value::String("0".repeat(64));
    pale_contracts::validate_value(
        &serde_json::to_value(&module).unwrap(),
        "workspace-module.v1",
    )
    .expect("the changed digest still satisfies the structural schema");
    let marker = "private-module-comment-not-diagnostic";
    let changed = format!("# {marker}\n{}", toml::to_string(&module).unwrap());
    let documents = [
        ("pale-workspace.toml", POPULATED_ROOT),
        ("module.toml", changed.as_bytes()),
        ("alpha.toml", POPULATED_ALPHA),
        ("beta.toml", POPULATED_BETA),
        ("resolution.lock.toml", POPULATED_RESOLUTION_LOCK),
        ("universe.lock.toml", POPULATED_UNIVERSE_LOCK),
    ]
    .into_iter()
    .map(|(label, bytes)| PortableWorkspaceDocument::new(label.to_owned(), bytes.to_vec()))
    .collect();
    let error = api()
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".into(),
            documents,
        ))
        .expect_err("the supplied module cannot pass typed verification");
    assert_diagnostic_tuple(
        &error,
        "workspace_contract_rejected",
        DiagnosticStage::ContractValidation,
        DiagnosticDisposition::InvalidInput,
        DiagnosticRetry::AfterCorrection,
    );
    assert_facts(&error, &[("record", "workspace module")]);
    let location = error.diagnostic().source().unwrap();
    assert_eq!(location.label(), "module.toml");
    assert_eq!(location.line(), None);
    assert_eq!(location.column(), None);
    assert!(error.diagnostic().schema_coordinate().is_none());
    assert!(!error.diagnostic().canonical_json().contains(marker));
    assert!(!format!("{error:?}").contains(marker));
}

#[test]
fn portable_semantic_failure_retains_the_typed_digest_subject() {
    let changed_alpha = std::str::from_utf8(POPULATED_ALPHA)
        .expect("fixture is UTF-8")
        .replacen("id = \"common\"", "id = \"changed\"", 1)
        .into_bytes();
    let request = PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_string(),
        populated_documents_with_alpha(&changed_alpha),
    );
    let error = api()
        .compile_portable_workspace(request)
        .expect_err("owner-referenced component digests bind exact source meaning");

    assert_diagnostic_tuple(
        &error,
        "workspace_semantics_rejected",
        DiagnosticStage::SemanticAdmission,
        DiagnosticDisposition::InvalidInput,
        DiagnosticRetry::AfterCorrection,
    );
    assert_eq!(
        error.diagnostic().subject(),
        Some(("workspace_component", "alpha-source"))
    );
    assert_facts(&error, &[("digest_domain", "semantic source component")]);
}

#[test]
fn portable_owner_proof_failure_is_blocked_and_actionable() {
    let request =
        PortableWorkspaceRequest::new("pale-workspace.toml".to_string(), populated_documents());
    let error = api()
        .compile_portable_workspace(request)
        .expect_err("the proof-free façade cannot infer publisher admission");

    assert_diagnostic_tuple(
        &error,
        "workspace_owner_proof_unavailable",
        DiagnosticStage::SemanticAdmission,
        DiagnosticDisposition::Blocked,
        DiagnosticRetry::AfterStateChange,
    );
    assert_eq!(
        error.diagnostic().subject(),
        Some(("publisher", "candidate-publisher"))
    );
    assert_facts(&error, &[("release", "release-1")]);
}

#[test]
fn portable_release_lifecycle_failure_is_blocked_separately_from_owner_proof() {
    let request = PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_string(),
        candidate_release_documents(),
    );
    let error = api()
        .compile_portable_workspace(request)
        .expect_err("a candidate release cannot enter a new admitted universe");

    assert_diagnostic_tuple(
        &error,
        "workspace_release_not_admissible",
        DiagnosticStage::SemanticAdmission,
        DiagnosticDisposition::Blocked,
        DiagnosticRetry::AfterStateChange,
    );
    assert_eq!(error.diagnostic().subject(), Some(("release", "release-1")));
    assert_facts(&error, &[("module", "candidate")]);
}

#[test]
fn portable_source_delivery_failure_is_invalid_not_owner_blocked() {
    let request = PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_string(),
        source_incomplete_documents(),
    );
    let error = api()
        .compile_portable_workspace(request)
        .expect_err("source delivery requires every exact referenced component");

    assert_diagnostic_tuple(
        &error,
        "workspace_source_incomplete",
        DiagnosticStage::SemanticAdmission,
        DiagnosticDisposition::InvalidInput,
        DiagnosticRetry::AfterCorrection,
    );
    assert_eq!(error.diagnostic().subject(), Some(("module", "candidate")));
    assert_facts(&error, &[("component", "alpha-source")]);
}

#[test]
fn unadmitted_document_and_request_debug_render_only_bounded_counts() {
    let label = "private/workspace.toml";
    let marker = "never-render-captured-body";
    let document = PortableWorkspaceDocument::new(label.to_string(), marker.as_bytes().to_vec());
    let debug = format!("{document:?}");
    assert!(debug.contains("label_byte_len"));
    assert!(debug.contains(&marker.len().to_string()));
    assert!(!debug.contains(label));
    assert!(!debug.contains(marker));

    let root = "private/root-manifest.toml";
    let request = PortableWorkspaceRequest::new(root.to_string(), vec![document]);
    let debug = format!("{request:?}");
    assert!(debug.contains("root_manifest_byte_len"));
    assert!(debug.contains("document_count"));
    assert!(!debug.contains(root));
    assert!(!debug.contains(label));
    assert!(!debug.contains(marker));
}

#[test]
fn nested_confined_paths_and_request_order_preserve_portable_semantics() {
    let admitted = api()
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "config/pale-workspace.toml".to_string(),
            nested_empty_documents(),
        ))
        .expect("nested confined logical paths are part of the workspace grammar");
    assert_eq!(admitted.workspace_id(), "empty-adopter");

    let baseline = api()
        .compile_portable_workspace(empty_request())
        .expect("baseline request is admitted");
    let mut permuted = empty_documents();
    permuted.reverse();
    let reordered = api()
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".to_string(),
            permuted,
        ))
        .expect("document order has no semantic meaning");
    assert_eq!(
        baseline.principal_universe_digest(),
        reordered.principal_universe_digest()
    );
    assert_eq!(
        baseline.semantic_ir_digest(),
        reordered.semantic_ir_digest()
    );
}

#[test]
fn unreferenced_captured_member_has_no_semantic_authority() {
    let baseline = api()
        .compile_portable_workspace(empty_request())
        .expect("baseline request is admitted");
    let mut documents = empty_documents();
    documents.push(PortableWorkspaceDocument::new(
        "unused/broken.toml".to_string(),
        b"this is deliberately not TOML [".to_vec(),
    ));
    let with_unused = api()
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".to_string(),
            documents,
        ))
        .expect("unreferenced members are bounded ingress, not semantic input");
    assert_eq!(with_unused.source_document_count(), 4);
    assert_eq!(
        baseline.principal_universe_digest(),
        with_unused.principal_universe_digest()
    );
    assert_eq!(
        baseline.semantic_ir_digest(),
        with_unused.semantic_ir_digest()
    );
}
