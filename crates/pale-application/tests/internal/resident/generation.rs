//! Provides resident-generation fixtures and checks deterministic compilation and impact classification.

use super::*;
use crate::compile::compile_portable_inputs;
use crate::resident_fixture::{
    applied_evidence, compile_live, live_workspace, synthetic_set_policy, workspace_fixtures,
    AppliedEvidence, SurfaceExecutableProbe, SyntheticWorkspace,
};
use crate::responsibility_fixtures;
use pale_core::records::*;

#[path = "nonempty_candidate.rs"]
mod nonempty_candidate;

impl AppliedEvidence {
    pub(in crate::resident) fn with_source_frontier(
        mut self,
        record: RepositorySourceFrontierV1,
    ) -> Self {
        self.source_frontier = record;
        self
    }
}

fn logical_repository_id(value: &str) -> LogicalRepositoryIdV1 {
    LogicalRepositoryIdV1::parse(value).expect("repository id")
}

pub(in crate::resident) fn activation_compatibility(
    prior: Option<CompiledGenerationDigest>,
    current: CompiledGenerationDigest,
) -> GenerationActivationCompatibility {
    GenerationActivationCompatibility {
        current_generation: current,
        lineage: prior.map_or(GenerationActivationLineage::Genesis, |prior| {
            GenerationActivationLineage::Successor {
                prior_generation: prior,
            }
        }),
    }
}

#[test]
fn activation_compatibility_distinguishes_genesis_and_exact_successors() {
    let prior = CompiledGenerationDigest::hash_canonical(&"prior").expect("prior digest");
    let current = CompiledGenerationDigest::hash_canonical(&"current").expect("current digest");
    let genesis = activation_compatibility(None, prior.clone());
    assert!(genesis.admits(None, &prior));
    assert!(!genesis.admits(Some(&prior), &prior));

    let successor = activation_compatibility(Some(prior.clone()), current.clone());
    assert!(successor.admits(Some(&prior), &current));
    assert!(!successor.admits(None, &current));
    assert!(!successor.admits(Some(&current), &current));
}

#[test]
fn source_frontier_proof_retains_every_owner_observed_coordinate() {
    let (workspace, probe) = live_workspace();
    let compilation = compile_live(&workspace, &probe);
    let evidence = applied_evidence(&compilation);
    let record = evidence.source_frontier.clone();
    let admitted = evidence.into_materialization();
    assert_eq!(admitted.source_frontier(), &record);
}

pub(in crate::resident) fn revise_profile_policy_description(
    workspace: &SyntheticWorkspace,
    description: &str,
) {
    let path = workspace.root.path().join("workspace/capabilities.toml");
    let source = std::fs::read_to_string(&path).expect("capability policy");
    let mut capabilities: CapabilitiesFile =
        toml::from_str(&source).expect("typed capability policy");
    capabilities
        .run_policies
        .first_mut()
        .expect("fixture run policy")
        .description = description.to_owned();
    std::fs::write(
        path,
        toml::to_string(&capabilities).expect("serialize capability policy"),
    )
    .expect("rewrite profile policy");
}

pub(in crate::resident) fn rename_repository(workspace: &SyntheticWorkspace, repository_id: &str) {
    let source = std::fs::read_to_string(&workspace.manifest).expect("workspace manifest");
    let mut manifest: WorkspaceManifestFile =
        toml::from_str(&source).expect("typed workspace manifest");
    let prior = logical_repository_id("engine");
    let replacement = logical_repository_id(repository_id);
    manifest
        .workspace
        .desired_topology
        .repositories
        .iter_mut()
        .find(|repository| repository.id == prior)
        .expect("fixture repository declaration")
        .id = replacement.clone();
    for membership in &mut manifest.workspace.desired_topology.project_repositories {
        if membership.repository_id == prior {
            membership.repository_id = replacement.clone();
        }
    }
    for membership in &mut manifest
        .workspace
        .desired_topology
        .execution_workspace_repositories
    {
        if membership.repository_id == prior {
            membership.repository_id = replacement.clone();
        }
    }
    std::fs::write(
        &workspace.manifest,
        toml::to_string(&manifest).expect("serialize workspace manifest"),
    )
    .expect("rewrite repository identity");

    let source = std::fs::read_to_string(&workspace.machine_binding).expect("machine binding");
    let mut machine: MachineBindingFile = toml::from_str(&source).expect("typed machine binding");
    machine
        .repository_checkouts
        .iter_mut()
        .find(|checkout| checkout.repository_id == logical_repository_id("engine"))
        .expect("fixture repository checkout")
        .repository_id = logical_repository_id(repository_id);
    std::fs::write(
        &workspace.machine_binding,
        toml::to_string(&machine).expect("serialize machine binding"),
    )
    .expect("write machine binding");
}

pub(in crate::resident) fn retarget_workspace_identity(
    workspace: &SyntheticWorkspace,
    workspace_id: &str,
) {
    let source = std::fs::read_to_string(&workspace.manifest).expect("workspace manifest");
    let mut manifest: WorkspaceManifestFile =
        toml::from_str(&source).expect("typed workspace manifest");
    manifest.workspace.workspace_id = WorkspaceId::parse(workspace_id).expect("workspace identity");
    std::fs::write(
        &workspace.manifest,
        toml::to_string(&manifest).expect("serialize workspace manifest"),
    )
    .expect("write workspace manifest");
    workspace_fixtures::write_empty_universe(
        &workspace.root.path().join("workspace"),
        workspace_id,
    );
}

pub(in crate::resident) fn select_composition_source(workspace: &SyntheticWorkspace) {
    set_composition_selection(workspace, vec!["composition.toml"]);
}

pub(in crate::resident) fn revise_composition_source(workspace: &SyntheticWorkspace) {
    let path = workspace.root.path().join("workspace/composition.toml");
    let source = std::fs::read_to_string(&path).expect("composition source");
    let mut composition: WorkspaceCompositionSourceV1 =
        toml::from_str(&source).expect("typed composition source");
    let selected = composition
        .declarations
        .selected_experience_paths
        .first_mut()
        .expect("fixture experience path");
    let mut preimage = selected.to_preimage();
    preimage.assurance_subject_id = "assurance-subject-revised".into();
    *selected = ExperiencePathV1::seal(preimage).expect("revised experience path seals");
    std::fs::write(
        path,
        toml::to_string(&composition).expect("serialize composition source"),
    )
    .expect("write revised composition source");
}

pub(in crate::resident) fn set_composition_selection(
    workspace: &SyntheticWorkspace,
    paths: Vec<&str>,
) {
    let source = std::fs::read_to_string(&workspace.manifest).expect("workspace manifest");
    let mut manifest: WorkspaceManifestFile =
        toml::from_str(&source).expect("typed workspace manifest");
    manifest.workspace.experience_compositions = paths
        .into_iter()
        .map(|path| pale_core::paths::OwnedConfinedRelativePath::parse(path).expect("path"))
        .collect();
    std::fs::write(
        &workspace.manifest,
        toml::to_string(&manifest).expect("serialize workspace manifest"),
    )
    .expect("write workspace manifest");
}

pub(in crate::resident) fn set_responsibility_selection(
    workspace: &SyntheticWorkspace,
    selected: bool,
) {
    let workspace_root = workspace.root.path().join("workspace");
    if selected {
        let mut source =
            responsibility_fixtures::responsibility_derivation("resident-removal", 1, false);
        source.publisher_module =
            responsibility_fixtures::omitted_answer("resident-removal-publisher");
        source.adopter_module = responsibility_fixtures::omitted_answer("resident-removal-adopter");
        std::fs::write(
            workspace_root.join("responsibility.toml"),
            toml::to_string(&source).expect("serialize responsibility source"),
        )
        .expect("write responsibility source");
    }
    let source = std::fs::read_to_string(&workspace.manifest).expect("workspace manifest");
    let mut manifest: WorkspaceManifestFile =
        toml::from_str(&source).expect("typed workspace manifest");
    manifest.workspace.responsibility_derivations = selected
        .then(|| {
            pale_core::paths::OwnedConfinedRelativePath::parse("responsibility.toml")
                .expect("responsibility path")
        })
        .into_iter()
        .collect();
    std::fs::write(
        &workspace.manifest,
        toml::to_string(&manifest).expect("serialize workspace manifest"),
    )
    .expect("write responsibility selection");
}

fn absent_resource() -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("example.engine").expect("Package identity"),
        resource_id: PackageResourceIdV1::parse("absent").expect("resource identity"),
    }
}

fn admit_generation(compilation: Compilation) -> AdmittedCompiledGeneration {
    let materialization = applied_evidence(&compilation).into_materialization();
    AdmittedCompiledGeneration::admit(compilation, vec![materialization], synthetic_set_policy())
        .expect("compiled generation admits")
}

#[test]
fn generation_admission_retains_exact_core_preimage_and_serves_eager_lookups() {
    let (workspace, probe) = live_workspace();
    let compilation = compile_live(&workspace, &probe);
    let expected_distribution = compilation.distribution_lock_digest().clone();
    let expected_profile = compilation.profile().profile_digest.clone();
    let expected_instance = compilation.instance().instance_digest.clone();
    let expected_workspace = compilation.workspace_semantic_ir_digest().clone();
    let expected_runtime = compilation
        .executable_identity_digest()
        .expect("live executable identity")
        .clone();
    let expected_execution_paths = compilation.profile().execution_paths.clone();
    let materialization = applied_evidence(&compilation).into_materialization();
    let expected_closure =
        materialization_closure(std::slice::from_ref(&materialization)).expect("closure digest");
    let expected_frontier = materialization.source_frontier().clone();

    let generation = AdmittedCompiledGeneration::admit(
        compilation,
        vec![materialization],
        synthetic_set_policy(),
    )
    .expect("generation admits");
    assert_eq!(
        generation.record().distribution_lock_digest(),
        &expected_distribution
    );
    assert_eq!(generation.record().profile_digest(), &expected_profile);
    assert_eq!(generation.record().instance_digest(), &expected_instance);
    assert_eq!(
        generation.record().workspace_semantic_ir_digest(),
        &expected_workspace
    );
    assert_eq!(
        generation.record().executable_identity_digest(),
        &expected_runtime
    );
    assert_eq!(
        generation.record().materialization_closure_digest(),
        &expected_closure
    );
    assert_eq!(
        generation.record().repository_source_frontiers(),
        std::slice::from_ref(&expected_frontier)
    );
    let repository_id = LogicalRepositoryIdV1::parse("engine").expect("repository identity");
    let repository = generation
        .repository(&repository_id)
        .expect("retained repository");
    assert_eq!(repository.id(), &repository_id);
    assert_eq!(
        generation.record().execution_paths(),
        expected_execution_paths.as_slice()
    );
    assert!(repository
        .render()
        .support
        .iter()
        .zip(&expected_execution_paths)
        .all(|(support, selection)| &support.execution_path_witness.selection == selection));
    assert_eq!(
        generation
            .materialization(&repository_id)
            .map(|proof| &proof.receipt().record().repository_id),
        Some(&repository_id)
    );
    let absent = LogicalRepositoryIdV1::parse("absent").expect("absent repository identity");
    assert!(generation.repository(&absent).is_none());
    assert!(generation
        .operation(&repository_id, &absent_resource())
        .is_none());
    assert!(generation
        .run_mode(&repository_id, &absent_resource())
        .is_none());
}

#[test]
fn generation_retains_the_compilers_exact_composition_reason_projection() {
    let (workspace, probe) = live_workspace();
    let compilation = compile_live(&workspace, &probe);
    let expected = compilation
        .experience_composition_plan()
        .reason_paths
        .clone();
    assert!(!expected.is_empty());
    let generation = admit_generation(compilation);
    assert_eq!(
        generation.composition_reason_paths(),
        Some(expected.as_slice())
    );

    revise_composition_source(&workspace);
    let revised_compilation = compile_live(&workspace, &probe);
    let revised = revised_compilation
        .experience_composition_plan()
        .reason_paths
        .clone();
    let revised_generation = admit_generation(revised_compilation);
    assert_eq!(
        revised_generation.composition_reason_paths(),
        Some(revised.as_slice())
    );
}

#[test]
fn generation_borrows_exact_provider_and_path_bodies_from_resident_custody() {
    let (workspace, probe) = live_workspace();
    let generation = admit_generation(compile_live(&workspace, &probe));

    let binding = generation
        .provider_binding("provider-binding-claude-code")
        .expect("fixture provider binding is retained");
    let path = generation
        .experience_path("direct-claude-code")
        .expect("fixture experience path is retained");

    assert_eq!(binding.id, "provider-binding-claude-code");
    assert_eq!(path.id, "direct-claude-code");
    assert!(path
        .provider_binding_ids
        .iter()
        .any(|candidate| candidate == &binding.id));
    assert!(std::ptr::eq(
        binding,
        generation
            .provider_binding("provider-binding-claude-code")
            .expect("repeat lookup resolves the same body"),
    ));
    assert!(std::ptr::eq(
        path,
        generation
            .experience_path("direct-claude-code")
            .expect("repeat lookup resolves the same body"),
    ));
    assert!(generation.provider_binding("missing-binding").is_none());
    assert!(generation.experience_path("missing-path").is_none());
}

#[test]
fn generation_admission_rejects_non_live_incomplete_and_misordered_inputs() {
    let portable = SyntheticWorkspace::new(false);
    let compilation = compile_portable_inputs(
        portable.capture_inputs().expect("portable inputs"),
        &pale_adapter_fs::LocalSourceFilesystem,
    )
    .expect("portable compilation");
    assert!(matches!(
        AdmittedCompiledGeneration::admit(compilation, Vec::new(), synthetic_set_policy()),
        Err(CompiledGenerationAdmissionError::MissingExecutableIdentity)
    ));

    let (workspace, probe) = live_workspace();
    let compilation = compile_live(&workspace, &probe);
    assert!(matches!(
        AdmittedCompiledGeneration::admit(compilation, Vec::new(), synthetic_set_policy()),
        Err(CompiledGenerationAdmissionError::RepositoryCoverage {
            compiled: 1,
            applied: 0
        })
    ));

    let (expected_workspace, expected_probe) = live_workspace();
    let expected = compile_live(&expected_workspace, &expected_probe);
    let (other_workspace, other_probe) = live_workspace();
    rename_repository(&other_workspace, "other");
    let other = compile_live(&other_workspace, &other_probe);
    let other_evidence = applied_evidence(&other).into_materialization();
    assert!(matches!(
        AdmittedCompiledGeneration::admit(
            expected,
            vec![other_evidence],
            synthetic_set_policy()
        ),
        Err(CompiledGenerationAdmissionError::RepositoryOrder {
            position: 0,
            compiled,
            applied
        }) if compiled == logical_repository_id("engine")
            && applied == logical_repository_id("other")
    ));
}

#[test]
fn admission_rejects_cross_generation_and_owner_record_drift_at_the_owning_boundary() {
    let (workspace, probe) = live_workspace();
    let prior_compilation = compile_live(&workspace, &probe);
    revise_profile_policy_description(&workspace, "Retitled profile policy");
    let changed_compilation = compile_live(&workspace, &probe);
    let changed_evidence = applied_evidence(&changed_compilation).into_materialization();
    assert!(matches!(
        AdmittedCompiledGeneration::admit(
            prior_compilation,
            vec![changed_evidence],
            synthetic_set_policy()
        ),
        Err(CompiledGenerationAdmissionError::CoordinateMismatch {
            coordinate: AppliedMaterializationCoordinate::ReceiptCompilation,
            ..
        })
    ));

    let compilation = compile_live(&workspace, &probe);
    let mut state_drift = applied_evidence(&compilation);
    let mut state_preimage = state_drift.state.to_preimage();
    state_preimage.manifest_digest =
        ManifestDigest::hash_canonical(&"wrong manifest").expect("replacement manifest digest");
    state_drift.state = MaterializationStateV1::seal(state_preimage).expect("state reseals");
    assert!(matches!(
        state_drift.try_into_materialization(),
        Err(CompiledGenerationAdmissionError::CoordinateMismatch {
            coordinate: AppliedMaterializationCoordinate::ReceiptTransition,
            ..
        })
    ));

    let compilation = compile_live(&workspace, &probe);
    let mut lineage_drift = applied_evidence(&compilation);
    let mut lineage_preimage = lineage_drift.lineage.to_preimage();
    lineage_preimage.target_root_sha256 = TargetRootDigest::hash_bytes(b"wrong target root");
    lineage_drift.lineage =
        MaterializationLineageV1::seal(lineage_preimage).expect("lineage reseals");
    assert!(matches!(
        lineage_drift.try_into_materialization(),
        Err(CompiledGenerationAdmissionError::CoordinateMismatch {
            coordinate: AppliedMaterializationCoordinate::LineageReceiptBinding,
            ..
        })
    ));

    let compilation = compile_live(&workspace, &probe);
    let mut frontier_drift = applied_evidence(&compilation);
    frontier_drift = frontier_drift.with_source_frontier(RepositorySourceFrontierV1 {
        repository_id: LogicalRepositoryIdV1::parse("other").expect("other repository"),
        base_revision: RepositoryRevisionV1::parse("base-revision").expect("base revision"),
        tracked_delta_sha256: ArtifactDigest::hash_bytes(b"tracked delta"),
        untracked_inventory_sha256: ArtifactDigest::hash_bytes(b"untracked inventory"),
        dependency_frontier_sha256: ArtifactDigest::hash_bytes(b"dependency frontier"),
    });
    assert!(matches!(
        frontier_drift.try_into_materialization(),
        Err(CompiledGenerationAdmissionError::CoordinateMismatch {
            coordinate: AppliedMaterializationCoordinate::SourceFrontier,
            ..
        })
    ));
}

#[test]
fn successor_records_typed_workspace_profile_and_package_input_changes() {
    let (workspace, probe) = live_workspace();
    let prior = admit_generation(compile_live(&workspace, &probe));

    retarget_workspace_identity(&workspace, "compile-tests-retargeted");
    let unrelated_compilation = compile_live(&workspace, &probe);
    assert_ne!(
        prior.record().profile_digest(),
        &unrelated_compilation.profile().profile_digest
    );
    let unrelated_materialization = applied_evidence(&unrelated_compilation).into_materialization();
    let unrelated = AdmittedCompiledGeneration::admit_successor(
        &prior,
        unrelated_compilation,
        vec![unrelated_materialization],
        synthetic_set_policy(),
    )
    .expect("unrelated workspace successor admits");
    assert!(unrelated.derivation_receipt().is_some());
    let unrelated_changes = unrelated
        .successor_input_changes()
        .expect("successor input changes");
    assert!(unrelated_changes
        .iter()
        .any(|change| matches!(change, GenerationInputChange::WorkspaceRoot { .. })));
    assert!(unrelated_changes
        .iter()
        .any(|change| matches!(change, GenerationInputChange::Profile { .. })));
    assert!(unrelated_changes
        .iter()
        .any(|change| matches!(change, GenerationInputChange::ProjectInstance { .. })));
    assert!(!unrelated_changes.iter().any(|change| matches!(
        change,
        GenerationInputChange::PackageClosure { .. }
            | GenerationInputChange::CompositionPlan { .. }
    )));
    assert!(unrelated.activation_compatibility().admits(
        Some(prior.record().compiled_generation_digest()),
        unrelated.record().compiled_generation_digest()
    ));

    revise_profile_policy_description(&workspace, "Profile-owned lookup rebuild");
    let profile_compilation = compile_live(&workspace, &probe);
    assert_ne!(
        unrelated.record().profile_digest(),
        &profile_compilation.profile().profile_digest
    );
    let profile_materialization = applied_evidence(&profile_compilation).into_materialization();
    let profile_successor = AdmittedCompiledGeneration::admit_successor(
        &unrelated,
        profile_compilation,
        vec![profile_materialization],
        synthetic_set_policy(),
    )
    .expect("profile successor admits");
    assert!(profile_successor
        .successor_input_changes()
        .expect("profile changes")
        .iter()
        .any(|change| matches!(change, GenerationInputChange::Profile { .. })));
    assert!(!profile_successor.activation_compatibility().admits(
        Some(prior.record().compiled_generation_digest()),
        profile_successor.record().compiled_generation_digest()
    ));

    std::fs::write(
        workspace
            .root
            .path()
            .join("distribution/packages/engine/role.md"),
        "# Engine\n\nSuccessor Package body.\n",
    )
    .expect("rewrite selected Package body");
    workspace.refresh_distribution_lock();
    let package_compilation = compile_live(&workspace, &probe);
    assert_ne!(
        profile_successor.record().package_semantic_ir_digest(),
        package_compilation.package_semantic_ir().digest()
    );
    let package_materialization = applied_evidence(&package_compilation).into_materialization();
    let package_successor = AdmittedCompiledGeneration::admit_successor(
        &profile_successor,
        package_compilation,
        vec![package_materialization],
        synthetic_set_policy(),
    )
    .expect("Package successor admits");
    assert!(package_successor
        .successor_input_changes()
        .expect("Package changes")
        .iter()
        .any(|change| matches!(change, GenerationInputChange::PackageClosure { .. })));
}

#[test]
fn failed_successor_admission_leaves_the_prior_generation_usable() {
    let (workspace, probe) = live_workspace();
    let prior = admit_generation(compile_live(&workspace, &probe));
    let repository_id = LogicalRepositoryIdV1::parse("engine").expect("repository identity");
    let selected_path = prior
        .repository(&repository_id)
        .expect("prior repository")
        .render()
        .support[0]
        .execution_path_witness
        .selection
        .clone();

    revise_profile_policy_description(&workspace, "Rejected successor profile");
    let candidate_compilation = compile_live(&workspace, &probe);
    revise_profile_policy_description(&workspace, "Substituted successor profile");
    let substituted_compilation = compile_live(&workspace, &probe);
    let substituted_materialization =
        applied_evidence(&substituted_compilation).into_materialization();

    assert!(matches!(
        AdmittedCompiledGeneration::admit_successor(
            &prior,
            candidate_compilation,
            vec![substituted_materialization],
            synthetic_set_policy(),
        ),
        Err(CompiledGenerationAdmissionError::CoordinateMismatch {
            coordinate: AppliedMaterializationCoordinate::ReceiptCompilation,
            ..
        })
    ));
    assert_eq!(
        prior.repository(&repository_id).map(CompiledRepo::id),
        Some(&repository_id)
    );
    assert_eq!(
        prior
            .repository(&repository_id)
            .expect("prior repository remains queryable")
            .render()
            .support[0]
            .execution_path_witness
            .selection,
        selected_path
    );
}
