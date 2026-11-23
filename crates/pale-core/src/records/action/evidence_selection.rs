//! Caller-supplied evidence rules for deterministic selection.
//!
//! The action interprets no file, stack, or package-manager semantics. Private
//! configuration supplies the rule order; a harness supplies observed evidence;
//! Rust performs exact set matching and reports why each rule matched.

use serde::{Deserialize, Serialize};

/// Contract and dispatch key for evidence selection in
/// [`crate::records::NativeActionCatalogV1`].
pub const EVIDENCE_SELECTION_ACTION_ID: &str = "pale.compute.evidence-selection";

wire_enum!(
    /// Whether selection returns every match or only the first by declared order.
    EvidenceSelectionMode {
        /// Evaluate every rule and retain every match in declaration order.
        AllMatches => "all_matches",
        /// Stop after the first matching rule in declaration order.
        FirstMatch => "first_match",
    }
);

/// One declarative evidence rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRule {
    /// Stable rule identity returned when the rule matches.
    #[schemars(length(min = 1, max = 256))]
    pub id: String,
    /// Signals that must all be observed.
    #[serde(default)]
    #[schemars(length(max = 1_024), inner(length(min = 1, max = 256)))]
    pub require_all: Vec<String>,
    /// Signals of which at least one must be observed when this list is non-empty.
    #[serde(default)]
    #[schemars(length(max = 1_024), inner(length(min = 1, max = 256)))]
    pub require_any: Vec<String>,
    /// Signals that prevent the rule from matching when observed.
    #[serde(default)]
    #[schemars(length(max = 1_024), inner(length(min = 1, max = 256)))]
    pub exclude: Vec<String>,
}

/// Observed evidence and ordered matching rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSelectionInput {
    /// Whether evaluation stops at the first match or retains all matches.
    pub mode: EvidenceSelectionMode,
    /// Unique caller-observed signal identities.
    #[schemars(length(max = 10_000), inner(length(min = 1, max = 256)))]
    pub observed: Vec<String>,
    /// Matching rules in explicit precedence order.
    #[schemars(length(max = 1_024))]
    pub rules: Vec<EvidenceRule>,
}

/// One matching rule with the observed signals that justified it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceMatch {
    /// Identity of the matching rule.
    #[schemars(length(min = 1))]
    pub id: String,
    /// Sorted observed requirements that positively witnessed the match.
    #[schemars(inner(length(min = 1)))]
    pub evidence: Vec<String>,
}

/// Matching rules in declaration order. `first_match` emits at most one row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSelectionOutput {
    /// Matching rules in declaration order, bounded by the selected mode.
    pub matches: Vec<EvidenceMatch>,
    /// Sorted observed signals not used as positive evidence by retained matches.
    #[schemars(inner(length(min = 1)))]
    pub unmatched_observed: Vec<String>,
}
