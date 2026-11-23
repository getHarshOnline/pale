//! Move a proposed configuration change through inspection, explanation,
//! proposal and commit, with a person governing the outcome.
//!
//! This module builds on `compile` and `workspace`; no other module in this
//! crate builds on it, and `pale-sdk` exposes it. It composes existing compiler
//! owners into one consuming state machine. It does not parse a second
//! language, infer intent, run an agent, mutate a Pale Workspace, or authorize a
//! native effect. Interface-specific editor, prompt, task, and conversation
//! coordinates remain outside this lifecycle and must be fenced by their native
//! owners.

use std::sync::Arc;

use crate::compile::{
    compile_captured_effective_profile_candidate_inputs, CapturedPrivateWorkspaceInputs,
    EffectiveProfileCandidateError, EffectiveProfileCompilation,
};
use crate::workspace::{
    compile_portable_workspace, CapturedWorkspaceSource, PortableWorkspaceDerivation,
    PortableWorkspaceFailure, PortableWorkspaceUniverse,
};

/// Exact admitted workspace frontier inspected before a proposal is formed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigurationImprovementBaseline {
    workspace_id: String,
    principal_universe_digest: String,
    workspace_semantic_ir_digest: String,
    workspace_admission_evidence_digest: String,
}

impl ConfigurationImprovementBaseline {
    fn from_universe(universe: &PortableWorkspaceUniverse) -> Self {
        Self {
            workspace_id: universe.workspace_id().to_owned(),
            principal_universe_digest: universe.principal_universe_digest().to_owned(),
            workspace_semantic_ir_digest: universe.semantic_ir_digest().to_owned(),
            workspace_admission_evidence_digest: universe.admission_evidence_digest().to_owned(),
        }
    }

    /// Principal workspace owning the inspected configuration.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    /// Principal-relative identity of the exact admitted workspace universe.
    #[must_use]
    pub fn principal_universe_digest(&self) -> &str {
        &self.principal_universe_digest
    }

    /// Verified semantic-IR identity of the inspected workspace.
    #[must_use]
    pub fn workspace_semantic_ir_digest(&self) -> &str {
        &self.workspace_semantic_ir_digest
    }

    /// Exact owner-proof frontier consumed by workspace admission.
    #[must_use]
    pub fn workspace_admission_evidence_digest(&self) -> &str {
        &self.workspace_admission_evidence_digest
    }
}

/// Exact profile and proof frontier used to explain the inspected baseline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigurationImprovementProfile {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    selected_topology_digest: String,
    profile_digest: String,
    instance_digest: String,
    package_semantic_ir_digest: String,
    composition_plan_digest: String,
}

impl ConfigurationImprovementProfile {
    fn from_compilation(compilation: &EffectiveProfileCompilation) -> Self {
        Self {
            project_id: compilation.profile().project_id.to_string(),
            execution_workspace_id: compilation.profile().execution_workspace_id.to_string(),
            machine_id: compilation.instance().machine_id.clone(),
            selected_topology_digest: compilation.selected_topology_digest().to_owned(),
            profile_digest: compilation.profile().profile_digest.to_string(),
            instance_digest: compilation.instance().instance_digest.to_string(),
            package_semantic_ir_digest: compilation
                .profile()
                .package_semantic_ir_digest
                .to_string(),
            composition_plan_digest: compilation
                .experience_composition_plan()
                .plan_digest
                .to_string(),
        }
    }

    /// Canonical project selected for explanation.
    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Canonical execution workspace selected for explanation.
    #[must_use]
    pub fn execution_workspace_id(&self) -> &str {
        &self.execution_workspace_id
    }

    /// Private machine selector used to derive the instance projection.
    #[must_use]
    pub fn machine_id(&self) -> &str {
        &self.machine_id
    }

    /// Digest of the exact project-relative desired topology selected for compilation.
    #[must_use]
    pub fn selected_topology_digest(&self) -> &str {
        &self.selected_topology_digest
    }

    /// Digest of the complete portable effective profile.
    #[must_use]
    pub fn profile_digest(&self) -> &str {
        &self.profile_digest
    }

    /// Digest of the admitted private machine projection.
    #[must_use]
    pub fn instance_digest(&self) -> &str {
        &self.instance_digest
    }

    /// Digest of the selected Package semantic closure.
    #[must_use]
    pub fn package_semantic_ir_digest(&self) -> &str {
        &self.package_semantic_ir_digest
    }

    /// Digest of the neutral composition proof used by explanation.
    #[must_use]
    pub fn composition_plan_digest(&self) -> &str {
        &self.composition_plan_digest
    }
}

/// Exact admitted candidate coordinates established during evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigurationImprovementCandidate {
    principal_universe_digest: String,
    workspace_semantic_ir_digest: String,
    workspace_admission_evidence_digest: String,
    profile: ConfigurationImprovementProfile,
}

impl ConfigurationImprovementCandidate {
    fn from_compilation(
        universe: &PortableWorkspaceUniverse,
        compilation: &EffectiveProfileCompilation,
    ) -> Self {
        Self {
            principal_universe_digest: universe.principal_universe_digest().to_owned(),
            workspace_semantic_ir_digest: universe.semantic_ir_digest().to_owned(),
            workspace_admission_evidence_digest: universe.admission_evidence_digest().to_owned(),
            profile: ConfigurationImprovementProfile::from_compilation(compilation),
        }
    }

    /// Principal-relative identity of the admitted candidate workspace universe.
    #[must_use]
    pub fn principal_universe_digest(&self) -> &str {
        &self.principal_universe_digest
    }

    /// Verified semantic-IR identity of the candidate workspace.
    #[must_use]
    pub fn workspace_semantic_ir_digest(&self) -> &str {
        &self.workspace_semantic_ir_digest
    }

    /// Exact owner-proof frontier consumed by candidate admission.
    #[must_use]
    pub fn workspace_admission_evidence_digest(&self) -> &str {
        &self.workspace_admission_evidence_digest
    }

    /// Effective profile and private-instance coordinates derived for the candidate.
    #[must_use]
    pub const fn profile(&self) -> &ConfigurationImprovementProfile {
        &self.profile
    }
}

/// Whether one canonical digest changed across candidate evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigurationImprovementCoordinateChange {
    /// Both compiler phases produced the same coordinate.
    Unchanged {
        /// Shared canonical digest.
        digest: String,
    },
    /// The candidate produced a distinct coordinate.
    Changed {
        /// Canonical digest of the explained baseline.
        prior_digest: String,
        /// Canonical digest produced by candidate compilation.
        candidate_digest: String,
    },
}

impl ConfigurationImprovementCoordinateChange {
    fn between(prior: &str, candidate: &str) -> Self {
        if prior == candidate {
            Self::Unchanged {
                digest: prior.to_owned(),
            }
        } else {
            Self::Changed {
                prior_digest: prior.to_owned(),
                candidate_digest: candidate.to_owned(),
            }
        }
    }
}

/// Exact proof coordinate and complete impact cardinalities for one diagnosis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigurationImprovementDiagnosis {
    derivation_receipt_digest: String,
    change_root_count: u32,
    affected_count: u32,
    unaffected_count: u32,
    selected_topology: ConfigurationImprovementCoordinateChange,
    profile: ConfigurationImprovementCoordinateChange,
    instance: ConfigurationImprovementCoordinateChange,
    package_semantic_ir: ConfigurationImprovementCoordinateChange,
    composition_plan: ConfigurationImprovementCoordinateChange,
}

impl ConfigurationImprovementDiagnosis {
    fn from_derivation(
        derivation: &PortableWorkspaceDerivation,
        prior: &ConfigurationImprovementProfile,
        candidate: &ConfigurationImprovementProfile,
    ) -> Self {
        Self {
            derivation_receipt_digest: derivation.receipt().receipt_digest.to_string(),
            change_root_count: derivation.root_count(),
            affected_count: derivation.affected_count(),
            unaffected_count: derivation.unaffected_count(),
            selected_topology: ConfigurationImprovementCoordinateChange::between(
                prior.selected_topology_digest(),
                candidate.selected_topology_digest(),
            ),
            profile: ConfigurationImprovementCoordinateChange::between(
                prior.profile_digest(),
                candidate.profile_digest(),
            ),
            instance: ConfigurationImprovementCoordinateChange::between(
                prior.instance_digest(),
                candidate.instance_digest(),
            ),
            package_semantic_ir: ConfigurationImprovementCoordinateChange::between(
                prior.package_semantic_ir_digest(),
                candidate.package_semantic_ir_digest(),
            ),
            composition_plan: ConfigurationImprovementCoordinateChange::between(
                prior.composition_plan_digest(),
                candidate.composition_plan_digest(),
            ),
        }
    }

    /// Digest of the compiler-verified source-change and impact receipt.
    #[must_use]
    pub fn derivation_receipt_digest(&self) -> &str {
        &self.derivation_receipt_digest
    }

    /// Number of exact source-change roots in the proposal.
    #[must_use]
    pub const fn change_root_count(&self) -> u32 {
        self.change_root_count
    }

    /// Number of current subjects reachable from at least one change root.
    #[must_use]
    pub const fn affected_count(&self) -> u32 {
        self.affected_count
    }

    /// Number of current subjects outside every change-root closure.
    #[must_use]
    pub const fn unaffected_count(&self) -> u32 {
        self.unaffected_count
    }

    /// Project-relative selected-topology transition.
    #[must_use]
    pub const fn selected_topology(&self) -> &ConfigurationImprovementCoordinateChange {
        &self.selected_topology
    }

    /// Effective portable-profile transition.
    #[must_use]
    pub const fn profile(&self) -> &ConfigurationImprovementCoordinateChange {
        &self.profile
    }

    /// Private machine-instance transition.
    #[must_use]
    pub const fn instance(&self) -> &ConfigurationImprovementCoordinateChange {
        &self.instance
    }

    /// Selected Package semantic-IR transition within the admitted distribution.
    #[must_use]
    pub const fn package_semantic_ir(&self) -> &ConfigurationImprovementCoordinateChange {
        &self.package_semantic_ir
    }

    /// Neutral composition-proof transition.
    #[must_use]
    pub const fn composition_plan(&self) -> &ConfigurationImprovementCoordinateChange {
        &self.composition_plan
    }
}

/// Which exact owner frontier contradicted the inspected baseline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigurationImprovementFrontier {
    /// Principal workspace identity.
    Workspace,
    /// Logical project selected by the explained profile.
    Project,
    /// Execution workspace selected by the explained profile.
    ExecutionWorkspace,
    /// Private machine selected by the explained profile.
    Machine,
    /// Principal-relative admitted-universe identity.
    PrincipalUniverse,
    /// Verified workspace semantic-IR identity.
    WorkspaceSemanticIr,
    /// Owner-proof set admitted with the workspace.
    WorkspaceAdmissionEvidence,
    /// Project-relative desired-topology selection.
    SelectedTopology,
}

/// Failure to advance the effect-free improvement lifecycle.
#[derive(Debug, thiserror::Error)]
pub enum ConfigurationImprovementError {
    /// The explanatory profile does not share the inspected workspace owner.
    #[error("effective-profile explanation contradicts the inspected {frontier:?} frontier")]
    ExplanationFrontierMismatch {
        /// Exact non-substitutable frontier that disagreed.
        frontier: ConfigurationImprovementFrontier,
    },
    /// The bounded proposal failed canonical workspace admission.
    #[error("configuration-improvement candidate was rejected")]
    CandidateWorkspaceRejected(#[source] PortableWorkspaceFailure),
    /// The candidate's private/profile inputs failed canonical compilation.
    #[error("configuration-improvement candidate profile was rejected")]
    CandidateProfileRejected(#[source] crate::compile::CompileError),
    /// Candidate evaluation cannot cross into another immutable distribution.
    #[error("configuration-improvement candidate changes the admitted distribution frontier")]
    CandidateDistributionFrontierMismatch,
    /// Candidate evaluation cannot invent or reload another Package release set.
    #[error("configuration-improvement candidate changes the admitted Package release frontier")]
    CandidatePackageReleaseFrontierMismatch,
    /// The candidate names another principal workspace.
    #[error("configuration-improvement candidate changes the principal workspace identity")]
    CandidateWorkspaceMismatch,
    /// Candidate selectors do not preserve the explained review context.
    #[error("configuration-improvement candidate changes the explained {frontier:?} frontier")]
    CandidateContextMismatch {
        /// Exact non-substitutable selector that disagreed.
        frontier: ConfigurationImprovementFrontier,
    },
    /// A prepared candidate was committed through another explanation owner.
    #[error("configuration-improvement preparation belongs to another explanation owner")]
    PreparedOwnerMismatch,
    /// Previously admitted owners contradicted the derivation law.
    #[error("configuration-improvement diagnosis could not verify its derivation proof")]
    DiagnosisRejected(#[source] PortableWorkspaceFailure),
}

impl ConfigurationImprovementError {
    /// Return the mismatched explanatory frontier, when applicable.
    #[must_use]
    pub const fn frontier(&self) -> Option<ConfigurationImprovementFrontier> {
        match self {
            Self::ExplanationFrontierMismatch { frontier }
            | Self::CandidateContextMismatch { frontier } => Some(*frontier),
            Self::CandidateWorkspaceRejected(_)
            | Self::CandidateProfileRejected(_)
            | Self::CandidateDistributionFrontierMismatch
            | Self::CandidatePackageReleaseFrontierMismatch
            | Self::CandidateWorkspaceMismatch
            | Self::PreparedOwnerMismatch
            | Self::DiagnosisRejected(_) => None,
        }
    }
}

/// Complete bounded Pale Workspace candidate excluding Package source substance.
///
/// Package bytes were already admitted and dropped before this value exists.
/// Evaluation may re-select source-free Package semantics only within the
/// baseline distribution and release frontier.
pub struct CapturedConfigurationImprovementProposal {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    workspace: CapturedWorkspaceSource,
    private: CapturedPrivateWorkspaceInputs,
}

impl std::fmt::Debug for CapturedConfigurationImprovementProposal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedConfigurationImprovementProposal")
            .field("project_id_byte_len", &self.project_id.len())
            .field(
                "execution_workspace_id_byte_len",
                &self.execution_workspace_id.len(),
            )
            .field("machine_id_byte_len", &self.machine_id.len())
            .field("workspace", &self.workspace)
            .field("private", &self.private)
            .finish()
    }
}

impl CapturedConfigurationImprovementProposal {
    /// Bind portable and private source families to explicit topology selectors.
    #[must_use]
    pub fn new(
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
        workspace: CapturedWorkspaceSource,
        private: CapturedPrivateWorkspaceInputs,
    ) -> Self {
        Self {
            project_id,
            execution_workspace_id,
            machine_id,
            workspace,
            private,
        }
    }
}

/// Private/profile inputs for a candidate workspace that is already admitted.
///
/// Unlike [`CapturedConfigurationImprovementProposal`], this value carries no
/// portable workspace source. It lets editor and resident owners reuse their
/// exact admitted candidate generation while the application reuses the
/// baseline's source-free Package frontier.
pub struct CapturedConfigurationImprovementCandidate {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    private: CapturedPrivateWorkspaceInputs,
}

impl CapturedConfigurationImprovementCandidate {
    /// Bind private inputs and topology selectors to an admitted candidate.
    #[must_use]
    pub fn new(
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
        private: CapturedPrivateWorkspaceInputs,
    ) -> Self {
        Self {
            project_id,
            execution_workspace_id,
            machine_id,
            private,
        }
    }
}

#[derive(Clone)]
struct BaselineOwner {
    universe: Arc<PortableWorkspaceUniverse>,
    coordinates: ConfigurationImprovementBaseline,
}

#[derive(Clone)]
struct ExplainedOwner {
    baseline: BaselineOwner,
    profile: ConfigurationImprovementProfile,
    compilation: Arc<EffectiveProfileCompilation>,
    preparation_identity: Arc<()>,
}

struct EvaluatedOwner {
    explained: ExplainedOwner,
    candidate: Arc<PortableWorkspaceUniverse>,
    candidate_profile: Arc<EffectiveProfileCompilation>,
    candidate_coordinates: ConfigurationImprovementCandidate,
}

/// Inspected admitted configuration from which improvement may begin.
///
/// Each arrow in the transition relation consumes its predecessor, so no public
/// value can skip evaluation, diagnosis, or preview:
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/configuration-improvement-lifecycle.display.mathml"
))]
pub struct InspectedConfigurationImprovement {
    baseline: BaselineOwner,
}

impl InspectedConfigurationImprovement {
    /// Borrow the exact inspected workspace frontier.
    #[must_use]
    pub fn baseline(&self) -> &ConfigurationImprovementBaseline {
        &self.baseline.coordinates
    }

    /// Bind the inspection to the canonical effective-profile explanation owner.
    ///
    /// This transition verifies that workspace, semantic, and admission-proof
    /// coordinates all refer to the same retained compiler owner. It does not
    /// select a UI explanation, run a query, or infer user intent.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigurationImprovementError::ExplanationFrontierMismatch`]
    /// when `profile` was compiled from another admitted workspace frontier.
    pub fn explain(
        self,
        profile: Arc<EffectiveProfileCompilation>,
    ) -> Result<ExplainedConfigurationImprovement, ConfigurationImprovementError> {
        let baseline = &self.baseline.coordinates;
        let selected_topology_matches = self
            .baseline
            .universe
            .selected_topology_digest(
                profile.profile().project_id.as_str(),
                profile.profile().execution_workspace_id.as_str(),
            )
            .is_ok_and(|digest| digest == profile.selected_topology_digest());
        let checks = [
            (
                baseline.workspace_id() == profile.profile().workspace_id.as_str(),
                ConfigurationImprovementFrontier::Workspace,
            ),
            (
                baseline.principal_universe_digest()
                    == profile.principal_admitted_universe_digest().as_str(),
                ConfigurationImprovementFrontier::PrincipalUniverse,
            ),
            (
                baseline.workspace_semantic_ir_digest()
                    == profile.workspace_semantic_ir_digest().as_str(),
                ConfigurationImprovementFrontier::WorkspaceSemanticIr,
            ),
            (
                baseline.workspace_admission_evidence_digest()
                    == profile.workspace_admission_evidence_digest().as_str(),
                ConfigurationImprovementFrontier::WorkspaceAdmissionEvidence,
            ),
            (
                selected_topology_matches,
                ConfigurationImprovementFrontier::SelectedTopology,
            ),
        ];
        if let Some((_, frontier)) = checks.into_iter().find(|(matches, _)| !matches) {
            return Err(ConfigurationImprovementError::ExplanationFrontierMismatch { frontier });
        }
        Ok(ExplainedConfigurationImprovement {
            owner: ExplainedOwner {
                baseline: self.baseline,
                profile: ConfigurationImprovementProfile::from_compilation(&profile),
                compilation: profile,
                preparation_identity: Arc::new(()),
            },
        })
    }
}

/// Explained baseline bound to one exact effective-profile proof frontier.
pub struct ExplainedConfigurationImprovement {
    owner: ExplainedOwner,
}

impl ExplainedConfigurationImprovement {
    /// Borrow the exact explanatory profile and proof coordinates.
    #[must_use]
    pub fn profile(&self) -> &ConfigurationImprovementProfile {
        &self.owner.profile
    }

    /// Attach one bounded candidate source without evaluating it.
    ///
    /// The candidate remains untrusted source until [`ProposedConfigurationImprovement::evaluate`]
    /// consumes it through the canonical workspace compiler.
    #[must_use]
    pub fn propose(
        self,
        candidate: CapturedConfigurationImprovementProposal,
    ) -> ProposedConfigurationImprovement {
        ProposedConfigurationImprovement {
            owner: self.owner,
            candidate,
        }
    }

    /// Prepare a complete preview from one already admitted candidate owner.
    ///
    /// Preparation borrows this explained state. Any capture, profile
    /// compilation, or derivation failure therefore leaves the state available
    /// for a corrected retry. The preview cannot be accepted or rejected until
    /// [`Self::commit_prepared`] consumes this exact explanation owner.
    ///
    /// # Errors
    ///
    /// Returns the candidate-admission errors of
    /// [`ProposedConfigurationImprovement::evaluate`] or a subsequent derivation
    /// failure. The candidate must retain the explained inspection context.
    pub fn prepare_admitted_candidate(
        &self,
        candidate: Arc<PortableWorkspaceUniverse>,
        inputs: CapturedConfigurationImprovementCandidate,
    ) -> Result<PreparedConfigurationImprovementPreview, ConfigurationImprovementError> {
        let diagnosed = diagnose_evaluated(evaluate_admitted_candidate(
            self.owner.clone(),
            candidate,
            inputs,
        )?)?;
        Ok(PreparedConfigurationImprovementPreview {
            preparation_identity: Arc::clone(&self.owner.preparation_identity),
            owner: diagnosed.owner,
            derivation: diagnosed.derivation,
            diagnosis: diagnosed.diagnosis,
        })
    }

    /// Commit owners prepared from this exact explanation into a preview gate.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigurationImprovementError::PreparedOwnerMismatch`] if a
    /// caller attempts to splice a preparation from another explanation.
    pub fn commit_prepared(
        self,
        gate: ConfigurationImprovementCommitGate,
    ) -> Result<ConfigurationImprovementDispositionGate, ConfigurationImprovementError> {
        if !Arc::ptr_eq(&self.owner.preparation_identity, &gate.preparation_identity) {
            return Err(ConfigurationImprovementError::PreparedOwnerMismatch);
        }
        Ok(ConfigurationImprovementDispositionGate { private: () })
    }
}

/// Unevaluated bounded source proposed against an explained baseline.
pub struct ProposedConfigurationImprovement {
    owner: ExplainedOwner,
    candidate: CapturedConfigurationImprovementProposal,
}

impl std::fmt::Debug for ProposedConfigurationImprovement {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProposedConfigurationImprovement")
            .field("baseline", &self.owner.baseline.coordinates)
            .field("profile", &self.owner.profile)
            .field("candidate", &self.candidate)
            .finish()
    }
}

impl ProposedConfigurationImprovement {
    /// Compile and admit the exact candidate through the canonical workspace path.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigurationImprovementError::CandidateWorkspaceRejected`]
    /// when portable source fails ordinary workspace admission,
    /// [`ConfigurationImprovementError::CandidateProfileRejected`] when
    /// private/profile admission fails, or a dedicated frontier error when the
    /// candidate attempts to replace its workspace, selected project,
    /// execution workspace, machine, distribution, or Package release owner.
    pub fn evaluate(
        self,
    ) -> Result<EvaluatedConfigurationImprovement, ConfigurationImprovementError> {
        let CapturedConfigurationImprovementProposal {
            project_id,
            execution_workspace_id,
            machine_id,
            workspace,
            private,
        } = self.candidate;
        let candidate = Arc::new(
            compile_portable_workspace(workspace)
                .map_err(ConfigurationImprovementError::CandidateWorkspaceRejected)?,
        );
        let owner = evaluate_admitted_candidate(
            self.owner,
            candidate,
            CapturedConfigurationImprovementCandidate::new(
                project_id,
                execution_workspace_id,
                machine_id,
                private,
            ),
        )?;
        Ok(EvaluatedConfigurationImprovement { owner })
    }
}

fn evaluate_admitted_candidate(
    explained: ExplainedOwner,
    candidate: Arc<PortableWorkspaceUniverse>,
    inputs: CapturedConfigurationImprovementCandidate,
) -> Result<EvaluatedOwner, ConfigurationImprovementError> {
    if candidate.workspace_id() != explained.baseline.coordinates.workspace_id() {
        return Err(ConfigurationImprovementError::CandidateWorkspaceMismatch);
    }
    let CapturedConfigurationImprovementCandidate {
        project_id,
        execution_workspace_id,
        machine_id,
        private,
    } = inputs;
    let selector_checks = [
        (
            project_id == explained.profile.project_id,
            ConfigurationImprovementFrontier::Project,
        ),
        (
            execution_workspace_id == explained.profile.execution_workspace_id,
            ConfigurationImprovementFrontier::ExecutionWorkspace,
        ),
        (
            machine_id == explained.profile.machine_id,
            ConfigurationImprovementFrontier::Machine,
        ),
    ];
    if let Some((_, frontier)) = selector_checks.into_iter().find(|(matches, _)| !matches) {
        return Err(ConfigurationImprovementError::CandidateContextMismatch { frontier });
    }
    let candidate_profile = Arc::new(
        compile_captured_effective_profile_candidate_inputs(
            project_id,
            execution_workspace_id,
            machine_id,
            private,
            &candidate,
            &explained.compilation,
        )
        .map_err(candidate_profile_error)?,
    );
    let candidate_coordinates =
        ConfigurationImprovementCandidate::from_compilation(&candidate, &candidate_profile);
    Ok(EvaluatedOwner {
        explained,
        candidate,
        candidate_profile,
        candidate_coordinates,
    })
}

fn candidate_profile_error(error: EffectiveProfileCandidateError) -> ConfigurationImprovementError {
    match error {
        EffectiveProfileCandidateError::Compile(source) => {
            ConfigurationImprovementError::CandidateProfileRejected(source)
        }
        EffectiveProfileCandidateError::DistributionFrontier => {
            ConfigurationImprovementError::CandidateDistributionFrontierMismatch
        }
        EffectiveProfileCandidateError::PackageReleaseFrontier => {
            ConfigurationImprovementError::CandidatePackageReleaseFrontierMismatch
        }
    }
}

/// Fully evaluated candidate whose explanation owner is not yet consumed.
///
/// The value is deliberately neither cloneable nor serializable. It owns the
/// exact comparison proof and admitted candidate, but cannot create a review
/// disposition until committed through its originating
/// [`ExplainedConfigurationImprovement`].
pub struct PreparedConfigurationImprovementPreview {
    preparation_identity: Arc<()>,
    owner: EvaluatedOwner,
    derivation: PortableWorkspaceDerivation,
    diagnosis: ConfigurationImprovementDiagnosis,
}

impl PreparedConfigurationImprovementPreview {
    /// Consume the preparation into API-owned projection parts.
    ///
    /// The returned commit gate still requires the originating explanation;
    /// it grants no terminal disposition by itself.
    #[must_use]
    pub fn into_parts(self) -> PreparedConfigurationImprovementPreviewParts {
        PreparedConfigurationImprovementPreviewParts {
            baseline_owner: self.owner.explained.baseline.universe,
            candidate_owner: self.owner.candidate,
            baseline_profile_owner: self.owner.explained.compilation,
            candidate_profile_owner: self.owner.candidate_profile,
            derivation: self.derivation,
            baseline: self.owner.explained.baseline.coordinates,
            profile: self.owner.explained.profile,
            candidate: self.owner.candidate_coordinates,
            diagnosis: self.diagnosis,
            commit_gate: ConfigurationImprovementCommitGate {
                preparation_identity: self.preparation_identity,
            },
        }
    }
}

/// Application-owned components prepared before the originating explanation
/// is consumed.
pub struct PreparedConfigurationImprovementPreviewParts {
    /// Retained exact baseline owner.
    pub baseline_owner: Arc<PortableWorkspaceUniverse>,
    /// Retained exact candidate owner.
    pub candidate_owner: Arc<PortableWorkspaceUniverse>,
    /// Retained exact explanatory profile owner.
    pub baseline_profile_owner: Arc<EffectiveProfileCompilation>,
    /// Retained exact candidate profile owner.
    pub candidate_profile_owner: Arc<EffectiveProfileCompilation>,
    /// Verified canonical comparison proof.
    pub derivation: PortableWorkspaceDerivation,
    /// Exact inspected baseline coordinates.
    pub baseline: ConfigurationImprovementBaseline,
    /// Exact explanatory profile coordinates.
    pub profile: ConfigurationImprovementProfile,
    /// Exact admitted candidate coordinates.
    pub candidate: ConfigurationImprovementCandidate,
    /// Exact diagnosis coordinate and counts.
    pub diagnosis: ConfigurationImprovementDiagnosis,
    /// Unforgeable gate bound to the borrowed explanation owner.
    pub commit_gate: ConfigurationImprovementCommitGate,
}

/// Opaque identity proving which explanation prepared a candidate.
pub struct ConfigurationImprovementCommitGate {
    preparation_identity: Arc<()>,
}

/// Admitted candidate that has not yet derived its source-change impact proof.
pub struct EvaluatedConfigurationImprovement {
    owner: EvaluatedOwner,
}

impl EvaluatedConfigurationImprovement {
    /// Borrow the exact admitted candidate coordinates.
    #[must_use]
    pub fn candidate(&self) -> &ConfigurationImprovementCandidate {
        &self.owner.candidate_coordinates
    }

    /// Derive the candidate's complete impact partition from canonical owners.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigurationImprovementError::DiagnosisRejected`] if the two
    /// already admitted universes contradict the derivation contract.
    pub fn diagnose(
        self,
    ) -> Result<DiagnosedConfigurationImprovement, ConfigurationImprovementError> {
        diagnose_evaluated(self.owner)
    }
}

fn diagnose_evaluated(
    owner: EvaluatedOwner,
) -> Result<DiagnosedConfigurationImprovement, ConfigurationImprovementError> {
    let derivation = owner
        .candidate
        .compare_with_predecessor(&owner.explained.baseline.universe)
        .map_err(ConfigurationImprovementError::DiagnosisRejected)?;
    let diagnosis = ConfigurationImprovementDiagnosis::from_derivation(
        &derivation,
        &owner.explained.profile,
        &owner.candidate_coordinates.profile,
    );
    Ok(DiagnosedConfigurationImprovement {
        owner,
        derivation,
        diagnosis,
    })
}

/// Evaluated candidate with an exact graph-derived impact proof.
pub struct DiagnosedConfigurationImprovement {
    owner: EvaluatedOwner,
    derivation: PortableWorkspaceDerivation,
    diagnosis: ConfigurationImprovementDiagnosis,
}

impl DiagnosedConfigurationImprovement {
    /// Borrow the exact derivation coordinate and impact cardinalities.
    #[must_use]
    pub fn diagnosis(&self) -> &ConfigurationImprovementDiagnosis {
        &self.diagnosis
    }

    /// Freeze the admitted candidate and diagnosis into a reviewable preview.
    #[must_use]
    pub fn preview(self) -> PreviewedConfigurationImprovement {
        PreviewedConfigurationImprovement {
            owner: self.owner,
            derivation: self.derivation,
            diagnosis: self.diagnosis,
        }
    }
}

/// Reviewable, effect-free candidate immediately before explicit disposition.
pub struct PreviewedConfigurationImprovement {
    owner: EvaluatedOwner,
    derivation: PortableWorkspaceDerivation,
    diagnosis: ConfigurationImprovementDiagnosis,
}

impl PreviewedConfigurationImprovement {
    /// Consume the preview into its owners, proof, and immutable coordinates.
    ///
    /// This is the narrow projection seam used by `pale-sdk`; protocol leaves
    /// do not receive compiler IR or workspace source.
    #[must_use]
    pub fn into_parts(self) -> ConfigurationImprovementPreviewParts {
        ConfigurationImprovementPreviewParts {
            baseline_owner: self.owner.explained.baseline.universe,
            candidate_owner: self.owner.candidate,
            baseline_profile_owner: self.owner.explained.compilation,
            candidate_profile_owner: self.owner.candidate_profile,
            derivation: self.derivation,
            baseline: self.owner.explained.baseline.coordinates,
            profile: self.owner.explained.profile,
            candidate: self.owner.candidate_coordinates,
            diagnosis: self.diagnosis,
            disposition_gate: ConfigurationImprovementDispositionGate { private: () },
        }
    }
}

/// Application-owned components of one reviewable preview.
///
/// This value is intentionally not serializable. The public façade consumes it
/// into bounded owned projections and retains the semantic owners opaquely.
pub struct ConfigurationImprovementPreviewParts {
    /// Retained exact baseline owner.
    pub baseline_owner: Arc<PortableWorkspaceUniverse>,
    /// Retained exact candidate owner.
    pub candidate_owner: Arc<PortableWorkspaceUniverse>,
    /// Retained exact explanatory profile owner.
    pub baseline_profile_owner: Arc<EffectiveProfileCompilation>,
    /// Retained exact candidate profile owner.
    pub candidate_profile_owner: Arc<EffectiveProfileCompilation>,
    /// Verified canonical comparison proof.
    pub derivation: PortableWorkspaceDerivation,
    /// Exact inspected baseline coordinates.
    pub baseline: ConfigurationImprovementBaseline,
    /// Exact explanatory profile coordinates.
    pub profile: ConfigurationImprovementProfile,
    /// Exact admitted candidate coordinates.
    pub candidate: ConfigurationImprovementCandidate,
    /// Exact diagnosis coordinate and counts.
    pub diagnosis: ConfigurationImprovementDiagnosis,
    /// Unforgeable gate proving that every review phase completed.
    pub disposition_gate: ConfigurationImprovementDispositionGate,
}

/// Explicit human-governed terminal disposition of a reviewed preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigurationImprovementDisposition {
    /// The reviewed candidate may proceed to a separate authority/effect gate.
    Accepted,
    /// The reviewing participant rejected the candidate.
    Rejected,
    /// The reviewing participant cancelled the review.
    Cancelled,
    /// An outer native coordinate changed before disposition completed.
    Stale,
}

/// Unforgeable authority-free gate produced only by a complete preview.
///
/// Consuming this gate records a disposition. It does not prove who made the
/// decision and cannot authorize source mutation; those obligations remain at
/// the separately admitted native boundary.
pub struct ConfigurationImprovementDispositionGate {
    private: (),
}

impl ConfigurationImprovementDispositionGate {
    /// Record one explicit terminal disposition for the reviewed preview.
    #[must_use]
    pub const fn finish(
        self,
        disposition: ConfigurationImprovementDisposition,
    ) -> ConfigurationImprovementDisposition {
        let Self { private: () } = self;
        disposition
    }
}

/// Inspect one already admitted configuration as the only lifecycle entry.
///
/// Construction is effect-free and preserves the exact admitted owner behind
/// the returned state. A caller cannot skip directly to proposal or terminal
/// disposition because each later state consumes its predecessor.
#[must_use]
pub fn inspect_configuration_improvement(
    baseline: Arc<PortableWorkspaceUniverse>,
) -> InspectedConfigurationImprovement {
    let coordinates = ConfigurationImprovementBaseline::from_universe(&baseline);
    InspectedConfigurationImprovement {
        baseline: BaselineOwner {
            universe: baseline,
            coordinates,
        },
    }
}
