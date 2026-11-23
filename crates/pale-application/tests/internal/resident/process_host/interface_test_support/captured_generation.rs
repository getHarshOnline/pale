//! Checked captured-generation fixture tests.

use super::captured_generation::*;
use pale_core::records::DistributionLockFile;

#[test]
fn checked_input_inventory_is_complete_and_platform_neutral() {
    let documents = source_fixture_documents();
    let expected_paths = [
        "sources/packages/engine/operations.toml",
        "sources/packages/engine/package.toml",
        "sources/packages/engine/policy.toml",
        "sources/packages/engine/review.md",
        "sources/packages/engine/role.md",
        "sources/packages/engine/roles.toml",
        "sources/private/capabilities.toml",
        "sources/private/distribution-lock.toml",
        "sources/private/machine.toml",
        "sources/workspace/composition.toml",
        "sources/workspace/pale-workspace.toml",
        "sources/workspace/workspace-resolution.lock.toml",
        "sources/workspace/workspace-universe.lock.toml",
    ];
    let mut paths = documents
        .iter()
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    paths.sort_unstable();
    assert_eq!(paths, expected_paths);

    let bootstrap = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/resident/invoke-conformance.v1/principal/bootstrap-authority-inputs.json"
    ));
    let total_bytes = documents
        .iter()
        .map(|(_, bytes)| bytes.len())
        .sum::<usize>()
        + bootstrap.len();
    assert_eq!(documents.len() + 1, 14);
    assert_eq!(total_bytes, 31_213);

    let forbidden_paths = [
        "compiled-generation",
        "lifecycle",
        "operation-handle",
        "observation",
        "response",
        "result",
        "session",
    ];
    assert!(paths.iter().all(|path| {
        forbidden_paths
            .iter()
            .all(|forbidden| !path.contains(forbidden))
    }));

    let all_input = documents
        .iter()
        .flat_map(|(_, bytes)| bytes.iter().copied())
        .chain(bootstrap.iter().copied())
        .collect::<Vec<_>>();
    let text = String::from_utf8(all_input).expect("fixture inputs are UTF-8");
    for target_coordinate in ["darwin", "linux", "windows", "aarch64", "x86_64"] {
        assert!(!text.contains(target_coordinate));
    }
}

#[test]
fn checked_distribution_lock_matches_the_admitted_package_bytes() {
    let inspected = package_source()
        .inspect_distribution()
        .expect("checked Package bytes admit for lock inspection");
    let lock: DistributionLockFile = toml::from_str(
        std::str::from_utf8(&distribution_lock_source())
            .expect("the checked distribution lock is UTF-8"),
    )
    .expect("the checked distribution lock parses");

    assert_eq!(inspected.compiler(), lock.distribution.compiler);
    assert_eq!(
        inspected.schema_manifest_sha256(),
        &lock.distribution.schema_manifest_sha256
    );
    assert_eq!(
        inspected.schema_set_sha256(),
        &lock.distribution.schema_set_sha256
    );
    assert_eq!(inspected.packages(), lock.packages);
}
