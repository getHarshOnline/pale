//! Plan the changes that write or remove generated repository guidance, check a
//! saved plan against a freshly computed one, and record what happened.
//!
//! This module builds on `compile`, `identity` and `render`; `compile`, `failure`
//! and `resident` build on it.
//!
//! Every mutation presents immutable verification evidence returned by
//! [`pale_core::admission::admit_plan`]. Admission verifies the saved document and
//! requires equality with a freshly computed plan of the same class. The admitted
//! plan is not itself a durable, linear, or one-use execution authorization.

use pale_core::records::UnixFileMode;
use serde::Serialize;

// These identities enter manifest and plan canonical preimages. Keep their
// meanings separate from executable releases and resident-operation admission.
/// Compiler identity inherited from the distribution contract.
pub(crate) const COMPILER_VERSION: &str = crate::compile::DISTRIBUTION_COMPILER_ID;
/// Renderer identity sealed into materialization manifests and plans.
pub(crate) const RENDERER_VERSION: &str = "pale-surface-renderer/1";
/// Marker proving that plan construction requires an explicit local operator.
pub(crate) const OPERATION_MARKER: &str = "local_explicit_operator";
/// Honest admission state while no resident authorization mechanism exists.
pub(crate) const ADMISSION_UNAVAILABLE: &str =
    "authenticated-resident-runtime-admission-unavailable";
// Generated repository files are ordinary content unless a surface contract
// deliberately selects a different mode at the filesystem adapter boundary.
/// Default Unix mode for generated, non-secret repository content.
pub(crate) const DEFAULT_MODE: UnixFileMode = UnixFileMode::DEFAULT_FILE;

mod admission;
mod errors;
mod manifest;
mod materialization;
mod result;
pub mod source_guidance;

pub use admission::{
    admit_saved_materialization_plan, SavedPlanAdmissionError, SAVED_PLAN_BYTE_LIMIT,
};
pub use errors::PlanError;
pub(crate) use manifest::build_manifest_with_identity;
pub(crate) use materialization::desired_materialization_state;
pub use materialization::{
    bind_machine_local_effects, compare_reconciled_materialization, is_filesystem_target,
    local_only_materialization_targets, materialization_contract_identity,
    BoundMaterializationPlan, MaterializationDriftReason, MaterializationIntent,
    MaterializationOwnershipChange, MaterializationPayload, MaterializationPayloads,
    RepositoryMaterializationComparison, RepositoryMaterializationInput,
    RepositoryMaterializationPlan, RepositoryMaterializationStatus,
};
pub use pale_core::admission::PlanKind;
pub use result::{
    build_run_result, build_success_receipt_with_lineage, decide_materialization_settlement,
    AdmittedRunResult, AdmittedSuccessReceipt, MaterializationLineageEvidence,
    MaterializationSettlement, ReceiptPublicationOutcome, RunTargetEvidence,
};

/// Reject a freshly built record when its Rust representation and canonical
/// schema diverge.
fn validate<T: Serialize>(record: &T, schema_id: &str) -> Result<(), PlanError> {
    Ok(pale_contracts::validate_record(record, schema_id)?)
}
