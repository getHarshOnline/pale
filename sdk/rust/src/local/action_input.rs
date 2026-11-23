//! Read one confined file into an owned native-action request without executing it.
//!
//! The filesystem adapter enforces path confinement and the existing input-byte
//! limit. The action registry still owns parsing and execution. Capturing these
//! bytes grants no workspace, runtime, or mutation authority.

use std::path::PathBuf;

use pale_application::ApplicationFailureDisposition;
use pale_core::records::DiagnosticStageV1;

use crate::{ApiError, NativeActionRequest, PaleApi};

use super::local_host_failure;

/// Confined local source for one effect-free native action request.
///
/// The root and relative name are ephemeral host inputs, not serialized Pale
/// coordinates. [`PaleApi::capture_local_native_action_request`] opens the
/// root as a retained descriptor capability and enforces the same byte ceiling
/// as [`NativeActionRequest`] before returning path-free owned bytes.
pub struct LocalNativeActionInputRequest {
    action_id: String,
    root: String,
    relative_path: String,
}

impl std::fmt::Debug for LocalNativeActionInputRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalNativeActionInputRequest")
            .field("action_id", &self.action_id)
            .field("root_byte_len", &self.root.len())
            .field("relative_path_byte_len", &self.relative_path.len())
            .finish()
    }
}

impl LocalNativeActionInputRequest {
    /// Collect an action identity and file location without reading the file.
    ///
    /// Path and file checks occur in [`PaleApi::capture_local_native_action_request`].
    #[must_use]
    pub fn new(action_id: String, root: String, relative_path: String) -> Self {
        Self {
            action_id,
            root,
            relative_path,
        }
    }
}

impl PaleApi {
    /// Read one confined local file into a path-free native-action request.
    ///
    /// The filesystem adapter owns descriptor-relative confinement and bounded
    /// observation. This operation performs no action parsing or execution;
    /// callers pass the returned [`NativeActionRequest`] to
    /// [`Self::invoke_native_action`] so the action registry remains the sole
    /// semantic owner.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the root is not absolute, cannot be retained,
    /// the relative name escapes confinement, the file is absent or not a
    /// confined regular file, or its body exceeds [`NativeActionRequest::MAX_BYTES`].
    pub fn capture_local_native_action_request(
        &self,
        request: LocalNativeActionInputRequest,
    ) -> Result<NativeActionRequest, ApiError> {
        let LocalNativeActionInputRequest {
            action_id,
            root,
            relative_path,
        } = request;
        let root_path = PathBuf::from(&root);
        if !root_path.is_absolute() {
            return Err(local_action_failure(
                &self.diagnostic_identity,
                LocalActionInputError::NonAbsoluteRoot,
            ));
        }
        let root = pale_adapter_fs::ConfinedRoot::open(&root_path)
            .map_err(LocalActionInputError::Root)
            .map_err(|error| local_action_failure(&self.diagnostic_identity, error))?;
        let location = root
            .location(&relative_path)
            .map_err(LocalActionInputError::RelativePath)
            .map_err(|error| local_action_failure(&self.diagnostic_identity, error))?;
        let file = location
            .read_file(pale_adapter_fs::ReadLimit::new(
                NativeActionRequest::MAX_BYTES,
            ))
            .map_err(LocalActionInputError::Read)
            .map_err(|error| local_action_failure(&self.diagnostic_identity, error))?
            .ok_or_else(|| {
                local_action_failure(&self.diagnostic_identity, LocalActionInputError::Missing)
            })?;
        Ok(NativeActionRequest::new(action_id, file.bytes))
    }
}

#[derive(Debug, thiserror::Error)]
enum LocalActionInputError {
    #[error("action input root must be absolute")]
    NonAbsoluteRoot,
    #[error("action input root could not be retained")]
    Root(#[source] pale_adapter_fs::FsError),
    #[error("action input relative path was rejected")]
    RelativePath(#[source] pale_adapter_fs::FsError),
    #[error("action input file could not be read within its bound")]
    Read(#[source] pale_adapter_fs::FsError),
    #[error("action input file does not exist")]
    Missing,
}

fn local_action_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: LocalActionInputError,
) -> ApiError {
    let disposition = match &error {
        LocalActionInputError::Read(pale_adapter_fs::FsError::FileReadLimit { .. }) => {
            ApplicationFailureDisposition::InvalidInput
        }
        LocalActionInputError::Root(_) | LocalActionInputError::Read(_) => {
            ApplicationFailureDisposition::Operational
        }
        LocalActionInputError::NonAbsoluteRoot
        | LocalActionInputError::RelativePath(_)
        | LocalActionInputError::Missing => ApplicationFailureDisposition::InvalidInput,
    };
    local_host_failure(
        identity,
        error,
        disposition,
        DiagnosticStageV1::HostObservation,
        "local_action_input_rejected",
        "the confined native-action input could not be admitted",
    )
}
