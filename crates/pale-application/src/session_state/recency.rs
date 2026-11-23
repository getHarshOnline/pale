//! Bounded recency updates for ephemeral hook-session observations.
//!
//! Oversized values become digest markers, repeated captures move to the newest
//! position, and saturation evicts only the oldest capture. These transitions
//! emit state effects; they are not durable owner evidence.

use std::borrow::Cow;

use super::{
    SessionRuntimeState, StateEffect, MAX_CAPTURES, MAX_STATE_TEXT_BYTES, MAX_TOUCHED_PATHS,
};

fn state_text(value: &str) -> Cow<'_, str> {
    // Prefix that disambiguates literal values resembling digest markers.
    const LITERAL_ESCAPE: &str = "text:";
    // Prefix identifying a bounded digest replacement for oversized text.
    const DIGEST_MARKER: &str = "sha256:";

    if value.len() <= MAX_STATE_TEXT_BYTES
        && !value.starts_with(LITERAL_ESCAPE)
        && !value.starts_with(DIGEST_MARKER)
    {
        Cow::Borrowed(value)
    } else if value
        .len()
        .checked_add(LITERAL_ESCAPE.len())
        .is_some_and(|length| length <= MAX_STATE_TEXT_BYTES)
    {
        Cow::Owned(format!("{LITERAL_ESCAPE}{value}"))
    } else {
        Cow::Owned(format!(
            "{DIGEST_MARKER}{}",
            pale_core::sha256_hex(value.as_bytes())
        ))
    }
}

impl SessionRuntimeState {
    /// Record a first-touch key once; duplicates and saturation return no effect.
    pub(crate) fn note_first_touch(&mut self, key: &str) -> Option<StateEffect> {
        let key = state_text(key).into_owned();
        let first = self.touched.len() < MAX_TOUCHED_PATHS && self.touched.insert(key.clone());
        first.then_some(StateEffect::FirstTouch { key })
    }

    /// Retain one bounded capture entry in recency order.
    /// Returns no effect when the entry is already newest.
    pub(crate) fn capture(&mut self, entry: &str) -> Option<StateEffect> {
        let entry = state_text(entry).into_owned();
        if let Some(position) = self.captures.iter().position(|current| current == &entry) {
            if position + 1 == self.captures.len() {
                return None;
            }
            self.captures.remove(position);
        } else if self.captures.len() == MAX_CAPTURES {
            self.captures.pop_front();
        }
        self.captures.push_back(entry.clone());
        Some(StateEffect::Captured { entry })
    }

    /// Report whether this bounded recency set already contains the key.
    pub(crate) fn has_touched(&self, key: &str) -> bool {
        self.touched.contains(state_text(key).as_ref())
    }
}
