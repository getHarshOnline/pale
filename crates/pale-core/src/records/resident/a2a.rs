//! A2A route admission, remote-task binding, and owner-state observation.
//!
//! These contracts bind Pale operations to exact A2A evidence without
//! copying Agent Card, Message, Task, JSON-RPC, or SSE wire records. A remote
//! task identity remains an owner correlation and never becomes a Pale
//! operation, result, effect, acceptance, or settlement coordinate. Every
//! record binds the exact [`DeploymentRuntimeAuthorityV1`] that issued it.

use std::cell::Cell;

use serde::{Deserialize, Serialize};
use url::Url;

use super::*;
use crate::records::*;

/// Public schema root for admitted A2A routes.
pub const A2A_ROUTE_ADMISSION_V1_SCHEMA_ID: &str = "a2a-route-admission.v1";
/// Wire schema carried by every A2A route admission.
pub const A2A_ROUTE_ADMISSION_V1_SCHEMA: &str = "pale/a2a-route-admission.v1";
/// Signature domain for [`A2aRouteAdmissionV1`].
pub const A2A_ROUTE_ADMISSION_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_A2A_ROUTE_ADMISSION_V1\0";

/// Public schema root for immutable A2A task bindings.
pub const A2A_TASK_BINDING_V1_SCHEMA_ID: &str = "a2a-task-binding.v1";
/// Wire schema carried by every A2A task binding.
pub const A2A_TASK_BINDING_V1_SCHEMA: &str = "pale/a2a-task-binding.v1";
/// Signature domain for [`A2aTaskBindingV1`].
pub const A2A_TASK_BINDING_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_A2A_TASK_BINDING_V1\0";

/// Public schema root for A2A task-observation revisions.
pub const A2A_TASK_OBSERVATION_V1_SCHEMA_ID: &str = "a2a-task-observation.v1";
/// Wire schema carried by every A2A task observation.
pub const A2A_TASK_OBSERVATION_V1_SCHEMA: &str = "pale/a2a-task-observation.v1";
/// Signature domain for [`A2aTaskObservationV1`].
pub const A2A_TASK_OBSERVATION_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_A2A_TASK_OBSERVATION_V1\0";

/// A bounded canonical A2A endpoint whose address uses HTTPS.
///
/// This value establishes only syntax. [`A2aRouteAdmissionV1`] separately
/// binds the evidence that TLS and the remote route were actually verified.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct A2aHttpsEndpointV1(
    #[schemars(length(min = 9, max = 512), regex(pattern = r"^https://[^\s]+$"))]
    String,
);

impl A2aHttpsEndpointV1 {
    /// Validate and canonicalize one bounded HTTPS endpoint.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1::InvalidIdentity`] unless `value`
    /// parses without a fatal or recoverable syntax violation, uses HTTPS with
    /// a host, carries no user information or fragment, is NFC, and fits the
    /// resident identity bound before and after canonicalization.
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidResidentRecordV1> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_RESIDENT_ID_BYTES
            || value
                .chars()
                .any(|character| character.is_whitespace() || character.is_control())
            || !unicode_normalization::is_nfc(&value)
        {
            return Err(InvalidResidentRecordV1::InvalidIdentity {
                kind: "A2A HTTPS endpoint",
            });
        }

        let syntax_violation = Cell::new(false);
        let mark_syntax_violation = |_: url::SyntaxViolation| syntax_violation.set(true);
        let endpoint = Url::options()
            .syntax_violation_callback(Some(&mark_syntax_violation))
            .parse(&value)
            .map_err(|_| InvalidResidentRecordV1::InvalidIdentity {
                kind: "A2A HTTPS endpoint",
            })?;

        let authority_contains_userinfo = value
            .split_once(':')
            .and_then(|(_, remainder)| remainder.strip_prefix("//"))
            .map(|remainder| {
                remainder
                    .split_once(['/', '?', '#'])
                    .map_or(remainder, |(authority, _)| authority)
                    .contains('@')
            })
            .unwrap_or(false);

        if syntax_violation.get()
            || endpoint.scheme() != "https"
            || endpoint.host_str().is_none_or(str::is_empty)
            || authority_contains_userinfo
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(InvalidResidentRecordV1::InvalidIdentity {
                kind: "A2A HTTPS endpoint",
            });
        }

        let canonical = endpoint.as_str().to_owned();
        if canonical.len() > MAX_RESIDENT_ID_BYTES {
            return Err(InvalidResidentRecordV1::InvalidIdentity {
                kind: "A2A HTTPS endpoint",
            });
        }

        Ok(Self(canonical))
    }

    /// Borrow the canonical endpoint spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for A2aHttpsEndpointV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Fixed A2A protocol version selected for initial v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum A2aProtocolVersionV1 {
    /// A2A protocol version 1.0, sent as `A2A-Version: 1.0`.
    #[serde(rename = "1.0")]
    V1_0,
}

/// Fixed request binding selected for initial-v1 A2A delegation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum A2aRequestBindingV1 {
    /// JSON-RPC 2.0 over verified HTTPS.
    JsonRpc2OverVerifiedHttps,
}

/// Agent-Card evidence for the selected route's streaming posture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum A2aStreamingAdmissionV1 {
    /// The admitted Agent Card does not advertise streaming.
    NotAdvertised,
    /// The admitted Agent Card advertises streaming under exact evidence.
    Advertised {
        /// Immutable evidence for the advertised streaming capability.
        evidence: ImmutableRecordReferenceV1,
    },
}

impl A2aStreamingAdmissionV1 {
    /// Whether bounded SSE is admitted for this exact route generation.
    #[must_use]
    pub const fn is_advertised(&self) -> bool {
        matches!(self, Self::Advertised { .. })
    }
}

/// A2A request mode whose response first identified a remote Task.
///
/// This remains distinct from [`A2aTaskObservationSourceV1`]: the request mode
/// is immutable binding provenance, while observation sources describe each
/// subsequently admitted item of owner evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum A2aTaskRequestModeV1 {
    /// Unary `SendMessage` whose response contained the Task.
    SendMessage,
    /// `SendStreamingMessage` whose first admitted stream event contained the Task.
    SendStreamingMessage,
}

/// Exact Agent-Card security schemes selected for one admitted route.
///
/// The identifiers are a sorted, unique subset defined by the digest-bound
/// Agent Card. An empty selection means the configured endpoint requires no
/// client authentication; it does not waive HTTPS peer verification, capability
/// authorization, or disclosure limits. The evidence qualifies that selection
/// and the HTTPS peer. This record stores no credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct A2aSecurityAdmissionV1 {
    /// Sorted unique identifiers selected from the exact Agent Card.
    pub selected_scheme_ids: Vec<A2aSecuritySchemeIdV1>,
    /// Immutable evidence for selected schemes and verified HTTPS.
    pub evidence: ImmutableRecordReferenceV1,
}

/// Closed owner-native A2A task state projected from admitted evidence.
///
/// This enum mirrors only the stable A2A-v1 state vocabulary needed for
/// observation. It does not embed the foreign Task wire record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum A2aRemoteTaskStateV1 {
    /// The owner supplied no more specific state.
    Unspecified,
    /// The task was accepted but work has not begun.
    Submitted,
    /// The remote owner reports active work.
    Working,
    /// The remote owner requires additional user input.
    InputRequired,
    /// The remote owner requires authentication.
    AuthRequired,
    /// The remote owner reports completion.
    Completed,
    /// The remote owner reports cancellation.
    Canceled,
    /// The remote owner reports failure.
    Failed,
    /// The remote owner rejected the task.
    Rejected,
}

impl A2aRemoteTaskStateV1 {
    /// Whether the remote owner classifies this task state as terminal.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Canceled | Self::Failed | Self::Rejected
        )
    }
}

/// Exact A2A operation whose evidence produced one task observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum A2aTaskObservationSourceV1 {
    /// Task state observed in the initial `SendMessage` response.
    InitialSendMessageResponse {
        /// Immutable exact response evidence containing the identified Task.
        evidence: ImmutableRecordReferenceV1,
    },
    /// Task state and identity observed in the first admitted streaming event.
    InitialStreamingEvent {
        /// Optional owner-issued SSE event identity.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event_id: Option<A2aSseEventIdV1>,
        /// Immutable exact SSE-event evidence containing the identified Task.
        evidence: ImmutableRecordReferenceV1,
    },
    /// Later Task state observed through the same admitted bounded SSE stream.
    StreamingEvent {
        /// Optional owner-issued SSE event identity.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event_id: Option<A2aSseEventIdV1>,
        /// Immutable exact SSE-event evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// Task state read back through `GetTask`.
    GetTask {
        /// Immutable exact read-back request evidence.
        request: ImmutableRecordReferenceV1,
        /// Immutable exact read-back response evidence.
        response: ImmutableRecordReferenceV1,
    },
    /// Task state read back after a `CancelTask` attempt.
    CancelTaskReadBack {
        /// Immutable exact cancellation-request evidence.
        request: ImmutableRecordReferenceV1,
        /// Immutable exact cancellation read-back evidence.
        response: ImmutableRecordReferenceV1,
    },
}

resident_signed_record!(
    /// Admission of one exact A2A-v1 client route and Agent Card.
    ///
    /// This record binds route use to immutable Agent-Card and transport-
    /// security evidence. It stores no credential. The carried freshness and
    /// expiry intervals limit new dispatch, not custody of an earlier valid
    /// send. Exact dispatch evidence may identify a Task later, and admitted
    /// read-back may continue while its [`OperationHandleV1`] remains retained.
    /// Its runtime-authority reference and digest identify the admitted issuer.
    A2aRouteAdmissionV1, A2aRouteAdmissionWireV1, A2aRouteAdmissionPreimageV1,
    canonical_a2a_route_admission_sha256: A2aRouteAdmissionDigest,
    A2A_ROUTE_ADMISSION_V1_SCHEMA, "A2A route admission",
    {
        /// Stable route-admission identity.
        route_admission_id: A2aRouteAdmissionIdV1,
        /// Exact selected provider binding.
        provider_binding: ProviderBindingRefV1,
        /// Exact admitted common-request route coordinate.
        route: CommonRequestRouteV1,
        /// Exact remote A2A endpoint.
        endpoint: A2aHttpsEndpointV1,
        /// Fixed A2A protocol version.
        protocol_version: A2aProtocolVersionV1,
        /// Fixed request binding.
        request_binding: A2aRequestBindingV1,
        /// Immutable digest-bound Agent Card evidence.
        agent_card: ImmutableRecordReferenceV1,
        /// Exact streaming posture advertised by that Agent Card.
        streaming: A2aStreamingAdmissionV1,
        /// Verified-HTTPS and advertised security-scheme admission.
        security: A2aSecurityAdmissionV1,
        /// Unix time at which the Agent Card and route were observed.
        observed_at: UnixTimeV1,
        /// Last time at which the observation is considered fresh.
        fresh_until: UnixTimeV1,
        /// Hard expiry of this route admission.
        expires_at: UnixTimeV1,
        /// Exact deployment-runtime authority under which this record was issued.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Pale principal whose admitted runtime issued the admission.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Exact issuer key.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_a2a_route_shape
);

resident_signed_record!(
    /// Immutable correlation between one Pale operation and one observed A2A Task.
    ///
    /// Admission requires owner evidence for a real Task. A task-less Message
    /// or a lost response before task identification cannot construct this type.
    /// Its runtime-authority reference and digest identify the Pale issuer;
    /// the foreign Task remains independently owner-authenticated evidence.
    A2aTaskBindingV1, A2aTaskBindingWireV1, A2aTaskBindingPreimageV1,
    canonical_a2a_task_binding_sha256: A2aTaskBindingDigest,
    A2A_TASK_BINDING_V1_SCHEMA, "A2A task binding",
    {
        /// Stable task-binding identity.
        task_binding_id: A2aTaskBindingIdV1,
        /// Stable Pale operation handle identity.
        operation_handle_id: OperationHandleIdV1,
        /// Immutable operation-handle reference.
        operation_handle_ref: ResidentRecordRefV1,
        /// Canonical identity of the operation handle.
        operation_handle_sha256: OperationHandleDigest,
        /// Immutable operation observation authorizing the outbound attempt.
        operation_observation_ref: ResidentRecordRefV1,
        /// Canonical identity of that operation observation.
        operation_observation_sha256: OperationObservationDigest,
        /// Stable admitted A2A route identity.
        route_admission_id: A2aRouteAdmissionIdV1,
        /// Immutable A2A route-admission reference.
        route_admission_ref: ResidentRecordRefV1,
        /// Canonical identity of the exact route admission.
        route_admission_sha256: A2aRouteAdmissionDigest,
        /// Pale transport-attempt identity.
        attempt_id: CapabilityAttemptIdV1,
        /// Exact JSON-RPC request identity sent in this attempt.
        json_rpc_request_id: A2aJsonRpcRequestIdV1,
        /// Exact request mode that first identified the remote Task.
        request_mode: A2aTaskRequestModeV1,
        /// Actual remote-owner Task identity observed in the response.
        remote_task_id: A2aRemoteTaskIdV1,
        /// Required remote-owner context identity carried by the observed Task.
        remote_context_id: A2aRemoteContextIdV1,
        /// Immutable exact originating request evidence.
        request_evidence: ImmutableRecordReferenceV1,
        /// Immutable exact response or stream-event evidence containing the Task.
        initial_task_evidence: ImmutableRecordReferenceV1,
        /// Unix time at which the real Task identity became bound.
        bound_at: UnixTimeV1,
        /// Exact deployment-runtime authority under which this record was issued.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Pale principal whose admitted runtime issued the binding.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Exact issuer key.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    |_| Ok::<(), InvalidResidentRecordV1>(())
);

resident_signed_record!(
    /// One revision in the monotone owner-state history of a bound A2A Task.
    ///
    /// Revision one is the initial response. Each later revision names exactly
    /// its immediate predecessor. A terminal remote state may be observed
    /// again, but it cannot transition to a different state. Its runtime-
    /// authority reference and digest identify the admitted Pale issuer.
    A2aTaskObservationV1, A2aTaskObservationWireV1, A2aTaskObservationPreimageV1,
    canonical_a2a_task_observation_sha256: A2aTaskObservationDigest,
    A2A_TASK_OBSERVATION_V1_SCHEMA, "A2A task observation",
    {
        /// Stable binding whose remote Task is observed.
        task_binding_id: A2aTaskBindingIdV1,
        /// Immutable A2A task-binding reference.
        task_binding_ref: ResidentRecordRefV1,
        /// Canonical identity of the exact task binding.
        task_binding_sha256: A2aTaskBindingDigest,
        /// Positive revision in this binding-local observation lineage.
        revision: A2aTaskObservationRevisionV1,
        /// Previous observation reference; absent only at revision one.
        predecessor_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::predecessor_ref`].
        predecessor_sha256: Option<A2aTaskObservationDigest>,
        /// A2A operation that produced the exact owner evidence.
        source: A2aTaskObservationSourceV1,
        /// Exact remote-owner state projected from that evidence.
        remote_state: A2aRemoteTaskStateV1,
        /// Existing reconciliation frontier when this observation remains uncertain.
        reconciliation: Option<ReconciliationFrontierReferenceV1>,
        /// Unix time at which the remote state was observed.
        observed_at: UnixTimeV1,
        /// Exact deployment-runtime authority under which this record was issued.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Pale principal whose admitted runtime issued the observation.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Exact issuer key.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_a2a_task_observation_shape,
    wire_transform = a2a_task_observation_schema_v1
);

impl A2aTaskBindingWireV1 {
    /// Borrow the claimed operation identity for discovery association, not admission.
    #[must_use]
    pub const fn operation_handle_id(&self) -> &OperationHandleIdV1 {
        &self.operation_handle_id
    }

    /// Borrow the claimed immutable operation reference; this proves no custody.
    #[must_use]
    pub const fn operation_handle_ref(&self) -> &ResidentRecordRefV1 {
        &self.operation_handle_ref
    }

    /// Borrow the claimed operation digest; protected recovery admits its meaning.
    #[must_use]
    pub const fn operation_handle_sha256(&self) -> &OperationHandleDigest {
        &self.operation_handle_sha256
    }

    /// Borrow the claimed binding identity for lookup, not admission.
    #[must_use]
    pub const fn task_binding_id(&self) -> &A2aTaskBindingIdV1 {
        &self.task_binding_id
    }

    /// Borrow the untrusted route reference for historical lookup.
    #[must_use]
    pub const fn route_admission_ref(&self) -> &ResidentRecordRefV1 {
        &self.route_admission_ref
    }

    /// Borrow the claimed route digest; this proves no route authority.
    #[must_use]
    pub const fn route_admission_sha256(&self) -> &A2aRouteAdmissionDigest {
        &self.route_admission_sha256
    }

    /// Borrow the claimed local observation reference for historical lookup.
    #[must_use]
    pub const fn operation_observation_ref(&self) -> &ResidentRecordRefV1 {
        &self.operation_observation_ref
    }

    /// Borrow the claimed local observation digest; this proves no dispatch.
    #[must_use]
    pub const fn operation_observation_sha256(&self) -> &OperationObservationDigest {
        &self.operation_observation_sha256
    }

    /// Borrow the claimed request evidence coordinate for bounded lookup.
    #[must_use]
    pub const fn request_evidence(&self) -> &ImmutableRecordReferenceV1 {
        &self.request_evidence
    }

    /// Borrow the claimed first Task evidence coordinate, not an admitted Task.
    #[must_use]
    pub const fn initial_task_evidence(&self) -> &ImmutableRecordReferenceV1 {
        &self.initial_task_evidence
    }
}

impl A2aTaskObservationWireV1 {
    /// Borrow the claimed task-binding reference for bounded recovery lookup.
    #[must_use]
    pub const fn task_binding_ref(&self) -> &ResidentRecordRefV1 {
        &self.task_binding_ref
    }

    /// Borrow the claimed task-binding digest; this proves no association.
    #[must_use]
    pub const fn task_binding_sha256(&self) -> &A2aTaskBindingDigest {
        &self.task_binding_sha256
    }

    /// Borrow the claimed revision for bounded history traversal.
    #[must_use]
    pub const fn revision(&self) -> &A2aTaskObservationRevisionV1 {
        &self.revision
    }

    /// Borrow the untrusted predecessor reference for historical lookup.
    #[must_use]
    pub const fn predecessor_ref(&self) -> &Option<ResidentRecordRefV1> {
        &self.predecessor_ref
    }

    /// Borrow the claimed predecessor digest; this proves no lineage.
    #[must_use]
    pub const fn predecessor_sha256(&self) -> &Option<A2aTaskObservationDigest> {
        &self.predecessor_sha256
    }

    /// Borrow the claimed source to select evidence, not to admit its meaning.
    #[must_use]
    pub const fn source(&self) -> &A2aTaskObservationSourceV1 {
        &self.source
    }
}

/// Resolved route and signer required for [`A2aRouteAdmissionV1`].
#[derive(Clone, Copy)]
pub struct A2aRouteAdmissionContextV1<'a> {
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
    /// Exact provider binding selected for this route.
    pub provider_binding: &'a ProviderBindingRefV1,
    /// Exact common-request route selected for this admission.
    pub route: &'a CommonRequestRouteV1,
}

/// Resolved operation and route parents required for [`A2aTaskBindingV1`].
#[derive(Clone, Copy)]
pub struct A2aTaskBindingAdmissionContextV1<'a> {
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
    /// Reference resolving [`Self::operation_handle`].
    pub operation_handle_ref: &'a ResidentRecordRefV1,
    /// Exact admitted durable operation handle.
    pub operation_handle: &'a OperationHandleV1,
    /// Reference resolving [`Self::operation_observation`].
    pub operation_observation_ref: &'a ResidentRecordRefV1,
    /// Exact historical dispatched-attempt observation for the authorized send.
    /// Later task identification does not renew its execution permission.
    pub operation_observation: &'a OperationObservationV1,
    /// Reference resolving [`Self::route_admission`].
    pub route_admission_ref: &'a ResidentRecordRefV1,
    /// Exact admitted A2A route.
    pub route_admission: &'a A2aRouteAdmissionV1,
}

/// Resolved binding, route, handle, and predecessor for a task observation.
#[derive(Clone, Copy)]
pub struct A2aTaskObservationAdmissionContextV1<'a> {
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
    /// Reference resolving [`Self::task_binding`].
    pub task_binding_ref: &'a ResidentRecordRefV1,
    /// Exact admitted A2A task binding.
    pub task_binding: &'a A2aTaskBindingV1,
    /// Reference resolving [`Self::route_admission`].
    pub route_admission_ref: &'a ResidentRecordRefV1,
    /// Exact admitted route governing observation.
    pub route_admission: &'a A2aRouteAdmissionV1,
    /// Reference resolving [`Self::operation_handle`].
    pub operation_handle_ref: &'a ResidentRecordRefV1,
    /// Exact durable operation handle governing retention.
    pub operation_handle: &'a OperationHandleV1,
    /// Current admitted Pale observation for the bound operation.
    pub operation_observation: &'a OperationObservationV1,
    /// Exact predecessor reference and record; absent only at revision one.
    pub predecessor: Option<(&'a ResidentRecordRefV1, &'a A2aTaskObservationV1)>,
}

impl A2aRouteAdmissionV1 {
    /// Issue a route admission from one owned preimage after contextual admission.
    ///
    /// The signer is invoked only after structural, provider, route, Agent-
    /// Card, security, and runtime-authority validation succeeds.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] on admission or signing failure.
    pub fn issue_contextual<Evidence, Sign>(
        preimage: A2aRouteAdmissionPreimageV1,
        context: &A2aRouteAdmissionContextV1<'_>,
        admit_evidence: Evidence,
        sign: Sign,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Evidence: FnOnce(&A2aRouteAdmissionPreimageV1) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &A2aRouteAdmissionPreimageV1,
            &A2aRouteAdmissionDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        validate_a2a_route_preimage(&preimage)?;
        validate_a2a_route_context(&preimage, context, admit_evidence)?;
        let digest = A2aRouteAdmissionDigest::hash_canonical(&preimage)?;
        let signature = sign(A2A_ROUTE_ADMISSION_V1_SIGNATURE_DOMAIN, &preimage, &digest)?;
        Self::seal(preimage, signature)
    }

    /// Admit a route after resolving its exact Agent Card and security evidence.
    ///
    /// `admit_evidence` owns Agent-Card parsing, digest comparison, endpoint,
    /// streaming, and security qualification under the official A2A wire owner.
    /// It must prove that selected schemes belong to the exact digest-bound
    /// Agent Card, or that the configured endpoint requires no client
    /// authentication. Both cases still require verified HTTPS peer evidence
    /// and the applicable capability and disclosure checks.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for structural, digest, provider,
    /// route, evidence, authority-coordinate, freshness, or signature failure.
    pub fn admit_contextual_wire<Evidence, Signature>(
        wire: A2aRouteAdmissionWireV1,
        context: &A2aRouteAdmissionContextV1<'_>,
        admit_evidence: Evidence,
        verify_signature: Signature,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Evidence: FnOnce(&A2aRouteAdmissionPreimageV1) -> Result<(), InvalidResidentRecordV1>,
        Signature: FnOnce(
            &[u8],
            &A2aRouteAdmissionPreimageV1,
            &A2aRouteAdmissionDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_a2a_route_context(preimage, context, admit_evidence)?;
            verify_signature(
                A2A_ROUTE_ADMISSION_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

impl A2aTaskBindingV1 {
    /// Issue a task binding from one owned preimage after contextual admission.
    ///
    /// The signer is invoked only after structural, operation, route, actual-
    /// Task, chronology, and runtime-authority validation succeeds.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] on admission or signing failure.
    pub fn issue_contextual<Task, Sign>(
        preimage: A2aTaskBindingPreimageV1,
        context: &A2aTaskBindingAdmissionContextV1<'_>,
        admit_remote_task: Task,
        sign: Sign,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Task: FnOnce(&A2aTaskBindingPreimageV1) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &A2aTaskBindingPreimageV1,
            &A2aTaskBindingDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        validate_a2a_task_binding_preimage(&preimage)?;
        validate_a2a_task_binding_context(&preimage, context, admit_remote_task)?;
        let digest = A2aTaskBindingDigest::hash_canonical(&preimage)?;
        let signature = sign(A2A_TASK_BINDING_V1_SIGNATURE_DOMAIN, &preimage, &digest)?;
        Self::seal(preimage, signature)
    }

    /// Admit a binding only after owner evidence identifies an actual remote Task.
    ///
    /// `admit_remote_task` must reject a task-less Message and must authenticate
    /// the task/context/request coordinates against the exact initial response.
    /// A lost response before task identification therefore has no constructible
    /// binding and remains at the existing diagnostic/reconciliation owners.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for structural, digest, operation,
    /// route, chronology, task-evidence, authority-coordinate, or signature failure.
    pub fn admit_contextual_wire<Task, Signature>(
        wire: A2aTaskBindingWireV1,
        context: &A2aTaskBindingAdmissionContextV1<'_>,
        admit_remote_task: Task,
        verify_signature: Signature,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Task: FnOnce(&A2aTaskBindingPreimageV1) -> Result<(), InvalidResidentRecordV1>,
        Signature: FnOnce(
            &[u8],
            &A2aTaskBindingPreimageV1,
            &A2aTaskBindingDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_a2a_task_binding_context(preimage, context, admit_remote_task)?;
            verify_signature(
                A2A_TASK_BINDING_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

impl A2aTaskObservationV1 {
    /// Issue a task observation from one owned preimage after contextual admission.
    ///
    /// The signer is invoked only after structural, binding, predecessor,
    /// source-evidence, chronology, and runtime-authority validation succeeds.
    /// A changed authority coordinate additionally requires a resolved rotation
    /// in the same deployment/control-store lineage.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] on admission or signing failure.
    pub fn issue_contextual<Evidence, Authority, Sign>(
        preimage: A2aTaskObservationPreimageV1,
        context: &A2aTaskObservationAdmissionContextV1<'_>,
        admit_evidence: Evidence,
        admit_authority_rotation: Authority,
        sign: Sign,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Evidence: FnOnce(&A2aTaskObservationPreimageV1) -> Result<(), InvalidResidentRecordV1>,
        Authority: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &A2aTaskObservationPreimageV1,
            &A2aTaskObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        validate_a2a_task_observation_preimage(&preimage)?;
        validate_a2a_task_observation_context(
            &preimage,
            context,
            admit_evidence,
            admit_authority_rotation,
        )?;
        let digest = A2aTaskObservationDigest::hash_canonical(&preimage)?;
        let signature = sign(A2A_TASK_OBSERVATION_V1_SIGNATURE_DOMAIN, &preimage, &digest)?;
        Self::seal(preimage, signature)
    }

    /// Admit one exact successor observation for a resolved remote Task binding.
    ///
    /// `admit_evidence` resolves the exact initial-response, SSE, `GetTask`, or
    /// `CancelTask` read-back evidence. It must not convert remote terminal state
    /// into Pale completion, effect, acceptance, or settlement. A changed
    /// runtime-authority coordinate is accepted only when
    /// `admit_authority_rotation` proves its admitted lineage from the exact
    /// predecessor authority.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for structural, digest, binding,
    /// route, predecessor, state, chronology, evidence, signer, or signature failure.
    pub fn admit_contextual_wire<Evidence, Authority, Signature>(
        wire: A2aTaskObservationWireV1,
        context: &A2aTaskObservationAdmissionContextV1<'_>,
        admit_evidence: Evidence,
        admit_authority_rotation: Authority,
        verify_signature: Signature,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Evidence: FnOnce(&A2aTaskObservationPreimageV1) -> Result<(), InvalidResidentRecordV1>,
        Authority: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Signature: FnOnce(
            &[u8],
            &A2aTaskObservationPreimageV1,
            &A2aTaskObservationDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_a2a_task_observation_context(
                preimage,
                context,
                admit_evidence,
                admit_authority_rotation,
            )?;
            verify_signature(
                A2A_TASK_OBSERVATION_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

fn validate_a2a_route_preimage(
    preimage: &A2aRouteAdmissionPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if preimage.schema_id.as_str() != A2A_ROUTE_ADMISSION_V1_SCHEMA {
        return Err(InvalidResidentRecordV1::SchemaCoordinate {
            family: "A2A route admission",
        });
    }
    validate_a2a_route_shape(preimage)
}

fn validate_a2a_task_binding_preimage(
    preimage: &A2aTaskBindingPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if preimage.schema_id.as_str() == A2A_TASK_BINDING_V1_SCHEMA {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::SchemaCoordinate {
            family: "A2A task binding",
        })
    }
}

fn validate_a2a_task_observation_preimage(
    preimage: &A2aTaskObservationPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if preimage.schema_id.as_str() != A2A_TASK_OBSERVATION_V1_SCHEMA {
        return Err(InvalidResidentRecordV1::SchemaCoordinate {
            family: "A2A task observation",
        });
    }
    validate_a2a_task_observation_shape(preimage)
}

fn validate_a2a_route_shape(
    value: &A2aRouteAdmissionPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if value.observed_at > value.fresh_until || value.fresh_until > value.expires_at {
        return Err(contextual(
            "A2A route admission",
            "ordered freshness and expiry",
        ));
    }
    strictly_sorted(
        "A2A route selected security schemes",
        &value.security.selected_scheme_ids,
    )
}

fn validate_a2a_route_context<Evidence>(
    preimage: &A2aRouteAdmissionPreimageV1,
    context: &A2aRouteAdmissionContextV1<'_>,
    admit_evidence: Evidence,
) -> Result<(), InvalidResidentRecordV1>
where
    Evidence: FnOnce(&A2aRouteAdmissionPreimageV1) -> Result<(), InvalidResidentRecordV1>,
{
    validate_signed_coordinates(
        "A2A route admission",
        &preimage.schema_sha256,
        context.signed.expected_schema_sha256,
        &preimage.issuer_principal_id,
        &preimage.issuer_key_id,
        preimage.issuer_key_epoch,
        context.signed.signing_authority,
    )?;
    validate_integrated_runtime_authority_coordinates(
        "A2A route admission",
        &preimage.deployment_runtime_authority_ref,
        &preimage.deployment_runtime_authority_sha256,
        context.signed.signing_authority,
    )?;
    if preimage.provider_binding != *context.provider_binding || preimage.route != *context.route {
        return Err(contextual(
            "A2A route admission",
            "exact provider and route",
        ));
    }
    admit_evidence(preimage)
}

fn validate_a2a_task_binding_context<Task>(
    preimage: &A2aTaskBindingPreimageV1,
    context: &A2aTaskBindingAdmissionContextV1<'_>,
    admit_remote_task: Task,
) -> Result<(), InvalidResidentRecordV1>
where
    Task: FnOnce(&A2aTaskBindingPreimageV1) -> Result<(), InvalidResidentRecordV1>,
{
    validate_signed_coordinates(
        "A2A task binding",
        &preimage.schema_sha256,
        context.signed.expected_schema_sha256,
        &preimage.issuer_principal_id,
        &preimage.issuer_key_id,
        preimage.issuer_key_epoch,
        context.signed.signing_authority,
    )?;
    validate_integrated_runtime_authority_coordinates(
        "A2A task binding",
        &preimage.deployment_runtime_authority_ref,
        &preimage.deployment_runtime_authority_sha256,
        context.signed.signing_authority,
    )?;
    if preimage.operation_handle_id != *context.operation_handle.operation_handle_id()
        || preimage.operation_handle_ref != *context.operation_handle_ref
        || preimage.operation_handle_sha256
            != *context.operation_handle.canonical_operation_handle_sha256()
        || preimage.operation_observation_ref != *context.operation_observation_ref
        || preimage.operation_observation_sha256
            != *context
                .operation_observation
                .canonical_operation_observation_sha256()
        || context.operation_observation.operation_handle_id()
            != context.operation_handle.operation_handle_id()
        || context
            .operation_observation
            .state()
            .dispatched_attempt_id()
            != Some(&preimage.attempt_id)
        || preimage.route_admission_id != *context.route_admission.route_admission_id()
        || preimage.route_admission_ref != *context.route_admission_ref
        || preimage.route_admission_sha256
            != *context
                .route_admission
                .canonical_a2a_route_admission_sha256()
        || !context
            .route_admission
            .route()
            .matches_request_binding(context.operation_handle.request_binding())
        || context.route_admission.observed_at() > context.operation_observation.observed_at()
        || context.operation_observation.observed_at() > context.route_admission.fresh_until()
        || context.operation_observation.observed_at() < context.operation_handle.created_at()
        || context.operation_observation.observed_at() > context.operation_handle.expires_at()
        || matches!(
            preimage.request_mode,
            A2aTaskRequestModeV1::SendStreamingMessage
        ) && !context.route_admission.streaming().is_advertised()
        || preimage.bound_at < *context.operation_observation.observed_at()
        || preimage.bound_at > *context.operation_handle.retain_until()
    {
        return Err(contextual(
            "A2A task binding",
            "exact authorized dispatch and retained operation",
        ));
    }
    admit_remote_task(preimage)
}

fn validate_a2a_task_observation_context<Evidence, Authority>(
    preimage: &A2aTaskObservationPreimageV1,
    context: &A2aTaskObservationAdmissionContextV1<'_>,
    admit_evidence: Evidence,
    admit_authority_rotation: Authority,
) -> Result<(), InvalidResidentRecordV1>
where
    Evidence: FnOnce(&A2aTaskObservationPreimageV1) -> Result<(), InvalidResidentRecordV1>,
    Authority: FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    validate_signed_coordinates(
        "A2A task observation",
        &preimage.schema_sha256,
        context.signed.expected_schema_sha256,
        &preimage.issuer_principal_id,
        &preimage.issuer_key_id,
        preimage.issuer_key_epoch,
        context.signed.signing_authority,
    )?;
    validate_integrated_runtime_authority_coordinates(
        "A2A task observation",
        &preimage.deployment_runtime_authority_ref,
        &preimage.deployment_runtime_authority_sha256,
        context.signed.signing_authority,
    )?;
    if preimage.task_binding_id != *context.task_binding.task_binding_id()
        || preimage.task_binding_ref != *context.task_binding_ref
        || preimage.task_binding_sha256 != *context.task_binding.canonical_a2a_task_binding_sha256()
        || context.task_binding.route_admission_id() != context.route_admission.route_admission_id()
        || context.task_binding.route_admission_ref() != context.route_admission_ref
        || context.task_binding.route_admission_sha256()
            != context
                .route_admission
                .canonical_a2a_route_admission_sha256()
        || context.task_binding.operation_handle_id()
            != context.operation_handle.operation_handle_id()
        || context.task_binding.operation_handle_ref() != context.operation_handle_ref
        || context.task_binding.operation_handle_sha256()
            != context.operation_handle.canonical_operation_handle_sha256()
        || context.operation_observation.operation_handle_id()
            != context.operation_handle.operation_handle_id()
        || preimage.observed_at < *context.task_binding.bound_at()
        || preimage.observed_at > *context.operation_handle.retain_until()
        || matches!(
            preimage.source,
            A2aTaskObservationSourceV1::InitialStreamingEvent { .. }
                | A2aTaskObservationSourceV1::StreamingEvent { .. }
        ) && (!context.route_admission.streaming().is_advertised()
            || context.task_binding.request_mode() != &A2aTaskRequestModeV1::SendStreamingMessage)
    {
        return Err(contextual(
            "A2A task observation",
            "exact retained binding and admitted observation route",
        ));
    }
    if preimage.revision.get() == 1
        && (!initial_task_source_matches_binding(&preimage.source, context.task_binding)
            || preimage.observed_at != *context.task_binding.bound_at())
    {
        return Err(contextual(
            "A2A task observation",
            "exact initial Task response and binding time",
        ));
    }
    admit_a2a_observation_source(preimage, context.operation_observation)?;
    admit_a2a_task_observation_predecessor(
        preimage,
        context.predecessor,
        admit_authority_rotation,
    )?;
    admit_evidence(preimage)
}

fn validate_a2a_task_observation_shape(
    value: &A2aTaskObservationPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    let has_predecessor = value.predecessor_ref.is_some() && value.predecessor_sha256.is_some();
    let initial_source = matches!(
        value.source,
        A2aTaskObservationSourceV1::InitialSendMessageResponse { .. }
            | A2aTaskObservationSourceV1::InitialStreamingEvent { .. }
    );
    if value.predecessor_ref.is_some() != value.predecessor_sha256.is_some()
        || (value.revision.get() == 1) == has_predecessor
        || (value.revision.get() == 1) != initial_source
    {
        return Err(contextual(
            "A2A task observation",
            "revision-one initial-response boundary",
        ));
    }
    Ok(())
}

fn initial_task_source_matches_binding(
    source: &A2aTaskObservationSourceV1,
    binding: &A2aTaskBindingV1,
) -> bool {
    match (*binding.request_mode(), source) {
        (
            A2aTaskRequestModeV1::SendMessage,
            A2aTaskObservationSourceV1::InitialSendMessageResponse { evidence },
        )
        | (
            A2aTaskRequestModeV1::SendStreamingMessage,
            A2aTaskObservationSourceV1::InitialStreamingEvent { evidence, .. },
        ) => evidence == binding.initial_task_evidence(),
        _ => false,
    }
}

fn admit_a2a_observation_source(
    value: &A2aTaskObservationPreimageV1,
    operation_observation: &OperationObservationV1,
) -> Result<(), InvalidResidentRecordV1> {
    if let A2aTaskObservationSourceV1::CancelTaskReadBack { request, .. } = &value.source {
        match operation_observation.state() {
            OperationObservationStateV1::CancellationRequested {
                request: admitted_request,
            } if admitted_request == request
                && value.observed_at >= *operation_observation.observed_at() => {}
            _ => {
                return Err(contextual(
                    "A2A task observation",
                    "exact prior cancellation-requested operation state",
                ));
            }
        }
    }
    Ok(())
}

fn admit_a2a_task_observation_predecessor<Authority>(
    value: &A2aTaskObservationPreimageV1,
    predecessor: Option<(&ResidentRecordRefV1, &A2aTaskObservationV1)>,
    admit_authority_rotation: Authority,
) -> Result<(), InvalidResidentRecordV1>
where
    Authority: FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    match (value.revision.get(), predecessor) {
        (1, None) => Ok(()),
        (1, Some(_)) | (_, None) => Err(contextual(
            "A2A task observation",
            "resolved predecessor at every successor revision",
        )),
        (_, Some((predecessor_ref, predecessor))) => {
            if predecessor.task_binding_id() != &value.task_binding_id
                || predecessor.task_binding_ref() != &value.task_binding_ref
                || predecessor.task_binding_sha256() != &value.task_binding_sha256
                || predecessor.revision().checked_successor() != Some(value.revision)
                || value.predecessor_ref.as_ref() != Some(predecessor_ref)
                || value.predecessor_sha256.as_ref()
                    != Some(predecessor.canonical_a2a_task_observation_sha256())
                || value.observed_at < *predecessor.observed_at()
                || !a2a_task_state_transition_is_monotone(
                    *predecessor.remote_state(),
                    value.remote_state,
                )
            {
                return Err(contextual(
                    "A2A task observation",
                    "exact monotone predecessor",
                ));
            }
            admit_runtime_authority_rotation(
                "A2A task observation",
                predecessor.deployment_runtime_authority_ref(),
                predecessor.deployment_runtime_authority_sha256(),
                &value.deployment_runtime_authority_ref,
                &value.deployment_runtime_authority_sha256,
                admit_authority_rotation,
            )
        }
    }
}

/// State class used by the A2A observation partial order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum A2aRemoteTaskPhaseV1 {
    Unspecified,
    Submitted,
    Active,
    Completed,
    Canceled,
    Failed,
    Rejected,
}

impl From<A2aRemoteTaskStateV1> for A2aRemoteTaskPhaseV1 {
    fn from(state: A2aRemoteTaskStateV1) -> Self {
        match state {
            A2aRemoteTaskStateV1::Unspecified => Self::Unspecified,
            A2aRemoteTaskStateV1::Submitted => Self::Submitted,
            A2aRemoteTaskStateV1::Working
            | A2aRemoteTaskStateV1::InputRequired
            | A2aRemoteTaskStateV1::AuthRequired => Self::Active,
            A2aRemoteTaskStateV1::Completed => Self::Completed,
            A2aRemoteTaskStateV1::Canceled => Self::Canceled,
            A2aRemoteTaskStateV1::Failed => Self::Failed,
            A2aRemoteTaskStateV1::Rejected => Self::Rejected,
        }
    }
}

/// Decide the A2A-v1 owner-state transition relation used by admission.
///
/// The quotient order groups work, input-required, and auth-required as one
/// active phase because the owner protocol may resume among them. Distinct
/// terminal phases remain incomparable and every terminal state is absorbing.
fn a2a_task_state_transition_is_monotone(
    predecessor: A2aRemoteTaskStateV1,
    successor: A2aRemoteTaskStateV1,
) -> bool {
    use A2aRemoteTaskPhaseV1::{Active, Submitted, Unspecified};
    let predecessor_phase = A2aRemoteTaskPhaseV1::from(predecessor);
    let successor_phase = A2aRemoteTaskPhaseV1::from(successor);
    match (predecessor_phase, successor_phase) {
        (Unspecified, _) => true,
        (Submitted, right) => right != Unspecified,
        (Active, right) => !matches!(right, Unspecified | Submitted),
        (left, right) => left == right && predecessor == successor,
    }
}

fn a2a_task_observation_schema_v1(schema: &mut schemars::Schema) {
    require_all_or_none_properties_v1(schema, &["predecessor_ref", "predecessor_sha256"]);
}

fn contextual(record: &'static str, invariant: &'static str) -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission { record, invariant }
}

#[cfg(test)]
#[path = "../../../tests/internal/records/resident/a2a.rs"]
mod tests;
