//! Source text and compiler compatibility of the current Package tree.

use std::fs;
use std::path::{Path, PathBuf};

use pale_application::packages::inspect_distribution;
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .to_path_buf()
}

fn collect_sources(root: &Path, directory: &Path, sources: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .expect("Package directory is readable")
        .map(|entry| entry.expect("Package directory entry is readable").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_sources(root, &path, sources);
        } else if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("md" | "toml")
        ) {
            sources.push(
                path.strip_prefix(root)
                    .expect("Package member is confined")
                    .to_path_buf(),
            );
        }
    }
}

#[test]
fn current_package_sources_are_text_and_load_in_the_compiler() {
    let root = repository_root();
    let packages = root.join("packages");
    let mut sources = Vec::new();
    collect_sources(&packages, &packages, &mut sources);
    for relative in sources {
        let path = packages.join(&relative);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", relative.display()));
        assert!(
            text.is_empty() || text.ends_with('\n'),
            "{} needs a final newline",
            relative.display()
        );
    }

    let info = inspect_distribution(&pale_adapter_fs::LocalSourceFilesystem, &root)
        .expect("current Package frontier is admissible to the Engine");
    assert_eq!(info.packages().len(), 14);
    assert_eq!(
        info.packages()
            .iter()
            .map(|release| release.id.as_str())
            .collect::<Vec<_>>(),
        [
            "pale.agent-work",
            "pale.architecture",
            "pale.assurance",
            "pale.automation",
            "pale.business",
            "pale.design",
            "pale.engineering",
            "pale.evaluation",
            "pale.finance",
            "pale.governance",
            "pale.model-work",
            "pale.product",
            "pale.publication",
            "pale.research",
        ]
    );
}
