//! Real isolated HTTPS etcd quorum; no installed service or production authority.

#![cfg(unix)]

mod support;

use std::{num::NonZeroU64, sync::atomic::Ordering};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, PrincipalTrustCheckpointEpochV1,
    ResidentPrincipalIdV1, ResidentRecordRefV1,
};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition as Advance,
    AntiRollbackAnchorErrorKind as ErrorKind, AntiRollbackAnchorObservation as Observation,
    AntiRollbackAnchorPort, ControlDomainAntiRollbackAnchor, ControlRecordCoordinate,
    ControlRecordDigest, PreparedAntiRollbackAnchorTransition as Transition,
    PrincipalTrustMinimumAnchor,
};
use serde_json::json;

use support::{
    proxy::{Fault, Proxy},
    Quorum,
};

fn coordinate(name: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(name).unwrap(),
        ControlRecordDigest::parse(byte.to_string().repeat(64)).unwrap(),
    )
}

fn principal(epoch: u64, byte: char) -> AntiRollbackAnchor {
    PrincipalTrustMinimumAnchor::new(
        ResidentPrincipalIdV1::parse("fixture-principal").unwrap(),
        PrincipalTrustCheckpointEpochV1::new(epoch).unwrap(),
        coordinate(&format!("trust-{epoch}-{byte}"), byte),
    )
    .into()
}

fn domain(sequence: u64) -> AntiRollbackAnchor {
    ControlDomainAntiRollbackAnchor::new(
        ControlStoreIdV1::parse("fixture-store").unwrap(),
        ControlIncarnationIdV1::parse("fixture-incarnation").unwrap(),
        ControlCheckpointDomainIdV1::parse("apply:a").unwrap(),
        ControlDomainEpochV1::new(4).unwrap(),
        ControlDomainSequenceV1::new(sequence).unwrap(),
        coordinate(&format!("domain-{sequence}"), 'd'),
    )
    .into()
}

#[test]
#[ignore = "requires verified etcd 3.6.16 binary and openssl; starts only an owned local quorum"]
fn native_quorum_cas_recovery_and_fault_boundaries() {
    let mut quorum = Quorum::start();
    let first = principal(1, '1');
    let second = principal(2, '2');
    let third = principal(3, '3');
    let initial = Transition::initial(first.clone()).unwrap();
    let step = Transition::successor(first.clone(), second.clone()).unwrap();
    let mut store = quorum.store(0);
    assert_eq!(store.read(initial.key()).unwrap(), None);
    assert_eq!(
        store.compare_and_advance(&initial).unwrap(),
        Advance::Advanced
    );
    assert_eq!(
        store.compare_and_advance(&initial).unwrap(),
        Advance::ExactReplay
    );
    assert_eq!(store.compare_and_advance(&step).unwrap(), Advance::Advanced);
    let mut other_member = quorum.store(1);
    assert_eq!(other_member.read(step.key()).unwrap(), Some(second.clone()));
    assert!(matches!(
        other_member.compare_and_advance(&initial).unwrap(),
        Advance::NotAdvanced(Observation::Ahead { .. })
    ));
    let domain_initial = Transition::initial(domain(1)).unwrap();
    assert_eq!(
        store.compare_and_advance(&domain_initial).unwrap(),
        Advance::Advanced
    );
    assert_eq!(store.read(initial.key()).unwrap(), Some(second.clone()));
    let wrong_id = NonZeroU64::new(if quorum.cluster.get() == 1 { 2 } else { 1 }).unwrap();
    assert_eq!(
        quorum
            .store_at(&quorum.endpoint(0), wrong_id, &quorum.token)
            .read(initial.key())
            .unwrap_err()
            .kind(),
        ErrorKind::Unavailable
    );
    assert_eq!(
        quorum
            .store_at(&quorum.endpoint(0), quorum.cluster, "invalid-fixture-token")
            .read(initial.key())
            .unwrap_err()
            .kind(),
        ErrorKind::Unavailable
    );

    // Both writers read the same predecessor before their actual etcd CAS.
    let proxy = Proxy::start(&quorum, Fault::FenceReadResponses);
    let mut left = quorum.store_at(&proxy.endpoint(), quorum.cluster, &quorum.token);
    let mut right = quorum.store_at(&proxy.endpoint(), quorum.cluster, &quorum.token);
    let a = Transition::successor(second.clone(), third.clone()).unwrap();
    let b = Transition::successor(second.clone(), principal(3, '4')).unwrap();
    let (left, right) = std::thread::scope(|s| {
        let left = s.spawn(|| left.compare_and_advance(&a));
        let right = s.spawn(|| right.compare_and_advance(&b));
        (
            left.join().unwrap().unwrap(),
            right.join().unwrap().unwrap(),
        )
    });
    assert_eq!(
        usize::from(left == Advance::Advanced) + usize::from(right == Advance::Advanced),
        1
    );
    assert!(matches!(
        (&left, &right),
        (
            Advance::Advanced,
            Advance::NotAdvanced(Observation::Conflicting { .. })
        ) | (
            Advance::NotAdvanced(Observation::Conflicting { .. }),
            Advance::Advanced
        )
    ));
    assert_eq!(proxy.transactions.load(Ordering::SeqCst), 2);
    drop(proxy);
    let retained = store.read(initial.key()).unwrap().unwrap();

    // A real transaction commits, but its reply is lost after etcd responds.
    let proxy = Proxy::start(&quorum, Fault::DropTransactionReply);
    let mut uncertain = quorum.store_at(&proxy.endpoint(), quorum.cluster, &quorum.token);
    let next = principal(4, '5');
    let transition = Transition::successor(retained, next.clone()).unwrap();
    assert_eq!(
        uncertain
            .compare_and_advance(&transition)
            .unwrap_err()
            .kind(),
        ErrorKind::OutcomeUnknown
    );
    assert_eq!(
        proxy.transactions.load(Ordering::SeqCst),
        1,
        "no native write retry"
    );
    assert_eq!(
        store.observe(&transition).unwrap(),
        Observation::ExactSuccessor
    );
    drop(proxy);

    quorum.stop(0);
    assert_eq!(
        other_member.read(initial.key()).unwrap(),
        Some(next.clone())
    );
    quorum.restart(0);
    quorum.wait_ready(0);
    // The fixture owner refreshes its native token explicitly after restart.
    // Credential persistence is not an anchor-durability assumption.
    quorum.refresh_token();
    store = quorum.store(0);
    assert_eq!(store.read(initial.key()).unwrap(), Some(next.clone()));
    for member in 0..3 {
        quorum.stop(member);
    }
    for member in 0..3 {
        quorum.restart(member);
    }
    quorum.wait_ready(0);
    quorum.refresh_token();
    store = quorum.store(0);
    assert_eq!(store.read(initial.key()).unwrap(), Some(next));
    assert_eq!(store.read(domain_initial.key()).unwrap(), Some(domain(1)));

    // Malformed external custody is never overwritten as a missing anchor.
    let keys=quorum.post(0,"/v3/kv/range",json!({"key":STANDARD.encode(b"/pale-test/"),"range_end":STANDARD.encode(b"/pale-test0"),"limit":"10"}));
    let key = keys["kvs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| {
            let bytes = STANDARD.decode(v["value"].as_str().unwrap()).unwrap();
            String::from_utf8(bytes)
                .unwrap()
                .contains("fixture-principal")
        })
        .unwrap()["key"]
        .as_str()
        .unwrap();
    quorum.post(
        0,
        "/v3/kv/put",
        json!({"key":key,"value":STANDARD.encode(b"not an anchor")}),
    );
    assert_eq!(
        store.compare_and_advance(&initial).unwrap_err().kind(),
        ErrorKind::Unavailable
    );
    let retained = quorum.post(0, "/v3/kv/range", json!({"key":key}));
    assert_eq!(
        STANDARD
            .decode(retained["kvs"][0]["value"].as_str().unwrap())
            .unwrap(),
        b"not an anchor"
    );
    // A native value larger than the bounded response is not treated as absent.
    quorum.post(
        0,
        "/v3/kv/put",
        json!({"key":key,"value":STANDARD.encode(vec![b'x';80 * 1024])}),
    );
    assert_eq!(
        store.read(initial.key()).unwrap_err().kind(),
        ErrorKind::Unavailable
    );
    assert_eq!(store.read(domain_initial.key()).unwrap(), Some(domain(1)));
}
