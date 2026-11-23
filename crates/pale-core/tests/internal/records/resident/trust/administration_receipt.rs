//! Administration outcome structure is not signing or transaction evidence.

use super::*;
use std::cell::Cell;

fn receipt() -> LocalAdminReceiptPreimageV1 {
    LocalAdminReceiptPreimageV1 {
        schema_id: SchemaId::parse(LOCAL_ADMIN_RECEIPT_V1_SCHEMA).unwrap(),
        schema_sha256: schema_digest('a'),
        admin_receipt_id: LocalAdminReceiptIdV1::parse("receipt-1").unwrap(),
        admin_command_ref: record_ref("command-1"),
        admin_command_sha256: LocalAdminCommandDigest::parse(sha('b')).unwrap(),
        disposition: LocalAdminDispositionV1::Committed,
        committed_records: vec![LocalAdminCommittedRecordV1 {
            committed_record_ref: record_ref("checkpoint-2"),
            committed_record_sha256: artifact('c'),
        }],
        head_transitions: vec![LocalAdminHeadTransitionV1 {
            head_kind: LocalAdminHeadSelectorV1::ControlCheckpoint {
                control_store_id: ControlStoreIdV1::parse("principal-control").unwrap(),
                control_incarnation_id: ControlIncarnationIdV1::parse("incarnation-1").unwrap(),
                checkpoint_domain_id: ControlCheckpointDomainIdV1::parse("principal-trust")
                    .unwrap(),
            },
            previous_head_ref: Some(record_ref("checkpoint-1")),
            previous_head_sha256: Some(artifact('d')),
            new_head_ref: Some(record_ref("checkpoint-2")),
            new_head_sha256: Some(artifact('c')),
        }],
        failure_code: None,
        committed_at: Some(UnixTimeV1::new(20).unwrap()),
        recorded_at: UnixTimeV1::new(21).unwrap(),
        deployment_runtime_authority_ref: record_ref("runtime-authority-1"),
        deployment_runtime_authority_sha256: DeploymentRuntimeAuthorityDigest::parse(sha('e'))
            .unwrap(),
        issuer_principal_id: principal(),
        issuer_key_id: issuer_key(),
        issuer_key_epoch: authority_key_epoch(),
    }
}

fn failed(disposition: LocalAdminDispositionV1) -> LocalAdminReceiptPreimageV1 {
    LocalAdminReceiptPreimageV1 {
        disposition,
        committed_records: vec![],
        head_transitions: vec![],
        failure_code: Some(LocalAdminFailureCodeV1::parse("expected_head_changed").unwrap()),
        committed_at: None,
        ..receipt()
    }
}

fn wire(value: &LocalAdminReceiptPreimageV1) -> LocalAdminReceiptWireV1 {
    // Deliberately build the unsigned wire without sealing: invalid combinations
    // must reach the same structure verifier as externally supplied bytes.
    let mut wire = serde_json::to_value(value).unwrap();
    wire["canonical_admin_receipt_sha256"] =
        serde_json::to_value(LocalAdminReceiptDigest::hash_canonical(value).unwrap()).unwrap();
    wire["signature"] = serde_json::to_value(signature()).unwrap();
    serde_json::from_value(wire).unwrap()
}

fn command() -> LocalAdminCommandV1 {
    let candidate_records = vec![LocalAdminCandidateRecordV1 {
        candidate_record_schema_id: SchemaId::parse(DEPLOYMENT_BINDING_V1_SCHEMA).unwrap(),
        candidate_record_ref: record_ref("binding-1"),
        candidate_record_sha256: artifact('7'),
    }];
    LocalAdminCommandV1::seal(
        LocalAdminCommandPreimageV1 {
            schema_id: SchemaId::parse(LOCAL_ADMIN_COMMAND_V1_SCHEMA).unwrap(),
            schema_sha256: schema_digest('6'),
            admin_command_id: LocalAdminCommandIdV1::parse("command-1").unwrap(),
            operation: LocalAdminOperationV1::FinalizeDeployment,
            admin_authority_ref: record_ref("admin-authority-1"),
            admin_authority_sha256: LocalAdminAuthorityDigest::parse(sha('5')).unwrap(),
            candidate_set_sha256: LocalAdminCandidateSetDigest::hash_canonical(&candidate_records)
                .unwrap(),
            candidate_records,
            expected_heads: vec![LocalAdminExpectedHeadV1 {
                head_kind: LocalAdminHeadSelectorV1::PrincipalTrustCheckpoint {
                    principal_id: principal(),
                    trust_store_id: ControlStoreIdV1::parse("trust-store-1").unwrap(),
                    trust_store_incarnation_id: ControlIncarnationIdV1::parse(
                        "trust-incarnation-1",
                    )
                    .unwrap(),
                },
                expected_head_ref: Some(record_ref("checkpoint-1")),
                expected_head_sha256: Some(artifact('d')),
                expected_head_epoch: Some(LocalAdminHeadEpochV1::new(1).unwrap()),
            }],
            idempotency_key: LocalAdminIdempotencyKeyV1::parse("once").unwrap(),
            issued_at: UnixTimeV1::new(12).unwrap(),
            expires_at: UnixTimeV1::new(25).unwrap(),
            signer_identity: LocalAdminSubjectIdentityV1::parse("operator").unwrap(),
            signer_key_id: ResidentKeyIdV1::parse("operator-key").unwrap(),
            signer_key_epoch: authority_key_epoch(),
        },
        signature(),
    )
    .unwrap()
}

fn runtime_authority_preimage() -> DeploymentRuntimeAuthorityPreimageV1 {
    DeploymentRuntimeAuthorityPreimageV1 {
        schema_id: SchemaId::parse(DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA).unwrap(),
        schema_sha256: schema_digest('4'),
        runtime_authority_id: DeploymentRuntimeAuthorityIdV1::parse("runtime-1").unwrap(),
        principal_id: principal(),
        deployment_binding_ref: record_ref("binding-1"),
        deployment_binding_sha256: DeploymentBindingDigest::parse(sha('7')).unwrap(),
        control_store_id: ControlStoreIdV1::parse("control-store-1").unwrap(),
        control_incarnation_id: ControlIncarnationIdV1::parse("incarnation-1").unwrap(),
        subject_key_id: issuer_key(),
        subject_key_epoch: authority_key_epoch(),
        subject_public_key: Ed25519PublicKeyV1::parse("A".repeat(43)).unwrap(),
        permitted_record_schema_ids: vec![SchemaId::parse(LOCAL_ADMIN_RECEIPT_V1_SCHEMA).unwrap()],
        authority_epoch: DeploymentRuntimeAuthorityEpochV1::new(1).unwrap(),
        previous_authority_ref: None,
        previous_authority_sha256: None,
        state: PrincipalAuthorityStateV1::Active,
        issued_at: UnixTimeV1::new(10).unwrap(),
        expires_at: UnixTimeV1::new(30).unwrap(),
        issuer_principal_id: principal(),
        issuer_key_id: ResidentKeyIdV1::parse("root-key").unwrap(),
        issuer_key_epoch: key_epoch(),
    }
}

fn runtime_authority() -> DeploymentRuntimeAuthorityV1 {
    DeploymentRuntimeAuthorityV1::seal(runtime_authority_preimage(), signature()).unwrap()
}

fn runtime_signing_authority(
    runtime_ref: &ResidentRecordRefV1,
    runtime: &DeploymentRuntimeAuthorityV1,
) -> SigningAuthorityBindingV1 {
    SigningAuthorityBindingV1::integrated_runtime(
        runtime.principal_id().clone(),
        runtime.subject_key_id().clone(),
        *runtime.subject_key_epoch(),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: runtime_ref.clone(),
            deployment_runtime_authority_sha256: ArtifactDigest::parse(
                runtime
                    .canonical_deployment_runtime_authority_sha256()
                    .as_str(),
            )
            .unwrap(),
            deployment_binding_ref: runtime.deployment_binding_ref().clone(),
            deployment_binding_sha256: ArtifactDigest::parse(
                runtime.deployment_binding_sha256().as_str(),
            )
            .unwrap(),
            checked_principal_trust_checkpoint_ref: record_ref("trust-checkpoint-2"),
            checked_principal_trust_checkpoint_sha256: artifact('8'),
        },
    )
    .unwrap()
}

fn contextual_receipt<'a>(
    preimage: &'a LocalAdminReceiptPreimageV1,
    command_ref: &'a ResidentRecordRefV1,
    command: &'a LocalAdminCommandV1,
    runtime: &'a DeploymentRuntimeAuthorityV1,
    signing: &'a SigningAuthorityBindingV1,
) -> LocalAdminReceiptAdmissionContextV1<'a> {
    LocalAdminReceiptAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            signing_authority: signing,
        },
        admin_command_ref: command_ref,
        admin_command: command,
        runtime_authority: runtime,
        expected_admin_receipt_id: &preimage.admin_receipt_id,
        expected_disposition: preimage.disposition,
        expected_committed_records: &preimage.committed_records,
        expected_head_transitions: &preimage.head_transitions,
        expected_failure_code: preimage.failure_code.as_ref(),
        expected_committed_at: preimage.committed_at.as_ref(),
        expected_recorded_at: &preimage.recorded_at,
    }
}

#[test]
fn contextual_issue_and_admission_bind_domain_runtime_command_and_exact_result() {
    let command = command();
    let command_ref = record_ref("command-1");
    let runtime = runtime_authority();
    let runtime_ref = record_ref("runtime-authority-1");
    let signing = runtime_signing_authority(&runtime_ref, &runtime);
    let schema = schema_digest('a');
    let receipt_id = LocalAdminReceiptIdV1::parse("receipt-1").unwrap();
    let committed_at = UnixTimeV1::new(20).unwrap();
    let recorded_at = UnixTimeV1::new(21).unwrap();
    let mut preimage = receipt();
    preimage.schema_sha256 = schema.clone();
    preimage.admin_command_ref = command_ref.clone();
    preimage.admin_command_sha256 = command.canonical_admin_command_sha256().clone();
    preimage.deployment_runtime_authority_ref = runtime_ref.clone();
    preimage.deployment_runtime_authority_sha256 = runtime
        .canonical_deployment_runtime_authority_sha256()
        .clone();
    preimage.issuer_principal_id = runtime.principal_id().clone();
    preimage.issuer_key_id = runtime.subject_key_id().clone();
    preimage.issuer_key_epoch = *runtime.subject_key_epoch();
    let context = LocalAdminReceiptAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &schema,
            signing_authority: &signing,
        },
        admin_command_ref: &command_ref,
        admin_command: &command,
        runtime_authority: &runtime,
        expected_admin_receipt_id: &receipt_id,
        expected_disposition: LocalAdminDispositionV1::Committed,
        expected_committed_records: &preimage.committed_records,
        expected_head_transitions: &preimage.head_transitions,
        expected_failure_code: None,
        expected_committed_at: Some(&committed_at),
        expected_recorded_at: &recorded_at,
    };
    let sign_calls = Cell::new(0);
    let issued = LocalAdminReceiptV1::issue_contextual(
        preimage.clone(),
        &context,
        |domain, value, digest| {
            sign_calls.set(sign_calls.get() + 1);
            assert_eq!(domain, LOCAL_ADMIN_RECEIPT_V1_SIGNATURE_DOMAIN);
            assert_eq!(value, &preimage);
            assert_eq!(
                *digest,
                LocalAdminReceiptDigest::hash_canonical(value).unwrap()
            );
            Ok(signature())
        },
    )
    .unwrap();
    assert_eq!(sign_calls.get(), 1);
    let verify_calls = Cell::new(0);
    let admitted = LocalAdminReceiptV1::admit_contextual_wire(
        wire_from(&issued),
        &context,
        |key, domain, value, digest, supplied_signature| {
            verify_calls.set(verify_calls.get() + 1);
            assert_eq!(key, runtime.subject_public_key());
            assert_eq!(domain, LOCAL_ADMIN_RECEIPT_V1_SIGNATURE_DOMAIN);
            assert_eq!(value, &preimage);
            assert_eq!(
                *digest,
                LocalAdminReceiptDigest::hash_canonical(value).unwrap()
            );
            assert_eq!(supplied_signature, &signature());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(verify_calls.get(), 1);
    assert_eq!(
        admitted.canonical_admin_receipt_sha256(),
        issued.canonical_admin_receipt_sha256()
    );

    let mut unsigned = serde_json::to_value(&issued).unwrap();
    unsigned.as_object_mut().unwrap().remove("signature");
    assert!(serde_json::from_value::<LocalAdminReceiptWireV1>(unsigned).is_err());
    assert!(LocalAdminReceiptV1::admit_contextual_wire(
        wire_from(&issued),
        &context,
        |_, _, _, _, _| reject_test_signature()
    )
    .is_err());

    let wrong_command_ref = record_ref("unrelated-command");
    let wrong_command_context = LocalAdminReceiptAdmissionContextV1 {
        admin_command_ref: &wrong_command_ref,
        ..context
    };
    assert!(LocalAdminReceiptV1::admit_contextual_wire(
        wire_from(&issued),
        &wrong_command_context,
        |_, _, _, _, _| panic!("unrelated command reached signature")
    )
    .is_err());

    let mut wrong_records = preimage.committed_records.clone();
    wrong_records[0].committed_record_sha256 = artifact('f');
    let wrong_records_context = LocalAdminReceiptAdmissionContextV1 {
        expected_committed_records: &wrong_records,
        ..context
    };
    assert!(LocalAdminReceiptV1::admit_contextual_wire(
        wire_from(&issued),
        &wrong_records_context,
        |_, _, _, _, _| panic!("unrelated result reached signature")
    )
    .is_err());

    let unrelated_ref = record_ref("unrelated-runtime");
    let unrelated_signing = runtime_signing_authority(&unrelated_ref, &runtime);
    let unrelated_context = LocalAdminReceiptAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &schema,
            signing_authority: &unrelated_signing,
        },
        ..context
    };
    assert!(LocalAdminReceiptV1::admit_contextual_wire(
        wire_from(&issued),
        &unrelated_context,
        |_, _, _, _, _| panic!("unrelated authority reached signature")
    )
    .is_err());

    let assert_rejected_before_signature =
        |changed: LocalAdminReceiptPreimageV1,
         selected_runtime: &DeploymentRuntimeAuthorityV1,
         selected_signing: &SigningAuthorityBindingV1,
         label: &str| {
            let candidate = LocalAdminReceiptV1::seal(changed.clone(), signature()).unwrap();
            let selected_context = contextual_receipt(
                &changed,
                &command_ref,
                &command,
                selected_runtime,
                selected_signing,
            );
            assert!(
                LocalAdminReceiptV1::admit_contextual_wire(
                    wire_from(&candidate),
                    &selected_context,
                    |_, _, _, _, _| panic!("{label} reached signature")
                )
                .is_err(),
                "{label}"
            );
        };
    let mut changed = preimage.clone();
    changed.deployment_runtime_authority_ref = record_ref("unrelated-runtime");
    assert_rejected_before_signature(changed, &runtime, &signing, "changed runtime ref only");
    let mut changed = preimage.clone();
    changed.deployment_runtime_authority_sha256 =
        DeploymentRuntimeAuthorityDigest::parse(sha('f')).unwrap();
    assert_rejected_before_signature(changed, &runtime, &signing, "changed runtime digest only");
    let mut changed = preimage.clone();
    changed.issuer_key_id = ResidentKeyIdV1::parse("unrelated-key").unwrap();
    assert_rejected_before_signature(changed, &runtime, &signing, "changed issuer key");
    let mut changed = preimage.clone();
    changed.issuer_key_epoch = SigningKeyEpochV1::new(2).unwrap();
    assert_rejected_before_signature(changed, &runtime, &signing, "changed issuer key epoch");
    let mut changed = preimage.clone();
    changed.recorded_at = UnixTimeV1::new(30).unwrap();
    assert_rejected_before_signature(changed, &runtime, &signing, "expired recording time");
    let mut changed = preimage.clone();
    changed.committed_at = Some(UnixTimeV1::new(9).unwrap());
    assert_rejected_before_signature(changed, &runtime, &signing, "early commit time");
    let mut changed = preimage.clone();
    changed.committed_at = Some(UnixTimeV1::new(11).unwrap());
    assert_rejected_before_signature(changed, &runtime, &signing, "commit before command issue");
    let mut changed = preimage.clone();
    changed.disposition = LocalAdminDispositionV1::Rejected;
    changed.committed_records.clear();
    changed.head_transitions.clear();
    changed.failure_code = Some(LocalAdminFailureCodeV1::parse("rejected").unwrap());
    changed.committed_at = None;
    changed.recorded_at = UnixTimeV1::new(11).unwrap();
    assert_rejected_before_signature(
        changed,
        &runtime,
        &signing,
        "failed receipt before command issue",
    );
    let mut inactive_preimage = runtime_authority_preimage();
    inactive_preimage.state = PrincipalAuthorityStateV1::Retired;
    let inactive_runtime =
        DeploymentRuntimeAuthorityV1::seal(inactive_preimage, signature()).unwrap();
    let inactive_signing = runtime_signing_authority(&runtime_ref, &inactive_runtime);
    let mut changed = preimage.clone();
    changed.deployment_runtime_authority_sha256 = inactive_runtime
        .canonical_deployment_runtime_authority_sha256()
        .clone();
    assert_rejected_before_signature(
        changed,
        &inactive_runtime,
        &inactive_signing,
        "inactive runtime authority",
    );

    let mut malformed = serde_json::to_value(&issued).unwrap();
    malformed["recorded_at"] = serde_json::json!(22);
    let malformed: LocalAdminReceiptWireV1 = serde_json::from_value(malformed).unwrap();
    assert!(matches!(
        LocalAdminReceiptV1::admit_contextual_wire(malformed, &context, |_, _, _, _, _| panic!(
            "malformed receipt reached signature"
        )),
        Err(InvalidResidentRecordV1::DigestMismatch { .. })
    ));
}

#[test]
fn outcome_dispositions_keep_committed_and_failed_payloads_separate() {
    let committed = receipt();
    assert!(wire(&committed).verify_structure().is_ok());
    for value in [
        failed(LocalAdminDispositionV1::Rejected),
        failed(LocalAdminDispositionV1::Conflict),
    ] {
        assert!(wire(&value).verify_structure().is_ok());
        let mut invalid = value.clone();
        invalid.failure_code = None;
        assert!(wire(&invalid).verify_structure().is_err());
        let mut invalid = value.clone();
        invalid.committed_records = committed.committed_records.clone();
        assert!(wire(&invalid).verify_structure().is_err());
        let mut invalid = value.clone();
        invalid.head_transitions = committed.head_transitions.clone();
        assert!(wire(&invalid).verify_structure().is_err());
        let mut invalid = value;
        invalid.committed_at = committed.committed_at;
        assert!(wire(&invalid).verify_structure().is_err());
    }
    let mut missing = committed.clone();
    missing.committed_records.clear();
    assert!(wire(&missing).verify_structure().is_err());
    let mut missing = committed.clone();
    missing.head_transitions.clear();
    assert!(wire(&missing).verify_structure().is_err());
    let mut missing = committed.clone();
    missing.committed_at = None;
    assert!(wire(&missing).verify_structure().is_err());
    let mut invalid = committed.clone();
    invalid.failure_code = failed(LocalAdminDispositionV1::Rejected).failure_code;
    assert!(wire(&invalid).verify_structure().is_err());
    let mut invalid = committed.clone();
    invalid.recorded_at = UnixTimeV1::new(19).unwrap();
    assert!(wire(&invalid).verify_structure().is_err());
    let mut simultaneous = committed;
    simultaneous.recorded_at = simultaneous.committed_at.unwrap();
    assert!(wire(&simultaneous).verify_structure().is_ok());
}

#[test]
fn head_coordinates_are_paired_and_logical_slots_are_unique() {
    for field in [
        "previous_head_ref",
        "previous_head_sha256",
        "new_head_ref",
        "new_head_sha256",
    ] {
        let mut json = serde_json::to_value(receipt()).unwrap();
        json["head_transitions"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        // Parse the transition to preserve the existing omission/serialization law.
        let head: LocalAdminHeadTransitionV1 =
            serde_json::from_value(json["head_transitions"][0].clone()).unwrap();
        let mut invalid = receipt();
        invalid.head_transitions[0] = head;
        assert!(wire(&invalid).verify_structure().is_err(), "{field}");
    }
    let mut inserted = receipt();
    inserted.head_transitions[0].previous_head_ref = None;
    inserted.head_transitions[0].previous_head_sha256 = None;
    assert!(wire(&inserted).verify_structure().is_ok());
    let mut duplicate = receipt();
    duplicate
        .head_transitions
        .push(duplicate.head_transitions[0].clone());
    assert!(wire(&duplicate).verify_structure().is_err());
    let mut duplicate = receipt();
    duplicate
        .committed_records
        .push(duplicate.committed_records[0].clone());
    assert!(wire(&duplicate).verify_structure().is_err());
    let mut missing = inserted;
    missing.head_transitions[0].new_head_ref = None;
    missing.head_transitions[0].new_head_sha256 = None;
    assert!(wire(&missing).verify_structure().is_err());
}

#[test]
fn committed_checkpoint_needs_both_reference_and_digest_in_affected_domain_head() {
    let mut unchanged = receipt();
    unchanged.head_transitions[0].previous_head_ref =
        unchanged.head_transitions[0].new_head_ref.clone();
    unchanged.head_transitions[0].previous_head_sha256 =
        unchanged.head_transitions[0].new_head_sha256.clone();
    assert!(wire(&unchanged).verify_structure().is_err());
    let mut changed = receipt();
    changed.committed_records[0].committed_record_ref = record_ref("unrelated");
    assert!(wire(&changed).verify_structure().is_err());
    let mut changed = receipt();
    changed.committed_records[0].committed_record_sha256 = artifact('e');
    assert!(wire(&changed).verify_structure().is_err());
    let mut changed = receipt();
    changed.head_transitions[0].head_kind = LocalAdminHeadSelectorV1::PrincipalRecord {
        principal_id: principal(),
    };
    assert!(wire(&changed).verify_structure().is_err());
    let mut valid = receipt();
    valid.committed_records.push(LocalAdminCommittedRecordV1 {
        committed_record_ref: record_ref("candidate"),
        committed_record_sha256: artifact('f'),
    });
    assert!(wire(&valid).verify_structure().is_ok());
}

#[test]
fn structure_check_rejects_changed_payload_but_does_not_authenticate_signature() {
    let original = wire(&receipt());
    let json = serde_json::to_value(&original).unwrap();
    let mut changed = json.clone();
    changed["recorded_at"] = serde_json::json!(22);
    assert!(matches!(
        serde_json::from_value::<LocalAdminReceiptWireV1>(changed)
            .unwrap()
            .verify_structure(),
        Err(InvalidResidentRecordV1::DigestMismatch { .. })
    ));
    let mut changed = json.clone();
    changed["signature"] = serde_json::json!(format!("B{}", "A".repeat(85)));
    assert!(serde_json::from_value::<LocalAdminReceiptWireV1>(changed)
        .unwrap()
        .verify_structure()
        .is_ok());
    let mut changed = json.clone();
    changed["disposition"] = serde_json::json!("replayed");
    assert!(serde_json::from_value::<LocalAdminReceiptWireV1>(changed).is_err());
    let mut changed = json.clone();
    changed["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<LocalAdminReceiptWireV1>(changed).is_err());
    let replay: LocalAdminReceiptWireV1 = serde_json::from_value(json).unwrap();
    assert_eq!(
        serde_json::to_vec(&original).unwrap(),
        serde_json::to_vec(&replay).unwrap()
    );
}

#[test]
fn receipt_collections_and_failure_codes_use_resident_bounds() {
    let mut oversized = receipt();
    oversized.committed_records =
        vec![oversized.committed_records[0].clone(); MAX_RESIDENT_SET_MEMBERS + 1];
    assert!(matches!(
        wire(&oversized).verify_structure(),
        Err(InvalidResidentRecordV1::BoundExceeded { .. })
    ));
    let mut oversized = receipt();
    oversized.head_transitions =
        vec![oversized.head_transitions[0].clone(); MAX_RESIDENT_SET_MEMBERS + 1];
    assert!(matches!(
        wire(&oversized).verify_structure(),
        Err(InvalidResidentRecordV1::BoundExceeded { .. })
    ));
    assert!(LocalAdminFailureCodeV1::parse("").is_err());
    assert!(LocalAdminFailureCodeV1::parse("x".repeat(MAX_RESIDENT_ID_BYTES)).is_ok());
    assert!(LocalAdminFailureCodeV1::parse("x".repeat(MAX_RESIDENT_ID_BYTES + 1)).is_err());
}

#[test]
fn one_linked_checkpoint_cannot_hide_an_unlinked_second_domain() {
    let mut value = receipt();
    let mut second = value.head_transitions[0].clone();
    let LocalAdminHeadSelectorV1::ControlCheckpoint {
        checkpoint_domain_id,
        ..
    } = &mut second.head_kind
    else {
        unreachable!("fixture checkpoint head");
    };
    *checkpoint_domain_id = ControlCheckpointDomainIdV1::parse("producer-incarnation").unwrap();
    second.new_head_ref = Some(record_ref("producer-checkpoint-2"));
    second.new_head_sha256 = Some(artifact('f'));
    value.head_transitions.push(second);
    assert!(wire(&value).verify_structure().is_err());
    value.committed_records.push(LocalAdminCommittedRecordV1 {
        committed_record_ref: record_ref("producer-checkpoint-2"),
        committed_record_sha256: artifact('f'),
    });
    assert!(wire(&value).verify_structure().is_ok());
    let mut changed = value.clone();
    changed.head_transitions[1].new_head_sha256 = Some(artifact('e'));
    assert!(wire(&changed).verify_structure().is_err());
    let mut removed = value;
    removed.head_transitions[1].new_head_ref = None;
    removed.head_transitions[1].new_head_sha256 = None;
    assert!(wire(&removed).verify_structure().is_err());
}
