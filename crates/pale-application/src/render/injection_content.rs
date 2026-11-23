//! Startup injection preserves selected doctrine and canonical repository boundaries.

use pale_core::records::{EffectiveProjectProfileV1, ProfileRepository};

use super::family_content::OrderedFamily;
use super::semantic_helpers::short;
use super::RenderIdentity;
use crate::semantic::RepositorySemanticPlan;

/// Render selected policy families in their already-resolved authority order.
fn skill_ladder(ordered: &[OrderedFamily<'_>]) -> String {
    let mut s = String::new();
    for selected in ordered {
        let family = selected.family;
        s.push_str(&format!(
            "  - {}: {}\n    Purpose: {}\n",
            crate::semantic::resource_label(&family.id),
            family.title,
            family.purpose,
        ));
    }
    s
}

/// Render bounded discovery pointers rather than injecting the semantic catalog.
fn semantic_pointer(
    semantics: &RepositorySemanticPlan,
    origin_file: &str,
    skills_root: &str,
) -> Result<String, super::RenderError> {
    let mut text = format!(
        "This host offers {} tasks, {} specialist agents and {} guides. Load the chosen task and its required guidance; the rest of the catalog is available when needed.\n",
        semantics
            .operations
            .iter()
            .filter(
                |operation| crate::render::semantic_content::operation_is_model_facing(operation)
            )
            .count(),
        semantics.roles.len(),
        semantics.doctrine.len(),
    );
    if !semantics.run_modes.is_empty() {
        text.push_str("Session modes (approach and effort; safety rules still apply):\n");
        text.push_str(&format!(
            "  Defaults: {}\n",
            crate::render::semantic_content::mode_default_summary(semantics)
        ));
        for mode in &semantics.run_modes {
            text.push_str(&format!(
                "  - {} - {}\n",
                crate::render::semantic_content::mode_name(mode),
                mode.record.description,
            ));
        }
    }
    if !semantics.startup_policies.is_empty() {
        text.push_str("\nStartup policy (active in this repository):\n");
        for policy in &semantics.startup_policies {
            let body =
                super::body_links::body_at(&policy.body, semantics, origin_file, skills_root)?;
            let reading = super::body_links::navigation_at(
                &policy.record.id,
                pale_core::records::PackageResourceKindV1::Doctrine,
                semantics,
                origin_file,
                skills_root,
            )?;
            let sources = super::semantic_helpers::official_source_guidance(
                &super::semantic_helpers::page_sources(
                    &policy.record.id,
                    pale_core::records::PackageResourceKindV1::Doctrine,
                    semantics,
                    &policy.record.official_sources,
                ),
            );
            text.push_str(&format!(
                "\n## {}\n\n{reading}{}\n\n{sources}",
                policy.title,
                body.trim()
            ));
        }
    }
    text.push_str(
        "Find tasks and guides in `.pale/semantic-index.md`. The generated JSON defines tool permissions and invocation rules.\n",
    );
    Ok(text)
}

/// The injection payload emitted for [`pale_core::records::LifecycleEvent::SessionStart`].
///
/// It carries doctrine context, ownership boundaries, repository-native
/// capability commands, and boundary-token authority for each host injection
/// point.
pub(super) fn injection_content(
    profile: &EffectiveProjectProfileV1,
    repo: &ProfileRepository,
    ordered: &[OrderedFamily<'_>],
    semantics: &RepositorySemanticPlan,
    identity: &RenderIdentity<'_>,
    origin_file: &str,
    skills_root: &str,
) -> Result<String, super::RenderError> {
    let dg = short(identity.profile().as_str());
    let fams = skill_ladder(ordered);
    let commands = if repo.capability_commands.is_empty() {
        "  (none declared)\n".to_string()
    } else {
        repo.capability_commands
            .iter()
            .map(|(class, commands)| format!("  - {class}: {}\n", commands.join("; ")))
            .collect()
    };
    let forbidden = if profile.forbidden_token_authority.is_empty() {
        "  (none declared)\n".to_string()
    } else {
        profile
            .forbidden_token_authority
            .iter()
            .map(|a| format!("  - {a}\n"))
            .collect()
    };
    // The session context presents the selected canonical project graph, not just this
    // repository. The effective profile is the sole owner of member ordering and each
    // repository's semantic boundary; the renderer adds no second topology model.
    let members: String = profile
        .repositories
        .iter()
        .map(|r| {
            let here = if r.id == repo.id {
                " (this session)"
            } else {
                ""
            };
            format!(
                "  - {} [role: {}; access: {}]{}\n",
                r.id, r.role, r.access, here
            )
        })
        .collect();
    let authority = if profile.authority_root_ids.is_empty() {
        "  (none declared)\n".to_string()
    } else {
        profile
            .authority_root_ids
            .iter()
            .map(|a| format!("  - {a}\n"))
            .collect()
    };
    Ok(format!(
        "<!-- pale: generated injection profile={dg} -- payload emitted by the hook mechanism; edit families + recompile, do not edit here. -->\n\
         # Pale agent surface: {role} (run policy `{run}`)\n\n\
         Use the configured decision order to find who can settle each question. Distinguish current\n\
         facts, history, accepted changes and open proposals. Hold the affected action when its\n\
         permissions or controlling source remain unresolved. Consequential actions are never automatic.\n\
         Give any subagent this effective profile ({dg}) and run policy, with the same task limits.\n\
         After activation checks pass, the host hook carries the verified execution-path identity;\n\
         these instructions alone do not activate that path.\n\n\
         Repository role: {role}\n\
         Repository access limit: {access}\n\n\
         Project `{project}` / execution workspace `{execution_workspace}` contains this repository\n\
         as one of its configured repositories. Respect each repository's ownership and permitted work:\n{members}\
         Sources for ownership and permissions (resolve their current paths and read in the declared order; keep read-only access):\n{authority}\n\
         Repository commands by permitted action (never run commands that change state automatically):\n{commands}\n\
         Current sources of rules about forbidden output tokens:\n{forbidden}\n\
         Policies, in their configured order:\n{fams}\n{ops}",
        role = repo.role,
        run = profile.run_policy.id,
        access = repo.access,
        project = profile.project_id,
        execution_workspace = profile.execution_workspace_id,
        members = members,
        authority = authority,
        commands = commands,
        forbidden = forbidden,
        fams = fams,
        ops = semantic_pointer(semantics, origin_file, skills_root)?,
    ))
}
