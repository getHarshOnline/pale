//! Closed custom LSP methods and lossless editor projections.
//!
//! This module contains only transport shapes. Compilation, comparison,
//! reachability, proof selection, and identity validation remain owned by
//! [`pale_sdk`]. Each projection is constructed from an admitted API value;
//! no JSON field is interpreted as a second semantic model.

use lsp_types::request::{
    CodeActionRequest, Completion, DocumentSymbolRequest, GotoDefinition, HoverRequest, References,
    Request as _, SemanticTokensFullRequest,
};
use lsp_types::{
    CodeActionParams, CodeActionResponse, CompletionParams, CompletionResponse,
    DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse,
    Hover, HoverParams, Location, ReferenceParams, SemanticTokensParams, SemanticTokensResult,
};
use pale_sdk::{
    AuthoringInstancePathSegment, AuthoringOccurrenceRole, AuthoringSourceLocation,
    DiagnosticIdentity, DiagnosticV1, EffectiveProfileCoordinates, EffectiveProfileExplanation,
    EffectiveProfileExplanationHandle, EffectiveProfilePage, EffectiveProfileSection,
    PortableWorkspaceCausalExplanation, PortableWorkspaceCausalStep, PortableWorkspaceChangeKind,
    PortableWorkspaceChangeRoot, PortableWorkspaceChangeRootPage, PortableWorkspaceDerivationRule,
    PortableWorkspaceExplanation, PortableWorkspaceExplanationFrontier,
    PortableWorkspaceExplanationState, PortableWorkspaceFact, PortableWorkspaceImpact,
    PortableWorkspaceImpactPage, PortableWorkspaceImpactState, PortableWorkspaceProvenance,
    PortableWorkspaceSubject, PortableWorkspaceSubjectInspection, PortableWorkspaceSubjectSelector,
};
use sha2::{Digest as _, Sha256};

use crate::document::DocumentRole;

/// Return the current admitted [`DocumentStore::snapshot`] compilation summary.
///
/// [`DocumentStore::snapshot`]: crate::document::DocumentStore::snapshot
pub(super) const COMPILATION_SUMMARY_METHOD: &str = "pale/workspace/compilationSummary";
/// Compare the immutable baseline with [`DocumentStore::snapshot`].
///
/// [`DocumentStore::snapshot`]: crate::document::DocumentStore::snapshot
pub(super) const COMPARISON_SUMMARY_METHOD: &str = "pale/workspace/comparisonSummary";
/// Return one bounded page of exact comparison roots.
pub(super) const CHANGE_ROOTS_METHOD: &str = "pale/workspace/changeRoots";
/// Return one bounded page of the current impact partition.
pub(super) const IMPACTS_METHOD: &str = "pale/workspace/impacts";
/// Explain one typed subject through the retained causal proof.
pub(super) const EXPLANATION_METHOD: &str = "pale/workspace/explain";
/// Enumerate exact project and execution-workspace pairs in the admitted topology.
pub(super) const PROFILE_CONTEXTS_METHOD: &str = "pale/workspace/profileContexts";
/// Inspect one bounded section of the retained effective profile.
pub(super) const EFFECTIVE_PROFILE_METHOD: &str = "pale/workspace/effectiveProfile";
/// Explain one row from the retained effective-profile compilation.
pub(super) const EFFECTIVE_PROFILE_EXPLANATION_METHOD: &str =
    "pale/workspace/effectiveProfileExplain";
/// Advance the single retained human-governed configuration review.
pub(super) const CONFIGURATION_IMPROVEMENT_METHOD: &str = "pale/workspace/configurationImprovement";
/// Stable identity of this private LSP extension protocol.
pub(super) const PROTOCOL_ID: &str = "getharshonline.pale-lsp.workspace";
/// SHA-256 of the exact cross-client wire fixture, checked against Rust output by tests.
///
/// This attests the represented exchanges, not every possible DTO shape. It
/// complements the method inventory; clients must still decode every response.
/// Only the digest is embedded, so runtime packaging needs no test fixtures.
const WIRE_FIXTURE_SHA256: &str =
    "25955a504c6470e203a673761d4a911b743ad523adaa87f5961fb249faad6e5c";
/// Notify clients after the single analysis owner accepts one current result.
pub(super) const ANALYSIS_PUBLISHED_NOTIFICATION: &str = "pale/workspace/analysisPublished";

/// All currently implemented custom methods in stable advertisement order.
const IMPLEMENTED_METHODS: [&str; 9] = [
    COMPILATION_SUMMARY_METHOD,
    COMPARISON_SUMMARY_METHOD,
    CHANGE_ROOTS_METHOD,
    IMPACTS_METHOD,
    EXPLANATION_METHOD,
    PROFILE_CONTEXTS_METHOD,
    EFFECTIVE_PROFILE_METHOD,
    EFFECTIVE_PROFILE_EXPLANATION_METHOD,
    CONFIGURATION_IMPROVEMENT_METHOD,
];

/// Closed query submitted to the single analysis owner.
#[derive(Debug)]
pub(super) enum WorkspaceQuery {
    /// Inspect the current admitted compilation.
    CompilationSummary,
    /// Inspect the immutable-baseline comparison.
    ComparisonSummary,
    /// Page exact source-change roots.
    ChangeRoots(PageParams),
    /// Page the complete current impact partition.
    Impacts(PageParams),
    /// Explain one validated semantic subject.
    Explanation(PortableWorkspaceSubject),
    /// Enumerate exact profile-selection pairs from the retained compilation.
    ProfileContexts,
    /// Inspect one API-owned profile section at an exact editor coordinate.
    EffectiveProfile(EffectiveProfileQuery),
    /// Explain one row from the sole retained API-owned profile.
    EffectiveProfileExplanation(EffectiveProfileExplanationQuery),
    /// Advance the one retained configuration-improvement typestate.
    ConfigurationImprovement(ConfigurationImprovementQuery),
    /// Standard configuration-authoring request over the current editor snapshot.
    Authoring(AuthoringQuery),
}

/// Standard LSP operation translated by the single analysis owner.
#[derive(Debug)]
pub(super) enum AuthoringQuery {
    /// Full semantic-token projection.
    SemanticTokens(SemanticTokensParams),
    /// Context-sensitive key, literal, or identity completion.
    Completion(CompletionParams),
    /// Source-bound field, value, or semantic hover.
    Hover(HoverParams),
    /// Structural and semantic document outline.
    DocumentSymbols(DocumentSymbolParams),
    /// Canonical semantic-owner navigation.
    Definition(GotoDefinitionParams),
    /// Bounded semantic occurrence navigation.
    References(ReferenceParams),
    /// Deterministic, source-bound, non-effectful edits.
    CodeActions(CodeActionParams),
}

/// Explicit, ephemeral project selection supplied by the editor user.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
pub(super) struct ProfileInspectionContext {
    /// Selected logical project identity.
    pub(super) project_id: String,
    /// Selected logical execution-workspace identity.
    pub(super) execution_workspace_id: String,
}

/// Prior-page coordinate preventing cross-generation pagination splices.
#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct ExpectedProfileCoordinateWire {
    generation: String,
    document_revision: String,
    profile_digest: String,
    instance_digest: String,
    selected_topology_digest: String,
    workspace_semantic_ir_digest: String,
    workspace_admission_evidence_digest: String,
    package_semantic_ir_digest: String,
    composition_plan_digest: String,
}

/// Validated process-local coordinate used inside the LSP worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ExpectedProfileCoordinate {
    /// Editor generation returned with the first page.
    pub(super) generation: u64,
    /// Document-store revision returned with the first page.
    pub(super) document_revision: u64,
    /// API-owned profile identity returned with the first page.
    pub(super) profile_digest: String,
    /// Private identity from [`EffectiveProfileCoordinates::instance_digest`].
    pub(super) instance_digest: String,
    /// Desired-topology selection admitted with this profile.
    pub(super) selected_topology_digest: String,
    /// API-owned workspace semantic-IR identity returned with the first page.
    pub(super) workspace_semantic_ir_digest: String,
    /// API-owned workspace-admission evidence identity returned with the first page.
    pub(super) workspace_admission_evidence_digest: String,
    /// API-owned semantic-closure identity returned with the first page.
    pub(super) package_semantic_ir_digest: String,
    /// API-owned composition-proof identity returned with the first page.
    pub(super) composition_plan_digest: String,
}

impl ExpectedProfileCoordinate {
    /// Bind one API profile product to its exact editor snapshot.
    pub(super) fn from_profile(
        editor: EditorCoordinate,
        coordinates: &EffectiveProfileCoordinates,
    ) -> Self {
        Self {
            generation: editor.generation,
            document_revision: editor.document_revision,
            profile_digest: coordinates.profile_digest.clone(),
            instance_digest: coordinates.instance_digest.clone(),
            selected_topology_digest: coordinates.selected_topology_digest.clone(),
            workspace_semantic_ir_digest: coordinates.workspace_semantic_ir_digest.clone(),
            workspace_admission_evidence_digest: coordinates
                .workspace_admission_evidence_digest
                .clone(),
            package_semantic_ir_digest: coordinates.package_semantic_ir_digest.clone(),
            composition_plan_digest: coordinates.composition_plan_digest.clone(),
        }
    }
}

/// Effective-profile request admitted at the JSON-RPC boundary.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct EffectiveProfileParams {
    context: ProfileInspectionContext,
    section: EffectiveProfileSection,
    offset: u32,
    limit: u16,
    expected_coordinate: Option<ExpectedProfileCoordinateWire>,
}

/// Effective-profile explanation request admitted at the JSON-RPC boundary.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct EffectiveProfileExplanationParams {
    context: ProfileInspectionContext,
    handle: EffectiveProfileExplanationHandle,
    expected_coordinate: ExpectedProfileCoordinateWire,
}

/// One closed transport operation over the retained improvement typestate.
#[derive(Debug, serde::Deserialize)]
#[serde(
    tag = "action",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
enum ConfigurationImprovementParams {
    /// Bind the current admitted editor/profile product as the baseline.
    Begin {
        context: ProfileInspectionContext,
        expected_profile: ExpectedProfileCoordinateWire,
    },
    /// Evaluate the exact current editor snapshot against that baseline.
    Evaluate {
        context: ProfileInspectionContext,
        expected_baseline: ExpectedProfileCoordinateWire,
    },
    /// Read bounded evidence from the exact retained preview.
    Inspect {
        context: ProfileInspectionContext,
        expected_preview: ImprovementPreviewCoordinateWire,
        detail: ConfigurationImprovementDetailParams,
    },
    /// Consume a preview into one explicit non-effectful disposition.
    Finish {
        context: ProfileInspectionContext,
        expected_preview: ImprovementPreviewCoordinateWire,
        disposition: ImprovementReviewDisposition,
    },
}

/// Closed bounded queries over one retained review preview.
#[derive(Debug, serde::Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
enum ConfigurationImprovementDetailParams {
    /// Read one page from each canonical comparison partition.
    Overview {
        roots_offset: u32,
        impacts_offset: u32,
        limit: u16,
    },
    /// Explain and locate one full compiler-owned subject.
    Subject {
        subject: SubjectSelector,
        input_limit: u16,
        maximum_inspected: u16,
        maximum_returned: u16,
    },
}

/// Closed worker query for one coherent configuration-improvement capability.
#[derive(Debug)]
pub(super) enum ConfigurationImprovementQuery {
    /// Start from the exact retained profile coordinate.
    Begin {
        /// Ephemeral profile-selection context.
        context: ProfileInspectionContext,
        /// Current editor/profile product selected as baseline.
        expected_profile: ExpectedProfileCoordinate,
    },
    /// Compile the exact current editor snapshot as candidate.
    Evaluate {
        /// Ephemeral profile-selection context.
        context: ProfileInspectionContext,
        /// Coordinate returned by the begin transition.
        expected_baseline: ExpectedProfileCoordinate,
    },
    /// Read bounded proof and source evidence without consuming the preview.
    Inspect {
        /// Ephemeral profile-selection context.
        context: ProfileInspectionContext,
        /// Coordinate returned by the preview transition.
        expected_preview: ImprovementPreviewCoordinate,
        /// One bounded detail query.
        detail: Box<ConfigurationImprovementDetailQuery>,
    },
    /// End the review without mutating configuration.
    Finish {
        /// Ephemeral profile-selection context.
        context: ProfileInspectionContext,
        /// Coordinate returned by the preview transition.
        expected_preview: ImprovementPreviewCoordinate,
        /// Human-selected terminal disposition.
        disposition: ImprovementReviewDisposition,
    },
}

/// Typed worker query over the retained preview's existing semantic owners.
#[derive(Debug)]
pub(super) enum ConfigurationImprovementDetailQuery {
    /// Read canonical change-root and impact pages.
    Overview {
        /// Zero-based change-root offset.
        roots_offset: u32,
        /// Zero-based impact-partition offset.
        impacts_offset: u32,
        /// Shared per-partition page bound.
        limit: u16,
    },
    /// Read causal and exact candidate-source evidence for one subject.
    Subject {
        /// Validated full compiler-proof identity.
        subject: PortableWorkspaceSubject,
        /// Maximum direct proof inputs returned.
        input_limit: u16,
        /// Maximum candidate source entries examined.
        maximum_inspected: u16,
        /// Maximum matching candidate occurrences returned.
        maximum_returned: u16,
    },
}

/// Exact preview product required before terminal disposition.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
pub(super) struct ImprovementPreviewCoordinate {
    /// Baseline editor and compiler product.
    pub(super) baseline: ExpectedProfileCoordinate,
    /// Editor snapshot admitted as the candidate.
    pub(super) candidate_editor: EditorCoordinate,
    /// Candidate selected-topology identity.
    pub(super) candidate_selected_topology_digest: String,
    /// Candidate effective-profile identity.
    pub(super) candidate_profile_digest: String,
    /// Candidate private-instance identity.
    pub(super) candidate_instance_digest: String,
    /// Exact candidate derivation proof.
    pub(super) derivation_receipt_digest: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct ImprovementPreviewCoordinateWire {
    baseline: ExpectedProfileCoordinateWire,
    candidate_editor: EditorCoordinateWire,
    candidate_selected_topology_digest: String,
    candidate_profile_digest: String,
    candidate_instance_digest: String,
    derivation_receipt_digest: String,
}

impl TryFrom<ImprovementPreviewCoordinateWire> for ImprovementPreviewCoordinate {
    type Error = ProtocolError;

    fn try_from(value: ImprovementPreviewCoordinateWire) -> Result<Self, Self::Error> {
        Ok(Self {
            baseline: ExpectedProfileCoordinate::try_from(value.baseline)?,
            candidate_editor: EditorCoordinate::try_from(value.candidate_editor)?,
            candidate_selected_topology_digest: value.candidate_selected_topology_digest,
            candidate_profile_digest: value.candidate_profile_digest,
            candidate_instance_digest: value.candidate_instance_digest,
            derivation_receipt_digest: value.derivation_receipt_digest,
        })
    }
}

/// Explicit review decision; every variant remains effect-free.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
pub(super) enum ImprovementReviewDisposition {
    /// Mark the preview accepted for a separate authority/effect gate.
    Accept,
    /// Reject the preview.
    Reject,
    /// Cancel the review.
    Cancel,
}

/// Closed effective-profile query passed to the single analysis owner.
#[derive(Debug)]
pub(super) struct EffectiveProfileQuery {
    /// Ephemeral project and execution-workspace selection.
    pub(super) context: ProfileInspectionContext,
    /// API-owned shallow inspection partition.
    pub(super) section: EffectiveProfileSection,
    /// Zero-based entry offset.
    pub(super) offset: u32,
    /// Requested page size, bounded again by the API owner.
    pub(super) limit: u16,
    /// Required continuation coordinate when [`Self::offset`] is nonzero.
    pub(super) expected_coordinate: Option<ExpectedProfileCoordinate>,
}

/// Closed profile-explanation query passed to the single analysis owner.
#[derive(Debug)]
pub(super) struct EffectiveProfileExplanationQuery {
    /// Ephemeral selection under which the profile was admitted.
    pub(super) context: ProfileInspectionContext,
    /// Opaque API-owned identity of one retained explanation.
    pub(super) handle: EffectiveProfileExplanationHandle,
    /// Exact editor/profile coordinate returned with the originating row.
    pub(super) expected_coordinate: ExpectedProfileCoordinate,
}

/// Bounded page coordinate passed unchanged to the API-owned proof handle.
#[derive(Clone, Copy, Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
pub(super) struct PageParams {
    /// Zero-based entry offset.
    pub(super) offset: u32,
    /// Requested page size; the API applies its default and maximum.
    pub(super) limit: u16,
}

/// Structured capacity failure carried in an LSP response-error `data` field.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct AuthoringLimitErrorData {
    kind: AuthoringLimitKind,
    unit: AuthoringLimitUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_minimum: Option<String>,
    recovery: &'static str,
}

impl AuthoringLimitErrorData {
    /// Bound a saturated request queue without claiming worker cancellation.
    pub(super) fn query_queue(maximum: usize) -> Self {
        Self::bounded(
            AuthoringLimitKind::QueryQueue,
            AuthoringLimitUnit::Requests,
            maximum,
            "Wait for one Pale request to finish, then retry against the current editor generation.",
        )
    }

    /// Direct reference projection was truncated by its API-owned ceiling.
    pub(super) const fn direct_references() -> Self {
        Self::unquantified(
            AuthoringLimitKind::DirectReferences,
            AuthoringLimitUnit::Locations,
            "Retry with an LSP partial-result token so Pale can stream bounded reference pages.",
        )
    }

    /// Snapshot-derived reference-location custody could not grow safely.
    pub(super) const fn reference_locations() -> Self {
        Self::unquantified(
            AuthoringLimitKind::ReferenceLocations,
            AuthoringLimitUnit::Locations,
            "Reduce the configured Pale source set, wait for the new analysis, and retry.",
        )
    }

    /// Process-local reference pagination cannot advance past `u64::MAX`.
    pub(super) fn reference_pages() -> Self {
        Self {
            kind: AuthoringLimitKind::ReferencePages,
            unit: AuthoringLimitUnit::Pages,
            scope: None,
            maximum: Some(u64::MAX.to_string()),
            observed: None,
            observed_minimum: None,
            recovery: "Start a new reference request against the current editor generation.",
        }
    }

    /// One source body exceeded its application-owned byte ceiling.
    pub(super) fn document_bytes(scope: &'static str, observed: u64, maximum: u64) -> Self {
        Self::exact(
            AuthoringLimitKind::DocumentBytes,
            AuthoringLimitUnit::Bytes,
            scope,
            observed,
            maximum,
            "Reduce this source document, then retry the editor change.",
        )
    }

    /// One source-custody class exceeded its document-count ceiling.
    pub(super) fn document_count(scope: &'static str, observed: usize, maximum: usize) -> Self {
        Self::exact(
            AuthoringLimitKind::DocumentCount,
            AuthoringLimitUnit::Documents,
            scope,
            u64::try_from(observed).expect("LSP document counts fit u64"),
            u64::try_from(maximum).expect("LSP document-count ceilings fit u64"),
            "Remove or combine sources in this custody class, then restart the Pale language server.",
        )
    }

    /// Aggregate bytes exceeded one application-owned custody-class ceiling.
    pub(super) fn total_bytes(scope: &'static str, observed: u64, maximum: u64) -> Self {
        Self::exact(
            AuthoringLimitKind::TotalBytes,
            AuthoringLimitUnit::Bytes,
            scope,
            observed,
            maximum,
            "Reduce the total source bytes in this custody class, then retry.",
        )
    }

    /// Tolerant parsing reached the LSP-owned structural-fact ceiling.
    pub(super) fn syntax_facts(maximum: usize) -> Self {
        Self::bounded(
            AuthoringLimitKind::SyntaxFacts,
            AuthoringLimitUnit::Facts,
            maximum,
            "Split the configuration into smaller source documents, wait for analysis, and retry.",
        )
    }

    /// One non-streaming LSP response exceeded its member ceiling.
    pub(super) fn standard_response(maximum: usize) -> Self {
        Self::bounded(
            AuthoringLimitKind::StandardResponse,
            AuthoringLimitUnit::Members,
            maximum,
            "Reduce the source document or request scope, wait for analysis, and retry.",
        )
    }

    pub(super) const fn recovery(&self) -> &'static str {
        self.recovery
    }

    fn bounded(
        kind: AuthoringLimitKind,
        unit: AuthoringLimitUnit,
        maximum: usize,
        recovery: &'static str,
    ) -> Self {
        let maximum = u64::try_from(maximum).expect("LSP capacity ceilings fit u64");
        Self {
            kind,
            unit,
            scope: None,
            maximum: Some(maximum.to_string()),
            observed: None,
            observed_minimum: maximum.checked_add(1).map(|value| value.to_string()),
            recovery,
        }
    }

    const fn unquantified(
        kind: AuthoringLimitKind,
        unit: AuthoringLimitUnit,
        recovery: &'static str,
    ) -> Self {
        Self {
            kind,
            unit,
            scope: None,
            maximum: None,
            observed: None,
            observed_minimum: None,
            recovery,
        }
    }

    fn exact(
        kind: AuthoringLimitKind,
        unit: AuthoringLimitUnit,
        scope: &'static str,
        observed: u64,
        maximum: u64,
        recovery: &'static str,
    ) -> Self {
        Self {
            kind,
            unit,
            scope: Some(scope),
            maximum: Some(maximum.to_string()),
            observed: Some(observed.to_string()),
            observed_minimum: None,
            recovery,
        }
    }
}

/// Capacity boundary that rejected an authoring operation.
#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum AuthoringLimitKind {
    QueryQueue,
    DirectReferences,
    ReferenceLocations,
    ReferencePages,
    SyntaxFacts,
    StandardResponse,
    DocumentBytes,
    DocumentCount,
    TotalBytes,
}

/// Unit used by an authoring capacity boundary.
#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum AuthoringLimitUnit {
    Requests,
    Locations,
    Pages,
    Facts,
    Members,
    Bytes,
    Documents,
}

/// Failure to decode one known custom method.
#[derive(Debug, thiserror::Error)]
pub(super) enum ProtocolError {
    /// Parameters did not match the method's closed transport shape.
    #[error("invalid parameters for `{method}`: {source}")]
    Parameters {
        /// Stable custom-method identity.
        method: &'static str,
        /// Typed JSON decoding failure without semantic interpretation.
        #[source]
        source: serde_json::Error,
    },
    /// One selector member violated the API-owned identity grammar.
    #[error("invalid subject selector: {0}")]
    Selector(#[from] pale_sdk::InvalidPortableWorkspaceSubjectSelector),
    /// A decimal editor coordinate was non-canonical or outside `u64`.
    #[error("invalid canonical decimal editor coordinate `{0}`")]
    EditorCoordinate(String),
}

/// Decode a known method at the JSON-RPC edge.
///
/// `Ok(None)` means the method is not owned by this LSP tranche. JSON values do
/// not cross this function: the analysis worker receives only [`WorkspaceQuery`].
pub(super) fn decode_query(
    method: &str,
    params: serde_json::Value,
) -> Result<Option<WorkspaceQuery>, ProtocolError> {
    let query = match method {
        COMPILATION_SUMMARY_METHOD => {
            decode_empty(COMPILATION_SUMMARY_METHOD, params)?;
            WorkspaceQuery::CompilationSummary
        }
        COMPARISON_SUMMARY_METHOD => {
            decode_empty(COMPARISON_SUMMARY_METHOD, params)?;
            WorkspaceQuery::ComparisonSummary
        }
        CHANGE_ROOTS_METHOD => WorkspaceQuery::ChangeRoots(decode(CHANGE_ROOTS_METHOD, params)?),
        IMPACTS_METHOD => WorkspaceQuery::Impacts(decode(IMPACTS_METHOD, params)?),
        EXPLANATION_METHOD => {
            let params: ExplanationParams = decode(EXPLANATION_METHOD, params)?;
            WorkspaceQuery::Explanation(PortableWorkspaceSubject::try_from(
                PortableWorkspaceSubjectSelector::from(params.subject),
            )?)
        }
        PROFILE_CONTEXTS_METHOD => {
            decode_empty(PROFILE_CONTEXTS_METHOD, params)?;
            WorkspaceQuery::ProfileContexts
        }
        EFFECTIVE_PROFILE_METHOD => {
            let params: EffectiveProfileParams = decode(EFFECTIVE_PROFILE_METHOD, params)?;
            let expected_coordinate = params
                .expected_coordinate
                .map(ExpectedProfileCoordinate::try_from)
                .transpose()?;
            WorkspaceQuery::EffectiveProfile(EffectiveProfileQuery {
                context: params.context,
                section: params.section,
                offset: params.offset,
                limit: params.limit,
                expected_coordinate,
            })
        }
        EFFECTIVE_PROFILE_EXPLANATION_METHOD => {
            let params: EffectiveProfileExplanationParams =
                decode(EFFECTIVE_PROFILE_EXPLANATION_METHOD, params)?;
            WorkspaceQuery::EffectiveProfileExplanation(EffectiveProfileExplanationQuery {
                context: params.context,
                handle: params.handle,
                expected_coordinate: ExpectedProfileCoordinate::try_from(
                    params.expected_coordinate,
                )?,
            })
        }
        CONFIGURATION_IMPROVEMENT_METHOD => {
            let params: ConfigurationImprovementParams =
                decode(CONFIGURATION_IMPROVEMENT_METHOD, params)?;
            WorkspaceQuery::ConfigurationImprovement(match params {
                ConfigurationImprovementParams::Begin {
                    context,
                    expected_profile,
                } => ConfigurationImprovementQuery::Begin {
                    context,
                    expected_profile: ExpectedProfileCoordinate::try_from(expected_profile)?,
                },
                ConfigurationImprovementParams::Evaluate {
                    context,
                    expected_baseline,
                } => ConfigurationImprovementQuery::Evaluate {
                    context,
                    expected_baseline: ExpectedProfileCoordinate::try_from(expected_baseline)?,
                },
                ConfigurationImprovementParams::Inspect {
                    context,
                    expected_preview,
                    detail,
                } => ConfigurationImprovementQuery::Inspect {
                    context,
                    expected_preview: ImprovementPreviewCoordinate::try_from(expected_preview)?,
                    detail: Box::new(match detail {
                        ConfigurationImprovementDetailParams::Overview {
                            roots_offset,
                            impacts_offset,
                            limit,
                        } => ConfigurationImprovementDetailQuery::Overview {
                            roots_offset,
                            impacts_offset,
                            limit,
                        },
                        ConfigurationImprovementDetailParams::Subject {
                            subject,
                            input_limit,
                            maximum_inspected,
                            maximum_returned,
                        } => ConfigurationImprovementDetailQuery::Subject {
                            subject: PortableWorkspaceSubject::try_from(
                                PortableWorkspaceSubjectSelector::from(subject),
                            )?,
                            input_limit,
                            maximum_inspected,
                            maximum_returned,
                        },
                    }),
                },
                ConfigurationImprovementParams::Finish {
                    context,
                    expected_preview,
                    disposition,
                } => ConfigurationImprovementQuery::Finish {
                    context,
                    expected_preview: ImprovementPreviewCoordinate::try_from(expected_preview)?,
                    disposition,
                },
            })
        }
        SemanticTokensFullRequest::METHOD => WorkspaceQuery::Authoring(
            AuthoringQuery::SemanticTokens(decode(SemanticTokensFullRequest::METHOD, params)?),
        ),
        Completion::METHOD => WorkspaceQuery::Authoring(AuthoringQuery::Completion(decode(
            Completion::METHOD,
            params,
        )?)),
        HoverRequest::METHOD => {
            WorkspaceQuery::Authoring(AuthoringQuery::Hover(decode(HoverRequest::METHOD, params)?))
        }
        DocumentSymbolRequest::METHOD => WorkspaceQuery::Authoring(
            AuthoringQuery::DocumentSymbols(decode(DocumentSymbolRequest::METHOD, params)?),
        ),
        GotoDefinition::METHOD => WorkspaceQuery::Authoring(AuthoringQuery::Definition(decode(
            GotoDefinition::METHOD,
            params,
        )?)),
        References::METHOD => WorkspaceQuery::Authoring(AuthoringQuery::References(decode(
            References::METHOD,
            params,
        )?)),
        CodeActionRequest::METHOD => WorkspaceQuery::Authoring(AuthoringQuery::CodeActions(
            decode(CodeActionRequest::METHOD, params)?,
        )),
        _ => return Ok(None),
    };
    Ok(Some(query))
}

/// Advertise independent inventory and represented-exchange compatibility evidence.
///
/// Neither digest proves exhaustive wire-shape equivalence. The fixture
/// attestation supplements strict request/response decoding rather than replacing it.
pub(super) fn experimental_capabilities() -> serde_json::Value {
    let inventory = ProtocolInventory {
        protocol_id: PROTOCOL_ID,
        methods: &IMPLEMENTED_METHODS,
        notifications: [ANALYSIS_PUBLISHED_NOTIFICATION],
        cancellation: RequestCancellation::ResultSuppression,
        document_roles: DocumentRole::protocol_names(),
    };
    let inventory_sha256 = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&inventory).expect("the private LSP inventory is serializable")
        )
    );
    serde_json::to_value(ExperimentalCapabilities {
        pale_workspace: PaleWorkspaceCapabilities {
            package_version: env!("CARGO_PKG_VERSION"),
            inventory_sha256,
            wire_fixture_sha256: WIRE_FIXTURE_SHA256,
            inventory,
        },
    })
    .expect("the static LSP capability advertisement is serializable")
}

/// Known-method result projected at the JSON-RPC edge.
pub(super) enum WorkspaceQueryResult {
    /// API-owned canonical compilation summary.
    CompilationSummary(pale_sdk::PortableWorkspaceCompilationSummary),
    /// API-owned canonical comparison summary.
    ComparisonSummary(pale_sdk::PortableWorkspaceComparisonSummary),
    /// LSP transport projection of exact API roots.
    ChangeRoots(ChangeRootPage),
    /// LSP transport projection of the API impact partition.
    Impacts(ImpactPage),
    /// LSP transport projection of one API causal explanation.
    Explanation(Box<Explanation>),
    /// Exact admitted pairs bound to one editor coordinate.
    ProfileContexts(ProfileContextsResponse),
    /// LSP-owned editor coordinate around one API-owned profile page.
    EffectiveProfile(Box<EffectiveProfileResult>),
    /// Coordinate-fenced explanation from the retained effective profile.
    EffectiveProfileExplanation(Box<EffectiveProfileExplanationResponse>),
    /// Coordinate-fenced result of one improvement-lifecycle transition.
    ConfigurationImprovement(Box<ConfigurationImprovementResponse>),
    /// Standard LSP result projected from one retained authoring coordinate.
    Authoring(AuthoringQueryResult),
}

/// Exact profile-selection pairs from one retained editor compilation.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
pub(super) struct ProfileContextsResponse {
    /// Editor snapshot that supplied these pairs.
    pub(super) editor: EditorCoordinate,
    /// Canonically ordered valid project and execution-workspace pairs.
    pub(super) contexts: Vec<pale_sdk::PortableWorkspaceProfileContext>,
}

/// Closed standard-LSP result family.
pub(super) enum AuthoringQueryResult {
    /// Full semantic tokens, when the selected document is TOML.
    SemanticTokens(Option<SemanticTokensResult>),
    /// Context-sensitive completion response.
    Completion(Option<CompletionResponse>),
    /// Hover response for one exact position.
    Hover(Option<Hover>),
    /// Structural and semantic document symbols.
    DocumentSymbols(Option<DocumentSymbolResponse>),
    /// Canonical definition target.
    Definition(Option<GotoDefinitionResponse>),
    /// Deterministically ordered reference locations.
    References(Option<Vec<Location>>),
    /// Previewable non-effectful edits.
    CodeActions(Option<CodeActionResponse>),
}

impl TryFrom<ExpectedProfileCoordinateWire> for ExpectedProfileCoordinate {
    type Error = ProtocolError;

    fn try_from(value: ExpectedProfileCoordinateWire) -> Result<Self, Self::Error> {
        Ok(Self {
            generation: parse_decimal_coordinate(value.generation)?,
            document_revision: parse_decimal_coordinate(value.document_revision)?,
            profile_digest: value.profile_digest,
            instance_digest: value.instance_digest,
            selected_topology_digest: value.selected_topology_digest,
            workspace_semantic_ir_digest: value.workspace_semantic_ir_digest,
            workspace_admission_evidence_digest: value.workspace_admission_evidence_digest,
            package_semantic_ir_digest: value.package_semantic_ir_digest,
            composition_plan_digest: value.composition_plan_digest,
        })
    }
}

/// Borrowed serialized profile coordinate; editor counters are decimal strings.
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
struct ExpectedProfileCoordinateOutput<'a> {
    #[cfg_attr(
        feature = "protocol-codegen",
        schemars(schema_with = "schema::u64_decimal")
    )]
    generation: String,
    #[cfg_attr(
        feature = "protocol-codegen",
        schemars(schema_with = "schema::u64_decimal")
    )]
    document_revision: String,
    profile_digest: &'a str,
    instance_digest: &'a str,
    selected_topology_digest: &'a str,
    workspace_semantic_ir_digest: &'a str,
    workspace_admission_evidence_digest: &'a str,
    package_semantic_ir_digest: &'a str,
    composition_plan_digest: &'a str,
}

impl serde::Serialize for ExpectedProfileCoordinate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ExpectedProfileCoordinateOutput {
            generation: self.generation.to_string(),
            document_revision: self.document_revision.to_string(),
            profile_digest: &self.profile_digest,
            instance_digest: &self.instance_digest,
            selected_topology_digest: &self.selected_topology_digest,
            workspace_semantic_ir_digest: &self.workspace_semantic_ir_digest,
            workspace_admission_evidence_digest: &self.workspace_admission_evidence_digest,
            package_semantic_ir_digest: &self.package_semantic_ir_digest,
            composition_plan_digest: &self.composition_plan_digest,
        }
        .serialize(serializer)
    }
}

fn parse_decimal_coordinate(value: String) -> Result<u64, ProtocolError> {
    value
        .parse::<u64>()
        .ok()
        .filter(|parsed| parsed.to_string() == value)
        .ok_or(ProtocolError::EditorCoordinate(value))
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct EditorCoordinateWire {
    generation: String,
    document_revision: String,
}

impl TryFrom<EditorCoordinateWire> for EditorCoordinate {
    type Error = ProtocolError;

    fn try_from(value: EditorCoordinateWire) -> Result<Self, Self::Error> {
        Ok(Self {
            generation: parse_decimal_coordinate(value.generation)?,
            document_revision: parse_decimal_coordinate(value.document_revision)?,
        })
    }
}

/// Exact editor snapshot represented by one profile response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct EditorCoordinate {
    pub(super) generation: u64,
    pub(super) document_revision: u64,
}

/// Serialized editor counters, not JavaScript numbers.
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
struct EditorCoordinateOutput {
    #[cfg_attr(
        feature = "protocol-codegen",
        schemars(schema_with = "schema::u64_decimal")
    )]
    generation: String,
    #[cfg_attr(
        feature = "protocol-codegen",
        schemars(schema_with = "schema::u64_decimal")
    )]
    document_revision: String,
}

impl serde::Serialize for EditorCoordinate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        EditorCoordinateOutput {
            generation: self.generation.to_string(),
            document_revision: self.document_revision.to_string(),
        }
        .serialize(serializer)
    }
}

/// Admission relationship between current editor input and the shown profile.
#[derive(serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
pub(super) enum EffectiveProfileState {
    /// Current input owns the admitted profile.
    Current,
    /// Current input is rejected while same-context prior evidence remains.
    Stale {
        /// Editor coordinate that admitted the displayed predecessor.
        admitted_editor: EditorCoordinate,
        /// API-owned identity of the displayed predecessor.
        admitted_profile_digest: String,
        /// Plain owner summary for the rejected current input.
        summary: String,
    },
    /// No profile can truthfully be shown for the selected context.
    NotAdmitted {
        /// Plain owner summary for the absent current product.
        summary: String,
    },
}

/// Typed LSP envelope around one API-owned effective-profile page.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
pub(super) struct EffectiveProfileResponse {
    /// Truthful relationship between editor input and displayed evidence.
    pub(super) state: EffectiveProfileState,
    /// Exact current editor request coordinate.
    pub(super) editor: EditorCoordinate,
    /// Ephemeral selection against which the response was admitted.
    pub(super) context: ProfileInspectionContext,
    /// Absent only when no admitted profile exists for the selected context.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "protocol-codegen",
        schemars(with = "EffectiveProfileCoordinates")
    )]
    pub(super) coordinates: Option<EffectiveProfileCoordinates>,
    /// Absent only when no admitted profile exists for the selected context.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "protocol-codegen", schemars(with = "EffectiveProfilePage"))]
    pub(super) page: Option<EffectiveProfilePage>,
    /// Exact API diagnostic also published through standard LSP, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "protocol-codegen", schemars(with = "DiagnosticIdentity"))]
    pub(super) diagnostic_identity: Option<DiagnosticIdentity>,
}

/// Typed LSP envelope around one retained API-owned profile explanation.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
pub(super) struct EffectiveProfileExplanationResponse {
    /// Exact current editor request coordinate.
    pub(super) editor: EditorCoordinate,
    /// Ephemeral selection under which the retained profile was admitted.
    pub(super) context: ProfileInspectionContext,
    /// API/profile coordinates fenced by the request.
    pub(super) coordinates: EffectiveProfileCoordinates,
    /// Bounded API-owned explanation evidence.
    pub(super) explanation: EffectiveProfileExplanation,
}

/// Closed transport projection of the retained improvement typestate.
#[derive(serde::Serialize)]
#[serde(
    tag = "state",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
pub(super) enum ConfigurationImprovementResponse {
    /// Baseline was inspected and joined to its canonical profile proof.
    Explained {
        /// Ephemeral selection bound to the review.
        context: ProfileInspectionContext,
        /// Exact editor/profile product selected as baseline.
        baseline: ExpectedProfileCoordinate,
    },
    /// Candidate was compiled, diagnosed, and frozen for review.
    Previewed {
        /// Ephemeral selection bound to the review.
        context: ProfileInspectionContext,
        /// Exact review product coordinate.
        coordinate: ImprovementPreviewCoordinate,
        /// API-owned bounded preview projection.
        preview: ConfigurationImprovementPreviewProjection,
    },
    /// Bounded root and impact pages from the retained preview.
    Details {
        /// Ephemeral selection bound to the review.
        context: ProfileInspectionContext,
        /// Exact review product coordinate.
        coordinate: ImprovementPreviewCoordinate,
        /// Canonical change-root page.
        change_roots: ChangeRootPage,
        /// Canonical current impact-partition page.
        impacts: ImpactPage,
    },
    /// Causal and source evidence for one candidate subject.
    SubjectDetails {
        /// Ephemeral selection bound to the review.
        context: ProfileInspectionContext,
        /// Exact review product coordinate.
        coordinate: ImprovementPreviewCoordinate,
        /// Canonical comparison-owned causal result.
        explanation: Explanation,
        /// Exact candidate-owned source result.
        source: ConfigurationImprovementSourceProjection,
    },
    /// Review ended without source or native mutation.
    Terminal {
        /// Ephemeral selection bound to the review.
        context: ProfileInspectionContext,
        /// Exact terminal disposition selected or proved stale.
        disposition: pale_sdk::ConfigurationImprovementDisposition,
        /// Exact reviewed preview coordinate.
        coordinate: ImprovementPreviewCoordinate,
        /// API-owned bounded preview projection.
        preview: ConfigurationImprovementPreviewProjection,
    },
}

/// Bounded editor transport over the API-owned preview summary.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
pub(super) struct ConfigurationImprovementPreviewProjection {
    /// Principal workspace under review.
    pub(super) workspace_id: String,
    /// Selected project.
    pub(super) project_id: String,
    /// Selected execution workspace.
    pub(super) execution_workspace_id: String,
    /// Baseline workspace semantic identity.
    pub(super) baseline_workspace_semantic_ir_digest: String,
    /// Baseline workspace admission-evidence identity.
    pub(super) baseline_workspace_admission_evidence_digest: String,
    /// Baseline effective-profile identity.
    pub(super) profile_digest: String,
    /// Baseline private-instance identity.
    pub(super) instance_digest: String,
    /// Candidate effective-profile identity.
    pub(super) candidate_profile_digest: String,
    /// Candidate private-instance identity.
    pub(super) candidate_instance_digest: String,
    /// Candidate workspace semantic identity.
    pub(super) candidate_workspace_semantic_ir_digest: String,
    /// Candidate workspace admission-evidence identity.
    pub(super) candidate_workspace_admission_evidence_digest: String,
    /// Exact derivation receipt.
    pub(super) derivation_receipt_digest: String,
    /// Number of exact changed roots.
    pub(super) change_root_count: u32,
    /// Number of affected semantic subjects.
    pub(super) affected_count: u32,
    /// Number of unaffected semantic subjects.
    pub(super) unaffected_count: u32,
    /// Exact desired-topology transition.
    pub(super) selected_topology_change: pale_sdk::ConfigurationImprovementCoordinateChange,
    /// Exact effective-profile transition.
    pub(super) profile_change: pale_sdk::ConfigurationImprovementCoordinateChange,
    /// Exact private-instance transition.
    pub(super) instance_change: pale_sdk::ConfigurationImprovementCoordinateChange,
    /// Exact selected Package semantic transition.
    pub(super) package_semantic_ir_change: pale_sdk::ConfigurationImprovementCoordinateChange,
    /// Exact composition-proof transition.
    pub(super) composition_plan_change: pale_sdk::ConfigurationImprovementCoordinateChange,
}

impl From<&pale_sdk::ConfigurationImprovementPreviewSummary>
    for ConfigurationImprovementPreviewProjection
{
    fn from(summary: &pale_sdk::ConfigurationImprovementPreviewSummary) -> Self {
        Self {
            workspace_id: summary.workspace_id().to_owned(),
            project_id: summary.project_id().to_owned(),
            execution_workspace_id: summary.execution_workspace_id().to_owned(),
            baseline_workspace_semantic_ir_digest: summary
                .baseline_workspace_semantic_ir_digest()
                .to_owned(),
            baseline_workspace_admission_evidence_digest: summary
                .baseline_workspace_admission_evidence_digest()
                .to_owned(),
            profile_digest: summary.profile_digest().to_owned(),
            instance_digest: summary.instance_digest().to_owned(),
            candidate_profile_digest: summary.candidate_profile_digest().to_owned(),
            candidate_instance_digest: summary.candidate_instance_digest().to_owned(),
            candidate_workspace_semantic_ir_digest: summary
                .candidate_workspace_semantic_ir_digest()
                .to_owned(),
            candidate_workspace_admission_evidence_digest: summary
                .candidate_workspace_admission_evidence_digest()
                .to_owned(),
            derivation_receipt_digest: summary.derivation_receipt_digest().to_owned(),
            change_root_count: summary.change_root_count(),
            affected_count: summary.affected_count(),
            unaffected_count: summary.unaffected_count(),
            selected_topology_change: summary.selected_topology_change().clone(),
            profile_change: summary.profile_change().clone(),
            instance_change: summary.instance_change().clone(),
            package_semantic_ir_change: summary.package_semantic_ir_change().clone(),
            composition_plan_change: summary.composition_plan_change().clone(),
        }
    }
}

/// One exact source path segment retained by candidate compilation.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
enum SourcePathSegment {
    /// Descend through one decoded field.
    Field(String),
    /// Select one exact array member.
    Index(u32),
}

impl From<&AuthoringInstancePathSegment> for SourcePathSegment {
    fn from(value: &AuthoringInstancePathSegment) -> Self {
        match value {
            AuthoringInstancePathSegment::Field(field) => Self::Field(field.clone()),
            AuthoringInstancePathSegment::Index(index) => Self::Index(
                u32::try_from(*index)
                    .expect("authoring paths are bounded below the protocol's u32 ceiling"),
            ),
        }
    }
}

/// One exact compiler-owned logical source location.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
struct SourceLocation {
    source_label: String,
    path: Vec<SourcePathSegment>,
}

impl From<&AuthoringSourceLocation> for SourceLocation {
    fn from(value: &AuthoringSourceLocation) -> Self {
        Self {
            source_label: value.source_label().to_owned(),
            path: value.path().iter().map(SourcePathSegment::from).collect(),
        }
    }
}

/// Closed role of one compiler-proved source occurrence.
#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
enum SourceOccurrenceRole {
    /// Canonical local definition.
    Definition,
    /// Reference to a separately defined identity.
    Reference,
    /// Content-bearing interface projection.
    Projection,
}

impl From<AuthoringOccurrenceRole> for SourceOccurrenceRole {
    fn from(value: AuthoringOccurrenceRole) -> Self {
        match value {
            AuthoringOccurrenceRole::Definition => Self::Definition,
            AuthoringOccurrenceRole::Reference => Self::Reference,
            AuthoringOccurrenceRole::Projection => Self::Projection,
        }
    }
}

/// One exact candidate source occurrence.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
struct SourceOccurrence {
    location: SourceLocation,
    role: SourceOccurrenceRole,
}

/// Bounded candidate-source evidence for one review subject.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
pub(super) struct ConfigurationImprovementSourceProjection {
    subject: SubjectSelector,
    occurrences: Vec<SourceOccurrence>,
    inspected: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "protocol-codegen", schemars(with = "SourceLocation"))]
    record_anchor: Option<SourceLocation>,
    sources_complete: bool,
}

impl From<&PortableWorkspaceSubjectInspection> for ConfigurationImprovementSourceProjection {
    fn from(value: &PortableWorkspaceSubjectInspection) -> Self {
        Self {
            subject: SubjectSelector::from(value.subject()),
            occurrences: value
                .occurrences()
                .iter()
                .map(|occurrence| SourceOccurrence {
                    location: SourceLocation::from(occurrence.location()),
                    role: SourceOccurrenceRole::from(occurrence.role()),
                })
                .collect(),
            inspected: value.inspected(),
            record_anchor: value.record_anchor().map(SourceLocation::from),
            sources_complete: value.sources_complete(),
        }
    }
}

/// Internal result keeping exact diagnostics out of the custom DTO.
pub(super) struct EffectiveProfileResult {
    /// Custom-method response containing no diagnostic reclassification.
    pub(super) response: EffectiveProfileResponse,
    /// Exact API diagnostic published through standard LSP when present.
    diagnostic: Option<DiagnosticV1>,
    /// Exact owner-retained source coordinate for [`Self::diagnostic`].
    authoring_location: Option<AuthoringSourceLocation>,
}

impl EffectiveProfileResult {
    /// Bind a response to the exact API diagnostic published through standard LSP.
    pub(super) const fn new(
        response: EffectiveProfileResponse,
        diagnostic: Option<DiagnosticV1>,
        authoring_location: Option<AuthoringSourceLocation>,
    ) -> Self {
        Self {
            response,
            diagnostic,
            authoring_location,
        }
    }

    /// Return the API diagnostic for standard `textDocument/publishDiagnostics`.
    pub(super) const fn diagnostic(&self) -> Option<&DiagnosticV1> {
        self.diagnostic.as_ref()
    }

    /// Return the exact compiler-owned field/index coordinate, when proved.
    pub(super) const fn authoring_location(&self) -> Option<&AuthoringSourceLocation> {
        self.authoring_location.as_ref()
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct EmptyParams {}

fn decode_empty(method: &'static str, params: serde_json::Value) -> Result<(), ProtocolError> {
    if params.is_null() {
        return Ok(());
    }
    decode::<EmptyParams>(method, params).map(|_| ())
}

fn decode<T: serde::de::DeserializeOwned>(
    method: &'static str,
    params: serde_json::Value,
) -> Result<T, ProtocolError> {
    serde_json::from_value(params).map_err(|source| ProtocolError::Parameters { method, source })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ExperimentalCapabilities {
    pale_workspace: PaleWorkspaceCapabilities,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
struct PaleWorkspaceCapabilities {
    package_version: &'static str,
    inventory_sha256: String,
    wire_fixture_sha256: &'static str,
    #[serde(flatten)]
    inventory: ProtocolInventory,
}

/// Exact private protocol vocabulary hashed before advertisement.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct ProtocolInventory {
    protocol_id: &'static str,
    methods: &'static [&'static str],
    notifications: [&'static str; 1],
    cancellation: RequestCancellation,
    document_roles: Vec<String>,
}

/// Transport-only notice that the retained analysis state changed.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "protocol-codegen", schemars(deny_unknown_fields))]
pub(super) struct AnalysisPublication {
    /// Exact editor state accepted by the server scheduler.
    pub(super) editor: EditorCoordinate,
    /// Whether the canonical compiler admitted that editor state.
    pub(super) admission: AnalysisAdmission,
}

/// Admission disposition of one accepted analysis publication.
#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
pub(super) enum AnalysisAdmission {
    /// The compiler admitted the current editor state.
    Admitted,
    /// The compiler rejected the current editor state and published diagnostics.
    Rejected,
}

/// Cancellation proof ceiling advertised by this process leaf.
#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
enum RequestCancellation {
    /// The worker finishes an in-flight API call, but its superseded result is
    /// discarded and the JSON-RPC request receives `RequestCancelled`.
    ResultSuppression,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct ExplanationParams {
    subject: SubjectSelector,
}

/// Closed transport spelling of [`PortableWorkspaceSubjectSelector`].
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
pub(super) enum SubjectSelector {
    /// Retained logical source document.
    Source { source_id: String },
    /// Admitted semantic source component.
    Component { component_id: String },
    /// Selected module release.
    Module { module_id: String },
    /// Canonically owned declaration.
    Declaration {
        owner_module_id: String,
        namespace_id: String,
        declaration_id: String,
    },
    /// Resolved import path.
    Import {
        importer_module_id: String,
        upstream_module_id: String,
        upstream_release_id: String,
        export_id: String,
        source_namespace_id: String,
        local_namespace_id: String,
    },
    /// Applied extension.
    Extension {
        owner_module_id: String,
        extension_namespace_id: String,
        target_namespace_id: String,
        target_declaration_id: String,
    },
    /// Effective declaration policy.
    Policy {
        owner_module_id: String,
        namespace_id: String,
        declaration_id: String,
    },
    /// Complete responsibility closure.
    ResponsibilityClosure,
    /// Complete optional organizational translation closure.
    OrganizationalTranslationClosure,
    /// Complete explicit experience-composition closure.
    ExperienceComposition,
    /// Selected logical or release-bound resource.
    Resource {
        owner_module_id: String,
        resource_id: String,
    },
    /// Exact resolution lock.
    ResolutionLock,
    /// Exact universe lock.
    UniverseLock,
    /// Complete workspace semantic IR.
    Workspace { workspace_id: String },
}

impl From<SubjectSelector> for PortableWorkspaceSubjectSelector {
    fn from(selector: SubjectSelector) -> Self {
        match selector {
            SubjectSelector::Source { source_id } => Self::Source { source_id },
            SubjectSelector::Component { component_id } => Self::Component { component_id },
            SubjectSelector::Module { module_id } => Self::Module { module_id },
            SubjectSelector::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            },
            SubjectSelector::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            } => Self::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            },
            SubjectSelector::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            } => Self::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            },
            SubjectSelector::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            },
            SubjectSelector::ResponsibilityClosure => Self::ResponsibilityClosure,
            SubjectSelector::OrganizationalTranslationClosure => {
                Self::OrganizationalTranslationClosure
            }
            SubjectSelector::ExperienceComposition => Self::ExperienceComposition,
            SubjectSelector::Resource {
                owner_module_id,
                resource_id,
            } => Self::Resource {
                owner_module_id,
                resource_id,
            },
            SubjectSelector::ResolutionLock => Self::ResolutionLock,
            SubjectSelector::UniverseLock => Self::UniverseLock,
            SubjectSelector::Workspace { workspace_id } => Self::Workspace { workspace_id },
        }
    }
}

impl From<&PortableWorkspaceSubject> for SubjectSelector {
    fn from(subject: &PortableWorkspaceSubject) -> Self {
        match subject.selector() {
            PortableWorkspaceSubjectSelector::Source { source_id } => Self::Source { source_id },
            PortableWorkspaceSubjectSelector::Component { component_id } => {
                Self::Component { component_id }
            }
            PortableWorkspaceSubjectSelector::Module { module_id } => Self::Module { module_id },
            PortableWorkspaceSubjectSelector::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            },
            PortableWorkspaceSubjectSelector::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            } => Self::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            },
            PortableWorkspaceSubjectSelector::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            } => Self::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            },
            PortableWorkspaceSubjectSelector::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            },
            PortableWorkspaceSubjectSelector::ResponsibilityClosure => Self::ResponsibilityClosure,
            PortableWorkspaceSubjectSelector::OrganizationalTranslationClosure => {
                Self::OrganizationalTranslationClosure
            }
            PortableWorkspaceSubjectSelector::ExperienceComposition => Self::ExperienceComposition,
            PortableWorkspaceSubjectSelector::Resource {
                owner_module_id,
                resource_id,
            } => Self::Resource {
                owner_module_id,
                resource_id,
            },
            PortableWorkspaceSubjectSelector::ResolutionLock => Self::ResolutionLock,
            PortableWorkspaceSubjectSelector::UniverseLock => Self::UniverseLock,
            PortableWorkspaceSubjectSelector::Workspace { workspace_id } => {
                Self::Workspace { workspace_id }
            }
        }
    }
}

/// One exact source-change transition.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
enum Change {
    Added { current: Fact },
    Modified { prior: Fact, current: Fact },
    Removed { prior: Fact },
}

/// One exact comparison root.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct ChangeRoot {
    root_digest: String,
    subject: SubjectSelector,
    change: Change,
}

impl From<&PortableWorkspaceChangeRoot> for ChangeRoot {
    fn from(root: &PortableWorkspaceChangeRoot) -> Self {
        let change = match root.kind() {
            PortableWorkspaceChangeKind::Added => Change::Added {
                current: Fact::from(
                    root.current_fact()
                        .expect("an admitted added root carries a current fact"),
                ),
            },
            PortableWorkspaceChangeKind::Modified => Change::Modified {
                prior: Fact::from(
                    root.prior_fact()
                        .expect("an admitted modified root carries a prior fact"),
                ),
                current: Fact::from(
                    root.current_fact()
                        .expect("an admitted modified root carries a current fact"),
                ),
            },
            PortableWorkspaceChangeKind::Removed => Change::Removed {
                prior: Fact::from(
                    root.prior_fact()
                        .expect("an admitted removed root carries a prior fact"),
                ),
            },
        };
        Self {
            root_digest: root.root_digest().to_owned(),
            subject: SubjectSelector::from(root.subject()),
            change,
        }
    }
}

/// One bounded page of exact comparison roots.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
pub(super) struct ChangeRootPage {
    entries: Vec<ChangeRoot>,
    next_offset: Option<u32>,
}

impl From<&PortableWorkspaceChangeRootPage> for ChangeRootPage {
    fn from(page: &PortableWorkspaceChangeRootPage) -> Self {
        Self {
            entries: page.entries().iter().map(ChangeRoot::from).collect(),
            next_offset: page.next_offset(),
        }
    }
}

/// One current causal subject and its root-backed reachability state.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct Impact {
    subject: SubjectSelector,
    state: ImpactState,
    contributing_root_count: u32,
}

impl From<&PortableWorkspaceImpact> for Impact {
    fn from(impact: &PortableWorkspaceImpact) -> Self {
        Self {
            subject: SubjectSelector::from(impact.subject()),
            state: ImpactState::from(impact.state()),
            contributing_root_count: impact.contributing_root_count(),
        }
    }
}

/// Closed reachability classification.
#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
enum ImpactState {
    Affected,
    Unaffected,
}

impl From<PortableWorkspaceImpactState> for ImpactState {
    fn from(state: PortableWorkspaceImpactState) -> Self {
        match state {
            PortableWorkspaceImpactState::Affected => Self::Affected,
            PortableWorkspaceImpactState::Unaffected => Self::Unaffected,
        }
    }
}

/// One bounded page of the complete current impact partition.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
pub(super) struct ImpactPage {
    entries: Vec<Impact>,
    next_offset: Option<u32>,
}

impl From<&PortableWorkspaceImpactPage> for ImpactPage {
    fn from(page: &PortableWorkspaceImpactPage) -> Self {
        Self {
            entries: page.entries().iter().map(Impact::from).collect(),
            next_offset: page.next_offset(),
        }
    }
}

/// API-owned typed fact projected without collapsing digest domains.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
enum Fact {
    SourceContent {
        source_path: String,
        source_digest: String,
        byte_len: u32,
    },
    Component {
        component_digest: String,
    },
    Module {
        module_digest: String,
    },
    Declaration {
        value_digest: String,
    },
    ImportSurface {
        surface_digest: String,
    },
    Extension {
        value_digest: String,
    },
    EffectivePolicy {
        value_digest: String,
    },
    ResponsibilityClosure {
        closure_digest: String,
    },
    OrganizationalTranslationClosure {
        closure_digest: String,
    },
    ExperienceComposition {
        source_digest: String,
    },
    Resource {
        value_digest: String,
    },
    ResolutionLock {
        resolution_digest: String,
    },
    UniverseLock {
        universe_digest: String,
    },
    SemanticIr {
        workspace_semantic_ir_digest: String,
    },
}

impl From<&PortableWorkspaceFact> for Fact {
    fn from(fact: &PortableWorkspaceFact) -> Self {
        match fact {
            PortableWorkspaceFact::SourceContent {
                source_path,
                source_digest,
                byte_len,
            } => Self::SourceContent {
                source_path: source_path.clone(),
                source_digest: source_digest.clone(),
                byte_len: *byte_len,
            },
            PortableWorkspaceFact::Component { component_digest } => Self::Component {
                component_digest: component_digest.clone(),
            },
            PortableWorkspaceFact::Module { module_digest } => Self::Module {
                module_digest: module_digest.clone(),
            },
            PortableWorkspaceFact::Declaration { value_digest } => Self::Declaration {
                value_digest: value_digest.clone(),
            },
            PortableWorkspaceFact::ImportSurface { surface_digest } => Self::ImportSurface {
                surface_digest: surface_digest.clone(),
            },
            PortableWorkspaceFact::Extension { value_digest } => Self::Extension {
                value_digest: value_digest.clone(),
            },
            PortableWorkspaceFact::EffectivePolicy { value_digest } => Self::EffectivePolicy {
                value_digest: value_digest.clone(),
            },
            PortableWorkspaceFact::ResponsibilityClosure { closure_digest } => {
                Self::ResponsibilityClosure {
                    closure_digest: closure_digest.clone(),
                }
            }
            PortableWorkspaceFact::OrganizationalTranslationClosure { closure_digest } => {
                Self::OrganizationalTranslationClosure {
                    closure_digest: closure_digest.clone(),
                }
            }
            PortableWorkspaceFact::ExperienceComposition { source_digest } => {
                Self::ExperienceComposition {
                    source_digest: source_digest.clone(),
                }
            }
            PortableWorkspaceFact::Resource { value_digest } => Self::Resource {
                value_digest: value_digest.clone(),
            },
            PortableWorkspaceFact::ResolutionLock { resolution_digest } => Self::ResolutionLock {
                resolution_digest: resolution_digest.clone(),
            },
            PortableWorkspaceFact::UniverseLock { universe_digest } => Self::UniverseLock {
                universe_digest: universe_digest.clone(),
            },
            PortableWorkspaceFact::SemanticIr {
                workspace_semantic_ir_digest,
            } => Self::SemanticIr {
                workspace_semantic_ir_digest: workspace_semantic_ir_digest.clone(),
            },
        }
    }
}

/// Semantic owner/provenance classification for one proof fact.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
enum Provenance {
    Source {
        source_id: String,
    },
    ModuleRelease {
        module_id: String,
        release_id: String,
        module_digest: String,
    },
    Import {
        importer_module_id: String,
        upstream_module_id: String,
        upstream_release_id: String,
        export_id: String,
        source_namespace_id: String,
        local_namespace_id: String,
    },
    Extension {
        owner_module_id: String,
        extension_namespace_id: String,
        target_namespace_id: String,
        target_declaration_id: String,
    },
    Compilation,
}

impl From<&PortableWorkspaceProvenance> for Provenance {
    fn from(provenance: &PortableWorkspaceProvenance) -> Self {
        match provenance {
            PortableWorkspaceProvenance::Source { source_id } => Self::Source {
                source_id: source_id.clone(),
            },
            PortableWorkspaceProvenance::ModuleRelease {
                module_id,
                release_id,
                module_digest,
            } => Self::ModuleRelease {
                module_id: module_id.clone(),
                release_id: release_id.clone(),
                module_digest: module_digest.clone(),
            },
            PortableWorkspaceProvenance::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            } => Self::Import {
                importer_module_id: importer_module_id.clone(),
                upstream_module_id: upstream_module_id.clone(),
                upstream_release_id: upstream_release_id.clone(),
                export_id: export_id.clone(),
                source_namespace_id: source_namespace_id.clone(),
                local_namespace_id: local_namespace_id.clone(),
            },
            PortableWorkspaceProvenance::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            } => Self::Extension {
                owner_module_id: owner_module_id.clone(),
                extension_namespace_id: extension_namespace_id.clone(),
                target_namespace_id: target_namespace_id.clone(),
                target_declaration_id: target_declaration_id.clone(),
            },
            PortableWorkspaceProvenance::Compilation => Self::Compilation,
        }
    }
}

/// Deterministic compiler rule that produced one causal step.
#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
enum DerivationRule {
    SourceAdmission,
    ComponentAdmission,
    ModuleSealing,
    DeclarationLinking,
    ImportLinking,
    ExtensionApplication,
    PolicyComposition,
    ResponsibilityLowering,
    OrganizationalTranslation,
    ExperienceComposition,
    ResourceSelection,
    ResolutionLocking,
    UniverseLocking,
    SemanticIrSealing,
}

impl From<PortableWorkspaceDerivationRule> for DerivationRule {
    fn from(rule: PortableWorkspaceDerivationRule) -> Self {
        match rule {
            PortableWorkspaceDerivationRule::SourceAdmission => Self::SourceAdmission,
            PortableWorkspaceDerivationRule::ComponentAdmission => Self::ComponentAdmission,
            PortableWorkspaceDerivationRule::ModuleSealing => Self::ModuleSealing,
            PortableWorkspaceDerivationRule::DeclarationLinking => Self::DeclarationLinking,
            PortableWorkspaceDerivationRule::ImportLinking => Self::ImportLinking,
            PortableWorkspaceDerivationRule::ExtensionApplication => Self::ExtensionApplication,
            PortableWorkspaceDerivationRule::PolicyComposition => Self::PolicyComposition,
            PortableWorkspaceDerivationRule::ResponsibilityLowering => Self::ResponsibilityLowering,
            PortableWorkspaceDerivationRule::OrganizationalTranslation => {
                Self::OrganizationalTranslation
            }
            PortableWorkspaceDerivationRule::ExperienceComposition => Self::ExperienceComposition,
            PortableWorkspaceDerivationRule::ResourceSelection => Self::ResourceSelection,
            PortableWorkspaceDerivationRule::ResolutionLocking => Self::ResolutionLocking,
            PortableWorkspaceDerivationRule::UniverseLocking => Self::UniverseLocking,
            PortableWorkspaceDerivationRule::SemanticIrSealing => Self::SemanticIrSealing,
        }
    }
}

/// One content-addressed fact on a causal witness.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct CausalStep {
    node_digest: String,
    subject: SubjectSelector,
    provenance: Provenance,
    rule: DerivationRule,
    fact: Fact,
}

impl From<&PortableWorkspaceCausalStep> for CausalStep {
    fn from(step: &PortableWorkspaceCausalStep) -> Self {
        Self {
            node_digest: step.node_digest().to_owned(),
            subject: SubjectSelector::from(step.subject()),
            provenance: Provenance::from(step.provenance()),
            rule: DerivationRule::from(step.rule()),
            fact: Fact::from(step.fact()),
        }
    }
}

/// Proof DAG selected for one causal witness.
#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
enum ExplanationFrontier {
    Predecessor,
    Current,
}

impl From<PortableWorkspaceExplanationFrontier> for ExplanationFrontier {
    fn from(frontier: PortableWorkspaceExplanationFrontier) -> Self {
        match frontier {
            PortableWorkspaceExplanationFrontier::Predecessor => Self::Predecessor,
            PortableWorkspaceExplanationFrontier::Current => Self::Current,
        }
    }
}

/// Deterministically selected causal witness.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct CausalExplanation {
    root: ChangeRoot,
    frontier: ExplanationFrontier,
    steps: Vec<CausalStep>,
}

impl From<&PortableWorkspaceCausalExplanation> for CausalExplanation {
    fn from(causal: &PortableWorkspaceCausalExplanation) -> Self {
        Self {
            root: ChangeRoot::from(causal.root()),
            frontier: ExplanationFrontier::from(causal.frontier()),
            steps: causal.steps().iter().map(CausalStep::from).collect(),
        }
    }
}

/// Closed explanation state.
#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
enum ExplanationState {
    NotPresent,
    Unaffected,
    Affected,
}

impl From<PortableWorkspaceExplanationState> for ExplanationState {
    fn from(state: PortableWorkspaceExplanationState) -> Self {
        match state {
            PortableWorkspaceExplanationState::NotPresent => Self::NotPresent,
            PortableWorkspaceExplanationState::Unaffected => Self::Unaffected,
            PortableWorkspaceExplanationState::Affected => Self::Affected,
        }
    }
}

/// Result of one typed causal explanation request.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
pub(super) struct Explanation {
    subject: SubjectSelector,
    state: ExplanationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "protocol-codegen", schemars(with = "CausalExplanation"))]
    causal: Option<CausalExplanation>,
}

impl From<&PortableWorkspaceExplanation> for Explanation {
    fn from(explanation: &PortableWorkspaceExplanation) -> Self {
        Self {
            subject: SubjectSelector::from(explanation.subject()),
            state: ExplanationState::from(explanation.state()),
            causal: explanation.causal().map(CausalExplanation::from),
        }
    }
}

#[cfg(test)]
#[path = "../tests/internal/protocol.rs"]
mod tests;

#[cfg(feature = "protocol-codegen")]
pub(super) mod schema;
