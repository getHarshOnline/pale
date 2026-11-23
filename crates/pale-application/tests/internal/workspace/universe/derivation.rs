//! Law tests for the ephemeral causal-proof index.

use super::*;

const NODE_COUNT: usize = 5;

#[test]
fn comparison_subject_pages_stop_before_the_identity_byte_budget_without_skipping() {
    use crate::workspace::comparison_subjects::{
        subject_identity_bytes, MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES,
    };
    use pale_core::records::{
        WorkspaceDerivationOutputV1, WorkspaceExportId, WorkspaceNamespaceId,
    };

    fn full_id(prefix: &str) -> String {
        format!("{prefix}{}", "a".repeat(255 - prefix.len()))
    }

    let largest_subject = WorkspaceDerivationSubjectV1::Import {
        importer_module: WorkspaceModuleId::parse(full_id("i")).unwrap(),
        upstream_module: WorkspaceModuleId::parse(full_id("u")).unwrap(),
        upstream_release: WorkspaceReleaseId::parse(full_id("r")).unwrap(),
        export_id: WorkspaceExportId::parse(full_id("e")).unwrap(),
        source_namespace: WorkspaceNamespaceId::parse(full_id("s")).unwrap(),
        local_namespace: WorkspaceNamespaceId::parse(full_id("l")).unwrap(),
    };
    assert_eq!(subject_identity_bytes(&largest_subject), 6 * 255);
    assert!(subject_identity_bytes(&largest_subject) <= MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES);

    let mut expected = (0..32)
        .map(|index| WorkspaceDerivationSubjectV1::Workspace {
            workspace_id: WorkspaceId::parse(full_id(&format!("w{index:02}"))).unwrap(),
        })
        .collect::<BTreeSet<_>>();
    expected.insert(largest_subject);

    let facts = expected
        .iter()
        .cloned()
        .map(|subject| {
            let node_digest =
                WorkspaceDerivationNodeDigest::hash_canonical(&("identity-budget", &subject))
                    .unwrap();
            let output = WorkspaceDerivationOutputV1::ResponsibilityClosure {
                closure_digest: ResponsibilityDerivationClosureDigest::hash_canonical(&(
                    "identity-budget",
                    &subject,
                ))
                .unwrap(),
            };
            (subject, (node_digest, output))
        })
        .collect::<BTreeMap<_, _>>();
    let frontier = WorkspaceDerivationFrontier {
        sources: Vec::new(),
        nodes: Vec::new(),
        facts,
        resolution_digest: WorkspaceResolutionLockDigest::hash_canonical(&()).unwrap(),
        universe_digest: WorkspaceUniverseLockDigest::hash_canonical(&()).unwrap(),
        responsibility_digest: ResponsibilityDerivationClosureDigest::hash_canonical(&()).unwrap(),
        organizational_digest: OrganizationalTranslationClosureDigest::hash_canonical(&()).unwrap(),
        composition_digest: ExperienceCompositionSourceDigest::hash_canonical(&()).unwrap(),
    };

    let mut after = None;
    let mut observed = Vec::new();
    let mut pages = 0;
    loop {
        let page = comparison_subject_page(&frontier, &frontier, after.as_ref(), u16::MAX);
        let subjects = page.subjects().cloned().collect::<Vec<_>>();
        assert!(!subjects.is_empty());
        assert_eq!(
            page.subject_identity_bytes(),
            subjects.iter().map(subject_identity_bytes).sum::<usize>()
        );
        assert!(page.subject_identity_bytes() <= MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES);
        assert!(subjects.windows(2).all(|pair| pair[0] < pair[1]));
        if let Some(previous) = &after {
            assert!(subjects.iter().all(|subject| subject > previous));
        }
        observed.extend(subjects);
        pages += 1;
        after = page.next_after().cloned();
        if after.is_none() {
            break;
        }
    }

    assert!(
        pages > 1,
        "the byte ceiling, not the row ceiling, must split this fixture"
    );
    assert_eq!(observed, expected.into_iter().collect::<Vec<_>>());
}

#[test]
fn direct_dependency_pages_stop_before_the_copied_string_budget_without_skipping() {
    use crate::workspace::MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES;
    use pale_core::paths::{OwnedConfinedRelativePath, MAX_CONFINED_RELATIVE_PATH_BYTES};
    use pale_core::records::{WorkspaceDerivationSourceId, WorkspaceSourceContentDigest};

    fn full_id(prefix: &str) -> String {
        format!("{prefix}{}", "a".repeat(255 - prefix.len()))
    }

    let source_path =
        OwnedConfinedRelativePath::parse("a".repeat(MAX_CONFINED_RELATIVE_PATH_BYTES)).unwrap();
    let mut nodes = (0..4)
        .map(|index| {
            let source_id = WorkspaceDerivationSourceId::parse(full_id(&format!("s{index}")))
                .expect("test source identity is valid");
            WorkspaceDerivationNodeV1::seal(
                WorkspaceDerivationOriginV1::Source {
                    source_id: source_id.clone(),
                },
                WorkspaceDerivationRuleV1::SourceAdmission,
                WorkspaceDerivationSubjectV1::Source {
                    source_id: source_id.clone(),
                },
                Output::SourceContent {
                    source_path: source_path.clone(),
                    source_digest: WorkspaceSourceContentDigest::hash_bytes(
                        source_id.as_str().as_bytes(),
                    ),
                    byte_len: 0,
                },
                Vec::new(),
            )
            .expect("maximal source-content step is individually valid")
        })
        .collect::<Vec<_>>();
    let root = WorkspaceDerivationNodeV1::seal(
        WorkspaceDerivationOriginV1::Compilation,
        WorkspaceDerivationRuleV1::SemanticIrSealing,
        WorkspaceDerivationSubjectV1::Workspace {
            workspace_id: WorkspaceId::parse("copied-string-budget").unwrap(),
        },
        Output::SemanticIr {
            workspace_semantic_ir_digest: WorkspaceSemanticIrDigest::hash_canonical(&(
                "copied-string-budget",
                &nodes,
            ))
            .unwrap(),
        },
        nodes.iter().map(|node| node.node_digest.clone()).collect(),
    )
    .expect("budget test root is valid");
    let subject = root.subject.clone();
    let expected = root.inputs.clone();
    nodes.push(root);
    nodes.sort_by(|left, right| left.node_digest.cmp(&right.node_digest));
    let frontier = WorkspaceDerivationFrontier {
        facts: nodes
            .iter()
            .map(|node| {
                (
                    node.subject.clone(),
                    (node.node_digest.clone(), node.output.clone()),
                )
            })
            .collect(),
        nodes,
        sources: Vec::new(),
        resolution_digest: WorkspaceResolutionLockDigest::hash_canonical(&()).unwrap(),
        universe_digest: WorkspaceUniverseLockDigest::hash_canonical(&()).unwrap(),
        responsibility_digest: ResponsibilityDerivationClosureDigest::hash_canonical(&()).unwrap(),
        organizational_digest: OrganizationalTranslationClosureDigest::hash_canonical(&()).unwrap(),
        composition_digest: ExperienceCompositionSourceDigest::hash_canonical(&()).unwrap(),
    };

    let mut offset = 0;
    let mut observed = Vec::new();
    let mut pages = 0;
    loop {
        let page = direct_dependency_page(&frontier, &subject, offset, u16::MAX)
            .unwrap()
            .unwrap();
        assert_eq!(page.dependencies().len(), 1);
        assert!(page.copied_string_bytes() <= MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES);
        observed.extend(page.dependencies().map(|node| node.node_digest.clone()));
        pages += 1;
        match page.next_offset() {
            Some(next) => {
                assert!(next > offset);
                offset = next;
            }
            None => break,
        }
    }
    assert_eq!(pages, expected.len());
    assert_eq!(observed, expected);
}

#[test]
fn direct_dependency_pages_borrow_exact_ordered_inputs_and_distinguish_absence() {
    for mask in [0, 1, 23, 1023] {
        let mut nodes = graph_nodes(mask, 2);
        nodes.sort_by(|left, right| left.node_digest.cmp(&right.node_digest));
        let frontier = WorkspaceDerivationFrontier {
            facts: nodes
                .iter()
                .map(|node| {
                    (
                        node.subject.clone(),
                        (node.node_digest.clone(), node.output.clone()),
                    )
                })
                .collect(),
            nodes,
            sources: Vec::new(),
            resolution_digest: WorkspaceResolutionLockDigest::hash_canonical(&()).unwrap(),
            universe_digest: WorkspaceUniverseLockDigest::hash_canonical(&()).unwrap(),
            responsibility_digest: ResponsibilityDerivationClosureDigest::hash_canonical(&())
                .unwrap(),
            organizational_digest: OrganizationalTranslationClosureDigest::hash_canonical(&())
                .unwrap(),
            composition_digest: ExperienceCompositionSourceDigest::hash_canonical(&()).unwrap(),
        };
        for root in &frontier.nodes {
            let end = direct_dependency_page(&frontier, &root.subject, root.inputs.len(), 1)
                .unwrap()
                .unwrap();
            assert_eq!(end.dependencies().len(), 0);
            assert!(end.next_offset().is_none());
            for limit in [0, 1, 2, 64, 256, u16::MAX] {
                let mut actual = Vec::new();
                let mut offset = 0;
                loop {
                    let page = direct_dependency_page(&frontier, &root.subject, offset, limit)
                        .unwrap()
                        .unwrap();
                    assert!(std::ptr::eq(page.root(), root));
                    let bound = if limit == 0 { 64 } else { limit.min(256) };
                    assert!(page.dependencies().len() <= usize::from(bound));
                    assert!(
                        page.copied_string_bytes()
                            <= crate::workspace::MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES
                    );
                    for input in page.dependencies() {
                        let retained = frontier
                            .nodes
                            .iter()
                            .find(|node| node.node_digest == input.node_digest)
                            .unwrap();
                        assert!(std::ptr::eq(input, retained));
                        actual.push(input.node_digest.clone());
                    }
                    match page.next_offset() {
                        Some(next) => {
                            assert!(next > offset);
                            offset = next;
                        }
                        None => break,
                    }
                }
                assert_eq!(actual, root.inputs);
            }
            assert!(
                direct_dependency_page(&frontier, &root.subject, root.inputs.len() + 1, 1).is_err()
            );
        }
        let absent = WorkspaceDerivationSubjectV1::Workspace {
            workspace_id: WorkspaceId::parse("missing-subject").unwrap(),
        };
        assert!(direct_dependency_page(&frontier, &absent, 0, 1)
            .unwrap()
            .is_none());
    }
}

#[test]
fn subject_value_comparison_uses_full_outputs_not_proof_digests() {
    use crate::workspace::{PortableWorkspaceSubjectComparison, PortableWorkspaceValueComparison};
    use pale_core::records::{WorkspaceDerivationSourceId, WorkspaceSourceContentDigest};

    let subject = WorkspaceDerivationSubjectV1::Source {
        source_id: WorkspaceDerivationSourceId::parse("source-comparison").unwrap(),
    };
    let output = Output::SourceContent {
        source_path: pale_core::paths::OwnedConfinedRelativePath::parse("source.toml").unwrap(),
        source_digest: WorkspaceSourceContentDigest::hash_bytes(b"first"),
        byte_len: 5,
    };
    let prior = WorkspaceDerivationFrontier {
        sources: Vec::new(),
        nodes: Vec::new(),
        facts: BTreeMap::from([(
            subject.clone(),
            (
                WorkspaceDerivationNodeDigest::hash_canonical(&"prior-proof").unwrap(),
                output.clone(),
            ),
        )]),
        resolution_digest: WorkspaceResolutionLockDigest::hash_canonical(&()).unwrap(),
        universe_digest: WorkspaceUniverseLockDigest::hash_canonical(&()).unwrap(),
        responsibility_digest: ResponsibilityDerivationClosureDigest::hash_canonical(&()).unwrap(),
        organizational_digest: OrganizationalTranslationClosureDigest::hash_canonical(&()).unwrap(),
        composition_digest: ExperienceCompositionSourceDigest::hash_canonical(&()).unwrap(),
    };
    let changed_outputs = [
        output.clone(),
        Output::SourceContent {
            source_path: pale_core::paths::OwnedConfinedRelativePath::parse("renamed.toml")
                .unwrap(),
            source_digest: WorkspaceSourceContentDigest::hash_bytes(b"first"),
            byte_len: 5,
        },
        Output::SourceContent {
            source_path: pale_core::paths::OwnedConfinedRelativePath::parse("source.toml").unwrap(),
            source_digest: WorkspaceSourceContentDigest::hash_bytes(b"other"),
            byte_len: 5,
        },
        Output::SourceContent {
            source_path: pale_core::paths::OwnedConfinedRelativePath::parse("source.toml").unwrap(),
            source_digest: WorkspaceSourceContentDigest::hash_bytes(b"first"),
            byte_len: 6,
        },
    ];
    for (index, current_output) in changed_outputs.into_iter().enumerate() {
        let mut current = prior.clone();
        current.facts.insert(
            subject.clone(),
            (
                WorkspaceDerivationNodeDigest::hash_canonical(&"different-proof").unwrap(),
                current_output.clone(),
            ),
        );
        let expected = if index == 0 {
            PortableWorkspaceValueComparison::Equal
        } else {
            PortableWorkspaceValueComparison::Changed
        };
        assert_eq!(
            compare_derivation_subject(&prior, &current, &subject),
            PortableWorkspaceSubjectComparison::Shared {
                prior: &output,
                current: &current_output,
                comparison: expected,
            }
        );
        assert_eq!(
            compare_derivation_subject(&current, &prior, &subject),
            PortableWorkspaceSubjectComparison::Shared {
                prior: &current_output,
                current: &output,
                comparison: expected,
            }
        );
    }
    let different_kind = WorkspaceDerivationSubjectV1::Module {
        module_id: WorkspaceModuleId::parse("source-comparison").unwrap(),
    };
    assert_eq!(
        compare_derivation_subject(&prior, &prior, &different_kind),
        PortableWorkspaceSubjectComparison::Absent
    );
}

fn graph_nodes(edge_mask: u16, label_rotation: usize) -> Vec<WorkspaceDerivationNodeV1> {
    let mut nodes: Vec<WorkspaceDerivationNodeV1> = Vec::new();
    let mut edge_bit = 0;
    for target in 0..NODE_COUNT {
        let label = (target + label_rotation) % NODE_COUNT;
        let mut inputs = Vec::new();
        for source in nodes.iter().take(target) {
            if edge_mask & (1 << edge_bit) != 0 {
                inputs.push(source.node_digest.clone());
            }
            edge_bit += 1;
        }
        nodes.push(
            WorkspaceDerivationNodeV1::seal(
                WorkspaceDerivationOriginV1::Compilation,
                WorkspaceDerivationRuleV1::SemanticIrSealing,
                WorkspaceDerivationSubjectV1::Workspace {
                    workspace_id: WorkspaceId::parse(format!("causal-law-{label}"))
                        .expect("test workspace identity is valid"),
                },
                Output::SemanticIr {
                    workspace_semantic_ir_digest: WorkspaceSemanticIrDigest::hash_canonical(&(
                        "causal-law-semantic-ir",
                        label,
                    ))
                    .expect("test semantic identity is canonical"),
                },
                inputs,
            )
            .expect("enumerated causal node is individually valid"),
        );
    }
    nodes
}

fn seed_roots(
    nodes: &[WorkspaceDerivationNodeV1],
    source_subset: usize,
) -> BTreeMap<WorkspaceDerivationNodeDigest, BTreeSet<WorkspaceDerivationRootDigest>> {
    nodes
        .iter()
        .filter(|node| node.inputs.is_empty())
        .enumerate()
        .filter(|(index, _)| source_subset & (1 << index) != 0)
        .map(|(_, node)| {
            (
                node.node_digest.clone(),
                BTreeSet::from([WorkspaceDerivationRootDigest::hash_canonical(&(
                    "causal-law-root",
                    &node.node_digest,
                ))
                .expect("test root identity is canonical")]),
            )
        })
        .collect()
}

fn reference_root_sets(
    nodes: &[WorkspaceDerivationNodeV1],
    seeds: &BTreeMap<WorkspaceDerivationNodeDigest, BTreeSet<WorkspaceDerivationRootDigest>>,
) -> BTreeMap<WorkspaceDerivationSubjectV1, BTreeSet<WorkspaceDerivationRootDigest>> {
    let mut roots = nodes
        .iter()
        .map(|node| {
            (
                node.node_digest.clone(),
                seeds.get(&node.node_digest).cloned().unwrap_or_default(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    loop {
        let mut changed = false;
        for node in nodes {
            let inherited = node
                .inputs
                .iter()
                .flat_map(|input| roots[input].iter().cloned())
                .collect::<BTreeSet<_>>();
            let before = roots[&node.node_digest].len();
            roots
                .get_mut(&node.node_digest)
                .expect("every graph node owns a root set")
                .extend(inherited);
            changed |= roots[&node.node_digest].len() != before;
        }
        if !changed {
            break;
        }
    }
    nodes
        .iter()
        .map(|node| (node.subject.clone(), roots[&node.node_digest].clone()))
        .collect()
}

fn reference_shortest_path(
    nodes: &[WorkspaceDerivationNodeV1],
    seed: &WorkspaceDerivationNodeDigest,
    target: &WorkspaceDerivationNodeDigest,
) -> Option<Vec<WorkspaceDerivationNodeDigest>> {
    let mut dependents = nodes
        .iter()
        .map(|node| (node.node_digest.clone(), Vec::new()))
        .collect::<BTreeMap<_, Vec<WorkspaceDerivationNodeDigest>>>();
    for node in nodes {
        for input in &node.inputs {
            dependents
                .get_mut(input)
                .expect("enumerated edge has an owner")
                .push(node.node_digest.clone());
        }
    }
    fn enumerate(
        current: &WorkspaceDerivationNodeDigest,
        target: &WorkspaceDerivationNodeDigest,
        dependents: &BTreeMap<WorkspaceDerivationNodeDigest, Vec<WorkspaceDerivationNodeDigest>>,
        path: &mut Vec<WorkspaceDerivationNodeDigest>,
        candidates: &mut Vec<Vec<WorkspaceDerivationNodeDigest>>,
    ) {
        path.push(current.clone());
        if current == target {
            candidates.push(path.clone());
        } else {
            for dependent in &dependents[current] {
                enumerate(dependent, target, dependents, path, candidates);
            }
        }
        path.pop();
    }

    let mut candidates = Vec::new();
    enumerate(seed, target, &dependents, &mut Vec::new(), &mut candidates);
    candidates
        .into_iter()
        .min_by(|left, right| left.len().cmp(&right.len()).then_with(|| left.cmp(right)))
}

#[test]
fn optimized_causal_index_matches_independent_finite_dag_oracles() {
    let edge_count = (NODE_COUNT * (NODE_COUNT - 1)) / 2;
    for label_rotation in 0..NODE_COUNT {
        for edge_mask in 0..(1_u16 << edge_count) {
            let nodes = graph_nodes(edge_mask, label_rotation);
            let index = CausalProofIndex::new(&nodes).expect("enumerated graph is acyclic");
            let mut permuted = nodes.clone();
            permuted.reverse();
            for node in &mut permuted {
                node.inputs.reverse();
            }
            let permuted_index =
                CausalProofIndex::new(&permuted).expect("permutation preserves the same DAG");

            let source_count = nodes.iter().filter(|node| node.inputs.is_empty()).count();
            for source_subset in 0..(1 << source_count) {
                let seeds = seed_roots(&nodes, source_subset);
                let expected_roots = reference_root_sets(&nodes, &seeds);
                for actual_roots in [
                    index
                        .root_sets(seeds.clone())
                        .expect("enumerated seeds belong to the graph"),
                    permuted_index
                        .root_sets(seeds.clone())
                        .expect("permuted graph retains every seed"),
                ] {
                    assert_eq!(
                        actual_roots, expected_roots,
                        "rotation {label_rotation}, edge mask {edge_mask:#x}, source subset {source_subset:#x}"
                    );
                    let affected = actual_roots
                        .iter()
                        .filter_map(|(subject, roots)| (!roots.is_empty()).then_some(subject))
                        .collect::<BTreeSet<_>>();
                    let unaffected = actual_roots
                        .iter()
                        .filter_map(|(subject, roots)| roots.is_empty().then_some(subject))
                        .collect::<BTreeSet<_>>();
                    assert!(affected.is_disjoint(&unaffected));
                    assert_eq!(affected.len() + unaffected.len(), nodes.len());
                }
            }

            for seed in &nodes {
                for target in &nodes {
                    let expected =
                        reference_shortest_path(&nodes, &seed.node_digest, &target.node_digest);
                    for actual in [
                        index.shortest_path(&seed.node_digest, &target.node_digest),
                        permuted_index.shortest_path(&seed.node_digest, &target.node_digest),
                    ] {
                        match &expected {
                            Some(expected) => assert_eq!(
                                actual
                                    .expect("reachable target has a witness")
                                    .into_iter()
                                    .map(|node| node.node_digest)
                                    .collect::<Vec<_>>(),
                                *expected,
                                "rotation {label_rotation}, edge mask {edge_mask:#x}"
                            ),
                            None => assert!(
                                actual.is_err(),
                                "rotation {label_rotation}, edge mask {edge_mask:#x}"
                            ),
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn removed_root_bridge_is_exact_subject_intersection() {
    let subject = |id| WorkspaceDerivationSubjectV1::Workspace {
        workspace_id: WorkspaceId::parse(id).expect("test workspace identity is valid"),
    };
    let removed_root = WorkspaceDerivationRootDigest::hash_canonical(&"removed-root")
        .expect("test root identity is canonical");
    let roots = BTreeSet::from([removed_root.clone()]);
    let removed = subject("removed");
    let shared = subject("shared");
    let predecessor_only = subject("predecessor-only");
    let unrelated_current = subject("unrelated-current");
    let predecessor = BTreeMap::from([
        (removed, roots.clone()),
        (shared.clone(), roots.clone()),
        (predecessor_only, roots),
    ]);
    let mut current = BTreeMap::from([
        (shared.clone(), BTreeSet::new()),
        (unrelated_current.clone(), BTreeSet::new()),
    ]);

    bridge_removed_root_sets(&mut current, predecessor);

    assert_eq!(current.len(), 2);
    assert_eq!(current[&shared], BTreeSet::from([removed_root]));
    assert!(current[&unrelated_current].is_empty());
}
