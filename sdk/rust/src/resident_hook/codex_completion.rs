//! Owned, read-only native command evidence; no invocation or carrier authority.

use pale_application::resident as application;

use super::ResidentHookAdmission;
use crate::{error::FailureSpecification, ApiError, PaleApi};
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
};

/// Native item status remains independent of process exit and operation acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexCommandTerminalStatus {
    /// Native command item completed.
    Completed,
    /// Native command item failed.
    Failed,
    /// Native command item was declined.
    Declined,
}

/// SDK-owned view of correlated native evidence, with raw data replaced by identities.
///
/// Serialization intentionally exposes the complete observation. Its native
/// correlation fields do not authenticate a provider, establish command equivalence,
/// register an operation or consume a tool occurrence. The application owns the
/// codec and exact observation meaning; the private wrapper leaks no engine type.
#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
pub struct CodexCommandObservation {
    native: application::CodexCommandObservation,
}
impl CodexCommandObservation {
    /// Exact native terminal state, without deriving process success from it.
    #[must_use]
    pub const fn status(&self) -> CodexCommandTerminalStatus {
        match self.native.status {
            application::CodexCommandTerminalStatus::Completed => {
                CodexCommandTerminalStatus::Completed
            }
            application::CodexCommandTerminalStatus::Failed => CodexCommandTerminalStatus::Failed,
            application::CodexCommandTerminalStatus::Declined => {
                CodexCommandTerminalStatus::Declined
            }
        }
    }
    /// Typed process exit when present; absence is not zero or success.
    #[must_use]
    pub const fn exit_code(&self) -> Option<i32> {
        self.native.exit_code
    }
    /// Native duration; this supplies no independent time-source assurance.
    #[must_use]
    pub const fn duration_millis(&self) -> Option<i64> {
        self.native.duration_millis
    }
    /// Original hook command identity; distinct from the shell representation.
    #[must_use]
    pub fn hook_command_sha256(&self) -> &str {
        self.native.hook_command_sha256.as_str()
    }
    /// Shell-represented command shared by the start and terminal observations.
    #[must_use]
    pub fn native_command_sha256(&self) -> &str {
        self.native.native_command_sha256.as_str()
    }
    /// Existing session-domain digest; no raw native session ID is retained.
    #[must_use]
    pub fn native_session_sha256(&self) -> &str {
        self.native.activity.as_digest_str()
    }
    /// Existing turn-domain digest.
    #[must_use]
    pub fn native_turn_sha256(&self) -> &str {
        self.native.turn.as_str()
    }
    /// Existing tool-use-domain digest for later exact registration checks.
    #[must_use]
    pub fn native_tool_use_sha256(&self) -> &str {
        self.native.tool_use_sha256.as_str()
    }
    /// Aggregate output identity when supplied, without retaining its private text.
    #[must_use]
    pub fn output_sha256(&self) -> Option<&str> {
        self.native.output_sha256.as_ref().map(|d| d.as_str())
    }
}
impl PaleApi {
    /// Join captured Codex v2 command items to an admitted original pre-tool event.
    ///
    /// This bounded, effect-free codec neither connects to Codex nor resolves or
    /// authenticates a provider. The selected native build/configuration and
    /// carrier must be qualified separately. It creates no durable mapping and
    /// changes no one-shot state. The original command and native shell command
    /// remain distinct; `commandActions` is not used as equivalence evidence.
    ///
    /// # Errors
    /// Rejects malformed/oversized notifications or mismatched native coordinates
    /// with opaque causes and no raw identifier, command, path or output in errors.
    pub fn observe_codex_command_completion(
        &self,
        original: &ResidentHookAdmission,
        started_notification: &[u8],
        completed_notification: &[u8],
    ) -> Result<CodexCommandObservation, ApiError> {
        original._request.observe_codex_command_completion(started_notification, completed_notification)
            .map(|native| CodexCommandObservation { native })
            .map_err(|error| ApiError::new_with_opaque_source(
                &self.diagnostic_identity,
                error,
                FailureSpecification {
                    code: "codex_command_observation_rejected",
                    stage: DiagnosticStageV1::HostObservation,
                    disposition: DiagnosticDispositionV1::InvalidInput,
                    summary: "captured native command observations do not establish the requested join",
                    subject: None,
                    schema_coordinate: None,
                    location: None,
                    retry: DiagnosticRetryV1::AfterCorrection,
                    effect: DiagnosticEffectCertaintyV1::NotApplicable,
                    request: None,
                    notes: vec!["No carrier was contacted, operation registered or tool occurrence consumed."],
                    help: Vec::new(),
                    facts: Vec::new(),
                }
            ))
    }
}
