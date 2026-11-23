//! Provider-neutral source graph declarations.

use pale_core::{paths::OwnedConfinedRelativePath, records::ImmutableRecordReferenceV1};
use pale_kernel::{GraphEdge, GraphNode};
use serde::Serialize;

use super::SourceEdgeProvenance;

/// Maximum nodes in one admitted source-index generation.
pub const MAX_SOURCE_GRAPH_NODES: usize = 65_536;
/// Maximum relationships in one admitted source-index generation.
pub const MAX_SOURCE_GRAPH_EDGES: usize = 262_144;

/// Source-index node class.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceNodeKind {
    /// Complete source file.
    File,
    /// Parser- or resolver-owned symbol.
    Symbol,
    /// Exact source range without a stable symbol identity.
    Range,
    /// Repository-native test target or test case.
    Test,
}

/// Exact source location attached to a node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceLocation {
    /// Repository-relative source path.
    pub path: OwnedConfinedRelativePath,
    /// Zero-based inclusive byte offset.
    pub start_byte: u32,
    /// Zero-based exclusive byte offset.
    pub end_byte: u32,
}

impl SourceLocation {
    /// Reprove the half-open source range.
    ///
    /// # Errors
    ///
    /// Returns [`SourceGraphDeclarationError::InvalidRange`] when the end lies
    /// before the start.
    pub fn validate(&self) -> Result<(), SourceGraphDeclarationError> {
        if self.start_byte <= self.end_byte {
            Ok(())
        } else {
            Err(SourceGraphDeclarationError::InvalidRange)
        }
    }
}

/// Body-free source-index node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceGraphNode {
    /// Stable exact evidence reference used as the semantic graph key.
    pub reference: ImmutableRecordReferenceV1,
    /// Structural node class.
    pub kind: SourceNodeKind,
    /// Exact source location when the owner exposes one.
    pub location: Option<SourceLocation>,
}

/// Cause-to-dependent source relationship used for impact traversal.
///
/// The direction is deliberate: a changed cause reaches every dependent. This
/// makes forward impact traversal direct. [`pale_kernel::PackedGraph`]
/// separately retains reverse adjacency for predecessor queries; edge
/// direction and physical adjacency layout are independent decisions.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDependencyKind {
    /// A file owns a nested symbol or range.
    Contains,
    /// The target imports the source.
    ImportedBy,
    /// The target calls the source.
    CalledBy,
    /// The target implements or specializes the source.
    ImplementedBy,
    /// The target references the source through another exact edge class.
    ReferencedBy,
    /// The source produces the generated target.
    Generates,
    /// The target test exercises the source.
    TestedBy,
}

/// One exact relationship in the provider-owned derived index.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceGraphEdge {
    /// Stable relationship identity.
    pub reference: ImmutableRecordReferenceV1,
    /// Causal/dependency input.
    pub cause: ImmutableRecordReferenceV1,
    /// Downstream dependent.
    pub dependent: ImmutableRecordReferenceV1,
    /// Closed structural relationship.
    pub kind: SourceDependencyKind,
    /// Method, exact basis, and independent confidence state.
    pub provenance: SourceEdgeProvenance,
}

pub(super) type SourcePackedGraph = pale_kernel::PackedGraph<
    ImmutableRecordReferenceV1,
    SourceGraphNode,
    ImmutableRecordReferenceV1,
    SourceDependencyKind,
    SourceEdgeProvenance,
>;

type SourceGraphInputs = (
    Vec<GraphNode<ImmutableRecordReferenceV1, SourceGraphNode>>,
    Vec<
        GraphEdge<
            ImmutableRecordReferenceV1,
            ImmutableRecordReferenceV1,
            SourceDependencyKind,
            SourceEdgeProvenance,
        >,
    >,
);

pub(super) fn graph_inputs(
    nodes: Vec<SourceGraphNode>,
    edges: Vec<SourceGraphEdge>,
) -> Result<SourceGraphInputs, SourceGraphDeclarationError> {
    if nodes.len() > MAX_SOURCE_GRAPH_NODES {
        return Err(SourceGraphDeclarationError::TooManyNodes(nodes.len()));
    }
    if edges.len() > MAX_SOURCE_GRAPH_EDGES {
        return Err(SourceGraphDeclarationError::TooManyEdges(edges.len()));
    }
    for node in &nodes {
        if let Some(location) = &node.location {
            location.validate()?;
        }
    }
    Ok((
        nodes
            .into_iter()
            .map(|node| GraphNode {
                key: node.reference.clone(),
                payload: node,
            })
            .collect(),
        edges
            .into_iter()
            .map(|edge| GraphEdge {
                id: edge.reference,
                source: edge.cause,
                target: edge.dependent,
                payload: edge.kind,
                reason: edge.provenance,
            })
            .collect(),
    ))
}

/// Invalid bounded source-graph declaration.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SourceGraphDeclarationError {
    /// Node bound exceeded.
    #[error("source graph contains {0} nodes; maximum is 65536")]
    TooManyNodes(usize),
    /// Edge bound exceeded.
    #[error("source graph contains {0} edges; maximum is 262144")]
    TooManyEdges(usize),
    /// Half-open source range is inverted.
    #[error("source graph range ends before it starts")]
    InvalidRange,
    /// A node location lies outside the admitted source frontier.
    #[error("source graph node path `{0}` is not admitted by the source frontier")]
    LocationOutsideFrontier(OwnedConfinedRelativePath),
}
