//! Copied causal selectors carried by profile entries, not admitted handles.

use napi_derive::napi;
use pale_sdk::PortableWorkspaceSubjectSelector as Selector;

/// Complete copied selector, narrowed by its `kind` in JavaScript.
///
/// Each variant contains only its required coordinates. Reading or changing
/// this output value performs no admission. It is not a `WorkspaceSubject`
/// accepted by comparison queries.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum WorkspaceSubjectValue {
    /// One retained logical source.
    Source {
        /// Receipt-local source identity.
        source_id: String,
    },
    /// One semantic source component.
    Component {
        /// Component identity.
        component_id: String,
    },
    /// One selected module.
    Module {
        /// Module identity.
        module_id: String,
    },
    /// One canonically owned declaration.
    Declaration {
        /// Canonical owner module.
        owner_module_id: String,
        /// Declaration namespace.
        namespace_id: String,
        /// Identity local to the namespace.
        declaration_id: String,
    },
    /// One resolved import path.
    Import {
        /// Importing module.
        importer_module_id: String,
        /// Selected upstream module.
        upstream_module_id: String,
        /// Selected upstream release.
        upstream_release_id: String,
        /// Named upstream export.
        export_id: String,
        /// Namespace selected from the upstream export.
        source_namespace_id: String,
        /// Namespace bound by the importer.
        local_namespace_id: String,
    },
    /// One applied extension.
    Extension {
        /// Extension owner module.
        owner_module_id: String,
        /// Namespace owning the extension rule.
        extension_namespace_id: String,
        /// Namespace of the extended declaration.
        target_namespace_id: String,
        /// Identity of the extended declaration.
        target_declaration_id: String,
    },
    /// Effective policy of one declaration.
    Policy {
        /// Canonical owner module.
        owner_module_id: String,
        /// Declaration namespace.
        namespace_id: String,
        /// Identity local to the namespace.
        declaration_id: String,
    },
    /// Selects the complete responsibility closure.
    ResponsibilityClosure,
    /// Selects the complete optional organizational translation closure.
    OrganizationalTranslationClosure,
    /// Selects the complete explicit experience composition.
    ExperienceComposition,
    /// One logical or release-bound resource.
    Resource {
        /// Selecting module.
        owner_module_id: String,
        /// Workspace-local resource identity.
        resource_id: String,
    },
    /// Selects the exact resolution lock.
    ResolutionLock,
    /// Selects the exact universe lock.
    UniverseLock,
    /// Selects the complete workspace by its identity, without copying its IR.
    Workspace {
        /// Principal-relative workspace identity.
        workspace_id: String,
    },
}

impl WorkspaceSubjectValue {
    pub(crate) fn from_api(value: &Selector) -> Self {
        match value {
            Selector::Source { source_id } => Self::Source {
                source_id: source_id.clone(),
            },
            Selector::Component { component_id } => Self::Component {
                component_id: component_id.clone(),
            },
            Selector::Module { module_id } => Self::Module {
                module_id: module_id.clone(),
            },
            Selector::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Declaration {
                owner_module_id: owner_module_id.clone(),
                namespace_id: namespace_id.clone(),
                declaration_id: declaration_id.clone(),
            },
            Selector::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            } => Self::Import {
                importer_module_id: importer_module_id.clone(),
                upstream_module_id: upstream_module_id.clone(),
                upstream_release_id: upstream_release_id.clone(),
                export_id: export_id.clone(),
                source_namespace_id: source_namespace_id.clone(),
                local_namespace_id: local_namespace_id.clone(),
            },
            Selector::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            } => Self::Extension {
                owner_module_id: owner_module_id.clone(),
                extension_namespace_id: extension_namespace_id.clone(),
                target_namespace_id: target_namespace_id.clone(),
                target_declaration_id: target_declaration_id.clone(),
            },
            Selector::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Policy {
                owner_module_id: owner_module_id.clone(),
                namespace_id: namespace_id.clone(),
                declaration_id: declaration_id.clone(),
            },
            Selector::ResponsibilityClosure => Self::ResponsibilityClosure,
            Selector::OrganizationalTranslationClosure => Self::OrganizationalTranslationClosure,
            Selector::ExperienceComposition => Self::ExperienceComposition,
            Selector::Resource {
                owner_module_id,
                resource_id,
            } => Self::Resource {
                owner_module_id: owner_module_id.clone(),
                resource_id: resource_id.clone(),
            },
            Selector::ResolutionLock => Self::ResolutionLock,
            Selector::UniverseLock => Self::UniverseLock,
            Selector::Workspace { workspace_id } => Self::Workspace {
                workspace_id: workspace_id.clone(),
            },
        }
    }
}
