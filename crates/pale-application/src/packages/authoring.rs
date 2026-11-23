//! Shared Package declaration-grammar selection.
//!
//! Older declaration files take their resource kinds from the explicit manifest.
//! Semantic parsing and editor projection both consume this classifier. Header
//! pages and typed data declare their own uses through the shared source reader.

use std::collections::BTreeSet;

use pale_core::records::{PackageResourceKindV1, PackageResourceV1};

use crate::workspace::AuthoringTomlGrammar;

/// Exact semantic grammar selected for one Package declaration source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PackageDeclarationGrammar {
    Doctrine,
    Operations,
    Roles,
    Stacks,
    RunModes,
    PolicyFamily,
    GoverningOrder,
    CapabilityPolicy,
    ToolRecipe,
}

impl PackageDeclarationGrammar {
    /// Return the matching editor schema without changing contract identity.
    pub(super) const fn authoring(self) -> AuthoringTomlGrammar {
        match self {
            Self::Doctrine => AuthoringTomlGrammar::record("doctrine-entry.v1"),
            Self::Operations => AuthoringTomlGrammar::record("operation.v1"),
            Self::Roles => AuthoringTomlGrammar::record("role.v1"),
            Self::Stacks => AuthoringTomlGrammar::record("stack.v1"),
            Self::RunModes => AuthoringTomlGrammar::record("run-mode.v1"),
            Self::PolicyFamily => AuthoringTomlGrammar::record("policy-family.v1"),
            Self::GoverningOrder => AuthoringTomlGrammar::record("governing-order.v1"),
            Self::CapabilityPolicy => {
                AuthoringTomlGrammar::definition("capability-procedure.v1", "CapabilityPolicy")
            }
            Self::ToolRecipe => {
                AuthoringTomlGrammar::definition("capability-procedure.v1", "ToolRecipe")
            }
        }
    }
}

/// Select a declaration grammar from one manifest-owned resource group.
///
/// `None` means the manifest combines resource kinds that no registered source
/// grammar can represent. The caller retains the source/package coordinates
/// needed to construct the owning diagnostic.
pub(super) fn declaration_grammar(
    declarations: &[PackageResourceV1],
) -> Option<PackageDeclarationGrammar> {
    use PackageDeclarationGrammar as Grammar;
    use PackageResourceKindV1 as Kind;

    let kinds = declarations
        .iter()
        .map(|resource| resource.kind)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    match kinds.as_slice() {
        [Kind::Doctrine] => Some(Grammar::Doctrine),
        [Kind::Operation]
        | [Kind::OperationArchetype]
        | [Kind::OperationArchetype, Kind::Operation] => Some(Grammar::Operations),
        [Kind::Role] | [Kind::RoleArchetype] | [Kind::RoleArchetype, Kind::Role] => {
            Some(Grammar::Roles)
        }
        [Kind::Stack] => Some(Grammar::Stacks),
        [Kind::RunMode] => Some(Grammar::RunModes),
        [Kind::PolicyFamily] if declarations.len() == 1 => Some(Grammar::PolicyFamily),
        [Kind::GoverningOrder] if declarations.len() == 1 => Some(Grammar::GoverningOrder),
        [Kind::CapabilityPolicy] if declarations.len() == 1 => Some(Grammar::CapabilityPolicy),
        [Kind::ToolRecipe] if declarations.len() == 1 => Some(Grammar::ToolRecipe),
        _ => None,
    }
}
