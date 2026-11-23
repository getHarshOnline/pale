//! Diagnose declared configuration and observed machine state without repairing it.
//!
//! This module builds on `compile` and `workspace`; no other module in this
//! crate builds on it, and `pale-sdk` exposes it. A passing check establishes its named observation only. Missing
//! declarations and unavailable runtime checks remain distinct from failed
//! observations; none can be promoted to readiness by a successful compilation or
//! socket lookup.

mod machine;

pub use machine::inspect_machine;

use serde::Serialize;

/// Result of one bounded diagnostic check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    /// The named check completed and met its declared condition.
    Passed,
    /// The observation contradicted the condition or could not finish.
    Failed,
    /// Required input or an implemented observation path is absent.
    Unavailable,
    /// The selected configuration does not require this check.
    NotSelected,
}

/// One observation and its concrete next action.
#[derive(Debug, Serialize)]
pub struct Check {
    /// Stable name for the condition being checked.
    pub code: String,
    /// Logical subject or explicitly supplied build path.
    pub subject: String,
    /// Closed result, including missing evidence.
    pub state: CheckState,
    /// What was established or what prevented the check.
    pub message: String,
    /// Action that supplies missing input or resolves the reported condition.
    pub help: String,
}

impl Check {
    /// Construct an observation without granting runtime authority.
    pub fn new(
        code: &str,
        subject: impl Into<String>,
        state: CheckState,
        message: impl Into<String>,
        help: impl Into<String>,
    ) -> Self {
        Self {
            code: code.to_owned(),
            subject: subject.into(),
            state,
            message: message.into(),
            help: help.into(),
        }
    }
}

/// Logical file-size observation for one caller-selected tree.
#[derive(Debug, Serialize)]
pub struct BuildSize {
    /// Explicit physical directory supplied for this observation.
    pub root: String,
    /// Sum of regular-file lengths; hard links count once per entry.
    /// This is not allocated or reclaimable disk space.
    pub logical_bytes: Option<u64>,
    /// Number of regular-file entries measured successfully.
    pub file_count: u64,
    /// Symbolic links observed and excluded.
    pub skipped_symlinks: u64,
    /// Whether all observations completed within their bounds.
    pub complete: bool,
    /// Bounded errors that prevented a complete measurement.
    pub errors: Vec<String>,
}

/// Complete diagnosis; it is never an activation or recovery decision.
#[derive(Debug, Default, Serialize)]
pub struct Report {
    /// Ordered checks, including failures after an earlier check failed.
    pub checks: Vec<Check>,
    /// Separate root observations; overlapping roots must not be added together.
    pub builds: Vec<BuildSize>,
}

impl Report {
    /// Whether every selected check completed successfully.
    ///
    /// Even true establishes only the listed checks, never permission to act.
    #[must_use]
    pub fn checks_passed(&self) -> bool {
        !self.checks.is_empty()
            && self
                .checks
                .iter()
                .all(|check| matches!(check.state, CheckState::Passed | CheckState::NotSelected))
    }

    /// Retain a build observation and apply its optional logical-byte ceiling.
    pub fn add_build(&mut self, build: BuildSize, limit: Option<u64>) {
        let over_limit = limit
            .zip(build.logical_bytes)
            .is_some_and(|(limit, size)| size > limit);
        let state = if build.complete && !over_limit {
            CheckState::Passed
        } else {
            CheckState::Failed
        };
        let message = if !build.complete {
            "Build tree measurement is incomplete; any reported size is partial.".to_owned()
        } else if over_limit {
            format!(
                "Build tree exceeds the declared logical-byte limit of {}.",
                limit.unwrap_or(0)
            )
        } else {
            "Build tree measured completely in logical regular-file bytes.".to_owned()
        };
        self.checks.push(Check::new(
            "build_size", &build.root, state, message,
            "Review scan errors or the declared limit. Size alone does not establish that files may be removed.",
        ));
        self.builds.push(build);
    }
}
