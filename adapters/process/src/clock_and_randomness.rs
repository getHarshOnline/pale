//! Read the host wall clock and fill buffers with OS cryptographic randomness.
//!
//! These implementations supply inputs, not identity-allocation or signing
//! policy. The [`WallClockSourcePort`] and [`CryptographicRandomSourcePort`]
//! contracts define what each observation establishes.

use std::time::{SystemTime, UNIX_EPOCH};

use pale_core::records::UnixTimeV1;
use pale_ports::{
    CryptographicRandomSourceError, CryptographicRandomSourcePort, WallClockSourceError,
    WallClockSourcePort,
};

/// Cryptographic randomness supplied directly by the operating system.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsCryptographicRandom;

impl CryptographicRandomSourcePort for OsCryptographicRandom {
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        getrandom::fill(destination).map_err(|_| CryptographicRandomSourceError::unavailable())
    }
}

/// Whole-second observation of the host wall clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemWallClock;

impl WallClockSourcePort for SystemWallClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(WallClockSourceError::from_source)?;
        UnixTimeV1::new(elapsed.as_secs()).map_err(WallClockSourceError::from_source)
    }
}
