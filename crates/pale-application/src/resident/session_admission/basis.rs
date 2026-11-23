//! Exact session-owned non-lease provenance; persisted custody is not current admission.

use pale_core::records::*;
use serde::{Deserialize, Serialize};

use super::{
    classify_generation, validate_authority, AdmittedAmbientSession,
    AmbientSessionAdmissionContext, AmbientSessionAdmissionError, AmbientSessionAuthorityPort,
    AmbientSessionRouteUse,
};

/// One representation shared by active admission and operation-scoped historical custody.
/// Its private encoding is not a public contract or independent grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::resident) struct AmbientSessionBasis {
    pub(super) grant_ref: ResidentRecordRefV1,
    pub(super) grant_id: SessionGrantIdV1,
    pub(super) grant_sha256: CapabilitySessionGrantDigest,
    pub(super) capability_checkpoint_ref: ResidentRecordRefV1,
    pub(super) capability_checkpoint_sha256: CapabilityRevocationCheckpointDigest,
    pub(super) route: SessionRouteConstraintV1,
    pub(super) origin_node_id: ResidentNodeIdV1,
    pub(super) transport_class: LocalTransportClassV1,
    pub(super) destination_principal_id: ResidentPrincipalIdV1,
    pub(super) destination_node_id: Option<ResidentNodeIdV1>,
    pub(super) contract_set: SessionContractSetConstraintV1,
}

impl AmbientSessionBasis {
    /// Project the exact route use retained with this historical session.
    ///
    /// The projection is immutable provenance, not current route authority.
    /// Current admission still proves it against the selected grant and owner
    /// frontiers through [`Self::revalidate`].
    pub(in crate::resident) const fn route_use(&self) -> AmbientSessionRouteUse<'_> {
        AmbientSessionRouteUse {
            route: &self.route,
            origin_node_id: &self.origin_node_id,
            transport_class: self.transport_class,
            destination_principal_id: &self.destination_principal_id,
            destination_node_id: self.destination_node_id.as_ref(),
            contract_set: &self.contract_set,
        }
    }

    /// The existing current check, shared without substituting recovered metadata for authority.
    pub(super) fn revalidate<P: AmbientSessionAuthorityPort>(
        &self,
        lease: &SessionAdmissionLeaseV1,
        context: &AmbientSessionAdmissionContext<'_>,
        authority: &mut P,
    ) -> Result<(), AmbientSessionAdmissionError> {
        let current_wire = SessionAdmissionLeaseWireV1::from(lease);
        classify_generation(&current_wire, context)?;
        SessionAdmissionLeaseV1::admit_contextual_wire(
            current_wire,
            &context.generation,
            |domain, preimage, digest, signature| {
                authority
                    .verify_session_admission_lease_signature(domain, preimage, digest, signature)
            },
        )?;
        let evidence = validate_authority(lease, context, authority)?;
        if evidence.grant_ref != &self.grant_ref
            || evidence.grant.session_grant_id() != &self.grant_id
            || evidence.grant.canonical_grant_sha256() != &self.grant_sha256
            || evidence.capability_checkpoint.checkpoint_ref != &self.capability_checkpoint_ref
            || evidence
                .capability_checkpoint
                .checkpoint
                .canonical_checkpoint_sha256()
                != &self.capability_checkpoint_sha256
            || !self.same_route(context.route_use)
        {
            return Err(AmbientSessionAdmissionError::SessionAuthorityChanged);
        }
        Ok(())
    }

    fn same_route(&self, route: &AmbientSessionRouteUse<'_>) -> bool {
        &self.route == route.route
            && &self.origin_node_id == route.origin_node_id
            && self.transport_class == route.transport_class
            && &self.destination_principal_id == route.destination_principal_id
            && self.destination_node_id.as_ref() == route.destination_node_id
            && &self.contract_set == route.contract_set
    }
}

/// Protected historical provenance and a contextually recovered core lease.
/// This type deliberately has no conversion into an admitted session without current checks.
#[cfg_attr(test, derive(Clone))]
pub(in crate::resident) struct RetainedAmbientSession {
    lease: SessionAdmissionLeaseV1,
    basis: AmbientSessionBasis,
}

impl RetainedAmbientSession {
    /// Copy the immutable historical basis into one exact recovery continuation.
    ///
    /// The copy does not construct a current admitted session. The continuation
    /// must still call [`Self::revalidate`] before it can advance.
    pub(in crate::resident) fn clone_for_recovery_continuation(&self) -> Self {
        Self {
            lease: self.lease.clone(),
            basis: self.basis.clone(),
        }
    }

    pub(in crate::resident) fn same_session(&self, session: &AdmittedAmbientSession) -> bool {
        self.lease == session.lease && self.basis == session.basis
    }

    /// Capture only the actual admitted value consumed by the operation producer.
    pub(in crate::resident) fn capture(session: &AdmittedAmbientSession) -> Self {
        Self {
            lease: session.lease.clone(),
            basis: session.basis.clone(),
        }
    }

    pub(in crate::resident) const fn basis(&self) -> &AmbientSessionBasis {
        &self.basis
    }

    pub(in crate::resident) const fn lease(&self) -> &SessionAdmissionLeaseV1 {
        &self.lease
    }

    /// Called only after exact operation membership and independent-anchor recovery.
    /// This restores provenance, never the current admitted phase.
    pub(in crate::resident) fn from_protected(
        lease: SessionAdmissionLeaseV1,
        basis: AmbientSessionBasis,
    ) -> Self {
        Self { lease, basis }
    }

    pub(in crate::resident) fn revalidate<P: AmbientSessionAuthorityPort>(
        &self,
        context: &AmbientSessionAdmissionContext<'_>,
        authority: &mut P,
    ) -> Result<AdmittedAmbientSession, AmbientSessionAdmissionError> {
        self.basis.revalidate(&self.lease, context, authority)?;
        Ok(AdmittedAmbientSession {
            lease: self.lease.clone(),
            basis: self.basis.clone(),
        })
    }
}
