//! Structural projection tests for private-workspace binding records.

use pale_core::records::{
    CapabilitiesFile, DistributionLockFile, MachineBindingFile, SessionModel,
    WorkspaceManifestFile, MAX_WORKSPACE_COMPONENTS, MAX_WORKSPACE_MODULES,
};

fn projected_schema<T: schemars::JsonSchema>() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("binding schema is JSON")
}

fn required_fields<'a>(value: &'a serde_json::Value, pointer: &str) -> Vec<&'a str> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect()
}

fn contains_null_type(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Array(values) => values.iter().any(contains_null_type),
        serde_json::Value::Object(entries) => {
            entries.get("type") == Some(&serde_json::json!("null"))
                || entries.values().any(contains_null_type)
        }
        _ => false,
    }
}

#[test]
fn workspace_projection_retains_initial_v1_bounds_and_canonical_graph() {
    let workspace = projected_schema::<WorkspaceManifestFile>();
    assert_eq!(
        workspace.pointer("/$defs/WorkspaceMeta/properties/schema_version/const"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        workspace.pointer("/$defs/WorkspaceId/minLength"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        workspace.pointer("/$defs/WorkspaceId/maxLength"),
        Some(&serde_json::json!(255))
    );
    assert_eq!(
        workspace.pointer("/$defs/WorkspaceMeta/properties/module_manifests/maxItems"),
        Some(&serde_json::json!(MAX_WORKSPACE_MODULES))
    );
    assert_eq!(
        workspace.pointer("/$defs/WorkspaceMeta/properties/source_components/maxItems"),
        Some(&serde_json::json!(MAX_WORKSPACE_COMPONENTS))
    );
    assert_eq!(
        workspace.pointer(
            "/$defs/WorkspaceDesiredTopologyV1/properties/project_repositories/uniqueItems"
        ),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        workspace.pointer(
            "/$defs/WorkspaceDesiredTopologyV1/properties/execution_workspace_repositories/uniqueItems"
        ),
        Some(&serde_json::json!(true))
    );
    assert!(workspace["$defs"].get("ProjectSetBinding").is_none());
    assert!(workspace["$defs"].get("ExecutionRoute").is_none());
}

#[test]
fn distribution_projection_retains_pins_and_nonempty_coordinates() {
    let schema = projected_schema::<DistributionLockFile>();
    assert_eq!(
        schema.pointer("/$defs/DistributionMeta/properties/compiler/minLength"),
        Some(&serde_json::json!(1))
    );
    assert!(schema
        .pointer("/$defs/PackageReleaseV1/properties/id/$ref")
        .is_some());
    assert_eq!(
        schema.pointer("/$defs/PackageIdV1/minLength"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        schema.pointer("/$defs/PackageVersionV1/minLength"),
        Some(&serde_json::json!(1))
    );
    assert!(
        required_fields(&schema, "/$defs/PackageReleaseV1/required").contains(&"content_digest")
    );
    assert!(schema["$defs"]["DistributionMeta"]["properties"]
        .get("catalog_digest")
        .is_none());
}

#[test]
fn capability_projection_preserves_unbound_as_list_absence() {
    let schema = projected_schema::<CapabilitiesFile>();
    assert!(!required_fields(&schema, "/required").contains(&"operation_bindings"));
    assert!(schema["properties"]["operation_bindings"]
        .get("minProperties")
        .is_none());
    assert_eq!(
        schema.pointer("/properties/operation_bindings/type"),
        Some(&serde_json::json!("array"))
    );
    assert!(schema
        .pointer("/$defs/OperationBindingEntryV1/properties/operation")
        .is_some());
    assert_eq!(
        schema.pointer("/$defs/RepositoryCommandCapabilities/properties/commands/minProperties"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        schema.pointer("/$defs/NonEmptyCommandList/uniqueItems"),
        Some(&serde_json::json!(true))
    );
    assert!(schema["properties"].get("tool_recipe").is_none());
    assert!(schema["properties"].get("capability_policy").is_none());
    assert!(schema["properties"].get("capability_narrowing").is_some());
    assert!(schema["properties"].get("lifecycle").is_some());
}

#[test]
fn machine_projection_retains_exact_local_binding_coordinates() {
    let schema = projected_schema::<MachineBindingFile>();
    assert_eq!(
        schema.pointer("/$defs/MachineMeta/properties/schema_version/const"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        schema.pointer("/$defs/SurfaceExecutableBinding/properties/protocol/const"),
        Some(&serde_json::json!("pale.agent-surface-interface.v1"))
    );
    assert_eq!(
        schema.pointer("/$defs/NativeHarnessBinding/properties/version/maxLength"),
        Some(&serde_json::json!(1_024))
    );
    assert_eq!(
        schema.pointer("/properties/repository_checkouts/uniqueItems"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        schema.pointer("/properties/provider_repositories/uniqueItems"),
        Some(&serde_json::json!(true))
    );
    assert!(schema["$defs"].get("OrchestratorAdapterBinding").is_none());
    assert!(!contains_null_type(
        &schema["properties"]["surface_executable"]
    ));
}

#[test]
fn session_runtime_is_optional_but_never_nullable() {
    let schema = projected_schema::<SessionModel>();
    assert!(!required_fields(&schema, "/required").contains(&"runtime"));
    assert_eq!(
        schema.pointer("/properties/runtime/$ref"),
        Some(&serde_json::json!("#/$defs/SessionRuntimeSelection"))
    );
    assert!(!contains_null_type(&schema["properties"]["runtime"]));
    assert!(schema["properties"]["runtime"].get("default").is_none());
}
