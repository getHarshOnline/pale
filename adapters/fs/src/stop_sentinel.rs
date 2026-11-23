//! Confined custody for the service-manager stop sentinel.
//!
//! The sentinel is a local keep-alive guard, not desired-state authority. This
//! module authenticates only the adapter-owned marker needed for safe repeated
//! publication and removal. The caller remains responsible for releasing a
//! sentinel transition only after the corresponding desired state is durable.

use std::path::Path;

use crate::confine::{ConfinedLocation, ConfinedRoot, FileIdentity, ReadLimit};
use crate::errors::{effect_uncertain, FsError};

/// Exact private marker recognized as a Pale-created stop sentinel.
///
/// Service-manager artifacts depend only on sentinel presence. These bytes are
/// retained solely so custody never removes an arbitrary file found at the
/// caller-selected deployment coordinate.
const MARKER_BYTES: &[u8] = b"pale-supervisor-stop\n";

/// Owner-only mode required for a recognized stop sentinel.
const MARKER_MODE: u32 = 0o600;

/// Read ceiling equal to the only admitted marker length.
const MARKER_READ_LIMIT: ReadLimit = ReadLimit::new(MARKER_BYTES.len());

/// Exact mechanically observed state of a stop-sentinel coordinate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopSentinelObservation {
    /// No entry exists at the configured coordinate.
    Absent,
    /// The coordinate contains the exact adapter-owned marker and mode.
    Present,
}

/// Result of converging one stop-sentinel coordinate on a requested state.
///
/// Separate variants preserve whether this call changed the filesystem without
/// reducing idempotence evidence to a boolean.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopSentinelDisposition {
    /// The exact marker already existed; no write occurred.
    AlreadyPresent,
    /// This call durably published the exact marker.
    Created,
    /// No marker existed; no removal occurred.
    AlreadyAbsent,
    /// This call durably removed the exact marker.
    Removed,
}

/// Retained, descriptor-confined custody for one stop-sentinel coordinate.
///
/// The root and relative path locate a deployment artifact; neither grants
/// lifecycle authority. An existing entry is accepted only when its bytes and
/// mode match the private marker exactly. This prevents an `up` transition from
/// deleting a foreign file merely because it occupies the configured path.
pub struct StopSentinelCustody {
    root: ConfinedRoot,
    marker: ConfinedLocation,
}

impl StopSentinelCustody {
    /// Bind one relative stop-sentinel coordinate beneath an existing root.
    ///
    /// Construction opens and retains the confinement root, validates the
    /// relative grammar, and checks every currently existing ancestor without
    /// creating directories or touching the marker.
    ///
    /// # Errors
    ///
    /// Rejects an unsafe root, a non-canonical relative coordinate, or a
    /// currently existing ancestor that cannot be resolved without following a
    /// symbolic link.
    pub fn new(root: &Path, relative: &str) -> Result<Self, FsError> {
        let root = ConfinedRoot::open(root)?;
        let marker = root.location(relative)?;
        marker.validate_ancestors()?;
        Ok(Self { root, marker })
    }

    /// Observe absence or the exact adapter-owned marker without mutation.
    ///
    /// # Errors
    ///
    /// Rejects symbolic links, non-regular entries, oversized or foreign bytes,
    /// a non-owner-only mode, root replacement, and incomplete confined I/O.
    pub fn observe(&self) -> Result<StopSentinelObservation, FsError> {
        self.root.verify_path_identity()?;
        let observation = if self.observe_marker()?.is_some() {
            StopSentinelObservation::Present
        } else {
            StopSentinelObservation::Absent
        };
        self.root.verify_path_identity()?;
        Ok(observation)
    }

    /// Durably ensure that the exact stop sentinel exists.
    ///
    /// The parent directory must already exist. Publication is atomic and
    /// synchronized through [`ConfinedLocation::write_atomic_noclobber`]. A
    /// repeated call returns [`StopSentinelDisposition::AlreadyPresent`].
    ///
    /// # Errors
    ///
    /// Rejects foreign or unsafe entries, missing or unsafe parents, root
    /// replacement, and incomplete publication. A post-publication root
    /// identity failure is [`FsError::EffectUncertain`] because the marker may
    /// already be visible through the retained directory capability.
    pub fn ensure_present(&self) -> Result<StopSentinelDisposition, FsError> {
        self.root.verify_path_identity()?;
        if self.observe_marker()?.is_some() {
            self.root.verify_path_identity()?;
            return Ok(StopSentinelDisposition::AlreadyPresent);
        }
        self.marker
            .write_atomic_noclobber(MARKER_BYTES, MARKER_MODE)?;
        self.root.verify_path_identity().map_err(|source| {
            effect_uncertain("publish stop sentinel", self.marker.display_path(), source)
        })?;
        Ok(StopSentinelDisposition::Created)
    }

    /// Durably ensure that the exact stop sentinel is absent.
    ///
    /// Only the exact adapter-owned bytes and mode are eligible for removal. A
    /// repeated call returns [`StopSentinelDisposition::AlreadyAbsent`].
    ///
    /// # Errors
    ///
    /// Rejects foreign or unsafe entries, root replacement, and incomplete
    /// removal. A visible unlink whose parent synchronization or subsequent root
    /// check fails is reported as [`FsError::EffectUncertain`].
    pub fn ensure_absent(&self) -> Result<StopSentinelDisposition, FsError> {
        self.root.verify_path_identity()?;
        let Some(identity) = self.observe_marker()? else {
            self.root.verify_path_identity()?;
            return Ok(StopSentinelDisposition::AlreadyAbsent);
        };
        let removed = self.marker.remove_file_if_identity(identity)?;
        if !removed {
            return Err(FsError::Collision(
                self.marker.relative().to_string(),
                "stop sentinel disappeared after exact admission".into(),
            ));
        }
        self.root.verify_path_identity().map_err(|source| {
            effect_uncertain("remove stop sentinel", self.marker.display_path(), source)
        })?;
        Ok(StopSentinelDisposition::Removed)
    }

    /// Admit the exact open-handle identity eligible for one removal attempt.
    fn observe_marker(&self) -> Result<Option<FileIdentity>, FsError> {
        let Some(existing) = self.marker.read_file(MARKER_READ_LIMIT)? else {
            return Ok(None);
        };
        if existing.bytes != MARKER_BYTES || existing.mode != MARKER_MODE {
            return Err(FsError::Collision(
                self.marker.relative().to_string(),
                format!(
                    "entry is not the exact owner-only stop sentinel (mode {:#o})",
                    existing.mode
                ),
            ));
        }
        Ok(Some(existing.identity))
    }
}

#[cfg(test)]
#[path = "../tests/internal/stop_sentinel.rs"]
mod tests;
