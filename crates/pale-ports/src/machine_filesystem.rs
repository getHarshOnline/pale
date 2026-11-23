//! Read-only observations of machine-local filesystem paths.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// Mechanical kind observed for one machine-local path.
///
/// This classification carries no admission policy. In particular,
/// [`Self::Symlink`] is reported rather than followed silently, while the
/// separately reported canonical target may have another kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachinePathKind {
    /// Directory entry.
    Directory,
    /// Regular-file entry.
    RegularFile,
    /// Symbolic-link entry.
    Symlink,
    /// Any other filesystem entry, such as a socket or named pipe.
    Other,
}

/// Mechanical stage at which a [`MachineFilesystemPort`] operation failed.
///
/// Callers may map these closed stages into semantic failures without parsing
/// adapter-specific error text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineFilesystemOperation {
    /// Inspect the requested path without following a final symbolic link.
    InspectRequested,
    /// Resolve the requested path to its canonical target.
    Canonicalize,
    /// Inspect the resolved canonical target.
    InspectCanonical,
}

impl fmt::Display for MachineFilesystemOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InspectRequested => "inspect requested",
            Self::Canonicalize => "canonicalize",
            Self::InspectCanonical => "inspect canonical",
        })
    }
}

/// Complete mechanical observation of one machine-local path.
///
/// The requested and canonical kinds remain separate so application policy can
/// distinguish an alias from its target. The Unix mode, when available, belongs
/// to the requested entry inspected without following its final symbolic link.
/// No field grants a retained handle or later mutation authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachinePathObservation {
    canonical_path: PathBuf,
    requested_kind: MachinePathKind,
    canonical_kind: MachinePathKind,
    unix_mode: Option<u32>,
}

impl MachinePathObservation {
    /// Construct an adapter-produced mechanical observation.
    ///
    /// This constructor establishes no path, type, permission, or containment
    /// policy; consumers decide whether the reported facts are admissible.
    pub fn new(
        canonical_path: PathBuf,
        requested_kind: MachinePathKind,
        canonical_kind: MachinePathKind,
        unix_mode: Option<u32>,
    ) -> Self {
        Self {
            canonical_path,
            requested_kind,
            canonical_kind,
            unix_mode,
        }
    }

    /// Return the resolved canonical target path.
    pub fn canonical_path(&self) -> &Path {
        &self.canonical_path
    }

    /// Return the kind inspected at the requested path without following its
    /// final symbolic link.
    pub const fn requested_kind(&self) -> MachinePathKind {
        self.requested_kind
    }

    /// Return the kind inspected at the canonical target.
    pub const fn canonical_kind(&self) -> MachinePathKind {
        self.canonical_kind
    }

    /// Return the requested entry's Unix permission and file-type mode.
    ///
    /// Non-Unix implementations return [`None`]. Consumers must not substitute
    /// a default mode when the platform does not provide this observation.
    pub const fn unix_mode(&self) -> Option<u32> {
        self.unix_mode
    }
}

/// Failure to complete one [`MachineFilesystemPort`] observation.
#[derive(Debug, thiserror::Error)]
#[error("cannot {operation} machine path `{path}`: {source}", path = .path.display())]
pub struct MachineFilesystemError {
    operation: MachineFilesystemOperation,
    path: PathBuf,
    #[source]
    source: std::io::Error,
}

impl MachineFilesystemError {
    /// Construct an adapter failure at one exact mechanical stage.
    pub fn new(
        operation: MachineFilesystemOperation,
        path: PathBuf,
        source: std::io::Error,
    ) -> Self {
        Self {
            operation,
            path,
            source,
        }
    }

    /// Return the failed mechanical stage.
    pub const fn operation(&self) -> MachineFilesystemOperation {
        self.operation
    }

    /// Return the path being inspected when the failure occurred.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the closed standard-library cause of the failed observation.
    ///
    /// Application policy can distinguish an absent path from a denied or
    /// interrupted observation without parsing adapter-authored error text.
    pub fn kind(&self) -> io::ErrorKind {
        self.source.kind()
    }
}

/// Read-only machine-filesystem observations required by application policy.
///
/// Implementations report requested-entry and canonical-target facts without
/// deciding whether those facts are admissible. Every result is a pre-action
/// observation: a successful call retains no filesystem handle, grants no
/// confinement against races, and authorizes no mutation.
pub trait MachineFilesystemPort {
    /// Inspect one requested entry without following its final symbolic link.
    ///
    /// This narrower operation lets consumers that need only entry presence or
    /// kind avoid canonicalizing and inspecting the target.
    ///
    /// # Errors
    ///
    /// Returns [`MachineFilesystemError`] with
    /// [`MachineFilesystemOperation::InspectRequested`] when the entry cannot
    /// be inspected.
    fn inspect_requested(&self, path: &Path) -> Result<MachinePathKind, MachineFilesystemError>;

    /// Resolve one requested path to its canonical target.
    ///
    /// # Errors
    ///
    /// Returns [`MachineFilesystemError`] with
    /// [`MachineFilesystemOperation::Canonicalize`] when resolution cannot
    /// complete.
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, MachineFilesystemError>;

    /// Observe one requested path and its canonical target.
    ///
    /// Implementations first inspect the requested entry without following its
    /// final symbolic link, then canonicalize it, then inspect the canonical
    /// target. Each failure identifies its exact
    /// [`MachineFilesystemOperation`].
    ///
    /// # Errors
    ///
    /// Returns [`MachineFilesystemError`] when any required mechanical stage
    /// cannot complete.
    fn observe(&self, path: &Path) -> Result<MachinePathObservation, MachineFilesystemError>;
}
