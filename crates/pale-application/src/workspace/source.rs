//! Explicit loading of portable workspace-language sources.
//!
//! The root [`WorkspaceManifestFile`] enumerates every workspace-language file
//! consumed by this loader. This stage performs no directory scan, acquisition,
//! environment lookup, network access, credential access, or publication.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::entry::confine_rel;
use super::source_admission::{
    admit_toml_with_evidence, AdmittedWorkspaceRecord, WorkspaceDocumentSource,
};
use super::WorkspaceError;
use pale_core::paths::OwnedConfinedRelativePath;
use pale_core::records::{
    OrganizationalTranslationV1, ResponsibilityDerivationId, ResponsibilityDerivationV1,
    WorkspaceComponentId, WorkspaceCompositionSourceV1, WorkspaceDerivationSourceId,
    WorkspaceDerivationSourceV1, WorkspaceLanguagePrimitivesV1, WorkspaceManifestFile,
    WorkspaceModuleId, WorkspaceModuleSourceV1, WorkspaceModuleV1, WorkspaceResolutionLockV1,
    WorkspaceSourceContentDigest, WorkspaceUniverseLockV1, MAX_EXPERIENCE_COMPOSITION_SOURCES,
    MAX_ORGANIZATIONAL_TRANSLATION_SOURCES, MAX_RESPONSIBILITY_DERIVATION_SOURCES,
    MAX_WORKSPACE_COMPONENTS, MAX_WORKSPACE_MODULES, ORGANIZATIONAL_TRANSLATION_V1_SCHEMA,
    ORGANIZATIONAL_TRANSLATION_V1_SCHEMA_ID, RESPONSIBILITY_DERIVATION_V1_SCHEMA,
    RESPONSIBILITY_DERIVATION_V1_SCHEMA_ID,
};

mod local_modules;

/// One typed source record retained with portable exact-byte derivation evidence.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct RetainedWorkspaceSource<T> {
    pub(super) value: T,
    pub(super) source: WorkspaceDerivationSourceV1,
}

/// Structurally admitted but not yet linked workspace sources.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ParsedWorkspaceSource {
    pub(super) manifest: WorkspaceManifestFile,
    pub(super) manifest_source: WorkspaceDerivationSourceV1,
    pub(super) modules: BTreeMap<WorkspaceModuleId, RetainedWorkspaceSource<WorkspaceModuleV1>>,
    pub(super) module_sources:
        BTreeMap<WorkspaceModuleId, RetainedWorkspaceSource<WorkspaceModuleSourceV1>>,
    pub(super) components:
        BTreeMap<WorkspaceComponentId, RetainedWorkspaceSource<WorkspaceLanguagePrimitivesV1>>,
    pub(super) responsibility_derivations:
        BTreeMap<ResponsibilityDerivationId, RetainedWorkspaceSource<ResponsibilityDerivationV1>>,
    pub(super) organizational_translation:
        Option<RetainedWorkspaceSource<OrganizationalTranslationV1>>,
    pub(super) experience_composition:
        Option<RetainedWorkspaceSource<WorkspaceCompositionSourceV1>>,
    pub(super) resolution_lock: RetainedWorkspaceSource<WorkspaceResolutionLockV1>,
    pub(super) universe_lock: RetainedWorkspaceSource<WorkspaceUniverseLockV1>,
}

/// Exact lexical source inventory declared by one workspace root manifest.
///
/// Both source-backed admission and host-to-portable capture consume this plan,
/// so adding a workspace source family cannot silently create a second host
/// enumeration policy.
pub(super) struct WorkspaceSourceInventory {
    documents: BTreeMap<String, PathBuf>,
}

impl WorkspaceSourceInventory {
    /// Resolve every declared source label below the root manifest's directory.
    pub(super) fn from_manifest(
        manifest_path: &Path,
        manifest: &WorkspaceManifestFile,
    ) -> Result<Self, WorkspaceError> {
        let root = manifest_path
            .parent()
            .ok_or_else(|| WorkspaceError::MissingParent(manifest_path.to_path_buf()))?;
        let root_label = manifest_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| WorkspaceError::EscapedReference(manifest_path.display().to_string()))?;
        let workspace = &manifest.workspace;
        let labels = std::iter::once(root_label)
            .chain(workspace.module_manifests.iter().map(AsRef::as_ref))
            .chain(workspace.module_sources.iter().map(AsRef::as_ref))
            .chain(workspace.source_components.iter().map(AsRef::as_ref))
            .chain(
                workspace
                    .responsibility_derivations
                    .iter()
                    .map(AsRef::as_ref),
            )
            .chain(
                workspace
                    .organizational_translations
                    .iter()
                    .map(AsRef::as_ref),
            )
            .chain(workspace.experience_compositions.iter().map(AsRef::as_ref))
            .chain([
                workspace.resolution_lock.as_ref(),
                workspace.universe_lock.as_ref(),
            ]);
        let documents = labels
            .map(|label| confine_rel(root, label).map(|path| (label.to_owned(), path)))
            .collect::<Result<_, _>>()?;
        Ok(Self { documents })
    }

    /// Borrow the resolved path for one manifest-declared logical label.
    fn path(&self, label: impl AsRef<str>) -> Result<&Path, WorkspaceError> {
        let label = label.as_ref();
        self.documents
            .get(label)
            .map(PathBuf::as_path)
            .ok_or_else(|| WorkspaceError::MissingReference {
                reference: label.to_owned(),
                path: label.to_owned(),
            })
    }

    /// Consume the manifest-selected lexical document map in label order.
    pub(super) fn into_documents(self) -> BTreeMap<String, PathBuf> {
        self.documents
    }
}

fn retain<T>(
    admitted: AdmittedWorkspaceRecord<T>,
    source_id: WorkspaceDerivationSourceId,
    source_path: impl AsRef<str>,
) -> Result<RetainedWorkspaceSource<T>, WorkspaceError> {
    Ok(RetainedWorkspaceSource {
        value: admitted.value,
        source: source_evidence(
            source_id,
            source_path.as_ref(),
            admitted.source_digest,
            admitted.byte_len,
        )?,
    })
}

/// Preserve the source identity recipe without assuming that its prefix fits.
///
/// A record ID can use the complete portable ID limit. Its prefixed source ID
/// must independently pass the existing parser; failure does not invalidate
/// the original record ID's grammar.
fn derivation_source_id(
    family: &'static str,
    id: &str,
    path: &Path,
    field_path: &'static [&'static str],
) -> Result<WorkspaceDerivationSourceId, WorkspaceError> {
    let combined = format!("{family}:{id}");
    let combined_byte_len = combined.len();
    WorkspaceDerivationSourceId::parse(combined).map_err(|source| {
        WorkspaceError::UnrepresentableDerivationSourceId {
            coordinate: Box::new(super::WorkspaceSourceCoordinate::path_only(path)),
            field_path,
            family,
            combined_byte_len,
            source,
        }
    })
}

fn source_evidence(
    source_id: WorkspaceDerivationSourceId,
    source_path: &str,
    source_digest: WorkspaceSourceContentDigest,
    byte_len: u32,
) -> Result<WorkspaceDerivationSourceV1, WorkspaceError> {
    let source_path = OwnedConfinedRelativePath::parse(source_path)
        .map_err(|_| WorkspaceError::EscapedReference(source_path.to_string()))?;
    Ok(WorkspaceDerivationSourceV1 {
        source_id,
        source_path,
        source_digest,
        byte_len,
    })
}

/// Load only the files explicitly named by one canonical root manifest.
pub(super) fn parse_workspace_source(
    sources: &(impl WorkspaceDocumentSource + ?Sized),
    manifest_path: &Path,
) -> Result<ParsedWorkspaceSource, WorkspaceError> {
    let admitted_manifest = admit_toml_with_evidence::<WorkspaceManifestFile>(
        sources,
        manifest_path,
        "pale-workspace.v1",
        "workspace manifest",
    )?;
    let manifest_name = manifest_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| WorkspaceError::EscapedReference(manifest_path.display().to_string()))?;
    let manifest_source = source_evidence(
        WorkspaceDerivationSourceId::parse("workspace-manifest")
            .expect("static derivation source identity is valid"),
        manifest_name,
        admitted_manifest.source_digest,
        admitted_manifest.byte_len,
    )?;
    let manifest = admitted_manifest.value;
    manifest
        .workspace
        .desired_topology
        .verify()
        .map_err(WorkspaceError::InvalidDesiredTopology)?;
    crate::composition::reject_candidate_only_surfaces(
        [
            RESPONSIBILITY_DERIVATION_V1_SCHEMA_ID,
            RESPONSIBILITY_DERIVATION_V1_SCHEMA,
            ORGANIZATIONAL_TRANSLATION_V1_SCHEMA_ID,
            ORGANIZATIONAL_TRANSLATION_V1_SCHEMA,
        ]
        .into_iter()
        .chain(
            manifest
                .workspace
                .module_manifests
                .iter()
                .map(|path| path.as_ref()),
        )
        .chain(
            manifest
                .workspace
                .module_sources
                .iter()
                .map(|path| path.as_ref()),
        )
        .chain(
            manifest
                .workspace
                .source_components
                .iter()
                .map(|path| path.as_ref()),
        )
        .chain(
            manifest
                .workspace
                .responsibility_derivations
                .iter()
                .map(|path| path.as_ref()),
        )
        .chain(
            manifest
                .workspace
                .organizational_translations
                .iter()
                .map(|path| path.as_ref()),
        )
        .chain(
            manifest
                .workspace
                .experience_compositions
                .iter()
                .map(|path| path.as_ref()),
        )
        .chain([
            manifest.workspace.resolution_lock.as_ref(),
            manifest.workspace.universe_lock.as_ref(),
        ]),
    )?;
    enforce_manifest_bound(
        "module manifest files",
        manifest.workspace.module_manifests.len(),
        MAX_WORKSPACE_MODULES,
    )?;
    enforce_manifest_bound(
        "source component files",
        manifest.workspace.source_components.len(),
        MAX_WORKSPACE_COMPONENTS,
    )?;
    enforce_manifest_bound(
        "responsibility derivation files",
        manifest.workspace.responsibility_derivations.len(),
        MAX_RESPONSIBILITY_DERIVATION_SOURCES,
    )?;
    enforce_manifest_bound(
        "organizational translation files",
        manifest.workspace.organizational_translations.len(),
        MAX_ORGANIZATIONAL_TRANSLATION_SOURCES,
    )?;
    enforce_manifest_bound(
        "experience composition files",
        manifest.workspace.experience_compositions.len(),
        MAX_EXPERIENCE_COMPOSITION_SOURCES,
    )?;
    let inventory = WorkspaceSourceInventory::from_manifest(manifest_path, &manifest)?;
    let module_sources = local_modules::read(sources, &manifest, &inventory)?;

    let mut modules = BTreeMap::new();
    for reference in &manifest.workspace.module_manifests {
        let path = inventory.path(reference)?;
        let admitted = admit_toml_with_evidence::<WorkspaceModuleV1>(
            sources,
            path,
            "workspace-module.v1",
            "workspace module",
        )?;
        let id = admitted.value.module.module_id.clone();
        let retained = retain(
            admitted,
            derivation_source_id("module", id.as_str(), path, &["module", "module_id"])?,
            reference,
        )?;
        if modules.insert(id.clone(), retained).is_some() {
            return Err(WorkspaceError::DuplicateLanguageIdentity {
                kind: "module",
                id: id.to_string(),
            });
        }
    }

    let mut components = BTreeMap::new();
    for reference in &manifest.workspace.source_components {
        let path = inventory.path(reference)?;
        let admitted = admit_toml_with_evidence::<WorkspaceLanguagePrimitivesV1>(
            sources,
            path,
            "workspace-language-primitives.v1",
            "workspace source component",
        )?;
        let id = admitted.value.component_id.clone();
        let retained = retain(
            admitted,
            derivation_source_id("component", id.as_str(), path, &["component_id"])?,
            reference,
        )?;
        if components.insert(id.clone(), retained).is_some() {
            return Err(WorkspaceError::DuplicateLanguageIdentity {
                kind: "source component",
                id: id.to_string(),
            });
        }
    }

    let mut responsibility_derivations = BTreeMap::new();
    for reference in &manifest.workspace.responsibility_derivations {
        let path = inventory.path(reference)?;
        let admitted = admit_toml_with_evidence::<ResponsibilityDerivationV1>(
            sources,
            path,
            RESPONSIBILITY_DERIVATION_V1_SCHEMA_ID,
            "responsibility derivation",
        )?;
        crate::composition::reject_candidate_only_derivation_identifiers(&admitted.value)?;
        let id = admitted.value.derivation_id.clone();
        let retained = retain(
            admitted,
            derivation_source_id("responsibility", id.as_str(), path, &["derivation_id"])?,
            reference,
        )?;
        if responsibility_derivations
            .insert(id.clone(), retained)
            .is_some()
        {
            return Err(WorkspaceError::DuplicateLanguageIdentity {
                kind: "responsibility derivation",
                id: id.to_string(),
            });
        }
    }

    let organizational_translation = manifest
        .workspace
        .organizational_translations
        .first()
        .map(|reference| {
            let path = inventory.path(reference)?;
            let admitted = admit_toml_with_evidence::<OrganizationalTranslationV1>(
                sources,
                path,
                ORGANIZATIONAL_TRANSLATION_V1_SCHEMA_ID,
                "organizational translation",
            )?;
            crate::composition::reject_candidate_only_record_identifiers(
                crate::composition::CandidateIdentityRecordV1::Organization(&admitted.value),
            )?;
            retain(
                admitted,
                WorkspaceDerivationSourceId::parse("organizational-translation")
                    .expect("static derivation source identity is valid"),
                reference,
            )
        })
        .transpose()?;

    let experience_composition = manifest
        .workspace
        .experience_compositions
        .first()
        .map(|reference| {
            let path = inventory.path(reference)?;
            let admitted = admit_toml_with_evidence::<WorkspaceCompositionSourceV1>(
                sources,
                path,
                "workspace-composition-source.v1",
                "workspace experience composition",
            )?;
            retain(
                admitted,
                WorkspaceDerivationSourceId::parse("experience-composition")
                    .expect("static derivation source identity is valid"),
                reference,
            )
        })
        .transpose()?;

    let resolution_path = inventory.path(&manifest.workspace.resolution_lock)?;
    let resolution = admit_toml_with_evidence::<WorkspaceResolutionLockV1>(
        sources,
        resolution_path,
        "workspace-resolution-lock.v1",
        "workspace resolution lock",
    )?;
    let universe_path = inventory.path(&manifest.workspace.universe_lock)?;
    let universe = admit_toml_with_evidence::<WorkspaceUniverseLockV1>(
        sources,
        universe_path,
        "workspace-universe-lock.v1",
        "workspace universe lock",
    )?;
    let resolution_lock = retain(
        resolution,
        WorkspaceDerivationSourceId::parse("resolution-lock")
            .expect("static derivation source identity is valid"),
        &manifest.workspace.resolution_lock,
    )?;
    let universe_lock = retain(
        universe,
        WorkspaceDerivationSourceId::parse("universe-lock")
            .expect("static derivation source identity is valid"),
        &manifest.workspace.universe_lock,
    )?;

    Ok(ParsedWorkspaceSource {
        manifest,
        manifest_source,
        modules,
        module_sources,
        components,
        responsibility_derivations,
        organizational_translation,
        experience_composition,
        resolution_lock,
        universe_lock,
    })
}

fn enforce_manifest_bound(
    kind: &'static str,
    observed: usize,
    limit: usize,
) -> Result<(), WorkspaceError> {
    if observed > limit {
        return Err(WorkspaceError::WorkspaceGraphLimit {
            kind,
            observed,
            limit,
        });
    }
    Ok(())
}
