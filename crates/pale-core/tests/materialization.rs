//! Structural contracts for exact materialization transitions and observations.

use pale_core::records::{
    ArtifactDigest, BaselineStateV1, MachineLocalEffectTransitionV1,
    MaterializationReceiptTransitionV1, MaterializationStateTransitionV1, ReceiptDigest,
    RunTargetResult, SurfaceOperationObservationV1, SurfaceOperationTransitionV1, UnixFileMode,
};

fn artifact_digest() -> ArtifactDigest {
    ArtifactDigest::parse("a".repeat(64)).unwrap()
}

fn receipt_digest() -> ReceiptDigest {
    ReceiptDigest::parse("b".repeat(64)).unwrap()
}

#[test]
fn unix_file_mode_is_a_bounded_transparent_scalar() {
    assert_eq!(UnixFileMode::new(0).unwrap().get(), 0);
    assert_eq!(UnixFileMode::new(0o7777).unwrap().get(), 0o7777);
    assert!(UnixFileMode::new(0o10_000).is_err());
    assert_eq!(
        serde_json::to_value(UnixFileMode::DEFAULT_FILE).unwrap(),
        serde_json::json!(0o644)
    );
    assert!(serde_json::from_value::<UnixFileMode>(serde_json::json!(0o10_000)).is_err());

    let schema = serde_json::to_value(schemars::schema_for!(UnixFileMode)).unwrap();
    assert_eq!(schema["type"], "integer");
    assert_eq!(schema["maximum"], UnixFileMode::MAX);
}

#[test]
fn baseline_presence_is_structural() {
    let absent = serde_json::json!({ "state": "absent" });
    assert_eq!(
        serde_json::from_value::<BaselineStateV1>(absent).unwrap(),
        BaselineStateV1::Absent {}
    );

    let contradictory = serde_json::json!({
        "state": "absent",
        "sha256": artifact_digest(),
        "mode": 0o644,
    });
    assert!(serde_json::from_value::<BaselineStateV1>(contradictory).is_err());

    let incomplete = serde_json::json!({
        "state": "present",
        "sha256": artifact_digest(),
    });
    assert!(serde_json::from_value::<BaselineStateV1>(incomplete).is_err());
}

#[test]
fn planned_transitions_reject_fields_from_other_variants() {
    let impossible_surface = serde_json::json!({
        "action": "delete",
        "prior_owned_sha256": artifact_digest(),
        "desired_sha256": artifact_digest(),
    });
    assert!(serde_json::from_value::<SurfaceOperationTransitionV1>(impossible_surface).is_err());

    let incomplete_state = serde_json::json!({ "kind": "initialize" });
    assert!(serde_json::from_value::<MaterializationStateTransitionV1>(incomplete_state).is_err());
    let embellished_absence = serde_json::json!({
        "kind": "remove_unowned",
        "prior": {},
    });
    assert!(
        serde_json::from_value::<MaterializationStateTransitionV1>(embellished_absence).is_err()
    );

    let machine_local_adopt = serde_json::json!({
        "action": "adopt",
        "prior": {
            "sha256": artifact_digest(),
            "mode": 0o644,
            "identity": { "device": 1, "inode": 2 },
        },
        "post": { "sha256": artifact_digest(), "mode": 0o644 },
    });
    assert!(serde_json::from_value::<MachineLocalEffectTransitionV1>(machine_local_adopt).is_err());
}

#[test]
fn receipt_and_run_evidence_cannot_encode_correlated_absence() {
    let impossible_receipt = serde_json::json!({
        "action": "delete",
        "pre_sha256": artifact_digest(),
        "post_sha256": artifact_digest(),
    });
    assert!(serde_json::from_value::<SurfaceOperationObservationV1>(impossible_receipt).is_err());

    let success_without_receipt = serde_json::json!({
        "outcome": "succeeded",
        "repository_id": "alpha",
    });
    assert!(serde_json::from_value::<RunTargetResult>(success_without_receipt).is_err());

    let failure_with_receipt = serde_json::json!({
        "outcome": "failed_no_effect",
        "repository_id": "alpha",
        "receipt_digest": receipt_digest(),
    });
    assert!(serde_json::from_value::<RunTargetResult>(failure_with_receipt).is_err());

    let embellished_no_owned_state = serde_json::json!({
        "kind": "no_owned_state",
        "post_lineage_digest": "c".repeat(64),
    });
    assert!(
        serde_json::from_value::<MaterializationReceiptTransitionV1>(embellished_no_owned_state,)
            .is_err()
    );
}
