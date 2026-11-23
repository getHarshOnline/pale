//! Behavioral laws for exact-frontier source intelligence.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::{
    canon::to_canonical_string,
    paths::OwnedConfinedRelativePath,
    records::{
        Approval, ArtifactDigest, CapabilityBinding, CapabilityIdV1, CapabilityProviderBinding,
        CapabilityRequirementIdV1, CompiledGenerationDigest, DeploymentIdV1, EffectClass,
        FailurePosture, ImmutableRecordReferenceV1, LogicalExecutionWorkspaceIdV1,
        LogicalProjectIdV1, LogicalRepositoryIdV1, OperationLeaseIdV1, OperationSelection,
        PackageIdV1, PackageResourceIdV1, PackageResourceRefV1, ProviderIndexCoordinateV1,
        ProviderIndexGenerationV1, ProviderIndexOwnerV1, RepositoryRevisionV1,
        RepositorySourceFrontierV1, ResidentControllerIdV1, ResidentPrincipalIdV1,
        ResidentRecordRefV1, ResourceContinuityMachineScopeIdV1, UnixTimeV1,
    },
};
use pale_kernel::AdaptiveNodeSetPolicy;
use serde::Serialize;

use crate::semantic::{
    CapabilityObligation, CapabilityObligationDisposition, ObligationSubject, OperationObligation,
    OperationObligationState, ProviderProcedureObligation,
};

use super::*;

fn reference(id: &str) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse(id).expect("valid record reference"),
        record_sha256: ArtifactDigest::hash_bytes(id.as_bytes()),
    }
}

fn resource(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("source-intelligence-tests").expect("valid package"),
        resource_id: PackageResourceIdV1::parse(id).expect("valid resource"),
    }
}

fn scope(provider: &str) -> SourceIndexScope {
    SourceIndexScope {
        principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("valid principal"),
        controller_id: ResidentControllerIdV1::parse("controller-1").expect("valid controller"),
        deployment_id: DeploymentIdV1::parse("deployment-1").expect("valid deployment"),
        machine_id: ResourceContinuityMachineScopeIdV1::parse("machine-1").expect("valid machine"),
        project_id: LogicalProjectIdV1::parse("project-1").expect("valid project"),
        execution_workspace_id: LogicalExecutionWorkspaceIdV1::parse("workspace-1")
            .expect("valid workspace"),
        repository_id: LogicalRepositoryIdV1::parse("repository-1").expect("valid repository"),
        worktree: reference("worktree-1"),
        provider_index_owner: ProviderIndexOwnerV1::parse(provider).expect("valid provider"),
        provider_pin: reference("provider-pin-1"),
    }
}

#[derive(Serialize)]
struct UntrackedPreimage<'a> {
    untracked: &'a [SourceByteMember],
    generated: &'a [SourceByteMember],
}

#[derive(Serialize)]
struct DependencyPreimage<'a> {
    source_roots: &'a BTreeSet<OwnedConfinedRelativePath>,
    submodules: &'a [SubmoduleFrontier],
    policy: &'a SourceFrontierPolicy,
}

fn digest(value: &impl Serialize) -> ArtifactDigest {
    ArtifactDigest::hash_bytes(
        to_canonical_string(value)
            .expect("fixture is canonical")
            .as_bytes(),
    )
}

fn frontier(repository_id: LogicalRepositoryIdV1) -> AdmittedSourceFrontier {
    let source_roots =
        BTreeSet::from([OwnedConfinedRelativePath::parse("src").expect("confined source root")]);
    let tracked = vec![TrackedSourceMember {
        path: OwnedConfinedRelativePath::parse("src/lib.rs").expect("confined source path"),
        disposition: TrackedSourceDisposition::Modified,
        content_sha256: Some(ArtifactDigest::hash_bytes(b"source bytes")),
        content_bytes: Some(12),
    }];
    let untracked = Vec::new();
    let generated = Vec::new();
    let submodules = Vec::new();
    let policy = SourceFrontierPolicy {
        vendor: VendorSourcePolicy::Exclude,
        vendor_roots: BTreeSet::new(),
        symlinks: SourceSymlinkPolicy::Reject,
        excluded_roots: BTreeSet::new(),
        concurrent_mutation: ConcurrentMutationPolicy::Fail,
    };
    let coordinate = RepositorySourceFrontierV1 {
        repository_id,
        base_revision: RepositoryRevisionV1::parse("revision-1").expect("valid revision"),
        tracked_delta_sha256: digest(&tracked),
        untracked_inventory_sha256: digest(&UntrackedPreimage {
            untracked: &untracked,
            generated: &generated,
        }),
        dependency_frontier_sha256: digest(&DependencyPreimage {
            source_roots: &source_roots,
            submodules: &submodules,
            policy: &policy,
        }),
    };
    admit_source_frontier(SourceFrontierCandidate {
        coordinate,
        source_roots,
        tracked,
        untracked,
        generated,
        submodules,
        policy,
    })
    .expect("exact frontier admits")
}

fn coverage() -> SourceCoverageReport {
    let axes = SourceCoverageDimension::ALL
        .into_iter()
        .enumerate()
        .map(|(index, dimension)| {
            let subject = reference(&format!("coverage-{index}"));
            (
                dimension,
                SourceCoveragePartition::admit(
                    BTreeSet::from([subject.clone()]),
                    BTreeSet::from([subject]),
                    BTreeMap::new(),
                )
                .expect("complete coverage partition"),
            )
        })
        .collect();
    SourceCoverageReport::admit(axes).expect("all five axes are present")
}

fn coverage_with_gap(gap: ImmutableRecordReferenceV1) -> SourceCoverageReport {
    let axes = SourceCoverageDimension::ALL
        .into_iter()
        .enumerate()
        .map(|(index, dimension)| {
            let observed = reference(&format!("coverage-{index}"));
            let mut declared = BTreeSet::from([observed.clone()]);
            let mut gaps = BTreeMap::new();
            if dimension == SourceCoverageDimension::Language {
                declared.insert(gap.clone());
                gaps.insert(gap.clone(), SourceCoverageGapKind::Unsupported);
            }
            (
                dimension,
                SourceCoveragePartition::admit(declared, BTreeSet::from([observed]), gaps)
                    .expect("complete coverage partition"),
            )
        })
        .collect();
    SourceCoverageReport::admit(axes).expect("all five axes are present")
}

fn coordinate(scope: &SourceIndexScope, generation: &str) -> ProviderIndexCoordinateV1 {
    ProviderIndexCoordinateV1 {
        provider_index_owner: scope.provider_index_owner.clone(),
        provider_index_generation: ProviderIndexGenerationV1::parse(generation)
            .expect("valid provider generation"),
        repository_source_frontier: frontier(scope.repository_id.clone()).coordinate().clone(),
    }
}

fn node(id: &str, kind: SourceNodeKind) -> SourceGraphNode {
    SourceGraphNode {
        reference: reference(id),
        kind,
        location: None,
    }
}

fn located_node(id: &str, path: &str) -> SourceGraphNode {
    SourceGraphNode {
        reference: reference(id),
        kind: SourceNodeKind::Symbol,
        location: Some(SourceLocation {
            path: OwnedConfinedRelativePath::parse(path).expect("confined source path"),
            start_byte: 0,
            end_byte: 1,
        }),
    }
}

fn edge(id: &str, cause: &str, dependent: &str) -> SourceGraphEdge {
    SourceGraphEdge {
        reference: reference(id),
        cause: reference(cause),
        dependent: reference(dependent),
        kind: SourceDependencyKind::ReferencedBy,
        provenance: SourceEdgeProvenance {
            kind: SourceEdgeProvenanceKind::Resolved,
            basis: reference(&format!("{id}-basis")),
            confidence: SourceConfidence::Established(reference(&format!("{id}-confidence"))),
        },
    }
}

fn heuristic_edge(id: &str, cause: &str, dependent: &str) -> SourceGraphEdge {
    SourceGraphEdge {
        reference: reference(id),
        cause: reference(cause),
        dependent: reference(dependent),
        kind: SourceDependencyKind::ReferencedBy,
        provenance: SourceEdgeProvenance {
            kind: SourceEdgeProvenanceKind::Heuristic,
            basis: reference(&format!("{id}-basis")),
            confidence: SourceConfidence::Estimated(reference(&format!("{id}-confidence"))),
        },
    }
}

fn graph_candidate(
    scope: SourceIndexScope,
    coordinate: ProviderIndexCoordinateV1,
    nodes: Vec<SourceGraphNode>,
    edges: Vec<SourceGraphEdge>,
) -> SourceIndexCandidate {
    SourceIndexCandidate {
        frontier: frontier(scope.repository_id.clone()),
        scope,
        coordinate,
        coverage: coverage(),
        nodes,
        edges,
    }
}

fn policy() -> AdaptiveNodeSetPolicy {
    AdaptiveNodeSetPolicy::new(2, 5_000).expect("valid set policy")
}

fn operation() -> OperationObligation {
    let operation = resource("inspect-source");
    let requirement =
        CapabilityRequirementIdV1::parse("source-graph").expect("valid capability requirement");
    OperationObligation {
        operation: operation.clone(),
        state: OperationObligationState::CapabilityBearing {
            selection: OperationSelection::Internal,
            capabilities: vec![CapabilityObligation {
                subject: ObligationSubject {
                    operation,
                    requirement,
                },
                capability: CapabilityIdV1::parse("source.graph").expect("valid capability"),
                effect: EffectClass::Read,
                procedure: ProviderProcedureObligation::CapabilityOnly,
                disposition: CapabilityObligationDisposition::Bound(CapabilityBinding {
                    effect_class: EffectClass::Read,
                    approval: Approval::None,
                    tool_contract: None,
                    implementation: CapabilityProviderBinding::NativeRust {
                        target: "source-intelligence".to_owned(),
                    },
                    note: None,
                }),
                failure_posture: FailurePosture::FailClosed,
            }],
        },
    }
}

fn admitted_query(
    scope: SourceIndexScope,
    index: ProviderIndexCoordinateV1,
    question: SourceStructuralQuestion,
    bounds: SourceQueryBounds,
) -> AdmittedSourceQuery {
    let obligation = reference("source-obligation");
    admit_source_query(
        &operation(),
        SourceQueryRequest {
            index,
            scope,
            compiled_generation: CompiledGenerationDigest::hash_canonical(&"generation-1")
                .expect("valid compiled generation"),
            operation_lease: OperationLeaseIdV1::parse("operation-lease-1")
                .expect("valid operation lease"),
            reason: SourceQueryReason {
                obligation: obligation.clone(),
                operation: resource("inspect-source"),
                requirement: CapabilityRequirementIdV1::parse("source-graph")
                    .expect("valid requirement"),
                effect: EffectClass::Read,
                causal_path: vec![obligation],
            },
            question,
            bounds,
        },
    )
    .expect("query admits")
}

fn graph_declarations() -> (Vec<SourceGraphNode>, Vec<SourceGraphEdge>) {
    (
        vec![
            node("node-a", SourceNodeKind::Symbol),
            node("node-b", SourceNodeKind::Symbol),
            node("node-mid-1", SourceNodeKind::Symbol),
            node("node-mid-2", SourceNodeKind::Symbol),
            node("node-test", SourceNodeKind::Test),
        ],
        vec![
            edge("edge-a-mid-1", "node-a", "node-mid-1"),
            edge("edge-mid-1-mid-2", "node-mid-1", "node-mid-2"),
            edge("edge-mid-2-test", "node-mid-2", "node-test"),
            edge("edge-b-test", "node-b", "node-test"),
        ],
    )
}

fn bounds() -> SourceQueryBounds {
    SourceQueryBounds {
        nodes: 64,
        reason_paths: 64,
        path_edges: 64,
        affected_tests: 64,
        output_bytes: MAX_SOURCE_QUERY_OUTPUT_BYTES,
    }
}

#[test]
fn five_axis_coverage_is_an_exact_partition_not_a_percentage() {
    let report = coverage();
    assert_eq!(report.axes().count(), 5);
    assert!(report.axes().all(|(_, axis)| {
        axis.denominator() == 1 && axis.observed().len() == 1 && axis.gaps().is_empty()
    }));

    let declared = BTreeSet::from([reference("declared-a"), reference("declared-b")]);
    assert!(matches!(
        SourceCoveragePartition::admit(
            declared,
            BTreeSet::from([reference("declared-a")]),
            BTreeMap::new(),
        ),
        Err(SourceCoverageError::IncompletePartition)
    ));
}

#[test]
fn exact_frontier_rejects_constituent_drift() {
    let admitted =
        frontier(LogicalRepositoryIdV1::parse("repository-1").expect("valid repository"));
    let mut drifted = admitted.candidate().clone();
    drifted.tracked[0].content_bytes = Some(13);
    assert!(matches!(
        admit_source_frontier(drifted),
        Err(SourceFrontierError::DigestMismatch {
            axis: "tracked delta"
        })
    ));
}

#[test]
fn exact_frontier_rejects_members_beneath_excluded_roots() {
    let admitted =
        frontier(LogicalRepositoryIdV1::parse("repository-1").expect("valid repository"));
    let mut excluded = admitted.candidate().clone();
    excluded.policy.excluded_roots =
        BTreeSet::from([
            OwnedConfinedRelativePath::parse("src/generated").expect("excluded source root")
        ]);
    excluded.generated = vec![SourceByteMember {
        path: OwnedConfinedRelativePath::parse("src/generated/model.rs").expect("generated path"),
        content_sha256: ArtifactDigest::hash_bytes(b"generated"),
        content_bytes: 9,
    }];
    excluded.coordinate.untracked_inventory_sha256 = digest(&UntrackedPreimage {
        untracked: &excluded.untracked,
        generated: &excluded.generated,
    });
    excluded.coordinate.dependency_frontier_sha256 = digest(&DependencyPreimage {
        source_roots: &excluded.source_roots,
        submodules: &excluded.submodules,
        policy: &excluded.policy,
    });

    assert!(matches!(
        admit_source_frontier(excluded),
        Err(SourceFrontierError::OutsideSourceRoots(path))
            if path.as_str() == "src/generated/model.rs"
    ));
}

#[test]
fn building_generation_rejects_graph_locations_outside_its_frontier() {
    let selected_scope = scope("provider-1");
    let selected_coordinate = coordinate(&selected_scope, "generation-1");
    let (administrator, _) = source_intelligence(policy());
    let candidate = graph_candidate(
        selected_scope,
        selected_coordinate,
        vec![located_node("outside", "tests/outside.rs")],
        Vec::new(),
    );

    assert!(matches!(
        administrator.begin(candidate),
        Err(SourceIndexLifecycleError::GraphDeclaration(
            SourceGraphDeclarationError::LocationOutsideFrontier(path)
        )) if path.as_str() == "tests/outside.rs"
    ));
}

#[test]
fn collision_cannot_replace_ready_bytes_and_self_supersession_is_rejected() {
    let selected_scope = scope("provider-1");
    let selected_coordinate = coordinate(&selected_scope, "generation-1");
    let (nodes, edges) = graph_declarations();
    let (administrator, queries) = source_intelligence(policy());
    let first = administrator
        .begin(graph_candidate(
            selected_scope.clone(),
            selected_coordinate.clone(),
            nodes,
            edges,
        ))
        .expect("first candidate begins");
    let colliding = administrator
        .begin(graph_candidate(
            selected_scope.clone(),
            selected_coordinate.clone(),
            vec![node("replacement-only", SourceNodeKind::Symbol)],
            Vec::new(),
        ))
        .expect("concurrent colliding candidate may begin before publication");
    let prepublication_query = admitted_query(
        selected_scope.clone(),
        selected_coordinate.clone(),
        SourceStructuralQuestion::Explore {
            origins: vec![reference("node-a")],
            target: None,
        },
        bounds(),
    );
    assert!(matches!(
        queries.query(&prepublication_query),
        Err(SourceQueryError::GenerationNotActive)
    ));
    let ready = administrator
        .publish(first)
        .expect("first candidate publishes");
    administrator
        .transition(SourceIndexTransition::Activate {
            scope: selected_scope.clone(),
            coordinate_digest: ready.coordinate_digest.clone(),
            evidence: reference("activation-1"),
        })
        .expect("first generation activates");
    {
        let registry = administrator
            .registry
            .read()
            .expect("source-index registry remains readable");
        let Some(super::lifecycle::SourceIndexEntry::Retained {
            selection_evidence, ..
        }) = registry.generations.get(&ready.coordinate_digest)
        else {
            panic!("activated generation remains retained")
        };
        assert_eq!(
            selection_evidence.as_ref(),
            Some(&reference("activation-1"))
        );
    }
    assert!(matches!(
        administrator.publish(colliding),
        Err(SourceIndexLifecycleError::GenerationCollision)
    ));

    let query = admitted_query(
        selected_scope.clone(),
        selected_coordinate,
        SourceStructuralQuestion::Explore {
            origins: vec![reference("node-a")],
            target: Some(reference("node-test")),
        },
        bounds(),
    );
    let result = queries
        .query(&query)
        .expect("original graph remains queryable");
    assert_eq!(
        result.reason_paths[0].nodes.first(),
        Some(&reference("node-a"))
    );
    assert!(matches!(
        administrator.transition(SourceIndexTransition::Supersede {
            scope: selected_scope,
            predecessor: ready.coordinate_digest.clone(),
            successor: ready.coordinate_digest,
            evidence: reference("invalid-self-supersession"),
        }),
        Err(SourceIndexLifecycleError::InvalidTransition)
    ));
    assert!(queries.query(&query).is_ok());
}

#[test]
fn removal_keeps_a_generation_tombstone_and_direct_activation_cannot_replace_an_active_index() {
    let selected_scope = scope("provider-1");
    let first_coordinate = coordinate(&selected_scope, "generation-1");
    let second_coordinate = coordinate(&selected_scope, "generation-2");
    let (nodes, edges) = graph_declarations();
    let (administrator, _) = source_intelligence(policy());
    let first = administrator
        .publish(
            administrator
                .begin(graph_candidate(
                    selected_scope.clone(),
                    first_coordinate.clone(),
                    nodes.clone(),
                    edges.clone(),
                ))
                .expect("first candidate begins"),
        )
        .expect("first candidate publishes");
    let second = administrator
        .publish(
            administrator
                .begin(graph_candidate(
                    selected_scope.clone(),
                    second_coordinate,
                    nodes,
                    edges,
                ))
                .expect("second candidate begins"),
        )
        .expect("second candidate publishes");
    administrator
        .transition(SourceIndexTransition::Activate {
            scope: selected_scope.clone(),
            coordinate_digest: first.coordinate_digest.clone(),
            evidence: reference("activation-1"),
        })
        .expect("first activates");
    assert!(matches!(
        administrator.transition(SourceIndexTransition::Activate {
            scope: selected_scope.clone(),
            coordinate_digest: second.coordinate_digest.clone(),
            evidence: reference("unfenced-replacement"),
        }),
        Err(SourceIndexLifecycleError::InvalidTransition)
    ));
    administrator
        .transition(SourceIndexTransition::Supersede {
            scope: selected_scope.clone(),
            predecessor: first.coordinate_digest.clone(),
            successor: second.coordinate_digest.clone(),
            evidence: reference("supersession-1"),
        })
        .expect("fenced successor activates");
    administrator
        .transition(SourceIndexTransition::Rollback {
            scope: selected_scope.clone(),
            coordinate_digest: first.coordinate_digest.clone(),
            evidence: reference("rollback-1"),
        })
        .expect("rollback restores the exact predecessor");
    assert_eq!(
        administrator
            .state(&first.coordinate_digest)
            .expect("lifecycle state remains readable"),
        Some(SourceIndexLifecycleState::Ready)
    );
    assert_eq!(
        administrator
            .state(&second.coordinate_digest)
            .expect("lifecycle state remains readable"),
        Some(SourceIndexLifecycleState::Superseded)
    );
    administrator
        .transition(SourceIndexTransition::Invalidate {
            coordinate_digest: first.coordinate_digest.clone(),
            evidence: reference("invalidation-1"),
        })
        .expect("active generation invalidates");
    administrator
        .transition(SourceIndexTransition::Remove {
            coordinate_digest: first.coordinate_digest.clone(),
            evidence: reference("removal-1"),
        })
        .expect("invalidated generation removes");
    assert!(matches!(
        administrator.transition(SourceIndexTransition::Remove {
            coordinate_digest: first.coordinate_digest.clone(),
            evidence: reference("removal-2"),
        }),
        Err(SourceIndexLifecycleError::RemovedGeneration)
    ));
    assert_eq!(
        administrator
            .state(&first.coordinate_digest)
            .expect("removed state remains readable"),
        Some(SourceIndexLifecycleState::Removed)
    );
    assert!(matches!(
        administrator.begin(graph_candidate(
            selected_scope,
            first_coordinate,
            vec![node("replacement-only", SourceNodeKind::Symbol)],
            Vec::new(),
        )),
        Err(SourceIndexLifecycleError::GenerationCollision)
    ));
}

#[test]
fn affected_test_selection_uses_the_globally_shortest_canonical_reason() {
    let selected_scope = scope("provider-1");
    let selected_coordinate = coordinate(&selected_scope, "generation-1");
    let (nodes, edges) = graph_declarations();
    let (administrator, queries) = source_intelligence(policy());
    let ready = administrator
        .publish(
            administrator
                .begin(graph_candidate(
                    selected_scope.clone(),
                    selected_coordinate.clone(),
                    nodes,
                    edges,
                ))
                .expect("candidate begins"),
        )
        .expect("candidate publishes");
    administrator
        .transition(SourceIndexTransition::Activate {
            scope: selected_scope.clone(),
            coordinate_digest: ready.coordinate_digest,
            evidence: reference("activation-1"),
        })
        .expect("generation activates");
    let mut query_bounds = bounds();
    query_bounds.nodes = 1;
    query_bounds.reason_paths = 0;
    let query = admitted_query(
        selected_scope,
        selected_coordinate,
        SourceStructuralQuestion::CandidateAffectedTests {
            changed: vec![reference("node-a"), reference("node-b")],
        },
        query_bounds,
    );
    let result = queries.query(&query).expect("affected-test query succeeds");
    let [candidate] = result.affected_tests.as_slice() else {
        panic!("one affected test is returned")
    };
    assert_eq!(
        candidate.reason.nodes,
        vec![reference("node-b"), reference("node-test")]
    );
    assert_eq!(candidate.reason.edges, vec![reference("edge-b-test")]);
    assert!(result.reason_paths.is_empty());
    let evidence = result.evidence_references();
    assert!(evidence.contains(&reference("node-b")));
    assert!(evidence.contains(&reference("node-test")));
    assert!(evidence.contains(&reference("edge-b-test")));
    assert_eq!(result.cost.indexed_nodes, 5);
    assert_eq!(result.cost.indexed_edges, 4);
    assert!(result.cost.output_bytes <= query.request().bounds.output_bytes);
}

#[test]
fn pathless_exploration_discloses_heuristic_edges_in_its_reachability_proof() {
    let selected_scope = scope("provider-1");
    let selected_coordinate = coordinate(&selected_scope, "generation-1");
    let nodes = vec![
        node("node-origin", SourceNodeKind::Symbol),
        node("node-dependent", SourceNodeKind::Symbol),
    ];
    let edges = vec![heuristic_edge(
        "edge-heuristic",
        "node-origin",
        "node-dependent",
    )];
    let (administrator, queries) = source_intelligence(policy());
    let ready = administrator
        .publish(
            administrator
                .begin(graph_candidate(
                    selected_scope.clone(),
                    selected_coordinate.clone(),
                    nodes,
                    edges,
                ))
                .expect("candidate begins"),
        )
        .expect("candidate publishes");
    administrator
        .transition(SourceIndexTransition::Activate {
            scope: selected_scope.clone(),
            coordinate_digest: ready.coordinate_digest,
            evidence: reference("activation-1"),
        })
        .expect("generation activates");
    let query = admitted_query(
        selected_scope,
        selected_coordinate,
        SourceStructuralQuestion::Explore {
            origins: vec![reference("node-origin")],
            target: None,
        },
        bounds(),
    );
    let result = queries
        .query(&query)
        .expect("pathless exploration succeeds");
    assert!(result.reason_paths.is_empty());
    assert!(result
        .limitations
        .contains(&SourceResultLimitation::HeuristicEdge {
            edge: reference("edge-heuristic"),
            basis: reference("edge-heuristic-basis"),
        }));
}

#[test]
fn truncation_counts_every_omitted_test_and_retains_limitation_evidence() {
    let selected_scope = scope("provider-1");
    let selected_coordinate = coordinate(&selected_scope, "generation-1");
    let nodes = vec![
        node("node-origin", SourceNodeKind::Symbol),
        node("node-mid-1", SourceNodeKind::Symbol),
        node("node-mid-2", SourceNodeKind::Symbol),
        node("node-test-1", SourceNodeKind::Test),
        node("node-test-2", SourceNodeKind::Test),
    ];
    let edges = vec![
        edge("edge-origin-mid-1", "node-origin", "node-mid-1"),
        edge("edge-mid-1-test", "node-mid-1", "node-test-1"),
        edge("edge-origin-mid-2", "node-origin", "node-mid-2"),
        edge("edge-mid-2-test", "node-mid-2", "node-test-2"),
    ];
    let gap = reference("unsupported-language");
    let mut candidate = graph_candidate(
        selected_scope.clone(),
        selected_coordinate.clone(),
        nodes,
        edges,
    );
    candidate.coverage = coverage_with_gap(gap.clone());
    let (administrator, queries) = source_intelligence(policy());
    let ready = administrator
        .publish(administrator.begin(candidate).expect("candidate begins"))
        .expect("candidate publishes");
    administrator
        .transition(SourceIndexTransition::Activate {
            scope: selected_scope.clone(),
            coordinate_digest: ready.coordinate_digest,
            evidence: reference("activation-1"),
        })
        .expect("generation activates");
    let mut query_bounds = bounds();
    query_bounds.path_edges = 1;
    let query = admitted_query(
        selected_scope,
        selected_coordinate,
        SourceStructuralQuestion::CandidateAffectedTests {
            changed: vec![reference("node-origin")],
        },
        query_bounds,
    );
    let result = queries.query(&query).expect("bounded query succeeds");
    assert!(result.affected_tests.is_empty());
    assert!(result
        .limitations
        .contains(&SourceResultLimitation::Truncated {
            collection: SourceResultCollection::AffectedTests,
            omitted: 2,
        }));
    assert!(result.evidence_references().contains(&gap));
}

#[test]
fn actual_use_and_invalidation_lineage_do_not_infer_one_another() {
    let selected_scope = scope("provider-1");
    let selected_coordinate = coordinate(&selected_scope, "generation-1");
    let (nodes, edges) = graph_declarations();
    let (administrator, queries) = source_intelligence(policy());
    let ready = administrator
        .publish(
            administrator
                .begin(graph_candidate(
                    selected_scope.clone(),
                    selected_coordinate.clone(),
                    nodes,
                    edges,
                ))
                .expect("candidate begins"),
        )
        .expect("candidate publishes");
    administrator
        .transition(SourceIndexTransition::Activate {
            scope: selected_scope.clone(),
            coordinate_digest: ready.coordinate_digest,
            evidence: reference("activation-1"),
        })
        .expect("generation activates");
    let query = admitted_query(
        selected_scope.clone(),
        selected_coordinate.clone(),
        SourceStructuralQuestion::CandidateAffectedTests {
            changed: vec![reference("node-b")],
        },
        bounds(),
    );
    let result = queries.query(&query).expect("query succeeds");
    let result_record = ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse("source-result-1").expect("valid result ref"),
        record_sha256: result.result_digest.clone(),
    };
    let mut tampered_result = result.clone();
    tampered_result.scope.principal_id =
        ResidentPrincipalIdV1::parse("principal-2").expect("valid principal");
    assert!(matches!(
        admit_source_result_record(&tampered_result, result_record.clone()),
        Err(SourceEvidenceUseError::ResultMismatch)
    ));
    let admitted_result = admit_source_result_record(&result, result_record.clone())
        .expect("result record binds exact result digest");
    let available = result.evidence_references();
    let used = BTreeSet::from([reference("node-test")]);
    let ignored = available
        .difference(&used)
        .cloned()
        .collect::<BTreeSet<_>>();
    let source_use = SourceEvidenceUse {
        use_record: reference("source-use-1"),
        result_record: result_record.clone(),
        result_digest: result.result_digest.clone(),
        index: selected_coordinate.clone(),
        scope: selected_scope.clone(),
        context: SourceUseContext {
            workspace: reference("workspace-record"),
            module_release: reference("module-release"),
            import: reference("import-record"),
            surface_grant: reference("surface-grant"),
        },
        compiled_generation: query.request().compiled_generation.clone(),
        operation: query.request().reason.operation.clone(),
        operation_lease: query.request().operation_lease.clone(),
        invocation: reference("invocation-1"),
        used,
        rejected: BTreeSet::new(),
        ignored,
        purpose: reference("purpose-1"),
        target: reference("target-1"),
        dependent: Some(reference("dependent-1")),
        settlement: Some(reference("settlement-1")),
        native_verification: Some(reference("native-verification-1")),
        occurred_at: UnixTimeV1::new(10).expect("portable occurrence time"),
        expires_at: UnixTimeV1::new(20).expect("portable expiry time"),
    };
    let admitted_use =
        admit_source_evidence_use(&query, &result, source_use).expect("exact actual use admits");

    let index_digest = digest(&selected_coordinate);
    let mut result_only = SourceEvidenceLineageBuilder::new(policy());
    result_only
        .add_result(&admitted_result, reference("result-lineage"))
        .expect("result lineage adds");
    let result_only = result_only.seal().expect("result-only lineage seals");
    let closure = result_only
        .invalidate(SourceEvidenceInvalidation {
            scope: selected_scope.clone(),
            roots: BTreeSet::from([SourceLineageNode::Index(index_digest.clone())]),
            evidence: reference("index-invalidation"),
        })
        .expect("index invalidation propagates");
    assert!(closure
        .affected
        .contains(&SourceLineageNode::Result(result_record.clone())));
    assert!(!closure
        .affected
        .contains(&SourceLineageNode::Use(reference("source-use-1"))));

    let mut complete = SourceEvidenceLineageBuilder::new(policy());
    complete
        .add_result(&admitted_result, reference("result-lineage"))
        .expect("result lineage adds");
    complete
        .add_use(&admitted_use, reference("use-lineage"))
        .expect("actual-use lineage adds");
    let complete = complete.seal().expect("complete lineage seals");
    let closure = complete
        .invalidate(SourceEvidenceInvalidation {
            scope: selected_scope,
            roots: BTreeSet::from([
                SourceLineageNode::Index(index_digest),
                SourceLineageNode::Result(result_record.clone()),
            ]),
            evidence: reference("index-invalidation"),
        })
        .expect("index invalidation reaches actual use");
    for affected in [
        SourceLineageNode::Result(result_record),
        SourceLineageNode::Use(reference("source-use-1")),
        SourceLineageNode::Dependent(reference("dependent-1")),
        SourceLineageNode::Settlement(reference("settlement-1")),
        SourceLineageNode::NativeVerification(reference("native-verification-1")),
    ] {
        assert!(closure.affected.contains(&affected));
    }
    assert_eq!(closure.reason_paths.len(), closure.affected.len());
}

#[test]
fn source_lineage_rejects_cross_scope_results_and_keeps_invalidation_isolated() {
    let first_scope = scope("provider-1");
    let coordinate = coordinate(&first_scope, "generation-1");
    let mut second_principal = first_scope.clone();
    second_principal.principal_id =
        ResidentPrincipalIdV1::parse("principal-2").expect("valid principal");
    let mut second_provider_pin = first_scope.clone();
    second_provider_pin.provider_pin = reference("provider-pin-2");

    let (nodes, edges) = graph_declarations();
    let query_result = |selected_scope: SourceIndexScope, result_id: &str| {
        let (administrator, queries) = source_intelligence(policy());
        let ready = administrator
            .publish(
                administrator
                    .begin(graph_candidate(
                        selected_scope.clone(),
                        coordinate.clone(),
                        nodes.clone(),
                        edges.clone(),
                    ))
                    .expect("candidate begins"),
            )
            .expect("candidate publishes");
        administrator
            .transition(SourceIndexTransition::Activate {
                scope: selected_scope.clone(),
                coordinate_digest: ready.coordinate_digest,
                evidence: reference(&format!("{result_id}-activation")),
            })
            .expect("generation activates");
        let query = admitted_query(
            selected_scope,
            coordinate.clone(),
            SourceStructuralQuestion::Explore {
                origins: vec![reference("node-a")],
                target: None,
            },
            bounds(),
        );
        let result = queries.query(&query).expect("query succeeds");
        let record = ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse(result_id).expect("valid result ref"),
            record_sha256: result.result_digest.clone(),
        };
        admit_source_result_record(&result, record).expect("result record admits")
    };

    let first = query_result(first_scope.clone(), "source-result-first");
    let other_principal = query_result(second_principal.clone(), "source-result-other-principal");
    let other_provider_pin = query_result(second_provider_pin, "source-result-other-provider-pin");
    let mut lineage = SourceEvidenceLineageBuilder::new(policy());
    lineage
        .add_result(&first, reference("first-lineage"))
        .expect("first scope fixes the lineage");
    assert!(matches!(
        lineage.add_result(&other_principal, reference("other-principal-lineage")),
        Err(SourceLineageError::ScopeMismatch)
    ));
    assert!(matches!(
        lineage.add_result(&other_provider_pin, reference("other-provider-pin-lineage")),
        Err(SourceLineageError::ScopeMismatch)
    ));
    let lineage = lineage.seal().expect("single-scope lineage seals");
    let closure = lineage
        .invalidate(SourceEvidenceInvalidation {
            scope: first_scope.clone(),
            roots: BTreeSet::from([SourceLineageNode::Index(digest(&coordinate))]),
            evidence: reference("first-invalidation"),
        })
        .expect("first-scope invalidation projects");
    assert!(closure
        .affected
        .contains(&SourceLineageNode::Result(first.record().clone())));
    assert!(!closure
        .affected
        .contains(&SourceLineageNode::Result(other_principal.record().clone())));
    assert!(!closure.affected.contains(&SourceLineageNode::Result(
        other_provider_pin.record().clone()
    )));

    let mut other_lineage = SourceEvidenceLineageBuilder::new(policy());
    other_lineage
        .add_result(&other_principal, reference("other-principal-lineage"))
        .expect("second scope fixes its own lineage");
    let other_lineage = other_lineage.seal().expect("second lineage seals");
    assert!(matches!(
        other_lineage.invalidate(SourceEvidenceInvalidation {
            scope: first_scope,
            roots: BTreeSet::from([SourceLineageNode::Index(digest(&coordinate))]),
            evidence: reference("first-scope-invalidation"),
        }),
        Err(SourceLineageError::ScopeMismatch)
    ));
}

#[test]
fn unbound_capability_and_substituted_obligation_path_cannot_admit_a_query() {
    let selected_scope = scope("provider-1");
    let selected_coordinate = coordinate(&selected_scope, "generation-1");
    let obligation = reference("source-obligation");
    let request = SourceQueryRequest {
        index: selected_coordinate,
        scope: selected_scope,
        compiled_generation: CompiledGenerationDigest::hash_canonical(&"generation-1")
            .expect("valid compiled generation"),
        operation_lease: OperationLeaseIdV1::parse("operation-lease-1")
            .expect("valid operation lease"),
        reason: SourceQueryReason {
            obligation: obligation.clone(),
            operation: resource("inspect-source"),
            requirement: CapabilityRequirementIdV1::parse("source-graph")
                .expect("valid requirement"),
            effect: EffectClass::Read,
            causal_path: vec![reference("substituted-obligation")],
        },
        question: SourceStructuralQuestion::Explore {
            origins: vec![reference("node-a")],
            target: None,
        },
        bounds: bounds(),
    };
    assert!(matches!(
        admit_source_query(&operation(), request.clone()),
        Err(SourceQueryAdmissionError::NonCanonicalBoundedRelation {
            relation: "causal path"
        })
    ));

    let mut unavailable = operation();
    let OperationObligationState::CapabilityBearing { capabilities, .. } = &mut unavailable.state
    else {
        unreachable!("fixture is capability-bearing")
    };
    capabilities[0].disposition = CapabilityObligationDisposition::Blocked {
        reason: crate::semantic::ProviderUnavailability::MissingBinding,
    };
    let mut request = request;
    request.reason.causal_path = vec![obligation];
    assert!(matches!(
        admit_source_query(&unavailable, request.clone()),
        Err(SourceQueryAdmissionError::NoStructuralObligation)
    ));

    let mut substituted_subject = operation();
    let OperationObligationState::CapabilityBearing { capabilities, .. } =
        &mut substituted_subject.state
    else {
        unreachable!("fixture is capability-bearing")
    };
    capabilities[0].subject.operation = resource("another-operation");
    assert!(matches!(
        admit_source_query(&substituted_subject, request.clone()),
        Err(SourceQueryAdmissionError::NoStructuralObligation)
    ));

    let mut malformed_binding = operation();
    let OperationObligationState::CapabilityBearing { capabilities, .. } =
        &mut malformed_binding.state
    else {
        unreachable!("fixture is capability-bearing")
    };
    let CapabilityObligationDisposition::Bound(binding) = &mut capabilities[0].disposition else {
        unreachable!("fixture capability is bound")
    };
    binding.effect_class = EffectClass::Write;
    assert!(matches!(
        admit_source_query(&malformed_binding, request),
        Err(SourceQueryAdmissionError::NoStructuralObligation)
    ));
}
