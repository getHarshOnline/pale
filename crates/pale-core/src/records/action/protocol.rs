//! Digest-bearing action contracts, requests, results, errors, and receipts.
//!
//! These are portable wire shapes. Deserialization validates their shape, not
//! their claimed self-digests or execution history. The owning action registry
//! must recompute and relate those claims before treating a decoded chain as
//! admitted evidence.

use serde::{Deserialize, Serialize};

use super::{ActionInput, ActionOutput};
use crate::records::digests::{
    ActionContractDigest, ActionReceiptDigest, ActionRequestDigest, ActionResultDigest,
    InstanceDigest, ProfileDigest,
};
use crate::records::{LogicalRepositoryIdV1, SchemaId};

/// Server component inserted by [`native_action_host_tool_name`].
///
/// This is a protocol token, not proof that a native action server is running
/// or admitted for a repository.
pub const NATIVE_ACTION_SERVER_NAME: &str = "pale";
/// Stable wire identity carried by [`NativeActionCatalogV1::schema`].
pub const NATIVE_ACTION_CATALOG_V1_SCHEMA: &str = "pale/native-action-catalog.v1";
/// Wire major of [`NativeActionCatalogV1`].
pub const NATIVE_ACTION_CATALOG_V1_SCHEMA_VERSION: u32 = 1;
/// Project a syntactically valid native-action ID into its provider-neutral MCP name.
///
/// This function validates the ID grammar, not membership in a selected action
/// catalog. Catalog admission remains the caller's responsibility.
///
/// # Examples
///
/// ```
/// use pale_core::records::native_action_tool_name;
///
/// assert_eq!(
///     native_action_tool_name("pale.compute.path-inventory").as_deref(),
///     Some("compute_path_inventory")
/// );
/// assert!(native_action_tool_name("other.compute.path-inventory").is_none());
/// ```
pub fn native_action_tool_name(action_id: &str) -> Option<String> {
    let suffix = action_id.strip_prefix("pale.compute.")?;
    if suffix.is_empty()
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return None;
    }
    Some(format!("compute_{}", suffix.replace('-', "_")))
}

/// Project a syntactically valid action ID into the native-harness MCP identity.
///
/// The result names the shared Pale server; it does not prove that the action
/// is selected or available for a particular repository.
///
/// # Examples
///
/// ```
/// use pale_core::records::native_action_host_tool_name;
///
/// assert_eq!(
///     native_action_host_tool_name("pale.compute.path-inventory").as_deref(),
///     Some("mcp__pale__compute_path_inventory")
/// );
/// ```
pub fn native_action_host_tool_name(action_id: &str) -> Option<String> {
    native_action_tool_name(action_id)
        .map(|name| format!("mcp__{NATIVE_ACTION_SERVER_NAME}__{name}"))
}

wire_enum!(
    /// Closed effect of every pure native action in this registry. The value is
    /// sealed into contracts and requests; callers do not choose an effect.
    ActionEffect {
        /// Computes only from supplied input and performs no external effect.
        Compute => "compute"
    }
);

wire_enum!(
    /// Runtime boundary of an action. The native registry admits only pure work.
    ActionBoundary {
        /// Uses no ambient filesystem, process, network, clock, randomness, or model access.
        Pure => "pure"
    }
);

wire_enum!(
    /// Closed request-payload kinds.
    ActionInputKind {
        /// Classified trace events and temporal compliance rules.
        ComplianceTraceGrade => "compliance_trace_grade",
        /// Literal invariants over caller-supplied configuration text.
        ConfigInvariantScan => "config_invariant_scan",
        /// Cumulative session-cost snapshots.
        CostAggregate => "cost_aggregate",
        /// Normalized loop lifecycle observations.
        LoopStatusReduce => "loop_status_reduce",
        /// Review findings and required verifier dimensions.
        ReviewConvergenceReduce => "review_convergence_reduce",
        /// Weighted rubric categories and caller-scored checks.
        RubricEvaluation => "rubric_evaluation",
        /// Provider-neutral resident-service definitions.
        ServiceDefinitionNormalize => "service_definition_normalize",
        /// Caller-supplied skill run observations and windows.
        SkillHealthAggregate => "skill_health_aggregate",
        /// Fixed-point criteria, candidates, and scores.
        WeightedRanking => "weighted_ranking",
        /// Caller-supplied paths under an explicit lexical grammar.
        PathInventory => "path_inventory",
        /// Ordered evidence rules and observed signal identities.
        EvidenceSelection => "evidence_selection",
    }
);

wire_enum!(
    /// Closed result-payload kinds.
    ActionOutputKind {
        /// Per-rule compliance results and promotion eligibility.
        ComplianceTraceGrade => "compliance_trace_grade",
        /// Literal-match evidence and aggregate invariant status.
        ConfigInvariantScan => "config_invariant_scan",
        /// Latest-snapshot cost totals grouped by date and model.
        CostAggregate => "cost_aggregate",
        /// Stale-work, wakeup, failure, and intervention signals.
        LoopStatusReduce => "loop_status_reduce",
        /// Deduplicated findings and a fail-closed review recommendation.
        ReviewConvergenceReduce => "review_convergence_reduce",
        /// Normalized category scores and ranked failures.
        RubricEvaluation => "rubric_evaluation",
        /// Validated service definitions in dependency-compatible order.
        ServiceDefinitionNormalize => "service_definition_normalize",
        /// Windowed skill-health rates and trends.
        SkillHealthAggregate => "skill_health_aggregate",
        /// Candidates ordered by exact weighted score.
        WeightedRanking => "weighted_ranking",
        /// A sorted lexical path inventory with occurrence counts.
        PathInventory => "path_inventory",
        /// Matching rules and observed signals unused by those matches.
        EvidenceSelection => "evidence_selection",
    }
);

wire_enum!(
    /// Stable machine-readable failure categories.
    ActionErrorCode {
        /// The action ID is not a member of the compiled native registry.
        UnknownAction => "unknown_action",
        /// Input is structurally valid JSON but violates the action's bounded contract.
        MalformedInput => "malformed_input",
        /// A request names a different contract version or contract digest.
        ContractMismatch => "contract_mismatch",
        /// A request payload kind differs from the selected contract's input kind.
        InputMismatch => "input_mismatch",
        /// A handler produced a payload kind outside its selected contract.
        OutputMismatch => "output_mismatch",
        /// A claimed canonical digest does not match the record content.
        DigestMismatch => "digest_mismatch",
        /// An identity that must be unique occurs more than once.
        DuplicateIdentifier => "duplicate_identifier",
        /// Weighted-ranking input violates its complete fixed-point matrix contract.
        InvalidRanking => "invalid_ranking",
        /// An integer operation exceeded its checked or canonical JSON bound.
        NumericOverflow => "numeric_overflow",
        /// A supplied path violates its declared portable lexical grammar.
        InvalidPath => "invalid_path",
        /// A record could not be encoded into its canonical digest preimage.
        CanonicalizationFailed => "canonicalization_failed",
        /// Compiled registry metadata or a handler broke a Pale application invariant.
        InternalInvariant => "internal_invariant",
    }
);

/// Digest-bearing declaration of one deterministic native action.
///
/// [`Self::contract_digest`] is a claim on decoded values. Registry admission
/// recomputes it and validates the remaining catalog relationships.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionContract {
    /// Stable record identity; currently `pale/action-contract.v1`.
    #[schemars(extend("const" = "pale/action-contract.v1"))]
    pub schema: String,
    /// Numeric version paired with `schema`.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Canonical native-action identity selected by tools and operations.
    #[schemars(regex(pattern = r"^pale\.compute\.[a-z0-9-]+$"))]
    pub id: String,
    /// Version of this action's behavior and input/output contract.
    #[schemars(extend("const" = 1))]
    pub version: u32,
    /// Concise provider-neutral purpose presented during tool discovery.
    #[schemars(length(min = 1))]
    pub description: String,
    /// Declared external-effect class; native actions are compute-only.
    pub effect: ActionEffect,
    /// Declared ambient-access boundary; native actions are pure.
    pub boundary: ActionBoundary,
    /// Exact tagged input variant accepted by this action.
    pub input_kind: ActionInputKind,
    /// Exact tagged output variant emitted by this action.
    pub output_kind: ActionOutputKind,
    /// Claimed canonical digest of the fields excluding this member.
    pub contract_digest: ActionContractDigest,
}

/// Wire catalog for the distribution's compiled deterministic native actions.
///
/// This record defines the reusable action mechanics available to Pale. It is
/// distinct from [`RepositoryNativeActionCatalogV1`], which is the selected,
/// repository-bound projection consumed by a native tool server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeActionCatalogV1 {
    /// Stable catalog identity; currently `pale/native-action-catalog.v1`.
    #[schemars(with = "String", extend("const" = NATIVE_ACTION_CATALOG_V1_SCHEMA))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Numeric version paired with `schema`.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Complete compiled action contracts in stable action-ID order.
    #[schemars(extend("uniqueItems" = true))]
    pub contracts: Vec<ActionContract>,
}

/// Request claiming one contract version, digest, and declared effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionRequest {
    /// Stable record identity; currently `pale/action-request.v1`.
    #[schemars(extend("const" = "pale/action-request.v1"))]
    pub schema: String,
    /// Numeric version paired with `schema`.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Action claimed to have been selected from the native registry.
    #[schemars(regex(pattern = r"^pale\.compute\.[a-z0-9-]+$"))]
    pub action_id: String,
    /// Claimed behavior version of the selected contract.
    #[schemars(extend("const" = 1))]
    pub action_version: u32,
    /// Claimed digest of the contract under which the request was prepared.
    pub contract_digest: ActionContractDigest,
    /// Claimed contract effect; admission verifies that the caller did not choose it.
    pub effect: ActionEffect,
    /// Closed typed payload whose kind must match the selected contract.
    pub input: ActionInput,
    /// Claimed canonical digest of the fields excluding this member.
    pub request_digest: ActionRequestDigest,
}

/// Result claiming successful execution under one contract and request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionResult {
    /// Stable record identity; currently `pale/action-result.v1`.
    #[schemars(extend("const" = "pale/action-result.v1"))]
    pub schema: String,
    /// Numeric version paired with `schema`.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Action claimed to have produced the output.
    #[schemars(regex(pattern = r"^pale\.compute\.[a-z0-9-]+$"))]
    pub action_id: String,
    /// Claimed behavior version of the admitted request.
    #[schemars(extend("const" = 1))]
    pub action_version: u32,
    /// Claimed digest of the governing contract.
    pub contract_digest: ActionContractDigest,
    /// Claimed digest of the executed request.
    pub request_digest: ActionRequestDigest,
    /// Closed typed payload emitted by the compiled handler.
    pub output: ActionOutput,
    /// Claimed canonical digest of the fields excluding this member.
    pub result_digest: ActionResultDigest,
}

/// Structured action failure for a harness boundary.
///
/// `message` is display text, not an evidence or authorization channel; callers
/// constructing an error remain responsible for excluding secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
#[error("{code}: {message}")]
pub struct ActionError {
    /// Stable record identity; currently `pale/action-error.v1`.
    pub schema: String,
    /// Numeric version paired with `schema`.
    pub schema_version: u32,
    /// Selected action, absent only when failure preceded action identification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    /// Stable category for programmatic failure handling.
    pub code: ActionErrorCode,
    /// Human explanation; producers must bound it and remove secrets.
    pub message: String,
    /// Identity of the rejected request when decoding progressed far enough to bind it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_digest: Option<ActionRequestDigest>,
}

/// Digest-chain claim linking one contract, request, and result.
///
/// Immutability and successful execution follow only after the owning boundary
/// verifies every self-digest and cross-record relationship.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionReceipt {
    /// Stable record identity; currently `pale/action-receipt.v1`.
    #[schemars(extend("const" = "pale/action-receipt.v1"))]
    pub schema: String,
    /// Numeric version paired with `schema`.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Action whose claimed chain this receipt closes.
    #[schemars(regex(pattern = r"^pale\.compute\.[a-z0-9-]+$"))]
    pub action_id: String,
    /// Claimed behavior version of the admitted request.
    #[schemars(extend("const" = 1))]
    pub action_version: u32,
    /// Claimed digest of the governing action contract.
    pub contract_digest: ActionContractDigest,
    /// Claimed digest of the executed request.
    pub request_digest: ActionRequestDigest,
    /// Claimed digest of the produced result.
    pub result_digest: ActionResultDigest,
    /// Claimed canonical digest of the fields excluding this member.
    pub receipt_digest: ActionReceiptDigest,
}

/// Portable action output carrying the complete chain presented for verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionExecution {
    /// Stable envelope identity; currently `pale/action-execution.v1`.
    #[schemars(with = "String", extend("const" = "pale/action-execution.v1"))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Numeric version paired with `schema`.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Request presented as admitted by the registry.
    pub request: ActionRequest,
    /// Result presented as produced for that request.
    pub result: ActionResult,
    /// Receipt presented as closing the execution.
    pub receipt: ActionReceipt,
}

/// Repository-bound discovery record claiming a selected native-action set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RepositoryNativeActionCatalogV1 {
    /// Stable catalog identity; currently `pale/repository-native-action-catalog.v1`.
    #[schemars(
        with = "String",
        extend("const" = "pale/repository-native-action-catalog.v1")
    )]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Numeric version paired with `schema`.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Portable effective-profile identity that selected these actions.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved instance identity for this materialized catalog.
    pub instance_digest: InstanceDigest,
    /// Repository whose local native-harness server may expose the contracts.
    pub repository_id: LogicalRepositoryIdV1,
    /// Contract claims in stable action-ID order.
    #[schemars(length(min = 1), extend("uniqueItems" = true))]
    pub contracts: Vec<ActionContract>,
}
