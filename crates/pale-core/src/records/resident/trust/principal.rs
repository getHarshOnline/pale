//! Principal identity, recovery, and key-succession trust roots.
//!
//! Bootstrap admits a mutually bound pair rather than trusting either record
//! independently. The principal record names the exact recovery-policy
//! reference and digest; the policy is signed by the same candidate root key
//! and names the same principal and key epoch. The consuming admission chain is
//! owned by [`super::bootstrap`].
//!
//! Successor admission extends that bootstrap root through two acyclic proof
//! branches documented by [`PrincipalRecordV1`].
//!
//! A key transition advances only the principal-key epoch. A recovery-policy
//! update advances only the policy epoch and preserves the active key. Neither
//! path generates keys, writes a store, consumes a recovery nonce, advances a
//! checkpoint, or activates a principal.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

mod recovery;
mod successor;
mod transition;

pub use recovery::*;
pub use successor::*;
pub use transition::*;

/// Public schema root for principal-record wires.
pub const PRINCIPAL_RECORD_V1_SCHEMA_ID: &str = "principal-record.v1";
/// Wire schema carried by every principal record.
pub const PRINCIPAL_RECORD_V1_SCHEMA: &str = "pale/principal-record.v1";
/// Signature domain for [`PrincipalRecordV1`].
pub const PRINCIPAL_RECORD_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_PRINCIPAL_RECORD_V1\0";

/// Public schema root for principal-recovery-policy wires.
pub const PRINCIPAL_RECOVERY_POLICY_V1_SCHEMA_ID: &str = "principal-recovery-policy.v1";
/// Wire schema carried by every principal recovery policy.
pub const PRINCIPAL_RECOVERY_POLICY_V1_SCHEMA: &str = "pale/principal-recovery-policy.v1";
/// Signature domain for [`PrincipalRecoveryPolicyV1`].
pub const PRINCIPAL_RECOVERY_POLICY_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_PRINCIPAL_RECOVERY_POLICY_V1\0";

/// Transition class encoded by one [`PrincipalRecordV1`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalRecordTransitionKindV1 {
    /// Epoch-one candidate with no predecessor or key-transition record.
    Initial,
    /// Successor produced by an admitted normal or recovery key transition.
    KeyTransition,
    /// Same-key successor binding an admitted recovery-policy successor.
    PolicyUpdate,
}

/// Replay domain governed by one principal recovery policy.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalRecoveryReplayScopeV1 {
    /// One proof may authorize exactly one principal-key recovery transition.
    PrincipalKeyRecoveryProof,
}

/// One exact recovery signer admitted by a [`PrincipalRecoveryPolicyV1`].
///
/// A policy counts neither the same `(principal, key, epoch)` coordinate nor
/// the same Ed25519 public key more than once toward its threshold.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct PrincipalRecoveryEligibleSignerV1 {
    /// Principal whose recovery key is eligible.
    pub recovery_principal_id: ResidentPrincipalIdV1,
    /// Stable recovery-key identity within that principal.
    pub recovery_key_id: ResidentKeyIdV1,
    /// Exact Ed25519 public key admitted by the policy.
    pub recovery_public_key: Ed25519PublicKeyV1,
    /// Positive recovery-key epoch.
    pub recovery_key_epoch: PrincipalKeyEpochV1,
}

resident_signed_record!(
    /// Signed principal identity and active root-key frontier.
    ///
    /// In the law below, `P_(j,k)` is principal-record lineage step `j` at key
    /// epoch `k`, `R_m` is its recovery policy at policy epoch `m`, and
    /// `T_(k+1)` is an admitted key transition. Both branches advance the
    /// hash-linked record lineage. Only the key-transition branch advances `k`;
    /// the policy branch advances `m` while preserving the key epoch. Neither
    /// pure branch writes the durable principal head or consumes recovery
    /// authority.
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/principal-successors.display.mathml"
    ))]
    PrincipalRecordV1,
    PrincipalRecordWireV1,
    PrincipalRecordPreimageV1,
    canonical_record_sha256: PrincipalRecordDigest,
    PRINCIPAL_RECORD_V1_SCHEMA,
    "principal record",
    {
        /// Stable sovereign principal identity.
        principal_id: ResidentPrincipalIdV1,
        /// Bounded human-facing hint that grants no authority.
        display_hint: BoundedResidentTextV1,
        /// Stable identity of the active accepted root key.
        root_key_id: ResidentKeyIdV1,
        /// Exact active Ed25519 root public key.
        root_public_key: Ed25519PublicKeyV1,
        /// Positive active root-key epoch.
        key_epoch: PrincipalKeyEpochV1,
        /// Explicit transition branch represented by this record.
        record_transition_kind: PrincipalRecordTransitionKindV1,
        /// Canonical predecessor digest, absent only for the initial record.
        previous_record_sha256: Option<PrincipalRecordDigest>,
        /// Key-transition record used only by [`PrincipalRecordTransitionKindV1::KeyTransition`].
        key_transition_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::key_transition_ref`].
        key_transition_sha256: Option<PrincipalKeyTransitionDigest>,
        /// Unix issue time in whole seconds.
        issued_at: UnixTimeV1,
        /// Unix expiry time; expiry blocks new authority use, not historical verification.
        expires_at: UnixTimeV1,
        /// Exact active recovery-policy record.
        recovery_policy_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the recovery policy.
        recovery_policy_sha256: PrincipalRecoveryPolicyDigest
    },
    validate_principal_record_shape,
    wire_transform = project_principal_record_schema_v1
);

resident_signed_record!(
    /// Signed hash-linked recovery policy for one principal.
    PrincipalRecoveryPolicyV1,
    PrincipalRecoveryPolicyWireV1,
    PrincipalRecoveryPolicyPreimageV1,
    canonical_record_sha256: PrincipalRecoveryPolicyDigest,
    PRINCIPAL_RECOVERY_POLICY_V1_SCHEMA,
    "principal recovery policy",
    {
        /// Stable identity shared by the complete policy lineage.
        recovery_policy_id: PrincipalRecoveryPolicyIdV1,
        /// Principal governed by this policy.
        principal_id: ResidentPrincipalIdV1,
        /// Positive policy succession epoch.
        policy_epoch: PrincipalRecoveryPolicyEpochV1,
        /// Canonically ordered eligible recovery keys.
        eligible_signers: Vec<PrincipalRecoveryEligibleSignerV1>,
        /// Number of distinct eligible signatures required by a proof.
        threshold: u16,
        /// Maximum lifetime of one recovery proof in whole seconds.
        proof_validity_seconds: RecoveryProofValiditySecondsV1,
        /// Closed replay domain governed by this policy.
        replay_scope: PrincipalRecoveryReplayScopeV1,
        /// Unix issue time in whole seconds.
        issued_at: UnixTimeV1,
        /// Optional Unix expiry time for authorizing a new proof.
        expires_at: Option<UnixTimeV1>,
        /// Exact predecessor policy record, absent only at epoch one.
        previous_policy_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_policy_ref`].
        previous_policy_sha256: Option<PrincipalRecoveryPolicyDigest>,
        /// Active accepted principal key that issued this policy.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive accepted principal-key epoch used for the signature.
        issuer_principal_key_epoch: PrincipalKeyEpochV1
    },
    validate_recovery_policy_shape,
    wire_transform = project_recovery_policy_schema_v1
);

/// Untrusted record pair presented for initial principal admission.
///
/// The pair keeps each immutable reference, wire record, and expected schema
/// identity together while [`AdmittedPrincipalGenesisV1::admit_initial`]
/// verifies their mutual binding. Constructing this value proves nothing.
pub struct InitialPrincipalGenesisCandidateV1<'schema> {
    /// Immutable reference assigned to the candidate principal record.
    pub principal_record_ref: ResidentRecordRefV1,
    /// Signed but not yet admitted principal record.
    pub principal_wire: PrincipalRecordWireV1,
    /// Expected schema identity for the principal record.
    pub expected_principal_schema_sha256: &'schema SchemaDigest,
    /// Immutable reference assigned to the candidate recovery policy.
    pub recovery_policy_ref: ResidentRecordRefV1,
    /// Signed but not yet admitted recovery policy.
    pub recovery_wire: PrincipalRecoveryPolicyWireV1,
    /// Expected schema identity for the recovery policy.
    pub expected_recovery_schema_sha256: &'schema SchemaDigest,
}

/// Admitted mutually bound initial principal record and recovery policy.
///
/// Construction is available only through [`Self::admit_initial`], so neither
/// member can escape as a trusted bootstrap head when its peer fails.
pub struct AdmittedPrincipalGenesisV1 {
    principal_record_ref: ResidentRecordRefV1,
    principal_record: PrincipalRecordV1,
    recovery_policy_ref: ResidentRecordRefV1,
    recovery_policy: PrincipalRecoveryPolicyV1,
}

impl AdmittedPrincipalGenesisV1 {
    /// Admit the candidate root and recovery policy as one pure initial pair.
    ///
    /// Both callbacks must verify the trust module's domain-separated signature
    /// preimage under the supplied Ed25519 public key. Returning this value
    /// records no persistent head and does not make the principal operational.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for a wrong schema, non-initial
    /// branch, mismatched policy reference/digest, key or principal mismatch,
    /// invalid policy threshold/order, digest mismatch, or rejected signature.
    pub fn admit_initial<PS, RS>(
        candidate: InitialPrincipalGenesisCandidateV1<'_>,
        verify_principal_signature: PS,
        verify_recovery_signature: RS,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        PS: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalRecordPreimageV1,
            &PrincipalRecordDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        RS: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalRecoveryPolicyPreimageV1,
            &PrincipalRecoveryPolicyDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let InitialPrincipalGenesisCandidateV1 {
            principal_record_ref,
            principal_wire,
            expected_principal_schema_sha256,
            recovery_policy_ref,
            recovery_wire,
            expected_recovery_schema_sha256,
        } = candidate;
        let principal_record =
            PrincipalRecordV1::admit_wire(principal_wire, |preimage, digest, signature| {
                if preimage.schema_sha256 != *expected_principal_schema_sha256
                    || preimage.record_transition_kind != PrincipalRecordTransitionKindV1::Initial
                    || preimage.key_epoch.get() != 1
                    || preimage.recovery_policy_ref != recovery_policy_ref
                {
                    return Err(contextual(
                        "principal record",
                        "exact initial schema, key epoch, and recovery policy reference",
                    ));
                }
                verify_principal_signature(
                    PRINCIPAL_RECORD_V1_SIGNATURE_DOMAIN,
                    &preimage.root_public_key,
                    preimage,
                    digest,
                    signature,
                )
            })?;

        let recovery_policy =
            PrincipalRecoveryPolicyV1::admit_wire(recovery_wire, |preimage, digest, signature| {
                if preimage.schema_sha256 != *expected_recovery_schema_sha256
                    || preimage.principal_id != *principal_record.principal_id()
                    || preimage.policy_epoch.get() != 1
                    || preimage.issuer_key_id != *principal_record.root_key_id()
                    || preimage.issuer_principal_key_epoch != *principal_record.key_epoch()
                {
                    return Err(contextual(
                        "principal recovery policy",
                        "exact initial principal and root-key coordinates",
                    ));
                }
                verify_recovery_signature(
                    PRINCIPAL_RECOVERY_POLICY_V1_SIGNATURE_DOMAIN,
                    principal_record.root_public_key(),
                    preimage,
                    digest,
                    signature,
                )
            })?;

        if principal_record.recovery_policy_sha256() != recovery_policy.canonical_record_sha256() {
            return Err(contextual(
                "principal bootstrap genesis",
                "principal record binds exact recovery policy digest",
            ));
        }

        Ok(Self {
            principal_record_ref,
            principal_record,
            recovery_policy_ref,
            recovery_policy,
        })
    }

    /// Borrow the immutable principal-record reference.
    #[must_use]
    pub fn principal_record_ref(&self) -> &ResidentRecordRefV1 {
        &self.principal_record_ref
    }

    /// Borrow the admitted initial principal record.
    #[must_use]
    pub fn principal_record(&self) -> &PrincipalRecordV1 {
        &self.principal_record
    }

    /// Borrow the immutable recovery-policy reference.
    #[must_use]
    pub fn recovery_policy_ref(&self) -> &ResidentRecordRefV1 {
        &self.recovery_policy_ref
    }

    /// Borrow the admitted initial recovery policy.
    #[must_use]
    pub fn recovery_policy(&self) -> &PrincipalRecoveryPolicyV1 {
        &self.recovery_policy
    }
}

fn validate_principal_record_shape(
    value: &PrincipalRecordPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if value.expires_at <= value.issued_at {
        return Err(contextual("principal record", "expiry follows issue time"));
    }
    let shape = match value.record_transition_kind {
        PrincipalRecordTransitionKindV1::Initial => {
            value.previous_record_sha256.is_none()
                && value.key_transition_ref.is_none()
                && value.key_transition_sha256.is_none()
        }
        PrincipalRecordTransitionKindV1::KeyTransition => {
            value.previous_record_sha256.is_some()
                && value.key_transition_ref.is_some()
                && value.key_transition_sha256.is_some()
        }
        PrincipalRecordTransitionKindV1::PolicyUpdate => {
            value.previous_record_sha256.is_some()
                && value.key_transition_ref.is_none()
                && value.key_transition_sha256.is_none()
        }
    };
    if shape {
        Ok(())
    } else {
        Err(contextual(
            "principal record",
            "transition branch owns its exact predecessor fields",
        ))
    }
}

fn validate_recovery_policy_shape(
    value: &PrincipalRecoveryPolicyPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    strictly_sorted(
        "eligible principal recovery signers",
        &value.eligible_signers,
    )?;
    let threshold = usize::from(value.threshold);
    let distinct_signer_coordinates = value
        .eligible_signers
        .iter()
        .map(|signer| {
            (
                &signer.recovery_principal_id,
                &signer.recovery_key_id,
                signer.recovery_key_epoch,
            )
        })
        .collect::<BTreeSet<_>>()
        .len()
        == value.eligible_signers.len();
    let distinct_public_keys = value
        .eligible_signers
        .iter()
        .map(|signer| &signer.recovery_public_key)
        .collect::<BTreeSet<_>>()
        .len()
        == value.eligible_signers.len();
    let predecessor_pair =
        value.previous_policy_ref.is_some() == value.previous_policy_sha256.is_some();
    let initial_shape = (value.policy_epoch.get() == 1) == value.previous_policy_ref.is_none();
    let expiry = value
        .expires_at
        .is_none_or(|expires_at| expires_at > value.issued_at);
    if value.eligible_signers.is_empty()
        || threshold == 0
        || threshold > value.eligible_signers.len()
        || !distinct_signer_coordinates
        || !distinct_public_keys
        || !predecessor_pair
        || !initial_shape
        || !expiry
    {
        Err(contextual(
            "principal recovery policy",
            "bounded threshold, distinct signer keys, lineage, and validity interval",
        ))
    } else {
        Ok(())
    }
}

fn project_recovery_policy_schema_v1(schema: &mut schemars::Schema) {
    project_resident_set_property_v1(schema, "eligible_signers", Some(1));
    require_all_or_none_properties_v1(schema, &["previous_policy_ref", "previous_policy_sha256"]);
    schema.ensure_object().insert(
        "oneOf".into(),
        serde_json::json!([
            {
                "properties": { "policy_epoch": { "const": 1 } },
                "not": {
                    "anyOf": [
                        { "required": ["previous_policy_ref"] },
                        { "required": ["previous_policy_sha256"] }
                    ]
                }
            },
            {
                "properties": { "policy_epoch": { "minimum": 2 } },
                "required": ["previous_policy_ref", "previous_policy_sha256"]
            }
        ]),
    );
    if let Some(threshold) = schema
        .ensure_object()
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .and_then(|properties| properties.get_mut("threshold"))
        .and_then(serde_json::Value::as_object_mut)
    {
        threshold.insert("minimum".into(), 1.into());
    }
}

fn project_principal_record_schema_v1(schema: &mut schemars::Schema) {
    require_all_or_none_properties_v1(schema, &["key_transition_ref", "key_transition_sha256"]);
    schema.ensure_object().insert(
        "oneOf".into(),
        serde_json::json!([
            {
                "properties": { "record_transition_kind": { "const": "initial" } },
                "not": {
                    "anyOf": [
                        { "required": ["previous_record_sha256"] },
                        { "required": ["key_transition_ref"] },
                        { "required": ["key_transition_sha256"] }
                    ]
                }
            },
            {
                "properties": { "record_transition_kind": { "const": "key_transition" } },
                "required": [
                    "previous_record_sha256",
                    "key_transition_ref",
                    "key_transition_sha256"
                ]
            },
            {
                "properties": { "record_transition_kind": { "const": "policy_update" } },
                "required": ["previous_record_sha256"],
                "not": {
                    "anyOf": [
                        { "required": ["key_transition_ref"] },
                        { "required": ["key_transition_sha256"] }
                    ]
                }
            }
        ]),
    );
}
