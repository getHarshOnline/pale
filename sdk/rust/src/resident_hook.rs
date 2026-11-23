//! Admit native hook input and deliver through caller-supplied resident connections.

use std::error::Error;

use pale_application::hooks::{RepositoryHookRoutingError, RepositoryHookRoutingV1};
use pale_application::resident::{
    ResidentHookIngressAdmissionError, ResidentHookIngressRequest, ResidentHookTransportUnavailable,
};
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
};

use crate::error::{ApiError, FailureSpecification};
use crate::PaleApi;

mod codex_completion;
mod delivery;
pub use delivery::ResidentHookOutput;
mod native_start;
pub use codex_completion::{CodexCommandObservation, CodexCommandTerminalStatus};
pub use native_start::{
    PreparedResidentNativeTool, ResidentNativeRequestSigner, ResidentNativeRequestSigningFailure,
    ResidentNativeRequestSigningInput, StartedResidentNativeTool,
};

/// Complete owned adapter event awaiting resident-hook admission.
///
/// Every identity remains an untrusted string until
/// [`PaleApi::admit_resident_hook`] invokes the application-owned routing and
/// payload admission algebras. The request contains no workspace path, carrier,
/// resident generation, provider state, or effect authority.
#[derive(Clone)]
pub struct ResidentHookRequest {
    event: String,
    workspace: String,
    repository: String,
    provider_binding: String,
    provider_binding_identity: String,
    payload: Vec<u8>,
}

impl ResidentHookRequest {
    /// Maximum native payload admitted before parsing.
    pub const MAX_BYTES: usize = pale_application::resident::RESIDENT_HOOK_EVENT_BYTE_LIMIT;

    /// Assemble one static repository coordinate and bounded native payload.
    #[must_use]
    pub fn new(
        event: String,
        workspace: String,
        repository: String,
        provider_binding: String,
        provider_binding_identity: String,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            event,
            workspace,
            repository,
            provider_binding,
            provider_binding_identity,
            payload,
        }
    }
}

impl std::fmt::Debug for ResidentHookRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentHookRequest")
            .field("event", &self.event)
            .field("workspace", &self.workspace)
            .field("repository", &self.repository)
            .field("provider_binding", &self.provider_binding)
            .field("provider_binding_identity", &self.provider_binding_identity)
            .field("payload_byte_len", &self.payload.len())
            .finish()
    }
}

/// Opaque proof that static routing and native payload shape agree.
///
/// Admission establishes no active resident generation, session, policy,
/// carrier, or effect. The application request stays private so callers cannot
/// treat parsed native fields as authority.
#[must_use = "resident-hook admission must be handed to an authenticated carrier or rejected"]
pub struct ResidentHookAdmission {
    _request: ResidentHookIngressRequest,
    frame: pale_ports::ResidentHookFrameRequest,
}

impl std::fmt::Debug for ResidentHookAdmission {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ResidentHookAdmission(<opaque>)")
    }
}

impl PaleApi {
    /// Admit one repository-hook request without selecting or invoking a carrier.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the routing grammar is invalid, an identity or
    /// lifecycle event is unknown, the native payload is absent or malformed,
    /// or its event disagrees with the static command coordinate. Every failure
    /// proves that no resident or external effect started.
    pub fn admit_resident_hook(
        &self,
        request: ResidentHookRequest,
    ) -> Result<ResidentHookAdmission, ApiError> {
        let ResidentHookRequest {
            event,
            workspace,
            repository,
            provider_binding,
            provider_binding_identity,
            payload,
        } = request;
        let routing = RepositoryHookRoutingV1::parse(&[
            "hook",
            event.as_str(),
            "--workspace",
            workspace.as_str(),
            "--repository",
            repository.as_str(),
            "--provider-binding",
            provider_binding.as_str(),
            "--provider-binding-identity",
            provider_binding_identity.as_str(),
        ])
        .map_err(|error| routing_failure(&self.diagnostic_identity, error))?;
        let admitted = ResidentHookIngressRequest::admit(routing, &payload)
            .map_err(|error| ingress_failure(&self.diagnostic_identity, error))?;
        let native_event = String::from_utf8(payload).map_err(|_| {
            ingress_failure(
                &self.diagnostic_identity,
                ResidentHookIngressAdmissionError::Malformed("native event is not UTF-8"),
            )
        })?;
        Ok(ResidentHookAdmission {
            _request: admitted,
            frame: pale_ports::ResidentHookFrameRequest::new(
                [
                    "hook".into(),
                    event,
                    "--workspace".into(),
                    workspace,
                    "--repository".into(),
                    repository,
                    "--provider-binding".into(),
                    provider_binding,
                    "--provider-binding-identity".into(),
                    provider_binding_identity,
                ],
                native_event,
            ),
        })
    }

    /// Project the explicit bootstrap state in which no resident carrier exists.
    ///
    /// The admitted request proves only input agreement. This method preserves
    /// that non-effect by returning a blocked diagnostic with
    /// [`DiagnosticEffectCertaintyV1::NotStarted`]. It does not attempt IPC,
    /// fall back to one-shot evaluation, or infer resident availability.
    #[must_use]
    pub fn resident_hook_transport_unavailable(
        &self,
        _admission: &ResidentHookAdmission,
    ) -> ApiError {
        let error = ResidentHookTransportUnavailable;
        let (disposition, retry) =
            crate::error::application_diagnostic_policy(error.failure_disposition());
        ApiError::new_with_opaque_source(
            &self.diagnostic_identity,
            error,
            FailureSpecification {
                code: "resident_hook_transport_unavailable",
                stage: DiagnosticStageV1::ResidentAdmission,
                disposition,
                summary: "no authenticated resident hook carrier is selected for this request",
                subject: None,
                schema_coordinate: None,
                location: None,
                retry,
                effect: DiagnosticEffectCertaintyV1::NotStarted,
                request: None,
                notes: vec![
                    "The native event was admitted, but no resident lookup or policy evaluation started.",
                ],
                help: vec![
                    "Select a host integration with an authenticated resident carrier before retrying.",
                ],
                facts: Vec::new(),
            },
        )
    }
}

fn routing_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: RepositoryHookRoutingError,
) -> ApiError {
    let (code, summary) = match &error {
        RepositoryHookRoutingError::InvalidGrammar => (
            "resident_hook_routing_invalid",
            "repository hook routing does not match the initial-v1 command grammar",
        ),
        RepositoryHookRoutingError::UnknownEvent => (
            "resident_hook_event_unknown",
            "repository hook routing names an unknown lifecycle event",
        ),
        RepositoryHookRoutingError::InvalidWorkspaceId(_) => (
            "resident_hook_workspace_invalid",
            "repository hook routing carries an invalid workspace identity",
        ),
        RepositoryHookRoutingError::InvalidRepositoryId(_) => (
            "resident_hook_repository_invalid",
            "repository hook routing carries an invalid repository identity",
        ),
        RepositoryHookRoutingError::InvalidProviderBindingId => (
            "resident_hook_provider_binding_invalid",
            "repository hook routing carries an empty provider-binding locator",
        ),
        RepositoryHookRoutingError::InvalidProviderBindingIdentity(_) => (
            "resident_hook_provider_binding_identity_invalid",
            "repository hook routing carries an invalid provider-binding identity",
        ),
    };
    let (disposition, retry) =
        crate::error::application_diagnostic_policy(error.failure_disposition());
    hook_failure(identity, error, code, summary, disposition, retry)
}

fn ingress_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: ResidentHookIngressAdmissionError,
) -> ApiError {
    let (code, summary) = match &error {
        ResidentHookIngressAdmissionError::Absent => (
            "resident_hook_payload_absent",
            "the selected adapter supplied no repository hook event",
        ),
        ResidentHookIngressAdmissionError::Malformed(_) => (
            "resident_hook_payload_malformed",
            "the native repository hook event is malformed or exceeds its bound",
        ),
        ResidentHookIngressAdmissionError::EventMismatch { .. } => (
            "resident_hook_event_mismatch",
            "the native event does not match the static repository hook coordinate",
        ),
    };
    let (disposition, retry) =
        crate::error::application_diagnostic_policy(error.failure_disposition());
    hook_failure(identity, error, code, summary, disposition, retry)
}

fn hook_failure<E>(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: E,
    code: &'static str,
    summary: &'static str,
    disposition: DiagnosticDispositionV1,
    retry: DiagnosticRetryV1,
) -> ApiError
where
    E: Error + Send + Sync + 'static,
{
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code,
            stage: DiagnosticStageV1::ResidentAdmission,
            disposition,
            summary,
            subject: None,
            schema_coordinate: None,
            location: None,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotStarted,
            request: None,
            notes: vec!["The request was rejected before resident lookup or carrier activity."],
            help: vec!["Correct the repository hook coordinate or native event before retrying."],
            facts: Vec::new(),
        },
    )
}
