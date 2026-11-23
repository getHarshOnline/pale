//! Confined source-guidance publication and private recovery custody.
//!
//! This adapter consumes the application's opaque reviewed-plan admission.
//! Source guidance is not a runtime generation, provider grant or saved plan.
//! File leaves are atomic; the batch is not. A durable private journal precedes
//! target effects and blocks another write until an interrupted run is reconciled.
//! Cold recovery is a separate admission path, not implicit retry or adoption.

mod journal;
mod recovery;
mod transaction;

pub use recovery::{
    plan_source_guidance_recovery, SourceGuidanceRecoveryDirectory,
    SourceGuidanceRecoveryDirectoryState, SourceGuidanceRecoveryEvidence,
    SourceGuidanceRecoveryFile, SourceGuidanceRecoveryIntent, SourceGuidanceRecoveryObservation,
    SourceGuidanceRecoveryPlan,
};

pub use transaction::{
    prepare_source_guidance_write, AppliedSourceGuidanceWrite, PreparedSourceGuidanceWrite,
    SourceGuidancePublicationFailure, SourceGuidancePublicationFailureState,
    SourceGuidanceWriteObservation,
};

/// Persistent root-level lock shared by cooperating source-guidance writers.
///
/// Retained after removal to avoid unlink/recreate lock races. It grants no
/// runtime authority and creates none of the observed target's ancestor paths.
pub const SOURCE_GUIDANCE_LOCK_PATH: &str = ".pale-source-guidance.lock";
/// Private recovery journal containing before-images of the selected project files.
///
/// Created owner-only before the first target effect and removed after a confirmed
/// commit or restoration. A leftover journal blocks another source-guidance write.
pub const SOURCE_GUIDANCE_RECOVERY_PATH: &str = ".pale-source-guidance-recovery.json";
