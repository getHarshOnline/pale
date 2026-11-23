//! Shared native connection for agent hosts using Pale's SDK.
//!
//! This crate builds on `pale-sdk` and connects an agent host to a configured local
//! resident. The Claude Code and Codex integrations that share it are not built, so no
//! Pale crate depends on it yet.
//!
//! The host supplies protected endpoint configuration, connection credentials
//! and signed-response admission. The integration owns no application grants,
//! operation store, policy evaluator or fallback execution path.

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use pale_sdk::{LocalAgentCarrierSource, LocalAgentCarrierSourceError};

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod producer;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use producer::{LocalAgentNativeTool, LocalAgentNativeToolPreparation};
