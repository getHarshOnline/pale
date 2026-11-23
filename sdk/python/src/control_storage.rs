//! Python projection of read-only physical control-store inspection.
//!
//! The API owns validation, classification and unresolved-requirement order.
//! Even a physical `Ready` result confers no writer or resident authority.

use std::sync::Arc;

use pale_sdk::{
    LocalControlStorageInspection, LocalControlStorageInspectionRejection,
    LocalControlStorageInspectionRequest, LocalControlStorageInspectionState,
    LocalControlStorageProfileRequirement, LocalControlStoreKind,
};
use pyo3::prelude::*;

use crate::{error::diagnostic_error, PalePython};

/// Scope-specific input; the API retains and redacts the private runtime root.
#[pyclass(name = "LocalControlStorageInspectionRequest", frozen, from_py_object)]
#[derive(Clone, Debug)]
pub struct PyLocalControlStorageInspectionRequest {
    inner: LocalControlStorageInspectionRequest,
}

#[pymethods]
impl PyLocalControlStorageInspectionRequest {
    /// Select a principal trust store without validation or host observation.
    #[staticmethod]
    #[pyo3(signature = (*, runtime_state_root, principal_id, logical_store_id, control_incarnation_id))]
    pub fn principal(
        runtime_state_root: String,
        principal_id: String,
        logical_store_id: String,
        control_incarnation_id: String,
    ) -> Self {
        Self {
            inner: LocalControlStorageInspectionRequest::principal(
                runtime_state_root,
                principal_id,
                logical_store_id,
                control_incarnation_id,
            ),
        }
    }

    /// Select a deployment control store without validation or host observation.
    #[staticmethod]
    #[pyo3(signature = (*, runtime_state_root, workspace_id, deployment_binding_ref, deployment_binding_sha256, logical_store_id, control_incarnation_id))]
    pub fn deployment(
        runtime_state_root: String,
        workspace_id: String,
        deployment_binding_ref: String,
        deployment_binding_sha256: String,
        logical_store_id: String,
        control_incarnation_id: String,
    ) -> Self {
        Self {
            inner: LocalControlStorageInspectionRequest::deployment(
                runtime_state_root,
                workspace_id,
                deployment_binding_ref,
                deployment_binding_sha256,
                logical_store_id,
                control_incarnation_id,
            ),
        }
    }
}

/// Python projection of [`LocalControlStoreKind`].
#[pyclass(
    name = "LocalControlStoreKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PyLocalControlStoreKind {
    /// Principal-scoped trust store.
    PrincipalTrust,
    /// Deployment-bound operational store.
    DeploymentControl,
}

impl From<LocalControlStoreKind> for PyLocalControlStoreKind {
    fn from(value: LocalControlStoreKind) -> Self {
        match value {
            LocalControlStoreKind::PrincipalTrust => Self::PrincipalTrust,
            LocalControlStoreKind::DeploymentControl => Self::DeploymentControl,
        }
    }
}

/// Python projection of [`LocalControlStorageProfileRequirement`].
#[pyclass(
    name = "LocalControlStorageProfileRequirement",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PyLocalControlStorageProfileRequirement {
    /// Exact `SQLite` compile-option fingerprint.
    SqliteCompileOptions,
    /// Transaction synchronization strength.
    SqliteSynchronous,
    /// Full-filesystem synchronization.
    SqliteFullFsync,
    /// Checkpoint full-filesystem synchronization.
    SqliteCheckpointFullFsync,
    /// WAL checkpoint mode.
    SqliteCheckpointMode,
    /// Automatic checkpoint threshold.
    SqliteWalAutocheckpointFrames,
    /// Durability grouping and release delay.
    SqliteDurabilityGrouping,
    /// Authority-frame encoding.
    AuthorityFrameEncoding,
    /// Corruption-checksum algorithm.
    AuthorityCorruptionChecksum,
    /// Corruption-checksum width.
    AuthorityCorruptionChecksumBytes,
    /// Authority-entry byte ceiling.
    AuthorityMaximumEntryBytes,
    /// Recovery-checkpoint byte ceiling.
    AuthorityMaximumCheckpointBytes,
    /// Recovery-candidate count ceiling.
    AuthorityMaximumCheckpointCandidates,
    /// Contiguous recovery-tail ceiling.
    AuthorityMaximumTailEntries,
    /// Recovery-checkpoint cadence.
    AuthorityCheckpointCadence,
    /// Journal and checkpoint retention.
    AuthorityRetentionPolicy,
    /// Prefix compaction.
    AuthorityCompactionPolicy,
    /// Verified backup and inactive restore.
    BackupRestorePolicy,
}

impl From<LocalControlStorageProfileRequirement> for PyLocalControlStorageProfileRequirement {
    fn from(value: LocalControlStorageProfileRequirement) -> Self {
        match value {
            LocalControlStorageProfileRequirement::SqliteCompileOptions => {
                Self::SqliteCompileOptions
            }
            LocalControlStorageProfileRequirement::SqliteSynchronous => Self::SqliteSynchronous,
            LocalControlStorageProfileRequirement::SqliteFullFsync => Self::SqliteFullFsync,
            LocalControlStorageProfileRequirement::SqliteCheckpointFullFsync => {
                Self::SqliteCheckpointFullFsync
            }
            LocalControlStorageProfileRequirement::SqliteCheckpointMode => {
                Self::SqliteCheckpointMode
            }
            LocalControlStorageProfileRequirement::SqliteWalAutocheckpointFrames => {
                Self::SqliteWalAutocheckpointFrames
            }
            LocalControlStorageProfileRequirement::SqliteDurabilityGrouping => {
                Self::SqliteDurabilityGrouping
            }
            LocalControlStorageProfileRequirement::AuthorityFrameEncoding => {
                Self::AuthorityFrameEncoding
            }
            LocalControlStorageProfileRequirement::AuthorityCorruptionChecksum => {
                Self::AuthorityCorruptionChecksum
            }
            LocalControlStorageProfileRequirement::AuthorityCorruptionChecksumBytes => {
                Self::AuthorityCorruptionChecksumBytes
            }
            LocalControlStorageProfileRequirement::AuthorityMaximumEntryBytes => {
                Self::AuthorityMaximumEntryBytes
            }
            LocalControlStorageProfileRequirement::AuthorityMaximumCheckpointBytes => {
                Self::AuthorityMaximumCheckpointBytes
            }
            LocalControlStorageProfileRequirement::AuthorityMaximumCheckpointCandidates => {
                Self::AuthorityMaximumCheckpointCandidates
            }
            LocalControlStorageProfileRequirement::AuthorityMaximumTailEntries => {
                Self::AuthorityMaximumTailEntries
            }
            LocalControlStorageProfileRequirement::AuthorityCheckpointCadence => {
                Self::AuthorityCheckpointCadence
            }
            LocalControlStorageProfileRequirement::AuthorityRetentionPolicy => {
                Self::AuthorityRetentionPolicy
            }
            LocalControlStorageProfileRequirement::AuthorityCompactionPolicy => {
                Self::AuthorityCompactionPolicy
            }
            LocalControlStorageProfileRequirement::BackupRestorePolicy => Self::BackupRestorePolicy,
        }
    }
}

/// Discriminant of [`LocalControlStorageInspectionRejection`].
#[pyclass(
    name = "LocalControlStorageInspectionRejectionKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PyLocalControlStorageInspectionRejectionKind {
    /// Unsafe persistent entry shape.
    UnsafeEntryKind,
    /// Undecodable or incomplete structures.
    CorruptOrPartial,
    /// Different physical store kind.
    WrongStoreKind,
    /// Different logical store identity.
    WrongLogicalStoreId,
    /// Different store incarnation.
    WrongControlIncarnation,
    /// Different principal scope.
    WrongPrincipalScope,
    /// Different deployment scope.
    WrongDeploymentScope,
    /// Different embedded source identity.
    SourceIdentityMismatch,
    /// Unsupported newer physical schema.
    NewerVersion,
    /// Different `SQLite` build facts.
    SqliteBuildMismatch,
    /// Different fixed structural settings.
    StructuralProfileMismatch,
}

/// Discriminant of [`LocalControlStorageInspectionState`].
#[pyclass(
    name = "LocalControlStorageInspectionStateKind",
    eq,
    eq_int,
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PyLocalControlStorageInspectionStateKind {
    /// Canonical store does not exist.
    Missing,
    /// Canonical store is an empty regular file.
    Empty,
    /// Structural match with unresolved profile requirements.
    ProfileUnselected,
    /// Physical checks match; no runtime or writer admission is implied.
    Ready,
    /// Persistent contradiction for the expected coordinate.
    Rejected,
}

/// Persistent physical contradiction, retaining any version evidence.
#[pyclass(name = "LocalControlStorageInspectionRejection", frozen)]
pub struct PyLocalControlStorageInspectionRejection {
    inner: LocalControlStorageInspectionRejection,
}

#[pymethods]
impl PyLocalControlStorageInspectionRejection {
    /// Exact API-owned rejection category.
    #[getter]
    pub fn kind(&self) -> PyLocalControlStorageInspectionRejectionKind {
        use LocalControlStorageInspectionRejection as Rejection;
        use PyLocalControlStorageInspectionRejectionKind as Kind;
        match self.inner {
            Rejection::UnsafeEntryKind => Kind::UnsafeEntryKind,
            Rejection::CorruptOrPartial => Kind::CorruptOrPartial,
            Rejection::WrongStoreKind => Kind::WrongStoreKind,
            Rejection::WrongLogicalStoreId => Kind::WrongLogicalStoreId,
            Rejection::WrongControlIncarnation => Kind::WrongControlIncarnation,
            Rejection::WrongPrincipalScope => Kind::WrongPrincipalScope,
            Rejection::WrongDeploymentScope => Kind::WrongDeploymentScope,
            Rejection::SourceIdentityMismatch => Kind::SourceIdentityMismatch,
            Rejection::NewerVersion { .. } => Kind::NewerVersion,
            Rejection::SqliteBuildMismatch => Kind::SqliteBuildMismatch,
            Rejection::StructuralProfileMismatch => Kind::StructuralProfileMismatch,
        }
    }

    /// Observed physical version, present only for a newer-version rejection.
    #[getter]
    pub fn observed(&self) -> Option<u32> {
        match self.inner {
            LocalControlStorageInspectionRejection::NewerVersion { observed, .. } => Some(observed),
            _ => None,
        }
    }

    /// Supported physical version, present only for a newer-version rejection.
    #[getter]
    pub fn supported(&self) -> Option<u32> {
        match self.inner {
            LocalControlStorageInspectionRejection::NewerVersion { supported, .. } => {
                Some(supported)
            }
            _ => None,
        }
    }
}

/// Physical classification, not an executable or runtime-readiness handle.
#[pyclass(name = "LocalControlStorageInspectionState", frozen)]
pub struct PyLocalControlStorageInspectionState {
    inner: LocalControlStorageInspectionState,
}

#[pymethods]
impl PyLocalControlStorageInspectionState {
    /// Exact physical-state category.
    #[getter]
    pub fn kind(&self) -> PyLocalControlStorageInspectionStateKind {
        use LocalControlStorageInspectionState as State;
        use PyLocalControlStorageInspectionStateKind as Kind;
        match self.inner {
            State::Missing => Kind::Missing,
            State::Empty => Kind::Empty,
            State::ProfileUnselected { .. } => Kind::ProfileUnselected,
            State::Ready => Kind::Ready,
            State::Rejected(_) => Kind::Rejected,
        }
    }

    /// Unselected requirements in source-owned order; absent in other states.
    #[getter]
    pub fn unresolved(&self) -> Option<Vec<PyLocalControlStorageProfileRequirement>> {
        match &self.inner {
            LocalControlStorageInspectionState::ProfileUnselected { unresolved } => {
                Some(unresolved.iter().copied().map(Into::into).collect())
            }
            _ => None,
        }
    }

    /// Persistent contradiction, present only in the rejected state.
    #[getter]
    pub fn rejection(&self) -> Option<PyLocalControlStorageInspectionRejection> {
        match &self.inner {
            LocalControlStorageInspectionState::Rejected(inner) => {
                Some(PyLocalControlStorageInspectionRejection {
                    inner: inner.clone(),
                })
            }
            _ => None,
        }
    }
}

/// Owned, path-free physical inspection returned by the canonical API.
#[pyclass(name = "LocalControlStorageInspection", frozen)]
pub struct PyLocalControlStorageInspection {
    inner: LocalControlStorageInspection,
}

#[pymethods]
impl PyLocalControlStorageInspection {
    /// Store kind selected by the exact request scope.
    #[getter]
    pub fn kind(&self) -> PyLocalControlStoreKind {
        self.inner.kind().into()
    }

    /// Physical classification; this never supplies Pale Runtime assembly evidence.
    #[getter]
    pub fn state(&self) -> PyLocalControlStorageInspectionState {
        PyLocalControlStorageInspectionState {
            inner: self.inner.state().clone(),
        }
    }
}

#[pymethods]
impl PalePython {
    /// Inspect one canonical store without creating, repairing or admitting it.
    ///
    /// Rust-only inspection detaches from the Python interpreter. Missing and
    /// rejected stores remain typed physical observations, not generic health.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] for invalid coordinates or a
    /// failed host observation, preserving the API-owned diagnostic.
    pub fn inspect_local_control_store(
        &self,
        py: Python<'_>,
        request: PyLocalControlStorageInspectionRequest,
    ) -> PyResult<PyLocalControlStorageInspection> {
        let api = Arc::clone(&self.api);
        py.detach(move || api.inspect_local_control_store(request.inner))
            .map(|inner| PyLocalControlStorageInspection { inner })
            .map_err(|error| diagnostic_error(py, error))
    }
}

#[cfg(test)]
#[path = "../tests/internal/control_storage.rs"]
mod tests;
