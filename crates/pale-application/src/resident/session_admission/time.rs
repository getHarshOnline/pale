//! Time limits shared by full admission and the final execution-time check.

use super::*;

pub(super) enum TimeRevalidationFailure {
    MissingInput(AmbientSessionAdmissionError),
    OutsideLimits(AmbientSessionAdmissionError),
}

impl TimeRevalidationFailure {
    pub(super) fn into_reason(self) -> AmbientSessionAdmissionError {
        match self {
            Self::MissingInput(reason) | Self::OutsideLimits(reason) => reason,
        }
    }
}

impl From<AmbientSessionAdmissionError> for TimeRevalidationFailure {
    fn from(reason: AmbientSessionAdmissionError) -> Self {
        Self::OutsideLimits(reason)
    }
}

pub(super) fn revalidate(
    lease: &SessionAdmissionLeaseV1,
    context: &AmbientSessionAdmissionContext<'_>,
) -> Result<(), TimeRevalidationFailure> {
    grant(
        context
            .grant
            .ok_or(TimeRevalidationFailure::MissingInput(
                AmbientSessionAdmissionError::GrantAbsent,
            ))?
            .grant,
        context.now,
    )?;
    trust(
        context
            .current_principal_trust
            .ok_or(TimeRevalidationFailure::MissingInput(
                AmbientSessionAdmissionError::PrincipalTrustCheckpointAbsent,
            ))?
            .checkpoint,
        context.now,
    )?;
    deployment(context.deployment_binding, context.now)?;
    let frontier =
        context
            .capability_revocations
            .as_ref()
            .ok_or(TimeRevalidationFailure::MissingInput(
                AmbientSessionAdmissionError::CapabilityRevocationCheckpointAbsent,
            ))?;
    capability_checkpoint(frontier.checkpoint, context.now)?;
    if let Some(head) = frontier.grant_head {
        grant_revocation(head.revocation, context.now)?;
    }
    if let Some(head) = context.deployment_revocations.binding_head {
        deployment_revocation(head.revocation, context.now)?;
    }
    runtime(
        context
            .runtime_authority
            .as_ref()
            .ok_or(TimeRevalidationFailure::MissingInput(
                AmbientSessionAdmissionError::RuntimeAuthorityAbsent,
            ))?
            .authority,
        context.now,
    )?;
    session(lease, context.now).map_err(Into::into)
}

pub(super) fn grant(
    grant: &CapabilitySessionGrantV1,
    now: UnixTimeV1,
) -> Result<(), AmbientSessionAdmissionError> {
    if now < *grant.issued_at() || now >= *grant.expires_at() {
        return Err(AmbientSessionAdmissionError::GrantOutsideValidity);
    }
    Ok(())
}

pub(super) fn trust(
    checkpoint: &PrincipalTrustCheckpointV1,
    now: UnixTimeV1,
) -> Result<(), AmbientSessionAdmissionError> {
    if now < *checkpoint.issued_at() {
        return Err(AmbientSessionAdmissionError::PrincipalTrustCheckpointNotYetIssued);
    }
    Ok(())
}

pub(super) fn deployment(
    binding: &DeploymentBindingV1,
    now: UnixTimeV1,
) -> Result<(), AmbientSessionAdmissionError> {
    if now < *binding.issued_at() || now >= *binding.expires_at() {
        return Err(AmbientSessionAdmissionError::DeploymentOutsideValidity);
    }
    Ok(())
}

pub(super) fn capability_checkpoint(
    checkpoint: &CapabilityRevocationCheckpointV1,
    now: UnixTimeV1,
) -> Result<(), AmbientSessionAdmissionError> {
    if checkpoint.generated_at() > &now {
        return Err(AmbientSessionAdmissionError::Authority(
            AmbientSessionAuthorityError::CapabilityRevocationFrontier,
        ));
    }
    Ok(())
}

pub(super) fn grant_revocation(
    revocation: &CapabilitySessionGrantRevocationV1,
    now: UnixTimeV1,
) -> Result<(), AmbientSessionAdmissionError> {
    if revocation.effective_at() <= &now {
        return Err(AmbientSessionAdmissionError::GrantRevoked);
    }
    Ok(())
}

pub(super) fn deployment_revocation(
    revocation: &DeploymentAcceptanceRevocationV1,
    now: UnixTimeV1,
) -> Result<(), AmbientSessionAdmissionError> {
    if revocation.effective_at() <= &now {
        return Err(AmbientSessionAdmissionError::DeploymentRevoked);
    }
    Ok(())
}

pub(super) fn runtime(
    authority: &DeploymentRuntimeAuthorityV1,
    now: UnixTimeV1,
) -> Result<(), AmbientSessionAdmissionError> {
    if now < *authority.issued_at() || now >= *authority.expires_at() {
        return Err(AmbientSessionAdmissionError::RuntimeAuthorityInactive);
    }
    Ok(())
}

pub(super) fn session(
    lease: &SessionAdmissionLeaseV1,
    now: UnixTimeV1,
) -> Result<(), AmbientSessionAdmissionError> {
    if now < *lease.issued_at() || now >= *lease.expires_at() {
        return Err(AmbientSessionAdmissionError::SessionOutsideValidity);
    }
    Ok(())
}
