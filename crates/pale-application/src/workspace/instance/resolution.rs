//! Pure projection from a portable profile and machine binding to one resolved instance.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use pale_core::records::{
    EffectiveProjectProfileV1, ExecutionPathWitnessV1, ExperienceCompositionPlanV1,
    LogicalRepositoryIdV1, MachineBindingFile, ProviderBindingRefV1,
    ProviderRepositoryProjectionV1, ResolvedProjectInstancePreimageV1, ResolvedProjectInstanceV1,
};

use crate::identity::VerifiedProfile;
use crate::workspace::WorkspaceError;

/// Resolve one profile and its admitted composition plan to one machine projection.
///
/// Every logical repository in the profile must resolve. A provider-native
/// repository coordinate enters the instance only when both its repository and
/// provider binding are selected by the active execution workspace. The
/// instance digest therefore excludes unrelated machine declarations without
/// treating a selected opaque coordinate as provider readiness.
pub(crate) fn resolve_instance(
    profile: &EffectiveProjectProfileV1,
    machine: &MachineBindingFile,
    composition_plan: &ExperienceCompositionPlanV1,
) -> Result<ResolvedProjectInstanceV1, WorkspaceError> {
    let profile_digest = VerifiedProfile::verify(profile)
        .map_err(|source| WorkspaceError::InvalidProfileIdentity {
            source: Box::new(source),
        })?
        .digest()
        .clone();
    let mut selected_terminal_bindings = profile
        .execution_paths
        .iter()
        .map(|path| {
            ExecutionPathWitnessV1::from_compiled_plan(path, composition_plan)
                .map(|witness| witness.terminal_provider_binding)
                .map_err(
                    |source| WorkspaceError::InvalidExecutionPathProviderWitness {
                        execution_path: path.id.to_string(),
                        source,
                    },
                )
        })
        .collect::<Result<Vec<Option<ProviderBindingRefV1>>, WorkspaceError>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    selected_terminal_bindings.sort_by(ProviderBindingRefV1::canonical_cmp);
    selected_terminal_bindings.dedup();
    if profile.session.is_some() && !profile.execution_paths.is_empty() {
        if machine.surface_executable.is_none() {
            return Err(WorkspaceError::ActivationRequiresSurfaceExecutable(
                machine.machine.id.clone(),
            ));
        }
        if machine.session_state.is_none() {
            return Err(WorkspaceError::ActivationRequiresSessionState(
                machine.machine.id.clone(),
            ));
        }
        if machine.durable_evidence.is_none() {
            return Err(WorkspaceError::ActivationRequiresDurableEvidence(
                machine.machine.id.clone(),
            ));
        }
        for provider in &selected_terminal_bindings {
            if !machine
                .native_harnesses
                .iter()
                .any(|binding| binding.provider_binding == *provider)
            {
                return Err(WorkspaceError::ActivationRequiresNativeHarness {
                    execution_path: profile
                        .execution_paths
                        .iter()
                        .find_map(|path| {
                            ExecutionPathWitnessV1::from_compiled_plan(path, composition_plan)
                                .ok()
                                .and_then(|witness| {
                                    (witness.terminal_provider_binding.as_ref() == Some(provider))
                                        .then(|| path.id.to_string())
                                })
                        })
                        .unwrap_or_else(|| "unknown".into()),
                    harness: provider.provider_binding_id.clone(),
                });
            }
        }
    }

    let selected_repository_ids = profile
        .repositories
        .iter()
        .map(|repository| repository.id.clone())
        .collect::<BTreeSet<_>>();
    let mut repositories = BTreeMap::<LogicalRepositoryIdV1, String>::new();
    let mut git_metadata_roots = BTreeMap::<LogicalRepositoryIdV1, String>::new();
    for checkout in &machine.repository_checkouts {
        if !selected_repository_ids.contains(&checkout.repository_id) {
            continue;
        }
        if repositories
            .insert(
                checkout.repository_id.clone(),
                checkout.checkout_root.clone(),
            )
            .is_some()
        {
            return Err(WorkspaceError::DuplicateRepository(
                checkout.repository_id.clone(),
            ));
        }
        if let Some(path) = &checkout.git_metadata_root {
            git_metadata_roots.insert(checkout.repository_id.clone(), path.clone());
        }
    }
    for repository in &profile.repositories {
        if !repositories.contains_key(&repository.id) {
            return Err(WorkspaceError::UnresolvedRepository {
                repository: repository.id.clone(),
                machine: machine.machine.id.clone(),
            });
        }
    }
    for authority_id in &profile.authority_root_ids {
        if !machine.authority_roots.contains_key(authority_id) {
            return Err(WorkspaceError::UnresolvedAuthorityRoot {
                authority_root: authority_id.clone(),
                machine: machine.machine.id.clone(),
            });
        }
    }

    let selected_experience_path_ids = profile
        .execution_paths
        .iter()
        .map(|selection| selection.experience_path_id.as_str())
        .collect::<BTreeSet<_>>();
    let selected_provider_binding_ids = composition_plan
        .declarations
        .selected_experience_paths
        .iter()
        .filter(|path| selected_experience_path_ids.contains(path.id.as_str()))
        .flat_map(|path| path.provider_binding_ids.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let mut provider_repositories = machine
        .provider_repositories
        .iter()
        .filter(|projection| {
            selected_repository_ids.contains(&projection.repository_id)
                && selected_provider_binding_ids.contains(projection.provider_binding_id.as_str())
        })
        .cloned()
        .collect::<Vec<ProviderRepositoryProjectionV1>>();
    provider_repositories.sort_by(|left, right| {
        (
            &left.repository_id,
            &left.provider_binding_id,
            &left.native_repository_coordinate,
        )
            .cmp(&(
                &right.repository_id,
                &right.provider_binding_id,
                &right.native_repository_coordinate,
            ))
    });
    if let Some(pair) = provider_repositories.windows(2).find(|pair| {
        pair[0].repository_id == pair[1].repository_id
            && pair[0].provider_binding_id == pair[1].provider_binding_id
    }) {
        return Err(WorkspaceError::DuplicateProviderRepositoryProjection {
            repository: pair[0].repository_id.clone(),
            provider: pair[0].provider_binding_id.clone(),
        });
    }
    let authority_roots: BTreeMap<String, String> = profile
        .authority_root_ids
        .iter()
        .filter_map(|authority_id| {
            machine
                .authority_roots
                .get(authority_id)
                .map(|path| (authority_id.clone(), path.clone()))
        })
        .collect();

    let mut seen_paths: BTreeSet<&str> = BTreeSet::new();
    for (id, path) in &repositories {
        if !Path::new(path).is_absolute() {
            return Err(WorkspaceError::NonAbsolutePath {
                binding: id.to_string(),
                path: path.clone(),
            });
        }
        if !seen_paths.insert(path.as_str()) {
            return Err(WorkspaceError::NonDistinctPath(path.clone()));
        }
    }
    for (id, path) in &git_metadata_roots {
        if !Path::new(path).is_absolute() {
            return Err(WorkspaceError::NonAbsolutePath {
                binding: format!("{id} Git metadata"),
                path: path.clone(),
            });
        }
    }

    let mut native_harnesses = machine
        .native_harnesses
        .iter()
        .filter(|binding| selected_terminal_bindings.contains(&binding.provider_binding))
        .cloned()
        .collect::<Vec<_>>();
    native_harnesses
        .sort_by(|left, right| left.provider_binding.canonical_cmp(&right.provider_binding));
    // Project overrides outrank user preferences. Irrelevant or shadowed preferences stay out of
    // the instance so they cannot perturb an unrelated project's digest.
    let user_mode_defaults = if profile.semantic.run_modes.is_empty() {
        BTreeMap::new()
    } else {
        machine
            .preferences
            .as_ref()
            .map(|preferences| {
                preferences
                    .mode_defaults
                    .iter()
                    .filter(|(axis, _)| !profile.semantic.mode_defaults.contains_key(axis))
                    .map(|(axis, id)| (*axis, id.clone()))
                    .collect()
            })
            .unwrap_or_default()
    };

    let schema_identity = pale_contracts::record_schema_identity("resolved-instance.v1")?;
    let instance = ResolvedProjectInstanceV1::seal(ResolvedProjectInstancePreimageV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        profile_digest,
        machine_id: machine.machine.id.clone(),
        repositories,
        git_metadata_roots,
        provider_repositories,
        authority_roots,
        user_mode_defaults,
        surface_executable: machine.surface_executable.clone(),
        native_harnesses,
        session_state: machine.session_state.clone(),
        durable_evidence: machine.durable_evidence.clone(),
        materialization_lineage: machine.materialization_lineage.clone(),
    })?;
    pale_contracts::validate_record(&instance, "resolved-instance.v1")?;
    Ok(instance)
}
