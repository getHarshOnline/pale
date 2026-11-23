//! Shared diagnostic text bounds, credential redaction and cause traversal.

use std::error::Error;

/// Maximum distinct causal messages retained in one diagnostic.
pub(super) const MAX_CAUSES: usize = 16;
/// Maximum causal links inspected, including a malformed cyclic source chain.
pub(super) const MAX_CAUSE_SCAN: usize = 64;
/// Maximum Unicode scalar count retained in one explanatory string.
pub(super) const MAX_TEXT_CHARS: usize = 4_096;
/// Stable replacement used for recognizable credential-bearing values.
const REDACTION: &str = "[REDACTED]";

/// Retain a bounded raw causal chain and count collapsed entries.
///
/// Exact summary repetitions consume only the scan budget, not a retained
/// cause or an omission. Distinct source text remains distinct here, even when
/// later redaction makes it identical. The shared projector performs the sole
/// redaction and field-bounding pass after this traversal.
pub(super) fn bounded_causes(error: &(dyn Error + 'static), summary: &str) -> (Vec<String>, u32) {
    let mut retained = Vec::new();
    let mut omitted = 0_u32;
    let mut scanned = 0_usize;
    let mut next = Some(error);
    while let Some(source) = next {
        if scanned == MAX_CAUSE_SCAN {
            omitted = omitted.saturating_add(1);
            break;
        }
        scanned += 1;
        let rendered = source.to_string();
        if rendered == summary {
            next = source.source();
            continue;
        }
        if retained.len() < MAX_CAUSES && retained.last() != Some(&rendered) {
            retained.push(rendered);
        } else {
            omitted = omitted.saturating_add(1);
        }
        next = source.source();
    }
    (retained, omitted)
}

/// Count a source chain without rendering any potentially sensitive member.
/// A count of 65 marks a chain that continues past the 64-link scan ceiling.
pub(super) fn causal_depth(error: &(dyn Error + 'static)) -> u32 {
    let mut count = 0_u32;
    let mut scanned = 0_usize;
    let mut next = Some(error);
    while let Some(source) = next {
        if scanned == MAX_CAUSE_SCAN {
            return count.saturating_add(1);
        }
        scanned += 1;
        count = count.saturating_add(1);
        next = source.source();
    }
    count
}

/// Redact and shorten explanatory text to the diagnostic text bound.
pub(super) fn bounded_redacted(value: &str, shortened_fields: &mut u32) -> String {
    bounded_redacted_to(value, MAX_TEXT_CHARS, shortened_fields)
}

/// Redact and shorten one string to its field-specific Unicode-scalar bound.
pub(super) fn bounded_redacted_to(value: &str, limit: usize, shortened_fields: &mut u32) -> String {
    let redacted = redact_sensitive(value);
    let mut retained_end = 0;
    let mut scalar_count = 0;
    for (index, scalar) in redacted.char_indices() {
        if scalar_count == limit {
            *shortened_fields = shortened_fields.saturating_add(1);
            let mut output = redacted[..retained_end].to_string();
            if limit != 0 {
                output.push('…');
            }
            return output;
        }
        scalar_count += 1;
        if scalar_count < limit {
            retained_end = index + scalar.len_utf8();
        }
    }
    redacted
}

/// Replace values following common credential markers without logging the source.
fn redact_sensitive(value: &str) -> String {
    let mut redacted = value.to_string();
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
        redact_values_after(&mut redacted, marker);
    }
    redacted
}

/// Redact every case-insensitive value occurrence following one marker.
fn redact_values_after(output: &mut String, marker: &str) {
    let mut cursor = 0;
    loop {
        let lower = output.to_ascii_lowercase();
        let Some(relative) = lower[cursor..].find(marker) else {
            break;
        };
        let mut value_start = cursor + relative + marker.len();
        while output
            .as_bytes()
            .get(value_start)
            .is_some_and(u8::is_ascii_whitespace)
        {
            value_start += 1;
        }
        let quote = output
            .as_bytes()
            .get(value_start)
            .copied()
            .filter(|byte| matches!(byte, b'\'' | b'"'));
        if quote.is_some() {
            value_start += 1;
        }
        let mut value_end = value_start;
        if let Some(quote) = quote {
            while output
                .as_bytes()
                .get(value_end)
                .is_some_and(|byte| *byte != quote)
            {
                value_end += 1;
            }
        } else {
            while output.as_bytes().get(value_end).is_some_and(|byte| {
                !byte.is_ascii_whitespace()
                    && !matches!(*byte, b'\'' | b'"' | b',' | b';' | b'}' | b']')
            }) {
                value_end += 1;
            }
        }
        if value_start == value_end {
            cursor = value_start;
            continue;
        }
        output.replace_range(value_start..value_end, REDACTION);
        cursor = value_start + REDACTION.len();
    }
}

#[cfg(test)]
#[path = "../../tests/internal/diagnostic/text.rs"]
mod tests;
