//! Decide how a coding agent's lifecycle event is answered, and encode the answer
//! in the form that agent's tool expects.
//!
//! This module builds on `host_command`, `host_tool`, `session_state`,
//! `terminal_adapters` and `operation_admission`; `compile`, `render` and
//! `resident` build on it. Untrusted event bytes become typed facts before
//! declarative run policy and lifecycle records are evaluated. Independent
//! outcomes compose by strength, mandatory write boundaries remain separate, and
//! the final result is encoded once for the selected harness. Session persistence
//! and operation admission surround this effects-free policy core.

mod behavior;
mod event;
mod matcher;
mod mode;
mod native_config;
mod outcome;
mod output;
mod path;
mod prompt;
mod protocol;
mod readonly;
mod requirements;
mod routing;
mod run_policy;
mod shell;
mod verification;

pub(crate) use crate::terminal_adapters::TerminalAdapterDescriptor;
pub use behavior::{
    protected_write_requires_verification, BoundLifecyclePayloads, HookContext,
    LifecyclePayloadError, LifecyclePlan, LifecyclePlanError,
};
pub use event::{parse_stdin_event, DirectOwnerPrompt, StdinEvent, StdinEventFields};
pub use mode::{resolve_mode_command, ModeCommand};
pub use native_config::{
    merge_codex_hook_config, merge_native_json_hook_config, remove_codex_hook_config,
    remove_native_json_hook_config, CodexHookConfigEdit, CodexHookRepresentation,
    NativeHookConfigError,
};
pub use outcome::HookOutcome;
pub use output::{deny_event, failure_event};
pub use path::{target_path_facts, TargetPathFact, TargetPresence};
pub(crate) use protocol::{
    command_timeout_ceiling_secs as harness_command_timeout_ceiling_secs,
    supports_event as harness_supports_event,
};
pub(crate) use protocol::{tool_confirmation, ToolConfirmation};
pub use readonly::{PreToolUseFacts, WriteBoundary};
pub use requirements::{HookRequirementFacts, HookRequirements};
pub use routing::{RepositoryHookIdentityV1, RepositoryHookRoutingError, RepositoryHookRoutingV1};
pub use run_policy::{
    child_context_admission, ChildContextAdmission, RunPolicyFacts, RunPolicyPayloadError,
    RunPolicyPlan,
};
pub(crate) use verification::profile_requires_any_activation_repository_verifier_observation;
pub use verification::{
    classify_repository_verifier, profile_requires_repository_verifier_observation,
    VerificationEvidence,
};

/// Host-facing alias for the core-owned closed lifecycle event set.
///
/// Parsing and stable tokens remain owned by the core record; this module owns
/// only protocol shaping.
pub use pale_core::records::LifecycleEvent as HookEvent;

/// Native event name where supported protocols share the same vocabulary.
pub fn host_event_name(event: HookEvent) -> &'static str {
    match event {
        HookEvent::SessionStart => "SessionStart",
        HookEvent::UserPromptSubmit => "UserPromptSubmit",
        HookEvent::UserPromptExpansion => "UserPromptExpansion",
        HookEvent::PermissionRequest => "PermissionRequest",
        HookEvent::PreToolUse => "PreToolUse",
        HookEvent::PostToolUse => "PostToolUse",
        HookEvent::PostToolUseFailure => "PostToolUseFailure",
        HookEvent::SubagentStart => "SubagentStart",
        HookEvent::SubagentStop => "SubagentStop",
        HookEvent::PreCompact => "PreCompact",
        HookEvent::PostCompact => "PostCompact",
        HookEvent::Stop => "Stop",
        HookEvent::SessionEnd => "SessionEnd",
    }
}

/// Complete host output for one hook invocation.
///
/// [`HookOutcome::into_output`] is the canonical policy encoder. [`Self::stdout`]
/// is already in the selected native protocol shape and must not be wrapped again.
#[must_use = "hook output must be delivered to the native harness"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookOutput {
    /// Exact standard output payload expected by the native harness protocol.
    pub stdout: String,
    /// Diagnostic standard error text that remains outside the model payload.
    pub stderr: String,
    /// Process exit status required by the native harness hook contract.
    pub exit_code: i32,
}

impl HookOutput {
    /// Bind all control-bearing response channels without emitting anything.
    ///
    /// Includes the requested process status and exact stdout/stderr bytes.
    /// It proves neither emission nor that the process exited with that status.
    /// The native emitter and host witness must establish those separately.
    ///
    /// # Errors
    /// Returns an error if the canonical response preimage cannot be encoded.
    pub fn decision_response_sha256(
        &self,
    ) -> Result<pale_core::records::ArtifactDigest, pale_core::canon::CanonError> {
        let preimage = serde_json::json!({
            "domain": "pale/native-hook-response.v1",
            "requested_exit_code": self.exit_code,
            "stdout_sha256": pale_core::records::ArtifactDigest::hash_bytes(self.stdout.as_bytes()),
            "stderr_sha256": pale_core::records::ArtifactDigest::hash_bytes(self.stderr.as_bytes()),
        });
        let canonical = pale_core::canon::to_canonical_string(&preimage)?;
        Ok(pale_core::records::ArtifactDigest::hash_bytes(
            canonical.as_bytes(),
        ))
    }

    /// Construct a successful invocation; empty stdout is an intentional no-op.
    pub(super) fn ok(stdout: String) -> Self {
        HookOutput {
            stdout,
            stderr: String::new(),
            exit_code: 0,
        }
    }
}
