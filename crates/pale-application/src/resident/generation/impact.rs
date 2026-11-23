//! Typed successor input changes and semantic impact witnesses.

use pale_core::records::{
    ArtifactDigest, ExperienceCompositionPlanDigest, InstanceDigest, LogicalRepositoryIdV1,
    ManifestDigest, MaterializationLineageDigest, MaterializationStateDigest,
    PackageSemanticIrDigest, PrincipalAdmittedUniverseDigest, ProfileDigest, ReceiptDigest,
    RepositorySourceFrontierV1, WorkspaceAdmissionEvidenceDigest, WorkspaceDerivationRootChangeV1,
    WorkspaceDerivationRootDigest, WorkspaceDerivationSubjectV1, WorkspaceSemanticIrDigest,
};

use crate::resident::definition::ResidentRelation;

use super::{ResidentDefinitionCoordinate, ResidentRelationshipEvidenceCoordinate};

/// Exact graph-derived invalidation proof for one admitted successor.
#[derive(Debug, PartialEq, Eq)]
pub struct GenerationSuccessorImpact {
    /// Canonically ordered exact input changes that seeded this closure.
    pub(in crate::resident) input_changes: Vec<GenerationInputChange>,
    /// Canonically ordered affected semantic definitions.
    pub(in crate::resident) affected: Vec<ResidentDefinitionCoordinate>,
    /// Exact unaffected complement in the successor generation.
    pub(in crate::resident) unaffected: Vec<ResidentDefinitionCoordinate>,
    /// Compact graph-owned reachability witness used for on-demand reasons.
    pub(in crate::resident) closure: pale_kernel::ImpactClosure,
    /// Explicit bridges for removed predecessor subjects absent from this graph.
    pub(in crate::resident) removed_root_bridges: Vec<RemovedWorkspaceRootBridge>,
    /// Removed repositories have no successor node and therefore no invented seed.
    pub(in crate::resident) removed_repository_bridges: Vec<RemovedRepositoryBridge>,
    /// Removed Package resources have no successor node and no invented seed.
    pub(in crate::resident) removed_package_resource_bridges: Vec<RemovedPackageResourceBridge>,
}

impl GenerationSuccessorImpact {
    /// Borrow exact typed transitions that seeded the graph closure.
    #[must_use]
    pub fn input_changes(&self) -> &[GenerationInputChange] {
        &self.input_changes
    }

    /// Borrow canonically ordered affected semantic definitions.
    #[must_use]
    pub fn affected(&self) -> &[ResidentDefinitionCoordinate] {
        &self.affected
    }

    /// Borrow the exact unaffected semantic complement.
    #[must_use]
    pub fn unaffected(&self) -> &[ResidentDefinitionCoordinate] {
        &self.unaffected
    }

    /// Borrow explicit transition bridges for removed workspace roots.
    #[must_use]
    pub fn removed_workspace_roots(&self) -> &[RemovedWorkspaceRootBridge] {
        &self.removed_root_bridges
    }

    /// Borrow explicit transition bridges for removed repositories.
    #[must_use]
    pub fn removed_repositories(&self) -> &[RemovedRepositoryBridge] {
        &self.removed_repository_bridges
    }

    /// Borrow explicit transition bridges for removed Package resources.
    #[must_use]
    pub fn removed_package_resources(&self) -> &[RemovedPackageResourceBridge] {
        &self.removed_package_resource_bridges
    }
}

/// One deterministic semantic witness through the resident dependency graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidentImpactReasonPath {
    /// Affected definition explained by this path.
    pub(in crate::resident) target: ResidentDefinitionCoordinate,
    /// Semantic definitions from an exact seed to [`Self::target`].
    ///
    /// This order follows reverse-impact traversal: each later node depends on
    /// the preceding node.
    pub(in crate::resident) nodes: Vec<ResidentDefinitionCoordinate>,
    /// Exact stored relationships between adjacent [`Self::nodes`].
    ///
    /// Each relationship is stored owner-to-dependency, so step `i` points
    /// from `nodes[i + 1]` back to `nodes[i]` while the witness itself advances
    /// from changed dependency to affected owner.
    pub(in crate::resident) steps: Vec<ResidentImpactReasonStep>,
}

impl ResidentImpactReasonPath {
    /// Borrow the affected definition explained by this path.
    #[must_use]
    pub fn target(&self) -> &ResidentDefinitionCoordinate {
        &self.target
    }

    /// Borrow semantic definitions from exact seed through the target.
    #[must_use]
    pub fn nodes(&self) -> &[ResidentDefinitionCoordinate] {
        &self.nodes
    }

    /// Borrow owner-to-dependency reasons for adjacent reverse-impact nodes.
    #[must_use]
    pub fn steps(&self) -> &[ResidentImpactReasonStep] {
        &self.steps
    }
}

/// One exact relationship step in a resident impact witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidentImpactReasonStep {
    /// Closed owner-to-dependency relationship kind.
    pub(in crate::resident) relation: ResidentRelation,
    /// Evidence discriminator not derivable from [`Self::relation`].
    pub(in crate::resident) evidence: ResidentRelationshipEvidenceCoordinate,
}

impl ResidentImpactReasonStep {
    /// Return the closed relationship kind.
    #[must_use]
    pub const fn relation(&self) -> ResidentRelation {
        self.relation
    }

    /// Borrow exact evidence not derivable from [`Self::relation`].
    #[must_use]
    pub fn evidence(&self) -> &ResidentRelationshipEvidenceCoordinate {
        &self.evidence
    }
}

/// One exact admitted input transition that seeds successor invalidation.
///
/// Variants keep unrelated identity domains disjoint. The owner traverses each
/// family and its canonical maps deterministically; no Boolean bag or string
/// tag participates in the impact algebra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerationInputChange {
    /// A workspace causal root changed under the owner-derived receipt.
    WorkspaceRoot {
        /// Exact causal-root digest.
        root_digest: WorkspaceDerivationRootDigest,
        /// Added, modified, or removed transition with exact value digests.
        change: Box<WorkspaceDerivationRootChangeV1>,
    },
    /// The admitted workspace semantic IR changed.
    WorkspaceSemanticIr {
        /// Predecessor semantic IR.
        prior: WorkspaceSemanticIrDigest,
        /// Successor semantic IR.
        current: WorkspaceSemanticIrDigest,
    },
    /// The admitted workspace proof frontier changed without implying semantic drift.
    WorkspaceAdmissionEvidence {
        /// Predecessor evidence frontier.
        prior: WorkspaceAdmissionEvidenceDigest,
        /// Successor evidence frontier.
        current: WorkspaceAdmissionEvidenceDigest,
    },
    /// The principal-relative admitted universe changed.
    PrincipalUniverse {
        /// Predecessor universe identity.
        prior: PrincipalAdmittedUniverseDigest,
        /// Successor universe identity.
        current: PrincipalAdmittedUniverseDigest,
    },
    /// The admitted Package semantic closure changed.
    PackageClosure {
        /// Predecessor Package closure.
        prior: PackageSemanticIrDigest,
        /// Successor Package closure.
        current: PackageSemanticIrDigest,
    },
    /// One Package resource entered the selected semantic closure.
    PackageResourceAdded {
        /// Exact successor definition coordinate.
        current: Box<PackageResourceResidentCoordinate>,
    },
    /// One retained Package resource definition changed.
    PackageResourceModified {
        /// Exact predecessor definition coordinate.
        prior: Box<PackageResourceResidentCoordinate>,
        /// Exact successor definition coordinate.
        current: Box<PackageResourceResidentCoordinate>,
    },
    /// One Package resource left the selected semantic closure.
    PackageResourceRemoved {
        /// Exact predecessor definition coordinate.
        prior: Box<PackageResourceResidentCoordinate>,
    },
    /// The effective private profile changed.
    Profile {
        /// Predecessor profile.
        prior: ProfileDigest,
        /// Successor profile.
        current: ProfileDigest,
    },
    /// The resolved project instance or machine overlay changed.
    ProjectInstance {
        /// Predecessor instance.
        prior: InstanceDigest,
        /// Successor instance.
        current: InstanceDigest,
    },
    /// The derived experience-composition plan changed.
    CompositionPlan {
        /// Predecessor plan.
        prior: ExperienceCompositionPlanDigest,
        /// Successor plan.
        current: ExperienceCompositionPlanDigest,
    },
    /// One repository entered the compiled set.
    RepositoryAdded {
        /// Exact successor repository evidence coordinate.
        current: Box<RepositoryResidentCoordinate>,
    },
    /// One retained repository changed compiled or applied evidence.
    RepositoryModified {
        /// Exact predecessor coordinate.
        prior: Box<RepositoryResidentCoordinate>,
        /// Exact successor coordinate.
        current: Box<RepositoryResidentCoordinate>,
    },
    /// One repository left the compiled set.
    RepositoryRemoved {
        /// Exact predecessor coordinate.
        prior: Box<RepositoryResidentCoordinate>,
    },
}

/// Exact compiled and owner-applied coordinate of one resident repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryResidentCoordinate {
    /// Stable repository identity.
    pub repository_id: LogicalRepositoryIdV1,
    /// Exact compiled repository manifest.
    pub manifest_digest: ManifestDigest,
    /// Owner-admitted successful materialization receipt.
    pub receipt_digest: ReceiptDigest,
    /// Applied state identity.
    pub state_digest: MaterializationStateDigest,
    /// Applied lineage identity.
    pub lineage_digest: MaterializationLineageDigest,
    /// Exact independently observed source frontier.
    pub source_frontier: RepositorySourceFrontierV1,
}

/// Exact canonical body coordinate of one resident Package resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageResourceResidentCoordinate {
    /// Owner-qualified resource and the exact use selected from it.
    pub resource: crate::packages::PackageResourceUse,
    /// Canonical identity of the admitted node and all supplemental bodies.
    pub definition_sha256: ArtifactDigest,
}

/// Current subjects used to bridge one removed predecessor causal root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedWorkspaceRootBridge {
    /// Exact removed-root identity from the admitted derivation receipt.
    pub(in crate::resident) root_digest: WorkspaceDerivationRootDigest,
    /// Current subjects whose receipt impact cites that removed root.
    pub(in crate::resident) current_subjects: Vec<WorkspaceDerivationSubjectV1>,
}

impl RemovedWorkspaceRootBridge {
    /// Borrow the exact removed-root identity.
    #[must_use]
    pub fn root_digest(&self) -> &WorkspaceDerivationRootDigest {
        &self.root_digest
    }

    /// Borrow current subjects reached through this predecessor-only root.
    #[must_use]
    pub fn current_subjects(&self) -> &[WorkspaceDerivationSubjectV1] {
        &self.current_subjects
    }
}

/// Exact predecessor evidence for a repository absent from the successor graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedRepositoryBridge {
    /// Exact predecessor repository coordinate.
    pub(in crate::resident) prior: RepositoryResidentCoordinate,
}

impl RemovedRepositoryBridge {
    /// Borrow the exact predecessor repository coordinate.
    #[must_use]
    pub fn prior(&self) -> &RepositoryResidentCoordinate {
        &self.prior
    }
}

/// Exact predecessor coordinate for a Package resource absent from the successor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedPackageResourceBridge {
    /// Exact predecessor Package-resource coordinate.
    pub(in crate::resident) prior: PackageResourceResidentCoordinate,
}

impl RemovedPackageResourceBridge {
    /// Borrow the exact predecessor Package-resource coordinate.
    #[must_use]
    pub fn prior(&self) -> &PackageResourceResidentCoordinate {
        &self.prior
    }
}
