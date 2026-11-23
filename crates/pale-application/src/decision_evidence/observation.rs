//! Explicit claimed observations at the bounded decision-view boundary.

use pale_core::engine_wire::{
    DecisionActivityV1, DecisionEvidenceEntryV1, DecisionEvidenceStageV1,
    DecisionNativeCoordinateV1, DecisionOccurrenceV1, DecisionOperationBindingV1, DecisionTargetV1,
};
use pale_core::records::{
    ArtifactDigest, HarnessActivityIdentity, LifecycleEvent, ResidentReasonV1, TurnIdentityDigest,
};

use super::DecisionEvidenceError;
use crate::operation_admission::ToolUseIdentityDigest;
use crate::resident::ResidentHookIngressRequest;

/// Evidence supplied for one selection or later transition.
///
/// This input authenticates no source and establishes no current lease or
/// custody. The native admission owner must establish those independently
/// before authorizing an effect or retaining a transition as authority.
/// The ledger checks bounded content, exact causal relationships and digests.
pub struct DecisionEvidenceObservation<'a> {
    /// Original occurrence, without substituting Pale identity for native IDs.
    pub occurrence: &'a DecisionOccurrenceV1,
    /// Actual visible target or explicit limitation.
    pub target: &'a DecisionTargetV1,
    /// Registration observation from its separate authority owner.
    pub operation: &'a DecisionOperationBindingV1,
    /// Selection, response attempt, or asserted native acknowledgement.
    pub stage: &'a DecisionEvidenceStageV1,
    /// Exact preceding stage, including when recovered outside the current ring.
    /// Its source and durable membership still require separate admission.
    pub predecessor: Option<&'a DecisionEvidenceEntryV1>,
}

impl ResidentHookIngressRequest {
    /// Observe activity without manufacturing an identity for missing fields.
    ///
    /// # Errors
    /// Rejects malformed exposed identities without exposing their spelling.
    pub fn decision_activity(&self) -> Result<DecisionActivityV1, DecisionEvidenceError> {
        Ok(
            match HarnessActivityIdentity::try_from_host(
                self.event().session_id(),
                self.event().turn_id(),
            )
            .map_err(|_| DecisionEvidenceError::Malformed)?
            {
                Some(activity) => DecisionActivityV1::Observed { activity },
                None => DecisionActivityV1::Unobserved {
                    reason: ResidentReasonV1::parse(
                        "the native event exposes neither session nor turn identity",
                    )
                    .map_err(|_| DecisionEvidenceError::Malformed)?,
                },
            },
        )
    }

    /// Preserve original event and existing native-coordinate digest domains.
    ///
    /// This derives observations only. Provider/session admission, operation
    /// registration, host authentication and effect permission remain separate.
    ///
    /// # Errors
    /// Rejects malformed exposed native identities without retaining their text.
    pub fn decision_occurrence(&self) -> Result<DecisionOccurrenceV1, DecisionEvidenceError> {
        let turn = match self.event().turn_id() {
            Some(value) => {
                let digest = TurnIdentityDigest::try_from_host_identity(value)
                    .map_err(|_| DecisionEvidenceError::Malformed)?;
                DecisionNativeCoordinateV1::Observed {
                    sha256: ArtifactDigest::parse(digest.as_str())
                        .map_err(|_| DecisionEvidenceError::Malformed)?,
                }
            }
            None => DecisionNativeCoordinateV1::Unobserved {
                reason: ResidentReasonV1::parse("the native event exposes no turn identity")
                    .map_err(|_| DecisionEvidenceError::Malformed)?,
            },
        };
        let tool_use = match self.event().tool_use_id() {
            Some(value) => DecisionNativeCoordinateV1::Observed {
                sha256: ToolUseIdentityDigest::try_from_host(value)
                    .map_err(|_| DecisionEvidenceError::Malformed)?
                    .as_artifact_digest()
                    .clone(),
            },
            None if matches!(
                self.routing().event(),
                LifecycleEvent::PreToolUse
                    | LifecycleEvent::PostToolUse
                    | LifecycleEvent::PostToolUseFailure
                    | LifecycleEvent::PermissionRequest
            ) =>
            {
                DecisionNativeCoordinateV1::Unobserved {
                    reason: ResidentReasonV1::parse(
                        "the native tool event exposes no tool-use identity",
                    )
                    .map_err(|_| DecisionEvidenceError::Malformed)?,
                }
            }
            None => DecisionNativeCoordinateV1::NotApplicable,
        };
        Ok(DecisionOccurrenceV1 {
            native_event_sha256: self.native_event_sha256().clone(),
            provider_binding: self.routing().identity().provider_binding().clone(),
            turn,
            tool_use,
        })
    }

    /// Observe an exact command or complete structured input, never guessed files.
    ///
    /// File normalization belongs to a separately qualified native path owner.
    /// This read-only method does not use shell parsing to invent that evidence.
    ///
    /// # Errors
    /// Rejects input that cannot be represented as canonical complete JSON.
    pub fn decision_target(&self) -> Result<DecisionTargetV1, DecisionEvidenceError> {
        if let Some(command) = self.event().command() {
            return Ok(DecisionTargetV1::Command {
                sha256: ArtifactDigest::hash_bytes(command.as_bytes()),
            });
        }
        if let Some(input) = self.event().tool_input() {
            let canonical = pale_core::canon::to_canonical_string(input)
                .map_err(|source| DecisionEvidenceError::Digest { source })?;
            return Ok(DecisionTargetV1::StructuredInput {
                sha256: ArtifactDigest::hash_bytes(canonical.as_bytes()),
            });
        }
        Ok(if self.event().tool_name().is_some() {
            DecisionTargetV1::Unobserved {
                reason: ResidentReasonV1::parse(
                    "the native tool event exposes no command or structured input",
                )
                .map_err(|_| DecisionEvidenceError::Malformed)?,
            }
        } else {
            DecisionTargetV1::NotApplicable
        })
    }
}
