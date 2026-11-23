//! Terminal projection of the API-owned physical control-store classifier.
//!
//! No workspace compilation, store initialization, writer acquisition, repair
//! or resident admission occurs here. A successful command reports the physical
//! classification, including rejection, rather than asserting system health.

use crate::cli::{ControlStoreInspectionArgs, ControlStoreInspectionScope, ResultFormatArg};
use crate::command::{CommandDispatcher, CommandOutcome, CommandRequest};
use crate::DispatchFailure;
use pale_sdk::LocalControlStorageInspectionRequest;

/// Translate argv, dispatch once and render the unchanged API-owned result.
pub(super) fn run(
    arguments: &ControlStoreInspectionArgs,
    format: ResultFormatArg,
) -> Result<(), DispatchFailure> {
    let root =
        crate::local_directory_argument(&arguments.runtime_state_root, "--runtime-state-root")?;
    let store = arguments.logical_store_id.clone();
    let incarnation = arguments.control_incarnation_id.clone();
    let request = match &arguments.scope {
        ControlStoreInspectionScope::Principal { principal_id } => {
            LocalControlStorageInspectionRequest::principal(
                root,
                principal_id.clone(),
                store,
                incarnation,
            )
        }
        ControlStoreInspectionScope::Deployment {
            workspace_id,
            deployment_binding_ref,
            deployment_binding_sha256,
        } => LocalControlStorageInspectionRequest::deployment(
            root,
            workspace_id.clone(),
            deployment_binding_ref.clone(),
            deployment_binding_sha256.clone(),
            store,
            incarnation,
        ),
    };
    let dispatcher =
        CommandDispatcher::new().map_err(|error| DispatchFailure::Host(error.into()))?;
    let outcome = dispatcher.dispatch(CommandRequest::LocalControlStorageInspection(request))?;
    let CommandOutcome::LocalControlStorageInspection(inspection) = outcome else {
        return Err(crate::diagnostic::internal_failure(
            "control-store inspection returned an unrelated command result",
            anyhow::anyhow!("closed command result did not match its request"),
        )
        .into());
    };
    match format {
        ResultFormatArg::Human => {
            let output = serde_json::to_string_pretty(&inspection).map_err(|error| {
                crate::diagnostic::internal_failure(
                    "the API-owned control-store inspection could not be rendered",
                    error.into(),
                )
            })?;
            crate::terminal::write_stdout(format_args!("{output}")).map_err(Into::into)
        }
        ResultFormatArg::Json => {
            let result = serde_json::to_value(&inspection).map_err(|error| {
                crate::diagnostic::internal_failure(
                    "the typed control-store inspection could not be encoded",
                    error.into(),
                )
            })?;
            crate::terminal::write_json_result(
                crate::terminal::ResultCompletion::Complete,
                &result,
                "control-store inspection",
            )
            .map_err(Into::into)
        }
    }
}
