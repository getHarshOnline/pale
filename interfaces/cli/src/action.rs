//! Bounded CLI boundary for deterministic native actions.

use std::io::Read;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use pale_sdk::{LocalNativeActionInputRequest, NativeActionRequest};

use crate::cli::ResultFormatArg;
use crate::command::{CommandDispatcher, CommandOutcome, CommandRequest};
use crate::DispatchFailure;

/// List effect-free built-in computations from the canonical SDK catalog.
pub(super) fn list_actions(
    result_format: ResultFormatArg,
) -> std::result::Result<(), DispatchFailure> {
    let descriptors = action_catalog()?;
    let actions = descriptors
        .iter()
        .map(|descriptor| {
            serde_json::json!({
                "action_id": descriptor.action_id(),
                "description": descriptor.description(),
            })
        })
        .collect::<Vec<_>>();
    let result = serde_json::json!({ "actions": actions });
    match result_format {
        ResultFormatArg::Json => crate::terminal::write_json_result(
            crate::terminal::ResultCompletion::Complete,
            &result,
            "action catalog",
        )
        .map_err(Into::into),
        ResultFormatArg::Human => {
            let mut output = String::from("Effect-free built-in computations:\n");
            for descriptor in descriptors {
                output.push_str(descriptor.action_id());
                output.push_str("  ");
                output.push_str(descriptor.description());
                output.push('\n');
            }
            output.pop();
            crate::terminal::write_stdout(format_args!("{output}")).map_err(Into::into)
        }
    }
}

/// Describe one compiled computation without inventing a second input contract.
pub(super) fn describe_action(
    action_id: &str,
    result_format: ResultFormatArg,
) -> std::result::Result<(), DispatchFailure> {
    let descriptors = action_catalog()?;
    let descriptor = descriptors
        .iter()
        .find(|descriptor| descriptor.action_id() == action_id)
        .ok_or_else(|| {
            crate::diagnostic::invalid_input(
                "action ID is not in the compiled catalog; use `pale action list` to see available IDs",
                anyhow!("unknown effect-free computation `{action_id}`"),
            )
        })?;
    let contract = parse_catalog_json(
        descriptor.canonical_contract_bytes(),
        "native-action contract",
    )?;
    let payload_schema = parse_catalog_json(
        descriptor.canonical_payload_schema_bytes(),
        "native-action payload schema",
    )?;
    let input_kind = contract
        .get("input_kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            crate::diagnostic::internal_failure(
                "native-action contract has no string input kind",
                anyhow!("compiled action contract omitted its input_kind"),
            )
        })?;
    let cli_input_description = serde_json::json!({
        "schema_id": "action-input.v1",
        "kind": input_kind,
        "payload_schema": payload_schema,
    });
    let result = serde_json::json!({
        "action_id": descriptor.action_id(),
        "description": descriptor.description(),
        "contract": contract,
        "cli_input_description": cli_input_description,
    });
    match result_format {
        ResultFormatArg::Json => crate::terminal::write_json_result(
            crate::terminal::ResultCompletion::Complete,
            &result,
            "native-action description",
        )
        .map_err(Into::into),
        ResultFormatArg::Human => {
            let contract = serde_json::to_string_pretty(&contract).map_err(|error| {
                crate::diagnostic::internal_failure(
                    "native-action contract could not be rendered",
                    error.into(),
                )
            })?;
            let cli_input_description = serde_json::to_string_pretty(&cli_input_description)
                .map_err(|error| {
                    crate::diagnostic::internal_failure(
                        "native-action CLI input description could not be rendered",
                        error.into(),
                    )
                })?;
            crate::terminal::write_stdout(format_args!(
                "{}\n{}\n\nAction contract:\n{}\n\nHow to construct CLI input:\n{}",
                descriptor.action_id(),
                descriptor.description(),
                contract,
                cli_input_description,
            ))
            .map_err(Into::into)
        }
    }
}

/// Read the complete stable descriptor sequence through the shared dispatcher.
fn action_catalog() -> std::result::Result<Vec<pale_sdk::NativeActionDescriptor>, DispatchFailure> {
    let dispatcher =
        CommandDispatcher::new().map_err(|error| DispatchFailure::Host(error.into()))?;
    let outcome = dispatcher.dispatch(CommandRequest::NativeActionCatalog)?;
    let CommandOutcome::NativeActionCatalog(descriptors) = outcome else {
        return Err(crate::diagnostic::internal_failure(
            "native-action catalog returned an unrelated command result",
            anyhow!("closed command result did not match its request"),
        )
        .into());
    };
    Ok(descriptors)
}

/// Decode canonical catalog-owned bytes for terminal presentation only.
fn parse_catalog_json(
    bytes: &[u8],
    label: &'static str,
) -> std::result::Result<serde_json::Value, DispatchFailure> {
    serde_json::from_slice(bytes).map_err(|error| {
        crate::diagnostic::internal_failure(format!("{label} is not valid JSON"), error.into())
            .into()
    })
}

/// Selected process source for a native-action request body awaiting confinement.
#[derive(Clone, Copy)]
pub(super) enum ActionInput<'a> {
    /// Read the bounded request from standard input.
    Stdin,
    /// Capture one path beneath an already declared confinement root.
    Confined {
        /// Absolute root spelling passed to the API-owned confinement capability.
        root: &'a Path,
        /// Relative name resolved by the API without following a final symlink.
        relative: &'a Path,
    },
}

/// Execute a pure action from bounded strict JSON and emit a validated execution record.
pub(super) fn run_action(
    action_id: &str,
    source: ActionInput<'_>,
    result_format: ResultFormatArg,
) -> std::result::Result<(), DispatchFailure> {
    let request = match source {
        ActionInput::Stdin => CommandRequest::NativeAction(NativeActionRequest::new(
            action_id.to_owned(),
            read_stdin()?,
        )),
        ActionInput::Confined { root, relative } => {
            confined_action_request(action_id, root, relative)?
        }
    };
    let dispatcher =
        CommandDispatcher::new().map_err(|error| DispatchFailure::Host(error.into()))?;
    let outcome = dispatcher.dispatch(request)?;
    let CommandOutcome::NativeAction(execution) = outcome else {
        return Err(crate::diagnostic::internal_failure(
            "native action returned an unrelated command result",
            anyhow!("closed command result did not match its request"),
        )
        .into());
    };
    write_execution(&execution, result_format)
}

/// Translate one confined input selection for API-owned capture and invocation.
///
/// The CLI keeps only argv conversion and terminal presentation. Descriptor-relative
/// confinement, byte limits, and their structured diagnostics belong to the local API seam.
fn confined_action_request(
    action_id: &str,
    root: &Path,
    relative: &Path,
) -> std::result::Result<CommandRequest, DispatchFailure> {
    let root = root.to_str().ok_or_else(|| {
        crate::diagnostic::invalid_input(
            "action input root is not UTF-8",
            anyhow!("--input-root must be valid UTF-8"),
        )
    })?;
    let relative_path = relative.to_str().ok_or_else(|| {
        crate::diagnostic::invalid_input(
            "action input file is not UTF-8",
            anyhow!("--input-file must be valid UTF-8"),
        )
    })?;
    Ok(CommandRequest::LocalNativeAction(
        LocalNativeActionInputRequest::new(
            action_id.to_owned(),
            root.to_owned(),
            relative_path.to_owned(),
        ),
    ))
}

/// Render one API-owned native-action execution for human-readable inspection.
fn write_execution(
    execution: &pale_sdk::NativeActionExecution,
    format: ResultFormatArg,
) -> std::result::Result<(), DispatchFailure> {
    match format {
        ResultFormatArg::Human => {
            let output = crate::terminal::render_pretty_json(
                execution.canonical_bytes(),
                "native-action execution",
            )?;
            crate::terminal::write_stdout(format_args!("{output}")).map_err(Into::into)
        }
        ResultFormatArg::Json => {
            let result = crate::terminal::parse_owner_json(
                execution.canonical_bytes(),
                "native-action execution",
            )?;
            crate::terminal::write_json_result(
                crate::terminal::ResultCompletion::Complete,
                &result,
                "native-action execution",
            )
            .map_err(Into::into)
        }
    }
}

/// Read standard input through the same byte ceiling enforced by [`NativeActionRequest`].
fn read_stdin() -> Result<Vec<u8>> {
    let mut input = Vec::new();
    let bounded_read = u64::try_from(NativeActionRequest::MAX_BYTES)
        .context("native-action input ceiling does not fit the stream limit")?
        + 1;
    let stdin = std::io::stdin();
    stdin
        .lock()
        .take(bounded_read)
        .read_to_end(&mut input)
        .context("read action input")?;
    Ok(input)
}
