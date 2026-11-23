//! Stable MCP projection over the shared typed command dispatcher.

use std::collections::BTreeMap;
use std::sync::Arc;

use pale_cli::command::{
    ApiError, CommandDispatcher, CommandOutcome, CommandRequest, DiagnosticPresentation,
};
use rmcp::model::{
    CacheScope, CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock,
    ErrorData as McpError, Implementation, ListResourceTemplatesResult, ListResourcesResult,
    ListToolsResult, ReadResourceRequestParams, ReadResourceResponse, ServerCapabilities,
    ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, ServerHandler};
use tokio::sync::{OwnedSemaphorePermit, TryAcquireError};

use crate::{protocol, resources, tools, ProcessResult};

/// Process-local concurrent worker ceiling for bounded MCP requests.
///
/// This limit is transport safety policy, not project meaning. Saturation is
/// rejected before payload encoding instead of creating an unbounded queue.
/// It is neither a principal-wide scheduler nor evidence of machine-wide
/// fairness: separate `pale-mcp` processes retain separate ceilings.
const MAX_CONCURRENT_REQUESTS: usize = 4;

/// Acquire one worker slot without queueing behind admitted requests.
fn acquire_worker(permits: &Arc<tokio::sync::Semaphore>) -> Result<OwnedSemaphorePermit, McpError> {
    Arc::clone(permits)
        .try_acquire_owned()
        .map_err(|error| match error {
            TryAcquireError::NoPermits => McpError::internal_error(
                "Pale MCP server is busy; retry the bounded request",
                Some(serde_json::json!({
                    "code": "pale_mcp_overloaded",
                    "retryable": true,
                    "active_limit": MAX_CONCURRENT_REQUESTS,
                })),
            ),
            TryAcquireError::Closed => McpError::internal_error(
                "Pale MCP server is shutting down",
                Some(serde_json::json!({
                    "code": "pale_mcp_server_closed",
                    "retryable": false,
                })),
            ),
        })
}

/// Immutable MCP descriptor paired with one closed dispatcher route.
struct RegisteredTool {
    route: tools::ToolRoute,
    descriptor: Tool,
}

/// Insert one MCP tool while enforcing global name uniqueness.
fn insert_tool(
    tools: &mut BTreeMap<String, RegisteredTool>,
    registration: tools::ToolRegistration,
) -> ProcessResult<()> {
    let name = registration.descriptor.name.to_string();
    if tools
        .insert(
            name.clone(),
            RegisteredTool {
                route: registration.route,
                descriptor: registration.descriptor,
            },
        )
        .is_some()
    {
        return Err(std::io::Error::other(format!("duplicate MCP tool `{name}`")).into());
    }
    Ok(())
}

/// MCP service projected from API-backed command journeys.
///
/// Request-local tool success proves only the matching bounded computation.
/// Resident tools exist only when the dispatcher holds an authenticated
/// attachment; their results retain the SDK's effect and uncertainty meaning.
/// Neither mode establishes installation, provider admission, or runtime
/// activation.
#[derive(Clone)]
pub(super) struct PaleMcpServer {
    dispatcher: Arc<CommandDispatcher>,
    tools: Arc<BTreeMap<String, RegisteredTool>>,
    permits: Arc<tokio::sync::Semaphore>,
    resident_observation: bool,
    resident_operations: bool,
}

impl PaleMcpServer {
    /// Project the complete immutable MCP façade from one shared dispatcher.
    pub(super) fn new(dispatcher: CommandDispatcher) -> ProcessResult<Self> {
        let resident_observation = dispatcher.has_resident_attachment();
        let resident_operations = dispatcher.has_resident_operation_attachment();
        let mut tools = BTreeMap::new();
        for registration in
            tools::FacadeTool::registrations(resident_observation, resident_operations)?
        {
            insert_tool(&mut tools, registration)?;
        }
        let action_catalog = dispatcher
            .dispatch(CommandRequest::NativeActionCatalog)
            .map_err(|error| crate::stage("project native-action MCP tools", error))?;
        let CommandOutcome::NativeActionCatalog(actions) = action_catalog else {
            return Err(std::io::Error::other(
                "native-action catalog request returned an incompatible outcome",
            )
            .into());
        };
        for action in &actions {
            insert_tool(&mut tools, tools::native_action_registration(action)?)?;
        }
        Ok(Self {
            dispatcher: Arc::new(dispatcher),
            tools: Arc::new(tools),
            permits: Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_REQUESTS)),
            resident_observation,
            resident_operations,
        })
    }

    /// Admit and execute one tool request through the bounded worker path.
    pub(crate) async fn execute_tool(
        &self,
        request: CallToolRequestParams,
    ) -> Result<CallToolResponse, McpError> {
        if request.input_responses.is_some() || request.request_state.is_some() {
            return Err(McpError::invalid_params(
                "Pale tools do not use input-required request state",
                None,
            ));
        }
        let route = self
            .tools
            .get(request.name.as_ref())
            .map(|tool| tool.route.clone())
            .ok_or_else(|| {
                McpError::invalid_params(format!("unknown Pale tool `{}`", request.name), None)
            })?;
        let permit = acquire_worker(&self.permits)?;
        let command = route.command(request.arguments.unwrap_or_default())?;
        let dispatcher = Arc::clone(&self.dispatcher);
        // Dropping the JSON-RPC caller cannot cancel an admitted blocking
        // computation. Protocol cancellation therefore remains unavailable.
        // An attached caller can request operation cancellation explicitly by
        // durable handle; that is a separate resident-owned journey.
        let execution = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            dispatcher.dispatch(command)
        })
        .await
        .map_err(|error| {
            eprintln!("pale-mcp worker failed: {error}");
            McpError::internal_error("Pale command worker failed", None)
        })?;
        match execution {
            Ok(outcome) => route.project(outcome).map(Into::into),
            Err(error) => encode_api_failure(&error).map(Into::into),
        }
    }
}

impl ServerHandler for PaleMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_protocol_version(protocol::protocol_version())
        .with_server_info(Implementation::new(
            "pale-mcp",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(match (self.resident_observation, self.resident_operations) {
            (_, true) => "Use Pale's tools to learn what a workspace configuration means before anyone edits it, and to carry durable operations through Pale's resident process: Invoke a declared operation, Consult for a continue-or-stop judgment under its retained compiled policy, Delegate one checked task to another agent system, Observe a retained operation's signed records, and send Cancel requests. A start returns a handle before dispatch; advance it one bounded turn at a time. Treat pending and unknown outcomes as non-terminal, and Observe or reconcile before any retry. Cancel acknowledgement does not prove terminal cancellation. The calls grant nothing beyond the attachment the caller provisioned, and the embedder, not MCP, asserts carrier authentication and response admission.",
            (true, false) => "Use Pale's tools to learn what a workspace configuration means before anyone edits it, and to check what happened to operations Pale's resident process has retained. This attachment provides read-only resident Observe: it can Observe authorized retained operations but cannot Invoke, Consult, Delegate or Cancel them, so starting or stopping work needs an operating attachment. Whether a change goes ahead stays with the workspace's owner. The embedder, not MCP, asserts carrier authentication and response admission.",
            (false, false) => "Use Pale's tools to learn what a workspace configuration means before anyone edits it: check that supplied workspace documents compile, see what a proposed change affects and why, inspect what applies to a repository, review a configuration change, and plan the guidance files Pale would generate. Each call works only on the documents and observations you supply: it reads no local files and applies no plan. If a call returns a diagnostic, report its file, line and identity and fix the supplied document rather than retrying it unchanged. These are pure deterministic Pale mechanics, so they give no contextual judgment or permission to act; whether a change goes ahead stays with the workspace's owner. Supply only fields the selected input schema defines.",
        })
    }

    fn supported_protocol_versions(
        &self,
    ) -> std::borrow::Cow<'static, [rmcp::model::ProtocolVersion]> {
        protocol::supported_protocol_versions()
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tools.get(name).map(|tool| tool.descriptor.clone())
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(
            self.tools
                .values()
                .map(|tool| tool.descriptor.clone())
                .collect(),
        )
        .with_ttl_ms(protocol::TOOL_LIST_CACHE_TTL_MS)
        .with_cache_scope(CacheScope::Private))
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(resources::list())
    }

    async fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(resources::templates())
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, McpError> {
        if request.input_responses.is_some() || request.request_state.is_some() {
            return Err(McpError::invalid_params(
                "Pale contract resources do not use input-required request state",
                None,
            ));
        }
        resources::read(&self.dispatcher, &request.uri)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        self.execute_tool(request).await
    }
}

/// Return this call's diagnostic and source evidence only to its requesting caller.
fn encode_api_failure(error: &ApiError) -> Result<CallToolResult, McpError> {
    let project = || -> Result<CallToolResult, serde_json::Error> {
        let structured = DiagnosticPresentation::from_api_error(error).to_json_value()?;
        let diagnostic = error.diagnostic();
        let concise = format!("{}: {}", diagnostic.code(), diagnostic.summary());
        let mut result = CallToolResult::error(vec![ContentBlock::text(concise)]);
        result.structured_content = Some(structured);
        Ok(result)
    };
    project().map_err(|decoding| {
        eprintln!("pale-mcp diagnostic projection failed: {decoding}");
        McpError::internal_error("Pale API diagnostic could not be projected", None)
    })
}

#[cfg(test)]
#[path = "../tests/internal/server.rs"]
mod tests;
