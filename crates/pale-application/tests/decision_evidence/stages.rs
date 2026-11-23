//! Claimed-stage integrity and native observation, without custody or host admission.

use super::*;
use pale_core::engine_wire::{
    DecisionEmissionOutcomeV1, DecisionEvidenceEntryV1, DecisionHostOutcomeV1,
};
use pale_core::records::{ArtifactDigest, ImmutableRecordReferenceV1, ResidentRecordRefV1};

fn record_stage(
    fixture: &IdentityFixture,
    ledger: &mut DecisionEvidenceLedger,
    stage: &DecisionEvidenceStageV1,
    predecessor: Option<&DecisionEvidenceEntryV1>,
) -> Result<DecisionEvidenceEntryV1, DecisionEvidenceError> {
    ledger.record(
        fixture.identity(),
        LifecycleEvent::PreToolUse,
        DecisionDispositionV1::Deny,
        Some("Bash"),
        "selected policy reason",
        DecisionEvidenceObservation {
            stage,
            predecessor,
            ..fixture.observation()
        },
    )
}

fn emission(
    entry: &DecisionEvidenceEntryV1,
    outcome: DecisionEmissionOutcomeV1,
) -> DecisionEvidenceStageV1 {
    DecisionEvidenceStageV1::ResponseEmission {
        selected_entry_sha256: entry.evidence_digest.clone(),
        response_sha256: ArtifactDigest::hash_bytes(b"synthetic response"),
        outcome,
    }
}

fn acknowledgement(entry: &DecisionEvidenceEntryV1) -> DecisionEvidenceStageV1 {
    DecisionEvidenceStageV1::HostAcknowledged {
        emission_entry_sha256: entry.evidence_digest.clone(),
        witness: ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse("fixture:native-witness").unwrap(),
            record_sha256: ArtifactDigest::hash_bytes(b"synthetic native witness"),
        },
        outcome: DecisionHostOutcomeV1::Prevented,
    }
}

#[test]
fn selected_emitted_and_acknowledged_claims_are_distinct_and_causally_bound() {
    let f = IdentityFixture::new();
    let mut ledger = DecisionEvidenceLedger::new(f.identity()).unwrap();
    let selected = record_stage(
        &f,
        &mut ledger,
        &DecisionEvidenceStageV1::PolicySelected,
        None,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&selected).unwrap()["disposition"],
        "deny"
    );
    assert_eq!(selected.stage, DecisionEvidenceStageV1::PolicySelected);
    assert!(!serde_json::to_string(&selected)
        .unwrap()
        .contains("prevented"));
    let emitted = record_stage(
        &f,
        &mut ledger,
        &emission(&selected, DecisionEmissionOutcomeV1::Emitted),
        Some(&selected),
    )
    .unwrap();
    assert!(!serde_json::to_string(&emitted)
        .unwrap()
        .contains("prevented"));
    let acknowledged =
        record_stage(&f, &mut ledger, &acknowledgement(&emitted), Some(&emitted)).unwrap();
    assert!(matches!(
        acknowledged.stage,
        DecisionEvidenceStageV1::HostAcknowledged { .. }
    ));
    let value = serde_json::to_value(&ledger).unwrap();
    assert_eq!(
        serde_json::from_value::<DecisionEvidenceLedger>(value).unwrap(),
        ledger
    );
    assert!(matches!(
        record_stage(&f, &mut ledger, &acknowledgement(&emitted), Some(&emitted)),
        Err(DecisionEvidenceError::StageReplay)
    ));
    assert!(matches!(
        record_stage(
            &f,
            &mut ledger,
            &emission(&selected, DecisionEmissionOutcomeV1::Emitted),
            Some(&selected)
        ),
        Err(DecisionEvidenceError::StageReplay)
    ));
}

#[test]
fn selection_cannot_be_acknowledged_and_failed_emission_cannot_be_promoted() {
    for outcome in [
        DecisionEmissionOutcomeV1::Failed,
        DecisionEmissionOutcomeV1::Unknown,
    ] {
        let f = IdentityFixture::new();
        let mut ledger = DecisionEvidenceLedger::new(f.identity()).unwrap();
        let selected = record_stage(
            &f,
            &mut ledger,
            &DecisionEvidenceStageV1::PolicySelected,
            None,
        )
        .unwrap();
        assert!(matches!(
            record_stage(
                &f,
                &mut ledger,
                &acknowledgement(&selected),
                Some(&selected)
            ),
            Err(DecisionEvidenceError::StageMismatch)
        ));
        assert!(matches!(
            record_stage(&f, &mut ledger, &emission(&selected, outcome), None),
            Err(DecisionEvidenceError::StageMismatch)
        ));
        let attempted = record_stage(
            &f,
            &mut ledger,
            &emission(&selected, outcome),
            Some(&selected),
        )
        .unwrap();
        let result = record_stage(
            &f,
            &mut ledger,
            &acknowledgement(&attempted),
            Some(&attempted),
        );
        if outcome == DecisionEmissionOutcomeV1::Failed {
            assert!(matches!(result, Err(DecisionEvidenceError::StageMismatch)));
        } else {
            // An asserted later witness can resolve unknown emission. This
            // structural view does not admit that synthetic witness as authority.
            assert!(result.is_ok());
            assert!(matches!(
                attempted.stage,
                DecisionEvidenceStageV1::ResponseEmission {
                    outcome: DecisionEmissionOutcomeV1::Unknown,
                    ..
                }
            ));
        }
    }
}

#[test]
fn changed_occurrence_provider_target_or_operation_cannot_complete_another_selection() {
    let changes: [fn(&mut IdentityFixture); 5] = [
        |f| f.occurrence.native_event_sha256 = ArtifactDigest::hash_bytes(b"other event"),
        |f| f.occurrence.provider_binding.provider_binding_id = "other-provider".into(),
        |f| {
            f.occurrence
                .provider_binding
                .provider_binding_identity
                .provider_binding_sha256 = ArtifactDigest::hash_bytes(b"other provider body")
        },
        |f| {
            f.target = DecisionTargetV1::Command {
                sha256: ArtifactDigest::hash_bytes(b"other command"),
            }
        },
        |f| f.operation = DecisionOperationBindingV1::Unbound,
    ];
    for change in changes {
        let f = IdentityFixture::new();
        let mut ledger = DecisionEvidenceLedger::new(f.identity()).unwrap();
        let selected = record_stage(
            &f,
            &mut ledger,
            &DecisionEvidenceStageV1::PolicySelected,
            None,
        )
        .unwrap();
        let mut other = IdentityFixture::new();
        change(&mut other);
        assert!(matches!(
            record_stage(
                &other,
                &mut ledger,
                &emission(&selected, DecisionEmissionOutcomeV1::Emitted),
                Some(&selected)
            ),
            Err(DecisionEvidenceError::StageMismatch)
        ));
        assert_eq!(
            serde_json::to_value(&ledger).unwrap()["entries"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
}

#[test]
fn complete_new_context_and_stage_are_committed_by_the_entry_digest() {
    let f = IdentityFixture::new();
    let mut ledger = DecisionEvidenceLedger::new(f.identity()).unwrap();
    let selected = record_stage(
        &f,
        &mut ledger,
        &DecisionEvidenceStageV1::PolicySelected,
        None,
    )
    .unwrap();
    let selected_view = serde_json::to_value(&ledger).unwrap();
    record_stage(
        &f,
        &mut ledger,
        &emission(&selected, DecisionEmissionOutcomeV1::Emitted),
        Some(&selected),
    )
    .unwrap();
    let encoded = serde_json::to_value(&ledger).unwrap();
    for pointer in [
        "/entries/0/occurrence/native_event_sha256",
        "/entries/0/occurrence/provider_binding/provider_binding_identity/provider_binding_sha256",
        "/entries/0/occurrence/tool_use/sha256",
        "/entries/0/target/sha256",
        "/entries/1/stage/response_sha256",
        "/entries/1/stage/selected_entry_sha256",
    ] {
        let mut value = if pointer.starts_with("/entries/0/") {
            selected_view.clone()
        } else {
            encoded.clone()
        };
        *value.pointer_mut(pointer).unwrap() = "f".repeat(64).into();
        assert!(
            serde_json::from_value::<DecisionEvidenceLedger>(value).is_err(),
            "{pointer}"
        );
    }
    let mut changed_operation = selected_view;
    changed_operation["entries"][0]["operation"] = serde_json::json!({"kind":"unbound"});
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(changed_operation).is_err());
    let mut value = encoded;
    value["entries"][1]["stage"]["outcome"] = "unknown".into();
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(value).is_err());
}

#[test]
fn file_target_keeps_literal_scope_and_cannot_hide_parent_traversal_or_wrong_digest() {
    for path in [
        "../secret",
        "/secret",
        "a//b",
        "a/./b",
        "a/../b",
        "a\\b",
        "",
    ] {
        let mut f = IdentityFixture::new();
        f.target = DecisionTargetV1::File {
            repository_id: f.repository.clone(),
            path: path.into(),
            sha256: ArtifactDigest::hash_bytes(path.as_bytes()),
        };
        let mut ledger = DecisionEvidenceLedger::new(f.identity()).unwrap();
        assert!(
            matches!(
                record_stage(
                    &f,
                    &mut ledger,
                    &DecisionEvidenceStageV1::PolicySelected,
                    None
                ),
                Err(DecisionEvidenceError::Malformed)
            ),
            "{path}"
        );
    }
    let mut f = IdentityFixture::new();
    f.target = DecisionTargetV1::File {
        repository_id: f.repository.clone(),
        path: "src/main.rs".into(),
        sha256: ArtifactDigest::hash_bytes(b"src/main.rs"),
    };
    let mut ledger = DecisionEvidenceLedger::new(f.identity()).unwrap();
    assert!(record_stage(
        &f,
        &mut ledger,
        &DecisionEvidenceStageV1::PolicySelected,
        None
    )
    .is_ok());
    if let DecisionTargetV1::File { sha256, .. } = &mut f.target {
        *sha256 = ArtifactDigest::hash_bytes(b"another path");
    }
    let mut ledger = DecisionEvidenceLedger::new(f.identity()).unwrap();
    assert!(matches!(
        record_stage(
            &f,
            &mut ledger,
            &DecisionEvidenceStageV1::PolicySelected,
            None
        ),
        Err(DecisionEvidenceError::Malformed)
    ));
}

#[test]
fn predecessor_outside_the_ring_remains_explicit_and_does_not_grant_custody() {
    let f = IdentityFixture::new();
    let mut ledger = DecisionEvidenceLedger::new(f.identity()).unwrap();
    let selected = record_stage(
        &f,
        &mut ledger,
        &DecisionEvidenceStageV1::PolicySelected,
        None,
    )
    .unwrap();
    for _ in 0..64 {
        record_stage(
            &f,
            &mut ledger,
            &DecisionEvidenceStageV1::PolicySelected,
            None,
        )
        .unwrap();
    }
    let round_trip: DecisionEvidenceLedger =
        serde_json::from_value(serde_json::to_value(&ledger).unwrap()).unwrap();
    ledger = round_trip;
    assert!(matches!(
        record_stage(
            &f,
            &mut ledger,
            &emission(&selected, DecisionEmissionOutcomeV1::Emitted),
            None
        ),
        Err(DecisionEvidenceError::StageMismatch)
    ));
    assert!(record_stage(
        &f,
        &mut ledger,
        &emission(&selected, DecisionEmissionOutcomeV1::Emitted),
        Some(&selected)
    )
    .is_ok());
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(
        serde_json::to_value(&ledger).unwrap()
    )
    .is_ok());
}

#[test]
fn native_observation_preserves_original_bytes_domains_and_visible_input_without_file_guesses() {
    use pale_application::hooks::RepositoryHookRoutingV1;
    use pale_application::resident::ResidentHookIngressRequest;
    let f = IdentityFixture::new();
    let routing = RepositoryHookRoutingV1::new(
        LifecycleEvent::PreToolUse,
        pale_core::records::WorkspaceId::parse("fixture-world").unwrap(),
        f.repository.clone(),
        f.occurrence.provider_binding.clone(),
    )
    .unwrap();
    let value = serde_json::json!({"hook_event_name":"PreToolUse","session_id":"native-session","turn_id":"native-turn","tool_use_id":"native-call","tool_name":"Bash","tool_input":{"command":"printf example"}});
    let compact = serde_json::to_vec(&value).unwrap();
    let pretty = serde_json::to_vec_pretty(&value).unwrap();
    let a = ResidentHookIngressRequest::admit(routing.clone(), &compact).unwrap();
    let b = ResidentHookIngressRequest::admit(routing.clone(), &pretty).unwrap();
    let observation = a.decision_occurrence().unwrap();
    assert_eq!(
        observation.native_event_sha256,
        ArtifactDigest::hash_bytes(&compact)
    );
    assert_ne!(
        observation.native_event_sha256,
        b.decision_occurrence().unwrap().native_event_sha256
    );
    assert_eq!(observation.turn, b.decision_occurrence().unwrap().turn);
    assert_eq!(
        observation.tool_use,
        b.decision_occurrence().unwrap().tool_use
    );
    assert_eq!(
        a.decision_target().unwrap(),
        DecisionTargetV1::Command {
            sha256: ArtifactDigest::hash_bytes(b"printf example")
        }
    );
    let DecisionNativeCoordinateV1::Observed { sha256 } = observation.turn else {
        panic!("native turn is present")
    };
    assert_eq!(
        sha256.as_str(),
        TurnIdentityDigest::try_from_host_identity("native-turn")
            .unwrap()
            .as_str()
    );
    let mut value = value;
    value["tool_name"] = "Read".into();
    value["tool_input"] = serde_json::json!({"file_path":"README.md"});
    let read =
        ResidentHookIngressRequest::admit(routing, &serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(matches!(
        read.decision_target().unwrap(),
        DecisionTargetV1::StructuredInput { .. }
    ));
}

#[test]
fn native_response_identity_includes_requested_status_and_both_exact_output_channels() {
    use pale_application::hooks::HookOutput;
    let original = HookOutput {
        stdout: "fixture response".into(),
        stderr: "fixture diagnostic".into(),
        exit_code: 0,
    };
    let digest = original.decision_response_sha256().unwrap();
    assert_eq!(digest, original.clone().decision_response_sha256().unwrap());
    let changes = [
        HookOutput {
            exit_code: 2,
            ..original.clone()
        },
        HookOutput {
            stdout: "fixture response\n".into(),
            ..original.clone()
        },
        HookOutput {
            stderr: "other diagnostic".into(),
            ..original
        },
    ];
    for changed in changes {
        assert_ne!(digest, changed.decision_response_sha256().unwrap());
    }
}

#[test]
fn missing_native_activity_can_be_recorded_without_inventing_a_session() {
    let mut f = IdentityFixture::new();
    f.activity = DecisionActivityV1::Unobserved {
        reason: pale_core::records::ResidentReasonV1::parse("private native visibility limitation")
            .unwrap(),
    };
    let mut ledger = DecisionEvidenceLedger::new(f.identity()).unwrap();
    record_stage(
        &f,
        &mut ledger,
        &DecisionEvidenceStageV1::PolicySelected,
        None,
    )
    .unwrap();
    let value = serde_json::to_value(&ledger).unwrap();
    assert_eq!(value["activity"]["visibility"], "unobserved");
    assert!(value["activity"].get("activity").is_none());
    assert!(!format!("{ledger:?}").contains("private native visibility limitation"));
    assert!(serde_json::from_value::<DecisionEvidenceLedger>(value).is_ok());
}
