//! Private operation association for the exact session basis consumed at start.
//! Decoding is not protected recovery and never admits a live session.

use pale_core::{canon, records::*};
use pale_ports::{
    ControlRecordCoordinate, OperationSessionBasis, MAX_OPERATION_SESSION_BASIS_BYTES,
};
use schemars::transform::{RecursiveTransform, Transform};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use super::{LocalOperationRecoveryError, LocalOperationStartError};
use crate::resident::{
    checkpoint_profile::OperationCheckpointSelection,
    consultation::ConsultationStartIntent,
    event_custody::{LocalOperationPublicationPolicy, LocalOperationPublicationPolicyInput},
    session_admission::{AmbientSessionBasis, RetainedAmbientSession},
};

const FORMAT: &str = "pale-operation-session-basis.v1";

/// Closed private body. Its source-derived shape digest is part of the selected profile.
#[derive(Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Body {
    format: String,
    control_store_id: ControlStoreIdV1,
    control_incarnation_id: ControlIncarnationIdV1,
    checkpoint_domain_id: ControlCheckpointDomainIdV1,
    domain_epoch: ControlDomainEpochV1,
    handle_ref: ResidentRecordRefV1,
    handle_sha256: OperationHandleDigest,
    request_ref: ResidentRecordRefV1,
    request_sha256: CommonRequestEnvelopeDigest,
    session_lease_digest: SessionAdmissionLeaseDigest,
    ambient_basis: AmbientSessionBasis,
    publication_policy: PublicationPolicyBasis,
    consultation_start_intent: Option<ConsultationStartIntent>,
}

/// Operation-stable publication meaning admitted before the start is signed.
#[derive(Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct PublicationPolicyBasis {
    target_scope: EventScopeV1,
    classification: DataClassificationV1,
    visibility: InteractionVisibilityV1,
    disclosure_ceiling: InteractionDisclosureCeilingV1,
    payload_resolution_policy: PayloadResolutionPolicyV1,
    actor_provenance: OwnerRecordCoordinateV1,
}

impl PublicationPolicyBasis {
    fn capture(policy: &LocalOperationPublicationPolicy) -> Self {
        Self {
            target_scope: policy.target_scope().clone(),
            classification: policy.classification().clone(),
            visibility: policy.visibility().clone(),
            disclosure_ceiling: policy.disclosure_ceiling().clone(),
            payload_resolution_policy: policy.payload_resolution_policy().clone(),
            actor_provenance: policy.actor_provenance().clone(),
        }
    }

    fn into_policy(self) -> LocalOperationPublicationPolicy {
        LocalOperationPublicationPolicy::from_admitted_input(LocalOperationPublicationPolicyInput {
            target_scope: &self.target_scope,
            classification: &self.classification,
            visibility: &self.visibility,
            disclosure_ceiling: &self.disclosure_ceiling,
            payload_resolution_policy: &self.payload_resolution_policy,
            actor_provenance: &self.actor_provenance,
        })
    }
}

/// The private body has no public schema registration; its shape is authenticated by profile.
pub(in crate::resident) fn shape_digest() -> Result<ArtifactDigest, canon::CanonError> {
    static DIGEST: OnceLock<ArtifactDigest> = OnceLock::new();
    if let Some(digest) = DIGEST.get() {
        return Ok(digest.clone());
    }
    let mut schema = schemars::schema_for!(Body);
    // The existing physical-schema route uses Schemars' schema-aware traversal;
    // editorial annotations must not change checkpoint signing authority.
    RecursiveTransform(|schema: &mut schemars::Schema| {
        schema.remove("description");
        schema.remove("title");
    })
    .transform(&mut schema);
    // The required discriminator is a value law, not inferred from String's schema.
    let canonical = canon::to_canonical_string(&(FORMAT, schema))?;
    let digest = ArtifactDigest::hash_bytes(canonical.as_bytes());
    Ok(DIGEST.get_or_init(|| digest).clone())
}

pub(super) fn capture(
    selection: &OperationCheckpointSelection,
    handle: (&ResidentRecordRefV1, &OperationHandleV1),
    request: (&ResidentRecordRefV1, &CommonRequestEnvelopeV1),
    session: &RetainedAmbientSession,
    publication_policy: &LocalOperationPublicationPolicy,
    consultation_start_intent: Option<&ConsultationStartIntent>,
) -> Result<OperationSessionBasis, LocalOperationStartError> {
    let scope = selection.initial_basis()?;
    let body = Body {
        format: FORMAT.to_owned(),
        control_store_id: scope.store().store_id().clone(),
        control_incarnation_id: scope.store().incarnation_id().clone(),
        checkpoint_domain_id: scope.domain().clone(),
        domain_epoch: scope.epoch(),
        handle_ref: handle.0.clone(),
        handle_sha256: handle.1.canonical_operation_handle_sha256().clone(),
        request_ref: request.0.clone(),
        request_sha256: request.1.canonical_request_sha256().clone(),
        session_lease_digest: session.lease().session_admission_lease_digest().clone(),
        ambient_basis: session.basis().clone(),
        publication_policy: PublicationPolicyBasis::capture(publication_policy),
        consultation_start_intent: consultation_start_intent.cloned(),
    };
    Ok(OperationSessionBasis::new(
        super::record_projection::coordinate(handle.0, body.handle_sha256.as_str())?,
        super::record_projection::coordinate(request.0, body.request_sha256.as_str())?,
        body.session_lease_digest.clone(),
        canon::to_canonical_string(&body)?.into_bytes(),
        MAX_OPERATION_SESSION_BASIS_BYTES,
    )?)
}

/// Validate the immutable association before requesting membership. The returned
/// basis remains private untrusted data until the caller completes that proof.
pub(super) fn decode(
    stored: &OperationSessionBasis,
    selection: &OperationCheckpointSelection,
    handle: &ControlRecordCoordinate,
    request: &ControlRecordCoordinate,
    lease: &SessionAdmissionLeaseV1,
    maximum_bytes: usize,
) -> Result<
    (
        AmbientSessionBasis,
        LocalOperationPublicationPolicy,
        Option<ConsultationStartIntent>,
    ),
    LocalOperationRecoveryError,
> {
    let bytes = stored.canonical_basis_jcs();
    if bytes.is_empty() || bytes.len() > maximum_bytes.min(MAX_OPERATION_SESSION_BASIS_BYTES) {
        return Err(LocalOperationRecoveryError::Bounds);
    }
    let body: Body = serde_json::from_value(canon::parse_strict(bytes)?)?;
    let scope = selection.initial_basis()?;
    if body.format != FORMAT
        || canon::to_canonical_string(&body)?.as_bytes() != bytes
        || stored.basis_sha256() != &ArtifactDigest::hash_bytes(bytes)
        || stored.handle() != handle
        || stored.request() != request
        || stored.session_lease_digest() != lease.session_admission_lease_digest()
        || body.control_store_id != *scope.store().store_id()
        || body.control_incarnation_id != *scope.store().incarnation_id()
        || body.checkpoint_domain_id != *scope.domain()
        || body.domain_epoch != scope.epoch()
        || &body.handle_ref != handle.record_ref()
        || body.handle_sha256.as_str() != handle.record_sha256().as_str()
        || &body.request_ref != request.record_ref()
        || body.request_sha256.as_str() != request.record_sha256().as_str()
        || &body.session_lease_digest != lease.session_admission_lease_digest()
    {
        return Err(LocalOperationRecoveryError::Association);
    }
    Ok((
        body.ambient_basis,
        body.publication_policy.into_policy(),
        body.consultation_start_intent,
    ))
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/local_operation_start/session_basis.rs"]
pub(in crate::resident) mod test_support;
