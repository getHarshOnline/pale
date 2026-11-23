//! Native bindings for source-checked observation/profile association.

use super::{ObservationCursor, ObservationSnapshot, ObservationSubject};
use crate::{
    EffectiveProfileCompilation, EffectiveProfileCoordinates, EffectiveProfileEntry,
    PortableWorkspaceCompilation,
};
use napi_derive::napi;
use pale_sdk::{
    ObservationProfileAssociation as ApiAssociation, ObservationProfileError as ApiError,
    ObservationProfileMapping as ApiMapping,
};
use std::sync::Arc;

/// Why profile rows could not be read; existing snapshot facts remain usable.
#[napi(js_name = "ObservationProfileError")]
pub enum ObservationProfileError {
    /// This snapshot was not produced from an effective profile.
    NoProfileSource,
    /// The original admitted source is no longer retained.
    ExpiredSource,
    /// A supplied compilation has another admitted source.
    SourceMismatch,
    /// The profile, private instance or composition changed.
    CoordinateMismatch,
    /// The token belongs to another query or limit pair.
    ContinuationMismatch,
    /// Both row limits must be between 1 and 256.
    InvalidLimit,
}

/// Whether this subject category has primary profile rows.
#[napi(js_name = "ObservationProfileMapping")]
pub enum ObservationProfileMapping {
    /// This category supports association with primary rows.
    PrimaryRows,
    /// This category has no primary-row mapping.
    NoPrimaryRows,
}

/// Whether the query returned a page or a typed refusal.
#[napi(js_name = "ObservationProfileResultKind")]
pub enum ObservationProfileResultKind {
    /// A bounded page is available.
    Ready,
    /// The query did not qualify; inspect its typed error.
    Refused,
}

/// Whether a profile association qualified or was refused.
#[napi(js_name = "ObservationProfileAssociationResultKind")]
pub enum ObservationProfileAssociationResultKind {
    /// The exact snapshot and profile were associated.
    Associated,
    /// The association did not qualify; inspect its typed error.
    Refused,
}

/// Association with one exact observation snapshot.
#[napi(js_name = "ObservationProfileAssociation")]
pub struct ObservationProfileAssociation {
    inner: ApiAssociation,
}

#[napi]
impl ObservationProfileAssociation {
    /// Return all coordinates of the associated profile.
    #[napi(getter)]
    pub fn coordinates(&self) -> EffectiveProfileCoordinates {
        EffectiveProfileCoordinates::from_api(self.inner.coordinates().clone())
    }

    /// Return the cursor of the exact retained snapshot.
    #[napi(getter)]
    pub fn cursor(&self) -> ObservationCursor {
        ObservationCursor {
            inner: self.inner.cursor(),
        }
    }

    /// Test exact snapshot custody. Equal cursors are insufficient.
    #[napi]
    pub fn belongs_to(&self, snapshot: &ObservationSnapshot) -> bool {
        self.inner.belongs_to(&snapshot.inner)
    }
}

/// A profile association or a typed refusal.
#[napi(js_name = "ObservationProfileAssociationResult")]
pub struct ObservationProfileAssociationResult {
    pub(crate) inner: Result<ApiAssociation, ApiError>,
}

#[napi]
impl ObservationProfileAssociationResult {
    /// Select the associated or refused branch.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationProfileAssociationResultKind {
        match &self.inner {
            Ok(_) => ObservationProfileAssociationResultKind::Associated,
            Err(_) => ObservationProfileAssociationResultKind::Refused,
        }
    }

    /// Return the association only when the check qualified.
    #[napi(getter)]
    pub fn association(&self) -> Option<ObservationProfileAssociation> {
        self.inner
            .as_ref()
            .ok()
            .cloned()
            .map(|inner| ObservationProfileAssociation { inner })
    }

    /// Return the refusal without converting it to text or an exception.
    #[napi(getter)]
    pub fn error(&self) -> Option<ObservationProfileError> {
        self.inner.as_ref().err().copied().map(project_error)
    }
}

/// Continue one exact query without retaining its source compilations.
#[napi(js_name = "ObservationProfileContinuation")]
pub struct ObservationProfileContinuation {
    inner: pale_sdk::ObservationProfileContinuation,
}

/// Original profile rows and the progress of this bounded query.
#[napi(js_name = "ObservationProfilePage")]
pub struct ObservationProfilePage {
    inner: Arc<pale_sdk::ObservationProfilePage>,
}

#[napi]
impl ObservationProfilePage {
    /// Identify the producing profile; this does not establish native readiness.
    #[napi(getter)]
    pub fn coordinates(&self) -> EffectiveProfileCoordinates {
        EffectiveProfileCoordinates::from_api(self.inner.coordinates.clone())
    }

    /// Distinguish unsupported categories from an empty intermediate page.
    #[napi(getter)]
    pub fn mapping(&self) -> ObservationProfileMapping {
        match self.inner.mapping {
            ApiMapping::PrimaryRows => ObservationProfileMapping::PrimaryRows,
            ApiMapping::NoPrimaryRows => ObservationProfileMapping::NoPrimaryRows,
        }
    }

    /// Return original row payloads with their existing explanation handles.
    #[napi(getter)]
    pub fn entries(&self) -> Vec<EffectiveProfileEntry> {
        self.inner
            .entries
            .iter()
            .map(EffectiveProfileEntry::from_api)
            .collect()
    }

    /// Count all examined rows, including rows that did not match.
    #[napi(getter)]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected
    }

    /// Continue if rows remain. An empty page can still have a continuation.
    #[napi(getter)]
    pub fn continuation(&self) -> Option<ObservationProfileContinuation> {
        self.inner
            .continuation
            .clone()
            .map(|inner| ObservationProfileContinuation { inner })
    }
}

/// A completed query or contextual refusal, not an operation-failure diagnostic.
#[napi(js_name = "ObservationProfileResult")]
pub struct ObservationProfileResult {
    inner: Result<Arc<pale_sdk::ObservationProfilePage>, ApiError>,
}

#[napi]
impl ObservationProfileResult {
    /// Select the page or refusal branch.
    #[napi(getter)]
    pub fn kind(&self) -> ObservationProfileResultKind {
        match &self.inner {
            Ok(_) => ObservationProfileResultKind::Ready,
            Err(_) => ObservationProfileResultKind::Refused,
        }
    }

    /// Return the page only when this query qualified.
    #[napi(getter)]
    pub fn page(&self) -> Option<ObservationProfilePage> {
        self.inner
            .as_ref()
            .ok()
            .cloned()
            .map(|inner| ObservationProfilePage { inner })
    }

    /// Return the refusal without converting it to a message or exception.
    #[napi(getter)]
    pub fn error(&self) -> Option<ObservationProfileError> {
        self.inner.as_ref().err().copied().map(project_error)
    }
}

fn project_error(error: ApiError) -> ObservationProfileError {
    match error {
        ApiError::NoProfileSource => ObservationProfileError::NoProfileSource,
        ApiError::ExpiredSource => ObservationProfileError::ExpiredSource,
        ApiError::SourceMismatch => ObservationProfileError::SourceMismatch,
        ApiError::CoordinateMismatch => ObservationProfileError::CoordinateMismatch,
        ApiError::ContinuationMismatch => ObservationProfileError::ContinuationMismatch,
        ApiError::InvalidLimit => ObservationProfileError::InvalidLimit,
    }
}

#[napi]
impl ObservationSnapshot {
    /// Associate this exact snapshot with an effective profile.
    ///
    /// The returned handle retains identity only. It copies no profile rows,
    /// observation facts, or causal paths.
    #[napi]
    pub fn associate_profile(
        &self,
        profile: &EffectiveProfileCompilation,
        workspace: &PortableWorkspaceCompilation,
    ) -> ObservationProfileAssociationResult {
        ObservationProfileAssociationResult {
            inner: self
                .inner
                .associate_profile(&profile.inner, &workspace.inner),
        }
    }

    /// Read profile rows for a subject from this snapshot's admitted source.
    ///
    /// Reuse retained profile and workspace handles; do not recompile between
    /// pages. Both limits are 1 to 256. They bound examined and returned rows,
    /// not total bytes or caller-retained memory. No source is parsed or written.
    #[napi]
    pub fn inspect_profile(
        &self,
        profile: &EffectiveProfileCompilation,
        workspace: &PortableWorkspaceCompilation,
        subject: &ObservationSubject,
        maximum_inspected: f64,
        maximum_returned: f64,
        after: Option<&ObservationProfileContinuation>,
    ) -> ObservationProfileResult {
        let limits = (
            crate::authoring::authoring_page_limit(maximum_inspected),
            crate::authoring::authoring_page_limit(maximum_returned),
        );
        let (Ok(maximum_inspected), Ok(maximum_returned)) = limits else {
            return ObservationProfileResult {
                inner: Err(ApiError::InvalidLimit),
            };
        };
        ObservationProfileResult {
            inner: self
                .inner
                .inspect_profile(
                    &profile.inner,
                    &workspace.inner,
                    &subject.inner,
                    after.map(|value| &value.inner),
                    maximum_inspected,
                    maximum_returned,
                )
                .map(Arc::new),
        }
    }
}
