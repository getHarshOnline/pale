//! Compile-pipeline outputs and the canonical seals that bind their contents.

use std::collections::BTreeMap;

use pale_core::records::{
    CompilationInputDigest, DistributionLockDigest, EffectiveProjectProfileV1,
    ExecutableIdentityDigest, ExecutionPathSelectionIdV1, ExperienceCompositionPlanDigest,
    ExperienceCompositionPlanV1, ExperienceCompositionSourceDigest,
    GeneratedAgentSurfaceManifestV1, InstanceDigest, LogicalRepositoryIdV1, MachineOverlayDigest,
    ManifestDigest, NormativePackReleaseV1, OneShotCompilationDigest,
    OrganizationalTranslationClosureDigest, PackageReleaseV1, PackageSemanticIrDigest,
    PrincipalAdmittedUniverseDigest, ProfileDigest, ProviderBindingClosureDigest,
    RepositoryRenderDigest, ResolvedProjectInstanceV1, ResponsibilityDerivationClosureDigest,
    SchemaRegistryIdentity, SurfaceOwnership, WorkspaceAdmissionEvidenceDigest,
    WorkspaceAdoptedReleaseV1, WorkspaceDerivationNodeV1, WorkspaceDerivationReceiptV1,
    WorkspaceDerivationSourceV1, WorkspaceResolutionLockDigest, WorkspaceSemanticIrDigest,
    WorkspaceUniverseLockDigest,
};
use serde::Serialize;

use crate::packages::SealedPackageGraph;
use crate::render::ProjectRender;
use crate::semantic::RepositorySemanticPlan;
use crate::workspace::{
    PrincipalAdmittedUniverse, ResidentWorkspaceDerivation, ResidentWorkspaceParts,
};

use super::CompilationInvariantError;

// Domain-separated canonical preimages: none is a standalone wire schema.
/// Domain tag for the complete one-shot compiler input preimage.
const COMPILATION_INPUT_SCHEMA: &str = "pale.one-shot-compilation-input.v1";
/// Domain tag for one repository's semantic render preimage.
const REPOSITORY_RENDER_SCHEMA: &str = "pale.repository-render.v1";
/// Domain tag for the aggregate one-shot compilation preimage.
const ONE_SHOT_COMPILATION_SCHEMA: &str = "pale.one-shot-compilation-content.v1";

/// Repository projection paired with the manifest that seals its surface set.
///
/// Values are produced only as part of [`Compilation::repositories`]. The
/// [`Self::semantic_plan`] is the input to [`Self::render`], while
/// [`Self::manifest`] seals that render's materializable surfaces.
#[derive(Debug)]
pub struct CompiledRepo {
    /// Canonical logical repository identity whose projection was compiled.
    pub(super) repo_id: LogicalRepositoryIdV1,
    /// Complete rendered surface set and qualified execution-path rows.
    ///
    /// Path support reports the exact compile mode; callers must not infer
    /// terminal-machine readiness from the presence of rendered bytes.
    pub(super) render: ProjectRender,
    /// Self-consistent manifest sealing every materializable surface in [`Self::render`].
    pub(super) manifest: GeneratedAgentSurfaceManifestV1,
    /// Exact repository semantic plan consumed by [`Self::render`].
    pub(super) semantics: RepositorySemanticPlan,
}

impl CompiledRepo {
    /// Canonical logical repository identity whose projection was compiled.
    pub fn id(&self) -> &LogicalRepositoryIdV1 {
        &self.repo_id
    }

    /// Complete rendered surface set and qualified execution-path rows.
    pub fn render(&self) -> &ProjectRender {
        &self.render
    }

    /// Self-consistent manifest sealing every materializable surface.
    pub fn manifest(&self) -> &GeneratedAgentSurfaceManifestV1 {
        &self.manifest
    }

    /// Exact repository semantic plan consumed by rendering.
    pub fn semantic_plan(&self) -> &RepositorySemanticPlan {
        &self.semantics
    }
}

/// Closed mode of the current one-shot compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CompilationMode {
    Live,
    Portable,
    Removal,
}

/// Canonical project and execution-workspace semantics with repository projections.
///
/// All distribution-derived fields come from one retained snapshot. Repository
/// results preserve the profile's validated materialization order; construction
/// alone does not imply machine activation readiness. [`Self::package_semantic_ir`],
/// [`Self::experience_composition_plan`], and [`Self::repositories`] are bound by
/// [`Self::compilation_input_digest`] and [`Self::one_shot_compilation_digest`].
#[derive(Debug)]
pub struct Compilation {
    /// Portable project and execution-workspace profile resolved from configuration.
    pub(super) profile: EffectiveProjectProfileV1,
    /// Machine-specific binding resolved separately from portable profile identity.
    pub(super) instance: ResolvedProjectInstanceV1,
    /// Selection-bounded Package semantic graph. Repository projections are
    /// derived from this graph and the portable profile, never from ad hoc
    /// renderer lookups.
    pub(super) package_semantic_ir: SealedPackageGraph,
    /// Principal-relative workspace semantics admitted before source custody ends.
    pub(super) workspace_universe: PrincipalAdmittedUniverse,
    /// Compiler-derived neutral v1 experience composition plan.
    pub(super) experience_composition_plan: ExperienceCompositionPlanV1,
    /// Repository projections in the profile's validated materialization order.
    pub(super) repos: Vec<CompiledRepo>,
    seal: CompilationSeal,
}

/// Source-free compiler bodies consumed by one resident-definition build.
///
/// This transition value is never retained by a generation. Its fields are
/// moved into the aligned definition arena, after which the one-shot
/// [`Compilation`] wrapper and private seal are dropped.
pub(crate) struct ResidentCompilationParts {
    pub(crate) profile: EffectiveProjectProfileV1,
    pub(crate) instance: ResolvedProjectInstanceV1,
    pub(crate) package_semantic_ir: SealedPackageGraph,
    pub(crate) workspace: ResidentWorkspaceParts,
    pub(crate) experience_composition_plan: ExperienceCompositionPlanV1,
    pub(crate) repositories: Vec<CompiledRepo>,
}

/// Complete compiler output before its one-shot content identity is sealed.
pub(super) struct CompilationParts {
    pub(super) profile: EffectiveProjectProfileV1,
    pub(super) instance: ResolvedProjectInstanceV1,
    pub(super) package_semantic_ir: SealedPackageGraph,
    pub(super) workspace_universe: PrincipalAdmittedUniverse,
    pub(super) experience_composition_plan: ExperienceCompositionPlanV1,
    pub(super) repos: Vec<CompiledRepo>,
    pub(super) schema_registry: SchemaRegistryIdentity,
    pub(super) package_closure: Vec<PackageReleaseV1>,
    pub(super) mode: CompilationMode,
    pub(super) distribution_lock_digest: DistributionLockDigest,
    pub(super) executable_identity_digest: Option<ExecutableIdentityDigest>,
    pub(super) compilation_input_digest: CompilationInputDigest,
}

#[derive(Debug)]
struct CompilationSeal {
    profile_digest: ProfileDigest,
    instance_digest: InstanceDigest,
    machine_overlay_digest: MachineOverlayDigest,
    provider_binding_closure_digest: ProviderBindingClosureDigest,
    schema_registry: SchemaRegistryIdentity,
    package_closure: Vec<PackageReleaseV1>,
    mode: CompilationMode,
    distribution_lock_digest: DistributionLockDigest,
    executable_identity_digest: Option<ExecutableIdentityDigest>,
    compilation_input_digest: CompilationInputDigest,
    one_shot_compilation_digest: OneShotCompilationDigest,
    repositories: BTreeMap<LogicalRepositoryIdV1, RepositoryClosure>,
}

#[derive(Serialize)]
struct CompilationInputPreimage<'a> {
    schema: &'static str,
    compiler_contract_id: &'static str,
    renderer_contract_id: &'static str,
    schema_registry: &'a SchemaRegistryIdentity,
    distribution_lock_digest: &'a DistributionLockDigest,
    packages: &'a [PackageReleaseV1],
    profile_digest: &'a ProfileDigest,
    instance_digest: &'a InstanceDigest,
    package_semantic_ir_digest: &'a PackageSemanticIrDigest,
    workspace_semantic_ir_digest: &'a WorkspaceSemanticIrDigest,
    workspace_admission_evidence_digest: &'a WorkspaceAdmissionEvidenceDigest,
    principal_admitted_universe_digest: &'a PrincipalAdmittedUniverseDigest,
    workspace_resolution_lock_digest: &'a WorkspaceResolutionLockDigest,
    workspace_universe_lock_digest: &'a WorkspaceUniverseLockDigest,
    responsibility_derivation_closure_digest: &'a ResponsibilityDerivationClosureDigest,
    organizational_translation_closure_digest: &'a OrganizationalTranslationClosureDigest,
    experience_composition_source_digest: &'a ExperienceCompositionSourceDigest,
    provider_binding_closure_digest: &'a ProviderBindingClosureDigest,
    machine_overlay_digest: &'a MachineOverlayDigest,
    executable_identity_digest: Option<&'a ExecutableIdentityDigest>,
    workspace_module_releases: &'a [WorkspaceAdoptedReleaseV1],
    workspace_package_releases: &'a [PackageReleaseV1],
    workspace_normative_pack_releases: &'a [NormativePackReleaseV1],
    experience_composition_plan_digest: &'a ExperienceCompositionPlanDigest,
    mode: CompilationMode,
}

/// Exact admitted coordinates that form the compilation-input digest.
pub(super) struct CompilationInputSources<'a> {
    pub(super) schema_registry: &'a SchemaRegistryIdentity,
    pub(super) distribution_lock_digest: &'a DistributionLockDigest,
    pub(super) package_closure: &'a [PackageReleaseV1],
    pub(super) profile: &'a EffectiveProjectProfileV1,
    pub(super) instance: &'a ResolvedProjectInstanceV1,
    pub(super) package_semantic_ir_digest: &'a PackageSemanticIrDigest,
    pub(super) workspace_universe: &'a PrincipalAdmittedUniverse,
    pub(super) experience_composition_plan: &'a ExperienceCompositionPlanV1,
    pub(super) executable_identity_digest: Option<&'a ExecutableIdentityDigest>,
    pub(super) mode: CompilationMode,
}

/// Recomputed identities that bind one complete compilation-input preimage.
pub(super) struct PreparedCompilationInput<'a> {
    pub(super) identity: crate::identity::ProjectIdentity<'a>,
    machine_overlay_digest: MachineOverlayDigest,
    provider_binding_closure_digest: ProviderBindingClosureDigest,
    pub(super) digest: CompilationInputDigest,
}

#[derive(Serialize)]
struct RenderedSurfacePreimage<'a> {
    rel_path: &'a str,
    kind: &'a str,
    ownership: SurfaceOwnership,
    content: &'a str,
    execution_path_consumers: Vec<&'a ExecutionPathSelectionIdV1>,
    referenced_policy_ids: Vec<&'a str>,
    committable: bool,
    deferred: bool,
    target_note: Option<&'a str>,
}

#[derive(Serialize)]
struct RepositoryRenderPreimage<'a> {
    schema: &'static str,
    repository_id: &'a LogicalRepositoryIdV1,
    surfaces: Vec<RenderedSurfacePreimage<'a>>,
    execution_path_support: Vec<crate::render::ExecutionPathSupportSeal<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RepositoryClosure {
    repository_id: LogicalRepositoryIdV1,
    manifest_digest: ManifestDigest,
    render_digest: RepositoryRenderDigest,
}

#[derive(Serialize)]
struct OneShotCompilationPreimage<'a> {
    schema: &'static str,
    compilation_input_digest: &'a CompilationInputDigest,
    repositories: Vec<&'a RepositoryClosure>,
}

impl Compilation {
    /// Borrow application-owned facts for outer inspection projections.
    ///
    /// The returned view owns neither serialization nor presentation. It keeps
    /// semantic cardinality and readiness derivation out of API, CLI, and
    /// protocol leaves.
    #[must_use]
    pub fn inspection(&self) -> super::CompilationInspection<'_> {
        super::CompilationInspection::new(self)
    }

    /// Portable project and execution-workspace profile admitted into this compilation.
    pub fn profile(&self) -> &EffectiveProjectProfileV1 {
        &self.profile
    }

    /// Machine-specific instance admitted into this one-shot compilation.
    pub fn instance(&self) -> &ResolvedProjectInstanceV1 {
        &self.instance
    }

    /// Selection-bounded semantic graph used for every repository projection.
    ///
    /// The returned IR remains inspectable and can be rechecked with
    /// [`SealedPackageGraph::verify`].
    pub fn package_semantic_ir(&self) -> &SealedPackageGraph {
        &self.package_semantic_ir
    }

    /// Principal-relative workspace universe admitted into this compilation.
    ///
    /// The returned identity proves desired-state admission only. It does not
    /// imply installation, activation, provider readiness, or native effect.
    pub fn principal_admitted_universe_digest(&self) -> &PrincipalAdmittedUniverseDigest {
        self.workspace_universe.digest()
    }

    /// Verified path-free workspace semantic identity bound into compilation.
    pub fn workspace_semantic_ir_digest(&self) -> &WorkspaceSemanticIrDigest {
        self.workspace_universe.workspace_semantic_ir_digest()
    }

    /// Canonical identity of the exact owner-proof set used for admission.
    pub fn workspace_admission_evidence_digest(&self) -> &WorkspaceAdmissionEvidenceDigest {
        self.workspace_universe.admission_evidence_digest()
    }

    /// Canonical identity of the selected workspace-resolution lock.
    pub fn workspace_resolution_lock_digest(&self) -> &WorkspaceResolutionLockDigest {
        self.workspace_universe.resolution_lock_digest()
    }

    /// Canonical identity of the complete desired workspace-universe lock.
    pub fn workspace_universe_lock_digest(&self) -> &WorkspaceUniverseLockDigest {
        self.workspace_universe.universe_lock_digest()
    }

    /// Canonical identity of the attributable responsibility closure.
    pub fn responsibility_derivation_closure_digest(
        &self,
    ) -> &ResponsibilityDerivationClosureDigest {
        self.workspace_universe
            .responsibility_derivation_closure_digest()
    }

    /// Canonical identity of the explicit optional organizational closure.
    pub fn organizational_translation_closure_digest(
        &self,
    ) -> &OrganizationalTranslationClosureDigest {
        self.workspace_universe
            .organizational_translation_closure_digest()
    }

    /// Canonical identity of the explicit neutral composition source closure.
    pub fn experience_composition_source_digest(&self) -> &ExperienceCompositionSourceDigest {
        self.workspace_universe
            .experience_composition_source_digest()
    }

    /// Canonical identity of the normalized provider-binding closure.
    ///
    /// This coordinate binds only declared provider bindings retained by the
    /// composition plan. It does not prove provider availability, activation,
    /// invocation, native effect, or owner acceptance.
    pub fn provider_binding_closure_digest(&self) -> &ProviderBindingClosureDigest {
        &self.seal.provider_binding_closure_digest
    }

    /// Canonical identity of the resolved machine-only overlay.
    ///
    /// The overlay is independently typed from [`Self::profile`] and
    /// [`Self::instance`]; none of the three identities can substitute for
    /// either of the others.
    pub fn machine_overlay_digest(&self) -> &MachineOverlayDigest {
        &self.seal.machine_overlay_digest
    }

    /// Executable identity admitted by byte, interface, and platform checks.
    ///
    /// Portable and removal compilations always retain [`None`]. Presence is
    /// machine-verification evidence, not activation or native-effect proof.
    pub fn executable_identity_digest(&self) -> Option<&ExecutableIdentityDigest> {
        self.seal.executable_identity_digest.as_ref()
    }

    /// Number of exact owner-proof records retained by workspace admission.
    ///
    /// Zero is meaningful for an explicitly empty sovereign universe. It does
    /// not mean that proof was skipped for a populated owner frontier.
    #[must_use]
    pub fn workspace_admission_evidence_count(&self) -> usize {
        self.workspace_universe.admission_evidence_count()
    }

    /// Count of owner-qualified Package resources retained by the sealed IR.
    ///
    /// This is a diagnostic cardinality across semantic record families; body
    /// components and repository projections are intentionally not counted.
    pub fn package_semantic_ir_record_count(&self) -> usize {
        self.package_semantic_ir.resource_count()
    }

    /// Borrow the compiler-admitted neutral composition plan.
    ///
    /// Its reason paths are the exact projections produced by successful path
    /// proofs and retained in this compilation's input digest. This provenance
    /// does not imply runtime readiness, activation, a native occurrence, or an
    /// effect.
    pub fn experience_composition_plan(&self) -> &ExperienceCompositionPlanV1 {
        &self.experience_composition_plan
    }

    /// Exact source coordinates retained by the admitted workspace derivation graph.
    ///
    /// These records retain paths, content digests, and byte lengths only. The
    /// loaded source bodies were dropped after successful admission.
    pub fn workspace_derivation_sources(&self) -> &[WorkspaceDerivationSourceV1] {
        self.workspace_universe.derivation_sources()
    }

    /// Canonical causal nodes explaining the admitted workspace semantic IR.
    ///
    /// Following [`WorkspaceDerivationNodeV1::inputs`] reconstructs the
    /// reviewable source-to-semantics paths. The graph is desired-state
    /// compilation evidence and makes no runtime or native-effect claim.
    pub fn workspace_derivation_nodes(&self) -> &[WorkspaceDerivationNodeV1] {
        self.workspace_universe.derivation_nodes()
    }

    /// Exact workspace module releases admitted into the resident compiler input.
    pub(crate) fn workspace_module_release_closure(&self) -> &[WorkspaceAdoptedReleaseV1] {
        self.workspace_universe.adopted_releases()
    }

    /// Package releases selected through admitted workspace resources.
    pub(crate) fn workspace_package_release_closure(&self) -> &[PackageReleaseV1] {
        self.workspace_universe.package_releases()
    }

    /// Normative Pack releases selected through admitted workspace resources.
    pub(crate) fn workspace_normative_pack_release_closure(&self) -> &[NormativePackReleaseV1] {
        self.workspace_universe.normative_pack_releases()
    }

    /// Derive the exact causal transition from a prior admitted workspace universe.
    pub(crate) fn workspace_derivation_receipt(
        &self,
        prior: &ResidentWorkspaceDerivation,
    ) -> Result<WorkspaceDerivationReceiptV1, crate::workspace::WorkspaceError> {
        self.workspace_universe.derivation_receipt(prior)
    }

    /// Consume verified one-shot bodies into the resident-definition boundary.
    pub(crate) fn into_resident_parts(self) -> ResidentCompilationParts {
        ResidentCompilationParts {
            profile: self.profile,
            instance: self.instance,
            package_semantic_ir: self.package_semantic_ir,
            workspace: self.workspace_universe.into_resident_parts(),
            experience_composition_plan: self.experience_composition_plan,
            repositories: self.repos,
        }
    }

    /// Repository projections in validated materialization order.
    ///
    /// Slice order is contract data and must not be replaced with map iteration.
    pub fn repositories(&self) -> &[CompiledRepo] {
        &self.repos
    }

    /// Manifest identity sealed for one compiled repository.
    pub fn repository_manifest_digest(
        &self,
        repository_id: &LogicalRepositoryIdV1,
    ) -> Option<&ManifestDigest> {
        self.seal
            .repositories
            .get(repository_id)
            .map(|closure| &closure.manifest_digest)
    }

    /// Render identity sealed for one compiled repository.
    pub fn repository_render_digest(
        &self,
        repository_id: &LogicalRepositoryIdV1,
    ) -> Option<&RepositoryRenderDigest> {
        self.seal
            .repositories
            .get(repository_id)
            .map(|closure| &closure.render_digest)
    }

    /// Exact descriptor-manifest and raw-schema-set identity admitted at compile time.
    ///
    /// This identifies the schema registry used for admission, not a runtime or
    /// resident-generation version.
    pub fn schema_registry_identity(&self) -> &SchemaRegistryIdentity {
        &self.seal.schema_registry
    }

    /// Canonical identity of the exact admitted distribution lock.
    pub fn distribution_lock_digest(&self) -> &DistributionLockDigest {
        &self.seal.distribution_lock_digest
    }

    /// Canonical identity of the selected pre-render compilation inputs.
    ///
    /// Repository render bytes are deliberately excluded; use
    /// [`Self::one_shot_compilation_digest`] for the complete one-shot output.
    pub fn compilation_input_digest(&self) -> &CompilationInputDigest {
        &self.seal.compilation_input_digest
    }

    /// Canonical identity of this complete post-render one-shot compilation.
    ///
    /// This is neither a resident generation identity nor an activation counter.
    pub fn one_shot_compilation_digest(&self) -> &OneShotCompilationDigest {
        &self.seal.one_shot_compilation_digest
    }

    /// Admit one complete compiler result and retain its one-shot content identity.
    pub(super) fn admit(parts: CompilationParts) -> Result<Self, CompilationInvariantError> {
        let prepared = prepare_compilation_input(CompilationInputSources {
            schema_registry: &parts.schema_registry,
            distribution_lock_digest: &parts.distribution_lock_digest,
            package_closure: &parts.package_closure,
            profile: &parts.profile,
            instance: &parts.instance,
            package_semantic_ir_digest: parts.package_semantic_ir.digest(),
            workspace_universe: &parts.workspace_universe,
            experience_composition_plan: &parts.experience_composition_plan,
            executable_identity_digest: parts.executable_identity_digest.as_ref(),
            mode: parts.mode,
        })?;
        if prepared.digest != parts.compilation_input_digest {
            return Err(CompilationInvariantError::CoordinateDrift {
                coordinate: "pre-render compilation input identity before admission",
            });
        }
        let repositories = repository_closure_index(
            prepared.identity,
            &parts.schema_registry,
            &prepared.digest,
            &parts.repos,
        )?;
        let one_shot_compilation_digest =
            one_shot_compilation_digest(&prepared.digest, &repositories)?;
        let seal = CompilationSeal {
            profile_digest: prepared.identity.profile().clone(),
            instance_digest: prepared.identity.instance().clone(),
            machine_overlay_digest: prepared.machine_overlay_digest,
            provider_binding_closure_digest: prepared.provider_binding_closure_digest,
            compilation_input_digest: prepared.digest,
            schema_registry: parts.schema_registry,
            package_closure: parts.package_closure,
            mode: parts.mode,
            distribution_lock_digest: parts.distribution_lock_digest,
            executable_identity_digest: parts.executable_identity_digest,
            one_shot_compilation_digest,
            repositories,
        };
        Ok(Self {
            profile: parts.profile,
            instance: parts.instance,
            package_semantic_ir: parts.package_semantic_ir,
            workspace_universe: parts.workspace_universe,
            experience_composition_plan: parts.experience_composition_plan,
            repos: parts.repos,
            seal,
        })
    }

    /// Verify that observable compiler output still matches its admitted one-shot identity.
    pub(crate) fn verify_admitted_compilation(&self) -> Result<(), CompilationInvariantError> {
        self.package_semantic_ir.verify()?;
        let composition_plan_digest = &self.experience_composition_plan.plan_digest;
        let recomputed_composition_plan_digest = self
            .experience_composition_plan
            .recompute_digest()
            .map_err(|source| CompilationInvariantError::Canonicalization {
                coordinate: "experience composition plan verification",
                source,
            })?;
        if composition_plan_digest != &recomputed_composition_plan_digest {
            return Err(CompilationInvariantError::CoordinateDrift {
                coordinate: "experience composition plan after admission",
            });
        }
        let prepared = prepare_compilation_input(CompilationInputSources {
            schema_registry: &self.seal.schema_registry,
            distribution_lock_digest: &self.seal.distribution_lock_digest,
            package_closure: &self.seal.package_closure,
            profile: &self.profile,
            instance: &self.instance,
            package_semantic_ir_digest: self.package_semantic_ir.digest(),
            workspace_universe: &self.workspace_universe,
            experience_composition_plan: &self.experience_composition_plan,
            executable_identity_digest: self.seal.executable_identity_digest.as_ref(),
            mode: self.seal.mode,
        })?;
        if prepared.identity.profile() != &self.seal.profile_digest {
            return Err(CompilationInvariantError::CoordinateDrift {
                coordinate: "profile identity after admission",
            });
        }
        if prepared.identity.instance() != &self.seal.instance_digest {
            return Err(CompilationInvariantError::CoordinateDrift {
                coordinate: "instance identity after admission",
            });
        }
        if prepared.machine_overlay_digest != self.seal.machine_overlay_digest {
            return Err(CompilationInvariantError::CoordinateDrift {
                coordinate: "machine overlay identity after admission",
            });
        }
        if prepared.provider_binding_closure_digest != self.seal.provider_binding_closure_digest {
            return Err(CompilationInvariantError::CoordinateDrift {
                coordinate: "provider-binding closure after admission",
            });
        }
        if prepared.digest != self.seal.compilation_input_digest {
            return Err(CompilationInvariantError::CoordinateDrift {
                coordinate: "compilation input identity after admission",
            });
        }
        let repositories = repository_closure_index(
            prepared.identity,
            &self.seal.schema_registry,
            &prepared.digest,
            &self.repos,
        )?;
        if repositories != self.seal.repositories {
            return Err(CompilationInvariantError::CoordinateDrift {
                coordinate: "repository projections after admission",
            });
        }
        let one_shot = one_shot_compilation_digest(&prepared.digest, &repositories)?;
        if one_shot != self.seal.one_shot_compilation_digest {
            return Err(CompilationInvariantError::CoordinateDrift {
                coordinate: "one-shot compilation identity after admission",
            });
        }
        Ok(())
    }
}

/// Verify owner-produced coordinates and retain their proof beside the input seal.
///
/// Rendering and manifest rebuilding reuse this immutable borrow within one
/// admission. Final admission and explicit revalidation each call this function
/// afresh; the proof is not a cross-compilation cache or an activation witness.
pub(super) fn prepare_compilation_input(
    sources: CompilationInputSources<'_>,
) -> Result<PreparedCompilationInput<'_>, CompilationInvariantError> {
    let identity = crate::identity::ProjectIdentity::verify(sources.profile, sources.instance)
        .map_err(|source| CompilationInvariantError::ProjectIdentity {
            source: Box::new(source),
        })?;
    let machine_overlay_digest = sources
        .instance
        .machine_overlay_digest()
        .map_err(|source| CompilationInvariantError::Canonicalization {
            coordinate: "resolved machine overlay",
            source,
        })?;
    let provider_binding_closure_digest = sources
        .experience_composition_plan
        .provider_binding_closure_digest()
        .map_err(|source| CompilationInvariantError::Canonicalization {
            coordinate: "provider-binding closure",
            source,
        })?;
    let profile_digest = identity.profile().clone();
    let instance_digest = identity.instance().clone();
    let digest = compilation_input_digest(&CompilationInputPreimage {
        schema: COMPILATION_INPUT_SCHEMA,
        compiler_contract_id: super::DISTRIBUTION_COMPILER_ID,
        renderer_contract_id: crate::plan::RENDERER_VERSION,
        schema_registry: sources.schema_registry,
        distribution_lock_digest: sources.distribution_lock_digest,
        packages: sources.package_closure,
        profile_digest: &profile_digest,
        instance_digest: &instance_digest,
        package_semantic_ir_digest: sources.package_semantic_ir_digest,
        workspace_semantic_ir_digest: sources.workspace_universe.workspace_semantic_ir_digest(),
        workspace_admission_evidence_digest: sources.workspace_universe.admission_evidence_digest(),
        principal_admitted_universe_digest: sources.workspace_universe.digest(),
        workspace_resolution_lock_digest: sources.workspace_universe.resolution_lock_digest(),
        workspace_universe_lock_digest: sources.workspace_universe.universe_lock_digest(),
        responsibility_derivation_closure_digest: sources
            .workspace_universe
            .responsibility_derivation_closure_digest(),
        organizational_translation_closure_digest: sources
            .workspace_universe
            .organizational_translation_closure_digest(),
        experience_composition_source_digest: sources
            .workspace_universe
            .experience_composition_source_digest(),
        provider_binding_closure_digest: &provider_binding_closure_digest,
        machine_overlay_digest: &machine_overlay_digest,
        executable_identity_digest: sources.executable_identity_digest,
        workspace_module_releases: sources.workspace_universe.adopted_releases(),
        workspace_package_releases: sources.workspace_universe.package_releases(),
        workspace_normative_pack_releases: sources.workspace_universe.normative_pack_releases(),
        experience_composition_plan_digest: &sources.experience_composition_plan.plan_digest,
        mode: sources.mode,
    })?;
    Ok(PreparedCompilationInput {
        identity,
        machine_overlay_digest,
        provider_binding_closure_digest,
        digest,
    })
}

fn compilation_input_digest(
    preimage: &CompilationInputPreimage<'_>,
) -> Result<CompilationInputDigest, CompilationInvariantError> {
    CompilationInputDigest::hash_canonical(preimage).map_err(|source| {
        CompilationInvariantError::Canonicalization {
            coordinate: "compilation inputs",
            source,
        }
    })
}

fn repository_render_digest(
    repository: &CompiledRepo,
) -> Result<RepositoryRenderDigest, CompilationInvariantError> {
    let mut surfaces = repository.render.surfaces.iter().collect::<Vec<_>>();
    surfaces.sort_by(|left, right| left.rel_path.cmp(&right.rel_path));
    for pair in surfaces.windows(2) {
        if pair[0].rel_path == pair[1].rel_path {
            return Err(CompilationInvariantError::structure(
                "repository render",
                format!(
                    "repository `{}` renders `{}` more than once",
                    repository.repo_id, pair[0].rel_path
                ),
            ));
        }
    }
    let surfaces = surfaces
        .into_iter()
        .map(|surface| {
            let mut execution_path_consumers =
                surface.execution_path_consumers.iter().collect::<Vec<_>>();
            execution_path_consumers.sort_unstable();
            reject_adjacent_duplicates(
                &execution_path_consumers,
                "surface execution-path consumer",
            )?;
            let referenced_policy_ids =
                sorted_unique_refs(&surface.referenced_policy_ids, "surface referenced policy")?;
            Ok(RenderedSurfacePreimage {
                rel_path: &surface.rel_path,
                kind: &surface.kind,
                ownership: surface.ownership,
                content: surface.content(),
                execution_path_consumers,
                referenced_policy_ids,
                committable: surface.committable,
                deferred: surface.deferred,
                target_note: surface.target_note.as_deref(),
            })
        })
        .collect::<Result<Vec<_>, CompilationInvariantError>>()?;

    let mut support = repository.render.support.iter().collect::<Vec<_>>();
    support.sort_unstable_by(|left, right| {
        left.execution_path_witness
            .selection
            .id()
            .cmp(right.execution_path_witness.selection.id())
    });
    for pair in support.windows(2) {
        if pair[0].execution_path_witness.selection.id()
            == pair[1].execution_path_witness.selection.id()
        {
            return Err(CompilationInvariantError::structure(
                "repository execution-path support",
                format!(
                    "repository `{}` qualifies execution path `{}` more than once",
                    repository.repo_id,
                    pair[0].execution_path_witness.selection.id()
                ),
            ));
        }
    }
    let execution_path_support = support
        .into_iter()
        .map(|row| {
            let policy_ids = sorted_unique_refs(&row.policy_ids, "execution-path policy")?;
            Ok(row.seal_projection(policy_ids))
        })
        .collect::<Result<Vec<_>, CompilationInvariantError>>()?;

    RepositoryRenderDigest::hash_canonical(&RepositoryRenderPreimage {
        schema: REPOSITORY_RENDER_SCHEMA,
        repository_id: &repository.repo_id,
        surfaces,
        execution_path_support,
    })
    .map_err(|source| CompilationInvariantError::Canonicalization {
        coordinate: "repository render",
        source,
    })
}

fn sorted_unique_refs<'a>(
    values: &'a [String],
    label: &str,
) -> Result<Vec<&'a str>, CompilationInvariantError> {
    let mut values = values.iter().map(String::as_str).collect::<Vec<_>>();
    values.sort_unstable();
    reject_adjacent_duplicates(&values, label)?;
    Ok(values)
}

fn reject_adjacent_duplicates<T: PartialEq>(
    values: &[T],
    label: &str,
) -> Result<(), CompilationInvariantError> {
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        Err(CompilationInvariantError::structure(
            "ordered unique collection",
            format!("{label} occurs more than once"),
        ))
    } else {
        Ok(())
    }
}

fn repository_closure_index(
    identity: crate::identity::ProjectIdentity<'_>,
    schema_registry: &SchemaRegistryIdentity,
    compilation_input_digest: &CompilationInputDigest,
    repositories: &[CompiledRepo],
) -> Result<BTreeMap<LogicalRepositoryIdV1, RepositoryClosure>, CompilationInvariantError> {
    let mut closures = BTreeMap::new();
    for repository in repositories {
        if repository.manifest.repository_id != repository.repo_id {
            return Err(CompilationInvariantError::structure(
                "repository closure",
                format!(
                    "repository `{}` carries manifest identity `{}`",
                    repository.repo_id, repository.manifest.repository_id
                ),
            ));
        }
        let manifest_digest = repository.manifest.manifest_digest.clone();
        let rebuilt = crate::plan::build_manifest_with_identity(
            schema_registry,
            compilation_input_digest,
            &repository.render,
            &repository.repo_id,
            identity,
        )
        .map_err(|source| CompilationInvariantError::ManifestRebuild {
            repository: repository.repo_id.to_string(),
            source: Box::new(source),
        })?;
        if rebuilt != repository.manifest {
            return Err(CompilationInvariantError::structure(
                "repository closure",
                format!(
                    "repository `{}` manifest does not seal its current render",
                    repository.repo_id
                ),
            ));
        }
        let closure = RepositoryClosure {
            repository_id: repository.repo_id.clone(),
            manifest_digest,
            render_digest: repository_render_digest(repository)?,
        };
        if closures
            .insert(repository.repo_id.clone(), closure)
            .is_some()
        {
            return Err(CompilationInvariantError::structure(
                "repository closure",
                format!(
                    "compiled repository `{}` occurs more than once",
                    repository.repo_id
                ),
            ));
        }
    }
    Ok(closures)
}

fn one_shot_compilation_digest(
    input_digest: &CompilationInputDigest,
    repositories: &BTreeMap<LogicalRepositoryIdV1, RepositoryClosure>,
) -> Result<OneShotCompilationDigest, CompilationInvariantError> {
    OneShotCompilationDigest::hash_canonical(&OneShotCompilationPreimage {
        schema: ONE_SHOT_COMPILATION_SCHEMA,
        compilation_input_digest: input_digest,
        repositories: repositories.values().collect(),
    })
    .map_err(|source| CompilationInvariantError::Canonicalization {
        coordinate: "one-shot compilation",
        source,
    })
}

#[cfg(test)]
#[path = "../../tests/internal/compile/types.rs"]
mod tests;
