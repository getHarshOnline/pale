//! Exact organization correspondence over an already-retained compilation pair.

use super::super::derivation::{
    ComparisonInputs, PortableWorkspaceComparison, PortableWorkspaceValueComparison,
};
use super::*;
use pale_application::workspace::{
    OrganizationCorrespondenceCursor as ApplicationCorrespondenceCursor,
    OrganizationCorrespondenceState as ApplicationState,
    PortableWorkspaceValueComparison as ApplicationValueComparison,
};

/// Which retained compilation supplies the selected record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationComparisonSide {
    /// Search the current compilation for a record selected from the prior one.
    Prior,
    /// Search the prior compilation for a record selected from the current one.
    Current,
}

/// Exact pair, direction, selected record and unchanged raw query limits.
///
/// Retains both complete compilations and their source maps. This opaque value
/// is not a semantic identity, serialized token or recipient permission.
#[derive(Clone)]
pub struct OrganizationCorrespondenceCursor {
    inputs: ComparisonInputs,
    selected_side: OrganizationComparisonSide,
    application: ApplicationCorrespondenceCursor,
    maximum_inspected: u16,
    maximum_copied_bytes: u32,
}

impl std::fmt::Debug for OrganizationCorrespondenceCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrganizationCorrespondenceCursor")
            .finish_non_exhaustive()
    }
}

/// Search progress remains distinct from a complete identity/value comparison.
#[derive(Debug)]
pub enum OrganizationCorrespondenceState {
    /// No match yet. Resume through the accompanying exact continuation.
    Searching(OrganizationCorrespondenceCursor),
    /// The complete other-side family has no matching canonical key.
    Unmatched,
    /// The other-side key matches; complete canonical values were compared.
    Matched {
        /// Existing owned row and source/detail handle from the other side.
        other: Box<OrganizationEntry>,
        /// Equal values do not establish equal source locations or proof ancestry.
        values: PortableWorkspaceValueComparison,
    },
}

/// One bounded search result retaining both sources without copying their bodies.
pub struct OrganizationCorrespondence {
    inputs: ComparisonInputs,
    selected_side: OrganizationComparisonSide,
    selected: OrganizationRecordHandle,
    state: OrganizationCorrespondenceState,
    inspected: u16,
    copied_string_bytes: u32,
}

/// Opaque continuation for one admitted cross-generation record lookup.
#[derive(Clone)]
pub struct RecipientQualifiedOrganizationCorrespondenceCursor {
    inner: OrganizationCorrespondenceCursor,
}

/// Search state that cannot expose a supplied-input record or cursor handle.
pub enum RecipientQualifiedOrganizationCorrespondenceState {
    /// No match yet. Resume through the same qualified comparison owner.
    Searching(RecipientQualifiedOrganizationCorrespondenceCursor),
    /// The complete other-side family has no matching canonical key.
    Unmatched,
    /// The other side has the same key and its complete value was compared.
    Matched {
        /// Admitted row and qualified record custody from the opposite side.
        other: Box<RecipientQualifiedOrganizationEntry>,
        /// Equal values do not imply equal source or proof ancestry.
        values: PortableWorkspaceValueComparison,
    },
}

/// One admitted correspondence result over an exact comparison pair.
pub struct RecipientQualifiedOrganizationCorrespondence {
    selected_side: OrganizationComparisonSide,
    selected: RecipientQualifiedOrganizationRecordHandle,
    state: RecipientQualifiedOrganizationCorrespondenceState,
    inspected: u16,
    copied_string_bytes: u32,
}

impl RecipientQualifiedOrganizationCorrespondence {
    /// Side that supplied the selected record.
    #[must_use]
    pub const fn selected_side(&self) -> OrganizationComparisonSide {
        self.selected_side
    }
    /// Exact selected record under recipient-qualified custody.
    #[must_use]
    pub const fn selected(&self) -> &RecipientQualifiedOrganizationRecordHandle {
        &self.selected
    }
    /// Incomplete search, complete absence, or one same-key match.
    #[must_use]
    pub const fn state(&self) -> &RecipientQualifiedOrganizationCorrespondenceState {
        &self.state
    }
    /// Other-side candidates examined by this bounded call.
    #[must_use]
    pub const fn inspected(&self) -> u16 {
        self.inspected
    }
    /// UTF-8 row bytes copied for a returned match.
    #[must_use]
    pub const fn copied_string_bytes(&self) -> u32 {
        self.copied_string_bytes
    }
}

impl OrganizationCorrespondence {
    /// Exact ordered pair and existing comparison receipt membership.
    #[must_use]
    pub fn belongs_to(&self, comparison: &PortableWorkspaceComparison) -> bool {
        self.inputs.matches(comparison)
    }
    /// Direction of selection; `other` always means the opposite side.
    #[must_use]
    pub const fn selected_side(&self) -> OrganizationComparisonSide {
        self.selected_side
    }
    /// Original source/detail handle, not re-created from displayed fields.
    #[must_use]
    pub const fn selected(&self) -> &OrganizationRecordHandle {
        &self.selected
    }
    /// Incomplete search, complete absence, or a same-key value comparison.
    #[must_use]
    pub const fn state(&self) -> &OrganizationCorrespondenceState {
        &self.state
    }
    /// Every candidate key examined, including nonmatches.
    #[must_use]
    pub const fn inspected(&self) -> u16 {
        self.inspected
    }
    /// Returned other-row UTF-8 payload; excludes retained owners and token metadata.
    #[must_use]
    pub const fn copied_string_bytes(&self) -> u32 {
        self.copied_string_bytes
    }
}

impl PortableWorkspaceComparison {
    /// Locate an exact selected organization record in the other compilation.
    ///
    /// The retained comparison already established the pair's existing proof
    /// and compatibility requirements. This result is relative to that pair;
    /// matching keys never permit substitution of source-owner handles.
    /// Uses application-owned canonical keys, not labels, source positions or
    /// shallow SDK row equality. Changed binding intervals are different keys;
    /// a stable explicit relation ID can have changed values. Source queries use
    /// the independently retained selected and other handles. No unmapped source
    /// is replaced with a guessed occurrence or proof.
    ///
    /// The visit limit is 1..=256; copied UTF-8 payload is independently bounded
    /// by [`MAX_ORGANIZATION_COPY_BYTES`]. At most one complete matching record
    /// is compared by its existing typed equality. Its nested work is bounded by
    /// admitted record limits, not the visit/output-copy budgets. This does not
    /// bound retained compilation memory, metadata allocation or elapsed time.
    /// This query grants no disclosure, rendering or native-operation authority.
    ///
    /// # Errors
    /// Rejects a different exact owner, pair, direction, selected record or raw
    /// limits before scanning. Copy failure publishes no successor; the caller's
    /// original handle and continuation remain usable.
    pub fn correspond_organization_record(
        &self,
        selected_side: OrganizationComparisonSide,
        selected: &OrganizationRecordHandle,
        after: Option<&OrganizationCorrespondenceCursor>,
        maximum_inspected: u16,
        maximum_copied_bytes: u32,
    ) -> Result<OrganizationCorrespondence, ApiError> {
        let execute = || -> Result<OrganizationCorrespondence, OrganizationFailure> {
            validate_limits(maximum_inspected, 1, maximum_copied_bytes)?;
            let (selected_owner, other_owner) = match selected_side {
                OrganizationComparisonSide::Prior => (&self.prior, &self.current),
                OrganizationComparisonSide::Current => (&self.current, &self.prior),
            };
            if !Arc::ptr_eq(selected_owner, &selected.owner)
                || after.is_some_and(|cursor| {
                    !cursor.inputs.matches(self)
                        || cursor.selected_side != selected_side
                        || cursor.maximum_inspected != maximum_inspected
                        || cursor.maximum_copied_bytes != maximum_copied_bytes
                })
            {
                return Err(OrganizationFailure::ContinuationMismatch);
            }
            let page = other_owner.correspond_organization_record(
                selected_owner,
                &selected.application,
                after.map(|cursor| &cursor.application),
                maximum_inspected,
            )?;
            let mut budget = CopyBudget::new(maximum_copied_bytes);
            let state = match page.state() {
                ApplicationState::Searching(cursor) => {
                    let continuation = OrganizationCorrespondenceCursor {
                        inputs: ComparisonInputs::retain(self),
                        selected_side,
                        application: cursor.clone(),
                        maximum_inspected,
                        maximum_copied_bytes,
                    };
                    OrganizationCorrespondenceState::Searching(continuation)
                }
                ApplicationState::Unmatched => OrganizationCorrespondenceState::Unmatched,
                ApplicationState::Matched { other, values } => {
                    let diagnostic_identity = Arc::new(self.diagnostic_identity.clone());
                    let other =
                        project_entry(other_owner, other, &diagnostic_identity, &mut budget)?;
                    let values = match values {
                        ApplicationValueComparison::Equal => {
                            PortableWorkspaceValueComparison::Equal
                        }
                        ApplicationValueComparison::Changed => {
                            PortableWorkspaceValueComparison::Changed
                        }
                    };
                    OrganizationCorrespondenceState::Matched {
                        other: Box::new(other),
                        values,
                    }
                }
            };
            Ok(OrganizationCorrespondence {
                inputs: ComparisonInputs::retain(self),
                selected_side,
                selected: selected.clone(),
                state,
                inspected: page.inspected(),
                copied_string_bytes: budget.used,
            })
        };
        execute().map_err(|error| failure(&self.diagnostic_identity, error))
    }
}

impl super::super::RecipientQualifiedWorkspaceComparisonInspection {
    /// Locate a selected organization record on the other comparison side.
    ///
    /// Every continuation receives fresh recipient and disclosure admission.
    ///
    /// # Errors
    /// Returns an admission, budget, custody, or projection failure.
    pub fn correspond_organization_record(
        &self,
        selected_side: OrganizationComparisonSide,
        selected: &RecipientQualifiedOrganizationRecordHandle,
        after: Option<&RecipientQualifiedOrganizationCorrespondenceCursor>,
        maximum_inspected: u16,
        maximum_copied_bytes: u32,
    ) -> Result<
        RecipientQualifiedOrganizationCorrespondence,
        super::super::RecipientQualifiedWorkspaceInspectionError,
    > {
        let application_side = match selected_side {
            OrganizationComparisonSide::Prior => {
                pale_application::workspace::WorkspaceInspectionComparisonSide::Prior
            }
            OrganizationComparisonSide::Current => {
                pale_application::workspace::WorkspaceInspectionComparisonSide::Current
            }
        };
        let permit = self.admitted().authorize_organization_correspondence(
            application_side,
            &selected.inner.application,
            after.is_some(),
            maximum_inspected,
        )?;
        debug_assert!(permit.matches(
            application_side,
            &selected.inner.application,
            after.is_some(),
            maximum_inspected,
        ));
        let result = self.raw_comparison().correspond_organization_record(
            selected_side,
            &selected.inner,
            after.map(|cursor| &cursor.inner),
            maximum_inspected,
            maximum_copied_bytes,
        )?;
        self.admitted()
            .budget()
            .charge_output(u64::from(result.copied_string_bytes()), 0)
            .map_err(pale_application::workspace::WorkspaceInspectionAccessError::from)?;
        let state = match result.state() {
            OrganizationCorrespondenceState::Searching(inner) => {
                RecipientQualifiedOrganizationCorrespondenceState::Searching(
                    RecipientQualifiedOrganizationCorrespondenceCursor {
                        inner: inner.clone(),
                    },
                )
            }
            OrganizationCorrespondenceState::Unmatched => {
                RecipientQualifiedOrganizationCorrespondenceState::Unmatched
            }
            OrganizationCorrespondenceState::Matched { other, values } => {
                RecipientQualifiedOrganizationCorrespondenceState::Matched {
                    other: Box::new(RecipientQualifiedOrganizationEntry {
                        row: other.row().clone(),
                        handle: RecipientQualifiedOrganizationRecordHandle {
                            inner: other.handle().clone(),
                        },
                    }),
                    values: *values,
                }
            }
        };
        Ok(RecipientQualifiedOrganizationCorrespondence {
            selected_side,
            selected: selected.clone(),
            state,
            inspected: result.inspected(),
            copied_string_bytes: result.copied_string_bytes(),
        })
    }
}
