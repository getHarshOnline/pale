//! Incoming references checked against the same captured Package sources as compilation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use pale_core::records::{PackageIdV1, PackageResourceIdV1};
use serde::Serialize;

use crate::markdown::{self, MarkdownProjection};
use crate::packages::{DistributionSnapshot, PackageError};

use super::PackageCatalog;

/// Why a caller's reference cannot reach the captured Package library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageCallerIssueKind {
    /// The linked local file is absent from the Package snapshot.
    MissingFile,
    /// The target exists but does not contain this heading or explicit anchor.
    MissingAnchor,
    /// A qualified ID in inline code names no declared resource use.
    UnknownResource,
}

/// One actionable source location, without copying a potentially private URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageCallerIssue {
    /// Caller path supplied by the capture owner.
    pub source: PathBuf,
    /// One-based Markdown line containing the reference.
    pub line: usize,
    /// The failed reference check.
    pub kind: PackageCallerIssueKind,
}

/// Result of checking caller text against an already admitted Package catalog.
#[derive(Debug, Default, Serialize)]
pub struct PackageCallerReport {
    /// Number of complete Markdown documents checked.
    pub documents: usize,
    /// Number of incoming links and qualified inline IDs checked.
    pub references: usize,
    /// Failed references in document order.
    pub issues: Vec<PackageCallerIssue>,
}

impl PackageCatalog {
    /// Check one caller's local links and qualified IDs using captured targets.
    ///
    /// `distribution_root` and `source` must be absolute logical paths in the
    /// same namespace. Generated index targets use their proposed replacement;
    /// other targets use captured source bytes. This performs no filesystem reads. Relative links resolve
    /// from the caller; links outside `packages/` are outside this check. Inline
    /// and reference-style Markdown links use the compiler's syntax reader.
    /// Qualified IDs have the form `namespace.package:resource/path`; code blocks
    /// and ordinary unqualified code words are not resource declarations.
    #[must_use]
    pub fn check_caller(
        &self,
        distribution_root: &Path,
        source: &Path,
        markdown: &str,
    ) -> PackageCallerReport {
        let package_root = normalize(&distribution_root.join("packages"));
        let projection = markdown::project(markdown);
        let mut report = PackageCallerReport {
            documents: 1,
            ..Default::default()
        };
        for link in projection.links() {
            let (destination, fragment) = link
                .destination
                .split_once('#')
                .map_or((link.destination.as_str(), None), |(path, anchor)| {
                    (path, Some(anchor))
                });
            let destination = destination
                .split_once('?')
                .map_or(destination, |(path, _)| path);
            let destination = destination.strip_prefix("file://").unwrap_or(destination);
            if destination.is_empty()
                || destination.starts_with("//")
                || destination.contains("://")
            {
                continue;
            }
            let decoded = destination
                .split('/')
                .map(|part| decode_path(part).unwrap_or_else(|| part.to_owned()))
                .collect::<Vec<_>>()
                .join("/");
            let path = Path::new(&decoded);
            let target = normalize(&if path.is_absolute() {
                path.to_path_buf()
            } else {
                source.parent().unwrap_or(Path::new("")).join(path)
            });
            let Ok(relative) = target.strip_prefix(&package_root) else {
                continue;
            };
            report.references += 1;
            let kind = match self.incoming_targets.get(relative) {
                None => Some(PackageCallerIssueKind::MissingFile),
                Some(projection)
                    if fragment.is_some_and(|anchor| {
                        !anchor.is_empty()
                            && projection.as_ref().is_none_or(|p| !p.has_fragment(anchor))
                    }) =>
                {
                    Some(PackageCallerIssueKind::MissingAnchor)
                }
                _ => None,
            };
            if let Some(kind) = kind {
                report.issues.push(PackageCallerIssue {
                    source: source.to_path_buf(),
                    line: link.line,
                    kind,
                });
            }
        }
        let resources = self
            .resources
            .iter()
            .map(|resource| format!("{}:{}", resource.package, resource.id))
            .collect::<BTreeSet<_>>();
        for (line, code) in projection.inline_code() {
            if !looks_like_resource(code, self) {
                continue;
            }
            report.references += 1;
            if !resources.contains(code) {
                report.issues.push(PackageCallerIssue {
                    source: source.to_path_buf(),
                    line: *line,
                    kind: PackageCallerIssueKind::UnknownResource,
                });
            }
        }
        report.issues.sort_by_key(|issue| issue.line);
        report
    }
}

fn looks_like_resource(code: &str, catalog: &PackageCatalog) -> bool {
    let Some((package, resource)) = code.split_once(':') else {
        return false;
    };
    (package.contains('.')
        || catalog
            .packages
            .iter()
            .any(|entry| entry.manifest.package.id.as_str() == package))
        && PackageIdV1::parse(package).is_ok()
        && PackageResourceIdV1::parse(resource).is_ok()
}

fn decode_path(value: &str) -> Option<String> {
    let mut result = Vec::with_capacity(value.len());
    let mut bytes = value.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = char::from(bytes.next()?).to_digit(16)?;
            let low = char::from(bytes.next()?).to_digit(16)?;
            result.push((high * 16 + low) as u8);
        } else {
            result.push(byte);
        }
    }
    String::from_utf8(result).ok()
}

fn normalize(path: &Path) -> PathBuf {
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

pub(super) fn targets(
    snapshot: &DistributionSnapshot,
) -> Result<BTreeMap<PathBuf, Option<MarkdownProjection>>, PackageError> {
    let mut targets = BTreeMap::new();
    for path in snapshot.files_under(Path::new("")) {
        let projection = if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("mdx"))
        {
            Some(markdown::project(snapshot.text(path)?))
        } else {
            None
        };
        targets.insert(path.to_path_buf(), projection);
        for parent in path.ancestors().skip(1) {
            targets.entry(parent.to_path_buf()).or_insert(None);
        }
    }
    Ok(targets)
}
