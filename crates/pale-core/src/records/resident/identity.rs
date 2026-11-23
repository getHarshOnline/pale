//! Resident identity, ordering, frontier, and process-local handle coordinates.

use serde::{Deserialize, Serialize};

use super::{
    resident_coordinate, resident_id, resident_sequence, resident_successor_coordinate,
    InvalidResidentRecordV1,
};

resident_id!(
    #[doc = "Stable identity of one authenticated administration submission."]
    LocalAdminCommandIdV1,
    "local administration command id"
);
resident_id!(
    #[doc = "Stable identity of one immutable administration outcome."]
    LocalAdminReceiptIdV1,
    "local administration receipt id"
);
resident_id!(
    #[doc = "Bounded operation-owned administration failure code, not diagnostic prose."]
    LocalAdminFailureCodeV1,
    "local administration failure code"
);
resident_id!(
    #[doc = "Replay key scoped by principal, admin authority and operation."]
    LocalAdminIdempotencyKeyV1,
    "local administration idempotency key"
);
resident_id!(
    #[doc = "Stable identity of one deployment control-store genesis receipt."]
    DeploymentControlStoreGenesisIdV1,
    "deployment control-store genesis id"
);
resident_id!(
    #[doc = "Principal-scoped immutable node-delegation lineage identity."]
    NodeDelegationIdV1,
    "node delegation id"
);
resident_id!(
    #[doc = "Stable owner-record signing-authority lineage identity."]
    OwnerRecordSigningAuthorityIdV1,
    "owner-record signing authority id"
);
resident_sequence!(
    #[doc = "Positive administration head comparison epoch; its selector owns interpretation."]
    LocalAdminHeadEpochV1,
    "local administration head epoch"
);

resident_id!(
    #[doc = "Stable activation-fence lineage identity."]
    ActivationFenceIdV1,
    "activation fence id"
);
resident_id!(
    #[doc = "Stable ambient session-admission identity."]
    SessionAdmissionIdV1,
    "session admission id"
);
resident_id!(
    #[doc = "Stable nested operation-lease identity."]
    OperationLeaseIdV1,
    "operation lease id"
);
resident_id!(
    #[doc = "Stable locator identity of one durably started Pale operation."]
    OperationHandleIdV1,
    "operation handle id"
);
resident_id!(
    #[doc = "Stable identity of one admitted A2A route record."]
    A2aRouteAdmissionIdV1,
    "A2A route admission id"
);
resident_id!(
    #[doc = "Stable identity binding one Pale operation to one remote A2A task."]
    A2aTaskBindingIdV1,
    "A2A task binding id"
);
resident_id!(
    #[doc = "Remote-owner A2A task identity; never a Pale operation identity."]
    A2aRemoteTaskIdV1,
    "A2A remote task id"
);
resident_id!(
    #[doc = "Remote-owner A2A context identity; never a Pale session identity."]
    A2aRemoteContextIdV1,
    "A2A remote context id"
);
resident_id!(
    #[doc = "Agent-Card security-scheme identity selected for one admitted A2A route."]
    A2aSecuritySchemeIdV1,
    "A2A security scheme id"
);
resident_id!(
    #[doc = "JSON-RPC request identity scoped to one admitted A2A attempt."]
    A2aJsonRpcRequestIdV1,
    "A2A JSON-RPC request id"
);
resident_id!(
    #[doc = "Optional owner-issued identifier of one observed A2A SSE event."]
    A2aSseEventIdV1,
    "A2A SSE event id"
);
resident_id!(
    #[doc = "Stable invocation correlation identity."]
    InvocationCorrelationIdV1,
    "invocation correlation id"
);
resident_id!(
    #[doc = "Owner-native invocation identity when safely exposed."]
    NativeInvocationIdV1,
    "native invocation id"
);
resident_id!(
    #[doc = "Stable event identity within one producer incarnation."]
    EventIdV1,
    "event id"
);
resident_id!(
    #[doc = "Bounded event semantic type."]
    EventTypeV1,
    "event type"
);
resident_id!(
    #[doc = "Bounded owner lifecycle namespace."]
    LifecycleNamespaceV1,
    "lifecycle namespace"
);
resident_id!(
    #[doc = "Native system that owns one source record."]
    SourceOwnerV1,
    "source owner"
);
resident_id!(
    #[doc = "Owner-defined immutable source version."]
    SourceVersionV1,
    "source version"
);
resident_id!(
    #[doc = "Correlation identity scoped to one admitted event lineage."]
    EventCorrelationIdV1,
    "event correlation id"
);
resident_id!(
    #[doc = "Causation identity scoped to one admitted event lineage."]
    EventCausationIdV1,
    "event causation id"
);
resident_id!(
    #[doc = "Owner-declared event ordering key."]
    EventOrderingKeyV1,
    "event ordering key"
);
resident_id!(
    #[doc = "Stable delivery-attempt identity."]
    DeliveryAttemptIdV1,
    "delivery attempt id"
);
resident_id!(
    #[doc = "Stable event-subscription identity."]
    EventSubscriptionIdV1,
    "event subscription id"
);
resident_id!(
    #[doc = "Consumer-owned state version after delivery."]
    ConsumerStateVersionV1,
    "consumer state version"
);
resident_id!(
    #[doc = "Stable capability-grant revocation identity."]
    CapabilityGrantRevocationIdV1,
    "capability grant revocation id"
);
resident_id!(
    #[doc = "Stable capability-revocation checkpoint identity."]
    CapabilityRevocationCheckpointIdV1,
    "capability revocation checkpoint id"
);
resident_id!(
    #[doc = "One-use session-grant nonce."]
    CapabilitySessionNonceV1,
    "capability session nonce"
);
resident_id!(
    #[doc = "Stable local route lineage identity."]
    LocalRouteIdV1,
    "local route id"
);
resident_id!(
    #[doc = "Stable neutral execution-owner fence lineage identity."]
    ExecutionOwnerFenceIdV1,
    "execution owner fence id"
);
resident_id!(
    #[doc = "Neutral execution-owner system identity."]
    ExecutionOwnerSystemIdV1,
    "execution owner system id"
);
resident_id!(
    #[doc = "Neutral execution-owner participant identity."]
    ExecutionOwnerParticipantIdV1,
    "execution owner participant id"
);
resident_id!(
    #[doc = "Stable interaction-occurrence identity."]
    InteractionOccurrenceIdV1,
    "interaction occurrence id"
);
resident_id!(
    #[doc = "Stable genealogy-edge identity."]
    GenealogyEdgeIdV1,
    "genealogy edge id"
);
resident_id!(
    #[doc = "Truth-class-local genealogy relation token."]
    GenealogyRelationV1,
    "genealogy relation"
);
resident_id!(
    #[doc = "Stable reconciliation-frontier identity."]
    ReconciliationFrontierIdV1,
    "reconciliation frontier id"
);
resident_id!(
    #[doc = "Owner-defined terminal frontier disposition."]
    OwnerFrontierDispositionV1,
    "owner frontier disposition"
);
resident_id!(
    #[doc = "Native container genre exposed by its owner."]
    NativeContainerGenreV1,
    "native container genre"
);
resident_id!(
    #[doc = "Visibility class exposed by the source owner."]
    InteractionVisibilityV1,
    "interaction visibility"
);
resident_id!(
    #[doc = "Disclosure ceiling admitted for an interaction occurrence."]
    InteractionDisclosureCeilingV1,
    "interaction disclosure ceiling"
);
resident_id!(
    #[doc = "Payload resolution policy bound to an occurrence."]
    PayloadResolutionPolicyV1,
    "payload resolution policy"
);
resident_id!(
    #[doc = "Bounded response-expectation class."]
    ResponseExpectationV1,
    "response expectation"
);
resident_id!(
    #[doc = "Bounded semantic-uncertainty classification."]
    SemanticUncertaintyV1,
    "semantic uncertainty"
);
resident_id!(
    #[doc = "Stable reconciliation-case identity."]
    ReconciliationCaseIdV1,
    "reconciliation case id"
);
resident_id!(
    #[doc = "Stable reconciliation-custody identity."]
    ReconciliationCustodyIdV1,
    "reconciliation custody id"
);
resident_id!(
    #[doc = "Stable reconciliation-request identity."]
    ReconciliationIdV1,
    "reconciliation id"
);
resident_id!(
    #[doc = "Stable transport-attempt identity for one capability invocation."]
    CapabilityAttemptIdV1,
    "capability attempt id"
);
resident_id!(
    #[doc = "Stable identity of one bounded adapter failure."]
    AdapterFailureIdV1,
    "adapter failure id"
);
resident_id!(
    #[doc = "Stable logical external-action identity."]
    ActionIdV1,
    "action id"
);
resident_id!(
    #[doc = "Stable conflict-resource claim identity."]
    EffectConflictClaimIdV1,
    "effect conflict claim id"
);
resident_id!(
    #[doc = "Stable target-system identity for an external action."]
    ActionTargetSystemV1,
    "action target system"
);
resident_id!(
    #[doc = "Stable target identity interpreted by the selected target system."]
    ActionTargetIdentityV1,
    "action target identity"
);
resident_id!(
    #[doc = "Bounded semantic operation selected by an external action."]
    ActionOperationV1,
    "action operation"
);
resident_id!(
    #[doc = "Stable business idempotency key for one logical action."]
    BusinessIdempotencyKeyV1,
    "business idempotency key"
);
resident_id!(
    #[doc = "Stable signing-key identity within its resolved authority."]
    ResidentKeyIdV1,
    "resident signing key id"
);

resident_successor_coordinate!(
    /// Positive principal-key epoch admitted by signed trust contracts.
    ///
    /// This trust-store coordinate is not substitutable for [`SigningKeyEpochV1`].
    PrincipalKeyEpochV1,
    "principal key epoch",
    positive
);
resident_coordinate!(
    /// Positive signing-key epoch resolved through one authority binding.
    ///
    /// This signature coordinate is not a principal trust-store key epoch.
    SigningKeyEpochV1,
    "signing key epoch",
    positive
);
resident_id!(
    #[doc = "Optional relationship boundary carried by a resident contract."]
    RelationshipScopeV1,
    "relationship scope"
);
resident_id!(
    #[doc = "Stable resident node identity within one principal."]
    ResidentNodeIdV1,
    "resident node id"
);
resident_id!(
    #[doc = "Exact repository revision spelling at a resident source frontier."]
    RepositoryRevisionV1,
    "repository revision"
);
resident_id!(
    #[doc = "Stable resident record-class token used in a control leaf key."]
    ControlRecordClassV1,
    "control record class"
);
resident_id!(
    #[doc = "Stable logical key within a typed control-leaf scope."]
    ControlLogicalKeyV1,
    "control logical key"
);
resident_id!(
    #[doc = "Stable control-checkpoint identity."]
    ControlCheckpointIdV1,
    "control checkpoint id"
);
resident_id!(
    #[doc = "Stable hard-conflict domain identity for control checkpoints."]
    ControlCheckpointDomainIdV1,
    "control checkpoint domain id"
);
resident_id!(
    #[doc = "Bounded policy- or owner-defined reason code."]
    ResidentReasonV1,
    "resident reason"
);
resident_id!(
    #[doc = "Bounded owner-defined observation kind."]
    ObservationKindV1,
    "observation kind"
);
resident_id!(
    #[doc = "Stable native system identity owning a referenced event."]
    OwnerSystemIdV1,
    "owner system id"
);
resident_id!(
    #[doc = "Stable effect-admission identity."]
    EffectAdmissionIdV1,
    "effect admission id"
);
resident_id!(
    #[doc = "Stable durable effect-dispatch intent identity."]
    EffectDispatchIntentIdV1,
    "effect dispatch intent id"
);
resident_id!(
    #[doc = "Stable caller identity under a resolved signing authority."]
    RequestCallerIdV1,
    "request caller id"
);
resident_id!(
    #[doc = "Stable admitted caller-role identity."]
    RequestCallerRoleV1,
    "request caller role"
);
resident_id!(
    #[doc = "Stable native or projected request-origin surface."]
    RequestCallerSurfaceV1,
    "request caller surface"
);
resident_id!(
    #[doc = "Stable native owner component identity."]
    OwnerComponentIdV1,
    "owner component id"
);
resident_id!(
    #[doc = "Stable admitted route identity."]
    ResidentRouteIdV1,
    "resident route id"
);
resident_id!(
    #[doc = "Stable end-to-end request correlation identity."]
    ResidentCorrelationIdV1,
    "resident correlation id"
);
resident_id!(
    #[doc = "Stable admitted session-grant identity."]
    SessionGrantIdV1,
    "session grant id"
);
resident_id!(
    #[doc = "Stable identity of one immutable deployment contract set."]
    ContractSetIdV1,
    "contract set id"
);
resident_id!(
    #[doc = "Exact workspace revision admitted by deployment terms."]
    WorkspaceCommitV1,
    "workspace commit"
);
resident_id!(
    #[doc = "Stable identity of one scoped deployment."]
    DeploymentIdV1,
    "deployment id"
);
resident_id!(
    #[doc = "Immutable distribution release selected for one deployment."]
    DistributionReleaseV1,
    "distribution release"
);
resident_id!(
    #[doc = "Bounded deployment-environment identity interpreted by its owner."]
    DeploymentEnvironmentV1,
    "deployment environment"
);
resident_id!(
    #[doc = "Bounded data-classification identity carried by deployment terms."]
    DataClassificationV1,
    "data classification"
);
resident_id!(
    #[doc = "Bounded purpose identity explicitly permitted by deployment terms."]
    PermittedPurposeV1,
    "permitted purpose"
);
resident_id!(
    #[doc = "Stable identity of one immutable deployment-binding proposal."]
    DeploymentProposalIdV1,
    "deployment proposal id"
);
resident_id!(
    #[doc = "Stable identity of one independently signed deployment acceptance."]
    DeploymentAcceptanceIdV1,
    "deployment acceptance id"
);
resident_id!(
    #[doc = "Stable identity of one deployment-acceptance revocation lineage."]
    DeploymentAcceptanceRevocationIdV1,
    "deployment acceptance revocation id"
);
resident_id!(
    #[doc = "Stable identity of one finalized deployment binding."]
    DeploymentBindingIdV1,
    "deployment binding id"
);
resident_id!(
    #[doc = "Stable identity of one deployment-runtime-authority lineage."]
    DeploymentRuntimeAuthorityIdV1,
    "deployment runtime authority id"
);
resident_id!(
    #[doc = "Stable organized-execution attempt identity."]
    ExecutionAttemptIdV1,
    "execution attempt id"
);
resident_id!(
    #[doc = "Stable federation-space identity."]
    FederationSpaceIdV1,
    "federation space id"
);
resident_id!(
    #[doc = "Stable federation-membership identity."]
    FederationMembershipIdV1,
    "federation membership id"
);
resident_id!(
    #[doc = "Stable federation-grant identity."]
    FederationGrantIdV1,
    "federation grant id"
);
resident_id!(
    #[doc = "Stable workspace identity carried by a common request."]
    ResidentWorkspaceIdV1,
    "resident workspace id"
);
resident_id!(
    #[doc = "Stable semantic idempotency key of a common request."]
    RequestIdempotencyKeyV1,
    "request idempotency key"
);
resident_id!(
    #[doc = "Bounded resident text used only where no separate identity domain exists."]
    BoundedResidentTextV1,
    "resident text"
);
resident_id!(
    #[doc = "Stable common-request identity."]
    ResidentRequestIdV1,
    "request id"
);
resident_id!(
    #[doc = "Stable control-store identity."]
    ControlStoreIdV1,
    "control store id"
);
resident_id!(
    #[doc = "Stable control-store incarnation identity."]
    ControlIncarnationIdV1,
    "control incarnation id"
);
resident_id!(
    #[doc = "Stable principal resident-writer fence lineage identity."]
    PrincipalResidentWriterFenceIdV1,
    "principal resident writer fence id"
);
resident_id!(
    #[doc = "Stable clean resident-writer terminal-receipt identity."]
    PrincipalResidentWriterTerminalReceiptIdV1,
    "principal resident writer terminal receipt id"
);
resident_id!(
    #[doc = "Stable authenticated resident-writer takeover-receipt identity."]
    PrincipalResidentWriterTakeoverReceiptIdV1,
    "principal resident writer takeover receipt id"
);
resident_id!(
    #[doc = "Stable process-instance identity allocated by the resident launcher."]
    ResidentProcessInstanceIdV1,
    "resident process instance id"
);
resident_id!(
    #[doc = "Stable machine-boot incarnation identity observed by the resident launcher."]
    ResidentBootIncarnationIdV1,
    "resident boot incarnation id"
);
resident_id!(
    #[doc = "One-use nonce bound to an authenticated resident-writer takeover."]
    PrincipalResidentWriterTakeoverNonceV1,
    "principal resident writer takeover nonce"
);
resident_id!(
    #[doc = "Stable producer identity within one principal's allocation registry."]
    ProducerIdV1,
    "producer id"
);
resident_id!(
    #[doc = "Stable identity of one producer-incarnation allocation record."]
    ProducerIncarnationIdV1,
    "producer incarnation id"
);
resident_id!(
    #[doc = "Stable identity of one producer-incarnation retirement record."]
    ProducerIncarnationRetirementIdV1,
    "producer incarnation retirement id"
);
resident_id!(
    #[doc = "Stable identity of one principal recovery policy lineage."]
    PrincipalRecoveryPolicyIdV1,
    "principal recovery policy id"
);
resident_id!(
    #[doc = "Stable identity of one principal recovery proof."]
    PrincipalRecoveryProofIdV1,
    "principal recovery proof id"
);
resident_id!(
    #[doc = "One-use nonce selected for one principal recovery proof."]
    PrincipalRecoveryNonceV1,
    "principal recovery nonce"
);
resident_id!(
    #[doc = "Stable identity of one local-administration authority lineage."]
    LocalAdminAuthorityIdV1,
    "local admin authority id"
);
resident_id!(
    #[doc = "Stable local-administrator subject identity."]
    LocalAdminSubjectIdentityV1,
    "local admin subject identity"
);
resident_id!(
    #[doc = "Stable identity of one producer-incarnation authority lineage."]
    ProducerIncarnationAuthorityIdV1,
    "producer incarnation authority id"
);
resident_id!(
    #[doc = "Canonical producer namespace rule interpreted by its authority owner."]
    ProducerNamespaceV1,
    "producer namespace"
);
resident_id!(
    #[doc = "Stable identity of one checkpoint-signing authority lineage."]
    CheckpointSigningAuthorityIdV1,
    "checkpoint signing authority id"
);
resident_id!(
    #[doc = "Canonical checkpoint-domain prefix interpreted by its profile owner."]
    CheckpointDomainPrefixV1,
    "checkpoint domain prefix"
);
resident_id!(
    #[doc = "Stable identity of one principal trust-checkpoint record."]
    PrincipalTrustCheckpointIdV1,
    "principal trust checkpoint id"
);
resident_id!(
    #[doc = "Stable identity of one offline principal bootstrap transaction."]
    PrincipalBootstrapIdV1,
    "principal bootstrap id"
);
resident_id!(
    #[doc = "Immutable bounded resident or owner-native reference."]
    ResidentRecordRefV1,
    "resident record ref"
);
resident_id!(
    #[doc = "Stable principal scope identity."]
    ResidentPrincipalIdV1,
    "principal id"
);
resident_id!(
    #[doc = "Stable controller scope identity."]
    ResidentControllerIdV1,
    "controller id"
);
resident_id!(
    #[doc = "Stable logical device scope in a resource-continuity declaration."]
    ResourceContinuityDeviceScopeIdV1,
    "resource continuity device scope id"
);
resident_id!(
    #[doc = "Stable logical machine scope in a resource-continuity declaration."]
    ResourceContinuityMachineScopeIdV1,
    "resource continuity machine scope id"
);
resident_id!(
    #[doc = "Stable owner namespace for an opaque native revision."]
    NativeRevisionOwnerV1,
    "native revision owner"
);
resident_id!(
    #[doc = "Owner-opaque native revision/CAS token."]
    NativeRevisionTokenV1,
    "native revision token"
);
resident_id!(
    #[doc = "Stable source-index provider instance."]
    ProviderIndexOwnerV1,
    "provider index owner"
);
resident_id!(
    #[doc = "Provider-opaque immutable index generation."]
    ProviderIndexGenerationV1,
    "provider index generation"
);

macro_rules! canonical_base64url {
    (
        $(#[$meta:meta])*
        $name:ident,
        $kind:literal,
        $encoded_len:literal,
        $pattern:literal,
        $final_chars:literal
    ) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            schemars::JsonSchema,
        )]
        #[serde(transparent)]
        pub struct $name(#[schemars(regex(pattern = $pattern))] String);

        impl $name {
            /// Validate one fixed-length canonical unpadded-base64url value.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidResidentRecordV1::InvalidIdentity`] unless the
            /// text uses the RFC 4648 URL-safe alphabet, has the exact encoded
            /// length, and has zero unused trailing bits.
            pub fn parse(value: impl Into<String>) -> Result<Self, InvalidResidentRecordV1> {
                let value = value.into();
                let alphabet = value.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
                });
                let canonical_tail = value
                    .as_bytes()
                    .last()
                    .is_some_and(|byte| $final_chars.as_bytes().contains(byte));
                if value.len() == $encoded_len && alphabet && canonical_tail {
                    Ok(Self(value))
                } else {
                    Err(InvalidResidentRecordV1::InvalidIdentity { kind: $kind })
                }
            }

            /// Borrow the canonical unpadded-base64url spelling.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
    };
}

canonical_base64url!(
    /// Ed25519 public key encoded as 32 bytes of canonical unpadded base64url.
    ///
    /// The 43-character representation admits only the four final symbols
    /// whose low two unused bits are zero.
    Ed25519PublicKeyV1,
    "Ed25519 public key",
    43,
    r"^[A-Za-z0-9_-]{42}[AEIMQUYcgkosw048]$",
    "AEIMQUYcgkosw048"
);

canonical_base64url!(
    /// Ed25519 signature encoded as 64 bytes of canonical unpadded base64url.
    ///
    /// The 86-character representation admits only the four final symbols
    /// whose low four unused bits are zero. The name is retained as the shared
    /// signed-resident wire type; initial v1 has exactly one signature profile.
    CanonicalSignatureV1,
    "Ed25519 signature",
    86,
    r"^[A-Za-z0-9_-]{85}[AQgw]$",
    "AQgw"
);

resident_coordinate!(
    /// Wire-safe Unix timestamp in whole seconds.
    ///
    /// Zero is the Unix epoch. This type cannot be substituted for a sequence,
    /// duration, generation, or revocation coordinate.
    UnixTimeV1,
    "Unix timestamp",
    zero_aware
);
resident_coordinate!(
    /// Positive sequence of an owner-published signing-authority record.
    OwnerSignerRecordSequenceV1,
    "owner signer record sequence",
    positive
);
resident_successor_coordinate!(
    /// Positive generation of one admitted request route.
    RouteGenerationV1,
    "route generation",
    positive
);
resident_coordinate!(
    /// Positive principal revocation frontier checked during admission.
    PrincipalRevocationEpochV1,
    "principal revocation epoch",
    positive
);
resident_successor_coordinate!(
    /// Positive generation of one selected neutral execution owner.
    ExecutionOwnerGenerationV1,
    "execution owner generation",
    positive
);
resident_successor_coordinate!(
    /// Positive control epoch within one selected execution-owner generation.
    ExecutionControlEpochV1,
    "execution control epoch",
    positive
);
resident_successor_coordinate!(
    /// Positive append-only epoch of one capability-grant revocation lineage.
    CapabilityGrantRevocationEpochV1,
    "capability grant revocation epoch",
    positive
);
resident_successor_coordinate!(
    /// Positive anti-rollback checkpoint epoch for capability revocations.
    CapabilityRevocationCheckpointEpochV1,
    "capability revocation checkpoint epoch",
    positive
);
resident_coordinate!(
    /// Zero-aware number of grants covered by one revocation checkpoint.
    CoveredGrantCountV1,
    "covered grant count",
    zero_aware
);
resident_coordinate!(
    /// Positive sequence declared only within one event ordering key.
    OwnerEventSequenceV1,
    "owner event sequence",
    positive
);
resident_coordinate!(
    /// Semantic confidence in basis points, bounded later by its genealogy proof.
    GenealogyConfidenceBasisPointsV1,
    "genealogy confidence basis points",
    zero_aware
);
resident_coordinate!(
    /// Positive governance epoch of one federation space.
    FederationGovernanceEpochV1,
    "federation governance epoch",
    positive
);
resident_coordinate!(
    /// Positive membership-state epoch within one federation space.
    FederationMembershipEpochV1,
    "federation membership epoch",
    positive
);
resident_successor_coordinate!(
    /// Positive immutable succession generation of an effect-conflict claim.
    EffectConflictClaimGenerationV1,
    "effect conflict claim generation",
    positive
);
resident_coordinate!(
    /// Positive epoch fencing incompatible control-domain histories.
    ControlDomainEpochV1,
    "control domain epoch",
    positive
);
resident_successor_coordinate!(
    /// Positive checkpoint sequence within one [`ControlDomainEpochV1`].
    ControlDomainSequenceV1,
    "control domain sequence",
    positive
);
resident_coordinate!(
    /// Positive durable control transaction included in a checkpoint.
    ControlTransactionSequenceV1,
    "control transaction sequence",
    positive
);
resident_coordinate!(
    /// Zero-aware number of authenticated leaves in a control checkpoint.
    ControlLeafCountV1,
    "control leaf count",
    zero_aware
);
resident_coordinate!(
    /// Positive abstract budget admitted for a session or child operation.
    LeaseBudgetUnitsV1,
    "lease budget units",
    positive
);
resident_coordinate!(
    /// Positive execution timeout in whole milliseconds.
    LeaseTimeoutMillisV1,
    "lease timeout milliseconds",
    positive
);
resident_successor_coordinate!(
    /// Zero-aware immutable revision of one reconciliation result chain.
    ReconciliationCaseRevisionV1,
    "reconciliation case revision",
    zero_aware
);
resident_successor_coordinate!(
    /// Positive exclusive-custody generation for one reconciliation case.
    ReconciliationCustodyGenerationV1,
    "reconciliation custody generation",
    positive
);

resident_sequence!(
    #[doc = "Monotone activation-fence generation."]
    ActivationFenceGenerationV1,
    "activation fence generation"
);
resident_sequence!(
    #[doc = "Positive revision of one operation-observation lineage."]
    OperationObservationRevisionV1,
    "operation observation revision"
);
resident_sequence!(
    #[doc = "Positive revision of one A2A task-observation lineage."]
    A2aTaskObservationRevisionV1,
    "A2A task observation revision"
);
resident_sequence!(
    #[doc = "Store-global durable journal append coordinate."]
    JournalSequenceV1,
    "journal sequence"
);
resident_sequence!(
    #[doc = "Positive checkpoint generation, distinct from journal sequence."]
    CheckpointGenerationV1,
    "checkpoint generation"
);
resident_sequence!(
    #[doc = "Positive principal resident-writer generation."]
    PrincipalResidentWriterGenerationV1,
    "principal resident writer generation"
);
resident_sequence!(
    #[doc = "Positive principal session-floor epoch."]
    PrincipalSessionFloorEpochV1,
    "principal session floor epoch"
);
resident_sequence!(
    #[doc = "Positive epoch of one principal recovery-policy lineage."]
    PrincipalRecoveryPolicyEpochV1,
    "principal recovery policy epoch"
);
resident_sequence!(
    #[doc = "Positive epoch of one local-administration authority lineage."]
    LocalAdminAuthorityEpochV1,
    "local admin authority epoch"
);
resident_sequence!(
    #[doc = "Positive epoch of one producer-incarnation authority lineage."]
    ProducerIncarnationAuthorityEpochV1,
    "producer incarnation authority epoch"
);
resident_sequence!(
    #[doc = "Positive epoch of one checkpoint-signing authority lineage."]
    CheckpointSignerEpochV1,
    "checkpoint signer epoch"
);
resident_sequence!(
    #[doc = "Positive anti-rollback epoch of the principal trust frontier."]
    PrincipalTrustCheckpointEpochV1,
    "principal trust checkpoint epoch"
);
resident_sequence!(
    #[doc = "Positive epoch of one deployment-acceptance revocation lineage."]
    DeploymentAcceptanceRevocationEpochV1,
    "deployment acceptance revocation epoch"
);
resident_sequence!(
    #[doc = "Positive epoch of one deployment-runtime-authority lineage."]
    DeploymentRuntimeAuthorityEpochV1,
    "deployment runtime authority epoch"
);
resident_coordinate!(
    /// Positive lifetime in whole seconds for one recovery proof.
    RecoveryProofValiditySecondsV1,
    "recovery proof validity seconds",
    positive
);

/// Runtime wake epoch. Zero is the initial process-local epoch.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(transparent)]
pub struct WakeEpochV1(u64);
impl WakeEpochV1 {
    /// Construct a zero-aware runtime wake epoch.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    /// Return the process-local wake epoch.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Reclamation epoch. Zero means no reclamation frontier has advanced.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(transparent)]
pub struct ReclamationEpochV1(u64);
impl ReclamationEpochV1 {
    /// Construct a zero-aware reclamation epoch.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    /// Return the process-local reclamation epoch.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Process-local transient per-consumer ring cursor.
///
/// Zero denotes the initial ring frontier. This coordinate does not persist a
/// durable consumer replay position across process lifetimes.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(transparent)]
pub struct RingCursorV1(u64);
impl RingCursorV1 {
    /// Construct a zero-aware process-local per-consumer ring cursor.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    /// Return the current process-local per-consumer ring cursor.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Owner-scoped mutable native revision/CAS coordinate.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct NativeRevisionV1 {
    /// Native owner defining token equality and succession.
    pub native_revision_owner: NativeRevisionOwnerV1,
    /// Opaque revision token interpreted only by that owner.
    pub native_revision_token: NativeRevisionTokenV1,
}

/// Exact repository/worktree/source frontier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RepositorySourceFrontierV1 {
    /// Stable repository or worktree identity.
    pub repository_id: crate::records::LogicalRepositoryIdV1,
    /// Exact base revision from which the frontier is measured.
    pub base_revision: RepositoryRevisionV1,
    /// Digest of the tracked source delta.
    pub tracked_delta_sha256: crate::records::ArtifactDigest,
    /// Digest of the bounded untracked-source inventory.
    pub untracked_inventory_sha256: crate::records::ArtifactDigest,
    /// Digest of the exact dependency frontier.
    pub dependency_frontier_sha256: crate::records::ArtifactDigest,
}

/// Exact replaceable provider index over one source frontier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderIndexCoordinateV1 {
    /// Native provider that owns the replaceable index.
    pub provider_index_owner: ProviderIndexOwnerV1,
    /// Owner-opaque immutable index generation.
    pub provider_index_generation: ProviderIndexGenerationV1,
    /// Exact source frontier indexed by this generation.
    pub repository_source_frontier: RepositorySourceFrontierV1,
}

/// Validate that one successor sequence advances exactly once.
///
/// # Errors
/// Returns [`InvalidResidentRecordV1::InvalidCoordinate`] unless `successor` is
/// exactly one greater than `predecessor`, including at portable-domain exhaustion.
pub fn validate_sequence_successor(
    predecessor: JournalSequenceV1,
    successor: JournalSequenceV1,
) -> Result<(), InvalidResidentRecordV1> {
    if predecessor.checked_successor() == Some(successor) {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::InvalidCoordinate {
            coordinate: "journal sequence successor",
        })
    }
}
