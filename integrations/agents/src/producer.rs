//! Preserve one signed native occurrence through an uncertain durable start.

use std::time::Instant;

use pale_sdk::{
    ApiError, PaleApi, PreparedResidentNativeTool, ResidentAttachment, ResidentHookOutput,
    ResidentHookRequest, ResidentInvokeStartRequest, ResidentNativeRequestSigner,
    ResidentOperationObservation, StartedResidentNativeTool,
};

/// Native input and its exact canonical body, awaiting the host's request claim.
pub struct LocalAgentNativeToolPreparation {
    request: ResidentHookRequest,
    prepared: PreparedResidentNativeTool,
}

impl LocalAgentNativeToolPreparation {
    /// Admit structural input and prepare the exact body before selecting a signer.
    ///
    /// # Errors
    /// Rejects malformed, oversized or unsupported native events and routes.
    pub fn new(api: &PaleApi, request: ResidentHookRequest) -> Result<Self, ApiError> {
        let prepared =
            api.prepare_resident_native_tool(api.admit_resident_hook(request.clone())?)?;
        Ok(Self { request, prepared })
    }

    /// Canonical body to include unchanged in the independently authorized claim.
    #[must_use]
    pub fn request_body_json(&self) -> &[u8] {
        self.prepared.request_body_json()
    }

    /// Sign once, retaining the exact result for an explicit start retry.
    /// The host supplies the claim and request-signing owner independently of
    /// the attachment's connection credential. No socket opens here.
    ///
    /// # Errors
    /// Rejects an invalid claim, substituted body, signer refusal or deadline.
    pub fn sign(
        self,
        api: &PaleApi,
        unsigned_request: &[u8],
        signer: &mut dyn ResidentNativeRequestSigner,
        deadline: Instant,
    ) -> Result<LocalAgentNativeTool, ApiError> {
        let signed = api.sign_resident_native_tool_request(
            &self.prepared,
            unsigned_request,
            signer,
            deadline,
        )?;
        Ok(LocalAgentNativeTool {
            request: self.request,
            signed,
            started: None,
            observation: None,
        })
    }
}

/// One caller-signed occurrence; its operation start grants no native permission.
///
/// An uncertain start preserves the original event and exact signed request.
/// An explicit retry asks the resident to recover that same start under current
/// admission; it never signs again or invents a new request identity. This is
/// process-local retention, not durable recovery after host restart.
pub struct LocalAgentNativeTool {
    request: ResidentHookRequest,
    signed: ResidentInvokeStartRequest,
    started: Option<StartedResidentNativeTool>,
    observation: Option<ResidentOperationObservation>,
}

impl LocalAgentNativeTool {
    /// Attempt the exact durable start once, retaining its verified result.
    /// A previously successful start returns without opening another connection.
    ///
    /// # Errors
    /// Preserves SDK admission, transport, uncertainty and response failures.
    /// No failure triggers an automatic retry or permits native execution.
    pub fn start(
        &mut self,
        api: &PaleApi,
        attachment: Option<&ResidentAttachment>,
    ) -> Result<(), ApiError> {
        if self.observation.is_none() {
            let prepared =
                api.prepare_resident_native_tool(api.admit_resident_hook(self.request.clone())?)?;
            let started =
                api.start_resident_native_tool(attachment, prepared, self.signed.clone())?;
            self.observation = Some(started.observation().clone());
            self.started = Some(started);
        }
        Ok(())
    }

    /// Initial verified start records, available even after hook delivery.
    /// These historical records do not establish the operation's current head.
    #[must_use]
    pub const fn observation(&self) -> Option<&ResidentOperationObservation> {
        self.observation.as_ref()
    }

    /// Deliver the original event at most once after verified durable start.
    ///
    /// A response requires the attachment's independent response admission.
    /// A lost response consumes this local delivery attempt as well; reconcile it rather than
    /// automatically replaying the event. `None` means start has not succeeded
    /// or delivery was already attempted, and likewise grants no permission.
    #[must_use]
    pub fn deliver_hook(
        &mut self,
        api: &PaleApi,
        attachment: Option<&ResidentAttachment>,
    ) -> Option<Result<ResidentHookOutput, ApiError>> {
        self.started
            .take()
            .map(|started| api.deliver_resident_hook(attachment, started.into_hook_admission()))
    }
}

impl std::fmt::Debug for LocalAgentNativeToolPreparation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("LocalAgentNativeToolPreparation(<private event>)")
    }
}

impl std::fmt::Debug for LocalAgentNativeTool {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("LocalAgentNativeTool(<awaiting native decision>)")
    }
}
