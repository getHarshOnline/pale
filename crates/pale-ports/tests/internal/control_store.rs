//! Behavioral tests for neutral physical control-store primitives.

use super::*;
use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, ControlTransactionSequenceV1, ResidentRecordRefV1,
};

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record reference"),
        ControlRecordDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>())
            .expect("record digest"),
    )
}

fn checkpoint(domain: &str, epoch: u64, sequence: u64, reference: &str) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse(domain).expect("checkpoint domain"),
        ControlDomainEpochV1::new(epoch).expect("domain epoch"),
        ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        ControlTransactionSequenceV1::new(29).expect("independent transaction"),
        coordinate(reference, reference.chars().last().unwrap_or('a')),
    )
}

#[test]
fn record_values_reject_noncanonical_digest_and_empty_bytes() {
    assert_eq!(
        ControlRecordDigest::parse("A".repeat(64)),
        Err(ControlRecordError::InvalidDigest)
    );
    assert_eq!(
        ControlRecordBytes::new(Vec::new()),
        Err(ControlRecordError::EmptyBytes)
    );
    assert_eq!(
        ControlRecordBytes::new(br#"{"record":"exact"}"#.to_vec())
            .expect("nonempty record")
            .as_slice(),
        br#"{"record":"exact"}"#
    );
}

#[test]
fn custody_debug_output_depends_on_payload_length_not_payload_contents() {
    let left_payload = br#"{"input":"private-a"}"#;
    let right_payload = br#"{"input":"private-b"}"#;
    let record = |payload: &[u8]| {
        ControlRecord::new(
            coordinate("record-a", 'a'),
            SchemaId::parse("pale/common-request-envelope.v1").expect("schema id"),
            SchemaDigest::parse("b".repeat(64)).expect("schema digest"),
            ControlRecordBytes::new(payload.to_vec()).expect("nonempty bytes"),
        )
    };
    let left = record(left_payload);
    let right = record(right_payload);
    assert_ne!(left, right);
    assert_eq!(left.canonical_record_jcs().as_slice(), left_payload);
    assert_eq!(right.canonical_record_jcs().as_slice(), right_payload);
    for (left_debug, right_debug) in [
        (format!("{left:?}"), format!("{right:?}")),
        (format!("{left:#?}"), format!("{right:#?}")),
    ] {
        assert_eq!(left_debug, right_debug);
        assert!(!left_debug.contains("private-a"));
        assert!(!left_debug.contains(&format!("{left_payload:?}")));
    }
}

#[test]
fn checkpoint_successor_preserves_domain_and_epoch_and_advances_once() {
    let predecessor = checkpoint("effect:resource-1", 3, 7, "checkpoint-a");
    assert!(
        checkpoint("effect:resource-1", 3, 8, "checkpoint-b").is_exact_successor_of(&predecessor)
    );
    assert!(
        !checkpoint("effect:resource-1", 3, 9, "checkpoint-c").is_exact_successor_of(&predecessor)
    );
    assert!(
        !checkpoint("effect:resource-1", 4, 8, "checkpoint-b").is_exact_successor_of(&predecessor)
    );
    assert!(
        !checkpoint("effect:resource-2", 3, 8, "checkpoint-b").is_exact_successor_of(&predecessor)
    );
    assert!(
        !checkpoint("effect:resource-1", 3, 8, "checkpoint-a").is_exact_successor_of(&predecessor)
    );
}

#[test]
fn transaction_proposal_binds_store_and_incarnation_and_rejects_exhaustion() {
    let store = |incarnation| {
        crate::AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("transaction-store").unwrap(),
            ControlIncarnationIdV1::parse(incarnation).unwrap(),
        )
    };
    let expected = ControlTransactionSequenceV1::new(41).unwrap();
    let proposal = ControlTransactionPreparation::new(store("incarnation-a"), expected).unwrap();
    assert_eq!(proposal.store(), &store("incarnation-a"));
    assert_eq!(proposal.expected(), expected);
    assert_eq!(proposal.successor().get(), 42);
    assert_ne!(
        proposal,
        ControlTransactionPreparation::new(store("incarnation-b"), expected).unwrap()
    );
    assert!(ControlTransactionPreparation::new(
        store("incarnation-a"),
        ControlTransactionSequenceV1::new(9_007_199_254_740_991).unwrap(),
    )
    .is_err());
}

#[test]
fn checkpoint_domain_succession_does_not_allocate_a_store_transaction() {
    let predecessor = checkpoint("effect:resource-1", 3, 7, "checkpoint-a");
    let successor = |transaction| {
        ControlCheckpointHead::new(
            predecessor.checkpoint_domain_id().clone(),
            predecessor.domain_epoch(),
            ControlDomainSequenceV1::new(8).unwrap(),
            ControlTransactionSequenceV1::new(transaction).unwrap(),
            coordinate("checkpoint-b", 'b'),
        )
    };
    assert!(successor(29).is_exact_successor_of(&predecessor));
    assert!(successor(41).is_exact_successor_of(&predecessor));
    assert!(!successor(28).is_exact_successor_of(&predecessor));
    assert_eq!(predecessor.domain_sequence().get(), 7);
    assert_eq!(predecessor.committed_transaction_sequence().get(), 29);
}
