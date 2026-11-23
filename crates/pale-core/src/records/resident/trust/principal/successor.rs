//! Exact principal-record successor admission.
//!
//! Successor records carry one explicit branch. Key succession consumes an
//! already verified [`PrincipalKeyTransitionV1`]; policy succession admits the
//! policy and same-key principal record as one in-memory pair. Neither result
//! is a durable head or checkpoint transaction.

use super::*;

/// Purely admitted same-key recovery-policy update pair.
///
/// The pair cannot be constructed with only the new policy, preventing a
/// caller from mistaking a separately verified policy record for an admitted
/// principal trust head. Storage, compare-and-swap, checkpoint, and replay
/// custody remain outside this type.
pub struct AdmittedPrincipalPolicyUpdateV1 {
    recovery_policy_ref: ResidentRecordRefV1,
    recovery_policy: PrincipalRecoveryPolicyV1,
    principal_record: PrincipalRecordV1,
}

/// Untrusted record pair presented for one same-key policy update.
///
/// Grouping the two wires and their schema/reference coordinates keeps the
/// aggregate admission boundary explicit without introducing a generic batch
/// abstraction.
pub struct PrincipalPolicyUpdateCandidateV1<'schema> {
    /// Signed but not yet admitted recovery-policy successor.
    pub policy_wire: PrincipalRecoveryPolicyWireV1,
    /// Immutable reference assigned to that policy successor.
    pub recovery_policy_ref: ResidentRecordRefV1,
    /// Expected schema digest for the policy wire.
    pub expected_policy_schema_sha256: &'schema SchemaDigest,
    /// Signed but not yet admitted principal-record successor.
    pub principal_wire: PrincipalRecordWireV1,
    /// Expected schema digest for the principal-record wire.
    pub expected_principal_schema_sha256: &'schema SchemaDigest,
    /// Immutable reference assigned to the active predecessor policy.
    pub predecessor_policy_ref: &'schema ResidentRecordRefV1,
}

impl PrincipalRecordV1 {
    /// Admit the exact principal-record successor of one key transition.
    ///
    /// The successor binds the predecessor and transition digests, adopts the
    /// transition's new key coordinate, preserves the active recovery policy,
    /// and is signed by that new key. Display hints and record-expiry policy are
    /// intentionally not inferred across records; their owner has not frozen a
    /// cross-record preservation law.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for wrong schema, branch,
    /// predecessor, transition, key, recovery policy, digest, or new-key
    /// signature failure.
    pub fn admit_key_transition_successor_wire<F>(
        wire: PrincipalRecordWireV1,
        expected_schema_sha256: &SchemaDigest,
        predecessor: &Self,
        transition_ref: &ResidentRecordRefV1,
        transition: &PrincipalKeyTransitionV1,
        verify_new_key_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalRecordPreimageV1,
            &PrincipalRecordDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        predecessor.verify()?;
        transition.verify()?;
        Self::admit_wire(wire, |preimage, digest, signature| {
            if preimage.schema_sha256 != *expected_schema_sha256
                || preimage.record_transition_kind != PrincipalRecordTransitionKindV1::KeyTransition
                || preimage.principal_id != *predecessor.principal_id()
                || preimage.previous_record_sha256.as_ref()
                    != Some(predecessor.canonical_record_sha256())
                || preimage.key_transition_ref.as_ref() != Some(transition_ref)
                || preimage.key_transition_sha256.as_ref()
                    != Some(transition.canonical_record_sha256())
                || transition.principal_id() != predecessor.principal_id()
                || transition.previous_record_sha256() != predecessor.canonical_record_sha256()
                || preimage.root_key_id != *transition.new_key_id()
                || preimage.root_public_key != *transition.new_public_key()
                || preimage.key_epoch != transition.new_key_epoch()
                || preimage.recovery_policy_ref != *predecessor.recovery_policy_ref()
                || preimage.recovery_policy_sha256 != *predecessor.recovery_policy_sha256()
            {
                return Err(contextual(
                    "principal record key successor",
                    "exact predecessor, transition, next key, and recovery policy",
                ));
            }
            verify_new_key_signature(
                PRINCIPAL_RECORD_V1_SIGNATURE_DOMAIN,
                transition.new_public_key(),
                preimage,
                digest,
                signature,
            )
        })
    }
}

impl AdmittedPrincipalPolicyUpdateV1 {
    /// Admit one recovery-policy successor and its exact same-key principal record.
    ///
    /// Both signature callbacks authenticate their supplied domain and digest
    /// under the active principal key. The function is a pure aggregate
    /// admission; it does not commit either record or advance a checkpoint.
    /// Display hints and record-expiry policy remain owner inputs; no
    /// cross-record preservation law is inferred for either field.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when either record fails its schema,
    /// lineage, predecessor, next-epoch, same-key, digest, or signature law, or
    /// when the principal record does not bind the exact new policy.
    pub fn admit<PolicySignature, PrincipalSignature>(
        candidate: PrincipalPolicyUpdateCandidateV1<'_>,
        predecessor_principal: &PrincipalRecordV1,
        predecessor_policy: &PrincipalRecoveryPolicyV1,
        verify_policy_signature: PolicySignature,
        verify_principal_signature: PrincipalSignature,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        PolicySignature: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalRecoveryPolicyPreimageV1,
            &PrincipalRecoveryPolicyDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        PrincipalSignature: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalRecordPreimageV1,
            &PrincipalRecordDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let PrincipalPolicyUpdateCandidateV1 {
            policy_wire,
            recovery_policy_ref,
            expected_policy_schema_sha256,
            principal_wire,
            expected_principal_schema_sha256,
            predecessor_policy_ref,
        } = candidate;
        let recovery_policy = PrincipalRecoveryPolicyV1::admit_successor_wire(
            policy_wire,
            predecessor_policy_ref,
            predecessor_policy,
            predecessor_principal,
            expected_policy_schema_sha256,
            verify_policy_signature,
        )?;
        let principal_record =
            PrincipalRecordV1::admit_wire(principal_wire, |preimage, digest, signature| {
                if preimage.schema_sha256 != *expected_principal_schema_sha256
                    || preimage.record_transition_kind
                        != PrincipalRecordTransitionKindV1::PolicyUpdate
                    || preimage.principal_id != *predecessor_principal.principal_id()
                    || preimage.previous_record_sha256.as_ref()
                        != Some(predecessor_principal.canonical_record_sha256())
                    || preimage.root_key_id != *predecessor_principal.root_key_id()
                    || preimage.root_public_key != *predecessor_principal.root_public_key()
                    || preimage.key_epoch != *predecessor_principal.key_epoch()
                    || preimage.recovery_policy_ref != recovery_policy_ref
                    || preimage.recovery_policy_sha256 != *recovery_policy.canonical_record_sha256()
                    || recovery_policy.principal_id() != predecessor_principal.principal_id()
                {
                    return Err(contextual(
                        "principal record policy successor",
                        "exact predecessor, same key, and successor recovery policy",
                    ));
                }
                verify_principal_signature(
                    PRINCIPAL_RECORD_V1_SIGNATURE_DOMAIN,
                    predecessor_principal.root_public_key(),
                    preimage,
                    digest,
                    signature,
                )
            })?;
        Ok(Self {
            recovery_policy_ref,
            recovery_policy,
            principal_record,
        })
    }

    /// Borrow the immutable successor recovery-policy reference.
    #[must_use]
    pub fn recovery_policy_ref(&self) -> &ResidentRecordRefV1 {
        &self.recovery_policy_ref
    }

    /// Borrow the admitted successor recovery policy.
    #[must_use]
    pub fn recovery_policy(&self) -> &PrincipalRecoveryPolicyV1 {
        &self.recovery_policy
    }

    /// Borrow the same-key principal-record successor that binds the policy.
    #[must_use]
    pub fn principal_record(&self) -> &PrincipalRecordV1 {
        &self.principal_record
    }
}
