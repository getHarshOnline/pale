//! Canonical admitted-generation evidence shared by resident and storage tests.
//!
//! The compiler and contextual admission are real. Runtime probes and
//! materialization observations are synthetic; this fixture proves no process
//! identity, target write, production storage profile, or native readiness.

use super::compile::{compile_live_inputs, Compilation, CompileError};
use super::packages::inspect_distribution;
use super::plan::{
    bind_machine_local_effects, build_success_receipt_with_lineage, is_filesystem_target,
    AdmittedSuccessReceipt, MaterializationIntent, MaterializationLineageEvidence,
    RepositoryMaterializationInput,
};
use super::resident::{
    AppliedRepositoryMaterialization, CompiledGenerationAdmissionError,
    GenerationLifecycleRecoveryResolver, ResidentApplication, ResidentApplicationError,
    ResidentGenerationInputSource, ResidentGenerationInputSourceError,
    ResidentGenerationReconstructionInput,
};
use super::workspace::{resolve_entry, WorkspaceInputSnapshot};
use super::{composition_source_fixtures, package_fixtures};
use pale_core::admission::{admit_plan, PlanKind};
use pale_core::records::*;
use pale_ports::{
    ExecutableIdentityError, ExecutableIdentityPort, ExecutableIdentityRequest,
    ExecutableObservation, IdentityProbeRequest, ProbeObservation, ProcessProbeError,
    ProcessProbePort,
};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[path = "compilation.rs"]
mod compilation_fixtures;
#[path = "role_package.rs"]
pub(crate) mod role_package_fixture;
#[path = "workspace.rs"]
pub(crate) mod workspace_fixtures;
pub(crate) use compilation_fixtures::SyntheticWorkspace;

pub(crate) fn synthetic_set_policy() -> pale_kernel::AdaptiveNodeSetPolicy {
    pale_kernel::AdaptiveNodeSetPolicy::new(32, 2_500).expect("synthetic set policy")
}

pub(crate) struct SurfaceExecutableProbe {
    identity: SurfaceExecutableIdentityV1,
}

impl ExecutableIdentityPort for SurfaceExecutableProbe {
    fn verify_executable(
        &self,
        request: ExecutableIdentityRequest<'_>,
    ) -> Result<ExecutableObservation, ExecutableIdentityError> {
        Ok(ExecutableObservation {
            canonical_path: PathBuf::from(request.executable),
        })
    }
}

impl ProcessProbePort for SurfaceExecutableProbe {
    fn probe(
        &self,
        request: IdentityProbeRequest<'_>,
    ) -> Result<ProbeObservation, ProcessProbeError> {
        assert_eq!(request.argument, "compiled-identity");
        Ok(ProbeObservation {
            stdout: serde_json::to_vec(&self.identity).expect("executable identity serializes"),
        })
    }
}

pub(crate) fn surface_executable_probe() -> SurfaceExecutableProbe {
    let schema = pale_contracts::record_schema_identity("surface-executable-identity.v1")
        .expect("executable schema identity");
    SurfaceExecutableProbe {
        identity: SurfaceExecutableIdentityV1 {
            schema: schema.schema,
            schema_sha256: schema.schema_sha256,
            schema_version: 1,
            version: "1.0.0".into(),
            protocol: "pale.agent-surface-interface.v1".into(),
            os: pale_core::platform::target_os().into(),
            arch: pale_core::platform::target_arch().into(),
            durable_publication: pale_core::platform::durable_publication(),
        },
    }
}

pub(crate) fn bind_surface_executable(
    workspace: &SyntheticWorkspace,
    probe: &SurfaceExecutableProbe,
) {
    let source = std::fs::read_to_string(&workspace.machine_binding).expect("machine binding");
    let mut machine: MachineBindingFile = toml::from_str(&source).expect("typed machine binding");
    machine.surface_executable = Some(SurfaceExecutableBinding {
        exec: "/synthetic/pale".into(),
        digest: ExecutableDigest::hash_bytes(b"synthetic surface executable"),
        version: probe.identity.version.clone(),
        protocol: probe.identity.protocol.clone(),
    });
    std::fs::write(
        &workspace.machine_binding,
        toml::to_string(&machine).expect("serialize machine binding"),
    )
    .expect("write machine binding");
}

pub(crate) fn compile_live(
    workspace: &SyntheticWorkspace,
    probe: &SurfaceExecutableProbe,
) -> Compilation {
    compile_live_inputs(
        workspace.capture_inputs().expect("capture fixture inputs"),
        &pale_adapter_fs::LocalSourceFilesystem,
        &pale_adapter_fs::LocalMachineFilesystem,
        probe,
    )
    .expect("fixture compiles with an admitted surface executable")
}

pub(crate) struct AppliedEvidence {
    pub(crate) receipt: AdmittedSuccessReceipt,
    pub(crate) state: MaterializationStateV1,
    pub(crate) lineage: MaterializationLineageV1,
    pub(crate) source_frontier: RepositorySourceFrontierV1,
}

impl AppliedEvidence {
    pub(crate) fn into_materialization(self) -> AppliedRepositoryMaterialization {
        self.try_into_materialization()
            .expect("owner materialization records admit")
    }

    pub(crate) fn try_into_materialization(
        self,
    ) -> Result<AppliedRepositoryMaterialization, CompiledGenerationAdmissionError> {
        AppliedRepositoryMaterialization::admit(
            self.receipt,
            self.state,
            self.lineage,
            self.source_frontier,
        )
    }
}

pub(crate) fn applied_evidence(compilation: &Compilation) -> AppliedEvidence {
    let repository = compilation
        .repositories()
        .first()
        .expect("fixture repository");
    let baseline = repository
        .render()
        .surfaces
        .iter()
        .filter(|surface| !surface.deferred && is_filesystem_target(&surface.rel_path))
        .map(|surface| BaselineEntry {
            rel_path: surface.rel_path.clone(),
            observation: BaselineStateV1::Absent {},
        })
        .collect();
    let surface_plan = RepositoryMaterializationInput::for_repository(compilation, repository.id())
        .expect("materialization input")
        .build(MaterializationIntent::Apply, None, baseline)
        .expect("initial apply plan")
        .into_parts()
        .0;
    let state = surface_plan
        .transition
        .next_state()
        .expect("apply has a next state")
        .clone();
    let operations = surface_plan
        .operations
        .iter()
        .map(|operation| {
            let SurfaceOperationTransitionV1::Create { desired_sha256, .. } = &operation.transition
            else {
                panic!("an absent baseline produces only create operations");
            };
            ReceiptOperation {
                rel_path: operation.rel_path.clone(),
                observation: SurfaceOperationObservationV1::Create {
                    target: SurfaceCreationObservationV1::AbsentFile {
                        post_sha256: desired_sha256.clone(),
                    },
                },
                mode: operation.mode,
            }
        })
        .collect();
    let materialization_plan =
        bind_machine_local_effects(surface_plan, Vec::new()).expect("materialization plan");
    let admitted_plan = admit_plan(
        &materialization_plan,
        &materialization_plan,
        PlanKind::Apply,
    )
    .expect("plan admits");
    let lineage_schema = pale_contracts::record_schema_identity("materialization-lineage.v1")
        .expect("lineage schema identity");
    let lineage = MaterializationLineageV1::seal(MaterializationLineagePreimageV1 {
        schema: lineage_schema.schema,
        schema_sha256: lineage_schema.schema_sha256,
        schema_version: MATERIALIZATION_LINEAGE_SCHEMA_VERSION,
        schema_manifest_sha256: state.schema_manifest_sha256.clone(),
        schema_set_sha256: state.schema_set_sha256.clone(),
        one_shot_compilation_digest: state.one_shot_compilation_digest.clone(),
        repository_id: state.repository_id.clone(),
        target_root_sha256: pale_adapter_fs::target_root_sha256(Path::new("/synthetic/engine")),
        profile_digest: state.profile_digest.clone(),
        instance_digest: state.instance_digest.clone(),
        manifest_digest: state.manifest_digest.clone(),
        state_digest: state.state_digest.clone(),
        admitted_plan_digest: admitted_plan.digest().clone(),
    })
    .expect("lineage seals");
    let receipt = build_success_receipt_with_lineage(
        &admitted_plan,
        operations,
        Vec::new(),
        MaterializationLineageEvidence::Initialized {
            post: lineage.lineage_digest.clone(),
        },
    )
    .expect("success receipt admits");
    let source_frontier = RepositorySourceFrontierV1 {
        repository_id: repository.id().clone(),
        base_revision: RepositoryRevisionV1::parse("base-revision").expect("base revision"),
        tracked_delta_sha256: ArtifactDigest::hash_bytes(b"tracked delta"),
        untracked_inventory_sha256: ArtifactDigest::hash_bytes(b"untracked inventory"),
        dependency_frontier_sha256: ArtifactDigest::hash_bytes(b"dependency frontier"),
    };
    AppliedEvidence {
        receipt,
        state,
        lineage,
        source_frontier,
    }
}

pub(crate) fn live_workspace() -> (SyntheticWorkspace, SurfaceExecutableProbe) {
    let workspace = SyntheticWorkspace::new(false);
    let probe = surface_executable_probe();
    bind_surface_executable(&workspace, &probe);
    (workspace, probe)
}

fn registered_schema_digest(registry_id: &'static str) -> SchemaDigest {
    pale_contracts::record_schema_identity(registry_id)
        .expect("resident lifecycle schema is registered")
        .schema_sha256
}

fn wire<P: Serialize, W: DeserializeOwned, D: Serialize>(
    preimage: &P,
    digest_field: &str,
    digest: &D,
) -> W {
    let mut value = serde_json::to_value(preimage).expect("preimage serializes");
    value
        .as_object_mut()
        .expect("record preimage is an object")
        .insert(
            digest_field.into(),
            serde_json::to_value(digest).expect("digest serializes"),
        );
    serde_json::from_value(value).expect("wire decodes")
}

pub(crate) fn published(
    generation: &CompiledGenerationV1,
    sequence: u64,
    predecessor: Option<&PublishedGenerationHeadV1>,
) -> PublishedGenerationHeadV1 {
    let schema_digest = registered_schema_digest(PUBLISHED_GENERATION_HEAD_V1_SCHEMA_ID);
    let preimage = PublishedGenerationHeadPreimageV1 {
        schema: SchemaId::parse(PUBLISHED_GENERATION_HEAD_V1_SCHEMA).expect("published schema"),
        schema_sha256: schema_digest.clone(),
        schema_version: PUBLISHED_GENERATION_HEAD_V1_SCHEMA_VERSION,
        compiled_generation_digest: generation.compiled_generation_digest().clone(),
        journal_sequence: JournalSequenceV1::new(sequence).expect("sequence"),
        predecessor_published_generation_head_digest: predecessor
            .map(|head| head.published_generation_head_digest().clone()),
    };
    let digest =
        PublishedGenerationHeadDigest::hash_canonical(&preimage).expect("published digest");
    PublishedGenerationHeadV1::admit_contextual_wire(
        wire(&preimage, "published_generation_head_digest", &digest),
        &PublishedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: &schema_digest,
            compiled_generation: generation,
            predecessor,
        },
    )
    .expect("published head admits")
}

pub(crate) fn advertised(
    published: &PublishedGenerationHeadV1,
    sequence: u64,
    predecessor: Option<&AdvertisedGenerationHeadV1>,
) -> AdvertisedGenerationHeadV1 {
    let schema_digest = registered_schema_digest(ADVERTISED_GENERATION_HEAD_V1_SCHEMA_ID);
    let preimage = AdvertisedGenerationHeadPreimageV1 {
        schema: SchemaId::parse(ADVERTISED_GENERATION_HEAD_V1_SCHEMA).expect("advertised schema"),
        schema_sha256: schema_digest.clone(),
        schema_version: ADVERTISED_GENERATION_HEAD_V1_SCHEMA_VERSION,
        compiled_generation_digest: published.compiled_generation_digest().clone(),
        published_generation_head_digest: published.published_generation_head_digest().clone(),
        journal_sequence: JournalSequenceV1::new(sequence).expect("sequence"),
        predecessor_advertised_generation_head_digest: predecessor
            .map(|head| head.advertised_generation_head_digest().clone()),
    };
    let digest =
        AdvertisedGenerationHeadDigest::hash_canonical(&preimage).expect("advertised digest");
    AdvertisedGenerationHeadV1::admit_contextual_wire(
        wire(&preimage, "advertised_generation_head_digest", &digest),
        &AdvertisedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: &schema_digest,
            published_head: published,
            predecessor,
        },
    )
    .expect("advertised head admits")
}

pub(crate) fn fence(
    generation: &CompiledGenerationV1,
    ordinal: u64,
    predecessor: Option<&ActivationFenceV1>,
) -> ActivationFenceV1 {
    let schema_digest = registered_schema_digest(ACTIVATION_FENCE_V1_SCHEMA_ID);
    let preimage = ActivationFencePreimageV1 {
        schema: SchemaId::parse(ACTIVATION_FENCE_V1_SCHEMA).expect("fence schema"),
        schema_sha256: schema_digest.clone(),
        schema_version: ACTIVATION_FENCE_V1_SCHEMA_VERSION,
        activation_fence_id: ActivationFenceIdV1::parse("deployment-fence").expect("fence id"),
        activation_fence_generation: ActivationFenceGenerationV1::new(ordinal)
            .expect("fence generation"),
        compiled_generation_digest: generation.compiled_generation_digest().clone(),
        predecessor_activation_fence_digest: predecessor
            .map(|value| value.activation_fence_digest().clone()),
    };
    let digest = ActivationFenceDigest::hash_canonical(&preimage).expect("fence digest");
    ActivationFenceV1::admit_contextual_wire(
        wire(&preimage, "activation_fence_digest", &digest),
        &ActivationFenceAdmissionContextV1 {
            expected_schema_sha256: &schema_digest,
            compiled_generation: generation,
            predecessor,
        },
    )
    .expect("fence admits")
}

pub(crate) fn active(
    advertised: &AdvertisedGenerationHeadV1,
    fence: &ActivationFenceV1,
    sequence: u64,
    predecessor: Option<&ActiveGenerationHeadV1>,
) -> ActiveGenerationHeadV1 {
    let schema_digest = registered_schema_digest(ACTIVE_GENERATION_HEAD_V1_SCHEMA_ID);
    let preimage = ActiveGenerationHeadPreimageV1 {
        schema: SchemaId::parse(ACTIVE_GENERATION_HEAD_V1_SCHEMA).expect("active schema"),
        schema_sha256: schema_digest.clone(),
        schema_version: ACTIVE_GENERATION_HEAD_V1_SCHEMA_VERSION,
        compiled_generation_digest: advertised.compiled_generation_digest().clone(),
        advertised_generation_head_digest: advertised.advertised_generation_head_digest().clone(),
        activation_fence_digest: fence.activation_fence_digest().clone(),
        journal_sequence: JournalSequenceV1::new(sequence).expect("sequence"),
        predecessor_active_generation_head_digest: predecessor
            .map(|head| head.active_generation_head_digest().clone()),
    };
    let digest = ActiveGenerationHeadDigest::hash_canonical(&preimage).expect("active digest");
    ActiveGenerationHeadV1::admit_contextual_wire(
        wire(&preimage, "active_generation_head_digest", &digest),
        &ActiveGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: &schema_digest,
            advertised_head: advertised,
            activation_fence: fence,
            predecessor,
        },
    )
    .expect("active head admits")
}

#[derive(Default)]
pub(crate) struct RecoveryResolver {
    pub(crate) generations: BTreeMap<CompiledGenerationDigest, CompiledGenerationV1>,
    pub(crate) published: BTreeMap<PublishedGenerationHeadDigest, PublishedGenerationHeadV1>,
    pub(crate) advertised: BTreeMap<AdvertisedGenerationHeadDigest, AdvertisedGenerationHeadV1>,
    pub(crate) active: BTreeMap<ActiveGenerationHeadDigest, ActiveGenerationHeadV1>,
    pub(crate) fences: BTreeMap<ActivationFenceDigest, ActivationFenceV1>,
    pub(crate) profiles: BTreeMap<ProfileDigest, EffectiveProjectProfileV1>,
}

impl RecoveryResolver {
    pub(crate) fn insert_generation(&mut self, value: &CompiledGenerationV1) {
        self.generations
            .insert(value.compiled_generation_digest().clone(), value.clone());
    }

    pub(crate) fn insert_published(&mut self, value: &PublishedGenerationHeadV1) {
        self.published.insert(
            value.published_generation_head_digest().clone(),
            value.clone(),
        );
    }

    pub(crate) fn insert_advertised(&mut self, value: &AdvertisedGenerationHeadV1) {
        self.advertised.insert(
            value.advertised_generation_head_digest().clone(),
            value.clone(),
        );
    }

    pub(crate) fn insert_active(&mut self, value: &ActiveGenerationHeadV1) {
        self.active
            .insert(value.active_generation_head_digest().clone(), value.clone());
    }

    pub(crate) fn insert_fence(&mut self, value: &ActivationFenceV1) {
        self.fences
            .insert(value.activation_fence_digest().clone(), value.clone());
    }

    pub(crate) fn insert_effective_profile(&mut self, value: &EffectiveProjectProfileV1) {
        self.profiles
            .insert(value.profile_digest.clone(), value.clone());
    }
}

impl GenerationLifecycleRecoveryResolver for RecoveryResolver {
    fn session_signing_authority(
        &self,
        authority_ref: &ResidentRecordRefV1,
        authority_sha256: &ArtifactDigest,
        issuer_principal_id: &ResidentPrincipalIdV1,
        issuer_key_id: &ResidentKeyIdV1,
        issuer_key_epoch: SigningKeyEpochV1,
    ) -> Option<SigningAuthorityBindingV1> {
        SigningAuthorityBindingV1::integrated_runtime(
            issuer_principal_id.clone(),
            issuer_key_id.clone(),
            issuer_key_epoch,
            IntegratedRuntimeAuthorityV1 {
                deployment_runtime_authority_ref: authority_ref.clone(),
                deployment_runtime_authority_sha256: authority_sha256.clone(),
                deployment_binding_ref: ResidentRecordRefV1::parse("deployment-binding")
                    .expect("deployment binding reference"),
                deployment_binding_sha256: ArtifactDigest::parse(
                    std::iter::repeat_n('1', 64).collect::<String>(),
                )
                .expect("deployment binding digest"),
                checked_principal_trust_checkpoint_ref: ResidentRecordRefV1::parse(
                    "trust-checkpoint",
                )
                .expect("trust checkpoint reference"),
                checked_principal_trust_checkpoint_sha256: ArtifactDigest::parse(
                    std::iter::repeat_n('7', 64).collect::<String>(),
                )
                .expect("trust checkpoint digest"),
            },
        )
        .ok()
    }

    fn verify_session_admission_lease_signature(
        &self,
        _: &SigningAuthorityBindingV1,
        _: &[u8],
        _: &SessionAdmissionLeasePreimageV1,
        _: &SessionAdmissionLeaseDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }

    fn compiled_generation(
        &self,
        digest: &CompiledGenerationDigest,
    ) -> Option<&CompiledGenerationV1> {
        self.generations.get(digest)
    }

    fn published_head(
        &self,
        digest: &PublishedGenerationHeadDigest,
    ) -> Option<&PublishedGenerationHeadV1> {
        self.published.get(digest)
    }

    fn advertised_head(
        &self,
        digest: &AdvertisedGenerationHeadDigest,
    ) -> Option<&AdvertisedGenerationHeadV1> {
        self.advertised.get(digest)
    }

    fn active_head(&self, digest: &ActiveGenerationHeadDigest) -> Option<&ActiveGenerationHeadV1> {
        self.active.get(digest)
    }

    fn activation_fence(&self, digest: &ActivationFenceDigest) -> Option<&ActivationFenceV1> {
        self.fences.get(digest)
    }

    fn effective_profile(&self, digest: &ProfileDigest) -> Option<&EffectiveProjectProfileV1> {
        self.profiles.get(digest)
    }
}

#[derive(Default)]
pub(crate) struct FixedInputSource {
    pub(crate) calls: usize,
    pub(crate) inputs: Vec<ResidentGenerationReconstructionInput>,
    pub(crate) requested: Option<BTreeSet<CompiledGenerationDigest>>,
}

impl ResidentGenerationInputSource for FixedInputSource {
    fn load_generation_inputs(
        &mut self,
        required_generations: &BTreeSet<CompiledGenerationDigest>,
    ) -> Result<Vec<ResidentGenerationReconstructionInput>, ResidentGenerationInputSourceError>
    {
        self.calls += 1;
        self.requested = Some(required_generations.clone());
        Ok(std::mem::take(&mut self.inputs))
    }
}

pub(crate) struct ActiveResidentFixture {
    pub(crate) generation: CompiledGenerationV1,
    pub(crate) profile: EffectiveProjectProfileV1,
    pub(crate) published: PublishedGenerationHeadV1,
    pub(crate) advertised: AdvertisedGenerationHeadV1,
    pub(crate) fence: ActivationFenceV1,
    pub(crate) active: ActiveGenerationHeadV1,
}

/// Seal one exact session lease against the fixture's admitted active generation.
pub(crate) fn session_lease_for_active(
    fixture: &ActiveResidentFixture,
    session_admission_id: &str,
    budget_ceiling_units: u64,
) -> SessionAdmissionLeaseV1 {
    session_lease_for_profile(
        &fixture.generation,
        &fixture.active,
        &fixture.fence,
        &fixture.profile,
        session_admission_id,
        budget_ceiling_units,
    )
}

/// Seal one contextual session lease through the same fixture owner used by resident tests.
pub(crate) fn session_lease_for_profile(
    generation: &CompiledGenerationV1,
    active: &ActiveGenerationHeadV1,
    fence: &ActivationFenceV1,
    profile: &EffectiveProjectProfileV1,
    session_admission_id: &str,
    budget_ceiling_units: u64,
) -> SessionAdmissionLeaseV1 {
    session_lease_for_profile_with_native_correlation(
        generation,
        active,
        fence,
        profile,
        session_admission_id,
        budget_ceiling_units,
        NativeSessionCorrelationV1::Unobserved {
            reason: ResidentReasonV1::parse("native identity unavailable").expect("reason"),
        },
    )
}

/// Seal one fixture lease with an explicit privacy-preserving host correlation.
pub(crate) fn session_lease_for_profile_with_native_correlation(
    generation: &CompiledGenerationV1,
    active: &ActiveGenerationHeadV1,
    fence: &ActivationFenceV1,
    profile: &EffectiveProjectProfileV1,
    session_admission_id: &str,
    budget_ceiling_units: u64,
    native_session_correlation: NativeSessionCorrelationV1,
) -> SessionAdmissionLeaseV1 {
    assert_eq!(
        generation.profile_digest(),
        &profile.profile_digest,
        "session fixture generation must bind its exact effective profile",
    );
    let schema_identity =
        pale_contracts::record_schema_identity(SESSION_ADMISSION_LEASE_V1_SCHEMA_ID)
            .expect("registered session schema identity");
    let record_ref =
        |value: &str| ResidentRecordRefV1::parse(value).expect("resident record reference");
    let runtime_authority_ref = record_ref("runtime-authority");
    let runtime_authority_sha256 =
        ArtifactDigest::parse(std::iter::repeat_n('6', 64).collect::<String>())
            .expect("runtime authority digest");
    let signing_authority = SigningAuthorityBindingV1::integrated_runtime(
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        ResidentKeyIdV1::parse("runtime-key-1").expect("runtime key"),
        SigningKeyEpochV1::new(1).expect("runtime key epoch"),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: runtime_authority_ref.clone(),
            deployment_runtime_authority_sha256: runtime_authority_sha256.clone(),
            deployment_binding_ref: record_ref("deployment-binding"),
            deployment_binding_sha256: ArtifactDigest::parse(
                std::iter::repeat_n('1', 64).collect::<String>(),
            )
            .expect("deployment binding digest"),
            checked_principal_trust_checkpoint_ref: record_ref("trust-checkpoint"),
            checked_principal_trust_checkpoint_sha256: ArtifactDigest::parse(
                std::iter::repeat_n('7', 64).collect::<String>(),
            )
            .expect("trust checkpoint digest"),
        },
    )
    .expect("integrated runtime signing authority");
    let preimage = SessionAdmissionLeasePreimageV1 {
        schema_id: schema_identity.schema.clone(),
        schema_sha256: schema_identity.schema_sha256.clone(),
        session_admission_id: SessionAdmissionIdV1::parse(session_admission_id)
            .expect("session id"),
        compiled_generation_digest: generation.compiled_generation_digest().clone(),
        active_generation_head_digest: active.active_generation_head_digest().clone(),
        activation_fence_digest: fence.activation_fence_digest().clone(),
        deployment_binding_ref: record_ref("deployment-binding"),
        deployment_binding_sha256: ArtifactDigest::parse(
            std::iter::repeat_n('1', 64).collect::<String>(),
        )
        .expect("deployment binding digest"),
        deployment_runtime_authority_ref: runtime_authority_ref,
        deployment_runtime_authority_sha256: runtime_authority_sha256,
        principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        controller_id: ResidentControllerIdV1::parse("controller-1").expect("controller"),
        relationship_scope: None,
        repository_id: generation.repository_source_frontiers()[0]
            .repository_id
            .clone(),
        project_id: profile.project_id.clone(),
        execution_workspace_id: profile.execution_workspace_id.clone(),
        profile_digest: generation.profile_digest().clone(),
        instance_digest: generation.instance_digest().clone(),
        execution_path: generation.execution_paths()[0].clone(),
        harness_adapter_source_sha256: ArtifactDigest::parse(
            std::iter::repeat_n('2', 64).collect::<String>(),
        )
        .expect("adapter source digest"),
        harness_adapter_build_sha256: ArtifactDigest::parse(
            std::iter::repeat_n('3', 64).collect::<String>(),
        )
        .expect("adapter build digest"),
        harness_executable_sha256: ArtifactDigest::parse(
            std::iter::repeat_n('4', 64).collect::<String>(),
        )
        .expect("harness executable digest"),
        harness_configuration_sha256: ArtifactDigest::parse(
            std::iter::repeat_n('5', 64).collect::<String>(),
        )
        .expect("harness configuration digest"),
        capability_ceiling: vec![CapabilityIdV1::parse("read").expect("capability")],
        read_ceiling: vec![record_ref("read-a")],
        effect_ceiling: vec![EffectClass::Read],
        budget_ceiling_units: LeaseBudgetUnitsV1::new(budget_ceiling_units)
            .expect("valid fixture budget"),
        timeout_ceiling_millis: LeaseTimeoutMillisV1::new(1_000).expect("valid fixture timeout"),
        evidence_obligations: vec![record_ref("evidence-a")],
        evidence_destination: record_ref("evidence-destination"),
        issued_at: UnixTimeV1::new(10).expect("valid fixture time"),
        expires_at: UnixTimeV1::new(100).expect("valid fixture time"),
        revocation_epoch: PrincipalRevocationEpochV1::new(1)
            .expect("valid fixture revocation epoch"),
        checked_revocation_ref: None,
        checked_revocation_sha256: None,
        native_session_correlation,
        issuer_principal_id: signing_authority.principal_id().clone(),
        issuer_key_id: ResidentKeyIdV1::parse(signing_authority.signer_key_id())
            .expect("issuer key"),
        issuer_key_epoch: *signing_authority.signer_key_epoch(),
    };
    let digest = SessionAdmissionLeaseDigest::hash_canonical(&preimage).expect("session digest");
    let mut wire = serde_json::to_value(&preimage).expect("session preimage serializes");
    wire.as_object_mut()
        .expect("session preimage is an object")
        .insert(
            "session_admission_lease_digest".into(),
            serde_json::to_value(digest).expect("session digest serializes"),
        );
    wire.as_object_mut()
        .expect("session preimage is an object")
        .insert(
            "signature".into(),
            serde_json::to_value(CanonicalSignatureV1::parse("A".repeat(86)).expect("signature"))
                .expect("signature serializes"),
        );
    SessionAdmissionLeaseV1::admit_contextual_wire(
        serde_json::from_value::<SessionAdmissionLeaseWireV1>(wire).expect("session wire"),
        &SessionAdmissionLeaseContextV1 {
            active_head: active,
            compiled_generation: generation,
            effective_profile: profile,
            activation_fence: fence,
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema_identity.schema_sha256,
                signing_authority: &signing_authority,
            },
        },
        |_, _, _, _| Ok(()),
    )
    .expect("session lease admits")
}

pub(crate) fn admit_and_activate(
    resident: &mut ResidentApplication<'_>,
    workspace: &SyntheticWorkspace,
    probe: &SurfaceExecutableProbe,
) -> Result<ActiveResidentFixture, ResidentApplicationError> {
    let compilation = compile_live(workspace, probe);
    let profile = compilation.profile().clone();
    let materializations = vec![applied_evidence(&compilation).into_materialization()];
    let digest =
        resident.admit_generation(compilation, materializations, synthetic_set_policy())?;
    let generation = resident
        .generation(&digest)
        .expect("admitted generation")
        .record()
        .clone();
    let published = published(&generation, 1, None);
    let _ = resident.publish(published.clone())?;
    let advertised = advertised(&published, 2, None);
    let _ = resident.advertise(advertised.clone())?;
    let fence = fence(&generation, 1, None);
    let active = active(&advertised, &fence, 3, None);
    let _ = resident.activate(active.clone(), fence.clone())?;
    Ok(ActiveResidentFixture {
        generation,
        profile,
        published,
        advertised,
        fence,
        active,
    })
}

pub(crate) fn resolver_for_active(fixture: &ActiveResidentFixture) -> RecoveryResolver {
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(&fixture.generation);
    resolver.insert_effective_profile(&fixture.profile);
    resolver.insert_published(&fixture.published);
    resolver.insert_advertised(&fixture.advertised);
    resolver.insert_fence(&fixture.fence);
    resolver.insert_active(&fixture.active);
    resolver
}

pub(crate) fn reconstruction_input(
    workspace: &SyntheticWorkspace,
    probe: &SurfaceExecutableProbe,
    fixture: &ActiveResidentFixture,
) -> ResidentGenerationReconstructionInput {
    let compilation = compile_live(workspace, probe);
    let materializations = vec![applied_evidence(&compilation).into_materialization()];
    ResidentGenerationReconstructionInput::new(
        fixture.generation.compiled_generation_digest().clone(),
        compilation,
        materializations,
    )
}
