//! Current principal-trust frontier closure tests.

use super::*;
use crate::{
    deployment_finalization_fixture, principal_trust_test_support, producer_custody_fixture,
    resident::{
        principal_trust_recovery::{
            recover_signed_principal_trust_checkpoint_chain, HistoricalPrincipalKeyResolution,
        },
        recover_principal_producer_incarnation_frontier, CurrentPrincipalTrustHead,
        PrincipalProducerIncarnationRecoveryAdmission,
    },
    session_admission_fixture,
};
use pale_core::canon;
use pale_core::records::*;
use pale_ports::*;

struct RecoveredWorld {
    bootstrap: principal_trust_test_support::AdmittedBootstrapFixture,
    checkpoints: RecoveredSignedPrincipalTrustCheckpointChain,
    producers: RecoveredPrincipalProducerIncarnationFrontier,
    finalizations: DeploymentFinalizationSetSnapshot,
    admitted: Vec<AdmittedHistoricalDeploymentFinalizationRecords>,
}

pub(in crate::resident) fn recovered_cold_start_trust_frontier(
    snapshot: DeploymentFinalizationSnapshot,
    finalization: &AdmittedDeploymentFinalization<'_>,
) -> RecoveredCurrentPrincipalTrustFrontier {
    recovered_cold_start_trust_frontier_with_later_checkpoint(snapshot, finalization, false)
}

pub(in crate::resident) fn recovered_cold_start_trust_frontier_with_later_checkpoint(
    snapshot: DeploymentFinalizationSnapshot,
    finalization: &AdmittedDeploymentFinalization<'_>,
    include_later_checkpoint: bool,
) -> RecoveredCurrentPrincipalTrustFrontier {
    let selected = finalization.records();
    let mut checkpoint_values = vec![
        (
            selected.trust.predecessor_ref.clone(),
            selected.trust.predecessor.clone(),
        ),
        (
            selected.trust.successor_ref.clone(),
            selected.trust.successor.clone(),
        ),
    ];
    if include_later_checkpoint {
        let later_ref = ResidentRecordRefV1::parse("cold-start-later-trust-checkpoint").unwrap();
        let later_wire = deployment_finalization_fixture::changed_wire(
            selected.trust.successor,
            "canonical_trust_checkpoint_sha256",
            |value| {
                value["trust_checkpoint_id"] = later_ref.as_str().into();
                value["checkpoint_epoch"] =
                    (selected.trust.successor.checkpoint_epoch().get() + 1).into();
                value["previous_trust_checkpoint_ref"] =
                    selected.trust.successor_ref.as_str().into();
                value["previous_trust_checkpoint_sha256"] = selected
                    .trust
                    .successor
                    .canonical_trust_checkpoint_sha256()
                    .as_str()
                    .into();
                value["issued_at"] = (selected.trust.successor.issued_at().get() + 1).into();
            },
        );
        let later = PrincipalTrustCheckpointV1::admit_successor_wire(
            later_wire,
            selected.trust.successor_ref,
            selected.trust.successor,
            selected.trust.successor.schema_sha256(),
            |_, _| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .unwrap();
        checkpoint_values.push((later_ref, later));
    }
    let records = checkpoint_values
        .iter()
        .enumerate()
        .map(|(index, (reference, checkpoint))| {
            PrincipalTrustCheckpointRecord::new(
                ControlTransactionSequenceV1::new(u64::try_from(index + 1).unwrap()).unwrap(),
                record_projection::record(
                    record_projection::coordinate(
                        reference,
                        checkpoint.canonical_trust_checkpoint_sha256().as_str(),
                    )
                    .unwrap(),
                    checkpoint,
                )
                .unwrap(),
            )
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let (current_ref, current_checkpoint) = checkpoint_values.last().unwrap();
    let successor_coordinate = record_projection::coordinate(
        current_ref,
        current_checkpoint
            .canonical_trust_checkpoint_sha256()
            .as_str(),
    )
    .unwrap();
    let evidence = PrincipalTrustCheckpointSetSnapshot::new(
        snapshot.binding().clone(),
        PrincipalTrustCheckpointHead::new(
            current_checkpoint.principal_id().clone(),
            *current_checkpoint.checkpoint_epoch(),
            successor_coordinate,
        ),
        ControlTransactionSequenceV1::new(u64::try_from(checkpoint_values.len()).unwrap()).unwrap(),
        records,
    )
    .unwrap();
    let mut resolver = |_: &ResidentPrincipalIdV1, _: &ResidentKeyIdV1, _: PrincipalKeyEpochV1| {
        HistoricalPrincipalKeyResolution::Resolved(
            Ed25519PublicKeyV1::parse("A".repeat(43)).unwrap(),
        )
    };
    let mut verifier = |_: &[u8],
                        _: &Ed25519PublicKeyV1,
                        _: &PrincipalTrustCheckpointPreimageV1,
                        _: &PrincipalTrustCheckpointDigest,
                        _: &CanonicalSignatureV1| { Ok(()) };
    let checkpoints = recover_signed_principal_trust_checkpoint_chain(
        evidence,
        &checkpoint_values[0].0,
        &checkpoint_values[0].1,
        selected.trust.predecessor.schema_sha256(),
        PrincipalTrustCheckpointSetReadLimits::new(checkpoint_values.len(), 65_536),
        &mut resolver,
        &mut verifier,
    )
    .unwrap();
    let producers = recover_empty_producers(current_ref, current_checkpoint);
    let admitted_records = AdmittedHistoricalDeploymentFinalizationRecords::new(
        selected.binding.0.clone(),
        selected.binding.1.clone(),
        selected.checkpoint_signer.0.clone(),
        selected.checkpoint_signer.1.clone(),
        selected.runtime_authority.0.clone(),
        selected.runtime_authority.1.clone(),
    );
    let checkpoint_head = finalization.checkpoint_signer_head().clone();
    let runtime_head = finalization.runtime_authority_head().clone();
    let finalizations = DeploymentFinalizationSetSnapshot::new(
        snapshot.binding().clone(),
        snapshot.committed_transaction(),
        vec![snapshot].into_boxed_slice(),
    )
    .unwrap();
    RecoveredCurrentPrincipalTrustFrontier {
        checkpoints,
        producers,
        finalizations: Some(finalizations),
        finalization_contexts: vec![RecoveredDeploymentFinalizationContext {
            records: admitted_records,
            predecessor_checkpoint_index: 0,
            successor_checkpoint_index: 1,
            predecessor_checkpoint_heads: selected.checkpoint_heads.to_vec().into_boxed_slice(),
            predecessor_runtime_heads: selected.runtime_heads.to_vec().into_boxed_slice(),
        }]
        .into_boxed_slice(),
        checkpoint_authorities: vec![RecoveredCheckpointSigningAuthority {
            record_ref: selected.checkpoint_signer.0.clone(),
            authority: selected.checkpoint_signer.1.clone(),
            head: checkpoint_head,
        }]
        .into_boxed_slice(),
        runtime_authorities: vec![RecoveredDeploymentRuntimeAuthority {
            binding_ref: selected.binding.0.clone(),
            binding: selected.binding.1.clone(),
            record_ref: selected.runtime_authority.0.clone(),
            authority: selected.runtime_authority.1.clone(),
            head: runtime_head,
        }]
        .into_boxed_slice(),
    }
}

#[test]
fn bootstrap_frontier_closes_without_requesting_finalization_admission() {
    let (bootstrap, _) = principal_trust_test_support::admitted_bootstrap_fixture();
    let view = bootstrap.admitted.atom_view();
    let (reference, checkpoint) = view.trust_checkpoint_entry();
    let checkpoints = recover_chain(&bootstrap, &[(reference.clone(), checkpoint.clone())]);
    let producers = recover_empty_producers(reference, checkpoint);
    let mut admission = |_: &ControlRecord, _: &ControlRecord, _: &ControlRecord| {
        panic!("bootstrap has no deployment finalization")
    };

    let recovered = recover_current_principal_trust_frontier(
        &bootstrap.admitted,
        checkpoints,
        producers,
        None,
        &mut admission,
    )
    .unwrap();

    assert_eq!(recovered.current_checkpoint().checkpoint(), checkpoint);
    assert_eq!(recovered.checkpoint_authorities().len(), 1);
    assert!(recovered.runtime_authorities().is_empty());
    assert!(recovered.finalizations().is_none());
}

#[test]
fn mixed_producer_and_multiple_finalization_history_closes_in_checkpoint_order() {
    let mut world = recovered_world();
    let expected_finalizations = world.admitted.len();
    let mut calls = 0usize;
    let mut admitted = world.admitted.drain(..);
    let mut admission =
        |binding: &ControlRecord, signer: &ControlRecord, runtime: &ControlRecord| {
            calls += 1;
            let records = admitted.next().expect("one semantic close per call");
            assert_eq!(binding.coordinate().record_ref(), &records.binding_ref);
            assert_eq!(
                signer.coordinate().record_ref(),
                &records.checkpoint_signer_ref
            );
            assert_eq!(
                runtime.coordinate().record_ref(),
                &records.runtime_authority_ref
            );
            Ok(records)
        };

    let recovered = recover_current_principal_trust_frontier(
        &world.bootstrap.admitted,
        world.checkpoints,
        world.producers,
        Some(world.finalizations),
        &mut admission,
    )
    .unwrap();

    assert_eq!(calls, expected_finalizations);
    assert_eq!(recovered.producers().heads().len(), 1);
    assert_eq!(recovered.checkpoint_authorities().len(), 3);
    assert_eq!(recovered.runtime_authorities().len(), 2);
    assert_eq!(recovered.finalization_contexts().len(), 2);
    for (index, context) in recovered.finalization_contexts().iter().enumerate() {
        let physical = &recovered.finalizations().unwrap().finalizations()[index];
        assert_eq!(
            recovered.checkpoints()[context.predecessor_checkpoint_index()].coordinate(),
            physical.predecessor_trust().coordinate()
        );
        assert_eq!(
            recovered.checkpoints()[context.successor_checkpoint_index()].coordinate(),
            physical.successor_trust().coordinate()
        );
    }
    assert!(recovered
        .checkpoint_authorities()
        .windows(2)
        .all(|pair| pair[0].head().checkpoint_signing_authority_id
            < pair[1].head().checkpoint_signing_authority_id));
    assert!(recovered
        .runtime_authorities()
        .windows(2)
        .all(|pair| pair[0].head().runtime_authority_id < pair[1].head().runtime_authority_id));
    for authority in recovered.checkpoint_authorities() {
        assert_eq!(authority.record_ref(), &authority.head().authority_ref);
        assert_eq!(
            authority
                .authority()
                .canonical_checkpoint_signing_authority_sha256(),
            &authority.head().authority_sha256
        );
    }
    for authority in recovered.runtime_authorities() {
        assert_eq!(authority.record_ref(), &authority.head().authority_ref);
        assert_eq!(
            authority.binding_ref(),
            authority.authority().deployment_binding_ref()
        );
        assert_eq!(
            authority.binding().canonical_binding_sha256(),
            authority.authority().deployment_binding_sha256()
        );
    }
}

#[test]
fn missing_surplus_reordered_and_forked_finalizations_fail_before_admission() {
    let cases = ["missing", "surplus", "reordered", "forked"];
    for case in cases {
        let mut world = recovered_world();
        let finalizations = world.finalizations.finalizations().to_vec();
        let selected = match case {
            "missing" => Some(
                DeploymentFinalizationSetSnapshot::new(
                    world.finalizations.binding().clone(),
                    world.finalizations.observed_through(),
                    finalizations[..1].to_vec().into_boxed_slice(),
                )
                .unwrap(),
            ),
            "surplus" => {
                let (bootstrap, _) = principal_trust_test_support::admitted_bootstrap_fixture();
                let view = bootstrap.admitted.atom_view();
                let (reference, checkpoint) = view.trust_checkpoint_entry();
                world.checkpoints =
                    recover_chain(&bootstrap, &[(reference.clone(), checkpoint.clone())]);
                world.producers = recover_empty_producers(reference, checkpoint);
                world.bootstrap = bootstrap;
                Some(world.finalizations.clone())
            }
            "reordered" => Some(
                DeploymentFinalizationSetSnapshot::new(
                    world.finalizations.binding().clone(),
                    world.finalizations.observed_through(),
                    vec![
                        rebuilt_snapshot(&finalizations[1], 3),
                        rebuilt_snapshot(&finalizations[0], 4),
                    ]
                    .into_boxed_slice(),
                )
                .unwrap(),
            ),
            "forked" => Some(
                DeploymentFinalizationSetSnapshot::new(
                    world.finalizations.binding().clone(),
                    world.finalizations.observed_through(),
                    vec![rebuilt_snapshot(&finalizations[1], 3)].into_boxed_slice(),
                )
                .unwrap(),
            ),
            _ => unreachable!(),
        };
        let mut calls = 0usize;
        let mut admission = |_: &ControlRecord, _: &ControlRecord, _: &ControlRecord| {
            calls += 1;
            Ok(world.admitted[0].clone())
        };
        let result = recover_current_principal_trust_frontier(
            &world.bootstrap.admitted,
            world.checkpoints,
            world.producers,
            selected,
            &mut admission,
        );
        assert!(
            matches!(
                (case, result),
                (
                    "missing",
                    Err(PrincipalTrustFrontierRecoveryError::MissingFinalization)
                ) | (
                    "surplus",
                    Err(PrincipalTrustFrontierRecoveryError::SurplusFinalization)
                ) | (
                    "reordered",
                    Err(PrincipalTrustFrontierRecoveryError::FinalizationOrder)
                ) | (
                    "forked",
                    Err(PrincipalTrustFrontierRecoveryError::FinalizationFork)
                )
            ),
            "case {case}"
        );
        assert_eq!(calls, usize::from(case == "missing"));
    }
}

#[test]
fn duplicate_and_absent_successor_finalizations_are_not_consumed() {
    let duplicate_world = recovered_world();
    let physical = duplicate_world.finalizations.finalizations().to_vec();
    let duplicates = DeploymentFinalizationSetSnapshot::new(
        duplicate_world.finalizations.binding().clone(),
        duplicate_world.finalizations.observed_through(),
        vec![
            rebuilt_snapshot(&physical[0], 3),
            rebuilt_snapshot(&physical[0], 4),
        ]
        .into_boxed_slice(),
    )
    .unwrap();
    let mut duplicate_calls = 0usize;
    let mut duplicate_admission = |_: &ControlRecord, _: &ControlRecord, _: &ControlRecord| {
        duplicate_calls += 1;
        Ok(duplicate_world.admitted[0].clone())
    };
    assert!(matches!(
        recover_current_principal_trust_frontier(
            &duplicate_world.bootstrap.admitted,
            duplicate_world.checkpoints,
            duplicate_world.producers,
            Some(duplicates),
            &mut duplicate_admission,
        ),
        Err(PrincipalTrustFrontierRecoveryError::FinalizationOrder)
    ));
    assert_eq!(duplicate_calls, 0);

    let mut absent_world = recovered_world();
    let retained = absent_world.finalizations.clone();
    let retained_chain = recover_chain(
        &absent_world.bootstrap,
        &absent_world
            .checkpoints
            .checkpoints()
            .iter()
            .take(3)
            .map(|checkpoint| {
                (
                    checkpoint.coordinate().record_ref().clone(),
                    checkpoint.checkpoint().clone(),
                )
            })
            .collect::<Vec<_>>(),
    );
    let (producer, _) = producer_custody_fixture::InitialProducerFixture::new();
    absent_world.producers = recover_one_producer(
        &producer,
        retained_chain.current().coordinate().record_ref(),
        retained_chain.current().checkpoint(),
    );
    let mut records = absent_world.admitted.drain(..);
    let mut absent_calls = 0usize;
    let mut absent_admission = |_: &ControlRecord, _: &ControlRecord, _: &ControlRecord| {
        absent_calls += 1;
        Ok(records.next().unwrap())
    };
    assert!(matches!(
        recover_current_principal_trust_frontier(
            &absent_world.bootstrap.admitted,
            retained_chain,
            absent_world.producers,
            Some(retained),
            &mut absent_admission,
        ),
        Err(PrincipalTrustFrontierRecoveryError::SurplusFinalization)
    ));
    assert_eq!(absent_calls, 1);
}

#[test]
fn substituted_finalization_bytes_and_wrong_producer_fence_fail_closed() {
    let mut world = recovered_world();
    let physical = world.finalizations.finalizations().to_vec();
    let mut records = physical[0].records().clone();
    records = DeploymentFinalizationRecords::new(
        records.command().clone(),
        ControlRecord::new(
            records.binding().coordinate().clone(),
            records.binding().schema_id().clone(),
            records.binding().schema_sha256().clone(),
            ControlRecordBytes::new(br#"{"substituted":true}"#.to_vec()).unwrap(),
        ),
        records.checkpoint_signing_authority().clone(),
        records.runtime_authority().clone(),
        records.trust_checkpoint().clone(),
        records.control_checkpoint().clone(),
        records.receipt().clone(),
    )
    .unwrap();
    let substituted = rebuild_snapshot_with_records(&physical[0], 3, records);
    world.finalizations = DeploymentFinalizationSetSnapshot::new(
        world.finalizations.binding().clone(),
        world.finalizations.observed_through(),
        vec![substituted, rebuilt_snapshot(&physical[1], 4)].into_boxed_slice(),
    )
    .unwrap();
    let mut admitted = world.admitted.clone().into_iter();
    let mut admission =
        |_: &ControlRecord, _: &ControlRecord, _: &ControlRecord| Ok(admitted.next().unwrap());
    assert!(matches!(
        recover_current_principal_trust_frontier(
            &world.bootstrap.admitted,
            world.checkpoints.clone(),
            world.producers.clone(),
            Some(world.finalizations),
            &mut admission,
        ),
        Err(PrincipalTrustFrontierRecoveryError::FinalizationRecordMismatch)
    ));

    let view = world.bootstrap.admitted.atom_view();
    let (initial_ref, initial) = view.trust_checkpoint_entry();
    let wrong_producers = recover_empty_producers(initial_ref, initial);
    let mut admission = |_: &ControlRecord, _: &ControlRecord, _: &ControlRecord| {
        panic!("the mismatched producer fence is rejected first")
    };
    assert!(matches!(
        recover_current_principal_trust_frontier(
            &world.bootstrap.admitted,
            world.checkpoints,
            wrong_producers,
            None,
            &mut admission,
        ),
        Err(PrincipalTrustFrontierRecoveryError::EvidenceAssociation)
    ));
}

#[test]
fn substituted_trust_record_fails_before_semantic_finalization_admission() {
    let world = recovered_world();
    let physical = world.finalizations.finalizations();
    let first = &physical[0];
    let substituted_predecessor = ControlRecord::new(
        first.predecessor_trust_record().coordinate().clone(),
        first.predecessor_trust_record().schema_id().clone(),
        first.predecessor_trust_record().schema_sha256().clone(),
        ControlRecordBytes::new(br#"{"substituted":true}"#.to_vec()).unwrap(),
    );
    let substituted = DeploymentFinalizationSnapshot::new(
        first.binding().clone(),
        first.replay().clone(),
        first.admin_authority_record().clone(),
        first.records().clone(),
        first.predecessor_trust().clone(),
        substituted_predecessor,
        first.successor_trust().clone(),
        first.predecessor_control().clone(),
        first.predecessor_control_record().clone(),
        first.successor_control().clone(),
        first.accumulator().clone(),
        first.committed_transaction(),
        first.candidate_identity_sha256().clone(),
    )
    .unwrap();
    let selected = DeploymentFinalizationSetSnapshot::new(
        world.finalizations.binding().clone(),
        world.finalizations.observed_through(),
        vec![substituted, physical[1].clone()].into_boxed_slice(),
    )
    .unwrap();
    let mut calls = 0usize;
    let mut admission = |_: &ControlRecord, _: &ControlRecord, _: &ControlRecord| {
        calls += 1;
        Ok(world.admitted[0].clone())
    };

    assert!(matches!(
        recover_current_principal_trust_frontier(
            &world.bootstrap.admitted,
            world.checkpoints,
            world.producers,
            Some(selected),
            &mut admission,
        ),
        Err(PrincipalTrustFrontierRecoveryError::FinalizationRecordMismatch)
    ));
    assert_eq!(calls, 0);
}

#[test]
fn changed_direct_signer_session_and_unsupported_roots_are_distinct() {
    for (field, value, expected) in [
        (
            "principal_record_ref",
            serde_json::json!("changed-principal-record"),
            "direct",
        ),
        ("issuer_key_epoch", serde_json::json!(2), "signer"),
        ("session_floor_epoch", serde_json::json!(2), "session"),
        (
            "node_delegation_heads_root_sha256",
            serde_json::json!("f".repeat(64)),
            "unsupported",
        ),
    ] {
        let (bootstrap, _) = principal_trust_test_support::admitted_bootstrap_fixture();
        let view = bootstrap.admitted.atom_view();
        let (initial_ref, initial) = view.trust_checkpoint_entry();
        let successor_ref =
            ResidentRecordRefV1::parse(format!("changed-{expected}")).expect("successor ref");
        let successor = changed_successor(initial_ref, initial, &successor_ref, |candidate| {
            candidate[field] = value;
        });
        let checkpoints = recover_chain(
            &bootstrap,
            &[
                (initial_ref.clone(), initial.clone()),
                (successor_ref, successor.clone()),
            ],
        );
        let producers =
            recover_empty_producers(checkpoints.current().coordinate().record_ref(), &successor);
        let mut admission = |_: &ControlRecord, _: &ControlRecord, _: &ControlRecord| {
            panic!("unsupported checkpoint is rejected before finalization admission")
        };
        let result = recover_current_principal_trust_frontier(
            &bootstrap.admitted,
            checkpoints,
            producers,
            None,
            &mut admission,
        );
        assert!(
            matches!(
                (expected, result),
                (
                    "direct",
                    Err(PrincipalTrustFrontierRecoveryError::DirectTrustChange)
                ) | (
                    "signer",
                    Err(PrincipalTrustFrontierRecoveryError::PrincipalSignerChange)
                ) | (
                    "session",
                    Err(PrincipalTrustFrontierRecoveryError::SessionFloorChange)
                ) | (
                    "unsupported",
                    Err(PrincipalTrustFrontierRecoveryError::UnsupportedFrontier)
                )
            ),
            "field {field}"
        );
    }
}

#[test]
fn signer_root_change_without_retained_close_is_not_replayed_or_guessed() {
    let world = recovered_world();
    let final_checkpoint = world.checkpoints.current().clone();
    let chain = recover_chain(
        &world.bootstrap,
        &world
            .checkpoints
            .checkpoints()
            .iter()
            .take(3)
            .map(|checkpoint| {
                (
                    checkpoint.coordinate().record_ref().clone(),
                    checkpoint.checkpoint().clone(),
                )
            })
            .collect::<Vec<_>>(),
    );
    let producers = recover_one_producer(
        &producer_custody_fixture::InitialProducerFixture::new().0,
        chain.current().coordinate().record_ref(),
        chain.current().checkpoint(),
    );
    let mut calls = 0usize;
    let mut admission = |_: &ControlRecord, _: &ControlRecord, _: &ControlRecord| {
        calls += 1;
        panic!("missing custody never invokes semantic admission")
    };
    assert!(matches!(
        recover_current_principal_trust_frontier(
            &world.bootstrap.admitted,
            chain,
            producers,
            None,
            &mut admission,
        ),
        Err(PrincipalTrustFrontierRecoveryError::MissingFinalization)
    ));
    assert_eq!(calls, 0);
    assert_eq!(final_checkpoint.checkpoint().checkpoint_epoch().get(), 4);
}

#[test]
fn authority_successor_is_not_accepted_as_an_initial_finalization_authority() {
    let world = recovered_world();
    let initial = &world.admitted[0];
    let runtime_ref = ResidentRecordRefV1::parse("runtime-rotation-2").unwrap();
    let runtime_wire = deployment_finalization_fixture::changed_wire(
        &initial.runtime_authority,
        "canonical_deployment_runtime_authority_sha256",
        |value| {
            value["authority_epoch"] = 2.into();
            value["previous_authority_ref"] = initial.runtime_authority_ref.as_str().into();
            value["previous_authority_sha256"] = initial
                .runtime_authority
                .canonical_deployment_runtime_authority_sha256()
                .as_str()
                .into();
            value["issued_at"] = 61.into();
        },
    );
    let signer = session_admission_fixture::principal_key(
        initial.runtime_authority.principal_id(),
        initial.runtime_authority.issuer_key_id(),
    );
    let rotated = DeploymentRuntimeAuthorityV1::admit_successor_wire(
        runtime_wire,
        &DeploymentRuntimeAuthorityAdmissionContextV1 {
            expected_schema_sha256: initial.runtime_authority.schema_sha256(),
            deployment_binding_ref: &initial.binding_ref,
            deployment_binding: &initial.binding,
            control_store_id: initial.runtime_authority.control_store_id(),
            control_incarnation_id: initial.runtime_authority.control_incarnation_id(),
            signer,
        },
        &initial.runtime_authority_ref,
        &initial.runtime_authority,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .unwrap();
    let records = AdmittedHistoricalDeploymentFinalizationRecords::new(
        initial.binding_ref.clone(),
        initial.binding.clone(),
        initial.checkpoint_signer_ref.clone(),
        initial.checkpoint_signer.clone(),
        runtime_ref,
        rotated,
    );

    assert!(matches!(
        require_initial_finalization_authorities(&records),
        Err(PrincipalTrustFrontierRecoveryError::AuthorityRotation)
    ));
}

fn recovered_world() -> RecoveredWorld {
    let (producer, _) = producer_custody_fixture::InitialProducerFixture::new();
    let bootstrap_view = producer.fixture.admitted.atom_view();
    let (initial_ref, initial) = bootstrap_view.trust_checkpoint_entry();
    let authorities = bootstrap_view.authorities();
    let mut first = deployment_finalization_fixture::Fixture::from_trust_named(
        &producer.next_trust_ref,
        &producer.next_trust,
        authorities.checkpoint_authority_ref(),
        authorities.checkpoint_authority(),
        "1",
    );
    first.checkpoint_heads = vec![authorities
        .checkpoint_authority()
        .active_head(authorities.checkpoint_authority_ref().clone())
        .unwrap()];
    first.runtime_heads = Vec::new();
    first.refresh_successor(|_| {});
    let mut second = deployment_finalization_fixture::Fixture::from_trust_named(
        &first.next_ref,
        &first.next,
        authorities.checkpoint_authority_ref(),
        authorities.checkpoint_authority(),
        "2",
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

    let checkpoints = recover_chain(
        &producer.fixture,
        &[
            (initial_ref.clone(), initial.clone()),
            (producer.next_trust_ref.clone(), producer.next_trust.clone()),
            (first.next_ref.clone(), first.next.clone()),
            (second.next_ref.clone(), second.next.clone()),
        ],
    );
    let producers = recover_one_producer(&producer, &second.next_ref, &second.next);
    let first_snapshot = finalization_snapshot(&first, 3);
    let second_snapshot = finalization_snapshot(&second, 4);
    let finalizations = DeploymentFinalizationSetSnapshot::new(
        checkpoints.evidence().binding().clone(),
        ControlTransactionSequenceV1::new(4).unwrap(),
        vec![first_snapshot, second_snapshot].into_boxed_slice(),
    )
    .unwrap();
    let admitted = vec![admitted_records(&first), admitted_records(&second)];
    RecoveredWorld {
        bootstrap: producer.fixture,
        checkpoints,
        producers,
        finalizations,
        admitted,
    }
}

fn admitted_records(
    fixture: &deployment_finalization_fixture::Fixture,
) -> AdmittedHistoricalDeploymentFinalizationRecords {
    AdmittedHistoricalDeploymentFinalizationRecords::new(
        fixture.binding_ref.clone(),
        fixture.binding.clone(),
        fixture.signer_ref.clone(),
        fixture.signer.clone(),
        fixture.runtime_ref.clone(),
        fixture.runtime.clone(),
    )
}

pub(in crate::resident) fn recover_chain(
    bootstrap: &principal_trust_test_support::AdmittedBootstrapFixture,
    checkpoints: &[(ResidentRecordRefV1, PrincipalTrustCheckpointV1)],
) -> RecoveredSignedPrincipalTrustCheckpointChain {
    let view = bootstrap.admitted.atom_view();
    let (initial_ref, initial) = view.trust_checkpoint_entry();
    let records = checkpoints
        .iter()
        .enumerate()
        .map(|(index, (reference, checkpoint))| {
            let coordinate = record_projection::coordinate(
                reference,
                checkpoint.canonical_trust_checkpoint_sha256().as_str(),
            )
            .unwrap();
            PrincipalTrustCheckpointRecord::new(
                ControlTransactionSequenceV1::new(u64::try_from(index + 1).unwrap()).unwrap(),
                record_projection::record(coordinate, checkpoint).unwrap(),
            )
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let (current_ref, current) = checkpoints.last().unwrap();
    let current_coordinate = record_projection::coordinate(
        current_ref,
        current.canonical_trust_checkpoint_sha256().as_str(),
    )
    .unwrap();
    let binding = PrincipalTrustStoreBinding::new(
        initial.trust_store_id().clone(),
        initial.trust_store_incarnation_id().clone(),
        initial.principal_id().clone(),
    );
    let evidence = PrincipalTrustCheckpointSetSnapshot::new(
        binding,
        PrincipalTrustCheckpointHead::new(
            current.principal_id().clone(),
            *current.checkpoint_epoch(),
            current_coordinate,
        ),
        ControlTransactionSequenceV1::new(u64::try_from(checkpoints.len()).unwrap()).unwrap(),
        records,
    )
    .unwrap();
    let key = view
        .authorities()
        .genesis()
        .principal_record()
        .root_public_key()
        .clone();
    let mut resolver = |_: &ResidentPrincipalIdV1, _: &ResidentKeyIdV1, _: PrincipalKeyEpochV1| {
        HistoricalPrincipalKeyResolution::Resolved(key.clone())
    };
    let mut verifier = |domain: &[u8],
                        _: &Ed25519PublicKeyV1,
                        _: &PrincipalTrustCheckpointPreimageV1,
                        _: &PrincipalTrustCheckpointDigest,
                        _: &CanonicalSignatureV1| {
        assert_eq!(domain, PRINCIPAL_TRUST_CHECKPOINT_V1_SIGNATURE_DOMAIN);
        Ok(())
    };
    recover_signed_principal_trust_checkpoint_chain(
        evidence,
        initial_ref,
        initial,
        &bootstrap.trust_preimage.schema_sha256,
        PrincipalTrustCheckpointSetReadLimits::new(16, 256 * 1024),
        &mut resolver,
        &mut verifier,
    )
    .unwrap()
}

fn recover_empty_producers(
    checkpoint_ref: &ResidentRecordRefV1,
    checkpoint: &PrincipalTrustCheckpointV1,
) -> RecoveredPrincipalProducerIncarnationFrontier {
    struct Empty;
    impl PrincipalProducerIncarnationReadPort for Empty {
        fn read_by_epoch(
            &mut self,
            _: &PrincipalTrustStoreBinding,
            _: &ProducerEpochV1,
        ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
            unreachable!()
        }
        fn read_head(
            &mut self,
            _: &PrincipalTrustStoreBinding,
            _: &ProducerIdV1,
        ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
            unreachable!()
        }
        fn read_by_allocation(
            &mut self,
            _: &PrincipalTrustSnapshot,
            _: &ControlRecordCoordinate,
        ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
            unreachable!()
        }
        fn read_heads_page(
            &mut self,
            request: &PrincipalProducerIncarnationHeadPageRead,
        ) -> Result<PrincipalProducerIncarnationHeadPage, ControlStoreError> {
            Ok(PrincipalProducerIncarnationHeadPage::new(
                request,
                Vec::new(),
                PrincipalProducerIncarnationHeadPageBoundary::Complete,
            )
            .unwrap())
        }
    }
    impl PrincipalProducerIncarnationRecoveryAdmission for Empty {
        fn admit_allocation(
            &mut self,
            _: ProducerIncarnationWireV1,
            _: &ProducerIncarnationHeadAdmissionStateV1<'_>,
        ) -> Result<ProducerIncarnationV1, InvalidResidentRecordV1> {
            unreachable!()
        }
        fn admit_retirement(
            &mut self,
            _: ProducerIncarnationRetirementWireV1,
            _: &ProducerIncarnationHeadAdmissionStateV1<'_>,
        ) -> Result<ProducerIncarnationRetirementV1, InvalidResidentRecordV1> {
            unreachable!()
        }
    }
    recover_principal_producer_incarnation_frontier(
        &mut Empty,
        trust_binding(checkpoint),
        CurrentPrincipalTrustHead {
            checkpoint_ref,
            checkpoint,
        },
        1,
        &mut Empty,
    )
    .unwrap()
}

pub(in crate::resident) fn recover_one_producer(
    fixture: &producer_custody_fixture::InitialProducerFixture,
    checkpoint_ref: &ResidentRecordRefV1,
    checkpoint: &PrincipalTrustCheckpointV1,
) -> RecoveredPrincipalProducerIncarnationFrontier {
    let coordinate = record_projection::coordinate(
        &fixture.allocation_ref,
        fixture.allocation.canonical_incarnation_sha256().as_str(),
    )
    .unwrap();
    let physical = PrincipalProducerIncarnationRecordSet::new(
        PrincipalProducerIncarnationHead::new(
            fixture.allocation.origin_principal_id().clone(),
            fixture.allocation.producer_id().clone(),
            fixture.allocation.producer_epoch().clone(),
            ProducerIncarnationHeadStateV1::Active,
            coordinate.clone(),
            None,
        )
        .unwrap(),
        None,
        record_projection::record(coordinate, &fixture.allocation).unwrap(),
        None,
    )
    .unwrap();
    struct One {
        physical: PrincipalProducerIncarnationRecordSet,
        semantic: ProducerIncarnationV1,
    }
    impl PrincipalProducerIncarnationReadPort for One {
        fn read_by_epoch(
            &mut self,
            _: &PrincipalTrustStoreBinding,
            _: &ProducerEpochV1,
        ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
            Ok(PrincipalProducerIncarnationReadObservation::Exact(
                Box::new(self.physical.clone()),
            ))
        }
        fn read_head(
            &mut self,
            _: &PrincipalTrustStoreBinding,
            _: &ProducerIdV1,
        ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
            Ok(PrincipalProducerIncarnationReadObservation::Exact(
                Box::new(self.physical.clone()),
            ))
        }
        fn read_by_allocation(
            &mut self,
            _: &PrincipalTrustSnapshot,
            _: &ControlRecordCoordinate,
        ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
            Ok(PrincipalProducerIncarnationReadObservation::Exact(
                Box::new(self.physical.clone()),
            ))
        }
        fn read_heads_page(
            &mut self,
            request: &PrincipalProducerIncarnationHeadPageRead,
        ) -> Result<PrincipalProducerIncarnationHeadPage, ControlStoreError> {
            Ok(PrincipalProducerIncarnationHeadPage::new(
                request,
                vec![PrincipalProducerIncarnationHeadPageEntry::new(
                    PrincipalProducerIncarnationHeadCursor::new(
                        self.physical.head().producer_id().clone(),
                    ),
                    self.physical.clone(),
                )
                .unwrap()],
                PrincipalProducerIncarnationHeadPageBoundary::Complete,
            )
            .unwrap())
        }
    }
    impl PrincipalProducerIncarnationRecoveryAdmission for One {
        fn admit_allocation(
            &mut self,
            wire: ProducerIncarnationWireV1,
            state: &ProducerIncarnationHeadAdmissionStateV1<'_>,
        ) -> Result<ProducerIncarnationV1, InvalidResidentRecordV1> {
            assert!(matches!(
                state,
                ProducerIncarnationHeadAdmissionStateV1::Unallocated
            ));
            assert_eq!(
                canon::to_canonical_string(&wire).unwrap(),
                canon::to_canonical_string(&self.semantic).unwrap()
            );
            Ok(self.semantic.clone())
        }
        fn admit_retirement(
            &mut self,
            _: ProducerIncarnationRetirementWireV1,
            _: &ProducerIncarnationHeadAdmissionStateV1<'_>,
        ) -> Result<ProducerIncarnationRetirementV1, InvalidResidentRecordV1> {
            unreachable!()
        }
    }
    let mut owner = One {
        physical,
        semantic: fixture.allocation.clone(),
    };
    let mut admission = One {
        physical: owner.physical.clone(),
        semantic: owner.semantic.clone(),
    };
    recover_principal_producer_incarnation_frontier(
        &mut owner,
        trust_binding(checkpoint),
        CurrentPrincipalTrustHead {
            checkpoint_ref,
            checkpoint,
        },
        1,
        &mut admission,
    )
    .unwrap()
}

fn trust_binding(checkpoint: &PrincipalTrustCheckpointV1) -> PrincipalTrustStoreBinding {
    PrincipalTrustStoreBinding::new(
        checkpoint.trust_store_id().clone(),
        checkpoint.trust_store_incarnation_id().clone(),
        checkpoint.principal_id().clone(),
    )
}

fn finalization_snapshot(
    fixture: &deployment_finalization_fixture::Fixture,
    transaction: u64,
) -> DeploymentFinalizationSnapshot {
    let records = DeploymentFinalizationRecords::new(
        opaque_record(
            LOCAL_ADMIN_COMMAND_V1_SCHEMA,
            &format!("command-{transaction}"),
            char::from(b'a' + u8::try_from(transaction).unwrap()),
        ),
        semantic_record(
            &fixture.binding_ref,
            fixture.binding.canonical_binding_sha256().as_str(),
            &fixture.binding,
        ),
        semantic_record(
            &fixture.signer_ref,
            fixture
                .signer
                .canonical_checkpoint_signing_authority_sha256()
                .as_str(),
            &fixture.signer,
        ),
        semantic_record(
            &fixture.runtime_ref,
            fixture
                .runtime
                .canonical_deployment_runtime_authority_sha256()
                .as_str(),
            &fixture.runtime,
        ),
        semantic_record(
            &fixture.next_ref,
            fixture.next.canonical_trust_checkpoint_sha256().as_str(),
            &fixture.next,
        ),
        opaque_record(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            &format!("control-{transaction}"),
            char::from(b'j' + u8::try_from(transaction).unwrap()),
        ),
        opaque_record(
            LOCAL_ADMIN_RECEIPT_V1_SCHEMA,
            &format!("receipt-{transaction}"),
            char::from(b'p' + u8::try_from(transaction).unwrap()),
        ),
    )
    .unwrap();
    let predecessor_coordinate = record_projection::coordinate(
        &fixture.prior_ref,
        fixture.prior.canonical_trust_checkpoint_sha256().as_str(),
    )
    .unwrap();
    let successor_coordinate = records.trust_checkpoint().coordinate().clone();
    let predecessor_trust = PrincipalTrustCheckpointHead::new(
        fixture.prior.principal_id().clone(),
        *fixture.prior.checkpoint_epoch(),
        predecessor_coordinate.clone(),
    );
    let successor_trust = PrincipalTrustCheckpointHead::new(
        fixture.next.principal_id().clone(),
        *fixture.next.checkpoint_epoch(),
        successor_coordinate,
    );
    let predecessor_control = control_head(transaction - 1, transaction - 1);
    let successor_control = control_head(transaction, transaction);
    let accumulator = accumulator(
        fixture.prior.trust_store_id(),
        fixture.prior.trust_store_incarnation_id(),
        &predecessor_control,
        &successor_control,
        transaction - 1,
    );
    let admin = opaque_record(
        LOCAL_ADMIN_AUTHORITY_V1_SCHEMA,
        &format!("admin-{transaction}"),
        char::from(b'u' + u8::try_from(transaction).unwrap()),
    );
    DeploymentFinalizationSnapshot::new(
        trust_binding(&fixture.prior),
        DeploymentFinalizationReplayIdentity::new(
            LocalAdminCommandIdV1::parse(format!("command-{transaction}")).unwrap(),
            LocalAdminIdempotencyKeyV1::parse(format!("replay-{transaction}")).unwrap(),
            admin.coordinate().clone(),
            records.command().coordinate().clone(),
            LocalAdminReceiptIdV1::parse(format!("receipt-{transaction}")).unwrap(),
        ),
        admin,
        records,
        predecessor_trust,
        record_projection::record(predecessor_coordinate, &fixture.prior).unwrap(),
        successor_trust,
        predecessor_control.clone(),
        opaque_record_at(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            predecessor_control.coordinate().clone(),
        ),
        successor_control,
        accumulator,
        ControlTransactionSequenceV1::new(transaction).unwrap(),
        ArtifactDigest::parse(format!("{:064x}", transaction)).unwrap(),
    )
    .unwrap()
}

fn rebuilt_snapshot(
    source: &DeploymentFinalizationSnapshot,
    transaction: u64,
) -> DeploymentFinalizationSnapshot {
    rebuild_snapshot_with_records(source, transaction, source.records().clone())
}

fn rebuild_snapshot_with_records(
    source: &DeploymentFinalizationSnapshot,
    transaction: u64,
    records: DeploymentFinalizationRecords,
) -> DeploymentFinalizationSnapshot {
    let predecessor_control = control_head(transaction - 1, transaction - 1);
    let successor_control = control_head(transaction, transaction);
    let records = DeploymentFinalizationRecords::new(
        records.command().clone(),
        records.binding().clone(),
        records.checkpoint_signing_authority().clone(),
        records.runtime_authority().clone(),
        records.trust_checkpoint().clone(),
        opaque_record_at(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            successor_control.coordinate().clone(),
        ),
        records.receipt().clone(),
    )
    .unwrap();
    let accumulator = accumulator(
        source.binding().trust_store_id(),
        source.binding().trust_store_incarnation_id(),
        &predecessor_control,
        &successor_control,
        transaction - 1,
    );
    DeploymentFinalizationSnapshot::new(
        source.binding().clone(),
        DeploymentFinalizationReplayIdentity::new(
            source.replay().admin_command_id().clone(),
            source.replay().idempotency_key().clone(),
            source.admin_authority_record().coordinate().clone(),
            records.command().coordinate().clone(),
            source.replay().receipt_id().clone(),
        ),
        source.admin_authority_record().clone(),
        records,
        source.predecessor_trust().clone(),
        source.predecessor_trust_record().clone(),
        source.successor_trust().clone(),
        predecessor_control.clone(),
        opaque_record_at(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            predecessor_control.coordinate().clone(),
        ),
        successor_control,
        accumulator,
        ControlTransactionSequenceV1::new(transaction).unwrap(),
        source.candidate_identity_sha256().clone(),
    )
    .unwrap()
}

fn semantic_record<T>(reference: &ResidentRecordRefV1, digest: &str, value: &T) -> ControlRecord
where
    T: serde::Serialize + record_projection::ResidentRecordSchema,
{
    record_projection::record(
        record_projection::coordinate(reference, digest).unwrap(),
        value,
    )
    .unwrap()
}

fn opaque_record(schema: &str, reference: &str, digest: char) -> ControlRecord {
    let digest = ArtifactDigest::hash_bytes(format!("{schema}:{reference}:{digest}").as_bytes());
    ControlRecord::new(
        ControlRecordCoordinate::new(
            ResidentRecordRefV1::parse(reference).unwrap(),
            ControlRecordDigest::parse(digest.as_str()).unwrap(),
        ),
        SchemaId::parse(schema).unwrap(),
        SchemaDigest::parse("f".repeat(64)).unwrap(),
        ControlRecordBytes::new(format!(r#"{{"record":"{reference}"}}"#).into_bytes()).unwrap(),
    )
}

fn opaque_record_at(schema: &str, coordinate: ControlRecordCoordinate) -> ControlRecord {
    let body = format!(r#"{{"record":"{}"}}"#, coordinate.record_ref().as_str());
    ControlRecord::new(
        coordinate,
        SchemaId::parse(schema).unwrap(),
        SchemaDigest::parse("f".repeat(64)).unwrap(),
        ControlRecordBytes::new(body.into_bytes()).unwrap(),
    )
}

fn control_head(sequence: u64, transaction: u64) -> ControlCheckpointHead {
    let checkpoint = opaque_record(
        CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
        &format!("control-{sequence}"),
        char::from(b'j' + u8::try_from(sequence).unwrap()),
    );
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("principal-head").unwrap(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(sequence).unwrap(),
        ControlTransactionSequenceV1::new(transaction).unwrap(),
        checkpoint.coordinate().clone(),
    )
}

fn accumulator(
    store_id: &ControlStoreIdV1,
    incarnation_id: &ControlIncarnationIdV1,
    predecessor: &ControlCheckpointHead,
    successor: &ControlCheckpointHead,
    current_transaction: u64,
) -> OperationAccumulatorBatch {
    let store = AuthorityStoreCoordinate::new(store_id.clone(), incarnation_id.clone());
    let preparation = ControlTransactionPreparation::new(
        store.clone(),
        ControlTransactionSequenceV1::new(current_transaction).unwrap(),
    )
    .unwrap();
    let binding = AccumulatorCheckpointBinding::new(
        store,
        ResidentRecordRefV1::parse("checkpoint-profile").unwrap(),
        CheckpointProfileDigest::parse("e".repeat(64)).unwrap(),
        AccumulatorCheckpointPhase::Successor(predecessor.clone()),
        successor.clone(),
        preparation,
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

fn changed_successor(
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &PrincipalTrustCheckpointV1,
    successor_ref: &ResidentRecordRefV1,
    change: impl FnOnce(&mut serde_json::Value),
) -> PrincipalTrustCheckpointV1 {
    PrincipalTrustCheckpointV1::admit_successor_wire(
        deployment_finalization_fixture::changed_wire(
            predecessor,
            "canonical_trust_checkpoint_sha256",
            |candidate| {
                candidate["trust_checkpoint_id"] = successor_ref.as_str().into();
                candidate["checkpoint_epoch"] = (predecessor.checkpoint_epoch().get() + 1).into();
                candidate["previous_trust_checkpoint_ref"] = predecessor_ref.as_str().into();
                candidate["previous_trust_checkpoint_sha256"] = predecessor
                    .canonical_trust_checkpoint_sha256()
                    .as_str()
                    .into();
                candidate["issued_at"] = (predecessor.issued_at().get() + 1).into();
                change(candidate);
            },
        ),
        predecessor_ref,
        predecessor,
        predecessor.schema_sha256(),
        |_, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .unwrap()
}
