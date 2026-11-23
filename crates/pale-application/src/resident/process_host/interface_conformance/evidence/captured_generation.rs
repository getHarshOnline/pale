//! Path-free compiler fixture for the resident interface conformance journey.

use std::path::{Path, PathBuf};

use pale_core::{
    admission::{admit_plan, PlanKind},
    records::*,
};

use crate::{
    compile::{
        compile_captured_machine_verified_inputs, CapturedOneShotCompilationInputs,
        CapturedPackageDocument, CapturedPackageSource, CapturedPrivateDocument,
        CapturedPrivateWorkspaceInputs, Compilation,
    },
    plan::{
        bind_machine_local_effects, build_success_receipt_with_lineage, is_filesystem_target,
        MaterializationIntent, MaterializationLineageEvidence, RepositoryMaterializationInput,
    },
    resident::{
        AdmittedCompiledGeneration, AppliedRepositoryMaterialization,
        ResidentGenerationReconstructionInput,
    },
    workspace::{compile_portable_workspace, CapturedWorkspaceDocument, CapturedWorkspaceSource},
};

use pale_ports::{
    ExecutableIdentityError, ExecutableIdentityPort, ExecutableIdentityRequest,
    ExecutableObservation, IdentityProbeRequest, MachineFilesystemError, MachineFilesystemPort,
    MachinePathKind, MachinePathObservation, ProbeObservation, ProcessProbeError, ProcessProbePort,
};

const CONFORMANCE_EXECUTABLE: &str = "/synthetic/pale";
const CONFORMANCE_EXECUTABLE_BYTES: &[u8] = b"in-memory Pale conformance executable";

struct ConformanceMachine {
    identity: SurfaceExecutableIdentityV1,
}

impl ConformanceMachine {
    fn new() -> Self {
        let schema = pale_contracts::record_schema_identity("surface-executable-identity.v1")
            .expect("executable identity schema");
        Self {
            identity: SurfaceExecutableIdentityV1 {
                schema: schema.schema,
                schema_sha256: schema.schema_sha256,
                schema_version: 1,
                version: "1.0.0".into(),
                protocol: SURFACE_INTERFACE_PROTOCOL.into(),
                os: pale_core::platform::target_os().into(),
                arch: pale_core::platform::target_arch().into(),
                durable_publication: pale_core::platform::durable_publication(),
            },
        }
    }
}

impl ExecutableIdentityPort for ConformanceMachine {
    fn verify_executable(
        &self,
        request: ExecutableIdentityRequest<'_>,
    ) -> Result<ExecutableObservation, ExecutableIdentityError> {
        if request.executable != CONFORMANCE_EXECUTABLE {
            return Err(ExecutableIdentityError::NotFound(request.executable.into()));
        }
        let actual = ExecutableDigest::hash_bytes(CONFORMANCE_EXECUTABLE_BYTES);
        if request.expected_digest != &actual {
            return Err(ExecutableIdentityError::DigestMismatch {
                actual,
                expected: request.expected_digest.clone(),
            });
        }
        Ok(ExecutableObservation {
            canonical_path: PathBuf::from(CONFORMANCE_EXECUTABLE),
        })
    }
}

impl ProcessProbePort for ConformanceMachine {
    fn probe(
        &self,
        request: IdentityProbeRequest<'_>,
    ) -> Result<ProbeObservation, ProcessProbeError> {
        self.verify_executable(ExecutableIdentityRequest {
            executable: request.executable,
            expected_digest: request.expected_digest,
        })?;
        assert_eq!(request.argument, "compiled-identity");
        Ok(ProbeObservation {
            stdout: pale_core::canon::to_canonical_string(&self.identity)
                .expect("executable identity canonicalizes")
                .into_bytes(),
        })
    }
}

impl MachineFilesystemPort for ConformanceMachine {
    fn inspect_requested(&self, _: &Path) -> Result<MachinePathKind, MachineFilesystemError> {
        Ok(MachinePathKind::Directory)
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, MachineFilesystemError> {
        Ok(path.to_path_buf())
    }

    fn observe(&self, path: &Path) -> Result<MachinePathObservation, MachineFilesystemError> {
        Ok(MachinePathObservation::new(
            path.to_path_buf(),
            MachinePathKind::Directory,
            MachinePathKind::Directory,
            None,
        ))
    }
}

pub(in super::super) struct CapturedGenerationFixture {
    pub(in super::super) generation: AdmittedCompiledGeneration,
    pub(in super::super) profile: EffectiveProjectProfileV1,
    pub(in super::super) initial_compilation: Compilation,
    pub(in super::super) initial_materializations: Vec<AppliedRepositoryMaterialization>,
    pub(in super::super) reconstructions: Vec<ResidentGenerationReconstructionInput>,
}

pub(in super::super) fn captured_generation() -> CapturedGenerationFixture {
    let (compilation, materializations, profile) = captured_generation_inputs();
    let generation = AdmittedCompiledGeneration::admit(
        compilation,
        materializations,
        pale_kernel::AdaptiveNodeSetPolicy::new(32, 2_500).expect("bounded conformance generation"),
    )
    .expect("captured interface conformance generation admits");
    let expected_generation = generation.record().compiled_generation_digest().clone();
    let (initial_compilation, initial_materializations, initial_profile) =
        captured_generation_inputs();
    let reconstruction_inputs = (0..2)
        .map(|_| captured_generation_inputs())
        .collect::<Vec<_>>();
    assert_eq!(
        profile.profile_digest, initial_profile.profile_digest,
        "initial admission uses the same captured profile"
    );
    assert_eq!(
        profile.profile_digest,
        reconstruction_inputs
            .first()
            .expect("one reconstruction")
            .2
            .profile_digest,
        "reconstruction uses the same captured profile"
    );
    let reconstructions = reconstruction_inputs
        .into_iter()
        .map(|(compilation, materializations, reconstruction_profile)| {
            assert_eq!(
                profile.profile_digest, reconstruction_profile.profile_digest,
                "each reconstruction uses the same captured profile"
            );
            ResidentGenerationReconstructionInput::new(
                expected_generation.clone(),
                compilation,
                materializations,
            )
        })
        .collect();
    CapturedGenerationFixture {
        generation,
        profile,
        initial_compilation,
        initial_materializations,
        reconstructions,
    }
}

fn captured_generation_inputs() -> (
    Compilation,
    Vec<AppliedRepositoryMaterialization>,
    EffectiveProjectProfileV1,
) {
    let package = package_source();
    let private = CapturedPrivateWorkspaceInputs::new(
        private_document("capabilities.toml", capabilities_source()),
        private_document("machines/machine.toml", machine_source()),
        private_document("distribution-lock.toml", distribution_lock_source()),
        Vec::new(),
    )
    .expect("interface conformance private inputs are bounded");
    let workspace = workspace_source();
    let machine = ConformanceMachine::new();
    let compilation = compile_captured_machine_verified_inputs(
        CapturedOneShotCompilationInputs::new(
            "demo".into(),
            "default".into(),
            "machine".into(),
            private,
            package,
        ),
        compile_portable_workspace(workspace).expect("captured workspace admits"),
        &machine,
        &machine,
    )
    .expect("captured interface conformance inputs compile");
    let profile = compilation.profile().clone();
    let materialization = applied_materialization(&compilation);
    (compilation, vec![materialization], profile)
}

pub(super) fn package_source() -> CapturedPackageSource {
    CapturedPackageSource::new(
        package_documents()
            .into_iter()
            .map(|(path, bytes)| package_document(path, bytes))
            .collect(),
    )
    .expect("interface conformance Package tree is bounded")
}

fn package_documents() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (
            "engine/package.toml",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/resident/invoke-conformance.v1/sources/packages/engine/package.toml"
            ))
            .to_vec(),
        ),
        (
            "engine/roles.toml",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/resident/invoke-conformance.v1/sources/packages/engine/roles.toml"
            ))
            .to_vec(),
        ),
        (
            "engine/role.md",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/resident/invoke-conformance.v1/sources/packages/engine/role.md"
            ))
            .to_vec(),
        ),
        (
            "engine/operations.toml",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/resident/invoke-conformance.v1/sources/packages/engine/operations.toml"
            ))
            .to_vec(),
        ),
        (
            "engine/policy.toml",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/resident/invoke-conformance.v1/sources/packages/engine/policy.toml"
            ))
            .to_vec(),
        ),
        (
            "engine/review.md",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/resident/invoke-conformance.v1/sources/packages/engine/review.md"
            ))
            .to_vec(),
        ),
    ]
}

fn capabilities_source() -> Vec<u8> {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/resident/invoke-conformance.v1/sources/private/capabilities.toml"
    ))
    .to_vec()
}

fn machine_source() -> Vec<u8> {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/resident/invoke-conformance.v1/sources/private/machine.toml"
    ))
    .to_vec()
}

pub(super) fn distribution_lock_source() -> Vec<u8> {
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/resident/invoke-conformance.v1/sources/private/distribution-lock.toml"
    ))
    .to_vec()
}

fn workspace_source() -> CapturedWorkspaceSource {
    let root = CapturedWorkspaceSource::admit_root_manifest("pale-workspace.toml".into())
        .expect("workspace root is confined");
    CapturedWorkspaceSource::new(
        root,
        workspace_documents()
            .into_iter()
            .map(|(path, bytes)| workspace_document(path, &bytes))
            .collect(),
    )
    .expect("captured workspace is complete")
}

fn workspace_documents() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (
            "pale-workspace.toml",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/resident/invoke-conformance.v1/sources/workspace/pale-workspace.toml"
            ))
            .to_vec(),
        ),
        (
            "composition.toml",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/resident/invoke-conformance.v1/sources/workspace/composition.toml"
            ))
            .to_vec(),
        ),
        (
            "workspace-resolution.lock.toml",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/resident/invoke-conformance.v1/sources/workspace/workspace-resolution.lock.toml"
            ))
            .to_vec(),
        ),
        (
            "workspace-universe.lock.toml",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/resident/invoke-conformance.v1/sources/workspace/workspace-universe.lock.toml"
            ))
            .to_vec(),
        ),
    ]
}

pub(in super::super) fn source_fixture_documents() -> Vec<(String, Vec<u8>)> {
    let mut documents = package_documents()
        .into_iter()
        .map(|(path, bytes)| (format!("sources/packages/{path}"), bytes))
        .collect::<Vec<_>>();
    documents.extend(
        workspace_documents()
            .into_iter()
            .map(|(path, bytes)| (format!("sources/workspace/{path}"), bytes)),
    );
    documents.extend([
        (
            "sources/private/capabilities.toml".to_owned(),
            capabilities_source(),
        ),
        ("sources/private/machine.toml".to_owned(), machine_source()),
        (
            "sources/private/distribution-lock.toml".to_owned(),
            distribution_lock_source(),
        ),
    ]);
    documents
}

fn workspace_document(label: &str, bytes: &[u8]) -> CapturedWorkspaceDocument {
    CapturedWorkspaceDocument::new(label.into(), bytes.to_vec())
        .expect("workspace document is bounded")
}

fn package_document(label: &str, bytes: Vec<u8>) -> CapturedPackageDocument {
    CapturedPackageDocument::new(label.into(), bytes).expect("Package document is bounded")
}

fn private_document(label: &str, bytes: Vec<u8>) -> CapturedPrivateDocument {
    CapturedPrivateDocument::new(label.into(), bytes).expect("private document is bounded")
}

fn applied_materialization(compilation: &Compilation) -> AppliedRepositoryMaterialization {
    let repository = compilation
        .repositories()
        .first()
        .expect("fixture has one repository");
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
                panic!("an absent baseline produces only create operations")
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
    .expect("materialization plan admits");
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
        target_root_sha256: TargetRootDigest::hash_bytes(b"/synthetic/interface-engine"),
        profile_digest: state.profile_digest.clone(),
        instance_digest: state.instance_digest.clone(),
        manifest_digest: state.manifest_digest.clone(),
        state_digest: state.state_digest.clone(),
        admitted_plan_digest: admitted_plan.digest().clone(),
    })
    .expect("materialization lineage seals");
    let receipt = build_success_receipt_with_lineage(
        &admitted_plan,
        operations,
        Vec::new(),
        MaterializationLineageEvidence::Initialized {
            post: lineage.lineage_digest.clone(),
        },
    )
    .expect("materialization receipt admits");
    AppliedRepositoryMaterialization::admit(
        receipt,
        state,
        lineage,
        RepositorySourceFrontierV1 {
            repository_id: repository.id().clone(),
            base_revision: RepositoryRevisionV1::parse("interface-base")
                .expect("repository revision"),
            tracked_delta_sha256: ArtifactDigest::hash_bytes(b"interface tracked delta"),
            untracked_inventory_sha256: ArtifactDigest::hash_bytes(b"interface untracked"),
            dependency_frontier_sha256: ArtifactDigest::hash_bytes(b"interface dependencies"),
        },
    )
    .expect("materialization evidence admits")
}
