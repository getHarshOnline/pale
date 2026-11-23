//! Exact native-action identity derived from compiled selection and the sealed registry.

use super::*;
use crate::actions::{NativeActionRegistry, PreparedNativeAction};
use crate::resident::native_request::{prepare_native_request, NativeRequestError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeSelection {
    action_id: BoundedResidentTextV1,
    action_version: u32,
    contract_digest: ActionContractDigest,
    request_digest: ActionRequestDigest,
    payload_schema_id: SchemaId,
    payload_schema_sha256: SchemaDigest,
}

impl NativeSelection {
    pub(super) fn action_identity(&self) -> (&str, &ActionRequestDigest) {
        (self.action_id.as_str(), &self.request_digest)
    }

    pub(super) fn payload_schema(&self) -> (&SchemaId, &SchemaDigest) {
        (&self.payload_schema_id, &self.payload_schema_sha256)
    }
}

pub(super) fn capture<'registry>(
    context: AttemptSelectionContext<'_>,
    registry: &'registry NativeActionRegistry,
    attempt_id: &CapabilityAttemptIdV1,
    admission: CapabilitySuccessAdmissionContextV1<'_>,
) -> Result<(AttemptSelection, PreparedNativeAction<'registry>), AttemptSelectionError> {
    let operation = context
        .generation
        .operation(context.session.repository_id(), context.lease.operation())
        .ok_or(AttemptSelectionError::Association)?;
    let call = prepare_native_request(
        operation,
        context
            .operation
            .handle()
            .1
            .request_binding()
            .capability_id(),
        context.request,
        registry,
    )
    .map_err(|error| match error {
        NativeRequestError::Association => AttemptSelectionError::Association,
        NativeRequestError::Implementation => AttemptSelectionError::Implementation,
        NativeRequestError::NativeAction(error) => AttemptSelectionError::NativeAction(error),
    })?;
    let contract = call.contract();
    let kind = BoundedResidentTextV1::parse(CapabilityProvider::NativeRust.as_str())
        .map_err(|_| AttemptSelectionError::Encoding)?;
    let version = BoundedResidentTextV1::parse(contract.version.to_string())
        .map_err(|_| AttemptSelectionError::Encoding)?;
    let outer = pale_contracts::record_schema_identity(CAPABILITY_RESULT_V1_SCHEMA_ID)
        .map_err(|_| AttemptSelectionError::Encoding)?;
    let payload = pale_contracts::record_schema_identity("action-execution.v1")
        .map_err(|_| AttemptSelectionError::Encoding)?;
    let (handle_ref, handle) = context.operation.handle();
    let expected = &admission.result;
    if expected.expected_request_binding != handle.request_binding()
        || expected.expected_attempt_id != attempt_id
        || expected.expected_implementation_kind != &kind
        || expected.expected_implementation_version != &version
        || expected.expected_schema_sha256 != &outer.schema_sha256
    {
        return Err(AttemptSelectionError::Association);
    }
    admission
        .validate_result_schema(&payload.schema, &payload.schema_sha256)
        .map_err(|_| AttemptSelectionError::Association)?;
    let selection = AttemptSelection {
        handle_ref: handle_ref.clone(),
        handle_sha256: handle.canonical_operation_handle_sha256().clone(),
        request_ref: context.request_ref.clone(),
        request_sha256: context.request.canonical_request_sha256().clone(),
        attempt_id: attempt_id.clone(),
        implementation: AttemptImplementation::NativeRust(NativeSelection {
            action_id: BoundedResidentTextV1::parse(&contract.id)
                .map_err(|_| AttemptSelectionError::Encoding)?,
            action_version: contract.version,
            contract_digest: contract.contract_digest.clone(),
            request_digest: call.request().request_digest.clone(),
            payload_schema_id: payload.schema,
            payload_schema_sha256: payload.schema_sha256,
        }),
        implementation_kind: kind,
        implementation_version: version,
        result_schema_id: outer.schema,
        result_schema_sha256: outer.schema_sha256,
        result_authority: expected.signing_authority.clone(),
    };
    // The existing selection owner proves the full local request binding,
    // including rejection of federation, before this call can be returned.
    selection.validate(&context)?;
    Ok((selection, call))
}
