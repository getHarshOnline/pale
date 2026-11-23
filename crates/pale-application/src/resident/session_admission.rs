//! Ambient-session admission and nested operation-lease policy.
//!
//! Core records own their wire shape, canonical identity, and local parent or
//! predecessor laws. This module owns the application join across an already
//! admitted capability grant, its current revocation frontiers, the active
//! deployment runtime authority, and one active compiled generation. It neither
//! resolves those current heads from storage. The signed
//! [`SessionAdmissionLeaseV1`] owns its deployment-runtime authority; private
//! application custody retains only the other revalidation coordinates.

use pale_core::records::{
    ArtifactDigest, CapabilityRevocationCheckpointV1, CapabilitySessionGrantDigest,
    CapabilitySessionGrantRevocationV1, CapabilitySessionGrantV1, CapabilitySessionSubjectV1,
    DeploymentAcceptanceRevocationEpochV1, DeploymentAcceptanceRevocationV1, DeploymentBindingV1,
    DeploymentRuntimeAuthorityV1, InvalidResidentRecordV1, LocalTransportClassV1,
    OperationLeaseRequestV1, OperationLeaseV1, PrincipalRevocationEpochV1,
    PrincipalTrustCheckpointV1, ResidentNodeIdV1, ResidentPrincipalIdV1, ResidentRecordRefV1,
    SessionAdmissionLeaseContextV1, SessionAdmissionLeaseV1, SessionAdmissionLeaseWireV1,
    SessionContractSetConstraintV1, SessionGrantIdV1, SessionRouteConstraintV1, UnixTimeV1,
};
pub use pale_ports::{AmbientSessionAuthorityError, AmbientSessionAuthorityPort};

mod authority;
mod basis;
mod custody;
mod scope;
mod time;

pub(in crate::resident) use basis::{AmbientSessionBasis, RetainedAmbientSession};
pub(in crate::resident) use custody::{
    decode as decode_admission_basis, encode as encode_admission_basis,
    SessionAdmissionBasisCustodyError,
};

use authority::validate_authority;
use scope::classify_generation;

/// Exact admitted grant and the trust checkpoint against which it was issued.
#[derive(Clone, Copy)]
pub struct CapabilityGrantAuthority<'a> {
    /// Immutable grant reference assigned by the control store.
    pub grant_ref: &'a ResidentRecordRefV1,
    /// Already signature-admitted grant record.
    pub grant: &'a CapabilitySessionGrantV1,
    /// Immutable reference named by [`CapabilitySessionGrantV1::checked_principal_trust_checkpoint_ref`].
    pub issued_trust_checkpoint_ref: &'a ResidentRecordRefV1,
    /// Already admitted trust checkpoint resolved at grant issuance.
    pub issued_trust_checkpoint: &'a PrincipalTrustCheckpointV1,
}

/// Current principal trust-checkpoint head selected by durable custody.
#[derive(Clone, Copy)]
pub struct CurrentPrincipalTrustHead<'a> {
    /// Immutable current checkpoint reference.
    pub checkpoint_ref: &'a ResidentRecordRefV1,
    /// Already admitted current checkpoint record.
    pub checkpoint: &'a PrincipalTrustCheckpointV1,
}

/// Current append-only revocation head for one capability grant.
#[derive(Clone, Copy)]
pub struct CapabilityGrantRevocationHead<'a> {
    /// Immutable current revocation reference.
    pub revocation_ref: &'a ResidentRecordRefV1,
    /// Already signature-admitted current revocation record.
    pub revocation: &'a CapabilitySessionGrantRevocationV1,
}

/// Current signed capability-revocation checkpoint and the selected grant head.
#[derive(Clone, Copy)]
pub struct CapabilityRevocationFrontier<'a> {
    /// Immutable current checkpoint reference.
    pub checkpoint_ref: &'a ResidentRecordRefV1,
    /// Already signature-admitted current checkpoint.
    pub checkpoint: &'a CapabilityRevocationCheckpointV1,
    /// Current revocation head for this grant, when one exists.
    pub grant_head: Option<CapabilityGrantRevocationHead<'a>>,
}

/// Current deployment-acceptance revocation head, when one exists.
#[derive(Clone, Copy)]
pub struct DeploymentAcceptanceRevocationHead<'a> {
    /// Immutable current revocation reference.
    pub revocation_ref: &'a ResidentRecordRefV1,
    /// Already owner-admitted current revocation record.
    pub revocation: &'a DeploymentAcceptanceRevocationV1,
}

/// Current deployment-acceptance revocation domain for the selected binding.
#[derive(Clone, Copy)]
pub struct DeploymentAcceptanceRevocationFrontier<'a> {
    /// Current monotone domain epoch checked for new session admission.
    pub epoch: DeploymentAcceptanceRevocationEpochV1,
    /// Current binding-specific head, when the acceptance has a revocation lineage.
    pub binding_head: Option<DeploymentAcceptanceRevocationHead<'a>>,
}

/// Active deployment-runtime-authority head selected from the trust frontier.
#[derive(Clone, Copy)]
pub struct DeploymentRuntimeAuthorityHead<'a> {
    /// Immutable current authority reference.
    pub authority_ref: &'a ResidentRecordRefV1,
    /// Already principal-admitted runtime authority.
    pub authority: &'a DeploymentRuntimeAuthorityV1,
}

/// Current principal-scoped session-revocation frontier carried by the lease.
#[derive(Clone, Copy)]
pub struct SessionRevocationFrontier<'a> {
    /// Current positive principal revocation epoch.
    pub epoch: PrincipalRevocationEpochV1,
    /// Current immutable revocation record, when one exists.
    pub revocation_ref: Option<&'a ResidentRecordRefV1>,
    /// Digest paired with [`Self::revocation_ref`].
    pub revocation_sha256: Option<&'a ArtifactDigest>,
}

/// One exact route use selected beneath a capability grant.
#[derive(Clone, Copy)]
pub struct AmbientSessionRouteUse<'a> {
    /// Immutable route generation selected for this session.
    pub route: &'a SessionRouteConstraintV1,
    /// Origin node that must equal the grant's exact route origin.
    pub origin_node_id: &'a ResidentNodeIdV1,
    /// Transport selected from the grant's allowlist.
    pub transport_class: LocalTransportClassV1,
    /// Destination principal selected from the grant's allowlist.
    pub destination_principal_id: &'a ResidentPrincipalIdV1,
    /// Optional destination node selected from the grant's allowlist.
    pub destination_node_id: Option<&'a ResidentNodeIdV1>,
    /// Immutable contract set selected from the grant's allowlist.
    pub contract_set: &'a SessionContractSetConstraintV1,
}

/// Complete resolved input for one pure ambient-session admission decision.
///
/// Optional authority members represent genuine absence so the result can
/// distinguish it from expiry, revocation, or stale evidence. Every borrowed
/// record must already have passed its owner-specific signature admission;
/// this application seam rechecks canonical identity and relationships but is
/// not a second signature verifier.
pub struct AmbientSessionAdmissionContext<'a> {
    /// Subsystem-supplied physical Unix time for this admission decision.
    ///
    /// This value can expose contradictions with admitted record times; it does
    /// not establish that the underlying clock is trustworthy.
    pub now: UnixTimeV1,
    /// Server-resolved caller subject; caller-declared identity is insufficient.
    pub subject: &'a CapabilitySessionSubjectV1,
    /// Exact capability grant, or explicit absence.
    pub grant: Option<CapabilityGrantAuthority<'a>>,
    /// Current principal trust checkpoint, or explicit absence.
    pub current_principal_trust: Option<CurrentPrincipalTrustHead<'a>>,
    /// Current capability-revocation frontier, or explicit absence.
    pub capability_revocations: Option<CapabilityRevocationFrontier<'a>>,
    /// Exact admitted deployment binding reference.
    pub deployment_binding_ref: &'a ResidentRecordRefV1,
    /// Exact admitted deployment binding.
    pub deployment_binding: &'a DeploymentBindingV1,
    /// Current deployment-acceptance revocation frontier.
    pub deployment_revocations: &'a DeploymentAcceptanceRevocationFrontier<'a>,
    /// Current deployment runtime authority, or explicit absence.
    pub runtime_authority: Option<DeploymentRuntimeAuthorityHead<'a>>,
    /// Current principal-scoped session-revocation frontier.
    pub session_revocations: &'a SessionRevocationFrontier<'a>,
    /// Exact route use selected beneath the grant.
    pub route_use: &'a AmbientSessionRouteUse<'a>,
    /// Exact active generation, profile, and activation-fence join.
    pub generation: SessionAdmissionLeaseContextV1<'a>,
}

impl<'a> AmbientSessionAdmissionContext<'a> {
    // Change only the observed time, without refreshing heads or renewing authority.
    // Full revalidate calls the owners; revalidate_time checks only retained time limits.
    pub(in crate::resident) fn at_time(&self, now: UnixTimeV1) -> Self {
        Self {
            now,
            subject: self.subject,
            grant: self.grant,
            current_principal_trust: self.current_principal_trust,
            capability_revocations: self.capability_revocations,
            deployment_binding_ref: self.deployment_binding_ref,
            deployment_binding: self.deployment_binding,
            deployment_revocations: self.deployment_revocations,
            runtime_authority: self.runtime_authority,
            session_revocations: self.session_revocations,
            route_use: self.route_use,
            generation: SessionAdmissionLeaseContextV1 {
                active_head: self.generation.active_head,
                compiled_generation: self.generation.compiled_generation,
                effective_profile: self.generation.effective_profile,
                activation_fence: self.generation.activation_fence,
                signed: self.generation.signed,
            },
        }
    }
}

/// Why ambient session or child-operation admission failed.
#[derive(Debug, thiserror::Error)]
pub enum AmbientSessionAdmissionError {
    /// No capability grant was resolved for the server-authenticated subject.
    #[error("ambient session has no capability grant")]
    GrantAbsent,
    /// The resolved grant belongs to another subject or key.
    #[error("capability grant belongs to a different subject")]
    WrongSubject,
    /// The grant is not yet valid or has expired.
    #[error("capability grant is outside its validity interval")]
    GrantOutsideValidity,
    /// The selected grant has an effective append-only revocation.
    #[error("capability grant is revoked")]
    GrantRevoked,
    /// The trust checkpoint referenced by the grant is absent.
    #[error("capability grant trust checkpoint is absent")]
    PrincipalTrustCheckpointAbsent,
    /// The selected current trust checkpoint is issued after the supplied time.
    ///
    /// This is a contradiction under the subsystem's declared time assumption,
    /// not proof that the operating-system clock moved backward.
    #[error("selected principal trust checkpoint is not yet issued at the supplied time")]
    PrincipalTrustCheckpointNotYetIssued,
    /// The current capability-revocation checkpoint is absent.
    #[error("capability-revocation checkpoint is absent")]
    CapabilityRevocationCheckpointAbsent,
    /// The grant floor differs from the active trusted floor.
    #[error("capability grant is below or ahead of the active session floor")]
    StaleSessionFloor,
    /// The selected deployment binding differs or is outside its validity interval.
    #[error("capability grant or session names the wrong deployment")]
    WrongDeployment,
    /// The deployment binding is not currently valid.
    #[error("deployment binding is outside its validity interval")]
    DeploymentOutsideValidity,
    /// The owner has effectively revoked the deployment acceptance.
    #[error("deployment acceptance is revoked")]
    DeploymentRevoked,
    /// The runtime authority required for session issuance is absent.
    #[error("deployment runtime authority is absent")]
    RuntimeAuthorityAbsent,
    /// The selected runtime authority is retired or outside its validity interval.
    #[error("deployment runtime authority is inactive")]
    RuntimeAuthorityInactive,
    /// Principal, controller, relationship, environment, purpose, classification, or authority exceeds the grant.
    #[error("ambient session is outside the capability grant scope ceiling")]
    WrongScope,
    /// One session capability is not present in the grant allowlist.
    #[error("ambient session capability ceiling exceeds the capability grant")]
    CapabilityOutsideGrant,
    /// The selected route, destination, transport, or contract set exceeds the grant.
    #[error("ambient session route use exceeds the capability grant")]
    RouteOutsideGrant,
    /// The lease points at a generation other than the active compiled generation.
    #[error("ambient session lease names a stale compiled generation")]
    StaleGeneration,
    /// The lease's active-head or activation-fence coordinate is stale.
    #[error("ambient session lease names a stale activation")]
    StaleActivation,
    /// The session is not currently live under its issue and expiry bounds.
    #[error("ambient session lease is outside its validity interval")]
    SessionOutsideValidity,
    /// Revalidation resolved another grant, checkpoint, runtime-authority, or route coordinate.
    #[error("ambient session revalidation resolved a different authority coordinate")]
    SessionAuthorityChanged,
    /// A child lease has already expired at construction time.
    #[error("operation lease expiry is not in the future")]
    OperationExpired,
    /// A child capability, read, effect, budget, timeout, evidence, or expiry axis widens the parent.
    #[error("operation lease widens its ambient session")]
    OperationWidening,
    /// Current owner authority could not prove a required head or ceiling relation.
    #[error(transparent)]
    Authority(#[from] AmbientSessionAuthorityError),
    /// An existing record owner rejected canonical identity or contextual admission.
    #[error(transparent)]
    Record(#[from] InvalidResidentRecordV1),
}

/// Application-admitted ambient session plus its private non-lease provenance.
///
/// The signed lease is the source of truth for its deployment-runtime
/// authority. The private basis retains only the other owner-resolved
/// coordinates needed for restart revalidation.
#[derive(Debug, Clone)]
pub struct AdmittedAmbientSession {
    lease: SessionAdmissionLeaseV1,
    basis: AmbientSessionBasis,
}

impl AdmittedAmbientSession {
    /// Borrow the exact core session lease admitted by this decision.
    #[must_use]
    pub const fn lease(&self) -> &SessionAdmissionLeaseV1 {
        &self.lease
    }

    /// Borrow the immutable capability-grant reference checked at admission.
    #[must_use]
    pub const fn grant_ref(&self) -> &ResidentRecordRefV1 {
        &self.basis.grant_ref
    }

    /// Borrow the capability grant identity retained outside the core lease.
    #[must_use]
    pub const fn grant_id(&self) -> &SessionGrantIdV1 {
        &self.basis.grant_id
    }

    /// Borrow the canonical capability-grant digest retained by this admission.
    #[must_use]
    pub const fn grant_sha256(&self) -> &CapabilitySessionGrantDigest {
        &self.basis.grant_sha256
    }

    /// Borrow the exact capability-revocation checkpoint reference checked at admission.
    #[must_use]
    pub const fn capability_checkpoint_ref(&self) -> &ResidentRecordRefV1 {
        &self.basis.capability_checkpoint_ref
    }

    /// Borrow the canonical capability-revocation checkpoint digest.
    #[must_use]
    pub const fn capability_checkpoint_sha256(
        &self,
    ) -> &pale_core::records::CapabilityRevocationCheckpointDigest {
        &self.basis.capability_checkpoint_sha256
    }

    /// Borrow the exact deployment-runtime-authority reference checked at admission.
    #[must_use]
    pub fn runtime_authority_ref(&self) -> &ResidentRecordRefV1 {
        self.lease.deployment_runtime_authority_ref()
    }

    /// Borrow the canonical deployment-runtime-authority digest.
    #[must_use]
    pub fn runtime_authority_sha256(&self) -> &ArtifactDigest {
        self.lease.deployment_runtime_authority_sha256()
    }

    /// Recheck this session against current, owner-resolved authority and generation.
    ///
    /// The context must contain a server-resolved caller and already admitted
    /// records. Success applies only to this check: it does not authenticate a
    /// transport, authorize source disclosure, renew the session, or create an
    /// operation. Historical lease recovery does not replace this current check.
    ///
    /// # Errors
    ///
    /// Returns [`AmbientSessionAdmissionError`] when the session's generation,
    /// activation, profile, grant, checkpoint, runtime-authority, route, or
    /// validity proof is no longer current. No lease or authority coordinate
    /// retained by this session is changed.
    pub fn revalidate<P: AmbientSessionAuthorityPort>(
        &self,
        context: &AmbientSessionAdmissionContext<'_>,
        authority: &mut P,
    ) -> Result<(), AmbientSessionAdmissionError> {
        self.basis.revalidate(&self.lease, context, authority)
    }

    /// Recheck only time limits on the same records after full revalidation.
    ///
    /// The caller must preserve the resolved context and change only its time.
    /// This does not refresh authority heads, authenticate a clock, or renew the
    /// session. It performs no owner-port calls.
    pub(in crate::resident) fn revalidate_time(
        &self,
        context: &AmbientSessionAdmissionContext<'_>,
    ) -> Result<(), AmbientSessionAdmissionError> {
        time::revalidate(&self.lease, context).map_err(time::TimeRevalidationFailure::into_reason)
    }

    // Missing resolved inputs do not prove that a retained time limit failed.
    pub(in crate::resident) fn has_temporal_refusal(
        &self,
        context: &AmbientSessionAdmissionContext<'_>,
    ) -> bool {
        matches!(
            time::revalidate(&self.lease, context),
            Err(time::TimeRevalidationFailure::OutsideLimits(_))
        )
    }

    /// Construct one operation lease after revalidating the complete active parent.
    ///
    /// The request must supply [`pale_core::records::InvocationCorrelationV1`]
    /// explicitly. An unavailable native invocation therefore remains the
    /// `Unobserved` variant; this function never synthesizes an identity.
    ///
    /// # Errors
    ///
    /// Returns [`AmbientSessionAdmissionError`] when [`Self::revalidate`] fails,
    /// the child has already expired, or any child axis widens its parent.
    pub fn admit_operation<P: AmbientSessionAuthorityPort>(
        &self,
        operation_schema_sha256: pale_core::records::SchemaDigest,
        request: OperationLeaseRequestV1,
        context: &AmbientSessionAdmissionContext<'_>,
        authority: &mut P,
    ) -> Result<OperationLeaseV1, AmbientSessionAdmissionError> {
        self.revalidate(context, authority)?;
        if request.expires_at <= context.now {
            return Err(AmbientSessionAdmissionError::OperationExpired);
        }
        OperationLeaseV1::seal(operation_schema_sha256, &self.lease, request).map_err(|error| {
            if matches!(error, InvalidResidentRecordV1::LeaseWidening) {
                AmbientSessionAdmissionError::OperationWidening
            } else {
                AmbientSessionAdmissionError::Record(error)
            }
        })
    }
}

/// Admit one exact core lease against all current ambient authority.
///
/// The returned wrapper retains private provenance beside the signed lease. It
/// does not persist the lease or prove that a native terminal session was
/// created or observed.
///
/// # Errors
///
/// Returns [`AmbientSessionAdmissionError`] for absent, expired, revoked,
/// stale-generation, stale-activation, wrong-subject, wrong-scope, authority,
/// or record-owned admission failure.
pub fn admit_ambient_session<P: AmbientSessionAuthorityPort>(
    wire: SessionAdmissionLeaseWireV1,
    context: &AmbientSessionAdmissionContext<'_>,
    authority: &mut P,
) -> Result<AdmittedAmbientSession, AmbientSessionAdmissionError> {
    classify_generation(&wire, context)?;
    let lease = SessionAdmissionLeaseV1::admit_contextual_wire(
        wire,
        &context.generation,
        |domain, preimage, digest, signature| {
            authority.verify_session_admission_lease_signature(domain, preimage, digest, signature)
        },
    )?;
    let evidence = validate_authority(&lease, context, authority)?;
    Ok(AdmittedAmbientSession {
        basis: AmbientSessionBasis {
            grant_ref: evidence.grant_ref.clone(),
            grant_id: evidence.grant.session_grant_id().clone(),
            grant_sha256: evidence.grant.canonical_grant_sha256().clone(),
            capability_checkpoint_ref: evidence.capability_checkpoint.checkpoint_ref.clone(),
            capability_checkpoint_sha256: evidence
                .capability_checkpoint
                .checkpoint
                .canonical_checkpoint_sha256()
                .clone(),
            route: context.route_use.route.clone(),
            origin_node_id: context.route_use.origin_node_id.clone(),
            transport_class: context.route_use.transport_class,
            destination_principal_id: context.route_use.destination_principal_id.clone(),
            destination_node_id: context.route_use.destination_node_id.cloned(),
            contract_set: context.route_use.contract_set.clone(),
        },
        lease,
    })
}

#[cfg(test)]
#[path = "../../tests/internal/resident/session_admission.rs"]
pub(super) mod tests;
