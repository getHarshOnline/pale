//! Private repository context from declared scope and ordered source references.
//!
//! This document is not a repository surface. Machine paths stay outside the
//! portable Package projection and its public instruction-file contribution.

use pale_core::records::{ExecutionWorkspaceRepositoryAccessV1, LogicalRepositoryIdV1};

use super::{RenderError, SourceGuidanceError};
use crate::compile::EffectiveProfileCompilation;

pub(crate) fn render_repository_context(
    compilation: &EffectiveProfileCompilation,
    repository_id: &LogicalRepositoryIdV1,
) -> Result<String, SourceGuidanceError> {
    let profile = compilation.profile();
    let repository = profile
        .repositories
        .iter()
        .find(|repository| &repository.id == repository_id)
        .ok_or(SourceGuidanceError::RepositoryNotSelected)?;
    let access = match repository.access {
        ExecutionWorkspaceRepositoryAccessV1::Read => "read only",
        ExecutionWorkspaceRepositoryAccessV1::Mutate => {
            "changes remain subject to the task and native tool permissions"
        }
    };
    let mut document = format!(
        "# Private repository context\n\nWorkspace: `{}`. Project: `{}`. Execution workspace: `{}`. Repository: `{repository_id}`.\n\nConfigured repository limit: {access}. Working role: `{}`.\n\nThis context contains private configured paths. Keep it within the selected project's disclosure scope, outside public repository files. Compilation has not checked these source files on this machine or admitted a host session. Read current sources only through permitted native tools; this document grants no source writes, publication or runtime execution.\n",
        profile.workspace_id, profile.project_id, profile.execution_workspace_id, repository.role,
    );
    let Some(session) = profile
        .session
        .as_ref()
        .filter(|session| session.activation_roots.contains(repository_id))
    else {
        document.push_str("\nNo source reading order is declared for this repository entry.\n");
        return Ok(document);
    };
    if session.reading_order.is_empty() {
        document.push_str("\nNo source reading order is declared for this repository entry.\n");
        return Ok(document);
    }
    document.push_str("\n## Read the current sources in this order\n\nThese sources are read only. Their own owners retain their facts and decisions. A path does not identify a project owner or supply acceptance of a duty.\n\nEach entry preserves the configured root and relative reference separately. Check the root through permitted native tools: a directory root resolves references beneath that directory; a file root resolves them from its parent but permits only the exact bound file. This output does not determine the root's kind or verify the resulting file.\n\n");
    for (index, source) in session.reading_order.iter().enumerate() {
        if !session
            .authority_access
            .iter()
            .any(|access| access.root_id == source.root_id)
        {
            return Err(RenderError::invalid_input(
                "session.reading_order",
                "a required source root has no declared read-only access",
            )
            .into());
        }
        let root = compilation
            .instance()
            .authority_roots
            .get(&source.root_id)
            .ok_or_else(|| {
                RenderError::invalid_input(
                    "session.reading_order",
                    "a required source root has no selected machine binding",
                )
            })?;
        document.push_str(&format!(
            "{}. Root {:?}, binding {root:?}, relative reference {:?}.\n",
            index + 1,
            source.root_id,
            source.rel_path,
        ));
    }
    Ok(document)
}
