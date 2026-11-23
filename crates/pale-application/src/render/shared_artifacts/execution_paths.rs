//! Render shared harness projections and path-specific bindings and hook contexts.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::ExecutionPathWitnessV1;

use super::super::family_content::OrderedFamily;
use super::super::{
    execution_path_ids, execution_paths_for_harness, family_ids, hook_context_json,
    operation_projection, operation_route_binding, render_projection_manifests, GeneratedSurface,
    HarnessSurfaceRefs, HookContextRender, RenderContext, RenderError, RenderIdentity, SurfaceKind,
};
use super::lifecycle::LifecycleArtifacts;

#[derive(Clone, Copy)]
/// Exact inputs used to render the bounded execution-path artifact cohort.
pub(super) struct ExecutionPathArtifactSources<'a, 'families> {
    /// Dependency-ordered Package families projected for each terminal harness.
    pub(super) by_harness: &'a BTreeMap<TerminalAdapterDescriptor, Vec<OrderedFamily<'families>>>,
    /// Sealed native-harness artifacts referenced by path-specific contracts.
    pub(super) harness_artifacts: &'a BTreeMap<TerminalAdapterDescriptor, HarnessSurfaceRefs>,
    /// Repository roots and authority paths denied write access.
    pub(super) read_only_paths: &'a [String],
    /// Pale-owned state directories protected from harness writes.
    pub(super) protected_runtime_dirs: &'a [String],
    /// Shared lifecycle artifacts referenced by every path-specific context.
    pub(super) lifecycle: &'a LifecycleArtifacts,
}

/// Render path-specific operation bindings and hook contexts.
pub(super) fn render_execution_path_artifacts(
    context: &RenderContext<'_, '_>,
    identity: &RenderIdentity<'_>,
    sources: ExecutionPathArtifactSources<'_, '_>,
    surfaces: &mut Vec<GeneratedSurface>,
) -> Result<(), RenderError> {
    let machine_admission = context.machine_admission;
    let input = &context.projection;
    let profile = input.identity.profile_record();
    let instance = input.identity.instance_record();
    let repo = input.repository;
    let policy = &profile.run_policy;
    let lifecycle = input.lifecycle;
    let semantics = input.semantics;
    let ExecutionPathArtifactSources {
        by_harness,
        harness_artifacts,
        read_only_paths,
        protected_runtime_dirs,
        lifecycle: lifecycle_artifacts,
    } = sources;
    let mut operation_projections = BTreeMap::new();
    for harness in crate::terminal_adapters::descriptors() {
        let paths =
            execution_paths_for_harness(profile, input.experience_composition_plan, harness)?;
        if paths.is_empty() {
            continue;
        }
        let prompt_artifacts = harness_artifacts.get(&harness).ok_or_else(|| {
            RenderError::projection(
                "operation projection",
                format!("missing rendered artifacts for {harness}"),
            )
        })?;
        let provider_binding =
            crate::terminal_adapters::for_selection(paths[0], input.experience_composition_plan)
                .map_err(|error| {
                    RenderError::invalid_input_error("execution-path terminal adapter", error)
                })?
                .0;
        let projection = operation_projection(
            context,
            identity,
            harness,
            &provider_binding,
            prompt_artifacts,
            &execution_path_ids(&paths),
            surfaces,
        )?;
        operation_projections.insert(harness, projection);
    }

    let projection_manifests = render_projection_manifests(
        profile,
        input.experience_composition_plan,
        identity,
        repo,
        surfaces,
    )?;
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
                "execution-path hook context",
                format!("{} has no terminal provider binding", path.id()),
            )
        })?;
        let harness = crate::terminal_adapters::for_provider(
            provider_binding,
            input.experience_composition_plan,
        )
        .map_err(|error| {
            RenderError::invalid_input_error("execution-path terminal adapter", error)
        })?;
        let injection_digest = lifecycle_artifacts
            .injection_digests
            .get(&harness)
            .ok_or_else(|| {
                RenderError::projection(
                    "execution-path hook context",
                    format!("{} has no lifecycle injection", path.id()),
                )
            })?;
        let projection = operation_projections.get(&harness).ok_or_else(|| {
            RenderError::projection(
                "execution-path hook context",
                format!("{} has no operation projection", path.id()),
            )
        })?;
        let projection_manifest = projection_manifests.get(&harness).ok_or_else(|| {
            RenderError::projection(
                "execution-path hook context",
                format!("{} has no harness projection manifest", path.id()),
            )
        })?;
        let operation_artifacts = harness_artifacts.get(&harness).ok_or_else(|| {
            RenderError::projection(
                "execution-path hook context",
                format!("{} has no rendered operation artifacts", path.id()),
            )
        })?;
        let operation_binding = operation_route_binding(&witness, projection)?;
        surfaces.push(
            GeneratedSurface::whole_file(
                operation_binding.artifact().rel_path.clone(),
                SurfaceKind::OperationRouteBinding,
                operation_binding.content(),
                vec![path.id.clone()],
                semantics
                    .operations
                    .iter()
                    .map(|operation| {
                        format!(
                            "operation:{}",
                            crate::semantic::resource_label(&operation.record.id)
                        )
                    })
                    .collect(),
                operation_binding.committable(),
            )
            .with_note(
                "Exact neutral execution path bound to the shared native-harness operation catalog.",
            ),
        );
        let available_tool_requirements = lifecycle
            .iter()
            .flat_map(|behaviour| behaviour.tool_reqs.iter())
            .filter(|capability| {
                machine_admission
                    .capability_observations(provider_binding)
                    .is_some_and(|observed| observed.contains(*capability))
            })
            .cloned()
            .collect::<BTreeSet<_>>();
        let context = hook_context_json(HookContextRender {
            schema_registry: input.schema_registry,
            compilation_input_digest: input.compilation_input_digest,
            repository: repo,
            instance,
            machine_admission,
            identity: *identity,
            execution_path_witness: &witness,
            adapter: harness,
            policy,
            injection_digest,
            projection_manifest,
            operation_binding: operation_binding.artifact(),
            read_only_paths,
            protected_runtime_dirs,
            behaviours: lifecycle,
            behaviour_guidance: &lifecycle_artifacts.behaviour_guidance,
            run_mode_bodies: &lifecycle_artifacts.run_mode_bodies,
            available_tool_requirements: &available_tool_requirements,
            operation_artifacts,
            semantics,
        })?;
        surfaces.push(
            GeneratedSurface::whole_file(
                format!(".pale/execution-paths/{}/hook-context.json", path.id()),
                SurfaceKind::HookContext,
                context,
                vec![path.id.clone()],
                by_harness
                    .get(&harness)
                    .map(|families| family_ids(families))
                    .unwrap_or_default(),
                false,
            )
            .with_note(
                "Path-sealed harness protocol, policy, boundaries, and lifecycle projection.",
            ),
        );
    }
    Ok(())
}
