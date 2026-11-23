//! Limits and failures for the documents handed to a one-shot compilation.
//!
//! `compile` and `workspace` build on these types; they build on nothing else in
//! this crate. The types classify bounded logical documents only. They grant no
//! filesystem authority and do not replace the typed workspace or Package
//! admission errors raised after source-shaped bytes have crossed this ingress.

/// Finite resource ceiling enforced before captured bytes enter compilation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CapturedInputLimit {
    /// Package files plus inferred non-root directory ancestors.
    #[error("Package entry count")]
    PackageEntries,
    /// Greatest inferred Package directory depth beneath the logical root.
    #[error("Package directory depth")]
    PackageDepth,
    /// Bytes in one captured Package file.
    #[error("Package file bytes")]
    PackageFileBytes,
    /// Aggregate bytes in all captured Package files.
    #[error("Package total bytes")]
    PackageTotalBytes,
    /// Bytes in one private TOML or lifecycle-guidance document.
    #[error("private document bytes")]
    PrivateFileBytes,
    /// Singular private records plus unique lifecycle-guidance documents.
    #[error("private document count")]
    PrivateDocuments,
    /// Aggregate bytes in all private records and lifecycle guidance.
    #[error("private total bytes")]
    PrivateTotalBytes,
}

/// Rejection of an owned logical document bundle before semantic admission.
///
/// Labels in duplicate and guidance-set errors have already passed the confined
/// UTF-8 path grammar. Raw document bytes are never retained by this error.
#[derive(Debug, thiserror::Error)]
pub enum CapturedCompilationInputError {
    /// One supplied label is not a bounded confined UTF-8 relative path.
    #[error("captured {kind} label is outside the confined logical-path grammar")]
    InvalidLogicalLabel {
        /// Input family whose label was rejected.
        kind: &'static str,
    },
    /// Two members claim the same admitted logical label.
    #[error("captured {kind} repeats logical label `{label}`")]
    DuplicateLogicalLabel {
        /// Input family containing the duplicate.
        kind: &'static str,
        /// Exact already-confined duplicate label.
        label: String,
    },
    /// One finite ingress dimension exceeds its existing compiler ceiling.
    #[error("captured input exceeds {limit}: {observed} > {maximum}")]
    Limit {
        /// Dimension whose bound was exceeded.
        limit: CapturedInputLimit,
        /// Exact observed value, or the first value proven to exceed the bound.
        observed: u64,
        /// Maximum admitted value.
        maximum: u64,
    },
    /// Captured guidance labels differ from the uniquely referenced label set.
    #[error(
        "captured lifecycle guidance differs from declared references (missing: {missing:?}; unexpected: {unexpected:?})"
    )]
    LifecycleGuidanceMismatch {
        /// Referenced labels absent from the captured documents.
        missing: Vec<String>,
        /// Captured labels referenced by no lifecycle behavior.
        unexpected: Vec<String>,
    },
}
