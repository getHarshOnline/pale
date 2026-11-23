//! Shared sovereign-workspace fixtures for application integration tests.

use std::path::Path;

use pale_core::records::{WorkspaceId, WorkspaceResolutionLockV1, WorkspaceUniverseLockV1};

/// Write the explicit empty resolution and adoption frontier for one workspace.
///
/// The empty records are real initial-v1 source contracts, not compiler
/// defaults. Tests therefore exercise the same root-directed admission path as
/// a populated workspace without fabricating publisher evidence.
pub(crate) fn write_empty_universe(root: &Path, workspace_id: &str) {
    let resolution = WorkspaceResolutionLockV1::seal(Vec::new())
        .expect("empty resolution frontier is canonical");
    let universe = WorkspaceUniverseLockV1::seal(
        WorkspaceId::parse(workspace_id).expect("fixture workspace identity is portable"),
        resolution.resolution_digest.clone(),
        Vec::new(),
    )
    .expect("empty sovereign universe is canonical");
    std::fs::write(
        root.join("workspace-resolution.lock.toml"),
        toml::to_string(&resolution).expect("serialize empty resolution lock"),
    )
    .expect("write empty resolution lock");
    std::fs::write(
        root.join("workspace-universe.lock.toml"),
        toml::to_string(&universe).expect("serialize empty universe lock"),
    )
    .expect("write empty universe lock");
}
