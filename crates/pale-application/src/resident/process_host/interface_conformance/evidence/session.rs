//! Current session authority for the interface conformance frontier.
//!
//! The constructors in this module admit real deployment, grant, revocation,
//! session, and child-lease records. Deterministic keys and signature callbacks
//! are test models. They establish record joins, not production authentication.

use pale_core::records::*;
use pale_ports::AuthorityStoreCoordinate;
use serde::{de::DeserializeOwned, Serialize};

use crate::resident::*;

use super::principal::{record_ref, signature, signed_wire};

fn unsigned_wire<P, D, W>(preimage: &P, digest_field: &str, digest: &D) -> W
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
    serde_json::from_value(value).expect("wire decodes")
}

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn artifact(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(sha(byte)).expect("artifact digest")
}

fn schema_digest(byte: char) -> SchemaDigest {
    SchemaDigest::parse(sha(byte)).expect("schema digest")
}

fn principal_key<'a>(
    principal: &'a ResidentPrincipalIdV1,
    key: &'a ResidentKeyIdV1,
) -> PrincipalKeyAdmissionContextV1<'a> {
    PrincipalKeyAdmissionContextV1::new(
        principal,
        key,
        PrincipalKeyEpochV1::new(1).expect("principal key epoch"),
    )
}

struct ConformanceDeploymentFixture {
    terms: DeploymentTermsV1,
    proposal_ref: ResidentRecordRefV1,
    proposal: DeploymentBindingProposalV1,
    acceptance_ref: ResidentRecordRefV1,
    acceptance: DeploymentAcceptanceV1,
    binding: DeploymentBindingV1,
}

fn deployment_binding() -> ConformanceDeploymentFixture {
    let principal = ResidentPrincipalIdV1::parse("principal-1").expect("principal");
    let root_key = ResidentKeyIdV1::parse("root-key-1").expect("root key");
    let terms_preimage = DeploymentTermsPreimageV1 {
        schema_id: SchemaId::parse(DEPLOYMENT_TERMS_V1_SCHEMA).expect("terms schema"),
        schema_sha256: schema_digest('e'),
        workspace_id: WorkspaceId::parse("workspace-1").expect("workspace"),
        workspace_commit: WorkspaceCommitV1::parse("commit-1").expect("workspace commit"),
        deployment_id: DeploymentIdV1::parse("deployment-1").expect("deployment"),
        distribution_release: DistributionReleaseV1::parse("release-1").expect("release"),
        distribution_release_sha256: DistributionReleaseDigest::parse(sha('1'))
            .expect("release digest"),
        principal_scope: principal.clone(),
        controller_scope: ResidentControllerIdV1::parse("controller-1").expect("controller"),
        relationship_scope: None,
        environment: DeploymentEnvironmentV1::parse("development").expect("environment"),
        environment_sha256: DeploymentEnvironmentDigest::parse(sha('2'))
            .expect("environment digest"),
        processing_role_ref: record_ref("processing-role-1"),
        processing_role_sha256: ProcessingRoleDigest::parse(sha('3')).expect("processing role"),
        confidentiality_policy_ref: record_ref("confidentiality-policy-1"),
        confidentiality_policy_sha256: ConfidentialityPolicyDigest::parse(sha('4'))
            .expect("confidentiality policy"),
        data_classification: DataClassificationV1::parse("internal").expect("classification"),
        permitted_purposes: vec![PermittedPurposeV1::parse("operations").expect("purpose")],
    };
    let terms_digest =
        DeploymentTermsDigest::hash_canonical(&terms_preimage).expect("terms digest");
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
        schema_sha256: schema_digest('f'),
        deployment_proposal_id: DeploymentProposalIdV1::parse("proposal-1").expect("proposal"),
        deployment_terms: terms.clone(),
        deployment_terms_sha256: terms.canonical_deployment_terms_sha256().clone(),
        proposed_at: UnixTimeV1::new(10).expect("proposal time"),
        proposal_expires_at: UnixTimeV1::new(40).expect("proposal expiry"),
        compiler_principal_id: principal.clone(),
        compiler_key_id: root_key.clone(),
        compiler_key_epoch: PrincipalKeyEpochV1::new(1).expect("root epoch"),
    };
    let proposal_digest = DeploymentBindingProposalDigest::hash_canonical(&proposal_preimage)
        .expect("proposal digest");
    let proposal = DeploymentBindingProposalV1::admit_contextual_wire(
        signed_wire(
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
    .expect("deployment proposal admits");
    let proposal_ref = record_ref("proposal-1");
    let owner_principal =
        ResidentPrincipalIdV1::parse("owner-principal-1").expect("owner principal");
    let owner_key = ResidentKeyIdV1::parse("owner-key-1").expect("owner key");
    let acceptance_authority_ref = record_ref("acceptance-authority-1");
    let acceptance_authority_sha256 = artifact('5');
    let acceptance_evidence_ref = record_ref("acceptance-evidence-1");
    let acceptance_evidence_sha256 = artifact('6');
    let acceptance_preimage = DeploymentAcceptancePreimageV1 {
        schema_id: SchemaId::parse(DEPLOYMENT_ACCEPTANCE_V1_SCHEMA).expect("acceptance schema"),
        schema_sha256: schema_digest('0'),
        deployment_acceptance_id: DeploymentAcceptanceIdV1::parse("acceptance-1")
            .expect("acceptance"),
        deployment_proposal_ref: proposal_ref.clone(),
        deployment_proposal_sha256: proposal.canonical_proposal_sha256().clone(),
        deployment_terms_sha256: proposal.deployment_terms_sha256().clone(),
        acceptance_authority_ref: acceptance_authority_ref.clone(),
        acceptance_authority_sha256: acceptance_authority_sha256.clone(),
        acceptance_evidence_ref: acceptance_evidence_ref.clone(),
        acceptance_evidence_sha256: acceptance_evidence_sha256.clone(),
        decision: DeploymentAcceptanceDecisionV1::Accepted,
        accepted_at: UnixTimeV1::new(20).expect("acceptance time"),
        binding_valid_until: UnixTimeV1::new(900).expect("binding validity"),
        permitted_binding_issuer_principal_id: principal.clone(),
        issuer_principal_id: owner_principal.clone(),
        issuer_key_id: owner_key.clone(),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("owner key epoch"),
    };
    let acceptance_digest = DeploymentAcceptanceDigest::hash_canonical(&acceptance_preimage)
        .expect("acceptance digest");
    let acceptance = DeploymentAcceptanceV1::admit_contextual_wire(
        signed_wire(
            &acceptance_preimage,
            "canonical_acceptance_sha256",
            &acceptance_digest,
        ),
        &DeploymentAcceptanceAdmissionContextV1 {
            expected_schema_sha256: &acceptance_preimage.schema_sha256,
            deployment_proposal_ref: &proposal_ref,
            deployment_proposal: &proposal,
            acceptance_authority_ref: &acceptance_authority_ref,
            acceptance_authority_sha256: &acceptance_authority_sha256,
            acceptance_evidence_ref: &acceptance_evidence_ref,
            acceptance_evidence_sha256: &acceptance_evidence_sha256,
            issuer_principal_id: &owner_principal,
            issuer_key_id: &owner_key,
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("owner key epoch"),
        },
        |_, _, _, _| Ok(()),
    )
    .expect("deployment acceptance admits");
    let acceptance_ref = record_ref("acceptance-1");
    let binding_preimage = DeploymentBindingPreimageV1 {
        schema_id: SchemaId::parse(DEPLOYMENT_BINDING_V1_SCHEMA).expect("binding schema"),
        schema_sha256: schema_digest('1'),
        deployment_binding_id: DeploymentBindingIdV1::parse("binding-1").expect("binding"),
        deployment_proposal_ref: proposal_ref.clone(),
        deployment_proposal_sha256: proposal.canonical_proposal_sha256().clone(),
        deployment_acceptance_ref: acceptance_ref.clone(),
        deployment_acceptance_sha256: acceptance.canonical_acceptance_sha256().clone(),
        deployment_terms: terms,
        deployment_terms_sha256: terms_digest,
        issued_at: UnixTimeV1::new(30).expect("binding issue time"),
        expires_at: UnixTimeV1::new(800).expect("binding expiry"),
        issuer_principal_id: principal.clone(),
        issuer_key_id: root_key.clone(),
        issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("root epoch"),
    };
    let binding_digest =
        DeploymentBindingDigest::hash_canonical(&binding_preimage).expect("binding digest");
    let binding = DeploymentBindingV1::admit_contextual_wire(
        signed_wire(
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
    ConformanceDeploymentFixture {
        terms: binding.deployment_terms().clone(),
        proposal_ref,
        proposal,
        acceptance_ref,
        acceptance,
        binding,
    }
}

fn runtime_authority(
    binding_ref: &ResidentRecordRefV1,
    binding: &DeploymentBindingV1,
    store: &AuthorityStoreCoordinate,
) -> DeploymentRuntimeAuthorityV1 {
    let principal = binding.deployment_terms().principal_scope().clone();
    let root_key = ResidentKeyIdV1::parse("root-key-1").expect("root key");
    let preimage = DeploymentRuntimeAuthorityPreimageV1 {
        schema_id: SchemaId::parse(DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA)
            .expect("runtime authority schema"),
        schema_sha256: schema_digest('2'),
        runtime_authority_id: DeploymentRuntimeAuthorityIdV1::parse("runtime-authority-1")
            .expect("runtime authority"),
        principal_id: principal.clone(),
        deployment_binding_ref: binding_ref.clone(),
        deployment_binding_sha256: binding.canonical_binding_sha256().clone(),
        control_store_id: store.store_id().clone(),
        control_incarnation_id: store.incarnation_id().clone(),
        subject_key_id: ResidentKeyIdV1::parse("runtime-key-1").expect("runtime key"),
        subject_key_epoch: SigningKeyEpochV1::new(1).expect("runtime key epoch"),
        subject_public_key: Ed25519PublicKeyV1::parse("M".repeat(43)).expect("runtime public key"),
        permitted_record_schema_ids: [
            CAPABILITY_RESULT_V1_SCHEMA,
            COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
            OPERATION_HANDLE_V1_SCHEMA,
            OPERATION_OBSERVATION_V1_SCHEMA,
            SESSION_ADMISSION_LEASE_V1_SCHEMA,
        ]
        .into_iter()
        .map(|schema| SchemaId::parse(schema).expect("runtime record schema"))
        .collect(),
        authority_epoch: DeploymentRuntimeAuthorityEpochV1::new(1)
            .expect("runtime authority epoch"),
        previous_authority_ref: None,
        previous_authority_sha256: None,
        state: PrincipalAuthorityStateV1::Active,
        issued_at: UnixTimeV1::new(35).expect("runtime issue time"),
        expires_at: UnixTimeV1::new(800).expect("runtime expiry"),
        issuer_principal_id: principal.clone(),
        issuer_key_id: root_key.clone(),
        issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("root epoch"),
    };
    let digest = DeploymentRuntimeAuthorityDigest::hash_canonical(&preimage)
        .expect("runtime authority digest");
    DeploymentRuntimeAuthorityV1::admit_initial_wire(
        signed_wire(
            &preimage,
            "canonical_deployment_runtime_authority_sha256",
            &digest,
        ),
        &DeploymentRuntimeAuthorityAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            deployment_binding_ref: binding_ref,
            deployment_binding: binding,
            control_store_id: store.store_id(),
            control_incarnation_id: store.incarnation_id(),
            signer: principal_key(&principal, &root_key),
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("runtime authority admits")
}

fn signing_authority(
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
            .expect("runtime authority digest"),
            deployment_binding_ref: binding_ref.clone(),
            deployment_binding_sha256: ArtifactDigest::parse(
                binding.canonical_binding_sha256().as_str(),
            )
            .expect("deployment binding digest"),
            checked_principal_trust_checkpoint_ref: trust_ref.clone(),
            checked_principal_trust_checkpoint_sha256: ArtifactDigest::parse(
                trust.canonical_trust_checkpoint_sha256().as_str(),
            )
            .expect("trust checkpoint digest"),
        },
    )
    .expect("integrated runtime signing authority")
}

/// Records that establish one current, admitted local session.
pub(in super::super) struct ConformanceSessionFixture {
    deployment: ConformanceDeploymentFixture,
    pub(in super::super) binding_ref: ResidentRecordRefV1,
    pub(in super::super) binding: DeploymentBindingV1,
    pub(in super::super) runtime_ref: ResidentRecordRefV1,
    pub(in super::super) runtime: DeploymentRuntimeAuthorityV1,
    pub(in super::super) signing: SigningAuthorityBindingV1,
    pub(in super::super) subject: CapabilitySessionSubjectV1,
    pub(in super::super) grant_ref: ResidentRecordRefV1,
    pub(in super::super) grant: CapabilitySessionGrantV1,
    pub(in super::super) capability_checkpoint_ref: ResidentRecordRefV1,
    pub(in super::super) capability_checkpoint: CapabilityRevocationCheckpointV1,
    pub(in super::super) route: SessionRouteConstraintV1,
    pub(in super::super) origin_node: ResidentNodeIdV1,
    pub(in super::super) principal: ResidentPrincipalIdV1,
    pub(in super::super) contract_set: SessionContractSetConstraintV1,
    pub(in super::super) session_schema_sha256: SchemaDigest,
}

impl ConformanceSessionFixture {
    pub(in super::super) fn construct(
        trust_ref: &ResidentRecordRefV1,
        trust: &PrincipalTrustCheckpointV1,
        store: &AuthorityStoreCoordinate,
    ) -> Self {
        let binding_ref = record_ref("deployment-binding-1");
        let deployment = deployment_binding();
        let binding = deployment.binding.clone();
        let runtime_ref = record_ref("runtime-authority-1");
        let runtime = runtime_authority(&binding_ref, &binding, store);
        let signing = signing_authority(
            &binding_ref,
            &binding,
            &runtime_ref,
            &runtime,
            trust_ref,
            trust,
        );
        let principal = binding.deployment_terms().principal_scope().clone();
        let subject = CapabilitySessionSubjectV1 {
            identity: RequestCallerIdV1::parse("caller-1").expect("caller"),
            role: RequestCallerRoleV1::parse("operator").expect("caller role"),
            surface: RequestCallerSurfaceV1::parse("mcp").expect("caller surface"),
            subject_key_id: runtime.subject_key_id().clone(),
            subject_key_epoch: *runtime.subject_key_epoch(),
            subject_public_key: runtime.subject_public_key().clone(),
        };
        let origin_node = ResidentNodeIdV1::parse("node-1").expect("node");
        let route = SessionRouteConstraintV1 {
            route_binding_schema_id: SchemaId::parse(LOCAL_ROUTE_BINDING_V1_SCHEMA)
                .expect("route schema"),
            route_binding_ref: record_ref("route-binding-1"),
            route_binding_sha256: artifact('3'),
            route_id: ResidentRouteIdV1::parse("route-1").expect("route"),
            route_generation: RouteGenerationV1::new(1).expect("route generation"),
        };
        let contract_set = SessionContractSetConstraintV1 {
            contract_set_ref: record_ref("contract-set-1"),
            contract_set_sha256: ContractSetDigest::parse(sha('4')).expect("contract set digest"),
        };
        let grant_preimage = CapabilitySessionGrantPreimageV1 {
            schema_id: SchemaId::parse(CAPABILITY_SESSION_GRANT_V1_SCHEMA).expect("grant schema"),
            schema_sha256: schema_digest('5'),
            session_grant_id: SessionGrantIdV1::parse("grant-1").expect("grant"),
            subject: subject.clone(),
            deployment_binding_ref: binding_ref.clone(),
            deployment_binding_sha256: binding.canonical_binding_sha256().clone(),
            checked_principal_trust_checkpoint_ref: trust_ref.clone(),
            checked_principal_trust_checkpoint_sha256: trust
                .canonical_trust_checkpoint_sha256()
                .clone(),
            checked_deployment_acceptance_revocation_epoch:
                DeploymentAcceptanceRevocationEpochV1::new(1).expect("deployment epoch"),
            session_floor_epoch: *trust.session_floor_epoch(),
            allowed_capability_ids: vec![CapabilityIdV1::parse("read").expect("capability")],
            allowed_route_constraints: CapabilitySessionRouteConstraintsV1 {
                origin_principal_id: principal.clone(),
                origin_node_id: origin_node.clone(),
                destination_principal_ids: vec![principal.clone()],
                destination_node_ids: Vec::new(),
                routes: vec![route.clone()],
                transport_classes: vec![LocalTransportClassV1::InProcess],
                contract_sets: vec![contract_set.clone()],
            },
            scope_ceiling: CapabilitySessionScopeCeilingV1 {
                principal_scopes: vec![principal.clone()],
                controller_scopes: vec![
                    ResidentControllerIdV1::parse("controller-1").expect("controller")
                ],
                relationship_scopes: Vec::new(),
                environments: vec![
                    DeploymentEnvironmentV1::parse("development").expect("environment")
                ],
                data_classification_ceiling: DataClassificationV1::parse("internal")
                    .expect("classification"),
                purposes: vec![PermittedPurposeV1::parse("operations").expect("purpose")],
                authority_evidence: vec![AuthorityEvidenceV1::new(
                    record_ref("scope-authority-1"),
                    artifact('6'),
                )],
            },
            issued_at: UnixTimeV1::new(40).expect("grant issue time"),
            expires_at: UnixTimeV1::new(700).expect("grant expiry"),
            nonce: CapabilitySessionNonceV1::parse("grant-nonce-1").expect("grant nonce"),
            non_transferable: true,
            issuer_principal_id: principal.clone(),
            issuer_key_id: runtime.subject_key_id().clone(),
            issuer_key_epoch: *runtime.subject_key_epoch(),
        };
        let grant_digest =
            CapabilitySessionGrantDigest::hash_canonical(&grant_preimage).expect("grant digest");
        let grant = CapabilitySessionGrantV1::admit_contextual_wire(
            signed_wire(&grant_preimage, "canonical_grant_sha256", &grant_digest),
            &SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &grant_preimage.schema_sha256,
                signing_authority: &signing,
            },
            |_| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .expect("capability grant admits");
        let checkpoint_preimage = CapabilityRevocationCheckpointPreimageV1 {
            schema_id: SchemaId::parse(CAPABILITY_REVOCATION_CHECKPOINT_V1_SCHEMA)
                .expect("capability checkpoint schema"),
            schema_sha256: schema_digest('6'),
            checkpoint_id: CapabilityRevocationCheckpointIdV1::parse("capability-checkpoint-1")
                .expect("capability checkpoint"),
            checkpoint_epoch: CapabilityRevocationCheckpointEpochV1::new(1)
                .expect("checkpoint epoch"),
            previous_checkpoint_ref: None,
            previous_checkpoint_sha256: None,
            authority_principal_id: principal.clone(),
            authority_heads_root_sha256: ArtifactDigest::hash_bytes(b"[]"),
            covered_grant_count: CoveredGrantCountV1::new(0).expect("covered grant count"),
            generated_at: UnixTimeV1::new(45).expect("checkpoint time"),
            issuer_key_id: runtime.subject_key_id().clone(),
            issuer_key_epoch: *runtime.subject_key_epoch(),
        };
        let checkpoint_digest =
            CapabilityRevocationCheckpointDigest::hash_canonical(&checkpoint_preimage)
                .expect("capability checkpoint digest");
        let capability_checkpoint = CapabilityRevocationCheckpointV1::admit_initial_wire(
            signed_wire(
                &checkpoint_preimage,
                "canonical_checkpoint_sha256",
                &checkpoint_digest,
            ),
            &SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &checkpoint_preimage.schema_sha256,
                signing_authority: &signing,
            },
            |_| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .expect("capability checkpoint admits");
        Self {
            deployment,
            binding_ref,
            binding,
            runtime_ref,
            runtime,
            signing,
            subject,
            grant_ref: record_ref("grant-1"),
            grant,
            capability_checkpoint_ref: record_ref("capability-checkpoint-1"),
            capability_checkpoint,
            route,
            origin_node,
            principal,
            contract_set,
            session_schema_sha256: pale_contracts::record_schema_identity(
                SESSION_ADMISSION_LEASE_V1_SCHEMA_ID,
            )
            .expect("session schema identity")
            .schema_sha256,
        }
    }
    /// Admit one ambient session and one root-operation lease, then lend them.
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn with_admitted_session<Output>(
        &self,
        generation: &CompiledGenerationV1,
        profile: &EffectiveProjectProfileV1,
        active: &ActiveGenerationHeadV1,
        fence: &ActivationFenceV1,
        trust_ref: &ResidentRecordRefV1,
        trust: &PrincipalTrustCheckpointV1,
        use_session: impl FnOnce(
            &AdmittedAmbientSession,
            &AmbientSessionAdmissionContext<'_>,
            &OperationLeaseV1,
        ) -> Output,
    ) -> Output {
        let deployment_revocations = DeploymentAcceptanceRevocationFrontier {
            epoch: DeploymentAcceptanceRevocationEpochV1::new(1)
                .expect("deployment revocation epoch"),
            binding_head: None,
        };
        let session_revocations = SessionRevocationFrontier {
            epoch: PrincipalRevocationEpochV1::new(1).expect("session revocation epoch"),
            revocation_ref: None,
            revocation_sha256: None,
        };
        let route_use = AmbientSessionRouteUse {
            route: &self.route,
            origin_node_id: &self.origin_node,
            transport_class: LocalTransportClassV1::InProcess,
            destination_principal_id: &self.principal,
            destination_node_id: None,
            contract_set: &self.contract_set,
        };
        let current = AmbientSessionAdmissionContext {
            now: UnixTimeV1::new(60).expect("session admission time"),
            subject: &self.subject,
            grant: Some(CapabilityGrantAuthority {
                grant_ref: &self.grant_ref,
                grant: &self.grant,
                issued_trust_checkpoint_ref: trust_ref,
                issued_trust_checkpoint: trust,
            }),
            current_principal_trust: Some(CurrentPrincipalTrustHead {
                checkpoint_ref: trust_ref,
                checkpoint: trust,
            }),
            capability_revocations: Some(CapabilityRevocationFrontier {
                checkpoint_ref: &self.capability_checkpoint_ref,
                checkpoint: &self.capability_checkpoint,
                grant_head: None,
            }),
            deployment_binding_ref: &self.binding_ref,
            deployment_binding: &self.binding,
            deployment_revocations: &deployment_revocations,
            runtime_authority: Some(DeploymentRuntimeAuthorityHead {
                authority_ref: &self.runtime_ref,
                authority: &self.runtime,
            }),
            session_revocations: &session_revocations,
            route_use: &route_use,
            generation: SessionAdmissionLeaseContextV1 {
                active_head: active,
                compiled_generation: generation,
                effective_profile: profile,
                activation_fence: fence,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &self.session_schema_sha256,
                    signing_authority: &self.signing,
                },
            },
        };
        let session_preimage = SessionAdmissionLeasePreimageV1 {
            schema_id: SchemaId::parse(SESSION_ADMISSION_LEASE_V1_SCHEMA).expect("session schema"),
            schema_sha256: self.session_schema_sha256.clone(),
            session_admission_id: SessionAdmissionIdV1::parse("session-1").expect("session"),
            compiled_generation_digest: generation.compiled_generation_digest().clone(),
            active_generation_head_digest: active.active_generation_head_digest().clone(),
            activation_fence_digest: fence.activation_fence_digest().clone(),
            deployment_binding_ref: self.binding_ref.clone(),
            deployment_binding_sha256: ArtifactDigest::parse(
                self.binding.canonical_binding_sha256().as_str(),
            )
            .expect("deployment binding digest"),
            deployment_runtime_authority_ref: self.runtime_ref.clone(),
            deployment_runtime_authority_sha256: ArtifactDigest::parse(
                self.runtime
                    .canonical_deployment_runtime_authority_sha256()
                    .as_str(),
            )
            .expect("runtime authority digest"),
            principal_id: self.principal.clone(),
            controller_id: ResidentControllerIdV1::parse("controller-1").expect("controller"),
            relationship_scope: None,
            repository_id: generation.repository_source_frontiers()[0]
                .repository_id
                .clone(),
            project_id: profile.project_id.clone(),
            execution_workspace_id: profile.execution_workspace_id.clone(),
            profile_digest: profile.profile_digest.clone(),
            instance_digest: generation.instance_digest().clone(),
            execution_path: profile.execution_paths[0].clone(),
            harness_adapter_source_sha256: artifact('7'),
            harness_adapter_build_sha256: artifact('8'),
            harness_executable_sha256: artifact('9'),
            harness_configuration_sha256: artifact('a'),
            capability_ceiling: vec![CapabilityIdV1::parse("read").expect("capability")],
            read_ceiling: Vec::new(),
            effect_ceiling: vec![EffectClass::Compute],
            budget_ceiling_units: LeaseBudgetUnitsV1::new(10).expect("session budget"),
            timeout_ceiling_millis: LeaseTimeoutMillisV1::new(1_000).expect("session timeout"),
            evidence_obligations: Vec::new(),
            evidence_destination: record_ref("evidence-destination"),
            issued_at: UnixTimeV1::new(55).expect("session issue time"),
            expires_at: UnixTimeV1::new(700).expect("session expiry"),
            revocation_epoch: PrincipalRevocationEpochV1::new(1).expect("session revocation"),
            checked_revocation_ref: None,
            checked_revocation_sha256: None,
            native_session_correlation: NativeSessionCorrelationV1::Unobserved {
                reason: ResidentReasonV1::parse("native session identity unavailable")
                    .expect("native session reason"),
            },
            issuer_principal_id: self.signing.principal_id().clone(),
            issuer_key_id: ResidentKeyIdV1::parse(self.signing.signer_key_id())
                .expect("issuer key"),
            issuer_key_epoch: *self.signing.signer_key_epoch(),
        };
        let session_digest = SessionAdmissionLeaseDigest::hash_canonical(&session_preimage)
            .expect("session lease digest");
        let session_wire: SessionAdmissionLeaseWireV1 = signed_wire(
            &session_preimage,
            "session_admission_lease_digest",
            &session_digest,
        );
        let mut authority = ProvingAuthority;
        let session = admit_ambient_session(session_wire, &current, &mut authority)
            .expect("ambient session admits");
        let lease = session
            .admit_operation(
                pale_contracts::record_schema_identity(OPERATION_LEASE_V1_SCHEMA_ID)
                    .expect("operation lease schema")
                    .schema_sha256,
                OperationLeaseRequestV1 {
                    operation_lease_id: OperationLeaseIdV1::parse("operation-lease-1")
                        .expect("operation lease"),
                    operation: PackageResourceRefV1 {
                        package_id: PackageIdV1::parse("example.engine").expect("Package"),
                        resource_id: PackageResourceIdV1::parse("review").expect("operation"),
                    },
                    capabilities: vec![CapabilityIdV1::parse("read").expect("capability")],
                    effects: vec![EffectClass::Compute],
                    read_set: Vec::new(),
                    evidence_obligations: Vec::new(),
                    budget_units: LeaseBudgetUnitsV1::new(5).expect("operation budget"),
                    timeout_millis: LeaseTimeoutMillisV1::new(500).expect("operation timeout"),
                    expires_at: UnixTimeV1::new(650).expect("operation expiry"),
                    invocation_correlation: InvocationCorrelationV1::Unobserved {
                        reason: ResidentReasonV1::parse(
                            "native invocation identity unavailable in conformance",
                        )
                        .expect("invocation correlation reason"),
                    },
                },
                &current,
                &mut authority,
            )
            .expect("operation lease admits");
        use_session(&session, &current, &lease)
    }
}

/// Deterministic authority proof model for already admitted records.
#[derive(Default)]
pub(in super::super) struct ProvingAuthority;

impl AmbientSessionAuthorityPort for ProvingAuthority {
    fn verify_session_admission_lease_signature(
        &mut self,
        _: &[u8],
        _: &SessionAdmissionLeasePreimageV1,
        _: &SessionAdmissionLeaseDigest,
        candidate: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        (candidate == &signature()).then_some(()).ok_or(
            InvalidResidentRecordV1::ContextualAdmission {
                record: "interface conformance session",
                invariant: "deterministic session signature",
            },
        )
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
