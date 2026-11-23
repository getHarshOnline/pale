//! Specialist-role Package fixture used only by end-to-end compilation tests.

use std::path::{Path, PathBuf};

use super::package_fixtures::write_manifest;

/// Write one minimal exported specialist-role Package and return its body path.
pub(crate) fn write_role_package(root: &Path, directory: &str, id: &str, body: &str) -> PathBuf {
    let package = write_manifest(
        root,
        directory,
        id,
        "roles.toml",
        "role.md",
        "{ id = 'engine', kind = 'role', definition_source = 'roles.toml', exposure = 'exported' }",
    );
    std::fs::write(
        package.join("roles.toml"),
        "[roles]\nschema_version = 1\n[[roles.role]]\nid = 'engine'\ndescription = 'Synthetic repository role'\ndoctrine = 'role.md'\n",
    )
    .expect("write synthetic role declaration");
    let body_path = package.join("role.md");
    std::fs::write(&body_path, body).expect("write synthetic role body");
    body_path
}
