//! Machine-local bindings for recoverable evidence and immutable lineage.

use std::num::{NonZeroU32, NonZeroU64};

use serde::{Deserialize, Deserializer, Serialize};

/// A nonzero per-collection route-record ceiling within Pale's hard limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct RouteRecordLimit(#[schemars(range(min = 1, max = 65_536))] NonZeroU32);

impl RouteRecordLimit {
    /// Hard limit enforced by [`Self::new`] and deserialization.
    ///
    /// Reaching this limit requires owner-authorized archival; callers must not
    /// treat it as a rollover point.
    pub const MAX: u32 = 65_536;

    /// Construct a nonzero route-record ceiling no greater than [`Self::MAX`].
    ///
    /// Returns [`None`] when `records` is zero or exceeds the hard limit.
    pub const fn new(records: u32) -> Option<Self> {
        match NonZeroU32::new(records) {
            Some(records) if records.get() <= Self::MAX => Some(Self(records)),
            _ => None,
        }
    }

    /// Return the configured record-count ceiling.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

impl<'de> Deserialize<'de> for RouteRecordLimit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let records = u32::deserialize(deserializer)?;
        Self::new(records).ok_or_else(|| {
            serde::de::Error::custom(format_args!(
                "route-record limit must be in 1..={}",
                Self::MAX
            ))
        })
    }
}

/// A nonzero retention interval expressed in seconds.
///
/// This is a retention policy for recoverable evidence, not a session lifetime
/// or proof that an individual record remains valid for the whole interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct EvidenceRetentionSeconds(
    #[schemars(schema_with = "crate::canon::positive_json_safe_u64_schema")] NonZeroU64,
);

impl EvidenceRetentionSeconds {
    /// Construct a retention interval in the portable JCS integer domain.
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

    /// Return the configured interval in seconds.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl<'de> Deserialize<'de> for EvidenceRetentionSeconds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds = u64::deserialize(deserializer)?;
        Self::new(seconds).ok_or_else(|| {
            serde::de::Error::custom("evidence retention must be a positive JCS-safe integer")
        })
    }
}

/// Machine-local storage declaration for bounded recoverable evidence.
///
/// This binding does not own ephemeral session continuity or immutable
/// materialization and execution-path lineage. Its path and retention policy do
/// not themselves authorize publication, pruning, or any filesystem effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DurableEvidenceBinding {
    /// Declared absolute path to a dedicated Pale evidence directory.
    #[schemars(length(min = 1))]
    pub root: String,
    /// Age after which bounded recoverable evidence becomes eligible to prune.
    pub retention_seconds: EvidenceRetentionSeconds,
}

/// Storage declaration for machine-local materialization lineage.
///
/// The consuming adapter validates that the root is absolute and outside target
/// checkouts. Presence of this value proves neither durability nor ownership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterializationLineageBinding {
    /// Declared absolute machine-local directory outside every target checkout.
    #[schemars(length(min = 1))]
    pub root: String,
    /// Saturation ceiling for each append-only route request/receipt collection.
    /// Full collections require an owner-authorized archive before new records.
    pub max_route_records_per_collection: RouteRecordLimit,
}
