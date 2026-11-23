//! Copied workspace selectors carried by effective-profile rows.
//!
//! Concrete variants expose only their own coordinates. They are descriptive
//! values, not admitted [`crate::PyWorkspaceSubject`] handles.

use pale_sdk::PortableWorkspaceSubjectSelector;
use pyo3::prelude::*;

/// Selects one retained logical source document.
#[pyclass(name = "WorkspaceSourceSubjectValue", frozen, get_all)]
pub struct PyWorkspaceSourceSubjectValue {
    /// Receipt-local source identity.
    source_id: String,
}

/// Selects one admitted semantic source component.
#[pyclass(name = "WorkspaceComponentSubjectValue", frozen, get_all)]
pub struct PyWorkspaceComponentSubjectValue {
    /// Component identity.
    component_id: String,
}

/// Selects one module in the admitted release closure.
#[pyclass(name = "WorkspaceModuleSubjectValue", frozen, get_all)]
pub struct PyWorkspaceModuleSubjectValue {
    /// Module identity.
    module_id: String,
}

/// Selects one canonically owned semantic declaration.
#[pyclass(name = "WorkspaceDeclarationSubjectValue", frozen, get_all)]
pub struct PyWorkspaceDeclarationSubjectValue {
    /// Canonical owner module.
    owner_module_id: String,
    /// Namespace containing the declaration.
    namespace_id: String,
    /// Identity local to the namespace.
    declaration_id: String,
}

/// Selects one resolved import path.
#[pyclass(name = "WorkspaceImportSubjectValue", frozen, get_all)]
pub struct PyWorkspaceImportSubjectValue {
    /// Module declaring the import.
    importer_module_id: String,
    /// Selected upstream module.
    upstream_module_id: String,
    /// Selected upstream release.
    upstream_release_id: String,
    /// Named upstream export.
    export_id: String,
    /// Namespace selected from the export.
    source_namespace_id: String,
    /// Namespace bound in the importing module.
    local_namespace_id: String,
}

/// Selects one applied extension.
#[pyclass(name = "WorkspaceExtensionSubjectValue", frozen, get_all)]
pub struct PyWorkspaceExtensionSubjectValue {
    /// Module owning the extension.
    owner_module_id: String,
    /// Namespace owning the extension rule.
    extension_namespace_id: String,
    /// Namespace of the extended declaration.
    target_namespace_id: String,
    /// Identity of the extended declaration.
    target_declaration_id: String,
}

/// Selects the effective policy of one declaration.
#[pyclass(name = "WorkspacePolicySubjectValue", frozen, get_all)]
pub struct PyWorkspacePolicySubjectValue {
    /// Canonical owner module.
    owner_module_id: String,
    /// Namespace containing the declaration.
    namespace_id: String,
    /// Declaration identity local to the namespace.
    declaration_id: String,
}

/// Selects the complete responsibility closure.
#[pyclass(name = "WorkspaceResponsibilityClosureSubjectValue", frozen)]
pub struct PyWorkspaceResponsibilityClosureSubjectValue {}

/// Selects the complete optional organizational-translation closure.
#[pyclass(name = "WorkspaceOrganizationalTranslationClosureSubjectValue", frozen)]
pub struct PyWorkspaceOrganizationalTranslationClosureSubjectValue {}

/// Selects the complete explicit experience-composition closure.
#[pyclass(name = "WorkspaceExperienceCompositionSubjectValue", frozen)]
pub struct PyWorkspaceExperienceCompositionSubjectValue {}

/// Selects one logical or release-bound resource.
#[pyclass(name = "WorkspaceResourceSubjectValue", frozen, get_all)]
pub struct PyWorkspaceResourceSubjectValue {
    /// Module selecting the resource.
    owner_module_id: String,
    /// Workspace-local resource identity.
    resource_id: String,
}

/// Selects the exact resolution lock.
#[pyclass(name = "WorkspaceResolutionLockSubjectValue", frozen)]
pub struct PyWorkspaceResolutionLockSubjectValue {}

/// Selects the exact universe lock.
#[pyclass(name = "WorkspaceUniverseLockSubjectValue", frozen)]
pub struct PyWorkspaceUniverseLockSubjectValue {}

/// Selects the complete workspace identified by the SDK.
#[pyclass(name = "WholeWorkspaceSubjectValue", frozen, get_all)]
pub struct PyWholeWorkspaceSubjectValue {
    /// Principal-relative workspace identity.
    workspace_id: String,
}

// Transparent conversion produces the concrete Python class and its generated
// union annotation. It neither admits selectors nor creates opaque handles.
#[derive(IntoPyObject)]
pub(crate) enum PyWorkspaceSubjectValue {
    Source(PyWorkspaceSourceSubjectValue),
    Component(PyWorkspaceComponentSubjectValue),
    Module(PyWorkspaceModuleSubjectValue),
    Declaration(PyWorkspaceDeclarationSubjectValue),
    Import(PyWorkspaceImportSubjectValue),
    Extension(PyWorkspaceExtensionSubjectValue),
    Policy(PyWorkspacePolicySubjectValue),
    ResponsibilityClosure(PyWorkspaceResponsibilityClosureSubjectValue),
    OrganizationalTranslationClosure(PyWorkspaceOrganizationalTranslationClosureSubjectValue),
    ExperienceComposition(PyWorkspaceExperienceCompositionSubjectValue),
    Resource(PyWorkspaceResourceSubjectValue),
    ResolutionLock(PyWorkspaceResolutionLockSubjectValue),
    UniverseLock(PyWorkspaceUniverseLockSubjectValue),
    Workspace(PyWholeWorkspaceSubjectValue),
}

impl PyWorkspaceSubjectValue {
    pub(crate) fn from_api(value: PortableWorkspaceSubjectSelector) -> Self {
        match value {
            PortableWorkspaceSubjectSelector::Source { source_id } => {
                Self::Source(PyWorkspaceSourceSubjectValue { source_id })
            }
            PortableWorkspaceSubjectSelector::Component { component_id } => {
                Self::Component(PyWorkspaceComponentSubjectValue { component_id })
            }
            PortableWorkspaceSubjectSelector::Module { module_id } => {
                Self::Module(PyWorkspaceModuleSubjectValue { module_id })
            }
            PortableWorkspaceSubjectSelector::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Declaration(PyWorkspaceDeclarationSubjectValue {
                owner_module_id,
                namespace_id,
                declaration_id,
            }),
            PortableWorkspaceSubjectSelector::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            } => Self::Import(PyWorkspaceImportSubjectValue {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            }),
            PortableWorkspaceSubjectSelector::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            } => Self::Extension(PyWorkspaceExtensionSubjectValue {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            }),
            PortableWorkspaceSubjectSelector::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Policy(PyWorkspacePolicySubjectValue {
                owner_module_id,
                namespace_id,
                declaration_id,
            }),
            PortableWorkspaceSubjectSelector::ResponsibilityClosure => {
                Self::ResponsibilityClosure(PyWorkspaceResponsibilityClosureSubjectValue {})
            }
            PortableWorkspaceSubjectSelector::OrganizationalTranslationClosure => {
                Self::OrganizationalTranslationClosure(
                    PyWorkspaceOrganizationalTranslationClosureSubjectValue {},
                )
            }
            PortableWorkspaceSubjectSelector::ExperienceComposition => {
                Self::ExperienceComposition(PyWorkspaceExperienceCompositionSubjectValue {})
            }
            PortableWorkspaceSubjectSelector::Resource {
                owner_module_id,
                resource_id,
            } => Self::Resource(PyWorkspaceResourceSubjectValue {
                owner_module_id,
                resource_id,
            }),
            PortableWorkspaceSubjectSelector::ResolutionLock => {
                Self::ResolutionLock(PyWorkspaceResolutionLockSubjectValue {})
            }
            PortableWorkspaceSubjectSelector::UniverseLock => {
                Self::UniverseLock(PyWorkspaceUniverseLockSubjectValue {})
            }
            PortableWorkspaceSubjectSelector::Workspace { workspace_id } => {
                Self::Workspace(PyWholeWorkspaceSubjectValue { workspace_id })
            }
        }
    }
}
