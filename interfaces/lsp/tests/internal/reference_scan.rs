//! Actual compiler references, bounded paging and exact editor mapping.

use super::*;
use crate::document::{BaselineDocument, DocumentRole, DocumentStore};
use pale_sdk::{PortableWorkspaceDocument, PortableWorkspaceRequest};

const ROOT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/portable/pale-workspace.toml"
));
const RESOLUTION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/portable/resolution.lock.toml"
));
const UNIVERSE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/portable/universe.lock.toml"
));

/// Shared real portable input for scan, worker and transport tests.
pub(crate) fn reference_store(count: usize) -> (DocumentStore, lsp_types::ReferenceParams) {
    let projects = (0..count).map(|index| format!("{{ id = \"project-{index:04}\", title = \"Synthetic 😀 project\", run_policy = \"standard\", package_seeds = [], authority_root_ids = [], capabilities_file = \"capabilities.toml\" }}")).collect::<Vec<_>>().join(", ");
    // Unrelated trailing occurrences force an empty, nonterminal SDK page.
    let repositories = (0..600)
        .map(|index| format!("{{ id = \"repository-{index:04}\", title = \"Repository\" }}"))
        .collect::<Vec<_>>()
        .join(", ");
    let root = ROOT
        .replace("projects = []", &format!("projects = [{projects}]"))
        .replace(
            ", repositories = []",
            &format!(", repositories = [{repositories}]"),
        );
    store_for_root(root, "standard")
}

fn store_for_root(root: String, needle: &str) -> (DocumentStore, lsp_types::ReferenceParams) {
    let root = format!("\u{feff}{}", root.replace('\n', "\r\n"));
    let mut store = DocumentStore::new(
        &"file:///workspace".parse().unwrap(),
        [
            (
                "pale-workspace.toml",
                DocumentRole::WorkspaceManifest,
                ROOT.to_owned(),
            ),
            (
                "resolution.lock.toml",
                DocumentRole::WorkspaceResolutionLock,
                RESOLUTION.to_owned(),
            ),
            (
                "universe.lock.toml",
                DocumentRole::WorkspaceUniverseLock,
                UNIVERSE.to_owned(),
            ),
        ]
        .into_iter()
        .map(|(label, role, text)| BaselineDocument {
            uri: format!("file:///workspace/{label}").parse().unwrap(),
            role,
            language_id: "pale-toml".to_owned(),
            text,
        })
        .collect(),
    )
    .unwrap();
    let uri: Uri = "file:///workspace/pale-workspace.toml".parse().unwrap();
    store
        .open(uri.clone(), "pale-toml".to_owned(), 1, root.clone())
        .unwrap();
    let byte = root.find(needle).unwrap();
    let prefix = &root[..byte];
    let position = Position::new(
        prefix.matches('\n').count() as u32,
        prefix[prefix.rfind('\n').map_or(0, |index| index + 1)..]
            .encode_utf16()
            .count() as u32,
    );
    let params = serde_json::from_value(serde_json::json!({ "textDocument": { "uri": uri }, "position": position, "context": { "includeDeclaration": false }, "partialResultToken": "references" })).unwrap();
    (store, params)
}

fn compile(
    store: &DocumentStore,
) -> (
    Snapshot,
    AuthoringSyntaxSnapshot,
    PortableWorkspaceCompilation,
) {
    let snapshot = store.snapshot();
    let mut syntax = AuthoringSyntaxSnapshot::default();
    for document in snapshot.documents() {
        syntax.insert(
            document.logical_label().to_owned(),
            AuthoringSyntax::parse(&document.text()),
        );
    }
    let compilation = PaleApi::new()
        .unwrap()
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".to_owned(),
            snapshot
                .documents()
                .iter()
                .map(|doc| {
                    PortableWorkspaceDocument::new(doc.logical_label().to_owned(), doc.bytes())
                })
                .collect(),
        ))
        .unwrap();
    (snapshot, syntax, compilation)
}

#[test]
fn streaming_enumerates_beyond_256_and_preserves_empty_page_and_utf16_laws() {
    let (store, params) = reference_store(300);
    let (snapshot, syntax, compilation) = compile(&store);
    let api = PaleApi::new().unwrap();
    let semantics = AuthoringSemanticContext {
        portable: Some(&compilation),
        effective: None,
        api: &api,
    };
    let mut scan = ReferenceScan::start(&params, &snapshot, &syntax, semantics)
        .unwrap()
        .unwrap();
    let mut locations = Vec::new();
    let mut empty_nonterminal = false;
    let mut complete = false;
    for _ in 0..100 {
        let page = scan.next(&snapshot, &syntax, semantics).unwrap();
        assert!(page.locations.len() <= 256);
        empty_nonterminal |= page.locations.is_empty() && !page.complete;
        locations.extend(page.locations);
        if page.complete {
            complete = true;
            break;
        }
    }
    assert!(complete && empty_nonterminal);
    assert_eq!(locations.len(), 300);
    assert_eq!(scan.seen.len(), 300);
    assert!(scan.seen.len() <= scan.maximum_locations);
    let document = snapshot
        .document(&params.text_document_position.text_document.uri)
        .unwrap();
    for location in &locations {
        let start = document.byte_at_position(location.range.start).unwrap();
        let end = document.byte_at_position(location.range.end).unwrap();
        assert_eq!(&document.text()[start..end], "\"standard\"");
    }
    assert!(matches!(
        super::super::execute(
            AuthoringQuery::References(params),
            AuthoringExecution {
                snapshot: &snapshot,
                syntax: &syntax,
                semantics,
                stale: StaleAuthoringCompilation::default(),
                corrections: &[],
                client: AuthoringClientCapabilities::default()
            }
        ),
        Err(AuthoringQueryError::ReferenceSetTooLarge)
    ));
}

#[test]
fn no_token_keeps_exact_256_raw_reference_boundary() {
    let (store, mut params) = reference_store(256);
    params.partial_result_params.partial_result_token = None;
    let (snapshot, syntax, compilation) = compile(&store);
    let api = PaleApi::new().unwrap();
    let result = super::super::execute(
        AuthoringQuery::References(params),
        AuthoringExecution {
            snapshot: &snapshot,
            syntax: &syntax,
            semantics: AuthoringSemanticContext {
                portable: Some(&compilation),
                effective: None,
                api: &api,
            },
            stale: StaleAuthoringCompilation::default(),
            corrections: &[],
            client: AuthoringClientCapabilities::default(),
        },
    )
    .unwrap();
    let AuthoringQueryResult::References(Some(locations)) = result else {
        panic!("reference result");
    };
    assert_eq!(locations.len(), 256);
}

#[test]
fn no_token_256_references_plus_canonical_declaration_is_allowed_but_257_raw_is_not() {
    for count in [256, 257] {
        let workspaces = (0..count)
            .map(|index| {
                format!(
                    "{{ id = \"ws-{index:04}\", title = \"Workspace\", execution_path_ids = [] }}"
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let memberships = (0..count)
            .map(|index| {
                format!("{{ project_id = \"demo\", execution_workspace_id = \"ws-{index:04}\" }}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        let root = ROOT.replace("projects = []", "projects = [{ id = \"demo\", title = \"Demo\", run_policy = \"standard\", package_seeds = [], authority_root_ids = [], capabilities_file = \"capabilities.toml\" }]")
            .replace(", execution_workspaces = []", &format!(", execution_workspaces = [{workspaces}]"))
            .replace(", project_execution_workspaces = []", &format!(", project_execution_workspaces = [{memberships}]"));
        let (store, mut params) = store_for_root(root, "demo");
        params.partial_result_params.partial_result_token = None;
        params.context.include_declaration = true;
        let (snapshot, syntax, compilation) = compile(&store);
        let api = PaleApi::new().unwrap();
        let result = super::super::execute(
            AuthoringQuery::References(params),
            AuthoringExecution {
                snapshot: &snapshot,
                syntax: &syntax,
                semantics: AuthoringSemanticContext {
                    portable: Some(&compilation),
                    effective: None,
                    api: &api,
                },
                stale: StaleAuthoringCompilation::default(),
                corrections: &[],
                client: AuthoringClientCapabilities::default(),
            },
        );
        if count == 256 {
            let AuthoringQueryResult::References(Some(locations)) = result.unwrap() else {
                panic!("references");
            };
            assert_eq!(locations.len(), 257);
        } else {
            assert!(matches!(
                result,
                Err(AuthoringQueryError::ReferenceSetTooLarge)
            ));
        }
    }
}

#[test]
fn effective_scan_reuses_canonical_definition_and_fails_missing_target_mapping() {
    let (snapshot, syntax, compilation, _, uri) = crate::authoring::tests::effective_fixture();
    let api = PaleApi::new().unwrap();
    let document = snapshot.document(&uri).unwrap();
    let text = document.text();
    let byte = text.find("standard").unwrap();
    let position = document.range_at_bytes(byte, byte).unwrap().start;
    let params = serde_json::from_value(serde_json::json!({ "textDocument": { "uri": uri }, "position": position, "context": { "includeDeclaration": true }, "partialResultToken": "effective" })).unwrap();
    let semantics = AuthoringSemanticContext {
        portable: None,
        effective: Some(&compilation),
        api: &api,
    };
    let mut scan = ReferenceScan::start(&params, &snapshot, &syntax, semantics)
        .unwrap()
        .unwrap();
    let first = scan.next(&snapshot, &syntax, semantics).unwrap();
    assert_eq!(first.locations.len(), 1);
    assert!(!first.complete);
    let mut all = first.locations;
    for _ in 0..100 {
        let page = scan.next(&snapshot, &syntax, semantics).unwrap();
        all.extend(page.locations);
        if page.complete {
            break;
        }
    }
    let ordinary = super::super::execute(
        AuthoringQuery::References(params.clone()),
        AuthoringExecution {
            snapshot: &snapshot,
            syntax: &syntax,
            semantics,
            stale: StaleAuthoringCompilation::default(),
            corrections: &[],
            client: AuthoringClientCapabilities::default(),
        },
    )
    .unwrap();
    let AuthoringQueryResult::References(Some(expected)) = ordinary else {
        panic!("reference result");
    };
    let keys = |values: Vec<Location>| {
        values
            .into_iter()
            .map(|value| {
                (
                    value.uri.to_string(),
                    value.range.start.line,
                    value.range.start.character,
                    value.range.end.line,
                    value.range.end.character,
                )
            })
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(keys(all), keys(expected));
    let mut broken = syntax.clone();
    broken.insert("pale-workspace.toml".to_owned(), AuthoringSyntax::parse(""));
    let mut scan = ReferenceScan::start(&params, &snapshot, &broken, semantics)
        .unwrap()
        .unwrap();
    let mut rejected = false;
    for _ in 0..100 {
        match scan.next(&snapshot, &broken, semantics) {
            Err(AuthoringQueryError::SourceMappingUnavailable) => {
                rejected = true;
                break;
            }
            Ok(page) if !page.complete => {}
            _ => break,
        }
    }
    assert!(
        rejected,
        "missing target facts cannot produce successful completion"
    );
}

#[test]
fn numeric_seen_ceiling_checks_new_keys_but_allows_duplicates_at_the_ceiling() {
    let (store, params) = reference_store(1);
    let (snapshot, syntax, compilation) = compile(&store);
    let api = PaleApi::new().unwrap();
    let mut scan = ReferenceScan::start(
        &params,
        &snapshot,
        &syntax,
        AuthoringSemanticContext {
            portable: Some(&compilation),
            effective: None,
            api: &api,
        },
    )
    .unwrap()
    .unwrap();
    let expected = snapshot
        .documents()
        .iter()
        .map(|doc| syntax.document(doc.logical_label()).unwrap().facts().len() + 1)
        .sum::<usize>();
    assert_eq!(scan.maximum_locations, expected);
    // Inject a smaller storage ceiling to exercise the same owning insertion guard.
    scan.maximum_locations = 1;
    let key = (0, 0, 0, 0, 1);
    assert!(scan.retain(key).unwrap());
    assert!(!scan.retain(key).unwrap());
    assert!(matches!(
        scan.retain((0, 0, 1, 0, 2)),
        Err(AuthoringQueryError::ReferenceLocationBound)
    ));
    assert_eq!(scan.seen.len(), 1);
}

/// Capture published source bytes; the supplied lock selects adoption separately.
fn populated_source_store(universe: &str) -> DocumentStore {
    macro_rules! populated {
        ($member:literal) => {
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/workspace-language/populated/",
                $member
            ))
        };
    }
    let sources = [
        (
            "pale-workspace.toml",
            DocumentRole::WorkspaceManifest,
            populated!("pale-workspace.toml"),
        ),
        (
            "module.toml",
            DocumentRole::WorkspaceModule,
            populated!("module.toml"),
        ),
        (
            "alpha.toml",
            DocumentRole::WorkspaceLanguagePrimitives,
            populated!("alpha.toml"),
        ),
        (
            "beta.toml",
            DocumentRole::WorkspaceLanguagePrimitives,
            populated!("beta.toml"),
        ),
        (
            "resolution.lock.toml",
            DocumentRole::WorkspaceResolutionLock,
            populated!("resolution.lock.toml"),
        ),
        (
            "universe.lock.toml",
            DocumentRole::WorkspaceUniverseLock,
            universe,
        ),
    ];
    DocumentStore::new(
        &"file:///workspace".parse().unwrap(),
        sources
            .into_iter()
            .map(|(label, role, text)| BaselineDocument {
                uri: format!("file:///workspace/{label}").parse().unwrap(),
                role,
                language_id: "pale-toml".to_owned(),
                text: text.to_owned(),
            })
            .collect(),
    )
    .unwrap()
}

#[test]
fn published_source_fixture_requires_unavailable_publisher_proof() {
    let store = populated_source_store(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/workspace-language/populated/universe.lock.toml"
    )));
    let snapshot = store.snapshot();
    let error = PaleApi::new()
        .unwrap()
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".to_owned(),
            snapshot
                .documents()
                .iter()
                .map(|doc| {
                    PortableWorkspaceDocument::new(doc.logical_label().to_owned(), doc.bytes())
                })
                .collect(),
        ))
        .expect_err("source capture cannot supply publisher admission");
    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.code(), "workspace_owner_proof_unavailable");
    assert_eq!(
        diagnostic.stage(),
        pale_sdk::DiagnosticStage::SemanticAdmission
    );
    assert_eq!(
        diagnostic.disposition(),
        pale_sdk::DiagnosticDisposition::Blocked
    );
    assert_eq!(
        diagnostic.retry(),
        pale_sdk::DiagnosticRetry::AfterStateChange
    );
    assert_eq!(
        diagnostic.subject(),
        Some(("publisher", "candidate-publisher"))
    );
    let facts = diagnostic.facts();
    assert_eq!(
        facts
            .iter()
            .map(|fact| (fact.key(), fact.value()))
            .collect::<Vec<_>>(),
        vec![("release", "release-1")]
    );
}

#[test]
fn unadopted_source_projections_match_navigation_without_claiming_release_membership() {
    // The shared conformance lock resolves sources but adopts no publisher release.
    // These canonical authoring occurrences are not selected proof membership.
    let store = populated_source_store(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/workspace-language/unadopted/universe.lock.toml"
    )));
    let (snapshot, syntax, compilation) = compile(&store);
    let api = PaleApi::new().unwrap();
    let uri: Uri = "file:///workspace/alpha.toml".parse().unwrap();
    let document = snapshot.document(&uri).unwrap();
    let byte = document.text().find("common").unwrap();
    let position = document.range_at_bytes(byte, byte).unwrap().start;
    let semantics = AuthoringSemanticContext {
        portable: Some(&compilation),
        effective: None,
        api: &api,
    };
    let nav = navigation(
        document,
        syntax.document("alpha.toml").unwrap(),
        semantics,
        position,
    )
    .unwrap()
    .unwrap();
    let mut params: lsp_types::ReferenceParams = serde_json::from_value(serde_json::json!({ "textDocument": { "uri": uri }, "position": position, "context": { "includeDeclaration": false }, "partialResultToken": "projection" })).unwrap();
    let mut scan = ReferenceScan::start(&params, &snapshot, &syntax, semantics)
        .unwrap()
        .unwrap();
    let page = scan.next(&snapshot, &syntax, semantics).unwrap();
    assert!(page.complete, "small admitted fixture fits one SDK page");
    let expected = nav
        .references()
        .iter()
        .map(|location| exact_location(&snapshot, &syntax, location).unwrap().0)
        .collect::<BTreeSet<_>>();
    assert_eq!(scan.seen, expected);
    assert!(page
        .locations
        .iter()
        .any(|location| location.uri.as_str().ends_with("/module.toml")));
    let projection = compilation
        .authoring_occurrences(AuthoringOccurrencePageRequest::new(
            nav.identity().unwrap().clone(),
            256,
            256,
        ))
        .unwrap();
    let projection = projection
        .occurrences()
        .iter()
        .find(|row| row.role() == AuthoringOccurrenceRole::Projection)
        .unwrap();
    let source = compilation
        .source_subject(pale_sdk::AuthoringNavigationRequest::new(
            projection.location().source_label().to_owned(),
            projection.location().path().to_vec(),
        ))
        .unwrap();
    let pale_sdk::PortableWorkspaceSourceSubject::Unselected { occurrence, .. } = source.result()
    else {
        panic!("unadopted source projection must not claim selected proof membership");
    };
    assert_eq!(occurrence.role(), AuthoringOccurrenceRole::Projection);
    assert_eq!(occurrence.location(), projection.location());
    assert!(scan.seen.contains(
        &exact_location(&snapshot, &syntax, projection.location())
            .unwrap()
            .0
    ));
    params.context.include_declaration = true;
    let mut declared = ReferenceScan::start(&params, &snapshot, &syntax, semantics)
        .unwrap()
        .unwrap();
    let declaration = declared.next(&snapshot, &syntax, semantics).unwrap();
    assert_eq!(
        declaration.locations,
        vec![
            exact_location(&snapshot, &syntax, nav.definition().unwrap())
                .unwrap()
                .1
        ]
    );
    let missing = DocumentStore::new(&"file:///workspace".parse().unwrap(), Vec::new())
        .unwrap()
        .snapshot();
    assert!(matches!(
        exact_location(&missing, &syntax, projection.location()),
        Err(AuthoringQueryError::SourceMappingUnavailable)
    ));
    let mut incomplete = syntax.clone();
    incomplete.insert(
        "module.toml".to_owned(),
        AuthoringSyntax::parse(
            &(0..crate::syntax::MAX_AUTHORING_SYNTAX_FACTS)
                .map(|index| format!("key_{index} = {index}\n"))
                .collect::<String>(),
        ),
    );
    assert!(matches!(
        exact_location(&snapshot, &incomplete, projection.location()),
        Err(AuthoringQueryError::SyntaxFactLimitExceeded)
    ));
}

#[test]
fn included_source_document_declaration_uses_exact_root_location_and_requires_the_document() {
    let (snapshot, syntax, compilation, uri, capabilities_uri) =
        crate::authoring::tests::effective_fixture();
    let document = snapshot.document(&uri).unwrap();
    let byte = document.text().find("capabilities.toml").unwrap();
    let position = document.range_at_bytes(byte, byte).unwrap().start;
    let api = PaleApi::new().unwrap();
    let semantics = AuthoringSemanticContext {
        portable: None,
        effective: Some(&compilation),
        api: &api,
    };
    let params = serde_json::from_value(serde_json::json!({ "textDocument": { "uri": uri }, "position": position, "context": { "includeDeclaration": true }, "partialResultToken": "root" })).unwrap();
    let nav = navigation(
        document,
        syntax.document(document.logical_label()).unwrap(),
        semantics,
        position,
    )
    .unwrap()
    .unwrap();
    assert!(nav.definition().unwrap().path().is_empty());
    let mut scan = ReferenceScan::start(&params, &snapshot, &syntax, semantics)
        .unwrap()
        .unwrap();
    let declaration = scan.next(&snapshot, &syntax, semantics).unwrap();
    assert_eq!(
        declaration.locations,
        vec![Location::new(
            capabilities_uri,
            Range::new(Position::new(0, 0), Position::new(0, 0))
        )]
    );
    let missing = DocumentStore::new(&"file:///workspace".parse().unwrap(), Vec::new())
        .unwrap()
        .snapshot();
    assert!(matches!(
        exact_location(&missing, &syntax, nav.definition().unwrap()),
        Err(AuthoringQueryError::SourceMappingUnavailable)
    ));
}
