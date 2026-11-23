//! Verifies typed workspace decoding retains precise, source-safe coordinates.

use serde::Deserialize;

use super::{decode_json, source_coordinate};
use crate::workspace::WorkspaceError;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct NestedRecord {
    #[serde(rename = "groups")]
    _groups: Vec<NestedGroup>,
}

#[derive(Debug, Deserialize)]
struct NestedGroup {
    #[serde(rename = "enabled")]
    _enabled: bool,
}

#[test]
fn typed_decode_retains_the_exact_nested_value_path_and_source() {
    let path = Path::new("/private/workspace/capabilities.toml");
    let error = decode_json::<NestedRecord>(
        serde_json::json!({"groups": [{"enabled": true}, {"enabled": 7}]}),
        path,
        "nested fixture",
    )
    .expect_err("the second enabled flag is not Boolean");

    match &error {
        WorkspaceError::Decode {
            coordinate,
            record,
            value_path,
            source,
        } => {
            assert_eq!(coordinate.path(), path);
            assert_eq!(coordinate.line(), None);
            assert_eq!(*record, "nested fixture");
            assert_eq!(value_path, "groups[1].enabled");
            assert_eq!(source.path().to_string(), *value_path);
        }
        other => panic!("expected typed decode failure, got {other:?}"),
    }
    assert!(format!("{error}").contains("groups[1].enabled"));
    assert_eq!(
        error.failure_disposition(),
        crate::ApplicationFailureDisposition::InvalidInput
    );
}

#[test]
fn source_coordinates_count_unicode_columns_from_real_parser_spans() {
    let path = Path::new("/private/workspace/configuration.toml");
    let text = "title = \"λ\"\nvalue = [\n";
    let mut parser_error =
        toml::from_str::<toml::Value>(text).expect_err("the array is deliberately unterminated");
    let span = parser_error.span().expect("TOML parser reports a span");
    let coordinate = source_coordinate(path, text, Some(span.clone()));
    parser_error.set_input(None);

    assert_eq!(coordinate.path(), path);
    assert_eq!(coordinate.span(), Some(&span));
    assert!(coordinate.line().is_some());
    assert!(coordinate.column().is_some());
    assert!(!format!("{parser_error:?}").contains("title"));
}
