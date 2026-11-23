// Python native-action projection fragment included by the shallow façade.

/// One API-owned native-action descriptor.
#[pyclass(name = "NativeActionDescriptor", frozen)]
pub struct PyNativeActionDescriptor {
    inner: NativeActionDescriptor,
}

impl PyNativeActionDescriptor {
    /// Retain one registry-owned descriptor.
    pub(crate) fn from_api(inner: NativeActionDescriptor) -> Self {
        Self { inner }
    }

    /// Clone the immutable API descriptor for a repeated host batch.
    pub(crate) fn clone_api(&self) -> NativeActionDescriptor {
        self.inner.clone()
    }
}

#[pymethods]
impl PyNativeActionDescriptor {
    /// Exact compiled action identity.
    #[getter]
    pub fn action_id(&self) -> &str {
        self.inner.action_id()
    }

    /// Registry-owned concise description.
    #[getter]
    pub fn description(&self) -> &str {
        self.inner.description()
    }

    /// Exact canonical action contract bytes.
    #[getter]
    pub fn contract_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, self.inner.canonical_contract_bytes())
    }

    /// Self-contained payload schema bytes.
    #[getter]
    pub fn payload_schema_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, self.inner.canonical_payload_schema_bytes())
    }
}
