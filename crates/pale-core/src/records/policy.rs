//! Run-policy records controlling lifecycle activation and child propagation.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};

use super::tool_matcher::{tool_call_matchers_schema, MAX_MATCHERS};
use crate::records::{ChildPropagation, ToolCallMatcher};

/// A private run-policy identity violates the initial-v1 grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("run policy identity must be lowercase Package-style text of at most 255 bytes")]
pub struct InvalidRunPolicyIdV1;

/// Stable private identity of one run policy.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct RunPolicyIdV1(
    #[schemars(
        length(min = 1, max = 255),
        regex(pattern = r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$")
    )]
    String,
);

impl RunPolicyIdV1 {
    /// Validate and own one run-policy identity.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRunPolicyIdV1`] for an invalid bounded token.
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidRunPolicyIdV1> {
        let value = value.into();
        crate::records::package::package_identifier_is_valid(&value)
            .then_some(Self(value))
            .ok_or(InvalidRunPolicyIdV1)
    }

    /// Borrow the validated wire representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for RunPolicyIdV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for RunPolicyIdV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One lifecycle-enforcement intensity level.
///
/// The entry selects session injection, child propagation, and optional action
/// gating. Host matchers remain data so lifecycle mechanics do not hardcode a
/// tool allowlist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunPolicyEntry {
    /// Stable intensity-level identity.
    pub id: RunPolicyIdV1,
    /// Compact operator-facing explanation of the level.
    pub description: String,
    /// Whether compact selected context enters the initial session.
    #[serde(default)]
    pub session_inject: bool,
    /// Whether selected context enters child sessions.
    #[serde(default = "default_child_propagation")]
    pub child_propagation: ChildPropagation,
    /// Optional case-insensitive globs over the host's child/agent type. Empty selects every
    /// child. A missing child identity propagates rather than silently dropping the policy.
    #[serde(default)]
    pub child_type_selectors: Vec<String>,
    /// Whether matching consequential tool calls require lifecycle admission.
    #[serde(default)]
    pub action_gate: bool,
    /// Exact tool-call classes governed by `action_gate`.
    #[serde(default)]
    #[schemars(schema_with = "tool_call_matchers_schema")]
    pub gated_tool_matchers: Vec<ToolCallMatcher>,
}

impl RunPolicyEntry {
    /// Validate relationships that span more than one serialized field.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRunPolicy`] when gate and matcher state disagree, child
    /// selectors cannot propagate, or either selector inventory contains a
    /// duplicate.
    pub fn validate_contract(&self) -> Result<(), InvalidRunPolicy> {
        if self.gated_tool_matchers.len() > MAX_MATCHERS {
            return Err(InvalidRunPolicy::TooManyToolMatchers {
                count: self.gated_tool_matchers.len(),
                maximum: MAX_MATCHERS,
            });
        }
        let has_gated_tools = !self.gated_tool_matchers.is_empty();
        if self.action_gate != has_gated_tools {
            return Err(InvalidRunPolicy::GateMatcherMismatch);
        }
        if !self.child_type_selectors.is_empty()
            && self.child_propagation != ChildPropagation::Propagate
        {
            return Err(InvalidRunPolicy::SelectorsWithoutPropagation);
        }
        require_unique(
            self.child_type_selectors.iter().map(String::as_str),
            InvalidRunPolicy::DuplicateChildSelector,
        )?;
        require_unique(
            self.gated_tool_matchers.iter().map(ToolCallMatcher::as_str),
            InvalidRunPolicy::DuplicateToolMatcher,
        )
    }
}

fn require_unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    duplicate: impl Fn(String) -> InvalidRunPolicy,
) -> Result<(), InvalidRunPolicy> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(duplicate(value.to_string()));
        }
    }
    Ok(())
}

/// A run-policy record contains contradictory or ambiguous declarations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidRunPolicy {
    /// A tool-matcher inventory exceeded its fixed per-policy work bound.
    #[error("gated tool matcher inventory has {count} entries; maximum is {maximum}")]
    TooManyToolMatchers {
        /// Number of entries supplied by the record.
        count: usize,
        /// Maximum entries admitted for one policy.
        maximum: usize,
    },
    /// A gate without matchers is inert; matchers without a gate are ignored.
    #[error("action_gate must be enabled exactly when gated_tool_matchers is non-empty")]
    GateMatcherMismatch,
    /// Child selectors have no meaning when project context is withheld.
    #[error("child selectors require child_propagation = propagate")]
    SelectorsWithoutPropagation,
    /// Repeated selectors add no meaning and obscure the sealed policy.
    #[error("duplicate child selector `{0}`")]
    DuplicateChildSelector(String),
    /// Repeated tool matchers add no meaning and obscure the sealed policy.
    #[error("duplicate tool matcher `{0}`")]
    DuplicateToolMatcher(String),
}

/// Children inherit the active policy unless a record explicitly exempts them.
fn default_child_propagation() -> ChildPropagation {
    ChildPropagation::Propagate
}
