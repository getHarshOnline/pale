//! Filesystem confinement for existing and not-yet-created write targets.

use std::path::{Component, Path, PathBuf};

use pale_ports::MachineFilesystemPort;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathRelation {
    ReadOnly,
    Outside,
    Unknown,
}

/// Normalize lexical components, rejecting traversal above the available root.
pub(super) fn lexical_normalize(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
        }
    }
    Some(normalized)
}

/// Resolve through the nearest observed ancestor for conservative containment.
///
/// This classifies the pre-action path, including a symlinked parent of a missing
/// leaf. It retains no handle and does not guarantee the path of a later mutation.
pub(super) fn resolve_target(
    filesystem: &dyn MachineFilesystemPort,
    path: &str,
    cwd: Option<&str>,
) -> Option<PathBuf> {
    let raw = Path::new(path);
    let absolute = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        let cwd = Path::new(cwd?);
        if !cwd.is_absolute() {
            return None;
        }
        cwd.join(raw)
    };
    // Retain the original component sequence for native lookup: a symbolic link
    // followed by `..` is resolved by the filesystem, not by lexical spelling.
    // The lexical pass only preserves the existing above-root rejection law.
    lexical_normalize(&absolute)?;

    let mut existing = absolute.as_path();
    let mut tail = Vec::new();
    loop {
        match filesystem.inspect_requested(existing) {
            Ok(_) => {
                let mut resolved = filesystem.canonicalize(existing).ok()?;
                for component in tail.into_iter().rev() {
                    resolved.push(component);
                }
                return lexical_normalize(&resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tail.push(existing.file_name()?.to_os_string());
                existing = existing.parent()?;
            }
            Err(_) => return None,
        }
    }
}

fn path_relation(
    filesystem: &dyn MachineFilesystemPort,
    path: &str,
    cwd: Option<&str>,
    readonly_dirs: &[PathBuf],
) -> PathRelation {
    if path
        .bytes()
        .any(|b| matches!(b, b'$' | b'`' | b'*' | b'?' | b'[' | b']' | b'{' | b'}'))
    {
        return PathRelation::Unknown;
    }
    let Some(target) = resolve_target(filesystem, path, cwd) else {
        return PathRelation::Unknown;
    };
    for readonly in readonly_dirs {
        if target.starts_with(readonly) {
            return PathRelation::ReadOnly;
        }
    }
    PathRelation::Outside
}

/// Classify one write target, failing closed when its destination is ambiguous.
pub(super) fn path_violation(
    filesystem: &dyn MachineFilesystemPort,
    path: &str,
    cwd: Option<&str>,
    readonly_dirs: &[PathBuf],
) -> Result<Option<String>, String> {
    match path_relation(filesystem, path, cwd, readonly_dirs) {
        PathRelation::ReadOnly => Ok(Some(format!(
            "a write resolves into sealed read-only path `{path}`"
        ))),
        PathRelation::Outside => Ok(None),
        PathRelation::Unknown => Err(format!(
            "write target `{path}` cannot be resolved against the sealed read-only roots"
        )),
    }
}
