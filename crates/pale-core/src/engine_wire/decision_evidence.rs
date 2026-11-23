//! Structural decision observations; none is native or custody admission.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::records::{
    ArtifactDigest, DecisionEvidenceDigest, HarnessActivityIdentity, ImmutableRecordReferenceV1,
    LogicalRepositoryIdV1, OperationHandleDigest, OperationLeaseDigest, ProviderBindingRefV1,
    ResidentReasonV1, ResidentRecordRefV1,
};

/// Native activity visibility; missing identity must not erase a denial.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "visibility", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionActivityV1 {
    /// Existing native session or turn digest, without a new identity domain.
    Observed {
        /// Strongest activity identity exposed by the native source.
        activity: HarnessActivityIdentity,
    },
    /// The source exposed neither identity; this creates no session or authority.
    Unobserved {
        /// Exact bounded visibility limitation.
        reason: ResidentReasonV1,
    },
}

impl DecisionActivityV1 {
    /// Borrow an observed identity without private visibility explanations.
    #[must_use]
    pub const fn observed(&self) -> Option<&HarnessActivityIdentity> {
        match self {
            Self::Observed { activity } => Some(activity),
            Self::Unobserved { .. } => None,
        }
    }
}

/// Visibility of one native coordinate in its existing digest domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "visibility", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionNativeCoordinateV1 {
    /// Digest of the observed identity, without its private native spelling.
    Observed {
        /// Existing turn or tool-use digest; this creates no new identity domain.
        sha256: ArtifactDigest,
    },
    /// The selected native source did not expose this coordinate.
    Unobserved {
        /// Bounded visibility limitation, not a claim of absence.
        reason: ResidentReasonV1,
    },
    /// This lifecycle event has no such coordinate.
    NotApplicable,
}

/// Original native occurrence coordinates, distinct from Pale authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DecisionOccurrenceV1 {
    /// Digest of the complete original native event bytes.
    pub native_event_sha256: ArtifactDigest,
    /// Exact provider locator and body identity; parsing does not admit it.
    pub provider_binding: ProviderBindingRefV1,
    /// Native turn observation in the existing turn digest domain.
    pub turn: DecisionNativeCoordinateV1,
    /// Native invocation observation in the existing tool-use digest domain.
    pub tool_use: DecisionNativeCoordinateV1,
}

/// Target actually observable to the selected native method.
///
/// A command is not a filesystem target. A literal repository-relative path is
/// not filesystem-object identity or evidence that an effect occurred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionTargetV1 {
    /// Native owner normalized a confined literal repository-relative path.
    File {
        /// Repository against which the path was normalized.
        repository_id: LogicalRepositoryIdV1,
        /// Nonempty literal components, without empty, dot or parent components.
        #[schemars(length(min = 1, max = 4_096))]
        path: String,
        /// Digest of the complete normalized UTF-8 path, without truncation.
        sha256: ArtifactDigest,
    },
    /// Exact original command text, without inferred file effects.
    Command {
        /// Digest of the complete original UTF-8 command.
        sha256: ArtifactDigest,
    },
    /// Complete structured native input under canonical JSON normalization.
    StructuredInput {
        /// Digest of the canonical complete input, not a selected excerpt.
        sha256: ArtifactDigest,
    },
    /// Native visibility cannot establish a target.
    Unobserved {
        /// Exact bounded limitation.
        reason: ResidentReasonV1,
    },
    /// The selected lifecycle decision has no target.
    NotApplicable,
}

/// Exact registration observation, never an operation grant or lease renewal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionOperationBindingV1 {
    /// Claims an admitted existing operation and lease association.
    Registered {
        /// Immutable operation-handle reference.
        handle_ref: ResidentRecordRefV1,
        /// Exact operation-handle identity.
        handle_sha256: OperationHandleDigest,
        /// Immutable narrowed operation-lease reference.
        lease_ref: ResidentRecordRefV1,
        /// Exact lease identity, not evidence of current validity.
        lease_sha256: OperationLeaseDigest,
    },
    /// An authoritative exact lookup established no registration.
    Unbound,
    /// An unavailable or ambiguous lookup cannot establish registration or absence.
    Unresolved {
        /// Bounded lookup limitation; no newest-operation fallback is permitted.
        reason: ResidentReasonV1,
    },
}

/// Observed outcome of attempting one native response emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DecisionEmissionOutcomeV1 {
    /// The emitter observed its complete write; host receipt remains separate.
    Emitted,
    /// The emitter established failure rather than complete emission.
    Failed,
    /// The emitter cannot establish whether the complete response crossed.
    Unknown,
}

/// Native outcome requiring independent admission of the exact witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DecisionHostOutcomeV1 {
    /// The native host acknowledged preventing the exact invocation.
    Prevented,
    /// The native host acknowledged holding this invocation for confirmation.
    ConfirmationPending,
    /// The native host acknowledged the selected steering response.
    Steered,
    /// The native host acknowledged the notification.
    Notified,
}

/// Separate selection, emission attempt and acknowledged native outcome.
///
/// These wire fields are claims until their native source, current authority,
/// exact predecessor and durable custody are admitted. JSON decoding does not
/// supply those admissions. Failed or uncertain emission is not called emitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionEvidenceStageV1 {
    /// Policy selected a response; nothing has been emitted or prevented.
    PolicySelected,
    /// One response attempt, with its actual observed transmission certainty.
    ResponseEmission {
        /// Exact policy-selection entry for this occurrence.
        selected_entry_sha256: DecisionEvidenceDigest,
        /// Complete response identity, including requested status and output channels.
        response_sha256: ArtifactDigest,
        /// Complete write, established failure, or uncertainty.
        outcome: DecisionEmissionOutcomeV1,
    },
    /// Claimed independent host handling of one exact response attempt.
    /// Its witness may resolve uncertain emission without rewriting that record.
    HostAcknowledged {
        /// Exact emission-attempt entry, never just a selection entry.
        emission_entry_sha256: DecisionEvidenceDigest,
        /// Independently admitted native witness; a handler exit is insufficient.
        witness: ImmutableRecordReferenceV1,
        /// Native outcome established by that witness.
        outcome: DecisionHostOutcomeV1,
    },
}
