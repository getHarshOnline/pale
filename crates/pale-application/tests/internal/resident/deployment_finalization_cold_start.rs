//! Cold-start laws for transaction-one deployment evidence.

use super::*;
use crate::resident::deployment_finalization::{
    recovery::HistoricalLocalAdminAuthorityResolution,
    tests::recovery::{with_evidence, StoredChange},
};
use crate::resident::{
    principal_control_checkpoint_recovery::tests::recovered_cold_start_control_chain,
    principal_trust_frontier::tests::{
        recover_chain, recover_one_producer, recovered_cold_start_trust_frontier,
        recovered_cold_start_trust_frontier_with_later_checkpoint,
    },
    principal_trust_frontier::AdmittedHistoricalDeploymentFinalizationRecords,
};
use pale_core::records::*;
use pale_ports::*;
use serde_json::json;
use std::cell::Cell;
use std::collections::BTreeMap;

#[derive(Default)]
struct Anchors {
    values: BTreeMap<AntiRollbackAnchorKey, AntiRollbackAnchor>,
    error: bool,
}

impl AntiRollbackAnchorPort for Anchors {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        if self.error {
            Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            ))
        } else {
            Ok(self.values.get(key).cloned())
        }
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        panic!("cold start must not advance an anchor")
    }

    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        panic!("cold start uses a direct read, not a proposed transition")
    }
}

struct ForbiddenAnchors;

impl AntiRollbackAnchorPort for ForbiddenAnchors {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        panic!("anchor evidence must not be read")
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        panic!("cold start must not advance an anchor")
    }

    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        panic!("cold start must not propose an anchor transition")
    }
}

fn coordinate(name: &str, digest: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(name).unwrap(),
        ControlRecordDigest::parse(digest.to_string().repeat(64)).unwrap(),
    )
}

fn trust(epoch: u64, name: &str, digest: char) -> AntiRollbackAnchor {
    PrincipalTrustMinimumAnchor::new(
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
        PrincipalTrustCheckpointEpochV1::new(epoch).unwrap(),
        coordinate(name, digest),
    )
    .into()
}

fn control(sequence: u64, name: &str, digest: char) -> AntiRollbackAnchor {
    ControlDomainAntiRollbackAnchor::new(
        ControlStoreIdV1::parse("principal-control-store").unwrap(),
        ControlIncarnationIdV1::parse("principal-control-incarnation").unwrap(),
        ControlCheckpointDomainIdV1::parse("principal-trust").unwrap(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(sequence).unwrap(),
        coordinate(name, digest),
    )
    .into()
}

fn classify(
    predecessor: AntiRollbackAnchor,
    successor: AntiRollbackAnchor,
    observed: Option<AntiRollbackAnchor>,
    later: Vec<AntiRollbackAnchor>,
) -> Result<bool, DeploymentFinalizationColdStartError> {
    let transition = PreparedAntiRollbackAnchorTransition::successor(predecessor, successor)
        .expect("test transition is exact");
    let mut anchors = Anchors::default();
    if let Some(value) = observed {
        anchors.values.insert(value.key(), value);
    }
    anchor_is_pending(&mut anchors, transition, later.into_iter())
}

fn exact_successor_anchors(
    finalization: &AdmittedDeploymentFinalization<'_>,
    predecessor_control: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    successor_control: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
) -> Anchors {
    let records = finalization.records();
    let trust = trust_transition_anchors_from_records(
        (records.trust.predecessor_ref, records.trust.predecessor),
        (records.trust.successor_ref, records.trust.successor),
    )
    .unwrap();
    let control =
        control_transition_anchors_from_records(predecessor_control, successor_control).unwrap();
    let mut anchors = Anchors::default();
    anchors
        .values
        .insert(trust.key().clone(), trust.next().clone());
    anchors
        .values
        .insert(control.key().clone(), control.next().clone());
    anchors
}

fn finalization_command_value(
    finalization: &AdmittedDeploymentFinalization<'_>,
    successor_control: &ControlStateCheckpointV1,
    context: &LocalAdminCommandAdmissionContextV1<'_>,
    identity: &str,
) -> serde_json::Value {
    let records = finalization.records();
    let candidate = |schema: &str, reference: &ResidentRecordRefV1, digest: &str| {
        json!({
            "candidate_record_schema_id": schema,
            "candidate_record_ref": reference,
            "candidate_record_sha256": digest,
        })
    };
    let trust = records.trust.predecessor;
    let signer = records.checkpoint_signer.1;
    let runtime = records.runtime_authority.1;
    json!({
        "schema_id": LOCAL_ADMIN_COMMAND_V1_SCHEMA,
        "schema_sha256": context.expected_schema_sha256,
        "admin_command_id": format!("cold-start-finalization-{identity}"),
        "operation": "finalize_deployment",
        "admin_authority_ref": context.admin_authority_ref,
        "admin_authority_sha256": context.admin_authority.canonical_admin_authority_sha256(),
        "candidate_records": [
            candidate(DEPLOYMENT_BINDING_V1_SCHEMA, records.binding.0, records.binding.1.canonical_binding_sha256().as_str()),
            candidate(CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA, records.checkpoint_signer.0, signer.canonical_checkpoint_signing_authority_sha256().as_str()),
            candidate(DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA, records.runtime_authority.0, runtime.canonical_deployment_runtime_authority_sha256().as_str()),
            candidate(PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA, records.trust.successor_ref, records.trust.successor.canonical_trust_checkpoint_sha256().as_str()),
        ],
        "expected_heads": [
            {
                "head_kind": {"kind": "principal_trust_checkpoint", "principal_id": trust.principal_id(),
                    "trust_store_id": trust.trust_store_id(), "trust_store_incarnation_id": trust.trust_store_incarnation_id()},
                "expected_head_ref": records.trust.predecessor_ref,
                "expected_head_sha256": trust.canonical_trust_checkpoint_sha256(),
                "expected_head_epoch": trust.checkpoint_epoch(),
            },
            {
                "head_kind": {"kind": "checkpoint_signing_authority", "principal_id": signer.principal_id(),
                    "checkpoint_signing_authority_id": signer.checkpoint_signing_authority_id(),
                    "control_store_id": signer.control_store_id(), "control_incarnation_id": signer.control_incarnation_id(),
                    "scope": signer.scope()},
                "expected_head_ref": null, "expected_head_sha256": null, "expected_head_epoch": null,
            },
            {
                "head_kind": {"kind": "deployment_runtime_authority", "principal_id": runtime.principal_id(),
                    "runtime_authority_id": runtime.runtime_authority_id(),
                    "deployment_binding_ref": runtime.deployment_binding_ref(),
                    "deployment_binding_sha256": runtime.deployment_binding_sha256(),
                    "control_store_id": runtime.control_store_id(), "control_incarnation_id": runtime.control_incarnation_id()},
                "expected_head_ref": null, "expected_head_sha256": null, "expected_head_epoch": null,
            },
            {
                "head_kind": {"kind": "control_checkpoint", "control_store_id": successor_control.control_store_id(),
                    "control_incarnation_id": successor_control.control_incarnation_id(),
                    "checkpoint_domain_id": successor_control.checkpoint_domain_id()},
                "expected_head_ref": successor_control.previous_control_checkpoint_ref(),
                "expected_head_sha256": successor_control.previous_control_checkpoint_sha256(),
                "expected_head_epoch": successor_control.domain_epoch(),
            },
        ],
        "idempotency_key": format!("cold-start-finalize-{identity}"),
        "issued_at": 60,
        "expires_at": 150,
        "signer_identity": context.admin_authority.subject_identity(),
        "signer_key_id": context.admin_authority.subject_key_id(),
        "signer_key_epoch": context.admin_authority.subject_key_epoch(),
    })
}

fn trust_head(
    reference: &ResidentRecordRefV1,
    checkpoint: &PrincipalTrustCheckpointV1,
) -> PrincipalTrustCheckpointHead {
    PrincipalTrustCheckpointHead::new(
        checkpoint.principal_id().clone(),
        *checkpoint.checkpoint_epoch(),
        record_projection::coordinate(
            reference,
            checkpoint.canonical_trust_checkpoint_sha256().as_str(),
        )
        .unwrap(),
    )
}

fn control_head(
    reference: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        *checkpoint.domain_epoch(),
        *checkpoint.domain_sequence(),
        *checkpoint.committed_transaction_sequence(),
        record_projection::coordinate(
            reference,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )
        .unwrap(),
    )
}

fn finalization_accumulator(
    predecessor: &ControlCheckpointHead,
    successor: &ControlCheckpointHead,
    transaction: ControlTransactionSequenceV1,
) -> OperationAccumulatorBatch {
    let store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("trust-store-1").unwrap(),
        ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
    );
    let binding = AccumulatorCheckpointBinding::new(
        store.clone(),
        ResidentRecordRefV1::parse("cold-start-checkpoint-profile").unwrap(),
        CheckpointProfileDigest::parse("e".repeat(64)).unwrap(),
        AccumulatorCheckpointPhase::Successor(predecessor.clone()),
        successor.clone(),
        ControlTransactionPreparation::new(store, transaction).unwrap(),
    )
    .unwrap();
    let zero = ArtifactDigest::parse("0".repeat(64)).unwrap();
    OperationAccumulatorBatch::new(
        binding,
        StateAccumulatorBatch::new(
            zero.clone(),
            ArtifactDigest::parse("b".repeat(64)).unwrap(),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([b'b'; 32]),
                None,
                Some(vec![b'b'; 32]),
                32,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(zero.clone(), zero, Vec::new(), 1).unwrap(),
    )
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn authenticated_finalization_snapshot(
    finalization: &AdmittedDeploymentFinalization<'_>,
    predecessor_control: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    successor_control: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    authority: &LocalAdminAuthorityV1,
    command_schema: &SchemaDigest,
    receipt_schema: &SchemaDigest,
    identity: &str,
    committed_at: u64,
) -> DeploymentFinalizationSnapshot {
    let command_context = LocalAdminCommandAdmissionContextV1 {
        expected_schema_sha256: command_schema,
        trust_checkpoint: finalization.records().trust.predecessor,
        admin_authority_ref: finalization
            .records()
            .trust
            .predecessor
            .local_admin_authority_ref(),
        admin_authority: authority,
        now: UnixTimeV1::new(committed_at).unwrap(),
    };
    let command_value = finalization_command_value(
        finalization,
        successor_control.1,
        &command_context,
        identity,
    );
    let command_wire =
        crate::resident::principal_checkpoint::tests::finalization::submission::wire(command_value);
    let command = LocalAdminCommandV1::admit_authenticated_wire(
        command_wire,
        &command_context,
        |_, _, _, _, _| Ok(()),
    )
    .unwrap();
    let command_ref = ResidentRecordRefV1::parse(format!("cold-start-command-{identity}")).unwrap();
    let receipt_ref = ResidentRecordRefV1::parse(format!("cold-start-receipt-{identity}")).unwrap();
    let receipt_id =
        LocalAdminReceiptIdV1::parse(format!("cold-start-receipt-{identity}")).unwrap();
    let committed_at = UnixTimeV1::new(committed_at).unwrap();
    let projection =
        crate::resident::deployment_finalization::projection::FinalizationReceiptProjection::new(
            finalization,
            successor_control,
            &command_ref,
            &command,
        )
        .unwrap();
    let selected = finalization.records();
    let runtime = selected.runtime_authority.1;
    let receipt = LocalAdminReceiptV1::issue_contextual(
        LocalAdminReceiptPreimageV1 {
            schema_id: SchemaId::parse(LOCAL_ADMIN_RECEIPT_V1_SCHEMA).unwrap(),
            schema_sha256: receipt_schema.clone(),
            admin_receipt_id: receipt_id.clone(),
            admin_command_ref: command_ref.clone(),
            admin_command_sha256: command.canonical_admin_command_sha256().clone(),
            disposition: LocalAdminDispositionV1::Committed,
            committed_records: projection.committed_records().to_vec(),
            head_transitions: projection.head_transitions().to_vec(),
            failure_code: None,
            committed_at: Some(committed_at),
            recorded_at: committed_at,
            deployment_runtime_authority_ref: selected.runtime_authority.0.clone(),
            deployment_runtime_authority_sha256: runtime
                .canonical_deployment_runtime_authority_sha256()
                .clone(),
            issuer_principal_id: runtime.principal_id().clone(),
            issuer_key_id: runtime.subject_key_id().clone(),
            issuer_key_epoch: *runtime.subject_key_epoch(),
        },
        &projection.admission_context(
            finalization,
            &command_ref,
            &command,
            &receipt_id,
            receipt_schema,
            &committed_at,
            &committed_at,
        ),
        |_, _, _| Ok(CanonicalSignatureV1::parse("A".repeat(86)).unwrap()),
    )
    .unwrap();
    let authority_coordinate = record_projection::coordinate(
        selected.trust.predecessor.local_admin_authority_ref(),
        authority.canonical_admin_authority_sha256().as_str(),
    )
    .unwrap();
    let command_coordinate = record_projection::coordinate(
        &command_ref,
        command.canonical_admin_command_sha256().as_str(),
    )
    .unwrap();
    let records = DeploymentFinalizationRecords::new(
        record_projection::record(command_coordinate.clone(), &command).unwrap(),
        record_projection::record(
            record_projection::coordinate(
                selected.binding.0,
                selected.binding.1.canonical_binding_sha256().as_str(),
            )
            .unwrap(),
            selected.binding.1,
        )
        .unwrap(),
        record_projection::record(
            record_projection::coordinate(
                selected.checkpoint_signer.0,
                selected
                    .checkpoint_signer
                    .1
                    .canonical_checkpoint_signing_authority_sha256()
                    .as_str(),
            )
            .unwrap(),
            selected.checkpoint_signer.1,
        )
        .unwrap(),
        record_projection::record(
            record_projection::coordinate(
                selected.runtime_authority.0,
                runtime
                    .canonical_deployment_runtime_authority_sha256()
                    .as_str(),
            )
            .unwrap(),
            runtime,
        )
        .unwrap(),
        record_projection::record(
            record_projection::coordinate(
                selected.trust.successor_ref,
                selected
                    .trust
                    .successor
                    .canonical_trust_checkpoint_sha256()
                    .as_str(),
            )
            .unwrap(),
            selected.trust.successor,
        )
        .unwrap(),
        record_projection::record(
            record_projection::coordinate(
                successor_control.0,
                successor_control
                    .1
                    .canonical_control_checkpoint_sha256()
                    .as_str(),
            )
            .unwrap(),
            successor_control.1,
        )
        .unwrap(),
        record_projection::record_with_schema(
            record_projection::coordinate(
                &receipt_ref,
                receipt.canonical_admin_receipt_sha256().as_str(),
            )
            .unwrap(),
            SchemaId::parse(LOCAL_ADMIN_RECEIPT_V1_SCHEMA).unwrap(),
            receipt_schema.clone(),
            &receipt,
        )
        .unwrap(),
    )
    .unwrap();
    let predecessor_control_head = control_head(predecessor_control.0, predecessor_control.1);
    let successor_control_head = control_head(successor_control.0, successor_control.1);
    DeploymentFinalizationSnapshot::new(
        PrincipalTrustStoreBinding::new(
            selected.trust.predecessor.trust_store_id().clone(),
            selected
                .trust
                .predecessor
                .trust_store_incarnation_id()
                .clone(),
            selected.trust.predecessor.principal_id().clone(),
        ),
        DeploymentFinalizationReplayIdentity::new(
            command.admin_command_id().clone(),
            command.idempotency_key().clone(),
            authority_coordinate.clone(),
            command_coordinate,
            receipt_id,
        ),
        record_projection::record(authority_coordinate, authority).unwrap(),
        records,
        trust_head(selected.trust.predecessor_ref, selected.trust.predecessor),
        record_projection::record(
            record_projection::coordinate(
                selected.trust.predecessor_ref,
                selected
                    .trust
                    .predecessor
                    .canonical_trust_checkpoint_sha256()
                    .as_str(),
            )
            .unwrap(),
            selected.trust.predecessor,
        )
        .unwrap(),
        trust_head(selected.trust.successor_ref, selected.trust.successor),
        predecessor_control_head.clone(),
        record_projection::record(
            predecessor_control_head.coordinate().clone(),
            predecessor_control.1,
        )
        .unwrap(),
        successor_control_head.clone(),
        finalization_accumulator(
            &predecessor_control_head,
            &successor_control_head,
            predecessor_control
                .1
                .committed_transaction_sequence()
                .to_owned(),
        ),
        *successor_control.1.committed_transaction_sequence(),
        ArtifactDigest::hash_bytes(identity.as_bytes()),
    )
    .unwrap()
}

#[test]
fn exact_successor_and_exact_predecessor_have_distinct_meanings() {
    let predecessor = trust(1, "trust-1", '1');
    let successor = trust(2, "trust-2", '2');
    assert!(!classify(
        predecessor.clone(),
        successor.clone(),
        Some(successor.clone()),
        Vec::new()
    )
    .unwrap());
    assert!(classify(
        predecessor.clone(),
        successor,
        Some(predecessor),
        Vec::new()
    )
    .unwrap());
}

#[test]
fn ahead_is_accepted_only_by_exact_later_checkpoint_identity() {
    let predecessor = trust(1, "trust-1", '1');
    let successor = trust(2, "trust-2", '2');
    let later = trust(3, "trust-3", '3');
    assert!(!classify(
        predecessor.clone(),
        successor.clone(),
        Some(later.clone()),
        vec![later]
    )
    .unwrap());
    assert!(matches!(
        classify(
            predecessor,
            successor,
            Some(trust(3, "unadmitted-trust-3", '4')),
            vec![trust(3, "trust-3", '3')]
        ),
        Err(DeploymentFinalizationColdStartError::AnchorEvidence)
    ));
}

#[test]
fn missing_behind_and_same_position_substitution_fail_closed() {
    let predecessor = trust(2, "trust-2", '2');
    let successor = trust(3, "trust-3", '3');
    for observed in [
        None,
        Some(trust(1, "trust-1", '1')),
        Some(trust(3, "substituted-trust-3", '4')),
    ] {
        assert!(matches!(
            classify(predecessor.clone(), successor.clone(), observed, Vec::new()),
            Err(DeploymentFinalizationColdStartError::AnchorEvidence)
        ));
    }
}

#[test]
fn control_anchor_uses_the_same_exact_identity_law() {
    let predecessor = control(4, "control-4", '4');
    let successor = control(5, "control-5", '5');
    let later = control(6, "control-6", '6');
    assert!(!classify(
        predecessor.clone(),
        successor.clone(),
        Some(later.clone()),
        vec![later]
    )
    .unwrap());
    assert!(matches!(
        classify(
            predecessor,
            successor,
            Some(control(6, "other-control-6", '7')),
            vec![control(6, "control-6", '6')]
        ),
        Err(DeploymentFinalizationColdStartError::AnchorEvidence)
    ));
}

#[test]
fn unavailable_anchor_and_nonempty_or_partial_genesis_are_not_absence() {
    let transition = PreparedAntiRollbackAnchorTransition::successor(
        trust(1, "trust-1", '1'),
        trust(2, "trust-2", '2'),
    )
    .unwrap();
    let mut anchors = Anchors {
        error: true,
        ..Anchors::default()
    };
    assert!(matches!(
        anchor_is_pending(&mut anchors, transition, std::iter::empty()),
        Err(DeploymentFinalizationColdStartError::Anchor(_))
    ));
    assert!(require_absent_genesis::<()>(Ok(None)).is_ok());
    assert!(matches!(
        require_absent_genesis(Ok(Some(()))),
        Err(DeploymentFinalizationColdStartError::GenesisPresent)
    ));
    assert!(matches!(
        require_absent_genesis::<()>(Err(ControlStoreError::new(
            pale_ports::ControlStoreErrorKind::RecoveryRequired,
        ))),
        Err(DeploymentFinalizationColdStartError::GenesisRead(_))
    ));
}

#[test]
fn phases_project_only_the_existing_classifier_facts() {
    assert_eq!(
        crate::resident::cold_start::classify_cold_start(cold_start_facts_for_phase(
            DeploymentFinalizationColdStartPhase::FinalizationAnchorsPending,
        )),
        crate::resident::cold_start::ColdStartDisposition::CompleteReceiptNamedAnchorAdvancesOnly
    );
    assert_eq!(
        crate::resident::cold_start::classify_cold_start(cold_start_facts_for_phase(
            DeploymentFinalizationColdStartPhase::FinalizedWithoutGenesis,
        )),
        crate::resident::cold_start::ColdStartDisposition::ResumeInactiveDeploymentGenesisOnly
    );
}

#[test]
fn multiple_deployments_are_canonical_and_duplicate_bindings_fail() {
    let mut coordinates = vec![
        coordinate("deployment-b", 'b'),
        coordinate("deployment-a", 'a'),
    ];
    order_deployments(&mut coordinates, |value| value).unwrap();
    assert_eq!(coordinates[0].record_ref().as_str(), "deployment-a");
    assert_eq!(coordinates[1].record_ref().as_str(), "deployment-b");

    let duplicate = coordinate("deployment-a", 'a');
    let mut coordinates = vec![duplicate.clone(), duplicate];
    assert!(matches!(
        order_deployments(&mut coordinates, |value| value),
        Err(DeploymentFinalizationColdStartError::DuplicateDeployment)
    ));
}

#[test]
fn composer_classifies_two_deployments_independently_and_orders_bindings() {
    let (producer, control_preimages) =
        crate::producer_custody_fixture::InitialProducerFixture::new();
    let bootstrap = producer.fixture.admitted.atom_view();
    let (initial_trust_ref, initial_trust) = bootstrap.trust_checkpoint_entry();
    let (initial_control_ref, initial_control) = bootstrap
        .initial_control_checkpoints()
        .next()
        .expect("bootstrap control checkpoint");
    let authorities = bootstrap.authorities();
    let mut first = crate::deployment_finalization_fixture::Fixture::from_trust_named(
        &producer.next_trust_ref,
        &producer.next_trust,
        authorities.checkpoint_authority_ref(),
        authorities.checkpoint_authority(),
        "z",
    );
    first.checkpoint_heads = vec![authorities
        .checkpoint_authority()
        .active_head(authorities.checkpoint_authority_ref().clone())
        .unwrap()];
    first.runtime_heads = Vec::new();
    first.refresh_successor(|_| {});
    let mut second = crate::deployment_finalization_fixture::Fixture::from_trust_named(
        &first.next_ref,
        &first.next,
        authorities.checkpoint_authority_ref(),
        authorities.checkpoint_authority(),
        "a",
    );
    second.checkpoint_heads = first.checkpoint_heads.clone();
    second
        .checkpoint_heads
        .push(first.signer.active_head(first.signer_ref.clone()).unwrap());
    second.checkpoint_heads.sort_by(|left, right| {
        left.checkpoint_signing_authority_id
            .cmp(&right.checkpoint_signing_authority_id)
    });
    second.runtime_heads = vec![first
        .runtime
        .active_head(first.runtime_ref.clone())
        .unwrap()];
    second.refresh_successor(|_| {});

    let (first_control_ref, first_control) = crate::producer_record_fixture::control_successor(
        control_preimages[0].clone(),
        &producer.next_control_ref,
        &producer.next_control,
        "cold-start-finalization-control-z",
    );
    let (second_control_ref, second_control) = crate::producer_record_fixture::control_successor(
        control_preimages[0].clone(),
        &first_control_ref,
        &first_control,
        "cold-start-finalization-control-a",
    );
    let first_finalization = AdmittedDeploymentFinalization::admit(first.candidate()).unwrap();
    let second_finalization = AdmittedDeploymentFinalization::admit(second.candidate()).unwrap();
    let command_schema = SchemaDigest::parse("d".repeat(64)).unwrap();
    let receipt_schema = SchemaDigest::parse("8".repeat(64)).unwrap();
    let first_snapshot = authenticated_finalization_snapshot(
        &first_finalization,
        (&producer.next_control_ref, &producer.next_control),
        (&first_control_ref, &first_control),
        authorities.local_admin_authority(),
        &command_schema,
        &receipt_schema,
        "z",
        70,
    );
    let second_snapshot = authenticated_finalization_snapshot(
        &second_finalization,
        (&first_control_ref, &first_control),
        (&second_control_ref, &second_control),
        authorities.local_admin_authority(),
        &command_schema,
        &receipt_schema,
        "a",
        71,
    );
    let checkpoints = recover_chain(
        &producer.fixture,
        &[
            (initial_trust_ref.clone(), initial_trust.clone()),
            (producer.next_trust_ref.clone(), producer.next_trust.clone()),
            (first.next_ref.clone(), first.next.clone()),
            (second.next_ref.clone(), second.next.clone()),
        ],
    );
    let producers = recover_one_producer(&producer, &second.next_ref, &second.next);
    let finalization_set = DeploymentFinalizationSetSnapshot::new(
        checkpoints.evidence().binding().clone(),
        *second_control.committed_transaction_sequence(),
        vec![first_snapshot, second_snapshot].into_boxed_slice(),
    )
    .unwrap();
    let mut admitted = vec![
        AdmittedHistoricalDeploymentFinalizationRecords::new(
            first.binding_ref.clone(),
            first.binding.clone(),
            first.signer_ref.clone(),
            first.signer.clone(),
            first.runtime_ref.clone(),
            first.runtime.clone(),
        ),
        AdmittedHistoricalDeploymentFinalizationRecords::new(
            second.binding_ref.clone(),
            second.binding.clone(),
            second.signer_ref.clone(),
            second.signer.clone(),
            second.runtime_ref.clone(),
            second.runtime.clone(),
        ),
    ]
    .into_iter();
    let mut admission =
        |_: &ControlRecord, _: &ControlRecord, _: &ControlRecord| Ok(admitted.next().unwrap());
    let trust =
        crate::resident::principal_trust_frontier::recover_current_principal_trust_frontier(
            &producer.fixture.admitted,
            checkpoints,
            producers,
            Some(finalization_set),
            &mut admission,
        )
        .unwrap();
    let control = recovered_cold_start_control_chain(
        trust.finalizations().unwrap().binding().clone(),
        &[
            (initial_control_ref, initial_control),
            (&producer.next_control_ref, &producer.next_control),
            (&first_control_ref, &first_control),
            (&second_control_ref, &second_control),
        ],
    );
    let trust_key = trust_anchor(trust.checkpoints().last().unwrap()).key();
    let control_key = control_anchor(control.checkpoints().last().unwrap()).key();

    let run = |trust_observed: AntiRollbackAnchor, control_observed: AntiRollbackAnchor| {
        let mut anchors = Anchors::default();
        anchors.values.insert(trust_key.clone(), trust_observed);
        anchors.values.insert(control_key.clone(), control_observed);
        let mut authority_resolver = |_: &ControlRecord, _: &PrincipalTrustCheckpointV1| {
            HistoricalLocalAdminAuthorityResolution::Resolved(Box::new(
                authorities.local_admin_authority().clone(),
            ))
        };
        let mut command_verifier =
            |_: &Ed25519PublicKeyV1,
             _: &[u8],
             _: &LocalAdminCommandPreimageV1,
             _: &LocalAdminCommandDigest,
             _: &CanonicalSignatureV1| { Ok(()) };
        let mut receipt_verifier =
            |_: &Ed25519PublicKeyV1,
             _: &[u8],
             _: &LocalAdminReceiptPreimageV1,
             _: &LocalAdminReceiptDigest,
             _: &CanonicalSignatureV1| { Ok(()) };
        let mut genesis = |_: &DeploymentControlStoreBinding,
                           _: DeploymentControlStoreGenesisReadLimits| {
            Ok::<Option<DeploymentControlStoreGenesisSnapshot>, ControlStoreError>(None)
        };
        recover_deployment_finalization_cold_start(
            &trust,
            &control,
            &command_schema,
            &receipt_schema,
            DeploymentControlStoreGenesisReadLimits::new(2, 16_384, 16),
            &mut authority_resolver,
            &mut command_verifier,
            &mut receipt_verifier,
            &mut anchors,
            &mut genesis,
        )
        .unwrap()
    };

    for recovered in [
        run(
            trust_anchor(trust.checkpoints().last().unwrap()),
            control_anchor(&control.checkpoints()[2]),
        ),
        run(
            trust_anchor(&trust.checkpoints()[2]),
            control_anchor(control.checkpoints().last().unwrap()),
        ),
    ] {
        assert_eq!(recovered.len(), 2);
        assert_eq!(
            recovered[0]
                .binding()
                .deployment_binding()
                .record_ref()
                .as_str(),
            "deployment-binding-finalization-a"
        );
        assert_eq!(
            recovered[0].phase(),
            DeploymentFinalizationColdStartPhase::FinalizationAnchorsPending
        );
        assert_eq!(
            recovered[1]
                .binding()
                .deployment_binding()
                .record_ref()
                .as_str(),
            "deployment-binding-finalization-z"
        );
        assert_eq!(
            recovered[1].phase(),
            DeploymentFinalizationColdStartPhase::FinalizedWithoutGenesis
        );
    }
}

#[test]
fn composer_joins_admitted_owners_before_classifying_one_finalization() {
    with_evidence(
        StoredChange::None,
        |snapshot,
         finalization,
         predecessor_control,
         successor_control,
         command_schema,
         receipt_schema,
         authority| {
            let trust = recovered_cold_start_trust_frontier(snapshot.clone(), finalization);
            let control = recovered_cold_start_control_chain(
                snapshot.binding().clone(),
                &[predecessor_control, successor_control],
            );
            let mut anchors =
                exact_successor_anchors(finalization, predecessor_control, successor_control);
            let mut authority_resolver =
                |_: &pale_ports::ControlRecord, _: &PrincipalTrustCheckpointV1| {
                    HistoricalLocalAdminAuthorityResolution::Resolved(Box::new(authority.clone()))
                };
            let command_calls = Cell::new(0);
            let mut command_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminCommandPreimageV1,
                 _: &pale_core::records::LocalAdminCommandDigest,
                 _: &pale_core::records::CanonicalSignatureV1| {
                    command_calls.set(command_calls.get() + 1);
                    Ok(())
                };
            let receipt_calls = Cell::new(0);
            let mut receipt_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminReceiptPreimageV1,
                 _: &pale_core::records::LocalAdminReceiptDigest,
                 _: &pale_core::records::CanonicalSignatureV1| {
                    receipt_calls.set(receipt_calls.get() + 1);
                    Ok(())
                };
            let genesis_calls = Cell::new(0);
            let mut genesis =
                |_: &DeploymentControlStoreBinding, _: DeploymentControlStoreGenesisReadLimits| {
                    genesis_calls.set(genesis_calls.get() + 1);
                    Ok::<Option<DeploymentControlStoreGenesisSnapshot>, ControlStoreError>(None)
                };

            let recovered = recover_deployment_finalization_cold_start(
                &trust,
                &control,
                command_schema,
                receipt_schema,
                DeploymentControlStoreGenesisReadLimits::new(2, 16_384, 16),
                &mut authority_resolver,
                &mut command_verifier,
                &mut receipt_verifier,
                &mut anchors,
                &mut genesis,
            )
            .unwrap();

            assert_eq!(recovered.len(), 1);
            assert_eq!(
                recovered[0].phase(),
                DeploymentFinalizationColdStartPhase::FinalizedWithoutGenesis
            );
            assert_eq!(command_calls.get(), 1);
            assert_eq!(receipt_calls.get(), 1);
            assert_eq!(genesis_calls.get(), 1);
        },
    );
}

#[test]
fn composer_accepts_an_exact_later_admitted_trust_anchor() {
    with_evidence(
        StoredChange::None,
        |snapshot,
         finalization,
         predecessor_control,
         successor_control,
         command_schema,
         receipt_schema,
         authority| {
            let trust = recovered_cold_start_trust_frontier_with_later_checkpoint(
                snapshot.clone(),
                finalization,
                true,
            );
            let control = recovered_cold_start_control_chain(
                snapshot.binding().clone(),
                &[predecessor_control, successor_control],
            );
            let records = finalization.records();
            let trust_transition = trust_transition_anchors_from_records(
                (records.trust.predecessor_ref, records.trust.predecessor),
                (records.trust.successor_ref, records.trust.successor),
            )
            .unwrap();
            let control_transition =
                control_transition_anchors_from_records(predecessor_control, successor_control)
                    .unwrap();
            let mut anchors = Anchors::default();
            anchors.values.insert(
                trust_transition.key().clone(),
                trust_anchor(trust.checkpoints().last().unwrap()),
            );
            anchors.values.insert(
                control_transition.key().clone(),
                control_transition.next().clone(),
            );
            let mut authority_resolver =
                |_: &pale_ports::ControlRecord, _: &PrincipalTrustCheckpointV1| {
                    HistoricalLocalAdminAuthorityResolution::Resolved(Box::new(authority.clone()))
                };
            let mut command_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminCommandPreimageV1,
                 _: &pale_core::records::LocalAdminCommandDigest,
                 _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
            let mut receipt_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminReceiptPreimageV1,
                 _: &pale_core::records::LocalAdminReceiptDigest,
                 _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
            let mut genesis =
                |_: &DeploymentControlStoreBinding, _: DeploymentControlStoreGenesisReadLimits| {
                    Ok::<Option<DeploymentControlStoreGenesisSnapshot>, ControlStoreError>(None)
                };

            let recovered = recover_deployment_finalization_cold_start(
                &trust,
                &control,
                command_schema,
                receipt_schema,
                DeploymentControlStoreGenesisReadLimits::new(2, 16_384, 16),
                &mut authority_resolver,
                &mut command_verifier,
                &mut receipt_verifier,
                &mut anchors,
                &mut genesis,
            )
            .unwrap();

            assert_eq!(recovered.len(), 1);
            assert_eq!(
                recovered[0].phase(),
                DeploymentFinalizationColdStartPhase::FinalizedWithoutGenesis
            );
        },
    );
}

#[test]
fn composer_rejects_substituted_control_store_binding_before_callbacks() {
    with_evidence(
        StoredChange::None,
        |snapshot,
         finalization,
         predecessor_control,
         successor_control,
         command_schema,
         receipt_schema,
         _| {
            let trust = recovered_cold_start_trust_frontier(snapshot.clone(), finalization);
            let substituted = PrincipalTrustStoreBinding::new(
                snapshot.binding().trust_store_id().clone(),
                ControlIncarnationIdV1::parse("substituted-trust-incarnation").unwrap(),
                snapshot.binding().principal_id().clone(),
            );
            let control = recovered_cold_start_control_chain(
                substituted,
                &[predecessor_control, successor_control],
            );
            let mut authority_resolver =
                |_: &pale_ports::ControlRecord, _: &PrincipalTrustCheckpointV1| {
                    panic!("evidence association must precede authentication")
                };
            let mut command_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminCommandPreimageV1,
                 _: &pale_core::records::LocalAdminCommandDigest,
                 _: &pale_core::records::CanonicalSignatureV1| {
                    panic!("evidence association must precede command verification")
                };
            let mut receipt_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminReceiptPreimageV1,
                 _: &pale_core::records::LocalAdminReceiptDigest,
                 _: &pale_core::records::CanonicalSignatureV1| {
                    panic!("evidence association must precede receipt verification")
                };
            let mut anchors = ForbiddenAnchors;
            let mut genesis =
                |_: &DeploymentControlStoreBinding, _: DeploymentControlStoreGenesisReadLimits| {
                    panic!("evidence association must precede genesis reads")
                };

            assert!(matches!(
                recover_deployment_finalization_cold_start(
                    &trust,
                    &control,
                    command_schema,
                    receipt_schema,
                    DeploymentControlStoreGenesisReadLimits::new(2, 16_384, 16),
                    &mut authority_resolver,
                    &mut command_verifier,
                    &mut receipt_verifier,
                    &mut anchors,
                    &mut genesis,
                ),
                Err(DeploymentFinalizationColdStartError::EvidenceAssociation)
            ));
        },
    );
}

#[test]
fn composer_rejects_a_nonadjacent_control_pair_before_callbacks() {
    with_evidence(
        StoredChange::None,
        |snapshot,
         finalization,
         predecessor_control,
         _successor_control,
         command_schema,
         receipt_schema,
         _| {
            let trust = recovered_cold_start_trust_frontier(snapshot.clone(), finalization);
            let control = recovered_cold_start_control_chain(
                snapshot.binding().clone(),
                &[predecessor_control],
            );
            let mut authority_resolver =
                |_: &pale_ports::ControlRecord, _: &PrincipalTrustCheckpointV1| {
                    panic!("control association must precede authentication")
                };
            let mut command_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminCommandPreimageV1,
                 _: &pale_core::records::LocalAdminCommandDigest,
                 _: &pale_core::records::CanonicalSignatureV1| {
                    panic!("control association must precede command verification")
                };
            let mut receipt_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminReceiptPreimageV1,
                 _: &pale_core::records::LocalAdminReceiptDigest,
                 _: &pale_core::records::CanonicalSignatureV1| {
                    panic!("control association must precede receipt verification")
                };
            let mut anchors = ForbiddenAnchors;
            let mut genesis =
                |_: &DeploymentControlStoreBinding, _: DeploymentControlStoreGenesisReadLimits| {
                    panic!("control association must precede genesis reads")
                };

            assert!(matches!(
                recover_deployment_finalization_cold_start(
                    &trust,
                    &control,
                    command_schema,
                    receipt_schema,
                    DeploymentControlStoreGenesisReadLimits::new(2, 16_384, 16),
                    &mut authority_resolver,
                    &mut command_verifier,
                    &mut receipt_verifier,
                    &mut anchors,
                    &mut genesis,
                ),
                Err(DeploymentFinalizationColdStartError::ControlTransition)
            ));
        },
    );
}

#[test]
fn composer_authenticates_before_reading_anchors_or_genesis() {
    with_evidence(
        StoredChange::None,
        |snapshot,
         finalization,
         predecessor_control,
         successor_control,
         command_schema,
         receipt_schema,
         _| {
            let trust = recovered_cold_start_trust_frontier(snapshot.clone(), finalization);
            let control = recovered_cold_start_control_chain(
                snapshot.binding().clone(),
                &[predecessor_control, successor_control],
            );
            let authority_calls = Cell::new(0);
            let mut authority_resolver =
                |_: &pale_ports::ControlRecord, _: &PrincipalTrustCheckpointV1| {
                    authority_calls.set(authority_calls.get() + 1);
                    HistoricalLocalAdminAuthorityResolution::Unavailable
                };
            let mut command_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminCommandPreimageV1,
                 _: &pale_core::records::LocalAdminCommandDigest,
                 _: &pale_core::records::CanonicalSignatureV1| {
                    panic!("unavailable authority must precede command verification")
                };
            let mut receipt_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminReceiptPreimageV1,
                 _: &pale_core::records::LocalAdminReceiptDigest,
                 _: &pale_core::records::CanonicalSignatureV1| {
                    panic!("unavailable authority must precede receipt verification")
                };
            let mut anchors = ForbiddenAnchors;
            let mut genesis =
                |_: &DeploymentControlStoreBinding, _: DeploymentControlStoreGenesisReadLimits| {
                    panic!("authentication must precede genesis reads")
                };

            assert!(matches!(
                recover_deployment_finalization_cold_start(
                    &trust,
                    &control,
                    command_schema,
                    receipt_schema,
                    DeploymentControlStoreGenesisReadLimits::new(2, 16_384, 16),
                    &mut authority_resolver,
                    &mut command_verifier,
                    &mut receipt_verifier,
                    &mut anchors,
                    &mut genesis,
                ),
                Err(DeploymentFinalizationColdStartError::HistoricalFinalization(
                    HistoricalDeploymentFinalizationRecoveryError::HistoricalAuthorityUnavailable
                ))
            ));
            assert_eq!(authority_calls.get(), 1);
        },
    );
}

#[test]
fn authenticated_finalization_with_exact_anchors_and_no_genesis_is_finalized_without_genesis() {
    with_evidence(
        StoredChange::None,
        |snapshot,
         finalization,
         predecessor_control,
         successor_control,
         command_schema,
         receipt_schema,
         authority| {
            let predecessor_trust = (
                finalization.records().trust.predecessor_ref,
                finalization.records().trust.predecessor,
            );
            let successor_trust = (
                finalization.records().trust.successor_ref,
                finalization.records().trust.successor,
            );
            let trust_transition =
                trust_transition_anchors_from_records(predecessor_trust, successor_trust).unwrap();
            let control_transition =
                control_transition_anchors_from_records(predecessor_control, successor_control)
                    .unwrap();
            let mut anchors = Anchors::default();
            anchors.values.insert(
                trust_transition.key().clone(),
                trust_transition.next().clone(),
            );
            anchors.values.insert(
                control_transition.key().clone(),
                control_transition.next().clone(),
            );
            let mut authority_resolver =
                |_: &pale_ports::ControlRecord, _: &PrincipalTrustCheckpointV1| {
                    HistoricalLocalAdminAuthorityResolution::Resolved(Box::new(authority.clone()))
                };
            let mut command_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminCommandPreimageV1,
                 _: &pale_core::records::LocalAdminCommandDigest,
                 _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
            let mut receipt_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminReceiptPreimageV1,
                 _: &pale_core::records::LocalAdminReceiptDigest,
                 _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
            let mut genesis =
                |_: &DeploymentControlStoreBinding, _: DeploymentControlStoreGenesisReadLimits| {
                    Ok::<Option<DeploymentControlStoreGenesisSnapshot>, ControlStoreError>(None)
                };
            let recovered = recover_one_finalization(
                snapshot,
                finalization,
                predecessor_trust,
                successor_trust,
                predecessor_control,
                successor_control,
                std::iter::empty(),
                std::iter::empty(),
                command_schema,
                receipt_schema,
                DeploymentControlStoreGenesisReadLimits::new(2, 16_384, 16),
                &mut authority_resolver,
                &mut command_verifier,
                &mut receipt_verifier,
                &mut anchors,
                &mut genesis,
            )
            .unwrap();

            assert_eq!(
                recovered.phase(),
                DeploymentFinalizationColdStartPhase::FinalizedWithoutGenesis
            );
            assert_eq!(recovered.finalization().command().expires_at().get(), 75);
            assert_eq!(
                recovered
                    .finalization()
                    .receipt()
                    .committed_at()
                    .unwrap()
                    .get(),
                74
            );
        },
    );
}

#[test]
fn authenticated_finalization_with_exact_predecessor_anchor_is_pending() {
    with_evidence(
        StoredChange::None,
        |snapshot,
         finalization,
         predecessor_control,
         successor_control,
         command_schema,
         receipt_schema,
         authority| {
            let predecessor_trust = (
                finalization.records().trust.predecessor_ref,
                finalization.records().trust.predecessor,
            );
            let successor_trust = (
                finalization.records().trust.successor_ref,
                finalization.records().trust.successor,
            );
            let trust_transition =
                trust_transition_anchors_from_records(predecessor_trust, successor_trust).unwrap();
            let control_transition =
                control_transition_anchors_from_records(predecessor_control, successor_control)
                    .unwrap();
            let mut anchors = Anchors::default();
            anchors.values.insert(
                trust_transition.key().clone(),
                trust_transition.next().clone(),
            );
            anchors.values.insert(
                control_transition.key().clone(),
                control_transition.expected().unwrap().clone(),
            );
            let mut authority_resolver =
                |_: &pale_ports::ControlRecord, _: &PrincipalTrustCheckpointV1| {
                    HistoricalLocalAdminAuthorityResolution::Resolved(Box::new(authority.clone()))
                };
            let mut command_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminCommandPreimageV1,
                 _: &pale_core::records::LocalAdminCommandDigest,
                 _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
            let mut receipt_verifier =
                |_: &pale_core::records::Ed25519PublicKeyV1,
                 _: &[u8],
                 _: &pale_core::records::LocalAdminReceiptPreimageV1,
                 _: &pale_core::records::LocalAdminReceiptDigest,
                 _: &pale_core::records::CanonicalSignatureV1| { Ok(()) };
            let mut genesis =
                |_: &DeploymentControlStoreBinding, _: DeploymentControlStoreGenesisReadLimits| {
                    Ok::<Option<DeploymentControlStoreGenesisSnapshot>, ControlStoreError>(None)
                };
            let recovered = recover_one_finalization(
                snapshot,
                finalization,
                predecessor_trust,
                successor_trust,
                predecessor_control,
                successor_control,
                std::iter::empty(),
                std::iter::empty(),
                command_schema,
                receipt_schema,
                DeploymentControlStoreGenesisReadLimits::new(2, 16_384, 16),
                &mut authority_resolver,
                &mut command_verifier,
                &mut receipt_verifier,
                &mut anchors,
                &mut genesis,
            )
            .unwrap();

            assert_eq!(
                recovered.phase(),
                DeploymentFinalizationColdStartPhase::FinalizationAnchorsPending
            );
        },
    );
}
