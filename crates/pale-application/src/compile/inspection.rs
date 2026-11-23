//! Read-only observations derived from one sealed compilation.

use std::collections::BTreeSet;

use pale_core::records::{
    AdmittedGoverningOrderV1, CapabilityIdV1, CapabilityNarrowingV1, CapabilityProvider,
    CapabilityRequirementIdV1, CompositionReasonStepV1, EffectClass, EffectiveProjectProfileV1,
    ExecutionPathSelectionIdV1, ExecutionPathWitnessV1, ExecutionWorkspaceRepositoryAccessV1,
    ExperienceCompositionPlanV1, LogicalRepositoryIdV1, ManifestDigest, NormativePackReleaseV1,
    PackageReleaseV1, PackageResourceRefV1, ParticipantRefV1, ProfileDigest,
    ProviderRepositoryProjectionV1, RepositoryRenderDigest, ResponsibilityPathIdentityV1,
    ResponsibilityV1, RunPolicyEntry, SemanticSelection, SessionModel,
};

use crate::observation::ObservationSubject;
use crate::render::ExecutionPathSupport;
use crate::semantic::{
    ProviderUnavailability, RepositorySemanticPlan, ResolvedCapability, ResolvedOperationMechanics,
    ResolvedProviderSelection,
};

use super::{Compilation, CompilationInvariantError, CompiledRepo, EffectiveProfileCompilation};

/// Prevalidated indices and witness for one selected execution path.
pub(super) struct EffectiveProfilePathProof {
    witness: ExecutionPathWitnessV1,
    path_index: usize,
    reason_index: usize,
    responsibility_contract_indices: Vec<usize>,
}

/// Validate and index every path proof once while the compiler phase is formed.
pub(super) fn build_effective_profile_path_proofs(
    profile: &EffectiveProjectProfileV1,
    plan: &ExperienceCompositionPlanV1,
) -> Result<Vec<EffectiveProfilePathProof>, CompilationInvariantError> {
    profile
        .execution_paths
        .iter()
        .map(|selection| {
            let witness =
                ExecutionPathWitnessV1::from_compiled_plan(selection, plan).map_err(|source| {
                    CompilationInvariantError::structure(
                        "effective-profile execution-path witness",
                        source.to_string(),
                    )
                })?;
            let mut paths = plan
                .declarations
                .selected_experience_paths
                .iter()
                .enumerate()
                .filter(|(_, path)| path.id == selection.experience_path_id);
            let (path_index, path) = paths.next().ok_or_else(|| {
                CompilationInvariantError::structure(
                    "effective-profile execution path",
                    "selected path disappeared after witness construction",
                )
            })?;
            if paths.next().is_some() {
                return Err(CompilationInvariantError::structure(
                    "effective-profile execution path",
                    "selected path identity is not unique",
                ));
            }
            let mut reasons = plan
                .reason_paths
                .iter()
                .enumerate()
                .filter(|(_, reason)| reason.path_id == selection.experience_path_id);
            let (reason_index, _) = reasons.next().ok_or_else(|| {
                CompilationInvariantError::structure(
                    "effective-profile reason path",
                    "selected path has no retained causal reason path",
                )
            })?;
            if reasons.next().is_some() {
                return Err(CompilationInvariantError::structure(
                    "effective-profile reason path",
                    "selected path has more than one causal reason path",
                ));
            }
            let responsibility_contract_indices = path
                .responsibility_contract_ids
                .iter()
                .map(|contract_id| {
                    let mut contracts = plan
                        .declarations
                        .responsibility_contracts
                        .iter()
                        .enumerate()
                        .filter(|(_, contract)| contract.id == *contract_id);
                    let (contract_index, _) = contracts.next().ok_or_else(|| {
                        CompilationInvariantError::structure(
                            "effective-profile responsibility contract",
                            format!("selected path references unknown contract `{contract_id}`"),
                        )
                    })?;
                    if contracts.next().is_some() {
                        return Err(CompilationInvariantError::structure(
                            "effective-profile responsibility contract",
                            format!("selected contract identity `{contract_id}` is not unique"),
                        ));
                    }
                    Ok(contract_index)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EffectiveProfilePathProof {
                witness,
                path_index,
                reason_index,
                responsibility_contract_indices,
            })
        })
        .collect()
}

/// Borrowed application-owned facts at the effective-profile compiler seam.
///
/// This view reports only admitted configuration and compiler proof. The
/// rendered set model treats compilation as a deterministic mapping from
/// admitted intent to the effective portable profile. Machine realization is
/// a separate observation of that profile on one selected machine. The named
/// sets contain requirement occurrences and expose four non-substitutable
/// relations. In the law, `I` is admitted intent, `C` is the compiler, and `E`
/// is its effective profile. `Required`, `Available`, and `Forbidden` select
/// their named occurrence sets. `M_m(E)` is machine `m`'s observation of `E`;
/// `Unknown` selects the requirements that observation does not prove.
///
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/effective-profile-relations.display.mathml"
))]
///
/// [`EffectiveProfileRequirementDisposition`] preserves the portable
/// configuration relation, [`EffectiveProfileProviderDisposition`] preserves
/// private provider selection, and [`EffectiveProfileNativeRealization`]
/// preserves owner-native evidence. [`EffectiveProfileOperationExposure`]
/// remains a fourth independent axis. This view cannot construct the model's
/// machine realization because no machine/provider/native observation occurred.
#[derive(Clone, Copy)]
pub struct EffectiveProfileInspection<'a> {
    compilation: &'a EffectiveProfileCompilation,
}

impl<'a> EffectiveProfileInspection<'a> {
    pub(super) const fn new(compilation: &'a EffectiveProfileCompilation) -> Self {
        Self { compilation }
    }

    /// Principal workspace identity owning the admitted desired graph.
    #[must_use]
    pub fn workspace_id(self) -> &'a str {
        self.compilation.profile().workspace_id.as_str()
    }

    /// Canonical logical project selected for inspection.
    #[must_use]
    pub fn project_id(self) -> &'a str {
        self.compilation.profile().project_id.as_str()
    }

    /// Canonical execution workspace selected for inspection.
    #[must_use]
    pub fn execution_workspace_id(self) -> &'a str {
        self.compilation.profile().execution_workspace_id.as_str()
    }

    /// Canonical portable profile identity.
    #[must_use]
    pub fn profile_digest(self) -> &'a ProfileDigest {
        &self.compilation.profile().profile_digest
    }

    /// Exact selected Package release closure.
    #[must_use]
    pub fn package_closure(self) -> &'a [PackageReleaseV1] {
        &self.compilation.profile().package_closure
    }

    /// Exact selected normative-Pack closure from workspace admission.
    #[must_use]
    pub fn normative_pack_closure(self) -> &'a [NormativePackReleaseV1] {
        &self.compilation.profile().normative_pack_closure
    }

    /// Exact declared semantic selectors retained by the admitted profile.
    ///
    /// These selectors describe declared intent; effective selected resources
    /// remain available through [`EffectiveProfileRepositoryInspection::selected_resources`].
    #[must_use]
    pub fn declared_semantic_selection(self) -> &'a SemanticSelection {
        &self.compilation.profile().semantic
    }

    /// Admitted Package-owned ceiling, obligation, authority, narrowing, and order algebra.
    #[must_use]
    pub fn governing_order(self) -> &'a AdmittedGoverningOrderV1 {
        &self.compilation.profile().governing_order
    }

    /// Private capability/effect narrowing applied to Package-owned policy.
    #[must_use]
    pub fn capability_narrowing(self) -> Option<&'a CapabilityNarrowingV1> {
        self.compilation.profile().capability_narrowing.as_ref()
    }

    /// Exact selected run policy.
    #[must_use]
    pub fn run_policy(self) -> &'a RunPolicyEntry {
        &self.compilation.profile().run_policy
    }

    /// Selected lifecycle behavior retained by the portable profile.
    #[must_use]
    pub fn lifecycle(self) -> &'a [pale_core::records::AdmittedLifecycleBehaviorV1] {
        &self.compilation.profile().lifecycle
    }

    /// Typed session model, when declared by the execution workspace.
    #[must_use]
    pub fn session(self) -> Option<&'a SessionModel> {
        self.compilation.profile().session.as_ref()
    }

    /// Logical authority roots selected by portable configuration.
    #[must_use]
    pub fn authority_root_ids(self) -> &'a [String] {
        &self.compilation.profile().authority_root_ids
    }

    /// Volatile forbidden-token authority coordinates selected by policy.
    #[must_use]
    pub fn forbidden_token_authority(self) -> &'a [String] {
        &self.compilation.profile().forbidden_token_authority
    }

    /// Current machine realization status at this pre-observation phase.
    #[must_use]
    pub fn native_realization(self) -> EffectiveProfileNativeRealization<'a> {
        EffectiveProfileNativeRealization::Unknown {
            machine_id: &self.compilation.instance().machine_id,
        }
    }

    /// Repository views in exact effective materialization order.
    pub fn repositories(
        self,
    ) -> impl ExactSizeIterator<Item = EffectiveProfileRepositoryInspection<'a>> {
        self.compilation
            .profile()
            .repositories
            .iter()
            .zip(self.compilation.repository_semantic_plans())
            .enumerate()
            .map(move |(materialization_order, (repository, semantics))| {
                EffectiveProfileRepositoryInspection {
                    materialization_order,
                    repository,
                    semantics,
                    capability_narrowing: self.capability_narrowing(),
                }
            })
    }

    /// Selected provider-native repository projections from private configuration.
    ///
    /// Each row has already passed the active execution workspace's repository
    /// and provider-binding selection fence during instance resolution. The
    /// opaque native coordinate is admitted configuration only; availability,
    /// authorization, and owner-native realization remain unknown here.
    pub fn provider_repositories(
        self,
    ) -> impl ExactSizeIterator<Item = EffectiveProfileProviderRepositoryInspection<'a>> {
        self.compilation
            .instance()
            .provider_repositories
            .iter()
            .map(
                move |projection| EffectiveProfileProviderRepositoryInspection {
                    projection,
                    machine_id: &self.compilation.instance().machine_id,
                },
            )
    }

    /// Selected execution paths joined to their prevalidated compiler proof.
    ///
    /// Path lookup and witness validation run once when the retained phase is
    /// formed. Repeated editor queries therefore traverse only stable indices;
    /// they do not reconstruct closure or revalidate the composition graph.
    pub fn execution_paths(
        self,
    ) -> impl ExactSizeIterator<Item = EffectiveProfileExecutionPathInspection<'a>> {
        self.compilation
            .path_proofs
            .iter()
            .map(|proof| EffectiveProfileExecutionPathInspection {
                proof,
                path: &self
                    .compilation
                    .experience_composition_plan()
                    .declarations
                    .selected_experience_paths[proof.path_index],
                reasons: &self.compilation.experience_composition_plan().reason_paths
                    [proof.reason_index]
                    .reasons,
                responsibility_contracts: &self
                    .compilation
                    .experience_composition_plan()
                    .declarations
                    .responsibility_contracts,
                machine_id: &self.compilation.instance().machine_id,
            })
    }

    /// Exact typed dispositions worth presenting as profile findings.
    ///
    /// These findings never infer an undeclared requirement. An unbound
    /// operation and each provider-resolution failure are reported because the
    /// canonical semantic plan proves that state; they are not relabeled as a
    /// recommendation or global readiness failure.
    #[must_use]
    pub fn findings(self) -> Vec<EffectiveProfileFinding<'a>> {
        let mut findings = vec![EffectiveProfileFinding::NativeRealizationUnknown {
            machine_id: &self.compilation.instance().machine_id,
        }];
        for repository in self.repositories() {
            for operation in repository.operations() {
                match &operation.operation.mechanics {
                    ResolvedOperationMechanics::Unbound => {
                        findings.push(EffectiveProfileFinding::OperationUnbound {
                            repository_id: repository.repository_id(),
                            operation_id: &operation.operation.record.id,
                        });
                    }
                    ResolvedOperationMechanics::SemanticOnly { .. } => {}
                    ResolvedOperationMechanics::CapabilityBearing { capabilities, .. } => {
                        for capability in capabilities {
                            let requirement = EffectiveProfileRequirementInspection {
                                requirement: capability,
                                capability_narrowing: self.capability_narrowing(),
                                repository_id: repository.repository_id(),
                                operation_id: operation.operation_id(),
                            };
                            findings.push(EffectiveProfileFinding::RequirementDisposition {
                                repository_id: repository.repository_id(),
                                operation_id: &operation.operation.record.id,
                                requirement_id: &capability.requirement_id,
                                capability_id: &capability.capability_id,
                                disposition: requirement.disposition(),
                            });
                        }
                    }
                }
            }
        }
        findings
    }
}

/// Machine/native realization state available before machine observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectiveProfileNativeRealization<'a> {
    /// A machine was selected, but no owner-native evidence has been observed.
    Unknown {
        /// Selected private machine-binding identity.
        machine_id: &'a str,
    },
}

impl EffectiveProfileNativeRealization<'_> {
    /// Exact configured machine, without claiming native observation.
    #[must_use]
    pub fn observation_subject(self) -> ObservationSubject {
        match self {
            Self::Unknown { machine_id } => ObservationSubject::Machine(machine_id.to_owned()),
        }
    }
}

/// One selected private provider projection over a canonical repository.
#[derive(Clone, Copy)]
pub struct EffectiveProfileProviderRepositoryInspection<'a> {
    projection: &'a ProviderRepositoryProjectionV1,
    machine_id: &'a str,
}

impl<'a> EffectiveProfileProviderRepositoryInspection<'a> {
    /// Canonical desired repository selected by the execution workspace.
    #[must_use]
    pub fn repository_id(self) -> &'a LogicalRepositoryIdV1 {
        &self.projection.repository_id
    }

    /// Exact neutral provider-binding identity selected by an experience path.
    #[must_use]
    pub fn provider_binding_id(self) -> &'a str {
        &self.projection.provider_binding_id
    }

    /// Opaque repository coordinate issued and interpreted by the provider.
    #[must_use]
    pub fn native_repository_coordinate(self) -> &'a str {
        &self.projection.native_repository_coordinate
    }

    /// Machine/native readiness at this pre-observation compiler phase.
    #[must_use]
    pub const fn native_realization(self) -> EffectiveProfileNativeRealization<'a> {
        EffectiveProfileNativeRealization::Unknown {
            machine_id: self.machine_id,
        }
    }
}

/// One repository's admitted access ceiling and semantic projection.
#[derive(Clone, Copy)]
pub struct EffectiveProfileRepositoryInspection<'a> {
    materialization_order: usize,
    repository: &'a pale_core::records::ProfileRepository,
    semantics: &'a RepositorySemanticPlan,
    capability_narrowing: Option<&'a CapabilityNarrowingV1>,
}

impl<'a> EffectiveProfileRepositoryInspection<'a> {
    /// Primary observation subject of this repository, not its selected resources.
    #[must_use]
    pub fn observation_subject(self) -> ObservationSubject {
        ObservationSubject::Repository(self.repository.id.clone())
    }

    /// Canonical logical repository identity.
    #[must_use]
    pub fn repository_id(self) -> &'a LogicalRepositoryIdV1 {
        &self.repository.id
    }

    /// Zero-based order used by later materialization planning.
    #[must_use]
    pub const fn materialization_order(self) -> usize {
        self.materialization_order
    }

    /// Maximum read/mutate access admitted by the execution-workspace edge.
    #[must_use]
    pub const fn access_ceiling(self) -> ExecutionWorkspaceRepositoryAccessV1 {
        self.repository.access
    }

    /// Exact Package-owned repository role selected for this repository.
    #[must_use]
    pub fn role(self) -> &'a PackageResourceRefV1 {
        &self.repository.role
    }

    /// Explicit stack overlays selected for this repository.
    #[must_use]
    pub fn stack_overlays(self) -> &'a [PackageResourceRefV1] {
        &self.repository.stacks
    }

    /// Exact Package-owned policy-family selections for this repository.
    #[must_use]
    pub fn policy_families(self) -> &'a [PackageResourceRefV1] {
        &self.repository.families
    }

    /// Selected operations with their closed exposure/provider dispositions.
    pub fn operations(
        self,
    ) -> impl ExactSizeIterator<Item = EffectiveProfileOperationInspection<'a>> {
        self.semantics
            .operations
            .iter()
            .map(move |operation| EffectiveProfileOperationInspection {
                operation,
                capability_narrowing: self.capability_narrowing,
                repository_id: self.repository_id(),
            })
    }

    /// Selected source-free semantic identities grouped by exact semantic kind.
    #[must_use]
    pub fn selected_resources(self) -> Vec<EffectiveProfileSelectedResource<'a>> {
        let mut selected = Vec::with_capacity(
            self.semantics.operations.len()
                + self.semantics.roles.len()
                + self.semantics.doctrine.len()
                + self.semantics.startup_policies.len()
                + self.semantics.run_modes.len()
                + self.semantics.stacks.len()
                + self.semantics.policy_families.len()
                + self.semantics.recipes.len(),
        );
        selected.extend(self.semantics.operations.iter().map(|value| {
            EffectiveProfileSelectedResource::new(
                EffectiveProfileResourceKind::Operation,
                &value.record.id,
            )
        }));
        selected.extend(self.semantics.roles.iter().map(|value| {
            EffectiveProfileSelectedResource::new(
                EffectiveProfileResourceKind::Role,
                &value.record.id,
            )
        }));
        selected.extend(self.semantics.doctrine.iter().map(|value| {
            EffectiveProfileSelectedResource::new(
                EffectiveProfileResourceKind::Doctrine,
                &value.record.id,
            )
        }));
        selected.extend(self.semantics.startup_policies.iter().map(|value| {
            EffectiveProfileSelectedResource::new(
                EffectiveProfileResourceKind::StartupPolicy,
                &value.record.id,
            )
        }));
        selected.extend(self.semantics.run_modes.iter().map(|value| {
            EffectiveProfileSelectedResource::new(
                EffectiveProfileResourceKind::RunMode,
                &value.record.id,
            )
        }));
        selected.extend(self.semantics.stacks.iter().map(|value| {
            EffectiveProfileSelectedResource::new(EffectiveProfileResourceKind::Stack, &value.id)
        }));
        selected.extend(self.semantics.policy_families.iter().map(|value| {
            EffectiveProfileSelectedResource::new(
                EffectiveProfileResourceKind::PolicyFamily,
                &value.id,
            )
        }));
        selected.extend(self.semantics.recipes.iter().map(|value| {
            EffectiveProfileSelectedResource::new(
                EffectiveProfileResourceKind::Recipe,
                &value.resource,
            )
        }));
        selected
    }
}

/// Closed semantic family of one selected Package resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectiveProfileResourceKind {
    /// Operation procedure.
    Operation,
    /// Specialist role.
    Role,
    /// Standalone doctrine.
    Doctrine,
    /// Doctrine placed into startup policy.
    StartupPolicy,
    /// Run-mode declaration.
    RunMode,
    /// Technology or workflow stack.
    Stack,
    /// Policy-family body.
    PolicyFamily,
    /// Reusable Package recipe.
    Recipe,
}

/// One selected owner-qualified semantic resource.
#[derive(Clone, Copy, Debug)]
pub struct EffectiveProfileSelectedResource<'a> {
    kind: EffectiveProfileResourceKind,
    resource: &'a PackageResourceRefV1,
}

impl<'a> EffectiveProfileSelectedResource<'a> {
    const fn new(kind: EffectiveProfileResourceKind, resource: &'a PackageResourceRefV1) -> Self {
        Self { kind, resource }
    }

    /// Exact semantic family.
    #[must_use]
    pub const fn kind(self) -> EffectiveProfileResourceKind {
        self.kind
    }

    /// Owner-qualified Package resource identity.
    #[must_use]
    pub const fn resource(self) -> &'a PackageResourceRefV1 {
        self.resource
    }
}

/// Closed private exposure state of one selected semantic operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectiveProfileOperationExposure {
    /// No private operation binding exists.
    Unbound,
    /// A binding exists and the Package operation declares no requirements.
    SemanticOnly,
    /// A binding exists and Package-owned capability requirements were joined.
    CapabilityBearing,
}

/// One selected operation and its independent requirement/provider joins.
#[derive(Clone, Copy)]
pub struct EffectiveProfileOperationInspection<'a> {
    operation: &'a crate::semantic::ResolvedOperation,
    capability_narrowing: Option<&'a CapabilityNarrowingV1>,
    repository_id: &'a LogicalRepositoryIdV1,
}

impl<'a> EffectiveProfileOperationInspection<'a> {
    /// Repository-qualified operation occurrence, independent of exposure state.
    #[must_use]
    pub fn observation_subject(self) -> ObservationSubject {
        ObservationSubject::Operation {
            repository_id: self.repository_id.clone(),
            operation_id: self.operation_id().clone(),
        }
    }

    /// Owner-qualified Package operation identity.
    #[must_use]
    pub fn operation_id(self) -> &'a PackageResourceRefV1 {
        &self.operation.record.id
    }

    /// Exact three-state private exposure classification.
    #[must_use]
    pub fn exposure(self) -> EffectiveProfileOperationExposure {
        match &self.operation.mechanics {
            ResolvedOperationMechanics::Unbound => EffectiveProfileOperationExposure::Unbound,
            ResolvedOperationMechanics::SemanticOnly { .. } => {
                EffectiveProfileOperationExposure::SemanticOnly
            }
            ResolvedOperationMechanics::CapabilityBearing { .. } => {
                EffectiveProfileOperationExposure::CapabilityBearing
            }
        }
    }

    /// Capability requirements, present only for capability-bearing operations.
    pub fn requirements(
        self,
    ) -> impl ExactSizeIterator<Item = EffectiveProfileRequirementInspection<'a>> {
        self.operation
            .mechanics
            .capabilities()
            .iter()
            .map(move |requirement| EffectiveProfileRequirementInspection {
                requirement,
                capability_narrowing: self.capability_narrowing,
                repository_id: self.repository_id,
                operation_id: self.operation_id(),
            })
    }
}

/// Provider-selection state independent of operation exposure and native readiness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectiveProfileProviderDisposition {
    /// No usable private provider was resolved for the stated reason.
    Unbound(ProviderUnavailability),
    /// Private configuration selected one provider kind; availability is unknown.
    Bound(CapabilityProvider),
}

/// Portable requirement/configuration result, independent of native observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectiveProfileRequirementDisposition {
    /// A usable private provider binding satisfies the portable requirement.
    Satisfied,
    /// The requirement is absent from a positive private permitted set.
    Narrowed(EffectiveProfileNarrowingReason),
    /// The required capability has no private provider binding.
    ProviderBindingAbsent,
    /// An explicit private denial or present binding contradicts policy.
    PolicyBlocked(EffectiveProfilePolicyBlock),
}

/// Exact policy dimension that rejects a present private provider binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectiveProfilePolicyBlock {
    /// Private configuration explicitly forbids the required capability.
    CapabilityForbidden,
    /// Private configuration explicitly forbids the required effect.
    EffectForbidden,
    /// Private configuration explicitly forbids both required dimensions.
    CapabilityAndEffectForbidden,
    /// Bound provider effect does not equal the Package requirement effect.
    ProviderEffectMismatch,
    /// Bound provider does not implement the Package recipe contract.
    RecipeContractMismatch,
}

/// Exact positive-set dimension that excluded one requirement occurrence.
///
/// This reason is a lossless projection of the admitted
/// [`CapabilityNarrowingV1`], not a second provider resolver. Explicit members
/// of a forbidden set are represented by [`EffectiveProfilePolicyBlock`]
/// instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectiveProfileNarrowingReason {
    /// The capability is absent from `permitted_capabilities`.
    CapabilityNotPermitted,
    /// The effect is absent from `permitted_effects`.
    EffectNotPermitted,
    /// Both positive permitted sets omit the requirement dimensions.
    CapabilityAndEffectNotPermitted,
}

/// One Package requirement joined to private provider configuration.
#[derive(Clone, Copy)]
pub struct EffectiveProfileRequirementInspection<'a> {
    requirement: &'a ResolvedCapability,
    capability_narrowing: Option<&'a CapabilityNarrowingV1>,
    repository_id: &'a LogicalRepositoryIdV1,
    operation_id: &'a PackageResourceRefV1,
}

impl<'a> EffectiveProfileRequirementInspection<'a> {
    /// Complete requirement occurrence; local requirement IDs are not global keys.
    #[must_use]
    pub fn observation_subject(self) -> ObservationSubject {
        ObservationSubject::CapabilityRequirement {
            repository_id: self.repository_id.clone(),
            operation_id: self.operation_id.clone(),
            requirement_id: self.requirement_id().clone(),
            capability_id: self.capability_id().clone(),
        }
    }

    /// Requirement identity local to the owning operation.
    #[must_use]
    pub fn requirement_id(self) -> &'a CapabilityRequirementIdV1 {
        &self.requirement.requirement_id
    }

    /// Portable capability identity required by Package meaning.
    #[must_use]
    pub fn capability_id(self) -> &'a CapabilityIdV1 {
        &self.requirement.capability_id
    }

    /// Maximum effect class declared by the Package requirement.
    #[must_use]
    pub const fn effect_class(self) -> EffectClass {
        self.requirement.effect_class
    }

    /// Private provider disposition without claiming that provider execution is available.
    #[must_use]
    pub fn provider(self) -> EffectiveProfileProviderDisposition {
        match &self.requirement.provider {
            ResolvedProviderSelection::Unbound(reason) => {
                EffectiveProfileProviderDisposition::Unbound(*reason)
            }
            ResolvedProviderSelection::Bound(binding) => {
                EffectiveProfileProviderDisposition::Bound(binding.provider())
            }
        }
    }

    /// Classify portable configuration without claiming native realization.
    ///
    /// # Panics
    ///
    /// Panics only if compiler-owned state says the requirement was excluded
    /// by narrowing while the retained admitted narrowing sets contain neither
    /// a forbidden member nor a positive-set omission. Such state contradicts
    /// the canonical semantic projection predicate.
    #[must_use]
    pub fn disposition(self) -> EffectiveProfileRequirementDisposition {
        match &self.requirement.provider {
            ResolvedProviderSelection::Bound(_) => {
                EffectiveProfileRequirementDisposition::Satisfied
            }
            ResolvedProviderSelection::Unbound(ProviderUnavailability::MissingBinding) => {
                EffectiveProfileRequirementDisposition::ProviderBindingAbsent
            }
            ResolvedProviderSelection::Unbound(ProviderUnavailability::ExcludedByNarrowing) => {
                self.narrowing_disposition().expect(
                    "canonical provider resolution records narrowing only when its admitted sets exclude the requirement",
                )
            }
            ResolvedProviderSelection::Unbound(ProviderUnavailability::EffectMismatch) => {
                EffectiveProfileRequirementDisposition::PolicyBlocked(
                    EffectiveProfilePolicyBlock::ProviderEffectMismatch,
                )
            }
            ResolvedProviderSelection::Unbound(ProviderUnavailability::RecipeContractMismatch) => {
                EffectiveProfileRequirementDisposition::PolicyBlocked(
                    EffectiveProfilePolicyBlock::RecipeContractMismatch,
                )
            }
        }
    }

    /// Project the exact admitted narrowing set relation for this requirement.
    fn narrowing_disposition(self) -> Option<EffectiveProfileRequirementDisposition> {
        let narrowing = self.capability_narrowing?;
        let capability_forbidden = narrowing
            .forbidden_capabilities
            .contains(&self.requirement.capability_id);
        let effect_forbidden = narrowing
            .forbidden_effects
            .contains(&self.requirement.effect_class);
        match (capability_forbidden, effect_forbidden) {
            (true, true) => {
                return Some(EffectiveProfileRequirementDisposition::PolicyBlocked(
                    EffectiveProfilePolicyBlock::CapabilityAndEffectForbidden,
                ));
            }
            (true, false) => {
                return Some(EffectiveProfileRequirementDisposition::PolicyBlocked(
                    EffectiveProfilePolicyBlock::CapabilityForbidden,
                ));
            }
            (false, true) => {
                return Some(EffectiveProfileRequirementDisposition::PolicyBlocked(
                    EffectiveProfilePolicyBlock::EffectForbidden,
                ));
            }
            (false, false) => {}
        }

        let capability_not_permitted = !narrowing
            .permitted_capabilities
            .contains(&self.requirement.capability_id);
        let effect_not_permitted = !narrowing
            .permitted_effects
            .contains(&self.requirement.effect_class);
        match (capability_not_permitted, effect_not_permitted) {
            (true, true) => Some(EffectiveProfileRequirementDisposition::Narrowed(
                EffectiveProfileNarrowingReason::CapabilityAndEffectNotPermitted,
            )),
            (true, false) => Some(EffectiveProfileRequirementDisposition::Narrowed(
                EffectiveProfileNarrowingReason::CapabilityNotPermitted,
            )),
            (false, true) => Some(EffectiveProfileRequirementDisposition::Narrowed(
                EffectiveProfileNarrowingReason::EffectNotPermitted,
            )),
            (false, false) => None,
        }
    }
}

/// One execution-path selection with exact responsibility and ceiling evidence.
#[derive(Clone, Copy)]
pub struct EffectiveProfileExecutionPathInspection<'a> {
    proof: &'a EffectiveProfilePathProof,
    path: &'a pale_core::records::ExperiencePathV1,
    reasons: &'a [CompositionReasonStepV1],
    responsibility_contracts: &'a [pale_core::records::ResponsibilityContractV1],
    machine_id: &'a str,
}

impl<'a> EffectiveProfileExecutionPathInspection<'a> {
    /// Exact path selection, not every responsibility carried by that path.
    #[must_use]
    pub fn observation_subject(self) -> ObservationSubject {
        ObservationSubject::ExecutionPath(self.selection_id().clone())
    }

    /// Stable execution-path selection identity.
    #[must_use]
    pub fn selection_id(self) -> &'a ExecutionPathSelectionIdV1 {
        self.proof.witness.selection.id()
    }

    /// Exact selected experience-path identity.
    #[must_use]
    pub fn experience_path_id(self) -> &'a str {
        &self.path.id
    }

    /// Exact provider binding selected at the terminal-leaf boundary.
    #[must_use]
    pub const fn terminal_provider_binding(
        self,
    ) -> Option<&'a pale_core::records::ProviderBindingRefV1> {
        self.proof.witness.terminal_provider_binding.as_ref()
    }

    /// Domain-separated identity of the complete responsibility path.
    #[must_use]
    pub fn responsibility_path_identity(self) -> &'a ResponsibilityPathIdentityV1 {
        &self.proof.witness.responsibility_path_identity
    }

    /// Exact responsibility contracts used by the path.
    #[must_use]
    pub fn responsibility_contract_ids(self) -> &'a [String] {
        &self.path.responsibility_contract_ids
    }

    /// Selected responsibility contracts joined once during phase admission.
    ///
    /// Selection proves that the contract participates in this path. It does
    /// not by itself prove provider completion, native readiness, or successful
    /// realization of any typed responsibility.
    pub fn responsibility_contracts(
        self,
    ) -> impl ExactSizeIterator<Item = EffectiveProfileResponsibilityContractInspection<'a>> {
        self.proof
            .responsibility_contract_indices
            .iter()
            .map(|index| EffectiveProfileResponsibilityContractInspection {
                contract: &self.responsibility_contracts[*index],
            })
    }

    /// Ordered neutral delegation-edge identities traversed by the path.
    #[must_use]
    pub fn work_delegation_edge_ids(self) -> &'a [String] {
        &self.proof.witness.work_delegation_edge_ids
    }

    /// Explicit provider-binding declarations required by the path.
    #[must_use]
    pub fn provider_binding_ids(self) -> &'a [String] {
        &self.path.provider_binding_ids
    }

    /// Capability ceiling admitted by the exact path.
    #[must_use]
    pub fn capability_ceiling(self) -> &'a [String] {
        &self.path.capability_ceiling
    }

    /// Effect ceiling admitted by the exact path.
    #[must_use]
    pub fn effect_ceiling(self) -> &'a [EffectClass] {
        &self.path.effect_ceiling
    }

    /// Compiler-owned causal steps proving this selected path.
    #[must_use]
    pub const fn reason_steps(self) -> &'a [CompositionReasonStepV1] {
        self.reasons
    }

    /// Machine/native readiness at this pre-observation compiler phase.
    #[must_use]
    pub const fn native_realization(self) -> EffectiveProfileNativeRealization<'a> {
        EffectiveProfileNativeRealization::Unknown {
            machine_id: self.machine_id,
        }
    }
}

/// One selected responsibility contract and its exact admitted authority ceiling.
#[derive(Clone, Copy)]
pub struct EffectiveProfileResponsibilityContractInspection<'a> {
    contract: &'a pale_core::records::ResponsibilityContractV1,
}

impl<'a> EffectiveProfileResponsibilityContractInspection<'a> {
    /// Stable responsibility-contract identity.
    #[must_use]
    pub fn contract_id(self) -> &'a str {
        &self.contract.id
    }

    /// Participant whose responsibility is bounded by this contract.
    #[must_use]
    pub const fn participant(self) -> &'a ParticipantRefV1 {
        &self.contract.participant
    }

    /// Typed responsibilities assigned by the semantic owner.
    #[must_use]
    pub fn responsibilities(self) -> &'a [ResponsibilityV1] {
        &self.contract.responsibilities
    }

    /// Maximum effect classes admitted by this selected contract.
    #[must_use]
    pub fn authority_ceiling(self) -> &'a [EffectClass] {
        &self.contract.authority_ceiling
    }
}

/// Exact non-heuristic dispositions derived from the retained semantic plan.
#[derive(Clone, Copy, Debug)]
pub enum EffectiveProfileFinding<'a> {
    /// No owner-native observation has qualified the selected machine.
    NativeRealizationUnknown {
        /// Selected private machine-binding identity.
        machine_id: &'a str,
    },
    /// One selected semantic operation has no private operation binding.
    OperationUnbound {
        /// Repository whose semantic plan contains the operation.
        repository_id: &'a LogicalRepositoryIdV1,
        /// Owner-qualified Package operation identity.
        operation_id: &'a PackageResourceRefV1,
    },
    /// One Package requirement's exact portable configuration disposition.
    RequirementDisposition {
        /// Repository whose semantic plan contains the operation.
        repository_id: &'a LogicalRepositoryIdV1,
        /// Owner-qualified Package operation identity.
        operation_id: &'a PackageResourceRefV1,
        /// Requirement identity local to the operation.
        requirement_id: &'a CapabilityRequirementIdV1,
        /// Portable capability identity.
        capability_id: &'a CapabilityIdV1,
        /// Exact application-owned requirement/configuration classification.
        disposition: EffectiveProfileRequirementDisposition,
    },
}

impl EffectiveProfileFinding<'_> {
    /// Primary observation subject whose canonical disposition produced this finding.
    #[must_use]
    pub fn observation_subject(self) -> ObservationSubject {
        match self {
            Self::NativeRealizationUnknown { machine_id } => {
                ObservationSubject::Machine(machine_id.to_owned())
            }
            Self::OperationUnbound {
                repository_id,
                operation_id,
            } => ObservationSubject::Operation {
                repository_id: repository_id.clone(),
                operation_id: operation_id.clone(),
            },
            Self::RequirementDisposition {
                repository_id,
                operation_id,
                requirement_id,
                capability_id,
                ..
            } => ObservationSubject::CapabilityRequirement {
                repository_id: repository_id.clone(),
                operation_id: operation_id.clone(),
                requirement_id: requirement_id.clone(),
                capability_id: capability_id.clone(),
            },
        }
    }
}

/// Borrowed compiler-owned facts used by outer inspection projections.
///
/// This view performs no serialization and defines no public wire shape. It
/// keeps cardinality, readiness, and generated-surface derivation in the
/// application layer so API and CLI projections cannot become independent
/// semantic owners.
#[derive(Clone, Copy)]
pub struct CompilationInspection<'a> {
    compilation: &'a Compilation,
}

impl<'a> CompilationInspection<'a> {
    pub(super) const fn new(compilation: &'a Compilation) -> Self {
        Self { compilation }
    }

    /// Canonical identity of the admitted portable profile.
    #[must_use]
    pub fn profile_digest(self) -> &'a ProfileDigest {
        &self.compilation.profile().profile_digest
    }

    /// Selected execution-path identities in their admitted order.
    pub fn execution_path_ids(
        self,
    ) -> impl ExactSizeIterator<Item = &'a ExecutionPathSelectionIdV1> {
        self.compilation
            .profile()
            .execution_paths
            .iter()
            .map(pale_core::records::ExecutionPathSelectionV1::id)
    }

    /// Number of repository projections sealed by this compilation.
    #[must_use]
    pub fn repository_count(self) -> usize {
        self.compilation.repositories().len()
    }

    /// Repository observations in validated materialization order.
    pub fn repositories(self) -> impl ExactSizeIterator<Item = RepositoryInspection<'a>> {
        self.compilation
            .repositories()
            .iter()
            .map(|repository| RepositoryInspection {
                compilation: self.compilation,
                repository,
            })
    }
}

/// Borrowed compiler-owned facts for one repository projection.
///
/// Counts are computed from the retained semantic plan and render, never from
/// generated JSON, Markdown, or terminal output. The view authorizes no
/// materialization and makes no activation or native-effect claim.
#[derive(Clone, Copy)]
pub struct RepositoryInspection<'a> {
    compilation: &'a Compilation,
    repository: &'a CompiledRepo,
}

impl<'a> RepositoryInspection<'a> {
    /// Canonical logical repository identity.
    #[must_use]
    pub fn repository_id(self) -> &'a LogicalRepositoryIdV1 {
        self.repository.id()
    }

    /// Manifest identity sealing the repository's materializable surfaces.
    #[must_use]
    pub fn manifest_digest(self) -> &'a ManifestDigest {
        &self.repository.manifest().manifest_digest
    }

    /// Render identity sealing all repository projections, when retained.
    #[must_use]
    pub fn render_digest(self) -> Option<&'a RepositoryRenderDigest> {
        self.compilation
            .repository_render_digest(self.repository.id())
    }

    /// Number of generated filesystem and logical-artifact surfaces.
    #[must_use]
    pub fn surface_count(self) -> usize {
        self.repository.render().surfaces.len()
    }

    /// Number of non-deferred filesystem targets.
    #[must_use]
    pub fn materialized_surface_count(self) -> usize {
        self.repository
            .render()
            .surfaces
            .iter()
            .filter(|surface| {
                !surface.deferred && crate::plan::is_filesystem_target(&surface.rel_path)
            })
            .count()
    }

    /// Number of surfaces whose placement remains deferred.
    #[must_use]
    pub fn deferred_surface_count(self) -> usize {
        self.repository
            .render()
            .surfaces
            .iter()
            .filter(|surface| surface.deferred)
            .count()
    }

    /// Number of surfaces eligible for repository history when materialized.
    #[must_use]
    pub fn committable_surface_count(self) -> usize {
        self.repository
            .render()
            .surfaces
            .iter()
            .filter(|surface| surface.committable)
            .count()
    }

    /// Generated target paths in stable lexical order.
    #[must_use]
    pub fn surface_paths(self) -> Vec<&'a str> {
        let mut paths = self
            .repository
            .render()
            .surfaces
            .iter()
            .map(|surface| surface.rel_path.as_str())
            .collect::<Vec<_>>();
        paths.sort_unstable();
        paths
    }

    /// Complete retained execution-path support rows.
    #[must_use]
    pub fn execution_paths(self) -> &'a [ExecutionPathSupport] {
        &self.repository.render().support
    }

    /// Activation-selected execution-path identities in stable compiler order.
    #[must_use]
    pub fn activation_selected_execution_path_ids(self) -> Vec<&'a ExecutionPathSelectionIdV1> {
        self.execution_paths()
            .iter()
            .filter(|support| support.activation_selected)
            .map(|support| support.execution_path_witness.selection.id())
            .collect()
    }

    /// Deduplicated hook events rendered for activation-selected execution paths.
    #[must_use]
    pub fn activation_selected_hook_events(self) -> BTreeSet<&'static str> {
        self.execution_paths()
            .iter()
            .filter(|support| support.activation_selected)
            .flat_map(|support| {
                support
                    .hook_requirements
                    .events()
                    .iter()
                    .map(|event| event.as_str())
            })
            .collect()
    }

    /// Number of selected semantic operations.
    #[must_use]
    pub fn operation_count(self) -> usize {
        self.repository.semantic_plan().operations.len()
    }

    /// Number of selected specialist roles.
    #[must_use]
    pub fn role_count(self) -> usize {
        self.repository.semantic_plan().roles.len()
    }

    /// Number of selected standalone doctrine records.
    #[must_use]
    pub fn doctrine_count(self) -> usize {
        self.repository.semantic_plan().doctrine.len()
    }

    /// Number of selected run modes.
    #[must_use]
    pub fn run_mode_count(self) -> usize {
        self.repository.semantic_plan().run_modes.len()
    }

    /// Number of selected technology or workflow stacks.
    #[must_use]
    pub fn stack_count(self) -> usize {
        self.repository.semantic_plan().stacks.len()
    }

    /// Number of content-bearing policy families.
    #[must_use]
    pub fn policy_family_count(self) -> usize {
        self.repository.semantic_plan().policy_families.len()
    }

    /// Number of selected reusable Package recipes.
    #[must_use]
    pub fn recipe_count(self) -> usize {
        self.repository.semantic_plan().recipes.len()
    }
}

#[cfg(test)]
#[path = "../../tests/internal/compile/effective_profile_inspection.rs"]
mod tests;
