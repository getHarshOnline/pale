//! Full removal planning from the last trusted ownership state.

use pale_core::records::{
    AgentSurfacePlanV1, BaselineEntry, BaselineStateV1, MaterializationStateTransitionV1,
    MaterializationStateV1, Operation, SurfaceOperationTransitionV1, SurfaceOwnership,
};

use super::state::index_state;
use super::{
    assemble, deferred_effects, PlanAssemblyInput, PlanParts, PreparedSurfaceSet,
    RepositoryMaterializationInput,
};
use crate::plan::{PlanError, PlanKind, DEFAULT_MODE};

/// Build removal operations solely from trusted prior ownership state.
///
/// Current render data contributes deferred-effect metadata and target
/// validation, but does not authorize deletion of a path absent from prior state.
///
/// # Errors
///
/// Returns [`PlanError::RepositoryMutationDenied`] when the selected
/// execution-workspace edge is read-only. Other [`PlanError`] variants report
/// invalid input identity, baseline, prior ownership state, or a derived
/// removal plan that violates the materialization contract.
pub(super) fn build_reconciled_remove_plan(
    input: RepositoryMaterializationInput<'_>,
    prior_state: Option<&MaterializationStateV1>,
    baseline: Vec<BaselineEntry>,
) -> Result<AgentSurfacePlanV1, PlanError> {
    input.require_repository_mutation()?;
    let RepositoryMaterializationInput {
        identity,
        render,
        package_closure,
        normative_pack_closure,
        repository,
        manifest_digest,
        package_semantic_ir_digest,
        schema_registry,
        one_shot_compilation_digest,
    } = input;
    let repository_id = &repository.id;
    let prepared = PreparedSurfaceSet::for_materialization(render, &baseline)?;
    let (prior_identity, prior) = index_state(prior_state, repository_id, schema_registry)?;
    let paths =
        super::observation_target_set(PlanKind::Remove, prior.keys().copied(), std::iter::empty());
    prepared.require_exact_baseline(&paths)?;
    let operations = prior
        .values()
        .map(|surface| Operation {
            rel_path: surface.rel_path.clone(),
            transition: match surface.ownership {
                SurfaceOwnership::ManagedBlock => SurfaceOperationTransitionV1::Splice {
                    prior_owned_sha256: surface.sha256.clone(),
                },
                SurfaceOwnership::WholeFile => SurfaceOperationTransitionV1::Delete {
                    prior_owned_sha256: surface.sha256.clone(),
                },
            },
            kind: surface.kind.clone(),
            mode: prepared
                .baseline_at(&surface.rel_path)
                .and_then(|entry| match &entry.observation {
                    BaselineStateV1::Present { mode, .. } => Some(*mode),
                    BaselineStateV1::Absent {} => None,
                })
                .unwrap_or(DEFAULT_MODE),
            committable: surface.committable,
        })
        .collect();
    let effects = deferred_effects(&prepared);
    drop(prepared);
    let transition = match prior_identity {
        Some(prior) => MaterializationStateTransitionV1::Remove { prior },
        None => MaterializationStateTransitionV1::RemoveUnowned {},
    };
    assemble(
        PlanAssemblyInput {
            identity,
            manifest_digest,
            package_closure,
            normative_pack_closure,
            repository_id,
            package_semantic_ir_digest,
            schema_registry,
            one_shot_compilation_digest,
        },
        PlanParts {
            operations,
            baseline,
            deferred_effects: effects,
            transition,
        },
    )
}
