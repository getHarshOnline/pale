//! Pure benchmark parser, percentile, resource-preflight, and report-contract tests.

use std::collections::BTreeSet;

use pale_core::sha256_hex;

use super::evidence::{disposition, render_coverage, ProfileEvidenceDisposition};
use super::input::{
    candidate_set_sha256, parse_pragma_candidates, preflight_resources, project_resources,
    BenchmarkConfiguration, EvidenceCoordinates, OperatorResourceBudget, ResourceProjection,
    SqlitePragmaCandidate, WorkloadClass,
};
use super::report::{report_row_claims, BenchmarkReporter, REPORT_COMPLETION_MARKER};
use super::statistics::{
    percentile, report_column_count, report_header, selection_grade_percentile,
    MINIMUM_P99_INDEPENDENT_RUNS,
};
use super::workload::CaseMeasurements;
use crate::frame::{
    AUTHORITY_CORRUPTION_CHECKSUM_CANDIDATE_ID, AUTHORITY_CORRUPTION_CHECKSUM_ENCODED_BYTES,
    AUTHORITY_FRAME_ENCODING_CANDIDATE_ID,
};
use crate::ControlStorageProfileRequirement;

#[test]
fn exact_pragma_and_workload_parsers_reject_ambiguous_inputs() {
    let profile = SqlitePragmaCandidate::parse("FULL/on/off/TRUNCATE/1000/16/8").unwrap();
    assert_eq!(profile.encode(), "FULL/on/off/TRUNCATE/1000/16/8");
    assert!(SqlitePragmaCandidate::parse("FULL/on/off/TRUNCATE/1000/16").is_err());
    assert!(SqlitePragmaCandidate::parse("FULL/yes/off/TRUNCATE/1000/16/8").is_err());
    assert_eq!(
        WorkloadClass::parse("exact_adapter_mechanics_v1").unwrap(),
        WorkloadClass::ExactAdapterMechanics
    );
    assert_eq!(
        WorkloadClass::parse("sqlite_characterization").unwrap(),
        WorkloadClass::SqliteCharacterization
    );
    assert!(WorkloadClass::parse("production").is_err());
    assert!(parse_pragma_candidates(
        "FULL/on/off/TRUNCATE/1000/16/8;FULL/on/off/TRUNCATE/1000/16/8"
    )
    .is_err());
}

#[test]
fn pragma_candidate_order_does_not_change_the_canonical_set_or_digest() {
    let left =
        parse_pragma_candidates("FULL/on/off/TRUNCATE/1000/16/8;NORMAL/off/off/PASSIVE/0/1/64")
            .unwrap();
    let right =
        parse_pragma_candidates("NORMAL/off/off/PASSIVE/0/1/64;FULL/on/off/TRUNCATE/1000/16/8")
            .unwrap();
    assert_eq!(left, right);
    assert_eq!(
        candidate_set_sha256(WorkloadClass::SqliteCharacterization, &left),
        candidate_set_sha256(WorkloadClass::SqliteCharacterization, &right)
    );
}

#[test]
fn p99_is_not_selection_grade_below_101_independent_samples() {
    let sufficient = (1..=(MINIMUM_P99_INDEPENDENT_RUNS as u128)).collect::<Vec<_>>();
    assert_eq!(
        selection_grade_percentile(&sufficient, 99, MINIMUM_P99_INDEPENDENT_RUNS - 1),
        None
    );
    assert_eq!(
        selection_grade_percentile(&sufficient, 99, MINIMUM_P99_INDEPENDENT_RUNS),
        Some(percentile(&sufficient, 99))
    );
    assert_eq!(percentile(&[1, 2, 3, 4], 50), 2);
    assert_eq!(percentile(&[1, 2, 3, 4], 100), 4);
}

#[test]
fn complete_run_projection_must_fit_every_explicit_operator_ceiling() {
    let profile = SqlitePragmaCandidate::parse("FULL/off/off/PASSIVE/0/1/64").unwrap();
    let projection = project_resources(
        100_000,
        1_048_576,
        1_048_576,
        WorkloadClass::SqliteCharacterization,
        &[profile],
        201,
    )
    .expect("large but representable projection");
    assert!(projection.payload_bytes > 1_000_000_000_000);
    assert!(preflight_resources(
        projection,
        OperatorResourceBudget {
            maximum_independent_runs: projection.independent_runs,
            maximum_operations: projection.operations,
            maximum_payload_bytes: 999_999_999_999,
        }
    )
    .is_err());
    assert!(preflight_resources(
        projection,
        OperatorResourceBudget {
            maximum_independent_runs: projection.independent_runs,
            maximum_operations: projection.operations,
            maximum_payload_bytes: projection.payload_bytes,
        }
    )
    .is_ok());
    assert!(preflight_resources(
        ResourceProjection {
            independent_runs: 2,
            operations: 10,
            payload_bytes: 100,
        },
        OperatorResourceBudget {
            maximum_independent_runs: 1,
            maximum_operations: 10,
            maximum_payload_bytes: 100,
        }
    )
    .is_err());
}

#[test]
fn report_header_is_rectangular_unique_and_names_separate_checkpoint_events() {
    let header = report_header();
    let columns = header.split('\t').collect::<Vec<_>>();
    assert_eq!(columns.len(), report_column_count());
    assert_eq!(
        columns.iter().copied().collect::<BTreeSet<_>>().len(),
        columns.len()
    );
    assert!(columns.contains(&"independent_runs"));
    assert!(columns.contains(&"adapter_path"));
    assert!(columns.contains(&"append_pooled_operations"));
    assert!(columns.contains(&"checkpoint_projection_commit_p99_ns"));
    assert!(columns.contains(&"wal_checkpoint_p99_ns"));
    assert!(columns.contains(&"checkpoint_status_observations"));
    assert!(columns.contains(&"checkpoint_busy_worst"));
    assert!(columns.contains(&"source_id"));
    assert!(columns.contains(&"binary_sha256"));
    assert!(columns.contains(&"storage_manifest_sha256"));
    assert!(columns.contains(&"filesystem"));
    assert!(columns.contains(&"storage"));
    assert!(columns.contains(&"power_envelope"));
    assert!(columns.contains(&"run_id"));
    assert!(columns.contains(&"explicit_payload_copy_lower_bound_bytes_per_run"));
    assert!(columns.contains(&"allocation_total_bytes_complete_case"));
    assert!(columns.contains(&"rss_process_lifetime_high_water_bytes"));
    assert!(!EvidenceCoordinates::default().is_complete());
}

#[test]
fn report_rows_distinguish_exact_adapter_mechanics_without_admission_claims() {
    let adapter = report_row_claims(WorkloadClass::ExactAdapterMechanics);
    assert!(adapter.adapter_path);
    assert!(!adapter.owner_qualified_actual);
    assert!(!adapter.profile_selected);
    assert!(!adapter.storage_admission_complete);
    assert!(!adapter.acceptance_budget_selected);
    assert!(!adapter.release_evidence);

    let characterization = report_row_claims(WorkloadClass::SqliteCharacterization);
    assert!(!characterization.adapter_path);
    assert!(!characterization.owner_qualified_actual);
    assert!(!characterization.profile_selected);
    assert!(!characterization.storage_admission_complete);
    assert!(!characterization.acceptance_budget_selected);
    assert!(!characterization.release_evidence);
}

#[test]
fn every_profile_requirement_has_one_truthful_lane_disposition() {
    use ControlStorageProfileRequirement as Requirement;
    use ProfileEvidenceDisposition as Evidence;

    let expected = [
        (
            WorkloadClass::ExactAdapterMechanics,
            [
                Evidence::ObservedCandidate,
                Evidence::NotExercised,
                Evidence::NotExercised,
                Evidence::NotExercised,
                Evidence::NotExercised,
                Evidence::NotExercised,
                Evidence::NotExercised,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::NotExercised,
                Evidence::OwnerPolicyAdmissionRequired,
                Evidence::OwnerPolicyAdmissionRequired,
                Evidence::EngineConformanceAndOwnerAdmissionRequired,
            ],
        ),
        (
            WorkloadClass::SqliteCharacterization,
            [
                Evidence::ObservedCandidate,
                Evidence::MeasuredCandidate,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::MeasuredCandidate,
                Evidence::MeasuredCandidate,
                Evidence::PartiallyMeasuredCandidate,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::ObservedCandidate,
                Evidence::MeasuredCandidate,
                Evidence::OwnerPolicyAdmissionRequired,
                Evidence::OwnerPolicyAdmissionRequired,
                Evidence::EngineConformanceAndOwnerAdmissionRequired,
            ],
        ),
    ];

    assert_eq!(
        ControlStorageProfileRequirement::ALL,
        [
            Requirement::SqliteCompileOptions,
            Requirement::SqliteSynchronous,
            Requirement::SqliteFullFsync,
            Requirement::SqliteCheckpointFullFsync,
            Requirement::SqliteCheckpointMode,
            Requirement::SqliteWalAutocheckpointFrames,
            Requirement::SqliteDurabilityGrouping,
            Requirement::AuthorityFrameEncoding,
            Requirement::AuthorityCorruptionChecksum,
            Requirement::AuthorityCorruptionChecksumBytes,
            Requirement::AuthorityMaximumEntryBytes,
            Requirement::AuthorityMaximumCheckpointBytes,
            Requirement::AuthorityMaximumCheckpointCandidates,
            Requirement::AuthorityMaximumTailEntries,
            Requirement::AuthorityCheckpointCadence,
            Requirement::AuthorityRetentionPolicy,
            Requirement::AuthorityCompactionPolicy,
            Requirement::BackupRestorePolicy,
        ]
    );

    for (workload_class, expected_dispositions) in expected {
        assert_eq!(
            ControlStorageProfileRequirement::ALL
                .map(|requirement| { disposition(requirement, workload_class) }),
            expected_dispositions
        );
        let coverage = render_coverage(workload_class);
        let entries = coverage.split(',').collect::<Vec<_>>();
        assert_eq!(entries.len(), ControlStorageProfileRequirement::ALL.len());
        assert_eq!(
            entries
                .iter()
                .filter_map(|entry| entry.split_once('=').map(|(requirement, _)| requirement))
                .collect::<Vec<_>>(),
            ControlStorageProfileRequirement::ALL
                .into_iter()
                .map(ControlStorageProfileRequirement::as_str)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn complete_report_rows_align_and_terminal_seal_reproduces() {
    let configuration = BenchmarkConfiguration::smoke_validation();
    let reporter = BenchmarkReporter::new(&configuration);
    let rows = configuration
        .cases()
        .iter()
        .enumerate()
        .map(|(index, case)| reporter.render_row(index + 1, case, &CaseMeasurements::default()))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), configuration.expected_rows);
    assert!(rows
        .iter()
        .all(|row| row.split('\t').count() == report_column_count()));

    let rendered_header = report_header();
    let header = rendered_header.split('\t').collect::<Vec<_>>();
    let first = rows[0].split('\t').collect::<Vec<_>>();
    let field = |name: &str| {
        let index = header
            .iter()
            .position(|candidate| *candidate == name)
            .expect("report field exists");
        first[index]
    };
    assert_eq!(
        field("authority_frame_encoding_candidate"),
        AUTHORITY_FRAME_ENCODING_CANDIDATE_ID
    );
    assert_eq!(
        field("physical_checksum_candidate"),
        AUTHORITY_CORRUPTION_CHECKSUM_CANDIDATE_ID
    );
    assert_eq!(
        field("physical_checksum_candidate_encoded_bytes"),
        AUTHORITY_CORRUPTION_CHECKSUM_ENCODED_BYTES.to_string()
    );
    assert_eq!(field("authority_frame_encoding_selected"), "false");
    assert_eq!(field("physical_checksum_selected"), "false");
    assert_eq!(
        field("allocator"),
        if cfg!(feature = "bench-memory") {
            "system_with_allocation-counter-0.8.1"
        } else {
            "system_uninstrumented"
        }
    );
    assert_eq!(
        field("timing_allocation_counter"),
        if cfg!(feature = "bench-memory") {
            "enabled_complete_case"
        } else {
            "disabled"
        }
    );
    assert_eq!(
        field("rss_scope"),
        "unavailable_feature_platform_or_observation"
    );
    assert_eq!(field("rss_source_unit"), "");
    assert_eq!(field("rss_process_lifetime_high_water_bytes"), "");

    let report = reporter.render_complete_report(&rows);
    let lines = report.lines().collect::<Vec<_>>();
    let terminal = lines.last().expect("report has terminal marker");
    let [marker, row_count, candidate_set_sha256, payload_sha256]: [&str; 4] = terminal
        .split('\t')
        .collect::<Vec<_>>()
        .try_into()
        .expect("terminal marker has four fields");
    assert_eq!(marker, REPORT_COMPLETION_MARKER);
    assert_eq!(row_count, configuration.expected_rows.to_string());
    assert_eq!(candidate_set_sha256, configuration.candidate_set_sha256);
    let mut payload = lines[..lines.len() - 1].join("\n");
    payload.push('\n');
    assert_eq!(payload_sha256, sha256_hex(payload.as_bytes()));

    assert!(std::panic::catch_unwind(|| reporter.render_complete_report(&rows[..2])).is_err());
}
