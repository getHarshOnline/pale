//! Recorded computation-chain validation without handler execution.

use pale_core::records::{
    ActionErrorCode, ActionExecution, ActionReceiptDigest, ActionResultDigest,
};

use super::seal::{ActionReceiptPreimage, ActionResultPreimage};
use super::PreparedNativeAction;
use crate::actions::{error::request_error, NativeActionInvocationError};

impl PreparedNativeAction<'_> {
    /// Check a recorded chain against this exact prepared call without running it.
    ///
    /// Success establishes schema, contract, input and digest consistency, not
    /// that the recorded output was computed honestly. The caller still owns
    /// result-signature admission and durable operation association. Historical
    /// contracts unsupported by this registry cannot be substituted here.
    pub(crate) fn validate_recorded_execution(
        &self,
        execution: &ActionExecution,
    ) -> Result<(), NativeActionInvocationError> {
        pale_contracts::validate_record(execution, "action-execution.v1").map_err(|error| {
            match error {
                error @ (pale_contracts::SchemaError::Invalid(_)
                | pale_contracts::SchemaError::IdentityMismatch(_)) => {
                    NativeActionInvocationError::MalformedInput(error)
                }
                error => NativeActionInvocationError::ExecutionSchema(error),
            }
        })?;
        let request = &self.request;
        let reject = |code, message| {
            NativeActionInvocationError::Action(request_error(
                &request.action_id,
                &request.request_digest,
                code,
                message,
            ))
        };
        if execution.request.action_id != request.action_id
            || execution.request.action_version != request.action_version
            || execution.request.contract_digest != request.contract_digest
            || execution.request.effect != request.effect
            || execution.result.action_id != request.action_id
            || execution.result.action_version != request.action_version
            || execution.result.contract_digest != request.contract_digest
            || execution.receipt.action_id != request.action_id
            || execution.receipt.action_version != request.action_version
            || execution.receipt.contract_digest != request.contract_digest
        {
            return Err(reject(
                ActionErrorCode::ContractMismatch,
                "recorded execution does not use the prepared action contract",
            ));
        }
        if execution.request != *request
            || execution.result.request_digest != request.request_digest
            || execution.receipt.request_digest != request.request_digest
            || execution.receipt.result_digest != execution.result.result_digest
        {
            return Err(reject(
                ActionErrorCode::DigestMismatch,
                "recorded execution does not retain the prepared request and result chain",
            ));
        }
        if execution.result.output.kind() != self.action.contract.output_kind {
            return Err(reject(
                ActionErrorCode::ContractMismatch,
                "recorded output kind does not match the prepared action contract",
            ));
        }
        let result_digest = ActionResultDigest::hash_canonical(&ActionResultPreimage::for_request(
            request,
            &execution.result.output,
        ))
        .map_err(|error| {
            NativeActionInvocationError::Action(request_error(
                &request.action_id,
                &request.request_digest,
                ActionErrorCode::CanonicalizationFailed,
                error.to_string(),
            ))
        })?;
        let receipt_digest = ActionReceiptDigest::hash_canonical(
            &ActionReceiptPreimage::for_request(request, &result_digest),
        )
        .map_err(|error| {
            NativeActionInvocationError::Action(request_error(
                &request.action_id,
                &request.request_digest,
                ActionErrorCode::CanonicalizationFailed,
                error.to_string(),
            ))
        })?;
        if execution.result.result_digest != result_digest
            || execution.receipt.receipt_digest != receipt_digest
        {
            return Err(reject(
                ActionErrorCode::DigestMismatch,
                "recorded result or receipt does not match its canonical digest",
            ));
        }
        Ok(())
    }
}
