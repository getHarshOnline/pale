//! Canonical producer-incarnation head-root tests.

use super::*;

fn head(principal_id: &str, producer_id: &str, epoch: char) -> ProducerIncarnationHeadV1 {
    let mut preimage = producer_preimage(epoch);
    preimage.origin_principal_id = ResidentPrincipalIdV1::parse(principal_id).expect("principal");
    preimage.issuer_principal_id = preimage.origin_principal_id.clone();
    preimage.producer_id = ProducerIdV1::parse(producer_id).expect("producer");
    let incarnation = producer(preimage);
    ProducerIncarnationHeadV1::active(
        &record_ref(&format!("incarnation-{principal_id}-{producer_id}-{epoch}")),
        &incarnation,
    )
    .expect("active producer head")
}

#[test]
fn producer_head_root_has_stable_empty_single_and_multi_vectors() {
    let single = vec![head("principal-1", "producer-a", 'a')];
    let multiple = vec![
        head("principal-1", "producer-a", 'a'),
        head("principal-1", "producer-b", 'b'),
        head("principal-2", "producer-a", 'c'),
    ];

    assert_eq!(
        producer_incarnation_heads_root_sha256(&[])
            .expect("empty root")
            .as_str(),
        "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945"
    );
    assert_eq!(
        producer_incarnation_heads_root_sha256(&single)
            .expect("single root")
            .as_str(),
        "b181e126af54c89a456c951ee4aa5eff6fdc95e9cfed92c2592dd557f7aff8e3"
    );
    assert_eq!(
        producer_incarnation_heads_root_sha256(&multiple)
            .expect("multi root")
            .as_str(),
        "5c0dbe7c733131fc7f2bb8a2be03ac1816f4fe0cb4bf0e71c9d072e024362cc7"
    );
}

#[test]
fn producer_head_root_rejects_duplicate_and_out_of_order_keys() {
    let first = head("principal-1", "producer-a", 'a');
    let duplicate = head("principal-1", "producer-a", 'b');
    assert!(matches!(
        producer_incarnation_heads_root_sha256(&[first.clone(), duplicate]),
        Err(InvalidResidentRecordV1::NonCanonical {
            relation: "producer incarnation heads"
        })
    ));

    let later = head("principal-1", "producer-b", 'b');
    assert!(matches!(
        producer_incarnation_heads_root_sha256(&[later, first]),
        Err(InvalidResidentRecordV1::NonCanonical {
            relation: "producer incarnation heads"
        })
    ));
}

#[test]
fn active_and_retired_projections_preserve_allocation_identity() {
    let incarnation_ref = record_ref("incarnation-a");
    let incarnation = producer(producer_preimage('a'));
    let active = ProducerIncarnationHeadV1::active(&incarnation_ref, &incarnation)
        .expect("active projection");
    let retirement = ProducerIncarnationRetirementV1::seal(
        ProducerIncarnationRetirementPreimageV1 {
            schema_id: SchemaId::parse(PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA)
                .expect("retirement schema"),
            schema_sha256: schema_digest('d'),
            retirement_id: ProducerIncarnationRetirementIdV1::parse("retirement-a")
                .expect("retirement id"),
            incarnation_ref: incarnation_ref.clone(),
            incarnation_sha256: incarnation.canonical_incarnation_sha256().clone(),
            origin_principal_id: incarnation.origin_principal_id().clone(),
            producer_id: incarnation.producer_id().clone(),
            producer_epoch: incarnation.producer_epoch().clone(),
            producer_incarnation_authority_ref: incarnation
                .producer_incarnation_authority_ref()
                .clone(),
            producer_incarnation_authority_sha256: incarnation
                .producer_incarnation_authority_sha256()
                .clone(),
            reason_code: ResidentReasonV1::parse("restart").expect("reason"),
            retired_at: UnixTimeV1::new(11).expect("retirement time"),
            issuer_principal_id: incarnation.issuer_principal_id().clone(),
            issuer_key_id: incarnation.issuer_key_id().clone(),
            issuer_key_epoch: *incarnation.issuer_key_epoch(),
        },
        signature(),
    )
    .expect("retirement");
    let retired = ProducerIncarnationHeadV1::retired(&incarnation_ref, &incarnation, &retirement)
        .expect("retired projection");

    assert_eq!(active.key(), retired.key());
    assert_eq!(active.incarnation_ref(), retired.incarnation_ref());
    assert_eq!(active.incarnation_sha256(), retired.incarnation_sha256());
    assert_eq!(active.producer_epoch(), retired.producer_epoch());
    assert_eq!(active.state(), ProducerIncarnationHeadStateV1::Active);
    assert_eq!(retired.state(), ProducerIncarnationHeadStateV1::Retired);
    assert_ne!(
        producer_incarnation_heads_root_sha256(&[active]).expect("active root"),
        producer_incarnation_heads_root_sha256(&[retired]).expect("retired root")
    );
}
