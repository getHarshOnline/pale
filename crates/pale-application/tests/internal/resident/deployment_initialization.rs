//! Behavioral qualification of the two deployment-initialization transactions.

use super::*;
use crate::deployment_finalization_fixture::changed_wire;
use crate::resident::principal_checkpoint::tests::finalization::submission;
use pale_ports::{
    AccumulatorCheckpointBinding, AccumulatorMutation, AccumulatorPosition,
    AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError, AntiRollbackAnchorErrorKind,
    AntiRollbackAnchorKey, AntiRollbackAnchorObservation, ControlRecordDigest, ControlStoreError,
    ControlStoreErrorKind, ControlTransactionPreparation,
    DeploymentControlStoreGenesisCommitDisposition,
    DeploymentControlStoreGenesisDatabaseObservation, DeploymentFinalizationCommitDisposition,
    DeploymentFinalizationDatabaseObservation, StateAccumulatorBatch,
    UnresolvedCustodyAccumulatorBatch,
};

fn hash(character: char) -> ArtifactDigest {
    ArtifactDigest::parse(character.to_string().repeat(64)).expect("fixture digest")
}

fn admitted_submission(test: impl FnOnce(DeploymentFinalizationSubmission<'_>)) {
    submission::with_admitted_submission(test);
}

fn genesis_input(
    submission: &DeploymentFinalizationSubmission<'_>,
) -> SignedDeploymentGenesisInput {
    let records = submission.construction().finalization().records();
    let binding_record = records.binding.1;
    let signer = records.checkpoint_signer.1;
    let runtime = records.runtime_authority.1;
    let domain = ControlCheckpointDomainIdV1::parse("deployment-operation").unwrap();
    let checkpoint_schema = SchemaDigest::parse("7".repeat(64)).unwrap();
    let store = AuthorityStoreCoordinate::new(
        runtime.control_store_id().clone(),
        runtime.control_incarnation_id().clone(),
    );
    let checkpoint_ref = ResidentRecordRefV1::parse("deployment-genesis-checkpoint").unwrap();
    let checkpoint_preimage = ControlStateCheckpointPreimageV1 {
        schema_id: SchemaId::parse(CONTROL_STATE_CHECKPOINT_V1_SCHEMA).unwrap(),
        schema_sha256: checkpoint_schema.clone(),
        control_checkpoint_id: ControlCheckpointIdV1::parse("deployment-genesis-checkpoint")
            .unwrap(),
        control_store_id: store.store_id().clone(),
        control_incarnation_id: store.incarnation_id().clone(),
        checkpoint_domain_id: domain.clone(),
        domain_epoch: ControlDomainEpochV1::new(1).unwrap(),
        domain_sequence: ControlDomainSequenceV1::new(1).unwrap(),
        previous_control_checkpoint_ref: None,
        previous_control_checkpoint_sha256: None,
        checkpoint_profile_ref: signer.checkpoint_profile_ref().clone(),
        checkpoint_profile_sha256: signer.checkpoint_profile_sha256().clone(),
        control_schema_version: 1,
        committed_transaction_sequence: ControlTransactionSequenceV1::new(2).unwrap(),
        state_root_sha256: hash('5'),
        leaf_count: ControlLeafCountV1::new(1).unwrap(),
        changed_leaf_keys: Vec::new(),
        unresolved_custody_root_sha256: hash('0'),
        issued_at: UnixTimeV1::new(80).unwrap(),
        checkpoint_signing_authority_ref: records.checkpoint_signer.0.clone(),
        checkpoint_signing_authority_sha256: signer
            .canonical_checkpoint_signing_authority_sha256()
            .clone(),
        issuer_principal_id: signer.principal_id().clone(),
        issuer_key_id: signer.subject_key_id().clone(),
        issuer_key_epoch: *signer.subject_key_epoch(),
    };
    let checkpoint = ControlStateCheckpointV1::issue_initial_contextual(
        checkpoint_preimage,
        &checkpoint_schema,
        &ControlCheckpointSigningAuthorityAdmissionContextV1 {
            authority_ref: records.checkpoint_signer.0,
            authority_sha256: signer.canonical_checkpoint_signing_authority_sha256(),
            principal_id: signer.principal_id(),
            control_store_id: signer.control_store_id(),
            control_incarnation_id: signer.control_incarnation_id(),
            checkpoint_domain_id: &domain,
            checkpoint_profile_ref: signer.checkpoint_profile_ref(),
            checkpoint_profile_sha256: signer.checkpoint_profile_sha256(),
            subject_key_id: signer.subject_key_id(),
            subject_key_epoch: *signer.subject_key_epoch(),
        },
        |_, _, _| Ok(CanonicalSignatureV1::parse("A".repeat(86)).unwrap()),
    )
    .unwrap();
    let checkpoint_head = ControlCheckpointHead::new(
        domain.clone(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(1).unwrap(),
        ControlTransactionSequenceV1::new(2).unwrap(),
        pale_ports::ControlRecordCoordinate::new(
            checkpoint_ref.clone(),
            ControlRecordDigest::parse(checkpoint.canonical_control_checkpoint_sha256().as_str())
                .unwrap(),
        ),
    );
    let transaction = ControlTransactionPreparation::new(
        store.clone(),
        ControlTransactionSequenceV1::new(1).unwrap(),
    )
    .unwrap();
    let accumulator = OperationAccumulatorBatch::new(
        AccumulatorCheckpointBinding::new(
            store.clone(),
            signer.checkpoint_profile_ref().clone(),
            signer.checkpoint_profile_sha256().clone(),
            AccumulatorCheckpointPhase::Initial,
            checkpoint_head,
            transaction,
        )
        .unwrap(),
        StateAccumulatorBatch::new(
            hash('0'),
            hash('5'),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([5; 32]),
                None,
                Some(vec![5; 32]),
                32,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(hash('0'), hash('0'), Vec::new(), 1).unwrap(),
    )
    .unwrap();
    let receipt_schema = SchemaDigest::parse("6".repeat(64)).unwrap();
    let genesis_id = DeploymentControlStoreGenesisIdV1::parse("deployment-genesis").unwrap();
    let checkpoint_member = InitialControlCheckpointV1 {
        checkpoint_domain_id: domain.clone(),
        domain_epoch: *checkpoint.domain_epoch(),
        domain_sequence: *checkpoint.domain_sequence(),
        control_checkpoint_ref: checkpoint_ref.clone(),
        control_checkpoint_sha256: checkpoint.canonical_control_checkpoint_sha256().clone(),
    };
    let integrated = IntegratedRuntimeAuthorityV1 {
        deployment_runtime_authority_ref: records.runtime_authority.0.clone(),
        deployment_runtime_authority_sha256: ArtifactDigest::parse(
            runtime
                .canonical_deployment_runtime_authority_sha256()
                .as_str(),
        )
        .unwrap(),
        deployment_binding_ref: records.binding.0.clone(),
        deployment_binding_sha256: ArtifactDigest::parse(
            binding_record.canonical_binding_sha256().as_str(),
        )
        .unwrap(),
        checked_principal_trust_checkpoint_ref: records.trust.successor_ref.clone(),
        checked_principal_trust_checkpoint_sha256: ArtifactDigest::parse(
            records
                .trust
                .successor
                .canonical_trust_checkpoint_sha256()
                .as_str(),
        )
        .unwrap(),
    };
    let signing_authority = SigningAuthorityBindingV1::integrated_runtime(
        runtime.principal_id().clone(),
        runtime.subject_key_id().clone(),
        *runtime.subject_key_epoch(),
        integrated,
    )
    .unwrap();
    let preimage = DeploymentControlStoreGenesisReceiptPreimageV1 {
        schema_id: SchemaId::parse(DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA).unwrap(),
        schema_sha256: receipt_schema.clone(),
        genesis_id: genesis_id.clone(),
        principal_id: runtime.principal_id().clone(),
        workspace_id: binding_record.deployment_terms().workspace_id().clone(),
        deployment_binding_ref: records.binding.0.clone(),
        deployment_binding_sha256: binding_record.canonical_binding_sha256().clone(),
        control_store_id: store.store_id().clone(),
        control_incarnation_id: store.incarnation_id().clone(),
        checkpoint_signing_authority_ref: records.checkpoint_signer.0.clone(),
        checkpoint_signing_authority_sha256: signer
            .canonical_checkpoint_signing_authority_sha256()
            .clone(),
        initial_control_checkpoints: vec![checkpoint_member],
        committed_at: UnixTimeV1::new(80).unwrap(),
        deployment_runtime_authority_ref: records.runtime_authority.0.clone(),
        deployment_runtime_authority_sha256: runtime
            .canonical_deployment_runtime_authority_sha256()
            .clone(),
        issuer_principal_id: runtime.principal_id().clone(),
        issuer_key_id: runtime.subject_key_id().clone(),
        issuer_key_epoch: *runtime.subject_key_epoch(),
    };
    let initial = [(&checkpoint_ref, &checkpoint)];
    let receipt = DeploymentControlStoreGenesisReceiptV1::issue_contextual(
        preimage,
        &DeploymentControlStoreGenesisAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &receipt_schema,
                signing_authority: &signing_authority,
            },
            deployment_binding_ref: records.binding.0,
            deployment_binding: binding_record,
            runtime_authority: runtime,
            checkpoint_signing_authority_ref: records.checkpoint_signer.0,
            checkpoint_signing_authority: signer,
            required_checkpoint_domains: std::slice::from_ref(&domain),
            initial_control_checkpoints: &initial,
        },
        |_, _, _| Ok(CanonicalSignatureV1::parse("A".repeat(86)).unwrap()),
    )
    .unwrap();
    SignedDeploymentGenesisInput::new(
        DeploymentControlStoreBinding::new(
            store.store_id().clone(),
            store.incarnation_id().clone(),
            runtime.principal_id().clone(),
            binding_record.deployment_terms().workspace_id().clone(),
            pale_ports::ControlRecordCoordinate::new(
                records.binding.0.clone(),
                ControlRecordDigest::parse(binding_record.canonical_binding_sha256().as_str())
                    .unwrap(),
            ),
            pale_ports::ControlRecordCoordinate::new(
                records.checkpoint_signer.0.clone(),
                ControlRecordDigest::parse(
                    signer
                        .canonical_checkpoint_signing_authority_sha256()
                        .as_str(),
                )
                .unwrap(),
            ),
            pale_ports::ControlRecordCoordinate::new(
                records.runtime_authority.0.clone(),
                ControlRecordDigest::parse(
                    runtime
                        .canonical_deployment_runtime_authority_sha256()
                        .as_str(),
                )
                .unwrap(),
            ),
        ),
        genesis_id,
        ResidentRecordRefV1::parse("deployment-genesis-receipt").unwrap(),
        receipt,
        vec![SignedDeploymentGenesisCheckpoint::new(
            checkpoint_ref,
            checkpoint,
            accumulator,
        )],
    )
}

#[derive(Clone, Copy)]
enum CommitMode {
    New,
    Replay,
    AmbiguousExact,
}

struct FinalizationStore {
    mode: CommitMode,
    compares: usize,
    observations: usize,
}

impl DeploymentFinalizationCustodyPort for FinalizationStore {
    fn compare_and_commit(
        &mut self,
        _: &PreparedDeploymentFinalizationAtom,
    ) -> Result<DeploymentFinalizationCommitDisposition, ControlStoreError> {
        self.compares += 1;
        match self.mode {
            CommitMode::New => Ok(DeploymentFinalizationCommitDisposition::New),
            CommitMode::Replay => Ok(DeploymentFinalizationCommitDisposition::ExactReplay),
            CommitMode::AmbiguousExact => Err(ControlStoreError::new(
                ControlStoreErrorKind::OutcomeUnknown,
            )),
        }
    }

    fn observe_exact(
        &mut self,
        _: &PreparedDeploymentFinalizationAtom,
    ) -> Result<DeploymentFinalizationDatabaseObservation, ControlStoreError> {
        self.observations += 1;
        Ok(DeploymentFinalizationDatabaseObservation::Exact)
    }
}

struct GenesisStore {
    mode: CommitMode,
    compares: usize,
    observations: usize,
}

impl DeploymentControlStoreGenesisCustodyPort for GenesisStore {
    fn compare_and_commit(
        &mut self,
        _: &PreparedDeploymentControlStoreGenesisAtom,
    ) -> Result<DeploymentControlStoreGenesisCommitDisposition, ControlStoreError> {
        self.compares += 1;
        match self.mode {
            CommitMode::New => Ok(DeploymentControlStoreGenesisCommitDisposition::New),
            CommitMode::Replay => Ok(DeploymentControlStoreGenesisCommitDisposition::ExactReplay),
            CommitMode::AmbiguousExact => Err(ControlStoreError::new(
                ControlStoreErrorKind::OutcomeUnknown,
            )),
        }
    }

    fn observe_exact(
        &mut self,
        _: &PreparedDeploymentControlStoreGenesisAtom,
    ) -> Result<DeploymentControlStoreGenesisDatabaseObservation, ControlStoreError> {
        self.observations += 1;
        Ok(DeploymentControlStoreGenesisDatabaseObservation::Exact)
    }
}

struct Anchors {
    compares: usize,
    observations: usize,
    fail_compare: Option<usize>,
    replay: bool,
}

impl AntiRollbackAnchorPort for Anchors {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        unreachable!("coordinator uses transition-scoped observation")
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.compares += 1;
        if self.fail_compare == Some(self.compares) {
            return Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            ));
        }
        Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)
    }

    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.observations += 1;
        Ok(if self.replay {
            AntiRollbackAnchorObservation::ExactSuccessor
        } else {
            AntiRollbackAnchorObservation::ExactPredecessor
        })
    }
}

fn finalization_input(
    submission: DeploymentFinalizationSubmission<'_>,
) -> SignedDeploymentFinalizationInput<'_> {
    SignedDeploymentFinalizationInput::issue(
        submission,
        DeploymentFinalizationReceiptParameters::new(
            ResidentRecordRefV1::parse("deployment-finalization-command").unwrap(),
            ResidentRecordRefV1::parse("deployment-finalization-receipt").unwrap(),
            LocalAdminReceiptIdV1::parse("deployment-finalization-receipt").unwrap(),
            SchemaDigest::parse("8".repeat(64)).unwrap(),
            UnixTimeV1::new(80).unwrap(),
            UnixTimeV1::new(81).unwrap(),
        ),
        |domain, _, _| {
            assert_eq!(domain, LOCAL_ADMIN_RECEIPT_V1_SIGNATURE_DOMAIN);
            Ok(CanonicalSignatureV1::parse("A".repeat(86)).unwrap())
        },
    )
    .unwrap()
}

fn receipt_wire(receipt: &LocalAdminReceiptV1) -> LocalAdminReceiptWireV1 {
    serde_json::from_value(serde_json::to_value(receipt).unwrap()).unwrap()
}

fn expect_phase<T, E>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(_) => panic!("expected phase transition to succeed"),
    }
}

fn expect_phase_failure<T, E>(result: Result<T, E>) -> E {
    match result {
        Ok(_) => panic!("expected phase transition to fail"),
        Err(failure) => failure,
    }
}

#[test]
fn genesis_is_unavailable_until_both_finalization_anchors_are_exact() {
    admitted_submission(|submission| {
        let mut store = FinalizationStore {
            mode: CommitMode::New,
            compares: 0,
            observations: 0,
        };
        let prepared = expect_phase(finalization_input(submission).prepare());
        let pending = expect_phase(prepared.commit(&mut store));
        let mut anchors = Anchors {
            compares: 0,
            observations: 0,
            fail_compare: Some(2),
            replay: false,
        };
        let failed = expect_phase_failure(pending.complete(&mut anchors));
        assert_eq!(
            failed.owner(),
            DeploymentFinalizationAnchorOwner::PrincipalControlDomain
        );
        assert_eq!(anchors.compares, 2);
        let anchored = expect_phase(failed.into_pending().complete(&mut anchors));
        assert_eq!(anchors.compares, 3);
        assert_eq!(anchors.observations, 1);
        assert_eq!(
            anchored.database_commit_evidence(),
            DeploymentFinalizationCommitEvidence::New
        );
    });
}

#[test]
fn separate_stores_commit_separate_receipt_families_in_order() {
    admitted_submission(|submission| {
        let genesis = genesis_input(&submission);
        let mut final_store = FinalizationStore {
            mode: CommitMode::New,
            compares: 0,
            observations: 0,
        };
        let mut anchors = Anchors {
            compares: 0,
            observations: 0,
            fail_compare: None,
            replay: false,
        };
        let prepared_finalization = expect_phase(finalization_input(submission).prepare());
        assert_eq!(
            prepared_finalization
                .atom()
                .records()
                .receipt()
                .schema_id()
                .as_str(),
            LOCAL_ADMIN_RECEIPT_V1_SCHEMA
        );
        assert_ne!(
            prepared_finalization
                .atom()
                .records()
                .receipt()
                .schema_id()
                .as_str(),
            DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA
        );
        let pending = expect_phase(prepared_finalization.commit(&mut final_store));
        let anchored = expect_phase(pending.complete(&mut anchors));
        let prepared = expect_phase(anchored.prepare_genesis(genesis));
        assert_ne!(
            prepared.atom().finalization_trust_store(),
            prepared.atom().binding().store()
        );
        assert_eq!(
            prepared.atom().records().receipt().schema_id().as_str(),
            DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA
        );
        assert_ne!(
            prepared.atom().records().receipt().schema_id().as_str(),
            LOCAL_ADMIN_RECEIPT_V1_SCHEMA
        );
        let mut genesis_store = GenesisStore {
            mode: CommitMode::New,
            compares: 0,
            observations: 0,
        };
        let genesis_pending = expect_phase(prepared.commit(&mut genesis_store));
        let completed = expect_phase(genesis_pending.complete(&mut anchors));
        assert_eq!(final_store.compares, 1);
        assert_eq!(genesis_store.compares, 1);
        assert_eq!(completed.genesis_anchor_evidence().len(), 1);
    });
}

#[test]
fn genesis_anchor_failure_retains_progress_for_exact_continuation() {
    admitted_submission(|submission| {
        let genesis = genesis_input(&submission);
        let mut final_store = FinalizationStore {
            mode: CommitMode::New,
            compares: 0,
            observations: 0,
        };
        let mut anchors = Anchors {
            compares: 0,
            observations: 0,
            fail_compare: Some(3),
            replay: false,
        };
        let prepared = expect_phase(finalization_input(submission).prepare());
        let pending = expect_phase(prepared.commit(&mut final_store));
        let anchored = expect_phase(pending.complete(&mut anchors));
        let mut genesis_store = GenesisStore {
            mode: CommitMode::New,
            compares: 0,
            observations: 0,
        };
        let prepared_genesis = expect_phase(anchored.prepare_genesis(genesis));
        let genesis_pending = expect_phase(prepared_genesis.commit(&mut genesis_store));
        let failed = expect_phase_failure(genesis_pending.complete(&mut anchors));
        assert!(matches!(
            failed.reason(),
            DeploymentGenesisAnchorPhaseFailureReason::Advance {
                reason: AnchorAdvanceFailureReason::Port(_),
                ..
            }
        ));
        let failed_owner = failed.owner().clone();
        let completed = expect_phase(failed.into_pending().complete(&mut anchors));
        assert_eq!(anchors.compares, 4);
        assert_eq!(anchors.observations, 1);
        assert_eq!(
            completed.genesis_anchor_evidence(),
            &[AnchorAdvanceEvidence::Advanced]
        );
        assert_eq!(
            failed_owner,
            AntiRollbackAnchorKey::ControlDomain {
                control_store_id: completed
                    .genesis()
                    .candidate()
                    .binding()
                    .store()
                    .store_id()
                    .clone(),
                control_incarnation_id: completed
                    .genesis()
                    .candidate()
                    .binding()
                    .store()
                    .incarnation_id()
                    .clone(),
                checkpoint_domain_id: completed.genesis().candidate().checkpoint_heads()[0]
                    .checkpoint_domain_id()
                    .clone(),
            }
        );
    });
}

#[test]
fn exact_replay_observes_anchors_without_repeating_writes() {
    admitted_submission(|submission| {
        let genesis = genesis_input(&submission);
        let mut final_store = FinalizationStore {
            mode: CommitMode::Replay,
            compares: 0,
            observations: 0,
        };
        let mut anchors = Anchors {
            compares: 0,
            observations: 0,
            fail_compare: None,
            replay: true,
        };
        let input = finalization_input(submission);
        let expected_receipt_ref = input.receipt_ref.clone();
        let expected_receipt_id = input.receipt.admin_receipt_id().clone();
        let expected_receipt_digest = input
            .receipt
            .canonical_admin_receipt_sha256()
            .as_str()
            .to_owned();
        let expected_receipt_bytes = pale_core::canon::to_canonical_string(&input.receipt)
            .unwrap()
            .into_bytes();
        let prepared = expect_phase(input.prepare());
        let retained_receipt = prepared.atom().records().receipt();
        assert_eq!(
            retained_receipt.coordinate().record_ref(),
            &expected_receipt_ref
        );
        assert_eq!(
            retained_receipt.coordinate().record_sha256().as_str(),
            expected_receipt_digest
        );
        assert_eq!(
            retained_receipt.canonical_record_jcs().as_slice(),
            expected_receipt_bytes
        );
        assert_eq!(prepared.atom().replay().receipt_id(), &expected_receipt_id);
        let pending = expect_phase(prepared.commit(&mut final_store));
        let anchored = expect_phase(pending.complete(&mut anchors));
        let mut genesis_store = GenesisStore {
            mode: CommitMode::Replay,
            compares: 0,
            observations: 0,
        };
        let prepared_genesis = expect_phase(anchored.prepare_genesis(genesis));
        let genesis_pending = expect_phase(prepared_genesis.commit(&mut genesis_store));
        let completed = expect_phase(genesis_pending.complete(&mut anchors));
        assert_eq!(anchors.compares, 0);
        assert_eq!(anchors.observations, 3);
        assert_eq!(
            completed.finalization().database_commit_evidence(),
            DeploymentFinalizationCommitEvidence::ExactReplay
        );
        assert_eq!(
            completed.genesis().evidence(),
            DeploymentControlStoreGenesisCommitEvidence::ExactReplay
        );
    });
}

#[test]
fn finalization_receipt_admission_rejects_malformed_unsigned_and_unrelated_material() {
    for case in 0..6 {
        admitted_submission(|submission| {
            let issued = finalization_input(submission);
            let SignedDeploymentFinalizationInput {
                submission,
                command_ref,
                receipt_ref,
                receipt,
            } = issued;
            let wire = match case {
                0 => {
                    let mut value = serde_json::to_value(&receipt).unwrap();
                    value["recorded_at"] = serde_json::json!(82);
                    serde_json::from_value(value).unwrap()
                }
                1 => receipt_wire(&receipt),
                2 => changed_wire(&receipt, "canonical_admin_receipt_sha256", |value| {
                    value["admin_command_ref"] = "unrelated-command".into();
                }),
                3 => changed_wire(&receipt, "canonical_admin_receipt_sha256", |value| {
                    value["deployment_runtime_authority_ref"] = "unrelated-runtime".into();
                }),
                4 => changed_wire(&receipt, "canonical_admin_receipt_sha256", |value| {
                    value["committed_records"][0]["committed_record_sha256"] =
                        "f".repeat(64).into();
                }),
                5 => changed_wire(&receipt, "canonical_admin_receipt_sha256", |value| {
                    value["head_transitions"][0]["previous_head_sha256"] = "f".repeat(64).into();
                }),
                _ => unreachable!(),
            };
            let result = SignedDeploymentFinalizationInput::admit(
                submission,
                DeploymentFinalizationReceiptParameters::new(
                    command_ref,
                    receipt_ref,
                    LocalAdminReceiptIdV1::parse("deployment-finalization-receipt").unwrap(),
                    SchemaDigest::parse("8".repeat(64)).unwrap(),
                    UnixTimeV1::new(80).unwrap(),
                    UnixTimeV1::new(81).unwrap(),
                ),
                wire,
                |_, _, _, _, _| {
                    if case == 1 {
                        Err(InvalidResidentRecordV1::ContextualAdmission {
                            record: "test runtime signature",
                            invariant: "authenticated",
                        })
                    } else {
                        panic!("invalid receipt case {case} reached signature")
                    }
                },
            );
            assert!(result.is_err(), "case {case}");
        });
    }
}

#[test]
fn finalization_receipt_admission_rejects_commit_before_command_issue() {
    admitted_submission(|submission| {
        let issued = finalization_input(submission);
        let SignedDeploymentFinalizationInput {
            submission,
            command_ref,
            receipt_ref,
            receipt,
        } = issued;
        let committed_at = UnixTimeV1::new(59).unwrap();
        let recorded_at = UnixTimeV1::new(81).unwrap();
        let wire = changed_wire(&receipt, "canonical_admin_receipt_sha256", |value| {
            value["committed_at"] = serde_json::json!(59);
        });
        let result = SignedDeploymentFinalizationInput::admit(
            submission,
            DeploymentFinalizationReceiptParameters::new(
                command_ref,
                receipt_ref,
                LocalAdminReceiptIdV1::parse("deployment-finalization-receipt").unwrap(),
                SchemaDigest::parse("8".repeat(64)).unwrap(),
                committed_at,
                recorded_at,
            ),
            wire,
            |_, _, _, _, _| panic!("pre-command commit reached signature verification"),
        );
        assert!(result.is_err());
    });
}

#[test]
fn contextual_failed_receipt_is_rejected_before_command_issue() {
    admitted_submission(|submission| {
        let issued = finalization_input(submission);
        let SignedDeploymentFinalizationInput {
            submission,
            command_ref,
            receipt,
            ..
        } = issued;
        let association = FinalizationReceiptProjection::new(
            submission.construction().finalization(),
            submission.construction().checkpoint(),
            &command_ref,
            submission.command(),
        )
        .unwrap();
        let receipt_id = LocalAdminReceiptIdV1::parse("deployment-finalization-receipt").unwrap();
        let schema_sha256 = SchemaDigest::parse("8".repeat(64)).unwrap();
        let recorded_at = UnixTimeV1::new(59).unwrap();
        let failure_code = LocalAdminFailureCodeV1::parse("rejected").unwrap();
        let wire = changed_wire(&receipt, "canonical_admin_receipt_sha256", |value| {
            value["disposition"] = serde_json::json!("rejected");
            value["committed_records"] = serde_json::json!([]);
            value["head_transitions"] = serde_json::json!([]);
            value["failure_code"] = serde_json::json!("rejected");
            value.as_object_mut().unwrap().remove("committed_at");
            value["recorded_at"] = serde_json::json!(59);
        });
        let selected = submission.construction().finalization().records();
        let result = LocalAdminReceiptV1::admit_contextual_wire(
            wire,
            &LocalAdminReceiptAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema_sha256,
                    signing_authority: association.signing_authority(),
                },
                admin_command_ref: &command_ref,
                admin_command: submission.command(),
                runtime_authority: selected.runtime_authority.1,
                expected_admin_receipt_id: &receipt_id,
                expected_disposition: LocalAdminDispositionV1::Rejected,
                expected_committed_records: &[],
                expected_head_transitions: &[],
                expected_failure_code: Some(&failure_code),
                expected_committed_at: None,
                expected_recorded_at: &recorded_at,
            },
            |_, _, _, _, _| panic!("pre-command failed receipt reached signature verification"),
        );
        assert!(result.is_err());
    });
}

#[test]
fn finalization_receipt_reference_cannot_alias_another_atomic_record() {
    admitted_submission(|submission| {
        let mut input = finalization_input(submission);
        input.receipt_ref = input.command_ref.clone();
        let failure = expect_phase_failure(input.prepare());
        assert!(matches!(
            failure.reason(),
            DeploymentFinalizationPreparationFailureReason::Candidate(
                DeploymentFinalizationCandidateError::ReferenceClosure
            )
        ));
    });
}

#[test]
fn ambiguous_commits_use_one_exact_read_back_per_store() {
    admitted_submission(|submission| {
        let genesis = genesis_input(&submission);
        let mut final_store = FinalizationStore {
            mode: CommitMode::AmbiguousExact,
            compares: 0,
            observations: 0,
        };
        let mut anchors = Anchors {
            compares: 0,
            observations: 0,
            fail_compare: None,
            replay: true,
        };
        let prepared = expect_phase(finalization_input(submission).prepare());
        let pending = expect_phase(prepared.commit(&mut final_store));
        let anchored = expect_phase(pending.complete(&mut anchors));
        let mut genesis_store = GenesisStore {
            mode: CommitMode::AmbiguousExact,
            compares: 0,
            observations: 0,
        };
        let prepared_genesis = expect_phase(anchored.prepare_genesis(genesis));
        let genesis_pending = expect_phase(prepared_genesis.commit(&mut genesis_store));
        let completed = expect_phase(genesis_pending.complete(&mut anchors));
        assert_eq!((final_store.compares, final_store.observations), (1, 1));
        assert_eq!((genesis_store.compares, genesis_store.observations), (1, 1));
        assert_eq!(
            completed.finalization().database_commit_evidence(),
            DeploymentFinalizationCommitEvidence::RecoveredExactAfterAmbiguousCommit
        );
        assert_eq!(
            completed.genesis().evidence(),
            DeploymentControlStoreGenesisCommitEvidence::RecoveredExactAfterAmbiguousCommit
        );
    });
}
