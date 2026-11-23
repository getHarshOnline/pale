//! Python exception projection without a second failure taxonomy.

use pale_sdk::{ApiError, ApiInitializationError};
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyRuntimeError};
use pyo3::types::{PyAnyMethods, PyBytes};
use pyo3::{PyErr, Python};

use crate::{PyAuthoringCorrection, PyAuthoringSourceLocation, PyDiagnostic};

create_exception!(
    _native,
    PaleDiagnosticError,
    PyException,
    "A Pale failure whose message is the canonical diagnostic summary, whose diagnostic attribute is a typed admitted view, and whose diagnostic_bytes attribute carries exact diagnostic.v1 bytes."
);

/// Project one canonical API diagnostic into a Python-native exception.
///
/// The exception message is the concise safe summary. Its `diagnostic`
/// attribute is a typed view projected directly from the admitted record, and
/// `diagnostic_bytes` retains the exact canonical `diagnostic.v1` bytes.
/// Separate `authoring_corrections` and `authoring_location` attributes retain
/// compiler-proved edits and exact source coordinates; neither grants write authority.
/// If typed-view allocation or host-controlled attribute assignment returns a
/// Python error, it escapes instead of a partially decorated Pale exception.
pub(crate) fn diagnostic_error(py: Python<'_>, error: ApiError) -> PyErr {
    let diagnostic = error.diagnostic();
    let projected = PaleDiagnosticError::new_err(diagnostic.summary().to_owned());
    let canonical = PyBytes::new(py, diagnostic.canonical_bytes());
    let typed = match pyo3::Py::new(py, PyDiagnostic::from_api(diagnostic)) {
        Ok(typed) => typed,
        Err(error) => return error,
    };
    let exception = projected.value(py);
    if let Err(error) = exception
        .setattr("diagnostic", typed)
        .and_then(|()| exception.setattr("diagnostic_bytes", canonical))
        .and_then(|()| {
            exception.setattr(
                "authoring_corrections",
                error
                    .authoring_corrections()
                    .iter()
                    .map(PyAuthoringCorrection::from_api)
                    .collect::<Vec<_>>(),
            )
        })
        .and_then(|()| {
            exception.setattr(
                "authoring_location",
                error
                    .authoring_location()
                    .map(PyAuthoringSourceLocation::from_api),
            )
        })
    {
        return error;
    }
    projected
}

/// Project the pre-diagnostic initialization boundary distinctly.
pub(crate) fn initialization_error(error: ApiInitializationError) -> PyErr {
    PyRuntimeError::new_err(format!("{}: {error}", error.code()))
}
