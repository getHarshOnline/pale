//! One compiled-target and body check shared by native ingress and dispatch.

use crate::actions::{NativeActionInvocationError, NativeActionRegistry, PreparedNativeAction};
use crate::semantic::ResolvedOperation;
use pale_core::records::{
    CapabilityIdV1, CapabilityProviderBinding, CommonRequestEnvelopeV1, EffectClass,
};

pub(super) enum NativeRequestError {
    Association,
    Implementation,
    NativeAction(NativeActionInvocationError),
}

/// Prepare the exact selected body without executing its handler.
/// The registry owns schema admission, input kind and request digest. Ingress
/// drops this call after validation; dispatch retains its separately prepared
/// call behind custody. Rechecking at these boundaries avoids carrying a raw
/// executable call through the operation-start and checkpoint APIs.
/// Start admission and attempt-selection validation own the complete local
/// request/operation association before either exposes its prepared value.
pub(super) fn prepare_native_request<'registry>(
    operation: &ResolvedOperation,
    capability: &CapabilityIdV1,
    request: &CommonRequestEnvelopeV1,
    registry: &'registry NativeActionRegistry,
) -> Result<PreparedNativeAction<'registry>, NativeRequestError> {
    let target = selected_target(operation, capability)?;
    let body = request.body();
    let input_schema = pale_contracts::schema_descriptor("action-input.v1").map_err(|error| {
        NativeRequestError::NativeAction(NativeActionInvocationError::InputSchema(error))
    })?;
    if body.schema_id.as_str() != input_schema.schema_id
        || body.schema_sha256.as_str() != input_schema.schema_sha256
        || request.capability_id() != capability
    {
        return Err(NativeRequestError::Association);
    }
    request
        .verify()
        .map_err(|_| NativeRequestError::Association)?;
    registry
        .prepare_value(target, body.payload.clone())
        .map_err(NativeRequestError::NativeAction)
}

fn selected_target<'a>(
    operation: &'a ResolvedOperation,
    capability: &CapabilityIdV1,
) -> Result<&'a str, NativeRequestError> {
    let mut selected = None;
    for requirement in operation
        .capabilities()
        .iter()
        .filter(|row| &row.capability_id == capability)
    {
        let binding = requirement
            .provider
            .binding()
            .ok_or(NativeRequestError::Implementation)?;
        if requirement.effect_class != EffectClass::Compute
            || binding.effect_class != EffectClass::Compute
        {
            return Err(NativeRequestError::Implementation);
        }
        let CapabilityProviderBinding::NativeRust { target } = &binding.implementation else {
            return Err(NativeRequestError::Implementation);
        };
        if selected.is_some_and(|previous| previous != target.as_str()) {
            return Err(NativeRequestError::Implementation);
        }
        selected = Some(target.as_str());
    }
    selected.ok_or(NativeRequestError::Implementation)
}
