//! Produce the files each coding tool reads from one resolved policy: Claude Code
//! and Codex each receive their own form of the same guidance and hooks.
//!
//! This module builds on `actions`, `compile`, `hooks`, `identity`, `markdown`,
//! `packages` and `semantic`; `compile`, `plan` and `resident` build on it.
//!
//! Claude Code and Codex are native agent harnesses. Harness surfaces are
//! rendered once per terminal harness and consumed by exact neutral execution
//! paths. Provider delegation remains in the admitted composition plan; this
//! renderer neither rebuilds it nor turns product names into route variants.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use pale_core::records::{LifecycleBehavior, ProfileRepository};

use crate::hooks::{HookRequirementFacts, HookRequirements};
use crate::semantic::{RepositoryObligationClosure, RepositorySemanticPlan};

#[path = "activation/mod.rs"]
mod activation_content;
mod activation_projection;
mod body_links;
mod capability_content;
mod closing_questions;
mod doctrine_content;
mod execution_path_support;
mod family_content;
mod front_door_content;
mod harness_projection;
mod injection_content;
mod operation_content;
mod operation_contract;
mod operation_support;
mod projection_manifest;
mod repository_context;
mod role_content;
mod semantic_content;
mod semantic_helpers;
mod semantic_index;
mod shared_artifacts;
mod source_guidance;
mod stack_content;
mod types;
mod validation;

use activation_content::{
    claude_hook_contribution, claude_hook_wiring, codex_hook_contribution, codex_hook_wiring,
    hook_context_json, ActiveHookWiring, HookContextRender,
};
pub use activation_content::{HOOK_CONTEXT_SCHEMA, HOOK_CONTEXT_SCHEMA_VERSION};
use activation_projection::render_activation;
use capability_content::{capabilities_content, capability_binding_record};
use execution_path_support::{
    all_execution_path_ids, execution_path_ids, execution_path_support,
    execution_paths_for_harness, ExecutionPathSupportContext,
};
use family_content::{ordered_families, skill_content, OrderedFamily};
use front_door_content::front_door;
use harness_projection::render_harness_projection;
use injection_content::injection_content;
use operation_support::{project_host_tool_effects, project_operation_support};
use projection_manifest::render_projection_manifests;
pub(crate) use repository_context::render_repository_context;
use semantic_content::{
    claude_role_agent, codex_role_agent, doctrine_name, doctrine_skill, ensure_unique_invocations,
    mode_name, operation_is_bound, operation_name, operation_projection,
    operation_requires_manual_invocation, operation_route_binding, operation_skill, role_name,
    run_mode_skill, runtime_mode_content, semantic_index,
};
use semantic_helpers::{host_name, json_pretty};
use shared_artifacts::render_shared_artifacts;
pub(crate) use source_guidance::render_source_guidance;
pub use source_guidance::{
    SourceGuidanceError, SourceGuidanceFile, SourceGuidanceHost, SourceGuidanceProjection,
};
pub use types::{ExecutionPathSupport, GeneratedSurface, ProjectRender, RenderError};
pub(crate) use types::{ExecutionPathSupportSeal, MachineAdmission};
use types::{HarnessSurfaceRefs, OperationSurfaceRefs, SurfaceKind};
use validation::{validate_render_input, validate_render_output};

use crate::identity::ProjectIdentity as RenderIdentity;

/// Complete immutable input for one repository projection.
///
/// Portable profile semantics remain distinct from the resolved machine
/// instance. Neither record proves activation; only the compile pipeline's
/// private machine admission can qualify readiness.
pub(crate) struct ProjectRenderInput<'a> {
    /// Exact descriptor-manifest and raw-schema-set identity admitted for this render.
    pub(crate) schema_registry: &'a pale_core::records::SchemaRegistryIdentity,
    /// Non-circular identity of the compilation inputs selected before rendering.
    pub(crate) compilation_input_digest: &'a pale_core::records::CompilationInputDigest,
    /// Exact immutable profile and instance verified at this compilation boundary.
    pub(crate) identity: RenderIdentity<'a>,
    /// Admitted neutral composition plan owning every selected experience path.
    pub(crate) experience_composition_plan: &'a pale_core::records::ExperienceCompositionPlanV1,
    /// Canonical repository position shared with the mechanical closure.
    pub(crate) repository_index: usize,
    /// Repository-specific projection declaration.
    pub(crate) repository: &'a ProfileRepository,
    /// Exact read-only repository roots and authority paths supplied by compilation.
    ///
    /// Verified compilation supplies canonical paths proven during machine
    /// admission. Portable compilation and removal reconstruction may supply
    /// declared paths for byte projection, but cannot make them machine-ready.
    /// Render preserves each entry exactly; in particular, a file path is not
    /// widened to its parent directory.
    pub(crate) read_only_paths: &'a [String],
    /// Source-free lifecycle mechanics projected once from the effective profile.
    pub(crate) lifecycle: &'a [LifecycleBehavior],
    /// Dependency-closed doctrine, roles, modes, operations, and stack semantics.
    pub(crate) semantics: &'a RepositorySemanticPlan,
    /// Pre-render Package obligations compiled from [`Self::semantics`].
    pub(crate) package_obligations: &'a RepositoryObligationClosure,
}

/// Portable render input paired with compile-owned machine admission.
struct RenderContext<'a, 'admission> {
    projection: ProjectRenderInput<'a>,
    machine_admission: &'admission MachineAdmission,
    native_actions: &'admission crate::actions::NativeActionRegistry,
}

/// Preserve dependency order when projecting one harness's family identities.
fn family_ids(ordered: &[OrderedFamily<'_>]) -> Vec<String> {
    ordered
        .iter()
        .map(|selected| crate::semantic::resource_label(&selected.family.id))
        .collect()
}

/// Produce one sorted, deduplicated family inventory across all harnesses.
fn all_family_ids(
    by_harness: &BTreeMap<TerminalAdapterDescriptor, Vec<OrderedFamily<'_>>>,
) -> Vec<String> {
    by_harness
        .values()
        .flat_map(|families| {
            families
                .iter()
                .map(|selected| crate::semantic::resource_label(&selected.family.id))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Seal one rendered artifact's target-relative path and exact content bytes.
fn artifact_ref(
    rel_path: impl Into<String>,
    content: &str,
) -> pale_core::records::SealedArtifactRef {
    pale_core::records::SealedArtifactRef {
        rel_path: rel_path.into(),
        sha256: pale_core::records::ArtifactDigest::hash_bytes(content.as_bytes()),
    }
}

/// Render with the machine admission and native-action registry established by
/// one admitted compile result.
///
/// The registry closes action identity and schema projection only. Repository
/// selection, path support, and invocation authority remain separate inputs.
pub(crate) fn render_project_with_admission(
    input: ProjectRenderInput<'_>,
    machine_admission: &MachineAdmission,
    native_actions: &crate::actions::NativeActionRegistry,
) -> Result<ProjectRender, RenderError> {
    let context = RenderContext {
        projection: input,
        machine_admission,
        native_actions,
    };
    let input = &context.projection;
    let ProjectRenderInput {
        identity,
        repository: repo,
        semantics,
        package_obligations,
        ..
    } = input;
    let profile = identity.profile_record();
    let instance = identity.instance_record();
    validate_render_input(input, machine_admission)?;
    ensure_unique_invocations(semantics)?;
    let by_harness = crate::terminal_adapters::descriptors()
        .filter_map(|harness| {
            match execution_paths_for_harness(profile, input.experience_composition_plan, harness) {
                Ok(paths) if paths.is_empty() => None,
                Ok(_) => Some(ordered_families(semantics).map(|ordered| (harness, ordered))),
                Err(error) => Some(Err(error)),
            }
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let read_only_paths = input.read_only_paths;
    let repository_root = instance.repositories.get(&repo.id).ok_or_else(|| {
        RenderError::invalid_input(
            "instance.repositories",
            format!("resolved instance omits repository `{}`", repo.id),
        )
    })?;
    let protected_namespace = Path::new(repository_root).join(".pale");
    let protected_namespace = protected_namespace.to_str().ok_or_else(|| {
        RenderError::projection(
            "protected runtime namespace",
            format!("repository `{}` has a non-utf-8 Pale namespace", repo.id),
        )
    })?;
    let mut protected_runtime_dirs = vec![protected_namespace.to_string()];
    if let Some(binding) = &instance.session_state {
        protected_runtime_dirs.push(binding.root.clone());
    }
    if let Some(binding) = &instance.durable_evidence {
        protected_runtime_dirs.push(binding.root.clone());
    }
    if let Some(binding) = &instance.materialization_lineage {
        protected_runtime_dirs.push(binding.root.clone());
    }
    protected_runtime_dirs.sort();
    protected_runtime_dirs.dedup();
    let mut surfaces = Vec::new();
    let path_requirements = profile
        .execution_paths
        .iter()
        .map(|path| {
            let (provider_binding, adapter) =
                crate::terminal_adapters::for_selection(path, input.experience_composition_plan)
                    .map_err(|error| {
                        RenderError::invalid_input_error("execution-path terminal adapter", error)
                    })?;
            let requirements = HookRequirements::resolve(
                adapter,
                &profile.run_policy,
                input.lifecycle,
                HookRequirementFacts {
                    write_boundary: !read_only_paths.is_empty()
                        || instance.session_state.is_some()
                        || instance.durable_evidence.is_some()
                        || instance.materialization_lineage.is_some(),
                    operations: package_obligations
                        .operations
                        .iter()
                        .any(|obligation| obligation.state.is_model_facing()),
                    modes: !semantics.run_modes.is_empty(),
                    repository_verifiers:
                        crate::hooks::profile_requires_repository_verifier_observation(
                            profile,
                            &repo.id,
                            &provider_binding,
                            machine_admission.is_verified(),
                            &semantics.repository_verifier_capabilities,
                        ),
                },
            );
            Ok((path.id.clone(), requirements))
        })
        .collect::<Result<BTreeMap<_, _>, RenderError>>()?;
    let mut harness_artifacts = BTreeMap::<TerminalAdapterDescriptor, HarnessSurfaceRefs>::new();
    let mut entry_paths = Vec::new();
    let mut entry_hosts = Vec::new();
    for harness in crate::terminal_adapters::descriptors() {
        if let Some(ordered) = by_harness.get(&harness) {
            let paths =
                execution_paths_for_harness(profile, input.experience_composition_plan, harness)?;
            if !paths.is_empty() {
                entry_hosts.push(harness.as_str());
                entry_paths.extend(paths);
            }
            let artifacts =
                render_harness_projection(&context, identity, harness, ordered, &mut surfaces)?;
            harness_artifacts.insert(harness, artifacts);
        }
    }
    if !entry_paths.is_empty() {
        surfaces.push(GeneratedSurface::managed_block(
            "AGENTS.md",
            SurfaceKind::FrontDoor,
            front_door(
                &entry_hosts.join(", "),
                repo,
                profile,
                instance,
                identity,
                &entry_paths,
            )?,
            execution_path_ids(&entry_paths),
            all_family_ids(&by_harness),
            false,
        )?);
    }
    render_shared_artifacts(
        &context,
        identity,
        &by_harness,
        &harness_artifacts,
        read_only_paths,
        &protected_runtime_dirs,
        &mut surfaces,
    )?;
    render_activation(
        &context,
        identity,
        &path_requirements,
        &by_harness,
        &mut surfaces,
    )?;
    let support_context = ExecutionPathSupportContext {
        repository_id: &repo.id,
        profile,
        composition_plan: input.experience_composition_plan,
        instance,
        machine_admission: context.machine_admission,
    };
    let support: Vec<ExecutionPathSupport> = profile
        .execution_paths
        .iter()
        .cloned()
        .map(|path| {
            let (_, adapter) =
                crate::terminal_adapters::for_selection(&path, input.experience_composition_plan)
                    .map_err(|error| {
                    RenderError::invalid_input_error("execution-path terminal adapter", error)
                })?;
            let policy_ids = by_harness
                .get(&adapter)
                .map(|families| family_ids(families))
                .unwrap_or_default();
            let requirements = path_requirements.get(path.id()).cloned().ok_or_else(|| {
                RenderError::projection(
                    "execution-path hook requirements",
                    format!(
                        "execution path `{}` has no resolved hook requirements",
                        path.id()
                    ),
                )
            })?;
            execution_path_support(path, &support_context, policy_ids, requirements)
        })
        .collect::<Result<_, _>>()?;
    validate_render_output(
        profile,
        &input.repository.id,
        input.experience_composition_plan,
        &surfaces,
        &support,
    )?;
    Ok(ProjectRender { surfaces, support })
}
