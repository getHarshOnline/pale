//! Producer-incarnation allocation and retirement contracts.
//!
//! [`ProducerIncarnationHeadStateV1`] exposes active and retired states. In the
//! rendered law, `alpha` and `beta` are producer epochs and `E` is the set of
//! epochs already allocated across the principal; successor admission proves
//! `beta` is fresh.
//!
//! These pure morphisms consume owner-resolved head and uniqueness witnesses;
//! atomic allocation, retirement persistence, and CAS remain external. A
//! retirement blocks new rows under its epoch but leaves immutable rows already
//! committed under that epoch eligible for bounded drain.

use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for producer-incarnation allocation wires.
pub const PRODUCER_INCARNATION_V1_SCHEMA_ID: &str = "producer-incarnation.v1";
/// Wire schema carried by every producer-incarnation allocation.
pub const PRODUCER_INCARNATION_V1_SCHEMA: &str = "pale/producer-incarnation.v1";
/// Signature domain for [`ProducerIncarnationV1`].
pub const PRODUCER_INCARNATION_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_PRODUCER_INCARNATION_V1\0";

/// Public schema root for producer-incarnation retirement wires.
pub const PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA_ID: &str = "producer-incarnation-retirement.v1";
/// Wire schema carried by every producer-incarnation retirement.
pub const PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA: &str =
    "pale/producer-incarnation-retirement.v1";
/// Signature domain for [`ProducerIncarnationRetirementV1`].
pub const PRODUCER_INCARNATION_RETIREMENT_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_PRODUCER_INCARNATION_RETIREMENT_V1\0";

/// Minimum canonical hex-character count encoding 16 bytes (128 bits).
const MIN_PRODUCER_EPOCH_HEX_CHARS: usize = 32;

/// Lifecycle state committed for one principal-wide producer allocation head.
///
/// Retirement is append-only: a retired head remains the current coordinate
/// until a fresh successor allocation is admitted. The state therefore
/// describes the head, not the historical allocation record itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProducerIncarnationHeadStateV1 {
    /// The named incarnation may commit new owner-outbox rows.
    Active,
    /// The named incarnation may drain immutable rows but may not create new ones.
    Retired,
}

/// Canonical producer-allocation head committed by a principal trust checkpoint.
///
/// A complete projection is ordered by
/// `(`[`Self::origin_principal_id`]`, `[`Self::producer_id`]`)`. Use
/// [`producer_incarnation_heads_root_sha256`] to validate that ordering and
/// derive the only canonical root accepted by
/// [`crate::records::PrincipalTrustCheckpointV1`].
///
/// In the rendered law, `H` is the ordered head sequence, `h_i` is member `i`,
/// and `root(H)` is the SHA-256 digest of its canonical JSON array. `alpha` and
/// `beta` are successive producer epochs, and `E` is the set of epochs already
/// allocated for the principal. A retired head can advance only to an unused
/// epoch; it cannot reactivate its prior allocation.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/producer-incarnation-head-root.display.mathml"
))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProducerIncarnationHeadV1 {
    /// Principal across which the producer epoch is unique.
    origin_principal_id: ResidentPrincipalIdV1,
    /// Operational producer whose allocation head is represented.
    producer_id: ProducerIdV1,
    /// Immutable allocation record reference.
    incarnation_ref: ResidentRecordRefV1,
    /// Canonical unsigned-preimage digest of the allocation.
    incarnation_sha256: ProducerIncarnationDigest,
    /// Principal-wide epoch carried by the allocation.
    producer_epoch: ProducerEpochV1,
    /// Current lifecycle state of this exact allocation head.
    state: ProducerIncarnationHeadStateV1,
}

impl ProducerIncarnationHeadV1 {
    /// Project one admitted allocation as an active canonical head.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when `incarnation` does not
    /// reproduce its canonical digest.
    pub fn active(
        incarnation_ref: &ResidentRecordRefV1,
        incarnation: &ProducerIncarnationV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        incarnation.verify()?;
        Ok(Self::from_incarnation(
            incarnation_ref,
            incarnation,
            ProducerIncarnationHeadStateV1::Active,
        ))
    }

    /// Project one exact allocation-and-retirement pair as a retired head.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] unless both records are canonical
    /// and the retirement closes the exact supplied allocation.
    pub fn retired(
        incarnation_ref: &ResidentRecordRefV1,
        incarnation: &ProducerIncarnationV1,
        retirement: &ProducerIncarnationRetirementV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        validate_retired_incarnation_head(incarnation_ref, incarnation, retirement)?;
        Ok(Self::from_incarnation(
            incarnation_ref,
            incarnation,
            ProducerIncarnationHeadStateV1::Retired,
        ))
    }

    /// Borrow the canonical ordering key for this projection.
    #[must_use]
    pub const fn key(&self) -> (&ResidentPrincipalIdV1, &ProducerIdV1) {
        (&self.origin_principal_id, &self.producer_id)
    }

    /// Borrow the principal across which the producer epoch is unique.
    #[must_use]
    pub const fn origin_principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.origin_principal_id
    }

    /// Borrow the operational producer whose allocation head is represented.
    #[must_use]
    pub const fn producer_id(&self) -> &ProducerIdV1 {
        &self.producer_id
    }

    /// Borrow the immutable allocation-record reference.
    #[must_use]
    pub const fn incarnation_ref(&self) -> &ResidentRecordRefV1 {
        &self.incarnation_ref
    }

    /// Borrow the canonical unsigned-preimage digest of the allocation.
    #[must_use]
    pub const fn incarnation_sha256(&self) -> &ProducerIncarnationDigest {
        &self.incarnation_sha256
    }

    /// Borrow the principal-wide unique epoch carried by this allocation.
    #[must_use]
    pub const fn producer_epoch(&self) -> &ProducerEpochV1 {
        &self.producer_epoch
    }

    /// Return the append-only lifecycle state of the current allocation head.
    #[must_use]
    pub const fn state(&self) -> ProducerIncarnationHeadStateV1 {
        self.state
    }

    fn from_incarnation(
        incarnation_ref: &ResidentRecordRefV1,
        incarnation: &ProducerIncarnationV1,
        state: ProducerIncarnationHeadStateV1,
    ) -> Self {
        Self {
            origin_principal_id: incarnation.origin_principal_id().clone(),
            producer_id: incarnation.producer_id().clone(),
            incarnation_ref: incarnation_ref.clone(),
            incarnation_sha256: incarnation.canonical_incarnation_sha256().clone(),
            producer_epoch: incarnation.producer_epoch().clone(),
            state,
        }
    }
}

/// Derive the canonical root of the complete producer-allocation head set.
///
/// The input must be strictly ordered and duplicate-free by
/// `(origin_principal_id, producer_id)`. The digest is SHA-256 over the RFC
/// 8785 canonical JSON array of the exact supplied projections. The empty
/// slice is the bootstrap root.
///
/// # Errors
///
/// Returns [`InvalidResidentRecordV1::NonCanonical`] for duplicate or
/// out-of-order head keys, [`InvalidResidentRecordV1::BoundExceeded`] above
/// the shared resident collection ceiling, or
/// [`InvalidResidentRecordV1::Canon`] if canonical encoding fails.
pub fn producer_incarnation_heads_root_sha256(
    heads: &[ProducerIncarnationHeadV1],
) -> Result<ProducerIncarnationHeadsRootDigest, InvalidResidentRecordV1> {
    bounded_len("producer incarnation heads", heads.len())?;
    if !heads.windows(2).all(|pair| pair[0].key() < pair[1].key()) {
        return Err(InvalidResidentRecordV1::NonCanonical {
            relation: "producer incarnation heads",
        });
    }
    Ok(ProducerIncarnationHeadsRootDigest::hash_canonical(heads)?)
}

/// Principal-wide unique producer epoch encoded as canonical lowercase hex.
///
/// The representation is even-length lowercase hexadecimal with at least 32
/// ASCII hex characters, encoding 16 bytes (128 bits). Its encoded text uses
/// the common [`MAX_RESIDENT_ID_BYTES`] ceiling. Construction validates
/// representation only; CSPRNG provenance and principal-wide non-reuse remain
/// owner evidence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct ProducerEpochV1(#[schemars(regex(pattern = r"^(?:[0-9a-f]{2}){16,256}$"))] String);

impl ProducerEpochV1 {
    /// Validate one producer epoch received across a trust boundary.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1::InvalidIdentity`] unless `value` is
    /// canonical even-length lowercase hexadecimal text between 32 and 512
    /// bytes inclusive.
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidResidentRecordV1> {
        let value = value.into();
        let valid = (MIN_PRODUCER_EPOCH_HEX_CHARS..=MAX_RESIDENT_ID_BYTES).contains(&value.len())
            && value.len().is_multiple_of(2)
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if valid {
            Ok(Self(value))
        } else {
            Err(InvalidResidentRecordV1::InvalidIdentity {
                kind: "producer epoch",
            })
        }
    }

    /// Borrow the canonical lowercase hexadecimal representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ProducerEpochV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Exact owner-resolved authority used for producer-incarnation admission.
///
/// Construction binds the immutable reference to one self-consistent
/// [`ProducerIncarnationAuthorityV1`]. Admission then derives the principal,
/// delegated signing key, validity interval, lifecycle state, namespace rules,
/// and canonical unsigned-preimage digest from that one record. Callers cannot
/// pair those coordinates independently.
///
/// This witness neither resolves nor admits the authority. The owner must pass
/// an authority already admitted through its root-delegation path.
pub struct ProducerIncarnationAuthorityAdmissionContextV1<'a> {
    authority_ref: &'a ResidentRecordRefV1,
    authority: &'a ProducerIncarnationAuthorityV1,
}

impl<'a> ProducerIncarnationAuthorityAdmissionContextV1<'a> {
    /// Bind an immutable reference to one exact producer authority.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when `authority` fails its
    /// canonical unsigned-preimage digest check. Root-signature and lineage
    /// admission remain the caller's prerequisite.
    pub fn new(
        authority_ref: &'a ResidentRecordRefV1,
        authority: &'a ProducerIncarnationAuthorityV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        authority.verify()?;
        Ok(Self {
            authority_ref,
            authority,
        })
    }
}

/// Owner-resolved allocation-head state for one `(principal, producer)` key.
///
/// This pure witness separates an allocation that may be retired from an exact
/// retired allocation that may be followed by a fresh successor. It neither
/// reads nor mutates the trust store; the owner must resolve it and later
/// perform the atomic compare-and-swap.
pub enum ProducerIncarnationHeadAdmissionStateV1<'a> {
    /// No allocation head exists for the selected principal and producer.
    Unallocated,
    /// The exact allocation is the current non-retired head.
    Active {
        /// Immutable reference resolving `incarnation`.
        incarnation_ref: &'a ResidentRecordRefV1,
        /// Exact current allocation record.
        incarnation: &'a ProducerIncarnationV1,
    },
    /// The exact allocation head has an admitted append-only retirement.
    Retired {
        /// Immutable reference resolving `incarnation`.
        incarnation_ref: &'a ResidentRecordRefV1,
        /// Exact retired allocation record.
        incarnation: &'a ProducerIncarnationV1,
        /// Exact admitted retirement record.
        retirement: &'a ProducerIncarnationRetirementV1,
    },
}

impl<'a> ProducerIncarnationHeadAdmissionStateV1<'a> {
    /// Bind an exact verified allocation as the current active head.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when `incarnation` fails its
    /// self-digest check.
    pub fn active(
        incarnation_ref: &'a ResidentRecordRefV1,
        incarnation: &'a ProducerIncarnationV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        incarnation.verify()?;
        Ok(Self::Active {
            incarnation_ref,
            incarnation,
        })
    }

    /// Bind an exact verified allocation and retirement as the retired head.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when either record fails its
    /// self-digest check or the retirement does not close the exact allocation.
    pub fn retired(
        incarnation_ref: &'a ResidentRecordRefV1,
        incarnation: &'a ProducerIncarnationV1,
        retirement: &'a ProducerIncarnationRetirementV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        validate_retired_incarnation_head(incarnation_ref, incarnation, retirement)?;
        Ok(Self::Retired {
            incarnation_ref,
            incarnation,
            retirement,
        })
    }

    /// Require the absence of an allocation head.
    fn require_unallocated(&self) -> Result<(), InvalidResidentRecordV1> {
        if matches!(self, Self::Unallocated) {
            Ok(())
        } else {
            Err(contextual(
                "producer incarnation allocation",
                "unallocated current head",
            ))
        }
    }

    /// Borrow the exact verified active allocation head.
    fn require_active(
        &self,
    ) -> Result<(&'a ResidentRecordRefV1, &'a ProducerIncarnationV1), InvalidResidentRecordV1> {
        match self {
            Self::Active {
                incarnation_ref,
                incarnation,
            } => {
                incarnation.verify()?;
                Ok((incarnation_ref, incarnation))
            }
            Self::Unallocated | Self::Retired { .. } => Err(contextual(
                "producer incarnation retirement",
                "exact active allocation head",
            )),
        }
    }

    /// Borrow the exact verified retired allocation head.
    fn require_retired(
        &self,
    ) -> Result<
        (
            &'a ResidentRecordRefV1,
            &'a ProducerIncarnationV1,
            &'a ProducerIncarnationRetirementV1,
        ),
        InvalidResidentRecordV1,
    > {
        match self {
            Self::Retired {
                incarnation_ref,
                incarnation,
                retirement,
            } => {
                validate_retired_incarnation_head(incarnation_ref, incarnation, retirement)?;
                Ok((incarnation_ref, incarnation, retirement))
            }
            Self::Unallocated | Self::Active { .. } => Err(contextual(
                "producer incarnation allocation",
                "exact retired predecessor",
            )),
        }
    }
}

resident_signed_record!(
    /// Signed allocation of one principal-wide unique producer incarnation.
    ProducerIncarnationV1,
    ProducerIncarnationWireV1,
    ProducerIncarnationPreimageV1,
    canonical_incarnation_sha256: ProducerIncarnationDigest,
    PRODUCER_INCARNATION_V1_SCHEMA,
    "producer incarnation",
    {
        /// Stable identity of this allocation record.
        incarnation_id: ProducerIncarnationIdV1,
        /// Principal across which [`Self::producer_epoch`] is unique.
        origin_principal_id: ResidentPrincipalIdV1,
        /// Operational producer whose active allocation head advances.
        producer_id: ProducerIdV1,
        /// Principal-wide unique epoch persisted before new outbox rows.
        producer_epoch: ProducerEpochV1,
        /// Immutable producer implementation reference.
        producer_implementation_ref: ResidentRecordRefV1,
        /// Digest of the exact producer implementation bytes.
        producer_implementation_sha256: ArtifactDigest,
        /// Exact active producer-incarnation authority record.
        producer_incarnation_authority_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the active authority.
        producer_incarnation_authority_sha256: ProducerIncarnationAuthorityDigest,
        /// Unix time at which the principal trust store allocated the epoch.
        allocated_at: UnixTimeV1,
        /// Exact prior allocation for this principal and producer, when present.
        previous_incarnation_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_incarnation_ref`].
        previous_incarnation_sha256: Option<ProducerIncarnationDigest>,
        /// Principal whose delegated producer authority issued this allocation.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Active producer-incarnation-authority subject key.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive delegated authority-subject key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    |value: &ProducerIncarnationPreimageV1| {
        validate_issuer_shape(
            "producer incarnation",
            &value.origin_principal_id,
            &value.issuer_principal_id,
        )?;
        if value.previous_incarnation_ref.is_some()
            != value.previous_incarnation_sha256.is_some()
        {
            Err(InvalidResidentRecordV1::NonCanonical {
                relation: "producer incarnation predecessor reference",
            })
        } else {
            Ok(())
        }
    }
);

resident_signed_record!(
    /// Separately signed append-only retirement of one producer incarnation.
    ProducerIncarnationRetirementV1,
    ProducerIncarnationRetirementWireV1,
    ProducerIncarnationRetirementPreimageV1,
    canonical_retirement_sha256: ProducerIncarnationRetirementDigest,
    PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA,
    "producer incarnation retirement",
    {
        /// Stable identity of this retirement record.
        retirement_id: ProducerIncarnationRetirementIdV1,
        /// Exact allocation record being retired.
        incarnation_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the allocation record.
        incarnation_sha256: ProducerIncarnationDigest,
        /// Principal that allocated the producer epoch.
        origin_principal_id: ResidentPrincipalIdV1,
        /// Operational producer whose incarnation is retired.
        producer_id: ProducerIdV1,
        /// Exact epoch copied from the allocation record.
        producer_epoch: ProducerEpochV1,
        /// Exact active producer-incarnation authority record.
        producer_incarnation_authority_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the active authority.
        producer_incarnation_authority_sha256: ProducerIncarnationAuthorityDigest,
        /// Stable bounded reason for retirement.
        reason_code: ResidentReasonV1,
        /// Unix time at which retirement was appended.
        retired_at: UnixTimeV1,
        /// Principal whose delegated producer authority issued this retirement.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Active producer-incarnation-authority subject key.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive delegated authority-subject key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    |value: &ProducerIncarnationRetirementPreimageV1| {
        validate_issuer_shape(
            "producer incarnation retirement",
            &value.origin_principal_id,
            &value.issuer_principal_id,
        )
    }
);

impl ProducerIncarnationV1 {
    /// Admit the first allocation for one principal and producer.
    ///
    /// `check_epoch_unallocated` is a validation-only witness that the epoch
    /// has never appeared under the principal. It does not reserve or persist
    /// the epoch; atomic CAS/co-commit remains owner work.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when initial-head, authority,
    /// uniqueness, signed coordinates, digest, or signature admission fails.
    pub fn admit_initial_wire<U, C, S>(
        wire: ProducerIncarnationWireV1,
        head_state: &ProducerIncarnationHeadAdmissionStateV1<'_>,
        expected_schema_sha256: &SchemaDigest,
        authority: &ProducerIncarnationAuthorityAdmissionContextV1<'_>,
        check_epoch_unallocated: U,
        check_namespace_coverage: C,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        U: FnOnce(&ResidentPrincipalIdV1, &ProducerEpochV1) -> Result<(), InvalidResidentRecordV1>,
        C: FnOnce(&[ProducerNamespaceV1], &ProducerIdV1) -> bool,
        S: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &ProducerIncarnationPreimageV1,
            &ProducerIncarnationDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        head_state.require_unallocated()?;
        Self::admit_allocation_wire(
            wire,
            None,
            expected_schema_sha256,
            authority,
            check_epoch_unallocated,
            check_namespace_coverage,
            verify_signature,
        )
    }

    /// Admit a successor after the exact current allocation was retired.
    ///
    /// The successor preserves principal and producer identity and changes
    /// epoch. `head_state` supplies the exact admitted retirement record, whose
    /// signed preimage must retire the predecessor allocation ref/hash. The
    /// successor wire binds that allocation as its predecessor, and its
    /// allocation time cannot precede the retirement. Global epoch uniqueness
    /// is checked by a validation-only owner witness. The retirement prevents
    /// new rows under the predecessor epoch without invalidating immutable rows
    /// already committed under it.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when predecessor, terminal state,
    /// authority, uniqueness, chronology, digest, or signature admission fails.
    pub fn admit_successor_wire<U, C, S>(
        wire: ProducerIncarnationWireV1,
        head_state: &ProducerIncarnationHeadAdmissionStateV1<'_>,
        expected_schema_sha256: &SchemaDigest,
        authority: &ProducerIncarnationAuthorityAdmissionContextV1<'_>,
        check_epoch_unallocated: U,
        check_namespace_coverage: C,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        U: FnOnce(&ResidentPrincipalIdV1, &ProducerEpochV1) -> Result<(), InvalidResidentRecordV1>,
        C: FnOnce(&[ProducerNamespaceV1], &ProducerIdV1) -> bool,
        S: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &ProducerIncarnationPreimageV1,
            &ProducerIncarnationDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (incarnation_ref, incarnation, retirement) = head_state.require_retired()?;
        Self::admit_allocation_wire(
            wire,
            Some(RetiredProducerIncarnationV1 {
                incarnation_ref,
                incarnation,
                retired_at: *retirement.retired_at(),
            }),
            expected_schema_sha256,
            authority,
            check_epoch_unallocated,
            check_namespace_coverage,
            verify_signature,
        )
    }

    fn admit_allocation_wire<U, C, S>(
        wire: ProducerIncarnationWireV1,
        predecessor: Option<RetiredProducerIncarnationV1<'_>>,
        expected_schema_sha256: &SchemaDigest,
        authority: &ProducerIncarnationAuthorityAdmissionContextV1<'_>,
        check_epoch_unallocated: U,
        check_namespace_coverage: C,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        U: FnOnce(&ResidentPrincipalIdV1, &ProducerEpochV1) -> Result<(), InvalidResidentRecordV1>,
        C: FnOnce(&[ProducerNamespaceV1], &ProducerIdV1) -> bool,
        S: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &ProducerIncarnationPreimageV1,
            &ProducerIncarnationDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_producer_authority(
                "producer incarnation",
                expected_schema_sha256,
                ProducerAuthorityCoordinatesV1::from_incarnation(preimage),
                authority,
                preimage.allocated_at,
                &preimage.producer_id,
                check_namespace_coverage,
            )?;
            let predecessor_valid = match predecessor {
                None => {
                    preimage.previous_incarnation_ref.is_none()
                        && preimage.previous_incarnation_sha256.is_none()
                }
                Some(predecessor) => {
                    preimage.previous_incarnation_ref.as_ref() == Some(predecessor.incarnation_ref)
                        && preimage.previous_incarnation_sha256.as_ref()
                            == Some(predecessor.incarnation.canonical_incarnation_sha256())
                        && preimage.origin_principal_id
                            == *predecessor.incarnation.origin_principal_id()
                        && preimage.producer_id == *predecessor.incarnation.producer_id()
                        && preimage.producer_epoch != *predecessor.incarnation.producer_epoch()
                        && preimage.allocated_at >= predecessor.retired_at
                }
            };
            if !predecessor_valid {
                return Err(contextual(
                    "producer incarnation allocation",
                    "exact predecessor allocation coordinates",
                ));
            }
            verify_signature(
                PRODUCER_INCARNATION_V1_SIGNATURE_DOMAIN,
                authority.authority.subject_public_key(),
                preimage,
                digest,
                signature,
            )?;
            check_epoch_unallocated(&preimage.origin_principal_id, &preimage.producer_epoch)
        })
    }
}

impl ProducerIncarnationRetirementV1 {
    /// Admit one append-only retirement under the current producer authority.
    ///
    /// The head must resolve to the exact active allocation. This pure check
    /// does not mutate the head or perform trust-store CAS/co-commit.
    /// An admitted retirement is owner state that must block new rows, but
    /// does not invalidate immutable rows already committed under the epoch.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when allocation identity, head
    /// state, authority, chronology, digest, or signature admission fails.
    pub fn admit_contextual_wire<C, S>(
        wire: ProducerIncarnationRetirementWireV1,
        head_state: &ProducerIncarnationHeadAdmissionStateV1<'_>,
        expected_schema_sha256: &SchemaDigest,
        authority: &ProducerIncarnationAuthorityAdmissionContextV1<'_>,
        check_namespace_coverage: C,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        C: FnOnce(&[ProducerNamespaceV1], &ProducerIdV1) -> bool,
        S: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &ProducerIncarnationRetirementPreimageV1,
            &ProducerIncarnationRetirementDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (incarnation_ref, incarnation) = head_state.require_active()?;
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_producer_authority(
                "producer incarnation retirement",
                expected_schema_sha256,
                ProducerAuthorityCoordinatesV1::from_retirement(preimage),
                authority,
                preimage.retired_at,
                &preimage.producer_id,
                check_namespace_coverage,
            )?;
            if preimage.incarnation_ref != *incarnation_ref
                || preimage.incarnation_sha256 != *incarnation.canonical_incarnation_sha256()
                || preimage.origin_principal_id != *incarnation.origin_principal_id()
                || preimage.producer_id != *incarnation.producer_id()
                || preimage.producer_epoch != *incarnation.producer_epoch()
                || preimage.retired_at < *incarnation.allocated_at()
            {
                return Err(contextual(
                    "producer incarnation retirement",
                    "exact allocation identity and monotone retirement time",
                ));
            }
            verify_signature(
                PRODUCER_INCARNATION_RETIREMENT_V1_SIGNATURE_DOMAIN,
                authority.authority.subject_public_key(),
                preimage,
                digest,
                signature,
            )
        })
    }
}

/// Verify that one retirement closes the exact named allocation.
fn validate_retired_incarnation_head(
    incarnation_ref: &ResidentRecordRefV1,
    incarnation: &ProducerIncarnationV1,
    retirement: &ProducerIncarnationRetirementV1,
) -> Result<(), InvalidResidentRecordV1> {
    incarnation.verify()?;
    retirement.verify()?;
    if retirement.incarnation_ref() != incarnation_ref
        || retirement.incarnation_sha256() != incarnation.canonical_incarnation_sha256()
        || retirement.origin_principal_id() != incarnation.origin_principal_id()
        || retirement.producer_id() != incarnation.producer_id()
        || retirement.producer_epoch() != incarnation.producer_epoch()
        || retirement.retired_at() < incarnation.allocated_at()
    {
        Err(contextual(
            "producer incarnation allocation",
            "exact retired predecessor allocation context",
        ))
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct RetiredProducerIncarnationV1<'a> {
    incarnation_ref: &'a ResidentRecordRefV1,
    incarnation: &'a ProducerIncarnationV1,
    retired_at: UnixTimeV1,
}

struct ProducerAuthorityCoordinatesV1<'a> {
    schema_sha256: &'a SchemaDigest,
    principal_id: &'a ResidentPrincipalIdV1,
    issuer_principal_id: &'a ResidentPrincipalIdV1,
    issuer_key_id: &'a ResidentKeyIdV1,
    issuer_key_epoch: SigningKeyEpochV1,
    authority_ref: &'a ResidentRecordRefV1,
    authority_sha256: &'a ProducerIncarnationAuthorityDigest,
}

impl<'a> ProducerAuthorityCoordinatesV1<'a> {
    fn from_incarnation(value: &'a ProducerIncarnationPreimageV1) -> Self {
        Self {
            schema_sha256: &value.schema_sha256,
            principal_id: &value.origin_principal_id,
            issuer_principal_id: &value.issuer_principal_id,
            issuer_key_id: &value.issuer_key_id,
            issuer_key_epoch: value.issuer_key_epoch,
            authority_ref: &value.producer_incarnation_authority_ref,
            authority_sha256: &value.producer_incarnation_authority_sha256,
        }
    }

    fn from_retirement(value: &'a ProducerIncarnationRetirementPreimageV1) -> Self {
        Self {
            schema_sha256: &value.schema_sha256,
            principal_id: &value.origin_principal_id,
            issuer_principal_id: &value.issuer_principal_id,
            issuer_key_id: &value.issuer_key_id,
            issuer_key_epoch: value.issuer_key_epoch,
            authority_ref: &value.producer_incarnation_authority_ref,
            authority_sha256: &value.producer_incarnation_authority_sha256,
        }
    }
}

fn validate_producer_authority<C>(
    family: &'static str,
    expected_schema_sha256: &SchemaDigest,
    coordinates: ProducerAuthorityCoordinatesV1<'_>,
    authority: &ProducerIncarnationAuthorityAdmissionContextV1<'_>,
    operation_time: UnixTimeV1,
    producer_id: &ProducerIdV1,
    check_namespace_coverage: C,
) -> Result<(), InvalidResidentRecordV1>
where
    C: FnOnce(&[ProducerNamespaceV1], &ProducerIdV1) -> bool,
{
    let authority_record = authority.authority;
    authority_record.verify()?;
    if coordinates.schema_sha256 != expected_schema_sha256
        || coordinates.authority_ref != authority.authority_ref
        || coordinates.authority_sha256 != authority_record.canonical_producer_authority_sha256()
        || coordinates.principal_id != authority_record.principal_id()
        || coordinates.issuer_principal_id != authority_record.principal_id()
        || coordinates.issuer_key_id != authority_record.subject_key_id()
        || coordinates.issuer_key_epoch != *authority_record.subject_key_epoch()
        || authority_record.state() != &PrincipalAuthorityStateV1::Active
        || operation_time < *authority_record.issued_at()
        || operation_time >= *authority_record.expires_at()
    {
        return Err(contextual(
            family,
            "exact active producer authority, delegated key, and validity interval",
        ));
    }
    if !check_namespace_coverage(
        authority_record.permitted_producer_namespaces(),
        producer_id,
    ) {
        return Err(contextual(
            family,
            "owner-resolved producer namespace coverage",
        ));
    }
    Ok(())
}
