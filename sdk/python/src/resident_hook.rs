//! Admit repository hook inputs through the Rust SDK; do not start a resident.

use std::sync::Arc;

use pale_sdk::{ResidentHookAdmission, ResidentHookRequest};
use pyo3::prelude::*;

use crate::error::diagnostic_error;
use crate::PalePython;

/// Complete native event awaiting API-owned resident-hook admission.
#[pyclass(name = "ResidentHookRequest", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyResidentHookRequest {
    event: String,
    workspace: String,
    repository: String,
    provider_binding: String,
    provider_binding_identity: String,
    payload: Vec<u8>,
}

#[pymethods]
impl PyResidentHookRequest {
    /// Capture one event and its complete bounded native payload.
    #[new]
    #[pyo3(signature = (*, event, workspace, repository, provider_binding, provider_binding_identity, payload))]
    pub fn new(
        event: String,
        workspace: String,
        repository: String,
        provider_binding: String,
        provider_binding_identity: String,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            event,
            workspace,
            repository,
            provider_binding,
            provider_binding_identity,
            payload,
        }
    }
}

/// Opaque proof that static routing and native payload shape agree.
///
/// The handle grants no carrier, active generation, session, policy, or effect
/// authority. Dropping it has no external consequence.
#[pyclass(name = "ResidentHookAdmission", frozen)]
pub struct PyResidentHookAdmission {
    inner: ResidentHookAdmission,
}

#[pymethods]
impl PalePython {
    /// Admit one repository-hook request without selecting or invoking a carrier.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] when routing, identity,
    /// lifecycle-event, provider-binding, or payload admission fails.
    pub fn admit_resident_hook(
        &self,
        py: Python<'_>,
        request: PyResidentHookRequest,
    ) -> PyResult<PyResidentHookAdmission> {
        let api = Arc::clone(&self.api);
        let request = ResidentHookRequest::new(
            request.event,
            request.workspace,
            request.repository,
            request.provider_binding,
            request.provider_binding_identity,
            request.payload,
        );
        py.detach(move || api.admit_resident_hook(request))
            .map(|inner| PyResidentHookAdmission { inner })
            .map_err(|error| diagnostic_error(py, error))
    }

    /// Reject an admitted hook explicitly when no authenticated carrier exists.
    ///
    /// This API-owned bootstrap non-effect never attempts IPC, falls back to
    /// one-shot evaluation, or infers resident availability.
    ///
    /// # Errors
    ///
    /// Always raises the canonical unavailable-carrier diagnostic.
    pub fn resident_hook_transport_unavailable(
        &self,
        py: Python<'_>,
        admission: &PyResidentHookAdmission,
    ) -> PyResult<()> {
        Err(diagnostic_error(
            py,
            self.api
                .resident_hook_transport_unavailable(&admission.inner),
        ))
    }
}
