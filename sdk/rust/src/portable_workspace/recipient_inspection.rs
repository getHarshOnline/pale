//! Recipient-qualified projections over one exact workspace comparison.

use pale_application::workspace::{
    AdmittedWorkspaceComparisonInspection as ApplicationAdmittedComparison,
    WorkspaceInspectionComparisonSide,
};

use super::{
    OrganizationComparisonSide, PortableWorkspaceChangeRootPage, PortableWorkspaceComparison,
    PortableWorkspaceComparisonSubjectCursor, PortableWorkspaceComparisonSubjectPage,
    PortableWorkspaceComparisonSummary, PortableWorkspaceCompilation,
    PortableWorkspaceContributingRootsCursor, PortableWorkspaceContributingRootsPage,
    PortableWorkspaceDependencyCursor, PortableWorkspaceDependencyPage,
    PortableWorkspaceImpactPage, PortableWorkspaceSubject, PortableWorkspaceSubjectComparison,
    PortableWorkspaceSubjectInspection, PortableWorkspaceSubjectInspectionCursor,
    RecipientQualifiedWorkspaceInspection, RecipientQualifiedWorkspaceInspectionError,
};

/// An admitted comparison does not retain the SDK pair and comparison it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("recipient-qualified comparison does not belong to the supplied SDK owners")]
pub struct RecipientQualifiedWorkspaceComparisonOwnerMismatch;

/// Recipient-qualified access to one exact ordered comparison.
///
/// The SDK can bind only an opaque value issued by the application. Every query,
/// including continuation, returns to that application authority owner.
/// Cursors issued here cannot be used with the supplied-input comparison API.
/// Live acquisition remains unavailable until the authenticated resident route
/// can return the jointly issued opaque handles.
pub struct RecipientQualifiedWorkspaceComparisonInspection {
    prior: PortableWorkspaceCompilation,
    current: PortableWorkspaceCompilation,
    comparison: PortableWorkspaceComparison,
    admitted: ApplicationAdmittedComparison,
}

/// Identity summaries for one recipient-qualified ordered comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipientQualifiedWorkspacePairSummary {
    prior: super::PortableWorkspaceCompilationSummary,
    current: super::PortableWorkspaceCompilationSummary,
    comparison: PortableWorkspaceComparisonSummary,
}

impl RecipientQualifiedWorkspacePairSummary {
    /// Summary of the exact predecessor owner.
    #[must_use]
    pub const fn prior(&self) -> &super::PortableWorkspaceCompilationSummary {
        &self.prior
    }

    /// Summary of the exact current owner.
    #[must_use]
    pub const fn current(&self) -> &super::PortableWorkspaceCompilationSummary {
        &self.current
    }

    /// Summary of the exact comparison derivation.
    #[must_use]
    pub const fn comparison(&self) -> &PortableWorkspaceComparisonSummary {
        &self.comparison
    }
}

impl std::fmt::Debug for RecipientQualifiedWorkspaceComparisonInspection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecipientQualifiedWorkspaceComparisonInspection")
            .finish_non_exhaustive()
    }
}

impl RecipientQualifiedWorkspaceComparisonInspection {
    /// Test whether this comparison retains these exact qualified inspections.
    ///
    /// This proves SDK custody only. It does not recheck recipient authority,
    /// disclosure, freshness, or the shared inspection budget.
    #[must_use]
    pub fn compares_exact_inspections(
        &self,
        prior: &RecipientQualifiedWorkspaceInspection,
        current: &RecipientQualifiedWorkspaceInspection,
    ) -> bool {
        self.comparison
            .compares(prior.compilation(), current.compilation())
    }

    /// Identity summaries of the exact retained pair and its comparison.
    ///
    /// # Errors
    /// Returns an authority or aggregate-budget failure before projection.
    pub fn summaries(
        &self,
    ) -> Result<RecipientQualifiedWorkspacePairSummary, RecipientQualifiedWorkspaceInspectionError>
    {
        self.admitted.authorize_summary()?;
        let summary = RecipientQualifiedWorkspacePairSummary {
            prior: self.prior.summary(),
            current: self.current.summary(),
            comparison: self.comparison.summary(),
        };
        let encoded_bytes = summary
            .prior
            .canonical_bytes()
            .len()
            .saturating_add(summary.current.canonical_bytes().len())
            .saturating_add(summary.comparison.canonical_bytes().len());
        self.admitted
            .budget()
            .charge_output(0, u64::try_from(encoded_bytes).unwrap_or(u64::MAX))
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        Ok(summary)
    }

    /// Enumerate subjects after fresh recipient admission.
    ///
    /// # Errors
    /// Returns an authority, budget, custody, or SDK projection failure.
    pub fn subjects(
        &self,
        after: Option<&RecipientQualifiedComparisonSubjectCursor>,
        limit: u16,
    ) -> Result<RecipientQualifiedComparisonSubjectPage, RecipientQualifiedWorkspaceInspectionError>
    {
        let permit = self.admitted.authorize_subjects(after.is_some(), limit)?;
        debug_assert!(permit.matches(after.is_some(), limit));
        let page = self
            .comparison
            .subjects(after.map(|cursor| &cursor.inner), limit)?;
        self.admitted
            .budget()
            .charge_output(
                u64::try_from(page.subject_identity_bytes()).unwrap_or(u64::MAX),
                u64::try_from(page.canonical_bytes().len()).unwrap_or(u64::MAX),
            )
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        Ok(RecipientQualifiedComparisonSubjectPage {
            continuation: page
                .continuation()
                .cloned()
                .map(|inner| RecipientQualifiedComparisonSubjectCursor { inner }),
            inner: page,
        })
    }

    /// Project one change-root page after fresh recipient admission.
    ///
    /// # Errors
    /// Returns an authority or aggregate-budget failure.
    pub fn change_roots(
        &self,
        offset: u32,
        limit: u16,
    ) -> Result<PortableWorkspaceChangeRootPage, RecipientQualifiedWorkspaceInspectionError> {
        let permit = self.admitted.authorize_change_roots(offset, limit)?;
        debug_assert!(permit.matches(offset, limit));
        let page = self.comparison.change_roots(offset, limit);
        self.admitted
            .budget()
            .charge_output(
                0,
                u64::try_from(page.canonical_bytes().len()).unwrap_or(u64::MAX),
            )
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        Ok(page)
    }

    /// Project one impact page after fresh recipient admission.
    ///
    /// # Errors
    /// Returns an authority or aggregate-budget failure.
    pub fn impacts(
        &self,
        offset: u32,
        limit: u16,
    ) -> Result<PortableWorkspaceImpactPage, RecipientQualifiedWorkspaceInspectionError> {
        let permit = self.admitted.authorize_impacts(offset, limit)?;
        debug_assert!(permit.matches(offset, limit));
        let page = self.comparison.impacts(offset, limit);
        self.admitted
            .budget()
            .charge_output(
                0,
                u64::try_from(page.canonical_bytes().len()).unwrap_or(u64::MAX),
            )
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        Ok(page)
    }

    /// Compare one subject after fresh recipient admission.
    ///
    /// # Errors
    /// Returns an authority or aggregate-budget failure.
    pub fn compare_subject(
        &self,
        subject: &PortableWorkspaceSubject,
    ) -> Result<PortableWorkspaceSubjectComparison, RecipientQualifiedWorkspaceInspectionError>
    {
        let permit = self
            .admitted
            .authorize_compare_subject(subject.application_subject())?;
        debug_assert!(permit.matches(subject.application_subject()));
        let comparison = self.comparison.compare_subject(subject);
        self.admitted
            .budget()
            .charge_output(
                0,
                u64::try_from(comparison.canonical_bytes().len()).unwrap_or(u64::MAX),
            )
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        Ok(comparison)
    }

    /// Page contributing roots after fresh recipient admission.
    ///
    /// # Errors
    /// Returns an authority, budget, custody, or SDK projection failure.
    pub fn contributing_roots(
        &self,
        subject: &PortableWorkspaceSubject,
        after: Option<&RecipientQualifiedContributingRootsCursor>,
        limit: u16,
    ) -> Result<RecipientQualifiedContributingRootsPage, RecipientQualifiedWorkspaceInspectionError>
    {
        let permit = self.admitted.authorize_contributing_roots(
            subject.application_subject(),
            after.is_some(),
            limit,
        )?;
        debug_assert!(permit.matches(subject.application_subject(), after.is_some(), limit));
        let page = self.comparison.contributing_roots(
            subject,
            after.map(|cursor| &cursor.inner),
            limit,
        )?;
        self.admitted
            .budget()
            .charge_output(
                0,
                u64::try_from(page.canonical_bytes().len()).unwrap_or(u64::MAX),
            )
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        Ok(RecipientQualifiedContributingRootsPage {
            continuation: page
                .continuation()
                .cloned()
                .map(|inner| RecipientQualifiedContributingRootsCursor { inner }),
            inner: page,
        })
    }

    /// Page direct dependencies on one selected side after fresh admission.
    ///
    /// # Errors
    /// Returns an authority, budget, custody, or SDK projection failure.
    pub fn direct_dependencies(
        &self,
        side: OrganizationComparisonSide,
        subject: &PortableWorkspaceSubject,
        after: Option<&RecipientQualifiedDependencyCursor>,
        limit: u16,
    ) -> Result<RecipientQualifiedDependencyPage, RecipientQualifiedWorkspaceInspectionError> {
        let selected_side = application_side(side);
        let permit = self.admitted.authorize_direct_dependencies(
            selected_side,
            subject.application_subject(),
            after.is_some(),
            limit,
        )?;
        debug_assert!(permit.matches(
            selected_side,
            subject.application_subject(),
            after.is_some(),
            limit
        ));
        let owner = self.compilation(side);
        let page = owner.direct_dependencies(subject, after.map(|cursor| &cursor.inner), limit)?;
        self.admitted
            .budget()
            .charge_output(
                u64::try_from(page.copied_string_bytes()).unwrap_or(u64::MAX),
                u64::try_from(page.canonical_bytes().len()).unwrap_or(u64::MAX),
            )
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        Ok(RecipientQualifiedDependencyPage {
            continuation: page
                .continuation()
                .cloned()
                .map(|inner| RecipientQualifiedDependencyCursor { inner }),
            inner: page,
        })
    }

    /// Inspect proof and source evidence on one selected side after fresh admission.
    ///
    /// # Errors
    /// Returns an authority, budget, custody, or SDK projection failure.
    pub fn inspect_subject(
        &self,
        side: OrganizationComparisonSide,
        subject: &PortableWorkspaceSubject,
        after: Option<&RecipientQualifiedSubjectInspectionCursor>,
        input_limit: u16,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<RecipientQualifiedSubjectInspection, RecipientQualifiedWorkspaceInspectionError>
    {
        let selected_side = application_side(side);
        let permit = self.admitted.authorize_subject_inspection(
            selected_side,
            subject.application_subject(),
            after.is_some(),
            input_limit,
            usize::from(maximum_inspected),
            usize::from(maximum_returned),
        )?;
        debug_assert!(permit.matches(
            selected_side,
            subject.application_subject(),
            after.is_some(),
            input_limit,
            usize::from(maximum_inspected),
            usize::from(maximum_returned)
        ));
        let page = self.compilation(side).inspect_subject(
            subject,
            after.map(|cursor| &cursor.inner),
            input_limit,
            maximum_inspected,
            maximum_returned,
        )?;
        self.admitted
            .budget()
            .charge_output(
                u64::from(page.dependency_copied_string_bytes())
                    .saturating_add(u64::from(page.source_copied_string_bytes())),
                u64::try_from(page.canonical_bytes().len()).unwrap_or(u64::MAX),
            )
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        Ok(RecipientQualifiedSubjectInspection {
            continuation: page
                .continuation()
                .cloned()
                .map(|inner| RecipientQualifiedSubjectInspectionCursor { inner }),
            inner: page,
        })
    }

    fn compilation(&self, side: OrganizationComparisonSide) -> &PortableWorkspaceCompilation {
        match side {
            OrganizationComparisonSide::Prior => &self.prior,
            OrganizationComparisonSide::Current => &self.current,
        }
    }

    pub(super) fn raw_comparison(&self) -> &PortableWorkspaceComparison {
        &self.comparison
    }

    pub(super) fn admitted(&self) -> &ApplicationAdmittedComparison {
        &self.admitted
    }
}

#[cfg(test)]
#[path = "../../tests/internal/portable_workspace/recipient_comparison_fixture.rs"]
mod tests;

fn application_side(side: OrganizationComparisonSide) -> WorkspaceInspectionComparisonSide {
    match side {
        OrganizationComparisonSide::Prior => WorkspaceInspectionComparisonSide::Prior,
        OrganizationComparisonSide::Current => WorkspaceInspectionComparisonSide::Current,
    }
}

/// Opaque continuation for admitted subject enumeration.
#[derive(Clone)]
pub struct RecipientQualifiedComparisonSubjectCursor {
    inner: PortableWorkspaceComparisonSubjectCursor,
}

/// Subject page with a continuation that can only resume through its admitted owner.
pub struct RecipientQualifiedComparisonSubjectPage {
    inner: PortableWorkspaceComparisonSubjectPage,
    continuation: Option<RecipientQualifiedComparisonSubjectCursor>,
}

impl RecipientQualifiedComparisonSubjectPage {
    /// Exact comparison coordinates and receipt identity.
    #[must_use]
    pub const fn comparison(&self) -> &PortableWorkspaceComparisonSummary {
        self.inner.comparison()
    }
    /// Subjects selected on either side in canonical order.
    #[must_use]
    pub fn subjects(&self) -> &[PortableWorkspaceSubject] {
        self.inner.subjects()
    }
    /// Native entries examined for this bounded merge page.
    #[must_use]
    pub const fn inspected(&self) -> usize {
        self.inner.inspected()
    }
    /// Copied variable-length identity bytes.
    #[must_use]
    pub const fn subject_identity_bytes(&self) -> usize {
        self.inner.subject_identity_bytes()
    }
    /// Resume only through the same qualified comparison owner.
    #[must_use]
    pub const fn continuation(&self) -> Option<&RecipientQualifiedComparisonSubjectCursor> {
        self.continuation.as_ref()
    }
    /// Canonical projection without opaque custody.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        self.inner.canonical_bytes()
    }
}

/// Opaque continuation for admitted contributing-root inspection.
#[derive(Clone)]
pub struct RecipientQualifiedContributingRootsCursor {
    inner: PortableWorkspaceContributingRootsCursor,
}

/// Contributing roots with an authority-bound continuation.
pub struct RecipientQualifiedContributingRootsPage {
    inner: PortableWorkspaceContributingRootsPage,
    continuation: Option<RecipientQualifiedContributingRootsCursor>,
}

impl RecipientQualifiedContributingRootsPage {
    /// Retained comparison coordinates and receipt identity.
    #[must_use]
    pub const fn comparison(&self) -> &PortableWorkspaceComparisonSummary {
        self.inner.comparison()
    }
    /// Subject selected by this query.
    #[must_use]
    pub const fn subject(&self) -> &PortableWorkspaceSubject {
        self.inner.subject()
    }
    /// Current-partition state.
    #[must_use]
    pub const fn state(&self) -> super::PortableWorkspaceContributingRootsState {
        self.inner.state()
    }
    /// Exact contributing roots.
    #[must_use]
    pub fn roots(&self) -> &[super::PortableWorkspaceChangeRoot] {
        self.inner.roots()
    }
    /// Resume only through the same qualified comparison owner.
    #[must_use]
    pub const fn continuation(&self) -> Option<&RecipientQualifiedContributingRootsCursor> {
        self.continuation.as_ref()
    }
    /// Canonical projection without opaque custody.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        self.inner.canonical_bytes()
    }
}

/// Opaque continuation for admitted direct-dependency inspection.
#[derive(Clone)]
pub struct RecipientQualifiedDependencyCursor {
    inner: PortableWorkspaceDependencyCursor,
}

/// Direct dependencies with an authority-bound continuation.
pub struct RecipientQualifiedDependencyPage {
    inner: PortableWorkspaceDependencyPage,
    continuation: Option<RecipientQualifiedDependencyCursor>,
}

impl RecipientQualifiedDependencyPage {
    /// Principal-relative universe identity of the selected side.
    #[must_use]
    pub fn principal_universe_digest(&self) -> &str {
        self.inner.principal_universe_digest()
    }
    /// Semantic identity of the selected side.
    #[must_use]
    pub fn semantic_ir_digest(&self) -> &str {
        self.inner.semantic_ir_digest()
    }
    /// Selected subject.
    #[must_use]
    pub const fn subject(&self) -> &PortableWorkspaceSubject {
        self.inner.subject()
    }
    /// Selected compiler fact, when present.
    #[must_use]
    pub const fn root(&self) -> Option<&super::PortableWorkspaceCausalStep> {
        self.inner.root()
    }
    /// Direct proof inputs.
    #[must_use]
    pub fn dependencies(&self) -> &[super::PortableWorkspaceCausalStep] {
        self.inner.dependencies()
    }
    /// Copied dependency string bytes.
    #[must_use]
    pub const fn copied_string_bytes(&self) -> usize {
        self.inner.copied_string_bytes()
    }
    /// Resume only through the same qualified comparison owner.
    #[must_use]
    pub const fn continuation(&self) -> Option<&RecipientQualifiedDependencyCursor> {
        self.continuation.as_ref()
    }
    /// Canonical projection without opaque custody.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        self.inner.canonical_bytes()
    }
}

/// Opaque continuation for admitted source-and-proof inspection.
#[derive(Clone)]
pub struct RecipientQualifiedSubjectInspectionCursor {
    inner: PortableWorkspaceSubjectInspectionCursor,
}

/// Subject proof and source evidence with an authority-bound continuation.
pub struct RecipientQualifiedSubjectInspection {
    inner: PortableWorkspaceSubjectInspection,
    continuation: Option<RecipientQualifiedSubjectInspectionCursor>,
}

impl RecipientQualifiedSubjectInspection {
    /// Exact workspace identity of the selected side.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        self.inner.workspace_id()
    }
    /// Full selected subject.
    #[must_use]
    pub const fn subject(&self) -> &PortableWorkspaceSubject {
        self.inner.subject()
    }
    /// Selected compiler fact, when present.
    #[must_use]
    pub const fn root(&self) -> Option<&super::PortableWorkspaceCausalStep> {
        self.inner.root()
    }
    /// This page's direct proof inputs.
    #[must_use]
    pub fn dependencies(&self) -> &[super::PortableWorkspaceCausalStep] {
        self.inner.dependencies()
    }
    /// Exact named source occurrences.
    #[must_use]
    pub fn occurrences(&self) -> &[crate::AuthoringOccurrence] {
        self.inner.occurrences()
    }
    /// Source candidates examined by this call.
    #[must_use]
    pub const fn inspected(&self) -> u16 {
        self.inner.inspected()
    }
    /// Copied dependency string bytes.
    #[must_use]
    pub const fn dependency_copied_string_bytes(&self) -> u32 {
        self.inner.dependency_copied_string_bytes()
    }
    /// Copied source-location string bytes.
    #[must_use]
    pub const fn source_copied_string_bytes(&self) -> u32 {
        self.inner.source_copied_string_bytes()
    }
    /// Resume only through the same qualified comparison owner.
    #[must_use]
    pub const fn continuation(&self) -> Option<&RecipientQualifiedSubjectInspectionCursor> {
        self.continuation.as_ref()
    }
    /// Canonical projection without opaque custody.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        self.inner.canonical_bytes()
    }
}
