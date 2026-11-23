//! Structural ownership for repository-local native hook configuration.
//!
//! Pale owns only the handlers carrying its exact workspace/repository routing
//! grammar. The surrounding Claude Code or Codex configuration remains native
//! owner state and is preserved during installation, update, and removal.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use pale_core::records::{LogicalRepositoryIdV1, ProviderBindingRefV1, WorkspaceId};
use serde_json::{Map, Value};
use toml_edit::{value as toml_value, ArrayOfTables, DocumentMut, Item, Table};

use super::{
    harness_command_timeout_ceiling_secs, host_event_name, HookEvent, RepositoryHookIdentityV1,
    RepositoryHookRoutingV1,
};

/// Maximum native configuration admitted for an in-memory structural edit.
const MAX_NATIVE_HOOK_CONFIG_BYTES: usize = 2 * 1024 * 1024;

/// Failure to isolate one Pale contribution inside native-owner configuration.
#[derive(Debug, thiserror::Error)]
pub enum NativeHookConfigError {
    /// A bounded configuration input exceeded the admission ceiling.
    #[error("{input} exceeds the native hook configuration byte ceiling")]
    TooLarge {
        /// Non-sensitive input role, never source text or a filesystem path.
        input: &'static str,
    },
    /// Strict JSON ingestion rejected malformed or ambiguous bytes.
    #[error("{input} is not unambiguous JSON: {source}")]
    InvalidJson {
        /// Non-sensitive input role.
        input: &'static str,
        /// Duplicate-key, syntax, number, or trailing-byte failure.
        #[source]
        source: pale_core::canon::CanonError,
    },
    /// TOML ingestion rejected malformed or ambiguous bytes.
    #[error("{input} is not unambiguous TOML: {source}")]
    InvalidToml {
        /// Non-sensitive input role.
        input: &'static str,
        /// TOML syntax or duplicate-key failure.
        #[source]
        source: toml_edit::TomlError,
    },
    /// The native root cannot carry host hook settings.
    #[error("{input} must be a JSON object")]
    RootNotObject {
        /// Non-sensitive input role.
        input: &'static str,
    },
    /// The native hook collection has an unsafe shape for structural merging.
    #[error("{input} `hooks` member must be a JSON object")]
    HooksNotObject {
        /// Non-sensitive input role.
        input: &'static str,
    },
    /// Inline Codex hooks cannot be amended without replacing native-owner TOML.
    #[error("existing Codex `hooks` member must be a TOML table")]
    HooksNotTable,
    /// One event cannot be extended without replacing native-owner data.
    #[error("native hook event `{event}` must be a JSON array")]
    EventNotArray {
        /// Native host event name, not user-authored content.
        event: String,
    },
    /// One inline Codex event is not in the documented array-of-tables form.
    #[error("Codex inline hook event `{event}` must be an array of tables")]
    EventNotArrayOfTables {
        /// Native host event name, not user-authored content.
        event: String,
    },
    /// A matcher group's inline handlers are not in the documented form.
    #[error(
        "Codex inline hook event `{event}` has a `hooks` member that is not an array of tables"
    )]
    HandlersNotArrayOfTables {
        /// Native host event name, not user-authored content.
        event: String,
    },
    /// The same Codex layer would load two independent hook sources.
    #[error("Codex layer contains both `hooks.json` and inline `config.toml` hooks")]
    AmbiguousCodexRepresentation,
    /// The generated contribution is not the closed repository-routing grammar.
    #[error("invalid Pale hook contribution: {0}")]
    InvalidContribution(&'static str),
    /// Pretty serialization failed after a valid structural edit.
    #[error("native hook configuration serialization failed: {0}")]
    Serialize(#[source] serde_json::Error),
}

/// Native Codex hook representation selected for one repository layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexHookRepresentation {
    /// Repository-local `.codex/hooks.json`.
    HooksJson,
    /// Inline `[hooks]` tables in repository-local `.codex/config.toml`.
    InlineConfigToml,
}

/// One effect-free edit of the already selected Codex hook representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexHookConfigEdit {
    representation: CodexHookRepresentation,
    content: String,
}

enum SelectedCodexHookSource<'source> {
    HooksJson(&'source str),
    InlineConfigToml(DocumentMut),
    Absent,
}

impl CodexHookConfigEdit {
    /// Representation whose native-owner bytes this edit replaces.
    #[must_use]
    pub const fn representation(&self) -> CodexHookRepresentation {
        self.representation
    }

    /// Exact post-edit content for the separately admitted filesystem effect.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }
}

/// Merge one generated Pale handler contribution into native-owner JSON.
///
/// The operation is the keyed-replacement law rendered below. `E` is the
/// existing owner configuration and `D` the desired Pale contribution;
/// `preserve` retains every member outside the handlers at `D`'s key.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/native-hook-merge.display.mathml"
))]
///
/// Unknown top-level members, events, matcher groups, handlers, and array order
/// are retained semantically. JSON whitespace and object-key order are not
/// preserved by reserialization. Reapplying the same contribution is
/// idempotent. A changed executable or event set replaces the prior contribution
/// for the same workspace, repository, and exact provider binding without claiming the
/// surrounding file.
///
/// This function performs no filesystem mutation and grants no hook activation;
/// it computes the exact post-edit bytes for a separately admitted effect.
///
/// # Errors
///
/// Returns [`NativeHookConfigError`] when either input is too large or ambiguous,
/// the existing file cannot safely accept a hook member, or the contribution is
/// not exactly one Pale workspace/repository/provider-binding identity.
pub fn merge_native_json_hook_config(
    existing: Option<&str>,
    contribution: &str,
) -> Result<String, NativeHookConfigError> {
    let mut native = match existing {
        Some(source) => parse_object("existing native configuration", source)?,
        None => Map::new(),
    };
    let (desired_hooks, identity) = parse_contribution(contribution)?;

    remove_identity(&mut native, &identity)?;
    let native_hooks = native
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or(NativeHookConfigError::HooksNotObject {
            input: "existing native configuration",
        })?;
    for (event, desired_groups) in desired_hooks {
        let desired_groups =
            desired_groups
                .as_array()
                .ok_or_else(|| NativeHookConfigError::EventNotArray {
                    event: event.clone(),
                })?;
        native_hooks
            .entry(event.clone())
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .ok_or_else(|| NativeHookConfigError::EventNotArray {
                event: event.clone(),
            })?
            .extend(desired_groups.iter().cloned());
    }
    pretty_object(native)
}

/// Remove one repository-scoped Pale contribution from native-owner JSON.
///
/// Non-Pale handlers and every unrelated native setting are preserved. If
/// Pale owned the last hook member, the empty top-level `hooks` object is
/// removed; the caller remains responsible for deciding whether an otherwise
/// empty file should be retained.
///
/// # Errors
///
/// Returns [`NativeHookConfigError`] for an ambiguous JSON input or a native
/// `hooks`/event member that cannot be edited structurally.
pub fn remove_native_json_hook_config(
    existing: &str,
    workspace_id: &WorkspaceId,
    repository_id: &LogicalRepositoryIdV1,
    provider_binding: &ProviderBindingRefV1,
) -> Result<String, NativeHookConfigError> {
    let mut native = parse_object("existing native configuration", existing)?;
    remove_identity(
        &mut native,
        &RepositoryHookIdentityV1::new(
            workspace_id.clone(),
            repository_id.clone(),
            provider_binding.clone(),
        ),
    )?;
    pretty_object(native)
}

/// Merge a Pale contribution into the one hook representation selected by Codex.
///
/// A present `hooks.json` selects JSON unless `config.toml` already contains an
/// inline `hooks` table, in which case the layer is ambiguous and rejected.
/// When neither source exists, the returned edit creates `hooks.json`; merely
/// having a `config.toml` without inline hooks does not select the TOML form.
/// Existing inline TOML is edited with [`toml_edit`] so comments, unrelated
/// settings, ordering, and formatting remain native-owner state.
///
/// This function computes bytes only. Codex still owns project trust review and
/// activation of the changed hook definition.
///
/// # Errors
///
/// Returns [`NativeHookConfigError`] for oversized, malformed, structurally
/// unsafe, dual-source, or invalid-contribution input.
pub fn merge_codex_hook_config(
    hooks_json: Option<&str>,
    config_toml: Option<&str>,
    contribution: &str,
) -> Result<CodexHookConfigEdit, NativeHookConfigError> {
    match select_codex_hook_source(hooks_json, config_toml)? {
        SelectedCodexHookSource::HooksJson(existing) => Ok(CodexHookConfigEdit {
            representation: CodexHookRepresentation::HooksJson,
            content: merge_native_json_hook_config(Some(existing), contribution)?,
        }),
        SelectedCodexHookSource::InlineConfigToml(mut document) => {
            merge_codex_inline_document(&mut document, contribution)?;
            Ok(CodexHookConfigEdit {
                representation: CodexHookRepresentation::InlineConfigToml,
                content: document.to_string(),
            })
        }
        SelectedCodexHookSource::Absent => Ok(CodexHookConfigEdit {
            representation: CodexHookRepresentation::HooksJson,
            content: merge_native_json_hook_config(None, contribution)?,
        }),
    }
}

/// Remove one Pale contribution from the hook representation selected by Codex.
///
/// Returns `None` when the repository layer has no hook source. A `config.toml`
/// that contains settings but no inline hooks remains untouched and likewise
/// produces `None`.
///
/// # Errors
///
/// Returns [`NativeHookConfigError`] for oversized, malformed, structurally
/// unsafe, or dual-source input.
pub fn remove_codex_hook_config(
    hooks_json: Option<&str>,
    config_toml: Option<&str>,
    workspace_id: &WorkspaceId,
    repository_id: &LogicalRepositoryIdV1,
    provider_binding: &ProviderBindingRefV1,
) -> Result<Option<CodexHookConfigEdit>, NativeHookConfigError> {
    match select_codex_hook_source(hooks_json, config_toml)? {
        SelectedCodexHookSource::HooksJson(existing) => Ok(Some(CodexHookConfigEdit {
            representation: CodexHookRepresentation::HooksJson,
            content: remove_native_json_hook_config(
                existing,
                workspace_id,
                repository_id,
                provider_binding,
            )?,
        })),
        SelectedCodexHookSource::InlineConfigToml(mut document) => {
            remove_identity_from_toml(
                &mut document,
                &RepositoryHookIdentityV1::new(
                    workspace_id.clone(),
                    repository_id.clone(),
                    provider_binding.clone(),
                ),
            )?;
            Ok(Some(CodexHookConfigEdit {
                representation: CodexHookRepresentation::InlineConfigToml,
                content: document.to_string(),
            }))
        }
        SelectedCodexHookSource::Absent => Ok(None),
    }
}

fn select_codex_hook_source<'source>(
    hooks_json: Option<&'source str>,
    config_toml: Option<&str>,
) -> Result<SelectedCodexHookSource<'source>, NativeHookConfigError> {
    let parsed_toml = config_toml
        .map(|source| parse_toml_document("existing Codex config.toml", source))
        .transpose()?;
    match (hooks_json, parsed_toml) {
        (Some(_), Some(document)) if document.as_table().contains_key("hooks") => {
            Err(NativeHookConfigError::AmbiguousCodexRepresentation)
        }
        (Some(existing), _) => Ok(SelectedCodexHookSource::HooksJson(existing)),
        (None, Some(document)) if document.as_table().contains_key("hooks") => {
            Ok(SelectedCodexHookSource::InlineConfigToml(document))
        }
        (None, _) => Ok(SelectedCodexHookSource::Absent),
    }
}

fn parse_toml_document(
    input: &'static str,
    source: &str,
) -> Result<DocumentMut, NativeHookConfigError> {
    if source.len() > MAX_NATIVE_HOOK_CONFIG_BYTES {
        return Err(NativeHookConfigError::TooLarge { input });
    }
    source
        .parse::<DocumentMut>()
        .map_err(|source| NativeHookConfigError::InvalidToml { input, source })
}

fn merge_codex_inline_document(
    document: &mut DocumentMut,
    contribution: &str,
) -> Result<(), NativeHookConfigError> {
    let (desired_hooks, identity) =
        parse_contribution_for_adapter(contribution, TerminalAdapterDescriptor::codex())?;
    remove_identity_from_toml(document, &identity)?;

    if !document.as_table().contains_key("hooks") {
        document
            .as_table_mut()
            .insert("hooks", Item::Table(Table::new()));
    }
    let hooks = document
        .as_table_mut()
        .get_mut("hooks")
        .and_then(Item::as_table_mut)
        .ok_or(NativeHookConfigError::HooksNotTable)?;
    for (event, groups) in desired_hooks {
        let groups = groups
            .as_array()
            .ok_or_else(|| NativeHookConfigError::EventNotArray {
                event: event.clone(),
            })?;
        let handler = groups
            .first()
            .and_then(Value::as_object)
            .and_then(|group| group.get("hooks"))
            .and_then(Value::as_array)
            .and_then(|handlers| handlers.first())
            .and_then(Value::as_object)
            .ok_or(NativeHookConfigError::InvalidContribution(
                "Codex contribution does not contain one command handler",
            ))?;
        if !hooks.contains_key(event.as_str()) {
            hooks.insert(event.as_str(), Item::ArrayOfTables(ArrayOfTables::new()));
        }
        hooks
            .get_mut(event.as_str())
            .and_then(Item::as_array_of_tables_mut)
            .ok_or_else(|| NativeHookConfigError::EventNotArrayOfTables {
                event: event.clone(),
            })?
            .push(codex_toml_group(handler)?);
    }
    Ok(())
}

fn parse_contribution(
    contribution: &str,
) -> Result<(Map<String, Value>, RepositoryHookIdentityV1), NativeHookConfigError> {
    for adapter in crate::terminal_adapters::descriptors() {
        if let Ok(parsed) = parse_contribution_for_adapter(contribution, adapter) {
            return Ok(parsed);
        }
    }
    Err(NativeHookConfigError::InvalidContribution(
        "contribution does not match a registered native adapter codec",
    ))
}

fn parse_contribution_for_adapter(
    contribution: &str,
    harness: TerminalAdapterDescriptor,
) -> Result<(Map<String, Value>, RepositoryHookIdentityV1), NativeHookConfigError> {
    let mut desired = parse_object("generated Pale contribution", contribution)?;
    if desired.keys().any(|key| key != "hooks") {
        return Err(NativeHookConfigError::InvalidContribution(
            "the root may contain only `hooks`",
        ));
    }
    let Some(Value::Object(hooks)) = desired.remove("hooks") else {
        return Err(NativeHookConfigError::InvalidContribution(
            "the `hooks` object is required",
        ));
    };
    let identity = contribution_identity(&hooks, harness)?;
    Ok((hooks, identity))
}

fn codex_toml_group(handler: &Map<String, Value>) -> Result<Table, NativeHookConfigError> {
    let command = handler.get("command").and_then(Value::as_str).ok_or(
        NativeHookConfigError::InvalidContribution(
            "Codex command handler requires a string `command`",
        ),
    )?;
    let mut handler_table = Table::new();
    handler_table.insert("type", toml_value("command"));
    handler_table.insert("command", toml_value(command));
    if let Some(timeout) = handler.get("timeout") {
        let timeout = timeout.as_i64().filter(|timeout| *timeout >= 0).ok_or(
            NativeHookConfigError::InvalidContribution(
                "Codex command timeout must be a host-admitted positive integer",
            ),
        )?;
        handler_table.insert("timeout", toml_value(timeout));
    }
    let mut handlers = ArrayOfTables::new();
    handlers.push(handler_table);
    let mut group = Table::new();
    group.insert("hooks", Item::ArrayOfTables(handlers));
    Ok(group)
}

fn remove_identity_from_toml(
    document: &mut DocumentMut,
    identity: &RepositoryHookIdentityV1,
) -> Result<(), NativeHookConfigError> {
    let Some(hooks_item) = document.as_table_mut().get_mut("hooks") else {
        return Ok(());
    };
    let hooks = hooks_item
        .as_table_mut()
        .ok_or(NativeHookConfigError::HooksNotTable)?;
    let event_names = hooks
        .iter()
        .map(|(name, _)| name.to_owned())
        .collect::<Vec<_>>();
    for event_name in event_names {
        let Some(event) = event_for_host_name(&event_name) else {
            continue;
        };
        let groups = hooks
            .get_mut(&event_name)
            .and_then(Item::as_array_of_tables_mut)
            .ok_or_else(|| NativeHookConfigError::EventNotArrayOfTables {
                event: event_name.clone(),
            })?;
        let mut group_index = 0;
        while group_index < groups.len() {
            let group = groups
                .get_mut(group_index)
                .expect("index is bounded by the array-of-tables length");
            if group.iter().any(|(key, _)| key != "hooks") {
                group_index += 1;
                continue;
            }
            let handlers = group
                .get_mut("hooks")
                .and_then(Item::as_array_of_tables_mut)
                .ok_or_else(|| NativeHookConfigError::HandlersNotArrayOfTables {
                    event: event_name.clone(),
                })?;
            handlers
                .retain(|handler| toml_handler_identity(handler, event).as_ref() != Some(identity));
            if handlers.is_empty() {
                groups.remove(group_index);
            } else {
                group_index += 1;
            }
        }
        if groups.is_empty() {
            hooks.remove(&event_name);
        }
    }
    if hooks.is_empty() {
        document.as_table_mut().remove("hooks");
    }
    Ok(())
}

/// Recognize only the exact Codex handler shape that Pale generates.
///
/// Matching routing arguments do not establish ownership when the native owner
/// has added another field.
fn toml_handler_identity(handler: &Table, event: HookEvent) -> Option<RepositoryHookIdentityV1> {
    let harness = TerminalAdapterDescriptor::codex();
    if handler.get("type")?.as_str()? != "command"
        || !handler
            .iter()
            .all(|(key, _)| matches!(key, "type" | "command" | "timeout"))
        || handler.get("timeout").is_some_and(|timeout| {
            let Some(seconds) = timeout.as_integer() else {
                return true;
            };
            let Ok(seconds) = u64::try_from(seconds) else {
                return true;
            };
            let Some(ceiling) = harness_command_timeout_ceiling_secs(harness, event) else {
                return true;
            };
            seconds == 0 || seconds > u64::from(ceiling)
        })
    {
        return None;
    }
    let words = shlex::split(handler.get("command")?.as_str()?)?;
    routing_identity(&words.into_iter().skip(1).collect::<Vec<_>>(), event)
}

fn parse_object(
    input: &'static str,
    source: &str,
) -> Result<Map<String, Value>, NativeHookConfigError> {
    if source.len() > MAX_NATIVE_HOOK_CONFIG_BYTES {
        return Err(NativeHookConfigError::TooLarge { input });
    }
    pale_core::canon::parse_unique(source.as_bytes())
        .map_err(|source| NativeHookConfigError::InvalidJson { input, source })?
        .as_object()
        .cloned()
        .ok_or(NativeHookConfigError::RootNotObject { input })
}

fn contribution_identity(
    hooks: &Map<String, Value>,
    harness: TerminalAdapterDescriptor,
) -> Result<RepositoryHookIdentityV1, NativeHookConfigError> {
    let mut selected = None;
    let mut count = 0usize;
    for (event_name, groups) in hooks {
        let event = event_for_host_name(event_name).ok_or(
            NativeHookConfigError::InvalidContribution("event is not in the Pale vocabulary"),
        )?;
        if !super::harness_supports_event(harness, event) {
            return Err(NativeHookConfigError::InvalidContribution(
                "event is unavailable in the selected native harness protocol",
            ));
        }
        let groups = groups
            .as_array()
            .ok_or_else(|| NativeHookConfigError::EventNotArray {
                event: event_name.clone(),
            })?;
        if groups.len() != 1 {
            return Err(NativeHookConfigError::InvalidContribution(
                "each event must contain one matcher-free handler group",
            ));
        }
        let group = groups[0]
            .as_object()
            .ok_or(NativeHookConfigError::InvalidContribution(
                "handler group must be an object",
            ))?;
        if group.len() != 1 || !group.contains_key("hooks") {
            return Err(NativeHookConfigError::InvalidContribution(
                "generated handler groups may contain only `hooks`",
            ));
        }
        let handlers =
            group["hooks"]
                .as_array()
                .ok_or(NativeHookConfigError::InvalidContribution(
                    "handler collection must be an array",
                ))?;
        if handlers.len() != 1 {
            return Err(NativeHookConfigError::InvalidContribution(
                "each event must contain one Pale command handler",
            ));
        }
        if !generated_handler_shape(&handlers[0], harness, event) {
            return Err(NativeHookConfigError::InvalidContribution(
                "command handler contains fields outside the native Pale projection",
            ));
        }
        let identity = handler_identity(&handlers[0], event, harness).ok_or(
            NativeHookConfigError::InvalidContribution(
                "handler does not carry the exact Pale routing grammar",
            ),
        )?;
        if selected
            .as_ref()
            .is_some_and(|current| current != &identity)
        {
            return Err(NativeHookConfigError::InvalidContribution(
                "all handlers must route the same workspace and repository",
            ));
        }
        selected = Some(identity);
        count += 1;
    }
    if count == 0 {
        return Err(NativeHookConfigError::InvalidContribution(
            "at least one event is required",
        ));
    }
    selected.ok_or(NativeHookConfigError::InvalidContribution(
        "at least one handler is required",
    ))
}

fn generated_handler_shape(
    handler: &Value,
    harness: TerminalAdapterDescriptor,
    event: HookEvent,
) -> bool {
    let Some(handler) = handler.as_object() else {
        return false;
    };
    if handler.get("type").and_then(Value::as_str) != Some("command")
        || handler.get("command").and_then(Value::as_str).is_none()
        || !handler.keys().all(|key| match key.as_str() {
            "type" | "command" | "timeout" => true,
            "args" => harness == TerminalAdapterDescriptor::claude_code(),
            _ => false,
        })
        || handler.get("timeout").is_some_and(|timeout| {
            let Some(seconds) = timeout.as_u64() else {
                return true;
            };
            let Some(ceiling) = harness_command_timeout_ceiling_secs(harness, event) else {
                return true;
            };
            seconds == 0 || seconds > u64::from(ceiling)
        })
    {
        return false;
    }
    if harness.is_claude_code() {
        handler.get("args").is_some_and(|arguments| {
            arguments
                .as_array()
                .is_some_and(|arguments| arguments.iter().all(Value::is_string))
        })
    } else {
        !handler.contains_key("args")
    }
}

fn remove_identity(
    root: &mut Map<String, Value>,
    identity: &RepositoryHookIdentityV1,
) -> Result<(), NativeHookConfigError> {
    let Some(hooks) = root.get_mut("hooks") else {
        return Ok(());
    };
    let hooks = hooks
        .as_object_mut()
        .ok_or(NativeHookConfigError::HooksNotObject {
            input: "existing native configuration",
        })?;
    let event_names = hooks.keys().cloned().collect::<Vec<_>>();
    for event_name in event_names {
        let Some(event) = event_for_host_name(&event_name) else {
            continue;
        };
        let groups = hooks
            .get_mut(&event_name)
            .and_then(Value::as_array_mut)
            .ok_or_else(|| NativeHookConfigError::EventNotArray {
                event: event_name.clone(),
            })?;
        groups.retain_mut(|group| {
            let Some(group) = group.as_object_mut() else {
                return true;
            };
            // Pale contributes matcher-free groups. A command under a native
            // matcher is native-owner policy even if its argv happens to match.
            if group.keys().any(|key| key != "hooks") {
                return true;
            }
            let Some(handlers) = group.get_mut("hooks").and_then(Value::as_array_mut) else {
                return true;
            };
            handlers.retain(|handler| {
                !crate::terminal_adapters::descriptors().any(|adapter| {
                    // Routing equality is not ownership. Pale removes only its
                    // complete generated shape and preserves owner extensions.
                    generated_handler_shape(handler, adapter, event)
                        && handler_identity(handler, event, adapter).as_ref() == Some(identity)
                })
            });
            !handlers.is_empty()
        });
        if groups.is_empty() {
            hooks.remove(&event_name);
        }
    }
    if hooks.is_empty() {
        root.remove("hooks");
    }
    Ok(())
}

fn handler_identity(
    handler: &Value,
    event: HookEvent,
    harness: TerminalAdapterDescriptor,
) -> Option<RepositoryHookIdentityV1> {
    let handler = handler.as_object()?;
    if handler.get("type")?.as_str()? != "command" {
        return None;
    }
    let command = handler.get("command")?.as_str()?;
    let arguments: Vec<String> = if harness.is_claude_code() {
        handler
            .get("args")?
            .as_array()?
            .iter()
            .map(Value::as_str)
            .map(|argument| argument.map(ToOwned::to_owned))
            .collect::<Option<Vec<_>>>()?
    } else {
        let words = shlex::split(command)?;
        words.into_iter().skip(1).collect::<Vec<_>>()
    };
    routing_identity(&arguments, event)
}

fn routing_identity<T: AsRef<str>>(
    arguments: &[T],
    event: HookEvent,
) -> Option<RepositoryHookIdentityV1> {
    let routing = RepositoryHookRoutingV1::parse(arguments).ok()?;
    (routing.event() == event).then(|| routing.identity().clone())
}

fn event_for_host_name(name: &str) -> Option<HookEvent> {
    HookEvent::vocabulary().find(|event| host_event_name(*event) == name)
}

fn pretty_object(root: Map<String, Value>) -> Result<String, NativeHookConfigError> {
    let mut rendered = serde_json::to_string_pretty(&Value::Object(root))
        .map_err(NativeHookConfigError::Serialize)?;
    rendered.push('\n');
    Ok(rendered)
}

#[cfg(test)]
#[path = "../../tests/internal/hooks/native_config.rs"]
mod tests;
