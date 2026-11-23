//! Pure nearest-rank statistics and stable benchmark report-schema contract.

use std::time::Duration;

/// Minimum independent-run count required before a p99 is emitted.
pub(crate) const MINIMUM_P99_INDEPENDENT_RUNS: usize = 101;

/// Stable tab-separated report schema.
const REPORT_COLUMNS: &[&str] = &[
    "evidence_class",
    "workload_class",
    "workload_label",
    "adapter_path",
    "owner_qualified_actual",
    "profile_selected",
    "storage_admission_complete",
    "acceptance_budget_selected",
    "operator_resource_budget_supplied",
    "evidence_coordinates_complete",
    "release_evidence",
    "case_ordinal",
    "expected_rows",
    "candidate_set_sha256",
    "profile_requirement_evidence",
    "scale",
    "records",
    "payload_bytes",
    "checkpoint_bytes",
    "maximum_entry_bytes",
    "maximum_checkpoint_bytes",
    "maximum_checkpoint_candidates",
    "maximum_tail_entries",
    "independent_runs",
    "synchronous",
    "fullfsync",
    "checkpoint_fullfsync",
    "checkpoint_mode",
    "wal_autocheckpoint_frames",
    "group_records",
    "checkpoint_every_groups",
    "operator_budget_max_independent_runs",
    "operator_budget_max_operations",
    "operator_budget_max_payload_bytes",
    "projected_total_independent_runs",
    "projected_total_operations",
    "projected_total_payload_bytes",
    "checksum_reference",
    "authority_frame_encoding_candidate",
    "authority_frame_encoding_selected",
    "physical_checksum_candidate",
    "physical_checksum_candidate_encoded_bytes",
    "physical_checksum_selected",
    "checksum_pooled_operations",
    "checksum_p50_ns",
    "checksum_p95_ns",
    "checksum_p99_ns",
    "checksum_max_ns",
    "append_pooled_operations",
    "append_p50_ns",
    "append_p95_ns",
    "append_p99_ns",
    "append_max_ns",
    "commit_pooled_operations",
    "commit_p50_ns",
    "commit_p95_ns",
    "commit_p99_ns",
    "commit_max_ns",
    "group_release_pooled_operations",
    "group_release_p50_ns",
    "group_release_p95_ns",
    "group_release_p99_ns",
    "group_release_max_ns",
    "checkpoint_projection_commit_pooled_operations",
    "checkpoint_projection_commit_p50_ns",
    "checkpoint_projection_commit_p95_ns",
    "checkpoint_projection_commit_p99_ns",
    "checkpoint_projection_commit_max_ns",
    "wal_checkpoint_pooled_operations",
    "wal_checkpoint_p50_ns",
    "wal_checkpoint_p95_ns",
    "wal_checkpoint_p99_ns",
    "wal_checkpoint_max_ns",
    "recovery_observations",
    "recovery_p50_ns",
    "recovery_p95_ns",
    "recovery_p99_ns",
    "recovery_max_ns",
    "logical_payload_bytes",
    "explicit_payload_copy_lower_bound_bytes_per_run",
    "explicit_copy_scope",
    "allocation_instrument",
    "allocation_scope",
    "allocation_total_count_complete_case",
    "allocation_total_bytes_complete_case",
    "allocation_peak_relative_count_complete_case",
    "allocation_peak_relative_bytes_complete_case",
    "allocation_retained_count_complete_case",
    "allocation_retained_bytes_complete_case",
    "rss_instrument",
    "rss_scope",
    "rss_source_unit",
    "rss_process_lifetime_high_water_bytes",
    "rss_terminated_children_lifetime_high_water_bytes",
    "main_before_p50_bytes",
    "wal_before_p50_bytes",
    "shm_before_p50_bytes",
    "main_after_p50_bytes",
    "wal_after_p50_bytes",
    "shm_after_p50_bytes",
    "recovery_tail_p50_bytes",
    "checkpoint_status_observations",
    "checkpoint_busy_p50",
    "checkpoint_busy_p95",
    "checkpoint_busy_p99",
    "checkpoint_busy_worst",
    "checkpoint_log_frames_p50",
    "checkpoint_log_frames_p95",
    "checkpoint_log_frames_p99",
    "checkpoint_log_frames_worst",
    "checkpointed_frames_p50",
    "checkpointed_frames_p95",
    "checkpointed_frames_p99",
    "checkpointed_frames_worst",
    "source_id",
    "binary_sha256",
    "storage_manifest_sha256",
    "filesystem",
    "storage",
    "power_envelope",
    "run_id",
    "hardware",
    "os",
    "arch",
    "kernel",
    "toolchain",
    "target",
    "build_profile",
    "allocator",
    "timing_allocation_counter",
    "pgo_evidence",
    "sqlite_version",
    "compile_options_sha256",
    "page_size",
    "durability_claim",
    "crash_recovery_claim",
    "concurrency_class",
];

/// Sorted nearest-rank distribution used for durations, bytes, and counters.
pub(crate) struct Distribution {
    /// Pooled observation count.
    pub(crate) count: usize,
    /// Median observation.
    pub(crate) p50: u128,
    /// 95th-percentile observation.
    pub(crate) p95: u128,
    /// 99th-percentile observation, absent until the independent-run floor is met.
    pub(crate) p99: Option<u128>,
    /// Maximum observation.
    pub(crate) max: u128,
}

impl Distribution {
    /// Convert durations to nanosecond observations.
    pub(crate) fn durations(values: &[Duration], independent_runs: usize) -> Option<Self> {
        Self::numbers(values.iter().map(Duration::as_nanos), independent_runs)
    }

    /// Sort one non-empty numeric observation stream.
    pub(crate) fn numbers<T>(
        values: impl IntoIterator<Item = T>,
        independent_runs: usize,
    ) -> Option<Self>
    where
        T: Into<u128>,
    {
        let mut values = values.into_iter().map(Into::into).collect::<Vec<_>>();
        if values.is_empty() {
            return None;
        }
        values.sort_unstable();
        Some(Self {
            count: values.len(),
            p50: percentile(&values, 50),
            p95: percentile(&values, 95),
            p99: selection_grade_percentile(&values, 99, independent_runs),
            max: *values.last().expect("nonempty distribution has a maximum"),
        })
    }
}

/// Nearest-rank percentile over an already sorted non-empty stream.
pub(crate) fn percentile(values: &[u128], percentile: usize) -> u128 {
    assert!(!values.is_empty(), "percentile input is nonempty");
    assert!((1..=100).contains(&percentile), "percentile is in 1..=100");
    let rank = values.len().saturating_mul(percentile).div_ceil(100);
    values[rank.saturating_sub(1).min(values.len() - 1)]
}

/// Return a percentile only when its independent samples meet the p99 floor.
pub(crate) fn selection_grade_percentile(
    sorted_values: &[u128],
    percentile_value: usize,
    independent_samples: usize,
) -> Option<u128> {
    if percentile_value == 99 && independent_samples < MINIMUM_P99_INDEPENDENT_RUNS {
        None
    } else {
        Some(percentile(sorted_values, percentile_value))
    }
}

/// Render the exact stable header used by the benchmark binary.
pub(crate) fn report_header() -> String {
    REPORT_COLUMNS.join("\t")
}

/// Return the stable report width for parity tests.
pub(crate) const fn report_column_count() -> usize {
    REPORT_COLUMNS.len()
}
