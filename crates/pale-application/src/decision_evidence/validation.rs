//! Shared bounds and monotonic-chain checks for new and deserialized ledgers.

use super::DecisionEvidenceObservation;
use super::{DecisionEvidenceError, MAX_REASON_BYTES, MAX_TOOL_BYTES};
use pale_core::engine_wire::{
    DecisionDispositionV1, DecisionEmissionOutcomeV1, DecisionEvidenceEntryV1,
    DecisionEvidenceStageV1, DecisionHostOutcomeV1, DecisionNativeCoordinateV1, DecisionTargetV1,
};
use pale_core::records::{
    ArtifactDigest, DecisionEvidenceDigest, LifecycleEvent, LogicalRepositoryIdV1,
};

/// Check target normalization and observable coordinates without granting authority.
pub(super) fn validate_observation(
    repository: &LogicalRepositoryIdV1,
    event: LifecycleEvent,
    disposition: DecisionDispositionV1,
    observation: &DecisionEvidenceObservation<'_>,
) -> Result<(), DecisionEvidenceError> {
    let binding = &observation.occurrence.provider_binding.provider_binding_id;
    if binding.is_empty() || binding.len() > 512 || binding.chars().any(char::is_control) {
        return Err(DecisionEvidenceError::Malformed);
    }
    let concerns_tool = matches!(
        event,
        LifecycleEvent::PreToolUse
            | LifecycleEvent::PostToolUse
            | LifecycleEvent::PostToolUseFailure
            | LifecycleEvent::PermissionRequest
    );
    if concerns_tool
        && matches!(
            observation.occurrence.tool_use,
            DecisionNativeCoordinateV1::NotApplicable
        )
    {
        return Err(DecisionEvidenceError::Malformed);
    }
    if let DecisionTargetV1::File {
        repository_id,
        path,
        sha256,
    } = observation.target
    {
        if repository_id != repository
            || path.is_empty()
            || path.len() > 4_096
            || path.contains('\\')
            || path.chars().any(char::is_control)
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || &ArtifactDigest::hash_bytes(path.as_bytes()) != sha256
        {
            return Err(DecisionEvidenceError::Malformed);
        }
    }
    if let DecisionEvidenceStageV1::HostAcknowledged { outcome, .. } = observation.stage {
        let compatible = match disposition {
            DecisionDispositionV1::Deny => *outcome == DecisionHostOutcomeV1::Prevented,
            DecisionDispositionV1::RequestConfirmation => {
                *outcome == DecisionHostOutcomeV1::ConfirmationPending
            }
            DecisionDispositionV1::PostEffectSteer | DecisionDispositionV1::CompletionSteer => {
                *outcome == DecisionHostOutcomeV1::Steered
            }
            DecisionDispositionV1::Notification => *outcome == DecisionHostOutcomeV1::Notified,
        };
        if !compatible {
            return Err(DecisionEvidenceError::StageMismatch);
        }
    }
    Ok(())
}

/// The exact causal reference named by a noninitial stage.
pub(super) fn stage_predecessor(
    stage: &DecisionEvidenceStageV1,
) -> Option<&DecisionEvidenceDigest> {
    match stage {
        DecisionEvidenceStageV1::PolicySelected => None,
        DecisionEvidenceStageV1::ResponseEmission {
            selected_entry_sha256,
            ..
        } => Some(selected_entry_sha256),
        DecisionEvidenceStageV1::HostAcknowledged {
            emission_entry_sha256,
            ..
        } => Some(emission_entry_sha256),
    }
}

/// Check one claimed transition; native witness and durable membership stay external.
pub(super) fn validate_transition<'a>(
    event: LifecycleEvent,
    disposition: DecisionDispositionV1,
    tool: Option<&str>,
    reason_sha256: &ArtifactDigest,
    observation: &DecisionEvidenceObservation<'_>,
    mut retained: impl Iterator<Item = &'a DecisionEvidenceEntryV1>,
) -> Result<(), DecisionEvidenceError> {
    let Some(reference) = stage_predecessor(observation.stage) else {
        return if observation.predecessor.is_none() {
            Ok(())
        } else {
            Err(DecisionEvidenceError::StageMismatch)
        };
    };
    let prior = observation
        .predecessor
        .ok_or(DecisionEvidenceError::StageMismatch)?;
    if &prior.evidence_digest != reference
        || prior.event != event
        || prior.disposition != disposition
        || prior.tool.as_deref() != tool
        || &prior.reason_sha256 != reason_sha256
        || &prior.occurrence != observation.occurrence
        || &prior.target != observation.target
        || &prior.operation != observation.operation
    {
        return Err(DecisionEvidenceError::StageMismatch);
    }
    match observation.stage {
        DecisionEvidenceStageV1::ResponseEmission { .. } => {
            if prior.stage != DecisionEvidenceStageV1::PolicySelected {
                return Err(DecisionEvidenceError::StageMismatch);
            }
        }
        DecisionEvidenceStageV1::HostAcknowledged { .. } => {
            if !matches!(
                prior.stage,
                DecisionEvidenceStageV1::ResponseEmission {
                    outcome: DecisionEmissionOutcomeV1::Emitted
                        | DecisionEmissionOutcomeV1::Unknown,
                    ..
                }
            ) {
                return Err(DecisionEvidenceError::StageMismatch);
            }
        }
        DecisionEvidenceStageV1::PolicySelected => unreachable!("initial stage returned above"),
    }
    if retained.any(|entry| {
        std::mem::discriminant(&entry.stage) == std::mem::discriminant(observation.stage)
            && stage_predecessor(&entry.stage) == Some(reference)
    }) {
        return Err(DecisionEvidenceError::StageReplay);
    }
    Ok(())
}

/// Enforce per-entry bounds and monotonic sequence adjacency.
pub(super) fn validate_entry_bounds(
    sequence: u64,
    tool: Option<&str>,
    reason: &str,
    prior_sequence: Option<u64>,
) -> Result<(), DecisionEvidenceError> {
    if sequence == 0
        || reason.len() > MAX_REASON_BYTES
        || tool.is_some_and(|value| value.len() > MAX_TOOL_BYTES)
        || prior_sequence.is_some_and(|prior| sequence != prior + 1)
    {
        return Err(DecisionEvidenceError::Malformed);
    }
    Ok(())
}

/// Verify the first-entry anchor rule and each subsequent predecessor link.
pub(super) fn validate_chain_anchor<T: PartialEq>(
    index: usize,
    sequence: u64,
    previous: Option<&T>,
    prior: Option<&T>,
) -> Result<(), DecisionEvidenceError> {
    if (index == 0 && ((sequence == 1) == previous.is_some())) || (index > 0 && previous != prior) {
        return Err(DecisionEvidenceError::BrokenChain);
    }
    Ok(())
}

/// Require the stored successor counter to follow the final retained entry.
pub(super) fn validate_next_sequence(
    next_sequence: u64,
    last_sequence: Option<u64>,
) -> Result<(), DecisionEvidenceError> {
    if let Some(sequence) = last_sequence {
        let expected = sequence
            .checked_add(1)
            .ok_or(DecisionEvidenceError::SequenceOverflow)?;
        if next_sequence != expected {
            return Err(DecisionEvidenceError::Malformed);
        }
    }
    Ok(())
}
