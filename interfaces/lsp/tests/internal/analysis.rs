//! Single-worker editor analysis, staleness, and profile-query laws.

use super::*;
use crate::document::{BaselineDocument, DocumentRole, DocumentStore};
use crate::protocol::{
    ConfigurationImprovementDetailQuery, ConfigurationImprovementQuery,
    ConfigurationImprovementResponse, EffectiveProfileQuery, ExpectedProfileCoordinate,
    ImprovementPreviewCoordinate, ImprovementReviewDisposition, PageParams,
    ProfileInspectionContext, SubjectSelector,
};
use lsp_server::RequestId;
use lsp_types::Uri;
use pale_sdk::{PortableWorkspaceSubject, PortableWorkspaceSubjectSelector};

#[test]
fn reference_worker_requires_exact_page_and_releases_on_discard_or_analysis() {
    let (store, params) = crate::authoring::references::tests::reference_store(300);
    let api = PaleApi::new().unwrap();
    let mut state = AnalysisState::new(None);
    let result = state.analyze(
        &api,
        "pale-workspace.toml",
        AnalysisJob {
            generation: 1,
            snapshot: store.snapshot(),
        },
    );
    assert!(result.outcome.is_ok());
    let coordinate = ReferenceCoordinate {
        request_id: RequestId::from(70),
        generation: 1,
        document_revision: store.revision(),
        page: 0,
    };
    let first = state.start_references(&api, coordinate.clone(), &params);
    assert!(matches!(first.outcome, Ok(ReferenceOutput::Page(page)) if !page.complete));
    let retained = state.references.as_ref().unwrap().coordinate.clone();
    assert_eq!(retained.page, 1);
    for altered in [
        ReferenceCoordinate {
            request_id: RequestId::from(71),
            ..retained.clone()
        },
        ReferenceCoordinate {
            generation: 2,
            ..retained.clone()
        },
        ReferenceCoordinate {
            document_revision: retained.document_revision + 1,
            ..retained.clone()
        },
        ReferenceCoordinate {
            page: 0,
            ..retained.clone()
        },
    ] {
        assert!(matches!(
            state.reference_step(&api, altered).outcome,
            Err(QueryFailure::AnalysisUnavailable)
        ));
        assert_eq!(state.references.as_ref().unwrap().coordinate, retained);
    }
    assert!(matches!(
        state
            .start_references(
                &api,
                ReferenceCoordinate {
                    request_id: RequestId::from(72),
                    ..coordinate.clone()
                },
                &params
            )
            .outcome,
        Err(QueryFailure::AnalysisUnavailable)
    ));
    assert_eq!(state.references.as_ref().unwrap().coordinate, retained);
    assert!(matches!(
        state.discard_references(retained.clone()).outcome,
        Ok(ReferenceOutput::Discarded)
    ));
    assert!(state.references.is_none());
    assert!(matches!(
        state.reference_step(&api, retained).outcome,
        Err(QueryFailure::AnalysisUnavailable)
    ));
    assert!(matches!(
        state.start_references(&api, coordinate, &params).outcome,
        Ok(ReferenceOutput::Page(_))
    ));
    let result = state.analyze(
        &api,
        "pale-workspace.toml",
        AnalysisJob {
            generation: 2,
            snapshot: store.snapshot(),
        },
    );
    assert!(result.outcome.is_ok());
    assert!(state.references.is_none());
}

#[test]
fn actual_worker_reference_pages_use_current_open_buffers_and_finish_once() {
    let (store, params) = crate::authoring::references::tests::reference_store(300);
    let worker = AnalysisWorker::spawn(
        PaleApi::new().unwrap(),
        "pale-workspace.toml".to_owned(),
        None,
        store.baseline_snapshot(),
    );
    worker
        .analyze(AnalysisJob {
            generation: 1,
            snapshot: store.snapshot(),
        })
        .unwrap();
    let WorkerResult::Analysis(analysis) = worker
        .results
        .recv_timeout(std::time::Duration::from_secs(30))
        .unwrap()
    else {
        panic!("analysis");
    };
    assert!(analysis.outcome.is_ok());
    worker
        .query(QueryJob {
            request_id: RequestId::from(80),
            generation: 1,
            document_revision: store.revision(),
            query: WorkspaceQuery::Authoring(crate::protocol::AuthoringQuery::References(params)),
            authoring_capabilities: crate::authoring::AuthoringClientCapabilities::default(),
        })
        .unwrap();
    let mut count = 0;
    let mut complete = false;
    for expected in 0..100 {
        let WorkerResult::References(reply) = worker
            .results
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap()
        else {
            panic!("reference page");
        };
        assert_eq!(reply.coordinate.page, expected);
        let ReferenceOutput::Page(page) = reply.outcome.unwrap() else {
            panic!("page");
        };
        count += page.locations.len();
        if page.complete {
            complete = true;
            break;
        }
        worker
            .continue_references(ReferenceCoordinate {
                page: expected + 1,
                ..reply.coordinate
            })
            .unwrap();
    }
    assert!(complete);
    assert_eq!(count, 300);
    assert!(worker.results.try_recv().is_err());
    let summary = query_at(
        &worker,
        81,
        1,
        store.revision(),
        WorkspaceQuery::CompilationSummary,
    )
    .unwrap();
    assert!(matches!(
        summary,
        WorkspaceQueryResult::CompilationSummary(_)
    ));
    worker.stop().unwrap();
}

/// Canonical portable root-manifest fixture used by editor analysis tests.
const ROOT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/portable/pale-workspace.toml"
));
const RESOLUTION_LOCK: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/portable/resolution.lock.toml"
));
const UNIVERSE_LOCK: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/portable/universe.lock.toml"
));

#[test]
fn authoring_api_failures_retain_the_canonical_diagnostic_channel() {
    let api = pale_sdk::PaleApi::new().unwrap();
    let error = api
        .inspect_standalone_authoring(
            pale_sdk::StandaloneAuthoringRole::WorkspaceManifest,
            (0..=pale_sdk::AUTHORING_PATH_SEGMENT_LIMIT)
                .map(|index| pale_sdk::AuthoringPathSegment::Field(index.to_string()))
                .collect(),
        )
        .unwrap_err();
    let expected = error.diagnostic().canonical_bytes().to_vec();
    let failure = QueryFailure::from(crate::authoring::AuthoringQueryError::from(error));
    let QueryFailure::Api(error) = failure else {
        panic!("authoring API failure escaped the canonical query channel");
    };
    assert_eq!(error.diagnostic().canonical_bytes(), expected);
}

fn uri(value: &str) -> Uri {
    value.parse().unwrap()
}

macro_rules! captured_fixture {
    ($member:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cross-leaf.v1/one-shot/",
            $member
        ))
    };
}

fn profile_store() -> DocumentStore {
    profile_store_with_machine(captured_fixture!("private/machine-binding.toml"))
}

fn profile_store_with_machine(machine_binding: &str) -> DocumentStore {
    let documents = [
        (
            "composition.toml",
            DocumentRole::ExperienceComposition,
            captured_fixture!("workspace/composition.toml"),
        ),
        (
            "pale-workspace.toml",
            DocumentRole::WorkspaceManifest,
            captured_fixture!("workspace/pale-workspace.toml"),
        ),
        (
            "workspace-resolution.lock.toml",
            DocumentRole::WorkspaceResolutionLock,
            captured_fixture!("workspace/workspace-resolution.lock.toml"),
        ),
        (
            "workspace-universe.lock.toml",
            DocumentRole::WorkspaceUniverseLock,
            captured_fixture!("workspace/workspace-universe.lock.toml"),
        ),
        (
            "capabilities.toml",
            DocumentRole::Capabilities,
            captured_fixture!("private/capabilities.toml"),
        ),
        (
            "machines/machine.toml",
            DocumentRole::MachineBinding,
            machine_binding,
        ),
        (
            "distribution-lock.toml",
            DocumentRole::DistributionLock,
            captured_fixture!("private/distribution-lock.toml"),
        ),
        (
            "engine/package.toml",
            DocumentRole::PackageManifest,
            captured_fixture!("packages/engine/package.toml"),
        ),
        (
            "engine/roles.toml",
            DocumentRole::PackageRoles,
            captured_fixture!("packages/engine/roles.toml"),
        ),
        (
            "engine/role.md",
            DocumentRole::PackageText,
            captured_fixture!("packages/engine/role.md"),
        ),
    ]
    .into_iter()
    .map(|(label, role, text)| BaselineDocument {
        uri: uri(&format!("file:///workspace/{label}")),
        language_id: if label.ends_with(".md") {
            "pale".to_owned()
        } else {
            "pale-toml".to_owned()
        },
        role,
        text: text.to_owned(),
    })
    .collect();
    DocumentStore::new(&uri("file:///workspace"), documents).unwrap()
}

#[test]
fn effective_profile_projects_selected_provider_repository_without_claiming_realization() {
    let machine_binding = format!(
        "{}\n[[provider_repositories]]\nrepository_id = \"engine\"\nprovider_binding_id = \"provider-binding-claude-code\"\nnative_repository_coordinate = \"owner://rig/repository\"\n",
        captured_fixture!("private/machine-binding.toml")
    );
    let snapshot = profile_store_with_machine(&machine_binding).baseline_snapshot();
    let worker = AnalysisWorker::spawn(
        PaleApi::new().unwrap(),
        "pale-workspace.toml".to_owned(),
        Some("machine".to_owned()),
        snapshot.clone(),
    );
    worker
        .analyze(AnalysisJob {
            generation: 1,
            snapshot,
        })
        .unwrap();
    let WorkerResult::Analysis(result) = worker.results.recv().unwrap() else {
        panic!("analysis must return an analysis result");
    };
    result.outcome.unwrap();

    let WorkspaceQueryResult::EffectiveProfile(response) = query_at(
        &worker,
        9,
        1,
        0,
        WorkspaceQuery::EffectiveProfile(EffectiveProfileQuery {
            context: ProfileInspectionContext {
                project_id: "demo".to_owned(),
                execution_workspace_id: "default".to_owned(),
            },
            section: pale_sdk::EffectiveProfileSection::EffectiveConfiguration,
            offset: 0,
            limit: 64,
            expected_coordinate: None,
        }),
    )
    .unwrap() else {
        panic!("profile query returned the wrong result shape");
    };
    let page = response.response.page.expect("current profile has a page");
    assert!(page.entries().iter().any(|entry| matches!(
        entry.detail(),
        pale_sdk::EffectiveProfileEntryDetail::ProviderRepository {
            repository_id,
            provider_binding_id,
            native_repository_coordinate,
            native_realization: pale_sdk::EffectiveProfileNativeRealization::Unknown,
        } if repository_id == "engine"
            && provider_binding_id == "provider-binding-claude-code"
            && native_repository_coordinate == "owner://rig/repository"
    )));
    worker.stop().unwrap();
}

#[test]
fn profile_contexts_are_bound_to_the_queried_editor_generation() {
    let snapshot = profile_store().baseline_snapshot();
    let revision = snapshot.revision();
    let worker = AnalysisWorker::spawn(
        PaleApi::new().unwrap(),
        "pale-workspace.toml".to_owned(),
        Some("machine".to_owned()),
        snapshot.clone(),
    );
    admit_snapshot(&worker, 7, snapshot);
    let WorkspaceQueryResult::ProfileContexts(response) =
        query_at(&worker, 10, 7, revision, WorkspaceQuery::ProfileContexts).unwrap()
    else {
        panic!("profile-context query returned the wrong result shape");
    };
    assert_eq!(response.editor.generation, 7);
    assert_eq!(response.editor.document_revision, revision);
    assert_eq!(response.contexts.len(), 1);
    assert_eq!(response.contexts[0].project_id(), "demo");
    assert_eq!(response.contexts[0].execution_workspace_id(), "default");
    assert!(matches!(
        query_at(&worker, 11, 6, revision, WorkspaceQuery::ProfileContexts),
        Err(QueryFailure::AnalysisUnavailable)
    ));
    worker.stop().unwrap();
}

fn baseline_store() -> DocumentStore {
    DocumentStore::new(
        &uri("file:///workspace"),
        [
            ("pale-workspace.toml", ROOT),
            ("resolution.lock.toml", RESOLUTION_LOCK),
            ("universe.lock.toml", UNIVERSE_LOCK),
        ]
        .into_iter()
        .map(|(label, text)| BaselineDocument {
            uri: uri(&format!("file:///workspace/{label}")),
            language_id: "pale-toml".to_owned(),
            role: match label {
                "pale-workspace.toml" => DocumentRole::WorkspaceManifest,
                "resolution.lock.toml" => DocumentRole::WorkspaceResolutionLock,
                "universe.lock.toml" => DocumentRole::WorkspaceUniverseLock,
                _ => unreachable!("closed baseline fixture"),
            },
            text: text.to_owned(),
        })
        .collect(),
    )
    .unwrap()
}

fn query(worker: &AnalysisWorker, id: i32, query: WorkspaceQuery) -> WorkspaceQueryResult {
    query_at(worker, id, 1, 0, query).unwrap()
}

fn query_at(
    worker: &AnalysisWorker,
    id: i32,
    generation: u64,
    document_revision: u64,
    query: WorkspaceQuery,
) -> Result<WorkspaceQueryResult, QueryFailure> {
    worker
        .query(QueryJob {
            request_id: RequestId::from(id),
            generation,
            document_revision,
            query,
            authoring_capabilities: crate::authoring::AuthoringClientCapabilities::default(),
        })
        .unwrap();
    let WorkerResult::Query(result) = worker.results.recv().unwrap() else {
        panic!("query must return a query result");
    };
    result.outcome
}

#[test]
fn worker_retains_one_admitted_baseline_comparison_for_typed_queries() {
    let mut store = baseline_store();
    let baseline = store.baseline_snapshot();
    let worker = AnalysisWorker::spawn(
        PaleApi::new().unwrap(),
        "pale-workspace.toml".to_owned(),
        None,
        baseline.clone(),
    );
    worker
        .analyze(AnalysisJob {
            generation: 1,
            snapshot: baseline,
        })
        .unwrap();
    let WorkerResult::Analysis(result) = worker.results.recv().unwrap() else {
        panic!("analysis command must return an analysis result");
    };
    result.outcome.unwrap();

    let WorkspaceQueryResult::CompilationSummary(compilation) =
        query(&worker, 1, WorkspaceQuery::CompilationSummary)
    else {
        panic!("compilation summary query returned the wrong shape");
    };
    assert_eq!(compilation.workspace_id(), "empty-adopter");
    assert_eq!(
        serde_json::to_value(&compilation).unwrap(),
        serde_json::from_slice::<serde_json::Value>(compilation.canonical_bytes()).unwrap()
    );

    let WorkspaceQueryResult::ComparisonSummary(comparison) =
        query(&worker, 2, WorkspaceQuery::ComparisonSummary)
    else {
        panic!("comparison summary query returned the wrong shape");
    };
    assert_eq!(comparison.change_root_count(), 0);
    assert_eq!(
        serde_json::to_value(&comparison).unwrap(),
        serde_json::from_slice::<serde_json::Value>(comparison.canonical_bytes()).unwrap()
    );

    let WorkspaceQueryResult::ChangeRoots(roots) = query(
        &worker,
        3,
        WorkspaceQuery::ChangeRoots(PageParams {
            offset: 0,
            limit: 16,
        }),
    ) else {
        panic!("change-root query returned the wrong shape");
    };
    assert_eq!(
        serde_json::to_value(&roots).unwrap(),
        serde_json::json!({"entries": [], "nextOffset": null})
    );

    let WorkspaceQueryResult::Impacts(impacts) = query(
        &worker,
        4,
        WorkspaceQuery::Impacts(PageParams {
            offset: 0,
            limit: 16,
        }),
    ) else {
        panic!("impact query returned the wrong shape");
    };
    assert!(!serde_json::to_value(&impacts).unwrap()["entries"]
        .as_array()
        .unwrap()
        .is_empty());

    let subject = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
        workspace_id: compilation.workspace_id().to_owned(),
    })
    .unwrap();
    let WorkspaceQueryResult::Explanation(explanation) =
        query(&worker, 5, WorkspaceQuery::Explanation(subject))
    else {
        panic!("explanation query returned the wrong shape");
    };
    let explanation = serde_json::to_value(explanation).unwrap();
    assert_eq!(explanation["state"], "unaffected");
    assert_eq!(
        explanation["subject"],
        serde_json::to_value(SubjectSelector::Workspace {
            workspace_id: compilation.workspace_id().to_owned()
        })
        .unwrap()
    );
    let mut cases = vec![
        serde_json::json!({"schema": "CompilationSummaryResponse", "value": compilation}),
        serde_json::json!({"schema": "ComparisonSummaryResponse", "value": comparison}),
        serde_json::json!({"schema": "ChangeRootsResponse", "value": roots}),
        serde_json::json!({"schema": "ImpactsResponse", "value": impacts}),
        serde_json::json!({"schema": "ExplanationResponse", "value": explanation}),
    ];
    // A source comment changes captured bytes, not constitutional meaning.
    store
        .open(
            uri("file:///workspace/pale-workspace.toml"),
            "pale-toml".to_owned(),
            1,
            format!("{ROOT}\n# private-wire-comment-only-change\n"),
        )
        .unwrap();
    let current = store.snapshot();
    let revision = current.revision();
    admit_snapshot(&worker, 2, current);
    let WorkspaceQueryResult::ChangeRoots(changed_roots) = query_at(
        &worker,
        6,
        2,
        revision,
        WorkspaceQuery::ChangeRoots(PageParams {
            offset: 0,
            limit: 1,
        }),
    )
    .unwrap() else {
        panic!("change roots must use the owning projection")
    };
    let changed_roots = serde_json::to_value(changed_roots).unwrap();
    assert_eq!(changed_roots["entries"].as_array().unwrap().len(), 1);
    assert_eq!(changed_roots["entries"][0]["change"]["kind"], "modified");
    let source = changed_roots["entries"][0]["subject"].clone();
    let explanation_query = crate::protocol::decode_query(
        crate::protocol::EXPLANATION_METHOD,
        serde_json::json!({"subject": source}),
    )
    .unwrap()
    .unwrap();
    let WorkspaceQueryResult::Explanation(affected) =
        query_at(&worker, 7, 2, revision, explanation_query).unwrap()
    else {
        panic!("explanation must use the owning projection")
    };
    let affected = serde_json::to_value(affected).unwrap();
    assert_eq!(affected["state"], "affected");
    let absent_query = crate::protocol::decode_query(
        crate::protocol::EXPLANATION_METHOD,
        serde_json::json!({"subject": {"kind": "workspace", "workspaceId": "missing-workspace"}}),
    )
    .unwrap()
    .unwrap();
    let WorkspaceQueryResult::Explanation(absent) =
        query_at(&worker, 8, 2, revision, absent_query).unwrap()
    else {
        panic!("absence must use the owning projection")
    };
    let absent = serde_json::to_value(absent).unwrap();
    assert_eq!(absent["state"], "not-present");
    let WorkspaceQueryResult::Impacts(first_page) = query_at(
        &worker,
        9,
        2,
        revision,
        WorkspaceQuery::Impacts(PageParams {
            offset: 0,
            limit: 1,
        }),
    )
    .unwrap() else {
        panic!("impact paging must use the owning projection")
    };
    let first_page = serde_json::to_value(first_page).unwrap();
    assert_eq!(first_page["entries"].as_array().unwrap().len(), 1);
    assert_eq!(first_page["nextOffset"], 1);
    cases.extend([
        serde_json::json!({"schema": "ChangeRootsResponse", "value": changed_roots}),
        serde_json::json!({"schema": "ExplanationResponse", "value": affected}),
        serde_json::json!({"schema": "ExplanationResponse", "value": absent}),
        serde_json::json!({"schema": "ImpactsResponse", "value": first_page}),
    ]);
    worker.stop().unwrap();
    // Reuse the existing wire fixture for ordinary fields. These six cases
    // exercise the actual serializers, not worker admission at these counters.
    let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cross-leaf.v1/effective-profile-wire.json"
    )))
    .unwrap();
    let Some(WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Begin {
        context,
        expected_profile,
    })) = crate::protocol::decode_query(
        crate::protocol::CONFIGURATION_IMPROVEMENT_METHOD,
        fixture["improvement"]["beginRequest"].clone(),
    )
    .unwrap()
    else {
        panic!("the existing wire fixture must decode as a begin request");
    };
    for counter in [0, 9_007_199_254_740_993, u64::MAX] {
        let mut baseline = expected_profile.clone();
        baseline.generation = counter;
        baseline.document_revision = counter;
        let response = EffectiveProfileResponse {
            state: EffectiveProfileState::NotAdmitted {
                summary: "serializer-only coordinate boundary".to_owned(),
            },
            editor: EditorCoordinate {
                generation: counter,
                document_revision: counter,
            },
            context: context.clone(),
            coordinates: None,
            page: None,
            diagnostic_identity: None,
        };
        cases.extend([
            serde_json::json!({
                "origin": "serializer-only-u64-coordinate",
                "schema": "EffectiveProfileResponse", "value": response
            }),
            serde_json::json!({
                "origin": "serializer-only-u64-coordinate",
                "schema": "ConfigurationImprovementResponse",
                "value": ConfigurationImprovementResponse::Explained {
                    context: context.clone(), baseline
                }
            }),
        ]);
    }
    if let Some(destination) = std::env::var_os("PALE_EXPORT_PRIVATE_WIRE") {
        let mut bytes = serde_json::to_vec_pretty(&cases).unwrap();
        bytes.push(b'\n');
        std::fs::write(destination, bytes).unwrap();
    }
}

fn profile_query(context: ProfileInspectionContext, offset: u32) -> WorkspaceQuery {
    profile_query_at(context, offset, None)
}

#[test]
fn invalid_private_buffer_keeps_portable_success_distinct_from_profile_admission() {
    let mut store = profile_store();
    let initial = store.baseline_snapshot();
    let worker = AnalysisWorker::spawn(
        PaleApi::new().unwrap(),
        "pale-workspace.toml".to_owned(),
        Some("machine".to_owned()),
        initial.clone(),
    );
    admit_snapshot(&worker, 1, initial);
    let context = ProfileInspectionContext {
        project_id: "demo".to_owned(),
        execution_workspace_id: "default".to_owned(),
    };
    let WorkspaceQueryResult::EffectiveProfile(initial_profile) =
        query(&worker, 20, profile_query(context.clone(), 0))
    else {
        panic!("expected the initial profile")
    };
    assert!(matches!(
        initial_profile.response.state,
        crate::protocol::EffectiveProfileState::Current
    ));

    store
        .open(
            uri("file:///workspace/machines/machine.toml"),
            "pale-toml".to_owned(),
            1,
            "this is not valid TOML = [".to_owned(),
        )
        .unwrap();
    let invalid = store.snapshot();
    let revision = invalid.revision();
    worker
        .analyze(AnalysisJob {
            generation: 2,
            snapshot: invalid,
        })
        .unwrap();
    let WorkerResult::Analysis(result) = worker.results.recv().unwrap() else {
        panic!("expected current analysis")
    };
    assert!(result.outcome.is_err());
    // The unchanged portable source still compiles. That is not evidence that
    // the private machine binding or the effective profile was admitted.
    assert!(matches!(
        query_at(&worker, 21, 2, revision, WorkspaceQuery::CompilationSummary).unwrap(),
        WorkspaceQueryResult::CompilationSummary(_)
    ));
    let WorkspaceQueryResult::EffectiveProfile(rejected) =
        query_at(&worker, 22, 2, revision, profile_query(context, 0)).unwrap()
    else {
        panic!("expected a marked prior profile")
    };
    assert_eq!(
        rejected.response.editor,
        crate::protocol::EditorCoordinate {
            generation: 2,
            document_revision: revision,
        }
    );
    let crate::protocol::EffectiveProfileState::Stale {
        admitted_editor,
        admitted_profile_digest,
        ..
    } = &rejected.response.state
    else {
        panic!("expected a marked prior profile")
    };
    assert_eq!(*admitted_editor, initial_profile.response.editor);
    assert_eq!(
        admitted_profile_digest,
        &initial_profile
            .response
            .coordinates
            .as_ref()
            .unwrap()
            .profile_digest
    );
    assert_eq!(
        rejected.response.coordinates,
        initial_profile.response.coordinates
    );
    worker.stop().unwrap();
}

fn profile_query_at(
    context: ProfileInspectionContext,
    offset: u32,
    expected_coordinate: Option<ExpectedProfileCoordinate>,
) -> WorkspaceQuery {
    WorkspaceQuery::EffectiveProfile(EffectiveProfileQuery {
        context,
        section: pale_sdk::EffectiveProfileSection::EffectiveConfiguration,
        offset,
        limit: 1,
        expected_coordinate,
    })
}

fn wire_fixture_profile_query(context: ProfileInspectionContext) -> WorkspaceQuery {
    WorkspaceQuery::EffectiveProfile(EffectiveProfileQuery {
        context,
        section: pale_sdk::EffectiveProfileSection::MachineRealization,
        offset: 0,
        limit: 1,
        expected_coordinate: None,
    })
}

fn profile_explanation_query(
    context: ProfileInspectionContext,
    handle: pale_sdk::EffectiveProfileExplanationHandle,
    expected_coordinate: ExpectedProfileCoordinate,
) -> WorkspaceQuery {
    WorkspaceQuery::EffectiveProfileExplanation(EffectiveProfileExplanationQuery {
        context,
        handle,
        expected_coordinate,
    })
}

#[test]
fn effective_profile_is_current_then_explicitly_stale_only_for_the_same_context() {
    let mut store = profile_store();
    let first = store.baseline_snapshot();
    let worker = AnalysisWorker::spawn(
        PaleApi::new().unwrap(),
        "pale-workspace.toml".to_owned(),
        Some("machine".to_owned()),
        first.clone(),
    );
    worker
        .analyze(AnalysisJob {
            generation: 1,
            snapshot: first,
        })
        .unwrap();
    let WorkerResult::Analysis(result) = worker.results.recv().unwrap() else {
        panic!("analysis must return an analysis result");
    };
    result.outcome.unwrap();

    let context = ProfileInspectionContext {
        project_id: "demo".to_owned(),
        execution_workspace_id: "default".to_owned(),
    };
    let WorkspaceQueryResult::EffectiveProfile(current) =
        query_at(&worker, 10, 1, 0, profile_query(context.clone(), 0)).unwrap()
    else {
        panic!("profile query returned the wrong result shape");
    };
    assert!(
        current.diagnostic().is_none(),
        "current profile unexpectedly failed: {}",
        current
            .diagnostic()
            .map_or("no diagnostic", pale_sdk::DiagnosticV1::canonical_json)
    );
    let current = current.response;
    assert!(matches!(current.state, EffectiveProfileState::Current));
    assert!(current.coordinates.is_some());
    assert_eq!(
        serde_json::to_value(&current).unwrap()["coordinates"]["instanceDigest"],
        current.coordinates.as_ref().unwrap().instance_digest
    );
    let page = current.page.as_ref().unwrap();
    assert_eq!(page.entries().len(), 1);
    let explanation_handle = page.entries()[0].explanation_handle().clone();
    assert_eq!(
        serde_json::to_value(page).unwrap(),
        serde_json::from_slice::<serde_json::Value>(page.canonical_bytes()).unwrap()
    );
    assert!(matches!(
        query_at(&worker, 13, 1, 0, profile_query(context.clone(), 1)),
        Err(QueryFailure::ProfileCoordinateRequired)
    ));
    let expected_coordinate = ExpectedProfileCoordinate::from_profile(
        current.editor,
        current.coordinates.as_ref().unwrap(),
    );
    let WorkspaceQueryResult::EffectiveProfileExplanation(explanation) = query_at(
        &worker,
        13,
        1,
        0,
        profile_explanation_query(
            context.clone(),
            explanation_handle.clone(),
            expected_coordinate.clone(),
        ),
    )
    .unwrap() else {
        panic!("profile explanation query returned the wrong result shape");
    };
    assert_eq!(explanation.explanation.handle, explanation_handle);
    assert!(matches!(
        query_at(
            &worker,
            13_001,
            1,
            0,
            profile_explanation_query(
                context.clone(),
                pale_sdk::EffectiveProfileExplanationHandle::parse("f".repeat(64)).unwrap(),
                expected_coordinate.clone(),
            ),
        ),
        Err(QueryFailure::ProfileExplanationUnavailable)
    ));
    assert!(query_at(
        &worker,
        14,
        1,
        0,
        profile_query_at(context.clone(), 1, Some(expected_coordinate.clone())),
    )
    .is_ok());
    let mismatches = [
        ExpectedProfileCoordinate {
            profile_digest: "sha256:mismatched".to_owned(),
            ..expected_coordinate.clone()
        },
        ExpectedProfileCoordinate {
            instance_digest: "sha256:mismatched".to_owned(),
            ..expected_coordinate.clone()
        },
        ExpectedProfileCoordinate {
            selected_topology_digest: "sha256:mismatched".to_owned(),
            ..expected_coordinate.clone()
        },
        ExpectedProfileCoordinate {
            workspace_semantic_ir_digest: "sha256:mismatched".to_owned(),
            ..expected_coordinate.clone()
        },
        ExpectedProfileCoordinate {
            workspace_admission_evidence_digest: "sha256:mismatched".to_owned(),
            ..expected_coordinate.clone()
        },
        ExpectedProfileCoordinate {
            package_semantic_ir_digest: "sha256:mismatched".to_owned(),
            ..expected_coordinate.clone()
        },
        ExpectedProfileCoordinate {
            composition_plan_digest: "sha256:mismatched".to_owned(),
            ..expected_coordinate
        },
    ];
    for (request_id, mismatch) in (15..).zip(mismatches) {
        assert!(matches!(
            query_at(
                &worker,
                request_id,
                1,
                0,
                profile_query_at(context.clone(), 1, Some(mismatch)),
            ),
            Err(QueryFailure::ProfileCoordinateMismatch)
        ));
    }

    store
        .open(
            uri("file:///workspace/pale-workspace.toml"),
            "pale-toml".to_owned(),
            1,
            "not valid TOML =".to_owned(),
        )
        .unwrap();
    let rejected = store.snapshot();
    worker
        .analyze(AnalysisJob {
            generation: 2,
            snapshot: rejected.clone(),
        })
        .unwrap();
    let WorkerResult::Analysis(result) = worker.results.recv().unwrap() else {
        panic!("analysis must return an analysis result");
    };
    let rejected_failures = result.outcome.unwrap_err();
    let rejected_diagnostic = rejected_failures
        .iter()
        .next()
        .expect("a rejected analysis retains at least one owning failure")
        .diagnostic()
        .clone();

    let WorkspaceQueryResult::EffectiveProfile(stale) = query_at(
        &worker,
        11,
        2,
        rejected.revision(),
        profile_query(context, 0),
    )
    .unwrap() else {
        panic!("stale profile query returned the wrong result shape");
    };
    assert_eq!(stale.diagnostic(), Some(&rejected_diagnostic));
    let stale = stale.response;
    assert_eq!(
        stale.diagnostic_identity.as_ref(),
        Some(rejected_diagnostic.identity())
    );
    assert!(matches!(stale.state, EffectiveProfileState::Stale { .. }));
    assert!(stale.page.is_some());
    let stale_expected =
        ExpectedProfileCoordinate::from_profile(stale.editor, stale.coordinates.as_ref().unwrap());
    assert!(matches!(
        query_at(
            &worker,
            13_002,
            2,
            rejected.revision(),
            profile_explanation_query(stale.context.clone(), explanation_handle, stale_expected,),
        ),
        Ok(WorkspaceQueryResult::EffectiveProfileExplanation(_))
    ));

    let other = ProfileInspectionContext {
        project_id: "other".to_owned(),
        execution_workspace_id: "default".to_owned(),
    };
    let WorkspaceQueryResult::EffectiveProfile(not_admitted) =
        query_at(&worker, 12, 2, rejected.revision(), profile_query(other, 0)).unwrap()
    else {
        panic!("not-admitted query returned the wrong result shape");
    };
    assert_eq!(not_admitted.diagnostic(), Some(&rejected_diagnostic));
    let not_admitted = not_admitted.response;
    assert_eq!(
        not_admitted.diagnostic_identity.as_ref(),
        Some(rejected_diagnostic.identity())
    );
    assert!(matches!(
        not_admitted.state,
        EffectiveProfileState::NotAdmitted { .. }
    ));
    assert!(not_admitted.coordinates.is_none());
    assert!(not_admitted.page.is_none());
    worker.stop().unwrap();
}

#[test]
fn effective_profile_wire_fixture_is_the_exact_rust_projection() {
    let mut store = profile_store();
    let initial = store.baseline_snapshot();
    let worker = AnalysisWorker::spawn(
        PaleApi::new().unwrap(),
        "pale-workspace.toml".to_owned(),
        Some("machine".to_owned()),
        initial.clone(),
    );
    admit_snapshot(&worker, 1, initial);
    let context = ProfileInspectionContext {
        project_id: "demo".to_owned(),
        execution_workspace_id: "default".to_owned(),
    };

    let WorkspaceQueryResult::EffectiveProfile(current) = query_at(
        &worker,
        50,
        1,
        0,
        wire_fixture_profile_query(context.clone()),
    )
    .unwrap() else {
        panic!("profile query returned the wrong result shape");
    };
    let current = current.response;
    let expected = ExpectedProfileCoordinate::from_profile(
        current.editor,
        current
            .coordinates
            .as_ref()
            .expect("the current profile carries exact coordinates"),
    );
    let handle = current
        .page
        .as_ref()
        .and_then(|page| page.entries().first())
        .expect("the effective-configuration fixture carries one row")
        .explanation_handle()
        .clone();
    let current_value = serde_json::to_value(&current).unwrap();

    let WorkspaceQueryResult::EffectiveProfileExplanation(explanation) = query_at(
        &worker,
        51,
        1,
        0,
        profile_explanation_query(context.clone(), handle.clone(), expected.clone()),
    )
    .unwrap() else {
        panic!("profile explanation returned the wrong result shape");
    };
    let explanation_value = serde_json::to_value(&explanation).unwrap();

    let WorkspaceQueryResult::ConfigurationImprovement(explained) = query_at(
        &worker,
        52,
        1,
        0,
        WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Begin {
            context: context.clone(),
            expected_profile: expected.clone(),
        }),
    )
    .unwrap() else {
        panic!("begin returned the wrong result shape");
    };
    let explained_value = serde_json::to_value(&*explained).unwrap();

    let WorkspaceQueryResult::ConfigurationImprovement(previewed) = query_at(
        &worker,
        53,
        1,
        0,
        WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Evaluate {
            context: context.clone(),
            expected_baseline: expected.clone(),
        }),
    )
    .unwrap() else {
        panic!("evaluate returned the wrong result shape");
    };
    let ConfigurationImprovementResponse::Previewed { coordinate, .. } = &*previewed else {
        panic!("evaluate did not return a preview");
    };
    let preview_coordinate = coordinate.clone();
    let previewed_value = serde_json::to_value(&*previewed).unwrap();

    let WorkspaceQueryResult::ConfigurationImprovement(terminal) = query_at(
        &worker,
        54,
        1,
        0,
        WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Finish {
            context: context.clone(),
            expected_preview: preview_coordinate.clone(),
            disposition: ImprovementReviewDisposition::Cancel,
        }),
    )
    .unwrap() else {
        panic!("finish returned the wrong result shape");
    };
    let terminal_value = serde_json::to_value(&*terminal).unwrap();

    store
        .open(
            uri("file:///workspace/pale-workspace.toml"),
            "pale-toml".to_owned(),
            1,
            "not valid TOML =".to_owned(),
        )
        .unwrap();
    let rejected = store.snapshot();
    worker
        .analyze(AnalysisJob {
            generation: 2,
            snapshot: rejected.clone(),
        })
        .unwrap();
    let WorkerResult::Analysis(result) = worker.results.recv().unwrap() else {
        panic!("analysis must return an analysis result");
    };
    assert!(result.outcome.is_err());
    let WorkspaceQueryResult::EffectiveProfile(stale) = query_at(
        &worker,
        55,
        2,
        rejected.revision(),
        wire_fixture_profile_query(context.clone()),
    )
    .unwrap() else {
        panic!("stale profile query returned the wrong result shape");
    };
    let stale_value = serde_json::to_value(&stale.response).unwrap();
    let other_context = ProfileInspectionContext {
        project_id: "other".to_owned(),
        execution_workspace_id: "default".to_owned(),
    };
    let WorkspaceQueryResult::EffectiveProfile(not_admitted) = query_at(
        &worker,
        56,
        2,
        rejected.revision(),
        wire_fixture_profile_query(other_context.clone()),
    )
    .unwrap() else {
        panic!("not-admitted query returned the wrong result shape");
    };

    let mut advertisement = crate::protocol::experimental_capabilities();
    // The fixture attests the actual Rust inventory, but cannot contain its own
    // byte digest. Clients restore that one field from these serialized bytes.
    assert!(advertisement["paleWorkspace"]
        .as_object_mut()
        .unwrap()
        .remove("wireFixtureSha256")
        .is_some());
    let actual = serde_json::json!({
        "protocolAdvertisementWithoutWireDigest": advertisement,
        "profileRequest": {
            "context": context,
            "section": "machine-realization",
            "offset": 0,
            "limit": 1
        },
        "currentProfileResponse": current_value,
        "explanationRequest": {
            "context": context,
            "handle": handle,
            "expectedCoordinate": expected
        },
        "explanationResponse": explanation_value,
        "staleProfileResponse": stale_value,
        "notAdmittedProfileRequest": {
            "context": other_context,
            "section": "machine-realization",
            "offset": 0,
            "limit": 1
        },
        "notAdmittedProfileResponse": not_admitted.response,
        "improvement": {
            "beginRequest": {
                "action": "begin",
                "context": context,
                "expectedProfile": expected
            },
            "explainedResponse": explained_value,
            "evaluateRequest": {
                "action": "evaluate",
                "context": context,
                "expectedBaseline": expected
            },
            "previewedResponse": previewed_value,
            "finishRequest": {
                "action": "finish",
                "context": context,
                "expectedPreview": preview_coordinate,
                "disposition": "cancel"
            },
            "terminalResponse": terminal_value
        }
    });
    worker.stop().unwrap();
    if std::env::var_os("PALE_UPDATE_CONFORMANCE").as_deref() == Some("1".as_ref()) {
        let mut bytes = serde_json::to_vec_pretty(&actual).expect("wire fixture serializes");
        bytes.push(b'\n');
        std::fs::write(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../contracts/conformance/cross-leaf.v1/effective-profile-wire.json"),
            bytes,
        )
        .expect("canonical LSP wire fixture writes");
    } else {
        let expected_fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cross-leaf.v1/effective-profile-wire.json"
        )))
        .unwrap();
        assert_eq!(actual, expected_fixture);
    }
}

fn admit_snapshot(worker: &AnalysisWorker, generation: u64, snapshot: Snapshot) {
    worker
        .analyze(AnalysisJob {
            generation,
            snapshot,
        })
        .unwrap();
    let WorkerResult::Analysis(result) = worker.results.recv().unwrap() else {
        panic!("analysis must return an analysis result");
    };
    result.outcome.unwrap();
}

fn admitted_profile(
    worker: &AnalysisWorker,
    request_id: i32,
    generation: u64,
    revision: u64,
    context: &ProfileInspectionContext,
) -> ExpectedProfileCoordinate {
    let WorkspaceQueryResult::EffectiveProfile(result) = query_at(
        worker,
        request_id,
        generation,
        revision,
        profile_query(context.clone(), 0),
    )
    .unwrap() else {
        panic!("profile query returned the wrong result shape");
    };
    let response = result.response;
    assert!(matches!(response.state, EffectiveProfileState::Current));
    ExpectedProfileCoordinate::from_profile(
        response.editor,
        response
            .coordinates
            .as_ref()
            .expect("a current profile carries its exact coordinates"),
    )
}

fn begin_review(
    worker: &AnalysisWorker,
    request_id: i32,
    generation: u64,
    revision: u64,
    context: &ProfileInspectionContext,
    baseline: ExpectedProfileCoordinate,
) -> ExpectedProfileCoordinate {
    let WorkspaceQueryResult::ConfigurationImprovement(result) = query_at(
        worker,
        request_id,
        generation,
        revision,
        WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Begin {
            context: context.clone(),
            expected_profile: baseline.clone(),
        }),
    )
    .unwrap() else {
        panic!("begin returned the wrong result shape");
    };
    let ConfigurationImprovementResponse::Explained {
        context: returned_context,
        baseline: returned_baseline,
    } = *result
    else {
        panic!("begin did not produce an explained baseline");
    };
    assert_eq!(returned_context, *context);
    assert_eq!(returned_baseline, baseline);
    returned_baseline
}

fn preview_review(
    worker: &AnalysisWorker,
    request_id: i32,
    generation: u64,
    revision: u64,
    context: &ProfileInspectionContext,
    baseline: ExpectedProfileCoordinate,
) -> ImprovementPreviewCoordinate {
    let WorkspaceQueryResult::ConfigurationImprovement(result) = query_at(
        worker,
        request_id,
        generation,
        revision,
        WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Evaluate {
            context: context.clone(),
            expected_baseline: baseline.clone(),
        }),
    )
    .unwrap() else {
        panic!("evaluate returned the wrong result shape");
    };
    let ConfigurationImprovementResponse::Previewed {
        context: returned_context,
        coordinate,
        preview,
    } = *result
    else {
        panic!("evaluate did not produce a preview");
    };
    assert_eq!(returned_context, *context);
    assert_eq!(coordinate.baseline, baseline);
    assert_eq!(
        coordinate.candidate_profile_digest,
        preview.candidate_profile_digest,
    );
    assert_eq!(
        coordinate.candidate_instance_digest,
        preview.candidate_instance_digest
    );
    assert_eq!(
        coordinate.derivation_receipt_digest,
        preview.derivation_receipt_digest
    );
    coordinate
}

fn finish_review(
    worker: &AnalysisWorker,
    request_id: i32,
    generation: u64,
    revision: u64,
    context: &ProfileInspectionContext,
    coordinate: ImprovementPreviewCoordinate,
    disposition: ImprovementReviewDisposition,
) -> pale_sdk::ConfigurationImprovementDisposition {
    let WorkspaceQueryResult::ConfigurationImprovement(result) = query_at(
        worker,
        request_id,
        generation,
        revision,
        WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Finish {
            context: context.clone(),
            expected_preview: coordinate.clone(),
            disposition,
        }),
    )
    .unwrap() else {
        panic!("finish returned the wrong result shape");
    };
    let ConfigurationImprovementResponse::Terminal {
        context: returned_context,
        disposition,
        coordinate: returned_coordinate,
        ..
    } = *result
    else {
        panic!("finish did not produce a terminal disposition");
    };
    assert_eq!(returned_context, *context);
    assert_eq!(returned_coordinate, coordinate);
    disposition
}

fn inspect_review_subject(
    worker: &AnalysisWorker,
    request_id: i32,
    generation: u64,
    revision: u64,
    context: &ProfileInspectionContext,
    coordinate: ImprovementPreviewCoordinate,
    subject: PortableWorkspaceSubject,
) -> (serde_json::Value, usize) {
    let WorkspaceQueryResult::ConfigurationImprovement(result) = query_at(
        worker,
        request_id,
        generation,
        revision,
        WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Inspect {
            context: context.clone(),
            expected_preview: coordinate,
            detail: Box::new(ConfigurationImprovementDetailQuery::Subject {
                subject,
                input_limit: 16,
                maximum_inspected: 32,
                maximum_returned: 16,
            }),
        }),
    )
    .unwrap() else {
        panic!("subject inspection returned the wrong result shape");
    };
    let ConfigurationImprovementResponse::SubjectDetails {
        context: returned_context,
        explanation,
        source,
        ..
    } = *result
    else {
        panic!("subject inspection did not return subject details");
    };
    assert_eq!(returned_context, *context);
    let explanation = serde_json::to_value(explanation).unwrap();
    let source = serde_json::to_value(source).unwrap();
    assert_eq!(explanation["subject"], source["subject"]);
    let occurrence_count = source["occurrences"].as_array().unwrap().len();
    (explanation, occurrence_count)
}

fn inspect_review_overview(
    worker: &AnalysisWorker,
    request_id: i32,
    generation: u64,
    revision: u64,
    context: &ProfileInspectionContext,
    coordinate: ImprovementPreviewCoordinate,
) -> (usize, usize, PortableWorkspaceSubject) {
    let WorkspaceQueryResult::ConfigurationImprovement(result) = query_at(
        worker,
        request_id,
        generation,
        revision,
        WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Inspect {
            context: context.clone(),
            expected_preview: coordinate,
            detail: Box::new(ConfigurationImprovementDetailQuery::Overview {
                roots_offset: 0,
                impacts_offset: 0,
                limit: 16,
            }),
        }),
    )
    .unwrap() else {
        panic!("overview inspection returned the wrong result shape");
    };
    let ConfigurationImprovementResponse::Details {
        context: returned_context,
        change_roots,
        impacts,
        ..
    } = *result
    else {
        panic!("overview inspection did not return details");
    };
    assert_eq!(returned_context, *context);
    let roots = serde_json::to_value(change_roots).unwrap();
    let impacts = serde_json::to_value(impacts).unwrap();
    let entries = impacts["entries"].as_array().unwrap();
    let source_subject = entries
        .iter()
        .find(|entry| entry["subject"]["kind"] == "source")
        .unwrap_or_else(|| entries.first().expect("the impact partition is non-empty"));
    let selector: SubjectSelector =
        serde_json::from_value(source_subject["subject"].clone()).unwrap();
    let subject =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::from(selector))
            .unwrap();
    (
        roots["entries"].as_array().unwrap().len(),
        entries.len(),
        subject,
    )
}

#[test]
fn configuration_improvement_is_one_coordinate_fenced_non_effectful_lifecycle() {
    let mut store = profile_store();
    let initial = store.baseline_snapshot();
    let worker = AnalysisWorker::spawn(
        PaleApi::new().unwrap(),
        "pale-workspace.toml".to_owned(),
        Some("machine".to_owned()),
        initial.clone(),
    );
    admit_snapshot(&worker, 1, initial);
    let context = ProfileInspectionContext {
        project_id: "demo".to_owned(),
        execution_workspace_id: "default".to_owned(),
    };
    let baseline = admitted_profile(&worker, 20, 1, 0, &context);

    assert!(matches!(
        query_at(
            &worker,
            21,
            1,
            0,
            WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Evaluate {
                context: context.clone(),
                expected_baseline: baseline.clone(),
            },),
        ),
        Err(QueryFailure::ImprovementCoordinateMismatch)
    ));

    let explained = begin_review(&worker, 22, 1, 0, &context, baseline.clone());
    let mut unlike = explained.clone();
    unlike.selected_topology_digest = "sha256:unlike-topology".to_owned();
    assert!(matches!(
        query_at(
            &worker,
            23,
            1,
            0,
            WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Evaluate {
                context: context.clone(),
                expected_baseline: unlike,
            },),
        ),
        Err(QueryFailure::ImprovementCoordinateMismatch)
    ));

    for (index, (request, expected)) in [
        (
            ImprovementReviewDisposition::Accept,
            pale_sdk::ConfigurationImprovementDisposition::Accepted,
        ),
        (
            ImprovementReviewDisposition::Reject,
            pale_sdk::ConfigurationImprovementDisposition::Rejected,
        ),
        (
            ImprovementReviewDisposition::Cancel,
            pale_sdk::ConfigurationImprovementDisposition::Cancelled,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let request_id = 30 + i32::try_from(index).unwrap() * 5;
        let explained = begin_review(&worker, request_id, 1, 0, &context, baseline.clone());
        let preview = preview_review(&worker, request_id + 1, 1, 0, &context, explained);
        if index == 0 {
            let mut unlike = preview.clone();
            unlike.derivation_receipt_digest = "sha256:unlike-preview".to_owned();
            assert!(matches!(
                query_at(
                    &worker,
                    request_id + 2,
                    1,
                    0,
                    WorkspaceQuery::ConfigurationImprovement(
                        ConfigurationImprovementQuery::Inspect {
                            context: context.clone(),
                            expected_preview: unlike,
                            detail: Box::new(ConfigurationImprovementDetailQuery::Overview {
                                roots_offset: 0,
                                impacts_offset: 0,
                                limit: 16,
                            }),
                        },
                    ),
                ),
                Err(QueryFailure::ImprovementCoordinateMismatch)
            ));
        }
        let (root_count, impact_count, subject) =
            inspect_review_overview(&worker, request_id + 2, 1, 0, &context, preview.clone());
        assert_eq!(root_count, 0);
        assert!(impact_count > 0);
        let (explanation, occurrence_count) = inspect_review_subject(
            &worker,
            request_id + 3,
            1,
            0,
            &context,
            preview.clone(),
            subject,
        );
        assert_eq!(explanation["state"], "unaffected");
        assert!(occurrence_count > 0);
        assert_eq!(
            finish_review(&worker, request_id + 4, 1, 0, &context, preview, request,),
            expected,
        );
    }

    let explained = begin_review(&worker, 40, 1, 0, &context, baseline.clone());
    let machine_uri = uri("file:///workspace/machines/machine.toml");
    store
        .open(
            machine_uri.clone(),
            "pale-toml".to_owned(),
            1,
            "not valid TOML =".to_owned(),
        )
        .unwrap();
    let rejected_candidate = store.snapshot();
    worker
        .analyze(AnalysisJob {
            generation: 2,
            snapshot: rejected_candidate.clone(),
        })
        .unwrap();
    let WorkerResult::Analysis(rejected) = worker.results.recv().unwrap() else {
        panic!("rejected analysis must still return its exact result");
    };
    assert!(rejected.outcome.is_err());
    assert!(matches!(
        query_at(
            &worker,
            41,
            2,
            rejected_candidate.revision(),
            WorkspaceQuery::ConfigurationImprovement(ConfigurationImprovementQuery::Evaluate {
                context: context.clone(),
                expected_baseline: explained,
            },),
        ),
        Err(QueryFailure::AnalysisUnavailable)
    ));

    let repaired_machine = format!(
        "{}\n# editor-only comment\n",
        captured_fixture!("private/machine-binding.toml")
    );
    store
        .change(
            &machine_uri,
            2,
            vec![lsp_types::TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: repaired_machine,
            }],
        )
        .unwrap();
    let repaired = store.snapshot();
    admit_snapshot(&worker, 3, repaired.clone());
    let repaired_baseline = admitted_profile(&worker, 42, 3, repaired.revision(), &context);
    let explained = begin_review(
        &worker,
        43,
        3,
        repaired.revision(),
        &context,
        repaired_baseline,
    );
    let preview = preview_review(&worker, 44, 3, repaired.revision(), &context, explained);

    store
        .change(
            &machine_uri,
            3,
            vec![lsp_types::TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: format!(
                    "{}\n# a later editor generation\n",
                    captured_fixture!("private/machine-binding.toml")
                ),
            }],
        )
        .unwrap();
    let successor = store.snapshot();
    admit_snapshot(&worker, 4, successor.clone());
    assert_eq!(
        finish_review(
            &worker,
            45,
            4,
            successor.revision(),
            &context,
            preview,
            ImprovementReviewDisposition::Accept,
        ),
        pale_sdk::ConfigurationImprovementDisposition::Stale,
    );
    worker.stop().unwrap();
}

#[path = "analysis/pages.rs"]
mod pages;
