//! Canonical captured one-shot fixture shared by boundary and integrity tests.

use super::{
    compile_captured_portable_inputs, compile_portable_workspace, CapturedOneShotCompilationInputs,
    CapturedPackageDocument, CapturedPackageSource, CapturedPrivateDocument,
    CapturedPrivateWorkspaceInputs, CapturedWorkspaceDocument, CapturedWorkspaceSource,
    Compilation,
};

pub(crate) fn private_document(label: &str, bytes: Vec<u8>) -> CapturedPrivateDocument {
    CapturedPrivateDocument::new(label.to_owned(), bytes)
        .expect("test private document is individually admissible")
}

pub(crate) fn shared_captured_fixture_compilation() -> Compilation {
    let private = CapturedPrivateWorkspaceInputs::new(
        private_document(
            "capabilities.toml",
            include_bytes!("../fixtures/cross-leaf.v1/one-shot/private/capabilities.toml").to_vec(),
        ),
        private_document(
            "machines/machine.toml",
            include_bytes!("../fixtures/cross-leaf.v1/one-shot/private/machine-binding.toml")
                .to_vec(),
        ),
        private_document(
            "distribution-lock.toml",
            include_bytes!("../fixtures/cross-leaf.v1/one-shot/private/distribution-lock.toml")
                .to_vec(),
        ),
        Vec::new(),
    )
    .expect("shared private fixture is bounded");
    let packages = CapturedPackageSource::new(vec![
        CapturedPackageDocument::new(
            "engine/package.toml".into(),
            include_bytes!("../fixtures/cross-leaf.v1/one-shot/packages/engine/package.toml")
                .to_vec(),
        )
        .expect("shared Package manifest is bounded"),
        CapturedPackageDocument::new(
            "engine/roles.toml".into(),
            include_bytes!("../fixtures/cross-leaf.v1/one-shot/packages/engine/roles.toml")
                .to_vec(),
        )
        .expect("shared role catalog is bounded"),
        CapturedPackageDocument::new(
            "engine/role.md".into(),
            include_bytes!("../fixtures/cross-leaf.v1/one-shot/packages/engine/role.md").to_vec(),
        )
        .expect("shared role body is bounded"),
    ])
    .expect("shared Package fixture is bounded");
    let workspace_root = CapturedWorkspaceSource::admit_root_manifest("pale-workspace.toml".into())
        .expect("shared workspace root is confined");
    let workspace = CapturedWorkspaceSource::new(
        workspace_root,
        vec![
            CapturedWorkspaceDocument::new(
                "pale-workspace.toml".into(),
                include_bytes!("../fixtures/cross-leaf.v1/one-shot/workspace/pale-workspace.toml")
                    .to_vec(),
            )
            .expect("shared workspace manifest is bounded"),
            CapturedWorkspaceDocument::new(
                "composition.toml".into(),
                include_bytes!("../fixtures/cross-leaf.v1/one-shot/workspace/composition.toml")
                    .to_vec(),
            )
            .expect("shared composition source is bounded"),
            CapturedWorkspaceDocument::new(
                "workspace-resolution.lock.toml".into(),
                include_bytes!(
                    "../fixtures/cross-leaf.v1/one-shot/workspace/workspace-resolution.lock.toml"
                )
                .to_vec(),
            )
            .expect("shared resolution lock is bounded"),
            CapturedWorkspaceDocument::new(
                "workspace-universe.lock.toml".into(),
                include_bytes!(
                    "../fixtures/cross-leaf.v1/one-shot/workspace/workspace-universe.lock.toml"
                )
                .to_vec(),
            )
            .expect("shared universe lock is bounded"),
        ],
    )
    .expect("shared workspace fixture is complete");
    compile_captured_portable_inputs(
        CapturedOneShotCompilationInputs::new(
            "demo".into(),
            "default".into(),
            "machine".into(),
            private,
            packages,
        ),
        compile_portable_workspace(workspace).expect("shared workspace fixture is admitted"),
    )
    .expect("shared captured one-shot fixture compiles")
}
