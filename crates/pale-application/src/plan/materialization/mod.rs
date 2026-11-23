//! Shared preparation and assembly for state-aware materialization plans.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    AgentSurfacePlanPreimageV1, AgentSurfacePlanV1, BaselineEntry, DeferredEffect,
    LogicalRepositoryIdV1, ManifestDigest, MaterializationContractIdentity,
    MaterializationStateTransitionV1, MaterializationStateV1, NormativePackReleaseV1,
    OneShotCompilationDigest, Operation, PackageReleaseV1, PackageSemanticIrDigest, PlanKind,
    ProfileRepository, SchemaRegistryIdentity,
};

use crate::compile::Compilation;
use crate::identity::ProjectIdentity;
use crate::render::{GeneratedSurface, ProjectRender};

use super::manifest::build_manifest_with_identity;
use super::{
    validate, PlanError, ADMISSION_UNAVAILABLE, COMPILER_VERSION, OPERATION_MARKER,
    RENDERER_VERSION,
};

mod apply;
mod local;
mod remove;
mod state;
mod status;

use apply::{build_reconciled_adoption_plan, build_reconciled_apply_plan};
pub use local::{bind_machine_local_effects, BoundMaterializationPlan};
use remove::build_reconciled_remove_plan;
pub use status::{
    compare_reconciled_materialization, MaterializationDriftReason, MaterializationOwnershipChange,
    RepositoryMaterializationComparison, RepositoryMaterializationStatus,
};

/// Closed operator intent for deterministic repository materialization.
///
/// Adoption remains an apply-class operation because it establishes the same
/// next ownership state while requiring an explicit first-ownership policy.
/// The enum selects an existing planning law; it is not a runtime dispatcher,
/// wire contract, or effect authorization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaterializationIntent {
    /// Reconcile current Pale ownership with the desired projection.
    Apply,
    /// Explicitly adopt matching pre-existing generated targets.
    AdoptUnowned,
    /// Remove only targets proven by the prior ownership state.
    Remove,
}

/// Bind the ownership-state contracts selected by one sealed compilation.
///
/// This is the single schema-selection owner used by filesystem observation and
/// mutation composition. It creates no filesystem capability and performs no
/// state read, admission, materialization, or native effect.
///
/// # Errors
///
/// Returns [`pale_contracts::SchemaError`] if either required initial-v1
/// record schema is absent from the embedded registry.
pub fn materialization_contract_identity(
    compilation: &Compilation,
) -> Result<MaterializationContractIdentity, pale_contracts::SchemaError> {
    Ok(MaterializationContractIdentity::new(
        pale_contracts::record_schema_identity("materialization-state.v1")?,
        pale_contracts::record_schema_identity("materialization-lineage.v1")?,
        compilation.schema_registry_identity().clone(),
        compilation.one_shot_compilation_digest().clone(),
    ))
}

impl MaterializationIntent {
    /// Return the operation class sealed into the resulting surface plan.
    #[must_use]
    pub const fn plan_kind(self) -> PlanKind {
        match self {
            Self::Apply | Self::AdoptUnowned => PlanKind::Apply,
            Self::Remove => PlanKind::Remove,
        }
    }
}

/// One repository plan paired with its exact current rendered payloads.
///
/// The payload map is empty for [`MaterializationIntent::Remove`]. For apply
/// and adoption it contains every materializable current surface in canonical
/// target order. Machine-bound roots and machine-local effects remain outside
/// this path-free application result.
pub struct RepositoryMaterializationPlan {
    surface_plan: AgentSurfacePlanV1,
    payloads: MaterializationPayloads,
}

/// Immutable rendered text carried from compilation into effect staging.
///
/// This value deliberately exposes text, not the application's allocation or
/// sharing policy. Cloning it is cheap in the current implementation, while
/// consumers remain free from relying on pointer identity or storage layout.
#[derive(Clone)]
pub struct MaterializationPayload(std::sync::Arc<String>);

impl MaterializationPayload {
    fn from_surface(surface: &GeneratedSurface) -> Self {
        Self(surface.shared_content())
    }

    /// Borrow the exact rendered text authenticated by the materialization plan.
    #[must_use]
    pub fn content(&self) -> &str {
        self.0.as_str()
    }

    /// Consume the payload while preserving its compiler-owned shared allocation.
    #[must_use]
    pub fn into_shared(self) -> std::sync::Arc<String> {
        self.0
    }
}

impl AsRef<str> for MaterializationPayload {
    fn as_ref(&self) -> &str {
        self.content()
    }
}

impl std::fmt::Debug for MaterializationPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MaterializationPayload")
            .field("content_bytes", &self.0.len())
            .finish_non_exhaustive()
    }
}

/// Rendered payloads indexed by their confined repository target.
pub type MaterializationPayloads = BTreeMap<String, MaterializationPayload>;

/// Redact rendered bytes while retaining useful planning coordinates.
impl std::fmt::Debug for RepositoryMaterializationPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RepositoryMaterializationPlan")
            .field("repository_id", &self.surface_plan.repository_id)
            .field("plan_kind", &self.surface_plan.transition.plan_kind())
            .field("operation_count", &self.surface_plan.operations.len())
            .field("payload_count", &self.payloads.len())
            .finish()
    }
}

impl RepositoryMaterializationPlan {
    /// Borrow the deterministic repository-surface plan.
    #[must_use]
    pub const fn surface_plan(&self) -> &AgentSurfacePlanV1 {
        &self.surface_plan
    }

    /// Borrow rendered bytes indexed by their confined repository target.
    #[must_use]
    pub const fn payloads(&self) -> &MaterializationPayloads {
        &self.payloads
    }

    /// Consume the result into the plan and its exact rendered payloads.
    #[must_use]
    pub fn into_parts(self) -> (AgentSurfacePlanV1, MaterializationPayloads) {
        (self.surface_plan, self.payloads)
    }
}

/// One repository projection bound to its complete one-shot compiler result.
///
/// Construction selects the repository from [`Compilation`], then rebuilds its
/// manifest from the same profile, instance, and render. Package, Pack, and
/// selection-bounded semantic identities therefore come from the same admitted
/// compilation rather than from independently supplied references.
///
/// This is pure planning input. It grants neither plan admission nor filesystem
/// mutation authority; only exact saved-plan admission can mint that capability.
#[derive(Clone, Copy)]
pub struct RepositoryMaterializationInput<'a> {
    identity: ProjectIdentity<'a>,
    render: &'a ProjectRender,
    package_closure: &'a [PackageReleaseV1],
    normative_pack_closure: &'a [NormativePackReleaseV1],
    repository: &'a ProfileRepository,
    manifest_digest: &'a ManifestDigest,
    package_semantic_ir_digest: &'a PackageSemanticIrDigest,
    schema_registry: &'a SchemaRegistryIdentity,
    one_shot_compilation_digest: &'a OneShotCompilationDigest,
}

impl<'a> RepositoryMaterializationInput<'a> {
    /// Select and verify one repository from a complete compilation.
    ///
    /// The repository must occur exactly once in the compilation. Its manifest
    /// must be the deterministic projection of the compilation's profile,
    /// instance, and render, and both profile/instance self-digests must verify.
    ///
    /// # Errors
    ///
    /// Returns [`PlanError`] when the compilation seal is invalid, the repository
    /// is absent or duplicated, or its identity and manifest cannot be reproduced.
    pub fn for_repository(
        compilation: &'a Compilation,
        repository_id: &LogicalRepositoryIdV1,
    ) -> Result<Self, PlanError> {
        compilation
            .verify_admitted_compilation()
            .map_err(PlanError::from)?;
        let mut repositories = compilation
            .repositories()
            .iter()
            .filter(|repository| repository.id() == repository_id);
        let repository = repositories
            .next()
            .ok_or_else(|| PlanError::InvalidRunTarget(repository_id.to_string()))?;
        if repositories.next().is_some() {
            return Err(PlanError::InvalidManifest(format!(
                "compilation contains duplicate repository `{repository_id}`"
            )));
        }
        let profile = compilation.profile();
        let profile_repository = profile
            .repositories
            .iter()
            .find(|candidate| &candidate.id == repository_id)
            .ok_or_else(|| {
                PlanError::InvalidManifest(format!(
                    "compiled repository `{repository_id}` has no sealed profile member"
                ))
            })?;
        let instance = compilation.instance();
        let render = repository.render();
        let manifest = repository.manifest();
        let identity = ProjectIdentity::verify(profile, instance).map_err(|source| {
            PlanError::InvalidIdentity {
                source: Box::new(source),
            }
        })?;
        let expected = build_manifest_with_identity(
            compilation.schema_registry_identity(),
            compilation.compilation_input_digest(),
            render,
            repository_id,
            identity,
        )?;
        if manifest != &expected {
            return Err(PlanError::InvalidManifest(
                "record does not match its profile, instance, repository, and rendered outputs"
                    .to_string(),
            ));
        }
        let manifest_digest = &manifest.manifest_digest;
        Ok(Self {
            identity,
            render,
            package_closure: &profile.package_closure,
            normative_pack_closure: &profile.normative_pack_closure,
            repository: profile_repository,
            manifest_digest,
            package_semantic_ir_digest: compilation.package_semantic_ir().digest(),
            schema_registry: compilation.schema_registry_identity(),
            one_shot_compilation_digest: compilation.one_shot_compilation_digest(),
        })
    }

    /// Require the selected workspace edge to permit later repository mutation.
    ///
    /// This consumes the access ceiling already sealed into the effective
    /// profile. It does not infer authority from a checkout path, a generated
    /// surface, an operation effect, or a saved plan.
    fn require_repository_mutation(&self) -> Result<(), PlanError> {
        if self.repository.access.permits_repository_mutation() {
            Ok(())
        } else {
            Err(PlanError::RepositoryMutationDenied {
                repository: self.repository.id.clone(),
            })
        }
    }

    /// Derive the exact filesystem targets that a baseline adapter must observe.
    ///
    /// Apply and explicit adoption share [`PlanKind::Apply`]: they observe the
    /// union of trusted prior ownership and the desired non-deferred filesystem
    /// surface set. [`PlanKind::Remove`] observes only trusted prior ownership.
    /// Logical/deferred targets are intentionally absent because a repository
    /// filesystem baseline cannot establish their native state.
    ///
    /// The returned owned labels are sorted and unique. They are effect-free
    /// observation intent, not path capabilities, plan admission, or mutation
    /// authority.
    ///
    /// # Errors
    ///
    /// Returns [`PlanError`] when the prior state does not belong to this
    /// repository and schema registry, or desired surface construction violates
    /// the same materialization laws used by apply planning.
    pub fn required_observation_targets(
        &self,
        plan_kind: PlanKind,
        prior_state: Option<&MaterializationStateV1>,
    ) -> Result<Vec<String>, PlanError> {
        let (_, prior) =
            state::index_state(prior_state, &self.repository.id, self.schema_registry)?;
        let desired = match plan_kind {
            PlanKind::Apply => Some(PreparedSurfaceSet::from_render(self.render)?),
            PlanKind::Remove => None,
        };
        Ok(observation_target_set(
            plan_kind,
            prior.keys().copied(),
            desired.iter().flat_map(|prepared| {
                prepared
                    .materializable_surfaces()
                    .map(|surface| surface.rel_path.as_str())
            }),
        )
        .into_iter()
        .map(str::to_owned)
        .collect())
    }

    /// Build the selected repository plan from exact claimed observations.
    ///
    /// A caller that first invokes [`Self::required_observation_targets`] can
    /// reuse this verified selection, avoiding another repository lookup and
    /// compilation-seal traversal. The result carries no machine-bound path or
    /// machine-local effect.
    ///
    /// # Errors
    ///
    /// Returns [`PlanError`] when the selected membership is read-only or the
    /// prior state, baseline, rendered projection, or sealed plan is invalid.
    pub fn build(
        self,
        intent: MaterializationIntent,
        prior_state: Option<&MaterializationStateV1>,
        baseline: Vec<BaselineEntry>,
    ) -> Result<RepositoryMaterializationPlan, PlanError> {
        let payloads = if intent == MaterializationIntent::Remove {
            BTreeMap::new()
        } else {
            self.render
                .surfaces
                .iter()
                .filter(|surface| {
                    !surface.deferred && is_filesystem_target(surface.rel_path.as_str())
                })
                .map(|surface| {
                    (
                        surface.rel_path.clone(),
                        MaterializationPayload::from_surface(surface),
                    )
                })
                .collect()
        };
        let surface_plan = match intent {
            MaterializationIntent::Apply => {
                build_reconciled_apply_plan(self, prior_state, baseline)
            }
            MaterializationIntent::AdoptUnowned => {
                build_reconciled_adoption_plan(self, prior_state, baseline)
            }
            MaterializationIntent::Remove => {
                build_reconciled_remove_plan(self, prior_state, baseline)
            }
        }?;
        Ok(RepositoryMaterializationPlan {
            surface_plan,
            payloads,
        })
    }
}

/// Close prior and desired filesystem paths under one plan-kind policy.
fn observation_target_set<'path>(
    plan_kind: PlanKind,
    prior: impl IntoIterator<Item = &'path str>,
    desired: impl IntoIterator<Item = &'path str>,
) -> BTreeSet<&'path str> {
    let mut targets = prior.into_iter().collect::<BTreeSet<_>>();
    if plan_kind == PlanKind::Apply {
        targets.extend(desired);
    }
    targets
}

/// A render and baseline indexed only after every target is valid and unique.
pub(super) struct PreparedSurfaceSet<'a> {
    surfaces: BTreeMap<&'a str, &'a GeneratedSurface>,
    baseline: BTreeMap<&'a str, &'a BaselineEntry>,
}

impl<'a> PreparedSurfaceSet<'a> {
    /// Validate and index rendered targets without a prior materialization baseline.
    pub(super) fn from_render(render: &'a ProjectRender) -> Result<Self, PlanError> {
        Self::prepare(render, &[])
    }

    /// Validate and index rendered targets together with exact prior state.
    pub(super) fn for_materialization(
        render: &'a ProjectRender,
        baseline: &'a [BaselineEntry],
    ) -> Result<Self, PlanError> {
        Self::prepare(render, baseline)
    }

    fn prepare(
        render: &'a ProjectRender,
        baseline: &'a [BaselineEntry],
    ) -> Result<Self, PlanError> {
        let mut surfaces = BTreeMap::new();
        for surface in &render.surfaces {
            validate_target(&surface.rel_path).map_err(|invalid| {
                PlanError::InvalidManifest(format!(
                    "generated surface target `{}` is not confined",
                    invalid.into_path()
                ))
            })?;
            if surface.deferred {
                return Err(PlanError::InvalidManifest(format!(
                    "generated surface `{}` claims deferred placement, but initial-v1 has no admitted deferred-surface producer",
                    surface.rel_path
                )));
            }
            match surfaces.entry(surface.rel_path.as_str()) {
                Entry::Vacant(entry) => {
                    entry.insert(surface);
                }
                Entry::Occupied(_) => {
                    return Err(PlanError::DuplicateSurfaceTarget(surface.rel_path.clone()));
                }
            }
        }

        let mut indexed_baseline = BTreeMap::new();
        for entry in baseline {
            validate_target(&entry.rel_path).map_err(|invalid| {
                PlanError::InvalidBaseline(format!(
                    "baseline target `{}` is not confined",
                    invalid.into_path()
                ))
            })?;
            match indexed_baseline.entry(entry.rel_path.as_str()) {
                Entry::Vacant(slot) => {
                    slot.insert(entry);
                }
                Entry::Occupied(_) => {
                    return Err(PlanError::DuplicateBaselineTarget(entry.rel_path.clone()));
                }
            }
        }
        Ok(Self {
            surfaces,
            baseline: indexed_baseline,
        })
    }

    /// Iterate unique rendered surfaces in canonical path order.
    pub(super) fn surfaces(&self) -> impl Iterator<Item = &'a GeneratedSurface> + '_ {
        self.surfaces.values().copied()
    }

    /// Iterate surfaces represented by repository filesystem ownership state.
    pub(super) fn materializable_surfaces(
        &self,
    ) -> impl Iterator<Item = &'a GeneratedSurface> + '_ {
        self.surfaces()
            .filter(|surface| !surface.deferred && is_filesystem_target(&surface.rel_path))
    }

    /// Look up the admitted prior state for one confined target.
    pub(super) fn baseline_at(&self, rel_path: &str) -> Option<&'a BaselineEntry> {
        self.baseline.get(rel_path).copied()
    }

    /// Reject a baseline whose target set differs from the planned effects.
    pub(super) fn require_exact_baseline(
        &self,
        expected: &BTreeSet<&str>,
    ) -> Result<(), PlanError> {
        let observed = self.baseline.keys().copied().collect::<BTreeSet<_>>();
        if &observed == expected {
            return Ok(());
        }
        let missing = expected.difference(&observed).copied().collect::<Vec<_>>();
        let unexpected = observed.difference(expected).copied().collect::<Vec<_>>();
        Err(PlanError::InvalidBaseline(format!(
            "target set differs from planned effects (missing: {missing:?}; unexpected: {unexpected:?})"
        )))
    }
}

/// Whether a target names a confined repository path rather than a logical artifact.
///
/// This classifies target shape only. It does not grant mutation authority or
/// override a surface's deferred-effect status.
pub fn is_filesystem_target(rel_path: &str) -> bool {
    pale_core::paths::SurfaceTarget::parse(rel_path).is_some_and(|target| target.is_filesystem())
}

/// Return the canonical local-only target set selected by surface operations.
///
/// The set includes materialized non-committable surfaces plus the apply lock
/// and ownership-state paths that must remain outside source control. It is
/// semantic protection policy, independent of Git syntax or filesystem I/O.
#[must_use]
pub fn local_only_materialization_targets(operations: &[Operation]) -> Vec<String> {
    let mut paths = operations
        .iter()
        .filter(|operation| {
            !operation.committable
                && !matches!(
                    operation.transition.action(),
                    pale_core::records::SurfaceAction::Delete
                        | pale_core::records::SurfaceAction::Splice
                )
        })
        .map(|operation| operation.rel_path.clone())
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return paths;
    }
    paths.push(".pale/apply.lock".into());
    paths.push(pale_core::records::MATERIALIZATION_STATE_PATH.into());
    paths.sort_unstable();
    paths.dedup();
    paths
}

/// Rejected target retained until its owning input boundary assigns blame.
struct InvalidMaterializationTarget(String);

impl InvalidMaterializationTarget {
    /// Consume the private rejection and return its exact target.
    fn into_path(self) -> String {
        self.0
    }
}

/// Require one target to satisfy the canonical confined-path grammar.
fn validate_target(rel_path: &str) -> Result<(), InvalidMaterializationTarget> {
    pale_core::paths::SurfaceTarget::parse(rel_path)
        .map(|_| ())
        .ok_or_else(|| InvalidMaterializationTarget(rel_path.to_string()))
}

/// Preserve the current public empty deferred-effect field.
///
/// Initial-v1 has no admitted deferred-surface producer: repository hook
/// contributions are ordinary Pale-owned files under `.pale`. Preparation
/// rejects any contrary surface before this point. [`DeferredEffect`] remains
/// only because removing its public contract family requires a separate
/// contract-wide approval and regeneration.
pub(super) fn deferred_effects(prepared: &PreparedSurfaceSet<'_>) -> Vec<DeferredEffect> {
    debug_assert!(prepared.surfaces().all(|surface| !surface.deferred));
    Vec::new()
}

/// Reconstruct the exact ownership state selected by one compiled repository.
///
/// This delegates to the same state producer used by apply planning. It does not
/// inspect a target, admit a plan, or claim that the returned state was applied.
pub(crate) fn desired_materialization_state(
    compilation: &Compilation,
    repository_id: &LogicalRepositoryIdV1,
) -> Result<pale_core::records::MaterializationStateV1, PlanError> {
    let input = RepositoryMaterializationInput::for_repository(compilation, repository_id)?;
    let prepared = PreparedSurfaceSet::from_render(input.render)?;
    state::desired_state(
        input.identity,
        &prepared,
        &input.repository.id,
        input.manifest_digest,
        input.schema_registry,
        input.one_shot_compilation_digest,
    )
}

/// Deterministic materialization components awaiting final plan identity.
pub(super) struct PlanParts {
    pub(super) operations: Vec<Operation>,
    pub(super) baseline: Vec<BaselineEntry>,
    pub(super) deferred_effects: Vec<DeferredEffect>,
    pub(super) transition: MaterializationStateTransitionV1,
}

/// Exact admitted coordinates bound into a new materialization plan.
pub(super) struct PlanAssemblyInput<'a> {
    pub(super) identity: ProjectIdentity<'a>,
    pub(super) manifest_digest: &'a ManifestDigest,
    pub(super) package_closure: &'a [PackageReleaseV1],
    pub(super) normative_pack_closure: &'a [NormativePackReleaseV1],
    pub(super) repository_id: &'a LogicalRepositoryIdV1,
    pub(super) package_semantic_ir_digest: &'a PackageSemanticIrDigest,
    pub(super) schema_registry: &'a SchemaRegistryIdentity,
    pub(super) one_shot_compilation_digest: &'a OneShotCompilationDigest,
}

/// Canonically order and seal a complete v1 materialization plan.
pub(super) fn assemble(
    input: PlanAssemblyInput<'_>,
    mut parts: PlanParts,
) -> Result<AgentSurfacePlanV1, PlanError> {
    let PlanAssemblyInput {
        identity,
        manifest_digest,
        package_closure,
        normative_pack_closure,
        repository_id,
        package_semantic_ir_digest,
        schema_registry,
        one_shot_compilation_digest,
    } = input;
    parts
        .operations
        .sort_by(|left, right| left.rel_path.cmp(&right.rel_path));
    parts
        .baseline
        .sort_by(|left, right| left.rel_path.cmp(&right.rel_path));
    let schema_identity = pale_contracts::record_schema_identity("agent-surface-plan.v1")?;
    let plan = AgentSurfacePlanV1::seal(AgentSurfacePlanPreimageV1 {
        schema: schema_identity.schema,
        schema_sha256: schema_identity.schema_sha256,
        schema_manifest_sha256: schema_registry.schema_manifest_sha256.clone(),
        schema_set_sha256: schema_registry.schema_set_sha256.clone(),
        one_shot_compilation_digest: one_shot_compilation_digest.clone(),
        transition: parts.transition,
        operation_marker: OPERATION_MARKER.to_string(),
        admission: ADMISSION_UNAVAILABLE.to_string(),
        compiler_version: COMPILER_VERSION.to_string(),
        renderer_version: RENDERER_VERSION.to_string(),
        package_closure: package_closure.to_vec(),
        normative_pack_closure: normative_pack_closure.to_vec(),
        profile_digest: identity.profile().clone(),
        instance_digest: identity.instance().clone(),
        manifest_digest: manifest_digest.clone(),
        package_semantic_ir_digest: package_semantic_ir_digest.clone(),
        repository_id: repository_id.clone(),
        target_baseline: parts.baseline,
        operations: parts.operations,
        deferred_effects: parts.deferred_effects,
    })?;
    validate(&plan, "agent-surface-plan.v1")?;
    Ok(plan)
}
