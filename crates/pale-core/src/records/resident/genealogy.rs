//! Deterministic interaction placement, truth-classed lineage, and frontiers.
//!
//! The three [`GenealogyProofV1`] variants are deliberately non-substitutable:
//! native structure requires native-owner evidence, Pale consequence lineage
//! requires both admission and destination receipt, and semantic inference
//! requires Scope-owned evidence with explicit confidence and uncertainty.
//! Reconciliation records expose independent progress dimensions; they never
//! settle or mutate a native owner.

use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for deterministic interaction occurrences.
pub const INTERACTION_OCCURRENCE_V1_SCHEMA_ID: &str = "interaction-occurrence.v1";
/// Wire schema carried by every interaction occurrence.
pub const INTERACTION_OCCURRENCE_V1_SCHEMA: &str = "pale/interaction-occurrence.v1";
/// Signature domain for [`InteractionOccurrenceV1`].
pub const INTERACTION_OCCURRENCE_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_INTERACTION_OCCURRENCE_V1\0";

/// Public schema root for truth-classed genealogy edges.
pub const GENEALOGY_EDGE_V1_SCHEMA_ID: &str = "genealogy-edge.v1";
/// Wire schema carried by every genealogy edge.
pub const GENEALOGY_EDGE_V1_SCHEMA: &str = "pale/genealogy-edge.v1";
/// Signature domain for [`GenealogyEdgeV1`].
pub const GENEALOGY_EDGE_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_GENEALOGY_EDGE_V1\0";

/// Public schema root for reconciliation frontiers.
pub const RECONCILIATION_FRONTIER_V1_SCHEMA_ID: &str = "reconciliation-frontier.v1";
/// Wire schema carried by every reconciliation frontier.
pub const RECONCILIATION_FRONTIER_V1_SCHEMA: &str = "pale/reconciliation-frontier.v1";
/// Signature domain for [`ReconciliationFrontierV1`].
pub const RECONCILIATION_FRONTIER_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_RECONCILIATION_FRONTIER_V1\0";

/// Exact immutable owner-record coordinate used without copying owner state.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct OwnerRecordCoordinateV1 {
    /// Native system that owns the record.
    pub owner_system_id: OwnerSystemIdV1,
    /// Immutable owner-native record reference.
    pub record_ref: ResidentRecordRefV1,
    /// Exact owner-published record schema identity.
    pub record_schema_id: SchemaId,
    /// Digest of the exact owner-published schema bytes.
    pub record_schema_sha256: SchemaDigest,
    /// Digest of the exact immutable owner-record bytes.
    pub record_content_sha256: ArtifactDigest,
    /// Owner-scoped revision observed with the record.
    pub native_revision: NativeRevisionV1,
}

/// Digest-bound immutable record reference without an authority interpretation.
///
/// This shape intentionally says only which bytes are referenced. It must not
/// be used as evidence that those bytes authorize, admit, or settle anything.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ImmutableRecordReferenceV1 {
    /// Immutable record reference.
    pub record_ref: ResidentRecordRefV1,
    /// Digest of the exact referenced bytes.
    pub record_sha256: ArtifactDigest,
}

/// Explicit knowledge state for one owner-native record coordinate.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerRecordKnowledgeV1 {
    /// The source contract does not expose whether the record exists.
    Unknown,
    /// The source owner explicitly proves the record does not exist.
    Absent,
    /// The source owner exposes one exact immutable record.
    Known {
        /// Exact owner-native placement record.
        coordinate: OwnerRecordCoordinateV1,
    },
}

/// Explicit expectation state; unknown is not equivalent to no response.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum InteractionResponseExpectationV1 {
    /// The source owner does not expose response expectation.
    Unknown,
    /// The source owner explicitly carries no response expectation.
    None,
    /// The source owner exposes one bounded expectation class.
    Expected {
        /// Owner-defined expectation class.
        expectation: ResponseExpectationV1,
    },
}

/// Deterministic source-owner lifecycle projection for an interaction.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum InteractionLifecycleProjectionV1 {
    /// The source contract exposes no lifecycle state.
    Unknown,
    /// The exact source event remains current at the recorded revision.
    Current,
    /// An owner record proves the source was edited.
    Edited {
        /// Exact owner-native edit record.
        evidence: OwnerRecordCoordinateV1,
    },
    /// An owner record proves the source was deleted or tombstoned.
    Deleted {
        /// Exact owner-native deletion/tombstone record.
        evidence: OwnerRecordCoordinateV1,
    },
    /// An owner record proves the source was superseded.
    Superseded {
        /// Exact owner-native successor record.
        evidence: OwnerRecordCoordinateV1,
    },
}

resident_signed_record!(
    /// Deterministic placement of one admitted event in owner-native context.
    ///
    /// Unknown and absent placement are distinct. Text, UI labels, and graph
    /// proximity cannot synthesize a reply, participant, approval, or thread.
    InteractionOccurrenceV1, InteractionOccurrenceWireV1, InteractionOccurrencePreimageV1,
    canonical_occurrence_sha256: InteractionOccurrenceDigest,
    INTERACTION_OCCURRENCE_V1_SCHEMA, "interaction occurrence",
    {
        /// Stable occurrence identity.
        occurrence_id: InteractionOccurrenceIdV1,
        /// Immutable admitted event-reference record.
        event_reference_ref: ResidentRecordRefV1,
        /// Canonical digest of the event reference.
        event_reference_sha256: EventReferenceDigest,
        /// Native container exposed by the source owner.
        native_container: OwnerRecordCoordinateV1,
        /// Owner-defined container genre.
        container_genre: NativeContainerGenreV1,
        /// Explicit root-placement state.
        root_placement: OwnerRecordKnowledgeV1,
        /// Explicit thread-placement state.
        thread_placement: OwnerRecordKnowledgeV1,
        /// Explicit parent-placement state.
        parent_placement: OwnerRecordKnowledgeV1,
        /// Canonically ordered quoted or referenced owner records.
        referenced_records: Vec<OwnerRecordCoordinateV1>,
        /// Exact actor-provenance record.
        actor_provenance: OwnerRecordCoordinateV1,
        /// Participant-snapshot knowledge state exposed by the source owner.
        participant_snapshot: OwnerRecordKnowledgeV1,
        /// Audience-snapshot knowledge state exposed by the source owner.
        audience_snapshot: OwnerRecordKnowledgeV1,
        /// Source-owner visibility class.
        visibility: InteractionVisibilityV1,
        /// Admitted disclosure ceiling.
        disclosure_ceiling: InteractionDisclosureCeilingV1,
        /// Canonically ordered mention/CC owner records.
        mention_and_cc_records: Vec<OwnerRecordCoordinateV1>,
        /// Explicit response-expectation knowledge state.
        response_expectation: InteractionResponseExpectationV1,
        /// Exact edit/deletion/supersession projection.
        lifecycle: InteractionLifecycleProjectionV1,
        /// Source-owner data classification.
        classification: DataClassificationV1,
        /// Exact payload-resolution policy.
        payload_resolution_policy: PayloadResolutionPolicyV1,
        /// Unix projection time.
        projected_at: UnixTimeV1,
        /// Exact deployment-runtime authority that signed this occurrence.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime-authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Pale principal whose admitted runtime issued the projection.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Exact issuer key.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_interaction_occurrence_preimage
);

fn validate_interaction_occurrence_preimage(
    value: &InteractionOccurrencePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if value.schema_id.as_str() != INTERACTION_OCCURRENCE_V1_SCHEMA {
        return Err(InvalidResidentRecordV1::SchemaCoordinate {
            family: "interaction occurrence",
        });
    }
    strictly_sorted("interaction referenced records", &value.referenced_records)?;
    strictly_sorted(
        "interaction mention and CC records",
        &value.mention_and_cc_records,
    )
}

/// Proof whose variant fixes one non-interchangeable genealogy truth class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "truth_class", rename_all = "snake_case", deny_unknown_fields)]
pub enum GenealogyProofV1 {
    /// Structural relation copied from exact native-owner metadata.
    NativeStructural {
        /// Relation token interpreted within this truth class only.
        relation: GenealogyRelationV1,
        /// Exact native-owner evidence for the relation.
        native_owner_evidence: OwnerRecordCoordinateV1,
    },
    /// Consequence admitted by Pale and committed by the destination owner.
    PaleConsequenceLineage {
        /// Relation token interpreted within this truth class only.
        relation: GenealogyRelationV1,
        /// Immutable Pale admission/consequence reference.
        admission_ref: ResidentRecordRefV1,
        /// Digest of the exact admission/consequence bytes.
        admission_sha256: ArtifactDigest,
        /// Immutable destination-owner receipt reference.
        destination_owner_receipt_ref: ResidentRecordRefV1,
        /// Digest of the exact destination-owner receipt bytes.
        destination_owner_receipt_sha256: ArtifactDigest,
    },
    /// Semantic relation inferred from Scope context with bounded evidence and uncertainty.
    SemanticInferred {
        /// Relation token interpreted within this truth class only.
        relation: GenealogyRelationV1,
        /// Immutable Scope semantic transaction reference.
        scope_transaction_ref: ResidentRecordRefV1,
        /// Digest of the exact Scope transaction bytes.
        scope_transaction_sha256: ArtifactDigest,
        /// Canonical root of the evidence set used by Scope.
        evidence_root_sha256: ArtifactDigest,
        /// Exact semantic profile reference.
        profile_ref: ResidentRecordRefV1,
        /// Digest of the exact semantic-profile bytes.
        profile_sha256: ArtifactDigest,
        /// Exact occupant/model-or-human judgment reference.
        occupant_ref: ResidentRecordRefV1,
        /// Digest of the exact occupant/judgment bytes.
        occupant_sha256: ArtifactDigest,
        /// Confidence in basis points, at most 10,000.
        confidence_basis_points: GenealogyConfidenceBasisPointsV1,
        /// Explicit uncertainty class; it cannot be inferred from confidence.
        uncertainty: SemanticUncertaintyV1,
    },
}

resident_signed_record!(
    /// One directed, truth-classed relation between admitted occurrences.
    GenealogyEdgeV1, GenealogyEdgeWireV1, GenealogyEdgePreimageV1,
    canonical_genealogy_edge_sha256: GenealogyEdgeDigest,
    GENEALOGY_EDGE_V1_SCHEMA, "genealogy edge",
    {
        /// Stable edge identity.
        edge_id: GenealogyEdgeIdV1,
        /// Immutable source occurrence.
        source_occurrence_ref: ResidentRecordRefV1,
        /// Canonical digest of the source occurrence.
        source_occurrence_sha256: InteractionOccurrenceDigest,
        /// Immutable destination occurrence.
        destination_occurrence_ref: ResidentRecordRefV1,
        /// Canonical digest of the destination occurrence.
        destination_occurrence_sha256: InteractionOccurrenceDigest,
        /// Exact non-interchangeable truth proof.
        proof: GenealogyProofV1,
        /// Unix admission time.
        admitted_at: UnixTimeV1,
        /// Exact deployment-runtime authority that signed this edge.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime-authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Pale principal whose admitted runtime issued the edge.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Exact issuer key.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_genealogy_edge_preimage
);

fn validate_genealogy_edge_preimage(
    value: &GenealogyEdgePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if value.schema_id.as_str() != GENEALOGY_EDGE_V1_SCHEMA {
        return Err(InvalidResidentRecordV1::SchemaCoordinate {
            family: "genealogy edge",
        });
    }
    if value.source_occurrence_ref == value.destination_occurrence_ref {
        return Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "genealogy edge",
            invariant: "distinct directed occurrence endpoints",
        });
    }
    if let GenealogyProofV1::SemanticInferred {
        confidence_basis_points,
        ..
    } = &value.proof
    {
        if confidence_basis_points.get() > 10_000 {
            return Err(InvalidResidentRecordV1::InvalidCoordinate {
                coordinate: "genealogy confidence basis points",
            });
        }
    }
    Ok(())
}

/// Subject class tracked by one reconciliation frontier.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReconciliationSubjectV1 {
    /// One native source publication.
    Source {
        /// Exact source record.
        coordinate: OwnerRecordCoordinateV1,
    },
    /// One consumer's independent delivery/admission state.
    Consumer {
        /// Stable consumer identity.
        consumer_id: OwnerSystemIdV1,
        /// Exact immutable consumer obligation or state reference.
        state_ref: ResidentRecordRefV1,
        /// Digest of the exact referenced bytes.
        state_sha256: ArtifactDigest,
    },
    /// One deterministic or semantic projection.
    Projection {
        /// Exact immutable projection reference.
        projection_ref: ResidentRecordRefV1,
        /// Digest of the exact projection bytes.
        projection_sha256: ArtifactDigest,
    },
    /// One independently owned consequence.
    Consequence {
        /// Native owner of the consequence.
        owner_system_id: OwnerSystemIdV1,
        /// Exact immutable consequence reference.
        consequence_ref: ResidentRecordRefV1,
        /// Digest of the exact consequence bytes.
        consequence_sha256: ArtifactDigest,
    },
}

/// Independent dimensions that a frontier may expose.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationDimensionV1 {
    /// Owner publication and source-gap state.
    OwnerPublication,
    /// Pale ingress/journal position.
    PaleIngress,
    /// Subscription or inbox delivery.
    Delivery,
    /// Deterministic occurrence/structural projection.
    DeterministicProjection,
    /// Scope context or semantic mapping.
    SemanticMapping,
    /// Required judgment.
    Judgment,
    /// Required articulation or response.
    Articulation,
    /// Owner-native consequence/admission/effect/outcome.
    NativeConsequence,
    /// Deletion, retention, expiry, or tombstone reconciliation.
    Retention,
}

/// State of one independent reconciliation dimension.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReconciliationDimensionStatusV1 {
    /// This dimension is complete at the bound evidence frontier.
    Complete,
    /// Required work remains pending.
    Pending,
    /// A source or delivery gap is known.
    Gap,
    /// Policy or authority blocks progress.
    Blocked,
    /// Evidence is retained for adjudication.
    Quarantined,
    /// The responsible component or owner is unavailable.
    Unavailable,
    /// The obligation expired without being silently completed.
    Expired,
    /// A native owner published its own terminal disposition.
    OwnerTerminal {
        /// Opaque owner-defined terminal disposition.
        disposition: OwnerFrontierDispositionV1,
    },
}

/// One independently typed reconciliation-vector entry.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationDimensionEntryV1 {
    /// Independent progress dimension.
    pub dimension: ReconciliationDimensionV1,
    /// Current status of only that dimension.
    pub status: ReconciliationDimensionStatusV1,
    /// Optional immutable evidence supporting the status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<AuthorityEvidenceV1>,
}

/// One owner-record revision retained at a reconciliation frontier.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct FrontierOwnerRevisionV1 {
    /// Native owner whose revision was observed.
    pub owner_system_id: OwnerSystemIdV1,
    /// Exact immutable owner record at the frontier.
    pub record_ref: ResidentRecordRefV1,
    /// Digest of the exact owner-record bytes.
    pub record_sha256: ArtifactDigest,
    /// Owner-scoped revision token.
    pub native_revision: NativeRevisionV1,
}

resident_signed_record!(
    /// Independent reconciliation vector for one source, consumer, projection, or consequence.
    ///
    /// In the law below, a frontier is a finite partial function from one typed
    /// dimension `D` to a status `S` paired with optional immutable evidence.
    /// No scalar reduction of `F` is stored, so one complete dimension
    /// cannot hide another pending one.
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/reconciliation-frontier.display.mathml"
    ))]
    ReconciliationFrontierV1, ReconciliationFrontierWireV1, ReconciliationFrontierPreimageV1,
    canonical_reconciliation_frontier_sha256: ReconciliationFrontierDigest,
    RECONCILIATION_FRONTIER_V1_SCHEMA, "reconciliation frontier",
    {
        /// Stable frontier identity.
        frontier_id: ReconciliationFrontierIdV1,
        /// Exact subject whose progress is represented.
        subject: ReconciliationSubjectV1,
        /// Canonically ordered independent dimension vector.
        dimensions: Vec<ReconciliationDimensionEntryV1>,
        /// Canonically ordered source occurrences contributing to this frontier.
        source_occurrence_refs: Vec<ImmutableRecordReferenceV1>,
        /// Canonically ordered owner revisions observed at this frontier.
        owner_record_revisions: Vec<FrontierOwnerRevisionV1>,
        /// Unix construction time.
        generated_at: UnixTimeV1,
        /// Exact deployment-runtime authority that signed this frontier.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime-authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Pale principal whose admitted runtime issued the frontier.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Exact issuer key.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive issuer-key epoch.
        issuer_key_epoch: SigningKeyEpochV1
    },
    validate_reconciliation_frontier_preimage
);

fn validate_reconciliation_frontier_preimage(
    value: &ReconciliationFrontierPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if value.schema_id.as_str() != RECONCILIATION_FRONTIER_V1_SCHEMA {
        return Err(InvalidResidentRecordV1::SchemaCoordinate {
            family: "reconciliation frontier",
        });
    }
    strictly_sorted("reconciliation dimensions", &value.dimensions)?;
    if value
        .dimensions
        .windows(2)
        .any(|pair| pair[0].dimension == pair[1].dimension)
    {
        return Err(InvalidResidentRecordV1::NonCanonical {
            relation: "reconciliation dimensions",
        });
    }
    strictly_sorted(
        "reconciliation source occurrences",
        &value.source_occurrence_refs,
    )?;
    strictly_sorted(
        "reconciliation owner revisions",
        &value.owner_record_revisions,
    )
}

macro_rules! impl_genealogy_signed_admission {
    ($record:ident, $wire:ident, $preimage:ident, $digest:ident, $domain:ident, $family:literal, $validate:ident) => {
        impl $record {
            /// Issue one record after exact dependency and signer admission.
            ///
            /// # Errors
            /// Returns [`InvalidResidentRecordV1`] for invalid structure,
            /// authority coordinates, dependencies, or signing failure.
            pub fn issue_contextual<D, F>(
                preimage: $preimage,
                context: &SignedResidentAdmissionContextV1<'_>,
                admit_dependencies: D,
                sign: F,
            ) -> Result<Self, InvalidResidentRecordV1>
            where
                D: FnOnce(&$preimage) -> Result<(), InvalidResidentRecordV1>,
                F: FnOnce(
                    &[u8],
                    &$preimage,
                    &$digest,
                ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
            {
                $validate(&preimage)?;
                validate_signed_coordinates(
                    $family,
                    &preimage.schema_sha256,
                    context.expected_schema_sha256,
                    &preimage.issuer_principal_id,
                    &preimage.issuer_key_id,
                    preimage.issuer_key_epoch,
                    context.signing_authority,
                )?;
                validate_integrated_runtime_authority_coordinates(
                    $family,
                    &preimage.deployment_runtime_authority_ref,
                    &preimage.deployment_runtime_authority_sha256,
                    context.signing_authority,
                )?;
                admit_dependencies(&preimage)?;
                let digest = <$digest>::hash_canonical(&preimage)?;
                let signature = sign($domain, &preimage, &digest)?;
                Self::seal(preimage, signature)
            }

            /// Admit a wire against the exact selected schema and runtime signer.
            ///
            /// # Errors
            /// Returns [`InvalidResidentRecordV1`] for structural, digest,
            /// authority-coordinate, or signature admission failure.
            pub fn admit_contextual_wire<D, F>(
                wire: $wire,
                context: &SignedResidentAdmissionContextV1<'_>,
                admit_dependencies: D,
                verify_signature: F,
            ) -> Result<Self, InvalidResidentRecordV1>
            where
                D: FnOnce(&$preimage) -> Result<(), InvalidResidentRecordV1>,
                F: FnOnce(
                    &[u8],
                    &$preimage,
                    &$digest,
                    &CanonicalSignatureV1,
                ) -> Result<(), InvalidResidentRecordV1>,
            {
                Self::admit_wire(wire, |preimage, digest, signature| {
                    validate_signed_coordinates(
                        $family,
                        &preimage.schema_sha256,
                        context.expected_schema_sha256,
                        &preimage.issuer_principal_id,
                        &preimage.issuer_key_id,
                        preimage.issuer_key_epoch,
                        context.signing_authority,
                    )?;
                    validate_integrated_runtime_authority_coordinates(
                        $family,
                        &preimage.deployment_runtime_authority_ref,
                        &preimage.deployment_runtime_authority_sha256,
                        context.signing_authority,
                    )?;
                    admit_dependencies(preimage)?;
                    verify_signature($domain, preimage, digest, signature)
                })
            }
        }
    };
}

impl_genealogy_signed_admission!(
    InteractionOccurrenceV1,
    InteractionOccurrenceWireV1,
    InteractionOccurrencePreimageV1,
    InteractionOccurrenceDigest,
    INTERACTION_OCCURRENCE_V1_SIGNATURE_DOMAIN,
    "interaction occurrence",
    validate_interaction_occurrence_preimage
);
impl_genealogy_signed_admission!(
    GenealogyEdgeV1,
    GenealogyEdgeWireV1,
    GenealogyEdgePreimageV1,
    GenealogyEdgeDigest,
    GENEALOGY_EDGE_V1_SIGNATURE_DOMAIN,
    "genealogy edge",
    validate_genealogy_edge_preimage
);
impl_genealogy_signed_admission!(
    ReconciliationFrontierV1,
    ReconciliationFrontierWireV1,
    ReconciliationFrontierPreimageV1,
    ReconciliationFrontierDigest,
    RECONCILIATION_FRONTIER_V1_SIGNATURE_DOMAIN,
    "reconciliation frontier",
    validate_reconciliation_frontier_preimage
);
