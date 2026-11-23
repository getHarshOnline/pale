//! Associate configured observation subjects with existing profile rows.

use std::sync::Arc;

use pale_core::records::{ExperienceCompositionPlanDigest, InstanceDigest, ProfileDigest};

use crate::{compile::EffectiveProfileCompilation, workspace::PortableWorkspaceUniverse};

use super::{
    ObservationGeneration, ObservationSnapshot, ObservationSubject, MAX_OBSERVATION_PAGE_LIMIT,
};

/// Why a configured observation cannot be joined to a supplied profile.
///
/// These query refusals do not invalidate retained facts or report an operation
/// failure. They carry no display-message policy or replacement source identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationProfileError {
    /// This snapshot came from a resident, not an effective profile.
    NoProfileSource,
    /// No retained compilation still owns the original admitted source.
    ExpiredSource,
    /// The supplied compilation does not share the original admitted source.
    SourceMismatch,
    /// The source matches, but the profile, private instance or composition differs.
    CoordinateMismatch,
    /// Both query limits must be within the supported page range.
    InvalidLimit,
}

/// Whether this subject category has primary profile rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationProfileMapping {
    /// The key may select rows; an empty result does not establish semantic absence.
    PrimaryRows,
    /// Whole-profile and resident subjects have no primary-row mapping.
    NoPrimaryRows,
}

/// One source-checked subject selector over application-issued row provenance.
///
/// The query borrows the supplied profile for the call; it does not retain it.
/// Presentation sections and row bodies remain outside this application owner.
pub struct ObservationProfileQuery<'a> {
    subject: &'a ObservationSubject,
    maximum_inspected: u16,
    maximum_returned: u16,
    _profile: &'a EffectiveProfileCompilation,
}

/// Positions within the supplied continuation window, not semantic identities.
#[derive(Debug, PartialEq, Eq)]
pub struct ObservationProfileRows {
    /// Category support is separate from whether a selected key matched a row.
    pub mapping: ObservationProfileMapping,
    /// Original zero-based positions to project without recreating their handles.
    pub selected: Vec<u16>,
    /// Raw rows visited, including absent provenance and nonmatching subjects.
    pub inspected: u16,
}

impl ObservationSnapshot {
    pub(super) fn profile_source_owner(
        &self,
    ) -> Result<Arc<crate::workspace::PrincipalAdmittedUniverse>, ObservationProfileError> {
        self.profile_origin
            .as_ref()
            .ok_or(ObservationProfileError::NoProfileSource)?
            .upgrade()
            .ok_or(ObservationProfileError::ExpiredSource)
    }

    /// Check whether this snapshot and profile share one admitted source and generation.
    ///
    /// Equal source bytes from a separate admission do not qualify. This check
    /// reads fixed identity fields only. It copies no facts, rows, or causal paths.
    ///
    /// # Errors
    /// Returns [`ObservationProfileError`] when the snapshot has no profile
    /// source, its source owner expired, or either source or generation differs.
    pub fn check_profile_association(
        &self,
        profile: &EffectiveProfileCompilation,
        workspace: &PortableWorkspaceUniverse,
    ) -> Result<(), ObservationProfileError> {
        let origin = self.profile_source_owner()?;
        if !Arc::ptr_eq(&origin, profile.admitted_source_owner())
            || !workspace.shares_admitted_owner(&origin)
        {
            return Err(ObservationProfileError::SourceMismatch);
        }
        if profile_coordinates_match(
            self.cursor().generation(),
            &profile.profile().profile_digest,
            &profile.instance().instance_digest,
            &profile.experience_composition_plan().plan_digest,
        ) {
            Ok(())
        } else {
            Err(ObservationProfileError::CoordinateMismatch)
        }
    }

    /// Check the source owner and exact profile coordinates before association.
    ///
    /// Equal bytes from a separately admitted workspace do not establish custody.
    /// Another profile allocation from the same retained owner may pass when all
    /// three profile coordinates match. Expired origin does not invalidate facts
    /// already retained in this snapshot. No native evidence or permission is added.
    ///
    /// # Errors
    /// Returns [`ObservationProfileError`] for unavailable or different source,
    /// different profile coordinates, or limits outside `1..=256`.
    pub fn profile_query<'a>(
        &self,
        profile: &'a EffectiveProfileCompilation,
        workspace: &PortableWorkspaceUniverse,
        subject: &'a ObservationSubject,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<ObservationProfileQuery<'a>, ObservationProfileError> {
        if !(1..=MAX_OBSERVATION_PAGE_LIMIT).contains(&maximum_inspected)
            || !(1..=MAX_OBSERVATION_PAGE_LIMIT).contains(&maximum_returned)
        {
            return Err(ObservationProfileError::InvalidLimit);
        }
        self.check_profile_association(profile, workspace)?;
        Ok(ObservationProfileQuery {
            subject,
            maximum_inspected,
            maximum_returned,
            _profile: profile,
        })
    }
}

fn profile_coordinates_match(
    generation: &ObservationGeneration,
    profile_digest: &ProfileDigest,
    instance_digest: &InstanceDigest,
    composition_plan_digest: &ExperienceCompositionPlanDigest,
) -> bool {
    matches!(
        generation,
        ObservationGeneration::EffectiveProfile {
            profile_digest: observed_profile,
            instance_digest: observed_instance,
            composition_plan_digest: observed_composition,
        } if observed_profile == profile_digest
            && observed_instance == instance_digest
            && observed_composition == composition_plan_digest
    )
}

impl ObservationProfileQuery<'_> {
    /// Select original rows from one continuation window of retained provenance.
    ///
    /// The caller supplies provenance issued during the same profile projection,
    /// starting directly at its retained cursor. Every consumed row costs one
    /// visit; the scan does not peek beyond either budget. Returned positions let
    /// the caller reuse existing row bodies and explanation handles. Row counts
    /// do not bound their nested payload bytes or backend allocation.
    #[must_use]
    pub fn select_rows<'a>(
        &self,
        rows: impl IntoIterator<Item = Option<&'a ObservationSubject>>,
    ) -> ObservationProfileRows {
        select_rows(
            self.subject,
            rows,
            self.maximum_inspected,
            self.maximum_returned,
        )
    }
}

fn select_rows<'a>(
    subject: &ObservationSubject,
    rows: impl IntoIterator<Item = Option<&'a ObservationSubject>>,
    maximum_inspected: u16,
    maximum_returned: u16,
) -> ObservationProfileRows {
    let mapping = match subject {
        ObservationSubject::Machine(_)
        | ObservationSubject::Repository(_)
        | ObservationSubject::Operation { .. }
        | ObservationSubject::CapabilityRequirement { .. }
        | ObservationSubject::ExecutionPath(_) => ObservationProfileMapping::PrimaryRows,
        ObservationSubject::EffectiveProfile(_)
        | ObservationSubject::CompiledGeneration(_)
        | ObservationSubject::ResidentProcess => ObservationProfileMapping::NoPrimaryRows,
    };
    let mut result = ObservationProfileRows {
        mapping,
        selected: Vec::new(),
        inspected: 0,
    };
    for row in rows.into_iter().take(usize::from(maximum_inspected)) {
        let position = result.inspected;
        result.inspected += 1;
        if mapping == ObservationProfileMapping::PrimaryRows && row == Some(subject) {
            result.selected.push(position);
            if result.selected.len() == usize::from(maximum_returned) {
                break;
            }
        }
    }
    result
}

#[cfg(test)]
#[path = "../../tests/internal/observation_profile.rs"]
mod tests;
