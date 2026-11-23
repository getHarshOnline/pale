//! Rust-source inventory shared by repository architecture regressions.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(crate) fn collect_rust_sources(root: &Path, output: &mut BTreeSet<PathBuf>) {
    for entry in
        rtepl_repository::source_entries(root, super::repository::EXCLUDED_SOURCE_DIRECTORIES)
            .expect("read Rust source tree")
    {
        if entry.file_type.is_file() && entry.path.extension().is_some_and(|value| value == "rs") {
            assert!(output.insert(entry.path), "duplicate Rust source path");
        }
    }
}
