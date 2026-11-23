//! Shared native workspace fixture; no duplicate producer or runtime policy.
use super::{LocalOneShotCompilationRequest, LocalWorkspaceLocator};
use std::{fs, path::Path};

macro_rules! captured_fixture {
    ($member:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cross-leaf.v1/one-shot/",
            $member
        ))
    };
}

pub(super) fn write(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture member has a parent"))
        .expect("fixture parent is created");
    fs::write(path, bytes).expect("fixture member is written");
}

pub(super) fn local_workspace() -> (tempfile::TempDir, String) {
    let temporary = tempfile::tempdir().expect("temporary workspace is created");
    let root = temporary
        .path()
        .canonicalize()
        .expect("temporary workspace path is canonical");
    let distribution = root.join("distribution");
    let checkout = root.join("checkout");
    let authority = root.join("authority");
    let lineage = root.join("lineage");
    fs::create_dir_all(&checkout).expect("checkout root is created");
    fs::create_dir_all(&authority).expect("authority root is created");
    fs::create_dir_all(&lineage).expect("lineage root is created");

    for member in [
        "workspace/composition.toml",
        "workspace/pale-workspace.toml",
        "workspace/workspace-resolution.lock.toml",
        "workspace/workspace-universe.lock.toml",
    ] {
        write(
            &root,
            member.strip_prefix("workspace/").expect("workspace member"),
            match member {
                "workspace/composition.toml" => captured_fixture!("workspace/composition.toml"),
                "workspace/pale-workspace.toml" => {
                    captured_fixture!("workspace/pale-workspace.toml")
                }
                "workspace/workspace-resolution.lock.toml" => {
                    captured_fixture!("workspace/workspace-resolution.lock.toml")
                }
                "workspace/workspace-universe.lock.toml" => {
                    captured_fixture!("workspace/workspace-universe.lock.toml")
                }
                _ => unreachable!("closed fixture member set"),
            },
        );
    }
    write(
        &root,
        "capabilities.toml",
        captured_fixture!("private/capabilities.toml"),
    );
    write(
        &root,
        "distribution-lock.toml",
        captured_fixture!("private/distribution-lock.toml"),
    );
    let machine = format!(
        "[machine]\nid = \"machine\"\nschema_version = 1\ndistribution_root = \"{}\"\n\n[[repository_checkouts]]\nrepository_id = \"engine\"\ncheckout_root = \"{}\"\n\n[authority_roots]\nauthority = \"{}\"\n\n[materialization_lineage]\nroot = \"{}\"\nmax_route_records_per_collection = 4096\n",
        distribution.display(),
        checkout.display(),
        authority.display(),
        lineage.display(),
    );
    write(&root, "machines/machine.toml", machine.as_bytes());
    write(
        &distribution.join("packages"),
        "engine/package.toml",
        captured_fixture!("packages/engine/package.toml"),
    );
    write(
        &distribution.join("packages"),
        "engine/roles.toml",
        captured_fixture!("packages/engine/roles.toml"),
    );
    write(
        &distribution.join("packages"),
        "engine/role.md",
        captured_fixture!("packages/engine/role.md"),
    );

    let manifest = root
        .join("pale-workspace.toml")
        .to_str()
        .expect("temporary fixture path is UTF-8")
        .to_owned();
    (temporary, manifest)
}

pub(super) fn request(manifest: String) -> LocalOneShotCompilationRequest {
    LocalOneShotCompilationRequest::new(
        LocalWorkspaceLocator::new(manifest),
        "demo".to_owned(),
        "default".to_owned(),
        "machine".to_owned(),
    )
}

pub(super) fn source_workspace() -> (tempfile::TempDir, String) {
    let (temporary, manifest) = local_workspace();
    let root = temporary.path().canonicalize().unwrap();
    let path = root.join("pale-workspace.toml");
    let mut workspace: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    workspace["workspace"]["desired_topology"]["execution_paths"] = toml::Value::Array(Vec::new());
    workspace["workspace"]["desired_topology"]["execution_workspaces"][0]["execution_path_ids"] =
        toml::Value::Array(Vec::new());
    fs::write(path, toml::to_string(&workspace).unwrap()).unwrap();
    let path = root.join("composition.toml");
    let mut composition: pale_core::records::WorkspaceCompositionSourceV1 =
        toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    composition.declarations = pale_core::records::ExperienceCompositionDeclarationsV1::empty();
    fs::write(path, toml::to_string(&composition).unwrap()).unwrap();
    (temporary, manifest)
}
