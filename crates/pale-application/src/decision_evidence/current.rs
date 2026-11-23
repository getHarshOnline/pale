//! Current v1 ledger state, append rules, truncation, and hash-chain verification.
//!
//! Entries are retained newest-last. When the bounded ring evicts history, its
//! truncation anchor preserves the digest and sequence required to verify the
//! remaining chain without pretending the discarded records are still present.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;

use pale_core::engine_wire::{
    DecisionActivityV1, DecisionDispositionV1, DecisionEvidenceEntryV1, DecisionEvidenceStageV1,
    DecisionEvidenceTruncationAnchorV1, DecisionOccurrenceV1, DecisionOperationBindingV1,
    DecisionTargetV1, DECISION_EVIDENCE_MAX_ENTRIES_V1,
};
use pale_core::records::{
    ArtifactDigest, CompilationInputDigest, DecisionEvidenceDigest, ExecutionPathSelectionV1,
    LifecycleEvent, LogicalRepositoryIdV1, ProfileDigest, SchemaDigest, SchemaId,
};
use serde::Serialize;

use super::validation::{
    validate_chain_anchor, validate_entry_bounds, validate_next_sequence, validate_observation,
    validate_transition,
};
use super::{
    DecisionEvidenceError, DecisionEvidenceObservation, DecisionLedgerIdentity, ENTRY_SCHEMA_V1,
    MAX_REASON_BYTES, MAX_TOOL_BYTES, SCHEMA_V1,
};

/// Complete domain-separated input committed by one retained entry digest.
#[derive(Serialize)]
struct EvidenceDigestInputV1<'a> {
    schema: &'static str,
    compilation_input_digest: &'a CompilationInputDigest,
    profile_digest: &'a ProfileDigest,
    repository_id: &'a LogicalRepositoryIdV1,
    execution_path: &'a ExecutionPathSelectionV1,
    activity: &'a DecisionActivityV1,
    sequence: u64,
    event: LifecycleEvent,
    disposition: DecisionDispositionV1,
    occurrence: &'a DecisionOccurrenceV1,
    target: &'a DecisionTargetV1,
    operation: &'a DecisionOperationBindingV1,
    stage: &'a DecisionEvidenceStageV1,
    tool: Option<&'a str>,
    reason: &'a str,
    reason_sha256: &'a ArtifactDigest,
    previous_digest: Option<&'a DecisionEvidenceDigest>,
}

/// Entry-local members supplied to the ledger-owned digest function.
struct EvidenceEntryPreimage<'a> {
    sequence: u64,
    event: LifecycleEvent,
    disposition: DecisionDispositionV1,
    occurrence: &'a DecisionOccurrenceV1,
    target: &'a DecisionTargetV1,
    operation: &'a DecisionOperationBindingV1,
    stage: &'a DecisionEvidenceStageV1,
    tool: Option<&'a str>,
    reason: &'a str,
    reason_sha256: &'a ArtifactDigest,
    previous_digest: Option<&'a DecisionEvidenceDigest>,
}

impl<'a> From<&'a DecisionEvidenceEntryV1> for EvidenceEntryPreimage<'a> {
    fn from(entry: &'a DecisionEvidenceEntryV1) -> Self {
        Self {
            sequence: entry.sequence,
            event: entry.event,
            disposition: entry.disposition,
            occurrence: &entry.occurrence,
            target: &entry.target,
            operation: &entry.operation,
            stage: &entry.stage,
            tool: entry.tool.as_deref(),
            reason: &entry.reason,
            reason_sha256: &entry.reason_sha256,
            previous_digest: entry.previous_digest.as_ref(),
        }
    }
}

/// Expiring per-activity view of structurally checked decision observations.
///
/// The ledger is bound to one [`DecisionLedgerIdentity`]. [`Self::record`]
/// rejects cross-ledger identity reuse and advances the hash chain only after the
/// complete candidate state satisfies its v1 schema. Every retained digest
/// commits to the complete ledger identity as well as its predecessor. In the
/// rendered chain law, `d` is the entry digest, `I` the ledger identity, `n`
/// the sequence, and `e`, `q`, and `t` are the event, query, and time
/// coordinates. `r` is the complete reason and its overbar is the bounded
/// retained preview. The separate full-reason digest prevents truncation from
/// changing the identity of the decision that was actually emitted.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/decision-evidence-chain.display.mathml"
))]
///
/// Consequently, moving a valid entry chain to another compilation, profile,
/// repository, execution path, or native activity invalidates that chain.
#[derive(Clone, PartialEq, Eq)]
pub struct DecisionEvidenceLedger {
    pub(super) schema: SchemaId,
    pub(super) schema_sha256: SchemaDigest,
    pub(super) compilation_input_digest: CompilationInputDigest,
    pub(super) profile_digest: ProfileDigest,
    pub(super) repository_id: LogicalRepositoryIdV1,
    pub(super) execution_path: ExecutionPathSelectionV1,
    pub(super) activity: DecisionActivityV1,
    pub(super) next_sequence: u64,
    pub(super) truncation_anchor: Option<DecisionEvidenceTruncationAnchorV1>,
    pub(super) entries: VecDeque<DecisionEvidenceEntryV1>,
}

impl fmt::Debug for DecisionEvidenceLedger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dispositions = BTreeMap::new();
        for entry in &self.entries {
            *dispositions
                .entry(disposition_name(entry.disposition))
                .or_insert(0usize) += 1;
        }
        formatter
            .debug_struct("DecisionEvidenceLedger")
            .field("schema", &self.schema)
            .field("schema_sha256", &self.schema_sha256)
            .field("compilation_input_digest", &self.compilation_input_digest)
            .field("profile_digest", &self.profile_digest)
            .field("repository_id", &self.repository_id)
            .field("execution_path", &self.execution_path)
            .field("activity", &self.activity.observed())
            .field("next_sequence", &self.next_sequence)
            .field(
                "truncation_anchor_sequence",
                &self
                    .truncation_anchor
                    .as_ref()
                    .map(|anchor| anchor.sequence),
            )
            .field("entry_count", &self.entries.len())
            .field("dispositions", &dispositions)
            .finish()
    }
}

impl DecisionEvidenceLedger {
    /// Create an empty v1 ledger bound to one compilation, profile, repository,
    /// path, and activity.
    ///
    /// # Errors
    ///
    /// Rejects a new record that cannot satisfy the current decision-evidence
    /// schema and bounds.
    pub fn new(identity: DecisionLedgerIdentity<'_>) -> Result<Self, DecisionEvidenceError> {
        let record_identity = pale_contracts::record_schema_identity("decision-evidence-ledger.v1")
            .map_err(|source| DecisionEvidenceError::Schema { source })?;
        let ledger = Self {
            schema: record_identity.schema,
            schema_sha256: record_identity.schema_sha256,
            compilation_input_digest: identity.compilation_input_digest.clone(),
            profile_digest: identity.profile_digest.clone(),
            repository_id: identity.repository_id.clone(),
            execution_path: identity.execution_path.clone(),
            activity: identity.activity.clone(),
            next_sequence: 1,
            truncation_anchor: None,
            entries: VecDeque::new(),
        };
        ledger.validate()?;
        Ok(ledger)
    }

    /// Append a bounded decision observation with exact stage correlation.
    ///
    /// This checks content and relationships, not native authenticity, current
    /// authority, durable membership or host enforcement. A persisted ring is
    /// not a replacement for the control store and independent anchor.
    ///
    /// # Errors
    ///
    /// Rejects identity reuse across ledgers, impossible event dispositions,
    /// malformed or tampered retained state, exhausted sequence space, and a
    /// new entry that cannot be canonically hashed or schema-validated.
    pub fn record(
        &mut self,
        identity: DecisionLedgerIdentity<'_>,
        event: LifecycleEvent,
        disposition: DecisionDispositionV1,
        tool: Option<&str>,
        reason: &str,
        observation: DecisionEvidenceObservation<'_>,
    ) -> Result<DecisionEvidenceEntryV1, DecisionEvidenceError> {
        self.validate_identity(identity)?;
        self.validate()?;
        if !disposition_is_valid(event, disposition) {
            return Err(DecisionEvidenceError::DispositionMismatch);
        }
        let reason_sha256 = ArtifactDigest::hash_bytes(reason.as_bytes());
        let reason = crate::bounded_text::digest_preview(reason, MAX_REASON_BYTES);
        let tool = tool.map(|value| crate::bounded_text::digest_preview(value, MAX_TOOL_BYTES));
        let sequence = self.next_sequence;
        validate_observation(&self.repository_id, event, disposition, &observation)?;
        if let Some(predecessor) = observation.predecessor {
            let expected = self.entry_digest(EvidenceEntryPreimage::from(predecessor))?;
            if expected != predecessor.evidence_digest || predecessor.sequence >= sequence {
                return Err(DecisionEvidenceError::StageMismatch);
            }
        }
        validate_transition(
            event,
            disposition,
            tool.as_deref(),
            &reason_sha256,
            &observation,
            self.entries.iter(),
        )?;
        let next_sequence = sequence
            .checked_add(1)
            .ok_or(DecisionEvidenceError::SequenceOverflow)?;
        let previous_digest = self
            .entries
            .back()
            .map(|entry| entry.evidence_digest.clone());
        let evidence_digest = self.entry_digest(EvidenceEntryPreimage {
            sequence,
            event,
            disposition,
            occurrence: observation.occurrence,
            target: observation.target,
            operation: observation.operation,
            stage: observation.stage,
            tool: tool.as_deref(),
            reason: &reason,
            reason_sha256: &reason_sha256,
            previous_digest: previous_digest.as_ref(),
        })?;
        let mut next = self.clone();
        next.next_sequence = next_sequence;
        let entry = DecisionEvidenceEntryV1 {
            sequence,
            event,
            disposition,
            occurrence: observation.occurrence.clone(),
            target: observation.target.clone(),
            operation: observation.operation.clone(),
            stage: observation.stage.clone(),
            tool,
            reason,
            reason_sha256,
            previous_digest,
            evidence_digest,
        };
        next.entries.push_back(entry.clone());
        if next.entries.len() > DECISION_EVIDENCE_MAX_ENTRIES_V1 {
            let evicted = next
                .entries
                .pop_front()
                .ok_or(DecisionEvidenceError::BrokenChain)?;
            next.truncation_anchor = Some(DecisionEvidenceTruncationAnchorV1 {
                sequence: evicted.sequence,
                evidence_digest: evicted.evidence_digest,
            });
        }
        next.validate()?;
        *self = next;
        Ok(entry)
    }

    /// Revalidate schema, bounds, identity, sequence, and every retained digest link.
    pub(super) fn validate(&self) -> Result<(), DecisionEvidenceError> {
        pale_contracts::validate_record(self, "decision-evidence-ledger.v1")
            .map_err(|source| DecisionEvidenceError::Schema { source })?;
        if self.schema.as_str() != SCHEMA_V1
            || self.next_sequence == 0
            || self.entries.len() > DECISION_EVIDENCE_MAX_ENTRIES_V1
        {
            return Err(DecisionEvidenceError::Malformed);
        }
        if self.entries.is_empty() && self.next_sequence != 1 {
            return Err(DecisionEvidenceError::Malformed);
        }
        if self.entries.is_empty() {
            if self.truncation_anchor.is_some() {
                return Err(DecisionEvidenceError::BrokenChain);
            }
        } else if let Some(anchor) = &self.truncation_anchor {
            let first = self
                .entries
                .front()
                .ok_or(DecisionEvidenceError::BrokenChain)?;
            if anchor.sequence.checked_add(1) != Some(first.sequence)
                || first.previous_digest.as_ref() != Some(&anchor.evidence_digest)
            {
                return Err(DecisionEvidenceError::BrokenChain);
            }
        }

        let mut prior: Option<&DecisionEvidenceDigest> = None;
        let mut prior_sequence = None;
        for (index, entry) in self.entries.iter().enumerate() {
            validate_entry_bounds(
                entry.sequence,
                entry.tool.as_deref(),
                &entry.reason,
                prior_sequence,
            )?;
            if !disposition_is_valid(entry.event, entry.disposition) {
                return Err(DecisionEvidenceError::DispositionMismatch);
            }
            let referenced = super::validation::stage_predecessor(&entry.stage);
            let predecessor = referenced.and_then(|digest| {
                self.entries
                    .iter()
                    .find(|candidate| &candidate.evidence_digest == digest)
            });
            let observation = DecisionEvidenceObservation {
                occurrence: &entry.occurrence,
                target: &entry.target,
                operation: &entry.operation,
                stage: &entry.stage,
                predecessor,
            };
            validate_observation(
                &self.repository_id,
                entry.event,
                entry.disposition,
                &observation,
            )?;
            if predecessor.is_some_and(|prior| prior.sequence >= entry.sequence) {
                return Err(DecisionEvidenceError::StageMismatch);
            }
            // A truncated view can name an earlier immutable stage outside the
            // ring. That relation remains a durable-history admission duty.
            if referenced.is_none() || predecessor.is_some() || self.truncation_anchor.is_none() {
                validate_transition(
                    entry.event,
                    entry.disposition,
                    entry.tool.as_deref(),
                    &entry.reason_sha256,
                    &observation,
                    self.entries
                        .iter()
                        .filter(|candidate| candidate.sequence < entry.sequence),
                )?;
            }
            validate_chain_anchor(index, entry.sequence, entry.previous_digest.as_ref(), prior)?;
            if index == 0
                && self.truncation_anchor.is_none()
                && (entry.sequence != 1 || entry.previous_digest.is_some())
            {
                return Err(DecisionEvidenceError::BrokenChain);
            }
            let expected = self.entry_digest(EvidenceEntryPreimage {
                sequence: entry.sequence,
                event: entry.event,
                disposition: entry.disposition,
                occurrence: &entry.occurrence,
                target: &entry.target,
                operation: &entry.operation,
                stage: &entry.stage,
                tool: entry.tool.as_deref(),
                reason: &entry.reason,
                reason_sha256: &entry.reason_sha256,
                previous_digest: entry.previous_digest.as_ref(),
            })?;
            if expected != entry.evidence_digest {
                return Err(DecisionEvidenceError::BrokenChain);
            }
            prior = Some(&entry.evidence_digest);
            prior_sequence = Some(entry.sequence);
        }
        validate_next_sequence(
            self.next_sequence,
            self.entries.back().map(|entry| entry.sequence),
        )
    }

    /// Hash one entry together with this ledger's complete immutable identity.
    fn entry_digest(
        &self,
        entry: EvidenceEntryPreimage<'_>,
    ) -> Result<DecisionEvidenceDigest, DecisionEvidenceError> {
        DecisionEvidenceDigest::hash_canonical(&EvidenceDigestInputV1 {
            schema: ENTRY_SCHEMA_V1,
            compilation_input_digest: &self.compilation_input_digest,
            profile_digest: &self.profile_digest,
            repository_id: &self.repository_id,
            execution_path: &self.execution_path,
            activity: &self.activity,
            sequence: entry.sequence,
            event: entry.event,
            disposition: entry.disposition,
            occurrence: entry.occurrence,
            target: entry.target,
            operation: entry.operation,
            stage: entry.stage,
            tool: entry.tool,
            reason: entry.reason,
            reason_sha256: entry.reason_sha256,
            previous_digest: entry.previous_digest,
        })
        .map_err(|source| DecisionEvidenceError::Digest { source })
    }

    fn validate_identity(
        &self,
        identity: DecisionLedgerIdentity<'_>,
    ) -> Result<(), DecisionEvidenceError> {
        if &self.compilation_input_digest == identity.compilation_input_digest
            && &self.profile_digest == identity.profile_digest
            && &self.repository_id == identity.repository_id
            && &self.execution_path == identity.execution_path
            && &self.activity == identity.activity
        {
            Ok(())
        } else {
            Err(DecisionEvidenceError::IdentityMismatch)
        }
    }
}

fn disposition_is_valid(event: LifecycleEvent, disposition: DecisionDispositionV1) -> bool {
    match disposition {
        DecisionDispositionV1::Deny => {
            matches!(
                event,
                LifecycleEvent::UserPromptSubmit
                    | LifecycleEvent::UserPromptExpansion
                    | LifecycleEvent::PermissionRequest
                    | LifecycleEvent::PreToolUse
            )
        }
        DecisionDispositionV1::RequestConfirmation => event == LifecycleEvent::PreToolUse,
        DecisionDispositionV1::PostEffectSteer => {
            matches!(
                event,
                LifecycleEvent::PostToolUse | LifecycleEvent::PostToolUseFailure
            )
        }
        DecisionDispositionV1::CompletionSteer => {
            matches!(
                event,
                LifecycleEvent::SubagentStop | LifecycleEvent::PreCompact | LifecycleEvent::Stop
            )
        }
        DecisionDispositionV1::Notification => true,
    }
}

fn disposition_name(disposition: DecisionDispositionV1) -> &'static str {
    match disposition {
        DecisionDispositionV1::Deny => "deny",
        DecisionDispositionV1::RequestConfirmation => "request_confirmation",
        DecisionDispositionV1::PostEffectSteer => "post_effect_steer",
        DecisionDispositionV1::CompletionSteer => "completion_steer",
        DecisionDispositionV1::Notification => "notification",
    }
}
