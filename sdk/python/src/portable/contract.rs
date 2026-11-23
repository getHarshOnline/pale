// Python contract-identity projection fragment included by the shallow façade.

/// Exact embedded bundle coordinate.
#[pyclass(name = "ContractBundleIdentity", frozen)]
pub struct PyContractBundleIdentity {
    schema_manifest_sha256: String,
    schema_set_sha256: String,
    member_count: u32,
}

#[pymethods]
impl PyContractBundleIdentity {
    /// RFC 8785 descriptor-manifest digest.
    #[getter]
    pub fn schema_manifest_sha256(&self) -> &str {
        &self.schema_manifest_sha256
    }

    /// Domain-separated active schema-set digest.
    #[getter]
    pub fn schema_set_sha256(&self) -> &str {
        &self.schema_set_sha256
    }

    /// Embedded schema count.
    #[getter]
    pub fn member_count(&self) -> u32 {
        self.member_count
    }
}

/// One selected generated schema and its exact bytes.
#[pyclass(name = "ContractSchema", frozen)]
pub struct PyContractSchema {
    registry_id: String,
    schema_uri: String,
    wire_schema_id: Option<String>,
    schema_sha256: String,
    bytes: Vec<u8>,
}

#[pymethods]
impl PyContractSchema {
    /// Descriptor-registry identity.
    #[getter]
    pub fn registry_id(&self) -> &str {
        &self.registry_id
    }

    /// Canonical schema URI.
    #[getter]
    pub fn schema_uri(&self) -> &str {
        &self.schema_uri
    }

    /// Standalone wire identity, when present.
    #[getter]
    pub fn wire_schema_id(&self) -> Option<&str> {
        self.wire_schema_id.as_deref()
    }

    /// SHA-256 of the exact raw schema bytes.
    #[getter]
    pub fn schema_sha256(&self) -> &str {
        &self.schema_sha256
    }

    /// Clone the exact raw schema bytes as one Python `bytes` value.
    #[getter]
    pub fn bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, &self.bytes)
    }
}

/// Canonical result of structural document validation.
#[pyclass(name = "ValidatedContractDocument", frozen)]
pub struct PyValidatedContractDocument {
    schema_id: String,
    schema_sha256: String,
    canonical_bytes: Vec<u8>,
}

#[pymethods]
impl PyValidatedContractDocument {
    /// Admitted registry identity.
    #[getter]
    pub fn schema_id(&self) -> &str {
        &self.schema_id
    }

    /// SHA-256 of the admitting schema.
    #[getter]
    pub fn schema_sha256(&self) -> &str {
        &self.schema_sha256
    }

    /// Clone the RFC 8785 document bytes.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, &self.canonical_bytes)
    }
}
