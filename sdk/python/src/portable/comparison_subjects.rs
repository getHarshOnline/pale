// Python projection of selected identities across a retained comparison.

/// Opaque continuation bound to the ordered input pair, receipt and raw limit.
/// Both compilation owners remain retained until this token is released.
#[pyclass(name = "WorkspaceComparisonSubjectCursor", frozen)]
pub struct PyWorkspaceComparisonSubjectCursor {
    inner: pale_sdk::PortableWorkspaceComparisonSubjectCursor,
}

/// Existing comparison coordinates and current-impact counts, not union size.
#[pyclass(name = "WorkspaceComparisonSummary", frozen)]
pub struct PyWorkspaceComparisonSummary {
    inner: pale_sdk::PortableWorkspaceComparisonSummary,
}

#[pymethods]
impl PyWorkspaceComparisonSummary {
    /// Predecessor admitted-universe identity.
    #[getter]
    pub fn prior_principal_universe_digest(&self) -> &str {
        self.inner.prior_principal_universe_digest()
    }
    /// Current admitted-universe identity.
    #[getter]
    pub fn current_principal_universe_digest(&self) -> &str {
        self.inner.current_principal_universe_digest()
    }
    /// Predecessor semantic-IR identity.
    #[getter]
    pub fn prior_semantic_ir_digest(&self) -> &str {
        self.inner.prior_semantic_ir_digest()
    }
    /// Current semantic-IR identity.
    #[getter]
    pub fn current_semantic_ir_digest(&self) -> &str {
        self.inner.current_semantic_ir_digest()
    }
    /// Exact comparison receipt identity.
    #[getter]
    pub fn receipt_digest(&self) -> &str {
        self.inner.receipt_digest()
    }
    /// Number of exact change roots.
    #[getter]
    pub fn change_root_count(&self) -> u32 {
        self.inner.change_root_count()
    }
    /// Number of affected current subjects.
    #[getter]
    pub fn affected_count(&self) -> u32 {
        self.inner.affected_count()
    }
    /// Number of unaffected current subjects.
    #[getter]
    pub fn unaffected_count(&self) -> u32 {
        self.inner.unaffected_count()
    }
    /// Copy the SDK's preconstructed canonical summary bytes.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, self.inner.canonical_bytes())
    }
}

/// Selected identities from either side, once each in canonical subject order.
/// Terminal and pending pages retain the exact pair. Point comparison supplies
/// membership and typed values; discovery does not classify dependency impact.
#[pyclass(name = "WorkspaceComparisonSubjectPage", frozen)]
pub struct PyWorkspaceComparisonSubjectPage {
    inner: pale_sdk::PortableWorkspaceComparisonSubjectPage,
}

#[pymethods]
impl PyWorkspaceComparisonSubjectPage {
    /// Copy the existing comparison summary without recalculating its counts.
    pub fn comparison(&self) -> PyWorkspaceComparisonSummary {
        PyWorkspaceComparisonSummary {
            inner: self.inner.comparison().clone(),
        }
    }
    /// Copy full identities selected by the application.
    pub fn subjects(&self) -> Vec<PyWorkspaceSubject> {
        self.inner
            .subjects()
            .iter()
            .map(PyWorkspaceSubject::from_api)
            .collect()
    }
    /// Entries fetched, including at most one look-ahead per input side.
    #[getter]
    pub fn inspected(&self) -> usize {
        self.inner.inspected()
    }
    /// Merge-head comparisons, excluding comparisons inside range seeks.
    #[getter]
    pub fn key_comparisons(&self) -> usize {
        self.inner.key_comparisons()
    }
    /// Whether both retained ranges are exhausted.
    #[getter]
    pub fn is_complete(&self) -> bool {
        self.inner.continuation().is_none()
    }
    /// Retain the next position, or return `None` after exhaustion.
    #[getter]
    pub fn continuation(&self) -> Option<PyWorkspaceComparisonSubjectCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| PyWorkspaceComparisonSubjectCursor { inner })
    }
    /// Check the exact ordered input owners and receipt, not equal digests.
    pub fn belongs_to(&self, comparison: &PyWorkspaceComparison) -> bool {
        self.inner.belongs_to(&comparison.inner)
    }
    /// Copy the SDK-owned canonical projection; it cannot reconstruct custody.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, self.inner.canonical_bytes())
    }
}

#[pymethods]
impl PyWorkspaceComparison {
    /// Discover selected identities on either side, including predecessor-only facts.
    /// Zero selects 64 rows; limits above 256 cap at 256. The raw limit remains
    /// bound into the cursor. This limits rows, not key bytes, time or retained
    /// memory. It performs no graph walk, value comparison or recipient admission.
    ///
    /// # Errors
    /// Raises [`crate::PaleDiagnosticError`] for a cursor from another ordered
    /// pair, receipt or raw limit. Invalid argument types/ranges fail at the ABI.
    #[pyo3(signature = (limit: "int", continuation=None))]
    pub fn subjects(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = crate::authoring::page_limit)] limit: u16,
        continuation: Option<&PyWorkspaceComparisonSubjectCursor>,
    ) -> PyResult<PyWorkspaceComparisonSubjectPage> {
        let comparison = Arc::clone(&self.inner);
        let continuation = continuation.map(|cursor| cursor.inner.clone());
        py.detach(move || comparison.subjects(continuation.as_ref(), limit))
            .map(|inner| PyWorkspaceComparisonSubjectPage { inner })
            .map_err(|error| diagnostic_error(py, error))
    }
}
