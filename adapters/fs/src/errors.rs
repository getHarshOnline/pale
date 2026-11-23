//! Filesystem failures classified by confinement and effect visibility.
//!
//! Callers can distinguish a restored no-effect failure from an outcome that
//! may already be visible through [`FsError::effect_visibility`]. [`FailureSet`]
//! preserves every recovery failure instead of flattening them into prose.

use std::error::Error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use pale_core::records::PlanKind;

type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// Every failure observed while attempting a best-effort batch or recovery.
///
/// The individual [`FsError`] values remain inspectable; they are never reduced
/// to a joined diagnostic string.
#[derive(Debug)]
pub struct FailureSet {
    failures: Vec<FsError>,
}

impl FailureSet {
    /// Preserve a non-empty failure vector as one structured error value.
    pub(crate) fn from_failures(failures: Vec<FsError>) -> Option<Self> {
        (!failures.is_empty()).then_some(Self { failures })
    }

    /// Structured failures in observation order.
    pub fn failures(&self) -> &[FsError] {
        &self.failures
    }

    /// Recover owned failures when composing nested recovery attempts.
    pub(crate) fn into_vec(self) -> Vec<FsError> {
        self.failures
    }
}

impl fmt::Display for FailureSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} failure(s)", self.failures.len())?;
        for (index, failure) in self.failures.iter().enumerate() {
            write!(formatter, "; {}: {failure}", index + 1)?;
        }
        Ok(())
    }
}

impl Error for FailureSet {}

/// Whether an [`FsError`] reports that the requested destination effect may
/// already be visible.
///
/// Cleanup failures are not visibility evidence by themselves. When cleanup
/// wraps the original publication failure, only that original `source` chain
/// determines whether callers must reconcile a possibly visible effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectVisibility {
    /// No visible destination effect is reported by this error chain.
    NoVisibleEffectReported,
    /// The destination effect became visible but completion could not be proved.
    MayBeVisible,
}

/// Filesystem failures classified for confinement and recovery decisions.
///
/// These values are runtime diagnostics, not persisted wire records. String detail
/// carries bounded local context; callers should branch on the variant, not prose.
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    /// An operating-system I/O operation failed at a named path.
    #[error("{operation} failed at {}: {source}", path.display())]
    Io {
        /// Stable operation description.
        operation: &'static str,
        /// Path or confined diagnostic path involved in the operation.
        path: PathBuf,
        /// Original operating-system error.
        #[source]
        source: io::Error,
    },
    /// Parsing, encoding, allocation, or another typed data operation failed.
    ///
    /// The original error remains available through
    /// [`std::error::Error::source`]. Use this variant when a typed operation
    /// failed; [`Self::InvalidState`] is reserved for source-less domain
    /// contradictions.
    #[error("{operation} failed at {}: {source}", path.display())]
    Data {
        /// Stable operation description.
        operation: &'static str,
        /// Record or filesystem path involved in the operation.
        path: PathBuf,
        /// Original typed error.
        #[source]
        source: BoxError,
    },
    /// An internal or domain precondition failed without an underlying OS error.
    #[error("{operation} failed at {}: {detail}", path.display())]
    Invariant {
        /// Stable operation description.
        operation: &'static str,
        /// Path or logical record involved in the precondition.
        path: PathBuf,
        /// Bounded explanation of the violated precondition.
        detail: String,
    },
    /// A bounded lock acquisition did not complete in time.
    #[error("{operation} timed out after {timeout:?} at {}", path.display())]
    Timeout {
        /// Lock purpose.
        operation: &'static str,
        /// Confined lock path.
        path: PathBuf,
        /// Declared acquisition bound.
        timeout: Duration,
    },
    /// The compiled platform cannot provide this adapter's confinement contract.
    #[error(
        "{operation} is unsupported on {platform}; descriptor-relative Unix semantics are required"
    )]
    UnsupportedPlatform {
        /// Attempted adapter operation.
        operation: &'static str,
        /// Compiled operating-system family.
        platform: &'static str,
    },
    /// A regular file exceeded its caller-selected byte budget.
    #[error("file at {path} exceeds the {max_bytes}-byte read limit")]
    FileReadLimit {
        /// Confined path whose stream crossed the bound.
        path: String,
        /// Caller-selected maximum byte count.
        max_bytes: usize,
    },
    /// A serialized record exceeded its store's publication budget.
    #[error("record at {path} exceeds the {max_bytes}-byte publication limit")]
    RecordSizeLimit {
        /// Confined record path whose encoded bytes crossed the bound.
        path: String,
        /// Store-selected maximum encoded byte count.
        max_bytes: usize,
    },
    /// One directory exceeded its caller-selected child-entry budget.
    #[error("directory at {path} exceeds the {max_entries}-entry read limit")]
    DirectoryEntryLimit {
        /// Confined directory whose immediate children crossed the bound.
        path: String,
        /// Caller-selected maximum child count.
        max_entries: usize,
    },
    /// A recursive scan exceeded its aggregate entry budget.
    #[error("directory traversal at {path} exceeds the {max_entries}-entry limit")]
    TraversalEntryLimit {
        /// Confined traversal root.
        path: String,
        /// Caller-selected maximum descendant count.
        max_entries: usize,
    },
    /// A recursive scan encountered an entry beyond its nesting budget.
    #[error("directory traversal at {path} exceeds the depth limit {max_depth}")]
    TraversalDepthLimit {
        /// First descendant observed beyond the nesting bound.
        path: String,
        /// Caller-selected maximum descendant depth.
        max_depth: usize,
    },
    /// A requested Unix mode contains bits outside the admitted permission mask.
    #[error("invalid Unix permission mode {mode:#o} at {path}; only bits in 0o7777 are admitted")]
    InvalidMode {
        /// Confined path that would have received the mode.
        path: String,
        /// Rejected raw mode.
        mode: u32,
    },
    /// One operation requested the same relative identity more than once.
    #[error("duplicate requested path: {path}")]
    DuplicateRequestPath {
        /// Repeated repository-relative path.
        path: String,
    },
    /// Existing filesystem state violates confinement, type, ownership, or an
    /// admitted precondition.
    #[error("collision at {0}: {1}")]
    Collision(String, String),
    /// Staged bytes do not match the required payload identity.
    #[error("staged content for {0} does not match the plan payload digest")]
    StageMismatch(String),
    /// A path is not an admitted project-local relative surface.
    #[error("path escapes the target root or is not a project-local surface: {0}")]
    Escape(String),
    /// Desired bytes were omitted for a write operation.
    #[error("missing content for planned operation {0}")]
    MissingContent(String),
    /// Materialization state is malformed, inconsistent, or unsupported.
    ///
    /// This variant carries a source-less domain contradiction. Typed parsing,
    /// decoding, canonicalization, and allocation failures use [`Self::Data`].
    #[error("invalid materialization state: {0}")]
    InvalidState(String),
    /// A machine-local effect contradicts its admitted root, baseline, or intent.
    #[error("invalid machine-local effect: {0}")]
    InvalidMachineLocalEffect(String),
    /// A failed publication also failed to remove its private staging entry.
    #[error("{operation} failed at {}; staging cleanup also failed: {cleanup}", path.display())]
    CleanupFailed {
        /// Publication operation.
        operation: &'static str,
        /// Destination associated with the staging entry.
        path: PathBuf,
        /// Original publication failure.
        #[source]
        source: Box<FsError>,
        /// Cleanup failure retained separately from the original cause.
        cleanup: Box<FsError>,
    },
    /// A filesystem effect became visible before its completion boundary failed.
    #[error("{operation} became visible at {} but completion is uncertain: {source}", path.display())]
    EffectUncertain {
        /// Effect whose visible state now requires reconciliation.
        operation: &'static str,
        /// Confined destination of the visible effect.
        path: PathBuf,
        /// Completion, durability, or post-publication cleanup failure.
        #[source]
        source: Box<FsError>,
    },
    /// A materialization failure with a separately observed persistent
    /// coordination-namespace effect.
    #[error("materialization coordination state changed before failure: {source}")]
    Coordination {
        /// Confirmed or uncertain persistent lock-file/parent effect.
        effect: crate::lock::AdvisoryLockEffect,
        /// Failure from lock admission or the guarded materialization operation.
        #[source]
        source: Box<FsError>,
    },
    /// Forward mutation failed and in-process recovery was incomplete.
    #[error("mutation failed ({source}); recovery was incomplete: {failures}")]
    RestoreFailed {
        /// Original forward-effect failure.
        #[source]
        source: Box<FsError>,
        /// Every recovery failure that was observed.
        failures: FailureSet,
    },
    /// Explicit rollback could not restore every recorded effect.
    #[error("{operation} was incomplete: {failures}")]
    RecoveryFailed {
        /// Recovery operation being attempted.
        operation: &'static str,
        /// Every recovery failure that was observed.
        failures: FailureSet,
    },
    /// A bounded cleanup batch completed with one or more structured failures.
    #[error("{operation} was incomplete: {failures}")]
    BatchFailed {
        /// Batch operation being attempted.
        operation: &'static str,
        /// Every per-entry failure that was observed.
        failures: FailureSet,
    },
    /// An expiring-record directory contains an entry outside its governed grammar.
    #[error("unexpected expiring-record entry at {}: expected {expected}", path.display())]
    UnexpectedExpiringRecordEntry {
        /// Unexpected entry path.
        path: PathBuf,
        /// Declared entry grammar.
        expected: &'static str,
    },
    /// An admitted operation class was used with the wrong materializer entry.
    #[error("admitted plan is class `{actual}` but this operation requires `{expected}`")]
    WrongPlanKind {
        /// Class carried by the admitted plan.
        actual: PlanKind,
        /// Class accepted by the called materializer entry.
        expected: PlanKind,
    },
}

impl FsError {
    /// Classify destination-effect visibility without parsing error prose.
    ///
    /// [`FsError::CleanupFailed`] can wrap a visible publication failure after staging
    /// cleanup also fails. The cleanup branch concerns the private staging
    /// entry, so it never widens destination visibility on its own.
    pub fn effect_visibility(&self) -> EffectVisibility {
        match self {
            Self::EffectUncertain { .. } => EffectVisibility::MayBeVisible,
            Self::CleanupFailed { source, .. } => source.effect_visibility(),
            Self::Coordination { effect, source } => match effect {
                crate::lock::AdvisoryLockEffect::OutcomeUnknown => EffectVisibility::MayBeVisible,
                crate::lock::AdvisoryLockEffect::Untouched
                | crate::lock::AdvisoryLockEffect::Confirmed => source.effect_visibility(),
            },
            Self::Io { .. }
            | Self::Data { .. }
            | Self::Invariant { .. }
            | Self::Timeout { .. }
            | Self::UnsupportedPlatform { .. }
            | Self::FileReadLimit { .. }
            | Self::RecordSizeLimit { .. }
            | Self::DirectoryEntryLimit { .. }
            | Self::TraversalEntryLimit { .. }
            | Self::TraversalDepthLimit { .. }
            | Self::InvalidMode { .. }
            | Self::DuplicateRequestPath { .. }
            | Self::Collision(..)
            | Self::StageMismatch(_)
            | Self::Escape(_)
            | Self::MissingContent(_)
            | Self::InvalidState(_)
            | Self::InvalidMachineLocalEffect(_)
            | Self::RestoreFailed { .. }
            | Self::RecoveryFailed { .. }
            | Self::BatchFailed { .. }
            | Self::UnexpectedExpiringRecordEntry { .. }
            | Self::WrongPlanKind { .. } => EffectVisibility::NoVisibleEffectReported,
        }
    }

    /// Persistent coordination effect retained alongside a materialization failure.
    #[must_use]
    pub fn coordination_effect(&self) -> crate::lock::AdvisoryLockEffect {
        match self {
            Self::Coordination { effect, .. } => *effect,
            _ => crate::lock::AdvisoryLockEffect::Untouched,
        }
    }

    /// Whether this error chain reports incomplete materialization restoration.
    #[must_use]
    pub fn restoration_failed(&self) -> bool {
        match self {
            Self::RestoreFailed { .. } | Self::RecoveryFailed { .. } => true,
            Self::Coordination { source, .. } | Self::CleanupFailed { source, .. } => {
                source.restoration_failed()
            }
            _ => false,
        }
    }

    /// Retain a nonempty persistent coordination effect alongside this failure.
    #[must_use]
    pub fn with_coordination_effect(self, effect: crate::lock::AdvisoryLockEffect) -> Self {
        match effect {
            crate::lock::AdvisoryLockEffect::Untouched => self,
            crate::lock::AdvisoryLockEffect::Confirmed
            | crate::lock::AdvisoryLockEffect::OutcomeUnknown => Self::Coordination {
                effect,
                source: Box::new(self),
            },
        }
    }
}

/// Attach stable operation and path context to a standard I/O failure.
pub(crate) fn io_error(
    operation: &'static str,
    path: impl Into<PathBuf>,
    source: io::Error,
) -> FsError {
    FsError::Io {
        operation,
        path: path.into(),
        source,
    }
}

#[cfg(unix)]
/// Convert a Unix `errno` while preserving stable operation and path context.
pub(crate) fn errno_error(
    operation: &'static str,
    path: impl Into<PathBuf>,
    source: rustix::io::Errno,
) -> FsError {
    io_error(
        operation,
        path,
        io::Error::from_raw_os_error(source.raw_os_error()),
    )
}

/// Box a typed data failure with stable operation and path context.
pub(crate) fn data_error(
    operation: &'static str,
    path: impl Into<PathBuf>,
    source: impl Error + Send + Sync + 'static,
) -> FsError {
    FsError::Data {
        operation,
        path: path.into(),
        source: Box::new(source),
    }
}

/// Construct a domain-precondition failure without inventing an OS source.
pub(crate) fn invariant(
    operation: &'static str,
    path: impl AsRef<Path>,
    detail: impl Into<String>,
) -> FsError {
    FsError::Invariant {
        operation,
        path: path.as_ref().to_path_buf(),
        detail: detail.into(),
    }
}

/// Preserve that an effect became visible before completion could be proved.
pub(crate) fn effect_uncertain(
    operation: &'static str,
    path: impl Into<PathBuf>,
    source: FsError,
) -> FsError {
    FsError::EffectUncertain {
        operation,
        path: path.into(),
        source: Box::new(source),
    }
}

#[cfg(not(unix))]
/// Report that the compiled platform cannot satisfy this adapter's contract.
pub(crate) fn unsupported(operation: &'static str) -> FsError {
    FsError::UnsupportedPlatform {
        operation,
        platform: std::env::consts::OS,
    }
}

#[cfg(test)]
#[path = "../tests/internal/errors.rs"]
mod tests;
