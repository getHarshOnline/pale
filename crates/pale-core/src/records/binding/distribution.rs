//! Portable pins for one exact distributable input set.
//!
//! [`DistributionLockFile`] binds compiler, schema registry, and exact Package
//! releases. It is compilation input. It does not establish workspace selection,
//! Pack attestation, installation, activation, or provider availability.

use serde::{Deserialize, Serialize};

use crate::records::{PackageReleaseV1, SchemaManifestDigest, SchemaSetDigest};

/// Compiler and schema-registry identity selected by the distribution lock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistributionMeta {
    /// Exact compiler release identity required by this lock.
    #[schemars(length(min = 1))]
    pub compiler: String,
    /// Exact descriptor-manifest identity compiled into the admitted runtime.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Exact active raw-schema-set identity compiled into the admitted runtime.
    pub schema_set_sha256: SchemaSetDigest,
}

/// The complete distribution lock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistributionLockFile {
    /// Compiler and schema-registry identity for the distribution.
    pub distribution: DistributionMeta,
    /// Complete package lock entries, serialized under repeated `package` keys.
    #[serde(rename = "package", default)]
    pub packages: Vec<PackageReleaseV1>,
}
