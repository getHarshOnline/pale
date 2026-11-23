//! Locate a Package page's declaration without rewriting its Markdown body.

use std::collections::BTreeSet;
use std::ops::Range;
use std::path::Path;

use pale_contracts::CompiledSchema;
use pale_core::records::{PackagePageV1, PackageResourceIdV1};

use super::source_reader::parse_source_with_schema;
use super::PackageError;

/// A declaration is small even when the Package page contains a long method.
const MAX_HEADER_BYTES: usize = 64 * 1024;

/// Why a page cannot be divided into its declaration and authored body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PageHeaderError {
    /// The first source line is not the opening delimiter.
    #[error("the first line must be +++")]
    MissingOpening,
    /// No closing delimiter follows the opening line.
    #[error("the declaration has no closing +++ line")]
    MissingClosing,
    /// The declaration exceeds the reader's bounded header size.
    #[error("the declaration exceeds 64 KiB")]
    TooLarge,
    /// A declaration must accompany an authored Markdown body.
    #[error("the page has no Markdown body")]
    EmptyBody,
}

/// Original source coordinates, shared by the semantic reader and editor.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PageParts {
    pub(crate) declaration: Range<usize>,
    pub(crate) markdown: Range<usize>,
}

/// One typed declaration and its coordinates in the retained source.
pub(super) struct PageSource {
    pub(super) header: PackagePageV1,
    pub(super) parts: PageParts,
}

/// Find the nearest authored topic above a page, within the same Package.
/// `motion/feedback` reads `motion`; a deeper page first looks for its own
/// folder's topic. The caller still proves that the target declares guide use.
pub(super) fn parent_topic<'a>(
    page: &PackageResourceIdV1,
    pages: &'a BTreeSet<PackageResourceIdV1>,
) -> Option<&'a PackageResourceIdV1> {
    let mut path = page.as_str();
    while let Some((parent, _)) = path.rsplit_once('/') {
        let candidate = PackageResourceIdV1::parse(parent)
            .expect("an admitted resource path has admitted nonempty prefixes");
        if let Some(topic) = pages.get(&candidate) {
            return Some(topic);
        }
        path = parent;
    }
    None
}

/// Read the declaration through the same pinned-schema path as Package data.
pub(super) fn read_page(text: &str, path: &Path) -> Result<PageSource, PackageError> {
    let parts = split_page(text).map_err(|reason| PackageError::PageHeader {
        path: path.to_path_buf(),
        reason,
    })?;
    let schema = CompiledSchema::for_id("package-page.v1")?;
    // A same-width TOML comment keeps syntax diagnostics in original file
    // coordinates without letting Markdown enter the declaration parser.
    let declaration = declaration_text(text, &parts.declaration);
    let header = parse_source_with_schema(&declaration, path, &schema, "Package page")?;
    Ok(PageSource { header, parts })
}

/// The first line must open the declaration; a later `+++` belongs to the body.
/// TOML validation consumes only `declaration`, and Markdown consumes only
/// `markdown`. Both ranges retain the original line endings and byte positions.
pub(crate) fn split_page(source: &str) -> Result<PageParts, PageHeaderError> {
    let (declaration, body_start) = locate_header(source)?;
    let body_start = body_start.ok_or(PageHeaderError::MissingClosing)?;
    if source[body_start..].trim().is_empty() {
        return Err(PageHeaderError::EmptyBody);
    }
    Ok(PageParts {
        declaration,
        markdown: body_start..source.len(),
    })
}

/// Tolerant authoring can inspect an unfinished header, within the same bound.
/// A missing opening or an oversized header supplies no TOML region.
pub(crate) fn header_range(source: &str) -> Option<Range<usize>> {
    locate_header(source)
        .ok()
        .map(|(declaration, _)| declaration)
}

/// Preserve source positions while excluding the delimiters and Markdown.
pub(crate) fn declaration_text(source: &str, declaration: &Range<usize>) -> String {
    let mut text = String::from("#  ");
    text.push_str(&source[3..declaration.end]);
    text
}

fn locate_header(source: &str) -> Result<(Range<usize>, Option<usize>), PageHeaderError> {
    let mut lines = source.split_inclusive('\n');
    let opening = lines.next().ok_or(PageHeaderError::MissingOpening)?;
    if !matches!(opening, "+++\n" | "+++\r\n") {
        return Err(PageHeaderError::MissingOpening);
    }
    let start = opening.len();
    let mut offset = start;
    for line in lines {
        if matches!(line, "+++\n" | "+++\r\n" | "+++") {
            return Ok((start..offset, Some(offset + line.len())));
        }
        offset += line.len();
        if offset - start > MAX_HEADER_BYTES {
            return Err(PageHeaderError::TooLarge);
        }
    }
    Ok((start..offset, None))
}

#[cfg(test)]
#[path = "../../tests/internal/packages/page.rs"]
mod tests;
