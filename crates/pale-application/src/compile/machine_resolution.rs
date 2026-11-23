//! Session requirements and machine-backed admission resolution.

use crate::terminal_adapters::ProviderBindingKey;

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    CapabilityIdV1, CapabilityProvider, EffectiveProjectProfileV1, ExperienceCompositionPlanV1,
    LifecycleBehavior, ProviderBindingRefV1, ResolvedProjectInstanceV1, RunPolicyEntry,
};

use crate::render::MachineAdmission;
use crate::workspace;

use super::{types::CompilationMode, CompileError};

/// Route requirements resolved against either verified or deferred machine state.
pub(super) struct MachineResolution {
    /// Exact read-only repository and authority paths sealed into the write boundary.
    pub(super) read_only_paths: Vec<String>,
    /// Exact admission state consumed by rendering.
    pub(super) machine_admission: MachineAdmission,
}

/// Controls whether compilation may claim machine-local activation readiness.
#[derive(Clone, Copy)]
pub(super) enum MachineVerification<'a> {
    /// Verify executable identities, state roots, authority files, and paths.
    Required {
        /// Fresh filesystem observations consumed by application policy.
        filesystem: &'a dyn pale_ports::MachineFilesystemPort,
        /// Fresh executable and process observations consumed by application policy.
        processes: &'a dyn pale_ports::ProcessProbePort,
    },
    /// Preserve portable projection while leaving activation explicitly unready.
    Deferred,
    /// Reproduce declared materialized bytes for cleanup without probing outer executables.
    ReconstructRemoval,
}

impl MachineVerification<'_> {
    /// Project the verification posture into the compilation identity sealed by
    /// downstream plans; callers must not infer live readiness from this value.
    pub(super) const fn mode(self) -> CompilationMode {
        match self {
            Self::Required { .. } => CompilationMode::Live,
            Self::Deferred => CompilationMode::Portable,
            Self::ReconstructRemoval => CompilationMode::Removal,
        }
    }

    fn reconstructs_removal(self) -> bool {
        matches!(self, Self::ReconstructRemoval)
    }
}

#[derive(Clone, Copy)]
/// Inputs required to resolve machine evidence under one verification posture.
pub(super) struct MachineResolutionRequest<'a> {
    pub(super) profile: &'a EffectiveProjectProfileV1,
    pub(super) instance: &'a ResolvedProjectInstanceV1,
    pub(super) composition_plan: &'a ExperienceCompositionPlanV1,
    pub(super) policy: &'a RunPolicyEntry,
    pub(super) behaviours: &'a [LifecycleBehavior],
    pub(super) repository_verifier_capabilities: &'a [CapabilityIdV1],
    pub(super) verification: MachineVerification<'a>,
}

/// Resolve hook requirements and, when required, prove their machine bindings.
///
/// Every mode validates that declared lifecycle events cover path requirements.
/// Non-live modes keep path strings unresolved and never report the machine as
/// verified; removal reconstruction retains declared projection identities only.
pub(super) fn resolve_machine(
    request: MachineResolutionRequest<'_>,
) -> Result<MachineResolution, CompileError> {
    let MachineResolutionRequest {
        profile,
        instance,
        composition_plan,
        policy,
        behaviours,
        repository_verifier_capabilities,
        verification,
    } = request;
    let has_operations = !profile.semantic.operations.is_empty();
    let has_modes = !profile.semantic.run_modes.is_empty();
    let live = match verification {
        MachineVerification::Required {
            filesystem,
            processes,
        } => Some((filesystem, processes)),
        MachineVerification::Deferred | MachineVerification::ReconstructRemoval => None,
    };
    let mut read_only_paths = if let Some((filesystem, _)) = live {
        workspace::resolve_readonly_repository_paths(filesystem, profile, instance)
            .map_err(|source| CompileError::Verify(Box::new(source)))?
    } else {
        workspace::declared_readonly_repository_paths(profile, instance)
            .map_err(|source| CompileError::Verify(Box::new(source)))?
    };
    let mut executable_identity_digest = None;
    let verified_harnesses = if let Some((filesystem, processes)) = live {
        let harnesses = workspace::verify_native_harnesses(processes, instance)
            .map_err(|source| CompileError::Verify(Box::new(source)))?;
        if instance.surface_executable.is_some() {
            executable_identity_digest = Some(
                workspace::verify_instance_executable_binding(processes, instance)
                    .map_err(|source| CompileError::Verify(Box::new(source)))?,
            );
        }
        if instance.session_state.is_some() {
            workspace::verify_session_state_root(filesystem, instance)
                .map_err(|source| CompileError::Verify(Box::new(source)))?;
        }
        if instance.durable_evidence.is_some() {
            workspace::verify_durable_evidence_root(filesystem, instance)
                .map_err(|source| CompileError::Verify(Box::new(source)))?;
        }
        if instance.materialization_lineage.is_some() {
            workspace::verify_materialization_lineage_root(filesystem, instance)
                .map_err(|source| CompileError::Verify(Box::new(source)))?;
        }
        harnesses
    } else if verification.reconstructs_removal() {
        instance
            .native_harnesses
            .iter()
            .map(|binding| binding.provider_binding.clone())
            .collect()
    } else {
        Vec::new()
    };
    if let Some(session) = &profile.session {
        let declared = session
            .lifecycle_events
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let has_write_boundary = !read_only_paths.is_empty()
            || instance.session_state.is_some()
            || session
                .authority_access
                .iter()
                .any(|access| access.class == pale_core::records::AuthorityAccessClass::ReadOnly);
        for path in &profile.execution_paths {
            let (provider_binding, adapter) =
                crate::terminal_adapters::for_selection(path, composition_plan).map_err(
                    |source| CompileError::TerminalProvider {
                        execution_path: path.id.clone(),
                        source,
                    },
                )?;
            let requirements = crate::hooks::HookRequirements::resolve(
                adapter,
                policy,
                behaviours,
                crate::hooks::HookRequirementFacts {
                    write_boundary: has_write_boundary,
                    operations: has_operations,
                    modes: has_modes,
                    repository_verifiers:
                        crate::hooks::profile_requires_any_activation_repository_verifier_observation(
                            profile,
                            &provider_binding,
                            verified_harnesses.contains(&provider_binding),
                            repository_verifier_capabilities,
                        ),
                },
            );
            for &event in requirements.events() {
                if !declared.contains(&event) {
                    return Err(CompileError::LifecycleConflict {
                        execution_path: path.id.clone(),
                        event,
                        declared: session.lifecycle_events.clone(),
                    });
                }
            }
        }
        if let Some((filesystem, _)) = live {
            workspace::verify_reading_order(filesystem, instance, session)
                .map_err(|source| CompileError::Verify(Box::new(source)))?;
            read_only_paths.extend(
                workspace::resolve_readonly_authority_paths(filesystem, instance, session)
                    .map_err(|source| CompileError::Verify(Box::new(source)))?,
            );
        } else {
            read_only_paths.extend(
                workspace::declared_readonly_authority_paths(instance, session)
                    .map_err(|source| CompileError::Verify(Box::new(source)))?,
            );
        }
    }
    read_only_paths = workspace::minimal_readonly_path_cover(read_only_paths);
    let capability_observations = projected_host_capability_observations(
        profile,
        &verified_harnesses,
        instance.surface_executable.is_some(),
    );
    let machine_admission = if live.is_some() {
        MachineAdmission::verified(
            verified_harnesses,
            capability_observations,
            executable_identity_digest.clone(),
        )
    } else if verification.reconstructs_removal() {
        MachineAdmission::reconstructed(verified_harnesses, capability_observations)
    } else {
        MachineAdmission::portable()
    };
    Ok(MachineResolution {
        read_only_paths,
        machine_admission,
    })
}

/// Project host-native capability carriers proven by this machine pass.
///
/// The exact Pale executable and harness identities must already be verified in live
/// mode. A private binding contributes only the capability name and constrained
/// host tool; it cannot produce a row without those outer observations.
/// Native-Rust actions use the separately verified Pale executable as their
/// carrier and therefore do not enter this host-tool observation map.
fn projected_host_capability_observations(
    profile: &EffectiveProjectProfileV1,
    provider_bindings: &[ProviderBindingRefV1],
    runtime_projected: bool,
) -> BTreeMap<ProviderBindingKey, BTreeSet<pale_core::records::CapabilityIdV1>> {
    if !runtime_projected {
        return BTreeMap::new();
    }
    provider_bindings
        .iter()
        .map(|provider_binding| {
            let capabilities = profile
                .capability_bindings
                .iter()
                .filter(|(_, binding)| binding.provider() == CapabilityProvider::HostNative)
                .filter(|(_, binding)| {
                    binding.provider_tools().is_some_and(|rows| {
                        rows.iter().any(|row| {
                            row.provider_binding == *provider_binding && !row.tools.is_empty()
                        })
                    })
                })
                .map(|(capability, _)| capability.clone())
                .collect();
            (ProviderBindingKey::from(provider_binding), capabilities)
        })
        .collect()
}
