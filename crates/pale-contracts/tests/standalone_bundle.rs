//! Parent-independent, local-only source-capsule proof for pale-contracts.
//!
//! The test copies an explicit, symlink-rejecting inventory into a fresh
//! three-crate workspace, proves Cargo resolves only those local crates, and
//! runs the capsule's identity test with dependency resolution locked and
//! network access disabled. This is deliberately not crates.io/docs.rs,
//! publication, installation, release, or compatibility evidence.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read as _, Write as _};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

const FIXTURE_FILES: &[&str] = &[
    "Cargo.lock",
    "Cargo.toml",
    "crates/pale-contract-codegen/Cargo.toml",
    "crates/pale-contracts/Cargo.toml",
    "crates/pale-contracts/tests/capsule_identity.rs",
    "crates/pale-core/Cargo.toml",
    "source-capsule.lock.json",
    "source-inventory.v1.txt",
];

const CANONICAL_TREES: &[&str] = &[
    "crates/pale-contract-codegen/src",
    "crates/pale-contracts/build",
    "contracts/bundle/agent-surface.v1",
    "crates/pale-contracts/src",
    "crates/pale-core/docs/math",
    "crates/pale-core/src",
];

const CANONICAL_FILES: &[&str] = &["LICENSE", "NOTICE", "crates/pale-contracts/build.rs"];

const EXPECTED_PACKAGES: &[&str] = &["pale-contract-codegen", "pale-contracts", "pale-core"];

#[test]
fn canonical_contract_sources_build_without_the_repository_parent() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository_root = crate_dir
        .parent()
        .and_then(Path::parent)
        .expect("pale-contracts remains directly below the repository's crates directory");
    let fixture_root = crate_dir.join("tests/fixtures/standalone");

    let actual_fixture_files = regular_files_beneath(&fixture_root, Path::new(""));
    let expected_fixture_files = paths(FIXTURE_FILES.iter().copied());
    assert_eq!(
        actual_fixture_files, expected_fixture_files,
        "the retained capsule fixture must be an explicit closed set"
    );

    let inventory_bytes = read_regular_file(&fixture_root, Path::new("source-inventory.v1.txt"));
    let inventory = parse_inventory(&inventory_bytes);
    let canonical_inputs = canonical_input_closure(repository_root);
    assert_eq!(
        inventory, canonical_inputs,
        "the explicit inventory must name every and only canonical capsule input"
    );

    let temporary = tempfile::tempdir().expect("create an ephemeral capsule root");
    let capsule_root = temporary.path();
    let canonical_repository_root = repository_root
        .canonicalize()
        .expect("canonicalize the repository root");
    let canonical_capsule_root = capsule_root
        .canonicalize()
        .expect("canonicalize the capsule root");
    assert!(
        !canonical_capsule_root.starts_with(&canonical_repository_root),
        "the proof capsule must not be nested beneath the repository"
    );

    for relative in &expected_fixture_files {
        copy_regular_file(
            &fixture_root,
            capsule_root,
            relative,
            &canonical_repository_root,
        );
    }
    for relative in &inventory {
        copy_regular_file(
            repository_root,
            capsule_root,
            relative,
            &canonical_repository_root,
        );
    }

    let cargo_lock_before = read_regular_file(capsule_root, Path::new("Cargo.lock"));
    let metadata_output = cargo(
        capsule_root,
        &[
            "metadata",
            "--offline",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
        ],
    );
    let metadata_bytes = require_success("read capsule metadata", metadata_output);
    assert_local_three_crate_metadata(&metadata_bytes, capsule_root);

    let test_output = cargo(
        capsule_root,
        &[
            "test",
            "--offline",
            "--locked",
            "--package",
            "pale-contracts",
            "--test",
            "capsule_identity",
        ],
    );
    require_success(
        "build and test the standalone contract capsule",
        test_output,
    );
    assert_eq!(
        read_regular_file(capsule_root, Path::new("Cargo.lock")),
        cargo_lock_before,
        "locked offline validation must not rewrite the capsule dependency lock"
    );
}

fn render_inventory(inputs: &BTreeSet<PathBuf>) -> Vec<u8> {
    assert!(!inputs.is_empty(), "a source capsule must have inputs");
    let mut paths = inputs
        .iter()
        .map(|path| {
            path.iter()
                .map(|component| component.to_str().expect("inventory path is UTF-8"))
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect::<Vec<_>>();
    paths.sort();
    let bytes = format!("{}\n", paths.join("\n")).into_bytes();
    assert_eq!(&parse_inventory(&bytes), inputs);
    bytes
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct CapsuleLock {
    format: String,
    proof_scope: String,
    cargo_lock_sha256: String,
    source_inventory_sha256: String,
    schema_count: u64,
    schema_manifest_sha256: String,
    schema_set_sha256: String,
}

/// Derive an expectation for a future capsule build, not a successful receipt.
fn regenerate_capsule_lock(source: &[u8], cargo_lock: &[u8], inventory: &[u8]) -> Vec<u8> {
    let value =
        pale_core::canon::parse_strict(source).expect("retained capsule lock is strict JSON");
    let mut lock: CapsuleLock =
        serde_json::from_value(value).expect("retained capsule lock has its closed shape");
    assert_eq!(lock.format, "pale-contract-source-capsule-lock.v1");
    assert_eq!(
        lock.proof_scope,
        "local_source_capsule_build_independence_only"
    );
    parse_inventory(inventory);
    let identity =
        pale_contracts::schema_registry_identity().expect("current generated registry identity");
    lock.cargo_lock_sha256 = pale_core::sha256_hex(cargo_lock);
    lock.source_inventory_sha256 = pale_core::sha256_hex(inventory);
    lock.schema_count =
        u64::try_from(pale_contracts::schema_count().expect("current generated schema count"))
            .expect("schema count fits u64");
    lock.schema_manifest_sha256 = identity.schema_manifest_sha256.as_str().to_owned();
    lock.schema_set_sha256 = identity.schema_set_sha256.as_str().to_owned();
    let mut bytes = serde_json::to_vec_pretty(&lock).expect("capsule lock serializes");
    bytes.push(b'\n');
    bytes
}

fn write_capsule_coordinates(
    directory: &Path,
    lock: &[u8],
    inventory: &[u8],
) -> std::io::Result<()> {
    assert!(directory.is_absolute(), "output directory must be absolute");
    fs::create_dir(directory)?;
    for (name, bytes) in [
        ("source-capsule.lock.json", lock),
        ("source-inventory.v1.txt", inventory),
    ] {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(name))?;
        file.write_all(bytes)?;
        file.flush()?;
    }
    Ok(())
}

#[test]
fn capsule_regeneration_binds_inputs_and_preserves_proof_scope() {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/standalone");
    let source = read_regular_file(&fixture_root, Path::new("source-capsule.lock.json"));
    let inventory = read_regular_file(&fixture_root, Path::new("source-inventory.v1.txt"));
    let cargo_lock = read_regular_file(&fixture_root, Path::new("Cargo.lock"));
    let output = regenerate_capsule_lock(&source, &cargo_lock, &inventory);
    assert!(output.ends_with(b"\n"));
    assert_eq!(
        regenerate_capsule_lock(&output, &cargo_lock, &inventory),
        output
    );
    let actual = pale_core::canon::parse_strict(&output).unwrap();
    assert_eq!(
        actual["cargo_lock_sha256"],
        pale_core::sha256_hex(&cargo_lock)
    );
    assert_eq!(
        actual["source_inventory_sha256"],
        pale_core::sha256_hex(&inventory)
    );
    let original = pale_core::canon::parse_strict(&source).unwrap();
    let mut restored = actual.clone();
    for field in [
        "cargo_lock_sha256",
        "source_inventory_sha256",
        "schema_count",
        "schema_manifest_sha256",
        "schema_set_sha256",
    ] {
        restored[field] = original[field].clone();
    }
    assert_eq!(restored, original);
    let changed_lock = regenerate_capsule_lock(&source, b"changed dependency input\n", &inventory);
    let changed_inventory = regenerate_capsule_lock(&source, &cargo_lock, b"changed/input.rs\n");
    for (changed, field) in [
        (changed_lock, "cargo_lock_sha256"),
        (changed_inventory, "source_inventory_sha256"),
    ] {
        let mut changed = pale_core::canon::parse_strict(&changed).unwrap();
        assert_ne!(changed[field], actual[field]);
        changed[field] = actual[field].clone();
        assert_eq!(
            changed, actual,
            "only the corresponding input identity changes"
        );
    }
}

#[test]
fn capsule_inventory_follows_actual_canonical_files() {
    let temporary = tempfile::tempdir().unwrap();
    for tree in CANONICAL_TREES {
        let directory = temporary.path().join(tree);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("existing.rs"), b"fixture input\n").unwrap();
    }
    for file in CANONICAL_FILES {
        let path = temporary.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"fixture input\n").unwrap();
    }
    let before = canonical_input_closure(temporary.path());
    let added = Path::new(CANONICAL_TREES[0]).join("added.rs");
    fs::write(temporary.path().join(&added), b"new fixture input\n").unwrap();
    let after = canonical_input_closure(temporary.path());
    assert_eq!(after.len(), before.len() + 1);
    assert!(after.contains(&added));
    assert_ne!(render_inventory(&before), render_inventory(&after));
    fs::remove_file(temporary.path().join(added)).unwrap();
    assert_eq!(canonical_input_closure(temporary.path()), before);
}

#[test]
fn capsule_regeneration_rejects_scope_drift_and_invalid_inventory() {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/standalone");
    let source = read_regular_file(&fixture_root, Path::new("source-capsule.lock.json"));
    let mut wrong_scope = pale_core::canon::parse_strict(&source).unwrap();
    wrong_scope["proof_scope"] = "publication".into();
    let mut unknown = pale_core::canon::parse_strict(&source).unwrap();
    unknown["unexpected"] = true.into();
    let mut wrong_type = pale_core::canon::parse_strict(&source).unwrap();
    wrong_type["schema_count"] = "131".into();
    for invalid in [wrong_scope, unknown, wrong_type] {
        assert!(std::panic::catch_unwind(|| regenerate_capsule_lock(
            &serde_json::to_vec(&invalid).unwrap(),
            b"lock",
            b"input.rs\n"
        ))
        .is_err());
    }
    for inventory in [b"../escape\n".as_slice(), b"b.rs\na.rs\n", b"a.rs\na.rs\n"] {
        assert!(
            std::panic::catch_unwind(|| regenerate_capsule_lock(&source, b"lock", inventory))
                .is_err()
        );
    }
}

#[test]
fn capsule_coordinate_output_refuses_existing_directory() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path().join("output");
    write_capsule_coordinates(&directory, b"first lock\n", b"first inventory\n").unwrap();
    assert!(
        write_capsule_coordinates(&directory, b"second lock\n", b"second inventory\n").is_err()
    );
    assert_eq!(
        fs::read(directory.join("source-capsule.lock.json")).unwrap(),
        b"first lock\n"
    );
    assert_eq!(
        fs::read(directory.join("source-inventory.v1.txt")).unwrap(),
        b"first inventory\n"
    );
}

#[test]
#[ignore = "development-only regeneration; requires PALE_CAPSULE_COORDINATE_OUTPUT"]
fn regenerate_standalone_capsule_coordinates() -> std::io::Result<()> {
    let directory = std::env::var_os("PALE_CAPSULE_COORDINATE_OUTPUT")
        .map(PathBuf::from)
        .expect("set PALE_CAPSULE_COORDINATE_OUTPUT to a new absolute directory");
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository_root = crate_dir.parent().and_then(Path::parent).unwrap();
    let fixture_root = crate_dir.join("tests/fixtures/standalone");
    assert_eq!(
        regular_files_beneath(&fixture_root, Path::new("")),
        paths(FIXTURE_FILES.iter().copied())
    );
    let inventory = render_inventory(&canonical_input_closure(repository_root));
    let source = read_regular_file(&fixture_root, Path::new("source-capsule.lock.json"));
    let cargo_lock = read_regular_file(&fixture_root, Path::new("Cargo.lock"));
    let lock = regenerate_capsule_lock(&source, &cargo_lock, &inventory);
    write_capsule_coordinates(&directory, &lock, &inventory)
}

fn paths<'a>(values: impl IntoIterator<Item = &'a str>) -> BTreeSet<PathBuf> {
    values.into_iter().map(checked_relative_path).collect()
}

fn parse_inventory(bytes: &[u8]) -> BTreeSet<PathBuf> {
    assert!(
        bytes.ends_with(b"\n"),
        "the source inventory must end in LF"
    );
    let source = std::str::from_utf8(bytes).expect("source inventory is UTF-8");
    let mut ordered = Vec::new();
    let mut previous = None;
    for line in source.lines() {
        assert!(!line.is_empty(), "source inventory contains a blank line");
        assert_eq!(line, line.trim(), "inventory paths have no whitespace");
        if let Some(previous) = previous {
            assert!(
                previous < line,
                "source inventory paths must be unique and byte-sorted"
            );
        }
        ordered.push(checked_relative_path(line));
        previous = Some(line);
    }
    ordered.into_iter().collect()
}

fn checked_relative_path(raw: &str) -> PathBuf {
    assert!(
        !raw.contains('\\'),
        "inventory paths use forward slashes: {raw}"
    );
    let path = PathBuf::from(raw);
    assert!(!path.as_os_str().is_empty(), "inventory path is empty");
    assert!(!path.is_absolute(), "inventory path is absolute: {raw}");
    assert!(
        path.components()
            .all(|component| matches!(component, Component::Normal(_))),
        "inventory path is not a confined relative path: {raw}"
    );
    path
}

fn canonical_input_closure(repository_root: &Path) -> BTreeSet<PathBuf> {
    let mut files = BTreeSet::new();
    for tree in CANONICAL_TREES {
        collect_regular_files(repository_root, Path::new(tree), &mut files);
    }
    for file in paths(CANONICAL_FILES.iter().copied()) {
        let metadata = safe_regular_metadata(repository_root, &file);
        assert!(
            metadata.is_file(),
            "canonical input is not a file: {}",
            file.display()
        );
        assert!(files.insert(file), "duplicate canonical capsule input");
    }
    files
}

fn regular_files_beneath(root: &Path, relative: &Path) -> BTreeSet<PathBuf> {
    let mut files = BTreeSet::new();
    collect_regular_files(root, relative, &mut files);
    files
}

fn collect_regular_files(root: &Path, relative: &Path, files: &mut BTreeSet<PathBuf>) {
    let path = root.join(relative);
    let metadata = safe_path_metadata(root, relative);
    if metadata.is_file() {
        assert!(
            files.insert(relative.to_path_buf()),
            "duplicate inventory path {}",
            relative.display()
        );
        return;
    }
    assert!(metadata.is_dir(), "non-file input {}", path.display());

    let mut entries = fs::read_dir(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
        .map(|entry| entry.expect("read directory entry"))
        .collect::<Vec<_>>();
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        collect_regular_files(root, &relative.join(entry.file_name()), files);
    }
}

fn safe_regular_metadata(root: &Path, relative: &Path) -> Metadata {
    let metadata = safe_path_metadata(root, relative);
    assert!(
        metadata.is_file(),
        "capsule input is not a file: {}",
        root.join(relative).display()
    );
    metadata
}

fn safe_path_metadata(root: &Path, relative: &Path) -> Metadata {
    let root_metadata = fs::symlink_metadata(root)
        .unwrap_or_else(|error| panic!("inspect {}: {error}", root.display()));
    assert!(
        root_metadata.is_dir() && !root_metadata.file_type().is_symlink(),
        "capsule source root must be a real directory: {}",
        root.display()
    );

    let components = relative.components().collect::<Vec<_>>();
    if components.is_empty() {
        return root_metadata;
    }
    let mut current = root.to_path_buf();
    let mut final_metadata = None;
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            panic!("unconfined capsule path {}", relative.display());
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current)
            .unwrap_or_else(|error| panic!("inspect {}: {error}", current.display()));
        assert!(
            !metadata.file_type().is_symlink(),
            "capsule inventory refuses symlink {}",
            current.display()
        );
        if index + 1 == components.len() {
            final_metadata = Some(metadata);
        } else {
            assert!(
                metadata.is_dir(),
                "capsule ancestor is not a directory: {}",
                current.display()
            );
        }
    }
    final_metadata.expect("relative path has a final component")
}

fn read_regular_file(root: &Path, relative: &Path) -> Vec<u8> {
    let path = root.join(relative);
    let before = safe_regular_metadata(root, relative);
    let mut file =
        File::open(&path).unwrap_or_else(|error| panic!("open {}: {error}", path.display()));
    let opened = file
        .metadata()
        .unwrap_or_else(|error| panic!("inspect opened {}: {error}", path.display()));
    same_file(&before, &opened, &path);

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let after = safe_regular_metadata(root, relative);
    same_file(&before, &after, &path);
    bytes
}

#[cfg(unix)]
fn same_file(expected: &Metadata, actual: &Metadata, path: &Path) {
    use std::os::unix::fs::MetadataExt as _;

    assert_eq!(
        (actual.dev(), actual.ino()),
        (expected.dev(), expected.ino()),
        "capsule input changed identity while reading {}",
        path.display()
    );
}

#[cfg(not(unix))]
fn same_file(expected: &Metadata, actual: &Metadata, path: &Path) {
    assert!(
        expected.is_file() && actual.is_file() && expected.len() == actual.len(),
        "capsule input changed while reading {}",
        path.display()
    );
}

fn copy_regular_file(
    source_root: &Path,
    destination_root: &Path,
    relative: &Path,
    forbidden_repository_root: &Path,
) {
    let bytes = read_regular_file(source_root, relative);
    if let Ok(text) = std::str::from_utf8(&bytes) {
        let forbidden = forbidden_repository_root.to_string_lossy();
        assert!(
            !text.contains(forbidden.as_ref()),
            "retained capsule input contains the repository's absolute path: {}",
            relative.display()
        );
    }

    let destination = destination_root.join(relative);
    let parent = destination
        .parent()
        .expect("every retained capsule file has a parent");
    fs::create_dir_all(parent)
        .unwrap_or_else(|error| panic!("create {}: {error}", parent.display()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .unwrap_or_else(|error| panic!("create {}: {error}", destination.display()));
    file.write_all(&bytes)
        .unwrap_or_else(|error| panic!("write {}: {error}", destination.display()));
    file.flush()
        .unwrap_or_else(|error| panic!("flush {}: {error}", destination.display()));
}

fn cargo(root: &Path, arguments: &[&str]) -> Output {
    let executable = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    Command::new(executable)
        .args(arguments)
        .current_dir(root)
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env_remove("CARGO_MANIFEST_DIR")
        .env_remove("CARGO_WORKSPACE_DIR")
        .env_remove("OUT_DIR")
        .output()
        .unwrap_or_else(|error| panic!("run Cargo in {}: {error}", root.display()))
}

fn require_success(label: &str, output: Output) -> Vec<u8> {
    if !output.status.success() {
        panic!(
            "{label} failed with {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    output.stdout
}

fn assert_local_three_crate_metadata(bytes: &[u8], capsule_root: &Path) {
    let metadata: serde_json::Value =
        serde_json::from_slice(bytes).expect("parse Cargo metadata for the capsule");
    let workspace_root = metadata["workspace_root"]
        .as_str()
        .expect("Cargo metadata workspace root");
    assert_eq!(
        Path::new(workspace_root)
            .canonicalize()
            .expect("canonicalize Cargo's capsule workspace root"),
        capsule_root
            .canonicalize()
            .expect("canonicalize expected capsule root")
    );

    let packages = metadata["packages"]
        .as_array()
        .expect("Cargo metadata package list");
    let names = packages
        .iter()
        .map(|package| {
            package["name"]
                .as_str()
                .expect("Cargo metadata package name")
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(names, EXPECTED_PACKAGES.iter().copied().collect());

    let canonical_capsule_root = capsule_root
        .canonicalize()
        .expect("canonicalize capsule root");
    for package in packages {
        assert_eq!(package["source"], serde_json::Value::Null);
        assert_eq!(package["license"], "AGPL-3.0-or-later");
        assert_eq!(package["rust_version"], "1.94");
        assert_eq!(package["publish"], serde_json::json!([]));

        let manifest_path = package["manifest_path"]
            .as_str()
            .expect("Cargo metadata manifest path");
        assert!(
            Path::new(manifest_path)
                .canonicalize()
                .expect("canonicalize capsule package manifest")
                .starts_with(&canonical_capsule_root),
            "package manifest escaped the capsule: {manifest_path}"
        );
        for dependency in package["dependencies"]
            .as_array()
            .expect("Cargo metadata dependencies")
        {
            let Some(path) = dependency["path"].as_str() else {
                continue;
            };
            assert!(
                Path::new(path)
                    .canonicalize()
                    .expect("canonicalize capsule path dependency")
                    .starts_with(&canonical_capsule_root),
                "path dependency escaped the capsule: {path}"
            );
        }
    }
}
