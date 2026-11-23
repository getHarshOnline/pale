//! Compile workspace documents supplied as logical labels and bytes.
//!
//! Use [`PaleApi::compile_portable_workspace`] for workspace validation,
//! authoring queries, and comparisons without filesystem access. To inspect a
//! selected project's effective configuration, pass its result to
//! [`PaleApi::compile_effective_profile`].

use std::path::Path;
use std::sync::Arc;

mod comparison_subjects;
mod derivation;
mod impact_roots;
mod inspection;
mod organization;
pub use organization::{
    OrganizationAttribution, OrganizationAxes, OrganizationCommunicationResolution,
    OrganizationCommunicationResolutionHandle, OrganizationComparisonSide,
    OrganizationConsumerAcceptanceAnswer, OrganizationCorrespondence,
    OrganizationCorrespondenceCursor, OrganizationCorrespondenceState, OrganizationCursor,
    OrganizationDerivedClass, OrganizationDerivedSource, OrganizationDetailCursor,
    OrganizationDetailPage, OrganizationDetailParent, OrganizationDetailRows,
    OrganizationDisclosure, OrganizationEffectCeiling, OrganizationEndpointLookup,
    OrganizationEntry, OrganizationEvidenceRequirement, OrganizationFactState,
    OrganizationGenealogyHandle, OrganizationGenealogyLink, OrganizationGroupRef,
    OrganizationInspectionCoordinate, OrganizationInterval, OrganizationLensRight,
    OrganizationNativeReadBackAnswer, OrganizationNode, OrganizationPage,
    OrganizationPlacementAnswer, OrganizationProjectionClass, OrganizationProviderAcceptanceAnswer,
    OrganizationProviderCompletionAnswer, OrganizationRecordHandle, OrganizationRef,
    OrganizationRelationClass, OrganizationRelationLocality, OrganizationResidence,
    OrganizationResidenceClass, OrganizationResourceAnswer, OrganizationResponsibility,
    OrganizationResponsibilityLifecycle, OrganizationResponsibilityPlacement,
    OrganizationResponsibilityProjection, OrganizationResponsibilityProjectionHandle,
    OrganizationRoleRef, OrganizationRow, OrganizationScopeRef, OrganizationServiceAssignment,
    OrganizationServiceAssignmentHandle, OrganizationServiceRef, OrganizationSettlementAnswer,
    OrganizationSourceCursor, OrganizationSourceMapping, OrganizationSourcePage,
    OrganizationStructuralKind, OrganizationStructuralSelection, OrganizationTreeRef,
    RecipientQualifiedOrganizationCorrespondence,
    RecipientQualifiedOrganizationCorrespondenceCursor,
    RecipientQualifiedOrganizationCorrespondenceState, RecipientQualifiedOrganizationCursor,
    RecipientQualifiedOrganizationEntry, RecipientQualifiedOrganizationPage,
    RecipientQualifiedOrganizationRecordHandle, RecipientQualifiedOrganizationSourceCursor,
    RecipientQualifiedOrganizationSourceMapping, RecipientQualifiedOrganizationSourcePage,
    MAX_ORGANIZATION_COPY_BYTES,
};
#[cfg(test)]
#[path = "../tests/internal/portable_workspace/inspection_performance.rs"]
mod inspection_performance;
mod projection;
mod proof_dependencies;
mod proof_dependents;
mod recipient_inspection;
pub use comparison_subjects::{
    PortableWorkspaceComparisonSubjectCursor, PortableWorkspaceComparisonSubjectPage,
    MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES,
};
pub use derivation::{
    InvalidPortableWorkspaceSubjectSelector, PortableWorkspaceCausalExplanation,
    PortableWorkspaceCausalStep, PortableWorkspaceChangeKind, PortableWorkspaceChangeRoot,
    PortableWorkspaceChangeRootPage, PortableWorkspaceComparison,
    PortableWorkspaceComparisonSummary, PortableWorkspaceDerivationRule,
    PortableWorkspaceExplanation, PortableWorkspaceExplanationFrontier,
    PortableWorkspaceExplanationState, PortableWorkspaceFact, PortableWorkspaceImpact,
    PortableWorkspaceImpactPage, PortableWorkspaceImpactState, PortableWorkspaceProvenance,
    PortableWorkspaceSubject, PortableWorkspaceSubjectComparison, PortableWorkspaceSubjectKind,
    PortableWorkspaceSubjectSelector, PortableWorkspaceSubjectValues,
    PortableWorkspaceValueComparison,
};
pub use impact_roots::{
    PortableWorkspaceContributingRootsCursor, PortableWorkspaceContributingRootsPage,
    PortableWorkspaceContributingRootsState,
};
pub use inspection::{
    PortableWorkspaceSubjectInspection, PortableWorkspaceSubjectInspectionCursor,
};
pub use proof_dependencies::{
    PortableWorkspaceDependencyCursor, PortableWorkspaceDependencyPage,
    MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES,
};
pub use proof_dependents::{
    PortableWorkspaceDependentCursor, PortableWorkspaceDependentPage,
    MAX_DEPENDENT_PAGE_INPUT_COMPARISONS,
};
pub use recipient_inspection::{
    RecipientQualifiedComparisonSubjectCursor, RecipientQualifiedComparisonSubjectPage,
    RecipientQualifiedContributingRootsCursor, RecipientQualifiedContributingRootsPage,
    RecipientQualifiedDependencyCursor, RecipientQualifiedDependencyPage,
    RecipientQualifiedSubjectInspection, RecipientQualifiedSubjectInspectionCursor,
    RecipientQualifiedWorkspaceComparisonInspection,
    RecipientQualifiedWorkspaceComparisonOwnerMismatch, RecipientQualifiedWorkspacePairSummary,
};

use crate::contracts::schema_violation_projection;
use crate::error::{ApiError, FailureSpecification};
use crate::{AuthoringSourceLocation, PaleApi};
use pale_application::workspace::{
    AdmittedWorkspaceInspection as ApplicationAdmittedWorkspaceInspection,
    CapturedWorkspaceDocument, CapturedWorkspaceSource, PortableWorkspaceFailure,
    PortableWorkspaceFailureClass, PortableWorkspaceUniverse, WorkspaceError,
    WorkspaceInspectionAccessError as ApplicationWorkspaceInspectionAccessError,
    WorkspaceInspectionBudgetExceeded,
};
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticFactV1Wire, DiagnosticRetryV1,
    DiagnosticSchemaCoordinateV1Wire, DiagnosticSourceV1Wire, DiagnosticStageV1,
    DiagnosticSubjectV1Wire, RecordSchemaIdentity, SchemaDigest, SchemaId,
};

/// One untrusted source document supplied to portable workspace compilation.
///
/// [`Self::new`] does not parse or admit the label or bytes. The complete
/// request is validated together by [`PaleApi::compile_portable_workspace`]
/// so every failure receives the same bounded [`crate::DiagnosticV1`].
pub struct PortableWorkspaceDocument {
    label: String,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for PortableWorkspaceDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceDocument")
            .field("label_byte_len", &self.label.len())
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}

impl PortableWorkspaceDocument {
    /// Capture one confined logical relative path and its complete bytes.
    ///
    /// The label carries no host-path capability. Validation remains deferred
    /// so request failures share one structured diagnostic boundary.
    #[must_use]
    pub fn new(label: String, bytes: Vec<u8>) -> Self {
        Self { label, bytes }
    }
}

/// Owned source request for one path-free sovereign workspace compilation.
///
/// The request carries logical relative paths and bytes only. It has no filesystem,
/// Package-distribution, machine, provider, resident-runtime, installation,
/// activation, or native-effect capability.
pub struct PortableWorkspaceRequest {
    root_manifest: String,
    documents: Vec<PortableWorkspaceDocument>,
}

impl std::fmt::Debug for PortableWorkspaceRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceRequest")
            .field("root_manifest_byte_len", &self.root_manifest.len())
            .field("document_count", &self.documents.len())
            .finish()
    }
}

impl PortableWorkspaceRequest {
    /// Bind untrusted captured documents to one logical root manifest.
    ///
    /// Admission is intentionally deferred to
    /// [`PaleApi::compile_portable_workspace`] so duplicate labels, missing
    /// roots, bounds, syntax, schema, linking, policy, and owner-proof failures
    /// share one typed diagnostic path.
    #[must_use]
    pub fn new(root_manifest: String, documents: Vec<PortableWorkspaceDocument>) -> Self {
        Self {
            root_manifest,
            documents,
        }
    }

    /// Consume the public request into the application-owned capture boundary.
    ///
    /// Keeping this conversion here gives portable workspace compilation and
    /// complete one-shot compilation one ingress implementation. Neither path
    /// can silently acquire a different label, count, or source-byte policy.
    pub(crate) fn into_captured_source(
        self,
    ) -> Result<CapturedWorkspaceSource, PortableWorkspaceFailure> {
        let Self {
            root_manifest,
            documents,
        } = self;
        CapturedWorkspaceSource::validate_document_count(documents.len())?;
        let root_manifest = CapturedWorkspaceSource::admit_root_manifest(root_manifest)?;
        let documents = documents
            .into_iter()
            .map(|document| CapturedWorkspaceDocument::new(document.label, document.bytes))
            .collect::<Result<Vec<_>, _>>()?;
        CapturedWorkspaceSource::new(root_manifest, documents)
    }
}

/// Opaque admitted handle for one portable workspace universe.
///
/// This handle is neither the complete one-shot Package compilation nor a
/// resident compiled generation. It retains only the admitted path-free
/// workspace universe; source bytes were consumed and dropped before return.
#[derive(Clone)]
pub struct PortableWorkspaceCompilation {
    universe: Arc<PortableWorkspaceUniverse>,
    summary: PortableWorkspaceCompilationSummary,
    pub(crate) diagnostic_identity: RecordSchemaIdentity,
}

/// Recipient-qualified access to one exact retained workspace compilation.
///
/// The application issues the admission object. The SDK only verifies that it
/// belongs to the same retained compilation and projects admitted results. This
/// type accepts no recipient fields, authority tokens, or callbacks. Every query,
/// including every continuation, returns to the application authority owner.
/// Live acquisition remains unavailable until the authenticated resident route
/// can return this opaque handle; callers cannot create or self-grant one.
#[derive(Clone)]
pub struct RecipientQualifiedWorkspaceInspection {
    compilation: PortableWorkspaceCompilation,
    admitted: ApplicationAdmittedWorkspaceInspection,
}

impl std::fmt::Debug for RecipientQualifiedWorkspaceInspection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecipientQualifiedWorkspaceInspection")
            .finish_non_exhaustive()
    }
}

/// An application-issued inspection does not retain the SDK owner it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("recipient-qualified inspection does not belong to the supplied SDK owner")]
pub struct RecipientQualifiedWorkspaceOwnerMismatch;

/// Failure from recipient admission or the already-bounded SDK projection.
#[derive(Debug, thiserror::Error)]
pub enum RecipientQualifiedWorkspaceInspectionError {
    /// The application authority owner denied this call or exhausted its shared budget.
    #[error(transparent)]
    Access(#[from] RecipientQualifiedWorkspaceAccessError),
    /// The admitted compiler call completed, but the SDK projection rejected its input or output.
    #[error(transparent)]
    Projection(#[from] ApiError),
}

/// Why recipient-qualified workspace access stopped before publication.
///
/// This projection preserves the actionable refusal class without exposing the
/// application crate that owns admission and aggregate accounting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RecipientQualifiedWorkspaceAccessError {
    /// Recipient disclosure or the captured authority generation was refused.
    #[error("workspace inspection is not admitted for this recipient")]
    Denied,
    /// The aggregate native/query work ceiling was exhausted.
    #[error("workspace inspection work budget is exhausted")]
    WorkBudgetExhausted,
    /// The aggregate copied-output ceiling was exhausted.
    #[error("workspace inspection copied-byte budget is exhausted")]
    CopiedByteBudgetExhausted,
    /// The aggregate encoded-delivery ceiling was exhausted.
    #[error("workspace inspection encoded-byte budget is exhausted")]
    EncodedByteBudgetExhausted,
    /// The inspection's monotonic deadline expired.
    #[error("workspace inspection deadline has expired")]
    DeadlineExpired,
}

impl From<ApplicationWorkspaceInspectionAccessError> for RecipientQualifiedWorkspaceAccessError {
    fn from(error: ApplicationWorkspaceInspectionAccessError) -> Self {
        match error {
            ApplicationWorkspaceInspectionAccessError::Denied(_) => Self::Denied,
            ApplicationWorkspaceInspectionAccessError::Budget(
                WorkspaceInspectionBudgetExceeded::Work,
            ) => Self::WorkBudgetExhausted,
            ApplicationWorkspaceInspectionAccessError::Budget(
                WorkspaceInspectionBudgetExceeded::CopiedBytes,
            ) => Self::CopiedByteBudgetExhausted,
            ApplicationWorkspaceInspectionAccessError::Budget(
                WorkspaceInspectionBudgetExceeded::EncodedBytes,
            ) => Self::EncodedByteBudgetExhausted,
            ApplicationWorkspaceInspectionAccessError::Budget(
                WorkspaceInspectionBudgetExceeded::Deadline,
            ) => Self::DeadlineExpired,
        }
    }
}

impl From<WorkspaceInspectionBudgetExceeded> for RecipientQualifiedWorkspaceAccessError {
    fn from(error: WorkspaceInspectionBudgetExceeded) -> Self {
        Self::from(ApplicationWorkspaceInspectionAccessError::from(error))
    }
}

impl From<ApplicationWorkspaceInspectionAccessError>
    for RecipientQualifiedWorkspaceInspectionError
{
    fn from(error: ApplicationWorkspaceInspectionAccessError) -> Self {
        Self::Access(error.into())
    }
}

impl RecipientQualifiedWorkspaceInspection {
    /// Identity-and-count projection of the exact retained compilation.
    ///
    /// This summary grants no independent query or disclosure authority.
    ///
    /// # Errors
    /// Returns an authority or aggregate-budget failure before projection.
    pub fn summary(
        &self,
    ) -> Result<PortableWorkspaceCompilationSummary, RecipientQualifiedWorkspaceAccessError> {
        self.admitted
            .authorize_summary()
            .map_err(RecipientQualifiedWorkspaceAccessError::from)?;
        let summary = self.compilation.summary();
        self.admitted
            .budget()
            .charge_output(
                0,
                u64::try_from(summary.canonical_bytes().len()).unwrap_or(u64::MAX),
            )
            .map_err(RecipientQualifiedWorkspaceAccessError::from)?;
        Ok(summary)
    }

    pub(crate) fn authorize_profile_observation_association(
        &self,
    ) -> Result<(), RecipientQualifiedWorkspaceAccessError> {
        self.admitted
            .authorize_profile_observation_association()
            .map_err(RecipientQualifiedWorkspaceAccessError::from)
    }

    pub(crate) const fn compilation(&self) -> &PortableWorkspaceCompilation {
        &self.compilation
    }
}

/// Owned identity-and-count projection of one admitted portable workspace.
///
/// The projection is lossless for the public observations on
/// [`PortableWorkspaceCompilation`] while carrying neither its retained
/// semantic owner nor the source bytes consumed during admission. It is safe
/// to move across a language or transport boundary, but proves no resident
/// admission, runtime activation, provider observation, or native effect.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
pub struct PortableWorkspaceCompilationSummary {
    workspace_id: String,
    principal_universe_digest: String,
    semantic_ir_digest: String,
    admission_evidence_digest: String,
    source_document_count: u32,
    derivation_source_count: u32,
    derivation_node_count: u32,
    admission_evidence_count: u32,
    adopted_release_count: u32,
    package_release_count: u32,
    normative_pack_release_count: u32,
    /// Exact API-owned JCS projection of every preceding field.
    #[serde(skip)]
    #[cfg_attr(feature = "wire-schema", schemars(skip))]
    canonical_bytes: Vec<u8>,
}

/// One valid effective-profile context from an admitted workspace topology.
///
/// The two identities remain separate because a project and an execution
/// workspace have different meanings. This value describes desired topology
/// only; it proves no machine readiness, activation, or native state.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
pub struct PortableWorkspaceProfileContext {
    project_id: String,
    execution_workspace_id: String,
}

impl PortableWorkspaceProfileContext {
    /// Borrow the logical project identity.
    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Borrow the logical execution-workspace identity.
    #[must_use]
    pub fn execution_workspace_id(&self) -> &str {
        &self.execution_workspace_id
    }
}

impl PortableWorkspaceCompilationSummary {
    fn project(universe: &PortableWorkspaceUniverse) -> Result<Self, pale_core::canon::CanonError> {
        let mut summary = Self {
            workspace_id: universe.workspace_id().to_owned(),
            principal_universe_digest: universe.principal_universe_digest().to_owned(),
            semantic_ir_digest: universe.semantic_ir_digest().to_owned(),
            admission_evidence_digest: universe.admission_evidence_digest().to_owned(),
            source_document_count: universe.source_document_count(),
            derivation_source_count: universe.derivation_source_count(),
            derivation_node_count: universe.derivation_node_count(),
            admission_evidence_count: universe.admission_evidence_count(),
            adopted_release_count: universe.adopted_release_count(),
            package_release_count: universe.package_release_count(),
            normative_pack_release_count: universe.normative_pack_release_count(),
            canonical_bytes: Vec::new(),
        };
        summary.canonical_bytes = pale_core::canon::to_canonical_string(&summary)?.into_bytes();
        Ok(summary)
    }

    /// Exact canonical JSON projection of this complete summary.
    ///
    /// The API constructs these bytes from the same private fields exposed by
    /// the typed accessors. Consumer leaves can therefore transport the result
    /// without defining a second summary shape or canonicalization policy.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Workspace identity declared by the admitted root manifest.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    /// Principal-relative admitted-universe digest.
    #[must_use]
    pub fn principal_universe_digest(&self) -> &str {
        &self.principal_universe_digest
    }

    /// Verified workspace semantic-IR digest.
    #[must_use]
    pub fn semantic_ir_digest(&self) -> &str {
        &self.semantic_ir_digest
    }

    /// Exact owner-proof frontier consumed during admission.
    #[must_use]
    pub fn admission_evidence_digest(&self) -> &str {
        &self.admission_evidence_digest
    }

    /// Number of captured source documents consumed at ingress.
    #[must_use]
    pub const fn source_document_count(&self) -> u32 {
        self.source_document_count
    }

    /// Number of retained causal source coordinates.
    #[must_use]
    pub const fn derivation_source_count(&self) -> u32 {
        self.derivation_source_count
    }

    /// Number of retained causal derivation nodes.
    #[must_use]
    pub const fn derivation_node_count(&self) -> u32 {
        self.derivation_node_count
    }

    /// Number of exact owner-proof records consumed at admission.
    #[must_use]
    pub const fn admission_evidence_count(&self) -> u32 {
        self.admission_evidence_count
    }

    /// Number of adopted module releases in the admitted closure.
    #[must_use]
    pub const fn adopted_release_count(&self) -> u32 {
        self.adopted_release_count
    }

    /// Number of selected Package releases in the admitted closure.
    #[must_use]
    pub const fn package_release_count(&self) -> u32 {
        self.package_release_count
    }

    /// Number of selected attestor-scoped normative-Pack releases.
    #[must_use]
    pub const fn normative_pack_release_count(&self) -> u32 {
        self.normative_pack_release_count
    }
}

impl std::fmt::Debug for PortableWorkspaceCompilation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceCompilation")
            .field("workspace_id", &self.workspace_id())
            .field(
                "principal_universe_digest",
                &self.principal_universe_digest(),
            )
            .field("semantic_ir_digest", &self.semantic_ir_digest())
            .field("source_document_count", &self.source_document_count())
            .finish()
    }
}

impl PortableWorkspaceCompilation {
    /// Borrow the admitted application owner for another API-owned use case.
    ///
    /// This seam remains crate-private so no consumer can bypass the opaque
    /// façade or inspect application records directly. Profile compilation and
    /// portable comparison may consequently share one exact admitted universe
    /// without parsing the same source twice.
    pub(crate) fn application_universe(&self) -> &PortableWorkspaceUniverse {
        &self.universe
    }

    /// Share the admitted application owner with another façade use case.
    ///
    /// The handle remains crate-private: public consumers cannot unwrap it or
    /// bypass this façade's typed operations.
    pub(crate) fn application_universe_handle(&self) -> Arc<PortableWorkspaceUniverse> {
        Arc::clone(&self.universe)
    }

    /// Project every public identity and count into an owned, path-free value.
    ///
    /// The opaque compilation remains the semantic owner required by
    /// [`PaleApi::compare_portable_workspaces`]. This projection is for
    /// inspection only and cannot be converted back into an admitted handle.
    #[must_use]
    pub fn summary(&self) -> PortableWorkspaceCompilationSummary {
        self.summary.clone()
    }

    /// Return every declared project and execution-workspace pairing.
    ///
    /// Results preserve the compiler's canonical relationship order. The
    /// workspace grammar admits at most 4,096 such pairs, so this owned
    /// projection has a fixed input-derived ceiling. It does not inspect source
    /// text or derive relationships absent from the admitted topology.
    #[must_use]
    pub fn profile_contexts(&self) -> Vec<PortableWorkspaceProfileContext> {
        self.universe
            .profile_contexts()
            .map(
                |(project_id, execution_workspace_id)| PortableWorkspaceProfileContext {
                    project_id: project_id.to_owned(),
                    execution_workspace_id: execution_workspace_id.to_owned(),
                },
            )
            .collect()
    }

    /// Workspace identity declared by the admitted root manifest.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        self.universe.workspace_id()
    }

    /// Principal-relative admitted-universe digest.
    #[must_use]
    pub fn principal_universe_digest(&self) -> &str {
        self.universe.principal_universe_digest()
    }

    /// Verified workspace semantic-IR digest.
    #[must_use]
    pub fn semantic_ir_digest(&self) -> &str {
        self.universe.semantic_ir_digest()
    }

    /// Exact owner-proof frontier consumed during admission.
    #[must_use]
    pub fn admission_evidence_digest(&self) -> &str {
        self.universe.admission_evidence_digest()
    }

    /// Number of captured source documents consumed at ingress.
    #[must_use]
    pub fn source_document_count(&self) -> u32 {
        self.universe.source_document_count()
    }

    /// Number of retained causal source coordinates.
    #[must_use]
    pub fn derivation_source_count(&self) -> u32 {
        self.universe.derivation_source_count()
    }

    /// Number of retained causal derivation nodes.
    #[must_use]
    pub fn derivation_node_count(&self) -> u32 {
        self.universe.derivation_node_count()
    }

    /// Number of exact owner-proof records consumed at admission.
    #[must_use]
    pub fn admission_evidence_count(&self) -> u32 {
        self.universe.admission_evidence_count()
    }

    /// Number of adopted module releases in the admitted closure.
    #[must_use]
    pub fn adopted_release_count(&self) -> u32 {
        self.universe.adopted_release_count()
    }

    /// Number of selected Package releases in the admitted closure.
    #[must_use]
    pub fn package_release_count(&self) -> u32 {
        self.universe.package_release_count()
    }

    /// Number of selected attestor-scoped normative-Pack releases.
    #[must_use]
    pub fn normative_pack_release_count(&self) -> u32 {
        self.universe.normative_pack_release_count()
    }
}

impl PaleApi {
    /// Validate and compile the workspace documents supplied in `request`.
    ///
    /// Each document has a logical relative label and its complete bytes. The
    /// root manifest names one of those labels; none is opened as a filesystem
    /// path. The returned handle supports authoring queries and comparison, but
    /// is not a complete Package compilation or a resident generation.
    ///
    /// The Pale application validates source shape, links references,
    /// normalizes records, seals the semantic representation, and verifies
    /// identities and owner evidence. This API supplies no publisher or
    /// protected-delivery proofs.
    /// Workspaces that require neither can be admitted; otherwise admission
    /// stops at the first missing publisher proof. It cannot independently reach
    /// a later protected-delivery check or infer trust from a declaration.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the request is malformed, structurally invalid,
    /// semantically inconsistent, owner-proof blocked, or contradicted by an
    /// Pale application invariant. A logical source coordinate is retained only
    /// when the rejected input supplies one that is safe to expose.
    pub fn compile_portable_workspace(
        &self,
        request: PortableWorkspaceRequest,
    ) -> Result<PortableWorkspaceCompilation, ApiError> {
        let universe = compile_portable_request(&self.diagnostic_identity, request)?;
        let summary = PortableWorkspaceCompilationSummary::project(&universe)
            .map_err(|error| portable_summary_failure(&self.diagnostic_identity, error))?;
        Ok(PortableWorkspaceCompilation {
            universe: Arc::new(universe),
            summary,
            diagnostic_identity: self.diagnostic_identity.clone(),
        })
    }
}

pub(crate) fn portable_summary_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: pale_core::canon::CanonError,
) -> ApiError {
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: "portable_workspace_summary_failed",
            stage: DiagnosticStageV1::Internal,
            disposition: DiagnosticDispositionV1::InternalFailure,
            summary: "portable workspace summary could not be canonicalized",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::NotIndicated,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}

/// Admit and compile one public workspace request for every API use case.
pub(crate) fn compile_portable_request(
    identity: &pale_core::records::RecordSchemaIdentity,
    request: PortableWorkspaceRequest,
) -> Result<PortableWorkspaceUniverse, ApiError> {
    let source = request
        .into_captured_source()
        .map_err(|error| workspace_failure(identity, error))?;
    pale_application::workspace::compile_portable_workspace(source)
        .map_err(|error| workspace_failure(identity, error))
}

pub(crate) fn workspace_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: PortableWorkspaceFailure,
) -> ApiError {
    let classification = classify_portable_workspace_failure(error.class());
    let mut projection = WorkspaceFailureProjection::from_error(error.workspace_error());
    let correction = error.workspace_error().authoring_correction();
    let authoring_location = workspace_authoring_location(error.workspace_error()).or_else(|| {
        correction.as_ref().map(|evidence| {
            AuthoringSourceLocation::new(
                evidence.source_label().to_owned(),
                evidence.path().iter().map(Into::into).collect(),
            )
        })
    });
    let failure = ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: classification.code.as_str(),
            stage: classification.stage,
            disposition: classification.disposition,
            summary: projection.summary.unwrap_or(classification.summary),
            subject: projection.subject.take(),
            schema_coordinate: projection.schema_coordinate.take(),
            location: projection.location.take(),
            retry: classification.retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: classification.notes,
            help: if projection.help.is_empty() {
                classification.help
            } else {
                std::mem::take(&mut projection.help)
            },
            facts: std::mem::take(&mut projection.facts),
        },
    )
    .with_authoring_location(authoring_location);
    match correction.as_ref() {
        Some(evidence) => failure.with_authoring_correction(evidence),
        None => failure,
    }
}

fn workspace_authoring_location(error: &WorkspaceError) -> Option<AuthoringSourceLocation> {
    let location = error.authoring_location()?;
    Some(AuthoringSourceLocation::from_application(&location))
}

struct WorkspaceFailureClassification {
    code: PortableWorkspaceDiagnosticCode,
    stage: DiagnosticStageV1,
    disposition: DiagnosticDispositionV1,
    summary: &'static str,
    retry: DiagnosticRetryV1,
    notes: Vec<&'static str>,
    help: Vec<&'static str>,
}

/// Closed diagnostic-code vocabulary for the portable workspace use case.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PortableWorkspaceDiagnosticCode {
    /// Captured source cannot cross syntax or bounded-ingress admission.
    SourceRejected,
    /// A source-shaped record contradicts its selected contract.
    ContractRejected,
    /// Linked or normalized workspace meaning is inconsistent.
    SemanticsRejected,
    /// Semantic workspace membership exceeds its language ceiling.
    SemanticLimitExceeded,
    /// Source delivery omits an exact required component.
    SourceIncomplete,
    /// Exact owner-published evidence is unavailable.
    OwnerProofUnavailable,
    /// A selected release lifecycle state forbids new admission.
    ReleaseNotAdmissible,
    /// A Pale application invariant failed.
    InternalFailure,
}

impl PortableWorkspaceDiagnosticCode {
    /// Return the stable wire spelling for one closed code.
    const fn as_str(self) -> &'static str {
        match self {
            Self::SourceRejected => "workspace_source_rejected",
            Self::ContractRejected => "workspace_contract_rejected",
            Self::SemanticsRejected => "workspace_semantics_rejected",
            Self::SemanticLimitExceeded => "workspace_semantic_limit_exceeded",
            Self::SourceIncomplete => "workspace_source_incomplete",
            Self::OwnerProofUnavailable => "workspace_owner_proof_unavailable",
            Self::ReleaseNotAdmissible => "workspace_release_not_admissible",
            Self::InternalFailure => "workspace_internal_failure",
        }
    }
}

fn classify_portable_workspace_failure(
    class: PortableWorkspaceFailureClass,
) -> WorkspaceFailureClassification {
    let (code, stage, disposition, summary, retry, notes, help) = match class {
        PortableWorkspaceFailureClass::OwnerProofUnavailable => (
            PortableWorkspaceDiagnosticCode::OwnerProofUnavailable,
            DiagnosticStageV1::SemanticAdmission,
            DiagnosticDispositionV1::Blocked,
            "portable workspace admission requires owner-published evidence",
            DiagnosticRetryV1::AfterStateChange,
            vec![
                "The affected portable workspace remains unadmitted; no runtime or native effect was attempted.",
            ],
            vec!["Provide the exact owner-published proof required by this workspace."],
        ),
        PortableWorkspaceFailureClass::ReleaseNotAdmissible => (
            PortableWorkspaceDiagnosticCode::ReleaseNotAdmissible,
            DiagnosticStageV1::SemanticAdmission,
            DiagnosticDispositionV1::Blocked,
            "a selected workspace release is not admissible at its declared lifecycle state",
            DiagnosticRetryV1::AfterStateChange,
            vec!["The lifecycle ceiling closes only the affected workspace admission."],
            vec!["Select an owner-published release that is admissible for a new universe."],
        ),
        PortableWorkspaceFailureClass::SourceIncomplete => (
            PortableWorkspaceDiagnosticCode::SourceIncomplete,
            DiagnosticStageV1::SemanticAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "a selected source-delivery module omits a referenced source component",
            DiagnosticRetryV1::AfterCorrection,
            vec!["The compiler stopped before constructing an admitted workspace universe."],
            vec!["Supply the exact referenced component through the selected source delivery."],
        ),
        PortableWorkspaceFailureClass::InternalInvariant => (
            PortableWorkspaceDiagnosticCode::InternalFailure,
            DiagnosticStageV1::Internal,
            DiagnosticDispositionV1::InternalFailure,
            "portable workspace compilation contradicted a Pale application invariant",
            DiagnosticRetryV1::NotIndicated,
            Vec::new(),
            Vec::new(),
        ),
        PortableWorkspaceFailureClass::ContractRejected => (
            PortableWorkspaceDiagnosticCode::ContractRejected,
            DiagnosticStageV1::ContractValidation,
            DiagnosticDispositionV1::InvalidInput,
            "a workspace source document does not satisfy its selected contract",
            DiagnosticRetryV1::AfterCorrection,
            vec!["Contract rejection occurs before workspace semantic admission."],
            vec![
                "Correct the rejected source document at the reported coordinate, then compile again.",
            ],
        ),
        PortableWorkspaceFailureClass::SourceRejected => (
            PortableWorkspaceDiagnosticCode::SourceRejected,
            DiagnosticStageV1::WorkspaceLoading,
            DiagnosticDispositionV1::InvalidInput,
            "portable workspace source was rejected",
            DiagnosticRetryV1::AfterCorrection,
            vec!["No workspace meaning was admitted from the rejected source."],
            vec!["Correct the bounded source, syntax, or required member and compile again."],
        ),
        PortableWorkspaceFailureClass::SemanticLimitExceeded => (
            PortableWorkspaceDiagnosticCode::SemanticLimitExceeded,
            DiagnosticStageV1::SemanticAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "admitted workspace meaning exceeds a workspace-language bound",
            DiagnosticRetryV1::AfterCorrection,
            vec!["The bound is an admission ceiling, not a partial-compilation threshold."],
            vec!["Reduce the affected declared workspace set below its reported maximum."],
        ),
        PortableWorkspaceFailureClass::SemanticRejected => (
            PortableWorkspaceDiagnosticCode::SemanticsRejected,
            DiagnosticStageV1::SemanticAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "portable workspace meaning was rejected",
            DiagnosticRetryV1::AfterCorrection,
            vec![
                "The compiler rejected the complete semantic relation rather than normalizing it silently.",
            ],
            vec!["Correct the inconsistent workspace relationship and compile again."],
        ),
    };
    WorkspaceFailureClassification {
        code,
        stage,
        disposition,
        summary,
        retry,
        notes,
        help,
    }
}

#[derive(Default)]
struct WorkspaceFailureProjection {
    summary: Option<&'static str>,
    help: Vec<&'static str>,
    subject: Option<DiagnosticSubjectV1Wire>,
    location: Option<DiagnosticSourceV1Wire>,
    schema_coordinate: Option<DiagnosticSchemaCoordinateV1Wire>,
    facts: Vec<DiagnosticFactV1Wire>,
}

impl WorkspaceFailureProjection {
    fn from_error(error: &WorkspaceError) -> Self {
        match error {
            WorkspaceError::InvalidWorkspaceLock(source) => Self::from_lock_error(source),
            WorkspaceError::Input(_) | WorkspaceError::CapturedPrivateInput(_) => Self::default(),
            WorkspaceError::InputNotUtf8(path)
            | WorkspaceError::PackagePageHeader { path, .. } => Self {
                location: logical_location(path, None, None, None, None),
                ..Self::default()
            },
            WorkspaceError::TomlSyntax { coordinate, .. }
            | WorkspaceError::TomlToJson { coordinate, .. } => Self {
                location: coordinate_location(coordinate, None),
                ..Self::default()
            },
            WorkspaceError::Decode {
                coordinate,
                record,
                value_path,
                ..
            } => Self {
                location: coordinate_location(coordinate, Some(value_path.clone())),
                facts: vec![fact("record", record)],
                ..Self::default()
            },
            WorkspaceError::UnrepresentableDerivationSourceId {
                coordinate,
                field_path,
                family,
                combined_byte_len,
                ..
            } => Self {
                summary: Some(
                    "the record ID and source-family prefix cannot form a derivation source identity",
                ),
                help: vec![
                    "Shorten the identified record ID so its prefixed source identity can be represented.",
                ],
                location: coordinate_location(coordinate, Some(field_path.join("."))),
                facts: vec![
                    fact("source_family", family),
                    fact("combined_byte_length", &combined_byte_len.to_string()),
                ],
                ..Self::default()
            },
            WorkspaceError::SchemaCheck { coordinate, source } => {
                let mut violation = schema_violation_projection(source);
                let value_path = violation
                    .location
                    .take()
                    .and_then(|location| location.value_path);
                Self {
                    location: coordinate_location(coordinate, value_path),
                    schema_coordinate: schema_coordinate(source, violation.schema_path),
                    facts: violation.facts,
                    ..Self::default()
                }
            }
            WorkspaceError::CapturedDocumentTooLarge { observed, limit }
            | WorkspaceError::CapturedDocumentCountExceeded { observed, limit } => Self {
                facts: vec![
                    fact("observed", &observed.to_string()),
                    fact("maximum", &limit.to_string()),
                ],
                ..Self::default()
            },
            WorkspaceError::WorkspaceGraphLimit {
                kind,
                observed,
                limit,
            } => Self {
                facts: vec![
                    fact("limit", kind),
                    fact("observed", &observed.to_string()),
                    fact("maximum", &limit.to_string()),
                ],
                ..Self::default()
            },
            WorkspaceError::PublisherAdmissionUnavailable { publisher, release } => Self {
                subject: subject("publisher", publisher),
                facts: vec![fact("release", release)],
                ..Self::default()
            },
            WorkspaceError::ProtectedDeliveryAdmissionUnavailable { importer, path } => Self {
                subject: subject("module", importer),
                facts: vec![fact("import_path", path)],
                ..Self::default()
            },
            WorkspaceError::ReleaseStateNotAdmissible { module, release } => Self {
                subject: subject("release", release),
                facts: vec![fact("module", module)],
                ..Self::default()
            },
            WorkspaceError::SourceDeliveryUnavailable { module, component } => Self {
                subject: subject("module", module),
                facts: vec![fact("component", component)],
                ..Self::default()
            },
            WorkspaceError::WorkspaceDigestMismatch { kind, id } => Self {
                subject: subject("workspace_component", id),
                facts: vec![fact("digest_domain", kind)],
                ..Self::default()
            },
            WorkspaceError::DuplicateCapturedSource(label)
            | WorkspaceError::MissingCapturedSourceRoot(label) => Self {
                location: logical_location(Path::new(label), None, None, None, None),
                ..Self::default()
            },
            WorkspaceError::MissingReference { path, .. } => Self {
                location: logical_location(Path::new(path), None, None, None, None),
                ..Self::default()
            },
            WorkspaceError::UnknownProject(project) => Self {
                subject: subject("project", project),
                ..Self::default()
            },
            WorkspaceError::UnknownExecutionWorkspace(execution_workspace) => Self {
                subject: subject("execution_workspace", execution_workspace),
                ..Self::default()
            },
            WorkspaceError::ProjectExecutionWorkspaceMembership {
                project,
                execution_workspace,
            } => Self {
                subject: subject("project", project),
                facts: vec![fact("execution_workspace", execution_workspace)],
                ..Self::default()
            },
            WorkspaceError::MissingPackageVersion(package)
            | WorkspaceError::DuplicateRunPolicy(package)
            | WorkspaceError::DuplicateLifecycleBehavior(package) => Self {
                subject: subject("declaration", package),
                ..Self::default()
            },
            WorkspaceError::DuplicateLockedPackage {
                id,
                first_index,
                second_index,
            } => Self {
                subject: subject("package", id),
                facts: vec![
                    fact("first_index", &first_index.to_string()),
                    fact("second_index", &second_index.to_string()),
                ],
                ..Self::default()
            },
            WorkspaceError::MissingRunPolicy(run_policy) => Self {
                subject: subject("run_policy", run_policy),
                ..Self::default()
            },
            WorkspaceError::InvalidRunPolicy { id, .. } => Self {
                subject: subject("run_policy", id),
                ..Self::default()
            },
            WorkspaceError::InvalidLifecycleBehavior { id, .. } => Self {
                subject: subject("lifecycle_behavior", id),
                ..Self::default()
            },
            WorkspaceError::UnknownFamily { family, repository } => Self {
                subject: subject("repository", repository.as_ref()),
                facts: vec![fact("family", family)],
                ..Self::default()
            },
            WorkspaceError::UnknownRepositoryCommandRole(role) => Self {
                subject: subject("repository_role", role),
                ..Self::default()
            },
            WorkspaceError::ConflictingRepositoryCommand { role, command } => Self {
                subject: subject("repository_role", role),
                facts: vec![fact("command", command)],
                ..Self::default()
            },
            WorkspaceError::UnknownActivationRoot { repository }
            | WorkspaceError::DuplicateActivationRoot { repository }
            | WorkspaceError::UnknownAdditionalRoot { repository }
            | WorkspaceError::DuplicateAdditionalRoot { repository }
            | WorkspaceError::ActivationAdditionalRootOverlap { repository } => Self {
                subject: subject("repository", repository.as_ref()),
                ..Self::default()
            },
            WorkspaceError::SessionUnknownAuthority(authority) => Self {
                subject: subject("authority_root", authority),
                ..Self::default()
            },
            WorkspaceError::InvalidExecutionPathProviderWitness { execution_path, .. } => Self {
                subject: subject("execution_path", execution_path),
                ..Self::default()
            },
            WorkspaceError::ImportVisibilityWidening {
                source_label,
                upstream_module,
                export_id,
                export_visibility,
                import_ceiling,
                import_index,
                ..
            } => Self {
                subject: subject("export", &format!("{upstream_module}::{export_id}")),
                location: logical_location(Path::new(source_label), None, None, None, None).map(
                    |mut location| {
                        location.value_path =
                            Some(format!("module.imports[{import_index}].visibility_ceiling"));
                        location
                    },
                ),
                facts: vec![
                    fact("export_visibility", &export_visibility.to_string()),
                    fact("import_ceiling", &import_ceiling.to_string()),
                ],
                ..Self::default()
            },
            WorkspaceError::MissingParent(_)
            | WorkspaceError::InvalidDesiredTopology(_)
            | WorkspaceError::InvalidProfileIdentity { .. }
            | WorkspaceError::Canon(_)
            | WorkspaceError::UnresolvedRepository { .. }
            | WorkspaceError::UnresolvedAuthorityRoot { .. }
            | WorkspaceError::DuplicateRepository(_)
            | WorkspaceError::DuplicateProviderRepositoryProjection { .. }
            | WorkspaceError::NonAbsolutePath { .. }
            | WorkspaceError::NonDistinctPath(_)
            | WorkspaceError::EscapedReference(_)
            | WorkspaceError::InvalidCapturedSourceRoot
            | WorkspaceError::InputChanged(_)
            | WorkspaceError::GeneratedRecordSchema(_)
            | WorkspaceError::InvalidWorkspaceComponent(_)
            | WorkspaceError::InvalidWorkspaceModule(_)
            | WorkspaceError::ReleaseClosure(_)
            | WorkspaceError::MissingDistributionRoot(_)
            | WorkspaceError::IdentityMismatch { .. }
            | WorkspaceError::EmptyActivationRoots
            | WorkspaceError::ActivationRequiresSurfaceExecutable(_)
            | WorkspaceError::ActivationRequiresSessionState(_)
            | WorkspaceError::ActivationRequiresDurableEvidence(_)
            | WorkspaceError::ActivationRequiresNativeHarness { .. }
            | WorkspaceError::NativeHarnessProbe { .. }
            | WorkspaceError::NativeHarnessUtf8 { .. }
            | WorkspaceError::NativeHarnessVersionMismatch { .. }
            | WorkspaceError::UnsortedOrDuplicateNativeHarnessBindings
            | WorkspaceError::ReadingOrderNotConfined(_)
            | WorkspaceError::SurfaceExecutableMissing
            | WorkspaceError::SurfaceExecutableIdentity { .. }
            | WorkspaceError::SurfaceExecutableProbe { .. }
            | WorkspaceError::SurfaceExecutableIdentityJson { .. }
            | WorkspaceError::SurfaceExecutableIdentitySchema { .. }
            | WorkspaceError::SurfaceExecutableIdentityDecode { .. }
            | WorkspaceError::SurfaceExecutablePlatformMismatch { .. }
            | WorkspaceError::SessionStateNotAbsolute(_)
            | WorkspaceError::SessionStateNotFound(_)
            | WorkspaceError::SessionStateNotDirectory(_)
            | WorkspaceError::SessionStateNotConfined(_)
            | WorkspaceError::SessionStateUnsafePermissions(_)
            | WorkspaceError::DurableEvidenceNotAbsolute(_)
            | WorkspaceError::DurableEvidenceNotFound(_)
            | WorkspaceError::DurableEvidenceNotDirectory(_)
            | WorkspaceError::DurableEvidenceNotConfined(_)
            | WorkspaceError::DurableEvidenceUnsafePermissions(_)
            | WorkspaceError::MaterializationLineageMissing(_)
            | WorkspaceError::MaterializationLineageNotAbsolute(_)
            | WorkspaceError::MaterializationLineageNotFound(_)
            | WorkspaceError::MaterializationLineageNotDirectory(_)
            | WorkspaceError::MaterializationLineageNotConfined(_)
            | WorkspaceError::MaterializationLineageUnsafePermissions(_)
            | WorkspaceError::SurfaceExecutablePathMismatch { .. }
            | WorkspaceError::SurfaceExecutableVersionMismatch { .. }
            | WorkspaceError::SurfaceExecutableProtocolMismatch { .. }
            | WorkspaceError::ReadingOrderMissing(_)
            | WorkspaceError::ReadingOrderEscapesRoot { .. }
            | WorkspaceError::SessionPathMissing { .. }
            | WorkspaceError::SessionPathObservation { .. }
            | WorkspaceError::SessionPathNotUtf8 { .. }
            | WorkspaceError::DuplicateLanguageIdentity { .. }
            | WorkspaceError::NonCanonicalLanguageSet { .. }
            | WorkspaceError::WorkspaceSemanticIrDigestMismatch(_)
            | WorkspaceError::PortableObservationOverflow { .. }
            | WorkspaceError::ExportProjectionMismatch(_)
            | WorkspaceError::ResolutionLockMismatch(_)
            | WorkspaceError::UniverseLockMismatch(_)
            | WorkspaceError::NamespaceResolution(_)
            | WorkspaceError::VisibilityDenied(_)
            | WorkspaceError::OnwardPropagationDenied(_)
            | WorkspaceError::PolicyWidening(_)
            | WorkspaceError::ImportPolicyWidening { .. }
            | WorkspaceError::PolicyCombination(_)
            | WorkspaceError::InadmissibleDeclarationCycle(_)
            | WorkspaceError::PortableInputLeakage(_)
            | WorkspaceError::ResponsibilityDerivation(_)
            | WorkspaceError::OrganizationalTranslation(_)
            | WorkspaceError::MissingDirectionalService { .. }
            | WorkspaceError::InvalidDerivationReceipt(_)
            | WorkspaceError::DerivationSemanticMismatch => Self::default(),
        }
    }

    /// Explain lock rejection from typed details without exposing its rendered source.
    ///
    /// The application still owns failure disposition. No source filename is
    /// inferred from a lock kind: a workspace may select a different logical path.
    fn from_lock_error(error: &pale_core::records::InvalidWorkspaceLock) -> Self {
        use pale_core::records::InvalidWorkspaceLock;

        let (summary, help, facts) = match error {
            InvalidWorkspaceLock::DigestMismatch { kind } => (
                "The workspace lock digest does not match its contents.",
                "Use a lock sealed from the intended inputs; do not edit the digest alone.",
                vec![
                    fact("lock_kind", kind),
                    fact("violation", "digest_mismatch"),
                ],
            ),
            InvalidWorkspaceLock::SchemaCoordinate { kind } => (
                "The workspace lock uses an unsupported schema or version.",
                "Supply a lock that matches the selected workspace lock contract.",
                vec![
                    fact("lock_kind", kind),
                    fact("violation", "schema_coordinate"),
                ],
            ),
            InvalidWorkspaceLock::NonCanonical { kind } => (
                "The workspace lock entries are duplicated or out of order.",
                "Regenerate the lock with unique entries in canonical identity order.",
                vec![fact("lock_kind", kind), fact("violation", "entry_order")],
            ),
            InvalidWorkspaceLock::DuplicateModule(module) => (
                "The workspace lock repeats a module identity.",
                "Resolve the duplicate module selection before sealing the lock.",
                vec![
                    fact("module", module.as_ref()),
                    fact("violation", "duplicate_module"),
                ],
            ),
            InvalidWorkspaceLock::Limit {
                kind,
                observed,
                limit,
            } => (
                "The workspace lock exceeds its entry limit.",
                "Reduce the selected entries to the reported limit before sealing the lock.",
                vec![
                    fact("collection", kind),
                    fact("observed", &observed.to_string()),
                    fact("limit", &limit.to_string()),
                    fact("violation", "entry_limit"),
                ],
            ),
            InvalidWorkspaceLock::Canonical(_) => return Self::default(),
        };
        Self {
            summary: Some(summary),
            help: vec![help],
            facts,
            ..Self::default()
        }
    }
}

fn coordinate_location(
    coordinate: &pale_application::workspace::WorkspaceSourceCoordinate,
    value_path: Option<String>,
) -> Option<DiagnosticSourceV1Wire> {
    logical_location(
        coordinate.path(),
        coordinate
            .line()
            .and_then(|value| u32::try_from(value).ok()),
        coordinate
            .column()
            .and_then(|value| u32::try_from(value).ok()),
        coordinate
            .span()
            .and_then(|span| u32::try_from(span.start).ok()),
        coordinate
            .span()
            .and_then(|span| u32::try_from(span.end).ok()),
    )
    .map(|mut location| {
        location.value_path = value_path;
        location
    })
}

fn logical_location(
    path: &Path,
    line: Option<u32>,
    column: Option<u32>,
    byte_start: Option<u32>,
    byte_end: Option<u32>,
) -> Option<DiagnosticSourceV1Wire> {
    let label = path.to_str()?.strip_prefix("./").unwrap_or(path.to_str()?);
    pale_core::paths::ConfinedRelativePath::parse(label)?;
    (!label.is_empty()).then(|| DiagnosticSourceV1Wire {
        label: label.to_string(),
        line,
        column,
        byte_start,
        byte_end,
        value_path: None,
    })
}

fn schema_coordinate(
    error: &pale_contracts::SchemaError,
    schema_path: Option<String>,
) -> Option<DiagnosticSchemaCoordinateV1Wire> {
    let pale_contracts::SchemaError::Invalid(violations) = error else {
        return None;
    };
    let descriptor = pale_contracts::schema_descriptor(violations.schema_id()).ok()?;
    Some(DiagnosticSchemaCoordinateV1Wire {
        registry_id: descriptor.schema_id.to_string(),
        wire_schema_id: descriptor
            .wire_schema_id
            .and_then(|identity| SchemaId::parse(identity).ok()),
        schema_sha256: SchemaDigest::parse(descriptor.schema_sha256).ok()?,
        schema_path,
    })
}

fn fact(key: &str, value: &str) -> DiagnosticFactV1Wire {
    DiagnosticFactV1Wire {
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn subject(kind: &str, id: &str) -> Option<DiagnosticSubjectV1Wire> {
    (!kind.is_empty() && !id.is_empty()).then(|| DiagnosticSubjectV1Wire {
        kind: kind.to_string(),
        id: id.to_string(),
    })
}

#[cfg(test)]
#[path = "../tests/internal/portable_workspace.rs"]
mod tests;
