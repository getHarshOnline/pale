//! Deterministic dependency algebra for linked workspace declarations.
//!
//! [`DeclarationDependencyGraph`] owns one bounded set of declaration identities
//! and their direct typed edges. Its analysis collapses positive cycles into
//! canonical [`DeclarationComponent`] values and retains only direct
//! condensation dependencies. Reverse impact is computed transiently from the
//! same direct edges, so affected-region queries cannot drift from cycle
//! analysis through a second adjacency owner.
//!
//! Ordered maps and sets make the result independent of source insertion order.
//! The complexity bounds are rendered on [`DeclarationDependencyAnalysis`].
//! The number of traversal steps is linear in the constructed adjacency,
//! excluding the bounded cost of cloning typed declaration identities. All
//! walks use explicit stacks.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    DeclarationDependencyKindV1, MAX_WORKSPACE_DECLARATIONS, MAX_WORKSPACE_DECLARATION_EDGES,
};

use super::link::QualifiedDeclarationRef;
use super::WorkspaceError;

/// Semantic role of one direct declaration edge.
///
/// Relationship endpoints are positive structural references, but retaining
/// their distinct roles lets later provenance identify whether a declaration
/// was used as the relationship source or destination. [`Self::Explicit`]
/// preserves the authored dependency kind exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum DeclarationEdgeRole {
    /// Explicit dependency authored on a declaration.
    Explicit(DeclarationDependencyKindV1),
    /// Source endpoint of a relationship declaration.
    RelationshipFrom,
    /// Destination endpoint of a relationship declaration.
    RelationshipTo,
}

impl DeclarationEdgeRole {
    /// Return whether this edge is admissible inside a declaration cycle.
    ///
    /// This classifies the closed edge vocabulary; it does not independently
    /// prove convergence for an arbitrary computation.
    fn is_positive(self) -> bool {
        matches!(
            self,
            Self::Explicit(DeclarationDependencyKindV1::Positive)
                | Self::RelationshipFrom
                | Self::RelationshipTo
        )
    }
}

impl std::fmt::Display for DeclarationEdgeRole {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Explicit(kind) => write!(formatter, "explicit {kind}"),
            Self::RelationshipFrom => formatter.write_str("relationship-from"),
            Self::RelationshipTo => formatter.write_str("relationship-to"),
        }
    }
}

/// One canonical direct dependency: `source` depends on `target` through `role`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DeclarationDependencyEdge {
    /// Declaration whose meaning consumes the target.
    pub(super) source: QualifiedDeclarationRef,
    /// Declaration consumed by the source.
    pub(super) target: QualifiedDeclarationRef,
    /// Exact semantic role of this dependency.
    pub(super) role: DeclarationEdgeRole,
}

/// Stable index of one component inside a canonical analysis result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DeclarationComponentId(usize);

/// One strongly connected declaration component and its direct dependencies.
///
/// Members are sorted. `dependencies` names other components consumed by this
/// component; self-dependencies are represented by membership, not by a
/// condensation edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclarationComponent {
    /// Canonical component identity within this analysis.
    pub(super) id: DeclarationComponentId,
    /// Complete, sorted SCC membership.
    pub(super) members: BTreeSet<QualifiedDeclarationRef>,
    /// Direct component dependencies after SCC condensation.
    pub(super) dependencies: BTreeSet<DeclarationComponentId>,
}

/// Bounded evidence that both SCC passes consumed the complete admitted graph.
///
/// Equality of each visit count with `declaration_count` is established while
/// constructing [`DeclarationDependencyAnalysis`]. This witnesses termination
/// of the finite graph analysis; it is not evidence for native effects or a
/// general-purpose rule evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclarationConvergenceWitness {
    /// Number of unique admitted declarations.
    pub(super) declaration_count: usize,
    /// Number of unique admitted direct typed edges.
    pub(super) direct_edge_count: usize,
    /// Declarations completed by the forward pass.
    pub(super) forward_completed: usize,
    /// Declarations assigned by the reverse pass.
    pub(super) reverse_assigned: usize,
    /// Number of canonical SCCs.
    pub(super) component_count: usize,
    /// Number of direct edges in the condensed acyclic graph.
    pub(super) condensation_edge_count: usize,
}

/// Exact transitive semantic inputs derived once from the SCC condensation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclarationInputPropagation<'a> {
    closures: BTreeMap<DeclarationComponentId, BTreeSet<QualifiedDeclarationRef>>,
    component_by_declaration: &'a BTreeMap<QualifiedDeclarationRef, DeclarationComponentId>,
    /// Number of SCCs finalized by the dependency-first pass.
    pub(super) components_finalized: usize,
    /// Number of member insertions attempted while materializing closures.
    pub(super) member_insertions: usize,
}

impl DeclarationInputPropagation<'_> {
    pub(super) fn inputs_for(
        &self,
        declaration: &QualifiedDeclarationRef,
    ) -> Option<&BTreeSet<QualifiedDeclarationRef>> {
        self.component_by_declaration
            .get(declaration)
            .and_then(|id| self.closures.get(id))
    }
}

/// Bounded canonical owner of declaration identities and direct typed edges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclarationDependencyGraph {
    declarations: BTreeSet<QualifiedDeclarationRef>,
    direct_edges: BTreeSet<DeclarationDependencyEdge>,
}

impl DeclarationDependencyGraph {
    /// Admit a complete finite declaration graph before SCC analysis.
    ///
    /// Duplicate facts are idempotent after their authored aggregate has passed
    /// the same operational bounds as unique facts. Every edge endpoint must be
    /// present in `declarations`.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceGraphLimit`](WorkspaceError::WorkspaceGraphLimit) when
    /// the declaration or edge aggregate exceeds its workspace-language bound.
    /// Returns [`NamespaceResolution`](WorkspaceError::NamespaceResolution) when
    /// an edge endpoint is absent.
    pub(super) fn admit(
        declarations: impl IntoIterator<Item = QualifiedDeclarationRef>,
        direct_edges: impl IntoIterator<Item = DeclarationDependencyEdge>,
    ) -> Result<Self, WorkspaceError> {
        let declarations = declarations.into_iter().collect::<Vec<_>>();
        enforce_bound(
            "dependency declarations",
            declarations.len(),
            MAX_WORKSPACE_DECLARATIONS,
        )?;
        let direct_edges = direct_edges.into_iter().collect::<Vec<_>>();
        enforce_bound(
            "direct declaration dependency edges",
            direct_edges.len(),
            MAX_WORKSPACE_DECLARATION_EDGES,
        )?;

        let declarations = declarations.into_iter().collect::<BTreeSet<_>>();
        let direct_edges = direct_edges.into_iter().collect::<BTreeSet<_>>();
        for edge in &direct_edges {
            if !declarations.contains(&edge.source) {
                return Err(absent_endpoint("source", &edge.source, edge));
            }
            if !declarations.contains(&edge.target) {
                return Err(absent_endpoint("target", &edge.target, edge));
            }
        }
        Ok(Self {
            declarations,
            direct_edges,
        })
    }

    /// Classify SCCs, reject non-positive cyclic edges, and condense the graph.
    ///
    /// SCC membership is canonicalized by sorted member set rather than
    /// discovery order. A relationship endpoint is always positive. An explicit
    /// dependency is positive only when its kind is
    /// [`DeclarationDependencyKindV1::Positive`].
    ///
    /// # Errors
    ///
    /// Returns
    /// [`InadmissibleDeclarationCycle`](WorkspaceError::InadmissibleDeclarationCycle)
    /// when any non-positive edge lies inside a multi-node SCC or is a self-loop.
    pub(super) fn analyze(self) -> Result<DeclarationDependencyAnalysis, WorkspaceError> {
        let (forward, reverse) = self.adjacency();
        let finish_order = finish_order(&self.declarations, &forward);
        let mut assigned = BTreeSet::new();
        let mut raw_components = Vec::new();

        for root in finish_order.iter().rev() {
            if assigned.contains(root) {
                continue;
            }
            let members = collect_component(root, &reverse, &mut assigned);
            raw_components.push(members);
        }
        raw_components.sort();

        let mut component_by_declaration = BTreeMap::new();
        for (index, members) in raw_components.iter().enumerate() {
            let id = DeclarationComponentId(index);
            for member in members {
                component_by_declaration.insert(member.clone(), id);
            }
        }
        reject_inadmissible_cycles(
            &raw_components,
            &component_by_declaration,
            &self.direct_edges,
        )?;

        let mut condensation_edges = BTreeSet::new();
        for edge in &self.direct_edges {
            let source = component_by_declaration[&edge.source];
            let target = component_by_declaration[&edge.target];
            if source != target {
                condensation_edges.insert((source, target));
            }
        }
        let components = raw_components
            .into_iter()
            .enumerate()
            .map(|(index, members)| {
                let id = DeclarationComponentId(index);
                let dependencies = condensation_edges
                    .iter()
                    .filter_map(|(source, target)| (*source == id).then_some(*target))
                    .collect();
                DeclarationComponent {
                    id,
                    members,
                    dependencies,
                }
            })
            .collect::<Vec<_>>();
        let witness = DeclarationConvergenceWitness {
            declaration_count: self.declarations.len(),
            direct_edge_count: self.direct_edges.len(),
            forward_completed: finish_order.len(),
            reverse_assigned: assigned.len(),
            component_count: components.len(),
            condensation_edge_count: condensation_edges.len(),
        };
        debug_assert_eq!(witness.forward_completed, witness.declaration_count);
        debug_assert_eq!(witness.reverse_assigned, witness.declaration_count);

        Ok(DeclarationDependencyAnalysis {
            components,
            component_by_declaration,
            witness,
        })
    }

    /// Build sorted forward and reverse adjacency from the canonical direct edges.
    fn adjacency(&self) -> (Adjacency, Adjacency) {
        let mut forward = empty_adjacency(&self.declarations);
        let mut reverse = empty_adjacency(&self.declarations);
        for edge in &self.direct_edges {
            forward
                .get_mut(&edge.source)
                .expect("admitted edge source")
                .push(edge.target.clone());
            reverse
                .get_mut(&edge.target)
                .expect("admitted edge target")
                .push(edge.source.clone());
        }
        for adjacent in forward.values_mut().chain(reverse.values_mut()) {
            adjacent.sort();
            adjacent.dedup();
        }
        (forward, reverse)
    }
}

/// Canonical SCC analysis and its finite condensation evidence.
///
/// In the rendered closure law, `C` ranges over condensation components,
/// `E_c` is their direct dependency edge set, and `F` counts bounded set-union
/// materialization. `V` and `E` name the original declaration graph.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/workspace-dependency-closure.display.mathml"
))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclarationDependencyAnalysis {
    /// Components sorted by their complete member sets.
    pub(super) components: Vec<DeclarationComponent>,
    component_by_declaration: BTreeMap<QualifiedDeclarationRef, DeclarationComponentId>,
    /// Finite-pass completion evidence for this analysis.
    pub(super) witness: DeclarationConvergenceWitness,
}

impl DeclarationDependencyAnalysis {
    /// Derive all transitive declaration inputs with one condensation traversal.
    ///
    /// The least finite solution and its materialization bound are rendered on
    /// [`DeclarationDependencyAnalysis`]. Each component is finalized once
    /// after its direct dependencies; no peer graph is constructed or retained.
    pub(super) fn input_propagation(&self) -> DeclarationInputPropagation<'_> {
        let mut order = Vec::with_capacity(self.components.len());
        let mut visited = BTreeSet::new();
        for component in &self.components {
            let mut stack = vec![(component.id, false)];
            while let Some((id, exiting)) = stack.pop() {
                if exiting {
                    order.push(id);
                    continue;
                }
                if !visited.insert(id) {
                    continue;
                }
                stack.push((id, true));
                for dependency in self.components[id.0].dependencies.iter().rev() {
                    if !visited.contains(dependency) {
                        stack.push((*dependency, false));
                    }
                }
            }
        }

        let mut by_component = BTreeMap::<DeclarationComponentId, BTreeSet<_>>::new();
        let mut member_insertions = 0;
        for id in &order {
            let component = &self.components[id.0];
            let mut closure = component.members.clone();
            member_insertions += component.members.len();
            for dependency in &component.dependencies {
                let dependency_inputs = &by_component[dependency];
                member_insertions += dependency_inputs.len();
                closure.extend(dependency_inputs.iter().cloned());
            }
            by_component.insert(*id, closure);
        }
        DeclarationInputPropagation {
            closures: by_component,
            component_by_declaration: &self.component_by_declaration,
            components_finalized: order.len(),
            member_insertions,
        }
    }
}

/// Sorted adjacency keyed by every admitted declaration, including isolated ones.
type Adjacency = BTreeMap<QualifiedDeclarationRef, Vec<QualifiedDeclarationRef>>;

/// Reject the first aggregate beyond a workspace-language graph bound.
fn enforce_bound(kind: &'static str, observed: usize, limit: usize) -> Result<(), WorkspaceError> {
    if observed > limit {
        return Err(WorkspaceError::WorkspaceGraphLimit {
            kind,
            observed,
            limit,
        });
    }
    Ok(())
}

/// Explain one direct edge whose requested endpoint is absent.
fn absent_endpoint(
    endpoint: &'static str,
    declaration: &QualifiedDeclarationRef,
    edge: &DeclarationDependencyEdge,
) -> WorkspaceError {
    WorkspaceError::NamespaceResolution(format!(
        "{endpoint} `{declaration}` is absent for {} dependency `{} -> {}`",
        edge.role, edge.source, edge.target
    ))
}

/// Create an empty adjacency entry for every admitted declaration.
fn empty_adjacency(declarations: &BTreeSet<QualifiedDeclarationRef>) -> Adjacency {
    declarations
        .iter()
        .cloned()
        .map(|declaration| (declaration, Vec::new()))
        .collect()
}

/// Compute deterministic DFS finish order without recursive stack growth.
fn finish_order(
    declarations: &BTreeSet<QualifiedDeclarationRef>,
    forward: &Adjacency,
) -> Vec<QualifiedDeclarationRef> {
    enum Frame {
        Enter(QualifiedDeclarationRef),
        Exit(QualifiedDeclarationRef),
    }

    let mut visited = BTreeSet::new();
    let mut order = Vec::with_capacity(declarations.len());
    for root in declarations {
        if visited.contains(root) {
            continue;
        }
        let mut stack = vec![Frame::Enter(root.clone())];
        while let Some(frame) = stack.pop() {
            match frame {
                Frame::Enter(declaration) => {
                    if !visited.insert(declaration.clone()) {
                        continue;
                    }
                    stack.push(Frame::Exit(declaration.clone()));
                    for dependency in forward[&declaration].iter().rev() {
                        if !visited.contains(dependency) {
                            stack.push(Frame::Enter(dependency.clone()));
                        }
                    }
                }
                Frame::Exit(declaration) => order.push(declaration),
            }
        }
    }
    order
}

/// Collect one reverse-graph component with an explicit stack.
fn collect_component(
    root: &QualifiedDeclarationRef,
    reverse: &Adjacency,
    assigned: &mut BTreeSet<QualifiedDeclarationRef>,
) -> BTreeSet<QualifiedDeclarationRef> {
    let mut members = BTreeSet::new();
    let mut stack = vec![root.clone()];
    while let Some(declaration) = stack.pop() {
        if !assigned.insert(declaration.clone()) {
            continue;
        }
        members.insert(declaration.clone());
        for predecessor in reverse[&declaration].iter().rev() {
            if !assigned.contains(predecessor) {
                stack.push(predecessor.clone());
            }
        }
    }
    members
}

/// Reject each non-positive edge that participates in an actual cycle.
fn reject_inadmissible_cycles(
    components: &[BTreeSet<QualifiedDeclarationRef>],
    component_by_declaration: &BTreeMap<QualifiedDeclarationRef, DeclarationComponentId>,
    direct_edges: &BTreeSet<DeclarationDependencyEdge>,
) -> Result<(), WorkspaceError> {
    for edge in direct_edges {
        let source_component = component_by_declaration[&edge.source];
        let target_component = component_by_declaration[&edge.target];
        let is_internal = source_component == target_component;
        let is_cycle = components[source_component.0].len() > 1 || edge.source == edge.target;
        if is_internal && is_cycle && !edge.role.is_positive() {
            return Err(WorkspaceError::InadmissibleDeclarationCycle(format!(
                "{} edge from `{}` to `{}`",
                edge.role, edge.source, edge.target
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/internal/workspace/dependency.rs"]
mod tests;
