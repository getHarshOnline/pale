//! Read-only Node projection of the canonical local control-store inspector.
//!
//! Physical classification remains owned by [`pale_sdk::PaleApi`]. These
//! inspection results retain no host root, connection, or authority. In particular,
//! [`LocalControlStorageStateKind::Ready`] is not writer or resident readiness.

use crate::{error::diagnostic_error, Pale};
use napi::{Env, Result};
use napi_derive::napi;
use pale_sdk::{
    LocalControlStorageInspection as ApiInspection,
    LocalControlStorageInspectionRejection as ApiRejection,
    LocalControlStorageInspectionRequest as ApiRequest,
    LocalControlStorageInspectionState as ApiState,
    LocalControlStorageProfileRequirement as ApiRequirement,
};

/// Exact physical store class; not a runtime or authority disposition.
#[napi]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalControlStoreKind {
    /// Principal-scoped trust and allocation store.
    PrincipalTrust,
    /// Immutable-deployment-bound operational store.
    DeploymentControl,
}

impl From<pale_sdk::LocalControlStoreKind> for LocalControlStoreKind {
    fn from(value: pale_sdk::LocalControlStoreKind) -> Self {
        match value {
            pale_sdk::LocalControlStoreKind::PrincipalTrust => Self::PrincipalTrust,
            pale_sdk::LocalControlStoreKind::DeploymentControl => Self::DeploymentControl,
        }
    }
}

/// Source-owned unresolved admission coordinate, returned in canonical order.
#[napi]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalControlStorageProfileRequirement {
    /// Exact `SQLite` compile-option fingerprint.
    SqliteCompileOptions,
    /// Transaction synchronization and barrier policy.
    SqliteSynchronous,
    /// Full-filesystem synchronization policy.
    SqliteFullFsync,
    /// Checkpoint full-filesystem synchronization policy.
    SqliteCheckpointFullFsync,
    /// Measured WAL checkpoint mode.
    SqliteCheckpointMode,
    /// Automatic WAL checkpoint threshold.
    SqliteWalAutocheckpointFrames,
    /// Durability grouping and release delay.
    SqliteDurabilityGrouping,
    /// Authority-frame encoding.
    AuthorityFrameEncoding,
    /// Corruption-checksum algorithm.
    AuthorityCorruptionChecksum,
    /// Corruption-checksum width.
    AuthorityCorruptionChecksumBytes,
    /// Journal-entry byte ceiling.
    AuthorityMaximumEntryBytes,
    /// Recovery-checkpoint byte ceiling.
    AuthorityMaximumCheckpointBytes,
    /// Recovery checkpoint candidate bound.
    AuthorityMaximumCheckpointCandidates,
    /// Contiguous recovery-tail entry bound.
    AuthorityMaximumTailEntries,
    /// Recovery-checkpoint cadence.
    AuthorityCheckpointCadence,
    /// Journal and checkpoint retention policy.
    AuthorityRetentionPolicy,
    /// Prefix-compaction policy.
    AuthorityCompactionPolicy,
    /// Verified backup and inactive-restore policy.
    BackupRestorePolicy,
}

impl From<ApiRequirement> for LocalControlStorageProfileRequirement {
    fn from(value: ApiRequirement) -> Self {
        match value {
            ApiRequirement::SqliteCompileOptions => Self::SqliteCompileOptions,
            ApiRequirement::SqliteSynchronous => Self::SqliteSynchronous,
            ApiRequirement::SqliteFullFsync => Self::SqliteFullFsync,
            ApiRequirement::SqliteCheckpointFullFsync => Self::SqliteCheckpointFullFsync,
            ApiRequirement::SqliteCheckpointMode => Self::SqliteCheckpointMode,
            ApiRequirement::SqliteWalAutocheckpointFrames => Self::SqliteWalAutocheckpointFrames,
            ApiRequirement::SqliteDurabilityGrouping => Self::SqliteDurabilityGrouping,
            ApiRequirement::AuthorityFrameEncoding => Self::AuthorityFrameEncoding,
            ApiRequirement::AuthorityCorruptionChecksum => Self::AuthorityCorruptionChecksum,
            ApiRequirement::AuthorityCorruptionChecksumBytes => {
                Self::AuthorityCorruptionChecksumBytes
            }
            ApiRequirement::AuthorityMaximumEntryBytes => Self::AuthorityMaximumEntryBytes,
            ApiRequirement::AuthorityMaximumCheckpointBytes => {
                Self::AuthorityMaximumCheckpointBytes
            }
            ApiRequirement::AuthorityMaximumCheckpointCandidates => {
                Self::AuthorityMaximumCheckpointCandidates
            }
            ApiRequirement::AuthorityMaximumTailEntries => Self::AuthorityMaximumTailEntries,
            ApiRequirement::AuthorityCheckpointCadence => Self::AuthorityCheckpointCadence,
            ApiRequirement::AuthorityRetentionPolicy => Self::AuthorityRetentionPolicy,
            ApiRequirement::AuthorityCompactionPolicy => Self::AuthorityCompactionPolicy,
            ApiRequirement::BackupRestorePolicy => Self::BackupRestorePolicy,
        }
    }
}

/// Mutually exclusive physical inspection states.
#[napi]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalControlStorageStateKind {
    /// Canonical descendants or the database leaf are absent.
    Missing,
    /// The canonical database is a private, zero-length regular file.
    Empty,
    /// Structural bytes match but admission requirements remain unresolved.
    ProfileUnselected,
    /// Physical source and profile checks match; no runtime authority follows.
    Ready,
    /// Persistent bytes or entry shape contradict the expected identity.
    Rejected,
}

/// Distinct persistent rejection reasons, projected without reclassification.
#[napi]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalControlStorageRejectionKind {
    /// The leaf is not a private, non-symlink regular file.
    UnsafeEntryKind,
    /// Required initial-v1 structures cannot be decoded.
    CorruptOrPartial,
    /// Persisted store kind names the other physical class.
    WrongStoreKind,
    /// Persisted logical identity does not match.
    WrongLogicalStoreId,
    /// Persisted control incarnation does not match.
    WrongControlIncarnation,
    /// Persisted principal scope does not match.
    WrongPrincipalScope,
    /// Persisted immutable deployment scope does not match.
    WrongDeploymentScope,
    /// Source or resultant-schema identity differs from embedded source.
    SourceIdentityMismatch,
    /// Persisted schema version is newer than the process understands.
    NewerVersion,
    /// Runtime `SQLite` build differs from the initialization observation.
    SqliteBuildMismatch,
    /// A fixed structural connection setting differs from the manifest.
    StructuralProfileMismatch,
}

/// A persistent rejection whose version detail exists only for a newer schema.
#[napi]
pub struct LocalControlStorageRejection {
    inner: ApiRejection,
}

#[napi]
impl LocalControlStorageRejection {
    /// Exact API-owned persistent contradiction.
    #[napi(getter)]
    pub fn kind(&self) -> LocalControlStorageRejectionKind {
        use LocalControlStorageRejectionKind as Kind;
        match self.inner {
            ApiRejection::UnsafeEntryKind => Kind::UnsafeEntryKind,
            ApiRejection::CorruptOrPartial => Kind::CorruptOrPartial,
            ApiRejection::WrongStoreKind => Kind::WrongStoreKind,
            ApiRejection::WrongLogicalStoreId => Kind::WrongLogicalStoreId,
            ApiRejection::WrongControlIncarnation => Kind::WrongControlIncarnation,
            ApiRejection::WrongPrincipalScope => Kind::WrongPrincipalScope,
            ApiRejection::WrongDeploymentScope => Kind::WrongDeploymentScope,
            ApiRejection::SourceIdentityMismatch => Kind::SourceIdentityMismatch,
            ApiRejection::NewerVersion { .. } => Kind::NewerVersion,
            ApiRejection::SqliteBuildMismatch => Kind::SqliteBuildMismatch,
            ApiRejection::StructuralProfileMismatch => Kind::StructuralProfileMismatch,
        }
    }

    /// Observed schema version, or absent for every non-version rejection.
    #[napi(getter)]
    pub fn observed_version(&self) -> Option<u32> {
        match self.inner {
            ApiRejection::NewerVersion { observed, .. } => Some(observed),
            _ => None,
        }
    }

    /// Greatest supported schema version, only for a newer-version rejection.
    #[napi(getter)]
    pub fn supported_version(&self) -> Option<u32> {
        match self.inner {
            ApiRejection::NewerVersion { supported, .. } => Some(supported),
            _ => None,
        }
    }
}

/// Owned scope-selected request; construction neither validates nor performs I/O.
///
/// The root stays inside the opaque API request. No property or debug projection
/// exposes it. Validation and root confinement occur only when
/// [`Pale::inspect_local_control_store`] invokes the canonical owner.
#[napi]
pub struct LocalControlStorageInspectionRequest {
    inner: ApiRequest,
}

#[napi]
impl LocalControlStorageInspectionRequest {
    /// Select the exact principal trust-store coordinate without host access.
    #[napi(factory)]
    pub fn principal(
        runtime_state_root: String,
        principal_id: String,
        logical_store_id: String,
        control_incarnation_id: String,
    ) -> Self {
        Self {
            inner: ApiRequest::principal(
                runtime_state_root,
                principal_id,
                logical_store_id,
                control_incarnation_id,
            ),
        }
    }

    /// Select the exact immutable deployment-store coordinate without host access.
    #[napi(factory)]
    pub fn deployment(
        runtime_state_root: String,
        workspace_id: String,
        deployment_binding_ref: String,
        deployment_binding_sha256: String,
        logical_store_id: String,
        control_incarnation_id: String,
    ) -> Self {
        Self {
            inner: ApiRequest::deployment(
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

/// Opaque physical result, independent of writer, resident and native readiness.
///
/// Optional details are tied to the retained closed API state, never supplied
/// by JavaScript. The result contains no runtime root or database connection.
#[napi]
pub struct LocalControlStorageInspection {
    inner: ApiInspection,
}

#[napi]
impl LocalControlStorageInspection {
    /// Store class implied by the exact inspected scope.
    #[napi(getter)]
    pub fn kind(&self) -> LocalControlStoreKind {
        self.inner.kind().into()
    }

    /// Physical classification only; [`LocalControlStorageStateKind::Ready`]
    /// never grants authority.
    #[napi(getter)]
    pub fn state(&self) -> LocalControlStorageStateKind {
        match self.inner.state() {
            ApiState::Missing => LocalControlStorageStateKind::Missing,
            ApiState::Empty => LocalControlStorageStateKind::Empty,
            ApiState::ProfileUnselected { .. } => LocalControlStorageStateKind::ProfileUnselected,
            ApiState::Ready => LocalControlStorageStateKind::Ready,
            ApiState::Rejected(_) => LocalControlStorageStateKind::Rejected,
        }
    }

    /// Requirements only for [`LocalControlStorageStateKind::ProfileUnselected`],
    /// preserving source-owned order.
    ///
    /// Absence means a different physical state, not an admitted profile.
    #[napi(getter)]
    pub fn unresolved(&self) -> Option<Vec<LocalControlStorageProfileRequirement>> {
        match self.inner.state() {
            ApiState::ProfileUnselected { unresolved } => {
                Some(unresolved.iter().copied().map(Into::into).collect())
            }
            _ => None,
        }
    }

    /// Persistent contradiction only for [`LocalControlStorageStateKind::Rejected`];
    /// not an API transport failure.
    #[napi(getter)]
    pub fn rejection(&self) -> Option<LocalControlStorageRejection> {
        match self.inner.state() {
            ApiState::Rejected(inner) => Some(LocalControlStorageRejection {
                inner: inner.clone(),
            }),
            _ => None,
        }
    }
}

#[napi]
impl Pale {
    /// Classify an exact local control store without creation, repair or sidecars.
    ///
    /// This synchronous native call returns physical facts, not a resident
    /// status, writer admission, or runtime-readiness report.
    ///
    /// # Errors
    ///
    /// Throws the canonical API diagnostic for invalid coordinates, unsafe
    /// confinement or host inspection failure. Persistent byte contradictions
    /// instead produce [`LocalControlStorageStateKind::Rejected`].
    #[napi]
    pub fn inspect_local_control_store(
        &self,
        env: Env,
        request: &LocalControlStorageInspectionRequest,
    ) -> Result<LocalControlStorageInspection> {
        self.api
            .inspect_local_control_store(request.inner.clone())
            .map(|inner| LocalControlStorageInspection { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }
}

#[cfg(test)]
#[path = "../tests/internal/control_storage.rs"]
mod tests;
