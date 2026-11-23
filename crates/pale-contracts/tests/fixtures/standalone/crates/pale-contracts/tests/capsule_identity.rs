//! Local source-capsule identity proof.

use pale_contracts::{schema_count, schema_registry_identity};

#[test]
fn capsule_embeds_the_locked_current_bundle_coordinate() {
    let lock: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../source-capsule.lock.json"
    )))
    .expect("parse the retained source-capsule proof lock");
    assert_eq!(lock["format"], "pale-contract-source-capsule-lock.v1");
    assert_eq!(
        lock["proof_scope"],
        "local_source_capsule_build_independence_only"
    );

    let cargo_lock = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock"));
    assert_eq!(
        pale_core::sha256_hex(cargo_lock),
        lock["cargo_lock_sha256"]
            .as_str()
            .expect("locked Cargo dependency graph digest")
    );

    let inventory = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../source-inventory.v1.txt"
    ));
    assert_eq!(
        pale_core::sha256_hex(inventory),
        lock["source_inventory_sha256"]
            .as_str()
            .expect("locked source-inventory digest")
    );

    let count = schema_count().expect("verify the capsule's embedded registry");
    let identity = schema_registry_identity().expect("read the capsule's bundle coordinate");
    assert_eq!(
        u64::try_from(count).expect("schema count fits u64"),
        lock["schema_count"].as_u64().expect("locked schema count")
    );
    assert_eq!(
        identity.schema_manifest_sha256.as_str(),
        lock["schema_manifest_sha256"]
            .as_str()
            .expect("locked manifest identity")
    );
    assert_eq!(
        identity.schema_set_sha256.as_str(),
        lock["schema_set_sha256"]
            .as_str()
            .expect("locked set identity")
    );
}
