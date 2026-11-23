//! Page direct consumers without rebuilding the compiler's proof graph.

use std::sync::Arc;

use pale_application::workspace::PortableWorkspaceUniverse;
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    RecordSchemaIdentity,
};

use super::{PortableWorkspaceCausalStep, PortableWorkspaceCompilation, PortableWorkspaceSubject};
use crate::{error::FailureSpecification, ApiError};

/// Maximum candidate-input comparisons performed by one direct-dependent page.
///
/// The application owns and enforces this fixed work ceiling. It is separate
/// from the caller's candidate and result limits.
pub const MAX_DEPENDENT_PAGE_INPUT_COMPARISONS: usize =
    pale_application::workspace::MAX_DEPENDENT_PAGE_INPUT_COMPARISONS;

/// Continue a direct-consumer scan on its exact compilation and query.
///
/// Clones share the compiler owner. This value has no public constructor,
/// parser or serializer. It binds the full subject and both raw limits, not
/// just their normalized values, and grants no disclosure authority.
#[derive(Clone)]
pub struct PortableWorkspaceDependentCursor {
    owner: Arc<PortableWorkspaceUniverse>,
    subject: PortableWorkspaceSubject,
    maximum_inspected: u16,
    maximum_returned: u16,
    offset: usize,
}

impl std::fmt::Debug for PortableWorkspaceDependentCursor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceDependentCursor")
            .finish_non_exhaustive()
    }
}

/// Selected compiler fact and direct consumers found within one scan budget.
///
/// Root absence means unselected. A selected root can have no consumers, and
/// an empty page can be incomplete. Every result retains its exact compiler
/// owner, including terminal and unselected results. Clones copy projected
/// values and bytes but share that owner; no graph or source body is copied.
#[derive(Clone)]
pub struct PortableWorkspaceDependentPage {
    owner: Arc<PortableWorkspaceUniverse>,
    subject: PortableWorkspaceSubject,
    root: Option<PortableWorkspaceCausalStep>,
    dependents: Vec<PortableWorkspaceCausalStep>,
    inspected: usize,
    input_digest_comparisons: usize,
    continuation: Option<PortableWorkspaceDependentCursor>,
    canonical_bytes: Vec<u8>,
}

impl std::fmt::Debug for PortableWorkspaceDependentPage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceDependentPage")
            .finish_non_exhaustive()
    }
}

impl PortableWorkspaceDependentPage {
    /// Workspace identity from the compilation actually queried.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        self.owner.workspace_id()
    }

    /// Principal-relative universe identity of that compilation.
    #[must_use]
    pub fn principal_universe_digest(&self) -> &str {
        self.owner.principal_universe_digest()
    }

    /// Semantic-IR identity, not a transferable exact-input handle.
    #[must_use]
    pub fn semantic_ir_digest(&self) -> &str {
        self.owner.semantic_ir_digest()
    }

    /// Owner-admission evidence used by the producing compilation.
    #[must_use]
    pub fn admission_evidence_digest(&self) -> &str {
        self.owner.admission_evidence_digest()
    }

    /// Full subject whose direct consumers were requested.
    #[must_use]
    pub const fn subject(&self) -> &PortableWorkspaceSubject {
        &self.subject
    }

    /// Repeated selected fact, or none when the subject is unselected.
    #[must_use]
    pub const fn root(&self) -> Option<&PortableWorkspaceCausalStep> {
        self.root.as_ref()
    }

    /// Consumers in the compiler's node-digest order.
    ///
    /// Each consumer uses the root as a direct input under its own derivation
    /// rule. This does not claim transitive impact or runtime causation.
    #[must_use]
    pub fn dependents(&self) -> &[PortableWorkspaceCausalStep] {
        &self.dependents
    }

    /// Candidate nodes examined, including leaves and nonmatches.
    #[must_use]
    pub const fn inspected(&self) -> usize {
        self.inspected
    }

    /// Actual comparisons within candidate input searches; root lookup is excluded.
    ///
    /// This mechanical count can vary with the standard library implementation.
    /// It is not semantic identity, generation membership or causal evidence.
    #[must_use]
    pub const fn input_digest_comparisons(&self) -> usize {
        self.input_digest_comparisons
    }

    /// Whether the selected root's candidate scan is exhausted, or it is unselected.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.continuation.is_none()
    }

    /// Next candidate position with its exact owner and query custody.
    #[must_use]
    pub const fn continuation(&self) -> Option<&PortableWorkspaceDependentCursor> {
        self.continuation.as_ref()
    }

    /// Check exact retained compilation custody, not matching digests or authority.
    #[must_use]
    pub fn belongs_to(&self, compilation: &PortableWorkspaceCompilation) -> bool {
        std::ptr::eq(self.owner.as_ref(), compilation.application_universe())
    }

    /// Canonical projection computed once, without encoding the opaque cursor.
    ///
    /// The bytes include mechanical scan counts but cannot reconstruct compiler
    /// custody or authorize disclosure. Existing canonicalization still creates
    /// temporary JSON values; once-only encoding does not mean zero allocation.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

impl PortableWorkspaceCompilation {
    /// Find direct consumers of one selected compiler fact without a reverse index.
    ///
    /// The application scans retained nodes in proof-digest order. Each inspected
    /// candidate costs one unit even when it has no inputs or does not match.
    /// Zero selects 64 for each independent limit; values above 256 cap at 256.
    /// An empty page can remain incomplete. Resume with this exact compilation,
    /// full subject and both unchanged raw limits; absence of a cursor starts a
    /// new scan, not an implicit continuation of a completed result.
    ///
    /// The root fact repeats. Results copy selected typed facts and encoded bytes
    /// while retaining the compiler owner. Candidate and row limits combine with
    /// the fixed application-owned aggregate input-comparison ceiling. Reaching
    /// any ceiling leaves a continuation and never skips its next candidate.
    /// These bounds do not limit elapsed time, encoded bytes or accumulated
    /// retained memory. This is a supplied-input query, not recipient admission
    /// or transitive impact.
    ///
    /// # Errors
    /// A foreign owner, subject or raw limit produces a canonical input diagnostic
    /// before scanning. An impossible retained position is an internal diagnostic.
    pub fn direct_dependents(
        &self,
        subject: &PortableWorkspaceSubject,
        after: Option<&PortableWorkspaceDependentCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<PortableWorkspaceDependentPage, ApiError> {
        if after.is_some_and(|cursor| {
            !Arc::ptr_eq(&cursor.owner, &self.universe)
                || cursor.subject != *subject
                || cursor.maximum_inspected != maximum_inspected
                || cursor.maximum_returned != maximum_returned
        }) {
            return Err(query_failure(
                &self.diagnostic_identity,
                QueryFailure::ContinuationMismatch,
            ));
        }
        let page = self
            .universe
            .direct_dependents(
                subject.application_subject(),
                after.map_or(0, |cursor| cursor.offset),
                maximum_inspected,
                maximum_returned,
            )
            .map_err(|_| query_failure(&self.diagnostic_identity, QueryFailure::InvalidPosition))?;
        let (root, dependents, inspected, input_digest_comparisons, continuation) = match page {
            None => (None, Vec::new(), 0, 0, None),
            Some(page) => (
                Some(PortableWorkspaceCausalStep::from_internal(page.root())),
                page.dependents()
                    .map(PortableWorkspaceCausalStep::from_internal)
                    .collect(),
                page.inspected(),
                page.input_digest_comparisons(),
                page.next_offset()
                    .map(|offset| PortableWorkspaceDependentCursor {
                        owner: Arc::clone(&self.universe),
                        subject: subject.clone(),
                        maximum_inspected,
                        maximum_returned,
                        offset,
                    }),
            ),
        };
        let mut result = PortableWorkspaceDependentPage {
            owner: Arc::clone(&self.universe),
            subject: subject.clone(),
            root,
            dependents,
            inspected,
            input_digest_comparisons,
            continuation,
            canonical_bytes: Vec::new(),
        };
        result.canonical_bytes = super::projection::dependent_page(&result);
        Ok(result)
    }
}

#[derive(Debug, thiserror::Error)]
enum QueryFailure {
    #[error("dependent continuation does not match this compilation and query")]
    ContinuationMismatch,
    #[error("dependent continuation exceeds the retained candidate list")]
    InvalidPosition,
}

fn query_failure(identity: &RecordSchemaIdentity, failure: QueryFailure) -> ApiError {
    let (code, stage, disposition, summary, retry) = match failure {
        QueryFailure::ContinuationMismatch => (
            "workspace_dependent_continuation_mismatch",
            DiagnosticStageV1::InputAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "dependent continuation does not match this compilation and query",
            DiagnosticRetryV1::AfterCorrection,
        ),
        QueryFailure::InvalidPosition => (
            "workspace_dependent_position_invalid",
            DiagnosticStageV1::Internal,
            DiagnosticDispositionV1::InternalFailure,
            "dependent continuation exceeds the retained candidate list",
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
