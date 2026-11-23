//! Workspace failure source-coordinate tests.

use std::collections::BTreeMap;
use std::path::Path;

use super::*;
use crate::workspace::AuthoringInstancePathSegment;

#[test]
fn typed_decode_location_preserves_a_numeric_map_key_as_a_field() {
    #[derive(Debug, serde::Deserialize)]
    struct Record {
        #[serde(rename = "values")]
        _values: BTreeMap<String, u64>,
    }

    let mut deserializer = serde_json::Deserializer::from_str(r#"{"values":{"123":"invalid"}}"#);
    let source = serde_path_to_error::deserialize::<_, Record>(&mut deserializer)
        .expect_err("fixture must fail typed decoding");
    let error = WorkspaceError::Decode {
        coordinate: Box::new(WorkspaceSourceCoordinate::path_only(Path::new(
            "numeric-map.toml",
        ))),
        record: "numeric map",
        value_path: source.path().to_string(),
        source: Box::new(source),
    };

    let location = error
        .authoring_location()
        .expect("structured decoder path is exact");
    assert_eq!(location.source_label(), "numeric-map.toml");
    assert_eq!(
        location.path(),
        &[
            AuthoringInstancePathSegment::Field("values".into()),
            AuthoringInstancePathSegment::Field("123".into()),
        ]
    );
}
