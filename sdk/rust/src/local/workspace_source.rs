//! Inspect local authored meaning before preparing any publication metadata.

use super::{local_workspace_failure, LocalWorkspaceLocator};
use crate::{ApiError, PaleApi};

/// Local source report; it cannot be used as an admitted workspace handle.
#[derive(Debug)]
pub struct WorkspaceSourceInspection {
    canonical_bytes: Vec<u8>,
    content_digest: String,
}

impl WorkspaceSourceInspection {
    /// Borrow the report, including exact source evidence and its source-only phase.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Borrow the identity of the lowered meaning, excluding source whitespace.
    #[must_use]
    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }
}

impl PaleApi {
    /// Check local module content, project owners and repository duties.
    ///
    /// Reads only manifest-selected source. It does not read machine bindings,
    /// Package distributions or credentials, prepare releases, or activate anything.
    ///
    /// # Errors
    ///
    /// Returns the normal structured workspace diagnostic for invalid source,
    /// unresolved references or a locator outside the source boundary.
    pub fn inspect_local_workspace_source(
        &self,
        workspace: LocalWorkspaceLocator,
    ) -> Result<WorkspaceSourceInspection, ApiError> {
        let inspect = || {
            let manifest = workspace.into_path()?;
            let lowered = pale_application::workspace::lower_local_workspace_source(
                &pale_adapter_fs::LocalSourceFilesystem,
                &manifest,
            )?;
            let canonical_bytes = lowered
                .canonical_bytes()
                .map_err(pale_application::workspace::WorkspaceError::Canon)?;
            Ok(WorkspaceSourceInspection {
                canonical_bytes,
                content_digest: lowered.content_digest().to_owned(),
            })
        };
        inspect().map_err(|error| local_workspace_failure(&self.diagnostic_identity, error))
    }
}
