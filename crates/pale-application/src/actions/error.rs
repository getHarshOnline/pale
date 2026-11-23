//! Action errors preserve stable record identity and request-digest attribution.

use pale_core::records::{ActionError, ActionErrorCode, ActionRequestDigest};

use super::{ACTION_RECORD_SCHEMA_VERSION, ERROR_RECORD};

/// Construct an action-scoped failure without inventing request identity.
pub(super) fn action_error(
    action_id: Option<&str>,
    code: ActionErrorCode,
    message: impl Into<String>,
) -> ActionError {
    ActionError {
        schema: ERROR_RECORD.to_string(),
        schema_version: ACTION_RECORD_SCHEMA_VERSION,
        action_id: action_id.map(str::to_string),
        code,
        message: message.into(),
        request_digest: None,
    }
}

/// Reject an action identifier absent from the sealed native registry.
pub(super) fn unknown_action_error(action_id: &str) -> ActionError {
    action_error(
        Some(action_id),
        ActionErrorCode::UnknownAction,
        "action ID is not present in the native registry",
    )
}

/// Attach a request identifier only after its envelope has supplied one.
pub(super) fn request_error(
    action_id: &str,
    request_digest: &ActionRequestDigest,
    code: ActionErrorCode,
    message: impl Into<String>,
) -> ActionError {
    let mut error = action_error(Some(action_id), code, message);
    error.request_digest = Some(request_digest.clone());
    error
}
