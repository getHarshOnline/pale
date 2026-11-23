//! Public resident diagnostics and semantic failure coordinates.

use pale_core::canon;
use pale_core::records::{
    CompiledContentIdentityV1, CompositionGraphKindV1, DeclarationDependencyKindV1,
    ExecutionPathSelectionIdV1, ExperienceCompositionPlanDigest, InstanceDigest,
    LifecycleBehaviorIdV1, LogicalRepositoryIdV1, PackageResourceRefV1, PackageSemanticIrDigest,
    ParticipantRefV1, PrincipalAdmittedUniverseDigest, ProfileDigest, SourceContentIdentityV1,
    WorkspaceAdmissionEvidenceDigest, WorkspaceDeclarationRefV1, WorkspaceDerivationSubjectV1,
    WorkspaceModuleId, WorkspaceSemanticIrDigest,
};

use crate::compile::CompilationInvariantError;
use crate::plan::PlanError;

use super::super::definition::{
    DefinitionStoreError, ResidentEdgeEvidence, ResidentNodeKey, ResidentRelation,
};
use super::AppliedMaterializationCoordinate;

/// A live compilation could not be admitted as one immutable resident generation.
#[derive(Debug, thiserror::Error)]
pub enum CompiledGenerationAdmissionError {
    /// The one-shot compiler body no longer reproduces its admitted identities.
    #[error(transparent)]
    InvalidCompilation(Box<CompilationInvariantError>),
    /// Retained hook policy cannot be prepared for this exact repository and path.
    #[error(
        "compiled hook policy for `{repository_id}` path `{execution_path}` is invalid: {source}"
    )]
    HookPolicy {
        /// Repository owning the policy.
        repository_id: LogicalRepositoryIdV1,
        /// Selected execution path whose policy failed admission.
        execution_path: ExecutionPathSelectionIdV1,
        /// Policy preparation failure, without native event or injection content.
        #[source]
        source: super::super::hook::ResidentHookPolicyAdmissionError,
    },
    /// Exact workspace transition derivation or impact classification failed.
    #[error(transparent)]
    Workspace(Box<crate::workspace::WorkspaceError>),
    /// Portable and removal compilations cannot become executable generations.
    #[error("compiled generation requires an admitted surface executable identity")]
    MissingExecutableIdentity,
    /// Applied proof cardinality differs from the compiled repository set.
    #[error(
        "compiled generation covers {compiled} repositories but received {applied} applied materializations"
    )]
    RepositoryCoverage {
        /// Number of compiled repositories.
        compiled: usize,
        /// Number of supplied applied proofs.
        applied: usize,
    },
    /// Applied evidence does not preserve compilation materialization order.
    #[error(
        "applied materialization {position} names `{applied}`, expected compiled repository `{compiled}`"
    )]
    RepositoryOrder {
        /// Zero-based materialization position.
        position: usize,
        /// Repository selected by compilation.
        compiled: LogicalRepositoryIdV1,
        /// Repository named by the applied receipt.
        applied: LogicalRepositoryIdV1,
    },
    /// A successful applied proof disagrees with one named owner coordinate.
    #[error("applied materialization for `{repository_id}` disagrees at {coordinate:?}")]
    CoordinateMismatch {
        /// Repository whose proof cannot be joined.
        repository_id: LogicalRepositoryIdV1,
        /// Typed coordinate class that differs.
        coordinate: AppliedMaterializationCoordinate,
    },
    /// A removal receipt cannot provide the resident applied state required here.
    #[error(
        "repository `{repository_id}` supplied removal evidence where an applied state is required"
    )]
    RemovalReceipt {
        /// Repository carrying the non-applied receipt.
        repository_id: LogicalRepositoryIdV1,
    },
    /// The supplied state does not satisfy its pinned schema.
    #[error("materialization state for `{repository_id}` is invalid: {source}")]
    StateSchema {
        /// Repository owning the state.
        repository_id: LogicalRepositoryIdV1,
        /// Exact schema-validation failure.
        #[source]
        source: Box<pale_contracts::SchemaError>,
    },
    /// The supplied lineage does not satisfy its pinned schema.
    #[error("materialization lineage for `{repository_id}` is invalid: {source}")]
    LineageSchema {
        /// Repository owning the lineage.
        repository_id: LogicalRepositoryIdV1,
        /// Exact schema-validation failure.
        #[source]
        source: Box<pale_contracts::SchemaError>,
    },
    /// Desired-state reconstruction failed before it could be compared with applied state.
    #[error(transparent)]
    Plan(Box<PlanError>),
    /// One proof or closure cannot be encoded canonically.
    #[error("canonicalization failed at {coordinate:?}: {source}")]
    Canonicalization {
        /// Typed coordinate being reproduced.
        coordinate: AppliedMaterializationCoordinate,
        /// Canonical JSON failure.
        #[source]
        source: Box<canon::CanonError>,
    },
    /// The resident definition graph/body join violated one closed invariant.
    #[error("resident definition admission failed at {invariant:?} ({coordinate:?})")]
    ResidentDefinition {
        /// Closed invariant class rejected by the resident-definition owner.
        invariant: ResidentDefinitionInvariant,
        /// Exact semantic definition or relationship coordinate, when available.
        coordinate: Option<Box<ResidentDefinitionFailureCoordinate>>,
    },
    /// The pinned compiled-generation schema is absent or invalid.
    #[error(transparent)]
    Schema(Box<pale_contracts::SchemaError>),
    /// The core public aggregate rejected its owner-resolved preimage.
    #[error(transparent)]
    InvalidGeneration(Box<pale_core::records::InvalidResidentRecordV1>),
}

impl From<CompilationInvariantError> for CompiledGenerationAdmissionError {
    fn from(source: CompilationInvariantError) -> Self {
        Self::InvalidCompilation(Box::new(source))
    }
}

impl From<crate::workspace::WorkspaceError> for CompiledGenerationAdmissionError {
    fn from(source: crate::workspace::WorkspaceError) -> Self {
        Self::Workspace(Box::new(source))
    }
}

impl From<PlanError> for CompiledGenerationAdmissionError {
    fn from(source: PlanError) -> Self {
        Self::Plan(Box::new(source))
    }
}

impl From<pale_contracts::SchemaError> for CompiledGenerationAdmissionError {
    fn from(source: pale_contracts::SchemaError) -> Self {
        Self::Schema(Box::new(source))
    }
}

impl From<pale_core::records::InvalidResidentRecordV1> for CompiledGenerationAdmissionError {
    fn from(source: pale_core::records::InvalidResidentRecordV1) -> Self {
        Self::InvalidGeneration(Box::new(source))
    }
}

/// Closed resident-definition invariant classes exposed by generation admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentDefinitionInvariant {
    /// The private store does not carry its exact public generation identity.
    OwnerDigest,
    /// A semantic definition identity occurred more than once.
    DuplicateDefinition,
    /// An edge or child definition referenced no admitted node.
    MissingDefinition,
    /// An edge identity occurred more than once.
    DuplicateRelationship,
    /// A relationship disagreed with its closed endpoint or evidence algebra.
    RelationshipTyping,
    /// Package body and Package resource custody disagreed.
    PackageCustody,
    /// Workspace and composition-plan declarations disagreed.
    CompositionSource,
    /// Repository and Package-scope custody disagreed.
    RepositoryPackageScope,
    /// Graph node kind and aligned body kind disagreed.
    BodyAlignment,
    /// A derivation receipt named no current declaration node.
    ImpactSeed,
    /// Dense graph or arena handle capacity was exceeded.
    HandleSpace,
    /// The host could not reserve a declared bounded arena before admission.
    CapacityReservation,
    /// A process-local generation identity could not be allocated.
    GenerationIdentity,
    /// A generation-local graph or arena handle failed validation.
    GenerationFence,
}

/// Public semantic coordinate of one resident definition.
///
/// The coordinate preserves the closed identity family without exposing packed
/// graph handles or the private resident key representation. String members are
/// used only where the admitted initial-v1 contract itself defines an opaque
/// string identity; [`ResidentCompositionDefinitionKind`] keeps those domains
/// disjoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResidentDefinitionCoordinate {
    /// One compiled repository projection.
    Repository(LogicalRepositoryIdV1),
    /// One effective project profile.
    Profile(ProfileDigest),
    /// One resolved project instance.
    ProjectInstance(InstanceDigest),
    /// One private selection over a neutral experience path.
    ExecutionPathSelection(ExecutionPathSelectionIdV1),
    /// One admitted Package resource.
    PackageResource(crate::packages::PackageResourceUse),
    /// The admitted Package semantic closure.
    PackageClosure(PackageSemanticIrDigest),
    /// The admitted workspace semantic IR.
    WorkspaceSemanticIr(WorkspaceSemanticIrDigest),
    /// Exact publisher and protected-delivery evidence admitted for the workspace.
    WorkspaceAdmissionEvidence(WorkspaceAdmissionEvidenceDigest),
    /// Exact principal-relative admitted workspace universe.
    PrincipalUniverse(PrincipalAdmittedUniverseDigest),
    /// The admitted experience-composition plan.
    CompositionPlan(ExperienceCompositionPlanDigest),
    /// One workspace declaration.
    WorkspaceDeclaration {
        /// Module that owns the declaration.
        owner_module: WorkspaceModuleId,
        /// Module-local declaration identity.
        declaration: WorkspaceDeclarationRefV1,
    },
    /// One non-declaration causal workspace subject.
    WorkspaceCausal(ResidentWorkspaceCausalCoordinate),
    /// One composition definition whose wire contract owns an opaque string id.
    Composition {
        /// Closed composition definition family.
        kind: ResidentCompositionDefinitionKind,
        /// Exact admitted identity within that family.
        id: String,
    },
    /// One typed participant identity.
    Participant(ParticipantRefV1),
    /// One compiled-content identity.
    CompiledContent(CompiledContentIdentityV1),
    /// One source-content identity.
    SourceContent(SourceContentIdentityV1),
    /// One repository-scoped operation.
    Operation {
        /// Owning repository.
        repository: LogicalRepositoryIdV1,
        /// Exact Package operation identity.
        operation: PackageResourceRefV1,
    },
    /// One repository-scoped run mode.
    RunMode {
        /// Owning repository.
        repository: LogicalRepositoryIdV1,
        /// Exact Package run-mode identity.
        mode: PackageResourceRefV1,
    },
    /// One admitted lifecycle behavior.
    Lifecycle(LifecycleBehaviorIdV1),
}

/// Closed composition-definition families with opaque initial-v1 identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentCompositionDefinitionKind {
    /// Responsibility contract.
    Responsibility,
    /// Provider binding.
    ProviderBinding,
    /// Selected experience path.
    ExperiencePath,
    /// Entry contract.
    EntryContract,
    /// Request translation.
    RequestTranslation,
    /// Result translation.
    ResultTranslation,
    /// Termination contract.
    TerminationContract,
    /// Settlement contract.
    SettlementContract,
    /// Continuity boundary contract admitted for one selected experience path.
    ///
    /// This leaf carries only the typed path/owner contract. Replica, writer,
    /// checkpoint, replay, merge, readiness, and recovery-state topology remain
    /// outside the resident operational graph and under their native owner.
    ContinuityContract,
}

/// Canonical coordinate for a causal workspace subject that is not a declaration.
///
/// Declaration subjects have the distinct
/// [`ResidentDefinitionCoordinate::WorkspaceDeclaration`] representation. The
/// private field prevents callers from constructing two public coordinates for
/// the same resident declaration node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidentWorkspaceCausalCoordinate(WorkspaceDerivationSubjectV1);

impl ResidentWorkspaceCausalCoordinate {
    /// Construct a causal coordinate when `subject` is not a declaration.
    ///
    /// Returns [`None`] for [`WorkspaceDerivationSubjectV1::Declaration`], whose
    /// canonical resident coordinate is
    /// [`ResidentDefinitionCoordinate::WorkspaceDeclaration`].
    #[must_use]
    pub fn new(subject: WorkspaceDerivationSubjectV1) -> Option<Self> {
        (!matches!(subject, WorkspaceDerivationSubjectV1::Declaration { .. }))
            .then_some(Self(subject))
    }

    /// Borrow the exact admitted derivation subject.
    #[must_use]
    pub fn subject(&self) -> &WorkspaceDerivationSubjectV1 {
        &self.0
    }
}

/// A read-only successor-impact query found an impossible resident invariant.
#[derive(Debug, thiserror::Error)]
#[error("resident impact query failed at {invariant:?} ({coordinate:?})")]
pub struct ResidentImpactQueryError {
    invariant: ResidentDefinitionInvariant,
    coordinate: Option<Box<ResidentDefinitionFailureCoordinate>>,
}

impl ResidentImpactQueryError {
    pub(super) fn from_definition_store(source: &DefinitionStoreError) -> Self {
        Self {
            invariant: resident_definition_invariant(source),
            coordinate: definition_failure_coordinate(source).map(Box::new),
        }
    }

    /// Return the closed resident invariant that failed.
    #[must_use]
    pub const fn invariant(&self) -> ResidentDefinitionInvariant {
        self.invariant
    }

    /// Borrow the exact semantic coordinate when one was recoverable.
    #[must_use]
    pub fn coordinate(&self) -> Option<&ResidentDefinitionFailureCoordinate> {
        self.coordinate.as_deref()
    }
}

/// Bounded semantic location attached to a resident-definition failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResidentDefinitionFailureCoordinate {
    /// One definition failed admission.
    Definition(ResidentDefinitionCoordinate),
    /// One exact owner-to-dependency relationship failed admission.
    Relationship {
        /// Semantic owner of the relationship.
        owner: ResidentDefinitionCoordinate,
        /// Semantic dependency of the relationship.
        dependency: ResidentDefinitionCoordinate,
        /// Exact closed relationship kind.
        relation: ResidentRelation,
        /// Evidence discriminator not derivable from the relationship kind.
        evidence: ResidentRelationshipEvidenceCoordinate,
    },
}

/// Exact evidence discriminator for one resident relationship.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResidentRelationshipEvidenceCoordinate {
    /// Relationship semantics are completely carried by its closed kind.
    None,
    /// Exact authored workspace dependency kind.
    WorkspaceDependency(DeclarationDependencyKindV1),
    /// Exact operational composition contract identity.
    OperationalContract {
        /// Closed composition graph family.
        kind: CompositionGraphKindV1,
        /// Stable admitted edge identity.
        id: String,
    },
}

impl CompiledGenerationAdmissionError {
    pub(super) fn from_definition_store(source: &DefinitionStoreError) -> Self {
        let coordinate = definition_failure_coordinate(source);
        let invariant = resident_definition_invariant(source);
        Self::ResidentDefinition {
            invariant,
            coordinate: coordinate.map(Box::new),
        }
    }
}

fn resident_definition_invariant(source: &DefinitionStoreError) -> ResidentDefinitionInvariant {
    use pale_kernel::{ArenaError, GraphAccessError, GraphBuildError};
    match source {
        DefinitionStoreError::DuplicateDefinition { .. } => {
            ResidentDefinitionInvariant::DuplicateDefinition
        }
        DefinitionStoreError::DuplicateRelationship(_) => {
            ResidentDefinitionInvariant::DuplicateRelationship
        }
        DefinitionStoreError::InvalidRelationship(_) => {
            ResidentDefinitionInvariant::RelationshipTyping
        }
        DefinitionStoreError::OrphanPackageBody | DefinitionStoreError::Canonical(_) => {
            ResidentDefinitionInvariant::PackageCustody
        }
        DefinitionStoreError::CompositionSourceMismatch
        | DefinitionStoreError::CompositionNormalization { .. } => {
            ResidentDefinitionInvariant::CompositionSource
        }
        DefinitionStoreError::MissingRepositoryPackageScope { .. }
        | DefinitionStoreError::RepositoryPackageProjectionMismatch { .. }
        | DefinitionStoreError::OrphanRepositoryPackageScope => {
            ResidentDefinitionInvariant::RepositoryPackageScope
        }
        DefinitionStoreError::MissingOwnerNode { .. }
        | DefinitionStoreError::MissingWorkspaceDeclaration { .. }
        | DefinitionStoreError::MissingCausalInput { .. } => {
            ResidentDefinitionInvariant::MissingDefinition
        }
        DefinitionStoreError::MissingImpactSeed { .. } => ResidentDefinitionInvariant::ImpactSeed,
        DefinitionStoreError::BodyKindMismatch { .. }
        | DefinitionStoreError::BodyIdentityMismatch { .. }
        | DefinitionStoreError::Arena(ArenaError::AlignmentMismatch) => {
            ResidentDefinitionInvariant::BodyAlignment
        }
        DefinitionStoreError::HandleSpace
        | DefinitionStoreError::Arena(ArenaError::InvalidCapacity) => {
            ResidentDefinitionInvariant::HandleSpace
        }
        DefinitionStoreError::Arena(ArenaError::CapacityUnavailable) => {
            ResidentDefinitionInvariant::CapacityReservation
        }
        DefinitionStoreError::Arena(ArenaError::IdentityExhausted) => {
            ResidentDefinitionInvariant::GenerationIdentity
        }
        DefinitionStoreError::Access(
            GraphAccessError::ForeignGeneration
            | GraphAccessError::OutOfRange
            | GraphAccessError::NodeSet(_),
        )
        | DefinitionStoreError::Arena(
            ArenaError::Foreign
            | ArenaError::Stale
            | ArenaError::Exhausted
            | ArenaError::SlotGenerationExhausted
            | ArenaError::ResetEpochExhausted,
        ) => ResidentDefinitionInvariant::GenerationFence,
        DefinitionStoreError::Graph(error) => match error.as_ref() {
            GraphBuildError::MissingNode(_) => ResidentDefinitionInvariant::MissingDefinition,
            GraphBuildError::HandleSpace => ResidentDefinitionInvariant::HandleSpace,
            GraphBuildError::DuplicateNode(_) => ResidentDefinitionInvariant::DuplicateDefinition,
            GraphBuildError::DuplicateEdge(_) => ResidentDefinitionInvariant::DuplicateRelationship,
            GraphBuildError::GenerationExhausted => ResidentDefinitionInvariant::GenerationIdentity,
        },
    }
}

fn definition_failure_coordinate(
    source: &DefinitionStoreError,
) -> Option<ResidentDefinitionFailureCoordinate> {
    match source {
        DefinitionStoreError::DuplicateDefinition { key }
        | DefinitionStoreError::MissingImpactSeed { key }
        | DefinitionStoreError::MissingWorkspaceDeclaration { key }
        | DefinitionStoreError::MissingCausalInput { owner: key } => {
            resident_definition_coordinate(key).map(ResidentDefinitionFailureCoordinate::Definition)
        }
        DefinitionStoreError::DuplicateRelationship(failure)
        | DefinitionStoreError::InvalidRelationship(failure) => {
            Some(ResidentDefinitionFailureCoordinate::Relationship {
                owner: resident_definition_coordinate(&failure.owner)?,
                dependency: resident_definition_coordinate(&failure.dependency)?,
                relation: failure.relation,
                evidence: resident_evidence_coordinate(&failure.evidence),
            })
        }
        DefinitionStoreError::MissingOwnerNode { key }
        | DefinitionStoreError::BodyKindMismatch { key }
        | DefinitionStoreError::BodyIdentityMismatch { key } => key
            .as_deref()
            .and_then(resident_definition_coordinate)
            .map(ResidentDefinitionFailureCoordinate::Definition),
        DefinitionStoreError::Graph(error) => match error.as_ref() {
            pale_kernel::GraphBuildError::DuplicateNode(key)
            | pale_kernel::GraphBuildError::MissingNode(key) => resident_definition_coordinate(key)
                .map(ResidentDefinitionFailureCoordinate::Definition),
            _ => None,
        },
        DefinitionStoreError::MissingRepositoryPackageScope { repository }
        | DefinitionStoreError::RepositoryPackageProjectionMismatch { repository } => {
            Some(ResidentDefinitionFailureCoordinate::Definition(
                ResidentDefinitionCoordinate::Repository(repository.clone()),
            ))
        }
        _ => None,
    }
}

pub(in crate::resident) fn resident_definition_coordinate(
    key: &ResidentNodeKey,
) -> Option<ResidentDefinitionCoordinate> {
    let composition = |kind, id: &String| ResidentDefinitionCoordinate::Composition {
        kind,
        id: id.clone(),
    };
    Some(match key {
        ResidentNodeKey::Repository(id) => ResidentDefinitionCoordinate::Repository(id.clone()),
        ResidentNodeKey::Profile(digest) => ResidentDefinitionCoordinate::Profile(digest.clone()),
        ResidentNodeKey::ProjectInstance(digest) => {
            ResidentDefinitionCoordinate::ProjectInstance(digest.clone())
        }
        ResidentNodeKey::ExecutionPathSelection(id) => {
            ResidentDefinitionCoordinate::ExecutionPathSelection(id.clone())
        }
        ResidentNodeKey::PackageResource(resource) => {
            ResidentDefinitionCoordinate::PackageResource(resource.clone())
        }
        ResidentNodeKey::PackageClosure(digest) => {
            ResidentDefinitionCoordinate::PackageClosure(digest.clone())
        }
        ResidentNodeKey::WorkspaceSemanticIr(digest) => {
            ResidentDefinitionCoordinate::WorkspaceSemanticIr(digest.clone())
        }
        ResidentNodeKey::WorkspaceAdmissionEvidence(digest) => {
            ResidentDefinitionCoordinate::WorkspaceAdmissionEvidence(digest.clone())
        }
        ResidentNodeKey::PrincipalUniverse(digest) => {
            ResidentDefinitionCoordinate::PrincipalUniverse(digest.clone())
        }
        ResidentNodeKey::CompositionPlan(digest) => {
            ResidentDefinitionCoordinate::CompositionPlan(digest.clone())
        }
        ResidentNodeKey::WorkspaceDeclaration {
            owner_module,
            declaration,
        } => ResidentDefinitionCoordinate::WorkspaceDeclaration {
            owner_module: owner_module.clone(),
            declaration: declaration.clone(),
        },
        ResidentNodeKey::WorkspaceCausal(subject) => ResidentDefinitionCoordinate::WorkspaceCausal(
            ResidentWorkspaceCausalCoordinate::new(subject.clone())?,
        ),
        ResidentNodeKey::Responsibility(id) => {
            composition(ResidentCompositionDefinitionKind::Responsibility, id)
        }
        ResidentNodeKey::Participant(participant) => {
            ResidentDefinitionCoordinate::Participant(participant.clone())
        }
        ResidentNodeKey::ProviderBinding(id) => {
            composition(ResidentCompositionDefinitionKind::ProviderBinding, id)
        }
        ResidentNodeKey::ExperiencePath(id) => {
            composition(ResidentCompositionDefinitionKind::ExperiencePath, id)
        }
        ResidentNodeKey::EntryContract(id) => {
            composition(ResidentCompositionDefinitionKind::EntryContract, id)
        }
        ResidentNodeKey::RequestTranslation(id) => {
            composition(ResidentCompositionDefinitionKind::RequestTranslation, id)
        }
        ResidentNodeKey::ResultTranslation(id) => {
            composition(ResidentCompositionDefinitionKind::ResultTranslation, id)
        }
        ResidentNodeKey::TerminationContract(id) => {
            composition(ResidentCompositionDefinitionKind::TerminationContract, id)
        }
        ResidentNodeKey::SettlementContract(id) => {
            composition(ResidentCompositionDefinitionKind::SettlementContract, id)
        }
        ResidentNodeKey::ContinuityContract(id) => {
            composition(ResidentCompositionDefinitionKind::ContinuityContract, id)
        }
        ResidentNodeKey::CompiledContent(key) => {
            ResidentDefinitionCoordinate::CompiledContent(key.0.clone())
        }
        ResidentNodeKey::SourceContent(key) => {
            ResidentDefinitionCoordinate::SourceContent(key.0.clone())
        }
        ResidentNodeKey::Operation {
            repository,
            operation,
        } => ResidentDefinitionCoordinate::Operation {
            repository: repository.clone(),
            operation: operation.clone(),
        },
        ResidentNodeKey::RunMode { repository, mode } => ResidentDefinitionCoordinate::RunMode {
            repository: repository.clone(),
            mode: mode.clone(),
        },
        ResidentNodeKey::Lifecycle(id) => ResidentDefinitionCoordinate::Lifecycle(id.clone()),
    })
}

pub(in crate::resident) fn resident_evidence_coordinate(
    evidence: &ResidentEdgeEvidence,
) -> ResidentRelationshipEvidenceCoordinate {
    match evidence {
        ResidentEdgeEvidence::None => ResidentRelationshipEvidenceCoordinate::None,
        ResidentEdgeEvidence::WorkspaceDependency(kind) => {
            ResidentRelationshipEvidenceCoordinate::WorkspaceDependency(*kind)
        }
        ResidentEdgeEvidence::OperationalContract(edge) => {
            ResidentRelationshipEvidenceCoordinate::OperationalContract {
                kind: edge.kind(),
                id: edge.id().to_string(),
            }
        }
    }
}
