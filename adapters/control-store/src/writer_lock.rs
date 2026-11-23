//! Operating-system lock held by one principal writer guard.
//!
//! Cooperating Pale residents retain this lock while they hold mutable state
//! for the same principal. It is separate from the signed writer fence and
//! proves no liveness, takeover, database, or Pale Runtime assembly fact.

use std::fs::{File, TryLockError};
use std::path::Path;

use pale_core::records::ResidentPrincipalIdV1;

use crate::location::ControlStorageRoot;
use crate::ControlSqliteError;

/// Held operating-system writer lock for one principal.
///
/// Keep this value alive until every trust and deployment store for the
/// principal is closed. Dropping it releases the operating-system lock but
/// leaves the empty coordination file in place for later reuse.
/// The operating system releases the lock when the last descriptor closes,
/// including after process exit. `CLOEXEC` closes this descriptor during a
/// successful `exec`.
///
/// The guard is not a PID file, a liveness observation, a takeover proof, or a
/// substitute for the signed principal-resident-writer fence.
#[must_use = "dropping the guard releases the principal writer lock"]
pub struct PrincipalWriterLock {
    principal_id: ResidentPrincipalIdV1,
    held_file: File,
}

impl PrincipalWriterLock {
    /// Borrow the principal protected by this held lock.
    #[must_use]
    pub const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }
}

impl pale_ports::PrincipalWriterGuardPort for PrincipalWriterLock {
    fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }
}

impl std::fmt::Debug for PrincipalWriterLock {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PrincipalWriterLock")
            .field("principal_id", &self.principal_id)
            .field("held_file", &"<locked>")
            .finish()
    }
}

impl Drop for PrincipalWriterLock {
    fn drop(&mut self) {
        let _ = self.held_file.unlock();
    }
}

/// Acquire the exclusive operating-system writer lock for one principal.
///
/// The caller selects an existing private absolute runtime-state root. The
/// adapter derives the same hashed principal directory used by the trust store,
/// opens or creates its persistent empty coordination file without following
/// links, and attempts the operating-system lock once. Contention fails closed;
/// this function never waits for or displaces the current holder.
///
/// The returned guard must be retained before any trust or deployment store is
/// opened and until all of those stores are closed. The adapter uses native
/// advisory file locks on macOS and Linux. It does not establish process
/// health, qualify network-filesystem lock behavior, or defend against writers
/// that ignore the lock.
///
/// # Errors
///
/// Returns [`ControlSqliteError::PrincipalWriterLockContended`] when another
/// descriptor holds the principal lock. Unsafe roots, directories, and lock-file
/// entries fail closed. Other host failures preserve their operating-system
/// source in [`ControlSqliteError`].
pub fn acquire_principal_writer_lock(
    runtime_state_root: &Path,
    principal_id: ResidentPrincipalIdV1,
) -> Result<PrincipalWriterLock, ControlSqliteError> {
    let root = ControlStorageRoot::retain(runtime_state_root)?;
    let (held_file, path) = root.open_principal_writer_lock_file(&principal_id)?;
    match held_file.try_lock() {
        Ok(()) => Ok(PrincipalWriterLock {
            principal_id,
            held_file,
        }),
        Err(TryLockError::WouldBlock) => Err(ControlSqliteError::PrincipalWriterLockContended),
        Err(TryLockError::Error(source)) => Err(ControlSqliteError::io(
            "acquire principal writer lock",
            path,
            source,
        )),
    }
}
