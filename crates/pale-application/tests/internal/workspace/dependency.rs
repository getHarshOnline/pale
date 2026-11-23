//! Exercises workspace dependency admission, strongly connected components, and proof-carrying reachability.

use super::*;
use pale_core::records::{
    WorkspaceDeclarationId, WorkspaceDeclarationRefV1, WorkspaceModuleId, WorkspaceNamespaceId,
};

impl DeclarationDependencyAnalysis {
    /// Return the canonical SCC containing one declaration for focused tests.
    fn component_of(&self, declaration: &QualifiedDeclarationRef) -> Option<&DeclarationComponent> {
        self.component_by_declaration
            .get(declaration)
            .map(|id| &self.components[id.0])
    }
}

/// Construct one valid qualified declaration for focused graph tests.
fn declaration(module: &str, name: &str) -> QualifiedDeclarationRef {
    QualifiedDeclarationRef {
        owner_module: WorkspaceModuleId::parse(module).expect("valid test module id"),
        local_ref: WorkspaceDeclarationRefV1 {
            namespace: WorkspaceNamespaceId::parse("test").expect("valid test namespace"),
            declaration_id: WorkspaceDeclarationId::parse(name).expect("valid test declaration id"),
        },
    }
}

/// Construct one typed direct edge.
fn edge(
    source: &QualifiedDeclarationRef,
    target: &QualifiedDeclarationRef,
    role: DeclarationEdgeRole,
) -> DeclarationDependencyEdge {
    DeclarationDependencyEdge {
        source: source.clone(),
        target: target.clone(),
        role,
    }
}

/// Shorthand for a positive explicit declaration dependency.
fn positive(
    source: &QualifiedDeclarationRef,
    target: &QualifiedDeclarationRef,
) -> DeclarationDependencyEdge {
    edge(
        source,
        target,
        DeclarationEdgeRole::Explicit(DeclarationDependencyKindV1::Positive),
    )
}

/// Admit and analyze a focused graph fixture.
fn analyze(
    declarations: impl IntoIterator<Item = QualifiedDeclarationRef>,
    edges: impl IntoIterator<Item = DeclarationDependencyEdge>,
) -> DeclarationDependencyAnalysis {
    DeclarationDependencyGraph::admit(declarations, edges)
        .expect("graph fixture is structurally valid")
        .analyze()
        .expect("graph fixture has admissible cycles")
}

#[test]
fn positive_self_loop_and_relationship_roles_are_retained() {
    let node = declaration("module-a", "node");
    let analysis = analyze(
        [node.clone()],
        [
            positive(&node, &node),
            edge(&node, &node, DeclarationEdgeRole::RelationshipFrom),
            edge(&node, &node, DeclarationEdgeRole::RelationshipTo),
        ],
    );

    assert_eq!(analysis.components.len(), 1);
    assert_eq!(analysis.components[0].members, BTreeSet::from([node]));
    assert!(analysis.components[0].dependencies.is_empty());
    assert_eq!(analysis.witness.direct_edge_count, 3);
}

#[test]
fn multi_node_and_disjoint_components_have_canonical_dependencies() {
    let a = declaration("module-a", "a");
    let b = declaration("module-a", "b");
    let c = declaration("module-a", "c");
    let d = declaration("module-a", "d");
    let analysis = analyze(
        [d.clone(), b.clone(), a.clone(), c.clone()],
        [positive(&a, &b), positive(&b, &a), positive(&d, &c)],
    );

    assert_eq!(analysis.components.len(), 3);
    assert_eq!(
        analysis.component_of(&a).expect("a component").members,
        BTreeSet::from([a.clone(), b.clone()])
    );
    assert_ne!(
        analysis.component_of(&a).expect("a component").id,
        analysis.component_of(&c).expect("c component").id
    );
    let d_component = analysis.component_of(&d).expect("d component");
    assert_eq!(
        d_component.dependencies,
        BTreeSet::from([analysis.component_of(&c).expect("c component").id])
    );
    assert_eq!(analysis.witness.forward_completed, 4);
    assert_eq!(analysis.witness.reverse_assigned, 4);
}

#[test]
fn every_non_positive_kind_is_rejected_inside_a_cycle() {
    let a = declaration("module-a", "a");
    let b = declaration("module-a", "b");
    for kind in [
        DeclarationDependencyKindV1::Negation,
        DeclarationDependencyKindV1::Effect,
        DeclarationDependencyKindV1::Callback,
        DeclarationDependencyKindV1::UnboundedGeneration,
    ] {
        let graph = DeclarationDependencyGraph::admit(
            [a.clone(), b.clone()],
            [
                positive(&a, &b),
                edge(&b, &a, DeclarationEdgeRole::Explicit(kind)),
            ],
        )
        .expect("bounded graph");
        assert!(matches!(
            graph.analyze(),
            Err(WorkspaceError::InadmissibleDeclarationCycle(_))
        ));

        let self_loop = DeclarationDependencyGraph::admit(
            [a.clone()],
            [edge(&a, &a, DeclarationEdgeRole::Explicit(kind))],
        )
        .expect("bounded self-loop");
        assert!(matches!(
            self_loop.analyze(),
            Err(WorkspaceError::InadmissibleDeclarationCycle(_))
        ));
    }
}

#[test]
fn analysis_is_permutation_invariant() {
    let a = declaration("module-a", "a");
    let b = declaration("module-a", "b");
    let c = declaration("module-a", "c");
    let nodes = vec![a.clone(), b.clone(), c.clone()];
    let edges = vec![positive(&a, &b), positive(&b, &a), positive(&c, &a)];
    let first = analyze(nodes.clone(), edges.clone());
    let second = analyze(nodes.into_iter().rev(), edges.into_iter().rev());
    assert_eq!(first, second);
}

#[test]
fn maximum_declaration_chain_uses_an_explicit_stack() {
    let nodes = (0..MAX_WORKSPACE_DECLARATIONS)
        .map(|index| declaration("module-a", &format!("d{index:04}")))
        .collect::<Vec<_>>();
    let edges = nodes
        .windows(2)
        .map(|pair| positive(&pair[0], &pair[1]))
        .collect::<Vec<_>>();
    let analysis = analyze(nodes.clone(), edges);

    assert_eq!(
        analysis.witness.declaration_count,
        MAX_WORKSPACE_DECLARATIONS
    );
    assert_eq!(analysis.witness.component_count, MAX_WORKSPACE_DECLARATIONS);
}

#[test]
fn one_condensation_pass_propagates_exact_inputs_and_counts_work() {
    let dependency = declaration("module-a", "dependency");
    let cycle_a = declaration("module-a", "cycle-a");
    let cycle_b = declaration("module-a", "cycle-b");
    let unrelated = declaration("module-a", "unrelated");
    let analysis = analyze(
        [
            dependency.clone(),
            cycle_a.clone(),
            cycle_b.clone(),
            unrelated.clone(),
        ],
        [
            positive(&cycle_a, &cycle_b),
            positive(&cycle_b, &cycle_a),
            positive(&cycle_a, &dependency),
        ],
    );

    let propagation = analysis.input_propagation();
    assert_eq!(propagation.components_finalized, analysis.components.len());
    assert!(propagation.member_insertions >= analysis.witness.declaration_count);
    assert_eq!(
        propagation.inputs_for(&cycle_a).unwrap(),
        &BTreeSet::from([dependency.clone(), cycle_a.clone(), cycle_b.clone()])
    );
    assert_eq!(
        propagation.inputs_for(&cycle_a),
        propagation.inputs_for(&cycle_b)
    );
    assert_eq!(
        propagation.inputs_for(&dependency).unwrap(),
        &BTreeSet::from([dependency])
    );
    assert_eq!(
        propagation.inputs_for(&unrelated).unwrap(),
        &BTreeSet::from([unrelated])
    );
}
