//! Capability witnesses and canonical-vector coverage at the composition root.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use pale_sdk::{capability_inventory, ApiLeaf};

#[path = "support/repository.rs"]
mod repository;

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn string_set(values: &[toml::Value], path: &Path) -> BTreeSet<String> {
    let set = values
        .iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("{} contains a non-string capability", path.display()))
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        set.len(),
        values.len(),
        "{} contains duplicate pending capabilities",
        path.display()
    );
    set
}

fn json_contains(value: &serde_json::Value, expected: &str) -> bool {
    match value {
        serde_json::Value::String(value) => value == expected,
        serde_json::Value::Array(values) => {
            values.iter().any(|value| json_contains(value, expected))
        }
        serde_json::Value::Object(values) => {
            values.values().any(|value| json_contains(value, expected))
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
            false
        }
    }
}

#[test]
fn typescript_public_witnesses_equal_the_api_owned_allowlist() {
    let repository = repository::repository_root();
    let path = repository.join("sdk/js/contracts/tests/capability-witnesses.json");
    let document: serde_json::Value = serde_json::from_str(&read(&path))
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    let witnesses = document
        .as_object()
        .unwrap_or_else(|| panic!("{} is not an object", path.display()))
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected = capability_inventory()
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::TypeScript))
        .map(|capability| capability.id().as_str().to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        witnesses, expected,
        "TypeScript public witnesses must neither widen nor narrow API eligibility"
    );
}

#[test]
fn every_public_capability_has_an_explicit_canonical_vector_disposition() {
    let repository = repository::repository_root();
    let path = repository.join("tools/repository-quality/capability-vector-coverage.toml");
    let document = read(&path)
        .parse::<toml::Table>()
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    assert_eq!(
        document.get("format").and_then(toml::Value::as_integer),
        Some(1),
        "{} has an unsupported format",
        path.display()
    );
    let pending = string_set(
        document
            .get("pending")
            .and_then(toml::Value::as_array)
            .unwrap_or_else(|| panic!("{} has no pending array", path.display())),
        &path,
    );
    let covered = document
        .get("covered")
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("{} has no covered table", path.display()))
        .iter()
        .map(|(capability, vector)| {
            let vector = vector.as_str().unwrap_or_else(|| {
                panic!(
                    "{} has a non-string vector for {capability}",
                    path.display()
                )
            });
            (capability.clone(), vector.to_owned())
        })
        .collect::<BTreeMap<_, _>>();
    let covered_ids = covered.keys().cloned().collect::<BTreeSet<_>>();
    assert!(
        covered_ids.is_disjoint(&pending),
        "a capability cannot be both covered and pending"
    );

    let classified = covered_ids
        .union(&pending)
        .cloned()
        .collect::<BTreeSet<_>>();
    let inventory = capability_inventory()
        .iter()
        .map(|capability| capability.id().as_str().to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        classified, inventory,
        "every public capability requires one explicit vector disposition"
    );

    let vector_root = repository.join("contracts/conformance/cross-leaf.v1");
    let manifest_path = vector_root.join("manifest.json");
    let manifest: serde_json::Value = serde_json::from_str(&read(&manifest_path))
        .unwrap_or_else(|error| panic!("parse {}: {error}", manifest_path.display()));
    let members = manifest["members"]
        .as_array()
        .unwrap_or_else(|| panic!("{} has no member inventory", manifest_path.display()))
        .iter()
        .map(|member| {
            member["path"]
                .as_str()
                .unwrap_or_else(|| {
                    panic!("{} has a member without a path", manifest_path.display())
                })
                .to_owned()
        })
        .collect::<BTreeSet<_>>();

    for (capability, vector) in covered {
        assert!(
            members.contains(&vector),
            "{capability} names an unsealed cross-leaf vector: {vector}"
        );
        let vector_path = vector_root.join(&vector);
        let vector_document: serde_json::Value = serde_json::from_str(&read(&vector_path))
            .unwrap_or_else(|error| panic!("parse {}: {error}", vector_path.display()));
        assert!(
            json_contains(&vector_document, &capability),
            "{capability} is not named by its selected canonical vector {vector}"
        );
    }
}
