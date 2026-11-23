//! Repository front doors preserve authority pointers and selected path order.

use pale_core::records::{
    EffectiveProjectProfileV1, ExecutionPathSelectionV1, ProfileRepository,
    ResolvedProjectInstanceV1,
};
use pale_core::{MANAGED_BEGIN as FRONT_BEGIN, MANAGED_END as FRONT_END};

use super::semantic_helpers::short;
use super::{RenderError, RenderIdentity};

/// Render the bounded repository entrypoint and its truthful availability summary.
pub(super) fn front_door(
    hosts: &str,
    repo: &ProfileRepository,
    profile: &EffectiveProjectProfileV1,
    instance: &ResolvedProjectInstanceV1,
    identity: &RenderIdentity<'_>,
    paths: &[&ExecutionPathSelectionV1],
) -> Result<String, RenderError> {
    let mut auth = String::new();
    for (lid, path) in &instance.authority_roots {
        auth.push_str(&format!("  - {lid}: {path}\n"));
    }
    // Session reading order (at every activation root): each pointer's authority
    // root resolved to its absolute path + the relative file, in declared order,
    // for the host to read FRESH at SessionStart. Empty for non-activation repos.
    let reading = match &profile.session {
        Some(s)
            if s.activation_roots.iter().any(|root| root == &repo.id)
                && !s.reading_order.is_empty() =>
        {
            let mut r = String::from("At session start, read the current files in this order:\n");
            for (n, ro) in s.reading_order.iter().enumerate() {
                let base = instance.authority_roots.get(&ro.root_id).ok_or_else(|| {
                    RenderError::invalid_input(
                        "session.reading_order",
                        format!(
                            "authority root `{}` has no resolved machine path",
                            ro.root_id
                        ),
                    )
                })?;
                r.push_str(&format!(
                    "  {}. {}/{}\n",
                    n + 1,
                    base.trim_end_matches('/'),
                    ro.rel_path
                ));
            }
            r.push('\n');
            r
        }
        _ => String::new(),
    };
    Ok(format!(
        "{begin}\n# Pale entry point: {role} ({host})\n\n\
         Generated instructions for profile `{dg}` (machine instance `{idg}`). The verified\n\
         session instructions carry the active policy. Change its Package source or private\n\
         project configuration and recompile to update it.\n\n\
         Repository membership:\n\
         - Access limit: {access}\n- Role: {role}\n- Run policy: {run}\n\n\
         Sources for ownership and permissions (absolute paths; check current facts):\n{auth}\n\
         {reading}Find tasks, specialist roles, guides, session modes and tools in `.pale/semantic-index.md`.\n\
         Load them when selected. The runtime delivers the verified session instructions at the\n\
         configured activation root only after checking the selected execution path, host and machine.\n\
         This file describes that setup; activation requires the runtime check. Delegated work follows\n\
         its configured rules for joining results and the permissions of each owner.\n\n\
         Selected execution paths: {targets}. Give every child host launched under this\n\
         profile the same decision rules and action limits. Consequential actions are never automatic.\n\
         {end}\n",
        begin = FRONT_BEGIN,
        end = FRONT_END,
        host = hosts,
        role = repo.role,
        dg = short(identity.profile().as_str()),
        idg = short(identity.instance().as_str()),
        run = profile.run_policy.id,
        access = repo.access,
        auth = auth,
        reading = reading,
        targets = paths
            .iter()
            .map(|target| target.id().to_string())
            .collect::<Vec<_>>()
            .join(", "),
    ))
}
