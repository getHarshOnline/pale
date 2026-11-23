//! Structured write-target extraction without granting filesystem authority.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use std::collections::BTreeSet;

use pale_core::records::HostToolEffectBinding;
use serde_json::{Map, Value};

use super::parse::optional_string;
use super::StdinEventFields;
use crate::hooks::PreToolUseFacts;

fn apply_patch_paths(command: &str) -> Result<Vec<String>, &'static str> {
    let lines: Vec<_> = command.lines().collect();
    if lines.first() != Some(&"*** Begin Patch") || lines.last() != Some(&"*** End Patch") {
        return Err("apply_patch is missing its document boundary");
    }

    let mut paths = BTreeSet::new();
    for line in &lines[1..lines.len() - 1] {
        let path = [
            "*** Add File: ",
            "*** Update File: ",
            "*** Delete File: ",
            "*** Move to: ",
        ]
        .iter()
        .find_map(|prefix| line.strip_prefix(prefix));
        let Some(path) = path else {
            continue;
        };
        if path.is_empty() || path.trim() != path || path.bytes().any(|byte| byte == 0) {
            return Err("apply_patch contains an invalid path header");
        }
        paths.insert(path.to_owned());
    }
    if paths.is_empty() {
        return Err("apply_patch contains no file targets");
    }
    Ok(paths.into_iter().collect())
}

/// Extract every literal target from recognized structured-write shapes.
pub(super) fn target_paths(
    tool_name: Option<&str>,
    tool_input: Option<&Map<String, Value>>,
) -> Result<Vec<String>, &'static str> {
    let Some(input) = tool_input else {
        return Ok(Vec::new());
    };
    let mut paths = BTreeSet::new();
    for key in ["file_path", "notebook_path"] {
        if let Some(path) = optional_string(input, key)? {
            paths.insert(path);
        }
    }

    // Multi-edit protocols carry one object per edit. Every target belongs to
    // the same authority decision; admitting only the first would be unsound.
    if let Some(edits) = input.get("edits") {
        let Value::Array(edits) = edits else {
            return Err("tool_input.edits is not an array");
        };
        for edit in edits {
            let Value::Object(edit) = edit else {
                return Err("tool_input.edits contains a non-object");
            };
            let Some(path) = optional_string(edit, "file_path")? else {
                return Err("tool_input.edits contains an edit without file_path");
            };
            paths.insert(path);
        }
    }
    if tool_name == Some("apply_patch") {
        let Some(command) = optional_string(input, "command")? else {
            return Err("apply_patch is missing its command document");
        };
        paths.extend(apply_patch_paths(&command)?);
    }
    Ok(paths.into_iter().collect())
}

impl StdinEventFields {
    /// Project this already parsed event into the facts consumed by the
    /// write-boundary evaluator.
    ///
    /// The selected host and effect bindings decide whether observed path
    /// fields belong to a structured write. This projection grants no tool,
    /// filesystem, operation, or native-host authority.
    pub(crate) fn pre_tool_use_facts<'a>(
        &'a self,
        host: TerminalAdapterDescriptor,
        bindings: &[HostToolEffectBinding],
    ) -> PreToolUseFacts<'a> {
        PreToolUseFacts {
            malformed: false,
            tool_name: self.tool_name(),
            write_paths: self.structured_write_paths(host, bindings),
            cwd: self.cwd(),
            tool_input: self.tool_input(),
            command: self.command(),
        }
    }

    /// Return targets only when the native protocol or a sealed binding proves a
    /// structured write. A read tool may carry the same `file_path` field without
    /// creating write-state transitions.
    pub fn structured_write_paths(
        &self,
        host: TerminalAdapterDescriptor,
        bindings: &[HostToolEffectBinding],
    ) -> &[String] {
        self.tool_name
            .as_deref()
            .filter(|name| {
                crate::host_tool::is_structured_write(
                    host,
                    bindings,
                    name,
                    self.tool_input.as_ref(),
                )
            })
            .map_or(&[], |_| self.target_paths.as_slice())
    }
}
