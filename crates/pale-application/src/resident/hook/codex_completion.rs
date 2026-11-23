//! Join captured Codex app-server items to an original pre-tool observation.
//!
//! This codec establishes structural correlation only. Its caller must still
//! authenticate the carrier, qualify the native build/configuration and bind
//! the occurrence to an exact admitted operation before using its outcome.

use pale_core::records::{
    ArtifactDigest, HarnessActivityIdentity, LifecycleEvent, TurnIdentityDigest,
};
use serde::Serialize;
use serde_json::Value;

use super::ResidentHookIngressRequest;
use crate::operation_admission::ToolUseIdentityDigest;

/// Terminal item state reported by the selected Codex app-server protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexCommandTerminalStatus {
    /// The item completed; its optional exit remains a separate fact.
    Completed,
    /// The native item failed; this does not synthesize a process exit.
    Failed,
    /// The native item was declined; this does not prove host enforcement.
    Declined,
}

/// Privacy-preserving observation, never a lease, registration or completion grant.
#[derive(Debug, Serialize)]
pub struct CodexCommandObservation {
    /// Existing native-session identity domain, without the raw identifier.
    pub activity: HarnessActivityIdentity,
    /// Exact native turn identity, in the existing turn domain.
    pub turn: TurnIdentityDigest,
    /// Exact tool-use identity, in the existing tool correlation domain.
    pub tool_use_sha256: ArtifactDigest,
    /// Exact original hook command, distinct from its native shell representation.
    pub hook_command_sha256: ArtifactDigest,
    /// Exact command representation shared by started/completed native items.
    pub native_command_sha256: ArtifactDigest,
    /// Equal reported working directories; this is not filesystem identity.
    pub reported_cwd_sha256: ArtifactDigest,
    /// Exact supplied notification bytes for provenance, not authentication.
    pub started_notification_sha256: ArtifactDigest,
    /// Exact supplied terminal notification bytes.
    pub completed_notification_sha256: ArtifactDigest,
    /// Native status, independently of the optional process exit.
    pub status: CodexCommandTerminalStatus,
    /// Native int32 exit, when supplied; never inferred from output text.
    pub exit_code: Option<i32>,
    /// Native duration, without a claim about clock authority or elapsed bounds.
    pub duration_millis: Option<i64>,
    /// Exact aggregate-output identity, when the native item supplies output.
    pub output_sha256: Option<ArtifactDigest>,
}

/// Captured native input could not establish the claimed structural join.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CodexCommandObservationError {
    /// One notification exceeded the existing native-event byte ceiling.
    #[error("native command notification exceeds its byte bound")]
    Bound,
    /// The pre-event or notification has unsupported or malformed known fields.
    #[error("native command observation has an invalid shape")]
    Shape,
    /// Session, turn, tool, command representation, source or cwd disagrees.
    #[error("native command observations do not identify the same occurrence")]
    Correlation,
}

impl ResidentHookIngressRequest {
    /// Observe one Codex command's start and terminal item against this pre-event.
    ///
    /// The v2 `item/started` and `item/completed` profile was observed on Codex
    /// 0.144.1. Notifications are bounded and decoded strictly, including duplicate
    /// key rejection. Unused native fields remain forward-compatible data.
    ///
    /// This method neither resolves the routing provider as Codex nor authenticates
    /// the supplied notifications. The authenticated native integration must do
    /// that independently. It creates no registration, consumes no invocation and
    /// admits no operation. The hook command and shell-wrapped native command are
    /// retained as separate identities: best-effort `commandActions` cannot prove
    /// their equivalence. Output text is never interpreted as an exit code.
    ///
    /// # Errors
    /// Rejects malformed/oversized input, an incomplete item lifecycle and unlike
    /// occurrence coordinates without exposing native identifiers or payloads.
    pub fn observe_codex_command_completion(
        &self,
        started: &[u8],
        completed: &[u8],
    ) -> Result<CodexCommandObservation, CodexCommandObservationError> {
        use CodexCommandObservationError::{Correlation, Shape};
        let event = self.event();
        if self.routing().event() != LifecycleEvent::PreToolUse || event.tool_name() != Some("Bash")
        {
            return Err(Shape);
        }
        let session = event.session_id().ok_or(Shape)?;
        let turn = event.turn_id().ok_or(Shape)?;
        let tool = event.tool_use_id().ok_or(Shape)?;
        let hook_command = event.command().filter(|s| !s.is_empty()).ok_or(Shape)?;
        let cwd = event.cwd().filter(|s| !s.is_empty()).ok_or(Shape)?;
        let activity = HarnessActivityIdentity::try_from_host(Some(session), Some(turn))
            .map_err(|_| Shape)?
            .ok_or(Shape)?;
        let turn_digest = TurnIdentityDigest::try_from_host_identity(turn).map_err(|_| Shape)?;
        let tool_digest = ToolUseIdentityDigest::try_from_host(tool).map_err(|_| Shape)?;
        let first = notification(started, "item/started")?;
        let last = notification(completed, "item/completed")?;
        let start = &first["params"];
        let end = &last["params"];
        for params in [start, end] {
            if text(params, "threadId")? != session
                || text(params, "turnId")? != turn
                || text(&params["item"], "id")? != tool
                || text(&params["item"], "cwd")? != cwd
            {
                return Err(Correlation);
            }
        }
        let before = &start["item"];
        let after = &end["item"];
        if text(before, "status")? != "inProgress"
            || optional_integer(before, "exitCode")?.is_some()
            || optional_integer(before, "durationMs")?.is_some()
        {
            return Err(Shape);
        }
        let command = text(before, "command")?;
        if command.is_empty()
            || text(after, "command")? != command
            || source(before)? != source(after)?
        {
            return Err(Correlation);
        }
        let status = match text(after, "status")? {
            "completed" => CodexCommandTerminalStatus::Completed,
            "failed" => CodexCommandTerminalStatus::Failed,
            "declined" => CodexCommandTerminalStatus::Declined,
            _ => return Err(Shape),
        };
        let exit_code = optional_integer(after, "exitCode")?
            .map(i32::try_from)
            .transpose()
            .map_err(|_| Shape)?;
        let output = match after.get("aggregatedOutput") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(ArtifactDigest::hash_bytes(s.as_bytes())),
            _ => return Err(Shape),
        };
        Ok(CodexCommandObservation {
            activity,
            turn: turn_digest,
            tool_use_sha256: tool_digest.as_artifact_digest().clone(),
            hook_command_sha256: ArtifactDigest::hash_bytes(hook_command.as_bytes()),
            native_command_sha256: ArtifactDigest::hash_bytes(command.as_bytes()),
            reported_cwd_sha256: ArtifactDigest::hash_bytes(cwd.as_bytes()),
            started_notification_sha256: ArtifactDigest::hash_bytes(started),
            completed_notification_sha256: ArtifactDigest::hash_bytes(completed),
            status,
            exit_code,
            duration_millis: optional_integer(after, "durationMs")?,
            output_sha256: output,
        })
    }
}

fn notification(bytes: &[u8], method: &str) -> Result<Value, CodexCommandObservationError> {
    use CodexCommandObservationError::{Bound, Shape};
    if bytes.len() > super::RESIDENT_HOOK_EVENT_BYTE_LIMIT {
        return Err(Bound);
    }
    let value = pale_core::canon::parse_strict(bytes).map_err(|_| Shape)?;
    if text(&value, "method")? != method || value.get("id").is_some() {
        return Err(Shape);
    }
    let params = value.get("params").filter(|v| v.is_object()).ok_or(Shape)?;
    let item = params.get("item").filter(|v| v.is_object()).ok_or(Shape)?;
    if text(item, "type")? != "commandExecution"
        || !item.get("commandActions").is_some_and(Value::is_array)
    {
        return Err(Shape);
    }
    let time = if method == "item/started" {
        "startedAtMs"
    } else {
        "completedAtMs"
    };
    params.get(time).and_then(Value::as_i64).ok_or(Shape)?;
    Ok(value)
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, CodexCommandObservationError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or(CodexCommandObservationError::Shape)
}
fn optional_integer(value: &Value, key: &str) -> Result<Option<i64>, CodexCommandObservationError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_i64()
            .map(Some)
            .ok_or(CodexCommandObservationError::Shape),
    }
}
fn source(value: &Value) -> Result<Option<&str>, CodexCommandObservationError> {
    match value.get("source") {
        None => Ok(None),
        Some(Value::String(s))
            if matches!(
                s.as_str(),
                "agent" | "userShell" | "unifiedExecStartup" | "unifiedExecInteraction"
            ) =>
        {
            Ok(Some(s))
        }
        _ => Err(CodexCommandObservationError::Shape),
    }
}
