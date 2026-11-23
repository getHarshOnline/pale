//! Path-free Package-release inspection for lock authoring and repair.
//!
//! The façade accepts the same bounded logical Package documents as one-shot
//! compilation, but deliberately does not accept a distribution lock. It
//! returns exact immutable release coordinates; it does not select Packages,
//! admit semantic meaning, write a lock, or activate a runtime.

use pale_application::compile::{CapturedPackageDocument, CapturedPackageSource};
use pale_application::packages::{PackageError, PackageGraphInvariant};
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticFactV1Wire, DiagnosticRetryV1,
    DiagnosticSourceV1Wire, DiagnosticStageV1, DiagnosticSubjectV1Wire,
};
use serde::Serialize;
use std::path::Path;

use crate::error::{ApiError, FailureSpecification};
use crate::{CompilationDocument, PaleApi};

#[cfg(feature = "native-local")]
mod callers;
#[cfg(feature = "native-local")]
pub use callers::LocalPackageCheckRequest;

/// Complete bounded Package byte set for one distribution inspection.
///
/// Construction performs no parsing. [`PaleApi::inspect_package_distribution`]
/// admits every logical label, finite source-tree bound, manifest, and source
/// inventory before returning a release coordinate.
pub struct PackageDistributionRequest {
    documents: Vec<CompilationDocument>,
}

/// Checked Package declarations for authoring tools, encoded by the shared reader.
///
/// The JSON includes source coordinates and declared relationships. It describes
/// reusable guidance and grants no workspace selection or permission to act.
#[derive(Debug)]
pub struct PackageCatalogInspection {
    canonical_bytes: Vec<u8>,
    catalog: pale_application::packages::PackageCatalog,
}

impl PackageCatalogInspection {
    /// Index paths whose captured bytes differ from the generated navigation.
    #[must_use]
    pub fn stale_indexes(&self) -> Vec<&str> {
        self.catalog
            .indexes()
            .iter()
            .filter(|index| index.is_stale())
            .map(|index| index.path())
            .collect()
    }
    /// Canonical JSON from the compiler-owned Package reader.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Show one resource's body and declared reading links, if it owns a body.
    #[must_use]
    pub fn show(&self, resource: &str) -> Option<String> {
        self.catalog.show(resource)
    }

    fn project(
        catalog: pale_application::packages::PackageCatalog,
        identity: &pale_core::records::RecordSchemaIdentity,
    ) -> Result<Self, ApiError> {
        let canonical_bytes = pale_core::canon::to_canonical_string(&catalog)
            .map_err(|error| projection_failure(identity, error))?
            .into_bytes();
        Ok(Self {
            canonical_bytes,
            catalog,
        })
    }
}

impl std::fmt::Debug for PackageDistributionRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PackageDistributionRequest")
            .field("document_count", &self.documents.len())
            .finish()
    }
}

impl PackageDistributionRequest {
    /// Bind logical Package documents to one inspection request.
    #[must_use]
    pub fn new(documents: Vec<CompilationDocument>) -> Self {
        Self { documents }
    }
}

/// Exact immutable identity of one inspected Package release.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PackageReleaseIdentity {
    id: String,
    version: String,
    content_digest: String,
}

impl PackageReleaseIdentity {
    /// Canonical Package identity.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Exact owner-published release label.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Digest of the complete manifest and owned source inventory.
    #[must_use]
    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }
}

/// Canonically ordered lock inputs derived from one admitted Package byte set.
///
/// The canonical bytes and typed accessors are projections of the same private
/// fields. CLI and language leaves can therefore deliver the wire form without
/// reconstructing this record or running another canonicalizer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PackageDistributionInspection {
    compiler: String,
    schema_manifest_sha256: String,
    schema_set_sha256: String,
    packages: Vec<PackageReleaseIdentity>,
    #[serde(skip)]
    canonical_bytes: Vec<u8>,
    #[serde(skip)]
    lockfile_toml: String,
}

impl PackageDistributionInspection {
    fn project(
        info: pale_application::packages::DistributionInfo,
        identity: &pale_core::records::RecordSchemaIdentity,
    ) -> Result<Self, ApiError> {
        let lockfile_toml = info
            .lockfile_toml()
            .map_err(|error| projection_failure(identity, error))?;
        let mut inspection = Self {
            compiler: info.compiler().to_string(),
            schema_manifest_sha256: info.schema_manifest_sha256().as_str().to_string(),
            schema_set_sha256: info.schema_set_sha256().as_str().to_string(),
            packages: info
                .packages()
                .iter()
                .map(|release| PackageReleaseIdentity {
                    id: release.id.as_str().to_string(),
                    version: release.version.as_str().to_string(),
                    content_digest: release.content_digest.as_str().to_string(),
                })
                .collect(),
            canonical_bytes: Vec::new(),
            lockfile_toml,
        };
        inspection.canonical_bytes = pale_core::canon::to_canonical_string(&inspection)
            .map_err(|error| projection_failure(identity, error))?
            .into_bytes();
        Ok(inspection)
    }

    /// Compiler contract required by the inspected releases.
    #[must_use]
    pub fn compiler(&self) -> &str {
        &self.compiler
    }

    /// Digest of the exact generated schema manifest used for admission.
    #[must_use]
    pub fn schema_manifest_sha256(&self) -> &str {
        &self.schema_manifest_sha256
    }

    /// Digest of the complete generated schema byte set used for admission.
    #[must_use]
    pub fn schema_set_sha256(&self) -> &str {
        &self.schema_set_sha256
    }

    /// Canonically ordered exact Package releases available for lock authoring.
    #[must_use]
    pub fn packages(&self) -> &[PackageReleaseIdentity] {
        &self.packages
    }

    /// RFC 8785 JSON projection shared by process and language transports.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Native TOML lock for exactly the inspected releases and schema registry.
    ///
    /// Emitting this proposed source does not write, adopt or approve a lock.
    #[must_use]
    pub fn lockfile_toml(&self) -> &str {
        &self.lockfile_toml
    }
}

impl PaleApi {
    /// Regenerate family indexes after checking the complete local Package tree.
    ///
    /// Only generated `INDEX.md` files are written. All index preconditions are
    /// checked before the first write; each file is replaced atomically. A failure
    /// during publication may leave a partial update, so inspect before retrying.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] for rejected sources, changed index bytes or failed writes.
    #[cfg(feature = "native-local")]
    pub fn update_local_package_indexes(
        &self,
        root: String,
    ) -> Result<PackageCatalogInspection, ApiError> {
        let catalog = self.read_local_package_catalog(root.clone())?;
        self.write_package_indexes(root, catalog)
    }

    #[cfg(feature = "native-local")]
    fn write_package_indexes(
        &self,
        root: String,
        catalog: PackageCatalogInspection,
    ) -> Result<PackageCatalogInspection, ApiError> {
        use pale_adapter_fs::{ConfinedRoot, ReadLimit};
        let directory = ConfinedRoot::open(&Path::new(&root).join("packages"))
            .map_err(|error| index_update_failure(&self.diagnostic_identity, error, false))?;
        let mut changes = Vec::new();
        for index in catalog
            .catalog
            .indexes()
            .iter()
            .filter(|index| index.is_stale())
        {
            let location = directory
                .location(index.path())
                .map_err(|error| index_update_failure(&self.diagnostic_identity, error, false))?;
            let current = location
                .read_file(ReadLimit::new(4 * 1024 * 1024))
                .map_err(|error| index_update_failure(&self.diagnostic_identity, error, false))?;
            if current.as_ref().map(|file| file.bytes.as_slice())
                != index.current().map(str::as_bytes)
            {
                return Err(index_update_failure(
                    &self.diagnostic_identity,
                    std::io::Error::other("Package index changed after capture"),
                    false,
                ));
            }
            let mode = current.map_or(0o644, |file| file.mode);
            changes.push((location, index.content().as_bytes(), mode));
        }
        let wrote = !changes.is_empty();
        for (location, bytes, mode) in changes {
            location
                .write_atomic(bytes, mode)
                .map_err(|error| index_update_failure(&self.diagnostic_identity, error, true))?;
        }
        let updated = self
            .read_local_package_catalog(root)
            .map_err(|error| index_update_failure(&self.diagnostic_identity, error, wrote))?;
        if !updated.stale_indexes().is_empty() {
            return Err(index_update_failure(
                &self.diagnostic_identity,
                std::io::Error::other(
                    "Package indexes differ from the checked sources after writing",
                ),
                wrote,
            ));
        }
        Ok(updated)
    }

    /// Read a local Package distribution for authoring and navigation.
    ///
    /// `root` names the distribution directory containing `packages/`. Capture
    /// refuses symlinks and applies the compiler's source limits. Reading checks
    /// the declarations, dependencies and body links without a workspace or lock.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when source capture or Package checks fail.
    #[cfg(feature = "native-local")]
    pub fn read_local_package_catalog(
        &self,
        root: String,
    ) -> Result<PackageCatalogInspection, ApiError> {
        let catalog = pale_application::packages::read_catalog(
            &pale_adapter_fs::LocalSourceFilesystem,
            Path::new(&root),
        )
        .map_err(|error| package_failure(&self.diagnostic_identity, error))?;
        PackageCatalogInspection::project(catalog, &self.diagnostic_identity)
    }

    /// Read captured Package documents through the compiler's authoring reader.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] for invalid labels, source limits, declarations or links.
    pub fn read_package_catalog(
        &self,
        request: PackageDistributionRequest,
    ) -> Result<PackageCatalogInspection, ApiError> {
        let documents = request
            .documents
            .into_iter()
            .map(|document| {
                let (label, bytes) = document.into_parts();
                CapturedPackageDocument::new(label, bytes)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| source_failure(&self.diagnostic_identity, error))?;
        let packages = CapturedPackageSource::new(documents)
            .map_err(|error| source_failure(&self.diagnostic_identity, error))?;
        let catalog = packages
            .read_catalog()
            .map_err(|error| package_failure(&self.diagnostic_identity, error))?;
        PackageCatalogInspection::project(catalog, &self.diagnostic_identity)
    }

    /// Inspect Package release coordinates without consuming a distribution lock.
    ///
    /// This is a source-admission operation for lock authoring and repair. It
    /// admits only the supplied Package tree and therefore cannot prove Package
    /// selection, dependency closure, semantic compilation, runtime readiness,
    /// publication, or any native effect.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when logical labels or finite bounds are invalid, a
    /// Package manifest or source inventory is incoherent, duplicate owners are
    /// observed, or the embedded schema coordinate contradicts the Pale application.
    pub fn inspect_package_distribution(
        &self,
        request: PackageDistributionRequest,
    ) -> Result<PackageDistributionInspection, ApiError> {
        let documents = request
            .documents
            .into_iter()
            .map(|document| {
                let (label, bytes) = document.into_parts();
                CapturedPackageDocument::new(label, bytes)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| source_failure(&self.diagnostic_identity, error))?;
        let packages = CapturedPackageSource::new(documents)
            .map_err(|error| source_failure(&self.diagnostic_identity, error))?;
        let info = packages
            .inspect_distribution()
            .map_err(|error| package_failure(&self.diagnostic_identity, error))?;
        PackageDistributionInspection::project(info, &self.diagnostic_identity)
    }
}

fn source_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: pale_application::compile::CapturedCompilationInputError,
) -> ApiError {
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: "package_distribution_source_rejected",
            stage: DiagnosticStageV1::InputAdmission,
            disposition: DiagnosticDispositionV1::InvalidInput,
            summary: "captured Package distribution source was rejected",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::AfterCorrection,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}

fn package_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: PackageError,
) -> ApiError {
    let classification = classify_package_failure(&error);
    let projection = PackageFailureProjection::from_error(&error);
    let (disposition, retry) =
        crate::error::application_diagnostic_policy(error.failure_disposition());
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: classification.code,
            stage: classification.stage,
            disposition,
            summary: classification.summary,
            subject: projection.subject,
            schema_coordinate: None,
            location: projection.location,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: classification.notes,
            help: classification.help,
            facts: projection.facts,
        },
    )
}

/// Public policy attached to one closed Package-admission failure class.
struct PackageFailureClassification {
    code: &'static str,
    stage: DiagnosticStageV1,
    summary: &'static str,
    notes: Vec<&'static str>,
    help: Vec<&'static str>,
}

/// Exhaustively project the application-owned Package algebra into API policy.
fn classify_package_failure(error: &PackageError) -> PackageFailureClassification {
    let (code, stage, summary, notes, help) = match error {
        PackageError::PageHeader { .. }
        | PackageError::TomlSyntax { .. }
        | PackageError::TomlToJson { .. } => (
            "package_distribution_source_syntax_rejected",
            DiagnosticStageV1::InputAdmission,
            "Package source syntax was rejected",
            vec!["No typed Package record was admitted from the rejected source."],
            vec!["Correct the named Package source and inspect the complete distribution again."],
        ),
        PackageError::SourceSchema { .. } => (
            "package_distribution_source_contract_rejected",
            DiagnosticStageV1::ContractValidation,
            "Package source violates its pinned structural contract",
            vec!["Schema validity does not by itself establish Package semantic admission."],
            vec!["Correct the named source value against its exact initial-v1 contract."],
        ),
        PackageError::SchemaCheck(_) => (
            "package_distribution_contract_failed",
            DiagnosticStageV1::ContractValidation,
            "embedded Package distribution contract is incoherent",
            Vec::new(),
            Vec::new(),
        ),
        PackageError::SourceDecode { .. } => (
            "package_distribution_typed_decode_failed",
            DiagnosticStageV1::Internal,
            "schema-admitted Package source could not enter its typed record",
            vec![
                "Structural admission succeeded, but the Pale application state is inconsistent.",
            ],
            Vec::new(),
        ),
        PackageError::Read { .. }
        | PackageError::Allocation { .. }
        | PackageError::UnsupportedPlatform { .. } => (
            "package_distribution_source_unavailable",
            DiagnosticStageV1::PackageLoading,
            "Package distribution source could not be retained safely",
            Vec::new(),
            vec!["Restore the named local prerequisite before inspecting the distribution again."],
        ),
        PackageError::InvalidEntry { .. }
        | PackageError::MissingSnapshotFile { .. }
        | PackageError::NotUtf8 { .. }
        | PackageError::SnapshotLimit { .. }
        | PackageError::InvalidManifest { .. }
        | PackageError::NestedPackageManifest { .. }
        | PackageError::SourceInventoryMismatch { .. }
        | PackageError::ReleaseMismatch { .. }
        | PackageError::MissingLockedPackage { .. }
        | PackageError::MissingExactDependency { .. }
        | PackageError::IncompatibleCompiler { .. }
        | PackageError::ReleaseClosure(_)
        | PackageError::ResourceDeclarationMismatch { .. }
        | PackageError::InvalidBodyCustody { .. } => (
            "package_distribution_custody_rejected",
            DiagnosticStageV1::PackageLoading,
            "Package distribution custody is incomplete or contradictory",
            vec![
                "The retained source tree must be an exact confined projection of each Package manifest.",
            ],
            vec!["Correct the named manifest, source inventory, or exact release coordinate."],
        ),
        PackageError::Diagnostics(_)
        | PackageError::InvalidMarkdownLink { .. }
        | PackageError::InvalidResourceLink { .. }
        | PackageError::InvalidOperationProcedure { .. }
        | PackageError::InvalidResourceSelector { .. }
        | PackageError::GoverningAlgebra(_)
        | PackageError::GraphInvariant(_)
        | PackageError::DuplicatePackage { .. } => (
            "package_distribution_semantics_rejected",
            DiagnosticStageV1::SemanticAdmission,
            "Package distribution meaning was rejected",
            vec![
                "The complete typed Package relation failed admission; it was not partially normalized.",
            ],
            vec![
                "Correct the reported Package identity or semantic relationship and inspect again.",
            ],
        ),
    };
    PackageFailureClassification {
        code,
        stage,
        summary,
        notes,
        help,
    }
}

#[derive(Default)]
struct PackageFailureProjection {
    subject: Option<DiagnosticSubjectV1Wire>,
    location: Option<DiagnosticSourceV1Wire>,
    facts: Vec<DiagnosticFactV1Wire>,
}

impl PackageFailureProjection {
    fn from_error(error: &PackageError) -> Self {
        match error {
            PackageError::PageHeader { path, reason } => Self {
                location: package_location(path, None),
                facts: vec![package_fact("page_header", &reason.to_string())],
                ..Self::default()
            },
            PackageError::Diagnostics(diagnostics) => Self {
                facts: vec![
                    package_fact("observed", &diagnostics.observed_len().to_string()),
                    package_fact("retained", &diagnostics.entries().len().to_string()),
                    package_fact("truncated", &diagnostics.truncated().to_string()),
                ],
                ..Self::default()
            },
            PackageError::InvalidManifest { path, .. }
            | PackageError::TomlSyntax { path, .. }
            | PackageError::TomlToJson { path, .. }
            | PackageError::SourceSchema { path, .. }
            | PackageError::Read { path, .. }
            | PackageError::Allocation { path, .. }
            | PackageError::InvalidEntry { path }
            | PackageError::MissingSnapshotFile { path }
            | PackageError::NotUtf8 { path } => Self {
                location: package_location(path, None),
                ..Self::default()
            },
            PackageError::NestedPackageManifest { package, path } => Self {
                subject: package_subject("package", package.as_ref()),
                location: package_location(path, None),
                ..Self::default()
            },
            PackageError::SourceInventoryMismatch {
                package,
                missing,
                undeclared,
            } => Self {
                subject: package_subject("package", package.as_ref()),
                facts: vec![
                    package_fact("missing_count", &missing.len().to_string()),
                    package_fact("undeclared_count", &undeclared.len().to_string()),
                ],
                ..Self::default()
            },
            PackageError::ReleaseMismatch { package }
            | PackageError::MissingLockedPackage { package } => Self {
                subject: package_subject("package", package.as_ref()),
                ..Self::default()
            },
            PackageError::MissingExactDependency {
                package,
                dependency,
                version,
            } => Self {
                subject: package_subject("package", package.as_ref()),
                facts: vec![
                    package_fact("dependency", dependency.as_ref()),
                    package_fact("version", &version.to_string()),
                ],
                ..Self::default()
            },
            PackageError::IncompatibleCompiler { package, compiler } => Self {
                subject: package_subject("package", package.as_ref()),
                facts: vec![package_fact("compiler", compiler)],
                ..Self::default()
            },
            PackageError::ResourceDeclarationMismatch {
                package,
                source_path,
                resources,
            } => Self {
                subject: package_subject("package", package.as_ref()),
                location: package_location(Path::new(source_path), None),
                facts: vec![package_fact("resource_count", &resources.len().to_string())],
            },
            PackageError::InvalidBodyCustody {
                resource,
                body,
                reason,
            } => Self {
                subject: package_subject("package_resource", &resource.to_string()),
                location: package_location(Path::new(body), None),
                facts: vec![package_fact("reason", &reason.to_string())],
            },
            PackageError::InvalidMarkdownLink {
                from,
                destination_sha256,
                reason,
            } => Self {
                subject: package_subject("package_resource", &from.to_string()),
                facts: vec![
                    package_fact("destination_sha256", destination_sha256.as_str()),
                    package_fact("reason", &reason.to_string()),
                ],
                ..Self::default()
            },
            PackageError::InvalidResourceLink {
                from,
                target,
                relation,
                reason,
            } => Self {
                subject: package_subject("package_resource", &from.to_string()),
                facts: vec![
                    package_fact("target", &target.to_string()),
                    package_fact("relation", &relation.to_string()),
                    package_fact("reason", &reason.to_string()),
                ],
                ..Self::default()
            },
            PackageError::InvalidOperationProcedure { operation, reason } => Self {
                subject: package_subject("operation", &operation.to_string()),
                facts: vec![package_fact("reason", &reason.to_string())],
                ..Self::default()
            },
            PackageError::InvalidResourceSelector {
                package,
                resource,
                reason,
                ..
            } => {
                let subject = resource.as_ref().map_or_else(
                    || package_subject("package", package.as_ref()),
                    |resource| package_subject("package_resource", resource.as_ref()),
                );
                Self {
                    subject,
                    facts: vec![package_fact("reason", &reason.to_string())],
                    ..Self::default()
                }
            }
            PackageError::SourceDecode {
                path,
                label,
                value_path,
                ..
            } => Self {
                location: package_location(path, Some(value_path.clone())),
                facts: vec![package_fact("record", label)],
                ..Self::default()
            },
            PackageError::GraphInvariant(PackageGraphInvariant::MissingRepositorySeeds {
                repository,
            }) => Self {
                subject: package_subject("repository", repository.as_ref()),
                ..Self::default()
            },
            PackageError::DuplicatePackage { id, first, second } => Self {
                subject: package_subject("package", id),
                facts: [
                    ("first_source", first.as_path()),
                    ("second_source", second.as_path()),
                ]
                .into_iter()
                .filter_map(|(key, path)| path.to_str().map(|value| package_fact(key, value)))
                .collect(),
                ..Self::default()
            },
            PackageError::UnsupportedPlatform { platform } => Self {
                facts: vec![package_fact("platform", platform)],
                ..Self::default()
            },
            PackageError::SnapshotLimit {
                limit,
                value,
                maximum,
            } => Self {
                facts: vec![
                    package_fact("limit", limit),
                    package_fact("observed", &value.to_string()),
                    package_fact("maximum", &maximum.to_string()),
                ],
                ..Self::default()
            },
            PackageError::ReleaseClosure(_)
            | PackageError::GoverningAlgebra(_)
            | PackageError::SchemaCheck(_)
            | PackageError::GraphInvariant(
                PackageGraphInvariant::SealMismatch | PackageGraphInvariant::CanonicalEncoding(_),
            ) => Self::default(),
        }
    }
}

fn package_fact(key: &str, value: &str) -> DiagnosticFactV1Wire {
    DiagnosticFactV1Wire {
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn package_subject(kind: &str, id: &str) -> Option<DiagnosticSubjectV1Wire> {
    (!kind.is_empty() && !id.is_empty()).then(|| DiagnosticSubjectV1Wire {
        kind: kind.to_string(),
        id: id.to_string(),
    })
}

fn package_location(path: &Path, value_path: Option<String>) -> Option<DiagnosticSourceV1Wire> {
    let label = path.to_str()?.strip_prefix("./").unwrap_or(path.to_str()?);
    pale_core::paths::ConfinedRelativePath::parse(label)?;
    (!label.is_empty()).then(|| DiagnosticSourceV1Wire {
        label: label.to_string(),
        line: None,
        column: None,
        byte_start: None,
        byte_end: None,
        value_path,
    })
}

fn projection_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: impl std::error::Error + Send + Sync + 'static,
) -> ApiError {
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: "package_distribution_projection_failed",
            stage: DiagnosticStageV1::Internal,
            disposition: DiagnosticDispositionV1::InternalFailure,
            summary: "Package distribution projection could not be encoded",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::NotIndicated,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}

#[cfg(feature = "native-local")]
fn index_update_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: impl std::error::Error + Send + Sync + 'static,
    started: bool,
) -> ApiError {
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: "package_index_update_failed",
            stage: DiagnosticStageV1::Materialization,
            disposition: if started {
                DiagnosticDispositionV1::EffectOutcomeUnknown
            } else {
                DiagnosticDispositionV1::Blocked
            },
            summary: "Package index update did not finish",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: if started {
                DiagnosticRetryV1::ReconcileFirst
            } else {
                DiagnosticRetryV1::AfterCorrection
            },
            effect: if started {
                DiagnosticEffectCertaintyV1::OutcomeUnknown
            } else {
                DiagnosticEffectCertaintyV1::NotStarted
            },
            request: None,
            notes: vec![
                "Each index is replaced atomically; the complete set is not one transaction.",
            ],
            help: vec!["Run packages check and inspect the changed indexes before retrying."],
            facts: Vec::new(),
        },
    )
}

#[cfg(test)]
#[path = "../tests/internal/distribution.rs"]
mod tests;
