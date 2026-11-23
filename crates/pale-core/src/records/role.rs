//! Specialist roles and their model-facing doctrine.
//!
//! A [`SpecialistRole`] is a delegated persona whose nuanced expertise lives in
//! Markdown doctrine. Roles carry no mechanical grants; the invoking operation
//! owns every capability and approval contract.

use serde::{Deserialize, Serialize};

use crate::paths::OwnedConfinedRelativePath;
use crate::records::{PackageResourceIdV1, PackageResourceLinkV1};

/// One delegated specialist, distinct from a repository's architectural role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpecialistRole {
    /// Stable specialist-role identity.
    pub id: PackageResourceIdV1,
    /// Compact delegation purpose shown during role discovery.
    pub description: String,
    /// Relative path (under the package) to the Markdown doctrine body carrying
    /// the specialist's nuanced expertise. The generated surface presents the
    /// persona; the model applies the doctrine.
    pub doctrine: OwnedConfinedRelativePath,
    /// Shared specialist archetypes composed before this role's own doctrine.
    #[serde(default)]
    pub extends: Vec<PackageResourceLinkV1>,
    /// Other specialist roles whose judgment lens is composed dependency-first
    /// into this role. This is inheritance, not a request to spawn another agent.
    #[serde(default)]
    pub inherits_lenses: Vec<PackageResourceLinkV1>,
    /// Additional doctrine ids required by the role.
    #[serde(default)]
    pub doctrine_refs: Vec<PackageResourceLinkV1>,
    /// The stack/domain scope this role applies to (e.g. `rust`); empty = general (any stack). The
    /// distinct dimension a role carries that an operation does not.
    #[serde(default)]
    pub applies_to: Vec<PackageResourceLinkV1>,
}

/// A reusable specialist-role persona and workflow body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleArchetype {
    /// Stable reusable role-archetype identity.
    pub id: PackageResourceIdV1,
    /// Compact purpose of the shared persona and workflow.
    pub description: String,
    /// Package-relative Markdown body for the archetype.
    pub doctrine: OwnedConfinedRelativePath,
    /// Archetype identities composed before this one.
    #[serde(default)]
    pub extends: Vec<PackageResourceLinkV1>,
    /// Additional doctrine identities required by the archetype.
    #[serde(default)]
    pub doctrine_refs: Vec<PackageResourceLinkV1>,
}

/// The distribution's specialist-role catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleCatalog {
    /// Version of the specialist-role declaration contract.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    #[serde(rename = "archetype", default)]
    /// Reusable role archetypes in authored order.
    pub archetypes: Vec<RoleArchetype>,
    #[serde(rename = "role")]
    /// Concrete specialist roles in authored order.
    pub roles: Vec<SpecialistRole>,
}

impl RoleCatalog {
    /// A role by id, if known.
    pub fn get(&self, id: &PackageResourceIdV1) -> Option<&SpecialistRole> {
        self.roles.iter().find(|role| &role.id == id)
    }
}

/// `roles.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RolesFile {
    /// The single specialist-role catalog in this file.
    pub roles: RoleCatalog,
}
