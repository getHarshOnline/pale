//! Host wiring preserves declared event order and exact runtime command seals.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use pale_core::records::{
    LifecycleEvent, LogicalRepositoryIdV1, ProviderBindingRefV1, WorkspaceId,
};

use super::super::semantic_helpers::json_pretty;
use super::super::types::RenderError;
use crate::hooks::{
    harness_command_timeout_ceiling_secs, host_event_name, HookEvent, HookRequirements,
    RepositoryHookRoutingV1,
};

/// Immutable seal and lifecycle input shared by both native-harness renderers.
pub(in crate::render) struct ActiveHookWiring<'a> {
    /// Exact admitted `pale` CLI executable invoked by the native hook host.
    pub(in crate::render) exec: &'a str,
    /// Principal workspace whose active generation the daemon resolves.
    pub(in crate::render) workspace_id: &'a WorkspaceId,
    /// Canonical repository whose native event entered the daemon.
    pub(in crate::render) repo_id: &'a LogicalRepositoryIdV1,
    /// Native hook host that owns the stdin and output protocols.
    pub(in crate::render) harness: TerminalAdapterDescriptor,
    /// Exact provider binding carried by every generated route.
    pub(in crate::render) provider_binding: &'a ProviderBindingRefV1,
    pub(in crate::render) lifecycle: &'a [LifecycleEvent],
    pub(in crate::render) requirements: &'a HookRequirements,
    pub(in crate::render) behaviours: &'a [pale_core::records::LifecycleBehavior],
}

/// Render an inert record of the Claude lifecycle wiring an authorized activation would install.
pub(in crate::render) fn claude_hook_wiring(
    requirements: &HookRequirements,
) -> Result<String, RenderError> {
    hook_activation_intent(
        "claude-hook-wiring",
        requirements,
        "Inert record; not a runnable settings.json. Activation structurally merges a repository-scoped Pale command contribution into native-owner settings; the daemon resolves changing generation, policy, context, and session state when the event arrives.",
    )
}

/// Render an inert record of the Codex lifecycle wiring an authorized activation would install.
pub(in crate::render) fn codex_hook_wiring(
    requirements: &HookRequirements,
) -> Result<String, RenderError> {
    hook_activation_intent(
        "codex-hook-wiring",
        requirements,
        "Inert record; not a runnable .codex/hooks.json. Activation structurally merges a repository-scoped Pale command contribution into native-owner hooks after project trust; the daemon resolves changing generation, policy, context, and session state when the event arrives.",
    )
}

/// Render inert activation intent; the result is never native configuration.
fn hook_activation_intent(
    intent: &str,
    requirements: &HookRequirements,
    note: &str,
) -> Result<String, RenderError> {
    let events = requirements
        .events()
        .iter()
        .map(|event| event_token(*event))
        .collect::<Vec<_>>();
    json_pretty(
        &serde_json::json!({
            "activation_intent": intent,
            "runnable": false,
            "note": note,
            "would_wire_events": events,
            "gated_tools": requirements.gated_tools(),
        }),
        "generated JSON surface",
    )
}

/// Host hook timeout in seconds, bounded by the host's command-hook ceiling.
fn event_hook_timeout_secs(
    behaviours: &[pale_core::records::LifecycleBehavior],
    harness: TerminalAdapterDescriptor,
    event: HookEvent,
) -> Option<u32> {
    let ceiling = harness_command_timeout_ceiling_secs(harness, event)?;
    behaviours
        .iter()
        .filter(|behaviour| behaviour.event == event)
        .filter_map(|behaviour| behaviour.timeout_ms)
        .filter(|milliseconds| *milliseconds > 0)
        .max()
        .map(|milliseconds| milliseconds.div_ceil(1000).clamp(1, ceiling))
}

/// Render the Pale-owned Claude Code handler contribution.
///
/// The returned object is not a complete `.claude/settings.json`. Activation
/// passes it to [`crate::hooks::merge_native_json_hook_config`] so permissions,
/// environment, matcher groups, and handlers owned by the repository remain
/// intact.
pub(in crate::render) fn claude_hook_contribution(
    input: &ActiveHookWiring<'_>,
) -> Result<String, RenderError> {
    if input.harness != TerminalAdapterDescriptor::claude_code() {
        return Err(RenderError::invalid_input(
            "native hook host",
            format!("Claude settings cannot encode `{}` hooks", input.harness),
        ));
    }
    let hooks = claude_hook_map(input)?;
    json_pretty(
        &serde_json::json!({ "hooks": serde_json::Value::Object(hooks) }),
        "generated JSON surface",
    )
}

/// Render the Pale-owned Codex handler contribution.
///
/// The returned JSON-shaped object is structurally merged into the already
/// selected `.codex/hooks.json` representation or translated into the
/// documented inline `.codex/config.toml` tables. It never replaces the
/// repository's complete native configuration.
pub(in crate::render) fn codex_hook_contribution(
    input: &ActiveHookWiring<'_>,
) -> Result<String, RenderError> {
    if input.harness != TerminalAdapterDescriptor::codex() {
        return Err(RenderError::invalid_input(
            "native hook host",
            format!("Codex settings cannot encode `{}` hooks", input.harness),
        ));
    }
    let hooks = codex_hook_map(input)?;
    json_pretty(
        &serde_json::json!({ "hooks": serde_json::Value::Object(hooks) }),
        "generated JSON surface",
    )
}

/// Build runnable hook entries only from an activation-admitted wiring input.
///
/// Build Claude Code's exec-form command handlers.
///
/// Claude Code accepts an executable in `command` and a separate `args` vector,
/// so no shell reparses Pale's sealed coordinates.
fn claude_hook_map(
    input: &ActiveHookWiring<'_>,
) -> Result<serde_json::Map<String, serde_json::Value>, RenderError> {
    let mut hooks = serde_json::Map::new();
    for event in input.requirements.events() {
        if !input.lifecycle.contains(event) {
            continue;
        }
        let routing = repository_hook_routing(input, *event)?;
        let mut record = serde_json::Map::new();
        record.insert("type".into(), serde_json::json!("command"));
        record.insert("command".into(), serde_json::json!(input.exec));
        record.insert("args".into(), serde_json::json!(routing.arguments()));
        if let Some(seconds) = event_hook_timeout_secs(input.behaviours, input.harness, *event) {
            record.insert("timeout".into(), serde_json::json!(seconds));
        }
        let entry = serde_json::json!({ "hooks": [serde_json::Value::Object(record)] });
        hooks.insert(host_event_name(*event).into(), serde_json::json!([entry]));
    }
    Ok(hooks)
}

/// Build Codex's shell-form command handlers from the same argument vector.
///
/// Codex's current hook schema has no separate argument vector, so every word
/// is quoted exactly once before the native hook runtime invokes the shell.
fn codex_hook_map(
    input: &ActiveHookWiring<'_>,
) -> Result<serde_json::Map<String, serde_json::Value>, RenderError> {
    let mut hooks = serde_json::Map::new();
    for event in input.requirements.events() {
        if !input.lifecycle.contains(event) {
            continue;
        }
        let routing = repository_hook_routing(input, *event)?;
        let command = std::iter::once(input.exec.to_owned())
            .chain(routing.arguments())
            .map(|word| crate::host_command::quote_posix_word(&word))
            .collect::<Vec<_>>()
            .join(" ");
        let mut record = serde_json::Map::new();
        record.insert("type".into(), serde_json::json!("command"));
        record.insert("command".into(), serde_json::json!(command));
        if let Some(seconds) = event_hook_timeout_secs(input.behaviours, input.harness, *event) {
            record.insert("timeout".into(), serde_json::json!(seconds));
        }
        let entry = serde_json::json!({ "hooks": [serde_json::Value::Object(record)] });
        hooks.insert(host_event_name(*event).into(), serde_json::json!([entry]));
    }
    Ok(hooks)
}

/// Bind one native event to the shared repository-hook routing coordinate.
///
/// The repository-local configuration does not freeze a compiled generation,
/// path, policy, context, or runtime digest. The resident daemon resolves those
/// changing coordinates from the admitted workspace/repository/session state
/// when the native event arrives on stdin.
fn repository_hook_routing(
    input: &ActiveHookWiring<'_>,
    event: HookEvent,
) -> Result<RepositoryHookRoutingV1, RenderError> {
    RepositoryHookRoutingV1::new(
        event,
        input.workspace_id.clone(),
        input.repo_id.clone(),
        input.provider_binding.clone(),
    )
    .map_err(|error| RenderError::invalid_input_error("repository hook routing", error))
}

/// Reuse the typed hook event's stable CLI spelling.
fn event_token(event: HookEvent) -> &'static str {
    event.as_str()
}
