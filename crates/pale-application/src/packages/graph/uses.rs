//! Selection identities for the distinct uses of one authored Package page.

use super::*;

impl PackageResource {
    /// Required graph nodes, retaining the kind of every relationship.
    pub(crate) fn required_uses(&self) -> Vec<PackageResourceUse> {
        use PackageResourceKindV1 as Kind;

        let mut uses = Vec::new();
        let mut add = |references: &[PackageResourceRefV1], kind| {
            uses.extend(
                references
                    .iter()
                    .cloned()
                    .map(|resource| PackageResourceUse::new(resource, kind)),
            );
        };
        match self {
            Self::Doctrine {
                extends, requires, ..
            } => {
                add(extends, Kind::Doctrine);
                add(requires, Kind::Doctrine);
            }
            Self::OperationArchetype {
                extends, doctrine, ..
            } => {
                add(extends, Kind::OperationArchetype);
                add(doctrine, Kind::Doctrine);
            }
            Self::RoleArchetype {
                extends, doctrine, ..
            } => {
                add(extends, Kind::RoleArchetype);
                add(doctrine, Kind::Doctrine);
            }
            Self::Operation {
                extends,
                doctrine,
                roles,
                requirements,
                capability_policy,
                ..
            } => {
                add(extends, Kind::OperationArchetype);
                add(doctrine, Kind::Doctrine);
                add(roles, Kind::Role);
                for requirement in requirements {
                    for recipe in requirement
                        .recipe
                        .iter()
                        .chain(requirement_policy_recipe(&requirement.policy))
                    {
                        add(std::slice::from_ref(recipe), Kind::ToolRecipe);
                    }
                }
                if let Some(policy) = capability_policy {
                    add(std::slice::from_ref(policy), Kind::CapabilityPolicy);
                }
            }
            Self::Role {
                extends,
                inherited_lenses,
                doctrine,
                stacks,
                ..
            } => {
                add(extends, Kind::RoleArchetype);
                add(inherited_lenses, Kind::Role);
                add(doctrine, Kind::Doctrine);
                add(stacks, Kind::Stack);
            }
            Self::Stack {
                extends,
                doctrine,
                role_doctrine,
                ..
            } => {
                add(extends, Kind::Stack);
                add(doctrine, Kind::Doctrine);
                add(role_doctrine, Kind::Doctrine);
            }
            Self::RunMode { extends, .. } => add(extends, Kind::RunMode),
            Self::PolicyFamily { .. }
            | Self::GoverningOrder
            | Self::CapabilityPolicy
            | Self::ToolRecipe => {}
        }
        uses
    }
}
