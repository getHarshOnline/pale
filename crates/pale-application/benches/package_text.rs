//! Non-default executable entrypoint for immutable Package-text characterization.
//!
//! Full characterization requires `PALE_BENCH_RUN=1`; smoke validation uses
//! `PALE_PACKAGE_TEXT_BENCH_SMOKE=1`. With neither flag, Cargo target
//! discovery exits without performing or misclassifying a benchmark run.

fn main() -> std::process::ExitCode {
    pale_application::packages::text_benchmark::run_package_text_benchmark()
}
