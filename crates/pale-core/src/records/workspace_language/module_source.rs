//! Author a module's content before selecting publication identities or keys.

use std::collections::BTreeMap;

use super::*;

/// Local module source, with component identities instead of author-supplied digests.
///
/// This is the source of a future module candidate, not an unsigned release.
/// Lowering computes component identities and canonicalizes the same module
/// content used by [`WorkspaceModuleV1`]. Publication metadata is supplied only
/// when a caller explicitly prepares that separate candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceModuleSourceV1 {
    /// Source grammar identity.
    #[schemars(extend("const" = "pale/workspace-module-source.v1"))]
    pub schema: String,
    /// Initial source grammar version.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Stable module identity, independent of any release or publisher.
    pub module_id: WorkspaceModuleId,
    /// Components explicitly named by the root and owned by this module.
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub source_components: Vec<WorkspaceComponentId>,
    /// Exact release imports to check separately before operational admission.
    #[schemars(schema_with = "imports_schema")]
    pub imports: Vec<WorkspaceImportV1>,
    /// Named exports of this module's content.
    #[schemars(schema_with = "exports_schema")]
    pub exports: Vec<WorkspaceExportV1>,
    /// Typed extensions and narrowings.
    #[schemars(schema_with = "extensions_schema")]
    pub extensions: Vec<WorkspaceExtensionV1>,
    /// Logical resources declared by this module.
    #[schemars(schema_with = "resources_schema")]
    pub resources: Vec<WorkspaceResourceSelectionV1>,
}

/// Canonical module content with computed component digests and no release authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkspaceModuleContentV1 {
    /// Stable module identity.
    pub module_id: WorkspaceModuleId,
    /// Exact component content selected by the source.
    pub source_components: Vec<WorkspaceSourceComponentRefV1>,
    /// Declared release imports; lowering alone does not admit them.
    pub imports: Vec<WorkspaceImportV1>,
    /// Canonical named exports.
    pub exports: Vec<WorkspaceExportV1>,
    /// Canonical extensions and narrowings.
    pub extensions: Vec<WorkspaceExtensionV1>,
    /// Canonical logical resource declarations.
    pub resources: Vec<WorkspaceResourceSelectionV1>,
}

impl WorkspaceModuleSourceV1 {
    /// Resolve named components and canonicalize the module's content.
    ///
    /// # Errors
    ///
    /// Rejects missing or differently owned components, duplicate identities,
    /// invalid records and exceeded module limits. This does not resolve
    /// imports, verify publisher evidence, or admit a runtime universe.
    pub fn lower(
        &self,
        components: &BTreeMap<WorkspaceComponentId, WorkspaceLanguagePrimitivesV1>,
    ) -> Result<WorkspaceModuleContentV1, InvalidWorkspaceModuleSource> {
        if self.schema != "pale/workspace-module-source.v1" || self.schema_version != 1 {
            return Err(InvalidWorkspaceModuleSource::SchemaCoordinate);
        }
        if self.source_components.len() > MAX_WORKSPACE_COMPONENTS {
            return Err(InvalidWorkspaceModule::Limit {
                kind: "source component",
                observed: self.source_components.len(),
                limit: MAX_WORKSPACE_COMPONENTS,
            }
            .into());
        }
        let mut content = WorkspaceModuleContentV1 {
            module_id: self.module_id.clone(),
            source_components: self
                .source_components
                .iter()
                .map(|id| {
                    let component = components.get(id).ok_or_else(|| {
                        InvalidWorkspaceModuleSource::MissingComponent(id.clone())
                    })?;
                    if component.component_id != *id || component.module_id != self.module_id {
                        return Err(InvalidWorkspaceModuleSource::ComponentOwner(id.clone()));
                    }
                    component.verify()?;
                    Ok(WorkspaceSourceComponentRefV1 {
                        component_id: id.clone(),
                        component_digest: component.component_digest()?,
                    })
                })
                .collect::<Result<_, InvalidWorkspaceModuleSource>>()?,
            imports: self.imports.clone(),
            exports: self.exports.clone(),
            extensions: self.extensions.clone(),
            resources: self.resources.clone(),
        };
        module::canonicalize_module_sets(
            &mut content.source_components,
            &mut content.imports,
            &mut content.exports,
            &mut content.extensions,
            &mut content.resources,
        )?;
        module::validate_module_content_sets(
            &content.source_components,
            &content.imports,
            &content.exports,
            &content.extensions,
            &content.resources,
        )?;
        Ok(content)
    }
}

impl WorkspaceModuleContentV1 {
    /// Hash canonical local content without claiming a release identity.
    ///
    /// # Errors
    ///
    /// Returns the canonical JSON encoding error for this content.
    pub fn source_digest(&self) -> Result<String, crate::canon::CanonError> {
        crate::canon::digest(&("pale.workspace-module-source.v1", self))
    }

    /// Prepare a module candidate using explicitly supplied publication identities.
    ///
    /// Sealing computes the candidate's digest; it does not sign or publish it.
    /// The resulting candidate still requires its ordinary release and admission checks.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspaceModule`] if the content cannot be sealed.
    pub fn prepare_release(
        self,
        release_id: WorkspaceReleaseId,
        publisher: WorkspacePublisherRefV1,
    ) -> Result<WorkspaceModuleV1, InvalidWorkspaceModule> {
        WorkspaceModuleV1::seal(WorkspaceModulePreimageV1 {
            module_id: self.module_id,
            release_id,
            publisher,
            source_components: self.source_components,
            imports: self.imports,
            exports: self.exports,
            extensions: self.extensions,
            resources: self.resources,
        })
    }
}

/// A local module cannot be lowered from its explicitly selected components.
#[derive(Debug, thiserror::Error)]
pub enum InvalidWorkspaceModuleSource {
    /// Source grammar identity or version is wrong.
    #[error("module source schema coordinate is not initial-v1")]
    SchemaCoordinate,
    /// The root did not supply a selected component.
    #[error("module source references missing component `{0}`")]
    MissingComponent(WorkspaceComponentId),
    /// A selected component belongs to another module or identity.
    #[error("module source does not own component `{0}`")]
    ComponentOwner(WorkspaceComponentId),
    /// The selected component violates its existing grammar.
    #[error(transparent)]
    Component(#[from] InvalidWorkspaceComponent),
    /// Module content violates the shared module rules.
    #[error(transparent)]
    Module(#[from] InvalidWorkspaceModule),
    /// Component content cannot be canonically encoded.
    #[error(transparent)]
    Canonical(#[from] crate::canon::CanonError),
}
