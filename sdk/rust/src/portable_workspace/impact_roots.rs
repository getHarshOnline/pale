//! Read contributing change roots without reconstructing their verified relation.

use pale_application::workspace::PortableWorkspaceContributingRoots as ApplicationRoots;
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    RecordSchemaIdentity,
};

use super::derivation::ComparisonInputs;
use super::{
    PortableWorkspaceChangeRoot, PortableWorkspaceComparison, PortableWorkspaceComparisonSummary,
    PortableWorkspaceSubject,
};
use crate::{error::FailureSpecification, ApiError};

/// Resume contributing roots for the same retained inputs, subject and raw limit.
///
/// Clones share the compilation owners. This value has no public constructor,
/// parser or serializer; matching digest strings cannot reconstruct it.
#[derive(Clone)]
pub struct PortableWorkspaceContributingRootsCursor {
    inputs: ComparisonInputs,
    subject: PortableWorkspaceSubject,
    requested_limit: u16,
    offset: usize,
}

impl std::fmt::Debug for PortableWorkspaceContributingRootsCursor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceContributingRootsCursor")
            .finish_non_exhaustive()
    }
}

/// Current-partition disposition of a contributing-root query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableWorkspaceContributingRootsState {
    /// Absent from the current receipt partition, even if present in the prior input.
    NotPresent,
    /// Current subject is outside every verified change-root closure.
    Unaffected,
    /// Current subject has contributing roots; a terminal page may have no remaining entries.
    Affected,
}

/// Owned roots that contribute to one subject in an exact retained comparison.
///
/// Dependency impact is independent of typed value equality. Root membership
/// comes from the verified receipt; this page supplies no additional causal path.
/// Both terminal and pending pages retain their exact compilation inputs.
#[derive(Clone)]
pub struct PortableWorkspaceContributingRootsPage {
    inputs: ComparisonInputs,
    comparison: PortableWorkspaceComparisonSummary,
    subject: PortableWorkspaceSubject,
    state: PortableWorkspaceContributingRootsState,
    roots: Vec<PortableWorkspaceChangeRoot>,
    continuation: Option<PortableWorkspaceContributingRootsCursor>,
    canonical_bytes: Vec<u8>,
}

impl std::fmt::Debug for PortableWorkspaceContributingRootsPage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceContributingRootsPage")
            .field("state", &self.state)
            .field("root_count", &self.roots.len())
            .field("complete", &self.continuation.is_none())
            .finish_non_exhaustive()
    }
}

impl PortableWorkspaceContributingRootsPage {
    /// Parent receipt and independently typed prior/current compilation coordinates.
    #[must_use]
    pub const fn comparison(&self) -> &PortableWorkspaceComparisonSummary {
        &self.comparison
    }

    /// Full subject whose current impact relation was queried.
    #[must_use]
    pub const fn subject(&self) -> &PortableWorkspaceSubject {
        &self.subject
    }

    /// Current-partition presence and impact, not prior/current value equality.
    #[must_use]
    pub const fn state(&self) -> PortableWorkspaceContributingRootsState {
        self.state
    }

    /// Exact contributing roots in canonical root-digest order.
    #[must_use]
    pub fn roots(&self) -> &[PortableWorkspaceChangeRoot] {
        &self.roots
    }

    /// Next page, or none when the contributing-root set is exhausted.
    #[must_use]
    pub const fn continuation(&self) -> Option<&PortableWorkspaceContributingRootsCursor> {
        self.continuation.as_ref()
    }

    /// Whether this page retains the comparison's exact prior/current inputs and receipt.
    ///
    /// A freshly compiled equal-digest input is not interchangeable. Recreating a
    /// comparison over the very same retained pair and receipt is compatible.
    #[must_use]
    pub fn belongs_to(&self, comparison: &PortableWorkspaceComparison) -> bool {
        self.inputs.matches(comparison)
    }

    /// Canonical answer built once; it does not encode or reconstruct its opaque owner.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

impl PortableWorkspaceComparison {
    /// Page the verified change roots that contribute to one current subject.
    ///
    /// Zero selects 64 roots; requests above 256 are capped. Continuations bind
    /// the exact prior/current compilation pair, receipt, full subject and raw
    /// requested limit. Even normalization-equivalent limits must match.
    ///
    /// The application reads the retained contribution list, not a new graph
    /// traversal. Each page performs one subject lookup, up to the returned
    /// limit of root lookups, and the existing current-fact lookup for each
    /// added/modified root. It copies selected root/fact payloads and parent
    /// summary, then canonicalizes once. This is not a byte, time or aggregate
    /// memory bound. Retained compilations are shared, not copied.
    ///
    /// No recipient authority, renderer state, or native observation is admitted.
    /// Existing explanation selection and value comparison remain separate.
    ///
    /// # Errors
    /// Rejects a foreign or stale input pair, receipt, full subject or raw limit.
    /// An impossible retained offset is an internal failure, never completion.
    pub fn contributing_roots(
        &self,
        subject: &PortableWorkspaceSubject,
        after: Option<&PortableWorkspaceContributingRootsCursor>,
        limit: u16,
    ) -> Result<PortableWorkspaceContributingRootsPage, ApiError> {
        if after.is_some_and(|cursor| {
            !cursor.inputs.matches(self)
                || cursor.subject != *subject
                || cursor.requested_limit != limit
        }) {
            return Err(query_failure(
                &self.diagnostic_identity,
                QueryFailure::ContinuationMismatch,
            ));
        }
        let inputs = ComparisonInputs::retain(self);
        let (state, roots, next) = match self
            .derivation
            .contributing_roots(
                subject.application_subject(),
                after.map_or(0, |cursor| cursor.offset),
                limit,
            )
            .map_err(|_| query_failure(&self.diagnostic_identity, QueryFailure::InvalidPosition))?
        {
            ApplicationRoots::NotPresent => (
                PortableWorkspaceContributingRootsState::NotPresent,
                Vec::new(),
                None,
            ),
            ApplicationRoots::Unaffected => (
                PortableWorkspaceContributingRootsState::Unaffected,
                Vec::new(),
                None,
            ),
            ApplicationRoots::Affected(page) => (
                PortableWorkspaceContributingRootsState::Affected,
                page.roots()
                    .map(|root| {
                        PortableWorkspaceChangeRoot::from_internal(
                            root,
                            &self.derivation.receipt().nodes,
                        )
                    })
                    .collect(),
                page.next_offset(),
            ),
        };
        let continuation = next.map(|offset| PortableWorkspaceContributingRootsCursor {
            inputs: inputs.clone(),
            subject: subject.clone(),
            requested_limit: limit,
            offset,
        });
        let mut result = PortableWorkspaceContributingRootsPage {
            inputs,
            comparison: self.summary(),
            subject: subject.clone(),
            state,
            roots,
            continuation,
            canonical_bytes: Vec::new(),
        };
        result.canonical_bytes = super::projection::contributing_roots(&result);
        Ok(result)
    }
}

#[derive(Debug, thiserror::Error)]
enum QueryFailure {
    #[error("contributing-root continuation does not match this comparison and query")]
    ContinuationMismatch,
    #[error("contributing-root continuation exceeds the retained list")]
    InvalidPosition,
}

fn query_failure(identity: &RecordSchemaIdentity, failure: QueryFailure) -> ApiError {
    let (code, stage, disposition, summary, retry) = match failure {
        QueryFailure::ContinuationMismatch => (
            "workspace_contributing_roots_continuation_mismatch",
            DiagnosticStageV1::InputAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "contributing-root continuation does not match this comparison and query",
            DiagnosticRetryV1::AfterCorrection,
        ),
        QueryFailure::InvalidPosition => (
            "workspace_contributing_roots_position_invalid",
            DiagnosticStageV1::Internal,
            DiagnosticDispositionV1::InternalFailure,
            "contributing-root continuation exceeds the retained list",
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
