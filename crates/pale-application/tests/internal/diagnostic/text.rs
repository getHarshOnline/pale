//! Byte-preserving text extraction, bounds and non-rendering source omission.

use std::{cell::Cell, fmt};

use super::*;

#[derive(Debug)]
struct ChainError {
    message: String,
    source: Option<Box<Self>>,
}

impl fmt::Display for ChainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ChainError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref().map(|source| source as &dyn Error)
    }
}

fn chain(messages: impl DoubleEndedIterator<Item = String>) -> Box<ChainError> {
    messages
        .rev()
        .fold(None, |source, message| {
            Some(Box::new(ChainError { message, source }))
        })
        .expect("nonempty test chain")
}

#[test]
fn summary_repetitions_preserve_distinct_raw_causes_before_redaction() {
    let summary = "input rejected";
    let source = chain(
        [
            summary,
            "input rejected: dependency missing",
            summary,
            "token=first",
            "token=second",
            summary,
        ]
        .into_iter()
        .map(str::to_owned),
    );
    let (causes, omitted) = bounded_causes(source.as_ref(), summary);
    assert_eq!(
        causes,
        [
            "input rejected: dependency missing",
            "token=first",
            "token=second"
        ]
    );
    assert_eq!(omitted, 0);
    let mut shortened = 0;
    let redacted: Vec<_> = causes
        .iter()
        .map(|cause| bounded_redacted(cause, &mut shortened))
        .collect();
    assert_eq!(
        redacted,
        [
            "input rejected: dependency missing",
            "token=[REDACTED]",
            "token=[REDACTED]"
        ]
    );
    assert_eq!(shortened, 0);
}

#[test]
fn summary_only_chains_still_exhaust_the_scan_ceiling() {
    let summary = "input rejected";
    for (length, expected_omitted) in [(MAX_CAUSE_SCAN, 0), (MAX_CAUSE_SCAN + 1, 1)] {
        let source = chain((0..length).map(|_| summary.to_owned()));
        let (causes, omitted) = bounded_causes(source.as_ref(), summary);
        assert!(causes.is_empty());
        assert_eq!(omitted, expected_omitted);
    }
}

#[test]
fn retention_and_adjacent_duplicates_preserve_exact_omission_accounting() {
    let source = chain((0..80).map(|index| format!("cause-{index}")));
    let (causes, omitted) = bounded_causes(source.as_ref(), "summary");
    assert_eq!(
        causes,
        (0..16)
            .map(|index| format!("cause-{index}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(omitted, 49);
    assert_eq!(causal_depth(source.as_ref()), 65);
    let repeated = chain(["one", "one", "two", "one"].into_iter().map(str::to_owned));
    assert_eq!(
        bounded_causes(repeated.as_ref(), "summary"),
        (vec!["one".into(), "two".into(), "one".into()], 1)
    );
    assert_eq!(causal_depth(repeated.as_ref()), 4);
}

#[derive(Debug)]
struct OpaqueCycle {
    visits: Cell<usize>,
}

impl fmt::Display for OpaqueCycle {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("opaque causes must not be rendered");
    }
}

impl Error for OpaqueCycle {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.visits.set(self.visits.get() + 1);
        Some(self)
    }
}

#[test]
fn opaque_cyclic_causes_are_counted_without_display_or_unbounded_traversal() {
    let source = OpaqueCycle {
        visits: Cell::new(0),
    };
    assert_eq!(causal_depth(&source), 65);
    assert_eq!(source.visits.get(), 64);
}

#[test]
fn every_credential_marker_preserves_exact_surrounding_bytes() {
    for marker in [
        "bearer ",
        "token=",
        "\"token\":",
        "secret=",
        "\"secret\":",
        "client_secret=",
        "\"client_secret\":",
        "password=",
        "\"password\":",
        "api_key=",
        "\"api_key\":",
        "authorization=",
        "authorization:",
    ] {
        let marker = marker.to_ascii_uppercase();
        let mut shortened = 0;
        let text = format!("prefix {marker}  '敏感 value'; suffix {marker}\"second\"]");
        assert_eq!(
            bounded_redacted(&text, &mut shortened),
            format!("prefix {marker}  '[REDACTED]'; suffix {marker}\"[REDACTED]\"]")
        );
        assert_eq!(shortened, 0);
    }
    let mut shortened = 0;
    assert_eq!(
        bounded_redacted("token=; token='' token=", &mut shortened),
        "token=; token='' token="
    );
    assert_eq!(shortened, 0);
}

#[test]
fn unquoted_values_retain_the_existing_delimiter_and_marker_order() {
    let mut shortened = 0;
    assert_eq!(
        bounded_redacted("token=one; PASSWORD=two]", &mut shortened),
        "token=[REDACTED]; PASSWORD=[REDACTED]]"
    );
    // The existing `secret=` pass precedes `client_secret=`. Preserve its
    // exact bytes during extraction, including the retained closing brackets.
    assert_eq!(
        bounded_redacted("client_secret=one]", &mut shortened),
        "client_secret=[REDACTED]]]"
    );
    assert_eq!(shortened, 0);
}

#[test]
fn unicode_scalar_bounds_include_the_ellipsis_and_count_only_changed_fields() {
    for (text, limit, expected, count) in [
        ("", 0, "", 0),
        ("é", 0, "", 1),
        ("é", 1, "é", 0),
        ("é🦀", 1, "…", 1),
        ("é🦀界", 2, "é…", 1),
        ("e\u{301}界", 3, "e\u{301}界", 0),
        ("e\u{301}界!", 3, "e\u{301}…", 1),
    ] {
        let mut shortened = 0;
        assert_eq!(bounded_redacted_to(text, limit, &mut shortened), expected);
        assert_eq!(shortened, count);
        assert!(expected.chars().count() <= limit);
    }
    let mut shortened = u32::MAX;
    assert_eq!(bounded_redacted_to("two", 1, &mut shortened), "…");
    assert_eq!(shortened, u32::MAX);
    let mut shortened = 0;
    let text = format!("token={} tail", "secret".repeat(MAX_TEXT_CHARS));
    assert_eq!(
        bounded_redacted(&text, &mut shortened),
        "token=[REDACTED] tail"
    );
    assert_eq!(shortened, 0, "redaction precedes field-length accounting");
    let shortened_text = bounded_redacted(&"界".repeat(MAX_TEXT_CHARS + 1), &mut shortened);
    assert_eq!(shortened_text.chars().count(), MAX_TEXT_CHARS);
    assert!(shortened_text.ends_with('…'));
    assert_eq!(shortened, 1);
}
