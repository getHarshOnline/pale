//! Portable in-memory mechanics for one admitted Pale resident generation.
//!
//! This crate builds on `pale-core`; `pale-application` builds resident state
//! on it. It owns packed adjacency, generation-local bodies, and rebuildable
//! set algebra. It owns no durable journal, runtime activation, native effect,
//! Package meaning, workspace selection, or provider state.

mod arena;
mod graph;
mod node_set;
mod wake_ring;

pub use arena::{ArenaError, ArenaHandle, GenerationArena, RuntimeArena, ScratchArena};
pub use graph::{
    BatchReachability, BatchReachabilityBackend, DirectedEdgeHandles, EdgeHandle, GraphAccessError,
    GraphBuildError, GraphEdge, GraphGeneration, GraphNode, GraphTopology, ImpactClosure,
    NodeHandle, PackedGraph, PetgraphEdgeReference, PetgraphEdgeReferences, PetgraphEdges,
    PetgraphNeighbors, PetgraphNodeIdentifiers, PetgraphView, PetgraphVisitTracker,
    ReachabilityIndex, ReachabilityProof, ReasonPath,
};
pub use node_set::{AdaptiveNodeSet, AdaptiveNodeSetEncoding, AdaptiveNodeSetPolicy, NodeSetError};
pub use petgraph::Direction;
pub use wake_ring::{
    WakeBatch, WakeConsumerCursor, WakePublishError, WakeReadError, WakeRingBuildError, WakeToken,
    WakeTokenRing,
};
