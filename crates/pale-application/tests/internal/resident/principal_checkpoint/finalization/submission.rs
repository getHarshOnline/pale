//! Signed submission joins over synthetic checkpoint roots, not durable custody.

use super::*;
use crate::deployment_finalization_fixture::changed_wire;
use crate::resident::DeploymentFinalizationSubmission;
use serde_json::{json, Value};

fn with_submission(
    now: u64,
    test: impl FnOnce(ConstructedDeploymentFinalization<'_>, &LocalAdminCommandAdmissionContextV1<'_>),
) {
    with_recovery_submission(now, |constructed, context, _| {
        test(constructed, context);
    });
}

pub(crate) fn with_recovery_submission(
    now: u64,
    test: impl FnOnce(
        ConstructedDeploymentFinalization<'_>,
        &LocalAdminCommandAdmissionContextV1<'_>,
        (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    ),
) {
    with_recovery_submission_at_store(now, "deployment-store-1", "deployment-incarnation-1", test);
}

pub(crate) fn with_recovery_submission_at_store(
    now: u64,
    store_id: &str,
    incarnation_id: &str,
    test: impl FnOnce(
        ConstructedDeploymentFinalization<'_>,
        &LocalAdminCommandAdmissionContextV1<'_>,
        (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    ),
) {
    with_recovery_submission_for_deployment(
        now,
        store_id,
        incarnation_id,
        "deployment-binding-finalization-1",
        "runtime-finalization-1",
        test,
    );
}

pub(crate) fn with_recovery_submission_for_deployment(
    now: u64,
    store_id: &str,
    incarnation_id: &str,
    binding_reference: &str,
    runtime_reference: &str,
    test: impl FnOnce(
        ConstructedDeploymentFinalization<'_>,
        &LocalAdminCommandAdmissionContextV1<'_>,
        (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    ),
) {
    let (mut store, control, records, finalization) = setup_for_deployment(
        store_id,
        incarnation_id,
        binding_reference,
        runtime_reference,
    );
    let source_profile = profile();
    let trust_store = AuthorityStoreCoordinate::new(
        finalization.prior.trust_store_id().clone(),
        finalization.prior.trust_store_incarnation_id().clone(),
    );
    let control_store = AuthorityStoreCoordinate::new(
        control.control_store_id.clone(),
        control.control_incarnation_id.clone(),
    );
    let selected =
        source_profile.bootstrap_profile(&trust_store, &control_store, &control.schema_sha256);
    let authorities = principal_trust_test_support::admit_profile_authorities(
        &PrincipalBootstrapIdV1::parse("submission-bootstrap").unwrap(),
        &selected,
    )
    .unwrap();
    let admin = authorities.local_admin_authority();
    let prior = records.checkpoint(&control);
    let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
    let prepared = profile()
        .prepare_finalization(
            AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
            (&prior_ref, &prior),
            65_536,
            limits(),
            &mut store,
            &mut anchors(&records, &prior_ref, &prior),
        )
        .unwrap();
    let preimage = fixture::finalization_preimage(&prepared, &prior_ref, &prior, control.clone());
    let constructed = prepared
        .admit_checkpoint(
            ResidentRecordRefV1::parse("finalization-control-head").unwrap(),
            principal_trust_test_support::signed_wire(
                &preimage,
                "canonical_control_checkpoint_sha256",
                &ControlStateCheckpointDigest::hash_canonical(&preimage).unwrap(),
            ),
            &control.schema_sha256,
            &records.authority(&control),
            |_, _, _, _| Ok(()),
        )
        .unwrap();
    let schema = SchemaDigest::parse("d".repeat(64)).unwrap();
    test(
        constructed,
        &LocalAdminCommandAdmissionContextV1 {
            expected_schema_sha256: &schema,
            trust_checkpoint: &finalization.prior,
            admin_authority_ref: finalization.prior.local_admin_authority_ref(),
            admin_authority: admin,
            now: UnixTimeV1::new(now).unwrap(),
        },
        (&prior_ref, &prior),
    );
    assert_eq!(store.transaction_reads, 1);
    assert_eq!(
        store.transaction,
        Some(control.committed_transaction_sequence)
    );
}

pub(crate) fn command(
    constructed: &ConstructedDeploymentFinalization<'_>,
    context: &LocalAdminCommandAdmissionContextV1<'_>,
) -> Value {
    let records = constructed.finalization().records();
    let cp = constructed.checkpoint().1;
    let candidate = |schema: &str, reference: &ResidentRecordRefV1, digest: &str| {
        json!({
            "candidate_record_schema_id": schema, "candidate_record_ref": reference,
            "candidate_record_sha256": digest,
        })
    };
    let trust = records.trust.predecessor;
    let signer = records.checkpoint_signer.1;
    let runtime = records.runtime_authority.1;
    json!({
        "schema_id": LOCAL_ADMIN_COMMAND_V1_SCHEMA, "schema_sha256": context.expected_schema_sha256,
        "admin_command_id": "finalization-command", "operation": "finalize_deployment",
        "admin_authority_ref": context.admin_authority_ref,
        "admin_authority_sha256": context.admin_authority.canonical_admin_authority_sha256(),
        "candidate_records": [
            candidate(DEPLOYMENT_BINDING_V1_SCHEMA, records.binding.0, records.binding.1.canonical_binding_sha256().as_str()),
            candidate(CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA, records.checkpoint_signer.0, signer.canonical_checkpoint_signing_authority_sha256().as_str()),
            candidate(DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA, records.runtime_authority.0, runtime.canonical_deployment_runtime_authority_sha256().as_str()),
            candidate(PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA, records.trust.successor_ref, records.trust.successor.canonical_trust_checkpoint_sha256().as_str()),
        ],
        "expected_heads": [
            {"head_kind": {"kind": "principal_trust_checkpoint", "principal_id": trust.principal_id(),
                "trust_store_id": trust.trust_store_id(), "trust_store_incarnation_id": trust.trust_store_incarnation_id()},
             "expected_head_ref": records.trust.predecessor_ref,
             "expected_head_sha256": trust.canonical_trust_checkpoint_sha256(),
             "expected_head_epoch": trust.checkpoint_epoch()},
            {"head_kind": {"kind": "checkpoint_signing_authority", "principal_id": signer.principal_id(),
                "checkpoint_signing_authority_id": signer.checkpoint_signing_authority_id(),
                "control_store_id": signer.control_store_id(), "control_incarnation_id": signer.control_incarnation_id(),
                "scope": signer.scope()},
             "expected_head_ref": null, "expected_head_sha256": null, "expected_head_epoch": null},
            {"head_kind": {"kind": "deployment_runtime_authority", "principal_id": runtime.principal_id(),
                "runtime_authority_id": runtime.runtime_authority_id(), "deployment_binding_ref": runtime.deployment_binding_ref(),
                "deployment_binding_sha256": runtime.deployment_binding_sha256(), "control_store_id": runtime.control_store_id(),
                "control_incarnation_id": runtime.control_incarnation_id()},
             "expected_head_ref": null, "expected_head_sha256": null, "expected_head_epoch": null},
            {"head_kind": {"kind": "control_checkpoint", "control_store_id": cp.control_store_id(),
                "control_incarnation_id": cp.control_incarnation_id(), "checkpoint_domain_id": cp.checkpoint_domain_id()},
             "expected_head_ref": cp.previous_control_checkpoint_ref(),
             "expected_head_sha256": cp.previous_control_checkpoint_sha256(), "expected_head_epoch": cp.domain_epoch()},
        ],
        "idempotency_key": "finalize-once", "issued_at": 60, "expires_at": 300,
        "signer_identity": context.admin_authority.subject_identity(),
        "signer_key_id": context.admin_authority.subject_key_id(),
        "signer_key_epoch": context.admin_authority.subject_key_epoch(),
    })
}

/// Supply one admitted finalization submission to another resident unit test.
pub(crate) fn with_admitted_submission(test: impl FnOnce(DeploymentFinalizationSubmission<'_>)) {
    with_submission(70, |constructed, context| {
        let value = command(&constructed, context);
        let submission = DeploymentFinalizationSubmission::admit(
            constructed,
            wire(value),
            context,
            |_, _, _, _, _| Ok(()),
        )
        .expect("finalization submission admits");
        test(submission);
    });
}

pub(crate) fn wire(mut value: Value) -> LocalAdminCommandWireV1 {
    let heads: Vec<LocalAdminExpectedHeadV1> =
        serde_json::from_value(value["expected_heads"].clone()).unwrap();
    value["expected_heads"] = serde_json::to_value(heads).unwrap();
    let candidates: Vec<LocalAdminCandidateRecordV1> =
        serde_json::from_value(value["candidate_records"].clone()).unwrap();
    value["candidate_set_sha256"] =
        serde_json::to_value(LocalAdminCandidateSetDigest::hash_canonical(&candidates).unwrap())
            .unwrap();
    changed_wire(&value, "canonical_admin_command_sha256", |_| {})
}

fn assert_selection_rejected(
    result: Result<DeploymentFinalizationSubmission<'_>, InvalidResidentRecordV1>,
) {
    assert!(matches!(
        result,
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "deployment finalization submission",
            ..
        })
    ));
}

#[test]
fn exact_submission_retains_command_checkpoint_and_batch() {
    for omit_epochs in [false, true] {
        with_submission(70, |constructed, context| {
            let mut value = command(&constructed, context);
            if omit_epochs {
                for head in value["expected_heads"].as_array_mut().unwrap() {
                    head["expected_head_epoch"] = Value::Null;
                }
            }
            value["expected_heads"].as_array_mut().unwrap().reverse();
            let checkpoint = constructed
                .checkpoint()
                .1
                .canonical_control_checkpoint_sha256()
                .clone();
            let root = constructed.accumulator().state().successor_root().clone();
            let calls = Cell::new(0);
            let submission = DeploymentFinalizationSubmission::admit(
                constructed,
                wire(value),
                context,
                |key, domain, preimage, digest, _| {
                    calls.set(calls.get() + 1);
                    assert_eq!(key, context.admin_authority.subject_public_key());
                    assert_eq!(domain, LOCAL_ADMIN_COMMAND_V1_SIGNATURE_DOMAIN);
                    assert_eq!(
                        *digest,
                        LocalAdminCommandDigest::hash_canonical(preimage).unwrap()
                    );
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(calls.get(), 1);
            assert_eq!(
                submission.command().idempotency_key().as_str(),
                "finalize-once"
            );
            assert_eq!(
                submission
                    .construction()
                    .checkpoint()
                    .1
                    .canonical_control_checkpoint_sha256(),
                &checkpoint
            );
            assert_eq!(
                submission
                    .construction()
                    .accumulator()
                    .state()
                    .successor_root(),
                &root
            );
        });
    }
}

#[test]
fn correctly_rehashed_candidate_substitutions_are_rejected_before_signature() {
    for case in 0..17 {
        with_submission(70, |constructed, context| {
            let mut value = command(&constructed, context);
            match case {
                0 => value["operation"] = "rotate_principal_key".into(),
                1 => {
                    value["candidate_records"].as_array_mut().unwrap().pop();
                }
                2 => {
                    let mut extra = value["candidate_records"][0].clone();
                    extra["candidate_record_ref"] = "extra".into();
                    value["candidate_records"]
                        .as_array_mut()
                        .unwrap()
                        .push(extra);
                }
                3 => value["candidate_records"]
                    .as_array_mut()
                    .unwrap()
                    .swap(0, 1),
                4 => {
                    value["candidate_records"][0]["candidate_record_ref"] =
                        "different-binding".into()
                }
                n => {
                    let candidate = (n - 5) / 3;
                    match (n - 5) % 3 {
                        0 => {
                            value["candidate_records"][candidate]["candidate_record_sha256"] =
                                "f".repeat(64).into()
                        }
                        1 => {
                            value["candidate_records"][candidate]["candidate_record_schema_id"] =
                                LOCAL_ADMIN_AUTHORITY_V1_SCHEMA.into()
                        }
                        _ => {
                            value["candidate_records"][candidate]["candidate_record_ref"] =
                                "substituted-record".into()
                        }
                    }
                }
            }
            assert_selection_rejected(DeploymentFinalizationSubmission::admit(
                constructed,
                wire(value),
                context,
                |_, _, _, _, _| panic!("changed candidates reached signer: {case}"),
            ));
        });
    }
}

#[test]
fn exact_head_membership_scope_contents_and_epochs_are_required() {
    for case in 0..14 {
        with_submission(70, |constructed, context| {
            let mut value = command(&constructed, context);
            match case {
                0 => {
                    value["expected_heads"].as_array_mut().unwrap().pop();
                }
                1 => {
                    let mut extra = value["expected_heads"][1].clone();
                    extra["head_kind"]["checkpoint_signing_authority_id"] = "extra-signer".into();
                    value["expected_heads"].as_array_mut().unwrap().push(extra);
                }
                2 => {
                    value["expected_heads"][0]["head_kind"]["trust_store_id"] =
                        "foreign-store".into()
                }
                3 => {
                    value["expected_heads"][1]["head_kind"]["scope"]["deployment_binding_ref"] =
                        "foreign-binding".into()
                }
                4 => {
                    value["expected_heads"][2]["head_kind"]["deployment_binding_sha256"] =
                        "f".repeat(64).into()
                }
                5 => {
                    value["expected_heads"][3]["head_kind"]["control_store_id"] =
                        "trust-store-1".into()
                }
                6 => {
                    value["expected_heads"][3]["head_kind"]["checkpoint_domain_id"] =
                        "foreign-domain".into()
                }
                7 | 8 => {
                    value["expected_heads"][if case == 7 { 0 } else { 3 }]["expected_head_epoch"] =
                        2.into()
                }
                9 | 10 => {
                    value["expected_heads"][if case == 9 { 0 } else { 3 }]["expected_head_sha256"] =
                        "f".repeat(64).into()
                }
                11 | 12 => {
                    let index = if case == 11 { 1 } else { 2 };
                    value["expected_heads"][index]["expected_head_ref"] = "already-present".into();
                    value["expected_heads"][index]["expected_head_sha256"] = "f".repeat(64).into();
                }
                13 => {
                    value["expected_heads"][3]["expected_head_ref"] = "different-predecessor".into()
                }
                _ => unreachable!(),
            }
            assert_selection_rejected(DeploymentFinalizationSubmission::admit(
                constructed,
                wire(value),
                context,
                |_, _, _, _, _| panic!("changed head reached signer: {case}"),
            ));
        });
    }
}

#[test]
fn unlike_trust_expired_candidates_and_failed_authentication_never_admit_submission() {
    for now in [64, 180, 200, 300] {
        with_submission(now, |constructed, context| {
            let value = command(&constructed, context);
            assert!(DeploymentFinalizationSubmission::admit(
                constructed,
                wire(value),
                context,
                |_, _, _, _, _| panic!("invalid time reached signer")
            )
            .is_err());
        });
    }
    with_submission(70, |constructed, context| {
        let value = command(&constructed, context);
        let foreign = constructed.finalization().records().trust.successor.clone();
        let foreign_context = LocalAdminCommandAdmissionContextV1 {
            trust_checkpoint: &foreign,
            ..*context
        };
        assert!(DeploymentFinalizationSubmission::admit(
            constructed,
            wire(value),
            &foreign_context,
            |_, _, _, _, _| panic!("unlike trust reached signer")
        )
        .is_err());
    });
    with_submission(70, |constructed, context| {
        let value = command(&constructed, context);
        let called = Cell::new(false);
        assert!(DeploymentFinalizationSubmission::admit(
            constructed,
            wire(value),
            context,
            |_, _, _, _, _| {
                called.set(true);
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "test signature",
                    invariant: "invalid signature",
                })
            }
        )
        .is_err());
        assert!(called.get());
    });
}
