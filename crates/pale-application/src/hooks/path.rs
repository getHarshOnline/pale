//! Filesystem facts used by path-sensitive lifecycle predicates.

use std::path::{Path, PathBuf};

use pale_ports::MachineFilesystemPort;

/// Whether a structured host target exists before the proposed tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPresence {
    /// Metadata lookup proved an existing filesystem object at the target.
    Present,
    /// Metadata lookup returned [`std::io::ErrorKind::NotFound`] for an absolute target.
    Missing,
    /// Relative resolution was unsafe or metadata could not prove either state.
    Unknown,
}

/// One host-reported target and its pre-action filesystem state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPathFact {
    /// Exact structured path reported by the native harness.
    pub path: String,
    /// Conservative pre-action presence classification.
    pub presence: TargetPresence,
}

fn absolute_target(path: &str, cwd: Option<&str>) -> Option<PathBuf> {
    let path = Path::new(path);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    let cwd = Path::new(cwd?);
    cwd.is_absolute().then(|| cwd.join(path))
}

/// Classify exact structured targets from one requested-entry inspection.
///
/// Success proves only that the directory entry was observed at this instant.
/// [`std::io::ErrorKind::NotFound`] proves absence; other failures and unsafe
/// relative paths stay unknown. This query does not canonicalize its input. No
/// result is a retained handle or a guarantee about a later mutation.
pub fn target_path_facts(
    filesystem: &dyn MachineFilesystemPort,
    paths: &[String],
    cwd: Option<&str>,
) -> Vec<TargetPathFact> {
    paths
        .iter()
        .map(|path| {
            let presence = absolute_target(path, cwd)
                .map(|target| match filesystem.inspect_requested(&target) {
                    Ok(_) => TargetPresence::Present,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        TargetPresence::Missing
                    }
                    Err(_) => TargetPresence::Unknown,
                })
                .unwrap_or(TargetPresence::Unknown);
            TargetPathFact {
                path: path.clone(),
                presence,
            }
        })
        .collect()
}
