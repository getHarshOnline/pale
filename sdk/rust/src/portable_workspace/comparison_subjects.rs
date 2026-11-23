//! Discover subject identities in either side of an existing comparison.

use super::{
    derivation::ComparisonInputs, PortableWorkspaceComparison, PortableWorkspaceComparisonSummary,
    PortableWorkspaceSubject,
};
use crate::{error::FailureSpecification, ApiError};
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    RecordSchemaIdentity,
};

/// Maximum UTF-8 bytes copied from variable-length subject identity fields per page.
///
/// The application enforces this separately from its row and merge-work limits.
pub const MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES: usize =
    pale_application::workspace::MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES;

/// Resume after one full subject in the same retained comparison.
///
/// The exact ordered pair, receipt and raw limit remain bound. This value has
/// no public constructor, parser or serializer; matching digests cannot recreate it.
#[derive(Clone)]
pub struct PortableWorkspaceComparisonSubjectCursor {
    inputs: ComparisonInputs,
    after: PortableWorkspaceSubject,
    requested_limit: u16,
}

impl std::fmt::Debug for PortableWorkspaceComparisonSubjectCursor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceComparisonSubjectCursor")
            .finish_non_exhaustive()
    }
}

/// Selected identities present on either side of one exact comparison.
///
/// A shared identity appears once. This does not classify impact, value change
/// or source occurrence. Call [`PortableWorkspaceComparison::compare_subject`]
/// on the producing comparison for membership and value details. Both terminal
/// and pending pages retain the pair.
#[derive(Clone)]
pub struct PortableWorkspaceComparisonSubjectPage {
    inputs: ComparisonInputs,
    comparison: PortableWorkspaceComparisonSummary,
    subjects: Vec<PortableWorkspaceSubject>,
    subject_identity_bytes: usize,
    inspected: usize,
    key_comparisons: usize,
    continuation: Option<PortableWorkspaceComparisonSubjectCursor>,
    canonical_bytes: Vec<u8>,
}

impl std::fmt::Debug for PortableWorkspaceComparisonSubjectPage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceComparisonSubjectPage")
            .finish_non_exhaustive()
    }
}

impl PortableWorkspaceComparisonSubjectPage {
    /// Exact pair coordinates and the existing receipt identity.
    ///
    /// Summary counts describe the existing current-impact receipt, not the
    /// number of subjects in this union.
    #[must_use]
    pub const fn comparison(&self) -> &PortableWorkspaceComparisonSummary {
        &self.comparison
    }
    /// Existing full subject identities, once each in canonical subject order.
    #[must_use]
    pub fn subjects(&self) -> &[PortableWorkspaceSubject] {
        &self.subjects
    }
    /// UTF-8 bytes copied from variable-length identity fields on this page.
    ///
    /// Fixed row storage is bounded by the row ceiling. Canonical encoding
    /// bytes are separate and are not represented by this count.
    #[must_use]
    pub const fn subject_identity_bytes(&self) -> usize {
        self.subject_identity_bytes
    }
    /// Map entries fetched, including at most one look-ahead entry per side.
    #[must_use]
    pub const fn inspected(&self) -> usize {
        self.inspected
    }
    /// Merge-head comparisons, excluding comparisons inside the two range seeks.
    #[must_use]
    pub const fn key_comparisons(&self) -> usize {
        self.key_comparisons
    }
    /// Next subject position, or none after both retained ranges are exhausted.
    #[must_use]
    pub const fn continuation(&self) -> Option<&PortableWorkspaceComparisonSubjectCursor> {
        self.continuation.as_ref()
    }
    /// Whether this result retains the comparison's exact ordered pair and receipt.
    #[must_use]
    pub fn belongs_to(&self, comparison: &PortableWorkspaceComparison) -> bool {
        self.inputs.matches(comparison)
    }
    /// Canonical identity projection computed once, without encoding opaque custody.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

impl PortableWorkspaceComparison {
    /// Enumerate selected subjects in either input, including predecessor-only facts.
    ///
    /// This merges bounded ranges of the existing fact maps. Zero selects 64;
    /// requests above 256 cap at 256. The application also caps the UTF-8 bytes
    /// copied from subject identity fields and resumes before the first subject
    /// that does not fit. No complete union, total count, graph walk, receipt
    /// rebuild or eager value comparison is performed. Every returned subject
    /// exists in at least one input. [`Self::compare_subject`] supplies membership
    /// and typed values on demand; [`Self::impacts`] remains current-only.
    ///
    /// Each page performs two range seeks, at most one merge-key comparison per
    /// returned subject and up to two entry reads per row plus one look-ahead per
    /// side. Look-ahead is charged again on resume. Subject identity string bytes
    /// have a separate fixed page ceiling. Range-seek work, canonical encoding
    /// bytes and the retained compilation owners are not measured by that ceiling.
    /// Results copy selected identities, summary and output bytes. They retain
    /// compilations through shared ownership, without copying graphs or source bodies.
    ///
    /// Continuations bind the ordered input pair, receipt and unchanged raw limit.
    /// A missing cursor starts a new enumeration. Supplied-input custody is not
    /// recipient authority and does not expose every unselected source occurrence.
    ///
    /// # Errors
    /// A mismatched pair, receipt or raw limit produces an input diagnostic before
    /// opening either range. It never silently restarts or skips to a nearby key.
    pub fn subjects(
        &self,
        after: Option<&PortableWorkspaceComparisonSubjectCursor>,
        limit: u16,
    ) -> Result<PortableWorkspaceComparisonSubjectPage, ApiError> {
        if after
            .is_some_and(|cursor| !cursor.inputs.matches(self) || cursor.requested_limit != limit)
        {
            return Err(continuation_mismatch(&self.diagnostic_identity));
        }
        let page = self.current.comparison_subjects(
            &self.prior,
            after.map(|cursor| cursor.after.application_subject()),
            limit,
        );
        let inputs = ComparisonInputs::retain(self);
        let continuation =
            page.next_after()
                .map(|subject| PortableWorkspaceComparisonSubjectCursor {
                    inputs: inputs.clone(),
                    after: PortableWorkspaceSubject::from_internal(subject),
                    requested_limit: limit,
                });
        let mut result = PortableWorkspaceComparisonSubjectPage {
            inputs,
            comparison: self.summary(),
            subjects: page
                .subjects()
                .map(PortableWorkspaceSubject::from_internal)
                .collect(),
            subject_identity_bytes: page.subject_identity_bytes(),
            inspected: page.inspected(),
            key_comparisons: page.key_comparisons(),
            continuation,
            canonical_bytes: Vec::new(),
        };
        result.canonical_bytes = super::projection::comparison_subject_page(&result);
        Ok(result)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("subject continuation does not match this comparison and raw limit")]
struct ContinuationMismatch;

fn continuation_mismatch(identity: &RecordSchemaIdentity) -> ApiError {
    ApiError::new_with_opaque_source(
        identity,
        ContinuationMismatch,
        FailureSpecification {
            code: "workspace_comparison_subject_continuation_mismatch",
            stage: DiagnosticStageV1::InputAdmission,
            disposition: DiagnosticDispositionV1::InvalidInput,
            summary: "subject continuation does not match this comparison and raw limit",
            retry: DiagnosticRetryV1::AfterCorrection,
            subject: None,
            schema_coordinate: None,
            location: None,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}
