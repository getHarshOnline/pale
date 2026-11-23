//! Bounded outward A2A exchange mechanics, below application admission.
//!
//! These values select bytes and preserve correlations. They do not prove
//! durable dispatch, grant route authority, or classify remote settlement.

//! Only the application's consuming post-custody phase may start an exchange.

use std::{
    num::NonZeroUsize,
    task::{Context, Poll},
    time::Duration,
};

use pale_core::records::{
    A2aHttpsEndpointV1, A2aJsonRpcRequestIdV1, A2aRemoteContextIdV1, A2aRemoteTaskIdV1,
    A2aRemoteTaskStateV1, A2aRouteAdmissionDigest, A2aRouteAdmissionV1, A2aTaskBindingV1,
    A2aTaskRequestModeV1, ArtifactDigest, CapabilityAttemptIdV1, CompiledGenerationDigest,
    InstanceDigest, OperationHandleDigest, OperationHandleV1, OperationObservationDigest,
    OperationObservationV1, PackageResourceRefV1, ProviderBindingIdentityV1, RequestBindingV1,
    ResidentRecordRefV1, MAX_RESIDENT_ID_BYTES,
};

use crate::ProviderInvocationRequest;
use pale_core::records::BoundedResidentTextV1;

/// Finite input, output and elapsed-time ceilings for one exchange.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct A2aExchangeLimits {
    /// Maximum serialized request bytes, including its JSON-RPC envelope.
    pub request_bytes: NonZeroUsize,
    /// Maximum unary response or total SSE wire bytes.
    pub response_bytes: NonZeroUsize,
    /// Maximum total bytes of delivered JSON, native IDs and SSE coordinates.
    pub evidence_bytes: NonZeroUsize,
    /// Maximum SSE line bytes, excluding its delimiter.
    pub line_bytes: NonZeroUsize,
    /// Maximum assembled SSE JSON bytes.
    pub event_bytes: NonZeroUsize,
    /// Maximum delivered SSE data events.
    pub events: NonZeroUsize,
    /// Maximum queued evidence items for this exchange.
    pub queued_events: NonZeroUsize,
    /// Deadline through DNS, verified connection and receipt of response headers.
    pub connect_timeout: Duration,
    /// Maximum inactivity during a read.
    pub read_timeout: Duration,
    /// Maximum duration from submission through local delivery, including backpressure.
    pub total_timeout: Duration,
}

impl A2aExchangeLimits {
    /// Reject inverted or empty limits before constructing requests.
    ///
    /// # Errors
    /// Returns [`A2aExchangeError::InvalidLimits`] for inconsistent ceilings.
    pub fn validate(self) -> Result<(), A2aExchangeError> {
        if self.line_bytes > self.response_bytes
            || self.event_bytes > self.response_bytes
            || self.queued_events > self.events
            || self.connect_timeout.is_zero()
            || self.read_timeout.is_zero()
            || self.total_timeout.is_zero()
            || self.connect_timeout > self.total_timeout
            || self.read_timeout > self.total_timeout
        {
            return Err(A2aExchangeError::InvalidLimits);
        }
        Ok(())
    }
}

/// Exact retained parents supplied by the application, not dispatch permission.
pub struct A2aExchangeParents<'a> {
    /// Immutable operation-handle reference and admitted record.
    pub operation: (&'a ResidentRecordRefV1, &'a OperationHandleV1),
    /// Immutable observation reference and admitted record.
    pub observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    /// Immutable route reference and admitted record.
    pub route: (&'a ResidentRecordRefV1, &'a A2aRouteAdmissionV1),
}

/// Borrowed command input, copied only after checking byte limits.
#[derive(Clone, Copy)]
pub enum A2aExchangeCommand<'a> {
    /// Submit one bounded user text message. IDs are never allocated here.
    SendText {
        /// Unary or advertised streaming send.
        mode: A2aTaskRequestModeV1,
        /// Distinct caller-assigned native message identity.
        message_id: &'a str,
        /// Already selected and disclosure-bounded text.
        text: &'a str,
    },
    /// Observe an existing task without sending another message.
    GetTask(&'a A2aTaskBindingV1),
    /// Request cancellation; the response is not cancellation proof.
    CancelTask(&'a A2aTaskBindingV1),
}

/// Owned complete correlation retained once by an exchange reader.
///
/// Equality is a substitution fence, not proof of admission or durability.
#[derive(Clone, PartialEq, Eq)]
pub struct A2aExchangeContext {
    compiled_generation: CompiledGenerationDigest,
    instance: InstanceDigest,
    operation: PackageResourceRefV1,
    request_binding: RequestBindingV1,
    provider_binding: ProviderBindingIdentityV1,
    attempt: CapabilityAttemptIdV1,
    implementation_kind: BoundedResidentTextV1,
    implementation_version: BoundedResidentTextV1,
    handle: (ResidentRecordRefV1, OperationHandleDigest),
    observation: (ResidentRecordRefV1, OperationObservationDigest),
    route: (ResidentRecordRefV1, A2aRouteAdmissionDigest),
    request_id: A2aJsonRpcRequestIdV1,
}

impl A2aExchangeContext {
    /// Borrow the original JSON-RPC request identity.
    #[must_use]
    pub const fn request_id(&self) -> &A2aJsonRpcRequestIdV1 {
        &self.request_id
    }
}

enum OwnedCommand {
    SendText {
        mode: A2aTaskRequestModeV1,
        message_id: String,
        text: String,
    },
    GetTask {
        task: A2aRemoteTaskIdV1,
        context: A2aRemoteContextIdV1,
    },
    CancelTask {
        task: A2aRemoteTaskIdV1,
        context: A2aRemoteContextIdV1,
    },
}

/// One owned input that can cross the bounded service queue.
pub struct A2aExchangeRequest {
    coordinate: A2aExchangeContext,
    endpoint: A2aHttpsEndpointV1,
    command: OwnedCommand,
    limits: A2aExchangeLimits,
}

/// Borrow the selected wire input without exposing an upstream DTO.
pub enum A2aExchangeInput<'a> {
    /// The first carrier path supports text, not arbitrary structured data.
    Text {
        /// Unary send or advertised streaming send.
        mode: A2aTaskRequestModeV1,
        /// Native message identity, separate from the JSON-RPC request.
        message_id: &'a str,
        /// Selected disclosure-bounded message text.
        text: &'a str,
    },
    /// Read the identified remote task and verify its returned context.
    GetTask {
        /// Remote Task already identified by the retained binding.
        task: &'a A2aRemoteTaskIdV1,
        /// Remote context expected in the response.
        context: &'a A2aRemoteContextIdV1,
    },
    /// Request cancellation of the identified remote task.
    CancelTask {
        /// Remote Task already identified by the retained binding.
        task: &'a A2aRemoteTaskIdV1,
        /// Remote context expected in the response.
        context: &'a A2aRemoteContextIdV1,
    },
}

impl A2aExchangeRequest {
    /// Own an exact mechanically consistent request. No authority is granted.
    ///
    /// The application must first prove all contextual admission and custody
    /// obligations. This constructor only rejects unlike inputs and bounds
    /// copies. It never interprets a record reference as durable permission.
    ///
    /// # Errors
    /// Returns a limit, coordinate, or unsupported-client-auth error before
    /// copying message content or opening a connection.
    pub fn new(
        invocation: ProviderInvocationRequest<'_>,
        parents: A2aExchangeParents<'_>,
        request_id: &A2aJsonRpcRequestIdV1,
        command: A2aExchangeCommand<'_>,
        limits: A2aExchangeLimits,
    ) -> Result<Self, A2aExchangeError> {
        limits.validate()?;
        let (handle_ref, handle) = parents.operation;
        let (observation_ref, observation) = parents.observation;
        let (route_ref, route) = parents.route;
        if handle.request_binding() != invocation.request_binding()
            || observation.operation_handle_id() != handle.operation_handle_id()
            || route.provider_binding().provider_binding_identity
                != invocation.provider_binding().binding_identity
        {
            return Err(A2aExchangeError::CoordinateMismatch);
        }
        if !route.security().selected_scheme_ids.is_empty() {
            return Err(A2aExchangeError::ClientAuthenticationUnavailable);
        }
        let command = match command {
            A2aExchangeCommand::SendText {
                mode,
                message_id,
                text,
            } => {
                if message_id.is_empty()
                    || message_id.len() > MAX_RESIDENT_ID_BYTES
                    || message_id.chars().any(char::is_control)
                    || message_id
                        .len()
                        .checked_add(text.len())
                        .is_none_or(|len| len > limits.request_bytes.get())
                {
                    return Err(A2aExchangeError::LimitExceeded);
                }
                if mode == A2aTaskRequestModeV1::SendStreamingMessage
                    && !route.streaming().is_advertised()
                {
                    return Err(A2aExchangeError::StreamingUnavailable);
                }
                OwnedCommand::SendText {
                    mode,
                    message_id: message_id.to_owned(),
                    text: text.to_owned(),
                }
            }
            A2aExchangeCommand::GetTask(binding) | A2aExchangeCommand::CancelTask(binding) => {
                if binding.operation_handle_ref() != handle_ref
                    || binding.operation_handle_sha256()
                        != handle.canonical_operation_handle_sha256()
                    || binding.route_admission_ref() != route_ref
                    || binding.route_admission_sha256()
                        != route.canonical_a2a_route_admission_sha256()
                {
                    return Err(A2aExchangeError::CoordinateMismatch);
                }
                let task = binding.remote_task_id().clone();
                let context = binding.remote_context_id().clone();
                if matches!(command, A2aExchangeCommand::GetTask(_)) {
                    OwnedCommand::GetTask { task, context }
                } else {
                    OwnedCommand::CancelTask { task, context }
                }
            }
        };
        Ok(Self {
            coordinate: A2aExchangeContext {
                compiled_generation: invocation.compiled_generation_digest().clone(),
                instance: invocation.instance_digest().clone(),
                operation: invocation.operation().clone(),
                request_binding: invocation.request_binding().clone(),
                provider_binding: invocation.provider_binding().binding_identity.clone(),
                attempt: invocation.attempt_id().clone(),
                implementation_kind: invocation.implementation_kind().clone(),
                implementation_version: invocation.implementation_version().clone(),
                handle: (
                    handle_ref.clone(),
                    handle.canonical_operation_handle_sha256().clone(),
                ),
                observation: (
                    observation_ref.clone(),
                    observation.canonical_operation_observation_sha256().clone(),
                ),
                route: (
                    route_ref.clone(),
                    route.canonical_a2a_route_admission_sha256().clone(),
                ),
                request_id: request_id.clone(),
            },
            endpoint: route.endpoint().clone(),
            command,
            limits,
        })
    }

    /// Borrow the complete expected exchange correlation.
    #[must_use]
    pub const fn coordinate(&self) -> &A2aExchangeContext {
        &self.coordinate
    }
    /// Borrow the endpoint selected by the supplied admitted route.
    #[must_use]
    pub const fn endpoint(&self) -> &A2aHttpsEndpointV1 {
        &self.endpoint
    }
    /// Return this exchange's explicit ceilings.
    #[must_use]
    pub const fn limits(&self) -> A2aExchangeLimits {
        self.limits
    }
    /// Borrow the finite input without copying its text.
    #[must_use]
    pub fn input(&self) -> A2aExchangeInput<'_> {
        match &self.command {
            OwnedCommand::SendText {
                mode,
                message_id,
                text,
            } => A2aExchangeInput::Text {
                mode: *mode,
                message_id,
                text,
            },
            OwnedCommand::GetTask { task, context } => A2aExchangeInput::GetTask { task, context },
            OwnedCommand::CancelTask { task, context } => {
                A2aExchangeInput::CancelTask { task, context }
            }
        }
    }
}

/// Untrusted native facts. No variant asserts Pale success or settlement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum A2aExchangeFacts {
    /// An actual Task was decoded; the application still admits its binding.
    Task {
        /// Remote-owned task identity.
        task: A2aRemoteTaskIdV1,
        /// Remote-owned context identity.
        context: A2aRemoteContextIdV1,
        /// Reported remote state, not a Pale result.
        state: A2aRemoteTaskStateV1,
    },
    /// A task-less message cannot create a task binding.
    DirectMessage,
    /// A status event does not itself identify an initial Task object.
    Status {
        /// Task named by the event.
        task: A2aRemoteTaskIdV1,
        /// Context named by the event.
        context: A2aRemoteContextIdV1,
        /// Reported remote state, not a Pale result.
        state: A2aRemoteTaskStateV1,
    },
    /// Artifact content remains opaque evidence and conveys no task state.
    Artifact {
        /// Task named by the event.
        task: A2aRemoteTaskIdV1,
        /// Context named by the event.
        context: A2aRemoteContextIdV1,
    },
    /// Only the upstream numeric code is projected. Text remains private evidence.
    RemoteError {
        /// Original numeric JSON-RPC error code.
        code: i64,
    },
}

/// One bounded response or SSE event, before application evidence admission.
///
/// `bytes` contains the exact unary JSON or assembled SSE JSON data. SSE
/// framing is represented separately; this is not a raw TCP transcript.
pub struct A2aExchangeEvidence {
    /// Untrusted facts selected from the official DTO.
    pub facts: A2aExchangeFacts,
    /// Confined JSON evidence bytes, not a public diagnostic or payload result.
    pub bytes: Vec<u8>,
    /// Original inherited/reset SSE ID and event type, absent for unary results.
    pub sse: Option<(String, String)>,
}

/// Qualify exact retained unary `GetTask` protocol bytes without any I/O.
///
/// The implementation owns the selected official wire vocabulary and correlates
/// the original string request ID with its response. This proves protocol meaning
/// only: not HTTP/TLS provenance, route authority, observation time, durable
/// custody, current access, safe retry or a local operation outcome.
pub trait A2aGetTaskEvidencePort {
    /// Decode one selected `GetTask` request and its Task response, requiring
    /// both to name the expected remote task and the response's expected context.
    /// The request has `historyLength: 0` and no tenant or additional parameters.
    /// Whitespace and object order do not change meaning; bytes are never replaced.
    ///
    /// `maximum_bytes` bounds the sum of both inputs before allocation or parsing;
    /// implementations must also retain bounded JSON nesting and identifier sizes.
    /// A remote error or non-Task response cannot become a reported task state.
    ///
    /// # Errors
    /// Rejects excessive bytes, invalid or ambiguous protocol shape, unsupported
    /// method/input, unlike request correlation, task or context.
    fn read_task_state(
        &self,
        request: &[u8],
        response: &[u8],
        expected_task: &A2aRemoteTaskIdV1,
        expected_context: &A2aRemoteContextIdV1,
        maximum_bytes: NonZeroUsize,
    ) -> Result<A2aRemoteTaskStateV1, A2aExchangeError>;
}

/// Qualify exact retained unary `CancelTask` protocol bytes without any I/O.
///
/// The implementation owns the selected official wire vocabulary and correlates
/// the original string request ID with its response. This proves protocol meaning
/// only: not HTTP/TLS provenance, route authority, cancellation, settlement,
/// durable custody, current access, or safe replay.
pub trait A2aCancelTaskEvidencePort {
    /// Decode one selected `CancelTask` request and its Task response, requiring
    /// both to name the expected remote task and the response's expected context.
    /// The request contains no metadata, tenant, or additional parameters.
    /// Whitespace and object order do not change meaning; bytes are never replaced.
    ///
    /// `maximum_bytes` bounds the sum of both inputs before allocation or parsing;
    /// implementations must also retain bounded JSON nesting and identifier sizes.
    /// A remote error or non-Task response cannot become a reported task state.
    ///
    /// # Errors
    /// Rejects excessive bytes, invalid or ambiguous protocol shape, unsupported
    /// method/input, unlike request correlation, task, or context.
    fn read_task_state_after_cancel(
        &self,
        request: &[u8],
        response: &[u8],
        expected_task: &A2aRemoteTaskIdV1,
        expected_context: &A2aRemoteContextIdV1,
        maximum_bytes: NonZeroUsize,
    ) -> Result<A2aRemoteTaskStateV1, A2aExchangeError>;
}

/// Qualify one retained `CancelTask` request without reading a response or using a transport.
///
/// This narrow recovery port owns only the selected official request shape. It
/// does not recover a task binding, prove checkpoint membership, authorize a
/// retry, or establish that the remote system received the request.
pub trait A2aCancelTaskRequestEvidencePort {
    /// Decode one exact official `CancelTask` request for the expected remote task.
    ///
    /// The request must be one JSON-RPC 2.0 request with a string ID, the
    /// selected `CancelTask` method, and only the required task parameter.
    /// Notifications, batches, responses, metadata, and extensions are rejected.
    /// `maximum_bytes` is checked before allocation or parsing.
    ///
    /// # Errors
    /// Rejects excessive bytes, malformed or ambiguous JSON, unsupported wire
    /// shape, or a task coordinate unlike `expected_task`.
    fn read_cancel_task_request(
        &self,
        request: &[u8],
        expected_task: &A2aRemoteTaskIdV1,
        maximum_bytes: NonZeroUsize,
    ) -> Result<A2aJsonRpcRequestIdV1, A2aExchangeError>;
}

/// Qualify retained unary `SendMessage` bytes without performing I/O.
///
/// The implementation owns the selected official wire vocabulary. It checks
/// that the retained request names the expected JSON-RPC identity and that the
/// retained response is its valid Task, Message, or remote-error result. This
/// proves protocol meaning only; it does not prove transport provenance,
/// durable custody, Pale success, or remote settlement.
pub trait A2aSendMessageEvidencePort {
    /// Decode one retained request and its first unary response.
    ///
    /// # Errors
    /// Rejects excessive, ambiguous, unsupported, or unlike protocol bytes.
    fn read_initial_response(
        &self,
        request: &[u8],
        response: &[u8],
        expected_request: &A2aJsonRpcRequestIdV1,
        maximum_bytes: NonZeroUsize,
    ) -> Result<A2aExchangeFacts, A2aExchangeError>;
}

/// Redacted mechanical failure; no variant claims remote effect certainty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum A2aExchangeError {
    /// Empty or inconsistent input ceilings.
    #[error("invalid A2A exchange limits")]
    InvalidLimits,
    /// Unlike input or response coordinates.
    #[error("A2A exchange coordinate mismatch")]
    CoordinateMismatch,
    /// The selected credential-free implementation cannot meet a client-auth scheme.
    #[error("selected A2A client authentication is unavailable")]
    ClientAuthenticationUnavailable,
    /// Streaming was not advertised by the admitted route.
    #[error("A2A streaming was not advertised")]
    StreamingUnavailable,
    /// A byte, event, queue or resolver ceiling was reached.
    #[error("A2A exchange limit exceeded")]
    LimitExceeded,
    /// The service is closing or unavailable.
    #[error("A2A exchange service is unavailable")]
    ServiceUnavailable,
    /// A local caller closed this exchange; the remote task may continue.
    #[error("A2A exchange was locally cancelled")]
    Cancelled,
    /// An elapsed-time ceiling ended local observation.
    #[error("A2A exchange timed out")]
    Timeout,
    /// DNS, TCP or verified TLS did not complete successfully.
    #[error("A2A verified connection failed")]
    Connection,
    /// HTTP status or headers were outside the selected response profile.
    #[error("A2A HTTP response was rejected")]
    HttpResponse,
    /// Response framing, JSON or official DTO shape was invalid.
    #[error("A2A response bytes were rejected")]
    Response,
}

/// Ordered pull access to one exchange, independent of an async framework.
pub trait A2aExchange: Send {
    /// Borrow the complete immutable request coordinate before consuming evidence.
    fn coordinate(&self) -> &A2aExchangeCoordinate;
    /// Poll the next item. End-of-stream proves only that local reads ended.
    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<A2aExchangeEvidence, A2aExchangeError>>>;
    /// Stop local reads without issuing another network request.
    fn cancel(&mut self);
}

/// Mechanical provider exchange below application-owned dispatch custody.
pub trait A2aExchangePort {
    /// Serialize one exact request without I/O, so its bytes can enter dispatch custody.
    ///
    /// # Errors
    /// Returns a closed rejection for invalid limits or excessive encoded bytes.
    fn prepare(&self, request: A2aExchangeRequest)
        -> Result<A2aPreparedExchange, A2aExchangeError>;

    /// Send already prepared bytes only after the application proves dispatch custody.
    ///
    /// # Errors
    /// Returns a closed rejection when the service has no available queue slot.
    fn start(&self, request: A2aPreparedExchange)
        -> Result<Box<dyn A2aExchange>, A2aExchangeError>;
}

/// Exact exchange context paired with the bytes this attempt will send.
#[derive(Clone, PartialEq, Eq)]
pub struct A2aExchangeCoordinate {
    context: A2aExchangeContext,
    limits: A2aExchangeLimits,
    request_sha256: ArtifactDigest,
    tls_trust_input_sha256: ArtifactDigest,
}

impl A2aExchangeCoordinate {
    /// Borrow the original request's distinct invocation identities.
    #[must_use]
    pub const fn context(&self) -> &A2aExchangeContext {
        &self.context
    }
    /// Borrow the exact encoded request-byte identity.
    #[must_use]
    pub const fn request_sha256(&self) -> &ArtifactDigest {
        &self.request_sha256
    }
    /// Borrow the TLS trust-input identity, not the whole transport configuration.
    /// This identifies supplied certificate bytes or platform-verifier selection;
    /// it does not pin the host operating system's mutable certificate store.
    #[must_use]
    pub const fn tls_trust_input_sha256(&self) -> &ArtifactDigest {
        &self.tls_trust_input_sha256
    }
    /// Return the exact finite limits used to encode and observe this exchange.
    #[must_use]
    pub const fn limits(&self) -> A2aExchangeLimits {
        self.limits
    }
}

/// Encoded request bytes ready for application-owned retention before dispatch.
///
/// This value proves only a mechanical byte identity. It neither claims that
/// those bytes are retained nor grants permission to send them.
pub struct A2aPreparedExchange {
    request: A2aExchangeRequest,
    coordinate: A2aExchangeCoordinate,
    bytes: Vec<u8>,
}

impl A2aPreparedExchange {
    /// Pair bounded adapter-encoded bytes with their original request context.
    ///
    /// This is a physical constructor. The concrete adapter owns wire-shape
    /// validation; the application owns admission and durable custody.
    ///
    /// # Errors
    /// Returns a limit error for empty or excessive request bytes.
    pub fn from_encoded(
        request: A2aExchangeRequest,
        bytes: Vec<u8>,
        tls_trust_input_sha256: ArtifactDigest,
    ) -> Result<Self, A2aExchangeError> {
        if bytes.is_empty() || bytes.len() > request.limits().request_bytes.get() {
            return Err(A2aExchangeError::LimitExceeded);
        }
        let coordinate = A2aExchangeCoordinate {
            context: request.coordinate.clone(),
            limits: request.limits(),
            request_sha256: ArtifactDigest::hash_bytes(&bytes),
            tls_trust_input_sha256,
        };
        Ok(Self {
            request,
            coordinate,
            bytes,
        })
    }
    /// Borrow the bytes that the application must retain before starting.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Borrow the exact input and limits.
    #[must_use]
    pub const fn request(&self) -> &A2aExchangeRequest {
        &self.request
    }
    /// Borrow the context plus exact encoded-byte identity.
    #[must_use]
    pub const fn coordinate(&self) -> &A2aExchangeCoordinate {
        &self.coordinate
    }
    /// Transfer the same bytes to the concrete carrier without another encoding.
    #[must_use]
    pub fn into_parts(self) -> (A2aExchangeRequest, A2aExchangeCoordinate, Vec<u8>) {
        (self.request, self.coordinate, self.bytes)
    }
}
