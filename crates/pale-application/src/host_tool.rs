//! Match a tool call from a coding agent to the host-tool binding a route
//! projects for it.
//!
//! `hooks`, `operation_admission` and `resident` build on it; it builds on
//! `host_command` and `terminal_adapters`. Matching is exact.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use pale_core::records::{EffectClass, HostToolBinding, HostToolEffectBinding, ToolObservability};

/// Filesystem-write observability exposed by a native host protocol.
///
/// [`ProtocolToolKind::NonWriting`] means the invocation has no direct
/// project-filesystem write surface at pre-tool time; it does not claim that the
/// tool is side-effect free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProtocolToolKind {
    NonWriting,
    StructuredWrite,
    Command,
    Unknown,
}

/// Classify an exact native-harness tool name for filesystem-boundary consumers.
pub(crate) fn protocol_tool_kind(
    host: TerminalAdapterDescriptor,
    tool_name: &str,
) -> ProtocolToolKind {
    match (host.is_claude_code(), tool_name) {
        (
            true,
            "Agent"
            | "CronCreate"
            | "CronDelete"
            | "CronList"
            | "EndConversation"
            | "EnterPlanMode"
            | "ExitPlanMode"
            | "ExitWorktree"
            | "Read"
            | "Glob"
            | "Grep"
            | "WebFetch"
            | "WebSearch"
            | "AskUserQuestion"
            | "Skill"
            | "LSP"
            | "ListMcpResourcesTool"
            | "ReadMcpResourceTool"
            | "ReportFindings"
            | "ScheduleWakeup"
            | "SendMessage"
            | "TaskCreate"
            | "TaskGet"
            | "TaskList"
            | "TaskOutput"
            | "TaskStop"
            | "TaskUpdate"
            | "TodoWrite"
            | "ToolSearch"
            | "WaitForMcpServers"
            | "Workflow",
        )
        | (
            false,
            "view_image"
            | "web__run"
            | "read_mcp_resource"
            | "list_mcp_resources"
            | "list_mcp_resource_templates"
            | "request_user_input"
            | "get_goal"
            | "update_plan",
        ) => ProtocolToolKind::NonWriting,
        (true, "Edit" | "Write" | "MultiEdit" | "NotebookEdit") | (false, "apply_patch") => {
            ProtocolToolKind::StructuredWrite
        }
        (true, "Bash" | "PowerShell" | "Monitor") | (false, "Bash" | "exec_command" | "shell") => {
            ProtocolToolKind::Command
        }
        _ => ProtocolToolKind::Unknown,
    }
}

/// Whether a sealed effect binding can be proven from observable tool and input facts alone.
/// A command-constrained binding cannot match without its observed command.
pub(crate) fn has_matching_effect_binding(
    bindings: &[HostToolEffectBinding],
    tool_name: &str,
    tool_input: Option<&serde_json::Value>,
) -> bool {
    bindings
        .iter()
        .any(|binding| matches_without_command(&binding.tool, tool_name, tool_input))
}

/// Whether this invocation has a protocol-known or sealed structured-file-write contract.
pub(crate) fn is_structured_write(
    host: TerminalAdapterDescriptor,
    bindings: &[HostToolEffectBinding],
    tool_name: &str,
    tool_input: Option<&serde_json::Value>,
) -> bool {
    match protocol_tool_kind(host, tool_name) {
        ProtocolToolKind::StructuredWrite => true,
        ProtocolToolKind::Unknown => bindings.iter().any(|binding| {
            binding.effect_class == EffectClass::Write
                && matches_without_command(&binding.tool, tool_name, tool_input)
        }),
        ProtocolToolKind::NonWriting | ProtocolToolKind::Command => false,
    }
}

fn matches_without_command(
    binding: &HostToolBinding,
    tool_name: &str,
    tool_input: Option<&serde_json::Value>,
) -> bool {
    binding.commands.is_empty() && matches_pre_tool_input(binding, tool_name, tool_input)
}

/// Match the observable tool identity and typed input predicates, without
/// interpreting an optional command contract. Effect classification and
/// command admission deliberately consume these facts separately.
fn matches_pre_tool_input(
    binding: &HostToolBinding,
    tool_name: &str,
    tool_input: Option<&serde_json::Value>,
) -> bool {
    binding.name == tool_name
        && binding.observability == ToolObservability::PreToolObservable
        && binding.command_source.is_none()
        && binding.input_equals.iter().all(|(pointer, expected)| {
            tool_input
                .and_then(|input| input.pointer(pointer.as_ref()))
                .and_then(serde_json::Value::as_str)
                == Some(expected.as_str())
        })
}

/// Match only a fully projected binding visible at the pre-tool boundary.
/// Declarative command sources must be expanded before a binding is sealed.
pub(crate) fn matches_pre_tool(
    binding: &HostToolBinding,
    tool_name: &str,
    tool_input: Option<&serde_json::Value>,
    command: Option<&str>,
) -> bool {
    matches_pre_tool_input(binding, tool_name, tool_input)
        && (binding.commands.is_empty()
            || command.is_some_and(|value| {
                binding
                    .commands
                    .iter()
                    .any(|constraint| crate::host_command::matches(constraint, value))
            }))
}

#[cfg(test)]
#[path = "../tests/internal/host_tool.rs"]
mod tests;
