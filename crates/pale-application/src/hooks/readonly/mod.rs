//! Write-protection evaluation for sealed read-only and Pale runtime-state roots.

mod command;
mod path;

use crate::terminal_adapters::TerminalAdapterDescriptor;

use std::path::PathBuf;

use self::command::shell_violation;
use self::path::{path_violation, resolve_target};
use pale_core::records::HostToolEffectBinding;
use pale_ports::MachineFilesystemPort;

use crate::host_tool::{protocol_tool_kind, ProtocolToolKind};

fn boundary_reason(why: &str) -> String {
    format!("Pale write-protected boundary: {why}.")
}

/// Host facts available before a tool permission or execution decision.
#[derive(Debug, Clone, Copy, Default)]
pub struct PreToolUseFacts<'a> {
    /// Whether the known host event shape failed strict parsing.
    pub malformed: bool,
    /// Native host tool identity, when supplied and type-valid.
    pub tool_name: Option<&'a str>,
    /// Structured write targets extracted from the admitted host protocol.
    pub write_paths: &'a [String],
    /// Host-reported working directory used to resolve relative targets.
    pub cwd: Option<&'a str>,
    /// Complete structured tool input retained for sealed effect matching.
    pub tool_input: Option<&'a serde_json::Value>,
    /// Parsed command text for a command-oriented host tool.
    pub command: Option<&'a str>,
}

/// Evaluate one sealed write-boundary policy over protected roots.
///
/// The caller owns the native deny decision. Constructing this value or calling
/// [`Self::violation`] does not by itself stop a host tool.
#[derive(Debug, Clone, Copy)]
pub struct WriteBoundary<'a> {
    host: TerminalAdapterDescriptor,
    readonly_dirs: &'a [String],
    runtime_dirs: &'a [String],
    host_tool_effects: &'a [HostToolEffectBinding],
}

impl<'a> WriteBoundary<'a> {
    /// Construct a boundary from sealed protected roots and host-effect bindings.
    pub fn new(
        host: TerminalAdapterDescriptor,
        readonly_dirs: &'a [String],
        runtime_dirs: &'a [String],
        host_tool_effects: &'a [HostToolEffectBinding],
    ) -> Self {
        Self {
            host,
            readonly_dirs,
            runtime_dirs,
            host_tool_effects,
        }
    }

    /// Return a host-independent denial reason, preserving it for evidence before
    /// native-harness encoding.
    pub fn violation(
        self,
        filesystem: &dyn MachineFilesystemPort,
        facts: PreToolUseFacts<'_>,
    ) -> Option<String> {
        if self.readonly_dirs.is_empty() && self.runtime_dirs.is_empty() {
            return None;
        }
        let protected_dirs = match self
            .readonly_dirs
            .iter()
            .chain(self.runtime_dirs)
            .map(|path| resolve_target(filesystem, path, None))
            .collect::<Option<Vec<_>>>()
        {
            Some(paths) => paths,
            None => {
                return Some(boundary_reason(
                    "a sealed read-only root cannot be resolved",
                ));
            }
        };
        if facts.malformed {
            return Some(boundary_reason(
                "the host event is malformed, so its effects cannot be evaluated",
            ));
        }
        let Some(tool_name) = facts.tool_name else {
            return Some(boundary_reason(
                "the host event carried no determinable tool",
            ));
        };
        self.violation_for_known_tool(filesystem, tool_name, facts, &protected_dirs)
    }

    fn violation_for_known_tool(
        self,
        filesystem: &dyn MachineFilesystemPort,
        tool_name: &str,
        facts: PreToolUseFacts<'_>,
        protected_dirs: &[PathBuf],
    ) -> Option<String> {
        let deny = |reason: String| Some(boundary_reason(&reason));
        let has_effect_binding = crate::host_tool::has_matching_effect_binding(
            self.host_tool_effects,
            tool_name,
            facts.tool_input,
        );
        match protocol_tool_kind(self.host, tool_name) {
            ProtocolToolKind::StructuredWrite => {
                if facts.write_paths.is_empty() {
                    return deny("a write-capable tool carried no verifiable target".to_string());
                }
                for path in facts.write_paths {
                    match path_violation(filesystem, path, facts.cwd, protected_dirs) {
                        Ok(Some(reason)) | Err(reason) => return deny(reason),
                        Ok(None) => {}
                    }
                }
                None
            }
            ProtocolToolKind::Command => {
                let Some(command) = facts.command else {
                    return deny("a command tool carried no verifiable command".to_string());
                };
                match shell_violation(filesystem, command, facts.cwd, protected_dirs) {
                    Ok(Some(reason)) | Err(reason) => deny(reason),
                    Ok(None) => None,
                }
            }
            ProtocolToolKind::NonWriting => None,
            ProtocolToolKind::Unknown if !has_effect_binding => deny(format!(
                "tool `{tool_name}` has no matching sealed effect binding, so write safety is unevaluable"
            )),
            ProtocolToolKind::Unknown => {
                if crate::host_tool::is_structured_write(
                    self.host,
                    self.host_tool_effects,
                    tool_name,
                    facts.tool_input,
                ) {
                    if facts.write_paths.is_empty() {
                        return deny(format!(
                            "write-classified tool `{tool_name}` carried no verifiable target"
                        ));
                    }
                    for path in facts.write_paths {
                        match path_violation(filesystem, path, facts.cwd, protected_dirs) {
                            Ok(Some(reason)) | Err(reason) => return deny(reason),
                            Ok(None) => {}
                        }
                    }
                }
                None
            }
        }
    }
}
