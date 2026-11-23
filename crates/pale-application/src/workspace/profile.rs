//! Portable effective-profile assembly after source-free Package admission.

use std::collections::BTreeMap;

use pale_core::records::{
    AdmittedLifecycleBehaviorV1, CapabilitiesFile, CapabilityBinding, CapabilityNarrowingV1,
    EffectiveProjectProfilePreimageV1, EffectiveProjectProfileV1, ExecutionPathSelectionV1,
    LogicalExecutionWorkspaceV1, LogicalProjectV1, NormativePackReleaseV1, OperationBindingEntryV1,
    RunPolicyEntry, SemanticSelection, WorkspaceId, EFFECTIVE_PROJECT_PROFILE_SCHEMA_VERSION,
};

use crate::packages::SealedPackageGraph;

use super::session::validate_session_model;
use super::{SelectedRepositoryMembership, WorkspaceError};

mod repos;

use repos::build_profile_repositories;

struct AuthoritySets {
    authority_root_ids: Vec<String>,
    forbidden_token_authority: Vec<String>,
}

/// Complete admitted input required to seal one portable effective profile.
pub(crate) struct EffectiveProfileInput<'a> {
    /// Principal workspace containing the selected project graph.
    pub(crate) workspace_id: WorkspaceId,
    /// Selected logical project and its portable policy references.
    pub(crate) project: &'a LogicalProjectV1,
    /// Selected execution workspace and optional session declaration.
    pub(crate) execution_workspace: &'a LogicalExecutionWorkspaceV1,
    /// Canonical execution paths selected by the composition compiler.
    pub(crate) execution_paths: &'a [ExecutionPathSelectionV1],
    /// Repository memberships selected from the canonical workspace graph.
    pub(crate) repositories: &'a [SelectedRepositoryMembership],
    /// Source-free admitted Package semantic graph.
    pub(crate) packages: &'a SealedPackageGraph,
    /// Attestor-scoped normative-Pack releases selected by workspace admission.
    pub(crate) normative_pack_closure: &'a [NormativePackReleaseV1],
    /// Lifecycle mechanics admitted independently of Package bodies.
    pub(crate) lifecycle: Vec<AdmittedLifecycleBehaviorV1>,
    /// Private semantic narrowing and operation/capability bindings.
    pub(crate) capabilities: &'a CapabilitiesFile,
}

fn sort_dedup<T: Ord>(values: &mut Vec<T>) {
    values.sort();
    values.dedup();
}

fn normalized_semantic_selection(capabilities: &CapabilitiesFile) -> SemanticSelection {
    let mut semantic = capabilities.semantic.clone();
    for selectors in [
        &mut semantic.operations,
        &mut semantic.roles,
        &mut semantic.doctrine,
        &mut semantic.run_modes,
        &mut semantic.stacks,
    ] {
        selectors.sort_by(|left, right| match (left, right) {
            (
                pale_core::records::PackageResourceSelectorV1::Exact { resource: left },
                pale_core::records::PackageResourceSelectorV1::Exact { resource: right },
            ) => left.cmp(right),
            (
                pale_core::records::PackageResourceSelectorV1::AllFromPackage { package_id: left },
                pale_core::records::PackageResourceSelectorV1::AllFromPackage { package_id: right },
            ) => left.cmp(right),
            (pale_core::records::PackageResourceSelectorV1::Exact { .. }, _) => {
                std::cmp::Ordering::Less
            }
            _ => std::cmp::Ordering::Greater,
        });
        selectors.dedup();
    }
    semantic
}

fn normalized_capability_bindings(
    capabilities: &CapabilitiesFile,
) -> BTreeMap<pale_core::records::CapabilityIdV1, CapabilityBinding> {
    let mut bindings = capabilities.capability_bindings.clone();
    for binding in bindings.values_mut() {
        if let Some(provider_tools) = binding.provider_tools_mut() {
            for row in provider_tools.iter_mut() {
                for tool in &mut row.tools {
                    sort_dedup(&mut tool.commands);
                }
                sort_dedup(&mut row.tools);
            }
            provider_tools.sort_by(|left, right| {
                left.provider_binding.canonical_cmp(&right.provider_binding)
            });
        }
    }
    bindings
}

fn normalized_capability_narrowing(
    capabilities: &CapabilitiesFile,
) -> Option<CapabilityNarrowingV1> {
    capabilities
        .capability_narrowing
        .clone()
        .map(|mut narrowing| {
            sort_dedup(&mut narrowing.permitted_capabilities);
            sort_dedup(&mut narrowing.forbidden_capabilities);
            sort_dedup(&mut narrowing.permitted_effects);
            sort_dedup(&mut narrowing.forbidden_effects);
            narrowing
        })
}

fn normalized_operation_bindings(capabilities: &CapabilitiesFile) -> Vec<OperationBindingEntryV1> {
    let mut bindings = capabilities.operation_bindings.clone();
    for entry in &mut bindings {
        if let Some(aliases) = entry.binding.selection_mut().aliases_mut() {
            sort_dedup(aliases);
        }
    }
    bindings.sort_by(|left, right| left.operation.cmp(&right.operation));
    bindings
}

fn selected_run_policy(
    project: &LogicalProjectV1,
    capabilities: &CapabilitiesFile,
) -> Result<RunPolicyEntry, WorkspaceError> {
    let mut selected = None;
    for policy in &capabilities.run_policies {
        policy
            .validate_contract()
            .map_err(|source| WorkspaceError::InvalidRunPolicy {
                id: policy.id.as_str().to_owned(),
                source,
            })?;
        if policy.id == project.run_policy && selected.replace(policy.clone()).is_some() {
            return Err(WorkspaceError::DuplicateRunPolicy(
                policy.id.as_str().to_owned(),
            ));
        }
    }
    selected.ok_or_else(|| WorkspaceError::MissingRunPolicy(project.run_policy.as_str().to_owned()))
}

fn resolved_authority_sets(
    project: &LogicalProjectV1,
    capabilities: &CapabilitiesFile,
) -> AuthoritySets {
    let mut authority_root_ids = project.authority_root_ids.clone();
    sort_dedup(&mut authority_root_ids);
    let mut forbidden_token_authority = capabilities
        .boundary
        .as_ref()
        .map(|boundary| boundary.forbidden_token_authority.clone())
        .unwrap_or_default();
    sort_dedup(&mut forbidden_token_authority);
    AuthoritySets {
        authority_root_ids,
        forbidden_token_authority,
    }
}

/// Seal the source-free portable profile consumed by instance resolution.
///
/// Package releases, resources, governing algebra, and semantic identity come
/// only from `packages`. Private configuration may select and narrow that
/// admitted graph, but cannot replace its meaning or widen its procedure.
pub(crate) fn compute_effective_profile(
    input: EffectiveProfileInput<'_>,
) -> Result<EffectiveProjectProfileV1, WorkspaceError> {
    let EffectiveProfileInput {
        workspace_id,
        project,
        execution_workspace,
        execution_paths,
        repositories,
        packages,
        normative_pack_closure,
        lifecycle,
        capabilities,
    } = input;
    let repositories = build_profile_repositories(repositories, capabilities)?;
    let AuthoritySets {
        authority_root_ids,
        forbidden_token_authority,
    } = resolved_authority_sets(project, capabilities);

    validate_session_model(
        execution_workspace.session.as_ref(),
        &authority_root_ids,
        repositories.as_slice(),
    )?;

    let schema_identity = pale_contracts::record_schema_identity("effective-profile.v1")?;
    let profile = EffectiveProjectProfileV1::seal(EffectiveProjectProfilePreimageV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        schema_version: EFFECTIVE_PROJECT_PROFILE_SCHEMA_VERSION,
        workspace_id,
        project_id: project.id.clone(),
        execution_workspace_id: execution_workspace.id.clone(),
        execution_paths: execution_paths.to_vec(),
        run_policy: selected_run_policy(project, capabilities)?,
        package_closure: packages.releases().to_vec(),
        normative_pack_closure: normative_pack_closure.to_vec(),
        package_semantic_ir_digest: packages.digest().clone(),
        authority_root_ids,
        repositories,
        governing_order: packages.profile_governing_order().clone(),
        semantic: normalized_semantic_selection(capabilities),
        capability_narrowing: normalized_capability_narrowing(capabilities),
        capability_bindings: normalized_capability_bindings(capabilities),
        operation_bindings: normalized_operation_bindings(capabilities),
        lifecycle,
        forbidden_token_authority,
        session: execution_workspace.session.clone(),
    })
    .map_err(|source| WorkspaceError::InvalidProfileIdentity {
        source: Box::new(source),
    })?;
    pale_contracts::validate_record(&profile, "effective-profile.v1")?;
    Ok(profile)
}
