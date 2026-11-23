//! Owned flat summaries of an admitted one-shot compiler result.

use pale_application::compile::{Compilation, RepositoryInspection};
use pale_application::render::ExecutionPathSupport;
use serde::Serialize;

use crate::EffectiveProfileProviderBindingRef;

/// Bounded, filesystem-path-free projection of one compiler-qualified execution path.
///
/// The proof coordinates come from the retained compiler witness. Hook
/// requirements come from the renderer's retained typed result; this API never
/// reparses generated files or reruns composition and lifecycle policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
pub struct ExecutionPathSummary {
    selection_id: String,
    experience_path_id: String,
    responsibility_path_identity: String,
    work_delegation_edge_ids: Vec<String>,
    direct: bool,
    composition_plan_digest: String,
    terminal_provider_binding: Option<EffectiveProfileProviderBindingRef>,
    activation_selected: bool,
    hook_events: Vec<String>,
    gated_tools: Vec<String>,
    qualification: String,
}

impl ExecutionPathSummary {
    fn project(support: &ExecutionPathSupport) -> Self {
        let witness = &support.execution_path_witness;
        Self {
            selection_id: witness.selection.id().to_string(),
            experience_path_id: witness.selection.experience_path_id().to_string(),
            responsibility_path_identity: witness
                .responsibility_path_identity
                .responsibility_path_sha256
                .as_str()
                .to_string(),
            work_delegation_edge_ids: witness.work_delegation_edge_ids.clone(),
            direct: witness.is_direct(),
            composition_plan_digest: witness.composition_plan_digest.as_str().to_string(),
            terminal_provider_binding: witness
                .terminal_provider_binding()
                .map(EffectiveProfileProviderBindingRef::from),
            activation_selected: support.activation_selected,
            hook_events: support
                .hook_requirements
                .events()
                .iter()
                .map(|event| event.as_str().to_string())
                .collect(),
            gated_tools: support.hook_requirements.gated_tools().to_vec(),
            qualification: support.note.clone(),
        }
    }

    /// Stable identity of the selected execution path.
    #[must_use]
    pub fn selection_id(&self) -> &str {
        &self.selection_id
    }

    /// Neutral experience-path identity selected by this projection.
    #[must_use]
    pub fn experience_path_id(&self) -> &str {
        &self.experience_path_id
    }

    /// Domain-separated identity of the complete selected responsibility path.
    #[must_use]
    pub fn responsibility_path_identity(&self) -> &str {
        &self.responsibility_path_identity
    }

    /// Ordered work-delegation edges copied from the admitted composition path.
    #[must_use]
    pub fn work_delegation_edge_ids(&self) -> &[String] {
        &self.work_delegation_edge_ids
    }

    /// Whether the admitted path has no work-delegation edge.
    ///
    /// The compiler derives this from the witness's exact ordered delegation
    /// edges. Terminal harness and path spelling are not directness heuristics.
    #[must_use]
    pub const fn is_direct(&self) -> bool {
        self.direct
    }

    /// Canonical composition-plan identity from which this witness was projected.
    #[must_use]
    pub fn composition_plan_digest(&self) -> &str {
        &self.composition_plan_digest
    }

    /// Exact opaque terminal provider binding selected for the path.
    #[must_use]
    pub const fn terminal_provider_binding(&self) -> Option<&EffectiveProfileProviderBindingRef> {
        self.terminal_provider_binding.as_ref()
    }

    /// Whether private configuration selects repository-local activation.
    #[must_use]
    pub const fn activation_selected(&self) -> bool {
        self.activation_selected
    }

    /// Lifecycle events required by the retained renderer-owned resolution.
    #[must_use]
    pub fn hook_events(&self) -> &[String] {
        &self.hook_events
    }

    /// Exact command-tool matchers gated by pre-tool lifecycle policy.
    #[must_use]
    pub fn gated_tools(&self) -> &[String] {
        &self.gated_tools
    }

    /// Human-readable evidence qualification accompanying the typed fields.
    #[must_use]
    pub fn qualification(&self) -> &str {
        &self.qualification
    }
}

/// Bounded owned observations for one compiled repository projection.
///
/// Counts describe compiler output only. Provider invocation readiness remains in the
/// operation-scoped, generation-bound provider invocation vector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
pub struct RepositorySummary {
    repository_id: String,
    manifest_digest: String,
    render_digest: String,
    surface_count: u32,
    materialized_surface_count: u32,
    deferred_surface_count: u32,
    committable_surface_count: u32,
    execution_path_count: u32,
    operation_count: u32,
    role_count: u32,
    doctrine_count: u32,
    run_mode_count: u32,
    stack_count: u32,
    policy_family_count: u32,
    recipe_count: u32,
    activation_selected_execution_path_ids: Vec<String>,
    activation_selected_hook_events: Vec<String>,
    surfaces: Vec<String>,
    execution_paths: Vec<ExecutionPathSummary>,
}

impl RepositorySummary {
    /// Canonical logical repository identity selected by the project.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Digest of the repository's generated-surface manifest.
    #[must_use]
    pub fn manifest_digest(&self) -> &str {
        &self.manifest_digest
    }

    /// Digest of the complete deterministic repository render.
    #[must_use]
    pub fn render_digest(&self) -> &str {
        &self.render_digest
    }

    /// Number of generated filesystem and logical-artifact surfaces.
    #[must_use]
    pub const fn surface_count(&self) -> u32 {
        self.surface_count
    }

    /// Number of non-deferred filesystem targets in the compiled projection.
    #[must_use]
    pub const fn materialized_surface_count(&self) -> u32 {
        self.materialized_surface_count
    }

    /// Number of surfaces whose placement remains an explicit later effect.
    #[must_use]
    pub const fn deferred_surface_count(&self) -> u32 {
        self.deferred_surface_count
    }

    /// Number of surfaces eligible for repository history when materialized.
    #[must_use]
    pub const fn committable_surface_count(&self) -> u32 {
        self.committable_surface_count
    }

    /// Number of independently qualified execution-path rows.
    #[must_use]
    pub const fn execution_path_count(&self) -> u32 {
        self.execution_path_count
    }

    /// Number of selected semantic operations.
    #[must_use]
    pub const fn operation_count(&self) -> u32 {
        self.operation_count
    }

    /// Number of selected specialist roles.
    #[must_use]
    pub const fn role_count(&self) -> u32 {
        self.role_count
    }

    /// Number of selected standalone doctrine records.
    #[must_use]
    pub const fn doctrine_count(&self) -> u32 {
        self.doctrine_count
    }

    /// Number of selected run modes.
    #[must_use]
    pub const fn run_mode_count(&self) -> u32 {
        self.run_mode_count
    }

    /// Number of selected technology or workflow stacks.
    #[must_use]
    pub const fn stack_count(&self) -> u32 {
        self.stack_count
    }

    /// Number of content-bearing policy families.
    #[must_use]
    pub const fn policy_family_count(&self) -> u32 {
        self.policy_family_count
    }

    /// Number of selected reusable Package recipes.
    #[must_use]
    pub const fn recipe_count(&self) -> u32 {
        self.recipe_count
    }

    /// Activation-selected execution-path selection identities in stable order.
    #[must_use]
    pub fn activation_selected_execution_path_ids(&self) -> &[String] {
        &self.activation_selected_execution_path_ids
    }

    /// Deduplicated lifecycle events rendered for activation-selected paths.
    ///
    /// Values come from [`ExecutionPathSummary::hook_events`], whose owner is the
    /// renderer's retained typed requirement result.
    #[must_use]
    pub fn activation_selected_hook_events(&self) -> &[String] {
        &self.activation_selected_hook_events
    }

    /// Generated target paths in stable lexical order.
    #[must_use]
    pub fn surfaces(&self) -> &[String] {
        &self.surfaces
    }

    /// Complete path-support rows in stable selection-identity order.
    #[must_use]
    pub fn execution_paths(&self) -> &[ExecutionPathSummary] {
        &self.execution_paths
    }

    fn project(repository: RepositoryInspection<'_>) -> Result<Self, SummaryProjectionError> {
        let mut execution_paths = repository
            .execution_paths()
            .iter()
            .map(ExecutionPathSummary::project)
            .collect::<Vec<_>>();
        execution_paths.sort_unstable_by(|left, right| left.selection_id.cmp(&right.selection_id));
        let activation_selected_execution_path_ids = repository
            .activation_selected_execution_path_ids()
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let activation_selected_hook_events = repository
            .activation_selected_hook_events()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let surfaces = repository
            .surface_paths()
            .into_iter()
            .map(str::to_owned)
            .collect();
        Ok(Self {
            repository_id: repository.repository_id().to_string(),
            manifest_digest: repository.manifest_digest().as_str().to_string(),
            render_digest: repository
                .render_digest()
                .ok_or(SummaryProjectionError::MissingRepositorySeal)?
                .as_str()
                .to_string(),
            surface_count: count("repository surfaces", repository.surface_count())?,
            materialized_surface_count: count(
                "materialized repository surfaces",
                repository.materialized_surface_count(),
            )?,
            deferred_surface_count: count(
                "deferred repository surfaces",
                repository.deferred_surface_count(),
            )?,
            committable_surface_count: count(
                "committable repository surfaces",
                repository.committable_surface_count(),
            )?,
            execution_path_count: count(
                "repository execution-path rows",
                repository.execution_paths().len(),
            )?,
            operation_count: count("repository operations", repository.operation_count())?,
            role_count: count("repository roles", repository.role_count())?,
            doctrine_count: count("repository doctrine", repository.doctrine_count())?,
            run_mode_count: count("repository run modes", repository.run_mode_count())?,
            stack_count: count("repository stacks", repository.stack_count())?,
            policy_family_count: count(
                "repository policy families",
                repository.policy_family_count(),
            )?,
            recipe_count: count("repository recipes", repository.recipe_count())?,
            activation_selected_execution_path_ids,
            activation_selected_hook_events,
            surfaces,
            execution_paths,
        })
    }
}

/// Owned identity and cardinality projection of one complete compilation.
///
/// Every digest remains a distinct named field even though each wire spelling
/// is lowercase SHA-256. This prevents an instance, semantic-IR, composition,
/// compilation-input, or one-shot-output coordinate from being treated as a
/// resident generation or substituted for another domain.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
pub struct OneShotCompilationSummary {
    project_id: String,
    execution_workspace_id: String,
    profile_digest: String,
    instance_digest: String,
    machine_overlay_digest: String,
    package_semantic_ir_digest: String,
    workspace_semantic_ir_digest: String,
    workspace_admission_evidence_digest: String,
    principal_admitted_universe_digest: String,
    workspace_resolution_lock_digest: String,
    workspace_universe_lock_digest: String,
    responsibility_derivation_closure_digest: String,
    organizational_translation_closure_digest: String,
    experience_composition_source_digest: String,
    experience_composition_plan_digest: String,
    provider_binding_closure_digest: String,
    distribution_lock_digest: String,
    compilation_input_digest: String,
    one_shot_compilation_digest: String,
    schema_manifest_sha256: String,
    schema_set_sha256: String,
    executable_identity_digest: Option<String>,
    package_release_count: u32,
    normative_pack_release_count: u32,
    package_semantic_record_count: u32,
    workspace_admission_evidence_count: u32,
    workspace_derivation_source_count: u32,
    workspace_derivation_node_count: u32,
    composition_reason_path_count: u32,
    repository_count: u32,
    repositories: Vec<RepositorySummary>,
    /// Exact API-owned JCS projection of every preceding field.
    #[serde(skip)]
    #[cfg_attr(feature = "wire-schema", schemars(skip))]
    canonical_bytes: Vec<u8>,
}

impl OneShotCompilationSummary {
    pub(super) fn project(compilation: &Compilation) -> Result<Self, SummaryProjectionError> {
        let profile = compilation.profile();
        let registry = compilation.schema_registry_identity();
        let inspection = compilation.inspection();
        let repositories = inspection
            .repositories()
            .map(RepositorySummary::project)
            .collect::<Result<Vec<_>, _>>()?;
        let repository_count = count("repository projections", inspection.repository_count())?;
        let mut summary = Self {
            project_id: profile.project_id.to_string(),
            execution_workspace_id: profile.execution_workspace_id.to_string(),
            profile_digest: profile.profile_digest.as_str().to_string(),
            instance_digest: compilation.instance().instance_digest.as_str().to_string(),
            machine_overlay_digest: compilation.machine_overlay_digest().as_str().to_string(),
            package_semantic_ir_digest: compilation
                .package_semantic_ir()
                .digest()
                .as_str()
                .to_string(),
            workspace_semantic_ir_digest: compilation
                .workspace_semantic_ir_digest()
                .as_str()
                .to_string(),
            workspace_admission_evidence_digest: compilation
                .workspace_admission_evidence_digest()
                .as_str()
                .to_string(),
            principal_admitted_universe_digest: compilation
                .principal_admitted_universe_digest()
                .as_str()
                .to_string(),
            workspace_resolution_lock_digest: compilation
                .workspace_resolution_lock_digest()
                .as_str()
                .to_string(),
            workspace_universe_lock_digest: compilation
                .workspace_universe_lock_digest()
                .as_str()
                .to_string(),
            responsibility_derivation_closure_digest: compilation
                .responsibility_derivation_closure_digest()
                .as_str()
                .to_string(),
            organizational_translation_closure_digest: compilation
                .organizational_translation_closure_digest()
                .as_str()
                .to_string(),
            experience_composition_source_digest: compilation
                .experience_composition_source_digest()
                .as_str()
                .to_string(),
            experience_composition_plan_digest: compilation
                .experience_composition_plan()
                .plan_digest
                .as_str()
                .to_string(),
            provider_binding_closure_digest: compilation
                .provider_binding_closure_digest()
                .as_str()
                .to_string(),
            distribution_lock_digest: compilation.distribution_lock_digest().as_str().to_string(),
            compilation_input_digest: compilation.compilation_input_digest().as_str().to_string(),
            one_shot_compilation_digest: compilation
                .one_shot_compilation_digest()
                .as_str()
                .to_string(),
            schema_manifest_sha256: registry.schema_manifest_sha256.as_str().to_string(),
            schema_set_sha256: registry.schema_set_sha256.as_str().to_string(),
            executable_identity_digest: compilation
                .executable_identity_digest()
                .map(|digest| digest.as_str().to_string()),
            package_release_count: count("Package release closure", profile.package_closure.len())?,
            normative_pack_release_count: count(
                "normative Pack release closure",
                profile.normative_pack_closure.len(),
            )?,
            package_semantic_record_count: count(
                "Package semantic records",
                compilation.package_semantic_ir_record_count(),
            )?,
            workspace_admission_evidence_count: count(
                "workspace admission evidence",
                compilation.workspace_admission_evidence_count(),
            )?,
            workspace_derivation_source_count: count(
                "workspace derivation sources",
                compilation.workspace_derivation_sources().len(),
            )?,
            workspace_derivation_node_count: count(
                "workspace derivation nodes",
                compilation.workspace_derivation_nodes().len(),
            )?,
            composition_reason_path_count: count(
                "composition reason paths",
                compilation.experience_composition_plan().reason_paths.len(),
            )?,
            repository_count,
            repositories,
            canonical_bytes: Vec::new(),
        };
        summary.canonical_bytes = pale_core::canon::to_canonical_string(&summary)
            .map_err(SummaryProjectionError::Canonicalization)?
            .into_bytes();
        Ok(summary)
    }

    /// Exact canonical JSON projection of this complete summary.
    ///
    /// The bytes are constructed once by the API from the same private fields
    /// exposed through the typed accessors. CLI and language leaves can therefore
    /// deliver or translate this result without reconstructing a second summary
    /// shape or invoking an independent canonicalizer.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Canonical logical project identity selected by the request.
    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Canonical execution-workspace identity selected by the request.
    #[must_use]
    pub fn execution_workspace_id(&self) -> &str {
        &self.execution_workspace_id
    }

    /// Canonical identity of the portable effective profile.
    #[must_use]
    pub fn profile_digest(&self) -> &str {
        &self.profile_digest
    }

    /// Canonical identity of the complete resolved private instance.
    #[must_use]
    pub fn instance_digest(&self) -> &str {
        &self.instance_digest
    }

    /// Canonical identity of only the machine-specific overlay.
    #[must_use]
    pub fn machine_overlay_digest(&self) -> &str {
        &self.machine_overlay_digest
    }

    /// Canonical identity of the selected Package semantic graph.
    #[must_use]
    pub fn package_semantic_ir_digest(&self) -> &str {
        &self.package_semantic_ir_digest
    }

    /// Canonical identity of the admitted workspace semantic graph.
    #[must_use]
    pub fn workspace_semantic_ir_digest(&self) -> &str {
        &self.workspace_semantic_ir_digest
    }

    /// Canonical identity of the exact owner-proof set used by workspace admission.
    #[must_use]
    pub fn workspace_admission_evidence_digest(&self) -> &str {
        &self.workspace_admission_evidence_digest
    }

    /// Principal-relative identity of the complete admitted workspace universe.
    #[must_use]
    pub fn principal_admitted_universe_digest(&self) -> &str {
        &self.principal_admitted_universe_digest
    }

    /// Canonical identity of the selected workspace-resolution lock.
    #[must_use]
    pub fn workspace_resolution_lock_digest(&self) -> &str {
        &self.workspace_resolution_lock_digest
    }

    /// Canonical identity of the complete desired workspace-universe lock.
    #[must_use]
    pub fn workspace_universe_lock_digest(&self) -> &str {
        &self.workspace_universe_lock_digest
    }

    /// Canonical identity of the attributable responsibility closure.
    #[must_use]
    pub fn responsibility_derivation_closure_digest(&self) -> &str {
        &self.responsibility_derivation_closure_digest
    }

    /// Canonical identity of the optional organizational translation closure.
    #[must_use]
    pub fn organizational_translation_closure_digest(&self) -> &str {
        &self.organizational_translation_closure_digest
    }

    /// Canonical identity of the explicit composition-source closure.
    #[must_use]
    pub fn experience_composition_source_digest(&self) -> &str {
        &self.experience_composition_source_digest
    }

    /// Canonical identity of the neutral compiled composition plan.
    #[must_use]
    pub fn experience_composition_plan_digest(&self) -> &str {
        &self.experience_composition_plan_digest
    }

    /// Canonical identity of declared provider bindings retained by composition.
    ///
    /// This is declaration identity, not provider availability or admission.
    #[must_use]
    pub fn provider_binding_closure_digest(&self) -> &str {
        &self.provider_binding_closure_digest
    }

    /// Canonical identity of the exact admitted distribution lock.
    #[must_use]
    pub fn distribution_lock_digest(&self) -> &str {
        &self.distribution_lock_digest
    }

    /// Canonical identity of all admitted inputs before repository rendering.
    #[must_use]
    pub fn compilation_input_digest(&self) -> &str {
        &self.compilation_input_digest
    }

    /// Canonical identity of the complete post-render one-shot result.
    ///
    /// This is not a resident generation coordinate.
    #[must_use]
    pub fn one_shot_compilation_digest(&self) -> &str {
        &self.one_shot_compilation_digest
    }

    /// Descriptor-manifest identity admitted by the compiler.
    #[must_use]
    pub fn schema_manifest_sha256(&self) -> &str {
        &self.schema_manifest_sha256
    }

    /// Raw-schema-set identity admitted by the compiler.
    #[must_use]
    pub fn schema_set_sha256(&self) -> &str {
        &self.schema_set_sha256
    }

    /// Executable identity admitted by machine verification, when present.
    ///
    /// Portable and removal compilations return [`None`]. Presence is neither
    /// activation nor native-effect evidence.
    #[must_use]
    pub fn executable_identity_digest(&self) -> Option<&str> {
        self.executable_identity_digest.as_deref()
    }

    /// Number of selected Package releases.
    #[must_use]
    pub const fn package_release_count(&self) -> u32 {
        self.package_release_count
    }

    /// Number of selected normative Pack releases.
    #[must_use]
    pub const fn normative_pack_release_count(&self) -> u32 {
        self.normative_pack_release_count
    }

    /// Number of selected typed Package semantic records.
    #[must_use]
    pub const fn package_semantic_record_count(&self) -> u32 {
        self.package_semantic_record_count
    }

    /// Number of exact owner-proof records used by workspace admission.
    #[must_use]
    pub const fn workspace_admission_evidence_count(&self) -> u32 {
        self.workspace_admission_evidence_count
    }

    /// Number of retained workspace source coordinates.
    #[must_use]
    pub const fn workspace_derivation_source_count(&self) -> u32 {
        self.workspace_derivation_source_count
    }

    /// Number of retained workspace causal nodes.
    #[must_use]
    pub const fn workspace_derivation_node_count(&self) -> u32 {
        self.workspace_derivation_node_count
    }

    /// Number of successful composition proof paths retained by the plan.
    #[must_use]
    pub const fn composition_reason_path_count(&self) -> u32 {
        self.composition_reason_path_count
    }

    /// Repository summaries in validated materialization order.
    #[must_use]
    pub fn repositories(&self) -> &[RepositorySummary] {
        &self.repositories
    }

    /// Number of repository projections.
    #[must_use]
    pub const fn repository_count(&self) -> u32 {
        self.repository_count
    }
}

/// A compiler-owned bounded count or expected repository seal was unavailable.
#[derive(Debug, thiserror::Error)]
pub(super) enum SummaryProjectionError {
    /// One cardinality exceeded the portable API's fixed counter width.
    #[error("{coordinate} count exceeds the portable API bound")]
    CountOverflow {
        /// Human-readable name of the overflowing compiler-owned collection.
        coordinate: &'static str,
    },
    /// The admitted compilation omitted a repository identity from its private seal.
    #[error("admitted compilation has no seal for a selected repository")]
    MissingRepositorySeal,
    /// The owned summary could not be projected into canonical JSON.
    #[error("one-shot summary cannot be encoded as canonical JSON")]
    Canonicalization(#[source] pale_core::canon::CanonError),
}

fn count(coordinate: &'static str, value: usize) -> Result<u32, SummaryProjectionError> {
    u32::try_from(value).map_err(|_| SummaryProjectionError::CountOverflow { coordinate })
}
