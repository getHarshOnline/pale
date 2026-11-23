//! Portable session declarations and machine-local session storage.

use std::collections::BTreeSet;
use std::num::NonZeroU64;

use serde::{Deserialize, Deserializer, Serialize};

use crate::records::{ExecutionPathSelectionIdV1, LifecycleEvent, LogicalRepositoryIdV1};

/// Access ceiling declared for one logical authority root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum AuthorityAccessClass {
    /// The session may read the root after admission.
    ///
    /// This declaration does not itself stop a native write. An admitted hook
    /// or effect adapter must apply the resolved write-protection policy.
    #[serde(rename = "read-only")]
    ReadOnly,
}

/// Runtime source selected by a portable session declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum SessionRuntimeSelection {
    /// Request resolution through the selected machine binding.
    #[serde(rename = "machine-local")]
    MachineLocal,
}

/// One declared logical authority-root access ceiling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthorityAccess {
    /// Logical authority-root identity declared by the selected project.
    #[schemars(length(min = 1))]
    pub root_id: String,
    /// Access ceiling to enforce after session admission.
    pub class: AuthorityAccessClass,
}

/// One ordered path relative to a logical authority root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadingOrderEntry {
    /// Logical authority-root identity beneath which the path must remain.
    #[schemars(length(min = 1))]
    pub root_id: String,
    /// Declared relative path; the loader must parse and confine it beneath `root_id`.
    #[schemars(length(min = 1))]
    pub rel_path: String,
}

/// Portable session activation and context declaration.
///
/// Deserialization requires at least one unique activation root and keeps
/// activation and additional roots disjoint. Direct struct construction does
/// not perform those relational checks; a producing compiler must validate the
/// same invariant before sealing a constructed value. Neither path validates
/// repository existence, authority-root access, path confinement, or activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct SessionModel {
    /// Repositories declared eligible for activation after admission.
    #[schemars(
        length(min = 1),
        inner(length(min = 1)),
        extend("uniqueItems" = true)
    )]
    pub activation_roots: Vec<LogicalRepositoryIdV1>,
    /// Non-entry repositories exposed to every activated session.
    #[serde(default)]
    #[schemars(
        inner(length(min = 1)),
        extend("uniqueItems" = true)
    )]
    pub additional_roots: Vec<LogicalRepositoryIdV1>,
    /// Declared authority-root accesses, each with its ceiling.
    #[serde(default)]
    pub authority_access: Vec<AuthorityAccess>,
    /// Neutral execution paths eligible for explicit session admission.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub activation_path_ids: Vec<ExecutionPathSelectionIdV1>,
    /// Native lifecycle events declared for later admission and wiring.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub lifecycle_events: Vec<LifecycleEvent>,
    /// Logical runtime selection; the machine binding owns installed identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "SessionRuntimeSelection")]
    pub runtime: Option<SessionRuntimeSelection>,
    /// Ordered project-authority documents resolved beneath declared roots.
    #[serde(default)]
    pub reading_order: Vec<ReadingOrderEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionModelWire {
    activation_roots: Vec<LogicalRepositoryIdV1>,
    #[serde(default)]
    additional_roots: Vec<LogicalRepositoryIdV1>,
    #[serde(default)]
    authority_access: Vec<AuthorityAccess>,
    #[serde(default)]
    activation_path_ids: Vec<ExecutionPathSelectionIdV1>,
    #[serde(default)]
    lifecycle_events: Vec<LifecycleEvent>,
    #[serde(default)]
    runtime: Option<SessionRuntimeSelection>,
    #[serde(default)]
    reading_order: Vec<ReadingOrderEntry>,
}

impl<'de> Deserialize<'de> for SessionModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = SessionModelWire::deserialize(deserializer)?;
        if wire.activation_roots.is_empty() {
            return Err(serde::de::Error::custom(
                "activation_roots must contain at least one repository",
            ));
        }
        let activation = wire.activation_roots.iter().collect::<BTreeSet<_>>();
        if activation.len() != wire.activation_roots.len() {
            return Err(serde::de::Error::custom("activation_roots must be unique"));
        }
        let additional = wire.additional_roots.iter().collect::<BTreeSet<_>>();
        if additional.len() != wire.additional_roots.len() {
            return Err(serde::de::Error::custom("additional_roots must be unique"));
        }
        if activation.iter().any(|root| additional.contains(root)) {
            return Err(serde::de::Error::custom(
                "activation_roots and additional_roots must be disjoint",
            ));
        }
        Ok(Self {
            activation_roots: wire.activation_roots,
            additional_roots: wire.additional_roots,
            authority_access: wire.authority_access,
            activation_path_ids: wire.activation_path_ids,
            lifecycle_events: wire.lifecycle_events,
            runtime: wire.runtime,
            reading_order: wire.reading_order,
        })
    }
}

/// A nonzero lifetime for ephemeral hook-session state, expressed in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct SessionTtlSeconds(
    #[schemars(schema_with = "crate::canon::positive_json_safe_u64_schema")] NonZeroU64,
);

impl SessionTtlSeconds {
    /// Construct a session-state lifetime in the portable JCS integer domain.
    ///
    /// Returns [`None`] for zero or a value above
    /// [`crate::canon::JSON_SAFE_INTEGER_MAX`].
    pub const fn new(seconds: u64) -> Option<Self> {
        match NonZeroU64::new(seconds) {
            Some(seconds) if seconds.get() <= crate::canon::JSON_SAFE_INTEGER_MAX => {
                Some(Self(seconds))
            }
            None => None,
            Some(_) => None,
        }
    }

    /// Return the configured lifetime in seconds.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl<'de> Deserialize<'de> for SessionTtlSeconds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds = u64::deserialize(deserializer)?;
        Self::new(seconds).ok_or_else(|| {
            serde::de::Error::custom("session TTL must be a positive JCS-safe integer")
        })
    }
}

/// Machine-local storage declaration for ephemeral hook session state.
///
/// The adapter validates the root and publication permissions and determines
/// record freshness. The configured TTL is policy, not proof of freshness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionStateBinding {
    /// Declared absolute path to a dedicated Pale state directory.
    #[schemars(length(min = 1))]
    pub root: String,
    /// Non-zero maximum age of one session record before it is treated as absent.
    pub ttl_seconds: SessionTtlSeconds,
}
