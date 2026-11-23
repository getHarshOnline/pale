//! Keep licence and notice ownership without requiring repeated Rustdoc prose.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "support/repository.rs"]
mod repository;
#[path = "support/rust_sources.rs"]
mod rust_sources;

use repository::{repository_root, rust_crate_roots};
use rust_sources::collect_rust_sources;

const ATTRIBUTION_FILE: &str = "RUSTDOC-ATTRIBUTION.md";
const PROJECT_URL: &str = "https://github.com/getHarshOnline/pale";

fn workspace_package_value<'a>(
    repository_manifest: &'a toml::Table,
    name: &str,
) -> Option<&'a toml::Value> {
    repository_manifest
        .get("workspace")
        .and_then(toml::Value::as_table)
        .and_then(|workspace| workspace.get("package"))
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get(name))
}

fn package_string(manifest: &toml::Table, repository_manifest: &toml::Table, name: &str) -> String {
    let value = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get(name))
        .expect("package metadata field exists");
    if let Some(value) = value.as_str() {
        return value.to_owned();
    }
    let inherited = value
        .as_table()
        .and_then(|value| value.get("workspace"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false);
    assert!(
        inherited,
        "package `{name}` metadata must be a string or workspace-inherited"
    );
    workspace_package_value(repository_manifest, name)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("workspace.package.{name} must be a string"))
        .to_owned()
}

fn assert_package_license(root: &Path, crate_root: &Path, license: &str) {
    let canonical = match license {
        "AGPL-3.0-or-later" => root.join("LICENSE"),
        "Apache-2.0" => root.join("contracts/LICENSE"),
        other => panic!(
            "{} selects unsupported source license `{other}`",
            crate_root.display()
        ),
    };
    let expected = fs::read(canonical).expect("read canonical license");
    let local = crate_root.join("LICENSE");
    if local.is_file() {
        assert_eq!(fs::read(local).expect("read component license"), expected);
    }
}

fn collect_layout_rust(root: &Path) -> BTreeSet<PathBuf> {
    let mut rust = BTreeSet::new();
    for source_root in [
        "crates",
        "adapters",
        "interfaces",
        "sdk",
        "integrations",
        "tools",
    ] {
        let source_root = root.join(source_root);
        if source_root.exists() {
            collect_rust_sources(&source_root, &mut rust);
        }
    }
    rust
}

#[test]
fn manifests_name_licenses_without_repeated_rustdoc_credits() {
    let root = repository_root();
    let repository_manifest = fs::read_to_string(root.join("Cargo.toml"))
        .expect("read repository manifest")
        .parse::<toml::Table>()
        .expect("parse repository manifest");
    for crate_root in rust_crate_roots(&root) {
        let manifest = fs::read_to_string(crate_root.join("Cargo.toml"))
            .expect("read crate manifest")
            .parse::<toml::Table>()
            .expect("parse crate manifest");
        let license = package_string(&manifest, &repository_manifest, "license");
        assert_package_license(&root, &crate_root, &license);
        assert_eq!(
            package_string(&manifest, &repository_manifest, "repository"),
            PROJECT_URL
        );
    }
    for path in collect_layout_rust(&root) {
        if path == root.join("tools/repository-quality/tests/rustdoc_attribution.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read Rust source");
        assert!(
            !source.contains(ATTRIBUTION_FILE),
            "{} includes a duplicated credit footer",
            path.display()
        );
    }
}
