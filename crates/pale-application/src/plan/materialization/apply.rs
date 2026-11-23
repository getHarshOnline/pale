//! Reconciled apply planning from prior ownership state to current render.

use std::collections::BTreeMap;

use pale_core::records::{
    AgentSurfacePlanV1, BaselineEntry, BaselineStateV1, MaterializationStateTransitionV1,
    MaterializationStateV1, Operation, SurfaceOperationTransitionV1, SurfaceOwnership,
};

use super::state::{desired_state, index_state};
use super::{
    assemble, deferred_effects, PlanAssemblyInput, PlanParts, PreparedSurfaceSet,
    RepositoryMaterializationInput,
};
use crate::plan::{PlanError, PlanKind, DEFAULT_MODE};

/// Reconcile a trusted prior ownership state with the current rendered target set.
///
/// The result is a deterministic description only: paths are validated, prior
/// and desired identities are compared, and operations are sorted before the
/// plan is sealed. No target filesystem mutation occurs here.
///
/// # Errors
///
/// Returns [`PlanError::RepositoryMutationDenied`] when the selected
/// execution-workspace edge is read-only. Other [`PlanError`] variants report
/// invalid input identity, baseline, prior state, rendered surfaces, or a
/// derived plan that violates the materialization contract.
pub(super) fn build_reconciled_apply_plan(
    input: RepositoryMaterializationInput<'_>,
    prior_state: Option<&MaterializationStateV1>,
    baseline: Vec<BaselineEntry>,
) -> Result<AgentSurfacePlanV1, PlanError> {
    build_apply_plan(input, prior_state, baseline, false)
}

/// Build an explicit first-ownership plan for pre-existing generated targets.
///
/// This is deliberately separate from ordinary apply planning. An adoption
/// plan records `adopt` only where the admitted baseline proves that a target
/// already exists. The filesystem boundary then requires an exact existing
/// managed block before adopting block ownership; it never reclassifies an
/// arbitrary user file. Absent targets remain `create`.
///
/// # Errors
///
/// Returns [`PlanError::RepositoryMutationDenied`] when the selected
/// execution-workspace edge is read-only. Other [`PlanError`] variants report
/// invalid input identity, baseline, prior state, adoption evidence, or a
/// derived plan that violates the materialization contract.
pub(super) fn build_reconciled_adoption_plan(
    input: RepositoryMaterializationInput<'_>,
    prior_state: Option<&MaterializationStateV1>,
    baseline: Vec<BaselineEntry>,
) -> Result<AgentSurfacePlanV1, PlanError> {
    build_apply_plan(input, prior_state, baseline, true)
}

/// Build the shared apply/adoption transition without performing filesystem effects.
///
/// `adopt_unowned_surfaces` changes only whether matching unowned bytes may enter
/// Pale ownership; every target still requires an exact observed baseline.
fn build_apply_plan(
    input: RepositoryMaterializationInput<'_>,
    prior_state: Option<&MaterializationStateV1>,
    baseline: Vec<BaselineEntry>,
    adopt_unowned_surfaces: bool,
) -> Result<AgentSurfacePlanV1, PlanError> {
    input.require_repository_mutation()?;
    let (assembly, parts) =
        prepare_apply_plan(input, prior_state, baseline, adopt_unowned_surfaces)?;
    assemble(assembly, parts)
}

/// Derive apply/adoption plan parts without asserting mutation permission.
///
/// The read-only status algebra uses this function to compare current observed
/// state with the desired projection. Only the public plan builders call
/// [`RepositoryMaterializationInput::require_repository_mutation`] before the
/// returned parts can become an operator plan.
pub(super) fn prepare_apply_plan<'a>(
    input: RepositoryMaterializationInput<'a>,
    prior_state: Option<&MaterializationStateV1>,
    baseline: Vec<BaselineEntry>,
    adopt_unowned_surfaces: bool,
) -> Result<(PlanAssemblyInput<'a>, PlanParts), PlanError> {
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
    let next_state = desired_state(
        identity,
        &prepared,
        repository_id,
        manifest_digest,
        schema_registry,
        one_shot_compilation_digest,
    )?;
    let (prior_identity, prior) = index_state(prior_state, repository_id, schema_registry)?;
    let desired = next_state
        .surfaces
        .iter()
        .map(|surface| (surface.rel_path.as_str(), surface))
        .collect::<BTreeMap<_, _>>();
    let paths = super::observation_target_set(
        PlanKind::Apply,
        prior.keys().copied(),
        desired.keys().copied(),
    );
    prepared.require_exact_baseline(&paths)?;
    let mut operations = Vec::with_capacity(paths.len());
    for rel_path in paths {
        let operation = match (prior.get(rel_path), desired.get(rel_path)) {
            (None, Some(next)) => Operation {
                rel_path: rel_path.into(),
                transition: if adopt_unowned_surfaces
                    && prepared.baseline_at(rel_path).is_some_and(|entry| {
                        matches!(&entry.observation, BaselineStateV1::Present { .. })
                    }) {
                    SurfaceOperationTransitionV1::Adopt {
                        ownership: next.ownership,
                        desired_sha256: next.sha256.clone(),
                    }
                } else {
                    SurfaceOperationTransitionV1::Create {
                        ownership: next.ownership,
                        desired_sha256: next.sha256.clone(),
                    }
                },
                kind: next.kind.clone(),
                mode: DEFAULT_MODE,
                committable: next.committable,
            },
            (Some(previous), None) => Operation {
                rel_path: rel_path.into(),
                transition: match previous.ownership {
                    SurfaceOwnership::ManagedBlock => SurfaceOperationTransitionV1::Splice {
                        prior_owned_sha256: previous.sha256.clone(),
                    },
                    SurfaceOwnership::WholeFile => SurfaceOperationTransitionV1::Delete {
                        prior_owned_sha256: previous.sha256.clone(),
                    },
                },
                kind: previous.kind.clone(),
                mode: prepared
                    .baseline_at(rel_path)
                    .and_then(|entry| match &entry.observation {
                        BaselineStateV1::Present { mode, .. } => Some(*mode),
                        BaselineStateV1::Absent {} => None,
                    })
                    .unwrap_or(DEFAULT_MODE),
                committable: previous.committable,
            },
            (Some(previous), Some(next)) => {
                if previous.ownership != next.ownership {
                    return Err(PlanError::OwnershipTransition(rel_path.into()));
                }
                Operation {
                    rel_path: rel_path.into(),
                    transition: if previous.sha256 == next.sha256 {
                        SurfaceOperationTransitionV1::Unchanged {
                            ownership: next.ownership,
                            owned_sha256: next.sha256.clone(),
                        }
                    } else {
                        SurfaceOperationTransitionV1::Update {
                            ownership: next.ownership,
                            desired_sha256: next.sha256.clone(),
                        }
                    },
                    kind: next.kind.clone(),
                    mode: DEFAULT_MODE,
                    committable: next.committable,
                }
            }
            (None, None) => {
                return Err(PlanError::PlanInvariant {
                    coordinate: "apply-path-union",
                    detail: format!(
                        "materialization path `{rel_path}` has no prior or desired surface"
                    ),
                });
            }
        };
        operations.push(operation);
    }
    let effects = deferred_effects(&prepared);
    drop(prepared);
    let transition = match prior_identity {
        Some(prior) => MaterializationStateTransitionV1::Advance { prior, next_state },
        None => MaterializationStateTransitionV1::Initialize { next_state },
    };
    Ok((
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
    ))
}
