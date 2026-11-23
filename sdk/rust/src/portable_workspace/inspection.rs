//! Inspect one subject through its retained compiler evidence and source owners.

use std::sync::Arc;

use pale_application::workspace::{
    PortableWorkspaceSubjectInspection as ApplicationPage,
    PortableWorkspaceSubjectInspectionCursor as ApplicationCursor,
    PortableWorkspaceSubjectInspectionError as ApplicationError, PortableWorkspaceUniverse,
};
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    RecordSchemaIdentity,
};

use super::{
    PortableWorkspaceCausalStep, PortableWorkspaceComparison, PortableWorkspaceCompilation,
    PortableWorkspaceSubject,
};
use crate::{error::FailureSpecification, ApiError, AuthoringOccurrence, AuthoringSourceLocation};

/// Continue unfinished inspection sections on the exact producing compilation.
///
/// Clones share the owner, without copying its graph. This opaque value binds
/// the full subject and all three raw limits, even when one section is done.
/// It has no public constructor, parser or serializer and grants no disclosure
/// authority. Dropping its last copy releases this cursor's retained ownership.
#[derive(Clone)]
pub struct PortableWorkspaceSubjectInspectionCursor {
    owner: Arc<PortableWorkspaceUniverse>,
    subject: PortableWorkspaceSubject,
    input_limit: u16,
    maximum_inspected: u16,
    maximum_returned: u16,
    application: ApplicationCursor,
}

impl std::fmt::Debug for PortableWorkspaceSubjectInspectionCursor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceSubjectInspectionCursor")
            .finish_non_exhaustive()
    }
}

/// Owned fact and source evidence from one exact retained compilation.
///
/// Selected membership, source matches and an optional owning record anchor are
/// separate observations. Terminal and unselected results retain the owner too;
/// equal semantic digests do not imply equal captured-input custody. Clones copy
/// projected values/bytes and share the immutable compilation until last drop.
/// This result grants no permission to disclose source or execute an operation.
#[derive(Clone)]
pub struct PortableWorkspaceSubjectInspection {
    owner: Arc<PortableWorkspaceUniverse>,
    subject: PortableWorkspaceSubject,
    root: Option<PortableWorkspaceCausalStep>,
    dependencies: Vec<PortableWorkspaceCausalStep>,
    occurrences: Vec<AuthoringOccurrence>,
    inspected: u16,
    record_anchor: Option<AuthoringSourceLocation>,
    dependency_copied_string_bytes: u32,
    source_copied_string_bytes: u32,
    inputs_complete: bool,
    sources_complete: bool,
    continuation: Option<PortableWorkspaceSubjectInspectionCursor>,
    canonical_bytes: Vec<u8>,
}

impl serde::Serialize for PortableWorkspaceSubjectInspection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(
            &super::projection::SubjectInspectionProjection::from(self),
            serializer,
        )
    }
}

#[cfg(feature = "wire-schema")]
impl schemars::JsonSchema for PortableWorkspaceSubjectInspection {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "PortableWorkspaceSubjectInspection".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::PortableWorkspaceSubjectInspection").into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        <super::projection::SubjectInspectionProjection<'static> as schemars::JsonSchema>::json_schema(
            generator,
        )
    }
}

impl std::fmt::Debug for PortableWorkspaceSubjectInspection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceSubjectInspection")
            .finish_non_exhaustive()
    }
}

impl PortableWorkspaceSubjectInspection {
    /// Workspace identity from the exact immutable owner that supplied this answer.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        self.owner.workspace_id()
    }

    /// Principal-universe identity from that same owner.
    #[must_use]
    pub fn principal_universe_digest(&self) -> &str {
        self.owner.principal_universe_digest()
    }

    /// Semantic-IR identity from that same owner, not an exact-input handle.
    #[must_use]
    pub fn semantic_ir_digest(&self) -> &str {
        self.owner.semantic_ir_digest()
    }

    /// Owner-admission evidence identity actually used by this compilation.
    #[must_use]
    pub fn admission_evidence_digest(&self) -> &str {
        self.owner.admission_evidence_digest()
    }

    /// Full semantic subject supplied to this inspection.
    #[must_use]
    pub const fn subject(&self) -> &PortableWorkspaceSubject {
        &self.subject
    }

    /// Repeated selected root, or none for an unselected subject.
    #[must_use]
    pub const fn root(&self) -> Option<&PortableWorkspaceCausalStep> {
        self.root.as_ref()
    }

    /// This page's direct inputs in the compiler's proof-digest order.
    #[must_use]
    pub fn dependencies(&self) -> &[PortableWorkspaceCausalStep] {
        &self.dependencies
    }

    /// This page's direct named source matches and their compiler-assigned roles.
    #[must_use]
    pub fn occurrences(&self) -> &[AuthoringOccurrence] {
        &self.occurrences
    }

    /// Source candidates examined this call, including nonmatches; zero when done.
    #[must_use]
    pub const fn inspected(&self) -> u16 {
        self.inspected
    }

    /// An owning import-path or extension record, separate from named occurrences.
    /// Absence does not establish that the subject or its provenance is absent.
    #[must_use]
    pub const fn record_anchor(&self) -> Option<&AuthoringSourceLocation> {
        self.record_anchor.as_ref()
    }

    /// UTF-8 bytes copied into direct dependency string fields on this page.
    #[must_use]
    pub const fn dependency_copied_string_bytes(&self) -> u32 {
        self.dependency_copied_string_bytes
    }

    /// UTF-8 bytes copied into occurrence and repeated record-anchor locations.
    #[must_use]
    pub const fn source_copied_string_bytes(&self) -> u32 {
        self.source_copied_string_bytes
    }

    /// Whether direct input enumeration is complete, independently of source scan.
    #[must_use]
    pub const fn inputs_complete(&self) -> bool {
        self.inputs_complete
    }

    /// Whether source scan exhaustion has been proved, independently of inputs.
    #[must_use]
    pub const fn sources_complete(&self) -> bool {
        self.sources_complete
    }

    /// Whether both independently bounded sections are complete.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.inputs_complete && self.sources_complete
    }

    /// Borrow a continuation only when at least one section remains unfinished.
    #[must_use]
    pub const fn continuation(&self) -> Option<&PortableWorkspaceSubjectInspectionCursor> {
        self.continuation.as_ref()
    }

    /// Check exact compilation custody, not semantic-digest equivalence or authority.
    #[must_use]
    pub fn belongs_to(&self, compilation: &PortableWorkspaceCompilation) -> bool {
        std::ptr::eq(self.owner.as_ref(), compilation.application_universe())
    }

    /// Borrow the canonical projection computed once for this answer.
    ///
    /// Bytes encode independent completion, not the native owner or continuation.
    /// They cannot reconstruct those handles. Encoding uses the existing JSON
    /// projection and canonicalizer, including its validation's temporary Value;
    /// once-only encoding does not imply zero temporary allocation.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

impl PortableWorkspaceCompilation {
    /// Inspect one subject's fact, direct inputs and source evidence without recompiling.
    ///
    /// The application owns selection and section consistency. Zero limits use
    /// its 64-row defaults and each is capped at 256. Completed sections are not
    /// restarted; an empty source page may still require continuation. The root,
    /// owner coordinates and optional record anchor repeat so every page is usable.
    ///
    /// Resume with this exact compilation, full subject and all three raw limits.
    /// Equal digests or a fresh identical compilation cannot substitute for it.
    /// Each result, including terminal/unselected results, retains the exact owner.
    /// Graph data are shared; root/input facts, source locations and encoded bytes
    /// are copied. Dependency and source counters report their actual copied
    /// string bytes. Source bytes include the occurrence page and any repeated
    /// record anchor; they do not bound time or aggregate retained memory.
    /// This supplied-input query neither admits recipients nor authorizes disclosure.
    ///
    /// # Errors
    /// A mismatched continuation is rejected before either query. Application
    /// position or membership failures map to canonical internal diagnostics.
    /// Unselected subjects and missing source evidence are successful results.
    ///
    /// # Panics
    /// Panics if the application violates its 256-candidate source-scan cap and
    /// returns an inspected count that does not fit the public 16-bit count.
    ///
    /// ```compile_fail
    /// # use pale_sdk::{PortableWorkspaceCompilation, PortableWorkspaceDependencyCursor, PortableWorkspaceSubject};
    /// fn wrong_cursor(owner: &PortableWorkspaceCompilation, subject: &PortableWorkspaceSubject,
    ///                 cursor: &PortableWorkspaceDependencyCursor) {
    ///     owner.inspect_subject(subject, Some(cursor), 1, 1, 1).unwrap();
    /// }
    /// ```
    pub fn inspect_subject(
        &self,
        subject: &PortableWorkspaceSubject,
        after: Option<&PortableWorkspaceSubjectInspectionCursor>,
        input_limit: u16,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<PortableWorkspaceSubjectInspection, ApiError> {
        inspect_subject(
            &self.universe,
            &self.diagnostic_identity,
            subject,
            after,
            input_limit,
            maximum_inspected,
            maximum_returned,
        )
    }
}

impl PortableWorkspaceComparison {
    /// Inspect one subject through the exact current input retained by this comparison.
    ///
    /// This is the same bounded query as
    /// [`PortableWorkspaceCompilation::inspect_subject`], applied to the existing
    /// current owner. It does not reconstruct a compilation, substitute another
    /// equal-digest input, or repeat comparison work. This supplied-input query
    /// grants no recipient disclosure authority.
    ///
    /// # Errors
    /// Returns the existing subject-inspection diagnostic when a continuation
    /// belongs to another owner or query, or when retained evidence is incoherent.
    pub fn inspect_current_subject(
        &self,
        subject: &PortableWorkspaceSubject,
        after: Option<&PortableWorkspaceSubjectInspectionCursor>,
        input_limit: u16,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<PortableWorkspaceSubjectInspection, ApiError> {
        inspect_subject(
            &self.current,
            &self.diagnostic_identity,
            subject,
            after,
            input_limit,
            maximum_inspected,
            maximum_returned,
        )
    }
}

fn inspect_subject(
    universe: &Arc<PortableWorkspaceUniverse>,
    diagnostic_identity: &RecordSchemaIdentity,
    subject: &PortableWorkspaceSubject,
    after: Option<&PortableWorkspaceSubjectInspectionCursor>,
    input_limit: u16,
    maximum_inspected: u16,
    maximum_returned: u16,
) -> Result<PortableWorkspaceSubjectInspection, ApiError> {
    if after.is_some_and(|cursor| {
        !Arc::ptr_eq(&cursor.owner, universe)
            || cursor.subject != *subject
            || cursor.input_limit != input_limit
            || cursor.maximum_inspected != maximum_inspected
            || cursor.maximum_returned != maximum_returned
    }) {
        return Err(inspection_failure(
            diagnostic_identity,
            InspectionFailure::ContinuationMismatch,
        ));
    }
    let page = universe
        .inspect_subject(
            subject.application_subject(),
            after.map(|cursor| &cursor.application),
            input_limit,
            usize::from(maximum_inspected),
            usize::from(maximum_returned),
        )
        .map_err(|error| {
            inspection_failure(diagnostic_identity, InspectionFailure::Application(error))
        })?;
    project_subject_page(
        universe,
        subject,
        input_limit,
        maximum_inspected,
        maximum_returned,
        page,
    )
}

fn project_subject_page(
    universe: &Arc<PortableWorkspaceUniverse>,
    subject: &PortableWorkspaceSubject,
    input_limit: u16,
    maximum_inspected: u16,
    maximum_returned: u16,
    page: ApplicationPage<'_>,
) -> Result<PortableWorkspaceSubjectInspection, ApiError> {
    let dependencies = page
        .dependency_page()
        .map(|page| {
            page.dependencies()
                .map(PortableWorkspaceCausalStep::from_internal)
                .collect()
        })
        .unwrap_or_default();
    let (occurrences, inspected) = page
        .source_page()
        .map(|page| {
            (
                page.occurrences()
                    .iter()
                    .map(|row| AuthoringOccurrence::from_application(row))
                    .collect(),
                u16::try_from(page.inspected()).expect("application source scan is capped at 256"),
            )
        })
        .unwrap_or_default();
    let continuation =
        page.continuation()
            .cloned()
            .map(|application| PortableWorkspaceSubjectInspectionCursor {
                owner: Arc::clone(universe),
                subject: subject.clone(),
                input_limit,
                maximum_inspected,
                maximum_returned,
                application,
            });
    let mut result = PortableWorkspaceSubjectInspection {
        owner: Arc::clone(universe),
        subject: subject.clone(),
        root: page.root().map(PortableWorkspaceCausalStep::from_internal),
        dependencies,
        occurrences,
        inspected,
        record_anchor: page
            .record_anchor()
            .map(AuthoringSourceLocation::from_application),
        dependency_copied_string_bytes: u32::try_from(page.dependency_copied_string_bytes())
            .expect("application dependency copied-string ceiling fits u32"),
        source_copied_string_bytes: u32::try_from(page.source_copied_string_bytes())
            .expect("application source copied-string ceiling fits u32"),
        inputs_complete: page.inputs_complete(),
        sources_complete: page.sources_complete(),
        continuation,
        canonical_bytes: Vec::new(),
    };
    result.canonical_bytes = super::projection::subject_inspection(&result);
    Ok(result)
}

#[derive(Debug, thiserror::Error)]
enum InspectionFailure {
    #[error("subject inspection continuation does not match this compilation and query")]
    ContinuationMismatch,
    #[error("subject inspection failed: {0}")]
    Application(#[source] ApplicationError),
}

fn inspection_failure(identity: &RecordSchemaIdentity, failure: InspectionFailure) -> ApiError {
    let (code, stage, disposition, summary, retry) = match &failure {
        InspectionFailure::ContinuationMismatch => (
            "workspace_subject_inspection_continuation_mismatch",
            DiagnosticStageV1::InputAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "subject inspection continuation does not match this compilation and query",
            DiagnosticRetryV1::AfterCorrection,
        ),
        InspectionFailure::Application(ApplicationError::InvalidInputPosition(_)) => (
            "workspace_subject_inspection_position_invalid",
            DiagnosticStageV1::Internal,
            DiagnosticDispositionV1::InternalFailure,
            "subject inspection input position is invalid",
            DiagnosticRetryV1::NotIndicated,
        ),
        InspectionFailure::Application(ApplicationError::MembershipIncoherent) => (
            "workspace_subject_inspection_membership_incoherent",
            DiagnosticStageV1::Internal,
            DiagnosticDispositionV1::InternalFailure,
            "subject inspection source membership disagrees with its selected fact",
            DiagnosticRetryV1::NotIndicated,
        ),
    };
    ApiError::new_with_opaque_source(
        identity,
        failure,
        FailureSpecification {
            code,
            stage,
            disposition,
            summary,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            subject: None,
            schema_coordinate: None,
            location: None,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}
