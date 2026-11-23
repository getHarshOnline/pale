//! Binding-safe projection of read-only local control-store classification.
//!
//! This module translates owned strings into typed adapter coordinates, invokes
//! the canonical read-only classifier once, and projects only physical facts.
//! It does not expose host paths, open connections, adapter types, writer
//! authority, Pale Runtime assembly, repair, initialization, or general health.

use std::path::{Component, PathBuf};

use pale_adapter_control_storage::{
    ControlStorageProfileRequirement, ControlStoreInspectionRejection,
    ControlStoreInspectionRequest, ControlStoreInspectionState, ControlStoreKind,
};
use pale_application::ApplicationFailureDisposition;
use pale_core::records::{
    ArtifactDigest, ControlIncarnationIdV1, ControlStoreIdV1, DiagnosticEffectCertaintyV1,
    DiagnosticStageV1, ResidentPrincipalIdV1, ResidentRecordRefV1, ResidentWorkspaceIdV1,
};

use crate::error::{ApiError, FailureSpecification};
use crate::PaleApi;

/// Exact control-store class selected for local physical inspection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LocalControlStoreKind {
    /// Principal-scoped trust and allocation store.
    PrincipalTrust,
    /// Immutable-deployment-bound operational control store.
    DeploymentControl,
}

/// One source-owned profile requirement that still prevents Pale Runtime assembly.
///
/// Variant order is the adapter's canonical admission/report order. Consumers
/// must preserve the order returned by
/// [`LocalControlStorageInspectionState::ProfileUnselected`] rather than sort
/// the names independently.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LocalControlStorageProfileRequirement {
    /// Exact sorted `SQLite` compile-option fingerprint.
    SqliteCompileOptions,
    /// Transaction synchronization strength and barrier interpretation.
    SqliteSynchronous,
    /// Apple full-filesystem synchronization behavior.
    SqliteFullFsync,
    /// Apple checkpoint full-filesystem synchronization behavior.
    SqliteCheckpointFullFsync,
    /// WAL checkpoint mode selected from measured evidence.
    SqliteCheckpointMode,
    /// Automatic WAL checkpoint threshold in frames.
    SqliteWalAutocheckpointFrames,
    /// Durability grouping and maximum release delay.
    SqliteDurabilityGrouping,
    /// Exact physical authority-frame encoding.
    AuthorityFrameEncoding,
    /// Corruption-checksum algorithm used by the frame.
    AuthorityCorruptionChecksum,
    /// Fixed corruption-checksum width.
    AuthorityCorruptionChecksumBytes,
    /// Hard byte ceiling for one authority-journal entry.
    AuthorityMaximumEntryBytes,
    /// Hard byte ceiling for one recovery checkpoint.
    AuthorityMaximumCheckpointBytes,
    /// Maximum checkpoint candidates returned by one recovery read.
    AuthorityMaximumCheckpointCandidates,
    /// Maximum contiguous tail entries returned by one recovery read.
    AuthorityMaximumTailEntries,
    /// Recovery-checkpoint cadence.
    AuthorityCheckpointCadence,
    /// Journal/checkpoint retention policy.
    AuthorityRetentionPolicy,
    /// Prefix-compaction policy.
    AuthorityCompactionPolicy,
    /// Verified backup and inactive-restore policy.
    BackupRestorePolicy,
}

/// Persistent physical contradiction found during read-only local inspection.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum LocalControlStorageInspectionRejection {
    /// The canonical leaf is not a private, non-symlink regular file.
    UnsafeEntryKind,
    /// `SQLite` could not decode the required initial-v1 structures.
    CorruptOrPartial,
    /// Persisted physical kind names the other control-store class.
    WrongStoreKind,
    /// Persisted logical store identity differs from the expected identity.
    WrongLogicalStoreId,
    /// Persisted control-store incarnation differs from the expected incarnation.
    WrongControlIncarnation,
    /// Persisted principal scope differs from the expected principal.
    WrongPrincipalScope,
    /// Persisted deployment scope differs from the expected immutable binding.
    WrongDeploymentScope,
    /// Persisted source or resultant-schema identity differs from embedded source.
    SourceIdentityMismatch,
    /// Physical schema is newer than the embedded source understands.
    NewerVersion {
        /// Observed physical schema version.
        observed: u32,
        /// Greatest source-owned version understood by this process.
        supported: u32,
    },
    /// Runtime `SQLite` build facts differ from the initialization observation.
    SqliteBuildMismatch,
    /// A fixed structural connection setting differs from the manifest.
    StructuralProfileMismatch,
}

/// Read-only state of one exact canonical local control store.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(tag = "kind", content = "details", rename_all = "kebab-case")]
pub enum LocalControlStorageInspectionState {
    /// The canonical descendant chain or database leaf does not exist.
    Missing,
    /// The canonical leaf is a private, zero-length regular file.
    Empty,
    /// Structural bytes match while the listed profile coordinates remain unselected.
    ProfileUnselected {
        /// Requirements in canonical source-owned admission order.
        unresolved: Vec<LocalControlStorageProfileRequirement>,
    },
    /// Physical source and profile checks match.
    ///
    /// This does not establish writer, resident, operation, or native readiness.
    Ready,
    /// Persistent bytes or entry shape fail closed for the expected identity.
    Rejected(LocalControlStorageInspectionRejection),
}

/// Owned binding-safe result of one local control-store inspection.
///
/// Serialization preserves the physical-state algebra and canonical unresolved
/// requirement order. It contains neither the caller's root nor admission
/// evidence; serializing [`LocalControlStorageInspectionState::Ready`] does not
/// confer writer or resident authority.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct LocalControlStorageInspection {
    kind: LocalControlStoreKind,
    state: LocalControlStorageInspectionState,
}

impl LocalControlStorageInspection {
    /// Physical store kind implied by the exact inspected scope.
    #[must_use]
    pub const fn kind(&self) -> LocalControlStoreKind {
        self.kind
    }

    /// Borrow the physical state; no host path or connection is retained.
    #[must_use]
    pub const fn state(&self) -> &LocalControlStorageInspectionState {
        &self.state
    }
}

/// Scope-specific input retained privately until typed adapter construction.
#[derive(Clone, Debug, Eq, PartialEq)]
enum LocalControlStoreScopeInput {
    Principal {
        principal_id: String,
    },
    Deployment {
        workspace_id: String,
        deployment_binding_ref: String,
        deployment_binding_sha256: String,
    },
}

/// Complete binding-safe request for local physical control-store inspection.
///
/// Construction performs no I/O or validation. The runtime root is redacted
/// from [`Debug`] and never appears in the successful result.
#[derive(Clone, Eq, PartialEq)]
pub struct LocalControlStorageInspectionRequest {
    runtime_state_root: String,
    scope: LocalControlStoreScopeInput,
    logical_store_id: String,
    control_incarnation_id: String,
}

impl std::fmt::Debug for LocalControlStorageInspectionRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalControlStorageInspectionRequest")
            .field("runtime_state_root", &"<redacted>")
            .field("scope", &self.scope)
            .field("logical_store_id", &self.logical_store_id)
            .field("control_incarnation_id", &self.control_incarnation_id)
            .finish()
    }
}

impl LocalControlStorageInspectionRequest {
    /// Select one principal trust store without observing the host.
    #[must_use]
    pub fn principal(
        runtime_state_root: String,
        principal_id: String,
        logical_store_id: String,
        control_incarnation_id: String,
    ) -> Self {
        Self {
            runtime_state_root,
            scope: LocalControlStoreScopeInput::Principal { principal_id },
            logical_store_id,
            control_incarnation_id,
        }
    }

    /// Select one deployment control store without observing the host.
    #[must_use]
    pub fn deployment(
        runtime_state_root: String,
        workspace_id: String,
        deployment_binding_ref: String,
        deployment_binding_sha256: String,
        logical_store_id: String,
        control_incarnation_id: String,
    ) -> Self {
        Self {
            runtime_state_root,
            scope: LocalControlStoreScopeInput::Deployment {
                workspace_id,
                deployment_binding_ref,
                deployment_binding_sha256,
            },
            logical_store_id,
            control_incarnation_id,
        }
    }
}

impl PaleApi {
    /// Inspect one canonical local control store without creating or repairing it.
    ///
    /// The native adapter verifies embedded storage source, retains the selected
    /// root, derives the canonical store location, and performs physically
    /// immutable classification without creating or updating `SQLite` sidecars.
    /// The result is deliberately narrower than a resident
    /// `status` or `doctor` response: it contains only physical store facts.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when a coordinate is invalid, the runtime root is
    /// not an absolute parent-free path, source verification fails, confinement
    /// cannot be established, the location changes, or host/`SQLite` operation
    /// prevents classification. Persistent byte contradictions are returned as
    /// [`LocalControlStorageInspectionState::Rejected`], not API failures.
    pub fn inspect_local_control_store(
        &self,
        request: LocalControlStorageInspectionRequest,
    ) -> Result<LocalControlStorageInspection, ApiError> {
        let request = adapter_request(request)
            .map_err(|error| local_control_storage_failure(&self.diagnostic_identity, error))?;
        let inspection =
            pale_adapter_control_storage::inspect_control_store(request).map_err(|error| {
                local_control_storage_failure(
                    &self.diagnostic_identity,
                    LocalControlStorageInspectionError::Adapter(error),
                )
            })?;
        Ok(LocalControlStorageInspection {
            kind: inspection.kind().into(),
            state: inspection.state().clone().into(),
        })
    }
}

fn adapter_request(
    request: LocalControlStorageInspectionRequest,
) -> Result<ControlStoreInspectionRequest, LocalControlStorageInspectionError> {
    let LocalControlStorageInspectionRequest {
        runtime_state_root,
        scope,
        logical_store_id,
        control_incarnation_id,
    } = request;
    let root = PathBuf::from(runtime_state_root);
    if !root.is_absolute()
        || root
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(LocalControlStorageInspectionError::InvalidCoordinate(
            "runtime_state_root",
        ));
    }
    let logical_store_id = ControlStoreIdV1::parse(logical_store_id)
        .map_err(|_| LocalControlStorageInspectionError::InvalidCoordinate("logical_store_id"))?;
    let control_incarnation_id =
        ControlIncarnationIdV1::parse(control_incarnation_id).map_err(|_| {
            LocalControlStorageInspectionError::InvalidCoordinate("control_incarnation_id")
        })?;
    match scope {
        LocalControlStoreScopeInput::Principal { principal_id } => {
            Ok(ControlStoreInspectionRequest::principal(
                root,
                ResidentPrincipalIdV1::parse(principal_id).map_err(|_| {
                    LocalControlStorageInspectionError::InvalidCoordinate("principal_id")
                })?,
                logical_store_id,
                control_incarnation_id,
            ))
        }
        LocalControlStoreScopeInput::Deployment {
            workspace_id,
            deployment_binding_ref,
            deployment_binding_sha256,
        } => Ok(ControlStoreInspectionRequest::deployment(
            root,
            ResidentWorkspaceIdV1::parse(workspace_id).map_err(|_| {
                LocalControlStorageInspectionError::InvalidCoordinate("workspace_id")
            })?,
            ResidentRecordRefV1::parse(deployment_binding_ref).map_err(|_| {
                LocalControlStorageInspectionError::InvalidCoordinate("deployment_binding_ref")
            })?,
            ArtifactDigest::parse(deployment_binding_sha256).map_err(|_| {
                LocalControlStorageInspectionError::InvalidCoordinate("deployment_binding_sha256")
            })?,
            logical_store_id,
            control_incarnation_id,
        )),
    }
}

#[derive(Debug, thiserror::Error)]
enum LocalControlStorageInspectionError {
    /// A caller-supplied coordinate did not satisfy its typed domain.
    #[error("invalid local control-store inspection coordinate: {0}")]
    InvalidCoordinate(&'static str),
    /// Native source, confinement, or host observation failed.
    #[error("local control-store inspection failed")]
    Adapter(#[source] pale_adapter_control_storage::ControlSqliteError),
}

fn local_control_storage_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: LocalControlStorageInspectionError,
) -> ApiError {
    let (disposition, summary) = match &error {
        LocalControlStorageInspectionError::InvalidCoordinate(_) => (
            ApplicationFailureDisposition::InvalidInput,
            "the local control-store inspection request was rejected",
        ),
        LocalControlStorageInspectionError::Adapter(
            pale_adapter_control_storage::ControlSqliteError::ManifestDecode(_)
            | pale_adapter_control_storage::ControlSqliteError::ManifestDigestMismatch
            | pale_adapter_control_storage::ControlSqliteError::SourceInvariant(_)
            | pale_adapter_control_storage::ControlSqliteError::MigrationDigestMismatch { .. }
            | pale_adapter_control_storage::ControlSqliteError::StoreLocationScopeMismatch,
        ) => (
            ApplicationFailureDisposition::Internal,
            "embedded control-storage source is incoherent",
        ),
        LocalControlStorageInspectionError::Adapter(
            pale_adapter_control_storage::ControlSqliteError::UnsafeRuntimeStateRoot(_),
        ) => (
            ApplicationFailureDisposition::InvalidInput,
            "the local runtime-state root was rejected",
        ),
        LocalControlStorageInspectionError::Adapter(
            pale_adapter_control_storage::ControlSqliteError::UnsafeControlStorageDirectory(_),
        ) => (
            ApplicationFailureDisposition::Blocked,
            "the local control-store path requires correction",
        ),
        LocalControlStorageInspectionError::Adapter(_) => (
            ApplicationFailureDisposition::Operational,
            "the local control store could not be inspected",
        ),
    };
    let (disposition, retry) = crate::error::application_diagnostic_policy(disposition);
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: "local_control_storage_inspection_failed",
            stage: DiagnosticStageV1::HostObservation,
            disposition,
            summary,
            subject: None,
            schema_coordinate: None,
            location: None,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}

impl From<ControlStoreKind> for LocalControlStoreKind {
    fn from(value: ControlStoreKind) -> Self {
        match value {
            ControlStoreKind::PrincipalTrust => Self::PrincipalTrust,
            ControlStoreKind::DeploymentControl => Self::DeploymentControl,
        }
    }
}

impl From<ControlStorageProfileRequirement> for LocalControlStorageProfileRequirement {
    fn from(value: ControlStorageProfileRequirement) -> Self {
        match value {
            ControlStorageProfileRequirement::SqliteCompileOptions => Self::SqliteCompileOptions,
            ControlStorageProfileRequirement::SqliteSynchronous => Self::SqliteSynchronous,
            ControlStorageProfileRequirement::SqliteFullFsync => Self::SqliteFullFsync,
            ControlStorageProfileRequirement::SqliteCheckpointFullFsync => {
                Self::SqliteCheckpointFullFsync
            }
            ControlStorageProfileRequirement::SqliteCheckpointMode => Self::SqliteCheckpointMode,
            ControlStorageProfileRequirement::SqliteWalAutocheckpointFrames => {
                Self::SqliteWalAutocheckpointFrames
            }
            ControlStorageProfileRequirement::SqliteDurabilityGrouping => {
                Self::SqliteDurabilityGrouping
            }
            ControlStorageProfileRequirement::AuthorityFrameEncoding => {
                Self::AuthorityFrameEncoding
            }
            ControlStorageProfileRequirement::AuthorityCorruptionChecksum => {
                Self::AuthorityCorruptionChecksum
            }
            ControlStorageProfileRequirement::AuthorityCorruptionChecksumBytes => {
                Self::AuthorityCorruptionChecksumBytes
            }
            ControlStorageProfileRequirement::AuthorityMaximumEntryBytes => {
                Self::AuthorityMaximumEntryBytes
            }
            ControlStorageProfileRequirement::AuthorityMaximumCheckpointBytes => {
                Self::AuthorityMaximumCheckpointBytes
            }
            ControlStorageProfileRequirement::AuthorityMaximumCheckpointCandidates => {
                Self::AuthorityMaximumCheckpointCandidates
            }
            ControlStorageProfileRequirement::AuthorityMaximumTailEntries => {
                Self::AuthorityMaximumTailEntries
            }
            ControlStorageProfileRequirement::AuthorityCheckpointCadence => {
                Self::AuthorityCheckpointCadence
            }
            ControlStorageProfileRequirement::AuthorityRetentionPolicy => {
                Self::AuthorityRetentionPolicy
            }
            ControlStorageProfileRequirement::AuthorityCompactionPolicy => {
                Self::AuthorityCompactionPolicy
            }
            ControlStorageProfileRequirement::BackupRestorePolicy => Self::BackupRestorePolicy,
        }
    }
}

impl From<ControlStoreInspectionState> for LocalControlStorageInspectionState {
    fn from(value: ControlStoreInspectionState) -> Self {
        match value {
            ControlStoreInspectionState::Missing => Self::Missing,
            ControlStoreInspectionState::Empty => Self::Empty,
            ControlStoreInspectionState::ProfileUnselected { unresolved } => {
                Self::ProfileUnselected {
                    unresolved: unresolved.into_iter().map(Into::into).collect(),
                }
            }
            ControlStoreInspectionState::Ready => Self::Ready,
            ControlStoreInspectionState::Rejected(reason) => Self::Rejected(reason.into()),
        }
    }
}

impl From<ControlStoreInspectionRejection> for LocalControlStorageInspectionRejection {
    fn from(value: ControlStoreInspectionRejection) -> Self {
        match value {
            ControlStoreInspectionRejection::UnsafeEntryKind => Self::UnsafeEntryKind,
            ControlStoreInspectionRejection::CorruptOrPartial => Self::CorruptOrPartial,
            ControlStoreInspectionRejection::WrongStoreKind => Self::WrongStoreKind,
            ControlStoreInspectionRejection::WrongLogicalStoreId => Self::WrongLogicalStoreId,
            ControlStoreInspectionRejection::WrongControlIncarnation => {
                Self::WrongControlIncarnation
            }
            ControlStoreInspectionRejection::WrongPrincipalScope => Self::WrongPrincipalScope,
            ControlStoreInspectionRejection::WrongDeploymentScope => Self::WrongDeploymentScope,
            ControlStoreInspectionRejection::SourceIdentityMismatch => Self::SourceIdentityMismatch,
            ControlStoreInspectionRejection::NewerVersion {
                observed,
                supported,
            } => Self::NewerVersion {
                observed,
                supported,
            },
            ControlStoreInspectionRejection::SqliteBuildMismatch => Self::SqliteBuildMismatch,
            ControlStoreInspectionRejection::StructuralProfileMismatch => {
                Self::StructuralProfileMismatch
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/internal/local_control_storage.rs"]
mod tests;
