//! Qualify execution paths using compiler proof and selected machine evidence.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use pale_core::records::{
    EffectiveProjectProfileV1, ExecutionPathSelectionIdV1, ExecutionPathSelectionV1,
    ExecutionPathWitnessV1, ExperienceCompositionPlanV1, LogicalRepositoryIdV1,
    ResolvedProjectInstanceV1,
};

use super::{ExecutionPathSupport, MachineAdmission};
use crate::hooks::HookRequirements;

/// Shared admitted context for qualifying every selected execution path.
pub(super) struct ExecutionPathSupportContext<'a> {
    /// Canonical repository receiving the rendered projection.
    pub(super) repository_id: &'a LogicalRepositoryIdV1,
    /// Portable profile that selected the path and activation policy.
    pub(super) profile: &'a EffectiveProjectProfileV1,
    /// Neutral composition plan that proves the path structure.
    pub(super) composition_plan: &'a ExperienceCompositionPlanV1,
    /// Machine-resolved profile used only for bounded explanatory evidence.
    pub(super) instance: &'a ResolvedProjectInstanceV1,
    /// Compiler-owned evidence level for machine-specific claims.
    pub(super) machine_admission: &'a MachineAdmission,
}

/// Borrow selected execution paths that terminate at one native harness.
pub(super) fn execution_paths_for_harness<'a>(
    profile: &'a EffectiveProjectProfileV1,
    composition_plan: &ExperienceCompositionPlanV1,
    harness: TerminalAdapterDescriptor,
) -> Result<Vec<&'a ExecutionPathSelectionV1>, super::RenderError> {
    profile
        .execution_paths
        .iter()
        .filter_map(
            |path| match crate::terminal_adapters::for_selection(path, composition_plan) {
                Ok((_, adapter)) if adapter == harness => Some(Ok(path)),
                Ok(_) => None,
                Err(error) => Some(Err(super::RenderError::invalid_input_error(
                    "execution-path terminal adapter",
                    error,
                ))),
            },
        )
        .collect()
}

/// Copy the stable identities of borrowed execution-path selections.
pub(super) fn execution_path_ids(
    paths: &[&ExecutionPathSelectionV1],
) -> Vec<ExecutionPathSelectionIdV1> {
    paths.iter().map(|path| path.id.clone()).collect()
}

/// Copy every selected execution-path identity in effective-profile order.
pub(super) fn all_execution_path_ids(
    profile: &EffectiveProjectProfileV1,
) -> Vec<ExecutionPathSelectionIdV1> {
    profile
        .execution_paths
        .iter()
        .map(|path| path.id.clone())
        .collect()
}

/// Qualify one path projection separately from delegated-owner and effect admission.
pub(super) fn execution_path_support(
    path: ExecutionPathSelectionV1,
    context: &ExecutionPathSupportContext<'_>,
    policy_ids: Vec<String>,
    hook_requirements: HookRequirements,
) -> Result<ExecutionPathSupport, super::RenderError> {
    let witness = ExecutionPathWitnessV1::from_compiled_plan(&path, context.composition_plan)
        .map_err(|error| {
            super::RenderError::invalid_input_record_error(
                "execution_paths",
                path.id().to_string(),
                error,
            )
        })?;
    let provider_binding = witness.terminal_provider_binding.as_ref().ok_or_else(|| {
        super::RenderError::invalid_input(
            "execution_paths",
            format!(
                "execution path `{}` has no terminal provider binding",
                path.id()
            ),
        )
    })?;
    let harness_binding = context
        .machine_admission
        .projected_harness_binding(context.instance, provider_binding)?;
    let provider_binding_admitted =
        context.machine_admission.is_verified() && harness_binding.is_some();
    let activation_selected = context.profile.session.as_ref().is_some_and(|session| {
        session
            .activation_roots
            .iter()
            .any(|root| root == context.repository_id)
            && session
                .activation_path_ids
                .iter()
                .any(|active| active == path.id())
    });
    let surface_executable_bound = context.instance.surface_executable.is_some();
    let session_state_bound = context.instance.session_state.is_some();
    let durable_evidence_bound = context.instance.durable_evidence.is_some();
    let provider_pin = || -> Result<String, super::RenderError> {
        let binding = harness_binding.ok_or_else(|| {
            super::RenderError::invalid_input(
                "machine admission",
                format!(
                    "admitted execution path `{}` has no exact provider binding `{}`",
                    path.id(),
                    provider_binding.provider_binding_id,
                ),
            )
        })?;
        Ok(format!(
            "provider binding `{}` version `{}`, sha256 `{}`",
            provider_binding.provider_binding_id, binding.version, binding.digest
        ))
    };
    let terminal_pins = || -> Result<String, super::RenderError> {
        let mut pins = vec![provider_pin()?];
        if let Some(executable) = &context.instance.surface_executable {
            pins.push(format!(
                "Pale executable version `{}`, sha256 `{}`, interface `{}`",
                executable.version, executable.digest, executable.protocol
            ));
        }
        Ok(pins.join("; "))
    };
    let note = if !context.machine_admission.is_verified() {
        "Evidence level: rendered. Machine admission is absent (portable compilation or removal reconstruction), so this path makes no executable, provider, session-continuity, durable-evidence, or readiness claim.".to_string()
    } else if !provider_binding_admitted {
        "Evidence level: rendered. The exact terminal provider binding did not pass machine admission; operation readiness remains separately derived.".to_string()
    } else if !surface_executable_bound {
        format!(
            "Evidence level: rendered; terminal provider admitted as {}. No machine-local Pale executable is bound; operation readiness remains separately derived.",
            provider_pin()?
        )
    } else if !session_state_bound {
        format!(
            "Evidence level: rendered; terminal executable pins admitted as {}. Ephemeral session continuity is not bound; operation readiness remains separately derived.",
            terminal_pins()?
        )
    } else if !durable_evidence_bound {
        format!(
            "Evidence level: rendered; terminal executable pins admitted as {}. Recoverable decision evidence is not bound; operation readiness remains separately derived.",
            terminal_pins()?
        )
    } else if activation_selected && witness.is_direct() {
        format!(
            "Evidence level: machine inputs admitted for {}. The exact direct path is selected for repository activation; operation readiness, host authorization, and the activation effect remain separate.",
            terminal_pins()?
        )
    } else if activation_selected {
        format!(
            "Evidence level: machine inputs admitted for {}. The selected path contains delegated work edges; this row claims no operation, delegated-owner, invocation, or effect readiness.",
            terminal_pins()?
        )
    } else {
        format!(
            "Evidence level: machine inputs admitted for {}. The path is not selected for repository activation; no operation readiness, invocation, or effect is claimed.",
            terminal_pins()?
        )
    };
    Ok(ExecutionPathSupport {
        execution_path_witness: witness,
        activation_selected,
        policy_ids,
        hook_requirements,
        note,
    })
}
