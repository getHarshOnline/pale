//! Immutable source-index generation lifecycle and atomic query visibility.

use std::collections::{btree_map::Entry, BTreeMap};
use std::sync::{Arc, RwLock};

use pale_core::{
    canon::{to_canonical_string, CanonError},
    records::{ArtifactDigest, ImmutableRecordReferenceV1, ProviderIndexCoordinateV1},
};
use pale_kernel::{AdaptiveNodeSetPolicy, EdgeHandle, PackedGraph};
use pale_ports::SourceIndexLifecyclePort;

use super::{
    graph_inputs, AdmittedSourceFrontier, SourceCoverageReport, SourceGraphDeclarationError,
    SourceGraphEdge, SourceGraphNode, SourceIndexScope, SourcePackedGraph,
};

/// Maximum immutable generations retained by one in-process registry.
pub const MAX_SOURCE_INDEX_GENERATIONS: usize = 1_024;

/// Complete candidate for one unqueryable building generation.
pub struct SourceIndexCandidate {
    /// Exact isolation coordinate.
    pub scope: SourceIndexScope,
    /// Provider generation and exact source frontier.
    pub coordinate: ProviderIndexCoordinateV1,
    /// Reproved source-frontier constituents.
    pub frontier: AdmittedSourceFrontier,
    /// Complete five-axis coverage partition.
    pub coverage: SourceCoverageReport,
    /// Provider-owned graph nodes.
    pub nodes: Vec<SourceGraphNode>,
    /// Provider-owned cause-to-dependent relationships.
    pub edges: Vec<SourceGraphEdge>,
}

/// Phase-typed in-progress generation.
///
/// This type has no query method and is never placed in the query registry.
/// Only [`SourceIndexAdministrator::publish`] can consume it into a ready
/// immutable generation. In the queryability law, `g` is a source-index
/// generation and `s` is its exact [`SourceIndexScope`]. A request can query
/// `g` only when the generation is ready, is the active generation for `s`,
/// retains that same scope, and matches the request's source frontier.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/source-index-queryability.display.mathml"
))]
pub struct BuildingSourceIndex {
    candidate: SourceIndexCandidate,
    coordinate_digest: ArtifactDigest,
    scope_digest: ArtifactDigest,
}

impl BuildingSourceIndex {
    /// Exact generation coordinate being built.
    #[must_use]
    pub fn coordinate(&self) -> &ProviderIndexCoordinateV1 {
        &self.candidate.coordinate
    }
}

/// Stable handle returned after one generation is atomically published.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadySourceIndexCoordinate {
    /// Internal exact coordinate digest.
    pub coordinate_digest: ArtifactDigest,
    /// Exact provider/index/source-frontier coordinate.
    pub coordinate: ProviderIndexCoordinateV1,
}

/// Lifecycle disposition kept separate from immutable graph bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceIndexLifecycleState {
    /// Generation may be selected for queries.
    Ready,
    /// A successor was selected; historical identity remains intact.
    Superseded,
    /// Exact invalidation evidence fenced future queries.
    Invalidated,
    /// Administrative retention ended while historical references remain.
    Retired,
    /// Derived graph bytes were deleted.
    Removed,
}

/// Administrative transition over already published generations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceIndexTransition {
    /// Atomically select a ready generation for its exact scope.
    Activate {
        /// Exact scope expected by the caller.
        scope: SourceIndexScope,
        /// Ready generation to select.
        coordinate_digest: ArtifactDigest,
        /// Immutable transition evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// Select a ready successor and retain the predecessor as superseded.
    Supersede {
        /// Exact scope shared by both generations.
        scope: SourceIndexScope,
        /// Currently active predecessor.
        predecessor: ArtifactDigest,
        /// Already-published ready successor.
        successor: ArtifactDigest,
        /// Immutable transition evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// Re-select a retained superseded generation.
    Rollback {
        /// Exact scope.
        scope: SourceIndexScope,
        /// Retained predecessor to restore.
        coordinate_digest: ArtifactDigest,
        /// Immutable rollback evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// Fence a generation because its source/provider frontier changed.
    Invalidate {
        /// Generation being invalidated.
        coordinate_digest: ArtifactDigest,
        /// Exact invalidation evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// End query eligibility without deleting historical graph bytes.
    Retire {
        /// Generation being retired.
        coordinate_digest: ArtifactDigest,
        /// Exact retirement evidence.
        evidence: ImmutableRecordReferenceV1,
    },
    /// Delete retained derived graph bytes after retirement or invalidation.
    Remove {
        /// Generation whose bytes are deleted.
        coordinate_digest: ArtifactDigest,
        /// Exact removal evidence.
        evidence: ImmutableRecordReferenceV1,
    },
}

pub(super) struct ReadySourceIndex {
    pub(super) scope: SourceIndexScope,
    pub(super) scope_digest: ArtifactDigest,
    pub(super) coordinate: ProviderIndexCoordinateV1,
    pub(super) coordinate_digest: ArtifactDigest,
    pub(super) frontier: AdmittedSourceFrontier,
    pub(super) coverage: SourceCoverageReport,
    pub(super) graph: SourcePackedGraph,
    pub(super) edge_by_reference: BTreeMap<ImmutableRecordReferenceV1, EdgeHandle>,
}

pub(super) enum SourceIndexEntry {
    Retained {
        index: Arc<ReadySourceIndex>,
        state: SourceIndexLifecycleState,
        selection_evidence: Option<ImmutableRecordReferenceV1>,
        terminal_evidence: Option<ImmutableRecordReferenceV1>,
    },
    Removed {
        _coordinate: Box<ProviderIndexCoordinateV1>,
        _scope_digest: ArtifactDigest,
        _prior_terminal_evidence: Option<ImmutableRecordReferenceV1>,
        _removal_evidence: ImmutableRecordReferenceV1,
    },
}

pub(super) struct SourceIndexRegistry {
    pub(super) generations: BTreeMap<ArtifactDigest, SourceIndexEntry>,
    pub(super) active_by_scope: BTreeMap<ArtifactDigest, ArtifactDigest>,
}

impl SourceIndexRegistry {
    fn new() -> Self {
        Self {
            generations: BTreeMap::new(),
            active_by_scope: BTreeMap::new(),
        }
    }
}

/// Administrative half of one source-intelligence subsystem.
///
/// Cloning this handle shares lifecycle custody; it does not grant the query
/// methods available on [`super::SourceIntelligenceQueryService`].
#[derive(Clone)]
pub struct SourceIndexAdministrator {
    pub(super) registry: Arc<RwLock<SourceIndexRegistry>>,
    set_policy: AdaptiveNodeSetPolicy,
}

impl SourceIndexAdministrator {
    pub(super) fn new(set_policy: AdaptiveNodeSetPolicy) -> Self {
        Self {
            registry: Arc::new(RwLock::new(SourceIndexRegistry::new())),
            set_policy,
        }
    }

    /// Admit a generation candidate into the unqueryable building phase.
    ///
    /// # Errors
    ///
    /// Returns [`SourceIndexLifecycleError`] when the candidate scope,
    /// frontier, coverage, declaration, or coordinate is inconsistent.
    pub fn begin(
        &self,
        candidate: SourceIndexCandidate,
    ) -> Result<BuildingSourceIndex, SourceIndexLifecycleError> {
        begin_candidate(&self.registry, candidate)
    }

    /// Atomically publish one complete immutable generation.
    ///
    /// # Errors
    ///
    /// Returns [`SourceIndexLifecycleError`] when registry access fails or the
    /// generation coordinate already names different immutable bytes.
    pub fn publish(
        &self,
        building: BuildingSourceIndex,
    ) -> Result<ReadySourceIndexCoordinate, SourceIndexLifecycleError> {
        publish_building(&self.registry, self.set_policy, building)
    }

    /// Apply one exact administrative transition.
    ///
    /// # Errors
    ///
    /// Returns [`SourceIndexLifecycleError`] when the named generation, scope,
    /// frontier, predecessor, or requested state transition is invalid.
    pub fn transition(
        &self,
        transition: SourceIndexTransition,
    ) -> Result<(), SourceIndexLifecycleError> {
        apply_transition(&self.registry, transition)
    }

    /// Observe lifecycle state without exposing derived graph contents.
    ///
    /// # Errors
    ///
    /// Returns [`SourceIndexLifecycleError::Poisoned`] when a panicking writer
    /// invalidated the shared registry guard.
    pub fn state(
        &self,
        coordinate_digest: &ArtifactDigest,
    ) -> Result<Option<SourceIndexLifecycleState>, SourceIndexLifecycleError> {
        Ok(self
            .registry
            .read()
            .map_err(|_| SourceIndexLifecycleError::Poisoned)?
            .generations
            .get(coordinate_digest)
            .map(|entry| match entry {
                SourceIndexEntry::Retained { state, .. } => *state,
                SourceIndexEntry::Removed { .. } => SourceIndexLifecycleState::Removed,
            }))
    }
}

impl SourceIndexLifecyclePort for SourceIndexAdministrator {
    type Candidate = SourceIndexCandidate;
    type Building = BuildingSourceIndex;
    type Ready = ReadySourceIndexCoordinate;
    type Transition = SourceIndexTransition;
    type Error = SourceIndexLifecycleError;

    fn begin(&self, candidate: Self::Candidate) -> Result<Self::Building, Self::Error> {
        SourceIndexAdministrator::begin(self, candidate)
    }

    fn publish(&self, building: Self::Building) -> Result<Self::Ready, Self::Error> {
        SourceIndexAdministrator::publish(self, building)
    }

    fn transition(&self, transition: Self::Transition) -> Result<(), Self::Error> {
        SourceIndexAdministrator::transition(self, transition)
    }
}

fn begin_candidate(
    registry: &Arc<RwLock<SourceIndexRegistry>>,
    candidate: SourceIndexCandidate,
) -> Result<BuildingSourceIndex, SourceIndexLifecycleError> {
    if candidate.scope.repository_id
        != candidate
            .coordinate
            .repository_source_frontier
            .repository_id
        || candidate.scope.repository_id != candidate.frontier.coordinate().repository_id
    {
        return Err(SourceIndexLifecycleError::RepositoryMismatch);
    }
    if candidate.scope.provider_index_owner != candidate.coordinate.provider_index_owner {
        return Err(SourceIndexLifecycleError::ProviderMismatch);
    }
    if candidate.coordinate.repository_source_frontier != *candidate.frontier.coordinate() {
        return Err(SourceIndexLifecycleError::FrontierMismatch);
    }
    let coordinate_digest = coordinate_digest(&candidate.coordinate)?;
    let scope_digest = candidate.scope.lookup_digest()?;
    if registry
        .read()
        .map_err(|_| SourceIndexLifecycleError::Poisoned)?
        .generations
        .contains_key(&coordinate_digest)
    {
        return Err(SourceIndexLifecycleError::GenerationCollision);
    }
    if candidate.nodes.len() > super::MAX_SOURCE_GRAPH_NODES {
        return Err(SourceIndexLifecycleError::GraphDeclaration(
            SourceGraphDeclarationError::TooManyNodes(candidate.nodes.len()),
        ));
    }
    if candidate.edges.len() > super::MAX_SOURCE_GRAPH_EDGES {
        return Err(SourceIndexLifecycleError::GraphDeclaration(
            SourceGraphDeclarationError::TooManyEdges(candidate.edges.len()),
        ));
    }
    for node in &candidate.nodes {
        if let Some(location) = &node.location {
            location.validate()?;
            if !candidate.frontier.admits_path(&location.path) {
                return Err(SourceIndexLifecycleError::GraphDeclaration(
                    SourceGraphDeclarationError::LocationOutsideFrontier(location.path.clone()),
                ));
            }
        }
    }
    Ok(BuildingSourceIndex {
        candidate,
        coordinate_digest,
        scope_digest,
    })
}

fn publish_building(
    registry: &Arc<RwLock<SourceIndexRegistry>>,
    set_policy: AdaptiveNodeSetPolicy,
    building: BuildingSourceIndex,
) -> Result<ReadySourceIndexCoordinate, SourceIndexLifecycleError> {
    let SourceIndexCandidate {
        scope,
        coordinate,
        frontier,
        coverage,
        nodes,
        edges,
    } = building.candidate;
    let (nodes, edges) = graph_inputs(nodes, edges)?;
    let graph = PackedGraph::build(nodes, edges, set_policy)
        .map_err(|_| SourceIndexLifecycleError::InvalidGraph)?;
    let edge_by_reference = (0..graph.edge_count())
        .map(|index| {
            let handle = graph
                .edge_at(index as u32)
                .expect("dense index came from this graph's edge count");
            let identity = graph
                .edge_identity(handle)
                .expect("handle came from this graph")
                .0
                .clone();
            (identity, handle)
        })
        .collect();
    let ready = Arc::new(ReadySourceIndex {
        scope,
        scope_digest: building.scope_digest,
        coordinate: coordinate.clone(),
        coordinate_digest: building.coordinate_digest.clone(),
        frontier,
        coverage,
        graph,
        edge_by_reference,
    });
    let mut registry = registry
        .write()
        .map_err(|_| SourceIndexLifecycleError::Poisoned)?;
    if registry.generations.len() >= MAX_SOURCE_INDEX_GENERATIONS {
        return Err(SourceIndexLifecycleError::GenerationCapacity);
    }
    match registry
        .generations
        .entry(building.coordinate_digest.clone())
    {
        Entry::Vacant(entry) => {
            entry.insert(SourceIndexEntry::Retained {
                index: ready,
                state: SourceIndexLifecycleState::Ready,
                selection_evidence: None,
                terminal_evidence: None,
            });
        }
        Entry::Occupied(_) => return Err(SourceIndexLifecycleError::GenerationCollision),
    }
    Ok(ReadySourceIndexCoordinate {
        coordinate_digest: building.coordinate_digest,
        coordinate,
    })
}

fn apply_transition(
    registry: &Arc<RwLock<SourceIndexRegistry>>,
    transition: SourceIndexTransition,
) -> Result<(), SourceIndexLifecycleError> {
    let mut registry = registry
        .write()
        .map_err(|_| SourceIndexLifecycleError::Poisoned)?;
    match transition {
        SourceIndexTransition::Activate {
            scope,
            coordinate_digest,
            evidence,
        } => activate_ready(&mut registry, scope, coordinate_digest, false, evidence),
        SourceIndexTransition::Supersede {
            scope,
            predecessor,
            successor,
            evidence,
        } => supersede(&mut registry, scope, predecessor, successor, evidence),
        SourceIndexTransition::Rollback {
            scope,
            coordinate_digest,
            evidence,
        } => activate_ready(&mut registry, scope, coordinate_digest, true, evidence),
        SourceIndexTransition::Invalidate {
            coordinate_digest,
            evidence,
        } => terminal_transition(
            &mut registry,
            coordinate_digest,
            SourceIndexLifecycleState::Invalidated,
            evidence,
        ),
        SourceIndexTransition::Retire {
            coordinate_digest,
            evidence,
        } => terminal_transition(
            &mut registry,
            coordinate_digest,
            SourceIndexLifecycleState::Retired,
            evidence,
        ),
        SourceIndexTransition::Remove {
            coordinate_digest,
            evidence,
        } => remove_generation(&mut registry, coordinate_digest, evidence),
    }
}

fn activate_ready(
    registry: &mut SourceIndexRegistry,
    scope: SourceIndexScope,
    coordinate_digest: ArtifactDigest,
    rollback: bool,
    evidence: ImmutableRecordReferenceV1,
) -> Result<(), SourceIndexLifecycleError> {
    let scope_digest = scope.lookup_digest()?;
    let SourceIndexEntry::Retained { index, state, .. } = registry
        .generations
        .get(&coordinate_digest)
        .ok_or(SourceIndexLifecycleError::UnknownGeneration)?
    else {
        return Err(SourceIndexLifecycleError::RemovedGeneration);
    };
    if index.scope != scope || index.scope_digest != scope_digest {
        return Err(SourceIndexLifecycleError::ScopeMismatch);
    }
    let displaced = registry.active_by_scope.get(&scope_digest).cloned();
    match (*state, rollback) {
        (SourceIndexLifecycleState::Ready, false)
            if displaced
                .as_ref()
                .is_none_or(|active| active == &coordinate_digest) => {}
        (SourceIndexLifecycleState::Superseded, true)
            if displaced
                .as_ref()
                .is_some_and(|active| active != &coordinate_digest) => {}
        _ => return Err(SourceIndexLifecycleError::InvalidTransition),
    }
    if rollback {
        let displaced = displaced.expect("rollback admission requires an active successor");
        let SourceIndexEntry::Retained {
            index,
            state,
            terminal_evidence,
            ..
        } = registry
            .generations
            .get_mut(&displaced)
            .expect("active generation is retained")
        else {
            return Err(SourceIndexLifecycleError::InvalidTransition);
        };
        if index.scope != scope
            || index.scope_digest != scope_digest
            || *state != SourceIndexLifecycleState::Ready
        {
            return Err(SourceIndexLifecycleError::InvalidTransition);
        }
        *state = SourceIndexLifecycleState::Superseded;
        *terminal_evidence = Some(evidence.clone());
        let SourceIndexEntry::Retained {
            state,
            selection_evidence,
            ..
        } = registry
            .generations
            .get_mut(&coordinate_digest)
            .expect("rollback target was checked above")
        else {
            unreachable!("rollback target was retained above")
        };
        *state = SourceIndexLifecycleState::Ready;
        *selection_evidence = Some(evidence);
    } else {
        let SourceIndexEntry::Retained {
            selection_evidence, ..
        } = registry
            .generations
            .get_mut(&coordinate_digest)
            .expect("activation target was checked above")
        else {
            unreachable!("activation target was retained above")
        };
        *selection_evidence = Some(evidence);
    }
    registry
        .active_by_scope
        .insert(scope_digest, coordinate_digest);
    Ok(())
}

fn supersede(
    registry: &mut SourceIndexRegistry,
    scope: SourceIndexScope,
    predecessor: ArtifactDigest,
    successor: ArtifactDigest,
    evidence: ImmutableRecordReferenceV1,
) -> Result<(), SourceIndexLifecycleError> {
    if predecessor == successor {
        return Err(SourceIndexLifecycleError::InvalidTransition);
    }
    let scope_digest = scope.lookup_digest()?;
    if registry.active_by_scope.get(&scope_digest) != Some(&predecessor) {
        return Err(SourceIndexLifecycleError::NotActiveGeneration);
    }
    for coordinate in [&predecessor, &successor] {
        let SourceIndexEntry::Retained { index, state, .. } = registry
            .generations
            .get(coordinate)
            .ok_or(SourceIndexLifecycleError::UnknownGeneration)?
        else {
            return Err(SourceIndexLifecycleError::RemovedGeneration);
        };
        if index.scope != scope || *state != SourceIndexLifecycleState::Ready {
            return Err(SourceIndexLifecycleError::InvalidTransition);
        }
    }
    let SourceIndexEntry::Retained {
        state,
        terminal_evidence,
        ..
    } = registry
        .generations
        .get_mut(&predecessor)
        .expect("predecessor was checked above")
    else {
        unreachable!("predecessor was retained above")
    };
    *state = SourceIndexLifecycleState::Superseded;
    *terminal_evidence = Some(evidence.clone());
    let SourceIndexEntry::Retained {
        selection_evidence, ..
    } = registry
        .generations
        .get_mut(&successor)
        .expect("successor was checked above")
    else {
        unreachable!("successor was retained above")
    };
    *selection_evidence = Some(evidence);
    registry.active_by_scope.insert(scope_digest, successor);
    Ok(())
}

fn terminal_transition(
    registry: &mut SourceIndexRegistry,
    coordinate_digest: ArtifactDigest,
    next: SourceIndexLifecycleState,
    evidence: ImmutableRecordReferenceV1,
) -> Result<(), SourceIndexLifecycleError> {
    let SourceIndexEntry::Retained {
        index,
        state,
        terminal_evidence,
        ..
    } = registry
        .generations
        .get_mut(&coordinate_digest)
        .ok_or(SourceIndexLifecycleError::UnknownGeneration)?
    else {
        return Err(SourceIndexLifecycleError::RemovedGeneration);
    };
    if !matches!(
        *state,
        SourceIndexLifecycleState::Ready | SourceIndexLifecycleState::Superseded
    ) {
        return Err(SourceIndexLifecycleError::InvalidTransition);
    }
    *state = next;
    *terminal_evidence = Some(evidence);
    if registry.active_by_scope.get(&index.scope_digest) == Some(&coordinate_digest) {
        registry.active_by_scope.remove(&index.scope_digest);
    }
    Ok(())
}

fn remove_generation(
    registry: &mut SourceIndexRegistry,
    coordinate_digest: ArtifactDigest,
    evidence: ImmutableRecordReferenceV1,
) -> Result<(), SourceIndexLifecycleError> {
    let entry = registry
        .generations
        .get(&coordinate_digest)
        .ok_or(SourceIndexLifecycleError::UnknownGeneration)?;
    let SourceIndexEntry::Retained {
        index,
        state,
        terminal_evidence,
        ..
    } = entry
    else {
        return Err(SourceIndexLifecycleError::RemovedGeneration);
    };
    if !matches!(
        state,
        SourceIndexLifecycleState::Invalidated | SourceIndexLifecycleState::Retired
    ) {
        return Err(SourceIndexLifecycleError::InvalidTransition);
    }
    let coordinate = index.coordinate.clone();
    let scope_digest = index.scope_digest.clone();
    let prior_terminal_evidence = terminal_evidence.clone();
    registry.active_by_scope.remove(&scope_digest);
    registry.generations.insert(
        coordinate_digest,
        SourceIndexEntry::Removed {
            _coordinate: Box::new(coordinate),
            _scope_digest: scope_digest,
            _prior_terminal_evidence: prior_terminal_evidence,
            _removal_evidence: evidence,
        },
    );
    Ok(())
}

pub(super) fn coordinate_digest(
    coordinate: &ProviderIndexCoordinateV1,
) -> Result<ArtifactDigest, CanonError> {
    Ok(ArtifactDigest::hash_bytes(
        to_canonical_string(coordinate)?.as_bytes(),
    ))
}

/// Source-index lifecycle admission or transition failure.
#[derive(Debug, thiserror::Error)]
pub enum SourceIndexLifecycleError {
    /// Desired repository and source-frontier repository differ.
    #[error("source-index scope repository differs from its source frontier")]
    RepositoryMismatch,
    /// Scope provider and index owner differ.
    #[error("source-index scope provider differs from its index coordinate")]
    ProviderMismatch,
    /// Candidate coordinate and admitted exact frontier differ.
    #[error("source-index coordinate differs from its admitted exact frontier")]
    FrontierMismatch,
    /// The same immutable generation coordinate is already retained.
    #[error("source-index generation coordinate already exists")]
    GenerationCollision,
    /// Registry reached its finite retained-generation bound.
    #[error("source-index registry contains the maximum 1024 generations")]
    GenerationCapacity,
    /// Graph declarations are malformed or outside finite bounds.
    #[error(transparent)]
    GraphDeclaration(#[from] SourceGraphDeclarationError),
    /// Packed graph rejected duplicates, missing endpoints, or handle exhaustion.
    #[error("source-index graph is not a valid canonical packed generation")]
    InvalidGraph,
    /// Named generation does not exist.
    #[error("source-index generation is unknown")]
    UnknownGeneration,
    /// Named generation's derived bytes have already been removed.
    #[error("source-index generation has been removed")]
    RemovedGeneration,
    /// Supplied scope differs from the retained generation scope.
    #[error("source-index scope does not match the retained generation")]
    ScopeMismatch,
    /// Expected predecessor is not the active generation.
    #[error("source-index predecessor is not active for the supplied scope")]
    NotActiveGeneration,
    /// Requested lifecycle edge is not admitted from the current state.
    #[error("source-index lifecycle transition is invalid from the current state")]
    InvalidTransition,
    /// Canonical digest construction failed.
    #[error("cannot canonicalize source-index coordinate: {0}")]
    Canonicalize(#[from] CanonError),
    /// Shared lifecycle custody was poisoned by a panicking writer.
    #[error("source-index lifecycle custody is poisoned")]
    Poisoned,
}
