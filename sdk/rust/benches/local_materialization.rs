//! Non-default entrypoint for complete local materialization characterization.
//!
//! `PALE_BENCH_RUN=1` selects characterization and the separate smoke flag
//! selects compile-and-execute validation. With neither flag, Cargo target
//! discovery exits without performing or misclassifying a benchmark run.

#[path = "support/local_materialization.rs"]
mod local_materialization;

fn main() -> std::process::ExitCode {
    local_materialization::run()
}
