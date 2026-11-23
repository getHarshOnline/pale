//! Actual source-evidence use and append-only invalidation propagation.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::{
    canon::{to_canonical_string, CanonError},
    records::{
        ArtifactDigest, CompiledGenerationDigest, ImmutableRecordReferenceV1, OperationLeaseIdV1,
        PackageResourceRefV1, ProviderIndexCoordinateV1, UnixTimeV1,
    },
};
use pale_kernel::{AdaptiveNodeSetPolicy, GraphEdge, GraphNode, PackedGraph};

use super::{coordinate_digest, AdmittedSourceQuery, SourceIndexScope, SourceQueryResult};

/// Maximum lineage nodes, edges, or invalidation roots in one bounded graph.
pub const MAX_SOURCE_LINEAGE_MEMBERS: usize = 65_536;
/// Maximum roots accepted by one invalidation event.
pub const MAX_SOURCE_INVALIDATION_ROOTS: usize = 64;

/// Exact owner/context references claimed by one evidence use.
///
/// These body-free coordinates are joined structurally; this type does not
/// authenticate source grants or establish native-owner authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceUseContext {
    /// Exact admitted workspace record.
    pub workspace: ImmutableRecordReferenceV1,
    /// Exact module release selected in that workspace.
    pub module_release: ImmutableRecordReferenceV1,
    /// Exact import/adoption record.
    pub import: ImmutableRecordReferenceV1,
    /// Exact source-evidence surface grant.
    pub surface_grant: ImmutableRecordReferenceV1,
}

/// Candidate immutable record describing actual source-evidence use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceEvidenceUse {
    /// Immutable actual-use record identity.
    pub use_record: ImmutableRecordReferenceV1,
    /// Immutable result record whose content digest equals `result_digest`.
    pub result_record: ImmutableRecordReferenceV1,
    /// Exact result digest.
    pub result_digest: ArtifactDigest,
    /// Exact provider/index/source-frontier coordinate.
    pub index: ProviderIndexCoordinateV1,
    /// Complete scope isolation coordinate.
    pub scope: SourceIndexScope,
    /// Workspace/module/import/grant context.
    pub context: SourceUseContext,
    /// Consuming compiled generation.
    pub compiled_generation: CompiledGenerationDigest,
    /// Consuming operation.
    pub operation: PackageResourceRefV1,
    /// Consuming operation lease.
    pub operation_lease: OperationLeaseIdV1,
    /// Exact invocation record.
    pub invocation: ImmutableRecordReferenceV1,
    /// Evidence portions that were actually used.
    pub used: BTreeSet<ImmutableRecordReferenceV1>,
    /// Evidence portions explicitly rejected.
    pub rejected: BTreeSet<ImmutableRecordReferenceV1>,
    /// Evidence portions deliberately ignored.
    pub ignored: BTreeSet<ImmutableRecordReferenceV1>,
    /// Exact use purpose.
    pub purpose: ImmutableRecordReferenceV1,
    /// Exact evaluated target.
    pub target: ImmutableRecordReferenceV1,
    /// Generated artifact or closure that depends on this use.
    pub dependent: Option<ImmutableRecordReferenceV1>,
    /// Typed settlement record, when separately produced.
    pub settlement: Option<ImmutableRecordReferenceV1>,
    /// Owner-native verification record, when separately produced.
    pub native_verification: Option<ImmutableRecordReferenceV1>,
    /// Occurrence time.
    pub occurred_at: UnixTimeV1,
    /// Evidence-use expiry.
    pub expires_at: UnixTimeV1,
}

/// Source-evidence use structurally joined to its exact request and result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedSourceEvidenceUse(SourceEvidenceUse);

impl AdmittedSourceEvidenceUse {
    /// Borrow the exact admitted use.
    #[must_use]
    pub fn record(&self) -> &SourceEvidenceUse {
        &self.0
    }
}

/// Validate actual-use structure without treating query completion as use.
///
/// The used/rejected/ignored sets must form an exact disjoint partition of the
/// evidence returned by `result`, and `used` must be non-empty. Later evidence,
/// activation, or settlement cannot alter this immutable historical partition.
/// Success proves those local joins only; source-grant authentication and
/// owner-native query authorization require a separately admitted contract.
///
/// # Errors
///
/// Returns [`SourceEvidenceUseError`] when any request/result/use coordinate
/// differs, the evidence partition is not exact, or the occurrence interval is
/// invalid.
pub fn admit_source_evidence_use(
    query: &AdmittedSourceQuery,
    result: &SourceQueryResult,
    candidate: SourceEvidenceUse,
) -> Result<AdmittedSourceEvidenceUse, SourceEvidenceUseError> {
    let request = query.request();
    if !result
        .has_valid_integrity()
        .map_err(SourceEvidenceUseError::Canonicalize)?
        || result.request_digest != *query.request_digest()
        || candidate.result_digest != result.result_digest
        || candidate.result_record.record_sha256 != result.result_digest
    {
        return Err(SourceEvidenceUseError::ResultMismatch);
    }
    if candidate.index != request.index
        || candidate.index != result.index
        || candidate.scope != request.scope
        || candidate.scope != result.scope
    {
        return Err(SourceEvidenceUseError::IndexOrScopeMismatch);
    }
    if candidate.compiled_generation != request.compiled_generation
        || candidate.operation != request.reason.operation
        || candidate.operation_lease != request.operation_lease
    {
        return Err(SourceEvidenceUseError::ConsumerMismatch);
    }
    if candidate.occurred_at.get() >= candidate.expires_at.get() {
        return Err(SourceEvidenceUseError::InvalidInterval);
    }
    let available = result.evidence_references();
    let partition = candidate
        .used
        .iter()
        .chain(&candidate.rejected)
        .chain(&candidate.ignored)
        .cloned()
        .collect::<BTreeSet<_>>();
    if candidate.used.is_empty()
        || candidate.used.len() + candidate.rejected.len() + candidate.ignored.len()
            != available.len()
        || !candidate.used.is_disjoint(&candidate.rejected)
        || !candidate.used.is_disjoint(&candidate.ignored)
        || !candidate.rejected.is_disjoint(&candidate.ignored)
        || partition != available
    {
        return Err(SourceEvidenceUseError::InvalidEvidencePartition);
    }
    Ok(AdmittedSourceEvidenceUse(candidate))
}

/// Immutable result identity admitted against its exact in-memory evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedSourceResultRecord {
    record: ImmutableRecordReferenceV1,
    result_digest: ArtifactDigest,
    scope_digest: ArtifactDigest,
    index_digest: ArtifactDigest,
    frontier_digest: ArtifactDigest,
}

impl AdmittedSourceResultRecord {
    /// Borrow the immutable result reference.
    #[must_use]
    pub fn record(&self) -> &ImmutableRecordReferenceV1 {
        &self.record
    }

    /// Digest of the exact result payload bound by [`Self::record`].
    #[must_use]
    pub fn result_digest(&self) -> &ArtifactDigest {
        &self.result_digest
    }
}

/// Bind a durable result reference to the exact result bytes and coordinates.
///
/// # Errors
///
/// Returns [`SourceEvidenceUseError::ResultMismatch`] when the immutable record
/// digest differs from the result digest.
pub fn admit_source_result_record(
    result: &SourceQueryResult,
    record: ImmutableRecordReferenceV1,
) -> Result<AdmittedSourceResultRecord, SourceEvidenceUseError> {
    if !result
        .has_valid_integrity()
        .map_err(SourceEvidenceUseError::Canonicalize)?
        || record.record_sha256 != result.result_digest
    {
        return Err(SourceEvidenceUseError::ResultMismatch);
    }
    Ok(AdmittedSourceResultRecord {
        record,
        result_digest: result.result_digest.clone(),
        scope_digest: result
            .scope
            .lookup_digest()
            .map_err(SourceEvidenceUseError::Canonicalize)?,
        index_digest: coordinate_digest(&result.index)
            .map_err(SourceEvidenceUseError::Canonicalize)?,
        frontier_digest: ArtifactDigest::hash_bytes(
            to_canonical_string(&result.index.repository_source_frontier)
                .map_err(SourceEvidenceUseError::Canonicalize)?
                .as_bytes(),
        ),
    })
}

/// Typed source-evidence lineage node.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SourceLineageNode {
    /// Exact repository source frontier.
    Frontier(ArtifactDigest),
    /// Exact provider index generation.
    Index(ArtifactDigest),
    /// Exact returned evidence portion.
    Evidence(ImmutableRecordReferenceV1),
    /// Immutable query result record.
    Result(ImmutableRecordReferenceV1),
    /// Immutable actual-use record.
    Use(ImmutableRecordReferenceV1),
    /// Generated artifact or closure dependent on actual use.
    Dependent(ImmutableRecordReferenceV1),
    /// Separately produced settlement.
    Settlement(ImmutableRecordReferenceV1),
    /// Separately produced owner-native verification.
    NativeVerification(ImmutableRecordReferenceV1),
}

/// Closed causal relationship in source-evidence lineage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceLineageRelation {
    /// Result was produced from the exact frontier.
    FrontierResult,
    /// Result was produced from the exact provider index generation.
    IndexResult,
    /// Actual use consumed this exact result record.
    ResultUse,
    /// Actual use consumed this exact evidence portion.
    EvidenceUse,
    /// Generated dependent consumed the actual use.
    UseDependent,
    /// Settlement classified consequences of the actual use.
    UseSettlement,
    /// Owner-native verification observed consequences of the actual use.
    UseNativeVerification,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SourceLineageEdgeId {
    cause: SourceLineageNode,
    dependent: SourceLineageNode,
}

type SourceLineagePackedGraph = PackedGraph<
    SourceLineageNode,
    (),
    SourceLineageEdgeId,
    SourceLineageRelation,
    ImmutableRecordReferenceV1,
>;

/// Builder that retains one scope's body-free lineage until an immutable seal.
///
/// The first admitted result fixes the complete [`SourceIndexScope`]. Later
/// results and uses must reproduce its exact digest; sharing an index/frontier
/// coordinate cannot merge evidence across principals, deployments, worktrees,
/// or provider pins.
pub struct SourceEvidenceLineageBuilder {
    scope_digest: Option<ArtifactDigest>,
    nodes: BTreeSet<SourceLineageNode>,
    edges: BTreeSet<SourceLineageEdgeId>,
    declarations: Vec<
        GraphEdge<
            SourceLineageEdgeId,
            SourceLineageNode,
            SourceLineageRelation,
            ImmutableRecordReferenceV1,
        >,
    >,
    set_policy: AdaptiveNodeSetPolicy,
}

impl SourceEvidenceLineageBuilder {
    /// Create an empty bounded lineage builder using the shared set policy.
    #[must_use]
    pub fn new(set_policy: AdaptiveNodeSetPolicy) -> Self {
        Self {
            scope_digest: None,
            nodes: BTreeSet::new(),
            edges: BTreeSet::new(),
            declarations: Vec::new(),
            set_policy,
        }
    }

    /// Add one exact result and its separate frontier/index causes.
    ///
    /// # Errors
    ///
    /// Returns [`SourceLineageError`] at a finite bound or duplicate result.
    pub fn add_result(
        &mut self,
        result: &AdmittedSourceResultRecord,
        evidence: ImmutableRecordReferenceV1,
    ) -> Result<(), SourceLineageError> {
        self.admit_scope(&result.scope_digest)?;
        let result_node = SourceLineageNode::Result(result.record.clone());
        if self.nodes.contains(&result_node) {
            return Err(SourceLineageError::DuplicateNode);
        }
        self.add_edges(vec![
            lineage_edge(
                SourceLineageNode::Frontier(result.frontier_digest.clone()),
                result_node.clone(),
                SourceLineageRelation::FrontierResult,
                evidence.clone(),
            ),
            lineage_edge(
                SourceLineageNode::Index(result.index_digest.clone()),
                result_node,
                SourceLineageRelation::IndexResult,
                evidence,
            ),
        ])
    }

    /// Add actual-use lineage and only the portions it truly consumed.
    ///
    /// # Errors
    ///
    /// Returns [`SourceLineageError`] at a finite bound or duplicate node/edge.
    pub fn add_use(
        &mut self,
        source_use: &AdmittedSourceEvidenceUse,
        evidence: ImmutableRecordReferenceV1,
    ) -> Result<(), SourceLineageError> {
        let record = source_use.record();
        let result = SourceLineageNode::Result(record.result_record.clone());
        if !self.nodes.contains(&result) {
            return Err(SourceLineageError::MissingResult);
        }
        let scope_digest = record
            .scope
            .lookup_digest()
            .map_err(SourceLineageError::Canonicalize)?;
        self.admit_scope(&scope_digest)?;
        let use_node = SourceLineageNode::Use(record.use_record.clone());
        if self.nodes.contains(&use_node) {
            return Err(SourceLineageError::DuplicateNode);
        }
        let mut declarations = vec![lineage_edge(
            result,
            use_node.clone(),
            SourceLineageRelation::ResultUse,
            evidence.clone(),
        )];
        for portion in &record.used {
            declarations.push(lineage_edge(
                SourceLineageNode::Evidence(portion.clone()),
                use_node.clone(),
                SourceLineageRelation::EvidenceUse,
                evidence.clone(),
            ));
        }
        for (dependent, relation) in [
            (
                record.dependent.clone().map(SourceLineageNode::Dependent),
                SourceLineageRelation::UseDependent,
            ),
            (
                record.settlement.clone().map(SourceLineageNode::Settlement),
                SourceLineageRelation::UseSettlement,
            ),
            (
                record
                    .native_verification
                    .clone()
                    .map(SourceLineageNode::NativeVerification),
                SourceLineageRelation::UseNativeVerification,
            ),
        ] {
            if let Some(dependent) = dependent {
                declarations.push(lineage_edge(
                    use_node.clone(),
                    dependent,
                    relation,
                    evidence.clone(),
                ));
            }
        }
        self.add_edges(declarations)
    }

    fn admit_scope(&mut self, candidate: &ArtifactDigest) -> Result<(), SourceLineageError> {
        match &self.scope_digest {
            Some(current) if current != candidate => Err(SourceLineageError::ScopeMismatch),
            Some(_) => Ok(()),
            None => {
                self.scope_digest = Some(candidate.clone());
                Ok(())
            }
        }
    }

    fn add_edges(
        &mut self,
        declarations: Vec<
            GraphEdge<
                SourceLineageEdgeId,
                SourceLineageNode,
                SourceLineageRelation,
                ImmutableRecordReferenceV1,
            >,
        >,
    ) -> Result<(), SourceLineageError> {
        let mut additional_nodes = BTreeSet::new();
        let mut additional_edges = BTreeSet::new();
        for declaration in &declarations {
            if declaration.source == declaration.target {
                return Err(SourceLineageError::SelfDependency);
            }
            if !self.nodes.contains(&declaration.source) {
                additional_nodes.insert(declaration.source.clone());
            }
            if !self.nodes.contains(&declaration.target) {
                additional_nodes.insert(declaration.target.clone());
            }
            if self.edges.contains(&declaration.id)
                || !additional_edges.insert(declaration.id.clone())
            {
                return Err(SourceLineageError::DuplicateEdge);
            }
        }
        if self
            .nodes
            .len()
            .checked_add(additional_nodes.len())
            .is_none_or(|count| count > MAX_SOURCE_LINEAGE_MEMBERS)
            || self
                .declarations
                .len()
                .checked_add(declarations.len())
                .is_none_or(|count| count > MAX_SOURCE_LINEAGE_MEMBERS)
        {
            return Err(SourceLineageError::CapacityExceeded);
        }
        self.nodes.extend(additional_nodes);
        self.edges.extend(additional_edges);
        self.declarations.extend(declarations);
        Ok(())
    }

    /// Seal one immutable packed lineage graph.
    ///
    /// # Errors
    ///
    /// Returns [`SourceLineageError`] when the declaration exceeds bounds or
    /// cannot form one canonical graph generation.
    pub fn seal(self) -> Result<SourceEvidenceLineage, SourceLineageError> {
        if self.nodes.len() > MAX_SOURCE_LINEAGE_MEMBERS
            || self.declarations.len() > MAX_SOURCE_LINEAGE_MEMBERS
        {
            return Err(SourceLineageError::CapacityExceeded);
        }
        let nodes = self
            .nodes
            .into_iter()
            .map(|key| GraphNode { key, payload: () })
            .collect();
        let graph = PackedGraph::build(nodes, self.declarations, self.set_policy)
            .map_err(|_| SourceLineageError::InvalidGraph)?;
        Ok(SourceEvidenceLineage {
            scope_digest: self.scope_digest,
            graph,
        })
    }
}

fn lineage_edge(
    cause: SourceLineageNode,
    dependent: SourceLineageNode,
    relation: SourceLineageRelation,
    evidence: ImmutableRecordReferenceV1,
) -> GraphEdge<
    SourceLineageEdgeId,
    SourceLineageNode,
    SourceLineageRelation,
    ImmutableRecordReferenceV1,
> {
    GraphEdge {
        id: SourceLineageEdgeId {
            cause: cause.clone(),
            dependent: dependent.clone(),
        },
        source: cause,
        target: dependent,
        payload: relation,
        reason: evidence,
    }
}

/// Immutable actual-use lineage over which invalidation is projected.
pub struct SourceEvidenceLineage {
    scope_digest: Option<ArtifactDigest>,
    graph: SourceLineagePackedGraph,
}

/// One append-only invalidation observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceEvidenceInvalidation {
    /// Complete isolation coordinate whose lineage is being invalidated.
    pub scope: SourceIndexScope,
    /// Exact invalidated frontier, index, result, or evidence roots.
    pub roots: BTreeSet<SourceLineageNode>,
    /// Immutable invalidation evidence.
    pub evidence: ImmutableRecordReferenceV1,
}

/// Bounded invalidation closure without rewriting historical records.
///
/// The closure returns the exact immutable evidence supplied by the caller; it
/// does not append that evidence to a history or claim custody of such a log.
/// In the law below, `R` is the selected root set, `r` is one selected root,
/// and `v` is a known lineage node. `r` leading to `v` means that the retained
/// directed lineage contains a dependency path from `r` to `v`. The affected
/// set contains the roots themselves and only their reachable dependents.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/source-evidence-invalidation.display.mathml"
))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceInvalidationClosure {
    /// Every known affected record in canonical order.
    pub affected: BTreeSet<SourceLineageNode>,
    /// Exact root-to-dependent reason paths.
    pub reason_paths: Vec<Vec<SourceLineageNode>>,
    /// Immutable invalidation evidence appended to the closure.
    pub evidence: ImmutableRecordReferenceV1,
}

impl SourceEvidenceLineage {
    /// Propagate invalidation through exact result/use/dependent relationships.
    ///
    /// The historical graph is borrowed immutably. Unknown roots are rejected;
    /// no unrelated record is marked stale merely because a label resembles a
    /// known coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`SourceLineageError`] for an empty/oversized/unknown root set or
    /// an impossible packed-graph access error.
    pub fn invalidate(
        &self,
        invalidation: SourceEvidenceInvalidation,
    ) -> Result<SourceInvalidationClosure, SourceLineageError> {
        let invalidation_scope = invalidation
            .scope
            .lookup_digest()
            .map_err(SourceLineageError::Canonicalize)?;
        if self.scope_digest.as_ref() != Some(&invalidation_scope) {
            return Err(SourceLineageError::ScopeMismatch);
        }
        if invalidation.roots.is_empty() || invalidation.roots.len() > MAX_SOURCE_INVALIDATION_ROOTS
        {
            return Err(SourceLineageError::InvalidRootCount);
        }
        let mut affected = BTreeSet::new();
        // Retain one globally shortest canonical root witness per affected
        // node. This bounds proof output by the graph's own node ceiling even
        // when all 64 invalidation roots reach the same large closure.
        let mut path_by_dependent = BTreeMap::<SourceLineageNode, Vec<SourceLineageNode>>::new();
        for root in &invalidation.roots {
            let origin = self
                .graph
                .find(root)
                .ok_or(SourceLineageError::UnknownRoot)?;
            let proof = self.graph.reachability(origin)?;
            for handle in proof.reached() {
                let node = self.graph.node_key(handle)?.clone();
                affected.insert(node.clone());
                if let Some(path) = proof.path_to(handle)? {
                    let candidate = path
                        .nodes
                        .into_iter()
                        .map(|member| self.graph.node_key(member).cloned())
                        .collect::<Result<Vec<_>, _>>()?;
                    path_by_dependent
                        .entry(node)
                        .and_modify(|selected| {
                            if (candidate.len(), &candidate) < (selected.len(), &*selected) {
                                *selected = candidate.clone();
                            }
                        })
                        .or_insert(candidate);
                }
            }
        }
        let mut reason_paths = path_by_dependent.into_values().collect::<Vec<_>>();
        reason_paths.sort();
        Ok(SourceInvalidationClosure {
            affected,
            reason_paths,
            evidence: invalidation.evidence,
        })
    }
}

/// Invalid actual-use record.
#[derive(Debug, thiserror::Error)]
pub enum SourceEvidenceUseError {
    /// Result record/digest/request relationship differs.
    #[error("source-evidence use does not name the exact admitted result")]
    ResultMismatch,
    /// Source scope or provider/index/frontier differs.
    #[error("source-evidence use index or scope differs from the admitted query")]
    IndexOrScopeMismatch,
    /// Consuming generation, operation, or lease differs.
    #[error("source-evidence use consumer differs from the admitted query")]
    ConsumerMismatch,
    /// Occurrence must precede expiry.
    #[error("source-evidence use occurrence does not precede expiry")]
    InvalidInterval,
    /// Used/rejected/ignored are not the exact disjoint result partition.
    #[error("source-evidence portions do not form an exact disjoint partition")]
    InvalidEvidencePartition,
    /// Canonical coordinate projection failed.
    #[error("cannot canonicalize source-evidence coordinate: {0}")]
    Canonicalize(#[source] CanonError),
}

/// Invalid source-evidence lineage declaration or query.
#[derive(Debug, thiserror::Error)]
pub enum SourceLineageError {
    /// Bounded graph member ceiling exceeded.
    #[error("source-evidence lineage exceeds 65536 nodes or edges")]
    CapacityExceeded,
    /// A result or use was admitted more than once.
    #[error("source-evidence lineage contains a duplicate node")]
    DuplicateNode,
    /// A cause/dependent pair was admitted more than once.
    #[error("source-evidence lineage contains a duplicate edge")]
    DuplicateEdge,
    /// Cause and dependent are the same record.
    #[error("source-evidence lineage contains a self dependency")]
    SelfDependency,
    /// Actual use names a result absent from the builder.
    #[error("source-evidence use names a result absent from lineage")]
    MissingResult,
    /// A result or use belongs to another complete source-index scope.
    #[error("source-evidence lineage cannot cross source-index scopes")]
    ScopeMismatch,
    /// Canonical scope projection failed.
    #[error("cannot canonicalize source-index scope: {0}")]
    Canonicalize(#[source] CanonError),
    /// Packed graph construction rejected the declaration.
    #[error("source-evidence lineage cannot form a canonical packed graph")]
    InvalidGraph,
    /// Invalidation root count is zero or exceeds the exact bound.
    #[error("source-evidence invalidation root count is outside 1..=64")]
    InvalidRootCount,
    /// Invalidation names a root absent from known lineage.
    #[error("source-evidence invalidation names an unknown root")]
    UnknownRoot,
    /// Packed graph rejected an internal handle.
    #[error("source-evidence lineage access failed: {0}")]
    Graph(#[from] pale_kernel::GraphAccessError),
}
