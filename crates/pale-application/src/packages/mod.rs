//! Read the Packages a workspace selects, check them against their manifests and
//! seal them into one graph that later steps use without the source files.
//!
//! This module builds on `compile`, `markdown` and `workspace`; `compile`,
//! `render`, `resident`, `semantic` and `workspace` build on it.
//!
//! Compilation captures one bounded distribution byte set, admits exact
//! Package releases and their dependency closure, qualifies authored links,
//! closes explicitly selected occurrences, and returns a source-free
//! [`SealedPackageGraph`]. There is no distribution-global family loader or
//! path-convention ownership. Capture is descriptor-relative and race-resistant
//! but is not a filesystem-wide atomic transaction. Bounded source reads fail
//! closed when the host lacks required no-follow filesystem primitives.

mod authoring;
mod catalog;
mod distribution;
mod errors;
mod graph;
mod inventory;
mod page;
mod release;
mod snapshot;
mod source_reader;
mod text;
#[cfg(feature = "bench")]
#[doc(hidden)]
pub mod text_benchmark;

pub(crate) use catalog::read_snapshot as read_catalog_snapshot;
pub use catalog::{
    read_catalog, CatalogIndex, CatalogPackage, CatalogResource, PackageCallerIssue,
    PackageCallerIssueKind, PackageCallerReport, PackageCatalog, PackageFormattingIssue,
};
pub(crate) use distribution::inspect_snapshot;
pub use distribution::{inspect_distribution, DistributionInfo};
pub(crate) use errors::PackageDiagnosticAccumulator;
pub use errors::{
    BodyError, MarkdownLinkViolation, OperationProcedureViolation, PackageDiagnostic,
    PackageDiagnosticCode, PackageDiagnostics, PackageError, PackageGraphInvariant,
    ResourceLinkRelation, ResourceLinkViolation, ResourceSelectorViolation,
    MAX_PACKAGE_DIAGNOSTICS,
};

pub(crate) use graph::{
    AdmittedMarkdown, AdmittedOperationRequirement, AdmittedPackageGraph, AdmittedResourceNode,
    PackageResource, RepositoryPackageSelection,
};
pub use graph::{PackageResourceUse, SealedPackageGraph};
pub use page::PageHeaderError;
pub(crate) use page::{
    declaration_text as page_declaration_text, header_range as page_header_range, split_page,
};
pub(crate) use release::{PackageReleaseIndex, ResourceSet, RetainedPackageRelease};
pub(crate) use snapshot::{
    DistributionSnapshot, MAX_DEPTH as MAX_PACKAGE_SNAPSHOT_DEPTH,
    MAX_ENTRIES as MAX_PACKAGE_SNAPSHOT_ENTRIES, MAX_FILE_BYTES as MAX_PACKAGE_SNAPSHOT_FILE_BYTES,
    MAX_TOTAL_BYTES as MAX_PACKAGE_SNAPSHOT_TOTAL_BYTES,
};
pub(crate) use text::{
    seal_texts, ResourceTextLink, ResourceTextReference, TextRecipe, TextRecipeBuilder, TextRef,
};
