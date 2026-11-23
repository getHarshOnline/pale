//! Identity-bound cleanup and synchronization for inactive-copy qualification.

use super::*;

impl CandidateReservation {
    /// Synchronize the closed candidate's main file and recheck its path identity.
    ///
    /// Callers must first close every `SQLite` connection that can write the
    /// candidate. This retained descriptor then synchronizes the exact file
    /// created by the reservation. The surrounding pathname checks retain the
    /// adapter's documented same-UID substitution limitation; they do not turn
    /// a pathname-consuming `SQLite` connection into descriptor-bound proof.
    pub(crate) fn sync_closed_main(
        &self,
        operation: &'static str,
    ) -> Result<(), ControlSqliteError> {
        self.verify_path_identity(operation)?;
        self.reservation
            .sync_all()
            .map_err(|source| ControlSqliteError::io(operation, &self.path, source))?;
        self.verify_path_identity(operation)
    }

    /// Reject any WAL or shared-memory sidecar before sealing an inactive copy.
    ///
    /// The reservation never removes a sidecar because a later observation
    /// cannot prove which same-principal process created it.
    pub(crate) fn verify_no_sidecars(
        &self,
        operation: &'static str,
    ) -> Result<(), ControlSqliteError> {
        self.verify_path_identity(operation)?;
        for (sidecar_leaf, sidecar_path) in sidecar_leaves_and_paths(&self.path, &self.leaf) {
            match rustix::fs::statat(&self.directory, &sidecar_leaf, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(_) => {
                    return Err(ControlSqliteError::InitializationTargetExists(sidecar_path));
                }
                Err(Errno::NOENT) => {}
                Err(source) => return Err(errno_error(operation, sidecar_path, source)),
            }
        }
        Ok(())
    }

    /// Remove only this reservation's main-file identity after an incomplete copy.
    ///
    /// A missing main file is already discarded. An observed foreign
    /// replacement is preserved and reported; no sidecar is ever removed. As
    /// elsewhere in this reservation, a same-UID actor can still substitute a
    /// pathname between descriptor-relative observation and unlink.
    pub(crate) fn discard(self) -> Result<(), ControlSqliteError> {
        let path = self.path.clone();
        match self.remove_owned_main("discard incomplete SQLite candidate")? {
            CandidateRemoval::RemovedOrAbsent => Ok(()),
            CandidateRemoval::ForeignPreserved => {
                Err(ControlSqliteError::CandidateOwnershipLost(path))
            }
        }
    }
}
