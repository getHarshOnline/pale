//! Content-addressed claims about generated repository artifacts.
//!
//! Deserialization validates only the record and digest representation. A
//! consuming boundary must parse [`SealedArtifactRef::rel_path`] as a
//! [`crate::paths::ConfinedRelativePath`], read beneath an opened trusted root,
//! hash the exact bytes, and compare the result before use.

use serde::{Deserialize, Serialize};

use crate::records::ArtifactDigest;

/// Portable reference claiming one repository-relative artifact and byte digest.
///
/// This value proves neither path confinement, existence, materialization, nor
/// semantic admission.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct SealedArtifactRef {
    /// Declared repository-relative artifact path.
    pub rel_path: String,
    /// Claimed SHA-256 identity of the artifact's exact bytes.
    pub sha256: ArtifactDigest,
}
