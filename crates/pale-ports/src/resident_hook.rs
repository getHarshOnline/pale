//! Local hook frames carry observations, never session or operation authority.

use serde::{Deserialize, Serialize};

/// One native event and the repository command that selected its route.
///
/// The event remains a string so its exact bytes survive transport. The
/// application checks the command grammar and native event after decoding.
#[derive(Clone, Serialize, Deserialize)]
#[serde(
    deny_unknown_fields,
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ResidentHookFrameRequest {
    /// Deliver one original event under its static repository route.
    Hook {
        /// Exact arguments following the `pale` executable.
        arguments: [String; 10],
        /// Original UTF-8 event bytes, preserved inside the frame.
        native_event: String,
    },
}

impl ResidentHookFrameRequest {
    /// Preserve the arguments following `pale` and the original UTF-8 event.
    #[must_use]
    pub const fn new(arguments: [String; 10], native_event: String) -> Self {
        Self::Hook {
            arguments,
            native_event,
        }
    }

    /// Borrow the exact repository-hook command arguments.
    #[must_use]
    pub const fn arguments(&self) -> &[String; 10] {
        let Self::Hook { arguments, .. } = self;
        arguments
    }

    /// Borrow the original native event without reserializing its JSON.
    #[must_use]
    pub fn native_event(&self) -> &[u8] {
        let Self::Hook { native_event, .. } = self;
        native_event.as_bytes()
    }
}

impl std::fmt::Debug for ResidentHookFrameRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ResidentHookFrameRequest(<private event>)")
    }
}

/// A refusal or exact response awaiting independent caller-side admission.
/// Transport authentication alone cannot authorize the returned native output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
pub enum ResidentHookFrameResponse {
    /// Exact output bound to a durably dispatched operation observation.
    /// The receiver must independently admit the signed records and response binding.
    Prepared {
        /// Signed operation identity and dispatch, requiring caller-side admission.
        records: crate::ResidentOperationFrameRecords,
        /// Canonical response record named by the dispatch evidence digest.
        response_canonical: String,
    },
    /// No current session-to-operation handler is installed for native hooks.
    HandlerUnavailable {},
    /// Current caller, occurrence or protected operation admission failed.
    Rejected {
        /// Bounded failure class; no private request or authority data is returned.
        failure: crate::ResidentOperationFrameFailure,
    },
}

/// Response bytes and their exact operation/native-event association.
/// This record has no independent authority until its signed dispatch is admitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResidentHookResponseRecord {
    /// Private response schema, preventing cross-family interpretation.
    pub schema_id: pale_core::records::SchemaId,
    /// Exact handle this response may advance.
    pub handle_sha256: pale_core::records::OperationHandleDigest,
    /// Caller-signed native request body, including route and original event bytes.
    pub native_request_sha256: pale_core::records::ArtifactDigest,
    /// Independently admitted action permission and approval evidence.
    pub permission: pale_core::records::ImmutableRecordReferenceV1,
    /// Independently admitted disclosure evidence.
    pub disclosure: pale_core::records::ImmutableRecordReferenceV1,
    /// Exact host-standard output, already encoded for its protocol.
    pub stdout: String,
    /// Exact host-standard error output.
    pub stderr: String,
    /// Requested native process status; this is not evidence of actual emission.
    pub exit_code: i32,
}
