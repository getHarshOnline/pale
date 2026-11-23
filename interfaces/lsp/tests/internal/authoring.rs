//! Standard authoring projection tests.

use super::*;
use crate::document::{BaselineDocument, DocumentRole, DocumentStore};
use lsp_types::{
    CodeActionOrCommand, CodeActionParams, CompletionParams, DocumentChanges, GotoDefinitionParams,
    OneOf, ReferenceParams,
};
use pale_sdk::{
    CompilationDocument, EffectiveProfileCompilation, EffectiveProfileCompilationRequest,
    OneShotPrivateInputs, PortableWorkspaceDocument, PortableWorkspaceRequest,
};

macro_rules! execute_query {
    (
        $query:expr,
        $snapshot:expr,
        $syntax:expr,
        $portable:expr,
        $effective:expr,
        $stale:expr,
        $corrections:expr,
        $client:expr,
        $api:expr $(,)?
    ) => {
        super::execute(
            $query,
            AuthoringExecution {
                snapshot: $snapshot,
                syntax: $syntax,
                semantics: AuthoringSemanticContext {
                    portable: $portable,
                    effective: $effective,
                    api: $api,
                },
                stale: $stale,
                corrections: $corrections,
                client: $client,
            },
        )
    };
}

const CAPTURED_ROOT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/one-shot/workspace/pale-workspace.toml"
));
const CAPTURED_COMPOSITION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/one-shot/workspace/composition.toml"
));
const CAPTURED_RESOLUTION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/one-shot/workspace/workspace-resolution.lock.toml"
));
const CAPTURED_UNIVERSE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/one-shot/workspace/workspace-universe.lock.toml"
));
const CAPTURED_CAPABILITIES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/one-shot/private/capabilities.toml"
));
const CAPTURED_MACHINE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/one-shot/private/machine-binding.toml"
));
const CAPTURED_DISTRIBUTION_LOCK: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/one-shot/private/distribution-lock.toml"
));
const CAPTURED_PACKAGE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/one-shot/packages/engine/package.toml"
));
const CAPTURED_ROLES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/one-shot/packages/engine/roles.toml"
));
const CAPTURED_ROLE_BODY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/one-shot/packages/engine/role.md"
));

macro_rules! visibility_widening_fixture {
    ($member:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/authoring/visibility-widening/",
            $member
        ))
    };
}

const VISIBILITY_WIDENING_DOCUMENTS: [(&str, &str); 8] = [
    (
        "pale-workspace.toml",
        visibility_widening_fixture!("pale-workspace.toml"),
    ),
    ("module.toml", visibility_widening_fixture!("module.toml")),
    (
        "consumer-module.toml",
        visibility_widening_fixture!("consumer-module.toml"),
    ),
    ("alpha.toml", visibility_widening_fixture!("alpha.toml")),
    ("beta.toml", visibility_widening_fixture!("beta.toml")),
    (
        "consumer.toml",
        visibility_widening_fixture!("consumer.toml"),
    ),
    (
        "resolution.lock.toml",
        visibility_widening_fixture!("resolution.lock.toml"),
    ),
    (
        "universe.lock.toml",
        visibility_widening_fixture!("universe.lock.toml"),
    ),
];

fn uri(value: &str) -> Uri {
    value.parse().expect("test URI is valid")
}

fn fixture(text: &str) -> (Snapshot, AuthoringSyntaxSnapshot, Uri) {
    let document_uri = uri("file:///workspace/module.toml");
    let snapshot = DocumentStore::new(
        &uri("file:///workspace"),
        vec![BaselineDocument {
            uri: document_uri.clone(),
            language_id: "pale-toml".to_owned(),
            role: DocumentRole::WorkspaceModule,
            text: text.to_owned(),
        }],
    )
    .expect("fixture store")
    .snapshot();
    let mut syntax = AuthoringSyntaxSnapshot::default();
    syntax.insert("module.toml".to_owned(), AuthoringSyntax::parse(text));
    (snapshot, syntax, document_uri)
}

fn document(label: &str, text: &str) -> CompilationDocument {
    CompilationDocument::new(label.to_owned(), text.as_bytes().to_vec())
}

fn position_of(text: &str, needle: &str) -> Position {
    let byte = text.find(needle).expect("needle exists in fixture");
    let prefix = &text[..byte];
    let line = prefix.matches('\n').count() as u32;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let character = text[line_start..byte].encode_utf16().count() as u32;
    Position::new(line, character)
}

fn visibility_widening_correction(api: &PaleApi) -> pale_sdk::AuthoringCorrection {
    let request = PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_owned(),
        VISIBILITY_WIDENING_DOCUMENTS
            .iter()
            .map(|(label, text)| {
                PortableWorkspaceDocument::new((*label).to_owned(), text.as_bytes().to_vec())
            })
            .collect(),
    );
    let error = match api.compile_portable_workspace(request) {
        Ok(_) => panic!("the visibility-widening fixture must be rejected"),
        Err(error) => error,
    };
    let [correction] = error.authoring_corrections() else {
        panic!("the rejected fixture must prove exactly one correction");
    };
    correction.clone()
}

fn versioned_consumer_snapshot(
    text: &str,
    version: i32,
) -> (Snapshot, AuthoringSyntaxSnapshot, Uri) {
    let document_uri = uri("file:///workspace/consumer-module.toml");
    let mut store = DocumentStore::new(
        &uri("file:///workspace"),
        vec![BaselineDocument {
            uri: document_uri.clone(),
            language_id: "pale-toml".to_owned(),
            role: DocumentRole::WorkspaceModule,
            text: text.to_owned(),
        }],
    )
    .expect("consumer document store");
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            version,
            text.to_owned(),
        )
        .expect("versioned consumer overlay");
    let mut syntax = AuthoringSyntaxSnapshot::default();
    syntax.insert(
        "consumer-module.toml".to_owned(),
        AuthoringSyntax::parse(text),
    );
    (store.snapshot(), syntax, document_uri)
}

fn code_action_params(
    document_uri: &Uri,
    range: Range,
    correction: &pale_sdk::AuthoringCorrection,
) -> CodeActionParams {
    serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": document_uri},
        "range": range,
        "context": {
            "diagnostics": [{
                "range": range,
                "message": "the import visibility ceiling widens its export",
                "data": {"identity": correction.diagnostic_identity()}
            }]
        }
    }))
    .expect("code-action parameters")
}

#[test]
fn compiler_proved_code_action_is_versioned_exact_deterministic_and_non_effectful() {
    let api = PaleApi::new().expect("API initializes");
    let correction = visibility_widening_correction(&api);
    assert_eq!(
        correction.kind(),
        pale_sdk::AuthoringCorrectionKind::NarrowImportVisibility
    );
    assert_eq!(correction.source_label(), "consumer-module.toml");
    assert_eq!(correction.expected_source(), "\"public\"");
    assert_eq!(correction.replacement_source(), "\"protected\"");

    let text = visibility_widening_fixture!("consumer-module.toml");
    let expected_start = position_of(text, correction.expected_source());
    let expected_range = Range::new(
        expected_start,
        Position::new(
            expected_start.line,
            expected_start.character
                + u32::try_from(correction.expected_source().encode_utf16().count())
                    .expect("the scalar length fits LSP coordinates"),
        ),
    );
    let (snapshot, syntax, document_uri) = versioned_consumer_snapshot(text, 17);
    let document = snapshot
        .document(&document_uri)
        .expect("the exact source document is retained");
    let source = document.text();
    let preimage_start = document
        .byte_at_position(expected_range.start)
        .expect("the correction start is a valid UTF-16 position");
    let preimage_end = document
        .byte_at_position(expected_range.end)
        .expect("the correction end is a valid UTF-16 position");
    assert_eq!(
        source.get(preimage_start..preimage_end),
        Some(correction.expected_source())
    );
    let params = code_action_params(&document_uri, expected_range, &correction);
    let client = AuthoringClientCapabilities {
        versioned_document_changes: true,
        ..AuthoringClientCapabilities::default()
    };

    let first = execute_query!(
        AuthoringQuery::CodeActions(params.clone()),
        &snapshot,
        &syntax,
        None,
        None,
        StaleAuthoringCompilation::default(),
        std::slice::from_ref(&correction),
        client,
        &api,
    )
    .expect("code-action projection succeeds");
    let second = execute_query!(
        AuthoringQuery::CodeActions(params),
        &snapshot,
        &syntax,
        None,
        None,
        StaleAuthoringCompilation::default(),
        std::slice::from_ref(&correction),
        client,
        &api,
    )
    .expect("repeated code-action projection succeeds");
    let AuthoringQueryResult::CodeActions(Some(actions)) = first else {
        panic!("the exact correction must produce one code action");
    };
    let AuthoringQueryResult::CodeActions(Some(repeated)) = second else {
        panic!("the repeated correction must produce one code action");
    };
    assert_eq!(
        serde_json::to_value(&actions).expect("first response serializes"),
        serde_json::to_value(repeated).expect("repeated response serializes")
    );
    let [CodeActionOrCommand::CodeAction(action)] = actions.as_slice() else {
        panic!("the correction must project exactly one native code action");
    };
    assert_eq!(action.kind, Some(lsp_types::CodeActionKind::QUICKFIX));
    assert_eq!(action.is_preferred, Some(true));
    assert_eq!(
        action.data,
        Some(serde_json::json!({
            "diagnosticIdentity": correction.diagnostic_identity(),
            "proofSubject": correction.proof_subject().value(),
        }))
    );
    assert!(
        action.command.is_none(),
        "the action must not execute an effect"
    );
    assert_eq!(
        action
            .diagnostics
            .as_ref()
            .expect("the matching diagnostic is retained")
            .len(),
        1
    );
    let edit = action
        .edit
        .as_ref()
        .expect("the action carries a preview edit");
    assert!(edit.changes.is_none());
    let Some(DocumentChanges::Edits(document_edits)) = &edit.document_changes else {
        panic!("the action must use versioned document changes");
    };
    let [document_edit] = document_edits.as_slice() else {
        panic!("the action must edit exactly one document");
    };
    assert_eq!(document_edit.text_document.uri, document_uri);
    assert_eq!(document_edit.text_document.version, Some(17));
    let [OneOf::Left(text_edit)] = document_edit.edits.as_slice() else {
        panic!("the action must contain one exact text edit");
    };
    assert_eq!(text_edit.range, expected_range);
    assert_eq!(text_edit.new_text, correction.replacement_source());
    assert_eq!(
        document.text(),
        text,
        "projecting an action must not mutate the open buffer"
    );
}

#[test]
fn code_action_refuses_unversioned_or_changed_preimage_state() {
    let api = PaleApi::new().expect("API initializes");
    let correction = visibility_widening_correction(&api);
    let text = visibility_widening_fixture!("consumer-module.toml");
    let expected_start = position_of(text, correction.expected_source());
    let expected_range = Range::new(
        expected_start,
        Position::new(expected_start.line, expected_start.character + 8),
    );
    let (snapshot, syntax, document_uri) = versioned_consumer_snapshot(text, 17);
    let params = code_action_params(&document_uri, expected_range, &correction);

    let unversioned = execute_query!(
        AuthoringQuery::CodeActions(params.clone()),
        &snapshot,
        &syntax,
        None,
        None,
        StaleAuthoringCompilation::default(),
        std::slice::from_ref(&correction),
        AuthoringClientCapabilities::default(),
        &api,
    )
    .expect("unversioned client refusal is a valid response");
    assert!(matches!(
        unversioned,
        AuthoringQueryResult::CodeActions(None)
    ));

    let changed = text.replacen(
        "visibility_ceiling = \"public\"",
        "visibility_ceiling = \"private\"",
        1,
    );
    let (changed_snapshot, changed_syntax, changed_uri) = versioned_consumer_snapshot(&changed, 18);
    let stale = execute_query!(
        AuthoringQuery::CodeActions(code_action_params(
            &changed_uri,
            expected_range,
            &correction,
        )),
        &changed_snapshot,
        &changed_syntax,
        None,
        None,
        StaleAuthoringCompilation::default(),
        std::slice::from_ref(&correction),
        AuthoringClientCapabilities {
            versioned_document_changes: true,
            ..AuthoringClientCapabilities::default()
        },
        &api,
    )
    .expect("stale correction refusal is a valid response");
    assert!(matches!(stale, AuthoringQueryResult::CodeActions(None)));
}

pub(crate) fn effective_fixture() -> (
    Snapshot,
    AuthoringSyntaxSnapshot,
    EffectiveProfileCompilation,
    Uri,
    Uri,
) {
    effective_fixture_with_capabilities(CAPTURED_CAPABILITIES)
}

fn effective_fixture_with_capabilities(
    capabilities: &str,
) -> (
    Snapshot,
    AuthoringSyntaxSnapshot,
    EffectiveProfileCompilation,
    Uri,
    Uri,
) {
    effective_fixture_with_machine(capabilities, CAPTURED_MACHINE)
}

fn effective_fixture_with_machine(
    capabilities: &str,
    machine_binding: &str,
) -> (
    Snapshot,
    AuthoringSyntaxSnapshot,
    EffectiveProfileCompilation,
    Uri,
    Uri,
) {
    let sources = [
        (
            "pale-workspace.toml",
            DocumentRole::WorkspaceManifest,
            CAPTURED_ROOT,
        ),
        (
            "composition.toml",
            DocumentRole::ExperienceComposition,
            CAPTURED_COMPOSITION,
        ),
        (
            "capabilities.toml",
            DocumentRole::Capabilities,
            capabilities,
        ),
        (
            "machines/machine.toml",
            DocumentRole::MachineBinding,
            machine_binding,
        ),
    ];
    let baseline = sources
        .iter()
        .map(|(label, role, text)| BaselineDocument {
            uri: uri(&format!("file:///workspace/{label}")),
            language_id: "pale-toml".to_owned(),
            role: *role,
            text: (*text).to_owned(),
        })
        .collect();
    let snapshot = DocumentStore::new(&uri("file:///workspace"), baseline)
        .expect("populated document store")
        .snapshot();
    let mut syntax = AuthoringSyntaxSnapshot::default();
    for (label, _, text) in sources {
        syntax.insert(label.to_owned(), AuthoringSyntax::parse(text));
    }
    let api = PaleApi::new().expect("API initializes");
    let workspace = api
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".to_owned(),
            [
                ("pale-workspace.toml", CAPTURED_ROOT),
                ("composition.toml", CAPTURED_COMPOSITION),
                ("workspace-resolution.lock.toml", CAPTURED_RESOLUTION),
                ("workspace-universe.lock.toml", CAPTURED_UNIVERSE),
            ]
            .into_iter()
            .map(|(label, text)| {
                PortableWorkspaceDocument::new(label.to_owned(), text.as_bytes().to_vec())
            })
            .collect(),
        ))
        .expect("captured workspace compiles");
    let compilation = api
        .compile_effective_profile(
            &workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".to_owned(),
                "default".to_owned(),
                "machine".to_owned(),
                OneShotPrivateInputs::new(
                    document("capabilities.toml", capabilities),
                    document("machines/machine.toml", machine_binding),
                    document("distribution-lock.toml", CAPTURED_DISTRIBUTION_LOCK),
                    Vec::new(),
                ),
                [
                    ("engine/package.toml", CAPTURED_PACKAGE),
                    ("engine/roles.toml", CAPTURED_ROLES),
                    ("engine/role.md", CAPTURED_ROLE_BODY),
                ]
                .into_iter()
                .map(|(label, text)| document(label, text))
                .collect(),
            ),
        )
        .expect("captured effective profile compiles");
    (
        snapshot,
        syntax,
        compilation,
        uri("file:///workspace/pale-workspace.toml"),
        uri("file:///workspace/capabilities.toml"),
    )
}

#[test]
fn provider_projection_reuses_cross_file_navigation_and_effective_profile_meaning() {
    let machine_binding = format!(
        "{CAPTURED_MACHINE}\n[[provider_repositories]]\nrepository_id = \"engine\"\nprovider_binding_id = \"provider-binding-claude-code\"\nnative_repository_coordinate = \"owner://rig/repository\"\n"
    );
    let (snapshot, syntax, compilation, _, _) =
        effective_fixture_with_machine(CAPTURED_CAPABILITIES, &machine_binding);
    let machine_uri = uri("file:///workspace/machines/machine.toml");
    let composition_uri = uri("file:///workspace/composition.toml");
    let api = PaleApi::new().expect("API initializes");

    let definition_params: GotoDefinitionParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": machine_uri},
        "position": position_of(&machine_binding, "provider-binding-claude-code")
    }))
    .expect("definition parameters");
    let definition = execute_query!(
        AuthoringQuery::Definition(definition_params),
        &snapshot,
        &syntax,
        None,
        Some(&compilation),
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &api,
    )
    .expect("provider definition succeeds");
    let AuthoringQueryResult::Definition(Some(GotoDefinitionResponse::Scalar(definition))) =
        definition
    else {
        panic!("definition returned the wrong result shape");
    };
    assert_eq!(definition.uri, composition_uri);

    let reference_params: ReferenceParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": composition_uri},
        "position": position_of(CAPTURED_COMPOSITION, "provider-binding-claude-code"),
        "context": {"includeDeclaration": true}
    }))
    .expect("reference parameters");
    let references = execute_query!(
        AuthoringQuery::References(reference_params),
        &snapshot,
        &syntax,
        None,
        Some(&compilation),
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &api,
    )
    .expect("provider references succeed");
    let AuthoringQueryResult::References(Some(references)) = references else {
        panic!("references returned the wrong result shape");
    };
    assert!(references
        .iter()
        .any(|reference| reference.uri == machine_uri));

    let hover_params: lsp_types::HoverParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": machine_uri},
        "position": position_of(&machine_binding, "provider-binding-claude-code")
    }))
    .expect("hover parameters");
    let hover = execute_query!(
        AuthoringQuery::Hover(hover_params),
        &snapshot,
        &syntax,
        None,
        Some(&compilation),
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities {
            hover_markup: AuthoringMarkupKind::Markdown,
            ..AuthoringClientCapabilities::default()
        },
        &api,
    )
    .expect("provider hover succeeds");
    let AuthoringQueryResult::Hover(Some(hover)) = hover else {
        panic!("hover returned the wrong result shape");
    };
    let HoverContents::Markup(content) = hover.contents else {
        panic!("hover must use negotiated markup content");
    };
    assert!(content.value.contains("owner://rig/repository"));
    assert!(content
        .value
        .contains("owner-native realization remains unknown"));
}

#[test]
fn rejected_incomplete_value_still_receives_generated_literal_completion() {
    let text = "[[module.exports]]\nvisibility = ";
    let (snapshot, syntax, document_uri) = fixture(text);
    let params: CompletionParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": document_uri},
        "position": {"line": 1, "character": 13}
    }))
    .expect("completion parameters");
    let result = execute_query!(
        AuthoringQuery::Completion(params),
        &snapshot,
        &syntax,
        None,
        None,
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &PaleApi::new().expect("API initializes"),
    )
    .expect("generated metadata is coherent");
    let AuthoringQueryResult::Completion(Some(CompletionResponse::Array(items))) = result else {
        panic!("completion returned the wrong result shape");
    };
    assert_eq!(
        items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        vec!["private", "protected", "public"]
    );
    assert!(items.iter().all(|item| item.text_edit.is_some()));
}

#[test]
fn incomplete_syntax_index_fails_standard_queries_instead_of_returning_partial_results() {
    let text = (0..crate::syntax::MAX_AUTHORING_SYNTAX_FACTS)
        .map(|index| format!("key_{index} = {index}\n"))
        .collect::<String>();
    let (snapshot, syntax, document_uri) = fixture(&text);
    let params: lsp_types::DocumentSymbolParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": document_uri}
    }))
    .expect("document-symbol parameters");

    assert!(matches!(
        execute_query!(
            AuthoringQuery::DocumentSymbols(params),
            &snapshot,
            &syntax,
            None,
            None,
            StaleAuthoringCompilation::default(),
            &[],
            AuthoringClientCapabilities::default(),
            &PaleApi::new().expect("API initializes"),
        ),
        Err(AuthoringQueryError::SyntaxFactLimitExceeded)
    ));
}

#[test]
fn complete_standard_response_overflow_is_rejected_without_truncation() {
    let text = "[[module.exports]]\n".repeat(MAX_STANDARD_AUTHORING_RESULTS + 1);
    let (snapshot, syntax, document_uri) = fixture(&text);
    assert!(syntax
        .document("module.toml")
        .expect("fixture syntax")
        .is_complete());
    let params: lsp_types::DocumentSymbolParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": document_uri}
    }))
    .expect("document-symbol parameters");

    assert!(matches!(
        execute_query!(
            AuthoringQuery::DocumentSymbols(params),
            &snapshot,
            &syntax,
            None,
            None,
            StaleAuthoringCompilation::default(),
            &[],
            AuthoringClientCapabilities::default(),
            &PaleApi::new().expect("API initializes"),
        ),
        Err(AuthoringQueryError::StandardResponseTooLarge)
    ));
}

#[test]
fn completion_replaces_partial_values_instead_of_appending_to_them() {
    let text = "[[module.exports]]\nvisibility = pro";
    let (snapshot, syntax, document_uri) = fixture(text);
    let params: CompletionParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": document_uri},
        "position": {"line": 1, "character": 16}
    }))
    .expect("completion parameters");
    let result = execute_query!(
        AuthoringQuery::Completion(params),
        &snapshot,
        &syntax,
        None,
        None,
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &PaleApi::new().expect("API initializes"),
    )
    .expect("generated metadata is coherent");
    let AuthoringQueryResult::Completion(Some(CompletionResponse::Array(items))) = result else {
        panic!("completion returned the wrong result shape");
    };
    let item = items
        .iter()
        .find(|item| item.label == "protected")
        .expect("protected completion");
    let Some(CompletionTextEdit::Edit(edit)) = &item.text_edit else {
        panic!("completion must carry an exact edit");
    };
    assert_eq!(
        edit.range,
        Range::new(Position::new(1, 13), Position::new(1, 16))
    );
    assert_eq!(edit.new_text, "\"protected\"");
}

#[test]
fn completion_replaces_the_active_key_without_hiding_its_canonical_field() {
    let text = "[[module.exports]]\nvis = ";
    let (snapshot, syntax, document_uri) = fixture(text);
    let params: CompletionParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": document_uri},
        "position": {"line": 1, "character": 3}
    }))
    .expect("completion parameters");
    let result = execute_query!(
        AuthoringQuery::Completion(params),
        &snapshot,
        &syntax,
        None,
        None,
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &PaleApi::new().expect("API initializes"),
    )
    .expect("generated metadata is coherent");
    let AuthoringQueryResult::Completion(Some(CompletionResponse::Array(items))) = result else {
        panic!("completion returned the wrong result shape");
    };
    let item = items
        .iter()
        .find(|item| item.label == "visibility")
        .expect("visibility completion");
    let Some(CompletionTextEdit::Edit(edit)) = &item.text_edit else {
        panic!("completion must carry an exact edit");
    };
    assert_eq!(
        edit.range,
        Range::new(Position::new(1, 0), Position::new(1, 3))
    );
    assert_eq!(edit.new_text, "visibility");
}

#[test]
fn known_keys_and_closed_literals_receive_deterministic_semantic_tokens() {
    let text = "[[module.exports]]\nvisibility = \"protected\"\n";
    let (snapshot, syntax, document_uri) = fixture(text);
    let params: lsp_types::SemanticTokensParams =
        serde_json::from_value(serde_json::json!({"textDocument": {"uri": document_uri}}))
            .expect("semantic-token parameters");
    let result = execute_query!(
        AuthoringQuery::SemanticTokens(params),
        &snapshot,
        &syntax,
        None,
        None,
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &PaleApi::new().expect("API initializes"),
    )
    .expect("generated metadata is coherent");
    let AuthoringQueryResult::SemanticTokens(Some(SemanticTokensResult::Tokens(tokens))) = result
    else {
        panic!("semantic tokens returned the wrong result shape");
    };
    assert!(tokens.data.iter().any(|token| token.token_type == 0));
    assert!(tokens.data.iter().any(|token| token.token_type == 1));
    assert!(tokens.data.iter().any(|token| token.token_type == 2));
    let mut line = 0;
    let mut start = 0;
    let mut prior_line = None;
    let mut prior_end = 0;
    for token in &tokens.data {
        line += token.delta_line;
        start = if token.delta_line == 0 {
            start + token.delta_start
        } else {
            token.delta_start
        };
        if prior_line == Some(line) {
            assert!(
                start >= prior_end,
                "semantic tokens overlap on line {line}: {start} < {prior_end}"
            );
        }
        prior_line = Some(line);
        prior_end = start + token.length;
    }
}

#[test]
fn open_buffer_queries_preserve_bom_crlf_and_astral_utf16_coordinates() {
    let text = "\u{feff}[[module.exports]]\r\nvisibility = \"😀\"\r\n";
    let root = uri("file:///workspace");
    let document_uri = uri("file:///workspace/module.toml");
    let mut store = DocumentStore::new(
        &root,
        vec![BaselineDocument {
            uri: document_uri.clone(),
            language_id: "pale-toml".to_owned(),
            role: DocumentRole::WorkspaceModule,
            text: text.to_owned(),
        }],
    )
    .expect("fixture store");
    store
        .open(
            document_uri.clone(),
            "pale-toml".to_owned(),
            1,
            text.to_owned(),
        )
        .expect("open-buffer overlay");
    let snapshot = store.snapshot();
    let mut syntax = AuthoringSyntaxSnapshot::default();
    syntax.insert("module.toml".to_owned(), AuthoringSyntax::parse(text));
    let api = PaleApi::new().expect("API initializes");

    let token_params: lsp_types::SemanticTokensParams =
        serde_json::from_value(serde_json::json!({"textDocument": {"uri": document_uri}}))
            .expect("semantic-token parameters");
    let token_result = execute_query!(
        AuthoringQuery::SemanticTokens(token_params),
        &snapshot,
        &syntax,
        None,
        None,
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &api,
    )
    .expect("semantic tokens preserve exact source coordinates");
    let AuthoringQueryResult::SemanticTokens(Some(SemanticTokensResult::Tokens(tokens))) =
        token_result
    else {
        panic!("semantic tokens returned the wrong result shape");
    };
    let first = tokens.data.first().expect("header key token");
    assert_eq!((first.delta_line, first.delta_start), (0, 3));

    let hover_params: lsp_types::HoverParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": document_uri},
        "position": {"line": 1, "character": 14}
    }))
    .expect("hover parameters");
    assert!(matches!(
        execute_query!(
            AuthoringQuery::Hover(hover_params),
            &snapshot,
            &syntax,
            None,
            None,
            StaleAuthoringCompilation::default(),
            &[],
            AuthoringClientCapabilities::default(),
            &api,
        ),
        Ok(AuthoringQueryResult::Hover(Some(_)))
    ));

    let invalid_params: lsp_types::HoverParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": document_uri},
        "position": {"line": 1, "character": 15}
    }))
    .expect("invalid-position hover parameters");
    assert!(matches!(
        execute_query!(
            AuthoringQuery::Hover(invalid_params),
            &snapshot,
            &syntax,
            None,
            None,
            StaleAuthoringCompilation::default(),
            &[],
            AuthoringClientCapabilities::default(),
            &api,
        ),
        Err(AuthoringQueryError::InvalidPosition)
    ));
}

#[test]
fn definitions_and_references_use_linker_owned_cross_file_identity() {
    let (snapshot, syntax, compilation, workspace_uri, capabilities_uri) = effective_fixture();
    let workspace_position = position_of(CAPTURED_ROOT, "standard");
    let definition_params: GotoDefinitionParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": workspace_uri},
        "position": workspace_position
    }))
    .expect("definition parameters");
    let result = execute_query!(
        AuthoringQuery::Definition(definition_params),
        &snapshot,
        &syntax,
        None,
        Some(&compilation),
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &PaleApi::new().expect("API initializes"),
    )
    .expect("navigation succeeds");
    let AuthoringQueryResult::Definition(Some(GotoDefinitionResponse::Scalar(location))) = result
    else {
        panic!("definition returned the wrong result shape");
    };
    assert_eq!(location.uri, capabilities_uri);

    let capabilities_position = position_of(CAPTURED_CAPABILITIES, "standard");
    let reference_params: ReferenceParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": capabilities_uri},
        "position": capabilities_position,
        "context": {"includeDeclaration": true}
    }))
    .expect("reference parameters");
    let result = execute_query!(
        AuthoringQuery::References(reference_params),
        &snapshot,
        &syntax,
        None,
        Some(&compilation),
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &PaleApi::new().expect("API initializes"),
    )
    .expect("references succeed");
    let AuthoringQueryResult::References(Some(locations)) = result else {
        panic!("references returned the wrong result shape");
    };
    assert!(locations
        .iter()
        .any(|location| location.uri == workspace_uri));
    assert!(locations
        .iter()
        .any(|location| location.uri == capabilities_uri));
}

#[test]
fn hover_projects_current_effective_profile_without_rederiving_run_policy() {
    let (snapshot, syntax, compilation, workspace_uri, _) = effective_fixture();
    let params: lsp_types::HoverParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": workspace_uri},
        "position": position_of(CAPTURED_ROOT, "standard")
    }))
    .expect("hover parameters");
    let result = execute_query!(
        AuthoringQuery::Hover(params),
        &snapshot,
        &syntax,
        None,
        Some(&compilation),
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities {
            hover_markup: AuthoringMarkupKind::Markdown,
            ..AuthoringClientCapabilities::default()
        },
        &PaleApi::new().expect("API initializes"),
    )
    .expect("effective hover succeeds");
    let AuthoringQueryResult::Hover(Some(hover)) = result else {
        panic!("hover returned the wrong result shape");
    };
    let HoverContents::Markup(content) = hover.contents else {
        panic!("hover must use negotiated markup content");
    };
    assert!(content.value.contains("Resolved run policy: `standard`"));
    assert!(content
        .value
        .contains("Run policy `standard` is the effective private selection."));
}

#[test]
fn semantic_identity_tokens_preserve_definition_role_without_overlap() {
    let (snapshot, syntax, compilation, workspace_uri, _) = effective_fixture();
    let params: lsp_types::SemanticTokensParams =
        serde_json::from_value(serde_json::json!({"textDocument": {"uri": workspace_uri}}))
            .expect("semantic-token parameters");
    let result = execute_query!(
        AuthoringQuery::SemanticTokens(params),
        &snapshot,
        &syntax,
        None,
        Some(&compilation),
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &PaleApi::new().expect("API initializes"),
    )
    .expect("semantic tokens project admitted identity roles");
    let AuthoringQueryResult::SemanticTokens(Some(SemanticTokensResult::Tokens(tokens))) = result
    else {
        panic!("semantic tokens returned the wrong result shape");
    };
    assert!(tokens
        .data
        .iter()
        .any(|token| token.token_type == 3 && token.token_modifiers_bitset == 1));
    assert!(tokens
        .data
        .iter()
        .any(|token| token.token_type == 3 && token.token_modifiers_bitset == 0));
}

#[test]
fn identity_bearing_map_keys_reuse_semantic_candidates_without_fabricated_definitions() {
    let capabilities = format!(
        "{CAPTURED_CAPABILITIES}\n\
[capability_bindings.local-inspection]\n\
effect_class = \"read\"\n\
approval = \"none\"\n\
[capability_bindings.local-inspection.implementation]\n\
kind = \"native_rust\"\n\
target = \"local-inspection\"\n"
    );
    let (snapshot, syntax, compilation, _, capabilities_uri) =
        effective_fixture_with_capabilities(&capabilities);
    let api = PaleApi::new().expect("API initializes");
    let key_position = position_of(&capabilities, "local-inspection");

    let completion_params: CompletionParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": capabilities_uri},
        "position": key_position
    }))
    .expect("completion parameters");
    let completion = execute_query!(
        AuthoringQuery::Completion(completion_params),
        &snapshot,
        &syntax,
        None,
        Some(&compilation),
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &api,
    )
    .expect("map-key completion succeeds");
    let AuthoringQueryResult::Completion(Some(CompletionResponse::Array(items))) = completion
    else {
        panic!("completion returned the wrong result shape");
    };
    let candidate = items
        .iter()
        .find(|item| item.label == "local-inspection")
        .expect("compiler-owned capability candidate");
    assert_eq!(candidate.kind, Some(CompletionItemKind::REFERENCE));
    let Some(CompletionTextEdit::Edit(edit)) = &candidate.text_edit else {
        panic!("identity candidate must carry an exact key replacement");
    };
    assert_eq!(edit.new_text, "local-inspection");

    let token_params: lsp_types::SemanticTokensParams =
        serde_json::from_value(serde_json::json!({"textDocument": {"uri": capabilities_uri}}))
            .expect("semantic-token parameters");
    let tokens = execute_query!(
        AuthoringQuery::SemanticTokens(token_params),
        &snapshot,
        &syntax,
        None,
        Some(&compilation),
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &api,
    )
    .expect("map-key semantic tokens succeed");
    let AuthoringQueryResult::SemanticTokens(Some(SemanticTokensResult::Tokens(tokens))) = tokens
    else {
        panic!("semantic tokens returned the wrong result shape");
    };
    assert!(tokens
        .data
        .iter()
        .any(|token| token.token_type == 3 && token.token_modifiers_bitset == 0));

    let definition_params: GotoDefinitionParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": capabilities_uri},
        "position": key_position
    }))
    .expect("definition parameters");
    assert!(matches!(
        execute_query!(
            AuthoringQuery::Definition(definition_params),
            &snapshot,
            &syntax,
            None,
            Some(&compilation),
            StaleAuthoringCompilation::default(),
            &[],
            AuthoringClientCapabilities::default(),
            &api,
        ),
        Ok(AuthoringQueryResult::Definition(None))
    ));

    let reference_params: ReferenceParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": capabilities_uri},
        "position": key_position,
        "context": {"includeDeclaration": false}
    }))
    .expect("reference parameters");
    let references = execute_query!(
        AuthoringQuery::References(reference_params),
        &snapshot,
        &syntax,
        None,
        Some(&compilation),
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &api,
    )
    .expect("map-key references succeed");
    let AuthoringQueryResult::References(Some(locations)) = references else {
        panic!("references returned the wrong result shape");
    };
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].uri, capabilities_uri);
}

#[test]
fn source_document_definition_targets_the_admitted_document_origin() {
    let (snapshot, syntax, compilation, workspace_uri, capabilities_uri) = effective_fixture();
    let params: GotoDefinitionParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": workspace_uri},
        "position": position_of(CAPTURED_ROOT, "capabilities.toml")
    }))
    .expect("definition parameters");
    let result = execute_query!(
        AuthoringQuery::Definition(params),
        &snapshot,
        &syntax,
        None,
        Some(&compilation),
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &PaleApi::new().expect("API initializes"),
    )
    .expect("source navigation succeeds");
    let AuthoringQueryResult::Definition(Some(GotoDefinitionResponse::Scalar(location))) = result
    else {
        panic!("definition returned the wrong result shape");
    };
    assert_eq!(location.uri, capabilities_uri);
    assert_eq!(
        location.range,
        Range::new(Position::new(0, 0), Position::new(0, 0))
    );
}

#[test]
fn stale_effective_profile_candidates_are_suppressed_without_exact_context() {
    let text = "[workspace.desired_topology]\nproject_repositories = [{ project_id = \"\" }]\n";
    let document_uri = uri("file:///workspace/pale-workspace.toml");
    let snapshot = DocumentStore::new(
        &uri("file:///workspace"),
        vec![BaselineDocument {
            uri: document_uri.clone(),
            language_id: "pale-toml".to_owned(),
            role: DocumentRole::WorkspaceManifest,
            text: text.to_owned(),
        }],
    )
    .expect("fixture store")
    .snapshot();
    let mut syntax = AuthoringSyntaxSnapshot::default();
    syntax.insert(
        "pale-workspace.toml".to_owned(),
        AuthoringSyntax::parse(text),
    );
    let params: CompletionParams = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": document_uri},
        "position": position_of(text, "\"\"")
    }))
    .expect("completion parameters");

    let result = execute_query!(
        AuthoringQuery::Completion(params),
        &snapshot,
        &syntax,
        None,
        None,
        StaleAuthoringCompilation { portable: None },
        &[],
        AuthoringClientCapabilities::default(),
        &PaleApi::new().expect("API initializes"),
    )
    .expect("stale completion projection succeeds");
    let AuthoringQueryResult::Completion(Some(CompletionResponse::Array(items))) = result else {
        panic!("completion returned the wrong result shape");
    };
    assert!(items.iter().all(|item| item.label != "demo"));
}

#[path = "definition_mapping.rs"]
mod definition_mapping;

#[path = "authoring/pages.rs"]
mod pages;
