//! Repository-layout discovery shared by repository quality regressions.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Return the composition root that owns repository-wide quality policy.
pub(crate) fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repository-quality is nested under tools/")
        .to_path_buf()
}

pub(crate) const EXCLUDED_SOURCE_DIRECTORIES: &[&str] = &[
    ".git",
    "dist",
    "node_modules",
    "target",
    ".venv",
    "__pycache__",
];

/// Exclude local metadata, build output and installed dependencies from source custody.
///
/// Authored generated sources are deliberately not excluded by this predicate.
#[allow(dead_code, reason = "shared test support is compiled per test target")]
pub(crate) fn is_excluded_source_directory(name: &str) -> bool {
    EXCLUDED_SOURCE_DIRECTORIES.contains(&name)
}

/// Discover every Rust package in the selected source homes.
///
/// The Engine, adapter, interface, SDK, and integration collections contain
/// component source homes in this repository. A directory without a `Cargo.toml` is not silently
/// promoted to a Rust package merely because it contains a Rust-looking file.
#[allow(
    dead_code,
    reason = "shared integration-test support is compiled once per test target"
)]
pub(crate) fn rust_crate_roots(root: &Path) -> Vec<PathBuf> {
    let mut crates = BTreeSet::new();
    for collection in ["crates", "adapters", "interfaces", "sdk", "integrations"] {
        let collection_root = root.join(collection);
        if !collection_root.exists() {
            continue;
        }
        for entry in fs::read_dir(&collection_root)
            .unwrap_or_else(|error| panic!("read {}: {error}", collection_root.display()))
        {
            let entry = entry.expect("read source-home collection entry");
            if !entry
                .file_type()
                .expect("inspect source-home collection entry")
                .is_dir()
            {
                continue;
            }
            let candidate = entry.path();
            if candidate.join("Cargo.toml").is_file() {
                assert!(
                    crates.insert(candidate.clone()),
                    "duplicate Rust crate root: {}",
                    candidate.display()
                );
            }
        }
    }

    assert!(!crates.is_empty(), "source-home layout has no Rust crates");
    crates.into_iter().collect()
}
