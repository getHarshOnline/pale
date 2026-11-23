//! Host-native front doors, skills, and specialist-agent projections.

use crate::terminal_adapters::TerminalAdapterDescriptor;
use pale_core::records::provider_binding_projection_root;

use super::{
    artifact_ref, claude_role_agent, codex_role_agent, doctrine_name, doctrine_skill,
    execution_path_ids, execution_paths_for_harness, host_name, mode_name, operation_name,
    operation_requires_manual_invocation, operation_skill, project_operation_support, role_name,
    run_mode_skill, skill_content, GeneratedSurface, HarnessSurfaceRefs, OperationSurfaceRefs,
    OrderedFamily, RenderContext, RenderError, RenderIdentity, SurfaceKind,
};
/// Render one native-harness projection from already resolved path selections.
pub(super) fn render_harness_projection(
    input: &RenderContext<'_, '_>,
    identity: &RenderIdentity<'_>,
    harness: TerminalAdapterDescriptor,
    ordered: &[OrderedFamily<'_>],
    surfaces: &mut Vec<GeneratedSurface>,
) -> Result<HarnessSurfaceRefs, RenderError> {
    let machine_admission = input.machine_admission;
    let input = &input.projection;
    let profile = input.identity.profile_record();
    let instance = input.identity.instance_record();
    let semantics = input.semantics;
    let paths = execution_paths_for_harness(profile, input.experience_composition_plan, harness)?;
    if paths.is_empty() {
        return Ok(HarnessSurfaceRefs::default());
    }
    let consumers = execution_path_ids(&paths);
    let digest = identity.profile().as_str();
    let layout = harness.projection_layout();
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
                "native-harness projection",
                format!("execution paths reaching {harness} select multiple provider bindings"),
            ));
        }
    }
    let native_tools_ready = instance.surface_executable.is_some()
        && machine_admission
            .projected_harness_binding(instance, &provider_binding)?
            .is_some();
    let capability_observations = machine_admission.capability_observations(&provider_binding);

    for selected in ordered {
        let family = selected.family;
        surfaces.push(GeneratedSurface::whole_file(
            format!(
                "{}/{}/SKILL.md",
                layout.skills_root(),
                host_name(&family.id)
            ),
            SurfaceKind::PolicySkill,
            skill_content(family, digest, harness, semantics)?,
            consumers.clone(),
            vec![crate::semantic::resource_label(&family.id)],
            true,
        ));
        if harness == TerminalAdapterDescriptor::codex() {
            surfaces.push(GeneratedSurface::whole_file(
                format!(
                    "{}/{}/agents/openai.yaml",
                    layout.skills_root(),
                    host_name(&family.id)
                ),
                SurfaceKind::PolicyInvocationPolicy,
                "policy:\n  allow_implicit_invocation: false\n",
                consumers.clone(),
                vec![crate::semantic::resource_label(&family.id)],
                true,
            ));
        }
    }
    let mut artifacts = HarnessSurfaceRefs::default();
    for obligation in &input.package_obligations.operations {
        if obligation.state.selection().is_none() {
            continue;
        }
        let operation = semantics
            .operations
            .iter()
            .find(|operation| operation.record.id == obligation.operation)
            .ok_or_else(|| {
                RenderError::invalid_input(
                    "native-harness operation",
                    format!(
                        "operation {} has no semantic record",
                        crate::semantic::resource_label(&obligation.operation)
                    ),
                )
            })?;
        let support = project_operation_support(
            operation,
            harness,
            &provider_binding,
            native_tools_ready,
            capability_observations,
        );
        let command_surface = obligation.state.is_model_facing();
        let name = operation_name(operation);
        let path = if command_surface {
            format!("{}/{name}/SKILL.md", layout.skills_root())
        } else {
            format!(
                "{}/internal-operations/{}.md",
                provider_binding_projection_root(&provider_binding),
                crate::render::semantic_helpers::resource_slug(&operation.record.id)
            )
        };
        let content = if command_surface {
            operation_skill(operation, semantics, harness, &support)?
        } else {
            let body = super::body_links::body_at(
                &operation.body,
                semantics,
                &path,
                layout.skills_root(),
            )?;
            let navigation = super::body_links::navigation_at(
                &operation.record.id,
                pale_core::records::PackageResourceKindV1::Operation,
                semantics,
                &path,
                layout.skills_root(),
            )?;
            let official = super::semantic_helpers::official_source_guidance(
                &super::semantic_helpers::page_sources(
                    &operation.record.id,
                    pale_core::records::PackageResourceKindV1::Operation,
                    semantics,
                    &[],
                ),
            );
            format!(
                "<!-- pale: generated internal operation; exact-id runtime dispatch only. -->\n# {}\n\n{navigation}{}\n\n{official}",
                operation.record.description,
                body.trim(),
            )
        };
        let skill = artifact_ref(&path, &content);
        surfaces.push(GeneratedSurface::whole_file(
            path,
            SurfaceKind::OperationSkill,
            content,
            consumers.clone(),
            vec![format!(
                "operation:{}",
                crate::semantic::resource_label(&operation.record.id)
            )],
            !support.machine_bound,
        ));
        let invocation_policy = if command_surface
            && harness == TerminalAdapterDescriptor::codex()
            && operation_requires_manual_invocation(operation)
        {
            let path = format!("{}/{name}/agents/openai.yaml", layout.skills_root());
            let content = "policy:\n  allow_implicit_invocation: false\n";
            surfaces.push(GeneratedSurface::whole_file(
                &path,
                SurfaceKind::OperationInvocationPolicy,
                content,
                consumers.clone(),
                vec![format!(
                    "operation:{}",
                    crate::semantic::resource_label(&operation.record.id)
                )],
                true,
            ));
            Some(artifact_ref(path, content))
        } else {
            None
        };
        artifacts.operations.insert(
            operation.record.id.clone(),
            OperationSurfaceRefs {
                skill,
                invocation_policy,
                support,
            },
        );
    }
    for doctrine in &semantics.doctrine {
        let path = format!(
            "{}/{}/SKILL.md",
            layout.skills_root(),
            doctrine_name(doctrine)
        );
        let content = doctrine_skill(doctrine, semantics)?;
        surfaces.push(GeneratedSurface::whole_file(
            path,
            SurfaceKind::DoctrineSkill,
            content,
            consumers.clone(),
            vec![format!(
                "doctrine:{}",
                crate::semantic::resource_label(&doctrine.record.id)
            )],
            true,
        ));
    }
    for stack in &semantics.stacks {
        if let Some(content) = super::stack_content::stack_skill(stack, semantics)? {
            surfaces.push(GeneratedSurface::whole_file(
                format!(
                    "{}/{}/SKILL.md",
                    layout.skills_root(),
                    super::semantic_helpers::stack_name(&stack.id)
                ),
                SurfaceKind::StackSkill,
                content,
                consumers.clone(),
                vec![format!(
                    "stack:{}",
                    crate::semantic::resource_label(&stack.id)
                )],
                true,
            ));
        }
    }
    for mode in &semantics.run_modes {
        surfaces.push(GeneratedSurface::whole_file(
            format!("{}/{}/SKILL.md", layout.skills_root(), mode_name(mode)),
            SurfaceKind::RunModeSkill,
            run_mode_skill(mode),
            consumers.clone(),
            vec![format!(
                "run-mode:{}",
                crate::semantic::resource_label(&mode.record.id)
            )],
            true,
        ));
    }
    for role in &semantics.roles {
        let path = format!(
            "{}/{}{}",
            layout.agents_root(),
            role_name(role),
            layout.agent_suffix()
        );
        let content = if harness.is_claude_code() {
            claude_role_agent(role, semantics)?
        } else {
            codex_role_agent(role, semantics)?
        };
        let reference = artifact_ref(&path, &content);
        surfaces.push(GeneratedSurface::whole_file(
            path,
            SurfaceKind::SpecialistAgent,
            content,
            consumers.clone(),
            vec![format!(
                "role:{}",
                crate::semantic::resource_label(&role.record.id)
            )],
            true,
        ));
        artifacts.roles.insert(role.record.id.clone(), reference);
    }
    Ok(artifacts)
}
