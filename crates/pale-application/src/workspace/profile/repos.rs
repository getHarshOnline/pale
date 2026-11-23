//! Repository normalization for one effective profile.
//!
//! Package-resource existence and kind are proved by the consuming Package
//! graph stage before this module runs. This module owns only repository-local
//! identity, private command attachment, and materialization order.

use std::collections::BTreeSet;

use pale_core::records::{CapabilitiesFile, ProfileRepository};

use super::super::{SelectedRepositoryMembership, WorkspaceError};

/// Resolve selected repository declarations into the effective profile order.
pub(super) fn build_profile_repositories(
    repositories: &[SelectedRepositoryMembership],
    capabilities: &CapabilitiesFile,
) -> Result<Vec<ProfileRepository>, WorkspaceError> {
    let declared_roles = repositories
        .iter()
        .map(|repository| repository.project.role.resource_id.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(role) = capabilities
        .repository_commands
        .keys()
        .find(|role| !declared_roles.contains(role.as_str()))
    {
        return Err(WorkspaceError::UnknownRepositoryCommandRole(role.clone()));
    }

    let mut normalized = Vec::with_capacity(repositories.len());
    for repository in repositories {
        let membership = &repository.project;
        let role_id = membership.role.resource_id.as_str();
        let command_profile = capabilities.repository_commands.get(role_id);
        let mut capability_commands = command_profile
            .map(|profile| profile.commands.clone())
            .unwrap_or_default();
        for commands in capability_commands.values_mut() {
            commands.sort();
            commands.dedup();
        }
        let mut mutating_not_readonly = command_profile
            .map(|profile| profile.mutating_not_readonly.clone())
            .unwrap_or_default();
        mutating_not_readonly.sort();
        mutating_not_readonly.dedup();
        if let Some(command) = capability_commands
            .values()
            .flatten()
            .find(|command| mutating_not_readonly.contains(command))
        {
            return Err(WorkspaceError::ConflictingRepositoryCommand {
                role: role_id.to_owned(),
                command: command.clone(),
            });
        }
        let mut stacks = membership.stacks.clone();
        stacks.sort();
        stacks.dedup();
        let mut families = membership.families.clone();
        families.sort();
        families.dedup();
        normalized.push(ProfileRepository {
            id: membership.repository_id.clone(),
            access: repository.access,
            role: membership.role.clone(),
            stacks,
            families,
            capability_commands,
            mutating_not_readonly,
            command_note: command_profile.and_then(|profile| profile.note.clone()),
        });
    }
    Ok(normalized)
}
