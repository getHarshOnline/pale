//! Admitted apply entry over the shared state-reconciliation transaction.

use std::collections::BTreeMap;
use std::path::Path;

use crate::errors::FsError;
use crate::file_transaction::AppliedFileTransaction;
use crate::reconcile::reconcile;
use pale_core::admission::{AdmittedPlan, PlanKind};
use pale_core::records::MaterializationContractIdentity;
use pale_ports::AdmittedMaterializationEffect;

/// Apply an admitted mutation and retain its recovery state and advisory target lock.
///
/// Effects are already visible on success. Persist required evidence before
/// [`crate::commit`], or call [`crate::rollback`] on downstream failure. Recovery
/// cannot span a process crash.
///
/// # Errors
///
/// Rejects a wrong plan class, unsafe or changed roots, untrusted prior state,
/// ownership or payload collisions, and any publication or recovery failure. A
/// recovery error means the target state is unknown and requires reconciliation.
pub fn apply_admitted<T: AsRef<str>>(
    root: &Path,
    lineage_root: &Path,
    admitted: &AdmittedPlan,
    contents: &BTreeMap<String, T>,
    contract_identity: &MaterializationContractIdentity,
) -> Result<AppliedFileTransaction, FsError> {
    if admitted.kind() != PlanKind::Apply {
        return Err(FsError::WrongPlanKind {
            actual: admitted.kind(),
            expected: PlanKind::Apply,
        });
    }
    reconcile(root, lineage_root, admitted, contents, contract_identity)
}

/// Apply one API-admitted opaque materialization capability.
///
/// The adapter consumes the same proof and compiler-retained payloads as
/// [`apply_admitted`] without requiring the composition shell to inspect their
/// application or core representation.
///
/// # Errors
///
/// Returns the same bounded filesystem failures as [`apply_admitted`].
pub fn apply_effect(
    root: &Path,
    lineage_root: &Path,
    effect: &AdmittedMaterializationEffect,
) -> Result<AppliedFileTransaction, FsError> {
    apply_admitted(
        root,
        lineage_root,
        effect.admitted_plan(),
        effect.payloads(),
        effect.contract_identity(),
    )
}
