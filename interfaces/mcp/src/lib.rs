//! The MCP server through which coding agents such as Claude Code and Codex reach
//! Pale. It builds on `pale-cli`'s command façade, and the `pale-mcp` binary that
//! agent hosts launch and the shared interface tests build on it.
//!
//! MCP frames translate into [`pale_cli::command::CommandRequest`] values;
//! the resulting [`pale_cli::command::CommandOutcome`] or admitted
//! [`pale_cli::command::DiagnosticV1`] is projected without semantic
//! reclassification. The leaf owns only protocol admission, bounded stdio,
//! cache hints, and request concurrency. [`run`] exposes only request-local
//! operations. [`run_with_resident_attachment`] can also project resident
//! operations through one caller-provisioned attachment. Its embedder remains
//! responsible for carrier authentication and response admission. MCP owns no
//! compiler, diagnostic, operation, task, cancellation, or native-effect state.

use std::collections::BTreeSet;
use std::error::Error;

use pale_cli::command::{
    capability_inventory, ApiCapabilityId, ApiLeaf, CommandDispatcher, ResidentAttachment,
};
use rmcp::RoleServer;

mod projection;
mod protocol;
mod resources;
mod server;
mod tools;
mod transport;
mod wire;

#[cfg(feature = "interface-test-support")]
pub mod conformance;

/// Crate-local fallible result for transport and process construction.
type ProcessResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Attach a short stable stage to an outer process error.
fn stage(stage: &'static str, error: impl std::fmt::Display) -> Box<dyn Error + Send + Sync> {
    std::io::Error::other(format!("{stage}: {error}")).into()
}

/// Collect the complete API capability projection implemented by this MCP leaf.
fn projected_capability_ids() -> BTreeSet<ApiCapabilityId> {
    tools::FacadeTool::all()
        .iter()
        .flat_map(|route| route.capability_ids())
        .chain(resources::CAPABILITY_IDS)
        .chain(tools::NATIVE_ACTION_CAPABILITY_IDS)
        .copied()
        .collect()
}

/// Reject startup when MCP exposure drifts from the API-owned allowlist.
///
/// This proves static projection parity only. Repository-bound catalog
/// admission remains separate from the built-in pure action tools.
fn verify_projected_capabilities() -> ProcessResult<()> {
    let projected = projected_capability_ids();
    let eligible = capability_inventory()
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::Mcp))
        .map(|capability| capability.id())
        .collect::<BTreeSet<_>>();
    if projected == eligible {
        return Ok(());
    }

    let missing = eligible.difference(&projected).copied().collect::<Vec<_>>();
    let unexpected = projected.difference(&eligible).copied().collect::<Vec<_>>();
    Err(std::io::Error::other(format!(
        "MCP capability projection drifted from pale-sdk: missing {missing:?}, unexpected {unexpected:?}"
    ))
    .into())
}

/// Maximum complete JSON-RPC line retained by the stdio carrier.
///
/// This transport ceiling includes envelope metadata, so the largest payload
/// representable over MCP is intentionally smaller than the API payload bound.
const TRANSPORT_LINE_LIMIT: usize = 16 << 20;

/// Serve request-local MCP tools over bounded stdio in either connection mode.
///
/// The process advertises no tasks, progress, cancellation, resumption, or
/// repository-bound action admission. Fixed façade tools and registry-derived
/// pure action tools are request-local projections of the reusable CLI
/// dispatcher and therefore inherit the API's non-effect.
/// The four-worker ceiling is process-local; this function makes no claim about
/// principal-wide load, fairness, compilation coalescing, or cancellation after
/// [`tokio::task::spawn_blocking`] starts. Those require resident-owned handles
/// and admission rather than protocol-local state.
///
/// # Errors
///
/// Returns an error when the command façade is incoherent, capability coverage
/// drifts, the MCP surface cannot be derived, the async runtime fails to start,
/// or the stdio service terminates unsuccessfully.
pub fn run() -> ProcessResult<()> {
    verify_projected_capabilities()?;
    let dispatcher =
        CommandDispatcher::new().map_err(|error| stage("initialize dispatcher", error))?;
    serve(dispatcher)
}

/// Serve MCP with one caller-provisioned resident attachment.
///
/// The caller remains responsible for endpoint selection, peer authentication,
/// response-authority admission, and attachment provisioning. MCP does not
/// independently verify those assertions.
/// This function adds only the resident tools supported by that attachment; it
/// performs no discovery and opens no resident storage.
///
/// # Errors
///
/// Returns an error when the command façade is incoherent, capability coverage
/// drifts, the MCP surface cannot be derived, the async runtime fails to start,
/// or the stdio service terminates unsuccessfully.
pub fn run_with_resident_attachment(attachment: ResidentAttachment) -> ProcessResult<()> {
    verify_projected_capabilities()?;
    let dispatcher = CommandDispatcher::with_resident_attachment(attachment)
        .map_err(|error| stage("initialize attached dispatcher", error))?;
    serve(dispatcher)
}

fn serve(dispatcher: CommandDispatcher) -> ProcessResult<()> {
    let server = server::PaleMcpServer::new(dispatcher)
        .map_err(|error| stage("project MCP surface", error))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .map_err(|error| stage("build MCP runtime", error))?;
    runtime.block_on(async move {
        let input = transport::BoundedLines::new(tokio::io::stdin(), TRANSPORT_LINE_LIMIT);
        let service = rmcp::service::serve_directly::<RoleServer, _, _, _, _>(
            protocol::ProtocolService::new(server),
            (input, tokio::io::stdout()),
            None,
        );
        service
            .waiting()
            .await
            .map(|_| ())
            .map_err(|error| stage("run MCP stdio service", error))
    })
}

#[cfg(test)]
#[path = "../tests/internal/lib.rs"]
mod tests;
