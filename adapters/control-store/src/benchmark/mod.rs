//! Bench-feature-only ownership of `SQLite` characterization mechanics and reports.

mod adapter_workload;
mod evidence;
mod input;
mod memory;
mod report;
#[cfg(feature = "resident-bench")]
pub(crate) mod resident_application;
mod sqlite_characterization;
mod statistics;
mod workload;

use std::io::{self, Write};
use std::process::{Command, ExitCode};

use input::BenchmarkConfiguration;
use memory::measure_case;
use report::BenchmarkReporter;
use workload::{
    crash_writer_request, private_tempdir, run_case_sample, run_crash_writer, run_recovery_sample,
    CaseMeasurements,
};

/// Child-mode marker used to preserve committed WAL state across process loss.
const CHILD_MODE: &str = "PALE_SQLITE_BENCH_CHILD_MODE";

/// Run the non-default benchmark harness without selecting or admitting a profile.
pub(crate) fn run() -> ExitCode {
    if std::env::var_os(CHILD_MODE).is_some() {
        run_crash_writer(&crash_writer_request());
    }

    let Some(configuration) = BenchmarkConfiguration::from_environment() else {
        eprintln!("SQLite benchmark skipped; set PALE_BENCH_RUN=1 or PALE_SQLITE_BENCH_SMOKE=1");
        return ExitCode::SUCCESS;
    };
    let reporter = BenchmarkReporter::new(&configuration);
    let executable = std::env::current_exe().expect("resolve benchmark executable");
    let mut rows = Vec::with_capacity(configuration.expected_rows);

    for (case_index, case) in configuration.cases().into_iter().enumerate() {
        let measurements = measure_case(&case, || {
            let mut measurements = CaseMeasurements::default();
            for _ in 0..configuration.independent_runs {
                let regular = run_case_sample(&case);
                let recovery_root = private_tempdir();
                let mut command = Command::new(&executable);
                command
                    .env(CHILD_MODE, "1")
                    .env("PALE_SQLITE_BENCH_CHILD_ROOT", recovery_root.path())
                    .env(
                        "PALE_SQLITE_BENCH_CHILD_WORKLOAD_CLASS",
                        case.workload_class.as_str(),
                    )
                    .env("PALE_SQLITE_BENCH_CHILD_RECORDS", case.records.to_string())
                    .env(
                        "PALE_SQLITE_BENCH_CHILD_PAYLOAD_BYTES",
                        case.payload_bytes.to_string(),
                    )
                    .env(
                        "PALE_SQLITE_BENCH_CHILD_CHECKPOINT_BYTES",
                        case.checkpoint_bytes.to_string(),
                    )
                    .env(
                        "PALE_SQLITE_BENCH_CHILD_MAXIMUM_ENTRY_BYTES",
                        case.limits.maximum_entry_bytes().to_string(),
                    )
                    .env(
                        "PALE_SQLITE_BENCH_CHILD_MAXIMUM_CHECKPOINT_BYTES",
                        case.limits.maximum_checkpoint_bytes().to_string(),
                    )
                    .env(
                        "PALE_SQLITE_BENCH_CHILD_MAXIMUM_CHECKPOINT_CANDIDATES",
                        case.limits.maximum_checkpoint_candidates().to_string(),
                    )
                    .env(
                        "PALE_SQLITE_BENCH_CHILD_MAXIMUM_TAIL_ENTRIES",
                        case.limits.maximum_tail_entries().to_string(),
                    );
                if let Some(candidate) = &case.pragma_candidate {
                    command.env("PALE_SQLITE_BENCH_CHILD_PROFILE", candidate.encode());
                }
                let status = command
                    .status()
                    .expect("start crash-writer benchmark child");
                assert!(status.success(), "crash-writer benchmark child failed");
                let recovery = run_recovery_sample(&case, recovery_root.path());
                measurements.record_independent_run(regular, recovery);
            }
            measurements
        });
        rows.push(reporter.render_row(case_index + 1, &case, &measurements));
    }
    let report = reporter.render_complete_report(&rows);
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(report.as_bytes())
        .and_then(|()| stdout.flush())
        .expect("write complete sealed benchmark report");
    ExitCode::SUCCESS
}

#[cfg(test)]
#[path = "../../tests/internal/benchmark.rs"]
mod tests;
