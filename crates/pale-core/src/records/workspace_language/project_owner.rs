//! Explicit project ownership through a module's selected principal or organization.

use serde::{Deserialize, Serialize};

use super::{WorkspaceModuleId, WorkspaceResourceId, WorkspaceResourceKindV1};

/// The declared owner of a project, qualified by the module that selects it.
///
/// Linking requires an adopted module resource of the stated kind. The
/// reference does not appoint a duty provider, establish acceptance, or confer
/// native permissions. A person need not be represented as an organization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectOwnerReferenceV1 {
    /// A principal owns the project in their own capacity.
    Principal {
        /// Module selecting the principal resource.
        module_id: WorkspaceModuleId,
        /// Exact principal resource in that module.
        resource_id: WorkspaceResourceId,
    },
    /// An independently constituted organization owns the project.
    Organization {
        /// Module selecting the organization resource.
        module_id: WorkspaceModuleId,
        /// Exact organization resource in that module.
        resource_id: WorkspaceResourceId,
    },
}

impl ProjectOwnerReferenceV1 {
    /// Borrow the module that selects the owner resource.
    #[must_use]
    pub fn module_id(&self) -> &WorkspaceModuleId {
        match self {
            Self::Principal { module_id, .. } | Self::Organization { module_id, .. } => module_id,
        }
    }

    /// Borrow the resource identity within the selected module.
    #[must_use]
    pub fn resource_id(&self) -> &WorkspaceResourceId {
        match self {
            Self::Principal { resource_id, .. } | Self::Organization { resource_id, .. } => {
                resource_id
            }
        }
    }

    /// Return the resource kind required by this ownership declaration.
    #[must_use]
    pub const fn kind(&self) -> WorkspaceResourceKindV1 {
        match self {
            Self::Principal { .. } => WorkspaceResourceKindV1::Principal,
            Self::Organization { .. } => WorkspaceResourceKindV1::Organization,
        }
    }
}
