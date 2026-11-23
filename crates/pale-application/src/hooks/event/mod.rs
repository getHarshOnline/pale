//! Typed inbound host-event parsing.
//!
//! Hooks receive untrusted JSON on stdin. Parsing keeps the host's structured
//! facts intact and rejects malformed known fields instead of quietly turning
//! them into `None`.

use std::fmt;

use serde_json::Value;

mod parse;
mod paths;
mod prompt;

pub use parse::parse_stdin_event;
pub use prompt::DirectOwnerPrompt;

#[derive(Clone, PartialEq, Eq)]
/// Optional host fields that distinguish an admitted project command expansion.
pub(super) struct PromptExpansionFields {
    pub(super) expansion_type: Option<String>,
    pub(super) command_name: Option<String>,
    pub(super) command_args: Option<String>,
    pub(super) command_source: Option<String>,
}

/// The host facts consumed by the hook engine.
///
/// Fields are type-checked but remain observations, not authorization.
/// [`std::fmt::Debug`] deliberately reports presence and byte/count metadata
/// instead of prompt, command, result, or identity contents.
#[derive(Clone, PartialEq, Eq)]
pub struct StdinEventFields {
    hook_event_name: Option<String>,
    tool_name: Option<String>,
    command: Option<String>,
    tool_input: Option<Value>,
    tool_response: Option<Value>,
    tool_use_id: Option<String>,
    target_paths: Vec<String>,
    message: Option<String>,
    session_id: Option<String>,
    turn_id: Option<String>,
    agent_type: Option<String>,
    cwd: Option<String>,
    stop_hook_active: Option<bool>,
    expansion: Option<PromptExpansionFields>,
}

impl fmt::Debug for StdinEventFields {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StdinEventFields")
            .field("hook_event_name_present", &self.hook_event_name.is_some())
            .field("tool_name_present", &self.tool_name.is_some())
            .field("command_present", &self.command.is_some())
            .field("tool_input_present", &self.tool_input.is_some())
            .field("tool_response_present", &self.tool_response.is_some())
            .field("tool_use_id_present", &self.tool_use_id.is_some())
            .field("target_count", &self.target_paths.len())
            .field("message_present", &self.message.is_some())
            .field("session_id_present", &self.session_id.is_some())
            .field("turn_id_present", &self.turn_id.is_some())
            .field("agent_type_present", &self.agent_type.is_some())
            .field("cwd_present", &self.cwd.is_some())
            .field("stop_hook_active", &self.stop_hook_active)
            .field("prompt_expansion_present", &self.expansion.is_some())
            .finish()
    }
}

impl StdinEventFields {
    /// Native host event name used to confirm the sealed hook boundary.
    pub fn hook_event_name(&self) -> Option<&str> {
        self.hook_event_name.as_deref()
    }

    /// Native host tool identity after alias reconciliation.
    pub fn tool_name(&self) -> Option<&str> {
        self.tool_name.as_deref()
    }

    /// Structured command text extracted from a command-oriented tool input.
    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }

    /// Complete validated host-tool input used by typed admission predicates.
    pub fn tool_input(&self) -> Option<&Value> {
        self.tool_input.as_ref()
    }

    /// Structured host result, without inferring status from output text.
    pub fn tool_response(&self) -> Option<&Value> {
        self.tool_response.as_ref()
    }

    /// Native per-call identity shared by pre- and post-tool events.
    pub fn tool_use_id(&self) -> Option<&str> {
        self.tool_use_id.as_deref()
    }

    /// User prompt or message text exposed by the native event.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Native session identity, when the harness exposes one.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Native turn identity, when the harness exposes one.
    pub fn turn_id(&self) -> Option<&str> {
        self.turn_id.as_deref()
    }

    /// Child type supplied by a child-start event.
    pub fn agent_type(&self) -> Option<&str> {
        self.agent_type.as_deref()
    }

    /// Host-reported working directory.
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    /// Native stop-loop guard when the harness exposes it.
    pub fn stop_hook_active(&self) -> Option<bool> {
        self.stop_hook_active
    }
}

/// A parsed host event, or why it cannot be trusted.
///
/// [`Self::Fields`] establishes JSON shape only. Route, origin, tool, and effect
/// admission remain separate typed boundaries.
#[must_use = "untrusted host input must be admitted or rejected explicitly"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdinEvent {
    /// The host supplied no event document.
    Absent,
    /// The document exceeded bounds or violated the admitted JSON shape.
    Malformed(&'static str),
    /// Known fields were type-checked; unknown fields remain inert.
    Fields(Box<StdinEventFields>),
}
