//! Exact private attempt data, not dispatch permission or recovered authority.

use pale_core::{canon, records::*};
use pale_ports::{
    ControlRecord, ControlRecordBytes, ControlRecordCoordinate, ControlRecordDigest,
    ProviderInvocationCoordinate, ProviderInvocationRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::provider_invocation::{
    AdmittedProviderInvocationFailure, AdmittedProviderInvocationSuccess,
    ProviderInvocationAdmissionContext,
};
use crate::resident::{AdmittedCompiledGeneration, IssuedLocalOperation};

mod native;
use native::NativeSelection;

/// Private storage role, deliberately absent from the public contract bundle.
pub(super) const SELECTION_SCHEMA: &str = pale_ports::PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA;

/// Original operation parents. Their custody and current authority are checked
/// by the dispatch phase, not inferred from this reference bundle.
#[derive(Clone, Copy)]
pub(super) struct AttemptSelectionContext<'a> {
    pub operation: &'a IssuedLocalOperation,
    pub request_ref: &'a ResidentRecordRefV1,
    pub request: &'a CommonRequestEnvelopeV1,
    pub session: &'a SessionAdmissionLeaseV1,
    pub lease: &'a OperationLeaseV1,
    pub generation: &'a AdmittedCompiledGeneration,
}

/// Actual source-selected request and admission context after authority checks.
pub(super) struct AttemptSelectionInput<'a> {
    pub parents: AttemptSelectionContext<'a>,
    pub invocation: ProviderInvocationRequest<'a>,
    pub admission: ProviderInvocationAdmissionContext<'a>,
}

/// Frozen selection bytes; construction alone establishes no custody or permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct AttemptSelection {
    handle_ref: ResidentRecordRefV1,
    handle_sha256: OperationHandleDigest,
    request_ref: ResidentRecordRefV1,
    request_sha256: CommonRequestEnvelopeDigest,
    attempt_id: CapabilityAttemptIdV1,
    implementation: AttemptImplementation,
    implementation_kind: BoundedResidentTextV1,
    implementation_version: BoundedResidentTextV1,
    // Outer CapabilityResult schema, not the successful payload's schema.
    result_schema_id: SchemaId,
    result_schema_sha256: SchemaDigest,
    result_authority: SigningAuthorityBindingV1,
}

/// The two executable owners have different identities; native actions do not
/// acquire a provider binding merely because they share attempt custody.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum AttemptImplementation {
    HostNative { provider: ProviderBindingRefV1 },
    NativeRust(NativeSelection),
}

#[derive(Debug, thiserror::Error)]
pub(super) enum AttemptSelectionError {
    #[error("attempt selection exceeds the caller's byte limit")]
    BoundExceeded,
    #[error("attempt selection has an invalid private encoding or record identity")]
    Encoding,
    #[error("attempt selection differs from its original operation or request")]
    Association,
    #[error("attempt selection is not an exact compiled compute implementation")]
    Implementation,
    #[error(transparent)]
    NativeAction(#[from] crate::actions::NativeActionInvocationError),
    #[error("result differs from the retained attempt selection")]
    Result,
}

impl AttemptSelection {
    /// Capture the actual selected request, never an independently supplied provider body.
    /// The caller still owns current route, contextual readiness, and signer admission.
    pub(super) fn capture(input: AttemptSelectionInput<'_>) -> Result<Self, AttemptSelectionError> {
        input
            .admission
            .validate_for(input.invocation)
            .map_err(|_| AttemptSelectionError::Association)?;
        input
            .admission
            .success_context()
            .contract_set
            .verify()
            .map_err(|_| AttemptSelectionError::Association)?;
        if input
            .invocation
            .request_binding()
            .contract_set_sha256()
            .as_str()
            != input
                .admission
                .success_context()
                .contract_set
                .canonical_contract_set_sha256()
                .as_str()
        {
            return Err(AttemptSelectionError::Association);
        }
        let (handle_ref, handle) = input.parents.operation.handle();
        let result = &input.admission.success_context().result;
        let selection = Self {
            handle_ref: handle_ref.clone(),
            handle_sha256: handle.canonical_operation_handle_sha256().clone(),
            request_ref: input.parents.request_ref.clone(),
            request_sha256: input.parents.request.canonical_request_sha256().clone(),
            attempt_id: input.invocation.attempt_id().clone(),
            implementation: AttemptImplementation::HostNative {
                provider: ProviderBindingRefV1 {
                    provider_binding_id: input.invocation.provider_binding().id.clone(),
                    provider_binding_identity: input
                        .invocation
                        .provider_binding()
                        .binding_identity
                        .clone(),
                },
            },
            implementation_kind: input.invocation.implementation_kind().clone(),
            implementation_version: input.invocation.implementation_version().clone(),
            result_schema_id: SchemaId::parse(CAPABILITY_RESULT_V1_SCHEMA)
                .map_err(|_| AttemptSelectionError::Encoding)?,
            result_schema_sha256: result.expected_schema_sha256.clone(),
            result_authority: result.signing_authority.clone(),
        };
        let selected = selection.provider_request(input.parents)?;
        if selected.compiled_generation_digest() != input.invocation.compiled_generation_digest()
            || selected.instance_digest() != input.invocation.instance_digest()
            || selected.operation() != input.invocation.operation()
            || selected.request_binding() != input.invocation.request_binding()
            || selected.provider_binding() != input.invocation.provider_binding()
        {
            return Err(AttemptSelectionError::Association);
        }
        Ok(selection)
    }

    pub(super) fn attempt_id(&self) -> &CapabilityAttemptIdV1 {
        &self.attempt_id
    }
    pub(super) fn provider(&self) -> Option<&ProviderBindingRefV1> {
        match &self.implementation {
            AttemptImplementation::HostNative { provider } => Some(provider),
            AttemptImplementation::NativeRust(_) => None,
        }
    }

    pub(super) fn implementation_kind(&self) -> &BoundedResidentTextV1 {
        &self.implementation_kind
    }

    pub(super) fn implementation_version(&self) -> &BoundedResidentTextV1 {
        &self.implementation_version
    }

    pub(super) fn result_schema(&self) -> (&SchemaId, &SchemaDigest) {
        (&self.result_schema_id, &self.result_schema_sha256)
    }

    pub(super) fn result_authority(&self) -> &SigningAuthorityBindingV1 {
        &self.result_authority
    }

    pub(super) fn native_payload_schema(&self) -> Option<(&SchemaId, &SchemaDigest)> {
        match &self.implementation {
            AttemptImplementation::NativeRust(native) => Some(native.payload_schema()),
            AttemptImplementation::HostNative { .. } => None,
        }
    }

    pub(super) fn native_action_identity(&self) -> Option<(&str, &ActionRequestDigest)> {
        match &self.implementation {
            AttemptImplementation::NativeRust(native) => Some(native.action_identity()),
            AttemptImplementation::HostNative { .. } => None,
        }
    }

    /// Bind one pure native call to the exact original request without executing it.
    pub(super) fn capture_native<'registry>(
        context: AttemptSelectionContext<'_>,
        registry: &'registry crate::actions::NativeActionRegistry,
        attempt_id: &CapabilityAttemptIdV1,
        admission: CapabilitySuccessAdmissionContextV1<'_>,
    ) -> Result<(Self, crate::actions::PreparedNativeAction<'registry>), AttemptSelectionError>
    {
        native::capture(context, registry, attempt_id, admission)
    }

    /// Encode a bounded private record under its actual source-derived schema.
    /// This schema is not a public contract or an authority signature.
    pub(super) fn record(
        &self,
        reference: ResidentRecordRefV1,
        maximum_bytes: usize,
    ) -> Result<ControlRecord, AttemptSelectionError> {
        let bytes = canon::to_canonical_string(self)
            .map_err(|_| AttemptSelectionError::Encoding)?
            .into_bytes();
        if bytes.len() > maximum_bytes {
            return Err(AttemptSelectionError::BoundExceeded);
        }
        Ok(ControlRecord::new(
            ControlRecordCoordinate::new(
                reference,
                ControlRecordDigest::parse(pale_core::sha256_hex(&bytes))
                    .map_err(|_| AttemptSelectionError::Encoding)?,
            ),
            SchemaId::parse(SELECTION_SCHEMA).map_err(|_| AttemptSelectionError::Encoding)?,
            Self::schema_digest()?,
            ControlRecordBytes::new(bytes).map_err(|_| AttemptSelectionError::Encoding)?,
        ))
    }

    /// Decode exact bytes only. The caller must prove the signed observation,
    /// its checkpoint/anchor and original co-commit before treating this as history.
    pub(super) fn decode(
        record: &ControlRecord,
        expected: &ImmutableRecordReferenceV1,
        maximum_bytes: usize,
    ) -> Result<Self, AttemptSelectionError> {
        let bytes = record.canonical_record_jcs().as_slice();
        if bytes.len() > maximum_bytes {
            return Err(AttemptSelectionError::BoundExceeded);
        }
        if record.coordinate().record_ref() != &expected.record_ref
            || record.coordinate().record_sha256().as_str() != expected.record_sha256.as_str()
            || record.schema_id().as_str() != SELECTION_SCHEMA
            || record.schema_sha256() != &Self::schema_digest()?
            || pale_core::sha256_hex(bytes) != expected.record_sha256.as_str()
        {
            return Err(AttemptSelectionError::Encoding);
        }
        let value = canon::parse_strict(bytes).map_err(|_| AttemptSelectionError::Encoding)?;
        let selection: Self =
            serde_json::from_value(value).map_err(|_| AttemptSelectionError::Encoding)?;
        if selection.result_schema_id.as_str() != CAPABILITY_RESULT_V1_SCHEMA
            || canon::to_canonical_string(&selection)
                .map_err(|_| AttemptSelectionError::Encoding)?
                .as_bytes()
                != bytes
        {
            return Err(AttemptSelectionError::Encoding);
        }
        Ok(selection)
    }

    // Derive nested field schemas from their existing owners. Changing the
    // typed codec changes this canonical schema identity without a hand-kept
    // field inventory or a public registry entry.
    fn schema_digest() -> Result<SchemaDigest, AttemptSelectionError> {
        static DIGEST: OnceLock<Result<SchemaDigest, ()>> = OnceLock::new();
        DIGEST
            .get_or_init(|| {
                let schema = schemars::schema_for!(AttemptSelection);
                let canonical = canon::to_canonical_string(&schema).map_err(|_| ())?;
                SchemaDigest::parse(pale_core::sha256_hex(canonical.as_bytes())).map_err(|_| ())
            })
            .as_ref()
            .cloned()
            .map_err(|_| AttemptSelectionError::Encoding)
    }

    /// Rejoin immutable parents without renewing their authority or execution time.
    pub(super) fn validate(
        &self,
        context: &AttemptSelectionContext<'_>,
    ) -> Result<(), AttemptSelectionError> {
        let (reference, handle) = context.operation.handle();
        context
            .lease
            .verify_against_parent(context.session)
            .map_err(|_| AttemptSelectionError::Association)?;
        let binding = context
            .request
            .local_request_binding()
            .map_err(|_| AttemptSelectionError::Association)?;
        let generation = context.generation.record();
        if reference != &self.handle_ref
            || handle.canonical_operation_handle_sha256() != &self.handle_sha256
            || context.request_ref != &self.request_ref
            || context.request.canonical_request_sha256() != &self.request_sha256
            || context.operation.request_coordinate().record_ref() != &self.request_ref
            || context
                .operation
                .request_coordinate()
                .record_sha256()
                .as_str()
                != self.request_sha256.as_str()
            || handle.request_binding() != &binding
            || handle.operation_lease_sha256() != context.lease.operation_lease_digest()
            || handle.compiled_generation_sha256() != generation.compiled_generation_digest()
            || handle.compiled_generation_sha256() != context.session.compiled_generation_digest()
            || handle.instance_sha256() != generation.instance_digest()
            || handle.instance_sha256() != context.session.instance_digest()
            || !context
                .lease
                .capabilities()
                .contains(binding.capability_id())
            || !context.lease.effects().contains(&EffectClass::Compute)
        {
            return Err(AttemptSelectionError::Association);
        }
        Ok(())
    }

    /// Resolve the frozen provider once for preparation, invocation and historical recovery.
    pub(super) fn provider_request<'a>(
        &'a self,
        context: AttemptSelectionContext<'a>,
    ) -> Result<ProviderInvocationRequest<'a>, AttemptSelectionError> {
        self.validate(&context)?;
        let selected_provider = self
            .provider()
            .ok_or(AttemptSelectionError::Implementation)?;
        let (_, handle) = context.operation.handle();
        let binding = handle.request_binding();
        let operation = context
            .generation
            .operation(context.session.repository_id(), context.lease.operation())
            .ok_or(AttemptSelectionError::Association)?;
        let mut matched = false;
        for requirement in operation
            .capabilities()
            .iter()
            .filter(|row| &row.capability_id == binding.capability_id())
        {
            let capability = requirement
                .provider
                .binding()
                .ok_or(AttemptSelectionError::Implementation)?;
            if requirement.effect_class != EffectClass::Compute
                || capability.effect_class != EffectClass::Compute
            {
                return Err(AttemptSelectionError::Implementation);
            }
            let CapabilityProviderBinding::HostNative { provider_tools } =
                &capability.implementation
            else {
                return Err(AttemptSelectionError::Implementation);
            };
            for row in provider_tools {
                if &row.provider_binding == selected_provider {
                    if row.tools.is_empty() {
                        return Err(AttemptSelectionError::Implementation);
                    }
                    matched = true;
                }
            }
        }
        if !matched {
            return Err(AttemptSelectionError::Implementation);
        }
        let provider = context
            .generation
            .provider_binding(&selected_provider.provider_binding_id)
            .ok_or(AttemptSelectionError::Implementation)?;
        if provider.id != selected_provider.provider_binding_id
            || provider.binding_identity != selected_provider.provider_binding_identity
        {
            return Err(AttemptSelectionError::Implementation);
        }
        ProviderInvocationRequest::new(
            ProviderInvocationCoordinate::new(
                context.generation.record().compiled_generation_digest(),
                context.generation.record().instance_digest(),
                context.lease.operation(),
                binding,
                &self.attempt_id,
            ),
            provider,
            &self.implementation_kind,
            &self.implementation_version,
        )
        .map_err(|_| AttemptSelectionError::Implementation)
    }

    /// Check an already admitted result against the selection retained before dispatch.
    pub(super) fn validate_success(
        &self,
        success: &AdmittedProviderInvocationSuccess,
        context: &AttemptSelectionContext<'_>,
    ) -> Result<(), AttemptSelectionError> {
        success
            .coordinate()
            .validate_for(self.provider_request(*context)?)
            .map_err(|_| AttemptSelectionError::Result)?;
        let result = success.result();
        if result.schema_id() != &self.result_schema_id
            || result.schema_sha256() != &self.result_schema_sha256
            || result.signing_authority_binding() != &self.result_authority
            || result.attempt_id() != &self.attempt_id
            || result.implementation_kind() != &self.implementation_kind
            || result.implementation_version() != &self.implementation_version
            || result.request_binding() != context.operation.handle().1.request_binding()
        {
            return Err(AttemptSelectionError::Result);
        }
        Ok(())
    }

    /// Check admitted safe-failure evidence against the pre-dispatch selection.
    pub(super) fn validate_failure(
        &self,
        failure: &AdmittedProviderInvocationFailure,
        context: &AttemptSelectionContext<'_>,
    ) -> Result<(), AttemptSelectionError> {
        failure
            .coordinate()
            .validate_for(self.provider_request(*context)?)
            .map_err(|_| AttemptSelectionError::Result)?;
        let result = failure.result();
        let adapter = failure.failure();
        let Some(reference) = result.failure() else {
            return Err(AttemptSelectionError::Result);
        };
        if result.schema_id() != &self.result_schema_id
            || result.schema_sha256() != &self.result_schema_sha256
            || result.signing_authority_binding() != &self.result_authority
            || result.attempt_id() != &self.attempt_id
            || result.implementation_kind() != &self.implementation_kind
            || result.implementation_version() != &self.implementation_version
            || result.request_binding() != context.operation.handle().1.request_binding()
            || reference.failure_ref() != failure.failure_ref()
            || reference.failure_sha256() != adapter.canonical_failure_sha256()
            || adapter.request_binding() != context.operation.handle().1.request_binding()
            || adapter.attempt_id() != &self.attempt_id
            || adapter.implementation_kind() != &self.implementation_kind
            || adapter.implementation_version() != &self.implementation_version
        {
            return Err(AttemptSelectionError::Result);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/operation_dispatch_selection.rs"]
pub(in crate::resident) mod tests;
