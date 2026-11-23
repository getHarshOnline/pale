//! The exact native occurrence carried inside a caller-signed common request.

use serde::{Deserialize, Serialize};

use super::{LifecycleEvent, LogicalRepositoryIdV1, ProviderBindingRefV1, WorkspaceId};

/// Schema root for a native tool occurrence awaiting application admission.
pub const NATIVE_TOOL_REQUEST_V1_SCHEMA_ID: &str = "native-tool-request.v1";

/// Candidate native event and stable route covered by the caller's signature.
///
/// The enclosing common request supplies schema and payload digests, session,
/// grant, operation and requested capability. These fields duplicate none of
/// those changing coordinates. Deserialization supplies no permission: the
/// application checks native event agreement, current authority and the exact
/// compiled provider before it can bind a durably issued operation.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeToolRequestV1 {
    /// Pre-effect lifecycle event selected by the host contribution.
    pub event: LifecycleEvent,
    /// Workspace named by that contribution.
    pub workspace_id: WorkspaceId,
    /// Repository from which the native event originated.
    pub repository_id: LogicalRepositoryIdV1,
    /// Exact terminal provider that owns this event route.
    pub provider_binding: ProviderBindingRefV1,
    /// Original UTF-8 event bytes, preserved without parsing and reserialization.
    /// The application bounds bytes and admits the native event's strict JSON.
    #[schemars(length(min = 1, max = 1048576))]
    pub native_event: String,
}

impl std::fmt::Debug for NativeToolRequestV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("NativeToolRequestV1(<candidate native event>)")
    }
}
