//! Lower local authored modules without constructing release or admission evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use pale_core::records::{
    AttributableAnswerV1, OrganizationalTranslationIrV1, ResponsibilityDerivationIrV1,
    WorkspaceDerivationSourceV1, WorkspaceDesiredTopologyV1, WorkspaceId, WorkspaceModuleContentV1,
    MAX_WORKSPACE_RESOURCES,
};
use pale_ports::SourceFilesystemPort;
use serde::Serialize;

use super::source::ParsedWorkspaceSource;
use super::source_admission::SourceBackedWorkspaceDocuments;
use super::{link, source, CapturedWorkspaceSource, WorkspaceError};

/// Inspectable local source content, with no conversion into an admitted universe.
///
/// Module records have passed structural lowering. Project owner and duty
/// references resolve within this source. Responsibility and organization
/// records use their existing lowerers. Import linking, policy normalization,
/// publisher admission and native operation remain separate phases.
#[derive(Debug, Serialize)]
pub struct WorkspaceSourceLowering {
    phase: SourcePhase,
    workspace_id: WorkspaceId,
    content_digest: String,
    topology: WorkspaceDesiredTopologyV1,
    modules: Vec<WorkspaceModuleContentV1>,
    responsibilities: ResponsibilityDerivationIrV1,
    organization: Option<OrganizationalTranslationIrV1>,
    sources: Vec<WorkspaceDerivationSourceV1>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum SourcePhase {
    SourceOnly,
}

impl WorkspaceSourceLowering {
    /// Borrow the canonical local module content, without publication metadata.
    #[must_use]
    pub fn modules(&self) -> &[WorkspaceModuleContentV1] {
        &self.modules
    }

    /// Borrow the identity of the lowered meaning, independent of source whitespace.
    #[must_use]
    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    /// Encode source-only content and its separate exact-byte source evidence.
    ///
    /// # Errors
    ///
    /// Returns the canonical JSON encoding failure; this never issues a release.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, pale_core::canon::CanonError> {
        pale_core::canon::to_canonical_string(self).map(String::into_bytes)
    }
}

/// Read only manifest-selected local sources and lower their authored meaning.
///
/// No machine binding, executable, credential or Package distribution is opened.
/// Release imports use the ordinary compilation path; this entry accepts local
/// definitions with empty release/adoption locks and no publication coordinates.
///
/// # Errors
///
/// Rejects invalid or unresolved source, non-local module selections, unknown
/// project owners and duties, and exceeded source bounds.
pub fn lower_local_workspace_source(
    filesystem: &dyn SourceFilesystemPort,
    manifest_path: &Path,
) -> Result<WorkspaceSourceLowering, WorkspaceError> {
    let root = manifest_path
        .parent()
        .ok_or_else(|| WorkspaceError::MissingParent(manifest_path.to_owned()))?;
    let confined = SourceBackedWorkspaceDocuments::new(filesystem, root);
    lower(source::parse_workspace_source(&confined, manifest_path)?)
}

/// Lower an explicitly captured source batch without reading any host path.
///
/// # Errors
///
/// Returns the same structural and reference errors as local source lowering.
pub fn lower_captured_workspace_source(
    captured: CapturedWorkspaceSource,
) -> Result<WorkspaceSourceLowering, WorkspaceError> {
    lower(source::parse_workspace_source(
        &captured,
        captured.root_manifest(),
    )?)
}

fn lower(parsed: ParsedWorkspaceSource) -> Result<WorkspaceSourceLowering, WorkspaceError> {
    parsed
        .resolution_lock
        .value
        .verify()
        .map_err(WorkspaceError::InvalidWorkspaceLock)?;
    parsed
        .universe_lock
        .value
        .verify()
        .map_err(WorkspaceError::InvalidWorkspaceLock)?;
    if parsed.universe_lock.value.resolution_digest
        != parsed.resolution_lock.value.resolution_digest
    {
        return Err(WorkspaceError::UniverseLockMismatch(
            "universe references a different resolution lock".into(),
        ));
    }
    if parsed.manifest.workspace.workspace_id != parsed.universe_lock.value.workspace_id {
        return Err(WorkspaceError::UniverseLockMismatch(
            "root and universe workspace identities differ".into(),
        ));
    }
    if !parsed.modules.is_empty()
        || !parsed.resolution_lock.value.releases.is_empty()
        || !parsed.universe_lock.value.adopted_releases.is_empty()
    {
        return Err(WorkspaceError::NamespaceResolution("source-only lowering accepts local module sources with empty release and adoption selections; use ordinary compilation for published modules".into()));
    }
    if parsed.experience_composition.is_some() {
        return Err(WorkspaceError::NamespaceResolution("source-only lowering does not compile execution paths; keep experience composition in its separately qualified runtime candidate".into()));
    }
    let components = parsed
        .components
        .iter()
        .map(|(id, retained)| (id.clone(), retained.value.clone()))
        .collect::<BTreeMap<_, _>>();
    let modules = parsed
        .module_sources
        .values()
        .map(|retained| {
            retained.value.lower(&components).map_err(|error| {
                WorkspaceError::NamespaceResolution(format!(
                    "local module `{}`: {error}",
                    retained.value.module_id
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut selected_components = BTreeSet::new();
    for module in &modules {
        if !module.imports.is_empty() {
            return Err(WorkspaceError::NamespaceResolution(format!("local module `{}` declares release imports; their linking and admission require the published-module path", module.module_id)));
        }
        selected_components.extend(
            module
                .source_components
                .iter()
                .map(|component| &component.component_id),
        );
    }
    if let Some(id) = parsed
        .components
        .keys()
        .find(|id| !selected_components.contains(id))
    {
        return Err(WorkspaceError::NamespaceResolution(format!(
            "source component `{id}` is not selected by its local module"
        )));
    }
    let resources = modules
        .iter()
        .flat_map(|module| {
            module
                .resources
                .iter()
                .cloned()
                .map(|resource| (module.module_id.clone(), resource))
        })
        .collect::<Vec<_>>();
    if resources.len() > MAX_WORKSPACE_RESOURCES {
        return Err(WorkspaceError::WorkspaceGraphLimit {
            kind: "local module resources",
            observed: resources.len(),
            limit: MAX_WORKSPACE_RESOURCES,
        });
    }
    let responsibilities = crate::composition::lower_responsibility_derivations(
        parsed
            .responsibility_derivations
            .values()
            .map(|source| source.value.clone())
            .collect(),
    )?;
    for duty in &responsibilities.derivations {
        if matches!(duty.publisher_module, AttributableAnswerV1::Supplied { .. })
            || matches!(duty.adopter_module, AttributableAnswerV1::Supplied { .. })
        {
            return Err(WorkspaceError::NamespaceResolution(format!("source-only responsibility `{}` supplies published or adopted module coordinates; retain an attributable omission until those releases exist", duty.derivation_id)));
        }
    }
    let organization = parsed
        .organizational_translation
        .as_ref()
        .map(|source| crate::composition::lower_organizational_translation(source.value.clone()))
        .transpose()?;
    link::validate_directional_service_closure(&responsibilities, organization.as_ref())?;
    link::validate_project_context(
        &parsed.manifest.workspace.desired_topology,
        &resources,
        &responsibilities,
    )?;
    let topology = parsed.manifest.workspace.desired_topology;
    let workspace_id = parsed.manifest.workspace.workspace_id;
    let content_digest = pale_core::canon::digest(&(
        "pale.workspace-source-lowering.v1",
        &workspace_id,
        &topology,
        &modules,
        &responsibilities,
        &organization,
    ))
    .map_err(WorkspaceError::Canon)?;
    let sources = std::iter::once(parsed.manifest_source)
        .chain(
            parsed
                .module_sources
                .into_values()
                .map(|source| source.source),
        )
        .chain(parsed.components.into_values().map(|source| source.source))
        .chain(
            parsed
                .responsibility_derivations
                .into_values()
                .map(|source| source.source),
        )
        .chain(
            parsed
                .organizational_translation
                .map(|source| source.source),
        )
        .chain([parsed.resolution_lock.source, parsed.universe_lock.source])
        .collect();
    Ok(WorkspaceSourceLowering {
        phase: SourcePhase::SourceOnly,
        workspace_id,
        content_digest,
        topology,
        modules,
        responsibilities,
        organization,
        sources,
    })
}
