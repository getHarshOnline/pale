//! Closed registry binding each action ID to one contract and compiled handler.

use std::collections::BTreeMap;

use pale_core::records::{
    ActionContract, ActionError, ActionInput, ActionInputKind, ActionOutput, ActionOutputKind,
    ActionRequest,
};

mod catalog;
mod execution;
mod recorded;
mod request;
mod seal;

/// Executable behavior generation bound into every built-in action contract.
///
/// Behavior changes move contract identity independently of record shape.
const ACTION_BEHAVIOR_VERSION: u32 = 1;

type NativeExecutor = fn(&ActionInput) -> Result<ActionOutput, ActionError>;

/// One authored definition owns the contract projection and compiled executor.
struct NativeActionDefinition {
    id: &'static str,
    description: &'static str,
    input_kind: ActionInputKind,
    output_kind: ActionOutputKind,
    execute: NativeExecutor,
}

#[derive(Debug)]
struct NativeAction {
    contract: ActionContract,
    execute: NativeExecutor,
}

/// One schema-checked pure call, bound to its registry contract and input.
///
/// Preparation does not run the handler. [`Self::execute`] consumes this value;
/// dropping it runs nothing. The registry remains borrowed until the call is
/// executed or dropped. This proves neither route authority nor durable start.
#[doc = include_str!("../../../tests/actions/preparation.md")]
#[derive(Debug)]
#[must_use = "execute the prepared call or drop it without running the handler"]
pub struct PreparedNativeAction<'a> {
    action: &'a NativeAction,
    request: ActionRequest,
}

impl PreparedNativeAction<'_> {
    /// Read the sealed contract of the handler that will execute this input.
    #[must_use]
    pub fn contract(&self) -> &ActionContract {
        &self.action.contract
    }

    /// Read the exact typed input and request identity retained for execution.
    ///
    /// The borrowed record cannot replace the input or change its digest.
    #[must_use]
    pub fn request(&self) -> &ActionRequest {
        &self.request
    }
}

/// One existing payload projection and its diagnostic-only root union coordinate.
#[derive(Debug)]
struct ActionPayloadSchema {
    branch: usize,
    schema: serde_json::Map<String, serde_json::Value>,
}

/// Immutable built-in action catalog and its exact payload-schema projections.
///
/// Registry membership proves only that deterministic mechanics exist. Project
/// selection, route admission, and interpretation remain separate boundaries.
#[derive(Debug)]
pub struct NativeActionRegistry {
    actions: BTreeMap<String, NativeAction>,
    payload_schemas: BTreeMap<ActionInputKind, ActionPayloadSchema>,
    catalog_bytes: Vec<u8>,
}

#[cfg(test)]
#[path = "../../../tests/internal/actions/preparation.rs"]
mod preparation_tests;
