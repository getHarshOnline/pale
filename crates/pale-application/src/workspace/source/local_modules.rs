//! Load explicitly selected module sources without assigning release identities.

use super::*;

pub(super) fn read(
    sources: &(impl WorkspaceDocumentSource + ?Sized),
    manifest: &WorkspaceManifestFile,
    inventory: &WorkspaceSourceInventory,
) -> Result<
    BTreeMap<WorkspaceModuleId, RetainedWorkspaceSource<WorkspaceModuleSourceV1>>,
    WorkspaceError,
> {
    enforce_manifest_bound(
        "module source files",
        manifest.workspace.module_sources.len(),
        MAX_WORKSPACE_MODULES,
    )?;
    let mut modules = BTreeMap::new();
    for reference in &manifest.workspace.module_sources {
        let path = inventory.path(reference)?;
        let admitted = admit_toml_with_evidence::<WorkspaceModuleSourceV1>(
            sources,
            path,
            "workspace-module-source.v1",
            "local module source",
        )?;
        let id = admitted.value.module_id.clone();
        let retained = retain(
            admitted,
            derivation_source_id("module-source", id.as_str(), path, &["module_id"])?,
            reference,
        )?;
        if modules.insert(id.clone(), retained).is_some() {
            return Err(WorkspaceError::DuplicateLanguageIdentity {
                kind: "local module source",
                id: id.to_string(),
            });
        }
    }
    Ok(modules)
}
