//! Tolerant, source-local TOML structure for editor coordinates.
//!
//! This module deliberately stops before semantic interpretation. The
//! recovering [`toml_parser`] event stream supplies only decoded keys, source
//! spans, and structural paths. Generated grammar facts and admitted semantic
//! identities remain owned by [`pale_sdk`].

use std::collections::{BTreeMap, BTreeSet};

use pale_sdk::{AuthoringInstancePathSegment, AuthoringPathSegment, AUTHORING_PATH_SEGMENT_LIMIT};
use toml_parser::parser::{parse_document, Event, EventKind, RecursionGuard};
use toml_parser::{ParseError, Source};

/// Maximum structural facts retained from one tolerant TOML buffer.
pub(super) const MAX_AUTHORING_SYNTAX_FACTS: usize = 16_384;

/// Half-open UTF-8 byte range produced by the selected TOML parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ByteRange {
    pub(super) start: usize,
    pub(super) end: usize,
}

impl ByteRange {
    pub(super) const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    const fn contains(self, byte: usize) -> bool {
        (self.start == self.end && byte == self.start) || (self.start <= byte && byte < self.end)
    }

    const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

/// Source-local role of one recoverable TOML span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SyntaxFactKind {
    /// One decoded key segment.
    Key,
    /// One decoded table-header key segment.
    HeaderKey,
    /// One authored scalar value.
    Value,
    /// The value position following `=`, including an incomplete value.
    ValueSlot,
    /// One standard table header.
    Table,
    /// One array-of-tables header.
    ArrayTable,
}

/// One structural source fact without semantic meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SyntaxFact {
    pub(super) kind: SyntaxFactKind,
    pub(super) range: ByteRange,
    pub(super) path: Vec<AuthoringPathSegment>,
    /// Exact instance path used only against linker-owned semantic evidence.
    pub(super) instance_path: Vec<AuthoringInstancePathSegment>,
}

/// Key-completion context active over one source interval.
#[derive(Clone, Debug, PartialEq, Eq)]
struct TableScope {
    range: ByteRange,
    path: Vec<AuthoringPathSegment>,
    existing_fields: BTreeSet<String>,
}

/// Best structural position for one editor query.
pub(super) struct SyntaxPosition<'a> {
    pub(super) fact: Option<&'a SyntaxFact>,
    pub(super) object_path: &'a [AuthoringPathSegment],
    pub(super) existing_fields: &'a BTreeSet<String>,
}

/// Recoverable structural index for one immutable editor document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AuthoringSyntax {
    source_range: ByteRange,
    source_len: usize,
    facts: Vec<SyntaxFact>,
    scopes: Vec<TableScope>,
    fact_limit_exceeded: bool,
}

/// Source-free tolerant indexes for one exact editor snapshot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct AuthoringSyntaxSnapshot {
    documents: BTreeMap<String, AuthoringSyntax>,
}

impl AuthoringSyntaxSnapshot {
    /// Add one explicitly TOML document under its compiler logical label.
    pub(super) fn insert(&mut self, label: String, syntax: AuthoringSyntax) {
        self.documents.insert(label, syntax);
    }

    /// Return the structural index for one exact compiler logical label.
    pub(super) fn document(&self, label: &str) -> Option<&AuthoringSyntax> {
        self.documents.get(label)
    }
}

impl AuthoringSyntax {
    /// Parse one current buffer without admitting any semantic fact.
    #[must_use]
    pub(super) fn parse(text: &str) -> Self {
        let source = Source::new(text);
        let tokens = source.lex().into_vec();
        let mut errors = Vec::<ParseError>::new();
        let mut decode_errors = Vec::<ParseError>::new();
        let mut builder = SyntaxBuilder::new(text.len());
        {
            let mut receiver = |event: Event| builder.event(&source, event, &mut decode_errors);
            let maximum_depth = u32::try_from(AUTHORING_PATH_SEGMENT_LIMIT)
                .expect("the authoring path limit fits the parser depth coordinate");
            let mut guarded = RecursionGuard::new(&mut receiver, maximum_depth);
            parse_document(&tokens, &mut guarded, &mut errors);
        }
        builder.finish(text.len())
    }

    /// Index only the compiler-selected declaration, keeping original offsets.
    pub(super) fn parse_region(text: &str, range: std::ops::Range<usize>) -> Option<Self> {
        let mut syntax = Self::parse(text.get(range.clone())?);
        for fact in &mut syntax.facts {
            fact.range.start += range.start;
            fact.range.end += range.start;
        }
        for scope in &mut syntax.scopes {
            scope.range.start += range.start;
            scope.range.end += range.start;
        }
        syntax.source_range = ByteRange::new(range.start, range.end);
        syntax.source_len = text.len();
        Some(syntax)
    }

    /// Return the narrowest source fact and active object context at `byte`.
    pub(super) fn position(&self, byte: usize) -> Option<SyntaxPosition<'_>> {
        if !(self.source_range.start <= byte && byte < self.source_range.end
            || byte == self.source_len && byte == self.source_range.end)
        {
            return None;
        }
        let fact = self
            .facts
            .iter()
            .filter(|fact| {
                fact.range.contains(byte)
                    || (matches!(
                        fact.kind,
                        SyntaxFactKind::Key | SyntaxFactKind::Value | SyntaxFactKind::ValueSlot
                    ) && fact.range.end == byte)
            })
            .min_by_key(|fact| (fact.range.len(), std::cmp::Reverse(fact.path.len())));
        let scope = self
            .scopes
            .iter()
            .filter(|scope| scope.range.contains(byte) || scope.range.end == byte)
            .max_by_key(|scope| (scope.range.start, scope.path.len()))
            .unwrap_or_else(|| {
                self.scopes
                    .first()
                    .expect("the root syntax scope covers the complete document")
            });
        Some(SyntaxPosition {
            fact,
            object_path: &scope.path,
            existing_fields: &scope.existing_fields,
        })
    }

    /// All recoverable facts in source order.
    pub(super) fn facts(&self) -> &[SyntaxFact] {
        &self.facts
    }

    /// Whether every recoverable structural fact fit the editor resource bound.
    pub(super) const fn is_complete(&self) -> bool {
        !self.fact_limit_exceeded
    }

    /// Resolve one linker-emitted instance path back to its narrowest value.
    pub(super) fn fact_for_instance_path(
        &self,
        path: &[AuthoringInstancePathSegment],
    ) -> Option<&SyntaxFact> {
        self.facts
            .iter()
            .filter(|fact| fact.instance_path == path)
            .min_by_key(|fact| match fact.kind {
                SyntaxFactKind::Value => 0,
                SyntaxFactKind::Key | SyntaxFactKind::HeaderKey => 1,
                SyntaxFactKind::Table | SyntaxFactKind::ArrayTable => 2,
                SyntaxFactKind::ValueSlot => 3,
            })
    }
}

#[derive(Clone, Copy)]
enum HeaderKind {
    Table,
    ArrayTable,
}

struct Header {
    kind: HeaderKind,
    start: usize,
    keys: Vec<(String, ByteRange)>,
    valid: bool,
}

struct PendingValueSlot {
    start: usize,
    path: Vec<AuthoringPathSegment>,
    instance_path: Vec<AuthoringInstancePathSegment>,
    value_observed: bool,
}

#[derive(Clone)]
enum Container {
    Array {
        path: Vec<AuthoringPathSegment>,
        instance_path: Vec<AuthoringInstancePathSegment>,
        next_index: usize,
    },
    InlineTable {
        path: Vec<AuthoringPathSegment>,
        instance_path: Vec<AuthoringInstancePathSegment>,
        scope: usize,
    },
}

struct SyntaxBuilder {
    facts: Vec<SyntaxFact>,
    scopes: Vec<TableScope>,
    current_table: Vec<AuthoringPathSegment>,
    current_instance_table: Vec<AuthoringInstancePathSegment>,
    current_scope: usize,
    header: Option<Header>,
    pending_keys: Vec<(String, ByteRange)>,
    pending_key_invalid: bool,
    value_path: Option<Vec<AuthoringPathSegment>>,
    value_instance_path: Option<Vec<AuthoringInstancePathSegment>>,
    value_slot: Option<PendingValueSlot>,
    containers: Vec<Container>,
    array_tables: BTreeSet<Vec<String>>,
    active_array_table_indices: BTreeMap<Vec<AuthoringInstancePathSegment>, usize>,
    next_array_table_indices: BTreeMap<Vec<AuthoringInstancePathSegment>, usize>,
    fact_limit_exceeded: bool,
}

impl SyntaxBuilder {
    fn new(document_len: usize) -> Self {
        Self {
            facts: Vec::new(),
            scopes: vec![TableScope {
                range: ByteRange::new(0, document_len),
                path: Vec::new(),
                existing_fields: BTreeSet::new(),
            }],
            current_table: Vec::new(),
            current_instance_table: Vec::new(),
            current_scope: 0,
            header: None,
            pending_keys: Vec::new(),
            pending_key_invalid: false,
            value_path: None,
            value_instance_path: None,
            value_slot: None,
            containers: Vec::new(),
            array_tables: BTreeSet::new(),
            active_array_table_indices: BTreeMap::new(),
            next_array_table_indices: BTreeMap::new(),
            fact_limit_exceeded: false,
        }
    }

    fn event(&mut self, source: &Source<'_>, event: Event, errors: &mut Vec<ParseError>) {
        let span = event.span();
        let range = ByteRange::new(span.start(), span.end());
        match event.kind() {
            EventKind::StdTableOpen => self.start_header(HeaderKind::Table, range.start),
            EventKind::ArrayTableOpen => self.start_header(HeaderKind::ArrayTable, range.start),
            EventKind::StdTableClose | EventKind::ArrayTableClose => {
                self.finish_header(range.end, range.len() != 0)
            }
            EventKind::SimpleKey => {
                if range.len() == 0 {
                    self.invalidate_pending_key();
                    return;
                }
                let Some(raw) = source.get(event) else {
                    return;
                };
                let mut key = String::new();
                let prior_error_count = errors.len();
                raw.decode_key(&mut key, errors);
                if errors.len() != prior_error_count {
                    self.invalidate_pending_key();
                    return;
                }
                if let Some(header) = self.header.as_mut() {
                    if !header.valid {
                        return;
                    }
                    if header.keys.len() >= AUTHORING_PATH_SEGMENT_LIMIT {
                        header.valid = false;
                        header.keys.clear();
                        return;
                    }
                    header.keys.push((key, range));
                } else if !self.pending_key_invalid {
                    if self.pending_keys.len() >= AUTHORING_PATH_SEGMENT_LIMIT {
                        self.invalidate_pending_key();
                        return;
                    }
                    self.pending_keys.push((key, range));
                }
            }
            EventKind::KeyValSep => self.start_value_slot(range.end),
            EventKind::Scalar if range.len() != 0 => self.scalar(source, range),
            EventKind::Scalar => {}
            EventKind::ArrayOpen => self.open_array(range),
            EventKind::ArrayClose => self.close_container(false, range.end),
            EventKind::InlineTableOpen => self.open_inline_table(range),
            EventKind::InlineTableClose => self.close_container(true, range.end),
            EventKind::Newline => {
                if self.header.is_some() {
                    self.finish_header(range.start, false);
                }
                if self.containers.is_empty() {
                    self.finish_value_slot(range.end);
                    self.pending_keys.clear();
                    self.pending_key_invalid = false;
                    self.value_path = None;
                    self.value_instance_path = None;
                }
            }
            EventKind::ValueSep => {
                self.finish_value_slot(range.start);
                self.pending_keys.clear();
                self.pending_key_invalid = false;
                self.value_path = None;
                self.value_instance_path = None;
            }
            EventKind::Error => {
                if self.header.is_some() {
                    self.finish_header(range.end, false);
                }
            }
            EventKind::KeySep | EventKind::Whitespace | EventKind::Comment => {}
        }
    }

    fn start_header(&mut self, kind: HeaderKind, start: usize) {
        self.finish_value_slot(start);
        self.header = Some(Header {
            kind,
            start,
            keys: Vec::new(),
            valid: true,
        });
        self.pending_keys.clear();
        self.pending_key_invalid = false;
        self.value_path = None;
        self.value_instance_path = None;
        self.containers.clear();
    }

    fn finish_header(&mut self, end: usize, complete: bool) {
        let Some(header) = self.header.take() else {
            return;
        };
        if !complete || !header.valid {
            return;
        }
        let raw_path = header
            .keys
            .iter()
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        let mut path = Vec::new();
        let mut instance_path = Vec::new();
        let mut prefix = Vec::new();
        for key in &raw_path {
            prefix.push(key.clone());
            path.push(AuthoringPathSegment::Field(key.clone()));
            instance_path.push(AuthoringInstancePathSegment::Field(key.clone()));
            let current_array_table =
                matches!(header.kind, HeaderKind::ArrayTable) && prefix.len() == raw_path.len();
            if self.array_tables.contains(&prefix) || current_array_table {
                path.push(AuthoringPathSegment::ArrayItem);
                let index = if current_array_table {
                    let next = self
                        .next_array_table_indices
                        .entry(instance_path.clone())
                        .or_default();
                    let index = *next;
                    *next = next.saturating_add(1);
                    self.active_array_table_indices
                        .insert(instance_path.clone(), index);
                    index
                } else {
                    let Some(index) = self.active_array_table_indices.get(&instance_path).copied()
                    else {
                        return;
                    };
                    index
                };
                instance_path.push(AuthoringInstancePathSegment::Index(index));
            }
            if path.len() > AUTHORING_PATH_SEGMENT_LIMIT
                || instance_path.len() > AUTHORING_PATH_SEGMENT_LIMIT
            {
                return;
            }
        }
        if matches!(header.kind, HeaderKind::ArrayTable) {
            self.array_tables.insert(raw_path);
        }
        let kind = match header.kind {
            HeaderKind::Table => SyntaxFactKind::Table,
            HeaderKind::ArrayTable => SyntaxFactKind::ArrayTable,
        };
        self.push_fact(SyntaxFact {
            kind,
            range: ByteRange::new(header.start, end),
            path: path.clone(),
            instance_path: instance_path.clone(),
        });
        for (index, (_, range)) in header.keys.iter().enumerate() {
            let key_path = path_through_key(&path, index);
            let key_instance_path = instance_path_through_key(&instance_path, index);
            self.push_fact(SyntaxFact {
                kind: SyntaxFactKind::HeaderKey,
                range: *range,
                path: key_path,
                instance_path: key_instance_path,
            });
        }
        if self.current_scope != 0 {
            if let Some(scope) = self.scopes.get_mut(self.current_scope) {
                scope.range.end = header.start;
            }
        }
        self.current_table = path.clone();
        self.current_instance_table = instance_path.clone();
        if self.scopes.len() >= MAX_AUTHORING_SYNTAX_FACTS {
            self.fact_limit_exceeded = true;
            return;
        }
        self.scopes.push(TableScope {
            range: ByteRange::new(end, end),
            path,
            existing_fields: BTreeSet::new(),
        });
        self.current_scope = self.scopes.len() - 1;
    }

    fn start_value_slot(&mut self, start: usize) {
        self.finish_value_slot(start);
        if self.pending_key_invalid || self.pending_keys.is_empty() {
            self.pending_keys.clear();
            self.pending_key_invalid = false;
            return;
        }
        let mut path = self
            .containers
            .iter()
            .rev()
            .find_map(|container| match container {
                Container::InlineTable { path, .. } => Some(path.clone()),
                Container::Array { .. } => None,
            })
            .unwrap_or_else(|| self.current_table.clone());
        let mut instance_path = self
            .containers
            .iter()
            .rev()
            .find_map(|container| match container {
                Container::InlineTable { instance_path, .. } => Some(instance_path.clone()),
                Container::Array { .. } => None,
            })
            .unwrap_or_else(|| self.current_instance_table.clone());
        if path.len().saturating_add(self.pending_keys.len()) > AUTHORING_PATH_SEGMENT_LIMIT
            || instance_path.len().saturating_add(self.pending_keys.len())
                > AUTHORING_PATH_SEGMENT_LIMIT
        {
            self.pending_keys.clear();
            self.pending_key_invalid = false;
            return;
        }
        let pending_keys = std::mem::take(&mut self.pending_keys);
        for (key, range) in pending_keys {
            path.push(AuthoringPathSegment::Field(key.clone()));
            instance_path.push(AuthoringInstancePathSegment::Field(key.clone()));
            self.push_fact(SyntaxFact {
                kind: SyntaxFactKind::Key,
                range,
                path: path.clone(),
                instance_path: instance_path.clone(),
            });
        }
        if let Some(AuthoringPathSegment::Field(field)) = path.last() {
            let scope_index = self
                .containers
                .iter()
                .rev()
                .find_map(|container| match container {
                    Container::InlineTable { scope, .. } => Some(*scope),
                    Container::Array { .. } => None,
                })
                .unwrap_or(self.current_scope);
            if let Some(scope) = self.scopes.get_mut(scope_index) {
                scope.existing_fields.insert(field.clone());
            }
        }
        self.value_path = Some(path.clone());
        self.value_instance_path = Some(instance_path.clone());
        self.value_slot = Some(PendingValueSlot {
            start,
            path,
            instance_path,
            value_observed: false,
        });
        self.pending_key_invalid = false;
    }

    fn invalidate_pending_key(&mut self) {
        if let Some(header) = self.header.as_mut() {
            header.valid = false;
            header.keys.clear();
        } else {
            self.pending_key_invalid = true;
            self.pending_keys.clear();
        }
    }

    fn scalar(&mut self, source: &Source<'_>, range: ByteRange) {
        if self.recover_inline_key(source, range) {
            return;
        }
        if let Some(slot) = self.value_slot.as_mut() {
            slot.value_observed = true;
        }
        if let Some((path, instance_path)) = self.take_value_target() {
            self.push_fact(SyntaxFact {
                kind: SyntaxFactKind::Value,
                range,
                path,
                instance_path,
            });
        }
    }

    fn recover_inline_key(&mut self, source: &Source<'_>, range: ByteRange) -> bool {
        let unresolved_inline_value = self
            .value_slot
            .as_ref()
            .is_some_and(|slot| !slot.value_observed)
            && self
                .containers
                .iter()
                .any(|container| matches!(container, Container::InlineTable { .. }));
        if !unresolved_inline_value {
            return false;
        }
        let Some(raw) = source.input().get(range.start..range.end) else {
            return false;
        };
        let valid_bare_key = !raw.is_empty()
            && raw
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
        let followed_by_separator = source
            .input()
            .get(range.end..)
            .is_some_and(|suffix| suffix.trim_start().starts_with('='));
        if !valid_bare_key || !followed_by_separator {
            return false;
        }
        self.finish_value_slot(range.start);
        self.pending_keys.push((raw.to_owned(), range));
        true
    }

    fn open_array(&mut self, range: ByteRange) {
        if range.len() == 0 {
            return;
        }
        if let Some(slot) = self.value_slot.as_mut() {
            slot.value_observed = true;
        }
        if let Some((mut path, instance_path)) = self.take_value_target() {
            path.push(AuthoringPathSegment::ArrayItem);
            self.containers.push(Container::Array {
                path,
                instance_path,
                next_index: 0,
            });
        }
    }

    fn open_inline_table(&mut self, range: ByteRange) {
        if range.len() == 0 {
            return;
        }
        if let Some(slot) = self.value_slot.as_mut() {
            slot.value_observed = true;
        }
        if let Some((path, instance_path)) = self.take_value_target() {
            if self.scopes.len() >= MAX_AUTHORING_SYNTAX_FACTS {
                self.fact_limit_exceeded = true;
                return;
            }
            self.scopes.push(TableScope {
                range: ByteRange::new(range.end, range.end),
                path: path.clone(),
                existing_fields: BTreeSet::new(),
            });
            let scope = self.scopes.len() - 1;
            self.containers.push(Container::InlineTable {
                path,
                instance_path,
                scope,
            });
        }
    }

    fn take_value_target(
        &mut self,
    ) -> Option<(Vec<AuthoringPathSegment>, Vec<AuthoringInstancePathSegment>)> {
        self.value_path
            .take()
            .zip(self.value_instance_path.take())
            .or_else(|| {
                self.containers
                    .iter_mut()
                    .rev()
                    .find_map(|container| match container {
                        Container::Array {
                            path,
                            instance_path,
                            next_index,
                        } => {
                            let mut exact = instance_path.clone();
                            exact.push(AuthoringInstancePathSegment::Index(*next_index));
                            *next_index = next_index.saturating_add(1);
                            Some((path.clone(), exact))
                        }
                        Container::InlineTable { .. } => None,
                    })
            })
    }

    fn close_container(&mut self, inline_table: bool, end: usize) {
        self.finish_value_slot(end);
        let expected = if inline_table {
            matches!(self.containers.last(), Some(Container::InlineTable { .. }))
        } else {
            matches!(self.containers.last(), Some(Container::Array { .. }))
        };
        if expected {
            if let Some(Container::InlineTable { scope, .. }) = self.containers.last() {
                if let Some(scope) = self.scopes.get_mut(*scope) {
                    scope.range.end = end;
                }
            }
            self.containers.pop();
        }
        self.value_path = None;
        self.value_instance_path = None;
    }

    fn finish_value_slot(&mut self, end: usize) {
        if let Some(PendingValueSlot {
            start,
            path,
            instance_path,
            value_observed: false,
        }) = self.value_slot.take()
        {
            self.push_fact(SyntaxFact {
                kind: SyntaxFactKind::ValueSlot,
                range: ByteRange::new(start, end.max(start)),
                path,
                instance_path,
            });
        }
    }

    fn finish(mut self, document_len: usize) -> AuthoringSyntax {
        self.finish_value_slot(document_len);
        if self.current_scope != 0 {
            if let Some(scope) = self.scopes.get_mut(self.current_scope) {
                scope.range.end = document_len;
            }
        }
        self.facts
            .sort_by_key(|fact| (fact.range.start, fact.range.end, fact_kind_order(fact.kind)));
        AuthoringSyntax {
            source_range: ByteRange::new(0, document_len),
            source_len: document_len,
            facts: self.facts,
            scopes: self.scopes,
            fact_limit_exceeded: self.fact_limit_exceeded,
        }
    }

    fn push_fact(&mut self, fact: SyntaxFact) {
        if self.facts.len() < MAX_AUTHORING_SYNTAX_FACTS {
            self.facts.push(fact);
        } else {
            self.fact_limit_exceeded = true;
        }
    }
}

fn path_through_key(
    complete: &[AuthoringPathSegment],
    key_index: usize,
) -> Vec<AuthoringPathSegment> {
    let field_count = key_index + 1;
    let mut seen = 0;
    complete
        .iter()
        .take_while(|segment| {
            if matches!(segment, AuthoringPathSegment::Field(_)) {
                seen += 1;
            }
            seen <= field_count
        })
        .cloned()
        .collect()
}

fn instance_path_through_key(
    complete: &[AuthoringInstancePathSegment],
    key_index: usize,
) -> Vec<AuthoringInstancePathSegment> {
    let field_count = key_index + 1;
    let mut seen = 0;
    complete
        .iter()
        .take_while(|segment| {
            if matches!(segment, AuthoringInstancePathSegment::Field(_)) {
                seen += 1;
            }
            seen <= field_count
        })
        .cloned()
        .collect()
}

const fn fact_kind_order(kind: SyntaxFactKind) -> u8 {
    match kind {
        SyntaxFactKind::Key | SyntaxFactKind::HeaderKey => 0,
        SyntaxFactKind::Value => 1,
        SyntaxFactKind::Table => 2,
        SyntaxFactKind::ArrayTable => 3,
        SyntaxFactKind::ValueSlot => 4,
    }
}

#[cfg(test)]
#[path = "../tests/internal/syntax.rs"]
mod tests;
