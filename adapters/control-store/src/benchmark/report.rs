//! Stable statistics, environment observation, and truthful TSV reporting.

use std::fs;
use std::process::Command;

use pale_core::sha256_hex;
use rusqlite::Connection;

use super::evidence::render_coverage;
use super::input::{
    switch, BenchmarkCase, BenchmarkConfiguration, EvidenceClass, EvidenceCoordinates,
    OperatorResourceBudget, ResourceProjection,
};
use super::memory::{
    allocation_scope, allocator_coordinate, rss_source_unit, timing_allocation_counter,
};
use super::sqlite_characterization::pragma_i64;
use super::statistics::{report_column_count, report_header, Distribution};
use super::workload::CaseMeasurements;
use crate::frame::{
    AUTHORITY_CORRUPTION_CHECKSUM_CANDIDATE_ID, AUTHORITY_CORRUPTION_CHECKSUM_ENCODED_BYTES,
    AUTHORITY_FRAME_ENCODING_CANDIDATE_ID,
};
use crate::store::compile_options_sha256;

/// Terminal line prefix proving that every bounded report row was rendered.
pub(crate) const REPORT_COMPLETION_MARKER: &str = "#pale_control_sqlite_report_complete_v1";

/// Environment, evidence, and dependency coordinates shared by report rows.
pub(crate) struct BenchmarkReporter {
    evidence_class: EvidenceClass,
    workload_label: String,
    hardware: String,
    coordinates: EvidenceCoordinates,
    operator_budget: OperatorResourceBudget,
    operator_budget_supplied: bool,
    projection: ResourceProjection,
    candidate_set_sha256: String,
    expected_rows: usize,
    os: &'static str,
    architecture: &'static str,
    kernel: String,
    toolchain: String,
    target: String,
    build_profile: &'static str,
    sqlite_version: &'static str,
    compile_options_sha256: String,
    page_size: i64,
}

impl BenchmarkReporter {
    /// Observe toolchain/SQLite coordinates and verify the supplied binary digest.
    pub(crate) fn new(configuration: &BenchmarkConfiguration) -> Self {
        if configuration.coordinates.is_complete() {
            let executable = std::env::current_exe().expect("resolve benchmark executable");
            let bytes = fs::read(&executable).unwrap_or_else(|error| {
                panic!("read benchmark binary {}: {error}", executable.display())
            });
            assert_eq!(
                sha256_hex(&bytes),
                configuration.coordinates.binary_sha256,
                "supplied benchmark binary SHA-256 does not match current executable"
            );
        }

        let rustc = command_output("rustc", &["-vV"]);
        let toolchain = rustc.lines().next().unwrap_or("unknown-rustc").to_owned();
        let target = rustc
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .unwrap_or("unknown-target")
            .to_owned();
        let connection = Connection::open_in_memory().expect("open SQLite metadata database");
        let compile_options_sha256 = compile_options_sha256(&connection)
            .expect("observe SQLite compile-option fingerprint for benchmark evidence");
        let page_size = pragma_i64(&connection, "page_size");
        Self {
            evidence_class: configuration.evidence_class,
            workload_label: configuration.workload_label.clone(),
            hardware: configuration.hardware.clone(),
            coordinates: configuration.coordinates.clone(),
            operator_budget: configuration.operator_budget,
            operator_budget_supplied: configuration.operator_budget_supplied,
            projection: configuration.projection,
            candidate_set_sha256: configuration.candidate_set_sha256.clone(),
            expected_rows: configuration.expected_rows,
            os: std::env::consts::OS,
            architecture: std::env::consts::ARCH,
            kernel: tsv(&command_output("uname", &["-srv"])),
            toolchain: tsv(&toolchain),
            target: tsv(&target),
            build_profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            },
            sqlite_version: rusqlite::version(),
            compile_options_sha256,
            page_size,
        }
    }

    /// Render one summarized profile/scale row without upgrading its proof class.
    pub(crate) fn render_row(
        &self,
        case_ordinal: usize,
        case: &BenchmarkCase,
        measurements: &CaseMeasurements,
    ) -> String {
        let claims = report_row_claims(case.workload_class);
        let independent_runs = measurements.independent_runs;
        let checksum = Distribution::durations(&measurements.checksum_reference, independent_runs);
        let append = Distribution::durations(&measurements.append, independent_runs);
        let commit = Distribution::durations(&measurements.commit, independent_runs);
        let release = Distribution::durations(&measurements.group_release, independent_runs);
        let projection_commit =
            Distribution::durations(&measurements.checkpoint_projection_commit, independent_runs);
        let wal_checkpoint =
            Distribution::durations(&measurements.wal_checkpoint, independent_runs);
        let recovery = Distribution::durations(&measurements.recovery, independent_runs);
        let before_main = Distribution::numbers(
            measurements
                .before_checkpoint
                .iter()
                .map(|sizes| sizes.main),
            independent_runs,
        );
        let before_wal = Distribution::numbers(
            measurements.before_checkpoint.iter().map(|sizes| sizes.wal),
            independent_runs,
        );
        let before_shm = Distribution::numbers(
            measurements.before_checkpoint.iter().map(|sizes| sizes.shm),
            independent_runs,
        );
        let after_main = Distribution::numbers(
            measurements.after_checkpoint.iter().map(|sizes| sizes.main),
            independent_runs,
        );
        let after_wal = Distribution::numbers(
            measurements.after_checkpoint.iter().map(|sizes| sizes.wal),
            independent_runs,
        );
        let after_shm = Distribution::numbers(
            measurements.after_checkpoint.iter().map(|sizes| sizes.shm),
            independent_runs,
        );
        let recovery_tail =
            Distribution::numbers(measurements.recovery_tail.iter().copied(), independent_runs);
        let busy = Distribution::numbers(
            measurements
                .checkpoint_statuses
                .iter()
                .map(|status| nonnegative(status.busy, "checkpoint busy")),
            independent_runs,
        );
        let log_frames = Distribution::numbers(
            measurements
                .checkpoint_statuses
                .iter()
                .map(|status| nonnegative(status.log_frames, "checkpoint log frames")),
            independent_runs,
        );
        let checkpointed_frames = Distribution::numbers(
            measurements
                .checkpoint_statuses
                .iter()
                .map(|status| nonnegative(status.checkpointed_frames, "checkpointed frames")),
            independent_runs,
        );
        let logical_payload_bytes = case
            .records
            .checked_mul(u64::try_from(case.payload_bytes).expect("payload bytes fit u64"))
            .expect("logical payload byte count overflow");
        let mut fields = vec![
            self.evidence_class.as_str().to_owned(),
            case.workload_class.as_str().to_owned(),
            self.workload_label.clone(),
            claims.adapter_path.to_string(),
            claims.owner_qualified_actual.to_string(),
            claims.profile_selected.to_string(),
            claims.storage_admission_complete.to_string(),
            claims.acceptance_budget_selected.to_string(),
            self.operator_budget_supplied.to_string(),
            self.coordinates.is_complete().to_string(),
            claims.release_evidence.to_string(),
            case_ordinal.to_string(),
            self.expected_rows.to_string(),
            self.candidate_set_sha256.clone(),
            render_coverage(case.workload_class),
            case.scale.as_str().to_owned(),
            case.records.to_string(),
            case.payload_bytes.to_string(),
            case.checkpoint_bytes.to_string(),
            case.limits.maximum_entry_bytes().to_string(),
            case.limits.maximum_checkpoint_bytes().to_string(),
            case.limits.maximum_checkpoint_candidates().to_string(),
            case.limits.maximum_tail_entries().to_string(),
            independent_runs.to_string(),
        ];
        if let Some(candidate) = &case.pragma_candidate {
            fields.extend([
                candidate.synchronous.as_str().to_owned(),
                switch(candidate.fullfsync).to_owned(),
                switch(candidate.checkpoint_fullfsync).to_owned(),
                candidate.checkpoint_mode.as_str().to_owned(),
                candidate.wal_autocheckpoint_frames.to_string(),
                candidate.group_records.to_string(),
                candidate.checkpoint_every_groups.to_string(),
            ]);
        } else {
            fields.extend(std::iter::repeat_n(String::new(), 7));
        }
        fields.extend([
            self.operator_budget.maximum_independent_runs.to_string(),
            self.operator_budget.maximum_operations.to_string(),
            self.operator_budget.maximum_payload_bytes.to_string(),
            self.projection.independent_runs.to_string(),
            self.projection.operations.to_string(),
            self.projection.payload_bytes.to_string(),
            "sha256_hash_and_hex_reference_only".to_owned(),
            AUTHORITY_FRAME_ENCODING_CANDIDATE_ID.to_owned(),
            "false".to_owned(),
            AUTHORITY_CORRUPTION_CHECKSUM_CANDIDATE_ID.to_owned(),
            AUTHORITY_CORRUPTION_CHECKSUM_ENCODED_BYTES.to_string(),
            "false".to_owned(),
        ]);
        push_distribution(&mut fields, checksum.as_ref());
        push_distribution(&mut fields, append.as_ref());
        push_distribution(&mut fields, commit.as_ref());
        push_distribution(&mut fields, release.as_ref());
        push_distribution(&mut fields, projection_commit.as_ref());
        push_distribution(&mut fields, wal_checkpoint.as_ref());
        push_distribution(&mut fields, recovery.as_ref());
        fields.extend([
            logical_payload_bytes.to_string(),
            measurements.memory.explicit_copy_bytes_per_run.to_string(),
            "pale_explicit_payload_slice_copies_static_lower_bound".to_owned(),
            if measurements.memory.allocations.is_some() {
                "allocation-counter-0.8.1"
            } else {
                "unavailable"
            }
            .to_owned(),
            allocation_scope().to_owned(),
            measurements
                .memory
                .allocations
                .map_or_else(String::new, |value| value.total_count.to_string()),
            measurements
                .memory
                .allocations
                .map_or_else(String::new, |value| value.total_bytes.to_string()),
            measurements
                .memory
                .allocations
                .map_or_else(String::new, |value| value.peak_relative_count.to_string()),
            measurements
                .memory
                .allocations
                .map_or_else(String::new, |value| value.peak_relative_bytes.to_string()),
            measurements
                .memory
                .allocations
                .map_or_else(String::new, |value| value.retained_count.to_string()),
            measurements
                .memory
                .allocations
                .map_or_else(String::new, |value| value.retained_bytes.to_string()),
            if measurements.memory.rss.is_some() {
                "nix-getrusage-0.31.3"
            } else {
                "unavailable"
            }
            .to_owned(),
            if measurements.memory.rss.is_some() {
                "process_lifetime_self_and_terminated_children_after_complete_case"
            } else {
                "unavailable_feature_platform_or_observation"
            }
            .to_owned(),
            measurements
                .memory
                .rss
                .map_or("", |_| rss_source_unit())
                .to_owned(),
            measurements
                .memory
                .rss
                .map_or_else(String::new, |value| value.process_bytes.to_string()),
            measurements.memory.rss.map_or_else(String::new, |value| {
                value.terminated_children_bytes.to_string()
            }),
            percentile_value(before_main.as_ref(), |value| value.p50),
            percentile_value(before_wal.as_ref(), |value| value.p50),
            percentile_value(before_shm.as_ref(), |value| value.p50),
            percentile_value(after_main.as_ref(), |value| value.p50),
            percentile_value(after_wal.as_ref(), |value| value.p50),
            percentile_value(after_shm.as_ref(), |value| value.p50),
            percentile_value(recovery_tail.as_ref(), |value| value.p50),
            measurements.checkpoint_statuses.len().to_string(),
        ]);
        push_status_distribution(&mut fields, busy.as_ref());
        push_status_distribution(&mut fields, log_frames.as_ref());
        push_status_distribution(&mut fields, checkpointed_frames.as_ref());
        fields.extend([
            self.coordinates.source_id.clone(),
            self.coordinates.binary_sha256.clone(),
            self.coordinates.storage_manifest_sha256.clone(),
            self.coordinates.filesystem.clone(),
            self.coordinates.storage.clone(),
            self.coordinates.power_envelope.clone(),
            self.coordinates.run_id.clone(),
            self.hardware.clone(),
            self.os.to_owned(),
            self.architecture.to_owned(),
            self.kernel.clone(),
            self.toolchain.clone(),
            self.target.clone(),
            self.build_profile.to_owned(),
            allocator_coordinate().to_owned(),
            timing_allocation_counter().to_owned(),
            "external_build_receipt_required".to_owned(),
            self.sqlite_version.to_owned(),
            self.compile_options_sha256.clone(),
            self.page_size.to_string(),
            "false".to_owned(),
            "process_loss_bounded_checkpoint_tail_validation_only".to_owned(),
            "single_writer".to_owned(),
        ]);
        assert_eq!(
            fields.len(),
            report_column_count(),
            "benchmark report row must match its header"
        );
        fields.join("\t")
    }

    /// Seal a complete bounded report after every case has succeeded.
    ///
    /// Rows are held in memory until this method verifies the exact count. A
    /// missing terminal marker or mismatched payload digest therefore exposes
    /// a killed or otherwise partial output as incomplete evidence.
    pub(crate) fn render_complete_report(&self, rows: &[String]) -> String {
        assert_eq!(
            rows.len(),
            self.expected_rows,
            "benchmark report cannot seal a partial case set"
        );
        let mut payload = report_header();
        payload.push('\n');
        for row in rows {
            payload.push_str(row);
            payload.push('\n');
        }
        let payload_sha256 = sha256_hex(payload.as_bytes());
        format!(
            "{payload}{REPORT_COMPLETION_MARKER}\t{}\t{}\t{payload_sha256}\n",
            self.expected_rows, self.candidate_set_sha256
        )
    }
}

/// Closed report-row claims derived solely from the workload's actual mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReportRowClaims {
    /// Whether the row exercised the exact crate-private adapter implementation.
    pub(crate) adapter_path: bool,
    /// Whether the supplied scale was qualified by its semantic owner.
    pub(crate) owner_qualified_actual: bool,
    /// Whether the unresolved storage profile was owner-selected.
    pub(crate) profile_selected: bool,
    /// Whether the physical store was admitted into Pale Runtime assembly.
    pub(crate) storage_admission_complete: bool,
    /// Whether an owner selected an acceptance budget from these observations.
    pub(crate) acceptance_budget_selected: bool,
    /// Whether the row is release evidence.
    pub(crate) release_evidence: bool,
}

/// Derive the exact adapter-path fact while retaining every authority nonclaim.
pub(crate) const fn report_row_claims(
    workload_class: super::input::WorkloadClass,
) -> ReportRowClaims {
    ReportRowClaims {
        adapter_path: matches!(
            workload_class,
            super::input::WorkloadClass::ExactAdapterMechanics
        ),
        owner_qualified_actual: false,
        profile_selected: false,
        storage_admission_complete: false,
        acceptance_budget_selected: false,
        release_evidence: false,
    }
}

/// Add count, p50, p95, gated p99, and maximum fields.
fn push_distribution(fields: &mut Vec<String>, distribution: Option<&Distribution>) {
    match distribution {
        Some(distribution) => fields.extend([
            distribution.count.to_string(),
            distribution.p50.to_string(),
            distribution.p95.to_string(),
            distribution
                .p99
                .map_or_else(String::new, |value| value.to_string()),
            distribution.max.to_string(),
        ]),
        None => fields.extend([
            "0".to_owned(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ]),
    }
}

/// Add p50, p95, gated p99, and worst-case checkpoint-status fields.
fn push_status_distribution(fields: &mut Vec<String>, distribution: Option<&Distribution>) {
    match distribution {
        Some(distribution) => fields.extend([
            distribution.p50.to_string(),
            distribution.p95.to_string(),
            distribution
                .p99
                .map_or_else(String::new, |value| value.to_string()),
            distribution.max.to_string(),
        ]),
        None => fields.extend(std::iter::repeat_n(String::new(), 4)),
    }
}

/// Render one distribution member or an empty field for an inapplicable lane.
fn percentile_value(
    distribution: Option<&Distribution>,
    select: impl FnOnce(&Distribution) -> u128,
) -> String {
    distribution.map_or_else(String::new, |value| select(value).to_string())
}

/// Convert a `SQLite` status counter to its documented nonnegative domain.
fn nonnegative(value: i64, label: &str) -> u128 {
    u128::try_from(value).unwrap_or_else(|_| panic!("{label} must be nonnegative"))
}

/// Run a local metadata command and normalize its UTF-8 output.
fn command_output(program: &str, arguments: &[&str]) -> String {
    let output = Command::new(program)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("run {program}: {error}"));
    assert!(output.status.success(), "{program} metadata command failed");
    String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("decode {program} output: {error}"))
        .trim()
        .to_owned()
}

/// Remove tabs and line breaks from observed command metadata.
fn tsv(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\t' | '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}
