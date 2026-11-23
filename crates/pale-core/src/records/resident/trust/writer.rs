//! Principal resident-writer fencing contracts and transition laws.
//!
//! For writer generation `g`, clean retirement preserves the generation,
//! while either later clean acquisition or authenticated unclean takeover
//! advances exactly once:

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/principal-writer-generation.inline.mathml"
))]
use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for principal resident-writer fence wires.
pub const PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA_ID: &str = "principal-resident-writer-fence.v1";
/// Wire schema carried by every principal resident-writer fence.
pub const PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA: &str =
    "pale/principal-resident-writer-fence.v1";
/// Signature domain for [`PrincipalResidentWriterFenceV1`].
pub const PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_PRINCIPAL_RESIDENT_WRITER_FENCE_V1\0";

/// Public schema root for clean principal resident-writer terminal receipts.
pub const PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA_ID: &str =
    "principal-resident-writer-terminal-receipt.v1";
/// Wire schema carried by every clean principal resident-writer terminal receipt.
pub const PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA: &str =
    "pale/principal-resident-writer-terminal-receipt.v1";
/// Signature domain for [`PrincipalResidentWriterTerminalReceiptV1`].
pub const PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1\0";

/// Public schema root for authenticated principal resident-writer takeover receipts.
pub const PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA_ID: &str =
    "principal-resident-writer-takeover-receipt.v1";
/// Wire schema carried by every authenticated resident-writer takeover receipt.
pub const PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA: &str =
    "pale/principal-resident-writer-takeover-receipt.v1";
/// Signature domain for [`PrincipalResidentWriterTakeoverReceiptV1`].
pub const PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SIGNATURE_DOMAIN: &[u8] =
    b"PALE_PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1\0";

/// Principal resident-writer lifecycle state.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalResidentWriterFenceStateV1 {
    /// The named process lifetime may own mutation after external lock and CAS admission.
    Active,
    /// A clean terminal receipt closed the named process lifetime.
    Retired,
}

/// Owner-resolved current principal resident-writer fence head.
///
/// Each variant carries the only head state accepted by its transition. The
/// admitting owner resolves this witness immediately before admission and
/// later performs the atomic compare-and-swap; this pure algebra does neither.
pub enum PrincipalResidentWriterFenceHeadAdmissionStateV1<'a> {
    /// No writer-fence head exists for this principal.
    Unallocated,
    /// The exact active fence is the current head.
    Active {
        /// Immutable reference resolving `writer_fence`.
        writer_fence_ref: &'a ResidentRecordRefV1,
        /// Exact current active fence record.
        writer_fence: &'a PrincipalResidentWriterFenceV1,
    },
    /// The exact cleanly retired fence is the current head.
    Retired {
        /// Immutable reference resolving `writer_fence`.
        writer_fence_ref: &'a ResidentRecordRefV1,
        /// Exact current retired fence record.
        writer_fence: &'a PrincipalResidentWriterFenceV1,
    },
}

impl<'a> PrincipalResidentWriterFenceHeadAdmissionStateV1<'a> {
    /// Bind an exact verified active writer fence as the current owner head.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when `writer_fence` fails its
    /// self-digest check or is not in the active state.
    pub fn active(
        writer_fence_ref: &'a ResidentRecordRefV1,
        writer_fence: &'a PrincipalResidentWriterFenceV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let state = Self::Active {
            writer_fence_ref,
            writer_fence,
        };
        state.validate_variant()?;
        Ok(state)
    }

    /// Bind an exact verified retired writer fence as the current owner head.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when `writer_fence` fails its
    /// self-digest check or is not in the retired state.
    pub fn retired(
        writer_fence_ref: &'a ResidentRecordRefV1,
        writer_fence: &'a PrincipalResidentWriterFenceV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let state = Self::Retired {
            writer_fence_ref,
            writer_fence,
        };
        state.validate_variant()?;
        Ok(state)
    }

    fn require_unallocated(&self) -> Result<(), InvalidResidentRecordV1> {
        self.validate_variant()?;
        if matches!(self, Self::Unallocated) {
            Ok(())
        } else {
            Err(contextual(
                "principal resident writer fence head",
                "unallocated current head",
            ))
        }
    }

    fn require_active(
        &self,
    ) -> Result<
        (&'a ResidentRecordRefV1, &'a PrincipalResidentWriterFenceV1),
        InvalidResidentRecordV1,
    > {
        self.validate_variant()?;
        match self {
            Self::Active {
                writer_fence_ref,
                writer_fence,
            } => Ok((writer_fence_ref, writer_fence)),
            Self::Unallocated | Self::Retired { .. } => Err(contextual(
                "principal resident writer fence head",
                "active current head",
            )),
        }
    }

    fn require_retired(
        &self,
    ) -> Result<
        (&'a ResidentRecordRefV1, &'a PrincipalResidentWriterFenceV1),
        InvalidResidentRecordV1,
    > {
        self.validate_variant()?;
        match self {
            Self::Retired {
                writer_fence_ref,
                writer_fence,
            } => Ok((writer_fence_ref, writer_fence)),
            Self::Unallocated | Self::Active { .. } => Err(contextual(
                "principal resident writer fence head",
                "retired current head",
            )),
        }
    }

    fn validate_variant(&self) -> Result<(), InvalidResidentRecordV1> {
        let (expected_state, writer_fence) = match self {
            Self::Unallocated => return Ok(()),
            Self::Active { writer_fence, .. } => {
                (PrincipalResidentWriterFenceStateV1::Active, writer_fence)
            }
            Self::Retired { writer_fence, .. } => {
                (PrincipalResidentWriterFenceStateV1::Retired, writer_fence)
            }
        };
        writer_fence.verify()?;
        if writer_fence.state() == &expected_state {
            Ok(())
        } else {
            Err(contextual(
                "principal resident writer fence head",
                "variant matches verified record state",
            ))
        }
    }
}

/// Exact admitted clean-terminal receipt and its immutable reference.
#[derive(Clone, Copy)]
pub struct PrincipalResidentWriterTerminalAdmissionContextV1<'a> {
    /// Immutable reference resolving [`Self::terminal_receipt`].
    pub terminal_receipt_ref: &'a ResidentRecordRefV1,
    /// Exact admitted terminal receipt.
    pub terminal_receipt: &'a PrincipalResidentWriterTerminalReceiptV1,
}

/// Exact admitted takeover receipt and its immutable reference.
#[derive(Clone, Copy)]
pub struct PrincipalResidentWriterTakeoverAdmissionContextV1<'a> {
    /// Immutable reference resolving [`Self::takeover_receipt`].
    pub takeover_receipt_ref: &'a ResidentRecordRefV1,
    /// Exact admitted takeover receipt.
    pub takeover_receipt: &'a PrincipalResidentWriterTakeoverReceiptV1,
}

resident_signed_record!(
    /// Signed lifetime fence for the sole principal resident writer.
    PrincipalResidentWriterFenceV1,
    PrincipalResidentWriterFenceWireV1,
    PrincipalResidentWriterFencePreimageV1,
    canonical_principal_resident_writer_fence_sha256: PrincipalResidentWriterFenceDigest,
    PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
    "principal resident writer fence",
    {
        /// Stable identity shared by the complete writer-fence lineage.
        writer_fence_id: PrincipalResidentWriterFenceIdV1,
        /// Principal whose trust and deployment stores this fence protects.
        principal_id: ResidentPrincipalIdV1,
        /// Positive generation; retirement preserves it and acquisition advances it.
        writer_generation: PrincipalResidentWriterGenerationV1,
        /// Exact prior fence, absent only for the initial active generation.
        previous_writer_fence_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_writer_fence_ref`].
        previous_writer_fence_sha256: Option<PrincipalResidentWriterFenceDigest>,
        /// Clean terminal receipt closing the predecessor when applicable.
        prior_terminal_receipt_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::prior_terminal_receipt_ref`].
        prior_terminal_receipt_sha256: Option<PrincipalResidentWriterTerminalReceiptDigest>,
        /// Authenticated takeover closing an unclean predecessor when applicable.
        takeover_receipt_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::takeover_receipt_ref`].
        takeover_receipt_sha256: Option<PrincipalResidentWriterTakeoverReceiptDigest>,
        /// Primary resident node bound to this writer lifetime.
        primary_node_id: ResidentNodeIdV1,
        /// Unique launcher-supplied process instance.
        process_instance_id: ResidentProcessInstanceIdV1,
        /// Machine-boot incarnation observed for this process instance.
        boot_incarnation_id: ResidentBootIncarnationIdV1,
        /// Logical principal trust store opened by this writer.
        trust_store_id: ControlStoreIdV1,
        /// Exact trust-store incarnation opened by this writer.
        trust_store_incarnation_id: ControlIncarnationIdV1,
        /// Authenticated root of deployment-store heads opened by this lifetime.
        opened_deployment_store_heads_root_sha256: ArtifactDigest,
        /// Session floor enforced by this writer lifetime.
        principal_session_floor_epoch: PrincipalSessionFloorEpochV1,
        /// Whether this lifetime is active or cleanly retired.
        state: PrincipalResidentWriterFenceStateV1,
        /// Unix time at which this writer generation was acquired.
        acquired_at: UnixTimeV1,
        /// Clean retirement time, present only on the retired successor record.
        retired_at: Option<UnixTimeV1>,
        /// Principal under whose active accepted key this fence was issued.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Active accepted principal-key identity used for the signature.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive accepted principal-key epoch used for the signature.
        issuer_key_epoch: PrincipalKeyEpochV1
    },
    validate_writer_fence_shape
);

resident_signed_record!(
    /// Signed evidence that one active resident writer shut down cleanly.
    PrincipalResidentWriterTerminalReceiptV1,
    PrincipalResidentWriterTerminalReceiptWireV1,
    PrincipalResidentWriterTerminalReceiptPreimageV1,
    canonical_principal_resident_writer_terminal_receipt_sha256:
        PrincipalResidentWriterTerminalReceiptDigest,
    PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA,
    "principal resident writer terminal receipt",
    {
        /// Stable identity of this terminal receipt.
        terminal_receipt_id: PrincipalResidentWriterTerminalReceiptIdV1,
        /// Principal whose resident writer stopped.
        principal_id: ResidentPrincipalIdV1,
        /// Exact active writer fence closed by this receipt.
        writer_fence_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the active writer fence.
        writer_fence_sha256: PrincipalResidentWriterFenceDigest,
        /// Writer generation closed by this receipt.
        writer_generation: PrincipalResidentWriterGenerationV1,
        /// Final accepted principal trust-checkpoint reference.
        final_trust_checkpoint_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the final trust checkpoint.
        final_trust_checkpoint_sha256: PrincipalTrustCheckpointDigest,
        /// Authenticated root of final opened-store heads.
        final_opened_store_heads_root_sha256: ArtifactDigest,
        /// Authenticated root of pending reconciliation custody at shutdown.
        final_pending_custody_root_sha256: ArtifactDigest,
        /// Unix time after work quiesced and final heads/custody were fixed.
        ///
        /// The terminal receipt and retired fence commit at this frontier;
        /// protected stores close afterward while the OS lock remains held.
        stopped_at: UnixTimeV1,
        /// Principal under whose active accepted key this receipt was issued.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Active accepted principal-key identity used for the signature.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive accepted principal-key epoch used for the signature.
        issuer_key_epoch: PrincipalKeyEpochV1
    },
    |value: &PrincipalResidentWriterTerminalReceiptPreimageV1| {
        validate_issuer_shape(
            "principal resident writer terminal receipt",
            &value.principal_id,
            &value.issuer_principal_id,
        )
    }
);

resident_signed_record!(
    /// Signed authorization and evidence for replacing an unclean resident writer.
    PrincipalResidentWriterTakeoverReceiptV1,
    PrincipalResidentWriterTakeoverReceiptWireV1,
    PrincipalResidentWriterTakeoverReceiptPreimageV1,
    canonical_principal_resident_writer_takeover_receipt_sha256:
        PrincipalResidentWriterTakeoverReceiptDigest,
    PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA,
    "principal resident writer takeover receipt",
    {
        /// Stable identity of this authenticated takeover receipt.
        takeover_receipt_id: PrincipalResidentWriterTakeoverReceiptIdV1,
        /// Principal whose stale writer is being replaced.
        principal_id: ResidentPrincipalIdV1,
        /// Exact still-active predecessor fence.
        predecessor_writer_fence_ref: ResidentRecordRefV1,
        /// Canonical unsigned-preimage digest of the predecessor fence.
        predecessor_writer_fence_sha256: PrincipalResidentWriterFenceDigest,
        /// Writer generation being retired by takeover.
        predecessor_writer_generation: PrincipalResidentWriterGenerationV1,
        /// Authenticated root of predecessor non-liveness evidence.
        predecessor_non_liveness_evidence_root_sha256: ArtifactDigest,
        /// Accepted principal trust-checkpoint reference used for takeover.
        accepted_trust_checkpoint_ref: ResidentRecordRefV1,
        /// Digest of the accepted principal trust-checkpoint bytes.
        accepted_trust_checkpoint_sha256: PrincipalTrustCheckpointDigest,
        /// Authenticated root of the exact verified store heads.
        verified_store_heads_root_sha256: ArtifactDigest,
        /// Session floor enforced by the predecessor writer.
        prior_session_floor_epoch: PrincipalSessionFloorEpochV1,
        /// Exact successor floor, one greater than the predecessor floor.
        new_session_floor_epoch: PrincipalSessionFloorEpochV1,
        /// Root of predecessor routes and leases invalidated by takeover.
        invalidated_route_lease_heads_root_sha256: ArtifactDigest,
        /// Root of unresolved custody quarantined by takeover.
        quarantined_custody_root_sha256: ArtifactDigest,
        /// Primary node selected for the successor writer.
        successor_primary_node_id: ResidentNodeIdV1,
        /// Process instance selected for the successor writer.
        successor_process_instance_id: ResidentProcessInstanceIdV1,
        /// Boot incarnation selected for the successor writer.
        successor_boot_incarnation_id: ResidentBootIncarnationIdV1,
        /// One-use authenticated takeover nonce.
        takeover_nonce: PrincipalResidentWriterTakeoverNonceV1,
        /// Unix time at which the takeover was authorized.
        authorized_at: UnixTimeV1,
        /// Principal under whose active accepted key takeover was authorized.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Active accepted principal-key identity used for the signature.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive accepted principal-key epoch used for the signature.
        issuer_key_epoch: PrincipalKeyEpochV1
    },
    |value: &PrincipalResidentWriterTakeoverReceiptPreimageV1| {
        validate_issuer_shape(
            "principal resident writer takeover receipt",
            &value.principal_id,
            &value.issuer_principal_id,
        )?;
        if value.prior_session_floor_epoch.checked_successor()
            != Some(value.new_session_floor_epoch)
        {
            Err(InvalidResidentRecordV1::InvalidCoordinate {
                coordinate: "takeover session floor successor",
            })
        } else {
            Ok(())
        }
    }
);

impl PrincipalResidentWriterFenceV1 {
    /// Admit the generation-one active writer fence.
    ///
    /// `head_state` must be
    /// [`PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated`].
    /// The signature callback receives the exact domain, canonical preimage,
    /// digest, and signature; this function does not acquire the writer lock.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the current head is occupied,
    /// or when signed coordinates, genesis state, digest, or signature
    /// admission fails.
    pub fn admit_initial_active_wire<F>(
        wire: PrincipalResidentWriterFenceWireV1,
        head_state: &PrincipalResidentWriterFenceHeadAdmissionStateV1<'_>,
        expected_schema_sha256: &SchemaDigest,
        signer: &PrincipalKeyAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &PrincipalResidentWriterFencePreimageV1,
            &PrincipalResidentWriterFenceDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        head_state.require_unallocated()?;
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_writer_coordinates(preimage, expected_schema_sha256, signer)?;
            if preimage.writer_generation.get() != 1
                || preimage.state != PrincipalResidentWriterFenceStateV1::Active
                || preimage.previous_writer_fence_ref.is_some()
                || preimage.prior_terminal_receipt_ref.is_some()
                || preimage.takeover_receipt_ref.is_some()
            {
                return Err(contextual(
                    "initial writer fence",
                    "generation-one active genesis",
                ));
            }
            verify_signature(
                PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }

    /// Admit the same-generation retired successor after clean shutdown.
    ///
    /// All writer/process/store/session coordinates, including the generation,
    /// are preserved as required by the rendered retirement law. The exact terminal receipt supplies the
    /// clean stop time and predecessor closure evidence. `head_state` must be
    /// the exact current [`PrincipalResidentWriterFenceHeadAdmissionStateV1::Active`]
    /// fence that the terminal receipt closes.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] unless the current head is active
    /// and the predecessor, terminal receipt, preserved lifetime, and signed
    /// coordinates agree exactly.
    pub fn admit_clean_retired_wire<F>(
        wire: PrincipalResidentWriterFenceWireV1,
        head_state: &PrincipalResidentWriterFenceHeadAdmissionStateV1<'_>,
        terminal: PrincipalResidentWriterTerminalAdmissionContextV1<'_>,
        expected_schema_sha256: &SchemaDigest,
        signer: &PrincipalKeyAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &PrincipalResidentWriterFencePreimageV1,
            &PrincipalResidentWriterFenceDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (active_ref, active) = head_state.require_active()?;
        let PrincipalResidentWriterTerminalAdmissionContextV1 {
            terminal_receipt_ref: terminal_ref,
            terminal_receipt: terminal,
        } = terminal;
        terminal.verify()?;
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_writer_coordinates(preimage, expected_schema_sha256, signer)?;
            if active.state() != &PrincipalResidentWriterFenceStateV1::Active
                || terminal.writer_fence_ref() != active_ref
                || terminal.writer_fence_sha256()
                    != active.canonical_principal_resident_writer_fence_sha256()
                || terminal.principal_id() != active.principal_id()
                || terminal.writer_generation() != active.writer_generation()
                || preimage.state != PrincipalResidentWriterFenceStateV1::Retired
                || preimage.previous_writer_fence_ref.as_ref() != Some(active_ref)
                || preimage.previous_writer_fence_sha256.as_ref()
                    != Some(active.canonical_principal_resident_writer_fence_sha256())
                || preimage.prior_terminal_receipt_ref.as_ref() != Some(terminal_ref)
                || preimage.prior_terminal_receipt_sha256.as_ref()
                    != Some(terminal.canonical_principal_resident_writer_terminal_receipt_sha256())
                || preimage.retired_at.as_ref() != Some(terminal.stopped_at())
                || !same_writer_lifetime(preimage, active)
            {
                return Err(contextual(
                    "clean writer retirement",
                    "exact active fence and terminal receipt closure",
                ));
            }
            verify_signature(
                PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }

    /// Admit an active successor after a cleanly retired generation.
    ///
    /// The successor increments generation once, points to the retired fence,
    /// repeats its terminal binding, and opens the terminal receipt's final
    /// store-head root. The retired fence alone cannot prove that final root.
    /// The launcher must supply a process-instance identity distinct from the
    /// stopped predecessor; primary-node and boot-incarnation identities may
    /// remain equal.
    /// [`PrincipalResidentWriterFenceV1::trust_store_incarnation_id`] is
    /// re-established by external store and recovery admission rather than
    /// inherited: lawful catastrophic recovery while stopped may create a new
    /// incarnation. This pure function neither proves nor performs recovery.
    /// `head_state` must be the exact current
    /// [`PrincipalResidentWriterFenceHeadAdmissionStateV1::Retired`] fence.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the current head is not the
    /// exact retired predecessor, or retirement, terminal evidence,
    /// next-generation coordinates, chronology, or signature admission fails.
    pub fn admit_clean_successor_active_wire<F>(
        wire: PrincipalResidentWriterFenceWireV1,
        head_state: &PrincipalResidentWriterFenceHeadAdmissionStateV1<'_>,
        terminal: PrincipalResidentWriterTerminalAdmissionContextV1<'_>,
        expected_schema_sha256: &SchemaDigest,
        signer: &PrincipalKeyAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &PrincipalResidentWriterFencePreimageV1,
            &PrincipalResidentWriterFenceDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (retired_ref, retired) = head_state.require_retired()?;
        let PrincipalResidentWriterTerminalAdmissionContextV1 {
            terminal_receipt_ref: terminal_ref,
            terminal_receipt: terminal,
        } = terminal;
        terminal.verify()?;
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_writer_coordinates(preimage, expected_schema_sha256, signer)?;
            let next = next_writer_generation(retired.writer_generation());
            if retired.state() != &PrincipalResidentWriterFenceStateV1::Retired
                || preimage.state != PrincipalResidentWriterFenceStateV1::Active
                || next != Some(preimage.writer_generation)
                || preimage.writer_fence_id != *retired.writer_fence_id()
                || preimage.principal_id != *retired.principal_id()
                || preimage.previous_writer_fence_ref.as_ref() != Some(retired_ref)
                || preimage.previous_writer_fence_sha256.as_ref()
                    != Some(retired.canonical_principal_resident_writer_fence_sha256())
                || retired.prior_terminal_receipt_ref().as_ref() != Some(terminal_ref)
                || retired.prior_terminal_receipt_sha256().as_ref()
                    != Some(terminal.canonical_principal_resident_writer_terminal_receipt_sha256())
                || retired.previous_writer_fence_ref().as_ref() != Some(terminal.writer_fence_ref())
                || retired.previous_writer_fence_sha256().as_ref()
                    != Some(terminal.writer_fence_sha256())
                || retired.retired_at().as_ref() != Some(terminal.stopped_at())
                || terminal.writer_generation() != retired.writer_generation()
                || terminal.principal_id() != retired.principal_id()
                || preimage.prior_terminal_receipt_ref.as_ref() != Some(terminal_ref)
                || preimage.prior_terminal_receipt_sha256.as_ref()
                    != Some(terminal.canonical_principal_resident_writer_terminal_receipt_sha256())
                || preimage.takeover_receipt_ref.is_some()
                || preimage.trust_store_id != *retired.trust_store_id()
                || preimage.process_instance_id == *retired.process_instance_id()
                || preimage.opened_deployment_store_heads_root_sha256
                    != *terminal.final_opened_store_heads_root_sha256()
                || preimage.principal_session_floor_epoch.get()
                    < retired.principal_session_floor_epoch().get()
                || retired
                    .retired_at()
                    .is_none_or(|stopped| preimage.acquired_at < stopped)
            {
                return Err(contextual(
                    "clean writer successor",
                    "retired predecessor terminal binding and exact next generation",
                ));
            }
            verify_signature(
                PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }

    /// Admit an active successor directly from an unclean active predecessor.
    ///
    /// The exact takeover receipt semantically retires the predecessor and
    /// binds the successor process, store-root, and session-floor coordinates;
    /// no synthetic retired fence exists on this path. `head_state` must be the
    /// exact current [`PrincipalResidentWriterFenceHeadAdmissionStateV1::Active`]
    /// predecessor.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] unless the current head is the exact
    /// active predecessor and the takeover receipt, successor coordinates, and
    /// signature agree exactly.
    pub fn admit_takeover_successor_active_wire<F>(
        wire: PrincipalResidentWriterFenceWireV1,
        head_state: &PrincipalResidentWriterFenceHeadAdmissionStateV1<'_>,
        takeover: PrincipalResidentWriterTakeoverAdmissionContextV1<'_>,
        expected_schema_sha256: &SchemaDigest,
        signer: &PrincipalKeyAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &PrincipalResidentWriterFencePreimageV1,
            &PrincipalResidentWriterFenceDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (active_ref, active) = head_state.require_active()?;
        let PrincipalResidentWriterTakeoverAdmissionContextV1 {
            takeover_receipt_ref: takeover_ref,
            takeover_receipt: takeover,
        } = takeover;
        takeover.verify()?;
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_writer_coordinates(preimage, expected_schema_sha256, signer)?;
            let next = next_writer_generation(active.writer_generation());
            if active.state() != &PrincipalResidentWriterFenceStateV1::Active
                || takeover.predecessor_writer_fence_ref() != active_ref
                || takeover.predecessor_writer_fence_sha256()
                    != active.canonical_principal_resident_writer_fence_sha256()
                || takeover.principal_id() != active.principal_id()
                || takeover.predecessor_writer_generation() != active.writer_generation()
                || takeover.prior_session_floor_epoch() != active.principal_session_floor_epoch()
                || takeover.authorized_at() < active.acquired_at()
                || preimage.state != PrincipalResidentWriterFenceStateV1::Active
                || next != Some(preimage.writer_generation)
                || preimage.writer_fence_id != *active.writer_fence_id()
                || preimage.principal_id != *active.principal_id()
                || preimage.previous_writer_fence_ref.as_ref() != Some(active_ref)
                || preimage.previous_writer_fence_sha256.as_ref()
                    != Some(active.canonical_principal_resident_writer_fence_sha256())
                || preimage.prior_terminal_receipt_ref.is_some()
                || preimage.takeover_receipt_ref.as_ref() != Some(takeover_ref)
                || preimage.takeover_receipt_sha256.as_ref()
                    != Some(takeover.canonical_principal_resident_writer_takeover_receipt_sha256())
                || preimage.primary_node_id != *takeover.successor_primary_node_id()
                || preimage.process_instance_id != *takeover.successor_process_instance_id()
                || preimage.boot_incarnation_id != *takeover.successor_boot_incarnation_id()
                || preimage.trust_store_id != *active.trust_store_id()
                || preimage.trust_store_incarnation_id != *active.trust_store_incarnation_id()
                || preimage.opened_deployment_store_heads_root_sha256
                    != *takeover.verified_store_heads_root_sha256()
                || preimage.principal_session_floor_epoch != *takeover.new_session_floor_epoch()
                || preimage.acquired_at < *takeover.authorized_at()
            {
                return Err(contextual(
                    "takeover writer successor",
                    "exact active predecessor takeover and successor coordinates",
                ));
            }
            verify_signature(
                PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

impl PrincipalResidentWriterTerminalReceiptV1 {
    /// Admit a clean terminal receipt against the exact active writer fence.
    ///
    /// `head_state` must identify that fence as the owner-resolved current
    /// [`PrincipalResidentWriterFenceHeadAdmissionStateV1::Active`] head.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the current head is not active,
    /// or fence identity, generation, chronology, signed coordinates, or
    /// signature admission fails.
    pub fn admit_contextual_wire<F>(
        wire: PrincipalResidentWriterTerminalReceiptWireV1,
        head_state: &PrincipalResidentWriterFenceHeadAdmissionStateV1<'_>,
        expected_schema_sha256: &SchemaDigest,
        signer: &PrincipalKeyAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &PrincipalResidentWriterTerminalReceiptPreimageV1,
            &PrincipalResidentWriterTerminalReceiptDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (writer_fence_ref, writer_fence) = head_state.require_active()?;
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_principal_signed_coordinates(
                "principal resident writer terminal receipt",
                expected_schema_sha256,
                PrincipalSignedCoordinatesV1 {
                    schema_sha256: &preimage.schema_sha256,
                    subject_principal_id: &preimage.principal_id,
                    issuer_principal_id: &preimage.issuer_principal_id,
                    issuer_key_id: &preimage.issuer_key_id,
                    issuer_key_epoch: preimage.issuer_key_epoch,
                },
                signer,
            )?;
            if writer_fence.state() != &PrincipalResidentWriterFenceStateV1::Active
                || preimage.writer_fence_ref != *writer_fence_ref
                || preimage.writer_fence_sha256
                    != *writer_fence.canonical_principal_resident_writer_fence_sha256()
                || preimage.principal_id != *writer_fence.principal_id()
                || preimage.writer_generation != *writer_fence.writer_generation()
                || preimage.stopped_at < *writer_fence.acquired_at()
            {
                return Err(contextual(
                    "writer terminal receipt",
                    "exact active writer fence and monotone stop time",
                ));
            }
            verify_signature(
                PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

impl PrincipalResidentWriterTakeoverReceiptV1 {
    /// Admit authenticated evidence for replacing one active predecessor.
    ///
    /// This validates declared non-liveness evidence binding but does not
    /// decide process liveness itself. `head_state` must identify the exact
    /// owner-resolved current
    /// [`PrincipalResidentWriterFenceHeadAdmissionStateV1::Active`] predecessor.
    /// `check_nonce_admissible` is a validation-only owner witness: before
    /// commit, owner policy may accept an unallocated nonce or an exact
    /// same-digest retry and must reject changed reuse. After commit, owner
    /// idempotency returns the stored exact receipt before this candidate path,
    /// because the current head has already advanced. This function neither
    /// consumes nor persists the nonce; the owner transaction must co-commit
    /// that mapping with its hard frontier.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the current head is not active,
    /// or predecessor, generation, session-floor, chronology, signed
    /// coordinates, signature, or nonce admissibility fails.
    pub fn admit_contextual_wire<N, S>(
        wire: PrincipalResidentWriterTakeoverReceiptWireV1,
        head_state: &PrincipalResidentWriterFenceHeadAdmissionStateV1<'_>,
        expected_schema_sha256: &SchemaDigest,
        signer: &PrincipalKeyAdmissionContextV1<'_>,
        check_nonce_admissible: N,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        N: FnOnce(
            &ResidentPrincipalIdV1,
            &PrincipalResidentWriterTakeoverNonceV1,
            &PrincipalResidentWriterTakeoverReceiptDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        S: FnOnce(
            &[u8],
            &PrincipalResidentWriterTakeoverReceiptPreimageV1,
            &PrincipalResidentWriterTakeoverReceiptDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (predecessor_ref, predecessor) = head_state.require_active()?;
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_principal_signed_coordinates(
                "principal resident writer takeover receipt",
                expected_schema_sha256,
                PrincipalSignedCoordinatesV1 {
                    schema_sha256: &preimage.schema_sha256,
                    subject_principal_id: &preimage.principal_id,
                    issuer_principal_id: &preimage.issuer_principal_id,
                    issuer_key_id: &preimage.issuer_key_id,
                    issuer_key_epoch: preimage.issuer_key_epoch,
                },
                signer,
            )?;
            if predecessor.state() != &PrincipalResidentWriterFenceStateV1::Active
                || preimage.predecessor_writer_fence_ref != *predecessor_ref
                || preimage.predecessor_writer_fence_sha256
                    != *predecessor.canonical_principal_resident_writer_fence_sha256()
                || preimage.principal_id != *predecessor.principal_id()
                || preimage.predecessor_writer_generation != *predecessor.writer_generation()
                || preimage.prior_session_floor_epoch
                    != *predecessor.principal_session_floor_epoch()
                || preimage.authorized_at < *predecessor.acquired_at()
                || preimage.successor_process_instance_id == *predecessor.process_instance_id()
            {
                return Err(contextual(
                    "writer takeover receipt",
                    "exact active predecessor and successor takeover coordinates",
                ));
            }
            verify_signature(
                PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )?;
            check_nonce_admissible(&preimage.principal_id, &preimage.takeover_nonce, digest)
        })
    }
}

fn validate_writer_coordinates(
    value: &PrincipalResidentWriterFencePreimageV1,
    expected_schema_sha256: &SchemaDigest,
    signer: &PrincipalKeyAdmissionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    validate_principal_signed_coordinates(
        "principal resident writer fence",
        expected_schema_sha256,
        PrincipalSignedCoordinatesV1 {
            schema_sha256: &value.schema_sha256,
            subject_principal_id: &value.principal_id,
            issuer_principal_id: &value.issuer_principal_id,
            issuer_key_id: &value.issuer_key_id,
            issuer_key_epoch: value.issuer_key_epoch,
        },
        signer,
    )
}

fn validate_writer_fence_shape(
    value: &PrincipalResidentWriterFencePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    validate_issuer_shape(
        "principal resident writer fence",
        &value.principal_id,
        &value.issuer_principal_id,
    )?;
    let predecessor_paired =
        value.previous_writer_fence_ref.is_some() == value.previous_writer_fence_sha256.is_some();
    let terminal_paired =
        value.prior_terminal_receipt_ref.is_some() == value.prior_terminal_receipt_sha256.is_some();
    let takeover_paired =
        value.takeover_receipt_ref.is_some() == value.takeover_receipt_sha256.is_some();
    let valid_state = match value.state {
        PrincipalResidentWriterFenceStateV1::Active => {
            value.retired_at.is_none()
                && match value.previous_writer_fence_ref {
                    None => {
                        value.prior_terminal_receipt_ref.is_none()
                            && value.takeover_receipt_ref.is_none()
                    }
                    Some(_) => {
                        value.prior_terminal_receipt_ref.is_some()
                            ^ value.takeover_receipt_ref.is_some()
                    }
                }
        }
        PrincipalResidentWriterFenceStateV1::Retired => {
            value.previous_writer_fence_ref.is_some()
                && value.prior_terminal_receipt_ref.is_some()
                && value.takeover_receipt_ref.is_none()
                && value
                    .retired_at
                    .is_some_and(|retired_at| retired_at >= value.acquired_at)
        }
    };
    if predecessor_paired && terminal_paired && takeover_paired && valid_state {
        Ok(())
    } else {
        Err(contextual(
            "principal resident writer fence",
            "closed active or clean-retired field state",
        ))
    }
}

fn same_writer_lifetime(
    value: &PrincipalResidentWriterFencePreimageV1,
    predecessor: &PrincipalResidentWriterFenceV1,
) -> bool {
    value.writer_fence_id == *predecessor.writer_fence_id()
        && value.principal_id == *predecessor.principal_id()
        && value.writer_generation == *predecessor.writer_generation()
        && value.primary_node_id == *predecessor.primary_node_id()
        && value.process_instance_id == *predecessor.process_instance_id()
        && value.boot_incarnation_id == *predecessor.boot_incarnation_id()
        && value.trust_store_id == *predecessor.trust_store_id()
        && value.trust_store_incarnation_id == *predecessor.trust_store_incarnation_id()
        && value.opened_deployment_store_heads_root_sha256
            == *predecessor.opened_deployment_store_heads_root_sha256()
        && value.principal_session_floor_epoch == *predecessor.principal_session_floor_epoch()
        && value.acquired_at == *predecessor.acquired_at()
}

/// Compute the only lawful successor generation without permitting wraparound.
pub(super) fn next_writer_generation(
    current: &PrincipalResidentWriterGenerationV1,
) -> Option<PrincipalResidentWriterGenerationV1> {
    current.checked_successor()
}
