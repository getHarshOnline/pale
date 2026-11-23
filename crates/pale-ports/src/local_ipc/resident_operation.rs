//! Transport-only frames for one bounded resident operation request.
//!
//! These types select a request shape on an already authenticated local
//! carrier. They do not select a deployment, provider, participant, route,
//! policy, signer, replay owner, or durable store. They are not public contract
//! families and carry no authority by themselves.

use serde::{Deserialize, Serialize};

/// Exact durable locator copied into one resident operation frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ResidentOperationFrameLocator {
    operation_handle_record_ref: String,
    operation_handle_sha256: String,
}

impl ResidentOperationFrameLocator {
    /// Preserve an immutable operation-handle reference and digest.
    #[must_use]
    pub fn new(operation_handle_record_ref: String, operation_handle_sha256: String) -> Self {
        Self {
            operation_handle_record_ref,
            operation_handle_sha256,
        }
    }

    /// Borrow the immutable operation-handle storage reference.
    #[must_use]
    pub fn operation_handle_record_ref(&self) -> &str {
        &self.operation_handle_record_ref
    }

    /// Borrow the canonical operation-handle digest.
    #[must_use]
    pub fn operation_handle_sha256(&self) -> &str {
        &self.operation_handle_sha256
    }
}

/// One request admitted by the private resident operation host.
///
/// The signed common-request body remains the semantic source. The outer tag
/// only selects which already-owned application journey must admit it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    deny_unknown_fields,
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ResidentOperationFrameRequest {
    /// Start one durable root invocation.
    StartInvoke {
        /// Exact pre-signed common request as UTF-8 canonical JSON.
        signed_common_request: String,
    },
    /// Advance one retained invocation through its selected implementation owner.
    AdvanceInvoke {
        /// Exact retained invocation coordinate.
        locator: ResidentOperationFrameLocator,
        /// Exact pre-signed common request as UTF-8 canonical JSON.
        signed_common_request: String,
    },
    /// Start one already-selected bounded consultation.
    StartConsult {
        /// Exact pre-signed common request as UTF-8 canonical JSON.
        signed_common_request: String,
    },
    /// Advance one retained consultation by at most one mechanical turn.
    AdvanceConsult {
        /// Exact retained consultation coordinate.
        locator: ResidentOperationFrameLocator,
        /// Exact pre-signed common request as UTF-8 canonical JSON.
        signed_common_request: String,
    },
    /// Start one already-selected pairwise child delegation.
    StartDelegate {
        /// Exact immediate parent coordinate.
        parent: ResidentOperationFrameLocator,
        /// Exact pre-signed common request as UTF-8 canonical JSON.
        signed_common_request: String,
    },
    /// Advance one retained pairwise delegation through its selected owner.
    AdvanceDelegate {
        /// Exact retained child coordinate.
        locator: ResidentOperationFrameLocator,
        /// Exact pre-signed common request as UTF-8 canonical JSON.
        signed_common_request: String,
    },
    /// Request cancellation of one retained operation.
    Cancel {
        /// Exact retained operation coordinate.
        locator: ResidentOperationFrameLocator,
        /// Exact pre-signed common request as UTF-8 canonical JSON.
        signed_common_request: String,
    },
}

impl ResidentOperationFrameRequest {
    /// Borrow the exact pre-signed common request carried by this frame.
    #[must_use]
    pub fn signed_common_request(&self) -> &str {
        match self {
            Self::StartInvoke {
                signed_common_request,
            }
            | Self::AdvanceInvoke {
                signed_common_request,
                ..
            }
            | Self::StartConsult {
                signed_common_request,
            }
            | Self::AdvanceConsult {
                signed_common_request,
                ..
            }
            | Self::StartDelegate {
                signed_common_request,
                ..
            }
            | Self::AdvanceDelegate {
                signed_common_request,
                ..
            }
            | Self::Cancel {
                signed_common_request,
                ..
            } => signed_common_request,
        }
    }
}

/// Exact signed handle and current observation returned by the resident.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ResidentOperationFrameRecords {
    operation_handle_record_ref: String,
    operation_handle_canonical: String,
    operation_observation_record_ref: String,
    operation_observation_canonical: String,
}

impl ResidentOperationFrameRecords {
    /// Preserve exact record references and UTF-8 canonical bytes.
    #[must_use]
    pub fn new(
        operation_handle_record_ref: String,
        operation_handle_canonical: String,
        operation_observation_record_ref: String,
        operation_observation_canonical: String,
    ) -> Self {
        Self {
            operation_handle_record_ref,
            operation_handle_canonical,
            operation_observation_record_ref,
            operation_observation_canonical,
        }
    }

    /// Borrow the immutable operation-handle storage reference.
    #[must_use]
    pub fn operation_handle_record_ref(&self) -> &str {
        &self.operation_handle_record_ref
    }

    /// Borrow the exact canonical `OperationHandleV1` JSON.
    #[must_use]
    pub fn operation_handle_canonical(&self) -> &str {
        &self.operation_handle_canonical
    }

    /// Borrow the immutable operation-observation storage reference.
    #[must_use]
    pub fn operation_observation_record_ref(&self) -> &str {
        &self.operation_observation_record_ref
    }

    /// Borrow the exact canonical `OperationObservationV1` JSON.
    #[must_use]
    pub fn operation_observation_canonical(&self) -> &str {
        &self.operation_observation_canonical
    }
}

/// Honest result of one already-authorized cancellation request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResidentOperationFrameCancellationDisposition {
    /// The operation was already terminal before this request.
    TerminalAlreadyEstablished,
    /// The same cancellation request was already retained.
    AlreadyRequested,
    /// The cancellation request reached durable custody.
    RequestRetained,
    /// A local operation reached terminal cancellation custody.
    LocallyCancelled,
    /// A dispatched operation retained the request and received a local notification.
    DispatchedRequestRetained,
    /// A local A2A response read stopped without proving remote cancellation.
    A2aResponseReadStopped,
    /// Consultation collection had already completed.
    ConsultationCollected,
    /// Recovery or uncertainty requires reconciliation.
    ReconciliationOnly,
    /// The selected dispatch needs another exact process owner.
    DispatchOwnerRequired,
    /// The consultation request and stopped roster reached custody.
    ConsultationStopped,
    /// The target is not an open consultation.
    NotCollecting,
}

/// Why the selected bounded journey cannot advance through this request host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResidentOperationFrameUnsupportedReason {
    /// No provider or A2A route was already selected for this delegation.
    DelegateRouteNotSelected,
}

/// Stable transport-only rejection classes for resident operation requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResidentOperationFrameFailure {
    /// The frame or signed request is malformed, oversized, or misbound.
    InvalidRequest,
    /// The same request identity was retained with different signed bytes.
    ReplayConflict,
    /// Caller, session, current authority, or disclosure admission failed.
    AdmissionDenied,
    /// The exact compiled policy or configuration required by this turn is unavailable.
    ConfigurationUnavailable,
    /// Protected recovery or resident work did not complete.
    ResidentUnavailable,
}

/// One closed result of a single bounded resident operation request.
///
/// `Pending` never means success. `Unsupported` is limited to an admitted
/// Delegate for which the compiled constitution selected no route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    deny_unknown_fields,
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ResidentOperationFrameResponse {
    /// Return the durable start records for a new operation or an exact Start replay.
    Started {
        /// Exact signed records for the durable start.
        records: ResidentOperationFrameRecords,
    },
    /// Return the exact current records after this bounded turn.
    Observed {
        /// Exact signed records for the selected current head.
        records: ResidentOperationFrameRecords,
    },
    /// A cancellation request returned one exact owner-defined disposition.
    CancellationDisposition {
        /// Exact target coordinate.
        locator: ResidentOperationFrameLocator,
        /// The cancellation owner's exact disposition.
        disposition: ResidentOperationFrameCancellationDisposition,
    },
    /// The operation remains valid but needs a later bounded turn.
    Pending {
        /// Exact retained operation coordinate.
        locator: ResidentOperationFrameLocator,
    },
    /// The compiled constitution selected no Delegate route.
    Unsupported {
        /// Exact retained operation coordinate.
        locator: ResidentOperationFrameLocator,
        /// Owner boundary that prevented advancement.
        reason: ResidentOperationFrameUnsupportedReason,
    },
    /// Bounded transport projection of an application-owned rejection.
    Rejected {
        /// Stable transport-only failure class.
        failure: ResidentOperationFrameFailure,
    },
}
