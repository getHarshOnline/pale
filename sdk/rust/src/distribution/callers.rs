//! Local caller capture for the shared Package authoring check.

use std::path::{Component, PathBuf};

use pale_application::packages::PackageCallerReport;
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
};
use pale_ports::SourceTreeLimits;

use super::{package_fact, ApiError, FailureSpecification, PackageCatalogInspection, PaleApi};

/// Local Package authoring inputs, separate from workspace selection.
#[derive(Debug)]
pub struct LocalPackageCheckRequest {
    /// Distribution directory containing `packages/`.
    pub root: String,
    /// Caller directories to inspect for Markdown links and qualified inline IDs.
    pub caller_roots: Vec<String>,
    /// Relative subtrees to exclude in each caller root. A single component
    /// excludes that name at any depth; longer paths are relative to each root.
    pub exclude_callers: Vec<String>,
    /// Regenerate family indexes only after all checks pass.
    pub write: bool,
}

impl PaleApi {
    /// Check Package declarations, paragraph formatting and selected caller trees.
    ///
    /// Caller capture reads only `.md` and `.mdx` files, skips `.git`, `target`
    /// and `node_modules`, and excludes the distribution's own Package tree,
    /// whose authored links the catalog already checks. Additional exclusions
    /// are explicit so historical records need not pretend to use current IDs.
    /// Sources are bounded, captured without following symlinks, and checked
    /// against one admitted catalog before any index write.
    ///
    /// # Errors
    ///
    /// Returns an API error for invalid input, unsafe or oversized source trees,
    /// wrapped paragraphs, missing caller targets, unknown IDs, or index publication failure.
    pub fn check_local_packages(
        &self,
        request: LocalPackageCheckRequest,
    ) -> Result<PackageCatalogInspection, ApiError> {
        let fail = |error| caller_failure(&self.diagnostic_identity, error, None);
        if request.caller_roots.len() > 16 || request.exclude_callers.len() > 256 {
            return Err(fail(std::io::Error::other(
                "too many caller roots or exclusions",
            )));
        }
        let exclusions = request
            .exclude_callers
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        if exclusions.iter().any(|path| {
            path.as_os_str().is_empty()
                || path
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
        }) {
            return Err(fail(std::io::Error::other(
                "caller exclusions must be confined relative paths",
            )));
        }
        let root = std::path::absolute(&request.root).map_err(fail)?;
        let catalog = self.read_local_package_catalog(request.root.clone())?;
        if !catalog.catalog.formatting_issues().is_empty() {
            return Err(formatting_failure(
                &self.diagnostic_identity,
                catalog.catalog.formatting_issues(),
            ));
        }
        let mut report = PackageCallerReport::default();
        let mut total_bytes = 0usize;
        for caller_root in &request.caller_roots {
            let caller_root = std::path::absolute(caller_root).map_err(fail)?;
            let snapshot = pale_adapter_fs::LocalSourceFilesystem
                .capture_selected_tree(
                    &caller_root,
                    SourceTreeLimits::new(250_000, 64, 4 * 1024 * 1024, 128 * 1024 * 1024),
                    &|path| {
                        normalized(&caller_root.join(path)) == normalized(&root.join("packages"))
                            || path.file_name().is_some_and(|name| {
                                [".git", "target", "node_modules"]
                                    .iter()
                                    .any(|skip| name == *skip)
                            })
                            || exclusions.iter().any(|excluded| {
                                if excluded.components().count() == 1 {
                                    path.file_name() == excluded.file_name()
                                } else {
                                    path.starts_with(excluded)
                                }
                            })
                    },
                    &|path| {
                        path.extension().is_some_and(|ext| {
                            ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("mdx")
                        })
                    },
                )
                .map_err(|error| caller_failure(&self.diagnostic_identity, error, None))?;
            for (path, bytes) in snapshot.into_files() {
                total_bytes = total_bytes.saturating_add(bytes.len());
                if total_bytes > 256 * 1024 * 1024 || report.documents >= 64_000 {
                    return Err(fail(std::io::Error::other(
                        "caller document or byte limit exceeded",
                    )));
                }
                let text = std::str::from_utf8(&bytes)
                    .map_err(|error| caller_failure(&self.diagnostic_identity, error, None))?;
                let found = catalog
                    .catalog
                    .check_caller(&root, &caller_root.join(path), text);
                report.documents += found.documents;
                report.references += found.references;
                report.issues.extend(found.issues);
                if report.issues.len() >= 256 {
                    return Err(caller_failure(
                        &self.diagnostic_identity,
                        std::io::Error::other(
                            "caller issue limit reached; correct these and rerun",
                        ),
                        Some(&report),
                    ));
                }
            }
        }
        if !report.issues.is_empty() {
            return Err(caller_failure(
                &self.diagnostic_identity,
                std::io::Error::other("incoming Package references failed"),
                Some(&report),
            ));
        }
        let mut result = if request.write {
            self.write_package_indexes(request.root, catalog)?
        } else {
            catalog
        };
        if !request.caller_roots.is_empty() {
            let mut value = serde_json::to_value(&result.catalog)
                .map_err(|error| super::projection_failure(&self.diagnostic_identity, error))?;
            value["callers"] = serde_json::to_value(report)
                .map_err(|error| super::projection_failure(&self.diagnostic_identity, error))?;
            result.canonical_bytes = pale_core::canon::to_canonical_string(&value)
                .map_err(|error| super::projection_failure(&self.diagnostic_identity, error))?
                .into_bytes();
        }
        Ok(result)
    }
}

fn formatting_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    issues: &[pale_application::packages::PackageFormattingIssue],
) -> ApiError {
    let mut facts = vec![package_fact("issues", &issues.len().to_string())];
    for (index, issue) in issues.iter().take(32).enumerate() {
        facts.push(package_fact(
            &format!("paragraph_{index}"),
            &format!("{}:{}", issue.source.display(), issue.line),
        ));
    }
    ApiError::new_with_opaque_source(
        identity,
        std::io::Error::other("wrapped Package paragraphs"),
        FailureSpecification {
            code: "package_formatting_rejected",
            stage: DiagnosticStageV1::InputAdmission,
            disposition: DiagnosticDispositionV1::InvalidInput,
            summary: "Package prose needs one paragraph per source line; no indexes were written",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::AfterCorrection,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: vec!["Run the Markdown formatter with proseWrap: never on the reported files, then rerun packages check."],
            facts,
        },
    )
}

fn caller_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: impl std::error::Error + Send + Sync + 'static,
    report: Option<&PackageCallerReport>,
) -> ApiError {
    let mut facts = vec![package_fact("reason", &error.to_string())];
    if let Some(report) = report {
        facts.push(package_fact("issues", &report.issues.len().to_string()));
        for (index, issue) in report.issues.iter().take(32).enumerate() {
            let location = format!(
                "{}:{}: {:?}",
                issue.source.display(),
                issue.line,
                issue.kind
            );
            facts.push(package_fact(&format!("caller_{index}"), &location));
        }
    }
    ApiError::new_with_opaque_source(identity, error, FailureSpecification {
        code: "package_callers_rejected",
        stage: DiagnosticStageV1::InputAdmission,
        disposition: DiagnosticDispositionV1::InvalidInput,
        summary: "Package caller check failed; no indexes were written",
        subject: None, schema_coordinate: None, location: None,
        retry: DiagnosticRetryV1::AfterCorrection,
        effect: DiagnosticEffectCertaintyV1::NotApplicable,
        request: None, notes: Vec::new(),
        help: vec!["Correct the caller locations or explicit capture scope, then rerun packages check."],
        facts,
    })
}

fn normalized(path: &std::path::Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}
