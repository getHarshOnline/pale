//! Repository-bound native-action catalog admission through the Rust SDK.

use std::sync::Arc;

use pale_sdk::RepositoryNativeActionCatalogRequest;
use pyo3::prelude::*;

use crate::error::diagnostic_error;
use crate::{PalePython, PyNativeActionDescriptor};

/// Repository-bound admitted action selection.
#[pyclass(name = "AdmittedRepositoryActions", frozen)]
pub struct PyAdmittedRepositoryActions {
    profile_digest: String,
    instance_digest: String,
    repository_id: String,
    descriptors: Vec<PyNativeActionDescriptor>,
}

#[pymethods]
impl PyAdmittedRepositoryActions {
    /// Exact profile digest carried by the admitted catalog.
    #[getter]
    pub fn profile_digest(&self) -> &str {
        &self.profile_digest
    }

    /// Exact instance digest carried by the admitted catalog.
    #[getter]
    pub fn instance_digest(&self) -> &str {
        &self.instance_digest
    }

    /// Exact repository identity carried by the admitted catalog.
    #[getter]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Clone the stable descriptor batch without re-admitting it.
    pub fn descriptors(&self) -> Vec<PyNativeActionDescriptor> {
        self.descriptors
            .iter()
            .map(|descriptor| PyNativeActionDescriptor::from_api(descriptor.clone_api()))
            .collect()
    }
}

#[pymethods]
impl PalePython {
    /// Admit one repository-bound pure-action catalog.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when catalog or context
    /// admission fails; the complete record is attached as `diagnostic`.
    pub fn admit_repository_native_actions(
        &self,
        py: Python<'_>,
        bytes: Vec<u8>,
        profile_digest: String,
        instance_digest: String,
        repository_id: String,
    ) -> PyResult<PyAdmittedRepositoryActions> {
        let api = Arc::clone(&self.api);
        py.detach(move || {
            api.admit_repository_native_actions(RepositoryNativeActionCatalogRequest::new(
                bytes,
                profile_digest,
                instance_digest,
                repository_id,
            ))
        })
        .map(|admitted| PyAdmittedRepositoryActions {
            profile_digest: admitted.profile_digest().to_owned(),
            instance_digest: admitted.instance_digest().to_owned(),
            repository_id: admitted.repository_id().to_owned(),
            descriptors: admitted
                .into_descriptors()
                .into_iter()
                .map(PyNativeActionDescriptor::from_api)
                .collect(),
        })
        .map_err(|error| diagnostic_error(py, error))
    }
}
