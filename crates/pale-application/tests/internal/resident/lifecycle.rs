//! Exercises digest-bound resident lifecycle transitions and their admission invariants.

use super::*;
use crate::resident_fixture::{active, advertised, fence, published};
use pale_core::records::*;

pub(in crate::resident) fn generation(byte: char) -> CompiledGenerationDigest {
    CompiledGenerationDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>())
        .expect("generation digest")
}

fn compatibility(
    prior: Option<&CompiledGenerationDigest>,
    current: &CompiledGenerationDigest,
) -> GenerationActivationCompatibility {
    crate::resident::generation::tests::activation_compatibility(prior.cloned(), current.clone())
}

pub(in crate::resident) fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

pub(in crate::resident) fn effective_profile() -> EffectiveProjectProfileV1 {
    EffectiveProjectProfileV1::seal(EffectiveProjectProfilePreimageV1 {
        schema: SchemaId::parse(EFFECTIVE_PROJECT_PROFILE_SCHEMA).expect("profile schema"),
        schema_sha256: SchemaDigest::parse(sha('0')).expect("profile schema digest"),
        schema_version: EFFECTIVE_PROJECT_PROFILE_SCHEMA_VERSION,
        workspace_id: WorkspaceId::parse("workspace-1").expect("workspace id"),
        project_id: LogicalProjectIdV1::parse("project-1").expect("project id"),
        execution_workspace_id: LogicalExecutionWorkspaceIdV1::parse("execution-workspace-1")
            .expect("execution workspace id"),
        execution_paths: vec![ExecutionPathSelectionV1 {
            id: ExecutionPathSelectionIdV1::parse("direct-codex").expect("execution path"),
            experience_path_id: "direct-codex".into(),
        }],
        run_policy: RunPolicyEntry {
            id: RunPolicyIdV1::parse("inert").expect("run policy id"),
            description: "No lifecycle gate.".into(),
            session_inject: false,
            child_propagation: ChildPropagation::Exempt,
            child_type_selectors: Vec::new(),
            action_gate: false,
            gated_tool_matchers: Vec::new(),
        },
        package_closure: Vec::new(),
        normative_pack_closure: Vec::new(),
        package_semantic_ir_digest: PackageSemanticIrDigest::parse(sha('b'))
            .expect("Package semantic IR digest"),
        authority_root_ids: Vec::new(),
        repositories: Vec::new(),
        governing_order: ResolvedGoverningContributionV1::compose(
            Vec::new(),
            &std::collections::BTreeMap::new(),
        )
        .expect("empty governing order"),
        semantic: SemanticSelection::default(),
        capability_narrowing: None,
        capability_bindings: std::collections::BTreeMap::new(),
        operation_bindings: Vec::new(),
        lifecycle: Vec::new(),
        forbidden_token_authority: Vec::new(),
        session: None,
    })
    .expect("effective profile seals")
}

pub(in crate::resident) fn compiled_generation(runtime_byte: char) -> CompiledGenerationV1 {
    let profile = effective_profile();
    let preimage = CompiledGenerationPreimageV1 {
        schema: SchemaId::parse(COMPILED_GENERATION_V1_SCHEMA).expect("compiled schema"),
        schema_sha256: SchemaDigest::parse(sha('a')).expect("compiled schema digest"),
        schema_version: COMPILED_GENERATION_V1_SCHEMA_VERSION,
        distribution_lock_digest: DistributionLockDigest::parse(sha('1')).expect("distribution"),
        profile_digest: profile.profile_digest,
        instance_digest: InstanceDigest::parse(sha('3')).expect("instance"),
        package_semantic_ir_digest: PackageSemanticIrDigest::parse(sha('4')).expect("package IR"),
        profile_package_release_closure: Vec::new(),
        profile_normative_pack_release_closure: Vec::new(),
        schema_manifest_digest: SchemaManifestDigest::parse(sha('5')).expect("manifest"),
        schema_set_digest: SchemaSetDigest::parse(sha('6')).expect("schema set"),
        workspace_semantic_ir_digest: WorkspaceSemanticIrDigest::parse(sha('7'))
            .expect("workspace IR"),
        workspace_admission_evidence_digest: WorkspaceAdmissionEvidenceDigest::parse(sha('8'))
            .expect("evidence"),
        principal_admitted_universe_digest: PrincipalAdmittedUniverseDigest::parse(sha('9'))
            .expect("universe"),
        workspace_module_release_closure: Vec::new(),
        workspace_package_release_closure: Vec::new(),
        workspace_normative_pack_release_closure: Vec::new(),
        workspace_resolution_lock_digest: WorkspaceResolutionLockDigest::parse(sha('a'))
            .expect("resolution"),
        workspace_universe_lock_digest: WorkspaceUniverseLockDigest::parse(sha('b'))
            .expect("universe lock"),
        responsibility_derivation_closure_digest: ResponsibilityDerivationClosureDigest::parse(
            sha('c'),
        )
        .expect("responsibility"),
        organizational_translation_closure_digest: OrganizationalTranslationClosureDigest::parse(
            sha('d'),
        )
        .expect("organization"),
        experience_composition_source_digest: ExperienceCompositionSourceDigest::parse(sha('e'))
            .expect("composition source"),
        provider_binding_closure_digest: ProviderBindingClosureDigest::parse(sha('f'))
            .expect("provider closure"),
        experience_composition_plan_digest: ExperienceCompositionPlanDigest::parse(sha('1'))
            .expect("composition plan"),
        execution_paths: vec![ExecutionPathSelectionV1 {
            id: ExecutionPathSelectionIdV1::parse("direct-codex").expect("execution path"),
            experience_path_id: "direct-codex".into(),
        }],
        materialization_closure_digest: MaterializationClosureDigest::parse(sha('2'))
            .expect("materialization"),
        machine_overlay_digest: MachineOverlayDigest::parse(sha('3')).expect("machine overlay"),
        executable_identity_digest: ExecutableIdentityDigest::parse(sha(runtime_byte))
            .expect("executable identity"),
        compilation_input_digest: CompilationInputDigest::parse(sha('5'))
            .expect("compilation input"),
        one_shot_compilation_digest: OneShotCompilationDigest::parse(sha('6')).expect("one-shot"),
        repository_source_frontiers: vec![RepositorySourceFrontierV1 {
            repository_id: LogicalRepositoryIdV1::parse("repository-1").expect("repository"),
            base_revision: RepositoryRevisionV1::parse("revision-1").expect("revision"),
            tracked_delta_sha256: ArtifactDigest::parse(sha('7')).expect("tracked"),
            untracked_inventory_sha256: ArtifactDigest::parse(sha('8')).expect("untracked"),
            dependency_frontier_sha256: ArtifactDigest::parse(sha('9')).expect("dependencies"),
        }],
    };
    CompiledGenerationV1::admit_owner_preimage(preimage).expect("compiled generation admits")
}

pub(in crate::resident) fn initial_active(
    generation: &CompiledGenerationV1,
) -> (
    GenerationLifecycle,
    PublishedGenerationHeadV1,
    AdvertisedGenerationHeadV1,
    ActivationFenceV1,
    ActiveGenerationHeadV1,
) {
    let p = published(generation, 1, None);
    let a = advertised(&p, 2, None);
    let f = fence(generation, 1, None);
    let h = active(&a, &f, 3, None);
    let state = GenerationLifecycle::empty(RecoveryReferenceKnowledge::Known(BTreeSet::new()));
    let state = state
        .publish(p.clone())
        .expect("publish")
        .successor()
        .clone();
    let state = state
        .advertise(a.clone())
        .expect("advertise")
        .successor()
        .clone();
    let state = state
        .activate(
            h.clone(),
            f.clone(),
            &compatibility(None, generation.compiled_generation_digest()),
        )
        .expect("activate")
        .successor()
        .clone();
    (state, p, a, f, h)
}

#[test]
fn exact_initial_and_successor_lifecycle_reactivates_equal_content_under_a_later_fence() {
    let generation = compiled_generation('4');
    let (state, p1, a1, f1, h1) = initial_active(&generation);
    let p2 = published(&generation, 4, Some(&p1));
    let state = state
        .publish(p2.clone())
        .expect("republish")
        .successor()
        .clone();
    let a2 = advertised(&p2, 5, Some(&a1));
    let state = state
        .advertise(a2.clone())
        .expect("readvertise")
        .successor()
        .clone();
    let f2 = fence(&generation, 2, Some(&f1));
    let h2 = active(&a2, &f2, 6, Some(&h1));
    assert!(matches!(
        state.activate(
            h2.clone(),
            f2.clone(),
            &compatibility(None, generation.compiled_generation_digest()),
        ),
        Err(GenerationLifecycleError::CoordinateMismatch(
            "activation compatibility"
        ))
    ));
    let state = state
        .activate(
            h2,
            f2.clone(),
            &compatibility(
                Some(h1.compiled_generation_digest()),
                generation.compiled_generation_digest(),
            ),
        )
        .expect("reactivate")
        .successor()
        .clone();
    assert_eq!(
        state.snapshot().activation_fence,
        Some(f2.activation_fence_digest().clone())
    );
    assert_eq!(state.snapshot().retained_generations.len(), 1);
}

#[test]
fn mixed_generation_fence_is_rejected_without_a_transition() {
    let first = compiled_generation('4');
    let second = compiled_generation('5');
    let (state, _, _, f1, h1) = initial_active(&first);
    let foreign_fence = fence(&second, 2, Some(&f1));
    assert!(matches!(
        state.activate(
            h1.clone(),
            foreign_fence,
            &compatibility(
                Some(h1.compiled_generation_digest()),
                h1.compiled_generation_digest(),
            ),
        ),
        Err(GenerationLifecycleError::CoordinateMismatch(_))
    ));
}

#[test]
fn old_session_pin_survives_successor_activation_and_rollback_moves_forward() {
    let old = compiled_generation('4');
    let new = compiled_generation('5');
    let (mut state, p1, a1, f1, h1) = initial_active(&old);
    let lease = crate::resident::journal::tests::session_lease_for(&old, &h1, &f1, "session-1", 10);
    let session_admission_id = lease.session_admission_id().clone();
    let lease_digest = lease.session_admission_lease_digest().clone();
    state.session_pins.insert(
        session_admission_id.clone(),
        SessionGenerationPin::bind_generation_retention(&lease, &h1, &f1)
            .expect("session retention pin"),
    );
    state
        .rollback_targets
        .insert(old.compiled_generation_digest().clone());

    let p2 = published(&new, 4, Some(&p1));
    let state = state
        .publish(p2.clone())
        .expect("publish successor")
        .successor()
        .clone();
    let a2 = advertised(&p2, 5, Some(&a1));
    let state = state
        .advertise(a2.clone())
        .expect("advertise successor")
        .successor()
        .clone();
    let f2 = fence(&new, 2, Some(&f1));
    let h2 = active(&a2, &f2, 6, Some(&h1));
    let state = state
        .activate(
            h2.clone(),
            f2.clone(),
            &compatibility(
                Some(h1.compiled_generation_digest()),
                new.compiled_generation_digest(),
            ),
        )
        .expect("activate successor")
        .successor()
        .clone();
    assert_eq!(
        state.snapshot().session_pins[&session_admission_id],
        SessionGenerationPinSnapshot {
            lease_digest,
            generation: old.compiled_generation_digest().clone(),
        }
    );

    let p3 = published(&old, 7, Some(&p2));
    let state = state
        .publish(p3.clone())
        .expect("republish rollback target")
        .successor()
        .clone();
    let a3 = advertised(&p3, 8, Some(&a2));
    let state = state
        .advertise(a3.clone())
        .expect("readvertise rollback target")
        .successor()
        .clone();
    let f3 = fence(&old, 3, Some(&f2));
    let h3 = active(&a3, &f3, 9, Some(&h2));
    let state = state
        .rollback(h3, f3.clone())
        .expect("forward rollback activation")
        .successor()
        .clone();
    assert_eq!(f3.activation_fence_generation().get(), 3);
    assert_eq!(
        state.snapshot().activation_fence,
        Some(f3.activation_fence_digest().clone())
    );
}

#[test]
fn incomplete_recovery_knowledge_never_becomes_collection_permission() {
    let lifecycle = GenerationLifecycle::empty(RecoveryReferenceKnowledge::Unknown);
    assert_eq!(
        lifecycle.reclamation_disposition(&generation('a')),
        ReclamationDisposition::Unknown
    );
}

#[test]
fn stale_lease_digest_cannot_drain_a_reused_logical_session() {
    let generation = compiled_generation('4');
    let (lifecycle, _, _, fence, active) = initial_active(&generation);
    let current_lease = crate::resident::journal::tests::session_lease_for(
        &generation,
        &active,
        &fence,
        "session-reused",
        10,
    );
    let session_admission_id = current_lease.session_admission_id().clone();
    let current_lease_digest = current_lease.session_admission_lease_digest().clone();
    let stale_lease = SessionAdmissionLeaseDigest::parse(sha('f')).expect("stale lease");
    let lifecycle = lifecycle
        .pin_session(&current_lease)
        .expect("current session lease")
        .into_successor();

    assert!(matches!(
        lifecycle.drain_session(&session_admission_id, &stale_lease),
        Err(GenerationLifecycleError::SessionIdentityConflict)
    ));
    let replay = GenerationLifecycleDelta::DrainSession {
        session_admission_id: session_admission_id.clone(),
        lease_digest: stale_lease,
    };
    let port = crate::resident::journal::tests::MemoryAuthorityStore::default();
    let session_leases = crate::resident::session_lease_custody::SessionLeaseRecoveryContext::new(
        &port,
        crate::resident::journal::tests::store(),
        crate::resident::journal::tests::limits().maximum_entry_bytes(),
    );
    assert!(matches!(
        replay.apply(
            &lifecycle,
            &crate::resident_fixture::RecoveryResolver::default(),
            &session_leases,
        ),
        Err(GenerationLifecycleError::SessionIdentityConflict)
    ));
    assert!(lifecycle
        .drain_session(&session_admission_id, &current_lease_digest)
        .expect("exact lease drains")
        .successor()
        .session_pins
        .is_empty());
}

#[test]
fn known_reference_sets_accumulate_typed_retention_reasons() {
    let retained = generation('a');
    let lifecycle = GenerationLifecycle {
        published: None,
        advertised: None,
        active: None,
        fence: None,
        retained_generations: BTreeSet::from([retained.clone()]),
        session_pins: BTreeMap::new(),
        rollback_targets: BTreeSet::from([retained.clone()]),
        recovery_references: RecoveryReferenceKnowledge::Known(BTreeSet::from([retained.clone()])),
    };
    assert_eq!(
        lifecycle.reclamation_disposition(&retained),
        ReclamationDisposition::Retained(BTreeSet::from([
            GenerationRetentionReason::RollbackTarget,
            GenerationRetentionReason::RecoveryReference,
        ]))
    );
    assert_eq!(
        lifecycle.reclamation_disposition(&generation('b')),
        ReclamationDisposition::Eligible
    );
}

#[test]
fn rollback_retention_requires_an_already_retained_generation() {
    let lifecycle = GenerationLifecycle::empty(RecoveryReferenceKnowledge::Known(BTreeSet::new()));
    assert!(matches!(
        lifecycle.retain_for_rollback(generation('a')),
        Err(GenerationLifecycleError::GenerationNotRetained)
    ));
}

#[test]
fn reclamation_removes_only_an_unreferenced_retained_coordinate() {
    let retained = generation('a');
    let lifecycle = GenerationLifecycle {
        published: None,
        advertised: None,
        active: None,
        fence: None,
        retained_generations: BTreeSet::from([retained.clone()]),
        session_pins: BTreeMap::new(),
        rollback_targets: BTreeSet::new(),
        recovery_references: RecoveryReferenceKnowledge::Known(BTreeSet::new()),
    };
    let successor = lifecycle
        .reclaim(&retained)
        .expect("unreferenced retained generation reclaims")
        .successor()
        .clone();
    assert!(!successor.retained_generations.contains(&retained));

    let unknown = GenerationLifecycle {
        recovery_references: RecoveryReferenceKnowledge::Unknown,
        ..lifecycle
    };
    assert!(matches!(
        unknown.reclaim(&retained),
        Err(GenerationLifecycleError::GenerationNotReclaimable)
    ));
}
