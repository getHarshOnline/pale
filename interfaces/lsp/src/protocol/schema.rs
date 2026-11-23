//! Development-only schemas for the private editor transport.
//!
//! These describe serde shapes in the direction in which each value travels.
//! They do not replace ID parsing, admission, or request-relative client checks.

use super::*;
use schemars::{generate::SchemaSettings, JsonSchema, Schema, SchemaGenerator};
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::BTreeMap;

/// Describe exactly the decimal strings emitted by the u64 coordinate serializers.
///
/// Input records retain their separate parsing contract. The qualified format
/// checks canonical spelling and the u64 maximum without converting to Number.
pub(super) fn u64_decimal(_generator: &mut SchemaGenerator) -> Schema {
    schemars::json_schema!({
        "type": "string",
        "format": "uint64-decimal",
        "minLength": 1,
        "maxLength": 20
    })
}

// The schema delegates to the very structs used by custom serialization. The
// internal u64 fields must never appear as JSON numbers in the editor protocol.
impl JsonSchema for ExpectedProfileCoordinate {
    fn schema_name() -> Cow<'static, str> {
        "ExpectedProfileCoordinateOutput".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        ExpectedProfileCoordinateOutput::json_schema(generator)
    }
}

impl JsonSchema for EditorCoordinate {
    fn schema_name() -> Cow<'static, str> {
        "EditorCoordinateOutput".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        EditorCoordinateOutput::json_schema(generator)
    }
}

/// Keep shared definitions in their selected serialization direction.
///
/// Properties are exported type aliases, not fields of an aggregate message.
fn catalog(generator: &mut SchemaGenerator, roots: BTreeMap<&'static str, Schema>) -> Value {
    let required = roots.keys().copied().collect::<Vec<_>>();
    // Pinned Draft 2020-12 settings have no transforms; the owning test fences
    // that choice for inline roots as well as definitions. Do not mix contracts.
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": roots,
        "required": required,
        "additionalProperties": false,
        "$defs": generator.take_definitions(true)
    })
}

/// Export the existing private transport roots with their exact directions.
pub(crate) fn export() -> Value {
    let mut request_generator = SchemaSettings::draft2020_12()
        .for_deserialize()
        .into_generator();
    let mut response_generator = SchemaSettings::draft2020_12()
        .for_serialize()
        .into_generator();
    let mut requests = BTreeMap::new();
    let mut responses = BTreeMap::new();
    let mut methods = BTreeMap::new();
    macro_rules! method {
        ($method:expr, $request_name:literal, $request:ty, $response_name:literal, $response:ty) => {
            requests.insert($request_name, request_generator.subschema_for::<$request>());
            responses.insert($response_name, response_generator.subschema_for::<$response>());
            methods.insert($method, json!({"request": $request_name, "response": $response_name}));
        };
    }

    // decode_empty explicitly accepts null as well as the closed empty object.
    // Option is only the schema expression of that existing decode branch.
    method!(
        COMPILATION_SUMMARY_METHOD,
        "CompilationSummaryRequest",
        Option<EmptyParams>,
        "CompilationSummaryResponse",
        pale_sdk::PortableWorkspaceCompilationSummary
    );
    method!(
        COMPARISON_SUMMARY_METHOD,
        "ComparisonSummaryRequest",
        Option<EmptyParams>,
        "ComparisonSummaryResponse",
        pale_sdk::PortableWorkspaceComparisonSummary
    );
    method!(
        CHANGE_ROOTS_METHOD,
        "ChangeRootsRequest",
        PageParams,
        "ChangeRootsResponse",
        ChangeRootPage
    );
    method!(
        IMPACTS_METHOD,
        "ImpactsRequest",
        PageParams,
        "ImpactsResponse",
        ImpactPage
    );
    method!(
        EXPLANATION_METHOD,
        "ExplanationRequest",
        ExplanationParams,
        "ExplanationResponse",
        Explanation
    );
    method!(
        PROFILE_CONTEXTS_METHOD,
        "ProfileContextsRequest",
        Option<EmptyParams>,
        "ProfileContextsResponse",
        ProfileContextsResponse
    );
    method!(
        EFFECTIVE_PROFILE_METHOD,
        "EffectiveProfileRequest",
        EffectiveProfileParams,
        "EffectiveProfileResponse",
        EffectiveProfileResponse
    );
    method!(
        EFFECTIVE_PROFILE_EXPLANATION_METHOD,
        "EffectiveProfileExplanationRequest",
        EffectiveProfileExplanationParams,
        "EffectiveProfileExplanationResponse",
        EffectiveProfileExplanationResponse
    );
    method!(
        CONFIGURATION_IMPROVEMENT_METHOD,
        "ConfigurationImprovementRequest",
        ConfigurationImprovementParams,
        "ConfigurationImprovementResponse",
        ConfigurationImprovementResponse
    );

    requests.insert(
        "InitializationOptions",
        crate::server::initialization_options_schema(&mut request_generator),
    );
    requests.insert(
        "BaselineDocument",
        request_generator.subschema_for::<crate::document::BaselineDocument>(),
    );
    requests.insert(
        "DocumentRole",
        request_generator.subschema_for::<crate::document::DocumentRole>(),
    );
    responses.insert(
        "AnalysisPublication",
        response_generator.subschema_for::<AnalysisPublication>(),
    );
    responses.insert(
        "PaleWorkspaceCapabilities",
        response_generator.subschema_for::<PaleWorkspaceCapabilities>(),
    );

    json!({
        "protocolId": PROTOCOL_ID,
        "methods": methods,
        "notifications": {ANALYSIS_PUBLISHED_NOTIFICATION: "AnalysisPublication"},
        "initialization": "InitializationOptions",
        "advertisement": "PaleWorkspaceCapabilities",
        "schemas": {
            "Requests": catalog(&mut request_generator, requests),
            "Responses": catalog(&mut response_generator, responses)
        }
    })
}
