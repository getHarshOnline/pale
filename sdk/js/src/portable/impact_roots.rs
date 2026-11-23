// Node projection of the retained comparison's contributing-root query.

/// Opaque continuation bound to the exact ordered inputs, receipt and query.
///
/// Keeps both compiled workspaces alive until garbage collection. Canonical
/// bytes cannot reconstruct it; it grants no disclosure or execution authority.
#[napi(js_name = "WorkspaceContributingRootsCursor")]
pub struct WorkspaceContributingRootsCursor {
    inner: pale_sdk::PortableWorkspaceContributingRootsCursor,
}

/// Current-partition membership and impact, independently of value equality.
#[napi(js_name = "WorkspaceContributingRootsState")]
pub enum WorkspaceContributingRootsState {
    /// Not present in the current partition, even if present in the prior input.
    NotPresent,
    /// Current subject is outside every verified change-root closure.
    Unaffected,
    /// Current subject has contributing roots; a terminal page can be empty.
    Affected,
}

/// Owned roots contributing to one subject in a retained comparison.
///
/// All results keep both compilations alive, including terminal and not-present
/// results. Child getters copy values, not graphs; garbage collection releases
/// this handle's ownership without changing semantic state.
#[napi(js_name = "WorkspaceContributingRootsPage")]
pub struct WorkspaceContributingRootsPage {
    inner: pale_sdk::PortableWorkspaceContributingRootsPage,
}

#[napi]
impl WorkspaceContributingRootsPage {
    /// Copy the parent receipt and independently typed prior/current coordinates.
    #[napi]
    pub fn comparison(&self) -> WorkspaceComparisonSummary {
        WorkspaceComparisonSummary::from_api(self.inner.comparison().clone())
    }

    /// Copy the full subject whose current impact relation was queried.
    #[napi]
    pub fn subject(&self) -> WorkspaceSubject {
        WorkspaceSubject::from_api(self.inner.subject())
    }

    /// Current membership and impact, not a prior/current value comparison.
    #[napi(getter)]
    pub fn state(&self) -> WorkspaceContributingRootsState {
        match self.inner.state() {
            pale_sdk::PortableWorkspaceContributingRootsState::NotPresent => {
                WorkspaceContributingRootsState::NotPresent
            }
            pale_sdk::PortableWorkspaceContributingRootsState::Unaffected => {
                WorkspaceContributingRootsState::Unaffected
            }
            pale_sdk::PortableWorkspaceContributingRootsState::Affected => {
                WorkspaceContributingRootsState::Affected
            }
        }
    }

    /// Copy the contributing roots in canonical root-digest order.
    #[napi]
    pub fn roots(&self) -> Vec<WorkspaceChangeRoot> {
        self.inner
            .roots()
            .iter()
            .cloned()
            .map(|inner| WorkspaceChangeRoot { inner })
            .collect()
    }

    /// Whether the contributing-root set is exhausted.
    #[napi(getter)]
    pub fn is_complete(&self) -> bool {
        self.inner.continuation().is_none()
    }

    /// Retain the next position, or return null after exhaustion.
    #[napi]
    pub fn continuation(&self) -> Option<WorkspaceContributingRootsCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| WorkspaceContributingRootsCursor { inner })
    }

    /// Check exact retained input-pair and receipt custody, not equal digests.
    /// Recreating a comparison over the same retained ordered pair is compatible.
    #[napi]
    pub fn belongs_to(&self, comparison: &WorkspaceComparison) -> bool {
        self.inner.belongs_to(comparison.inner())
    }

    /// Copy the SDK's already-computed canonical answer without re-encoding.
    /// These bytes cannot reconstruct the retained inputs or cursor.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.inner.canonical_bytes().to_vec().into()
    }
}

#[napi]
impl WorkspaceComparison {
    /// Page all verified change roots contributing to one current subject.
    ///
    /// The limit must be an integer from 0 through 65535. The SDK applies its
    /// 64-row default and 256-row cap. Resume with the exact ordered compilation
    /// pair, receipt, full subject and raw limit, even when another limit would
    /// normalize identically. Fresh equal-digest compilations are not substitutes.
    /// This reuses the verified contribution relation, not the preferred path
    /// chosen by explanation. Row limits do not bound bytes, time or aggregate
    /// retained memory. No recipient or native-effect authority is admitted.
    ///
    /// # Errors
    /// Invalid numbers produce `InvalidArg`. Continuation and internal failures
    /// retain the SDK-owned canonical diagnostic.
    #[napi]
    pub fn contributing_roots(
        &self,
        env: Env,
        subject: &WorkspaceSubject,
        limit: f64,
        after: Option<&WorkspaceContributingRootsCursor>,
    ) -> Result<WorkspaceContributingRootsPage> {
        self.inner()
            .contributing_roots(
                &subject.inner,
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(limit)?,
            )
            .map(|inner| WorkspaceContributingRootsPage { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }
}
