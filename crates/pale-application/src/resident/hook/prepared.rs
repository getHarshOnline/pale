//! Prepare immutable hook policy while admitting a compiled generation.

use std::collections::BTreeMap;

use pale_core::records::{
    harness_injection_path, ArtifactDigest, ChildPropagation, ExecutionPathSelectionIdV1,
    ExecutionPathSelectionV1, HookContextV1, LifecycleEvent, LogicalRepositoryIdV1,
    ProviderBindingRefV1,
};

use crate::compile::Compilation;
use crate::resident::CompiledGenerationAdmissionError;
use crate::terminal_adapters::{for_exact_binding, TerminalAdapterDescriptor};

use super::{
    select_hook_context_surface, HookContextSurfaceCandidate, ResidentHookPolicyAdmissionError,
};

pub(in crate::resident) struct PreparedHookPolicies {
    repositories:
        BTreeMap<LogicalRepositoryIdV1, BTreeMap<ExecutionPathSelectionIdV1, PreparedHookPath>>,
}

pub(in crate::resident) struct PreparedHookPath {
    pub(super) selection: ExecutionPathSelectionV1,
    pub(super) provider: Option<ProviderBindingRefV1>,
    pub(super) events: Vec<LifecycleEvent>,
    pub(super) policy: Option<PreparedHookPolicy>,
}

pub(super) struct PreparedHookPolicy {
    pub(super) context: HookContextV1,
    pub(super) adapter: TerminalAdapterDescriptor,
    pub(super) injection_surface: Option<usize>,
}

impl PreparedHookPolicies {
    pub(in crate::resident) fn prepare(
        compilation: &Compilation,
    ) -> Result<Self, CompiledGenerationAdmissionError> {
        let mut repositories = BTreeMap::new();
        for repository in compilation.repositories() {
            let mut paths = BTreeMap::new();
            for support in &repository.render().support {
                let witness = &support.execution_path_witness;
                let events = support.hook_requirements.events();
                let fail = |source| CompiledGenerationAdmissionError::HookPolicy {
                    repository_id: repository.id().clone(),
                    execution_path: witness.selection.id().clone(),
                    source,
                };
                let policy = if events.is_empty() {
                    None
                } else {
                    let content = select_hook_context_surface(
                        repository
                            .render()
                            .surfaces
                            .iter()
                            .map(HookContextSurfaceCandidate::from),
                        witness.selection.id(),
                    )
                    .map_err(fail)?;
                    let context: HookContextV1 = serde_json::from_value(
                        pale_core::canon::parse_strict(content.as_bytes()).map_err(|_| {
                            fail(ResidentHookPolicyAdmissionError::MalformedHookContext)
                        })?,
                    )
                    .map_err(|_| fail(ResidentHookPolicyAdmissionError::MalformedHookContext))?;
                    pale_contracts::validate_record(&context, "hook-context.v1")
                        .map_err(|_| fail(ResidentHookPolicyAdmissionError::InvalidHookContext))?;
                    let provider_ref =
                        witness.terminal_provider_binding.as_ref().ok_or_else(|| {
                            fail(ResidentHookPolicyAdmissionError::MissingProviderBinding)
                        })?;
                    let registry = compilation.schema_registry_identity();
                    if context.schema_manifest_sha256 != registry.schema_manifest_sha256
                        || context.schema_set_sha256 != registry.schema_set_sha256
                        || context.compilation_input_digest
                            != *compilation.compilation_input_digest()
                        || context.profile_digest != compilation.profile().profile_digest
                        || context.instance_digest != compilation.instance().instance_digest
                        || context.repository_id != *repository.id()
                        || context.execution_path_witness != *witness
                        || context
                            .native_harness_binding
                            .as_ref()
                            .is_some_and(|binding| &binding.provider_binding != provider_ref)
                    {
                        return Err(fail(
                            ResidentHookPolicyAdmissionError::HookContextSubstitution,
                        ));
                    }
                    let provider = compilation
                        .experience_composition_plan()
                        .declarations
                        .provider_bindings
                        .iter()
                        .find(|binding| binding.id == provider_ref.provider_binding_id)
                        .ok_or_else(|| {
                            fail(ResidentHookPolicyAdmissionError::MissingProviderBinding)
                        })?;
                    let adapter = for_exact_binding(provider_ref, provider).map_err(|_| {
                        fail(ResidentHookPolicyAdmissionError::InvalidProviderBinding)
                    })?;
                    if context.adapter_plugin_id != adapter.registry_id() {
                        return Err(fail(
                            ResidentHookPolicyAdmissionError::HookContextSubstitution,
                        ));
                    }
                    let needs_injection = (events.contains(&LifecycleEvent::SessionStart)
                        && context.run_policy.session_inject)
                        || (events.contains(&LifecycleEvent::SubagentStart)
                            && context.run_policy.child_propagation == ChildPropagation::Propagate);
                    let injection_surface = if needs_injection {
                        let mut candidates =
                            repository.render().surfaces.iter().enumerate().filter(
                                |(_, surface)| {
                                    surface.kind == "injection"
                                        && surface
                                            .execution_path_consumers
                                            .contains(witness.selection.id())
                                },
                            );
                        let (index, surface) = candidates.next().ok_or_else(|| {
                            fail(ResidentHookPolicyAdmissionError::InvalidInjection)
                        })?;
                        if candidates.next().is_some()
                            || surface.rel_path != harness_injection_path(provider_ref)
                            || ArtifactDigest::hash_bytes(surface.content().as_bytes())
                                != context.injection_digest
                        {
                            return Err(fail(ResidentHookPolicyAdmissionError::InvalidInjection));
                        }
                        Some(index)
                    } else {
                        None
                    };
                    Some(PreparedHookPolicy {
                        context,
                        adapter,
                        injection_surface,
                    })
                };
                let path = PreparedHookPath {
                    selection: witness.selection.clone(),
                    provider: witness.terminal_provider_binding.clone(),
                    events: events.to_vec(),
                    policy,
                };
                if paths.insert(witness.selection.id().clone(), path).is_some() {
                    return Err(fail(
                        ResidentHookPolicyAdmissionError::RepeatedExecutionPath,
                    ));
                }
            }
            repositories.insert(repository.id().clone(), paths);
        }
        Ok(Self { repositories })
    }

    pub(in crate::resident) fn path(
        &self,
        repository: &LogicalRepositoryIdV1,
        selection: &ExecutionPathSelectionIdV1,
    ) -> Option<&PreparedHookPath> {
        self.repositories.get(repository)?.get(selection)
    }
}
