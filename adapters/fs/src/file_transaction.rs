//! Track reversible file changes through completion and in-process recovery.
//!
//! Each file publication is atomic, but a process crash can leave a multi-file
//! operation partially applied. The retained advisory lock coordinates only other
//! users of this adapter.

use std::fmt;
use std::time::Duration;

use pale_core::records::{ArtifactDigest, MaterializationLineageDigest, ReceiptOperation};

use crate::confine::{
    ConfinedLocation, ConfinedRoot, CreatedDirectory, FileIdentity, ReadLimit,
    MATERIALIZED_SURFACE_READ_LIMIT,
};
use crate::errors::{FailureSet, FsError};
use crate::lock::{AdvisoryLock, AdvisoryLockEffect, AdvisoryLockFailure};

#[derive(Clone)]
/// Exact pre-effect file state retained for in-process recovery.
pub(crate) struct FileSnapshot {
    /// Complete bytes observed before the effect.
    pub(crate) bytes: Vec<u8>,
    /// Unix permission bits observed before the effect.
    pub(crate) mode: u32,
    /// Open-handle identity observed before the effect.
    pub(crate) identity: FileIdentity,
}

/// Prior file state; `None` records exact absence.
pub(crate) type PriorFile = Option<FileSnapshot>;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Exact currently observed entry identity used by recovery collision checks.
enum EntryIdentity {
    Absent,
    File {
        sha256: ArtifactDigest,
        mode: u32,
        identity: FileIdentity,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Exact post-effect identity planned before publication.
enum PlannedIdentity {
    Absent,
    File { sha256: ArtifactDigest, mode: u32 },
}

impl PlannedIdentity {
    fn admits(&self, actual: &EntryIdentity) -> bool {
        match (self, actual) {
            (Self::Absent, EntryIdentity::Absent) => true,
            (
                Self::File {
                    sha256: expected_sha256,
                    mode: expected_mode,
                },
                EntryIdentity::File { sha256, mode, .. },
            ) => expected_sha256 == sha256 && expected_mode == mode,
            _ => false,
        }
    }
}

/// Recovery record binding a pre-state snapshot to the exact published identity.
///
/// The post-state binding prevents rollback from overwriting a third-party edit
/// made after the corresponding forward effect.
pub(crate) struct Backup {
    location: ConfinedLocation,
    prior: PriorFile,
    planned: PlannedIdentity,
    published: Option<EntryIdentity>,
}

impl Backup {
    /// Bind a write recovery record to the exact bytes and mode being published.
    pub(crate) fn write(
        location: ConfinedLocation,
        prior: PriorFile,
        bytes: &[u8],
        mode: u32,
    ) -> Self {
        Self {
            location,
            prior,
            planned: PlannedIdentity::File {
                sha256: ArtifactDigest::hash_bytes(bytes),
                mode,
            },
            published: None,
        }
    }

    /// Bind a delete recovery record to absence as its exact published state.
    pub(crate) fn delete(location: ConfinedLocation, prior: PriorFile) -> Self {
        Self {
            location,
            prior,
            planned: PlannedIdentity::Absent,
            published: None,
        }
    }

    /// Confirm that a store implementation published exactly the admitted effect.
    pub(crate) fn verify_published(&mut self) -> Result<Option<FileIdentity>, FsError> {
        let actual = observe(&self.location)?;
        if self.planned.admits(&actual) {
            let identity = match &actual {
                EntryIdentity::Absent => None,
                EntryIdentity::File { identity, .. } => Some(*identity),
            };
            self.published = Some(actual);
            Ok(identity)
        } else {
            Err(FsError::Collision(
                self.location.relative().to_string(),
                "published entry does not match the admitted transaction result".into(),
            ))
        }
    }

    fn prior_identity(&self) -> EntryIdentity {
        identity_of(self.prior.as_ref())
    }

    fn restore_if_unchanged(&self) -> Result<(), FsError> {
        let actual = observe(&self.location)?;
        if actual == self.prior_identity() {
            return Ok(());
        }
        if self.published.as_ref() != Some(&actual) {
            return Err(FsError::Collision(
                self.location.relative().to_string(),
                "entry no longer has the exact transaction-published identity; current entry preserved"
                    .into(),
            ));
        }
        match &self.prior {
            Some(snapshot) => self.location.write_atomic(&snapshot.bytes, snapshot.mode),
            None => self.location.remove_file().map(|_| ()),
        }
    }
}

/// Rollback snapshot ceiling matching forward materialization admission.
const ROLLBACK_SNAPSHOT_LIMIT: ReadLimit = MATERIALIZED_SURFACE_READ_LIMIT;

/// Maximum wait for the target-wide materialization lock.
const APPLY_LOCK_TIMEOUT: Duration = Duration::from_secs(10);

/// Serialize cooperating target mutations until the transaction becomes terminal.
///
/// The lock lives at `.pale/apply.lock` and
/// remains owned by [`AppliedFileTransaction`] through commit, rollback, or drop.
pub(crate) fn acquire_apply_lock(
    root: &ConfinedRoot,
) -> Result<(AdvisoryLock, AdvisoryLockEffect), AdvisoryLockFailure> {
    crate::lock::acquire_advisory_lock(root, ".pale/apply.lock", APPLY_LOCK_TIMEOUT, "target apply")
}

/// A published but still in-process reversible mutation.
///
/// The caller either [`commit`]s to keep the writes or [`rollback`]s to restore the
/// recorded pre-state, allowing downstream evidence persistence to occur while the
/// advisory target lock remains held. Dropping an unfinished value attempts
/// best-effort recovery but cannot report failure; callers must complete explicitly.
#[must_use = "an AppliedFileTransaction must be either committed or rolled back"]
pub struct AppliedFileTransaction {
    observation: AppliedFileTransactionObservation,
    coordination_effect: AdvisoryLockEffect,
    backups: Vec<Backup>,
    created_dirs: Vec<CreatedDirectory>,
    terminal: bool,
    // Declared last so recovery state drops before the lock.
    _lock: AdvisoryLock,
}

/// Read-only receipt input observed from one applied filesystem transaction.
///
/// This is not durable evidence and does not itself authorize [`commit`].
/// Call [`AppliedFileTransaction::observation`] to persist a receipt while the target lock
/// remains held. The transaction deliberately exposes no mutable observation.
#[derive(Debug)]
pub struct AppliedFileTransactionObservation {
    /// Receipt operations for externally visible target surfaces.
    pub operations: Vec<ReceiptOperation>,
    /// Trusted lineage identity observed before mutation, when one existed.
    pub prior_lineage_digest: Option<MaterializationLineageDigest>,
    /// Trusted lineage identity published by the mutation, when one remains.
    pub post_lineage_digest: Option<MaterializationLineageDigest>,
}

impl fmt::Debug for AppliedFileTransaction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AppliedFileTransaction")
            .field("observation", &self.observation)
            .field("coordination_effect", &self.coordination_effect)
            .field("recovery_file_count", &self.backups.len())
            .field("recovery_directory_count", &self.created_dirs.len())
            .field("terminal", &self.terminal)
            .finish_non_exhaustive()
    }
}

impl AppliedFileTransaction {
    /// Assemble a live transaction from its observation, recovery set, and held lock.
    pub(crate) fn new(
        observation: AppliedFileTransactionObservation,
        coordination_effect: AdvisoryLockEffect,
        backups: Vec<Backup>,
        created_dirs: Vec<CreatedDirectory>,
        lock: AdvisoryLock,
    ) -> Self {
        Self {
            observation,
            coordination_effect,
            backups,
            created_dirs,
            terminal: false,
            _lock: lock,
        }
    }

    /// Borrow the receipt-facing observation while the transaction remains live.
    pub fn observation(&self) -> &AppliedFileTransactionObservation {
        &self.observation
    }

    /// Persistent coordination namespace effect observed before materialization.
    #[must_use]
    pub const fn coordination_effect(&self) -> AdvisoryLockEffect {
        self.coordination_effect
    }
}

/// Keep the published files, disable in-process rollback, and release the target lock.
///
/// Performs no additional synchronization and does not prove successful settlement.
/// The owning settlement decision may retain these files after durable receipt
/// publication or after uncertain publication that requires reconciliation.
pub fn commit(mut txn: AppliedFileTransaction) {
    txn.terminal = true;
}

/// Undo a still-uncommitted mutation in the current process.
///
/// Recorded files are attempted in reverse order and recorded directories are
/// pruned when empty. Failure leaves the transaction terminal and target state
/// unknown. [`FsError::RecoveryFailed`] retains every observed restoration or
/// directory-pruning failure; a collision preserves the newer entry. Persistent
/// apply-lock coordination state is intentionally retained and is available from
/// [`AppliedFileTransaction::coordination_effect`].
///
/// # Errors
///
/// Returns [`FsError::RecoveryFailed`] after attempting the complete recovery set
/// when any file cannot be restored or any transaction-created directory cannot be
/// safely pruned. The transaction is terminal even on error, preventing blind retry.
pub fn rollback(mut txn: AppliedFileTransaction) -> Result<(), FsError> {
    let result =
        recover(&txn.backups, &txn.created_dirs).map_err(|failures| FsError::RecoveryFailed {
            operation: "roll back materialization",
            failures,
        });
    txn.terminal = true;
    result
}

impl Drop for AppliedFileTransaction {
    fn drop(&mut self) {
        if self.terminal {
            return;
        }
        // Drop cannot report recovery failure, but it must not introduce a second
        // panic while unwinding an unfinished transaction.
        let backups = std::mem::take(&mut self.backups);
        let created = std::mem::take(&mut self.created_dirs);
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            recover(&backups, &created).is_ok()
        }));
        match outcome {
            Ok(true) => eprintln!(
                "pale-adapter-fs: AppliedFileTransaction dropped without commit/rollback; pre-state restored (safety net)"
            ),
            Ok(false) => eprintln!(
                "pale-adapter-fs: AppliedFileTransaction dropped without commit/rollback; best-effort restore had errors (target may be UNKNOWN)"
            ),
            Err(_) => eprintln!(
                "pale-adapter-fs: AppliedFileTransaction dropped without commit/rollback; restore panicked (swallowed; target may be UNKNOWN)"
            ),
        }
    }
}

/// Restore every backed-up path to its recorded bytes and mode in reverse order.
///
/// Each rewrite is atomically published, so interruption cannot expose a truncated
/// individual file. The sequence as a whole is not crash-atomic.
fn restore(backups: &[Backup]) -> Result<(), FailureSet> {
    let mut errors = Vec::new();
    for backup in backups.iter().rev() {
        if let Err(error) = backup.restore_if_unchanged() {
            errors.push(error);
        }
    }
    finish_recovery(errors)
}

/// Attempt every file restoration and directory prune, preserving all failures.
pub(crate) fn recover(
    backups: &[Backup],
    created_dirs: &[CreatedDirectory],
) -> Result<(), FailureSet> {
    let mut errors = Vec::new();
    if let Err(failures) = restore(backups) {
        errors.extend(failures.into_vec());
    }
    for directory in created_dirs.iter().rev() {
        if let Err(error) = directory.remove_if_unchanged() {
            errors.push(error);
        }
    }
    finish_recovery(errors)
}

fn finish_recovery(errors: Vec<FsError>) -> Result<(), FailureSet> {
    match FailureSet::from_failures(errors) {
        Some(failures) => Err(failures),
        None => Ok(()),
    }
}

/// Snapshot a bounded regular file's bytes and Unix mode, or `None` when absent.
///
/// Symlinks and other non-regular entries are collisions, not absent state. Files
/// larger than [`MATERIALIZED_SURFACE_READ_LIMIT`] cannot enter this recovery set.
pub(crate) fn backup_of(location: &ConfinedLocation) -> Result<PriorFile, FsError> {
    Ok(location
        .read_file(ROLLBACK_SNAPSHOT_LIMIT)?
        .map(|file| FileSnapshot {
            bytes: file.bytes,
            mode: file.mode,
            identity: file.identity,
        }))
}

fn observe(location: &ConfinedLocation) -> Result<EntryIdentity, FsError> {
    Ok(match location.read_file(ROLLBACK_SNAPSHOT_LIMIT)? {
        Some(file) => EntryIdentity::File {
            sha256: ArtifactDigest::hash_bytes(&file.bytes),
            mode: file.mode,
            identity: file.identity,
        },
        None => EntryIdentity::Absent,
    })
}

fn identity_of(file: Option<&FileSnapshot>) -> EntryIdentity {
    match file {
        Some(snapshot) => EntryIdentity::File {
            sha256: ArtifactDigest::hash_bytes(&snapshot.bytes),
            mode: snapshot.mode,
            identity: snapshot.identity,
        },
        None => EntryIdentity::Absent,
    }
}

#[cfg(all(test, unix))]
#[path = "../tests/internal/file_transaction.rs"]
mod tests;
