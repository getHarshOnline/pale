//! Exact linking for portable workspace modules, releases, and declarations.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    canonicalize_release_closures, AttributableAnswerV1, DeclarationDependencyKindV1,
    ExperienceCompositionDeclarationsV1, NormativePackReleaseV1, OrganizationalTranslationIrV1,
    PackageReleaseV1, ResponsibilityDerivationIrV1, ResponsibilityPlacementV1,
    WorkspaceAdoptionPolicyV1, WorkspaceDeclarationRefV1, WorkspaceDeclarationV1,
    WorkspaceDeliveryV1, WorkspaceDerivationSubjectV1, WorkspaceExportId, WorkspaceExportV1,
    WorkspaceImportV1, WorkspaceModuleId, WorkspaceNamespaceId, WorkspaceObligationId,
    WorkspacePolicyCeilingV1, WorkspacePropagationV1, WorkspaceReleaseId, WorkspaceResourceId,
    WorkspaceResourceSelectionV1, WorkspaceVisibilityV1, MAX_WORKSPACE_ACCESS_ENTRIES,
    MAX_WORKSPACE_COMPONENTS, MAX_WORKSPACE_DECLARATIONS, MAX_WORKSPACE_DECLARATION_EDGES,
    MAX_WORKSPACE_EXPORTS, MAX_WORKSPACE_EXPORT_PROJECTION_ENTRIES, MAX_WORKSPACE_EXTENSIONS,
    MAX_WORKSPACE_IMPORTS, MAX_WORKSPACE_IMPORT_PATHS, MAX_WORKSPACE_MODULES,
    MAX_WORKSPACE_RESOURCES,
};

use super::dependency::{
    DeclarationDependencyAnalysis, DeclarationDependencyEdge, DeclarationDependencyGraph,
    DeclarationEdgeRole,
};
use super::source::ParsedWorkspaceSource;
use super::{AuthoringInstancePathSegment, WorkspaceAuthoringLocation, WorkspaceError};

mod project_context;
pub(super) use project_context::validate_project_context;

/// Canonical declaration identity after a module-relative wire reference resolves.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub(super) struct QualifiedDeclarationRef {
    pub(super) owner_module: WorkspaceModuleId,
    pub(super) local_ref: WorkspaceDeclarationRefV1,
}

impl std::fmt::Display for QualifiedDeclarationRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}::{}", self.owner_module, self.local_ref)
    }
}

/// One dependency whose module-relative target has resolved exactly once.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct LinkedDependency {
    pub(super) target: QualifiedDeclarationRef,
    pub(super) kind: DeclarationDependencyKindV1,
}

/// One authored declaration plus its separately resolved references.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct LinkedDeclaration {
    pub(super) declaration: WorkspaceDeclarationV1,
    pub(super) dependencies: Vec<LinkedDependency>,
    pub(super) relationship_from: Option<QualifiedDeclarationRef>,
    pub(super) relationship_to: Option<QualifiedDeclarationRef>,
}

/// One extension after every source-relative reference has resolved to its owner coordinate.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct LinkedExtension {
    pub(super) owner_module: WorkspaceModuleId,
    pub(super) namespace: WorkspaceNamespaceId,
    pub(super) target: QualifiedDeclarationRef,
    pub(super) added_traits: Vec<QualifiedDeclarationRef>,
    pub(super) narrowed_ceiling: Option<WorkspacePolicyCeilingV1>,
    pub(super) added_obligations: Vec<WorkspaceObligationId>,
}

impl LinkedExtension {
    /// Build the full proof subject from resolved extension fields.
    pub(super) fn subject(&self) -> WorkspaceDerivationSubjectV1 {
        WorkspaceDerivationSubjectV1::Extension {
            owner_module: self.owner_module.clone(),
            namespace: self.namespace.clone(),
            target: self.target.local_ref.clone(),
        }
    }
}

/// How one importer-local reference reached its canonical owner.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) enum AccessOrigin {
    Local,
    Import {
        upstream_module: WorkspaceModuleId,
        upstream_release: WorkspaceReleaseId,
        export_id: WorkspaceExportId,
        source_namespace: WorkspaceNamespaceId,
        local_namespace: WorkspaceNamespaceId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One declaration's canonical owner, access provenance, and effective policy.
struct DeclarationAccess {
    canonical: QualifiedDeclarationRef,
    propagation: WorkspacePropagationV1,
    policy: Option<EffectiveAccessPolicy>,
    origin: AccessOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
/// Effective portable policy inherited by a resolved declaration surface.
pub(super) struct EffectiveAccessPolicy {
    pub(super) visibility: WorkspaceVisibilityV1,
    pub(super) delivery: WorkspaceDeliveryV1,
    pub(super) ceiling: WorkspacePolicyCeilingV1,
    pub(super) obligations: Vec<WorkspaceObligationId>,
    pub(super) exclusive_authority: Vec<pale_core::records::WorkspaceExclusiveAuthorityClaimV1>,
}

impl EffectiveAccessPolicy {
    fn from_export(export: &WorkspaceExportV1) -> Self {
        Self {
            visibility: export.visibility,
            delivery: export.delivery,
            ceiling: export.ceiling.clone(),
            obligations: export.obligations.clone(),
            exclusive_authority: export.exclusive_authority.clone(),
        }
    }
}

#[derive(Debug, Clone)]
/// One declaration resolved into an export with its access provenance intact.
struct ResolvedExportDeclaration {
    source_ref: WorkspaceDeclarationRefV1,
    access: DeclarationAccess,
}

#[derive(Debug, Clone)]
/// Canonical export surface paired with its resolved declaration access.
struct ResolvedExport {
    surface: WorkspaceExportV1,
    declarations: Vec<ResolvedExportDeclaration>,
}

/// Per-module declaration access indexed by module-relative reference.
type AccessMaps =
    BTreeMap<WorkspaceModuleId, BTreeMap<WorkspaceDeclarationRefV1, DeclarationAccess>>;
/// One access fact that can satisfy an export projection.
type AccessKey = (WorkspaceModuleId, WorkspaceDeclarationRefV1);
/// Exact resolved export indexed by owner module and export identity.
type ResolvedExports = BTreeMap<(WorkspaceModuleId, WorkspaceExportId), ResolvedExport>;
/// Stable source-order identity of one import path.
type ImportPathKey = (WorkspaceModuleId, usize, usize);
/// Linked access, exports, and import provenance returned as one resolution product.
type ResolvedAccessMaps = (
    AccessMaps,
    ResolvedExports,
    Vec<ResolvedImportPath>,
    AccessConvergenceWitness,
);

/// Finite progress evidence for workspace export/import access closure.
///
/// Each export and import-path rule enters the deterministic worklist at most
/// once. The final equality between rule and resolution counts is established
/// before this witness is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AccessConvergenceWitness {
    /// Complete admitted export-rule count.
    pub(super) export_rule_count: usize,
    /// Complete admitted import-path-rule count.
    pub(super) import_path_rule_count: usize,
    /// Total rules resolved; successful return proves this equals both counts.
    pub(super) resolution_step_count: usize,
}

/// Exact provenance and effective policy of one selected import path.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct ResolvedImportPath {
    pub(super) importer_module: WorkspaceModuleId,
    pub(super) upstream_module: WorkspaceModuleId,
    pub(super) upstream_release: WorkspaceReleaseId,
    pub(super) upstream_module_digest: pale_core::records::WorkspaceModuleDigest,
    pub(super) upstream_publisher: pale_core::records::WorkspacePublisherRefV1,
    pub(super) export_id: WorkspaceExportId,
    pub(super) source_namespace: WorkspaceNamespaceId,
    pub(super) local_namespace: WorkspaceNamespaceId,
    pub(super) export_visibility: WorkspaceVisibilityV1,
    pub(super) effective_visibility: WorkspaceVisibilityV1,
    pub(super) export_delivery: WorkspaceDeliveryV1,
    pub(super) export_propagation: WorkspacePropagationV1,
    pub(super) effective_propagation: WorkspacePropagationV1,
    pub(super) adoption_policy: WorkspaceAdoptionPolicyV1,
    pub(super) protected_delivery_authority: Option<WorkspaceResourceId>,
    pub(super) declarations: Vec<QualifiedDeclarationRef>,
    /// Ceiling declared by the selected export at this hop.
    pub(super) export_ceiling: WorkspacePolicyCeilingV1,
    /// Equal-or-narrower ceiling declared by the importing edge.
    pub(super) import_ceiling: WorkspacePolicyCeilingV1,
    /// Product meet of the export and import ceilings.
    pub(super) effective_ceiling: WorkspacePolicyCeilingV1,
    pub(super) obligations: Vec<WorkspaceObligationId>,
    pub(super) exclusive_authority: Vec<pale_core::records::WorkspaceExclusiveAuthorityClaimV1>,
}

impl ResolvedImportPath {
    /// Project the existing complete import proof identity without policy fields.
    pub(super) fn subject(&self) -> WorkspaceDerivationSubjectV1 {
        WorkspaceDerivationSubjectV1::Import {
            importer_module: self.importer_module.clone(),
            upstream_module: self.upstream_module.clone(),
            upstream_release: self.upstream_release.clone(),
            export_id: self.export_id.clone(),
            source_namespace: self.source_namespace.clone(),
            local_namespace: self.local_namespace.clone(),
        }
    }
}

/// Serializable exact access result retained in private workspace IR.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct LinkedAccessEntry {
    pub(super) importer_module: WorkspaceModuleId,
    pub(super) local_ref: WorkspaceDeclarationRefV1,
    pub(super) canonical: QualifiedDeclarationRef,
    pub(super) propagation: WorkspacePropagationV1,
    pub(super) policy: Option<EffectiveAccessPolicy>,
    pub(super) origin: AccessOrigin,
}

/// Linked workspace whose selected releases, namespaces, and declarations agree.
#[derive(Debug)]
pub(super) struct LinkedWorkspace {
    pub(super) source: ParsedWorkspaceSource,
    /// Source-order record custody moved into authoring evidence before normalization.
    /// It is not part of any serialized semantic record or hash preimage.
    pub(super) record_anchors: BTreeMap<WorkspaceDerivationSubjectV1, WorkspaceAuthoringLocation>,
    /// Canonical attributable responsibility closure from explicit root files.
    pub(super) responsibility_derivations: ResponsibilityDerivationIrV1,
    /// Canonical optional organizational closure; absence remains explicit.
    pub(super) organizational_translation: Option<OrganizationalTranslationIrV1>,
    /// Explicit neutral composition declarations; absence in the root is empty.
    pub(super) experience_composition: ExperienceCompositionDeclarationsV1,
    pub(super) adopted_modules: BTreeSet<WorkspaceModuleId>,
    pub(super) declarations: BTreeMap<QualifiedDeclarationRef, LinkedDeclaration>,
    pub(super) dependency_analysis: DeclarationDependencyAnalysis,
    pub(super) extensions: Vec<LinkedExtension>,
    pub(super) resolved_imports: Vec<ResolvedImportPath>,
    pub(super) access_entries: Vec<LinkedAccessEntry>,
    pub(super) resources: Vec<(WorkspaceModuleId, WorkspaceResourceSelectionV1)>,
    /// Canonical Package releases selected by adopted logical resources.
    pub(super) package_releases: Vec<PackageReleaseV1>,
    /// Canonical attestor-scoped normative Pack releases selected by resources.
    pub(super) normative_pack_releases: Vec<NormativePackReleaseV1>,
}

fn export_preserves_access_policy(
    export: &WorkspaceExportV1,
    inherited: &EffectiveAccessPolicy,
) -> Result<bool, WorkspaceError> {
    if !export.visibility.is_narrower_or_equal(inherited.visibility)
        || !export.delivery.is_narrower_or_equal(inherited.delivery)
        || !export.ceiling.is_narrower_or_equal(&inherited.ceiling)
    {
        return Ok(false);
    }
    let obligations =
        WorkspaceObligationId::try_union_sets(&inherited.obligations, &export.obligations)?;
    if obligations != export.obligations {
        return Ok(false);
    }
    Ok(
        pale_core::records::WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(
            &inherited.exclusive_authority,
            &export.exclusive_authority,
        )? == export.exclusive_authority,
    )
}

fn bounded(kind: &'static str, observed: usize, limit: usize) -> Result<(), WorkspaceError> {
    if observed > limit {
        Err(WorkspaceError::WorkspaceGraphLimit {
            kind,
            observed,
            limit,
        })
    } else {
        Ok(())
    }
}

fn validate_component_sets(
    component: &pale_core::records::WorkspaceLanguagePrimitivesV1,
) -> Result<(), WorkspaceError> {
    component
        .verify()
        .map_err(WorkspaceError::InvalidWorkspaceComponent)?;
    Ok(())
}

fn local_declarations(
    source: &ParsedWorkspaceSource,
    source_modules: &BTreeSet<WorkspaceModuleId>,
    adopted_modules: &BTreeSet<WorkspaceModuleId>,
) -> Result<
    (
        BTreeMap<QualifiedDeclarationRef, WorkspaceDeclarationV1>,
        AccessMaps,
    ),
    WorkspaceError,
> {
    let declaration_count = source
        .components
        .values()
        .filter(|component| source_modules.contains(&component.value.module_id))
        .map(|component| component.value.declarations.len())
        .sum::<usize>();
    bounded(
        "declarations",
        declaration_count,
        MAX_WORKSPACE_DECLARATIONS,
    )?;
    let edge_count = source
        .components
        .values()
        .filter(|component| source_modules.contains(&component.value.module_id))
        .flat_map(|component| &component.value.declarations)
        .map(|declaration| {
            declaration.dependencies().len()
                + usize::from(matches!(
                    declaration,
                    WorkspaceDeclarationV1::Relationship { .. }
                )) * 2
        })
        .sum();
    bounded(
        "declaration reference edges",
        edge_count,
        MAX_WORKSPACE_DECLARATION_EDGES,
    )?;
    let mut declarations = BTreeMap::new();
    let mut access = adopted_modules
        .iter()
        .cloned()
        .map(|module_id| (module_id, BTreeMap::new()))
        .collect::<AccessMaps>();
    for component in source
        .components
        .values()
        .filter(|component| source_modules.contains(&component.value.module_id))
    {
        for declaration in &component.value.declarations {
            let declaration_ref = declaration.declaration_ref();
            let owner = component.value.module_id.clone();
            let qualified = QualifiedDeclarationRef {
                owner_module: owner.clone(),
                local_ref: declaration_ref.clone(),
            };
            if declarations
                .insert(qualified.clone(), declaration.clone())
                .is_some()
            {
                return Err(WorkspaceError::DuplicateLanguageIdentity {
                    kind: "declaration",
                    id: qualified.to_string(),
                });
            }
            if access
                .get_mut(&owner)
                .expect("adopted access map")
                .insert(
                    declaration_ref,
                    DeclarationAccess {
                        canonical: qualified,
                        propagation: WorkspacePropagationV1::Transitive,
                        policy: None,
                        origin: AccessOrigin::Local,
                    },
                )
                .is_some()
            {
                return Err(WorkspaceError::DuplicateLanguageIdentity {
                    kind: "module-local declaration",
                    id: owner.to_string(),
                });
            }
        }
    }
    Ok((declarations, access))
}

fn required_source_modules(
    source: &ParsedWorkspaceSource,
    adopted_modules: &BTreeSet<WorkspaceModuleId>,
) -> Result<BTreeSet<WorkspaceModuleId>, WorkspaceError> {
    let imported_modules = adopted_modules
        .iter()
        .flat_map(|module_id| &source.modules[module_id].value.module.imports)
        .map(|import| import.module_id.clone())
        .collect::<BTreeSet<_>>();
    let mut required = adopted_modules
        .difference(&imported_modules)
        .cloned()
        .collect::<BTreeSet<_>>();
    for importer in adopted_modules {
        for import in &source.modules[importer].value.module.imports {
            for path in &import.paths {
                let export = source.modules[&import.module_id]
                    .value
                    .module
                    .exports
                    .iter()
                    .find(|export| export.export_id == path.export_id)
                    .ok_or_else(|| {
                        WorkspaceError::NamespaceResolution(format!(
                            "import path `{}` selects absent export `{}`",
                            import.module_id, path.export_id
                        ))
                    })?;
                if export.delivery == WorkspaceDeliveryV1::Source {
                    required.insert(import.module_id.clone());
                }
            }
        }
    }
    Ok(required)
}

fn add_export_projections(
    source: &ParsedWorkspaceSource,
    adopted_modules: &BTreeSet<WorkspaceModuleId>,
    declarations: &mut BTreeMap<QualifiedDeclarationRef, WorkspaceDeclarationV1>,
    access: &mut AccessMaps,
) -> Result<(), WorkspaceError> {
    let projected_refs = adopted_modules
        .iter()
        .flat_map(|module_id| &source.modules[module_id].value.module.exports)
        .flat_map(|export| &export.declarations)
        .map(|projection| QualifiedDeclarationRef {
            owner_module: projection.owner_module.clone(),
            local_ref: projection.declaration.declaration_ref(),
        })
        .collect::<BTreeSet<_>>();
    let mut present_source = BTreeMap::new();
    for component in source.components.values() {
        for declaration in &component.value.declarations {
            let canonical = QualifiedDeclarationRef {
                owner_module: component.value.module_id.clone(),
                local_ref: declaration.declaration_ref(),
            };
            if projected_refs.contains(&canonical)
                && present_source
                    .insert(canonical.clone(), declaration)
                    .is_some()
            {
                return Err(WorkspaceError::DuplicateLanguageIdentity {
                    kind: "projected source declaration",
                    id: canonical.to_string(),
                });
            }
        }
    }
    for module_id in adopted_modules {
        for export in &source.modules[module_id].value.module.exports {
            for projection in &export.declarations {
                let canonical = QualifiedDeclarationRef {
                    owner_module: projection.owner_module.clone(),
                    local_ref: projection.declaration.declaration_ref(),
                };
                if projection.owner_module == *module_id
                    && projection.local_ref != canonical.local_ref
                {
                    return Err(WorkspaceError::NamespaceResolution(format!(
                        "owner export `{module_id}:{}` aliases its own declaration `{}`",
                        export.export_id, projection.local_ref
                    )));
                }
                if present_source
                    .get(&canonical)
                    .is_some_and(|source_declaration| {
                        *source_declaration != &projection.declaration
                    })
                {
                    return Err(WorkspaceError::ExportProjectionMismatch(
                        canonical.to_string(),
                    ));
                }
                if declarations
                    .insert(canonical.clone(), projection.declaration.clone())
                    .is_some_and(|existing| existing != projection.declaration)
                {
                    return Err(WorkspaceError::ExportProjectionMismatch(
                        canonical.to_string(),
                    ));
                }
                let owner_access = access.get_mut(&canonical.owner_module).ok_or_else(|| {
                    WorkspaceError::UniverseLockMismatch(format!(
                        "projection owner `{}` is not adopted",
                        canonical.owner_module
                    ))
                })?;
                let projected_access = DeclarationAccess {
                    canonical: canonical.clone(),
                    propagation: WorkspacePropagationV1::Transitive,
                    policy: None,
                    origin: AccessOrigin::Local,
                };
                if owner_access
                    .get(&canonical.local_ref)
                    .is_some_and(|existing| existing != &projected_access)
                {
                    return Err(WorkspaceError::DuplicateLanguageIdentity {
                        kind: "projection owner reference",
                        id: canonical.to_string(),
                    });
                }
                owner_access.insert(canonical.local_ref.clone(), projected_access);
            }
        }
    }
    bounded(
        "declarations",
        declarations.len(),
        MAX_WORKSPACE_DECLARATIONS,
    )?;
    let edge_count = declarations
        .values()
        .map(|declaration| {
            declaration.dependencies().len()
                + usize::from(matches!(
                    declaration,
                    WorkspaceDeclarationV1::Relationship { .. }
                )) * 2
        })
        .sum();
    bounded(
        "declaration reference edges",
        edge_count,
        MAX_WORKSPACE_DECLARATION_EDGES,
    )
}

fn export_catalog(
    source: &ParsedWorkspaceSource,
    adopted_modules: &BTreeSet<WorkspaceModuleId>,
) -> Result<BTreeMap<(WorkspaceModuleId, WorkspaceExportId), WorkspaceExportV1>, WorkspaceError> {
    let mut exports = BTreeMap::new();
    for module_id in adopted_modules {
        for export in &source.modules[module_id].value.module.exports {
            let key = (module_id.clone(), export.export_id.clone());
            if exports.insert(key.clone(), export.clone()).is_some() {
                return Err(WorkspaceError::DuplicateLanguageIdentity {
                    kind: "module export",
                    id: format!("{}:{}", key.0, key.1),
                });
            }
        }
    }
    bounded("exports", exports.len(), MAX_WORKSPACE_EXPORTS)?;
    Ok(exports)
}

fn validate_source_and_locks(
    source: &ParsedWorkspaceSource,
) -> Result<BTreeSet<WorkspaceModuleId>, WorkspaceError> {
    source
        .resolution_lock
        .value
        .verify()
        .map_err(WorkspaceError::InvalidWorkspaceLock)?;
    source
        .universe_lock
        .value
        .verify()
        .map_err(WorkspaceError::InvalidWorkspaceLock)?;
    if source.universe_lock.value.resolution_digest
        != source.resolution_lock.value.resolution_digest
    {
        return Err(WorkspaceError::UniverseLockMismatch(
            "universe references a different resolution lock".into(),
        ));
    }
    if source.manifest.workspace.workspace_id != source.universe_lock.value.workspace_id {
        return Err(WorkspaceError::UniverseLockMismatch(
            "root and universe workspace identities differ".into(),
        ));
    }

    let resolved = source
        .resolution_lock
        .value
        .releases
        .iter()
        .map(|release| (release.module_id.clone(), release))
        .collect::<BTreeMap<_, _>>();
    if resolved.len() != source.resolution_lock.value.releases.len()
        || resolved.len() != source.modules.len()
    {
        return Err(WorkspaceError::ResolutionLockMismatch(
            "release set is not a bijection with enumerated modules".into(),
        ));
    }
    bounded("resolved modules", resolved.len(), MAX_WORKSPACE_MODULES)?;
    for (module_id, retained) in &source.modules {
        retained
            .value
            .verify()
            .map_err(WorkspaceError::InvalidWorkspaceModule)?;
        let release = resolved.get(module_id).ok_or_else(|| {
            WorkspaceError::ResolutionLockMismatch(format!("missing module `{module_id}`"))
        })?;
        if release.release_id != retained.value.module.release_id
            || release.module_digest != retained.value.module_digest
            || release.publisher != retained.value.module.publisher
        {
            return Err(WorkspaceError::ResolutionLockMismatch(format!(
                "release coordinate differs for `{module_id}`"
            )));
        }
    }

    let adopted = source
        .universe_lock
        .value
        .adopted_releases
        .iter()
        .map(|release| (release.module_id.clone(), release))
        .collect::<BTreeMap<_, _>>();
    if adopted.len() != source.universe_lock.value.adopted_releases.len() {
        return Err(WorkspaceError::UniverseLockMismatch(
            "adoption set contains a duplicate module".into(),
        ));
    }
    bounded("adopted modules", adopted.len(), MAX_WORKSPACE_MODULES)?;
    for (module_id, adoption) in &adopted {
        let release = resolved.get(module_id).ok_or_else(|| {
            WorkspaceError::UniverseLockMismatch(format!(
                "adopted module `{module_id}` is not resolved"
            ))
        })?;
        if adoption.release_id != release.release_id
            || adoption.module_digest != release.module_digest
        {
            return Err(WorkspaceError::UniverseLockMismatch(format!(
                "adoption coordinate differs for `{module_id}`"
            )));
        }
    }
    let adopted_modules = adopted.keys().cloned().collect::<BTreeSet<_>>();
    for module_id in &adopted_modules {
        for import in &source.modules[module_id].value.module.imports {
            if !adopted_modules.contains(&import.module_id) {
                return Err(WorkspaceError::UniverseLockMismatch(format!(
                    "adopted module `{module_id}` imports unadopted module `{}`",
                    import.module_id
                )));
            }
        }
    }

    let mut claimed_components = BTreeSet::new();
    for component in source.components.values() {
        validate_component_sets(&component.value)?;
    }
    for (module_id, module) in &source.modules {
        for reference in &module.value.module.source_components {
            let Some(component) = source.components.get(&reference.component_id) else {
                continue;
            };
            let component_digest = component.value.component_digest()?;
            if component.value.module_id != *module_id
                || component_digest != reference.component_digest
            {
                return Err(WorkspaceError::WorkspaceDigestMismatch {
                    kind: "semantic source component",
                    id: reference.component_id.to_string(),
                });
            }
            if !claimed_components.insert(reference.component_id.clone()) {
                return Err(WorkspaceError::ResolutionLockMismatch(format!(
                    "component `{}` belongs to more than one module",
                    reference.component_id
                )));
            }
        }
    }
    bounded(
        "source components",
        claimed_components.len(),
        MAX_WORKSPACE_COMPONENTS,
    )?;
    if claimed_components.len() != source.components.len() {
        return Err(WorkspaceError::ResolutionLockMismatch(
            "enumerated source component is not claimed by a module".into(),
        ));
    }
    Ok(adopted_modules)
}

fn validate_required_sources(
    source: &ParsedWorkspaceSource,
    required_modules: &BTreeSet<WorkspaceModuleId>,
) -> Result<(), WorkspaceError> {
    for module_id in required_modules {
        for reference in &source.modules[module_id].value.module.source_components {
            if !source.components.contains_key(&reference.component_id) {
                return Err(WorkspaceError::SourceDeliveryUnavailable {
                    module: module_id.to_string(),
                    component: reference.component_id.to_string(),
                });
            }
        }
    }
    Ok(())
}

fn validate_import_coordinate(
    source: &ParsedWorkspaceSource,
    import: &WorkspaceImportV1,
) -> Result<(), WorkspaceError> {
    let upstream = source.modules.get(&import.module_id).ok_or_else(|| {
        WorkspaceError::ResolutionLockMismatch(format!(
            "import references absent module `{}`",
            import.module_id
        ))
    })?;
    if import.release_id != upstream.value.module.release_id
        || import.module_digest != upstream.value.module_digest
        || import.publisher != upstream.value.module.publisher
    {
        return Err(WorkspaceError::ResolutionLockMismatch(format!(
            "import coordinate differs for `{}`",
            import.module_id
        )));
    }
    if import.paths.is_empty() {
        return Err(WorkspaceError::NamespaceResolution(format!(
            "import `{}` selects no namespace path",
            import.module_id
        )));
    }
    Ok(())
}

/// Return whether every non-local projection of `export` is currently accessible.
fn export_is_ready(
    module_id: &WorkspaceModuleId,
    export: &WorkspaceExportV1,
    access: &AccessMaps,
) -> bool {
    export.declarations.iter().all(|projection| {
        if projection.owner_module == *module_id {
            return true;
        }
        let canonical = QualifiedDeclarationRef {
            owner_module: projection.owner_module.clone(),
            local_ref: projection.declaration.declaration_ref(),
        };
        access[module_id]
            .get(&projection.local_ref)
            .is_some_and(|entry| entry.canonical == canonical)
    })
}

/// Resolve import/export projections to canonical declaration access.
fn resolve_access_maps(
    source: &ParsedWorkspaceSource,
    adopted_modules: &BTreeSet<WorkspaceModuleId>,
    mut access: AccessMaps,
    record_anchors: &mut BTreeMap<WorkspaceDerivationSubjectV1, WorkspaceAuthoringLocation>,
) -> Result<ResolvedAccessMaps, WorkspaceError> {
    let catalog = export_catalog(source, adopted_modules)?;
    let import_count = adopted_modules
        .iter()
        .map(|module_id| source.modules[module_id].value.module.imports.len())
        .sum();
    let path_count = adopted_modules
        .iter()
        .flat_map(|module_id| &source.modules[module_id].value.module.imports)
        .map(|import| import.paths.len())
        .sum();
    bounded("imports", import_count, MAX_WORKSPACE_IMPORTS)?;
    bounded("import paths", path_count, MAX_WORKSPACE_IMPORT_PATHS)?;
    for module_id in adopted_modules {
        for import in &source.modules[module_id].value.module.imports {
            validate_import_coordinate(source, import)?;
        }
    }
    let projection_count = catalog
        .values()
        .map(|export| export.declarations.len())
        .sum();
    bounded(
        "export declaration projections",
        projection_count,
        MAX_WORKSPACE_EXPORT_PROJECTION_ENTRIES,
    )?;

    let mut exports_by_access = BTreeMap::<AccessKey, BTreeSet<_>>::new();
    for ((module_id, export_id), export) in &catalog {
        for projection in &export.declarations {
            if projection.owner_module != *module_id {
                exports_by_access
                    .entry((module_id.clone(), projection.local_ref.clone()))
                    .or_default()
                    .insert((module_id.clone(), export_id.clone()));
            }
        }
    }
    let mut paths_by_export = BTreeMap::<_, BTreeSet<ImportPathKey>>::new();
    for module_id in adopted_modules {
        for (import_index, import) in source.modules[module_id]
            .value
            .module
            .imports
            .iter()
            .enumerate()
        {
            for (path_index, path) in import.paths.iter().enumerate() {
                paths_by_export
                    .entry((import.module_id.clone(), path.export_id.clone()))
                    .or_default()
                    .insert((module_id.clone(), import_index, path_index));
            }
        }
    }

    let mut resolved_exports = BTreeMap::new();
    let mut resolved_paths = BTreeSet::new();
    let mut ready_exports = catalog
        .iter()
        .filter(|((module_id, _), export)| export_is_ready(module_id, export, &access))
        .map(|(key, _)| key.clone())
        .collect::<BTreeSet<_>>();
    let mut ready_paths = BTreeSet::<ImportPathKey>::new();
    let mut access_entry_count = access.values().map(BTreeMap::len).sum();
    bounded(
        "resolved declaration access entries",
        access_entry_count,
        MAX_WORKSPACE_ACCESS_ENTRIES,
    )?;

    while !ready_exports.is_empty() || !ready_paths.is_empty() {
        let export_wave = std::mem::take(&mut ready_exports);
        for (module_id, export_id) in export_wave {
            if resolved_exports.contains_key(&(module_id.clone(), export_id.clone())) {
                continue;
            }
            let export = &catalog[&(module_id.clone(), export_id.clone())];
            let module_access = &access[&module_id];
            let Some(mut entries) = export
                .declarations
                .iter()
                .map(|projection| {
                    let canonical = QualifiedDeclarationRef {
                        owner_module: projection.owner_module.clone(),
                        local_ref: projection.declaration.declaration_ref(),
                    };
                    let access = if projection.owner_module == module_id {
                        Some(DeclarationAccess {
                            canonical,
                            propagation: WorkspacePropagationV1::Transitive,
                            policy: None,
                            origin: AccessOrigin::Local,
                        })
                    } else {
                        module_access
                            .get(&projection.local_ref)
                            .filter(|access| access.canonical == canonical)
                            .cloned()
                    };
                    access.map(|access| ResolvedExportDeclaration {
                        source_ref: projection.local_ref.clone(),
                        access,
                    })
                })
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            if entries.iter().any(|entry| {
                entry.access.origin != AccessOrigin::Local
                    && entry.access.propagation == WorkspacePropagationV1::Prevent
            }) {
                return Err(WorkspaceError::OnwardPropagationDenied(format!(
                    "prevented declaration in `{module_id}:{export_id}`"
                )));
            }
            for entry in &entries {
                if let Some(policy) = &entry.access.policy {
                    if !export_preserves_access_policy(export, policy)? {
                        return Err(WorkspaceError::PolicyWidening(format!(
                            "re-export `{module_id}:{export_id}` drops or widens an inherited policy"
                        )));
                    }
                }
            }
            let exported_policy = EffectiveAccessPolicy::from_export(export);
            for entry in &mut entries {
                entry.access.propagation = export.propagation;
                entry.access.policy = Some(exported_policy.clone());
            }
            resolved_exports.insert(
                (module_id.clone(), export_id.clone()),
                ResolvedExport {
                    surface: export.clone(),
                    declarations: entries,
                },
            );
            if let Some(paths) = paths_by_export.get(&(module_id, export_id)) {
                ready_paths.extend(
                    paths
                        .iter()
                        .filter(|path| !resolved_paths.contains(*path))
                        .cloned(),
                );
            }
        }

        let path_wave = std::mem::take(&mut ready_paths);
        for path_key in path_wave {
            if resolved_paths.contains(&path_key) {
                continue;
            }
            let (module_id, import_index, path_index) = path_key.clone();
            let import = &source.modules[&module_id].value.module.imports[import_index];
            let path = &import.paths[path_index];
            let Some(export) =
                resolved_exports.get(&(import.module_id.clone(), path.export_id.clone()))
            else {
                continue;
            };
            if export.surface.visibility == WorkspaceVisibilityV1::Private {
                return Err(WorkspaceError::VisibilityDenied(format!(
                    "private export `{}:{}`",
                    import.module_id, path.export_id
                )));
            }
            if export.surface.visibility == WorkspaceVisibilityV1::Protected
                && import.protected_delivery_authority.is_none()
            {
                return Err(WorkspaceError::VisibilityDenied(format!(
                    "protected export `{}:{}` declares no delivery-authority selector",
                    import.module_id, path.export_id
                )));
            }
            if !import
                .visibility_ceiling
                .is_narrower_or_equal(export.surface.visibility)
            {
                return Err(WorkspaceError::ImportVisibilityWidening {
                    source_label: source.modules[&module_id]
                        .source
                        .source_path
                        .as_str()
                        .to_owned(),
                    importer_module: module_id.clone(),
                    import_index,
                    upstream_module: import.module_id.clone(),
                    export_id: path.export_id.clone(),
                    export_visibility: export.surface.visibility,
                    import_ceiling: import.visibility_ceiling,
                });
            }
            if !import
                .propagation
                .is_narrower_or_equal(export.surface.propagation)
            {
                return Err(WorkspaceError::PolicyWidening(format!(
                    "propagation for `{}:{}`",
                    import.module_id, path.export_id
                )));
            }
            if let Some(axis) = import
                .edge_policy
                .widening_dimensions(&export.surface.ceiling)
                .into_iter()
                .next()
            {
                return Err(WorkspaceError::ImportPolicyWidening {
                    axis,
                    importer_module: module_id.clone(),
                    upstream_module: import.module_id.clone(),
                    release_id: import.release_id.clone(),
                    export_id: path.export_id.clone(),
                    source_namespace: Box::new(path.source_namespace.clone()),
                    local_namespace: Box::new(path.local_namespace.clone()),
                });
            }
            let selected = export
                .declarations
                .iter()
                .filter(|entry| entry.source_ref.namespace == path.source_namespace)
                .collect::<Vec<_>>();
            if selected.is_empty() {
                return Err(WorkspaceError::NamespaceResolution(format!(
                    "import path `{}:{}:{}` selects no declaration",
                    import.module_id, path.export_id, path.source_namespace
                )));
            }
            for entry in selected {
                let local_ref = WorkspaceDeclarationRefV1 {
                    namespace: path.local_namespace.clone(),
                    declaration_id: entry.source_ref.declaration_id.clone(),
                };
                let candidate = DeclarationAccess {
                    canonical: entry.access.canonical.clone(),
                    propagation: entry
                        .access
                        .propagation
                        .meet(export.surface.propagation)
                        .meet(import.propagation),
                    policy: entry.access.policy.clone().map(|mut policy| {
                        policy.visibility = policy.visibility.meet(import.visibility_ceiling);
                        policy.ceiling = policy.ceiling.meet(&import.edge_policy);
                        policy
                    }),
                    origin: AccessOrigin::Import {
                        upstream_module: import.module_id.clone(),
                        upstream_release: import.release_id.clone(),
                        export_id: path.export_id.clone(),
                        source_namespace: path.source_namespace.clone(),
                        local_namespace: path.local_namespace.clone(),
                    },
                };
                if access[&module_id]
                    .get(&local_ref)
                    .is_some_and(|existing| existing != &candidate)
                {
                    return Err(WorkspaceError::DuplicateLanguageIdentity {
                        kind: "import path",
                        id: format!("{module_id}:{local_ref}"),
                    });
                }
                let access_key = (module_id.clone(), local_ref.clone());
                if access
                    .get_mut(&module_id)
                    .expect("adopted access map")
                    .insert(local_ref, candidate)
                    .is_none()
                {
                    access_entry_count += 1;
                    bounded(
                        "resolved declaration access entries",
                        access_entry_count,
                        MAX_WORKSPACE_ACCESS_ENTRIES,
                    )?;
                    if let Some(exports) = exports_by_access.get(&access_key) {
                        ready_exports.extend(
                            exports
                                .iter()
                                .filter(|key| {
                                    !resolved_exports.contains_key(*key)
                                        && export_is_ready(&key.0, &catalog[*key], &access)
                                })
                                .cloned(),
                        );
                    }
                }
            }
            resolved_paths.insert(path_key);
        }
    }

    if resolved_exports.len() != catalog.len() {
        return Err(WorkspaceError::NamespaceResolution(
            "one or more exports reference an inaccessible declaration".into(),
        ));
    }
    if resolved_paths.len() != path_count {
        return Err(WorkspaceError::NamespaceResolution(
            "one or more import paths are unresolved".into(),
        ));
    }

    let mut surfaces = Vec::new();
    for module_id in adopted_modules {
        let retained = &source.modules[module_id];
        for (import_index, import) in retained.value.module.imports.iter().enumerate() {
            for (path_index, path) in import.paths.iter().enumerate() {
                let export = &resolved_exports[&(import.module_id.clone(), path.export_id.clone())];
                let mut policy = EffectiveAccessPolicy::from_export(&export.surface);
                policy.visibility = policy.visibility.meet(import.visibility_ceiling);
                policy.ceiling = policy.ceiling.meet(&import.edge_policy);
                let surface = ResolvedImportPath {
                    importer_module: module_id.clone(),
                    upstream_module: import.module_id.clone(),
                    upstream_release: import.release_id.clone(),
                    upstream_module_digest: import.module_digest.clone(),
                    upstream_publisher: import.publisher.clone(),
                    export_id: path.export_id.clone(),
                    source_namespace: path.source_namespace.clone(),
                    local_namespace: path.local_namespace.clone(),
                    export_visibility: export.surface.visibility,
                    effective_visibility: policy.visibility,
                    export_delivery: export.surface.delivery,
                    export_propagation: export.surface.propagation,
                    effective_propagation: export.surface.propagation.meet(import.propagation),
                    adoption_policy: import.adoption_policy,
                    protected_delivery_authority: import.protected_delivery_authority.clone(),
                    declarations: export
                        .declarations
                        .iter()
                        .filter(|entry| entry.source_ref.namespace == path.source_namespace)
                        .map(|entry| entry.access.canonical.clone())
                        .collect(),
                    export_ceiling: export.surface.ceiling.clone(),
                    import_ceiling: import.edge_policy.clone(),
                    effective_ceiling: policy.ceiling,
                    obligations: policy.obligations,
                    exclusive_authority: policy.exclusive_authority,
                };
                // Preserve the first record on a collision; the existing proof
                // sealer, not this sidecar, owns duplicate-subject rejection.
                record_anchors.entry(surface.subject()).or_insert_with(|| {
                    use AuthoringInstancePathSegment::{Field, Index};
                    WorkspaceAuthoringLocation::new(
                        retained.source.source_path.as_str().to_owned(),
                        vec![
                            Field("module".into()),
                            Field("imports".into()),
                            Index(import_index),
                            Field("paths".into()),
                            Index(path_index),
                        ],
                    )
                });
                surfaces.push(surface);
            }
        }
    }
    surfaces.sort_by(|left, right| {
        (
            &left.importer_module,
            &left.local_namespace,
            &left.upstream_module,
            &left.export_id,
            &left.source_namespace,
        )
            .cmp(&(
                &right.importer_module,
                &right.local_namespace,
                &right.upstream_module,
                &right.export_id,
                &right.source_namespace,
            ))
    });
    let access_convergence = AccessConvergenceWitness {
        export_rule_count: catalog.len(),
        import_path_rule_count: path_count,
        resolution_step_count: resolved_exports.len() + resolved_paths.len(),
    };
    debug_assert_eq!(
        access_convergence.resolution_step_count,
        access_convergence.export_rule_count + access_convergence.import_path_rule_count
    );
    Ok((access, resolved_exports, surfaces, access_convergence))
}

fn resolve_reference(
    module_id: &WorkspaceModuleId,
    source_ref: &WorkspaceDeclarationRefV1,
    access: &AccessMaps,
) -> Result<QualifiedDeclarationRef, WorkspaceError> {
    access[module_id]
        .get(source_ref)
        .map(|entry| entry.canonical.clone())
        .ok_or_else(|| {
            WorkspaceError::NamespaceResolution(format!(
                "module `{module_id}` cannot resolve `{source_ref}` through a local or selected import path"
            ))
        })
}

fn resolve_declaration(
    module_id: &WorkspaceModuleId,
    declaration: &WorkspaceDeclarationV1,
    access: &AccessMaps,
) -> Result<LinkedDeclaration, WorkspaceError> {
    let dependencies = declaration
        .dependencies()
        .iter()
        .map(|dependency| {
            Ok(LinkedDependency {
                target: resolve_reference(module_id, &dependency.target, access)?,
                kind: dependency.kind,
            })
        })
        .collect::<Result<Vec<_>, WorkspaceError>>()?;
    let (relationship_from, relationship_to) = match declaration {
        WorkspaceDeclarationV1::Relationship { from, to, .. } => (
            Some(resolve_reference(module_id, from, access)?),
            Some(resolve_reference(module_id, to, access)?),
        ),
        _ => (None, None),
    };
    Ok(LinkedDeclaration {
        declaration: declaration.clone(),
        dependencies,
        relationship_from,
        relationship_to,
    })
}

fn resolve_extensions(
    source: &ParsedWorkspaceSource,
    adopted_modules: &BTreeSet<WorkspaceModuleId>,
    access: &AccessMaps,
    declarations: &BTreeMap<QualifiedDeclarationRef, WorkspaceDeclarationV1>,
    record_anchors: &mut BTreeMap<WorkspaceDerivationSubjectV1, WorkspaceAuthoringLocation>,
) -> Result<Vec<LinkedExtension>, WorkspaceError> {
    let extension_count = adopted_modules
        .iter()
        .map(|module_id| source.modules[module_id].value.module.extensions.len())
        .sum();
    bounded("extensions", extension_count, MAX_WORKSPACE_EXTENSIONS)?;
    let mut extensions = Vec::new();
    for module_id in adopted_modules {
        let retained = &source.modules[module_id];
        for (extension_index, extension) in retained.value.module.extensions.iter().enumerate() {
            let target_ref = resolve_reference(module_id, &extension.target, access)?;
            let target = declarations.get(&target_ref).ok_or_else(|| {
                WorkspaceError::NamespaceResolution(format!(
                    "extension target `{}` is absent",
                    extension.target
                ))
            })?;
            if let Some(proposed) = &extension.narrowed_ceiling {
                let WorkspaceDeclarationV1::Policy { ceiling, .. } = target else {
                    return Err(WorkspaceError::PolicyWidening(extension.target.to_string()));
                };
                if !proposed.is_narrower_or_equal(ceiling) {
                    return Err(WorkspaceError::PolicyWidening(extension.target.to_string()));
                }
            }
            let added_traits = extension
                .added_traits
                .iter()
                .map(|trait_ref| resolve_reference(module_id, trait_ref, access))
                .collect::<Result<Vec<_>, _>>()?;
            for trait_ref in &added_traits {
                if !matches!(
                    declarations.get(trait_ref),
                    Some(WorkspaceDeclarationV1::Trait { .. })
                ) {
                    return Err(WorkspaceError::NamespaceResolution(format!(
                        "extension added-trait `{trait_ref}` is not a trait declaration"
                    )));
                }
            }
            let extension = LinkedExtension {
                owner_module: module_id.clone(),
                namespace: extension.namespace.clone(),
                target: target_ref,
                added_traits,
                narrowed_ceiling: extension.narrowed_ceiling.clone(),
                added_obligations: extension.added_obligations.clone(),
            };
            record_anchors
                .entry(extension.subject())
                .or_insert_with(|| {
                    use AuthoringInstancePathSegment::{Field, Index};
                    WorkspaceAuthoringLocation::new(
                        retained.source.source_path.as_str().to_owned(),
                        vec![
                            Field("module".into()),
                            Field("extensions".into()),
                            Index(extension_index),
                        ],
                    )
                });
            extensions.push(extension);
        }
    }
    extensions.sort_by(|left, right| {
        (&left.owner_module, &left.namespace, &left.target).cmp(&(
            &right.owner_module,
            &right.namespace,
            &right.target,
        ))
    });
    Ok(extensions)
}

/// Build and analyze the one canonical linked-declaration dependency graph.
///
/// Explicit dependencies retain their authored
/// [`DeclarationDependencyKindV1`]. Relationship endpoints are separate
/// positive roles so later derivation evidence can distinguish their meaning
/// without reconstructing edges from declaration payloads.
fn analyze_declaration_dependencies(
    declarations: &BTreeMap<QualifiedDeclarationRef, LinkedDeclaration>,
) -> Result<DeclarationDependencyAnalysis, WorkspaceError> {
    let direct_edges = declarations
        .iter()
        .flat_map(|(source, declaration)| {
            let explicit =
                declaration
                    .dependencies
                    .iter()
                    .map(|dependency| DeclarationDependencyEdge {
                        source: source.clone(),
                        target: dependency.target.clone(),
                        role: DeclarationEdgeRole::Explicit(dependency.kind),
                    });
            let relationship_from =
                declaration
                    .relationship_from
                    .iter()
                    .map(|target| DeclarationDependencyEdge {
                        source: source.clone(),
                        target: target.clone(),
                        role: DeclarationEdgeRole::RelationshipFrom,
                    });
            let relationship_to =
                declaration
                    .relationship_to
                    .iter()
                    .map(|target| DeclarationDependencyEdge {
                        source: source.clone(),
                        target: target.clone(),
                        role: DeclarationEdgeRole::RelationshipTo,
                    });
            explicit.chain(relationship_from).chain(relationship_to)
        })
        .collect::<Vec<_>>();

    DeclarationDependencyGraph::admit(declarations.keys().cloned(), direct_edges)?.analyze()
}

/// Consume parsed records and establish the exact adopted workspace graph.
pub(super) fn link_workspace(
    source: ParsedWorkspaceSource,
) -> Result<LinkedWorkspace, WorkspaceError> {
    if !source.module_sources.is_empty() {
        return Err(WorkspaceError::NamespaceResolution(
            "local module sources are not releases; use source-only lowering before preparing and admitting published modules".into(),
        ));
    }
    let responsibility_derivations = crate::composition::lower_responsibility_derivations(
        source
            .responsibility_derivations
            .values()
            .map(|retained| retained.value.clone())
            .collect(),
    )?;
    let organizational_translation = source
        .organizational_translation
        .as_ref()
        .map(|retained| {
            crate::composition::lower_organizational_translation(retained.value.clone())
        })
        .transpose()?;
    validate_directional_service_closure(
        &responsibility_derivations,
        organizational_translation.as_ref(),
    )?;
    let experience_composition = source
        .experience_composition
        .as_ref()
        .map_or_else(ExperienceCompositionDeclarationsV1::empty, |retained| {
            retained.value.declarations.clone()
        });
    let adopted_modules = validate_source_and_locks(&source)?;
    let source_modules = required_source_modules(&source, &adopted_modules)?;
    validate_required_sources(&source, &source_modules)?;
    let (mut semantic_declarations, mut access) =
        local_declarations(&source, &source_modules, &adopted_modules)?;
    add_export_projections(
        &source,
        &adopted_modules,
        &mut semantic_declarations,
        &mut access,
    )?;
    let mut record_anchors = BTreeMap::new();
    let (access, resolved_exports, resolved_imports, _access_convergence) =
        resolve_access_maps(&source, &adopted_modules, access, &mut record_anchors)?;
    validate_responsibility_module_reference_closure(
        &source,
        &responsibility_derivations,
        &adopted_modules,
        &resolved_exports,
        &resolved_imports,
    )?;
    let declarations = semantic_declarations
        .iter()
        .map(|(declaration_ref, declaration)| {
            Ok((
                declaration_ref.clone(),
                resolve_declaration(&declaration_ref.owner_module, declaration, &access)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, WorkspaceError>>()?;
    let extensions = resolve_extensions(
        &source,
        &adopted_modules,
        &access,
        &semantic_declarations,
        &mut record_anchors,
    )?;
    let dependency_analysis = analyze_declaration_dependencies(&declarations)?;
    let access_entries = access
        .iter()
        .flat_map(|(importer_module, entries)| {
            entries
                .iter()
                .map(move |(local_ref, access)| LinkedAccessEntry {
                    importer_module: importer_module.clone(),
                    local_ref: local_ref.clone(),
                    canonical: access.canonical.clone(),
                    propagation: access.propagation,
                    policy: access.policy.clone(),
                    origin: access.origin.clone(),
                })
        })
        .collect();
    let resources = adopted_modules
        .iter()
        .flat_map(|module_id| {
            source.modules[module_id]
                .value
                .module
                .resources
                .iter()
                .cloned()
                .map(|resource| (module_id.clone(), resource))
        })
        .collect::<Vec<_>>();
    bounded("resources", resources.len(), MAX_WORKSPACE_RESOURCES)?;
    project_context::validate_project_context(
        &source.manifest.workspace.desired_topology,
        &resources,
        &responsibility_derivations,
    )?;
    let package_occurrences = resources.iter().filter_map(|(_, resource)| match resource {
        WorkspaceResourceSelectionV1::Package { release, .. } => Some(release.clone()),
        _ => None,
    });
    let pack_occurrences = resources.iter().filter_map(|(_, resource)| match resource {
        WorkspaceResourceSelectionV1::NormativePack { release, .. } => Some(release.clone()),
        _ => None,
    });
    let release_closures =
        canonicalize_release_closures(package_occurrences.collect(), pack_occurrences.collect())?;
    Ok(LinkedWorkspace {
        source,
        record_anchors,
        responsibility_derivations,
        organizational_translation,
        experience_composition,
        adopted_modules,
        declarations,
        dependency_analysis,
        extensions,
        resolved_imports,
        access_entries,
        resources,
        package_releases: release_closures.packages,
        normative_pack_releases: release_closures.normative_packs,
    })
}

/// Require every supplied responsibility publisher and adopter coordinate to
/// resolve in the selected module, release, export, and import closure.
///
/// Semantic and adoption owners remain attributable owner assertions: this
/// linker validates only coordinates already owned by workspace admission and
/// introduces no owner, module, release, export, import, or namespace.
///
/// # Errors
///
/// Returns [`WorkspaceError::NamespaceResolution`] for the affected
/// responsibility when a supplied publisher or adopter coordinate does not
/// resolve exactly in the selected closure.
fn validate_responsibility_module_reference_closure(
    source: &ParsedWorkspaceSource,
    responsibilities: &ResponsibilityDerivationIrV1,
    adopted_modules: &BTreeSet<WorkspaceModuleId>,
    resolved_exports: &ResolvedExports,
    resolved_imports: &[ResolvedImportPath],
) -> Result<(), WorkspaceError> {
    for derivation in &responsibilities.derivations {
        if let AttributableAnswerV1::Supplied { value, .. } = &derivation.publisher_module {
            let publisher_resolves = adopted_modules.contains(&value.module_id)
                && source
                    .modules
                    .get(&value.module_id)
                    .is_some_and(|module| module.value.module.release_id == value.release_id)
                && resolved_exports
                    .contains_key(&(value.module_id.clone(), value.export_id.clone()));
            if !publisher_resolves {
                return Err(WorkspaceError::NamespaceResolution(format!(
                    "responsibility `{}` publisher module `{}` release `{}` export `{}` does not resolve in the selected adopted export closure",
                    derivation.derivation_id, value.module_id, value.release_id, value.export_id
                )));
            }
        }

        if let AttributableAnswerV1::Supplied { value, .. } = &derivation.adopter_module {
            let adopter_resolves = adopted_modules.contains(&value.module_id)
                && adopted_modules.contains(&value.imported_module_id)
                && resolved_imports.iter().any(|path| {
                    path.importer_module == value.module_id
                        && path.upstream_module == value.imported_module_id
                        && path.upstream_release == value.imported_release_id
                        && path.export_id == value.imported_export_id
                        && path.local_namespace == value.local_namespace
                });
            if !adopter_resolves {
                return Err(WorkspaceError::NamespaceResolution(format!(
                    "responsibility `{}` adopter module `{}` import of `{}` release `{}` export `{}` as namespace `{}` does not resolve in the selected import closure",
                    derivation.derivation_id,
                    value.module_id,
                    value.imported_module_id,
                    value.imported_release_id,
                    value.imported_export_id,
                    value.local_namespace
                )));
            }
        }
    }
    Ok(())
}

/// Require every directional responsibility to resolve in the selected organization source.
///
/// # Errors
///
/// Returns [`WorkspaceError::MissingDirectionalService`] for the affected
/// derivation when its exact qualified service coordinate is unavailable.
pub(super) fn validate_directional_service_closure(
    responsibilities: &ResponsibilityDerivationIrV1,
    organization: Option<&OrganizationalTranslationIrV1>,
) -> Result<(), WorkspaceError> {
    let services = organization
        .into_iter()
        .flat_map(|translation| translation.source.services.iter())
        .map(pale_core::records::DirectionalServiceContractV1::contract_ref)
        .collect::<BTreeSet<_>>();
    for derivation in &responsibilities.derivations {
        let AttributableAnswerV1::Supplied {
            value:
                ResponsibilityPlacementV1::DirectionalService {
                    service_contract, ..
                },
            ..
        } = &derivation.placement
        else {
            continue;
        };
        if !services.contains(service_contract) {
            return Err(WorkspaceError::MissingDirectionalService {
                derivation: derivation.derivation_id.clone(),
                contract: Box::new(service_contract.clone()),
            });
        }
    }
    Ok(())
}
