//! Exercises checked deserialization of digest-bound effective project profiles.

use super::*;

fn profile_preimage() -> EffectiveProjectProfilePreimageV1 {
    EffectiveProjectProfilePreimageV1 {
        schema: SchemaId::parse(EFFECTIVE_PROJECT_PROFILE_SCHEMA).unwrap(),
        schema_sha256: crate::records::SchemaDigest::parse("a".repeat(64)).unwrap(),
        schema_version: EFFECTIVE_PROJECT_PROFILE_SCHEMA_VERSION,
        workspace_id: WorkspaceId::parse("workspace").unwrap(),
        project_id: LogicalProjectIdV1::parse("project").unwrap(),
        execution_workspace_id: LogicalExecutionWorkspaceIdV1::parse("workspace").unwrap(),
        execution_paths: Vec::new(),
        run_policy: RunPolicyEntry {
            id: crate::records::RunPolicyIdV1::parse("inert").unwrap(),
            description: "No lifecycle gate.".into(),
            session_inject: false,
            child_propagation: crate::records::ChildPropagation::Exempt,
            child_type_selectors: Vec::new(),
            action_gate: false,
            gated_tool_matchers: Vec::new(),
        },
        package_closure: Vec::new(),
        normative_pack_closure: Vec::new(),
        package_semantic_ir_digest: PackageSemanticIrDigest::parse("b".repeat(64)).unwrap(),
        authority_root_ids: Vec::new(),
        repositories: Vec::new(),
        governing_order: crate::records::ResolvedGoverningContributionV1::compose(
            Vec::new(),
            &BTreeMap::new(),
        )
        .unwrap(),
        semantic: SemanticSelection::default(),
        capability_narrowing: None,
        capability_bindings: BTreeMap::new(),
        operation_bindings: Vec::new(),
        lifecycle: Vec::new(),
        forbidden_token_authority: Vec::new(),
        session: None,
    }
}

#[test]
fn effective_profile_valid_value_round_trips_through_checked_deserialization() {
    let profile = EffectiveProjectProfileV1::seal(profile_preimage()).unwrap();
    let decoded: EffectiveProjectProfileV1 =
        serde_json::from_value(serde_json::to_value(&profile).unwrap()).unwrap();
    assert_eq!(decoded, profile);
}

#[test]
fn checked_deserialization_rejects_redigested_effective_profile_invalidity() {
    let mut profile = profile_preimage();
    profile.schema_version = 2;
    let profile_digest = ProfileDigest::hash_canonical(&profile).unwrap();
    let mut wire = serde_json::to_value(profile).unwrap();
    wire.as_object_mut().unwrap().insert(
        "profile_digest".into(),
        serde_json::to_value(profile_digest).unwrap(),
    );
    assert!(serde_json::from_value::<EffectiveProjectProfileV1>(wire).is_err());
}
