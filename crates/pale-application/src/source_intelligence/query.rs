//! Operation-bound source-intelligence query admission and projection.

use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

use pale_core::{
    canon::{to_canonical_string, CanonError},
    records::{
        ArtifactDigest, CapabilityRequirementIdV1, CompiledGenerationDigest, EffectClass,
        ImmutableRecordReferenceV1, OperationLeaseIdV1, PackageResourceRefV1,
        ProviderIndexCoordinateV1,
    },
};
use pale_kernel::{Direction, EdgeHandle, ReasonPath};
use pale_ports::SourceIntelligenceQueryPort;
use serde::Serialize;

use crate::semantic::{OperationObligation, OperationObligationState};

use super::{
    coordinate_digest, SourceCoverageDimension, SourceCoverageGapKind, SourceIndexEntry,
    SourceIndexLifecycleState, SourceIndexRegistry, SourceIndexScope, SourceNodeKind,
};

/// Maximum anchors or obligation-path records on one query.
pub const MAX_SOURCE_QUERY_ANCHORS: usize = 64;
/// Maximum returned members of any one query collection.
pub const MAX_SOURCE_QUERY_MEMBERS: usize = 4_096;
/// Maximum edges in one returned reason path.
pub const MAX_SOURCE_REASON_PATH_EDGES: usize = 256;
/// Maximum canonical evidence payload admitted from one query.
pub const MAX_SOURCE_QUERY_OUTPUT_BYTES: u64 = 4 * 1024 * 1024;

/// Exact Package obligation selecting one structural question.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceQueryReason {
    /// Immutable reference to the compiled obligation evidence.
    pub obligation: ImmutableRecordReferenceV1,
    /// Operation selected by that obligation.
    pub operation: PackageResourceRefV1,
    /// Exact operation-local capability requirement.
    pub requirement: CapabilityRequirementIdV1,
    /// Maximum effect carried by the requirement.
    pub effect: EffectClass,
    /// Ordered body-free causal path from obligation to structural question.
    pub causal_path: Vec<ImmutableRecordReferenceV1>,
}

/// One bounded structural question.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SourceStructuralQuestion {
    /// Explore downstream dependents and, optionally, exact target paths.
    Explore {
        /// Canonically ordered starting evidence references.
        origins: Vec<ImmutableRecordReferenceV1>,
        /// Optional exact target for reason-path construction.
        target: Option<ImmutableRecordReferenceV1>,
    },
    /// Locate repository-native tests affected by changed structural subjects.
    CandidateAffectedTests {
        /// Canonically ordered changed subjects.
        changed: Vec<ImmutableRecordReferenceV1>,
    },
}

impl SourceStructuralQuestion {
    fn anchors(&self) -> &[ImmutableRecordReferenceV1] {
        match self {
            Self::Explore { origins, .. } => origins,
            Self::CandidateAffectedTests { changed } => changed,
        }
    }
}

/// Hard query/output ceilings.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SourceQueryBounds {
    /// Maximum returned graph nodes.
    pub nodes: usize,
    /// Maximum returned reason paths.
    pub reason_paths: usize,
    /// Maximum edges in each reason path.
    pub path_edges: usize,
    /// Maximum returned affected-test candidates.
    pub affected_tests: usize,
    /// Maximum canonical evidence payload bytes.
    pub output_bytes: u64,
}

/// Candidate source-intelligence request before contextual admission.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceQueryRequest {
    /// Exact provider/index/source-frontier coordinate.
    pub index: ProviderIndexCoordinateV1,
    /// Complete isolation coordinate.
    pub scope: SourceIndexScope,
    /// Consuming compiled generation.
    pub compiled_generation: CompiledGenerationDigest,
    /// Consuming operation lease.
    pub operation_lease: OperationLeaseIdV1,
    /// Compiled reason selecting this call.
    pub reason: SourceQueryReason,
    /// Structural question and anchors.
    pub question: SourceStructuralQuestion,
    /// Hard result ceilings.
    pub bounds: SourceQueryBounds,
}

/// Source request structurally matched to one supplied operation obligation.
///
/// This wrapper proves the local joins below, not that the caller obtained the
/// obligation from an authenticated compiler handle or holds native query
/// authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedSourceQuery {
    request: SourceQueryRequest,
    request_digest: ArtifactDigest,
}

impl AdmittedSourceQuery {
    /// Borrow the admitted request.
    #[must_use]
    pub fn request(&self) -> &SourceQueryRequest {
        &self.request
    }

    /// Exact internal digest of the admitted request.
    #[must_use]
    pub fn request_digest(&self) -> &ArtifactDigest {
        &self.request_digest
    }
}

/// Validate one query against a supplied mechanical operation record.
///
/// This function cannot create a structural obligation from provider
/// availability. It accepts only a bound capability requirement present in
/// [`OperationObligationState::CapabilityBearing`], while authentication of the
/// containing compiler handle remains a separate caller responsibility.
///
/// # Errors
///
/// Returns [`SourceQueryAdmissionError`] for unlike operation/requirement/effect
/// coordinates, an empty or noncanonical reason/anchor path, or an invalid bound.
pub fn admit_source_query(
    operation: &OperationObligation,
    request: SourceQueryRequest,
) -> Result<AdmittedSourceQuery, SourceQueryAdmissionError> {
    if operation.operation != request.reason.operation {
        return Err(SourceQueryAdmissionError::OperationMismatch);
    }
    let OperationObligationState::CapabilityBearing { capabilities, .. } = &operation.state else {
        return Err(SourceQueryAdmissionError::NoStructuralObligation);
    };
    if !capabilities.iter().any(|capability| {
        capability.subject.operation == request.reason.operation
            && capability.subject.requirement == request.reason.requirement
            && capability.effect == request.reason.effect
            && matches!(
                &capability.disposition,
                crate::semantic::CapabilityObligationDisposition::Bound(binding)
                    if binding.effect_class == request.reason.effect
            )
    }) {
        return Err(SourceQueryAdmissionError::NoStructuralObligation);
    }
    validate_bounded_unique_path(
        "causal path",
        &request.reason.causal_path,
        MAX_SOURCE_QUERY_ANCHORS,
    )?;
    if request.reason.causal_path.first() != Some(&request.reason.obligation) {
        return Err(SourceQueryAdmissionError::NonCanonicalBoundedRelation {
            relation: "causal path",
        });
    }
    validate_canonical_nonempty_set(
        "question anchors",
        request.question.anchors(),
        MAX_SOURCE_QUERY_ANCHORS,
    )?;
    if request.bounds.nodes == 0
        || request.bounds.nodes > MAX_SOURCE_QUERY_MEMBERS
        || request.bounds.reason_paths > MAX_SOURCE_QUERY_MEMBERS
        || request.bounds.path_edges == 0
        || request.bounds.path_edges > MAX_SOURCE_REASON_PATH_EDGES
        || request.bounds.affected_tests > MAX_SOURCE_QUERY_MEMBERS
        || request.bounds.output_bytes == 0
        || request.bounds.output_bytes > MAX_SOURCE_QUERY_OUTPUT_BYTES
    {
        return Err(SourceQueryAdmissionError::InvalidBounds);
    }
    let request_digest = ArtifactDigest::hash_bytes(
        to_canonical_string(&request)
            .map_err(SourceQueryAdmissionError::Canonicalize)?
            .as_bytes(),
    );
    Ok(AdmittedSourceQuery {
        request,
        request_digest,
    })
}

fn validate_bounded_unique_path(
    relation: &'static str,
    values: &[ImmutableRecordReferenceV1],
    maximum: usize,
) -> Result<(), SourceQueryAdmissionError> {
    let unique = values.iter().collect::<BTreeSet<_>>();
    if values.is_empty() || values.len() > maximum || unique.len() != values.len() {
        Err(SourceQueryAdmissionError::NonCanonicalBoundedRelation { relation })
    } else {
        Ok(())
    }
}

fn validate_canonical_nonempty_set(
    relation: &'static str,
    values: &[ImmutableRecordReferenceV1],
    maximum: usize,
) -> Result<(), SourceQueryAdmissionError> {
    if values.is_empty()
        || values.len() > maximum
        || !values.windows(2).all(|pair| pair[0] < pair[1])
    {
        Err(SourceQueryAdmissionError::NonCanonicalBoundedRelation { relation })
    } else {
        Ok(())
    }
}

/// Bounded causal path projected from one immutable index generation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SourceReasonPath {
    /// Exact nodes in path order.
    pub nodes: Vec<ImmutableRecordReferenceV1>,
    /// Exact provider relationship identities in path order.
    pub edges: Vec<ImmutableRecordReferenceV1>,
}

/// Independent limitation retained in a query result.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SourceResultLimitation {
    /// One declared coverage subject was not observed.
    CoverageGap {
        /// Independent coverage dimension.
        dimension: SourceCoverageDimension,
        /// Exact declared subject.
        subject: ImmutableRecordReferenceV1,
        /// Unsupported, omitted, failed, or truncated disposition.
        disposition: SourceCoverageGapKind,
    },
    /// One returned path depends on a heuristic edge.
    HeuristicEdge {
        /// Exact edge identity.
        edge: ImmutableRecordReferenceV1,
        /// Exact provider evidence for the heuristic.
        basis: ImmutableRecordReferenceV1,
    },
    /// A result collection hit the caller's exact cardinality ceiling.
    Truncated {
        /// Bounded collection.
        collection: SourceResultCollection,
        /// Exact additional members not returned.
        omitted: usize,
    },
}

/// Independently bounded result collection.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceResultCollection {
    /// Reached graph nodes.
    Nodes,
    /// Exact reason paths.
    ReasonPaths,
    /// Candidate affected tests.
    AffectedTests,
}

/// Candidate repository-native test and the structural reason it is affected.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct AffectedTestCandidate {
    /// Exact provider-owned test identity.
    pub test: ImmutableRecordReferenceV1,
    /// Canonical shortest cause-to-test path.
    pub reason: SourceReasonPath,
}

/// Measured bounded work/output of one query.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SourceQueryCost {
    /// Graph node count in the selected immutable generation.
    pub indexed_nodes: usize,
    /// Graph edge count in the selected immutable generation.
    pub indexed_edges: usize,
    /// Canonical evidence payload bytes before digest attachment.
    pub output_bytes: u64,
}

/// Bounded structural evidence from one exact ready index generation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceQueryResult {
    /// Exact admitted request identity.
    pub request_digest: ArtifactDigest,
    /// Exact provider/index/source-frontier coordinate.
    pub index: ProviderIndexCoordinateV1,
    /// Exact scope retained for cross-scope rejection.
    pub scope: SourceIndexScope,
    /// Canonically ordered reached subjects.
    pub nodes: Vec<ImmutableRecordReferenceV1>,
    /// Canonically ordered reason paths.
    pub reason_paths: Vec<SourceReasonPath>,
    /// Canonically ordered candidate tests.
    pub affected_tests: Vec<AffectedTestCandidate>,
    /// Explicit coverage and heuristic/truncation limits.
    pub limitations: Vec<SourceResultLimitation>,
    /// Bounded work/output coordinate.
    pub cost: SourceQueryCost,
    /// Digest of every preceding result member.
    pub result_digest: ArtifactDigest,
}

type SourceQueryProjection = (
    Vec<ImmutableRecordReferenceV1>,
    Vec<SourceReasonPath>,
    Vec<AffectedTestCandidate>,
    Vec<SourceResultLimitation>,
);

impl SourceQueryResult {
    /// Exact returned evidence references available for later use admission.
    #[must_use]
    pub fn evidence_references(&self) -> BTreeSet<ImmutableRecordReferenceV1> {
        let mut evidence = self.nodes.iter().cloned().collect::<BTreeSet<_>>();
        for path in &self.reason_paths {
            evidence.extend(path.nodes.iter().cloned());
            evidence.extend(path.edges.iter().cloned());
        }
        for candidate in &self.affected_tests {
            evidence.insert(candidate.test.clone());
            evidence.extend(candidate.reason.nodes.iter().cloned());
            evidence.extend(candidate.reason.edges.iter().cloned());
        }
        for limitation in &self.limitations {
            match limitation {
                SourceResultLimitation::CoverageGap { subject, .. } => {
                    evidence.insert(subject.clone());
                }
                SourceResultLimitation::HeuristicEdge { edge, basis } => {
                    evidence.insert(edge.clone());
                    evidence.insert(basis.clone());
                }
                SourceResultLimitation::Truncated { .. } => {}
            }
        }
        evidence
    }

    pub(super) fn has_valid_integrity(&self) -> Result<bool, CanonError> {
        let (cost, digest) = canonical_result_integrity(
            ResultPayload::from_result(self),
            self.cost.indexed_nodes,
            self.cost.indexed_edges,
        )?;
        Ok(self.cost == cost && self.result_digest == digest)
    }
}

/// Query-only half of the source-intelligence subsystem.
#[derive(Clone)]
pub struct SourceIntelligenceQueryService {
    pub(super) registry: Arc<RwLock<SourceIndexRegistry>>,
}

impl SourceIntelligenceQueryService {
    pub(super) fn new(registry: Arc<RwLock<SourceIndexRegistry>>) -> Self {
        Self { registry }
    }

    /// Answer one exact admitted query from the active immutable generation.
    ///
    /// # Errors
    ///
    /// Returns [`SourceQueryError`] when the exact generation is not active and
    /// queryable, an anchor is unknown, graph access fails, or the result
    /// exceeds its admitted output budget.
    pub fn query(
        &self,
        admitted: &AdmittedSourceQuery,
    ) -> Result<SourceQueryResult, SourceQueryError> {
        execute_query(&self.registry, admitted)
    }
}

impl SourceIntelligenceQueryPort for SourceIntelligenceQueryService {
    type Request = AdmittedSourceQuery;
    type Result = SourceQueryResult;
    type Error = SourceQueryError;

    fn query(&self, request: &Self::Request) -> Result<Self::Result, Self::Error> {
        SourceIntelligenceQueryService::query(self, request)
    }
}

fn execute_query(
    registry: &Arc<RwLock<SourceIndexRegistry>>,
    admitted: &AdmittedSourceQuery,
) -> Result<SourceQueryResult, SourceQueryError> {
    let request = admitted.request();
    let scope_digest = request.scope.lookup_digest()?;
    let index_digest = coordinate_digest(&request.index)?;
    // Keep the read fence for the complete bounded traversal. An invalidation,
    // removal, or activation change must linearize either before this query or
    // after its complete result; it cannot race a detached `Arc` snapshot.
    let registry = registry.read().map_err(|_| SourceQueryError::Poisoned)?;
    if registry.active_by_scope.get(&scope_digest) != Some(&index_digest) {
        return Err(SourceQueryError::GenerationNotActive);
    }
    let SourceIndexEntry::Retained { index, state, .. } =
        registry
            .generations
            .get(&index_digest)
            .ok_or(SourceQueryError::UnknownGeneration)?
    else {
        return Err(SourceQueryError::GenerationNotQueryable);
    };
    if *state != SourceIndexLifecycleState::Ready {
        return Err(SourceQueryError::GenerationNotQueryable);
    }
    if index.scope != request.scope
        || index.scope_digest != scope_digest
        || index.coordinate != request.index
        || index.coordinate_digest != index_digest
        || index.frontier.coordinate() != &request.index.repository_source_frontier
    {
        return Err(SourceQueryError::CoordinateMismatch);
    }

    let mut limitations = coverage_limitations(index);
    let (mut nodes, mut reason_paths, mut affected_tests, query_limitations) =
        match &request.question {
            SourceStructuralQuestion::Explore { origins, target } => {
                explore(index, origins, target.as_ref(), request.bounds)?
            }
            SourceStructuralQuestion::CandidateAffectedTests { changed } => {
                affected_tests(index, changed, request.bounds)?
            }
        };
    limitations.extend(query_limitations);
    truncate_vec(
        &mut nodes,
        request.bounds.nodes,
        SourceResultCollection::Nodes,
        &mut limitations,
    );
    truncate_vec(
        &mut reason_paths,
        request.bounds.reason_paths,
        SourceResultCollection::ReasonPaths,
        &mut limitations,
    );
    truncate_vec(
        &mut affected_tests,
        request.bounds.affected_tests,
        SourceResultCollection::AffectedTests,
        &mut limitations,
    );
    limitations.extend(heuristic_limitations(
        index,
        &reason_paths,
        &affected_tests,
    )?);
    limitations.sort();
    limitations.dedup();

    let payload = ResultPayload {
        request_digest: admitted.request_digest(),
        index: &request.index,
        scope: &request.scope,
        nodes: &nodes,
        reason_paths: &reason_paths,
        affected_tests: &affected_tests,
        limitations: &limitations,
    };
    let (cost, result_digest) =
        canonical_result_integrity(payload, index.graph.node_count(), index.graph.edge_count())?;
    if cost.output_bytes > request.bounds.output_bytes {
        return Err(SourceQueryError::OutputBudgetExceeded {
            observed: cost.output_bytes as usize,
            maximum: request.bounds.output_bytes,
        });
    }
    Ok(SourceQueryResult {
        request_digest: admitted.request_digest().clone(),
        index: request.index.clone(),
        scope: request.scope.clone(),
        nodes,
        reason_paths,
        affected_tests,
        limitations,
        cost,
        result_digest,
    })
}

#[derive(Serialize)]
struct ResultPayload<'a> {
    request_digest: &'a ArtifactDigest,
    index: &'a ProviderIndexCoordinateV1,
    scope: &'a SourceIndexScope,
    nodes: &'a [ImmutableRecordReferenceV1],
    reason_paths: &'a [SourceReasonPath],
    affected_tests: &'a [AffectedTestCandidate],
    limitations: &'a [SourceResultLimitation],
}

impl<'a> ResultPayload<'a> {
    fn from_result(result: &'a SourceQueryResult) -> Self {
        Self {
            request_digest: &result.request_digest,
            index: &result.index,
            scope: &result.scope,
            nodes: &result.nodes,
            reason_paths: &result.reason_paths,
            affected_tests: &result.affected_tests,
            limitations: &result.limitations,
        }
    }
}

#[derive(Serialize)]
struct ResultPreimage<'a> {
    #[serde(flatten)]
    payload: ResultPayload<'a>,
    cost: SourceQueryCost,
}

fn canonical_result_integrity(
    payload: ResultPayload<'_>,
    indexed_nodes: usize,
    indexed_edges: usize,
) -> Result<(SourceQueryCost, ArtifactDigest), CanonError> {
    let output_bytes = to_canonical_string(&payload)?.len() as u64;
    let cost = SourceQueryCost {
        indexed_nodes,
        indexed_edges,
        output_bytes,
    };
    let preimage = ResultPreimage { payload, cost };
    let digest = ArtifactDigest::hash_bytes(to_canonical_string(&preimage)?.as_bytes());
    Ok((cost, digest))
}

fn coverage_limitations(index: &super::ReadySourceIndex) -> Vec<SourceResultLimitation> {
    index
        .coverage
        .axes()
        .flat_map(|(dimension, partition)| {
            partition.gaps().iter().map(move |(subject, disposition)| {
                SourceResultLimitation::CoverageGap {
                    dimension,
                    subject: subject.clone(),
                    disposition: *disposition,
                }
            })
        })
        .collect()
}

fn explore(
    index: &super::ReadySourceIndex,
    origins: &[ImmutableRecordReferenceV1],
    target: Option<&ImmutableRecordReferenceV1>,
    bounds: SourceQueryBounds,
) -> Result<SourceQueryProjection, SourceQueryError> {
    let handles = origins
        .iter()
        .map(|origin| {
            index
                .graph
                .find(origin)
                .ok_or(SourceQueryError::UnknownAnchor)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut reached = BTreeSet::new();
    let mut paths = Vec::new();
    let mut limitations = Vec::new();
    let mut overlong_paths = 0_usize;
    for origin in handles {
        let proof = index.graph.reachability(origin)?;
        for node in proof.reached() {
            reached.insert(index.graph.node_key(node)?.clone());
        }
        limitations.extend(heuristic_limitations_for_handles(
            index,
            proof.witness_edges(),
        )?);
        if let Some(target) = target {
            let target = index
                .graph
                .find(target)
                .ok_or(SourceQueryError::UnknownAnchor)?;
            if let Some(path) = proof.path_to(target)? {
                if path.edges.len() <= bounds.path_edges {
                    paths.push(project_path(&index.graph, path)?);
                } else {
                    overlong_paths += 1;
                }
            }
        }
    }
    record_truncation(
        &mut limitations,
        SourceResultCollection::ReasonPaths,
        overlong_paths,
    );
    paths.sort();
    paths.dedup();
    Ok((
        reached.into_iter().collect(),
        paths,
        Vec::new(),
        limitations,
    ))
}

fn affected_tests(
    index: &super::ReadySourceIndex,
    changed: &[ImmutableRecordReferenceV1],
    bounds: SourceQueryBounds,
) -> Result<SourceQueryProjection, SourceQueryError> {
    let changed_handles = changed
        .iter()
        .map(|item| {
            index
                .graph
                .find(item)
                .ok_or(SourceQueryError::UnknownAnchor)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut batch = index.graph.batch_reachability();
    let closures = batch.query(
        Direction::Outgoing,
        changed_handles.iter().copied().map(|handle| [handle]),
    )?;
    let mut reached = BTreeSet::new();
    let mut tests = BTreeSet::new();
    for closure in closures {
        for handle in &closure {
            let node = index.graph.node_payload(handle)?;
            reached.insert(node.reference.clone());
            if node.kind == SourceNodeKind::Test {
                tests.insert(node.reference.clone());
            }
        }
    }
    let mut path_index = index.graph.reachability_index();
    let mut candidates = Vec::new();
    let mut limitations = Vec::new();
    let mut overlong_tests = 0_usize;
    for test in tests {
        let target = index.graph.find(&test).expect("test came from this graph");
        let mut path = None::<SourceReasonPath>;
        for origin in &changed_handles {
            if let Some(candidate) = path_index.path(*origin, target)? {
                let candidate = project_path(&index.graph, candidate)?;
                if path.as_ref().is_none_or(|selected| {
                    (candidate.edges.len(), &candidate) < (selected.edges.len(), selected)
                }) {
                    path = Some(candidate);
                }
            }
        }
        let Some(path) = path else {
            continue;
        };
        if path.edges.len() <= bounds.path_edges {
            candidates.push(AffectedTestCandidate { test, reason: path });
        } else {
            overlong_tests += 1;
        }
    }
    record_truncation(
        &mut limitations,
        SourceResultCollection::AffectedTests,
        overlong_tests,
    );
    let paths = candidates
        .iter()
        .map(|candidate| candidate.reason.clone())
        .collect();
    Ok((
        reached.into_iter().collect(),
        paths,
        candidates,
        limitations,
    ))
}

fn project_path(
    graph: &super::SourcePackedGraph,
    path: ReasonPath,
) -> Result<SourceReasonPath, SourceQueryError> {
    let nodes = path
        .nodes
        .into_iter()
        .map(|node| graph.node_key(node).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let edges = path
        .edges
        .into_iter()
        .map(|edge| {
            graph
                .edge_identity(edge)
                .map(|(identity, _, _)| identity.clone())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SourceReasonPath { nodes, edges })
}

fn heuristic_limitations(
    index: &super::ReadySourceIndex,
    paths: &[SourceReasonPath],
    tests: &[AffectedTestCandidate],
) -> Result<Vec<SourceResultLimitation>, SourceQueryError> {
    let edges = paths
        .iter()
        .flat_map(|path| path.edges.iter())
        .chain(tests.iter().flat_map(|test| test.reason.edges.iter()))
        .filter_map(|edge| index.edge_by_reference.get(edge).copied())
        .collect::<BTreeSet<_>>();
    heuristic_limitations_for_handles(index, edges)
}

fn heuristic_limitations_for_handles(
    index: &super::ReadySourceIndex,
    edges: impl IntoIterator<Item = EdgeHandle>,
) -> Result<Vec<SourceResultLimitation>, SourceQueryError> {
    let mut limitations = Vec::new();
    for edge in edges {
        let (identity, _, _) = index.graph.edge_identity(edge)?;
        let (_, provenance) = index.graph.edge_payload(edge)?;
        if provenance.kind == super::SourceEdgeProvenanceKind::Heuristic {
            limitations.push(SourceResultLimitation::HeuristicEdge {
                edge: identity.clone(),
                basis: provenance.basis.clone(),
            });
        }
    }
    Ok(limitations)
}

fn truncate_vec<T>(
    values: &mut Vec<T>,
    maximum: usize,
    collection: SourceResultCollection,
    limitations: &mut Vec<SourceResultLimitation>,
) {
    if values.len() > maximum {
        let omitted = values.len() - maximum;
        values.truncate(maximum);
        record_truncation(limitations, collection, omitted);
    }
}

fn record_truncation(
    limitations: &mut Vec<SourceResultLimitation>,
    collection: SourceResultCollection,
    omitted: usize,
) {
    if omitted == 0 {
        return;
    }
    if let Some(SourceResultLimitation::Truncated {
        omitted: already_omitted,
        ..
    }) = limitations.iter_mut().find(|limitation| {
        matches!(
            limitation,
            SourceResultLimitation::Truncated {
                collection: existing,
                ..
            } if *existing == collection
        )
    }) {
        *already_omitted = already_omitted
            .checked_add(omitted)
            .expect("omitted members are bounded by the source graph ceiling");
    } else {
        limitations.push(SourceResultLimitation::Truncated {
            collection,
            omitted,
        });
    }
}

/// Contextual request-admission failure.
#[derive(Debug, thiserror::Error)]
pub enum SourceQueryAdmissionError {
    /// Request names another operation.
    #[error("source-intelligence request names another operation")]
    OperationMismatch,
    /// No exact compiled capability obligation supports the request.
    #[error("source-intelligence request has no matching compiled capability obligation")]
    NoStructuralObligation,
    /// Ordered causal or anchor relation is empty, oversized, duplicated, or unordered.
    #[error("source-intelligence {relation} is not a canonical bounded non-empty path")]
    NonCanonicalBoundedRelation {
        /// Rejected relation.
        relation: &'static str,
    },
    /// At least one hard query bound is zero or above the application ceiling.
    #[error("source-intelligence request bounds are invalid")]
    InvalidBounds,
    /// Request digest projection failed.
    #[error("cannot canonicalize source-intelligence request: {0}")]
    Canonicalize(#[source] CanonError),
}

/// Failure to execute an admitted source-intelligence query.
#[derive(Debug, thiserror::Error)]
pub enum SourceQueryError {
    /// No retained generation has the requested exact identity.
    #[error("source-index generation is unknown")]
    UnknownGeneration,
    /// Requested generation is not the active generation for the exact scope.
    #[error("source-index generation is not active for the exact scope")]
    GenerationNotActive,
    /// Generation is invalidated, superseded, retired, removed, or otherwise fenced.
    #[error("source-index generation is not queryable")]
    GenerationNotQueryable,
    /// Exact scope/frontier/index coordinates do not all match.
    #[error("source-index query coordinate does not match the immutable generation")]
    CoordinateMismatch,
    /// A question anchor does not exist in the selected immutable graph.
    #[error("source-intelligence question names an unknown anchor")]
    UnknownAnchor,
    /// Canonical output exceeds the admitted byte ceiling.
    #[error("source-intelligence output contains {observed} bytes; maximum is {maximum}")]
    OutputBudgetExceeded {
        /// Observed canonical evidence payload bytes.
        observed: usize,
        /// Admitted maximum.
        maximum: u64,
    },
    /// Registry custody was poisoned by a panicking writer.
    #[error("source-index registry custody is poisoned")]
    Poisoned,
    /// Canonical digest projection failed.
    #[error("cannot canonicalize source-intelligence evidence: {0}")]
    Canonicalize(#[from] CanonError),
    /// Packed graph rejected a foreign or out-of-range handle.
    #[error("source-intelligence graph access failed: {0}")]
    Graph(#[from] pale_kernel::GraphAccessError),
}
