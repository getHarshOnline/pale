//! Logical-leaf laws exercised with the existing admitted-observation fixture.

use super::*;
use crate::resident::checkpoint_profile::{
    operation_observation_change, OperationLeafProjectionError, OperationObservationLeafChange,
};

fn project(
    fixture: &OperationFixture,
    checkpoint: &ControlStateCheckpointV1,
) -> OperationObservationLeafChange {
    let append = prepare_operation_observation_append(
        fixture.transition(),
        &|_: &OperationObservationV1| Ok(store_coordinate()),
    )
    .expect("exact admitted append");
    operation_observation_change(&append, checkpoint).expect("logical leaf")
}

#[test]
fn checkpoint_leaf_keeps_identity_across_revision_and_checkpoint_progress() {
    let mut fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let projected = project(&fixture, &checkpoints.predecessor);
    assert_eq!(projected, project(&fixture, &checkpoints.successor));
    assert_eq!(projected.key.logical_key.as_str(), "operation-1");
    assert_eq!(projected.key.record_class.as_str(), "operation-observation");
    // Independently calculated from explicit JCS strings with Node's SHA-256,
    // not by calling the Rust canonicalizer to obtain an expected result.
    assert_eq!(
        projected.key.scope_key_sha256.as_str(),
        "3809218339175154d40e1aab7e4e9b120f1d91c10c80ed5f5f861cace96e6ec9"
    );
    assert_eq!(
        projected.key_sha256.as_str(),
        "3e714ad6247300d97127c966b215ac2efca44d70a83d875d606ca860b127274f"
    );
    assert_ne!(projected.predecessor_sha256, projected.successor_sha256);

    let mut another_reference = OperationFixture::exact();
    another_reference.successor_ref = record_ref("operation/alternate-location");
    let renamed = project(&another_reference, &checkpoints.predecessor);
    assert_eq!(renamed.key, projected.key);
    assert_eq!(renamed.key_sha256, projected.key_sha256);
    assert_eq!(renamed.predecessor_sha256, projected.predecessor_sha256);
    assert_ne!(renamed.successor_sha256, projected.successor_sha256);

    let next = issue_observation(
        &signing_authority(),
        &fixture.handle_ref,
        &fixture.handle,
        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
        OperationObservationRevisionV1::new(3).expect("revision"),
        Some((&fixture.successor_ref, &fixture.successor)),
        OperationObservationStateV1::CancellationRequested {
            request: evidence("cancellation/request", 'c'),
        },
        32,
    );
    fixture.predecessor_ref = fixture.successor_ref;
    fixture.predecessor = fixture.successor;
    fixture.successor_ref = record_ref("operation/observation-1-3");
    fixture.successor = next;
    let continued = project(&fixture, &checkpoints.successor);
    assert_eq!(continued.key, projected.key);
    assert_eq!(continued.key_sha256, projected.key_sha256);
    assert_eq!(continued.predecessor_sha256, projected.successor_sha256);
    assert_ne!(continued.successor_sha256, projected.successor_sha256);
}

#[test]
fn checkpoint_leaf_separates_domain_epoch_and_operation_and_rejects_unlike_store() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let original = project(&fixture, &checkpoints.predecessor);
    let another = project(&OperationFixture::named("2"), &checkpoints.predecessor);
    assert_eq!(another.key.scope_key_sha256, original.key.scope_key_sha256);
    assert_ne!(another.key.logical_key, original.key.logical_key);
    assert_ne!(another.key_sha256, original.key_sha256);
    for change_domain in [false, true] {
        let mut preimage = checkpoint_preimage(1, None);
        if change_domain {
            preimage.checkpoint_domain_id =
                ControlCheckpointDomainIdV1::parse("operation:principal-2").expect("domain");
        } else {
            preimage.domain_epoch = ControlDomainEpochV1::new(2).expect("epoch");
        }
        let checkpoint = admit_initial_checkpoint(&preimage);
        let changed = project(&fixture, &checkpoint);
        assert_eq!(changed.key.logical_key, original.key.logical_key);
        assert_ne!(changed.key.scope_key_sha256, original.key.scope_key_sha256);
        assert_ne!(changed.key_sha256, original.key_sha256);
    }

    for change_store in [false, true] {
        let mut preimage = checkpoint_preimage(1, None);
        if change_store {
            preimage.control_store_id = ControlStoreIdV1::parse("foreign-store").expect("store");
        } else {
            preimage.control_incarnation_id =
                ControlIncarnationIdV1::parse("foreign-incarnation").expect("incarnation");
        }
        let checkpoint = admit_initial_checkpoint(&preimage);
        let append = prepare_operation_observation_append(
            fixture.transition(),
            &|_: &OperationObservationV1| Ok(store_coordinate()),
        )
        .expect("admitted append");
        assert!(matches!(
            operation_observation_change(&append, &checkpoint),
            Err(OperationLeafProjectionError::Store)
        ));
    }
}

#[test]
fn checkpoint_leaf_retains_terminal_disposition_instead_of_deleting_the_operation() {
    let mut fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let active = project(&fixture, &checkpoints.predecessor);
    fixture.successor = issue_observation(
        &signing_authority(),
        &fixture.handle_ref,
        &fixture.handle,
        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
        OperationObservationRevisionV1::new(2).expect("revision"),
        Some((&fixture.predecessor_ref, &fixture.predecessor)),
        OperationObservationStateV1::Failed {
            result: None,
            diagnostic: evidence("diagnostic/failure", 'a'),
        },
        31,
    );
    let terminal = project(&fixture, &checkpoints.predecessor);
    assert_eq!(terminal.key, active.key);
    assert_eq!(terminal.key_sha256, active.key_sha256);
    assert_eq!(terminal.predecessor_sha256, active.predecessor_sha256);
    assert_ne!(terminal.successor_sha256, active.successor_sha256);
    assert_ne!(terminal.successor_sha256, artifact_digest('0'));
}

#[test]
fn checkpoint_leaf_unicode_identity_and_maximum_epoch_match_independent_vectors() {
    let fixture = OperationFixture::with_handle_identity("unicode", "opération-雪", 120);
    let mut preimage = checkpoint_preimage(1, None);
    preimage.domain_epoch =
        ControlDomainEpochV1::new(pale_core::canon::JSON_SAFE_INTEGER_MAX).expect("last epoch");
    let checkpoint = admit_initial_checkpoint(&preimage);
    let projected = project(&fixture, &checkpoint);
    assert_eq!(projected.key.logical_key.as_str(), "opération-雪");
    assert_eq!(
        projected.key.scope_key_sha256.as_str(),
        "36d07af903bfe5edc27e58bdb8d347dd8e269ff58509fe71ced2e874e52901fb"
    );
    assert_eq!(
        projected.key_sha256.as_str(),
        "a5051d40b2809020fa11b3bbcfc572daa5926b13671d7b687a78761c8f70adb8"
    );
}
