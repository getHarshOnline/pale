//! Language Server for authoring Pale Workspace configuration, built on
//! [`pale_sdk`].
//!
//! The VS Code integration starts it as the `pale-lsp` process, one server per
//! workspace folder.
//!
//! The crate owns editor transport mechanics only: typed initialization,
//! immutable comparison anchors, client-observed saved baselines, open-buffer
//! overlays, exact registered-source refresh, document versions, UTF-16 ranges,
//! bounded scheduling, custom-request cancellation, and stale-result suppression. Its custom
//! inspection requests retain and project opaque [`pale_sdk`] compilation and
//! comparison handles. Effective-profile pages come directly from
//! [`pale_sdk::EffectiveProfileCompilation`]; the server adds only exact
//! editor coordinates and may reveal a prior page solely for the same explicit
//! project/execution-workspace context, marked stale. Workspace grammar,
//! semantic admission, reachability, proof selection, and diagnostics remain
//! owned by [`pale_sdk::PaleApi`].
//! Source languages in repositories that Pale later assists are deliberately
//! outside this server; it analyzes only Pale Workspace configuration
//! documents.

mod analysis;
mod authoring;
#[cfg(feature = "bench")]
#[doc(hidden)]
pub mod benchmark;
mod document;
mod protocol;
mod scheduling;
mod server;
mod syntax;

pub use server::{run_stdio, ServerError};

/// Export private editor wire shapes for development-time client generation.
///
/// This does not validate a workspace or start the server.
#[cfg(feature = "protocol-codegen")]
#[doc(hidden)]
#[must_use]
pub fn private_protocol_schemas() -> serde_json::Value {
    protocol::schema::export()
}
