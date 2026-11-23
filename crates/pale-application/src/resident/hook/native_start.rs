//! Admit the exact native occurrence covered by an operation-start signature.

use pale_core::{
    canon,
    records::{
        ArtifactDigest, CommonRequestBodyV1, HarnessActivityIdentity, LifecycleEvent,
        NativeInvocationIdV1, NativeToolRequestV1, SchemaDigest, SchemaId,
        NATIVE_TOOL_REQUEST_V1_SCHEMA_ID,
    },
};

use crate::hooks::RepositoryHookRoutingV1;

use super::{ResidentHookIngressRequest, RESIDENT_HOOK_EVENT_BYTE_LIMIT};

/// A bounded native occurrence and route prepared for a signed `StartInvoke`.
///
/// This is input admission only. The enclosing common request must separately
/// prove its caller, current session, grant and requested capability. Neither
/// preparation nor a successful operation start permits the host to run a tool.
pub struct ResidentNativeToolStart {
    body: CommonRequestBodyV1,
    request: ResidentHookIngressRequest,
    invocation: NativeInvocationIdV1,
}

impl ResidentNativeToolStart {
    /// Preserve the original event bytes inside a digest-bound request body.
    ///
    /// # Errors
    /// Rejects non-pre-effect events, malformed or duplicate JSON fields,
    /// missing native activity or tool identity, and oversized input.
    pub fn prepare(
        routing: RepositoryHookRoutingV1,
        native_event: &[u8],
    ) -> Result<Self, ResidentNativeToolStartError> {
        let (request, invocation) = admit_event(routing, native_event)?;
        let route = request.routing();
        let identity = route.identity();
        let payload = serde_json::to_value(NativeToolRequestV1 {
            event: route.event(),
            workspace_id: identity.workspace_id().clone(),
            repository_id: identity.repository_id().clone(),
            provider_binding: identity.provider_binding().clone(),
            native_event: std::str::from_utf8(native_event)
                .map_err(|_| ResidentNativeToolStartError)?
                .to_owned(),
        })
        .map_err(|_| ResidentNativeToolStartError)?;
        pale_contracts::validate_value(&payload, NATIVE_TOOL_REQUEST_V1_SCHEMA_ID)
            .map_err(|_| ResidentNativeToolStartError)?;
        let descriptor = pale_contracts::schema_descriptor(NATIVE_TOOL_REQUEST_V1_SCHEMA_ID)
            .map_err(|_| ResidentNativeToolStartError)?;
        let body = CommonRequestBodyV1 {
            schema_id: SchemaId::parse(descriptor.schema_id)
                .map_err(|_| ResidentNativeToolStartError)?,
            schema_sha256: SchemaDigest::parse(descriptor.schema_sha256)
                .map_err(|_| ResidentNativeToolStartError)?,
            payload_sha256: payload_digest(&payload)?,
            payload,
        };
        Ok(Self {
            body,
            request,
            invocation,
        })
    }

    /// Admit a body already retained by the caller's signed common request.
    ///
    /// # Errors
    /// Rejects schema or digest substitution, unknown fields, invalid routing,
    /// and native input rejected by [`Self::prepare`]. This does not verify the
    /// enclosing signature or authenticate its carrier.
    pub fn admit(body: &CommonRequestBodyV1) -> Result<Self, ResidentNativeToolStartError> {
        let descriptor = pale_contracts::schema_descriptor(NATIVE_TOOL_REQUEST_V1_SCHEMA_ID)
            .map_err(|_| ResidentNativeToolStartError)?;
        if body.schema_id.as_str() != descriptor.schema_id
            || body.schema_sha256.as_str() != descriptor.schema_sha256
            || payload_digest(&body.payload)? != body.payload_sha256
        {
            return Err(ResidentNativeToolStartError);
        }
        pale_contracts::validate_value(&body.payload, NATIVE_TOOL_REQUEST_V1_SCHEMA_ID)
            .map_err(|_| ResidentNativeToolStartError)?;
        let candidate: NativeToolRequestV1 = serde_json::from_value(body.payload.clone())
            .map_err(|_| ResidentNativeToolStartError)?;
        let route = RepositoryHookRoutingV1::new(
            candidate.event,
            candidate.workspace_id,
            candidate.repository_id,
            candidate.provider_binding,
        )
        .map_err(|_| ResidentNativeToolStartError)?;
        let (request, invocation) = admit_event(route, candidate.native_event.as_bytes())?;
        Ok(Self {
            body: body.clone(),
            request,
            invocation,
        })
    }

    /// Exact nested body to include before signing the common request.
    #[must_use]
    pub const fn body(&self) -> &CommonRequestBodyV1 {
        &self.body
    }

    /// Parsed occurrence, still without session or effect authority.
    pub const fn request(&self) -> &ResidentHookIngressRequest {
        &self.request
    }

    /// Native tool-use identity derived from the exact body rather than policy defaults.
    #[must_use]
    pub const fn invocation(&self) -> &NativeInvocationIdV1 {
        &self.invocation
    }

    /// Retain the transport key only after the operation owner has verified
    /// its association with the enclosing signed request's caller.
    pub(in crate::resident) fn into_authenticated(
        self,
        caller_key: [u8; 32],
    ) -> super::AuthenticatedResidentHookRequest {
        super::AuthenticatedResidentHookRequest {
            request: self.request,
            caller_key,
        }
    }
}

fn payload_digest(
    value: &serde_json::Value,
) -> Result<ArtifactDigest, ResidentNativeToolStartError> {
    let canonical = canon::to_canonical_string(value).map_err(|_| ResidentNativeToolStartError)?;
    Ok(ArtifactDigest::hash_bytes(canonical.as_bytes()))
}

fn admit_event(
    routing: RepositoryHookRoutingV1,
    bytes: &[u8],
) -> Result<(ResidentHookIngressRequest, NativeInvocationIdV1), ResidentNativeToolStartError> {
    if bytes.is_empty()
        || bytes.len() > RESIDENT_HOOK_EVENT_BYTE_LIMIT
        || !matches!(
            routing.event(),
            LifecycleEvent::PreToolUse | LifecycleEvent::PermissionRequest
        )
    {
        return Err(ResidentNativeToolStartError);
    }
    canon::parse_strict(bytes).map_err(|_| ResidentNativeToolStartError)?;
    let request = ResidentHookIngressRequest::admit(routing, bytes)
        .map_err(|_| ResidentNativeToolStartError)?;
    let event = request.event();
    HarnessActivityIdentity::try_from_host(event.session_id(), event.turn_id())
        .map_err(|_| ResidentNativeToolStartError)?
        .ok_or(ResidentNativeToolStartError)?;
    let invocation =
        NativeInvocationIdV1::parse(event.tool_use_id().ok_or(ResidentNativeToolStartError)?)
            .map_err(|_| ResidentNativeToolStartError)?;
    if event.tool_name().is_none_or(str::is_empty) {
        return Err(ResidentNativeToolStartError);
    }
    Ok((request, invocation))
}

impl std::fmt::Debug for ResidentNativeToolStart {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ResidentNativeToolStart(<candidate native event>)")
    }
}

/// A native start body failed before any session or operation was selected.
#[derive(Debug, thiserror::Error)]
#[error("native tool start body is invalid or exceeds its bound")]
pub struct ResidentNativeToolStartError;

#[cfg(test)]
#[path = "../../../tests/internal/resident/hook/native_start.rs"]
mod tests;
