//! Compile a workspace's configuration and the Packages it selects into the
//! guidance, hooks and manifests for each repository, reading without writing.
//!
//! This module builds mainly on `workspace`, `packages`, `semantic` and
//! `render`; `doctor`, `improvement`, `observation`, `plan`, `provider_invocation`
//! and `resident` build on its results.
//!
//! A compilation captures distribution bytes once. Manifest-directed Package
//! admission, body projection, and release digests all consume that retained set,
//! so later filesystem changes cannot split one result across observations.
//! Portable profile identity remains distinct from machine-backed activation
//! evidence.
//!
//! The three compilation postures share one semantic compiler but differ in
//! the evidence admitted at the machine boundary:
//!

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/compilation-postures.display.mathml"
))]
//!
//! In the rendered law, `S` is the retained source snapshot, `A` is shared
//! Package admission, `O` is fresh machine observation, and `D` is declared
//! removal reconstruction. Portable compilation supplies no machine evidence;
//! removal can reproduce cleanup inputs but cannot prove live readiness.
use std::collections::BTreeSet;
use std::sync::Arc;

use pale_core::records::{
    AdmittedLifecycleBehaviorV1, CapabilitiesFile, DistributionLockDigest, DistributionLockFile,
    EffectiveProjectProfileV1, ExecutionPathSelectionV1, LifecycleBehavior,
    LogicalExecutionWorkspaceV1, LogicalProjectV1, MachineBindingFile, PackageIdV1,
    ResolvedProjectInstanceV1, SchemaRegistryIdentity, WorkspaceId,
};

use crate::actions::NativeActionRegistry;
use crate::{packages, plan, render, semantic, workspace};

mod captured;
mod distribution;
mod errors;
mod inspection;
mod machine_resolution;
mod types;

use distribution::{compile_package_graph, select_package_graph, CompiledPackageGraph};
use machine_resolution::{
    resolve_machine, MachineResolution, MachineResolutionRequest, MachineVerification,
};

pub use crate::captured_input::{CapturedCompilationInputError, CapturedInputLimit};
pub use crate::composition::{CompositionError, ProviderBindingViolation};
pub use crate::workspace::{
    CapturedLifecycleGuidance, CapturedPrivateDocument, CapturedPrivateWorkspaceInputs,
};
pub use captured::{
    CapturedOneShotCompilationInputs, CapturedPackageDocument, CapturedPackageSource,
};
pub use errors::{CompilationInvariantError, CompileError};
pub use inspection::{
    CompilationInspection, EffectiveProfileExecutionPathInspection, EffectiveProfileFinding,
    EffectiveProfileInspection, EffectiveProfileNarrowingReason, EffectiveProfileNativeRealization,
    EffectiveProfileOperationExposure, EffectiveProfileOperationInspection,
    EffectiveProfilePolicyBlock, EffectiveProfileProviderDisposition,
    EffectiveProfileProviderRepositoryInspection, EffectiveProfileRepositoryInspection,
    EffectiveProfileRequirementDisposition, EffectiveProfileRequirementInspection,
    EffectiveProfileResourceKind, EffectiveProfileResponsibilityContractInspection,
    EffectiveProfileSelectedResource, RepositoryInspection,
};
pub(crate) use types::ResidentCompilationParts;
use types::{prepare_compilation_input, CompilationInputSources, CompilationParts};
pub use types::{Compilation, CompiledRepo};

/// Compatibility identity checked by package and distribution locks.
///
/// Package compatibility declarations and the canonical compilation-input
/// preimage consume this value. It identifies the initial-v1 surface contract,
/// not the executable's release version or its machine-local interface identity.
pub const DISTRIBUTION_COMPILER_ID: &str = "pale-surface@1";

struct RepositoryRenderBatch<'a> {
    identity: crate::identity::ProjectIdentity<'a>,
    experience_composition_plan: &'a pale_core::records::ExperienceCompositionPlanV1,
    read_only_paths: &'a [String],
    lifecycle: &'a [LifecycleBehavior],
    semantic_plans: Vec<semantic::RepositorySemanticPlan>,
    package_obligations: semantic::MechanicalObligationClosure,
    native_actions: &'a NativeActionRegistry,
    machine_admission: &'a render::MachineAdmission,
    schema_registry: &'a pale_core::records::SchemaRegistryIdentity,
    compilation_input_digest: &'a pale_core::records::CompilationInputDigest,
}

/// Render each repository's closed semantics and generated manifest.
fn render_repositories(
    batch: RepositoryRenderBatch<'_>,
) -> Result<Vec<CompiledRepo>, CompileError> {
    let RepositoryRenderBatch {
        identity,
        experience_composition_plan,
        read_only_paths,
        lifecycle,
        semantic_plans,
        package_obligations,
        native_actions,
        machine_admission,
        schema_registry,
        compilation_input_digest,
    } = batch;
    let profile = identity.profile_record();
    let mut compiled = Vec::new();
    if semantic_plans.len() != profile.repositories.len()
        || package_obligations.repositories.len() != profile.repositories.len()
    {
        return Err(CompilationInvariantError::structure(
            "repository pre-render closure",
            "semantic-plan or Package-obligation count differs from the effective repository count",
        )
        .into());
    }
    for (repository_index, ((repository, repository_semantics), package_obligations)) in profile
        .repositories
        .iter()
        .zip(semantic_plans)
        .zip(package_obligations.repositories)
        .enumerate()
    {
        let rendered = render::render_project_with_admission(
            render::ProjectRenderInput {
                schema_registry,
                compilation_input_digest,
                identity,
                experience_composition_plan,
                repository_index,
                repository,
                read_only_paths,
                lifecycle,
                semantics: &repository_semantics,
                package_obligations: &package_obligations,
            },
            machine_admission,
            native_actions,
        )?;
        let manifest = plan::build_manifest_with_identity(
            schema_registry,
            compilation_input_digest,
            &rendered,
            &repository.id,
            identity,
        )?;
        compiled.push(CompiledRepo {
            repo_id: repository.id.clone(),
            render: rendered,
            manifest,
            semantics: repository_semantics,
        });
    }
    Ok(compiled)
}

struct CompilationDeclarations {
    workspace_id: WorkspaceId,
    selected_topology_digest: String,
    project: LogicalProjectV1,
    execution_workspace: LogicalExecutionWorkspaceV1,
    execution_paths: Vec<ExecutionPathSelectionV1>,
    repositories: Vec<workspace::SelectedRepositoryMembership>,
    capabilities: CapabilitiesFile,
    lifecycle: Vec<AdmittedLifecycleBehaviorV1>,
    machine: MachineBindingFile,
    lock: DistributionLockFile,
}

/// Source-free application-owned result at the effective-profile compiler seam.
///
/// This value retains the exact admitted workspace owner, Package graph,
/// portable profile, private machine projection, repository semantic plans,
/// and composition proof produced by the canonical compiler. It deliberately
/// stops before machine observation, runtime rendering, one-shot sealing, resident
/// admission, activation, or native effect.
///
/// A [`workspace::PortableWorkspaceUniverse`] and this handle may coexist: both
/// share the same private admitted owner, so comparison and profile inspection
/// never require a second source parse. Shared ownership does not escape this
/// crate's public fields or the eventual `pale-sdk` projection.
pub struct EffectiveProfileCompilation {
    selected_topology_digest: String,
    profile: EffectiveProjectProfileV1,
    instance: ResolvedProjectInstanceV1,
    package_semantic_ir: packages::SealedPackageGraph,
    package_universe: Arc<packages::AdmittedPackageGraph>,
    package_seed_frontier: Vec<PackageIdV1>,
    package_authoring_sources: workspace::AuthoringSourceMap,
    package_authoring_semantics: workspace::AuthoringSemanticIndex,
    workspace_universe: Arc<workspace::PrincipalAdmittedUniverse>,
    experience_composition_plan: pale_core::records::ExperienceCompositionPlanV1,
    path_proofs: Vec<inspection::EffectiveProfilePathProof>,
    semantic_plans: Vec<semantic::RepositorySemanticPlan>,
    package_obligations: semantic::MechanicalObligationClosure,
    lifecycle: Vec<LifecycleBehavior>,
    schema_registry: SchemaRegistryIdentity,
    distribution_lock_digest: DistributionLockDigest,
    authoring_sources: workspace::AuthoringSourceMap,
    authoring_semantics: workspace::AuthoringSemanticIndex,
}

impl std::fmt::Debug for EffectiveProfileCompilation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EffectiveProfileCompilation")
            .field("selected_topology_digest", &self.selected_topology_digest)
            .field("profile_digest", &self.profile.profile_digest)
            .field("instance_digest", &self.instance.instance_digest)
            .field("repository_count", &self.semantic_plans.len())
            .field(
                "principal_admitted_universe_digest",
                self.workspace_universe.digest(),
            )
            .finish()
    }
}

impl EffectiveProfileCompilation {
    pub(crate) fn admitted_source_owner(&self) -> &Arc<workspace::PrincipalAdmittedUniverse> {
        &self.workspace_universe
    }

    /// Borrow the exact selected desired-topology identity.
    #[must_use]
    pub fn selected_topology_digest(&self) -> &str {
        &self.selected_topology_digest
    }

    /// Borrow compiler-selected source grammars for editor projections.
    ///
    /// The map is source-free and carries no admission, activation, or native
    /// readiness meaning.
    #[must_use]
    pub fn authoring_sources(&self) -> &workspace::AuthoringSourceMap {
        &self.authoring_sources
    }

    /// Borrow linker/compiler-owned source-free navigation evidence.
    #[must_use]
    pub fn authoring_semantics(&self) -> &workspace::AuthoringSemanticIndex {
        &self.authoring_semantics
    }

    /// Borrow the application-owned bounded inspection projection.
    ///
    /// The view derives exact operation, provider, repository, path, and proof
    /// dispositions from this retained phase. It owns no serialization or UI
    /// policy; outer façades may only copy these values into their boundary DTOs.
    #[must_use]
    pub fn inspection(&self) -> EffectiveProfileInspection<'_> {
        EffectiveProfileInspection::new(self)
    }

    /// Borrow the canonical portable profile derived from admitted declarations.
    #[must_use]
    pub fn profile(&self) -> &EffectiveProjectProfileV1 {
        &self.profile
    }

    /// Borrow the selected machine projection without implying observed readiness.
    ///
    /// This instance contains admitted private configuration only. Machine paths,
    /// executables, providers, and native state remain unproven at this phase.
    #[must_use]
    pub fn instance(&self) -> &ResolvedProjectInstanceV1 {
        &self.instance
    }

    /// Borrow the source-free Package graph that owns selected semantic meaning.
    #[must_use]
    pub fn package_semantic_ir(&self) -> &packages::SealedPackageGraph {
        &self.package_semantic_ir
    }

    /// Borrow repository semantic plans in effective-profile repository order.
    ///
    /// Each plan retains [`semantic::ResolvedOperationMechanics`], so unbound,
    /// semantic-only, and capability-bearing operations remain non-substitutable.
    #[must_use]
    pub fn repository_semantic_plans(&self) -> &[semantic::RepositorySemanticPlan] {
        &self.semantic_plans
    }

    /// Project selected Package judgment into source-guidance files for one host.
    ///
    /// This uses retained source admission only: it observes no machine, issues
    /// no provider generation and creates no runtime surface, plan or grant.
    /// The entrypoint is a proposed contribution, never a replacement for local
    /// project instructions. A native caller must separately plan and qualify
    /// any write, including ownership and fresh target baselines.
    ///
    /// # Errors
    ///
    /// Rejects an unselected repository or selections requiring an operation,
    /// recipe, mode or lifecycle gate. Such duties need the admitted runtime
    /// path; this projection must not silently translate them into prose.
    pub fn source_guidance(
        &self,
        repository_id: &pale_core::records::LogicalRepositoryIdV1,
        host: render::SourceGuidanceHost,
    ) -> Result<render::SourceGuidanceProjection, render::SourceGuidanceError> {
        render::render_source_guidance(self, repository_id, host)
    }

    /// Render private context for one selected repository from retained declarations.
    ///
    /// The output includes configured machine paths and must remain within the
    /// project's disclosure scope. It is separate from portable repository
    /// surfaces, performs no source reads or writes, and grants no host admission.
    ///
    /// # Errors
    ///
    /// Rejects an unselected repository, an unbound reading root, or required
    /// reading without an explicit read-only authority-access declaration.
    pub fn repository_context(
        &self,
        repository_id: &pale_core::records::LogicalRepositoryIdV1,
    ) -> Result<String, render::SourceGuidanceError> {
        render::render_repository_context(self, repository_id)
    }

    /// Borrow the mechanical Package closure that every renderer must consume.
    ///
    /// The closure is produced from the admitted semantic plans before any
    /// Markdown, JSON, or native-harness projection is rendered. Provider
    /// absence closes only the affected requirement or operation.
    #[must_use]
    pub fn package_obligations(&self) -> &semantic::MechanicalObligationClosure {
        &self.package_obligations
    }

    /// Borrow the compiler-owned neutral composition proof and its reason paths.
    #[must_use]
    pub fn experience_composition_plan(&self) -> &pale_core::records::ExperienceCompositionPlanV1 {
        &self.experience_composition_plan
    }

    /// Borrow the exact principal-relative workspace admission coordinate.
    #[must_use]
    pub fn principal_admitted_universe_digest(
        &self,
    ) -> &pale_core::records::PrincipalAdmittedUniverseDigest {
        self.workspace_universe.digest()
    }

    /// Borrow the verified workspace semantic-IR coordinate.
    #[must_use]
    pub fn workspace_semantic_ir_digest(&self) -> &pale_core::records::WorkspaceSemanticIrDigest {
        self.workspace_universe.workspace_semantic_ir_digest()
    }

    /// Borrow the exact owner-proof set admitted with the workspace universe.
    #[must_use]
    pub fn workspace_admission_evidence_digest(
        &self,
    ) -> &pale_core::records::WorkspaceAdmissionEvidenceDigest {
        self.workspace_universe.admission_evidence_digest()
    }

    /// Borrow the schema registry identity checked against the distribution lock.
    #[must_use]
    pub fn schema_registry_identity(&self) -> &SchemaRegistryIdentity {
        &self.schema_registry
    }

    /// Borrow the exact admitted distribution-lock identity.
    #[must_use]
    pub fn distribution_lock_digest(&self) -> &DistributionLockDigest {
        &self.distribution_lock_digest
    }

    pub(crate) fn package_universe(&self) -> &packages::AdmittedPackageGraph {
        &self.package_universe
    }
}

fn compile_admitted_effective_profile(
    declarations: CompilationDeclarations,
    workspace_universe: Arc<workspace::PrincipalAdmittedUniverse>,
    authoring_sources: workspace::AuthoringSourceMap,
    authoring_semantics: workspace::AuthoringSemanticIndex,
    snapshot: packages::DistributionSnapshot,
) -> Result<EffectiveProfileCompilation, CompileError> {
    let CompilationDeclarations {
        workspace_id,
        selected_topology_digest,
        project,
        execution_workspace,
        execution_paths,
        repositories,
        capabilities,
        lifecycle: admitted_lifecycle,
        machine,
        lock,
    } = declarations;
    let registry = pale_contracts::schema_registry_identity()?;
    let pinned_registry = SchemaRegistryIdentity {
        schema_manifest_sha256: lock.distribution.schema_manifest_sha256.clone(),
        schema_set_sha256: lock.distribution.schema_set_sha256.clone(),
    };
    if pinned_registry != registry {
        return Err(CompileError::SchemaRegistryMismatch {
            pinned: pinned_registry,
            running: registry,
        });
    }
    let distribution_lock_digest = DistributionLockDigest::from_lock(&lock).map_err(|source| {
        CompilationInvariantError::structure("distribution lock identity", source.to_string())
    })?;
    let packages = compile_package_graph(&snapshot, &lock, &project, &repositories, &capabilities)?;
    // Source custody ends here. Every later stage consumes only typed, sealed,
    // content-bearing values.
    drop(snapshot);
    finish_effective_profile(
        CompilationDeclarations {
            workspace_id,
            selected_topology_digest,
            project,
            execution_workspace,
            execution_paths,
            repositories,
            capabilities,
            lifecycle: admitted_lifecycle,
            machine,
            lock,
        },
        workspace_universe,
        authoring_sources,
        authoring_semantics,
        packages,
        registry,
        distribution_lock_digest,
    )
}

fn finish_effective_profile(
    declarations: CompilationDeclarations,
    workspace_universe: Arc<workspace::PrincipalAdmittedUniverse>,
    mut authoring_sources: workspace::AuthoringSourceMap,
    mut authoring_semantics: workspace::AuthoringSemanticIndex,
    packages: CompiledPackageGraph,
    registry: SchemaRegistryIdentity,
    distribution_lock_digest: DistributionLockDigest,
) -> Result<EffectiveProfileCompilation, CompileError> {
    let CompilationDeclarations {
        workspace_id,
        selected_topology_digest,
        project,
        execution_workspace,
        execution_paths,
        repositories,
        capabilities,
        lifecycle: admitted_lifecycle,
        machine,
        lock: _,
    } = declarations;
    let package_seed_frontier = project.package_seeds.clone();
    let CompiledPackageGraph {
        graph: package_semantic_ir,
        universe: package_universe,
        authoring_sources: package_authoring_sources,
        authoring_semantics: package_authoring_semantics,
    } = packages;
    let retained_package_authoring_sources = package_authoring_sources.clone();
    let retained_package_authoring_semantics = package_authoring_semantics.clone();
    authoring_sources
        .extend(package_authoring_sources)
        .map_err(|label| {
            CompilationInvariantError::structure(
                "authoring-source custody",
                format!("logical source label `{label}` belongs to more than one compiler role"),
            )
        })?;
    authoring_semantics
        .extend(package_authoring_semantics)
        .map_err(|error| {
            CompilationInvariantError::structure("authoring-semantic custody", error.to_string())
        })?;
    let profile = workspace::compute_effective_profile(workspace::EffectiveProfileInput {
        workspace_id,
        project: &project,
        execution_workspace: &execution_workspace,
        execution_paths: &execution_paths,
        repositories: &repositories,
        packages: &package_semantic_ir,
        normative_pack_closure: workspace_universe.normative_pack_releases(),
        lifecycle: admitted_lifecycle,
        capabilities: &capabilities,
    })?;
    let experience_composition_plan = crate::composition::compile_current_composition_plan(
        &profile,
        workspace_universe.experience_composition().clone(),
    )
    .map_err(|failure| match failure {
        crate::composition::CompositionPlanFailure::Declaration(source) => {
            CompileError::CompositionInput(Box::new(source))
        }
        crate::composition::CompositionPlanFailure::GeneratedOutput(source) => {
            CompilationInvariantError::Composition(source).into()
        }
    })?;
    let instance = workspace::resolve_instance(&profile, &machine, &experience_composition_plan)?;
    let semantic_plans = semantic::project_repository_semantic_plans(
        &profile,
        &package_semantic_ir,
        &instance.user_mode_defaults,
    )?;
    if semantic_plans.len() != profile.repositories.len() {
        return Err(CompilationInvariantError::structure(
            "effective-profile repository semantics",
            format!(
                "profile carries {} repositories but semantic projection produced {} plans",
                profile.repositories.len(),
                semantic_plans.len()
            ),
        )
        .into());
    }
    let package_obligations =
        semantic::compile_package_obligations(&semantic_plans).map_err(|source| {
            CompilationInvariantError::structure(
                "Package mechanical obligation closure",
                source.to_string(),
            )
        })?;
    let lifecycle = crate::lifecycle::mechanics_set(&profile.lifecycle);
    let path_proofs =
        inspection::build_effective_profile_path_proofs(&profile, &experience_composition_plan)?;
    Ok(EffectiveProfileCompilation {
        selected_topology_digest,
        profile,
        instance,
        package_semantic_ir,
        package_universe,
        package_seed_frontier,
        package_authoring_sources: retained_package_authoring_sources,
        package_authoring_semantics: retained_package_authoring_semantics,
        workspace_universe,
        experience_composition_plan,
        path_proofs,
        semantic_plans,
        package_obligations,
        lifecycle,
        schema_registry: registry,
        distribution_lock_digest,
        authoring_sources,
        authoring_semantics,
    })
}

fn finish_admitted_compilation(
    effective: EffectiveProfileCompilation,
    verification: MachineVerification<'_>,
) -> Result<Compilation, CompileError> {
    // Profile inspection accepts an unfinished selection. Host rendering needs
    // a path, so reject the declaration before machine checks or projection.
    if effective.profile.execution_paths.is_empty() {
        return Err(CompileError::MissingExecutionPaths {
            execution_workspace: effective.profile.execution_workspace_id.clone(),
        });
    }
    let EffectiveProfileCompilation {
        selected_topology_digest: _,
        profile,
        instance,
        package_semantic_ir,
        package_universe: _,
        package_seed_frontier: _,
        package_authoring_sources: _,
        package_authoring_semantics: _,
        workspace_universe,
        experience_composition_plan,
        path_proofs: _,
        semantic_plans,
        package_obligations,
        lifecycle,
        schema_registry: registry,
        distribution_lock_digest,
        authoring_sources: _,
        authoring_semantics: _,
    } = effective;
    let repository_verifier_capabilities = semantic_plans
        .iter()
        .flat_map(|plan| plan.repository_verifier_capabilities.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let MachineResolution {
        read_only_paths,
        machine_admission,
    } = resolve_machine(MachineResolutionRequest {
        profile: &profile,
        instance: &instance,
        composition_plan: &experience_composition_plan,
        policy: &profile.run_policy,
        behaviours: &lifecycle,
        repository_verifier_capabilities: &repository_verifier_capabilities,
        verification,
    })?;
    let native_actions = NativeActionRegistry::new()
        .map_err(|error| CompileError::NativeActionRegistry(Box::new(error)))?;
    let executable_identity_digest = machine_admission.executable_identity_digest().cloned();
    let prepared = prepare_compilation_input(CompilationInputSources {
        schema_registry: &registry,
        distribution_lock_digest: &distribution_lock_digest,
        package_closure: &profile.package_closure,
        profile: &profile,
        instance: &instance,
        package_semantic_ir_digest: package_semantic_ir.digest(),
        workspace_universe: workspace_universe.as_ref(),
        experience_composition_plan: &experience_composition_plan,
        executable_identity_digest: executable_identity_digest.as_ref(),
        mode: verification.mode(),
    })?;
    let compilation_input_digest = prepared.digest;
    let repos = render_repositories(RepositoryRenderBatch {
        identity: prepared.identity,
        experience_composition_plan: &experience_composition_plan,
        read_only_paths: &read_only_paths,
        lifecycle: &lifecycle,
        semantic_plans,
        package_obligations,
        native_actions: &native_actions,
        machine_admission: &machine_admission,
        schema_registry: &registry,
        compilation_input_digest: &compilation_input_digest,
    })?;

    let package_closure = profile.package_closure.clone();
    let workspace_universe = Arc::try_unwrap(workspace_universe).map_err(|_| {
        CompilationInvariantError::structure(
            "effective-profile workspace custody",
            "one-shot completion retained a second admitted workspace owner",
        )
    })?;
    Compilation::admit(CompilationParts {
        profile,
        instance,
        package_semantic_ir,
        workspace_universe,
        experience_composition_plan,
        repos,
        schema_registry: registry,
        package_closure,
        mode: verification.mode(),
        distribution_lock_digest,
        executable_identity_digest,
        compilation_input_digest,
    })
    .map_err(CompileError::from)
}

fn compile_input_snapshot(
    inputs: workspace::WorkspaceInputSnapshot,
    verification: MachineVerification<'_>,
    sources: &dyn pale_ports::SourceFilesystemPort,
) -> Result<Compilation, CompileError> {
    let workspace::WorkspaceInputSnapshot {
        distribution_root,
        workspace_id,
        project,
        execution_workspace,
        execution_paths,
        repositories,
        capabilities,
        lifecycle,
        machine,
        lock,
        workspace_source,
    } = inputs;
    let selected_topology_digest = workspace::selected_topology_digest_from_parts(
        &project,
        &execution_workspace,
        &execution_paths,
        &repositories,
    )?;
    let authoring_sources = workspace::AuthoringSourceMap::from_workspace(&workspace_source)?;
    let (workspace_universe, authoring_semantics) =
        workspace::admit_workspace_source_with_authoring(workspace_source)?;
    let workspace_universe = Arc::new(workspace_universe);
    let snapshot = packages::DistributionSnapshot::capture(sources, &distribution_root)?;
    let effective = compile_admitted_effective_profile(
        CompilationDeclarations {
            workspace_id,
            selected_topology_digest,
            project,
            execution_workspace,
            execution_paths,
            repositories,
            capabilities,
            lifecycle,
            machine,
            lock,
        },
        workspace_universe,
        authoring_sources,
        authoring_semantics,
        snapshot,
    )?;
    finish_admitted_compilation(effective, verification)
}

/// Compile an inspectable effective profile from one admitted workspace owner.
///
/// The workspace is borrowed so its portable comparison handle and this
/// returned compiler phase share the exact admitted semantic owner. Private and
/// Package source bytes are consumed once. The result has no machine-observation
/// evidence, runtime surface, resident identity, activation, or native effect.
///
/// # Errors
///
/// Returns [`CompileError`] when topology selection, private-record admission,
/// Package admission, profile construction, instance projection, semantic
/// planning, or composition proof rejects the supplied inputs.
pub fn compile_captured_effective_profile_inputs(
    inputs: CapturedOneShotCompilationInputs,
    workspace_universe: &workspace::PortableWorkspaceUniverse,
) -> Result<EffectiveProfileCompilation, CompileError> {
    let CapturedOneShotCompilationInputs {
        project_id,
        execution_workspace_id,
        machine_id,
        private,
        packages,
    } = inputs;
    let admitted = admit_captured_effective_profile_inputs(
        project_id,
        execution_workspace_id,
        machine_id,
        private,
        workspace_universe,
    )?;
    compile_admitted_effective_profile(
        admitted.declarations,
        admitted.workspace_universe,
        admitted.authoring_sources,
        admitted.authoring_semantics,
        packages.into_snapshot(),
    )
}

struct CapturedEffectiveProfileAdmission {
    declarations: CompilationDeclarations,
    workspace_universe: Arc<workspace::PrincipalAdmittedUniverse>,
    authoring_sources: workspace::AuthoringSourceMap,
    authoring_semantics: workspace::AuthoringSemanticIndex,
}

fn admit_captured_effective_profile_inputs(
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    private: CapturedPrivateWorkspaceInputs,
    workspace_universe: &workspace::PortableWorkspaceUniverse,
) -> Result<CapturedEffectiveProfileAdmission, CompileError> {
    let mut authoring_sources = workspace_universe.authoring_sources().clone();
    let mut authoring_semantics = workspace_universe.authoring_semantics().clone();
    let (workspace_universe, workspace_id, selection) =
        workspace_universe.shared_compilation_parts(&project_id, &execution_workspace_id)?;
    let selected_topology_digest = workspace::selected_topology_digest(&selection)?;
    let private = match workspace::admit_captured_private_inputs(selection, &machine_id, private) {
        Ok(private) => private,
        Err(workspace::CapturedPrivateAdmissionError::Captured(source)) => {
            return Err(CompileError::from(source));
        }
        Err(workspace::CapturedPrivateAdmissionError::Workspace(source)) => {
            return Err(CompileError::from(source));
        }
    };
    let workspace::CapturedPrivateRecords {
        project,
        execution_workspace,
        execution_paths,
        repositories,
        capabilities,
        lifecycle,
        machine,
        lock,
        authoring_sources: private_authoring_sources,
        authoring_semantics: private_authoring_semantics,
    } = private;
    authoring_sources
        .extend(private_authoring_sources)
        .map_err(|label| {
            CompilationInvariantError::structure(
                "authoring-source custody",
                format!("logical source label `{label}` belongs to more than one compiler role"),
            )
        })?;
    authoring_semantics
        .extend(private_authoring_semantics)
        .map_err(|error| {
            CompilationInvariantError::structure("authoring-semantic custody", error.to_string())
        })?;
    Ok(CapturedEffectiveProfileAdmission {
        declarations: CompilationDeclarations {
            workspace_id,
            selected_topology_digest,
            project,
            execution_workspace,
            execution_paths,
            repositories,
            capabilities,
            lifecycle,
            machine,
            lock,
        },
        workspace_universe,
        authoring_sources,
        authoring_semantics,
    })
}

/// Why a configuration candidate could not reuse an admitted Package frontier.
pub(crate) enum EffectiveProfileCandidateError {
    /// Canonical workspace/private/profile compilation rejected the candidate.
    Compile(CompileError),
    /// The candidate selected another immutable distribution lock.
    DistributionFrontier,
    /// The candidate selected another Package release seed frontier.
    PackageReleaseFrontier,
}

impl From<CompileError> for EffectiveProfileCandidateError {
    fn from(source: CompileError) -> Self {
        Self::Compile(source)
    }
}

/// Compile a full configuration candidate against source-free Package semantics.
///
/// The candidate may change portable and private Pale Workspace declarations within
/// the exact distribution and release frontier admitted by `baseline`. A
/// different Package frontier requires separate Package-distribution admission;
/// this route never retains or reloads dropped Package source.
pub(crate) fn compile_captured_effective_profile_candidate_inputs(
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    private: CapturedPrivateWorkspaceInputs,
    workspace_universe: &workspace::PortableWorkspaceUniverse,
    baseline: &EffectiveProfileCompilation,
) -> Result<EffectiveProfileCompilation, EffectiveProfileCandidateError> {
    let admitted = admit_captured_effective_profile_inputs(
        project_id,
        execution_workspace_id,
        machine_id,
        private,
        workspace_universe,
    )?;
    let lock_digest =
        DistributionLockDigest::from_lock(&admitted.declarations.lock).map_err(|source| {
            EffectiveProfileCandidateError::Compile(
                CompilationInvariantError::structure(
                    "distribution lock identity",
                    source.to_string(),
                )
                .into(),
            )
        })?;
    if &lock_digest != baseline.distribution_lock_digest() {
        return Err(EffectiveProfileCandidateError::DistributionFrontier);
    }
    if admitted.declarations.project.package_seeds != baseline.package_seed_frontier {
        return Err(EffectiveProfileCandidateError::PackageReleaseFrontier);
    }
    let registry = pale_contracts::schema_registry_identity()
        .map_err(CompileError::from)
        .map_err(EffectiveProfileCandidateError::Compile)?;
    let graph = select_package_graph(
        baseline.package_universe(),
        &admitted.declarations.repositories,
        &admitted.declarations.capabilities,
    )?;
    finish_effective_profile(
        admitted.declarations,
        admitted.workspace_universe,
        admitted.authoring_sources,
        admitted.authoring_semantics,
        CompiledPackageGraph {
            graph,
            universe: Arc::clone(&baseline.package_universe),
            authoring_sources: baseline.package_authoring_sources.clone(),
            authoring_semantics: baseline.package_authoring_semantics.clone(),
        },
        registry,
        lock_digest,
    )
    .map_err(EffectiveProfileCandidateError::Compile)
}

/// Compile owned private and Package documents with an admitted workspace universe.
///
/// This path-free boundary reuses the same typed private loaders, Package graph
/// compiler, profile/instance resolver, semantic planner, renderer, and seal as
/// [`compile_portable_inputs`]. It always uses the deferred machine mode: a
/// successful result proves one-shot semantic compilation, never live machine
/// readiness, resident-generation admission, activation, or native effect.
/// Source-shaped private and Package bytes are consumed before the returned
/// [`Compilation`] is sealed.
///
/// # Errors
///
/// Returns [`CompileError`] when selectors contradict their typed private
/// records, lifecycle guidance is not an exact unique-label projection,
/// Package admission fails, or any downstream compiler invariant rejects.
pub fn compile_captured_portable_inputs(
    inputs: CapturedOneShotCompilationInputs,
    workspace_universe: workspace::PortableWorkspaceUniverse,
) -> Result<Compilation, CompileError> {
    let effective = compile_captured_effective_profile_inputs(inputs, &workspace_universe)?;
    drop(workspace_universe);
    finish_admitted_compilation(effective, MachineVerification::Deferred)
}

/// Compile captured inputs with development-supplied machine observations.
///
/// This feature-only route preserves path-free source custody while exercising
/// the ordinary executable-identity and machine-resolution owners. It exists
/// only for cross-interface conformance tests and establishes no production
/// executable, filesystem, installation, or readiness claim.
#[cfg(feature = "interface-test-support")]
pub(crate) fn compile_captured_machine_verified_inputs(
    inputs: CapturedOneShotCompilationInputs,
    workspace_universe: workspace::PortableWorkspaceUniverse,
    filesystem: &dyn pale_ports::MachineFilesystemPort,
    processes: &dyn pale_ports::ProcessProbePort,
) -> Result<Compilation, CompileError> {
    let effective = compile_captured_effective_profile_inputs(inputs, &workspace_universe)?;
    drop(workspace_universe);
    finish_admitted_compilation(
        effective,
        MachineVerification::Required {
            filesystem,
            processes,
        },
    )
}

/// Compile one canonical workspace snapshot with live machine verification.
///
/// # Errors
///
/// Returns [`CompileError`] when the captured workspace is incoherent, machine
/// activation cannot be proven, or semantic projection, rendering, or manifest
/// construction fails.
pub fn compile_live_inputs(
    inputs: workspace::WorkspaceInputSnapshot,
    sources: &dyn pale_ports::SourceFilesystemPort,
    filesystem: &dyn pale_ports::MachineFilesystemPort,
    processes: &dyn pale_ports::ProcessProbePort,
) -> Result<Compilation, CompileError> {
    compile_input_snapshot(
        inputs,
        MachineVerification::Required {
            filesystem,
            processes,
        },
        sources,
    )
}

/// Compile one canonical workspace snapshot without claiming machine readiness.
///
/// This entry point is suitable for deterministic inspection and fixtures whose
/// machine paths intentionally do not exist. Its route-support rows always keep
/// execution and automatic activation unready.
///
/// # Errors
///
/// Returns [`CompileError`] when portable inputs are unreadable or incoherent,
/// or when semantic projection, rendering, or manifest construction fails.
pub fn compile_portable_inputs(
    inputs: workspace::WorkspaceInputSnapshot,
    sources: &dyn pale_ports::SourceFilesystemPort,
) -> Result<Compilation, CompileError> {
    compile_input_snapshot(inputs, MachineVerification::Deferred, sources)
}

/// Reconstruct removal plans from one already-captured workspace snapshot.
///
/// # Errors
///
/// Returns [`CompileError`] when the captured inputs cannot reproduce a coherent
/// materialization for removal.
pub fn reconstruct_removal_inputs(
    inputs: workspace::WorkspaceInputSnapshot,
    sources: &dyn pale_ports::SourceFilesystemPort,
) -> Result<Compilation, CompileError> {
    compile_input_snapshot(inputs, MachineVerification::ReconstructRemoval, sources)
}
