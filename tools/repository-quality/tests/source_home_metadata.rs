//! Distribution metadata for independently packaged components in one repository.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "support/repository.rs"]
mod repository;

use repository::repository_root;

const OWNER: &str = "RawThoughts Enterprises Private Limited";
const AUTHOR: &str = "Harsh Joshi";
const INITIAL_VERSION: &str = "0.0.1";
const REPOSITORY_URL: &str = "https://github.com/getHarshOnline/pale";
const SELECTED_INTEGRATION_HOMES: [&str; 3] = ["agents", "gazette", "vscode"];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Ecosystem {
    Cargo,
    Go,
    Node,
    Python,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProjectLicense {
    Agpl,
    Apache,
}

impl ProjectLicense {
    fn spdx(self) -> &'static str {
        match self {
            Self::Agpl => "AGPL-3.0-or-later",
            Self::Apache => "Apache-2.0",
        }
    }

    fn from_source_bytes(repository: &Path, source: &Path) -> Self {
        let agpl = read_bytes(&repository.join("LICENSE"));
        let apache = read_bytes(&repository.join("contracts/LICENSE"));
        let source_license = if source.join("LICENSE").is_file() {
            read_bytes(&source.join("LICENSE"))
        } else {
            agpl.clone()
        };

        assert_ne!(agpl, apache, "license authorities must have distinct bytes");
        if source_license == agpl {
            Self::Agpl
        } else if source_license == apache {
            Self::Apache
        } else {
            panic!(
                "{} does not carry either canonical project license",
                source.display()
            );
        }
    }
}

#[derive(Debug)]
struct DistributionMetadata {
    version: String,
    authors: Vec<String>,
    license: String,
}

#[derive(Debug)]
struct NativeManifest {
    ecosystem: Ecosystem,
    package: String,
    repository: String,
    distribution: Option<DistributionMetadata>,
}

#[derive(Debug)]
struct SourceHome {
    root: PathBuf,
    relative: PathBuf,
    license: ProjectLicense,
    manifests: Vec<NativeManifest>,
}

impl SourceHome {
    fn discover(repository: &Path, root: PathBuf) -> Self {
        let relative = root
            .strip_prefix(repository)
            .unwrap_or_else(|_| panic!("{} is outside the repository", root.display()))
            .to_path_buf();
        let mut manifests = Vec::new();
        for (marker, parser) in [
            ("Cargo.toml", cargo_manifest as fn(&Path) -> NativeManifest),
            ("go.mod", go_manifest),
            ("package.json", node_manifest),
            ("pyproject.toml", python_manifest),
        ] {
            let path = root.join(marker);
            if path.is_file() {
                manifests.push(parser(&path));
            }
        }
        if relative == Path::new("sdk/js") {
            manifests.push(node_manifest(&root.join("contracts/package.json")));
        }
        assert!(
            !manifests.is_empty(),
            "{} has no native package manifest",
            relative.display()
        );

        Self {
            license: ProjectLicense::from_source_bytes(repository, &root),
            root,
            relative,
            manifests,
        }
    }

    fn canonical_repository(&self) -> &str {
        let repository = &self.manifests[0].repository;
        assert!(
            self.manifests
                .iter()
                .all(|manifest| manifest.repository == *repository),
            "{} native manifests disagree on repository identity",
            self.relative.display()
        );
        repository
    }

    fn assert_repository_identity(&self) {
        let repository = self.canonical_repository();
        assert_eq!(
            repository,
            REPOSITORY_URL,
            "{} must identify the shared Pale repository",
            self.relative.display()
        );
    }

    fn ecosystems(&self) -> BTreeSet<Ecosystem> {
        self.manifests
            .iter()
            .map(|manifest| manifest.ecosystem)
            .collect()
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn read_bytes(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn table(path: &Path) -> toml::Table {
    read(path)
        .parse()
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn required_string<'a>(table: &'a toml::Table, field: &str, path: &Path) -> &'a str {
    table
        .get(field)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("{} has no string {field}", path.display()))
}

fn required_authors(table: &toml::Table, path: &Path) -> Vec<String> {
    table
        .get("authors")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("{} has no authors array", path.display()))
        .iter()
        .map(|author| {
            author
                .as_str()
                .unwrap_or_else(|| panic!("{} contains a non-string author", path.display()))
                .to_owned()
        })
        .collect()
}

fn cargo_manifest(path: &Path) -> NativeManifest {
    let document = table(path);
    let package = document
        .get("package")
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("{} has no [package] table", path.display()));

    NativeManifest {
        ecosystem: Ecosystem::Cargo,
        package: required_string(package, "name", path).to_owned(),
        repository: required_string(package, "repository", path).to_owned(),
        distribution: Some(DistributionMetadata {
            version: required_string(package, "version", path).to_owned(),
            authors: required_authors(package, path),
            license: required_string(package, "license", path).to_owned(),
        }),
    }
}

fn go_manifest(path: &Path) -> NativeManifest {
    let document = read(path);
    let module = document
        .lines()
        .find_map(|line| line.strip_prefix("module "))
        .unwrap_or_else(|| panic!("{} has no module directive", path.display()));
    let root = repository_root();
    let directory = path.parent().expect("Go manifest has a parent");
    let relative = directory
        .strip_prefix(&root)
        .expect("Go module belongs to the Pale repository")
        .to_string_lossy()
        .replace('\\', "/");
    assert_eq!(
        module,
        format!("github.com/getHarshOnline/pale/{relative}"),
        "{} module path must name its source directory",
        path.display()
    );
    let repository = REPOSITORY_URL.to_owned();

    NativeManifest {
        ecosystem: Ecosystem::Go,
        package: module.to_owned(),
        repository,
        distribution: None,
    }
}

fn node_manifest(path: &Path) -> NativeManifest {
    let package: serde_json::Value = serde_json::from_str(&read(path))
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    let required = |field: &str| {
        package[field]
            .as_str()
            .unwrap_or_else(|| panic!("{} has no string {field}", path.display()))
            .to_owned()
    };
    let root = repository_root();
    let relative = path
        .parent()
        .expect("Node manifest has a parent")
        .strip_prefix(&root)
        .expect("Node package belongs to the Pale repository")
        .to_string_lossy()
        .replace('\\', "/");
    assert_eq!(
        package["repository"]["directory"].as_str(),
        Some(relative.as_str()),
        "{} must identify its package directory",
        path.display()
    );
    let repository = match &package["repository"] {
        serde_json::Value::String(repository) => repository.as_str(),
        serde_json::Value::Object(repository) => repository
            .get("url")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("{} repository object has no URL", path.display())),
        _ => panic!("{} has no repository identity", path.display()),
    };
    let repository = repository.strip_prefix("git+").unwrap_or(repository);
    let repository = repository
        .strip_suffix(".git")
        .unwrap_or(repository)
        .to_owned();

    NativeManifest {
        ecosystem: Ecosystem::Node,
        package: required("name"),
        repository,
        distribution: Some(DistributionMetadata {
            version: required("version"),
            authors: vec![required("author")],
            license: required("license"),
        }),
    }
}

fn python_manifest(path: &Path) -> NativeManifest {
    let document = table(path);
    let project = document
        .get("project")
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("{} has no [project] table", path.display()));
    let authors = project
        .get("authors")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("{} has no project authors", path.display()))
        .iter()
        .map(|author| {
            author
                .as_table()
                .and_then(|author| author.get("name"))
                .and_then(toml::Value::as_str)
                .unwrap_or_else(|| panic!("{} has an invalid project author", path.display()))
                .to_owned()
        })
        .collect();
    let repository = project
        .get("urls")
        .and_then(toml::Value::as_table)
        .and_then(|urls| urls.get("Repository"))
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("{} has no project Repository URL", path.display()));

    NativeManifest {
        ecosystem: Ecosystem::Python,
        package: required_string(project, "name", path).to_owned(),
        repository: repository.to_owned(),
        distribution: Some(DistributionMetadata {
            version: required_string(project, "version", path).to_owned(),
            authors,
            license: required_string(project, "license", path).to_owned(),
        }),
    }
}

fn has_native_manifest(path: &Path) -> bool {
    ["Cargo.toml", "go.mod", "package.json", "pyproject.toml"]
        .iter()
        .any(|marker| path.join(marker).is_file())
}

fn discover_source_homes(repository: &Path) -> Vec<SourceHome> {
    let mut roots = BTreeSet::new();
    for collection in ["interfaces", "sdk", "integrations"] {
        let collection_root = repository.join(collection);
        for entry in fs::read_dir(&collection_root)
            .unwrap_or_else(|error| panic!("read {}: {error}", collection_root.display()))
        {
            let entry = entry.expect("read standalone source-home entry");
            if entry
                .file_type()
                .expect("inspect standalone source-home entry")
                .is_dir()
                && has_native_manifest(&entry.path())
            {
                assert!(
                    roots.insert(entry.path()),
                    "duplicate standalone source home"
                );
            }
        }
    }

    assert!(!roots.is_empty(), "no standalone source homes discovered");
    roots
        .into_iter()
        .map(|root| SourceHome::discover(repository, root))
        .collect()
}

fn assert_manifest_metadata(home: &SourceHome) {
    let expected_license = home.license.spdx();
    for manifest in &home.manifests {
        assert!(
            !manifest.package.trim().is_empty(),
            "{} has an empty {:?} package identity",
            home.relative.display(),
            manifest.ecosystem
        );
        if let Some(distribution) = &manifest.distribution {
            assert_eq!(
                distribution.version,
                INITIAL_VERSION,
                "{} {:?} package has the wrong initial version",
                home.relative.display(),
                manifest.ecosystem
            );
            assert_eq!(
                distribution.authors.as_slice(),
                [AUTHOR],
                "{} {:?} package has the wrong authorship",
                home.relative.display(),
                manifest.ecosystem
            );
            assert_eq!(
                distribution.license,
                expected_license,
                "{} {:?} package disagrees with its license bytes",
                home.relative.display(),
                manifest.ecosystem
            );
        }
    }

    home.assert_repository_identity();
}

#[test]
fn discovered_source_homes_own_consistent_distribution_metadata() {
    let repository = repository_root();
    let rust_crates = repository::rust_crate_roots(&repository);
    let homes = discover_source_homes(&repository);
    let mut source_paths = BTreeSet::new();
    let mut native_packages = BTreeSet::new();

    for home in homes {
        assert_manifest_metadata(&home);
        assert!(
            home.root.join("README.md").is_file(),
            "component README is absent"
        );
        assert!(
            source_paths.insert(home.relative.clone()),
            "duplicate component source path: {}",
            home.relative.display()
        );
        for manifest in &home.manifests {
            assert!(
                native_packages.insert((manifest.ecosystem, manifest.package.clone())),
                "duplicate {:?} package identity: {}",
                manifest.ecosystem,
                manifest.package
            );
        }
        if home.ecosystems().contains(&Ecosystem::Cargo) {
            assert!(
                rust_crates.contains(&home.root),
                "{} is absent from Rust source-home discovery",
                home.relative.display()
            );
        }
    }
}

#[test]
fn package_corpus_owns_local_distribution_metadata() {
    let repository = repository_root();
    let root = repository.join("packages");
    let relative = Path::new("packages");

    assert_eq!(
        read_bytes(&root.join("LICENSE")),
        read_bytes(&repository.join("LICENSE")),
        "{} does not carry the canonical Package-corpus license",
        relative.display()
    );
    assert!(
        root.join("README.md").is_file(),
        "{} owns no README",
        relative.display()
    );

    let citation = read(&repository.join("CITATION.cff"));
    for required in [
        "family-names: \"Joshi\"",
        "given-names: \"Harsh\"",
        OWNER,
        REPOSITORY_URL,
    ] {
        assert!(
            citation.contains(required),
            "root citation omits {required}"
        );
    }
    assert!(repository.join("CONTRIBUTING.md").is_file());
    assert!(repository.join("CLA.md").is_file());
    assert!(root.join("NOTICE").is_file());
}

#[test]
fn selected_integration_roster_owns_truthful_component_metadata() {
    let repository = repository_root();
    let integrations = repository.join("integrations");
    let observed = fs::read_dir(&integrations)
        .unwrap_or_else(|error| panic!("read {}: {error}", integrations.display()))
        .map(|entry| entry.expect("read integration source-home entry"))
        .filter(|entry| {
            entry
                .file_type()
                .expect("inspect integration source-home entry")
                .is_dir()
        })
        .map(|entry| {
            entry
                .file_name()
                .into_string()
                .expect("integration source-home name is valid UTF-8")
        })
        .collect::<BTreeSet<_>>();
    let expected = SELECTED_INTEGRATION_HOMES
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        observed, expected,
        "integration source-home roster drifted from the selected initial boundary"
    );

    for name in SELECTED_INTEGRATION_HOMES {
        let root = integrations.join(name);
        let relative = PathBuf::from("integrations").join(name);
        assert_eq!(
            ProjectLicense::from_source_bytes(&repository, &root),
            ProjectLicense::Agpl,
            "{} must use the repository's integration license",
            relative.display()
        );
        assert!(root.join("README.md").is_file());
    }
}
