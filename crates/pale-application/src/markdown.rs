//! Pale's rules for reading and adjusting Markdown documents, built on the
//! Comrak `CommonMark` parser.
//!
//! `packages` and `render` build on it, and it uses `packages` types in return.
//! Comrak owns `CommonMark` block recognition and source positions. This module
//! keeps Pale policy for document titles and permitted heading transformations
//! without exposing Comrak types to semantic or rendering code. Transformations
//! patch only source bytes selected by the AST;
//! they do not normalize or re-render an authored document.

use comrak::{
    nodes::{NodeHeading, NodeValue},
    parse_document, Arena, Options,
};

use crate::packages::{ResourceTextLink, TextRecipeBuilder, TextRef};

/// Paragraph starts containing a soft line break, excluding code, maths, tables and
/// front matter. Explicit Markdown hard breaks retain their intended layout.
pub(crate) fn wrapped_paragraphs(markdown: &str) -> Vec<usize> {
    let arena = Arena::new();
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.math_dollars = true;
    options.extension.math_code = true;
    options.extension.front_matter_delimiter = Some(
        if markdown.starts_with("+++\n") || markdown.starts_with("+++\r\n") {
            "+++"
        } else {
            "---"
        }
        .to_owned(),
    );
    let root = parse_document(&arena, markdown, &options);
    root.descendants()
        .filter(|node| matches!(node.data().value, NodeValue::Paragraph))
        .filter(|node| {
            node.descendants()
                .any(|child| matches!(child.data().value, NodeValue::SoftBreak))
        })
        .map(|node| node.data().sourcepos.start.line)
        .collect()
}

/// Pale-owned structural projection of one admitted Markdown document.
///
/// The projection stores byte coordinates and plain Rust values only; Comrak's
/// arena and node types never cross this module. Consumers can therefore reuse
/// the exact title/body/heading interpretation without reparsing source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkdownProjection {
    content_end: usize,
    title_span: Option<(usize, usize)>,
    body_start: usize,
    atx_headings: Vec<AtxHeadingSpan>,
    inline_links: Vec<MarkdownLinkSpan>,
    heading_ids: std::collections::BTreeSet<String>,
    inline_code: Vec<(usize, String)>,
}

/// An AST-recognized link and its exact inline destination, when authored inline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkdownLinkSpan {
    pub(crate) destination: String,
    pub(crate) line: usize,
    pub(crate) range: Option<std::ops::Range<usize>>,
    pub(crate) link_range: Option<std::ops::Range<usize>>,
}

/// One structurally admitted ATX marker in exact source-byte coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AtxHeadingSpan {
    start: usize,
    width: usize,
}

impl MarkdownProjection {
    /// Check a URL fragment against parsed headings and explicit anchor elements.
    pub(crate) fn has_fragment(&self, fragment: &str) -> bool {
        if fragment.is_empty() {
            return true;
        }
        let mut decoded = Vec::with_capacity(fragment.len());
        let mut bytes = fragment.bytes();
        while let Some(byte) = bytes.next() {
            if byte == b'%' {
                let Some(high) = bytes
                    .next()
                    .and_then(|value| char::from(value).to_digit(16))
                else {
                    return false;
                };
                let Some(low) = bytes
                    .next()
                    .and_then(|value| char::from(value).to_digit(16))
                else {
                    return false;
                };
                decoded.push((high * 16 + low) as u8);
            } else {
                decoded.push(byte);
            }
        }
        std::str::from_utf8(&decoded).is_ok_and(|id| self.heading_ids.contains(id))
    }

    pub(crate) fn inline_code(&self) -> &[(usize, String)] {
        &self.inline_code
    }

    pub(crate) fn links(&self) -> &[MarkdownLinkSpan] {
        &self.inline_links
    }

    /// Compose qualified destination edits with the same parsed heading spans.
    pub(crate) fn append_linked_recipe(
        &self,
        markdown: &TextRef,
        range: std::ops::Range<usize>,
        levels: usize,
        links: &[ResourceTextLink],
        target: &mut TextRecipeBuilder,
    ) {
        if links.is_empty() && range == self.body_range() {
            self.append_shifted_recipe(markdown, levels, target);
            return;
        }
        let mut edits = Vec::new();
        if levels > 0 {
            for heading in &self.atx_headings {
                if range.contains(&heading.start) {
                    edits.push((
                        heading.start,
                        heading.start + heading.width,
                        None,
                        heading.width.saturating_add(levels).min(6),
                    ));
                }
            }
        }
        for link in links {
            if range.contains(&link.range.start) {
                edits.push((link.range.start, link.range.end, Some(link), 0));
            }
        }
        edits.sort_by_key(|edit| edit.0);
        let mut cursor = range.start;
        for (start, end, link, width) in edits {
            assert!(start >= cursor && end <= range.end);
            target.push_ref(markdown.slice(cursor..start).expect("admitted edit span"));
            if let Some(link) = link {
                target.push_resource_link(
                    markdown.slice(start..end).expect("admitted link span"),
                    link,
                );
            } else {
                target.push_hashes(u8::try_from(width).expect("bounded heading level"));
            }
            cursor = end;
        }
        target.push_ref(
            markdown
                .slice(cursor..range.end)
                .expect("admitted trailing span"),
        );
    }
    /// Authored title selected by the Pale top-level ATX policy.
    pub(crate) fn title<'a>(&self, markdown: &'a str) -> Option<&'a str> {
        self.title_span.map(|(start, end)| &markdown[start..end])
    }

    /// Exact full-document byte range occupied by the admitted body.
    pub(crate) fn body_range(&self) -> std::ops::Range<usize> {
        self.body_start..self.content_end
    }

    /// Append a heading-shifted body as immutable spans and explicit markers.
    ///
    /// Untouched source ranges remain references into the admitted text store;
    /// only replacement ATX markers become compiler-owned recipe parts.
    pub(crate) fn append_shifted_recipe(
        &self,
        markdown: &TextRef,
        levels: usize,
        target: &mut TextRecipeBuilder,
    ) {
        if self.body_start == self.content_end {
            return;
        }
        if levels == 0 {
            target.push_ref(
                markdown
                    .slice(self.body_range())
                    .expect("projection spans derive from the same admitted text"),
            );
            return;
        }
        let mut cursor = self.body_start;
        for heading in self
            .atx_headings
            .iter()
            .filter(|heading| heading.start >= self.body_start)
        {
            target.push_ref(
                markdown
                    .slice(cursor..heading.start)
                    .expect("heading spans derive from the same admitted text"),
            );
            let width = heading.width.saturating_add(levels).min(6);
            target.push_hashes(u8::try_from(width).expect("ATX levels are bounded by six"));
            cursor = heading.start + heading.width;
        }
        target.push_ref(
            markdown
                .slice(cursor..self.content_end)
                .expect("projection spans derive from the same admitted text"),
        );
    }
}

/// Return the byte offset for a one-based Comrak source position.
///
/// Comrak reports UTF-8 byte columns by default. Keeping that default makes a
/// source position directly usable as a Rust string byte offset, including on
/// lines containing non-ASCII text before a nested heading.
fn byte_offset(line_starts: &[usize], line: usize, column: usize) -> Option<usize> {
    line.checked_sub(1)
        .and_then(|index| line_starts.get(index))
        .and_then(|start| column.checked_sub(1).map(|column| start + column))
}

/// Yield each `CommonMark` line terminator and the following line's byte offset.
///
/// A CRLF pair is one terminator; lone carriage returns and line feeds are
/// independent terminators. This mirrors the line model behind Comrak source
/// positions without normalizing the authored bytes.
fn line_boundaries(markdown: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
    let bytes = markdown.as_bytes();
    let mut cursor = 0;
    std::iter::from_fn(move || {
        while cursor < bytes.len() {
            let terminator = cursor;
            match bytes[cursor] {
                b'\r' => {
                    cursor += 1;
                    if bytes.get(cursor) == Some(&b'\n') {
                        cursor += 1;
                    }
                    return Some((terminator, cursor));
                }
                b'\n' => {
                    cursor += 1;
                    return Some((terminator, cursor));
                }
                _ => cursor += 1,
            }
        }
        None
    })
}

fn line_starts(markdown: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(line_boundaries(markdown).map(|(_, next_start)| next_start))
        .collect()
}

fn skip_blank_lines(markdown: &str) -> usize {
    let mut consumed = 0;
    for (line_end, next_start) in line_boundaries(markdown) {
        if !markdown[consumed..line_end]
            .bytes()
            .all(|byte| matches!(byte, b' ' | b'\t'))
        {
            break;
        }
        consumed = next_start;
    }
    consumed
}

/// Locate authored closing questions without treating fenced examples as instructions.
/// The span includes the heading and its immediately following checklist, leaving
/// later host instructions available for placement before the questions.
pub(crate) fn closing_questions(markdown: &str) -> Vec<ClosingQuestions> {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &Options::default());
    let starts = line_starts(markdown);
    root.children()
        .filter_map(|node| {
            if !matches!(node.data().value, NodeValue::Heading(_))
                || node.collect_text() != "Before handing off"
            {
                return None;
            }
            let list = node
                .next_sibling()
                .filter(|next| matches!(next.data().value, NodeValue::List(_)));
            let items = list
                .into_iter()
                .flat_map(|list| list.children())
                .collect::<Vec<_>>();
            let count = if items.iter().any(|item| {
                let position = item.data().sourcepos.start;
                byte_offset(&starts, position.line, position.column)
                    .and_then(|offset| markdown.get(offset..))
                    .is_none_or(|text| !text.starts_with("- [ ] "))
            }) {
                0
            } else {
                items.len()
            };
            let start = node.data().sourcepos.start;
            let end = list.unwrap_or(node).data().sourcepos.end;
            Some(ClosingQuestions {
                range: byte_offset(&starts, start.line, start.column)?
                    ..byte_offset(&starts, end.line, end.column)?.checked_add(1)?,
                count,
            })
        })
        .collect()
}

/// A closing section's exact bytes and number of unchecked questions.
pub(crate) struct ClosingQuestions {
    pub(crate) range: std::ops::Range<usize>,
    pub(crate) count: usize,
}

/// Locate an inline destination inside a link the syntax adapter already proved.
/// Reference-style links retain no guessed edit span and are handled explicitly
/// by resource admission. Code examples never arrive as link AST nodes.
fn inline_destination(raw: &str, label_end: usize) -> Option<std::ops::Range<usize>> {
    // Child positions exclude the destination and title. Start after the last
    // label child so literal `](` in a label or title cannot choose the edit.
    let mut start = raw.get(label_end..)?.find("](")? + label_end + 2;
    let bytes = raw.as_bytes();
    while bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
        start += 1;
    }
    if bytes.get(start) == Some(&b'<') {
        start += 1;
        let end = raw.get(start..)?.find('>')? + start;
        return Some(start..end);
    }
    let mut cursor = start;
    let mut depth = 0usize;
    while let Some(byte) = bytes.get(cursor) {
        match byte {
            b'\\' => {
                cursor += 2;
                continue;
            }
            b'(' => depth += 1,
            b')' if depth == 0 => return Some(start..cursor),
            b')' => depth -= 1,
            byte if byte.is_ascii_whitespace() => return Some(start..cursor),
            _ => {}
        }
        cursor += 1;
    }
    None
}

/// Serialize a semantic URL into an existing inline destination span. Entity
/// escaping works in both bare and angle destinations without changing the URL.
pub(crate) fn escape_link_destination(destination: &str) -> String {
    let mut escaped = String::with_capacity(destination.len());
    for character in destination.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\\' | '(' | ')' => {
                escaped.push('\\');
                escaped.push(character);
            }
            character if character.is_ascii_whitespace() => {
                use std::fmt::Write as _;
                write!(&mut escaped, "&#{};", u32::from(character)).expect("write to String");
            }
            character => escaped.push(character),
        }
    }
    escaped
}

/// Extract the raw authored title from a Pale top-level ATX heading.
///
/// Pale deliberately requires the narrow `# ` title spelling. Comrak decides
/// whether the line is structurally an ATX level-one heading and whether final
/// hashes are a closing sequence; Pale retains the lexical title policy and
/// returns a slice of the original bytes.
fn atx_title(line: &str, heading: NodeHeading) -> Option<&str> {
    let title = line.strip_prefix("# ")?.trim();
    if heading.closed {
        Some(title.trim_end_matches('#').trim_end())
    } else {
        Some(title)
    }
}

/// Parse one Markdown document into Pale-owned structural coordinates.
///
/// This is the admission entry point for Package bodies. Callers retain the
/// returned projection beside exact source text and reuse it for every render;
/// the syntax adapter is not revisited after source custody is dropped.
pub(crate) fn project(markdown: &str) -> MarkdownProjection {
    let content_start = skip_blank_lines(markdown);
    let trimmed = markdown[content_start..].trim_end();
    let content_end = content_start + trimmed.len();
    let arena = Arena::new();
    let root = parse_document(&arena, trimmed, &Options::default());
    let starts = line_starts(trimmed);
    let mut atx_headings = Vec::new();
    let mut inline_links = Vec::new();
    let mut inline_code = Vec::new();
    let leading_lines = line_boundaries(&markdown[..content_start]).count();
    let mut anchorizer = comrak::Anchorizer::new();
    let mut heading_ids = std::collections::BTreeSet::new();
    for node in root.descendants() {
        let data = node.data();
        if let NodeValue::Code(code) = &data.value {
            inline_code.push((
                leading_lines + data.sourcepos.start.line,
                code.literal.clone(),
            ));
        }
        let html = match &data.value {
            NodeValue::HtmlInline(html) => Some(html.as_str()),
            NodeValue::HtmlBlock(html) if html.block_type == 7 => Some(html.literal.as_str()),
            _ => None,
        };
        if let Some(html) = html {
            // Package sources use empty anchor elements for stable section links.
            // Inspect only HTML nodes, so examples and comments cannot create targets.
            for line in html.lines() {
                if let Some(anchor) = line.trim().strip_prefix("<a id=\"") {
                    if let Some((id, tail)) = anchor.split_once('"') {
                        if matches!(tail, ">" | "></a>") {
                            heading_ids.insert(id.to_owned());
                        }
                    }
                }
            }
        }
        if let NodeValue::Link(link) = &data.value {
            let mut link_range = None;
            let range = byte_offset(
                &starts,
                data.sourcepos.start.line,
                data.sourcepos.start.column,
            )
            .zip(byte_offset(
                &starts,
                data.sourcepos.end.line,
                data.sourcepos.end.column,
            ))
            .and_then(|(start, end)| {
                let raw = trimmed.get(start..end.checked_add(1)?)?;
                let label_end = node
                    .last_child()
                    .and_then(|child| {
                        let end = child.data().sourcepos.end;
                        byte_offset(&starts, end.line, end.column)
                            .and_then(|end| end.checked_add(1)?.checked_sub(start))
                    })
                    .unwrap_or(0);
                let relative = inline_destination(raw, label_end)?;
                link_range = Some(content_start + start..content_start + end + 1);
                Some(content_start + start + relative.start..content_start + start + relative.end)
            });
            inline_links.push(MarkdownLinkSpan {
                destination: link.url.clone(),
                line: leading_lines + data.sourcepos.start.line,
                range,
                link_range,
            });
        }
        let NodeValue::Heading(heading) = data.value else {
            continue;
        };
        heading_ids.insert(anchorizer.anchorize(&node.collect_text()));
        if heading.setext {
            continue;
        }
        if let Some(start) = byte_offset(
            &starts,
            data.sourcepos.start.line,
            data.sourcepos.start.column,
        ) {
            let width = usize::from(heading.level);
            if trimmed
                .as_bytes()
                .get(start..start.saturating_add(width))
                .is_some_and(|bytes| bytes.iter().all(|byte| *byte == b'#'))
            {
                atx_headings.push(AtxHeadingSpan {
                    start: content_start + start,
                    width,
                });
            }
        }
    }

    let title_projection = root.first_child().and_then(|first| {
        let heading = match first.data().value {
            NodeValue::Heading(heading) if heading.level == 1 && !heading.setext => heading,
            _ => return None,
        };
        let (line, body_start) = line_boundaries(trimmed)
            .next()
            .map_or((trimmed, trimmed.len()), |(line_end, body_start)| {
                (&trimmed[..line_end], body_start)
            });
        let title = atx_title(line.trim_start_matches([' ', '\t']), heading)?;
        let title_start = title.as_ptr() as usize - markdown.as_ptr() as usize;
        let title_end = title_start + title.len();
        let body_start = content_start + body_start + skip_blank_lines(&trimmed[body_start..]);
        Some(((title_start, title_end), body_start))
    });
    let (title_span, body_start) = title_projection
        .map(|(title_span, body_start)| (Some(title_span), body_start))
        .unwrap_or((None, content_start));
    MarkdownProjection {
        content_end,
        title_span,
        body_start,
        atx_headings,
        inline_links,
        heading_ids,
        inline_code,
    }
}

#[cfg(test)]
#[path = "../tests/internal/markdown.rs"]
mod tests;
