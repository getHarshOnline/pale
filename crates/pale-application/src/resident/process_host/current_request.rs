//! Current authority and retained policy for one resident request.
//!
//! This private seam keeps two inputs distinct. The evidence port returns
//! already admitted owner records at exact session coordinates. The retained
//! policy projection supplies the request values selected by compiled policy.
//! Neither input admits a request, authenticates a peer, reads a clock, or
//! authorizes an operation by itself.

use pale_core::records::*;

use crate::resident::{
    AdmittedAmbientSession, CapabilityGrantAuthority, CapabilityGrantRevocationHead,
    CapabilityRevocationFrontier, DeploymentAcceptanceRevocationFrontier,
    DeploymentAcceptanceRevocationHead, SessionRevocationFrontier,
};

/// Exact session coordinates used to read current authority after peer admission.
///
/// `PeerWitness` is produced by the local transport's peer-admission owner. The
/// remaining fields are selectors only until the returned records are joined to
/// an admitted session.
pub(super) struct ResidentCurrentRequestSelector<'a, PeerWitness> {
    pub(super) peer_witness: &'a PeerWitness,
    pub(super) session_admission_id: &'a SessionAdmissionIdV1,
    pub(super) session_admission_lease_digest: &'a SessionAdmissionLeaseDigest,
    pub(super) session_grant_id: &'a SessionGrantIdV1,
    pub(super) deployment_binding_ref: &'a ResidentRecordRefV1,
    pub(super) deployment_binding_sha256: &'a ArtifactDigest,
}

/// An owned capability grant paired with the trust checkpoint used at issuance.
#[derive(Debug, Clone)]
pub(super) struct ResidentCurrentGrantEvidence {
    pub(super) grant_ref: ResidentRecordRefV1,
    pub(super) grant: CapabilitySessionGrantV1,
    pub(super) issued_trust_checkpoint_ref: ResidentRecordRefV1,
    pub(super) issued_trust_checkpoint: PrincipalTrustCheckpointV1,
}

/// One current grant-specific revocation head.
#[derive(Debug, Clone)]
pub(super) struct ResidentCurrentGrantRevocationEvidence {
    pub(super) revocation_ref: ResidentRecordRefV1,
    pub(super) revocation: CapabilitySessionGrantRevocationV1,
}

/// The current capability-revocation checkpoint and selected grant head.
#[derive(Debug, Clone)]
pub(super) struct ResidentCurrentCapabilityRevocationEvidence {
    pub(super) checkpoint_ref: ResidentRecordRefV1,
    pub(super) checkpoint: CapabilityRevocationCheckpointV1,
    pub(super) grant_head: Option<ResidentCurrentGrantRevocationEvidence>,
}

/// One current deployment-acceptance revocation head.
#[derive(Debug, Clone)]
pub(super) struct ResidentCurrentDeploymentRevocationHeadEvidence {
    pub(super) revocation_ref: ResidentRecordRefV1,
    pub(super) revocation: DeploymentAcceptanceRevocationV1,
}

/// The current deployment-acceptance revocation domain.
#[derive(Debug, Clone)]
pub(super) struct ResidentCurrentDeploymentRevocationEvidence {
    pub(super) epoch: DeploymentAcceptanceRevocationEpochV1,
    pub(super) binding_head: Option<ResidentCurrentDeploymentRevocationHeadEvidence>,
}

/// The current principal-scoped session-revocation frontier.
#[derive(Debug, Clone)]
pub(super) struct ResidentCurrentSessionRevocationEvidence {
    pub(super) epoch: PrincipalRevocationEpochV1,
    pub(super) revocation_ref: Option<ResidentRecordRefV1>,
    pub(super) revocation_sha256: Option<ArtifactDigest>,
}

/// Raw current authority records resolved for one authenticated peer and session.
///
/// The value contains no authorization Boolean and no assembled admission
/// context. Its records must still be joined to the retained session,
/// deployment authority, generation, route, subject, and physical time.
#[derive(Debug, Clone)]
pub(super) struct ResidentCurrentRequestEvidence {
    pub(super) grant: ResidentCurrentGrantEvidence,
    pub(super) capability_revocations: ResidentCurrentCapabilityRevocationEvidence,
    pub(super) deployment_revocations: ResidentCurrentDeploymentRevocationEvidence,
    pub(super) session_revocations: ResidentCurrentSessionRevocationEvidence,
}

/// Which exact evidence relation disagreed with the admitted session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResidentCurrentRequestEvidenceMismatch {
    Grant,
    IssuedTrust,
    CapabilityCheckpoint,
    GrantRevocation,
    DeploymentRevocation,
    SessionRevocation,
}

/// Why current request evidence could not be supplied or joined.
#[derive(Debug, thiserror::Error)]
pub(super) enum ResidentCurrentRequestEvidenceError {
    /// The physical owner could not read a complete current frontier.
    #[error("current request authority evidence is unavailable")]
    Unavailable,
    /// One already admitted record no longer reproduces its canonical identity.
    #[error("current request authority record is invalid")]
    Record(#[from] InvalidResidentRecordV1),
    /// A typed digest could not be projected into the generic artifact domain.
    #[error("current request authority digest has an invalid representation")]
    Digest(#[from] InvalidDigest),
    /// Current records do not belong to the selected admitted session.
    #[error("current request authority evidence does not match the admitted session")]
    Mismatch(ResidentCurrentRequestEvidenceMismatch),
}

/// Physical owner of current authority records for an admitted peer.
///
/// The port may use the authenticated peer witness and exact session selectors
/// to locate records. It returns records, not an authorization decision or an
/// assembled [`crate::resident::AmbientSessionAdmissionContext`].
pub(super) trait ResidentCurrentRequestEvidencePort<PeerWitness> {
    fn read_current_request_evidence(
        &mut self,
        selector: ResidentCurrentRequestSelector<'_, PeerWitness>,
    ) -> Result<ResidentCurrentRequestEvidence, ResidentCurrentRequestEvidenceError>;
}

/// Evidence-derived session coordinate for common-request admission.
///
/// Construction is private to [`ResidentCurrentRequestEvidence`], so a caller
/// cannot replace it with fields copied from an untrusted request.
pub(super) struct ResidentCurrentRequestSessionAdmission(CommonRequestSessionAdmissionV1);

impl ResidentCurrentRequestSessionAdmission {
    pub(super) const fn as_record(&self) -> &CommonRequestSessionAdmissionV1 {
        &self.0
    }
}

impl ResidentCurrentRequestEvidence {
    /// Borrow the exact grant and issuance checkpoint for ambient revalidation.
    pub(super) const fn grant_authority(&self) -> CapabilityGrantAuthority<'_> {
        CapabilityGrantAuthority {
            grant_ref: &self.grant.grant_ref,
            grant: &self.grant.grant,
            issued_trust_checkpoint_ref: &self.grant.issued_trust_checkpoint_ref,
            issued_trust_checkpoint: &self.grant.issued_trust_checkpoint,
        }
    }

    /// Borrow the current capability-revocation frontier.
    pub(super) fn capability_revocation_frontier(&self) -> CapabilityRevocationFrontier<'_> {
        CapabilityRevocationFrontier {
            checkpoint_ref: &self.capability_revocations.checkpoint_ref,
            checkpoint: &self.capability_revocations.checkpoint,
            grant_head: self.capability_revocations.grant_head.as_ref().map(|head| {
                CapabilityGrantRevocationHead {
                    revocation_ref: &head.revocation_ref,
                    revocation: &head.revocation,
                }
            }),
        }
    }

    /// Borrow the current deployment-acceptance revocation frontier.
    pub(super) fn deployment_revocation_frontier(
        &self,
    ) -> DeploymentAcceptanceRevocationFrontier<'_> {
        DeploymentAcceptanceRevocationFrontier {
            epoch: self.deployment_revocations.epoch,
            binding_head: self
                .deployment_revocations
                .binding_head
                .as_ref()
                .map(|head| DeploymentAcceptanceRevocationHead {
                    revocation_ref: &head.revocation_ref,
                    revocation: &head.revocation,
                }),
        }
    }

    /// Borrow the current session-revocation frontier.
    pub(super) const fn session_revocation_frontier(&self) -> SessionRevocationFrontier<'_> {
        SessionRevocationFrontier {
            epoch: self.session_revocations.epoch,
            revocation_ref: self.session_revocations.revocation_ref.as_ref(),
            revocation_sha256: self.session_revocations.revocation_sha256.as_ref(),
        }
    }

    /// Join current authority evidence to an already admitted, revalidated session.
    ///
    /// `checked_at` is supplied separately by the physical clock owner. It is
    /// not retained policy and this method does not establish clock trust.
    pub(super) fn session_admission(
        &self,
        session: &AdmittedAmbientSession,
        checked_at: UnixTimeV1,
    ) -> Result<ResidentCurrentRequestSessionAdmission, ResidentCurrentRequestEvidenceError> {
        self.validate_session_association(session)?;
        let lease = session.lease();
        let grant = &self.grant.grant;
        Ok(ResidentCurrentRequestSessionAdmission(
            CommonRequestSessionAdmissionV1 {
                session_admission_id: lease.session_admission_id().clone(),
                session_admission_lease_digest: lease.session_admission_lease_digest().clone(),
                session_grant_id: session.grant_id().clone(),
                session_grant_sha256: ArtifactDigest::parse(session.grant_sha256().as_str())?,
                deployment_binding_ref: lease.deployment_binding_ref().clone(),
                deployment_binding_sha256: lease.deployment_binding_sha256().clone(),
                session_floor_epoch: *grant.session_floor_epoch(),
                grant_issuer_principal_id: grant.issuer_principal_id().clone(),
                grant_issuer_key_id: grant.issuer_key_id().clone(),
                grant_issuer_key_epoch: PrincipalKeyEpochV1::new(grant.issuer_key_epoch().get())
                    .map_err(|_| {
                        ResidentCurrentRequestEvidenceError::Mismatch(
                            ResidentCurrentRequestEvidenceMismatch::Grant,
                        )
                    })?,
                checked_revocation_epoch: self.session_revocations.epoch,
                checked_revocation_ref: self.session_revocations.revocation_ref.clone(),
                checked_revocation_sha256: self.session_revocations.revocation_sha256.clone(),
                checked_revocation_checkpoint_ref: self
                    .capability_revocations
                    .checkpoint_ref
                    .clone(),
                checked_revocation_checkpoint_sha256: ArtifactDigest::parse(
                    self.capability_revocations
                        .checkpoint
                        .canonical_checkpoint_sha256()
                        .as_str(),
                )?,
                checked_at,
            },
        ))
    }

    fn validate_session_association(
        &self,
        session: &AdmittedAmbientSession,
    ) -> Result<(), ResidentCurrentRequestEvidenceError> {
        let lease = session.lease();
        let grant = &self.grant.grant;
        grant.verify()?;
        self.grant.issued_trust_checkpoint.verify()?;
        self.capability_revocations.checkpoint.verify()?;

        if self.grant.grant_ref != *session.grant_ref()
            || grant.session_grant_id() != session.grant_id()
            || grant.canonical_grant_sha256() != session.grant_sha256()
            || grant.deployment_binding_ref() != lease.deployment_binding_ref()
            || grant.deployment_binding_sha256().as_str()
                != lease.deployment_binding_sha256().as_str()
        {
            return Err(ResidentCurrentRequestEvidenceError::Mismatch(
                ResidentCurrentRequestEvidenceMismatch::Grant,
            ));
        }
        if grant.checked_principal_trust_checkpoint_ref() != &self.grant.issued_trust_checkpoint_ref
            || grant.checked_principal_trust_checkpoint_sha256()
                != self
                    .grant
                    .issued_trust_checkpoint
                    .canonical_trust_checkpoint_sha256()
            || grant.issuer_principal_id() != self.grant.issued_trust_checkpoint.principal_id()
        {
            return Err(ResidentCurrentRequestEvidenceError::Mismatch(
                ResidentCurrentRequestEvidenceMismatch::IssuedTrust,
            ));
        }
        if self.capability_revocations.checkpoint_ref != *session.capability_checkpoint_ref()
            || self
                .capability_revocations
                .checkpoint
                .canonical_checkpoint_sha256()
                != session.capability_checkpoint_sha256()
            || self
                .capability_revocations
                .checkpoint
                .authority_principal_id()
                != grant.issuer_principal_id()
        {
            return Err(ResidentCurrentRequestEvidenceError::Mismatch(
                ResidentCurrentRequestEvidenceMismatch::CapabilityCheckpoint,
            ));
        }
        if let Some(head) = self.capability_revocations.grant_head.as_ref() {
            head.revocation.verify()?;
            if head.revocation.grant_ref() != &self.grant.grant_ref
                || head.revocation.grant_sha256() != grant.canonical_grant_sha256()
                || head.revocation.grant_id() != grant.session_grant_id()
            {
                return Err(ResidentCurrentRequestEvidenceError::Mismatch(
                    ResidentCurrentRequestEvidenceMismatch::GrantRevocation,
                ));
            }
        }
        if self.deployment_revocations.epoch
            != *grant.checked_deployment_acceptance_revocation_epoch()
        {
            return Err(ResidentCurrentRequestEvidenceError::Mismatch(
                ResidentCurrentRequestEvidenceMismatch::DeploymentRevocation,
            ));
        }
        if let Some(head) = self.deployment_revocations.binding_head.as_ref() {
            head.revocation.verify()?;
            if head.revocation.revocation_epoch() != &self.deployment_revocations.epoch {
                return Err(ResidentCurrentRequestEvidenceError::Mismatch(
                    ResidentCurrentRequestEvidenceMismatch::DeploymentRevocation,
                ));
            }
        }
        if self.session_revocations.revocation_ref.is_some()
            != self.session_revocations.revocation_sha256.is_some()
            || lease.revocation_epoch() != &self.session_revocations.epoch
            || lease.checked_revocation_ref().as_ref()
                != self.session_revocations.revocation_ref.as_ref()
            || lease.checked_revocation_sha256().as_ref()
                != self.session_revocations.revocation_sha256.as_ref()
        {
            return Err(ResidentCurrentRequestEvidenceError::Mismatch(
                ResidentCurrentRequestEvidenceMismatch::SessionRevocation,
            ));
        }
        Ok(())
    }
}

/// One execution fence retained by compiled request policy.
#[derive(Debug, Clone)]
pub(in crate::resident) struct ResidentRetainedRequestExecutionFence {
    pub(in crate::resident) fence_ref: ResidentRecordRefV1,
    pub(in crate::resident) fence: ExecutionOwnerFenceV1,
}

/// Exact common-request values selected by retained compiled policy.
///
/// This is not a second request DTO. The policy owner constructs it from the
/// admitted generation and policy projection. Request admission then compares
/// caller-signed bytes against these values; it never fills this projection
/// from the untrusted request.
#[derive(Debug, Clone)]
pub(in crate::resident) struct ResidentRetainedCommonRequestPolicy {
    pub(in crate::resident) expected_schema_sha256: SchemaDigest,
    pub(in crate::resident) expected_body_schema_id: SchemaId,
    pub(in crate::resident) expected_body_schema_sha256: SchemaDigest,
    pub(in crate::resident) caller: RequestCallerV1,
    pub(in crate::resident) signing_authority: SigningAuthorityBindingV1,
    pub(in crate::resident) route: CommonRequestRouteV1,
    pub(in crate::resident) operation_target: Option<OperationTargetReferenceV1>,
    pub(in crate::resident) execution_owner_fence: Option<ResidentRetainedRequestExecutionFence>,
    pub(in crate::resident) federation_claim: Option<CommonRequestFederationClaimV1>,
    pub(in crate::resident) workspace_id: ResidentWorkspaceIdV1,
    pub(in crate::resident) capability_id: CapabilityIdV1,
    pub(in crate::resident) operation: PackageResourceRefV1,
    pub(in crate::resident) scope: CommonRequestScopeV1,
    pub(in crate::resident) risk_class: BoundedResidentTextV1,
    pub(in crate::resident) resource_constraints: Vec<BoundedResidentTextV1>,
    pub(in crate::resident) expected_precondition: Option<ExpectedPreconditionV1>,
    /// Maximum caller-selected request lifetime accepted by this policy.
    pub(in crate::resident) maximum_request_lifetime_seconds: u64,
    pub(in crate::resident) generation: Option<CompiledGenerationDigest>,
    pub(in crate::resident) fence_token: Option<ActivationFenceDigest>,
}

/// Caller-selected request coordinates admitted under retained lifetime policy.
///
/// Construction is private so copying request fields cannot stand in for the
/// application-owned time check. The idempotency key remains caller-selected;
/// this witness proves only that the complete request window is currently
/// usable under the retained ceiling.
pub(super) struct ResidentAdmittedRequestWindow {
    idempotency_key: RequestIdempotencyKeyV1,
    issued_at: UnixTimeV1,
    expires_at: UnixTimeV1,
}

impl ResidentRetainedCommonRequestPolicy {
    /// Derive one point-addressed request policy without changing its retained base.
    ///
    /// The operation target is a request-local signed coordinate. It is not
    /// inserted into the compiled policy retained for later requests.
    pub(super) fn with_operation_target(&self, target: OperationTargetReferenceV1) -> Self {
        let mut derived = self.clone();
        derived.operation_target = Some(target);
        derived
    }

    /// Return whether two policies can admit the same fixed signed request.
    ///
    /// Caller-selected identities, time, payload bytes, and current session
    /// evidence are intentionally excluded. Every field selected by retained
    /// policy and compared during contextual admission remains part of this
    /// domain.
    pub(super) fn overlaps_fixed_admission_domain(&self, other: &Self) -> bool {
        self.expected_schema_sha256 == other.expected_schema_sha256
            && self.expected_body_schema_id == other.expected_body_schema_id
            && self.expected_body_schema_sha256 == other.expected_body_schema_sha256
            && self.caller == other.caller
            && self.signing_authority == other.signing_authority
            && self.route == other.route
            && self.operation_target == other.operation_target
            && project_execution_fence(self.execution_owner_fence.as_ref())
                == project_execution_fence(other.execution_owner_fence.as_ref())
            && self.federation_claim == other.federation_claim
            && self.workspace_id == other.workspace_id
            && self.capability_id == other.capability_id
            && self.operation == other.operation
            && self.scope == other.scope
            && self.risk_class == other.risk_class
            && self.resource_constraints == other.resource_constraints
            && self.expected_precondition == other.expected_precondition
            && self.generation == other.generation
            && self.fence_token == other.fence_token
    }

    /// Return whether one untrusted preimage claims this fixed policy domain.
    pub(super) fn matches_fixed_claim(&self, claimed: &CommonRequestEnvelopePreimageV1) -> bool {
        self.expected_schema_sha256 == claimed.envelope_schema_sha256
            && self.expected_body_schema_id == claimed.body.schema_id
            && self.expected_body_schema_sha256 == claimed.body.schema_sha256
            && self.caller == claimed.caller
            && self.signing_authority == claimed.signing_authority_binding
            && self.route == claimed.route
            && self.operation_target == claimed.operation_target
            && project_execution_fence(self.execution_owner_fence.as_ref())
                == claimed.execution_owner_fence
            && self.federation_claim == claimed.federation_claim
            && self.workspace_id == claimed.workspace_id
            && self.capability_id == claimed.capability_id
            && self.operation == claimed.operation
            && self.scope == claimed.scope
            && self.risk_class == claimed.risk_class
            && self.resource_constraints == claimed.resource_constraints
            && self.expected_precondition == claimed.expected_precondition
            && self.generation == claimed.generation
            && self.fence_token == claimed.fence_token
    }

    /// Admit caller-selected request coordinates under the retained time ceiling.
    pub(super) fn admit_request_window(
        &self,
        request: &CommonRequestEnvelopePreimageV1,
        now: UnixTimeV1,
    ) -> Result<ResidentAdmittedRequestWindow, InvalidResidentRecordV1> {
        let issued = request.issued_at.get();
        let expires = request.expires_at.get();
        if self.maximum_request_lifetime_seconds == 0
            || issued > now.get()
            || now.get() > expires
            || expires < issued
            || expires - issued > self.maximum_request_lifetime_seconds
        {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "common request",
                invariant: "request lifetime is outside retained policy",
            });
        }
        Ok(ResidentAdmittedRequestWindow {
            idempotency_key: request.idempotency_key.clone(),
            issued_at: request.issued_at,
            expires_at: request.expires_at,
        })
    }

    /// Borrow a complete common-request admission context.
    ///
    /// Every policy member comes from this retained projection. The only
    /// session member comes from current authority evidence joined to an
    /// admitted session. No untrusted request field is copied here.
    pub(super) fn admission_context<'a>(
        &'a self,
        session: &'a ResidentCurrentRequestSessionAdmission,
        window: &'a ResidentAdmittedRequestWindow,
    ) -> CommonRequestAdmissionContextV1<'a> {
        CommonRequestAdmissionContextV1 {
            expected_schema_sha256: &self.expected_schema_sha256,
            expected_body_schema_id: &self.expected_body_schema_id,
            expected_body_schema_sha256: &self.expected_body_schema_sha256,
            caller: &self.caller,
            signing_authority: &self.signing_authority,
            route: &self.route,
            session_admission: session.as_record(),
            operation_target: self.operation_target.as_ref(),
            execution_owner_fence: self.execution_owner_fence.as_ref().map(|fence| {
                CommonRequestExecutionOwnerFenceAdmissionV1::new(&fence.fence_ref, &fence.fence)
            }),
            federation_claim: self.federation_claim.as_ref(),
            workspace_id: &self.workspace_id,
            capability_id: &self.capability_id,
            operation: &self.operation,
            scope: &self.scope,
            risk_class: &self.risk_class,
            resource_constraints: &self.resource_constraints,
            expected_precondition: self.expected_precondition.as_ref(),
            idempotency_key: &window.idempotency_key,
            issued_at: window.issued_at,
            expires_at: window.expires_at,
            generation: self.generation.as_ref(),
            fence_token: self.fence_token.as_ref(),
        }
    }
}

fn project_execution_fence(
    retained: Option<&ResidentRetainedRequestExecutionFence>,
) -> Option<CommonRequestExecutionOwnerFenceV1> {
    retained.map(|value| {
        CommonRequestExecutionOwnerFenceV1::from_admitted(&value.fence_ref, &value.fence)
    })
}
