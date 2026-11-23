//! Exercises resident-definition lowering, validation, and dependency-sensitive successor classification.

use super::types::{
    operational_relationship, relationship_is_valid, PendingResidentEdge, ResidentNodeQuery,
};
use super::*;
use crate::resident_fixture::{
    applied_evidence, compile_live, live_workspace, role_package_fixture, synthetic_set_policy,
};

use crate::composition_source_fixtures;
use crate::resident::generation::tests::{
    rename_repository, retarget_workspace_identity, revise_composition_source,
    revise_profile_policy_description, select_composition_source, set_responsibility_selection,
};
use crate::resident::generation::{
    resident_definition_coordinate, resident_evidence_coordinate, GenerationInputChange,
    ResidentDefinitionCoordinate,
};
use pale_core::records::{
    ArtifactDigest, CapabilitiesFile, CompiledGenerationDigest, CompositionEdgeV1,
    DeclarationDependencyKindV1, EffectClass, LifecycleBehaviorIdV1, LogicalRepositoryIdV1,
    MachineBindingFile, PackageIdV1, PackageResourceIdV1, PackageResourceRefV1,
    PackageResourceSelectorV1, ParticipantRefV1, PathBudgetV1, RepositoryRevisionV1,
    RepositorySourceFrontierV1, WorkspaceDeclarationId, WorkspaceDeclarationRefV1,
    WorkspaceDerivationRuleV1, WorkspaceDerivationSubjectV1, WorkspaceManifestFile,
    WorkspaceModuleId, WorkspaceNamespaceId,
};
use pale_kernel::Direction;

fn repository_id(value: &str) -> LogicalRepositoryIdV1 {
    LogicalRepositoryIdV1::parse(value).expect("repository id")
}

fn configure_support_package(
    workspace: &crate::resident_fixture::SyntheticWorkspace,
    selected: bool,
) {
    let source = std::fs::read_to_string(&workspace.manifest).expect("workspace manifest");
    let mut manifest: WorkspaceManifestFile =
        toml::from_str(&source).expect("typed workspace manifest");
    let project = manifest
        .workspace
        .desired_topology
        .projects
        .first_mut()
        .expect("fixture project");
    project.package_seeds = ["example.engine"]
        .into_iter()
        .chain(selected.then_some("example.support"))
        .map(|package| PackageIdV1::parse(package).expect("Package id"))
        .collect();
    std::fs::write(
        &workspace.manifest,
        toml::to_string(&manifest).expect("serialize workspace manifest"),
    )
    .expect("write Package selection");

    let path = workspace.root.path().join("workspace/capabilities.toml");
    let source = std::fs::read_to_string(&path).expect("capability policy");
    let mut capabilities: CapabilitiesFile =
        toml::from_str(&source).expect("typed capability policy");
    capabilities
        .semantic
        .roles
        .retain(|selector| match selector {
            PackageResourceSelectorV1::Exact { resource } => {
                resource.package_id.as_str() != "example.support"
            }
            PackageResourceSelectorV1::AllFromPackage { package_id } => {
                package_id.as_str() != "example.support"
            }
        });
    if selected {
        capabilities
            .semantic
            .roles
            .push(PackageResourceSelectorV1::Exact {
                resource: PackageResourceRefV1 {
                    package_id: PackageIdV1::parse("example.support").expect("Package id"),
                    resource_id: PackageResourceIdV1::parse("engine").expect("resource id"),
                },
            });
    }
    std::fs::write(
        path,
        toml::to_string(&capabilities).expect("serialize capability policy"),
    )
    .expect("write semantic role selection");
}

fn retarget_machine_repository(workspace: &crate::resident_fixture::SyntheticWorkspace) {
    let alternate = workspace.root.path().join("repository-alternate");
    std::fs::create_dir_all(&alternate).expect("alternate repository");
    let source = std::fs::read_to_string(&workspace.machine_binding).expect("machine binding");
    let mut binding: MachineBindingFile = toml::from_str(&source).expect("typed machine binding");
    binding
        .repository_checkouts
        .iter_mut()
        .find(|checkout| checkout.repository_id == repository_id("engine"))
        .expect("fixture repository checkout")
        .checkout_root = alternate.to_string_lossy().into_owned();
    std::fs::write(
        &workspace.machine_binding,
        toml::to_string(&binding).expect("serialize machine binding"),
    )
    .expect("retarget machine repository");
}

fn resident_store(compilation: crate::compile::Compilation) -> ResidentDefinitionStore {
    let evidence = applied_evidence(&compilation).into_materialization();
    resident_store_with(compilation, evidence)
}

fn resident_store_with(
    compilation: crate::compile::Compilation,
    evidence: AppliedRepositoryMaterialization,
) -> ResidentDefinitionStore {
    let owner = CompiledGenerationDigest::hash_canonical(&(
        "test.resident-definition-owner.v1",
        compilation.one_shot_compilation_digest(),
    ))
    .expect("test owner digest");
    ResidentDefinitionStore::build(
        owner,
        compilation.into_resident_parts(),
        vec![evidence],
        synthetic_set_policy(),
    )
    .expect("resident definition store")
}

fn graph_snapshot(store: &ResidentDefinitionStore) -> (Vec<String>, Vec<String>) {
    let nodes = (0..store.graph.node_count())
        .map(|index| {
            let handle = store
                .graph
                .node_at(u32::try_from(index).expect("bounded node index"))
                .expect("node handle");
            format!(
                "{:?}:{:?}",
                store.graph.node_key(handle).expect("node key"),
                store.graph.node_payload(handle).expect("node kind")
            )
        })
        .collect();
    let edges = (0..store.graph.edge_count())
        .map(|index| {
            let handle = store
                .graph
                .edge_at(u32::try_from(index).expect("bounded edge index"))
                .expect("edge handle");
            let (_, source, target) = store.graph.edge_identity(handle).expect("edge identity");
            let (evidence, relation) = store.graph.edge_payload(handle).expect("edge payload");
            format!(
                "{:?}->{:?}:{relation:?}:{evidence:?}",
                store.graph.node_key(source).expect("source key"),
                store.graph.node_key(target).expect("target key")
            )
        })
        .collect();
    (nodes, edges)
}

#[test]
fn lowering_is_deterministic_aligned_and_lookup_equivalent() {
    let (workspace, probe) = live_workspace();
    let first = resident_store(compile_live(&workspace, &probe));
    let second = resident_store(compile_live(&workspace, &probe));

    assert_eq!(graph_snapshot(&first), graph_snapshot(&second));
    assert_ne!(first.graph.generation(), second.graph.generation());
    for index in 0..first.graph.node_count() {
        let handle = first
            .graph
            .node_at(u32::try_from(index).expect("bounded node index"))
            .expect("node handle");
        assert_eq!(
            first.graph.node_payload(handle).expect("node kind"),
            &first.resolve(handle).expect("aligned body").kind()
        );
        assert!(matches!(
            second.graph.node_key(handle),
            Err(GraphAccessError::ForeignGeneration)
        ));
    }

    let repository_id = repository_id("engine");
    let repository = first.repository(&repository_id).expect("repository lookup");
    for operation in &repository.semantic_plan().operations {
        assert!(first
            .operation(&repository_id, &operation.record.id)
            .is_some());
    }
    for mode in &repository.semantic_plan().run_modes {
        assert!(first.run_mode(&repository_id, &mode.record.id).is_some());
    }
    for support in &repository.render().support {
        let selection = &support.execution_path_witness.selection;
        let selection_key = ResidentNodeKey::ExecutionPathSelection(selection.id.clone());
        let selection_handle = first
            .graph
            .find(&selection_key)
            .expect("execution-path selection node");
        let ResidentDefinition::ExecutionPathSelection { profile, position } =
            first.resolve(selection_handle).expect("selection body")
        else {
            panic!("selection node has the wrong body kind");
        };
        let ResidentDefinition::Profile(profile) =
            first.resolve(*profile).expect("selection profile")
        else {
            panic!("selection owner has the wrong body kind");
        };
        assert_eq!(profile.execution_paths.get(*position), Some(selection));

        let dependencies = first
            .graph
            .edges_directed(selection_handle, Direction::Outgoing)
            .expect("selection dependencies")
            .map(|edge| {
                let (_, _, target) = first.graph.edge_identity(edge).expect("edge identity");
                let (_, relation) = first.graph.edge_payload(edge).expect("edge payload");
                (
                    *relation,
                    first.graph.node_key(target).expect("target key").clone(),
                )
            })
            .collect::<Vec<_>>();
        assert!(dependencies.contains(&(
            ResidentRelation::ExecutionPathSelectionProfile,
            ResidentNodeKey::Profile(profile.profile_digest.clone()),
        )));
        assert!(dependencies.contains(&(
            ResidentRelation::ExecutionPathSelectionExperiencePath,
            ResidentNodeKey::ExperiencePath(selection.experience_path_id.clone()),
        )));
    }
}

#[test]
fn canonical_key_coordinate_and_borrowed_query_orders_are_identical() {
    let (workspace, probe) = live_workspace();
    select_composition_source(&workspace);
    let store = resident_store(compile_live(&workspace, &probe));

    let handles = (0..store.graph.node_count())
        .map(|index| {
            store
                .graph
                .node_at(u32::try_from(index).expect("bounded node index"))
                .expect("node handle")
        })
        .collect::<Vec<_>>();
    let resident_keys = handles
        .iter()
        .map(|handle| store.graph.node_key(*handle).expect("resident key"))
        .collect::<Vec<_>>();
    let representative_resource = resident_keys
        .iter()
        .find_map(|key| match key {
            ResidentNodeKey::PackageResource(resource) => Some(resource.clone()),
            _ => None,
        })
        .expect("resident Package resource");
    let mut keys = resident_keys
        .iter()
        .map(|key| (*key).clone())
        .collect::<Vec<_>>();
    keys.extend([
        ResidentNodeKey::WorkspaceDeclaration {
            owner_module: WorkspaceModuleId::parse("ordering").expect("module id"),
            declaration: WorkspaceDeclarationRefV1 {
                namespace: WorkspaceNamespaceId::parse("ordering").expect("namespace id"),
                declaration_id: WorkspaceDeclarationId::parse("representative")
                    .expect("declaration id"),
            },
        },
        ResidentNodeKey::Operation {
            repository: repository_id("ordering"),
            operation: representative_resource.resource.clone(),
        },
        ResidentNodeKey::RunMode {
            repository: repository_id("ordering"),
            mode: representative_resource.resource,
        },
        ResidentNodeKey::Lifecycle(LifecycleBehaviorIdV1::parse("ordering").expect("lifecycle id")),
    ]);
    keys.sort();
    let coordinates = keys
        .iter()
        .map(|key| resident_definition_coordinate(key).expect("public semantic coordinate"))
        .collect::<Vec<_>>();

    for (key, coordinate) in keys.iter().zip(&coordinates) {
        assert_eq!(key.cmp_coordinate(coordinate), std::cmp::Ordering::Equal);
    }
    for ((left, right), right_coordinate) in keys
        .windows(2)
        .map(|pair| (&pair[0], &pair[1]))
        .zip(coordinates.iter().skip(1))
    {
        assert_eq!(left.cmp(right), left.cmp_coordinate(right_coordinate));
    }

    let mut present_kinds = handles
        .iter()
        .map(|handle| *store.graph.node_payload(*handle).expect("resident kind"))
        .collect::<std::collections::BTreeSet<_>>();
    present_kinds.extend([
        ResidentNodeKind::WorkspaceDeclaration,
        ResidentNodeKind::Operation,
        ResidentNodeKind::RunMode,
        ResidentNodeKind::Lifecycle,
    ]);
    assert_eq!(
        present_kinds,
        std::collections::BTreeSet::from([
            ResidentNodeKind::Repository,
            ResidentNodeKind::Profile,
            ResidentNodeKind::ProjectInstance,
            ResidentNodeKind::ExecutionPathSelection,
            ResidentNodeKind::PackageResource,
            ResidentNodeKind::PackageClosure,
            ResidentNodeKind::WorkspaceSemanticIr,
            ResidentNodeKind::WorkspaceAdmissionEvidence,
            ResidentNodeKind::PrincipalUniverse,
            ResidentNodeKind::CompositionPlan,
            ResidentNodeKind::WorkspaceDeclaration,
            ResidentNodeKind::WorkspaceCausal,
            ResidentNodeKind::Responsibility,
            ResidentNodeKind::Participant,
            ResidentNodeKind::ProviderBinding,
            ResidentNodeKind::ExperiencePath,
            ResidentNodeKind::EntryContract,
            ResidentNodeKind::RequestTranslation,
            ResidentNodeKind::ResultTranslation,
            ResidentNodeKind::TerminationContract,
            ResidentNodeKind::SettlementContract,
            ResidentNodeKind::ContinuityContract,
            ResidentNodeKind::CompiledContent,
            ResidentNodeKind::SourceContent,
            ResidentNodeKind::Operation,
            ResidentNodeKind::RunMode,
            ResidentNodeKind::Lifecycle,
        ])
    );

    for key in &keys {
        let query = match key {
            ResidentNodeKey::Repository(repository) => {
                Some(ResidentNodeQuery::Repository(repository))
            }
            ResidentNodeKey::CompositionPlan(digest) => {
                Some(ResidentNodeQuery::CompositionPlan(digest))
            }
            ResidentNodeKey::Operation {
                repository,
                operation,
            } => Some(ResidentNodeQuery::Operation {
                repository,
                operation,
            }),
            ResidentNodeKey::RunMode { repository, mode } => {
                Some(ResidentNodeQuery::RunMode { repository, mode })
            }
            ResidentNodeKey::Lifecycle(behavior) => Some(ResidentNodeQuery::Lifecycle(behavior)),
            _ => None,
        };
        if let Some(query) = query {
            assert_eq!(key.cmp_query(query), std::cmp::Ordering::Equal);
            if let Some(handle) = store.graph.find(key) {
                assert_eq!(store.find_query(query), Some(handle));
            }
        }
    }
}

#[test]
fn canonical_relationships_coalesce_only_derived_idempotent_edges() {
    let derived = || PendingResidentEdge {
        source: ResidentNodeKey::Repository(repository_id("owner")),
        target: ResidentNodeKey::Repository(repository_id("dependency")),
        relation: ResidentRelation::RepositoryProfile,
        evidence: ResidentEdgeEvidence::None,
    };
    let mut edges = vec![derived(), derived()];
    canonicalize_edges(&mut edges).expect("derived duplicate coalesces");
    assert_eq!(edges.len(), 1);

    let evidenced = || PendingResidentEdge {
        source: ResidentNodeKey::Repository(repository_id("owner")),
        target: ResidentNodeKey::Repository(repository_id("dependency")),
        relation: ResidentRelation::WorkspaceExplicitDependency,
        evidence: ResidentEdgeEvidence::WorkspaceDependency(DeclarationDependencyKindV1::Positive),
    };
    let mut duplicates = vec![evidenced(), evidenced()];
    assert!(matches!(
        canonicalize_edges(&mut duplicates),
        Err(DefinitionStoreError::DuplicateRelationship(_))
    ));

    let mut parallel = vec![
        evidenced(),
        PendingResidentEdge {
            evidence: ResidentEdgeEvidence::WorkspaceDependency(
                DeclarationDependencyKindV1::Negation,
            ),
            ..evidenced()
        },
    ];
    canonicalize_edges(&mut parallel).expect("distinct evidence remains parallel");
    assert_eq!(parallel.len(), 2);
}

#[test]
fn successor_impact_is_semantic_complete_and_preserves_unaffected_regions() {
    let (workspace, probe) = live_workspace();
    let prior = resident_store(compile_live(&workspace, &probe));

    retarget_workspace_identity(&workspace, "resident-impact-retargeted");
    let current_compilation = compile_live(&workspace, &probe);
    let receipt = current_compilation
        .workspace_derivation_receipt(prior.derivation())
        .expect("exact workspace receipt");
    let current = resident_store(current_compilation);
    let impact = current
        .successor_impact(&prior, &receipt)
        .expect("semantic successor impact");

    assert_eq!(
        impact.affected().len() + impact.unaffected().len(),
        current.graph.node_count()
    );
    let explained = impact
        .affected()
        .iter()
        .find_map(|target| {
            current
                .impact_reason_path(&impact.closure, target)
                .expect("resident reason lookup")
                .filter(|path| !path.steps().is_empty())
        })
        .expect("non-trivial resident reason path");
    assert_eq!(explained.nodes().last(), Some(explained.target()));
    assert_eq!(explained.nodes().len(), explained.steps().len() + 1);
    for (nodes, step) in explained.nodes().windows(2).zip(explained.steps()) {
        let dependency = store_handle_for_coordinate(&current, &nodes[0]);
        let owner = store_handle_for_coordinate(&current, &nodes[1]);
        assert!((0..current.graph.edge_count()).any(|index| {
            let edge = current
                .graph
                .edge_at(u32::try_from(index).expect("bounded edge index"))
                .expect("resident edge");
            let (_, candidate_owner, candidate_dependency) =
                current.graph.edge_identity(edge).expect("edge identity");
            let (evidence, relation) = current.graph.edge_payload(edge).expect("edge payload");
            candidate_owner == owner
                && candidate_dependency == dependency
                && relation == &step.relation()
                && resident_evidence_coordinate(evidence) == *step.evidence()
        }));
    }
    assert_eq!(
        current
            .impact_reason_path(&impact.closure, explained.target())
            .expect("repeat resident reason lookup"),
        Some(explained)
    );
    assert!(!impact.unaffected().is_empty());
    assert!(impact
        .input_changes()
        .iter()
        .any(|change| matches!(change, GenerationInputChange::WorkspaceSemanticIr { .. })));
}

fn store_handle_for_coordinate(
    store: &ResidentDefinitionStore,
    coordinate: &ResidentDefinitionCoordinate,
) -> pale_kernel::NodeHandle {
    store
        .graph
        .find_by(|key| key.cmp_coordinate(coordinate))
        .expect("resident coordinate handle")
}

fn package_resources(store: &ResidentDefinitionStore) -> Vec<crate::packages::PackageResourceUse> {
    (0..store.graph.node_count())
        .filter_map(|index| {
            let handle = store
                .graph
                .node_at(u32::try_from(index).expect("bounded node index"))
                .expect("resident node handle");
            match store.graph.node_key(handle).expect("resident node key") {
                ResidentNodeKey::PackageResource(resource) => Some(resource.clone()),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn package_resource_impact_preserves_seed_custody_and_precise_change() {
    let (workspace, probe) = live_workspace();
    let distribution = workspace.root.path().join("distribution");
    role_package_fixture::write_role_package(
        &distribution,
        "support",
        "example.support",
        "# Support\n\nIndependent unchanged Package body.\n",
    );
    configure_support_package(&workspace, true);
    workspace.refresh_distribution_lock();
    let prior = resident_store(compile_live(&workspace, &probe));
    let prior_resources = package_resources(&prior);
    assert!(prior_resources.len() > 1);

    std::fs::write(
        workspace
            .root
            .path()
            .join("distribution/packages/engine/role.md"),
        "# Engine\n\nChanged resident Package body.\n",
    )
    .expect("rewrite selected Package body");
    workspace.refresh_distribution_lock();
    let current_compilation = compile_live(&workspace, &probe);
    let receipt = current_compilation
        .workspace_derivation_receipt(prior.derivation())
        .expect("workspace receipt");
    let current = resident_store(current_compilation);

    let impact = current
        .successor_impact(&prior, &receipt)
        .expect("Package successor impact");
    let changed = impact
        .input_changes()
        .iter()
        .filter_map(|change| match change {
            GenerationInputChange::PackageResourceModified { prior, current } => {
                assert_eq!(prior.resource, current.resource);
                assert_ne!(prior.definition_sha256, current.definition_sha256);
                Some(&current.resource)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(changed.len(), 1);
    let unrelated = prior_resources
        .iter()
        .find(|resource| !changed.contains(resource))
        .expect("unrelated Package resource");
    assert!(impact.unaffected().contains(
        &crate::resident::generation::resident_definition_coordinate(
            &ResidentNodeKey::PackageResource((*unrelated).clone())
        )
        .expect("semantic resource coordinate")
    ));
}

#[test]
fn populated_composition_is_owned_once_with_exact_operational_dependencies() {
    let (workspace, probe) = live_workspace();
    select_composition_source(&workspace);
    let store = resident_store(compile_live(&workspace, &probe));

    let mut composition_counts = std::collections::BTreeMap::new();
    for index in 0..store.graph.node_count() {
        let handle = store
            .graph
            .node_at(u32::try_from(index).expect("bounded node index"))
            .expect("node handle");
        let kind = *store.graph.node_payload(handle).expect("node kind");
        if matches!(
            kind,
            ResidentNodeKind::Responsibility
                | ResidentNodeKind::Participant
                | ResidentNodeKind::ProviderBinding
                | ResidentNodeKind::ExperiencePath
                | ResidentNodeKind::EntryContract
                | ResidentNodeKind::RequestTranslation
                | ResidentNodeKind::ResultTranslation
                | ResidentNodeKind::TerminationContract
                | ResidentNodeKind::SettlementContract
                | ResidentNodeKind::ContinuityContract
                | ResidentNodeKind::CompiledContent
                | ResidentNodeKind::SourceContent
        ) {
            *composition_counts.entry(kind).or_insert(0_usize) += 1;
        }
        assert_eq!(kind, store.resolve(handle).expect("aligned body").kind());
    }
    assert_eq!(
        composition_counts,
        std::collections::BTreeMap::from([
            (ResidentNodeKind::Responsibility, 2),
            (ResidentNodeKind::Participant, 9),
            (ResidentNodeKind::ProviderBinding, 1),
            (ResidentNodeKind::ExperiencePath, 1),
            (ResidentNodeKind::EntryContract, 1),
            (ResidentNodeKind::RequestTranslation, 1),
            (ResidentNodeKind::ResultTranslation, 1),
            (ResidentNodeKind::TerminationContract, 1),
            (ResidentNodeKind::SettlementContract, 1),
            (ResidentNodeKind::ContinuityContract, 1),
            (ResidentNodeKind::CompiledContent, 1),
            (ResidentNodeKind::SourceContent, 1),
        ])
    );

    let entry = composition_source_fixtures::participant("entry");
    let worker = composition_source_fixtures::participant("worker");
    let terminal = ParticipantRefV1::TerminalLeaf {
        terminal_leaf_id: "terminal-output".into(),
    };
    let expected = [
        (
            ResidentNodeKey::ContinuityContract("continuity-entry".into()),
            ResidentNodeKey::Participant(entry.clone()),
            ResidentRelation::ContinuityOwner,
            None,
        ),
        (
            ResidentNodeKey::ExperiencePath("direct-claude-code".into()),
            ResidentNodeKey::ContinuityContract("continuity-entry".into()),
            ResidentRelation::PathContinuityContract,
            None,
        ),
        (
            ResidentNodeKey::Participant(worker.clone()),
            ResidentNodeKey::Participant(terminal),
            ResidentRelation::TerminalLeaf,
            Some("terminal-worker-output"),
        ),
        (
            ResidentNodeKey::Participant(worker),
            ResidentNodeKey::Participant(entry),
            ResidentRelation::ObservationSettlement,
            Some("observe-entry"),
        ),
        (
            ResidentNodeKey::Participant(composition_source_fixtures::participant("worker")),
            ResidentNodeKey::Participant(composition_source_fixtures::participant("entry")),
            ResidentRelation::ObservationSettlement,
            Some("observe-entry-again"),
        ),
    ];
    for (owner, dependency, relation, evidence_id) in expected {
        let found = (0..store.graph.edge_count()).any(|index| {
            let handle = store
                .graph
                .edge_at(u32::try_from(index).expect("bounded edge index"))
                .expect("edge handle");
            let (_, source, target) = store.graph.edge_identity(handle).expect("edge identity");
            let (evidence, candidate) = store.graph.edge_payload(handle).expect("edge payload");
            store.graph.node_key(source).expect("owner key") == &owner
                && store.graph.node_key(target).expect("dependency key") == &dependency
                && candidate == &relation
                && evidence_id.is_none_or(|id| {
                    matches!(evidence, ResidentEdgeEvidence::OperationalContract(edge) if edge.id() == id)
                })
        });
        assert!(
            found,
            "missing {relation:?} from {owner:?} to {dependency:?}"
        );
    }

    let plan = (0..store.graph.node_count())
        .find_map(|index| {
            let handle = store.graph.node_at(u32::try_from(index).ok()?).ok()?;
            match store.resolve(handle).ok()? {
                ResidentDefinition::CompositionPlan(plan) => Some(plan),
                _ => None,
            }
        })
        .expect("retained composition plan");
    assert_eq!(
        plan.non_operational_edges
            .iter()
            .map(CompositionEdgeV1::id)
            .collect::<Vec<_>>(),
        ["plugin-worker-entry", "ui-entry-worker"]
    );
    assert!(plan.graph_views.iter().any(|view| !view.edges.is_empty()));
    assert!((0..store.graph.edge_count()).all(|index| {
        let handle = store
            .graph
            .edge_at(u32::try_from(index).expect("bounded edge index"))
            .expect("edge handle");
        let (evidence, _) = store.graph.edge_payload(handle).expect("edge payload");
        !matches!(
            evidence,
            ResidentEdgeEvidence::OperationalContract(edge)
                if matches!(
                    edge.as_ref(),
                    CompositionEdgeV1::UiEmbedding { .. }
                        | CompositionEdgeV1::PluginDependency { .. }
                )
        )
    }));
}

#[test]
fn successor_impact_seeds_profile_instance_composition_and_repository_evidence() {
    let (workspace, probe) = live_workspace();
    let prior = resident_store(compile_live(&workspace, &probe));
    revise_profile_policy_description(&workspace, "Changed profile policy");
    let compilation = compile_live(&workspace, &probe);
    let receipt = compilation
        .workspace_derivation_receipt(prior.derivation())
        .expect("profile receipt");
    let current = resident_store(compilation);
    let impact = current
        .successor_impact(&prior, &receipt)
        .expect("profile impact");
    assert!(impact
        .input_changes()
        .iter()
        .any(|change| matches!(change, GenerationInputChange::Profile { .. })));

    let (workspace, probe) = live_workspace();
    let prior = resident_store(compile_live(&workspace, &probe));
    retarget_machine_repository(&workspace);
    let compilation = compile_live(&workspace, &probe);
    let receipt = compilation
        .workspace_derivation_receipt(prior.derivation())
        .expect("instance receipt");
    let current = resident_store(compilation);
    let impact = current
        .successor_impact(&prior, &receipt)
        .expect("instance impact");
    assert!(impact
        .input_changes()
        .iter()
        .any(|change| matches!(change, GenerationInputChange::ProjectInstance { .. })));

    let (workspace, probe) = live_workspace();
    let prior = resident_store(compile_live(&workspace, &probe));
    revise_composition_source(&workspace);
    let compilation = compile_live(&workspace, &probe);
    let receipt = compilation
        .workspace_derivation_receipt(prior.derivation())
        .expect("composition receipt");
    let current = resident_store(compilation);
    let impact = current
        .successor_impact(&prior, &receipt)
        .expect("composition impact");
    assert!(impact
        .input_changes()
        .iter()
        .any(|change| matches!(change, GenerationInputChange::CompositionPlan { .. })));

    let (workspace, probe) = live_workspace();
    let prior = resident_store(compile_live(&workspace, &probe));
    let compilation = compile_live(&workspace, &probe);
    let receipt = compilation
        .workspace_derivation_receipt(prior.derivation())
        .expect("repository-evidence receipt");
    let evidence = applied_evidence(&compilation)
        .with_source_frontier(RepositorySourceFrontierV1 {
            repository_id: repository_id("engine"),
            base_revision: RepositoryRevisionV1::parse("base-revision").expect("base revision"),
            tracked_delta_sha256: ArtifactDigest::hash_bytes(b"changed tracked delta"),
            untracked_inventory_sha256: ArtifactDigest::hash_bytes(b"untracked inventory"),
            dependency_frontier_sha256: ArtifactDigest::hash_bytes(b"dependency frontier"),
        })
        .into_materialization();
    let current = resident_store_with(compilation, evidence);
    let impact = current
        .successor_impact(&prior, &receipt)
        .expect("repository-evidence impact");
    assert!(impact
        .input_changes()
        .iter()
        .any(|change| matches!(change, GenerationInputChange::RepositoryModified { .. })));
    assert!(!impact.input_changes().iter().any(|change| matches!(
        change,
        GenerationInputChange::Profile { .. }
            | GenerationInputChange::ProjectInstance { .. }
            | GenerationInputChange::PackageClosure { .. }
            | GenerationInputChange::WorkspaceSemanticIr { .. }
            | GenerationInputChange::CompositionPlan { .. }
    )));
    assert!(impact
        .unaffected()
        .iter()
        .any(|coordinate| matches!(coordinate, ResidentDefinitionCoordinate::PackageResource(_))));
}

#[test]
fn removed_workspace_and_repository_roots_remain_typed_without_invented_nodes() {
    let (workspace, probe) = live_workspace();
    set_responsibility_selection(&workspace, true);
    let prior = resident_store(compile_live(&workspace, &probe));
    set_responsibility_selection(&workspace, false);
    let compilation = compile_live(&workspace, &probe);
    let receipt = compilation
        .workspace_derivation_receipt(prior.derivation())
        .expect("removed workspace root receipt");
    let current = resident_store(compilation);
    let impact = current
        .successor_impact(&prior, &receipt)
        .expect("removed workspace root impact");
    assert!(impact.input_changes().iter().any(|change| matches!(
        change,
        GenerationInputChange::WorkspaceRoot { change, .. }
            if matches!(
                change.as_ref(),
                pale_core::records::WorkspaceDerivationRootChangeV1::Removed { .. }
            )
    )));
    assert!(!impact.removed_workspace_roots().is_empty());
    assert!(impact
        .input_changes()
        .iter()
        .any(|change| matches!(change, GenerationInputChange::WorkspaceSemanticIr { .. })));
    assert!(impact
        .unaffected()
        .iter()
        .any(|coordinate| matches!(coordinate, ResidentDefinitionCoordinate::PackageResource(_))));

    let (workspace, probe) = live_workspace();
    let prior = resident_store(compile_live(&workspace, &probe));
    rename_repository(&workspace, "replacement");
    let compilation = compile_live(&workspace, &probe);
    let receipt = compilation
        .workspace_derivation_receipt(prior.derivation())
        .expect("repository replacement receipt");
    let current = resident_store(compilation);
    let impact = current
        .successor_impact(&prior, &receipt)
        .expect("repository replacement impact");
    assert!(impact.input_changes().iter().any(
        |change| matches!(change, GenerationInputChange::RepositoryAdded { current } if current.repository_id.as_str() == "replacement")
    ));
    assert_eq!(impact.removed_repositories().len(), 1);
    assert_eq!(
        impact.removed_repositories()[0]
            .prior()
            .repository_id
            .as_str(),
        "engine"
    );
    assert!(current.repository(&repository_id("engine")).is_none());
}

#[test]
fn removed_package_resource_keeps_predecessor_bridge_and_unrelated_resource() {
    let (workspace, probe) = live_workspace();
    let distribution = workspace.root.path().join("distribution");
    role_package_fixture::write_role_package(
        &distribution,
        "support",
        "example.support",
        "# Support\n\nIndependent Package body.\n",
    );
    configure_support_package(&workspace, true);
    workspace.refresh_distribution_lock();
    let prior = resident_store(compile_live(&workspace, &probe));

    configure_support_package(&workspace, false);
    let compilation = compile_live(&workspace, &probe);
    let receipt = compilation
        .workspace_derivation_receipt(prior.derivation())
        .expect("Package removal receipt");
    let current = resident_store(compilation);
    let impact = current
        .successor_impact(&prior, &receipt)
        .expect("Package removal impact");
    assert_eq!(impact.removed_package_resources().len(), 1);
    assert_eq!(
        impact.removed_package_resources()[0]
            .prior()
            .resource
            .resource
            .package_id
            .as_str(),
        "example.support"
    );
    assert!(impact.unaffected().iter().any(|coordinate| matches!(
        coordinate,
        ResidentDefinitionCoordinate::PackageResource(resource)
            if resource.resource.package_id.as_str() == "example.engine"
    )));
}

#[test]
fn relationship_algebra_accepts_every_closed_family_and_rejects_wrong_proofs() {
    use ResidentNodeKind as Kind;
    use ResidentRelation as Relation;

    let ordinary = [
        (
            Relation::ExecutionPathSelectionProfile,
            Kind::ExecutionPathSelection,
            Kind::Profile,
        ),
        (
            Relation::ExecutionPathSelectionExperiencePath,
            Kind::ExecutionPathSelection,
            Kind::ExperiencePath,
        ),
        (
            Relation::RepositoryPackageScope,
            Kind::Repository,
            Kind::PackageResource,
        ),
        (
            Relation::OperationRepository,
            Kind::Operation,
            Kind::Repository,
        ),
        (Relation::RunModeRepository, Kind::RunMode, Kind::Repository),
        (
            Relation::RepositoryLifecycle,
            Kind::Repository,
            Kind::Lifecycle,
        ),
        (
            Relation::OperationPackageResource,
            Kind::Operation,
            Kind::PackageResource,
        ),
        (
            Relation::RunModePackageResource,
            Kind::RunMode,
            Kind::PackageResource,
        ),
        (
            Relation::PackageDependency,
            Kind::PackageResource,
            Kind::PackageResource,
        ),
        (
            Relation::PackageClosureResource,
            Kind::PackageClosure,
            Kind::PackageResource,
        ),
        (
            Relation::RepositoryPackageClosure,
            Kind::Repository,
            Kind::PackageClosure,
        ),
        (
            Relation::ProjectInstanceProfile,
            Kind::ProjectInstance,
            Kind::Profile,
        ),
        (
            Relation::ProfilePackageClosure,
            Kind::Profile,
            Kind::PackageClosure,
        ),
        (Relation::RepositoryProfile, Kind::Repository, Kind::Profile),
        (
            Relation::RepositoryProjectInstance,
            Kind::Repository,
            Kind::ProjectInstance,
        ),
        (
            Relation::PrincipalUniverseSemanticIr,
            Kind::PrincipalUniverse,
            Kind::WorkspaceSemanticIr,
        ),
        (
            Relation::PrincipalUniverseAdmissionEvidence,
            Kind::PrincipalUniverse,
            Kind::WorkspaceAdmissionEvidence,
        ),
        (
            Relation::WorkspaceSemanticIrSource,
            Kind::WorkspaceSemanticIr,
            Kind::WorkspaceCausal,
        ),
        (Relation::LifecycleProfile, Kind::Lifecycle, Kind::Profile),
        (
            Relation::CompositionDefinitionPlan,
            Kind::ExperiencePath,
            Kind::CompositionPlan,
        ),
        (
            Relation::WorkspaceRelationshipFrom,
            Kind::WorkspaceDeclaration,
            Kind::WorkspaceDeclaration,
        ),
        (
            Relation::WorkspaceRelationshipTo,
            Kind::WorkspaceDeclaration,
            Kind::WorkspaceDeclaration,
        ),
        (
            Relation::WorkspaceCausalInput(WorkspaceDerivationRuleV1::SemanticIrSealing),
            Kind::WorkspaceCausal,
            Kind::WorkspaceCausal,
        ),
        (
            Relation::CompositionSourceDependency,
            Kind::ContinuityContract,
            Kind::WorkspaceCausal,
        ),
        (
            Relation::ResponsibilityParticipant,
            Kind::Responsibility,
            Kind::Participant,
        ),
        (
            Relation::ProviderParticipant,
            Kind::ProviderBinding,
            Kind::Participant,
        ),
        (
            Relation::ProviderResponsibility,
            Kind::ProviderBinding,
            Kind::Responsibility,
        ),
        (
            Relation::PathResponsibility,
            Kind::ExperiencePath,
            Kind::Responsibility,
        ),
        (
            Relation::PathProvider,
            Kind::ExperiencePath,
            Kind::ProviderBinding,
        ),
        (
            Relation::PathEntryContract,
            Kind::ExperiencePath,
            Kind::EntryContract,
        ),
        (
            Relation::PathTerminationContract,
            Kind::ExperiencePath,
            Kind::TerminationContract,
        ),
        (
            Relation::PathSettlementContract,
            Kind::ExperiencePath,
            Kind::SettlementContract,
        ),
        (
            Relation::PathContinuityContract,
            Kind::ExperiencePath,
            Kind::ContinuityContract,
        ),
        (
            Relation::PathCompiledContent,
            Kind::ExperiencePath,
            Kind::CompiledContent,
        ),
        (
            Relation::PathSourceContent,
            Kind::ExperiencePath,
            Kind::SourceContent,
        ),
        (
            Relation::EntryParticipant,
            Kind::EntryContract,
            Kind::Participant,
        ),
        (
            Relation::EntryResponsibility,
            Kind::EntryContract,
            Kind::Responsibility,
        ),
        (
            Relation::RequestTranslationSource,
            Kind::RequestTranslation,
            Kind::Participant,
        ),
        (
            Relation::RequestTranslationTarget,
            Kind::RequestTranslation,
            Kind::Participant,
        ),
        (
            Relation::ResultTranslationSource,
            Kind::ResultTranslation,
            Kind::Participant,
        ),
        (
            Relation::ResultTranslationTarget,
            Kind::ResultTranslation,
            Kind::Participant,
        ),
        (
            Relation::TerminationOwner,
            Kind::TerminationContract,
            Kind::Participant,
        ),
        (
            Relation::TerminationLeaf,
            Kind::TerminationContract,
            Kind::Participant,
        ),
        (
            Relation::SettlementObserver,
            Kind::SettlementContract,
            Kind::Participant,
        ),
        (
            Relation::ContinuityOwner,
            Kind::ContinuityContract,
            Kind::Participant,
        ),
        (
            Relation::DelegationResponsibility,
            Kind::Participant,
            Kind::Responsibility,
        ),
        (
            Relation::OperationalRequestTranslation,
            Kind::Participant,
            Kind::RequestTranslation,
        ),
        (
            Relation::OperationalResultTranslation,
            Kind::Participant,
            Kind::ResultTranslation,
        ),
        (
            Relation::NativeCallProvider,
            Kind::Participant,
            Kind::ProviderBinding,
        ),
        (
            Relation::TerminalContract,
            Kind::Participant,
            Kind::TerminationContract,
        ),
        (
            Relation::ObservationSettlementContract,
            Kind::Participant,
            Kind::SettlementContract,
        ),
    ];
    for (relation, source_kind, target_kind) in ordinary {
        let target = if relation == Relation::CompositionSourceDependency {
            ResidentNodeKey::WorkspaceCausal(WorkspaceDerivationSubjectV1::ExperienceComposition)
        } else {
            ResidentNodeKey::Repository(repository_id("target"))
        };
        let edge = PendingResidentEdge {
            source: ResidentNodeKey::Repository(repository_id("source")),
            target,
            relation,
            evidence: ResidentEdgeEvidence::None,
        };
        assert!(
            relationship_is_valid(&edge, source_kind, target_kind),
            "{relation:?}"
        );
    }

    let explicit = PendingResidentEdge {
        source: ResidentNodeKey::Repository(repository_id("source")),
        target: ResidentNodeKey::Repository(repository_id("target")),
        relation: Relation::WorkspaceExplicitDependency,
        evidence: ResidentEdgeEvidence::WorkspaceDependency(DeclarationDependencyKindV1::Positive),
    };
    assert!(relationship_is_valid(
        &explicit,
        Kind::WorkspaceDeclaration,
        Kind::WorkspaceDeclaration,
    ));

    let from = ParticipantRefV1::Capability {
        capability_id: "from".into(),
    };
    let to = ParticipantRefV1::Capability {
        capability_id: "to".into(),
    };
    let operational = [
        CompositionEdgeV1::WorkDelegation {
            id: "work".into(),
            from: from.clone(),
            to: to.clone(),
            responsibility_contract_id: "responsibility".into(),
            request_translation_id: "request".into(),
            result_translation_id: "result".into(),
            authority_ceiling: vec![EffectClass::Read],
            budget: PathBudgetV1 {
                max_depth: 1,
                max_fan_out: 1,
                max_time_ms: 1,
                max_cost_units: 1,
                max_resource_units: 1,
                max_effect_count: 1,
                max_recursion_iterations: 1,
            },
        },
        CompositionEdgeV1::NativeApplicationCall {
            id: "call".into(),
            caller: from.clone(),
            application: to.clone(),
            provider_binding_id: "provider".into(),
            request_translation_id: "request".into(),
            result_translation_id: "result".into(),
        },
        CompositionEdgeV1::TerminalLeaf {
            id: "terminal".into(),
            owner: from.clone(),
            terminal_leaf: to.clone(),
            termination_contract_id: "termination".into(),
        },
        CompositionEdgeV1::ObservationSettlement {
            id: "observation".into(),
            subject: from.clone(),
            observer: to.clone(),
            settlement_contract_id: "settlement".into(),
        },
    ];
    for contract in operational {
        let (source, target, relation) =
            operational_relationship(&contract).expect("operational relation");
        let edge = PendingResidentEdge {
            source,
            target,
            relation,
            evidence: ResidentEdgeEvidence::OperationalContract(Box::new(contract.clone())),
        };
        assert!(relationship_is_valid(
            &edge,
            Kind::Participant,
            Kind::Participant
        ));
        assert!(!relationship_is_valid(
            &edge,
            Kind::PackageResource,
            Kind::Participant
        ));
        let mut wrong = edge;
        wrong.target = ResidentNodeKey::Participant(ParticipantRefV1::Capability {
            capability_id: "unrelated".into(),
        });
        assert!(!relationship_is_valid(
            &wrong,
            Kind::Participant,
            Kind::Participant
        ));
    }

    let wrong_kind = PendingResidentEdge {
        source: ResidentNodeKey::Repository(repository_id("source")),
        target: ResidentNodeKey::Repository(repository_id("target")),
        relation: Relation::RepositoryProfile,
        evidence: ResidentEdgeEvidence::None,
    };
    assert!(!relationship_is_valid(
        &wrong_kind,
        Kind::PackageResource,
        Kind::PackageResource
    ));
    let missing_evidence = PendingResidentEdge {
        source: ResidentNodeKey::Participant(from),
        target: ResidentNodeKey::Participant(to),
        relation: Relation::WorkDelegation,
        evidence: ResidentEdgeEvidence::None,
    };
    assert!(!relationship_is_valid(
        &missing_evidence,
        Kind::Participant,
        Kind::Participant
    ));
}
