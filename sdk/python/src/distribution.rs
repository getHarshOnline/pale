//! Python projection of Package distribution inspection.

use std::sync::Arc;

use pale_sdk::{PackageDistributionInspection, PackageDistributionRequest};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::error::diagnostic_error;
use crate::{PalePython, PyCompilationDocument};

/// Exact immutable identity of one inspected Package release.
#[pyclass(name = "PackageReleaseIdentity", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct PyPackageReleaseIdentity {
    id: String,
    version: String,
    content_digest: String,
}

#[pymethods]
impl PyPackageReleaseIdentity {
    /// Canonical Package identity.
    #[getter]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Exact owner-published release label.
    #[getter]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Digest of the complete manifest and owned source inventory.
    #[getter]
    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }
}

/// Canonically ordered Package lock inputs derived by the API owner.
#[pyclass(name = "PackageDistributionInspection", frozen)]
pub struct PyPackageDistributionInspection {
    compiler: String,
    schema_manifest_sha256: String,
    schema_set_sha256: String,
    packages: Vec<PyPackageReleaseIdentity>,
    canonical_bytes: Vec<u8>,
}

impl PyPackageDistributionInspection {
    /// Project API-owned facts without parsing or re-canonicalizing them.
    pub(crate) fn from_api(value: PackageDistributionInspection) -> Self {
        Self {
            compiler: value.compiler().to_owned(),
            schema_manifest_sha256: value.schema_manifest_sha256().to_owned(),
            schema_set_sha256: value.schema_set_sha256().to_owned(),
            packages: value
                .packages()
                .iter()
                .map(|release| PyPackageReleaseIdentity {
                    id: release.id().to_owned(),
                    version: release.version().to_owned(),
                    content_digest: release.content_digest().to_owned(),
                })
                .collect(),
            canonical_bytes: value.canonical_bytes().to_vec(),
        }
    }
}

#[pymethods]
impl PyPackageDistributionInspection {
    /// Compiler contract required by the inspected releases.
    #[getter]
    pub fn compiler(&self) -> &str {
        &self.compiler
    }

    /// Digest of the generated schema manifest used for admission.
    #[getter]
    pub fn schema_manifest_sha256(&self) -> &str {
        &self.schema_manifest_sha256
    }

    /// Digest of the complete generated schema byte set used for admission.
    #[getter]
    pub fn schema_set_sha256(&self) -> &str {
        &self.schema_set_sha256
    }

    /// Clone the canonically ordered Package release identities.
    #[getter]
    pub fn packages(&self) -> Vec<PyPackageReleaseIdentity> {
        self.packages.clone()
    }

    /// Copy the canonical API-owned bytes for durable transport.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, &self.canonical_bytes)
    }
}

#[pymethods]
impl PalePython {
    /// Inspect captured Package release coordinates without reading a lock.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when logical labels, finite
    /// source bounds, manifests, or source inventories fail API admission.
    pub fn inspect_package_distribution(
        &self,
        py: Python<'_>,
        documents: Vec<PyCompilationDocument>,
    ) -> PyResult<PyPackageDistributionInspection> {
        let api = Arc::clone(&self.api);
        let request = PackageDistributionRequest::new(
            documents
                .into_iter()
                .map(PyCompilationDocument::into_api)
                .collect(),
        );
        py.detach(move || api.inspect_package_distribution(request))
            .map(PyPackageDistributionInspection::from_api)
            .map_err(|error| diagnostic_error(py, error))
    }
}
