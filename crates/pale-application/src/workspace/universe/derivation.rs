//! Causal derivation frontiers and transition-receipt mechanics.

use super::*;
use pale_core::records::WorkspaceDerivationOutputV1 as Output;
use std::collections::VecDeque;

/// Exact causal receipt frontier recomputed from one verified workspace IR.
///
/// This bounded compilation artifact is not the resident kernel's packed
/// operational topology. It retains the immutable public causal-proof DAG;
/// only the comparison index constructed for one receipt is ephemeral. Receipt
/// impact follows that separately typed semantic proof, while declaration
/// dependency analysis supplies policy inputs but does not own receipt
/// reachability.
#[derive(Debug, Clone)]
pub(super) struct WorkspaceDerivationFrontier {
    pub(super) sources: Vec<WorkspaceDerivationSourceV1>,
    pub(super) nodes: Vec<WorkspaceDerivationNodeV1>,
    pub(super) facts: BTreeMap<
        WorkspaceDerivationSubjectV1,
        (
            WorkspaceDerivationNodeDigest,
            pale_core::records::WorkspaceDerivationOutputV1,
        ),
    >,
    pub(super) resolution_digest: WorkspaceResolutionLockDigest,
    pub(super) universe_digest: WorkspaceUniverseLockDigest,
    pub(super) responsibility_digest: ResponsibilityDerivationClosureDigest,
    pub(super) organizational_digest: OrganizationalTranslationClosureDigest,
    pub(super) composition_digest: ExperienceCompositionSourceDigest,
}

/// Compare indexed typed outputs without inferring change from proof ancestry.
pub(super) fn compare_derivation_subject<'a>(
    prior: &'a WorkspaceDerivationFrontier,
    current: &'a WorkspaceDerivationFrontier,
    subject: &WorkspaceDerivationSubjectV1,
) -> super::super::portable::PortableWorkspaceSubjectComparison<'a> {
    use super::super::portable::{
        PortableWorkspaceSubjectComparison as Comparison, PortableWorkspaceValueComparison,
    };

    match (prior.facts.get(subject), current.facts.get(subject)) {
        (None, None) => Comparison::Absent,
        (Some((_, prior)), None) => Comparison::PriorOnly { prior },
        (None, Some((_, current))) => Comparison::CurrentOnly { current },
        (Some((_, prior)), Some((_, current))) => Comparison::Shared {
            prior,
            current,
            comparison: if prior == current {
                PortableWorkspaceValueComparison::Equal
            } else {
                PortableWorkspaceValueComparison::Changed
            },
        },
    }
}

/// Merge bounded ordered ranges without constructing the complete subject union.
pub(super) fn comparison_subject_page<'a>(
    prior: &'a WorkspaceDerivationFrontier,
    current: &'a WorkspaceDerivationFrontier,
    after: Option<&WorkspaceDerivationSubjectV1>,
    requested_limit: u16,
) -> super::super::PortableWorkspaceComparisonSubjectPage<'a> {
    use super::super::comparison_subjects::{
        subject_identity_bytes, DEFAULT_COMPARISON_SUBJECT_LIMIT, MAX_COMPARISON_SUBJECT_LIMIT,
        MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES,
    };
    use std::cmp::Ordering;
    use std::ops::Bound::{Excluded, Unbounded};

    let lower = after.map_or(Unbounded, Excluded);
    let mut prior_range = prior.facts.range((lower, Unbounded));
    let mut current_range = current.facts.range((lower, Unbounded));
    let mut prior_entry = prior_range.next();
    let mut current_entry = current_range.next();
    let mut inspected = usize::from(prior_entry.is_some()) + usize::from(current_entry.is_some());
    let limit = usize::from(if requested_limit == 0 {
        DEFAULT_COMPARISON_SUBJECT_LIMIT
    } else {
        requested_limit.min(MAX_COMPARISON_SUBJECT_LIMIT)
    });
    let mut subjects =
        Vec::with_capacity(limit.min(prior.facts.len().saturating_add(current.facts.len())));
    let mut selected_identity_bytes: usize = 0;
    let mut key_comparisons = 0;
    while subjects.len() < limit {
        let (subject, take_prior, take_current) = match (prior_entry, current_entry) {
            (None, None) => break,
            (Some((subject, _)), None) => (subject, true, false),
            (None, Some((subject, _))) => (subject, false, true),
            (Some((prior_subject, _)), Some((current_subject, _))) => {
                key_comparisons += 1;
                match prior_subject.cmp(current_subject) {
                    Ordering::Less => (prior_subject, true, false),
                    Ordering::Equal => (prior_subject, true, true),
                    Ordering::Greater => (current_subject, false, true),
                }
            }
        };
        let next_identity_bytes = subject_identity_bytes(subject);
        assert!(
            next_identity_bytes <= MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES,
            "one admitted workspace subject exceeds the fixed page identity-byte budget"
        );
        if selected_identity_bytes.saturating_add(next_identity_bytes)
            > MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES
        {
            break;
        }
        subjects.push(subject);
        selected_identity_bytes += next_identity_bytes;
        if take_prior {
            prior_entry = prior_range.next();
            inspected += usize::from(prior_entry.is_some());
        }
        if take_current {
            current_entry = current_range.next();
            inspected += usize::from(current_entry.is_some());
        }
    }
    let next_after = if prior_entry.is_some() || current_entry.is_some() {
        subjects.last().copied()
    } else {
        None
    };
    super::super::PortableWorkspaceComparisonSubjectPage {
        subjects,
        subject_identity_bytes: selected_identity_bytes,
        inspected,
        key_comparisons,
        next_after,
    }
}

/// Select one subject and borrow its direct inputs without building an index.
pub(super) fn direct_dependency_page<'a>(
    frontier: &'a WorkspaceDerivationFrontier,
    subject: &WorkspaceDerivationSubjectV1,
    offset: usize,
    limit: u16,
) -> Result<
    Option<super::super::PortableWorkspaceDependencyPage<'a>>,
    super::super::InvalidDependencyOffset,
> {
    let Some(root) = subject_node(frontier, subject) else {
        return Ok(None);
    };
    super::super::PortableWorkspaceDependencyPage::new(root, &frontier.nodes, offset, limit)
        .map(Some)
}

/// Scan bounded candidates for direct consumers of one selected root.
pub(super) fn direct_dependent_page<'a>(
    frontier: &'a WorkspaceDerivationFrontier,
    subject: &WorkspaceDerivationSubjectV1,
    offset: usize,
    maximum_inspected: u16,
    maximum_returned: u16,
) -> Result<
    Option<super::super::PortableWorkspaceDependentPage<'a>>,
    super::super::InvalidDependentOffset,
> {
    let Some(root) = subject_node(frontier, subject) else {
        return Ok(None);
    };
    super::super::PortableWorkspaceDependentPage::new(
        root,
        &frontier.nodes,
        offset,
        maximum_inspected,
        maximum_returned,
    )
    .map(Some)
}

/// Borrow the existing selected node by full subject and sealed digest order.
pub(super) fn subject_node<'a>(
    frontier: &'a WorkspaceDerivationFrontier,
    subject: &WorkspaceDerivationSubjectV1,
) -> Option<&'a WorkspaceDerivationNodeV1> {
    let (digest, _) = frontier.facts.get(subject)?;
    let index = frontier
        .nodes
        .binary_search_by(|node| node.node_digest.cmp(digest))
        .expect("admitted subject names one retained proof node");
    Some(&frontier.nodes[index])
}

/// Which admitted frontier owns a causal explanation path.
///
/// Removed roots have no current node. Their witness therefore traverses the
/// predecessor proof DAG and bridges by exact semantic subject into the
/// receipt's current impact partition. Added and modified roots traverse the
/// current proof DAG directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::workspace) enum WorkspaceDerivationExplanationFrontier {
    /// The path belongs to the verified predecessor frontier.
    Predecessor,
    /// The path belongs to the verified current frontier.
    Current,
}

/// One exact result of querying the admitted derivation receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workspace) enum WorkspaceDerivationExplanation {
    /// The queried subject is absent from the current impact partition.
    NotPresent,
    /// The subject is current and outside every change-root closure.
    Unaffected,
    /// The subject is reachable from the selected exact root.
    Affected {
        /// Deterministically selected root from the subject's root set.
        root: Box<WorkspaceDerivationRootV1>,
        /// Admitted frontier traversed by `path`.
        frontier: WorkspaceDerivationExplanationFrontier,
        /// Shortest deterministic path from the root node through the subject.
        path: Vec<WorkspaceDerivationNodeV1>,
    },
}

pub(super) struct WorkspaceDerivationFrontierBuilder {
    sources: Vec<WorkspaceDerivationSourceV1>,
    nodes: Vec<WorkspaceDerivationNodeV1>,
    facts: BTreeMap<
        WorkspaceDerivationSubjectV1,
        (
            WorkspaceDerivationNodeDigest,
            pale_core::records::WorkspaceDerivationOutputV1,
        ),
    >,
    semantic_inputs: Vec<WorkspaceDerivationNodeDigest>,
    workspace_id: WorkspaceId,
    resolution_digest: WorkspaceResolutionLockDigest,
    universe_digest: WorkspaceUniverseLockDigest,
    responsibility_digest: ResponsibilityDerivationClosureDigest,
    organizational_digest: OrganizationalTranslationClosureDigest,
    composition_digest: ExperienceCompositionSourceDigest,
}

impl WorkspaceDerivationFrontierBuilder {
    pub(super) fn seal(
        mut self,
        digest: &WorkspaceSemanticIrDigest,
    ) -> Result<WorkspaceDerivationFrontier, WorkspaceError> {
        let node = seal_derivation_node(
            WorkspaceDerivationOriginV1::Compilation,
            WorkspaceDerivationRuleV1::SemanticIrSealing,
            WorkspaceDerivationSubjectV1::Workspace {
                workspace_id: self.workspace_id,
            },
            Output::SemanticIr {
                workspace_semantic_ir_digest: digest.clone(),
            },
            self.semantic_inputs,
        )?;
        insert_derivation_node(&mut self.nodes, &mut self.facts, node)?;
        self.nodes
            .sort_by(|left, right| left.node_digest.cmp(&right.node_digest));
        Ok(WorkspaceDerivationFrontier {
            sources: self.sources,
            nodes: self.nodes,
            facts: self.facts,
            resolution_digest: self.resolution_digest,
            universe_digest: self.universe_digest,
            responsibility_digest: self.responsibility_digest,
            organizational_digest: self.organizational_digest,
            composition_digest: self.composition_digest,
        })
    }
}

/// Seed a causal frontier from normalized source, locks, and selected semantics.
pub(super) fn derivation_frontier_from(
    normalized: &NormalizedWorkspace,
) -> Result<WorkspaceDerivationFrontierBuilder, WorkspaceError> {
    let mut sources = BTreeMap::new();
    sources.insert(
        normalized.linked.source.manifest_source.source_id.clone(),
        normalized.linked.source.manifest_source.clone(),
    );
    sources.insert(
        normalized
            .linked
            .source
            .resolution_lock
            .source
            .source_id
            .clone(),
        normalized.linked.source.resolution_lock.source.clone(),
    );
    sources.insert(
        normalized
            .linked
            .source
            .universe_lock
            .source
            .source_id
            .clone(),
        normalized.linked.source.universe_lock.source.clone(),
    );
    let mut selected_components = BTreeSet::new();
    for module_id in &normalized.linked.adopted_modules {
        let module = &normalized.linked.source.modules[module_id];
        sources.insert(module.source.source_id.clone(), module.source.clone());
        for component in &module.value.module.source_components {
            if let Some(retained) = normalized
                .linked
                .source
                .components
                .get(&component.component_id)
            {
                selected_components.insert(component.component_id.clone());
                sources.insert(retained.source.source_id.clone(), retained.source.clone());
            }
        }
    }
    for retained in normalized.linked.source.responsibility_derivations.values() {
        sources.insert(retained.source.source_id.clone(), retained.source.clone());
    }
    if let Some(retained) = &normalized.linked.source.organizational_translation {
        sources.insert(retained.source.source_id.clone(), retained.source.clone());
    }
    if let Some(retained) = &normalized.linked.source.experience_composition {
        sources.insert(retained.source.source_id.clone(), retained.source.clone());
    }
    let sources = sources.into_values().collect::<Vec<_>>();
    let mut nodes = Vec::new();
    let mut facts = BTreeMap::new();
    let mut source_nodes = BTreeMap::new();
    for source in &sources {
        let subject = WorkspaceDerivationSubjectV1::Source {
            source_id: source.source_id.clone(),
        };
        let node = seal_derivation_node(
            WorkspaceDerivationOriginV1::Source {
                source_id: source.source_id.clone(),
            },
            WorkspaceDerivationRuleV1::SourceAdmission,
            subject,
            Output::SourceContent {
                source_path: source.source_path.clone(),
                source_digest: source.source_digest.clone(),
                byte_len: source.byte_len,
            },
            Vec::new(),
        )?;
        source_nodes.insert(source.source_id.clone(), node.node_digest.clone());
        insert_derivation_node(&mut nodes, &mut facts, node)?;
    }

    let responsibility_digest = ResponsibilityDerivationClosureDigest::hash_canonical(&(
        "pale.responsibility-derivation-closure.v1",
        &normalized.linked.responsibility_derivations,
    ))?;
    let mut responsibility_inputs =
        vec![source_nodes[&normalized.linked.source.manifest_source.source_id].clone()];
    responsibility_inputs.extend(
        normalized
            .linked
            .source
            .responsibility_derivations
            .values()
            .map(|retained| source_nodes[&retained.source.source_id].clone()),
    );
    let responsibility_node = seal_derivation_node(
        WorkspaceDerivationOriginV1::Compilation,
        WorkspaceDerivationRuleV1::ResponsibilityLowering,
        WorkspaceDerivationSubjectV1::ResponsibilityClosure,
        Output::ResponsibilityClosure {
            closure_digest: responsibility_digest.clone(),
        },
        responsibility_inputs,
    )?;
    let responsibility_node_digest = responsibility_node.node_digest.clone();
    insert_derivation_node(&mut nodes, &mut facts, responsibility_node)?;

    let organizational_digest = OrganizationalTranslationClosureDigest::hash_canonical(&(
        "pale.organizational-translation-closure.v1",
        &normalized.linked.organizational_translation,
    ))?;
    let mut organizational_inputs =
        vec![source_nodes[&normalized.linked.source.manifest_source.source_id].clone()];
    if let Some(retained) = &normalized.linked.source.organizational_translation {
        organizational_inputs.push(source_nodes[&retained.source.source_id].clone());
    }
    let organizational_node = seal_derivation_node(
        WorkspaceDerivationOriginV1::Compilation,
        WorkspaceDerivationRuleV1::OrganizationalTranslation,
        WorkspaceDerivationSubjectV1::OrganizationalTranslationClosure,
        Output::OrganizationalTranslationClosure {
            closure_digest: organizational_digest.clone(),
        },
        organizational_inputs,
    )?;
    let organizational_node_digest = organizational_node.node_digest.clone();
    insert_derivation_node(&mut nodes, &mut facts, organizational_node)?;

    let composition_digest = ExperienceCompositionSourceDigest::hash_canonical(&(
        "pale.experience-composition-source.v1",
        &normalized.linked.experience_composition,
    ))?;
    let mut composition_inputs =
        vec![source_nodes[&normalized.linked.source.manifest_source.source_id].clone()];
    if let Some(retained) = &normalized.linked.source.experience_composition {
        composition_inputs.push(source_nodes[&retained.source.source_id].clone());
    }
    let composition_node = seal_derivation_node(
        WorkspaceDerivationOriginV1::Compilation,
        WorkspaceDerivationRuleV1::ExperienceComposition,
        WorkspaceDerivationSubjectV1::ExperienceComposition,
        Output::ExperienceComposition {
            source_digest: composition_digest.clone(),
        },
        composition_inputs,
    )?;
    let composition_node_digest = composition_node.node_digest.clone();
    insert_derivation_node(&mut nodes, &mut facts, composition_node)?;

    let mut component_nodes = BTreeMap::new();
    let mut declaration_source_nodes = BTreeMap::new();
    for component_id in &selected_components {
        let retained = &normalized.linked.source.components[component_id];
        let component_digest = retained.value.component_digest()?;
        let node = seal_derivation_node(
            WorkspaceDerivationOriginV1::Source {
                source_id: retained.source.source_id.clone(),
            },
            WorkspaceDerivationRuleV1::ComponentAdmission,
            WorkspaceDerivationSubjectV1::Component {
                component_id: component_id.clone(),
            },
            Output::Component { component_digest },
            vec![source_nodes[&retained.source.source_id].clone()],
        )?;
        for declaration in &retained.value.declarations {
            declaration_source_nodes.insert(
                QualifiedDeclarationRef {
                    owner_module: retained.value.module_id.clone(),
                    local_ref: declaration.declaration_ref(),
                },
                node.node_digest.clone(),
            );
        }
        component_nodes.insert(component_id.clone(), node.node_digest.clone());
        insert_derivation_node(&mut nodes, &mut facts, node)?;
    }

    let mut module_nodes = BTreeMap::new();
    for module_id in &normalized.linked.adopted_modules {
        let retained = &normalized.linked.source.modules[module_id];
        let mut inputs = vec![source_nodes[&retained.source.source_id].clone()];
        inputs.extend(
            retained
                .value
                .module
                .source_components
                .iter()
                .filter_map(|component| component_nodes.get(&component.component_id).cloned()),
        );
        let node = seal_derivation_node(
            module_origin(&retained.value),
            WorkspaceDerivationRuleV1::ModuleSealing,
            WorkspaceDerivationSubjectV1::Module {
                module_id: module_id.clone(),
            },
            Output::Module {
                module_digest: retained.value.module_digest.clone(),
            },
            inputs,
        )?;
        module_nodes.insert(module_id.clone(), node.node_digest.clone());
        insert_derivation_node(&mut nodes, &mut facts, node)?;
    }

    let mut declaration_nodes = BTreeMap::new();
    for (declaration_ref, declaration) in &normalized.linked.declarations {
        let module = &normalized.linked.source.modules[&declaration_ref.owner_module].value;
        let input = declaration_source_nodes
            .get(declaration_ref)
            .cloned()
            .unwrap_or_else(|| module_nodes[&declaration_ref.owner_module].clone());
        let value_digest = WorkspaceDerivationValueDigest::hash_canonical(&(
            "pale.workspace-linked-declaration.v1",
            declaration_ref,
            declaration,
        ))?;
        let node = seal_derivation_node(
            module_origin(module),
            WorkspaceDerivationRuleV1::DeclarationLinking,
            declaration_subject(declaration_ref),
            Output::Declaration { value_digest },
            vec![input],
        )?;
        declaration_nodes.insert(declaration_ref.clone(), node.node_digest.clone());
        insert_derivation_node(&mut nodes, &mut facts, node)?;
    }

    let mut import_nodes = Vec::new();
    for import in &normalized.linked.resolved_imports {
        let mut inputs = vec![
            module_nodes[&import.importer_module].clone(),
            module_nodes[&import.upstream_module].clone(),
        ];
        inputs.extend(
            import
                .declarations
                .iter()
                .filter_map(|declaration| declaration_nodes.get(declaration).cloned()),
        );
        let node = seal_derivation_node(
            import_origin(import),
            WorkspaceDerivationRuleV1::ImportLinking,
            import.subject(),
            Output::ImportSurface {
                surface_digest: WorkspaceImportSurfaceDigest::hash_canonical(&(
                    "pale.workspace-import-surface.v1",
                    import,
                ))?,
            },
            inputs,
        )?;
        import_nodes.push((import.clone(), node.node_digest.clone()));
        insert_derivation_node(&mut nodes, &mut facts, node)?;
    }

    let mut extension_nodes = Vec::new();
    for extension in &normalized.linked.extensions {
        let mut inputs = vec![
            module_nodes[&extension.owner_module].clone(),
            declaration_nodes[&extension.target].clone(),
        ];
        inputs.extend(
            extension
                .added_traits
                .iter()
                .map(|declaration| declaration_nodes[declaration].clone()),
        );
        let value_digest = WorkspaceDerivationValueDigest::hash_canonical(&(
            "pale.workspace-extension-fact.v1",
            extension,
        ))?;
        let node = seal_derivation_node(
            WorkspaceDerivationOriginV1::Extension {
                owner_module: extension.owner_module.clone(),
                namespace: extension.namespace.clone(),
                target: extension.target.local_ref.clone(),
            },
            WorkspaceDerivationRuleV1::ExtensionApplication,
            extension.subject(),
            Output::Extension { value_digest },
            inputs,
        )?;
        extension_nodes.push((extension.clone(), node.node_digest.clone()));
        insert_derivation_node(&mut nodes, &mut facts, node)?;
    }

    let propagation = normalized.linked.dependency_analysis.input_propagation();
    debug_assert_eq!(
        propagation.components_finalized,
        normalized.linked.dependency_analysis.components.len()
    );
    debug_assert!(propagation.member_insertions >= normalized.linked.declarations.len());
    let mut policy_inputs = normalized
        .linked
        .declarations
        .keys()
        .map(|declaration| {
            let inputs = propagation
                .inputs_for(declaration)
                .expect("linked declaration belongs to one SCC");
            let nodes = inputs
                .iter()
                .map(|input| declaration_nodes[input].clone())
                .collect();
            (declaration.clone(), nodes)
        })
        .collect::<BTreeMap<_, BTreeSet<_>>>();
    for (import, node) in &import_nodes {
        for declaration in &import.declarations {
            policy_inputs
                .get_mut(declaration)
                .expect("resolved import declaration belongs to the linked graph")
                .insert(node.clone());
        }
    }
    for (extension, node) in &extension_nodes {
        policy_inputs
            .get_mut(&extension.target)
            .expect("linked extension target belongs to the declaration graph")
            .insert(node.clone());
    }
    let mut policy_nodes = Vec::new();
    for (declaration_ref, policy) in &normalized.policies {
        let value_digest = WorkspaceDerivationValueDigest::hash_canonical(&(
            "pale.workspace-effective-policy.v1",
            declaration_ref,
            policy,
        ))?;
        let node = seal_derivation_node(
            WorkspaceDerivationOriginV1::Compilation,
            WorkspaceDerivationRuleV1::PolicyComposition,
            WorkspaceDerivationSubjectV1::Policy {
                owner_module: declaration_ref.owner_module.clone(),
                declaration: declaration_ref.local_ref.clone(),
            },
            Output::EffectivePolicy { value_digest },
            policy_inputs[declaration_ref].iter().cloned().collect(),
        )?;
        policy_nodes.push(node.node_digest.clone());
        insert_derivation_node(&mut nodes, &mut facts, node)?;
    }

    let mut resource_nodes = Vec::new();
    for (owner_module, resource) in &normalized.linked.resources {
        let value_digest = WorkspaceDerivationValueDigest::hash_canonical(&(
            "pale.workspace-resource-selection.v1",
            owner_module,
            resource,
        ))?;
        let node = seal_derivation_node(
            module_origin(&normalized.linked.source.modules[owner_module].value),
            WorkspaceDerivationRuleV1::ResourceSelection,
            WorkspaceDerivationSubjectV1::Resource {
                owner_module: owner_module.clone(),
                resource_id: resource.resource_id().clone(),
            },
            Output::Resource { value_digest },
            vec![module_nodes[owner_module].clone()],
        )?;
        resource_nodes.push(node.node_digest.clone());
        insert_derivation_node(&mut nodes, &mut facts, node)?;
    }

    let resolution = &normalized.linked.source.resolution_lock;
    let resolution_node = seal_derivation_node(
        WorkspaceDerivationOriginV1::Source {
            source_id: resolution.source.source_id.clone(),
        },
        WorkspaceDerivationRuleV1::ResolutionLocking,
        WorkspaceDerivationSubjectV1::ResolutionLock,
        Output::ResolutionLock {
            resolution_digest: resolution.value.resolution_digest.clone(),
        },
        vec![source_nodes[&resolution.source.source_id].clone()],
    )?;
    let resolution_node_digest = resolution_node.node_digest.clone();
    insert_derivation_node(&mut nodes, &mut facts, resolution_node)?;
    let universe = &normalized.linked.source.universe_lock;
    let universe_node = seal_derivation_node(
        WorkspaceDerivationOriginV1::Source {
            source_id: universe.source.source_id.clone(),
        },
        WorkspaceDerivationRuleV1::UniverseLocking,
        WorkspaceDerivationSubjectV1::UniverseLock,
        Output::UniverseLock {
            universe_digest: universe.value.universe_digest.clone(),
        },
        vec![
            source_nodes[&universe.source.source_id].clone(),
            resolution_node_digest.clone(),
        ],
    )?;
    insert_derivation_node(&mut nodes, &mut facts, universe_node)?;

    // Lock nodes remain validation evidence. Semantic identity depends only on
    // the exact adopted release closure, retained in the semantic preimage.
    let mut semantic_inputs =
        vec![source_nodes[&normalized.linked.source.manifest_source.source_id].clone()];
    semantic_inputs.push(responsibility_node_digest);
    semantic_inputs.push(organizational_node_digest);
    semantic_inputs.push(composition_node_digest);
    semantic_inputs.extend(module_nodes.into_values());
    semantic_inputs.extend(policy_nodes);
    semantic_inputs.extend(import_nodes.iter().map(|(_, node)| node.clone()));
    semantic_inputs.extend(extension_nodes.iter().map(|(_, node)| node.clone()));
    semantic_inputs.extend(resource_nodes);
    Ok(WorkspaceDerivationFrontierBuilder {
        sources,
        nodes,
        facts,
        semantic_inputs,
        workspace_id: normalized
            .linked
            .source
            .manifest
            .workspace
            .workspace_id
            .clone(),
        resolution_digest: normalized
            .linked
            .source
            .resolution_lock
            .value
            .resolution_digest
            .clone(),
        universe_digest: normalized
            .linked
            .source
            .universe_lock
            .value
            .universe_digest
            .clone(),
        responsibility_digest,
        organizational_digest,
        composition_digest,
    })
}

fn seal_derivation_node(
    origin: WorkspaceDerivationOriginV1,
    rule: WorkspaceDerivationRuleV1,
    subject: WorkspaceDerivationSubjectV1,
    output: pale_core::records::WorkspaceDerivationOutputV1,
    inputs: Vec<WorkspaceDerivationNodeDigest>,
) -> Result<WorkspaceDerivationNodeV1, WorkspaceError> {
    let inputs = inputs
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    WorkspaceDerivationNodeV1::seal(origin, rule, subject, output, inputs)
        .map_err(|error| WorkspaceError::InvalidDerivationReceipt(Box::new(error)))
}

fn insert_derivation_node(
    nodes: &mut Vec<WorkspaceDerivationNodeV1>,
    facts: &mut BTreeMap<
        WorkspaceDerivationSubjectV1,
        (
            WorkspaceDerivationNodeDigest,
            pale_core::records::WorkspaceDerivationOutputV1,
        ),
    >,
    node: WorkspaceDerivationNodeV1,
) -> Result<(), WorkspaceError> {
    if facts
        .insert(
            node.subject.clone(),
            (node.node_digest.clone(), node.output.clone()),
        )
        .is_some()
    {
        return Err(WorkspaceError::DuplicateLanguageIdentity {
            kind: "derivation subject",
            id: format!("{:?}", node.subject),
        });
    }
    nodes.push(node);
    Ok(())
}

fn module_origin(module: &pale_core::records::WorkspaceModuleV1) -> WorkspaceDerivationOriginV1 {
    WorkspaceDerivationOriginV1::ModuleRelease {
        module_id: module.module.module_id.clone(),
        release_id: module.module.release_id.clone(),
        module_digest: module.module_digest.clone(),
    }
}

fn declaration_subject(declaration: &QualifiedDeclarationRef) -> WorkspaceDerivationSubjectV1 {
    WorkspaceDerivationSubjectV1::Declaration {
        owner_module: declaration.owner_module.clone(),
        declaration: declaration.local_ref.clone(),
    }
}

fn import_origin(import: &ResolvedImportPath) -> WorkspaceDerivationOriginV1 {
    WorkspaceDerivationOriginV1::Import {
        importer_module: import.importer_module.clone(),
        upstream_module: import.upstream_module.clone(),
        upstream_release: import.upstream_release.clone(),
        export_id: import.export_id.clone(),
        source_namespace: import.source_namespace.clone(),
        local_namespace: import.local_namespace.clone(),
    }
}

pub(super) fn build_derivation_receipt(
    prior: &WorkspaceDerivationFrontier,
    current: &WorkspaceDerivationFrontier,
    current_workspace_semantic_ir_digest: &WorkspaceSemanticIrDigest,
) -> Result<WorkspaceDerivationReceiptV1, WorkspaceError> {
    let subjects = prior
        .facts
        .keys()
        .chain(current.facts.keys())
        .filter(|subject| matches!(subject, WorkspaceDerivationSubjectV1::Source { .. }))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut roots = Vec::new();
    let mut current_seeds =
        BTreeMap::<WorkspaceDerivationNodeDigest, BTreeSet<WorkspaceDerivationRootDigest>>::new();
    let mut prior_removed_seeds =
        BTreeMap::<WorkspaceDerivationNodeDigest, BTreeSet<WorkspaceDerivationRootDigest>>::new();
    for subject in subjects {
        let root = match (prior.facts.get(&subject), current.facts.get(&subject)) {
            (None, Some((current_node, _))) => WorkspaceDerivationRootV1::seal(
                subject,
                WorkspaceDerivationRootChangeV1::Added {
                    current_node: current_node.clone(),
                },
            )?,
            (Some((_, prior_output)), Some((current_node, current_output)))
                if prior_output != current_output =>
            {
                WorkspaceDerivationRootV1::seal(
                    subject,
                    WorkspaceDerivationRootChangeV1::Modified {
                        prior_output: prior_output.clone(),
                        current_node: current_node.clone(),
                    },
                )?
            }
            (Some((prior_node, prior_output)), None) => {
                let root = WorkspaceDerivationRootV1::seal(
                    subject,
                    WorkspaceDerivationRootChangeV1::Removed {
                        prior_output: prior_output.clone(),
                    },
                )?;
                prior_removed_seeds
                    .entry(prior_node.clone())
                    .or_default()
                    .insert(root.root_digest.clone());
                roots.push(root);
                continue;
            }
            (Some(_), Some(_)) => continue,
            (None, None) => unreachable!("source subject came from one compared frontier"),
        };
        let current_node = match &root.change {
            WorkspaceDerivationRootChangeV1::Added { current_node }
            | WorkspaceDerivationRootChangeV1::Modified { current_node, .. } => current_node,
            WorkspaceDerivationRootChangeV1::Removed { .. } => {
                unreachable!("removed roots were handled above")
            }
        };
        current_seeds
            .entry(current_node.clone())
            .or_default()
            .insert(root.root_digest.clone());
        roots.push(root);
    }

    let mut impact_roots = causal_root_sets(&current.nodes, current_seeds)?;
    let removed_reachability = causal_root_sets(&prior.nodes, prior_removed_seeds)?;
    bridge_removed_root_sets(&mut impact_roots, removed_reachability);
    let impacts = impact_roots
        .into_iter()
        .map(|(subject, roots)| WorkspaceDerivationImpactV1 {
            subject,
            state: if roots.is_empty() {
                WorkspaceDerivationImpactStateV1::Unaffected
            } else {
                WorkspaceDerivationImpactStateV1::Affected {
                    root_digests: roots.into_iter().collect(),
                }
            },
        })
        .collect();
    let receipt = WorkspaceDerivationReceiptV1::seal(WorkspaceDerivationReceiptPreimageV1 {
        prior_resolution_digest: prior.resolution_digest.clone(),
        current_resolution_digest: current.resolution_digest.clone(),
        prior_universe_digest: prior.universe_digest.clone(),
        current_universe_digest: current.universe_digest.clone(),
        workspace_semantic_ir_digest: current_workspace_semantic_ir_digest.clone(),
        sources: current.sources.clone(),
        nodes: current.nodes.clone(),
        roots,
        impacts,
    })
    .map_err(|error| WorkspaceError::InvalidDerivationReceipt(Box::new(error)))?;
    Ok(receipt)
}

/// Bridge predecessor-only reachability by exact current semantic subject.
///
/// Set intersection is structural here: predecessor subjects absent from the
/// current proof are discarded, while unrelated current subjects retain their
/// existing root set.
fn bridge_removed_root_sets(
    current: &mut BTreeMap<WorkspaceDerivationSubjectV1, BTreeSet<WorkspaceDerivationRootDigest>>,
    predecessor: BTreeMap<WorkspaceDerivationSubjectV1, BTreeSet<WorkspaceDerivationRootDigest>>,
) {
    for (subject, removed_roots) in predecessor {
        if let Some(current_roots) = current.get_mut(&subject) {
            current_roots.extend(removed_roots);
        }
    }
}

fn causal_root_sets(
    nodes: &[WorkspaceDerivationNodeV1],
    seeds: BTreeMap<WorkspaceDerivationNodeDigest, BTreeSet<WorkspaceDerivationRootDigest>>,
) -> Result<
    BTreeMap<WorkspaceDerivationSubjectV1, BTreeSet<WorkspaceDerivationRootDigest>>,
    WorkspaceError,
> {
    CausalProofIndex::new(nodes)?.root_sets(seeds)
}

/// Explain one current receipt subject against the exact compared frontiers.
///
/// The receipt is assumed to have been rebuilt and verified from these same
/// admitted owners. This function adds no semantic edge: it extracts one
/// shortest witness from the appropriate immutable proof DAG.
pub(in crate::workspace) fn explain_derivation_subject(
    prior: &WorkspaceDerivationFrontier,
    current: &WorkspaceDerivationFrontier,
    receipt: &WorkspaceDerivationReceiptV1,
    subject: &WorkspaceDerivationSubjectV1,
) -> Result<WorkspaceDerivationExplanation, WorkspaceError> {
    let Ok(impact_index) = receipt
        .impacts
        .binary_search_by(|impact| impact.subject.cmp(subject))
    else {
        return Ok(WorkspaceDerivationExplanation::NotPresent);
    };
    let WorkspaceDerivationImpactStateV1::Affected { root_digests } =
        &receipt.impacts[impact_index].state
    else {
        return Ok(WorkspaceDerivationExplanation::Unaffected);
    };
    let root = root_digests
        .iter()
        .map(|root_digest| {
            receipt
                .roots
                .binary_search_by(|root| root.root_digest.cmp(root_digest))
                .map(|index| &receipt.roots[index])
                .map_err(|_| WorkspaceError::DerivationSemanticMismatch)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .min_by(|left, right| {
            explanation_root_rank(&left.change)
                .cmp(&explanation_root_rank(&right.change))
                .then_with(|| left.root_digest.cmp(&right.root_digest))
        })
        .ok_or(WorkspaceError::DerivationSemanticMismatch)?
        .clone();
    let (frontier, seed, owner) = match &root.change {
        WorkspaceDerivationRootChangeV1::Added { current_node }
        | WorkspaceDerivationRootChangeV1::Modified { current_node, .. } => (
            current,
            current_node,
            WorkspaceDerivationExplanationFrontier::Current,
        ),
        WorkspaceDerivationRootChangeV1::Removed { .. } => (
            prior,
            &prior
                .facts
                .get(&root.subject)
                .ok_or(WorkspaceError::DerivationSemanticMismatch)?
                .0,
            WorkspaceDerivationExplanationFrontier::Predecessor,
        ),
    };
    let target = &frontier
        .facts
        .get(subject)
        .ok_or(WorkspaceError::DerivationSemanticMismatch)?
        .0;
    let path = CausalProofIndex::new(&frontier.nodes)?.shortest_path(seed, target)?;
    Ok(WorkspaceDerivationExplanation::Affected {
        root: Box::new(root),
        frontier: owner,
        path,
    })
}

/// Prefer a predecessor bridge when one contributes to the queried subject.
///
/// Removed roots are the only causes whose nodes no longer exist in the current
/// DAG. Surfacing that discontinuity first preserves more information; modified
/// roots precede added roots, and exact root digest breaks same-kind ties.
const fn explanation_root_rank(change: &WorkspaceDerivationRootChangeV1) -> u8 {
    match change {
        WorkspaceDerivationRootChangeV1::Removed { .. } => 0,
        WorkspaceDerivationRootChangeV1::Modified { .. } => 1,
        WorkspaceDerivationRootChangeV1::Added { .. } => 2,
    }
}

/// Ephemeral adjacency over one canonical causal proof.
///
/// This index has no identity and never enters a resident generation. It exists
/// only while propagating roots or answering one causal query, then is dropped.
struct CausalProofIndex<'a> {
    nodes: BTreeMap<WorkspaceDerivationNodeDigest, &'a WorkspaceDerivationNodeV1>,
    dependents: BTreeMap<WorkspaceDerivationNodeDigest, Vec<WorkspaceDerivationNodeDigest>>,
    topological: Vec<WorkspaceDerivationNodeDigest>,
}

impl<'a> CausalProofIndex<'a> {
    fn new(nodes: &'a [WorkspaceDerivationNodeV1]) -> Result<Self, WorkspaceError> {
        let by_digest = nodes
            .iter()
            .map(|node| (node.node_digest.clone(), node))
            .collect::<BTreeMap<_, _>>();
        if by_digest.len() != nodes.len() {
            return Err(WorkspaceError::DerivationSemanticMismatch);
        }
        let mut indegree = nodes
            .iter()
            .map(|node| (node.node_digest.clone(), node.inputs.len()))
            .collect::<BTreeMap<_, _>>();
        let mut dependents = nodes
            .iter()
            .map(|node| (node.node_digest.clone(), Vec::new()))
            .collect::<BTreeMap<_, Vec<WorkspaceDerivationNodeDigest>>>();
        for node in nodes {
            for input in &node.inputs {
                dependents
                    .get_mut(input)
                    .ok_or(WorkspaceError::DerivationSemanticMismatch)?
                    .push(node.node_digest.clone());
            }
        }
        for children in dependents.values_mut() {
            children.sort();
        }
        let mut ready = indegree
            .iter()
            .filter_map(|(node, degree)| (*degree == 0).then_some(node.clone()))
            .collect::<BTreeSet<_>>();
        let mut topological = Vec::with_capacity(nodes.len());
        while let Some(node) = ready.pop_first() {
            topological.push(node.clone());
            for dependent in &dependents[&node] {
                let degree = indegree
                    .get_mut(dependent)
                    .ok_or(WorkspaceError::DerivationSemanticMismatch)?;
                *degree = degree
                    .checked_sub(1)
                    .ok_or(WorkspaceError::DerivationSemanticMismatch)?;
                if *degree == 0 {
                    ready.insert(dependent.clone());
                }
            }
        }
        if topological.len() != nodes.len() {
            return Err(WorkspaceError::DerivationSemanticMismatch);
        }
        Ok(Self {
            nodes: by_digest,
            dependents,
            topological,
        })
    }

    fn root_sets(
        &self,
        seeds: BTreeMap<WorkspaceDerivationNodeDigest, BTreeSet<WorkspaceDerivationRootDigest>>,
    ) -> Result<
        BTreeMap<WorkspaceDerivationSubjectV1, BTreeSet<WorkspaceDerivationRootDigest>>,
        WorkspaceError,
    > {
        if seeds.keys().any(|seed| !self.nodes.contains_key(seed)) {
            return Err(WorkspaceError::DerivationSemanticMismatch);
        }
        let mut reachable = self
            .nodes
            .keys()
            .map(|node| (node.clone(), seeds.get(node).cloned().unwrap_or_default()))
            .collect::<BTreeMap<_, _>>();
        for node in &self.topological {
            let roots = reachable[node].clone();
            for dependent in &self.dependents[node] {
                reachable
                    .get_mut(dependent)
                    .ok_or(WorkspaceError::DerivationSemanticMismatch)?
                    .extend(roots.iter().cloned());
            }
        }
        Ok(self
            .nodes
            .iter()
            .map(|(digest, node)| (node.subject.clone(), reachable[digest].clone()))
            .collect())
    }

    /// Return the shortest path, breaking equal-length ties lexicographically.
    ///
    /// Breadth-first discovery visits the already-sorted dependent lists in
    /// canonical path order. Retaining only one predecessor per discovered
    /// node keeps the index linear in the proof graph; the selected witness is
    /// reconstructed once after the target is reached.
    fn shortest_path(
        &self,
        seed: &WorkspaceDerivationNodeDigest,
        target: &WorkspaceDerivationNodeDigest,
    ) -> Result<Vec<WorkspaceDerivationNodeV1>, WorkspaceError> {
        if !self.nodes.contains_key(seed) || !self.nodes.contains_key(target) {
            return Err(WorkspaceError::DerivationSemanticMismatch);
        }
        let mut queue = VecDeque::from([seed.clone()]);
        let mut predecessor = BTreeMap::from([(seed.clone(), None)]);
        while let Some(node) = queue.pop_front() {
            if &node == target {
                break;
            }
            for dependent in &self.dependents[&node] {
                if !predecessor.contains_key(dependent) {
                    predecessor.insert(dependent.clone(), Some(node.clone()));
                    queue.push_back(dependent.clone());
                }
            }
        }

        let mut reverse_path = Vec::new();
        let mut cursor = target;
        loop {
            reverse_path.push(cursor.clone());
            match predecessor
                .get(cursor)
                .ok_or(WorkspaceError::DerivationSemanticMismatch)?
            {
                Some(parent) => cursor = parent,
                None => break,
            }
        }
        reverse_path.reverse();
        reverse_path
            .into_iter()
            .map(|digest| {
                self.nodes
                    .get(&digest)
                    .map(|node| (*node).clone())
                    .ok_or(WorkspaceError::DerivationSemanticMismatch)
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/workspace/universe/derivation.rs"]
mod tests;
