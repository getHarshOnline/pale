//! Law tests for canonical one-shot compilation coordinates.

use super::*;
use crate::compile::{
    compile_captured_portable_inputs, CapturedOneShotCompilationInputs, CapturedPackageDocument,
    CapturedPackageSource, CapturedPrivateDocument, CapturedPrivateWorkspaceInputs,
};
use crate::identity::{ProjectIdentity, ProjectIdentityError};
use crate::workspace::{
    compile_portable_workspace, CapturedWorkspaceDocument, CapturedWorkspaceSource,
};
use pale_core::records::{
    SchemaManifestDigest, SchemaSetDigest, WorkspaceModuleDigest, WorkspaceModuleId,
    WorkspaceReleaseId,
};

#[path = "../../support/captured_one_shot.rs"]
mod captured_one_shot;
use captured_one_shot::shared_captured_fixture_compilation;

#[test]
fn reused_identity_retains_exact_owners_and_rejects_corrupted_records() {
    let compilation = shared_captured_fixture_compilation();
    let profile = &compilation.profile;
    let instance = &compilation.instance;
    let identity = ProjectIdentity::verify(profile, instance).expect("admitted identity");
    assert!(std::ptr::eq(identity.profile_record(), profile));
    assert!(std::ptr::eq(identity.instance_record(), instance));

    let mut wrong_profile = profile.clone();
    wrong_profile.profile_digest = ProfileDigest::parse(sha('a')).unwrap();
    assert!(matches!(
        ProjectIdentity::verify(&wrong_profile, instance),
        Err(ProjectIdentityError::ProfileDigestMismatch)
    ));

    let mut wrong_instance = instance.clone();
    wrong_instance.instance_digest = InstanceDigest::parse(sha('b')).unwrap();
    assert!(matches!(
        ProjectIdentity::verify(profile, &wrong_instance),
        Err(ProjectIdentityError::InstanceDigestMismatch)
    ));
    wrong_instance = instance.clone();
    wrong_instance.profile_digest = ProfileDigest::parse(sha('c')).unwrap();
    assert!(matches!(
        ProjectIdentity::verify(profile, &wrong_instance),
        Err(ProjectIdentityError::ProfileBinding)
    ));

    for repository in &compilation.repos {
        let rebuilt = crate::plan::build_manifest_with_identity(
            compilation.schema_registry_identity(),
            compilation.compilation_input_digest(),
            &repository.render,
            &repository.repo_id,
            identity,
        )
        .expect("same immutable owners reproduce the exact manifest");
        assert_eq!(rebuilt, repository.manifest);
    }
}

/// Re-enter final admission using the same owned parts, without a test-only
/// production constructor or an alternate verification implementation.
fn readmit(compilation: Compilation) -> Result<Compilation, CompilationInvariantError> {
    Compilation::admit(CompilationParts {
        profile: compilation.profile,
        instance: compilation.instance,
        package_semantic_ir: compilation.package_semantic_ir,
        workspace_universe: compilation.workspace_universe,
        experience_composition_plan: compilation.experience_composition_plan,
        repos: compilation.repos,
        schema_registry: compilation.seal.schema_registry,
        package_closure: compilation.seal.package_closure,
        mode: compilation.seal.mode,
        distribution_lock_digest: compilation.seal.distribution_lock_digest,
        executable_identity_digest: compilation.seal.executable_identity_digest,
        compilation_input_digest: compilation.seal.compilation_input_digest,
    })
}

#[test]
fn identity_reuse_does_not_replace_final_admission_or_revalidation() {
    let original = shared_captured_fixture_compilation();
    let digest = original.one_shot_compilation_digest().clone();
    let readmitted = readmit(original).expect("unchanged parts admit again");
    assert_eq!(readmitted.one_shot_compilation_digest(), &digest);
    readmitted
        .verify_admitted_compilation()
        .expect("fresh verification");

    let mutations: [fn(&mut Compilation); 4] = [
        |compilation| compilation.profile.profile_digest = ProfileDigest::parse(sha('a')).unwrap(),
        |compilation| {
            compilation.instance.instance_digest = InstanceDigest::parse(sha('b')).unwrap()
        },
        |compilation| compilation.repos[0].render.surfaces[0].rel_path = "../escape".into(),
        |compilation| {
            compilation.repos[0].manifest.outputs.pop();
        },
    ];
    for mutate in mutations {
        let mut compilation = shared_captured_fixture_compilation();
        mutate(&mut compilation);
        assert!(compilation.verify_admitted_compilation().is_err());
        assert!(readmit(compilation).is_err());
    }
}

fn sha(character: char) -> String {
    std::iter::repeat_n(character, 64).collect()
}

#[test]
fn adopted_module_release_moves_input_and_one_shot_coordinates() {
    let schema_registry = SchemaRegistryIdentity {
        schema_manifest_sha256: SchemaManifestDigest::parse(sha('0')).expect("manifest digest"),
        schema_set_sha256: SchemaSetDigest::parse(sha('1')).expect("schema-set digest"),
    };
    let distribution_lock_digest =
        DistributionLockDigest::parse(sha('2')).expect("distribution-lock digest");
    let profile_digest = ProfileDigest::parse(sha('3')).expect("profile digest");
    let instance_digest = InstanceDigest::parse(sha('4')).expect("instance digest");
    let package_semantic_ir_digest =
        PackageSemanticIrDigest::parse(sha('5')).expect("Package semantic-IR digest");
    let workspace_semantic_ir_digest =
        WorkspaceSemanticIrDigest::parse(sha('6')).expect("workspace semantic-IR digest");
    let workspace_admission_evidence_digest = WorkspaceAdmissionEvidenceDigest::parse(sha('7'))
        .expect("workspace admission-evidence digest");
    let principal_admitted_universe_digest = PrincipalAdmittedUniverseDigest::parse(sha('8'))
        .expect("principal admitted-universe digest");
    let workspace_resolution_lock_digest =
        WorkspaceResolutionLockDigest::parse(sha('9')).expect("workspace resolution-lock digest");
    let workspace_universe_lock_digest =
        WorkspaceUniverseLockDigest::parse(sha('a')).expect("workspace universe-lock digest");
    let responsibility_derivation_closure_digest =
        ResponsibilityDerivationClosureDigest::parse(sha('b'))
            .expect("responsibility-derivation closure digest");
    let organizational_translation_closure_digest =
        OrganizationalTranslationClosureDigest::parse(sha('c'))
            .expect("organizational-translation closure digest");
    let experience_composition_source_digest = ExperienceCompositionSourceDigest::parse(sha('d'))
        .expect("experience-composition source digest");
    let provider_binding_closure_digest =
        ProviderBindingClosureDigest::parse(sha('e')).expect("provider-binding closure digest");
    let machine_overlay_digest =
        MachineOverlayDigest::parse(sha('f')).expect("machine-overlay digest");
    let experience_composition_plan_digest = ExperienceCompositionPlanDigest::parse(sha('0'))
        .expect("experience-composition plan digest");
    let packages = Vec::new();
    let package_releases = Vec::new();
    let normative_pack_releases = Vec::new();

    let digest_for = |module_releases: &[WorkspaceAdoptedReleaseV1]| {
        compilation_input_digest(&CompilationInputPreimage {
            schema: COMPILATION_INPUT_SCHEMA,
            compiler_contract_id: super::super::DISTRIBUTION_COMPILER_ID,
            renderer_contract_id: crate::plan::RENDERER_VERSION,
            schema_registry: &schema_registry,
            distribution_lock_digest: &distribution_lock_digest,
            packages: &packages,
            profile_digest: &profile_digest,
            instance_digest: &instance_digest,
            package_semantic_ir_digest: &package_semantic_ir_digest,
            workspace_semantic_ir_digest: &workspace_semantic_ir_digest,
            workspace_admission_evidence_digest: &workspace_admission_evidence_digest,
            principal_admitted_universe_digest: &principal_admitted_universe_digest,
            workspace_resolution_lock_digest: &workspace_resolution_lock_digest,
            workspace_universe_lock_digest: &workspace_universe_lock_digest,
            responsibility_derivation_closure_digest: &responsibility_derivation_closure_digest,
            organizational_translation_closure_digest: &organizational_translation_closure_digest,
            experience_composition_source_digest: &experience_composition_source_digest,
            provider_binding_closure_digest: &provider_binding_closure_digest,
            machine_overlay_digest: &machine_overlay_digest,
            executable_identity_digest: None,
            workspace_module_releases: module_releases,
            workspace_package_releases: &package_releases,
            workspace_normative_pack_releases: &normative_pack_releases,
            experience_composition_plan_digest: &experience_composition_plan_digest,
            mode: CompilationMode::Portable,
        })
        .expect("canonical compilation-input digest")
    };

    let empty_input = digest_for(&[]);
    let adopted = [WorkspaceAdoptedReleaseV1 {
        module_id: WorkspaceModuleId::parse("shared-module").expect("module id"),
        release_id: WorkspaceReleaseId::parse("release-1").expect("release id"),
        module_digest: WorkspaceModuleDigest::parse(sha('1')).expect("module digest"),
    }];
    let adopted_input = digest_for(&adopted);

    assert_ne!(empty_input, adopted_input);
    let repositories = BTreeMap::<LogicalRepositoryIdV1, RepositoryClosure>::new();
    assert_ne!(
        one_shot_compilation_digest(&empty_input, &repositories).expect("empty one-shot digest"),
        one_shot_compilation_digest(&adopted_input, &repositories)
            .expect("adopted one-shot digest")
    );
}
