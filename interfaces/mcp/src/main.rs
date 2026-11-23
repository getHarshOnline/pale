//! `pale-mcp` stdio transport process.
//!
//! All Pale command meaning is delegated to [`pale_cli::command`]; this
//! binary owns only process termination and transport startup.

use std::process::ExitCode;

fn main() -> ExitCode {
    match pale_mcp::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("pale-mcp: {error}");
            ExitCode::FAILURE
        }
    }
}
