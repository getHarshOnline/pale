//! Durable operation locators and predecessor-linked observations.
//!
//! Signed locator records are prepared before custody; only the application
//! may expose an issued handle after durable start and independent confirmation. Later
//! observations reuse the established result, diagnostic, receipt, and
//! reconciliation records by exact reference; they do not redefine those
//! contracts or claim a native effect. Both families bind the exact
//! [`DeploymentRuntimeAuthorityV1`] that issued the signed record.

use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for immutable operation handles.
pub const OPERATION_HANDLE_V1_SCHEMA_ID: &str = "operation-handle.v1";
/// Wire schema carried by every operation handle.
pub const OPERATION_HANDLE_V1_SCHEMA: &str = "pale/operation-handle.v1";
/// Signature domain for [`OperationHandleV1`].
pub const OPERATION_HANDLE_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_OPERATION_HANDLE_V1\0";

/// Public schema root for operation-observation revisions.
pub const OPERATION_OBSERVATION_V1_SCHEMA_ID: &str = "operation-observation.v1";
/// Wire schema carried by every operation observation.
pub const OPERATION_OBSERVATION_V1_SCHEMA: &str = "pale/operation-observation.v1";
/// Signature domain for [`OperationObservationV1`].
pub const OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_OPERATION_OBSERVATION_V1\0";

/// Responsibility relationship of one durably started operation.
///
/// [`OperationHandleV1`] persists this classification because observation and
/// recovery must not infer whether responsibility was retained or transferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperationClassV1 {
    /// Exercise one bounded capability while the caller retains responsibility.
    Invoke,
    /// Solicit attributable contributions while the parent retains integration.
    Consult,
    /// Transfer one bounded child responsibility under narrower authority.
    Delegate,
}

/// Exact parent coordinate for a child [`OperationHandleV1`].
///
/// The locator and canonical digest are intentionally carried together. The
/// referenced handle and its [`OperationLeaseV1`] must be resolved during
/// contextual admission before this coordinate can establish parent authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParentOperationReferenceV1 {
    /// Stable parent operation-handle identity.
    pub operation_handle_id: OperationHandleIdV1,
    /// Immutable parent operation-handle reference.
    pub operation_handle_ref: ResidentRecordRefV1,
    /// Canonical identity of the exact parent handle.
    pub operation_handle_sha256: OperationHandleDigest,
}

/// Exact reference to an existing [`CapabilityResultV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityResultReferenceV1 {
    /// Immutable capability-result record reference.
    pub record_ref: ResidentRecordRefV1,
    /// Canonical identity of the referenced result.
    pub record_sha256: CapabilityResultDigest,
}

/// Exact reference to an existing [`EffectReceiptV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffectReceiptReferenceV1 {
    /// Immutable effect-receipt record reference.
    pub record_ref: ResidentRecordRefV1,
    /// Canonical identity of the referenced receipt.
    pub record_sha256: EffectReceiptDigest,
}

/// Exact reference to an existing [`ReconciliationFrontierV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationFrontierReferenceV1 {
    /// Immutable reconciliation-frontier record reference.
    pub record_ref: ResidentRecordRefV1,
    /// Canonical identity of the referenced frontier.
    pub record_sha256: ReconciliationFrontierDigest,
}

/// Active phase of one durably started operation.
///
/// The variants expose only the evidence or existing owner coordinate needed
/// to resume the phase. They do not duplicate results, diagnostics, receipts,
/// reconciliation state, provider records, or foreign transport payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationActivePhaseV1 {
    /// The canonical application owner is resolving the admitted request.
    Resolving,
    /// Bounded contextual material has been admitted for the operation.
    ContextBound {
        /// Exact immutable evidence for the admitted contextual projection.
        evidence: ImmutableRecordReferenceV1,
    },
    /// The operation has resolved its exact private provider binding.
    ProviderResolved {
        /// Exact provider binding selected by the application owner.
        provider_binding: ProviderBindingRefV1,
    },
    /// One transport attempt is prepared but has not crossed dispatch.
    DispatchPrepared {
        /// Attempt identity reserved by the existing capability owner.
        attempt_id: CapabilityAttemptIdV1,
        /// Exact immutable preparation evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// One transport attempt has crossed its durable dispatch boundary.
    Dispatched {
        /// Attempt identity that crossed dispatch.
        attempt_id: CapabilityAttemptIdV1,
        /// Exact immutable dispatch evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// A consultation is collecting independently attributable child results.
    CollectingConsultation {
        /// Exact immutable collection-frontier evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// Collection stopped under its admitted policy or boundary condition.
    /// The parent can still evaluate or synthesize the collected evidence.
    ConsultationCollected {
        /// Exact immutable stopped collection-frontier evidence.
        evidence: ImmutableRecordReferenceV1,
    },
}

impl OperationActivePhaseV1 {
    /// Return the dispatched attempt eligible for a fresh remote-task binding.
    #[must_use]
    pub const fn dispatched_attempt_id(&self) -> Option<&CapabilityAttemptIdV1> {
        match self {
            Self::Dispatched { attempt_id, .. } => Some(attempt_id),
            _ => None,
        }
    }

    /// Return the phase rank used by the active-state successor envelope.
    const fn rank(&self) -> u8 {
        match self {
            Self::Resolving => 0,
            Self::ContextBound { .. } => 1,
            Self::ProviderResolved { .. } => 2,
            Self::DispatchPrepared { .. } => 3,
            Self::Dispatched { .. } => 4,
            Self::CollectingConsultation { .. } => 5,
            Self::ConsultationCollected { .. } => 6,
        }
    }
}

/// Closed state of one durable Pale operation observation.
///
/// Each evidence member is a reference to its existing semantic owner. In
/// particular, `succeeded` does not itself prove an effect, and
/// `outcome_unknown` does not invent a retry decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationObservationStateV1 {
    /// The durably started operation remains in one exact active phase.
    Active {
        /// Monotone application-owned execution phase.
        phase: OperationActivePhaseV1,
    },
    /// Cancellation was requested but is not yet a terminal observation.
    CancellationRequested {
        /// Exact immutable cancellation-request evidence.
        request: ImmutableRecordReferenceV1,
    },
    /// The operation is recovering from an interruption or uncertain boundary.
    Recovering {
        /// Exact existing reconciliation frontier governing recovery.
        reconciliation: ReconciliationFrontierReferenceV1,
    },
    /// The operation produced a successful capability result.
    Succeeded {
        /// Exact existing capability result.
        result: CapabilityResultReferenceV1,
        /// Effect receipt when this operation crossed an effect boundary.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        effect_receipt: Option<EffectReceiptReferenceV1>,
    },
    /// The operation terminated unsuccessfully.
    Failed {
        /// Existing failed capability result when one was produced.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<CapabilityResultReferenceV1>,
        /// Exact diagnostic explaining this terminal state.
        diagnostic: ImmutableRecordReferenceV1,
    },
    /// Cancellation was confirmed through its owning boundary.
    Cancelled {
        /// Exact diagnostic carrying the cancellation evidence and recovery posture.
        diagnostic: ImmutableRecordReferenceV1,
    },
    /// The operation crossed an uncertain boundary and awaits reconciliation.
    OutcomeUnknown {
        /// Exact diagnostic carrying the existing uncertainty classification.
        diagnostic: ImmutableRecordReferenceV1,
        /// Exact existing reconciliation frontier.
        reconciliation: ReconciliationFrontierReferenceV1,
        /// Effect receipt when the uncertainty followed external dispatch.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        effect_receipt: Option<EffectReceiptReferenceV1>,
    },
    /// The operation expired before another terminal state was admitted.
    Expired {
        /// Exact diagnostic carrying expiry and retry guidance.
        diagnostic: ImmutableRecordReferenceV1,
    },
}

impl OperationObservationStateV1 {
    /// Whether admitted conclusive evidence closes this observation lineage.
    ///
    /// Execution expiry and an unknown outcome do not close the lineage.
    /// Later observations may establish their outcome within handle retention;
    /// neither state grants authority to resume execution or retry a dispatch.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Succeeded { .. } | Self::Failed { .. } | Self::Cancelled { .. }
        )
    }

    /// Return the exact dispatched attempt eligible for a fresh task binding.
    #[must_use]
    pub const fn dispatched_attempt_id(&self) -> Option<&CapabilityAttemptIdV1> {
        match self {
            Self::Active { phase } => phase.dispatched_attempt_id(),
            _ => None,
        }
    }
}

resident_signed_record!(
    /// Signed immutable locator record for one Pale operation.
    ///
    /// Contextual admission binds the exact request, lease, compiled generation, activation fence, and
    /// private instance needed for safe observation and recovery re-entry. Its
    /// runtime-authority reference and digest identify the admitted issuer. A
    /// child handle reuses [`OperationLeaseV1::verify_against_parent_operation`]
    /// for the lease axes; contextual admission separately proves the compiled
    /// constitutional-path and provider ceilings. A signed record is not evidence
    /// of durable custody. The application must commit the exact request, handle,
    /// initial observation and checkpoint, then confirm the independent anchor
    /// before exposing the locator or allowing dispatch.
    OperationHandleV1, OperationHandleWireV1, OperationHandlePreimageV1,
    canonical_operation_handle_sha256: OperationHandleDigest,
    OPERATION_HANDLE_V1_SCHEMA, "operation handle",
    {
        /// Stable operation-handle identity.
        operation_handle_id: OperationHandleIdV1,
        /// Responsibility relationship established when the operation started.
        operation_class: OperationClassV1,
        /// Exact parent operation; absent only for a root operation.
        parent_operation: Option<ParentOperationReferenceV1>,
        /// Exact immutable request-admission lineage.
        request_binding: RequestBindingV1,
        /// Immutable reference to the admitted child operation lease.
        operation_lease_ref: ResidentRecordRefV1,
        /// Canonical identity of the admitted child operation lease.
        operation_lease_sha256: OperationLeaseDigest,
        /// Exact compiled generation used by the operation.
        compiled_generation_sha256: CompiledGenerationDigest,
        /// Exact activation fence crossed at durable start.
        activation_fence_sha256: ActivationFenceDigest,
        /// Exact private resolved instance selected at durable start.
        instance_sha256: InstanceDigest,
        /// Unix issue time of the signed locator record prepared for durable start.
        created_at: UnixTimeV1,
        /// Hard operation expiry.
        expires_at: UnixTimeV1,
        /// Last time at which the handle and observations must remain resolvable.
        retain_until: UnixTimeV1,
        /// Exact deployment-runtime authority under which this record was issued.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Pale principal whose admitted runtime issued the handle.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Exact issuer key.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_operation_handle_shape
);

resident_signed_record!(
    /// One revision in the monotone observation history of an operation handle.
    ///
    /// In the equation below, `r`, `t`, and `s` denote the revision,
    /// observation time, and state. `coord` pairs an observation's immutable
    /// record reference with its canonical digest; the successor must retain
    /// that exact pair as its predecessor. The state relation is the partial
    /// order owned by this module.
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/operation-observation-successor.display.mathml"
    ))]
    /// The runtime-authority reference and digest identify the admitted issuer
    /// of each revision.
    OperationObservationV1, OperationObservationWireV1, OperationObservationPreimageV1,
    canonical_operation_observation_sha256: OperationObservationDigest,
    OPERATION_OBSERVATION_V1_SCHEMA, "operation observation",
    {
        /// Stable handle whose operation is observed.
        operation_handle_id: OperationHandleIdV1,
        /// Immutable operation-handle record reference.
        operation_handle_ref: ResidentRecordRefV1,
        /// Canonical identity of the exact operation handle.
        operation_handle_sha256: OperationHandleDigest,
        /// Positive revision in this handle-local observation lineage.
        revision: OperationObservationRevisionV1,
        /// Previous observation reference; absent only at revision one.
        predecessor_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::predecessor_ref`].
        predecessor_sha256: Option<OperationObservationDigest>,
        /// Exact closed observation state and owner references.
        state: OperationObservationStateV1,
        /// Unix time at which this state was observed.
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
    validate_operation_observation_shape,
    wire_transform = operation_observation_schema_v1
);

/// Resolved parents required to admit one [`OperationHandleV1`].
#[derive(Clone, Copy)]
pub struct OperationHandleAdmissionContextV1<'a> {
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
    /// Exact request binding admitted before durable start.
    pub request_binding: &'a RequestBindingV1,
    /// Reference resolving [`Self::operation_lease`].
    pub operation_lease_ref: &'a ResidentRecordRefV1,
    /// Exact child lease admitted for this operation.
    pub operation_lease: &'a OperationLeaseV1,
    /// Exact resolved parent operation and lease, absent for a root operation.
    pub parent_operation: Option<ParentOperationAdmissionContextV1<'a>>,
    /// Exact compiled generation selected at start.
    pub compiled_generation_sha256: &'a CompiledGenerationDigest,
    /// Exact activation fence selected at start.
    pub activation_fence_sha256: &'a ActivationFenceDigest,
    /// Exact private instance selected at start.
    pub instance_sha256: &'a InstanceDigest,
}

/// Resolved parent handle and operation lease for child admission.
#[derive(Clone, Copy)]
pub struct ParentOperationAdmissionContextV1<'a> {
    /// Reference resolving [`Self::operation_handle`].
    pub operation_handle_ref: &'a ResidentRecordRefV1,
    /// Exact admitted parent operation handle.
    pub operation_handle: &'a OperationHandleV1,
    /// Exact operation lease bound by the parent handle.
    pub operation_lease: &'a OperationLeaseV1,
}

/// Resolved handle and predecessor required for one observation revision.
#[derive(Clone, Copy)]
pub struct OperationObservationAdmissionContextV1<'a> {
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
    /// Reference resolving [`Self::handle`].
    pub handle_ref: &'a ResidentRecordRefV1,
    /// Exact admitted durable operation handle.
    pub handle: &'a OperationHandleV1,
    /// Exact predecessor reference and record; absent only at revision one.
    pub predecessor: Option<(&'a ResidentRecordRefV1, &'a OperationObservationV1)>,
}

impl OperationObservationWireV1 {
    /// Borrow the untrusted handle reference used only for bounded recovery.
    #[must_use]
    pub const fn operation_handle_ref(&self) -> &ResidentRecordRefV1 {
        &self.operation_handle_ref
    }

    /// Borrow the untrusted handle digest used only for exact recovery lookup.
    #[must_use]
    pub const fn operation_handle_sha256(&self) -> &OperationHandleDigest {
        &self.operation_handle_sha256
    }

    /// Borrow the untrusted claimed revision for bounded history lookup, not admission.
    #[must_use]
    pub const fn revision(&self) -> &OperationObservationRevisionV1 {
        &self.revision
    }

    /// Borrow the untrusted claimed predecessor reference for exact parent lookup.
    #[must_use]
    pub const fn predecessor_ref(&self) -> &Option<ResidentRecordRefV1> {
        &self.predecessor_ref
    }

    /// Borrow the untrusted claimed predecessor digest; this proves no lineage.
    #[must_use]
    pub const fn predecessor_sha256(&self) -> &Option<OperationObservationDigest> {
        &self.predecessor_sha256
    }

    /// Borrow the untrusted runtime-authority reference for historical lookup.
    #[must_use]
    pub const fn deployment_runtime_authority_ref(&self) -> &ResidentRecordRefV1 {
        &self.deployment_runtime_authority_ref
    }

    /// Borrow the untrusted runtime-authority digest for historical lookup.
    #[must_use]
    pub const fn deployment_runtime_authority_sha256(&self) -> &ArtifactDigest {
        &self.deployment_runtime_authority_sha256
    }

    /// Borrow the untrusted claimed principal for historical signer lookup.
    #[must_use]
    pub const fn issuer_principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.issuer_principal_id
    }

    /// Borrow the untrusted claimed key for historical signer lookup.
    #[must_use]
    pub const fn issuer_key_id(&self) -> &ResidentKeyIdV1 {
        &self.issuer_key_id
    }

    /// Borrow the untrusted claimed key epoch for historical signer lookup.
    #[must_use]
    pub const fn issuer_key_epoch(&self) -> &SigningKeyEpochV1 {
        &self.issuer_key_epoch
    }

    /// Borrow the untrusted claimed issuance time; this is not a clock or authority proof.
    #[must_use]
    pub const fn observed_at(&self) -> &UnixTimeV1 {
        &self.observed_at
    }
}

impl OperationHandleWireV1 {
    /// Borrow the untrusted handle identity for bounded lookup, not admission.
    #[must_use]
    pub fn operation_handle_id(&self) -> &OperationHandleIdV1 {
        &self.operation_handle_id
    }

    /// Borrow the untrusted claimed class for early rejection, not admission.
    #[must_use]
    pub fn operation_class(&self) -> &OperationClassV1 {
        &self.operation_class
    }

    /// Borrow the untrusted claimed parent for lookup or early rejection.
    /// This does not establish parent linkage or admit the handle.
    #[must_use]
    pub fn parent_operation(&self) -> &Option<ParentOperationReferenceV1> {
        &self.parent_operation
    }

    /// Borrow the untrusted claimed lease reference for parent lookup.
    /// The returned reference establishes no custody or admission.
    #[must_use]
    pub fn operation_lease_ref(&self) -> &ResidentRecordRefV1 {
        &self.operation_lease_ref
    }

    /// Borrow the untrusted claimed lease digest for exact parent lookup.
    /// The returned digest establishes no custody or admission.
    #[must_use]
    pub fn operation_lease_sha256(&self) -> &OperationLeaseDigest {
        &self.operation_lease_sha256
    }
}

impl OperationHandleV1 {
    /// Sign a locator record from one owned preimage after contextual admission.
    ///
    /// `sign` is invoked only after structural, parent, lease,
    /// and runtime-authority validation succeeds. It receives the same domain,
    /// canonical preimage, and digest used by [`Self::admit_contextual_wire`].
    /// The returned signed record is prepared input to application custody, not
    /// permission to expose a durable operation handle or dispatch work.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] on admission or signing failure.
    pub fn issue_contextual<Authority, Sign>(
        preimage: OperationHandlePreimageV1,
        context: &OperationHandleAdmissionContextV1<'_>,
        admit_compiled_authority: Authority,
        sign: Sign,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Authority: FnOnce(
            &OperationHandlePreimageV1,
            Option<&OperationHandleV1>,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        validate_operation_handle_preimage(&preimage)?;
        validate_operation_handle_context(&preimage, context, admit_compiled_authority)?;
        let digest = OperationHandleDigest::hash_canonical(&preimage)?;
        let signature = sign(OPERATION_HANDLE_V1_SIGNATURE_DOMAIN, &preimage, &digest)?;
        Self::seal(preimage, signature)
    }

    /// Admit a signed locator record against its exact parents and signer.
    ///
    /// This validates record meaning, not storage. The application separately
    /// proves exact atomic start custody and independent checkpoint confirmation
    /// before exposing a locator, including during recovery.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for structural, digest, parent,
    /// lifetime, authority-coordinate, or signature failure.
    pub fn admit_contextual_wire<Authority, Signature>(
        wire: OperationHandleWireV1,
        context: &OperationHandleAdmissionContextV1<'_>,
        admit_compiled_authority: Authority,
        verify_signature: Signature,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Authority: FnOnce(
            &OperationHandlePreimageV1,
            Option<&OperationHandleV1>,
        ) -> Result<(), InvalidResidentRecordV1>,
        Signature: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_operation_handle_context(preimage, context, admit_compiled_authority)?;
            verify_signature(
                OPERATION_HANDLE_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

impl OperationObservationV1 {
    /// Recheck the pure handle-local successor law after contextual admission.
    ///
    /// This method verifies canonical record identity, the exact retained
    /// handle, predecessor lineage, chronology, terminal closure, and the state
    /// successor envelope. It deliberately does not repeat owner-resolved state-
    /// evidence or runtime-authority-rotation admission; callers must invoke it
    /// only for records that have already crossed [`Self::issue_contextual`] or
    /// [`Self::admit_contextual_wire`].
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the records do not form one
    /// canonical successor in the supplied handle-local lineage.
    pub fn verify_admitted_successor_of(
        &self,
        handle_ref: &ResidentRecordRefV1,
        handle: &OperationHandleV1,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &Self,
    ) -> Result<(), InvalidResidentRecordV1> {
        handle.verify()?;
        predecessor.verify()?;
        self.verify()?;
        if self.operation_handle_id() != handle.operation_handle_id()
            || self.operation_handle_ref() != handle_ref
            || self.operation_handle_sha256() != handle.canonical_operation_handle_sha256()
            || self.observed_at() < handle.created_at()
            || self.observed_at() > handle.retain_until()
        {
            return Err(contextual("operation observation", "exact retained handle"));
        }
        validate_operation_observation_predecessor(
            &operation_observation_preimage(self),
            Some((predecessor_ref, predecessor)),
        )
    }

    /// Issue an observation from one owned preimage after contextual admission.
    ///
    /// `sign` is invoked only after structural, handle, predecessor, state-
    /// evidence, and runtime-authority validation succeeds. When a successor
    /// changes authority coordinates, `admit_authority_rotation` must resolve
    /// an admitted succession in the same deployment/control-store lineage.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] on admission or signing failure.
    pub fn issue_contextual<Evidence, Authority, Sign>(
        preimage: OperationObservationPreimageV1,
        context: &OperationObservationAdmissionContextV1<'_>,
        admit_state: Evidence,
        admit_authority_rotation: Authority,
        sign: Sign,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Evidence: FnOnce(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
        Authority: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        validate_operation_observation_preimage(&preimage)?;
        validate_operation_observation_context(
            &preimage,
            context,
            admit_state,
            admit_authority_rotation,
        )?;
        let digest = OperationObservationDigest::hash_canonical(&preimage)?;
        let signature = sign(
            OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN,
            &preimage,
            &digest,
        )?;
        Self::seal(preimage, signature)
    }

    /// Admit one exact successor observation for a resolved operation handle.
    ///
    /// `admit_state` resolves evidence owned by the existing result,
    /// diagnostic, receipt, or reconciliation contracts. A changed runtime-
    /// authority coordinate is accepted only when `admit_authority_rotation`
    /// proves its admitted deployment/control-store lineage from the exact
    /// predecessor authority.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for structural, digest, handle,
    /// predecessor, terminality, chronology, evidence, signer, or signature failure.
    pub fn admit_contextual_wire<Evidence, Authority, Signature>(
        wire: OperationObservationWireV1,
        context: &OperationObservationAdmissionContextV1<'_>,
        admit_state: Evidence,
        admit_authority_rotation: Authority,
        verify_signature: Signature,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Evidence: FnOnce(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
        Authority: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Signature: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_operation_observation_context(
                preimage,
                context,
                admit_state,
                admit_authority_rotation,
            )?;
            verify_signature(
                OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

fn operation_observation_preimage(
    value: &OperationObservationV1,
) -> OperationObservationPreimageV1 {
    OperationObservationPreimageV1 {
        schema_id: value.schema_id().clone(),
        schema_sha256: value.schema_sha256().clone(),
        operation_handle_id: value.operation_handle_id().clone(),
        operation_handle_ref: value.operation_handle_ref().clone(),
        operation_handle_sha256: value.operation_handle_sha256().clone(),
        revision: *value.revision(),
        predecessor_ref: value.predecessor_ref().clone(),
        predecessor_sha256: value.predecessor_sha256().clone(),
        state: value.state().clone(),
        observed_at: *value.observed_at(),
        deployment_runtime_authority_ref: value.deployment_runtime_authority_ref().clone(),
        deployment_runtime_authority_sha256: value.deployment_runtime_authority_sha256().clone(),
        issuer_principal_id: value.issuer_principal_id().clone(),
        issuer_key_id: value.issuer_key_id().clone(),
        issuer_key_epoch: *value.issuer_key_epoch(),
    }
}

fn validate_operation_handle_preimage(
    preimage: &OperationHandlePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if preimage.schema_id.as_str() != OPERATION_HANDLE_V1_SCHEMA {
        return Err(InvalidResidentRecordV1::SchemaCoordinate {
            family: "operation handle",
        });
    }
    validate_operation_handle_shape(preimage)
}

fn validate_operation_observation_preimage(
    preimage: &OperationObservationPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if preimage.schema_id.as_str() != OPERATION_OBSERVATION_V1_SCHEMA {
        return Err(InvalidResidentRecordV1::SchemaCoordinate {
            family: "operation observation",
        });
    }
    validate_operation_observation_shape(preimage)
}

fn validate_operation_handle_shape(
    value: &OperationHandlePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    value.request_binding.verify()?;
    if value.created_at >= value.expires_at || value.expires_at > value.retain_until {
        return Err(contextual("operation handle", "ordered lifetime"));
    }
    Ok(())
}

fn validate_operation_handle_context<Authority>(
    preimage: &OperationHandlePreimageV1,
    context: &OperationHandleAdmissionContextV1<'_>,
    admit_compiled_authority: Authority,
) -> Result<(), InvalidResidentRecordV1>
where
    Authority: FnOnce(
        &OperationHandlePreimageV1,
        Option<&OperationHandleV1>,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    validate_signed_coordinates(
        "operation handle",
        &preimage.schema_sha256,
        context.signed.expected_schema_sha256,
        &preimage.issuer_principal_id,
        &preimage.issuer_key_id,
        preimage.issuer_key_epoch,
        context.signed.signing_authority,
    )?;
    validate_integrated_runtime_authority_coordinates(
        "operation handle",
        &preimage.deployment_runtime_authority_ref,
        &preimage.deployment_runtime_authority_sha256,
        context.signed.signing_authority,
    )?;
    if preimage.request_binding != *context.request_binding
        || preimage.operation_lease_ref != *context.operation_lease_ref
        || preimage.operation_lease_sha256 != *context.operation_lease.operation_lease_digest()
        || preimage.compiled_generation_sha256 != *context.compiled_generation_sha256
        || preimage.activation_fence_sha256 != *context.activation_fence_sha256
        || preimage.instance_sha256 != *context.instance_sha256
        || preimage.expires_at > context.operation_lease.expires_at()
    {
        return Err(contextual(
            "operation handle",
            "exact durable-start parents",
        ));
    }
    admit_parent_operation(preimage, context.parent_operation, context.operation_lease)?;
    admit_compiled_authority(
        preimage,
        context
            .parent_operation
            .map(|parent| parent.operation_handle),
    )
}

fn validate_operation_observation_context<Evidence, Authority>(
    preimage: &OperationObservationPreimageV1,
    context: &OperationObservationAdmissionContextV1<'_>,
    admit_state: Evidence,
    admit_authority_rotation: Authority,
) -> Result<(), InvalidResidentRecordV1>
where
    Evidence: FnOnce(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
    Authority: FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    validate_signed_coordinates(
        "operation observation",
        &preimage.schema_sha256,
        context.signed.expected_schema_sha256,
        &preimage.issuer_principal_id,
        &preimage.issuer_key_id,
        preimage.issuer_key_epoch,
        context.signed.signing_authority,
    )?;
    validate_integrated_runtime_authority_coordinates(
        "operation observation",
        &preimage.deployment_runtime_authority_ref,
        &preimage.deployment_runtime_authority_sha256,
        context.signed.signing_authority,
    )?;
    if preimage.operation_handle_id != *context.handle.operation_handle_id()
        || preimage.operation_handle_ref != *context.handle_ref
        || preimage.operation_handle_sha256 != *context.handle.canonical_operation_handle_sha256()
        || preimage.observed_at < *context.handle.created_at()
        || preimage.observed_at > *context.handle.retain_until()
    {
        return Err(contextual("operation observation", "exact retained handle"));
    }
    validate_operation_observation_predecessor(preimage, context.predecessor)?;
    admit_operation_observation_authority_rotation(
        preimage,
        context.predecessor,
        admit_authority_rotation,
    )?;
    admit_state(&preimage.state)
}

fn validate_operation_observation_shape(
    value: &OperationObservationPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    let has_predecessor = value.predecessor_ref.is_some() && value.predecessor_sha256.is_some();
    if value.predecessor_ref.is_some() != value.predecessor_sha256.is_some()
        || (value.revision.get() == 1) == has_predecessor
    {
        return Err(contextual(
            "operation observation",
            "revision-one predecessor boundary",
        ));
    }
    Ok(())
}

fn validate_operation_observation_predecessor(
    value: &OperationObservationPreimageV1,
    predecessor: Option<(&ResidentRecordRefV1, &OperationObservationV1)>,
) -> Result<(), InvalidResidentRecordV1> {
    match (value.revision.get(), predecessor) {
        (1, None) => Ok(()),
        (1, Some(_)) | (_, None) => Err(contextual(
            "operation observation",
            "resolved predecessor at every successor revision",
        )),
        (_, Some((predecessor_ref, predecessor))) => {
            if predecessor.state().is_terminal()
                || predecessor.operation_handle_id() != &value.operation_handle_id
                || predecessor.revision().checked_successor() != Some(value.revision)
                || value.predecessor_ref.as_ref() != Some(predecessor_ref)
                || value.predecessor_sha256.as_ref()
                    != Some(predecessor.canonical_operation_observation_sha256())
                || value.observed_at < *predecessor.observed_at()
                || !operation_state_phase_allows_successor(predecessor.state(), &value.state)
            {
                return Err(contextual(
                    "operation observation",
                    "exact monotone predecessor",
                ));
            }
            Ok(())
        }
    }
}

fn admit_operation_observation_authority_rotation<Authority>(
    value: &OperationObservationPreimageV1,
    predecessor: Option<(&ResidentRecordRefV1, &OperationObservationV1)>,
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
    match predecessor {
        None => Ok(()),
        Some((_, predecessor)) => admit_runtime_authority_rotation(
            "operation observation",
            predecessor.deployment_runtime_authority_ref(),
            predecessor.deployment_runtime_authority_sha256(),
            &value.deployment_runtime_authority_ref,
            &value.deployment_runtime_authority_sha256,
            admit_authority_rotation,
        ),
    }
}

/// State class used by the operation-observation successor envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationObservationPhaseV1 {
    Active,
    CancellationRequested,
    Recovering,
    Succeeded,
    Failed,
    Cancelled,
    OutcomeUnknown,
    Expired,
}

impl From<&OperationObservationStateV1> for OperationObservationPhaseV1 {
    fn from(state: &OperationObservationStateV1) -> Self {
        match state {
            OperationObservationStateV1::Active { .. } => Self::Active,
            OperationObservationStateV1::CancellationRequested { .. } => {
                Self::CancellationRequested
            }
            OperationObservationStateV1::Recovering { .. } => Self::Recovering,
            OperationObservationStateV1::Succeeded { .. } => Self::Succeeded,
            OperationObservationStateV1::Failed { .. } => Self::Failed,
            OperationObservationStateV1::Cancelled { .. } => Self::Cancelled,
            OperationObservationStateV1::OutcomeUnknown { .. } => Self::OutcomeUnknown,
            OperationObservationStateV1::Expired { .. } => Self::Expired,
        }
    }
}

/// Coarse operation-state envelope used by successor admission.
///
/// Active phases precede cancellation-requested, execution expiry, recovery,
/// an unknown outcome, and finally the incomparable conclusive classes.
/// Permitted skips do not permit a return to execution, recovery after unknown,
/// or expiry that would hide existing recovery or uncertainty. Reconciliation
/// can continue while the observation remains unknown; a conclusive successor
/// must pass the existing owner-evidence admission before issuance.
///
/// This relation limits possible operational successors; it does not prove the
/// truth or order of arbitrary evidence. The complete payload must remain equal
/// at the same phase, except while a consultation remains open. Each admitted
/// child then advances the private collection frontier without changing the
/// public active phase. The application owner proves that exact frontier
/// succession before it asks the core to admit the observation successor.
///
/// Exact revision and predecessor coordinates preserve the observation
/// history. Contextual admission separately closes conclusive lineages, so
/// terminal reflexivity does not authorize another observation.
fn operation_state_phase_allows_successor(
    predecessor: &OperationObservationStateV1,
    successor: &OperationObservationStateV1,
) -> bool {
    use OperationObservationPhaseV1::{
        Active, CancellationRequested, Expired, OutcomeUnknown, Recovering,
    };
    let predecessor_class = OperationObservationPhaseV1::from(predecessor);
    let successor_class = OperationObservationPhaseV1::from(successor);
    match (predecessor_class, successor_class) {
        (Active, Active) => match (predecessor, successor) {
            (
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::CollectingConsultation { .. },
                },
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::CollectingConsultation { .. },
                },
            ) => true,
            (
                OperationObservationStateV1::Active { phase: predecessor },
                OperationObservationStateV1::Active { phase: successor },
            ) => predecessor == successor || predecessor.rank() < successor.rank(),
            _ => false,
        },
        (left, right) if left == right => predecessor == successor,
        (Active, _) => true,
        (CancellationRequested, right) => right != Active,
        (Expired, right) => !matches!(right, Active | CancellationRequested),
        (Recovering, right) => !matches!(right, Active | CancellationRequested | Expired),
        (OutcomeUnknown, right) => {
            !matches!(right, Active | CancellationRequested | Expired | Recovering)
        }
        _ => false,
    }
}

fn admit_parent_operation(
    value: &OperationHandlePreimageV1,
    parent: Option<ParentOperationAdmissionContextV1<'_>>,
    child_lease: &OperationLeaseV1,
) -> Result<(), InvalidResidentRecordV1> {
    match (&value.parent_operation, parent) {
        (None, None) => Ok(()),
        (Some(claimed), Some(parent)) => {
            parent.operation_handle.verify()?;
            if claimed.operation_handle_id != *parent.operation_handle.operation_handle_id()
                || claimed.operation_handle_ref != *parent.operation_handle_ref
                || claimed.operation_handle_sha256
                    != *parent.operation_handle.canonical_operation_handle_sha256()
                || parent.operation_handle.operation_lease_sha256()
                    != parent.operation_lease.operation_lease_digest()
            {
                return Err(contextual(
                    "operation handle",
                    "exact parent operation coordinate",
                ));
            }
            child_lease.verify_against_parent_operation(parent.operation_lease)
        }
        _ => Err(contextual(
            "operation handle",
            "resolved optional parent operation",
        )),
    }
}

fn operation_observation_schema_v1(schema: &mut schemars::Schema) {
    require_all_or_none_properties_v1(schema, &["predecessor_ref", "predecessor_sha256"]);
}

fn contextual(record: &'static str, invariant: &'static str) -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission { record, invariant }
}

#[cfg(test)]
#[path = "../../../tests/internal/records/resident/operation.rs"]
mod tests;
