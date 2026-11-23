//! Shared canonical API and wire fixtures for CLI unit boundaries.

use pale_sdk::{
    CompilationDocument, EffectiveProfileCompilationRequest, OneShotBaselineEntry,
    OneShotBaselineObservation, OneShotCompilationRequest, OneShotObservationTarget,
    OneShotPrivateInputs, PortableWorkspaceDocument, PortableWorkspaceRequest,
};

#[cfg(feature = "native-local")]
#[path = "activation_workspace.rs"]
mod activation_workspace;
#[cfg(feature = "native-local")]
#[path = "distribution.rs"]
mod synthetic_distribution;

macro_rules! workspace_fixture {
    ($member:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cross-leaf.v1/portable/",
            $member
        ))
    };
}

macro_rules! captured_fixture {
    ($member:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cross-leaf.v1/one-shot/",
            $member
        ))
    };
}

fn compilation_document(label: &str, source: &str) -> CompilationDocument {
    CompilationDocument::new(label.to_owned(), source.as_bytes().to_vec())
}

/// Build the proof-free empty workspace fixture for an API request.
pub(super) fn empty_workspace_request(comment: Option<&str>) -> PortableWorkspaceRequest {
    let mut root = workspace_fixture!("pale-workspace.toml").to_string();
    if let Some(comment) = comment {
        root.push_str(&format!("\n# {comment}\n"));
    }
    PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_string(),
        vec![
            PortableWorkspaceDocument::new("pale-workspace.toml".to_string(), root.into_bytes()),
            PortableWorkspaceDocument::new(
                "resolution.lock.toml".to_string(),
                workspace_fixture!("resolution.lock.toml")
                    .as_bytes()
                    .to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "universe.lock.toml".to_string(),
                workspace_fixture!("universe.lock.toml").as_bytes().to_vec(),
            ),
        ],
    )
}

/// Build one captured one-shot request from the shared application fixture.
pub(super) fn one_shot_request() -> OneShotCompilationRequest {
    let workspace = captured_workspace("mutate");
    let (private, packages) = captured_profile_sources();
    OneShotCompilationRequest::new(
        "demo".to_string(),
        "default".to_string(),
        "machine".to_string(),
        private,
        packages,
        workspace,
    )
}

/// Build role-separated inputs for the effective-profile compiler seam.
pub(super) fn effective_profile_request(
) -> (PortableWorkspaceRequest, EffectiveProfileCompilationRequest) {
    let (private, packages) = captured_profile_sources();
    (
        captured_workspace("mutate"),
        EffectiveProfileCompilationRequest::new(
            "demo".to_string(),
            "default".to_string(),
            "machine".to_string(),
            private,
            packages,
        ),
    )
}

/// Build the admitted portable workspace source role once per request.
pub(super) fn captured_workspace(access: &str) -> PortableWorkspaceRequest {
    PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_string(),
        vec![
            PortableWorkspaceDocument::new(
                "pale-workspace.toml".to_string(),
                captured_fixture!("workspace/pale-workspace.toml")
                    .replace("access = \"mutate\"", &format!("access = \"{access}\""))
                    .into_bytes(),
            ),
            PortableWorkspaceDocument::new(
                "composition.toml".to_string(),
                captured_fixture!("workspace/composition.toml")
                    .as_bytes()
                    .to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "workspace-resolution.lock.toml".to_string(),
                captured_fixture!("workspace/workspace-resolution.lock.toml")
                    .as_bytes()
                    .to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "workspace-universe.lock.toml".to_string(),
                captured_fixture!("workspace/workspace-universe.lock.toml")
                    .as_bytes()
                    .to_vec(),
            ),
        ],
    )
}

/// Build the private and Package source roles without flattening their custody.
pub(super) fn captured_profile_sources() -> (OneShotPrivateInputs, Vec<CompilationDocument>) {
    let private = OneShotPrivateInputs::new(
        compilation_document(
            "capabilities.toml",
            captured_fixture!("private/capabilities.toml"),
        ),
        compilation_document(
            "machines/machine.toml",
            captured_fixture!("private/machine-binding.toml"),
        ),
        compilation_document(
            "distribution-lock.toml",
            captured_fixture!("private/distribution-lock.toml"),
        ),
        Vec::new(),
    );
    let packages = vec![
        CompilationDocument::new(
            "engine/package.toml".to_string(),
            captured_fixture!("packages/engine/package.toml")
                .as_bytes()
                .to_vec(),
        ),
        CompilationDocument::new(
            "engine/roles.toml".to_string(),
            captured_fixture!("packages/engine/roles.toml")
                .as_bytes()
                .to_vec(),
        ),
        CompilationDocument::new(
            "engine/role.md".to_string(),
            captured_fixture!("packages/engine/role.md")
                .as_bytes()
                .to_vec(),
        ),
    ];
    (private, packages)
}

/// Claim every required planning target as absent through the API carrier.
pub(super) fn absent_baseline(targets: &[OneShotObservationTarget]) -> Vec<OneShotBaselineEntry> {
    targets
        .iter()
        .map(|target| {
            OneShotBaselineEntry::new(
                target.relative_path().to_string(),
                OneShotBaselineObservation::Absent,
            )
        })
        .collect()
}

/// Build one isolated local workspace bound to the current test process.
#[cfg(feature = "native-local")]
pub(super) fn local_executable_binding_request(
) -> (tempfile::TempDir, pale_sdk::LocalOneShotCompilationRequest) {
    use sha2::{Digest as _, Sha256};

    let temporary = tempfile::tempdir().expect("create local executable-binding fixture");
    let target = temporary.path().join("target");
    let authority = temporary.path().join("authority");
    let state = temporary.path().join("state");
    let lineage = temporary.path().join("lineage");
    for directory in [&target, &authority, &state, &lineage] {
        std::fs::create_dir_all(directory)
            .expect("create local executable-binding fixture directory");
    }
    let executable = std::env::current_exe()
        .expect("test executable exists")
        .canonicalize()
        .expect("test executable is canonical");
    let digest = hex::encode(Sha256::digest(
        std::fs::read(&executable).expect("test executable is readable"),
    ));
    let manifest = activation_workspace::lay_out(
        &temporary.path().join("workspace"),
        &activation_workspace::MachineInputs {
            target: &target,
            auth: &authority,
            exec: &executable,
            digest: &digest,
            state_root: &state,
            lineage_root: &lineage,
        },
    );
    let request = pale_sdk::LocalOneShotCompilationRequest::new(
        pale_sdk::LocalWorkspaceLocator::new(
            manifest
                .to_str()
                .expect("fixture manifest is UTF-8")
                .to_owned(),
        ),
        "act".to_owned(),
        "default".to_owned(),
        "m".to_owned(),
    );
    (temporary, request)
}
