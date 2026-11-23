//! Behavioral and evidence laws for assembled resident characterization.

use std::collections::BTreeMap;

use super::*;

impl Configuration {
    fn smoke_for_test() -> Self {
        Self::preflight(
            true,
            1,
            1,
            &SMOKE_SCALES,
            Bounds {
                maximum_entry_bytes: 1 << 20,
                maximum_checkpoint_bytes: 1 << 24,
                maximum_checkpoint_candidates: 8,
                maximum_tail_entries: 8,
                maximum_total_transitions: 64,
            },
            EvidenceCoordinates {
                source: "test-source".into(),
                run: "test-run".into(),
                hardware: "test-hardware".into(),
            },
        )
    }
}

#[test]
fn schedule_projects_every_transition_before_workload_allocation() {
    assert_eq!(projected_transitions(1, &SMOKE_SCALES, 5), 85);
    assert_eq!(projected_transitions(2, &FULL_SCALES, 201), 186_327);
    assert_eq!(Operation::ALL.len(), OPERATIONS_PER_SCALE);
    assert_eq!(FULL_SCALES.len() * OPERATIONS_PER_SCALE, 12);

    let accepted = Configuration::smoke_for_test();
    assert_eq!(accepted.projected_total_transitions, 17);
    let bounds = Bounds {
        maximum_entry_bytes: 1 << 20,
        maximum_checkpoint_bytes: 1 << 24,
        maximum_checkpoint_candidates: 8,
        maximum_tail_entries: 8,
        maximum_total_transitions: 64,
    };
    assert!(std::panic::catch_unwind(|| {
        Configuration::preflight(
            true,
            1,
            0,
            &SMOKE_SCALES,
            bounds,
            EvidenceCoordinates {
                source: "test-source".into(),
                run: "test-run".into(),
                hardware: "test-hardware".into(),
            },
        )
    })
    .is_err());
    assert!(std::panic::catch_unwind(|| {
        Configuration::preflight(
            true,
            1,
            9,
            &SMOKE_SCALES,
            bounds,
            EvidenceCoordinates {
                source: "test-source".into(),
                run: "test-run".into(),
                hardware: "test-hardware".into(),
            },
        )
    })
    .is_err());
    let transition_limited = Bounds {
        maximum_total_transitions: 16,
        ..bounds
    };
    assert!(std::panic::catch_unwind(|| {
        Configuration::preflight(
            true,
            1,
            1,
            &SMOKE_SCALES,
            transition_limited,
            EvidenceCoordinates {
                source: "test-source".into(),
                run: "test-run".into(),
                hardware: "test-hardware".into(),
            },
        )
    })
    .is_err());
}

#[test]
fn rows_keep_assembled_mechanics_separate_from_admission_and_release_claims() {
    let configuration = Configuration::smoke_for_test();
    let measurements = Measurements {
        elapsed: vec![std::time::Duration::from_nanos(7)],
        main_bytes: vec![8],
        wal_bytes: vec![9],
        shm_bytes: vec![10],
        journal_heads: vec![4],
        retained_sessions: vec![1],
    };
    let row = render_row(
        1,
        Operation::Pin,
        SMOKE_SCALES[0],
        1,
        &configuration,
        &measurements,
    );
    let fields = report_header()
        .split('\t')
        .zip(row.split('\t'))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(fields["adapter_path"], "true");
    assert_eq!(fields["semantic_generation_admission"], "true");
    assert_eq!(fields["owner_qualified_actual"], "false");
    assert_eq!(fields["profile_selected"], "false");
    assert_eq!(fields["storage_admission_complete"], "false");
    assert_eq!(fields["acceptance_budget_selected"], "false");
    assert_eq!(fields["release_evidence"], "false");
    assert_eq!(fields["elapsed_p99_ns"], "");
    assert_eq!(
        fields["allocation_scope"],
        "not_measured_by_this_initial_lane"
    );
    assert_eq!(fields["rss_scope"], "not_measured_by_this_initial_lane");
}

#[test]
fn real_sqlite_samples_pin_drain_and_reconstruct_both_checkpoint_frontiers() {
    let configuration = Configuration::smoke_for_test();
    let (workspace, probe) = live_workspace();
    let expected = [
        (1, Operation::Pin, 4, 1),
        (1, Operation::Drain, 5, 0),
        (1, Operation::ReconstructCheckpointTail, 4, 1),
        (1, Operation::ReconstructCheckpointHead, 4, 1),
        (3, Operation::Pin, 6, 3),
        (3, Operation::Drain, 7, 2),
        (3, Operation::ReconstructCheckpointTail, 6, 3),
        (3, Operation::ReconstructCheckpointHead, 6, 3),
    ];
    for (sessions, operation, journal_head, retained_sessions) in expected {
        let sample = run_sample(operation, sessions, &configuration, &workspace, &probe);
        assert_eq!(sample.journal_head, journal_head);
        assert_eq!(sample.retained_sessions, retained_sessions);
        assert!(sample.sizes.main > 0);
    }
}
