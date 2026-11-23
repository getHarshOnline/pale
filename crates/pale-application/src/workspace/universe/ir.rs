//! Path-free workspace semantic IR construction and verification.

use super::admission::{admission_requirements_from, WorkspaceAdmissionRequirements};
use super::derivation::{derivation_frontier_from, WorkspaceDerivationFrontier};
use super::*;

/// Canonical, path-free semantic preimage for one desired workspace universe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct WorkspaceSemanticIrPreimage {
    pub(super) domain: &'static str,
    pub(super) workspace_id: WorkspaceId,
    pub(super) adopted_releases: Vec<WorkspaceAdoptedReleaseV1>,
    pub(super) responsibility_derivations: ResponsibilityDerivationIrV1,
    pub(super) organizational_translation: Option<OrganizationalTranslationIrV1>,
    pub(super) experience_composition: ExperienceCompositionDeclarationsV1,
    pub(super) declarations: Vec<WorkspaceSemanticDeclarationEntry>,
    pub(super) extensions: Vec<LinkedExtension>,
    pub(super) resolved_imports: Vec<ResolvedImportPath>,
    pub(super) access_entries: Vec<LinkedAccessEntry>,
    pub(super) resources: Vec<WorkspaceSemanticResourceOccurrence>,
    pub(super) package_releases: Vec<PackageReleaseV1>,
    pub(super) normative_pack_releases: Vec<NormativePackReleaseV1>,
    pub(super) exclusive_authority: Vec<pale_core::records::WorkspaceExclusiveAuthorityClaimV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct WorkspaceSemanticResourceOccurrence {
    owner_module: WorkspaceModuleId,
    #[serde(flatten)]
    selection: WorkspaceSemanticResourceSelection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WorkspaceSemanticResourceSelection {
    Principal {
        resource_id: WorkspaceResourceId,
        authority_resolver: WorkspaceResourceId,
    },
    Organization {
        resource_id: WorkspaceResourceId,
        authority_resolver: WorkspaceResourceId,
    },
    NativeApplication {
        resource_id: WorkspaceResourceId,
        authority_resolver: WorkspaceResourceId,
    },
    Participant {
        resource_id: WorkspaceResourceId,
        authority_resolver: WorkspaceResourceId,
    },
    Package {
        resource_id: WorkspaceResourceId,
        package_id: PackageIdV1,
    },
    NormativePack {
        resource_id: WorkspaceResourceId,
        attestor: NormativePackAttestorV1,
        pack_id: NormativePackIdV1,
    },
}

fn semantic_resource_occurrences(
    resources: Vec<(
        WorkspaceModuleId,
        pale_core::records::WorkspaceResourceSelectionV1,
    )>,
) -> Vec<WorkspaceSemanticResourceOccurrence> {
    use pale_core::records::WorkspaceResourceSelectionV1 as Source;
    resources
        .into_iter()
        .map(|(owner_module, source)| {
            let selection = match source {
                Source::Principal {
                    resource_id,
                    authority_resolver,
                } => WorkspaceSemanticResourceSelection::Principal {
                    resource_id,
                    authority_resolver,
                },
                Source::Organization {
                    resource_id,
                    authority_resolver,
                } => WorkspaceSemanticResourceSelection::Organization {
                    resource_id,
                    authority_resolver,
                },
                Source::NativeApplication {
                    resource_id,
                    authority_resolver,
                } => WorkspaceSemanticResourceSelection::NativeApplication {
                    resource_id,
                    authority_resolver,
                },
                Source::Participant {
                    resource_id,
                    authority_resolver,
                } => WorkspaceSemanticResourceSelection::Participant {
                    resource_id,
                    authority_resolver,
                },
                Source::Package {
                    resource_id,
                    release,
                } => WorkspaceSemanticResourceSelection::Package {
                    resource_id,
                    package_id: release.id,
                },
                Source::NormativePack {
                    resource_id,
                    release,
                } => WorkspaceSemanticResourceSelection::NormativePack {
                    resource_id,
                    attestor: release.attestor,
                    pack_id: release.id,
                },
            };
            WorkspaceSemanticResourceOccurrence {
                owner_module,
                selection,
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// One declaration and effective policy retained in semantic IR.
pub(crate) struct WorkspaceSemanticDeclarationEntry {
    declaration_ref: QualifiedDeclarationRef,
    declaration: LinkedDeclaration,
    policy: EffectiveDeclarationPolicy,
}

/// Closed role of one resident workspace dependency edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ResidentWorkspaceDependencyKind {
    /// Explicit dependency with its authored dependency kind.
    Explicit(DeclarationDependencyKindV1),
    /// Relationship source endpoint consumed by its declaration.
    RelationshipFrom,
    /// Relationship destination endpoint consumed by its declaration.
    RelationshipTo,
}

/// One owner-qualified dependency projected from an admitted declaration body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResidentWorkspaceDependency {
    pub(crate) owner_module: WorkspaceModuleId,
    pub(crate) declaration: WorkspaceDeclarationRefV1,
    pub(crate) kind: ResidentWorkspaceDependencyKind,
}

impl WorkspaceSemanticDeclarationEntry {
    pub(crate) fn owner_module(&self) -> &WorkspaceModuleId {
        &self.declaration_ref.owner_module
    }

    pub(crate) fn declaration_ref(&self) -> &WorkspaceDeclarationRefV1 {
        &self.declaration_ref.local_ref
    }

    pub(crate) fn resident_dependencies(&self) -> Vec<ResidentWorkspaceDependency> {
        let explicit =
            self.declaration
                .dependencies
                .iter()
                .map(|dependency| ResidentWorkspaceDependency {
                    owner_module: dependency.target.owner_module.clone(),
                    declaration: dependency.target.local_ref.clone(),
                    kind: ResidentWorkspaceDependencyKind::Explicit(dependency.kind),
                });
        let from =
            self.declaration
                .relationship_from
                .iter()
                .map(|target| ResidentWorkspaceDependency {
                    owner_module: target.owner_module.clone(),
                    declaration: target.local_ref.clone(),
                    kind: ResidentWorkspaceDependencyKind::RelationshipFrom,
                });
        let to =
            self.declaration
                .relationship_to
                .iter()
                .map(|target| ResidentWorkspaceDependency {
                    owner_module: target.owner_module.clone(),
                    declaration: target.local_ref.clone(),
                    kind: ResidentWorkspaceDependencyKind::RelationshipTo,
                });
        explicit.chain(from).chain(to).collect()
    }
}

/// Sealed but not yet reverified workspace semantic IR.
#[derive(Debug)]
pub(super) struct WorkspaceSemanticIr {
    pub(super) preimage: WorkspaceSemanticIrPreimage,
    pub(super) digest: WorkspaceSemanticIrDigest,
    pub(super) derivation_frontier: WorkspaceDerivationFrontier,
    pub(super) admission_requirements: WorkspaceAdmissionRequirements,
    source_custody: super::super::source::ParsedWorkspaceSource,
}

/// Consuming proof that [`WorkspaceSemanticIr`] passed identity verification.
#[derive(Debug)]
pub(in crate::workspace) struct VerifiedWorkspaceSemanticIr {
    pub(super) preimage: WorkspaceSemanticIrPreimage,
    pub(super) digest: WorkspaceSemanticIrDigest,
    pub(super) derivation_frontier: WorkspaceDerivationFrontier,
    pub(super) admission_requirements: WorkspaceAdmissionRequirements,
    pub(super) source_custody: super::super::source::ParsedWorkspaceSource,
}

/// Verified path-free workspace semantics after source-custody release.
#[derive(Debug)]
pub(super) struct AdmittedWorkspaceSemanticIr {
    pub(super) preimage: WorkspaceSemanticIrPreimage,
    pub(super) digest: WorkspaceSemanticIrDigest,
    pub(super) derivation_frontier: WorkspaceDerivationFrontier,
}

impl WorkspaceSemanticIr {
    /// Seal normalized desired-state semantics without admitting a publisher.
    pub(super) fn seal(normalized: NormalizedWorkspace) -> Result<Self, WorkspaceError> {
        let admission_requirements = admission_requirements_from(&normalized)?;
        let derivation_builder = derivation_frontier_from(&normalized)?;
        let NormalizedWorkspace {
            linked,
            mut policies,
            exclusive_authority,
        } = normalized;
        let super::super::link::LinkedWorkspace {
            source,
            responsibility_derivations,
            organizational_translation,
            experience_composition,
            declarations,
            dependency_analysis: _,
            extensions,
            resolved_imports,
            access_entries,
            resources,
            package_releases,
            normative_pack_releases,
            ..
        } = linked;
        let workspace_id = source.manifest.workspace.workspace_id.clone();
        let adopted_releases = source.universe_lock.value.adopted_releases.clone();
        let preimage = WorkspaceSemanticIrPreimage {
            domain: "pale.workspace-semantic-ir.v1",
            workspace_id,
            adopted_releases,
            responsibility_derivations,
            organizational_translation,
            experience_composition,
            declarations: declarations
                .into_iter()
                .map(
                    |(declaration_ref, declaration)| WorkspaceSemanticDeclarationEntry {
                        policy: policies
                            .remove(&declaration_ref)
                            .expect("normalization assigns every linked declaration exactly once"),
                        declaration_ref,
                        declaration,
                    },
                )
                .collect(),
            extensions,
            resolved_imports,
            access_entries,
            resources: semantic_resource_occurrences(resources),
            package_releases,
            normative_pack_releases,
            exclusive_authority: exclusive_authority.into_values().collect(),
        };
        let digest = WorkspaceSemanticIrDigest::hash_canonical(&preimage)?;
        let derivation_frontier = derivation_builder.seal(&digest)?;
        Ok(Self {
            preimage,
            digest,
            derivation_frontier,
            admission_requirements,
            source_custody: source,
        })
    }

    /// Consume and reverify the sealed IR before publisher admission.
    pub(super) fn verify(self) -> Result<VerifiedWorkspaceSemanticIr, WorkspaceError> {
        let observed = WorkspaceSemanticIrDigest::hash_canonical(&self.preimage)?;
        if observed != self.digest {
            return Err(WorkspaceError::WorkspaceSemanticIrDigestMismatch(
                self.preimage.workspace_id.to_string(),
            ));
        }
        Ok(VerifiedWorkspaceSemanticIr {
            preimage: self.preimage,
            digest: self.digest,
            derivation_frontier: self.derivation_frontier,
            admission_requirements: self.admission_requirements,
            source_custody: self.source_custody,
        })
    }
}

/// Seal and verify one normalized workspace, stopping before publisher admission.
pub(in crate::workspace) fn seal_and_verify_workspace(
    normalized: NormalizedWorkspace,
) -> Result<VerifiedWorkspaceSemanticIr, WorkspaceError> {
    WorkspaceSemanticIr::seal(normalized)?.verify()
}
