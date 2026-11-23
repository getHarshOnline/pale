// Python projection of one selected fact's direct compiler-proof inputs.

/// Opaque continuation retaining the exact compilation, subject and raw limit.
#[pyclass(name = "PortableWorkspaceDependencyCursor", frozen)]
pub struct PyPortableWorkspaceDependencyCursor {
    inner: pale_sdk::PortableWorkspaceDependencyCursor,
}

/// Selected root fact and its direct inputs, with no transitive graph traversal.
/// An absent root differs from a present root with no dependencies. Inputs point
/// toward the root in compiler proof-digest order, not execution or routing order.
#[pyclass(name = "PortableWorkspaceDependencyPage", frozen)]
pub struct PyPortableWorkspaceDependencyPage {
    inner: pale_sdk::PortableWorkspaceDependencyPage,
}

#[pymethods]
impl PyPortableWorkspaceDependencyPage {
    /// Queried compilation's admitted-universe identity.
    #[getter]
    pub fn principal_universe_digest(&self) -> &str {
        self.inner.principal_universe_digest()
    }
    /// Queried compilation's semantic-IR identity.
    #[getter]
    pub fn semantic_ir_digest(&self) -> &str {
        self.inner.semantic_ir_digest()
    }
    /// Copy the full requested semantic subject.
    pub fn subject(&self) -> PyWorkspaceSubject {
        PyWorkspaceSubject::from_api(self.inner.subject())
    }
    /// Copy the selected fact and all its proof fields, or `None` when absent.
    pub fn root(&self) -> Option<PyWorkspaceCausalStep> {
        self.inner
            .root()
            .cloned()
            .map(|inner| PyWorkspaceCausalStep { inner })
    }
    /// Copy every direct input with its subject, rule, provenance and typed fact.
    pub fn dependencies(&self) -> Vec<PyWorkspaceCausalStep> {
        self.inner
            .dependencies()
            .iter()
            .cloned()
            .map(|inner| PyWorkspaceCausalStep { inner })
            .collect()
    }
    /// Whether this query has exhausted its direct inputs.
    #[getter]
    pub fn is_complete(&self) -> bool {
        self.inner.continuation().is_none()
    }
    /// Retain the next position, or `None` after exhaustion.
    #[getter]
    pub fn continuation(&self) -> Option<PyPortableWorkspaceDependencyCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| PyPortableWorkspaceDependencyCursor { inner })
    }
    /// Copy the SDK's canonical projection without encoding the opaque cursor.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, self.inner.canonical_bytes())
    }
}

#[pymethods]
impl PyPortableWorkspaceCompilation {
    /// Page direct proof inputs from the retained compiler owner.
    /// Zero selects 64 rows; limits above 256 cap at 256. Resume using the same
    /// raw limit, full subject and owner. Equal-digest recompilation is not the
    /// same owner. Limits bound input rows, not bytes or aggregate memory; no
    /// source parsing, dependency closure or recipient admission occurs here.
    ///
    /// # Errors
    /// Raises [`crate::PaleDiagnosticError`] for a cursor from another owner,
    /// subject or raw limit, or an inconsistent internal position. Invalid
    /// argument types/ranges fail at the ABI.
    #[pyo3(signature = (subject, limit: "int", continuation=None))]
    pub fn direct_dependencies(
        &self,
        py: Python<'_>,
        subject: &PyWorkspaceSubject,
        #[pyo3(from_py_with = crate::authoring::page_limit)] limit: u16,
        continuation: Option<&PyPortableWorkspaceDependencyCursor>,
    ) -> PyResult<PyPortableWorkspaceDependencyPage> {
        let owner = Arc::clone(&self.inner);
        let subject = subject.inner.clone();
        let continuation = continuation.map(|cursor| cursor.inner.clone());
        py.detach(move || owner.direct_dependencies(&subject, continuation.as_ref(), limit))
            .map(|inner| PyPortableWorkspaceDependencyPage { inner })
            .map_err(|error| diagnostic_error(py, error))
    }
}
