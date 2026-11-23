// Python values for one subject compared across retained compilations.

/// Application-established equality of a shared subject's typed outputs.
///
/// Source equality includes its logical label, byte digest and length. This
/// does not classify dependency impact, proof ancestry or native state.
#[pyclass(
    name = "WorkspaceValueComparison",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyWorkspaceValueComparison {
    /// Both compiler-owned outputs are equal.
    Equal,
    /// The compiler-owned outputs differ.
    Changed,
}

/// Neither retained compilation contains the requested subject.
#[pyclass(name = "AbsentWorkspaceSubjectValues", frozen)]
pub struct PyAbsentWorkspaceSubjectValues {}

/// Only the predecessor contains the requested subject.
#[pyclass(name = "PriorOnlyWorkspaceSubjectValues", frozen)]
pub struct PyPriorOnlyWorkspaceSubjectValues {
    prior: PortableWorkspaceFact,
}

#[pymethods]
impl PyPriorOnlyWorkspaceSubjectValues {
    /// Copy the predecessor's typed output.
    #[getter]
    fn prior(&self) -> PyWorkspaceFact {
        PyWorkspaceFact::from_api(&self.prior)
    }
}

/// Only the current compilation contains the requested subject.
#[pyclass(name = "CurrentOnlyWorkspaceSubjectValues", frozen)]
pub struct PyCurrentOnlyWorkspaceSubjectValues {
    current: PortableWorkspaceFact,
}

#[pymethods]
impl PyCurrentOnlyWorkspaceSubjectValues {
    /// Copy the current typed output.
    #[getter]
    fn current(&self) -> PyWorkspaceFact {
        PyWorkspaceFact::from_api(&self.current)
    }
}

/// Both compilations contain the subject, with independently compared values.
#[pyclass(name = "SharedWorkspaceSubjectValues", frozen)]
pub struct PySharedWorkspaceSubjectValues {
    prior: PortableWorkspaceFact,
    current: PortableWorkspaceFact,
    comparison: pale_sdk::PortableWorkspaceValueComparison,
}

#[pymethods]
impl PySharedWorkspaceSubjectValues {
    /// Copy the predecessor's typed output.
    #[getter]
    fn prior(&self) -> PyWorkspaceFact {
        PyWorkspaceFact::from_api(&self.prior)
    }

    /// Copy the current typed output.
    #[getter]
    fn current(&self) -> PyWorkspaceFact {
        PyWorkspaceFact::from_api(&self.current)
    }

    /// Equality decided by the application, not by the Python projection.
    #[getter]
    fn comparison(&self) -> PyWorkspaceValueComparison {
        match self.comparison {
            pale_sdk::PortableWorkspaceValueComparison::Equal => PyWorkspaceValueComparison::Equal,
            pale_sdk::PortableWorkspaceValueComparison::Changed => {
                PyWorkspaceValueComparison::Changed
            }
        }
    }
}

#[derive(IntoPyObject)]
pub(crate) enum PyWorkspaceSubjectValues {
    #[pyo3(transparent)]
    Absent(PyAbsentWorkspaceSubjectValues),
    #[pyo3(transparent)]
    PriorOnly(PyPriorOnlyWorkspaceSubjectValues),
    #[pyo3(transparent)]
    CurrentOnly(PyCurrentOnlyWorkspaceSubjectValues),
    #[pyo3(transparent)]
    Shared(PySharedWorkspaceSubjectValues),
}

/// Owned answer for one subject in an existing workspace comparison.
///
/// The receipt identifies the parent comparison. This answer carries selected
/// facts, not source bodies or graph storage, and survives its producer handles.
#[pyclass(name = "WorkspaceSubjectComparison", frozen)]
pub struct PyWorkspaceSubjectComparison {
    inner: pale_sdk::PortableWorkspaceSubjectComparison,
}

#[pymethods]
impl PyWorkspaceSubjectComparison {
    /// Identity of the existing comparison that produced this answer.
    #[getter]
    fn receipt_digest(&self) -> &str {
        self.inner.receipt_digest()
    }

    /// Copy the full semantic subject supplied to the query.
    fn subject(&self) -> PyWorkspaceSubject {
        PyWorkspaceSubject::from_api(self.inner.subject())
    }

    /// Return the closed presence variant and all of its typed fact payloads.
    #[getter]
    fn values(&self) -> PyWorkspaceSubjectValues {
        use pale_sdk::PortableWorkspaceSubjectValues as Values;
        match self.inner.values() {
            Values::Absent => PyWorkspaceSubjectValues::Absent(PyAbsentWorkspaceSubjectValues {}),
            Values::PriorOnly { prior } => {
                PyWorkspaceSubjectValues::PriorOnly(PyPriorOnlyWorkspaceSubjectValues {
                    prior: prior.clone(),
                })
            }
            Values::CurrentOnly { current } => {
                PyWorkspaceSubjectValues::CurrentOnly(PyCurrentOnlyWorkspaceSubjectValues {
                    current: current.clone(),
                })
            }
            Values::Shared {
                prior,
                current,
                comparison,
            } => PyWorkspaceSubjectValues::Shared(PySharedWorkspaceSubjectValues {
                prior: prior.clone(),
                current: current.clone(),
                comparison: *comparison,
            }),
        }
    }

    /// Copy the SDK's once-produced canonical projection into Python bytes.
    #[getter]
    fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, self.inner.canonical_bytes())
    }
}
