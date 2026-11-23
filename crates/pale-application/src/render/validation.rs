//! Relational admission for render inputs and complete output sets.

use std::collections::BTreeSet;

use pale_core::records::{
    EffectiveProjectProfileV1, ExecutionPathWitnessV1, ExperienceCompositionPlanV1,
    LogicalRepositoryIdV1, ResolvedCapabilityRequirementPolicyV1,
};

use super::{
    ExecutionPathSupport, GeneratedSurface, MachineAdmission, ProjectRenderInput, RenderError,
};
use crate::semantic::{
    CapabilityObligation, CapabilityObligationDisposition, OperationObligationState,
    ProviderProcedureObligation, ResolvedCapability, ResolvedOperationMechanics,
    ResolvedProviderSelection,
};

/// Check repository-specific relationships against the admitted immutable owners.
///
/// The input's identity capability already verifies both complete records.
/// This check retains run-policy, repository, path, session and Package-obligation
/// laws; identity reuse does not admit any of those relationships by itself.
pub(super) fn validate_render_input(
    input: &ProjectRenderInput<'_>,
    machine_admission: &MachineAdmission,
) -> Result<(), RenderError> {
    let profile = input.identity.profile_record();
    let instance = input.identity.instance_record();
    profile
        .run_policy
        .validate_contract()
        .map_err(|error| RenderError::invalid_input_error("run_policy", error))?;

    let Some(declared_repository) = profile
        .repositories
        .iter()
        .find(|repository| repository.id == input.repository.id)
    else {
        return Err(RenderError::invalid_input(
            "repository",
            format!(
                "repository `{}` is not selected by the effective profile",
                input.repository.id
            ),
        ));
    };
    if declared_repository != input.repository {
        return Err(RenderError::invalid_input(
            "repository",
            format!(
                "repository `{}` differs from its effective-profile declaration",
                input.repository.id
            ),
        ));
    }
    if !instance.repositories.contains_key(&input.repository.id) {
        return Err(RenderError::invalid_input(
            "instance.repositories",
            format!(
                "resolved instance omits repository `{}`",
                input.repository.id
            ),
        ));
    }
    if profile.execution_paths.is_empty() {
        return Err(RenderError::invalid_input(
            "execution_paths",
            "at least one neutral execution-path selection is required",
        ));
    }
    let selected_path_ids = profile
        .execution_paths
        .iter()
        .map(|path| &path.id)
        .collect::<BTreeSet<_>>();
    if selected_path_ids.len() != profile.execution_paths.len() {
        return Err(RenderError::invalid_input(
            "execution_paths",
            "the effective profile contains a duplicate execution-path identity",
        ));
    }
    for path in &profile.execution_paths {
        let witness =
            ExecutionPathWitnessV1::from_compiled_plan(path, input.experience_composition_plan)
                .map_err(|error| {
                    RenderError::invalid_input_record_error(
                        "execution_paths",
                        path.id().to_string(),
                        error,
                    )
                })?;
        let provider_binding = witness.terminal_provider_binding.as_ref().ok_or_else(|| {
            RenderError::invalid_input(
                "execution_paths",
                format!(
                    "execution path `{}` has no terminal provider binding",
                    path.id()
                ),
            )
        })?;
        machine_admission.projected_harness_binding(instance, provider_binding)?;
    }
    if let Some(session) = &profile.session {
        if session.activation_roots.is_empty() {
            return Err(RenderError::invalid_input(
                "session.activation_roots",
                "at least one activation repository is required",
            ));
        }
        let mut activation_roots = BTreeSet::new();
        for root in &session.activation_roots {
            if !profile
                .repositories
                .iter()
                .any(|repository| repository.id == *root)
            {
                return Err(RenderError::invalid_input(
                    "session.activation_roots",
                    format!("activation root `{root}` is not a selected repository"),
                ));
            }
            if !activation_roots.insert(root) {
                return Err(RenderError::invalid_input(
                    "session.activation_roots",
                    format!("activation root `{root}` occurs more than once"),
                ));
            }
        }
        for root in &session.additional_roots {
            if activation_roots.contains(root) {
                return Err(RenderError::invalid_input(
                    "session.additional_roots",
                    format!("activation root `{root}` cannot also be an additional root"),
                ));
            }
        }
        let mut active_path_ids = BTreeSet::new();
        for path_id in &session.activation_path_ids {
            if !selected_path_ids.contains(path_id) {
                return Err(RenderError::invalid_input(
                    "session.activation_path_ids",
                    format!("execution path `{path_id}` is not selected by the profile"),
                ));
            }
            if !active_path_ids.insert(path_id) {
                return Err(RenderError::invalid_input(
                    "session.activation_path_ids",
                    format!("execution path `{path_id}` is selected more than once"),
                ));
            }
        }
    }

    let rendered_families = input
        .semantics
        .policy_families
        .iter()
        .map(|family| &family.id)
        .collect::<BTreeSet<_>>();
    let declared_families = input.repository.families.iter().collect::<BTreeSet<_>>();
    // The compiler closes selected resources over required dependencies, including
    // policy families activated by a role's governing order. The profile retains
    // the explicit selections; the semantic plan carries that admitted closure.
    if !declared_families.is_subset(&rendered_families)
        || rendered_families.len() != input.semantics.policy_families.len()
    {
        return Err(RenderError::invalid_input(
            "repository.families",
            "content-bearing policy-family closure omits a repository selection or repeats a family",
        ));
    }

    validate_package_obligations(input)?;

    Ok(())
}

/// Verify that one renderer received the exact compiler-owned Package closure.
fn validate_package_obligations(input: &ProjectRenderInput<'_>) -> Result<(), RenderError> {
    let closure = input.package_obligations;
    if closure.repository_index != input.repository_index {
        return Err(RenderError::invalid_input(
            "package obligations",
            "repository position differs from the effective-profile projection",
        ));
    }
    if closure.operations.len() != input.semantics.operations.len()
        || !closure
            .operations
            .windows(2)
            .all(|pair| pair[0].operation < pair[1].operation)
    {
        return Err(RenderError::invalid_input(
            "package obligations",
            "operation closure is incomplete, duplicated, or non-canonical",
        ));
    }

    for operation in &input.semantics.operations {
        let obligation = closure.operation(&operation.record.id).ok_or_else(|| {
            RenderError::invalid_input(
                "package obligations",
                format!(
                    "operation `{}` has no pre-render obligation",
                    crate::semantic::resource_label(&operation.record.id)
                ),
            )
        })?;
        let state_matches = match (&obligation.state, &operation.mechanics) {
            (OperationObligationState::Unbound, ResolvedOperationMechanics::Unbound) => true,
            (
                OperationObligationState::SemanticOnly { selection },
                ResolvedOperationMechanics::SemanticOnly {
                    selection: expected,
                },
            ) => selection == expected,
            (
                OperationObligationState::CapabilityBearing {
                    selection,
                    capabilities,
                },
                ResolvedOperationMechanics::CapabilityBearing {
                    selection: expected_selection,
                    capabilities: expected_capabilities,
                },
            ) => {
                selection == expected_selection
                    && capabilities.len() == expected_capabilities.len()
                    && capabilities.iter().all(|capability| {
                        expected_capabilities.iter().any(|expected| {
                            capability_matches(
                                capability,
                                expected,
                                &operation.record.id,
                                &input.semantics.recipes,
                            )
                        })
                    })
            }
            _ => false,
        };
        if !state_matches {
            return Err(RenderError::invalid_input(
                "package obligations",
                format!(
                    "operation `{}` differs from its pre-render obligation",
                    crate::semantic::resource_label(&operation.record.id)
                ),
            ));
        }
    }
    Ok(())
}

fn capability_matches(
    obligation: &CapabilityObligation,
    expected: &ResolvedCapability,
    operation: &pale_core::records::PackageResourceRefV1,
    recipes: &[crate::semantic::ResolvedRecipe],
) -> bool {
    obligation.subject.operation == *operation
        && obligation.subject.requirement == expected.requirement_id
        && obligation.capability == expected.capability_id
        && obligation.effect == expected.effect_class
        && obligation.failure_posture == expected.failure_posture
        && disposition_matches(&obligation.disposition, expected)
        && procedure_matches(&obligation.procedure, expected, recipes)
}

fn disposition_matches(
    disposition: &CapabilityObligationDisposition,
    expected: &ResolvedCapability,
) -> bool {
    match (&expected.provider, &expected.policy, disposition) {
        (
            ResolvedProviderSelection::Bound(expected),
            _,
            CapabilityObligationDisposition::Bound(observed),
        ) => expected == observed,
        (
            ResolvedProviderSelection::Unbound(expected),
            ResolvedCapabilityRequirementPolicyV1::RequiredBlock,
            CapabilityObligationDisposition::Blocked { reason },
        ) => expected == reason,
        (
            ResolvedProviderSelection::Unbound(expected),
            ResolvedCapabilityRequirementPolicyV1::RequiredDegrade {
                fallback,
                degraded_outcome,
            },
            CapabilityObligationDisposition::Degraded {
                reason,
                fallback: observed_fallback,
                degraded_outcome: observed_outcome,
            },
        ) => {
            expected == reason
                && fallback == observed_fallback
                && degraded_outcome == observed_outcome
        }
        (
            ResolvedProviderSelection::Unbound(expected),
            ResolvedCapabilityRequirementPolicyV1::Optional { fallback },
            CapabilityObligationDisposition::Omitted {
                reason,
                fallback: observed_fallback,
            },
        ) => expected == reason && fallback == observed_fallback,
        _ => false,
    }
}

fn procedure_matches(
    procedure: &ProviderProcedureObligation,
    expected: &ResolvedCapability,
    recipes: &[crate::semantic::ResolvedRecipe],
) -> bool {
    match (&expected.recipe, procedure) {
        (None, ProviderProcedureObligation::CapabilityOnly) => true,
        (
            Some(expected_recipe),
            ProviderProcedureObligation::Recipe {
                recipe,
                tool_contract,
                tool_operation,
                required_fields,
                proof_ceiling,
                repository_verification,
            },
        ) => recipes
            .iter()
            .find(|candidate| candidate.resource == *expected_recipe)
            .is_some_and(|candidate| {
                recipe == expected_recipe
                    && candidate.procedure.capability == expected.capability_id
                    && candidate.procedure.effect_class == expected.effect_class
                    && candidate.procedure.tool_contract == *tool_contract
                    && candidate.procedure.tool_operation == *tool_operation
                    && candidate.procedure.evidence.required_fields == *required_fields
                    && candidate.procedure.evidence.proof_ceiling == *proof_ceiling
                    && crate::semantic::RepositoryVerificationDisposition::from(
                        candidate.procedure.evidence.repository_verifier,
                    ) == *repository_verification
            }),
        _ => false,
    }
}

/// Reject incomplete, escaped, duplicated, or contradictory render results.
pub(super) fn validate_render_output(
    profile: &EffectiveProjectProfileV1,
    repository_id: &LogicalRepositoryIdV1,
    composition_plan: &ExperienceCompositionPlanV1,
    surfaces: &[GeneratedSurface],
    support: &[ExecutionPathSupport],
) -> Result<(), RenderError> {
    let selected_path_ids = profile
        .execution_paths
        .iter()
        .map(|path| &path.id)
        .collect::<BTreeSet<_>>();
    let mut paths = BTreeSet::new();
    for surface in surfaces {
        if pale_core::paths::SurfaceTarget::parse(&surface.rel_path).is_none() {
            return Err(RenderError::projection(
                "surface set",
                format!("target `{}` is not confined", surface.rel_path),
            ));
        }
        if !paths.insert(surface.rel_path.as_str()) {
            return Err(RenderError::projection(
                "surface set",
                format!("target `{}` is rendered more than once", surface.rel_path),
            ));
        }
        if surface.execution_path_consumers.is_empty() {
            return Err(RenderError::projection(
                "surface set",
                format!(
                    "target `{}` has no execution-path consumer",
                    surface.rel_path
                ),
            ));
        }
        let mut consumers = BTreeSet::new();
        for path_id in &surface.execution_path_consumers {
            if !selected_path_ids.contains(path_id) {
                return Err(RenderError::projection(
                    "surface set",
                    format!(
                        "target `{}` names unselected execution path `{path_id}`",
                        surface.rel_path
                    ),
                ));
            }
            if !consumers.insert(path_id) {
                return Err(RenderError::projection(
                    "surface set",
                    format!(
                        "target `{}` repeats execution path `{path_id}`",
                        surface.rel_path
                    ),
                ));
            }
        }
        if surface.deferred
            && (surface.committable
                || surface
                    .target_note
                    .as_deref()
                    .is_none_or(|note| note.trim().is_empty()))
        {
            return Err(RenderError::projection(
                "surface set",
                format!(
                    "deferred target `{}` must be non-committable and explain its gated placement",
                    surface.rel_path
                ),
            ));
        }
        let policy_ids = surface
            .referenced_policy_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if policy_ids.len() != surface.referenced_policy_ids.len() {
            return Err(RenderError::projection(
                "surface set",
                format!("target `{}` repeats a policy identity", surface.rel_path),
            ));
        }
    }

    if support.len() != profile.execution_paths.len() {
        return Err(RenderError::projection(
            "execution-path support",
            "support rows do not cover every selected execution path exactly once",
        ));
    }
    for (expected_path, row) in profile.execution_paths.iter().zip(support) {
        let expected_witness =
            ExecutionPathWitnessV1::from_compiled_plan(expected_path, composition_plan).map_err(
                |error| {
                    RenderError::invalid_input_record_error(
                        "execution_paths",
                        expected_path.id().to_string(),
                        error,
                    )
                },
            )?;
        if row.execution_path_witness != expected_witness {
            return Err(RenderError::projection(
                "execution-path support",
                format!(
                    "support row for `{}` does not carry its canonical composition-plan witness",
                    expected_path.id()
                ),
            ));
        }
        let activation_selected = profile.session.as_ref().is_some_and(|session| {
            session
                .activation_roots
                .iter()
                .any(|root| root == repository_id)
                && session
                    .activation_path_ids
                    .iter()
                    .any(|active| active == expected_path.id())
        });
        if row.activation_selected != activation_selected || row.note.trim().is_empty() {
            return Err(RenderError::projection(
                "execution-path support",
                format!(
                    "execution path `{}` has contradictory activation-selection evidence",
                    expected_path.id()
                ),
            ));
        }
    }
    Ok(())
}
