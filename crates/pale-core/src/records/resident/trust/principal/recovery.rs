//! Principal recovery-policy succession and threshold recovery proofs.
//!
//! A recovery proof is an immutable, signature-verified statement about one
//! exact successor key. Its unsigned identity excludes the complete signature
//! collection. [`PrincipalRecoveryProofV1`] owns the threshold, eligibility,
//! and common-digest law.
//!
//! Admission verifies that finite relation but does not reserve or consume the
//! proof nonce. One-use custody belongs to the durable principal trust store.

use std::collections::BTreeSet;

use super::*;

/// Public schema root for principal-recovery-proof wires.
pub const PRINCIPAL_RECOVERY_PROOF_V1_SCHEMA_ID: &str = "principal-recovery-proof.v1";
/// Wire schema carried by every principal recovery proof.
pub const PRINCIPAL_RECOVERY_PROOF_V1_SCHEMA: &str = "pale/principal-recovery-proof.v1";
/// Signature domain for every eligible signer of [`PrincipalRecoveryProofV1`].
pub const PRINCIPAL_RECOVERY_PROOF_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_PRINCIPAL_RECOVERY_PROOF_V1\0";

/// One threshold signature carried by a [`PrincipalRecoveryProofV1`].
///
/// The signer coordinate resolves its public key from the exact pinned
/// [`PrincipalRecoveryPolicyV1`]. Array order grants no meaning; admission
/// treats the collection as a finite set and rejects duplicate coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrincipalRecoverySignatureV1 {
    /// Principal that owns the eligible recovery key.
    signer_principal_id: ResidentPrincipalIdV1,
    /// Stable recovery-key identity within that principal.
    signer_key_id: ResidentKeyIdV1,
    /// Exact eligible recovery-key epoch.
    signer_key_epoch: PrincipalKeyEpochV1,
    /// Canonical proof identity attested by this signature.
    signed_record_sha256: PrincipalRecoveryProofDigest,
    /// Ed25519 signature over the proof domain and canonical proof identity.
    signature: CanonicalSignatureV1,
}

impl PrincipalRecoverySignatureV1 {
    /// Borrow the principal that owns the signing recovery key.
    #[must_use]
    pub fn signer_principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.signer_principal_id
    }

    /// Borrow the stable signing-key identity.
    #[must_use]
    pub fn signer_key_id(&self) -> &ResidentKeyIdV1 {
        &self.signer_key_id
    }

    /// Return the exact signing-key epoch.
    #[must_use]
    pub const fn signer_key_epoch(&self) -> PrincipalKeyEpochV1 {
        self.signer_key_epoch
    }

    /// Borrow the canonical proof identity attested by the signer.
    #[must_use]
    pub fn signed_record_sha256(&self) -> &PrincipalRecoveryProofDigest {
        &self.signed_record_sha256
    }

    /// Borrow the opaque Ed25519 signature.
    #[must_use]
    pub fn signature(&self) -> &CanonicalSignatureV1 {
        &self.signature
    }
}

/// Canonical unsigned preimage of [`PrincipalRecoveryProofV1`].
///
/// The signature collection is deliberately absent. Every threshold signer
/// signs the digest of this exact value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrincipalRecoveryProofPreimageV1 {
    /// Stable initial-v1 schema identity.
    pub schema_id: SchemaId,
    /// SHA-256 of the exact selected schema bytes.
    pub schema_sha256: SchemaDigest,
    /// Stable identity of this recovery proof.
    pub recovery_proof_id: PrincipalRecoveryProofIdV1,
    /// Principal whose active key is being recovered.
    pub principal_id: ResidentPrincipalIdV1,
    /// Immutable reference to the exact predecessor principal record.
    pub prior_principal_record_ref: ResidentRecordRefV1,
    /// Canonical digest of that predecessor principal record.
    pub prior_principal_record_sha256: PrincipalRecordDigest,
    /// Immutable reference to the pinned recovery policy.
    pub recovery_policy_ref: ResidentRecordRefV1,
    /// Canonical digest of the pinned recovery policy.
    pub recovery_policy_sha256: PrincipalRecoveryPolicyDigest,
    /// Stable identity of the proposed replacement key.
    pub new_key_id: ResidentKeyIdV1,
    /// Exact proposed replacement Ed25519 public key.
    pub new_public_key: Ed25519PublicKeyV1,
    /// Exact successor principal-key epoch.
    pub new_key_epoch: PrincipalKeyEpochV1,
    /// One-use nonce whose durable consumption is trust-store owned.
    pub nonce: PrincipalRecoveryNonceV1,
    /// Unix issue time in whole seconds.
    pub issued_at: UnixTimeV1,
    /// Unix expiry time in whole seconds.
    pub expires_at: UnixTimeV1,
}

/// Signature-verified principal recovery proof.
///
/// Construction is available only through [`Self::admit_wire`]. The value
/// proves structural, contextual, threshold, digest, and signature admission;
/// it does not prove durable nonce consumption or head mutation.
///
/// In the law below, `S` is the finite set of distinct supplied recovery
/// signatures, `E` is the exact eligible-signer set from the pinned policy,
/// and `t` is that policy's threshold. `P_unsigned` is the canonical proof
/// preimage without signatures and `H` is its domain-separated digest. Every
/// admitted signer is eligible and attests that same digest; admission still
/// does not reserve the nonce or advance the principal head.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/principal-recovery.display.mathml"
))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(transform = project_recovery_proof_schema_v1)]
#[serde(deny_unknown_fields)]
pub struct PrincipalRecoveryProofV1 {
    #[schemars(with = "String", extend("const" = PRINCIPAL_RECOVERY_PROOF_V1_SCHEMA))]
    schema_id: SchemaId,
    schema_sha256: SchemaDigest,
    recovery_proof_id: PrincipalRecoveryProofIdV1,
    principal_id: ResidentPrincipalIdV1,
    prior_principal_record_ref: ResidentRecordRefV1,
    prior_principal_record_sha256: PrincipalRecordDigest,
    recovery_policy_ref: ResidentRecordRefV1,
    recovery_policy_sha256: PrincipalRecoveryPolicyDigest,
    new_key_id: ResidentKeyIdV1,
    new_public_key: Ed25519PublicKeyV1,
    new_key_epoch: PrincipalKeyEpochV1,
    nonce: PrincipalRecoveryNonceV1,
    issued_at: UnixTimeV1,
    expires_at: UnixTimeV1,
    canonical_record_sha256: PrincipalRecoveryProofDigest,
    signatures: Vec<PrincipalRecoverySignatureV1>,
}

/// Untrusted flat wire shape for [`PrincipalRecoveryProofV1`].
///
/// Deserialization proves field shape only. Use
/// [`PrincipalRecoveryProofV1::admit_wire`] with owner-resolved predecessor and
/// policy context before using the proof in a key transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(transform = project_recovery_proof_schema_v1)]
#[serde(deny_unknown_fields)]
pub struct PrincipalRecoveryProofWireV1 {
    schema_id: SchemaId,
    schema_sha256: SchemaDigest,
    recovery_proof_id: PrincipalRecoveryProofIdV1,
    principal_id: ResidentPrincipalIdV1,
    prior_principal_record_ref: ResidentRecordRefV1,
    prior_principal_record_sha256: PrincipalRecordDigest,
    recovery_policy_ref: ResidentRecordRefV1,
    recovery_policy_sha256: PrincipalRecoveryPolicyDigest,
    new_key_id: ResidentKeyIdV1,
    new_public_key: Ed25519PublicKeyV1,
    new_key_epoch: PrincipalKeyEpochV1,
    nonce: PrincipalRecoveryNonceV1,
    issued_at: UnixTimeV1,
    expires_at: UnixTimeV1,
    canonical_record_sha256: PrincipalRecoveryProofDigest,
    signatures: Vec<PrincipalRecoverySignatureV1>,
}

impl_resident_wire_deserialize!(PrincipalRecoveryProofWireV1 {
    schema_id: SchemaId,
    schema_sha256: SchemaDigest,
    recovery_proof_id: PrincipalRecoveryProofIdV1,
    principal_id: ResidentPrincipalIdV1,
    prior_principal_record_ref: ResidentRecordRefV1,
    prior_principal_record_sha256: PrincipalRecordDigest,
    recovery_policy_ref: ResidentRecordRefV1,
    recovery_policy_sha256: PrincipalRecoveryPolicyDigest,
    new_key_id: ResidentKeyIdV1,
    new_public_key: Ed25519PublicKeyV1,
    new_key_epoch: PrincipalKeyEpochV1,
    nonce: PrincipalRecoveryNonceV1,
    issued_at: UnixTimeV1,
    expires_at: UnixTimeV1,
    canonical_record_sha256: PrincipalRecoveryProofDigest,
    signatures: Vec<PrincipalRecoverySignatureV1>,
});

/// Owner-resolved context for one recovery-proof presentation.
///
/// `presented_at` is explicit input rather than ambient clock access. The
/// caller must supply the accepted predecessor and its exact active recovery
/// policy; constructing this context grants no authority by itself.
pub struct PrincipalRecoveryProofAdmissionContextV1<'a> {
    /// Digest of the exact selected recovery-proof schema bytes.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Immutable reference assigned to the active predecessor principal record.
    pub prior_principal_record_ref: &'a ResidentRecordRefV1,
    /// Active predecessor principal record.
    pub prior_principal_record: &'a PrincipalRecordV1,
    /// Immutable reference assigned to the active recovery policy.
    pub recovery_policy_ref: &'a ResidentRecordRefV1,
    /// Active recovery policy pinned by the predecessor record.
    pub recovery_policy: &'a PrincipalRecoveryPolicyV1,
    /// Explicit verification time used for proof-expiry admission.
    pub presented_at: UnixTimeV1,
}

impl PrincipalRecoveryProofV1 {
    /// Admit one threshold recovery proof against the exact active trust roots.
    ///
    /// The callback must verify the trust module's domain-separated signature
    /// preimage under the supplied policy-pinned public key. Signature array
    /// order is semantically irrelevant; exact membership and duplicate
    /// rejection are enforced as a finite set.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for schema, predecessor, policy,
    /// next-epoch, validity-window, threshold, eligibility, digest, duplicate,
    /// or signature-verification failure.
    pub fn admit_wire<F>(
        wire: PrincipalRecoveryProofWireV1,
        context: &PrincipalRecoveryProofAdmissionContextV1<'_>,
        mut verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnMut(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalRecoveryProofPreimageV1,
            &PrincipalRecoveryProofDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        context.prior_principal_record.verify()?;
        context.recovery_policy.verify()?;
        let preimage = wire.preimage();
        validate_recovery_proof_context(&preimage, context)?;
        let digest = PrincipalRecoveryProofDigest::hash_canonical(&preimage)?;
        if digest != wire.canonical_record_sha256 {
            return Err(InvalidResidentRecordV1::DigestMismatch {
                family: "principal recovery proof",
            });
        }
        let mut signatures = wire.signatures;
        validate_recovery_signatures(
            &signatures,
            &digest,
            context.recovery_policy,
            |public_key, signature| {
                verify_signature(
                    PRINCIPAL_RECOVERY_PROOF_V1_SIGNATURE_DOMAIN,
                    public_key,
                    &preimage,
                    &digest,
                    signature,
                )
            },
        )?;
        signatures.sort_by(|left, right| {
            (
                &left.signer_principal_id,
                &left.signer_key_id,
                left.signer_key_epoch,
            )
                .cmp(&(
                    &right.signer_principal_id,
                    &right.signer_key_id,
                    right.signer_key_epoch,
                ))
        });
        Ok(Self::seal(preimage, signatures, digest))
    }

    /// Revalidate the structural preimage and canonical record identity.
    ///
    /// This does not repeat contextual signature verification and does not
    /// prove that the nonce has been durably consumed.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the admitted value no longer
    /// reproduces its canonical unsigned preimage or bounded signature shape.
    pub fn verify(&self) -> Result<(), InvalidResidentRecordV1> {
        bounded_len("principal recovery proof signatures", self.signatures.len())?;
        if self.signatures.is_empty()
            || PrincipalRecoveryProofDigest::hash_canonical(&self.preimage())?
                != self.canonical_record_sha256
        {
            return Err(InvalidResidentRecordV1::DigestMismatch {
                family: "principal recovery proof",
            });
        }
        Ok(())
    }

    fn seal(
        preimage: PrincipalRecoveryProofPreimageV1,
        signatures: Vec<PrincipalRecoverySignatureV1>,
        canonical_record_sha256: PrincipalRecoveryProofDigest,
    ) -> Self {
        let PrincipalRecoveryProofPreimageV1 {
            schema_id,
            schema_sha256,
            recovery_proof_id,
            principal_id,
            prior_principal_record_ref,
            prior_principal_record_sha256,
            recovery_policy_ref,
            recovery_policy_sha256,
            new_key_id,
            new_public_key,
            new_key_epoch,
            nonce,
            issued_at,
            expires_at,
        } = preimage;
        Self {
            schema_id,
            schema_sha256,
            recovery_proof_id,
            principal_id,
            prior_principal_record_ref,
            prior_principal_record_sha256,
            recovery_policy_ref,
            recovery_policy_sha256,
            new_key_id,
            new_public_key,
            new_key_epoch,
            nonce,
            issued_at,
            expires_at,
            canonical_record_sha256,
            signatures,
        }
    }

    fn preimage(&self) -> PrincipalRecoveryProofPreimageV1 {
        PrincipalRecoveryProofPreimageV1 {
            schema_id: self.schema_id.clone(),
            schema_sha256: self.schema_sha256.clone(),
            recovery_proof_id: self.recovery_proof_id.clone(),
            principal_id: self.principal_id.clone(),
            prior_principal_record_ref: self.prior_principal_record_ref.clone(),
            prior_principal_record_sha256: self.prior_principal_record_sha256.clone(),
            recovery_policy_ref: self.recovery_policy_ref.clone(),
            recovery_policy_sha256: self.recovery_policy_sha256.clone(),
            new_key_id: self.new_key_id.clone(),
            new_public_key: self.new_public_key.clone(),
            new_key_epoch: self.new_key_epoch,
            nonce: self.nonce.clone(),
            issued_at: self.issued_at,
            expires_at: self.expires_at,
        }
    }

    /// Borrow the stable recovery-proof identity.
    #[must_use]
    pub fn recovery_proof_id(&self) -> &PrincipalRecoveryProofIdV1 {
        &self.recovery_proof_id
    }

    /// Borrow the principal whose key this proof replaces.
    #[must_use]
    pub fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }

    /// Borrow the exact predecessor principal-record reference.
    #[must_use]
    pub fn prior_principal_record_ref(&self) -> &ResidentRecordRefV1 {
        &self.prior_principal_record_ref
    }

    /// Borrow the exact predecessor principal-record digest.
    #[must_use]
    pub fn prior_principal_record_sha256(&self) -> &PrincipalRecordDigest {
        &self.prior_principal_record_sha256
    }

    /// Borrow the exact pinned recovery-policy reference.
    #[must_use]
    pub fn recovery_policy_ref(&self) -> &ResidentRecordRefV1 {
        &self.recovery_policy_ref
    }

    /// Borrow the exact pinned recovery-policy digest.
    #[must_use]
    pub fn recovery_policy_sha256(&self) -> &PrincipalRecoveryPolicyDigest {
        &self.recovery_policy_sha256
    }

    /// Borrow the replacement key identity.
    #[must_use]
    pub fn new_key_id(&self) -> &ResidentKeyIdV1 {
        &self.new_key_id
    }

    /// Borrow the replacement Ed25519 public key.
    #[must_use]
    pub fn new_public_key(&self) -> &Ed25519PublicKeyV1 {
        &self.new_public_key
    }

    /// Return the exact successor principal-key epoch.
    #[must_use]
    pub const fn new_key_epoch(&self) -> PrincipalKeyEpochV1 {
        self.new_key_epoch
    }

    /// Borrow the one-use recovery nonce.
    #[must_use]
    pub fn nonce(&self) -> &PrincipalRecoveryNonceV1 {
        &self.nonce
    }

    /// Return the proof issue time.
    #[must_use]
    pub const fn issued_at(&self) -> UnixTimeV1 {
        self.issued_at
    }

    /// Return the proof expiry time.
    #[must_use]
    pub const fn expires_at(&self) -> UnixTimeV1 {
        self.expires_at
    }

    /// Borrow the canonical unsigned proof identity.
    #[must_use]
    pub fn canonical_record_sha256(&self) -> &PrincipalRecoveryProofDigest {
        &self.canonical_record_sha256
    }

    /// Borrow the exact threshold signature collection.
    #[must_use]
    pub fn signatures(&self) -> &[PrincipalRecoverySignatureV1] {
        &self.signatures
    }
}

impl PrincipalRecoveryProofWireV1 {
    fn preimage(&self) -> PrincipalRecoveryProofPreimageV1 {
        PrincipalRecoveryProofPreimageV1 {
            schema_id: self.schema_id.clone(),
            schema_sha256: self.schema_sha256.clone(),
            recovery_proof_id: self.recovery_proof_id.clone(),
            principal_id: self.principal_id.clone(),
            prior_principal_record_ref: self.prior_principal_record_ref.clone(),
            prior_principal_record_sha256: self.prior_principal_record_sha256.clone(),
            recovery_policy_ref: self.recovery_policy_ref.clone(),
            recovery_policy_sha256: self.recovery_policy_sha256.clone(),
            new_key_id: self.new_key_id.clone(),
            new_public_key: self.new_public_key.clone(),
            new_key_epoch: self.new_key_epoch,
            nonce: self.nonce.clone(),
            issued_at: self.issued_at,
            expires_at: self.expires_at,
        }
    }
}

impl ResidentSchemaIdentityV1 for PrincipalRecoveryProofV1 {
    const SCHEMA_FIELD: &'static str = "schema_id";
    const SCHEMA: &'static str = PRINCIPAL_RECOVERY_PROOF_V1_SCHEMA;
    const VERSION: Option<(&'static str, u32)> = None;
}

impl ResidentSchemaIdentityV1 for PrincipalRecoveryProofWireV1 {
    const SCHEMA_FIELD: &'static str = "schema_id";
    const SCHEMA: &'static str = PRINCIPAL_RECOVERY_PROOF_V1_SCHEMA;
    const VERSION: Option<(&'static str, u32)> = None;
}

impl PrincipalRecoveryPolicyV1 {
    /// Admit the exact same-lineage successor recovery policy.
    ///
    /// The predecessor remains historical evidence. Returning the successor
    /// does not publish a policy head; the caller must pair it with the exact
    /// [`PrincipalRecordTransitionKindV1::PolicyUpdate`] successor in one
    /// owner-controlled transaction.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for wrong schema, principal,
    /// lineage, active-policy binding, predecessor, next epoch, active-key
    /// signer, local validity interval, digest, or signature failure.
    pub fn admit_successor_wire<F>(
        wire: PrincipalRecoveryPolicyWireV1,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &Self,
        active_principal: &PrincipalRecordV1,
        expected_schema_sha256: &SchemaDigest,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalRecoveryPolicyPreimageV1,
            &PrincipalRecoveryPolicyDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        predecessor.verify()?;
        active_principal.verify()?;
        Self::admit_wire(wire, |preimage, digest, signature| {
            let expected_epoch =
                predecessor
                    .policy_epoch()
                    .checked_successor()
                    .ok_or_else(|| {
                        contextual(
                            "principal recovery policy successor",
                            "non-exhausted next policy epoch",
                        )
                    })?;
            if preimage.schema_sha256 != *expected_schema_sha256
                || preimage.recovery_policy_id != *predecessor.recovery_policy_id()
                || preimage.principal_id != *predecessor.principal_id()
                || preimage.principal_id != *active_principal.principal_id()
                || active_principal.recovery_policy_ref() != predecessor_ref
                || active_principal.recovery_policy_sha256()
                    != predecessor.canonical_record_sha256()
                || preimage.policy_epoch != expected_epoch
                || preimage.previous_policy_ref.as_ref() != Some(predecessor_ref)
                || preimage.previous_policy_sha256.as_ref()
                    != Some(predecessor.canonical_record_sha256())
                || preimage.issuer_key_id != *active_principal.root_key_id()
                || preimage.issuer_principal_key_epoch != *active_principal.key_epoch()
            {
                return Err(contextual(
                    "principal recovery policy successor",
                    "exact lineage, active key, predecessor, and next epoch",
                ));
            }
            verify_signature(
                PRINCIPAL_RECOVERY_POLICY_V1_SIGNATURE_DOMAIN,
                active_principal.root_public_key(),
                preimage,
                digest,
                signature,
            )
        })
    }
}

fn validate_recovery_proof_context(
    preimage: &PrincipalRecoveryProofPreimageV1,
    context: &PrincipalRecoveryProofAdmissionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    let principal = context.prior_principal_record;
    let policy = context.recovery_policy;
    let expected_epoch = principal.key_epoch().checked_successor().ok_or_else(|| {
        contextual(
            "principal recovery proof",
            "non-exhausted successor key epoch",
        )
    })?;
    let lifetime = preimage
        .expires_at
        .get()
        .checked_sub(preimage.issued_at.get())
        .filter(|lifetime| *lifetime > 0)
        .ok_or_else(|| contextual("principal recovery proof", "positive validity interval"))?;
    let policy_active_at_issue = preimage.issued_at >= *policy.issued_at()
        && policy
            .expires_at()
            .is_none_or(|expires_at| preimage.issued_at < expires_at);
    let presented_in_window =
        context.presented_at >= preimage.issued_at && context.presented_at < preimage.expires_at;
    if preimage.schema_id.as_str() != PRINCIPAL_RECOVERY_PROOF_V1_SCHEMA
        || preimage.schema_sha256 != *context.expected_schema_sha256
        || preimage.principal_id != *principal.principal_id()
        || preimage.prior_principal_record_ref != *context.prior_principal_record_ref
        || preimage.prior_principal_record_sha256 != *principal.canonical_record_sha256()
        || preimage.recovery_policy_ref != *context.recovery_policy_ref
        || preimage.recovery_policy_ref != *principal.recovery_policy_ref()
        || preimage.recovery_policy_sha256 != *policy.canonical_record_sha256()
        || preimage.recovery_policy_sha256 != *principal.recovery_policy_sha256()
        || preimage.principal_id != *policy.principal_id()
        || policy.replay_scope() != &PrincipalRecoveryReplayScopeV1::PrincipalKeyRecoveryProof
        || preimage.new_key_epoch != expected_epoch
        || lifetime > policy.proof_validity_seconds().get()
        || !policy_active_at_issue
        || !presented_in_window
    {
        Err(contextual(
            "principal recovery proof",
            "exact active predecessor, pinned policy, next key, and validity window",
        ))
    } else {
        Ok(())
    }
}

fn validate_recovery_signatures<F>(
    signatures: &[PrincipalRecoverySignatureV1],
    digest: &PrincipalRecoveryProofDigest,
    policy: &PrincipalRecoveryPolicyV1,
    mut verify_signature: F,
) -> Result<(), InvalidResidentRecordV1>
where
    F: FnMut(&Ed25519PublicKeyV1, &CanonicalSignatureV1) -> Result<(), InvalidResidentRecordV1>,
{
    bounded_len("principal recovery proof signatures", signatures.len())?;
    if signatures.len() != usize::from(*policy.threshold()) {
        return Err(contextual(
            "principal recovery proof",
            "exact recovery threshold",
        ));
    }
    let mut seen = BTreeSet::new();
    for signer in signatures {
        let coordinate = (
            signer.signer_principal_id(),
            signer.signer_key_id(),
            signer.signer_key_epoch(),
        );
        if !seen.insert(coordinate) || signer.signed_record_sha256() != digest {
            return Err(contextual(
                "principal recovery proof",
                "distinct signer coordinates over one proof digest",
            ));
        }
        let eligible = policy
            .eligible_signers()
            .iter()
            .find(|eligible| {
                eligible.recovery_principal_id == *signer.signer_principal_id()
                    && eligible.recovery_key_id == *signer.signer_key_id()
                    && eligible.recovery_key_epoch == signer.signer_key_epoch()
            })
            .ok_or_else(|| contextual("principal recovery proof", "eligible recovery signer"))?;
        verify_signature(&eligible.recovery_public_key, signer.signature())?;
    }
    Ok(())
}

fn project_recovery_proof_schema_v1(schema: &mut schemars::Schema) {
    close_resident_schema_identity_v1::<PrincipalRecoveryProofWireV1>(schema);
    project_resident_set_property_v1(schema, "signatures", Some(1));
}
