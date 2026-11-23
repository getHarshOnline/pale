//! Find one-hop consumers in the retained compiler-proof frontier.

use pale_core::records::WorkspaceDerivationNodeV1;

/// Default candidate and returned-row limit for a direct-dependent page.
pub const DEFAULT_DEPENDENT_PAGE_LIMIT: u16 = 64;
/// Largest candidate or returned-row limit for a direct-dependent page.
pub const MAX_DEPENDENT_PAGE_LIMIT: u16 = 256;
/// Largest number of candidate-input comparisons performed by one page.
///
/// This independent work ceiling prevents 256 individually bounded fan-in
/// searches from becoming one unbounded aggregate. It is larger than the
/// worst possible binary search for one admitted input vector, so every
/// nonterminal page can still inspect at least one candidate.
pub const MAX_DEPENDENT_PAGE_INPUT_COMPARISONS: usize = 1_024;
const _: () = assert!(
    MAX_DEPENDENT_PAGE_INPUT_COMPARISONS > usize::BITS as usize,
    "one addressable input vector must fit the page work ceiling"
);

/// A candidate position beyond the retained proof-node array.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("dependent page position exceeds the retained candidate list")]
pub struct InvalidDependentOffset;

/// Borrow a selected fact and a bounded page of its direct consumers.
///
/// Each returned node names the root digest in its sorted direct inputs. The
/// edge is root to consumer under the consumer's existing derivation rule;
/// this does not claim transitive impact or an operational prerequisite.
/// Results follow sealed node-digest order. Only a bounded vector of borrowed
/// matches is allocated, not a graph copy, reverse index or visited set.
pub struct PortableWorkspaceDependentPage<'a> {
    root: &'a WorkspaceDerivationNodeV1,
    dependents: Vec<&'a WorkspaceDerivationNodeV1>,
    inspected: usize,
    input_digest_comparisons: usize,
    next_offset: Option<usize>,
}

impl<'a> PortableWorkspaceDependentPage<'a> {
    pub(in crate::workspace) fn new(
        root: &'a WorkspaceDerivationNodeV1,
        nodes: &'a [WorkspaceDerivationNodeV1],
        offset: usize,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<Self, InvalidDependentOffset> {
        let remaining = nodes.get(offset..).ok_or(InvalidDependentOffset)?;
        let inspected_limit = page_limit(maximum_inspected);
        let returned_limit = page_limit(maximum_returned);
        let mut dependents =
            Vec::with_capacity(remaining.len().min(inspected_limit).min(returned_limit));
        let mut inspected = 0;
        let mut input_digest_comparisons = 0_usize;
        for candidate in remaining.iter().take(inspected_limit) {
            let comparison_bound = binary_search_comparison_bound(candidate.inputs.len());
            if input_digest_comparisons
                .checked_add(comparison_bound)
                .is_none_or(|total| total > MAX_DEPENDENT_PAGE_INPUT_COMPARISONS)
            {
                break;
            }
            inspected += 1;
            if candidate
                .inputs
                .binary_search_by(|input| {
                    input_digest_comparisons += 1;
                    input.cmp(&root.node_digest)
                })
                .is_ok()
            {
                dependents.push(candidate);
                if dependents.len() == returned_limit {
                    break;
                }
            }
        }
        let next = offset + inspected;
        Ok(Self {
            root,
            dependents,
            inspected,
            input_digest_comparisons,
            next_offset: (next < nodes.len()).then_some(next),
        })
    }

    /// Selected fact consumed by each returned node.
    #[must_use]
    pub const fn root(&self) -> &'a WorkspaceDerivationNodeV1 {
        self.root
    }

    /// Borrow the matching consumers without performing another search.
    pub fn dependents(&self) -> impl ExactSizeIterator<Item = &'a WorkspaceDerivationNodeV1> + '_ {
        self.dependents.iter().copied()
    }

    /// Candidate nodes examined, including empty-input nodes and nonmatches.
    #[must_use]
    pub const fn inspected(&self) -> usize {
        self.inspected
    }

    /// Actual comparator calls made while searching candidate input arrays.
    ///
    /// This is mechanical work accounting, not semantic identity or proof.
    /// The standard library's binary-search implementation can change the
    /// exact count between toolchains. Root lookup comparisons are excluded.
    #[must_use]
    pub const fn input_digest_comparisons(&self) -> usize {
        self.input_digest_comparisons
    }

    /// Next candidate position, or none after every candidate was examined.
    ///
    /// An empty page can still be incomplete. This local offset has no owner
    /// custody; an outer cursor must bind owner, full subject and both raw limits.
    #[must_use]
    pub const fn next_offset(&self) -> Option<usize> {
        self.next_offset
    }
}

fn binary_search_comparison_bound(len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (usize::BITS - len.leading_zeros()) as usize + 1
    }
}

fn page_limit(requested: u16) -> usize {
    usize::from(if requested == 0 {
        DEFAULT_DEPENDENT_PAGE_LIMIT
    } else {
        requested.min(MAX_DEPENDENT_PAGE_LIMIT)
    })
}
