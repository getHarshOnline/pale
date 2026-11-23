//! Admit repository-hook input without local policy evaluation.
//!
//! The command carries only the stable routing fields in [`ResidentHookRequest`]
//! rendered into native repository configuration and one bounded native event
//! from standard input. Compilation generations, execution paths, policy,
//! context, sessions, and effects remain resident-owned. An embedder can supply
//! an authenticated carrier; the executable can load an explicitly selected
//! protected hook connection configuration.

use std::io::{IsTerminal as _, Read as _, Write as _};

use anyhow::{Context as _, Result};

use crate::command::{CommandDispatcher, CommandRequest, ResidentHookRequest};
use crate::DispatchFailure;

/// Stable command-line routing for one repository-native event.
pub(super) struct HookInvocation<'a> {
    /// Lifecycle event selected by the native configuration entry.
    pub(super) event: &'a str,
    /// Workspace identity carried by the native hook configuration.
    pub(super) workspace: &'a str,
    /// Canonical repository from which the event originated.
    pub(super) repository: &'a str,
    /// Stable locator of the exact terminal provider binding.
    pub(super) provider_binding: &'a str,
    /// Domain-separated identity of the complete provider-binding body.
    pub(super) provider_binding_identity: &'a str,
}

/// Read one native event with a single byte of overflow evidence.
fn read_event() -> Result<Vec<u8>> {
    if std::io::stdin().is_terminal() {
        return Ok(Vec::new());
    }
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(ResidentHookRequest::MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .context("read bounded native hook event")?;
    Ok(bytes)
}

/// Admit one native event and deliver it through an explicitly supplied attachment.
///
/// # Errors
///
/// Returns an error when routing or native input admission fails, or when this
/// invocation has no authenticated resident carrier or usable handler. It never
/// recompiles a workspace, reads Package source, loads a repository-local semantic
/// projection, or falls back to one-shot policy evaluation.
pub(super) fn run_hook(
    invocation: HookInvocation<'_>,
    attachment: Option<crate::command::ResidentAttachment>,
    configuration: Option<&std::path::Path>,
) -> std::result::Result<std::process::ExitCode, DispatchFailure> {
    let bytes = read_event()?;
    let attachment = match (attachment, configuration) {
        (Some(_), Some(_)) => {
            return Err(anyhow::anyhow!("select one resident hook attachment").into())
        }
        (attachment, None) => attachment,
        (None, Some(path)) => Some(load_attachment(path)?),
    };
    let dispatcher = match attachment {
        Some(attachment) => CommandDispatcher::with_resident_attachment(attachment),
        None => CommandDispatcher::new(),
    }
    .context("initialize the Pale command dispatcher")?;
    match dispatcher.dispatch(CommandRequest::ResidentHook(ResidentHookRequest::new(
        invocation.event.to_owned(),
        invocation.workspace.to_owned(),
        invocation.repository.to_owned(),
        invocation.provider_binding.to_owned(),
        invocation.provider_binding_identity.to_owned(),
        bytes,
    ))) {
        Err(error) => Err(error.into()),
        Ok(crate::command::CommandOutcome::ResidentHook(output)) => {
            // Flush the diagnostic channel first. A failed emitter must never
            // return the success status requested by an un-emitted response.
            let mut stderr = std::io::stderr().lock();
            stderr
                .write_all(output.stderr().as_bytes())
                .context("write native hook standard error")?;
            stderr.flush().context("flush native hook standard error")?;
            let mut stdout = std::io::stdout().lock();
            stdout
                .write_all(output.stdout().as_bytes())
                .context("write native hook standard output")?;
            stdout
                .flush()
                .context("flush native hook standard output")?;
            Ok(std::process::ExitCode::from(output.exit_code()))
        }
        Ok(_) => Err(crate::diagnostic::internal_failure(
            "resident-hook dispatch returned an unsupported success result",
            anyhow::anyhow!("resident hook command returned another outcome family"),
        )
        .into()),
    }
}

fn load_attachment(path: &std::path::Path) -> Result<crate::command::ResidentAttachment> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        Ok(crate::command::ResidentAttachment::load_hook_configuration(
            path.to_str()
                .context("resident hook configuration path must be UTF-8")?,
        )?)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = path;
        anyhow::bail!("resident hook configuration is unsupported on this platform")
    }
}

#[cfg(test)]
#[path = "../tests/internal/hook.rs"]
mod tests;
