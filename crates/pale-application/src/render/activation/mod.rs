//! Activation rendering keeps host wiring separate from sealed context construction.

mod context;
mod wiring;

pub(super) use context::{hook_context_json, HookContextRender};
pub use context::{HOOK_CONTEXT_SCHEMA, HOOK_CONTEXT_SCHEMA_VERSION};
pub(super) use wiring::{
    claude_hook_contribution, claude_hook_wiring, codex_hook_contribution, codex_hook_wiring,
    ActiveHookWiring,
};
