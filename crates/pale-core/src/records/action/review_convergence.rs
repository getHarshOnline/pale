//! Review findings and verifier dimensions reduced into one stable verdict.

use serde::{Deserialize, Serialize};

/// Contract and dispatch key for review convergence in
/// [`crate::records::NativeActionCatalogV1`].
pub const REVIEW_CONVERGENCE_REDUCE_ACTION_ID: &str = "pale.compute.review-convergence-reduce";
/// Fixed-point blocking threshold applied to
/// [`ReviewFinding::verifier_confidence_micros`].
///
/// This value is part of the action behavior contract: a finding at or above it
/// receives [`FindingDisposition::Blocking`].
pub const REVIEW_BLOCKING_CONFIDENCE_MICROS: u32 = 800_000;

wire_enum!(
    /// Ordered review severity used during deterministic deduplication.
    ReviewSeverity {
        /// Immediate correctness or safety failure at the highest review severity.
        Critical => "critical",
        /// Material issue that should block acceptance when sufficiently verified.
        High => "high",
        /// Bounded issue requiring attention but below high severity.
        Medium => "medium",
        /// Minor issue or improvement opportunity.
        Low => "low",
    }
);

/// One source finding before deterministic deduplication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewFinding {
    /// Source finding identity, unique within one convergence request.
    #[schemars(length(min = 1, max = 256))]
    pub id: String,
    /// Caller-supplied file identity used as part of deduplication.
    #[schemars(length(min = 1, max = 4_096))]
    pub file: String,
    /// Optional one-based source line; required when evidence is empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1))]
    pub line: Option<u32>,
    /// Concise finding title used for fallback deduplication.
    #[schemars(length(min = 1, max = 512))]
    pub title: String,
    /// Supplied evidence normalized for primary deduplication when non-empty.
    #[serde(default)]
    #[schemars(length(max = 8_192))]
    pub evidence: String,
    /// Reviewer-assigned severity; the action does not infer it.
    pub severity: ReviewSeverity,
    /// Confidence on a one-million fixed-point scale.
    #[schemars(range(max = 1_000_000))]
    pub verifier_confidence_micros: u32,
}

/// One independently verified condition that may gate acceptance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerifierDimension {
    /// Stable verifier identity reported when a required check fails.
    #[schemars(length(min = 1, max = 256))]
    pub id: String,
    /// Whether failure of this verifier independently blocks acceptance.
    pub required: bool,
    /// Caller-supplied outcome of the independent verifier.
    pub passed: bool,
}

/// Findings and verifier outcomes to reduce into one recommendation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewConvergenceInput {
    /// Reviewer-supplied findings to deduplicate and order.
    #[schemars(length(max = 10_000))]
    pub findings: Vec<ReviewFinding>,
    /// Independent verifier outcomes, including at least one required dimension.
    #[schemars(length(min = 1, max = 128))]
    pub verifier_dimensions: Vec<VerifierDimension>,
}

wire_enum!(
    /// Whether a converged finding prevents acceptance.
    FindingDisposition {
        /// Maximum group confidence meets the fixed blocking threshold.
        Blocking => "blocking",
        /// Maximum group confidence remains below the blocking threshold.
        Advisory => "advisory",
    }
);

/// One deduplicated finding with its strictest independently aggregated signals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConvergedFinding {
    /// Identity of the deterministic representative finding.
    #[schemars(length(min = 1))]
    pub canonical_id: String,
    /// Sorted identities of every finding merged into this group.
    #[schemars(inner(length(min = 1)))]
    pub source_ids: Vec<String>,
    /// File identity copied from the representative finding.
    #[schemars(length(min = 1))]
    pub file: String,
    /// Optional one-based line copied from the representative finding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1))]
    pub line: Option<u32>,
    /// Title copied from the representative finding.
    #[schemars(length(min = 1))]
    pub title: String,
    /// Evidence copied from the representative finding.
    #[schemars(length(min = 1))]
    pub evidence: String,
    /// Strictest severity across every finding in the deduplication group.
    pub severity: ReviewSeverity,
    /// Highest verifier confidence across the group, independently aggregated
    /// from severity so input order cannot weaken either signal.
    pub verifier_confidence_micros: u32,
    /// Threshold disposition derived from the group's maximum confidence.
    pub disposition: FindingDisposition,
}

wire_enum!(
    /// Final acceptance decision derived from findings and required verifiers.
    ReviewRecommendation {
        /// Required verifiers pass and no converged finding is blocking.
        Accept => "accept",
        /// A required verifier failed or a converged finding is blocking.
        Block => "block",
    }
);

/// Stable converged findings and their fail-closed recommendation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewConvergenceOutput {
    /// Whether every verifier marked required passed.
    pub required_dimensions_passed: bool,
    /// Sorted identities of failed required verifiers.
    #[schemars(inner(length(min = 1)))]
    pub failed_required_dimensions: Vec<String>,
    /// Deduplicated findings in blocking, severity, and source order.
    pub findings: Vec<ConvergedFinding>,
    /// Fail-closed aggregate recommendation.
    pub recommendation: ReviewRecommendation,
}
