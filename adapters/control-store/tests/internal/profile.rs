//! Causal admission-classification tests for unresolved storage profiles.

use super::*;

fn sqlite_profile(status: &str, complete: bool) -> serde_json::Value {
    serde_json::json!({
        "journal_mode": "wal",
        "synchronous": complete.then_some("full"),
        "fullfsync": complete.then_some(true),
        "checkpoint_fullfsync": complete.then_some(true),
        "compile_options_sha256": complete.then(|| "a".repeat(64)),
        "checkpoint_mode": complete.then_some("truncate"),
        "wal_autocheckpoint_frames": complete.then_some(1),
        "durability_grouping": complete.then_some("per_record"),
        "foreign_keys": true,
        "trusted_schema": false,
        "measurement_status": status,
    })
}

fn authority_profile(status: &str, complete: bool) -> serde_json::Value {
    serde_json::json!({
        "frame_encoding": complete.then_some("measurement-candidate-frame"),
        "corruption_checksum": complete.then_some("measurement-candidate-checksum"),
        "corruption_checksum_bytes": complete.then_some(32),
        "maximum_entry_bytes": complete.then_some(4096),
        "maximum_checkpoint_bytes": complete.then_some(65536),
        "maximum_checkpoint_candidates": complete.then_some(4),
        "maximum_tail_entries": complete.then_some(4096),
        "checkpoint_cadence": complete.then_some("measurement-candidate-cadence"),
        "retention_policy": complete.then_some("measurement-candidate-retention"),
        "compaction_policy": complete.then_some("measurement-candidate-compaction"),
        "measurement_status": status,
    })
}

#[test]
fn measurement_status_rejects_unknown_spellings() {
    assert!(serde_json::from_value::<SqliteProfileV1>(sqlite_profile("ready-ish", false)).is_err());
}

#[test]
fn only_the_exact_unresolved_sqlite_frontier_is_coherent() {
    let incomplete_selected: SqliteProfileV1 =
        serde_json::from_value(sqlite_profile("selected", false)).unwrap();
    assert!(!incomplete_selected.is_selected());
    assert!(!incomplete_selected.has_coherent_frontier());
    assert_eq!(incomplete_selected.admission(), ProfileAdmissionV1::Invalid);

    let unresolved: SqliteProfileV1 =
        serde_json::from_value(sqlite_profile("required_before_runtime_admission", false)).unwrap();
    assert!(!unresolved.is_selected());
    assert!(unresolved.has_coherent_frontier());
    assert_eq!(unresolved.admission(), ProfileAdmissionV1::Unresolved);

    let complete_candidate: SqliteProfileV1 =
        serde_json::from_value(sqlite_profile("selected", true)).unwrap();
    assert!(!complete_candidate.is_selected());
    assert!(!complete_candidate.has_coherent_frontier());
    assert_eq!(
        complete_candidate.admission(),
        ProfileAdmissionV1::CandidateRequiresOwnerAdmission
    );

    let mut partial_required = sqlite_profile("required_before_runtime_admission", false);
    partial_required["synchronous"] = serde_json::json!("full");
    let partial_required: SqliteProfileV1 = serde_json::from_value(partial_required).unwrap();
    assert_eq!(partial_required.admission(), ProfileAdmissionV1::Invalid);
}

#[test]
fn sqlite_candidate_rejects_arbitrary_tokens_hashes_and_integer_bounds() {
    let mutations = [
        ("synchronous", serde_json::json!("sometimes")),
        ("checkpoint_mode", serde_json::json!("eventually")),
        ("compile_options_sha256", serde_json::json!("a".repeat(63))),
        ("compile_options_sha256", serde_json::json!("A".repeat(64))),
        ("compile_options_sha256", serde_json::json!("g".repeat(64))),
        ("wal_autocheckpoint_frames", serde_json::json!(0)),
        (
            "wal_autocheckpoint_frames",
            serde_json::json!(JSON_SAFE_INTEGER_MAX + 1),
        ),
    ];

    for (field, invalid_value) in mutations {
        let mut candidate = sqlite_profile("selected", true);
        candidate[field] = invalid_value;
        let candidate: SqliteProfileV1 = serde_json::from_value(candidate).unwrap();
        assert_eq!(
            candidate.admission(),
            ProfileAdmissionV1::Invalid,
            "{field}"
        );
        assert!(!candidate.is_selected(), "{field}");
    }

    for (field, invalid_value) in [
        ("journal_mode", serde_json::json!("delete")),
        ("foreign_keys", serde_json::json!(false)),
        ("trusted_schema", serde_json::json!(true)),
    ] {
        let mut candidate = sqlite_profile("selected", true);
        candidate[field] = invalid_value;
        let candidate: SqliteProfileV1 = serde_json::from_value(candidate).unwrap();
        assert_eq!(
            candidate.admission(),
            ProfileAdmissionV1::Invalid,
            "{field}"
        );
    }
}

#[test]
fn authority_candidate_never_self_selects_and_rejects_unsafe_bounds() {
    let complete_candidate: AuthorityJournalProfileV1 =
        serde_json::from_value(authority_profile("selected", true)).unwrap();
    assert_eq!(
        complete_candidate.admission(),
        ProfileAdmissionV1::CandidateRequiresOwnerAdmission
    );
    assert!(!complete_candidate.is_selected());
    assert!(!complete_candidate.has_coherent_frontier());

    let mut algorithm_width_claim = authority_profile("selected", true);
    algorithm_width_claim["corruption_checksum"] = serde_json::json!("sha256");
    algorithm_width_claim["corruption_checksum_bytes"] = serde_json::json!(31);
    let algorithm_width_claim: AuthorityJournalProfileV1 =
        serde_json::from_value(algorithm_width_claim).unwrap();
    assert!(!algorithm_width_claim.is_selected());
    assert!(!algorithm_width_claim.has_coherent_frontier());

    for (field, invalid_value) in [
        ("corruption_checksum_bytes", serde_json::json!(0)),
        ("maximum_entry_bytes", serde_json::json!(0)),
        ("maximum_checkpoint_bytes", serde_json::json!(0)),
        ("maximum_checkpoint_candidates", serde_json::json!(0)),
        ("maximum_tail_entries", serde_json::json!(0)),
        (
            "maximum_entry_bytes",
            serde_json::json!(JSON_SAFE_INTEGER_MAX + 1),
        ),
    ] {
        let mut candidate = authority_profile("selected", true);
        candidate[field] = invalid_value;
        let candidate: AuthorityJournalProfileV1 = serde_json::from_value(candidate).unwrap();
        assert_eq!(
            candidate.admission(),
            ProfileAdmissionV1::Invalid,
            "{field}"
        );
        assert!(!candidate.is_selected(), "{field}");
    }
}

#[test]
fn authority_checksum_width_must_fit_both_payload_ceilings() {
    for field in ["maximum_entry_bytes", "maximum_checkpoint_bytes"] {
        let mut candidate = authority_profile("selected", true);
        candidate[field] = serde_json::json!(16);
        let candidate: AuthorityJournalProfileV1 = serde_json::from_value(candidate).unwrap();
        assert_eq!(
            candidate.admission(),
            ProfileAdmissionV1::Invalid,
            "{field}"
        );
        assert!(!candidate.is_selected(), "{field}");
    }

    let unresolved: AuthorityJournalProfileV1 = serde_json::from_value(authority_profile(
        "required_before_runtime_admission",
        false,
    ))
    .unwrap();
    assert_eq!(unresolved.admission(), ProfileAdmissionV1::Unresolved);
    assert!(unresolved.has_coherent_frontier());

    let mut partial_required = authority_profile("required_before_runtime_admission", false);
    partial_required["frame_encoding"] = serde_json::json!("candidate");
    let partial_required: AuthorityJournalProfileV1 =
        serde_json::from_value(partial_required).unwrap();
    assert_eq!(partial_required.admission(), ProfileAdmissionV1::Invalid);
}
