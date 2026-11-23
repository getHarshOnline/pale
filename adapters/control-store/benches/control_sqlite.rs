//! Non-default executable entrypoint for the crate-owned `SQLite` benchmark harness.
//!
//! Full characterization requires `PALE_BENCH_RUN=1`; smoke validation uses
//! `PALE_SQLITE_BENCH_SMOKE=1`. With neither flag, Cargo target discovery
//! exits without performing or misclassifying a benchmark run.

fn main() -> std::process::ExitCode {
    pale_adapter_control_storage::run_control_sqlite_benchmark()
}
