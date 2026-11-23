//! In-process MCP caller for development conformance tests.
//!
//! The caller runs the same tool registry, command dispatcher, Rust SDK, and
//! resident attachment as the MCP server. It opens no listener and performs no
//! endpoint discovery, peer authentication, key resolution, storage access, or
//! deployment. Tests must provide an attachment whose carrier and response
//! admission enforce those boundaries.

use pale_cli::command::{CommandDispatcher, ResidentAttachment};
use rmcp::model::{CallToolRequestParams, CallToolResponse, CallToolResult};

use crate::server::PaleMcpServer;

/// Failure to construct or call the development MCP projection.
#[derive(Debug, thiserror::Error)]
pub enum McpConformanceError {
    /// The attached command dispatcher could not establish its static surface.
    #[error("initialize attached command dispatcher: {0}")]
    Dispatcher(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The immutable MCP tool projection could not be constructed.
    #[error("project MCP tools: {0}")]
    Projection(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The current-thread development runtime could not be constructed.
    #[error("construct MCP development runtime: {0}")]
    Runtime(#[source] std::io::Error),
    /// MCP rejected the request or failed while its bounded worker ran.
    #[error("call MCP tool: {0}")]
    Protocol(String),
    /// The tool returned an interactive or task continuation.
    #[error("MCP tool returned an unsupported continuation")]
    UnsupportedContinuation,
}

/// Development-only caller for the in-process MCP projection.
///
/// This type exists only with the `interface-test-support` feature. Its methods
/// prove projection through MCP, the CLI dispatcher, and the Rust SDK. They do
/// not prove stdio framing, process isolation, endpoint discovery, production
/// transport authentication, or deployment readiness.
pub struct McpConformanceClient {
    runtime: tokio::runtime::Runtime,
    server: PaleMcpServer,
}

impl McpConformanceClient {
    /// Construct the complete MCP projection for one supplied attachment.
    ///
    /// # Errors
    ///
    /// Returns an error when the dispatcher, MCP registry, or local async
    /// runtime cannot be constructed.
    pub fn new(attachment: ResidentAttachment) -> Result<Self, McpConformanceError> {
        crate::verify_projected_capabilities().map_err(McpConformanceError::Projection)?;
        let dispatcher = CommandDispatcher::with_resident_attachment(attachment)
            .map_err(|error| McpConformanceError::Dispatcher(Box::new(error)))?;
        let server = PaleMcpServer::new(dispatcher).map_err(McpConformanceError::Projection)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .map_err(McpConformanceError::Runtime)?;
        Ok(Self { runtime, server })
    }

    /// Call one named MCP tool with an exact JSON object.
    ///
    /// # Errors
    ///
    /// Returns an error when MCP rejects the request, its worker fails, or a
    /// Pale tool unexpectedly returns a continuation.
    pub fn call_tool(
        &self,
        name: impl Into<String>,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, McpConformanceError> {
        let response =
            self.runtime
                .block_on(self.server.execute_tool(
                    CallToolRequestParams::new(name.into()).with_arguments(arguments),
                ))
                .map_err(|error| McpConformanceError::Protocol(error.to_string()))?;
        match response {
            CallToolResponse::Complete(result) => Ok(result),
            CallToolResponse::InputRequired(_) | CallToolResponse::Task(_) => {
                Err(McpConformanceError::UnsupportedContinuation)
            }
            _ => Err(McpConformanceError::UnsupportedContinuation),
        }
    }
}
