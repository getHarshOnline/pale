//! Read-only classification of one canonical physical control-store location.
//!
//! Inspection verifies the embedded storage source, derives the same private
//! location used by store creation, and opens an existing database through
//! `SQLite`'s physically immutable reader.
//! It returns physical classification only: no state grants writer authority,
//! selects the unresolved durability profile, or admits a resident service.

use std::path::PathBuf;

use pale_core::records::{
    ArtifactDigest, ControlIncarnationIdV1, ControlStoreIdV1, ResidentPrincipalIdV1,
    ResidentRecordRefV1, ResidentWorkspaceIdV1,
};

use crate::location::{
    ControlStorageRoot, ControlStoreScope, DeploymentStoreScope, PrincipalStoreScope,
};
use crate::store::{
    inspect_store_with_sources, ControlStoreIdentity, StoreOpenClassification, StoreOpenRejection,
};
use crate::{ControlSqliteError, ControlStorageProfileRequirement, ControlStoreKind};

/// Scope-specific coordinates needed to derive one canonical store location.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ControlStoreInspectionScope {
    /// Principal-scoped trust-store coordinates.
    Principal(ResidentPrincipalIdV1),
    /// Immutable deployment-control-store coordinates.
    Deployment {
        /// Workspace bound to the deployment store.
        workspace_id: ResidentWorkspaceIdV1,
        /// Immutable deployment-binding record reference.
        deployment_binding_ref: ResidentRecordRefV1,
        /// Exact deployment-binding record digest.
        deployment_binding_sha256: ArtifactDigest,
    },
}

/// Complete read-only locator and expected identity for one control store.
///
/// The runtime root is caller-selected host input and is intentionally absent
/// from [`Debug`]. Construction performs no I/O; [`inspect_control_store`]
/// retains the root and resolves descendants without creating them.
#[derive(Clone, PartialEq, Eq)]
pub struct ControlStoreInspectionRequest {
    runtime_state_root: PathBuf,
    scope: ControlStoreInspectionScope,
    logical_store_id: ControlStoreIdV1,
    control_incarnation_id: ControlIncarnationIdV1,
}

impl std::fmt::Debug for ControlStoreInspectionRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ControlStoreInspectionRequest")
            .field("runtime_state_root", &"<redacted>")
            .field("scope", &self.scope)
            .field("logical_store_id", &self.logical_store_id)
            .field("control_incarnation_id", &self.control_incarnation_id)
            .finish()
    }
}

impl ControlStoreInspectionRequest {
    /// Select one principal trust store without observing the filesystem.
    #[must_use]
    pub fn principal(
        runtime_state_root: PathBuf,
        principal_id: ResidentPrincipalIdV1,
        logical_store_id: ControlStoreIdV1,
        control_incarnation_id: ControlIncarnationIdV1,
    ) -> Self {
        Self {
            runtime_state_root,
            scope: ControlStoreInspectionScope::Principal(principal_id),
            logical_store_id,
            control_incarnation_id,
        }
    }

    /// Select one deployment control store without observing the filesystem.
    #[must_use]
    pub fn deployment(
        runtime_state_root: PathBuf,
        workspace_id: ResidentWorkspaceIdV1,
        deployment_binding_ref: ResidentRecordRefV1,
        deployment_binding_sha256: ArtifactDigest,
        logical_store_id: ControlStoreIdV1,
        control_incarnation_id: ControlIncarnationIdV1,
    ) -> Self {
        Self {
            runtime_state_root,
            scope: ControlStoreInspectionScope::Deployment {
                workspace_id,
                deployment_binding_ref,
                deployment_binding_sha256,
            },
            logical_store_id,
            control_incarnation_id,
        }
    }
}

/// Physical classification of one exact expected control store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlStoreInspection {
    kind: ControlStoreKind,
    state: ControlStoreInspectionState,
}

impl ControlStoreInspection {
    /// Physical store kind implied by the inspected typed scope.
    #[must_use]
    pub const fn kind(&self) -> ControlStoreKind {
        self.kind
    }

    /// Read-only physical state observed at the canonical location.
    #[must_use]
    pub const fn state(&self) -> &ControlStoreInspectionState {
        &self.state
    }
}

/// Closed physical state returned by control-store inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlStoreInspectionState {
    /// The canonical descendant chain or database leaf does not exist.
    Missing,
    /// The canonical leaf is a private, zero-length regular file.
    Empty,
    /// Structural bytes match, but runtime profile requirements remain unselected.
    ProfileUnselected {
        /// Requirements in their canonical source-owned admission order.
        unresolved: Vec<ControlStorageProfileRequirement>,
    },
    /// Structural bytes and the complete source-owned profile match.
    ///
    /// This is physical open classification only, not writer, resident, or
    /// operation readiness.
    Ready,
    /// Persistent bytes or entry shape fail closed for the exact expected identity.
    Rejected(ControlStoreInspectionRejection),
}

/// Persistent physical contradiction found during read-only inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlStoreInspectionRejection {
    /// The canonical leaf is not a private, non-symlink regular file.
    UnsafeEntryKind,
    /// `SQLite` could not decode the required initial-v1 structures.
    CorruptOrPartial,
    /// The database application ID or persisted kind names the other store class.
    WrongStoreKind,
    /// Persisted logical store identity differs from the caller's expectation.
    WrongLogicalStoreId,
    /// Persisted control-store incarnation differs from the caller's expectation.
    WrongControlIncarnation,
    /// Persisted raw principal scope differs from the expected principal.
    WrongPrincipalScope,
    /// Persisted deployment scope differs from the expected immutable binding.
    WrongDeploymentScope,
    /// Persisted schema, manifest, migration, or resultant-schema identity differs.
    SourceIdentityMismatch,
    /// The store schema is newer than this binary's embedded source.
    NewerVersion {
        /// Observed physical schema version.
        observed: u32,
        /// Greatest source-owned version understood by this binary.
        supported: u32,
    },
    /// Runtime `SQLite` build facts differ from the initialization observation.
    SqliteBuildMismatch,
    /// A fixed structural connection setting differs from the manifest.
    StructuralProfileMismatch,
}

/// Inspect one canonical control-store location without creating or repairing it.
///
/// The embedded manifest and migration bytes are verified before filesystem
/// classification. Missing canonical descendants collapse only to
/// [`ControlStoreInspectionState::Missing`]; existing unsafe descendants remain
/// errors because no trustworthy canonical store location can be established.
/// The immutable `SQLite` reader neither creates nor updates WAL/shared-memory
/// sidecars. A successful result retains no open database connection and is not
/// a concurrency snapshot or resident-readiness claim.
///
/// # Errors
///
/// Returns [`ControlSqliteError`] when embedded source is incoherent, the root
/// or an existing descendant violates confinement, the location changes during
/// inspection, or an operational host/`SQLite` failure prevents classification.
pub fn inspect_control_store(
    request: ControlStoreInspectionRequest,
) -> Result<ControlStoreInspection, ControlSqliteError> {
    let ControlStoreInspectionRequest {
        runtime_state_root,
        scope,
        logical_store_id,
        control_incarnation_id,
    } = request;
    let sources = crate::verify_embedded_storage_sources()?;
    let root = ControlStorageRoot::retain(&runtime_state_root)?;
    let scope = match scope {
        ControlStoreInspectionScope::Principal(principal_id) => {
            ControlStoreScope::Principal(PrincipalStoreScope::new(principal_id))
        }
        ControlStoreInspectionScope::Deployment {
            workspace_id,
            deployment_binding_ref,
            deployment_binding_sha256,
        } => ControlStoreScope::Deployment(DeploymentStoreScope::new(
            workspace_id,
            deployment_binding_ref,
            deployment_binding_sha256,
        )),
    };
    let kind = scope.kind();
    let Some(location) = root.lookup_store_location(scope.clone())? else {
        return Ok(ControlStoreInspection {
            kind,
            state: ControlStoreInspectionState::Missing,
        });
    };
    let identity = ControlStoreIdentity::new(scope, logical_store_id, control_incarnation_id);
    let state = inspect_store_with_sources(&location, &identity, &sources)?.into();
    Ok(ControlStoreInspection { kind, state })
}

impl From<StoreOpenClassification> for ControlStoreInspectionState {
    fn from(value: StoreOpenClassification) -> Self {
        match value {
            StoreOpenClassification::Missing => Self::Missing,
            StoreOpenClassification::Empty => Self::Empty,
            StoreOpenClassification::ProfileUnselected { unresolved } => {
                Self::ProfileUnselected { unresolved }
            }
            StoreOpenClassification::Ready => Self::Ready,
            StoreOpenClassification::Rejected(reason) => Self::Rejected(reason.into()),
        }
    }
}

impl From<StoreOpenRejection> for ControlStoreInspectionRejection {
    fn from(value: StoreOpenRejection) -> Self {
        match value {
            StoreOpenRejection::UnsafeEntryKind => Self::UnsafeEntryKind,
            StoreOpenRejection::CorruptOrPartial => Self::CorruptOrPartial,
            StoreOpenRejection::WrongStoreKind => Self::WrongStoreKind,
            StoreOpenRejection::WrongLogicalStoreId => Self::WrongLogicalStoreId,
            StoreOpenRejection::WrongControlIncarnation => Self::WrongControlIncarnation,
            StoreOpenRejection::WrongPrincipalScope => Self::WrongPrincipalScope,
            StoreOpenRejection::WrongDeploymentScope => Self::WrongDeploymentScope,
            StoreOpenRejection::SourceIdentityMismatch => Self::SourceIdentityMismatch,
            StoreOpenRejection::NewerVersion {
                observed,
                supported,
            } => Self::NewerVersion {
                observed,
                supported,
            },
            StoreOpenRejection::SqliteBuildMismatch => Self::SqliteBuildMismatch,
            StoreOpenRejection::StructuralProfileMismatch => Self::StructuralProfileMismatch,
        }
    }
}

#[cfg(test)]
#[path = "../tests/internal/inspection.rs"]
mod tests;
