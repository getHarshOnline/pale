//! Workspace-compiler proposal for independent deployment acceptance.
//!
//! A proposal signs complete admitted [`DeploymentTermsV1`] without naming or
//! predicting a future acceptance. The independently governed acceptance and
//! final binding therefore remain acyclic successors rather than members of
//! this signature preimage.

use super::*;
use serde::Serialize;

/// Public schema root for deployment-binding proposals.
pub const DEPLOYMENT_BINDING_PROPOSAL_V1_SCHEMA_ID: &str = "deployment-binding-proposal.v1";
/// Wire schema carried by every deployment-binding proposal.
pub const DEPLOYMENT_BINDING_PROPOSAL_V1_SCHEMA: &str = "pale/deployment-binding-proposal.v1";
/// Signature domain for [`DeploymentBindingProposalV1`].
pub const DEPLOYMENT_BINDING_PROPOSAL_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_DEPLOYMENT_BINDING_PROPOSAL_V1\0";

resident_signed_context_record!(
    /// Immutable compiler-signed proposal awaiting independent owner acceptance.
    DeploymentBindingProposalV1,
    DeploymentBindingProposalWireV1,
    DeploymentBindingProposalPreimageV1,
    canonical_proposal_sha256: DeploymentBindingProposalDigest,
    DEPLOYMENT_BINDING_PROPOSAL_V1_SCHEMA,
    "deployment binding proposal",
    {
        /// Stable proposal identity committed by the workspace compiler.
        deployment_proposal_id: DeploymentProposalIdV1,
        /// Complete admitted terms proposed without reinterpretation.
        deployment_terms: DeploymentTermsV1,
        /// Canonical identity of `deployment_terms`.
        deployment_terms_sha256: DeploymentTermsDigest,
        /// Earliest time at which an acceptance may be issued.
        proposed_at: UnixTimeV1,
        /// Latest time at which an acceptance may be issued.
        proposal_expires_at: UnixTimeV1,
        /// Workspace-compiler principal that signed this proposal.
        compiler_principal_id: ResidentPrincipalIdV1,
        /// Exact active principal key used by the compiler.
        compiler_key_id: ResidentKeyIdV1,
        /// Active principal-key epoch used by the compiler.
        compiler_key_epoch: PrincipalKeyEpochV1
    },
    wire {
        deployment_proposal_id: DeploymentProposalIdV1,
        deployment_terms: DeploymentTermsWireV1,
        deployment_terms_sha256: DeploymentTermsDigest,
        proposed_at: UnixTimeV1,
        proposal_expires_at: UnixTimeV1,
        compiler_principal_id: ResidentPrincipalIdV1,
        compiler_key_id: ResidentKeyIdV1,
        compiler_key_epoch: PrincipalKeyEpochV1
    },
    validate_deployment_binding_proposal_shape
);

impl DeploymentBindingProposalV1 {
    /// Admit a compiler-signed proposal and its complete nested deployment terms.
    ///
    /// `resolve_terms` performs the external semantic admission required by
    /// [`DeploymentTermsV1::admit_contextual_wire`]. `signer` must be the exact
    /// active compiler-principal key resolved by the trust owner. The signature
    /// callback authenticates the provided domain, complete unsigned preimage,
    /// and canonical digest; this function commits no proposal record.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when nested terms, either schema
    /// digest, terms-digest equality, compiler-key coordinates, proposal digest,
    /// or signature admission fails.
    pub fn admit_contextual_wire<T, S>(
        wire: DeploymentBindingProposalWireV1,
        expected_schema_sha256: &SchemaDigest,
        expected_terms_schema_sha256: &SchemaDigest,
        signer: &PrincipalKeyAdmissionContextV1<'_>,
        resolve_terms: T,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        T: FnOnce(&DeploymentTermsPreimageV1) -> Result<(), InvalidResidentRecordV1>,
        S: FnOnce(
            &[u8],
            &DeploymentBindingProposalPreimageV1,
            &DeploymentBindingProposalDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let DeploymentBindingProposalWireV1 {
            schema_id,
            schema_sha256,
            deployment_proposal_id,
            deployment_terms,
            deployment_terms_sha256,
            proposed_at,
            proposal_expires_at,
            compiler_principal_id,
            compiler_key_id,
            compiler_key_epoch,
            canonical_proposal_sha256,
            signature,
        } = wire;
        let deployment_terms = DeploymentTermsV1::admit_contextual_wire(
            deployment_terms,
            expected_terms_schema_sha256,
            resolve_terms,
        )?;
        let preimage = DeploymentBindingProposalPreimageV1 {
            schema_id,
            schema_sha256,
            deployment_proposal_id,
            deployment_terms,
            deployment_terms_sha256,
            proposed_at,
            proposal_expires_at,
            compiler_principal_id,
            compiler_key_id,
            compiler_key_epoch,
        };
        Self::admit_parts(
            preimage,
            canonical_proposal_sha256,
            signature,
            |preimage, digest, signature| {
                validate_principal_signed_coordinates(
                    "deployment binding proposal",
                    expected_schema_sha256,
                    PrincipalSignedCoordinatesV1 {
                        schema_sha256: &preimage.schema_sha256,
                        subject_principal_id: &preimage.compiler_principal_id,
                        issuer_principal_id: &preimage.compiler_principal_id,
                        issuer_key_id: &preimage.compiler_key_id,
                        issuer_key_epoch: preimage.compiler_key_epoch,
                    },
                    signer,
                )?;
                verify_signature(
                    DEPLOYMENT_BINDING_PROPOSAL_V1_SIGNATURE_DOMAIN,
                    preimage,
                    digest,
                    signature,
                )
            },
        )
    }
}

fn validate_deployment_binding_proposal_shape(
    value: &DeploymentBindingProposalPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    value.deployment_terms.verify()?;
    if value.deployment_terms_sha256 != *value.deployment_terms.canonical_deployment_terms_sha256()
    {
        return Err(contextual(
            "deployment binding proposal",
            "exact deployment terms digest",
        ));
    }
    // The acceptance protocol owns `proposed_at <= accepted_at <=
    // proposal_expires_at`; no additional structural time law is inferred here.
    Ok(())
}
