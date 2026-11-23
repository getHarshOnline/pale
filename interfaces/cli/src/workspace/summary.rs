//! Terminal delivery of the API-owned semantic summary.

use anyhow::Result;
use pale_sdk::OneShotCompilationSummary;

use crate::cli::ResultFormatArg;

/// Emit the API-owned compiler summary as inspectable JSON.
///
/// The CLI owns terminal delivery only. Field selection, path readiness,
/// activation selection, hook requirements, and canonical identity all remain
/// owned by [`OneShotCompilationSummary`].
pub(crate) fn cmd_summary(
    summary: &OneShotCompilationSummary,
    format: ResultFormatArg,
) -> Result<()> {
    match format {
        ResultFormatArg::Human => {
            let output =
                crate::terminal::render_pretty_json(summary.canonical_bytes(), "semantic summary")?;
            crate::terminal::write_stdout(format_args!("{output}"))
        }
        ResultFormatArg::Json => {
            let result =
                crate::terminal::parse_owner_json(summary.canonical_bytes(), "semantic summary")?;
            crate::terminal::write_json_result(
                crate::terminal::ResultCompletion::Complete,
                &result,
                "semantic summary",
            )
        }
    }
}
