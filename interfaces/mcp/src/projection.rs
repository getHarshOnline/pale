//! Protocol-only decoding of API-owned canonical JSON results.

use rmcp::model::{CallToolResult, ErrorData as McpError};

/// Decode exact API-owned JSON bytes at the unavoidable MCP wire edge.
pub(super) fn structured_bytes(bytes: &[u8]) -> Result<CallToolResult, McpError> {
    serde_json::from_slice(bytes)
        .map(CallToolResult::structured)
        .map_err(|_| {
            McpError::internal_error(
                "Pale API result could not be projected as structured MCP content",
                None,
            )
        })
}
