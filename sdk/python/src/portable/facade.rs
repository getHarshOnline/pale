// Python façade-method fragment included by the shallow projection owner.
#[pymethods]
impl PalePython {
    /// Inspect the complete embedded public-contract bundle coordinate.
    pub fn contract_bundle_identity(&self) -> PyContractBundleIdentity {
        let identity = self.api.contract_bundle_identity();
        PyContractBundleIdentity {
            schema_manifest_sha256: identity.schema_manifest_sha256().to_owned(),
            schema_set_sha256: identity.schema_set_sha256().to_owned(),
            member_count: identity.member_count(),
        }
    }

    /// Look up one façade-selected generated schema.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the identity is not
    /// selected by the API contract bundle.
    pub fn contract_schema(&self, py: Python<'_>, schema_id: String) -> PyResult<PyContractSchema> {
        let api = Arc::clone(&self.api);
        py.detach(move || api.contract_schema(&schema_id))
            .map(|schema| PyContractSchema {
                registry_id: schema.registry_id().to_owned(),
                schema_uri: schema.schema_uri().to_owned(),
                wire_schema_id: schema.wire_schema_id().map(str::to_owned),
                schema_sha256: schema.schema_sha256().to_owned(),
                bytes: schema.bytes().to_vec(),
            })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Validate and canonicalize one selected contract document.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the schema identity or
    /// supplied document fails API-owned contract admission.
    pub fn validate_contract_document(
        &self,
        py: Python<'_>,
        schema_id: String,
        bytes: Vec<u8>,
    ) -> PyResult<PyValidatedContractDocument> {
        let api = Arc::clone(&self.api);
        py.detach(move || api.validate_contract_document(ContractDocument::new(schema_id, bytes)))
            .map(|document| PyValidatedContractDocument {
                schema_id: document.schema_id().to_owned(),
                schema_sha256: document.schema_sha256().to_owned(),
                canonical_bytes: document.canonical_bytes().to_vec(),
            })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Seal a pure workspace-module preimage into canonical initial-v1 bytes.
    ///
    /// Python supplies owned JSON-shaped input only. The API-owned Rust kernel
    /// performs strict decoding, canonical set ordering, invariant validation,
    /// digest construction, generated-schema validation, and RFC 8785 encoding.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the candidate is malformed
    /// or violates the workspace-module construction contract.
    pub fn build_workspace_module(
        &self,
        py: Python<'_>,
        preimage: Vec<u8>,
    ) -> PyResult<PyValidatedContractDocument> {
        let api = Arc::clone(&self.api);
        py.detach(move || api.build_workspace_module(WorkspaceModuleCandidate::new(preimage)))
            .map(|document| PyValidatedContractDocument {
                schema_id: document.schema_id().to_owned(),
                schema_sha256: document.schema_sha256().to_owned(),
                canonical_bytes: document.canonical_bytes().to_vec(),
            })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Project a typed owner classification into canonical `diagnostic.v1` bytes.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the typed projection cannot
    /// be admitted or canonically encoded.
    pub fn project_diagnostic(
        &self,
        py: Python<'_>,
        projection: PyDiagnosticProjection,
    ) -> PyResult<Py<PyBytes>> {
        let projection = projection.into_api();
        let api = Arc::clone(&self.api);
        py.detach(move || api.project_diagnostic(projection))
            .map(|diagnostic| crate::python_bytes(py, diagnostic.canonical_bytes()))
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Admit canonical `diagnostic.v1` bytes and return the same bytes.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the bytes are not the
    /// canonical encoding of the initial-v1 diagnostic contract.
    pub fn admit_diagnostic(
        &self,
        py: Python<'_>,
        bytes: Vec<u8>,
    ) -> PyResult<Py<PyBytes>> {
        let api = Arc::clone(&self.api);
        py.detach(move || api.admit_diagnostic(&bytes))
            .map(|diagnostic| crate::python_bytes(py, diagnostic.canonical_bytes()))
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Compile one path-free workspace source batch.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when a source member or
    /// contextual workspace relationship fails API-owned admission.
    pub fn compile_portable_workspace(
        &self,
        py: Python<'_>,
        root_manifest: String,
        documents: Vec<(String, Vec<u8>)>,
    ) -> PyResult<PyPortableWorkspaceCompilation> {
        let request = PortableWorkspaceRequest::new(
            root_manifest,
            documents
                .into_iter()
                .map(|(label, bytes)| PortableWorkspaceDocument::new(label, bytes))
                .collect(),
        );
        let api = Arc::clone(&self.api);
        py.detach(move || api.compile_portable_workspace(request))
            .map(|inner| PyPortableWorkspaceCompilation {
                inner: Arc::new(inner),
            })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Compare two admitted portable workspace handles.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the admitted workspaces
    /// cannot form one valid comparison coordinate.
    pub fn compare_portable_workspaces(
        &self,
        py: Python<'_>,
        prior: &PyPortableWorkspaceCompilation,
        current: &PyPortableWorkspaceCompilation,
    ) -> PyResult<PyWorkspaceComparison> {
        let api = Arc::clone(&self.api);
        let prior = Arc::clone(&prior.inner);
        let current = Arc::clone(&current.inner);
        py.detach(move || api.compare_portable_workspaces(&prior, &current))
            .map(|inner| PyWorkspaceComparison {
                inner: Arc::new(inner),
            })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Return the complete effect-free native-action descriptor batch.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] if the compiled registry and its
    /// generated action contracts contradict one another.
    pub fn native_action_catalog(&self, py: Python<'_>) -> PyResult<Vec<PyNativeActionDescriptor>> {
        let api = Arc::clone(&self.api);
        py.detach(move || api.native_action_descriptors())
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(PyNativeActionDescriptor::from_api)
                    .collect()
            })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Execute one complete tagged effect-free native action.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the tagged request fails
    /// action-contract admission or pure execution.
    pub fn invoke_native_action(
        &self,
        py: Python<'_>,
        action_id: String,
        input: Vec<u8>,
    ) -> PyResult<Py<PyBytes>> {
        let api = Arc::clone(&self.api);
        py.detach(move || api.invoke_native_action(NativeActionRequest::new(action_id, input)))
            .map(|execution| crate::python_bytes(py, execution.canonical_bytes()))
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Execute one carrier-selected effect-free native-action payload.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when the action identity,
    /// payload, or pure execution fails API-owned admission.
    pub fn invoke_native_action_payload(
        &self,
        py: Python<'_>,
        action_id: String,
        payload: Vec<u8>,
    ) -> PyResult<Py<PyBytes>> {
        let api = Arc::clone(&self.api);
        py.detach(move || {
            api.invoke_native_action_payload(NativeActionPayloadRequest::new(action_id, payload))
        })
        .map(|execution| crate::python_bytes(py, execution.canonical_bytes()))
        .map_err(|error| diagnostic_error(py, error))
    }
}
