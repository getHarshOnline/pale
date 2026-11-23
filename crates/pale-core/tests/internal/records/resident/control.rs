//! Exercises signed control checkpoints, continuity, bounds, and checked wire admission.

use super::*;

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn artifact(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(sha(byte)).expect("artifact digest")
}

fn checkpoint_profile_digest(byte: char) -> CheckpointProfileDigest {
    CheckpointProfileDigest::parse(sha(byte)).expect("checkpoint profile digest")
}

fn checkpoint_authority_digest(byte: char) -> CheckpointSigningAuthorityDigest {
    CheckpointSigningAuthorityDigest::parse(sha(byte)).expect("checkpoint authority digest")
}

fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("record ref")
}

fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("signature")
}

fn signing_authority(
    expected: &ControlStateCheckpointPreimageV1,
) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
    ControlCheckpointSigningAuthorityAdmissionContextV1 {
        authority_ref: &expected.checkpoint_signing_authority_ref,
        authority_sha256: &expected.checkpoint_signing_authority_sha256,
        principal_id: &expected.issuer_principal_id,
        control_store_id: &expected.control_store_id,
        control_incarnation_id: &expected.control_incarnation_id,
        checkpoint_domain_id: &expected.checkpoint_domain_id,
        checkpoint_profile_ref: &expected.checkpoint_profile_ref,
        checkpoint_profile_sha256: &expected.checkpoint_profile_sha256,
        subject_key_id: &expected.issuer_key_id,
        subject_key_epoch: expected.issuer_key_epoch,
    }
}

fn preimage(sequence: u64) -> ControlStateCheckpointPreimageV1 {
    ControlStateCheckpointPreimageV1 {
        schema_id: SchemaId::parse("pale/control-state-checkpoint.v1").expect("checkpoint schema"),
        schema_sha256: SchemaDigest::parse(sha('a')).expect("schema digest"),
        control_checkpoint_id: ControlCheckpointIdV1::parse(format!("checkpoint-{sequence}"))
            .expect("checkpoint id"),
        control_store_id: ControlStoreIdV1::parse("store-1").expect("store id"),
        control_incarnation_id: ControlIncarnationIdV1::parse("incarnation-1")
            .expect("incarnation id"),
        checkpoint_domain_id: ControlCheckpointDomainIdV1::parse("effects")
            .expect("checkpoint domain"),
        domain_epoch: ControlDomainEpochV1::new(1).expect("valid fixture domain epoch"),
        domain_sequence: ControlDomainSequenceV1::new(sequence)
            .expect("valid fixture domain sequence"),
        previous_control_checkpoint_ref: None,
        previous_control_checkpoint_sha256: None,
        checkpoint_profile_ref: record_ref("checkpoint-profile"),
        checkpoint_profile_sha256: checkpoint_profile_digest('b'),
        control_schema_version: 1,
        committed_transaction_sequence: ControlTransactionSequenceV1::new(10 + sequence)
            .expect("valid fixture transaction sequence"),
        state_root_sha256: artifact('c'),
        leaf_count: ControlLeafCountV1::new(1).expect("valid fixture leaf count"),
        changed_leaf_keys: vec![ChangedControlLeafKeyV1 {
            record_class: ControlRecordClassV1::parse("effect-claim").expect("record class"),
            scope_key_sha256: artifact('d'),
            logical_key: ControlLogicalKeyV1::parse("claim-1").expect("logical key"),
        }],
        unresolved_custody_root_sha256: artifact('e'),
        issued_at: UnixTimeV1::new(10 + sequence).expect("valid fixture time"),
        checkpoint_signing_authority_ref: record_ref("checkpoint-authority"),
        checkpoint_signing_authority_sha256: checkpoint_authority_digest('f'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    }
}

fn wire(preimage: ControlStateCheckpointPreimageV1) -> ControlStateCheckpointWireV1 {
    let value = ControlStateCheckpointV1::seal(preimage, signature())
        .expect("structurally valid checkpoint");
    serde_json::from_value(serde_json::to_value(value).expect("checkpoint serializes"))
        .expect("checkpoint wire decodes")
}

#[test]
fn signed_checkpoint_admission_does_not_assert_succession_or_anchor() {
    let mut value = preimage(7);
    value.previous_control_checkpoint_ref = Some(record_ref("unresolved-six"));
    value.previous_control_checkpoint_sha256 =
        Some(ControlStateCheckpointDigest::parse(sha('6')).unwrap());
    let calls = std::cell::Cell::new(0);
    let admitted = ControlStateCheckpointV1::admit_signed_wire(
        wire(value.clone()),
        &value.schema_sha256,
        &signing_authority(&value),
        |domain, actual, digest, actual_signature| {
            calls.set(calls.get() + 1);
            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &value);
            assert_eq!(
                digest,
                &ControlStateCheckpointDigest::hash_canonical(actual).unwrap()
            );
            assert_eq!(actual_signature, &signature());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(admitted.domain_sequence().get(), 7);
    assert!(ControlStateCheckpointV1::admit_initial_wire(
        wire(value.clone()),
        &value.schema_sha256,
        &signing_authority(&value),
        |_, _, _, _| panic!("transition rejection must precede signature verification"),
    )
    .is_err());
    let predecessor = ControlStateCheckpointV1::seal(preimage(1), signature()).unwrap();
    assert!(ControlStateCheckpointV1::admit_successor_wire(
        wire(value.clone()),
        &record_ref("one"),
        &predecessor,
        &value.schema_sha256,
        &signing_authority(&value),
        |_, _, _, _| panic!("unproved predecessor relation must still reject"),
    )
    .is_err());
    assert!(ControlStateCheckpointV1::admit_signed_wire(
        wire(value.clone()),
        &SchemaDigest::parse(sha('9')).unwrap(),
        &signing_authority(&value),
        |_, _, _, _| panic!("schema rejection precedes signature verification"),
    )
    .is_err());
    let other_principal = ResidentPrincipalIdV1::parse("other-principal").unwrap();
    let mut authority = signing_authority(&value);
    authority.principal_id = &other_principal;
    assert!(ControlStateCheckpointV1::admit_signed_wire(
        wire(value.clone()),
        &value.schema_sha256,
        &authority,
        |_, _, _, _| panic!("signer coordinate rejection precedes verification"),
    )
    .is_err());
    assert!(ControlStateCheckpointV1::admit_signed_wire(
        wire(value.clone()),
        &value.schema_sha256,
        &signing_authority(&value),
        |_, _, _, _| Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "checkpoint signature test",
            invariant: "signature verifier rejected"
        }),
    )
    .is_err());
}

#[test]
fn checkpoint_successor_rejects_every_immutable_substitution_and_regression() {
    let predecessor =
        ControlStateCheckpointV1::seal(preimage(1), signature()).expect("initial checkpoint");
    let predecessor_ref = record_ref("checkpoint-1");
    let mut successor = preimage(2);
    successor.previous_control_checkpoint_ref = Some(predecessor_ref.clone());
    successor.previous_control_checkpoint_sha256 =
        Some(predecessor.canonical_control_checkpoint_sha256().clone());
    let expected = successor.clone();
    let signing_authority = signing_authority(&expected);

    ControlStateCheckpointV1::admit_successor_wire(
        wire(successor.clone()),
        &predecessor_ref,
        &predecessor,
        predecessor.schema_sha256(),
        &signing_authority,
        |domain, _, _, _| {
            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
            Ok(())
        },
    )
    .expect("exact checkpoint successor admits");

    let mut cases = Vec::new();
    let mut changed = successor.clone();
    changed.schema_sha256 = SchemaDigest::parse(sha('0')).expect("schema digest");
    cases.push(("schema digest", changed));
    let mut changed = successor.clone();
    changed.domain_sequence =
        ControlDomainSequenceV1::new(3).expect("valid changed domain sequence");
    cases.push(("sequence jump", changed));
    let mut changed = successor.clone();
    changed.previous_control_checkpoint_ref = Some(record_ref("other-checkpoint"));
    cases.push(("predecessor ref", changed));
    let mut changed = successor.clone();
    changed.previous_control_checkpoint_sha256 =
        Some(ControlStateCheckpointDigest::parse(sha('0')).expect("checkpoint digest"));
    cases.push(("predecessor digest", changed));
    let mut changed = successor.clone();
    changed.control_store_id = ControlStoreIdV1::parse("other-store").expect("store id");
    cases.push(("store id", changed));
    let mut changed = successor.clone();
    changed.control_incarnation_id =
        ControlIncarnationIdV1::parse("other-incarnation").expect("incarnation id");
    cases.push(("incarnation id", changed));
    let mut changed = successor.clone();
    changed.checkpoint_domain_id =
        ControlCheckpointDomainIdV1::parse("other-domain").expect("domain id");
    cases.push(("domain id", changed));
    let mut changed = successor.clone();
    changed.domain_epoch = ControlDomainEpochV1::new(2).expect("valid changed domain epoch");
    cases.push(("domain epoch", changed));
    let mut changed = successor.clone();
    changed.checkpoint_profile_ref = record_ref("other-profile");
    cases.push(("profile ref", changed));
    let mut changed = successor.clone();
    changed.checkpoint_profile_sha256 = checkpoint_profile_digest('0');
    cases.push(("profile digest", changed));
    let mut changed = successor.clone();
    changed.control_schema_version = 2;
    cases.push(("control schema version", changed));
    let mut changed = successor.clone();
    changed.committed_transaction_sequence =
        ControlTransactionSequenceV1::new(10).expect("valid changed transaction sequence");
    cases.push(("transaction regression", changed));
    let mut changed = successor.clone();
    changed.issued_at = UnixTimeV1::new(10).expect("valid fixture time");
    cases.push(("time regression", changed));

    for (name, candidate) in cases {
        assert!(
            ControlStateCheckpointV1::issue_successor_contextual(
                candidate.clone(),
                &predecessor_ref,
                &predecessor,
                predecessor.schema_sha256(),
                &signing_authority,
                |_, _, _| panic!("{name} must fail before signing"),
            )
            .is_err(),
            "{name} substitution must fail issuance"
        );
        let admitted = ControlStateCheckpointV1::seal(candidate.clone(), signature())
            .expect("structurally valid successor candidate");
        assert!(
            admitted
                .verify_successor_of(&predecessor_ref, &predecessor)
                .is_err(),
            "{name} substitution must fail admitted-record recheck"
        );
        assert!(
            ControlStateCheckpointV1::admit_successor_wire(
                wire(candidate),
                &predecessor_ref,
                &predecessor,
                predecessor.schema_sha256(),
                &signing_authority,
                |_, _, _, _| Ok(()),
            )
            .is_err(),
            "{name} substitution must fail"
        );
    }
}

#[test]
fn checkpoint_admission_binds_every_resolved_signer_coordinate() {
    let expected = preimage(1);
    let signing_authority = signing_authority(&expected);

    ControlStateCheckpointV1::admit_initial_wire(
        wire(expected.clone()),
        &expected.schema_sha256,
        &signing_authority,
        |domain, _, _, _| {
            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
            Ok(())
        },
    )
    .expect("exact resolved checkpoint signer admits");

    let mut cases = Vec::new();
    let mut changed = expected.clone();
    changed.schema_sha256 = SchemaDigest::parse(sha('0')).expect("schema digest");
    cases.push(("schema digest", changed));
    let mut changed = expected.clone();
    changed.checkpoint_signing_authority_ref = record_ref("other-authority");
    cases.push(("authority ref", changed));
    let mut changed = expected.clone();
    changed.checkpoint_signing_authority_sha256 = checkpoint_authority_digest('0');
    cases.push(("authority digest", changed));
    let mut changed = expected.clone();
    changed.issuer_principal_id = ResidentPrincipalIdV1::parse("principal-2").expect("principal");
    cases.push(("principal", changed));
    let mut changed = expected.clone();
    changed.control_store_id = ControlStoreIdV1::parse("store-2").expect("store id");
    cases.push(("store", changed));
    let mut changed = expected.clone();
    changed.control_incarnation_id =
        ControlIncarnationIdV1::parse("incarnation-2").expect("incarnation id");
    cases.push(("incarnation", changed));
    let mut changed = expected.clone();
    changed.checkpoint_domain_id =
        ControlCheckpointDomainIdV1::parse("other-domain").expect("domain id");
    cases.push(("domain", changed));
    let mut changed = expected.clone();
    changed.checkpoint_profile_ref = record_ref("other-profile");
    cases.push(("profile ref", changed));
    let mut changed = expected.clone();
    changed.checkpoint_profile_sha256 = checkpoint_profile_digest('0');
    cases.push(("profile digest", changed));
    let mut changed = expected.clone();
    changed.issuer_key_id = ResidentKeyIdV1::parse("key-2").expect("issuer key");
    cases.push(("subject key", changed));
    let mut changed = expected.clone();
    changed.issuer_key_epoch = SigningKeyEpochV1::new(2).expect("valid changed signing key epoch");
    cases.push(("subject key epoch", changed));

    for (name, candidate) in cases {
        assert!(
            ControlStateCheckpointV1::issue_initial_contextual(
                candidate.clone(),
                &expected.schema_sha256,
                &signing_authority,
                |_, _, _| panic!("{name} must fail before signing"),
            )
            .is_err(),
            "{name} substitution must fail issuance"
        );
        assert!(
            ControlStateCheckpointV1::admit_initial_wire(
                wire(candidate),
                &expected.schema_sha256,
                &signing_authority,
                |_, _, _, _| Ok(()),
            )
            .is_err(),
            "{name} substitution must fail"
        );
    }
}

#[test]
fn zero_leaf_checkpoint_admission_requires_an_empty_changed_set() {
    let mut empty = preimage(1);
    empty.leaf_count = ControlLeafCountV1::new(0).expect("zero is representable");
    empty.changed_leaf_keys.clear();
    let checkpoint = ControlStateCheckpointV1::admit_initial_wire(
        wire(empty.clone()),
        &empty.schema_sha256,
        &signing_authority(&empty),
        |_, _, _, _| Ok(()),
    )
    .expect("empty first-domain representation admits");
    assert_eq!(checkpoint.leaf_count().get(), 0);
    assert!(checkpoint.changed_leaf_keys().is_empty());
    assert!(checkpoint.previous_control_checkpoint_ref().is_none());
    // This characterizes the record contract, not the contents of an actual
    // bootstrap profile or the correctness of its accumulator root.
    empty.changed_leaf_keys = preimage(1).changed_leaf_keys;
    assert!(matches!(
        ControlStateCheckpointV1::seal(empty, signature()),
        Err(InvalidResidentRecordV1::InvalidCoordinate {
            coordinate: "control checkpoint frontier"
        })
    ));
}

#[test]
fn changed_key_bound_uses_successor_leaf_count_without_selecting_a_removal_policy() {
    let predecessor =
        ControlStateCheckpointV1::seal(preimage(1), signature()).expect("one-leaf predecessor");
    let mut successor = preimage(2);
    successor.previous_control_checkpoint_ref = Some(record_ref("checkpoint-1"));
    successor.previous_control_checkpoint_sha256 =
        Some(predecessor.canonical_control_checkpoint_sha256().clone());
    // Reporting a changed key with zero successor leaves is not representable
    // today. This does not assert that physical removal is an adopted policy.
    successor.leaf_count = ControlLeafCountV1::new(0).expect("zero leaf count");
    assert!(matches!(
        ControlStateCheckpointV1::seal(successor.clone(), signature()),
        Err(InvalidResidentRecordV1::InvalidCoordinate {
            coordinate: "control checkpoint frontier"
        })
    ));
    successor.leaf_count = ControlLeafCountV1::new(1).expect("one leaf");
    let admitted = ControlStateCheckpointV1::seal(successor, signature())
        .expect("one changed key fits one successor leaf");
    admitted
        .verify_successor_of(&record_ref("checkpoint-1"), &predecessor)
        .expect("the boundary control preserves exact succession");
}

#[test]
fn issued_checkpoints_sign_exact_preimages_and_readmit_without_wire_changes() {
    let predecessor =
        ControlStateCheckpointV1::seal(preimage(1), signature()).expect("predecessor");
    let predecessor_ref = record_ref("checkpoint-1");

    for sequence in [1, 2] {
        let mut expected = preimage(sequence);
        if sequence == 2 {
            expected.previous_control_checkpoint_ref = Some(predecessor_ref.clone());
            expected.previous_control_checkpoint_sha256 =
                Some(predecessor.canonical_control_checkpoint_sha256().clone());
            // Equality is allowed: sequence advances even when these coordinates do not.
            expected.committed_transaction_sequence = *predecessor.committed_transaction_sequence();
            expected.issued_at = *predecessor.issued_at();
        }
        let authority = signing_authority(&expected);
        let expected_digest =
            ControlStateCheckpointDigest::hash_canonical(&expected).expect("canonical digest");
        let calls = std::cell::Cell::new(0);
        let sign = |domain: &[u8],
                    signed: &ControlStateCheckpointPreimageV1,
                    digest: &ControlStateCheckpointDigest| {
            calls.set(calls.get() + 1);
            assert_eq!(domain, b"PALE_CONTROL_STATE_CHECKPOINT_V1\0");
            assert_eq!(signed, &expected);
            assert_eq!(digest, &expected_digest);
            Ok(signature())
        };
        let issued = if sequence == 1 {
            ControlStateCheckpointV1::issue_initial_contextual(
                expected.clone(),
                &expected.schema_sha256,
                &authority,
                sign,
            )
        } else {
            ControlStateCheckpointV1::issue_successor_contextual(
                expected.clone(),
                &predecessor_ref,
                &predecessor,
                &expected.schema_sha256,
                &authority,
                sign,
            )
        }
        .expect("checkpoint issues");
        assert_eq!(calls.get(), 1);
        issued.verify().expect("issued self-digest verifies");
        let sealed = ControlStateCheckpointV1::seal(expected.clone(), signature())
            .expect("existing sealing path");
        let bytes = serde_json::to_vec(&issued).expect("issued wire bytes");
        assert_eq!(
            bytes,
            serde_json::to_vec(&sealed).expect("sealed wire bytes")
        );
        let decoded = serde_json::from_slice(&bytes).expect("wire decodes");
        let verify = |domain: &[u8],
                      admitted: &ControlStateCheckpointPreimageV1,
                      digest: &ControlStateCheckpointDigest,
                      admitted_signature: &CanonicalSignatureV1| {
            assert_eq!(domain, b"PALE_CONTROL_STATE_CHECKPOINT_V1\0");
            assert_eq!(admitted, &expected);
            assert_eq!(digest, &expected_digest);
            assert_eq!(admitted_signature, &signature());
            Ok(())
        };
        let admitted = if sequence == 1 {
            ControlStateCheckpointV1::admit_initial_wire(
                decoded,
                &expected.schema_sha256,
                &authority,
                verify,
            )
        } else {
            ControlStateCheckpointV1::admit_successor_wire(
                decoded,
                &predecessor_ref,
                &predecessor,
                &expected.schema_sha256,
                &authority,
                verify,
            )
        }
        .expect("issued record contextually readmits");
        assert_eq!(admitted, issued);
    }
}

#[test]
fn malformed_checkpoint_preimages_never_reach_either_signer() {
    let expected = preimage(1);
    let authority = signing_authority(&expected);
    let predecessor =
        ControlStateCheckpointV1::seal(expected.clone(), signature()).expect("predecessor");
    let predecessor_ref = record_ref("checkpoint-1");
    let mut cases = Vec::new();
    let mut changed = expected.clone();
    changed.schema_id = SchemaId::parse("pale/other.v1").expect("schema id");
    cases.push(("schema identity", changed));
    let mut changed = expected.clone();
    changed.control_schema_version = 0;
    cases.push(("zero control schema", changed));
    let mut changed = expected.clone();
    changed.previous_control_checkpoint_ref = Some(predecessor_ref.clone());
    cases.push(("unpaired predecessor ref", changed));
    let mut changed = expected.clone();
    changed.previous_control_checkpoint_sha256 =
        Some(predecessor.canonical_control_checkpoint_sha256().clone());
    cases.push(("unpaired predecessor digest", changed));
    let mut changed = expected.clone();
    changed.previous_control_checkpoint_ref = Some(predecessor_ref.clone());
    changed.previous_control_checkpoint_sha256 =
        Some(predecessor.canonical_control_checkpoint_sha256().clone());
    cases.push(("initial sequence with predecessor", changed));
    cases.push(("successor without predecessor", preimage(2)));
    let mut changed = expected.clone();
    changed.leaf_count = ControlLeafCountV1::new(0).expect("zero leaves");
    cases.push(("changed key exceeds leaf count", changed));
    let mut changed = expected.clone();
    changed.leaf_count = ControlLeafCountV1::new(2).expect("two leaves");
    changed
        .changed_leaf_keys
        .push(changed.changed_leaf_keys[0].clone());
    cases.push(("duplicate changed key", changed.clone()));
    changed.changed_leaf_keys[0].logical_key =
        ControlLogicalKeyV1::parse("claim-2").expect("logical key");
    cases.push(("unsorted changed keys", changed));

    for (name, candidate) in cases {
        assert!(ControlStateCheckpointV1::seal(candidate.clone(), signature()).is_err());
        assert!(
            ControlStateCheckpointV1::issue_initial_contextual(
                candidate.clone(),
                &expected.schema_sha256,
                &authority,
                |_, _, _| panic!("{name} must fail before initial signing"),
            )
            .is_err(),
            "{name}"
        );
        assert!(
            ControlStateCheckpointV1::issue_successor_contextual(
                candidate,
                &predecessor_ref,
                &predecessor,
                &expected.schema_sha256,
                &authority,
                |_, _, _| panic!("{name} must fail before successor signing"),
            )
            .is_err(),
            "{name}"
        );
    }
}

#[test]
fn checkpoint_issuance_distinguishes_initial_successor_and_sequence_exhaustion() {
    let initial = preimage(1);
    let authority = signing_authority(&initial);
    let predecessor =
        ControlStateCheckpointV1::seal(initial.clone(), signature()).expect("predecessor");
    let predecessor_ref = record_ref("checkpoint-1");
    let mut successor = preimage(2);
    successor.previous_control_checkpoint_ref = Some(predecessor_ref.clone());
    successor.previous_control_checkpoint_sha256 =
        Some(predecessor.canonical_control_checkpoint_sha256().clone());
    assert!(ControlStateCheckpointV1::issue_initial_contextual(
        successor.clone(),
        &initial.schema_sha256,
        &authority,
        |_, _, _| panic!("a successor cannot issue as an initial checkpoint"),
    )
    .is_err());
    assert!(ControlStateCheckpointV1::issue_successor_contextual(
        initial.clone(),
        &predecessor_ref,
        &predecessor,
        &initial.schema_sha256,
        &authority,
        |_, _, _| panic!("an initial checkpoint cannot issue as a successor"),
    )
    .is_err());

    let mut corrupt_predecessor = predecessor.clone();
    corrupt_predecessor.canonical_control_checkpoint_sha256 =
        ControlStateCheckpointDigest::parse(sha('0')).expect("different digest");
    let mut follows_corrupt = successor.clone();
    follows_corrupt.previous_control_checkpoint_sha256 = Some(
        corrupt_predecessor
            .canonical_control_checkpoint_sha256()
            .clone(),
    );
    assert!(matches!(
        ControlStateCheckpointV1::issue_successor_contextual(
            follows_corrupt,
            &predecessor_ref,
            &corrupt_predecessor,
            &initial.schema_sha256,
            &authority,
            |_, _, _| panic!("an invalid predecessor cannot reach signing"),
        ),
        Err(InvalidResidentRecordV1::DigestMismatch {
            family: "control state checkpoint",
        })
    ));

    let mut exhausted = successor.clone();
    exhausted.domain_sequence =
        ControlDomainSequenceV1::new(crate::canon::JSON_SAFE_INTEGER_MAX).expect("last sequence");
    let exhausted =
        ControlStateCheckpointV1::seal(exhausted, signature()).expect("last checkpoint");
    successor.previous_control_checkpoint_sha256 =
        Some(exhausted.canonical_control_checkpoint_sha256().clone());
    assert!(ControlStateCheckpointV1::issue_successor_contextual(
        successor,
        &predecessor_ref,
        &exhausted,
        &initial.schema_sha256,
        &authority,
        |_, _, _| panic!("an exhausted sequence cannot reach signing"),
    )
    .is_err());
}

#[test]
fn checkpoint_signing_failure_is_returned_unchanged() {
    let predecessor =
        ControlStateCheckpointV1::seal(preimage(1), signature()).expect("predecessor");
    let predecessor_ref = record_ref("checkpoint-1");
    for sequence in [1, 2] {
        let mut candidate = preimage(sequence);
        if sequence == 2 {
            candidate.previous_control_checkpoint_ref = Some(predecessor_ref.clone());
            candidate.previous_control_checkpoint_sha256 =
                Some(predecessor.canonical_control_checkpoint_sha256().clone());
        }
        let authority = signing_authority(&candidate);
        let calls = std::cell::Cell::new(0);
        let sign =
            |_: &[u8], _: &ControlStateCheckpointPreimageV1, _: &ControlStateCheckpointDigest| {
                calls.set(calls.get() + 1);
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "test signer",
                    invariant: "signing unavailable",
                })
            };
        let result = if sequence == 1 {
            ControlStateCheckpointV1::issue_initial_contextual(
                candidate.clone(),
                &candidate.schema_sha256,
                &authority,
                sign,
            )
        } else {
            ControlStateCheckpointV1::issue_successor_contextual(
                candidate.clone(),
                &predecessor_ref,
                &predecessor,
                &candidate.schema_sha256,
                &authority,
                sign,
            )
        };
        assert_eq!(calls.get(), 1);
        assert!(matches!(
            result,
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "test signer",
                invariant: "signing unavailable",
            })
        ));
    }
}
