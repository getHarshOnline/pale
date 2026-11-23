//! Immutable workspace module, import, export, extension, and resource records.
//!
//! Sealing establishes canonical bytes and structural invariants, not publisher
//! verification, installation, activation, or native authority.

use super::*;

/// One content-bearing declaration projection in a named export.
///
/// [`Self::local_ref`] is the exporter-local name selected by import paths.
/// [`Self::owner_module`] and [`Self::declaration`] retain the canonical semantic
/// owner, allowing an interface consumer to compile without the owner's source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceExportProjectionV1 {
    /// Module that owns the projected declaration identity.
    pub owner_module: WorkspaceModuleId,
    /// Exporter-local reference selected by namespace paths.
    pub local_ref: WorkspaceDeclarationRefV1,
    /// Canonical semantic declaration delivered by this projection.
    pub declaration: WorkspaceDeclarationV1,
}

/// One named, self-contained module export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceExportV1 {
    /// Export identity scoped to the module release.
    pub export_id: WorkspaceExportId,
    /// Canonical declaration projections made available by this surface.
    #[schemars(schema_with = "export_projections_schema")]
    pub declarations: Vec<WorkspaceExportProjectionV1>,
    /// Visibility of the surface.
    pub visibility: WorkspaceVisibilityV1,
    /// Permitted delivery form.
    pub delivery: WorkspaceDeliveryV1,
    /// Next-hop propagation rule.
    pub propagation: WorkspacePropagationV1,
    /// Ceiling carried by this surface.
    pub ceiling: WorkspacePolicyCeilingV1,
    /// Obligations accumulated by consumers.
    #[schemars(schema_with = "obligation_ids_schema")]
    pub obligations: Vec<WorkspaceObligationId>,
    /// Exclusive authority claims carried by the surface.
    #[schemars(schema_with = "authority_claims_schema")]
    pub exclusive_authority: Vec<WorkspaceExclusiveAuthorityClaimV1>,
}

/// One exact namespace path through a selected upstream export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceImportPathV1 {
    /// Named export selected from the upstream release.
    pub export_id: WorkspaceExportId,
    /// Namespace selected inside that export.
    pub source_namespace: WorkspaceNamespaceId,
    /// Importer-local namespace used by declaration references.
    pub local_namespace: WorkspaceNamespaceId,
}

/// Exact import of named namespace paths from one immutable upstream release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceImportV1 {
    /// Upstream module identity.
    pub module_id: WorkspaceModuleId,
    /// Exact upstream release.
    pub release_id: WorkspaceReleaseId,
    /// Exact upstream module digest.
    pub module_digest: WorkspaceModuleDigest,
    /// Upstream publisher reference.
    pub publisher: WorkspacePublisherRefV1,
    /// Exact export and namespace paths admitted from that release.
    #[schemars(schema_with = "import_paths_schema")]
    pub paths: Vec<WorkspaceImportPathV1>,
    /// Declared selector for separately proved protected-delivery authority.
    pub protected_delivery_authority: Option<WorkspaceResourceId>,
    /// Local visibility narrowing; never a widening.
    pub visibility_ceiling: WorkspaceVisibilityV1,
    /// Local propagation narrowing.
    pub propagation: WorkspacePropagationV1,
    /// Edge-local policy ceiling; every dimension must equal or narrow the selected export.
    pub edge_policy: WorkspacePolicyCeilingV1,
    /// Consumer successor-adoption policy.
    pub adoption_policy: WorkspaceAdoptionPolicyV1,
}

/// Typed extension or narrowing of an admitted declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceExtensionV1 {
    /// Imported or local declaration being extended.
    pub target: WorkspaceDeclarationRefV1,
    /// Namespace that owns the extension.
    pub namespace: WorkspaceNamespaceId,
    /// Additional trait declarations; identities remain namespaced.
    #[schemars(schema_with = "extension_traits_schema")]
    pub added_traits: Vec<WorkspaceDeclarationRefV1>,
    /// Optional strictly equal-or-narrower ceiling.
    pub narrowed_ceiling: Option<WorkspacePolicyCeilingV1>,
    /// Additional obligations; existing obligations cannot be removed.
    #[schemars(schema_with = "obligation_ids_schema")]
    pub added_obligations: Vec<WorkspaceObligationId>,
}

/// Logical resource selection made by one module.
///
/// Package and Normative-Pack variants require exact release coordinates. The
/// other variants remain logical references because their observed native truth
/// belongs to the named authority resolver, not to this desired-state record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkspaceResourceSelectionV1 {
    /// Principal-owned logical resource.
    Principal {
        /// Workspace-local selection identity.
        resource_id: WorkspaceResourceId,
        /// Declared logical resolver; this is not native authorization proof.
        authority_resolver: WorkspaceResourceId,
    },
    /// Independently constituted organization reference.
    Organization {
        /// Workspace-local selection identity.
        resource_id: WorkspaceResourceId,
        /// Declared logical resolver; this is not native authorization proof.
        authority_resolver: WorkspaceResourceId,
    },
    /// Native application contract reference.
    NativeApplication {
        /// Workspace-local selection identity.
        resource_id: WorkspaceResourceId,
        /// Declared logical resolver; this is not native authorization proof.
        authority_resolver: WorkspaceResourceId,
    },
    /// Participant or responsibility-provider reference.
    Participant {
        /// Workspace-local selection identity.
        resource_id: WorkspaceResourceId,
        /// Declared logical resolver; this is not native authorization proof.
        authority_resolver: WorkspaceResourceId,
    },
    /// Exact Pale Package release reference.
    Package {
        /// Workspace-local selection identity.
        resource_id: WorkspaceResourceId,
        /// Exact Package coordinate; no Package body or semantic graph is copied.
        release: PackageReleaseV1,
    },
    /// Exact owner-attested Normative-Pack release reference.
    NormativePack {
        /// Workspace-local selection identity.
        resource_id: WorkspaceResourceId,
        /// Exact Pack coordinate; no Pack facts, rights, or source are copied.
        release: NormativePackReleaseV1,
    },
}

impl WorkspaceResourceSelectionV1 {
    /// Borrow the workspace-local selection identity.
    #[must_use]
    pub fn resource_id(&self) -> &WorkspaceResourceId {
        match self {
            Self::Principal { resource_id, .. }
            | Self::Organization { resource_id, .. }
            | Self::NativeApplication { resource_id, .. }
            | Self::Participant { resource_id, .. }
            | Self::Package { resource_id, .. }
            | Self::NormativePack { resource_id, .. } => resource_id,
        }
    }

    /// Borrow the declared logical authority resolver.
    #[must_use]
    pub fn authority_resolver(&self) -> Option<&WorkspaceResourceId> {
        match self {
            Self::Principal {
                authority_resolver, ..
            }
            | Self::Organization {
                authority_resolver, ..
            }
            | Self::NativeApplication {
                authority_resolver, ..
            }
            | Self::Participant {
                authority_resolver, ..
            } => Some(authority_resolver),
            Self::Package { .. } | Self::NormativePack { .. } => None,
        }
    }

    /// Return the closed resource class carried by this selection.
    #[must_use]
    pub const fn kind(&self) -> WorkspaceResourceKindV1 {
        match self {
            Self::Principal { .. } => WorkspaceResourceKindV1::Principal,
            Self::Organization { .. } => WorkspaceResourceKindV1::Organization,
            Self::NativeApplication { .. } => WorkspaceResourceKindV1::NativeApplication,
            Self::Participant { .. } => WorkspaceResourceKindV1::Participant,
            Self::Package { .. } => WorkspaceResourceKindV1::Package,
            Self::NormativePack { .. } => WorkspaceResourceKindV1::NormativePack,
        }
    }
}

/// Preimage whose canonical identity seals one module candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceModulePreimageV1 {
    /// Stable module identity.
    pub module_id: WorkspaceModuleId,
    /// Immutable release identity.
    pub release_id: WorkspaceReleaseId,
    /// Declared publisher and delegated release-key identifiers.
    pub publisher: WorkspacePublisherRefV1,
    /// Content-addressed source components in the release.
    #[schemars(schema_with = "source_components_schema")]
    pub source_components: Vec<WorkspaceSourceComponentRefV1>,
    /// Exact imports.
    #[schemars(schema_with = "imports_schema")]
    pub imports: Vec<WorkspaceImportV1>,
    /// Named exports.
    #[schemars(schema_with = "exports_schema")]
    pub exports: Vec<WorkspaceExportV1>,
    /// Typed extensions and narrowings.
    #[schemars(schema_with = "extensions_schema")]
    pub extensions: Vec<WorkspaceExtensionV1>,
    /// Logical resources selected by the module.
    #[schemars(schema_with = "resources_schema")]
    pub resources: Vec<WorkspaceResourceSelectionV1>,
}

/// Sealed portable module candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceModuleV1 {
    /// Stable wire schema identity.
    #[schemars(extend("const" = "pale/workspace-module.v1"))]
    pub schema: String,
    /// Initial contract version.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Candidate content excluding this digest.
    pub module: WorkspaceModulePreimageV1,
    /// Domain-separated canonical identity of [`Self::module`].
    pub module_digest: WorkspaceModuleDigest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceModuleWireV1 {
    schema: String,
    schema_version: u32,
    module: WorkspaceModulePreimageV1,
    module_digest: WorkspaceModuleDigest,
}

impl<'de> Deserialize<'de> for WorkspaceModuleV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = WorkspaceModuleWireV1::deserialize(deserializer)?;
        let value = Self {
            schema: wire.schema,
            schema_version: wire.schema_version,
            module: wire.module,
            module_digest: wire.module_digest,
        };
        value.verify().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

impl WorkspaceModuleV1 {
    /// Seal a pure candidate module without performing I/O or publication.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspaceModule`] when a logical set repeats an
    /// identity, a schema-advertised collection bound is exceeded, or canonical
    /// encoding fails. Set-like inputs are sorted before hashing.
    pub fn seal(mut module: WorkspaceModulePreimageV1) -> Result<Self, InvalidWorkspaceModule> {
        canonicalize_module(&mut module)?;
        validate_workspace_module("pale/workspace-module.v1", 1, &module)?;
        let module_digest =
            WorkspaceModuleDigest::hash_canonical(&("pale.workspace-module.v1", &module))
                .map_err(InvalidWorkspaceModule::Canonical)?;
        Ok(Self {
            schema: "pale/workspace-module.v1".into(),
            schema_version: 1,
            module,
            module_digest,
        })
    }

    /// Recompute and compare the domain-separated candidate identity.
    ///
    /// This preimage check assumes the record already passed wire/schema admission.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspaceModule`] if the retained preimage is not one
    /// canonical unique and bounded logical-set representation, its claimed
    /// identity disagrees, or canonical encoding fails.
    pub fn verify(&self) -> Result<(), InvalidWorkspaceModule> {
        validate_workspace_module(&self.schema, self.schema_version, &self.module)?;
        let observed =
            WorkspaceModuleDigest::hash_canonical(&("pale.workspace-module.v1", &self.module))
                .map_err(InvalidWorkspaceModule::Canonical)?;
        if observed == self.module_digest {
            Ok(())
        } else {
            Err(InvalidWorkspaceModule::DigestMismatch)
        }
    }

    /// Render this sealed candidate's canonical RFC 8785 representation.
    ///
    /// # Errors
    ///
    /// Returns a canonicalization error when the record is not representable.
    pub fn canonical_candidate(&self) -> Result<String, crate::canon::CanonError> {
        crate::canon::to_canonical_string(self)
    }
}

/// A workspace module is duplicated, noncanonical, unencodable, or missealed.
#[derive(Debug, thiserror::Error)]
pub enum InvalidWorkspaceModule {
    /// The record does not carry the exact initial-v1 schema coordinate.
    #[error("workspace module schema coordinate is not initial-v1")]
    SchemaCoordinate,
    /// Canonical encoding failed.
    #[error("workspace module canonicalization failed: {0}")]
    Canonical(#[source] crate::canon::CanonError),
    /// A logical set repeats one identity.
    #[error("workspace module repeats {kind} identity `{id}`")]
    Duplicate {
        /// Closed logical set containing the duplicate.
        kind: &'static str,
        /// Repeated logical identity.
        id: String,
    },
    /// A bounded logical set exceeds its initial-v1 wire ceiling.
    #[error("workspace module {kind} count {observed} exceeds {limit}")]
    Limit {
        /// Closed logical set whose bound was exceeded.
        kind: &'static str,
        /// Observed element count.
        observed: usize,
        /// Maximum admitted element count.
        limit: usize,
    },
    /// A projected or exported policy cannot form one bounded lawful set.
    #[error("workspace module policy is invalid: {0}")]
    PolicyCombination(#[source] WorkspacePolicyCombinationError),
    /// A logical-set vector is not in its canonical order.
    #[error("workspace module logical sets are not canonically ordered")]
    NonCanonical,
    /// Claimed and recomputed digest differ.
    #[error("workspace module digest does not match its canonical preimage")]
    DigestMismatch,
}

fn validate_workspace_module(
    schema: &str,
    schema_version: u32,
    module: &WorkspaceModulePreimageV1,
) -> Result<(), InvalidWorkspaceModule> {
    if schema != "pale/workspace-module.v1" || schema_version != 1 {
        return Err(InvalidWorkspaceModule::SchemaCoordinate);
    }
    let mut canonical = module.clone();
    canonicalize_module(&mut canonical)?;
    validate_module_sets(&canonical)?;
    if canonical != *module {
        return Err(InvalidWorkspaceModule::NonCanonical);
    }
    Ok(())
}

fn canonicalize_module(
    module: &mut WorkspaceModulePreimageV1,
) -> Result<(), InvalidWorkspaceModule> {
    canonicalize_module_sets(
        &mut module.source_components,
        &mut module.imports,
        &mut module.exports,
        &mut module.extensions,
        &mut module.resources,
    )
}

pub(super) fn canonicalize_module_sets(
    source_components: &mut [WorkspaceSourceComponentRefV1],
    imports: &mut [WorkspaceImportV1],
    exports: &mut [WorkspaceExportV1],
    extensions: &mut [WorkspaceExtensionV1],
    resources: &mut [WorkspaceResourceSelectionV1],
) -> Result<(), InvalidWorkspaceModule> {
    source_components.sort_by(|left, right| left.component_id.cmp(&right.component_id));
    for import in imports.iter_mut() {
        import
            .edge_policy
            .canonicalize_policy_sets()
            .map_err(workspace_policy_error)?;
        import.paths.sort_by(|left, right| {
            (
                &left.local_namespace,
                &left.export_id,
                &left.source_namespace,
            )
                .cmp(&(
                    &right.local_namespace,
                    &right.export_id,
                    &right.source_namespace,
                ))
        });
    }
    imports.sort_by(|left, right| {
        (&left.module_id, &left.release_id, &left.module_digest).cmp(&(
            &right.module_id,
            &right.release_id,
            &right.module_digest,
        ))
    });
    for export in exports.iter_mut() {
        export
            .ceiling
            .canonicalize_policy_sets()
            .map_err(workspace_policy_error)?;
        for projection in &mut export.declarations {
            canonicalize_declaration(&mut projection.declaration);
        }
        export.declarations.sort_by(|left, right| {
            (
                &left.local_ref,
                &left.owner_module,
                left.declaration.declaration_ref(),
            )
                .cmp(&(
                    &right.local_ref,
                    &right.owner_module,
                    right.declaration.declaration_ref(),
                ))
        });
        export.obligations.sort();
        export.exclusive_authority.sort_by(|left, right| {
            (&left.scope, &left.authority_resolver).cmp(&(&right.scope, &right.authority_resolver))
        });
    }
    exports.sort_by(|left, right| left.export_id.cmp(&right.export_id));
    for extension in extensions.iter_mut() {
        if let Some(ceiling) = &mut extension.narrowed_ceiling {
            ceiling
                .canonicalize_policy_sets()
                .map_err(workspace_policy_error)?;
        }
        extension.added_traits.sort();
        extension.added_obligations.sort();
    }
    extensions.sort_by(|left, right| {
        (&left.namespace, &left.target).cmp(&(&right.namespace, &right.target))
    });
    resources.sort_by(|left, right| left.resource_id().cmp(right.resource_id()));
    Ok(())
}

fn validate_module_sets(module: &WorkspaceModulePreimageV1) -> Result<(), InvalidWorkspaceModule> {
    validate_module_content_sets(
        &module.source_components,
        &module.imports,
        &module.exports,
        &module.extensions,
        &module.resources,
    )
}

pub(super) fn validate_module_content_sets(
    source_components: &[WorkspaceSourceComponentRefV1],
    imports: &[WorkspaceImportV1],
    exports: &[WorkspaceExportV1],
    extensions: &[WorkspaceExtensionV1],
    resources: &[WorkspaceResourceSelectionV1],
) -> Result<(), InvalidWorkspaceModule> {
    bounded_module_len(
        "source component",
        source_components.len(),
        MAX_WORKSPACE_COMPONENTS,
    )?;
    bounded_module_len("import", imports.len(), MAX_WORKSPACE_IMPORTS)?;
    bounded_module_len("export", exports.len(), MAX_WORKSPACE_EXPORTS)?;
    bounded_module_len("extension", extensions.len(), MAX_WORKSPACE_EXTENSIONS)?;
    bounded_module_len("resource", resources.len(), MAX_WORKSPACE_RESOURCES)?;
    reject_adjacent_duplicate(
        source_components.iter().map(|item| &item.component_id),
        "component",
    )?;
    reject_adjacent_duplicate(
        imports.iter().map(|item| &item.module_id),
        "imported module",
    )?;
    for import in imports {
        bounded_module_len(
            "import path",
            import.paths.len(),
            MAX_WORKSPACE_IMPORT_PATHS,
        )?;
        reject_adjacent_duplicate(
            import.paths.iter().map(|path| {
                (
                    &path.local_namespace,
                    &path.export_id,
                    &path.source_namespace,
                )
            }),
            "import path",
        )?;
    }
    reject_adjacent_duplicate(exports.iter().map(|item| &item.export_id), "export")?;
    for export in exports {
        bounded_module_len(
            "export projection",
            export.declarations.len(),
            MAX_WORKSPACE_EXPORT_PROJECTIONS,
        )?;
        WorkspaceObligationId::try_union_sets(&[], &export.obligations)
            .map_err(InvalidWorkspaceModule::PolicyCombination)?;
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&[], &export.exclusive_authority)
            .map_err(InvalidWorkspaceModule::PolicyCombination)?;
        for projection in &export.declarations {
            validate_declaration(&projection.declaration).map_err(module_declaration_error)?;
        }
        reject_adjacent_duplicate(
            export.declarations.iter().map(|item| &item.local_ref),
            "export declaration",
        )?;
        reject_adjacent_duplicate(export.obligations.iter(), "export obligation")?;
        reject_adjacent_duplicate(
            export
                .exclusive_authority
                .iter()
                .map(|claim| (&claim.scope, &claim.authority_resolver)),
            "export authority claim",
        )?;
    }
    for extension in extensions {
        bounded_module_len(
            "extension trait",
            extension.added_traits.len(),
            MAX_WORKSPACE_EXTENSION_TRAITS,
        )?;
        bounded_module_len(
            "extension obligation",
            extension.added_obligations.len(),
            MAX_WORKSPACE_OBLIGATIONS,
        )?;
        reject_adjacent_duplicate(extension.added_traits.iter(), "extension trait")?;
        reject_adjacent_duplicate(extension.added_obligations.iter(), "extension obligation")?;
    }
    reject_adjacent_duplicate(
        extensions
            .iter()
            .map(|extension| (&extension.namespace, &extension.target)),
        "extension",
    )?;
    reject_adjacent_duplicate(
        resources
            .iter()
            .map(WorkspaceResourceSelectionV1::resource_id),
        "resource",
    )?;
    Ok(())
}

fn workspace_policy_error(error: InvalidWorkspacePolicyCeilingV1) -> InvalidWorkspaceModule {
    match error {
        InvalidWorkspacePolicyCeilingV1::Limit {
            axis,
            observed,
            limit,
        } => InvalidWorkspaceModule::Limit {
            kind: axis.as_str(),
            observed,
            limit,
        },
        InvalidWorkspacePolicyCeilingV1::Duplicate { axis, member } => {
            InvalidWorkspaceModule::Duplicate {
                kind: axis.as_str(),
                id: member,
            }
        }
    }
}

fn bounded_module_len(
    kind: &'static str,
    observed: usize,
    limit: usize,
) -> Result<(), InvalidWorkspaceModule> {
    if observed > limit {
        Err(InvalidWorkspaceModule::Limit {
            kind,
            observed,
            limit,
        })
    } else {
        Ok(())
    }
}

fn reject_adjacent_duplicate<T: Ord + std::fmt::Debug>(
    mut values: impl Iterator<Item = T>,
    kind: &'static str,
) -> Result<(), InvalidWorkspaceModule> {
    let Some(mut prior) = values.next() else {
        return Ok(());
    };
    for value in values {
        if prior == value {
            return Err(InvalidWorkspaceModule::Duplicate {
                kind,
                id: format!("{value:?}"),
            });
        }
        prior = value;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/internal/records/workspace_language/module.rs"]
mod checked_deserialization_tests;
