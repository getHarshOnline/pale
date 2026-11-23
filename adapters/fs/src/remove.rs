//! Admitted full-removal entry over the shared state-reconciliation transaction.

use std::collections::BTreeMap;
use std::path::Path;

use crate::errors::FsError;
use crate::file_transaction::AppliedFileTransaction;
use crate::reconcile::reconcile;
use pale_core::admission::{AdmittedPlan, PlanKind};
use pale_core::records::MaterializationContractIdentity;
use pale_ports::AdmittedMaterializationEffect;

/// Apply an admitted removal and retain its recovery state and advisory target lock.
///
/// Effects are already visible on success. Persist required evidence before
/// [`crate::commit`], or call [`crate::rollback`] on downstream failure. Recovery
/// cannot span a process crash.
///
/// # Errors
///
/// Rejects a wrong plan class, unsafe or changed roots, missing or modified owned
/// state, and any removal or recovery failure. A recovery error means the target
/// state is unknown and requires reconciliation.
pub fn remove_admitted(
    root: &Path,
    lineage_root: &Path,
    admitted: &AdmittedPlan,
    contract_identity: &MaterializationContractIdentity,
) -> Result<AppliedFileTransaction, FsError> {
    if admitted.kind() != PlanKind::Remove {
        return Err(FsError::WrongPlanKind {
            actual: admitted.kind(),
            expected: PlanKind::Remove,
        });
    }
    reconcile(
        root,
        lineage_root,
        admitted,
        &BTreeMap::<String, String>::new(),
        contract_identity,
    )
}

/// Remove surfaces through one API-admitted opaque materialization capability.
///
/// # Errors
///
/// Returns the same bounded filesystem failures as [`remove_admitted`].
pub fn remove_effect(
    root: &Path,
    lineage_root: &Path,
    effect: &AdmittedMaterializationEffect,
) -> Result<AppliedFileTransaction, FsError> {
    remove_admitted(
        root,
        lineage_root,
        effect.admitted_plan(),
        effect.contract_identity(),
    )
}
