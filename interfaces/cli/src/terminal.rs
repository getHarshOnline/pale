//! Process-stream writes classified by whether reporting is part of success.

use std::fmt;
use std::io::{self, Write};

use anyhow::{Context, Result};
use serde::Serialize;

/// CLI-owned terminal envelope around one typed result projection.
#[derive(Serialize)]
struct ResultEnvelope<'a, T: ?Sized> {
    completion: &'static str,
    result: &'a T,
}

/// Whether the returned value completes the selected command invocation.
///
/// This is a CLI transport classification over explicit SDK or dispatcher
/// outcomes. It is not a workflow, operation, or application lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResultCompletion {
    /// The command returned its complete result.
    Complete,
    /// The command returned an admitted bounded subset or partial settlement.
    Partial,
    /// The command returned an admitted stage that requires a later decision.
    NonTerminal,
}

impl ResultCompletion {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::NonTerminal => "non_terminal",
        }
    }
}

/// Render one API-owned JSON value for terminal inspection without changing
/// the semantic record, field selection, or canonical bytes held by its owner.
///
/// # Errors
///
/// Returns an internal CLI projection failure if the owner-supplied bytes are
/// not JSON or cannot be rendered as a two-space-indented document.
pub(crate) fn render_pretty_json(bytes: &[u8], subject: &'static str) -> Result<String> {
    let value = parse_owner_json(bytes, subject)?;
    serde_json::to_string_pretty(&value).map_err(|error| {
        crate::diagnostic::internal_failure(
            format!("the API-owned {subject} projection could not be rendered"),
            error.into(),
        )
    })
}

/// Decode one exact SDK-owned JSON projection without reconstructing its fields.
pub(crate) fn parse_owner_json(bytes: &[u8], subject: &'static str) -> Result<serde_json::Value> {
    serde_json::from_slice(bytes).map_err(|error| {
        crate::diagnostic::internal_failure(
            format!("the API-owned {subject} projection is not valid JSON"),
            error.into(),
        )
    })
}

/// Write one deterministic JSON result envelope to standard output.
///
/// `result` must be a complete owner value or a typed CLI transport projection.
/// Callers must not pass a debug or display-only projection.
pub(crate) fn write_json_result<T: Serialize + ?Sized>(
    completion: ResultCompletion,
    result: &T,
    subject: &'static str,
) -> Result<()> {
    let output = render_json_result(completion, result, subject)?;
    write_stdout(format_args!("{output}"))
}

/// Render one CLI-owned envelope around a typed result projection.
pub(crate) fn render_json_result<T: Serialize + ?Sized>(
    completion: ResultCompletion,
    result: &T,
    subject: &'static str,
) -> Result<String> {
    let envelope = ResultEnvelope {
        completion: completion.as_str(),
        result,
    };
    serde_json::to_string_pretty(&envelope).map_err(|error| {
        crate::diagnostic::internal_failure(
            format!("the CLI-owned {subject} envelope could not be rendered"),
            error.into(),
        )
    })
}

/// Report a result after its owner has already established an external effect.
///
/// Output failure cannot change the committed result, so this helper retains
/// the same best-effort behavior as [`report_stdout`].
pub(crate) fn report_json_result<T: Serialize + ?Sized>(
    completion: ResultCompletion,
    result: &T,
    subject: &'static str,
) {
    let output = render_json_result(completion, result, subject);
    if let Ok(output) = output {
        report_stdout(format_args!("{output}"));
    }
}

/// Write one newline-terminated result to the selected process stream.
pub(crate) fn write_line(writer: &mut impl Write, arguments: fmt::Arguments<'_>) -> io::Result<()> {
    writeln!(writer, "{arguments}")
}

/// Make one best-effort newline-terminated report to the selected stream.
pub(crate) fn report_line(writer: &mut impl Write, arguments: fmt::Arguments<'_>) {
    drop(write_line(writer, arguments));
}

/// Write required command output and preserve an I/O failure for the caller.
pub(crate) fn write_stdout(arguments: fmt::Arguments<'_>) -> Result<()> {
    write_line(&mut io::stdout().lock(), arguments).context("write command output")
}

/// Report an already-established effect without changing its outcome.
pub(crate) fn report_stdout(arguments: fmt::Arguments<'_>) {
    report_line(&mut io::stdout().lock(), arguments);
}

/// Report a failure or recovery observation without masking the primary result.
pub(crate) fn report_stderr(arguments: fmt::Arguments<'_>) {
    report_line(&mut io::stderr().lock(), arguments);
}

#[cfg(test)]
#[path = "../tests/internal/terminal.rs"]
mod tests;
