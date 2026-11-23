//! Closed resident-definition vocabulary and single-copy bodies.

use pale_core::records::{
    CapabilityPolicy, CompiledContentIdentityV1, CompositionEdgeV1, CompositionGraphViewV1,
    CompositionReasonPathV1, ContinuityContractV1, DeclarationDependencyKindV1,
    EffectiveProjectProfileV1, EntryContractV1, ExecutionPathSelectionIdV1,
    ExperienceCompositionPlanDigest, ExperiencePathV1, InstanceDigest, LifecycleBehaviorIdV1,
    LogicalRepositoryIdV1, PackageResourceRefV1, PackageSemanticIrDigest,
    ParticipantProviderManifestV1, ParticipantRefV1, PrincipalAdmittedUniverseDigest,
    ProfileDigest, ProviderBindingV1, RequestTranslationV1, ResolvedProjectInstanceV1,
    ResponsibilityContractV1, ResultTranslationV1, SchemaDigest, SchemaId, SettlementContractV1,
    SourceContentIdentityV1, TerminationContractV1, ToolRecipe, WorkspaceAdmissionEvidenceDigest,
    WorkspaceDeclarationRefV1, WorkspaceDerivationRuleV1, WorkspaceDerivationSubjectV1,
    WorkspaceModuleId, WorkspaceSemanticIrDigest,
};
use pale_kernel::NodeHandle;
use std::collections::{BTreeMap, BTreeSet};

use crate::compile::CompiledRepo;
use crate::packages::{AdmittedMarkdown, AdmittedResourceNode, PackageResource};
use crate::workspace::{
    ResidentWorkspaceDefinition, WorkspaceAdmissionEvidence, WorkspaceSemanticDeclarationEntry,
};

use super::super::generation::AppliedRepositoryMaterialization;
use super::super::generation::{ResidentCompositionDefinitionKind, ResidentDefinitionCoordinate};

/// Typed compiled-content identity with ordering over its owned digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct ResidentCompiledContentKey(
    pub(in crate::resident) CompiledContentIdentityV1,
);

impl PartialOrd for ResidentCompiledContentKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResidentCompiledContentKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .compiled_content_sha256
            .cmp(&other.0.compiled_content_sha256)
    }
}

/// Typed source-content identity with ordering over its owned digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct ResidentSourceContentKey(
    pub(in crate::resident) SourceContentIdentityV1,
);

impl PartialOrd for ResidentSourceContentKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResidentSourceContentKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .source_content_sha256
            .cmp(&other.0.source_content_sha256)
    }
}

/// Stable semantic identity of one resident definition.
///
/// Variants keep unrelated identity domains disjoint even when their wire text
/// happens to be equal. Provider-family/product labels and other loose strings
/// never enter this key space.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::resident) enum ResidentNodeKey {
    Repository(LogicalRepositoryIdV1),
    Profile(ProfileDigest),
    ProjectInstance(InstanceDigest),
    ExecutionPathSelection(ExecutionPathSelectionIdV1),
    PackageResource(crate::packages::PackageResourceUse),
    PackageClosure(PackageSemanticIrDigest),
    WorkspaceSemanticIr(WorkspaceSemanticIrDigest),
    WorkspaceAdmissionEvidence(WorkspaceAdmissionEvidenceDigest),
    PrincipalUniverse(PrincipalAdmittedUniverseDigest),
    CompositionPlan(ExperienceCompositionPlanDigest),
    WorkspaceDeclaration {
        owner_module: WorkspaceModuleId,
        declaration: WorkspaceDeclarationRefV1,
    },
    WorkspaceCausal(WorkspaceDerivationSubjectV1),
    Responsibility(String),
    Participant(ParticipantRefV1),
    ProviderBinding(String),
    ExperiencePath(String),
    EntryContract(String),
    RequestTranslation(String),
    ResultTranslation(String),
    TerminationContract(String),
    SettlementContract(String),
    ContinuityContract(String),
    CompiledContent(ResidentCompiledContentKey),
    SourceContent(ResidentSourceContentKey),
    Operation {
        repository: LogicalRepositoryIdV1,
        operation: PackageResourceRefV1,
    },
    RunMode {
        repository: LogicalRepositoryIdV1,
        mode: PackageResourceRefV1,
    },
    Lifecycle(LifecycleBehaviorIdV1),
}

impl ResidentNodeKey {
    /// Map one admitted workspace causal subject into its sole resident node.
    pub(super) fn from_workspace_subject(subject: &WorkspaceDerivationSubjectV1) -> Self {
        match subject {
            WorkspaceDerivationSubjectV1::Declaration {
                owner_module,
                declaration,
            } => Self::WorkspaceDeclaration {
                owner_module: owner_module.clone(),
                declaration: declaration.clone(),
            },
            _ => Self::WorkspaceCausal(subject.clone()),
        }
    }

    /// Compare this owned key with one borrowed hot-path query.
    pub(super) fn cmp_query(&self, query: ResidentNodeQuery<'_>) -> std::cmp::Ordering {
        self.kind()
            .cmp(&query.kind())
            .then_with(|| match (self, query) {
                (Self::Repository(candidate), ResidentNodeQuery::Repository(query)) => {
                    candidate.cmp(query)
                }
                (Self::CompositionPlan(candidate), ResidentNodeQuery::CompositionPlan(query)) => {
                    candidate.cmp(query)
                }
                (Self::ProviderBinding(candidate), ResidentNodeQuery::ProviderBinding(query))
                | (Self::ExperiencePath(candidate), ResidentNodeQuery::ExperiencePath(query)) => {
                    candidate.as_str().cmp(query)
                }
                (
                    Self::Operation {
                        repository,
                        operation,
                    },
                    ResidentNodeQuery::Operation {
                        repository: query_repository,
                        operation: query_operation,
                    },
                ) => repository
                    .cmp(query_repository)
                    .then_with(|| operation.cmp(query_operation)),
                (
                    Self::RunMode { repository, mode },
                    ResidentNodeQuery::RunMode {
                        repository: query_repository,
                        mode: query_mode,
                    },
                ) => repository
                    .cmp(query_repository)
                    .then_with(|| mode.cmp(query_mode)),
                (Self::Lifecycle(candidate), ResidentNodeQuery::Lifecycle(query)) => {
                    candidate.cmp(query)
                }
                _ => std::cmp::Ordering::Equal,
            })
    }

    /// Compare one owned key with a borrowed public semantic coordinate.
    pub(super) fn cmp_coordinate(
        &self,
        coordinate: &ResidentDefinitionCoordinate,
    ) -> std::cmp::Ordering {
        self.kind()
            .cmp(&coordinate_kind(coordinate))
            .then_with(|| match (self, coordinate) {
                (Self::Repository(candidate), ResidentDefinitionCoordinate::Repository(query)) => {
                    candidate.cmp(query)
                }
                (Self::Profile(candidate), ResidentDefinitionCoordinate::Profile(query)) => {
                    candidate.cmp(query)
                }
                (
                    Self::ProjectInstance(candidate),
                    ResidentDefinitionCoordinate::ProjectInstance(query),
                ) => candidate.cmp(query),
                (
                    Self::ExecutionPathSelection(candidate),
                    ResidentDefinitionCoordinate::ExecutionPathSelection(query),
                ) => candidate.cmp(query),
                (
                    Self::PackageResource(candidate),
                    ResidentDefinitionCoordinate::PackageResource(query),
                ) => candidate.cmp(query),
                (
                    Self::PackageClosure(candidate),
                    ResidentDefinitionCoordinate::PackageClosure(query),
                ) => candidate.cmp(query),
                (
                    Self::WorkspaceSemanticIr(candidate),
                    ResidentDefinitionCoordinate::WorkspaceSemanticIr(query),
                ) => candidate.cmp(query),
                (
                    Self::WorkspaceAdmissionEvidence(candidate),
                    ResidentDefinitionCoordinate::WorkspaceAdmissionEvidence(query),
                ) => candidate.cmp(query),
                (
                    Self::PrincipalUniverse(candidate),
                    ResidentDefinitionCoordinate::PrincipalUniverse(query),
                ) => candidate.cmp(query),
                (
                    Self::CompositionPlan(candidate),
                    ResidentDefinitionCoordinate::CompositionPlan(query),
                ) => candidate.cmp(query),
                (
                    Self::WorkspaceDeclaration {
                        owner_module,
                        declaration,
                    },
                    ResidentDefinitionCoordinate::WorkspaceDeclaration {
                        owner_module: query_owner,
                        declaration: query_declaration,
                    },
                ) => (owner_module, declaration).cmp(&(query_owner, query_declaration)),
                (
                    Self::WorkspaceCausal(candidate),
                    ResidentDefinitionCoordinate::WorkspaceCausal(query),
                ) => candidate.cmp(query.subject()),
                (
                    Self::Responsibility(candidate),
                    ResidentDefinitionCoordinate::Composition {
                        kind: ResidentCompositionDefinitionKind::Responsibility,
                        id,
                    },
                )
                | (
                    Self::ProviderBinding(candidate),
                    ResidentDefinitionCoordinate::Composition {
                        kind: ResidentCompositionDefinitionKind::ProviderBinding,
                        id,
                    },
                )
                | (
                    Self::ExperiencePath(candidate),
                    ResidentDefinitionCoordinate::Composition {
                        kind: ResidentCompositionDefinitionKind::ExperiencePath,
                        id,
                    },
                )
                | (
                    Self::EntryContract(candidate),
                    ResidentDefinitionCoordinate::Composition {
                        kind: ResidentCompositionDefinitionKind::EntryContract,
                        id,
                    },
                )
                | (
                    Self::RequestTranslation(candidate),
                    ResidentDefinitionCoordinate::Composition {
                        kind: ResidentCompositionDefinitionKind::RequestTranslation,
                        id,
                    },
                )
                | (
                    Self::ResultTranslation(candidate),
                    ResidentDefinitionCoordinate::Composition {
                        kind: ResidentCompositionDefinitionKind::ResultTranslation,
                        id,
                    },
                )
                | (
                    Self::TerminationContract(candidate),
                    ResidentDefinitionCoordinate::Composition {
                        kind: ResidentCompositionDefinitionKind::TerminationContract,
                        id,
                    },
                )
                | (
                    Self::SettlementContract(candidate),
                    ResidentDefinitionCoordinate::Composition {
                        kind: ResidentCompositionDefinitionKind::SettlementContract,
                        id,
                    },
                )
                | (
                    Self::ContinuityContract(candidate),
                    ResidentDefinitionCoordinate::Composition {
                        kind: ResidentCompositionDefinitionKind::ContinuityContract,
                        id,
                    },
                ) => candidate.cmp(id),
                (
                    Self::Participant(candidate),
                    ResidentDefinitionCoordinate::Participant(query),
                ) => candidate.cmp(query),
                (
                    Self::CompiledContent(candidate),
                    ResidentDefinitionCoordinate::CompiledContent(query),
                ) => candidate
                    .0
                    .compiled_content_sha256
                    .cmp(&query.compiled_content_sha256),
                (
                    Self::SourceContent(candidate),
                    ResidentDefinitionCoordinate::SourceContent(query),
                ) => candidate
                    .0
                    .source_content_sha256
                    .cmp(&query.source_content_sha256),
                (
                    Self::Operation {
                        repository,
                        operation,
                    },
                    ResidentDefinitionCoordinate::Operation {
                        repository: query_repository,
                        operation: query_operation,
                    },
                ) => repository
                    .cmp(query_repository)
                    .then_with(|| operation.cmp(query_operation)),
                (
                    Self::RunMode { repository, mode },
                    ResidentDefinitionCoordinate::RunMode {
                        repository: query_repository,
                        mode: query_mode,
                    },
                ) => repository
                    .cmp(query_repository)
                    .then_with(|| mode.cmp(query_mode)),
                (Self::Lifecycle(candidate), ResidentDefinitionCoordinate::Lifecycle(query)) => {
                    candidate.cmp(query)
                }
                _ => std::cmp::Ordering::Equal,
            })
    }

    const fn kind(&self) -> ResidentNodeKind {
        match self {
            Self::Repository(_) => ResidentNodeKind::Repository,
            Self::Profile(_) => ResidentNodeKind::Profile,
            Self::ProjectInstance(_) => ResidentNodeKind::ProjectInstance,
            Self::ExecutionPathSelection(_) => ResidentNodeKind::ExecutionPathSelection,
            Self::PackageResource(_) => ResidentNodeKind::PackageResource,
            Self::PackageClosure(_) => ResidentNodeKind::PackageClosure,
            Self::WorkspaceSemanticIr(_) => ResidentNodeKind::WorkspaceSemanticIr,
            Self::WorkspaceAdmissionEvidence(_) => ResidentNodeKind::WorkspaceAdmissionEvidence,
            Self::PrincipalUniverse(_) => ResidentNodeKind::PrincipalUniverse,
            Self::CompositionPlan(_) => ResidentNodeKind::CompositionPlan,
            Self::WorkspaceDeclaration { .. } => ResidentNodeKind::WorkspaceDeclaration,
            Self::WorkspaceCausal(_) => ResidentNodeKind::WorkspaceCausal,
            Self::Responsibility(_) => ResidentNodeKind::Responsibility,
            Self::Participant(_) => ResidentNodeKind::Participant,
            Self::ProviderBinding(_) => ResidentNodeKind::ProviderBinding,
            Self::ExperiencePath(_) => ResidentNodeKind::ExperiencePath,
            Self::EntryContract(_) => ResidentNodeKind::EntryContract,
            Self::RequestTranslation(_) => ResidentNodeKind::RequestTranslation,
            Self::ResultTranslation(_) => ResidentNodeKind::ResultTranslation,
            Self::TerminationContract(_) => ResidentNodeKind::TerminationContract,
            Self::SettlementContract(_) => ResidentNodeKind::SettlementContract,
            Self::ContinuityContract(_) => ResidentNodeKind::ContinuityContract,
            Self::CompiledContent(_) => ResidentNodeKind::CompiledContent,
            Self::SourceContent(_) => ResidentNodeKind::SourceContent,
            Self::Operation { .. } => ResidentNodeKind::Operation,
            Self::RunMode { .. } => ResidentNodeKind::RunMode,
            Self::Lifecycle(_) => ResidentNodeKind::Lifecycle,
        }
    }
}

const fn coordinate_kind(coordinate: &ResidentDefinitionCoordinate) -> ResidentNodeKind {
    match coordinate {
        ResidentDefinitionCoordinate::Repository(_) => ResidentNodeKind::Repository,
        ResidentDefinitionCoordinate::Profile(_) => ResidentNodeKind::Profile,
        ResidentDefinitionCoordinate::ProjectInstance(_) => ResidentNodeKind::ProjectInstance,
        ResidentDefinitionCoordinate::ExecutionPathSelection(_) => {
            ResidentNodeKind::ExecutionPathSelection
        }
        ResidentDefinitionCoordinate::PackageResource(_) => ResidentNodeKind::PackageResource,
        ResidentDefinitionCoordinate::PackageClosure(_) => ResidentNodeKind::PackageClosure,
        ResidentDefinitionCoordinate::WorkspaceSemanticIr(_) => {
            ResidentNodeKind::WorkspaceSemanticIr
        }
        ResidentDefinitionCoordinate::WorkspaceAdmissionEvidence(_) => {
            ResidentNodeKind::WorkspaceAdmissionEvidence
        }
        ResidentDefinitionCoordinate::PrincipalUniverse(_) => ResidentNodeKind::PrincipalUniverse,
        ResidentDefinitionCoordinate::CompositionPlan(_) => ResidentNodeKind::CompositionPlan,
        ResidentDefinitionCoordinate::WorkspaceDeclaration { .. } => {
            ResidentNodeKind::WorkspaceDeclaration
        }
        ResidentDefinitionCoordinate::WorkspaceCausal(_) => ResidentNodeKind::WorkspaceCausal,
        ResidentDefinitionCoordinate::Composition { kind, .. } => match kind {
            ResidentCompositionDefinitionKind::Responsibility => ResidentNodeKind::Responsibility,
            ResidentCompositionDefinitionKind::ProviderBinding => ResidentNodeKind::ProviderBinding,
            ResidentCompositionDefinitionKind::ExperiencePath => ResidentNodeKind::ExperiencePath,
            ResidentCompositionDefinitionKind::EntryContract => ResidentNodeKind::EntryContract,
            ResidentCompositionDefinitionKind::RequestTranslation => {
                ResidentNodeKind::RequestTranslation
            }
            ResidentCompositionDefinitionKind::ResultTranslation => {
                ResidentNodeKind::ResultTranslation
            }
            ResidentCompositionDefinitionKind::TerminationContract => {
                ResidentNodeKind::TerminationContract
            }
            ResidentCompositionDefinitionKind::SettlementContract => {
                ResidentNodeKind::SettlementContract
            }
            ResidentCompositionDefinitionKind::ContinuityContract => {
                ResidentNodeKind::ContinuityContract
            }
        },
        ResidentDefinitionCoordinate::Participant(_) => ResidentNodeKind::Participant,
        ResidentDefinitionCoordinate::CompiledContent(_) => ResidentNodeKind::CompiledContent,
        ResidentDefinitionCoordinate::SourceContent(_) => ResidentNodeKind::SourceContent,
        ResidentDefinitionCoordinate::Operation { .. } => ResidentNodeKind::Operation,
        ResidentDefinitionCoordinate::RunMode { .. } => ResidentNodeKind::RunMode,
        ResidentDefinitionCoordinate::Lifecycle(_) => ResidentNodeKind::Lifecycle,
    }
}

#[derive(Clone, Copy)]
pub(super) enum ResidentNodeQuery<'a> {
    Repository(&'a LogicalRepositoryIdV1),
    CompositionPlan(&'a ExperienceCompositionPlanDigest),
    ProviderBinding(&'a str),
    ExperiencePath(&'a str),
    Operation {
        repository: &'a LogicalRepositoryIdV1,
        operation: &'a PackageResourceRefV1,
    },
    RunMode {
        repository: &'a LogicalRepositoryIdV1,
        mode: &'a PackageResourceRefV1,
    },
    Lifecycle(&'a LifecycleBehaviorIdV1),
}

impl ResidentNodeQuery<'_> {
    const fn kind(self) -> ResidentNodeKind {
        match self {
            Self::Repository(_) => ResidentNodeKind::Repository,
            Self::CompositionPlan(_) => ResidentNodeKind::CompositionPlan,
            Self::ProviderBinding(_) => ResidentNodeKind::ProviderBinding,
            Self::ExperiencePath(_) => ResidentNodeKind::ExperiencePath,
            Self::Operation { .. } => ResidentNodeKind::Operation,
            Self::RunMode { .. } => ResidentNodeKind::RunMode,
            Self::Lifecycle(_) => ResidentNodeKind::Lifecycle,
        }
    }
}

/// Closed body class retained in the packed node record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ResidentNodeKind {
    Repository,
    Profile,
    ProjectInstance,
    ExecutionPathSelection,
    PackageResource,
    PackageClosure,
    WorkspaceSemanticIr,
    WorkspaceAdmissionEvidence,
    PrincipalUniverse,
    CompositionPlan,
    WorkspaceDeclaration,
    WorkspaceCausal,
    Responsibility,
    Participant,
    ProviderBinding,
    ExperiencePath,
    EntryContract,
    RequestTranslation,
    ResultTranslation,
    TerminationContract,
    SettlementContract,
    ContinuityContract,
    CompiledContent,
    SourceContent,
    Operation,
    RunMode,
    Lifecycle,
}

/// Complete selected Package resource retained once.
pub(super) struct ResidentPackageDefinition {
    pub(super) node: AdmittedResourceNode,
    pub(super) markdown: Option<std::sync::Arc<AdmittedMarkdown>>,
    pub(super) capability_policy: Option<CapabilityPolicy>,
    pub(super) recipe: Option<ToolRecipe>,
}

/// Non-reconstructible occurrence seeds retained by the Package closure.
///
/// Selected releases and whole-profile governing order are already bound by
/// the public generation/profile records. These two seed sets are not
/// reconstructible from closure membership and therefore remain here exactly
/// once as the residual Package semantic-IR body.
pub(super) struct ResidentPackageClosureDefinition {
    pub(super) global_seeds: BTreeSet<crate::packages::PackageResourceUse>,
    pub(super) repository_seeds:
        BTreeMap<LogicalRepositoryIdV1, BTreeSet<crate::packages::PackageResourceUse>>,
}

impl ResidentPackageDefinition {
    /// Verify exact supplemental-body custody for this admitted resource kind.
    pub(super) fn custody_is_complete(&self) -> bool {
        let markdown_is_valid = match &self.node.resource {
            PackageResource::Doctrine { .. }
            | PackageResource::OperationArchetype { .. }
            | PackageResource::Operation { .. }
            | PackageResource::RoleArchetype { .. }
            | PackageResource::Role { .. }
            | PackageResource::PolicyFamily { .. } => self.markdown.is_some(),
            PackageResource::RunMode { .. } => true,
            PackageResource::Stack { .. } => self.markdown.is_some() == self.node.page.is_some(),
            PackageResource::GoverningOrder
            | PackageResource::CapabilityPolicy
            | PackageResource::ToolRecipe => self.markdown.is_none(),
        };
        markdown_is_valid
            && (self.capability_policy.is_some()
                == matches!(self.node.resource, PackageResource::CapabilityPolicy))
            && (self.recipe.is_some() == matches!(self.node.resource, PackageResource::ToolRecipe))
    }
}

/// Complete repository projection and its exact applied evidence.
pub(super) struct ResidentRepositoryDefinition {
    pub(super) compiled: CompiledRepo,
    pub(super) materialization: AppliedRepositoryMaterialization,
}

/// Composition-plan fields that are not represented by child bodies or edges.
///
/// Declaration bodies move into their own aligned nodes.
/// Operational edge contracts move into [`ResidentEdgeEvidence`]. This residual
/// therefore retains the remaining admitted plan proof exactly once, including
/// non-operational views that must not become resident adjacency.
pub(super) struct ResidentCompositionPlanDefinition {
    pub(super) schema: SchemaId,
    pub(super) schema_sha256: SchemaDigest,
    pub(super) schema_version: u32,
    pub(super) participant_provider_manifests: Vec<ParticipantProviderManifestV1>,
    pub(super) non_operational_edges: Vec<CompositionEdgeV1>,
    pub(super) graph_views: Vec<CompositionGraphViewV1>,
    pub(super) reason_paths: Vec<CompositionReasonPathV1>,
    pub(super) plan_digest: ExperienceCompositionPlanDigest,
}

/// One immutable resident body.
///
/// Operation and run-mode nodes carry a generation-local repository handle and
/// a position into that repository's sole [`CompiledRepo`] body. They do not
/// clone nested semantic records merely to accelerate lookup.
pub(super) enum ResidentDefinition {
    Repository(Box<ResidentRepositoryDefinition>),
    Profile(Box<EffectiveProjectProfileV1>),
    ProjectInstance(Box<ResolvedProjectInstanceV1>),
    ExecutionPathSelection {
        profile: NodeHandle,
        position: usize,
    },
    PackageResource(Box<ResidentPackageDefinition>),
    PackageClosure(Box<ResidentPackageClosureDefinition>),
    WorkspaceSemanticIr(Box<ResidentWorkspaceDefinition>),
    WorkspaceAdmissionEvidence(Box<WorkspaceAdmissionEvidence>),
    PrincipalUniverse,
    CompositionPlan(Box<ResidentCompositionPlanDefinition>),
    WorkspaceDeclaration(Box<WorkspaceSemanticDeclarationEntry>),
    WorkspaceCausal,
    Responsibility(Box<ResponsibilityContractV1>),
    Participant,
    ProviderBinding(Box<ProviderBindingV1>),
    ExperiencePath(Box<ExperiencePathV1>),
    EntryContract(Box<EntryContractV1>),
    RequestTranslation(Box<RequestTranslationV1>),
    ResultTranslation(Box<ResultTranslationV1>),
    TerminationContract(Box<TerminationContractV1>),
    SettlementContract(Box<SettlementContractV1>),
    ContinuityContract(Box<ContinuityContractV1>),
    CompiledContent,
    SourceContent,
    Operation {
        repository: NodeHandle,
        position: usize,
    },
    RunMode {
        repository: NodeHandle,
        position: usize,
    },
    Lifecycle {
        profile: NodeHandle,
        position: usize,
    },
}

impl ResidentDefinition {
    pub(super) const fn kind(&self) -> ResidentNodeKind {
        match self {
            Self::Repository(_) => ResidentNodeKind::Repository,
            Self::Profile(_) => ResidentNodeKind::Profile,
            Self::ProjectInstance(_) => ResidentNodeKind::ProjectInstance,
            Self::ExecutionPathSelection { .. } => ResidentNodeKind::ExecutionPathSelection,
            Self::PackageResource(_) => ResidentNodeKind::PackageResource,
            Self::PackageClosure(_) => ResidentNodeKind::PackageClosure,
            Self::WorkspaceSemanticIr(_) => ResidentNodeKind::WorkspaceSemanticIr,
            Self::WorkspaceAdmissionEvidence(_) => ResidentNodeKind::WorkspaceAdmissionEvidence,
            Self::PrincipalUniverse => ResidentNodeKind::PrincipalUniverse,
            Self::CompositionPlan(_) => ResidentNodeKind::CompositionPlan,
            Self::WorkspaceDeclaration(_) => ResidentNodeKind::WorkspaceDeclaration,
            Self::WorkspaceCausal => ResidentNodeKind::WorkspaceCausal,
            Self::Responsibility(_) => ResidentNodeKind::Responsibility,
            Self::Participant => ResidentNodeKind::Participant,
            Self::ProviderBinding(_) => ResidentNodeKind::ProviderBinding,
            Self::ExperiencePath(_) => ResidentNodeKind::ExperiencePath,
            Self::EntryContract(_) => ResidentNodeKind::EntryContract,
            Self::RequestTranslation(_) => ResidentNodeKind::RequestTranslation,
            Self::ResultTranslation(_) => ResidentNodeKind::ResultTranslation,
            Self::TerminationContract(_) => ResidentNodeKind::TerminationContract,
            Self::SettlementContract(_) => ResidentNodeKind::SettlementContract,
            Self::ContinuityContract(_) => ResidentNodeKind::ContinuityContract,
            Self::CompiledContent => ResidentNodeKind::CompiledContent,
            Self::SourceContent => ResidentNodeKind::SourceContent,
            Self::Operation { .. } => ResidentNodeKind::Operation,
            Self::RunMode { .. } => ResidentNodeKind::RunMode,
            Self::Lifecycle { .. } => ResidentNodeKind::Lifecycle,
        }
    }
}

/// Exact resident operational/alignment relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResidentRelation {
    /// Execution-path selection belongs to the effective profile.
    ExecutionPathSelectionProfile,
    /// Execution-path selection depends on its neutral experience path.
    ExecutionPathSelectionExperiencePath,
    /// Repository depends on a Package resource in its selected scope.
    RepositoryPackageScope,
    /// Operation child depends on its repository body.
    OperationRepository,
    /// Run-mode child depends on its repository body.
    RunModeRepository,
    /// Repository depends on an admitted lifecycle behavior.
    RepositoryLifecycle,
    /// Operation depends on its Package resource.
    OperationPackageResource,
    /// Run mode depends on its Package resource.
    RunModePackageResource,
    /// Package resource depends on another Package resource.
    PackageDependency,
    /// Package closure contains a selected Package resource.
    PackageClosureResource,
    /// Repository depends on the Package closure.
    RepositoryPackageClosure,
    /// Project instance depends on its effective profile.
    ProjectInstanceProfile,
    /// Effective profile depends on the Package closure.
    ProfilePackageClosure,
    /// Repository depends on the effective profile.
    RepositoryProfile,
    /// Repository depends on the resolved project instance.
    RepositoryProjectInstance,
    /// Principal universe depends on its admitted semantic IR.
    PrincipalUniverseSemanticIr,
    /// Principal universe depends on its exact admission-evidence frontier.
    PrincipalUniverseAdmissionEvidence,
    /// Workspace semantic IR depends on one admitted causal source.
    WorkspaceSemanticIrSource,
    /// Lifecycle behavior belongs to the effective profile.
    LifecycleProfile,
    /// Composition definition depends on the admitted plan.
    CompositionDefinitionPlan,
    /// Workspace declaration has one explicit declared dependency.
    WorkspaceExplicitDependency,
    /// Workspace relationship declaration depends on its source declaration.
    WorkspaceRelationshipFrom,
    /// Workspace relationship declaration depends on its target declaration.
    WorkspaceRelationshipTo,
    /// Workspace causal fact depends on an exact proof input under this rule.
    WorkspaceCausalInput(WorkspaceDerivationRuleV1),
    /// Composition definition depends on admitted composition source meaning.
    CompositionSourceDependency,
    /// Responsibility contract depends on its participant.
    ResponsibilityParticipant,
    /// Provider binding depends on its participant.
    ProviderParticipant,
    /// Provider binding depends on a responsibility contract.
    ProviderResponsibility,
    /// Experience path depends on a responsibility contract.
    PathResponsibility,
    /// Experience path depends on a provider binding.
    PathProvider,
    /// Experience path depends on its entry contract.
    PathEntryContract,
    /// Experience path depends on its termination contract.
    PathTerminationContract,
    /// Experience path depends on its settlement contract.
    PathSettlementContract,
    /// Experience path depends on its continuity contract.
    PathContinuityContract,
    /// Experience path depends on compiled-content identity.
    PathCompiledContent,
    /// Experience path depends on source-content identity.
    PathSourceContent,
    /// Entry contract depends on its entry participant.
    EntryParticipant,
    /// Entry contract depends on its responsibility contract.
    EntryResponsibility,
    /// Request translation depends on its source participant.
    RequestTranslationSource,
    /// Request translation depends on its target participant.
    RequestTranslationTarget,
    /// Result translation depends on its source participant.
    ResultTranslationSource,
    /// Result translation depends on its target participant.
    ResultTranslationTarget,
    /// Termination contract depends on its owner.
    TerminationOwner,
    /// Termination contract depends on its optional terminal leaf.
    TerminationLeaf,
    /// Settlement contract depends on its observer.
    SettlementObserver,
    /// Continuity boundary contract depends on its owning participant.
    ///
    /// The leaf does not absorb replica, writer, checkpoint, replay, merge,
    /// readiness, or recovery-state topology from the native continuity owner.
    ContinuityOwner,
    /// Operational work delegation from owner to dependency.
    WorkDelegation,
    /// Operational native-application call from caller to application.
    NativeApplicationCall,
    /// Operational terminal-leaf relationship.
    TerminalLeaf,
    /// Observer depends on the observed subject for settlement invalidation.
    ///
    /// The public and resident edge orientations are rendered by the private
    /// definition-store owner. The resident graph deliberately
    /// reverses the public observation direction, so a subject change
    /// reverse-invalidates the observer.
    ObservationSettlement,
    /// Delegating participant depends on its responsibility contract.
    DelegationResponsibility,
    /// Operational participant depends on request translation.
    OperationalRequestTranslation,
    /// Operational participant depends on result translation.
    OperationalResultTranslation,
    /// Native-call participant depends on its provider binding.
    NativeCallProvider,
    /// Terminal participant depends on its termination contract.
    TerminalContract,
    /// Observer depends on its settlement contract.
    ObservationSettlementContract,
}

/// Evidence not derivable from [`ResidentRelation`].
#[derive(Debug, Clone)]
pub(in crate::resident) enum ResidentEdgeEvidence {
    None,
    WorkspaceDependency(DeclarationDependencyKindV1),
    OperationalContract(Box<CompositionEdgeV1>),
}

pub(super) fn operational_relationship(
    edge: &CompositionEdgeV1,
) -> Option<(ResidentNodeKey, ResidentNodeKey, ResidentRelation)> {
    match edge {
        CompositionEdgeV1::WorkDelegation { from, to, .. } => Some((
            ResidentNodeKey::Participant(from.clone()),
            ResidentNodeKey::Participant(to.clone()),
            ResidentRelation::WorkDelegation,
        )),
        CompositionEdgeV1::NativeApplicationCall {
            caller,
            application,
            ..
        } => Some((
            ResidentNodeKey::Participant(caller.clone()),
            ResidentNodeKey::Participant(application.clone()),
            ResidentRelation::NativeApplicationCall,
        )),
        CompositionEdgeV1::TerminalLeaf {
            owner,
            terminal_leaf,
            ..
        } => Some((
            ResidentNodeKey::Participant(owner.clone()),
            ResidentNodeKey::Participant(terminal_leaf.clone()),
            ResidentRelation::TerminalLeaf,
        )),
        CompositionEdgeV1::ObservationSettlement {
            subject, observer, ..
        } => Some((
            ResidentNodeKey::Participant(observer.clone()),
            ResidentNodeKey::Participant(subject.clone()),
            ResidentRelation::ObservationSettlement,
        )),
        CompositionEdgeV1::UiEmbedding { .. } | CompositionEdgeV1::PluginDependency { .. } => None,
    }
}

/// Return whether one pending edge is valid for its typed endpoints and evidence.
pub(super) fn relationship_is_valid(
    edge: &PendingResidentEdge,
    source_kind: ResidentNodeKind,
    target_kind: ResidentNodeKind,
) -> bool {
    use ResidentNodeKind as Kind;
    use ResidentRelation as Relation;

    match &edge.evidence {
        ResidentEdgeEvidence::OperationalContract(contract) => {
            source_kind == Kind::Participant
                && target_kind == Kind::Participant
                && operational_relationship(contract).is_some_and(|(source, target, relation)| {
                    source == edge.source && target == edge.target && relation == edge.relation
                })
        }
        ResidentEdgeEvidence::WorkspaceDependency(_) => {
            edge.relation == Relation::WorkspaceExplicitDependency
                && source_kind == Kind::WorkspaceDeclaration
                && target_kind == Kind::WorkspaceDeclaration
        }
        ResidentEdgeEvidence::None => {
            let composition = |kind| {
                matches!(
                    kind,
                    Kind::Responsibility
                        | Kind::Participant
                        | Kind::ProviderBinding
                        | Kind::ExperiencePath
                        | Kind::EntryContract
                        | Kind::RequestTranslation
                        | Kind::ResultTranslation
                        | Kind::TerminationContract
                        | Kind::SettlementContract
                        | Kind::ContinuityContract
                        | Kind::CompiledContent
                        | Kind::SourceContent
                )
            };
            let workspace_subject =
                |kind| matches!(kind, Kind::WorkspaceDeclaration | Kind::WorkspaceCausal);
            match edge.relation {
                Relation::ExecutionPathSelectionProfile => {
                    (source_kind, target_kind) == (Kind::ExecutionPathSelection, Kind::Profile)
                }
                Relation::ExecutionPathSelectionExperiencePath => {
                    (source_kind, target_kind)
                        == (Kind::ExecutionPathSelection, Kind::ExperiencePath)
                }
                Relation::RepositoryPackageScope => {
                    (source_kind, target_kind) == (Kind::Repository, Kind::PackageResource)
                }
                Relation::OperationRepository => {
                    (source_kind, target_kind) == (Kind::Operation, Kind::Repository)
                }
                Relation::RunModeRepository => {
                    (source_kind, target_kind) == (Kind::RunMode, Kind::Repository)
                }
                Relation::RepositoryLifecycle => {
                    (source_kind, target_kind) == (Kind::Repository, Kind::Lifecycle)
                }
                Relation::OperationPackageResource => {
                    (source_kind, target_kind) == (Kind::Operation, Kind::PackageResource)
                }
                Relation::RunModePackageResource => {
                    (source_kind, target_kind) == (Kind::RunMode, Kind::PackageResource)
                }
                Relation::PackageDependency => {
                    (source_kind, target_kind) == (Kind::PackageResource, Kind::PackageResource)
                }
                Relation::PackageClosureResource => {
                    (source_kind, target_kind) == (Kind::PackageClosure, Kind::PackageResource)
                }
                Relation::RepositoryPackageClosure => {
                    (source_kind, target_kind) == (Kind::Repository, Kind::PackageClosure)
                }
                Relation::ProjectInstanceProfile => {
                    (source_kind, target_kind) == (Kind::ProjectInstance, Kind::Profile)
                }
                Relation::ProfilePackageClosure => {
                    (source_kind, target_kind) == (Kind::Profile, Kind::PackageClosure)
                }
                Relation::RepositoryProfile => {
                    (source_kind, target_kind) == (Kind::Repository, Kind::Profile)
                }
                Relation::RepositoryProjectInstance => {
                    (source_kind, target_kind) == (Kind::Repository, Kind::ProjectInstance)
                }
                Relation::PrincipalUniverseSemanticIr => {
                    (source_kind, target_kind)
                        == (Kind::PrincipalUniverse, Kind::WorkspaceSemanticIr)
                }
                Relation::PrincipalUniverseAdmissionEvidence => {
                    (source_kind, target_kind)
                        == (Kind::PrincipalUniverse, Kind::WorkspaceAdmissionEvidence)
                }
                Relation::WorkspaceSemanticIrSource => {
                    source_kind == Kind::WorkspaceSemanticIr && workspace_subject(target_kind)
                }
                Relation::LifecycleProfile => {
                    (source_kind, target_kind) == (Kind::Lifecycle, Kind::Profile)
                }
                Relation::CompositionDefinitionPlan => {
                    composition(source_kind) && target_kind == Kind::CompositionPlan
                }
                Relation::WorkspaceRelationshipFrom | Relation::WorkspaceRelationshipTo => {
                    (source_kind, target_kind)
                        == (Kind::WorkspaceDeclaration, Kind::WorkspaceDeclaration)
                }
                Relation::WorkspaceCausalInput(_) => {
                    workspace_subject(source_kind) && workspace_subject(target_kind)
                }
                Relation::CompositionSourceDependency => {
                    composition(source_kind)
                        && target_kind == Kind::WorkspaceCausal
                        && edge.target
                            == ResidentNodeKey::WorkspaceCausal(
                                WorkspaceDerivationSubjectV1::ExperienceComposition,
                            )
                }
                Relation::ResponsibilityParticipant => {
                    (source_kind, target_kind) == (Kind::Responsibility, Kind::Participant)
                }
                Relation::ProviderParticipant => {
                    (source_kind, target_kind) == (Kind::ProviderBinding, Kind::Participant)
                }
                Relation::ProviderResponsibility => {
                    (source_kind, target_kind) == (Kind::ProviderBinding, Kind::Responsibility)
                }
                Relation::PathResponsibility => {
                    (source_kind, target_kind) == (Kind::ExperiencePath, Kind::Responsibility)
                }
                Relation::PathProvider => {
                    (source_kind, target_kind) == (Kind::ExperiencePath, Kind::ProviderBinding)
                }
                Relation::PathEntryContract => {
                    (source_kind, target_kind) == (Kind::ExperiencePath, Kind::EntryContract)
                }
                Relation::PathTerminationContract => {
                    (source_kind, target_kind) == (Kind::ExperiencePath, Kind::TerminationContract)
                }
                Relation::PathSettlementContract => {
                    (source_kind, target_kind) == (Kind::ExperiencePath, Kind::SettlementContract)
                }
                Relation::PathContinuityContract => {
                    (source_kind, target_kind) == (Kind::ExperiencePath, Kind::ContinuityContract)
                }
                Relation::PathCompiledContent => {
                    (source_kind, target_kind) == (Kind::ExperiencePath, Kind::CompiledContent)
                }
                Relation::PathSourceContent => {
                    (source_kind, target_kind) == (Kind::ExperiencePath, Kind::SourceContent)
                }
                Relation::EntryParticipant => {
                    (source_kind, target_kind) == (Kind::EntryContract, Kind::Participant)
                }
                Relation::EntryResponsibility => {
                    (source_kind, target_kind) == (Kind::EntryContract, Kind::Responsibility)
                }
                Relation::RequestTranslationSource | Relation::RequestTranslationTarget => {
                    (source_kind, target_kind) == (Kind::RequestTranslation, Kind::Participant)
                }
                Relation::ResultTranslationSource | Relation::ResultTranslationTarget => {
                    (source_kind, target_kind) == (Kind::ResultTranslation, Kind::Participant)
                }
                Relation::TerminationOwner | Relation::TerminationLeaf => {
                    (source_kind, target_kind) == (Kind::TerminationContract, Kind::Participant)
                }
                Relation::SettlementObserver => {
                    (source_kind, target_kind) == (Kind::SettlementContract, Kind::Participant)
                }
                Relation::ContinuityOwner => {
                    (source_kind, target_kind) == (Kind::ContinuityContract, Kind::Participant)
                }
                Relation::DelegationResponsibility => {
                    (source_kind, target_kind) == (Kind::Participant, Kind::Responsibility)
                }
                Relation::OperationalRequestTranslation => {
                    (source_kind, target_kind) == (Kind::Participant, Kind::RequestTranslation)
                }
                Relation::OperationalResultTranslation => {
                    (source_kind, target_kind) == (Kind::Participant, Kind::ResultTranslation)
                }
                Relation::NativeCallProvider => {
                    (source_kind, target_kind) == (Kind::Participant, Kind::ProviderBinding)
                }
                Relation::TerminalContract => {
                    (source_kind, target_kind) == (Kind::Participant, Kind::TerminationContract)
                }
                Relation::ObservationSettlementContract => {
                    (source_kind, target_kind) == (Kind::Participant, Kind::SettlementContract)
                }
                Relation::WorkspaceExplicitDependency
                | Relation::WorkDelegation
                | Relation::NativeApplicationCall
                | Relation::TerminalLeaf
                | Relation::ObservationSettlement => false,
            }
        }
    }
}

impl ResidentEdgeEvidence {
    /// Compare only the exact evidence coordinate retained by each variant.
    pub(super) fn cmp_canonical(&self, other: &Self) -> std::cmp::Ordering {
        use ResidentEdgeEvidence::{None, OperationalContract, WorkspaceDependency};
        let rank = |evidence: &Self| match evidence {
            None => 0_u8,
            WorkspaceDependency(_) => 1,
            OperationalContract(_) => 2,
        };
        rank(self)
            .cmp(&rank(other))
            .then_with(|| match (self, other) {
                (None, None) => std::cmp::Ordering::Equal,
                (WorkspaceDependency(left), WorkspaceDependency(right)) => left.cmp(right),
                (OperationalContract(left), OperationalContract(right)) => {
                    (left.kind(), left.id()).cmp(&(right.kind(), right.id()))
                }
                _ => unreachable!("equal evidence ranks fix the variant family"),
            })
    }
}

/// Canonical ordinal assigned after semantic edge ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::resident) struct ResidentEdgeId(pub(super) u32);

/// Edge awaiting deterministic identity assignment.
#[derive(Debug)]
pub(super) struct PendingResidentEdge {
    pub(super) source: ResidentNodeKey,
    pub(super) target: ResidentNodeKey,
    pub(super) relation: ResidentRelation,
    pub(super) evidence: ResidentEdgeEvidence,
}
