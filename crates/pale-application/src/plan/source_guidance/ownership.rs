//! Private local-writer metadata; never a provider or runtime authority record.

use pale_core::records::{
    ArtifactDigest, ExecutionWorkspaceRepositoryAccessV1, LogicalExecutionWorkspaceIdV1,
    LogicalProjectIdV1, LogicalRepositoryIdV1, MachineLocalRootIdentity, UnixFileMode, WorkspaceId,
};
use pale_ports::MachineLocalFileObservation;
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Owner {
    pub(super) workspace: WorkspaceId,
    pub(super) project: LogicalProjectIdV1,
    pub(super) execution_workspace: LogicalExecutionWorkspaceIdV1,
    pub(super) repository: LogicalRepositoryIdV1,
    pub(super) host: SourceGuidanceHost,
}

#[derive(Serialize, Deserialize)]
enum Format {
    #[serde(rename = "pale-private-source-guidance")]
    SourceGuidance,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OwnedFile {
    pub(super) path: String,
    pub(super) sha256: ArtifactDigest,
    pub(super) mode: UnixFileMode,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EntryOwnership {
    /// Includes only the exact contributed separator/markers/body, never local facts.
    pub(super) block: String,
    pub(super) created_file: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ownership {
    format: Format,
    version: u32,
    owner_sha256: ArtifactDigest,
    root_sha256: ArtifactDigest,
    pub(super) projection: ArtifactDigest,
    pub(super) files: Vec<OwnedFile>,
    pub(super) entry: EntryOwnership,
}

impl Ownership {
    pub(super) fn entry_path(&self, host: SourceGuidanceHost) -> &'static str {
        if self.version == 1 && host == SourceGuidanceHost::ClaudeCode {
            "CLAUDE.md"
        } else {
            host.adapter().projection_layout().front_door()
        }
    }

    pub(super) fn proposed(
        owner: Owner,
        root: MachineLocalRootIdentity,
        projection: &SourceGuidanceProjection,
        entry: EntryOwnership,
    ) -> Result<Self, GuidancePlanError> {
        Ok(Self {
            format: Format::SourceGuidance,
            version: 2,
            owner_sha256: identity_digest("owner", &owner)?,
            root_sha256: identity_digest("root", &root)?,
            projection: projection.digest().clone(),
            files: projection
                .files()
                .iter()
                .map(|file| OwnedFile {
                    path: file.rel_path().to_owned(),
                    sha256: file.sha256().clone(),
                    mode: UnixFileMode::DEFAULT_FILE,
                })
                .collect(),
            entry,
        })
    }
}

pub(super) fn prepare(
    compilation: &EffectiveProfileCompilation,
    repository: &LogicalRepositoryIdV1,
    host: SourceGuidanceHost,
    intent: GuidancePlanIntent,
    observation: MachineLocalFileObservation,
) -> Result<SourceGuidancePreparation, GuidancePlanError> {
    let profile = compilation.profile();
    let selected = profile
        .repositories
        .iter()
        .find(|entry| &entry.id == repository)
        .ok_or(crate::render::SourceGuidanceError::RepositoryNotSelected)?;
    if selected.access != ExecutionWorkspaceRepositoryAccessV1::Mutate {
        return Err(GuidancePlanError::ReadOnly);
    }
    let declared_root = compilation
        .instance()
        .repositories
        .get(repository)
        .ok_or(GuidancePlanError::Baseline("selected checkout is absent"))?;
    if declared_root != &observation.root.canonical_path {
        return Err(GuidancePlanError::Baseline(
            "selected checkout must match the observed canonical root",
        ));
    }
    let owner = Owner {
        workspace: profile.workspace_id.clone(),
        project: profile.project_id.clone(),
        execution_workspace: profile.execution_workspace_id.clone(),
        repository: repository.clone(),
        host,
    };
    let path = ownership_path(host);
    planning::validate_observation(&path, &observation.root, &observation)?;
    let state = match &observation.bytes {
        None => None,
        Some(bytes) => {
            if bytes.len() > MAX_OWNERSHIP_BYTES {
                return Err(GuidancePlanError::Limit("ownership bytes"));
            }
            let state: Ownership =
                serde_json::from_slice(bytes).map_err(GuidancePlanError::Decode)?;
            if !matches!(state.version, 1 | 2)
                || state.owner_sha256 != identity_digest("owner", &owner)?
                || state.root_sha256 != identity_digest("root", &observation.root)?
            {
                return Err(GuidancePlanError::Ownership(
                    "version, owner or native root differs",
                ));
            }
            if state.files.len() > MAX_GUIDANCE_FILES {
                return Err(GuidancePlanError::Limit("owned files"));
            }
            let mut previous: Option<&str> = None;
            for file in &state.files {
                if !owned_path(host, &file.path)
                    || previous.is_some_and(|prior| prior >= file.path.as_str())
                {
                    return Err(GuidancePlanError::Ownership(
                        "file paths are invalid, duplicated or unordered",
                    ));
                }
                previous = Some(&file.path);
            }
            entrypoint::validate_owned_block(host, &state.entry.block)?;
            Some(state)
        }
    };
    let requested = match intent {
        GuidancePlanIntent::Apply => {
            if state.as_ref().is_some_and(|state| {
                state.entry_path(host) != host.adapter().projection_layout().front_door()
            }) {
                return Err(GuidancePlanError::Ownership(
                    "earlier Claude guidance uses CLAUDE.md; review guidance plan --intent remove and apply that removal, then plan installation into AGENTS.md",
                ));
            }
            let projection = compilation.source_guidance(repository, host)?;
            if projection.files().len() > MAX_GUIDANCE_FILES {
                return Err(GuidancePlanError::Limit("selected files"));
            }
            if projection
                .files()
                .iter()
                .any(|file| !owned_path(host, file.rel_path()))
            {
                return Err(GuidancePlanError::Ownership(
                    "projection contains a target outside the guidance namespace",
                ));
            }
            RequestedChange::Apply(projection)
        }
        GuidancePlanIntent::Remove => RequestedChange::Remove,
    };
    Ok(SourceGuidancePreparation {
        context: owner,
        profile_digest: profile.profile_digest.clone(),
        root: observation.root.clone(),
        host,
        requested,
        state,
        state_observation: observation,
    })
}

/// Old metadata can name only files belonging to this exact native layout.
pub(super) fn owned_path(host: SourceGuidanceHost, path: &str) -> bool {
    if path.len() > pale_core::paths::MAX_CONFINED_RELATIVE_PATH_BYTES
        || pale_core::paths::ConfinedRelativePath::parse(path).is_none()
    {
        return false;
    }
    let reference_root = format!(".pale/guidance/{}/", host.adapter().as_str());
    if path == format!("{reference_root}index.md") {
        return true;
    }
    if let Some(name) = path.strip_prefix(&format!("{reference_root}roles/")) {
        return !name.contains('/') && name.starts_with("pale-role-") && name.ends_with(".md");
    }
    let skills_root = host.adapter().projection_layout().skills_root();
    let Some(relative) = path.strip_prefix(&format!("{skills_root}/")) else {
        return false;
    };
    let parts = relative.split('/').collect::<Vec<_>>();
    let Some(name) = parts.first() else {
        return false;
    };
    if !name.starts_with("pale-")
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return false;
    }
    parts.len() == 2 && parts[1] == "SKILL.md"
        || host == SourceGuidanceHost::Codex
            && parts.len() == 3
            && parts[1..] == ["agents", "openai.yaml"]
}

pub(super) fn identity_digest<T: Serialize>(
    kind: &str,
    value: &T,
) -> Result<ArtifactDigest, GuidancePlanError> {
    let preimage = serde_json::json!({"domain":"pale.private-source-guidance-identity.v1", "kind":kind, "value":value});
    Ok(ArtifactDigest::hash_bytes(
        pale_core::canon::to_canonical_string(&preimage)?.as_bytes(),
    ))
}
