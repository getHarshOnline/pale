//! Source-admitted Package guidance, distinct from an admitted runtime surface.
//!
//! This projection is usable before a deployment exists. It reuses the Package
//! compiler and host content renderers, but never manufactures execution-path
//! witnesses, provider generations, native agents, hooks or invocation grants.

use std::collections::BTreeMap;

use pale_core::records::{
    ArtifactDigest, ExecutionWorkspaceRepositoryAccessV1, LogicalRepositoryIdV1, ProfileDigest,
};

use super::{
    doctrine_content::doctrine_skill,
    family_content::{ordered_families, skill_content},
    role_content::role_reference,
    semantic_helpers::{doctrine_name, host_name, role_name},
    RenderError,
};
use crate::{compile::EffectiveProfileCompilation, terminal_adapters::TerminalAdapterDescriptor};

/// Explicit native layout for guidance, without a claim that the host exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceGuidanceHost {
    /// Codex skills and a proposed `AGENTS.md` contribution.
    Codex,
    /// Claude Code skills and a proposed shared `AGENTS.md` contribution.
    ClaudeCode,
}

/// Closed refusal at the source-guidance boundary, before any filesystem write.
#[derive(Debug, thiserror::Error)]
pub enum SourceGuidanceError {
    /// The supplied logical repository is outside the retained profile.
    #[error("repository is not selected by this effective profile")]
    RepositoryNotSelected,
    /// A source-only result cannot fulfill selected runtime duties.
    #[error(
        "operations, recipes, modes and lifecycle delivery/gates require the runtime projection; select a source-guidance profile explicitly rather than dropping them"
    )]
    RuntimeSelection,
    /// Retained semantic content could not be projected faithfully.
    #[error(transparent)]
    Content(#[from] RenderError),
}

impl SourceGuidanceError {
    /// Application-owned failure classification for outer diagnostics.
    #[must_use]
    pub const fn failure_disposition(&self) -> crate::ApplicationFailureDisposition {
        match self {
            Self::RepositoryNotSelected | Self::RuntimeSelection => {
                crate::ApplicationFailureDisposition::InvalidInput
            }
            Self::Content(source) => source.failure_disposition(),
        }
    }
}

impl SourceGuidanceHost {
    pub(crate) fn adapter(self) -> TerminalAdapterDescriptor {
        match self {
            Self::Codex => TerminalAdapterDescriptor::codex(),
            Self::ClaudeCode => TerminalAdapterDescriptor::claude_code(),
        }
    }
}

/// Immutable proposed file with exact content identity; not a write permit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceGuidanceFile {
    rel_path: String,
    content: String,
    sha256: ArtifactDigest,
}

impl SourceGuidanceFile {
    /// Repository-relative destination. No machine checkout is embedded.
    #[must_use]
    pub fn rel_path(&self) -> &str {
        &self.rel_path
    }

    /// Complete proposed UTF-8 bytes.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Hash of those exact bytes.
    #[must_use]
    pub fn sha256(&self) -> &ArtifactDigest {
        &self.sha256
    }
}

/// Retained source-guidance result with a separately proposed entrypoint link.
///
/// There is deliberately no runtime generation, readiness, execution-path
/// consumer, operation or machine authority in this type. It cannot be passed
/// to the runtime surface planner. The digest is content identity, not a
/// signature, lease or permission to write. Integrators must preserve existing
/// project instructions and qualify exact file ownership before materializing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceGuidanceProjection {
    repository_id: LogicalRepositoryIdV1,
    host: SourceGuidanceHost,
    profile_digest: ProfileDigest,
    digest: ArtifactDigest,
    entrypoint_path: String,
    entrypoint_contribution: String,
    files: Vec<SourceGuidanceFile>,
}

impl SourceGuidanceProjection {
    /// Selected logical repository; never inferred from a directory name.
    #[must_use]
    pub fn repository_id(&self) -> &LogicalRepositoryIdV1 {
        &self.repository_id
    }

    /// Selected host layout; no executable observation is implied.
    #[must_use]
    pub fn host(&self) -> SourceGuidanceHost {
        self.host
    }

    /// Exact source-compiled profile that selected these Package resources.
    #[must_use]
    pub fn profile_digest(&self) -> &ProfileDigest {
        &self.profile_digest
    }

    /// Domain-separated identity of sources, selection, contribution and files.
    #[must_use]
    pub fn digest(&self) -> &ArtifactDigest {
        &self.digest
    }

    /// Native project instruction file to which the contribution belongs.
    #[must_use]
    pub fn entrypoint_path(&self) -> &str {
        &self.entrypoint_path
    }

    /// Proposed addition only. This is not ownership of the entrypoint file.
    #[must_use]
    pub fn entrypoint_contribution(&self) -> &str {
        &self.entrypoint_contribution
    }

    /// Deterministically sorted proposed complete files, excluding entrypoint.
    #[must_use]
    pub fn files(&self) -> &[SourceGuidanceFile] {
        &self.files
    }
}

fn add_file(
    files: &mut BTreeMap<String, SourceGuidanceFile>,
    rel_path: String,
    content: String,
) -> Result<(), RenderError> {
    let file = SourceGuidanceFile {
        sha256: ArtifactDigest::hash_bytes(content.as_bytes()),
        content,
        rel_path: rel_path.clone(),
    };
    if files.insert(rel_path.clone(), file).is_some() {
        return Err(RenderError::projection(
            "source guidance",
            format!("selected resources collide at `{rel_path}`"),
        ));
    }
    Ok(())
}

pub(crate) fn render_source_guidance(
    compilation: &EffectiveProfileCompilation,
    repository_id: &LogicalRepositoryIdV1,
    host: SourceGuidanceHost,
) -> Result<SourceGuidanceProjection, SourceGuidanceError> {
    let profile = compilation.profile();
    let position = profile
        .repositories
        .iter()
        .position(|repository| &repository.id == repository_id)
        .ok_or(SourceGuidanceError::RepositoryNotSelected)?;
    let repository = &profile.repositories[position];
    let semantics = &compilation.repository_semantic_plans()[position];
    if !semantics.operations.is_empty()
        || !semantics.recipes.is_empty()
        || !semantics.run_modes.is_empty()
        || !profile.lifecycle.is_empty()
        || profile.run_policy.action_gate
        || profile.run_policy.session_inject
    {
        return Err(SourceGuidanceError::RuntimeSelection);
    }
    let adapter = host.adapter();
    let layout = adapter.projection_layout();
    let skills_root = layout.skills_root();
    let reference_root = format!(".pale/guidance/{}", adapter.as_str());
    let index_path = format!("{reference_root}/index.md");
    let mut files = BTreeMap::new();
    let access = match repository.access {
        ExecutionWorkspaceRepositoryAccessV1::Read => "read only",
        ExecutionWorkspaceRepositoryAccessV1::Mutate => {
            "changes allowed within tool permissions and project approval"
        }
    };
    let mut index = format!(
        "# Selected Package guidance\n\nRepository: `{repository_id}`. Configured access limit: {access}.\n\nRead project instructions for local facts and limits. These Package references supply reusable methods. Run the work with the project’s configured tools and permissions. This installation contains guidance only; a running Pale service, hooks, providers and agents require runtime setup.\n\nRead the repository role and selected policies below. Open every Read first link before applying a method. When guidance is included in the page, use that copy; included text does not replace its Read first links. Follow optional references when their subject matters to the request.\n\n## Repository role\n\n",
    );
    let primary_role = semantics
        .roles
        .iter()
        .find(|role| role.record.id == repository.role)
        .ok_or_else(|| {
            RenderError::invalid_input("source guidance role", "selected role is missing")
        })?;
    let primary_name = role_name(primary_role);
    index.push_str(&format!(
        "[{}](roles/{primary_name}.md)\n",
        primary_role.record.description,
    ));
    for role in &semantics.roles {
        let path = format!("{reference_root}/roles/{}.md", role_name(role));
        let content = role_reference(role, semantics, &path, skills_root)?;
        add_file(&mut files, path, content)?;
    }
    if semantics.roles.len() > 1 {
        index.push_str("\nOther roles, when their task is needed. Use the shared guidance included in each role and open its Read first links:\n\n");
        for role in semantics
            .roles
            .iter()
            .filter(|role| role.record.id != repository.role)
        {
            index.push_str(&format!(
                "- [{}](roles/{}.md)\n",
                role.record.description,
                role_name(role),
            ));
        }
    }
    index.push_str("\n## Selected policies\n\n");
    for selected in ordered_families(semantics)? {
        let family = selected.family;
        let name = host_name(&family.id);
        let path = format!("{skills_root}/{name}/SKILL.md");
        index.push_str(&format!(
            "- [{}](../../../{path}): {}\n",
            family.title, family.purpose,
        ));
        add_file(
            &mut files,
            path,
            skill_content(family, profile.profile_digest.as_str(), adapter, semantics)?,
        )?;
        if host == SourceGuidanceHost::Codex {
            add_file(
                &mut files,
                format!("{skills_root}/{name}/agents/openai.yaml"),
                "policy:\n  allow_implicit_invocation: false\n".to_owned(),
            )?;
        }
    }
    if !semantics.startup_policies.is_empty() {
        index.push_str("\n## Required repository guidance\n\n");
        for policy in &semantics.startup_policies {
            let body =
                super::body_links::body_at(&policy.body, semantics, &index_path, skills_root)?;
            let reading = super::body_links::navigation_at(
                &policy.record.id,
                pale_core::records::PackageResourceKindV1::Doctrine,
                semantics,
                &index_path,
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
            index.push_str(&format!(
                "### {}\n\n{reading}{}\n\n{sources}",
                policy.title,
                body.trim()
            ));
        }
    }
    if !semantics.doctrine.is_empty() {
        index.push_str("\n## Task references\n\nOpen the relevant subject and its Read first links. When the method is already included in a role or guide, use that copy and still follow its required reading.\n\n");
        for doctrine in &semantics.doctrine {
            let path = format!("{skills_root}/{}/SKILL.md", doctrine_name(doctrine));
            index.push_str(&format!(
                "- [{}](../../../{path})\n",
                doctrine.record.description,
            ));
            add_file(&mut files, path, doctrine_skill(doctrine, semantics)?)?;
        }
    }
    if semantics.stacks.iter().any(|stack| stack.body.is_some()) {
        index.push_str("\n## Stacks\n\n");
    }
    for stack in &semantics.stacks {
        if let Some(content) = super::stack_content::stack_skill(stack, semantics)? {
            let path = format!(
                "{skills_root}/{}/SKILL.md",
                super::semantic_helpers::stack_name(&stack.id)
            );
            index.push_str(&format!("- [{}](../../../{path})\n", stack.description));
            add_file(&mut files, path, content)?;
        }
    }
    add_file(&mut files, index_path.clone(), index)?;
    let entrypoint_contribution = format!(
        "Before doing the work, read the [repository role]({reference_root}/roles/{primary_name}.md) and the applicable policy bodies in the [selected Package guidance]({index_path}); their link labels and descriptions do not replace those instructions. Keep project facts, local choices and tool configuration with the project that owns them.\n",
    );
    let entrypoint_path = layout.front_door().to_owned();
    let files = files.into_values().collect::<Vec<_>>();
    let preimage = serde_json::json!({
        "domain": "pale.source-guidance.v1",
        "profile": profile.profile_digest,
        "selected_topology": compilation.selected_topology_digest(),
        "principal_universe": compilation.principal_admitted_universe_digest(),
        "workspace_semantics": compilation.workspace_semantic_ir_digest(),
        "workspace_evidence": compilation.workspace_admission_evidence_digest(),
        "schema_registry": compilation.schema_registry_identity(),
        "distribution_lock": compilation.distribution_lock_digest(),
        "repository": repository_id,
        "host": adapter.registry_id(),
        "entrypoint_path": entrypoint_path,
        "entrypoint_contribution": entrypoint_contribution,
        "files": files.iter().map(|file| (&file.rel_path, &file.sha256)).collect::<Vec<_>>(),
    });
    let bytes = serde_json::to_vec(&preimage)
        .map_err(|error| RenderError::projection_error("source guidance identity", error))?;
    Ok(SourceGuidanceProjection {
        repository_id: repository_id.clone(),
        host,
        profile_digest: profile.profile_digest.clone(),
        digest: ArtifactDigest::hash_bytes(&bytes),
        entrypoint_path,
        entrypoint_contribution,
        files,
    })
}
