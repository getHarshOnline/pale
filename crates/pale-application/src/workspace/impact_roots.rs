//! Page the verified change roots that contribute to one current subject.

use pale_core::records::{
    WorkspaceDerivationImpactStateV1, WorkspaceDerivationRootDigest, WorkspaceDerivationRootV1,
    WorkspaceDerivationSubjectV1,
};

use super::PortableWorkspaceDerivation;

/// Default number of contributing roots returned by one page.
pub const DEFAULT_CONTRIBUTING_ROOT_PAGE_LIMIT: u16 = 64;
/// Largest number of contributing roots returned by one page.
pub const MAX_CONTRIBUTING_ROOT_PAGE_LIMIT: u16 = 256;

/// A position beyond this subject's verified contributing-root list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("contributing-root page position exceeds the retained list")]
pub struct InvalidContributingRootOffset;

/// Membership in the current comparison partition and its contributing roots.
///
/// This classifies dependency impact, not equality of prior/current values.
pub enum PortableWorkspaceContributingRoots<'a> {
    /// The subject is absent from the current partition; it may exist in the prior input.
    NotPresent,
    /// The current subject is outside every verified change-root closure.
    Unaffected,
    /// One page of the current subject's nonempty complete contributing-root set.
    Affected(PortableWorkspaceContributingRootPage<'a>),
}

/// Borrow part of one already-verified root set without recomputing reachability.
pub struct PortableWorkspaceContributingRootPage<'a> {
    selected: &'a [WorkspaceDerivationRootDigest],
    roots: &'a [WorkspaceDerivationRootV1],
    next_offset: Option<usize>,
}

impl<'a> PortableWorkspaceContributingRootPage<'a> {
    /// Borrow roots in the receipt's canonical root-digest order.
    ///
    /// Consuming the iterator performs one binary lookup per returned root.
    /// It does not scan other roots or build a proof index.
    ///
    /// # Panics
    /// Panics if an internal defect leaves an already-verified contribution
    /// without its root. Callers cannot construct this borrowed page.
    pub fn roots(&self) -> impl ExactSizeIterator<Item = &'a WorkspaceDerivationRootV1> + '_ {
        self.selected.iter().map(|digest| {
            let index = self
                .roots
                .binary_search_by(|root| root.root_digest.cmp(digest))
                .expect("verified contribution names one retained change root");
            &self.roots[index]
        })
    }

    /// Next application-local offset, or none when the list is exhausted.
    ///
    /// An outer SDK must bind this position to the exact comparison and query.
    #[must_use]
    pub const fn next_offset(&self) -> Option<usize> {
        self.next_offset
    }
}

impl PortableWorkspaceDerivation {
    /// Read the roots that affect one current subject from the retained receipt.
    ///
    /// Zero selects 64 roots; larger requests are capped at 256. The query
    /// performs one full-subject binary lookup and at most one root lookup per
    /// returned entry. It borrows the sealed relation: no graph traversal,
    /// visited set, reverse index, source read, or receipt reconstruction occurs.
    /// These limits bound rows and lookups, not encoded bytes or disclosure.
    ///
    /// An exact-end offset returns an empty affected page with no continuation.
    /// Not-present and unaffected results have no entries and require offset zero.
    /// Root order is digest order, not the separate explanation's preferred-root rank.
    ///
    /// # Errors
    /// Rejects an offset beyond the selected subject's contribution list.
    pub fn contributing_roots(
        &self,
        subject: &WorkspaceDerivationSubjectV1,
        offset: usize,
        requested_limit: u16,
    ) -> Result<PortableWorkspaceContributingRoots<'_>, InvalidContributingRootOffset> {
        let receipt = self.receipt();
        let Ok(index) = receipt
            .impacts
            .binary_search_by(|impact| impact.subject.cmp(subject))
        else {
            return if offset == 0 {
                Ok(PortableWorkspaceContributingRoots::NotPresent)
            } else {
                Err(InvalidContributingRootOffset)
            };
        };
        let WorkspaceDerivationImpactStateV1::Affected { root_digests } =
            &receipt.impacts[index].state
        else {
            return if offset == 0 {
                Ok(PortableWorkspaceContributingRoots::Unaffected)
            } else {
                Err(InvalidContributingRootOffset)
            };
        };
        let remaining = root_digests
            .get(offset..)
            .ok_or(InvalidContributingRootOffset)?;
        let limit = if requested_limit == 0 {
            DEFAULT_CONTRIBUTING_ROOT_PAGE_LIMIT
        } else {
            requested_limit.min(MAX_CONTRIBUTING_ROOT_PAGE_LIMIT)
        };
        let count = remaining.len().min(usize::from(limit));
        Ok(PortableWorkspaceContributingRoots::Affected(
            PortableWorkspaceContributingRootPage {
                selected: &remaining[..count],
                roots: &receipt.roots,
                next_offset: (count < remaining.len()).then_some(offset + count),
            },
        ))
    }
}
