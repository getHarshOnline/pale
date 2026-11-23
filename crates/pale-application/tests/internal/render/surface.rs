//! Storage-policy witnesses for generated content.

use std::sync::Arc;

use super::*;

#[test]
fn application_staging_shares_rendered_storage_without_exposing_pointer_identity() {
    let surface = GeneratedSurface::whole_file(
        ".pale/generated.md",
        SurfaceKind::FrontDoor,
        "retained content",
        Vec::new(),
        Vec::new(),
        true,
    );

    let shared = surface.shared_content();

    assert!(Arc::ptr_eq(&surface.content, &shared));
    assert_eq!(surface.content(), shared.as_str());
}

#[test]
fn generated_json_document_uses_two_space_indentation_and_one_line_feed() {
    let document = crate::render::semantic_helpers::json_pretty(
        &serde_json::json!({"z": 1, "a": [2, 1]}),
        "test JSON document",
    )
    .expect("render readable JSON");

    assert_eq!(
        document,
        "{\n  \"a\": [\n    2,\n    1\n  ],\n  \"z\": 1\n}\n"
    );
}
