# Write Rust with clear ownership

Apply this after [the shared engineering method](../judgment/common.md) and the repository's toolchain, formatter, lint, feature and unsafe policies are known.

Use ownership to expose responsibility. Borrow when that expresses the needed lifetime, make transfers explicit, and avoid clones whose only purpose is to escape a design problem. Model closed states with enums and domain identities with newtypes; use traits for real substitution.

Recoverable failures use typed results whose actionable identity survives added context. Reserve panics for proven invariants, never untrusted or ordinarily recoverable input. Every `unsafe` block explains the invariants its callers must preserve; examine FFI, aliasing, pinning, thread safety and destruction order where they apply.

For concurrent and asynchronous work, identify task ownership, cancellation, channel bounds, lock order, blocking boundaries and cleanup. Choose iterators and static or dynamic dispatch for clarity while exposing material allocation or locking cost.

## Keep a folder module's public API small

Suppose a crate exposes `job::Job`. Its `src/lib.rs` declares `pub mod job;`, and `src/job/mod.rs` keeps the public type while the parser stays private:

```rust
//! Job creation and validation.
//!
//! Construct a [`Job`] with [`Job::parse`] to reject an empty name.

mod parse;

pub use parse::{Job, ParseError};

#[cfg(test)]
#[path = "../../tests/unit/job.rs"]
mod tests;
```

The implementation in `src/job/parse.rs` owns parsing, rather than an arbitrary fraction of the old file:

```rust
//! Validate a job name before constructing a job.

/// A job with a nonempty name.
#[derive(Debug, PartialEq, Eq)]
pub struct Job {
    name: String,
}

/// The supplied name contains no non-whitespace characters.
#[derive(Debug, PartialEq, Eq)]
pub struct ParseError;

impl Job {
    /// Construct a job, rejecting a blank name.
    pub fn parse(name: &str) -> Result<Self, ParseError> {
        if !valid_name(name) {
            return Err(ParseError);
        }
        Ok(Self { name: name.to_owned() })
    }

    /// Return the name as supplied by the caller.
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub(super) fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
}
```

Follow the repository's test layout. When it keeps tests outside `src/`, the `#[path]` declaration above makes `tests/unit/job.rs` a child of the module for unit testing, so helpers need no public visibility. Its path is relative to `src/job/`. The subfolder keeps this file out of Cargo's automatic discovery of top-level integration tests.

```rust
//! Unit checks for job parsing.

use super::{Job, ParseError, parse::valid_name};

#[test]
fn blank_names_are_rejected_and_valid_names_are_preserved() {
    assert!(!valid_name(" \t"));
    assert_eq!(Job::parse(" \t"), Err(ParseError));
    let job = Job::parse("nightly").expect("a nonblank name");
    assert_eq!(job.name(), "nightly");
}
```

Keep integration tests at the public crate boundary. Add private-item tests when they protect a local invariant that is difficult to exercise there; do not expose implementation just to test it.

## Model results the caller can act on

Represent an operation's results as an enum with separate `Complete`, `Retryable` and `Rejected` cases. Have the caller handle each case explicitly and check a retry after partial input. Keep a lower-level failure available to the caller instead of turning every failure into an undifferentiated string.

Choose focused unit behaviour, public-crate integration, error and genuine panic cases, deterministic async control, and property tests where they protect an invariant. Formatting and lint checks support repository policy; a behaviour claim still needs a test that reaches it.

## Before handing off

- [ ] Do the types make ownership, valid states and recoverable failures clear?
- [ ] Are unsafe assumptions explained and checked at their real callers?
- [ ] Do the relevant behaviour and failure tests run deterministically?
