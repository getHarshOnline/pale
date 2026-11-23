//! Application-private wire records shared across workspace crate boundaries.
//!
//! These data-transfer objects are public only so lower-level contract tooling
//! and upper-level application crates can share one serialized shape. They are not a
//! stable consumer facade and do not confer admission, lifecycle authority, or
//! permission to perform an effect. Owning application and adapter modules must
//! still validate domain relationships and execute state transitions.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::num::NonZeroU64;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::canon::{
    json_safe_u64_schema as safe_u64_schema,
    positive_json_safe_u64_schema as positive_safe_u64_schema,
};
use crate::records::{
    ArtifactDigest, CapabilityRequirementIdV1, CompilationInputDigest, DecisionEvidenceDigest,
    ExecutionPathSelectionV1, FailurePosture, LifecycleEvent, LogicalRepositoryIdV1,
    ManifestDigest, ModeAxis, OneShotCompilationDigest, OperationBindingDigest,
    OperationCatalogDigest, OperationContractDigest, PackageResourceRefV1, ProfileDigest,
    SchemaDigest, SchemaId, SchemaManifestDigest, SchemaSetDigest, ToolAuthorizationDigest,
    TurnIdentityDigest,
};

mod decision_evidence;
pub use decision_evidence::{
    DecisionActivityV1, DecisionEmissionOutcomeV1, DecisionEvidenceStageV1, DecisionHostOutcomeV1,
    DecisionNativeCoordinateV1, DecisionOccurrenceV1, DecisionOperationBindingV1, DecisionTargetV1,
};

/// Maximum decision entries retained before the initial-v1 chain rotates its anchor.
pub const DECISION_EVIDENCE_MAX_ENTRIES_V1: usize = 64;

fn decision_evidence_entries_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    let item = generator.subschema_for::<DecisionEvidenceEntryV1>();
    schemars::Schema::try_from(serde_json::json!({
        "type": "array",
        "maxItems": DECISION_EVIDENCE_MAX_ENTRIES_V1,
        "items": item
    }))
    .expect("the static decision-evidence entry schema is valid")
}

/// Selected response intent, independent from emission and host outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DecisionDispositionV1 {
    /// Policy selected denial; this does not establish native prevention.
    Deny,
    /// Policy requested confirmation; this does not establish a native pause.
    RequestConfirmation,
    /// Select steering at a native completion event; no earlier effect is prevented.
    PostEffectSteer,
    /// Select completion steering without rewriting an earlier effect.
    CompletionSteer,
    /// Select advisory output without an applicable control channel.
    Notification,
}

/// One bounded wire entry presented for a decision-evidence digest chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DecisionEvidenceEntryV1 {
    /// Monotonic ledger sequence.
    #[schemars(schema_with = "positive_safe_u64_schema")]
    pub sequence: u64,
    /// Native lifecycle event claimed to have produced the decision.
    pub event: LifecycleEvent,
    /// Selected response intent, independent from observed emission or native effect.
    pub disposition: DecisionDispositionV1,
    /// Original occurrence observations; native identifiers supply no authority.
    pub occurrence: DecisionOccurrenceV1,
    /// Exact observable target, with visibility limitations kept explicit.
    pub target: DecisionTargetV1,
    /// Exact operation/lease binding, authoritative absence, or unresolved lookup.
    pub operation: DecisionOperationBindingV1,
    /// Policy selection, response attempt, or independently witnessed outcome.
    pub stage: DecisionEvidenceStageV1,
    /// Bounded tool identity when the event concerns a tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(max = 256))]
    pub tool: Option<String>,
    /// Bounded human-readable decision reason.
    #[schemars(length(max = 1_024))]
    pub reason: String,
    /// Digest of the unabridged reason.
    pub reason_sha256: ArtifactDigest,
    /// Digest of the previous retained or anchored entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_digest: Option<DecisionEvidenceDigest>,
    /// Digest of this entry, its ledger identity, and its predecessor link.
    pub evidence_digest: DecisionEvidenceDigest,
}

/// Digest-chain predecessor retained after bounded history is evicted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DecisionEvidenceTruncationAnchorV1 {
    /// Sequence of the most recently evicted entry.
    #[schemars(schema_with = "positive_safe_u64_schema")]
    pub sequence: u64,
    /// Digest of the most recently evicted entry.
    pub evidence_digest: DecisionEvidenceDigest,
}

/// Serialized decision-evidence ledger; application code owns chain admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DecisionEvidenceLedgerV1 {
    /// Manifest-owned schema identifier.
    #[schemars(
        with = "String",
        extend("const" = "pale/decision-evidence-ledger.v1")
    )]
    pub schema: SchemaId,
    /// Digest of the manifest-owned schema bytes.
    pub schema_sha256: SchemaDigest,
    /// Exact compilation inputs whose hook decision boundary produced the ledger.
    pub compilation_input_digest: CompilationInputDigest,
    /// Profile whose decision boundary produced the ledger.
    pub profile_digest: ProfileDigest,
    /// Repository identity bound to the ledger.
    pub repository_id: LogicalRepositoryIdV1,
    /// Neutral execution path that governed the decision.
    pub execution_path: ExecutionPathSelectionV1,
    /// Observed native activity or an explicit visibility limitation.
    pub activity: DecisionActivityV1,
    /// Sequence assigned to the next appended entry.
    #[schemars(schema_with = "positive_safe_u64_schema")]
    pub next_sequence: u64,
    /// Predecessor retained after bounded eviction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncation_anchor: Option<DecisionEvidenceTruncationAnchorV1>,
    /// Retained entries in oldest-to-newest order, bounded by
    /// [`DECISION_EVIDENCE_MAX_ENTRIES_V1`].
    #[schemars(schema_with = "decision_evidence_entries_schema")]
    pub entries: VecDeque<DecisionEvidenceEntryV1>,
}

/// Canonical operation-admission denial carried through a host string channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationDenialV1 {
    /// Manifest-owned schema identifier.
    #[schemars(with = "String", extend("const" = "pale/operation-denial.v1"))]
    pub schema: SchemaId,
    /// Digest of the manifest-owned schema bytes.
    pub schema_sha256: SchemaDigest,
    /// Stable machine-readable denial class.
    #[schemars(length(min = 1, max = 128))]
    pub code: String,
    /// Bounded human-readable denial explanation.
    #[schemars(length(min = 1, max = 1_024))]
    pub message: String,
    /// Selected operation, when relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 256))]
    pub operation: Option<String>,
    /// Capability requirement, when relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 256))]
    pub requirement: Option<String>,
    /// Native tool identity, when relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 256))]
    pub tool: Option<String>,
    /// Bounded structured detail, when relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 256))]
    pub detail: Option<String>,
    /// Exact authorization scope, when relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_digest: Option<ToolAuthorizationDigest>,
}

/// Admission state of one correlated tool invocation on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InflightAdmissionStatusV1 {
    /// Durable decision evidence has not yet confirmed admission.
    EvidencePending,
    /// The invocation may be correlated with one exact post-tool event.
    Admitted,
}

/// Wire representation of one persisted in-flight invocation admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InflightToolAdmissionV1 {
    /// Digest of the complete route- and lease-bound invocation.
    pub invocation_digest: ArtifactDigest,
    /// Capability requirement matched by the invocation, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(max = 512))]
    pub requirement_id: Option<CapabilityRequirementIdV1>,
    /// Current evidence-confirmation state.
    pub status: InflightAdmissionStatusV1,
}

/// Failure retained on an active operation lease.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationFailureV1 {
    /// Requirement whose execution failed.
    #[schemars(length(min = 1, max = 512))]
    pub requirement_id: CapabilityRequirementIdV1,
    /// Contract-selected failure posture.
    pub posture: FailurePosture,
}

/// Serialized operation lease; application code owns activation and transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActiveOperationLeaseV1 {
    /// Monotonic activation epoch.
    #[schemars(schema_with = "positive_safe_u64_schema")]
    pub activation_epoch: NonZeroU64,
    /// Selected operation identity.
    #[schemars(length(min = 1, max = 512))]
    pub operation_id: PackageResourceRefV1,
    /// Digest of the exact operation contract.
    pub contract_digest: OperationContractDigest,
    /// Digest of the catalog containing the operation.
    pub catalog_digest: OperationCatalogDigest,
    /// Digest of the private route binding.
    pub binding_digest: OperationBindingDigest,
    /// Optional native turn confinement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_digest: Option<TurnIdentityDigest>,
    /// Requirement IDs mapped to exact authorized request digests.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[schemars(length(max = 128))]
    pub authorized_actions: BTreeMap<CapabilityRequirementIdV1, ToolAuthorizationDigest>,
    /// Privacy-preserving tool-use identities mapped to in-flight admissions.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[schemars(length(max = 128))]
    pub inflight_tools: BTreeMap<ArtifactDigest, InflightToolAdmissionV1>,
    /// Sticky operation failure, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<OperationFailureV1>,
}

/// Strength required to discharge one persisted verification debt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VerificationRequirementV1 {
    /// Ordinary structured writes require the standard verifier.
    Standard,
    /// Protected configuration requires the exact repository verifier.
    ProtectedConfiguration,
}

/// One outstanding verification epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerificationDebtV1 {
    /// Mutation epoch awaiting verification.
    #[schemars(schema_with = "positive_safe_u64_schema")]
    pub epoch: NonZeroU64,
    /// Whether completion guidance was already emitted for this epoch.
    #[serde(default)]
    pub completion_prompted: bool,
    /// Verification strength required to discharge the debt.
    pub requirement: VerificationRequirementV1,
}

/// One outstanding rebaseline epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RebaselineDebtV1 {
    /// Context-compaction epoch awaiting refresh.
    #[schemars(schema_with = "positive_safe_u64_schema")]
    pub epoch: NonZeroU64,
}

/// Serialized session continuity; application code owns semantic transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionRuntimeStateV1 {
    /// Manifest-owned schema identifier.
    #[schemars(
        with = "String",
        extend("const" = "pale.session-runtime-state.v1")
    )]
    pub schema: SchemaId,
    /// Digest of the manifest-owned schema bytes.
    pub schema_sha256: SchemaDigest,
    /// Independently selected mode axes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[schemars(length(max = 8))]
    pub active_modes: BTreeMap<ModeAxis, PackageResourceRefV1>,
    /// Lexical paths already observed by first-touch logic.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    #[schemars(length(max = 1_024), inner(length(max = 512)))]
    pub touched: BTreeSet<String>,
    /// Recency-ordered capture identities.
    #[serde(default, skip_serializing_if = "VecDeque::is_empty")]
    #[schemars(length(max = 128), inner(length(max = 512)))]
    pub captures: VecDeque<String>,
    /// Active turn-bounded operation lease.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_operation: Option<ActiveOperationLeaseV1>,
    /// Latest allocated operation epoch.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[schemars(schema_with = "safe_u64_schema")]
    pub operation_epoch: u64,
    /// Latest allocated verification epoch.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[schemars(schema_with = "safe_u64_schema")]
    pub verification_epoch: u64,
    /// Outstanding verification debt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_debt: Option<VerificationDebtV1>,
    /// Latest allocated rebaseline epoch.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[schemars(schema_with = "safe_u64_schema")]
    pub rebaseline_epoch: u64,
    /// Outstanding rebaseline debt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rebaseline_debt: Option<RebaselineDebtV1>,
}

fn is_zero(value: &u64) -> bool {
    *value == 0
}

/// One deferred-artifact claim and its intended inert publication path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeferredArtifactV1 {
    /// Rendered surface kind.
    #[schemars(length(min = 1, max = 128))]
    pub kind: String,
    /// Intended logical or filesystem target.
    #[schemars(length(min = 1, max = 4_096))]
    pub target: String,
    /// Digest of the exact rendered bytes.
    pub content_sha256: ArtifactDigest,
    /// Declared reversible path of the inert artifact; the adapter confines it.
    #[schemars(length(min = 1, max = 4_096))]
    pub inert_path: String,
    /// Human-facing activation note; never executable authority.
    #[schemars(length(max = 4_096))]
    pub activation: String,
}

/// Manifest for surfaces exported inertly rather than installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeferredSurfaceManifestV1 {
    /// Manifest-owned schema identifier.
    #[schemars(
        with = "String",
        extend("const" = "pale.deferred-surface-manifest.v1")
    )]
    pub schema: SchemaId,
    /// Digest of the manifest-owned schema bytes.
    pub schema_sha256: SchemaDigest,
    /// Initial-v1 wire major.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Digest of the schema manifest used for compilation.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Digest of the complete schema set used for compilation.
    pub schema_set_sha256: SchemaSetDigest,
    /// Digest of the complete compilation input.
    pub compilation_input_digest: CompilationInputDigest,
    /// Digest of this one-shot compilation.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Repository whose surfaces were exported.
    pub repository_id: LogicalRepositoryIdV1,
    /// Digest of the sealed repository manifest.
    pub repository_manifest_digest: ManifestDigest,
    /// Deferred artifacts in deterministic order.
    #[schemars(length(min = 1), extend("uniqueItems" = true))]
    pub artifacts: Vec<DeferredArtifactV1>,
}

/// Publication semantics selected by an expiring-record store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ExpiringRecordClassV1 {
    /// Recoverable per-session continuity state.
    SessionContinuity,
    /// Identity-bound evidence that requires durable publication.
    DurableEvidence,
}

/// Generic expiring publication envelope; the adapter owns freshness semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpiringRecordEnvelopeV1<T> {
    /// Manifest-owned envelope schema identifier.
    #[schemars(
        with = "String",
        extend("const" = "pale.expiring-record-envelope.v1")
    )]
    pub schema: SchemaId,
    /// Digest of the manifest-owned envelope schema bytes.
    pub schema_sha256: SchemaDigest,
    /// Initial-v1 wire major.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Digest of the schema manifest used for compilation.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Digest of the complete schema set used for compilation.
    pub schema_set_sha256: SchemaSetDigest,
    /// Digest of the complete compilation input.
    pub compilation_input_digest: CompilationInputDigest,
    /// Payload schema selected for `state`.
    pub payload_schema: SchemaId,
    /// Digest of the selected payload schema bytes.
    pub payload_schema_sha256: SchemaDigest,
    /// Publication class selected by the owning store.
    pub record_class: ExpiringRecordClassV1,
    /// Confined root and relative-path binding selected by the adapter.
    pub record_binding: ArtifactDigest,
    /// Monotonic publication generation.
    #[schemars(schema_with = "positive_safe_u64_schema")]
    pub publication_generation: u64,
    /// Unix wall-clock seconds used only for bounded retention.
    #[schemars(schema_with = "safe_u64_schema")]
    pub updated_unix_seconds: u64,
    /// Nanosecond fraction of the publication timestamp.
    #[serde(default)]
    #[schemars(range(max = 999_999_999))]
    pub updated_subsec_nanos: u32,
    /// Caller-owned payload whose admission remains with its domain.
    pub state: T,
}
