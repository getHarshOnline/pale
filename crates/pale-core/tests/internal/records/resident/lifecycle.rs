//! Exercises resident session and generation lifecycle admission across typed ceilings and fences.

use super::*;

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("resident record ref")
}

fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("signature")
}

fn session_signing_authority() -> SigningAuthorityBindingV1 {
    SigningAuthorityBindingV1::integrated_runtime(
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        ResidentKeyIdV1::parse("runtime-key-1").expect("runtime key"),
        SigningKeyEpochV1::new(1).expect("runtime key epoch"),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: record_ref("runtime-authority-1"),
            deployment_runtime_authority_sha256: ArtifactDigest::parse(sha('6'))
                .expect("runtime authority digest"),
            deployment_binding_ref: record_ref("deployment-binding"),
            deployment_binding_sha256: ArtifactDigest::parse(sha('d'))
                .expect("deployment binding digest"),
            checked_principal_trust_checkpoint_ref: record_ref("trust-checkpoint"),
            checked_principal_trust_checkpoint_sha256: ArtifactDigest::parse(sha('5'))
                .expect("trust checkpoint digest"),
        },
    )
    .expect("session signing authority")
}

fn seal_session(
    preimage: SessionAdmissionLeasePreimageV1,
) -> Result<SessionAdmissionLeaseV1, InvalidResidentRecordV1> {
    SessionAdmissionLeaseV1::seal(preimage, signature())
}

fn execution_path() -> ExecutionPathSelectionV1 {
    ExecutionPathSelectionV1 {
        id: ExecutionPathSelectionIdV1::parse("direct-terminal").expect("execution path id"),
        experience_path_id: "direct/terminal".into(),
    }
}

fn alternate_execution_path() -> ExecutionPathSelectionV1 {
    ExecutionPathSelectionV1 {
        id: ExecutionPathSelectionIdV1::parse("delegated-terminal").expect("execution path id"),
        experience_path_id: "delegated/terminal".into(),
    }
}

fn effective_profile() -> EffectiveProjectProfileV1 {
    EffectiveProjectProfileV1::seal(EffectiveProjectProfilePreimageV1 {
        schema: SchemaId::parse(EFFECTIVE_PROJECT_PROFILE_SCHEMA).expect("profile schema"),
        schema_sha256: SchemaDigest::parse(sha('0')).expect("profile schema digest"),
        schema_version: EFFECTIVE_PROJECT_PROFILE_SCHEMA_VERSION,
        workspace_id: WorkspaceId::parse("workspace-1").expect("workspace id"),
        project_id: LogicalProjectIdV1::parse("project-1").expect("project id"),
        execution_workspace_id: LogicalExecutionWorkspaceIdV1::parse("execution-workspace-1")
            .expect("execution workspace id"),
        execution_paths: vec![execution_path()],
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

fn parent_session_preimage(
    session_id: &str,
    budget_ceiling_units: u64,
) -> SessionAdmissionLeasePreimageV1 {
    SessionAdmissionLeasePreimageV1 {
        schema_id: SchemaId::parse("pale/session-admission-lease.v1").expect("session schema"),
        schema_sha256: SchemaDigest::parse(sha('a')).expect("session schema digest"),
        session_admission_id: SessionAdmissionIdV1::parse(session_id)
            .expect("session admission id"),
        compiled_generation_digest: CompiledGenerationDigest::parse(sha('b'))
            .expect("compiled generation digest"),
        active_generation_head_digest: ActiveGenerationHeadDigest::parse(sha('7'))
            .expect("active generation head digest"),
        activation_fence_digest: ActivationFenceDigest::parse(sha('c'))
            .expect("activation fence digest"),
        deployment_binding_ref: record_ref("deployment-binding"),
        deployment_binding_sha256: ArtifactDigest::parse(sha('d')).expect("deployment digest"),
        deployment_runtime_authority_ref: record_ref("runtime-authority-1"),
        deployment_runtime_authority_sha256: ArtifactDigest::parse(sha('6'))
            .expect("runtime authority digest"),
        principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        controller_id: ResidentControllerIdV1::parse("controller-1").expect("controller"),
        relationship_scope: None,
        repository_id: LogicalRepositoryIdV1::parse("repository-1").expect("repository id"),
        project_id: LogicalProjectIdV1::parse("project-1").expect("project id"),
        execution_workspace_id: LogicalExecutionWorkspaceIdV1::parse("execution-workspace-1")
            .expect("execution workspace id"),
        profile_digest: ProfileDigest::parse(sha('e')).expect("profile digest"),
        instance_digest: InstanceDigest::parse(sha('f')).expect("instance digest"),
        execution_path: execution_path(),
        harness_adapter_source_sha256: ArtifactDigest::parse(sha('1'))
            .expect("adapter source digest"),
        harness_adapter_build_sha256: ArtifactDigest::parse(sha('2'))
            .expect("adapter build digest"),
        harness_executable_sha256: ArtifactDigest::parse(sha('3'))
            .expect("harness executable digest"),
        harness_configuration_sha256: ArtifactDigest::parse(sha('4'))
            .expect("harness configuration digest"),
        capability_ceiling: vec![
            CapabilityIdV1::parse("compute").expect("capability"),
            CapabilityIdV1::parse("read").expect("capability"),
        ],
        read_ceiling: vec![record_ref("read-a"), record_ref("read-b")],
        effect_ceiling: vec![EffectClass::Compute, EffectClass::Read],
        budget_ceiling_units: LeaseBudgetUnitsV1::new(budget_ceiling_units)
            .expect("valid fixture budget ceiling"),
        timeout_ceiling_millis: LeaseTimeoutMillisV1::new(1_000)
            .expect("valid fixture timeout ceiling"),
        evidence_obligations: vec![record_ref("evidence-a"), record_ref("evidence-b")],
        evidence_destination: record_ref("evidence-destination"),
        issued_at: UnixTimeV1::new(10).expect("valid fixture time"),
        expires_at: UnixTimeV1::new(100).expect("valid fixture time"),
        revocation_epoch: PrincipalRevocationEpochV1::new(1)
            .expect("valid fixture revocation epoch"),
        checked_revocation_ref: None,
        checked_revocation_sha256: None,
        native_session_correlation: NativeSessionCorrelationV1::Unobserved {
            reason: ResidentReasonV1::parse("native identity unavailable")
                .expect("native identity reason"),
        },
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
        issuer_key_id: ResidentKeyIdV1::parse("runtime-key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer key epoch"),
    }
}

fn parent_session(session_id: &str, budget_ceiling_units: u64) -> SessionAdmissionLeaseV1 {
    seal_session(parent_session_preimage(session_id, budget_ceiling_units))
        .expect("valid parent session")
}

fn operation_request() -> OperationLeaseRequestV1 {
    OperationLeaseRequestV1 {
        operation_lease_id: OperationLeaseIdV1::parse("operation-lease-1")
            .expect("operation lease id"),
        operation: PackageResourceRefV1 {
            package_id: PackageIdV1::parse("operations").expect("package id"),
            resource_id: PackageResourceIdV1::parse("inspect").expect("resource id"),
        },
        capabilities: vec![CapabilityIdV1::parse("compute").expect("capability")],
        effects: vec![EffectClass::Compute],
        read_set: vec![record_ref("read-a")],
        evidence_obligations: vec![record_ref("evidence-a"), record_ref("evidence-b")],
        budget_units: LeaseBudgetUnitsV1::new(5).expect("valid fixture budget"),
        timeout_millis: LeaseTimeoutMillisV1::new(500).expect("valid fixture timeout"),
        expires_at: UnixTimeV1::new(90).expect("valid fixture time"),
        invocation_correlation: InvocationCorrelationV1::Unobserved {
            reason: ResidentReasonV1::parse("native invocation unavailable")
                .expect("native invocation reason"),
        },
    }
}

fn wire_from<T, W>(value: &T) -> W
where
    T: serde::Serialize,
    W: serde::de::DeserializeOwned,
{
    serde_json::from_value(serde_json::to_value(value).expect("record serializes"))
        .expect("structural wire decodes")
}

fn compiled_generation_preimage() -> CompiledGenerationPreimageV1 {
    let profile = effective_profile();
    CompiledGenerationPreimageV1 {
        schema: SchemaId::parse(COMPILED_GENERATION_V1_SCHEMA).expect("compiled schema"),
        schema_sha256: SchemaDigest::parse(sha('a')).expect("compiled schema digest"),
        schema_version: COMPILED_GENERATION_V1_SCHEMA_VERSION,
        distribution_lock_digest: DistributionLockDigest::parse(sha('1'))
            .expect("distribution lock digest"),
        profile_digest: profile.profile_digest,
        instance_digest: InstanceDigest::parse(sha('3')).expect("instance digest"),
        package_semantic_ir_digest: PackageSemanticIrDigest::parse(sha('4'))
            .expect("package semantic digest"),
        profile_package_release_closure: Vec::new(),
        profile_normative_pack_release_closure: Vec::new(),
        schema_manifest_digest: SchemaManifestDigest::parse(sha('5'))
            .expect("schema manifest digest"),
        schema_set_digest: SchemaSetDigest::parse(sha('6')).expect("schema set digest"),
        workspace_semantic_ir_digest: WorkspaceSemanticIrDigest::parse(sha('7'))
            .expect("workspace semantic digest"),
        workspace_admission_evidence_digest: WorkspaceAdmissionEvidenceDigest::parse(sha('8'))
            .expect("workspace evidence digest"),
        principal_admitted_universe_digest: PrincipalAdmittedUniverseDigest::parse(sha('9'))
            .expect("admitted universe digest"),
        workspace_module_release_closure: Vec::new(),
        workspace_package_release_closure: Vec::new(),
        workspace_normative_pack_release_closure: Vec::new(),
        workspace_resolution_lock_digest: WorkspaceResolutionLockDigest::parse(sha('a'))
            .expect("workspace resolution digest"),
        workspace_universe_lock_digest: WorkspaceUniverseLockDigest::parse(sha('b'))
            .expect("workspace universe digest"),
        responsibility_derivation_closure_digest: ResponsibilityDerivationClosureDigest::parse(
            sha('c'),
        )
        .expect("responsibility digest"),
        organizational_translation_closure_digest: OrganizationalTranslationClosureDigest::parse(
            sha('d'),
        )
        .expect("organizational digest"),
        experience_composition_source_digest: ExperienceCompositionSourceDigest::parse(sha('e'))
            .expect("composition source digest"),
        provider_binding_closure_digest: ProviderBindingClosureDigest::parse(sha('f'))
            .expect("provider binding digest"),
        experience_composition_plan_digest: ExperienceCompositionPlanDigest::parse(sha('1'))
            .expect("composition plan digest"),
        execution_paths: vec![execution_path()],
        materialization_closure_digest: MaterializationClosureDigest::parse(sha('2'))
            .expect("materialization digest"),
        machine_overlay_digest: MachineOverlayDigest::parse(sha('3'))
            .expect("machine overlay digest"),
        executable_identity_digest: ExecutableIdentityDigest::parse(sha('4'))
            .expect("executable identity digest"),
        compilation_input_digest: CompilationInputDigest::parse(sha('5'))
            .expect("compilation input digest"),
        one_shot_compilation_digest: OneShotCompilationDigest::parse(sha('6'))
            .expect("one-shot compilation digest"),
        repository_source_frontiers: vec![RepositorySourceFrontierV1 {
            repository_id: LogicalRepositoryIdV1::parse("repository-1").expect("repository id"),
            base_revision: RepositoryRevisionV1::parse("revision-1").expect("revision"),
            tracked_delta_sha256: ArtifactDigest::parse(sha('7')).expect("tracked digest"),
            untracked_inventory_sha256: ArtifactDigest::parse(sha('8')).expect("untracked digest"),
            dependency_frontier_sha256: ArtifactDigest::parse(sha('9')).expect("dependency digest"),
        }],
    }
}

fn compiled_generation() -> CompiledGenerationV1 {
    CompiledGenerationV1::admit_owner_preimage(compiled_generation_preimage())
        .expect("owner-admitted compiled generation")
}

fn published_head(
    compiled_generation: &CompiledGenerationV1,
    sequence: u64,
    predecessor: Option<&PublishedGenerationHeadV1>,
) -> PublishedGenerationHeadV1 {
    PublishedGenerationHeadV1::seal(PublishedGenerationHeadPreimageV1 {
        schema: SchemaId::parse(PUBLISHED_GENERATION_HEAD_V1_SCHEMA).expect("published schema"),
        schema_sha256: SchemaDigest::parse(sha('a')).expect("published schema digest"),
        schema_version: PUBLISHED_GENERATION_HEAD_V1_SCHEMA_VERSION,
        compiled_generation_digest: compiled_generation.compiled_generation_digest().clone(),
        journal_sequence: JournalSequenceV1::new(sequence).expect("journal sequence"),
        predecessor_published_generation_head_digest: predecessor
            .map(|value| value.published_generation_head_digest().clone()),
    })
    .expect("published head seals")
}

fn advertised_head(
    published: &PublishedGenerationHeadV1,
    sequence: u64,
    predecessor: Option<&AdvertisedGenerationHeadV1>,
) -> AdvertisedGenerationHeadV1 {
    AdvertisedGenerationHeadV1::seal(AdvertisedGenerationHeadPreimageV1 {
        schema: SchemaId::parse(ADVERTISED_GENERATION_HEAD_V1_SCHEMA).expect("advertised schema"),
        schema_sha256: SchemaDigest::parse(sha('b')).expect("advertised schema digest"),
        schema_version: ADVERTISED_GENERATION_HEAD_V1_SCHEMA_VERSION,
        compiled_generation_digest: published.compiled_generation_digest().clone(),
        published_generation_head_digest: published.published_generation_head_digest().clone(),
        journal_sequence: JournalSequenceV1::new(sequence).expect("journal sequence"),
        predecessor_advertised_generation_head_digest: predecessor
            .map(|value| value.advertised_generation_head_digest().clone()),
    })
    .expect("advertised head seals")
}

fn activation_fence(
    compiled_generation: &CompiledGenerationV1,
    generation: u64,
    predecessor: Option<&ActivationFenceV1>,
) -> ActivationFenceV1 {
    ActivationFenceV1::seal(ActivationFencePreimageV1 {
        schema: SchemaId::parse(ACTIVATION_FENCE_V1_SCHEMA).expect("fence schema"),
        schema_sha256: SchemaDigest::parse(sha('c')).expect("fence schema digest"),
        schema_version: ACTIVATION_FENCE_V1_SCHEMA_VERSION,
        activation_fence_id: ActivationFenceIdV1::parse("deployment-fence").expect("fence id"),
        activation_fence_generation: ActivationFenceGenerationV1::new(generation)
            .expect("fence generation"),
        compiled_generation_digest: compiled_generation.compiled_generation_digest().clone(),
        predecessor_activation_fence_digest: predecessor
            .map(|value| value.activation_fence_digest().clone()),
    })
    .expect("activation fence seals")
}

fn active_head(
    advertised: &AdvertisedGenerationHeadV1,
    fence: &ActivationFenceV1,
    sequence: u64,
    predecessor: Option<&ActiveGenerationHeadV1>,
) -> ActiveGenerationHeadV1 {
    ActiveGenerationHeadV1::seal(ActiveGenerationHeadPreimageV1 {
        schema: SchemaId::parse(ACTIVE_GENERATION_HEAD_V1_SCHEMA).expect("active schema"),
        schema_sha256: SchemaDigest::parse(sha('d')).expect("active schema digest"),
        schema_version: ACTIVE_GENERATION_HEAD_V1_SCHEMA_VERSION,
        compiled_generation_digest: advertised.compiled_generation_digest().clone(),
        advertised_generation_head_digest: advertised.advertised_generation_head_digest().clone(),
        activation_fence_digest: fence.activation_fence_digest().clone(),
        journal_sequence: JournalSequenceV1::new(sequence).expect("journal sequence"),
        predecessor_active_generation_head_digest: predecessor
            .map(|value| value.active_generation_head_digest().clone()),
    })
    .expect("active head seals")
}

#[test]
fn distribution_lock_digest_requires_the_canonical_complete_lock() {
    let lock = DistributionLockFile {
        distribution: DistributionMeta {
            compiler: "pale-compiler-v1".into(),
            schema_manifest_sha256: SchemaManifestDigest::parse(sha('1'))
                .expect("schema manifest digest"),
            schema_set_sha256: SchemaSetDigest::parse(sha('2')).expect("schema set digest"),
        },
        packages: Vec::new(),
    };
    let digest = DistributionLockDigest::from_lock(&lock).expect("canonical distribution lock");
    let mut changed = lock.clone();
    changed.distribution.compiler = "pale-compiler-v1-modified".into();
    assert_ne!(
        digest,
        DistributionLockDigest::from_lock(&changed).expect("changed canonical lock")
    );
}

#[test]
fn compiled_generation_admission_binds_every_owner_input() {
    let preimage = compiled_generation_preimage();
    let generation =
        CompiledGenerationV1::admit_owner_preimage(preimage.clone()).expect("owner preimage seals");
    let context = CompiledGenerationAdmissionContextV1 {
        admitted_preimage: &preimage,
    };
    CompiledGenerationV1::admit_contextual_wire(wire_from(&generation), &context)
        .expect("exact owner inputs admit wire");

    let mut substituted = preimage.clone();
    substituted.executable_identity_digest =
        ExecutableIdentityDigest::parse(sha('a')).expect("substituted runtime");
    let substituted_generation = CompiledGenerationV1::admit_owner_preimage(substituted)
        .expect("substituted owner preimage remains structurally valid");
    assert!(matches!(
        CompiledGenerationV1::admit_contextual_wire(wire_from(&substituted_generation), &context),
        Err(InvalidResidentRecordV1::ContextualAdmission { .. })
    ));
}

#[test]
fn compiled_generation_schema_keeps_distinct_closures_and_typed_digest_names() {
    let schema = serde_json::to_value(schemars::schema_for!(CompiledGenerationWireV1))
        .expect("schema serializes");
    let properties = schema
        .pointer("/properties")
        .and_then(serde_json::Value::as_object)
        .expect("root properties");
    for required in [
        "distribution_lock_digest",
        "profile_package_release_closure",
        "profile_normative_pack_release_closure",
        "workspace_module_release_closure",
        "workspace_package_release_closure",
        "workspace_normative_pack_release_closure",
        "workspace_admission_evidence_digest",
        "principal_admitted_universe_digest",
        "compilation_input_digest",
        "one_shot_compilation_digest",
        "repository_source_frontiers",
    ] {
        assert!(properties.contains_key(required), "missing {required}");
    }
    for obsolete in [
        "distribution_sha256",
        "schema_manifest_sha256",
        "schema_set_sha256",
        "materialization_closure_sha256",
        "machine_overlay_sha256",
        "runtime_identity_sha256",
    ] {
        assert!(!properties.contains_key(obsolete), "retained {obsolete}");
    }
}

#[test]
fn compiled_generation_rejects_noncanonical_path_and_repository_closures() {
    let mut duplicate_paths = compiled_generation_preimage();
    duplicate_paths.execution_paths.push(execution_path());
    assert!(matches!(
        CompiledGenerationV1::admit_owner_preimage(duplicate_paths),
        Err(InvalidResidentRecordV1::NonCanonical { .. })
    ));

    let mut duplicate_repositories = compiled_generation_preimage();
    duplicate_repositories
        .repository_source_frontiers
        .push(duplicate_repositories.repository_source_frontiers[0].clone());
    assert!(matches!(
        CompiledGenerationV1::admit_owner_preimage(duplicate_repositories),
        Err(InvalidResidentRecordV1::NonCanonical { .. })
    ));
}

#[test]
fn generation_heads_admit_only_exact_publish_advertise_activate_chain() {
    let generation = compiled_generation();
    let published = published_head(&generation, 1, None);
    PublishedGenerationHeadV1::admit_contextual_wire(
        wire_from(&published),
        &PublishedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: published.schema_sha256(),
            compiled_generation: &generation,
            predecessor: None,
        },
    )
    .expect("initial publication admits");

    let advertised = advertised_head(&published, 2, None);
    AdvertisedGenerationHeadV1::admit_contextual_wire(
        wire_from(&advertised),
        &AdvertisedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: advertised.schema_sha256(),
            published_head: &published,
            predecessor: None,
        },
    )
    .expect("published generation advertises");

    let fence = activation_fence(&generation, 1, None);
    ActivationFenceV1::admit_contextual_wire(
        wire_from(&fence),
        &ActivationFenceAdmissionContextV1 {
            expected_schema_sha256: fence.schema_sha256(),
            compiled_generation: &generation,
            predecessor: None,
        },
    )
    .expect("initial fence admits");

    let active = active_head(&advertised, &fence, 3, None);
    ActiveGenerationHeadV1::admit_contextual_wire(
        wire_from(&active),
        &ActiveGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: active.schema_sha256(),
            advertised_head: &advertised,
            activation_fence: &fence,
            predecessor: None,
        },
    )
    .expect("advertised and fenced generation activates");
}

#[test]
fn generation_head_lineages_accept_interleaving_and_reject_non_advance() {
    let generation = compiled_generation();
    let published_initial = published_head(&generation, 2, None);
    let published_next = published_head(&generation, 7, Some(&published_initial));
    PublishedGenerationHeadV1::admit_contextual_wire(
        wire_from(&published_next),
        &PublishedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: published_next.schema_sha256(),
            compiled_generation: &generation,
            predecessor: Some(&published_initial),
        },
    )
    .expect("interleaved publication successor admits");
    let published_non_advance = published_head(&generation, 2, Some(&published_initial));
    assert!(PublishedGenerationHeadV1::admit_contextual_wire(
        wire_from(&published_non_advance),
        &PublishedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: published_non_advance.schema_sha256(),
            compiled_generation: &generation,
            predecessor: Some(&published_initial),
        },
    )
    .is_err());
    let substituted_predecessor = published_head(&generation, 3, None);
    assert!(PublishedGenerationHeadV1::admit_contextual_wire(
        wire_from(&published_next),
        &PublishedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: published_next.schema_sha256(),
            compiled_generation: &generation,
            predecessor: Some(&substituted_predecessor),
        },
    )
    .is_err());

    let advertised_initial = advertised_head(&published_initial, 3, None);
    let advertised_next = advertised_head(&published_next, 8, Some(&advertised_initial));
    AdvertisedGenerationHeadV1::admit_contextual_wire(
        wire_from(&advertised_next),
        &AdvertisedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: advertised_next.schema_sha256(),
            published_head: &published_next,
            predecessor: Some(&advertised_initial),
        },
    )
    .expect("interleaved advertisement successor admits");
    let advertised_non_advance = advertised_head(&published_next, 3, Some(&advertised_initial));
    assert!(AdvertisedGenerationHeadV1::admit_contextual_wire(
        wire_from(&advertised_non_advance),
        &AdvertisedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: advertised_non_advance.schema_sha256(),
            published_head: &published_next,
            predecessor: Some(&advertised_initial),
        },
    )
    .is_err());

    let mut alternate_generation_preimage = compiled_generation_preimage();
    alternate_generation_preimage.profile_digest =
        ProfileDigest::parse(sha('f')).expect("alternate profile digest");
    let alternate_generation =
        CompiledGenerationV1::admit_owner_preimage(alternate_generation_preimage)
            .expect("alternate compiled generation");
    let alternate_published = published_head(&alternate_generation, 1, None);
    assert!(AdvertisedGenerationHeadV1::admit_contextual_wire(
        wire_from(&advertised_next),
        &AdvertisedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: advertised_next.schema_sha256(),
            published_head: &alternate_published,
            predecessor: Some(&advertised_initial),
        },
    )
    .is_err());
}

#[test]
fn activation_fence_requires_stable_identity_and_exact_successor() {
    let generation = compiled_generation();
    let initial = activation_fence(&generation, 1, None);
    let successor = activation_fence(&generation, 2, Some(&initial));
    ActivationFenceV1::admit_contextual_wire(
        wire_from(&successor),
        &ActivationFenceAdmissionContextV1 {
            expected_schema_sha256: successor.schema_sha256(),
            compiled_generation: &generation,
            predecessor: Some(&initial),
        },
    )
    .expect("exact fence successor admits");

    let jump = activation_fence(&generation, 3, Some(&initial));
    assert!(ActivationFenceV1::admit_contextual_wire(
        wire_from(&jump),
        &ActivationFenceAdmissionContextV1 {
            expected_schema_sha256: jump.schema_sha256(),
            compiled_generation: &generation,
            predecessor: Some(&initial),
        },
    )
    .is_err());

    let changed_identity = ActivationFenceV1::seal(ActivationFencePreimageV1 {
        schema: SchemaId::parse(ACTIVATION_FENCE_V1_SCHEMA).expect("fence schema"),
        schema_sha256: successor.schema_sha256().clone(),
        schema_version: ACTIVATION_FENCE_V1_SCHEMA_VERSION,
        activation_fence_id: ActivationFenceIdV1::parse("other-fence").expect("fence id"),
        activation_fence_generation: ActivationFenceGenerationV1::new(2).expect("fence generation"),
        compiled_generation_digest: generation.compiled_generation_digest().clone(),
        predecessor_activation_fence_digest: Some(initial.activation_fence_digest().clone()),
    })
    .expect("structurally valid changed fence identity");
    assert!(ActivationFenceV1::admit_contextual_wire(
        wire_from(&changed_identity),
        &ActivationFenceAdmissionContextV1 {
            expected_schema_sha256: changed_identity.schema_sha256(),
            compiled_generation: &generation,
            predecessor: Some(&initial),
        },
    )
    .is_err());
}

#[test]
fn active_head_successor_accepts_interleaving_and_rejects_non_advance() {
    let generation = compiled_generation();
    let published = published_head(&generation, 1, None);
    let advertised = advertised_head(&published, 2, None);
    let fence = activation_fence(&generation, 1, None);
    let initial = active_head(&advertised, &fence, 3, None);
    let successor = active_head(&advertised, &fence, 9, Some(&initial));
    ActiveGenerationHeadV1::admit_contextual_wire(
        wire_from(&successor),
        &ActiveGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: successor.schema_sha256(),
            advertised_head: &advertised,
            activation_fence: &fence,
            predecessor: Some(&initial),
        },
    )
    .expect("interleaved active successor admits");

    let non_advance = active_head(&advertised, &fence, 3, Some(&initial));
    assert!(ActiveGenerationHeadV1::admit_contextual_wire(
        wire_from(&non_advance),
        &ActiveGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: non_advance.schema_sha256(),
            advertised_head: &advertised,
            activation_fence: &fence,
            predecessor: Some(&initial),
        },
    )
    .is_err());

    let other_fence = ActivationFenceV1::seal(ActivationFencePreimageV1 {
        schema: SchemaId::parse(ACTIVATION_FENCE_V1_SCHEMA).expect("fence schema"),
        schema_sha256: fence.schema_sha256().clone(),
        schema_version: ACTIVATION_FENCE_V1_SCHEMA_VERSION,
        activation_fence_id: ActivationFenceIdV1::parse("other-fence").expect("fence id"),
        activation_fence_generation: ActivationFenceGenerationV1::new(1).expect("fence generation"),
        compiled_generation_digest: generation.compiled_generation_digest().clone(),
        predecessor_activation_fence_digest: None,
    })
    .expect("other initial fence");
    assert!(ActiveGenerationHeadV1::admit_contextual_wire(
        wire_from(&successor),
        &ActiveGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: successor.schema_sha256(),
            advertised_head: &advertised,
            activation_fence: &other_fence,
            predecessor: Some(&initial),
        },
    )
    .is_err());
}

#[test]
fn session_admission_binds_active_head_generation_fence_route_and_repository() {
    let profile = effective_profile();
    let generation = compiled_generation();
    let published = published_head(&generation, 1, None);
    let advertised = advertised_head(&published, 1, None);
    let fence = activation_fence(&generation, 1, None);
    let active = active_head(&advertised, &fence, 1, None);
    let mut preimage = parent_session_preimage("session-1", 10);
    preimage.compiled_generation_digest = generation.compiled_generation_digest().clone();
    preimage.active_generation_head_digest = active.active_generation_head_digest().clone();
    preimage.activation_fence_digest = fence.activation_fence_digest().clone();
    preimage.profile_digest = generation.profile_digest().clone();
    preimage.instance_digest = generation.instance_digest().clone();
    let session = seal_session(preimage.clone()).expect("session seals");
    let authority = session_signing_authority();
    let context = SessionAdmissionLeaseContextV1 {
        active_head: &active,
        compiled_generation: &generation,
        effective_profile: &profile,
        activation_fence: &fence,
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: session.schema_sha256(),
            signing_authority: &authority,
        },
    };
    SessionAdmissionLeaseV1::admit_contextual_wire(wire_from(&session), &context, |_, _, _, _| {
        Ok(())
    })
    .expect("exact active generation state admits session");

    let mut wrong_head = preimage.clone();
    wrong_head.active_generation_head_digest =
        ActiveGenerationHeadDigest::parse(sha('e')).expect("wrong active head digest");
    let wrong_head = seal_session(wrong_head).expect("session shape seals");
    assert!(SessionAdmissionLeaseV1::admit_contextual_wire(
        wire_from(&wrong_head),
        &context,
        |_, _, _, _| Ok(())
    )
    .is_err());

    let mut wrong_generation = preimage.clone();
    wrong_generation.compiled_generation_digest =
        CompiledGenerationDigest::parse(sha('e')).expect("wrong generation digest");
    let wrong_generation = seal_session(wrong_generation).expect("session shape seals");
    assert!(SessionAdmissionLeaseV1::admit_contextual_wire(
        wire_from(&wrong_generation),
        &context,
        |_, _, _, _| Ok(())
    )
    .is_err());

    let mut wrong_fence = preimage.clone();
    wrong_fence.activation_fence_digest =
        ActivationFenceDigest::parse(sha('e')).expect("wrong fence digest");
    let wrong_fence = seal_session(wrong_fence).expect("session shape seals");
    assert!(SessionAdmissionLeaseV1::admit_contextual_wire(
        wire_from(&wrong_fence),
        &context,
        |_, _, _, _| Ok(())
    )
    .is_err());

    let mut wrong_path = preimage.clone();
    wrong_path.execution_path = alternate_execution_path();
    let wrong_path = seal_session(wrong_path).expect("session shape seals");
    assert!(SessionAdmissionLeaseV1::admit_contextual_wire(
        wire_from(&wrong_path),
        &context,
        |_, _, _, _| Ok(())
    )
    .is_err());

    let mut wrong_project = preimage.clone();
    wrong_project.project_id = LogicalProjectIdV1::parse("project-2").expect("project id");
    let wrong_project = seal_session(wrong_project).expect("session shape seals");
    assert!(SessionAdmissionLeaseV1::admit_contextual_wire(
        wire_from(&wrong_project),
        &context,
        |_, _, _, _| Ok(())
    )
    .is_err());

    let mut wrong_execution_workspace = preimage.clone();
    wrong_execution_workspace.execution_workspace_id =
        LogicalExecutionWorkspaceIdV1::parse("execution-workspace-2")
            .expect("execution workspace id");
    let wrong_execution_workspace =
        seal_session(wrong_execution_workspace).expect("session shape seals");
    assert!(SessionAdmissionLeaseV1::admit_contextual_wire(
        wire_from(&wrong_execution_workspace),
        &context,
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let mut wrong_repository = preimage;
    wrong_repository.repository_id =
        LogicalRepositoryIdV1::parse("repository-2").expect("repository id");
    let wrong_repository = seal_session(wrong_repository).expect("session shape seals");
    assert!(SessionAdmissionLeaseV1::admit_contextual_wire(
        wire_from(&wrong_repository),
        &context,
        |_, _, _, _| Ok(())
    )
    .is_err());
}

#[test]
fn session_admission_requires_one_exact_signed_runtime_authority_coordinate() {
    let profile = effective_profile();
    let generation = compiled_generation();
    let published = published_head(&generation, 1, None);
    let advertised = advertised_head(&published, 1, None);
    let fence = activation_fence(&generation, 1, None);
    let active = active_head(&advertised, &fence, 1, None);
    let mut preimage = parent_session_preimage("session-authority", 10);
    preimage.compiled_generation_digest = generation.compiled_generation_digest().clone();
    preimage.active_generation_head_digest = active.active_generation_head_digest().clone();
    preimage.activation_fence_digest = fence.activation_fence_digest().clone();
    preimage.profile_digest = generation.profile_digest().clone();
    preimage.instance_digest = generation.instance_digest().clone();

    let authority = session_signing_authority();
    let context = SessionAdmissionLeaseContextV1 {
        active_head: &active,
        compiled_generation: &generation,
        effective_profile: &profile,
        activation_fence: &fence,
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            signing_authority: &authority,
        },
    };

    let exact = seal_session(preimage.clone()).expect("exact session seals");
    SessionAdmissionLeaseV1::admit_contextual_wire(
        wire_from(&exact),
        &context,
        |_, _, _, _| Ok(()),
    )
    .expect("exact historical authority remains admissible");

    let mut missing_ref = serde_json::to_value(wire_from::<_, SessionAdmissionLeaseWireV1>(&exact))
        .expect("session wire serializes");
    missing_ref
        .as_object_mut()
        .expect("session wire object")
        .remove("deployment_runtime_authority_ref");
    assert!(serde_json::from_value::<SessionAdmissionLeaseWireV1>(missing_ref).is_err());

    let mut wrong_ref = preimage.clone();
    wrong_ref.deployment_runtime_authority_ref = record_ref("runtime-authority-2");
    let wrong_ref = seal_session(wrong_ref).expect("wrong-ref session seals structurally");
    assert!(SessionAdmissionLeaseV1::admit_contextual_wire(
        wire_from(&wrong_ref),
        &context,
        |_, _, _, _| Ok(())
    )
    .is_err());

    let mut wrong_digest = preimage.clone();
    wrong_digest.deployment_runtime_authority_sha256 =
        ArtifactDigest::parse(sha('7')).expect("other runtime authority digest");
    let wrong_digest = seal_session(wrong_digest).expect("wrong-digest session seals structurally");
    assert!(SessionAdmissionLeaseV1::admit_contextual_wire(
        wire_from(&wrong_digest),
        &context,
        |_, _, _, _| Ok(())
    )
    .is_err());

    let unrelated_authority = SigningAuthorityBindingV1::integrated_runtime(
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        ResidentKeyIdV1::parse("runtime-key-1").expect("same runtime key"),
        SigningKeyEpochV1::new(1).expect("same runtime key epoch"),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: record_ref("runtime-authority-2"),
            deployment_runtime_authority_sha256: ArtifactDigest::parse(sha('7'))
                .expect("unrelated runtime authority digest"),
            deployment_binding_ref: record_ref("deployment-binding"),
            deployment_binding_sha256: ArtifactDigest::parse(sha('d'))
                .expect("deployment binding digest"),
            checked_principal_trust_checkpoint_ref: record_ref("trust-checkpoint"),
            checked_principal_trust_checkpoint_sha256: ArtifactDigest::parse(sha('5'))
                .expect("trust checkpoint digest"),
        },
    )
    .expect("same-key unrelated authority");
    let unrelated_context = SessionAdmissionLeaseContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            signing_authority: &unrelated_authority,
        },
        ..context
    };
    assert!(SessionAdmissionLeaseV1::admit_contextual_wire(
        wire_from(&exact),
        &unrelated_context,
        |_, _, _, _| Ok(())
    )
    .is_err());

    let mut wrong_signer = preimage.clone();
    wrong_signer.issuer_key_id = ResidentKeyIdV1::parse("runtime-key-2").expect("other key");
    let wrong_signer = seal_session(wrong_signer).expect("wrong-signer session seals structurally");
    assert!(SessionAdmissionLeaseV1::admit_contextual_wire(
        wire_from(&wrong_signer),
        &context,
        |_, _, _, _| Ok(())
    )
    .is_err());
}

#[test]
fn operation_lease_schema_contains_no_self_attested_ambient_axes() {
    let schema =
        serde_json::to_value(schemars::schema_for!(OperationLeaseV1)).expect("schema serializes");
    let properties = schema
        .pointer("/properties")
        .and_then(serde_json::Value::as_object)
        .expect("root properties");
    assert!(properties.contains_key("session_admission_id"));
    assert!(properties.contains_key("session_admission_lease_digest"));
    assert!(properties
        .keys()
        .all(|field| !field.starts_with("ambient_")));
}

#[test]
fn operation_lease_binds_exact_parent_identity_and_digest() {
    let parent = parent_session("session-1", 10);
    let lease = OperationLeaseV1::seal(
        SchemaDigest::parse(sha('9')).expect("operation schema digest"),
        &parent,
        operation_request(),
    )
    .expect("narrow child lease");

    assert!(lease.verify_against_parent(&parent).is_ok());
    let identity_mismatch = parent_session("session-2", 10);
    assert!(matches!(
        lease.verify_against_parent(&identity_mismatch),
        Err(InvalidResidentRecordV1::LeaseWidening)
    ));
    let digest_mismatch = parent_session("session-1", 20);
    assert_eq!(
        lease.session_admission_id(),
        digest_mismatch.session_admission_id()
    );
    assert_ne!(
        lease.session_admission_lease_digest(),
        digest_mismatch.session_admission_lease_digest()
    );
    assert!(matches!(
        lease.verify_against_parent(&digest_mismatch),
        Err(InvalidResidentRecordV1::LeaseWidening)
    ));
}

#[test]
fn operation_lease_wire_requires_the_exact_resolved_parent() {
    let parent = parent_session("session-1", 10);
    let lease = OperationLeaseV1::seal(
        SchemaDigest::parse(sha('9')).expect("operation schema digest"),
        &parent,
        operation_request(),
    )
    .expect("narrow child lease");
    let encoded = serde_json::to_value(&lease).expect("lease serializes");
    let wire: OperationLeaseWireV1 =
        serde_json::from_value(encoded.clone()).expect("structural wire decodes");
    assert_eq!(
        wire.session_admission_lease_digest(),
        parent.session_admission_lease_digest(),
        "the untrusted wire exposes only the parent coordinate needed for resolution"
    );
    OperationLeaseV1::admit_wire(wire, &parent, lease.schema_sha256())
        .expect("exact parent admits");

    let wrong_parent = parent_session("session-2", 10);
    let wrong_wire: OperationLeaseWireV1 =
        serde_json::from_value(encoded).expect("structural wire decodes");
    assert!(matches!(
        OperationLeaseV1::admit_wire(wrong_wire, &wrong_parent, lease.schema_sha256()),
        Err(InvalidResidentRecordV1::LeaseWidening)
    ));
}

#[test]
fn operation_lease_narrowing_rejects_each_widened_axis() {
    #[derive(Clone, Copy)]
    enum Axis {
        Capabilities,
        Effects,
        Reads,
        Evidence,
        Budget,
        Timeout,
        Expiry,
    }

    let parent = parent_session("session-1", 10);
    OperationLeaseV1::seal(
        SchemaDigest::parse(sha('9')).expect("operation schema digest"),
        &parent,
        operation_request(),
    )
    .expect("every child axis is a lawful strict narrowing");

    let cases = [
        ("capabilities", Axis::Capabilities),
        ("effects", Axis::Effects),
        ("reads", Axis::Reads),
        ("evidence", Axis::Evidence),
        ("budget", Axis::Budget),
        ("timeout", Axis::Timeout),
        ("expiry", Axis::Expiry),
    ];
    for (name, axis) in cases {
        let mut request = operation_request();
        match axis {
            Axis::Capabilities => request
                .capabilities
                .push(CapabilityIdV1::parse("write").expect("capability")),
            Axis::Effects => request.effects.push(EffectClass::Publish),
            Axis::Reads => request.read_set.push(record_ref("read-z")),
            Axis::Evidence => {
                request.evidence_obligations.remove(0);
            }
            Axis::Budget => {
                request.budget_units = LeaseBudgetUnitsV1::new(11).expect("valid widened budget")
            }
            Axis::Timeout => {
                request.timeout_millis =
                    LeaseTimeoutMillisV1::new(1_001).expect("valid widened timeout")
            }
            Axis::Expiry => request.expires_at = UnixTimeV1::new(101).expect("valid fixture time"),
        }
        assert!(
            matches!(
                OperationLeaseV1::seal(
                    SchemaDigest::parse(sha('9')).expect("operation schema digest"),
                    &parent,
                    request,
                ),
                Err(InvalidResidentRecordV1::LeaseWidening)
            ),
            "{name} widening must fail closed"
        );
    }
}

#[test]
fn operation_lease_may_add_bounded_evidence_obligations() {
    let parent = parent_session("session-1", 10);
    let mut request = operation_request();
    request.evidence_obligations.push(record_ref("evidence-z"));
    OperationLeaseV1::seal(
        SchemaDigest::parse(sha('9')).expect("operation schema digest"),
        &parent,
        request,
    )
    .expect("child may retain every parent obligation and add a bounded obligation");
}

#[test]
fn admitted_resident_record_projects_once_into_an_inspectable_untrusted_wire() {
    let generation = compiled_generation();
    let head = published_head(&generation, 1, None);
    let wire = PublishedGenerationHeadWireV1::from(&head);
    assert_eq!(wire.schema(), head.schema());
    assert_eq!(wire.schema_sha256(), head.schema_sha256());
    assert_eq!(wire.schema_version(), head.schema_version());
    assert_eq!(
        wire.compiled_generation_digest(),
        head.compiled_generation_digest()
    );
    assert_eq!(wire.journal_sequence(), head.journal_sequence());
    assert_eq!(
        wire.published_generation_head_digest(),
        head.published_generation_head_digest()
    );
    let schema = head.schema_sha256().clone();
    let readmitted = PublishedGenerationHeadV1::admit_contextual_wire(
        wire,
        &PublishedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: &schema,
            compiled_generation: &generation,
            predecessor: None,
        },
    )
    .expect("projected wire re-admits only with exact owner context");
    assert_eq!(readmitted, head);
}

#[test]
fn session_admission_lease_rejects_each_revocation_half_pair() {
    let mut ref_only = parent_session_preimage("session-1", 10);
    ref_only.checked_revocation_ref = Some(record_ref("revocation"));
    assert!(seal_session(ref_only).is_err());

    let mut digest_only = parent_session_preimage("session-1", 10);
    digest_only.checked_revocation_sha256 =
        Some(ArtifactDigest::parse(sha('8')).expect("revocation digest"));
    assert!(seal_session(digest_only).is_err());
}

#[test]
fn resident_public_root_inventory_is_unique_and_contains_required_authority_roots() {
    let unique = RESIDENT_PUBLIC_ROOT_SCHEMA_IDS_V1
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), RESIDENT_PUBLIC_ROOT_SCHEMA_IDS_V1.len());
    for required in [
        ADAPTER_FAILURE_V1_SCHEMA_ID,
        CAPABILITY_RESULT_V1_SCHEMA_ID,
        COMPILED_GENERATION_V1_SCHEMA_ID,
        PRINCIPAL_RECORD_V1_SCHEMA_ID,
        PRINCIPAL_RECOVERY_POLICY_V1_SCHEMA_ID,
        LOCAL_ADMIN_AUTHORITY_V1_SCHEMA_ID,
        PRODUCER_INCARNATION_AUTHORITY_V1_SCHEMA_ID,
        CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA_ID,
        PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA_ID,
        PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SCHEMA_ID,
        PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA_ID,
        RESOURCE_CONTINUITY_PROFILE_V1_SCHEMA_ID,
        PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA_ID,
        PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA_ID,
        PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA_ID,
        PRODUCER_INCARNATION_V1_SCHEMA_ID,
        CONTRACT_SET_V1_SCHEMA_ID,
        DEPLOYMENT_TERMS_V1_SCHEMA_ID,
        DEPLOYMENT_BINDING_PROPOSAL_V1_SCHEMA_ID,
        DEPLOYMENT_ACCEPTANCE_V1_SCHEMA_ID,
        DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SCHEMA_ID,
        DEPLOYMENT_BINDING_V1_SCHEMA_ID,
        DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA_ID,
    ] {
        assert!(
            unique.contains(required),
            "missing resident root `{required}`"
        );
    }
}
