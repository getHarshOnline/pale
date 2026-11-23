//! Producer-incarnation transition-law tests.

use std::cell::Cell;

use super::*;

#[test]
fn producer_epoch_enforces_the_shared_canonical_resident_bound() {
    for length in [32, 512] {
        let value = std::iter::repeat_n('a', length).collect::<String>();
        assert_eq!(
            ProducerEpochV1::parse(value.clone()).unwrap().as_str(),
            value
        );
    }
    for value in [
        "a".repeat(30),
        "a".repeat(33),
        "A".repeat(32),
        "g".repeat(32),
        "a".repeat(514),
    ] {
        assert!(ProducerEpochV1::parse(value).is_err());
    }
}

#[test]
fn producer_initial_admission_authenticates_before_owner_uniqueness_check() {
    let principal_id = principal();
    let authority_ref = record_ref("producer-authority-1");
    let authority_record = producer_authority_record(producer_authority_preimage());
    let authority = producer_authority_context(&authority_ref, &authority_record);
    let preimage = producer_preimage('a');
    let uniqueness_observed = Cell::new(false);
    let rejected = ProducerIncarnationV1::admit_initial_wire(
        producer_wire(preimage.clone()),
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
        &preimage.schema_sha256,
        &authority,
        |_, _| {
            uniqueness_observed.set(true);
            Ok(())
        },
        fixture_namespace_coverage,
        |domain, _, _, _, _| {
            assert_eq!(domain, PRODUCER_INCARNATION_V1_SIGNATURE_DOMAIN);
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "test signature",
                invariant: "authenticated",
            })
        },
    );
    assert!(rejected.is_err());
    assert!(
        !uniqueness_observed.get(),
        "unauthenticated input must not query principal epoch allocation"
    );

    ProducerIncarnationV1::admit_initial_wire(
        producer_wire(preimage.clone()),
        &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
        &preimage.schema_sha256,
        &authority,
        |observed_principal, observed_epoch| {
            assert_eq!(observed_principal, &principal_id);
            assert_eq!(observed_epoch, &preimage.producer_epoch);
            Ok(())
        },
        fixture_namespace_coverage,
        |domain, public_key, _, _, _| {
            assert_eq!(domain, PRODUCER_INCARNATION_V1_SIGNATURE_DOMAIN);
            assert_eq!(public_key, authority_record.subject_public_key());
            Ok(())
        },
    )
    .expect("authenticated globally unallocated epoch admits");

    assert!(
        ProducerIncarnationV1::admit_initial_wire(
            producer_wire(preimage.clone()),
            &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
            &schema_digest('0'),
            &authority,
            |_, _| Ok(()),
            fixture_namespace_coverage,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "schema selection is exact"
    );
    let wrong_authority_ref = record_ref("producer-authority-2");
    let wrong_authority = producer_authority_context(&wrong_authority_ref, &authority_record);
    assert!(
        ProducerIncarnationV1::admit_initial_wire(
            producer_wire(preimage.clone()),
            &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
            &preimage.schema_sha256,
            &wrong_authority,
            |_, _| Ok(()),
            fixture_namespace_coverage,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "authority reference is exact"
    );
    let mut changed_authority = producer_authority_preimage();
    changed_authority.subject_public_key =
        Ed25519PublicKeyV1::parse("Q".repeat(43)).expect("changed public key");
    let changed_authority = producer_authority_record(changed_authority);
    let wrong_authority = producer_authority_context(&authority_ref, &changed_authority);
    assert!(
        ProducerIncarnationV1::admit_initial_wire(
            producer_wire(preimage.clone()),
            &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
            &preimage.schema_sha256,
            &wrong_authority,
            |_, _| Ok(()),
            fixture_namespace_coverage,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "authority digest is exact"
    );
    let mut wrong_key = preimage.clone();
    wrong_key.issuer_key_id = ResidentKeyIdV1::parse("key-2").expect("wrong subject key");
    assert!(
        ProducerIncarnationV1::admit_initial_wire(
            producer_wire(wrong_key),
            &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
            &preimage.schema_sha256,
            &authority,
            |_, _| Ok(()),
            fixture_namespace_coverage,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "authority subject key is exact"
    );

    let predecessor = producer(producer_preimage('b'));
    let mut contaminated = preimage;
    contaminated.previous_incarnation_ref = Some(record_ref("incarnation-b"));
    contaminated.previous_incarnation_sha256 =
        Some(predecessor.canonical_incarnation_sha256().clone());
    assert!(
        ProducerIncarnationV1::admit_initial_wire(
            producer_wire(contaminated),
            &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
            &schema_digest('1'),
            &authority,
            |_, _| Ok(()),
            fixture_namespace_coverage,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "initial allocation cannot carry a predecessor"
    );
}

#[test]
fn producer_authority_state_interval_namespace_and_key_are_exact() {
    let authority_ref = record_ref("producer-authority-1");

    for (law, mutate) in [
        (
            "active state",
            (PrincipalAuthorityStateV1::Retired, 5_u64, 20_u64),
        ),
        (
            "inclusive issue time",
            (PrincipalAuthorityStateV1::Active, 11, 20),
        ),
        (
            "exclusive expiry time",
            (PrincipalAuthorityStateV1::Active, 1, 10),
        ),
    ] {
        let (state, issued_at, expires_at) = mutate;
        let mut authority_preimage = producer_authority_preimage();
        authority_preimage.state = state;
        authority_preimage.issued_at = UnixTimeV1::new(issued_at).expect("issue time");
        authority_preimage.expires_at = UnixTimeV1::new(expires_at).expect("expiry time");
        let authority_record = producer_authority_record(authority_preimage);
        let authority = producer_authority_context(&authority_ref, &authority_record);
        let candidate = bind_producer_authority(producer_preimage('a'), &authority_record);
        assert!(
            ProducerIncarnationV1::admit_initial_wire(
                producer_wire(candidate.clone()),
                &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
                &candidate.schema_sha256,
                &authority,
                |_, _| Ok(()),
                fixture_namespace_coverage,
                |_, _, _, _, _| Ok(()),
            )
            .is_err(),
            "producer authority requires {law}"
        );
    }

    let authority_record = producer_authority_record(producer_authority_preimage());
    let authority = producer_authority_context(&authority_ref, &authority_record);
    let candidate = producer_preimage('a');
    assert!(
        ProducerIncarnationV1::admit_initial_wire(
            producer_wire(candidate.clone()),
            &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
            &candidate.schema_sha256,
            &authority,
            |_, _| Ok(()),
            |_, _| false,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "owner-resolved namespace coverage is mandatory"
    );

    let mut wrong_epoch = candidate;
    wrong_epoch.issuer_key_epoch = SigningKeyEpochV1::new(2).expect("changed signer epoch");
    assert!(
        ProducerIncarnationV1::admit_initial_wire(
            producer_wire(wrong_epoch.clone()),
            &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
            &wrong_epoch.schema_sha256,
            &authority,
            |_, _| Ok(()),
            fixture_namespace_coverage,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "delegated signer epoch is exact and cannot be a principal-key epoch"
    );
}

#[test]
fn producer_rotation_requires_exact_retirement_before_a_fresh_successor() {
    let authority_ref = record_ref("producer-authority-1");
    let authority_record = producer_authority_record(producer_authority_preimage());
    let authority = producer_authority_context(&authority_ref, &authority_record);
    let predecessor_ref = record_ref("incarnation-a");
    let predecessor = producer(producer_preimage('a'));
    let active_head = ProducerIncarnationHeadAdmissionStateV1::Active {
        incarnation_ref: &predecessor_ref,
        incarnation: &predecessor,
    };

    let mut successor = producer_preimage('b');
    successor.previous_incarnation_ref = Some(predecessor_ref.clone());
    successor.previous_incarnation_sha256 =
        Some(predecessor.canonical_incarnation_sha256().clone());
    successor.allocated_at = UnixTimeV1::new(13).expect("valid fixture time");
    assert!(
        ProducerIncarnationV1::admit_successor_wire(
            producer_wire(successor.clone()),
            &active_head,
            &successor.schema_sha256,
            &authority,
            |_, _| Ok(()),
            fixture_namespace_coverage,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "an active allocation must be retired before rotation"
    );

    let retirement_preimage = ProducerIncarnationRetirementPreimageV1 {
        schema_id: SchemaId::parse(PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA)
            .expect("retirement schema"),
        schema_sha256: schema_digest('4'),
        retirement_id: ProducerIncarnationRetirementIdV1::parse("retirement-a")
            .expect("retirement id"),
        incarnation_ref: predecessor_ref.clone(),
        incarnation_sha256: predecessor.canonical_incarnation_sha256().clone(),
        origin_principal_id: predecessor.origin_principal_id().clone(),
        producer_id: predecessor.producer_id().clone(),
        producer_epoch: predecessor.producer_epoch().clone(),
        producer_incarnation_authority_ref: authority_ref.clone(),
        producer_incarnation_authority_sha256: authority_record
            .canonical_producer_authority_sha256()
            .clone(),
        reason_code: ResidentReasonV1::parse("clean rotation").expect("reason"),
        retired_at: UnixTimeV1::new(12).expect("valid fixture time"),
        issuer_principal_id: principal(),
        issuer_key_id: issuer_key(),
        issuer_key_epoch: authority_key_epoch(),
    };
    let retirement = ProducerIncarnationRetirementV1::admit_contextual_wire(
        wire_from(
            &ProducerIncarnationRetirementV1::seal(retirement_preimage.clone(), signature())
                .expect("retirement"),
        ),
        &active_head,
        &retirement_preimage.schema_sha256,
        &authority,
        fixture_namespace_coverage,
        |domain, public_key, _, _, _| {
            assert_eq!(domain, PRODUCER_INCARNATION_RETIREMENT_V1_SIGNATURE_DOMAIN);
            assert_eq!(public_key, authority_record.subject_public_key());
            Ok(())
        },
    )
    .expect("exact active allocation retirement admits");

    let mut invalid_retirements = Vec::new();
    let mut changed = retirement_preimage.clone();
    changed.incarnation_ref = record_ref("wrong-incarnation");
    invalid_retirements.push(("incarnation reference", changed));
    let mut changed = retirement_preimage.clone();
    changed.incarnation_sha256 =
        ProducerIncarnationDigest::parse(sha('0')).expect("incarnation digest");
    invalid_retirements.push(("incarnation digest", changed));
    let mut changed = retirement_preimage.clone();
    changed.origin_principal_id =
        ResidentPrincipalIdV1::parse("principal-2").expect("other principal");
    changed.issuer_principal_id = changed.origin_principal_id.clone();
    invalid_retirements.push(("principal identity", changed));
    let mut changed = retirement_preimage.clone();
    changed.producer_id = ProducerIdV1::parse("producer-2").expect("other producer");
    invalid_retirements.push(("producer identity", changed));
    let mut changed = retirement_preimage.clone();
    changed.producer_epoch = ProducerEpochV1::parse("c".repeat(32)).expect("other epoch");
    invalid_retirements.push(("producer epoch", changed));
    let mut changed = retirement_preimage.clone();
    changed.retired_at = UnixTimeV1::new(9).expect("valid fixture time");
    invalid_retirements.push(("monotone retirement time", changed));
    for (law, candidate) in invalid_retirements {
        let sealed = ProducerIncarnationRetirementV1::seal(candidate, signature())
            .expect("structurally valid negative retirement");
        assert!(
            ProducerIncarnationRetirementV1::admit_contextual_wire(
                wire_from(&sealed),
                &active_head,
                &schema_digest('4'),
                &authority,
                fixture_namespace_coverage,
                |_, _, _, _, _| Ok(()),
            )
            .is_err(),
            "retirement must preserve {law}"
        );
    }
    let wrong_retirement_authority_ref = record_ref("producer-authority-2");
    let wrong_retirement_authority =
        producer_authority_context(&wrong_retirement_authority_ref, &authority_record);
    assert!(
        ProducerIncarnationRetirementV1::admit_contextual_wire(
            wire_from(&retirement),
            &active_head,
            &retirement_preimage.schema_sha256,
            &wrong_retirement_authority,
            fixture_namespace_coverage,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "retirement authority must resolve exactly"
    );
    assert!(
        ProducerIncarnationRetirementV1::admit_contextual_wire(
            wire_from(&retirement),
            &active_head,
            &retirement_preimage.schema_sha256,
            &authority,
            fixture_namespace_coverage,
            |_, _, _, _, _| Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "test signature",
                invariant: "authenticated",
            }),
        )
        .is_err(),
        "retirement signature rejection must fail admission"
    );
    let retired_head = ProducerIncarnationHeadAdmissionStateV1::Retired {
        incarnation_ref: &predecessor_ref,
        incarnation: &predecessor,
        retirement: &retirement,
    };

    let mut wrong_retirement_ref_preimage = retirement_preimage.clone();
    wrong_retirement_ref_preimage.incarnation_ref = record_ref("wrong-incarnation");
    let wrong_retirement_ref =
        ProducerIncarnationRetirementV1::seal(wrong_retirement_ref_preimage, signature())
            .expect("structurally valid mismatched retirement reference");
    let mismatched_retirement_ref_head = ProducerIncarnationHeadAdmissionStateV1::Retired {
        incarnation_ref: &predecessor_ref,
        incarnation: &predecessor,
        retirement: &wrong_retirement_ref,
    };
    assert!(
        ProducerIncarnationV1::admit_successor_wire(
            producer_wire(successor.clone()),
            &mismatched_retirement_ref_head,
            &successor.schema_sha256,
            &authority,
            |_, _| Ok(()),
            fixture_namespace_coverage,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "successor requires retirement of the exact predecessor reference"
    );

    let mut wrong_retirement_digest_preimage = retirement_preimage.clone();
    wrong_retirement_digest_preimage.incarnation_sha256 =
        ProducerIncarnationDigest::parse(sha('0')).expect("incarnation digest");
    let wrong_retirement_digest =
        ProducerIncarnationRetirementV1::seal(wrong_retirement_digest_preimage, signature())
            .expect("structurally valid mismatched retirement digest");
    let mismatched_retirement_digest_head = ProducerIncarnationHeadAdmissionStateV1::Retired {
        incarnation_ref: &predecessor_ref,
        incarnation: &predecessor,
        retirement: &wrong_retirement_digest,
    };
    assert!(
        ProducerIncarnationV1::admit_successor_wire(
            producer_wire(successor.clone()),
            &mismatched_retirement_digest_head,
            &successor.schema_sha256,
            &authority,
            |_, _| Ok(()),
            fixture_namespace_coverage,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "successor requires retirement of the exact predecessor digest"
    );

    ProducerIncarnationV1::admit_successor_wire(
        producer_wire(successor.clone()),
        &retired_head,
        &successor.schema_sha256,
        &authority,
        |_, _| Ok(()),
        fixture_namespace_coverage,
        |domain, public_key, _, _, _| {
            assert_eq!(domain, PRODUCER_INCARNATION_V1_SIGNATURE_DOMAIN);
            assert_eq!(public_key, authority_record.subject_public_key());
            Ok(())
        },
    )
    .expect("a fresh successor admits after the exact retirement");
    assert_eq!(
        retirement.producer_epoch(),
        predecessor.producer_epoch(),
        "retirement preserves the epoch needed to drain immutable old rows"
    );

    let mut invalid_successors = Vec::new();
    let mut changed = successor.clone();
    changed.previous_incarnation_ref = Some(record_ref("wrong-incarnation"));
    invalid_successors.push(("predecessor reference", changed));
    let mut changed = successor.clone();
    changed.previous_incarnation_sha256 =
        Some(ProducerIncarnationDigest::parse(sha('0')).expect("incarnation digest"));
    invalid_successors.push(("predecessor digest", changed));
    let mut changed = successor.clone();
    changed.origin_principal_id =
        ResidentPrincipalIdV1::parse("principal-2").expect("other principal");
    changed.issuer_principal_id = changed.origin_principal_id.clone();
    invalid_successors.push(("principal identity", changed));
    let mut changed = successor.clone();
    changed.producer_id = ProducerIdV1::parse("producer-2").expect("other producer");
    invalid_successors.push(("producer identity", changed));
    let mut changed = successor.clone();
    changed.producer_epoch = predecessor.producer_epoch().clone();
    invalid_successors.push(("distinct epoch", changed));
    let mut changed = successor.clone();
    changed.allocated_at = UnixTimeV1::new(11).expect("valid fixture time");
    invalid_successors.push(("chronology after retirement", changed));
    for (law, candidate) in invalid_successors {
        assert!(
            ProducerIncarnationV1::admit_successor_wire(
                producer_wire(candidate),
                &retired_head,
                &schema_digest('1'),
                &authority,
                |_, _| Ok(()),
                fixture_namespace_coverage,
                |_, _, _, _, _| Ok(()),
            )
            .is_err(),
            "successor must preserve {law}"
        );
    }
    assert!(
        ProducerIncarnationV1::admit_successor_wire(
            producer_wire(successor.clone()),
            &retired_head,
            &successor.schema_sha256,
            &authority,
            |_, _| Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "producer epoch",
                invariant: "globally unallocated",
            }),
            fixture_namespace_coverage,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "owner uniqueness rejection must fail admission"
    );
    assert!(
        ProducerIncarnationV1::admit_successor_wire(
            producer_wire(successor.clone()),
            &retired_head,
            &successor.schema_sha256,
            &authority,
            |_, _| Ok(()),
            fixture_namespace_coverage,
            |_, _, _, _, _| Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "test signature",
                invariant: "authenticated",
            }),
        )
        .is_err(),
        "successor signature rejection must fail admission"
    );
    assert!(
        ProducerIncarnationRetirementV1::admit_contextual_wire(
            wire_from(
                &ProducerIncarnationRetirementV1::seal(retirement_preimage, signature())
                    .expect("retirement")
            ),
            &retired_head,
            &schema_digest('4'),
            &authority,
            fixture_namespace_coverage,
            |_, _, _, _, _| Ok(()),
        )
        .is_err(),
        "the append-only retirement is admitted at most once"
    );
}

#[test]
fn producer_initial_rejects_an_existing_allocation_head() {
    let authority_ref = record_ref("producer-authority-1");
    let authority_record = producer_authority_record(producer_authority_preimage());
    let authority = producer_authority_context(&authority_ref, &authority_record);
    let incarnation_ref = record_ref("incarnation-a");
    let incarnation = producer(producer_preimage('a'));
    let active = ProducerIncarnationHeadAdmissionStateV1::Active {
        incarnation_ref: &incarnation_ref,
        incarnation: &incarnation,
    };
    let candidate = producer_preimage('b');
    assert!(ProducerIncarnationV1::admit_initial_wire(
        producer_wire(candidate.clone()),
        &active,
        &candidate.schema_sha256,
        &authority,
        |_, _| Ok(()),
        fixture_namespace_coverage,
        |_, _, _, _, _| Ok(()),
    )
    .is_err());
}
