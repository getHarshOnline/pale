//! Decision-evidence integrity at the public persistence boundary.

use pale_application::decision_evidence::{
    DecisionEvidenceError, DecisionEvidenceLedger, DecisionEvidenceObservation,
    DecisionLedgerIdentity,
};
use pale_core::engine_wire::{DecisionActivityV1, DecisionDispositionV1};
use pale_core::engine_wire::{
    DecisionEvidenceStageV1, DecisionNativeCoordinateV1, DecisionOccurrenceV1,
    DecisionOperationBindingV1, DecisionTargetV1,
};
use pale_core::records::{
    CompilationInputDigest, ExecutionPathSelectionIdV1, ExecutionPathSelectionV1,
    HarnessActivityIdentity, LifecycleEvent, LogicalRepositoryIdV1, ProfileDigest,
    SessionIdentityDigest, TurnIdentityDigest,
};

struct IdentityFixture {
    compilation: CompilationInputDigest,
    profile: ProfileDigest,
    repository: LogicalRepositoryIdV1,
    path: ExecutionPathSelectionV1,
    activity: DecisionActivityV1,
    occurrence: DecisionOccurrenceV1,
    target: DecisionTargetV1,
    operation: DecisionOperationBindingV1,
}

#[path = "decision_evidence/stages.rs"]
mod stages;

impl IdentityFixture {
    fn new() -> Self {
        Self {
            compilation: CompilationInputDigest::parse("9".repeat(64)).unwrap(),
            profile: ProfileDigest::parse("a".repeat(64)).unwrap(),
            repository: LogicalRepositoryIdV1::parse("repo").unwrap(),
            path: ExecutionPathSelectionV1 {
                id: ExecutionPathSelectionIdV1::parse("direct-claude-code").unwrap(),
                experience_path_id: "direct".into(),
            },
            activity: DecisionActivityV1::Observed {
                activity: HarnessActivityIdentity::Session(
                    SessionIdentityDigest::try_from_host_identity("session").unwrap(),
                ),
            },
            occurrence: DecisionOccurrenceV1 {
                native_event_sha256: pale_core::records::ArtifactDigest::hash_bytes(
                    b"synthetic native event",
                ),
                provider_binding: pale_core::records::ProviderBindingRefV1 {
                    provider_binding_id: "fixture-provider".into(),
                    provider_binding_identity: pale_core::records::ProviderBindingIdentityV1 {
                        provider_binding_sha256: pale_core::records::ArtifactDigest::parse(
                            "d".repeat(64),
                        )
                        .unwrap(),
                    },
                },
                turn: DecisionNativeCoordinateV1::Unobserved {
                    reason: pale_core::records::ResidentReasonV1::parse("fixture exposes no turn")
                        .unwrap(),
                },
                tool_use: DecisionNativeCoordinateV1::Observed {
                    sha256: pale_core::records::ArtifactDigest::parse("e".repeat(64)).unwrap(),
                },
            },
            target: DecisionTargetV1::Command {
                sha256: pale_core::records::ArtifactDigest::hash_bytes(b"synthetic command"),
            },
            operation: DecisionOperationBindingV1::Unresolved {
                reason: pale_core::records::ResidentReasonV1::parse(
                    "fixture supplies no authority lookup",
                )
                .unwrap(),
            },
        }
    }

    fn observation(&self) -> DecisionEvidenceObservation<'_> {
        DecisionEvidenceObservation {
            occurrence: &self.occurrence,
            target: &self.target,
            operation: &self.operation,
            stage: &DecisionEvidenceStageV1::PolicySelected,
            predecessor: None,
        }
    }

    fn identity(&self) -> DecisionLedgerIdentity<'_> {
        DecisionLedgerIdentity {
            compilation_input_digest: &self.compilation,
            profile_digest: &self.profile,
            repository_id: &self.repository,
            execution_path: &self.path,
            activity: &self.activity,
        }
    }
}

#[test]
fn bounded_ring_preserves_chain_and_original_reason_identity() {
    let fixture = IdentityFixture::new();
    let mut ledger = DecisionEvidenceLedger::new(fixture.identity()).unwrap();
    let long_reason = "x".repeat(2_048);
    ledger
        .record(
            fixture.identity(),
            LifecycleEvent::PreToolUse,
            DecisionDispositionV1::Deny,
            Some("Bash"),
            &long_reason,
            fixture.observation(),
        )
        .unwrap();
    let initial = serde_json::to_value(&ledger).unwrap();
    let first = &initial["entries"][0];
    assert_eq!(
        first["reason_sha256"],
        pale_core::sha256_hex(long_reason.as_bytes())
    );
    assert!(first["reason"].as_str().unwrap().len() <= 1_024);
    for index in 0..64 {
        ledger
            .record(
                fixture.identity(),
                LifecycleEvent::PreToolUse,
                DecisionDispositionV1::Deny,
                None,
                &format!("decision {index}"),
                fixture.observation(),
            )
            .unwrap();
    }

    let encoded = serde_json::to_value(&ledger).unwrap();
    let round_trip: DecisionEvidenceLedger = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(round_trip, ledger);
    assert_eq!(encoded["entries"].as_array().unwrap().len(), 64);
    assert_eq!(encoded["entries"][0]["sequence"], 2);
    assert_eq!(encoded["truncation_anchor"]["sequence"], 1);
    assert_eq!(
        encoded["entries"][0]["previous_digest"],
        encoded["truncation_anchor"]["evidence_digest"]
    );
    assert_eq!(encoded["next_sequence"], 66);
    assert_eq!(encoded["compilation_input_digest"], "9".repeat(64));
    assert_eq!(encoded["repository_id"], "repo");
    assert_eq!(encoded["execution_path"]["id"], "direct-claude-code");
    assert!(encoded.get("execution_route").is_none());
}

#[test]
fn truncation_anchor_is_exact_and_advances_with_each_eviction() {
    let fixture = IdentityFixture::new();
    let mut ledger = DecisionEvidenceLedger::new(fixture.identity()).unwrap();
    for index in 0..66 {
        ledger
            .record(
                fixture.identity(),
                LifecycleEvent::PreToolUse,
                DecisionDispositionV1::Deny,
                None,
                &format!("decision {index}"),
                fixture.observation(),
            )
            .unwrap();
    }

    let encoded = serde_json::to_value(&ledger).unwrap();
    assert_eq!(encoded["truncation_anchor"]["sequence"], 2);
    assert_eq!(encoded["entries"][0]["sequence"], 3);
    assert_eq!(
        encoded["entries"][0]["previous_digest"],
        encoded["truncation_anchor"]["evidence_digest"]
    );

    let mut wrong_sequence = encoded.clone();
    wrong_sequence["truncation_anchor"]["sequence"] = 1.into();
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(wrong_sequence).is_err());

    let mut wrong_digest = encoded.clone();
    wrong_digest["truncation_anchor"]["evidence_digest"] = "f".repeat(64).into();
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(wrong_digest).is_err());

    let mut forged_predecessor = encoded;
    forged_predecessor
        .as_object_mut()
        .unwrap()
        .remove("truncation_anchor");
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(forged_predecessor).is_err());
}

#[test]
fn tampering_cross_activity_reuse_and_effect_overclaim_are_rejected() {
    let fixture = IdentityFixture::new();
    let mut ledger = DecisionEvidenceLedger::new(fixture.identity()).unwrap();
    ledger
        .record(
            fixture.identity(),
            LifecycleEvent::PostToolUse,
            DecisionDispositionV1::PostEffectSteer,
            Some("Write"),
            "effect observed",
            fixture.observation(),
        )
        .unwrap();
    let mut encoded = serde_json::to_value(&ledger).unwrap();
    encoded["entries"][0]["reason"] = "different".into();
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(encoded).is_err());

    let other_activity = DecisionActivityV1::Observed {
        activity: HarnessActivityIdentity::Turn(
            TurnIdentityDigest::try_from_host_identity("session").unwrap(),
        ),
    };
    let swapped = DecisionLedgerIdentity {
        activity: &other_activity,
        ..fixture.identity()
    };
    assert!(matches!(
        ledger.record(
            swapped,
            LifecycleEvent::PreToolUse,
            DecisionDispositionV1::Deny,
            None,
            "reason",
            fixture.observation(),
        ),
        Err(DecisionEvidenceError::IdentityMismatch)
    ));

    let other_compilation = CompilationInputDigest::parse("8".repeat(64)).unwrap();
    let swapped = DecisionLedgerIdentity {
        compilation_input_digest: &other_compilation,
        ..fixture.identity()
    };
    assert!(matches!(
        ledger.record(
            swapped,
            LifecycleEvent::PreToolUse,
            DecisionDispositionV1::Deny,
            None,
            "reason",
            fixture.observation(),
        ),
        Err(DecisionEvidenceError::IdentityMismatch)
    ));

    let other_repository = LogicalRepositoryIdV1::parse("other-repo").unwrap();
    let swapped = DecisionLedgerIdentity {
        repository_id: &other_repository,
        ..fixture.identity()
    };
    assert!(matches!(
        ledger.record(
            swapped,
            LifecycleEvent::PreToolUse,
            DecisionDispositionV1::Deny,
            None,
            "reason",
            fixture.observation(),
        ),
        Err(DecisionEvidenceError::IdentityMismatch)
    ));

    let other_path = ExecutionPathSelectionV1 {
        id: ExecutionPathSelectionIdV1::parse("other-path").unwrap(),
        ..fixture.path.clone()
    };
    let swapped = DecisionLedgerIdentity {
        execution_path: &other_path,
        ..fixture.identity()
    };
    assert!(matches!(
        ledger.record(
            swapped,
            LifecycleEvent::PreToolUse,
            DecisionDispositionV1::Deny,
            None,
            "reason",
            fixture.observation(),
        ),
        Err(DecisionEvidenceError::IdentityMismatch)
    ));
    assert!(matches!(
        ledger.record(
            fixture.identity(),
            LifecycleEvent::PostToolUse,
            DecisionDispositionV1::Deny,
            None,
            "cannot claim a completed effect was prevented",
            fixture.observation(),
        ),
        Err(DecisionEvidenceError::DispositionMismatch)
    ));
}

#[test]
fn retained_chain_commits_to_every_ledger_identity_coordinate() {
    let fixture = IdentityFixture::new();
    let mut ledger = DecisionEvidenceLedger::new(fixture.identity()).unwrap();
    ledger
        .record(
            fixture.identity(),
            LifecycleEvent::PreToolUse,
            DecisionDispositionV1::Deny,
            Some("Bash"),
            "identity-bound decision",
            fixture.observation(),
        )
        .unwrap();
    let encoded = serde_json::to_value(&ledger).unwrap();

    let mut tampered = encoded.clone();
    tampered["compilation_input_digest"] = "8".repeat(64).into();
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(tampered).is_err());

    let mut tampered = encoded.clone();
    tampered["profile_digest"] = "b".repeat(64).into();
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(tampered).is_err());

    let mut tampered = encoded.clone();
    tampered["repository_id"] = "other-repo".into();
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(tampered).is_err());

    let mut tampered = encoded.clone();
    tampered["execution_path"]["id"] = "other-path".into();
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(tampered).is_err());

    let mut tampered = encoded.clone();
    tampered["execution_path"]["experience_path_id"] = "other-experience".into();
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(tampered).is_err());

    let mut tampered = encoded.clone();
    tampered["activity"]["activity"]["digest"] = "c".repeat(64).into();
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(tampered).is_err());

    let mut tampered = encoded;
    tampered["activity"]["activity"]["kind"] = "turn".into();
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(tampered).is_err());
}

#[test]
fn debug_exposes_identity_and_counts_but_not_decision_material() {
    let fixture = IdentityFixture::new();
    let mut ledger = DecisionEvidenceLedger::new(fixture.identity()).unwrap();
    ledger
        .record(
            fixture.identity(),
            LifecycleEvent::PreToolUse,
            DecisionDispositionV1::RequestConfirmation,
            Some("secret-tool-name"),
            "secret prompt and authorization token",
            fixture.observation(),
        )
        .unwrap();

    let debug = format!("{ledger:?}");
    assert!(debug.contains("entry_count: 1"));
    assert!(debug.contains("request_confirmation"));
    for secret in ["secret-tool-name", "secret prompt", "authorization token"] {
        assert!(!debug.contains(secret));
    }
}
