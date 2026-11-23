//! Final principal-signed deployment binding.
//!
//! A binding closes one compiler proposal and one independent owner acceptance
//! over byte-identical [`DeploymentTermsV1`]. It does not activate a runtime;
//! it is the immutable authority input later contracts may consume.

use super::*;
use serde::Serialize;

/// Public schema root for final deployment-binding wires.
pub const DEPLOYMENT_BINDING_V1_SCHEMA_ID: &str = "deployment-binding.v1";
/// Wire schema carried by every final deployment binding.
pub const DEPLOYMENT_BINDING_V1_SCHEMA: &str = "pale/deployment-binding.v1";
/// Signature domain for [`DeploymentBindingV1`].
pub const DEPLOYMENT_BINDING_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_DEPLOYMENT_BINDING_V1\0";

resident_signed_context_record!(
    /// Final immutable binding of a proposal, acceptance, and complete terms.
    ///
    /// In the proof graph below, `T`, `P`, `A`, `B`, and `R` are admitted
    /// deployment terms, proposal, owner acceptance, final binding, and runtime
    /// authority; `V` is a monotone acceptance-revocation successor. The time
    /// law names the issue, acceptance, expiry, and owner-imposed validity
    /// projections of those records. Together the laws prove an acyclic,
    /// time-bounded dependency chain. They do not activate `R`, mutate a trust
    /// store, or establish provider or native readiness.
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/deployment-proof-graph.display.mathml"
    ))]
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/deployment-time-horizon.display.mathml"
    ))]
    DeploymentBindingV1,
    DeploymentBindingWireV1,
    DeploymentBindingPreimageV1,
    canonical_binding_sha256: DeploymentBindingDigest,
    DEPLOYMENT_BINDING_V1_SCHEMA,
    "deployment binding",
    {
        /// Stable identity of the finalized deployment binding.
        deployment_binding_id: DeploymentBindingIdV1,
        /// Exact compiler proposal reference.
        deployment_proposal_ref: ResidentRecordRefV1,
        /// Canonical digest of the exact compiler proposal.
        deployment_proposal_sha256: DeploymentBindingProposalDigest,
        /// Exact independently governed acceptance reference.
        deployment_acceptance_ref: ResidentRecordRefV1,
        /// Canonical digest of the exact acceptance.
        deployment_acceptance_sha256: DeploymentAcceptanceDigest,
        /// Complete terms, byte-identical to those in the proposal.
        deployment_terms: DeploymentTermsV1,
        /// Canonical digest of the complete deployment terms.
        deployment_terms_sha256: DeploymentTermsDigest,
        /// Unix time at which the binding was issued.
        issued_at: UnixTimeV1,
        /// Latest Unix time for which this binding remains eligible.
        expires_at: UnixTimeV1,
        /// Principal permitted by the acceptance to finalize the binding.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Active principal key used to sign the binding.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive active principal-key epoch.
        issuer_key_epoch: PrincipalKeyEpochV1
    },
    wire {
        deployment_binding_id: DeploymentBindingIdV1,
        deployment_proposal_ref: ResidentRecordRefV1,
        deployment_proposal_sha256: DeploymentBindingProposalDigest,
        deployment_acceptance_ref: ResidentRecordRefV1,
        deployment_acceptance_sha256: DeploymentAcceptanceDigest,
        deployment_terms: DeploymentTermsWireV1,
        deployment_terms_sha256: DeploymentTermsDigest,
        issued_at: UnixTimeV1,
        expires_at: UnixTimeV1,
        issuer_principal_id: ResidentPrincipalIdV1,
        issuer_key_id: ResidentKeyIdV1,
        issuer_key_epoch: PrincipalKeyEpochV1
    },
    validate_deployment_binding_shape
);

/// Resolved immutable dependencies for final deployment-binding admission.
#[derive(Clone, Copy)]
pub struct DeploymentBindingAdmissionContextV1<'a> {
    /// Exact selected binding schema digest.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Exact selected deployment-terms schema digest.
    pub expected_terms_schema_sha256: &'a SchemaDigest,
    /// Immutable reference resolving the compiler proposal.
    pub deployment_proposal_ref: &'a ResidentRecordRefV1,
    /// Admitted compiler proposal.
    pub deployment_proposal: &'a DeploymentBindingProposalV1,
    /// Immutable reference resolving the owner acceptance.
    pub deployment_acceptance_ref: &'a ResidentRecordRefV1,
    /// Admitted owner acceptance.
    pub deployment_acceptance: &'a DeploymentAcceptanceV1,
    /// Owner-resolved active principal key allowed to finalize the binding.
    pub signer: PrincipalKeyAdmissionContextV1<'a>,
}

impl DeploymentBindingV1 {
    /// Admit a final binding over one exact proposal/acceptance pair.
    ///
    /// `resolve_terms` repeats the owner-resolved semantic admission for the
    /// nested wire terms. The record is accepted only when those terms are
    /// byte-identical to the proposal terms, every duplicated digest agrees,
    /// the acceptance names that proposal, all temporal bounds hold, and the
    /// final signer is the principal explicitly permitted by the acceptance.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for nested-terms, schema, digest,
    /// reference, chronology, signer, or signature failure.
    pub fn admit_contextual_wire<T, S>(
        wire: DeploymentBindingWireV1,
        context: &DeploymentBindingAdmissionContextV1<'_>,
        resolve_terms: T,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        T: FnOnce(&DeploymentTermsPreimageV1) -> Result<(), InvalidResidentRecordV1>,
        S: FnOnce(
            &[u8],
            &DeploymentBindingPreimageV1,
            &DeploymentBindingDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let DeploymentBindingWireV1 {
            schema_id,
            schema_sha256,
            deployment_binding_id,
            deployment_proposal_ref,
            deployment_proposal_sha256,
            deployment_acceptance_ref,
            deployment_acceptance_sha256,
            deployment_terms,
            deployment_terms_sha256,
            issued_at,
            expires_at,
            issuer_principal_id,
            issuer_key_id,
            issuer_key_epoch,
            canonical_binding_sha256,
            signature,
        } = wire;
        let deployment_terms = DeploymentTermsV1::admit_contextual_wire(
            deployment_terms,
            context.expected_terms_schema_sha256,
            resolve_terms,
        )?;
        let preimage = DeploymentBindingPreimageV1 {
            schema_id,
            schema_sha256,
            deployment_binding_id,
            deployment_proposal_ref,
            deployment_proposal_sha256,
            deployment_acceptance_ref,
            deployment_acceptance_sha256,
            deployment_terms,
            deployment_terms_sha256,
            issued_at,
            expires_at,
            issuer_principal_id,
            issuer_key_id,
            issuer_key_epoch,
        };
        Self::admit_parts(
            preimage,
            canonical_binding_sha256,
            signature,
            |preimage, digest, signature| {
                validate_deployment_binding_context(preimage, context)?;
                validate_principal_signed_coordinates(
                    "deployment binding",
                    context.expected_schema_sha256,
                    PrincipalSignedCoordinatesV1 {
                        schema_sha256: &preimage.schema_sha256,
                        subject_principal_id: context
                            .deployment_acceptance
                            .permitted_binding_issuer_principal_id(),
                        issuer_principal_id: &preimage.issuer_principal_id,
                        issuer_key_id: &preimage.issuer_key_id,
                        issuer_key_epoch: preimage.issuer_key_epoch,
                    },
                    &context.signer,
                )?;
                verify_signature(
                    DEPLOYMENT_BINDING_V1_SIGNATURE_DOMAIN,
                    preimage,
                    digest,
                    signature,
                )
            },
        )
    }
}

fn validate_deployment_binding_shape(
    value: &DeploymentBindingPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    value.deployment_terms.verify()?;
    if value.deployment_terms_sha256 == *value.deployment_terms.canonical_deployment_terms_sha256()
    {
        Ok(())
    } else {
        Err(contextual(
            "deployment binding",
            "exact embedded deployment terms digest",
        ))
    }
}

fn validate_deployment_binding_context(
    candidate: &DeploymentBindingPreimageV1,
    context: &DeploymentBindingAdmissionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    let proposal = context.deployment_proposal;
    let acceptance = context.deployment_acceptance;
    proposal.verify()?;
    acceptance.verify()?;

    let exact_lineage = candidate.deployment_proposal_ref == *context.deployment_proposal_ref
        && candidate.deployment_proposal_sha256 == *proposal.canonical_proposal_sha256()
        && candidate.deployment_acceptance_ref == *context.deployment_acceptance_ref
        && candidate.deployment_acceptance_sha256 == *acceptance.canonical_acceptance_sha256()
        && acceptance.deployment_proposal_ref() == context.deployment_proposal_ref
        && acceptance.deployment_proposal_sha256() == proposal.canonical_proposal_sha256();
    let exact_terms = candidate.deployment_terms == *proposal.deployment_terms()
        && candidate.deployment_terms_sha256 == *proposal.deployment_terms_sha256()
        && candidate.deployment_terms_sha256 == *acceptance.deployment_terms_sha256();
    let temporal = *proposal.proposed_at() <= *acceptance.accepted_at()
        && *acceptance.accepted_at() <= *proposal.proposal_expires_at()
        && candidate.issued_at >= *acceptance.accepted_at()
        && candidate.expires_at <= *acceptance.binding_valid_until();
    if exact_lineage && exact_terms && temporal {
        Ok(())
    } else {
        Err(contextual(
            "deployment binding",
            "exact proposal, acceptance, terms, and temporal bounds",
        ))
    }
}
