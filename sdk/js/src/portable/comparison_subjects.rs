// Node projection of identity discovery across one retained comparison.

/// Opaque subject position bound to the ordered inputs, receipt and raw limit.
///
/// Keeps both compilations alive until garbage collection. Digest strings or
/// serialized identities cannot reconstruct it or grant recipient authority.
#[napi(js_name = "WorkspaceComparisonSubjectCursor")]
pub struct WorkspaceComparisonSubjectCursor {
    inner: pale_sdk::PortableWorkspaceComparisonSubjectCursor,
}

/// Identities selected on either side of one exact comparison, without values.
///
/// Shared identities appear once. Both terminal and pending pages retain the
/// exact input pair. Use the existing point comparison for membership and values;
/// identity discovery does not classify impact or source occurrences.
#[napi(js_name = "WorkspaceComparisonSubjectPage")]
pub struct WorkspaceComparisonSubjectPage {
    inner: pale_sdk::PortableWorkspaceComparisonSubjectPage,
}

#[napi]
impl WorkspaceComparisonSubjectPage {
    /// Copy the existing comparison summary and its exact prior/current coordinates.
    /// Its current-impact counts are not a count of this union's subjects.
    #[napi]
    pub fn comparison(&self) -> WorkspaceComparisonSummary {
        WorkspaceComparisonSummary::from_api(self.inner.comparison().clone())
    }

    /// Copy complete identities, once each in canonical subject order.
    #[napi]
    pub fn subjects(&self) -> Vec<WorkspaceSubject> {
        self.inner
            .subjects()
            .iter()
            .map(WorkspaceSubject::from_api)
            .collect()
    }

    /// UTF-8 bytes copied from the variable-length subject identities.
    #[napi(getter)]
    pub fn subject_identity_bytes(&self) -> u32 {
        self.inner.subject_identity_bytes() as u32
    }

    /// Map entries fetched, including at most one look-ahead entry per side.
    /// Look-ahead is charged again on resume; this excludes range-seek comparisons.
    ///
    /// # Errors
    /// Returns a projection error if the count cannot fit the Node ABI.
    #[napi(getter)]
    pub fn inspected(&self) -> Result<u32> {
        u32::try_from(self.inner.inspected()).map_err(|_| {
            napi::Error::new(
                napi::Status::GenericFailure,
                "comparison subject inspected count exceeds the Node ABI",
            )
        })
    }

    /// Merge-head comparisons, excluding comparisons inside the two range seeks.
    /// This is mechanical work accounting, not semantic or causal identity.
    ///
    /// # Errors
    /// Returns a projection error if the count cannot fit the Node ABI.
    #[napi(getter)]
    pub fn key_comparisons(&self) -> Result<u32> {
        u32::try_from(self.inner.key_comparisons()).map_err(|_| {
            napi::Error::new(
                napi::Status::GenericFailure,
                "comparison subject key count exceeds the Node ABI",
            )
        })
    }

    /// Whether both retained subject ranges are exhausted.
    #[napi(getter)]
    pub fn is_complete(&self) -> bool {
        self.inner.continuation().is_none()
    }

    /// Retain the next full subject position, or return null after exhaustion.
    #[napi]
    pub fn continuation(&self) -> Option<WorkspaceComparisonSubjectCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| WorkspaceComparisonSubjectCursor { inner })
    }

    /// Check the exact retained ordered input pair and receipt, not equal digests.
    /// Recreating a comparison over that same retained pair remains compatible.
    #[napi]
    pub fn belongs_to(&self, comparison: &WorkspaceComparison) -> bool {
        self.inner.belongs_to(comparison.inner())
    }

    /// Copy the SDK's canonical projection without re-encoding opaque custody.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.inner.canonical_bytes().to_vec().into()
    }
}

#[napi]
impl WorkspaceComparison {
    /// Discover selected identities in either input, including prior-only subjects.
    ///
    /// The limit must be an integer from 0 through 65535; the SDK owns the
    /// 64-row default and 256-row cap. Resume with the exact ordered input pair,
    /// receipt and raw limit. A missing cursor starts a new enumeration.
    /// This reuses bounded fact-map ranges, without a full union, graph walk,
    /// receipt rebuild or eager value comparison. The row limit and fixed copied
    /// identity-byte ceiling do not bound time or total retained memory. No
    /// recipient is admitted.
    ///
    /// # Errors
    /// Invalid numbers produce `InvalidArg`. Mismatched continuations retain
    /// the SDK-owned canonical diagnostic and do not silently restart.
    #[napi]
    pub fn subjects(
        &self,
        env: Env,
        limit: f64,
        after: Option<&WorkspaceComparisonSubjectCursor>,
    ) -> Result<WorkspaceComparisonSubjectPage> {
        self.inner()
            .subjects(
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(limit)?,
            )
            .map(|inner| WorkspaceComparisonSubjectPage { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }
}
