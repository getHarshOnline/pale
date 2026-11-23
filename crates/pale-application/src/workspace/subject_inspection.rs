//! Inspect one compiler subject without traversing its transitive closure.

use pale_core::records::{WorkspaceDerivationNodeV1, WorkspaceDerivationSubjectV1};

use super::{
    AuthoringOccurrencePage, AuthoringOccurrencePageCursor, InvalidDependencyOffset,
    PortableWorkspaceDependencyPage, PortableWorkspaceSubjectOccurrences,
    PortableWorkspaceUniverse, WorkspaceAuthoringLocation,
};

/// Independent application-local positions for the two bounded sections.
///
/// A missing outer cursor starts both sections. Within a cursor, a missing
/// position means that section is done, never restart-at-zero. The SDK must
/// bind this opaque value to its exact compilation, full subject and raw limits.
/// It carries no source text or transferable authority.
#[derive(Clone)]
pub struct PortableWorkspaceSubjectInspectionCursor {
    input_offset: Option<usize>,
    source_after: Option<AuthoringOccurrencePageCursor>,
}

impl std::fmt::Debug for PortableWorkspaceSubjectInspectionCursor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceSubjectInspectionCursor")
            .finish_non_exhaustive()
    }
}

/// A retained position or membership invariant failed during inspection.
#[derive(Debug, thiserror::Error)]
pub enum PortableWorkspaceSubjectInspectionError {
    /// The direct-input position does not fit this subject's retained list.
    #[error("subject inspection input position is invalid: {0}")]
    InvalidInputPosition(#[from] InvalidDependencyOffset),
    /// Two queries of the same immutable compiler owner disagree on selection.
    #[error("subject inspection source membership disagrees with its selected fact")]
    MembershipIncoherent,
}

/// A consistent borrowed fact, direct-input page and direct source evidence.
///
/// Root absence means unselected, not absence from captured source. A record
/// anchor is separate from lexical occurrences and need not exist. Source
/// rows preserve their existing definition, reference and projection roles.
/// The result borrows one admitted owner, without copying its graph or bodies.
pub struct PortableWorkspaceSubjectInspection<'a> {
    root: Option<&'a WorkspaceDerivationNodeV1>,
    dependencies: Option<PortableWorkspaceDependencyPage<'a>>,
    sources: Option<AuthoringOccurrencePage<'a>>,
    record_anchor: Option<&'a WorkspaceAuthoringLocation>,
    dependency_copied_string_bytes: usize,
    source_copied_string_bytes: usize,
    continuation: Option<PortableWorkspaceSubjectInspectionCursor>,
}

impl<'a> PortableWorkspaceSubjectInspection<'a> {
    /// Repeat the selected fact on every page, or none for an unselected subject.
    #[must_use]
    pub const fn root(&self) -> Option<&'a WorkspaceDerivationNodeV1> {
        self.root
    }

    /// Borrow this call's direct inputs; none when unselected or already done.
    #[must_use]
    pub const fn dependency_page(&self) -> Option<&PortableWorkspaceDependencyPage<'a>> {
        self.dependencies.as_ref()
    }

    /// Borrow this call's named matches and inspected count; none when already done.
    #[must_use]
    pub const fn source_page(&self) -> Option<&AuthoringOccurrencePage<'a>> {
        self.sources.as_ref()
    }

    /// Borrow an owning import-path or extension record, not a named occurrence.
    #[must_use]
    pub const fn record_anchor(&self) -> Option<&'a WorkspaceAuthoringLocation> {
        self.record_anchor
    }

    /// UTF-8 bytes copied into this page's dependency-step string fields.
    #[must_use]
    pub const fn dependency_copied_string_bytes(&self) -> usize {
        self.dependency_copied_string_bytes
    }

    /// UTF-8 bytes copied into occurrence and repeated record-anchor locations.
    #[must_use]
    pub const fn source_copied_string_bytes(&self) -> usize {
        self.source_copied_string_bytes
    }

    /// Whether all direct inputs have been returned, independently of sources.
    #[must_use]
    pub fn inputs_complete(&self) -> bool {
        self.continuation
            .as_ref()
            .is_none_or(|cursor| cursor.input_offset.is_none())
    }

    /// Whether the source scan proved exhaustion, independently of inputs.
    #[must_use]
    pub fn sources_complete(&self) -> bool {
        self.continuation
            .as_ref()
            .is_none_or(|cursor| cursor.source_after.is_none())
    }

    /// Borrow unfinished section positions. Absence means both sections are done.
    #[must_use]
    pub const fn continuation(&self) -> Option<&PortableWorkspaceSubjectInspectionCursor> {
        self.continuation.as_ref()
    }
}

impl PortableWorkspaceUniverse {
    /// Inspect one subject's fact, direct inputs and source evidence in bounded pages.
    ///
    /// Reuses the owning subject lookup and source scan. Each zero limit selects
    /// 64; each section is independently capped at 256. Completed sections are
    /// never restarted. The root and optional record anchor repeat on later pages.
    /// An empty source page may remain pending until exhaustion is proved.
    ///
    /// Work includes full-key comparisons, root and anchor lookups, up to the
    /// direct-input limit of node lookups, and the source owner's charged scan.
    /// The result reports dependency and source string bytes separately. Source
    /// bytes include the occurrence page plus a repeated record anchor, when one
    /// exists; only the occurrence page has its own fixed string-byte ceiling.
    /// These limits do not bound elapsed time, aggregate memory or disclosure.
    /// Caller-supplied positions must be bound to this exact owner, subject and
    /// all three raw limits before invocation. No recipient authority is admitted.
    ///
    /// # Errors
    /// Returns a sourced invalid position or an internal selected-membership
    /// inconsistency. Unselected subjects and missing source evidence succeed.
    pub fn inspect_subject(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
        after: Option<&PortableWorkspaceSubjectInspectionCursor>,
        input_limit: u16,
        maximum_inspected: usize,
        maximum_returned: usize,
    ) -> Result<PortableWorkspaceSubjectInspection<'_>, PortableWorkspaceSubjectInspectionError>
    {
        let inputs_pending = after.is_none_or(|cursor| cursor.input_offset.is_some());
        let dependencies = if inputs_pending {
            self.direct_dependencies(
                subject,
                after.and_then(|cursor| cursor.input_offset).unwrap_or(0),
                input_limit,
            )?
        } else {
            None
        };
        let root = if inputs_pending {
            dependencies
                .as_ref()
                .map(PortableWorkspaceDependencyPage::root)
        } else {
            self.subject_root(subject)
        };
        let sources = if after.is_none_or(|cursor| cursor.source_after.is_some()) {
            Some(consistent_source_page(
                root.is_some(),
                self.subject_occurrences(
                    subject,
                    after.and_then(|cursor| cursor.source_after.as_ref()),
                    maximum_inspected,
                    maximum_returned,
                ),
            )?)
        } else {
            None
        };
        let input_offset = dependencies
            .as_ref()
            .and_then(PortableWorkspaceDependencyPage::next_offset);
        let source_after = sources
            .as_ref()
            .and_then(AuthoringOccurrencePage::continuation)
            .cloned();
        let continuation = (input_offset.is_some() || source_after.is_some()).then_some(
            PortableWorkspaceSubjectInspectionCursor {
                input_offset,
                source_after,
            },
        );
        let dependency_copied_string_bytes = dependencies
            .as_ref()
            .map_or(0, PortableWorkspaceDependencyPage::copied_string_bytes);
        let record_anchor = self.subject_record_anchor(subject);
        let source_copied_string_bytes =
            subject_source_copied_string_bytes(sources.as_ref(), record_anchor);
        Ok(PortableWorkspaceSubjectInspection {
            root,
            dependencies,
            sources,
            record_anchor,
            dependency_copied_string_bytes,
            source_copied_string_bytes,
            continuation,
        })
    }
}

fn subject_source_copied_string_bytes(
    sources: Option<&AuthoringOccurrencePage<'_>>,
    record_anchor: Option<&WorkspaceAuthoringLocation>,
) -> usize {
    sources
        .map_or(0, AuthoringOccurrencePage::copied_string_bytes)
        .checked_add(record_anchor.map_or(0, WorkspaceAuthoringLocation::copied_string_bytes))
        .expect("subject source copied-string accounting overflowed")
}

fn consistent_source_page(
    root_selected: bool,
    sources: PortableWorkspaceSubjectOccurrences<'_>,
) -> Result<AuthoringOccurrencePage<'_>, PortableWorkspaceSubjectInspectionError> {
    let (source_selected, page) = match sources {
        PortableWorkspaceSubjectOccurrences::Selected(page) => (true, page),
        PortableWorkspaceSubjectOccurrences::Unselected(page) => (false, page),
    };
    if source_selected != root_selected {
        return Err(PortableWorkspaceSubjectInspectionError::MembershipIncoherent);
    }
    Ok(page)
}

#[cfg(test)]
#[path = "../../tests/internal/workspace/subject_inspection.rs"]
mod tests;
