//! External-owner acceptance and monotone acceptance-revocation contracts.

use super::*;
use serde::{Deserialize, Serialize};

/// Public schema root for deployment-acceptance wires.
pub const DEPLOYMENT_ACCEPTANCE_V1_SCHEMA_ID: &str = "deployment-acceptance.v1";
/// Wire schema carried by every deployment acceptance.
pub const DEPLOYMENT_ACCEPTANCE_V1_SCHEMA: &str = "pale/deployment-acceptance.v1";
/// Signature domain for [`DeploymentAcceptanceV1`].
pub const DEPLOYMENT_ACCEPTANCE_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_DEPLOYMENT_ACCEPTANCE_V1\0";

/// Public schema root for deployment-acceptance-revocation wires.
pub const DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SCHEMA_ID: &str =
    "deployment-acceptance-revocation.v1";
/// Wire schema carried by every deployment acceptance revocation.
pub const DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SCHEMA: &str =
    "pale/deployment-acceptance-revocation.v1";
/// Signature domain for [`DeploymentAcceptanceRevocationV1`].
pub const DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_DEPLOYMENT_ACCEPTANCE_REVOCATION_V1\0";

/// Closed decision vocabulary for a positive deployment acceptance.
///
/// Rejection is the absence of an accepted record, not a signed record whose
/// later fields misleadingly resemble authority to bind.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentAcceptanceDecisionV1 {
    /// The referenced proposal and exact terms were accepted.
    Accepted,
}

resident_signed_record!(
    /// Independently owner-signed acceptance of one exact deployment proposal.
    ///
    /// The record carries owner authority and evidence references without
    /// absorbing their semantics. [`Self::admit_contextual_wire`] requires the
    /// caller to resolve and authenticate that external authority explicitly.
    DeploymentAcceptanceV1,
    DeploymentAcceptanceWireV1,
    DeploymentAcceptancePreimageV1,
    canonical_acceptance_sha256: DeploymentAcceptanceDigest,
    DEPLOYMENT_ACCEPTANCE_V1_SCHEMA,
    "deployment acceptance",
    {
        /// Stable identity of this immutable acceptance.
        deployment_acceptance_id: DeploymentAcceptanceIdV1,
        /// Exact immutable proposal reference accepted by the owner.
        deployment_proposal_ref: ResidentRecordRefV1,
        /// Canonical digest of the referenced proposal.
        deployment_proposal_sha256: DeploymentBindingProposalDigest,
        /// Canonical digest of the complete accepted [`DeploymentTermsV1`].
        deployment_terms_sha256: DeploymentTermsDigest,
        /// Exact owner-published authority record used for acceptance.
        acceptance_authority_ref: ResidentRecordRefV1,
        /// Digest of the owner-published authority record bytes.
        acceptance_authority_sha256: ArtifactDigest,
        /// Exact owner-published evidence record supporting acceptance.
        acceptance_evidence_ref: ResidentRecordRefV1,
        /// Digest of the owner-published evidence record bytes.
        acceptance_evidence_sha256: ArtifactDigest,
        /// Closed positive decision; rejection produces no acceptance record.
        decision: DeploymentAcceptanceDecisionV1,
        /// Unix time at which the owner accepted the proposal.
        accepted_at: UnixTimeV1,
        /// Latest Unix time through which a final binding may remain valid.
        binding_valid_until: UnixTimeV1,
        /// Principal allowed to issue the final deployment binding.
        permitted_binding_issuer_principal_id: ResidentPrincipalIdV1,
        /// External semantic-owner principal that issued this acceptance.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Owner-resolved key that signed this acceptance.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive owner-signing key epoch; not a principal trust-store epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_deployment_acceptance_shape
);

resident_signed_record!(
    /// Monotone revocation of one independently owner-signed acceptance.
    ///
    /// Revocation preserves the acceptance bytes and advances an independent
    /// lineage. The caller must prove that the signer belongs to the same
    /// authority class that could issue the referenced acceptance; Pale does
    /// not infer that class from principal names or key equality.
    DeploymentAcceptanceRevocationV1,
    DeploymentAcceptanceRevocationWireV1,
    DeploymentAcceptanceRevocationPreimageV1,
    canonical_deployment_acceptance_revocation_sha256: DeploymentAcceptanceRevocationDigest,
    DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SCHEMA,
    "deployment acceptance revocation",
    {
        /// Stable identity shared by this revocation lineage.
        deployment_acceptance_revocation_id: DeploymentAcceptanceRevocationIdV1,
        /// Exact immutable acceptance being revoked.
        deployment_acceptance_ref: ResidentRecordRefV1,
        /// Canonical digest of the referenced acceptance.
        deployment_acceptance_sha256: DeploymentAcceptanceDigest,
        /// Canonical digest of the acceptance's exact deployment terms.
        deployment_terms_sha256: DeploymentTermsDigest,
        /// Positive monotone revocation epoch.
        revocation_epoch: DeploymentAcceptanceRevocationEpochV1,
        /// Exact predecessor revocation, absent only at epoch one.
        previous_revocation_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_revocation_ref`].
        previous_revocation_sha256: Option<DeploymentAcceptanceRevocationDigest>,
        /// Epoch paired with [`Self::previous_revocation_ref`].
        previous_revocation_epoch: Option<DeploymentAcceptanceRevocationEpochV1>,
        /// Unix time from which the lineage blocks new dependent admission.
        ///
        /// A successor may preserve or advance this time earlier, but cannot
        /// postpone an already effective revocation.
        effective_at: UnixTimeV1,
        /// Bounded owner-defined revocation reason.
        reason_code: ResidentReasonV1,
        /// External semantic-owner principal that issued the revocation.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Owner-resolved key that signed the revocation.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive owner-signing key epoch; not a principal trust-store epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_deployment_acceptance_revocation_shape,
    wire_transform = deployment_acceptance_revocation_wire_schema
);

/// Resolved immutable dependencies for one deployment acceptance.
#[derive(Clone, Copy)]
pub struct DeploymentAcceptanceAdmissionContextV1<'a> {
    /// Exact selected acceptance schema digest.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Reference used to address the admitted proposal.
    pub deployment_proposal_ref: &'a ResidentRecordRefV1,
    /// Admitted immutable proposal accepted by the owner.
    pub deployment_proposal: &'a DeploymentBindingProposalV1,
    /// Exact owner authority reference selected for this decision.
    pub acceptance_authority_ref: &'a ResidentRecordRefV1,
    /// Digest of the exact owner authority bytes resolved by the caller.
    pub acceptance_authority_sha256: &'a ArtifactDigest,
    /// Exact owner evidence reference selected for this decision.
    pub acceptance_evidence_ref: &'a ResidentRecordRefV1,
    /// Digest of the exact owner evidence bytes resolved by the caller.
    pub acceptance_evidence_sha256: &'a ArtifactDigest,
    /// Owner-resolved principal expected to sign this decision.
    pub issuer_principal_id: &'a ResidentPrincipalIdV1,
    /// Owner-resolved signing key expected to sign this decision.
    pub issuer_key_id: &'a ResidentKeyIdV1,
    /// Owner-resolved positive signing-key epoch.
    pub issuer_key_epoch: SigningKeyEpochV1,
}

/// Current accepted head of one acceptance-revocation lineage.
#[derive(Clone, Copy)]
pub struct DeploymentAcceptanceRevocationHeadV1<'a> {
    /// Exact immutable head reference.
    pub revocation_ref: &'a ResidentRecordRefV1,
    /// Admitted current head record.
    pub revocation: &'a DeploymentAcceptanceRevocationV1,
}

/// Resolved immutable dependencies for one acceptance revocation.
#[derive(Clone, Copy)]
pub struct DeploymentAcceptanceRevocationAdmissionContextV1<'a> {
    /// Exact selected revocation schema digest.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Reference used to address the acceptance being revoked.
    pub deployment_acceptance_ref: &'a ResidentRecordRefV1,
    /// Admitted immutable acceptance being revoked.
    pub deployment_acceptance: &'a DeploymentAcceptanceV1,
    /// Current lineage head, or `None` before epoch one.
    pub current_head: Option<DeploymentAcceptanceRevocationHeadV1<'a>>,
}

impl DeploymentAcceptanceV1 {
    /// Admit one acceptance against its exact proposal and owner evidence.
    ///
    /// `verify_owner_signature` must authenticate
    /// [`DEPLOYMENT_ACCEPTANCE_V1_SIGNATURE_DOMAIN`] followed by the canonical
    /// acceptance digest and prove that the supplied authority/evidence pair
    /// permits this decision. The callback is the semantic-owner boundary;
    /// this function does not reinterpret owner-native authority bytes.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for schema, digest, proposal,
    /// chronology, owner-evidence, signer-coordinate, or signature failure.
    pub fn admit_contextual_wire<F>(
        wire: DeploymentAcceptanceWireV1,
        context: &DeploymentAcceptanceAdmissionContextV1<'_>,
        verify_owner_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &DeploymentAcceptancePreimageV1,
            &DeploymentAcceptanceDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            context.deployment_proposal.verify()?;
            let proposal = context.deployment_proposal;
            if &preimage.schema_sha256 != context.expected_schema_sha256
                || &preimage.deployment_proposal_ref != context.deployment_proposal_ref
                || preimage.deployment_proposal_sha256 != *proposal.canonical_proposal_sha256()
                || preimage.deployment_terms_sha256 != *proposal.deployment_terms_sha256()
                || &preimage.acceptance_authority_ref != context.acceptance_authority_ref
                || &preimage.acceptance_authority_sha256 != context.acceptance_authority_sha256
                || &preimage.acceptance_evidence_ref != context.acceptance_evidence_ref
                || &preimage.acceptance_evidence_sha256 != context.acceptance_evidence_sha256
                || &preimage.issuer_principal_id != context.issuer_principal_id
                || &preimage.issuer_key_id != context.issuer_key_id
                || preimage.issuer_key_epoch != context.issuer_key_epoch
                || preimage.accepted_at < *proposal.proposed_at()
                || preimage.accepted_at > *proposal.proposal_expires_at()
            {
                return Err(contextual(
                    "deployment acceptance",
                    "exact proposal, terms, authority evidence, signer, and proposal horizon",
                ));
            }
            verify_owner_signature(
                DEPLOYMENT_ACCEPTANCE_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

impl DeploymentAcceptanceRevocationV1 {
    /// Admit a monotone acceptance revocation against the exact current head.
    ///
    /// `verify_owner_signature` must authenticate the revocation domain and
    /// prove that its signer belongs to the same authority class permitted to
    /// issue the referenced acceptance. Principal or key equality alone is not
    /// that proof. A successor cannot move [`Self::effective_at`] later than
    /// the current head because a newer head cannot undo an effective denial.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for schema, digest, acceptance,
    /// predecessor, epoch, authority-class, or signature failure.
    pub fn admit_contextual_wire<F>(
        wire: DeploymentAcceptanceRevocationWireV1,
        context: &DeploymentAcceptanceRevocationAdmissionContextV1<'_>,
        verify_owner_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &DeploymentAcceptanceRevocationPreimageV1,
            &DeploymentAcceptanceRevocationDigest,
            &CanonicalSignatureV1,
            &DeploymentAcceptanceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            context.deployment_acceptance.verify()?;
            let acceptance = context.deployment_acceptance;
            if &preimage.schema_sha256 != context.expected_schema_sha256
                || &preimage.deployment_acceptance_ref != context.deployment_acceptance_ref
                || preimage.deployment_acceptance_sha256
                    != *acceptance.canonical_acceptance_sha256()
                || preimage.deployment_terms_sha256 != *acceptance.deployment_terms_sha256()
            {
                return Err(contextual(
                    "deployment acceptance revocation",
                    "exact acceptance and terms",
                ));
            }
            validate_revocation_head(preimage, context.current_head)?;
            verify_owner_signature(
                DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
                acceptance,
            )
        })
    }
}

fn validate_deployment_acceptance_shape(
    _: &DeploymentAcceptancePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    Ok(())
}

fn validate_deployment_acceptance_revocation_shape(
    value: &DeploymentAcceptanceRevocationPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    let predecessor_members = [
        value.previous_revocation_ref.is_some(),
        value.previous_revocation_sha256.is_some(),
        value.previous_revocation_epoch.is_some(),
    ];
    if predecessor_members.iter().all(|present| *present)
        || predecessor_members.iter().all(|present| !*present)
    {
        Ok(())
    } else {
        Err(contextual(
            "deployment acceptance revocation",
            "complete predecessor triple",
        ))
    }
}

fn deployment_acceptance_revocation_wire_schema(schema: &mut schemars::Schema) {
    require_all_or_none_properties_v1(
        schema,
        &[
            "previous_revocation_ref",
            "previous_revocation_sha256",
            "previous_revocation_epoch",
        ],
    );
}

fn validate_revocation_head(
    candidate: &DeploymentAcceptanceRevocationPreimageV1,
    current: Option<DeploymentAcceptanceRevocationHeadV1<'_>>,
) -> Result<(), InvalidResidentRecordV1> {
    match current {
        None if candidate.revocation_epoch.get() == 1
            && candidate.previous_revocation_ref.is_none() =>
        {
            Ok(())
        }
        Some(current) => {
            current.revocation.verify()?;
            let expected_epoch = current
                .revocation
                .revocation_epoch()
                .checked_successor()
                .ok_or_else(|| {
                    contextual(
                        "deployment acceptance revocation",
                        "non-exhausted successor epoch",
                    )
                })?;
            if candidate.deployment_acceptance_revocation_id
                == *current.revocation.deployment_acceptance_revocation_id()
                && candidate.deployment_acceptance_ref
                    == *current.revocation.deployment_acceptance_ref()
                && candidate.deployment_acceptance_sha256
                    == *current.revocation.deployment_acceptance_sha256()
                && candidate.deployment_terms_sha256
                    == *current.revocation.deployment_terms_sha256()
                && candidate.revocation_epoch == expected_epoch
                && candidate.previous_revocation_ref.as_ref() == Some(current.revocation_ref)
                && candidate.previous_revocation_sha256.as_ref()
                    == Some(
                        current
                            .revocation
                            .canonical_deployment_acceptance_revocation_sha256(),
                    )
                && candidate.previous_revocation_epoch.as_ref()
                    == Some(current.revocation.revocation_epoch())
                && candidate.effective_at <= *current.revocation.effective_at()
            {
                Ok(())
            } else {
                Err(contextual(
                    "deployment acceptance revocation",
                    "exact current head and next epoch",
                ))
            }
        }
        _ => Err(contextual(
            "deployment acceptance revocation",
            "epoch-one genesis or exact successor",
        )),
    }
}
