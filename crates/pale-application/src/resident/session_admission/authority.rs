//! Current-authority and revocation-frontier policy for ambient admission.

use super::scope::{validate_capabilities, validate_route, validate_scope};
use super::*;
use pale_core::records::{PrincipalAuthorityStateV1, SESSION_ADMISSION_LEASE_V1_SCHEMA};

/// Borrowed authority coordinates proved current for one admission decision.
pub(super) struct ValidatedAuthority<'a> {
    /// Exact immutable reference of the admitted grant.
    pub(super) grant_ref: &'a ResidentRecordRefV1,
    /// Structurally verified, already signature-admitted grant.
    pub(super) grant: &'a CapabilitySessionGrantV1,
    /// Current capability-revocation checkpoint and optional grant head.
    pub(super) capability_checkpoint: &'a CapabilityRevocationFrontier<'a>,
}

/// Join one lease to every current trust, revocation, deployment, and route proof.
///
/// # Errors
///
/// Returns [`AmbientSessionAdmissionError`] at the exact record, policy, or
/// owner-proof frontier that cannot admit the lease.
pub(super) fn validate_authority<'a, P: AmbientSessionAuthorityPort>(
    lease: &SessionAdmissionLeaseV1,
    context: &'a AmbientSessionAdmissionContext<'a>,
    authority: &mut P,
) -> Result<ValidatedAuthority<'a>, AmbientSessionAdmissionError> {
    let grant_authority = context
        .grant
        .ok_or(AmbientSessionAdmissionError::GrantAbsent)?;
    let grant = grant_authority.grant;
    grant.verify()?;
    if grant.subject() != context.subject {
        return Err(AmbientSessionAdmissionError::WrongSubject);
    }
    time::grant(grant, context.now)?;

    let current_trust = context
        .current_principal_trust
        .ok_or(AmbientSessionAdmissionError::PrincipalTrustCheckpointAbsent)?;
    grant_authority.issued_trust_checkpoint.verify()?;
    current_trust.checkpoint.verify()?;
    if grant.checked_principal_trust_checkpoint_ref() != grant_authority.issued_trust_checkpoint_ref
        || grant.checked_principal_trust_checkpoint_sha256()
            != grant_authority
                .issued_trust_checkpoint
                .canonical_trust_checkpoint_sha256()
        || grant.issuer_principal_id() != grant_authority.issued_trust_checkpoint.principal_id()
        || current_trust.checkpoint.principal_id()
            != grant_authority.issued_trust_checkpoint.principal_id()
    {
        return Err(AmbientSessionAdmissionError::WrongScope);
    }
    authority.prove_current_principal_trust(
        grant_authority.issued_trust_checkpoint_ref,
        grant_authority.issued_trust_checkpoint,
        current_trust.checkpoint_ref,
        current_trust.checkpoint,
    )?;
    time::trust(current_trust.checkpoint, context.now)?;
    if grant.session_floor_epoch() != current_trust.checkpoint.session_floor_epoch() {
        return Err(AmbientSessionAdmissionError::StaleSessionFloor);
    }

    context.deployment_binding.verify()?;
    time::deployment(context.deployment_binding, context.now)?;
    if grant.deployment_binding_ref() != context.deployment_binding_ref
        || grant.deployment_binding_sha256()
            != context.deployment_binding.canonical_binding_sha256()
        || lease.deployment_binding_ref() != context.deployment_binding_ref
        || lease.deployment_binding_sha256().as_str()
            != context
                .deployment_binding
                .canonical_binding_sha256()
                .as_str()
    {
        return Err(AmbientSessionAdmissionError::WrongDeployment);
    }

    let capability_checkpoint = context
        .capability_revocations
        .as_ref()
        .ok_or(AmbientSessionAdmissionError::CapabilityRevocationCheckpointAbsent)?;
    capability_checkpoint.checkpoint.verify()?;
    if capability_checkpoint.checkpoint.authority_principal_id() != grant.issuer_principal_id() {
        return Err(AmbientSessionAdmissionError::Authority(
            AmbientSessionAuthorityError::CapabilityRevocationFrontier,
        ));
    }
    time::capability_checkpoint(capability_checkpoint.checkpoint, context.now)?;
    validate_genesis_capability_frontier(capability_checkpoint)?;
    validate_grant_revocation_head(grant_authority, capability_checkpoint, context.now)?;
    authority.prove_capability_revocation_frontier(
        capability_checkpoint.checkpoint_ref,
        capability_checkpoint.checkpoint,
        grant_authority.grant_ref,
        grant,
        capability_checkpoint
            .grant_head
            .map(|head| head.revocation_ref),
        capability_checkpoint.grant_head.map(|head| head.revocation),
    )?;

    if grant.checked_deployment_acceptance_revocation_epoch()
        != &context.deployment_revocations.epoch
    {
        return Err(AmbientSessionAdmissionError::DeploymentRevoked);
    }
    validate_deployment_revocation_head(
        context.deployment_binding,
        context.deployment_revocations,
        context.now,
    )?;
    authority.prove_deployment_revocation_frontier(
        context.deployment_binding,
        context.deployment_revocations.epoch,
        context
            .deployment_revocations
            .binding_head
            .map(|head| head.revocation_ref),
        context
            .deployment_revocations
            .binding_head
            .map(|head| head.revocation),
        current_trust.checkpoint,
    )?;

    let runtime = context
        .runtime_authority
        .as_ref()
        .ok_or(AmbientSessionAdmissionError::RuntimeAuthorityAbsent)?;
    validate_runtime_authority(
        runtime,
        context.deployment_binding_ref,
        context.deployment_binding,
        context.now,
    )?;
    if lease.deployment_runtime_authority_ref() != runtime.authority_ref
        || lease.deployment_runtime_authority_sha256().as_str()
            != runtime
                .authority
                .canonical_deployment_runtime_authority_sha256()
                .as_str()
    {
        return Err(AmbientSessionAdmissionError::SessionAuthorityChanged);
    }
    authority.prove_runtime_authority_head(
        runtime.authority_ref,
        runtime.authority,
        current_trust.checkpoint,
    )?;

    if lease.revocation_epoch() != &context.session_revocations.epoch
        || lease.checked_revocation_ref().as_ref() != context.session_revocations.revocation_ref
        || lease.checked_revocation_sha256().as_ref()
            != context.session_revocations.revocation_sha256
    {
        return Err(AmbientSessionAdmissionError::Authority(
            AmbientSessionAuthorityError::SessionRevocationFrontier,
        ));
    }
    authority.prove_session_revocation_frontier(
        lease.principal_id(),
        context.session_revocations.epoch,
        context.session_revocations.revocation_ref,
        context.session_revocations.revocation_sha256,
        current_trust.checkpoint,
    )?;

    time::session(lease, context.now)?;
    validate_scope(lease, grant, context.deployment_binding, authority)?;
    validate_capabilities(lease, grant)?;
    validate_route(context.route_use, grant, lease.principal_id())?;

    Ok(ValidatedAuthority {
        grant_ref: grant_authority.grant_ref,
        grant,
        capability_checkpoint,
    })
}

/// Enforce the canonical empty grant-head set at checkpoint epoch one.
///
/// # Errors
///
/// Returns [`AmbientSessionAuthorityError::CapabilityRevocationFrontier`]
/// when the genesis checkpoint claims a covered grant or selected head.
fn validate_genesis_capability_frontier(
    frontier: &CapabilityRevocationFrontier<'_>,
) -> Result<(), AmbientSessionAdmissionError> {
    if frontier.checkpoint.checkpoint_epoch().get() != 1 {
        return Ok(());
    }
    // RFC 8785's canonical representation of an empty JSON array is exactly `[]`.
    let empty_root = ArtifactDigest::hash_bytes(b"[]");
    if frontier.checkpoint.covered_grant_count().get() != 0
        || frontier.checkpoint.authority_heads_root_sha256() != &empty_root
        || frontier.grant_head.is_some()
    {
        return Err(AmbientSessionAdmissionError::Authority(
            AmbientSessionAuthorityError::CapabilityRevocationFrontier,
        ));
    }
    Ok(())
}

/// Validate the selected grant-specific revocation head and its effective time.
///
/// # Errors
///
/// Returns [`AmbientSessionAdmissionError::GrantRevoked`] for an effective
/// revocation, or the exact record/frontier error for malformed lineage.
fn validate_grant_revocation_head(
    grant: CapabilityGrantAuthority<'_>,
    frontier: &CapabilityRevocationFrontier<'_>,
    now: UnixTimeV1,
) -> Result<(), AmbientSessionAdmissionError> {
    let Some(head) = frontier.grant_head else {
        return Ok(());
    };
    head.revocation.verify()?;
    if head.revocation.grant_ref() != grant.grant_ref
        || head.revocation.grant_sha256() != grant.grant.canonical_grant_sha256()
        || head.revocation.grant_id() != grant.grant.session_grant_id()
    {
        return Err(AmbientSessionAdmissionError::Authority(
            AmbientSessionAuthorityError::CapabilityRevocationFrontier,
        ));
    }
    time::grant_revocation(head.revocation, now)
}

/// Validate the selected deployment-acceptance revocation head.
///
/// # Errors
///
/// Returns [`AmbientSessionAdmissionError::DeploymentRevoked`] for an
/// effective revocation, or the exact record/frontier error for malformed
/// acceptance lineage.
fn validate_deployment_revocation_head(
    binding: &DeploymentBindingV1,
    frontier: &DeploymentAcceptanceRevocationFrontier<'_>,
    now: UnixTimeV1,
) -> Result<(), AmbientSessionAdmissionError> {
    let Some(head) = frontier.binding_head else {
        return Ok(());
    };
    head.revocation.verify()?;
    if head.revocation.deployment_acceptance_ref() != binding.deployment_acceptance_ref()
        || head.revocation.deployment_acceptance_sha256() != binding.deployment_acceptance_sha256()
        || head.revocation.revocation_epoch() != &frontier.epoch
    {
        return Err(AmbientSessionAdmissionError::Authority(
            AmbientSessionAuthorityError::DeploymentRevocationFrontier,
        ));
    }
    time::deployment_revocation(head.revocation, now)
}

/// Validate the active runtime-authority head against the selected deployment.
///
/// # Errors
///
/// Returns [`AmbientSessionAdmissionError::RuntimeAuthorityInactive`] when the
/// head is retired or outside its validity interval, and
/// [`AmbientSessionAdmissionError::WrongDeployment`] when its deployment,
/// principal, or permitted-record coordinates differ.
fn validate_runtime_authority(
    runtime: &DeploymentRuntimeAuthorityHead<'_>,
    binding_ref: &ResidentRecordRefV1,
    binding: &DeploymentBindingV1,
    now: UnixTimeV1,
) -> Result<(), AmbientSessionAdmissionError> {
    runtime.authority.verify()?;
    if runtime.authority.state() != &PrincipalAuthorityStateV1::Active {
        return Err(AmbientSessionAdmissionError::RuntimeAuthorityInactive);
    }
    time::runtime(runtime.authority, now)?;
    if runtime.authority.deployment_binding_ref() != binding_ref
        || runtime.authority.deployment_binding_sha256() != binding.canonical_binding_sha256()
        || runtime.authority.principal_id() != binding.deployment_terms().principal_scope()
        || !runtime
            .authority
            .permitted_record_schema_ids()
            .iter()
            .any(|schema| schema.as_str() == SESSION_ADMISSION_LEASE_V1_SCHEMA)
    {
        return Err(AmbientSessionAdmissionError::WrongDeployment);
    }
    Ok(())
}
