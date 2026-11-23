//! Authenticated administration submissions, not candidate admission or execution.
//! The transaction owner separately checks each candidate signature, compares
//! exact heads, commits its disposition and proves any required anchor.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::*;

/// Public schema root for an administration command.
pub const LOCAL_ADMIN_COMMAND_V1_SCHEMA_ID: &str = "local-admin-command.v1";
/// Wire schema carried by an administration command.
pub const LOCAL_ADMIN_COMMAND_V1_SCHEMA: &str = "pale/local-admin-command.v1";
/// Domain authenticated by the delegated administrator's subject key.
pub const LOCAL_ADMIN_COMMAND_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_LOCAL_ADMIN_COMMAND_V1\0";

/// One exact candidate in the operation-defined order, not a candidate body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalAdminCandidateRecordV1 {
    /// Schema identity interpreted by the candidate's own admission owner.
    pub candidate_record_schema_id: SchemaId,
    /// Immutable candidate record reference.
    pub candidate_record_ref: ResidentRecordRefV1,
    /// Canonical identity defined by that record's contract.
    pub candidate_record_sha256: ArtifactDigest,
}

resident_signed_record!(
    /// Administrator-authenticated submission, not admission of its candidate bodies.
    LocalAdminCommandV1, LocalAdminCommandWireV1, LocalAdminCommandPreimageV1,
    canonical_admin_command_sha256: LocalAdminCommandDigest,
    LOCAL_ADMIN_COMMAND_V1_SCHEMA, "local administration command",
    {
        /// Stable immutable command identity.
        admin_command_id: LocalAdminCommandIdV1,
        /// One operation from the delegated administrator's allowlist.
        operation: LocalAdminOperationV1,
        /// Exact selected current administrator authority.
        admin_authority_ref: ResidentRecordRefV1,
        /// Canonical identity of the selected authority.
        admin_authority_sha256: LocalAdminAuthorityDigest,
        /// Complete operation-ordered candidate descriptors, excluding system checkpoints.
        candidate_records: Vec<LocalAdminCandidateRecordV1>,
        /// Canonical hash of the complete ordered descriptor array.
        candidate_set_sha256: LocalAdminCandidateSetDigest,
        /// Every affected logical head's expected contents.
        expected_heads: Vec<LocalAdminExpectedHeadV1>,
        /// Replay key within principal, administrator authority and operation.
        idempotency_key: LocalAdminIdempotencyKeyV1,
        /// Unix submission issue time in seconds.
        issued_at: UnixTimeV1,
        /// Exclusive expiry for a new submission.
        expires_at: UnixTimeV1,
        /// Delegated administrator subject, not the principal root identity.
        signer_identity: LocalAdminSubjectIdentityV1,
        /// Exact delegated subject key.
        signer_key_id: ResidentKeyIdV1,
        /// Exact delegated subject-key epoch.
        signer_key_epoch: SigningKeyEpochV1
    },
    validate_command
);

/// Current accepted trust/administrator records resolved by the admitting owner.
/// Construction performs no freshness, anchor, storage or candidate-body lookup.
pub struct LocalAdminCommandAdmissionContextV1<'a> {
    /// Exact selected command schema digest.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Current accepted trust frontier, already checked against its external minimum.
    pub trust_checkpoint: &'a PrincipalTrustCheckpointV1,
    /// Reference resolved from that frontier's current administrator head.
    pub admin_authority_ref: &'a ResidentRecordRefV1,
    /// Admitted authority record at that exact head.
    pub admin_authority: &'a LocalAdminAuthorityV1,
    /// Current time under the caller's declared time source.
    pub now: UnixTimeV1,
}

/// Historical trust and authority selected for one retained command.
///
/// The commit time is evidence from the command's retained receipt. It checks
/// whether the command was valid when that outcome was committed. It is not a
/// current clock and does not restore present administration permission.
pub struct HistoricalLocalAdminCommandAdmissionContextV1<'a> {
    /// Exact selected command schema digest.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Admitted trust checkpoint that selected the authority at commit time.
    pub trust_checkpoint: &'a PrincipalTrustCheckpointV1,
    /// Reference selected by that historical trust checkpoint.
    pub admin_authority_ref: &'a ResidentRecordRefV1,
    /// Admitted authority record at that exact historical head.
    pub admin_authority: &'a LocalAdminAuthorityV1,
    /// Commit time retained by the command's authenticated receipt.
    pub committed_at: UnixTimeV1,
}

impl LocalAdminCommandV1 {
    /// Authenticate a new submission under the exact current delegated subject key.
    ///
    /// The signature callback authenticates the supplied public key and domain
    /// followed by the canonical command digest. Candidate signatures, operation-
    /// specific set membership, expected-head contents and replay storage remain
    /// independently required before execution. Historical replay must recover its
    /// original disposition; it must not reclassify an expired command as new.
    ///
    /// # Errors
    /// Rejects malformed commands, stale authority coordinates, missing operation
    /// permission, invalid times, changed subject keys or failed authentication.
    pub fn admit_authenticated_wire<F>(
        wire: LocalAdminCommandWireV1,
        context: &LocalAdminCommandAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &Ed25519PublicKeyV1,
            &[u8],
            &LocalAdminCommandPreimageV1,
            &LocalAdminCommandDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        context.trust_checkpoint.verify()?;
        context.admin_authority.verify()?;
        Self::admit_wire(wire, |value, digest, signature| {
            validate_authority_context(
                value,
                context.expected_schema_sha256,
                context.trust_checkpoint,
                context.admin_authority_ref,
                context.admin_authority,
                "current delegated subject, operation and validity interval",
            )?;
            validate_use_time(
                value,
                context.now,
                "current delegated subject, operation and validity interval",
            )?;
            verify_signature(
                context.admin_authority.subject_public_key(),
                LOCAL_ADMIN_COMMAND_V1_SIGNATURE_DOMAIN,
                value,
                digest,
                signature,
            )
        })
    }

    /// Authenticate a retained command at its recorded commit time.
    ///
    /// This path uses the exact trust checkpoint and delegated authority that
    /// governed the committed outcome. It does not compare the current clock,
    /// select current authority, or permit the command to run again.
    ///
    /// # Errors
    /// Rejects malformed commands, unlike historical authority coordinates,
    /// missing operation permission, invalid record-time intervals, changed
    /// subject keys, or failed authentication.
    pub fn admit_historical_authenticated_wire<F>(
        wire: LocalAdminCommandWireV1,
        context: &HistoricalLocalAdminCommandAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &Ed25519PublicKeyV1,
            &[u8],
            &LocalAdminCommandPreimageV1,
            &LocalAdminCommandDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        context.trust_checkpoint.verify()?;
        context.admin_authority.verify()?;
        Self::admit_wire(wire, |value, digest, signature| {
            validate_authority_context(
                value,
                context.expected_schema_sha256,
                context.trust_checkpoint,
                context.admin_authority_ref,
                context.admin_authority,
                "selected delegated subject, operation and authority interval",
            )?;
            validate_use_time(
                value,
                context.committed_at,
                "historical command validity interval",
            )?;
            verify_signature(
                context.admin_authority.subject_public_key(),
                LOCAL_ADMIN_COMMAND_V1_SIGNATURE_DOMAIN,
                value,
                digest,
                signature,
            )
        })
    }
}

fn validate_authority_context(
    value: &LocalAdminCommandPreimageV1,
    expected_schema_sha256: &SchemaDigest,
    trust: &PrincipalTrustCheckpointV1,
    admin_authority_ref: &ResidentRecordRefV1,
    authority: &LocalAdminAuthorityV1,
    invariant: &'static str,
) -> Result<(), InvalidResidentRecordV1> {
    if value.schema_sha256 != *expected_schema_sha256
        || value.admin_authority_ref != *admin_authority_ref
        || value.admin_authority_sha256 != *authority.canonical_admin_authority_sha256()
        || trust.local_admin_authority_ref() != admin_authority_ref
        || trust.local_admin_authority_sha256() != authority.canonical_admin_authority_sha256()
        || trust.principal_id() != authority.principal_id()
        || authority.state() != &PrincipalAuthorityStateV1::Active
        || !authority.allowed_operations().contains(&value.operation)
        || value.signer_identity != *authority.subject_identity()
        || value.signer_key_id != *authority.subject_key_id()
        || value.signer_key_epoch != *authority.subject_key_epoch()
        || value.issued_at < *authority.issued_at()
        || value.expires_at > *authority.expires_at()
    {
        return Err(contextual("local administration command", invariant));
    }
    Ok(())
}

fn validate_use_time(
    value: &LocalAdminCommandPreimageV1,
    use_time: UnixTimeV1,
    invariant: &'static str,
) -> Result<(), InvalidResidentRecordV1> {
    if use_time < value.issued_at || use_time >= value.expires_at {
        return Err(contextual("local administration command", invariant));
    }
    Ok(())
}

fn validate_command(value: &LocalAdminCommandPreimageV1) -> Result<(), InvalidResidentRecordV1> {
    bounded_len("administration candidates", value.candidate_records.len())?;
    bounded_len("administration expected heads", value.expected_heads.len())?;
    if value.candidate_records.is_empty()
        || value.expected_heads.is_empty()
        || value.issued_at >= value.expires_at
    {
        return Err(contextual(
            "local administration command",
            "nonempty candidate/head sets and ordered validity",
        ));
    }
    let mut references = BTreeSet::new();
    for candidate in &value.candidate_records {
        if !references.insert(&candidate.candidate_record_ref)
            || candidate.candidate_record_schema_id.as_str() == CONTROL_STATE_CHECKPOINT_V1_SCHEMA
        {
            return Err(contextual(
                "local administration command",
                "unique candidate references and no system-produced checkpoint candidate",
            ));
        }
    }
    if LocalAdminCandidateSetDigest::hash_canonical(&value.candidate_records)?
        != value.candidate_set_sha256
    {
        return Err(InvalidResidentRecordV1::DigestMismatch {
            family: "local administration candidate set",
        });
    }
    for (index, head) in value.expected_heads.iter().enumerate() {
        if head.expected_head_ref.is_some() != head.expected_head_sha256.is_some()
            || (head.expected_head_ref.is_none() && head.expected_head_epoch.is_some())
            || value.expected_heads[..index]
                .iter()
                .any(|prior| prior.head_kind.same_slot(&head.head_kind))
        {
            return Err(contextual(
                "local administration command",
                "paired expected coordinates and unique logical head slots",
            ));
        }
    }
    Ok(())
}
