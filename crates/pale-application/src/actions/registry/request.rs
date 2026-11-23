//! Strict request decoding, contract admission, and digest verification.

use pale_core::canon;
use pale_core::records::{
    ActionError, ActionErrorCode, ActionInput, ActionRequest, ActionRequestDigest,
};

use super::seal::ActionRequestPreimage;
use super::{NativeAction, NativeActionRegistry, PreparedNativeAction};
use crate::actions::{
    error::{action_error, unknown_action_error},
    ACTION_RECORD_SCHEMA_VERSION, REQUEST_RECORD,
};

impl NativeActionRegistry {
    /// Bind a typed input to the exact registered contract and request digest.
    pub(in crate::actions) fn prepare_request(
        &self,
        action_id: &str,
        input: ActionInput,
    ) -> Result<PreparedNativeAction<'_>, ActionError> {
        let action = self.require_action(action_id)?;
        if input.kind() != action.contract.input_kind {
            return Err(action_error(
                Some(action_id),
                ActionErrorCode::InputMismatch,
                format!(
                    "contract expects `{}`, request supplied `{}`",
                    action.contract.input_kind,
                    input.kind()
                ),
            ));
        }
        Self::validate_strict_input(action_id, &input)?;
        let contract_digest = action.contract.contract_digest.clone();
        let preimage = ActionRequestPreimage {
            schema: REQUEST_RECORD,
            schema_version: ACTION_RECORD_SCHEMA_VERSION,
            action_id,
            action_version: action.contract.version,
            contract_digest: &contract_digest,
            effect: action.contract.effect,
            input: &input,
        };
        let digest = ActionRequestDigest::hash_canonical(&preimage).map_err(|error| {
            action_error(
                Some(action_id),
                ActionErrorCode::CanonicalizationFailed,
                error.to_string(),
            )
        })?;
        let request = ActionRequest {
            schema: REQUEST_RECORD.to_string(),
            schema_version: ACTION_RECORD_SCHEMA_VERSION,
            action_id: action_id.to_string(),
            action_version: action.contract.version,
            contract_digest,
            effect: action.contract.effect,
            input,
            request_digest: digest,
        };
        Ok(PreparedNativeAction { action, request })
    }

    fn require_action(&self, action_id: &str) -> Result<&NativeAction, ActionError> {
        self.actions
            .get(action_id)
            .ok_or_else(|| unknown_action_error(action_id))
    }

    fn validate_strict_input(action_id: &str, input: &ActionInput) -> Result<(), ActionError> {
        let encoded = serde_json::to_vec(input).map_err(|error| {
            action_error(
                Some(action_id),
                ActionErrorCode::CanonicalizationFailed,
                error.to_string(),
            )
        })?;
        canon::parse_strict(&encoded).map_err(|error| {
            action_error(
                Some(action_id),
                ActionErrorCode::MalformedInput,
                error.to_string(),
            )
        })?;
        Ok(())
    }
}
