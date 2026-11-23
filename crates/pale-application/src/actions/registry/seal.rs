//! Canonical digest preimages for successful action records.

use pale_core::records::{
    ActionBoundary, ActionContract, ActionContractDigest, ActionEffect, ActionInput,
    ActionInputKind, ActionOutput, ActionOutputKind, ActionRequest, ActionRequestDigest,
    ActionResultDigest,
};
use serde::Serialize;

#[derive(Serialize)]
/// Canonical action-contract fields before the self-digest is attached.
pub(super) struct ActionContractPreimage<'a> {
    pub(super) schema: &'a str,
    pub(super) schema_version: u32,
    pub(super) id: &'a str,
    pub(super) version: u32,
    pub(super) description: &'a str,
    pub(super) effect: ActionEffect,
    pub(super) boundary: ActionBoundary,
    pub(super) input_kind: ActionInputKind,
    pub(super) output_kind: ActionOutputKind,
}

impl ActionContractPreimage<'_> {
    /// Attach the separately computed digest without changing its preimage fields.
    pub(super) fn into_sealed(self, contract_digest: ActionContractDigest) -> ActionContract {
        ActionContract {
            schema: self.schema.to_string(),
            schema_version: self.schema_version,
            id: self.id.to_string(),
            version: self.version,
            description: self.description.to_string(),
            effect: self.effect,
            boundary: self.boundary,
            input_kind: self.input_kind,
            output_kind: self.output_kind,
            contract_digest,
        }
    }
}

#[derive(Serialize)]
/// Canonical request fields used to derive [`ActionRequestDigest`].
pub(super) struct ActionRequestPreimage<'a> {
    pub(super) schema: &'a str,
    pub(super) schema_version: u32,
    pub(super) action_id: &'a str,
    pub(super) action_version: u32,
    pub(super) contract_digest: &'a ActionContractDigest,
    pub(super) effect: ActionEffect,
    pub(super) input: &'a ActionInput,
}

#[derive(Serialize)]
/// Canonical result fields used to derive [`ActionResultDigest`].
pub(super) struct ActionResultPreimage<'a> {
    pub(super) schema: &'a str,
    pub(super) schema_version: u32,
    pub(super) action_id: &'a str,
    pub(super) action_version: u32,
    pub(super) contract_digest: &'a ActionContractDigest,
    pub(super) request_digest: &'a ActionRequestDigest,
    pub(super) output: &'a ActionOutput,
}

#[derive(Serialize)]
/// Canonical terminal fields used to derive [`ActionReceiptDigest`](pale_core::records::ActionReceiptDigest).
pub(super) struct ActionReceiptPreimage<'a> {
    pub(super) schema: &'a str,
    pub(super) schema_version: u32,
    pub(super) action_id: &'a str,
    pub(super) action_version: u32,
    pub(super) contract_digest: &'a ActionContractDigest,
    pub(super) request_digest: &'a ActionRequestDigest,
    pub(super) result_digest: &'a ActionResultDigest,
}

impl<'a> ActionResultPreimage<'a> {
    /// Bind an output to the exact prepared request without attaching a digest.
    pub(super) fn for_request(request: &'a ActionRequest, output: &'a ActionOutput) -> Self {
        Self {
            schema: crate::actions::RESULT_RECORD,
            schema_version: crate::actions::ACTION_RECORD_SCHEMA_VERSION,
            action_id: &request.action_id,
            action_version: request.action_version,
            contract_digest: &request.contract_digest,
            request_digest: &request.request_digest,
            output,
        }
    }
}

impl<'a> ActionReceiptPreimage<'a> {
    /// Bind a result digest to the exact prepared request.
    pub(super) fn for_request(
        request: &'a ActionRequest,
        result_digest: &'a ActionResultDigest,
    ) -> Self {
        Self {
            schema: crate::actions::RECEIPT_RECORD,
            schema_version: crate::actions::ACTION_RECORD_SCHEMA_VERSION,
            action_id: &request.action_id,
            action_version: request.action_version,
            contract_digest: &request.contract_digest,
            request_digest: &request.request_digest,
            result_digest,
        }
    }
}
