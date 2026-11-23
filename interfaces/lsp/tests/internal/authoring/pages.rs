//! Header completion retains source positions and leaves Markdown alone.

use super::*;

fn page_fixture(text: &str) -> (Snapshot, AuthoringSyntaxSnapshot, Uri) {
    let document_uri = uri("file:///workspace/review.md");
    let snapshot = DocumentStore::new(
        &uri("file:///workspace"),
        vec![BaselineDocument {
            uri: document_uri.clone(),
            language_id: "pale-page".into(),
            role: DocumentRole::PackagePage,
            text: text.into(),
        }],
    )
    .unwrap()
    .snapshot();
    let mut syntax = AuthoringSyntaxSnapshot::default();
    let range = StandaloneAuthoringRole::PackagePage
        .declaration_range(text)
        .unwrap();
    syntax.insert(
        "review.md".into(),
        AuthoringSyntax::parse_region(text, range).unwrap(),
    );
    (snapshot, syntax, document_uri)
}

fn complete(text: &str, position: Position) -> Vec<CompletionItem> {
    let (snapshot, syntax, document_uri) = page_fixture(text);
    let params = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": document_uri}, "position": position,
    }))
    .unwrap();
    let result = execute_query!(
        AuthoringQuery::Completion(params),
        &snapshot,
        &syntax,
        None,
        None,
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &PaleApi::new().unwrap()
    )
    .unwrap();
    let AuthoringQueryResult::Completion(Some(CompletionResponse::Array(items))) = result else {
        panic!("completion must return its normal item list");
    };
    items
}

#[test]
fn page_header_completion_keeps_crlf_positions_and_never_edits_markdown() {
    let text = "+++\r\ndescription = 'Review λ.'\r\n[guide]\r\nexp = \r\n+++\r\n# Review\r\n[guide]\r\nexp = \r\n";
    let items = complete(text, Position::new(3, 3));
    let item = items.iter().find(|item| item.label == "exposure").unwrap();
    let Some(CompletionTextEdit::Edit(edit)) = &item.text_edit else {
        panic!("key completion needs an edit");
    };
    assert_eq!(
        edit.range,
        Range::new(Position::new(3, 0), Position::new(3, 3))
    );
    assert!(complete(text, Position::new(7, 3)).is_empty());
    assert!(complete(text, Position::new(0, 0)).is_empty());
    assert!(complete(text, Position::new(4, 0)).is_empty());
}

#[test]
fn unclosed_page_header_offers_literals_without_admitting_the_page() {
    let text = "+++\ndescription = 'Review.'\n[guide]\nexposure = ";
    let items = complete(text, Position::new(3, 11));
    assert!(items.iter().any(|item| item.label == "exported"));
    assert!(items.iter().any(|item| item.label == "internal"));
    assert!(PaleApi::new()
        .unwrap()
        .validate_authoring_document(
            StandaloneAuthoringRole::PackagePage,
            "review.md".into(),
            text.as_bytes().to_vec()
        )
        .is_err());
}
