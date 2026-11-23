//! Exact Package-release inspection over one retained distribution snapshot.

use std::path::Path;

use pale_core::records::PackageReleaseV1;
use pale_ports::SourceFilesystemPort;

use super::{DistributionSnapshot, PackageError, PackageReleaseIndex};

/// Machine-readable lock inputs derived from one retained Package byte set.
///
/// Inspection reports exact releases but does not select an occurrence, write a
/// lock, admit Package meaning, activate a runtime, or perform a native effect.
#[derive(Debug, serde::Serialize)]
pub struct DistributionInfo {
    compiler: String,
    schema_manifest_sha256: pale_core::records::SchemaManifestDigest,
    schema_set_sha256: pale_core::records::SchemaSetDigest,
    packages: Vec<PackageReleaseV1>,
}

impl DistributionInfo {
    /// Encode these exact inspected coordinates as a distribution-lock document.
    ///
    /// The result is proposed source input, not publication or lock adoption.
    ///
    /// # Errors
    ///
    /// Returns the native TOML encoder error if the typed record cannot encode.
    pub fn lockfile_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(&pale_core::records::DistributionLockFile {
            distribution: pale_core::records::DistributionMeta {
                compiler: self.compiler.clone(),
                schema_manifest_sha256: self.schema_manifest_sha256.clone(),
                schema_set_sha256: self.schema_set_sha256.clone(),
            },
            packages: self.packages.clone(),
        })
    }

    /// Compiler contract required by the inspected Package releases.
    #[must_use]
    pub fn compiler(&self) -> &str {
        &self.compiler
    }

    /// Digest of the exact generated schema manifest used for admission.
    #[must_use]
    pub fn schema_manifest_sha256(&self) -> &pale_core::records::SchemaManifestDigest {
        &self.schema_manifest_sha256
    }

    /// Digest of the complete generated schema byte set used for admission.
    #[must_use]
    pub fn schema_set_sha256(&self) -> &pale_core::records::SchemaSetDigest {
        &self.schema_set_sha256
    }

    /// Canonically ordered exact Package releases available for lock authoring.
    #[must_use]
    pub fn packages(&self) -> &[PackageReleaseV1] {
        &self.packages
    }
}

/// Derive canonical Package lock inputs from one bounded retained snapshot.
///
/// # Errors
///
/// Returns [`PackageError`] when snapshot capture, manifest admission, source
/// bijection, or exact release digesting fails.
pub fn inspect_distribution(
    sources: &dyn SourceFilesystemPort,
    distribution_root: &Path,
) -> Result<DistributionInfo, PackageError> {
    let snapshot = DistributionSnapshot::capture(sources, distribution_root)?;
    inspect_snapshot(&snapshot)
}

/// Derive the same lock inputs from an already-retained Package snapshot.
///
/// This is the semantic half of [`inspect_distribution`]. Source-backed and
/// path-free callers therefore share manifest admission, source-custody
/// validation, release ordering, and schema coordinates without converting a
/// retained snapshot back into host paths.
pub(crate) fn inspect_snapshot(
    snapshot: &DistributionSnapshot,
) -> Result<DistributionInfo, PackageError> {
    let index = PackageReleaseIndex::capture(snapshot)?;
    let registry = pale_contracts::schema_registry_identity()?;
    Ok(DistributionInfo {
        compiler: crate::compile::DISTRIBUTION_COMPILER_ID.to_string(),
        schema_manifest_sha256: registry.schema_manifest_sha256,
        schema_set_sha256: registry.schema_set_sha256,
        packages: index.releases().cloned().collect(),
    })
}
