//! Administration outcome contents and canonical identity.
//!
//! Structural verification does not authenticate the issuer or prove a store
//! commit. Receipt issuance, command association, atomic custody and anchor
//! completion belong to the administration transaction.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::*;

/// Public schema root for an administration receipt.
pub const LOCAL_ADMIN_RECEIPT_V1_SCHEMA_ID: &str = "local-admin-receipt.v1";
/// Wire schema carried by an administration receipt.
pub const LOCAL_ADMIN_RECEIPT_V1_SCHEMA: &str = "pale/local-admin-receipt.v1";
/// Domain signed by the exact admitted deployment runtime key.
pub const LOCAL_ADMIN_RECEIPT_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_LOCAL_ADMIN_RECEIPT_V1\0";

/// Recorded administration outcome. Replaying a command preserves this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LocalAdminDispositionV1 {
    /// The transaction committed its records and affected heads.
    Committed,
    /// The request was rejected without committing candidate records.
    Rejected,
    /// Replay or expected-state comparison conflicted; no candidate was committed.
    Conflict,
}

/// Immutable record named by a committed administration outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalAdminCommittedRecordV1 {
    /// Immutable record reference, including system-produced checkpoints.
    pub committed_record_ref: ResidentRecordRefV1,
    /// Canonical identity owned by the referenced record's contract.
    pub committed_record_sha256: ArtifactDigest,
}

/// Previous and resulting contents of one affected logical head.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalAdminHeadTransitionV1 {
    /// Reuses the same scoped head identity as command expectations.
    pub head_kind: LocalAdminHeadSelectorV1,
    /// Previous record; absent when the head did not exist.
    pub previous_head_ref: Option<ResidentRecordRefV1>,
    /// Canonical digest paired with the previous reference.
    pub previous_head_sha256: Option<ArtifactDigest>,
    /// Resulting record; absent when the head was removed.
    pub new_head_ref: Option<ResidentRecordRefV1>,
    /// Canonical digest paired with the resulting reference.
    pub new_head_sha256: Option<ArtifactDigest>,
}

resident_signed_record!(
    /// Administration outcome with valid structure and canonical identity.
    ///
    /// Contextual issuance and admission bind the exact deployment runtime
    /// authority, command and transaction result. Even an authenticated receipt
    /// proves neither physical commitment nor an independent anchor.
    LocalAdminReceiptV1, LocalAdminReceiptWireV1, LocalAdminReceiptPreimageV1,
    canonical_admin_receipt_sha256: LocalAdminReceiptDigest,
    LOCAL_ADMIN_RECEIPT_V1_SCHEMA, "local administration receipt",
    {
        /// Stable identity retained on exact command replay.
        admin_receipt_id: LocalAdminReceiptIdV1,
        /// Immutable reference of the original administration command.
        admin_command_ref: ResidentRecordRefV1,
        /// Canonical digest of that exact command.
        admin_command_sha256: LocalAdminCommandDigest,
        /// Recorded outcome, never a synthetic replay state.
        disposition: LocalAdminDispositionV1,
        /// Records attested by this receipt, including its hard checkpoint but
        /// excluding the receipt itself, which cannot canonically self-reference.
        committed_records: Vec<LocalAdminCommittedRecordV1>,
        /// Every affected logical head, once.
        head_transitions: Vec<LocalAdminHeadTransitionV1>,
        /// Operation-owned failure code for rejected or conflicting requests.
        failure_code: Option<LocalAdminFailureCodeV1>,
        /// Claimed commit time in Unix seconds, present only for committed outcomes.
        committed_at: Option<UnixTimeV1>,
        /// Time the outcome was recorded, in Unix seconds.
        recorded_at: UnixTimeV1,
        /// Exact delegated runtime authority that signs this receipt.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Canonical identity of the selected runtime delegation.
        deployment_runtime_authority_sha256: DeploymentRuntimeAuthorityDigest,
        /// Principal of the selected runtime signer.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Delegated online runtime key.
        issuer_key_id: ResidentKeyIdV1,
        /// Delegated runtime subject-key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_receipt
);

/// Exact admitted dependencies and transaction result for one administration receipt.
///
/// The caller resolves the current runtime authority and supplies the transaction's
/// exact result. This context proves neither database commitment nor independent
/// anti-rollback anchors.
#[derive(Clone, Copy)]
pub struct LocalAdminReceiptAdmissionContextV1<'a> {
    /// Selected schema and owner-resolved integrated runtime signer binding.
    pub signed: SignedResidentAdmissionContextV1<'a>,
    /// Immutable reference of the authenticated command.
    pub admin_command_ref: &'a ResidentRecordRefV1,
    /// Exact authenticated command whose outcome is recorded.
    pub admin_command: &'a LocalAdminCommandV1,
    /// Current admitted runtime delegation used to sign the receipt.
    pub runtime_authority: &'a DeploymentRuntimeAuthorityV1,
    /// Stable receipt identity selected by the transaction owner.
    pub expected_admin_receipt_id: &'a LocalAdminReceiptIdV1,
    /// Exact recorded disposition.
    pub expected_disposition: LocalAdminDispositionV1,
    /// Exact records committed by the transaction, in transaction-owned order.
    pub expected_committed_records: &'a [LocalAdminCommittedRecordV1],
    /// Exact logical head transitions committed by the transaction.
    pub expected_head_transitions: &'a [LocalAdminHeadTransitionV1],
    /// Exact failure code for a non-committing outcome.
    pub expected_failure_code: Option<&'a LocalAdminFailureCodeV1>,
    /// Exact claimed commit time, absent for a non-committing outcome.
    pub expected_committed_at: Option<&'a UnixTimeV1>,
    /// Exact receipt recording time.
    pub expected_recorded_at: &'a UnixTimeV1,
}

impl LocalAdminReceiptV1 {
    /// Issue a receipt candidate under the exact deployment runtime key.
    ///
    /// The returned record is still only a candidate: it proves neither store
    /// commitment nor completion of any independent anti-rollback anchor.
    ///
    /// # Errors
    /// Rejects mismatched command, result, runtime authority, schema, validity,
    /// canonical digest or a signing failure.
    pub fn issue_contextual<F>(
        preimage: LocalAdminReceiptPreimageV1,
        context: &LocalAdminReceiptAdmissionContextV1<'_>,
        sign: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &LocalAdminReceiptPreimageV1,
            &LocalAdminReceiptDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        context.admin_command.verify()?;
        context.runtime_authority.verify()?;
        validate_receipt(&preimage)?;
        validate_receipt_context(&preimage, context)?;
        let digest = LocalAdminReceiptDigest::hash_canonical(&preimage)?;
        let signature = sign(LOCAL_ADMIN_RECEIPT_V1_SIGNATURE_DOMAIN, &preimage, &digest)?;
        Self::seal(preimage, signature)
    }

    /// Admit a receipt candidate under the exact runtime key and transaction result.
    ///
    /// `verify_signature` authenticates the runtime public key over the dedicated
    /// receipt domain followed by its canonical digest. No callback can waive the
    /// command, result, authority, schema, issuer or validity joins here.
    ///
    /// # Errors
    /// Rejects malformed or mismatched receipts and rejected runtime signatures.
    pub fn admit_contextual_wire<F>(
        wire: LocalAdminReceiptWireV1,
        context: &LocalAdminReceiptAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &Ed25519PublicKeyV1,
            &[u8],
            &LocalAdminReceiptPreimageV1,
            &LocalAdminReceiptDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        context.admin_command.verify()?;
        context.runtime_authority.verify()?;
        Self::admit_wire(wire, |value, digest, signature| {
            validate_receipt_context(value, context)?;
            verify_signature(
                context.runtime_authority.subject_public_key(),
                LOCAL_ADMIN_RECEIPT_V1_SIGNATURE_DOMAIN,
                value,
                digest,
                signature,
            )
        })
    }
}

impl LocalAdminReceiptWireV1 {
    /// Check outcome fields and canonical digest without authenticating the issuer.
    ///
    /// This consumes untrusted wire data and returns no admitted record.
    /// A successful check proves neither the selected schema digest, signature,
    /// issuer authority, command association nor a store commit.
    ///
    /// # Errors
    /// Rejects inconsistent outcomes, oversized or duplicate sets, incomplete
    /// head coordinates, missing checkpoint linkage and changed canonical bytes.
    pub fn verify_structure(self) -> Result<(), InvalidResidentRecordV1> {
        bounded_len(
            "administration committed records",
            self.committed_records.len(),
        )?;
        bounded_len(
            "administration head transitions",
            self.head_transitions.len(),
        )?;
        LocalAdminReceiptV1::admit_wire(self, |_, _, _| Ok(())).map(drop)
    }
}

fn validate_receipt(value: &LocalAdminReceiptPreimageV1) -> Result<(), InvalidResidentRecordV1> {
    bounded_len(
        "administration committed records",
        value.committed_records.len(),
    )?;
    bounded_len(
        "administration head transitions",
        value.head_transitions.len(),
    )?;
    match value.disposition {
        LocalAdminDispositionV1::Committed => {
            if value.committed_records.is_empty()
                || value.head_transitions.is_empty()
                || value.failure_code.is_some()
                || value.committed_at.is_none_or(|at| at > value.recorded_at)
            {
                return Err(contextual(
                    "local administration receipt",
                    "committed records, heads and time without a failure code",
                ));
            }
        }
        LocalAdminDispositionV1::Rejected | LocalAdminDispositionV1::Conflict => {
            if value.failure_code.is_none()
                || !value.committed_records.is_empty()
                || !value.head_transitions.is_empty()
                || value.committed_at.is_some()
            {
                return Err(contextual(
                    "local administration receipt",
                    "failure code without committed fields",
                ));
            }
        }
    }
    let mut references = BTreeSet::new();
    for record in &value.committed_records {
        if !references.insert(&record.committed_record_ref) {
            return Err(contextual(
                "local administration receipt",
                "unique committed record references",
            ));
        }
    }
    let mut checkpoint_linked = false;
    for (index, head) in value.head_transitions.iter().enumerate() {
        if head.previous_head_ref.is_some() != head.previous_head_sha256.is_some()
            || head.new_head_ref.is_some() != head.new_head_sha256.is_some()
            || (head.previous_head_ref.is_none() && head.new_head_ref.is_none())
            || value.head_transitions[..index]
                .iter()
                .any(|prior| prior.head_kind.same_slot(&head.head_kind))
        {
            return Err(contextual(
                "local administration receipt",
                "paired head coordinates and unique affected slots",
            ));
        }
        if matches!(
            head.head_kind,
            LocalAdminHeadSelectorV1::ControlCheckpoint { .. }
        ) {
            if head.previous_head_ref == head.new_head_ref
                && head.previous_head_sha256 == head.new_head_sha256
            {
                return Err(contextual(
                    "local administration receipt",
                    "affected checkpoint head advances to a successor",
                ));
            }
            let linked = value.committed_records.iter().any(|record| {
                head.new_head_ref.as_ref() == Some(&record.committed_record_ref)
                    && head.new_head_sha256.as_ref() == Some(&record.committed_record_sha256)
            });
            if !linked {
                return Err(contextual(
                    "local administration receipt",
                    "each affected checkpoint head names its committed record",
                ));
            }
            checkpoint_linked = true;
        }
    }
    if value.disposition == LocalAdminDispositionV1::Committed && !checkpoint_linked {
        return Err(contextual(
            "local administration receipt",
            "committed hard checkpoint linked through its affected domain head",
        ));
    }
    Ok(())
}

fn validate_receipt_context(
    value: &LocalAdminReceiptPreimageV1,
    context: &LocalAdminReceiptAdmissionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    const FAMILY: &str = "local administration receipt";
    validate_signed_coordinates(
        FAMILY,
        &value.schema_sha256,
        context.signed.expected_schema_sha256,
        &value.issuer_principal_id,
        &value.issuer_key_id,
        value.issuer_key_epoch,
        context.signed.signing_authority,
    )?;
    let signing_digest = ArtifactDigest::parse(value.deployment_runtime_authority_sha256.as_str())
        .map_err(|_| contextual(FAMILY, "runtime authority digest representation"))?;
    validate_integrated_runtime_authority_coordinates(
        FAMILY,
        &value.deployment_runtime_authority_ref,
        &signing_digest,
        context.signed.signing_authority,
    )?;
    let command = context.admin_command;
    let runtime = context.runtime_authority;
    if value.admin_receipt_id != *context.expected_admin_receipt_id
        || value.admin_command_ref != *context.admin_command_ref
        || value.admin_command_sha256 != *command.canonical_admin_command_sha256()
        || value.disposition != context.expected_disposition
        || value.committed_records != context.expected_committed_records
        || value.head_transitions != context.expected_head_transitions
        || value.failure_code.as_ref() != context.expected_failure_code
        || value.committed_at.as_ref() != context.expected_committed_at
        || value.recorded_at != *context.expected_recorded_at
        || runtime.canonical_deployment_runtime_authority_sha256()
            != &value.deployment_runtime_authority_sha256
        || runtime.principal_id() != &value.issuer_principal_id
        || runtime.subject_key_id() != &value.issuer_key_id
        || runtime.subject_key_epoch() != &value.issuer_key_epoch
        || runtime.state() != &PrincipalAuthorityStateV1::Active
        || value.recorded_at < *command.issued_at()
        || value
            .committed_at
            .is_some_and(|at| at < *command.issued_at())
        || value.recorded_at < *runtime.issued_at()
        || value.recorded_at >= *runtime.expires_at()
        || value
            .committed_at
            .is_some_and(|at| at < *runtime.issued_at() || at >= *runtime.expires_at())
        || !runtime
            .permitted_record_schema_ids()
            .contains(&value.schema_id)
    {
        return Err(contextual(
            FAMILY,
            "exact command, transaction result and deployment runtime authority",
        ));
    }
    Ok(())
}
