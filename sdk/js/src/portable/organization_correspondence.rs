// Organization correspondence over the retained SDK comparison pair.

/// The compilation supplying the selected record; search runs on the other side.
#[napi]
pub enum OrganizationComparisonSide {
    /// Select from the prior compilation and search the current compilation.
    Prior,
    /// Select from the current compilation and search the prior compilation.
    Current,
}

impl OrganizationComparisonSide {
    fn into_api(self) -> pale_sdk::OrganizationComparisonSide {
        match self {
            Self::Prior => pale_sdk::OrganizationComparisonSide::Prior,
            Self::Current => pale_sdk::OrganizationComparisonSide::Current,
        }
    }
    fn from_api(value: pale_sdk::OrganizationComparisonSide) -> Self {
        match value {
            pale_sdk::OrganizationComparisonSide::Prior => Self::Prior,
            pale_sdk::OrganizationComparisonSide::Current => Self::Current,
        }
    }
}

/// Retains the exact pair, selected record, direction and original raw limits.
/// No public constructor or serialized position; see [`OrganizationCursor`] for retention costs.
#[napi]
pub struct OrganizationCorrespondenceCursor {
    inner: pale_sdk::OrganizationCorrespondenceCursor,
}

/// Incomplete search, complete absence, or a same-key canonical value comparison.
#[expect(
    clippy::large_enum_variant,
    reason = "This output-only value goes directly to napi conversion; boxing adds an allocation without reducing retained state."
)]
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum OrganizationCorrespondenceState {
    /// Resume the exact search; this is not absence.
    Searching {
        /// Continuation with unchanged pair, selection and raw limits.
        cursor: OrganizationCorrespondenceCursor,
    },
    /// The other-side family has no matching canonical key.
    Unmatched,
    /// Matching keys retain separate source owners; values use existing typed equality.
    Matched {
        /// Shallow row and retained source/detail handle from the opposite side.
        other: OrganizationEntry,
        /// Complete canonical value comparison, not shallow row equality.
        values: WorkspaceValueComparison,
    },
}

/// One bounded query result retaining its comparison pair and original selection.
#[napi]
pub struct OrganizationCorrespondence {
    inner: pale_sdk::OrganizationCorrespondence,
}

#[napi]
impl WorkspaceComparison {
    /// Find the selected record's canonical key in the other retained compilation.
    /// Visits and copied UTF-8 payload are separate SDK bounds, not time or total memory limits.
    /// A failure publishes no continuation; keep the original cursor for retry.
    /// This does not authorize recipient delivery or equate source identities across generations.
    /// # Errors
    /// Preserves SDK pair, record, side, limit and copy diagnostics. Malformed numeric
    /// transport limits reject as `InvalidArg` before the SDK query.
    #[napi]
    pub fn correspond_organization_record(
        &self,
        env: Env,
        selected_side: OrganizationComparisonSide,
        selected: &OrganizationRecordHandle,
        maximum_inspected: f64,
        maximum_copied_bytes: f64,
        after: Option<&OrganizationCorrespondenceCursor>,
    ) -> Result<OrganizationCorrespondence> {
        self.inner()
            .correspond_organization_record(
                selected_side.into_api(),
                &selected.inner,
                after.map(|value| &value.inner),
                authoring_page_limit(maximum_inspected)?,
                organization_copy_limit(maximum_copied_bytes)?,
            )
            .map(|inner| OrganizationCorrespondence { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }
}

#[napi]
impl OrganizationRecordHandle {
    /// Exact retained owner and record, including handles issued by independent pages.
    /// Equal values in another compilation do not identify the same record handle.
    #[napi]
    pub fn same_record(&self, other: &OrganizationRecordHandle) -> bool {
        self.inner.same_record(&other.inner)
    }
}

#[napi]
impl OrganizationCorrespondence {
    /// Exact ordered comparison pair and receipt membership.
    #[napi]
    pub fn belongs_to(&self, comparison: &WorkspaceComparison) -> bool {
        self.inner.belongs_to(comparison.inner())
    }
    /// Side supplying the original selection.
    #[napi(getter)]
    pub fn selected_side(&self) -> OrganizationComparisonSide {
        OrganizationComparisonSide::from_api(self.inner.selected_side())
    }
    /// Independently retain the original record, without reconstructing its identity.
    #[napi(getter)]
    pub fn selected(&self) -> OrganizationRecordHandle {
        OrganizationRecordHandle {
            inner: self.inner.selected().clone(),
        }
    }
    /// Copy the bounded result projection; handles retain their original owners.
    #[napi(getter)]
    pub fn state(&self) -> OrganizationCorrespondenceState {
        use pale_sdk::OrganizationCorrespondenceState as State;
        match self.inner.state() {
            State::Searching(cursor) => OrganizationCorrespondenceState::Searching {
                cursor: OrganizationCorrespondenceCursor {
                    inner: cursor.clone(),
                },
            },
            State::Unmatched => OrganizationCorrespondenceState::Unmatched,
            State::Matched { other, values } => OrganizationCorrespondenceState::Matched {
                other: OrganizationEntry {
                    inner: other.as_ref().clone(),
                },
                values: match values {
                    pale_sdk::PortableWorkspaceValueComparison::Equal => {
                        WorkspaceValueComparison::Equal
                    }
                    pale_sdk::PortableWorkspaceValueComparison::Changed => {
                        WorkspaceValueComparison::Changed
                    }
                },
            },
        }
    }
    /// Candidate keys examined, including nonmatches.
    #[napi(getter)]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected()
    }
    /// SDK returned UTF-8 payload, excluding retained owners and repeated getter copies.
    #[napi(getter)]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes()
    }
}
