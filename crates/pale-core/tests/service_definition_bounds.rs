//! Schema locks for the service-definition bound owner.

use pale_core::records::{
    ServiceDefinitionInput, SERVICE_DEFINITION_MAX_ARGUMENTS_V1,
    SERVICE_DEFINITION_MAX_ARGUMENT_BYTES_V1, SERVICE_DEFINITION_MAX_DEPENDENCIES_V1,
    SERVICE_DEFINITION_MAX_KIND_BYTES_V1, SERVICE_DEFINITION_MAX_NAME_BYTES_V1,
    SERVICE_DEFINITION_MAX_PORTS_V1, SERVICE_DEFINITION_MAX_PROGRAM_BYTES_V1,
    SERVICE_DEFINITION_MAX_SERVICES_V1,
};

fn at<'a>(schema: &'a serde_json::Value, pointer: &str) -> &'a serde_json::Value {
    schema
        .pointer(pointer)
        .unwrap_or_else(|| panic!("generated service schema lacks `{pointer}`"))
}

#[test]
fn generated_schema_numeric_ceilings_track_runtime_constants() {
    let schema = serde_json::to_value(schemars::schema_for!(ServiceDefinitionInput))
        .expect("service-definition schema must serialize");
    assert_eq!(
        at(&schema, "/properties/services/maxItems").as_u64(),
        Some(SERVICE_DEFINITION_MAX_SERVICES_V1 as u64)
    );
    assert_eq!(
        at(
            &schema,
            "/$defs/ServiceDefinition/properties/kind/maxLength"
        )
        .as_u64(),
        Some(SERVICE_DEFINITION_MAX_KIND_BYTES_V1 as u64)
    );
    assert_eq!(
        at(
            &schema,
            "/$defs/ServiceDefinition/properties/ports/maxItems"
        )
        .as_u64(),
        Some(SERVICE_DEFINITION_MAX_PORTS_V1 as u64)
    );
    assert_eq!(
        at(
            &schema,
            "/$defs/ServiceDefinition/properties/depends_on/maxItems"
        )
        .as_u64(),
        Some(SERVICE_DEFINITION_MAX_DEPENDENCIES_V1 as u64)
    );
    assert_eq!(
        at(&schema, "/$defs/ServiceLaunch/properties/program/maxLength").as_u64(),
        Some(SERVICE_DEFINITION_MAX_PROGRAM_BYTES_V1 as u64)
    );
    assert_eq!(
        at(&schema, "/$defs/ServiceLaunch/properties/args/maxItems").as_u64(),
        Some(SERVICE_DEFINITION_MAX_ARGUMENTS_V1 as u64)
    );
    assert_eq!(
        at(
            &schema,
            "/$defs/ServiceLaunch/properties/args/items/maxLength"
        )
        .as_u64(),
        Some(SERVICE_DEFINITION_MAX_ARGUMENT_BYTES_V1 as u64)
    );

    let expected_name_pattern = format!(
        "^[a-z][a-z0-9-]{{0,{}}}$",
        SERVICE_DEFINITION_MAX_NAME_BYTES_V1 - 1
    );
    assert_eq!(
        at(&schema, "/$defs/ServiceDefinition/properties/name/pattern").as_str(),
        Some(expected_name_pattern.as_str())
    );
    assert_eq!(
        at(
            &schema,
            "/$defs/ServiceDefinition/properties/depends_on/items/pattern"
        )
        .as_str(),
        Some(expected_name_pattern.as_str())
    );
}
