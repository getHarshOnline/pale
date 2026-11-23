//! Native bindings for source-checked observation/profile association.

use super::{PyObservationSnapshot, PyObservationSubject};
use crate::{
    PyEffectiveProfileCompilation, PyEffectiveProfileCoordinates, PyEffectiveProfileEntry,
    PyPortableWorkspaceCompilation,
};
use pale_sdk::{ObservationProfileError as ApiError, ObservationProfileMapping as ApiMapping};
use pyo3::prelude::*;
use std::sync::Arc;

/// Why profile rows could not be read; existing snapshot facts remain usable.
#[pyclass(
    name = "ObservationProfileError",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationProfileError {
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
#[pyclass(
    name = "ObservationProfileMapping",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationProfileMapping {
    /// This category supports association with primary rows.
    PrimaryRows,
    /// This category has no primary-row mapping.
    NoPrimaryRows,
}

/// Whether the query returned a page or a typed refusal.
#[pyclass(
    name = "ObservationProfileResultKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyObservationProfileResultKind {
    /// A bounded page is available.
    Ready,
    /// The query did not qualify; inspect its typed error.
    Refused,
}

/// Continue one exact query without retaining its source compilations.
#[pyclass(name = "ObservationProfileContinuation", frozen)]
pub struct PyObservationProfileContinuation {
    inner: pale_sdk::ObservationProfileContinuation,
}

/// Original profile rows and the progress of this bounded query.
#[pyclass(name = "ObservationProfilePage", frozen)]
pub struct PyObservationProfilePage {
    inner: Arc<pale_sdk::ObservationProfilePage>,
}

#[pymethods]
impl PyObservationProfilePage {
    /// Identify the producing profile; this does not establish native readiness.
    #[getter]
    pub fn coordinates(&self) -> PyEffectiveProfileCoordinates {
        PyEffectiveProfileCoordinates::from_api(self.inner.coordinates.clone())
    }

    /// Distinguish unsupported categories from an empty intermediate page.
    #[getter]
    pub fn mapping(&self) -> PyObservationProfileMapping {
        match self.inner.mapping {
            ApiMapping::PrimaryRows => PyObservationProfileMapping::PrimaryRows,
            ApiMapping::NoPrimaryRows => PyObservationProfileMapping::NoPrimaryRows,
        }
    }

    /// Return original row payloads with their existing explanation handles.
    #[getter]
    pub fn entries(&self) -> Vec<PyEffectiveProfileEntry> {
        self.inner
            .entries
            .iter()
            .cloned()
            .map(PyEffectiveProfileEntry::from_api)
            .collect()
    }

    /// Count all examined rows, including rows that did not match.
    #[getter]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected
    }

    /// Continue if rows remain. An empty page can still have a continuation.
    #[getter]
    pub fn continuation(&self) -> Option<PyObservationProfileContinuation> {
        self.inner
            .continuation
            .clone()
            .map(|inner| PyObservationProfileContinuation { inner })
    }
}

/// A completed query or contextual refusal, not an operation-failure diagnostic.
#[pyclass(name = "ObservationProfileResult", frozen)]
pub struct PyObservationProfileResult {
    inner: Result<Arc<pale_sdk::ObservationProfilePage>, ApiError>,
}

#[pymethods]
impl PyObservationProfileResult {
    /// Select the page or refusal branch.
    #[getter]
    pub fn kind(&self) -> PyObservationProfileResultKind {
        match &self.inner {
            Ok(_) => PyObservationProfileResultKind::Ready,
            Err(_) => PyObservationProfileResultKind::Refused,
        }
    }

    /// Return the page only when this query qualified.
    #[getter]
    pub fn page(&self) -> Option<PyObservationProfilePage> {
        self.inner
            .as_ref()
            .ok()
            .cloned()
            .map(|inner| PyObservationProfilePage { inner })
    }

    /// Return the refusal without converting it to a message or exception.
    #[getter]
    pub fn error(&self) -> Option<PyObservationProfileError> {
        self.inner.as_ref().err().map(|error| match error {
            ApiError::NoProfileSource => PyObservationProfileError::NoProfileSource,
            ApiError::ExpiredSource => PyObservationProfileError::ExpiredSource,
            ApiError::SourceMismatch => PyObservationProfileError::SourceMismatch,
            ApiError::CoordinateMismatch => PyObservationProfileError::CoordinateMismatch,
            ApiError::ContinuationMismatch => PyObservationProfileError::ContinuationMismatch,
            ApiError::InvalidLimit => PyObservationProfileError::InvalidLimit,
        })
    }
}

#[pymethods]
impl PyObservationSnapshot {
    /// Read profile rows for a subject from this snapshot's admitted source.
    ///
    /// Reuse retained profile and workspace handles; do not recompile between
    /// pages. Both limits are 1 to 256. They bound examined and returned rows,
    /// not total bytes or caller-retained memory. No source is parsed or written.
    #[pyo3(signature = (profile, workspace, subject, maximum_inspected, maximum_returned, after=None))]
    // The extra interpreter argument permits releasing Python during the query.
    #[allow(clippy::too_many_arguments)]
    pub fn inspect_profile(
        &self,
        py: Python<'_>,
        profile: &PyEffectiveProfileCompilation,
        workspace: &PyPortableWorkspaceCompilation,
        subject: &PyObservationSubject,
        maximum_inspected: u16,
        maximum_returned: u16,
        after: Option<&PyObservationProfileContinuation>,
    ) -> PyObservationProfileResult {
        let snapshot = Arc::clone(&self.inner);
        let profile = Arc::clone(&profile.inner);
        let workspace = Arc::clone(&workspace.inner);
        let subject = subject.inner.clone();
        let after = after.map(|value| value.inner.clone());
        py.detach(move || PyObservationProfileResult {
            inner: snapshot
                .inspect_profile(
                    &profile,
                    &workspace,
                    &subject,
                    after.as_ref(),
                    maximum_inspected,
                    maximum_returned,
                )
                .map(Arc::new),
        })
    }
}
