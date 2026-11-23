//! Non-default executable entrypoint for assembled resident characterization.
//!
//! Full characterization requires `PALE_BENCH_RUN=1`; smoke validation uses
//! `PALE_RESIDENT_BENCH_SMOKE=1`. Ordinary target discovery performs no work.

fn main() -> std::process::ExitCode {
    pale_adapter_control_storage::run_resident_application_benchmark()
}
