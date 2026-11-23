//! Validated host observations for per-tool admission.

use std::fmt;

use pale_core::records::{
    ArtifactDigest, HarnessActivityIdentity, InstanceDigest, ProviderBindingRefV1,
    TurnIdentityDigest,
};
use serde::{Deserialize, Serialize};

use super::OperationAdmissionError;

#[derive(Serialize)]
struct ToolUseIdentityPreimage<'a> {
    schema: &'static str,
    host_tool_use_id: &'a str,
}

/// Privacy-preserving identity shared by one host's pre- and post-tool events.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ToolUseIdentityDigest(ArtifactDigest);

impl fmt::Debug for ToolUseIdentityDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ToolUseIdentityDigest")
            .field(&self.0)
            .finish()
    }
}

impl ToolUseIdentityDigest {
    pub(crate) fn try_from_host(value: &str) -> Result<Self, OperationAdmissionError> {
        if value.is_empty() || value.len() > 1_024 || value.chars().any(char::is_control) {
            return Err(OperationAdmissionError::InvalidToolUseIdentity);
        }
        let canonical = pale_core::canon::to_canonical_string(&ToolUseIdentityPreimage {
            schema: "pale/tool-use-identity.v1",
            host_tool_use_id: value,
        })?;
        Ok(Self(ArtifactDigest::hash_bytes(canonical.as_bytes())))
    }

    pub(crate) fn as_artifact_digest(&self) -> &ArtifactDigest {
        &self.0
    }

    pub(crate) fn from_artifact_digest(value: ArtifactDigest) -> Self {
        Self(value)
    }
}

/// Digest of the complete route- and lease-bound invocation observation.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ToolInvocationDigest(ArtifactDigest);

impl fmt::Debug for ToolInvocationDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ToolInvocationDigest")
            .field(&self.0)
            .finish()
    }
}

impl ToolInvocationDigest {
    /// Hash the complete canonical invocation observation.
    pub(super) fn hash_canonical<T: Serialize + ?Sized>(
        value: &T,
    ) -> Result<Self, OperationAdmissionError> {
        let canonical = pale_core::canon::to_canonical_string(value)?;
        Ok(Self(ArtifactDigest::hash_bytes(canonical.as_bytes())))
    }

    pub(crate) fn as_artifact_digest(&self) -> &ArtifactDigest {
        &self.0
    }

    pub(crate) fn from_artifact_digest(value: ArtifactDigest) -> Self {
        Self(value)
    }
}

/// Native-host facts observed before turn identity is normalized and bounded.
///
/// Named fields keep the command, turn identity, and machine identity from
/// being transposed at the host adapter boundary.
pub struct ObservedToolInvocation<'a> {
    /// Exact provider binding whose adapter emitted this observation.
    pub provider_binding: &'a ProviderBindingRefV1,
    /// Opaque adapter descriptor previously resolved from this exact binding.
    pub adapter: crate::terminal_adapters::TerminalAdapterDescriptor,
    /// Exact native-harness tool name.
    pub tool_name: &'a str,
    /// Native per-call identity shared by pre- and post-tool events.
    pub tool_use_id: Option<&'a str>,
    /// Structured host input, when observable.
    pub tool_input: Option<&'a serde_json::Value>,
    /// Parsed command text for a command-oriented host tool, when observable.
    pub command: Option<&'a str>,
    /// Machine-resolved profile identity binding paths and executable pins.
    pub instance_digest: &'a InstanceDigest,
    /// Optional raw host session identity.
    pub session_id: Option<&'a str>,
    /// Optional raw host turn identity.
    pub turn_id: Option<&'a str>,
}

/// Host facts admitted for evaluation against an active operation.
pub struct ToolInvocation<'a> {
    pub(super) provider_binding: ProviderBindingRefV1,
    pub(super) adapter: crate::terminal_adapters::TerminalAdapterDescriptor,
    pub(super) tool_name: &'a str,
    pub(super) tool_input: Option<&'a serde_json::Value>,
    pub(super) command: Option<&'a str>,
    pub(super) instance_digest: &'a InstanceDigest,
    pub(super) activity: HarnessActivityIdentity,
    pub(super) turn_digest: Option<TurnIdentityDigest>,
    pub(crate) tool_use_id_digest: ToolUseIdentityDigest,
}

impl fmt::Debug for ToolInvocation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ToolInvocation")
            .field("tool_name", &self.tool_name)
            .field("tool_input_present", &self.tool_input.is_some())
            .field("command_present", &self.command.is_some())
            .field("instance_digest", &self.instance_digest)
            .field("activity", &self.activity)
            .field("turn_bound", &self.turn_digest.is_some())
            .field("tool_use_id_digest", &self.tool_use_id_digest)
            .finish()
    }
}

impl<'a> ToolInvocation<'a> {
    /// Normalize untrusted host facts without retaining raw activity or tool-use identities.
    ///
    /// # Errors
    ///
    /// Rejects a missing or malformed per-tool identity, a missing activity
    /// identity, or an invalid session/turn identity. No raw identity or tool
    /// payload is retained in the returned value.
    pub fn try_from_host(
        observed: ObservedToolInvocation<'a>,
    ) -> Result<Self, OperationAdmissionError> {
        let tool_use_id = observed
            .tool_use_id
            .ok_or(OperationAdmissionError::MissingToolUseIdentity)?;
        let tool_use_id_digest = ToolUseIdentityDigest::try_from_host(tool_use_id)?;
        let activity =
            HarnessActivityIdentity::try_from_host(observed.session_id, observed.turn_id)?
                .ok_or(OperationAdmissionError::MissingActivityIdentity)?;
        let turn_digest = observed
            .turn_id
            .map(TurnIdentityDigest::try_from_host_identity)
            .transpose()?;
        Ok(Self {
            provider_binding: observed.provider_binding.clone(),
            adapter: observed.adapter,
            tool_name: observed.tool_name,
            tool_input: observed.tool_input,
            command: observed.command,
            instance_digest: observed.instance_digest,
            activity,
            turn_digest,
            tool_use_id_digest,
        })
    }
}
