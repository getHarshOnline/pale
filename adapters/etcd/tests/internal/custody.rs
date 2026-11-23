//! Corrupt external custody and incomplete transaction evidence are fail closed.

use super::*;
use pale_core::records::{
    PrincipalTrustCheckpointEpochV1, ResidentPrincipalIdV1, ResidentRecordRefV1,
};
use pale_ports::{ControlRecordCoordinate, ControlRecordDigest, PrincipalTrustMinimumAnchor};

fn anchor(name: &str) -> AntiRollbackAnchor {
    PrincipalTrustMinimumAnchor::new(
        ResidentPrincipalIdV1::parse(name).unwrap(),
        PrincipalTrustCheckpointEpochV1::new(1).unwrap(),
        ControlRecordCoordinate::new(
            ResidentRecordRefV1::parse("fixture-checkpoint").unwrap(),
            ControlRecordDigest::parse("a".repeat(64)).unwrap(),
        ),
    )
    .into()
}

fn store() -> EtcdAnchorStore {
    // No network operations or trust provisioning in these decoder tests.
    EtcdAnchorStore {
        endpoint: Url::parse("https://127.0.0.1:1").unwrap(),
        client: Client::builder().no_proxy().build().unwrap(),
        token: HeaderValue::from_static("fixture-only"),
        cluster_id: NonZeroU64::new(1).unwrap(),
        prefix: b"/fixture/".to_vec(),
    }
}

fn range(anchor: &AntiRollbackAnchor, raw_key: &[u8]) -> Value {
    json!({"header":{"revision":"8"},"count":"1","kvs":[{
        "key":STANDARD.encode(raw_key),"value":STANDARD.encode(codec::encode(anchor).unwrap()),
        "mod_revision":"8"
    }]})
}

#[test]
fn custody_encoding_refuses_unknown_or_noncanonical_representations() {
    let expected = anchor("fixture-principal");
    let bytes = codec::encode(&expected).unwrap();
    assert_eq!(codec::decode(&bytes).unwrap(), expected);
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["version"] = json!(2);
    assert!(codec::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["unexpected"] = json!(true);
    assert!(codec::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut noncanonical = bytes;
    noncanonical.push(b'\n');
    assert!(codec::decode(&noncanonical).is_err());
}

#[test]
fn range_refuses_foreign_expiring_or_incomplete_custody() {
    let expected = anchor("fixture-principal");
    let key = expected.key();
    let raw_key = codec::physical_key(b"/fixture/", &key).unwrap();
    let native = range(&expected, &raw_key);
    let decode =
        |value| gateway::decode_range(serde_json::from_value(value).unwrap(), &raw_key, &key);
    assert_eq!(decode(native.clone()).unwrap().unwrap().anchor, expected);
    for (field, value) in [
        ("key", json!(STANDARD.encode(b"foreign-key"))),
        ("lease", json!("19")),
        ("mod_revision", json!("0")),
        (
            "value",
            json!(STANDARD.encode(codec::encode(&anchor("other-principal")).unwrap())),
        ),
    ] {
        let mut bad = native.clone();
        bad["kvs"][0][field] = value;
        assert!(decode(bad).is_err(), "{field}");
    }
    for (field, value) in [("more", json!(true)), ("count", json!("0"))] {
        let mut bad = native.clone();
        bad[field] = value;
        assert!(decode(bad).is_err(), "{field}");
    }
}

#[test]
fn advanced_requires_admitted_root_header_and_same_transaction_readback() {
    let store = store();
    let expected = anchor("fixture-principal");
    let transition = PreparedAntiRollbackAnchorTransition::initial(expected.clone()).unwrap();
    let raw_key = codec::physical_key(&store.prefix, transition.key()).unwrap();
    // Captures the native v3.6.16 shape: IDs in the root, revision in children.
    let native = json!({"header":{"cluster_id":"1","member_id":"2","revision":"8"},
        "succeeded":true,"responses":[{"response_put":{"header":{"revision":"8"}}},
        {"response_range":range(&expected,&raw_key)}]});
    assert_eq!(
        store
            .transaction_result(&transition, &raw_key, native.clone())
            .unwrap(),
        Advance::Advanced
    );
    let mut bad = native.clone();
    bad["header"]["cluster_id"] = json!("3");
    assert!(store
        .transaction_result(&transition, &raw_key, bad)
        .is_err());
    let mut bad = native.clone();
    bad["header"].as_object_mut().unwrap().remove("cluster_id");
    assert!(store
        .transaction_result(&transition, &raw_key, bad)
        .is_err());
    let mut bad = native.clone();
    bad["responses"][1]["response_range"]["header"]["revision"] = json!("7");
    assert!(store
        .transaction_result(&transition, &raw_key, bad)
        .is_err());
    let mut bad = native;
    bad["responses"][1]["response_range"]["kvs"][0]["mod_revision"] = json!("7");
    assert!(store
        .transaction_result(&transition, &raw_key, bad)
        .is_err());
}
