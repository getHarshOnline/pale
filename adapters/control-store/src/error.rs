//! Typed failures at the `SQLite` storage-source boundary.

use std::path::PathBuf;

use pale_ports::{ControlStoreError, ControlStoreErrorKind};

/// Retain a stable adapter operation label beside the native `SQLite` source.
pub(crate) fn sqlite(operation: &'static str, source: rusqlite::Error) -> ControlStoreError {
    ControlStoreError::with_source(
        ControlStoreErrorKind::Storage,
        ControlSqliteError::sqlite(operation, source),
    )
}

/// Convert an unsigned integer to `SQLite`'s signed integer range.
pub(crate) fn sqlite_integer(value: u64) -> Result<i64, ControlStoreError> {
    i64::try_from(value)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
}

/// Failure to verify, initialize, or inspect a control store.
#[derive(Debug, thiserror::Error)]
pub enum ControlSqliteError {
    /// The embedded storage manifest is not valid JSON.
    #[error("decode the embedded Pale control-storage manifest")]
    ManifestDecode(#[source] serde_json::Error),
    /// The manifest value does not reproduce its pinned RFC 8785 identity.
    #[error(
        "the embedded Pale control-storage manifest digest does not match its pinned JCS identity"
    )]
    ManifestDigestMismatch,
    /// An embedded source contradicts the selected initial-v1 storage contract.
    #[error("invalid embedded control-storage source: {0}")]
    SourceInvariant(&'static str),
    /// One migration's bytes do not reproduce the manifest digest.
    #[error("migration {migration_id} digest differs from the storage manifest")]
    MigrationDigestMismatch {
        /// Stable source-owned migration identity.
        migration_id: String,
    },
    /// A target database path or one of its `SQLite` sidecar paths already exists.
    #[error("control-store initialization target already exists: {0}")]
    InitializationTargetExists(PathBuf),
    /// Runtime-state root lacks the required absolute, no-follow, UID, or mode proof.
    #[error("unsafe runtime-state root: {0}")]
    UnsafeRuntimeStateRoot(PathBuf),
    /// A canonical descendant lacks the required no-follow, UID, or mode proof.
    #[error("unsafe control-storage directory: {0}")]
    UnsafeControlStorageDirectory(PathBuf),
    /// The principal writer-lock entry is not a private empty regular file.
    #[error("unsafe principal writer lock: {0}")]
    UnsafePrincipalWriterLock(PathBuf),
    /// Another descriptor already holds the principal's operating-system writer lock.
    #[error("the principal writer lock is already held")]
    PrincipalWriterLockContended,
    /// A retained runtime root or canonical descendant was replaced by pathname.
    #[error("control-storage location changed while retained: {0}")]
    StorageLocationChanged(PathBuf),
    /// Canonical path scope differs from the raw scope supplied for store admission.
    #[error("control-store location and identity scopes differ")]
    StoreLocationScopeMismatch,
    /// A reserved path no longer names the file created by this invocation.
    #[error("SQLite candidate ownership changed during creation: {0}")]
    CandidateOwnershipLost(PathBuf),
    /// Candidate creation failed and its owned main file could not be removed.
    #[error("{operation} left an unverified partial candidate at {path}")]
    CandidateCleanupFailed {
        /// Stable cleanup operation label.
        operation: &'static str,
        /// Newly created candidate that must not be treated as valid evidence.
        path: PathBuf,
        /// Original candidate-creation failure.
        failure: Box<ControlSqliteError>,
        /// Failure while removing the identity-bound main file or syncing its parent.
        #[source]
        cleanup: std::io::Error,
    },
    /// A filesystem operation failed.
    #[error("{operation} at {path}")]
    Io {
        /// Stable operation label.
        operation: &'static str,
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Original operating-system failure.
        #[source]
        source: std::io::Error,
    },
    /// A `SQLite` operation failed before it could be reduced to an open classification.
    #[error("SQLite control-store operation failed: {operation}")]
    Sqlite {
        /// Stable operation label.
        operation: &'static str,
        /// Original `SQLite` failure.
        #[source]
        source: rusqlite::Error,
    },
}

/// Whether a `SQLite` failure proves malformed database bytes rather than an
/// operationally unavailable inspection.
pub(crate) fn sqlite_error_is_corruption(source: &rusqlite::Error) -> bool {
    match source {
        rusqlite::Error::SqliteFailure(error, _) => matches!(
            error.code,
            rusqlite::ffi::ErrorCode::DatabaseCorrupt | rusqlite::ffi::ErrorCode::NotADatabase
        ),
        rusqlite::Error::SqlInputError { error, .. } => matches!(
            error.code,
            rusqlite::ffi::ErrorCode::DatabaseCorrupt | rusqlite::ffi::ErrorCode::NotADatabase
        ),
        _ => false,
    }
}

/// Whether a `SQLite` failure describes availability or host operation rather
/// than malformed persistent bytes.
pub(crate) fn sqlite_error_is_operational(source: &rusqlite::Error) -> bool {
    match source {
        rusqlite::Error::SqliteFailure(error, _) | rusqlite::Error::SqlInputError { error, .. } => {
            !matches!(
                error.code,
                rusqlite::ffi::ErrorCode::DatabaseCorrupt | rusqlite::ffi::ErrorCode::NotADatabase
            )
        }
        rusqlite::Error::SqliteSingleThreadedMode | rusqlite::Error::InvalidPath(_) => true,
        _ => false,
    }
}

impl ControlSqliteError {
    /// Construct a filesystem failure without discarding its native source.
    pub(crate) fn io(
        operation: &'static str,
        path: impl Into<PathBuf>,
        source: std::io::Error,
    ) -> Self {
        Self::Io {
            operation,
            path: path.into(),
            source,
        }
    }

    /// Construct a `SQLite` failure with a stable adapter operation label.
    pub(crate) fn sqlite(operation: &'static str, source: rusqlite::Error) -> Self {
        Self::Sqlite { operation, source }
    }
}
