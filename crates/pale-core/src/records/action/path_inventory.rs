//! Wire records for the filesystem-independent path-inventory action.
//!
//! [`PathInventoryInput`] selects an explicit lexical grammar. Normalization
//! neither consults a filesystem nor proves that a path exists or is confined.

use serde::{Deserialize, Serialize};

/// Contract and dispatch key for lexical path inventory in
/// [`crate::records::NativeActionCatalogV1`].
pub const PATH_INVENTORY_ACTION_ID: &str = "pale.compute.path-inventory";

wire_enum!(
    /// Portable lexical path grammar used by the path-inventory action.
    PathStyle {
        /// Paths are relative to an abstract lexical root and may not escape it.
        PosixRelative => "posix_relative",
        /// Paths are rooted POSIX paths normalized without filesystem access.
        PosixAbsolute => "posix_absolute",
    }
);

/// Caller-supplied paths and an explicit portable grammar. No path is inspected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PathInventoryInput {
    /// Grammar applied uniformly to every supplied path.
    pub style: PathStyle,
    /// Paths to normalize lexically; duplicates are counted after normalization.
    #[schemars(length(max = 100_000), inner(length(min = 1, max = 4_096)))]
    pub paths: Vec<String>,
}

/// One unique normalized path and its number of supplied occurrences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PathInventoryEntry {
    /// Canonical lexical spelling under the selected path style.
    #[schemars(length(min = 1))]
    pub normalized_path: String,
    /// Number of supplied paths that normalize to this spelling.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub occurrences: u64,
}

/// Sorted, deduplicated lexical inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PathInventoryOutput {
    /// Number of paths supplied before normalization.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub input_count: u64,
    /// Number of distinct normalized paths.
    #[schemars(schema_with = "super::safe_u64_schema")]
    pub unique_count: u64,
    /// Unique entries in ascending normalized-path order.
    pub entries: Vec<PathInventoryEntry>,
}
