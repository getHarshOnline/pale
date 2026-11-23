//! Pure admission algebra for the initial principal-trust transaction.
//!
//! Bootstrap is deliberately split into consuming stages: an admitted
//! principal pair becomes an admitted authority set, then an admitted trust
//! frontier, and finally an admitted bootstrap set. A caller cannot construct
//! a later stage while omitting an earlier proof. The complete consuming-stage
//! chain is rendered below. `G` is admitted principal genesis; authority
//! candidates `A` produce admitted authority set `A_1`; trust checkpoint `T`
//! and initial control checkpoints `C_1` produce frontier `F_1`; receipt `rho`
//! produces complete bootstrap `B_1`. `P`, `R`, and `K` are principal,
//! recovery-policy, and candidate-key inputs.

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/principal-bootstrap-admission.display.mathml"
))]
//!
//! These morphisms validate immutable records only. They do not write `SQLite`,
//! advance an anti-rollback anchor, generate key material, open IPC, or report
//! resident activation.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::*;
use crate::records::*;

/// Public schema root for principal-trust checkpoint wires.
pub const PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA_ID: &str = "principal-trust-checkpoint.v1";
/// Wire schema carried by every principal-trust checkpoint.
pub const PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA: &str = "pale/principal-trust-checkpoint.v1";
/// Signature domain for [`PrincipalTrustCheckpointV1`].
pub const PRINCIPAL_TRUST_CHECKPOINT_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_PRINCIPAL_TRUST_CHECKPOINT_V1\0";

/// Public schema root for principal-bootstrap receipt wires.
pub const PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SCHEMA_ID: &str = "principal-bootstrap-receipt.v1";
/// Wire schema carried by every principal-bootstrap receipt.
pub const PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SCHEMA: &str = "pale/principal-bootstrap-receipt.v1";
/// Signature domain for [`PrincipalBootstrapReceiptV1`].
pub const PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_PRINCIPAL_BOOTSTRAP_RECEIPT_V1\0";

resident_signed_record!(
    /// Signed root of the complete principal-owned trust frontier.
    PrincipalTrustCheckpointV1,
    PrincipalTrustCheckpointWireV1,
    PrincipalTrustCheckpointPreimageV1,
    canonical_trust_checkpoint_sha256: PrincipalTrustCheckpointDigest,
    PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
    "principal trust checkpoint",
    {
        /// Stable identity of this trust-checkpoint record.
        trust_checkpoint_id: PrincipalTrustCheckpointIdV1,
        /// Principal whose trust frontier is authenticated.
        principal_id: ResidentPrincipalIdV1,
        /// Principal trust store carrying the authoritative heads.
        trust_store_id: ControlStoreIdV1,
        /// Exact trust-store incarnation fenced against restored bytes.
        trust_store_incarnation_id: ControlIncarnationIdV1,
        /// Principal-local control store whose hard domains are checkpointed.
        principal_control_store_id: ControlStoreIdV1,
        /// Exact principal-control-store incarnation.
        principal_control_incarnation_id: ControlIncarnationIdV1,
        /// Positive trust-checkpoint succession epoch.
        checkpoint_epoch: PrincipalTrustCheckpointEpochV1,
        /// Exact predecessor checkpoint, absent only at epoch one.
        previous_trust_checkpoint_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_trust_checkpoint_ref`].
        previous_trust_checkpoint_sha256: Option<PrincipalTrustCheckpointDigest>,
        /// Active accepted principal record.
        principal_record_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the principal record.
        principal_record_sha256: PrincipalRecordDigest,
        /// Active accepted principal recovery policy.
        recovery_policy_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the recovery policy.
        recovery_policy_sha256: PrincipalRecoveryPolicyDigest,
        /// Active local-administration authority.
        local_admin_authority_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the local-admin authority.
        local_admin_authority_sha256: LocalAdminAuthorityDigest,
        /// Active producer-incarnation authority.
        producer_incarnation_authority_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the producer authority.
        producer_incarnation_authority_sha256: ProducerIncarnationAuthorityDigest,
        /// Canonical root of current producer-incarnation allocation heads.
        ///
        /// A retired allocation remains the current head until an admitted
        /// successor replaces it; retirement blocks new rows without erasing
        /// immutable predecessor custody.
        producer_incarnation_heads_root_sha256: ProducerIncarnationHeadsRootDigest,
        /// Minimum session epoch accepted after trust recovery.
        session_floor_epoch: PrincipalSessionFloorEpochV1,
        /// Canonical root of active checkpoint-signing authority heads.
        checkpoint_signing_authority_heads_root_sha256: CheckpointSigningAuthorityHeadsRootDigest,
        /// Canonical root of active deployment-runtime authority heads.
        deployment_runtime_authority_heads_root_sha256: DeploymentRuntimeAuthorityHeadsRootDigest,
        /// Canonical root of active owner-record signing-authority heads.
        owner_record_signing_authority_heads_root_sha256: OwnerRecordSigningAuthorityHeadsRootDigest,
        /// Canonical root of owner-signer publication frontiers.
        owner_signer_publication_frontier_heads_root_sha256: OwnerSignerPublicationFrontierHeadsRootDigest,
        /// Canonical root of deployment-acceptance revocation heads.
        deployment_acceptance_revocation_heads_root_sha256: DeploymentAcceptanceRevocationHeadsRootDigest,
        /// Canonical root of federation-space home heads.
        federation_space_home_heads_root_sha256: FederationSpaceHomeHeadsRootDigest,
        /// Canonical root of principal node-delegation heads.
        node_delegation_heads_root_sha256: NodeDelegationHeadsRootDigest,
        /// Canonical root of principal node-delegation revocation heads.
        node_delegation_revocation_heads_root_sha256: NodeDelegationRevocationHeadsRootDigest,
        /// Unix issue time in whole seconds.
        issued_at: UnixTimeV1,
        /// Principal whose active accepted key issued this checkpoint.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Active accepted root-key identity.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive accepted root-key epoch.
        issuer_key_epoch: PrincipalKeyEpochV1
    },
    validate_trust_checkpoint_shape
);

impl PrincipalTrustCheckpointV1 {
    /// Recheck the operation-independent successor law against an admitted predecessor.
    ///
    /// This is the reusable relation shared by contextual wire admission and
    /// later application closure. It proves exact predecessor lineage, the
    /// next checkpoint epoch, stable principal and store incarnations,
    /// monotone issue time, and the checkpoint-principal issuer relation. It
    /// does not decide which trust-frontier fields a particular operation may
    /// change or verify the successor signature against a resolved key.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when either record fails its
    /// canonical self-check or `self` is not the exact generic successor of
    /// `predecessor` at `predecessor_ref`.
    pub fn verify_successor_of(
        &self,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &Self,
    ) -> Result<(), InvalidResidentRecordV1> {
        predecessor.verify()?;
        self.verify()?;
        validate_trust_checkpoint_successor(
            TrustCheckpointSuccessorView::from(self),
            predecessor_ref,
            predecessor,
            predecessor.schema_sha256(),
        )
    }

    /// Admit the exact next signed principal-trust frontier.
    ///
    /// This method owns the generic succession law: the principal and both
    /// store incarnations are stable, the epoch advances once, the predecessor
    /// ref/digest is exact, issue time is monotone, and the issuer remains the
    /// checkpoint principal. `validate_frontier` must additionally resolve
    /// every referenced head and enforce the selected operation's permitted
    /// field changes before the principal signature is accepted.
    /// In the law below, `e`, `p`, `s`, `i`, `r`, and `t` denote checkpoint
    /// epoch, principal, stable store coordinates, predecessor coordinate, and
    /// issue time; `C` is the predecessor checkpoint and primed symbols are the
    /// candidate values.
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/principal-trust-checkpoint-successor.display.mathml"
    ))]
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] for structural, schema, predecessor,
    /// epoch, store, contextual-frontier, digest, or signature failure.
    pub fn admit_successor_wire<C, S>(
        wire: PrincipalTrustCheckpointWireV1,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &Self,
        expected_schema_sha256: &SchemaDigest,
        validate_frontier: C,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        C: FnOnce(
            &PrincipalTrustCheckpointPreimageV1,
            &Self,
        ) -> Result<(), InvalidResidentRecordV1>,
        S: FnOnce(
            &[u8],
            &PrincipalTrustCheckpointPreimageV1,
            &PrincipalTrustCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        predecessor.verify()?;
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_trust_checkpoint_successor(
                TrustCheckpointSuccessorView::from(preimage),
                predecessor_ref,
                predecessor,
                expected_schema_sha256,
            )?;
            validate_frontier(preimage, predecessor)?;
            verify_signature(
                PRINCIPAL_TRUST_CHECKPOINT_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

#[derive(Clone, Copy)]
struct TrustCheckpointSuccessorView<'a> {
    schema_sha256: &'a SchemaDigest,
    checkpoint_epoch: PrincipalTrustCheckpointEpochV1,
    previous_ref: Option<&'a ResidentRecordRefV1>,
    previous_sha256: Option<&'a PrincipalTrustCheckpointDigest>,
    principal_id: &'a ResidentPrincipalIdV1,
    trust_store_id: &'a ControlStoreIdV1,
    trust_store_incarnation_id: &'a ControlIncarnationIdV1,
    principal_control_store_id: &'a ControlStoreIdV1,
    principal_control_incarnation_id: &'a ControlIncarnationIdV1,
    issued_at: UnixTimeV1,
    issuer_principal_id: &'a ResidentPrincipalIdV1,
}

impl<'a> From<&'a PrincipalTrustCheckpointPreimageV1> for TrustCheckpointSuccessorView<'a> {
    fn from(value: &'a PrincipalTrustCheckpointPreimageV1) -> Self {
        Self {
            schema_sha256: &value.schema_sha256,
            checkpoint_epoch: value.checkpoint_epoch,
            previous_ref: value.previous_trust_checkpoint_ref.as_ref(),
            previous_sha256: value.previous_trust_checkpoint_sha256.as_ref(),
            principal_id: &value.principal_id,
            trust_store_id: &value.trust_store_id,
            trust_store_incarnation_id: &value.trust_store_incarnation_id,
            principal_control_store_id: &value.principal_control_store_id,
            principal_control_incarnation_id: &value.principal_control_incarnation_id,
            issued_at: value.issued_at,
            issuer_principal_id: &value.issuer_principal_id,
        }
    }
}

impl<'a> From<&'a PrincipalTrustCheckpointV1> for TrustCheckpointSuccessorView<'a> {
    fn from(value: &'a PrincipalTrustCheckpointV1) -> Self {
        Self {
            schema_sha256: value.schema_sha256(),
            checkpoint_epoch: *value.checkpoint_epoch(),
            previous_ref: value.previous_trust_checkpoint_ref().as_ref(),
            previous_sha256: value.previous_trust_checkpoint_sha256().as_ref(),
            principal_id: value.principal_id(),
            trust_store_id: value.trust_store_id(),
            trust_store_incarnation_id: value.trust_store_incarnation_id(),
            principal_control_store_id: value.principal_control_store_id(),
            principal_control_incarnation_id: value.principal_control_incarnation_id(),
            issued_at: *value.issued_at(),
            issuer_principal_id: value.issuer_principal_id(),
        }
    }
}

/// Receipt projection for one required sequence-one hard-domain checkpoint.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct PrincipalBootstrapInitialControlCheckpointV1 {
    /// Exact hard-conflict domain initialized by bootstrap.
    pub checkpoint_domain_id: ControlCheckpointDomainIdV1,
    /// Initial domain epoch, exactly one.
    pub domain_epoch: ControlDomainEpochV1,
    /// Initial domain sequence, exactly one.
    pub domain_sequence: ControlDomainSequenceV1,
    /// Immutable control-checkpoint record reference.
    pub control_checkpoint_ref: ResidentRecordRefV1,
    /// Canonical unsigned-preimage digest of the control checkpoint.
    pub control_checkpoint_sha256: ControlStateCheckpointDigest,
}

resident_signed_record!(
    /// Root-signed receipt binding the database-atomic bootstrap candidate set.
    PrincipalBootstrapReceiptV1,
    PrincipalBootstrapReceiptWireV1,
    PrincipalBootstrapReceiptPreimageV1,
    canonical_bootstrap_receipt_sha256: PrincipalBootstrapReceiptDigest,
    PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SCHEMA,
    "principal bootstrap receipt",
    {
        /// Stable identity of the one-shot bootstrap transaction.
        bootstrap_id: PrincipalBootstrapIdV1,
        /// Principal trust store receiving the immutable candidate set.
        trust_store_id: ControlStoreIdV1,
        /// Exact trust-store incarnation receiving the candidate set.
        trust_store_incarnation_id: ControlIncarnationIdV1,
        /// Principal-local control store initialized by the transaction.
        principal_control_store_id: ControlStoreIdV1,
        /// Exact principal-control-store incarnation.
        principal_control_incarnation_id: ControlIncarnationIdV1,
        /// Initial principal-record reference.
        principal_record_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the principal record.
        principal_record_sha256: PrincipalRecordDigest,
        /// Initial recovery-policy reference.
        recovery_policy_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the recovery policy.
        recovery_policy_sha256: PrincipalRecoveryPolicyDigest,
        /// Initial local-administration authority reference.
        initial_admin_authority_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the local-admin authority.
        initial_admin_authority_sha256: LocalAdminAuthorityDigest,
        /// Initial producer-incarnation authority reference.
        initial_producer_incarnation_authority_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the producer authority.
        initial_producer_incarnation_authority_sha256: ProducerIncarnationAuthorityDigest,
        /// Bootstrap-scoped checkpoint-signing authority reference.
        initial_trust_store_checkpoint_signing_authority_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the checkpoint authority.
        initial_trust_store_checkpoint_signing_authority_sha256: CheckpointSigningAuthorityDigest,
        /// Initial principal-trust checkpoint reference.
        trust_checkpoint_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the trust checkpoint.
        trust_checkpoint_sha256: PrincipalTrustCheckpointDigest,
        /// Initial trust-checkpoint epoch, exactly one.
        trust_checkpoint_epoch: PrincipalTrustCheckpointEpochV1,
        /// Complete ordered sequence-one hard-domain checkpoint set.
        initial_control_checkpoints: Vec<PrincipalBootstrapInitialControlCheckpointV1>,
        /// Root-signed logical commit coordinate in Unix seconds.
        ///
        /// This pre-signed value orders the candidate after every member's issue
        /// time. It is not evidence that a physical database commit completed;
        /// durable read-back of the exact receipt-headed atom provides that proof.
        committed_at: UnixTimeV1,
        /// Candidate root key that signs this receipt.
        candidate_root_key_id: ResidentKeyIdV1
    },
    validate_bootstrap_receipt_shape
);

/// Owner-resolved immutable coordinates needed to admit bootstrap checkpoints.
///
/// The trust-store owner supplies this value after selecting the exact stores,
/// accumulator profile, schema set, and complete hard-domain inventory. It is
/// an admission witness, not a public wire record and not a source of defaults.
#[derive(Clone, Copy)]
pub struct PrincipalBootstrapCheckpointProfileV1<'a> {
    /// Principal trust store selected for the transaction.
    pub trust_store_id: &'a ControlStoreIdV1,
    /// Fresh trust-store incarnation selected for the transaction.
    pub trust_store_incarnation_id: &'a ControlIncarnationIdV1,
    /// Principal-local control store selected for hard checkpoints.
    pub principal_control_store_id: &'a ControlStoreIdV1,
    /// Fresh principal-control-store incarnation.
    pub principal_control_incarnation_id: &'a ControlIncarnationIdV1,
    /// Immutable accumulator-profile reference.
    pub checkpoint_profile_ref: &'a ResidentRecordRefV1,
    /// Digest of the exact accumulator-profile bytes.
    pub checkpoint_profile_sha256: &'a CheckpointProfileDigest,
    /// Complete strictly ordered initial hard-domain inventory.
    pub required_checkpoint_domains: &'a [ControlCheckpointDomainIdV1],
    /// Exact control-checkpoint wire schema identity.
    pub control_checkpoint_schema_sha256: &'a SchemaDigest,
    /// Positive control-record schema version represented by every checkpoint.
    pub control_schema_version: u32,
}

struct AdmittedInitialControlCheckpointV1 {
    record_ref: ResidentRecordRefV1,
    record: ControlStateCheckpointV1,
}

/// Untrusted checkpoint set presented for initial trust-frontier admission.
///
/// The profile supplies owner-selected checkpoint semantics; the records are
/// only candidates until [`AdmittedPrincipalAuthoritiesV1::admit_initial_trust_frontier`]
/// verifies the complete exact hard-domain set and every signature.
pub struct InitialPrincipalTrustFrontierCandidateV1<'profile> {
    /// Immutable reference assigned to the principal trust checkpoint.
    pub trust_checkpoint_ref: ResidentRecordRefV1,
    /// Signed but not yet admitted principal trust checkpoint.
    pub trust_wire: PrincipalTrustCheckpointWireV1,
    /// Expected schema identity for the principal trust checkpoint.
    pub expected_trust_schema_sha256: &'profile SchemaDigest,
    /// Owner-selected store and hard-domain profile.
    pub profile: PrincipalBootstrapCheckpointProfileV1<'profile>,
    /// Candidate control checkpoints paired with their immutable references.
    pub checkpoint_candidates: Vec<(ResidentRecordRefV1, ControlStateCheckpointWireV1)>,
}

/// Admitted trust checkpoint and complete initial hard-domain checkpoint set.
///
/// This value is still only a pure candidate for the storage transaction. See
/// [`Self::admit_receipt`] for the final cryptographic binding.
pub struct AdmittedPrincipalTrustFrontierV1 {
    authorities: AdmittedPrincipalAuthoritiesV1,
    trust_checkpoint_ref: ResidentRecordRefV1,
    trust_checkpoint: PrincipalTrustCheckpointV1,
    control_checkpoints: Vec<AdmittedInitialControlCheckpointV1>,
}

/// Initial signed trust head awaiting its containing control checkpoints.
///
/// Construction may use the admitted head and authorities to calculate the
/// checkpoint roots. This stage proves neither complete bootstrap admission
/// nor database custody. The exact selected profile remains borrowed until
/// [`Self::admit_control_checkpoints`] completes the frontier.
pub struct AdmittedInitialPrincipalTrustCheckpointV1<'profile> {
    authorities: AdmittedPrincipalAuthoritiesV1,
    trust_checkpoint_ref: ResidentRecordRefV1,
    trust_checkpoint: PrincipalTrustCheckpointV1,
    profile: PrincipalBootstrapCheckpointProfileV1<'profile>,
}

impl AdmittedPrincipalAuthoritiesV1 {
    /// Test membership in the five admitted trust records' validity intersection.
    ///
    /// Issue bounds are inclusive and expiry bounds exclusive. A recovery
    /// policy without expiry contributes no upper bound. This neither reads
    /// a clock nor re-verifies signatures.
    fn validity_contains(&self, time: UnixTimeV1) -> bool {
        let principal = self.genesis().principal_record();
        let policy = self.genesis().recovery_policy();

        time >= *principal.issued_at()
            && time < *principal.expires_at()
            && time >= *policy.issued_at()
            && policy.expires_at().is_none_or(|expiry| time < expiry)
            && time >= *self.local_admin_authority().issued_at()
            && time < *self.local_admin_authority().expires_at()
            && time >= *self.producer_authority().issued_at()
            && time < *self.producer_authority().expires_at()
            && time >= *self.checkpoint_authority().issued_at()
            && time < *self.checkpoint_authority().expires_at()
    }

    /// Admit the initial trust checkpoint and every required hard-domain checkpoint.
    ///
    /// `authority_covers_domain` interprets the owner-published domain-prefix
    /// grammar. Keeping that rule outside this neutral record crate prevents
    /// the Pale application from inventing prefix semantics.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when any record is not the exact
    /// epoch-one, no-predecessor, store/profile-bound candidate required by the
    /// supplied profile, or when a signature callback rejects its signature.
    pub fn admit_initial_trust_frontier<TS, CS, DC>(
        self,
        candidate: InitialPrincipalTrustFrontierCandidateV1<'_>,
        verify_trust_signature: TS,
        verify_control_signature: CS,
        authority_covers_domain: DC,
    ) -> Result<AdmittedPrincipalTrustFrontierV1, InvalidResidentRecordV1>
    where
        TS: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalTrustCheckpointPreimageV1,
            &PrincipalTrustCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        CS: FnMut(
            &[u8],
            &Ed25519PublicKeyV1,
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        DC: FnMut(&[CheckpointDomainPrefixV1], &ControlCheckpointDomainIdV1) -> bool,
    {
        let InitialPrincipalTrustFrontierCandidateV1 {
            trust_checkpoint_ref,
            trust_wire,
            expected_trust_schema_sha256,
            profile,
            checkpoint_candidates,
        } = candidate;
        self.admit_initial_trust_checkpoint(
            trust_checkpoint_ref,
            trust_wire,
            expected_trust_schema_sha256,
            profile,
            verify_trust_signature,
        )?
        .admit_control_checkpoints(
            checkpoint_candidates,
            verify_control_signature,
            authority_covers_domain,
        )
    }

    /// Admit the initial signed trust head before constructing its control roots.
    ///
    /// This applies the same principal, authority, store, empty-frontier and
    /// signature checks as [`Self::admit_initial_trust_frontier`]. It retains
    /// those inputs for the remaining control-checkpoint admission; callers
    /// cannot replace the admitted authorities or profile between the stages.
    ///
    /// # Errors
    /// Rejects a mismatched initial trust head, invalid profile domain inventory
    /// or rejected principal signature. No control root or custody is proved.
    pub fn admit_initial_trust_checkpoint<'profile, TS>(
        self,
        trust_checkpoint_ref: ResidentRecordRefV1,
        trust_wire: PrincipalTrustCheckpointWireV1,
        expected_trust_schema_sha256: &SchemaDigest,
        profile: PrincipalBootstrapCheckpointProfileV1<'profile>,
        verify_trust_signature: TS,
    ) -> Result<AdmittedInitialPrincipalTrustCheckpointV1<'profile>, InvalidResidentRecordV1>
    where
        TS: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalTrustCheckpointPreimageV1,
            &PrincipalTrustCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        strictly_sorted(
            "principal bootstrap hard domains",
            profile.required_checkpoint_domains,
        )?;
        if profile.required_checkpoint_domains.is_empty() || profile.control_schema_version == 0 {
            return Err(contextual(
                "principal bootstrap checkpoint profile",
                "non-empty canonical hard domains and positive control schema",
            ));
        }

        let root_key = self.genesis().principal_record().root_public_key();
        let trust_checkpoint =
            PrincipalTrustCheckpointV1::admit_wire(trust_wire, |preimage, digest, signature| {
                validate_initial_trust_context(
                    preimage,
                    &self,
                    expected_trust_schema_sha256,
                    &profile,
                )?;
                verify_trust_signature(
                    PRINCIPAL_TRUST_CHECKPOINT_V1_SIGNATURE_DOMAIN,
                    root_key,
                    preimage,
                    digest,
                    signature,
                )
            })?;

        let checkpoint_authority = self.checkpoint_authority();
        if checkpoint_authority.control_store_id() != profile.principal_control_store_id
            || checkpoint_authority.control_incarnation_id()
                != profile.principal_control_incarnation_id
            || checkpoint_authority.checkpoint_profile_ref() != profile.checkpoint_profile_ref
            || checkpoint_authority.checkpoint_profile_sha256() != profile.checkpoint_profile_sha256
        {
            return Err(contextual(
                "principal bootstrap checkpoint authority",
                "exact principal control store and accumulator profile",
            ));
        }

        Ok(AdmittedInitialPrincipalTrustCheckpointV1 {
            authorities: self,
            trust_checkpoint_ref,
            trust_checkpoint,
            profile,
        })
    }
}

impl AdmittedInitialPrincipalTrustCheckpointV1<'_> {
    /// Borrow the admitted principal pair and initial signing authorities.
    #[must_use]
    pub const fn authorities(&self) -> &AdmittedPrincipalAuthoritiesV1 {
        &self.authorities
    }

    /// Borrow the signed trust head and its exact immutable reference.
    #[must_use]
    pub const fn trust_checkpoint_entry(
        &self,
    ) -> (&ResidentRecordRefV1, &PrincipalTrustCheckpointV1) {
        (&self.trust_checkpoint_ref, &self.trust_checkpoint)
    }

    /// Borrow the selected stores, profile and complete hard-domain inventory.
    #[must_use]
    pub const fn profile(&self) -> &PrincipalBootstrapCheckpointProfileV1<'_> {
        &self.profile
    }

    /// Complete initial admission with the exact signed control-checkpoint set.
    ///
    /// The profile and authorities are those retained by initial trust admission.
    /// Signature verification and domain-prefix interpretation keep their
    /// existing owners. This does not prove the roots' construction or custody.
    ///
    /// # Errors
    /// Rejects missing or different domains, unlike authority/store/profile,
    /// invalid sequence-one checkpoints, or rejected signatures.
    pub fn admit_control_checkpoints<CS, DC>(
        self,
        checkpoint_candidates: Vec<(ResidentRecordRefV1, ControlStateCheckpointWireV1)>,
        mut verify_control_signature: CS,
        mut authority_covers_domain: DC,
    ) -> Result<AdmittedPrincipalTrustFrontierV1, InvalidResidentRecordV1>
    where
        CS: FnMut(
            &[u8],
            &Ed25519PublicKeyV1,
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        DC: FnMut(&[CheckpointDomainPrefixV1], &ControlCheckpointDomainIdV1) -> bool,
    {
        let Self {
            authorities,
            trust_checkpoint_ref,
            trust_checkpoint,
            profile,
        } = self;

        if checkpoint_candidates.len() != profile.required_checkpoint_domains.len() {
            return Err(contextual(
                "principal bootstrap control checkpoints",
                "complete exact required hard-domain set",
            ));
        }

        let checkpoint_authority = authorities.checkpoint_authority();
        let mut control_checkpoints = Vec::with_capacity(checkpoint_candidates.len());
        for ((record_ref, wire), required_domain) in checkpoint_candidates
            .into_iter()
            .zip(profile.required_checkpoint_domains)
        {
            if !authority_covers_domain(
                checkpoint_authority.allowed_domain_prefixes(),
                required_domain,
            ) {
                return Err(contextual(
                    "principal bootstrap control checkpoint",
                    "owner-resolved checkpoint authority covers required domain",
                ));
            }
            let signing_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
                authority_ref: authorities.checkpoint_authority_ref(),
                authority_sha256: checkpoint_authority
                    .canonical_checkpoint_signing_authority_sha256(),
                principal_id: checkpoint_authority.principal_id(),
                control_store_id: checkpoint_authority.control_store_id(),
                control_incarnation_id: checkpoint_authority.control_incarnation_id(),
                checkpoint_domain_id: required_domain,
                checkpoint_profile_ref: profile.checkpoint_profile_ref,
                checkpoint_profile_sha256: profile.checkpoint_profile_sha256,
                subject_key_id: checkpoint_authority.subject_key_id(),
                subject_key_epoch: *checkpoint_authority.subject_key_epoch(),
            };
            let record = ControlStateCheckpointV1::admit_initial_wire(
                wire,
                profile.control_checkpoint_schema_sha256,
                &signing_authority,
                |domain, preimage, digest, signature| {
                    verify_control_signature(
                        domain,
                        checkpoint_authority.subject_public_key(),
                        preimage,
                        digest,
                        signature,
                    )
                },
            )?;
            if record.control_schema_version() != &profile.control_schema_version
                || record.issued_at() < checkpoint_authority.issued_at()
                || record.issued_at() >= checkpoint_authority.expires_at()
            {
                return Err(contextual(
                    "principal bootstrap control checkpoint",
                    "exact control schema and signer validity interval",
                ));
            }
            control_checkpoints.push(AdmittedInitialControlCheckpointV1 { record_ref, record });
        }

        Ok(AdmittedPrincipalTrustFrontierV1 {
            authorities,
            trust_checkpoint_ref,
            trust_checkpoint,
            control_checkpoints,
        })
    }
}

impl AdmittedPrincipalTrustFrontierV1 {
    /// Iterate the admitted initial control checkpoints in canonical domain order.
    /// Construction and custody callers retain the original record identities.
    pub fn initial_control_checkpoints(
        &self,
    ) -> impl ExactSizeIterator<Item = (&ResidentRecordRefV1, &ControlStateCheckpointV1)> + '_ {
        self.control_checkpoints
            .iter()
            .map(|checkpoint| (&checkpoint.record_ref, &checkpoint.record))
    }

    /// Admit the root-signed receipt over this exact immutable candidate set.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when any receipt coordinate differs
    /// from this admitted frontier, the logical commit time precedes a member
    /// or falls outside a governing trust-root validity interval, or
    /// root-signature verification fails.
    pub fn admit_receipt<RS>(
        self,
        receipt_ref: ResidentRecordRefV1,
        receipt_wire: PrincipalBootstrapReceiptWireV1,
        expected_receipt_schema_sha256: &SchemaDigest,
        verify_receipt_signature: RS,
    ) -> Result<AdmittedPrincipalBootstrapV1, InvalidResidentRecordV1>
    where
        RS: FnOnce(
            &[u8],
            &Ed25519PublicKeyV1,
            &PrincipalBootstrapReceiptPreimageV1,
            &PrincipalBootstrapReceiptDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        validate_unique_bootstrap_record_refs(&self, &receipt_ref)?;
        let root_key = self
            .authorities
            .genesis()
            .principal_record()
            .root_public_key();
        let receipt = PrincipalBootstrapReceiptV1::admit_wire(
            receipt_wire,
            |preimage, digest, signature| {
                validate_receipt_context(preimage, &self, expected_receipt_schema_sha256)?;
                verify_receipt_signature(
                    PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SIGNATURE_DOMAIN,
                    root_key,
                    preimage,
                    digest,
                    signature,
                )
            },
        )?;
        Ok(AdmittedPrincipalBootstrapV1 {
            frontier: self,
            receipt_ref,
            receipt,
        })
    }
}

/// Require one immutable reference namespace across the database-atomic atom.
fn validate_unique_bootstrap_record_refs(
    frontier: &AdmittedPrincipalTrustFrontierV1,
    receipt_ref: &ResidentRecordRefV1,
) -> Result<(), InvalidResidentRecordV1> {
    let authorities = &frontier.authorities;
    let genesis = authorities.genesis();
    let mut refs = BTreeSet::new();
    for record_ref in [
        genesis.principal_record_ref(),
        genesis.recovery_policy_ref(),
        authorities.local_admin_authority_ref(),
        authorities.producer_authority_ref(),
        authorities.checkpoint_authority_ref(),
        &frontier.trust_checkpoint_ref,
        receipt_ref,
    ] {
        if !refs.insert(record_ref) {
            return Err(contextual(
                "principal bootstrap receipt",
                "globally unique immutable record references",
            ));
        }
    }
    if frontier
        .control_checkpoints
        .iter()
        .any(|checkpoint| !refs.insert(&checkpoint.record_ref))
    {
        Err(contextual(
            "principal bootstrap receipt",
            "globally unique immutable record references",
        ))
    } else {
        Ok(())
    }
}

/// Fully admitted immutable bootstrap set, still uncommitted by any store.
pub struct AdmittedPrincipalBootstrapV1 {
    frontier: AdmittedPrincipalTrustFrontierV1,
    receipt_ref: ResidentRecordRefV1,
    receipt: PrincipalBootstrapReceiptV1,
}

/// Borrowed, non-wire view of one completely admitted bootstrap atom.
///
/// This view exists so a concrete custody adapter can retain the exact signed
/// records without acquiring authority to construct, replace, or reinterpret
/// them. It has no independent identity and cannot outlive its admitted proof.
#[derive(Clone, Copy)]
pub struct PrincipalBootstrapAtomViewV1<'record> {
    authorities: &'record AdmittedPrincipalAuthoritiesV1,
    trust_checkpoint_ref: &'record ResidentRecordRefV1,
    trust_checkpoint: &'record PrincipalTrustCheckpointV1,
    control_checkpoints: &'record [AdmittedInitialControlCheckpointV1],
    receipt_ref: &'record ResidentRecordRefV1,
    receipt: &'record PrincipalBootstrapReceiptV1,
}

impl PrincipalBootstrapAtomViewV1<'_> {
    /// Borrow the admitted principal pair and three initial authorities.
    #[must_use]
    pub const fn authorities(&self) -> &AdmittedPrincipalAuthoritiesV1 {
        self.authorities
    }

    /// Borrow the admitted principal-trust checkpoint and its immutable reference.
    #[must_use]
    pub const fn trust_checkpoint_entry(
        &self,
    ) -> (&ResidentRecordRefV1, &PrincipalTrustCheckpointV1) {
        (self.trust_checkpoint_ref, self.trust_checkpoint)
    }

    /// Iterate the complete admitted hard-domain checkpoint set in canonical order.
    pub fn initial_control_checkpoints(
        &self,
    ) -> impl ExactSizeIterator<Item = (&ResidentRecordRefV1, &ControlStateCheckpointV1)> + '_ {
        self.control_checkpoints
            .iter()
            .map(|checkpoint| (&checkpoint.record_ref, &checkpoint.record))
    }

    /// Borrow the root-signed bootstrap receipt and its immutable reference.
    #[must_use]
    pub const fn receipt_entry(&self) -> (&ResidentRecordRefV1, &PrincipalBootstrapReceiptV1) {
        (self.receipt_ref, self.receipt)
    }
}

impl AdmittedPrincipalBootstrapV1 {
    /// Borrow the principal whose exact bootstrap set was admitted.
    #[must_use]
    pub fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        self.frontier
            .authorities
            .genesis()
            .principal_record()
            .principal_id()
    }

    /// Borrow the exact root-signed bootstrap receipt.
    #[must_use]
    pub fn receipt(&self) -> &PrincipalBootstrapReceiptV1 {
        &self.receipt
    }

    /// Borrow the immutable bootstrap-receipt reference.
    #[must_use]
    pub fn receipt_ref(&self) -> &ResidentRecordRefV1 {
        &self.receipt_ref
    }

    /// Borrow the admitted principal-trust checkpoint.
    #[must_use]
    pub fn trust_checkpoint(&self) -> &PrincipalTrustCheckpointV1 {
        &self.frontier.trust_checkpoint
    }

    /// Borrow the complete admitted atom for exact immutable custody.
    #[must_use]
    pub fn atom_view(&self) -> PrincipalBootstrapAtomViewV1<'_> {
        PrincipalBootstrapAtomViewV1 {
            authorities: &self.frontier.authorities,
            trust_checkpoint_ref: &self.frontier.trust_checkpoint_ref,
            trust_checkpoint: &self.frontier.trust_checkpoint,
            control_checkpoints: &self.frontier.control_checkpoints,
            receipt_ref: &self.receipt_ref,
            receipt: &self.receipt,
        }
    }
}

fn validate_trust_checkpoint_shape(
    value: &PrincipalTrustCheckpointPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    let predecessor_pair = value.previous_trust_checkpoint_ref.is_some()
        == value.previous_trust_checkpoint_sha256.is_some();
    let initial_shape =
        (value.checkpoint_epoch.get() == 1) == value.previous_trust_checkpoint_ref.is_none();
    if predecessor_pair && initial_shape {
        Ok(())
    } else {
        Err(contextual(
            "principal trust checkpoint",
            "epoch and predecessor lineage",
        ))
    }
}

/// Validate the operation-independent principal-trust succession law.
fn validate_trust_checkpoint_successor(
    value: TrustCheckpointSuccessorView<'_>,
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &PrincipalTrustCheckpointV1,
    expected_schema_sha256: &SchemaDigest,
) -> Result<(), InvalidResidentRecordV1> {
    let reject = |invariant| contextual("principal trust checkpoint successor", invariant);
    if predecessor
        .checkpoint_epoch()
        .checked_successor()
        .is_none_or(|expected| value.checkpoint_epoch != expected)
    {
        return Err(reject("exact next checkpoint epoch"));
    }
    if value.previous_ref != Some(predecessor_ref)
        || value.previous_sha256 != Some(predecessor.canonical_trust_checkpoint_sha256())
    {
        return Err(reject("exact predecessor reference"));
    }
    if value.schema_sha256 != expected_schema_sha256
        || value.schema_sha256 != predecessor.schema_sha256()
        || value.principal_id != predecessor.principal_id()
        || value.trust_store_id != predecessor.trust_store_id()
        || value.trust_store_incarnation_id != predecessor.trust_store_incarnation_id()
        || value.principal_control_store_id != predecessor.principal_control_store_id()
        || value.principal_control_incarnation_id != predecessor.principal_control_incarnation_id()
        || value.issued_at < *predecessor.issued_at()
        || value.issuer_principal_id != value.principal_id
    {
        return Err(reject(
            "exact schema, principal, store incarnations, issue time, and issuer",
        ));
    }
    Ok(())
}

fn validate_bootstrap_receipt_shape(
    value: &PrincipalBootstrapReceiptPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    strictly_sorted(
        "principal bootstrap initial checkpoints",
        &value.initial_control_checkpoints,
    )?;
    if value.initial_control_checkpoints.is_empty()
        || value.trust_checkpoint_epoch.get() != 1
        || value.initial_control_checkpoints.iter().any(|checkpoint| {
            checkpoint.domain_epoch.get() != 1 || checkpoint.domain_sequence.get() != 1
        })
    {
        Err(contextual(
            "principal bootstrap receipt",
            "non-empty canonical sequence-one checkpoint set",
        ))
    } else {
        Ok(())
    }
}

fn validate_initial_trust_context(
    value: &PrincipalTrustCheckpointPreimageV1,
    authorities: &AdmittedPrincipalAuthoritiesV1,
    expected_schema_sha256: &SchemaDigest,
    profile: &PrincipalBootstrapCheckpointProfileV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    let genesis = authorities.genesis();
    let principal = genesis.principal_record();
    let policy = genesis.recovery_policy();
    let empty = [] as [(); 0];

    if value.schema_sha256 != *expected_schema_sha256
        || value.checkpoint_epoch.get() != 1
        || value.principal_id != *principal.principal_id()
        || value.trust_store_id != *profile.trust_store_id
        || value.trust_store_incarnation_id != *profile.trust_store_incarnation_id
        || value.principal_control_store_id != *profile.principal_control_store_id
        || value.principal_control_incarnation_id != *profile.principal_control_incarnation_id
        || value.principal_record_ref != *genesis.principal_record_ref()
        || value.principal_record_sha256 != *principal.canonical_record_sha256()
        || value.recovery_policy_ref != *genesis.recovery_policy_ref()
        || value.recovery_policy_sha256 != *policy.canonical_record_sha256()
        || value.local_admin_authority_ref != *authorities.local_admin_authority_ref()
        || value.local_admin_authority_sha256
            != *authorities
                .local_admin_authority()
                .canonical_admin_authority_sha256()
        || value.producer_incarnation_authority_ref != *authorities.producer_authority_ref()
        || value.producer_incarnation_authority_sha256
            != *authorities
                .producer_authority()
                .canonical_producer_authority_sha256()
        || value.producer_incarnation_heads_root_sha256
            != ProducerIncarnationHeadsRootDigest::hash_canonical(&empty)?
        || value.session_floor_epoch.get() != 1
        || value.checkpoint_signing_authority_heads_root_sha256
            != authorities.checkpoint_authority_heads_root()?
        || value.deployment_runtime_authority_heads_root_sha256
            != deployment_runtime_authority_heads_root_sha256(&[])?
        || value.owner_record_signing_authority_heads_root_sha256
            != OwnerRecordSigningAuthorityHeadsRootDigest::hash_canonical(&empty)?
        || value.owner_signer_publication_frontier_heads_root_sha256
            != OwnerSignerPublicationFrontierHeadsRootDigest::hash_canonical(&empty)?
        || value.deployment_acceptance_revocation_heads_root_sha256
            != DeploymentAcceptanceRevocationHeadsRootDigest::hash_canonical(&empty)?
        || value.federation_space_home_heads_root_sha256
            != FederationSpaceHomeHeadsRootDigest::hash_canonical(&empty)?
        || value.node_delegation_heads_root_sha256
            != NodeDelegationHeadsRootDigest::hash_canonical(&empty)?
        || value.node_delegation_revocation_heads_root_sha256
            != NodeDelegationRevocationHeadsRootDigest::hash_canonical(&empty)?
        || value.issuer_principal_id != *principal.principal_id()
        || value.issuer_key_id != *principal.root_key_id()
        || value.issuer_key_epoch != *principal.key_epoch()
        || !authorities.validity_contains(value.issued_at)
    {
        Err(contextual(
            "principal trust checkpoint",
            "exact initial principal, authority, store, and empty-head frontier",
        ))
    } else {
        Ok(())
    }
}

fn validate_receipt_context(
    value: &PrincipalBootstrapReceiptPreimageV1,
    frontier: &AdmittedPrincipalTrustFrontierV1,
    expected_schema_sha256: &SchemaDigest,
) -> Result<(), InvalidResidentRecordV1> {
    let authorities = &frontier.authorities;
    let genesis = authorities.genesis();
    let principal = genesis.principal_record();
    let expected_checkpoints = frontier
        .control_checkpoints
        .iter()
        .map(|checkpoint| PrincipalBootstrapInitialControlCheckpointV1 {
            checkpoint_domain_id: checkpoint.record.checkpoint_domain_id().clone(),
            domain_epoch: *checkpoint.record.domain_epoch(),
            domain_sequence: *checkpoint.record.domain_sequence(),
            control_checkpoint_ref: checkpoint.record_ref.clone(),
            control_checkpoint_sha256: checkpoint
                .record
                .canonical_control_checkpoint_sha256()
                .clone(),
        })
        .collect::<Vec<_>>();
    let latest_issue_time = frontier
        .control_checkpoints
        .iter()
        .map(|checkpoint| *checkpoint.record.issued_at())
        .fold(*frontier.trust_checkpoint.issued_at(), Ord::max);

    if value.schema_sha256 != *expected_schema_sha256
        || value.bootstrap_id
            != bootstrap_id_from_checkpoint_scope(authorities.checkpoint_authority())?
        || value.trust_store_id != *frontier.trust_checkpoint.trust_store_id()
        || value.trust_store_incarnation_id
            != *frontier.trust_checkpoint.trust_store_incarnation_id()
        || value.principal_control_store_id
            != *frontier.trust_checkpoint.principal_control_store_id()
        || value.principal_control_incarnation_id
            != *frontier.trust_checkpoint.principal_control_incarnation_id()
        || value.principal_record_ref != *genesis.principal_record_ref()
        || value.principal_record_sha256 != *principal.canonical_record_sha256()
        || value.recovery_policy_ref != *genesis.recovery_policy_ref()
        || value.recovery_policy_sha256 != *genesis.recovery_policy().canonical_record_sha256()
        || value.initial_admin_authority_ref != *authorities.local_admin_authority_ref()
        || value.initial_admin_authority_sha256
            != *authorities
                .local_admin_authority()
                .canonical_admin_authority_sha256()
        || value.initial_producer_incarnation_authority_ref != *authorities.producer_authority_ref()
        || value.initial_producer_incarnation_authority_sha256
            != *authorities
                .producer_authority()
                .canonical_producer_authority_sha256()
        || value.initial_trust_store_checkpoint_signing_authority_ref
            != *authorities.checkpoint_authority_ref()
        || value.initial_trust_store_checkpoint_signing_authority_sha256
            != *authorities
                .checkpoint_authority()
                .canonical_checkpoint_signing_authority_sha256()
        || value.trust_checkpoint_ref != frontier.trust_checkpoint_ref
        || value.trust_checkpoint_sha256
            != *frontier
                .trust_checkpoint
                .canonical_trust_checkpoint_sha256()
        || value.trust_checkpoint_epoch != *frontier.trust_checkpoint.checkpoint_epoch()
        || value.initial_control_checkpoints != expected_checkpoints
        || value.committed_at < latest_issue_time
        || !authorities.validity_contains(value.committed_at)
        || value.candidate_root_key_id != *principal.root_key_id()
    {
        Err(contextual(
            "principal bootstrap receipt",
            "exact admitted bootstrap member set and valid logical commit time",
        ))
    } else {
        Ok(())
    }
}

fn bootstrap_id_from_checkpoint_scope(
    authority: &CheckpointSigningAuthorityV1,
) -> Result<PrincipalBootstrapIdV1, InvalidResidentRecordV1> {
    match authority.scope() {
        CheckpointSigningScopeV1::PrincipalTrustStore { bootstrap_id } => Ok(bootstrap_id.clone()),
        CheckpointSigningScopeV1::Deployment { .. } => Err(contextual(
            "principal bootstrap receipt",
            "bootstrap-scoped checkpoint signing authority",
        )),
    }
}
