//! Sealed hook contexts preserve path identity, semantic order, and digest-bound references.

use std::collections::{BTreeMap, BTreeSet};

use crate::terminal_adapters::TerminalAdapterDescriptor;

use pale_core::records::{
    ArtifactDigest, CapabilityIdV1, ExecutionPathWitnessV1, LifecycleBehaviorIdV1,
    PackageResourceRefV1, ProfileRepository, RunPolicyEntry,
};

use super::super::semantic_helpers::json_pretty;
use super::super::types::{HarnessSurfaceRefs, MachineAdmission, RenderError};
use super::super::{project_host_tool_effects, RenderIdentity};
use crate::semantic::RepositorySemanticPlan;

/// Schema identifier embedded in every sealed hook context and its digest preimage.
///
/// Native hook readers must require this exact value before interpreting any
/// path or authority field.
pub const HOOK_CONTEXT_SCHEMA: &str = "pale.agent-surface.hook-context.v1";
/// Wire major embedded beside [`HOOK_CONTEXT_SCHEMA`].
///
/// This is the record's closed initial-v1 major, not a renderer or executable version.
pub const HOOK_CONTEXT_SCHEMA_VERSION: u32 = 1;

/// Inputs shared by host-specific hook projections for one admitted repository.
pub(in crate::render) struct HookContextRender<'a> {
    pub(in crate::render) schema_registry: &'a pale_core::records::SchemaRegistryIdentity,
    pub(in crate::render) compilation_input_digest: &'a pale_core::records::CompilationInputDigest,
    pub(in crate::render) repository: &'a ProfileRepository,
    pub(in crate::render) instance: &'a pale_core::records::ResolvedProjectInstanceV1,
    pub(in crate::render) machine_admission: &'a MachineAdmission,
    pub(in crate::render) identity: RenderIdentity<'a>,
    pub(in crate::render) execution_path_witness: &'a ExecutionPathWitnessV1,
    pub(in crate::render) adapter: TerminalAdapterDescriptor,
    pub(in crate::render) policy: &'a RunPolicyEntry,
    pub(in crate::render) injection_digest: &'a ArtifactDigest,
    pub(in crate::render) projection_manifest: &'a pale_core::records::SealedArtifactRef,
    pub(in crate::render) operation_binding: &'a pale_core::records::SealedArtifactRef,
    /// Repository roots and authority paths denied write access for this projection.
    pub(in crate::render) read_only_paths: &'a [String],
    pub(in crate::render) protected_runtime_dirs: &'a [String],
    pub(in crate::render) behaviours: &'a [pale_core::records::LifecycleBehavior],
    pub(in crate::render) behaviour_guidance:
        &'a BTreeMap<LifecycleBehaviorIdV1, pale_core::records::SealedArtifactRef>,
    pub(in crate::render) run_mode_bodies:
        &'a BTreeMap<PackageResourceRefV1, pale_core::records::SealedArtifactRef>,
    pub(in crate::render) available_tool_requirements: &'a BTreeSet<CapabilityIdV1>,
    pub(in crate::render) operation_artifacts: &'a HarnessSurfaceRefs,
    pub(in crate::render) semantics: &'a RepositorySemanticPlan,
}

/// Render one witnessed-path context consumed directly by the hook runtime.
pub(in crate::render) fn hook_context_json(
    input: HookContextRender<'_>,
) -> Result<String, RenderError> {
    let HookContextRender {
        schema_registry,
        compilation_input_digest,
        repository,
        instance,
        machine_admission,
        identity,
        execution_path_witness,
        adapter,
        policy,
        injection_digest,
        projection_manifest,
        operation_binding,
        read_only_paths,
        protected_runtime_dirs,
        behaviours,
        behaviour_guidance,
        run_mode_bodies,
        available_tool_requirements,
        operation_artifacts,
        semantics,
    } = input;
    policy.validate_contract().map_err(|error| {
        RenderError::invalid_input_record_error("run_policy", policy.id.to_string(), error)
    })?;
    for behaviour in behaviours {
        behaviour.validate_contract().map_err(|error| {
            RenderError::invalid_input_record_error(
                "lifecycle.behaviours",
                behaviour.id.to_string(),
                error,
            )
        })?;
    }
    let host_tool_effects = project_host_tool_effects(operation_artifacts);

    let mut repository_verifiers = Vec::new();
    for capability_id in &semantics.repository_verifier_capabilities {
        let Some(binding) = semantics
            .operations
            .iter()
            .flat_map(|operation| operation.capabilities())
            .find(|capability| &capability.capability_id == capability_id)
            .and_then(crate::semantic::ResolvedCapability::binding)
        else {
            continue;
        };
        if !machine_admission.is_verified() {
            continue;
        }
        let Some(provider_binding) = execution_path_witness.terminal_provider_binding.as_ref()
        else {
            continue;
        };
        for tool in binding
            .provider_tools()
            .into_iter()
            .flatten()
            .filter(|row| row.provider_binding == *provider_binding)
            .flat_map(|row| &row.tools)
        {
            for command in tool
                .commands
                .iter()
                .filter(|command| !command.allow_trailing_args)
            {
                repository_verifiers.push(pale_core::records::RepositoryVerifierBinding {
                    capability_id: capability_id.clone(),
                    tool_name: tool.name.clone(),
                    command: command.clone(),
                });
            }
        }
    }
    repository_verifiers.sort();
    repository_verifiers.dedup();

    let behaviours = behaviours
        .iter()
        .cloned()
        .map(|mut behaviour| {
            behaviour.guidance = None;
            behaviour
        })
        .collect();
    let native_harness_binding = execution_path_witness
        .terminal_provider_binding
        .as_ref()
        .map(|provider| machine_admission.projected_harness_binding(instance, provider))
        .transpose()?
        .flatten()
        .cloned();
    let schema_identity = pale_contracts::record_schema_identity("hook-context.v1")
        .map_err(|error| RenderError::projection_error("hook context schema identity", error))?;
    let context = pale_core::records::HookContextV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        schema_version: HOOK_CONTEXT_SCHEMA_VERSION,
        schema_manifest_sha256: schema_registry.schema_manifest_sha256.clone(),
        schema_set_sha256: schema_registry.schema_set_sha256.clone(),
        compilation_input_digest: compilation_input_digest.clone(),
        profile_digest: identity.profile().clone(),
        instance_digest: identity.instance().clone(),
        repository_id: repository.id.clone(),
        execution_path_witness: execution_path_witness.clone(),
        adapter_plugin_id: adapter.registry_id().to_owned(),
        native_harness_binding,
        run_policy: policy.clone(),
        injection_digest: injection_digest.clone(),
        projection_manifest: projection_manifest.clone(),
        operation_binding: operation_binding.clone(),
        session_state: instance.session_state.clone(),
        durable_evidence: instance.durable_evidence.clone(),
        materialization_lineage: instance.materialization_lineage.clone(),
        mode_defaults: semantics
            .mode_defaults
            .iter()
            .map(|(axis, id)| (*axis, id.clone()))
            .collect(),
        run_modes: semantics
            .run_modes
            .iter()
            .map(|mode| pale_core::records::SealedRunMode {
                id: mode.record.id.clone(),
                axis: mode.record.axis,
                aliases: mode.record.aliases.clone(),
                exact_triggers: mode.record.exact_triggers.clone(),
                body: run_mode_bodies.get(&mode.record.id).cloned(),
            })
            .collect(),
        read_only_dirs: read_only_paths.to_vec(),
        protected_runtime_dirs: protected_runtime_dirs.to_vec(),
        host_tool_effects,
        repository_verifiers,
        available_tool_requirements: available_tool_requirements.iter().cloned().collect(),
        behaviour_guidance: behaviour_guidance.clone(),
        behaviours,
    };
    pale_contracts::validate_record(&context, "hook-context.v1")
        .map_err(|error| RenderError::projection_error("sealed hook context", error))?;
    json_pretty(&context, "sealed hook context")
}
