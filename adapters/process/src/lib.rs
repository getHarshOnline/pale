//! Verify executables, supervise child processes, and read host time and randomness.
//!
//! This adapter implements the executable-identity, process and clock and
//! randomness ports of `pale-ports` on top of `pale-core`; `pale-sdk` composes
//! it for local work. It owns executable retention, no-follow byte
//! verification, process groups, bounded stream capture, timeout, termination,
//! and reaping. Version and identity payloads, semantic identities, signing
//! authority, execution-path admission and native effects stay with their
//! owners.

mod clock_and_randomness;
mod executable;
mod probe;
mod supervision;
mod termination;

pub use clock_and_randomness::{OsCryptographicRandom, SystemWallClock};
pub use probe::LocalProcessProbe;
pub use supervision::LocalProcessSupervisor;
