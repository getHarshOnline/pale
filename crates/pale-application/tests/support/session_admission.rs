//! Shared contextual session and child-lease fixtures.
//! Sealed record and current admission checks are real; signatures, trust heads,
//! materialization and external authority answers are declared test models.

use super::principal_trust_test_support as principal_trust_bootstrap;
use super::resident::*;
use pale_core::records::*;
use pale_ports::AuthorityStoreCoordinate;
use serde::{de::DeserializeOwned, Serialize};

pub(crate) fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

pub(crate) fn schema_digest(byte: char) -> SchemaDigest {
    SchemaDigest::parse(sha(byte)).expect("schema digest")
}

pub(crate) fn artifact(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(sha(byte)).expect("artifact digest")
}

pub(crate) fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("resident record reference")
}

pub(crate) fn unsigned_wire<P, D, W>(preimage: &P, digest_field: &str, digest: &D) -> W
where
    P: Serialize,
    D: Serialize,
    W: DeserializeOwned,
{
    let mut value = serde_json::to_value(preimage).expect("preimage serializes");
    value
        .as_object_mut()
        .expect("preimage is an object")
        .insert(
            digest_field.into(),
            serde_json::to_value(digest).expect("digest serializes"),
        );
    serde_json::from_value(value).expect("unsigned wire decodes")
}

pub(crate) fn execution_path() -> ExecutionPathSelectionV1 {
    ExecutionPathSelectionV1 {
        id: ExecutionPathSelectionIdV1::parse("direct-codex").expect("execution path id"),
        experience_path_id: "direct/codex".into(),
    }
}

pub(crate) fn effective_profile() -> EffectiveProjectProfileV1 {
    effective_profile_with_run_policy("No lifecycle gate.")
}

pub(crate) fn effective_profile_with_run_policy(
    run_policy_description: &str,
) -> EffectiveProjectProfileV1 {
    EffectiveProjectProfileV1::seal(EffectiveProjectProfilePreimageV1 {
        schema: SchemaId::parse(EFFECTIVE_PROJECT_PROFILE_SCHEMA).expect("profile schema"),
        schema_sha256: schema_digest('0'),
        schema_version: EFFECTIVE_PROJECT_PROFILE_SCHEMA_VERSION,
        workspace_id: WorkspaceId::parse("workspace-1").expect("workspace id"),
        project_id: LogicalProjectIdV1::parse("project-1").expect("project id"),
        execution_workspace_id: LogicalExecutionWorkspaceIdV1::parse("execution-workspace-1")
            .expect("execution workspace id"),
        execution_paths: vec![execution_path()],
        run_policy: RunPolicyEntry {
            id: RunPolicyIdV1::parse("inert").expect("run policy id"),
            description: run_policy_description.into(),
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

pub(crate) fn compiled_generation(profile: &EffectiveProjectProfileV1) -> CompiledGenerationV1 {
    compiled_generation_with_instance(profile, '2')
}

pub(crate) fn compiled_generation_with_instance(
    profile: &EffectiveProjectProfileV1,
    instance_digest: char,
) -> CompiledGenerationV1 {
    CompiledGenerationV1::admit_owner_preimage(CompiledGenerationPreimageV1 {
        schema: SchemaId::parse(COMPILED_GENERATION_V1_SCHEMA).expect("generation schema"),
        schema_sha256: schema_digest('1'),
        schema_version: COMPILED_GENERATION_V1_SCHEMA_VERSION,
        distribution_lock_digest: DistributionLockDigest::parse(sha('1'))
            .expect("distribution lock digest"),
        profile_digest: profile.profile_digest.clone(),
        instance_digest: InstanceDigest::parse(sha(instance_digest)).expect("instance digest"),
        package_semantic_ir_digest: PackageSemanticIrDigest::parse(sha('3'))
            .expect("Package semantic IR digest"),
        profile_package_release_closure: Vec::new(),
        profile_normative_pack_release_closure: Vec::new(),
        schema_manifest_digest: SchemaManifestDigest::parse(sha('4'))
            .expect("schema manifest digest"),
        schema_set_digest: SchemaSetDigest::parse(sha('5')).expect("schema set digest"),
        workspace_semantic_ir_digest: WorkspaceSemanticIrDigest::parse(sha('6'))
            .expect("workspace semantic IR digest"),
        workspace_admission_evidence_digest: WorkspaceAdmissionEvidenceDigest::parse(sha('7'))
            .expect("workspace admission evidence digest"),
        principal_admitted_universe_digest: PrincipalAdmittedUniverseDigest::parse(sha('8'))
            .expect("principal universe digest"),
        workspace_module_release_closure: Vec::new(),
        workspace_package_release_closure: Vec::new(),
        workspace_normative_pack_release_closure: Vec::new(),
        workspace_resolution_lock_digest: WorkspaceResolutionLockDigest::parse(sha('9'))
            .expect("workspace resolution digest"),
        workspace_universe_lock_digest: WorkspaceUniverseLockDigest::parse(sha('a'))
            .expect("workspace universe digest"),
        responsibility_derivation_closure_digest: ResponsibilityDerivationClosureDigest::parse(
            sha('b'),
        )
        .expect("responsibility closure digest"),
        organizational_translation_closure_digest: OrganizationalTranslationClosureDigest::parse(
            sha('c'),
        )
        .expect("organization closure digest"),
        experience_composition_source_digest: ExperienceCompositionSourceDigest::parse(sha('d'))
            .expect("composition source digest"),
        provider_binding_closure_digest: ProviderBindingClosureDigest::parse(sha('e'))
            .expect("provider binding digest"),
        experience_composition_plan_digest: ExperienceCompositionPlanDigest::parse(sha('f'))
            .expect("composition plan digest"),
        execution_paths: vec![execution_path()],
        materialization_closure_digest: MaterializationClosureDigest::parse(sha('1'))
            .expect("materialization digest"),
        machine_overlay_digest: MachineOverlayDigest::parse(sha('2'))
            .expect("machine overlay digest"),
        executable_identity_digest: ExecutableIdentityDigest::parse(sha('3'))
            .expect("executable identity digest"),
        compilation_input_digest: CompilationInputDigest::parse(sha('4'))
            .expect("compilation input digest"),
        one_shot_compilation_digest: OneShotCompilationDigest::parse(sha('5'))
            .expect("one-shot compilation digest"),
        repository_source_frontiers: vec![RepositorySourceFrontierV1 {
            repository_id: LogicalRepositoryIdV1::parse("repository-1").expect("repository id"),
            base_revision: RepositoryRevisionV1::parse("revision-1").expect("revision"),
            tracked_delta_sha256: artifact('6'),
            untracked_inventory_sha256: artifact('7'),
            dependency_frontier_sha256: artifact('8'),
        }],
    })
    .expect("compiled generation admits")
}

pub(crate) fn active_generation(
    generation: &CompiledGenerationV1,
) -> (ActivationFenceV1, ActiveGenerationHeadV1) {
    active_generation_with_fence(generation, "activation-fence-1")
}

fn registered_active_generation(
    generation: &CompiledGenerationV1,
) -> (ActivationFenceV1, ActiveGenerationHeadV1) {
    let published = crate::resident_fixture::published(generation, 1, None);
    let advertised = crate::resident_fixture::advertised(&published, 2, None);
    let fence = crate::resident_fixture::fence(generation, 1, None);
    let active = crate::resident_fixture::active(&advertised, &fence, 3, None);
    (fence, active)
}

pub(crate) fn active_generation_with_fence(
    generation: &CompiledGenerationV1,
    activation_fence_id: &str,
) -> (ActivationFenceV1, ActiveGenerationHeadV1) {
    let published_preimage = PublishedGenerationHeadPreimageV1 {
        schema: SchemaId::parse(PUBLISHED_GENERATION_HEAD_V1_SCHEMA).expect("published schema"),
        schema_sha256: schema_digest('2'),
        schema_version: PUBLISHED_GENERATION_HEAD_V1_SCHEMA_VERSION,
        compiled_generation_digest: generation.compiled_generation_digest().clone(),
        journal_sequence: JournalSequenceV1::new(1).expect("journal sequence"),
        predecessor_published_generation_head_digest: None,
    };
    let published_digest = PublishedGenerationHeadDigest::hash_canonical(&published_preimage)
        .expect("published head digest");
    let published = PublishedGenerationHeadV1::admit_contextual_wire(
        unsigned_wire(
            &published_preimage,
            "published_generation_head_digest",
            &published_digest,
        ),
        &PublishedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: &published_preimage.schema_sha256,
            compiled_generation: generation,
            predecessor: None,
        },
    )
    .expect("published head admits");
    let advertised_preimage = AdvertisedGenerationHeadPreimageV1 {
        schema: SchemaId::parse(ADVERTISED_GENERATION_HEAD_V1_SCHEMA).expect("advertised schema"),
        schema_sha256: schema_digest('3'),
        schema_version: ADVERTISED_GENERATION_HEAD_V1_SCHEMA_VERSION,
        compiled_generation_digest: generation.compiled_generation_digest().clone(),
        published_generation_head_digest: published.published_generation_head_digest().clone(),
        journal_sequence: JournalSequenceV1::new(2).expect("journal sequence"),
        predecessor_advertised_generation_head_digest: None,
    };
    let advertised_digest = AdvertisedGenerationHeadDigest::hash_canonical(&advertised_preimage)
        .expect("advertised head digest");
    let advertised = AdvertisedGenerationHeadV1::admit_contextual_wire(
        unsigned_wire(
            &advertised_preimage,
            "advertised_generation_head_digest",
            &advertised_digest,
        ),
        &AdvertisedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: &advertised_preimage.schema_sha256,
            published_head: &published,
            predecessor: None,
        },
    )
    .expect("advertised head admits");
    let fence_preimage = ActivationFencePreimageV1 {
        schema: SchemaId::parse(ACTIVATION_FENCE_V1_SCHEMA).expect("activation schema"),
        schema_sha256: schema_digest('4'),
        schema_version: ACTIVATION_FENCE_V1_SCHEMA_VERSION,
        activation_fence_id: ActivationFenceIdV1::parse(activation_fence_id)
            .expect("activation fence id"),
        activation_fence_generation: ActivationFenceGenerationV1::new(1)
            .expect("activation generation"),
        compiled_generation_digest: generation.compiled_generation_digest().clone(),
        predecessor_activation_fence_digest: None,
    };
    let fence_digest =
        ActivationFenceDigest::hash_canonical(&fence_preimage).expect("activation fence digest");
    let fence = ActivationFenceV1::admit_contextual_wire(
        unsigned_wire(&fence_preimage, "activation_fence_digest", &fence_digest),
        &ActivationFenceAdmissionContextV1 {
            expected_schema_sha256: &fence_preimage.schema_sha256,
            compiled_generation: generation,
            predecessor: None,
        },
    )
    .expect("activation fence admits");
    let active_preimage = ActiveGenerationHeadPreimageV1 {
        schema: SchemaId::parse(ACTIVE_GENERATION_HEAD_V1_SCHEMA).expect("active schema"),
        schema_sha256: schema_digest('5'),
        schema_version: ACTIVE_GENERATION_HEAD_V1_SCHEMA_VERSION,
        compiled_generation_digest: generation.compiled_generation_digest().clone(),
        advertised_generation_head_digest: advertised.advertised_generation_head_digest().clone(),
        activation_fence_digest: fence.activation_fence_digest().clone(),
        journal_sequence: JournalSequenceV1::new(3).expect("journal sequence"),
        predecessor_active_generation_head_digest: None,
    };
    let active_digest =
        ActiveGenerationHeadDigest::hash_canonical(&active_preimage).expect("active head digest");
    let active = ActiveGenerationHeadV1::admit_contextual_wire(
        unsigned_wire(
            &active_preimage,
            "active_generation_head_digest",
            &active_digest,
        ),
        &ActiveGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: &active_preimage.schema_sha256,
            advertised_head: &advertised,
            activation_fence: &fence,
            predecessor: None,
        },
    )
    .expect("active head admits");
    (fence, active)
}

pub(crate) fn principal_key<'a>(
    principal: &'a ResidentPrincipalIdV1,
    key: &'a ResidentKeyIdV1,
) -> PrincipalKeyAdmissionContextV1<'a> {
    PrincipalKeyAdmissionContextV1::new(
        principal,
        key,
        PrincipalKeyEpochV1::new(1).expect("principal key epoch"),
    )
}

pub(crate) fn deployment_binding(
    validity: (u64, u64),
) -> (DeploymentBindingV1, DeploymentAcceptanceV1) {
    deployment_binding_named(validity, "1")
}

/// Build one admitted deployment binding with distinct test identities.
pub(crate) fn deployment_binding_named(
    validity: (u64, u64),
    identity: &str,
) -> (DeploymentBindingV1, DeploymentAcceptanceV1) {
    let principal = ResidentPrincipalIdV1::parse("principal-1").expect("principal");
    let root_key = ResidentKeyIdV1::parse("root-key-1").expect("root key");
    let terms_preimage = DeploymentTermsPreimageV1 {
        schema_id: SchemaId::parse(DEPLOYMENT_TERMS_V1_SCHEMA).expect("terms schema"),
        schema_sha256: schema_digest('6'),
        workspace_id: WorkspaceId::parse(format!("workspace-{identity}")).expect("workspace id"),
        workspace_commit: WorkspaceCommitV1::parse(format!("commit-{identity}"))
            .expect("workspace commit"),
        deployment_id: DeploymentIdV1::parse(format!("deployment-{identity}"))
            .expect("deployment id"),
        distribution_release: DistributionReleaseV1::parse(format!("release-{identity}"))
            .expect("distribution release"),
        distribution_release_sha256: DistributionReleaseDigest::parse(sha('1'))
            .expect("release digest"),
        principal_scope: principal.clone(),
        controller_scope: ResidentControllerIdV1::parse(format!("controller-{identity}"))
            .expect("controller"),
        relationship_scope: None,
        environment: DeploymentEnvironmentV1::parse("production").expect("environment"),
        environment_sha256: DeploymentEnvironmentDigest::parse(sha('2'))
            .expect("environment digest"),
        processing_role_ref: record_ref(&format!("processing-role-{identity}")),
        processing_role_sha256: ProcessingRoleDigest::parse(sha('3'))
            .expect("processing-role digest"),
        confidentiality_policy_ref: record_ref(&format!("confidentiality-policy-{identity}")),
        confidentiality_policy_sha256: ConfidentialityPolicyDigest::parse(sha('4'))
            .expect("confidentiality-policy digest"),
        data_classification: DataClassificationV1::parse("confidential").expect("classification"),
        permitted_purposes: vec![
            PermittedPurposeV1::parse("operations").expect("purpose"),
            PermittedPurposeV1::parse("review").expect("purpose"),
        ],
    };
    let terms_digest =
        DeploymentTermsDigest::hash_canonical(&terms_preimage).expect("deployment terms digest");
    let terms = DeploymentTermsV1::admit_contextual_wire(
        unsigned_wire(
            &terms_preimage,
            "canonical_deployment_terms_sha256",
            &terms_digest,
        ),
        &terms_preimage.schema_sha256,
        |_| Ok(()),
    )
    .expect("deployment terms admit");

    let proposal_preimage = DeploymentBindingProposalPreimageV1 {
        schema_id: SchemaId::parse(DEPLOYMENT_BINDING_PROPOSAL_V1_SCHEMA).expect("proposal schema"),
        schema_sha256: schema_digest('7'),
        deployment_proposal_id: DeploymentProposalIdV1::parse(format!("proposal-{identity}"))
            .expect("proposal id"),
        deployment_terms: terms.clone(),
        deployment_terms_sha256: terms.canonical_deployment_terms_sha256().clone(),
        proposed_at: UnixTimeV1::new(10).expect("proposal time"),
        proposal_expires_at: UnixTimeV1::new(40).expect("proposal expiry"),
        compiler_principal_id: principal.clone(),
        compiler_key_id: root_key.clone(),
        compiler_key_epoch: PrincipalKeyEpochV1::new(1).expect("principal key epoch"),
    };
    let proposal_digest = DeploymentBindingProposalDigest::hash_canonical(&proposal_preimage)
        .expect("proposal digest");
    let proposal = DeploymentBindingProposalV1::admit_contextual_wire(
        principal_trust_bootstrap::signed_wire(
            &proposal_preimage,
            "canonical_proposal_sha256",
            &proposal_digest,
        ),
        &proposal_preimage.schema_sha256,
        &terms_preimage.schema_sha256,
        &principal_key(&principal, &root_key),
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("proposal admits");
    let proposal_ref = record_ref(&format!("proposal-{identity}"));
    let owner_key = ResidentKeyIdV1::parse(format!("owner-key-{identity}")).expect("owner key");
    let owner_principal = ResidentPrincipalIdV1::parse(format!("owner-principal-{identity}"))
        .expect("owner principal");
    let authority_ref = record_ref(&format!("acceptance-authority-{identity}"));
    let authority_sha256 = artifact('5');
    let evidence_ref = record_ref(&format!("acceptance-evidence-{identity}"));
    let evidence_sha256 = artifact('6');
    let acceptance_preimage = DeploymentAcceptancePreimageV1 {
        schema_id: SchemaId::parse(DEPLOYMENT_ACCEPTANCE_V1_SCHEMA).expect("acceptance schema"),
        schema_sha256: schema_digest('8'),
        deployment_acceptance_id: DeploymentAcceptanceIdV1::parse(format!("acceptance-{identity}"))
            .expect("acceptance id"),
        deployment_proposal_ref: proposal_ref.clone(),
        deployment_proposal_sha256: proposal.canonical_proposal_sha256().clone(),
        deployment_terms_sha256: proposal.deployment_terms_sha256().clone(),
        acceptance_authority_ref: authority_ref.clone(),
        acceptance_authority_sha256: authority_sha256.clone(),
        acceptance_evidence_ref: evidence_ref.clone(),
        acceptance_evidence_sha256: evidence_sha256.clone(),
        decision: DeploymentAcceptanceDecisionV1::Accepted,
        accepted_at: UnixTimeV1::new(20).expect("acceptance time"),
        binding_valid_until: UnixTimeV1::new(200).expect("binding validity"),
        permitted_binding_issuer_principal_id: principal.clone(),
        issuer_principal_id: owner_principal.clone(),
        issuer_key_id: owner_key.clone(),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("owner key epoch"),
    };
    let acceptance_digest = DeploymentAcceptanceDigest::hash_canonical(&acceptance_preimage)
        .expect("acceptance digest");
    let acceptance = DeploymentAcceptanceV1::admit_contextual_wire(
        principal_trust_bootstrap::signed_wire(
            &acceptance_preimage,
            "canonical_acceptance_sha256",
            &acceptance_digest,
        ),
        &DeploymentAcceptanceAdmissionContextV1 {
            expected_schema_sha256: &acceptance_preimage.schema_sha256,
            deployment_proposal_ref: &proposal_ref,
            deployment_proposal: &proposal,
            acceptance_authority_ref: &authority_ref,
            acceptance_authority_sha256: &authority_sha256,
            acceptance_evidence_ref: &evidence_ref,
            acceptance_evidence_sha256: &evidence_sha256,
            issuer_principal_id: &owner_principal,
            issuer_key_id: &owner_key,
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("owner key epoch"),
        },
        |_, _, _, _| Ok(()),
    )
    .expect("acceptance admits");
    let acceptance_ref = record_ref(&format!("acceptance-{identity}"));
    let binding_preimage = DeploymentBindingPreimageV1 {
        schema_id: SchemaId::parse(DEPLOYMENT_BINDING_V1_SCHEMA).expect("binding schema"),
        schema_sha256: schema_digest('9'),
        deployment_binding_id: DeploymentBindingIdV1::parse(format!("binding-{identity}"))
            .expect("binding id"),
        deployment_proposal_ref: proposal_ref.clone(),
        deployment_proposal_sha256: proposal.canonical_proposal_sha256().clone(),
        deployment_acceptance_ref: acceptance_ref.clone(),
        deployment_acceptance_sha256: acceptance.canonical_acceptance_sha256().clone(),
        deployment_terms: terms,
        deployment_terms_sha256: terms_digest,
        issued_at: UnixTimeV1::new(validity.0).expect("binding time"),
        expires_at: UnixTimeV1::new(validity.1).expect("binding expiry"),
        issuer_principal_id: principal.clone(),
        issuer_key_id: root_key.clone(),
        issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("principal key epoch"),
    };
    let binding_digest =
        DeploymentBindingDigest::hash_canonical(&binding_preimage).expect("binding digest");
    let binding = DeploymentBindingV1::admit_contextual_wire(
        principal_trust_bootstrap::signed_wire(
            &binding_preimage,
            "canonical_binding_sha256",
            &binding_digest,
        ),
        &DeploymentBindingAdmissionContextV1 {
            expected_schema_sha256: &binding_preimage.schema_sha256,
            expected_terms_schema_sha256: &terms_preimage.schema_sha256,
            deployment_proposal_ref: &proposal_ref,
            deployment_proposal: &proposal,
            deployment_acceptance_ref: &acceptance_ref,
            deployment_acceptance: &acceptance,
            signer: principal_key(&principal, &root_key),
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("deployment binding admits");
    (binding, acceptance)
}

pub(crate) fn runtime_authority(
    binding_ref: &ResidentRecordRefV1,
    binding: &DeploymentBindingV1,
    validity: (u64, u64),
) -> DeploymentRuntimeAuthorityV1 {
    runtime_authority_in_store(
        binding_ref,
        binding,
        validity,
        &[SESSION_ADMISSION_LEASE_V1_SCHEMA],
        "deployment-store-1",
        "deployment-incarnation-1",
    )
}

pub(crate) fn runtime_authority_in_store(
    binding_ref: &ResidentRecordRefV1,
    binding: &DeploymentBindingV1,
    validity: (u64, u64),
    schemas: &[&str],
    store: &str,
    incarnation: &str,
) -> DeploymentRuntimeAuthorityV1 {
    runtime_authority_in_store_named(
        binding_ref,
        binding,
        validity,
        schemas,
        store,
        incarnation,
        "1",
    )
}

/// Build one admitted runtime authority with a distinct lineage identity.
#[allow(clippy::too_many_arguments)]
pub(crate) fn runtime_authority_in_store_named(
    binding_ref: &ResidentRecordRefV1,
    binding: &DeploymentBindingV1,
    validity: (u64, u64),
    schemas: &[&str],
    store: &str,
    incarnation: &str,
    identity: &str,
) -> DeploymentRuntimeAuthorityV1 {
    let principal = binding.deployment_terms().principal_scope().clone();
    let root_key = ResidentKeyIdV1::parse("root-key-1").expect("root key");
    let control_store = ControlStoreIdV1::parse(store).expect("control store");
    let control_incarnation =
        ControlIncarnationIdV1::parse(incarnation).expect("control incarnation");
    let preimage = DeploymentRuntimeAuthorityPreimageV1 {
        schema_id: SchemaId::parse(DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA)
            .expect("runtime authority schema"),
        schema_sha256: schema_digest('a'),
        runtime_authority_id: DeploymentRuntimeAuthorityIdV1::parse(format!(
            "runtime-authority-{identity}"
        ))
        .expect("runtime authority id"),
        principal_id: principal.clone(),
        deployment_binding_ref: binding_ref.clone(),
        deployment_binding_sha256: binding.canonical_binding_sha256().clone(),
        control_store_id: control_store.clone(),
        control_incarnation_id: control_incarnation.clone(),
        subject_key_id: ResidentKeyIdV1::parse(format!("runtime-key-{identity}"))
            .expect("runtime key"),
        subject_key_epoch: SigningKeyEpochV1::new(1).expect("runtime key epoch"),
        subject_public_key: Ed25519PublicKeyV1::parse("M".repeat(43)).expect("runtime public key"),
        permitted_record_schema_ids: schemas
            .iter()
            .map(|schema| SchemaId::parse(*schema).expect("selected record schema"))
            .collect(),
        authority_epoch: DeploymentRuntimeAuthorityEpochV1::new(1)
            .expect("runtime authority epoch"),
        previous_authority_ref: None,
        previous_authority_sha256: None,
        state: PrincipalAuthorityStateV1::Active,
        issued_at: UnixTimeV1::new(validity.0).expect("runtime issue time"),
        expires_at: UnixTimeV1::new(validity.1).expect("runtime expiry"),
        issuer_principal_id: principal.clone(),
        issuer_key_id: root_key.clone(),
        issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("principal key epoch"),
    };
    let digest = DeploymentRuntimeAuthorityDigest::hash_canonical(&preimage)
        .expect("runtime authority digest");
    DeploymentRuntimeAuthorityV1::admit_initial_wire(
        principal_trust_bootstrap::signed_wire(
            &preimage,
            "canonical_deployment_runtime_authority_sha256",
            &digest,
        ),
        &DeploymentRuntimeAuthorityAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            deployment_binding_ref: binding_ref,
            deployment_binding: binding,
            control_store_id: &control_store,
            control_incarnation_id: &control_incarnation,
            signer: principal_key(&principal, &root_key),
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("runtime authority admits")
}

pub(crate) fn signing_authority(
    binding_ref: &ResidentRecordRefV1,
    binding: &DeploymentBindingV1,
    runtime_ref: &ResidentRecordRefV1,
    runtime: &DeploymentRuntimeAuthorityV1,
    trust_ref: &ResidentRecordRefV1,
    trust: &PrincipalTrustCheckpointV1,
) -> SigningAuthorityBindingV1 {
    SigningAuthorityBindingV1::integrated_runtime(
        binding.deployment_terms().principal_scope().clone(),
        runtime.subject_key_id().clone(),
        *runtime.subject_key_epoch(),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: runtime_ref.clone(),
            deployment_runtime_authority_sha256: ArtifactDigest::parse(
                runtime
                    .canonical_deployment_runtime_authority_sha256()
                    .as_str(),
            )
            .expect("runtime authority artifact digest"),
            deployment_binding_ref: binding_ref.clone(),
            deployment_binding_sha256: ArtifactDigest::parse(
                binding.canonical_binding_sha256().as_str(),
            )
            .expect("deployment binding artifact digest"),
            checked_principal_trust_checkpoint_ref: trust_ref.clone(),
            checked_principal_trust_checkpoint_sha256: ArtifactDigest::parse(
                trust.canonical_trust_checkpoint_sha256().as_str(),
            )
            .expect("trust checkpoint artifact digest"),
        },
    )
    .expect("integrated runtime signing authority")
}

pub(crate) struct Fixture {
    pub(crate) profile: EffectiveProjectProfileV1,
    pub(crate) generation: CompiledGenerationV1,
    pub(crate) fence: ActivationFenceV1,
    pub(crate) active: ActiveGenerationHeadV1,
    pub(crate) trust_ref: ResidentRecordRefV1,
    pub(crate) trust: PrincipalTrustCheckpointV1,
    pub(crate) binding_ref: ResidentRecordRefV1,
    pub(crate) binding: DeploymentBindingV1,
    pub(crate) runtime_ref: ResidentRecordRefV1,
    pub(crate) runtime: DeploymentRuntimeAuthorityV1,
    pub(crate) signing_authority: SigningAuthorityBindingV1,
    pub(crate) subject: CapabilitySessionSubjectV1,
    pub(crate) grant_ref: ResidentRecordRefV1,
    pub(crate) grant: CapabilitySessionGrantV1,
    pub(crate) checkpoint_ref: ResidentRecordRefV1,
    pub(crate) checkpoint: CapabilityRevocationCheckpointV1,
    pub(crate) origin_node: ResidentNodeIdV1,
    pub(crate) destination_principal: ResidentPrincipalIdV1,
    pub(crate) route: SessionRouteConstraintV1,
    pub(crate) contract_set: SessionContractSetConstraintV1,
    pub(crate) session_schema_sha256: SchemaDigest,
}

enum FixtureAuthoritySource<'a> {
    Generated,
    Existing {
        trust_ref: &'a ResidentRecordRefV1,
        trust: &'a PrincipalTrustCheckpointV1,
        binding_ref: &'a ResidentRecordRefV1,
        binding: &'a DeploymentBindingV1,
        runtime_ref: &'a ResidentRecordRefV1,
        runtime: &'a DeploymentRuntimeAuthorityV1,
    },
}

impl Fixture {
    pub(crate) fn with_subject_contract_set(
        binding_validity: (u64, u64),
        runtime_validity: (u64, u64),
        grant_validity: (u64, u64),
        subject_key: &str,
        public_key_character: &str,
        change_routes: impl FnOnce(&mut Vec<SessionRouteConstraintV1>),
        contract_set: SessionContractSetConstraintV1,
    ) -> (
        Self,
        PrincipalTrustCheckpointPreimageV1,
        DeploymentAcceptanceV1,
    ) {
        let (fixture, generated) = Self::with_subject_contract_set_from(
            binding_validity,
            runtime_validity,
            grant_validity,
            subject_key,
            public_key_character,
            change_routes,
            contract_set,
            FixtureAuthoritySource::Generated,
        );
        let (trust_preimage, acceptance) =
            generated.expect("generated fixture retains its construction evidence");
        (fixture, trust_preimage, acceptance)
    }

    #[allow(clippy::too_many_arguments)]
    fn with_subject_contract_set_from(
        binding_validity: (u64, u64),
        runtime_validity: (u64, u64),
        grant_validity: (u64, u64),
        subject_key: &str,
        public_key_character: &str,
        change_routes: impl FnOnce(&mut Vec<SessionRouteConstraintV1>),
        contract_set: SessionContractSetConstraintV1,
        authority_source: FixtureAuthoritySource<'_>,
    ) -> (
        Self,
        Option<(PrincipalTrustCheckpointPreimageV1, DeploymentAcceptanceV1)>,
    ) {
        let profile = effective_profile();
        let generation = compiled_generation(&profile);
        let (fence, active) = active_generation(&generation);
        let (trust_ref, trust, binding_ref, binding, runtime_ref, runtime, generated) =
            match authority_source {
                FixtureAuthoritySource::Generated => {
                    let (bootstrap, _) = principal_trust_bootstrap::admitted_bootstrap_fixture();
                    let atom = bootstrap.admitted.atom_view();
                    let (trust_ref, trust) = atom.trust_checkpoint_entry();
                    let binding_ref = record_ref("deployment-binding-1");
                    let (binding, acceptance) = deployment_binding(binding_validity);
                    let runtime_ref = record_ref("runtime-authority-1");
                    let runtime = runtime_authority(&binding_ref, &binding, runtime_validity);
                    (
                        trust_ref.clone(),
                        trust.clone(),
                        binding_ref,
                        binding,
                        runtime_ref,
                        runtime,
                        Some((bootstrap.trust_preimage.clone(), acceptance)),
                    )
                }
                FixtureAuthoritySource::Existing {
                    trust_ref,
                    trust,
                    binding_ref,
                    binding,
                    runtime_ref,
                    runtime,
                } => (
                    trust_ref.clone(),
                    trust.clone(),
                    binding_ref.clone(),
                    binding.clone(),
                    runtime_ref.clone(),
                    runtime.clone(),
                    None,
                ),
            };
        let authority = signing_authority(
            &binding_ref,
            &binding,
            &runtime_ref,
            &runtime,
            &trust_ref,
            &trust,
        );
        let subject = CapabilitySessionSubjectV1 {
            identity: RequestCallerIdV1::parse("caller-1").expect("caller id"),
            role: RequestCallerRoleV1::parse("operator").expect("caller role"),
            surface: RequestCallerSurfaceV1::parse("cli").expect("caller surface"),
            subject_key_id: ResidentKeyIdV1::parse(subject_key).expect("caller key"),
            subject_key_epoch: SigningKeyEpochV1::new(1).expect("caller key epoch"),
            subject_public_key: Ed25519PublicKeyV1::parse(public_key_character.repeat(43))
                .expect("caller public key"),
        };
        let origin_node = ResidentNodeIdV1::parse("node-1").expect("origin node");
        let destination_principal =
            ResidentPrincipalIdV1::parse("principal-1").expect("destination principal");
        let route = SessionRouteConstraintV1 {
            route_binding_schema_id: SchemaId::parse(LOCAL_ROUTE_BINDING_V1_SCHEMA)
                .expect("route schema"),
            route_binding_ref: record_ref("route-binding-1"),
            route_binding_sha256: artifact('b'),
            route_id: ResidentRouteIdV1::parse("route-1").expect("route id"),
            route_generation: RouteGenerationV1::new(1).expect("route generation"),
        };
        let mut grant_routes = vec![route.clone()];
        change_routes(&mut grant_routes);
        let grant_preimage = CapabilitySessionGrantPreimageV1 {
            schema_id: SchemaId::parse(CAPABILITY_SESSION_GRANT_V1_SCHEMA).expect("grant schema"),
            schema_sha256: schema_digest('b'),
            session_grant_id: SessionGrantIdV1::parse("grant-1").expect("grant id"),
            subject: subject.clone(),
            deployment_binding_ref: binding_ref.clone(),
            deployment_binding_sha256: binding.canonical_binding_sha256().clone(),
            checked_principal_trust_checkpoint_ref: trust_ref.clone(),
            checked_principal_trust_checkpoint_sha256: trust
                .canonical_trust_checkpoint_sha256()
                .clone(),
            checked_deployment_acceptance_revocation_epoch:
                DeploymentAcceptanceRevocationEpochV1::new(1).expect("deployment revocation epoch"),
            session_floor_epoch: *trust.session_floor_epoch(),
            allowed_capability_ids: vec![
                CapabilityIdV1::parse("read").expect("capability"),
                CapabilityIdV1::parse("write").expect("capability"),
            ],
            allowed_route_constraints: CapabilitySessionRouteConstraintsV1 {
                origin_principal_id: destination_principal.clone(),
                origin_node_id: origin_node.clone(),
                destination_principal_ids: vec![destination_principal.clone()],
                destination_node_ids: Vec::new(),
                routes: grant_routes,
                transport_classes: vec![LocalTransportClassV1::InProcess],
                contract_sets: vec![contract_set.clone()],
            },
            scope_ceiling: CapabilitySessionScopeCeilingV1 {
                principal_scopes: vec![destination_principal.clone()],
                controller_scopes: vec![
                    ResidentControllerIdV1::parse("controller-1").expect("controller")
                ],
                relationship_scopes: Vec::new(),
                environments: vec![
                    DeploymentEnvironmentV1::parse("production").expect("environment")
                ],
                data_classification_ceiling: DataClassificationV1::parse("confidential")
                    .expect("classification"),
                purposes: vec![
                    PermittedPurposeV1::parse("operations").expect("purpose"),
                    PermittedPurposeV1::parse("review").expect("purpose"),
                ],
                authority_evidence: vec![AuthorityEvidenceV1::new(
                    record_ref("scope-authority-1"),
                    artifact('d'),
                )],
            },
            issued_at: UnixTimeV1::new(grant_validity.0).expect("grant issue time"),
            expires_at: UnixTimeV1::new(grant_validity.1).expect("grant expiry"),
            nonce: CapabilitySessionNonceV1::parse("grant-nonce-1").expect("grant nonce"),
            non_transferable: true,
            issuer_principal_id: destination_principal.clone(),
            issuer_key_id: runtime.subject_key_id().clone(),
            issuer_key_epoch: *runtime.subject_key_epoch(),
        };
        let grant_digest =
            CapabilitySessionGrantDigest::hash_canonical(&grant_preimage).expect("grant digest");
        let grant = CapabilitySessionGrantV1::admit_contextual_wire(
            principal_trust_bootstrap::signed_wire(
                &grant_preimage,
                "canonical_grant_sha256",
                &grant_digest,
            ),
            &SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &grant_preimage.schema_sha256,
                signing_authority: &authority,
            },
            |_| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .expect("capability grant admits");
        let checkpoint_preimage = CapabilityRevocationCheckpointPreimageV1 {
            schema_id: SchemaId::parse(CAPABILITY_REVOCATION_CHECKPOINT_V1_SCHEMA)
                .expect("checkpoint schema"),
            schema_sha256: schema_digest('c'),
            checkpoint_id: CapabilityRevocationCheckpointIdV1::parse("capability-checkpoint-1")
                .expect("checkpoint id"),
            checkpoint_epoch: CapabilityRevocationCheckpointEpochV1::new(1)
                .expect("checkpoint epoch"),
            previous_checkpoint_ref: None,
            previous_checkpoint_sha256: None,
            authority_principal_id: destination_principal.clone(),
            authority_heads_root_sha256: ArtifactDigest::hash_bytes(b"[]"),
            covered_grant_count: CoveredGrantCountV1::new(0).expect("covered grant count"),
            generated_at: UnixTimeV1::new(50).expect("checkpoint time"),
            issuer_key_id: runtime.subject_key_id().clone(),
            issuer_key_epoch: *runtime.subject_key_epoch(),
        };
        let checkpoint_digest =
            CapabilityRevocationCheckpointDigest::hash_canonical(&checkpoint_preimage)
                .expect("capability checkpoint digest");
        let checkpoint = CapabilityRevocationCheckpointV1::admit_initial_wire(
            principal_trust_bootstrap::signed_wire(
                &checkpoint_preimage,
                "canonical_checkpoint_sha256",
                &checkpoint_digest,
            ),
            &SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &checkpoint_preimage.schema_sha256,
                signing_authority: &authority,
            },
            |_| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .expect("capability checkpoint admits");

        (
            Self {
                profile,
                generation,
                fence,
                active,
                trust_ref,
                trust,
                binding_ref,
                binding,
                runtime_ref,
                runtime,
                signing_authority: authority,
                subject,
                grant_ref: record_ref("grant-1"),
                grant,
                checkpoint_ref: record_ref("capability-checkpoint-1"),
                checkpoint,
                origin_node,
                destination_principal,
                route,
                contract_set,
                session_schema_sha256: schema_digest('d'),
            },
            generated,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn with_existing_authorities(
        binding_validity: (u64, u64),
        runtime_validity: (u64, u64),
        grant_validity: (u64, u64),
        subject_key: &str,
        public_key_character: &str,
        change_routes: impl FnOnce(&mut Vec<SessionRouteConstraintV1>),
        contract_set: SessionContractSetConstraintV1,
        trust_ref: &ResidentRecordRefV1,
        trust: &PrincipalTrustCheckpointV1,
        binding_ref: &ResidentRecordRefV1,
        binding: &DeploymentBindingV1,
        runtime_ref: &ResidentRecordRefV1,
        runtime: &DeploymentRuntimeAuthorityV1,
    ) -> Self {
        Self::with_subject_contract_set_from(
            binding_validity,
            runtime_validity,
            grant_validity,
            subject_key,
            public_key_character,
            change_routes,
            contract_set,
            FixtureAuthoritySource::Existing {
                trust_ref,
                trust,
                binding_ref,
                binding,
                runtime_ref,
                runtime,
            },
        )
        .0
    }

    pub(crate) fn session_preimage(&self) -> SessionAdmissionLeasePreimageV1 {
        let runtime_authority = self
            .signing_authority
            .integrated_runtime_authority()
            .expect("integrated runtime authority");
        SessionAdmissionLeasePreimageV1 {
            schema_id: SchemaId::parse(SESSION_ADMISSION_LEASE_V1_SCHEMA).expect("session schema"),
            schema_sha256: self.session_schema_sha256.clone(),
            session_admission_id: SessionAdmissionIdV1::parse("session-1").expect("session id"),
            compiled_generation_digest: self.generation.compiled_generation_digest().clone(),
            active_generation_head_digest: self.active.active_generation_head_digest().clone(),
            activation_fence_digest: self.fence.activation_fence_digest().clone(),
            deployment_binding_ref: self.binding_ref.clone(),
            deployment_binding_sha256: ArtifactDigest::parse(
                self.binding.canonical_binding_sha256().as_str(),
            )
            .expect("deployment binding artifact digest"),
            deployment_runtime_authority_ref: runtime_authority
                .deployment_runtime_authority_ref
                .clone(),
            deployment_runtime_authority_sha256: runtime_authority
                .deployment_runtime_authority_sha256
                .clone(),
            principal_id: self.destination_principal.clone(),
            controller_id: ResidentControllerIdV1::parse("controller-1").expect("controller"),
            relationship_scope: None,
            repository_id: LogicalRepositoryIdV1::parse("repository-1").expect("repository id"),
            project_id: self.profile.project_id.clone(),
            execution_workspace_id: self.profile.execution_workspace_id.clone(),
            profile_digest: self.profile.profile_digest.clone(),
            instance_digest: self.generation.instance_digest().clone(),
            execution_path: execution_path(),
            harness_adapter_source_sha256: artifact('1'),
            harness_adapter_build_sha256: artifact('2'),
            harness_executable_sha256: artifact('3'),
            harness_configuration_sha256: artifact('4'),
            capability_ceiling: vec![CapabilityIdV1::parse("read").expect("capability")],
            read_ceiling: vec![record_ref("read-a")],
            effect_ceiling: vec![EffectClass::Read],
            budget_ceiling_units: LeaseBudgetUnitsV1::new(10).expect("session budget"),
            timeout_ceiling_millis: LeaseTimeoutMillisV1::new(1_000).expect("session timeout"),
            evidence_obligations: vec![record_ref("evidence-a")],
            evidence_destination: record_ref("evidence-destination"),
            issued_at: UnixTimeV1::new(55).expect("session issue time"),
            expires_at: UnixTimeV1::new(100).expect("session expiry"),
            revocation_epoch: PrincipalRevocationEpochV1::new(1).expect("session revocation epoch"),
            checked_revocation_ref: None,
            checked_revocation_sha256: None,
            native_session_correlation: NativeSessionCorrelationV1::Unobserved {
                reason: ResidentReasonV1::parse("native session identity unavailable")
                    .expect("native session reason"),
            },
            issuer_principal_id: self.signing_authority.principal_id().clone(),
            issuer_key_id: ResidentKeyIdV1::parse(self.signing_authority.signer_key_id())
                .expect("issuer key"),
            issuer_key_epoch: *self.signing_authority.signer_key_epoch(),
        }
    }

    pub(crate) fn session_wire_with(
        &self,
        change: impl FnOnce(&mut SessionAdmissionLeasePreimageV1),
    ) -> SessionAdmissionLeaseWireV1 {
        let mut preimage = self.session_preimage();
        change(&mut preimage);
        let digest =
            SessionAdmissionLeaseDigest::hash_canonical(&preimage).expect("session lease digest");
        principal_trust_bootstrap::signed_wire(&preimage, "session_admission_lease_digest", &digest)
    }

    pub(crate) fn view(&self) -> AdmissionView<'_> {
        AdmissionView {
            fixture: self,
            now: UnixTimeV1::new(60).expect("current time"),
            subject: &self.subject,
            grant: Some(CapabilityGrantAuthority {
                grant_ref: &self.grant_ref,
                grant: &self.grant,
                issued_trust_checkpoint_ref: &self.trust_ref,
                issued_trust_checkpoint: &self.trust,
            }),
            current_trust: Some(CurrentPrincipalTrustHead {
                checkpoint_ref: &self.trust_ref,
                checkpoint: &self.trust,
            }),
            capability_revocations: Some(CapabilityRevocationFrontier {
                checkpoint_ref: &self.checkpoint_ref,
                checkpoint: &self.checkpoint,
                grant_head: None,
            }),
            deployment_revocations: DeploymentAcceptanceRevocationFrontier {
                epoch: DeploymentAcceptanceRevocationEpochV1::new(1)
                    .expect("deployment revocation epoch"),
                binding_head: None,
            },
            runtime_authority: Some(DeploymentRuntimeAuthorityHead {
                authority_ref: &self.runtime_ref,
                authority: &self.runtime,
            }),
            session_revocations: SessionRevocationFrontier {
                epoch: PrincipalRevocationEpochV1::new(1).expect("session revocation epoch"),
                revocation_ref: None,
                revocation_sha256: None,
            },
            route_use: AmbientSessionRouteUse {
                route: &self.route,
                origin_node_id: &self.origin_node,
                transport_class: LocalTransportClassV1::InProcess,
                destination_principal_id: &self.destination_principal,
                destination_node_id: None,
                contract_set: &self.contract_set,
            },
        }
    }
}

pub(crate) struct AdmissionView<'a> {
    pub(crate) fixture: &'a Fixture,
    pub(crate) now: UnixTimeV1,
    pub(crate) subject: &'a CapabilitySessionSubjectV1,
    pub(crate) grant: Option<CapabilityGrantAuthority<'a>>,
    pub(crate) current_trust: Option<CurrentPrincipalTrustHead<'a>>,
    pub(crate) capability_revocations: Option<CapabilityRevocationFrontier<'a>>,
    pub(crate) deployment_revocations: DeploymentAcceptanceRevocationFrontier<'a>,
    pub(crate) runtime_authority: Option<DeploymentRuntimeAuthorityHead<'a>>,
    pub(crate) session_revocations: SessionRevocationFrontier<'a>,
    pub(crate) route_use: AmbientSessionRouteUse<'a>,
}

impl<'a> AdmissionView<'a> {
    pub(crate) fn context(&'a self) -> AmbientSessionAdmissionContext<'a> {
        AmbientSessionAdmissionContext {
            now: self.now,
            subject: self.subject,
            grant: self.grant,
            current_principal_trust: self.current_trust,
            capability_revocations: self.capability_revocations,
            deployment_binding_ref: &self.fixture.binding_ref,
            deployment_binding: &self.fixture.binding,
            deployment_revocations: &self.deployment_revocations,
            runtime_authority: self.runtime_authority,
            session_revocations: &self.session_revocations,
            route_use: &self.route_use,
            generation: SessionAdmissionLeaseContextV1 {
                active_head: &self.fixture.active,
                compiled_generation: &self.fixture.generation,
                effective_profile: &self.fixture.profile,
                activation_fence: &self.fixture.fence,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &self.fixture.session_schema_sha256,
                    signing_authority: &self.fixture.signing_authority,
                },
            },
        }
    }
}

#[derive(Default)]
pub(crate) struct ProvingAuthority;

impl AmbientSessionAuthorityPort for ProvingAuthority {
    fn verify_session_admission_lease_signature(
        &mut self,
        _: &[u8],
        _: &SessionAdmissionLeasePreimageV1,
        _: &SessionAdmissionLeaseDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }

    fn prove_current_principal_trust(
        &mut self,
        _: &ResidentRecordRefV1,
        _: &PrincipalTrustCheckpointV1,
        _: &ResidentRecordRefV1,
        _: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        Ok(())
    }

    fn prove_capability_revocation_frontier(
        &mut self,
        _: &ResidentRecordRefV1,
        _: &CapabilityRevocationCheckpointV1,
        _: &ResidentRecordRefV1,
        _: &CapabilitySessionGrantV1,
        _: Option<&ResidentRecordRefV1>,
        _: Option<&CapabilitySessionGrantRevocationV1>,
    ) -> Result<(), AmbientSessionAuthorityError> {
        Ok(())
    }

    fn prove_deployment_revocation_frontier(
        &mut self,
        _: &DeploymentBindingV1,
        _: DeploymentAcceptanceRevocationEpochV1,
        _: Option<&ResidentRecordRefV1>,
        _: Option<&DeploymentAcceptanceRevocationV1>,
        _: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        Ok(())
    }

    fn prove_runtime_authority_head(
        &mut self,
        _: &ResidentRecordRefV1,
        _: &DeploymentRuntimeAuthorityV1,
        _: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        Ok(())
    }

    fn prove_session_revocation_frontier(
        &mut self,
        _: &ResidentPrincipalIdV1,
        _: PrincipalRevocationEpochV1,
        _: Option<&ResidentRecordRefV1>,
        _: Option<&ArtifactDigest>,
        _: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        Ok(())
    }

    fn prove_data_classification_ceiling(
        &mut self,
        _: &DataClassificationV1,
        _: &DataClassificationV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        Ok(())
    }
}

/// Reuse actual contextual session/child admission in sibling custody laws.
/// Signature and external-authority callbacks remain explicit test doubles.
pub(crate) fn local_start_contract_set() -> ContractSetV1 {
    local_start_contract_set_named("local-start-contract-set")
}

/// Contract set with the same entries but a deliberately different identity.
#[allow(
    dead_code,
    reason = "the control-store suite reuses this fixture module without the application substitution test"
)]
pub(crate) fn substituted_local_start_contract_set() -> ContractSetV1 {
    local_start_contract_set_named("substituted-local-start-contract-set")
}

fn local_start_contract_set_named(contract_set_id: &str) -> ContractSetV1 {
    let native_execution = pale_contracts::record_schema_identity("action-execution.v1").unwrap();
    let preimage = ContractSetPreimageV1 {
        schema_id: SchemaId::parse(CONTRACT_SET_V1_SCHEMA).unwrap(),
        schema_sha256: pale_contracts::record_schema_identity(CONTRACT_SET_V1_SCHEMA_ID)
            .unwrap()
            .schema_sha256,
        contract_set_id: ContractSetIdV1::parse(contract_set_id).unwrap(),
        entries: vec![
            ContractSetEntryV1::new(
                SchemaId::parse("example/result.v1").unwrap(),
                record_ref("example/result-schema"),
                schema_digest('2'),
            ),
            ContractSetEntryV1::new(
                native_execution.schema,
                record_ref("native/action-execution-schema"),
                native_execution.schema_sha256,
            ),
        ],
        compatibility_profile_ref: record_ref("local-start-compatibility"),
        compatibility_profile_sha256: CompatibilityProfileDigest::parse(sha('3')).unwrap(),
    };
    let digest = ContractSetDigest::hash_canonical(&preimage).unwrap();
    ContractSetV1::admit_contextual_wire(
        unsigned_wire(&preimage, "canonical_contract_set_sha256", &digest),
        &preimage.schema_sha256,
        |_| Ok(()),
        |_, _, _| Ok(()),
    )
    .expect("actual contract-set identity with declared schema-resolution test models")
}

#[derive(Clone, Copy)]
pub(crate) enum LocalStartContext {
    Inherited,
    ObservedNative,
    SelfContained,
    ReadAuthorityOnly,
    EvidenceOnly,
}

/// Existing deployment authorities selected by a cold-open trust fixture.
#[derive(Clone, Copy)]
pub(crate) struct LocalStartExistingAuthorities<'a> {
    pub(crate) trust_ref: &'a ResidentRecordRefV1,
    pub(crate) trust: &'a PrincipalTrustCheckpointV1,
    pub(crate) binding_ref: &'a ResidentRecordRefV1,
    pub(crate) binding: &'a DeploymentBindingV1,
    pub(crate) runtime_ref: &'a ResidentRecordRefV1,
    pub(crate) runtime: &'a DeploymentRuntimeAuthorityV1,
}

impl LocalStartContext {
    pub(crate) const fn has_read_authority(self) -> bool {
        matches!(
            self,
            Self::Inherited | Self::ObservedNative | Self::ReadAuthorityOnly
        )
    }

    pub(crate) const fn has_evidence_obligations(self) -> bool {
        matches!(
            self,
            Self::Inherited | Self::ObservedNative | Self::EvidenceOnly
        )
    }
}

pub(crate) struct LocalStartSessionStore<'a> {
    pub(crate) store: &'a AuthorityStoreCoordinate,
    pub(crate) registered_generation_heads: bool,
    pub(crate) existing_authorities: Option<LocalStartExistingAuthorities<'a>>,
}

pub(crate) fn with_local_start_session_context_at_store<R>(
    generation: &super::resident::AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    operation: OperationLeaseRequestV1,
    context_selection: LocalStartContext,
    selection: LocalStartSessionStore<'_>,
    run: impl FnOnce(
        &AdmittedAmbientSession,
        &AmbientSessionAdmissionContext<'_>,
        &OperationLeaseV1,
        &SigningAuthorityBindingV1,
    ) -> R,
) -> R {
    let LocalStartSessionStore {
        store,
        registered_generation_heads,
        existing_authorities,
    } = selection;
    let contracts = local_start_contract_set();
    let constraint = SessionContractSetConstraintV1 {
        contract_set_ref: record_ref("contract-set-1"),
        contract_set_sha256: contracts.canonical_contract_set_sha256().clone(),
    };
    let (fixture, retain_existing_runtime) = if let Some(authorities) = existing_authorities {
        (
            Fixture::with_existing_authorities(
                (30, 180),
                (40, 150),
                (50, 220),
                authorities.runtime.subject_key_id().as_str(),
                "M",
                |_| {},
                constraint,
                authorities.trust_ref,
                authorities.trust,
                authorities.binding_ref,
                authorities.binding,
                authorities.runtime_ref,
                authorities.runtime,
            ),
            true,
        )
    } else {
        let (fixture, _, _) = Fixture::with_subject_contract_set(
            (30, 180),
            (40, 150),
            (50, 220),
            "runtime-key-1",
            "M",
            |_| {},
            constraint,
        );
        (fixture, false)
    };
    with_local_start_session_fixture(
        fixture,
        LocalStartSessionFixtureInput {
            generation,
            profile,
            operation,
            context_selection,
            store,
            registered_generation_heads,
            retain_existing_runtime,
        },
        run,
    )
}

/// Supply the source-backed local session against an already admitted trust set.
#[allow(clippy::too_many_arguments)]
pub(crate) fn with_local_start_session_context_at_store_and_authorities<R>(
    generation: &super::resident::AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    operation: OperationLeaseRequestV1,
    context_selection: LocalStartContext,
    store: &AuthorityStoreCoordinate,
    trust_ref: &ResidentRecordRefV1,
    trust: &PrincipalTrustCheckpointV1,
    binding_ref: &ResidentRecordRefV1,
    binding: &DeploymentBindingV1,
    runtime_ref: &ResidentRecordRefV1,
    runtime: &DeploymentRuntimeAuthorityV1,
    run: impl FnOnce(
        &AdmittedAmbientSession,
        &AmbientSessionAdmissionContext<'_>,
        &OperationLeaseV1,
        &SigningAuthorityBindingV1,
    ) -> R,
) -> R {
    with_local_start_session_context_at_store(
        generation,
        profile,
        operation,
        context_selection,
        LocalStartSessionStore {
            store,
            registered_generation_heads: true,
            existing_authorities: Some(LocalStartExistingAuthorities {
                trust_ref,
                trust,
                binding_ref,
                binding,
                runtime_ref,
                runtime,
            }),
        },
        run,
    )
}

struct LocalStartSessionFixtureInput<'a> {
    generation: &'a super::resident::AdmittedCompiledGeneration,
    profile: &'a EffectiveProjectProfileV1,
    operation: OperationLeaseRequestV1,
    context_selection: LocalStartContext,
    store: &'a AuthorityStoreCoordinate,
    registered_generation_heads: bool,
    retain_existing_runtime: bool,
}

fn with_local_start_session_fixture<R>(
    mut fixture: Fixture,
    input: LocalStartSessionFixtureInput<'_>,
    run: impl FnOnce(
        &AdmittedAmbientSession,
        &AmbientSessionAdmissionContext<'_>,
        &OperationLeaseV1,
        &SigningAuthorityBindingV1,
    ) -> R,
) -> R {
    let LocalStartSessionFixtureInput {
        generation,
        profile,
        operation,
        context_selection,
        store,
        registered_generation_heads,
        retain_existing_runtime,
    } = input;
    fixture.profile = profile.clone();
    fixture.generation = generation.record().clone();
    (fixture.fence, fixture.active) = if registered_generation_heads {
        registered_active_generation(generation.record())
    } else {
        active_generation(generation.record())
    };
    fixture.session_schema_sha256 =
        pale_contracts::record_schema_identity(SESSION_ADMISSION_LEASE_V1_SCHEMA_ID)
            .expect("registered session admission schema")
            .schema_sha256;
    // The generation-head representation and the selected authority store are
    // independent fixture choices. Registered heads must still materialize the
    // runtime authority in the store supplied by the operation-start caller.
    if !retain_existing_runtime {
        fixture.runtime = runtime_authority_in_store(
            &fixture.binding_ref,
            &fixture.binding,
            (40, 150),
            &[
                CAPABILITY_RESULT_V1_SCHEMA,
                COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
                OPERATION_HANDLE_V1_SCHEMA,
                OPERATION_OBSERVATION_V1_SCHEMA,
                SESSION_ADMISSION_LEASE_V1_SCHEMA,
            ],
            store.store_id().as_str(),
            store.incarnation_id().as_str(),
        );
    }
    fixture.signing_authority = signing_authority(
        &fixture.binding_ref,
        &fixture.binding,
        &fixture.runtime_ref,
        &fixture.runtime,
        &fixture.trust_ref,
        &fixture.trust,
    );
    let view = fixture.view();
    let context = view.context();
    let wire = fixture.session_wire_with(|preimage| {
        preimage.repository_id = generation.record().repository_source_frontiers()[0]
            .repository_id
            .clone();
        preimage.execution_path = profile.execution_paths[0].clone();
        preimage.effect_ceiling = operation.effects.clone();
        if matches!(context_selection, LocalStartContext::ObservedNative) {
            preimage.native_session_correlation = NativeSessionCorrelationV1::Observed {
                native_session_identity: HarnessActivityIdentity::try_from_host(
                    Some("native-session"),
                    None,
                )
                .unwrap()
                .unwrap(),
            };
        }
        if !context_selection.has_read_authority() {
            preimage.read_ceiling.clear();
        }
        if !context_selection.has_evidence_obligations() {
            preimage.evidence_obligations.clear();
        }
    });
    let session = admit_ambient_session(wire, &context, &mut ProvingAuthority)
        .expect("source-backed generation session admission");
    let lease = session
        .admit_operation(
            pale_contracts::record_schema_identity(OPERATION_LEASE_V1_SCHEMA_ID)
                .expect("registered operation lease schema")
                .schema_sha256,
            operation,
            &context,
            &mut ProvingAuthority,
        )
        .expect("current child admission");
    let signing = signing_authority(
        &fixture.binding_ref,
        &fixture.binding,
        &fixture.runtime_ref,
        &fixture.runtime,
        &fixture.trust_ref,
        &fixture.trust,
    );
    run(&session, &context, &lease, &signing)
}
