//! Observe the host wall clock and request OS cryptographic random bytes.
//!
//! These ports supply time and random bytes for callers that need them. They
//! do not allocate identities, prove uniqueness, select record references, or
//! authorize signing. Wall-clock readings do not establish monotonic ordering,
//! freshness, or protection against rollback.

use std::error::Error;

use pale_core::records::UnixTimeV1;

/// The selected operating-system cryptographic random source was unavailable.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("operating-system cryptographic random source is unavailable")]
pub struct CryptographicRandomSourceError;

impl CryptographicRandomSourceError {
    /// Classify failure of the selected operating-system source.
    pub const fn unavailable() -> Self {
        Self
    }
}

/// The host wall clock could not produce a portable Unix-time observation.
#[derive(Debug, thiserror::Error)]
#[error("host wall clock did not produce a portable Unix-time observation")]
pub struct WallClockSourceError {
    source: Box<dyn Error + Send + Sync + 'static>,
}

impl WallClockSourceError {
    /// Preserve the concrete source failure behind the neutral port boundary.
    pub fn from_source(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

/// Operating-system cryptographic randomness without semantic allocation policy.
///
/// The caller supplies a bounded destination and remains responsible for
/// domain separation, representation, uniqueness evidence, durable allocation,
/// and recovery. A successful fill establishes no such higher-level property.
pub trait CryptographicRandomSourcePort: Send + Sync {
    /// Fill the complete destination from the selected OS cryptographic source.
    ///
    /// # Errors
    /// Returns [`CryptographicRandomSourceError`] when the operating-system
    /// source cannot complete the request. The caller must discard the whole
    /// destination after any error.
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError>;
}

/// Wall-clock observation without continuity or authority claims.
///
/// This source supplies record times only. It does not establish monotonic
/// ordering, freshness, anti-rollback state, or active signing authority.
pub trait WallClockSourcePort: Send + Sync {
    /// Observe whole Unix seconds in the portable resident coordinate domain.
    ///
    /// # Errors
    /// Returns [`WallClockSourceError`] when the host clock precedes the Unix
    /// epoch or lies outside the portable signed-record integer range.
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError>;
}
