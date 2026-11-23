//! Enumerate selected subject identities across two retained compiler frontiers.

use pale_core::records::WorkspaceDerivationSubjectV1;

pub(super) const DEFAULT_COMPARISON_SUBJECT_LIMIT: u16 = 64;
pub(super) const MAX_COMPARISON_SUBJECT_LIMIT: u16 = 256;
/// Maximum UTF-8 bytes in variable-length subject identity fields on one page.
///
/// This bounds the strings copied by SDK projections independently of the row
/// limit. Fixed enum and vector storage remains bounded by the 256-row ceiling;
/// canonical encoding bytes are a separate measurement.
pub const MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES: usize = 4 * 1024;

/// Borrow a bounded portion of the ordered subject union, without comparing values.
///
/// A subject occurs once even when both inputs contain it. These are selected
/// compiler identities, not all captured authoring occurrences. Membership and
/// value equality remain available through the existing point query.
pub struct PortableWorkspaceComparisonSubjectPage<'a> {
    pub(super) subjects: Vec<&'a WorkspaceDerivationSubjectV1>,
    pub(super) subject_identity_bytes: usize,
    pub(super) inspected: usize,
    pub(super) key_comparisons: usize,
    pub(super) next_after: Option<&'a WorkspaceDerivationSubjectV1>,
}

impl<'a> PortableWorkspaceComparisonSubjectPage<'a> {
    /// Subjects in the existing full-subject order, with shared keys deduplicated.
    pub fn subjects(&self) -> impl ExactSizeIterator<Item = &'a WorkspaceDerivationSubjectV1> + '_ {
        self.subjects.iter().copied()
    }

    /// UTF-8 bytes in the variable-length identity fields returned by this page.
    ///
    /// This is the copy budget enforced by the application. It excludes fixed
    /// enum storage and the separately produced canonical encoding.
    #[must_use]
    pub const fn subject_identity_bytes(&self) -> usize {
        self.subject_identity_bytes
    }

    /// Map entries fetched, including at most one look-ahead entry per side.
    ///
    /// Look-ahead is charged again when the next page reopens its two ranges.
    /// This count excludes internal range-seek comparisons and output copying.
    #[must_use]
    pub const fn inspected(&self) -> usize {
        self.inspected
    }

    /// Full-key comparisons between the two current merge entries.
    ///
    /// The standard library does not expose comparisons made by range seeks.
    /// Those two seeks are additional work, not included in this count.
    #[must_use]
    pub const fn key_comparisons(&self) -> usize {
        self.key_comparisons
    }

    /// Resume strictly after this full subject, or none when both ranges ended.
    ///
    /// An outer cursor must bind the exact input pair, receipt and raw limit.
    #[must_use]
    pub const fn next_after(&self) -> Option<&'a WorkspaceDerivationSubjectV1> {
        self.next_after
    }
}

pub(crate) fn subject_identity_bytes(subject: &WorkspaceDerivationSubjectV1) -> usize {
    match subject {
        WorkspaceDerivationSubjectV1::Source { source_id } => source_id.as_str().len(),
        WorkspaceDerivationSubjectV1::Component { component_id } => component_id.as_str().len(),
        WorkspaceDerivationSubjectV1::Module { module_id } => module_id.as_str().len(),
        WorkspaceDerivationSubjectV1::Declaration {
            owner_module,
            declaration,
        }
        | WorkspaceDerivationSubjectV1::Policy {
            owner_module,
            declaration,
        } => {
            owner_module.as_str().len()
                + declaration.namespace.as_str().len()
                + declaration.declaration_id.as_str().len()
        }
        WorkspaceDerivationSubjectV1::Import {
            importer_module,
            upstream_module,
            upstream_release,
            export_id,
            source_namespace,
            local_namespace,
        } => {
            importer_module.as_str().len()
                + upstream_module.as_str().len()
                + upstream_release.as_str().len()
                + export_id.as_str().len()
                + source_namespace.as_str().len()
                + local_namespace.as_str().len()
        }
        WorkspaceDerivationSubjectV1::Extension {
            owner_module,
            namespace,
            target,
        } => {
            owner_module.as_str().len()
                + namespace.as_str().len()
                + target.namespace.as_str().len()
                + target.declaration_id.as_str().len()
        }
        WorkspaceDerivationSubjectV1::Resource {
            owner_module,
            resource_id,
        } => owner_module.as_str().len() + resource_id.as_str().len(),
        WorkspaceDerivationSubjectV1::Workspace { workspace_id } => workspace_id.as_str().len(),
        WorkspaceDerivationSubjectV1::ResponsibilityClosure
        | WorkspaceDerivationSubjectV1::OrganizationalTranslationClosure
        | WorkspaceDerivationSubjectV1::ExperienceComposition
        | WorkspaceDerivationSubjectV1::ResolutionLock
        | WorkspaceDerivationSubjectV1::UniverseLock => 0,
    }
}
