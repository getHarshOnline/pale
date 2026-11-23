//! Source-checked association with existing effective-profile rows.

use std::sync::Arc;

use super::{application, ObservationSnapshot, ObservationSubject};
use crate::{
    EffectiveProfileCompilation, EffectiveProfileCoordinates, EffectiveProfileEntry,
    PortableWorkspaceCompilation,
};

/// Why a profile association could not be read. Existing snapshot facts remain usable.
///
/// This projects typed query refusals, not operation-failure diagnostics or
/// display messages. No replacement source is implied when the origin expires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationProfileError {
    /// No effective-profile source produced this snapshot.
    NoProfileSource,
    /// The original admitted source no longer has a retained compilation owner.
    ExpiredSource,
    /// A supplied compilation has a different admitted source owner.
    SourceMismatch,
    /// Profile, private instance or composition differs from the snapshot.
    CoordinateMismatch,
    /// The token belongs to another snapshot, subject or limit pair.
    ContinuationMismatch,
    /// Both limits must be in `1..=256`.
    InvalidLimit,
}

impl From<application::ObservationProfileError> for ObservationProfileError {
    fn from(error: application::ObservationProfileError) -> Self {
        match error {
            application::ObservationProfileError::NoProfileSource => Self::NoProfileSource,
            application::ObservationProfileError::ExpiredSource => Self::ExpiredSource,
            application::ObservationProfileError::SourceMismatch => Self::SourceMismatch,
            application::ObservationProfileError::CoordinateMismatch => Self::CoordinateMismatch,
            application::ObservationProfileError::InvalidLimit => Self::InvalidLimit,
        }
    }
}

/// Category support, not a claim that the selected subject exists or was observed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationProfileMapping {
    /// Primary rows can be associated with this category.
    PrimaryRows,
    /// Whole-profile and resident subjects have no primary-row mapping.
    NoPrimaryRows,
}

/// Association between one exact snapshot and effective profile.
///
/// This opaque handle retains the snapshot and the profile coordinates proved
/// against it. It contains no profile rows, observation facts, or causal paths.
#[derive(Clone)]
pub struct ObservationProfileAssociation {
    snapshot: ObservationSnapshot,
    coordinates: EffectiveProfileCoordinates,
}

impl std::fmt::Debug for ObservationProfileAssociation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ObservationProfileAssociation")
            .field("coordinates", &self.coordinates)
            .field("cursor", &self.snapshot.cursor())
            .finish_non_exhaustive()
    }
}

impl ObservationProfileAssociation {
    /// Return all coordinates of the associated profile.
    #[must_use]
    pub const fn coordinates(&self) -> &EffectiveProfileCoordinates {
        &self.coordinates
    }

    /// Return the cursor of the exact retained snapshot.
    #[must_use]
    pub fn cursor(&self) -> super::ObservationCursor {
        self.snapshot.cursor()
    }

    /// Test exact snapshot custody. Equal cursors are insufficient.
    #[must_use]
    pub fn belongs_to(&self, snapshot: &ObservationSnapshot) -> bool {
        Arc::ptr_eq(&self.snapshot.application, &snapshot.application)
    }
}

/// Position in one exact snapshot/subject query; retains no complete profile.
#[derive(Clone)]
pub struct ObservationProfileContinuation {
    snapshot: ObservationSnapshot,
    subject: application::ObservationSubject,
    position: (usize, usize),
    maximum_inspected: u16,
    maximum_returned: u16,
}

impl std::fmt::Debug for ObservationProfileContinuation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ObservationProfileContinuation")
            .finish_non_exhaustive()
    }
}

/// Original retained profile rows selected by the application association owner.
#[derive(Clone, Debug)]
pub struct ObservationProfilePage {
    /// Producing profile coordinates, not native or recipient authority.
    pub coordinates: EffectiveProfileCoordinates,
    /// Category support stays separate from an empty or incomplete page.
    pub mapping: ObservationProfileMapping,
    /// Existing row bodies and their unchanged explanation handles.
    pub entries: Vec<EffectiveProfileEntry>,
    /// Raw provenance rows examined, including unmapped and nonmatching rows.
    pub inspected: u16,
    /// Continue when rows remain; an empty intermediate page is not absence.
    pub continuation: Option<ObservationProfileContinuation>,
}

impl ObservationSnapshot {
    /// Associate this snapshot with a profile through an admitted inspection owner.
    ///
    /// Recipient authority is checked before source or coordinate comparison. A
    /// denial therefore reveals neither source membership nor profile freshness.
    ///
    /// # Errors
    /// Returns an authority or aggregate-budget failure. Profile association
    /// refusals remain the inner typed result.
    pub fn associate_recipient_qualified_profile(
        &self,
        profile: &EffectiveProfileCompilation,
        inspection: &crate::RecipientQualifiedWorkspaceInspection,
    ) -> Result<
        Result<ObservationProfileAssociation, ObservationProfileError>,
        crate::RecipientQualifiedWorkspaceAccessError,
    > {
        inspection.authorize_profile_observation_association()?;
        Ok(self.associate_profile(profile, inspection.compilation()))
    }

    /// Associate this exact snapshot with an effective profile.
    ///
    /// The profile and workspace must share the snapshot's admitted source and
    /// the snapshot must carry the same profile, private instance, and
    /// composition coordinates. The returned handle copies no rows or facts.
    ///
    /// # Errors
    /// Returns [`ObservationProfileError`] when the source is unavailable or
    /// differs, or when the profile generation differs.
    pub fn associate_profile(
        &self,
        profile: &EffectiveProfileCompilation,
        workspace: &PortableWorkspaceCompilation,
    ) -> Result<ObservationProfileAssociation, ObservationProfileError> {
        self.application
            .check_profile_association(&profile.compilation, workspace.application_universe())?;
        Ok(ObservationProfileAssociation {
            snapshot: self.clone(),
            coordinates: profile.coordinates(),
        })
    }

    /// Associate one configured subject with its retained profile rows.
    ///
    /// Supply the profile and portable workspace explicitly. Both must share
    /// the snapshot's original admitted source; a separately admitted equal-byte
    /// workspace does not qualify. Another profile allocation from that source
    /// may qualify when profile, instance and composition coordinates match.
    /// This snapshot and its continuation do not keep the profile alive.
    ///
    /// Both limits are in `1..=256`. Each call examines at most
    /// `maximum_inspected` raw rows and copies at most `maximum_returned` rows.
    /// Nested payloads, provenance keys and caller-retained pages have additional
    /// costs; these limits do not establish aggregate memory or byte bounds.
    /// No source is parsed, state is activated or native evidence inferred.
    ///
    /// # Errors
    /// Returns [`ObservationProfileError`] for expired/different source, different
    /// coordinates, invalid limits, or a foreign/differently parameterized token.
    pub fn inspect_profile(
        &self,
        profile: &EffectiveProfileCompilation,
        workspace: &PortableWorkspaceCompilation,
        subject: &ObservationSubject,
        after: Option<&ObservationProfileContinuation>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<ObservationProfilePage, ObservationProfileError> {
        let query = self.application.profile_query(
            &profile.compilation,
            workspace.application_universe(),
            &subject.application,
            maximum_inspected,
            maximum_returned,
        )?;
        if after.is_some_and(|cursor| {
            !Arc::ptr_eq(&cursor.snapshot.application, &self.application)
                || cursor.subject != subject.application
                || cursor.maximum_inspected != maximum_inspected
                || cursor.maximum_returned != maximum_returned
        }) {
            return Err(ObservationProfileError::ContinuationMismatch);
        }
        let (entries, next, inspected, mapping) =
            profile.observation_page(&query, after.map_or((0, 0), |cursor| cursor.position));
        Ok(ObservationProfilePage {
            coordinates: profile.coordinates(),
            entries,
            inspected,
            mapping: match mapping {
                application::ObservationProfileMapping::PrimaryRows => {
                    ObservationProfileMapping::PrimaryRows
                }
                application::ObservationProfileMapping::NoPrimaryRows => {
                    ObservationProfileMapping::NoPrimaryRows
                }
            },
            continuation: next.map(|position| ObservationProfileContinuation {
                snapshot: self.clone(),
                subject: subject.application.clone(),
                position,
                maximum_inspected,
                maximum_returned,
            }),
        })
    }
}
