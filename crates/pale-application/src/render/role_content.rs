//! Specialist-role instructions and native-harness agent projections.

use super::semantic_content::{
    description, doctrine_skill_name, official_source_guidance, relevant_sources, role_name,
    yaml_string,
};
use super::types::RenderError;
use crate::semantic::{RepositorySemanticPlan, ResolvedRole};

/// Compose one role's inherited lenses and doctrine without granting mechanics.
fn role_instructions(
    role: &ResolvedRole,
    semantics: &RepositorySemanticPlan,
) -> Result<String, RenderError> {
    let body = super::body_links::agent_body(&role.body, semantics)?;
    let navigation = super::body_links::page_navigation(
        &role.record.id,
        pale_core::records::PackageResourceKindV1::Role,
        semantics,
        "../skills/",
    )?;
    Ok(format!(
        "{navigation}{}",
        composed_role_instructions(role, semantics, &body)?
    ))
}

/// Use the same composed method in a reference file, without an agent grant.
pub(super) fn role_reference(
    role: &ResolvedRole,
    semantics: &RepositorySemanticPlan,
    origin_file: &str,
    skills_root: &str,
) -> Result<String, RenderError> {
    let body = super::body_links::body_at(&role.body, semantics, origin_file, skills_root)?;
    let navigation = super::body_links::navigation_at(
        &role.record.id,
        pale_core::records::PackageResourceKindV1::Role,
        semantics,
        origin_file,
        skills_root,
    )?;
    super::closing_questions::finish(&role.record.id, pale_core::records::PackageResourceKindV1::Role, semantics, &super::body_links::skills_prefix(origin_file, skills_root), format!(
        "# {}\n\n{navigation}{}\n\nUse this reference for the method. Read project instructions for local facts and limits, and use tools under their configured permissions.\n",
        role.record.description,
        composed_role_instructions(role, semantics, &body)?,
    ))
}

fn composed_role_instructions(
    role: &ResolvedRole,
    semantics: &RepositorySemanticPlan,
    body: &str,
) -> Result<String, RenderError> {
    let inherited = if role.record.inherits_lenses.is_empty() {
        String::new()
    } else {
        let names = role
            .record
            .inherits_lenses
            .iter()
            .map(|id| {
                semantics
                    .roles
                    .iter()
                    .find(|item| item.record.id == *id)
                    .map(role_name)
                    .ok_or_else(|| {
                        RenderError::invalid_input(
                            "role lenses",
                            format!(
                                "role `{}` references missing inherited lens `{id}`",
                                crate::semantic::resource_label(&role.record.id)
                            ),
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        format!(
            "Specialist guidance included below: {}.\n\n",
            names
                .iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let doctrine = if role.doctrine_ids.is_empty() {
        String::new()
    } else {
        let names = role
            .doctrine_ids
            .iter()
            .map(|id| format!("`{}`", doctrine_skill_name(id)))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Required guides are included once below, with prerequisites first: {names}. Use these copies.\n\n"
        )
    };
    let sources = relevant_sources(&role.official_sources, &role.record.applies_to, semantics);
    Ok(format!(
        "{}{}## Specialist guidance\n\n{}\n\n{}",
        inherited,
        doctrine,
        body.trim(),
        official_source_guidance(&super::semantic_helpers::page_sources(
            &role.record.id,
            pale_core::records::PackageResourceKindV1::Role,
            semantics,
            &sources
        )),
    ))
}

/// Render one resolved role for Claude's native agent surface.
pub(super) fn claude_role_agent(
    role: &ResolvedRole,
    semantics: &RepositorySemanticPlan,
) -> Result<String, RenderError> {
    let instructions = role_instructions(role, semantics)?;
    super::closing_questions::finish(&role.record.id, pale_core::records::PackageResourceKindV1::Role, semantics, "../skills/", format!(
        "---\nname: {name}\ndescription: {description}\ntools: []\npermissionMode: plan\n---\n\n# {title}\n\n{instructions}\n\nKeep the parent task’s scope, configured permissions, decision order, repository ownership and safety limits.\n",
        name = role_name(role),
        description = yaml_string(&description(&role.record.description)),
        title = role.record.description,
        instructions = format_args!(
            "{instructions}\n\nWork from the supplied evidence and ask the parent for anything missing. Return findings with sources. This role authorizes no tool actions; any action requires a separately permitted parent task."
        ),
    ))
}

/// Minimal read-only Codex agent projection for one semantic specialist role.
#[derive(serde::Serialize)]
struct CodexRoleAgent {
    name: String,
    description: String,
    sandbox_mode: String,
    developer_instructions: String,
}

/// Render one resolved role for Codex's native agent surface.
pub(super) fn codex_role_agent(
    role: &ResolvedRole,
    semantics: &RepositorySemanticPlan,
) -> Result<String, RenderError> {
    let instructions = super::closing_questions::finish(
        &role.record.id,
        pale_core::records::PackageResourceKindV1::Role,
        semantics,
        "../skills/",
        format!(
            "{}\n\nReturn findings with supporting evidence to the parent. This role authorizes no changes; any change requires a separately permitted parent task. Keep the parent task’s scope, configured permissions, decision order, repository ownership and safety limits.",
            role_instructions(role, semantics)?
        ),
    )?;
    let doc = CodexRoleAgent {
        name: role_name(role),
        description: description(&role.record.description),
        sandbox_mode: "read-only".to_string(),
        developer_instructions: instructions,
    };
    toml::to_string(&doc).map_err(|error| {
        RenderError::projection_error(
            format!(
                "Codex specialist role `{}`",
                crate::semantic::resource_label(&role.record.id)
            ),
            error,
        )
    })
}
