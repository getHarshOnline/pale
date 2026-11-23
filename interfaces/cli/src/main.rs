//! Pale project-surface CLI binary.
//!
//! Delegates command handling to [`pale_cli::run`].

use std::process::ExitCode;

fn main() -> ExitCode {
    pale_cli::run()
}
