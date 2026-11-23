//! Declarative lifecycle behavior records.
//!
//! A [`LifecycleBehavior`] binds one [`LifecycleEvent`] to deterministic predicate,
//! effect, state, failure, and generic fallback data. Nuanced judgment remains
//! in Markdown doctrine and specialist workflows.

use super::tool_matcher::{matcher_strings_schema, matcher_within_scalar_limit, MAX_MATCHERS};
use crate::paths::OwnedConfinedRelativePath;
use crate::records::{
    CapabilityIdV1, FailurePosture, LifecycleAction, LifecycleEvent, PackageResourceRefV1,
    PredicateKind, StateScope, ToolCallMatcher,
};
use serde::{Deserialize, Deserializer, Serialize};

/// A private lifecycle-behavior identity violates the initial-v1 grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("lifecycle behavior identity must be lowercase Package-style text of at most 255 bytes")]
pub struct InvalidLifecycleBehaviorIdV1;

/// Stable private identity of one lifecycle behavior.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct LifecycleBehaviorIdV1(
    #[schemars(
        length(min = 1, max = 255),
        regex(pattern = r"^[a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_-]*)*$")
    )]
    String,
);

impl LifecycleBehaviorIdV1 {
    /// Validate and own one lifecycle-behavior identity.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidLifecycleBehaviorIdV1`] for an invalid bounded token.
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidLifecycleBehaviorIdV1> {
        let value = value.into();
        crate::records::package::package_identifier_is_valid(&value)
            .then_some(Self(value))
            .ok_or(InvalidLifecycleBehaviorIdV1)
    }

    /// Borrow the validated wire representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for LifecycleBehaviorIdV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for LifecycleBehaviorIdV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Generic carriage used only when the selected adapter lacks the declared event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LifecycleFallbackV1 {
    /// Compact model guidance carried by the fallback event.
    #[schemars(length(min = 1))]
    pub guidance: String,
    /// Adapter-supported event that carries the guidance.
    pub carrier_event: LifecycleEvent,
}

/// One event-bound behavior with deterministic activation and failure policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LifecycleBehavior {
    /// Stable behavior identity within the lifecycle catalog.
    pub id: LifecycleBehaviorIdV1,
    /// Compact purpose shown in generated lifecycle indexes.
    pub description: String,
    /// Relative path to compact model guidance for this event. The compiler confines and
    /// materializes it separately; the source path never survives into a hook context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guidance: Option<OwnedConfinedRelativePath>,
    /// The lifecycle event this behaviour is keyed to.
    pub event: LifecycleEvent,
    /// What gates the action (data, not a nuanced Rust condition).
    #[serde(default = "default_predicate")]
    pub predicate_kind: PredicateKind,
    /// Predicate-specific exact values, command prefixes, tool matchers, or path
    /// globs. Empty for predicates that do not consume declared strings.
    #[serde(default)]
    #[schemars(schema_with = "matcher_strings_schema")]
    pub matchers: Vec<String>,
    /// Path predicates may match case-insensitively when host filesystems make case variants
    /// equivalent. Other predicate kinds ignore this field.
    #[serde(default)]
    pub case_insensitive: bool,
    /// A path predicate may require a pre-existing target. Only `NotFound` proves absence;
    /// unevaluable targets follow `failure_posture`.
    #[serde(default)]
    pub existing_targets_only: bool,
    /// Suppress this path predicate only after an explicitly invoked operation has admitted the
    /// current exact tool request through its sealed contract.
    #[serde(default)]
    pub allow_admitted_operation: bool,
    /// What the behaviour does when the predicate holds.
    pub action: LifecycleAction,
    /// Target mode for `activate`; absent for every other action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activate_mode: Option<PackageResourceRefV1>,
    /// The run-modes this behaviour is active in (empty = every mode).
    #[serde(default)]
    pub modes: Vec<PackageResourceRefV1>,
    /// Abstract tool/capability requirements (resolved to bindings by private config).
    #[serde(default)]
    pub tool_reqs: Vec<CapabilityIdV1>,
    /// How the behaviour behaves when it cannot be evaluated / fails.
    #[serde(default = "default_failure")]
    pub failure_posture: FailurePosture,
    /// Where the behaviour's state lives (`durable` is a governed seam, not built here).
    #[serde(default = "default_state_scope")]
    pub state_scope: StateScope,
    /// Named owner of a durable effect/state contract. Required by schema when
    /// `state_scope = durable`; absent for stateless, message, and session state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governed_by: Option<String>,
    /// Maximum evaluation time in milliseconds, when the behavior has bounded work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
    /// Optional provider-neutral fallback. The selected adapter descriptor is
    /// the sole source of native event support.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<LifecycleFallbackV1>,
}

impl LifecycleBehavior {
    /// Validate relationships that span predicate, action, and matcher fields.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidLifecycleBehavior`] for contradictory predicate options,
    /// invalid or duplicate matchers, mismatched state ownership or fallback
    /// carriage, an activation without exactly one target mode, or a zero timeout.
    pub fn validate_contract(&self) -> Result<(), InvalidLifecycleBehavior> {
        if self.matchers.len() > MAX_MATCHERS {
            return Err(InvalidLifecycleBehavior::TooManyMatchers {
                count: self.matchers.len(),
                maximum: MAX_MATCHERS,
            });
        }
        for (index, matcher) in self.matchers.iter().enumerate() {
            if !matcher_within_scalar_limit(matcher) {
                return Err(InvalidLifecycleBehavior::MatcherTooLong {
                    index,
                    maximum: super::tool_matcher::MAX_MATCHER_SCALARS,
                });
            }
        }
        let consumes_matchers = matches!(
            self.predicate_kind,
            PredicateKind::ToolMatch
                | PredicateKind::PathMatch
                | PredicateKind::CommandPrefix
                | PredicateKind::ExactMessage
        );
        let has_matchers = !self.matchers.is_empty();
        if consumes_matchers != has_matchers {
            return Err(InvalidLifecycleBehavior::MatcherPresence {
                predicate: self.predicate_kind,
            });
        }
        if self.predicate_kind == PredicateKind::ToolMatch {
            for matcher in &self.matchers {
                ToolCallMatcher::parse(matcher).map_err(|source| {
                    InvalidLifecycleBehavior::InvalidToolMatcher {
                        matcher: matcher.clone(),
                        source,
                    }
                })?;
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for matcher in &self.matchers {
            if !seen.insert(matcher.as_str()) {
                return Err(InvalidLifecycleBehavior::DuplicateMatcher(matcher.clone()));
            }
        }
        if self.predicate_kind != PredicateKind::PathMatch
            && (self.case_insensitive
                || self.existing_targets_only
                || self.allow_admitted_operation)
        {
            return Err(InvalidLifecycleBehavior::PathOptionsWithoutPathPredicate);
        }
        if (self.action == LifecycleAction::Activate) != self.activate_mode.is_some() {
            return Err(InvalidLifecycleBehavior::ActivateModeMismatch);
        }
        if self.timeout_ms == Some(0) {
            return Err(InvalidLifecycleBehavior::ZeroTimeout);
        }
        match (self.state_scope, self.governed_by.as_deref()) {
            (StateScope::Durable, Some(owner)) if !owner.is_empty() => {}
            (StateScope::Durable, _) | (_, Some(_)) => {
                return Err(InvalidLifecycleBehavior::DurableOwnerMismatch);
            }
            _ => {}
        }
        if self
            .fallback
            .as_ref()
            .is_some_and(|fallback| fallback.guidance.is_empty())
        {
            return Err(InvalidLifecycleBehavior::EmptyFallbackGuidance);
        }
        Ok(())
    }

    /// Return fallback guidance only on its declared carrier event.
    #[must_use]
    pub fn fallback_for(&self, event: LifecycleEvent) -> Option<&str> {
        self.fallback.as_ref().and_then(|fallback| {
            (fallback.carrier_event == event).then_some(fallback.guidance.as_str())
        })
    }
}

/// A lifecycle record contains declarations that cannot be applied together.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidLifecycleBehavior {
    /// A matcher inventory exceeded its fixed per-record work bound.
    #[error("matcher inventory has {count} entries; maximum is {maximum}")]
    TooManyMatchers {
        /// Number of entries supplied by the record.
        count: usize,
        /// Maximum entries admitted for one behavior.
        maximum: usize,
    },
    /// One heterogeneous matcher exceeded its fixed scalar bound.
    #[error("matcher at index {index} exceeds {maximum} Unicode scalar values")]
    MatcherTooLong {
        /// Zero-based position of the oversized matcher.
        index: usize,
        /// Maximum Unicode scalar values admitted for one matcher.
        maximum: usize,
    },
    /// Matchers must be present exactly for predicates that consume them.
    #[error("predicate `{predicate}` has an invalid matcher presence")]
    MatcherPresence {
        /// Predicate whose matcher requirement was contradicted.
        predicate: PredicateKind,
    },
    /// Tool-match syntax must be admitted before the behavior is sealed.
    #[error("invalid tool matcher `{matcher}`: {source}")]
    InvalidToolMatcher {
        /// Rejected matcher spelling from the declaration.
        matcher: String,
        #[source]
        /// Typed grammar failure raised by the matcher parser.
        source: crate::records::InvalidToolCallMatcher,
    },
    /// Duplicate matcher rows are ambiguous catalog noise.
    #[error("duplicate matcher `{0}`")]
    DuplicateMatcher(String),
    /// Path comparison options cannot silently decorate another predicate.
    #[error("path-only options require predicate_kind = path_match")]
    PathOptionsWithoutPathPredicate,
    /// Only activation carries a target mode, and every activation needs one.
    #[error("activate_mode must be present exactly for action = activate")]
    ActivateModeMismatch,
    /// A declared bound must represent positive work time.
    #[error("timeout_ms must be greater than zero when present")]
    ZeroTimeout,
    /// Durable state has exactly one named owner; other state scopes have none.
    #[error("governed_by must be non-empty exactly when state_scope = durable")]
    DurableOwnerMismatch,
    /// Fallback guidance must carry useful content.
    #[error("lifecycle fallback guidance must not be empty")]
    EmptyFallbackGuidance,
}

/// A safety-relevant gate is closed by default; guidance behaviours override to open.
fn default_failure() -> FailurePosture {
    FailurePosture::FailClosed
}
fn default_predicate() -> PredicateKind {
    PredicateKind::Always
}
fn default_state_scope() -> StateScope {
    StateScope::Stateless
}

/// Private lifecycle-behaviour catalog carried by [`crate::records::CapabilitiesFile`].
///
/// This is concrete workspace action configuration, not Package substance or a
/// distribution-global semantic selection. The catalog is digested into the
/// effective profile before any runtime can evaluate it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LifecycleBehaviorCatalog {
    #[serde(rename = "behaviour")]
    /// Behavior declarations in authored catalog order.
    pub behaviours: Vec<LifecycleBehavior>,
}

/// Source-free admitted lifecycle behavior carried by an effective profile.
///
/// Package-relative guidance paths are resolved and materialized before this
/// record is constructed; generated consumers never need the private source
/// file or its location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdmittedLifecycleBehaviorV1 {
    /// Stable private behavior identity.
    pub id: LifecycleBehaviorIdV1,
    /// Compact behavior purpose.
    pub description: String,
    /// Materialized guidance content, when the action carries model guidance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guidance: Option<String>,
    /// Native lifecycle event.
    pub event: LifecycleEvent,
    /// Deterministic predicate class.
    pub predicate_kind: PredicateKind,
    /// Validated predicate values.
    pub matchers: Vec<String>,
    /// Whether path matching is case-insensitive.
    pub case_insensitive: bool,
    /// Whether path targets must already exist.
    pub existing_targets_only: bool,
    /// Whether an admitted operation suppresses the path predicate.
    pub allow_admitted_operation: bool,
    /// Deterministic lifecycle action.
    pub action: LifecycleAction,
    /// Qualified run mode activated by this behavior, when applicable.
    pub activate_mode: Option<PackageResourceRefV1>,
    /// Qualified run modes in which this behavior is active.
    pub modes: Vec<PackageResourceRefV1>,
    /// Portable capabilities required by the behavior.
    pub tool_reqs: Vec<CapabilityIdV1>,
    /// Evaluation failure posture.
    pub failure_posture: FailurePosture,
    /// State ownership class.
    pub state_scope: StateScope,
    /// Native owner of durable state, when applicable.
    pub governed_by: Option<String>,
    /// Evaluation timeout in milliseconds, when bounded.
    pub timeout_ms: Option<u32>,
    /// Generic fallback retained independently of adapter support.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<LifecycleFallbackV1>,
}

impl AdmittedLifecycleBehaviorV1 {
    /// Revalidate the deterministic lifecycle grammar after deserialization.
    ///
    /// The source guidance path has already been replaced by content, so it is
    /// deliberately excluded. Every action, predicate, state, matcher, and
    /// fallback law is delegated to the source family's single owning validator.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidLifecycleBehavior`] when retained mechanics contradict
    /// the lifecycle grammar.
    pub fn validate_contract(&self) -> Result<(), InvalidLifecycleBehavior> {
        LifecycleBehavior {
            id: self.id.clone(),
            description: self.description.clone(),
            guidance: None,
            event: self.event,
            predicate_kind: self.predicate_kind,
            matchers: self.matchers.clone(),
            case_insensitive: self.case_insensitive,
            existing_targets_only: self.existing_targets_only,
            allow_admitted_operation: self.allow_admitted_operation,
            action: self.action,
            activate_mode: self.activate_mode.clone(),
            modes: self.modes.clone(),
            tool_reqs: self.tool_reqs.clone(),
            failure_posture: self.failure_posture,
            state_scope: self.state_scope,
            governed_by: self.governed_by.clone(),
            timeout_ms: self.timeout_ms,
            fallback: self.fallback.clone(),
        }
        .validate_contract()
    }
}

impl LifecycleBehaviorCatalog {
    /// Clone one behavior after catalog-level validation.
    #[must_use]
    pub fn project(&self, behavior: &LifecycleBehavior) -> LifecycleBehavior {
        behavior.clone()
    }
}
