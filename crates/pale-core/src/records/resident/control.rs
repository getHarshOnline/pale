//! Durable control-state checkpoint contracts.
//!
//! Within one store, incarnation, domain epoch, profile, and schema lineage,
//! checkpoint succession preserves the immutable coordinate and advances the
//! domain sequence exactly once. The complete successor equality is rendered
//! on [`ControlStateCheckpointV1`].
//!
//! Admission also binds the exact owner-resolved checkpoint signer. These pure
//! checks neither resolve the signer from a trust store nor persist or anchor a
//! checkpoint.

use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for control-state checkpoint wires.
pub const CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID: &str = "control-state-checkpoint.v1";
/// Wire schema carried by every control-state checkpoint.
pub const CONTROL_STATE_CHECKPOINT_V1_SCHEMA: &str = "pale/control-state-checkpoint.v1";
/// Signature domain for [`ControlStateCheckpointV1`].
pub const CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_CONTROL_STATE_CHECKPOINT_V1\0";

/// Owner-resolved active checkpoint signer for one exact control domain.
///
/// The trust-store owner constructs this witness only after resolving an
/// active authority whose allowed domain prefixes cover `checkpoint_domain_id`
/// and whose lifetime covers the candidate checkpoint. This pure value binds
/// the exact authority, store, profile, principal, and subject-key coordinates;
/// it does not fetch an authority record, evaluate expiry, or verify a
/// signature by itself.
#[derive(Clone, Copy)]
pub struct ControlCheckpointSigningAuthorityAdmissionContextV1<'a> {
    /// Immutable reference to the resolved checkpoint-signing authority.
    pub authority_ref: &'a ResidentRecordRefV1,
    /// Digest of the exact resolved authority bytes.
    pub authority_sha256: &'a CheckpointSigningAuthorityDigest,
    /// Principal that owns the resolved authority.
    pub principal_id: &'a ResidentPrincipalIdV1,
    /// Durable control store covered by the authority.
    pub control_store_id: &'a ControlStoreIdV1,
    /// Exact store incarnation covered by the authority.
    pub control_incarnation_id: &'a ControlIncarnationIdV1,
    /// Exact hard-conflict domain admitted from the authority's prefix set.
    pub checkpoint_domain_id: &'a ControlCheckpointDomainIdV1,
    /// Immutable accumulator-profile reference selected by the authority.
    pub checkpoint_profile_ref: &'a ResidentRecordRefV1,
    /// Digest of the exact selected accumulator profile.
    pub checkpoint_profile_sha256: &'a CheckpointProfileDigest,
    /// Subject key delegated to sign control checkpoints.
    pub subject_key_id: &'a ResidentKeyIdV1,
    /// Positive epoch of the delegated subject key.
    pub subject_key_epoch: SigningKeyEpochV1,
}

/// Canonical key of one authenticated-accumulator leaf changed by a transaction.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ChangedControlLeafKeyV1 {
    /// Record class selecting the leaf encoding.
    pub record_class: ControlRecordClassV1,
    /// Digest of the typed scope key.
    pub scope_key_sha256: ArtifactDigest,
    /// Stable logical key within that scope.
    pub logical_key: ControlLogicalKeyV1,
}

resident_signed_record!(
    /// Signed authenticated-accumulator frontier for one hard conflict domain.
    ControlStateCheckpointV1, ControlStateCheckpointWireV1, ControlStateCheckpointPreimageV1,
    canonical_control_checkpoint_sha256: ControlStateCheckpointDigest,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA, "control state checkpoint",
    {
        /// Stable identity of this checkpoint record.
        control_checkpoint_id: ControlCheckpointIdV1,
        /// Durable control store whose state is authenticated.
        control_store_id: ControlStoreIdV1,
        /// Exact store incarnation, preventing replay across replacement stores.
        control_incarnation_id: ControlIncarnationIdV1,
        /// Hard-conflict domain covered by this accumulator frontier.
        checkpoint_domain_id: ControlCheckpointDomainIdV1,
        /// Positive domain epoch fencing incompatible domain histories.
        domain_epoch: ControlDomainEpochV1,
        /// Positive sequence within the selected domain epoch.
        domain_sequence: ControlDomainSequenceV1,
        /// Prior checkpoint reference, absent only at sequence one.
        previous_control_checkpoint_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_control_checkpoint_ref`].
        previous_control_checkpoint_sha256: Option<ControlStateCheckpointDigest>,
        /// Immutable accumulator-profile reference defining root construction.
        checkpoint_profile_ref: ResidentRecordRefV1,
        /// Digest of the exact checkpoint-profile bytes.
        checkpoint_profile_sha256: CheckpointProfileDigest,
        /// Positive control-record schema version represented by the root.
        control_schema_version: u32,
        /// Last durable transaction included in this checkpoint.
        committed_transaction_sequence: ControlTransactionSequenceV1,
        /// Authenticated root of every live control-state leaf.
        state_root_sha256: ArtifactDigest,
        /// Total leaf count committed beneath [`Self::state_root_sha256`].
        leaf_count: ControlLeafCountV1,
        /// Canonically ordered keys changed by the checkpointed transaction.
        changed_leaf_keys: Vec<ChangedControlLeafKeyV1>,
        /// Authenticated root of the unresolved reconciliation-custody projection.
        unresolved_custody_root_sha256: ArtifactDigest,
        /// Checkpoint issue time in Unix seconds.
        issued_at: UnixTimeV1,
        /// Immutable authority record governing the checkpoint signature.
        checkpoint_signing_authority_ref: ResidentRecordRefV1,
        /// Digest of the checkpoint-signing authority bytes.
        checkpoint_signing_authority_sha256: CheckpointSigningAuthorityDigest,
        /// Principal under whose authority the checkpoint was issued.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Key identity used for the checkpoint signature.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch checked at checkpoint admission.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_checkpoint_shape
);

impl ControlStateCheckpointV1 {
    /// Sign the first checkpoint under an owner-resolved schema and signing authority.
    ///
    /// Applies the same structural and contextual checks as
    /// [`Self::admit_initial_wire`] before calling `sign` with the signature
    /// domain, complete preimage, and canonical digest. The callback supplies
    /// the signature; this constructor does not verify it.
    ///
    /// Issuance does not persist or anchor the checkpoint, prove its roots, or
    /// resolve authority lifetime. The caller must establish those separately.
    ///
    /// # Errors
    ///
    /// Returns an error before signing if the record shape, schema, authority,
    /// or initial sequence is invalid. Returns signing failures unchanged.
    pub fn issue_initial_contextual<Sign>(
        preimage: ControlStateCheckpointPreimageV1,
        expected_schema_sha256: &SchemaDigest,
        signing_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        validate_checkpoint_preimage(&preimage)?;
        validate_checkpoint_signing_coordinates(
            &preimage,
            expected_schema_sha256,
            signing_authority,
        )?;
        validate_initial_checkpoint_sequence(&preimage)?;
        let digest = ControlStateCheckpointDigest::hash_canonical(&preimage)?;
        let signature = sign(
            CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN,
            &preimage,
            &digest,
        )?;
        Self::seal(preimage, signature)
    }

    /// Sign an exact successor of an admitted checkpoint.
    ///
    /// Rechecks the predecessor and applies the structural and contextual
    /// relation used by [`Self::admit_successor_wire`] before calling `sign`.
    /// The callback receives the signature domain, complete preimage, and
    /// canonical digest. The constructor does not verify the returned signature.
    ///
    /// Like [`Self::issue_initial_contextual`], this does not persist or anchor
    /// the checkpoint, prove its roots, or resolve authority lifetime.
    ///
    /// # Errors
    ///
    /// Returns an error before signing if either record is invalid, the
    /// resolved schema or authority differs, or the successor relation fails.
    /// Returns signing failures unchanged.
    pub fn issue_successor_contextual<Sign>(
        preimage: ControlStateCheckpointPreimageV1,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &Self,
        expected_schema_sha256: &SchemaDigest,
        signing_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        validate_checkpoint_preimage(&preimage)?;
        validate_checkpoint_signing_coordinates(
            &preimage,
            expected_schema_sha256,
            signing_authority,
        )?;
        predecessor.verify()?;
        validate_checkpoint_successor(
            CheckpointSuccessorView::from(&preimage),
            predecessor_ref,
            predecessor,
        )?;
        let digest = ControlStateCheckpointDigest::hash_canonical(&preimage)?;
        let signature = sign(
            CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN,
            &preimage,
            &digest,
        )?;
        Self::seal(preimage, signature)
    }

    /// Recheck the complete pure successor relation against an admitted predecessor.
    ///
    /// This is the single reusable relation used after contextual wire admission.
    /// It covers the exact predecessor, immutable schema/store/domain/profile
    /// lineage, monotone transaction and issue-time coordinates, and both
    /// records' canonical self-digests. It deliberately does not prove that the
    /// checkpoint accumulator represents a particular application mutation;
    /// that requires an owner-produced construction witness at the application
    /// boundary.
    /// Here `q` is domain sequence; `a` is the record-schema digest; `s`, `i`, `d`, and `e` are the store,
    /// incarnation, domain, and domain-epoch coordinates; `p` and `h` are the
    /// profile reference and digest; `v` is the control-schema version; `r` is
    /// predecessor lineage; and `x` and `t` are committed-transaction sequence
    /// and issue time. Primed symbols denote the candidate successor and `C`
    /// denotes the predecessor checkpoint.
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/control-state-checkpoint-successor.display.mathml"
    ))]
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when either record fails its
    /// canonical self-check or this checkpoint is not the exact successor of
    /// `predecessor` at `predecessor_ref`.
    pub fn verify_successor_of(
        &self,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &Self,
    ) -> Result<(), InvalidResidentRecordV1> {
        predecessor.verify()?;
        self.verify()?;
        validate_checkpoint_successor(
            CheckpointSuccessorView::from(self),
            predecessor_ref,
            predecessor,
        )
    }

    /// Admit checkpoint bytes under exact schema and resolved signing authority.
    ///
    /// This checks shape, canonical identity, signing coordinates and signature,
    /// but not succession, ancestry, accumulator construction or independent
    /// anchoring. A recovery owner must independently bind the admitted record
    /// to its actual retained checkpoint and anchor. Transitions still require
    /// [`Self::admit_initial_wire`] or [`Self::admit_successor_wire`].
    ///
    /// # Errors
    /// Returns an error for invalid shape, unlike schema or signing coordinates,
    /// or a rejected signature. The verifier must authenticate the exact domain,
    /// preimage and digest using the resolved signing authority.
    pub fn admit_signed_wire<F>(
        wire: ControlStateCheckpointWireV1,
        expected_schema_sha256: &SchemaDigest,
        signing_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_checkpoint_signing_coordinates(
                preimage,
                expected_schema_sha256,
                signing_authority,
            )?;
            verify_signature(
                CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }

    /// Admit the first checkpoint for a domain under exact schema and signing authority.
    ///
    /// # Errors
    ///
    /// Returns an error when the wire is structurally invalid, its schema or
    /// signing coordinates differ from the resolved authority, or signature
    /// verification fails.
    pub fn admit_initial_wire<F>(
        wire: ControlStateCheckpointWireV1,
        expected_schema_sha256: &SchemaDigest,
        signing_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_checkpoint_signing_coordinates(
                preimage,
                expected_schema_sha256,
                signing_authority,
            )?;
            validate_initial_checkpoint_sequence(preimage)?;
            verify_signature(
                CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }

    /// Admit an exact successor checkpoint against its resolved predecessor.
    ///
    /// Wire admission applies the same pure relation documented by
    /// [`Self::verify_successor_of`] before owner-resolved signature admission.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when signature admission fails, the
    /// domain sequence does not advance exactly once, immutable store/domain/
    /// profile fields change, transaction progress regresses, or issue time
    /// moves backwards.
    pub fn admit_successor_wire<F>(
        wire: ControlStateCheckpointWireV1,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &Self,
        expected_schema_sha256: &SchemaDigest,
        signing_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_checkpoint_signing_coordinates(
                preimage,
                expected_schema_sha256,
                signing_authority,
            )?;
            validate_checkpoint_successor(
                CheckpointSuccessorView::from(preimage),
                predecessor_ref,
                predecessor,
            )?;
            verify_signature(
                CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

/// Check the schema and record shape before requesting a signature.
fn validate_checkpoint_preimage(
    preimage: &ControlStateCheckpointPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if preimage.schema_id.as_str() != CONTROL_STATE_CHECKPOINT_V1_SCHEMA {
        return Err(InvalidResidentRecordV1::SchemaCoordinate {
            family: "control state checkpoint",
        });
    }
    validate_checkpoint_shape(preimage)
}

fn validate_checkpoint_shape(
    value: &ControlStateCheckpointPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    strictly_sorted("changed control leaf keys", &value.changed_leaf_keys)?;
    if value.control_schema_version == 0
        || value.previous_control_checkpoint_ref.is_some()
            != value.previous_control_checkpoint_sha256.is_some()
        || (value.domain_sequence.get() == 1) != value.previous_control_checkpoint_ref.is_none()
        || value.changed_leaf_keys.len() as u64 > value.leaf_count.get()
    {
        Err(InvalidResidentRecordV1::InvalidCoordinate {
            coordinate: "control checkpoint frontier",
        })
    } else {
        Ok(())
    }
}

fn validate_initial_checkpoint_sequence(
    preimage: &ControlStateCheckpointPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if preimage.domain_sequence.get() != 1 {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "control state checkpoint",
            invariant: "initial domain sequence",
        })
    } else {
        Ok(())
    }
}

/// Bind a checkpoint candidate to one exact owner-resolved signer witness.
fn validate_checkpoint_signing_coordinates(
    value: &ControlStateCheckpointPreimageV1,
    expected_schema_sha256: &SchemaDigest,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    if value.schema_sha256 != *expected_schema_sha256
        || value.checkpoint_signing_authority_ref != *authority.authority_ref
        || value.checkpoint_signing_authority_sha256 != *authority.authority_sha256
        || value.issuer_principal_id != *authority.principal_id
        || value.control_store_id != *authority.control_store_id
        || value.control_incarnation_id != *authority.control_incarnation_id
        || value.checkpoint_domain_id != *authority.checkpoint_domain_id
        || value.checkpoint_profile_ref != *authority.checkpoint_profile_ref
        || value.checkpoint_profile_sha256 != *authority.checkpoint_profile_sha256
        || value.issuer_key_id != *authority.subject_key_id
        || value.issuer_key_epoch != authority.subject_key_epoch
    {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "control state checkpoint",
            invariant: "exact schema and active checkpoint-signing authority",
        })
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct CheckpointSuccessorView<'a> {
    schema_sha256: &'a SchemaDigest,
    control_store_id: &'a ControlStoreIdV1,
    control_incarnation_id: &'a ControlIncarnationIdV1,
    checkpoint_domain_id: &'a ControlCheckpointDomainIdV1,
    domain_epoch: ControlDomainEpochV1,
    domain_sequence: ControlDomainSequenceV1,
    previous_ref: Option<&'a ResidentRecordRefV1>,
    previous_sha256: Option<&'a ControlStateCheckpointDigest>,
    checkpoint_profile_ref: &'a ResidentRecordRefV1,
    checkpoint_profile_sha256: &'a CheckpointProfileDigest,
    control_schema_version: u32,
    committed_transaction_sequence: ControlTransactionSequenceV1,
    issued_at: UnixTimeV1,
}

impl<'a> From<&'a ControlStateCheckpointPreimageV1> for CheckpointSuccessorView<'a> {
    fn from(value: &'a ControlStateCheckpointPreimageV1) -> Self {
        Self {
            schema_sha256: &value.schema_sha256,
            control_store_id: &value.control_store_id,
            control_incarnation_id: &value.control_incarnation_id,
            checkpoint_domain_id: &value.checkpoint_domain_id,
            domain_epoch: value.domain_epoch,
            domain_sequence: value.domain_sequence,
            previous_ref: value.previous_control_checkpoint_ref.as_ref(),
            previous_sha256: value.previous_control_checkpoint_sha256.as_ref(),
            checkpoint_profile_ref: &value.checkpoint_profile_ref,
            checkpoint_profile_sha256: &value.checkpoint_profile_sha256,
            control_schema_version: value.control_schema_version,
            committed_transaction_sequence: value.committed_transaction_sequence,
            issued_at: value.issued_at,
        }
    }
}

impl<'a> From<&'a ControlStateCheckpointV1> for CheckpointSuccessorView<'a> {
    fn from(value: &'a ControlStateCheckpointV1) -> Self {
        Self {
            schema_sha256: value.schema_sha256(),
            control_store_id: value.control_store_id(),
            control_incarnation_id: value.control_incarnation_id(),
            checkpoint_domain_id: value.checkpoint_domain_id(),
            domain_epoch: *value.domain_epoch(),
            domain_sequence: *value.domain_sequence(),
            previous_ref: value.previous_control_checkpoint_ref().as_ref(),
            previous_sha256: value.previous_control_checkpoint_sha256().as_ref(),
            checkpoint_profile_ref: value.checkpoint_profile_ref(),
            checkpoint_profile_sha256: value.checkpoint_profile_sha256(),
            control_schema_version: *value.control_schema_version(),
            committed_transaction_sequence: *value.committed_transaction_sequence(),
            issued_at: *value.issued_at(),
        }
    }
}

fn validate_checkpoint_successor(
    value: CheckpointSuccessorView<'_>,
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &ControlStateCheckpointV1,
) -> Result<(), InvalidResidentRecordV1> {
    let reject = |invariant| InvalidResidentRecordV1::ContextualAdmission {
        record: "control checkpoint successor",
        invariant,
    };
    if predecessor
        .domain_sequence()
        .checked_successor()
        .is_none_or(|next| value.domain_sequence != next)
    {
        return Err(reject("exact next domain sequence"));
    }
    if value.previous_ref != Some(predecessor_ref)
        || value.previous_sha256 != Some(predecessor.canonical_control_checkpoint_sha256())
    {
        return Err(reject("exact predecessor reference"));
    }
    if value.schema_sha256 != predecessor.schema_sha256()
        || value.control_store_id != predecessor.control_store_id()
        || value.control_incarnation_id != predecessor.control_incarnation_id()
        || value.checkpoint_domain_id != predecessor.checkpoint_domain_id()
        || value.domain_epoch != *predecessor.domain_epoch()
        || value.checkpoint_profile_ref != predecessor.checkpoint_profile_ref()
        || value.checkpoint_profile_sha256 != predecessor.checkpoint_profile_sha256()
        || value.control_schema_version != *predecessor.control_schema_version()
    {
        return Err(reject(
            "immutable schema, store, domain, and profile lineage",
        ));
    }
    if value.committed_transaction_sequence < *predecessor.committed_transaction_sequence()
        || value.issued_at < *predecessor.issued_at()
    {
        return Err(reject("monotone transaction and issue time"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/internal/records/resident/control.rs"]
mod tests;
