//! Review a proposed Pale Workspace change before applying it.
//!
//! Start with a successfully compiled baseline, explain its effective profile,
//! and supply a candidate configuration. The application compiles the candidate
//! and explains its impact. A preview lets the caller accept, reject, cancel,
//! or mark the review stale. Accepting a review does not apply the change.
//!
//! Each stage returns an opaque handle: a value that retains the application’s
//! validated state without exposing its internal representation. Consuming a
//! handle advances the review; callers cannot construct or skip stages. See
//! [`PaleApi::inspect_configuration_improvement`] for the entry point and
//! [`PaleApi::prepare_configuration_improvement_preview`] to reuse an already
//! compiled candidate without reparsing its workspace documents.
//!
//! Compilation digests identify the configurations under review. Editor
//! revisions, ACP sessions, remote tasks, and native conversations have separate
//! identities. Their callers must check those identities for changes before
//! using a result; compilation proof does not replace that check.
//!
//! [`ConfigurationImprovementDisposition::Accepted`] permits presentation to a
//! separate authorization and effect path, not a write or an autonomous
//! improvement loop. This review API is neither the offline `SkillOpt` search
//! layer nor a cognitive seat, and grants neither one decision authority.

use std::sync::Arc;

use pale_application::improvement as application;
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
};

use crate::error::{application_diagnostic_policy, FailureSpecification};
use crate::portable_workspace::{portable_summary_failure, workspace_failure};
use crate::{
    ApiError, EffectiveProfileCompilation, OneShotPrivateInputs, PaleApi,
    PortableWorkspaceChangeRootPage, PortableWorkspaceComparison, PortableWorkspaceExplanation,
    PortableWorkspaceImpactPage, PortableWorkspaceRequest, PortableWorkspaceSubject,
    PortableWorkspaceSubjectInspection, PortableWorkspaceSubjectInspectionCursor,
};

/// Closed, ordered phases of one effect-free configuration review.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationImprovementPhase {
    /// An admitted baseline was captured for inspection.
    Inspected,
    /// The baseline was joined to its exact effective-profile proof.
    Explained,
    /// A bounded candidate source was supplied but not admitted.
    Proposed,
    /// The candidate passed canonical workspace admission.
    Evaluated,
    /// The canonical derivation graph produced a complete impact partition.
    Diagnosed,
    /// The candidate and its proof are frozen for explicit review.
    Previewed,
}

/// Explicit terminal disposition after a complete preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
pub enum ConfigurationImprovementDisposition {
    /// The candidate may be presented to a separate authority/effect gate.
    Accepted,
    /// The reviewing participant rejected the candidate.
    Rejected,
    /// The reviewing participant cancelled the review.
    Cancelled,
    /// A separately owned outer coordinate changed before disposition.
    Stale,
}

/// Captured baseline and candidate for a new configuration review.
///
/// Package source belongs to the baseline only. Candidate evaluation reuses
/// its admitted Package meaning. Source labels do not grant filesystem access.
/// Constructing this request does not validate its source or input limits;
/// [`PaleApi::preview_configuration_improvement_sources`] performs those checks.
#[derive(Debug)]
pub struct ConfigurationImprovementSources {
    workspace: PortableWorkspaceRequest,
    profile: crate::EffectiveProfileCompilationRequest,
    candidate: ConfigurationImprovementProposalRequest,
}

impl ConfigurationImprovementSources {
    /// Keep baseline source, profile inputs and candidate source in distinct roles.
    #[must_use]
    pub fn new(
        workspace: PortableWorkspaceRequest,
        profile: crate::EffectiveProfileCompilationRequest,
        candidate: ConfigurationImprovementProposalRequest,
    ) -> Self {
        Self {
            workspace,
            profile,
            candidate,
        }
    }
}

/// Complete Pale Workspace proposal against an admitted Package distribution.
///
/// Portable workspace and private machine/configuration source are consumed by
/// their canonical admission routes. Package source is deliberately absent:
/// candidate evaluation reuses the baseline's source-free admitted semantics
/// and rejects a different distribution or release frontier.
pub struct ConfigurationImprovementProposalRequest {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    workspace: PortableWorkspaceRequest,
    private_inputs: OneShotPrivateInputs,
}

impl std::fmt::Debug for ConfigurationImprovementProposalRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ConfigurationImprovementProposalRequest")
            .field("project_id_byte_len", &self.project_id.len())
            .field(
                "execution_workspace_id_byte_len",
                &self.execution_workspace_id.len(),
            )
            .field("machine_id_byte_len", &self.machine_id.len())
            .field("workspace", &self.workspace)
            .field("private_inputs", &self.private_inputs)
            .finish()
    }
}

impl ConfigurationImprovementProposalRequest {
    /// Collect project, execution-workspace and machine selectors with their source.
    ///
    /// Compilation checks the selectors and source limits; this constructor
    /// does not admit the candidate or read files.
    #[must_use]
    pub fn new(
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
        workspace: PortableWorkspaceRequest,
        private_inputs: OneShotPrivateInputs,
    ) -> Self {
        Self {
            project_id,
            execution_workspace_id,
            machine_id,
            workspace,
            private_inputs,
        }
    }
}

/// Private/profile inputs for an already admitted candidate workspace.
///
/// This request deliberately contains no portable workspace documents. It is
/// paired with an opaque [`crate::PortableWorkspaceCompilation`] so editor and
/// resident leaves cannot trigger a second parse of the same admitted
/// generation while preparing a review.
pub struct ConfigurationImprovementCandidateRequest {
    project_id: String,
    execution_workspace_id: String,
    machine_id: String,
    private_inputs: OneShotPrivateInputs,
}

impl ConfigurationImprovementCandidateRequest {
    /// Bind explicit topology selectors and private source to an admitted candidate.
    #[must_use]
    pub fn new(
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
        private_inputs: OneShotPrivateInputs,
    ) -> Self {
        Self {
            project_id,
            execution_workspace_id,
            machine_id,
            private_inputs,
        }
    }
}

/// Opaque inspected baseline; the only entry into the lifecycle.
pub struct ConfigurationImprovementInspection {
    state: application::InspectedConfigurationImprovement,
}

impl ConfigurationImprovementInspection {
    /// Current consuming state of this handle.
    #[must_use]
    pub const fn phase(&self) -> ConfigurationImprovementPhase {
        ConfigurationImprovementPhase::Inspected
    }

    /// Principal workspace identity of the inspected baseline.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        self.state.baseline().workspace_id()
    }

    /// Exact admitted-universe digest fenced by subsequent transitions.
    #[must_use]
    pub fn principal_universe_digest(&self) -> &str {
        self.state.baseline().principal_universe_digest()
    }
}

/// Opaque baseline joined to one canonical effective-profile proof owner.
pub struct ConfigurationImprovementExplanation {
    state: application::ExplainedConfigurationImprovement,
}

impl ConfigurationImprovementExplanation {
    /// Current consuming state of this handle.
    #[must_use]
    pub const fn phase(&self) -> ConfigurationImprovementPhase {
        ConfigurationImprovementPhase::Explained
    }

    /// Exact portable profile whose retained proof explains the baseline.
    #[must_use]
    pub fn profile_digest(&self) -> &str {
        self.state.profile().profile_digest()
    }

    /// Identify the compiled private configuration used for this explanation.
    ///
    /// This digest does not show whether a machine is running that configuration.
    #[must_use]
    pub fn instance_digest(&self) -> &str {
        self.state.profile().instance_digest()
    }
}

/// Opaque, bounded candidate that has not crossed semantic admission.
pub struct ConfigurationImprovementProposal {
    state: application::ProposedConfigurationImprovement,
}

impl ConfigurationImprovementProposal {
    /// Current consuming state of this handle.
    #[must_use]
    pub const fn phase(&self) -> ConfigurationImprovementPhase {
        ConfigurationImprovementPhase::Proposed
    }
}

/// Opaque candidate admitted by the canonical workspace compiler.
pub struct ConfigurationImprovementEvaluation {
    state: application::EvaluatedConfigurationImprovement,
}

impl ConfigurationImprovementEvaluation {
    /// Current consuming state of this handle.
    #[must_use]
    pub const fn phase(&self) -> ConfigurationImprovementPhase {
        ConfigurationImprovementPhase::Evaluated
    }

    /// Principal-relative identity of the admitted candidate workspace universe.
    #[must_use]
    pub fn candidate_principal_universe_digest(&self) -> &str {
        self.state.candidate().principal_universe_digest()
    }

    /// Candidate effective-profile identity established by full compilation.
    #[must_use]
    pub fn candidate_profile_digest(&self) -> &str {
        self.state.candidate().profile().profile_digest()
    }

    /// Identify the candidate's compiled private configuration, not machine readiness.
    #[must_use]
    pub fn candidate_instance_digest(&self) -> &str {
        self.state.candidate().profile().instance_digest()
    }
}

/// Exact digest transition established by candidate diagnosis.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ConfigurationImprovementCoordinateChange {
    /// Baseline and candidate share one exact digest.
    Unchanged {
        /// Shared canonical digest.
        digest: String,
    },
    /// Candidate compilation produced a distinct digest.
    Changed {
        /// Canonical digest of the explained baseline.
        prior_digest: String,
        /// Canonical digest produced by candidate compilation.
        candidate_digest: String,
    },
}

impl ConfigurationImprovementCoordinateChange {
    fn project(value: &application::ConfigurationImprovementCoordinateChange) -> Self {
        match value {
            application::ConfigurationImprovementCoordinateChange::Unchanged { digest } => {
                Self::Unchanged {
                    digest: digest.clone(),
                }
            }
            application::ConfigurationImprovementCoordinateChange::Changed {
                prior_digest,
                candidate_digest,
            } => Self::Changed {
                prior_digest: prior_digest.clone(),
                candidate_digest: candidate_digest.clone(),
            },
        }
    }
}

/// Opaque candidate with a verified complete impact partition.
pub struct ConfigurationImprovementDiagnosis {
    state: application::DiagnosedConfigurationImprovement,
}

impl ConfigurationImprovementDiagnosis {
    /// Current consuming state of this handle.
    #[must_use]
    pub const fn phase(&self) -> ConfigurationImprovementPhase {
        ConfigurationImprovementPhase::Diagnosed
    }

    /// Digest of the exact derivation receipt.
    #[must_use]
    pub fn derivation_receipt_digest(&self) -> &str {
        self.state.diagnosis().derivation_receipt_digest()
    }
}

/// Owned coordinates and cardinalities for one reviewable preview.
///
/// The projection contains no source bytes, IR nodes, graph storage, private
/// values, provider payloads, native handles, or generic correlation identity.
/// Its impact counts retain the same complete disjoint-partition law as
/// [`PortableWorkspaceComparison`]; that comparison remains the equation's
/// single documentation owner.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
pub struct ConfigurationImprovementPreviewSummary {
    workspace_id: String,
    project_id: String,
    execution_workspace_id: String,
    baseline_principal_universe_digest: String,
    baseline_workspace_semantic_ir_digest: String,
    baseline_workspace_admission_evidence_digest: String,
    selected_topology_digest: String,
    profile_digest: String,
    instance_digest: String,
    package_semantic_ir_digest: String,
    composition_plan_digest: String,
    candidate_principal_universe_digest: String,
    candidate_workspace_semantic_ir_digest: String,
    candidate_workspace_admission_evidence_digest: String,
    candidate_selected_topology_digest: String,
    candidate_profile_digest: String,
    candidate_instance_digest: String,
    candidate_package_semantic_ir_digest: String,
    candidate_composition_plan_digest: String,
    derivation_receipt_digest: String,
    change_root_count: u32,
    affected_count: u32,
    unaffected_count: u32,
    selected_topology_change: ConfigurationImprovementCoordinateChange,
    profile_change: ConfigurationImprovementCoordinateChange,
    instance_change: ConfigurationImprovementCoordinateChange,
    package_semantic_ir_change: ConfigurationImprovementCoordinateChange,
    composition_plan_change: ConfigurationImprovementCoordinateChange,
    /// Exact API-owned JCS projection of every preceding field.
    #[serde(skip)]
    #[cfg_attr(feature = "wire-schema", schemars(skip))]
    canonical_bytes: Vec<u8>,
}

impl ConfigurationImprovementPreviewSummary {
    /// Maximum UTF-8 bytes accepted when a caller returns a presented summary.
    ///
    /// This 64 KiB input limit includes JSON whitespace and escapes. The summary
    /// contains bounded logical identifiers, digests, counts and fixed-shape
    /// changes, not source documents or impact lists.
    pub const MAX_PRESENTED_BYTES: usize = 64 * 1024;

    fn project(
        identity: &pale_core::records::RecordSchemaIdentity,
        baseline: &application::ConfigurationImprovementBaseline,
        profile: &application::ConfigurationImprovementProfile,
        candidate: &application::ConfigurationImprovementCandidate,
        diagnosis: &application::ConfigurationImprovementDiagnosis,
    ) -> Result<Self, ApiError> {
        let mut projection = Self {
            workspace_id: baseline.workspace_id().to_owned(),
            project_id: profile.project_id().to_owned(),
            execution_workspace_id: profile.execution_workspace_id().to_owned(),
            baseline_principal_universe_digest: baseline.principal_universe_digest().to_owned(),
            baseline_workspace_semantic_ir_digest: baseline
                .workspace_semantic_ir_digest()
                .to_owned(),
            baseline_workspace_admission_evidence_digest: baseline
                .workspace_admission_evidence_digest()
                .to_owned(),
            selected_topology_digest: profile.selected_topology_digest().to_owned(),
            profile_digest: profile.profile_digest().to_owned(),
            instance_digest: profile.instance_digest().to_owned(),
            package_semantic_ir_digest: profile.package_semantic_ir_digest().to_owned(),
            composition_plan_digest: profile.composition_plan_digest().to_owned(),
            candidate_principal_universe_digest: candidate.principal_universe_digest().to_owned(),
            candidate_workspace_semantic_ir_digest: candidate
                .workspace_semantic_ir_digest()
                .to_owned(),
            candidate_workspace_admission_evidence_digest: candidate
                .workspace_admission_evidence_digest()
                .to_owned(),
            candidate_selected_topology_digest: candidate
                .profile()
                .selected_topology_digest()
                .to_owned(),
            candidate_profile_digest: candidate.profile().profile_digest().to_owned(),
            candidate_instance_digest: candidate.profile().instance_digest().to_owned(),
            candidate_package_semantic_ir_digest: candidate
                .profile()
                .package_semantic_ir_digest()
                .to_owned(),
            candidate_composition_plan_digest: candidate
                .profile()
                .composition_plan_digest()
                .to_owned(),
            derivation_receipt_digest: diagnosis.derivation_receipt_digest().to_owned(),
            change_root_count: diagnosis.change_root_count(),
            affected_count: diagnosis.affected_count(),
            unaffected_count: diagnosis.unaffected_count(),
            selected_topology_change: ConfigurationImprovementCoordinateChange::project(
                diagnosis.selected_topology(),
            ),
            profile_change: ConfigurationImprovementCoordinateChange::project(diagnosis.profile()),
            instance_change: ConfigurationImprovementCoordinateChange::project(
                diagnosis.instance(),
            ),
            package_semantic_ir_change: ConfigurationImprovementCoordinateChange::project(
                diagnosis.package_semantic_ir(),
            ),
            composition_plan_change: ConfigurationImprovementCoordinateChange::project(
                diagnosis.composition_plan(),
            ),
            canonical_bytes: Vec::new(),
        };
        projection.canonical_bytes = pale_core::canon::to_canonical_string(&projection)
            .map(String::into_bytes)
            .map_err(|error| portable_summary_failure(identity, error))?;
        Ok(projection)
    }

    /// Exact canonical JSON projection of this summary.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Principal workspace whose configuration is being reviewed.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    /// Canonical project selected by the explanatory profile.
    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Canonical execution workspace selected by the explanatory profile.
    #[must_use]
    pub fn execution_workspace_id(&self) -> &str {
        &self.execution_workspace_id
    }

    /// Principal-relative identity of the inspected baseline.
    #[must_use]
    pub fn baseline_principal_universe_digest(&self) -> &str {
        &self.baseline_principal_universe_digest
    }

    /// Semantic-IR identity of the inspected baseline workspace.
    #[must_use]
    pub fn baseline_workspace_semantic_ir_digest(&self) -> &str {
        &self.baseline_workspace_semantic_ir_digest
    }

    /// Admission-evidence identity of the inspected baseline workspace.
    #[must_use]
    pub fn baseline_workspace_admission_evidence_digest(&self) -> &str {
        &self.baseline_workspace_admission_evidence_digest
    }

    /// Identity of the desired topology selected for baseline compilation.
    #[must_use]
    pub fn selected_topology_digest(&self) -> &str {
        &self.selected_topology_digest
    }

    /// Identity of the effective profile used for explanation.
    #[must_use]
    pub fn profile_digest(&self) -> &str {
        &self.profile_digest
    }

    /// Identity of the private instance used for explanation.
    #[must_use]
    pub fn instance_digest(&self) -> &str {
        &self.instance_digest
    }

    /// Package semantic-IR identity retained by the baseline profile.
    #[must_use]
    pub fn package_semantic_ir_digest(&self) -> &str {
        &self.package_semantic_ir_digest
    }

    /// Composition-proof identity retained by the baseline profile.
    #[must_use]
    pub fn composition_plan_digest(&self) -> &str {
        &self.composition_plan_digest
    }

    /// Principal-relative identity of the admitted candidate.
    #[must_use]
    pub fn candidate_principal_universe_digest(&self) -> &str {
        &self.candidate_principal_universe_digest
    }

    /// Semantic-IR identity of the admitted candidate workspace.
    #[must_use]
    pub fn candidate_workspace_semantic_ir_digest(&self) -> &str {
        &self.candidate_workspace_semantic_ir_digest
    }

    /// Admission-evidence identity of the admitted candidate workspace.
    #[must_use]
    pub fn candidate_workspace_admission_evidence_digest(&self) -> &str {
        &self.candidate_workspace_admission_evidence_digest
    }

    /// Identity of the desired topology selected for candidate compilation.
    #[must_use]
    pub fn candidate_selected_topology_digest(&self) -> &str {
        &self.candidate_selected_topology_digest
    }

    /// Identity of the fully compiled candidate effective profile.
    #[must_use]
    pub fn candidate_profile_digest(&self) -> &str {
        &self.candidate_profile_digest
    }

    /// Identity of the candidate's admitted private machine projection.
    #[must_use]
    pub fn candidate_instance_digest(&self) -> &str {
        &self.candidate_instance_digest
    }

    /// Package semantic-IR identity retained by the candidate profile.
    #[must_use]
    pub fn candidate_package_semantic_ir_digest(&self) -> &str {
        &self.candidate_package_semantic_ir_digest
    }

    /// Composition-proof identity retained by the candidate profile.
    #[must_use]
    pub fn candidate_composition_plan_digest(&self) -> &str {
        &self.candidate_composition_plan_digest
    }

    /// Identity of the exact derivation proof.
    #[must_use]
    pub fn derivation_receipt_digest(&self) -> &str {
        &self.derivation_receipt_digest
    }

    /// Number of exact source-change roots.
    #[must_use]
    pub const fn change_root_count(&self) -> u32 {
        self.change_root_count
    }

    /// Number of current subjects reachable from a change root.
    #[must_use]
    pub const fn affected_count(&self) -> u32 {
        self.affected_count
    }

    /// Number of current subjects outside every change-root closure.
    #[must_use]
    pub const fn unaffected_count(&self) -> u32 {
        self.unaffected_count
    }

    /// Exact selected-topology digest transition.
    #[must_use]
    pub const fn selected_topology_change(&self) -> &ConfigurationImprovementCoordinateChange {
        &self.selected_topology_change
    }

    /// Exact effective-profile digest transition.
    #[must_use]
    pub const fn profile_change(&self) -> &ConfigurationImprovementCoordinateChange {
        &self.profile_change
    }

    /// Exact private-instance digest transition.
    #[must_use]
    pub const fn instance_change(&self) -> &ConfigurationImprovementCoordinateChange {
        &self.instance_change
    }

    /// Exact selected Package semantic-IR digest transition.
    #[must_use]
    pub const fn package_semantic_ir_change(&self) -> &ConfigurationImprovementCoordinateChange {
        &self.package_semantic_ir_change
    }

    /// Exact neutral composition-proof digest transition.
    #[must_use]
    pub const fn composition_plan_change(&self) -> &ConfigurationImprovementCoordinateChange {
        &self.composition_plan_change
    }
}

/// Complete reviewable candidate and its retained canonical impact owner.
pub struct ConfigurationImprovementPreview {
    summary: ConfigurationImprovementPreviewSummary,
    comparison: PortableWorkspaceComparison,
    baseline_profile: EffectiveProfileCompilation,
    candidate_profile: EffectiveProfileCompilation,
    disposition_gate: application::ConfigurationImprovementDispositionGate,
}

/// Fully projected candidate waiting for its explanation owner to commit.
///
/// Preparation performs every fallible compilation, derivation, and canonical
/// projection while borrowing the explanation. Cancellation or stale-result
/// suppression can therefore discard this value without advancing the public
/// lifecycle. The value exposes only its bounded summary until
/// [`PaleApi::commit_configuration_improvement_preview`] consumes the exact
/// explanation that prepared it.
pub struct PreparedConfigurationImprovementPreview {
    summary: ConfigurationImprovementPreviewSummary,
    comparison: PortableWorkspaceComparison,
    baseline_profile: EffectiveProfileCompilation,
    candidate_profile: EffectiveProfileCompilation,
    commit_gate: application::ConfigurationImprovementCommitGate,
}

impl PreparedConfigurationImprovementPreview {
    /// Borrow the candidate coordinates and impact counts before commit.
    #[must_use]
    pub const fn summary(&self) -> &ConfigurationImprovementPreviewSummary {
        &self.summary
    }
}

impl ConfigurationImprovementPreview {
    /// Current consuming state of this handle.
    #[must_use]
    pub const fn phase(&self) -> ConfigurationImprovementPhase {
        ConfigurationImprovementPhase::Previewed
    }

    /// Borrow the owned review coordinates and counts.
    #[must_use]
    pub const fn summary(&self) -> &ConfigurationImprovementPreviewSummary {
        &self.summary
    }

    /// Borrow the existing canonical comparison owner for paging and explanation.
    #[must_use]
    pub const fn comparison(&self) -> &PortableWorkspaceComparison {
        &self.comparison
    }

    /// Borrow the exact baseline effective-profile inspection owner.
    #[must_use]
    pub const fn baseline_profile(&self) -> &EffectiveProfileCompilation {
        &self.baseline_profile
    }

    /// Borrow the exact candidate effective-profile inspection owner.
    #[must_use]
    pub const fn candidate_profile(&self) -> &EffectiveProfileCompilation {
        &self.candidate_profile
    }

    /// Page exact change roots from this retained review comparison.
    ///
    /// This delegates to the canonical comparison owner and performs no new
    /// compilation or derivation. A zero limit selects the comparison default;
    /// larger limits are capped by that owner.
    #[must_use]
    pub fn change_roots(&self, offset: u32, limit: u16) -> PortableWorkspaceChangeRootPage {
        self.comparison.change_roots(offset, limit)
    }

    /// Page the retained candidate impact partition.
    ///
    /// Each row keeps its compiler-owned affected or unaffected disposition.
    /// This query does not infer typed-value change from reachability.
    #[must_use]
    pub fn impacts(&self, offset: u32, limit: u16) -> PortableWorkspaceImpactPage {
        self.comparison.impacts(offset, limit)
    }

    /// Explain one subject through this review's retained causal comparison.
    ///
    /// The explanation selects the comparison owner's deterministic witness;
    /// it does not recompute a path or replace the candidate compilation.
    ///
    /// # Errors
    /// Returns the existing comparison diagnostic if retained proof evidence is
    /// internally inconsistent.
    pub fn explain_impact(
        &self,
        subject: &PortableWorkspaceSubject,
    ) -> Result<PortableWorkspaceExplanation, ApiError> {
        self.comparison.explain(subject)
    }

    /// Inspect exact candidate source occurrences for one review subject.
    ///
    /// The candidate input already retained by this review owns the query. It
    /// is never reconstructed from a digest and the baseline is never used as
    /// a substitute. Source disclosure remains the caller's responsibility.
    ///
    /// # Errors
    /// Returns the existing subject-inspection diagnostic for an unlike cursor
    /// or incoherent retained evidence.
    pub fn inspect_candidate_subject(
        &self,
        subject: &PortableWorkspaceSubject,
        after: Option<&PortableWorkspaceSubjectInspectionCursor>,
        input_limit: u16,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<PortableWorkspaceSubjectInspection, ApiError> {
        self.comparison.inspect_current_subject(
            subject,
            after,
            input_limit,
            maximum_inspected,
            maximum_returned,
        )
    }

    /// Explicitly accept this preview without mutating source or granting authority.
    #[must_use]
    pub fn accept(self) -> ConfigurationImprovementTerminal {
        self.finish(ConfigurationImprovementDisposition::Accepted)
    }

    /// Explicitly reject this preview without mutating source.
    #[must_use]
    pub fn reject(self) -> ConfigurationImprovementTerminal {
        self.finish(ConfigurationImprovementDisposition::Rejected)
    }

    /// Explicitly cancel this preview without mutating source.
    #[must_use]
    pub fn cancel(self) -> ConfigurationImprovementTerminal {
        self.finish(ConfigurationImprovementDisposition::Cancelled)
    }

    /// Mark this preview stale after an outer owner proves coordinate drift.
    ///
    /// The application does not invent that proof. An outer owner such as LSP
    /// or ACP must compare its separately typed native coordinate first.
    #[must_use]
    pub fn mark_stale(self) -> ConfigurationImprovementTerminal {
        self.finish(ConfigurationImprovementDisposition::Stale)
    }

    fn finish(
        self,
        disposition: ConfigurationImprovementDisposition,
    ) -> ConfigurationImprovementTerminal {
        let application_disposition = match disposition {
            ConfigurationImprovementDisposition::Accepted => {
                application::ConfigurationImprovementDisposition::Accepted
            }
            ConfigurationImprovementDisposition::Rejected => {
                application::ConfigurationImprovementDisposition::Rejected
            }
            ConfigurationImprovementDisposition::Cancelled => {
                application::ConfigurationImprovementDisposition::Cancelled
            }
            ConfigurationImprovementDisposition::Stale => {
                application::ConfigurationImprovementDisposition::Stale
            }
        };
        let _ = self.disposition_gate.finish(application_disposition);
        ConfigurationImprovementTerminal {
            disposition,
            preview: self.summary,
        }
    }
}

/// Effect-free terminal record for one reviewed candidate.
#[derive(serde::Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationImprovementTerminal {
    disposition: ConfigurationImprovementDisposition,
    #[serde(rename = "summary")]
    preview: ConfigurationImprovementPreviewSummary,
}

/// Whether the returned candidate-source page exhausts its evidence.
///
/// Stateless transports do not carry the inspection cursor because that
/// cursor belongs to its retained review. An incomplete page therefore tells
/// the caller to continue through a retained SDK or editor review.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationImprovementSourcePageState {
    /// The proof-input and source-occurrence sections are both exhausted.
    Complete,
    /// More evidence exists and this stateless result cannot resume the scan.
    RetainedReviewRequired,
}

impl From<&PortableWorkspaceSubjectInspection> for ConfigurationImprovementSourcePageState {
    fn from(source: &PortableWorkspaceSubjectInspection) -> Self {
        if source.is_complete() {
            Self::Complete
        } else {
            Self::RetainedReviewRequired
        }
    }
}

/// Exact transport result of a stateless source-based configuration review.
///
/// A preview carries the first bounded impact page beside its complete summary.
/// Details carry one subject's causal and source evidence after checking that
/// summary. A finished result carries the caller's explicit disposition. No
/// result writes configuration or establishes authority for a later effect.
#[derive(serde::Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ConfigurationImprovementReviewResult {
    /// Fresh review facts and the first bounded impact page.
    Preview {
        /// Complete coordinates and counts for the review.
        summary: ConfigurationImprovementPreviewSummary,
        /// First impact page in the SDK's canonical order.
        impacts: crate::PortableWorkspaceImpactPage,
    },
    /// Causal and source evidence for one subject in a checked preview.
    Details {
        /// Complete coordinates and counts for the checked review.
        summary: ConfigurationImprovementPreviewSummary,
        /// Existing comparison-owner explanation for the selected subject.
        explanation: crate::PortableWorkspaceExplanation,
        /// Bounded candidate-source evidence from the exact compilation.
        source: Box<crate::PortableWorkspaceSubjectInspection>,
        /// Whether this stateless page is complete or needs retained review.
        source_page_state: ConfigurationImprovementSourcePageState,
    },
    /// One explicit effect-free review decision.
    Finished {
        /// Disposition and exact preview governed by that decision.
        #[serde(flatten)]
        terminal: ConfigurationImprovementTerminal,
    },
}

impl ConfigurationImprovementTerminal {
    /// Explicit terminal disposition selected by the reviewing caller.
    #[must_use]
    pub const fn disposition(&self) -> ConfigurationImprovementDisposition {
        self.disposition
    }

    /// Exact preview coordinates governed by this disposition.
    #[must_use]
    pub const fn preview(&self) -> &ConfigurationImprovementPreviewSummary {
        &self.preview
    }
}

impl PaleApi {
    /// Build a new review from captured inputs and stop at its preview.
    ///
    /// The baseline is compiled once, then reused for profile explanation.
    /// Candidate evaluation uses its admitted Package semantics. This is not
    /// a continuation of a retained editor review and does not write source.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] from the owning compilation, profile, proposal or
    /// impact stage. No terminal decision is chosen on success or failure.
    pub fn preview_configuration_improvement_sources(
        &self,
        sources: ConfigurationImprovementSources,
    ) -> Result<ConfigurationImprovementPreview, ApiError> {
        let ConfigurationImprovementSources {
            workspace,
            profile,
            candidate,
        } = sources;
        let baseline = self.compile_portable_workspace(workspace)?;
        let profile = self.compile_effective_profile(&baseline, profile)?;
        let inspection = self.inspect_configuration_improvement(&baseline);
        let explanation = self.explain_configuration_improvement(inspection, &profile)?;
        let proposal = self.propose_configuration_improvement(explanation, candidate)?;
        let evaluation = self.evaluate_configuration_improvement(proposal)?;
        let diagnosis = self.diagnose_configuration_improvement(evaluation)?;
        self.preview_configuration_improvement(diagnosis)
    }

    /// Start a review from a successfully compiled workspace.
    #[must_use]
    pub fn inspect_configuration_improvement(
        &self,
        baseline: &crate::PortableWorkspaceCompilation,
    ) -> ConfigurationImprovementInspection {
        ConfigurationImprovementInspection {
            state: application::inspect_configuration_improvement(
                baseline.application_universe_handle(),
            ),
        }
    }

    /// Attach the baseline's compiled effective profile to the review.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when `profile` was compiled from a different
    /// admitted workspace frontier.
    pub fn explain_configuration_improvement(
        &self,
        inspection: ConfigurationImprovementInspection,
        profile: &EffectiveProfileCompilation,
    ) -> Result<ConfigurationImprovementExplanation, ApiError> {
        inspection
            .state
            .explain(Arc::clone(&profile.compilation))
            .map(|state| ConfigurationImprovementExplanation { state })
            .map_err(|error| improvement_failure(&self.diagnostic_identity, error))
    }

    /// Attach one bounded source candidate without evaluating it.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the captured request violates source bounds,
    /// label confinement, uniqueness, or root-presence requirements.
    pub fn propose_configuration_improvement(
        &self,
        explanation: ConfigurationImprovementExplanation,
        candidate: ConfigurationImprovementProposalRequest,
    ) -> Result<ConfigurationImprovementProposal, ApiError> {
        let ConfigurationImprovementProposalRequest {
            project_id,
            execution_workspace_id,
            machine_id,
            workspace,
            private_inputs,
        } = candidate;
        let workspace = workspace
            .into_captured_source()
            .map_err(|error| workspace_failure(&self.diagnostic_identity, error))?;
        let private = private_inputs
            .into_application()
            .map_err(pale_application::compile::CompileError::from)
            .map_err(|error| improvement_profile_failure(&self.diagnostic_identity, error))?;
        Ok(ConfigurationImprovementProposal {
            state: explanation.state.propose(
                application::CapturedConfigurationImprovementProposal::new(
                    project_id,
                    execution_workspace_id,
                    machine_id,
                    workspace,
                    private,
                ),
            ),
        })
    }

    /// Admit one proposal through the canonical workspace compiler.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the candidate fails ordinary workspace
    /// admission or attempts to replace the principal workspace identity.
    pub fn evaluate_configuration_improvement(
        &self,
        proposal: ConfigurationImprovementProposal,
    ) -> Result<ConfigurationImprovementEvaluation, ApiError> {
        proposal
            .state
            .evaluate()
            .map(|state| ConfigurationImprovementEvaluation { state })
            .map_err(|error| improvement_failure(&self.diagnostic_identity, error))
    }

    /// Derive the exact source-change and impact proof for an admitted candidate.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] only if previously admitted owners contradict the
    /// canonical derivation contract.
    pub fn diagnose_configuration_improvement(
        &self,
        evaluation: ConfigurationImprovementEvaluation,
    ) -> Result<ConfigurationImprovementDiagnosis, ApiError> {
        evaluation
            .state
            .diagnose()
            .map(|state| ConfigurationImprovementDiagnosis { state })
            .map_err(|error| improvement_failure(&self.diagnostic_identity, error))
    }

    /// Freeze one diagnosis into a bounded preview and retained comparison.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] only if an already verified owned projection cannot
    /// be represented in canonical JSON.
    pub fn preview_configuration_improvement(
        &self,
        diagnosis: ConfigurationImprovementDiagnosis,
    ) -> Result<ConfigurationImprovementPreview, ApiError> {
        let parts = diagnosis.state.preview().into_parts();
        let summary = ConfigurationImprovementPreviewSummary::project(
            &self.diagnostic_identity,
            &parts.baseline,
            &parts.profile,
            &parts.candidate,
            &parts.diagnosis,
        )?;
        let comparison = PortableWorkspaceComparison::from_application_parts(
            parts.baseline_owner,
            parts.candidate_owner,
            parts.derivation,
            self.diagnostic_identity.clone(),
        )
        .map_err(|error| portable_summary_failure(&self.diagnostic_identity, error))?;
        Ok(ConfigurationImprovementPreview {
            summary,
            comparison,
            baseline_profile: EffectiveProfileCompilation::from_application(
                parts.baseline_profile_owner,
                self.diagnostic_identity.clone(),
            ),
            candidate_profile: EffectiveProfileCompilation::from_application(
                parts.candidate_profile_owner,
                self.diagnostic_identity.clone(),
            ),
            disposition_gate: parts.disposition_gate,
        })
    }

    /// Prepare a candidate from one already admitted portable generation.
    ///
    /// The candidate workspace handle is reused directly; this route never
    /// rebuilds portable source. Private profile compilation reuses the
    /// explanation's source-free Package owner and exact project, execution
    /// workspace, and machine selectors, after which impact derivation
    /// and all fallible public projection complete before commit.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when private capture, candidate admission,
    /// derivation, or canonical projection fails. `explanation` remains
    /// available to retry after every such failure.
    pub fn prepare_configuration_improvement_preview(
        &self,
        explanation: &ConfigurationImprovementExplanation,
        candidate: &crate::PortableWorkspaceCompilation,
        request: ConfigurationImprovementCandidateRequest,
    ) -> Result<PreparedConfigurationImprovementPreview, ApiError> {
        let ConfigurationImprovementCandidateRequest {
            project_id,
            execution_workspace_id,
            machine_id,
            private_inputs,
        } = request;
        let private = private_inputs
            .into_application()
            .map_err(pale_application::compile::CompileError::from)
            .map_err(|error| improvement_profile_failure(&self.diagnostic_identity, error))?;
        let prepared = explanation
            .state
            .prepare_admitted_candidate(
                candidate.application_universe_handle(),
                application::CapturedConfigurationImprovementCandidate::new(
                    project_id,
                    execution_workspace_id,
                    machine_id,
                    private,
                ),
            )
            .map_err(|error| improvement_failure(&self.diagnostic_identity, error))?;
        let parts = prepared.into_parts();
        let summary = ConfigurationImprovementPreviewSummary::project(
            &self.diagnostic_identity,
            &parts.baseline,
            &parts.profile,
            &parts.candidate,
            &parts.diagnosis,
        )?;
        let comparison = PortableWorkspaceComparison::from_application_parts(
            parts.baseline_owner,
            parts.candidate_owner,
            parts.derivation,
            self.diagnostic_identity.clone(),
        )
        .map_err(|error| portable_summary_failure(&self.diagnostic_identity, error))?;
        Ok(PreparedConfigurationImprovementPreview {
            summary,
            comparison,
            baseline_profile: EffectiveProfileCompilation::from_application(
                parts.baseline_profile_owner,
                self.diagnostic_identity.clone(),
            ),
            candidate_profile: EffectiveProfileCompilation::from_application(
                parts.candidate_profile_owner,
                self.diagnostic_identity.clone(),
            ),
            commit_gate: parts.commit_gate,
        })
    }

    /// Commit a prepared candidate through its exact explanation owner.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] if a caller attempts to splice a preparation from
    /// another explanation lifecycle.
    pub fn commit_configuration_improvement_preview(
        &self,
        explanation: ConfigurationImprovementExplanation,
        prepared: PreparedConfigurationImprovementPreview,
    ) -> Result<ConfigurationImprovementPreview, ApiError> {
        let disposition_gate = explanation
            .state
            .commit_prepared(prepared.commit_gate)
            .map_err(|error| improvement_failure(&self.diagnostic_identity, error))?;
        Ok(ConfigurationImprovementPreview {
            summary: prepared.summary,
            comparison: prepared.comparison,
            baseline_profile: prepared.baseline_profile,
            candidate_profile: prepared.candidate_profile,
            disposition_gate,
        })
    }

    /// Finish a fresh review only if its complete summary matches the supplied preview.
    ///
    /// `presented_summary_bytes` is the JSON summary returned by an earlier
    /// preview. Whitespace and object-member order may differ. All fields must
    /// still match; checking only a profile digest is insufficient.
    ///
    /// This comparison does not resume the earlier handle or prove that a human
    /// saw it. It also does not establish source-byte equality, a current editor
    /// revision, or permission to write. Callers retain those separate checks.
    /// Acceptance returns the review decision without persisting it or applying a change.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] if the input exceeds
    /// [`ConfigurationImprovementPreviewSummary::MAX_PRESENTED_BYTES`], violates
    /// strict JSON, or differs from the fresh preview. No terminal is returned
    /// on failure; the consumed preview must be reconstructed for another
    /// attempt. A mismatch alone does not prove why the reviews differ.
    pub fn finish_presented_configuration_improvement(
        &self,
        preview: ConfigurationImprovementPreview,
        presented_summary_bytes: &[u8],
        disposition: ConfigurationImprovementDisposition,
    ) -> Result<ConfigurationImprovementTerminal, ApiError> {
        self.validate_presented_configuration_improvement(&preview, presented_summary_bytes)?;
        Ok(preview.finish(disposition))
    }

    /// Check an earlier summary against a fresh review without consuming it.
    ///
    /// Paging callers can inspect the retained comparison after this check,
    /// then use the same summary for another page or an explicit decision.
    /// All summary fields must match, including private-instance coordinates;
    /// JSON whitespace and object-member order may differ. This does not
    /// resume a handle, establish source-byte equality or an editor revision,
    /// prove human assent, or grant permission to write.
    ///
    /// # Errors
    ///
    /// Returns the same input-limit, strict-JSON or summary-mismatch diagnostic
    /// as [`Self::finish_presented_configuration_improvement`]. The caller
    /// retains the preview on both success and failure; no decision is made.
    pub fn validate_presented_configuration_improvement(
        &self,
        preview: &ConfigurationImprovementPreview,
        presented_summary_bytes: &[u8],
    ) -> Result<(), ApiError> {
        if presented_summary_bytes.len()
            > ConfigurationImprovementPreviewSummary::MAX_PRESENTED_BYTES
        {
            return Err(presented_summary_failure(
                &self.diagnostic_identity,
                PresentedSummaryError::TooLarge,
            ));
        }
        let value = pale_core::canon::parse_strict(presented_summary_bytes).map_err(|source| {
            presented_summary_failure(
                &self.diagnostic_identity,
                PresentedSummaryError::Syntax(source),
            )
        })?;
        let canonical = pale_core::canon::to_canonical_string(&value)
            .map_err(|source| portable_summary_failure(&self.diagnostic_identity, source))?;
        if canonical.as_bytes() != preview.summary().canonical_bytes() {
            return Err(presented_summary_failure(
                &self.diagnostic_identity,
                PresentedSummaryError::Mismatch,
            ));
        }
        Ok(())
    }
}

/// Rejected caller input, distinct from failure to project an admitted preview.
#[derive(Debug, thiserror::Error)]
enum PresentedSummaryError {
    #[error("presented configuration summary exceeds its input limit")]
    TooLarge,
    #[error("presented configuration summary is not strict JSON")]
    Syntax(#[source] pale_core::canon::CanonError),
    #[error("presented configuration summary differs from this review")]
    Mismatch,
}

fn presented_summary_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: PresentedSummaryError,
) -> ApiError {
    let stage = match &error {
        PresentedSummaryError::TooLarge | PresentedSummaryError::Syntax(_) => {
            DiagnosticStageV1::InputAdmission
        }
        PresentedSummaryError::Mismatch => DiagnosticStageV1::SemanticAdmission,
    };
    let (code, summary) = match &error {
        PresentedSummaryError::TooLarge => (
            "configuration_improvement_summary_too_large",
            "The presented configuration summary exceeds the input limit.",
        ),
        PresentedSummaryError::Syntax(_) => (
            "configuration_improvement_summary_invalid_json",
            "The presented configuration summary is not strict JSON.",
        ),
        PresentedSummaryError::Mismatch => (
            "configuration_improvement_summary_mismatch",
            "The presented configuration summary does not match this review.",
        ),
    };
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code,
            stage,
            disposition: DiagnosticDispositionV1::InvalidInput,
            summary,
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::AfterCorrection,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: vec!["No review decision was returned and no configuration was written."],
            help: vec![
                "Inspect the current preview and submit its complete summary with your decision.",
            ],
            facts: Vec::new(),
        },
    )
}

fn improvement_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: application::ConfigurationImprovementError,
) -> ApiError {
    match error {
        application::ConfigurationImprovementError::CandidateWorkspaceRejected(source)
        | application::ConfigurationImprovementError::DiagnosisRejected(source) => {
            workspace_failure(identity, source)
        }
        application::ConfigurationImprovementError::CandidateProfileRejected(source) => {
            improvement_profile_failure(identity, source)
        }
        error @ application::ConfigurationImprovementError::ExplanationFrontierMismatch {
            ..
        } => improvement_owner_failure(
            identity,
            error,
            "configuration_improvement_frontier_mismatch",
        ),
        error @ application::ConfigurationImprovementError::CandidateWorkspaceMismatch => {
            improvement_owner_failure(
                identity,
                error,
                "configuration_improvement_workspace_mismatch",
            )
        }
        error @ application::ConfigurationImprovementError::CandidateContextMismatch { .. } => {
            improvement_owner_failure(
                identity,
                error,
                "configuration_improvement_candidate_context_mismatch",
            )
        }
        error @ application::ConfigurationImprovementError::CandidateDistributionFrontierMismatch => {
            improvement_owner_failure(
                identity,
                error,
                "configuration_improvement_distribution_frontier_mismatch",
            )
        }
        error @ application::ConfigurationImprovementError::CandidatePackageReleaseFrontierMismatch => {
            improvement_owner_failure(
                identity,
                error,
                "configuration_improvement_package_release_frontier_mismatch",
            )
        }
        error @ application::ConfigurationImprovementError::PreparedOwnerMismatch => {
            improvement_owner_failure(
                identity,
                error,
                "configuration_improvement_preparation_owner_mismatch",
            )
        }
    }
}

fn improvement_profile_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: pale_application::compile::CompileError,
) -> ApiError {
    let (disposition, retry) = application_diagnostic_policy(error.failure_disposition());
    let (subject, facts) = match &error {
        pale_application::compile::CompileError::CompositionInput(source)
            if disposition == DiagnosticDispositionV1::InvalidInput =>
        {
            crate::error::composition_failure_details(source)
        }
        _ => (None, Vec::new()),
    };
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: "configuration_improvement_candidate_rejected",
            stage: DiagnosticStageV1::Compilation,
            disposition,
            summary: "configuration-improvement candidate did not produce an admitted profile",
            subject,
            schema_coordinate: None,
            location: None,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: vec![
                "The candidate stopped before preview and no source or native effect was attempted.",
            ],
            help: if disposition == DiagnosticDispositionV1::InvalidInput {
                vec!["Correct the candidate inputs and evaluate the proposal again."]
            } else {
                Vec::new()
            },
            facts,
        },
    )
}

fn improvement_owner_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: application::ConfigurationImprovementError,
    code: &'static str,
) -> ApiError {
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code,
            stage: DiagnosticStageV1::SemanticAdmission,
            disposition: DiagnosticDispositionV1::InvalidInput,
            summary: "configuration improvement does not share one admitted owner frontier",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::AfterCorrection,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: vec![
                "The review stopped before preview and no source or native effect was attempted.",
            ],
            help: vec![
                "Restart inspection from the current admitted workspace and profile handles.",
            ],
            facts: Vec::new(),
        },
    )
}

#[cfg(test)]
#[path = "../tests/internal/improvement.rs"]
mod tests;
