//! Definition mapping over an actually compiled, wide target document.

use super::*;
use crate::syntax::MAX_AUTHORING_SYNTAX_FACTS;
use pale_sdk::AuthoringInstancePathSegment;

fn wide_capabilities(selected_first: bool) -> (String, usize) {
    // Derive the width from the real tolerant parser, not an assumed number of
    // fields or facts per policy. Replacing one scalar preserves its shape.
    let row = AuthoringSyntax::parse(CAPTURED_CAPABILITIES);
    assert!(row.is_complete());
    let row_facts = row.facts().len();
    assert!(row_facts > 0);
    let padding_count = MAX_AUTHORING_SYNTAX_FACTS / row_facts + 1;
    let mut text = String::new();
    if selected_first {
        text.push_str(CAPTURED_CAPABILITIES);
        text.push('\n');
    }
    for index in 0..padding_count {
        let padding =
            CAPTURED_CAPABILITIES.replace("\"standard\"", &format!("\"padding-{index:04}\""));
        if index == 0 {
            let padding_syntax = AuthoringSyntax::parse(&padding);
            assert!(padding_syntax.is_complete());
            assert_eq!(padding_syntax.facts().len(), row_facts);
        }
        text.push_str(&padding);
        text.push('\n');
    }
    if !selected_first {
        text.push_str(CAPTURED_CAPABILITIES);
    }
    (text, if selected_first { 0 } else { padding_count })
}

#[test]
fn definition_rejects_incomplete_target_syntax_after_actual_effective_admission() {
    for selected_first in [false, true] {
        let (capabilities, selected_index) = wide_capabilities(selected_first);
        // This existing fixture uses the public portable and effective profile
        // compilation APIs. No publisher proof or test admission hook is added.
        let (snapshot, syntax, compilation, workspace_uri, _) =
            effective_fixture_with_capabilities(&capabilities);
        let query_syntax = syntax.document("pale-workspace.toml").unwrap();
        let target_syntax = syntax.document("capabilities.toml").unwrap();
        assert!(query_syntax.is_complete());
        assert!(!target_syntax.is_complete());
        assert_eq!(target_syntax.facts().len(), MAX_AUTHORING_SYNTAX_FACTS);
        let query_document = snapshot
            .documents()
            .iter()
            .find(|document| document.uri() == &workspace_uri)
            .unwrap();
        let api = PaleApi::new().unwrap();
        let position = position_of(CAPTURED_ROOT, "standard");
        let navigation = navigation(
            query_document,
            query_syntax,
            AuthoringSemanticContext {
                portable: None,
                effective: Some(&compilation),
                api: &api,
            },
            position,
        )
        .unwrap()
        .expect("the admitted owner resolves the selected policy");
        let target = navigation
            .definition()
            .expect("the admitted owner retains a definition");
        assert_eq!(target.source_label(), "capabilities.toml");
        assert_eq!(
            target.path(),
            &[
                AuthoringInstancePathSegment::Field("run_policies".to_owned()),
                AuthoringInstancePathSegment::Index(selected_index),
                AuthoringInstancePathSegment::Field("id".to_owned()),
            ]
        );
        assert_eq!(
            target_syntax
                .fact_for_instance_path(target.path())
                .is_some(),
            selected_first
        );
        // The predecessor's Option mapper silently lost the late target. The
        // early target also must not turn an incomplete index into success.
        assert_eq!(
            source_location(&snapshot, &syntax, target).is_some(),
            selected_first
        );
        let params = serde_json::from_value(serde_json::json!({
            "textDocument": {"uri": workspace_uri}, "position": position
        }))
        .unwrap();
        assert!(matches!(
            execute_query!(
                AuthoringQuery::Definition(params),
                &snapshot,
                &syntax,
                None,
                Some(&compilation),
                StaleAuthoringCompilation::default(),
                &[],
                AuthoringClientCapabilities::default(),
                &api,
            ),
            Err(AuthoringQueryError::SyntaxFactLimitExceeded)
        ));
    }
}

#[test]
fn document_origin_definition_needs_no_complete_target_syntax() {
    let (capabilities, _) = wide_capabilities(false);
    let (snapshot, syntax, compilation, workspace_uri, capabilities_uri) =
        effective_fixture_with_capabilities(&capabilities);
    assert!(!syntax.document("capabilities.toml").unwrap().is_complete());
    let params = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": workspace_uri},
        "position": position_of(CAPTURED_ROOT, "capabilities.toml")
    }))
    .unwrap();
    let result = execute_query!(
        AuthoringQuery::Definition(params),
        &snapshot,
        &syntax,
        None,
        Some(&compilation),
        StaleAuthoringCompilation::default(),
        &[],
        AuthoringClientCapabilities::default(),
        &PaleApi::new().unwrap(),
    )
    .unwrap();
    let AuthoringQueryResult::Definition(Some(GotoDefinitionResponse::Scalar(location))) = result
    else {
        panic!("source-document definition must retain its exact origin");
    };
    assert_eq!(location.uri, capabilities_uri);
    assert_eq!(
        location.range,
        Range::new(Position::new(0, 0), Position::new(0, 0))
    );
}
