//! Bounded cross-process locking for confined mutable state and evidence.
//!
//! Locks are advisory and attached to the owned file handle. Acquisition times
//! out so a stalled process cannot block a caller indefinitely. They serialize only
//! cooperating processes and do not prevent unrelated filesystem writers.

use std::fs::{File, TryLockError};
use std::time::{Duration, Instant};

use crate::confine::{ConfinedLocation, ConfinedRoot};
use crate::errors::{io_error, FsError};

/// Poll interval while acquiring a cooperating advisory lock.
const LOCK_SPIN: Duration = Duration::from_millis(2);

/// Held advisory lock for one confined coordination file.
///
/// The guard serializes only cooperating processes. Dropping it releases the
/// operating-system lock but deliberately leaves the admitted lock file for
/// reuse; it is coordination state, not durable decision evidence.
#[must_use = "the advisory lock is released when its guard is dropped"]
pub struct AdvisoryLock(File);

/// Persistent coordination-state observation from advisory-lock admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdvisoryLockEffect {
    /// No lock-file or parent-directory change was observed.
    Untouched,
    /// A lock-file or parent-directory effect was completed and confirmed.
    Confirmed,
    /// A namespace effect may be visible, but its final result is not known.
    OutcomeUnknown,
}

/// Advisory-lock failure with the separately observed persistent-file effect.
#[derive(Debug, thiserror::Error)]
#[error("{source}")]
pub struct AdvisoryLockFailure {
    effect: AdvisoryLockEffect,
    #[source]
    source: FsError,
}

impl AdvisoryLockFailure {
    pub(crate) const fn new(source: FsError, effect: AdvisoryLockEffect) -> Self {
        Self { effect, source }
    }

    /// Persistent lock-file effect observed before acquisition failed.
    #[must_use]
    pub const fn effect(&self) -> AdvisoryLockEffect {
        self.effect
    }

    /// Underlying filesystem failure without its persistent-effect observation.
    #[must_use]
    pub const fn source_error(&self) -> &FsError {
        &self.source
    }

    /// Recover the underlying filesystem failure.
    #[must_use]
    pub fn into_source(self) -> FsError {
        self.source
    }
}

impl std::fmt::Debug for AdvisoryLock {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AdvisoryLock(<held-file>)")
    }
}

impl Drop for AdvisoryLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

/// Acquire a no-follow lock file beneath a retained confinement root.
///
/// This serializes cooperating processes only. The lock file is retained for
/// reuse, and acquisition fails after `timeout` rather than waiting forever. A
/// retained lock must be an empty owner-only regular file; mismatched pre-existing
/// entries are preserved as collisions rather than repaired or adopted.
/// The persistent lock file is coordination state, not durable decision evidence.
/// Parent directories created for the persistent lock namespace are not rolled
/// back when acquisition fails.
///
/// # Errors
///
/// Rejects unsafe paths and untrusted lock entries, and reports namespace,
/// inspection, locking, or timeout failures. It never reports success without a
/// held guard.
pub fn acquire_advisory_lock(
    root: &ConfinedRoot,
    relative: &str,
    timeout: Duration,
    what: &'static str,
) -> Result<(AdvisoryLock, AdvisoryLockEffect), AdvisoryLockFailure> {
    let location = root
        .location(relative)
        .map_err(|source| AdvisoryLockFailure::new(source, AdvisoryLockEffect::Untouched))?;
    acquire_location(&location, timeout, what)
}

/// Acquire a confined advisory lock with bounded waiting.
/// The lock namespace is created when absent, and the lock file persists after
/// the handle is unlocked.
pub(crate) fn acquire_location(
    location: &ConfinedLocation,
    timeout: Duration,
    what: &'static str,
) -> Result<(AdvisoryLock, AdvisoryLockEffect), AdvisoryLockFailure> {
    let (file, effect) = location.open_lock_file()?;
    acquire_file(file, location.display_path(), timeout, what)
        .map(|lock| (lock, effect))
        .map_err(|source| AdvisoryLockFailure::new(source, effect))
}

fn acquire_file(
    lock: File,
    path: std::path::PathBuf,
    timeout: Duration,
    what: &'static str,
) -> Result<AdvisoryLock, FsError> {
    let start = Instant::now();
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(AdvisoryLock(lock)),
            Err(TryLockError::WouldBlock) => {
                if start.elapsed() >= timeout {
                    return Err(FsError::Timeout {
                        operation: what,
                        path,
                        timeout,
                    });
                }
                std::thread::sleep(LOCK_SPIN);
            }
            Err(TryLockError::Error(source)) => {
                return Err(io_error("acquire advisory lock", path, source));
            }
        }
    }
}
