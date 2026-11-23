//! Disposable `GraphBLAS` projection for [`super::BatchReachability`].
//!
//! The public type owns the canonical dependency and fallback rationale; this
//! module contains only the bounded mechanical projection.

use issundb_graphblas::{
    ewise_add, mxv, Context, Descriptor, GraphblasError, Matrix, Monoid, Reducer, Semiring, Vector,
};
use petgraph::Direction;

use super::PackedGraph;

/// A support-algebra matrix derived once from one borrowed packed generation.
pub(super) struct ReachabilityIndex {
    context: std::sync::Arc<Context>,
    adjacency: Matrix<i32>,
    node_count: usize,
}

impl ReachabilityIndex {
    /// Project canonical endpoints while collapsing parallel edges only in this
    /// reachability-specific view.
    pub(super) fn build<K, N, I, E, R>(
        graph: &PackedGraph<K, N, I, E, R>,
    ) -> Result<Self, GraphblasError> {
        // The safe wrapper initializes the process-global runtime exactly once,
        // never finalizes it, and permits repeated handles to coexist.
        let context = Context::init_default()?;
        let triples = graph
            .edges
            .iter()
            .map(|edge| (edge.source as usize, edge.target as usize, 1_i32))
            .collect::<Vec<_>>();
        let mut adjacency = Matrix::from_triples(
            context.clone(),
            graph.nodes.len(),
            graph.nodes.len(),
            &triples,
            Reducer::First,
        )?;
        adjacency.wait()?;
        Ok(Self {
            context,
            adjacency,
            node_count: graph.nodes.len(),
        })
    }

    /// Evaluate every independent seed set over the same immutable matrix.
    pub(super) fn closures(
        &self,
        direction: Direction,
        batches: &[Vec<u32>],
    ) -> Result<Vec<Vec<u32>>, GraphblasError> {
        batches
            .iter()
            .map(|seeds| self.closure(direction, seeds))
            .collect()
    }

    fn closure(&self, direction: Direction, seeds: &[u32]) -> Result<Vec<u32>, GraphblasError> {
        let seed_pairs = seeds
            .iter()
            .map(|&index| (index as usize, 1_i32))
            .collect::<Vec<_>>();
        let mut visited = Vector::from_pairs(
            self.context.clone(),
            self.node_count,
            &seed_pairs,
            Reducer::First,
        )?;
        let mut frontier = Vector::from_pairs(
            self.context.clone(),
            self.node_count,
            &seed_pairs,
            Reducer::First,
        )?;
        let descriptor = Descriptor::new(true, true, true, direction == Direction::Outgoing);

        loop {
            let mut next = Vector::new(self.context.clone(), self.node_count)?;
            mxv(
                &mut next,
                Some(&visited),
                Semiring::MinSecond,
                &self.adjacency,
                &frontier,
                descriptor,
            )?;
            if next.nvals()? == 0 {
                // The safe wrapper promises ascending tuple indices. Preserve
                // that guarantee at this boundary because AdaptiveNodeSet's
                // canonical constructor requires sorted, duplicate-free input.
                let indices = visited.indices()?;
                return indices
                    .into_iter()
                    .map(|index| {
                        u32::try_from(index).map_err(|_| {
                            GraphblasError(
                                "GraphBLAS returned an index outside Pale's dense u32 space"
                                    .to_owned(),
                            )
                        })
                    })
                    .collect();
            }
            let mut expanded = Vector::new(self.context.clone(), self.node_count)?;
            ewise_add(
                &mut expanded,
                None,
                Monoid::Min,
                &visited,
                &next,
                Descriptor::NULL,
            )?;
            visited = expanded;
            frontier = next;
        }
    }
}
