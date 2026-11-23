//! Page direct proof inputs without exposing the retained compiler graph.

use super::{PortableWorkspaceCausalStep, PortableWorkspaceCompilation, PortableWorkspaceSubject};
use crate::{error::FailureSpecification, ApiError};
use pale_application::workspace::PortableWorkspaceUniverse;
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    RecordSchemaIdentity,
};
use std::sync::Arc;

/// Maximum UTF-8 bytes copied into dependency-step string fields per page.
///
/// The application enforces this independently of the row limit. The repeated
/// root and canonical encoding remain separate fixed/derived costs.
pub const MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES: usize =
    pale_application::workspace::MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES;

/// Continue a direct-dependency query on its exact compilation and subject.
///
/// Copies share the immutable compilation. This value has no parser, serializer
/// or public constructor and grants no disclosure permission.
#[derive(Clone)]
pub struct PortableWorkspaceDependencyCursor {
    owner: Arc<PortableWorkspaceUniverse>,
    subject: PortableWorkspaceSubject,
    requested_limit: u16,
    offset: usize,
}

impl std::fmt::Debug for PortableWorkspaceDependencyCursor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceDependencyCursor")
            .finish_non_exhaustive()
    }
}

/// Selected compiler fact and a page of its direct proof inputs.
///
/// A missing root means the subject is absent. A present root with no inputs
/// is different: it is a known fact with no direct proof dependencies. Neither
/// condition describes provider readiness or native execution. Edges point
/// from each input fact to this root; they do not imply transitive reachability.
#[derive(Clone, Debug)]
pub struct PortableWorkspaceDependencyPage {
    principal_universe_digest: String,
    semantic_ir_digest: String,
    subject: PortableWorkspaceSubject,
    root: Option<PortableWorkspaceCausalStep>,
    dependencies: Vec<PortableWorkspaceCausalStep>,
    copied_string_bytes: usize,
    continuation: Option<PortableWorkspaceDependencyCursor>,
    canonical_bytes: Vec<u8>,
}

impl PortableWorkspaceDependencyPage {
    /// Principal-relative identity of the compilation actually queried.
    #[must_use]
    pub fn principal_universe_digest(&self) -> &str {
        &self.principal_universe_digest
    }

    /// Semantic-IR identity of that same compilation.
    #[must_use]
    pub fn semantic_ir_digest(&self) -> &str {
        &self.semantic_ir_digest
    }

    /// Full semantic subject selected by the caller.
    #[must_use]
    pub const fn subject(&self) -> &PortableWorkspaceSubject {
        &self.subject
    }

    /// Selected compiler fact, or none when the subject is absent.
    #[must_use]
    pub const fn root(&self) -> Option<&PortableWorkspaceCausalStep> {
        self.root.as_ref()
    }

    /// Direct input facts in the compiler's proof-digest order.
    #[must_use]
    pub fn dependencies(&self) -> &[PortableWorkspaceCausalStep] {
        &self.dependencies
    }

    /// UTF-8 bytes copied into this page's dependency-step string fields.
    ///
    /// The repeated root, fixed enum/vector storage and canonical encoding are
    /// not represented by this count.
    #[must_use]
    pub const fn copied_string_bytes(&self) -> usize {
        self.copied_string_bytes
    }

    /// Next page on the same owner, subject and raw requested limit.
    #[must_use]
    pub const fn continuation(&self) -> Option<&PortableWorkspaceDependencyCursor> {
        self.continuation.as_ref()
    }

    /// Canonical projection built once; the opaque continuation is not encoded.
    ///
    /// The encoded completion flag describes this page. It cannot reconstruct
    /// the continuation or establish a new admitted compilation.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

impl PortableWorkspaceCompilation {
    /// Read direct compiler-proof inputs without recompilation or a closure walk.
    ///
    /// Zero selects 64 input facts and requests above 256 are capped. Each page
    /// also includes its selected root fact. Resume with the
    /// same raw limit and full subject on this exact retained compilation; equal
    /// digests do not make a freshly compiled owner interchangeable. Input order
    /// comes from the compiler. The application performs one subject lookup and
    /// at most one binary node lookup per returned input, plus the root lookup.
    ///
    /// The row limit combines with a fixed aggregate ceiling over copied dependency
    /// string fields. The first whole step that would exceed either ceiling is left
    /// for the existing continuation; no causal fact is truncated. The repeated root,
    /// fixed storage, canonical encoding, retained graph and caller-accumulated pages
    /// are not included. This does not filter disclosure, observe a resident, infer
    /// impact, or calculate a renderer layout.
    ///
    /// # Errors
    ///
    /// Rejects a continuation from another owner, subject or raw limit. Internal
    /// position inconsistency is a separate diagnostic, never an empty result.
    pub fn direct_dependencies(
        &self,
        subject: &PortableWorkspaceSubject,
        after: Option<&PortableWorkspaceDependencyCursor>,
        limit: u16,
    ) -> Result<PortableWorkspaceDependencyPage, ApiError> {
        if after.is_some_and(|cursor| {
            !Arc::ptr_eq(&cursor.owner, &self.universe)
                || cursor.subject != *subject
                || cursor.requested_limit != limit
        }) {
            return Err(query_failure(
                &self.diagnostic_identity,
                QueryFailure::ContinuationMismatch,
            ));
        }
        let page = self
            .universe
            .direct_dependencies(
                &subject.inner,
                after.map_or(0, |cursor| cursor.offset),
                limit,
            )
            .map_err(|_| query_failure(&self.diagnostic_identity, QueryFailure::InvalidPosition))?;
        let (root, dependencies, copied_string_bytes, continuation) = match page {
            None => (None, Vec::new(), 0, None),
            Some(page) => (
                Some(PortableWorkspaceCausalStep::from_internal(page.root())),
                page.dependencies()
                    .map(PortableWorkspaceCausalStep::from_internal)
                    .collect(),
                page.copied_string_bytes(),
                page.next_offset()
                    .map(|offset| PortableWorkspaceDependencyCursor {
                        owner: Arc::clone(&self.universe),
                        subject: subject.clone(),
                        requested_limit: limit,
                        offset,
                    }),
            ),
        };
        let mut result = PortableWorkspaceDependencyPage {
            principal_universe_digest: self.principal_universe_digest().to_owned(),
            semantic_ir_digest: self.semantic_ir_digest().to_owned(),
            subject: subject.clone(),
            root,
            dependencies,
            copied_string_bytes,
            continuation,
            canonical_bytes: Vec::new(),
        };
        result.canonical_bytes = super::projection::dependency_page(&result);
        Ok(result)
    }
}

#[derive(Debug, thiserror::Error)]
enum QueryFailure {
    #[error("dependency continuation does not match this compilation and query")]
    ContinuationMismatch,
    #[error("dependency continuation exceeds the retained input list")]
    InvalidPosition,
}

fn query_failure(identity: &RecordSchemaIdentity, failure: QueryFailure) -> ApiError {
    let (code, stage, disposition, summary, retry) = match failure {
        QueryFailure::ContinuationMismatch => (
            "workspace_dependency_continuation_mismatch",
            DiagnosticStageV1::InputAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "dependency continuation does not match this compilation and query",
            DiagnosticRetryV1::AfterCorrection,
        ),
        QueryFailure::InvalidPosition => (
            "workspace_dependency_position_invalid",
            DiagnosticStageV1::Internal,
            DiagnosticDispositionV1::InternalFailure,
            "dependency continuation exceeds the retained input list",
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
