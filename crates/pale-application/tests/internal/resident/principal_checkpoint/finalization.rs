//! Finalization construction preserves the admitted records and exact preparation.
//! Synthetic roots test association; native tests prove the tree calculation.

use super::*;
use crate::deployment_finalization_fixture::Fixture;
use crate::resident::AdmittedDeploymentFinalization;
use std::cell::Cell;

#[path = "finalization/submission.rs"]
pub(crate) mod submission;

#[path = "finalization/predecessor.rs"]
mod predecessor;
use predecessor::{anchors, snapshot};

fn profile() -> SupportedPrincipalTrustCheckpointProfile {
    SupportedPrincipalTrustCheckpointProfile::admit(
        ResidentRecordRefV1::parse("source-principal-profile").unwrap(),
        &SupportedPrincipalTrustCheckpointProfile::content_digest().unwrap(),
        SupportedPrincipalTrustCheckpointProfile::canonical_content()
            .unwrap()
            .as_bytes(),
    )
    .unwrap()
}

fn limits() -> AccumulatorPreparationLimits {
    AccumulatorPreparationLimits::new(4, 4096, 4096, 4096, 262_144).unwrap()
}

fn setup() -> (
    Store,
    ControlStateCheckpointPreimageV1,
    fixture::BootstrapRecords,
    Fixture,
) {
    setup_for_deployment(
        "deployment-store-1",
        "deployment-incarnation-1",
        "deployment-binding-finalization-1",
        "runtime-finalization-1",
    )
}

fn setup_for_deployment(
    store_id: &str,
    incarnation_id: &str,
    binding_reference: &str,
    runtime_reference: &str,
) -> (
    Store,
    ControlStateCheckpointPreimageV1,
    fixture::BootstrapRecords,
    Fixture,
) {
    let mut store = Store::default();
    let (_, control, records) = fixture::prepare_with_records(&mut store, |_| {}).unwrap();
    store.transaction = Some(control.committed_transaction_sequence);
    store.request = None;
    store.transaction_reads = 0;
    store.trust_reads = [Ok(Some(snapshot(&records))), Ok(Some(snapshot(&records)))].into();
    let finalization = Fixture::from_trust_named_at_store(
        (&records.trust_ref, &records.trust),
        (&records.signer_ref, &records.signer),
        "1",
        store_id,
        incarnation_id,
        binding_reference,
        runtime_reference,
    );
    (store, control, records, finalization)
}

#[test]
fn construction_retains_four_changes_and_distinct_store_identities() {
    let (mut store, control, records, finalization) = setup();
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
    assert_eq!(prepared.leaf_count().get(), 5);
    assert_eq!(prepared.transaction_sequence().get(), 3);
    assert_eq!(
        prepared
            .changed_keys()
            .iter()
            .map(|key| key.record_class.as_str())
            .collect::<Vec<_>>(),
        [
            "checkpoint-signing-authority-head",
            "deployment-finalization",
            "deployment-runtime-authority-head",
            "principal-trust-head"
        ]
    );
    let request = store.request.as_ref().unwrap();
    assert_eq!(request.basis().store().store_id().as_str(), "trust-store-1");
    assert_ne!(request.basis().store().store_id(), prior.control_store_id());
    assert_eq!(request.state().changes().len(), 4);
    assert_eq!(
        request
            .state()
            .changes()
            .iter()
            .filter(|change| change.predecessor().is_some())
            .count(),
        1
    );
    assert!(request.unresolved().changes().is_empty());
    let preimage = fixture::finalization_preimage(&prepared, &prior_ref, &prior, control.clone());
    let verified = Cell::new(false);
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
            |domain, _, _, _| {
                assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                verified.set(true);
                Ok(())
            },
        )
        .unwrap();
    assert!(verified.get());
    assert_eq!(
        constructed.finalization().records().binding.0,
        &finalization.binding_ref
    );
    assert_eq!(constructed.checkpoint().1.state_root_sha256(), &hash('b'));
    assert_eq!(
        constructed.accumulator().state().successor_root(),
        &hash('b')
    );
}

#[test]
fn changed_checkpoint_cannot_reach_signature_verification() {
    for change in 0..6 {
        let (mut store, control, records, finalization) = setup();
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
        let mut preimage =
            fixture::finalization_preimage(&prepared, &prior_ref, &prior, control.clone());
        match change {
            0 => preimage.state_root_sha256 = hash('c'),
            1 => preimage.unresolved_custody_root_sha256 = hash('d'),
            2 => {
                preimage.changed_leaf_keys.pop();
            }
            3 => preimage.leaf_count = ControlLeafCountV1::new(6).unwrap(),
            4 => {
                preimage.committed_transaction_sequence =
                    ControlTransactionSequenceV1::new(4).unwrap()
            }
            5 => preimage.issued_at = UnixTimeV1::new(59).unwrap(),
            _ => unreachable!(),
        }
        let result = prepared.admit_checkpoint(
            ResidentRecordRefV1::parse("finalization-control-head").unwrap(),
            principal_trust_test_support::signed_wire(
                &preimage,
                "canonical_control_checkpoint_sha256",
                &ControlStateCheckpointDigest::hash_canonical(&preimage).unwrap(),
            ),
            &control.schema_sha256,
            &records.authority(&control),
            |_, _, _, _| panic!("changed construction must fail before verifying its signature"),
        );
        assert!(result.is_err(), "change {change}");
    }
}

#[test]
fn reused_reference_and_signature_failure_do_not_construct_finalization() {
    for reused in [true, false] {
        let (mut store, control, records, finalization) = setup();
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
        let preimage =
            fixture::finalization_preimage(&prepared, &prior_ref, &prior, control.clone());
        let verified = Cell::new(false);
        let result = prepared.admit_checkpoint(
            if reused {
                finalization.binding_ref.clone()
            } else {
                ResidentRecordRefV1::parse("finalization-control-head").unwrap()
            },
            principal_trust_test_support::signed_wire(
                &preimage,
                "canonical_control_checkpoint_sha256",
                &ControlStateCheckpointDigest::hash_canonical(&preimage).unwrap(),
            ),
            &control.schema_sha256,
            &records.authority(&control),
            |_, _, _, _| {
                verified.set(true);
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "test signature",
                    invariant: "invalid signature",
                })
            },
        );
        assert!(result.is_err());
        assert_eq!(verified.get(), !reused);
    }
}

#[test]
fn port_substitution_and_limit_failures_are_not_prepared_results() {
    for response in [Response::DifferentRequest, Response::Limit] {
        let (mut store, control, records, finalization) = setup();
        store.response = response;
        let prior = records.checkpoint(&control);
        let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
        let result = profile().prepare_finalization(
            AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
            (&prior_ref, &prior),
            65_536,
            limits(),
            &mut store,
            &mut anchors(&records, &prior_ref, &prior),
        );
        match response {
            Response::DifferentRequest => {
                assert!(matches!(result, Err(PrincipalCheckpointError::Association)))
            }
            Response::Limit => assert!(matches!(
                result,
                Err(PrincipalCheckpointError::Native(
                    OperationAccumulatorPreparationError::Limit
                ))
            )),
            Response::Exact => unreachable!(),
        }
    }
}

#[test]
fn insufficient_change_budget_fails_before_native_preparation() {
    let (mut store, control, records, finalization) = setup();
    let prior = records.checkpoint(&control);
    let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
    let result = profile().prepare_finalization(
        AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
        (&prior_ref, &prior),
        65_536,
        AccumulatorPreparationLimits::new(3, 4096, 4096, 4096, 262_144).unwrap(),
        &mut store,
        &mut anchors(&records, &prior_ref, &prior),
    );
    assert!(matches!(
        result,
        Err(PrincipalCheckpointError::Preparation(_))
    ));
    assert!(store.request.is_none());
    assert_eq!(store.transaction_reads, 0);
}

#[test]
fn unsupported_predecessors_are_rejected_before_native_reads() {
    for change in 0..5 {
        let (mut store, mut control, records, finalization) = setup();
        let mut selected = profile();
        match change {
            0 => {
                selected = SupportedPrincipalTrustCheckpointProfile::admit(
                    ResidentRecordRefV1::parse("another-profile-reference").unwrap(),
                    &SupportedPrincipalTrustCheckpointProfile::content_digest().unwrap(),
                    SupportedPrincipalTrustCheckpointProfile::canonical_content()
                        .unwrap()
                        .as_bytes(),
                )
                .unwrap()
            }
            1 => control.issued_at = UnixTimeV1::new(61).unwrap(),
            2 => control.unresolved_custody_root_sha256 = hash('c'),
            3 => control.control_schema_version = 2,
            4 => {
                control.leaf_count = ControlLeafCountV1::new(1).unwrap();
                control.changed_leaf_keys.truncate(1);
            }
            _ => unreachable!(),
        }
        let prior = records.checkpoint(&control);
        let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
        let result = selected.prepare_finalization(
            AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
            (&prior_ref, &prior),
            65_536,
            limits(),
            &mut store,
            &mut anchors(&records, &prior_ref, &prior),
        );
        assert!(
            matches!(result, Err(PrincipalCheckpointError::Association)),
            "change {change}"
        );
        assert_eq!(store.transaction_reads, 0);
        assert!(store.request.is_none());
    }
}
