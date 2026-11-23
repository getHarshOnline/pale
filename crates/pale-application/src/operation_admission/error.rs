//! Fail-closed failures for sealed operation activation and authorization.
//!
//! [`OperationAdmissionError`] keeps artifact integrity, route identity,
//! availability, authorization, and replay failures distinct. None of these
//! variants is evidence that a provider effect occurred.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use pale_core::records::{
    CapabilityProvider, CapabilityRequirementIdV1, EffectClass, InvalidHostIdentity,
    InvalidOperationContractFamilyV1, PackageResourceRefV1, ToolAuthorizationDigest,
};

/// Failure to verify, activate, or apply one sealed operation contract.
#[derive(Debug, thiserror::Error)]
pub enum OperationAdmissionError {
    /// Exact artifact bytes do not match the digest sealed by their parent record.
    #[error("operation artifact `{artifact}` does not match its sealed byte digest")]
    ArtifactDigestMismatch {
        /// Confined artifact identity whose bytes were replaced or misrouted.
        artifact: String,
    },
    /// Exact sealed bytes cannot be decoded as their expected typed record.
    #[error("operation artifact `{artifact}` is not a valid typed record")]
    MalformedArtifact {
        /// Confined artifact identity whose bytes failed strict decoding.
        artifact: String,
    },
    /// Model-facing activation content is missing, extra, or byte-mismatched.
    #[error("operation activation closure is incomplete at `{artifact}`")]
    SemanticClosureMismatch {
        /// Missing or unexpected closure member identity.
        artifact: String,
    },
    /// A deserialized operation-family record violates its core-owned contract.
    #[error("invalid {family}: {source}")]
    InvalidOperationRecord {
        /// Exact record family being admitted.
        family: &'static str,
        /// Structural, schema-coordinate, canonical-order, or self-digest failure.
        #[source]
        source: InvalidOperationContractFamilyV1,
    },
    /// A catalog identity occurs more than once.
    #[error("operation catalog contains duplicate identity `{identity}`")]
    DuplicateIdentity {
        /// Repeated operation id, invocation, alias, or contract path.
        identity: String,
    },
    /// Route binding and shared catalog describe different identities.
    #[error("operation route binding and catalog identities do not match")]
    BindingCatalogMismatch,
    /// Internally valid operation artifacts belong to another sealed route context.
    #[error("operation bundle does not match the expected profile, repository, and route")]
    RouteContextMismatch,
    /// Catalog entry and loaded contract do not describe the same operation.
    #[error("operation catalog entry and contract do not match for `{operation}`")]
    EntryContractMismatch {
        /// Operation whose compact and full records disagree.
        operation: PackageResourceRefV1,
    },
    /// Requested operation is absent from the sealed catalog.
    #[error("unknown Pale operation `{operation}`")]
    UnknownOperation {
        /// Requested operation identity or invocation.
        operation: String,
    },
    /// Prompt evidence came from a weaker boundary than this native harness exposes.
    #[error("operation invocation origin does not match the native harness contract")]
    InvocationOriginMismatch,
    /// Direct prompt activation was attempted through a delegated execution path.
    #[error("direct operation invocation does not match the witnessed execution path")]
    InvocationPathMismatch,
    /// A tool event omitted the host identity needed to correlate pre and post observations.
    #[error("tool event omitted its native per-call identity")]
    MissingToolUseIdentity,
    /// A supplied native tool identity exceeded bounds or contained control characters.
    #[error("native per-call tool identity is invalid")]
    InvalidToolUseIdentity,
    /// A tool event cannot be bound to a durable native-harness activity.
    #[error("tool event omitted both native session and turn identity")]
    MissingActivityIdentity,
    /// A supplied native session or turn identity violated its bounded contract.
    #[error("native harness activity identity is invalid: {0}")]
    HostIdentity(#[from] InvalidHostIdentity),
    /// Requested operation exists but is unavailable on this route.
    #[error("operation `{operation}` is not available for this route")]
    Unavailable {
        /// Selected operation identity.
        operation: PackageResourceRefV1,
    },
    /// Explicit authorization does not use the required `requirement=digest` form.
    #[error("operation authorization flag is malformed")]
    MalformedAuthorization,
    /// An authorization names a requirement not gated by the selected operation.
    #[error(
        "operation `{operation}` does not accept explicit authorization for requirement `{requirement}`"
    )]
    UnknownAuthorization {
        /// Selected operation identity.
        operation: PackageResourceRefV1,
        /// Requirement for which this operation accepts no authorization.
        requirement: CapabilityRequirementIdV1,
    },
    /// Session lease no longer matches the route's sealed operation identities.
    #[error("active operation admission is stale or belongs to another route")]
    StaleLease,
    /// Session lease belongs to a different host turn.
    #[error("active operation admission belongs to another turn")]
    CrossTurnLease,
    /// The same native tool identity already has an unconsumed admission.
    #[error("native tool invocation has already been admitted and remains in flight")]
    ToolInvocationReplay,
    /// No unconsumed admission exists for this native post-tool identity.
    #[error("post-tool event has no matching unconsumed pre-tool admission")]
    ToolInvocationNotAdmitted,
    /// A post-tool event reused an identity with different route, tool, or input facts.
    #[error("post-tool event does not match its admitted pre-tool invocation")]
    ToolInvocationMismatch,
    /// A capability failure transition produced no durable failure state.
    #[error("capability failure did not produce an operation failure transition")]
    FailureTransitionMissing,
    /// Tool invocation matches neither an auxiliary tool nor an admitted capability.
    #[error("active operation `{operation}` does not admit tool `{tool}`")]
    ToolOutsideContract {
        /// Active operation identity.
        operation: PackageResourceRefV1,
        /// Native-harness tool identity observed at the boundary.
        tool: String,
    },
    /// Exact owner authorization is required before this invocation may proceed.
    #[error(
        "active operation `{operation}` requires exact authorization `{requirement}={scope_digest}` for this tool request"
    )]
    ToolAuthorizationRequired {
        /// Active operation identity.
        operation: PackageResourceRefV1,
        /// Capability requirement awaiting owner authorization.
        requirement: CapabilityRequirementIdV1,
        /// Digest of the exact consequential tool request.
        scope_digest: ToolAuthorizationDigest,
    },
    /// More than one capability contract admits the same invocation.
    #[error(
        "active operation `{operation}` has ambiguous contracts for tool `{tool}`: {requirements}"
    )]
    AmbiguousToolContract {
        /// Active operation identity.
        operation: PackageResourceRefV1,
        /// Native-harness tool identity matching more than one capability.
        tool: String,
        /// Deterministically ordered matching requirement identities.
        requirements: String,
    },
    /// A prior fail-closed capability failure halted the active operation.
    #[error(
        "active operation `{operation}` is halted after fail-closed requirement `{requirement}` failed"
    )]
    OperationHalted {
        /// Active operation identity.
        operation: PackageResourceRefV1,
        /// Failed requirement whose posture halted the operation.
        requirement: CapabilityRequirementIdV1,
    },
    /// A prior steering failure permits diagnostic work but rejects this effect.
    #[error(
        "active operation `{operation}` is steering after requirement `{requirement}` failed and does not admit another `{effect}` effect"
    )]
    OperationSteering {
        /// Active operation identity.
        operation: PackageResourceRefV1,
        /// Failed requirement whose posture narrowed subsequent work.
        requirement: CapabilityRequirementIdV1,
        /// Effect class rejected while diagnostic steering is active.
        effect: EffectClass,
    },
    /// A prior retry posture admits only another attempt at the same requirement.
    #[error(
        "active operation `{operation}` is waiting to retry failed requirement `{requirement}` and does not admit `{attempted}`"
    )]
    OperationRetrying {
        /// Active operation identity.
        operation: PackageResourceRefV1,
        /// Requirement that must be retried.
        requirement: CapabilityRequirementIdV1,
        /// Different requirement or auxiliary tool that was attempted.
        attempted: String,
    },
    /// A prior failure requires owner escalation before any further operation work.
    #[error(
        "active operation `{operation}` requires owner escalation after requirement `{requirement}` failed"
    )]
    OperationEscalated {
        /// Active operation identity.
        operation: PackageResourceRefV1,
        /// Failed requirement requiring owner escalation.
        requirement: CapabilityRequirementIdV1,
    },
    /// The selected host route cannot observe the failure needed by this posture.
    #[error(
        "operation `{operation}` requirement `{requirement}` cannot observe a failed `{effect}` effect from its `{provider}` provider through {adapter} hooks"
    )]
    FailureObservationUnavailable {
        /// Operation whose route-specific failure transition is unenforceable.
        operation: PackageResourceRefV1,
        /// Capability requiring a stronger failure observation.
        requirement: CapabilityRequirementIdV1,
        /// Exact route-resolved provider whose failure needs observation.
        provider: CapabilityProvider,
        /// Exact effect whose failed attempt must remain distinguishable.
        effect: EffectClass,
        /// Selected adapter that lacks the necessary event.
        adapter: TerminalAdapterDescriptor,
    },
    /// Canonicalization of an operation record failed.
    #[error("operation contract could not be canonicalized: {0}")]
    Canon(#[from] pale_core::canon::CanonError),
    /// A verified operation transition would exceed the bounded session record.
    #[error("operation session state rejected the transition: {0}")]
    SessionState(#[from] crate::session_state::SessionStateError),
}
