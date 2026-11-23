//! Immutable contract resources exposed by the MCP leaf.

use rmcp::model::{
    CacheScope, ErrorData as McpError, ListResourceTemplatesResult, ListResourcesResult,
    ReadResourceResponse, ReadResourceResult, Resource, ResourceContents, ResourceTemplate,
};

use pale_cli::command::{
    ApiCapabilityId, ApiError, CommandDispatcher, CommandOutcome, CommandRequest, DiagnosticV1,
};

/// API-owned capabilities projected by the immutable resource carrier.
pub(super) const CAPABILITY_IDS: &[ApiCapabilityId] = &[
    ApiCapabilityId::ContractBundleIdentity,
    ApiCapabilityId::ContractSchemaLookup,
];

/// Stable URI of the embedded contract-bundle coordinate.
pub(super) const BUNDLE_URI: &str = "pale://contracts/bundle";
/// Prefix for exact generated schema resources.
pub(super) const SCHEMA_URI_PREFIX: &str = "pale://contracts/schema/";
/// Private freshness hint for immutable process-local contract resources.
const RESOURCE_CACHE_TTL_MS: u64 = 5 * 60 * 1_000;

/// List the fixed bundle resource without duplicating the API schema allowlist.
pub(super) fn list() -> ListResourcesResult {
    ListResourcesResult::with_all_items(vec![Resource::new(BUNDLE_URI, "Pale contract bundle")
        .with_description("Exact embedded descriptor-manifest and schema-set coordinate.")
        .with_mime_type("application/json")])
    .with_ttl_ms(RESOURCE_CACHE_TTL_MS)
    .with_cache_scope(CacheScope::Private)
}

/// Advertise typed lookup for API-selected generated schemas.
pub(super) fn templates() -> ListResourceTemplatesResult {
    ListResourceTemplatesResult::with_all_items(vec![ResourceTemplate::new(
        format!("{SCHEMA_URI_PREFIX}{{schema_id}}"),
        "Pale generated contract schema",
    )
    .with_description("Exact embedded JSON Schema selected by API registry identity.")
    .with_mime_type("application/schema+json")])
    .with_ttl_ms(RESOURCE_CACHE_TTL_MS)
    .with_cache_scope(CacheScope::Private)
}

/// Read one immutable bundle or schema resource through the dispatcher.
pub(super) fn read(
    dispatcher: &CommandDispatcher,
    uri: &str,
) -> Result<ReadResourceResponse, McpError> {
    let (text, mime_type) = if uri == BUNDLE_URI {
        let outcome = dispatcher
            .dispatch(CommandRequest::ContractBundleIdentity)
            .map_err(api_resource_error)?;
        let CommandOutcome::ContractBundleIdentity(identity) = outcome else {
            return Err(incompatible_outcome());
        };
        (
            String::from_utf8(identity.canonical_bytes().to_vec())
                .map_err(|_| incompatible_outcome())?,
            "application/json",
        )
    } else if let Some(schema_id) = uri.strip_prefix(SCHEMA_URI_PREFIX) {
        if schema_id.is_empty() || schema_id.contains('/') {
            return Err(not_found(uri, None));
        }
        let outcome = dispatcher
            .dispatch(CommandRequest::ContractSchema(schema_id.to_owned()))
            .map_err(api_resource_error)?;
        let CommandOutcome::ContractSchema(schema) = outcome else {
            return Err(incompatible_outcome());
        };
        (
            String::from_utf8(schema.bytes().to_vec()).map_err(|_| incompatible_outcome())?,
            "application/schema+json",
        )
    } else {
        return Err(not_found(uri, None));
    };

    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(text, uri).with_mime_type(mime_type)
    ])
    .with_ttl_ms(RESOURCE_CACHE_TTL_MS)
    .with_cache_scope(CacheScope::Private)
    .into())
}

fn api_resource_error(error: ApiError) -> McpError {
    diagnostic_resource_error(error.diagnostic())
}

/// Carry the exact API-admitted diagnostic inside the MCP resource error.
///
/// The resource transport does not inspect the diagnostic disposition or
/// derive a second failure class. The API-owned canonical object remains the
/// sole semantic diagnostic; [`McpError`] supplies only protocol framing.
pub(super) fn diagnostic_resource_error(diagnostic: &DiagnosticV1) -> McpError {
    let data = serde_json::from_slice(diagnostic.canonical_bytes())
        .expect("DiagnosticV1 canonical bytes are valid JSON");
    McpError::internal_error("Pale contract resource lookup failed", Some(data))
}

fn not_found(message: impl Into<String>, data: Option<serde_json::Value>) -> McpError {
    McpError::resource_not_found(message.into(), data)
}

fn incompatible_outcome() -> McpError {
    McpError::internal_error(
        "Pale command dispatcher returned an incompatible contract resource outcome",
        None,
    )
}
