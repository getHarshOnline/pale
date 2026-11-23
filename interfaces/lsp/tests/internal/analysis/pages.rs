//! The real analysis worker links Package headers using their retained ranges.

use super::*;
use crate::protocol::{AuthoringQuery, AuthoringQueryResult};
use lsp_types::{GotoDefinitionResponse, Position};

#[test]
fn worker_navigates_between_page_headers_without_using_body_lookalikes() {
    let api = PaleApi::new().unwrap();
    let manifest = captured_fixture!("packages/engine/package.toml")
        .split("sources =")
        .next()
        .unwrap();
    let page = "+++\ndescription = 'Engineering.'\nuses = [{ scope = 'local', resource_id = 'context' }]\n[role]\n+++\n# Engineering\nuses = [{ scope = 'local', resource_id = 'context' }]\n";
    let context =
        "+++\ndescription = 'Read the context.'\n[guide]\n+++\n# Context\nRead the source.\n";
    let sources = [
        (
            "engine/package.toml",
            DocumentRole::PackageManifest,
            manifest,
        ),
        ("engine/engine.md", DocumentRole::PackagePage, page),
        ("engine/context.md", DocumentRole::PackagePage, context),
    ];
    let inspected = api
        .inspect_package_distribution(PackageDistributionRequest::new(
            sources
                .iter()
                .map(|(label, _, text)| {
                    CompilationDocument::new((*label).into(), text.as_bytes().to_vec())
                })
                .collect(),
        ))
        .unwrap();
    let original = profile_store().baseline_snapshot();
    let mut documents = original
        .documents()
        .iter()
        .filter(|document| !document.role().is_package())
        .map(|document| BaselineDocument {
            uri: document.uri().clone(),
            language_id: "pale-toml".into(),
            role: document.role(),
            text: if document.logical_label() == "distribution-lock.toml" {
                inspected.lockfile_toml().into()
            } else {
                document.text().to_string()
            },
        })
        .collect::<Vec<_>>();
    documents.extend(sources.into_iter().map(|(label, role, text)| {
        BaselineDocument {
            uri: uri(&format!("file:///workspace/{label}")),
            language_id: if role == DocumentRole::PackagePage {
                "pale-page"
            } else {
                "pale-toml"
            }
            .into(),
            role,
            text: text.into(),
        }
    }));
    let store = DocumentStore::new(&uri("file:///workspace"), documents).unwrap();
    let snapshot = store.baseline_snapshot();
    let worker = AnalysisWorker::spawn(
        api,
        "pale-workspace.toml".into(),
        Some("machine".into()),
        snapshot.clone(),
    );
    admit_snapshot(&worker, 1, snapshot);
    admitted_profile(
        &worker,
        200,
        1,
        0,
        &ProfileInspectionContext {
            project_id: "demo".into(),
            execution_workspace_id: "default".into(),
        },
    );
    for (request, line, expected) in [(201, 2, true), (202, 6, false)] {
        let params = serde_json::from_value(serde_json::json!({
            "textDocument": {"uri": "file:///workspace/engine/engine.md"},
            "position": Position::new(line, 43),
        }))
        .unwrap();
        let result = query_at(
            &worker,
            request,
            1,
            0,
            WorkspaceQuery::Authoring(AuthoringQuery::Definition(params)),
        )
        .unwrap();
        match result {
            WorkspaceQueryResult::Authoring(AuthoringQueryResult::Definition(Some(
                GotoDefinitionResponse::Scalar(location),
            ))) if expected => {
                assert_eq!(location.uri, uri("file:///workspace/engine/context.md"));
                assert_eq!(location.range.start.line, 2);
            }
            WorkspaceQueryResult::Authoring(AuthoringQueryResult::Definition(None))
                if !expected => {}
            _ => panic!("unexpected header navigation result"),
        }
    }
    worker.stop().unwrap();
}
