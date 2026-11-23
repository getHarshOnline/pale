//! Explicit source roles and compiler-owned declaration coordinates.

use pale_application::workspace as application;

/// Explicit standalone grammar role selected by a binding client.
///
/// These roles have one stable Pale contract independent of a selected
/// project. The host still supplies the role explicitly; filenames and source
/// text never select a grammar. An admitted compilation remains authoritative
/// when a document's role depends on manifest-owned Package custody.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandaloneAuthoringRole {
    /// Root portable workspace manifest.
    WorkspaceManifest,
    /// Reusable workspace module.
    WorkspaceModule,
    /// Workspace-language primitive declarations.
    WorkspaceLanguagePrimitives,
    /// Responsibility derivation declarations.
    ResponsibilityDerivation,
    /// Organization-to-responsibility translation declarations.
    OrganizationalTranslation,
    /// Experience-composition declarations.
    ExperienceComposition,
    /// Exact module-resolution lock.
    WorkspaceResolutionLock,
    /// Exact workspace-universe lock.
    WorkspaceUniverseLock,
    /// Private capability policy, recipes, provider bindings, and operation bindings.
    Capabilities,
    /// Private machine realization and authority-root bindings.
    MachineBinding,
    /// Exact admitted distribution identities.
    DistributionLock,
    /// Package manifest and resource inventory.
    PackageManifest,
    /// A Package page with a TOML header and a Markdown body.
    PackagePage,
    /// A typed Package data document.
    PackageData,
    /// Package doctrine declarations.
    PackageDoctrine,
    /// Package operation declarations.
    PackageOperations,
    /// Package role declarations.
    PackageRoles,
    /// Package stack declarations.
    PackageStacks,
    /// Package run-mode declarations.
    PackageRunModes,
    /// Package policy-family declaration.
    PackagePolicyFamily,
    /// Package governing-order declaration.
    PackageGoverningOrder,
    /// Package capability-policy declaration.
    PackageCapabilityPolicy,
    /// Package tool-recipe declaration.
    PackageToolRecipe,
}

impl From<StandaloneAuthoringRole> for application::AuthoringSourceRole {
    fn from(role: StandaloneAuthoringRole) -> Self {
        match role {
            StandaloneAuthoringRole::WorkspaceManifest => Self::WorkspaceManifest,
            StandaloneAuthoringRole::WorkspaceModule => Self::WorkspaceModule,
            StandaloneAuthoringRole::WorkspaceLanguagePrimitives => {
                Self::WorkspaceLanguagePrimitives
            }
            StandaloneAuthoringRole::ResponsibilityDerivation => Self::ResponsibilityDerivation,
            StandaloneAuthoringRole::OrganizationalTranslation => Self::OrganizationalTranslation,
            StandaloneAuthoringRole::ExperienceComposition => Self::ExperienceComposition,
            StandaloneAuthoringRole::WorkspaceResolutionLock => Self::WorkspaceResolutionLock,
            StandaloneAuthoringRole::WorkspaceUniverseLock => Self::WorkspaceUniverseLock,
            StandaloneAuthoringRole::Capabilities => Self::Capabilities,
            StandaloneAuthoringRole::MachineBinding => Self::MachineBinding,
            StandaloneAuthoringRole::DistributionLock => Self::DistributionLock,
            StandaloneAuthoringRole::PackageManifest => Self::PackageManifest,
            StandaloneAuthoringRole::PackagePage => Self::PackagePage,
            StandaloneAuthoringRole::PackageData => Self::PackageData,
            StandaloneAuthoringRole::PackageDoctrine => Self::PackageDoctrine,
            StandaloneAuthoringRole::PackageOperations => Self::PackageOperations,
            StandaloneAuthoringRole::PackageRoles => Self::PackageRoles,
            StandaloneAuthoringRole::PackageStacks => Self::PackageStacks,
            StandaloneAuthoringRole::PackageRunModes => Self::PackageRunModes,
            StandaloneAuthoringRole::PackagePolicyFamily => Self::PackagePolicyFamily,
            StandaloneAuthoringRole::PackageGoverningOrder => Self::PackageGoverningOrder,
            StandaloneAuthoringRole::PackageCapabilityPolicy => Self::PackageCapabilityPolicy,
            StandaloneAuthoringRole::PackageToolRecipe => Self::PackageToolRecipe,
        }
    }
}

impl StandaloneAuthoringRole {
    /// Locate the declaration for tolerant editor help in the original source.
    ///
    /// An unfinished page header remains inspectable within the compiler's
    /// header bound. Missing opening delimiters or oversized headers return
    /// no region. This neither parses the declaration nor admits its meaning.
    #[must_use]
    pub fn declaration_range(self, source: &str) -> Option<std::ops::Range<usize>> {
        application::AuthoringSourceRole::from(self).declaration_range(source)
    }
}
