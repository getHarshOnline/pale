//! Generation-fenced resident definitions over one packed dependency graph.
//!
//! Lowering consumes the one-shot compiler body into one finite definition
//! system whose aligned vertex, edge, and body law is rendered below. For
//! compiled generation `G`, `V`, `E`, and `B` are definition vertices,
//! owner-to-dependency edges, and bodies. Public observation runs from subject
//! to observer, while `E_resident` is the reverse dependency edge used for
//! invalidation.

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/resident-definition-store.display.mathml"
))]
//!
//! [`PackedGraph`] owns the
//! only live operational/alignment adjacency and [`GenerationArena`] owns each
//! complete definition body exactly once. Reverse impact is therefore the
//! graph's least fixed point over incoming edges; it is not reconstructed from
//! a second map. Construction follows the bound on [`PackedGraph`], while keyed
//! lookup is logarithmic and aligned-body resolution is constant-time.
//!
//! Graph and arena handles are process-local capabilities fenced to this build.
//! They express neither semantic authority nor activation, release, native
//! observation, or effect. Every successor rebuilds both structures, making
//! predecessor handles invalid by construction.
//! Cycles remain an upstream semantic-admission decision: this projection
//! preserves admitted recursion and relies on [`PackedGraph`] strongly
//! connected-component and visited-set laws for finite traversal. It does not
//! invent a second Package, workspace, or composition cycle policy.
mod lower;
mod successor;
mod types;
mod validation;

use pale_core::records::{
    AdmittedLifecycleBehaviorV1, CompiledGenerationDigest, CompositionGraphViewV1,
    CompositionReasonPathV1, ExperienceCompositionPlanDigest, ExperiencePathV1,
    LifecycleBehaviorIdV1, LogicalRepositoryIdV1, ModeAxis, PackageResourceRefV1,
    ProviderBindingV1, WorkspaceDerivationNodeV1, WorkspaceDerivationSourceV1,
};
use pale_kernel::{
    AdaptiveNodeSetPolicy, ArenaError, GenerationArena, GraphAccessError, GraphBuildError,
    GraphEdge, GraphNode, NodeHandle, PackedGraph,
};

use crate::compile::{CompiledRepo, ResidentCompilationParts};
use crate::semantic::{ResolvedOperation, ResolvedRunMode};
use crate::workspace::ResidentWorkspaceDerivation;

use super::generation::AppliedRepositoryMaterialization;
use lower::{LoweredDefinitions, ResidentDefinitionDraft};
pub use types::ResidentRelation;
use types::{ResidentDefinition, ResidentEdgeId, ResidentNodeKind, ResidentNodeQuery};
pub(in crate::resident) use types::{ResidentEdgeEvidence, ResidentNodeKey};
use validation::{body_matches_key, canonicalize_edges, validate_relationships};

type DefinitionGraph = PackedGraph<
    ResidentNodeKey,
    ResidentNodeKind,
    ResidentEdgeId,
    ResidentEdgeEvidence,
    ResidentRelation,
>;

/// One immutable resident body store and its sole live adjacency owner.
pub(super) struct ResidentDefinitionStore {
    owner: CompiledGenerationDigest,
    graph: DefinitionGraph,
    arena: GenerationArena<ResidentDefinition>,
    derivation: ResidentWorkspaceDerivation,
}

#[derive(Debug)]
pub(super) struct ResidentRelationshipFailure {
    pub(super) owner: ResidentNodeKey,
    pub(super) dependency: ResidentNodeKey,
    pub(super) relation: ResidentRelation,
    pub(super) evidence: ResidentEdgeEvidence,
}

impl ResidentDefinitionStore {
    /// Consume verified compiler bodies into one generation-fenced store.
    pub(super) fn build(
        owner: CompiledGenerationDigest,
        parts: ResidentCompilationParts,
        materializations: Vec<AppliedRepositoryMaterialization>,
        set_policy: AdaptiveNodeSetPolicy,
    ) -> Result<Self, DefinitionStoreError> {
        let LoweredDefinitions {
            definitions,
            mut edges,
            derivation,
        } = lower::lower(parts, materializations)?;
        validate_relationships(&definitions, &edges)?;
        canonicalize_edges(&mut edges)?;

        let nodes = definitions
            .iter()
            .map(|(key, kind, _)| GraphNode {
                key: key.clone(),
                payload: *kind,
            })
            .collect();
        let graph_edges = edges
            .into_iter()
            .enumerate()
            .map(|(position, edge)| {
                let id = u32::try_from(position).map_err(|_| DefinitionStoreError::HandleSpace)?;
                Ok(GraphEdge {
                    id: ResidentEdgeId(id),
                    source: edge.source,
                    target: edge.target,
                    payload: edge.evidence,
                    reason: edge.relation,
                })
            })
            .collect::<Result<Vec<_>, DefinitionStoreError>>()?;
        let graph = PackedGraph::build(nodes, graph_edges, set_policy)?;

        let bodies = definitions
            .into_iter()
            .map(|(key, expected_kind, draft)| {
                let body = match draft {
                    ResidentDefinitionDraft::Complete(body) => body,
                    ResidentDefinitionDraft::Operation {
                        repository,
                        position,
                    } => ResidentDefinition::Operation {
                        repository: graph.find(&repository).ok_or_else(|| {
                            DefinitionStoreError::MissingOwnerNode {
                                key: Some(Box::new(repository)),
                            }
                        })?,
                        position,
                    },
                    ResidentDefinitionDraft::RunMode {
                        repository,
                        position,
                    } => ResidentDefinition::RunMode {
                        repository: graph.find(&repository).ok_or_else(|| {
                            DefinitionStoreError::MissingOwnerNode {
                                key: Some(Box::new(repository)),
                            }
                        })?,
                        position,
                    },
                    ResidentDefinitionDraft::Lifecycle { profile, position } => {
                        ResidentDefinition::Lifecycle {
                            profile: graph.find(&profile).ok_or_else(|| {
                                DefinitionStoreError::MissingOwnerNode {
                                    key: Some(Box::new(profile)),
                                }
                            })?,
                            position,
                        }
                    }
                    ResidentDefinitionDraft::ExecutionPathSelection { profile, position } => {
                        ResidentDefinition::ExecutionPathSelection {
                            profile: graph.find(&profile).ok_or_else(|| {
                                DefinitionStoreError::MissingOwnerNode {
                                    key: Some(Box::new(profile)),
                                }
                            })?,
                            position,
                        }
                    }
                };
                if body.kind() != expected_kind {
                    return Err(DefinitionStoreError::BodyKindMismatch {
                        key: Some(Box::new(key)),
                    });
                }
                if !body_matches_key(&graph, &key, &body)? {
                    return Err(DefinitionStoreError::BodyIdentityMismatch {
                        key: Some(Box::new(key)),
                    });
                }
                Ok(body)
            })
            .collect::<Result<Vec<_>, DefinitionStoreError>>()?;
        let arena = GenerationArena::seal_for_graph(&graph, bodies)?;
        let store = Self {
            owner,
            graph,
            arena,
            derivation,
        };
        store.validate_child_coordinates()?;
        Ok(store)
    }

    pub(super) fn owner(&self) -> &CompiledGenerationDigest {
        &self.owner
    }

    pub(super) fn derivation(&self) -> &ResidentWorkspaceDerivation {
        &self.derivation
    }

    pub(super) fn workspace_derivation_sources(&self) -> &[WorkspaceDerivationSourceV1] {
        self.derivation.sources()
    }

    pub(super) fn workspace_derivation_nodes(&self) -> &[WorkspaceDerivationNodeV1] {
        self.derivation.causal_nodes()
    }

    pub(super) fn composition_reason_paths(
        &self,
        digest: &ExperienceCompositionPlanDigest,
    ) -> Option<&[CompositionReasonPathV1]> {
        match self.resolve_query(ResidentNodeQuery::CompositionPlan(digest))? {
            ResidentDefinition::CompositionPlan(plan) => Some(&plan.reason_paths),
            _ => None,
        }
    }

    pub(super) fn composition_graph_views(
        &self,
        digest: &ExperienceCompositionPlanDigest,
    ) -> Option<&[CompositionGraphViewV1]> {
        match self.resolve_query(ResidentNodeQuery::CompositionPlan(digest))? {
            ResidentDefinition::CompositionPlan(plan) => Some(&plan.graph_views),
            _ => None,
        }
    }

    pub(super) fn repository(
        &self,
        repository_id: &LogicalRepositoryIdV1,
    ) -> Option<&CompiledRepo> {
        match self.resolve_query(ResidentNodeQuery::Repository(repository_id))? {
            ResidentDefinition::Repository(definition) => Some(&definition.compiled),
            _ => None,
        }
    }

    pub(super) fn materialization(
        &self,
        repository_id: &LogicalRepositoryIdV1,
    ) -> Option<&AppliedRepositoryMaterialization> {
        match self.resolve_query(ResidentNodeQuery::Repository(repository_id))? {
            ResidentDefinition::Repository(definition) => Some(&definition.materialization),
            _ => None,
        }
    }

    pub(super) fn provider_binding(&self, binding_id: &str) -> Option<&ProviderBindingV1> {
        match self.resolve_query(ResidentNodeQuery::ProviderBinding(binding_id))? {
            ResidentDefinition::ProviderBinding(binding) => Some(binding),
            _ => None,
        }
    }

    pub(super) fn experience_path(&self, path_id: &str) -> Option<&ExperiencePathV1> {
        match self.resolve_query(ResidentNodeQuery::ExperiencePath(path_id))? {
            ResidentDefinition::ExperiencePath(path) => Some(path),
            _ => None,
        }
    }

    pub(super) fn operation(
        &self,
        repository_id: &LogicalRepositoryIdV1,
        operation_id: &PackageResourceRefV1,
    ) -> Option<&ResolvedOperation> {
        let body = self.resolve_query(ResidentNodeQuery::Operation {
            repository: repository_id,
            operation: operation_id,
        })?;
        let ResidentDefinition::Operation {
            repository,
            position,
        } = body
        else {
            return None;
        };
        let ResidentDefinition::Repository(repository) = self.resolve(*repository).ok()? else {
            return None;
        };
        repository
            .compiled
            .semantic_plan()
            .operations
            .get(*position)
    }

    pub(super) fn run_mode(
        &self,
        repository_id: &LogicalRepositoryIdV1,
        mode_id: &PackageResourceRefV1,
    ) -> Option<&ResolvedRunMode> {
        let body = self.resolve_query(ResidentNodeQuery::RunMode {
            repository: repository_id,
            mode: mode_id,
        })?;
        let ResidentDefinition::RunMode {
            repository,
            position,
        } = body
        else {
            return None;
        };
        let ResidentDefinition::Repository(repository) = self.resolve(*repository).ok()? else {
            return None;
        };
        repository.compiled.semantic_plan().run_modes.get(*position)
    }

    pub(super) fn lifecycle_behavior(
        &self,
        behavior_id: &LifecycleBehaviorIdV1,
    ) -> Option<&AdmittedLifecycleBehaviorV1> {
        let ResidentDefinition::Lifecycle { profile, position } =
            self.resolve_query(ResidentNodeQuery::Lifecycle(behavior_id))?
        else {
            return None;
        };
        let ResidentDefinition::Profile(profile) = self.resolve(*profile).ok()? else {
            return None;
        };
        profile.lifecycle.get(*position)
    }

    pub(super) fn mode_default(
        &self,
        repository_id: &LogicalRepositoryIdV1,
        axis: ModeAxis,
    ) -> Option<&PackageResourceRefV1> {
        self.repository(repository_id)?
            .semantic_plan()
            .mode_defaults
            .get(&axis)
    }

    fn find_query(&self, query: ResidentNodeQuery<'_>) -> Option<NodeHandle> {
        self.graph.find_by(|candidate| candidate.cmp_query(query))
    }

    fn resolve_query(&self, query: ResidentNodeQuery<'_>) -> Option<&ResidentDefinition> {
        self.resolve(self.find_query(query)?).ok()
    }

    fn resolve(&self, handle: NodeHandle) -> Result<&ResidentDefinition, DefinitionStoreError> {
        self.graph.node_payload(handle)?;
        let arena_handle = self
            .arena
            .handle(handle.index() as usize)
            .ok_or(DefinitionStoreError::MissingOwnerNode { key: None })?;
        self.arena.get(arena_handle).map_err(Into::into)
    }
}

/// Resident-definition construction rejected an incomplete or ambiguous join.
#[derive(Debug, thiserror::Error)]
pub(super) enum DefinitionStoreError {
    #[error("resident definition identity occurs more than once")]
    DuplicateDefinition { key: Box<ResidentNodeKey> },
    #[error("resident relationship occurs more than once")]
    DuplicateRelationship(Box<ResidentRelationshipFailure>),
    #[error("resident relationship violates its closed endpoint or evidence algebra")]
    InvalidRelationship(Box<ResidentRelationshipFailure>),
    #[error("Package body exists without its admitted resource definition")]
    OrphanPackageBody,
    #[error("workspace and composition-plan declarations disagree")]
    CompositionSourceMismatch,
    #[error("workspace composition cannot be normalized for resident admission")]
    CompositionNormalization {
        source: Box<crate::composition::CompositionError>,
    },
    #[error("repository `{repository}` has no admitted Package scope")]
    MissingRepositoryPackageScope { repository: LogicalRepositoryIdV1 },
    #[error("repository `{repository}` Package scope disagrees with its compiled semantic plan")]
    RepositoryPackageProjectionMismatch { repository: LogicalRepositoryIdV1 },
    #[error("an admitted Package repository scope has no compiled repository")]
    OrphanRepositoryPackageScope,
    #[error("a resident child definition has no repository owner node")]
    MissingOwnerNode { key: Option<Box<ResidentNodeKey>> },
    #[error("admitted causal graph names no retained workspace declaration")]
    MissingWorkspaceDeclaration { key: Box<ResidentNodeKey> },
    #[error("admitted causal graph names no predecessor fact")]
    MissingCausalInput { owner: Box<ResidentNodeKey> },
    #[error("a derivation transition cannot map one current impact seed")]
    MissingImpactSeed { key: Box<ResidentNodeKey> },
    #[error("resident graph node kind and aligned body kind disagree")]
    BodyKindMismatch { key: Option<Box<ResidentNodeKey>> },
    #[error("resident graph node identity and aligned body identity disagree")]
    BodyIdentityMismatch { key: Option<Box<ResidentNodeKey>> },
    #[error("resident relationship count exceeds dense handle space")]
    HandleSpace,
    #[error(transparent)]
    Canonical(#[from] pale_core::canon::CanonError),
    #[error(transparent)]
    Graph(Box<GraphBuildError<ResidentNodeKey, ResidentEdgeId>>),
    #[error(transparent)]
    Access(#[from] GraphAccessError),
    #[error(transparent)]
    Arena(#[from] ArenaError),
}

impl From<GraphBuildError<ResidentNodeKey, ResidentEdgeId>> for DefinitionStoreError {
    fn from(source: GraphBuildError<ResidentNodeKey, ResidentEdgeId>) -> Self {
        Self::Graph(Box::new(source))
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/definition.rs"]
mod tests;
