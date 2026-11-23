//! Read-only formatting checks on the authored bodies used by a Package.

use std::collections::BTreeSet;
use std::path::PathBuf;

use super::{DistributionSnapshot, PackageCatalog, PackageError};

/// A paragraph that needs the author's Markdown formatter.
#[derive(Debug)]
pub struct PackageFormattingIssue {
    /// Source path relative to the distribution root.
    pub source: PathBuf,
    /// One-based source line where the wrapped paragraph starts.
    pub line: usize,
}

pub(super) fn check(
    snapshot: &DistributionSnapshot,
    catalog: &PackageCatalog,
) -> Result<Vec<PackageFormattingIssue>, PackageError> {
    let paths = catalog
        .resources
        .iter()
        .filter(|resource| resource.text.is_some())
        .filter_map(|resource| catalog.source_path(resource))
        .collect::<BTreeSet<_>>();
    let mut issues = Vec::new();
    for path in paths {
        let text = snapshot.text(&path)?;
        issues.extend(
            crate::markdown::wrapped_paragraphs(text)
                .into_iter()
                .map(|line| PackageFormattingIssue {
                    source: PathBuf::from("packages").join(&path),
                    line,
                }),
        );
    }
    Ok(issues)
}
