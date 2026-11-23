//! SQLite-WAL realization of the Pale control-storage source contract.
//!
//! This crate implements the custody ports of `pale-ports` on top of
//! `pale-core`; `pale-sdk` builds on it for local control storage. It owns
//! physical `SQLite` layout, source verification, initial store creation, open
//! classification, and deployment-only custody of opaque authority bytes.
//! Portable trust, journal, checkpoint, recovery, and writer-fence meaning
//! remains outside the adapter. In particular, a database that passes
//! structural inspection is not runtime-admitted while its complete `SQLite`
//! and authority-journal profile still has measurement-owned fields.
//! [`inspect_control_store`] exposes that physical classification without
//! creating canonical descendants, retaining a connection, or opening a
//! resident service.
//!
//! # Structural initialization
//!
//! Physical initialization remains crate-private while the measured
//! durability profile is unresolved. Canonical principal and deployment
//! locations are derived internally from an already-selected retained runtime
//! root and typed raw binding coordinates; no database pathname is an authority.
//!
//! ```
//! use pale_adapter_control_storage::verify_embedded_storage_sources;
//!
//! let sources = verify_embedded_storage_sources()?;
//! assert!(!sources.is_runtime_profile_selected());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod accumulator;
#[cfg(any(test, feature = "bench"))]
mod authority;
#[cfg(test)]
#[path = "../tests/support/backup.rs"]
mod backup_test_support;
#[cfg(feature = "bench")]
mod benchmark;
mod bootstrap;
#[cfg(any(test, feature = "bench"))]
mod candidate;
mod cold_start;
mod deployment;
mod deployment_initialization;
mod effect_case;
mod error;
mod event;
mod frame;
mod inspection;
mod location;
mod operation;
mod principal_trust;
mod producer;
mod profile;
mod schema;
#[cfg(any(test, feature = "bench"))]
mod session_lease_custody;
mod sources;
mod store;
mod writer;
mod writer_lock;

#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;

#[cfg(any(test, feature = "resident-bench"))]
#[path = "../tests/support/resident.rs"]
mod resident_fixture;

#[cfg(test)]
#[path = "../tests/internal/writer.rs"]
mod writer_tests;

pub use bootstrap::SqlitePrincipalBootstrapCustody;
pub use cold_start::{
    SqliteDeploymentFinalizationSetReader, SqliteDeploymentGenesisReader,
    SqlitePrincipalColdStartReader,
};
pub use deployment_initialization::{
    SqliteDeploymentControlStoreGenesisCustody, SqliteDeploymentFinalizationCustody,
};
pub use effect_case::SqliteEffectCaseCustody;
pub use error::ControlSqliteError;
pub use event::SqliteEventCustody;
pub use inspection::{
    inspect_control_store, ControlStoreInspection, ControlStoreInspectionRejection,
    ControlStoreInspectionRequest, ControlStoreInspectionState,
};
pub use operation::SqliteOperationCustody;
pub use principal_trust::checkpoint::SqlitePrincipalCheckpointPreparation;
pub use principal_trust::read::SqlitePrincipalTrustReader;
pub use producer::SqlitePrincipalProducerIncarnationCustody;
pub use profile::{
    AuthorityJournalProfileV1, BackupRestorePolicyV1, ControlStorageProfileRequirement,
    SqliteProfileV1,
};
pub use sources::{
    verify_embedded_storage_sources, ControlStoreKind, VerifiedMigrationSource,
    VerifiedStorageSources, VerifiedStoreSource,
};
pub use writer::SqlitePrincipalResidentWriterCustody;
pub use writer_lock::{acquire_principal_writer_lock, PrincipalWriterLock};

#[cfg(test)]
#[path = "../tests/internal/principal_writer_lock.rs"]
mod principal_writer_lock_tests;
/// Run the non-default SQLite benchmark harness as unadmitted development evidence.
///
/// This symbol exists only with the `bench` feature. It does not expose a store,
/// select a storage profile, admit runtime assembly, or authorize release use.
#[cfg(feature = "bench")]
#[doc(hidden)]
pub fn run_control_sqlite_benchmark() -> std::process::ExitCode {
    benchmark::run()
}

/// Run assembled resident characterization over the unadmitted `SQLite` candidate.
///
/// This feature-gated harness exercises application behavior but does not admit
/// a storage profile, resident process, native effect, acceptance budget, or release.
#[cfg(feature = "resident-bench")]
#[doc(hidden)]
pub fn run_resident_application_benchmark() -> std::process::ExitCode {
    benchmark::resident_application::run()
}
