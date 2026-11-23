//! Exact ambiguity-case, exclusive-custody, and result contracts.
//!
//! In the rendered lineage law, `c_i` is a custody generation and `r_j` is a
//! result revision. Each direct successor advances its own coordinate exactly
//! once. A later result may bind a transferred custody only through the shown
//! gap-free custody-successor path from its predecessor result.

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/reconciliation-lineage-successors.display.mathml"
))]
use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for reconciliation-case wires.
pub const RECONCILIATION_CASE_V1_SCHEMA_ID: &str = "reconciliation-case.v1";
/// Wire schema carried by every reconciliation case.
pub const RECONCILIATION_CASE_V1_SCHEMA: &str = "pale/reconciliation-case.v1";
/// Public schema root for reconciliation-custody wires.
pub const RECONCILIATION_CUSTODY_V1_SCHEMA_ID: &str = "reconciliation-custody.v1";
/// Wire schema carried by every reconciliation custody record.
pub const RECONCILIATION_CUSTODY_V1_SCHEMA: &str = "pale/reconciliation-custody.v1";
/// Public schema root for unsigned reconciliation-request wires.
pub const RECONCILIATION_REQUEST_V1_SCHEMA_ID: &str = "reconciliation-request.v1";
/// Schema selected by a common request carrying a reconciliation body.
pub const RECONCILIATION_REQUEST_V1_SCHEMA: &str = "pale/reconciliation-request.v1";
/// Public schema root for reconciliation-result wires.
pub const RECONCILIATION_RESULT_V1_SCHEMA_ID: &str = "reconciliation-result.v1";
/// Wire schema carried by every reconciliation result.
pub const RECONCILIATION_RESULT_V1_SCHEMA: &str = "pale/reconciliation-result.v1";

/// Why an immutable reconciliation case exists in its lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationCaseOriginV1 {
    /// The first case opened for the ambiguous action.
    Initial,
    /// A new case opened from one terminal predecessor result.
    Reopened,
}

/// One typed opening-evidence member. Exactly one tagged variant is present.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReconciliationOpeningEvidenceV1 {
    /// An immutable effect receipt.
    EffectReceipt {
        /// Receipt reference.
        evidence_ref: ResidentRecordRefV1,
        /// Digest of the referenced bytes.
        evidence_sha256: ArtifactDigest,
        /// Exact receipt schema identity.
        evidence_schema_id: SchemaId,
        /// Exact receipt schema digest.
        evidence_schema_sha256: SchemaDigest,
    },
    /// An immutable native observation.
    Observation {
        /// Observation reference.
        evidence_ref: ResidentRecordRefV1,
        /// Digest of the referenced bytes.
        evidence_sha256: ArtifactDigest,
        /// Exact observation schema identity.
        evidence_schema_id: SchemaId,
        /// Exact observation schema digest.
        evidence_schema_sha256: SchemaDigest,
        /// Owner-defined observation classification.
        observation_kind: ObservationKindV1,
    },
    /// Reviewer- or policy-qualified evidence.
    ReviewerEvidence {
        /// Evidence reference.
        evidence_ref: ResidentRecordRefV1,
        /// Digest of the referenced bytes.
        evidence_sha256: ArtifactDigest,
        /// Exact evidence schema identity.
        evidence_schema_id: SchemaId,
        /// Exact evidence schema digest.
        evidence_schema_sha256: SchemaDigest,
        /// Immutable reviewer or policy reference.
        reviewer_or_policy_ref: ResidentRecordRefV1,
        /// Digest of the reviewer or policy bytes.
        reviewer_or_policy_sha256: ArtifactDigest,
    },
}

resident_signed_context_record!(
    /// Immutable initial or reopened ambiguity case; `case_revision` is exactly zero.
    ReconciliationCaseV1, ReconciliationCaseWireV1, ReconciliationCasePreimageV1,
    canonical_case_sha256: ReconciliationCaseDigest,
    RECONCILIATION_CASE_V1_SCHEMA, "reconciliation case",
    {
        /// Stable identity of this immutable ambiguity case.
        case_id: ReconciliationCaseIdV1,
        /// Logical action whose ambiguous outcome opened the case.
        action_id: ActionIdV1,
        /// Immutable effect admission governing the action.
        effect_admission_ref: ResidentRecordRefV1,
        /// Digest of the exact effect-admission bytes.
        effect_admission_sha256: EffectAdmissionDigest,
        /// Schema identity defining the protected conflict-resource key.
        conflict_resource_key_schema_id: SchemaId,
        /// Digest of the selected conflict-resource-key schema bytes.
        conflict_resource_key_schema_sha256: SchemaDigest,
        /// Canonical typed/scoped resource key held in reconciliation custody.
        conflict_resource_key: serde_json::Value,
        /// Preallocated custody identity paired acyclically with this completed case.
        reserved_custody_id: ReconciliationCustodyIdV1,
        /// Whether this case begins or reopens an ambiguity lineage.
        case_origin: ReconciliationCaseOriginV1,
        /// Canonically ordered evidence that justifies opening this case.
        opening_evidence: Vec<ReconciliationOpeningEvidenceV1>,
        /// Typed digest of the state against which observations reconcile.
        expected_state_digest: TypedDigestV1,
        /// Immutable reconciliation policy selected at effect admission.
        reconciliation_policy_ref: ResidentRecordRefV1,
        /// Digest of the reconciliation-policy bytes.
        reconciliation_policy_sha256: ArtifactDigest,
        /// Exact predecessor case for a reopened lineage.
        predecessor_case_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::predecessor_case_ref`].
        predecessor_case_sha256: Option<ReconciliationCaseDigest>,
        /// Terminal predecessor result that permits reopening.
        predecessor_terminal_result_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::predecessor_terminal_result_ref`].
        predecessor_terminal_result_sha256: Option<ReconciliationResultDigest>,
        /// Separately admitted authority permitting a reopened case.
        reopen_authority_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::reopen_authority_ref`].
        reopen_authority_sha256: Option<ArtifactDigest>,
        /// Immutable policy permitting this reopening.
        reopen_policy_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::reopen_policy_ref`].
        reopen_policy_sha256: Option<ArtifactDigest>,
        /// Bounded policy-owned reason for reopening.
        reopen_reason_code: Option<ResidentReasonV1>,
        /// Immutable opening revision, which is exactly zero in initial-v1.
        case_revision: ReconciliationCaseRevisionV1,
        /// Case-open time in Unix seconds.
        opened_at: UnixTimeV1,
        /// Exact deployment-runtime authority under which this record was issued.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Principal under whose authority the case was issued.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Key identity used for the case signature.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch checked for the case.
        issuer_key_epoch: SigningKeyEpochV1
    },
    wire {
        case_id: ReconciliationCaseIdV1,
        action_id: ActionIdV1,
        effect_admission_ref: ResidentRecordRefV1,
        effect_admission_sha256: EffectAdmissionDigest,
        conflict_resource_key_schema_id: SchemaId,
        conflict_resource_key_schema_sha256: SchemaDigest,
        conflict_resource_key: serde_json::Value,
        reserved_custody_id: ReconciliationCustodyIdV1,
        case_origin: ReconciliationCaseOriginV1,
        opening_evidence: Vec<ReconciliationOpeningEvidenceV1>,
        expected_state_digest: TypedDigestWireV1,
        reconciliation_policy_ref: ResidentRecordRefV1,
        reconciliation_policy_sha256: ArtifactDigest,
        predecessor_case_ref: Option<ResidentRecordRefV1>,
        predecessor_case_sha256: Option<ReconciliationCaseDigest>,
        predecessor_terminal_result_ref: Option<ResidentRecordRefV1>,
        predecessor_terminal_result_sha256: Option<ReconciliationResultDigest>,
        reopen_authority_ref: Option<ResidentRecordRefV1>,
        reopen_authority_sha256: Option<ArtifactDigest>,
        reopen_policy_ref: Option<ResidentRecordRefV1>,
        reopen_policy_sha256: Option<ArtifactDigest>,
        reopen_reason_code: Option<ResidentReasonV1>,
        case_revision: ReconciliationCaseRevisionV1,
        opened_at: UnixTimeV1,
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        deployment_runtime_authority_sha256: ArtifactDigest,
        issuer_principal_id: ResidentPrincipalIdV1,
        issuer_key_id: ResidentKeyIdV1,
        issuer_key_epoch: SigningKeyEpochV1
    },
    |value: &ReconciliationCasePreimageV1| {
        validate_typed_digest(&value.expected_state_digest)?;
        validate_bounded_json(
            "reconciliation conflict-resource key bytes",
            &value.conflict_resource_key,
        )?;
        bounded_len("reconciliation opening evidence", value.opening_evidence.len())?;
        if value.opening_evidence.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(InvalidResidentRecordV1::NonCanonical {
                relation: "reconciliation opening evidence",
            });
        }
        if value.opening_evidence.is_empty() || value.case_revision.get() != 0 {
            return Err(InvalidResidentRecordV1::SettlementState);
        }
        let predecessor = value.predecessor_case_ref.is_some()
            && value.predecessor_case_sha256.is_some()
            && value.predecessor_terminal_result_ref.is_some()
            && value.predecessor_terminal_result_sha256.is_some()
            && value.reopen_authority_ref.is_some()
            && value.reopen_authority_sha256.is_some()
            && value.reopen_policy_ref.is_some()
            && value.reopen_policy_sha256.is_some()
            && value.reopen_reason_code.is_some();
        match value.case_origin {
            ReconciliationCaseOriginV1::Initial if !predecessor
                && value.predecessor_case_ref.is_none()
                && value.predecessor_case_sha256.is_none()
                && value.predecessor_terminal_result_ref.is_none()
                && value.predecessor_terminal_result_sha256.is_none()
                && value.reopen_authority_ref.is_none()
                && value.reopen_authority_sha256.is_none()
                && value.reopen_policy_ref.is_none()
                && value.reopen_policy_sha256.is_none()
                && value.reopen_reason_code.is_none()
                && value.opening_evidence.iter().any(|item| matches!(
                    item, ReconciliationOpeningEvidenceV1::EffectReceipt { .. }
                )) => Ok(()),
            ReconciliationCaseOriginV1::Reopened if predecessor
                && value.opening_evidence.iter().any(|item| matches!(
                    item,
                    ReconciliationOpeningEvidenceV1::Observation { .. }
                        | ReconciliationOpeningEvidenceV1::ReviewerEvidence { .. }
                )) => Ok(()),
            _ => Err(InvalidResidentRecordV1::SettlementState),
        }
    }
);

/// Resolved lineage for initial reconciliation-case admission.
pub struct ReconciliationCaseInitialAdmissionContextV1<'a> {
    /// Reference resolving `receipt`.
    pub receipt_ref: &'a ResidentRecordRefV1,
    /// Exact ambiguity-producing effect receipt.
    pub receipt: &'a EffectReceiptV1,
    /// Reference resolving `dispatch`.
    pub dispatch_ref: &'a ResidentRecordRefV1,
    /// Exact dispatch that reserved the case and custody identities.
    pub dispatch: &'a EffectDispatchIntentV1,
    /// Reference resolving `admission`.
    pub admission_ref: &'a ResidentRecordRefV1,
    /// Exact effect admission.
    pub admission: &'a EffectAdmissionV1,
    /// Exact admitted action body.
    pub action: &'a ActionEnvelopeV1,
    /// Owner-resolved expected state fixed for reconciliation.
    pub expected_state_digest: &'a TypedDigestV1,
    /// Digest of the exact reconciliation-policy bytes.
    pub reconciliation_policy_sha256: &'a ArtifactDigest,
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
}

/// Resolved predecessor lineage for a reopened reconciliation case.
pub struct ReconciliationCaseReopenAdmissionContextV1<'a> {
    /// Reference resolving `predecessor_case`.
    pub predecessor_case_ref: &'a ResidentRecordRefV1,
    /// Exact predecessor case.
    pub predecessor_case: &'a ReconciliationCaseV1,
    /// Reference resolving `predecessor_terminal_result`.
    pub predecessor_terminal_result_ref: &'a ResidentRecordRefV1,
    /// Exact terminal result authorizing reopen.
    pub predecessor_terminal_result: &'a ReconciliationResultV1,
    /// Canonical complete evidence closure inherited from the prior case chain.
    pub predecessor_evidence_closure: &'a [ReconciliationOpeningEvidenceV1],
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
}

impl ReconciliationCaseV1 {
    /// Admit an initial ambiguity case after typed-state, schema, authority,
    /// and signature resolution.
    ///
    /// # Errors
    ///
    /// Returns an error when typed bytes cannot be resolved, the case is not
    /// an exact initial record, or schema, authority, or signature checks fail.
    pub fn admit_initial_wire<F, S>(
        wire: ReconciliationCaseWireV1,
        context: &ReconciliationCaseInitialAdmissionContextV1<'_>,
        resolve_immutable: F,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
        S: FnOnce(
            &ReconciliationCasePreimageV1,
            &ReconciliationCaseDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (preimage, claimed, signature) = case_wire_parts(wire, resolve_immutable)?;
        Self::admit_parts(
            preimage,
            claimed,
            signature,
            |preimage, digest, signature| {
                validate_signed_coordinates(
                    "reconciliation case",
                    &preimage.schema_sha256,
                    context.signed.expected_schema_sha256,
                    &preimage.issuer_principal_id,
                    &preimage.issuer_key_id,
                    preimage.issuer_key_epoch,
                    context.signed.signing_authority,
                )?;
                validate_integrated_runtime_authority_coordinates(
                    "reconciliation case",
                    &preimage.deployment_runtime_authority_ref,
                    &preimage.deployment_runtime_authority_sha256,
                    context.signed.signing_authority,
                )?;
                if preimage.case_origin != ReconciliationCaseOriginV1::Initial {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "reconciliation case",
                        invariant: "initial origin",
                    });
                }
                let exact_receipt_evidence = preimage.opening_evidence.iter().any(|item| {
                    matches!(
                        item,
                        ReconciliationOpeningEvidenceV1::EffectReceipt {
                            evidence_ref,
                            evidence_sha256,
                            evidence_schema_id,
                            evidence_schema_sha256,
                        } if evidence_ref == context.receipt_ref
                            && evidence_sha256.as_str() == context.receipt.canonical_receipt_sha256().as_str()
                            && evidence_schema_id == context.receipt.schema_id()
                            && evidence_schema_sha256 == context.receipt.schema_sha256()
                    )
                });
                if !exact_receipt_evidence
                    || !*context.receipt.reconciliation_required()
                    || context.receipt.reconciliation_case_id().as_ref() != Some(&preimage.case_id)
                    || context.receipt.action_id() != context.action.action_id()
                    || context.receipt.effect_dispatch_intent_ref() != context.dispatch_ref
                    || context.receipt.effect_dispatch_intent_sha256()
                        != context.dispatch.canonical_dispatch_intent_sha256()
                    || context.receipt.effect_admission_ref() != context.admission_ref
                    || context.receipt.effect_admission_sha256()
                        != context.admission.canonical_effect_admission_sha256()
                    || preimage.action_id != *context.action.action_id()
                    || preimage.effect_admission_ref != *context.admission_ref
                    || preimage.effect_admission_sha256
                        != *context.admission.canonical_effect_admission_sha256()
                    || preimage.reserved_custody_id
                        != *context.dispatch.reserved_reconciliation_custody_id()
                    || preimage.case_id != *context.dispatch.reserved_reconciliation_case_id()
                    || preimage.conflict_resource_key_schema_id
                        != *context.action.conflict_resource_key_schema_id()
                    || preimage.conflict_resource_key_schema_sha256
                        != *context.action.conflict_resource_key_schema_sha256()
                    || preimage.conflict_resource_key != *context.action.conflict_resource_key()
                    || preimage.reconciliation_policy_ref != *context.action.reconciliation_policy()
                    || preimage.reconciliation_policy_sha256
                        != *context.reconciliation_policy_sha256
                    || preimage.expected_state_digest != *context.expected_state_digest
                {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "reconciliation case",
                        invariant:
                            "exact receipt dispatch admission action and reserved custody lineage",
                    });
                }
                verify_signature(preimage, digest, signature)
            },
        )
    }

    /// Admit a reopened case against the exact resolved predecessor case and
    /// terminal result.
    ///
    /// # Errors
    /// `admit_authority_rotation` must prove that changed runtime-authority
    /// coordinates continue the predecessor case's admitted deployment
    /// lineage.
    ///
    /// Returns [`InvalidResidentRecordV1`] when signature admission fails, the
    /// supplied records are not the exact referenced predecessors, immutable
    /// action/case-key lineage changes, authority continuity is unproved, or
    /// reopening precedes terminal closure.
    pub fn admit_reopened_wire<F, A, S>(
        wire: ReconciliationCaseWireV1,
        context: &ReconciliationCaseReopenAdmissionContextV1<'_>,
        resolve_immutable: F,
        admit_authority_rotation: A,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
        A: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        S: FnOnce(
            &ReconciliationCasePreimageV1,
            &ReconciliationCaseDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (preimage, claimed, signature) = case_wire_parts(wire, resolve_immutable)?;
        Self::admit_parts(
            preimage,
            claimed,
            signature,
            |preimage, digest, signature| {
                validate_signed_coordinates(
                    "reconciliation case",
                    &preimage.schema_sha256,
                    context.signed.expected_schema_sha256,
                    &preimage.issuer_principal_id,
                    &preimage.issuer_key_id,
                    preimage.issuer_key_epoch,
                    context.signed.signing_authority,
                )?;
                validate_integrated_runtime_authority_coordinates(
                    "reconciliation case",
                    &preimage.deployment_runtime_authority_ref,
                    &preimage.deployment_runtime_authority_sha256,
                    context.signed.signing_authority,
                )?;
                validate_reopened_case(
                    preimage,
                    context.predecessor_case_ref,
                    context.predecessor_case,
                    context.predecessor_terminal_result_ref,
                    context.predecessor_terminal_result,
                    context.predecessor_evidence_closure,
                )?;
                admit_runtime_authority_rotation(
                    "reconciliation case",
                    context.predecessor_case.deployment_runtime_authority_ref(),
                    context
                        .predecessor_case
                        .deployment_runtime_authority_sha256(),
                    &preimage.deployment_runtime_authority_ref,
                    &preimage.deployment_runtime_authority_sha256,
                    admit_authority_rotation,
                )?;
                verify_signature(preimage, digest, signature)
            },
        )
    }
}

fn case_wire_parts<F>(
    wire: ReconciliationCaseWireV1,
    mut resolve_immutable: F,
) -> Result<
    (
        ReconciliationCasePreimageV1,
        ReconciliationCaseDigest,
        CanonicalSignatureV1,
    ),
    InvalidResidentRecordV1,
>
where
    F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
{
    let expected_state_digest =
        TypedDigestV1::admit_wire(wire.expected_state_digest, &mut resolve_immutable)?;
    Ok((
        ReconciliationCasePreimageV1 {
            schema_id: wire.schema_id,
            schema_sha256: wire.schema_sha256,
            case_id: wire.case_id,
            action_id: wire.action_id,
            effect_admission_ref: wire.effect_admission_ref,
            effect_admission_sha256: wire.effect_admission_sha256,
            conflict_resource_key_schema_id: wire.conflict_resource_key_schema_id,
            conflict_resource_key_schema_sha256: wire.conflict_resource_key_schema_sha256,
            conflict_resource_key: wire.conflict_resource_key,
            reserved_custody_id: wire.reserved_custody_id,
            case_origin: wire.case_origin,
            opening_evidence: wire.opening_evidence,
            expected_state_digest,
            reconciliation_policy_ref: wire.reconciliation_policy_ref,
            reconciliation_policy_sha256: wire.reconciliation_policy_sha256,
            predecessor_case_ref: wire.predecessor_case_ref,
            predecessor_case_sha256: wire.predecessor_case_sha256,
            predecessor_terminal_result_ref: wire.predecessor_terminal_result_ref,
            predecessor_terminal_result_sha256: wire.predecessor_terminal_result_sha256,
            reopen_authority_ref: wire.reopen_authority_ref,
            reopen_authority_sha256: wire.reopen_authority_sha256,
            reopen_policy_ref: wire.reopen_policy_ref,
            reopen_policy_sha256: wire.reopen_policy_sha256,
            reopen_reason_code: wire.reopen_reason_code,
            case_revision: wire.case_revision,
            opened_at: wire.opened_at,
            deployment_runtime_authority_ref: wire.deployment_runtime_authority_ref,
            deployment_runtime_authority_sha256: wire.deployment_runtime_authority_sha256,
            issuer_principal_id: wire.issuer_principal_id,
            issuer_key_id: wire.issuer_key_id,
            issuer_key_epoch: wire.issuer_key_epoch,
        },
        wire.canonical_case_sha256,
        wire.signature,
    ))
}

fn validate_reopened_case(
    value: &ReconciliationCasePreimageV1,
    predecessor_case_ref: &ResidentRecordRefV1,
    predecessor_case: &ReconciliationCaseV1,
    predecessor_terminal_result_ref: &ResidentRecordRefV1,
    predecessor_terminal_result: &ReconciliationResultV1,
    predecessor_evidence_closure: &[ReconciliationOpeningEvidenceV1],
) -> Result<(), InvalidResidentRecordV1> {
    let reject = |invariant| InvalidResidentRecordV1::ContextualAdmission {
        record: "reopened reconciliation case",
        invariant,
    };
    if value.case_origin != ReconciliationCaseOriginV1::Reopened {
        return Err(reject("origin"));
    }
    if value.predecessor_case_ref.as_ref() != Some(predecessor_case_ref)
        || value.predecessor_case_sha256.as_ref() != Some(predecessor_case.canonical_case_sha256())
        || value.predecessor_terminal_result_ref.as_ref() != Some(predecessor_terminal_result_ref)
        || value.predecessor_terminal_result_sha256.as_ref()
            != Some(predecessor_terminal_result.canonical_reconciliation_sha256())
    {
        return Err(reject("exact predecessor references"));
    }
    if predecessor_terminal_result.case_ref() != predecessor_case_ref
        || predecessor_terminal_result.case_sha256() != predecessor_case.canonical_case_sha256()
        || predecessor_terminal_result.closed_at().is_none()
    {
        return Err(reject("resolved terminal predecessor"));
    }
    if value.case_id == *predecessor_case.case_id()
        || value.reserved_custody_id == *predecessor_case.reserved_custody_id()
    {
        return Err(reject("fresh case and custody identities"));
    }
    if value.action_id != *predecessor_case.action_id()
        || value.effect_admission_ref != *predecessor_case.effect_admission_ref()
        || value.effect_admission_sha256 != *predecessor_case.effect_admission_sha256()
        || value.conflict_resource_key_schema_id
            != *predecessor_case.conflict_resource_key_schema_id()
        || value.conflict_resource_key_schema_sha256
            != *predecessor_case.conflict_resource_key_schema_sha256()
        || value.conflict_resource_key != *predecessor_case.conflict_resource_key()
        || value.expected_state_digest != *predecessor_case.expected_state_digest()
        || value.reconciliation_policy_ref != *predecessor_case.reconciliation_policy_ref()
        || value.reconciliation_policy_sha256 != *predecessor_case.reconciliation_policy_sha256()
    {
        return Err(reject("immutable action and conflict-key lineage"));
    }
    if value.opened_at
        < *predecessor_terminal_result
            .closed_at()
            .as_ref()
            .expect("checked above")
    {
        return Err(reject("monotone opening time"));
    }
    strictly_sorted(
        "predecessor reconciliation evidence closure",
        predecessor_evidence_closure,
    )?;
    if predecessor_evidence_closure != predecessor_case.opening_evidence() {
        return Err(reject("complete predecessor evidence closure"));
    }
    if !value.opening_evidence.iter().any(|item| {
        matches!(
            item,
            ReconciliationOpeningEvidenceV1::Observation { .. }
                | ReconciliationOpeningEvidenceV1::ReviewerEvidence { .. }
        ) && predecessor_evidence_closure.binary_search(item).is_err()
    }) {
        return Err(reject("at least one new observation or reviewer item"));
    }
    Ok(())
}

/// Exact predecessor and signing coordinate shared by a custody transition.
#[derive(Clone, Copy)]
pub struct ReconciliationCustodyPredecessorV1<'a> {
    /// Reference resolving `predecessor`.
    pub predecessor_ref: &'a ResidentRecordRefV1,
    /// Exact current custody generation.
    pub predecessor: &'a ReconciliationCustodyV1,
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
}

/// Closed transition context for transfer or terminal closure of custody.
pub enum ReconciliationCustodyTransitionContextV1<'a> {
    /// Transfer to a different principal/node holder tuple.
    Transfer {
        /// Shared exact predecessor lineage.
        predecessor: ReconciliationCustodyPredecessorV1<'a>,
    },
    /// Close against the exact terminal result for the current custody.
    Close {
        /// Shared exact predecessor lineage.
        predecessor: ReconciliationCustodyPredecessorV1<'a>,
        /// Reference resolving `terminal_result`.
        terminal_result_ref: &'a ResidentRecordRefV1,
        /// Exact terminal result closing custody.
        terminal_result: &'a ReconciliationResultV1,
    },
}

impl<'a> ReconciliationCustodyTransitionContextV1<'a> {
    fn predecessor(&self) -> ReconciliationCustodyPredecessorV1<'a> {
        match self {
            Self::Transfer { predecessor } | Self::Close { predecessor, .. } => *predecessor,
        }
    }
}

impl ReconciliationCustodyV1 {
    /// Admit the initial active custody generation for an exact resolved case.
    ///
    /// # Errors
    ///
    /// Returns an error when the wire is invalid, custody is not the exact
    /// initial generation for the case, or authority or signature checks fail.
    pub fn admit_initial_wire<F>(
        wire: ReconciliationCustodyWireV1,
        case_ref: &ResidentRecordRefV1,
        case: &ReconciliationCaseV1,
        expected_schema_sha256: &SchemaDigest,
        signing_authority: &SigningAuthorityBindingV1,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &ReconciliationCustodyPreimageV1,
            &ReconciliationCustodyDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_signed_coordinates(
                "reconciliation custody",
                &preimage.schema_sha256,
                expected_schema_sha256,
                &preimage.issuer_principal_id,
                &preimage.issuer_key_id,
                preimage.issuer_key_epoch,
                signing_authority,
            )?;
            validate_integrated_runtime_authority_coordinates(
                "reconciliation custody",
                &preimage.deployment_runtime_authority_ref,
                &preimage.deployment_runtime_authority_sha256,
                signing_authority,
            )?;
            if preimage.custody_generation.get() != 1
                || preimage.custody_state != ReconciliationCustodyStateV1::Active
                || preimage.case_ref != *case_ref
                || preimage.case_sha256 != *case.canonical_case_sha256()
                || preimage.custody_id != *case.reserved_custody_id()
                || preimage.conflict_resource_key_schema_id
                    != *case.conflict_resource_key_schema_id()
                || preimage.conflict_resource_key_schema_sha256
                    != *case.conflict_resource_key_schema_sha256()
                || preimage.conflict_resource_key != *case.conflict_resource_key()
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "reconciliation custody",
                    invariant: "exact initial case key and custody lineage",
                });
            }
            verify_signature(preimage, digest, signature)
        })
    }

    /// Admit an exact successor custody generation against its resolved predecessor.
    ///
    /// Every transition advances the custody generation exactly once under the
    /// module-level lineage law; transfer and close then apply their distinct,
    /// closed evidence laws.
    ///
    /// # Errors
    /// `admit_authority_rotation` must prove that changed runtime-authority
    /// coordinates continue the predecessor custody's admitted deployment
    /// lineage.
    ///
    /// Returns [`InvalidResidentRecordV1`] when signature admission fails, the
    /// generation does not advance exactly once, the predecessor reference is
    /// not exact, immutable case/key/custody fields change, authority
    /// continuity is unproved, or time regresses.
    pub fn admit_successor_wire<A, F>(
        wire: ReconciliationCustodyWireV1,
        context: &ReconciliationCustodyTransitionContextV1<'_>,
        admit_authority_rotation: A,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        A: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        F: FnOnce(
            &ReconciliationCustodyPreimageV1,
            &ReconciliationCustodyDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let predecessor = context.predecessor();
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_signed_coordinates(
                "reconciliation custody",
                &preimage.schema_sha256,
                predecessor.signed.expected_schema_sha256,
                &preimage.issuer_principal_id,
                &preimage.issuer_key_id,
                preimage.issuer_key_epoch,
                predecessor.signed.signing_authority,
            )?;
            validate_integrated_runtime_authority_coordinates(
                "reconciliation custody",
                &preimage.deployment_runtime_authority_ref,
                &preimage.deployment_runtime_authority_sha256,
                predecessor.signed.signing_authority,
            )?;
            validate_custody_successor(
                preimage,
                predecessor.predecessor_ref,
                predecessor.predecessor,
            )?;
            validate_custody_transition(preimage, context)?;
            admit_runtime_authority_rotation(
                "reconciliation custody",
                predecessor.predecessor.deployment_runtime_authority_ref(),
                predecessor
                    .predecessor
                    .deployment_runtime_authority_sha256(),
                &preimage.deployment_runtime_authority_ref,
                &preimage.deployment_runtime_authority_sha256,
                admit_authority_rotation,
            )?;
            verify_signature(preimage, digest, signature)
        })
    }
}

fn validate_custody_successor(
    value: &ReconciliationCustodyPreimageV1,
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &ReconciliationCustodyV1,
) -> Result<(), InvalidResidentRecordV1> {
    let reject = |invariant| InvalidResidentRecordV1::ContextualAdmission {
        record: "reconciliation custody successor",
        invariant,
    };
    if predecessor.custody_state() == &ReconciliationCustodyStateV1::Closed {
        return Err(reject("closed predecessor"));
    }
    if predecessor
        .custody_generation()
        .checked_successor()
        .is_none_or(|next| value.custody_generation != next)
    {
        return Err(reject("exact next generation"));
    }
    if value.previous_custody_ref.as_ref() != Some(predecessor_ref)
        || value.previous_custody_sha256.as_ref() != Some(predecessor.canonical_custody_sha256())
    {
        return Err(reject("exact predecessor reference"));
    }
    if value.custody_id != *predecessor.custody_id()
        || value.case_ref != *predecessor.case_ref()
        || value.case_sha256 != *predecessor.case_sha256()
        || value.conflict_resource_key_schema_id != *predecessor.conflict_resource_key_schema_id()
        || value.conflict_resource_key_schema_sha256
            != *predecessor.conflict_resource_key_schema_sha256()
        || value.conflict_resource_key != *predecessor.conflict_resource_key()
    {
        return Err(reject("immutable custody case and conflict key"));
    }
    if predecessor.transferred_or_closed_at().is_some_and(|prior| {
        value
            .transferred_or_closed_at
            .is_none_or(|current| current < prior)
    }) {
        return Err(reject("monotone custody time"));
    }
    Ok(())
}

fn validate_custody_transition(
    value: &ReconciliationCustodyPreimageV1,
    context: &ReconciliationCustodyTransitionContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    let reject = |invariant| InvalidResidentRecordV1::ContextualAdmission {
        record: "reconciliation custody successor",
        invariant,
    };
    let predecessor = context.predecessor();
    match context {
        ReconciliationCustodyTransitionContextV1::Transfer { .. } => {
            if value.custody_state != ReconciliationCustodyStateV1::Transferred
                || (value.holder_principal_id == *predecessor.predecessor.holder_principal_id()
                    && value.holder_node_id.as_ref()
                        == predecessor.predecessor.holder_node_id().as_ref())
            {
                return Err(reject("different successor holder tuple"));
            }
        }
        ReconciliationCustodyTransitionContextV1::Close {
            terminal_result_ref,
            terminal_result,
            ..
        } => {
            let exact = value.custody_state == ReconciliationCustodyStateV1::Closed
                && value.holder_principal_id == *predecessor.predecessor.holder_principal_id()
                && value.holder_node_id.as_ref()
                    == predecessor.predecessor.holder_node_id().as_ref()
                && value.terminal_result_ref.as_ref() == Some(*terminal_result_ref)
                && value.terminal_result_sha256.as_ref()
                    == Some(terminal_result.canonical_reconciliation_sha256())
                && terminal_result.closed_at().is_some()
                && terminal_result.case_ref() == predecessor.predecessor.case_ref()
                && terminal_result.case_sha256() == predecessor.predecessor.case_sha256()
                && terminal_result.custody_ref() == predecessor.predecessor_ref
                && terminal_result.custody_sha256()
                    == predecessor.predecessor.canonical_custody_sha256()
                && terminal_result.custody_generation()
                    == predecessor.predecessor.custody_generation()
                && value.transferred_or_closed_at.as_ref() >= terminal_result.closed_at().as_ref();
            if !exact {
                return Err(reject("exact terminal result for current custody"));
            }
        }
    }
    Ok(())
}

/// Resolved lineage for initial reconciliation-result admission.
pub struct ReconciliationResultInitialAdmissionContextV1<'a> {
    /// Caller-signed envelope carrying `request`.
    pub outer_request: &'a CommonRequestEnvelopeV1,
    /// Exact admitted reconciliation request body.
    pub request: &'a ReconciliationRequestV1,
    /// Complete owner-resolved request binding expected in the result.
    pub request_binding: &'a RequestBindingV1,
    /// Optional separately resolved compensation lineage.
    pub compensation: Option<&'a ResolvedReconciliationCompensationV1<'a>>,
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
}

/// Resolved lineage for successor reconciliation-result admission.
pub struct ReconciliationResultSuccessorAdmissionContextV1<'a> {
    /// Caller-signed envelope carrying `request`.
    pub outer_request: &'a CommonRequestEnvelopeV1,
    /// Exact admitted reconciliation request body for this compare-and-swap.
    pub request: &'a ReconciliationRequestV1,
    /// Complete owner-resolved request binding expected in the result.
    pub request_binding: &'a RequestBindingV1,
    /// Reference resolving `predecessor`.
    pub predecessor_ref: &'a ResidentRecordRefV1,
    /// Exact open predecessor result.
    pub predecessor: &'a ReconciliationResultV1,
    /// Optional separately resolved compensation lineage.
    pub compensation: Option<&'a ResolvedReconciliationCompensationV1<'a>>,
    /// Exact schema and signer coordinates.
    pub signed: SignedResidentAdmissionContextV1<'a>,
}

impl ReconciliationResultV1 {
    /// Admit an initial result revision with resolved typed state, schema,
    /// signing authority, and exact reconciliation request body.
    ///
    /// # Errors
    ///
    /// Returns an error when typed bytes cannot be resolved, the result does
    /// not match the request, or schema, authority, or signature checks fail.
    pub fn admit_initial_wire<F, S>(
        wire: ReconciliationResultWireV1,
        context: &ReconciliationResultInitialAdmissionContextV1<'_>,
        resolve_immutable: F,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
        S: FnOnce(
            &ReconciliationResultPreimageV1,
            &ReconciliationResultDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (preimage, claimed, signature) = result_wire_parts(wire, resolve_immutable)?;
        Self::admit_parts(
            preimage,
            claimed,
            signature,
            |preimage, digest, signature| {
                validate_signed_coordinates(
                    "reconciliation result",
                    &preimage.schema_sha256,
                    context.signed.expected_schema_sha256,
                    &preimage.issuer_principal_id,
                    &preimage.issuer_key_id,
                    preimage.issuer_key_epoch,
                    context.signed.signing_authority,
                )?;
                validate_integrated_runtime_authority_coordinates(
                    "reconciliation result",
                    &preimage.deployment_runtime_authority_ref,
                    &preimage.deployment_runtime_authority_sha256,
                    context.signed.signing_authority,
                )?;
                validate_result_against_request(
                    preimage,
                    context.outer_request,
                    context.request,
                    context.request_binding,
                )?;
                validate_compensation_context(preimage, context.compensation)?;
                verify_signature(preimage, digest, signature)
            },
        )
    }

    /// Admit an exact successor result revision against its resolved predecessor.
    ///
    /// Its compare-and-swap law advances the result revision exactly once under
    /// the module-level lineage law, bound to the exact caller-signed request
    /// and predecessor reference/digest pair.
    ///
    /// # Errors
    /// `admit_authority_rotation` must prove that changed runtime-authority
    /// coordinates continue the predecessor result's admitted deployment
    /// lineage.
    ///
    /// Returns [`InvalidResidentRecordV1`] when signature admission fails, the
    /// revision skips, predecessor identity differs, immutable case lineage
    /// changes, the request does not prove any custody transfer, authority
    /// continuity is unproved, a terminal result is extended, or time
    /// regresses.
    pub fn admit_successor_wire<F, A, S>(
        wire: ReconciliationResultWireV1,
        context: &ReconciliationResultSuccessorAdmissionContextV1<'_>,
        resolve_immutable: F,
        admit_authority_rotation: A,
        verify_signature: S,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
        A: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        S: FnOnce(
            &ReconciliationResultPreimageV1,
            &ReconciliationResultDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let (preimage, claimed, signature) = result_wire_parts(wire, resolve_immutable)?;
        Self::admit_parts(
            preimage,
            claimed,
            signature,
            |preimage, digest, signature| {
                validate_signed_coordinates(
                    "reconciliation result",
                    &preimage.schema_sha256,
                    context.signed.expected_schema_sha256,
                    &preimage.issuer_principal_id,
                    &preimage.issuer_key_id,
                    preimage.issuer_key_epoch,
                    context.signed.signing_authority,
                )?;
                validate_integrated_runtime_authority_coordinates(
                    "reconciliation result",
                    &preimage.deployment_runtime_authority_ref,
                    &preimage.deployment_runtime_authority_sha256,
                    context.signed.signing_authority,
                )?;
                validate_result_against_request(
                    preimage,
                    context.outer_request,
                    context.request,
                    context.request_binding,
                )?;
                validate_result_successor(preimage, context.predecessor_ref, context.predecessor)?;
                validate_compensation_context(preimage, context.compensation)?;
                admit_runtime_authority_rotation(
                    "reconciliation result",
                    context.predecessor.deployment_runtime_authority_ref(),
                    context.predecessor.deployment_runtime_authority_sha256(),
                    &preimage.deployment_runtime_authority_ref,
                    &preimage.deployment_runtime_authority_sha256,
                    admit_authority_rotation,
                )?;
                verify_signature(preimage, digest, signature)
            },
        )
    }
}

fn validate_result_against_request(
    value: &ReconciliationResultPreimageV1,
    outer_request: &CommonRequestEnvelopeV1,
    request: &ReconciliationRequestV1,
    expected_request_binding: &RequestBindingV1,
) -> Result<(), InvalidResidentRecordV1> {
    let reject = |invariant| InvalidResidentRecordV1::ContextualAdmission {
        record: "reconciliation result request binding",
        invariant,
    };
    outer_request.verify_reconciliation_request_body(request)?;
    validate_exact_request_binding(
        &value.request_binding,
        expected_request_binding,
        "reconciliation result",
    )?;
    let expected_revision = request
        .expected_case_revision()
        .checked_successor()
        .ok_or_else(|| reject("bounded next request revision"))?;
    if value.case_revision != expected_revision
        || value.predecessor_result_ref.as_ref() != request.predecessor_result_ref.as_ref()
        || value.predecessor_result_sha256.as_ref() != request.predecessor_result_sha256.as_ref()
        || value.request_binding.request_id() != outer_request.request_id()
        || value.request_binding.canonical_request_sha256()
            != outer_request.canonical_request_sha256()
        || value.reconciliation_id != *request.reconciliation_id()
        || value.case_ref != *request.case_ref()
        || value.case_sha256 != *request.case_sha256()
        || value.custody_ref != *request.custody_ref()
        || value.custody_sha256 != *request.custody_sha256()
        || value.custody_generation != *request.expected_custody_generation()
        || value.action_id != *request.action_id()
        || value.effect_admission_ref != *request.effect_admission_ref()
        || value.effect_admission_sha256 != *request.effect_admission_sha256()
        || value.effect_receipts.as_slice() != request.effect_receipts()
        || value.expected_state_digest != *request.expected_state_digest()
        || value.reconciliation_policy_ref != *request.reconciliation_policy_ref()
        || value.reconciliation_policy_sha256 != *request.reconciliation_policy_sha256()
        || value.requested_at != *request.requested_at()
        || value.principal_scope != outer_request.scope().principal_scope
        || value.controller_scope != outer_request.scope().controller_scope
        || value.relationship_scope != outer_request.scope().relationship_scope
    {
        return Err(reject("exact request compare-and-swap and result lineage"));
    }
    Ok(())
}

fn validate_compensation_context(
    value: &ReconciliationResultPreimageV1,
    resolved: Option<&ResolvedReconciliationCompensationV1<'_>>,
) -> Result<(), InvalidResidentRecordV1> {
    let reject = |invariant| InvalidResidentRecordV1::ContextualAdmission {
        record: "reconciliation compensation",
        invariant,
    };
    match (&value.compensation, resolved) {
        (None, None) if value.resolution != Some(ReconciliationResolutionV1::Compensated) => Ok(()),
        (Some(binding), Some(resolved))
            if value.resolution == Some(ReconciliationResolutionV1::Compensated) =>
        {
            resolved.request.verify_action_body(resolved.action)?;
            validate_exact_request_binding(
                resolved.result.request_binding(),
                resolved.request_binding,
                "reconciliation compensation result",
            )?;
            let action_digest = resolved.action.canonical_action_body_sha256()?;
            let exact = binding.compensation_request_ref == *resolved.request_ref
                && binding.compensation_request_sha256
                    == *resolved.request.canonical_request_sha256()
                && binding.compensation_action_ref == *resolved.action_ref
                && binding.compensation_action_sha256 == action_digest
                && binding.compensation_result_ref == *resolved.result_ref
                && binding.compensation_result_sha256
                    == *resolved.result.canonical_receipt_sha256()
                && resolved.result.request_binding().request_id() == resolved.request.request_id()
                && resolved.result.request_binding().canonical_request_sha256()
                    == resolved.request.canonical_request_sha256()
                && resolved.result.action_id() == resolved.action.action_id()
                && resolved.result.action_body_sha256() == &action_digest
                && resolved.result.observed_effect() == &ObservedEffectV1::Confirmed
                && binding.case_ref == value.case_ref
                && binding.case_sha256 == value.case_sha256
                && binding.custody_ref == value.custody_ref
                && binding.custody_sha256 == value.custody_sha256
                && binding.effect_admission_ref == value.effect_admission_ref
                && binding.effect_admission_sha256 == value.effect_admission_sha256
                && binding.effect_receipts == value.effect_receipts;
            if exact {
                Ok(())
            } else {
                Err(reject(
                    "exact admitted compensation request action and result lineage",
                ))
            }
        }
        _ => Err(reject("resolved compensation iff compensated")),
    }
}

fn result_wire_parts<F>(
    wire: ReconciliationResultWireV1,
    mut resolve_immutable: F,
) -> Result<
    (
        ReconciliationResultPreimageV1,
        ReconciliationResultDigest,
        CanonicalSignatureV1,
    ),
    InvalidResidentRecordV1,
>
where
    F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
{
    let expected_state_digest =
        TypedDigestV1::admit_wire(wire.expected_state_digest, &mut resolve_immutable)?;
    let observed_state_digest = wire
        .observed_state_digest
        .map(|value| TypedDigestV1::admit_wire(value, &mut resolve_immutable))
        .transpose()?;
    Ok((
        ReconciliationResultPreimageV1 {
            schema_id: wire.schema_id,
            schema_sha256: wire.schema_sha256,
            request_binding: wire.request_binding,
            reconciliation_id: wire.reconciliation_id,
            case_ref: wire.case_ref,
            case_sha256: wire.case_sha256,
            custody_ref: wire.custody_ref,
            custody_sha256: wire.custody_sha256,
            custody_generation: wire.custody_generation,
            case_revision: wire.case_revision,
            predecessor_result_ref: wire.predecessor_result_ref,
            predecessor_result_sha256: wire.predecessor_result_sha256,
            action_id: wire.action_id,
            effect_admission_ref: wire.effect_admission_ref,
            effect_admission_sha256: wire.effect_admission_sha256,
            effect_receipts: wire.effect_receipts,
            principal_scope: wire.principal_scope,
            controller_scope: wire.controller_scope,
            relationship_scope: wire.relationship_scope,
            reconciliation_policy_ref: wire.reconciliation_policy_ref,
            reconciliation_policy_sha256: wire.reconciliation_policy_sha256,
            expected_state_digest,
            observed_state_digest,
            observation_evidence: wire.observation_evidence,
            absence_evidence_ref: wire.absence_evidence_ref,
            absence_evidence_sha256: wire.absence_evidence_sha256,
            known_attempt_ids: wire.known_attempt_ids,
            case_state: wire.case_state,
            resolution: wire.resolution,
            compensation: wire.compensation,
            owner_evidence_ref: wire.owner_evidence_ref,
            owner_evidence_sha256: wire.owner_evidence_sha256,
            reviewer_or_policy_ref: wire.reviewer_or_policy_ref,
            reviewer_or_policy_sha256: wire.reviewer_or_policy_sha256,
            requested_at: wire.requested_at,
            recorded_at: wire.recorded_at,
            closed_at: wire.closed_at,
            deployment_runtime_authority_ref: wire.deployment_runtime_authority_ref,
            deployment_runtime_authority_sha256: wire.deployment_runtime_authority_sha256,
            issuer_principal_id: wire.issuer_principal_id,
            issuer_key_id: wire.issuer_key_id,
            issuer_key_epoch: wire.issuer_key_epoch,
        },
        wire.canonical_reconciliation_sha256,
        wire.signature,
    ))
}

fn validate_result_successor(
    value: &ReconciliationResultPreimageV1,
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &ReconciliationResultV1,
) -> Result<(), InvalidResidentRecordV1> {
    let reject = |invariant| InvalidResidentRecordV1::ContextualAdmission {
        record: "reconciliation result successor",
        invariant,
    };
    if predecessor
        .case_revision()
        .checked_successor()
        .is_none_or(|next| value.case_revision != next)
    {
        return Err(reject("exact next revision"));
    }
    if value.predecessor_result_ref.as_ref() != Some(predecessor_ref)
        || value.predecessor_result_sha256.as_ref()
            != Some(predecessor.canonical_reconciliation_sha256())
    {
        return Err(reject("exact predecessor reference"));
    }
    if predecessor.closed_at().is_some() {
        return Err(reject("terminal predecessor"));
    }
    if value.reconciliation_id != *predecessor.reconciliation_id()
        || value.case_ref != *predecessor.case_ref()
        || value.case_sha256 != *predecessor.case_sha256()
        || value.action_id != *predecessor.action_id()
        || value.effect_admission_ref != *predecessor.effect_admission_ref()
        || value.effect_admission_sha256 != *predecessor.effect_admission_sha256()
        || value.principal_scope != *predecessor.principal_scope()
        || value.controller_scope != *predecessor.controller_scope()
        || value.relationship_scope != *predecessor.relationship_scope()
        || value.reconciliation_policy_ref != *predecessor.reconciliation_policy_ref()
        || value.reconciliation_policy_sha256 != *predecessor.reconciliation_policy_sha256()
        || value.expected_state_digest != *predecessor.expected_state_digest()
    {
        return Err(reject("immutable result case lineage"));
    }
    if value.requested_at < *predecessor.requested_at()
        || value.recorded_at < *predecessor.recorded_at()
        || predecessor
            .closed_at()
            .zip(value.closed_at.as_ref())
            .is_some_and(|(prior, current)| *current < prior)
    {
        return Err(reject("monotone result time"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/internal/records/resident/reconciliation.rs"]
mod tests;

/// State of one non-expiring exclusive reconciliation-custody generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationCustodyStateV1 {
    /// Exclusively held for active reconciliation.
    Active,
    /// Superseded by a successor custody record.
    Transferred,
    /// Closed by one terminal result.
    Closed,
}

resident_signed_record!(
    /// Exclusive custody of one exact conflict-resource key.
    ReconciliationCustodyV1, ReconciliationCustodyWireV1, ReconciliationCustodyPreimageV1,
    canonical_custody_sha256: ReconciliationCustodyDigest,
    RECONCILIATION_CUSTODY_V1_SCHEMA, "reconciliation custody",
    {
        /// Stable identity shared by the custody succession chain.
        custody_id: ReconciliationCustodyIdV1,
        /// Immutable ambiguity case protected by this custody generation.
        case_ref: ResidentRecordRefV1,
        /// Digest of the exact case bytes.
        case_sha256: ReconciliationCaseDigest,
        /// Schema identity defining the protected conflict-resource key.
        conflict_resource_key_schema_id: SchemaId,
        /// Digest of the conflict-resource-key schema bytes.
        conflict_resource_key_schema_sha256: SchemaDigest,
        /// Canonical typed/scoped resource key blocked by custody.
        conflict_resource_key: serde_json::Value,
        /// Positive generation advanced by every transfer or closure.
        custody_generation: ReconciliationCustodyGenerationV1,
        /// Lifecycle state of this immutable custody generation.
        custody_state: ReconciliationCustodyStateV1,
        /// Principal exclusively responsible for the current generation.
        holder_principal_id: ResidentPrincipalIdV1,
        /// Optional responsible node within the holder principal.
        holder_node_id: Option<ResidentNodeIdV1>,
        /// Prior custody generation, absent only at generation one.
        previous_custody_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::previous_custody_ref`].
        previous_custody_sha256: Option<ReconciliationCustodyDigest>,
        /// Terminal result that closes custody; absent while active or transferred.
        terminal_result_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::terminal_result_ref`].
        terminal_result_sha256: Option<ReconciliationResultDigest>,
        /// Time of transfer or closure; absent only for active generation one.
        transferred_or_closed_at: Option<UnixTimeV1>,
        /// Exact deployment-runtime authority under which this record was issued.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Principal under whose authority this custody generation was issued.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Key identity used for the custody signature.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch checked for custody.
        issuer_key_epoch: SigningKeyEpochV1
    },
    |value: &ReconciliationCustodyPreimageV1| {
        validate_bounded_json(
            "reconciliation custody conflict-resource key bytes",
            &value.conflict_resource_key,
        )?;
        if value.previous_custody_ref.is_some() != value.previous_custody_sha256.is_some()
            || (value.custody_generation.get() == 1) != value.previous_custody_ref.is_none()
        { return Err(InvalidResidentRecordV1::SettlementState); }
        let terminal = value.terminal_result_ref.is_some()
            && value.terminal_result_sha256.is_some()
            && value.transferred_or_closed_at.is_some();
        match value.custody_state {
            ReconciliationCustodyStateV1::Active if value.custody_generation.get() == 1 && !terminal
                && value.terminal_result_ref.is_none()
                && value.terminal_result_sha256.is_none()
                && value.transferred_or_closed_at.is_none() => Ok(()),
            ReconciliationCustodyStateV1::Transferred if value.custody_generation.get() > 1
                && value.transferred_or_closed_at.is_some()
                && value.terminal_result_ref.is_none() && value.terminal_result_sha256.is_none() => Ok(()),
            ReconciliationCustodyStateV1::Closed if value.custody_generation.get() > 1 && terminal => Ok(()),
            _ => Err(InvalidResidentRecordV1::SettlementState),
        }
    }
);

/// Immutable effect-receipt coordinate used by reconciliation.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationEffectReceiptRefV1 {
    /// Immutable receipt reference.
    pub receipt_ref: ResidentRecordRefV1,
    /// Exact receipt digest.
    pub receipt_sha256: EffectReceiptDigest,
}

/// Candidate unsigned body admitted into [`ReconciliationRequestV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReconciliationRequestPreimageV1 {
    /// Stable identity shared with the admitted result chain.
    pub reconciliation_id: ReconciliationIdV1,
    /// Exact ambiguity case requested for observation.
    pub case_ref: ResidentRecordRefV1,
    /// Digest of the exact ambiguity-case bytes.
    pub case_sha256: ReconciliationCaseDigest,
    /// Exact exclusive custody generation authorizing observation.
    pub custody_ref: ResidentRecordRefV1,
    /// Digest of the exact custody-record bytes.
    pub custody_sha256: ReconciliationCustodyDigest,
    /// Custody generation expected by compare-and-swap.
    pub expected_custody_generation: ReconciliationCustodyGenerationV1,
    /// Current case-result revision expected by compare-and-swap.
    pub expected_case_revision: ReconciliationCaseRevisionV1,
    /// Exact prior result reference, absent only before revision one.
    pub predecessor_result_ref: Option<ResidentRecordRefV1>,
    /// Digest paired with [`Self::predecessor_result_ref`].
    pub predecessor_result_sha256: Option<ReconciliationResultDigest>,
    /// Logical action whose outcome will be observed.
    pub action_id: ActionIdV1,
    /// Immutable effect admission governing the action.
    pub effect_admission_ref: ResidentRecordRefV1,
    /// Digest of the exact effect-admission bytes.
    pub effect_admission_sha256: EffectAdmissionDigest,
    /// Canonically ordered effect receipts considered by this request.
    pub effect_receipts: Vec<ReconciliationEffectReceiptRefV1>,
    /// Typed expected state fixed by the ambiguity case.
    pub expected_state_digest: TypedDigestV1,
    /// Immutable reconciliation policy applied to observation.
    pub reconciliation_policy_ref: ResidentRecordRefV1,
    /// Digest of the reconciliation-policy bytes.
    pub reconciliation_policy_sha256: ArtifactDigest,
    /// Bounded canonical owner-specific observation limits.
    pub observation_bounds: serde_json::Value,
    /// Time at which the reconciliation request was issued.
    pub requested_at: UnixTimeV1,
}

/// Untrusted structural wire body for [`ReconciliationRequestV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationRequestWireV1 {
    reconciliation_id: ReconciliationIdV1,
    case_ref: ResidentRecordRefV1,
    case_sha256: ReconciliationCaseDigest,
    custody_ref: ResidentRecordRefV1,
    custody_sha256: ReconciliationCustodyDigest,
    expected_custody_generation: ReconciliationCustodyGenerationV1,
    expected_case_revision: ReconciliationCaseRevisionV1,
    predecessor_result_ref: Option<ResidentRecordRefV1>,
    predecessor_result_sha256: Option<ReconciliationResultDigest>,
    action_id: ActionIdV1,
    effect_admission_ref: ResidentRecordRefV1,
    effect_admission_sha256: EffectAdmissionDigest,
    effect_receipts: Vec<ReconciliationEffectReceiptRefV1>,
    expected_state_digest: TypedDigestWireV1,
    reconciliation_policy_ref: ResidentRecordRefV1,
    reconciliation_policy_sha256: ArtifactDigest,
    observation_bounds: serde_json::Value,
    requested_at: UnixTimeV1,
}

/// Semantically admitted read-only reconciliation body.
///
/// Caller authority and the only signature live in the enclosing
/// [`CommonRequestEnvelopeV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationRequestV1 {
    reconciliation_id: ReconciliationIdV1,
    case_ref: ResidentRecordRefV1,
    case_sha256: ReconciliationCaseDigest,
    custody_ref: ResidentRecordRefV1,
    custody_sha256: ReconciliationCustodyDigest,
    expected_custody_generation: ReconciliationCustodyGenerationV1,
    expected_case_revision: ReconciliationCaseRevisionV1,
    predecessor_result_ref: Option<ResidentRecordRefV1>,
    predecessor_result_sha256: Option<ReconciliationResultDigest>,
    action_id: ActionIdV1,
    effect_admission_ref: ResidentRecordRefV1,
    effect_admission_sha256: EffectAdmissionDigest,
    effect_receipts: Vec<ReconciliationEffectReceiptRefV1>,
    expected_state_digest: TypedDigestV1,
    reconciliation_policy_ref: ResidentRecordRefV1,
    reconciliation_policy_sha256: ArtifactDigest,
    observation_bounds: serde_json::Value,
    requested_at: UnixTimeV1,
}

/// Resolved caller, case, custody, and predecessor for request admission.
pub struct ReconciliationRequestAdmissionContextV1<'a> {
    /// Caller-signed envelope carrying the request body.
    pub outer_request: &'a CommonRequestEnvelopeV1,
    /// Reference resolving `case`.
    pub case_ref: &'a ResidentRecordRefV1,
    /// Exact ambiguity case.
    pub case: &'a ReconciliationCaseV1,
    /// Reference resolving `custody`.
    pub custody_ref: &'a ResidentRecordRefV1,
    /// Exact active or transferred custody generation.
    pub custody: &'a ReconciliationCustodyV1,
    /// Optional exact open predecessor result.
    pub predecessor: Option<(&'a ResidentRecordRefV1, &'a ReconciliationResultV1)>,
    /// Ordered custody successors strictly between the predecessor result's
    /// custody and `custody`.
    ///
    /// The slice is empty when the predecessor result already names the
    /// current custody. Every supplied member must be an admitted successor in
    /// the same case; it permits holder transfer without breaking the
    /// append-only result chain.
    pub intervening_custody: &'a [ReconciliationCustodyLineageMemberV1<'a>],
}

/// One admitted custody successor used to prove a transferred-custody path.
#[derive(Debug, Clone, Copy)]
pub struct ReconciliationCustodyLineageMemberV1<'a> {
    /// Immutable reference resolving `custody`.
    pub custody_ref: &'a ResidentRecordRefV1,
    /// Exact admitted custody successor at `custody_ref`.
    pub custody: &'a ReconciliationCustodyV1,
}

impl ReconciliationRequestV1 {
    pub(crate) fn seal(
        preimage: ReconciliationRequestPreimageV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        validate_reconciliation_request(&preimage)?;
        Ok(Self::from_preimage(preimage))
    }

    /// Admit a request body against its caller-signed common envelope and exact
    /// resolved case, custody, optional predecessor result, and any intervening
    /// admitted custody transfers.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when signature admission fails or
    /// any expected compare-and-swap coordinate, immutable lineage, active
    /// custody relationship, or request time differs from resolved context.
    pub fn admit_contextual_wire<F>(
        wire: ReconciliationRequestWireV1,
        context: &ReconciliationRequestAdmissionContextV1<'_>,
        mut resolve_immutable: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnMut(&ResidentRecordRefV1) -> Option<Vec<u8>>,
    {
        let preimage = ReconciliationRequestPreimageV1 {
            reconciliation_id: wire.reconciliation_id,
            case_ref: wire.case_ref,
            case_sha256: wire.case_sha256,
            custody_ref: wire.custody_ref,
            custody_sha256: wire.custody_sha256,
            expected_custody_generation: wire.expected_custody_generation,
            expected_case_revision: wire.expected_case_revision,
            predecessor_result_ref: wire.predecessor_result_ref,
            predecessor_result_sha256: wire.predecessor_result_sha256,
            action_id: wire.action_id,
            effect_admission_ref: wire.effect_admission_ref,
            effect_admission_sha256: wire.effect_admission_sha256,
            effect_receipts: wire.effect_receipts,
            expected_state_digest: TypedDigestV1::admit_wire(
                wire.expected_state_digest,
                &mut resolve_immutable,
            )?,
            reconciliation_policy_ref: wire.reconciliation_policy_ref,
            reconciliation_policy_sha256: wire.reconciliation_policy_sha256,
            observation_bounds: wire.observation_bounds,
            requested_at: wire.requested_at,
        };
        validate_reconciliation_request(&preimage)?;
        let value = Self::seal(preimage.clone())?;
        context
            .outer_request
            .verify_reconciliation_request_body(&value)?;
        let reject = |invariant| InvalidResidentRecordV1::ContextualAdmission {
            record: "reconciliation request",
            invariant,
        };
        if preimage.case_ref != *context.case_ref
            || preimage.case_sha256 != *context.case.canonical_case_sha256()
            || preimage.custody_ref != *context.custody_ref
            || preimage.custody_sha256 != *context.custody.canonical_custody_sha256()
            || preimage.expected_custody_generation != *context.custody.custody_generation()
        {
            return Err(reject("exact case and custody coordinates"));
        }
        if context.custody.case_ref() != context.case_ref
            || context.custody.case_sha256() != context.case.canonical_case_sha256()
            || matches!(
                context.custody.custody_state(),
                ReconciliationCustodyStateV1::Closed
            )
        {
            return Err(reject("active custody for resolved case"));
        }
        if preimage.action_id.as_str() != context.case.action_id().as_str()
            || preimage.effect_admission_ref != *context.case.effect_admission_ref()
            || preimage.effect_admission_sha256 != *context.case.effect_admission_sha256()
            || preimage.expected_state_digest != *context.case.expected_state_digest()
            || preimage.reconciliation_policy_ref != *context.case.reconciliation_policy_ref()
            || preimage.reconciliation_policy_sha256 != *context.case.reconciliation_policy_sha256()
        {
            return Err(reject("immutable action and policy lineage"));
        }
        match context.predecessor {
            None if preimage.expected_case_revision.get() == 0
                && preimage.predecessor_result_ref.is_none()
                && context.intervening_custody.is_empty() => {}
            Some((predecessor_ref, predecessor))
                if preimage.expected_case_revision == *predecessor.case_revision()
                    && preimage.predecessor_result_ref.as_ref() == Some(predecessor_ref)
                    && preimage.predecessor_result_sha256.as_ref()
                        == Some(predecessor.canonical_reconciliation_sha256())
                    && predecessor.case_ref() == context.case_ref
                    && predecessor.case_sha256() == context.case.canonical_case_sha256()
                    && custody_descends_from_result(
                        predecessor,
                        context.intervening_custody,
                        context.custody_ref,
                        context.custody,
                    )
                    && preimage.reconciliation_id == *predecessor.reconciliation_id()
                    && predecessor.closed_at().is_none()
                    && preimage.requested_at >= *predecessor.recorded_at() => {}
            _ => return Err(reject("exact predecessor revision")),
        }
        if preimage.requested_at < *context.case.opened_at() {
            return Err(reject("monotone request time"));
        }
        if preimage.requested_at < context.outer_request.issued_at()
            || preimage.requested_at >= context.outer_request.expires_at()
        {
            return Err(reject("caller-signed request lifetime"));
        }
        Ok(value)
    }

    fn from_preimage(value: ReconciliationRequestPreimageV1) -> Self {
        let ReconciliationRequestPreimageV1 {
            reconciliation_id,
            case_ref,
            case_sha256,
            custody_ref,
            custody_sha256,
            expected_custody_generation,
            expected_case_revision,
            predecessor_result_ref,
            predecessor_result_sha256,
            action_id,
            effect_admission_ref,
            effect_admission_sha256,
            effect_receipts,
            expected_state_digest,
            reconciliation_policy_ref,
            reconciliation_policy_sha256,
            observation_bounds,
            requested_at,
        } = value;
        Self {
            reconciliation_id,
            case_ref,
            case_sha256,
            custody_ref,
            custody_sha256,
            expected_custody_generation,
            expected_case_revision,
            predecessor_result_ref,
            predecessor_result_sha256,
            action_id,
            effect_admission_ref,
            effect_admission_sha256,
            effect_receipts,
            expected_state_digest,
            reconciliation_policy_ref,
            reconciliation_policy_sha256,
            observation_bounds,
            requested_at,
        }
    }

    /// Compute the canonical identity used by the enclosing common request body.
    ///
    /// # Errors
    ///
    /// Returns an error when canonical serialization of the admitted body fails.
    pub fn canonical_reconciliation_request_sha256(
        &self,
    ) -> Result<ReconciliationRequestDigest, InvalidResidentRecordV1> {
        Ok(ReconciliationRequestDigest::hash_canonical(self)?)
    }

    /// Borrow the reconciliation chain identity.
    #[must_use]
    pub const fn reconciliation_id(&self) -> &ReconciliationIdV1 {
        &self.reconciliation_id
    }
    /// Borrow the exact case reference.
    #[must_use]
    pub const fn case_ref(&self) -> &ResidentRecordRefV1 {
        &self.case_ref
    }
    /// Borrow the exact case digest.
    #[must_use]
    pub const fn case_sha256(&self) -> &ReconciliationCaseDigest {
        &self.case_sha256
    }
    /// Borrow the exact custody reference.
    #[must_use]
    pub const fn custody_ref(&self) -> &ResidentRecordRefV1 {
        &self.custody_ref
    }
    /// Borrow the exact custody digest.
    #[must_use]
    pub const fn custody_sha256(&self) -> &ReconciliationCustodyDigest {
        &self.custody_sha256
    }
    /// Return the expected custody generation.
    #[must_use]
    pub const fn expected_custody_generation(&self) -> &ReconciliationCustodyGenerationV1 {
        &self.expected_custody_generation
    }
    /// Return the expected prior result revision.
    #[must_use]
    pub const fn expected_case_revision(&self) -> &ReconciliationCaseRevisionV1 {
        &self.expected_case_revision
    }
    /// Borrow the logical action identity.
    #[must_use]
    pub const fn action_id(&self) -> &ActionIdV1 {
        &self.action_id
    }
    /// Borrow the original effect-admission reference.
    #[must_use]
    pub const fn effect_admission_ref(&self) -> &ResidentRecordRefV1 {
        &self.effect_admission_ref
    }
    /// Borrow the original effect-admission digest.
    #[must_use]
    pub const fn effect_admission_sha256(&self) -> &EffectAdmissionDigest {
        &self.effect_admission_sha256
    }
    /// Borrow the exact considered receipt set.
    #[must_use]
    pub fn effect_receipts(&self) -> &[ReconciliationEffectReceiptRefV1] {
        &self.effect_receipts
    }
    /// Borrow the expected typed state.
    #[must_use]
    pub const fn expected_state_digest(&self) -> &TypedDigestV1 {
        &self.expected_state_digest
    }
    /// Borrow the reconciliation-policy reference.
    #[must_use]
    pub const fn reconciliation_policy_ref(&self) -> &ResidentRecordRefV1 {
        &self.reconciliation_policy_ref
    }
    /// Borrow the reconciliation-policy digest.
    #[must_use]
    pub const fn reconciliation_policy_sha256(&self) -> &ArtifactDigest {
        &self.reconciliation_policy_sha256
    }
    /// Return the original request time.
    #[must_use]
    pub const fn requested_at(&self) -> &UnixTimeV1 {
        &self.requested_at
    }
}

/// Prove that `current` is either the predecessor result's custody or a
/// complete, gap-free successor chain from it.
fn custody_descends_from_result(
    predecessor: &ReconciliationResultV1,
    intervening: &[ReconciliationCustodyLineageMemberV1<'_>],
    current_ref: &ResidentRecordRefV1,
    current: &ReconciliationCustodyV1,
) -> bool {
    if intervening.is_empty()
        && predecessor.custody_ref() == current_ref
        && predecessor.custody_sha256() == current.canonical_custody_sha256()
        && predecessor.custody_generation() == current.custody_generation()
    {
        return true;
    }

    let mut prior_ref = predecessor.custody_ref();
    let mut prior_sha256 = predecessor.custody_sha256();
    let mut prior_generation = *predecessor.custody_generation();
    for member in
        intervening
            .iter()
            .copied()
            .chain(std::iter::once(ReconciliationCustodyLineageMemberV1 {
                custody_ref: current_ref,
                custody: current,
            }))
    {
        let custody = member.custody;
        if custody.previous_custody_ref().as_ref() != Some(prior_ref)
            || custody.previous_custody_sha256().as_ref() != Some(prior_sha256)
            || prior_generation
                .checked_successor()
                .is_none_or(|next| next != *custody.custody_generation())
            || custody.case_ref() != predecessor.case_ref()
            || custody.case_sha256() != predecessor.case_sha256()
            || custody.custody_state() == &ReconciliationCustodyStateV1::Closed
        {
            return false;
        }
        prior_ref = member.custody_ref;
        prior_sha256 = custody.canonical_custody_sha256();
        prior_generation = *custody.custody_generation();
    }
    true
}

fn validate_reconciliation_request(
    value: &ReconciliationRequestPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    validate_typed_digest(&value.expected_state_digest)?;
    validate_bounded_json(
        "reconciliation observation bounds",
        &value.observation_bounds,
    )?;
    strictly_sorted(
        "reconciliation request effect receipts",
        &value.effect_receipts,
    )?;
    if value.effect_receipts.is_empty()
        || value.predecessor_result_ref.is_some() != value.predecessor_result_sha256.is_some()
        || (value.expected_case_revision.get() == 0) != value.predecessor_result_ref.is_none()
    {
        Err(InvalidResidentRecordV1::SettlementState)
    } else {
        Ok(())
    }
}

/// Terminal or unresolved reconciliation observation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationCaseStateV1 {
    /// Observed native state matches the expected state.
    Matched,
    /// Independent absence evidence proves the effect was not applied.
    NotApplied,
    /// Observed native state differs from the expected state.
    Drift,
    /// Bounded observation cannot establish effect presence or absence.
    Unknown,
}

/// Optional resolution attached according to the observed case state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationResolutionV1 {
    /// Native evidence confirms the intended effect.
    EffectConfirmed,
    /// Native evidence confirms that the effect is absent.
    EffectAbsent,
    /// A separately authorized compensation action completed.
    Compensated,
    /// An authorized policy accepted the provider's observed state.
    ProviderStateAccepted,
    /// A separately authorized action restored the expected state.
    ExpectedStateRestored,
    /// A human or other explicitly selected reviewer must decide.
    ManualReviewRequired,
    /// The bounded terminal observation remained inconclusive.
    StillUnknown,
}

/// One immutable observation-evidence coordinate.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationObservationEvidenceV1 {
    /// Evidence reference.
    pub evidence_ref: ResidentRecordRefV1,
    /// Digest of the referenced evidence bytes.
    pub evidence_sha256: ArtifactDigest,
}

/// Exact separately admitted compensation lineage for one terminal result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationCompensationBindingV1 {
    /// Caller-signed compensation request.
    pub compensation_request_ref: ResidentRecordRefV1,
    /// Digest of the exact caller-signed compensation request.
    pub compensation_request_sha256: CommonRequestEnvelopeDigest,
    /// Separately admitted compensation action body.
    pub compensation_action_ref: ResidentRecordRefV1,
    /// Digest of the exact compensation action body.
    pub compensation_action_sha256: ActionEnvelopeDigest,
    /// Admitted terminal compensation effect result.
    pub compensation_result_ref: ResidentRecordRefV1,
    /// Digest of the exact compensation result receipt.
    pub compensation_result_sha256: EffectReceiptDigest,
    /// Exact ambiguity case authorizing compensation.
    pub case_ref: ResidentRecordRefV1,
    /// Digest of the exact ambiguity case.
    pub case_sha256: ReconciliationCaseDigest,
    /// Exact custody generation authorizing compensation.
    pub custody_ref: ResidentRecordRefV1,
    /// Digest of the exact custody generation.
    pub custody_sha256: ReconciliationCustodyDigest,
    /// Original effect admission whose ambiguity is compensated.
    pub effect_admission_ref: ResidentRecordRefV1,
    /// Digest of the original effect admission.
    pub effect_admission_sha256: EffectAdmissionDigest,
    /// Exact original effect receipts considered by compensation.
    pub effect_receipts: Vec<ReconciliationEffectReceiptRefV1>,
}

/// Exact separately admitted records resolved for a compensated result.
pub struct ResolvedReconciliationCompensationV1<'a> {
    /// Caller-signed request authorizing the compensation action.
    pub request_ref: &'a ResidentRecordRefV1,
    /// Admitted caller-signed request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Complete owner-resolved request binding expected in the result receipt.
    pub request_binding: &'a RequestBindingV1,
    /// Unsigned action body carried by `request`.
    pub action_ref: &'a ResidentRecordRefV1,
    /// Admitted compensation action body.
    pub action: &'a ActionEnvelopeV1,
    /// Terminal receipt proving the compensation effect.
    pub result_ref: &'a ResidentRecordRefV1,
    /// Admitted compensation result receipt.
    pub result: &'a EffectReceiptV1,
}

resident_signed_context_record!(
    /// Immutable result of one bounded read-only reconciliation request.
    ReconciliationResultV1, ReconciliationResultWireV1, ReconciliationResultPreimageV1,
    canonical_reconciliation_sha256: ReconciliationResultDigest,
    RECONCILIATION_RESULT_V1_SCHEMA, "reconciliation result",
    {
        /// Immutable common-request admission lineage of the reconciliation request.
        request_binding: RequestBindingV1,
        /// Stable identity of the reconciliation request and result chain.
        reconciliation_id: ReconciliationIdV1,
        /// Immutable ambiguity case being reconciled.
        case_ref: ResidentRecordRefV1,
        /// Digest of the exact ambiguity-case bytes.
        case_sha256: ReconciliationCaseDigest,
        /// Exclusive custody generation authorizing this result attempt.
        custody_ref: ResidentRecordRefV1,
        /// Digest of the exact custody-record bytes.
        custody_sha256: ReconciliationCustodyDigest,
        /// Positive custody generation checked by compare-and-swap.
        custody_generation: ReconciliationCustodyGenerationV1,
        /// Positive result revision committed to the case chain.
        case_revision: ReconciliationCaseRevisionV1,
        /// Prior result revision, absent only for revision one.
        predecessor_result_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::predecessor_result_ref`].
        predecessor_result_sha256: Option<ReconciliationResultDigest>,
        /// Logical action whose outcome is reconciled.
        action_id: ActionIdV1,
        /// Immutable effect admission governing the action.
        effect_admission_ref: ResidentRecordRefV1,
        /// Digest of the exact effect-admission bytes.
        effect_admission_sha256: EffectAdmissionDigest,
        /// Canonically ordered effect receipts considered by this revision.
        effect_receipts: Vec<ReconciliationEffectReceiptRefV1>,
        /// Principal scope within which native state was observed.
        principal_scope: ResidentPrincipalIdV1,
        /// Controller scope within which native state was observed.
        controller_scope: ResidentControllerIdV1,
        /// Optional relationship boundary carried from the admitted request.
        relationship_scope: Option<RelationshipScopeV1>,
        /// Immutable reconciliation policy applied by this revision.
        reconciliation_policy_ref: ResidentRecordRefV1,
        /// Digest of the exact reconciliation-policy bytes.
        reconciliation_policy_sha256: ArtifactDigest,
        /// Typed expected-state digest fixed by the case.
        expected_state_digest: TypedDigestV1,
        /// Typed native state actually observed, when observation establishes one.
        observed_state_digest: Option<TypedDigestV1>,
        /// Canonically ordered bounded evidence supporting native observation.
        observation_evidence: Vec<ReconciliationObservationEvidenceV1>,
        /// Independent evidence proving effect absence for a not-applied result.
        absence_evidence_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::absence_evidence_ref`].
        absence_evidence_sha256: Option<ArtifactDigest>,
        /// Canonically ordered effect attempts known to this reconciliation revision.
        known_attempt_ids: Vec<CapabilityAttemptIdV1>,
        /// Native-state classification supported by the recorded evidence.
        case_state: ReconciliationCaseStateV1,
        /// Optional terminal or routing disposition permitted by [`Self::case_state`].
        resolution: Option<ReconciliationResolutionV1>,
        /// Separately admitted compensation request/action/result lineage.
        compensation: Option<ReconciliationCompensationBindingV1>,
        /// Immutable owner evidence required by every terminal drift resolution.
        owner_evidence_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::owner_evidence_ref`].
        owner_evidence_sha256: Option<ArtifactDigest>,
        /// Immutable reviewer or policy governing this result disposition.
        reviewer_or_policy_ref: ResidentRecordRefV1,
        /// Digest of the reviewer-or-policy bytes.
        reviewer_or_policy_sha256: ArtifactDigest,
        /// Original reconciliation-request time in Unix seconds.
        requested_at: UnixTimeV1,
        /// Time at which this result revision was recorded.
        recorded_at: UnixTimeV1,
        /// Terminal close time; absent for every open result.
        closed_at: Option<UnixTimeV1>,
        /// Exact deployment-runtime authority under which this record was issued.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Principal under whose authority this result was issued.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Key identity used for the result signature.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch checked for the result.
        issuer_key_epoch: SigningKeyEpochV1
    },
    wire {
        request_binding: RequestBindingV1,
        reconciliation_id: ReconciliationIdV1,
        case_ref: ResidentRecordRefV1,
        case_sha256: ReconciliationCaseDigest,
        custody_ref: ResidentRecordRefV1,
        custody_sha256: ReconciliationCustodyDigest,
        custody_generation: ReconciliationCustodyGenerationV1,
        case_revision: ReconciliationCaseRevisionV1,
        predecessor_result_ref: Option<ResidentRecordRefV1>,
        predecessor_result_sha256: Option<ReconciliationResultDigest>,
        action_id: ActionIdV1,
        effect_admission_ref: ResidentRecordRefV1,
        effect_admission_sha256: EffectAdmissionDigest,
        effect_receipts: Vec<ReconciliationEffectReceiptRefV1>,
        principal_scope: ResidentPrincipalIdV1,
        controller_scope: ResidentControllerIdV1,
        relationship_scope: Option<RelationshipScopeV1>,
        reconciliation_policy_ref: ResidentRecordRefV1,
        reconciliation_policy_sha256: ArtifactDigest,
        expected_state_digest: TypedDigestWireV1,
        observed_state_digest: Option<TypedDigestWireV1>,
        observation_evidence: Vec<ReconciliationObservationEvidenceV1>,
        absence_evidence_ref: Option<ResidentRecordRefV1>,
        absence_evidence_sha256: Option<ArtifactDigest>,
        known_attempt_ids: Vec<CapabilityAttemptIdV1>,
        case_state: ReconciliationCaseStateV1,
        resolution: Option<ReconciliationResolutionV1>,
        compensation: Option<ReconciliationCompensationBindingV1>,
        owner_evidence_ref: Option<ResidentRecordRefV1>,
        owner_evidence_sha256: Option<ArtifactDigest>,
        reviewer_or_policy_ref: ResidentRecordRefV1,
        reviewer_or_policy_sha256: ArtifactDigest,
        requested_at: UnixTimeV1,
        recorded_at: UnixTimeV1,
        closed_at: Option<UnixTimeV1>,
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        deployment_runtime_authority_sha256: ArtifactDigest,
        issuer_principal_id: ResidentPrincipalIdV1,
        issuer_key_id: ResidentKeyIdV1,
        issuer_key_epoch: SigningKeyEpochV1
    },
    |value: &ReconciliationResultPreimageV1| {
        value.request_binding.verify()?;
        validate_typed_digest(&value.expected_state_digest)?;
        if let Some(observed) = &value.observed_state_digest {
            validate_typed_digest(observed)?;
        }
        strictly_sorted("reconciliation effect receipts", &value.effect_receipts)?;
        strictly_sorted("reconciliation observation evidence", &value.observation_evidence)?;
        strictly_sorted("reconciliation known attempts", &value.known_attempt_ids)?;
        if value.requested_at > value.recorded_at
            || value.predecessor_result_ref.is_some() != value.predecessor_result_sha256.is_some()
            || (value.case_revision.get() == 1) != value.predecessor_result_ref.is_none()
            || value.absence_evidence_ref.is_some() != value.absence_evidence_sha256.is_some()
            || value.owner_evidence_ref.is_some() != value.owner_evidence_sha256.is_some()
        { return Err(InvalidResidentRecordV1::SettlementState); }
        let closed = value.closed_at.is_some();
        if value.closed_at.is_some_and(|closed_at| closed_at < value.recorded_at) {
            return Err(InvalidResidentRecordV1::SettlementState);
        }
        let state_ok = match (value.case_state, value.resolution) {
            (ReconciliationCaseStateV1::Matched, Some(ReconciliationResolutionV1::EffectConfirmed)) => {
                value.observed_state_digest.as_ref() == Some(&value.expected_state_digest)
                    && value.absence_evidence_ref.is_none() && closed
            }
            (ReconciliationCaseStateV1::NotApplied, Some(ReconciliationResolutionV1::EffectAbsent)) => {
                value.absence_evidence_ref.is_some() && closed
            }
            (ReconciliationCaseStateV1::Drift, None) => {
                value.observed_state_digest.is_some() && !closed
            }
            (ReconciliationCaseStateV1::Drift, Some(
                ReconciliationResolutionV1::Compensated
                | ReconciliationResolutionV1::ProviderStateAccepted
                | ReconciliationResolutionV1::ExpectedStateRestored,
            )) => value.observed_state_digest.is_some()
                && value.owner_evidence_ref.is_some()
                && closed,
            (ReconciliationCaseStateV1::Unknown, None | Some(ReconciliationResolutionV1::ManualReviewRequired)) => {
                value.observed_state_digest.is_none()
                    && value.absence_evidence_ref.is_none()
                    && !value.observation_evidence.is_empty()
                    && !closed
            }
            (ReconciliationCaseStateV1::Unknown, Some(ReconciliationResolutionV1::StillUnknown)) => {
                value.observed_state_digest.is_none()
                    && value.absence_evidence_ref.is_none()
                    && !value.observation_evidence.is_empty()
                    && closed
            }
            _ => false,
        };
        let compensation_ok = matches!(value.resolution, Some(ReconciliationResolutionV1::Compensated))
            == value.compensation.is_some();
        let owner_evidence_ok = matches!(
            (value.case_state, value.resolution),
            (
                ReconciliationCaseStateV1::Drift,
                Some(
                    ReconciliationResolutionV1::Compensated
                        | ReconciliationResolutionV1::ProviderStateAccepted
                        | ReconciliationResolutionV1::ExpectedStateRestored
                )
            )
        ) == value.owner_evidence_ref.is_some();
        if state_ok && compensation_ok && owner_evidence_ok { Ok(()) } else { Err(InvalidResidentRecordV1::SettlementState) }
    }
);
