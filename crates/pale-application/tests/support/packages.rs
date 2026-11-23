//! Synthetic initial-v1 Package fixtures shared by application integration tests.

use std::path::{Path, PathBuf};

pub(crate) fn write_manifest(
    root: &Path,
    directory: &str,
    id: &str,
    declaration: &str,
    body: &str,
    resource: &str,
) -> PathBuf {
    let package = root.join(format!("packages/{directory}"));
    std::fs::create_dir_all(&package).expect("create synthetic Package directory");
    std::fs::write(
        package.join("package.toml"),
        format!(
            "[package]\nid = '{id}'\nversion = '1.0.0'\nschema_version = 1\ntitle = 'Synthetic'\ndescription = 'Synthetic Package'\ncompatible_compilers = ['pale-surface@1']\nsources = [{{ path = '{declaration}', role = 'declaration' }}, {{ path = '{body}', role = 'body' }}]\nresources = [{resource}]\n"
        ),
    )
    .expect("write synthetic Package manifest");
    package
}

/// Write the manifest for one minimal content-bearing policy-family Package.
pub(crate) fn write_policy_manifest(root: &Path, directory: &str, id: &str) -> PathBuf {
    write_manifest(
        root,
        directory,
        id,
        "family.toml",
        "body.md",
        "{ id = 'policy', kind = 'policy_family', definition_source = 'family.toml', exposure = 'exported' }",
    )
}

/// Write one minimal content-bearing policy-family Package.
pub(crate) fn write_policy_package(root: &Path, directory: &str, id: &str, body: &str) {
    let package = write_policy_manifest(root, directory, id);
    std::fs::write(
        package.join("family.toml"),
        "id = 'policy'\ntitle = 'Policy'\npurpose = 'Synthetic proof'\nbody = 'body.md'\n",
    )
    .expect("write synthetic policy declaration");
    std::fs::write(package.join("body.md"), body).expect("write synthetic Package body");
}
