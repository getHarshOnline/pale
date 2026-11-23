//! Literal invariant checks over caller-supplied configuration text.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Contract and dispatch key for configuration-invariant scanning in
/// [`crate::records::NativeActionCatalogV1`].
pub const CONFIG_INVARIANT_SCAN_ACTION_ID: &str = "pale.compute.config-invariant-scan";

wire_enum!(
    /// Whether a literal must occur or must be absent in the selected records.
    ConfigInvariantExpectation {
        /// Pass only when the literal occurs at least once in scope.
        RequiredLiteral => "required_literal",
        /// Pass only when the literal never occurs in scope.
        ForbiddenLiteral => "forbidden_literal",
    }
);

/// Opaque caller-supplied text; this action never resolves `id` as a path.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigTextRecord {
    /// Caller-stable identity used by invariant scopes and evidence rows.
    #[schemars(length(min = 1, max = 256))]
    pub id: String,
    /// Opaque UTF-8 text inspected lexically without format interpretation.
    /// Debug formatting redacts it because configuration can contain credentials.
    #[schemars(length(max = 262_144))]
    pub text: String,
}

impl fmt::Debug for ConfigTextRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConfigTextRecord")
            .field("id", &self.id)
            .field("text_bytes", &self.text.len())
            .finish()
    }
}

/// A literal condition over selected records. An empty `record_ids` scope
/// selects every supplied record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigInvariant {
    /// Stable invariant identity reported in the output.
    #[schemars(length(min = 1, max = 256))]
    pub id: String,
    /// Presence or absence condition applied to the literal.
    pub expectation: ConfigInvariantExpectation,
    /// Exact, case-sensitive substring counted line by line.
    #[schemars(length(min = 1, max = 512))]
    pub literal: String,
    /// Selected record identities; an empty list selects every record.
    #[serde(default)]
    #[schemars(
        length(max = 128),
        inner(length(min = 1, max = 256)),
        extend("uniqueItems" = true)
    )]
    pub record_ids: Vec<String>,
}

/// Caller-supplied text records and literal invariants evaluated as one set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigInvariantScanInput {
    /// Supplied text records; the action performs no filesystem reads.
    #[schemars(length(min = 1, max = 128))]
    pub records: Vec<ConfigTextRecord>,
    /// Literal conditions evaluated over those records.
    #[schemars(length(min = 1, max = 256))]
    pub invariants: Vec<ConfigInvariant>,
}

/// Bounded line-level evidence. `occurrences` counts literal matches on that line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigInvariantEvidence {
    /// Identity of the supplied record containing the match.
    #[schemars(length(min = 1))]
    pub record_id: String,
    /// One-based source line containing at least one literal match.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub line: u64,
    /// Number of non-overlapping literal matches on this line.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub occurrences: u64,
}

/// Result of evaluating one invariant across its selected records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigInvariantResult {
    /// Identity of the evaluated invariant.
    #[schemars(length(min = 1))]
    pub invariant_id: String,
    /// Whether the declared literal condition held; this is not a security verdict.
    pub passed: bool,
    /// Total literal occurrences, including matches omitted from `evidence`.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub match_count: u64,
    /// Whether the bounded evidence list omits matching lines.
    pub evidence_truncated: bool,
    /// Matching lines retained up to the action's evidence bound.
    pub evidence: Vec<ConfigInvariantEvidence>,
}

/// Invariant results sorted by invariant ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigInvariantScanOutput {
    /// Whether every lexical invariant held; semantic interpretation remains with the caller.
    pub passed: bool,
    /// Per-invariant results sorted by invariant identity.
    pub results: Vec<ConfigInvariantResult>,
}
