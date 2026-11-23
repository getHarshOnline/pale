//! Writer-fence transition-law tests.

use super::*;

#[test]
fn initial_writer_is_generation_one_and_uses_the_exact_domain() {
    let principal_id = principal();
    let key_id = issuer_key();
    let signer_context = signer(&principal_id, &key_id);
    let preimage = writer_preimage();

    PrincipalResidentWriterFenceV1::admit_initial_active_wire(
        writer_wire(preimage.clone()),
        &PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
        &preimage.schema_sha256,
        &signer_context,
        |domain, observed, _, _| {
            assert_eq!(domain, PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN);
            assert_eq!(observed, &preimage);
            Ok(())
        },
    )
    .expect("generation-one genesis admits");

    let mut wrong_generation = preimage;
    wrong_generation.writer_generation =
        PrincipalResidentWriterGenerationV1::new(2).expect("generation");
    assert!(
        PrincipalResidentWriterFenceV1::admit_initial_active_wire(
            writer_wire(wrong_generation),
            &PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
            &schema_digest('a'),
            &signer_context,
            |_, _, _, _| Ok(()),
        )
        .is_err(),
        "non-genesis generation must not enter the initial branch"
    );

    let mut contaminated = writer_preimage();
    contaminated.previous_writer_fence_ref = Some(record_ref("prior-writer"));
    contaminated.previous_writer_fence_sha256 =
        Some(PrincipalResidentWriterFenceDigest::parse(sha('1')).expect("writer digest"));
    contaminated.prior_terminal_receipt_ref = Some(record_ref("prior-terminal"));
    contaminated.prior_terminal_receipt_sha256 = Some(
        PrincipalResidentWriterTerminalReceiptDigest::parse(sha('2')).expect("terminal digest"),
    );
    assert!(
        PrincipalResidentWriterFenceV1::admit_initial_active_wire(
            writer_wire(contaminated),
            &PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
            &schema_digest('a'),
            &signer_context,
            |_, _, _, _| Ok(()),
        )
        .is_err(),
        "genesis cannot carry terminal predecessor evidence"
    );

    let mut takeover_contaminated = writer_preimage();
    takeover_contaminated.previous_writer_fence_ref = Some(record_ref("prior-writer"));
    takeover_contaminated.previous_writer_fence_sha256 =
        Some(PrincipalResidentWriterFenceDigest::parse(sha('1')).expect("writer digest"));
    takeover_contaminated.takeover_receipt_ref = Some(record_ref("takeover"));
    takeover_contaminated.takeover_receipt_sha256 = Some(
        PrincipalResidentWriterTakeoverReceiptDigest::parse(sha('2')).expect("takeover digest"),
    );
    assert!(
        PrincipalResidentWriterFenceV1::admit_initial_active_wire(
            writer_wire(takeover_contaminated),
            &PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
            &schema_digest('a'),
            &signer_context,
            |_, _, _, _| Ok(()),
        )
        .is_err(),
        "genesis cannot carry takeover predecessor evidence"
    );

    assert!(
        PrincipalResidentWriterFenceV1::admit_initial_active_wire(
            writer_wire(writer_preimage()),
            &PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
            &schema_digest('0'),
            &signer_context,
            |_, _, _, _| Ok(()),
        )
        .is_err(),
        "selected schema digest must match exactly"
    );
    let wrong_key = ResidentKeyIdV1::parse("key-2").expect("wrong key");
    let wrong_signer = signer(&principal_id, &wrong_key);
    assert!(
        PrincipalResidentWriterFenceV1::admit_initial_active_wire(
            writer_wire(writer_preimage()),
            &PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
            &schema_digest('a'),
            &wrong_signer,
            |_, _, _, _| Ok(()),
        )
        .is_err(),
        "issuer key must equal the active principal key"
    );
    assert!(
        PrincipalResidentWriterFenceV1::admit_initial_active_wire(
            writer_wire(writer_preimage()),
            &PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
            &schema_digest('a'),
            &signer_context,
            |_, _, _, _| Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "test signature",
                invariant: "authenticated",
            }),
        )
        .is_err(),
        "signature rejection must fail admission"
    );
}

#[test]
fn clean_retirement_and_successor_preserve_the_complete_terminal_chain() {
    let principal_id = principal();
    let key_id = issuer_key();
    let signer = signer(&principal_id, &key_id);
    let active_ref = record_ref("writer-active-1");
    let mut active_preimage = writer_preimage();
    active_preimage.principal_session_floor_epoch =
        PrincipalSessionFloorEpochV1::new(2).expect("active floor");
    let active = writer(active_preimage);
    let terminal_ref = record_ref("writer-terminal-1");
    let terminal_preimage = terminal_preimage(&active_ref, &active);
    let terminal = PrincipalResidentWriterTerminalReceiptV1::admit_contextual_wire(
        terminal_wire(terminal_preimage.clone()),
        &active_writer_head(&active_ref, &active),
        &terminal_preimage.schema_sha256,
        &signer,
        |domain, _, _, _| {
            assert_eq!(
                domain,
                PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SIGNATURE_DOMAIN
            );
            Ok(())
        },
    )
    .expect("exact terminal receipt admits");
    assert!(
        PrincipalResidentWriterTerminalReceiptV1::admit_contextual_wire(
            terminal_wire(terminal_preimage.clone()),
            &active_writer_head(&active_ref, &active),
            &terminal_preimage.schema_sha256,
            &signer,
            |domain, _, _, _| {
                assert_eq!(
                    domain,
                    PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SIGNATURE_DOMAIN
                );
                reject_test_signature()
            },
        )
        .is_err(),
        "terminal receipt signature rejection must fail admission"
    );

    let mut retired_preimage = writer_preimage();
    retired_preimage.principal_session_floor_epoch = *active.principal_session_floor_epoch();
    retired_preimage.previous_writer_fence_ref = Some(active_ref.clone());
    retired_preimage.previous_writer_fence_sha256 = Some(
        active
            .canonical_principal_resident_writer_fence_sha256()
            .clone(),
    );
    retired_preimage.prior_terminal_receipt_ref = Some(terminal_ref.clone());
    retired_preimage.prior_terminal_receipt_sha256 = Some(
        terminal
            .canonical_principal_resident_writer_terminal_receipt_sha256()
            .clone(),
    );
    retired_preimage.state = PrincipalResidentWriterFenceStateV1::Retired;
    retired_preimage.retired_at = Some(*terminal.stopped_at());
    let retired = PrincipalResidentWriterFenceV1::admit_clean_retired_wire(
        writer_wire(retired_preimage.clone()),
        &active_writer_head(&active_ref, &active),
        terminal_context(&terminal_ref, &terminal),
        &retired_preimage.schema_sha256,
        &signer,
        |domain, observed, _, _| {
            assert_eq!(domain, PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN);
            assert_eq!(observed, &retired_preimage);
            Ok(())
        },
    )
    .expect("same-generation clean retirement admits");
    assert_eq!(retired.writer_generation(), active.writer_generation());
    assert!(
        PrincipalResidentWriterFenceV1::admit_clean_retired_wire(
            writer_wire(retired_preimage.clone()),
            &active_writer_head(&active_ref, &active),
            terminal_context(&terminal_ref, &terminal),
            &retired_preimage.schema_sha256,
            &signer,
            |domain, _, _, _| {
                assert_eq!(domain, PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN);
                reject_test_signature()
            },
        )
        .is_err(),
        "clean-retirement signature rejection must fail admission"
    );

    let mut invalid_retirements = Vec::new();
    let mut changed = retired_preimage.clone();
    changed.writer_generation =
        PrincipalResidentWriterGenerationV1::new(2).expect("changed generation");
    invalid_retirements.push(("generation", changed));
    let mut changed = retired_preimage.clone();
    changed.process_instance_id =
        ResidentProcessInstanceIdV1::parse("different-process").expect("process");
    invalid_retirements.push(("process lifetime", changed));
    let mut changed = retired_preimage.clone();
    changed.prior_terminal_receipt_ref = Some(record_ref("different-terminal"));
    invalid_retirements.push(("terminal reference", changed));
    let mut changed = retired_preimage.clone();
    changed.retired_at = Some(UnixTimeV1::new(19).expect("valid fixture time"));
    invalid_retirements.push(("terminal chronology", changed));
    for (law, candidate) in invalid_retirements {
        assert!(
            PrincipalResidentWriterFenceV1::admit_clean_retired_wire(
                writer_wire(candidate),
                &active_writer_head(&active_ref, &active),
                terminal_context(&terminal_ref, &terminal),
                &schema_digest('a'),
                &signer,
                |_, _, _, _| Ok(()),
            )
            .is_err(),
            "clean retirement must preserve {law}"
        );
    }

    let retired_ref = record_ref("writer-retired-1");
    let mut successor = writer_preimage();
    successor.writer_generation =
        PrincipalResidentWriterGenerationV1::new(2).expect("generation two");
    successor.previous_writer_fence_ref = Some(retired_ref.clone());
    successor.previous_writer_fence_sha256 = Some(
        retired
            .canonical_principal_resident_writer_fence_sha256()
            .clone(),
    );
    successor.prior_terminal_receipt_ref = Some(terminal_ref.clone());
    successor.prior_terminal_receipt_sha256 = Some(
        terminal
            .canonical_principal_resident_writer_terminal_receipt_sha256()
            .clone(),
    );
    successor.opened_deployment_store_heads_root_sha256 =
        terminal.final_opened_store_heads_root_sha256().clone();
    successor.principal_session_floor_epoch = *retired.principal_session_floor_epoch();
    successor.process_instance_id =
        ResidentProcessInstanceIdV1::parse("process-2").expect("successor process");
    successor.acquired_at = UnixTimeV1::new(21).expect("valid fixture time");

    PrincipalResidentWriterFenceV1::admit_clean_successor_active_wire(
        writer_wire(successor.clone()),
        &retired_writer_head(&retired_ref, &retired),
        terminal_context(&terminal_ref, &terminal),
        &successor.schema_sha256,
        &signer,
        |domain, observed, _, _| {
            assert_eq!(domain, PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN);
            assert_eq!(observed, &successor);
            Ok(())
        },
    )
    .expect("exact clean successor admits");
    assert!(
        PrincipalResidentWriterFenceV1::admit_clean_successor_active_wire(
            writer_wire(successor.clone()),
            &retired_writer_head(&retired_ref, &retired),
            terminal_context(&terminal_ref, &terminal),
            &successor.schema_sha256,
            &signer,
            |domain, _, _, _| {
                assert_eq!(domain, PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN);
                reject_test_signature()
            },
        )
        .is_err(),
        "clean-successor signature rejection must fail admission"
    );

    let mut invalid_successors = Vec::new();
    let mut changed = successor.clone();
    changed.writer_generation =
        PrincipalResidentWriterGenerationV1::new(3).expect("skipped generation");
    invalid_successors.push(("exact next generation", changed));
    let mut changed = successor.clone();
    changed.principal_session_floor_epoch =
        PrincipalSessionFloorEpochV1::new(1).expect("regressed floor");
    invalid_successors.push(("monotone session floor", changed));
    let mut changed = successor.clone();
    changed.acquired_at = UnixTimeV1::new(19).expect("valid fixture time");
    invalid_successors.push(("monotone acquisition time", changed));
    let mut changed = successor.clone();
    changed.process_instance_id = retired.process_instance_id().clone();
    invalid_successors.push(("distinct successor process instance", changed));
    for (law, candidate) in invalid_successors {
        assert!(
            PrincipalResidentWriterFenceV1::admit_clean_successor_active_wire(
                writer_wire(candidate),
                &retired_writer_head(&retired_ref, &retired),
                terminal_context(&terminal_ref, &terminal),
                &schema_digest('a'),
                &signer,
                |_, _, _, _| Ok(()),
            )
            .is_err(),
            "clean successor must preserve {law}"
        );
    }

    let mut broken_retired_records = Vec::new();
    let mut changed = retired_preimage.clone();
    changed.previous_writer_fence_ref = Some(record_ref("different-active"));
    broken_retired_records.push(("terminal-to-active reference", writer(changed)));
    let mut changed = retired_preimage.clone();
    changed.previous_writer_fence_sha256 =
        Some(PrincipalResidentWriterFenceDigest::parse(sha('0')).expect("writer digest"));
    broken_retired_records.push(("terminal-to-active digest", writer(changed)));
    let mut changed = retired_preimage.clone();
    changed.retired_at = Some(UnixTimeV1::new(21).expect("valid fixture time"));
    broken_retired_records.push(("retirement stop time", writer(changed)));
    for (law, broken_retired) in broken_retired_records {
        let mut candidate = successor.clone();
        candidate.previous_writer_fence_sha256 = Some(
            broken_retired
                .canonical_principal_resident_writer_fence_sha256()
                .clone(),
        );
        assert!(
            PrincipalResidentWriterFenceV1::admit_clean_successor_active_wire(
                writer_wire(candidate),
                &retired_writer_head(&retired_ref, &broken_retired),
                terminal_context(&terminal_ref, &terminal),
                &schema_digest('a'),
                &signer,
                |_, _, _, _| Ok(()),
            )
            .is_err(),
            "clean successor must re-establish {law}"
        );
    }

    let mut wrong_final_head = successor;
    wrong_final_head.opened_deployment_store_heads_root_sha256 = artifact('0');
    assert!(
        PrincipalResidentWriterFenceV1::admit_clean_successor_active_wire(
            writer_wire(wrong_final_head),
            &retired_writer_head(&retired_ref, &retired),
            terminal_context(&terminal_ref, &terminal),
            &schema_digest('a'),
            &signer,
            |_, _, _, _| Ok(()),
        )
        .is_err(),
        "a retired fence cannot substitute for the terminal final store-head root"
    );
}

#[test]
fn unclean_takeover_advances_once_without_a_synthetic_retired_fence() {
    let principal_id = principal();
    let key_id = issuer_key();
    let signer = signer(&principal_id, &key_id);
    let active_ref = record_ref("writer-active-1");
    let active = writer(writer_preimage());
    let takeover_ref = record_ref("takeover-1");
    let takeover_preimage = takeover_preimage(&active_ref, &active);
    let sealed_takeover =
        PrincipalResidentWriterTakeoverReceiptV1::seal(takeover_preimage.clone(), signature())
            .expect("takeover receipt");
    let takeover_digest = sealed_takeover
        .canonical_principal_resident_writer_takeover_receipt_sha256()
        .clone();
    let takeover = PrincipalResidentWriterTakeoverReceiptV1::admit_contextual_wire(
        wire_from(&sealed_takeover),
        &active_writer_head(&active_ref, &active),
        &takeover_preimage.schema_sha256,
        &signer,
        |observed_principal, observed_nonce, observed_digest| {
            assert_eq!(observed_principal, &takeover_preimage.principal_id);
            assert_eq!(observed_nonce, &takeover_preimage.takeover_nonce);
            assert_eq!(observed_digest, &takeover_digest);
            Ok(())
        },
        |domain, _, _, _| {
            assert_eq!(
                domain,
                PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SIGNATURE_DOMAIN
            );
            Ok(())
        },
    )
    .expect("takeover receipt admits");

    let nonce_checked = std::cell::Cell::new(false);
    assert!(
        PrincipalResidentWriterTakeoverReceiptV1::admit_contextual_wire(
            wire_from(&sealed_takeover),
            &active_writer_head(&active_ref, &active),
            &takeover_preimage.schema_sha256,
            &signer,
            |_, _, _| {
                nonce_checked.set(true);
                Ok(())
            },
            |_, _, _, _| Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "test signature",
                invariant: "authenticated",
            }),
        )
        .is_err(),
        "an unauthenticated takeover must be rejected"
    );
    assert!(
        !nonce_checked.get(),
        "signature verification must precede the owner nonce witness"
    );
    assert!(
        PrincipalResidentWriterTakeoverReceiptV1::admit_contextual_wire(
            wire_from(&sealed_takeover),
            &active_writer_head(&active_ref, &active),
            &takeover_preimage.schema_sha256,
            &signer,
            |_, _, _| Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "takeover nonce",
                invariant: "unallocated or exact same-digest replay",
            }),
            |_, _, _, _| Ok(()),
        )
        .is_err(),
        "a nonce mapped to different bytes must reject changed reuse"
    );

    let mut invalid_receipts = Vec::new();
    let mut changed = takeover_preimage.clone();
    changed.successor_process_instance_id = active.process_instance_id().clone();
    invalid_receipts.push(("distinct successor process", changed));
    let mut changed = takeover_preimage.clone();
    changed.prior_session_floor_epoch =
        PrincipalSessionFloorEpochV1::new(2).expect("wrong prior floor");
    changed.new_session_floor_epoch =
        PrincipalSessionFloorEpochV1::new(3).expect("paired new floor");
    invalid_receipts.push(("predecessor session floor", changed));
    let mut changed = takeover_preimage.clone();
    changed.authorized_at = UnixTimeV1::new(9).expect("valid fixture time");
    invalid_receipts.push(("monotone authorization time", changed));
    for (law, candidate) in invalid_receipts {
        let sealed = PrincipalResidentWriterTakeoverReceiptV1::seal(candidate, signature())
            .expect("structurally valid negative receipt");
        assert!(
            PrincipalResidentWriterTakeoverReceiptV1::admit_contextual_wire(
                wire_from(&sealed),
                &active_writer_head(&active_ref, &active),
                &schema_digest('6'),
                &signer,
                |_, _, _| Ok(()),
                |_, _, _, _| Ok(()),
            )
            .is_err(),
            "takeover receipt must preserve {law}"
        );
    }
    let mut invalid_floor_step = takeover_preimage.clone();
    invalid_floor_step.new_session_floor_epoch =
        PrincipalSessionFloorEpochV1::new(3).expect("skipped floor");
    assert!(
        PrincipalResidentWriterTakeoverReceiptV1::seal(invalid_floor_step, signature()).is_err(),
        "takeover session floor must advance exactly once"
    );

    let mut successor = writer_preimage();
    successor.writer_generation =
        PrincipalResidentWriterGenerationV1::new(2).expect("generation two");
    successor.previous_writer_fence_ref = Some(active_ref.clone());
    successor.previous_writer_fence_sha256 = Some(
        active
            .canonical_principal_resident_writer_fence_sha256()
            .clone(),
    );
    successor.takeover_receipt_ref = Some(takeover_ref.clone());
    successor.takeover_receipt_sha256 = Some(
        takeover
            .canonical_principal_resident_writer_takeover_receipt_sha256()
            .clone(),
    );
    successor.primary_node_id = takeover.successor_primary_node_id().clone();
    successor.process_instance_id = takeover.successor_process_instance_id().clone();
    successor.boot_incarnation_id = takeover.successor_boot_incarnation_id().clone();
    successor.opened_deployment_store_heads_root_sha256 =
        takeover.verified_store_heads_root_sha256().clone();
    successor.principal_session_floor_epoch = *takeover.new_session_floor_epoch();
    successor.acquired_at = *takeover.authorized_at();

    let admitted = PrincipalResidentWriterFenceV1::admit_takeover_successor_active_wire(
        writer_wire(successor.clone()),
        &active_writer_head(&active_ref, &active),
        takeover_context(&takeover_ref, &takeover),
        &schema_digest('a'),
        &signer,
        |domain, observed, _, _| {
            assert_eq!(domain, PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN);
            assert_eq!(observed, &successor);
            Ok(())
        },
    )
    .expect("direct takeover successor admits");
    assert_eq!(
        admitted.state(),
        &PrincipalResidentWriterFenceStateV1::Active
    );
    assert!(admitted.prior_terminal_receipt_ref().is_none());
    assert_eq!(
        admitted.previous_writer_fence_ref().as_ref(),
        Some(&active_ref)
    );
    assert!(
        PrincipalResidentWriterFenceV1::admit_takeover_successor_active_wire(
            writer_wire(successor.clone()),
            &active_writer_head(&active_ref, &active),
            takeover_context(&takeover_ref, &takeover),
            &schema_digest('a'),
            &signer,
            |domain, _, _, _| {
                assert_eq!(domain, PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN);
                reject_test_signature()
            },
        )
        .is_err(),
        "takeover-successor signature rejection must fail admission"
    );

    let mut invalid_successors = Vec::new();
    let mut changed = successor.clone();
    changed.writer_generation =
        PrincipalResidentWriterGenerationV1::new(3).expect("skipped generation");
    invalid_successors.push(("exact next generation", changed));
    let mut changed = successor.clone();
    changed.primary_node_id = ResidentNodeIdV1::parse("wrong-node").expect("node");
    invalid_successors.push(("successor process tuple", changed));
    let mut changed = successor.clone();
    changed.opened_deployment_store_heads_root_sha256 = artifact('0');
    invalid_successors.push(("verified store-head root", changed));
    let mut changed = successor;
    changed.acquired_at = UnixTimeV1::new(19).expect("valid fixture time");
    invalid_successors.push(("takeover chronology", changed));
    for (law, candidate) in invalid_successors {
        assert!(
            PrincipalResidentWriterFenceV1::admit_takeover_successor_active_wire(
                writer_wire(candidate),
                &active_writer_head(&active_ref, &active),
                takeover_context(&takeover_ref, &takeover),
                &schema_digest('a'),
                &signer,
                |_, _, _, _| Ok(()),
            )
            .is_err(),
            "takeover successor must preserve {law}"
        );
    }
}

#[test]
fn every_writer_transition_consumes_the_exact_current_head_state() {
    let principal_id = principal();
    let key_id = issuer_key();
    let signer = signer(&principal_id, &key_id);
    let active_ref = record_ref("writer-active-1");
    let active = writer(writer_preimage());
    let terminal_ref = record_ref("writer-terminal-1");
    let terminal_preimage = terminal_preimage(&active_ref, &active);
    let terminal = terminal(terminal_preimage.clone());

    let mut retired_preimage = writer_preimage();
    retired_preimage.previous_writer_fence_ref = Some(active_ref.clone());
    retired_preimage.previous_writer_fence_sha256 = Some(
        active
            .canonical_principal_resident_writer_fence_sha256()
            .clone(),
    );
    retired_preimage.prior_terminal_receipt_ref = Some(terminal_ref.clone());
    retired_preimage.prior_terminal_receipt_sha256 = Some(
        terminal
            .canonical_principal_resident_writer_terminal_receipt_sha256()
            .clone(),
    );
    retired_preimage.state = PrincipalResidentWriterFenceStateV1::Retired;
    retired_preimage.retired_at = Some(*terminal.stopped_at());
    let retired = writer(retired_preimage.clone());
    let retired_ref = record_ref("writer-retired-1");

    let takeover_ref = record_ref("takeover-1");
    let takeover_preimage = takeover_preimage(&active_ref, &active);
    let takeover =
        PrincipalResidentWriterTakeoverReceiptV1::seal(takeover_preimage.clone(), signature())
            .expect("takeover receipt");

    let mut clean_successor = writer_preimage();
    clean_successor.writer_generation =
        PrincipalResidentWriterGenerationV1::new(2).expect("successor generation");
    clean_successor.previous_writer_fence_ref = Some(retired_ref.clone());
    clean_successor.previous_writer_fence_sha256 = Some(
        retired
            .canonical_principal_resident_writer_fence_sha256()
            .clone(),
    );
    clean_successor.prior_terminal_receipt_ref = Some(terminal_ref.clone());
    clean_successor.prior_terminal_receipt_sha256 = Some(
        terminal
            .canonical_principal_resident_writer_terminal_receipt_sha256()
            .clone(),
    );
    clean_successor.opened_deployment_store_heads_root_sha256 =
        terminal.final_opened_store_heads_root_sha256().clone();
    clean_successor.acquired_at = UnixTimeV1::new(21).expect("valid fixture time");

    let mut takeover_successor = writer_preimage();
    takeover_successor.writer_generation =
        PrincipalResidentWriterGenerationV1::new(2).expect("successor generation");
    takeover_successor.previous_writer_fence_ref = Some(active_ref.clone());
    takeover_successor.previous_writer_fence_sha256 = Some(
        active
            .canonical_principal_resident_writer_fence_sha256()
            .clone(),
    );
    takeover_successor.takeover_receipt_ref = Some(takeover_ref.clone());
    takeover_successor.takeover_receipt_sha256 = Some(
        takeover
            .canonical_principal_resident_writer_takeover_receipt_sha256()
            .clone(),
    );
    takeover_successor.primary_node_id = takeover.successor_primary_node_id().clone();
    takeover_successor.process_instance_id = takeover.successor_process_instance_id().clone();
    takeover_successor.boot_incarnation_id = takeover.successor_boot_incarnation_id().clone();
    takeover_successor.opened_deployment_store_heads_root_sha256 =
        takeover.verified_store_heads_root_sha256().clone();
    takeover_successor.principal_session_floor_epoch = *takeover.new_session_floor_epoch();
    takeover_successor.acquired_at = *takeover.authorized_at();

    for occupied_head in [
        active_writer_head(&active_ref, &active),
        retired_writer_head(&retired_ref, &retired),
    ] {
        assert!(
            PrincipalResidentWriterFenceV1::admit_initial_active_wire(
                writer_wire(writer_preimage()),
                &occupied_head,
                &schema_digest('a'),
                &signer,
                |_, _, _, _| Ok(()),
            )
            .is_err(),
            "initial acquisition requires an unallocated current head"
        );
    }

    for non_active_head in [
        PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
        retired_writer_head(&retired_ref, &retired),
    ] {
        assert!(
            PrincipalResidentWriterTerminalReceiptV1::admit_contextual_wire(
                terminal_wire(terminal_preimage.clone()),
                &non_active_head,
                &schema_digest('c'),
                &signer,
                |_, _, _, _| Ok(()),
            )
            .is_err()
        );
        assert!(PrincipalResidentWriterFenceV1::admit_clean_retired_wire(
            writer_wire(retired_preimage.clone()),
            &non_active_head,
            terminal_context(&terminal_ref, &terminal),
            &schema_digest('a'),
            &signer,
            |_, _, _, _| Ok(()),
        )
        .is_err());
        assert!(
            PrincipalResidentWriterTakeoverReceiptV1::admit_contextual_wire(
                wire_from(&takeover),
                &non_active_head,
                &schema_digest('6'),
                &signer,
                |_, _, _| Ok(()),
                |_, _, _, _| Ok(()),
            )
            .is_err()
        );
        assert!(
            PrincipalResidentWriterFenceV1::admit_takeover_successor_active_wire(
                writer_wire(takeover_successor.clone()),
                &non_active_head,
                takeover_context(&takeover_ref, &takeover),
                &schema_digest('a'),
                &signer,
                |_, _, _, _| Ok(()),
            )
            .is_err()
        );
    }

    for non_retired_head in [
        PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
        active_writer_head(&active_ref, &active),
    ] {
        assert!(
            PrincipalResidentWriterFenceV1::admit_clean_successor_active_wire(
                writer_wire(clean_successor.clone()),
                &non_retired_head,
                terminal_context(&terminal_ref, &terminal),
                &schema_digest('a'),
                &signer,
                |_, _, _, _| Ok(()),
            )
            .is_err(),
            "clean successor requires a retired current head"
        );
    }

    let mut current_preimage = writer_preimage();
    current_preimage.process_instance_id =
        ResidentProcessInstanceIdV1::parse("current-process").expect("current process");
    let current = writer(current_preimage);
    for current_ref in [record_ref("writer-current"), active_ref.clone()] {
        assert!(
            PrincipalResidentWriterTerminalReceiptV1::admit_contextual_wire(
                terminal_wire(terminal_preimage.clone()),
                &active_writer_head(&current_ref, &current),
                &schema_digest('c'),
                &signer,
                |_, _, _, _| Ok(()),
            )
            .is_err(),
            "a stale reference or forked digest cannot replace the current head"
        );
    }

    let mismatched_variant = PrincipalResidentWriterFenceHeadAdmissionStateV1::Active {
        writer_fence_ref: &retired_ref,
        writer_fence: &retired,
    };
    assert!(
        PrincipalResidentWriterTerminalReceiptV1::admit_contextual_wire(
            terminal_wire(terminal_preimage),
            &mismatched_variant,
            &schema_digest('c'),
            &signer,
            |_, _, _, _| Ok(()),
        )
        .is_err(),
        "the head variant must agree with the verified record state"
    );
}

#[test]
fn writer_generation_overflow_is_rejected_before_transition() {
    let generation = PrincipalResidentWriterGenerationV1::new(crate::canon::JSON_SAFE_INTEGER_MAX)
        .expect("maximum portable generation");
    assert_eq!(
        super::super::writer::next_writer_generation(&generation),
        None,
        "successor arithmetic must not wrap"
    );
    let mut active_preimage = writer_preimage();
    active_preimage.writer_generation = generation;
    assert!(
        PrincipalResidentWriterFenceV1::seal(active_preimage, signature()).is_ok(),
        "the portable ceiling itself remains a valid terminal generation"
    );
}
