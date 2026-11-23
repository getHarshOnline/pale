//! Deterministic generation proof for manifest-selected responsibility sources.

use super::*;

use crate::organization_fixtures;
use pale_core::paths::OwnedConfinedRelativePath;

#[derive(Debug)]
struct CandidateObservation {
    portable_compilation_input: CompilationInputDigest,
    portable_one_shot: OneShotCompilationDigest,
    generation: CompiledGenerationV1,
    sources: Vec<WorkspaceDerivationSourceV1>,
    nodes: Vec<WorkspaceDerivationNodeV1>,
    admission_evidence_count: usize,
}

fn candidate_responsibility(scope: &str, directional: bool) -> ResponsibilityDerivationV1 {
    let mut responsibility =
        responsibility_fixtures::responsibility_derivation(scope, 1, directional);
    responsibility.publisher_module =
        responsibility_fixtures::omitted_answer(&format!("{scope}-publisher"));
    responsibility.adopter_module =
        responsibility_fixtures::omitted_answer(&format!("{scope}-adopter"));
    responsibility
}

fn candidate_organization() -> OrganizationalTranslationV1 {
    let mut organization = organization_fixtures::organizational_translation();
    organization.inspection = None;
    organization
}

fn write_candidate(
    workspace: &SyntheticWorkspace,
    responsibilities: &[(&str, ResponsibilityDerivationV1)],
    manifest_order: &[&str],
    organization: &OrganizationalTranslationV1,
) {
    let root = workspace.root.path().join("workspace");
    for (path, responsibility) in responsibilities {
        std::fs::write(
            root.join(path),
            toml::to_string(responsibility).expect("serialize candidate responsibility"),
        )
        .expect("write candidate responsibility");
    }
    std::fs::write(
        root.join("organization.toml"),
        toml::to_string(organization).expect("serialize candidate organization"),
    )
    .expect("write candidate organization");

    let source = std::fs::read_to_string(&workspace.manifest).expect("workspace manifest");
    let mut manifest: WorkspaceManifestFile =
        toml::from_str(&source).expect("typed workspace manifest");
    manifest.workspace.responsibility_derivations = manifest_order
        .iter()
        .map(|path| OwnedConfinedRelativePath::parse(*path).expect("confined candidate path"))
        .collect();
    manifest.workspace.organizational_translations =
        vec![OwnedConfinedRelativePath::parse("organization.toml")
            .expect("confined organization path")];
    std::fs::write(
        &workspace.manifest,
        toml::to_string(&manifest).expect("serialize workspace manifest"),
    )
    .expect("write workspace manifest");
}

fn observe_candidate(
    workspace: &SyntheticWorkspace,
    probe: &SurfaceExecutableProbe,
) -> CandidateObservation {
    let portable = compile_portable_inputs(
        workspace
            .capture_inputs()
            .expect("capture portable candidate"),
        &pale_adapter_fs::LocalSourceFilesystem,
    )
    .expect("portable candidate compiles");
    assert!(portable.executable_identity_digest().is_none());
    assert!(portable.instance().provider_repositories.is_empty());
    assert!(portable.instance().git_metadata_roots.is_empty());
    assert!(portable.instance().native_harnesses.is_empty());
    assert!(portable.instance().session_state.is_none());
    assert!(portable.instance().durable_evidence.is_none());
    assert!(portable.instance().materialization_lineage.is_none());

    let live = compile_live(workspace, probe);
    assert_eq!(portable.profile(), live.profile());
    assert_eq!(portable.instance(), live.instance());
    assert_eq!(
        portable.workspace_semantic_ir_digest(),
        live.workspace_semantic_ir_digest()
    );
    assert_eq!(
        portable.responsibility_derivation_closure_digest(),
        live.responsibility_derivation_closure_digest()
    );
    assert_eq!(
        portable.organizational_translation_closure_digest(),
        live.organizational_translation_closure_digest()
    );
    assert_eq!(
        portable.workspace_resolution_lock_digest(),
        live.workspace_resolution_lock_digest()
    );
    assert_eq!(
        portable.workspace_universe_lock_digest(),
        live.workspace_universe_lock_digest()
    );
    assert_eq!(
        portable.provider_binding_closure_digest(),
        live.provider_binding_closure_digest()
    );
    assert_eq!(
        portable.machine_overlay_digest(),
        live.machine_overlay_digest()
    );

    let portable_compilation_input = portable.compilation_input_digest().clone();
    let portable_one_shot = portable.one_shot_compilation_digest().clone();
    let sources = live.workspace_derivation_sources().to_vec();
    let nodes = live.workspace_derivation_nodes().to_vec();
    let admission_evidence_count = live.workspace_admission_evidence_count();
    let generation = admit_generation(live).record().clone();
    CandidateObservation {
        portable_compilation_input,
        portable_one_shot,
        generation,
        sources,
        nodes,
        admission_evidence_count,
    }
}

fn expected_responsibility_digest(
    responsibilities: &[(&str, ResponsibilityDerivationV1)],
    expected_inventory: &[&str],
) -> ResponsibilityDerivationClosureDigest {
    let lowered = crate::workspace::lower_responsibility_derivations_v1(
        responsibilities
            .iter()
            .map(|(_, responsibility)| responsibility.clone())
            .collect(),
    )
    .expect("candidate responsibility set lowers");
    assert_eq!(
        lowered
            .derivations
            .iter()
            .map(|responsibility| responsibility.derivation_id.as_str())
            .collect::<Vec<_>>(),
        expected_inventory
    );
    ResponsibilityDerivationClosureDigest::hash_canonical(&(
        "pale.responsibility-derivation-closure.v1",
        &lowered,
    ))
    .expect("expected responsibility closure is canonical")
}

fn expected_organization_digest(
    organization: &OrganizationalTranslationV1,
) -> OrganizationalTranslationClosureDigest {
    let lowered = crate::workspace::lower_organizational_translation_v1(organization.clone())
        .expect("candidate organization lowers");
    OrganizationalTranslationClosureDigest::hash_canonical(&(
        "pale.organizational-translation-closure.v1",
        Some(lowered),
    ))
    .expect("expected organizational closure is canonical")
}

fn assert_inventory(
    observation: &CandidateObservation,
    responsibilities: &[(&str, ResponsibilityDerivationV1)],
    expected_inventory: &[&str],
    expected_paths: &[&str],
) {
    assert_eq!(observation.admission_evidence_count, 0);
    assert_eq!(
        observation
            .generation
            .responsibility_derivation_closure_digest(),
        &expected_responsibility_digest(responsibilities, expected_inventory)
    );
    assert_eq!(
        observation
            .sources
            .iter()
            .filter(|source| source.source_id.as_str().starts_with("responsibility:"))
            .map(|source| source.source_path.as_str())
            .collect::<Vec<_>>(),
        expected_paths
    );
    assert!(observation
        .sources
        .iter()
        .any(|source| source.source_path.as_str() == "organization.toml"));
    assert!(observation.sources.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(observation
        .nodes
        .windows(2)
        .all(|pair| pair[0].node_digest < pair[1].node_digest));
}

fn assert_owner_independent_coordinates_unchanged(
    prior: &CompiledGenerationV1,
    current: &CompiledGenerationV1,
) {
    assert_eq!(
        prior.distribution_lock_digest(),
        current.distribution_lock_digest()
    );
    assert_eq!(prior.profile_digest(), current.profile_digest());
    assert_eq!(prior.instance_digest(), current.instance_digest());
    assert_eq!(
        prior.package_semantic_ir_digest(),
        current.package_semantic_ir_digest()
    );
    assert_eq!(
        prior.profile_package_release_closure(),
        current.profile_package_release_closure()
    );
    assert_eq!(
        prior.profile_normative_pack_release_closure(),
        current.profile_normative_pack_release_closure()
    );
    assert_eq!(
        prior.workspace_admission_evidence_digest(),
        current.workspace_admission_evidence_digest()
    );
    assert_eq!(
        prior.workspace_resolution_lock_digest(),
        current.workspace_resolution_lock_digest()
    );
    assert_eq!(
        prior.workspace_universe_lock_digest(),
        current.workspace_universe_lock_digest()
    );
    assert_eq!(
        prior.workspace_module_release_closure(),
        current.workspace_module_release_closure()
    );
    assert_eq!(
        prior.workspace_package_release_closure(),
        current.workspace_package_release_closure()
    );
    assert_eq!(
        prior.workspace_normative_pack_release_closure(),
        current.workspace_normative_pack_release_closure()
    );
    assert_eq!(
        prior.experience_composition_source_digest(),
        current.experience_composition_source_digest()
    );
    assert_eq!(
        prior.provider_binding_closure_digest(),
        current.provider_binding_closure_digest()
    );
    assert_eq!(
        prior.experience_composition_plan_digest(),
        current.experience_composition_plan_digest()
    );
    assert_eq!(prior.execution_paths(), current.execution_paths());
    assert_eq!(
        prior.machine_overlay_digest(),
        current.machine_overlay_digest()
    );
    assert_eq!(
        prior.executable_identity_digest(),
        current.executable_identity_digest()
    );
    assert_eq!(
        prior.repository_source_frontiers(),
        current.repository_source_frontiers()
    );
}

fn assert_responsibility_change(prior: &CandidateObservation, current: &CandidateObservation) {
    assert_owner_independent_coordinates_unchanged(&prior.generation, &current.generation);
    assert_ne!(
        prior.generation.responsibility_derivation_closure_digest(),
        current
            .generation
            .responsibility_derivation_closure_digest()
    );
    assert_ne!(
        prior.generation.workspace_semantic_ir_digest(),
        current.generation.workspace_semantic_ir_digest()
    );
    assert_ne!(
        prior.generation.principal_admitted_universe_digest(),
        current.generation.principal_admitted_universe_digest()
    );
    assert_ne!(
        prior.portable_compilation_input,
        current.portable_compilation_input
    );
    assert_ne!(prior.portable_one_shot, current.portable_one_shot);
    assert_ne!(
        prior.generation.compilation_input_digest(),
        current.generation.compilation_input_digest()
    );
    assert_ne!(
        prior.generation.one_shot_compilation_digest(),
        current.generation.one_shot_compilation_digest()
    );
    assert_ne!(
        prior.generation.compiled_generation_digest(),
        current.generation.compiled_generation_digest()
    );
}

fn manifest_source(observation: &CandidateObservation) -> &WorkspaceDerivationSourceV1 {
    observation
        .sources
        .iter()
        .find(|source| source.source_id.as_str() == "workspace-manifest")
        .expect("workspace manifest source is retained")
}

#[test]
fn manifest_selected_responsibilities_are_deterministic_and_generation_bound() {
    let (workspace, probe) = live_workspace();
    let empty = observe_candidate(&workspace, &probe);

    let alpha = candidate_responsibility("alpha", true);
    let beta = candidate_responsibility("beta", true);
    let organization = candidate_organization();
    let pair = [
        ("responsibility-alpha.toml", alpha.clone()),
        ("responsibility-beta.toml", beta.clone()),
    ];
    let expected_pair = ["responsibility.alpha", "responsibility.beta"];
    let expected_pair_paths = ["responsibility-alpha.toml", "responsibility-beta.toml"];
    write_candidate(
        &workspace,
        &pair,
        &["responsibility-beta.toml", "responsibility-alpha.toml"],
        &organization,
    );
    let candidate = observe_candidate(&workspace, &probe);
    assert_inventory(&candidate, &pair, &expected_pair, &expected_pair_paths);
    assert_ne!(
        empty.generation.organizational_translation_closure_digest(),
        candidate
            .generation
            .organizational_translation_closure_digest()
    );
    assert_eq!(
        candidate
            .generation
            .organizational_translation_closure_digest(),
        &expected_organization_digest(&organization)
    );
    assert_responsibility_change(&empty, &candidate);
    assert!(candidate.sources.iter().all(|source| {
        !source.source_path.as_str().starts_with('/')
            && !source
                .source_path
                .as_str()
                .contains(workspace.root.path().to_string_lossy().as_ref())
    }));
    assert!(!serde_json::to_string(&candidate.generation)
        .expect("generation serializes")
        .contains(workspace.root.path().to_string_lossy().as_ref()));

    write_candidate(
        &workspace,
        &pair,
        &["responsibility-alpha.toml", "responsibility-beta.toml"],
        &organization,
    );
    let reordered = observe_candidate(&workspace, &probe);
    assert_inventory(&reordered, &pair, &expected_pair, &expected_pair_paths);
    assert_ne!(
        manifest_source(&candidate).source_digest,
        manifest_source(&reordered).source_digest,
        "the manifest document order must actually change"
    );
    assert_eq!(
        candidate.portable_compilation_input,
        reordered.portable_compilation_input
    );
    assert_eq!(candidate.portable_one_shot, reordered.portable_one_shot);
    assert_eq!(candidate.generation, reordered.generation);

    let gamma = candidate_responsibility("gamma", false);
    let triple = [
        ("responsibility-alpha.toml", alpha.clone()),
        ("responsibility-beta.toml", beta.clone()),
        ("responsibility-gamma.toml", gamma),
    ];
    write_candidate(
        &workspace,
        &triple,
        &[
            "responsibility-gamma.toml",
            "responsibility-alpha.toml",
            "responsibility-beta.toml",
        ],
        &organization,
    );
    let added = observe_candidate(&workspace, &probe);
    assert_inventory(
        &added,
        &triple,
        &[
            "responsibility.alpha",
            "responsibility.beta",
            "responsibility.gamma",
        ],
        &[
            "responsibility-alpha.toml",
            "responsibility-beta.toml",
            "responsibility-gamma.toml",
        ],
    );
    assert_eq!(
        reordered
            .generation
            .organizational_translation_closure_digest(),
        added.generation.organizational_translation_closure_digest()
    );
    assert_responsibility_change(&reordered, &added);

    write_candidate(
        &workspace,
        &pair,
        &["responsibility-alpha.toml", "responsibility-beta.toml"],
        &organization,
    );
    let removed = observe_candidate(&workspace, &probe);
    assert_inventory(&removed, &pair, &expected_pair, &expected_pair_paths);
    assert_responsibility_change(&added, &removed);
    assert_eq!(reordered.generation, removed.generation);
    assert_eq!(
        reordered.portable_compilation_input,
        removed.portable_compilation_input
    );
    assert_eq!(reordered.portable_one_shot, removed.portable_one_shot);

    let mut revised_alpha = alpha;
    let AttributableAnswerV1::Supplied { value, .. } = &mut revised_alpha.responsibility else {
        panic!("candidate responsibility is explicitly supplied")
    };
    value.outcome = responsibility_fixtures::fixture_id("outcome.alpha.revised");
    let revised = [
        ("responsibility-alpha.toml", revised_alpha),
        ("responsibility-beta.toml", beta),
    ];
    write_candidate(
        &workspace,
        &revised,
        &["responsibility-alpha.toml", "responsibility-beta.toml"],
        &organization,
    );
    let edited = observe_candidate(&workspace, &probe);
    assert_inventory(&edited, &revised, &expected_pair, &expected_pair_paths);
    assert_eq!(
        removed
            .generation
            .organizational_translation_closure_digest(),
        edited
            .generation
            .organizational_translation_closure_digest()
    );
    assert_responsibility_change(&removed, &edited);
}
