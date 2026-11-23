//! Compile-time platform and durable-publication identity.

use serde::{Deserialize, Serialize};

/// Strongest file-data flush used by Pale durable publication on this build.
///
/// The value names the operating-system mechanism, not an unconditional claim
/// about storage hardware, firmware, or power-loss behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DurablePublication {
    /// Apple `F_FULLFSYNC` for file data, followed by parent-directory `fsync`.
    AppleFullFsync,
    /// POSIX file and parent-directory `fsync`.
    UnixFsync,
    /// Rust [`std::fs::File::sync_all`] for the file and parent directory.
    StandardSyncAll,
}

/// Operating-system token compiled into this executable.
///
/// This reports the build target, not a runtime probe of the host kernel.
#[must_use]
pub const fn target_os() -> &'static str {
    std::env::consts::OS
}

/// Architecture token compiled into this executable.
///
/// This reports the build target, not a runtime probe of the current process.
#[must_use]
pub const fn target_arch() -> &'static str {
    std::env::consts::ARCH
}

/// Strongest durable-publication mechanism implemented by this build.
///
/// See [`DurablePublication`] for the boundary of the durability claim.
#[must_use]
pub const fn durable_publication() -> DurablePublication {
    #[cfg(target_vendor = "apple")]
    {
        DurablePublication::AppleFullFsync
    }
    #[cfg(all(unix, not(target_vendor = "apple")))]
    {
        DurablePublication::UnixFsync
    }
    #[cfg(not(unix))]
    {
        DurablePublication::StandardSyncAll
    }
}
