//! Private Go-bridge request-admission laws.

use super::*;

mod authoring_pages;
mod compiler_lifecycle;
mod control_storage;
mod diagnostic;
mod fixtures;
mod improvement;
mod navigation_custody;
mod observation_custody;
mod observation_profile;
mod one_shot_custody;
mod one_shot_responses;
mod profile_custody;

#[test]
fn local_configuration_review_request_is_strict_and_complete() {
    let valid = br#"{"baselineManifest":"/baseline/pale-workspace.toml","candidateManifest":"/candidate/pale-workspace.toml","projectId":"demo","executionWorkspaceId":"default","machineId":"machine"}"#;
    let request = decode::<wire::LocalConfigurationImprovementRequest>(valid)
        .expect("the exact local request must decode");
    assert_eq!(request.baseline_manifest, "/baseline/pale-workspace.toml");
    assert_eq!(request.candidate_manifest, "/candidate/pale-workspace.toml");
    for invalid in [
        br#"{"baselineManifest":"/baseline","candidateManifest":"/candidate","projectId":"demo","executionWorkspaceId":"default","machineId":"machine","extra":true}"#.as_slice(),
        br#"{"baselineManifest":"/baseline","projectId":"demo","executionWorkspaceId":"default","machineId":"machine"}"#.as_slice(),
        br#"{"baselineManifest":42,"candidateManifest":"/candidate","projectId":"demo","executionWorkspaceId":"default","machineId":"machine"}"#.as_slice(),
    ] {
        let error = decode::<wire::LocalConfigurationImprovementRequest>(invalid)
            .err().expect("unknown, missing, or wrongly typed fields must fail");
        assert_eq!(error.code, "invalid_binding_request");
    }
}

#[test]
fn request_decoder_rejects_unknown_top_level_fields() {
    let failure = decode::<wire::SchemaRequest>(br#"{"schemaId":"diagnostic.v1","extra":true}"#)
        .err()
        .expect("unknown top-level fields must not cross the private ABI");

    assert_eq!(failure.code, "invalid_binding_request");
    assert!(failure.message.contains("unknown field `extra`"));
}

#[test]
fn request_decoder_rejects_unknown_nested_fields() {
    let failure = decode::<wire::PortableRequest>(
        br#"{"rootManifest":"pale-workspace.toml","documents":[{"label":"pale-workspace.toml","bytes":"","extra":true}]}"#,
    )
    .err()
    .expect("unknown nested fields must not cross the private ABI");

    assert_eq!(failure.code, "invalid_binding_request");
    assert!(failure.message.contains("unknown field `extra`"));
}

#[test]
fn request_decoder_rejects_unknown_tagged_variant_fields() {
    let failure = decode::<wire::WorkspaceExplanationRequest>(
        br#"{"handle":1,"subject":{"kind":"source","sourceId":"source","extra":true}}"#,
    )
    .err()
    .expect("unknown tagged-union fields must not cross the private ABI");

    assert_eq!(failure.code, "invalid_binding_request");
    assert!(failure.message.contains("unknown field `extra`"));
}
