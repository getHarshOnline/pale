//! Read local sources, observe filesystem entries, and apply admitted file changes.
//!
//! This adapter implements the source and machine-filesystem ports of
//! `pale-ports` on top of `pale-core`. `pale-sdk` composes it for local work,
//! and `pale-adapter-uds` and the macOS signing adapter reuse its protected
//! reader.
//!
//! [`LocalSourceFilesystem`] captures source files under caller-supplied limits.
//! [`LocalMachineFilesystem`] reports fresh path observations without granting
//! permission to mutate them. Separate stores retain session records, evidence,
//! and service-manager stop markers.
//!
//! Confined writes require Unix directory descriptors for no-follow descendant
//! resolution, exact permission modes, and parent-directory synchronization.
//! Other platforms fail at [`ConfinedRoot::open`] rather than weaken those
//! guarantees. Relative-path parsing alone does not provide confinement.
//!
//! Reconciliation authenticates ownership and payloads before mutation, then
//! rechecks bytes and mode at each effect boundary. Individual publications are
//! atomic and synchronized with their parent directory. Multi-file recovery is
//! in-process only, uses recorded pre-state, and coordinates only cooperating
//! writers.

mod apply;
mod baseline;
mod build_sizes;
mod confine;
mod errors;
mod file_transaction;
mod local_effect;
mod lock;
mod machine_filesystem;
mod materialization_state;
#[cfg(unix)]
mod private_configuration;
mod reconcile;
mod remove;
mod session;
mod source;
mod source_guidance;
mod stop_sentinel;

pub use source_guidance::{
    plan_source_guidance_recovery, prepare_source_guidance_write, AppliedSourceGuidanceWrite,
    PreparedSourceGuidanceWrite, SourceGuidancePublicationFailure,
    SourceGuidancePublicationFailureState, SourceGuidanceRecoveryDirectory,
    SourceGuidanceRecoveryDirectoryState, SourceGuidanceRecoveryEvidence,
    SourceGuidanceRecoveryFile, SourceGuidanceRecoveryIntent, SourceGuidanceRecoveryObservation,
    SourceGuidanceRecoveryPlan, SourceGuidanceWriteObservation, SOURCE_GUIDANCE_LOCK_PATH,
    SOURCE_GUIDANCE_RECOVERY_PATH,
};

pub use apply::{apply_admitted, apply_effect};
pub use baseline::{owned_surface_matches, read_baseline};
pub use build_sizes::{measure_build_tree, BuildSizeObservation};
pub use confine::{
    ConfinedDirectoryEntry, ConfinedEntryKind, ConfinedLocation, ConfinedRoot, ConfinedTreeEntry,
    DirectoryLimit, ExistingFile, ExistingFileMetadata, ReadLimit, TraversalLimit,
    MATERIALIZED_SURFACE_READ_LIMIT,
};
pub use errors::{EffectVisibility, FailureSet, FsError};
pub use file_transaction::{
    commit, rollback, AppliedFileTransaction, AppliedFileTransactionObservation,
};
pub use local_effect::{
    observe_machine_local_file, prepare_machine_local_file, AppliedMachineLocalFile,
    MachineLocalFileIntent, MachineLocalFileObservation, PreparedMachineLocalFile,
};
pub use lock::{acquire_advisory_lock, AdvisoryLock, AdvisoryLockEffect, AdvisoryLockFailure};
pub use machine_filesystem::LocalMachineFilesystem;
pub use materialization_state::{read_materialization_state, target_root_sha256};
#[cfg(unix)]
pub use private_configuration::{
    capture_private_configuration, OwnerPrivateDirectory, PrivateConfigurationError,
};
pub use remove::{remove_admitted, remove_effect};
pub use session::{DurableEvidenceStore, SessionStore};
pub use source::LocalSourceFilesystem;
pub use stop_sentinel::{StopSentinelCustody, StopSentinelDisposition, StopSentinelObservation};
