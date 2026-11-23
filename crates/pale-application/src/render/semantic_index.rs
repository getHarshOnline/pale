//! Compact repository discovery index and invocation-name integrity checks.

use std::collections::BTreeMap;

use pale_core::records::{PackageResourceRefV1, ProfileRepository};

use super::semantic_content::{
    doctrine_name, mode_default_summary, mode_name, operation_is_model_facing, operation_name,
    role_name,
};
use super::types::RenderError;
use crate::semantic::RepositorySemanticPlan;

/// Render the deterministic index of selected semantic artifacts and invocations.
pub(super) fn semantic_index(
    repository: &ProfileRepository,
    semantics: &RepositorySemanticPlan,
) -> String {
    let mut text = format!(
        "# Pale guidance for {}\n\nFind the guide, task or specialist needed for the work. The host loads its instructions when used.\n\n",
        repository.id
    );
    text.push_str("## Stacks\n\n");
    for stack in &semantics.stacks {
        if stack.body.is_some() {
            text.push_str(&format!(
                "- `{}` - {}\n",
                super::semantic_helpers::stack_name(&stack.id),
                stack.description
            ));
        } else {
            text.push_str(&format!(
                "- `{}` - {}\n",
                crate::semantic::resource_label(&stack.id),
                stack.description
            ));
        }
        for doctrine in semantics
            .doctrine
            .iter()
            .filter(|doctrine| stack.doctrine.contains(&doctrine.record.id))
        {
            text.push_str(&format!(
                "  - Guidance skill `{}`\n",
                doctrine_name(doctrine)
            ));
        }
    }
    text.push_str("\n## Tasks\n\n");
    for operation in semantics
        .operations
        .iter()
        .filter(|operation| operation_is_model_facing(operation))
    {
        text.push_str(&format!(
            "- `{}` - {} (check availability in this host’s skill)\n",
            operation_name(operation),
            operation.record.description,
        ));
    }
    text.push_str("\n## Specialist roles\n\n");
    for role in &semantics.roles {
        text.push_str(&format!(
            "- `{}` - {} (check availability in this host’s agent)\n",
            role_name(role),
            role.record.description,
        ));
    }
    text.push_str("\n## Guides\n\n");
    for doctrine in &semantics.doctrine {
        text.push_str(&format!(
            "- `{}` - {}\n",
            doctrine_name(doctrine),
            doctrine.record.description,
        ));
    }
    text.push_str("\n## Active startup policy\n\n");
    for policy in &semantics.startup_policies {
        text.push_str(&format!(
            "- `{}` - {}\n",
            crate::semantic::resource_label(&policy.record.id),
            policy.record.description,
        ));
    }
    text.push_str("\n## Session modes\n\n");
    if !semantics.mode_defaults.is_empty() {
        text.push_str(&format!(
            "Current defaults: {}. Project defaults override user defaults.\n\n",
            mode_default_summary(semantics)
        ));
    }
    for mode in &semantics.run_modes {
        text.push_str(&format!(
            "- `{}` - {}\n",
            mode_name(mode),
            mode.record.description,
        ));
    }
    text.push_str("\n## Tool procedures used by tasks\n\n");
    text.push_str("Each recipe describes a procedure that different providers can implement. The selected task’s configuration says which provider is usable.\n\n");
    for recipe in &semantics.recipes {
        text.push_str(&format!(
            "- `{}` - {}\n",
            crate::semantic::resource_label(&recipe.resource),
            recipe.procedure.description,
        ));
    }
    text
}

/// Reject host invocation collisions before any surface is materialized.
pub(super) fn ensure_unique_invocations(
    semantics: &RepositorySemanticPlan,
) -> Result<(), RenderError> {
    let skills = semantics
        .operations
        .iter()
        .filter(|operation| operation_is_model_facing(operation))
        .map(|operation| (operation_name(operation), &operation.record.id))
        .chain(
            semantics
                .run_modes
                .iter()
                .map(|mode| (mode_name(mode), &mode.record.id)),
        );
    validate_unique_host_names("skill", skills)?;
    validate_unique_host_names(
        "specialist agent",
        semantics
            .roles
            .iter()
            .map(|role| (role_name(role), &role.record.id)),
    )
}

fn validate_unique_host_names<'a>(
    kind: &'static str,
    entries: impl IntoIterator<Item = (String, &'a PackageResourceRefV1)>,
) -> Result<(), RenderError> {
    let mut names = BTreeMap::new();
    for (name, id) in entries {
        validate_host_name(kind, &name)?;
        if let Some(first) = names.insert(name.clone(), id) {
            return Err(RenderError::HostNameCollision {
                kind,
                name,
                first: crate::semantic::resource_label(first),
                second: crate::semantic::resource_label(id),
            });
        }
    }
    Ok(())
}

/// Enforce the shared host-visible invocation-name grammar before rendering.
fn validate_host_name(kind: &str, name: &str) -> Result<(), RenderError> {
    let bytes = name.as_bytes();
    let valid = (1..=64).contains(&bytes.len())
        && bytes
            .first()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(RenderError::invalid_input(
            "semantics",
            format!(
                "{kind} invocation `{name}` must be 1-64 lowercase ASCII letters, digits, or hyphens, and start and end with a letter or digit"
            ),
        ))
    }
}

#[cfg(test)]
#[path = "../../tests/internal/render/semantic_index.rs"]
mod tests;
