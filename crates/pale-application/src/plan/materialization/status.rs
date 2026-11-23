//! Read-only materialization comparison over the apply transition algebra.
//!
//! Status derives the same surface and ownership transitions as apply without
//! asserting repository-mutation permission or constructing an admitted plan.
//! Host-specific machine-local observations are attached in a second step and
//! classified here so terminal leaves do not reimplement drift policy.

use std::collections::BTreeSet;

use pale_core::records::{
    BaselineEntry, LogicalRepositoryIdV1, MachineLocalEffectKind, MachineLocalEffectV1,
    MaterializationStateTransitionV1, MaterializationStateV1, Operation, SurfaceAction,
};

use super::apply::prepare_apply_plan;
use super::local::validate_and_sort_machine_local_effects;
use super::{local_only_materialization_targets, RepositoryMaterializationInput};
use crate::plan::PlanError;

/// Ownership-state reason for a read-only materialization difference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaterializationOwnershipChange {
    /// No prior ownership state exists for the desired projection.
    Initialize,
    /// The desired ownership-state digest differs from the prior state.
    Advance,
}

/// One typed reason why observed repository state differs from the projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaterializationDriftReason {
    /// One repository surface requires the stated transition.
    Surface {
        /// Confined repository-relative target.
        relative_path: String,
        /// Transition required to reach desired content and ownership.
        action: SurfaceAction,
    },
    /// The durable ownership record itself must be initialized or advanced.
    OwnershipState {
        /// Required ownership-state transition.
        change: MaterializationOwnershipChange,
    },
    /// One machine-local file requires the stated transition.
    MachineLocal {
        /// Closed effect kind selected by the materialization policy.
        kind: MachineLocalEffectKind,
        /// Path relative to the admitted machine-local root.
        relative_path: String,
        /// Transition required to reach the desired local state.
        action: SurfaceAction,
    },
}

/// Read-only comparison awaiting machine-local host observations.
///
/// The value carries only deterministic surface transitions and the exact
/// local-only target set. It is not serializable, cannot become an
/// [`pale_core::admission::AdmittedPlan`], and grants no mutation authority.
pub struct RepositoryMaterializationComparison {
    repository_id: LogicalRepositoryIdV1,
    operations: Vec<Operation>,
    transition: MaterializationStateTransitionV1,
    local_only_targets: Vec<String>,
}

impl RepositoryMaterializationComparison {
    /// Borrow the exact sorted local-only targets needed by host protection.
    #[must_use]
    pub fn local_only_targets(&self) -> &[String] {
        &self.local_only_targets
    }

    /// Operations whose unchanged ownership transition still requires a byte observation.
    pub fn unchanged_operations(&self) -> impl Iterator<Item = &Operation> {
        self.operations
            .iter()
            .filter(|op| op.transition.action() == SurfaceAction::Unchanged)
    }

    /// Complete status with observed machine-local effects and owned-region mismatches.
    ///
    /// # Errors
    ///
    /// Returns [`PlanError`] when a machine-local effect is malformed,
    /// duplicated, or contradicts its closed effect-kind path law.
    pub fn complete(
        self,
        mut machine_local_effects: Vec<MachineLocalEffectV1>,
        changed_owned_paths: &BTreeSet<String>,
    ) -> Result<RepositoryMaterializationStatus, PlanError> {
        validate_and_sort_machine_local_effects(&mut machine_local_effects)?;
        let mut reasons = self
            .operations
            .into_iter()
            .filter_map(|operation| {
                let action = match operation.transition.action() {
                    SurfaceAction::Unchanged
                        if changed_owned_paths.contains(&operation.rel_path) =>
                    {
                        SurfaceAction::Update
                    }
                    action => action,
                };
                (action != SurfaceAction::Unchanged).then_some(
                    MaterializationDriftReason::Surface {
                        relative_path: operation.rel_path,
                        action,
                    },
                )
            })
            .collect::<Vec<_>>();
        match self.transition {
            MaterializationStateTransitionV1::Initialize { .. } => {
                reasons.push(MaterializationDriftReason::OwnershipState {
                    change: MaterializationOwnershipChange::Initialize,
                });
            }
            MaterializationStateTransitionV1::Advance { prior, next_state }
                if prior.state_digest != next_state.state_digest =>
            {
                reasons.push(MaterializationDriftReason::OwnershipState {
                    change: MaterializationOwnershipChange::Advance,
                });
            }
            MaterializationStateTransitionV1::Advance { .. } => {}
            MaterializationStateTransitionV1::Remove { .. }
            | MaterializationStateTransitionV1::RemoveUnowned {} => {
                return Err(PlanError::PlanInvariant {
                    coordinate: "read-only-status-transition",
                    detail: "apply status comparison produced a removal transition".into(),
                });
            }
        }
        reasons.extend(machine_local_effects.into_iter().filter_map(|effect| {
            let action = effect.transition.action();
            (action != SurfaceAction::Unchanged).then_some(
                MaterializationDriftReason::MachineLocal {
                    kind: effect.kind,
                    relative_path: effect.rel_path,
                    action,
                },
            )
        }));
        Ok(RepositoryMaterializationStatus {
            repository_id: self.repository_id,
            reasons,
        })
    }
}

/// Complete typed result of one read-only repository comparison.
pub struct RepositoryMaterializationStatus {
    repository_id: LogicalRepositoryIdV1,
    reasons: Vec<MaterializationDriftReason>,
}

impl RepositoryMaterializationStatus {
    /// Logical repository whose observed state was compared.
    #[must_use]
    pub const fn repository_id(&self) -> &LogicalRepositoryIdV1 {
        &self.repository_id
    }

    /// Whether repository and machine-local observations match desired state.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.reasons.is_empty()
    }

    /// Typed drift reasons in deterministic surface/ownership/local order.
    #[must_use]
    pub fn reasons(&self) -> &[MaterializationDriftReason] {
        &self.reasons
    }
}

/// Compare observed repository state through the apply transition law.
///
/// Unlike [`RepositoryMaterializationInput::build`], this operation does not
/// require mutation permission and cannot produce a plan record. The caller
/// observes the returned local-only targets, derives exact machine-local
/// effects, and passes them to [`RepositoryMaterializationComparison::complete`].
///
/// # Errors
///
/// Returns [`PlanError`] when prior ownership, baseline evidence, rendered
/// surfaces, or compilation identity is inconsistent.
pub fn compare_reconciled_materialization(
    input: RepositoryMaterializationInput<'_>,
    prior_state: Option<&MaterializationStateV1>,
    baseline: Vec<BaselineEntry>,
) -> Result<RepositoryMaterializationComparison, PlanError> {
    let repository_id = input.repository.id.clone();
    let (_, parts) = prepare_apply_plan(input, prior_state, baseline, false)?;
    let local_only_targets = local_only_materialization_targets(&parts.operations);
    Ok(RepositoryMaterializationComparison {
        repository_id,
        operations: parts.operations,
        transition: parts.transition,
        local_only_targets,
    })
}
