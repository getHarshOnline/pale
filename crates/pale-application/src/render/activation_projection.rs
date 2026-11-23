//! Repository-hook contributions and inert activation-intent surfaces.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use std::collections::BTreeMap;

use pale_core::records::ExecutionPathSelectionIdV1;

use super::family_content::OrderedFamily;
use super::{
    claude_hook_contribution, claude_hook_wiring, codex_hook_contribution, codex_hook_wiring,
    execution_path_ids, execution_paths_for_harness, family_ids, ActiveHookWiring,
    GeneratedSurface, RenderContext, RenderError, RenderIdentity, SurfaceKind,
};
use crate::hooks::HookRequirements;

/// Return the Pale-owned activation-intent target for one terminal harness.
fn activation_intent_path(harness: TerminalAdapterDescriptor) -> String {
    format!(".pale/hooks/{harness}/activation-intent.json")
}

/// Return the Pale-owned hook-contribution target for one terminal harness.
fn repository_hook_contribution_path(harness: TerminalAdapterDescriptor) -> String {
    format!(".pale/hooks/{harness}/repository-hook-contribution.json")
}

/// Render one inert activation-intent surface for a harness that is not
/// activation-ready.
fn activation_intent_surface(
    harness: TerminalAdapterDescriptor,
    requirements: &HookRequirements,
    consumers: Vec<ExecutionPathSelectionIdV1>,
    ids: Vec<String>,
    note: &'static str,
) -> Result<GeneratedSurface, RenderError> {
    let content = if harness.is_claude_code() {
        claude_hook_wiring(requirements)?
    } else {
        codex_hook_wiring(requirements)?
    };
    Ok(GeneratedSurface::whole_file(
        activation_intent_path(harness),
        SurfaceKind::HookActivationIntent,
        content,
        consumers,
        ids,
        true,
    )
    .with_note(note))
}

/// Explain why a selected activation path cannot emit runnable wiring.
fn activation_block_reason(
    machine_admission_verified: bool,
    harness_binding_present: bool,
    surface_executable_present: bool,
    session_state_present: bool,
    durable_evidence_present: bool,
) -> Option<&'static str> {
    if !machine_admission_verified {
        Some(
            "Activation requires live machine admission; portable or removal reconstruction is inert.",
        )
    } else if !harness_binding_present {
        Some(
            "Activation requires a verified machine-local executable for the exact provider binding.",
        )
    } else if !surface_executable_present {
        Some("Activation requires a verified machine-local Pale executable.")
    } else if !session_state_present {
        Some("Activation requires machine-bound ephemeral session continuity.")
    } else if !durable_evidence_present {
        Some("Activation requires machine-bound recoverable decision evidence.")
    } else {
        None
    }
}

/// Render host activation surfaces without claiming that installation occurred.
pub(super) fn render_activation(
    input: &RenderContext<'_, '_>,
    _identity: &RenderIdentity<'_>,
    requirements_by_path: &BTreeMap<ExecutionPathSelectionIdV1, HookRequirements>,
    by_harness: &BTreeMap<TerminalAdapterDescriptor, Vec<OrderedFamily<'_>>>,
    surfaces: &mut Vec<GeneratedSurface>,
) -> Result<(), RenderError> {
    let machine_admission = input.machine_admission;
    let input = &input.projection;
    let profile = input.identity.profile_record();
    let instance = input.identity.instance_record();
    let repo = input.repository;
    let behaviours = input.lifecycle;
    let Some(session) = profile.session.as_ref() else {
        return Ok(());
    };
    if !session.activation_roots.iter().any(|root| root == &repo.id) {
        return Ok(());
    }
    let surface_executable = instance.surface_executable.as_ref();

    for harness in crate::terminal_adapters::descriptors() {
        let paths =
            execution_paths_for_harness(profile, input.experience_composition_plan, harness)?;
        if paths.is_empty() {
            continue;
        }
        let provider_binding =
            crate::terminal_adapters::for_selection(paths[0], input.experience_composition_plan)
                .map_err(|error| {
                    RenderError::invalid_input_error("execution-path terminal adapter", error)
                })?
                .0;
        for path in &paths[1..] {
            let observed =
                crate::terminal_adapters::for_selection(path, input.experience_composition_plan)
                    .map_err(|error| {
                        RenderError::invalid_input_error("execution-path terminal adapter", error)
                    })?
                    .0;
            if observed != provider_binding {
                return Err(RenderError::projection(
                    "native-harness hook contribution",
                    format!("execution paths reaching {harness} select multiple provider bindings"),
                ));
            }
        }
        let consumers = execution_path_ids(&paths);
        let active = session
            .activation_path_ids
            .iter()
            .filter_map(|active_id| paths.iter().copied().find(|path| path.id() == active_id))
            .collect::<Vec<_>>();
        let ids = by_harness
            .get(&harness)
            .map(|families| family_ids(families))
            .unwrap_or_default();
        let requirements_path = active
            .first()
            .copied()
            .or_else(|| paths.first().copied())
            .ok_or_else(|| {
                RenderError::invalid_input(
                    "execution paths",
                    format!("no selected execution path reaches {harness}"),
                )
            })?;
        let requirements = requirements_by_path
            .get(requirements_path.id())
            .ok_or_else(|| {
                RenderError::projection(
                    "execution-path hook requirements",
                    format!(
                        "execution path `{}` has no resolved hook requirements",
                        requirements_path.id()
                    ),
                )
            })?;
        for path in &paths {
            let observed = requirements_by_path.get(path.id()).ok_or_else(|| {
                RenderError::projection(
                    "execution-path hook requirements",
                    format!(
                        "execution path `{}` has no resolved hook requirements",
                        path.id()
                    ),
                )
            })?;
            if observed != requirements {
                return Err(RenderError::projection(
                    "native-harness hook contribution",
                    format!(
                        "execution paths reaching {harness} require incompatible repository hooks"
                    ),
                ));
            }
        }
        let contribution_path = repository_hook_contribution_path(harness);
        if active.is_empty() {
            surfaces.push(activation_intent_surface(
                harness,
                requirements,
                consumers,
                ids,
                "Inert compiler intent: no activation path is declared for this native harness.",
            )?);
            continue;
        }
        let active_consumers = active
            .iter()
            .map(|path| path.id.clone())
            .collect::<Vec<_>>();
        let harness_binding =
            machine_admission.projected_harness_binding(instance, &provider_binding)?;
        let activation_gap = activation_block_reason(
            machine_admission.is_verified(),
            harness_binding.is_some(),
            surface_executable.is_some(),
            instance.session_state.is_some(),
            instance.durable_evidence.is_some(),
        );
        if let Some(note) = activation_gap {
            surfaces.push(activation_intent_surface(
                harness,
                requirements,
                active_consumers,
                ids,
                note,
            )?);
            continue;
        }
        let surface_executable = surface_executable.ok_or_else(|| {
            RenderError::invalid_input(
                "instance.surface_executable",
                "activation-ready path has no surface executable binding",
            )
        })?;
        let wiring = ActiveHookWiring {
            exec: &surface_executable.exec,
            workspace_id: &profile.workspace_id,
            repo_id: &repo.id,
            harness,
            provider_binding: &provider_binding,
            lifecycle: &session.lifecycle_events,
            requirements,
            behaviours,
        };
        let content = if harness.is_claude_code() {
            claude_hook_contribution(&wiring)?
        } else {
            codex_hook_contribution(&wiring)?
        };
        surfaces.push(
            GeneratedSurface::whole_file(
                contribution_path,
                SurfaceKind::RepositoryHookContribution,
                content,
                active_consumers,
                ids,
                false,
            )
            .with_note(
                "Exact Pale-owned repository-hook contribution; it is not native settings. A separately admitted effect must structurally merge it into native-owner configuration and verify the result.",
            ),
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/internal/render/activation_projection.rs"]
mod tests;
