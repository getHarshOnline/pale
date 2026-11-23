//! Bounded JSON admission for untrusted native-harness event bytes.

use serde_json::{Map, Value};

use super::paths::target_paths;
use super::{PromptExpansionFields, StdinEvent, StdinEventFields};

/// Decode an optional string while rejecting a present value of another type.
pub(super) fn optional_string(
    object: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, &'static str> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err("a known event field has the wrong type"),
    }
}

fn optional_bool(object: &Map<String, Value>, key: &str) -> Result<Option<bool>, &'static str> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err("a known event field has the wrong type"),
    }
}

fn object_field<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a Map<String, Value>>, &'static str> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(value)) => Ok(Some(value)),
        Some(_) => Err("a known event object has the wrong type"),
    }
}

/// Parse one host event under the caller-supplied byte limit.
///
/// Empty input is absent. Oversized, non-UTF-8, non-object, malformed JSON,
/// trailing data, or malformed known fields are untrusted.
///
/// # Examples
///
/// ```
/// use pale_application::hooks::{parse_stdin_event, StdinEvent};
///
/// let event = parse_stdin_event(
///     br#"{"hook_event_name":"PreToolUse","tool_name":"Read"}"#,
///     1_024,
/// );
/// let StdinEvent::Fields(fields) = event else {
///     panic!("valid event must retain typed fields");
/// };
/// assert_eq!(fields.tool_name(), Some("Read"));
/// ```
pub fn parse_stdin_event(bytes: &[u8], limit: usize) -> StdinEvent {
    if bytes.len() > limit {
        return StdinEvent::Malformed("oversized event payload");
    }
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return StdinEvent::Absent;
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => return StdinEvent::Malformed("non-UTF-8 event payload"),
    };
    let value: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(_) => return StdinEvent::Malformed("invalid JSON event payload"),
    };
    let Value::Object(object) = value else {
        return StdinEvent::Malformed("event payload is not an object");
    };

    let parsed = (|| {
        let tool_input = object_field(&object, "tool_input")?;
        let nested_tool = object_field(&object, "tool")?;
        let nested_tool_name = nested_tool
            .map(|tool| optional_string(tool, "name"))
            .transpose()?
            .flatten();
        let top_level_tool_name = optional_string(&object, "tool_name")?;
        if top_level_tool_name.is_some()
            && nested_tool_name.is_some()
            && top_level_tool_name != nested_tool_name
        {
            return Err("conflicting native tool identities");
        }
        let tool_name = top_level_tool_name.or(nested_tool_name);
        let command = tool_input
            .map(|input| optional_string(input, "command"))
            .transpose()?
            .flatten();

        let expansion_type = optional_string(&object, "expansion_type")?;
        let command_name = optional_string(&object, "command_name")?;
        let command_args = optional_string(&object, "command_args")?;
        let command_source = optional_string(&object, "command_source")?;
        let expansion = [
            expansion_type.as_ref(),
            command_name.as_ref(),
            command_args.as_ref(),
            command_source.as_ref(),
        ]
        .iter()
        .any(|field| field.is_some())
        .then_some(PromptExpansionFields {
            expansion_type,
            command_name,
            command_args,
            command_source,
        });
        let target_paths = target_paths(tool_name.as_deref(), tool_input)?;

        Ok::<_, &'static str>(StdinEventFields {
            hook_event_name: optional_string(&object, "hook_event_name")?,
            tool_name,
            command,
            tool_input: tool_input.cloned().map(Value::Object),
            tool_response: object.get("tool_response").cloned(),
            tool_use_id: optional_string(&object, "tool_use_id")?,
            target_paths,
            message: optional_string(&object, "prompt")?,
            session_id: optional_string(&object, "session_id")?,
            turn_id: optional_string(&object, "turn_id")?,
            agent_type: optional_string(&object, "agent_type")?,
            cwd: optional_string(&object, "cwd")?,
            stop_hook_active: optional_bool(&object, "stop_hook_active")?,
            expansion,
        })
    })();

    match parsed {
        Ok(fields) => StdinEvent::Fields(Box::new(fields)),
        Err(reason) => StdinEvent::Malformed(reason),
    }
}
