//! Typed invocation boundary behind the owned `pale-sdk` façade.

use pale_core::records::{ActionExecution, ActionInput};

use super::{error::unknown_action_error, NativeActionRegistry, PreparedNativeAction};

/// Failure at the public boundary of one closed native action invocation.
///
/// [`Self::Action`] preserves a caller-safe domain rejection and
/// [`Self::MalformedInput`] retains its structured schema evidence. The remaining
/// variants report a Pale application or embedded-contract fault and must not be treated
/// as evidence that a requested computation ran.
#[derive(Debug, thiserror::Error)]
pub enum NativeActionInvocationError {
    /// Supplied action data, whether input or a recorded chain, failed schema admission.
    #[error("{0}")]
    MalformedInput(#[source] pale_contracts::SchemaError),
    /// The Pale application could not load or compile the schema used to admit input.
    #[error("native action input schema: {0}")]
    InputSchema(#[source] pale_contracts::SchemaError),
    /// A Pale application result violates its pinned JSON schema.
    #[error("native action execution schema: {0}")]
    ExecutionSchema(#[source] pale_contracts::SchemaError),
    /// The selected deterministic action rejected the typed request or failed.
    #[error(transparent)]
    Action(#[from] pale_core::records::ActionError),
    /// Compiled bindings and the embedded action-schema union disagree.
    #[error("native action registry invariant: {0}")]
    RegistryInvariant(String),
}

impl NativeActionRegistry {
    /// Execute one complete action-input value through the sealed registry.
    ///
    /// Schema rejection retains safe field-level evidence; deterministic handler
    /// failures retain their [`pale_core::records::ActionError`]. A successful execution
    /// is returned only after its wire schema has also been verified. Its receipt
    /// proves this pure computation only; it does not prove project selection,
    /// route admission, actor authority, or approval.
    ///
    /// # Errors
    ///
    /// Returns a structured action rejection for an unknown action or handler
    /// failure, and [`NativeActionInvocationError::MalformedInput`] for rejected
    /// schema input. Embedded-schema and registry mismatches
    /// remain Pale application errors and never produce a successful receipt.
    pub fn execute_value(
        &self,
        action_id: &str,
        value: serde_json::Value,
    ) -> Result<ActionExecution, NativeActionInvocationError> {
        self.prepare_value(action_id, value)?.execute()
    }

    /// Check an input and retain its exact registry handler without running it.
    ///
    /// This applies the same input schema, kind, and request-digest checks as
    /// [`Self::execute_value`]. Handler-specific checks still run at execution.
    /// The returned call borrows this registry and owns its typed input; it
    /// accepts no replacement target or input when executed.
    ///
    /// # Errors
    ///
    /// Returns the same pre-execution failures as [`Self::execute_value`].
    /// Preparation produces no result, receipt, authority, or durable operation.
    pub fn prepare_value(
        &self,
        action_id: &str,
        value: serde_json::Value,
    ) -> Result<PreparedNativeAction<'_>, NativeActionInvocationError> {
        if self.contract(action_id).is_none() {
            return Err(unknown_action_error(action_id).into());
        }
        if let Err(error) = pale_contracts::validate_value_with_root_branch(
            &value,
            "action-input.v1",
            self.input_branch(&value),
        ) {
            return Err(match error {
                error @ pale_contracts::SchemaError::Invalid(_) => {
                    NativeActionInvocationError::MalformedInput(error)
                }
                error => NativeActionInvocationError::InputSchema(error),
            });
        }
        let input: ActionInput = serde_json::from_value(value).map_err(|error| {
            NativeActionInvocationError::RegistryInvariant(format!(
                "schema-valid action input could not be decoded: {error}"
            ))
        })?;
        Ok(self.prepare_request(action_id, input)?)
    }

    /// Execute an object payload after binding it to the action's exact input kind.
    ///
    /// Unknown identities and payload failures use the same typed rejection
    /// path as [`Self::execute_value`]. The returned receipt proves
    /// the pure computation, not project selection, route admission, or approval.
    ///
    /// # Errors
    ///
    /// Returns the same structured action and Pale application failures as
    /// [`Self::execute_value`], including an unknown action identity before a
    /// request is constructed.
    pub fn execute_payload(
        &self,
        action_id: &str,
        payload: serde_json::Map<String, serde_json::Value>,
    ) -> Result<ActionExecution, NativeActionInvocationError> {
        let contract = self
            .contract(action_id)
            .ok_or_else(|| unknown_action_error(action_id))?;
        self.execute_value(
            action_id,
            serde_json::json!({
                "kind": contract.input_kind.as_str(),
                "payload": serde_json::Value::Object(payload),
            }),
        )
    }

    /// Return the strict payload schema sealed into this registry at construction.
    ///
    /// Unknown identities retain the structured action-error envelope; embedded
    /// schema drift is reported as a Pale application registry failure. A returned
    /// projection describes payload shape, not route or project availability.
    ///
    /// # Errors
    ///
    /// Returns a structured unknown-action error when `action_id` is not in the
    /// sealed registry, or a Pale application invariant error when its admitted
    /// input kind has no matching embedded payload schema.
    pub fn payload_schema(
        &self,
        action_id: &str,
    ) -> Result<serde_json::Map<String, serde_json::Value>, NativeActionInvocationError> {
        let kind = self
            .contract(action_id)
            .ok_or_else(|| unknown_action_error(action_id))?
            .input_kind;
        self.payload_schema_for(kind).cloned().ok_or_else(|| {
            NativeActionInvocationError::RegistryInvariant(format!(
                "missing sealed payload schema for input kind `{kind}`"
            ))
        })
    }
}

impl PreparedNativeAction<'_> {
    /// Consume this call, then verify and return its sealed computation chain.
    ///
    /// The result proves only the pure computation, not route authority,
    /// durable operation creation, or an external effect.
    ///
    /// # Errors
    ///
    /// Handler failures retain the prepared request digest. Invalid handler
    /// outputs, result sealing failures, and output-schema faults return the
    /// same errors as [`NativeActionRegistry::execute_value`].
    pub fn execute(self) -> Result<ActionExecution, NativeActionInvocationError> {
        let execution = self.execute_request()?;
        pale_contracts::validate_record(&execution, "action-execution.v1")
            .map_err(NativeActionInvocationError::ExecutionSchema)?;
        Ok(execution)
    }
}
