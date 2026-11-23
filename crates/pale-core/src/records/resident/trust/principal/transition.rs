//! Principal-key transition records and signature-branch admission.
//!
//! A transition is a pure epoch successor over one exact predecessor;
//! [`PrincipalKeyTransitionV1`] owns the exact successor and predecessor-digest
//! law.
//!
//! Normal rotation requires both old- and new-key attestations over the same
//! canonical digest. Recovery replaces the old-key attestation with one exact
//! admitted [`PrincipalRecoveryProofV1`] and still requires the new key. This
//! module validates that algebra; it does not consume the proof or mutate the
//! principal head.

use super::*;

/// Public schema root for principal-key-transition wires.
pub const PRINCIPAL_KEY_TRANSITION_V1_SCHEMA_ID: &str = "principal-key-transition.v1";
/// Wire schema carried by every principal key transition.
pub const PRINCIPAL_KEY_TRANSITION_V1_SCHEMA: &str = "pale/principal-key-transition.v1";
/// Old-key signature domain for normal rotation.
pub const PRINCIPAL_KEY_TRANSITION_OLD_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_PRINCIPAL_KEY_TRANSITION_OLD_V1\0";
/// New-key signature domain shared by normal rotation and recovery.
pub const PRINCIPAL_KEY_TRANSITION_NEW_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_PRINCIPAL_KEY_TRANSITION_NEW_V1\0";

/// Closed principal-key transition mode.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalKeyTransitionModeV1 {
    /// Both the exact predecessor key and proposed successor key attest.
    NormalRotation,
    /// An admitted recovery proof replaces the unavailable predecessor signature.
    Recovery,
}

/// Canonical transition preimage signed by every required key.
///
/// Both key-signature members are absent by construction. Recovery proof
/// coordinates remain covered because they are dependency evidence rather
/// than record-owned signatures.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrincipalKeyTransitionPreimageV1 {
    /// Stable initial-v1 schema identity.
    pub schema_id: SchemaId,
    /// SHA-256 of the exact selected schema bytes.
    pub schema_sha256: SchemaDigest,
    /// Principal whose accepted key advances.
    pub principal_id: ResidentPrincipalIdV1,
    /// Explicit rotation or recovery branch.
    pub transition_mode: PrincipalKeyTransitionModeV1,
    /// Exact predecessor key epoch.
    pub old_key_epoch: PrincipalKeyEpochV1,
    /// Exact successor key epoch.
    pub new_key_epoch: PrincipalKeyEpochV1,
    /// Stable identity of the predecessor key.
    pub old_key_id: ResidentKeyIdV1,
    /// Stable identity of the successor key.
    pub new_key_id: ResidentKeyIdV1,
    /// Exact predecessor Ed25519 public key.
    pub old_public_key: Ed25519PublicKeyV1,
    /// Exact successor Ed25519 public key.
    pub new_public_key: Ed25519PublicKeyV1,
    /// Bounded operator-authored reason that grants no authority.
    pub transition_reason: BoundedResidentTextV1,
    /// Canonical digest of the exact predecessor principal record.
    pub previous_record_sha256: PrincipalRecordDigest,
    /// Recovery proof reference, present only for recovery.
    pub recovery_authority_proof_ref: Option<ResidentRecordRefV1>,
    /// Recovery proof digest paired with [`Self::recovery_authority_proof_ref`].
    pub recovery_authority_proof_sha256: Option<PrincipalRecoveryProofDigest>,
    /// Unix time at which the transition becomes effective.
    pub effective_at: UnixTimeV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PrincipalKeyTransitionAttestationV1 {
    NormalRotation {
        old_key_signature: CanonicalSignatureV1,
        new_key_signature: CanonicalSignatureV1,
    },
    Recovery {
        new_key_signature: CanonicalSignatureV1,
    },
}

/// Signature-verified principal-key transition.
///
/// The private attestation enum makes mixed rotation/recovery states
/// unrepresentable after [`Self::admit_normal_rotation_wire`] or
/// [`Self::admit_recovery_wire`] succeeds.
///
/// In the law below, `e_old` and `e_new` are the exact predecessor and
/// successor key epochs, `P_prior` is the admitted predecessor principal
/// record, `T` is this transition, and `H` is the canonical record digest.
/// Admission requires a one-step epoch successor and binds `T` to the exact
/// predecessor digest. It does not install the new key or mutate a trust-store
/// head.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/principal-transition.display.mathml"
))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalKeyTransitionV1 {
    preimage: PrincipalKeyTransitionPreimageV1,
    canonical_record_sha256: PrincipalKeyTransitionDigest,
    attestation: PrincipalKeyTransitionAttestationV1,
}

/// Untrusted flat wire shape for [`PrincipalKeyTransitionV1`].
///
/// Deserialization establishes only structural shape. The two public admission
/// functions resolve the exact predecessor, proof branch, and signing keys.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(transform = project_key_transition_schema_v1)]
#[serde(deny_unknown_fields)]
pub struct PrincipalKeyTransitionWireV1 {
    schema_id: SchemaId,
    schema_sha256: SchemaDigest,
    principal_id: ResidentPrincipalIdV1,
    transition_mode: PrincipalKeyTransitionModeV1,
    old_key_epoch: PrincipalKeyEpochV1,
    new_key_epoch: PrincipalKeyEpochV1,
    old_key_id: ResidentKeyIdV1,
    new_key_id: ResidentKeyIdV1,
    old_public_key: Ed25519PublicKeyV1,
    new_public_key: Ed25519PublicKeyV1,
    transition_reason: BoundedResidentTextV1,
    previous_record_sha256: PrincipalRecordDigest,
    old_key_signature: Option<CanonicalSignatureV1>,
    new_key_signature: CanonicalSignatureV1,
    recovery_authority_proof_ref: Option<ResidentRecordRefV1>,
    recovery_authority_proof_sha256: Option<PrincipalRecoveryProofDigest>,
    effective_at: UnixTimeV1,
    canonical_record_sha256: PrincipalKeyTransitionDigest,
}

impl_resident_wire_deserialize!(PrincipalKeyTransitionWireV1 {
    schema_id: SchemaId,
    schema_sha256: SchemaDigest,
    principal_id: ResidentPrincipalIdV1,
    transition_mode: PrincipalKeyTransitionModeV1,
    old_key_epoch: PrincipalKeyEpochV1,
    new_key_epoch: PrincipalKeyEpochV1,
    old_key_id: ResidentKeyIdV1,
    new_key_id: ResidentKeyIdV1,
    old_public_key: Ed25519PublicKeyV1,
    new_public_key: Ed25519PublicKeyV1,
    transition_reason: BoundedResidentTextV1,
    previous_record_sha256: PrincipalRecordDigest,
    old_key_signature: Option<CanonicalSignatureV1>,
    new_key_signature: CanonicalSignatureV1,
    recovery_authority_proof_ref: Option<ResidentRecordRefV1>,
    recovery_authority_proof_sha256: Option<PrincipalRecoveryProofDigest>,
    effective_at: UnixTimeV1,
    canonical_record_sha256: PrincipalKeyTransitionDigest,
});

/// Owner-resolved predecessor coordinate for principal-key transition admission.
pub struct PrincipalKeyTransitionAdmissionContextV1<'a> {
    /// Digest of the exact selected transition schema bytes.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Immutable reference assigned to the active predecessor record.
    ///
    /// Recovery admission compares this coordinate with the already-admitted
    /// proof; normal rotation has no proof reference to compare.
    pub predecessor_ref: &'a ResidentRecordRefV1,
    /// Active predecessor principal record.
    pub predecessor: &'a PrincipalRecordV1,
}

impl PrincipalKeyTransitionV1 {
    /// Admit a normal key rotation with old- and new-key signatures.
    ///
    /// Both callbacks must verify the supplied domain and the same canonical
    /// transition digest under their respective exact public keys.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for schema, branch, predecessor,
    /// key, next-epoch, digest, or either signature-verification failure.
    pub fn admit_normal_rotation_wire<Old, New>(
        wire: PrincipalKeyTransitionWireV1,
        context: &PrincipalKeyTransitionAdmissionContextV1<'_>,
        verify_old_signature: Old,
        verify_new_signature: New,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Old: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalKeyTransitionPreimageV1,
            &PrincipalKeyTransitionDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        New: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalKeyTransitionPreimageV1,
            &PrincipalKeyTransitionDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (preimage, digest) = validate_transition_wire(&wire, context)?;
        let PrincipalKeyTransitionWireV1 {
            old_key_signature,
            new_key_signature,
            recovery_authority_proof_ref,
            recovery_authority_proof_sha256,
            ..
        } = wire;
        let old_key_signature = match (
            preimage.transition_mode,
            old_key_signature,
            recovery_authority_proof_ref,
            recovery_authority_proof_sha256,
        ) {
            (PrincipalKeyTransitionModeV1::NormalRotation, Some(signature), None, None) => {
                signature
            }
            _ => {
                return Err(contextual(
                    "principal key transition",
                    "exclusive normal-rotation signature branch",
                ));
            }
        };
        verify_old_signature(
            PRINCIPAL_KEY_TRANSITION_OLD_V1_SIGNATURE_DOMAIN,
            &preimage.old_public_key,
            &preimage,
            &digest,
            &old_key_signature,
        )?;
        verify_new_signature(
            PRINCIPAL_KEY_TRANSITION_NEW_V1_SIGNATURE_DOMAIN,
            &preimage.new_public_key,
            &preimage,
            &digest,
            &new_key_signature,
        )?;
        Ok(Self {
            preimage,
            canonical_record_sha256: digest,
            attestation: PrincipalKeyTransitionAttestationV1::NormalRotation {
                old_key_signature,
                new_key_signature,
            },
        })
    }

    /// Admit a recovery transition bound to one exact verified recovery proof.
    ///
    /// The proof must name the same predecessor and proposed successor key. The
    /// callback verifies the new-key attestation; durable proof consumption and
    /// exact-replay disposition remain trust-store responsibilities.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for schema, branch, predecessor,
    /// proof, key, next-epoch, digest, or new-key signature failure.
    pub fn admit_recovery_wire<New>(
        wire: PrincipalKeyTransitionWireV1,
        context: &PrincipalKeyTransitionAdmissionContextV1<'_>,
        recovery_proof_ref: &ResidentRecordRefV1,
        recovery_proof: &PrincipalRecoveryProofV1,
        verify_new_signature: New,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        New: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalKeyTransitionPreimageV1,
            &PrincipalKeyTransitionDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        recovery_proof.verify()?;
        let (preimage, digest) = validate_transition_wire(&wire, context)?;
        let PrincipalKeyTransitionWireV1 {
            old_key_signature,
            new_key_signature,
            recovery_authority_proof_ref,
            recovery_authority_proof_sha256,
            ..
        } = wire;
        let (proof_ref, proof_digest) = match (
            preimage.transition_mode,
            old_key_signature,
            recovery_authority_proof_ref,
            recovery_authority_proof_sha256,
        ) {
            (PrincipalKeyTransitionModeV1::Recovery, None, Some(reference), Some(digest)) => {
                (reference, digest)
            }
            _ => {
                return Err(contextual(
                    "principal key transition",
                    "exclusive recovery proof branch",
                ));
            }
        };
        if proof_ref != *recovery_proof_ref
            || proof_digest != *recovery_proof.canonical_record_sha256()
            || recovery_proof.principal_id() != &preimage.principal_id
            || recovery_proof.prior_principal_record_ref() != context.predecessor_ref
            || recovery_proof.prior_principal_record_sha256() != &preimage.previous_record_sha256
            || recovery_proof.new_key_id() != &preimage.new_key_id
            || recovery_proof.new_public_key() != &preimage.new_public_key
            || recovery_proof.new_key_epoch() != preimage.new_key_epoch
        {
            return Err(contextual(
                "principal key transition",
                "exact admitted recovery proof and successor key",
            ));
        }
        verify_new_signature(
            PRINCIPAL_KEY_TRANSITION_NEW_V1_SIGNATURE_DOMAIN,
            &preimage.new_public_key,
            &preimage,
            &digest,
            &new_key_signature,
        )?;
        Ok(Self {
            preimage,
            canonical_record_sha256: digest,
            attestation: PrincipalKeyTransitionAttestationV1::Recovery { new_key_signature },
        })
    }

    /// Revalidate the transition's local branch shape and canonical identity.
    ///
    /// This does not repeat signature callbacks or prove durable proof
    /// consumption.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the admitted branch and
    /// canonical preimage no longer agree.
    pub fn verify(&self) -> Result<(), InvalidResidentRecordV1> {
        let branch_matches = match (&self.preimage.transition_mode, &self.attestation) {
            (
                PrincipalKeyTransitionModeV1::NormalRotation,
                PrincipalKeyTransitionAttestationV1::NormalRotation { .. },
            ) => {
                self.preimage.recovery_authority_proof_ref.is_none()
                    && self.preimage.recovery_authority_proof_sha256.is_none()
            }
            (
                PrincipalKeyTransitionModeV1::Recovery,
                PrincipalKeyTransitionAttestationV1::Recovery { .. },
            ) => {
                self.preimage.recovery_authority_proof_ref.is_some()
                    && self.preimage.recovery_authority_proof_sha256.is_some()
            }
            _ => false,
        };
        if !branch_matches
            || PrincipalKeyTransitionDigest::hash_canonical(&self.preimage)?
                != self.canonical_record_sha256
        {
            Err(InvalidResidentRecordV1::DigestMismatch {
                family: "principal key transition",
            })
        } else {
            Ok(())
        }
    }

    /// Borrow the transition mode.
    #[must_use]
    pub const fn transition_mode(&self) -> PrincipalKeyTransitionModeV1 {
        self.preimage.transition_mode
    }

    /// Borrow the principal whose key advances.
    #[must_use]
    pub fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.preimage.principal_id
    }

    /// Return the predecessor key epoch.
    #[must_use]
    pub const fn old_key_epoch(&self) -> PrincipalKeyEpochV1 {
        self.preimage.old_key_epoch
    }

    /// Return the successor key epoch.
    #[must_use]
    pub const fn new_key_epoch(&self) -> PrincipalKeyEpochV1 {
        self.preimage.new_key_epoch
    }

    /// Borrow the predecessor key identity.
    #[must_use]
    pub fn old_key_id(&self) -> &ResidentKeyIdV1 {
        &self.preimage.old_key_id
    }

    /// Borrow the successor key identity.
    #[must_use]
    pub fn new_key_id(&self) -> &ResidentKeyIdV1 {
        &self.preimage.new_key_id
    }

    /// Borrow the predecessor Ed25519 public key.
    #[must_use]
    pub fn old_public_key(&self) -> &Ed25519PublicKeyV1 {
        &self.preimage.old_public_key
    }

    /// Borrow the successor Ed25519 public key.
    #[must_use]
    pub fn new_public_key(&self) -> &Ed25519PublicKeyV1 {
        &self.preimage.new_public_key
    }

    /// Borrow the exact predecessor principal-record digest.
    #[must_use]
    pub fn previous_record_sha256(&self) -> &PrincipalRecordDigest {
        &self.preimage.previous_record_sha256
    }

    /// Return the transition effective time.
    #[must_use]
    pub const fn effective_at(&self) -> UnixTimeV1 {
        self.preimage.effective_at
    }

    /// Borrow the canonical unsigned transition identity.
    #[must_use]
    pub fn canonical_record_sha256(&self) -> &PrincipalKeyTransitionDigest {
        &self.canonical_record_sha256
    }

    /// Borrow the old-key signature when this is normal rotation.
    #[must_use]
    pub fn old_key_signature(&self) -> Option<&CanonicalSignatureV1> {
        match &self.attestation {
            PrincipalKeyTransitionAttestationV1::NormalRotation {
                old_key_signature, ..
            } => Some(old_key_signature),
            PrincipalKeyTransitionAttestationV1::Recovery { .. } => None,
        }
    }

    /// Borrow the required new-key signature.
    #[must_use]
    pub fn new_key_signature(&self) -> &CanonicalSignatureV1 {
        match &self.attestation {
            PrincipalKeyTransitionAttestationV1::NormalRotation {
                new_key_signature, ..
            }
            | PrincipalKeyTransitionAttestationV1::Recovery {
                new_key_signature, ..
            } => new_key_signature,
        }
    }

    /// Borrow the recovery-proof reference when this is recovery.
    #[must_use]
    pub fn recovery_authority_proof_ref(&self) -> Option<&ResidentRecordRefV1> {
        self.preimage.recovery_authority_proof_ref.as_ref()
    }

    /// Borrow the recovery-proof digest when this is recovery.
    #[must_use]
    pub fn recovery_authority_proof_sha256(&self) -> Option<&PrincipalRecoveryProofDigest> {
        self.preimage.recovery_authority_proof_sha256.as_ref()
    }
}

impl Serialize for PrincipalKeyTransitionV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        PrincipalKeyTransitionWireV1::from(self).serialize(serializer)
    }
}

impl From<&PrincipalKeyTransitionV1> for PrincipalKeyTransitionWireV1 {
    fn from(value: &PrincipalKeyTransitionV1) -> Self {
        let (old_key_signature, new_key_signature) = match &value.attestation {
            PrincipalKeyTransitionAttestationV1::NormalRotation {
                old_key_signature,
                new_key_signature,
            } => (Some(old_key_signature.clone()), new_key_signature.clone()),
            PrincipalKeyTransitionAttestationV1::Recovery { new_key_signature } => {
                (None, new_key_signature.clone())
            }
        };
        Self {
            schema_id: value.preimage.schema_id.clone(),
            schema_sha256: value.preimage.schema_sha256.clone(),
            principal_id: value.preimage.principal_id.clone(),
            transition_mode: value.preimage.transition_mode,
            old_key_epoch: value.preimage.old_key_epoch,
            new_key_epoch: value.preimage.new_key_epoch,
            old_key_id: value.preimage.old_key_id.clone(),
            new_key_id: value.preimage.new_key_id.clone(),
            old_public_key: value.preimage.old_public_key.clone(),
            new_public_key: value.preimage.new_public_key.clone(),
            transition_reason: value.preimage.transition_reason.clone(),
            previous_record_sha256: value.preimage.previous_record_sha256.clone(),
            old_key_signature,
            new_key_signature,
            recovery_authority_proof_ref: value.preimage.recovery_authority_proof_ref.clone(),
            recovery_authority_proof_sha256: value.preimage.recovery_authority_proof_sha256.clone(),
            effective_at: value.preimage.effective_at,
            canonical_record_sha256: value.canonical_record_sha256.clone(),
        }
    }
}

impl ResidentSchemaIdentityV1 for PrincipalKeyTransitionWireV1 {
    const SCHEMA_FIELD: &'static str = "schema_id";
    const SCHEMA: &'static str = PRINCIPAL_KEY_TRANSITION_V1_SCHEMA;
    const VERSION: Option<(&'static str, u32)> = None;
}

fn validate_transition_wire(
    wire: &PrincipalKeyTransitionWireV1,
    context: &PrincipalKeyTransitionAdmissionContextV1<'_>,
) -> Result<
    (
        PrincipalKeyTransitionPreimageV1,
        PrincipalKeyTransitionDigest,
    ),
    InvalidResidentRecordV1,
> {
    context.predecessor.verify()?;
    let expected_epoch = context
        .predecessor
        .key_epoch()
        .checked_successor()
        .ok_or_else(|| {
            contextual(
                "principal key transition",
                "non-exhausted successor key epoch",
            )
        })?;
    let preimage = wire.preimage();
    if preimage.schema_id.as_str() != PRINCIPAL_KEY_TRANSITION_V1_SCHEMA
        || preimage.schema_sha256 != *context.expected_schema_sha256
        || preimage.principal_id != *context.predecessor.principal_id()
        || preimage.old_key_epoch != *context.predecessor.key_epoch()
        || preimage.new_key_epoch != expected_epoch
        || preimage.old_key_id != *context.predecessor.root_key_id()
        || preimage.old_public_key != *context.predecessor.root_public_key()
        || preimage.previous_record_sha256 != *context.predecessor.canonical_record_sha256()
    {
        return Err(contextual(
            "principal key transition",
            "exact predecessor key and next epoch",
        ));
    }
    let digest = PrincipalKeyTransitionDigest::hash_canonical(&preimage)?;
    if digest != wire.canonical_record_sha256 {
        return Err(InvalidResidentRecordV1::DigestMismatch {
            family: "principal key transition",
        });
    }
    Ok((preimage, digest))
}

impl PrincipalKeyTransitionWireV1 {
    fn preimage(&self) -> PrincipalKeyTransitionPreimageV1 {
        PrincipalKeyTransitionPreimageV1 {
            schema_id: self.schema_id.clone(),
            schema_sha256: self.schema_sha256.clone(),
            principal_id: self.principal_id.clone(),
            transition_mode: self.transition_mode,
            old_key_epoch: self.old_key_epoch,
            new_key_epoch: self.new_key_epoch,
            old_key_id: self.old_key_id.clone(),
            new_key_id: self.new_key_id.clone(),
            old_public_key: self.old_public_key.clone(),
            new_public_key: self.new_public_key.clone(),
            transition_reason: self.transition_reason.clone(),
            previous_record_sha256: self.previous_record_sha256.clone(),
            recovery_authority_proof_ref: self.recovery_authority_proof_ref.clone(),
            recovery_authority_proof_sha256: self.recovery_authority_proof_sha256.clone(),
            effective_at: self.effective_at,
        }
    }
}

fn project_key_transition_schema_v1(schema: &mut schemars::Schema) {
    close_resident_schema_identity_v1::<PrincipalKeyTransitionWireV1>(schema);
    require_all_or_none_properties_v1(
        schema,
        &[
            "recovery_authority_proof_ref",
            "recovery_authority_proof_sha256",
        ],
    );
    schema.ensure_object().insert(
        "oneOf".into(),
        serde_json::json!([
            {
                "properties": { "transition_mode": { "const": "normal_rotation" } },
                "required": ["old_key_signature"],
                "not": {
                    "anyOf": [
                        { "required": ["recovery_authority_proof_ref"] },
                        { "required": ["recovery_authority_proof_sha256"] }
                    ]
                }
            },
            {
                "properties": { "transition_mode": { "const": "recovery" } },
                "required": [
                    "recovery_authority_proof_ref",
                    "recovery_authority_proof_sha256"
                ],
                "not": { "required": ["old_key_signature"] }
            }
        ]),
    );
}
