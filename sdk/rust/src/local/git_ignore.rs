//! Private Git common-directory observation for machine-local surface protection.

use std::path::{Path, PathBuf};

use pale_adapter_fs::{
    ConfinedRoot, LocalSourceFilesystem, MachineLocalFileObservation, ReadLimit,
};
use pale_core::paths::ConfinedRelativePath;
use pale_core::records::{
    ArtifactDigest, MachineLocalEffectKind, MachineLocalEffectTransitionV1, MachineLocalEffectV1,
    PlannedFileV1, UnixFileMode,
};
use pale_ports::{SourceFilesystemPort, SourceReadLimit};

/// Managed ignore path beneath the resolved common Git directory.
const GIT_EXCLUDE_PATH: &str = "info/exclude";
/// Maximum pre-existing exclude-file bytes admitted by local protection.
const GIT_EXCLUDE_LIMIT: ReadLimit = ReadLimit::new(1 << 20);
/// Maximum bytes admitted from a linked-worktree pointer file.
const MAX_GITFILE_BYTES: usize = 16 * 1024;
/// Opening marker delimiting the only Git-exclude block Pale owns.
const BEGIN: &str = "# >>> pale:begin managed (agent-surface local-only) >>>";
/// Closing marker delimiting the only Git-exclude block Pale owns.
const END: &str = "# <<< pale:end managed <<<";
/// Marker proving the exclude file did not predate Pale management.
const ABSENT_ORIGIN: &str = "# pale:exclude-file-origin absent";

#[derive(Debug, thiserror::Error)]
pub(super) enum LocalIgnoreError {
    #[error("filesystem observation failed")]
    Filesystem(#[from] pale_adapter_fs::FsError),
    #[error("bounded Git metadata observation failed")]
    Source(#[from] pale_ports::SourceFilesystemError),
    #[error("Git metadata observation failed")]
    Io(#[from] std::io::Error),
    #[error("Git exclude content is not UTF-8")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Git metadata is not UTF-8")]
    GitMetadataUtf8(#[from] std::string::FromUtf8Error),
    #[error("{0}")]
    Invalid(&'static str),
    #[error("local-only target is not repository-confined")]
    InvalidTarget,
}

struct PreparedLocalIgnore {
    root: ConfinedRoot,
    effect: MachineLocalEffectV1,
    intent: pale_adapter_fs::MachineLocalFileIntent,
}

/// Derive the exact apply-side Git-exclude effect without writing.
pub(super) fn plan_protection(
    repository_root: &Path,
    configured_git_root: Option<&Path>,
    local_only_targets: &[String],
) -> Result<Vec<MachineLocalEffectV1>, LocalIgnoreError> {
    Ok(
        derive_protection(repository_root, configured_git_root, local_only_targets)?
            .map(|prepared| vec![prepared.effect])
            .unwrap_or_default(),
    )
}

/// Derive the exact remove-side Git-exclude effect without writing.
pub(super) fn plan_unprotection(
    repository_root: &Path,
    configured_git_root: Option<&Path>,
) -> Result<Vec<MachineLocalEffectV1>, LocalIgnoreError> {
    Ok(derive_unprotection(repository_root, configured_git_root)?
        .map(|prepared| vec![prepared.effect])
        .unwrap_or_default())
}

/// Reconstruct and prepare the exact Git-local effect carried by an admitted plan.
pub(super) fn prepare(
    repository_root: &Path,
    configured_git_root: Option<&Path>,
    admitted: &pale_core::admission::AdmittedPlan,
) -> Result<Option<pale_adapter_fs::PreparedMachineLocalFile>, LocalIgnoreError> {
    let recomputed = match admitted.kind() {
        pale_core::records::PlanKind::Apply => {
            let targets = pale_application::plan::local_only_materialization_targets(
                &admitted.plan().operations,
            );
            derive_protection(repository_root, configured_git_root, &targets)?
        }
        pale_core::records::PlanKind::Remove => {
            derive_unprotection(repository_root, configured_git_root)?
        }
    };
    match (
        admitted.record().machine_local_effects.as_slice(),
        recomputed,
    ) {
        ([], None) => Ok(None),
        ([expected], Some(prepared)) if expected == &prepared.effect => {
            pale_adapter_fs::prepare_machine_local_file(
                prepared.root,
                admitted,
                expected,
                prepared.intent,
                GIT_EXCLUDE_LIMIT,
            )
            .map(Some)
            .map_err(Into::into)
        }
        _ => Err(LocalIgnoreError::Invalid(
            "Git protection no longer matches the admitted plan",
        )),
    }
}

fn derive_protection(
    repository_root: &Path,
    configured_git_root: Option<&Path>,
    local_only_targets: &[String],
) -> Result<Option<PreparedLocalIgnore>, LocalIgnoreError> {
    if local_only_targets.is_empty() {
        return Ok(None);
    }
    let Some(root) = resolve_common_dir(repository_root, configured_git_root)? else {
        return Ok(None);
    };
    let observed = observe(&root)?;
    let existing = observed
        .bytes
        .as_deref()
        .map(std::str::from_utf8)
        .transpose()?;
    let origin_absent = existing
        .is_none_or(|content| content.is_empty() || managed_file_origin_was_absent(content));
    let block = managed_block(local_only_targets, origin_absent)?;
    let desired = splice(existing.unwrap_or_default(), &block)?.into_bytes();
    let effect = effect(observed, desired.clone());
    Ok(Some(PreparedLocalIgnore {
        root,
        effect,
        intent: pale_adapter_fs::MachineLocalFileIntent::Write(desired),
    }))
}

fn derive_unprotection(
    repository_root: &Path,
    configured_git_root: Option<&Path>,
) -> Result<Option<PreparedLocalIgnore>, LocalIgnoreError> {
    let Some(root) = resolve_common_dir(repository_root, configured_git_root)? else {
        return Ok(None);
    };
    let observed = observe(&root)?;
    let Some(existing) = observed.bytes.as_deref() else {
        return Ok(None);
    };
    let existing = std::str::from_utf8(existing)?;
    let Some(stripped) = strip(existing)? else {
        return Ok(None);
    };
    let (transition, intent) = if stripped.remove_file {
        let prior = observed.prior.clone().ok_or(LocalIgnoreError::Invalid(
            "cannot delete an absent Git exclude file",
        ))?;
        (
            MachineLocalEffectTransitionV1::Delete { prior },
            pale_adapter_fs::MachineLocalFileIntent::Delete,
        )
    } else {
        let desired = stripped.content.into_bytes();
        (
            transition(&observed, desired.clone()),
            pale_adapter_fs::MachineLocalFileIntent::Write(desired),
        )
    };
    Ok(Some(PreparedLocalIgnore {
        root,
        effect: MachineLocalEffectV1 {
            kind: MachineLocalEffectKind::GitExcludeProtection,
            root: observed.root,
            rel_path: GIT_EXCLUDE_PATH.into(),
            retained_directories: observed.retained_parent_directories,
            transition,
        },
        intent,
    }))
}

fn observe(root: &ConfinedRoot) -> Result<MachineLocalFileObservation, LocalIgnoreError> {
    pale_adapter_fs::observe_machine_local_file(root, GIT_EXCLUDE_PATH, GIT_EXCLUDE_LIMIT)
        .map_err(Into::into)
}

fn effect(observed: MachineLocalFileObservation, desired: Vec<u8>) -> MachineLocalEffectV1 {
    MachineLocalEffectV1 {
        kind: MachineLocalEffectKind::GitExcludeProtection,
        root: observed.root.clone(),
        rel_path: GIT_EXCLUDE_PATH.into(),
        retained_directories: observed.retained_parent_directories.clone(),
        transition: transition(&observed, desired),
    }
}

fn transition(
    observed: &MachineLocalFileObservation,
    desired: Vec<u8>,
) -> MachineLocalEffectTransitionV1 {
    let mode = observed
        .prior
        .as_ref()
        .map_or(UnixFileMode::DEFAULT_FILE, |prior| prior.mode);
    let post = PlannedFileV1 {
        sha256: ArtifactDigest::hash_bytes(&desired),
        mode,
    };
    match &observed.prior {
        None => MachineLocalEffectTransitionV1::Create {
            post,
            create_directories: observed.missing_parent_directories.clone(),
        },
        Some(prior) if prior.sha256 == post.sha256 && prior.mode == post.mode => {
            MachineLocalEffectTransitionV1::Unchanged {
                state: prior.clone(),
            }
        }
        Some(prior) => MachineLocalEffectTransitionV1::Splice {
            prior: prior.clone(),
            post,
        },
    }
}

fn bounds(existing: &str) -> Result<Option<(usize, usize)>, LocalIgnoreError> {
    let mut begins = existing.match_indices(BEGIN).map(|(index, _)| index);
    let mut ends = existing.match_indices(END).map(|(index, _)| index);
    let begin = begins.next();
    let end = ends.next();
    if begins.next().is_some() || ends.next().is_some() {
        return Err(LocalIgnoreError::Invalid(
            "duplicate managed Git exclude block",
        ));
    }
    match (begin, end) {
        (None, None) => Ok(None),
        (Some(begin), Some(end)) if end >= begin => Ok(Some((begin, end + END.len()))),
        _ => Err(LocalIgnoreError::Invalid(
            "malformed managed Git exclude block",
        )),
    }
}

fn managed_file_origin_was_absent(existing: &str) -> bool {
    bounds(existing).ok().flatten().is_some_and(|(begin, end)| {
        existing[begin..end]
            .lines()
            .any(|line| line == ABSENT_ORIGIN)
    })
}

fn managed_block(paths: &[String], origin_absent: bool) -> Result<String, LocalIgnoreError> {
    let mut text = String::from(BEGIN);
    text.push('\n');
    if origin_absent {
        text.push_str(ABSENT_ORIGIN);
        text.push('\n');
    }
    for path in paths {
        let path = ConfinedRelativePath::parse(path).ok_or(LocalIgnoreError::InvalidTarget)?;
        text.push('/');
        for character in path.as_str().chars() {
            if matches!(character, '*' | '?' | '[' | ']' | '#' | '!' | ' ') {
                text.push('\\');
            }
            text.push(character);
        }
        text.push('\n');
    }
    text.push_str(END);
    Ok(text)
}

fn splice(existing: &str, block: &str) -> Result<String, LocalIgnoreError> {
    if let Some((begin, end)) = bounds(existing)? {
        return Ok(format!(
            "{}{}{}",
            &existing[..begin],
            block,
            &existing[end..]
        ));
    }
    Ok(if existing.is_empty() {
        format!("{block}\n")
    } else if existing.ends_with('\n') {
        format!("{existing}{block}\n")
    } else {
        format!("{existing}\n{block}\n")
    })
}

struct Stripped {
    content: String,
    remove_file: bool,
}

fn strip(existing: &str) -> Result<Option<Stripped>, LocalIgnoreError> {
    let Some((begin, mut end)) = bounds(existing)? else {
        return Ok(None);
    };
    let origin_absent = existing[begin..end]
        .lines()
        .any(|line| line == ABSENT_ORIGIN);
    if existing[end..].starts_with('\n') {
        end += 1;
    }
    let content = format!("{}{}", &existing[..begin], &existing[end..]);
    Ok(Some(Stripped {
        remove_file: origin_absent && content.is_empty(),
        content,
    }))
}

fn read_small_regular(path: &Path) -> Result<String, LocalIgnoreError> {
    LocalSourceFilesystem
        .read_regular(path, SourceReadLimit::new(MAX_GITFILE_BYTES))
        .map(String::from_utf8)?
        .map_err(Into::into)
}

fn one_path(value: &str) -> Result<PathBuf, LocalIgnoreError> {
    let mut lines = value.lines().filter(|line| !line.trim().is_empty());
    let line = lines.next().ok_or(LocalIgnoreError::Invalid(
        "Git pointer must contain one path",
    ))?;
    if lines.next().is_some() {
        return Err(LocalIgnoreError::Invalid(
            "Git pointer must contain one path",
        ));
    }
    Ok(PathBuf::from(line.trim()))
}

fn configured_common_dir(path: &Path) -> Result<ConfinedRoot, LocalIgnoreError> {
    if !path.is_absolute() {
        return Err(LocalIgnoreError::Invalid(
            "configured Git root is not absolute",
        ));
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LocalIgnoreError::Invalid(
            "configured Git root is not a directory",
        ));
    }
    ConfinedRoot::open(path).map_err(Into::into)
}

fn linked_common_dir(
    repository_root: &Path,
    gitfile: &Path,
    configured: &Path,
) -> Result<ConfinedRoot, LocalIgnoreError> {
    let value = read_small_regular(gitfile)?;
    let pointer = value
        .strip_prefix("gitdir:")
        .ok_or(LocalIgnoreError::Invalid(
            "checkout Git file lacks gitdir prefix",
        ))?
        .trim();
    if pointer.is_empty() || pointer.contains(['\n', '\r', '\0']) {
        return Err(LocalIgnoreError::Invalid(
            "checkout gitdir pointer is invalid",
        ));
    }
    let git_dir = Path::new(pointer);
    let git_dir = std::fs::canonicalize(if git_dir.is_absolute() {
        git_dir.to_path_buf()
    } else {
        repository_root.join(git_dir)
    })?;
    let backlink = std::fs::canonicalize(one_path(&read_small_regular(&git_dir.join("gitdir"))?)?)?;
    if backlink != std::fs::canonicalize(gitfile)? {
        return Err(LocalIgnoreError::Invalid(
            "linked-worktree backlink does not match checkout",
        ));
    }
    let common = match std::fs::symlink_metadata(git_dir.join("commondir")) {
        Ok(_) => {
            let relative = one_path(&read_small_regular(&git_dir.join("commondir"))?)?;
            std::fs::canonicalize(if relative.is_absolute() {
                relative
            } else {
                git_dir.join(relative)
            })?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => git_dir,
        Err(error) => return Err(error.into()),
    };
    let actual = ConfinedRoot::open(&common)?;
    let expected = configured_common_dir(configured)?;
    if actual.canonical_path() != expected.canonical_path() {
        return Err(LocalIgnoreError::Invalid(
            "linked-worktree common Git root mismatches binding",
        ));
    }
    Ok(actual)
}

fn resolve_common_dir(
    repository_root: &Path,
    configured: Option<&Path>,
) -> Result<Option<ConfinedRoot>, LocalIgnoreError> {
    let repository_root = std::fs::canonicalize(repository_root)?;
    let git = repository_root.join(".git");
    let metadata = match std::fs::symlink_metadata(&git) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        return Err(LocalIgnoreError::Invalid(
            "repository .git entry is a symlink",
        ));
    }
    if metadata.is_dir() {
        let actual = ConfinedRoot::open(&git)?;
        if let Some(configured) = configured {
            let expected = configured_common_dir(configured)?;
            if actual.canonical_path() != expected.canonical_path() {
                return Err(LocalIgnoreError::Invalid(
                    "repository Git root mismatches binding",
                ));
            }
        }
        return Ok(Some(actual));
    }
    if metadata.is_file() {
        let configured = configured.ok_or(LocalIgnoreError::Invalid(
            "linked worktree requires a machine-bound Git root",
        ))?;
        return linked_common_dir(&repository_root, &git, configured).map(Some);
    }
    Err(LocalIgnoreError::Invalid(
        "repository .git entry has unsupported kind",
    ))
}
